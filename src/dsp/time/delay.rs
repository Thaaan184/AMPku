//! Stereo delay effect with circular buffer, fractional interpolation,
//! low-pass feedback damping, and cross-channel ping-pong spread.

use crate::core::frame::AudioFrame;
use crate::dsp::traits::AudioEffect;

/// Maximum default delay buffer allocation in milliseconds (2.5 seconds).
pub const DEFAULT_MAX_DELAY_MS: f32 = 2500.0;

/// Serializable configuration state for `StereoDelay`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StereoDelayConfig {
    pub sample_rate: f32,
    pub delay_ms: f32,
    pub feedback: f32,
    pub mix: f32,
    pub damping: f32,
    pub ping_pong: bool,
    pub stereo_spread: f32,
    pub enabled: bool,
}

/// Stereo ping-pong delay featuring fractional delay interpolation,
/// feedback loop low-pass tone damping, and stereo spread control.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(from = "StereoDelayConfig", into = "StereoDelayConfig")]
pub struct StereoDelay {
    sample_rate: f32,
    delay_ms: f32,
    feedback: f32,
    mix: f32,
    damping: f32,
    ping_pong: bool,
    stereo_spread: f32,
    enabled: bool,

    #[serde(skip)]
    buffer_left: Vec<f32>,
    #[serde(skip)]
    buffer_right: Vec<f32>,
    #[serde(skip)]
    buffer_len: usize,
    #[serde(skip)]
    write_pos: usize,
    #[serde(skip)]
    damped_left: f32,
    #[serde(skip)]
    damped_right: f32,
}

impl PartialEq for StereoDelay {
    fn eq(&self, other: &Self) -> bool {
        self.sample_rate == other.sample_rate
            && self.delay_ms == other.delay_ms
            && self.feedback == other.feedback
            && self.mix == other.mix
            && self.damping == other.damping
            && self.ping_pong == other.ping_pong
            && self.stereo_spread == other.stereo_spread
            && self.enabled == other.enabled
    }
}

impl Default for StereoDelay {
    fn default() -> Self {
        Self::new(48000.0, 350.0, 0.4, 0.3)
    }
}

impl From<StereoDelayConfig> for StereoDelay {
    fn from(config: StereoDelayConfig) -> Self {
        let mut delay = Self::with_options(
            config.sample_rate,
            config.delay_ms,
            config.feedback,
            config.mix,
            config.damping,
            config.ping_pong,
            config.stereo_spread,
        );
        delay.set_enabled(config.enabled);
        delay
    }
}

impl From<StereoDelay> for StereoDelayConfig {
    fn from(delay: StereoDelay) -> Self {
        StereoDelayConfig {
            sample_rate: delay.sample_rate,
            delay_ms: delay.delay_ms,
            feedback: delay.feedback,
            mix: delay.mix,
            damping: delay.damping,
            ping_pong: delay.ping_pong,
            stereo_spread: delay.stereo_spread,
            enabled: delay.enabled,
        }
    }
}

impl StereoDelay {
    /// Create a new StereoDelay with sample rate, delay time (ms), feedback (0.0..1.0), and mix (0.0..1.0).
    pub fn new(sample_rate: f32, delay_ms: f32, feedback: f32, mix: f32) -> Self {
        Self::with_options(sample_rate, delay_ms, feedback, mix, 0.0, false, 0.0)
    }

