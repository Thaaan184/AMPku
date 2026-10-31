use ampku::core::frame::AudioFrame;
use ampku::dsp::dynamics::compressor::Compressor;
use ampku::dsp::dynamics::noise_gate::NoiseGate;
use ampku::dsp::saturation::distortion::Distortion;
use ampku::dsp::saturation::fuzz::Fuzz;
use ampku::dsp::saturation::overdrive::Overdrive;
use ampku::dsp::traits::AudioEffect;

#[test]
fn test_noise_gate_mutes_low_level() {
    let mut gate = NoiseGate::new(48000.0, -40.0, 5.0, 50.0);
    let low_sig = AudioFrame::new(0.0001, 0.0001); // ~ -80 dB
    let out = gate.process_frame(low_sig);
    assert_eq!(out.left, 0.0);
    assert_eq!(out.right, 0.0);
}

#[test]
fn test_noise_gate_opens_on_loud_signal() {
    let mut gate = NoiseGate::new(48000.0, -40.0, 1.0, 50.0);

    // Initial closed state
    assert_eq!(
        gate.process_frame(AudioFrame::new(0.0001, 0.0001)).left,
        0.0
    );

    // Send high-level burst (0.5 = -6 dBFS > -40 dBFS threshold)
    let loud_frame = AudioFrame::new(0.5, 0.5);
    let mut out = AudioFrame::ZERO;
    // Attack is 1 ms = 48 samples
    for _ in 0..100 {
        out = gate.process_frame(loud_frame);
    }
    // Should be fully open (gain = 1.0)
    assert!(
        (out.left - 0.5).abs() < 1e-4,
        "Gate should pass signal at full amplitude once open, got {}",
        out.left
    );
    assert!(gate.is_open());
}

#[test]
fn test_noise_gate_bypass_and_reset() {
    let mut gate = NoiseGate::new(48000.0, -40.0, 5.0, 50.0);
    let low_sig = AudioFrame::new(0.0001, 0.0001);

    gate.set_enabled(false);
    assert!(!gate.is_enabled());
    let bypassed = gate.process_frame(low_sig);
    assert_eq!(bypassed.left, 0.0001);

    gate.set_enabled(true);
    let _ = gate.process_frame(AudioFrame::new(1.0, 1.0));
    gate.reset();
    assert_eq!(gate.current_gain(), 0.0);
    let muted_again = gate.process_frame(low_sig);
    assert_eq!(muted_again.left, 0.0);
}

#[test]
fn test_noise_gate_nan_protection() {
    let mut gate = NoiseGate::new(48000.0, -40.0, 5.0, 50.0);
    let out = gate.process_frame(AudioFrame::new(f32::NAN, f32::INFINITY));
    assert!(out.is_finite());
}

#[test]
fn test_compressor_passes_low_level_untouched() {
    let mut comp = Compressor::new(48000.0, -20.0, 4.0, 5.0, 50.0, 0.0);
    // -40 dB signal is well below -20 dB threshold -> no compression
    let low_level = 0.01;
    let frame = AudioFrame::new(low_level, low_level);
    let mut out = AudioFrame::ZERO;
    for _ in 0..500 {
        out = comp.process_frame(frame);
    }
    assert!(
        (out.left - low_level).abs() < 1e-4,
        "Low level below threshold must pass unattenuated"
    );
    assert_eq!(comp.gain_reduction_db(), 0.0);
}

#[test]
fn test_compressor_attenuates_loud_signal() {
    let mut comp = Compressor::new(48000.0, -20.0, 4.0, 1.0, 50.0, 0.0);
    // 0 dBFS input (1.0). Threshold is -20 dBFS.
    // Overshoot = 20 dB. Ratio = 4:1. Reduction target = 20 * (1 - 1/4) = 15 dB.
    // Expected output = -15 dBFS = 10^(-15/20) ≈ 0.1778.
    let loud_frame = AudioFrame::new(1.0, 1.0);
    let mut out = AudioFrame::ZERO;
    for _ in 0..500 {
        out = comp.process_frame(loud_frame);
    }

    let expected_gain = 10.0f32.powf(-15.0 / 20.0);
    assert!(
        (out.left - expected_gain).abs() < 0.03,
        "Output expected around {expected_gain}, got {}",
        out.left
    );
    assert!(comp.gain_reduction_db() > 14.0);
}

