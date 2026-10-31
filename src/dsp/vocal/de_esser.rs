//! Sibilance detector and dynamic frequency attenuator (Vocal De-Esser).

use crate::core::frame::AudioFrame;
use crate::dsp::traits::AudioEffect;

/// Operational mode for gain reduction application in the de-esser.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum DeEsserMode {
    /// Attenuates high frequencies using a dynamic high-shelf filter.
    #[default]
    SplitBand,
    /// Attenuates the entire audio signal when sibilance is detected.
    WideBand,
}

/// Serializable configuration for `VocalDeEsser`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct VocalDeEsserConfig {
    pub sample_rate: f32,
    pub frequency: f32,
    pub threshold_db: f32,
    pub ratio: f32,
    pub attack_ms: f32,
    pub release_ms: f32,
    pub mode: DeEsserMode,
    pub enabled: bool,
}

/// Vocal De-Esser combining a Biquad bandpass side-chain detector (typically 5 kHz - 8 kHz)
/// with dynamic gain reduction.
///
/// Operates with zero heap allocations during real-time streaming audio processing.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(from = "VocalDeEsserConfig", into = "VocalDeEsserConfig")]
pub struct VocalDeEsser {
    sample_rate: f32,
    frequency: f32,
    threshold_db: f32,
    ratio: f32,
    attack_ms: f32,
    release_ms: f32,
    mode: DeEsserMode,
    enabled: bool,
    listen: bool,

    // Envelope follower state
    envelope_gain_db: f32,
    attack_alpha: f32,
    release_alpha: f32,

    // Sidechain bandpass filter coefficients & state registers
    sc_b0: f32,
    sc_b1: f32,
    sc_b2: f32,
    sc_a1: f32,
    sc_a2: f32,
    sc_s1_l: f32,
    sc_s2_l: f32,
    sc_s1_r: f32,
    sc_s2_r: f32,

    // Dynamic high-shelf filter precalculated factors & state registers
    cos_w: f32,
    alpha_hs: f32,
    last_gain_db: f32,
    hs_b0: f32,
    hs_b1: f32,
    hs_b2: f32,
    hs_a1: f32,
    hs_a2: f32,
    hs_s1_l: f32,
    hs_s2_l: f32,
    hs_s1_r: f32,
    hs_s2_r: f32,
}

impl PartialEq for VocalDeEsser {
    fn eq(&self, other: &Self) -> bool {
        (self.sample_rate - other.sample_rate).abs() < 1e-4
            && (self.frequency - other.frequency).abs() < 1e-4
            && (self.threshold_db - other.threshold_db).abs() < 1e-4
            && (self.ratio - other.ratio).abs() < 1e-4
            && (self.attack_ms - other.attack_ms).abs() < 1e-4
            && (self.release_ms - other.release_ms).abs() < 1e-4
            && self.mode == other.mode
            && self.enabled == other.enabled
            && self.listen == other.listen
    }
}

impl VocalDeEsser {
    /// Create a new VocalDeEsser with the given sample rate, detection frequency, threshold, and ratio.
    pub fn new(sample_rate: f32, frequency: f32, threshold_db: f32, ratio: f32) -> Self {
        let sample_rate = if sample_rate.is_finite() && sample_rate > 1000.0 {
            sample_rate
        } else {
            48000.0
        };

        let nyquist = sample_rate * 0.499;
        let frequency = if frequency.is_finite() {
            frequency.clamp(500.0, nyquist)
        } else {
            6000.0
        };

        let threshold_db = if threshold_db.is_finite() {
            threshold_db.clamp(-80.0, 0.0)
        } else {
            -20.0
        };

        let ratio = if ratio.is_finite() {
            ratio.clamp(1.0, 64.0)
        } else {
            4.0
        };

        let mut de_esser = Self {
            sample_rate,
            frequency,
            threshold_db,
            ratio,
            attack_ms: 1.0,
            release_ms: 50.0,
            mode: DeEsserMode::SplitBand,
            enabled: true,
            listen: false,
            envelope_gain_db: 0.0,
            attack_alpha: 0.0,
            release_alpha: 0.0,
            sc_b0: 0.0,
            sc_b1: 0.0,
            sc_b2: 0.0,
            sc_a1: 0.0,
            sc_a2: 0.0,
            sc_s1_l: 0.0,
            sc_s2_l: 0.0,
            sc_s1_r: 0.0,
            sc_s2_r: 0.0,
            cos_w: 0.0,
            alpha_hs: 0.0,
            last_gain_db: 99999.0,
            hs_b0: 1.0,
            hs_b1: 0.0,
            hs_b2: 0.0,
            hs_a1: 0.0,
            hs_a2: 0.0,
            hs_s1_l: 0.0,
            hs_s2_l: 0.0,
            hs_s1_r: 0.0,
            hs_s2_r: 0.0,
        };

        de_esser.update_frequency_coeffs();
        de_esser.update_time_constants();
        de_esser.update_high_shelf(0.0);
        de_esser
    }

