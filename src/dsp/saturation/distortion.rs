//! Asymmetric diode hard-clipping distortion with pre-emphasis high-pass filtering and variable tone control.

use crate::core::frame::AudioFrame;
use crate::dsp::traits::AudioEffect;

/// Asymmetric diode distortion node providing aggressive saturation and even/odd harmonic textures.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Distortion {
    drive: f32,
    tone: f32,
    level: f32,
    sample_rate: f32,
    pre_filter_left: f32,
    pre_filter_right: f32,
    tone_filter_left: f32,
    tone_filter_right: f32,
    enabled: bool,
}

impl Distortion {
    /// Positive diode clipping threshold (+0.7V silicon barrier).
    pub const POS_THRESHOLD: f32 = 0.7;
    /// Negative diode clipping threshold (-0.35V asymmetric germanium / single-diode barrier).
    pub const NEG_THRESHOLD: f32 = -0.35;

    /// Create a new Distortion effect with drive, tone, and level controls.
    pub fn new(drive: f32, tone: f32, level: f32) -> Self {
        Self::with_sample_rate(drive, tone, level, 48000.0)
    }

    /// Create a Distortion effect with an explicit sample rate.
    pub fn with_sample_rate(drive: f32, tone: f32, level: f32, sample_rate: f32) -> Self {
        let drive = if drive.is_finite() && drive >= 0.0 {
            drive.clamp(0.0, 100.0)
        } else {
            8.0
        };
        let tone = if tone.is_finite() {
            tone.clamp(0.0, 1.0)
        } else {
            0.5
        };
        let level = if level.is_finite() && level >= 0.0 {
            level.clamp(0.0, 10.0)
        } else {
            0.7
        };
        let sample_rate = if sample_rate.is_finite() && sample_rate > 1000.0 {
            sample_rate
        } else {
            48000.0
        };

        Self {
            drive,
            tone,
            level,
            sample_rate,
            pre_filter_left: 0.0,
            pre_filter_right: 0.0,
            tone_filter_left: 0.0,
            tone_filter_right: 0.0,
            enabled: true,
        }
    }

    /// Set distortion parameters dynamically.
    pub fn set_params(&mut self, drive: f32, tone: f32, level: f32) {
        self.set_drive(drive);
        self.set_tone(tone);
        self.set_level(level);
    }

    /// Set drive multiplier (pre-clip gain).
    pub fn set_drive(&mut self, drive: f32) {
        if drive.is_finite() && drive >= 0.0 {
            self.drive = drive.clamp(0.0, 100.0);
        }
    }

    /// Set tone control in `[0.0, 1.0]` (0.0 = dark low-pass, 1.0 = sharp bright).
    pub fn set_tone(&mut self, tone: f32) {
        if tone.is_finite() {
            self.tone = tone.clamp(0.0, 1.0);
        }
    }

    /// Set output level.
    pub fn set_level(&mut self, level: f32) {
        if level.is_finite() && level >= 0.0 {
            self.level = level.clamp(0.0, 10.0);
        }
    }

    /// Set sample rate in Hz.
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        if sample_rate.is_finite() && sample_rate > 1000.0 {
            self.sample_rate = sample_rate;
        }
    }

    /// Current drive setting.
    #[inline]
    pub fn drive(&self) -> f32 {
        self.drive
    }

    /// Current tone setting.
    #[inline]
    pub fn tone(&self) -> f32 {
        self.tone
    }

    /// Current output level.
    #[inline]
    pub fn level(&self) -> f32 {
        self.level
    }

    /// Current sample rate in Hz.
    #[inline]
    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    #[inline]
    fn process_single_channel(
        sample: f32,
        pre_state: &mut f32,
        tone_state: &mut f32,
        drive: f32,
        tone: f32,
        level: f32,
        sample_rate: f32,
    ) -> f32 {
        let dt = 1.0 / sample_rate;

        // 1. Pre-emphasis high-pass filter (720 Hz cutoff) to tighten low-end before high-gain clipping
        let pre_rc = 1.0 / (2.0 * std::f32::consts::PI * 720.0);
        let pre_alpha = (dt / (pre_rc + dt)).clamp(0.01, 1.0);
        *pre_state += pre_alpha * (sample - *pre_state);
        let high_passed = sample - *pre_state;
        let pre_emphasized = 0.35 * sample + 0.65 * high_passed;

        // 2. High-gain amplification
        let driven = pre_emphasized * drive;

        // 3. Asymmetric diode clipping (e.g. 2 x 1N4148 positive, 1 x 1N4148 negative)
        let clipped = if driven > Self::POS_THRESHOLD {
            Self::POS_THRESHOLD + ((driven - Self::POS_THRESHOLD) * 0.5).tanh() * 0.15
        } else if driven < Self::NEG_THRESHOLD {
            Self::NEG_THRESHOLD + ((driven - Self::NEG_THRESHOLD) * 0.5).tanh() * 0.1
        } else {
            driven
        };

        // 4. Tone low-pass filter (800 Hz to 11000 Hz)
        let tone_cutoff = 800.0 + tone.clamp(0.0, 1.0) * 10200.0;
        let tone_rc = 1.0 / (2.0 * std::f32::consts::PI * tone_cutoff);
        let tone_alpha = (dt / (tone_rc + dt)).clamp(0.01, 1.0);
        *tone_state += tone_alpha * (clipped - *tone_state);

        // Subnormal guard
        if tone_state.abs() < 1e-25 {
            *tone_state = 0.0;
        }

        *tone_state * level
    }
}

impl Default for Distortion {
    fn default() -> Self {
        Self::new(8.0, 0.5, 0.7)
    }
}

impl AudioEffect for Distortion {
    #[inline]
    fn process_frame(&mut self, frame: AudioFrame) -> AudioFrame {
        let frame = frame.sanitize();
        if !self.enabled {
            return frame;
        }

        let left = Self::process_single_channel(
            frame.left,
            &mut self.pre_filter_left,
            &mut self.tone_filter_left,
            self.drive,
            self.tone,
            self.level,
            self.sample_rate,
        );
        let right = Self::process_single_channel(
            frame.right,
            &mut self.pre_filter_right,
            &mut self.tone_filter_right,
            self.drive,
            self.tone,
            self.level,
            self.sample_rate,
        );

        AudioFrame::new(left, right).clamp(-2.0, 2.0).sanitize()
    }

    #[inline]
    fn reset(&mut self) {
        self.pre_filter_left = 0.0;
        self.pre_filter_right = 0.0;
        self.tone_filter_left = 0.0;
        self.tone_filter_right = 0.0;
    }

    fn name(&self) -> &'static str {
        "Distortion"
    }

    #[inline]
    fn is_enabled(&self) -> bool {
        self.enabled
    }

    #[inline]
    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    #[inline]
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    #[inline]
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
