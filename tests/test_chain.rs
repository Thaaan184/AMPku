use ampku::core::atomic_float::AtomicF32;
use ampku::core::frame::AudioFrame;
use ampku::dsp::filters::biquad::{BiquadFilter, FilterType};
use ampku::dsp::saturation::distortion::Distortion;
use ampku::dsp::saturation::overdrive::Overdrive;
use ampku::dsp::traits::AudioEffect;
use ampku::engine::chain::{ChainCommand, EffectChain};
use ampku::engine::channel::{
    calculate_pan_gains, db_to_linear, linear_to_db, ChannelStrip, PanLaw,
};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread;

// ============================================================================
// 1. EffectChain Tests
// ============================================================================

#[test]
fn test_effect_chain_add_and_process() {
    let mut chain = EffectChain::new();
    chain.add_effect(Box::new(Overdrive::new(2.0, 0.5, 0.8)));
    assert_eq!(chain.len(), 1);
    assert!(!chain.is_empty());
    let out = chain.process_frame(AudioFrame::new(0.5, 0.5));
    assert!(out.left > 0.0);
    assert!(out.right > 0.0);
}

#[test]
fn test_effect_chain_empty_passthrough() {
    let mut chain = EffectChain::new();
    assert_eq!(chain.len(), 0);
    assert!(chain.is_empty());

    let input = AudioFrame::new(0.42, -0.73);
    let out = chain.process_frame(input);
    assert_eq!(out, input);

    let mut block = [input, input];
    chain.process_block(&mut block);
    assert_eq!(block[0], input);
    assert_eq!(block[1], input);
}

#[test]
fn test_effect_chain_insert_remove_and_get() {
    let mut chain = EffectChain::with_capacity(4);

    let idx0 = chain.add_effect(Box::new(Overdrive::new(1.0, 0.5, 0.5)));
    assert_eq!(idx0, 0);

    let idx1 = chain.add_effect(Box::new(Distortion::new(1.0, 0.5, 0.5)));
    assert_eq!(idx1, 1);
    assert_eq!(chain.len(), 2);

    assert_eq!(chain.get_effect(0).unwrap().name(), "Overdrive");
    assert_eq!(chain.get_effect(1).unwrap().name(), "Distortion");

    // Insert BiquadFilter at index 1
    chain.insert_effect(
        1,
        Box::new(BiquadFilter::new(
            FilterType::LowPass,
            48000.0,
            1000.0,
            0.707,
            0.0,
        )),
    );
    assert_eq!(chain.len(), 3);
    assert_eq!(chain.get_effect(0).unwrap().name(), "Overdrive");
    assert_eq!(chain.get_effect(1).unwrap().name(), "Low-Pass Filter");
    assert_eq!(chain.get_effect(2).unwrap().name(), "Distortion");

    // Remove middle effect
    let removed = chain.remove_effect(1);
    assert!(removed.is_some());
    assert_eq!(removed.unwrap().name(), "Low-Pass Filter");
    assert_eq!(chain.len(), 2);
    assert_eq!(chain.get_effect(0).unwrap().name(), "Overdrive");
    assert_eq!(chain.get_effect(1).unwrap().name(), "Distortion");

    // Out of bounds remove
    assert!(chain.remove_effect(99).is_none());
}

#[test]
fn test_effect_chain_swap_and_move() {
    let mut chain = EffectChain::new();
    chain.add_effect(Box::new(Overdrive::new(1.0, 0.5, 0.5)));
    chain.add_effect(Box::new(Distortion::new(1.0, 0.5, 0.5)));

    assert_eq!(chain.get_effect(0).unwrap().name(), "Overdrive");
    assert_eq!(chain.get_effect(1).unwrap().name(), "Distortion");

    // Swap
    assert!(chain.swap_effects(0, 1));
    assert_eq!(chain.get_effect(0).unwrap().name(), "Distortion");
    assert_eq!(chain.get_effect(1).unwrap().name(), "Overdrive");

    // Invalid swap
    assert!(!chain.swap_effects(0, 5));

    // Move
    chain.add_effect(Box::new(BiquadFilter::new(
        FilterType::HighPass,
        48000.0,
        1000.0,
        0.707,
        0.0,
    )));
    assert_eq!(chain.len(), 3);
    assert_eq!(chain.get_effect(2).unwrap().name(), "High-Pass Filter");

    // Move from 2 to 0
    assert!(chain.move_effect(2, 0));
    assert_eq!(chain.get_effect(0).unwrap().name(), "High-Pass Filter");
    assert_eq!(chain.get_effect(1).unwrap().name(), "Distortion");
    assert_eq!(chain.get_effect(2).unwrap().name(), "Overdrive");

    // Invalid move
    assert!(!chain.move_effect(10, 0));
}

