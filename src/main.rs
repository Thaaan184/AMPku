//! AMPku - Standalone Low-Latency Guitar & Vocal DSP Studio application entrypoint.

use ampku::gui::app::AmpkuApp;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 750.0])
            .with_min_inner_size([900.0, 600.0])
            .with_title("AMPku - Low-Latency Guitar & Vocal DSP Studio"),
        default_theme: eframe::Theme::Dark,
        ..Default::default()
    };

    eframe::run_native(
        "AMPku - Low-Latency Guitar & Vocal DSP Studio",
        native_options,
        Box::new(|cc| Box::new(AmpkuApp::new(cc))),
    )
}
