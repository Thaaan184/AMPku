//! Transport controls, top header bar, and master sum limiter bar for AMPku Studio.
//!
//! Provides:
//! - Hardware I/O device selection dropdowns (WASAPI / ALSA duplex streams)
//! - Buffer size selector (32 to 1024 frames) and real-time latency badge (`64 smp - 1.3 ms`)
//! - Preset loader and saver control bar
//! - Record transport bar with red `● REC` button, duration timer, and recordings folder shortcut
//! - Master sum bus mixer with brickwall limiter controls, gain reduction meter, and master VU meter

use egui::{vec2, Color32, Frame, Margin, RichText, Rounding, Slider, Stroke, Ui};

use crate::audio::device::AudioDeviceInfo;
use crate::engine::master_bus::MasterBus;
use crate::gui::meter::{render_gain_reduction_meter, render_labeled_stereo_vu_meter};
use crate::gui::theme::{
    well_frame, COLOR_ACCENT_BLUE, COLOR_ALERT_RED, COLOR_ALERT_RED_BG, COLOR_BORDER,
    COLOR_BORDER_ACTIVE, COLOR_SOLO_AMBER, COLOR_TEXT_DIM, COLOR_TEXT_MUTED, COLOR_TEXT_PRIMARY,
};

/// Available audio buffer sizes in frames.
pub const BUFFER_SIZES: [u32; 6] = [32, 64, 128, 256, 512, 1024];

/// User actions triggered from transport and top header controls.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportAction {
    /// No action triggered.
    None,
    /// Request switching the active audio hardware input device.
    ChangeInputDevice(String),
    /// Request switching the active audio hardware output device.
    ChangeOutputDevice(String),
    /// Request switching the audio streaming buffer size in frames.
    ChangeBufferSize(u32),
    /// Request starting or stopping the duplex audio engine streams.
    ToggleEngine,
    /// Request loading and applying a preset by name.
    SelectPreset(String),
    /// Request opening the preset save dialog.
    SavePresetPrompt,
    /// Request saving the preset with the given name.
    SavePresetConfirm(String),
    /// Request starting or stopping WAV recording.
    ToggleRecord,
    /// Request opening the system file manager to the recordings folder.
    OpenRecordingsFolder,
}

/// Snapshot of transport state needed to render the top header bar.
#[derive(Debug, Clone)]
pub struct TopBarState<'a> {
    /// True if the CPAL duplex audio streams are actively running.
    pub is_running: bool,
    /// List of enumerated hardware input devices.
    pub available_inputs: &'a [AudioDeviceInfo],
    /// List of enumerated hardware output devices.
    pub available_outputs: &'a [AudioDeviceInfo],
    /// Name of the currently selected or active input device.
    pub selected_input: Option<&'a str>,
    /// Name of the currently selected or active output device.
    pub selected_output: Option<&'a str>,
    /// Currently configured buffer size in frames (e.g. 64).
    pub buffer_size: u32,
    /// Currently configured audio sample rate in Hz (e.g. 48000).
    pub sample_rate: u32,
    /// List of available factory and user preset names.
    pub preset_names: &'a [String],
    /// Name of the currently active preset.
    pub selected_preset: &'a str,
    /// True if the background WAV recorder is actively capturing audio.
    pub is_recording: bool,
    /// Elapsed duration of current recording session in seconds.
    pub record_duration_secs: f64,
}

/// Calculate and format buffer size and latency badge text (e.g. `64 smp - 1.3 ms`).
pub fn format_latency(buffer_size: u32, sample_rate: u32) -> String {
    let sr = if sample_rate > 0 {
        sample_rate as f32
    } else {
        48000.0
    };
    let latency_ms = (buffer_size as f32 / sr) * 1000.0;
    format!("{} smp - {:.1} ms", buffer_size, latency_ms)
}

