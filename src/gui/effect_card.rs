//! Modular DSP effect card UI component for AMPku studio rack.
//!
//! Renders collapsible parameter cards for all 10 audio effect nodes:
//! - NoiseGate
//! - Compressor
//! - Overdrive
//! - Distortion
//! - Fuzz
//! - BiquadFilter
//! - CabinetSim
//! - StereoDelay
//! - StereoReverb
//! - VocalDeEsser

use egui::{collapsing_header::CollapsingState, Color32, RichText, Slider, Ui};

use crate::dsp::cab_sim::{CabinetProfile, CabinetSim};
use crate::dsp::dynamics::{Compressor, NoiseGate};
use crate::dsp::filters::biquad::{BiquadFilter, FilterType};
use crate::dsp::saturation::{Distortion, Fuzz, Overdrive};
use crate::dsp::time::{StereoDelay, StereoReverb};
use crate::dsp::traits::AudioEffect;
use crate::dsp::vocal::{DeEsserMode, VocalDeEsser};
use crate::gui::theme::{
    effect_card_frame, COLOR_ACCENT_BLUE, COLOR_ACTIVE_GREEN, COLOR_ALERT_RED, COLOR_TEXT_DIM,
    COLOR_TEXT_MUTED,
};

/// User action triggered from an effect card control header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectCardAction {
    /// No structural action taken.
    None,
    /// Request deletion of this effect node from the rack.
    Delete,
    /// Request moving this effect up (earlier in signal flow).
    MoveUp,
    /// Request moving this effect down (later in signal flow).
    MoveDown,
    /// Power / bypass status was toggled.
    ToggleBypass,
}

/// Render a modular effect card with power toggle, move up/down, delete [X], and parameters.
pub fn render_effect_card(ui: &mut Ui, effect: &mut dyn AudioEffect) -> EffectCardAction {
    render_effect_card_indexed(ui, effect, 0, 1)
}

/// Render a modular effect card aware of its rack position for enabling/disabling reorder buttons.
pub fn render_effect_card_indexed(
    ui: &mut Ui,
    effect: &mut dyn AudioEffect,
    index: usize,
    total_count: usize,
) -> EffectCardAction {
    let mut action = EffectCardAction::None;
    let enabled = effect.is_enabled();

    effect_card_frame(enabled).show(ui, |ui| {
        let card_id =
            ui.make_persistent_id(format!("effect_card_{}_{:p}", index, effect as *const _));
        let state = CollapsingState::load_with_default_open(ui.ctx(), card_id, true);

        state
            .show_header(ui, |ui| {
                ui.horizontal(|ui| {
                    let title_color = if enabled {
                        Color32::WHITE
                    } else {
                        COLOR_TEXT_DIM
                    };
                    ui.label(
                        RichText::new(effect.name())
                            .strong()
                            .size(13.5)
                            .color(title_color),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Delete [X] button
                        let delete_btn = ui.add(
                            egui::Button::new(RichText::new("✕").size(11.0).color(COLOR_ALERT_RED))
                                .frame(true),
                        );
                        if delete_btn.clicked() {
                            action = EffectCardAction::Delete;
                        }

                        // Move Down [▼] button
                        let can_move_down = index + 1 < total_count;
                        let down_btn = ui.add_enabled(
                            can_move_down,
                            egui::Button::new(RichText::new("▼").size(10.0)).frame(true),
                        );
                        if down_btn.clicked() {
                            action = EffectCardAction::MoveDown;
                        }

                        // Move Up [▲] button
                        let can_move_up = index > 0;
                        let up_btn = ui.add_enabled(
                            can_move_up,
                            egui::Button::new(RichText::new("▲").size(10.0)).frame(true),
                        );
                        if up_btn.clicked() {
                            action = EffectCardAction::MoveUp;
                        }

                        // Power / Bypass toggle button
                        let (power_text, power_color) = if enabled {
                            ("● ON", COLOR_ACTIVE_GREEN)
                        } else {
                            ("○ BYPASS", COLOR_TEXT_MUTED)
                        };
                        let power_btn = ui.add(
                            egui::Button::new(
                                RichText::new(power_text).size(11.0).color(power_color),
                            )
                            .frame(true),
                        );
                        if power_btn.clicked() {
                            effect.set_enabled(!enabled);
                            action = EffectCardAction::ToggleBypass;
                        }
                    });
                });
            })
            .body(|ui| {
                ui.add_space(4.0);
                render_effect_parameters(ui, effect);
            });
    });

    action
}

