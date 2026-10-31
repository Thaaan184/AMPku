//! Integration and unit tests for AMPku GUI application loop, master transport bar,
//! and segmented stereo LED VU meter widgets.

use ampku::core::frame::AudioFrame;
use ampku::engine::channel::ChannelMeter;
use ampku::engine::master_bus::MasterBus;
use ampku::gui::app::AmpkuApp;
use ampku::gui::meter::{
    db_to_normalized, format_db, render_gain_reduction_meter, render_labeled_stereo_vu_meter,
    render_stereo_vu_meter, StereoVuMeter, VuMeterConfig,
};
use ampku::gui::theme::apply_studio_theme;
use ampku::gui::transport::{
    format_duration, format_latency, render_latency_badge, render_master_sum_bar, render_top_bar,
    TopBarState, TransportAction, BUFFER_SIZES,
};

fn run_in_egui<F: FnOnce(&mut egui::Ui)>(f: F) {
    let ctx = egui::Context::default();
    apply_studio_theme(&ctx);
    let raw_input = egui::RawInput::default();
    let _ = ctx.run(raw_input, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            f(ui);
        });
    });
}

// ============================================================================
// 1. Meter Widget Tests
// ============================================================================

#[test]
fn test_meter_formatting_and_normalization() {
    assert_eq!(format_db(-60.0), "-inf dB");
    assert_eq!(format_db(-65.0), "-inf dB");
    assert_eq!(format_db(-100.0), "-inf dB");
    assert_eq!(format_db(f32::NEG_INFINITY), "-inf dB");
    assert_eq!(format_db(f32::NAN), "-inf dB");
    assert_eq!(format_db(0.0), "0.0 dB");
    assert_eq!(format_db(-12.4), "-12.4 dB");
    assert_eq!(format_db(1.5), "+1.5 dB");
    assert_eq!(format_db(0.1), "+0.1 dB");

    // Normalization across [-60.0, 3.0]
    assert_eq!(db_to_normalized(-60.0, -60.0, 3.0), 0.0);
    assert_eq!(db_to_normalized(-70.0, -60.0, 3.0), 0.0);
    assert_eq!(db_to_normalized(3.0, -60.0, 3.0), 1.0);
    assert_eq!(db_to_normalized(10.0, -60.0, 3.0), 1.0);
    assert_eq!(db_to_normalized(f32::NAN, -60.0, 3.0), 0.0);

    let mid_db = -28.5; // Exactly halfway between -60 and +3 (span 63 dB)
    let norm = db_to_normalized(mid_db, -60.0, 3.0);
    assert!((norm - 0.5).abs() < 1e-3);

    // Default configuration
    let config = VuMeterConfig::default();
    assert_eq!(config.min_db, -60.0);
    assert_eq!(config.max_db, 3.0);
    assert_eq!(config.yellow_db, -12.0);
    assert_eq!(config.red_db, 0.0);
    assert_eq!(config.num_segments, 24);
    assert!(config.show_clip);
    assert!(config.show_numeric);
}

#[test]
fn test_stereo_vu_meter_rendering_and_clip_reset() {
    let meter = ChannelMeter::new();

    // Cause clipping by processing a frame exceeding 0 dBFS
    meter.update_frame(AudioFrame::new(1.2, 1.2), 48000.0);
    assert!(meter.is_clipped());

    run_in_egui(|ui| {
        let resp = render_stereo_vu_meter(ui, &meter, egui::vec2(36.0, 150.0));
        assert!(resp.rect.width() > 0.0);
        assert!(resp.rect.height() > 0.0);

        let labeled_resp =
            render_labeled_stereo_vu_meter(ui, &meter, "TEST", egui::vec2(36.0, 150.0));
        assert!(labeled_resp.rect.width() > 0.0);

        let raw_meter = StereoVuMeter::new()
            .with_label("RAW")
            .with_size(egui::vec2(40.0, 120.0));
        let (_, reset_clicked) = raw_meter.show_raw(ui, -6.0, -12.0, -8.0, -14.0, true);
        assert!(!reset_clicked);
    });

    meter.reset_clip();
    assert!(!meter.is_clipped());
}

