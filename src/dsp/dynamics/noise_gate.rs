//! Real-time noise gate with peak/RMS envelope detection, hysteresis, and attack/release state machine.

use crate::core::frame::AudioFrame;
use crate::dsp::traits::AudioEffect;

/// Operational state of the noise gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum GateState {
    /// Gate is fully closed, muting audio.
    Closed,
    /// Gate is ramping open (attack phase).
    Opening,
    /// Gate is fully open, passing audio untouched.
    Open,
    /// Signal fell below threshold; hold timer counting down.
    Holding,
    /// Gate is ramping closed (release phase).
    Closing,
}

/// Noise gate effect node for eliminating background hum, hiss, and line noise.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NoiseGate {
    sample_rate: f32,
    threshold_db: f32,
    threshold_linear: f32,
    attack_ms: f32,
    hold_ms: f32,
    release_ms: f32,
    state: GateState,
    current_gain: f32,
    hold_counter: usize,
    enabled: bool,
}

impl NoiseGate {
    /// Create a new NoiseGate with specified sample rate, threshold (dB), attack (ms), and release (ms).
    pub fn new(sample_rate: f32, threshold_db: f32, attack_ms: f32, release_ms: f32) -> Self {
        let sample_rate = if sample_rate.is_finite() && sample_rate > 1000.0 {
            sample_rate
        } else {
            48000.0
        };
        let threshold_db = if threshold_db.is_finite() {
            threshold_db.clamp(-100.0, 0.0)
        } else {
            -40.0
        };
        let attack_ms = if attack_ms.is_finite() && attack_ms > 0.01 {
            attack_ms
        } else {
            5.0
        };
        let release_ms = if release_ms.is_finite() && release_ms > 0.1 {
            release_ms
        } else {
            50.0
        };

        let threshold_linear = 10.0f32.powf(threshold_db / 20.0);

        Self {
            sample_rate,
            threshold_db,
            threshold_linear,
            attack_ms,
            hold_ms: 10.0,
            release_ms,
            state: GateState::Closed,
            current_gain: 0.0,
            hold_counter: 0,
            enabled: true,
        }
    }

    /// Set runtime parameters for the noise gate.
    pub fn set_params(
        &mut self,
        sample_rate: f32,
        threshold_db: f32,
        attack_ms: f32,
        release_ms: f32,
    ) {
        self.set_sample_rate(sample_rate);
        self.set_threshold_db(threshold_db);
        self.set_attack_ms(attack_ms);
        self.set_release_ms(release_ms);
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
            self.threshold_db = threshold_db.clamp(-100.0, 0.0);
            self.threshold_linear = 10.0f32.powf(self.threshold_db / 20.0);
        }
    }

    /// Set attack time in milliseconds.
    pub fn set_attack_ms(&mut self, attack_ms: f32) {
        if attack_ms.is_finite() && attack_ms > 0.01 {
            self.attack_ms = attack_ms;
        }
    }

    /// Set hold time in milliseconds before release begins.
    pub fn set_hold_ms(&mut self, hold_ms: f32) {
        if hold_ms.is_finite() && hold_ms >= 0.0 {
            self.hold_ms = hold_ms;
        }
    }

    /// Set release time in milliseconds.
    pub fn set_release_ms(&mut self, release_ms: f32) {
        if release_ms.is_finite() && release_ms > 0.1 {
            self.release_ms = release_ms;
        }
    }

    /// Current threshold in dB.
    #[inline]
    pub fn threshold_db(&self) -> f32 {
        self.threshold_db
    }

    /// Current attack time in ms.
    #[inline]
    pub fn attack_ms(&self) -> f32 {
        self.attack_ms
    }

    /// Current hold time in ms.
    #[inline]
    pub fn hold_ms(&self) -> f32 {
        self.hold_ms
    }

    /// Current release time in ms.
    #[inline]
    pub fn release_ms(&self) -> f32 {
        self.release_ms
    }

    /// Current sample rate in Hz.
    #[inline]
    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    /// Current internal gate gain factor (0.0 = fully muted, 1.0 = fully open).
    #[inline]
    pub fn current_gain(&self) -> f32 {
        self.current_gain
    }

    /// Current gate state.
    #[inline]
    pub fn state(&self) -> GateState {
        self.state
    }

    /// Returns true if the gate is passing audio (Open, Opening, or Holding).
    #[inline]
    pub fn is_open(&self) -> bool {
        matches!(
            self.state,
            GateState::Open | GateState::Opening | GateState::Holding
        )
    }
}

impl Default for NoiseGate {
    fn default() -> Self {
        Self::new(48000.0, -40.0, 5.0, 50.0)
    }
}

impl AudioEffect for NoiseGate {
    #[inline]
    fn process_frame(&mut self, frame: AudioFrame) -> AudioFrame {
        let frame = frame.sanitize();
        if !self.enabled {
            return frame;
        }

        let peak = frame.peak();
        let open_threshold = self.threshold_linear;
        // 2 dB hysteresis below open threshold to prevent fluttering around threshold boundary
        let close_threshold = self.threshold_linear * 0.7943;

        let attack_step = 1.0 / (self.attack_ms.max(0.1) * 0.001 * self.sample_rate);
        let release_step = 1.0 / (self.release_ms.max(1.0) * 0.001 * self.sample_rate);

        match self.state {
            GateState::Closed => {
                if peak >= open_threshold {
                    self.state = GateState::Opening;
                    self.current_gain = (self.current_gain + attack_step).min(1.0);
                    if self.current_gain >= 1.0 {
                        self.state = GateState::Open;
                    }
                } else {
                    self.current_gain = 0.0;
                }
            }
            GateState::Opening => {
                self.current_gain = (self.current_gain + attack_step).min(1.0);
                if self.current_gain >= 1.0 {
                    self.current_gain = 1.0;
                    self.state = GateState::Open;
                }
            }
            GateState::Open => {
                self.current_gain = 1.0;
                if peak < close_threshold {
                    self.state = GateState::Holding;
                    self.hold_counter = (self.hold_ms * 0.001 * self.sample_rate).round() as usize;
                }
            }
            GateState::Holding => {
                self.current_gain = 1.0;
                if peak >= open_threshold {
                    self.state = GateState::Open;
                } else if self.hold_counter > 0 {
                    self.hold_counter -= 1;
                } else {
                    self.state = GateState::Closing;
                }
            }
            GateState::Closing => {
                if peak >= open_threshold {
                    self.state = GateState::Opening;
                    self.current_gain = (self.current_gain + attack_step).min(1.0);
                } else {
                    self.current_gain = (self.current_gain - release_step).max(0.0);
                    if self.current_gain <= 0.0 {
                        self.current_gain = 0.0;
                        self.state = GateState::Closed;
                    }
                }
            }
        }

        // Subnormal guard
        if self.current_gain < 1e-6 {
            self.current_gain = 0.0;
        }

        let out = frame * self.current_gain;
        out.sanitize()
    }

    #[inline]
    fn reset(&mut self) {
        self.state = GateState::Closed;
        self.current_gain = 0.0;
        self.hold_counter = 0;
    }

    fn name(&self) -> &'static str {
        "Noise Gate"
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
