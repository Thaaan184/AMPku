use ampku::core::frame::AudioFrame;
use ampku::dsp::time::delay::StereoDelay;
use ampku::dsp::time::reverb::StereoReverb;
use ampku::dsp::traits::AudioEffect;

#[test]
fn test_delay_ring_buffer_playback() {
    let mut delay = StereoDelay::new(48000.0, 10.0, 0.5, 1.0); // 10ms delay, 100% wet
    let impulse = AudioFrame::new(1.0, 1.0);
    let _ = delay.process_frame(impulse);
    let mut found = false;
    for _ in 0..500 {
        let out = delay.process_frame(AudioFrame::new(0.0, 0.0));
        if out.left.abs() > 0.01 {
            found = true;
            break;
        }
    }
    assert!(found);
}

#[test]
fn test_delay_timing_accuracy() {
    let sample_rate = 48000.0;
    let delay_ms = 10.0; // 480 samples exactly
    let mut delay = StereoDelay::new(sample_rate, delay_ms, 0.0, 1.0);

    // Initial impulse
    let out0 = delay.process_frame(AudioFrame::new(1.0, 1.0));
    assert_eq!(out0.left, 0.0, "Wet output at t=0 should be zero");

    let mut peak_sample_idx = 0;
    let mut peak_val = 0.0;

    for i in 1..=500 {
        let out = delay.process_frame(AudioFrame::ZERO);
        if out.left.abs() > peak_val {
            peak_val = out.left.abs();
            peak_sample_idx = i;
        }
    }

    assert_eq!(
        peak_sample_idx, 480,
        "Peak should emerge at exactly sample 480 (10ms at 48kHz)"
    );
    assert!(
        (peak_val - 1.0).abs() < 1e-4,
        "Impulse peak value should be 1.0 with 0 feedback"
    );
}

#[test]
fn test_delay_dry_wet_mix() {
    let sample_rate = 48000.0;

    // 100% Dry
    let mut delay_dry = StereoDelay::new(sample_rate, 10.0, 0.5, 0.0);
    let out_dry = delay_dry.process_frame(AudioFrame::new(0.8, 0.8));
    assert!(
        (out_dry.left - 0.8).abs() < 1e-4,
        "100% dry should pass input immediately"
    );

    // 50% Mix
    let mut delay_half = StereoDelay::new(sample_rate, 10.0, 0.5, 0.5);
    let out_half = delay_half.process_frame(AudioFrame::new(0.8, 0.8));
    assert!(
        (out_half.left - 0.4).abs() < 1e-4,
        "50% mix should attenuate dry signal by half"
    );

    // 100% Wet
    let mut delay_wet = StereoDelay::new(sample_rate, 10.0, 0.5, 1.0);
    let out_wet = delay_wet.process_frame(AudioFrame::new(0.8, 0.8));
    assert_eq!(
        out_wet.left, 0.0,
        "100% wet should output 0 at t=0 before delay arrives"
    );
}

#[test]
fn test_delay_feedback_decay() {
    let sample_rate = 48000.0;
    let delay_ms = 5.0; // 240 samples
    let feedback = 0.5;
    let mut delay = StereoDelay::new(sample_rate, delay_ms, feedback, 1.0);

    let _ = delay.process_frame(AudioFrame::new(1.0, 1.0));

    let mut tap1_peak = 0.0;
    let mut tap2_peak = 0.0;
    let mut tap3_peak = 0.0;

    for i in 1..=800 {
        let out = delay.process_frame(AudioFrame::ZERO);
        if i == 240 {
            tap1_peak = out.left;
        } else if i == 480 {
            tap2_peak = out.left;
        } else if i == 720 {
            tap3_peak = out.left;
        }
    }

    assert!(
        (tap1_peak - 1.0).abs() < 1e-3,
        "Tap 1 should be ~1.0, got {tap1_peak}"
    );
    assert!(
        (tap2_peak - 0.5).abs() < 1e-3,
        "Tap 2 should be ~0.5 (feedback * 1.0), got {tap2_peak}"
    );
    assert!(
        (tap3_peak - 0.25).abs() < 1e-3,
        "Tap 3 should be ~0.25 (feedback^2 * 1.0), got {tap3_peak}"
    );
}

