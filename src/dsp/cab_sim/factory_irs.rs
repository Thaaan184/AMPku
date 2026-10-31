//! Acoustic speaker cabinet impulse response synthesis.
//!
//! Synthesizes physical acoustic responses for 4 factory profiles using
//! calibrated multi-stage resonant biquad filters and cabinet cavity reflections.

use crate::dsp::cab_sim::ir_engine::CabinetProfile;
use crate::dsp::filters::biquad::{BiquadCoefficients, FilterType};

/// Standard impulse response length (number of taps) for zero-latency direct FIR convolution.
///
/// 512 samples corresponds to ~10.67 ms at 48 kHz, fully capturing direct acoustic wave,
/// cabinet cavity resonance, speaker breakup modes, and enclosure boundary reflections.
pub const IR_LENGTH: usize = 512;

/// Synthesize a 512-tap acoustic impulse response for a factory cabinet profile.
///
/// Models enclosure high-pass cutoff, fundamental box thump, cone breakup peaks,
/// voice coil inductance acoustic rolloff, and internal cabinet reflections.
pub fn generate_factory_ir(profile: CabinetProfile, sample_rate: f32) -> [f32; IR_LENGTH] {
    let sample_rate = if sample_rate.is_finite() && sample_rate > 1000.0 {
        sample_rate
    } else {
        48000.0
    };

    let mut buf = [0.0f32; IR_LENGTH];
    buf[0] = 1.0;

    match profile {
        CabinetProfile::CelestionV30 => {
            // Closed-back 4x12 with Celestion Vintage 30:
            // Punchy upper-mid peak (~2.8 kHz), tight bass cutoff (~85 Hz), voice coil rolloff (~5.2 kHz).
            buf[45] -= 0.15; // Enclosure back-wall reflection
            apply_biquad(&mut buf, FilterType::HighPass, sample_rate, 85.0, 1.0, 0.0);
            apply_biquad(&mut buf, FilterType::Peaking, sample_rate, 105.0, 1.8, 3.5);
            apply_biquad(&mut buf, FilterType::Peaking, sample_rate, 750.0, 1.5, -3.5);
            apply_biquad(&mut buf, FilterType::Peaking, sample_rate, 2850.0, 1.8, 6.5);
            apply_biquad(&mut buf, FilterType::Peaking, sample_rate, 4200.0, 2.2, 3.0);
            apply_biquad(
                &mut buf,
                FilterType::LowPass,
                sample_rate,
                5200.0,
                0.85,
                0.0,
            );
            apply_biquad(
                &mut buf,
                FilterType::LowPass,
                sample_rate,
                8000.0,
                0.707,
                0.0,
            );
        }
        CabinetProfile::FenderTwin => {
            // Open-back 2x12 vintage American tube combo:
            // Sparkling highs (4.8 kHz), scooped midrange (450 Hz), warm open bottom (90 Hz).
            buf[35] -= 0.10; // Open baffle reflection
            apply_biquad(&mut buf, FilterType::HighPass, sample_rate, 75.0, 0.8, 0.0);
            apply_biquad(&mut buf, FilterType::Peaking, sample_rate, 90.0, 1.4, 2.0);
            apply_biquad(&mut buf, FilterType::Peaking, sample_rate, 450.0, 1.2, -5.0);
            apply_biquad(&mut buf, FilterType::Peaking, sample_rate, 2200.0, 1.5, 1.5);
            apply_biquad(&mut buf, FilterType::Peaking, sample_rate, 4800.0, 2.0, 5.5);
            apply_biquad(&mut buf, FilterType::LowPass, sample_rate, 6800.0, 0.9, 0.0);
            apply_biquad(
                &mut buf,
                FilterType::LowPass,
                sample_rate,
                10000.0,
                0.707,
                0.0,
            );
        }
        CabinetProfile::Marshall1960 => {
            // Slanted 4x12 British rock cabinet with G12T-75 speakers:
            // Heavy 115 Hz cabinet thump, scooped 800 Hz mids, aggressive 3.8 kHz bite.
            buf[55] -= 0.18; // Slant baffle internal bounce
            apply_biquad(&mut buf, FilterType::HighPass, sample_rate, 90.0, 1.2, 0.0);
            apply_biquad(&mut buf, FilterType::Peaking, sample_rate, 115.0, 2.0, 4.5);
            apply_biquad(&mut buf, FilterType::Peaking, sample_rate, 800.0, 1.3, -4.5);
            apply_biquad(&mut buf, FilterType::Peaking, sample_rate, 2400.0, 1.6, 3.0);
            apply_biquad(&mut buf, FilterType::Peaking, sample_rate, 3800.0, 2.2, 5.0);
            apply_biquad(
                &mut buf,
                FilterType::LowPass,
                sample_rate,
                4900.0,
                0.85,
                0.0,
            );
            apply_biquad(
                &mut buf,
                FilterType::LowPass,
                sample_rate,
                7500.0,
                0.707,
                0.0,
            );
        }
        CabinetProfile::AcousticSim => {
            // Acoustic guitar body simulation:
            // 108 Hz soundhole Helmholtz resonance, 215 Hz top-plate resonance,
            // 3.2 kHz piezo de-quack notch, silky 8.5 kHz air sparkle.
            buf[50] += 0.12; // Soundboard internal reflection
            buf[110] += 0.08;
            apply_biquad(&mut buf, FilterType::HighPass, sample_rate, 65.0, 1.0, 0.0);
            apply_biquad(&mut buf, FilterType::Peaking, sample_rate, 108.0, 3.5, 6.5);
            apply_biquad(&mut buf, FilterType::Peaking, sample_rate, 215.0, 2.8, 5.0);
            apply_biquad(&mut buf, FilterType::Peaking, sample_rate, 420.0, 1.8, 2.5);
            apply_biquad(
                &mut buf,
                FilterType::Peaking,
                sample_rate,
                3200.0,
                3.0,
                -6.0,
            );
            apply_biquad(&mut buf, FilterType::Peaking, sample_rate, 8500.0, 1.5, 5.0);
            apply_biquad(
                &mut buf,
                FilterType::LowPass,
                sample_rate,
                13500.0,
                0.707,
                0.0,
            );
        }
    }

    // Apply smooth half-Hanning decay window to eliminate truncation discontinuity at buffer end
    const FADE_START: usize = 350;
    for (i, sample) in buf.iter_mut().enumerate().skip(FADE_START) {
        let phase = (i - FADE_START) as f32 / (IR_LENGTH - FADE_START) as f32;
        let fade = 0.5 * (1.0 + (std::f32::consts::PI * phase).cos());
        *sample *= fade;
    }
    buf[IR_LENGTH - 1] = 0.0;

    // Calibrate passband gain around 1000 Hz to prevent abrupt level drops or digital clipping
    let omega_1k = 2.0 * std::f32::consts::PI * 1000.0 / sample_rate;
    let mut real = 0.0f32;
    let mut imag = 0.0f32;
    for (n, &val) in buf.iter().enumerate() {
        let angle = omega_1k * n as f32;
        real += val * angle.cos();
        imag -= val * angle.sin();
    }
    let mag = (real * real + imag * imag).sqrt();
    if mag > 1e-5 {
        // Target passband gain ~ -1.0 dB (0.891)
        let scale = 0.891 / mag;
        for sample in buf.iter_mut() {
            *sample *= scale;
        }
    }

    buf
}

/// Apply a biquad filter in-place to an impulse response buffer.
fn apply_biquad(
    buf: &mut [f32; IR_LENGTH],
    filter_type: FilterType,
    sample_rate: f32,
    freq: f32,
    q: f32,
    gain_db: f32,
) {
    let coeffs = BiquadCoefficients::calculate(filter_type, sample_rate, freq, q, gain_db);
    let mut s1 = 0.0f32;
    let mut s2 = 0.0f32;
    for sample in buf.iter_mut() {
        let x = *sample;
        let y = coeffs.b0 * x + s1;
        s1 = coeffs.b1 * x - coeffs.a1 * y + s2;
        s2 = coeffs.b2 * x - coeffs.a2 * y;
        *sample = y;
    }
}
