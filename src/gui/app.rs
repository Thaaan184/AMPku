//! Main AMPku studio application struct wiring audio engine, GUI transport, channel strips,
//! master limiter bus, preset management, and real-time recording into the `eframe::App` loop.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use egui::{Color32, Frame, Margin, RichText, Rounding, Stroke, Ui};

use crate::audio::device::{AudioDeviceInfo, AudioStreamConfig};
use crate::audio::engine::AudioEngine;
use crate::audio::recorder::WavRecorder;
use crate::dsp::cab_sim::CabinetSim;
use crate::dsp::dynamics::{Compressor, NoiseGate};
use crate::dsp::filters::biquad::{BiquadFilter, FilterType};
use crate::dsp::saturation::{Distortion, Fuzz, Overdrive};
use crate::dsp::time::{StereoDelay, StereoReverb};
use crate::dsp::vocal::{DeEsserMode, VocalDeEsser};
use crate::engine::chain::EffectChain;
use crate::gui::channel_strip::render_dual_channel_strips;
use crate::gui::theme::{
    apply_studio_theme, COLOR_ACCENT_BLUE, COLOR_ALERT_RED, COLOR_ALERT_RED_BG,
};
use crate::gui::transport::{render_master_sum_bar, render_top_bar, TopBarState, TransportAction};
use crate::presets::manager::PresetManager;
use crate::presets::schema::{ChannelPreset, EffectPreset, MasterPreset, PresetData};

/// Main application state for AMPku Studio.
pub struct AmpkuApp {
    /// Core audio engine managing duplex hardware streaming, DSP routing, and metering.
    pub engine: AudioEngine,
    /// Preset manager handling saving, loading, and discovering tone presets.
    pub preset_manager: PresetManager,
    /// Background WAV recorder reference.
    pub recorder: Arc<Mutex<WavRecorder>>,
    /// Available hardware audio input devices.
    pub available_inputs: Vec<AudioDeviceInfo>,
    /// Available hardware audio output devices.
    pub available_outputs: Vec<AudioDeviceInfo>,
    /// Currently selected hardware input device name (None = default system device).
    pub selected_input: Option<String>,
    /// Currently selected hardware output device name (None = default system device).
    pub selected_output: Option<String>,
    /// Streaming buffer size in frames (default: 64 frames = ~1.3 ms at 48 kHz).
    pub buffer_size: u32,
    /// Audio sample rate in Hz (default: 48000 Hz).
    pub sample_rate: u32,
    /// List of available factory and user preset names.
    pub preset_names: Vec<String>,
    /// Name of the currently selected or active preset.
    pub selected_preset: String,
    /// Temporary text buffer for preset name in the Save dialog.
    pub save_preset_name: String,
    /// Flag controlling visibility of the Save Preset modal window.
    pub show_save_dialog: bool,
    /// Target directory for exported WAV recordings.
    pub recordings_dir: PathBuf,
    /// Ephemeral status/error message banner displayed in the UI: `(text, is_error, timestamp)`.
    pub status_message: Option<(String, bool, Instant)>,
}

impl Default for AmpkuApp {
    fn default() -> Self {
        Self::create()
    }
}

