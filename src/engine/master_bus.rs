//! Master sum bus with brickwall limiter, soft-saturation knee, and lock-free VU metering.

use std::sync::atomic::{AtomicBool, Ordering};

use crate::core::atomic_float::AtomicF32;
use crate::core::frame::AudioFrame;
use crate::engine::channel::{db_to_linear, linear_to_db, ChannelMeter};

/// Default limiter release time in milliseconds.
pub const DEFAULT_RELEASE_MS: f32 = 50.0;
/// Default limiter threshold amplitude (~-0.92 dBFS).
pub const DEFAULT_THRESHOLD: f32 = 0.90;
/// Default brickwall limiter ceiling amplitude (0 dBFS).
pub const DEFAULT_CEILING: f32 = 1.0;
/// Default sample rate in Hz.
pub const DEFAULT_SAMPLE_RATE: f32 = 48000.0;

/// Master sum bus providing channel summing, master volume fader,
/// transparent soft-knee saturation brickwall limiter, and real-time metering.
#[derive(Debug)]
pub struct MasterBus {
    gain: AtomicF32,
    threshold: AtomicF32,
    ceiling: AtomicF32,
    release_ms: AtomicF32,
    bypassed: AtomicBool,
    sample_rate: AtomicF32,
    gain_reduction_db: AtomicF32,
    meter: ChannelMeter,
    envelope: f32,
    lookahead_samples: usize,
    lookahead_buffer: Vec<AudioFrame>,
    lookahead_write_pos: usize,
}

impl MasterBus {
    /// Create a new MasterBus with default settings:
    /// 0 dB master volume, 0.90 threshold, 1.0 ceiling, 50 ms release, 48 kHz sample rate.
    pub fn new() -> Self {
        Self::with_sample_rate(DEFAULT_SAMPLE_RATE)
    }

    /// Create a MasterBus with a specific sample rate.
    pub fn with_sample_rate(sample_rate: f32) -> Self {
        let sr = if sample_rate > 0.0 {
            sample_rate
        } else {
            DEFAULT_SAMPLE_RATE
        };
        Self {
            gain: AtomicF32::new(1.0),
            threshold: AtomicF32::new(DEFAULT_THRESHOLD),
            ceiling: AtomicF32::new(DEFAULT_CEILING),
            release_ms: AtomicF32::new(DEFAULT_RELEASE_MS),
            bypassed: AtomicBool::new(false),
            sample_rate: AtomicF32::new(sr),
            gain_reduction_db: AtomicF32::new(0.0),
            meter: ChannelMeter::new(),
            envelope: 0.0,
            lookahead_samples: 0,
            lookahead_buffer: Vec::new(),
            lookahead_write_pos: 0,
        }
    }

    /// Create a MasterBus with custom limiter settings.
    pub fn with_settings(threshold: f32, ceiling: f32, release_ms: f32) -> Self {
        let bus = Self::new();
        bus.set_threshold(threshold);
        bus.set_ceiling(ceiling);
        bus.set_release_ms(release_ms);
        bus
    }

    /// Configure optional lookahead buffer size in samples.
    pub fn with_lookahead(mut self, samples: usize) -> Self {
        self.set_lookahead_samples(samples);
        self
    }

    /// Master output volume linear gain factor.
    #[inline]
    pub fn gain(&self) -> f32 {
        self.gain.get()
    }

    /// Set master output volume linear gain factor (clamped >= 0.0).
    #[inline]
    pub fn set_gain(&self, gain: f32) {
        self.gain.set(gain.max(0.0));
    }

    /// Master output volume in decibels (dBFS).
    #[inline]
    pub fn volume_db(&self) -> f32 {
        linear_to_db(self.gain())
    }

    /// Set master output volume in decibels (dBFS).
    #[inline]
    pub fn set_volume_db(&self, db: f32) {
        self.set_gain(db_to_linear(db));
    }

    /// Current limiter threshold as linear amplitude.
    #[inline]
    pub fn threshold(&self) -> f32 {
        self.threshold.get()
    }

    /// Set limiter threshold as linear amplitude (clamped to `[0.01, 1.0]`).
    #[inline]
    pub fn set_threshold(&self, threshold: f32) {
        self.threshold.set(threshold.clamp(0.01, 1.0));
    }

    /// Limiter threshold in decibels (dBFS).
    #[inline]
    pub fn threshold_db(&self) -> f32 {
        linear_to_db(self.threshold())
    }

    /// Set limiter threshold in decibels (dBFS).
    #[inline]
    pub fn set_threshold_db(&self, db: f32) {
        self.set_threshold(db_to_linear(db));
    }

    /// Current limiter ceiling as linear amplitude.
    #[inline]
    pub fn ceiling(&self) -> f32 {
        self.ceiling.get()
    }

    /// Set limiter ceiling as linear amplitude (clamped to `[0.01, 1.0]`).
    #[inline]
    pub fn set_ceiling(&self, ceiling: f32) {
        self.ceiling.set(ceiling.clamp(0.01, 1.0));
    }

    /// Limiter ceiling in decibels (dBFS).
    #[inline]
    pub fn ceiling_db(&self) -> f32 {
        linear_to_db(self.ceiling())
    }

    /// Set limiter ceiling in decibels (dBFS).
    #[inline]
    pub fn set_ceiling_db(&self, db: f32) {
        self.set_ceiling(db_to_linear(db));
    }

    /// Limiter release time in milliseconds.
    #[inline]
    pub fn release_ms(&self) -> f32 {
        self.release_ms.get()
    }