#[test]
fn test_compressor_makeup_gain() {
    let mut comp = Compressor::new(48000.0, -20.0, 4.0, 5.0, 50.0, 6.0); // +6 dB makeup gain (~2x)
    let low_level = 0.01;
    let frame = AudioFrame::new(low_level, low_level);
    let mut out = AudioFrame::ZERO;
    for _ in 0..500 {
        out = comp.process_frame(frame);
    }

    let expected = low_level * 10.0f32.powf(6.0 / 20.0);
    assert!(
        (out.left - expected).abs() < 1e-3,
        "Expected output with makeup gain {expected}, got {}",
        out.left
    );
}

#[test]
fn test_compressor_bypass_and_reset() {
    let mut comp = Compressor::new(48000.0, -20.0, 4.0, 1.0, 50.0, 0.0);
    comp.set_enabled(false);
    assert!(!comp.is_enabled());

    let frame = AudioFrame::new(1.0, 1.0);
    let out = comp.process_frame(frame);
    assert_eq!(out.left, 1.0);

    comp.set_enabled(true);
    for _ in 0..100 {
        let _ = comp.process_frame(frame);
    }
    assert!(comp.gain_reduction_db() > 0.0);

    comp.reset();
    assert_eq!(comp.gain_reduction_db(), 0.0);
}

#[test]
fn test_overdrive_soft_clips() {
    let mut od = Overdrive::new(5.0, 0.5, 0.8);
    let loud_sig = AudioFrame::new(0.9, 0.9);
    let out = od.process_frame(loud_sig);
    assert!(out.left > 0.0 && out.left <= 0.85);
}

#[test]
fn test_overdrive_tone_filtering() {
    // Dark overdrive (tone = 0.0) vs Bright overdrive (tone = 1.0)
    let mut od_dark = Overdrive::new(1.0, 0.0, 1.0);
    let mut od_bright = Overdrive::new(1.0, 1.0, 1.0);

    // High frequency excitation (alternating +0.5, -0.5 = 24 kHz Nyquist tone)
    let mut dark_energy = 0.0f32;
    let mut bright_energy = 0.0f32;
    for i in 0..1000 {
        let sign = if i % 2 == 0 { 0.5 } else { -0.5 };
        let out_d = od_dark.process_frame(AudioFrame::new(sign, sign));
        let out_b = od_bright.process_frame(AudioFrame::new(sign, sign));
        if i > 500 {
            dark_energy += out_d.left.abs();
            bright_energy += out_b.left.abs();
        }
    }

    assert!(
        bright_energy > dark_energy * 1.5,
        "Bright tone setting must pass significantly more high frequency content than dark setting"
    );
}

#[test]
fn test_overdrive_bypass_and_reset() {
    let mut od = Overdrive::new(10.0, 0.5, 0.5);
    od.set_enabled(false);
    let frame = AudioFrame::new(0.7, 0.7);
    let out = od.process_frame(frame);
    assert_eq!(out.left, 0.7);

    od.set_enabled(true);
    let _ = od.process_frame(frame);
    od.reset();
    let zero_out = od.process_frame(AudioFrame::ZERO);
    assert_eq!(zero_out.left, 0.0);
}

#[test]
fn test_distortion_asymmetric_clipping() {
    let mut dist = Distortion::new(20.0, 0.5, 1.0);

    // Run high amplitude positive and negative pulses through distortion
    let pos_in = AudioFrame::new(1.0, 1.0);
    let neg_in = AudioFrame::new(-1.0, -1.0);

    let mut pos_out = AudioFrame::ZERO;
    let mut neg_out = AudioFrame::ZERO;

    for _ in 0..500 {
        pos_out = dist.process_frame(pos_in);
    }
    dist.reset();
    for _ in 0..500 {
        neg_out = dist.process_frame(neg_in);
    }

    // Diode positive threshold is ~0.7, negative is ~-0.35
    // The positive and negative clipped peak amplitudes must be significantly asymmetrical!
    assert!(
        pos_out.left.abs() > neg_out.left.abs() * 1.2,
        "Asymmetric diode clipping should yield greater positive swing than negative swing: pos={}, neg={}",
        pos_out.left,
        neg_out.left
    );
}

