//! High-performance CPAL audio engine managing WASAPI Exclusive duplex streams,
//! clock drift compensation, channel strip DSP routing, and master sum bus limiting.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, StreamTrait};

use crate::audio::clock_drift::{ClockDriftBuffer, SharedClockDriftBuffer};
use crate::audio::device::{
    default_device, find_device, query_input_devices, query_output_devices, AudioDeviceInfo,
    AudioStreamConfig, DeviceError,
};
use crate::audio::recorder::WavRecorder;
use crate::core::frame::AudioFrame;
use crate::engine::channel::ChannelStrip;
use crate::engine::master_bus::MasterBus;

/// Maximum buffer frames handled without heap allocation on the audio callback stack.
const MAX_STACK_FRAMES: usize = 1024;

/// Select the preferred CPAL audio host for the current operating system.
///
/// On Windows, prefers the low-latency WASAPI backend; falls back to the default host.
fn get_audio_host() -> cpal::Host {
    #[cfg(target_os = "windows")]
    {
        if let Ok(host) = cpal::host_from_id(cpal::HostId::Wasapi) {
            return host;
        }
    }
    cpal::default_host()
}

/// Core audio engine managing duplex hardware streaming, DSP routing, and metering.
pub struct AudioEngine {
    host: cpal::Host,
    config: AudioStreamConfig,
    guitar_channel: Arc<Mutex<ChannelStrip>>,
    mic_channel: Arc<Mutex<ChannelStrip>>,
    master_bus: Arc<Mutex<MasterBus>>,
    drift_buffer: SharedClockDriftBuffer,
    recorder: Arc<Mutex<Option<Arc<Mutex<WavRecorder>>>>>,
    input_stream: Option<cpal::Stream>,
    output_stream: Option<cpal::Stream>,
    active_input_name: Option<String>,
    active_output_name: Option<String>,
    is_running: Arc<AtomicBool>,
}

impl AudioEngine {
    /// Create a new AudioEngine with default configuration (48 kHz, 64 buffer frames).
    pub fn new() -> Result<Self, DeviceError> {
        Self::with_config(AudioStreamConfig::DEFAULT)
    }

