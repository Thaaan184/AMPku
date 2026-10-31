use ampku::core::frame::AudioFrame;
use ampku::dsp::cab_sim::factory_irs::{generate_factory_ir, IR_LENGTH};
use ampku::dsp::cab_sim::ir_engine::{CabinetProfile, CabinetSim};
use ampku::dsp::traits::AudioEffect;

#[test]
fn test_cab_sim_impulse_response_convolves() {
    let mut cab = CabinetSim::new(CabinetProfile::CelestionV30);
    let delta = AudioFrame::new(1.0, 1.0);
    let out1 = cab.process_frame(delta);
    let out2 = cab.process_frame(AudioFrame::new(0.0, 0.0));
    assert!(out1.left != 0.0 || out2.left != 0.0);
}

#[test]
fn test_cab_sim_all_four_factory_profiles_convolving_and_distinct() {
    let profiles = CabinetProfile::ALL;
    assert_eq!(profiles.len(), 4);

    let mut outputs: Vec<Vec<f32>> = Vec::new();

    for &profile in &profiles {
        let mut cab = CabinetSim::new(profile);
        assert_eq!(cab.profile(), profile);
        assert_eq!(cab.active_profile_name(), profile.name());

        // Feed unit impulse
        let mut profile_out = Vec::new();
        let first = cab.process_frame(AudioFrame::new(1.0, 1.0));
        profile_out.push(first.left);

        for _ in 1..IR_LENGTH {
            let out = cab.process_frame(AudioFrame::ZERO);
            profile_out.push(out.left);
        }

        // Must convolve with non-zero energy
        let energy: f32 = profile_out.iter().map(|&x| x * x).sum();
        assert!(
            energy > 0.01,
            "Profile {:?} must produce significant energy, got {}",
            profile,
            energy
        );

        outputs.push(profile_out);
    }

    // Verify all 4 profiles produce distinct impulse responses
    for i in 0..outputs.len() {
        for j in (i + 1)..outputs.len() {
            let diff: f32 = outputs[i]
                .iter()
                .zip(outputs[j].iter())
                .map(|(&a, &b)| (a - b).abs())
                .sum();
            assert!(
                diff > 0.1,
                "Profiles {:?} and {:?} must be acoustically distinct, diff = {}",
                profiles[i],
                profiles[j],
                diff
            );
        }
    }
}

#[test]
fn test_cab_sim_spectral_characteristics() {
    // Celestion V30: guitar cab high-frequency rolloff
    // Energy at 10 kHz must be substantially lower than at 3 kHz
    let v30_ir = generate_factory_ir(CabinetProfile::CelestionV30, 48000.0);

    let response_at = |freq: f32| -> f32 {
        let omega = 2.0 * std::f32::consts::PI * freq / 48000.0;
        let mut real = 0.0f32;
        let mut imag = 0.0f32;
        for (n, &val) in v30_ir.iter().enumerate() {
            let angle = omega * n as f32;
            real += val * angle.cos();
            imag -= val * angle.sin();
        }
        (real * real + imag * imag).sqrt()
    };

    let resp_3k = response_at(3000.0);
    let resp_10k = response_at(10000.0);

    assert!(
        resp_3k > resp_10k * 4.0,
        "Celestion V30 must have strong HF rolloff (3kHz: {}, 10kHz: {})",
        resp_3k,
        resp_10k
    );

    // AcousticSim: acoustic sparkle
    // AcousticSim at 10 kHz has much higher air response than electric guitar cabs
    let acoustic_ir = generate_factory_ir(CabinetProfile::AcousticSim, 48000.0);
    let mut real_ac = 0.0f32;
    let mut imag_ac = 0.0f32;
    let omega_10k = 2.0 * std::f32::consts::PI * 10000.0 / 48000.0;
    for (n, &val) in acoustic_ir.iter().enumerate() {
        let angle = omega_10k * n as f32;
        real_ac += val * angle.cos();
        imag_ac -= val * angle.sin();
    }
    let acoustic_10k = (real_ac * real_ac + imag_ac * imag_ac).sqrt();

    assert!(
        acoustic_10k > resp_10k * 3.0,
        "AcousticSim should retain more 10kHz air than V30 (Acoustic: {}, V30: {})",
        acoustic_10k,
        resp_10k
    );
}