#[test]
fn test_effect_chain_bypass_individual_and_global() {
    let mut chain = EffectChain::new();
    chain.add_effect(Box::new(Overdrive::new(10.0, 1.0, 1.0)));

    let test_frame = AudioFrame::new(0.2, 0.2);
    let wet_out = chain.process_frame(test_frame);
    assert!((wet_out.left - test_frame.left).abs() > 0.05);

    // Bypass individual effect
    chain.bypass_effect(0, true);
    assert_eq!(chain.is_effect_enabled(0), Some(false));
    let dry_out = chain.process_frame(test_frame);
    assert_eq!(dry_out, test_frame);

    // Re-enable
    chain.set_effect_enabled(0, true);
    assert_eq!(chain.is_effect_enabled(0), Some(true));
    let wet_again = chain.process_frame(test_frame);
    assert!((wet_again.left - test_frame.left).abs() > 0.05);

    // Global chain bypass
    chain.set_enabled(false);
    assert!(!chain.is_enabled());
    let globally_bypassed = chain.process_frame(test_frame);
    assert_eq!(globally_bypassed, test_frame);
}

#[test]
fn test_effect_chain_process_block_matches_frame() {
    let mut chain1 = EffectChain::new();
    chain1.add_effect(Box::new(Overdrive::new(4.0, 0.7, 0.8)));

    let mut chain2 = EffectChain::new();
    chain2.add_effect(Box::new(Overdrive::new(4.0, 0.7, 0.8)));

    let frames = vec![
        AudioFrame::new(0.1, -0.1),
        AudioFrame::new(0.5, 0.5),
        AudioFrame::new(-0.8, 0.8),
    ];

    let mut block_frames = frames.clone();
    chain1.process_block(&mut block_frames);

    let mut frame_by_frame = Vec::new();
    for f in frames {
        frame_by_frame.push(chain2.process_frame(f));
    }

    for (b, f) in block_frames.iter().zip(frame_by_frame.iter()) {
        assert!((b.left - f.left).abs() < 1e-6);
        assert!((b.right - f.right).abs() < 1e-6);
    }
}

#[test]
fn test_effect_chain_reset_and_clear() {
    let mut chain = EffectChain::new();
    chain.add_effect(Box::new(Overdrive::new(2.0, 0.5, 0.5)));
    chain.add_effect(Box::new(Distortion::new(2.0, 0.5, 0.5)));

    chain.reset(); // Should reset without panic

    chain.clear();
    assert_eq!(chain.len(), 0);
    assert!(chain.is_empty());
}

#[test]
fn test_effect_chain_sanitization_nan_inf() {
    let mut chain = EffectChain::new();
    chain.add_effect(Box::new(Overdrive::new(2.0, 0.5, 0.5)));

    let nan_frame = AudioFrame::new(f32::NAN, f32::INFINITY);
    let out = chain.process_frame(nan_frame);
    assert!(out.is_finite());
}

