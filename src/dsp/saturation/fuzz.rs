//! Square-wave wavefolding fuzz effect with high-gain transistor saturation.

use crate::core::frame::AudioFrame;
use crate::dsp::traits::AudioEffect;

/// Aggressive fuzz effect combining high-gain transistor clipping and multi-stage wavefolding.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Fuzz {
    gain: f32,
    tone: f32,
    level: f32,
    sample_rate: f32,
    tone_filter_left: f32,
    tone_filter_right: f32,
    enabled: bool,
}

impl Fuzz {
    /// Create a new Fuzz effect with gain, tone, and level controls.
    pub fn new(gain: f32, tone: f32, level: f32) -> Self {
        Self::with_sample_rate(gain, tone, level, 48000.0)
    }

    /// Create a Fuzz effect with an explicit sample rate.
    pub fn with_sample_rate(gain: f32, tone: f32, level: f32, sample_rate: f32) -> Self {
        let gain = if gain.is_finite() && gain >= 0.0 {
            gain.clamp(0.0, 100.0)
        } else {
            12.0
        };
        let tone = if tone.is_finite() {
            tone.clamp(0.0, 1.0)
        } else {
            0.5
        };
        let level = if level.is_finite() && level >= 0.0 {
            level.clamp(0.0, 10.0)
        } else {
            0.6
        };
        let sample_rate = if sample_rate.is_finite() && sample_rate > 1000.0 {
            sample_rate
        } else {
            48000.0
        };

        Self {
            gain,
            tone,
            level,
            sample_rate,
            tone_filter_left: 0.0,
            tone_filter_right: 0.0,
            enabled: true,
        }
    }

    /// Set fuzz parameters dynamically.
    pub fn set_params(&mut self, gain: f32, tone: f32, level: f32) {
        self.set_gain(gain);
        self.set_tone(tone);
        self.set_level(level);
    }

    /// Set gain / fuzz intensity (higher values push harder into wavefolding and transistor saturation).
    pub fn set_gain(&mut self, gain: f32) {
        if gain.is_finite() && gain >= 0.0 {
            self.gain = gain.clamp(0.0, 100.0);
        }
    }

    /// Set tone control in `[0.0, 1.0]` (0.0 = dark/wooly fuzz, 1.0 = sharp biting buzz).
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

    /// Current gain setting.
    #[inline]
    pub fn gain(&self) -> f32 {
        self.gain
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

    /// Multi-stage wavefolder folding signal exceeding `[-1.0, 1.0]` boundaries.
    #[inline]
    fn fold(sample: f32) -> f32 {
        let mut v = sample;
        // Fold up to 4 times for high-amplitude inputs
        for _ in 0..4 {
            if v > 1.0 {
                v = 2.0 - v;
            } else if v < -1.0 {
                v = -2.0 - v;
            } else {
                break;
            }
        }
        v.clamp(-1.0, 1.0)
    }

    #[inline]
    fn process_single_channel(
        sample: f32,
        tone_state: &mut f32,
        gain: f32,
        tone: f32,
        level: f32,
        sample_rate: f32,
    ) -> f32 {
        // 1. High-gain transistor pre-drive with asymmetric bias
        let driven = sample * gain;
        let biased = driven + 0.05 * driven * driven.abs().min(1.0);

        // 2. Wavefold the amplified signal
        let folded = Self::fold(biased);

        // 3. High-gain transistor saturation into square-wave rails
        let squared = (folded * 2.0).clamp(-1.0, 1.0);

        // 4. Tone low-pass filter (500 Hz to 9000 Hz)
        let cutoff = 500.0 + tone.clamp(0.0, 1.0) * 8500.0;
        let dt = 1.0 / sample_rate;
        let rc = 1.0 / (2.0 * std::f32::consts::PI * cutoff);
        let alpha = (dt / (rc + dt)).clamp(0.01, 1.0);

        *tone_state += alpha * (squared - *tone_state);

        // Subnormal guard
        if tone_state.abs() < 1e-25 {
            *tone_state = 0.0;
        }

        *tone_state * level
    }
}

impl Default for Fuzz {
    fn default() -> Self {
        Self::new(12.0, 0.5, 0.6)
    }
}

impl AudioEffect for Fuzz {
    #[inline]
    fn process_frame(&mut self, frame: AudioFrame) -> AudioFrame {
        let frame = frame.sanitize();
        if !self.enabled {
            return frame;
        }

        let left = Self::process_single_channel(
            frame.left,
            &mut self.tone_filter_left,
            self.gain,
            self.tone,
            self.level,
            self.sample_rate,
        );
        let right = Self::process_single_channel(
            frame.right,
            &mut self.tone_filter_right,
            self.gain,
            self.tone,
            self.level,
            self.sample_rate,
        );

        AudioFrame::new(left, right).clamp(-2.0, 2.0).sanitize()
    }

    #[inline]
    fn reset(&mut self) {
        self.tone_filter_left = 0.0;
        self.tone_filter_right = 0.0;
    }

    fn name(&self) -> &'static str {
        "Fuzz"
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