#[test]
fn test_delay_fractional_interpolation() {
    let sample_rate = 48000.0;
    // 10.25 ms = 492.0 samples at 48 kHz
    let mut delay = StereoDelay::new(sample_rate, 10.25, 0.0, 1.0);
    let _ = delay.process_frame(AudioFrame::new(1.0, 1.0));

    let mut peak_val = 0.0;
    let mut peak_idx = 0;

    for i in 1..=600 {
        let out = delay.process_frame(AudioFrame::ZERO);
        if out.left.abs() > peak_val {
            peak_val = out.left.abs();
            peak_idx = i;
        }
    }

    assert_eq!(peak_idx, 492);
    assert!(
        (peak_val - 1.0).abs() < 1e-4,
        "Fractional interpolation should preserve unit amplitude"
    );
}

#[test]
fn test_delay_lowpass_damping() {
    let sample_rate = 48000.0;
    let delay_ms = 5.0; // 240 samples

    // Delay with NO damping (damping = 0.0)
    let mut delay_bright =
        StereoDelay::with_options(sample_rate, delay_ms, 0.7, 1.0, 0.0, false, 0.0);
    // Delay with HIGH damping (damping = 0.8)
    let mut delay_dark =
        StereoDelay::with_options(sample_rate, delay_ms, 0.7, 1.0, 0.8, false, 0.0);

    // Feed alternating Nyquist-rate burst (+1, -1, +1, -1) to test high-frequency attenuation
    for step in 0..10 {
        let val = if step % 2 == 0 { 1.0 } else { -1.0 };
        let frame = AudioFrame::new(val, val);
        let _ = delay_bright.process_frame(frame);
        let _ = delay_dark.process_frame(frame);
    }

    let mut energy_bright = 0.0;
    let mut energy_dark = 0.0;

    // Collect energy across second repeat (samples 400..600)
    for _ in 10..800 {
        let out_b = delay_bright.process_frame(AudioFrame::ZERO);
        let out_d = delay_dark.process_frame(AudioFrame::ZERO);
        energy_bright += out_b.left.abs();
        energy_dark += out_d.left.abs();
    }

    assert!(
        energy_bright > energy_dark * 1.5,
        "Bright delay energy ({energy_bright}) should be significantly higher than damped ({energy_dark})"
    );
}

#[test]
fn test_delay_ping_pong_cross_channel() {
    let sample_rate = 48000.0;
    let delay_ms = 5.0; // 240 samples
    let feedback = 0.6;
    let mut delay = StereoDelay::with_options(
        sample_rate,
        delay_ms,
        feedback,
        1.0,  // 100% wet
        0.0,  // no damping
        true, // ping-pong active
        0.0,  // no stereo spread (exact rhythmic ping-pong)
    );

    // Initial mono impulse
    let _ = delay.process_frame(AudioFrame::new(1.0, 1.0));

    let mut tap1 = AudioFrame::ZERO;
    let mut tap2 = AudioFrame::ZERO;
    let mut tap3 = AudioFrame::ZERO;

    for i in 1..=800 {
        let out = delay.process_frame(AudioFrame::ZERO);
        if i == 240 {
            tap1 = out;
        } else if i == 480 {
            tap2 = out;
        } else if i == 720 {
            tap3 = out;
        }
    }

    // Tap 1 (at D = 240): Should emerge on LEFT
    assert!(
        tap1.left > 0.8,
        "Tap 1 should be on LEFT channel, got {}",
        tap1.left
    );
    assert!(
        tap1.right < 0.05,
        "Tap 1 should be silent on RIGHT channel, got {}",
        tap1.right
    );

    // Tap 2 (at 2D = 480): Should bounce to RIGHT
    assert!(
        tap2.right > 0.45,
        "Tap 2 should bounce to RIGHT channel, got {}",
        tap2.right
    );
    assert!(
        tap2.left < 0.05,
        "Tap 2 should be silent on LEFT channel, got {}",
        tap2.left
    );

    // Tap 3 (at 3D = 720): Should bounce back to LEFT
    assert!(
        tap3.left > 0.25,
        "Tap 3 should bounce back to LEFT channel, got {}",
        tap3.left
    );
    assert!(
        tap3.right < 0.05,
        "Tap 3 should be silent on RIGHT channel, got {}",
        tap3.right
    );
}