impl AmpkuApp {
    /// Create and initialize `AmpkuApp` for native eframe desktop execution.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        apply_studio_theme(&cc.egui_ctx);
        Self::create()
    }

    /// Core constructor initializing audio engine, preset manager, and recorder.
    pub fn create() -> Self {
        let buffer_size = 64;
        let sample_rate = 48000;
        let config = AudioStreamConfig::new(sample_rate, buffer_size);

        let engine = AudioEngine::with_config(config)
            .unwrap_or_else(|_| AudioEngine::new().expect("Failed to create default AudioEngine"));

        let recorder = Arc::new(Mutex::new(WavRecorder::new()));
        engine.attach_recorder(recorder.clone());

        let available_inputs = engine.input_devices();
        let available_outputs = engine.output_devices();

        let preset_manager = PresetManager::new();
        // Ensure default directory and factory presets are written to disk
        let _ = preset_manager.save_factory_presets_to_disk();
        let preset_names = preset_manager.list_presets();

        let default_preset_name = if preset_names.iter().any(|p| p == "Texas Blues") {
            "Texas Blues".to_string()
        } else if let Some(first) = preset_names.first() {
            first.clone()
        } else {
            "Default Tone".to_string()
        };

        let mut app = Self {
            engine,
            preset_manager,
            recorder,
            available_inputs,
            available_outputs,
            selected_input: None,
            selected_output: None,
            buffer_size,
            sample_rate,
            preset_names,
            selected_preset: default_preset_name.clone(),
            save_preset_name: default_preset_name.clone(),
            show_save_dialog: false,
            recordings_dir: PathBuf::from("./recordings"),
            status_message: None,
        };

        // Ensure recordings directory exists
        let _ = std::fs::create_dir_all(&app.recordings_dir);

        // Apply initial preset if available
        app.load_preset(&default_preset_name);

        app
    }

    /// Display an ephemeral status or alert banner in the studio header.
    pub fn set_status(&mut self, message: impl Into<String>, is_error: bool) {
        self.status_message = Some((message.into(), is_error, Instant::now()));
    }

    /// Return true if the background WAV recorder is actively capturing audio.
    #[inline]
    pub fn is_recording(&self) -> bool {
        self.recorder
            .lock()
            .map(|r| r.is_recording())
            .unwrap_or(false)
    }

    /// Return elapsed duration of current recording session in seconds.
    #[inline]
    pub fn record_duration_seconds(&self) -> f64 {
        self.recorder
            .lock()
            .map(|r| r.duration_seconds())
            .unwrap_or(0.0)
    }

    /// Load and apply a preset by name to Guitar and Mic channel strips and Master Bus.
    pub fn load_preset(&mut self, name: &str) {
        match self.preset_manager.load_preset(name) {
            Ok(preset) => {
                let sr = self.sample_rate as f32;
                let apply_result = if let (Ok(mut guitar), Ok(mut mic)) = (
                    self.engine.guitar_channel().lock(),
                    self.engine.mic_channel().lock(),
                ) {
                    preset.apply_to_channels(&mut guitar, &mut mic, sr)
                } else {
                    Ok(())
                };

                if let Err(e) = apply_result {
                    self.set_status(format!("Preset apply warning: {e}"), true);
                }

                // Apply master bus parameters if preset contains master settings
                if let Some(ref master_preset) = preset.master {
                    if let Ok(master) = self.engine.master_bus().lock() {
                        master.set_gain(master_preset.volume);
                        master.set_threshold_db(master_preset.limiter_threshold_db);
                        master.set_ceiling_db(master_preset.limiter_ceiling_db);
                        master.set_release_ms(master_preset.limiter_release_ms);
                    }
                }

                self.selected_preset = name.to_string();
                self.save_preset_name = name.to_string();
                self.set_status(format!("Loaded preset '{}'", name), false);
            }
            Err(e) => {
                self.set_status(format!("Failed to load preset '{name}': {e}"), true);
            }
        }
    }

    /// Capture current channel strips and master bus state into a `PresetData` and persist to disk.
    pub fn save_preset(&mut self, name: &str) {
        let channel_presets = if let (Ok(guitar), Ok(mic)) = (
            self.engine.guitar_channel().lock(),
            self.engine.mic_channel().lock(),
        ) {
            Some((
                ChannelPreset {
                    volume: guitar.gain(),
                    pan: guitar.pan(),
                    muted: guitar.is_muted(),
                    solo: guitar.is_solo(),
                    mono: guitar.is_mono(),
                    effects: serialize_chain(&guitar.chain),
                },
                ChannelPreset {
                    volume: mic.gain(),
                    pan: mic.pan(),
                    muted: mic.is_muted(),
                    solo: mic.is_solo(),
                    mono: mic.is_mono(),
                    effects: serialize_chain(&mic.chain),
                },
            ))
        } else {
            None
        };

        let (guitar_preset, mic_preset) = match channel_presets {
            Some(presets) => presets,
            None => {
                self.set_status("Failed to acquire channel strip lock for saving", true);
                return;
            }
        };

        let master_preset = if let Ok(master) = self.engine.master_bus().lock() {
            MasterPreset {
                volume: master.gain(),
                pan: 0.0,
                limiter_threshold_db: master.threshold_db(),
                limiter_ceiling_db: master.ceiling_db(),
                limiter_release_ms: master.release_ms(),
            }
        } else {
            MasterPreset::default()
        };

        let preset_data =
            PresetData::new(name, guitar_preset, mic_preset).with_master(master_preset);

        match self.preset_manager.save_preset(&preset_data) {
            Ok(saved_path) => {
                self.preset_names = self.preset_manager.list_presets();
                self.selected_preset = name.to_string();
                self.set_status(format!("Saved preset to {}", saved_path.display()), false);
            }
            Err(e) => {
                self.set_status(format!("Failed to save preset: {e}"), true);
            }
        }
    }

    /// Toggle background WAV recording on or off.
    pub fn toggle_recording(&mut self) {
        if self.is_recording() {
            let stop_result = if let Ok(mut rec) = self.recorder.lock() {
                let res = rec.stop_recording();
                let total = rec.frames_recorded();
                Some((res, total))
            } else {
                None
            };

            match stop_result {
                Some((Ok(()), total)) => {
                    self.set_status(format!("Recording stopped ({total} frames saved)"), false);
                }
                Some((Err(e), _)) => {
                    self.set_status(format!("Recording stop error: {e}"), true);
                }
                None => {
                    self.set_status("Failed to lock recorder", true);
                }
            }
        } else {
            let _ = std::fs::create_dir_all(&self.recordings_dir);
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let target_path = self
                .recordings_dir
                .join(format!("ampku_rec_{timestamp}.wav"));

            let start_result = if let Ok(mut rec) = self.recorder.lock() {
                Some(rec.start_recording(&target_path, self.sample_rate))
            } else {
                None
            };

            match start_result {
                Some(Ok(())) => {
                    self.set_status(
                        format!("Recording started: {}", target_path.display()),
                        false,
                    );
                }
                Some(Err(e)) => {
                    self.set_status(format!("Failed to start recording: {e}"), true);
                }
                None => {
                    self.set_status("Failed to lock recorder", true);
                }
            }
        }
    }

    /// Open recordings directory in the host system file browser.
    pub fn open_recordings_folder(&mut self) {
        let _ = std::fs::create_dir_all(&self.recordings_dir);

        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("explorer")
                .arg(&self.recordings_dir)
                .spawn();
        }
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open")
                .arg(&self.recordings_dir)
                .spawn();
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            let _ = std::process::Command::new("xdg-open")
                .arg(&self.recordings_dir)
                .spawn();
        }

        self.set_status(
            format!("Recordings directory: {}", self.recordings_dir.display()),
            false,
        );
    }

    /// Handle high-level transport actions emitted from the top header bar.
    pub fn handle_transport_action(&mut self, action: TransportAction) {
        match action {
            TransportAction::None => {}
            TransportAction::ToggleEngine => {
                if self.engine.is_running() {
                    let _ = self.engine.stop();
                    self.set_status("Audio engine stopped", false);
                } else {
                    let in_dev = self.selected_input.as_deref();
                    let out_dev = self.selected_output.as_deref();
                    match self.engine.start(in_dev, out_dev) {
                        Ok(()) => self.set_status("Audio engine streaming active", false),
                        Err(e) => self.set_status(format!("Audio engine error: {e}"), true),
                    }
                }
            }
            TransportAction::ChangeInputDevice(dev_name) => {
                if dev_name == "Default Input" {
                    self.selected_input = None;
                } else {
                    self.selected_input = Some(dev_name.clone());
                }

                if self.engine.is_running() {
                    let in_dev = self.selected_input.as_deref();
                    let out_dev = self.selected_output.as_deref();
                    let _ = self.engine.start(in_dev, out_dev);
                }
                self.set_status(format!("Input device: {dev_name}"), false);
            }
            TransportAction::ChangeOutputDevice(dev_name) => {
                if dev_name == "Default Output" {
                    self.selected_output = None;
                } else {
                    self.selected_output = Some(dev_name.clone());
                }

                if self.engine.is_running() {
                    let in_dev = self.selected_input.as_deref();
                    let out_dev = self.selected_output.as_deref();
                    let _ = self.engine.start(in_dev, out_dev);
                }
                self.set_status(format!("Output device: {dev_name}"), false);
            }
            TransportAction::ChangeBufferSize(size) => {
                self.buffer_size = size;
                let config = AudioStreamConfig::new(self.sample_rate, size);
                match self.engine.set_config(config) {
                    Ok(()) => {
                        let lat_ms = (size as f32 / self.sample_rate as f32) * 1000.0;
                        self.set_status(
                            format!("Buffer size updated: {size} smp ({:.1} ms)", lat_ms),
                            false,
                        );
                    }
                    Err(e) => {
                        self.set_status(format!("Buffer size update error: {e}"), true);
                    }
                }
            }
            TransportAction::SelectPreset(name) => {
                self.load_preset(&name);
            }
            TransportAction::SavePresetPrompt => {
                self.save_preset_name = self.selected_preset.clone();
                self.show_save_dialog = true;
            }
            TransportAction::SavePresetConfirm(name) => {
                self.save_preset(&name);
            }
            TransportAction::ToggleRecord => {
                self.toggle_recording();
            }
            TransportAction::OpenRecordingsFolder => {
                self.open_recordings_folder();
            }
        }
    }

    /// Render temporary status or error banner if active.
    fn render_status_banner(&mut self, ui: &mut Ui) {
        if let Some((msg, is_error, timestamp)) = &self.status_message {
            if timestamp.elapsed() < std::time::Duration::from_secs(5) {
                let bg_color = if *is_error {
                    COLOR_ALERT_RED_BG
                } else {
                    Color32::from_rgb(0x13, 0x2e, 0x20)
                };
                let text_color = if *is_error {
                    COLOR_ALERT_RED
                } else {
                    COLOR_ACCENT_BLUE
                };
                Frame::none()
                    .fill(bg_color)
                    .stroke(Stroke::new(1.0_f32, text_color))
                    .rounding(Rounding::same(4.0))
                    .inner_margin(Margin::symmetric(8.0, 4.0))
                    .show(ui, |ui| {
                        ui.label(RichText::new(msg).size(11.0).color(text_color));
                    });
                ui.add_space(4.0);
            }
        }
    }

    /// Render modal popup window for saving tone presets.
    fn render_save_preset_dialog(&mut self, ctx: &egui::Context) {
        let mut close_dialog = false;
        let mut confirmed_name: Option<String> = None;

        egui::Window::new("Save Tone Preset")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.label(RichText::new("Preset Name:").size(11.5));
                ui.text_edit_singleline(&mut self.save_preset_name);
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    if ui
                        .add(egui::Button::new(RichText::new("Save").strong()))
                        .clicked()
                    {
                        let name = self.save_preset_name.trim().to_string();
                        if !name.is_empty() {
                            confirmed_name = Some(name);
                            close_dialog = true;
                        }
                    }

                    if ui.button("Cancel").clicked() {
                        close_dialog = true;
                    }
                });
            });

        if let Some(name) = confirmed_name {
            self.save_preset(&name);
        }

        if close_dialog {
            self.show_save_dialog = false;
        }
    }

    /// Render all studio GUI panels and widgets for the current frame.
    pub fn render_ui(&mut self, ctx: &egui::Context) {
        // Continuous 60 FPS repainting for real-time VU meter ballistics and duration timer
        ctx.request_repaint_after(std::time::Duration::from_millis(16));

        // 1. Top Transport Header Panel
        let mut top_action = TransportAction::None;
        egui::TopBottomPanel::top("ampku_top_panel").show(ctx, |ui| {
            let is_recording = self.is_recording();
            let record_dur = self.record_duration_seconds();
            let is_running = self.engine.is_running();

            let state = TopBarState {
                is_running,
                available_inputs: &self.available_inputs,
                available_outputs: &self.available_outputs,
                selected_input: self.selected_input.as_deref(),
                selected_output: self.selected_output.as_deref(),
                buffer_size: self.buffer_size,
                sample_rate: self.sample_rate,
                preset_names: &self.preset_names,
                selected_preset: &self.selected_preset,
                is_recording,
                record_duration_secs: record_dur,
            };

            top_action = render_top_bar(ui, &state);
        });

        // Handle transport action from header
        self.handle_transport_action(top_action);

        // 2. Bottom Master Bus & Limiter Panel
        egui::TopBottomPanel::bottom("ampku_bottom_panel").show(ctx, |ui| {
            if let Ok(mut master) = self.engine.master_bus().lock() {
                render_master_sum_bar(ui, &mut master);
            }
        });

        // 3. Central Dual Channel Strips (Guitar Ch 1 & Mic Ch 2)
        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_status_banner(ui);

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if let (Ok(mut guitar), Ok(mut mic)) = (
                        self.engine.guitar_channel().lock(),
                        self.engine.mic_channel().lock(),
                    ) {
                        render_dual_channel_strips(ui, &mut guitar, &mut mic);
                    }
                });
        });

        // 4. Modal Save Preset Dialog
        if self.show_save_dialog {
            self.render_save_preset_dialog(ctx);
        }
    }
}

