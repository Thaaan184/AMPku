//! Channel strip UI component for AMPku studio mixer.
//!
//! Renders dual columns (Guitar Ch 1 and Mic Ch 2) with:
//! - Gain fader (-inf to +6 dB)
//! - Pan slider (-1.0 to +1.0)
//! - Mute and Solo toggle buttons with visual states
//! - Mono / Stereo downmix mode switch
//! - Live stereo peak and RMS level VU meter with clipping indicator
//! - Modular dynamic effect rack with reorder and delete actions
//! - `[+ Add Effect]` menu listing all 10 available DSP modules

use egui::{vec2, Color32, RichText, Slider, Ui};

use crate::dsp::cab_sim::{CabinetProfile, CabinetSim};
use crate::dsp::dynamics::{Compressor, NoiseGate};
use crate::dsp::filters::biquad::{BiquadFilter, FilterType};
use crate::dsp::saturation::{Distortion, Fuzz, Overdrive};
use crate::dsp::time::{StereoDelay, StereoReverb};
use crate::dsp::traits::AudioEffect;
use crate::dsp::vocal::VocalDeEsser;
use crate::engine::channel::{ChannelMeter, ChannelStrip};
use crate::gui::effect_card::{render_effect_card_indexed, EffectCardAction};
use crate::gui::theme::{
    channel_strip_frame, well_frame, COLOR_ACCENT_BLUE, COLOR_ALERT_RED, COLOR_SOLO_AMBER,
    COLOR_TEXT_DIM, COLOR_TEXT_MUTED, COLOR_TEXT_PRIMARY,
};

/// Available DSP modules for the `[+ Add Effect]` menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AvailableEffect {
    NoiseGate,
    Compressor,
    Overdrive,
    Distortion,
    Fuzz,
    BiquadFilter,
    CabinetSim,
    StereoDelay,
    StereoReverb,
    VocalDeEsser,
}

impl AvailableEffect {
    /// All 10 available effect variants in standard order.
    pub const ALL: [AvailableEffect; 10] = [
        AvailableEffect::NoiseGate,
        AvailableEffect::Compressor,
        AvailableEffect::Overdrive,
        AvailableEffect::Distortion,
        AvailableEffect::Fuzz,
        AvailableEffect::BiquadFilter,
        AvailableEffect::CabinetSim,
        AvailableEffect::StereoDelay,
        AvailableEffect::StereoReverb,
        AvailableEffect::VocalDeEsser,
    ];

    /// Display title for the effect menu.
    pub fn name(&self) -> &'static str {
        match self {
            AvailableEffect::NoiseGate => "Noise Gate",
            AvailableEffect::Compressor => "Compressor",
            AvailableEffect::Overdrive => "Overdrive",
            AvailableEffect::Distortion => "Distortion",
            AvailableEffect::Fuzz => "Fuzz",
            AvailableEffect::BiquadFilter => "Biquad Filter",
            AvailableEffect::CabinetSim => "Cabinet Sim (IR)",
            AvailableEffect::StereoDelay => "Stereo Delay",
            AvailableEffect::StereoReverb => "Stereo Reverb",
            AvailableEffect::VocalDeEsser => "Vocal De-Esser",
        }
    }

    /// Instantiate a real-time boxed `AudioEffect` at the specified sample rate.
    pub fn instantiate(&self, sample_rate: f32) -> Box<dyn AudioEffect> {
        match self {
            AvailableEffect::NoiseGate => Box::new(NoiseGate::new(sample_rate, -40.0, 5.0, 50.0)),
            AvailableEffect::Compressor => {
                Box::new(Compressor::new(sample_rate, -20.0, 4.0, 10.0, 100.0, 0.0))
            }
            AvailableEffect::Overdrive => {
                Box::new(Overdrive::with_sample_rate(4.0, 0.5, 0.7, sample_rate))
            }
            AvailableEffect::Distortion => {
                Box::new(Distortion::with_sample_rate(8.0, 0.5, 0.7, sample_rate))
            }
            AvailableEffect::Fuzz => Box::new(Fuzz::with_sample_rate(12.0, 0.5, 0.6, sample_rate)),
            AvailableEffect::BiquadFilter => Box::new(BiquadFilter::new(
                FilterType::Peaking,
                sample_rate,
                1000.0,
                std::f32::consts::FRAC_1_SQRT_2,
                0.0,
            )),
            AvailableEffect::CabinetSim => Box::new(CabinetSim::with_sample_rate(
                CabinetProfile::CelestionV30,
                sample_rate,
            )),
            AvailableEffect::StereoDelay => Box::new(StereoDelay::with_options(
                sample_rate,
                350.0,
                0.3,
                0.25,
                0.2,
                false,
                0.5,
            )),
            AvailableEffect::StereoReverb => {
                Box::new(StereoReverb::new(sample_rate, 0.5, 0.5, 1.0, 0.3))
            }
            AvailableEffect::VocalDeEsser => {
                Box::new(VocalDeEsser::new(sample_rate, 6500.0, -20.0, 4.0))
            }
        }
    }
}