    /// Update precomputed filter coefficients for sidechain bandpass and dynamic high-shelf.
    fn update_frequency_coeffs(&mut self) {
        let nyquist = self.sample_rate * 0.499;
        let freq = self.frequency.clamp(500.0, nyquist);

        let omega = 2.0 * std::f32::consts::PI * freq / self.sample_rate;
        self.cos_w = omega.cos();
        let sin_w = omega.sin();

        // 1. Sidechain Bandpass filter (Q = 1.0, ~1.4 octave band around sibilance frequency)
        let q_sc = 1.0;
        let alpha_sc = sin_w / (2.0 * q_sc);
        let a0_sc = 1.0 + alpha_sc;
        let inv_a0_sc = 1.0 / a0_sc;

        self.sc_b0 = alpha_sc * inv_a0_sc;
        self.sc_b1 = 0.0;
        self.sc_b2 = -alpha_sc * inv_a0_sc;
        self.sc_a1 = -2.0 * self.cos_w * inv_a0_sc;
        self.sc_a2 = (1.0 - alpha_sc) * inv_a0_sc;

        // 2. High-shelf Q factor (0.7071 Butterworth slope)
        let q_hs = std::f32::consts::FRAC_1_SQRT_2;
        self.alpha_hs = sin_w / (2.0 * q_hs);

        // Force high-shelf coefficient recalculation
        self.last_gain_db = 99999.0;
        self.update_high_shelf(self.envelope_gain_db);
    }

    /// Update attack and release filter time constants.
    fn update_time_constants(&mut self) {
        self.attack_alpha =
            1.0 - (-1.0 / (self.attack_ms.max(0.05) * 0.001 * self.sample_rate)).exp();
        self.release_alpha =
            1.0 - (-1.0 / (self.release_ms.max(1.0) * 0.001 * self.sample_rate)).exp();
    }

    /// Update dynamic high-shelf filter coefficients given gain in dB.
    #[inline]
    fn update_high_shelf(&mut self, gain_db: f32) {
        if (gain_db - self.last_gain_db).abs() < 1e-4 {
            return;
        }
        self.last_gain_db = gain_db;

        if gain_db.abs() < 1e-5 {
            // Neutral passthrough
            self.hs_b0 = 1.0;
            self.hs_b1 = 0.0;
            self.hs_b2 = 0.0;
            self.hs_a1 = 0.0;
            self.hs_a2 = 0.0;
            return;
        }

        let a_amp = 10.0f32.powf(gain_db / 40.0);
        let sqrt_a = a_amp.sqrt();
        let two_sqrt_a_alpha = 2.0 * sqrt_a * self.alpha_hs;

        let b0 = a_amp * ((a_amp + 1.0) + (a_amp - 1.0) * self.cos_w + two_sqrt_a_alpha);
        let b1 = -2.0 * a_amp * ((a_amp - 1.0) + (a_amp + 1.0) * self.cos_w);
        let b2 = a_amp * ((a_amp + 1.0) + (a_amp - 1.0) * self.cos_w - two_sqrt_a_alpha);
        let a0 = (a_amp + 1.0) - (a_amp - 1.0) * self.cos_w + two_sqrt_a_alpha;
        let a1 = 2.0 * ((a_amp - 1.0) - (a_amp + 1.0) * self.cos_w);
        let a2 = (a_amp + 1.0) - (a_amp - 1.0) * self.cos_w - two_sqrt_a_alpha;

        if a0.abs() > 1e-15 {
            let inv_a0 = 1.0 / a0;
            self.hs_b0 = b0 * inv_a0;
            self.hs_b1 = b1 * inv_a0;
            self.hs_b2 = b2 * inv_a0;
            self.hs_a1 = a1 * inv_a0;
            self.hs_a2 = a2 * inv_a0;
        } else {
            self.hs_b0 = 1.0;
            self.hs_b1 = 0.0;
            self.hs_b2 = 0.0;
            self.hs_a1 = 0.0;
            self.hs_a2 = 0.0;
        }
    }