#[test]
fn test_delay_stereo_spread() {
    let sample_rate = 48000.0;
    let delay_ms = 10.0; // 480 samples base
    let mut delay = StereoDelay::with_options(
        sample_rate,
        delay_ms,
        0.0,
        1.0,
        0.0,
        false, // standard stereo
        0.5,   // spread = 0.5 -> right channel has 480 * (1 + 0.5 * 0.25) = 540 samples
    );

    let _ = delay.process_frame(AudioFrame::new(1.0, 1.0));

    let mut left_peak_idx = 0;
    let mut right_peak_idx = 0;

    for i in 1..=600 {
        let out = delay.process_frame(AudioFrame::ZERO);
        if out.left > 0.5 && left_peak_idx == 0 {
            left_peak_idx = i;
        }
        if out.right > 0.5 && right_peak_idx == 0 {
            right_peak_idx = i;
        }
    }

    assert_eq!(
        left_peak_idx, 480,
        "Left peak should occur at base 480 samples"
    );
    assert_eq!(
        right_peak_idx, 540,
        "Right peak should occur at spread 540 samples"
    );
}

#[test]
fn test_delay_bypass_and_reset() {
    let mut delay = StereoDelay::new(48000.0, 10.0, 0.5, 1.0);

    // Test bypass
    delay.set_enabled(false);
    assert!(!delay.is_enabled());
    let sig = AudioFrame::new(0.77, -0.44);
    let out_bypassed = delay.process_frame(sig);
    assert_eq!(
        out_bypassed, sig,
        "Bypassed delay must pass signal untouched"
    );

    // Re-enable and fill buffer
    delay.set_enabled(true);
    let _ = delay.process_frame(AudioFrame::new(1.0, 1.0));

    // Reset clears buffers
    delay.reset();
    for _ in 0..500 {
        let out = delay.process_frame(AudioFrame::ZERO);
        assert_eq!(out, AudioFrame::ZERO, "Reset delay must output zeros");
    }
}

#[test]
fn test_delay_nan_inf_protection() {
    let mut delay = StereoDelay::new(48000.0, 10.0, 0.5, 0.5);
    let nan_frame = AudioFrame::new(f32::NAN, f32::INFINITY);
    let out = delay.process_frame(nan_frame);
    assert!(out.is_finite(), "Output must be sanitized from NaN/Inf");

    // Verify buffer was not poisoned
    let normal_out = delay.process_frame(AudioFrame::new(0.5, 0.5));
    assert!(normal_out.is_finite());
}

#[test]
fn test_reverb_impulse_response_and_decay() {
    let mut reverb = StereoReverb::new(48000.0, 0.7, 0.3, 1.0, 1.0); // 100% wet
    let impulse = AudioFrame::new(1.0, 1.0);
    let _ = reverb.process_frame(impulse);

    let mut energy_early = 0.0;
    let mut energy_late = 0.0;

    // Early reverberation window (first 5000 samples)
    for _ in 0..5000 {
        let out = reverb.process_frame(AudioFrame::ZERO);
        energy_early += out.left.abs() + out.right.abs();
    }

    // Late reverberation window (next 15000 samples)
    for _ in 0..15000 {
        let out = reverb.process_frame(AudioFrame::ZERO);
        energy_late += out.left.abs() + out.right.abs();
    }

    assert!(energy_early > 0.01, "Reverb must produce early reflections");
    assert!(energy_late > 0.01, "Reverb must produce sustained tail");

    // Check decay towards tail end
    let mut tail_energy = 0.0;
    for _ in 0..20000 {
        let out = reverb.process_frame(AudioFrame::ZERO);
        tail_energy += out.left.abs();
    }
    assert!(tail_energy > 0.0, "Reverb tail energy should be non-zero");
    let end_sample = reverb.process_frame(AudioFrame::ZERO);
    assert!(
        end_sample.left.abs() < 0.01,
        "Reverb tail should decay towards zero over 40000 samples"
    );
}

#[test]
fn test_reverb_stereo_decorrelation() {
    // Width = 1.0: Full stereo decorrelation
    let mut reverb_stereo = StereoReverb::new(48000.0, 0.8, 0.3, 1.0, 1.0);
    let _ = reverb_stereo.process_frame(AudioFrame::new(1.0, 1.0));

    let mut diff_sum = 0.0;
    for _ in 0..3000 {
        let out = reverb_stereo.process_frame(AudioFrame::ZERO);
        diff_sum += (out.left - out.right).abs();
    }
    assert!(
        diff_sum > 0.05,
        "Full width reverb should produce decorrelated left and right channels"
    );

    // Width = 0.0: Mono reverb (left and right identical)
    let mut reverb_mono = StereoReverb::new(48000.0, 0.8, 0.3, 0.0, 1.0);
    let _ = reverb_mono.process_frame(AudioFrame::new(1.0, 1.0));

    for _ in 0..3000 {
        let out = reverb_mono.process_frame(AudioFrame::ZERO);
        assert!(
            (out.left - out.right).abs() < 1e-5,
            "Width = 0.0 reverb must produce identical left and right outputs"
        );
    }
}