#[test]
fn test_effect_chain_command_queue() {
    let (mut chain, mut producer) = EffectChain::with_command_queue(16);
    chain.add_effect(Box::new(Overdrive::new(5.0, 1.0, 1.0)));
    chain.add_effect(Box::new(Distortion::new(5.0, 1.0, 1.0)));

    assert_eq!(chain.get_effect(0).unwrap().name(), "Overdrive");
    assert_eq!(chain.get_effect(1).unwrap().name(), "Distortion");

    // Push Swap command
    assert!(producer
        .push(ChainCommand::Swap {
            index_a: 0,
            index_b: 1
        })
        .is_ok());

    // Processing frame automatically triggers process_commands()
    let _ = chain.process_frame(AudioFrame::new(0.2, 0.2));
    assert_eq!(chain.get_effect(0).unwrap().name(), "Distortion");
    assert_eq!(chain.get_effect(1).unwrap().name(), "Overdrive");

    // Push SetEnabled (bypass) command
    assert!(producer
        .push(ChainCommand::SetEnabled {
            index: 0,
            enabled: false
        })
        .is_ok());
    let _ = chain.process_frame(AudioFrame::new(0.2, 0.2));
    assert_eq!(chain.is_effect_enabled(0), Some(false));

    // Push Move command
    assert!(producer.push(ChainCommand::Move { from: 1, to: 0 }).is_ok());
    let _ = chain.process_frame(AudioFrame::new(0.2, 0.2));
    assert_eq!(chain.get_effect(0).unwrap().name(), "Overdrive");
    assert_eq!(chain.get_effect(1).unwrap().name(), "Distortion");

    // Push Clear command
    assert!(producer.push(ChainCommand::Clear).is_ok());
    let _ = chain.process_frame(AudioFrame::new(0.2, 0.2));
    assert_eq!(chain.is_effect_enabled(0), Some(false));
    assert_eq!(chain.is_effect_enabled(1), Some(false));
}

#[test]
fn test_effect_chain_trait_object() {
    let mut chain = EffectChain::new();
    chain.add_effect(Box::new(Overdrive::new(2.0, 0.5, 0.8)));

    let mut boxed_chain: Box<dyn AudioEffect> = Box::new(chain);
    assert_eq!(boxed_chain.name(), "Effect Chain");
    assert!(boxed_chain.is_enabled());

    let out = boxed_chain.process_frame(AudioFrame::new(0.3, 0.3));
    assert!(out.left > 0.0);
}

// ============================================================================
// 2. ChannelStrip Tests
// ============================================================================

#[test]
fn test_channel_strip_defaults() {
    let strip = ChannelStrip::new();
    assert_eq!(strip.name(), "Channel");
    assert_eq!(strip.gain(), 1.0);
    assert!((strip.volume_db() - 0.0).abs() < 1e-4);
    assert_eq!(strip.pan(), 0.0);
    assert!(!strip.is_muted());
    assert!(!strip.is_solo());
    assert!(!strip.is_mono());
    assert!(strip.is_stereo());
    assert_eq!(strip.sample_rate(), 48000.0);

    let (peak_l, peak_r) = strip.peak();
    assert_eq!(peak_l, 0.0);
    assert_eq!(peak_r, 0.0);
    assert!(!strip.is_clipped());
}

#[test]
fn test_channel_strip_custom_constructor_and_naming() {
    let mut strip = ChannelStrip::with_sample_rate("Guitar Lead", 96000.0);
    assert_eq!(strip.name(), "Guitar Lead");
    assert_eq!(strip.sample_rate(), 96000.0);

    strip.set_name("Acoustic");
    assert_eq!(strip.name(), "Acoustic");
}

#[test]
fn test_channel_strip_gain_and_volume_db() {
    let mut strip = ChannelStrip::new();

    // Unity gain
    let out = strip.process_frame(AudioFrame::new(0.5, 0.5));
    // Constant power pan at center (0.0): each channel is cos(pi/4) = sqrt(2)/2 ~= 0.7071
    let expected_center = 0.5 * std::f32::consts::FRAC_1_SQRT_2;
    assert!((out.left - expected_center).abs() < 1e-4);

    // Double gain
    strip.set_gain(2.0);
    assert_eq!(strip.gain(), 2.0);
    assert!((strip.volume_db() - 6.0206).abs() < 0.01);
    let out_2x = strip.process_frame(AudioFrame::new(0.5, 0.5));
    assert!((out_2x.left - (expected_center * 2.0)).abs() < 1e-4);

    // Set dB
    strip.set_volume_db(-6.0206);
    assert!((strip.gain() - 0.5).abs() < 1e-3);

    // Mute via dB
    strip.set_volume_db(-120.0);
    assert_eq!(strip.gain(), 0.0);
    let silent = strip.process_frame(AudioFrame::new(0.5, 0.5));
    assert_eq!(silent, AudioFrame::ZERO);
}

