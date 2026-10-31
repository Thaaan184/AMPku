use ampku::core::frame::AudioFrame;
use ampku::dsp::traits::AudioEffect;
use ampku::dsp::vocal::de_esser::{DeEsserMode, VocalDeEsser};
use ampku::dsp::vocal::{vocal_compressor, vocal_high_pass};

#[test]
fn test_de_esser_attenuates_sibilance() {
    let mut de_esser = VocalDeEsser::new(48000.0, 6000.0, -20.0, 4.0);
    let sibilant_frame = AudioFrame::new(0.8, 0.8);
    let out = de_esser.process_frame(sibilant_frame);
    assert!(
        out.left < 0.8,
        "De-esser must attenuate high-energy sibilant frame immediately, got {}",
        out.left
    );
}

#[test]
fn test_de_esser_leaves_low_frequencies_uncompressed() {
    let mut de_esser = VocalDeEsser::new(48000.0, 6000.0, -20.0, 4.0);

    // 200 Hz vocal fundamental tone at 0.7 amplitude (~ -3 dBFS)
    // Even though 0.7 > -20 dBFS threshold, the 6 kHz bandpass filter attenuates
    // 200 Hz by ~40 dB, so detector level is ~ -43 dBFS (< -20 dBFS threshold).
    let mut max_out = 0.0f32;
    for i in 0..1000 {
        let t = i as f32 / 48000.0;
        let s = 0.7 * (2.0 * std::f32::consts::PI * 200.0 * t).sin();
        let out = de_esser.process_frame(AudioFrame::new(s, s));
        if i > 700 {
            max_out = max_out.max(out.left.abs());
        }
    }

    assert!(
        (max_out - 0.7).abs() < 0.015,
        "200 Hz tone should remain untouched, got peak {max_out} vs expected 0.7"
    );
    assert!(
        de_esser.gain_reduction_db() < 0.05,
        "Gain reduction should be 0 dB for low frequencies, got {} dB",
        de_esser.gain_reduction_db()
    );
}

#[test]
fn test_de_esser_attenuates_high_frequency_sibilance_tone() {
    let mut de_esser = VocalDeEsser::new(48000.0, 6000.0, -20.0, 4.0);

    // 6000 Hz harsh sibilance tone at 0.8 amplitude
    let mut max_out = 0.0f32;
    for i in 0..1500 {
        let t = i as f32 / 48000.0;
        let s = 0.8 * (2.0 * std::f32::consts::PI * 6000.0 * t).sin();
        let out = de_esser.process_frame(AudioFrame::new(s, s));
        if i > 1000 {
            max_out = max_out.max(out.left.abs());
        }
    }

    assert!(
        max_out < 0.5,
        "6 kHz sibilance tone must be substantially attenuated, got peak {max_out}"
    );
    assert!(
        de_esser.gain_reduction_db() > 6.0,
        "Gain reduction should be active (> 6 dB), got {} dB",
        de_esser.gain_reduction_db()
    );
}

#[test]
fn test_de_esser_reduction_scales_with_input_level() {
    let mut de_soft = VocalDeEsser::new(48000.0, 6000.0, -20.0, 4.0);
    let mut de_loud = VocalDeEsser::new(48000.0, 6000.0, -20.0, 4.0);

    for i in 0..1500 {
        let t = i as f32 / 48000.0;
        let s_soft = 0.25 * (2.0 * std::f32::consts::PI * 6000.0 * t).sin();
        let s_loud = 0.90 * (2.0 * std::f32::consts::PI * 6000.0 * t).sin();

        de_soft.process_frame(AudioFrame::new(s_soft, s_soft));
        de_loud.process_frame(AudioFrame::new(s_loud, s_loud));
    }

    assert!(
        de_loud.gain_reduction_db() > de_soft.gain_reduction_db(),
        "Louder sibilance must trigger greater gain reduction: loud {} dB vs soft {} dB",
        de_loud.gain_reduction_db(),
        de_soft.gain_reduction_db()
    );
}

#[test]
fn test_de_esser_wideband_mode() {
    let mut de_esser = VocalDeEsser::new(48000.0, 6000.0, -20.0, 4.0);
    de_esser.set_mode(DeEsserMode::WideBand);
    assert_eq!(de_esser.mode(), DeEsserMode::WideBand);

    // Run 6 kHz loud burst to trigger gain reduction
    for i in 0..1000 {
        let t = i as f32 / 48000.0;
        let s = 0.8 * (2.0 * std::f32::consts::PI * 6000.0 * t).sin();
        de_esser.process_frame(AudioFrame::new(s, s));
    }

    let gr = de_esser.gain_reduction_db();
    assert!(gr > 6.0, "Gain reduction should be active, got {gr} dB");

    // In wideband mode, broadband frame is scaled by linear gain factor
    let expected_gain = 10.0f32.powf(-gr / 20.0);
    let test_frame = AudioFrame::new(0.5, 0.5);
    let out = de_esser.process_frame(test_frame);

    assert!(
        (out.left - 0.5 * expected_gain).abs() < 0.05,
        "Wideband mode must scale signal by gain multiplier"
    );
}

