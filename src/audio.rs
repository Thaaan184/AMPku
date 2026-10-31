use anyhow::{anyhow, Context, Result};
use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{BufferSize, Device, SampleFormat, SampleRate, Stream, StreamConfig};
use ringbuf::traits::{Consumer, Producer, Split};
use ringbuf::HeapRb;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::effects::{EffectEngine, EffectParams};

pub struct AudioEngine {
    pub input_device_name: String,
    pub output_device_name: String,
    pub sample_rate: u32,
    pub buffer_size: u32,
    pub is_running: Arc<AtomicBool>,
    pub params: Arc<EffectParams>,
    input_stream: Option<Stream>,
    output_stream: Option<Stream>,
}

impl AudioEngine {
    pub fn new(params: Arc<EffectParams>) -> Self {
        Self {
            input_device_name: String::from("Default"),
            output_device_name: String::from("Default"),
            sample_rate: 48000,
            buffer_size: 256,
            is_running: Arc::new(AtomicBool::new(false)),
            params,
            input_stream: None,
            output_stream: None,
        }
    }

    pub fn start(&mut self, input_dev: &Device, output_dev: &Device) -> Result<()> {
        self.stop();

        let in_name = input_dev.name().unwrap_or_else(|_| "Unknown Input".into());
        let out_name = output_dev
            .name()
            .unwrap_or_else(|_| "Unknown Output".into());

        self.input_device_name = in_name;
        self.output_device_name = out_name;

        let sample_rate = SampleRate(self.sample_rate);

        // 1. Configure Input Stream (Mono or Stereo converted to mono)
        let in_supported = input_dev
            .supported_input_configs()
            .context("Failed to query input configs")?;
        let in_config = in_supported
            .filter(|c| c.sample_format() == SampleFormat::F32)
            .find(|c| c.min_sample_rate() <= sample_rate && c.max_sample_rate() >= sample_rate)
            .ok_or_else(|| anyhow!("No supported 48kHz float32 input config found"))?
            .with_sample_rate(sample_rate);

        let in_channels = in_config.channels() as usize;
        let in_stream_cfg = StreamConfig {
            channels: in_config.channels(),
            sample_rate,
            buffer_size: BufferSize::Fixed(self.buffer_size),
        };

        // 2. Configure Output Stream (Stereo float32)
        let out_supported = output_dev
            .supported_output_configs()
            .context("Failed to query output configs")?;
        let out_config = out_supported
            .filter(|c| c.sample_format() == SampleFormat::F32)
            .find(|c| c.min_sample_rate() <= sample_rate && c.max_sample_rate() >= sample_rate)
            .ok_or_else(|| anyhow!("No supported 48kHz float32 output config found"))?
            .with_sample_rate(sample_rate);

        let out_channels = out_config.channels() as usize;
        let out_stream_cfg = StreamConfig {
            channels: out_config.channels(),
            sample_rate,
            buffer_size: BufferSize::Fixed(self.buffer_size),
        };

        // 3. Pre-allocated lock-free SPSC Ring Buffer (capacity = 48000 frames = 1.0 second)
        let rb = HeapRb::<f32>::new(48000);
        let (mut producer, mut consumer) = rb.split();

        // 4. Build Input Stream Callback
        let err_fn = |err| eprintln!("Audio Stream Error: {}", err);

        let in_stream = input_dev.build_input_stream(
            &in_stream_cfg,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                if in_channels == 1 {
                    for &sample in data {
                        let _ = producer.try_push(sample);
                    }
                } else {
                    // Extract first channel (mono guitar input)
                    for frame in data.chunks_exact(in_channels) {
                        let _ = producer.try_push(frame[0]);
                    }
                }
            },
            err_fn,
            None,
        )?;

        // 5. Build Output Stream Callback with EffectEngine
        let mut effect_engine =
            EffectEngine::new(Arc::clone(&self.params), self.sample_rate as f32);
        let mut mono_scratch = vec![0.0f32; 4096];

        let out_stream = output_dev.build_output_stream(
            &out_stream_cfg,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let frames = data.len() / out_channels;
                if mono_scratch.len() < frames {
                    mono_scratch.resize(frames, 0.0);
                }

                // Pull mono samples from ring buffer
                for sample in mono_scratch.iter_mut().take(frames) {
                    *sample = consumer.try_pop().unwrap_or(0.0);
                }

                // Process DSP effects mono -> stereo
                if out_channels == 2 {
                    effect_engine.process_mono_to_stereo(&mono_scratch[..frames], data);
                } else {
                    // Fallback for mono or multi-channel
                    let mut temp_stereo = [0.0f32; 2];
                    for (i, frame) in data.chunks_mut(out_channels).enumerate() {
                        let s = mono_scratch[i];
                        effect_engine.process_mono_to_stereo(&[s], &mut temp_stereo);
                        frame.fill(temp_stereo[0]);
                    }
                }
            },
            err_fn,
            None,
        )?;

        in_stream.play()?;
        out_stream.play()?;

        self.input_stream = Some(in_stream);
        self.output_stream = Some(out_stream);
        self.is_running.store(true, Ordering::SeqCst);

        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(s) = self.input_stream.take() {
            let _ = s.pause();
        }
        if let Some(s) = self.output_stream.take() {
            let _ = s.pause();
        }
        self.is_running.store(false, Ordering::SeqCst);
    }
}