/// Format seconds into `MM:SS.s` recording duration timer string.
pub fn format_duration(seconds: f64) -> String {
    if !seconds.is_finite() || seconds < 0.0 {
        return "00:00.0".to_string();
    }
    let total_secs = seconds.floor() as u64;
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    let tenths = ((seconds - total_secs as f64) * 10.0).floor() as u64;
    format!("{:02}:{:02}.{}", mins, secs, tenths)
}

/// Render the real-time buffer size and latency badge.
pub fn render_latency_badge(ui: &mut Ui, buffer_size: u32, sample_rate: u32) {
    let text = format_latency(buffer_size, sample_rate);
    Frame::none()
        .fill(Color32::from_rgb(0x12, 0x1a, 0x26))
        .stroke(Stroke::new(1.0_f32, COLOR_BORDER_ACTIVE))
        .rounding(Rounding::same(4.0))
        .inner_margin(Margin::symmetric(8.0, 4.0))
        .show(ui, |ui| {
            ui.label(
                RichText::new(text)
                    .monospace()
                    .strong()
                    .size(11.0)
                    .color(COLOR_ACCENT_BLUE),
            );
        });
}

/// Render the top transport and configuration header bar.
///
/// Returns any `TransportAction` triggered by user interactions.
pub fn render_top_bar(ui: &mut Ui, state: &TopBarState<'_>) -> TransportAction {
    let mut action = TransportAction::None;

    well_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            // App Branding & Title
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("AMPku")
                        .heading()
                        .strong()
                        .color(COLOR_ACCENT_BLUE),
                );
                ui.label(
                    RichText::new("STUDIO")
                        .strong()
                        .size(11.0)
                        .color(COLOR_TEXT_DIM),
                );
            });

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            // Engine Power Toggle Button
            let engine_btn = if state.is_running {
                egui::Button::new(
                    RichText::new("⏹ STOP")
                        .strong()
                        .size(11.0)
                        .color(Color32::WHITE),
                )
                .fill(COLOR_ALERT_RED_BG)
                .stroke(Stroke::new(1.0_f32, COLOR_ALERT_RED))
            } else {
                egui::Button::new(
                    RichText::new("▶ START")
                        .strong()
                        .size(11.0)
                        .color(Color32::BLACK),
                )
                .fill(COLOR_ACCENT_BLUE)
            };

            if ui.add(engine_btn).clicked() {
                action = TransportAction::ToggleEngine;
            }

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            // Hardware Input Device Dropdown
            ui.label(RichText::new("In:").size(11.0).color(COLOR_TEXT_MUTED));
            let current_in = state.selected_input.unwrap_or("Default Input");
            egui::ComboBox::from_id_source("transport_input_device_combo")
                .selected_text(RichText::new(current_in).size(11.0))
                .width(130.0)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(state.selected_input.is_none(), "Default Input")
                        .clicked()
                    {
                        action = TransportAction::ChangeInputDevice("Default Input".to_string());
                    }
                    for dev in state.available_inputs {
                        let is_selected = state.selected_input == Some(dev.name.as_str());
                        if ui.selectable_label(is_selected, &dev.name).clicked() {
                            action = TransportAction::ChangeInputDevice(dev.name.clone());
                        }
                    }
                });

            // Hardware Output Device Dropdown
            ui.label(RichText::new("Out:").size(11.0).color(COLOR_TEXT_MUTED));
            let current_out = state.selected_output.unwrap_or("Default Output");
            egui::ComboBox::from_id_source("transport_output_device_combo")
                .selected_text(RichText::new(current_out).size(11.0))
                .width(130.0)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(state.selected_output.is_none(), "Default Output")
                        .clicked()
                    {
                        action = TransportAction::ChangeOutputDevice("Default Output".to_string());
                    }
                    for dev in state.available_outputs {
                        let is_selected = state.selected_output == Some(dev.name.as_str());
                        if ui.selectable_label(is_selected, &dev.name).clicked() {
                            action = TransportAction::ChangeOutputDevice(dev.name.clone());
                        }
                    }
                });

            // Buffer Size Dropdown & Latency Badge
            ui.label(RichText::new("Buf:").size(11.0).color(COLOR_TEXT_MUTED));
            egui::ComboBox::from_id_source("transport_buffer_size_combo")
                .selected_text(RichText::new(format!("{}", state.buffer_size)).size(11.0))
                .width(60.0)
                .show_ui(ui, |ui| {
                    for &size in &BUFFER_SIZES {
                        let is_selected = state.buffer_size == size;
                        if ui
                            .selectable_label(is_selected, format!("{size}"))
                            .clicked()
                        {
                            action = TransportAction::ChangeBufferSize(size);
                        }
                    }
                });

            render_latency_badge(ui, state.buffer_size, state.sample_rate);

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            // Preset Manager Bar
            ui.label(RichText::new("Preset:").size(11.0).color(COLOR_TEXT_MUTED));
            egui::ComboBox::from_id_source("transport_preset_combo")
                .selected_text(RichText::new(state.selected_preset).strong().size(11.0))
                .width(120.0)
                .show_ui(ui, |ui| {
                    for name in state.preset_names {
                        let is_selected = state.selected_preset == name;
                        if ui.selectable_label(is_selected, name).clicked() {
                            action = TransportAction::SelectPreset(name.clone());
                        }
                    }
                });

            if ui
                .add(egui::Button::new(RichText::new("💾 Save").size(11.0)))
                .clicked()
            {
                action = TransportAction::SavePresetPrompt;
            }

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            // Record Transport Controls (Red ● REC, timer, folder shortcut)
            let rec_btn = if state.is_recording {
                egui::Button::new(
                    RichText::new("■ STOP REC")
                        .strong()
                        .size(11.0)
                        .color(Color32::WHITE),
                )
                .fill(COLOR_ALERT_RED)
            } else {
                egui::Button::new(
                    RichText::new("● REC")
                        .strong()
                        .size(11.0)
                        .color(COLOR_ALERT_RED),
                )
                .fill(Color32::from_rgb(0x28, 0x16, 0x16))
                .stroke(Stroke::new(1.0_f32, COLOR_ALERT_RED))
            };

            if ui.add(rec_btn).clicked() {
                action = TransportAction::ToggleRecord;
            }

            // Duration Timer Badge
            Frame::none()
                .fill(if state.is_recording {
                    Color32::from_rgb(0x2d, 0x12, 0x12)
                } else {
                    Color32::from_rgb(0x18, 0x18, 0x1c)
                })
                .stroke(Stroke::new(
                    1.0_f32,
                    if state.is_recording {
                        COLOR_ALERT_RED
                    } else {
                        COLOR_BORDER
                    },
                ))
                .rounding(Rounding::same(4.0))
                .inner_margin(Margin::symmetric(8.0, 4.0))
                .show(ui, |ui| {
                    let dur_text = format_duration(state.record_duration_secs);
                    ui.label(
                        RichText::new(dur_text)
                            .monospace()
                            .strong()
                            .size(11.5)
                            .color(if state.is_recording {
                                Color32::WHITE
                            } else {
                                COLOR_TEXT_DIM
                            }),
                    );
                });

            // Open Folder Shortcut
            if ui
                .add(egui::Button::new(RichText::new("📁 Folder").size(11.0)).frame(true))
                .on_hover_text("Open recordings folder")
                .clicked()
            {
                action = TransportAction::OpenRecordingsFolder;
            }
        });
    });

    action
}