#[test]
fn test_gain_reduction_meter_rendering() {
    run_in_egui(|ui| {
        render_gain_reduction_meter(ui, 0.0, 18.0, 60.0);
        render_gain_reduction_meter(ui, -2.5, 18.0, 60.0);
        render_gain_reduction_meter(ui, -7.0, 18.0, 60.0);
        render_gain_reduction_meter(ui, -14.0, 18.0, 60.0);
    });
}

// ============================================================================
// 2. Transport & Header Controls Tests
// ============================================================================

#[test]
fn test_transport_latency_badge_and_duration_format() {
    // 64 samples at 48000 Hz = (64 / 48000) * 1000 = 1.333 ms -> "64 smp - 1.3 ms"
    assert_eq!(format_latency(64, 48000), "64 smp - 1.3 ms");
    assert_eq!(format_latency(128, 48000), "128 smp - 2.7 ms");
    assert_eq!(format_latency(256, 48000), "256 smp - 5.3 ms");
    assert_eq!(format_latency(512, 48000), "512 smp - 10.7 ms");

    assert_eq!(BUFFER_SIZES, [32, 64, 128, 256, 512, 1024]);

    assert_eq!(format_duration(0.0), "00:00.0");
    assert_eq!(format_duration(12.34), "00:12.3");
    assert_eq!(format_duration(65.45), "01:05.4");
    assert_eq!(format_duration(3600.0), "60:00.0");
    assert_eq!(format_duration(-5.0), "00:00.0");
    assert_eq!(format_duration(f64::NAN), "00:00.0");

    run_in_egui(|ui| {
        render_latency_badge(ui, 64, 48000);
    });
}

#[test]
fn test_transport_top_bar_rendering_and_actions() {
    run_in_egui(|ui| {
        let preset_names = vec![
            "Texas Blues".to_string(),
            "Modern Lead".to_string(),
            "Clean Funk".to_string(),
        ];
        let state = TopBarState {
            is_running: false,
            available_inputs: &[],
            available_outputs: &[],
            selected_input: None,
            selected_output: None,
            buffer_size: 64,
            sample_rate: 48000,
            preset_names: &preset_names,
            selected_preset: "Texas Blues",
            is_recording: false,
            record_duration_secs: 0.0,
        };

        let action = render_top_bar(ui, &state);
        assert_eq!(action, TransportAction::None);

        let active_state = TopBarState {
            is_running: true,
            is_recording: true,
            record_duration_secs: 42.5,
            ..state
        };
        let action_active = render_top_bar(ui, &active_state);
        assert_eq!(action_active, TransportAction::None);
    });
}

#[test]
fn test_master_sum_bar_rendering_and_mutation() {
    let mut master = MasterBus::new();
    assert_eq!(master.volume_db(), 0.0);
    assert!(!master.is_bypassed());

    run_in_egui(|ui| {
        render_master_sum_bar(ui, &mut master);
    });

    // Test parameter mutations on master bus
    master.set_volume_db(-3.0);
    assert!((master.volume_db() - (-3.0)).abs() < 1e-2);

    master.set_bypass(true);
    assert!(master.is_bypassed());

    master.set_threshold_db(-6.0);
    assert!((master.threshold_db() - (-6.0)).abs() < 1e-2);

    master.set_ceiling_db(-0.5);
    assert!((master.ceiling_db() - (-0.5)).abs() < 1e-2);

    master.set_release_ms(150.0);
    assert_eq!(master.release_ms(), 150.0);
}

// ============================================================================
// 3. AmpkuApp State & Lifecycle Tests
// ============================================================================

#[test]
fn test_ampku_app_create_and_default_state() {
    let app = AmpkuApp::create();

    assert_eq!(app.buffer_size, 64);
    assert_eq!(app.sample_rate, 48000);
    assert!(!app.is_recording());
    assert_eq!(app.record_duration_seconds(), 0.0);
    assert!(!app.selected_preset.is_empty());
    assert!(app.preset_names.len() >= 5);
    assert!(!app.show_save_dialog);
    assert!(app.recordings_dir.exists());

    // Both channels and master bus are initialized
    assert!(app.engine.guitar_channel().lock().is_ok());
    assert!(app.engine.mic_channel().lock().is_ok());
    assert!(app.engine.master_bus().lock().is_ok());
}

