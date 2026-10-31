//! Hyperbolic tangent soft-clipping overdrive with post-saturation tone low-pass filtering.

use crate::core::frame::AudioFrame;
use crate::dsp::traits::AudioEffect;

/// Warm overdrive effect featuring tanh soft-clipping saturation and dynamic tone low-pass control.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Overdrive {
    drive: f32,
    tone: f32,
    level: f32,
    sample_rate: f32,
    filter_left: f32,
    filter_right: f32,
    enabled: bool,
}

impl Overdrive {
    /// Create a new Overdrive effect node with drive, tone, and level controls.
    pub fn new(drive: f32, tone: f32, level: f32) -> Self {
        Self::with_sample_rate(drive, tone, level, 48000.0)
    }

    /// Create an Overdrive effect node with an explicit sample rate.
    pub fn with_sample_rate(drive: f32, tone: f32, level: f32, sample_rate: f32) -> Self {
        let drive = if drive.is_finite() && drive >= 0.0 {
            drive.clamp(0.0, 50.0)
        } else {
            4.0
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
            filter_left: 0.0,
            filter_right: 0.0,
            enabled: true,
        }
    }

    /// Set overdrive parameters dynamically.
    pub fn set_params(&mut self, drive: f32, tone: f32, level: f32) {
        self.set_drive(drive);
        self.set_tone(tone);
        self.set_level(level);
    }

    /// Set drive multiplier (higher values produce more saturation and sustain).
    pub fn set_drive(&mut self, drive: f32) {
        if drive.is_finite() && drive >= 0.0 {
            self.drive = drive.clamp(0.0, 50.0);
        }
    }

    /// Set tone control in `[0.0, 1.0]` (0.0 = dark/warm low-pass, 1.0 = bright/open).
    pub fn set_tone(&mut self, tone: f32) {
        if tone.is_finite() {
            self.tone = tone.clamp(0.0, 1.0);
        }
    }

    /// Set output level multiplier.
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
        filter_state: &mut f32,
        drive: f32,
        tone: f32,
        level: f32,
        sample_rate: f32,
    ) -> f32 {
        // Tanh soft-clipping saturation curve
        let saturated = (sample * drive).tanh();

        // 1-pole tone low-pass filter (1000 Hz to 12000 Hz)
        let cutoff = 1000.0 + tone.clamp(0.0, 1.0) * 11000.0;
        let dt = 1.0 / sample_rate;
        let rc = 1.0 / (2.0 * std::f32::consts::PI * cutoff);
        let alpha = (dt / (rc + dt)).clamp(0.01, 1.0);

        *filter_state += alpha * (saturated - *filter_state);

        // Subnormal guard
        if filter_state.abs() < 1e-25 {
            *filter_state = 0.0;
        }

        *filter_state * level
    }
}

impl Default for Overdrive {
    fn default() -> Self {
        Self::new(4.0, 0.5, 0.7)
    }
}

impl AudioEffect for Overdrive {
    #[inline]
    fn process_frame(&mut self, frame: AudioFrame) -> AudioFrame {
        let frame = frame.sanitize();
        if !self.enabled {
            return frame;
        }

        let left = Self::process_single_channel(
            frame.left,
            &mut self.filter_left,
            self.drive,
            self.tone,
            self.level,
            self.sample_rate,
        );
        let right = Self::process_single_channel(
            frame.right,
            &mut self.filter_right,
            self.drive,
            self.tone,
            self.level,
            self.sample_rate,
        );

        AudioFrame::new(left, right).clamp(-2.0, 2.0).sanitize()
    }

    #[inline]
    fn reset(&mut self) {
        self.filter_left = 0.0;
        self.filter_right = 0.0;
    }

    fn name(&self) -> &'static str {
        "Overdrive"
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