    /// Create a StereoDelay with explicit configuration options.
    pub fn with_options(
        sample_rate: f32,
        delay_ms: f32,
        feedback: f32,
        mix: f32,
        damping: f32,
        ping_pong: bool,
        stereo_spread: f32,
    ) -> Self {
        let sr = if sample_rate.is_finite() && sample_rate > 1000.0 {
            sample_rate
        } else {
            48000.0
        };
        let max_ms = delay_ms.max(DEFAULT_MAX_DELAY_MS).max(100.0);
        let buffer_len = ((max_ms * 0.001 * sr).ceil() as usize).max(1024) + 16;

        let mut delay = Self {
            sample_rate: sr,
            delay_ms: 100.0,
            feedback: 0.0,
            mix: 0.5,
            damping: 0.0,
            ping_pong,
            stereo_spread: 0.0,
            enabled: true,
            buffer_left: vec![0.0; buffer_len],
            buffer_right: vec![0.0; buffer_len],
            buffer_len,
            write_pos: 0,
            damped_left: 0.0,
            damped_right: 0.0,
        };

        delay.set_delay_ms(delay_ms);
        delay.set_feedback(feedback);
        delay.set_mix(mix);
        delay.set_damping(damping);
        delay.set_stereo_spread(stereo_spread);

        delay
    }

    /// Set runtime parameters.
    pub fn set_params(
        &mut self,
        delay_ms: f32,
        feedback: f32,
        mix: f32,
        damping: f32,
        ping_pong: bool,
        stereo_spread: f32,
    ) {
        self.set_delay_ms(delay_ms);
        self.set_feedback(feedback);
        self.set_mix(mix);
        self.set_damping(damping);
        self.set_ping_pong(ping_pong);
        self.set_stereo_spread(stereo_spread);
    }

    /// Delay time in milliseconds.
    #[inline]
    pub fn delay_ms(&self) -> f32 {
        self.delay_ms
    }

    /// Set delay time in milliseconds (clamped to 0.1ms .. max buffer limit).
    pub fn set_delay_ms(&mut self, delay_ms: f32) {
        if delay_ms.is_finite() && delay_ms > 0.0 {
            let max_supported = (self.buffer_len as f32 - 8.0) / self.sample_rate * 1000.0;
            self.delay_ms = delay_ms.clamp(0.1, max_supported);
        }
    }

    /// Feedback ratio in range `[0.0, 0.99]`.
    #[inline]
    pub fn feedback(&self) -> f32 {
        self.feedback
    }

    /// Set feedback ratio (clamped to `[0.0, 0.99]` to prevent runaway feedback).
    pub fn set_feedback(&mut self, feedback: f32) {
        if feedback.is_finite() {
            self.feedback = feedback.clamp(0.0, 0.99);
        }
    }

    /// Dry/wet mix in range `[0.0, 1.0]` (0.0 = 100% dry, 1.0 = 100% wet).
    #[inline]
    pub fn mix(&self) -> f32 {
        self.mix
    }

    /// Set dry/wet mix.
    pub fn set_mix(&mut self, mix: f32) {
        if mix.is_finite() {
            self.mix = mix.clamp(0.0, 1.0);
        }
    }

    /// Low-pass feedback damping in range `[0.0, 0.95]`.
    #[inline]
    pub fn damping(&self) -> f32 {
        self.damping
    }

    /// Set low-pass damping for repeats.
    pub fn set_damping(&mut self, damping: f32) {
        if damping.is_finite() {
            self.damping = damping.clamp(0.0, 0.95);
        }
    }

    /// Return true if ping-pong delay is active.
    #[inline]
    pub fn ping_pong(&self) -> bool {
        self.ping_pong
    }

    /// Enable or disable ping-pong cross-channel bouncing.
    pub fn set_ping_pong(&mut self, ping_pong: bool) {
        self.ping_pong = ping_pong;
    }

    /// Stereo spread offset in range `[0.0, 1.0]`.
    #[inline]
    pub fn stereo_spread(&self) -> f32 {
        self.stereo_spread
    }

    /// Set stereo spread factor.
    pub fn set_stereo_spread(&mut self, spread: f32) {
        if spread.is_finite() {
            self.stereo_spread = spread.clamp(0.0, 1.0);
        }
    }