    /// Set sample rate in Hz.
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        if sample_rate.is_finite() && sample_rate > 1000.0 {
            self.sample_rate = sample_rate;
            self.update_frequency_coeffs();
            self.update_time_constants();
        }
    }

    /// Set detection center frequency in Hz.
    pub fn set_frequency(&mut self, frequency: f32) {
        let nyquist = self.sample_rate * 0.499;
        if frequency.is_finite() {
            self.frequency = frequency.clamp(500.0, nyquist);
            self.update_frequency_coeffs();
        }
    }

    /// Set sibilance detection threshold in dBFS.
    pub fn set_threshold_db(&mut self, threshold_db: f32) {
        if threshold_db.is_finite() {
            self.threshold_db = threshold_db.clamp(-80.0, 0.0);
        }
    }

    /// Set gain reduction ratio.
    pub fn set_ratio(&mut self, ratio: f32) {
        if ratio.is_finite() {
            self.ratio = ratio.clamp(1.0, 64.0);
        }
    }

    /// Set attack time in milliseconds.
    pub fn set_attack_ms(&mut self, attack_ms: f32) {
        if attack_ms.is_finite() && attack_ms > 0.01 {
            self.attack_ms = attack_ms.clamp(0.05, 50.0);
            self.update_time_constants();
        }
    }

    /// Set release time in milliseconds.
    pub fn set_release_ms(&mut self, release_ms: f32) {
        if release_ms.is_finite() && release_ms > 0.5 {
            self.release_ms = release_ms.clamp(1.0, 500.0);
            self.update_time_constants();
        }
    }

    /// Set operating mode (SplitBand vs WideBand).
    pub fn set_mode(&mut self, mode: DeEsserMode) {
        self.mode = mode;
    }

    /// Enable or disable sidechain audition (listen mode).
    pub fn set_listen(&mut self, listen: bool) {
        self.listen = listen;
    }

    /// Current sample rate in Hz.
    #[inline]
    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    /// Current detection frequency in Hz.
    #[inline]
    pub fn frequency(&self) -> f32 {
        self.frequency
    }

    /// Current threshold in dBFS.
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

    /// Current operational mode.
    #[inline]
    pub fn mode(&self) -> DeEsserMode {
        self.mode
    }

    /// Whether sidechain audition is active.
    #[inline]
    pub fn is_listen(&self) -> bool {
        self.listen
    }

    /// Current active gain reduction in decibels (non-negative).
    #[inline]
    pub fn gain_reduction_db(&self) -> f32 {
        -self.envelope_gain_db
    }
}

impl Default for VocalDeEsser {
    fn default() -> Self {
        Self::new(48000.0, 6000.0, -20.0, 4.0)
    }
}

impl From<VocalDeEsserConfig> for VocalDeEsser {
    fn from(cfg: VocalDeEsserConfig) -> Self {
        let mut de_esser =
            VocalDeEsser::new(cfg.sample_rate, cfg.frequency, cfg.threshold_db, cfg.ratio);
        de_esser.set_attack_ms(cfg.attack_ms);
        de_esser.set_release_ms(cfg.release_ms);
        de_esser.set_mode(cfg.mode);
        de_esser.set_enabled(cfg.enabled);
        de_esser
    }
}

impl From<VocalDeEsser> for VocalDeEsserConfig {
    fn from(de_esser: VocalDeEsser) -> Self {
        Self {
            sample_rate: de_esser.sample_rate,
            frequency: de_esser.frequency,
            threshold_db: de_esser.threshold_db,
            ratio: de_esser.ratio,
            attack_ms: de_esser.attack_ms,
            release_ms: de_esser.release_ms,
            mode: de_esser.mode,
            enabled: de_esser.enabled,
        }
    }
}

