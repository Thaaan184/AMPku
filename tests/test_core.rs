use ampku::core::frame::AudioFrame;
use ampku::core::ring_buffer::{create_audio_ring_buffer, AudioRingBuffer};

#[test]
fn test_audio_frame_mono_stereo_conversion() {
    let mono = 0.5f32;
    let frame = AudioFrame::from_mono(mono);
    assert_eq!(frame.left, 0.5);
    assert_eq!(frame.right, 0.5);
    assert_eq!(frame.to_mono(), 0.5);
}

#[test]
fn test_ring_buffer_push_pop() {
    let (mut producer, mut consumer) = create_audio_ring_buffer::<AudioFrame>(128);
    assert!(producer
        .push(AudioFrame {
            left: 0.1,
            right: -0.1,
        })
        .is_ok());
    let popped = consumer.pop().unwrap();
    assert_eq!(popped.left, 0.1);
}

#[test]
fn test_audio_frame_arithmetic_and_clamp() {
    let a = AudioFrame::new(0.4, -0.6);
    let b = AudioFrame::new(0.2, 0.3);

    let sum = a + b;
    assert!((sum.left - 0.6).abs() < 1e-6);
    assert!((sum.right - (-0.3)).abs() < 1e-6);

    let scaled = a * 2.0;
    assert!((scaled.left - 0.8).abs() < 1e-6);
    assert!((scaled.right - (-1.2)).abs() < 1e-6);

    let clamped = scaled.clamp(-1.0, 1.0);
    assert_eq!(clamped.left, 0.8);
    assert_eq!(clamped.right, -1.0);
}

#[test]
fn test_audio_frame_full_operations() {
    let mut frame = AudioFrame::ZERO;
    assert_eq!(frame.left, 0.0);
    assert_eq!(frame.right, 0.0);

    frame += AudioFrame::new(0.5, 0.25);
    assert_eq!(frame, AudioFrame::new(0.5, 0.25));

    frame -= AudioFrame::new(0.1, 0.05);
    assert!((frame.left - 0.4).abs() < 1e-6);
    assert!((frame.right - 0.2).abs() < 1e-6);

    frame *= 2.0;
    assert!((frame.left - 0.8).abs() < 1e-6);
    assert!((frame.right - 0.4).abs() < 1e-6);

    frame /= 2.0;
    assert!((frame.left - 0.4).abs() < 1e-6);
    assert!((frame.right - 0.2).abs() < 1e-6);

    let neg = -frame;
    assert!((neg.left - (-0.4)).abs() < 1e-6);
    assert!((neg.right - (-0.2)).abs() < 1e-6);

    assert_eq!(frame.peak(), 0.4);
    assert_eq!(neg.abs(), frame);

    let tuple: (f32, f32) = frame.into();
    assert_eq!(tuple, (frame.left, frame.right));

    let array: [f32; 2] = frame.into();
    assert_eq!(array, [frame.left, frame.right]);

    assert_eq!(AudioFrame::from((0.3, 0.7)), AudioFrame::new(0.3, 0.7));
    assert_eq!(AudioFrame::from([0.3, 0.7]), AudioFrame::new(0.3, 0.7));
}

#[test]
fn test_audio_frame_nan_inf_sanitization() {
    let dirty = AudioFrame::new(f32::NAN, f32::INFINITY);
    assert!(!dirty.is_finite());

    let cleaned = dirty.sanitize();
    assert!(cleaned.is_finite());
    assert_eq!(cleaned.left, 0.0);
    assert_eq!(cleaned.right, 0.0);

    let neg_inf = AudioFrame::new(f32::NEG_INFINITY, 0.5);
    let cleaned_neg = neg_inf.sanitize();
    assert_eq!(cleaned_neg.left, 0.0);
    assert_eq!(cleaned_neg.right, 0.5);
}

#[test]
fn test_ring_buffer_capacity_and_fifo() {
    let (mut producer, mut consumer) = AudioRingBuffer::<AudioFrame>::new(4);
    assert_eq!(producer.slots(), 4);
    assert_eq!(consumer.slots(), 0);

    for i in 0..4 {
        assert!(producer.push(AudioFrame::from_mono(i as f32)).is_ok());
    }

    // Buffer is full (capacity 4)
    assert!(producer.is_full());
    assert!(producer.push(AudioFrame::from_mono(99.0)).is_err());

    // Pop in FIFO order
    for i in 0..4 {
        let frame = consumer.pop().expect("pop failed");
        assert_eq!(frame.left, i as f32);
        assert_eq!(frame.right, i as f32);
    }

    // Now empty
    assert!(consumer.is_empty());
    assert!(consumer.pop().is_err());
}

#[test]
fn test_audio_frame_serialization() {
    let original = AudioFrame::new(0.123, -0.456);
    let json = serde_json::to_string(&original).expect("serialize failed");
    let deserialized: AudioFrame = serde_json::from_str(&json).expect("deserialize failed");
    assert_eq!(original, deserialized);
}