impl eframe::App for AmpkuApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.render_ui(ctx);
    }
}

/// Helper function serializing an `EffectChain` into a vector of `EffectPreset`.
fn serialize_chain(chain: &EffectChain) -> Vec<EffectPreset> {
    let mut presets = Vec::with_capacity(chain.len());

    for effect in chain.effects() {
        let is_bypassed = !effect.is_enabled();
        let any = effect.as_any();

        if let Some(ng) = any.downcast_ref::<NoiseGate>() {
            presets.push(EffectPreset::noise_gate(
                ng.threshold_db(),
                ng.attack_ms(),
                ng.release_ms(),
                is_bypassed,
            ));
        } else if let Some(comp) = any.downcast_ref::<Compressor>() {
            presets.push(EffectPreset::compressor(
                comp.threshold_db(),
                comp.ratio(),
                comp.attack_ms(),
                comp.release_ms(),
                comp.makeup_gain_db(),
                is_bypassed,
            ));
        } else if let Some(od) = any.downcast_ref::<Overdrive>() {
            presets.push(EffectPreset::overdrive(
                od.drive(),
                od.tone(),
                od.level(),
                is_bypassed,
            ));
        } else if let Some(dist) = any.downcast_ref::<Distortion>() {
            presets.push(EffectPreset::distortion(
                dist.drive(),
                dist.tone(),
                dist.level(),
                is_bypassed,
            ));
        } else if let Some(fuzz) = any.downcast_ref::<Fuzz>() {
            presets.push(EffectPreset::fuzz(
                fuzz.gain(),
                fuzz.tone(),
                fuzz.level(),
                is_bypassed,
            ));
        } else if let Some(cab) = any.downcast_ref::<CabinetSim>() {
            presets.push(EffectPreset::cab_sim(
                cab.profile().name(),
                cab.mix(),
                cab.output_gain_db(),
                is_bypassed,
            ));
        } else if let Some(del) = any.downcast_ref::<StereoDelay>() {
            presets.push(EffectPreset::delay(
                del.delay_ms(),
                del.feedback(),
                del.mix(),
                del.damping(),
                del.ping_pong(),
                del.stereo_spread(),
                is_bypassed,
            ));
        } else if let Some(rev) = any.downcast_ref::<StereoReverb>() {
            presets.push(EffectPreset::reverb(
                rev.room_size(),
                rev.damping(),
                rev.width(),
                rev.mix(),
                is_bypassed,
            ));
        } else if let Some(bi) = any.downcast_ref::<BiquadFilter>() {
            let type_str = match bi.filter_type() {
                FilterType::LowPass => "LowPass",
                FilterType::HighPass => "HighPass",
                FilterType::Peaking => "Peaking",
                FilterType::BandPass => "BandPass",
                FilterType::Notch => "Notch",
                FilterType::LowShelf => "LowShelf",
                FilterType::HighShelf => "HighShelf",
            };
            presets.push(EffectPreset::biquad(
                type_str,
                bi.frequency(),
                bi.q(),
                bi.gain_db(),
                is_bypassed,
            ));
        } else if let Some(de) = any.downcast_ref::<VocalDeEsser>() {
            let mode_str = match de.mode() {
                DeEsserMode::SplitBand => "SplitBand",
                DeEsserMode::WideBand => "WideBand",
            };
            presets.push(EffectPreset::vocal_de_esser(
                de.frequency(),
                de.threshold_db(),
                de.ratio(),
                de.attack_ms(),
                de.release_ms(),
                mode_str,
                is_bypassed,
            ));
        }
    }

    presets
}
