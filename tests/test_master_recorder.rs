use ampku::audio::recorder::{i24_to_sample, sample_to_i24, RecorderError, WavRecorder};
use ampku::core::frame::AudioFrame;
use ampku::engine::master_bus::MasterBus;
use std::path::PathBuf;

#[test]
fn test_master_bus_clamps_overs() {
    let mut bus = MasterBus::new();
    let super_hot = AudioFrame::new(3.0, 3.0);
    let limited = bus.process_frame(super_hot);
    assert!(limited.left <= 1.0 && limited.right <= 1.0);
}

#[test]
fn test_recorder_start_stop() {
    let mut recorder = WavRecorder::new();
    let tmp_path = PathBuf::from("/tmp/test_rec.wav");
    assert!(recorder.start_recording(tmp_path.clone(), 48000).is_ok());
    recorder.push_frame(AudioFrame::new(0.1, 0.1));
    assert!(recorder.stop_recording().is_ok());
    assert!(tmp_path.exists());
    let _ = std::fs::remove_file(tmp_path);
}

#[test]
fn test_master_bus_negative_overs() {
    let mut bus = MasterBus::new();
    let neg_hot = AudioFrame::new(-4.0, -3.5);
    let limited = bus.process_frame(neg_hot);
    assert!(limited.left >= -1.0 && limited.right >= -1.0);
    assert!(limited.left < 0.0 && limited.right < 0.0);
}

#[test]
fn test_master_bus_transparent_below_threshold() {
    let mut bus = MasterBus::new();
    bus.set_threshold(0.90);
    bus.set_ceiling(1.0);

    let clean = AudioFrame::new(0.4, -0.6);
    let out = bus.process_frame(clean);
    assert!((out.left - 0.4).abs() < 1e-5);
    assert!((out.right - (-0.6)).abs() < 1e-5);
    assert_eq!(bus.gain_reduction_db(), 0.0);
}

#[test]
fn test_master_bus_volume_and_bypass() {
    let mut bus = MasterBus::new();
    bus.set_volume_db(-6.0);
    assert!((bus.volume_db() - (-6.0)).abs() < 0.1);

    let half = bus.gain();
    assert!((half - 0.501187).abs() < 1e-3);

    // Bypass limiter
    bus.set_bypass(true);
    assert!(bus.is_bypassed());
    let hot = AudioFrame::new(2.0, 2.0);
    let out = bus.process_frame(hot);
    // With -6dB gain, 2.0 * ~0.5012 = ~1.002
    assert!((out.left - 2.0 * half).abs() < 1e-4);
}

#[test]
fn test_master_bus_sum_channels() {
    let mut bus = MasterBus::new();
    let guitar = AudioFrame::new(0.6, 0.4);
    let mic = AudioFrame::new(0.5, 0.7);

    // Sum is (1.1, 1.1) which exceeds threshold 0.9 and is soft-limited below 1.0
    let out = bus.sum_channels(&[guitar, mic]);
    assert!(out.left <= 1.0 && out.right <= 1.0);
    assert!(out.left > 0.8 && out.right > 0.8);
    assert!(bus.gain_reduction_db() < 0.0);
}

#[test]
fn test_master_bus_process_block() {
    let mut bus = MasterBus::new();
    let mut block = [
        AudioFrame::new(0.2, 0.2),
        AudioFrame::new(2.5, 2.5),
        AudioFrame::new(-3.0, -3.0),
    ];
    bus.process_block(&mut block);

    assert!((block[0].left - 0.2).abs() < 1e-4);
    assert!(block[1].left <= 1.0 && block[1].left > 0.0);
    assert!(block[2].left >= -1.0 && block[2].left < 0.0);
}

#[test]
fn test_master_bus_lookahead() {
    let mut bus = MasterBus::new().with_lookahead(4);
    assert_eq!(bus.lookahead_samples(), 4);

    let input = AudioFrame::new(0.5, 0.5);
    // First 4 samples should be zero due to lookahead delay
    for _ in 0..4 {
        let out = bus.process_frame(input);
        assert_eq!(out, AudioFrame::ZERO);
    }
    // 5th sample produces the delayed first input
    let out = bus.process_frame(input);
    assert!((out.left - 0.5).abs() < 1e-4);
}

