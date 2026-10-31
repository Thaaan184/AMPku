//! High-performance CPAL audio engine managing WASAPI Exclusive duplex streams,
//! clock drift compensation, channel strip DSP routing, and master sum bus limiting.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, StreamTrait};

use crate::audio::clock_drift::{
    try_lock_spin, ClockDriftBuffer, SharedClockDriftBuffer, DEFAULT_TRY_LOCK_SPINS,
};
use crate::audio::device::{
    default_device, find_device, query_input_devices, query_output_devices, AudioDeviceInfo,
    AudioStreamConfig, DeviceError,
};
use crate::audio::recorder::WavRecorder;
use crate::core::atomic_float::AtomicF32;
use crate::core::frame::AudioFrame;
use crate::engine::channel::{linear_to_db, ChannelStrip};
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

/// Real-time lock-free stereo VU meters and limiter gain reduction for GUI visualization.
///
/// Thread-safe and non-blocking: the audio stream callbacks update these meters atomically,
/// and the GUI thread can poll them at 60 Hz without taking mutex locks or causing priority inversion.
#[derive(Debug)]
pub struct EngineMeters {
    pub guitar_peak_left: AtomicF32,
    pub guitar_peak_right: AtomicF32,
    pub guitar_rms_left: AtomicF32,
    pub guitar_rms_right: AtomicF32,
    pub mic_peak_left: AtomicF32,
    pub mic_peak_right: AtomicF32,
    pub mic_rms_left: AtomicF32,
    pub mic_rms_right: AtomicF32,
    pub master_peak_left: AtomicF32,
    pub master_peak_right: AtomicF32,
    pub master_rms_left: AtomicF32,
    pub master_rms_right: AtomicF32,
    pub master_gain_reduction: AtomicF32,
}

impl Default for EngineMeters {
    fn default() -> Self {
        Self::new()
    }
}

impl EngineMeters {
    /// Create zeroed engine meters.
    pub fn new() -> Self {
        Self {
            guitar_peak_left: AtomicF32::new(0.0),
            guitar_peak_right: AtomicF32::new(0.0),
            guitar_rms_left: AtomicF32::new(0.0),
            guitar_rms_right: AtomicF32::new(0.0),
            mic_peak_left: AtomicF32::new(0.0),
            mic_peak_right: AtomicF32::new(0.0),
            mic_rms_left: AtomicF32::new(0.0),
            mic_rms_right: AtomicF32::new(0.0),
            master_peak_left: AtomicF32::new(0.0),
            master_peak_right: AtomicF32::new(0.0),
            master_rms_left: AtomicF32::new(0.0),
            master_rms_right: AtomicF32::new(0.0),
            master_gain_reduction: AtomicF32::new(0.0),
        }
    }

    /// Reset all meters and gain reduction registers to zero.
    pub fn reset(&self) {
        self.guitar_peak_left.set(0.0);
        self.guitar_peak_right.set(0.0);
        self.guitar_rms_left.set(0.0);
        self.guitar_rms_right.set(0.0);
        self.mic_peak_left.set(0.0);
        self.mic_peak_right.set(0.0);
        self.mic_rms_left.set(0.0);
        self.mic_rms_right.set(0.0);
        self.master_peak_left.set(0.0);
        self.master_peak_right.set(0.0);
        self.master_rms_left.set(0.0);
        self.master_rms_right.set(0.0);
        self.master_gain_reduction.set(0.0);
    }

    /// Guitar channel instantaneous peak levels `(left, right)` linear amplitude.
    #[inline]
    pub fn guitar_peak(&self) -> (f32, f32) {
        (self.guitar_peak_left.get(), self.guitar_peak_right.get())
    }

    /// Guitar channel RMS levels `(left, right)` linear amplitude.
    #[inline]
    pub fn guitar_rms(&self) -> (f32, f32) {
        (self.guitar_rms_left.get(), self.guitar_rms_right.get())
    }

    /// Mic channel instantaneous peak levels `(left, right)` linear amplitude.
    #[inline]
    pub fn mic_peak(&self) -> (f32, f32) {
        (self.mic_peak_left.get(), self.mic_peak_right.get())
    }

    /// Mic channel RMS levels `(left, right)` linear amplitude.
    #[inline]
    pub fn mic_rms(&self) -> (f32, f32) {
        (self.mic_rms_left.get(), self.mic_rms_right.get())
    }

    /// Master sum bus instantaneous peak levels `(left, right)` linear amplitude.
    #[inline]
    pub fn master_peak(&self) -> (f32, f32) {
        (self.master_peak_left.get(), self.master_peak_right.get())
    }

    /// Master sum bus RMS levels `(left, right)` linear amplitude.
    #[inline]
    pub fn master_rms(&self) -> (f32, f32) {
        (self.master_rms_left.get(), self.master_rms_right.get())
    }

    /// Current Guitar channel peak levels in dBFS `(left_db, right_db)`.
    #[inline]
    pub fn guitar_peak_db(&self) -> (f32, f32) {
        (
            linear_to_db(self.guitar_peak_left.get()),
            linear_to_db(self.guitar_peak_right.get()),
        )
    }