#[test]
fn test_cab_sim_bypass() {
    let mut cab = CabinetSim::new(CabinetProfile::Marshall1960);
    cab.set_enabled(false);
    assert!(!cab.is_enabled());

    let input = AudioFrame::new(0.42, -0.73);
    let out = cab.process_frame(input);
    assert_eq!(out.left, input.left);
    assert_eq!(out.right, input.right);

    cab.set_enabled(true);
    assert!(cab.is_enabled());
    let active_out = cab.process_frame(input);
    assert_ne!(active_out.left, input.left);
}

#[test]
fn test_cab_sim_reset() {
    let mut cab = CabinetSim::new(CabinetProfile::FenderTwin);

    // Prime the convolution history with signal
    for _ in 0..100 {
        cab.process_frame(AudioFrame::new(1.0, 1.0));
    }

    // Reset clears history
    cab.reset();

    // With zero input, output should immediately be zero
    let out = cab.process_frame(AudioFrame::ZERO);
    assert_eq!(out.left, 0.0);
    assert_eq!(out.right, 0.0);
}

#[test]
fn test_cab_sim_mix_control() {
    let mut cab = CabinetSim::new(CabinetProfile::CelestionV30);

    // 100% dry (mix = 0.0)
    cab.set_mix(0.0);
    assert_eq!(cab.mix(), 0.0);

    let test_frame = AudioFrame::new(0.65, -0.35);
    let dry_out = cab.process_frame(test_frame);
    assert_eq!(dry_out.left, test_frame.left);
    assert_eq!(dry_out.right, test_frame.right);

    // 100% wet (mix = 1.0)
    cab.reset();
    cab.set_mix(1.0);
    let wet_out = cab.process_frame(test_frame);
    assert_ne!(wet_out.left, test_frame.left);

    // 50% wet (mix = 0.5)
    cab.reset();
    cab.set_mix(0.5);
    let blend_out = cab.process_frame(test_frame);
    let expected_left = test_frame.left * 0.5 + wet_out.left * 0.5;
    assert!((blend_out.left - expected_left).abs() < 1e-4);
}

#[test]
fn test_cab_sim_output_gain_db() {
    let mut cab = CabinetSim::new(CabinetProfile::CelestionV30);
    let input = AudioFrame::new(0.5, 0.5);

    let out_0db = cab.process_frame(input);

    cab.reset();
    cab.set_output_gain_db(6.0); // +6 dB ≈ 2x gain
    assert_eq!(cab.output_gain_db(), 6.0);
    let out_plus6 = cab.process_frame(input);

    let ratio = out_plus6.left / out_0db.left;
    assert!(
        (ratio - 2.0).abs() < 0.05,
        "Expected gain ratio ~2.0 for +6dB, got {}",
        ratio
    );

    cab.reset();
    cab.set_output_gain_db(-6.0); // -6 dB ≈ 0.5x gain
    let out_minus6 = cab.process_frame(input);
    let ratio_neg = out_minus6.left / out_0db.left;
    assert!(
        (ratio_neg - 0.5).abs() < 0.05,
        "Expected gain ratio ~0.5 for -6dB, got {}",
        ratio_neg
    );
}

#[test]
fn test_cab_sim_load_custom_ir() {
    let mut cab = CabinetSim::new(CabinetProfile::CelestionV30);

    // Create custom IR: a simple 3-sample delay impulse: [0.0, 0.0, 1.0]
    let mut custom = [0.0f32; 16];
    custom[2] = 1.0;
    cab.load_custom_ir(&custom);

    // Custom IR should be normalized to 0.5 peak
    assert_eq!(cab.impulse_response()[2], 0.5);

    // Send unit impulse
    let o0 = cab.process_frame(AudioFrame::new(1.0, 1.0));
    let o1 = cab.process_frame(AudioFrame::ZERO);
    let o2 = cab.process_frame(AudioFrame::ZERO);
    let o3 = cab.process_frame(AudioFrame::ZERO);

    assert_eq!(o0.left, 0.0);
    assert_eq!(o1.left, 0.0);
    assert!((o2.left - 0.5).abs() < 1e-5);
    assert_eq!(o3.left, 0.0);
}