#[test]
fn test_distortion_bypass_and_reset() {
    let mut dist = Distortion::new(10.0, 0.5, 0.7);
    dist.set_enabled(false);
    let out = dist.process_frame(AudioFrame::new(0.5, 0.5));
    assert_eq!(out.left, 0.5);

    dist.set_enabled(true);
    let _ = dist.process_frame(AudioFrame::new(0.5, 0.5));
    dist.reset();
    let zero_out = dist.process_frame(AudioFrame::ZERO);
    assert_eq!(zero_out.left, 0.0);
}

#[test]
fn test_fuzz_wavefolding_behavior() {
    let mut fuzz = Fuzz::new(10.0, 0.8, 1.0);

    // Feed a continuous sinusoidal cycle through the wavefolder
    let sample_rate = 48000.0;
    let freq = 100.0;
    let mut max_abs = 0.0f32;
    for i in 0..480 {
        let t = i as f32 / sample_rate;
        let s = (2.0 * std::f32::consts::PI * freq * t).sin();
        let out = fuzz.process_frame(AudioFrame::new(s, s));
        max_abs = max_abs.max(out.left.abs());
    }

    // Fuzz wavefolder clamps and shapes signal strictly within bounds
    assert!(max_abs > 0.0 && max_abs <= 1.5);
}

#[test]
fn test_fuzz_bypass_and_reset() {
    let mut fuzz = Fuzz::new(10.0, 0.5, 0.5);
    fuzz.set_enabled(false);
    let out = fuzz.process_frame(AudioFrame::new(0.8, 0.8));
    assert_eq!(out.left, 0.8);

    fuzz.set_enabled(true);
    let _ = fuzz.process_frame(AudioFrame::new(0.8, 0.8));
    fuzz.reset();
    let zero_out = fuzz.process_frame(AudioFrame::ZERO);
    assert_eq!(zero_out.left, 0.0);
}

#[test]
fn test_audio_effect_trait_objects() {
    // Verify all 5 nodes can be dynamically dispatched as Box<dyn AudioEffect>
    let mut effects: Vec<Box<dyn AudioEffect>> = vec![
        Box::new(NoiseGate::default()),
        Box::new(Compressor::default()),
        Box::new(Overdrive::default()),
        Box::new(Distortion::default()),
        Box::new(Fuzz::default()),
    ];

    assert_eq!(effects[0].name(), "Noise Gate");
    assert_eq!(effects[1].name(), "Compressor");
    assert_eq!(effects[2].name(), "Overdrive");
    assert_eq!(effects[3].name(), "Distortion");
    assert_eq!(effects[4].name(), "Fuzz");

    let test_frame = AudioFrame::new(0.2, 0.2);
    for effect in effects.iter_mut() {
        assert!(effect.is_enabled());
        let processed = effect.process_frame(test_frame);
        assert!(processed.is_finite());
        effect.reset();
    }
}

#[test]
fn test_nan_and_inf_robustness() {
    let mut effects: Vec<Box<dyn AudioEffect>> = vec![
        Box::new(NoiseGate::default()),
        Box::new(Compressor::default()),
        Box::new(Overdrive::default()),
        Box::new(Distortion::default()),
        Box::new(Fuzz::default()),
    ];

    let bad_frames = [
        AudioFrame::new(f32::NAN, f32::NAN),
        AudioFrame::new(f32::INFINITY, f32::NEG_INFINITY),
        AudioFrame::new(1e20, -1e20),
    ];

    for effect in effects.iter_mut() {
        for &bad in &bad_frames {
            let out = effect.process_frame(bad);
            assert!(
                out.left.is_finite() && out.right.is_finite(),
                "Effect {} produced non-finite output: {:?}",
                effect.name(),
                out
            );
        }
    }
}
