use std::sync::{Arc, Mutex};
use std::thread;

use ampku::audio::clock_drift::{hermite_interp, ClockDriftBuffer};
use ampku::audio::device::{
    calculate_latency_ms, latency_badge, validate_buffer_size, validate_sample_rate,
    AudioDeviceInfo, AudioStreamConfig, DEFAULT_BUFFER_SIZE, DEFAULT_SAMPLE_RATE,
    SUPPORTED_BUFFER_SIZES,
};
use ampku::audio::engine::AudioEngine;
use ampku::audio::recorder::WavRecorder;
use ampku::core::frame::AudioFrame;

#[test]
fn test_clock_drift_buffer_resilience() {
    let mut drift_buf = ClockDriftBuffer::new(256);
    // Push 64 frames from input clock
    for i in 0..64 {
        drift_buf.write(AudioFrame::new(i as f32, i as f32));
    }
    // Read 64 frames from output clock
    let read = drift_buf.read_block(64);
    assert_eq!(read.len(), 64);
}

#[test]
fn test_clock_drift_interpolation_exactness() {
    let mut drift_buf = ClockDriftBuffer::new(256);
    drift_buf.set_target_fill(64.0);

    for i in 0..64 {
        drift_buf.write(AudioFrame::new(i as f32 * 0.1, i as f32 * 0.2));
    }

    let mut out = [AudioFrame::ZERO; 64];
    let read_count = drift_buf.read(&mut out);
    assert_eq!(read_count, 64);

    // Frame 0 should match exactly
    assert!((out[0].left - 0.0).abs() < 1e-5);
    assert!((out[0].right - 0.0).abs() < 1e-5);

    // Later frames should track the linear ramp cleanly
    for (i, frame) in out.iter().enumerate() {
        assert!(
            (frame.left - (i as f32 * 0.1)).abs() < 1e-3,
            "Frame {i} left mismatch: expected {}, got {}",
            i as f32 * 0.1,
            frame.left
        );
    }
}

#[test]
fn test_clock_drift_hermite_interpolation_math() {
    // Hermite on a linear ramp should reproduce exact linear values
    let y_m1 = 0.0;
    let y0 = 1.0;
    let y1 = 2.0;
    let y2 = 3.0;

    let mid = hermite_interp(y_m1, y0, y1, y2, 0.5);
    assert!((mid - 1.5).abs() < 1e-5);

    let start = hermite_interp(y_m1, y0, y1, y2, 0.0);
    assert!((start - 1.0).abs() < 1e-5);

    let end = hermite_interp(y_m1, y0, y1, y2, 1.0);
    assert!((end - 2.0).abs() < 1e-5);
}

#[test]
fn test_clock_drift_pi_controller_fast_clock() {
    // Input clock produces 68 frames while output reads 64 frames (faster producer)
    let mut drift_buf = ClockDriftBuffer::new(512);
    drift_buf.set_target_fill(128.0);
    drift_buf.set_pi_constants(0.002, 0.0005);

    // Prime to target fill
    for i in 0..128 {
        drift_buf.write(AudioFrame::new(i as f32, i as f32));
    }

    // Run 30 callback cycles with fast producer
    for _ in 0..30 {
        for _ in 0..68 {
            drift_buf.write(AudioFrame::new(0.5, -0.5));
        }
        let _ = drift_buf.read_block(64);
    }

    // Because input was faster, PI controller should adapt drift_ratio > 1.0
    assert!(
        drift_buf.drift_ratio() > 1.0,
        "Expected drift ratio > 1.0, got {}",
        drift_buf.drift_ratio()
    );
    assert!(drift_buf.integral() > 0.0);
}

#[test]
fn test_clock_drift_pi_controller_slow_clock() {
    // Input clock produces 60 frames while output reads 64 frames (slower producer)
    let mut drift_buf = ClockDriftBuffer::new(512);
    drift_buf.set_target_fill(128.0);
    drift_buf.set_pi_constants(0.002, 0.0005);

    // Prime to target fill
    for i in 0..128 {
        drift_buf.write(AudioFrame::new(i as f32, i as f32));
    }

    // Run 30 callback cycles with slow producer
    for _ in 0..30 {
        for _ in 0..60 {
            drift_buf.write(AudioFrame::new(0.5, -0.5));
        }
        let _ = drift_buf.read_block(64);
    }

    // Because input was slower, PI controller should adapt drift_ratio < 1.0
    assert!(
        drift_buf.drift_ratio() < 1.0,
        "Expected drift ratio < 1.0, got {}",
        drift_buf.drift_ratio()
    );
    assert!(drift_buf.integral() < 0.0);
}

#[test]
fn test_clock_drift_underflow_overflow_handling() {
    let mut drift_buf = ClockDriftBuffer::new(64);

    // Underflow test: read 32 from empty buffer
    let read = drift_buf.read_block(32);
    assert_eq!(read.len(), 32);
    for frame in read {
        assert_eq!(frame, AudioFrame::ZERO);
    }
    assert!(drift_buf.underflow_count() >= 32);

    // Overflow test: write 100 frames into capacity 64
    for i in 0..100 {
        drift_buf.write(AudioFrame::new(i as f32, i as f32));
    }
    assert!(drift_buf.overflow_count() > 0);
    assert!(drift_buf.len() <= 64);

    // Reset clears state and counters
    drift_buf.reset();
    assert_eq!(drift_buf.len(), 0);
    assert_eq!(drift_buf.underflow_count(), 0);
    assert_eq!(drift_buf.overflow_count(), 0);
}