#[test]
fn test_de_esser_splitband_mode() {
    let mut de_esser = VocalDeEsser::new(48000.0, 6000.0, -20.0, 4.0);
    de_esser.set_mode(DeEsserMode::SplitBand);
    assert_eq!(de_esser.mode(), DeEsserMode::SplitBand);

    // Initial state without sibilance should pass signal untouched
    let dry_frame = AudioFrame::new(0.5, 0.5);
    let out = de_esser.process_frame(dry_frame);
    assert!(
        (out.left - 0.5).abs() < 0.02,
        "Split-band mode with no compression should pass frame transparently, got {}",
        out.left
    );
}

#[test]
fn test_de_esser_listen_mode() {
    let mut de_esser = VocalDeEsser::new(48000.0, 6000.0, -20.0, 4.0);
    de_esser.set_listen(true);
    assert!(de_esser.is_listen());

    // In listen mode, output is sidechain bandpass output
    // A 6000 Hz sine should pass with ~0 dB gain
    let mut peak_6k = 0.0f32;
    for i in 0..500 {
        let t = i as f32 / 48000.0;
        let s = 0.5 * (2.0 * std::f32::consts::PI * 6000.0 * t).sin();
        let out = de_esser.process_frame(AudioFrame::new(s, s));
        if i > 300 {
            peak_6k = peak_6k.max(out.left.abs());
        }
    }

    assert!(
        (peak_6k - 0.5).abs() < 0.08,
        "Listen mode should pass 6 kHz sibilance tone at full bandpass level, got {peak_6k}"
    );

    // A 100 Hz tone should be heavily attenuated in listen mode
    de_esser.reset();
    let mut peak_100 = 0.0f32;
    for i in 0..500 {
        let t = i as f32 / 48000.0;
        let s = 0.5 * (2.0 * std::f32::consts::PI * 100.0 * t).sin();
        let out = de_esser.process_frame(AudioFrame::new(s, s));
        if i > 300 {
            peak_100 = peak_100.max(out.left.abs());
        }
    }

    assert!(
        peak_100 < 0.05,
        "Listen mode must reject 100 Hz tone, got {peak_100}"
    );
}

#[test]
fn test_de_esser_bypass_and_reset() {
    let mut de_esser = VocalDeEsser::new(48000.0, 6000.0, -20.0, 4.0);

    // Trigger gain reduction
    for i in 0..500 {
        let t = i as f32 / 48000.0;
        let s = 0.8 * (2.0 * std::f32::consts::PI * 6000.0 * t).sin();
        de_esser.process_frame(AudioFrame::new(s, s));
    }
    assert!(de_esser.gain_reduction_db() > 5.0);

    // Bypass
    de_esser.set_enabled(false);
    assert!(!de_esser.is_enabled());
    let frame = AudioFrame::new(0.77, -0.66);
    let out = de_esser.process_frame(frame);
    assert_eq!(out.left, 0.77);
    assert_eq!(out.right, -0.66);

    // Reset clears state
    de_esser.set_enabled(true);
    de_esser.reset();
    assert_eq!(de_esser.gain_reduction_db(), 0.0);
}

#[test]
fn test_de_esser_nan_inf_protection() {
    let mut de_esser = VocalDeEsser::new(48000.0, 6000.0, -20.0, 4.0);

    let nan_frame = AudioFrame::new(f32::NAN, f32::INFINITY);
    let out = de_esser.process_frame(nan_frame);
    assert!(out.is_finite(), "Output must be sanitized to finite values");

    // Clean frame afterwards must process without producing NaN
    let normal_frame = AudioFrame::new(0.5, 0.5);
    let out2 = de_esser.process_frame(normal_frame);
    assert!(out2.is_finite());
}

#[test]
fn test_de_esser_process_block_matches_process_frame() {
    let mut de1 = VocalDeEsser::new(48000.0, 6000.0, -20.0, 4.0);
    let mut de2 = VocalDeEsser::new(48000.0, 6000.0, -20.0, 4.0);

    let mut frames1 = Vec::new();
    let mut frames2 = Vec::new();
    for i in 0..128 {
        let t = i as f32 / 48000.0;
        let s = 0.8 * (2.0 * std::f32::consts::PI * 6000.0 * t).sin();
        let frame = AudioFrame::new(s, s * 0.9);
        frames1.push(frame);
        frames2.push(frame);
    }

    // Process via process_frame
    for f in frames1.iter_mut() {
        *f = de1.process_frame(*f);
    }

    // Process via process_block
    de2.process_block(&mut frames2);

    for (i, (f1, f2)) in frames1.iter().zip(frames2.iter()).enumerate() {
        assert!(
            (f1.left - f2.left).abs() < 1e-6 && (f1.right - f2.right).abs() < 1e-6,
            "Block processing mismatch at frame {i}: {f1:?} vs {f2:?}"
        );
    }
}