/// Dispatch and render parameters for the concrete DSP effect implementation.
fn render_effect_parameters(ui: &mut Ui, effect: &mut dyn AudioEffect) {
    let any_mut = effect.as_any_mut();

    if let Some(gate) = any_mut.downcast_mut::<NoiseGate>() {
        render_noise_gate(ui, gate);
    } else if let Some(comp) = any_mut.downcast_mut::<Compressor>() {
        render_compressor(ui, comp);
    } else if let Some(od) = any_mut.downcast_mut::<Overdrive>() {
        render_overdrive(ui, od);
    } else if let Some(dist) = any_mut.downcast_mut::<Distortion>() {
        render_distortion(ui, dist);
    } else if let Some(fuzz) = any_mut.downcast_mut::<Fuzz>() {
        render_fuzz(ui, fuzz);
    } else if let Some(biquad) = any_mut.downcast_mut::<BiquadFilter>() {
        render_biquad(ui, biquad);
    } else if let Some(cab) = any_mut.downcast_mut::<CabinetSim>() {
        render_cab_sim(ui, cab);
    } else if let Some(delay) = any_mut.downcast_mut::<StereoDelay>() {
        render_delay(ui, delay);
    } else if let Some(reverb) = any_mut.downcast_mut::<StereoReverb>() {
        render_reverb(ui, reverb);
    } else if let Some(de_esser) = any_mut.downcast_mut::<VocalDeEsser>() {
        render_de_esser(ui, de_esser);
    } else {
        ui.label(
            RichText::new("Generic Audio Effect Node")
                .italics()
                .color(COLOR_TEXT_MUTED),
        );
    }
}

fn render_noise_gate(ui: &mut Ui, gate: &mut NoiseGate) {
    let mut thresh = gate.threshold_db();
    let mut attack = gate.attack_ms();
    let mut hold = gate.hold_ms();
    let mut release = gate.release_ms();

    ui.horizontal(|ui| {
        ui.label("Threshold:");
        if ui
            .add(
                Slider::new(&mut thresh, -100.0..=0.0)
                    .suffix(" dB")
                    .step_by(0.5),
            )
            .changed()
        {
            gate.set_threshold_db(thresh);
        }
    });

    ui.horizontal(|ui| {
        ui.label("Attack:");
        if ui
            .add(
                Slider::new(&mut attack, 0.1..=50.0)
                    .suffix(" ms")
                    .step_by(0.1),
            )
            .changed()
        {
            gate.set_attack_ms(attack);
        }
    });

    ui.horizontal(|ui| {
        ui.label("Hold:");
        if ui
            .add(
                Slider::new(&mut hold, 0.0..=200.0)
                    .suffix(" ms")
                    .step_by(1.0),
            )
            .changed()
        {
            gate.set_hold_ms(hold);
        }
    });

    ui.horizontal(|ui| {
        ui.label("Release:");
        if ui
            .add(
                Slider::new(&mut release, 1.0..=500.0)
                    .suffix(" ms")
                    .step_by(1.0),
            )
            .changed()
        {
            gate.set_release_ms(release);
        }
    });

    ui.horizontal(|ui| {
        let (status_text, color) = if gate.is_open() {
            ("OPEN", COLOR_ACTIVE_GREEN)
        } else {
            ("CLOSED", COLOR_TEXT_DIM)
        };
        ui.label(
            RichText::new(format!("Gate: {}", status_text))
                .size(11.0)
                .color(color),
        );
    });
}