    /// Set limiter release time in milliseconds (clamped >= 1.0 ms).
    #[inline]
    pub fn set_release_ms(&self, ms: f32) {
        self.release_ms.set(ms.max(1.0));
    }

    /// Sample rate in Hz.
    #[inline]
    pub fn sample_rate(&self) -> f32 {
        self.sample_rate.get()
    }

    /// Set sample rate in Hz.
    pub fn set_sample_rate(&self, sr: f32) {
        if sr > 0.0 {
            self.sample_rate.set(sr);
        }
    }

    /// Return true if the limiter is bypassed.
    #[inline]
    pub fn is_bypassed(&self) -> bool {
        self.bypassed.load(Ordering::Relaxed)
    }

    /// Enable or disable limiter bypass.
    #[inline]
    pub fn set_bypass(&self, bypass: bool) {
        self.bypassed.store(bypass, Ordering::Relaxed);
    }

    /// Current limiter gain reduction in decibels (negative or zero dBFS).
    #[inline]
    pub fn gain_reduction_db(&self) -> f32 {
        self.gain_reduction_db.get()
    }

    /// Set current limiter gain reduction in decibels (for decoupled UI visualization).
    #[inline]
    pub fn set_gain_reduction_db(&self, db: f32) {
        self.gain_reduction_db.set(db);
    }

    /// Reference to master output VU meter.
    #[inline]
    pub fn meter(&self) -> &ChannelMeter {
        &self.meter
    }

    /// Configured lookahead delay in samples.
    #[inline]
    pub fn lookahead_samples(&self) -> usize {
        self.lookahead_samples
    }

    /// Set lookahead delay in samples.
    pub fn set_lookahead_samples(&mut self, samples: usize) {
        self.lookahead_samples = samples;
        if samples > 0 {
            self.lookahead_buffer = vec![AudioFrame::ZERO; samples];
            self.lookahead_write_pos = 0;
        } else {
            self.lookahead_buffer.clear();
            self.lookahead_write_pos = 0;
        }
    }

    /// Reset limiter envelope detector, meter, and lookahead buffer.
    pub fn reset(&mut self) {
        self.envelope = 0.0;
        self.gain_reduction_db.set(0.0);
        self.meter.reset();
        if !self.lookahead_buffer.is_empty() {
            self.lookahead_buffer.fill(AudioFrame::ZERO);
            self.lookahead_write_pos = 0;
        }
    }

    /// Process a single stereo audio frame through master gain and limiter.
    #[inline]
    pub fn process_frame(&mut self, frame: AudioFrame) -> AudioFrame {
        let sanitized = frame.sanitize();
        let gained = sanitized * self.gain.get();

        if self.is_bypassed() {
            self.gain_reduction_db.set(0.0);
            self.meter.update_frame(gained, self.sample_rate.get());
            return gained;
        }

        let sr = self.sample_rate.get();
        let release_sec = (self.release_ms.get() * 0.001).max(0.001);
        let release_coeff = (-1.0 / (release_sec * sr)).exp();

        let thresh = self.threshold.get();
        let ceil = self.ceiling.get().max(thresh);

        // Instant peak detection (linked stereo peak)
        let peak = gained.peak();

        if peak > self.envelope {
            // Instant attack for brickwall protection
            self.envelope = peak;
        } else {
            // Smooth release decay
            self.envelope *= release_coeff;
            if self.envelope < 1e-15 {
                self.envelope = 0.0;
            }
        }

        // Soft saturation knee curve calculation
        let (gain_factor, gr_db) = if self.envelope > thresh {
            let over = self.envelope - thresh;
            let range = ceil - thresh;
            let sat_env = if range <= 1e-6 {
                ceil
            } else {
                thresh + range * (over / range).tanh()
            };
            let sat_env = sat_env.min(ceil);
            let g = (sat_env / self.envelope).clamp(0.0, 1.0);
            let gr = linear_to_db(g);
            (g, gr)
        } else {
            (1.0, 0.0)
        };

        self.gain_reduction_db.set(gr_db);

        let output_frame = if self.lookahead_samples > 0 && !self.lookahead_buffer.is_empty() {
            let delayed = self.lookahead_buffer[self.lookahead_write_pos];
            self.lookahead_buffer[self.lookahead_write_pos] = gained;
            self.lookahead_write_pos = (self.lookahead_write_pos + 1) % self.lookahead_samples;
            (delayed * gain_factor).clamp_symmetric(ceil)
        } else {
            (gained * gain_factor).clamp_symmetric(ceil)
        };

        self.meter.update_frame(output_frame, sr);
        output_frame
    }

    /// Process an in-place block of audio frames.
    pub fn process_block(&mut self, block: &mut [AudioFrame]) {
        for frame in block.iter_mut() {
            *frame = self.process_frame(*frame);
        }
    }

    /// Sum multiple stereo audio channels (e.g. Guitar + Mic) and process through the master bus.
    pub fn sum_channels(&mut self, channels: &[AudioFrame]) -> AudioFrame {
        let mut sum = AudioFrame::ZERO;
        for &ch in channels {
            sum += ch;
        }
        self.process_frame(sum)
    }

    /// Sum a slice of audio frames (alias for `sum_channels`).
    #[inline]
    pub fn sum_frames(&mut self, frames: &[AudioFrame]) -> AudioFrame {
        self.sum_channels(frames)
    }
}

impl Default for MasterBus {
    fn default() -> Self {
        Self::new()
    }
}
