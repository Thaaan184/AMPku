//! Direct Form II Transposed biquad filter implementation based on Robert Bristow-Johnson's Audio EQ Cookbook.

use crate::core::frame::AudioFrame;
use crate::dsp::traits::AudioEffect;

/// Biquad filter topology type based on Robert Bristow-Johnson Audio EQ Cookbook.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FilterType {
    /// Low-pass filter (attenuates frequencies above cutoff).
    LowPass,
    /// High-pass filter (attenuates frequencies below cutoff).
    HighPass,
    /// Peaking EQ filter (boosts or cuts around center frequency).
    Peaking,
    /// Band-pass filter (passes frequencies within a band around center).
    BandPass,
    /// Notch filter (rejects frequencies in a narrow band around center).
    Notch,
    /// Low-shelf filter (boosts or cuts frequencies below frequency).
    LowShelf,
    /// High-shelf filter (boosts or cuts frequencies above frequency).
    HighShelf,
}

impl FilterType {
    /// Peaking EQ alias for compatibility.
    #[allow(non_upper_case_globals)]
    pub const PeakingEq: FilterType = FilterType::Peaking;
}

/// Normalized biquad filter transfer function coefficients.
///
/// H(z) = (b0 + b1*z^-1 + b2*z^-2) / (1 + a1*z^-1 + a2*z^-2)
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BiquadCoefficients {
    pub b0: f32,
    pub b1: f32,
    pub b2: f32,
    pub a1: f32,
    pub a2: f32,
}