fn render_compressor(ui: &mut Ui, comp: &mut Compressor) {
    let mut thresh = comp.threshold_db();
    let mut ratio = comp.ratio();
    let mut attack = comp.attack_ms();
    let mut release = comp.release_ms();
    let mut makeup = comp.makeup_gain_db();

    ui.horizontal(|ui| {
        ui.label("Threshold:");
        if ui
            .add(
                Slider::new(&mut thresh, -80.0..=0.0)
                    .suffix(" dB")
                    .step_by(0.5),
            )
            .changed()
        {
            comp.set_threshold_db(thresh);
        }
    });

    ui.horizontal(|ui| {
        ui.label("Ratio:");
        if ui
            .add(
                Slider::new(&mut ratio, 1.0..=20.0)
                    .custom_formatter(|v, _| format!("{:.1}:1", v))
                    .step_by(0.1),
            )
            .changed()
        {
            comp.set_ratio(ratio);
        }
    });

    ui.horizontal(|ui| {
        ui.label("Attack:");
        if ui
            .add(
                Slider::new(&mut attack, 0.1..=100.0)
                    .suffix(" ms")
                    .step_by(0.5),
            )
            .changed()
        {
            comp.set_attack_ms(attack);
        }
    });

    ui.horizontal(|ui| {
        ui.label("Release:");
        if ui
            .add(
                Slider::new(&mut release, 5.0..=1000.0)
                    .suffix(" ms")
                    .step_by(5.0),
            )
            .changed()
        {
            comp.set_release_ms(release);
        }
    });

    ui.horizontal(|ui| {
        ui.label("Makeup:");
        if ui
            .add(
                Slider::new(&mut makeup, -12.0..=24.0)
                    .custom_formatter(|v, _| format!("{:+.1} dB", v))
                    .step_by(0.5),
            )
            .changed()
        {
            comp.set_makeup_gain_db(makeup);
        }
    });

    // Active Gain Reduction indicator
    let gr = comp.gain_reduction_db();
    ui.horizontal(|ui| {
        ui.label(RichText::new("GR:").size(11.0).color(COLOR_TEXT_MUTED));
        let gr_fraction = (gr / 24.0).clamp(0.0, 1.0);
        let progress = egui::ProgressBar::new(gr_fraction)
            .text(format!("{:.1} dB", gr))
            .fill(COLOR_ACCENT_BLUE);
        ui.add(progress);
    });
}

fn render_overdrive(ui: &mut Ui, od: &mut Overdrive) {
    let mut drive = od.drive();
    let mut tone = od.tone();
    let mut level = od.level();

    ui.horizontal(|ui| {
        ui.label("Drive:");
        if ui
            .add(Slider::new(&mut drive, 0.0..=30.0).step_by(0.1))
            .changed()
        {
            od.set_drive(drive);
        }
    });

    ui.horizontal(|ui| {
        ui.label("Tone:");
        if ui
            .add(Slider::new(&mut tone, 0.0..=1.0).step_by(0.01))
            .changed()
        {
            od.set_tone(tone);
        }
    });

    ui.horizontal(|ui| {
        ui.label("Level:");
        if ui
            .add(Slider::new(&mut level, 0.0..=3.0).step_by(0.05))
            .changed()
        {
            od.set_level(level);
        }
    });
}

fn render_distortion(ui: &mut Ui, dist: &mut Distortion) {
    let mut drive = dist.drive();
    let mut tone = dist.tone();
    let mut level = dist.level();

    ui.horizontal(|ui| {
        ui.label("Drive:");
        if ui
            .add(Slider::new(&mut drive, 0.0..=50.0).step_by(0.1))
            .changed()
        {
            dist.set_drive(drive);
        }
    });

    ui.horizontal(|ui| {
        ui.label("Tone:");
        if ui
            .add(Slider::new(&mut tone, 0.0..=1.0).step_by(0.01))
            .changed()
        {
            dist.set_tone(tone);
        }
    });

    ui.horizontal(|ui| {
        ui.label("Level:");
        if ui
            .add(Slider::new(&mut level, 0.0..=3.0).step_by(0.05))
            .changed()
        {
            dist.set_level(level);
        }
    });
}