#[test]
fn test_channel_strip_pan_matrix() {
    let mut strip = ChannelStrip::new();

    // Center pan
    strip.set_pan(0.0);
    let (g_l, g_r) = strip.pan_gains();
    assert!((g_l - g_r).abs() < 1e-6);

    // Hard left (-1.0)
    strip.set_pan(-1.0);
    assert_eq!(strip.pan(), -1.0);
    let out_left = strip.process_frame(AudioFrame::new(0.5, 0.5));
    assert!(out_left.left > 0.4);
    assert_eq!(out_left.right, 0.0);

    // Hard right (+1.0)
    strip.set_pan(1.0);
    assert_eq!(strip.pan(), 1.0);
    let out_right = strip.process_frame(AudioFrame::new(0.5, 0.5));
    assert_eq!(out_right.left, 0.0);
    assert!(out_right.right > 0.4);

    // Linear Pan Law
    strip.set_pan_law(PanLaw::Linear);
    strip.set_pan(0.0);
    let out_lin_center = strip.process_frame(AudioFrame::new(0.5, 0.5));
    assert!((out_lin_center.left - 0.5).abs() < 1e-4);
    assert!((out_lin_center.right - 0.5).abs() < 1e-4);
}

#[test]
fn test_channel_strip_mute_and_solo() {
    let mut strip = ChannelStrip::new();
    let frame = AudioFrame::new(0.5, 0.5);

    // Mute
    strip.set_muted(true);
    assert!(strip.is_muted());
    let muted_out = strip.process_frame(frame);
    assert_eq!(muted_out, AudioFrame::ZERO);

    // Toggle mute
    assert!(!strip.toggle_mute());
    assert!(!strip.is_muted());
    let active_out = strip.process_frame(frame);
    assert!(active_out.left > 0.0);

    // Solo test
    // If another channel is soloed, this non-soloed channel must be muted
    let solo_silenced = strip.process_frame_with_solo(frame, true);
    assert_eq!(solo_silenced, AudioFrame::ZERO);

    // If this channel is also soloed, it plays
    strip.set_solo(true);
    assert!(strip.is_solo());
    let solo_playing = strip.process_frame_with_solo(frame, true);
    assert!(solo_playing.left > 0.0);

    // Toggle solo
    assert!(!strip.toggle_solo());
    assert!(!strip.is_solo());
}

#[test]
fn test_channel_strip_mono_downmix() {
    let mut strip = ChannelStrip::new();
    strip.set_pan_law(PanLaw::Linear);
    strip.set_pan(0.0);

    // Stereo input with left only
    let stereo_frame = AudioFrame::new(1.0, 0.0);

    // Stereo mode: left passes, right stays 0
    let out_stereo = strip.process_frame(stereo_frame);
    assert!((out_stereo.left - 1.0).abs() < 1e-4);
    assert_eq!(out_stereo.right, 0.0);

    // Enable Mono mode
    strip.set_mono(true);
    assert!(strip.is_mono());
    assert!(!strip.is_stereo());

    // In mono mode: (1.0 + 0.0) / 2 = 0.5 on both channels
    let out_mono = strip.process_frame(stereo_frame);
    assert!((out_mono.left - 0.5).abs() < 1e-4);
    assert!((out_mono.right - 0.5).abs() < 1e-4);

    // Set stereo back
    strip.set_stereo(true);
    assert!(strip.is_stereo());
}

#[test]
fn test_channel_strip_vu_metering_and_clipping() {
    let mut strip = ChannelStrip::new();
    let meter = strip.meter();

    assert_eq!(meter.peak(), (0.0, 0.0));
    assert_eq!(meter.rms(), (0.0, 0.0));
    assert!(!strip.is_clipped());

    // Process non-clipping burst for ~300ms (14400 frames)
    for _ in 0..15000 {
        strip.process_frame(AudioFrame::new(0.5, 0.5));
    }

    let (peak_l, peak_r) = strip.peak();
    let (rms_l, rms_r) = strip.rms();
    assert!(peak_l > 0.2);
    assert!(peak_r > 0.2);
    assert!(rms_l > 0.1);
    assert!(rms_r > 0.1);
    assert!(!strip.is_clipped());

    // Peak and RMS in dB
    let (peak_db_l, _) = strip.peak_db();
    let (rms_db_l, _) = strip.rms_db();
    assert!(peak_db_l > -20.0);
    assert!(rms_db_l > -20.0);

    // Send clipping signal (>= 1.0)
    strip.process_frame(AudioFrame::new(1.5, 1.5));
    assert!(strip.is_clipped());

    // Reset meter
    strip.reset_meter();
    assert_eq!(strip.peak(), (0.0, 0.0));
    assert_eq!(strip.rms(), (0.0, 0.0));
    assert!(!strip.is_clipped());
}