    /// Create an AudioEngine with a specific stream configuration.
    pub fn with_config(config: AudioStreamConfig) -> Result<Self, DeviceError> {
        config.validate()?;
        let host = get_audio_host();
        let drift_capacity = (config.buffer_size as usize * 4).max(256);
        let drift_buffer = ClockDriftBuffer::shared(drift_capacity);

        let guitar = Arc::new(Mutex::new(ChannelStrip::with_sample_rate(
            "Guitar",
            config.sample_rate as f32,
        )));
        let mic = Arc::new(Mutex::new(ChannelStrip::with_sample_rate(
            "Mic",
            config.sample_rate as f32,
        )));
        let master = Arc::new(Mutex::new(MasterBus::with_sample_rate(
            config.sample_rate as f32,
        )));

        Ok(Self {
            host,
            config,
            guitar_channel: guitar,
            mic_channel: mic,
            master_bus: master,
            drift_buffer,
            recorder: Arc::new(Mutex::new(None)),
            input_stream: None,
            output_stream: None,
            active_input_name: None,
            active_output_name: None,
            is_running: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Enumerate all available audio input devices.
    pub fn input_devices(&self) -> Vec<AudioDeviceInfo> {
        query_input_devices(&self.host)
    }

    /// Enumerate all available audio output devices.
    pub fn output_devices(&self) -> Vec<AudioDeviceInfo> {
        query_output_devices(&self.host)
    }

    /// Name of the currently active hardware input device, if streaming.
    #[inline]
    pub fn active_input_device(&self) -> Option<&str> {
        self.active_input_name.as_deref()
    }

    /// Name of the currently active hardware output device, if streaming.
    #[inline]
    pub fn active_output_device(&self) -> Option<&str> {
        self.active_output_name.as_deref()
    }

    /// Return true if CPAL duplex streams are actively running.
    #[inline]
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }

    /// Current audio engine configuration.
    #[inline]
    pub fn config(&self) -> AudioStreamConfig {
        self.config
    }

    /// Update audio stream configuration and propagate sample rate to channels and master bus.
    pub fn set_config(&mut self, config: AudioStreamConfig) -> Result<(), DeviceError> {
        config.validate()?;
        let was_running = self.is_running();
        if was_running {
            self.stop()?;
        }

        self.config = config;
        let sr = config.sample_rate as f32;

        if let Ok(g) = self.guitar_channel.lock() {
            g.set_sample_rate(sr);
        }
        if let Ok(m) = self.mic_channel.lock() {
            m.set_sample_rate(sr);
        }
        if let Ok(mb) = self.master_bus.lock() {
            mb.set_sample_rate(sr);
        }

        let drift_capacity = (config.buffer_size as usize * 4).max(256);
        if let Ok(mut db) = self.drift_buffer.lock() {
            *db = ClockDriftBuffer::new(drift_capacity);
        }

        if was_running {
            let in_dev = self.active_input_name.clone();
            let out_dev = self.active_output_name.clone();
            self.start(in_dev.as_deref(), out_dev.as_deref())?;
        }

        Ok(())
    }

    /// Reference to Guitar channel strip (`ChannelStrip`).
    #[inline]
    pub fn guitar_channel(&self) -> &Arc<Mutex<ChannelStrip>> {
        &self.guitar_channel
    }

    /// Reference to Mic channel strip (`ChannelStrip`).
    #[inline]
    pub fn mic_channel(&self) -> &Arc<Mutex<ChannelStrip>> {
        &self.mic_channel
    }

    /// Reference to master output bus (`MasterBus`).
    #[inline]
    pub fn master_bus(&self) -> &Arc<Mutex<MasterBus>> {
        &self.master_bus
    }

    /// Reference to adaptive clock drift buffer (`ClockDriftBuffer`).
    #[inline]
    pub fn drift_buffer(&self) -> &SharedClockDriftBuffer {
        &self.drift_buffer
    }

    /// Attach a background WAV recorder to capture master sum bus output.
    pub fn attach_recorder(&self, recorder: Arc<Mutex<WavRecorder>>) {
        if let Ok(mut r) = self.recorder.lock() {
            *r = Some(recorder);
        }
    }

    /// Detach the background WAV recorder.
    pub fn detach_recorder(&self) -> Option<Arc<Mutex<WavRecorder>>> {
        if let Ok(mut r) = self.recorder.lock() {
            r.take()
        } else {
            None
        }
    }

    /// Current Guitar channel peak levels in dBFS `(left_db, right_db)`.
    pub fn guitar_peak_db(&self) -> (f32, f32) {
        self.guitar_channel
            .lock()
            .map(|g| g.peak_db())
            .unwrap_or((-120.0, -120.0))
    }

    /// Current Guitar channel RMS levels in dBFS `(left_db, right_db)`.
    pub fn guitar_rms_db(&self) -> (f32, f32) {
        self.guitar_channel
            .lock()
            .map(|g| g.rms_db())
            .unwrap_or((-120.0, -120.0))
    }

    /// Current Mic channel peak levels in dBFS `(left_db, right_db)`.
    pub fn mic_peak_db(&self) -> (f32, f32) {
        self.mic_channel
            .lock()
            .map(|m| m.peak_db())
            .unwrap_or((-120.0, -120.0))
    }

    /// Current Mic channel RMS levels in dBFS `(left_db, right_db)`.
    pub fn mic_rms_db(&self) -> (f32, f32) {
        self.mic_channel
            .lock()
            .map(|m| m.rms_db())
            .unwrap_or((-120.0, -120.0))
    }

    /// Current master output bus peak levels in dBFS `(left_db, right_db)`.
    pub fn master_peak_db(&self) -> (f32, f32) {
        self.master_bus
            .lock()
            .map(|mb| mb.meter().peak_db())
            .unwrap_or((-120.0, -120.0))
    }

    /// Current master output bus RMS levels in dBFS `(left_db, right_db)`.
    pub fn master_rms_db(&self) -> (f32, f32) {
        self.master_bus
            .lock()
            .map(|mb| mb.meter().rms_db())
            .unwrap_or((-120.0, -120.0))
    }

    /// Current master bus limiter gain reduction in decibels (negative or zero dBFS).
    pub fn master_gain_reduction_db(&self) -> f32 {
        self.master_bus
            .lock()
            .map(|mb| mb.gain_reduction_db())
            .unwrap_or(0.0)
    }

    /// Start CPAL hardware audio streams (WASAPI Exclusive on Windows).
    ///
    /// If device names are `None`, the default input and output devices are selected.
    pub fn start(
        &mut self,
        input_device_name: Option<&str>,
        output_device_name: Option<&str>,
    ) -> Result<(), DeviceError> {
        self.stop()?;

        let input_dev = match input_device_name {
            Some(name) => find_device(&self.host, name, true)?,
            None => default_device(&self.host, true)?,
        };

        let output_dev = match output_device_name {
            Some(name) => find_device(&self.host, name, false)?,
            None => default_device(&self.host, false)?,
        };

        let in_name = input_dev
            .name()
            .unwrap_or_else(|_| "Default Input".to_string());
        let out_name = output_dev
            .name()
            .unwrap_or_else(|_| "Default Output".to_string());

        let in_channels = input_dev
            .default_input_config()
            .map(|c| c.channels())
            .unwrap_or(2)
            .min(2);
        let out_channels = output_dev
            .default_output_config()
            .map(|c| c.channels())
            .unwrap_or(2)
            .max(2);

        let in_config = cpal::StreamConfig {
            channels: in_channels,
            sample_rate: cpal::SampleRate(self.config.sample_rate),
            buffer_size: cpal::BufferSize::Fixed(self.config.buffer_size),
        };

        let out_config = cpal::StreamConfig {
            channels: out_channels,
            sample_rate: cpal::SampleRate(self.config.sample_rate),
            buffer_size: cpal::BufferSize::Fixed(self.config.buffer_size),
        };

        // Reset drift buffer for fresh stream start
        if let Ok(mut db) = self.drift_buffer.lock() {
            db.reset();
        }

        // Input stream callback
        let drift_in = self.drift_buffer.clone();
        let in_stream = input_dev
            .build_input_stream(
                &in_config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    let ch = in_channels as usize;
                    if ch == 0 {
                        return;
                    }
                    let frame_count = data.len() / ch;
                    if let Ok(mut buf) = drift_in.lock() {
                        for f in 0..frame_count {
                            let left = data[f * ch];
                            let right = if ch > 1 { data[f * ch + 1] } else { left };
                            buf.write(AudioFrame::new(left, right));
                        }
                    }
                },
                move |err| {
                    eprintln!("AMPku audio input stream error: {err}");
                },
                None,
            )
            .map_err(|e| DeviceError::StreamBuildError(e.to_string()))?;

        // Output stream callback (stack-allocated scratch buffers to prevent mallocs)
        let drift_out = self.drift_buffer.clone();
        let guitar_out = self.guitar_channel.clone();
        let mic_out = self.mic_channel.clone();
        let master_out = self.master_bus.clone();
        let rec_out = self.recorder.clone();

        let out_stream = output_dev
            .build_output_stream(
                &out_config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let ch = out_channels as usize;
                    if ch == 0 {
                        return;
                    }
                    let frame_count = data.len() / ch;
                    let count = frame_count.min(MAX_STACK_FRAMES);

                    let mut in_frames = [AudioFrame::ZERO; MAX_STACK_FRAMES];
                    let mut guitar_frames = [AudioFrame::ZERO; MAX_STACK_FRAMES];
                    let mut mic_frames = [AudioFrame::ZERO; MAX_STACK_FRAMES];
                    let mut master_frames = [AudioFrame::ZERO; MAX_STACK_FRAMES];

                    // 1. Pull frames from drift buffer
                    if let Ok(mut db) = drift_out.lock() {
                        db.read(&mut in_frames[..count]);
                    }

                    // 2. Route left to guitar, right to mic
                    for i in 0..count {
                        guitar_frames[i] = AudioFrame::from_mono(in_frames[i].left);
                        mic_frames[i] = AudioFrame::from_mono(in_frames[i].right);
                    }

                    // 3. Process channels with solo bus state
                    let solo_g = guitar_out.lock().map(|g| g.is_solo()).unwrap_or(false);
                    let solo_m = mic_out.lock().map(|m| m.is_solo()).unwrap_or(false);
                    let solo_active = solo_g || solo_m;

                    if let Ok(mut g) = guitar_out.lock() {
                        g.process_block_with_solo(&mut guitar_frames[..count], solo_active);
                    }
                    if let Ok(mut m) = mic_out.lock() {
                        m.process_block_with_solo(&mut mic_frames[..count], solo_active);
                    }

                    // 4. Sum channels into master bus
                    for i in 0..count {
                        master_frames[i] = guitar_frames[i] + mic_frames[i];
                    }
                    if let Ok(mut mb) = master_out.lock() {
                        mb.process_block(&mut master_frames[..count]);
                    }

                    // 5. Push to background recorder if recording is active
                    if let Ok(r_guard) = rec_out.lock() {
                        if let Some(ref rec_arc) = *r_guard {
                            if let Ok(mut recorder) = rec_arc.lock() {
                                recorder.push_block(&master_frames[..count]);
                            }
                        }
                    }

                    // 6. Write output hardware buffer
                    for f in 0..count {
                        let frame = master_frames[f];
                        if ch == 1 {
                            data[f] = frame.to_mono();
                        } else {
                            data[f * ch] = frame.left;
                            data[f * ch + 1] = frame.right;
                            for extra in 2..ch {
                                data[f * ch + extra] = 0.0;
                            }
                        }
                    }
                },
                move |err| {
                    eprintln!("AMPku audio output stream error: {err}");
                },
                None,
            )
            .map_err(|e| DeviceError::StreamBuildError(e.to_string()))?;

        in_stream
            .play()
            .map_err(|e| DeviceError::StreamPlayError(e.to_string()))?;
        out_stream
            .play()
            .map_err(|e| DeviceError::StreamPlayError(e.to_string()))?;

        self.input_stream = Some(in_stream);
        self.output_stream = Some(out_stream);
        self.active_input_name = Some(in_name);
        self.active_output_name = Some(out_name);
        self.is_running.store(true, Ordering::Relaxed);

        Ok(())
    }