/// Render side-by-side dual columns for Guitar (Ch 1) and Mic (Ch 2).
pub fn render_dual_channel_strips(ui: &mut Ui, guitar: &mut ChannelStrip, mic: &mut ChannelStrip) {
    ui.columns(2, |columns| {
        render_channel_strip(&mut columns[0], guitar, "Guitar (Ch 1)");
        render_channel_strip(&mut columns[1], mic, "Mic (Ch 2)");
    });
}

/// Render an individual channel strip column.
pub fn render_channel_strip(ui: &mut Ui, channel: &mut ChannelStrip, title: &str) {
    channel_strip_frame().show(ui, |ui| {
        ui.vertical(|ui| {
            // Header: Channel Title and Mode Switch
            render_channel_header(ui, channel, title);

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            // Channel Controls (Mute, Solo, Pan, Volume, VU Meter)
            render_fader_section(ui, channel);

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);

            // Effect Rack Header & [+ Add Effect] Button
            render_rack_header(ui, channel);

            ui.add_space(4.0);

            // Modular Effect Chain Cards
            render_effect_rack(ui, channel);
        });
    });
}

/// Render the channel title and mono/stereo toggle switch.
fn render_channel_header(ui: &mut Ui, channel: &mut ChannelStrip, title: &str) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(title)
                .heading()
                .strong()
                .color(COLOR_ACCENT_BLUE),
        );

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let is_mono = channel.is_mono();
            let mode_label = if is_mono { "MONO" } else { "STEREO" };
            let mode_color = if is_mono {
                COLOR_ACCENT_BLUE
            } else {
                COLOR_TEXT_MUTED
            };

            let btn = ui.add(
                egui::Button::new(RichText::new(mode_label).size(10.5).color(mode_color))
                    .frame(true),
            );
            if btn.clicked() {
                channel.set_mono(!is_mono);
            }
        });
    });
}

/// Render the mixer controls section: Mute, Solo, Pan, Fader, and VU Meter.
fn render_fader_section(ui: &mut Ui, channel: &mut ChannelStrip) {
    well_frame().show(ui, |ui| {
        // Buttons Row: Mute and Solo
        ui.horizontal(|ui| {
            let muted = channel.is_muted();
            let mute_btn =
                egui::Button::new(RichText::new("MUTE").strong().size(11.0).color(if muted {
                    Color32::WHITE
                } else {
                    COLOR_TEXT_DIM
                }))
                .fill(if muted {
                    COLOR_ALERT_RED
                } else {
                    Color32::from_rgb(0x28, 0x28, 0x2c)
                });

            if ui.add(mute_btn).clicked() {
                channel.toggle_mute();
            }

            let solo = channel.is_solo();
            let solo_btn =
                egui::Button::new(RichText::new("SOLO").strong().size(11.0).color(if solo {
                    Color32::BLACK
                } else {
                    COLOR_TEXT_DIM
                }))
                .fill(if solo {
                    COLOR_SOLO_AMBER
                } else {
                    Color32::from_rgb(0x28, 0x28, 0x2c)
                });

            if ui.add(solo_btn).clicked() {
                channel.toggle_solo();
            }

            // Pan Slider
            let mut pan = channel.pan();
            ui.label(RichText::new("Pan:").size(11.0).color(COLOR_TEXT_MUTED));
            let pan_slider = Slider::new(&mut pan, -1.0..=1.0)
                .custom_formatter(|v, _| {
                    if v.abs() < 0.05 {
                        "C".to_string()
                    } else if v < 0.0 {
                        format!("L {:.0}%", -v * 100.0)
                    } else {
                        format!("R {:.0}%", v * 100.0)
                    }
                })
                .step_by(0.01);

            if ui.add(pan_slider).changed() {
                channel.set_pan(pan);
            }
        });

        ui.add_space(6.0);

        // Fader and VU Meter Side-by-Side
        ui.horizontal(|ui| {
            // Volume Fader (Vertical Slider: -60 dB to +6 dB)
            ui.vertical(|ui| {
                let current_gain = channel.gain();
                let mut vol_db = if current_gain <= 1e-4 {
                    -60.0
                } else {
                    channel.volume_db().clamp(-60.0, 6.0)
                };

                ui.label(RichText::new("Gain").size(10.5).color(COLOR_TEXT_MUTED));

                let slider = Slider::new(&mut vol_db, -60.0..=6.0)
                    .vertical()
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
                        channel.set_gain(0.0);
                    } else {
                        channel.set_volume_db(vol_db);
                    }
                }

                // Unity 0.0 dB Reset Button
                if ui
                    .add(egui::Button::new(RichText::new("0 dB").size(9.5)).frame(true))
                    .clicked()
                {
                    channel.set_volume_db(0.0);
                }
            });

            ui.add_space(10.0);

            // VU Meter
            ui.vertical(|ui| {
                ui.label(RichText::new("Meter").size(10.5).color(COLOR_TEXT_MUTED));
                render_vu_meter(ui, channel.meter());
            });
        });
    });
}