/// Render the master sum bus bar with brickwall limiter controls, gain reduction meter,
/// and segmented master VU meter.
pub fn render_master_sum_bar(ui: &mut Ui, master: &mut MasterBus) {
    well_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            // Master Section Header
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("MASTER SUM & LIMITER")
                        .strong()
                        .size(12.0)
                        .color(COLOR_TEXT_PRIMARY),
                );
                ui.label(
                    RichText::new("Brickwall Peak Protection")
                        .size(10.0)
                        .color(COLOR_TEXT_DIM),
                );
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            // Master Volume Fader (-60 dB to +6 dB)
            ui.vertical(|ui| {
                let current_vol_db = master.volume_db();
                let mut vol_db = if current_vol_db <= -59.5 {
                    -60.0
                } else {
                    current_vol_db.clamp(-60.0, 6.0)
                };

                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Master Volume:")
                            .size(11.0)
                            .color(COLOR_TEXT_MUTED),
                    );
                    if ui
                        .add(egui::Button::new(RichText::new("0 dB").size(9.5)))
                        .clicked()
                    {
                        master.set_volume_db(0.0);
                    }
                });

                let slider = Slider::new(&mut vol_db, -60.0..=6.0)
                    .step_by(0.1)
                    .custom_formatter(|v, _| {
                        if v <= -59.5 {
                            "-inf".to_string()
                        } else {
                            format!("{:+.1} dB", v)
                        }
                    });

                if ui.add(slider).changed() {
                    if vol_db <= -59.5 {
                        master.set_gain(0.0);
                    } else {
                        master.set_volume_db(vol_db);
                    }
                }
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            // Brickwall Limiter Controls (Bypass, Threshold, Ceiling, Release)
            let is_bypassed = master.is_bypassed();
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Limiter:")
                            .strong()
                            .size(11.0)
                            .color(COLOR_TEXT_MUTED),
                    );

                    let bypass_btn = egui::Button::new(
                        RichText::new(if is_bypassed { "BYPASS" } else { "ACTIVE" })
                            .strong()
                            .size(10.0)
                            .color(if is_bypassed {
                                Color32::WHITE
                            } else {
                                Color32::BLACK
                            }),
                    )
                    .fill(if is_bypassed {
                        COLOR_ALERT_RED_BG
                    } else {
                        COLOR_SOLO_AMBER
                    });

                    if ui.add(bypass_btn).clicked() {
                        master.set_bypass(!is_bypassed);
                    }
                });

                ui.horizontal(|ui| {
                    // Threshold Slider (-12.0 dB to 0.0 dB)
                    let mut thresh_db = master.threshold_db().clamp(-12.0, 0.0);
                    ui.label(RichText::new("Thresh:").size(10.0).color(COLOR_TEXT_MUTED));
                    let thresh_slider = Slider::new(&mut thresh_db, -12.0..=0.0)
                        .step_by(0.1)
                        .custom_formatter(|v, _| format!("{:.1} dB", v));
                    if ui.add(thresh_slider).changed() {
                        master.set_threshold_db(thresh_db);
                    }

                    // Ceiling Slider (-1.0 dB to 0.0 dB)
                    let mut ceiling_db = master.ceiling_db().clamp(-1.0, 0.0);
                    ui.label(RichText::new("Ceil:").size(10.0).color(COLOR_TEXT_MUTED));
                    let ceil_slider = Slider::new(&mut ceiling_db, -1.0..=0.0)
                        .step_by(0.05)
                        .custom_formatter(|v, _| format!("{:.2} dB", v));
                    if ui.add(ceil_slider).changed() {
                        master.set_ceiling_db(ceiling_db);
                    }

                    // Release Slider (10 ms to 500 ms)
                    let mut release_ms = master.release_ms().clamp(10.0, 500.0);
                    ui.label(RichText::new("Rel:").size(10.0).color(COLOR_TEXT_MUTED));
                    let rel_slider = Slider::new(&mut release_ms, 10.0..=500.0)
                        .step_by(1.0)
                        .custom_formatter(|v, _| format!("{:.0} ms", v));
                    if ui.add(rel_slider).changed() {
                        master.set_release_ms(release_ms);
                    }
                });
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            // Limiter Gain Reduction (GR) Meter
            let gr_db = master.gain_reduction_db();
            render_gain_reduction_meter(ui, gr_db, 18.0, 60.0);

            ui.add_space(4.0);

            // Master Stereo VU Meter
            render_labeled_stereo_vu_meter(ui, master.meter(), "MASTER", vec2(36.0, 80.0));
        });
    });
}