fn render_fuzz(ui: &mut Ui, fuzz: &mut Fuzz) {
    let mut gain = fuzz.gain();
    let mut tone = fuzz.tone();
    let mut level = fuzz.level();

    ui.horizontal(|ui| {
        ui.label("Gain:");
        if ui
            .add(Slider::new(&mut gain, 0.0..=50.0).step_by(0.1))
            .changed()
        {
            fuzz.set_gain(gain);
        }
    });

    ui.horizontal(|ui| {
        ui.label("Tone:");
        if ui
            .add(Slider::new(&mut tone, 0.0..=1.0).step_by(0.01))
            .changed()
        {
            fuzz.set_tone(tone);
        }
    });

    ui.horizontal(|ui| {
        ui.label("Level:");
        if ui
            .add(Slider::new(&mut level, 0.0..=3.0).step_by(0.05))
            .changed()
        {
            fuzz.set_level(level);
        }
    });
}

fn render_biquad(ui: &mut Ui, biquad: &mut BiquadFilter) {
    let current_type = biquad.filter_type();
    let mut selected_type = current_type;

    ui.horizontal(|ui| {
        ui.label("Type:");
        egui::ComboBox::from_id_source("biquad_type_combo")
            .selected_text(match selected_type {
                FilterType::LowPass => "Low-Pass",
                FilterType::HighPass => "High-Pass",
                FilterType::Peaking => "Peaking EQ",
                FilterType::BandPass => "Band-Pass",
                FilterType::Notch => "Notch",
                FilterType::LowShelf => "Low-Shelf",
                FilterType::HighShelf => "High-Shelf",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut selected_type, FilterType::LowPass, "Low-Pass");
                ui.selectable_value(&mut selected_type, FilterType::HighPass, "High-Pass");
                ui.selectable_value(&mut selected_type, FilterType::Peaking, "Peaking EQ");
                ui.selectable_value(&mut selected_type, FilterType::BandPass, "Band-Pass");
                ui.selectable_value(&mut selected_type, FilterType::Notch, "Notch");
                ui.selectable_value(&mut selected_type, FilterType::LowShelf, "Low-Shelf");
                ui.selectable_value(&mut selected_type, FilterType::HighShelf, "High-Shelf");
            });
    });

    if selected_type != current_type {
        biquad.set_type(selected_type);
    }

    let mut freq = biquad.frequency();
    let mut q = biquad.q();
    let mut gain = biquad.gain_db();

    ui.horizontal(|ui| {
        ui.label("Freq:");
        if ui
            .add(
                Slider::new(&mut freq, 20.0..=20000.0)
                    .logarithmic(true)
                    .suffix(" Hz")
                    .step_by(1.0),
            )
            .changed()
        {
            biquad.set_frequency(freq);
        }
    });

    ui.horizontal(|ui| {
        ui.label("Q:");
        if ui
            .add(Slider::new(&mut q, 0.1..=10.0).step_by(0.05))
            .changed()
        {
            biquad.set_q(q);
        }
    });

    if matches!(
        selected_type,
        FilterType::Peaking | FilterType::LowShelf | FilterType::HighShelf
    ) {
        ui.horizontal(|ui| {
            ui.label("Gain:");
            if ui
                .add(
                    Slider::new(&mut gain, -24.0..=24.0)
                        .custom_formatter(|v, _| format!("{:+.1} dB", v))
                        .step_by(0.5),
                )
                .changed()
            {
                biquad.set_gain_db(gain);
            }
        });
    }
}