impl BiquadCoefficients {
    /// Calculate normalized biquad coefficients based on Robert Bristow-Johnson Audio EQ Cookbook.
    pub fn calculate(
        filter_type: FilterType,
        sample_rate: f32,
        freq: f32,
        q: f32,
        gain_db: f32,
    ) -> Self {
        let sample_rate = if sample_rate.is_finite() && sample_rate > 1000.0 {
            sample_rate
        } else {
            48000.0
        };

        // Clamp frequency to valid audio range below Nyquist limit
        let nyquist = sample_rate * 0.499;
        let freq = if freq.is_finite() {
            freq.clamp(10.0, nyquist)
        } else {
            1000.0
        };

        let q = if q.is_finite() && q > 0.001 {
            q
        } else {
            std::f32::consts::FRAC_1_SQRT_2
        };

        let gain_db = if gain_db.is_finite() {
            gain_db.clamp(-60.0, 60.0)
        } else {
            0.0
        };

        let omega = 2.0 * std::f32::consts::PI * freq / sample_rate;
        let cos_w = omega.cos();
        let sin_w = omega.sin();
        let alpha = sin_w / (2.0 * q);
        let a_amp = 10.0f32.powf(gain_db / 40.0);

        let (b0, b1, b2, a0, a1, a2) = match filter_type {
            FilterType::LowPass => {
                let b0 = (1.0 - cos_w) * 0.5;
                let b1 = 1.0 - cos_w;
                let b2 = (1.0 - cos_w) * 0.5;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::HighPass => {
                let b0 = (1.0 + cos_w) * 0.5;
                let b1 = -(1.0 + cos_w);
                let b2 = (1.0 + cos_w) * 0.5;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::Peaking => {
                let b0 = 1.0 + alpha * a_amp;
                let b1 = -2.0 * cos_w;
                let b2 = 1.0 - alpha * a_amp;
                let a0 = 1.0 + alpha / a_amp;
                let a1 = -2.0 * cos_w;
                let a2 = 1.0 - alpha / a_amp;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::BandPass => {
                let b0 = alpha;
                let b1 = 0.0;
                let b2 = -alpha;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::Notch => {
                let b0 = 1.0;
                let b1 = -2.0 * cos_w;
                let b2 = 1.0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::LowShelf => {
                let sqrt_a = a_amp.sqrt();
                let two_sqrt_a_alpha = 2.0 * sqrt_a * alpha;
                let b0 = a_amp * ((a_amp + 1.0) - (a_amp - 1.0) * cos_w + two_sqrt_a_alpha);
                let b1 = 2.0 * a_amp * ((a_amp - 1.0) - (a_amp + 1.0) * cos_w);
                let b2 = a_amp * ((a_amp + 1.0) - (a_amp - 1.0) * cos_w - two_sqrt_a_alpha);
                let a0 = (a_amp + 1.0) + (a_amp - 1.0) * cos_w + two_sqrt_a_alpha;
                let a1 = -2.0 * ((a_amp - 1.0) + (a_amp + 1.0) * cos_w);
                let a2 = (a_amp + 1.0) + (a_amp - 1.0) * cos_w - two_sqrt_a_alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::HighShelf => {
                let sqrt_a = a_amp.sqrt();
                let two_sqrt_a_alpha = 2.0 * sqrt_a * alpha;
                let b0 = a_amp * ((a_amp + 1.0) + (a_amp - 1.0) * cos_w + two_sqrt_a_alpha);
                let b1 = -2.0 * a_amp * ((a_amp - 1.0) + (a_amp + 1.0) * cos_w);
                let b2 = a_amp * ((a_amp + 1.0) - (a_amp - 1.0) * cos_w - two_sqrt_a_alpha);
                let a0 = (a_amp + 1.0) - (a_amp - 1.0) * cos_w + two_sqrt_a_alpha;
                let a1 = 2.0 * ((a_amp - 1.0) - (a_amp + 1.0) * cos_w);
                let a2 = (a_amp + 1.0) - (a_amp - 1.0) * cos_w - two_sqrt_a_alpha;
                (b0, b1, b2, a0, a1, a2)
            }
        };

        if a0.abs() > 1e-15 {
            let inv_a0 = 1.0 / a0;
            Self {
                b0: b0 * inv_a0,
                b1: b1 * inv_a0,
                b2: b2 * inv_a0,
                a1: a1 * inv_a0,
                a2: a2 * inv_a0,
            }
        } else {
            // Neutral passthrough on degenerate a0
            Self {
                b0: 1.0,
                b1: 0.0,
                b2: 0.0,
                a1: 0.0,
                a2: 0.0,
            }
        }
    }
}

impl Default for BiquadCoefficients {
    fn default() -> Self {
        Self::calculate(
            FilterType::LowPass,
            48000.0,
            1000.0,
            std::f32::consts::FRAC_1_SQRT_2,
            0.0,
        )
    }
}

/// Stereo Direct Form II Transposed biquad filter.
///
/// Features independent left and right state variables with automatic subnormal flushing
/// and NaN/Inf sanitation to prevent audio pipeline corruption.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BiquadFilter {
    filter_type: FilterType,
    sample_rate: f32,
    freq: f32,
    q: f32,
    gain_db: f32,
    coeffs: BiquadCoefficients,
    #[serde(skip)]
    s1_left: f32,
    #[serde(skip)]
    s2_left: f32,
    #[serde(skip)]
    s1_right: f32,
    #[serde(skip)]
    s2_right: f32,
    enabled: bool,
}

#[inline]
fn process_sample(coeffs: &BiquadCoefficients, s1: &mut f32, s2: &mut f32, x: f32) -> f32 {
    let x = if x.is_finite() { x } else { 0.0 };
    let y = coeffs.b0 * x + *s1;
    *s1 = coeffs.b1 * x - coeffs.a1 * y + *s2;
    *s2 = coeffs.b2 * x - coeffs.a2 * y;

    // Flush subnormals and sanitize state registers
    if !s1.is_finite() || s1.abs() < 1e-25 {
        *s1 = 0.0;
    }
    if !s2.is_finite() || s2.abs() < 1e-25 {
        *s2 = 0.0;
    }

    if y.is_finite() {
        y
    } else {
        0.0
    }
}

impl BiquadFilter {
    /// Create a new biquad filter with the specified topology and parameters.
    pub fn new(filter_type: FilterType, sample_rate: f32, freq: f32, q: f32, gain_db: f32) -> Self {
        let coeffs = BiquadCoefficients::calculate(filter_type, sample_rate, freq, q, gain_db);
        Self {
            filter_type,
            sample_rate,
            freq,
            q,
            gain_db,
            coeffs,
            s1_left: 0.0,
            s2_left: 0.0,
            s1_right: 0.0,
            s2_right: 0.0,
            enabled: true,
        }
    }

    /// Update filter parameters and recompute coefficients without clearing filter state.
    pub fn set_params(&mut self, sample_rate: f32, freq: f32, q: f32, gain_db: f32) {
        self.sample_rate = sample_rate;
        self.freq = freq;
        self.q = q;
        self.gain_db = gain_db;
        self.update_coefficients();
    }

    /// Set filter topology type and recompute coefficients.
    pub fn set_type(&mut self, filter_type: FilterType) {
        self.filter_type = filter_type;
        self.update_coefficients();
    }

    /// Set cutoff / center frequency in Hz and recompute coefficients.
    pub fn set_frequency(&mut self, freq: f32) {
        self.freq = freq;
        self.update_coefficients();
    }

    /// Set Q factor and recompute coefficients.
    pub fn set_q(&mut self, q: f32) {
        self.q = q;
        self.update_coefficients();
    }

    /// Set gain in dB (for peaking and shelf filters) and recompute coefficients.
    pub fn set_gain_db(&mut self, gain_db: f32) {
        self.gain_db = gain_db;
        self.update_coefficients();
    }

    /// Set sample rate in Hz and recompute coefficients.
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.update_coefficients();
    }

    /// Get current filter type.
    #[inline]
    pub fn filter_type(&self) -> FilterType {
        self.filter_type
    }

    /// Get current sample rate.
    #[inline]
    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    /// Get current cutoff or center frequency in Hz.
    #[inline]
    pub fn frequency(&self) -> f32 {
        self.freq
    }

    /// Get current Q factor.
    #[inline]
    pub fn q(&self) -> f32 {
        self.q
    }

    /// Get current gain in dB.
    #[inline]
    pub fn gain_db(&self) -> f32 {
        self.gain_db
    }

    /// Get current normalized biquad coefficients.
    #[inline]
    pub fn coefficients(&self) -> &BiquadCoefficients {
        &self.coeffs
    }

    fn update_coefficients(&mut self) {
        self.coeffs = BiquadCoefficients::calculate(
            self.filter_type,
            self.sample_rate,
            self.freq,
            self.q,
            self.gain_db,
        );
    }
}

impl Default for BiquadFilter {
    fn default() -> Self {
        Self::new(
            FilterType::LowPass,
            48000.0,
            1000.0,
            std::f32::consts::FRAC_1_SQRT_2,
            0.0,
        )
    }
}

impl AudioEffect for BiquadFilter {
    #[inline]
    fn process_frame(&mut self, frame: AudioFrame) -> AudioFrame {
        let frame = frame.sanitize();
        if !self.enabled {
            return frame;
        }

        let left = process_sample(
            &self.coeffs,
            &mut self.s1_left,
            &mut self.s2_left,
            frame.left,
        );
        let right = process_sample(
            &self.coeffs,
            &mut self.s1_right,
            &mut self.s2_right,
            frame.right,
        );

        AudioFrame::new(left, right)
    }

    #[inline]
    fn reset(&mut self) {
        self.s1_left = 0.0;
        self.s2_left = 0.0;
        self.s1_right = 0.0;
        self.s2_right = 0.0;
    }

    fn name(&self) -> &'static str {
        match self.filter_type {
            FilterType::LowPass => "Low-Pass Filter",
            FilterType::HighPass => "High-Pass Filter",
            FilterType::Peaking => "Peaking EQ",
            FilterType::BandPass => "Band-Pass Filter",
            FilterType::Notch => "Notch Filter",
            FilterType::LowShelf => "Low-Shelf Filter",
            FilterType::HighShelf => "High-Shelf Filter",
        }
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
