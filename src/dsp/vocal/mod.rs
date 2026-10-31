//! Vocal DSP processing suite for AMPku.

pub mod de_esser;

pub use de_esser::{DeEsserMode, VocalDeEsser, VocalDeEsserConfig};

use crate::dsp::dynamics::compressor::Compressor;
use crate::dsp::filters::biquad::{BiquadFilter, FilterType};

/// Standard vocal high-pass filter (80 Hz Butterworth high-pass filter).
///
/// Removes low-frequency stage rumble, air conditioning hum, handling noise,
/// and microphone plosive pops while preserving vocal body and warmth.
pub fn vocal_high_pass(sample_rate: f32) -> BiquadFilter {
    BiquadFilter::new(
        FilterType::HighPass,
        sample_rate,
        80.0,
        std::f32::consts::FRAC_1_SQRT_2,
        0.0,
    )
}

/// Factory preset for a gentle, transparent vocal channel compressor.
///
/// Configured with 3:1 ratio, -18 dBFS threshold, 10 ms attack, 100 ms release,
/// and +2.0 dB makeup gain.
pub fn vocal_compressor(sample_rate: f32) -> Compressor {
    Compressor::new(sample_rate, -18.0, 3.0, 10.0, 100.0, 2.0)
}