#[test]
fn test_reverb_room_size_decay_time() {
    let mut reverb_small = StereoReverb::new(48000.0, 0.1, 0.5, 1.0, 1.0);
    let mut reverb_large = StereoReverb::new(48000.0, 0.95, 0.5, 1.0, 1.0);

    let impulse = AudioFrame::new(1.0, 1.0);
    let _ = reverb_small.process_frame(impulse);
    let _ = reverb_large.process_frame(impulse);

    // Process 10,000 samples to let small room decay
    for _ in 0..10000 {
        let _ = reverb_small.process_frame(AudioFrame::ZERO);
        let _ = reverb_large.process_frame(AudioFrame::ZERO);
    }

    // Measure energy in late window (samples 10,000 .. 15,000)
    let mut small_late_energy = 0.0;
    let mut large_late_energy = 0.0;
    for _ in 0..5000 {
        let out_s = reverb_small.process_frame(AudioFrame::ZERO);
        let out_l = reverb_large.process_frame(AudioFrame::ZERO);
        small_late_energy += out_s.left.abs();
        large_late_energy += out_l.left.abs();
    }

    assert!(
        large_late_energy > small_late_energy * 3.0,
        "Large room ({large_late_energy}) should retain much more late energy than small room ({small_late_energy})"
    );
}

#[test]
fn test_reverb_bypass_and_reset() {
    let mut reverb = StereoReverb::new(48000.0, 0.7, 0.5, 1.0, 1.0);

    // Bypass
    reverb.set_enabled(false);
    assert!(!reverb.is_enabled());
    let sig = AudioFrame::new(0.55, -0.22);
    assert_eq!(reverb.process_frame(sig), sig);

    // Enable and seed with impulse
    reverb.set_enabled(true);
    let _ = reverb.process_frame(AudioFrame::new(1.0, 1.0));

    // Reset clears buffers
    reverb.reset();
    for _ in 0..3000 {
        let out = reverb.process_frame(AudioFrame::ZERO);
        assert_eq!(out, AudioFrame::ZERO, "Reset reverb must output zero");
    }
}

#[test]
fn test_reverb_nan_inf_protection() {
    let mut reverb = StereoReverb::new(48000.0, 0.7, 0.5, 1.0, 0.5);
    let nan_frame = AudioFrame::new(f32::NAN, f32::INFINITY);
    let out = reverb.process_frame(nan_frame);
    assert!(out.is_finite(), "Reverb must sanitize NaN/Inf input");

    let normal_out = reverb.process_frame(AudioFrame::new(0.5, 0.5));
    assert!(normal_out.is_finite());
}

#[test]
fn test_time_effects_process_block_matches_process_frame() {
    let sample_rate = 48000.0;

    // Test delay process_block
    let mut delay1 = StereoDelay::new(sample_rate, 5.0, 0.5, 0.5);
    let mut delay2 = delay1.clone();

    let mut input_block: Vec<AudioFrame> = (0..256)
        .map(|i| AudioFrame::new((i as f32 * 0.05).sin(), (i as f32 * 0.03).cos()))
        .collect();
    let mut expected_block = input_block.clone();

    delay1.process_block(&mut input_block);
    for frame in expected_block.iter_mut() {
        *frame = delay2.process_frame(*frame);
    }

    for (a, b) in input_block.iter().zip(expected_block.iter()) {
        assert_eq!(a.left, b.left);
        assert_eq!(a.right, b.right);
    }

    // Test reverb process_block
    let mut reverb1 = StereoReverb::new(sample_rate, 0.6, 0.4, 0.8, 0.4);
    let mut reverb2 = reverb1.clone();

    let mut rev_block: Vec<AudioFrame> = (0..256)
        .map(|i| AudioFrame::new((i as f32 * 0.02).sin(), (i as f32 * 0.04).cos()))
        .collect();
    let mut rev_expected = rev_block.clone();

    reverb1.process_block(&mut rev_block);
    for frame in rev_expected.iter_mut() {
        *frame = reverb2.process_frame(*frame);
    }

    for (a, b) in rev_block.iter().zip(rev_expected.iter()) {
        assert_eq!(a.left, b.left);
        assert_eq!(a.right, b.right);
    }
}