fn render_cab_sim(ui: &mut Ui, cab: &mut CabinetSim) {
    let current_profile = cab.profile();
    let mut selected_profile = current_profile;

    ui.horizontal(|ui| {
        ui.label("Cab Profile:");
        egui::ComboBox::from_id_source("cab_profile_combo")
            .selected_text(selected_profile.name())
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut selected_profile,
                    CabinetProfile::CelestionV30,
                    CabinetProfile::CelestionV30.name(),
                );
                ui.selectable_value(
                    &mut selected_profile,
                    CabinetProfile::FenderTwin,
                    CabinetProfile::FenderTwin.name(),
                );
                ui.selectable_value(
                    &mut selected_profile,
                    CabinetProfile::Marshall1960,
                    CabinetProfile::Marshall1960.name(),
                );
                ui.selectable_value(
                    &mut selected_profile,
                    CabinetProfile::AcousticSim,
                    CabinetProfile::AcousticSim.name(),
                );
            });
    });

    if selected_profile != current_profile {
        cab.set_profile(selected_profile);
    }

    let mut mix = cab.mix();
    let mut out_gain = cab.output_gain_db();

    ui.horizontal(|ui| {
        ui.label("Mix:");
        if ui
            .add(
                Slider::new(&mut mix, 0.0..=1.0)
                    .custom_formatter(|v, _| format!("{:.0}%", v * 100.0))
                    .step_by(0.01),
            )
            .changed()
        {
            cab.set_mix(mix);
        }
    });

    ui.horizontal(|ui| {
        ui.label("Output:");
        if ui
            .add(
                Slider::new(&mut out_gain, -24.0..=12.0)
                    .custom_formatter(|v, _| format!("{:+.1} dB", v))
                    .step_by(0.5),
            )
            .changed()
        {
            cab.set_output_gain_db(out_gain);
        }
    });
}

fn render_delay(ui: &mut Ui, delay: &mut StereoDelay) {
    let mut delay_ms = delay.delay_ms();
    let mut feedback = delay.feedback();
    let mut mix = delay.mix();
    let mut damping = delay.damping();
    let mut ping_pong = delay.ping_pong();
    let mut spread = delay.stereo_spread();

    ui.horizontal(|ui| {
        ui.label("Time:");
        if ui
            .add(
                Slider::new(&mut delay_ms, 10.0..=1500.0)
                    .suffix(" ms")
                    .step_by(5.0),
            )
            .changed()
        {
            delay.set_delay_ms(delay_ms);
        }
    });

    ui.horizontal(|ui| {
        ui.label("Feedback:");
        if ui
            .add(
                Slider::new(&mut feedback, 0.0..=0.95)
                    .custom_formatter(|v, _| format!("{:.0}%", v * 100.0))
                    .step_by(0.01),
            )
            .changed()
        {
            delay.set_feedback(feedback);
        }
    });

    ui.horizontal(|ui| {
        ui.label("Mix (Wet/Dry):");
        if ui
            .add(
                Slider::new(&mut mix, 0.0..=1.0)
                    .custom_formatter(|v, _| format!("{:.0}%", v * 100.0))
                    .step_by(0.01),
            )
            .changed()
        {
            delay.set_mix(mix);
        }
    });

    ui.horizontal(|ui| {
        ui.label("Damping:");
        if ui
            .add(
                Slider::new(&mut damping, 0.0..=0.95)
                    .custom_formatter(|v, _| format!("{:.0}%", v * 100.0))
                    .step_by(0.01),
            )
            .changed()
        {
            delay.set_damping(damping);
        }
    });

    ui.horizontal(|ui| {
        ui.label("Stereo Spread:");
        if ui
            .add(Slider::new(&mut spread, 0.0..=1.0).step_by(0.01))
            .changed()
        {
            delay.set_stereo_spread(spread);
        }
    });

    ui.horizontal(|ui| {
        if ui.checkbox(&mut ping_pong, "Ping-Pong Bounce").changed() {
            delay.set_ping_pong(ping_pong);
        }
    });
}

fn render_reverb(ui: &mut Ui, reverb: &mut StereoReverb) {
    let mut room_size = reverb.room_size();
    let mut damping = reverb.damping();
    let mut width = reverb.width();
    let mut mix = reverb.mix();

    ui.horizontal(|ui| {
        ui.label("Room Size:");
        if ui
            .add(
                Slider::new(&mut room_size, 0.0..=1.0)
                    .custom_formatter(|v, _| format!("{:.0}%", v * 100.0))
                    .step_by(0.01),
            )
            .changed()
        {
            reverb.set_room_size(room_size);
        }
    });

    ui.horizontal(|ui| {
        ui.label("Damping:");
        if ui
            .add(
                Slider::new(&mut damping, 0.0..=1.0)
                    .custom_formatter(|v, _| format!("{:.0}%", v * 100.0))
                    .step_by(0.01),
            )
            .changed()
        {
            reverb.set_damping(damping);
        }
    });

    ui.horizontal(|ui| {
        ui.label("Width:");
        if ui
            .add(
                Slider::new(&mut width, 0.0..=1.0)
                    .custom_formatter(|v, _| format!("{:.0}%", v * 100.0))
                    .step_by(0.01),
            )
            .changed()
        {
            reverb.set_width(width);
        }
    });

    ui.horizontal(|ui| {
        ui.label("Mix (Wet/Dry):");
        if ui
            .add(
                Slider::new(&mut mix, 0.0..=1.0)
                    .custom_formatter(|v, _| format!("{:.0}%", v * 100.0))
                    .step_by(0.01),
            )
            .changed()
        {
            reverb.set_mix(mix);
        }
    });
}