#[test]
fn test_clock_drift_shared_thread_safety() {
    let shared = ClockDriftBuffer::shared(256);
    let writer_shared = shared.clone();
    let reader_shared = shared;

    let writer = thread::spawn(move || {
        for _ in 0..10 {
            if let Ok(mut buf) = writer_shared.lock() {
                for i in 0..32 {
                    buf.write(AudioFrame::new(i as f32, i as f32));
                }
            }
            thread::yield_now();
        }
    });

    let reader = thread::spawn(move || {
        let mut total_read = 0;
        for _ in 0..10 {
            if let Ok(mut buf) = reader_shared.lock() {
                let frames = buf.read_block(32);
                total_read += frames.len();
            }
            thread::yield_now();
        }
        total_read
    });

    writer.join().unwrap();
    let total = reader.join().unwrap();
    assert_eq!(total, 320);
}

#[test]
fn test_audio_device_config_and_latency_validation() {
    // Default config: 48 kHz, 64 buffer frames
    let cfg = AudioStreamConfig::DEFAULT;
    assert_eq!(cfg.sample_rate, DEFAULT_SAMPLE_RATE);
    assert_eq!(cfg.buffer_size, DEFAULT_BUFFER_SIZE);
    assert!(cfg.validate().is_ok());

    // Latency calculation: 64 samples @ 48 kHz = 1.333 ms
    let latency = calculate_latency_ms(64, 48000);
    assert!((latency - 1.3333).abs() < 0.01);
    assert_eq!(latency_badge(64, 48000), "64 smp - 1.3 ms");

    // 128 samples @ 48 kHz = 2.666 ms
    let latency_128 = calculate_latency_ms(128, 48000);
    assert!((latency_128 - 2.6666).abs() < 0.01);
    assert_eq!(latency_badge(128, 48000), "128 smp - 2.7 ms");

    // Validations
    for &size in SUPPORTED_BUFFER_SIZES {
        assert!(validate_buffer_size(size).is_ok());
    }
    assert!(validate_buffer_size(32).is_err());
    assert!(validate_buffer_size(2048).is_err());

    assert!(validate_sample_rate(48000).is_ok());
    assert!(validate_sample_rate(44100).is_ok());
    assert!(validate_sample_rate(22050).is_err());
}

#[test]
fn test_audio_device_info_serde() {
    let info = AudioDeviceInfo {
        name: "Vention USB Audio".to_string(),
        is_default: true,
        is_input: true,
        max_channels: 2,
        default_sample_rate: 48000,
        supported_sample_rates: vec![44100, 48000],
        min_buffer_size: Some(64),
        max_buffer_size: Some(1024),
    };

    let json = serde_json::to_string(&info).unwrap();
    let deserialized: AudioDeviceInfo = serde_json::from_str(&json).unwrap();
    assert_eq!(info, deserialized);
}

#[test]
fn test_audio_engine_offline_dsp_processing() {
    let engine = AudioEngine::new().expect("Failed to initialize AudioEngine");
    assert!(!engine.is_running());

    // Configure channel volumes: Guitar = 0.5 (-6 dB), Mic = 0.5 (-6 dB)
    if let Ok(g) = engine.guitar_channel().lock() {
        g.set_gain(0.5);
    }
    if let Ok(m) = engine.mic_channel().lock() {
        m.set_gain(0.5);
    }

    // Input signal: Left (Guitar) = 1.0, Right (Mic) = 1.0
    let input = vec![AudioFrame::new(1.0, 1.0); 64];
    let mut output = vec![AudioFrame::ZERO; 64];

    engine.process_block(&input, &mut output);

    // Summing: Guitar (mono 1.0 * 0.5 = 0.5 left/right) + Mic (mono 1.0 * 0.5 = 0.5 left/right)
    // Sum = 1.0 on both channels (or slight limiter action)
    assert!(output[0].left > 0.0);
    assert!(output[0].right > 0.0);
    assert!(output[0].left <= 1.0);
    assert!(output[0].right <= 1.0);

    // Verify VU meters updated
    let (g_pk_l, _) = engine.guitar_peak_db();
    let (m_pk_l, _) = engine.mic_peak_db();
    let (mst_pk_l, _) = engine.master_peak_db();

    assert!(g_pk_l > -60.0);
    assert!(m_pk_l > -60.0);
    assert!(mst_pk_l > -60.0);
}

#[test]
fn test_audio_engine_with_recorder() {
    let engine = AudioEngine::new().expect("AudioEngine creation failed");
    let test_dir = std::env::temp_dir().join("ampku_engine_test");
    let _ = std::fs::create_dir_all(&test_dir);
    let wav_path = test_dir.join("engine_record_test.wav");

    let recorder = Arc::new(Mutex::new(WavRecorder::new()));
    recorder
        .lock()
        .unwrap()
        .start_recording(&wav_path, 48000)
        .expect("Failed to start recording");

    engine.attach_recorder(recorder.clone());

    // Process blocks through engine
    let input = vec![AudioFrame::new(0.3, -0.3); 128];
    let mut output = vec![AudioFrame::ZERO; 128];
    engine.process_block(&input, &mut output);

    // Stop and finalize recording
    let _ = engine.detach_recorder();
    recorder
        .lock()
        .unwrap()
        .stop_recording()
        .expect("Failed to stop recording");

    assert!(wav_path.exists());
    let metadata = std::fs::metadata(&wav_path).unwrap();
    assert!(metadata.len() > 44);

    let _ = std::fs::remove_file(wav_path);
}

#[test]
fn test_audio_engine_device_queries_safe() {
    let engine = AudioEngine::new().expect("Failed to initialize AudioEngine");
    // Queries must never panic regardless of available hardware
    let inputs = engine.input_devices();
    let outputs = engine.output_devices();
    println!("Found {} inputs, {} outputs", inputs.len(), outputs.len());
}