    /// Current sample rate in Hz.
    #[inline]
    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    /// Update sample rate, re-allocating delay line buffers if needed.
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        if sample_rate.is_finite()
            && sample_rate > 1000.0
            && (sample_rate - self.sample_rate).abs() > 1.0
        {
            self.sample_rate = sample_rate;
            let max_ms = self.delay_ms.max(DEFAULT_MAX_DELAY_MS).max(100.0);
            let new_len = ((max_ms * 0.001 * sample_rate).ceil() as usize).max(1024) + 16;
            self.buffer_left = vec![0.0; new_len];
            self.buffer_right = vec![0.0; new_len];
            self.buffer_len = new_len;
            self.write_pos = 0;
            self.damped_left = 0.0;
            self.damped_right = 0.0;
        }
    }

    /// Read linearly interpolated fractional sample from a circular buffer.
    #[inline]
    fn read_interpolated(
        buffer: &[f32],
        buf_len: usize,
        write_pos: usize,
        delay_samples: f32,
    ) -> f32 {
        let max_delay = (buf_len - 4) as f32;
        let delay_clamped = delay_samples.clamp(1.0, max_delay);
        let delay_int = delay_clamped.floor() as usize;
        let frac = delay_clamped - (delay_int as f32);

        let idx0 = (write_pos + buf_len - delay_int) % buf_len;
        let idx1 = (write_pos + buf_len - delay_int - 1) % buf_len;

        buffer[idx0] * (1.0 - frac) + buffer[idx1] * frac
    }
}

impl AudioEffect for StereoDelay {
    #[inline]
    fn process_frame(&mut self, frame: AudioFrame) -> AudioFrame {
        let clean_frame = frame.sanitize();
        if !self.enabled {
            return clean_frame;
        }

        let base_delay_samples = self.delay_ms * 0.001 * self.sample_rate;
        let (delay_samples_l, delay_samples_r) = if self.ping_pong {
            (
                base_delay_samples,
                base_delay_samples * (1.0 + self.stereo_spread * 0.2),
            )
        } else {
            (
                base_delay_samples,
                base_delay_samples * (1.0 + self.stereo_spread * 0.25),
            )
        };

        // Read fractional delayed samples
        let delayed_l = Self::read_interpolated(
            &self.buffer_left,
            self.buffer_len,
            self.write_pos,
            delay_samples_l,
        );
        let delayed_r = Self::read_interpolated(
            &self.buffer_right,
            self.buffer_len,
            self.write_pos,
            delay_samples_r,
        );

        // Low-pass filter damping on feedback path
        self.damped_left = delayed_l * (1.0 - self.damping) + self.damped_left * self.damping;
        self.damped_right = delayed_r * (1.0 - self.damping) + self.damped_right * self.damping;

        // Subnormal / denormal arithmetic protection
        if self.damped_left.abs() < 1e-15 {
            self.damped_left = 0.0;
        }
        if self.damped_right.abs() < 1e-15 {
            self.damped_right = 0.0;
        }

        // Compute feedback and write inputs
        let (write_l, write_r) = if self.ping_pong {
            let mono_in = (clean_frame.left + clean_frame.right) * 0.5;
            (
                mono_in + self.feedback * self.damped_right,
                self.feedback * self.damped_left,
            )
        } else {
            (
                clean_frame.left + self.feedback * self.damped_left,
                clean_frame.right + self.feedback * self.damped_right,
            )
        };

        // Write to pre-allocated circular buffers
        self.buffer_left[self.write_pos] = write_l;
        self.buffer_right[self.write_pos] = write_r;
        self.write_pos = (self.write_pos + 1) % self.buffer_len;

        // Dry / wet mixing
        let out_left = clean_frame.left * (1.0 - self.mix) + delayed_l * self.mix;
        let out_right = clean_frame.right * (1.0 - self.mix) + delayed_r * self.mix;

        AudioFrame::new(out_left, out_right)
    }

    fn reset(&mut self) {
        self.buffer_left.fill(0.0);
        self.buffer_right.fill(0.0);
        self.write_pos = 0;
        self.damped_left = 0.0;
        self.damped_right = 0.0;
    }

    fn name(&self) -> &'static str {
        "Stereo Delay"
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

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