#[test]
fn test_channel_strip_block_processing() {
    let mut strip = ChannelStrip::new();
    strip.add_effect(Box::new(Overdrive::new(2.0, 0.5, 0.8)));

    let mut frames = vec![
        AudioFrame::new(0.2, 0.2),
        AudioFrame::new(0.4, 0.4),
        AudioFrame::new(0.6, 0.6),
        AudioFrame::new(0.8, 0.8),
    ];

    strip.process_block(&mut frames);

    for f in frames {
        assert!(f.left > 0.0);
        assert!(f.right > 0.0);
        assert!(f.is_finite());
    }

    assert!(strip.peak().0 > 0.0);
}

#[test]
fn test_channel_strip_audio_effect_trait() {
    let mut strip = ChannelStrip::new();
    strip.add_effect(Box::new(Overdrive::new(2.0, 0.5, 0.8)));

    let mut boxed: Box<dyn AudioEffect> = Box::new(strip);
    assert_eq!(boxed.name(), "Channel Strip");
    assert!(boxed.is_enabled());

    let out = boxed.process_frame(AudioFrame::new(0.4, 0.4));
    assert!(out.left > 0.0);

    boxed.set_enabled(false);
    assert!(!boxed.is_enabled());
    let muted_out = boxed.process_frame(AudioFrame::new(0.4, 0.4));
    assert_eq!(muted_out, AudioFrame::ZERO);
}

// ============================================================================
// 3. Helper and AtomicF32 Tests
// ============================================================================

#[test]
fn test_db_linear_conversions() {
    assert_eq!(db_to_linear(0.0), 1.0);
    assert!((db_to_linear(-6.0206) - 0.5).abs() < 1e-3);
    assert_eq!(db_to_linear(-120.0), 0.0);

    assert_eq!(linear_to_db(1.0), 0.0);
    assert!((linear_to_db(0.5) - -6.0206).abs() < 1e-3);
    assert_eq!(linear_to_db(0.0), -120.0);
}

#[test]
fn test_pan_gains_calculation() {
    // Constant power
    let (cl_l, cl_r) = calculate_pan_gains(0.0, PanLaw::ConstantPower);
    assert!((cl_l - cl_r).abs() < 1e-6);
    assert!((cl_l.powi(2) + cl_r.powi(2) - 1.0).abs() < 1e-4);

    let (left_l, left_r) = calculate_pan_gains(-1.0, PanLaw::ConstantPower);
    assert!((left_l - 1.0).abs() < 1e-4);
    assert!(left_r.abs() < 1e-4);

    let (right_l, right_r) = calculate_pan_gains(1.0, PanLaw::ConstantPower);
    assert!(right_l.abs() < 1e-4);
    assert!((right_r - 1.0).abs() < 1e-4);

    // Linear
    let (lin_l, lin_r) = calculate_pan_gains(0.0, PanLaw::Linear);
    assert_eq!(lin_l, 1.0);
    assert_eq!(lin_r, 1.0);
}

#[test]
fn test_atomic_f32_concurrent_access() {
    let atomic_val = Arc::new(AtomicF32::new(10.0));

    assert_eq!(atomic_val.get(), 10.0);
    atomic_val.set(20.5);
    assert_eq!(atomic_val.get(), 20.5);
    assert_eq!(atomic_val.swap(30.0, Ordering::SeqCst), 20.5);
    assert_eq!(atomic_val.get(), 30.0);

    // Concurrent update_max
    let val_clone1 = Arc::clone(&atomic_val);
    let val_clone2 = Arc::clone(&atomic_val);

    let t1 = thread::spawn(move || {
        for i in 0..1000 {
            val_clone1.update_max(i as f32);
        }
    });

    let t2 = thread::spawn(move || {
        for i in 0..1000 {
            val_clone2.update_max((i * 2) as f32);
        }
    });

    t1.join().unwrap();
    t2.join().unwrap();

    assert_eq!(atomic_val.get(), 1998.0);
}