#[test]
fn test_recorder_audio_fidelity_24bit() {
    let mut recorder = WavRecorder::new();
    let tmp_path = PathBuf::from("/tmp/test_fidelity_24bit.wav");

    assert!(recorder.start_recording(tmp_path.clone(), 48000).is_ok());
    assert!(recorder.is_recording());

    // Push 100 test frames
    for i in 0..100 {
        let val = (i as f32 / 100.0) * 0.8;
        assert!(recorder.push_frame(AudioFrame::new(val, -val)));
    }
    assert_eq!(recorder.frames_recorded(), 100);
    assert_eq!(recorder.dropped_frames(), 0);

    assert!(recorder.stop_recording().is_ok());
    assert!(!recorder.is_recording());
    assert!(tmp_path.exists());

    // Verify WAV file with hound WavReader
    let mut reader = hound::WavReader::open(&tmp_path).expect("Failed to open WAV reader");
    let spec = reader.spec();
    assert_eq!(spec.channels, 2);
    assert_eq!(spec.sample_rate, 48000);
    assert_eq!(spec.bits_per_sample, 24);
    assert_eq!(spec.sample_format, hound::SampleFormat::Int);

    let samples: Vec<i32> = reader.samples::<i32>().map(|s| s.unwrap()).collect();
    assert_eq!(samples.len(), 200); // 100 frames * 2 channels

    // Verify sample values match 24-bit conversion
    for i in 0..100 {
        let expected_l = sample_to_i24((i as f32 / 100.0) * 0.8);
        let expected_r = sample_to_i24(-((i as f32 / 100.0) * 0.8));
        assert_eq!(samples[i * 2], expected_l);
        assert_eq!(samples[i * 2 + 1], expected_r);
    }

    let _ = std::fs::remove_file(tmp_path);
}

#[test]
fn test_recorder_push_block() {
    let mut recorder = WavRecorder::new();
    let tmp_path = PathBuf::from("/tmp/test_block_rec.wav");

    assert!(recorder.start_recording(tmp_path.clone(), 48000).is_ok());
    let frames = [
        AudioFrame::new(0.1, 0.2),
        AudioFrame::new(0.3, 0.4),
        AudioFrame::new(0.5, 0.6),
    ];
    let pushed = recorder.push_block(&frames);
    assert_eq!(pushed, 3);
    assert_eq!(recorder.frames_recorded(), 3);

    assert!(recorder.stop_recording().is_ok());
    let _ = std::fs::remove_file(tmp_path);
}

#[test]
fn test_recorder_error_states() {
    let mut recorder = WavRecorder::new();
    // Stop without start
    match recorder.stop_recording() {
        Err(RecorderError::NotRecording) => {}
        other => panic!("Expected NotRecording, got {:?}", other),
    }

    let tmp_path = PathBuf::from("/tmp/test_err_rec.wav");
    assert!(recorder.start_recording(tmp_path.clone(), 48000).is_ok());

    // Double start
    match recorder.start_recording(tmp_path.clone(), 48000) {
        Err(RecorderError::AlreadyRecording) => {}
        other => panic!("Expected AlreadyRecording, got {:?}", other),
    }

    assert!(recorder.stop_recording().is_ok());
    let _ = std::fs::remove_file(tmp_path);
}

#[test]
fn test_recorder_drop_safely_finalizes() {
    let tmp_path = PathBuf::from("/tmp/test_drop_rec.wav");
    {
        let mut recorder = WavRecorder::new();
        assert!(recorder.start_recording(tmp_path.clone(), 48000).is_ok());
        recorder.push_frame(AudioFrame::new(0.25, 0.25));
        // recorder drops here without explicit stop_recording
    }

    // Verify WAV file was cleanly finalized by Drop
    assert!(tmp_path.exists());
    let reader = hound::WavReader::open(&tmp_path).expect("Should open finalized WAV");
    assert_eq!(reader.len(), 2); // 1 frame = 2 samples
    let _ = std::fs::remove_file(tmp_path);
}

#[test]
fn test_sample_to_i24_and_back() {
    assert_eq!(sample_to_i24(1.0), 8_388_607);
    assert_eq!(sample_to_i24(-1.0), -8_388_608);
    assert_eq!(sample_to_i24(0.0), 0);
    assert_eq!(sample_to_i24(2.0), 8_388_607);
    assert_eq!(sample_to_i24(-5.0), -8_388_608);
    assert_eq!(sample_to_i24(f32::NAN), 0);
    assert_eq!(sample_to_i24(f32::INFINITY), 0);

    let roundtrip = i24_to_sample(sample_to_i24(0.5));
    assert!((roundtrip - 0.5).abs() < 1e-6);
}