    /// Current Guitar channel RMS levels in dBFS `(left_db, right_db)`.
    #[inline]
    pub fn guitar_rms_db(&self) -> (f32, f32) {
        (
            linear_to_db(self.guitar_rms_left.get()),
            linear_to_db(self.guitar_rms_right.get()),
        )
    }

    /// Current Mic channel peak levels in dBFS `(left_db, right_db)`.
    #[inline]
    pub fn mic_peak_db(&self) -> (f32, f32) {
        (
            linear_to_db(self.mic_peak_left.get()),
            linear_to_db(self.mic_peak_right.get()),
        )
    }

    /// Current Mic channel RMS levels in dBFS `(left_db, right_db)`.
    #[inline]
    pub fn mic_rms_db(&self) -> (f32, f32) {
        (
            linear_to_db(self.mic_rms_left.get()),
            linear_to_db(self.mic_rms_right.get()),
        )
    }

    /// Current master output bus peak levels in dBFS `(left_db, right_db)`.
    #[inline]
    pub fn master_peak_db(&self) -> (f32, f32) {
        (
            linear_to_db(self.master_peak_left.get()),
            linear_to_db(self.master_peak_right.get()),
        )
    }

    /// Current master output bus RMS levels in dBFS `(left_db, right_db)`.
    #[inline]
    pub fn master_rms_db(&self) -> (f32, f32) {
        (
            linear_to_db(self.master_rms_left.get()),
            linear_to_db(self.master_rms_right.get()),
        )
    }

    /// Current master bus limiter gain reduction in decibels (negative or zero dBFS).
    #[inline]
    pub fn master_gain_reduction_db(&self) -> f32 {
        self.master_gain_reduction.get()
    }
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
    meters: Arc<EngineMeters>,
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
        let meters = Arc::new(EngineMeters::new());

