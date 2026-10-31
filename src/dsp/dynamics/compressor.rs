//! Dynamic range compressor with logarithmic gain-reduction envelope detection.

use crate::core::frame::AudioFrame;
use crate::dsp::traits::AudioEffect;

/// Dynamic range compressor with configurable threshold, ratio, attack, release, and makeup gain.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Compressor {
    sample_rate: f32,
    threshold_db: f32,
    ratio: f32,
    attack_ms: f32,
    release_ms: f32,
    makeup_gain_db: f32,
    envelope_gain_db: f32,
    enabled: bool,
}

impl Compressor {
    /// Create a new Compressor with specified parameters.
    pub fn new(
        sample_rate: f32,
        threshold_db: f32,
        ratio: f32,
        attack_ms: f32,
        release_ms: f32,
        makeup_gain_db: f32,
    ) -> Self {
        let sample_rate = if sample_rate.is_finite() && sample_rate > 1000.0 {
            sample_rate
        } else {
            48000.0
        };
        let threshold_db = if threshold_db.is_finite() {
            threshold_db.clamp(-80.0, 0.0)
        } else {
            -20.0
        };
        let ratio = if ratio.is_finite() && ratio >= 1.0 {
            ratio.clamp(1.0, 64.0)
        } else {
            4.0
        };
        let attack_ms = if attack_ms.is_finite() && attack_ms > 0.05 {
            attack_ms
        } else {
            10.0
        };
        let release_ms = if release_ms.is_finite() && release_ms > 0.5 {
            release_ms
        } else {
            100.0
        };
        let makeup_gain_db = if makeup_gain_db.is_finite() {
            makeup_gain_db.clamp(-24.0, 36.0)
        } else {
            0.0
        };

        Self {
            sample_rate,
            threshold_db,
            ratio,
            attack_ms,
            release_ms,
            makeup_gain_db,
            envelope_gain_db: 0.0,
            enabled: true,
        }
    }

    /// Set runtime parameters for the compressor.
    pub fn set_params(
        &mut self,
        sample_rate: f32,
        threshold_db: f32,
        ratio: f32,
        attack_ms: f32,
        release_ms: f32,
        makeup_gain_db: f32,
    ) {
        self.set_sample_rate(sample_rate);
        self.set_threshold_db(threshold_db);
        self.set_ratio(ratio);
        self.set_attack_ms(attack_ms);
        self.set_release_ms(release_ms);
        self.set_makeup_gain_db(makeup_gain_db);
    }

    /// Set sample rate in Hz.
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        if sample_rate.is_finite() && sample_rate > 1000.0 {
            self.sample_rate = sample_rate;
        }
    }

    /// Set threshold in decibels (dBFS).
    pub fn set_threshold_db(&mut self, threshold_db: f32) {
        if threshold_db.is_finite() {
            self.threshold_db = threshold_db.clamp(-80.0, 0.0);
        }
    }

    /// Set compression ratio (e.g. 2.0 = 2:1, 4.0 = 4:1).
    pub fn set_ratio(&mut self, ratio: f32) {
        if ratio.is_finite() && ratio >= 1.0 {
            self.ratio = ratio.clamp(1.0, 64.0);
        }
    }

    /// Set attack time in milliseconds.
    pub fn set_attack_ms(&mut self, attack_ms: f32) {
        if attack_ms.is_finite() && attack_ms > 0.05 {
            self.attack_ms = attack_ms;
        }
    }

    /// Set release time in milliseconds.
    pub fn set_release_ms(&mut self, release_ms: f32) {
        if release_ms.is_finite() && release_ms > 0.5 {
            self.release_ms = release_ms;
        }
    }

    /// Set makeup gain in decibels.
    pub fn set_makeup_gain_db(&mut self, makeup_gain_db: f32) {
        if makeup_gain_db.is_finite() {
            self.makeup_gain_db = makeup_gain_db.clamp(-24.0, 36.0);
        }
    }

    /// Current threshold in dB.
    #[inline]
    pub fn threshold_db(&self) -> f32 {
        self.threshold_db
    }

    /// Current compression ratio.
    #[inline]
    pub fn ratio(&self) -> f32 {
        self.ratio
    }

    /// Current attack time in ms.
    #[inline]
    pub fn attack_ms(&self) -> f32 {
        self.attack_ms
    }

    /// Current release time in ms.
    #[inline]
    pub fn release_ms(&self) -> f32 {
        self.release_ms
    }

    /// Current makeup gain in dB.
    #[inline]
    pub fn makeup_gain_db(&self) -> f32 {
        self.makeup_gain_db
    }

    /// Current sample rate in Hz.
    #[inline]
    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    /// Current amount of active gain reduction in decibels (always non-negative).
    #[inline]
    pub fn gain_reduction_db(&self) -> f32 {
        -self.envelope_gain_db
    }
}

impl Default for Compressor {
    fn default() -> Self {
        Self::new(48000.0, -20.0, 4.0, 10.0, 100.0, 0.0)
    }
}

impl AudioEffect for Compressor {
    #[inline]
    fn process_frame(&mut self, frame: AudioFrame) -> AudioFrame {
        let frame = frame.sanitize();
        if !self.enabled {
            return frame;
        }

        let peak = frame.peak();
        let peak_safe = peak.max(1e-9);
        let input_db = 20.0 * peak_safe.log10();

        // Feedforward static gain curve
        let target_gain_db = if input_db > self.threshold_db {
            let overshoot = input_db - self.threshold_db;
            let reduction = overshoot * (1.0 - 1.0 / self.ratio);
            -reduction
        } else {
            0.0
        };

        // Smooth gain envelope using attack / release one-pole low-pass filters
        let attack_alpha =
            1.0 - (-1.0 / (self.attack_ms.max(0.1) * 0.001 * self.sample_rate)).exp();
        let release_alpha =
            1.0 - (-1.0 / (self.release_ms.max(1.0) * 0.001 * self.sample_rate)).exp();

        if target_gain_db < self.envelope_gain_db {
            // Attack phase: compression increases
            self.envelope_gain_db += attack_alpha * (target_gain_db - self.envelope_gain_db);
        } else {
            // Release phase: compression decreases back toward 0 dB
            self.envelope_gain_db += release_alpha * (target_gain_db - self.envelope_gain_db);
        }

        // Subnormal protection
        if self.envelope_gain_db.abs() < 1e-6 {
            self.envelope_gain_db = 0.0;
        }

        let total_gain_db = self.envelope_gain_db + self.makeup_gain_db;
        let linear_gain = 10.0f32.powf(total_gain_db / 20.0);

        let out = frame * linear_gain;
        out.sanitize()
    }

    #[inline]
    fn reset(&mut self) {
        self.envelope_gain_db = 0.0;
    }

    fn name(&self) -> &'static str {
        "Compressor"
    }

    #[inline]
    fn is_enabled(&self) -> bool {
        self.enabled
    }

    #[inline]
    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}