#[test]
fn test_time_effects_trait_object_dispatch() {
    let mut effects: Vec<Box<dyn AudioEffect>> = vec![
        Box::new(StereoDelay::new(48000.0, 10.0, 0.4, 0.5)),
        Box::new(StereoReverb::new(48000.0, 0.7, 0.5, 1.0, 0.3)),
    ];

    assert_eq!(effects[0].name(), "Stereo Delay");
    assert_eq!(effects[1].name(), "Stereo Reverb");

    let mut frame = AudioFrame::new(0.5, 0.5);
    for effect in effects.iter_mut() {
        frame = effect.process_frame(frame);
        assert!(frame.is_finite());
    }

    for effect in effects.iter_mut() {
        effect.reset();
    }
}

#[test]
fn test_time_effects_serde_json_roundtrip() {
    // StereoDelay serde
    let delay = StereoDelay::with_options(48000.0, 250.0, 0.65, 0.45, 0.3, true, 0.5);
    let json_delay = serde_json::to_string(&delay).expect("Delay serialization failed");
    let restored_delay: StereoDelay =
        serde_json::from_str(&json_delay).expect("Delay deserialization failed");

    assert_eq!(delay, restored_delay);
    assert_eq!(restored_delay.delay_ms(), 250.0);
    assert_eq!(restored_delay.feedback(), 0.65);
    assert_eq!(restored_delay.mix(), 0.45);
    assert_eq!(restored_delay.damping(), 0.3);
    assert!(restored_delay.ping_pong());
    assert_eq!(restored_delay.stereo_spread(), 0.5);

    // StereoReverb serde
    let reverb = StereoReverb::new(48000.0, 0.85, 0.25, 0.75, 0.35);
    let json_reverb = serde_json::to_string(&reverb).expect("Reverb serialization failed");
    let restored_reverb: StereoReverb =
        serde_json::from_str(&json_reverb).expect("Reverb deserialization failed");

    assert_eq!(reverb, restored_reverb);
    assert_eq!(restored_reverb.room_size(), 0.85);
    assert_eq!(restored_reverb.damping(), 0.25);
    assert_eq!(restored_reverb.width(), 0.75);
    assert_eq!(restored_reverb.mix(), 0.35);
}

#[test]
fn test_time_effects_sample_rate_switching() {
    let mut delay = StereoDelay::new(48000.0, 20.0, 0.4, 0.5);
    delay.set_sample_rate(96000.0);
    assert_eq!(delay.sample_rate(), 96000.0);
    let out = delay.process_frame(AudioFrame::new(1.0, 1.0));
    assert!(out.is_finite());

    let mut reverb = StereoReverb::new(48000.0, 0.7, 0.5, 1.0, 0.3);
    reverb.set_sample_rate(96000.0);
    assert_eq!(reverb.sample_rate(), 96000.0);
    let out_rev = reverb.process_frame(AudioFrame::new(1.0, 1.0));
    assert!(out_rev.is_finite());
}

#[test]
fn test_reverb_subnormal_flush() {
    let mut reverb = StereoReverb::new(48000.0, 0.8, 0.5, 1.0, 0.5);
    // Send a subnormal impulse
    let _ = reverb.process_frame(AudioFrame::new(1e-30, 1e-30));

    // After decay in silence, reverb output must flush to exact zero without denormals
    let mut reached_zero = false;
    for _ in 0..50000 {
        let out = reverb.process_frame(AudioFrame::ZERO);
        assert!(out.is_finite());
        if out == AudioFrame::ZERO {
            reached_zero = true;
            break;
        }
    }
    assert!(
        reached_zero,
        "Reverb must flush subnormals and reach exact zero"
    );
}

#[test]
fn test_delay_buffer_headroom_at_max_spread() {
    let sample_rate = 48000.0;
    // Set max delay of 2500ms with full stereo spread 1.0 (+25% on right channel = 3125ms)
    let mut delay = StereoDelay::with_options(sample_rate, 2500.0, 0.0, 1.0, 0.0, false, 1.0);

    // Write a unit impulse
    let _ = delay.process_frame(AudioFrame::new(1.0, 1.0));

    // Right channel delay is 2500 * 1.25 = 3125 ms = 150,000 samples
    // Run frames to verify no clamping or panic occurs at buffer boundaries
    for _ in 0..100 {
        let out = delay.process_frame(AudioFrame::ZERO);
        assert!(out.is_finite());
    }
}
