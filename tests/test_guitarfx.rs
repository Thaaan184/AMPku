use guitarfx::devices::{find_device_by_name, DeviceInfo};
use guitarfx::effects::{
    load_f32, store_f32, CleanDsp, CrunchDsp, DelayDsp, DistortionDsp, EffectEngine, EffectParams,
    OverdriveDsp, WahDsp,
};
use std::sync::atomic::Ordering;
use std::sync::Arc;

#[test]
fn test_device_matcher() {
    let devs = vec![
        DeviceInfo {
            index: 0,
            name: "Microphone (Realtek Audio)".into(),
            is_default: false,
        },
        DeviceInfo {
            index: 1,
            name: "USB Audio Device (Vention)".into(),
            is_default: true,
        },
        DeviceInfo {
            index: 2,
            name: "VoiceMeeter Aux Input (VB-Audio VoiceMeeter AUX VAIO)".into(),
            is_default: false,
        },
    ];

    let found_vention = find_device_by_name(&devs, "vention").expect("Vention not found");
    assert_eq!(found_vention.index, 1);

    let found_aux = find_device_by_name(&devs, "aux").expect("AUX not found");
    assert_eq!(found_aux.index, 2);

    let not_found = find_device_by_name(&devs, "nonexistent");
    assert!(not_found.is_none());
}

#[test]
fn test_overdrive_and_distortion_math() {
    let mut od = OverdriveDsp::new();
    let (mut left, mut right) = (0.5f32, 0.5f32);
    od.process(&mut left, &mut right, 0.5, 0.5);

    assert!(left.abs() <= 1.0, "Overdrive output exceeds 1.0");
    assert!(right.abs() <= 1.0, "Overdrive output exceeds 1.0");
    assert!(left > 0.0, "Overdrive failed to produce positive signal");

    let mut dist = DistortionDsp::new();
    let (mut d_left, mut d_right) = (0.8f32, 0.8f32);
    dist.process(&mut d_left, &mut d_right, 0.8, 0.5);

    assert!(d_left.abs() <= 1.0, "Distortion output exceeds 1.0");
    assert!(d_right.abs() <= 1.0, "Distortion output exceeds 1.0");
}

#[test]
fn test_delay_buffer() {
    let mut delay = DelayDsp::new(1.0, 48000.0);
    let (mut left, mut right) = (1.0f32, 1.0f32);

    // Initial sample with delay mix = 0.5
    delay.process(&mut left, &mut right, 10.0, 0.3, 0.5);
    assert_eq!(left, 0.5); // Direct signal halved by mix

    // Process silence for enough frames to hear echo
    let delay_frames = (0.010 * 48000.0) as usize;
    let mut echo_heard = false;
    for _ in 0..(delay_frames + 5) {
        let (mut sl, mut sr) = (0.0f32, 0.0f32);
        delay.process(&mut sl, &mut sr, 10.0, 0.3, 0.5);
        if sl > 0.0 {
            echo_heard = true;
        }
    }

    assert!(echo_heard, "Echo was not heard from delay buffer");
}

#[test]
fn test_effect_engine_processing_and_mute() {
    let params = Arc::new(EffectParams::default());
    let mut engine = EffectEngine::new(Arc::clone(&params), 48000.0);

    let input = [0.2f32; 128];
    let mut output = [0.0f32; 256];

    engine.process_mono_to_stereo(&input, &mut output);

    let out_peak = load_f32(&params.output_peak);
    assert!(out_peak > 0.0, "Output peak should be greater than 0");

    // Test mute
    params.mute.store(true, Ordering::Relaxed);
    engine.process_mono_to_stereo(&input, &mut output);
    assert!(
        output.iter().all(|&s| s == 0.0),
        "Mute failed to silence output"
    );
}

#[test]
fn test_clean_crunch_wah_math() {
    let mut clean = CleanDsp::new();
    let (mut c_l, mut c_r) = (0.4f32, 0.4f32);
    clean.process(&mut c_l, &mut c_r, 0.7, 0.5);
    assert!(
        !c_l.is_nan() && c_l.abs() <= 1.5,
        "Clean output out of bounds"
    );
    assert!(
        !c_r.is_nan() && c_r.abs() <= 1.5,
        "Clean output out of bounds"
    );

    let mut crunch = CrunchDsp::new();
    let (mut cr_l, mut cr_r) = (0.7f32, 0.7f32);
    crunch.process(&mut cr_l, &mut cr_r, 0.6, 0.5);
    assert!(
        !cr_l.is_nan() && cr_l.abs() <= 1.5,
        "Crunch output out of bounds"
    );
    assert!(
        !cr_r.is_nan() && cr_r.abs() <= 1.5,
        "Crunch output out of bounds"
    );

    let mut wah = WahDsp::new(48000.0);
    let (mut w_l, mut w_r) = (0.6f32, 0.6f32);
    wah.process(&mut w_l, &mut w_r, 0.8, 0.7);
    assert!(!w_l.is_nan() && w_l.abs() > 0.0, "Wah output invalid");
    assert!(!w_r.is_nan() && w_r.abs() > 0.0, "Wah output invalid");
}

#[test]
fn test_atomic_f32_helpers() {
    use std::f32::consts::PI;
    use std::sync::atomic::AtomicU32;
    let atom = AtomicU32::new(0);
    store_f32(&atom, PI);
    let val = load_f32(&atom);
    assert!((val - PI).abs() < 1e-6);
}