fn render_de_esser(ui: &mut Ui, de_esser: &mut VocalDeEsser) {
    let mut freq = de_esser.frequency();
    let mut thresh = de_esser.threshold_db();
    let mut ratio = de_esser.ratio();
    let mut attack = de_esser.attack_ms();
    let mut release = de_esser.release_ms();
    let current_mode = de_esser.mode();
    let mut selected_mode = current_mode;
    let mut listen = de_esser.is_listen();

    ui.horizontal(|ui| {
        ui.label("Frequency:");
        if ui
            .add(
                Slider::new(&mut freq, 2000.0..=12000.0)
                    .suffix(" Hz")
                    .step_by(50.0),
            )
            .changed()
        {
            de_esser.set_frequency(freq);
        }
    });

    ui.horizontal(|ui| {
        ui.label("Threshold:");
        if ui
            .add(
                Slider::new(&mut thresh, -60.0..=0.0)
                    .suffix(" dB")
                    .step_by(0.5),
            )
            .changed()
        {
            de_esser.set_threshold_db(thresh);
        }
    });

    ui.horizontal(|ui| {
        ui.label("Ratio:");
        if ui
            .add(
                Slider::new(&mut ratio, 1.0..=16.0)
                    .custom_formatter(|v, _| format!("{:.1}:1", v))
                    .step_by(0.1),
            )
            .changed()
        {
            de_esser.set_ratio(ratio);
        }
    });

    ui.horizontal(|ui| {
        ui.label("Attack:");
        if ui
            .add(
                Slider::new(&mut attack, 0.1..=20.0)
                    .suffix(" ms")
                    .step_by(0.1),
            )
            .changed()
        {
            de_esser.set_attack_ms(attack);
        }
    });

    ui.horizontal(|ui| {
        ui.label("Release:");
        if ui
            .add(
                Slider::new(&mut release, 5.0..=200.0)
                    .suffix(" ms")
                    .step_by(1.0),
            )
            .changed()
        {
            de_esser.set_release_ms(release);
        }
    });

    ui.horizontal(|ui| {
        ui.label("Mode:");
        egui::ComboBox::from_id_source("de_esser_mode_combo")
            .selected_text(match selected_mode {
                DeEsserMode::SplitBand => "Split-Band (High-Shelf)",
                DeEsserMode::WideBand => "Wide-Band (Broadband)",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut selected_mode,
                    DeEsserMode::SplitBand,
                    "Split-Band (High-Shelf)",
                );
                ui.selectable_value(
                    &mut selected_mode,
                    DeEsserMode::WideBand,
                    "Wide-Band (Broadband)",
                );
            });
    });

    if selected_mode != current_mode {
        de_esser.set_mode(selected_mode);
    }

    ui.horizontal(|ui| {
        if ui
            .checkbox(&mut listen, "Audition Sidechain (Listen)")
            .changed()
        {
            de_esser.set_listen(listen);
        }
    });

    // Active Gain Reduction indicator
    let gr = de_esser.gain_reduction_db();
    ui.horizontal(|ui| {
        ui.label(RichText::new("GR:").size(11.0).color(COLOR_TEXT_MUTED));
        let gr_fraction = (gr / 24.0).clamp(0.0, 1.0);
        let progress = egui::ProgressBar::new(gr_fraction)
            .text(format!("{:.1} dB", gr))
            .fill(COLOR_ACCENT_BLUE);
        ui.add(progress);
    });
}