        Ok(Self {
            host,
            config,
            guitar_channel: guitar,
            mic_channel: mic,
            master_bus: master,
            drift_buffer,
            recorder: Arc::new(Mutex::new(None)),
            meters,
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

    /// Reference to real-time atomic meters for lock-free UI polling.
    #[inline]
    pub fn meters(&self) -> &Arc<EngineMeters> {
        &self.meters
    }

    /// Reset all VU meter levels and limiter gain reduction registers.
    pub fn reset_meters(&self) {
        self.meters.reset();
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
    #[inline]
    pub fn guitar_peak_db(&self) -> (f32, f32) {
        self.meters.guitar_peak_db()
    }

    /// Current Guitar channel RMS levels in dBFS `(left_db, right_db)`.
    #[inline]
    pub fn guitar_rms_db(&self) -> (f32, f32) {
        self.meters.guitar_rms_db()
    }

    /// Current Mic channel peak levels in dBFS `(left_db, right_db)`.
    #[inline]
    pub fn mic_peak_db(&self) -> (f32, f32) {
        self.meters.mic_peak_db()
    }

    /// Current Mic channel RMS levels in dBFS `(left_db, right_db)`.
    #[inline]
    pub fn mic_rms_db(&self) -> (f32, f32) {
        self.meters.mic_rms_db()
    }

    /// Current master output bus peak levels in dBFS `(left_db, right_db)`.
    #[inline]
    pub fn master_peak_db(&self) -> (f32, f32) {
        self.meters.master_peak_db()
    }

    /// Current master output bus RMS levels in dBFS `(left_db, right_db)`.
    #[inline]
    pub fn master_rms_db(&self) -> (f32, f32) {
        self.meters.master_rms_db()
    }

    /// Current master bus limiter gain reduction in decibels (negative or zero dBFS).
    #[inline]
    pub fn master_gain_reduction_db(&self) -> f32 {
        self.meters.master_gain_reduction_db()
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
                    if let Some(mut buf) = try_lock_spin(&drift_in, DEFAULT_TRY_LOCK_SPINS) {
                        for f in 0..frame_count {
                            let left = data[f * ch];
                            let right = if ch > 1 { data[f * ch + 1] } else { 0.0 };
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
        let meters_out = self.meters.clone();

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

                    // 1. Pull frames from drift buffer (non-blocking with short spin retry loop)
                    if let Some(mut db) = try_lock_spin(&drift_out, DEFAULT_TRY_LOCK_SPINS) {
                        db.read(&mut in_frames[..count]);
                    }

                    // 2. Route left to guitar, right to mic
                    for i in 0..count {
                        guitar_frames[i] = AudioFrame::from_mono(in_frames[i].left);
                        mic_frames[i] = AudioFrame::from_mono(in_frames[i].right);
                    }

                    // 3. Process channels with solo bus state without redundant double locks
                    let mut guitar_guard = try_lock_spin(&guitar_out, DEFAULT_TRY_LOCK_SPINS);
                    let mut mic_guard = try_lock_spin(&mic_out, DEFAULT_TRY_LOCK_SPINS);

                    let solo_active = guitar_guard.as_ref().map(|g| g.is_solo()).unwrap_or(false)
                        || mic_guard.as_ref().map(|m| m.is_solo()).unwrap_or(false);

                    if let Some(ref mut g) = guitar_guard {
                        g.process_block_with_solo(&mut guitar_frames[..count], solo_active);
                        meters_out.guitar_peak_left.set(g.meter().peak_left());
                        meters_out.guitar_peak_right.set(g.meter().peak_right());
                        meters_out.guitar_rms_left.set(g.meter().rms_left());
                        meters_out.guitar_rms_right.set(g.meter().rms_right());
                    } else {
                        guitar_frames[..count].fill(AudioFrame::ZERO);
                    }

                    if let Some(ref mut m) = mic_guard {
                        m.process_block_with_solo(&mut mic_frames[..count], solo_active);
                        meters_out.mic_peak_left.set(m.meter().peak_left());
                        meters_out.mic_peak_right.set(m.meter().peak_right());
                        meters_out.mic_rms_left.set(m.meter().rms_left());
                        meters_out.mic_rms_right.set(m.meter().rms_right());
                    } else {
                        mic_frames[..count].fill(AudioFrame::ZERO);
                    }

                    // 4. Sum channels into master bus
                    for i in 0..count {
                        master_frames[i] = guitar_frames[i] + mic_frames[i];
                    }
                    if let Some(mut mb) = try_lock_spin(&master_out, DEFAULT_TRY_LOCK_SPINS) {
                        mb.process_block(&mut master_frames[..count]);
                        meters_out.master_peak_left.set(mb.meter().peak_left());
                        meters_out.master_peak_right.set(mb.meter().peak_right());
                        meters_out.master_rms_left.set(mb.meter().rms_left());
                        meters_out.master_rms_right.set(mb.meter().rms_right());
                        meters_out.master_gain_reduction.set(mb.gain_reduction_db());
                    } else {
                        master_frames[..count].fill(AudioFrame::ZERO);
                    }

                    // 5. Push to background recorder if recording is active (non-blocking)
                    if let Ok(r_guard) = rec_out.try_lock() {
                        if let Some(ref rec_arc) = *r_guard {
                            if let Ok(mut recorder) = rec_arc.try_lock() {
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

                    // Zero remaining hardware buffer if frame_count exceeds stack scratch limit
                    if frame_count > MAX_STACK_FRAMES {
                        data[count * ch..].fill(0.0);
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
        if let Some(mut db) = try_lock_spin(&self.drift_buffer, DEFAULT_TRY_LOCK_SPINS) {
            for &f in &input[..count] {
                db.write(f);
            }
        }

        // Pull from drift buffer
        let mut in_frames = vec![AudioFrame::ZERO; count];
        if let Some(mut db) = try_lock_spin(&self.drift_buffer, DEFAULT_TRY_LOCK_SPINS) {
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

        let mut guitar_guard = try_lock_spin(&self.guitar_channel, DEFAULT_TRY_LOCK_SPINS);
        let mut mic_guard = try_lock_spin(&self.mic_channel, DEFAULT_TRY_LOCK_SPINS);

        let solo_active = guitar_guard.as_ref().map(|g| g.is_solo()).unwrap_or(false)
            || mic_guard.as_ref().map(|m| m.is_solo()).unwrap_or(false);

        if let Some(ref mut g) = guitar_guard {
            g.process_block_with_solo(&mut guitar_frames, solo_active);
            self.meters.guitar_peak_left.set(g.meter().peak_left());
            self.meters.guitar_peak_right.set(g.meter().peak_right());
            self.meters.guitar_rms_left.set(g.meter().rms_left());
            self.meters.guitar_rms_right.set(g.meter().rms_right());
        } else {
            guitar_frames.fill(AudioFrame::ZERO);
        }

        if let Some(ref mut m) = mic_guard {
            m.process_block_with_solo(&mut mic_frames, solo_active);
            self.meters.mic_peak_left.set(m.meter().peak_left());
            self.meters.mic_peak_right.set(m.meter().peak_right());
            self.meters.mic_rms_left.set(m.meter().rms_left());
            self.meters.mic_rms_right.set(m.meter().rms_right());
        } else {
            mic_frames.fill(AudioFrame::ZERO);
        }

        let mut sum_frames = vec![AudioFrame::ZERO; count];
        for i in 0..count {
            sum_frames[i] = guitar_frames[i] + mic_frames[i];
        }

        if let Some(mut mb) = try_lock_spin(&self.master_bus, DEFAULT_TRY_LOCK_SPINS) {
            mb.process_block(&mut sum_frames);
            self.meters.master_peak_left.set(mb.meter().peak_left());
            self.meters.master_peak_right.set(mb.meter().peak_right());
            self.meters.master_rms_left.set(mb.meter().rms_left());
            self.meters.master_rms_right.set(mb.meter().rms_right());
            self.meters
                .master_gain_reduction
                .set(mb.gain_reduction_db());
        } else {
            sum_frames.fill(AudioFrame::ZERO);
        }

        if let Ok(rec_guard) = self.recorder.try_lock() {
            if let Some(ref rec_arc) = *rec_guard {
                if let Ok(mut rec) = rec_arc.try_lock() {
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
