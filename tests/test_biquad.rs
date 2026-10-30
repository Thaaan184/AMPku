use ampku::core::frame::AudioFrame;
use ampku::dsp::filters::biquad::{BiquadFilter, FilterType};
use ampku::dsp::traits::AudioEffect;

#[test]
fn test_biquad_high_pass_attenuates_dc() {
    let mut hpf = BiquadFilter::new(FilterType::HighPass, 48000.0, 100.0, 0.707, 0.0);
    let mut out = AudioFrame::new(1.0, 1.0);
    for _ in 0..1000 {
        out = hpf.process_frame(AudioFrame::new(1.0, 1.0));
    }
    assert!(out.left.abs() < 0.01);
}

#[test]
fn test_biquad_nan_protection() {
    let mut hpf = BiquadFilter::new(FilterType::HighPass, 48000.0, 100.0, 0.707, 0.0);
    let out = hpf.process_frame(AudioFrame::new(f32::NAN, f32::INFINITY));
    assert!(!out.left.is_nan() && !out.right.is_nan());
}

#[test]
fn test_biquad_low_pass_passes_dc_and_attenuates_nyquist() {
    let mut lpf = BiquadFilter::new(FilterType::LowPass, 48000.0, 1000.0, 0.707, 0.0);

    // DC signal (1.0) should pass through low-pass filter
    let mut out = AudioFrame::ZERO;
    for _ in 0..2000 {
        out = lpf.process_frame(AudioFrame::new(1.0, 1.0));
    }
    assert!((out.left - 1.0).abs() < 0.01);
    assert!((out.right - 1.0).abs() < 0.01);

    // Nyquist signal (+1, -1, +1, -1) should be heavily attenuated
    let mut nyq_out = AudioFrame::ZERO;
    for i in 0..2000 {
        let sign = if i % 2 == 0 { 1.0 } else { -1.0 };
        nyq_out = lpf.process_frame(AudioFrame::new(sign, sign));
    }
    assert!(nyq_out.left.abs() < 0.05);
    assert!(nyq_out.right.abs() < 0.05);
}

#[test]
fn test_biquad_peaking_eq_boost() {
    let sample_rate = 48000.0;
    let center_freq = 1000.0;
    let boost_db = 6.0; // ~2x gain

    let mut eq = BiquadFilter::new(FilterType::Peaking, sample_rate, center_freq, 2.0, boost_db);

    // Run sine wave at center frequency
    let num_samples = 4800; // 100 ms
    let mut max_out = 0.0f32;
    for i in 0..num_samples {
        let t = i as f32 / sample_rate;
        let s = (2.0 * std::f32::consts::PI * center_freq * t).sin();
        let out = eq.process_frame(AudioFrame::new(s, s));
        if i > 2400 {
            // Steady state
            max_out = max_out.max(out.left.abs());
        }
    }

    // 6 dB boost means amplitude is roughly doubled (around 2.0)
    let expected_gain = 10.0f32.powf(boost_db / 20.0);
    assert!(
        (max_out - expected_gain).abs() < 0.15,
        "Expected peak around {expected_gain}, got {max_out}"
    );
}

#[test]
fn test_biquad_bypass_and_reset() {
    let mut hpf = BiquadFilter::new(FilterType::HighPass, 48000.0, 100.0, 0.707, 0.0);
    assert!(hpf.is_enabled());

    hpf.set_enabled(false);
    assert!(!hpf.is_enabled());

    // When disabled/bypassed, DC signal passes through untouched
    let out = hpf.process_frame(AudioFrame::new(1.0, 1.0));
    assert_eq!(out.left, 1.0);
    assert_eq!(out.right, 1.0);

    // Re-enable and verify reset clears states
    hpf.set_enabled(true);
    let _ = hpf.process_frame(AudioFrame::new(1.0, 1.0));
    hpf.reset();
    let out_after_reset = hpf.process_frame(AudioFrame::new(0.0, 0.0));
    assert_eq!(out_after_reset.left, 0.0);
    assert_eq!(out_after_reset.right, 0.0);
}

#[test]
fn test_biquad_stereo_channel_independence() {
    let mut hpf = BiquadFilter::new(FilterType::HighPass, 48000.0, 100.0, 0.707, 0.0);

    // Feed left channel only
    for _ in 0..100 {
        let out = hpf.process_frame(AudioFrame::new(0.5, 0.0));
        assert_eq!(out.right, 0.0);
    }
}

#[test]
fn test_biquad_trait_object_dispatch() {
    let mut effect: Box<dyn AudioEffect> = Box::new(BiquadFilter::new(
        FilterType::HighPass,
        48000.0,
        100.0,
        0.707,
        0.0,
    ));

    assert_eq!(effect.name(), "High-Pass Filter");
    assert!(effect.is_enabled());

    let mut out = AudioFrame::new(1.0, 1.0);
    for _ in 0..1000 {
        out = effect.process_frame(AudioFrame::new(1.0, 1.0));
    }
    assert!(out.left.abs() < 0.01);
}

#[test]
fn test_biquad_set_params_runtime_update() {
    let mut filter = BiquadFilter::new(FilterType::LowPass, 48000.0, 500.0, 0.707, 0.0);
    assert_eq!(filter.frequency(), 500.0);

    filter.set_params(48000.0, 2000.0, 1.0, 3.0);
    assert_eq!(filter.frequency(), 2000.0);
    assert_eq!(filter.q(), 1.0);
    assert_eq!(filter.gain_db(), 3.0);
}