impl AudioEffect for VocalDeEsser {
    #[inline]
    fn process_frame(&mut self, frame: AudioFrame) -> AudioFrame {
        let frame = frame.sanitize();
        if !self.enabled {
            return frame;
        }

        // 1. Process sidechain bandpass filter for left and right channels
        let sc_yl = self.sc_b0 * frame.left + self.sc_s1_l;
        self.sc_s1_l = self.sc_b1 * frame.left - self.sc_a1 * sc_yl + self.sc_s2_l;
        self.sc_s2_l = self.sc_b2 * frame.left - self.sc_a2 * sc_yl;

        let sc_yr = self.sc_b0 * frame.right + self.sc_s1_r;
        self.sc_s1_r = self.sc_b1 * frame.right - self.sc_a1 * sc_yr + self.sc_s2_r;
        self.sc_s2_r = self.sc_b2 * frame.right - self.sc_a2 * sc_yr;

        // Subnormal & non-finite protection for sidechain registers
        if !self.sc_s1_l.is_finite() || self.sc_s1_l.abs() < 1e-25 {
            self.sc_s1_l = 0.0;
        }
        if !self.sc_s2_l.is_finite() || self.sc_s2_l.abs() < 1e-25 {
            self.sc_s2_l = 0.0;
        }
        if !self.sc_s1_r.is_finite() || self.sc_s1_r.abs() < 1e-25 {
            self.sc_s1_r = 0.0;
        }
        if !self.sc_s2_r.is_finite() || self.sc_s2_r.abs() < 1e-25 {
            self.sc_s2_r = 0.0;
        }

        // Sidechain audition mode: listen directly to isolated sibilant band
        if self.listen {
            return AudioFrame::new(sc_yl, sc_yr).sanitize();
        }

        // 2. Sidechain peak level detection in dBFS
        let sc_peak = sc_yl.abs().max(sc_yr.abs()).max(1e-9);
        let sc_db = 20.0 * sc_peak.log10();

        // 3. Static gain reduction calculation
        let target_gain_db = if sc_db > self.threshold_db {
            let overshoot = sc_db - self.threshold_db;
            let reduction = overshoot * (1.0 - 1.0 / self.ratio);
            -reduction
        } else {
            0.0
        };

        // 4. Smooth envelope follower
        if target_gain_db < self.envelope_gain_db {
            self.envelope_gain_db += self.attack_alpha * (target_gain_db - self.envelope_gain_db);
        } else {
            self.envelope_gain_db += self.release_alpha * (target_gain_db - self.envelope_gain_db);
        }

        if self.envelope_gain_db.abs() < 1e-6 {
            self.envelope_gain_db = 0.0;
        }

        // 5. Apply gain reduction according to selected mode
        let out = match self.mode {
            DeEsserMode::WideBand => {
                let linear_gain = 10.0f32.powf(self.envelope_gain_db / 20.0);
                frame * linear_gain
            }
            DeEsserMode::SplitBand => {
                self.update_high_shelf(self.envelope_gain_db);

                let yl = self.hs_b0 * frame.left + self.hs_s1_l;
                self.hs_s1_l = self.hs_b1 * frame.left - self.hs_a1 * yl + self.hs_s2_l;
                self.hs_s2_l = self.hs_b2 * frame.left - self.hs_a2 * yl;

                let yr = self.hs_b0 * frame.right + self.hs_s1_r;
                self.hs_s1_r = self.hs_b1 * frame.right - self.hs_a1 * yr + self.hs_s2_r;
                self.hs_s2_r = self.hs_b2 * frame.right - self.hs_a2 * yr;

                // Subnormal protection for high-shelf registers
                if !self.hs_s1_l.is_finite() || self.hs_s1_l.abs() < 1e-25 {
                    self.hs_s1_l = 0.0;
                }
                if !self.hs_s2_l.is_finite() || self.hs_s2_l.abs() < 1e-25 {
                    self.hs_s2_l = 0.0;
                }
                if !self.hs_s1_r.is_finite() || self.hs_s1_r.abs() < 1e-25 {
                    self.hs_s1_r = 0.0;
                }
                if !self.hs_s2_r.is_finite() || self.hs_s2_r.abs() < 1e-25 {
                    self.hs_s2_r = 0.0;
                }

                AudioFrame::new(yl, yr)
            }
        };

        out.sanitize()
    }

    #[inline]
    fn reset(&mut self) {
        self.envelope_gain_db = 0.0;
        self.sc_s1_l = 0.0;
        self.sc_s2_l = 0.0;
        self.sc_s1_r = 0.0;
        self.sc_s2_r = 0.0;
        self.hs_s1_l = 0.0;
        self.hs_s2_l = 0.0;
        self.hs_s1_r = 0.0;
        self.hs_s2_r = 0.0;
        self.update_high_shelf(0.0);
    }

    fn name(&self) -> &'static str {
        "Vocal De-Esser"
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