#[test]
fn test_de_esser_audio_effect_trait_object() {
    let mut effect: Box<dyn AudioEffect> = Box::new(VocalDeEsser::new(48000.0, 6000.0, -20.0, 4.0));
    assert_eq!(effect.name(), "Vocal De-Esser");
    assert!(effect.is_enabled());

    let out = effect.process_frame(AudioFrame::new(0.8, 0.8));
    assert!(out.left < 0.8);

    effect.set_enabled(false);
    assert!(!effect.is_enabled());
    let bypassed = effect.process_frame(AudioFrame::new(0.5, 0.5));
    assert_eq!(bypassed.left, 0.5);
}

#[test]
fn test_de_esser_parameter_clamping_and_getters() {
    let mut de = VocalDeEsser::new(-100.0, 99999.0, 50.0, 0.1);

    // Fallbacks and clamps
    assert_eq!(de.sample_rate(), 48000.0);
    assert!(de.frequency() <= 48000.0 * 0.499);
    assert_eq!(de.threshold_db(), 0.0);
    assert_eq!(de.ratio(), 1.0);

    // Valid updates
    de.set_sample_rate(96000.0);
    assert_eq!(de.sample_rate(), 96000.0);

    de.set_frequency(7500.0);
    assert_eq!(de.frequency(), 7500.0);

    de.set_threshold_db(-15.0);
    assert_eq!(de.threshold_db(), -15.0);

    de.set_ratio(6.0);
    assert_eq!(de.ratio(), 6.0);

    de.set_attack_ms(2.5);
    assert_eq!(de.attack_ms(), 2.5);

    de.set_release_ms(80.0);
    assert_eq!(de.release_ms(), 80.0);
}

#[test]
fn test_de_esser_serde_roundtrip() {
    let mut original = VocalDeEsser::new(48000.0, 6500.0, -18.0, 5.0);
    original.set_attack_ms(1.5);
    original.set_release_ms(60.0);
    original.set_mode(DeEsserMode::WideBand);

    let json = serde_json::to_string(&original).expect("Serialization failed");
    let deserialized: VocalDeEsser = serde_json::from_str(&json).expect("Deserialization failed");

    assert_eq!(original, deserialized);
    assert_eq!(deserialized.frequency(), 6500.0);
    assert_eq!(deserialized.threshold_db(), -18.0);
    assert_eq!(deserialized.ratio(), 5.0);
    assert_eq!(deserialized.attack_ms(), 1.5);
    assert_eq!(deserialized.release_ms(), 60.0);
    assert_eq!(deserialized.mode(), DeEsserMode::WideBand);
}

#[test]
fn test_vocal_high_pass_filter() {
    let mut hpf = vocal_high_pass(48000.0);
    assert_eq!(hpf.name(), "High-Pass Filter");
    assert_eq!(hpf.frequency(), 80.0);

    // 20 Hz sub-bass rumble should be attenuated by > 20 dB
    let mut max_20hz = 0.0f32;
    for i in 0..2000 {
        let t = i as f32 / 48000.0;
        let s = (2.0 * std::f32::consts::PI * 20.0 * t).sin();
        let out = hpf.process_frame(AudioFrame::new(s, s));
        if i > 1500 {
            max_20hz = max_20hz.max(out.left.abs());
        }
    }
    assert!(
        max_20hz < 0.1,
        "20 Hz sub-bass must be heavily attenuated by 80 Hz HPF, got peak {max_20hz}"
    );

    // 1000 Hz voice tone should pass unattenuated (near 1.0)
    hpf.reset();
    let mut max_1000hz = 0.0f32;
    for i in 0..1000 {
        let t = i as f32 / 48000.0;
        let s = (2.0 * std::f32::consts::PI * 1000.0 * t).sin();
        let out = hpf.process_frame(AudioFrame::new(s, s));
        if i > 500 {
            max_1000hz = max_1000hz.max(out.left.abs());
        }
    }
    assert!(
        (max_1000hz - 1.0).abs() < 0.02,
        "1000 Hz vocal midrange must pass transparently, got peak {max_1000hz}"
    );
}

#[test]
fn test_vocal_compressor_preset() {
    let mut comp = vocal_compressor(48000.0);
    assert_eq!(comp.name(), "Compressor");
    assert_eq!(comp.threshold_db(), -18.0);
    assert_eq!(comp.ratio(), 3.0);
    assert_eq!(comp.attack_ms(), 10.0);
    assert_eq!(comp.release_ms(), 100.0);
    assert_eq!(comp.makeup_gain_db(), 2.0);

    // Quiet voice (-30 dBFS) should receive makeup gain (+2 dB) with no compression
    let quiet_amp = 10.0f32.powf(-30.0 / 20.0);
    let mut out = AudioFrame::ZERO;
    for _ in 0..500 {
        out = comp.process_frame(AudioFrame::new(quiet_amp, quiet_amp));
    }
    let expected_quiet = 10.0f32.powf((-30.0 + 2.0) / 20.0);
    assert!(
        (out.left - expected_quiet).abs() < 0.005,
        "Quiet signal below threshold should be amplified by makeup gain"
    );
}