/// Render stereo peak and RMS level VU meter with clipping indicator.
fn render_vu_meter(ui: &mut Ui, meter: &ChannelMeter) {
    crate::gui::meter::render_stereo_vu_meter(ui, meter, vec2(36.0, 150.0));
}

/// Render the effect rack section title and `[+ Add Effect]` dropdown menu.
fn render_rack_header(ui: &mut Ui, channel: &mut ChannelStrip) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("EFFECT RACK")
                .strong()
                .size(12.0)
                .color(COLOR_TEXT_PRIMARY),
        );

        let rack_count = channel.chain.len();
        ui.label(
            RichText::new(format!("({} active)", rack_count))
                .size(11.0)
                .color(COLOR_TEXT_DIM),
        );

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            egui::ComboBox::from_id_source(format!("{}_add_effect_menu", channel.name()))
                .selected_text(RichText::new("+ Add Effect").color(COLOR_ACCENT_BLUE))
                .show_ui(ui, |ui| {
                    for effect_kind in AvailableEffect::ALL {
                        if ui.button(effect_kind.name()).clicked() {
                            let new_effect = effect_kind.instantiate(channel.sample_rate());
                            channel.chain_mut().add_effect(new_effect);
                        }
                    }
                });
        });
    });
}

/// Render all modular cards in the channel's effect chain and execute reorder/delete actions.
fn render_effect_rack(ui: &mut Ui, channel: &mut ChannelStrip) {
    let mut pending_delete: Option<usize> = None;
    let mut pending_move_up: Option<usize> = None;
    let mut pending_move_down: Option<usize> = None;

    let total = channel.chain.len();
    if total == 0 {
        ui.add_space(10.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new("Rack empty. Click [+ Add Effect] to insert DSP modules.")
                    .italics()
                    .size(11.0)
                    .color(COLOR_TEXT_DIM),
            );
        });
        ui.add_space(10.0);
        return;
    }

    for (i, effect) in channel.chain.iter_mut().enumerate() {
        let action = render_effect_card_indexed(ui, effect.as_mut(), i, total);
        match action {
            EffectCardAction::Delete => pending_delete = Some(i),
            EffectCardAction::MoveUp => pending_move_up = Some(i),
            EffectCardAction::MoveDown => pending_move_down = Some(i),
            EffectCardAction::ToggleBypass | EffectCardAction::None => {}
        }
    }

    // Apply any structural changes after rendering loop
    if let Some(idx) = pending_delete {
        channel.chain_mut().remove_effect(idx);
    } else if let Some(idx) = pending_move_up {
        if idx > 0 {
            channel.chain_mut().swap_effects(idx, idx - 1);
        }
    } else if let Some(idx) = pending_move_down {
        if idx + 1 < total {
            channel.chain_mut().swap_effects(idx, idx + 1);
        }
    }
}