#[test]
fn test_cab_sim_stereo_independent() {
    let mut cab = CabinetSim::new(CabinetProfile::Marshall1960);

    let impulse = AudioFrame::new(1.0, 0.0); // Left channel only
    let out = cab.process_frame(impulse);

    assert!(out.left != 0.0);
    assert_eq!(
        out.right, 0.0,
        "Right channel must remain completely silent"
    );
}

#[test]
fn test_cab_sim_process_block_matches_process_frame() {
    let mut cab_frame = CabinetSim::new(CabinetProfile::FenderTwin);
    let mut cab_block = CabinetSim::new(CabinetProfile::FenderTwin);

    let input_frames: Vec<AudioFrame> = (0..64)
        .map(|i| {
            let t = i as f32 / 64.0;
            AudioFrame::new((t * 10.0).sin(), (t * 15.0).cos())
        })
        .collect();

    let mut frame_by_frame_out = Vec::new();
    for &frame in &input_frames {
        frame_by_frame_out.push(cab_frame.process_frame(frame));
    }

    let mut block_out = input_frames;
    cab_block.process_block(&mut block_out);

    for (f, b) in frame_by_frame_out.iter().zip(block_out.iter()) {
        assert_eq!(f.left, b.left);
        assert_eq!(f.right, b.right);
    }
}

#[test]
fn test_cab_sim_nan_inf_sanitization() {
    let mut cab = CabinetSim::new(CabinetProfile::CelestionV30);

    let poisoned = AudioFrame::new(f32::NAN, f32::INFINITY);
    let out = cab.process_frame(poisoned);

    assert!(out.is_finite());
    assert_eq!(out.left, 0.0);
    assert_eq!(out.right, 0.0);
}

#[test]
fn test_cab_sim_trait_object() {
    let mut effect: Box<dyn AudioEffect> = Box::new(CabinetSim::new(CabinetProfile::AcousticSim));
    assert_eq!(effect.name(), "Cabinet IR Simulator");
    assert!(effect.is_enabled());

    let out = effect.process_frame(AudioFrame::new(1.0, 1.0));
    assert!(out.is_finite());
}

#[test]
fn test_cab_sim_serde_json_roundtrip() {
    let mut original = CabinetSim::new(CabinetProfile::Marshall1960);
    original.set_mix(0.85);
    original.set_output_gain_db(-2.5);
    original.set_enabled(true);

    let json = serde_json::to_string(&original).expect("Serialization failed");
    let deserialized: CabinetSim = serde_json::from_str(&json).expect("Deserialization failed");

    assert_eq!(original, deserialized);
    assert_eq!(deserialized.profile(), CabinetProfile::Marshall1960);
    assert!((deserialized.mix() - 0.85).abs() < 1e-5);
    assert!((deserialized.output_gain_db() - (-2.5)).abs() < 1e-5);
}

#[test]
fn test_cab_sim_sample_rate_switching() {
    let mut cab = CabinetSim::new(CabinetProfile::CelestionV30);
    assert_eq!(cab.sample_rate(), 48000.0);

    cab.set_sample_rate(96000.0);
    assert_eq!(cab.sample_rate(), 96000.0);

    let delta = AudioFrame::new(1.0, 1.0);
    let out = cab.process_frame(delta);
    assert!(out.is_finite());
    assert!(out.left != 0.0);
}

#[test]
fn test_cab_sim_load_custom_ir_nan_inf_sanitization() {
    let mut cab = CabinetSim::new(CabinetProfile::CelestionV30);
    let mut bad_samples = vec![0.5; 512];
    bad_samples[0] = f32::NAN;
    bad_samples[1] = f32::INFINITY;
    bad_samples[2] = f32::NEG_INFINITY;
    bad_samples[3] = 1.0;

    cab.load_custom_ir(&bad_samples);

    let ir = cab.impulse_response();
    assert_eq!(ir[0], 0.0);
    assert_eq!(ir[1], 0.0);
    assert_eq!(ir[2], 0.0);
    assert!(ir.iter().all(|s| s.is_finite()));

    // Convolution with sanitized IR must produce finite audio
    for _ in 0..100 {
        let out = cab.process_frame(AudioFrame::new(0.5, -0.5));
        assert!(out.is_finite());
    }
}