    /// Stop running CPAL audio hardware streams.
    pub fn stop(&mut self) -> Result<(), DeviceError> {
        if let Some(stream) = self.input_stream.take() {
            let _ = stream.pause();
        }
        if let Some(stream) = self.output_stream.take() {
            let _ = stream.pause();
        }
        self.active_input_name = None;
        self.active_output_name = None;
        self.is_running.store(false, Ordering::Relaxed);
        Ok(())
    }

    /// In-memory DSP block processing without physical hardware (for testing and offline rendering).
    pub fn process_block(&self, input: &[AudioFrame], output: &mut [AudioFrame]) {
        let count = input.len().min(output.len());
        if count == 0 {
            return;
        }

        // Write input into drift buffer
        if let Ok(mut db) = self.drift_buffer.lock() {
            for &f in &input[..count] {
                db.write(f);
            }
        }

        // Pull from drift buffer
        let mut in_frames = vec![AudioFrame::ZERO; count];
        if let Ok(mut db) = self.drift_buffer.lock() {
            db.read(&mut in_frames);
        }

        // Split Guitar and Mic
        let mut guitar_frames = in_frames.clone();
        for f in guitar_frames.iter_mut() {
            *f = AudioFrame::from_mono(f.left);
        }
        let mut mic_frames = in_frames.clone();
        for f in mic_frames.iter_mut() {
            *f = AudioFrame::from_mono(f.right);
        }

        let solo_g = self
            .guitar_channel
            .lock()
            .map(|g| g.is_solo())
            .unwrap_or(false);
        let solo_m = self
            .mic_channel
            .lock()
            .map(|m| m.is_solo())
            .unwrap_or(false);
        let solo_active = solo_g || solo_m;

        if let Ok(mut g) = self.guitar_channel.lock() {
            g.process_block_with_solo(&mut guitar_frames, solo_active);
        }
        if let Ok(mut m) = self.mic_channel.lock() {
            m.process_block_with_solo(&mut mic_frames, solo_active);
        }

        let mut sum_frames = vec![AudioFrame::ZERO; count];
        for i in 0..count {
            sum_frames[i] = guitar_frames[i] + mic_frames[i];
        }

        if let Ok(mut mb) = self.master_bus.lock() {
            mb.process_block(&mut sum_frames);
        }

        if let Ok(rec_guard) = self.recorder.lock() {
            if let Some(ref rec_arc) = *rec_guard {
                if let Ok(mut rec) = rec_arc.lock() {
                    rec.push_block(&sum_frames);
                }
            }
        }

        output[..count].copy_from_slice(&sum_frames[..count]);
    }
}

impl Drop for AudioEngine {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