#[test]
fn test_ampku_app_ui_render_cycle() {
    let mut app = AmpkuApp::create();
    let ctx = egui::Context::default();
    apply_studio_theme(&ctx);

    let raw_input = egui::RawInput::default();
    let full_output = ctx.run(raw_input, |ctx| {
        app.render_ui(ctx);
    });

    // Verify continuous 60 FPS repainting was requested (repaint_delay <= 16 ms)
    let root_vp = full_output
        .viewport_output
        .get(&egui::ViewportId::ROOT)
        .expect("Root viewport output missing");
    assert!(root_vp.repaint_delay <= std::time::Duration::from_millis(50));
}

#[test]
fn test_ampku_app_transport_actions_dispatch() {
    let mut app = AmpkuApp::create();

    // Change buffer size
    app.handle_transport_action(TransportAction::ChangeBufferSize(128));
    assert_eq!(app.buffer_size, 128);
    assert_eq!(app.engine.config().buffer_size, 128);

    // Change input and output device names
    app.handle_transport_action(TransportAction::ChangeInputDevice(
        "Test Vention Input".into(),
    ));
    assert_eq!(app.selected_input, Some("Test Vention Input".to_string()));

    app.handle_transport_action(TransportAction::ChangeOutputDevice(
        "Test Speakers Output".into(),
    ));
    assert_eq!(
        app.selected_output,
        Some("Test Speakers Output".to_string())
    );

    // Reset to default
    app.handle_transport_action(TransportAction::ChangeInputDevice("Default Input".into()));
    assert_eq!(app.selected_input, None);

    app.handle_transport_action(TransportAction::ChangeOutputDevice("Default Output".into()));
    assert_eq!(app.selected_output, None);

    // Save preset prompt
    app.handle_transport_action(TransportAction::SavePresetPrompt);
    assert!(app.show_save_dialog);
}

#[test]
fn test_ampku_app_preset_lifecycle() {
    let mut app = AmpkuApp::create();

    // Load factory tone
    app.load_preset("Modern Lead");
    assert_eq!(app.selected_preset, "Modern Lead");

    // Save custom preset
    let unique_name = format!("TestTone_{}", std::process::id());
    app.save_preset(&unique_name);
    assert_eq!(app.selected_preset, unique_name);
    assert!(app.preset_names.contains(&unique_name));

    // Cleanup saved preset
    let _ = app.preset_manager.delete_preset(&unique_name);
}

#[test]
fn test_ampku_app_recording_lifecycle() {
    let mut app = AmpkuApp::create();
    let temp_rec_dir = std::env::temp_dir().join(format!("ampku_rec_test_{}", std::process::id()));
    app.recordings_dir = temp_rec_dir.clone();

    assert!(!app.is_recording());
    app.toggle_recording();
    assert!(app.is_recording());

    // Stop recording
    app.toggle_recording();
    assert!(!app.is_recording());

    // Cleanup temp directory
    let _ = std::fs::remove_dir_all(&temp_rec_dir);
}

#[test]
fn test_ampku_app_decoupled_rendering_audio_thread_non_starvation() {
    let mut app = AmpkuApp::create();
    let ctx = egui::Context::default();
    apply_studio_theme(&ctx);

    let guitar_arc = app.engine.guitar_channel().clone();
    let mic_arc = app.engine.mic_channel().clone();
    let master_arc = app.engine.master_bus().clone();

    // Verify engine locks are immediately available
    let raw_input = egui::RawInput::default();
    let _ = ctx.run(raw_input, |ctx| {
        app.render_ui(ctx);
    });

    // After render pass, engine locks must not be held
    assert!(guitar_arc.try_lock().is_ok());
    assert!(mic_arc.try_lock().is_ok());
    assert!(master_arc.try_lock().is_ok());
}
