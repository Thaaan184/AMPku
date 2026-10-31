use anyhow::Result;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};
use std::io::{stdout, Write};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use crate::audio::AudioEngine;
use crate::devices::{
    get_capture_devices, get_input_device_by_name, get_output_device_by_name, get_playback_devices,
    DeviceInfo,
};
use crate::effects::{load_f32, store_f32, EffectParams};

pub enum SelectedParam {
    CleanBright,
    CleanWarmth,
    CrunchDrive,
    CrunchTone,
    OverdriveDrive,
    OverdriveTone,
    DistortionDrive,
    DistortionTone,
    WahDepth,
    WahResonance,
    DelayTime,
    DelayFeedback,
    DelayMix,
}

impl SelectedParam {
    pub fn next(&self) -> Self {
        match self {
            Self::CleanBright => Self::CleanWarmth,
            Self::CleanWarmth => Self::CrunchDrive,
            Self::CrunchDrive => Self::CrunchTone,
            Self::CrunchTone => Self::OverdriveDrive,
            Self::OverdriveDrive => Self::OverdriveTone,
            Self::OverdriveTone => Self::DistortionDrive,
            Self::DistortionDrive => Self::DistortionTone,
            Self::DistortionTone => Self::WahDepth,
            Self::WahDepth => Self::WahResonance,
            Self::WahResonance => Self::DelayTime,
            Self::DelayTime => Self::DelayFeedback,
            Self::DelayFeedback => Self::DelayMix,
            Self::DelayMix => Self::CleanBright,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::CleanBright => "Clean Preamp: Bright",
            Self::CleanWarmth => "Clean Preamp: Warmth",
            Self::CrunchDrive => "Crunch Amp: Drive",
            Self::CrunchTone => "Crunch Amp: Tone",
            Self::OverdriveDrive => "Overdrive: Drive",
            Self::OverdriveTone => "Overdrive: Tone",
            Self::DistortionDrive => "Distortion: Drive",
            Self::DistortionTone => "Distortion: Tone",
            Self::WahDepth => "Auto-Wah: Depth",
            Self::WahResonance => "Auto-Wah: Resonance",
            Self::DelayTime => "Delay: Time",
            Self::DelayFeedback => "Delay: Feedback",
            Self::DelayMix => "Delay: Mix",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceSelectStage {
    None,
    SelectInput,
    SelectOutput,
}

pub struct TerminalApp {
    pub params: Arc<EffectParams>,
    pub selected_param: SelectedParam,
    pub device_stage: DeviceSelectStage,
    pub selected_input_cursor: usize,
    pub selected_output_cursor: usize,
    pub chosen_input_device: Option<DeviceInfo>,
    pub status_message: Option<String>,
    pub host: cpal::Host,
    pub capture_devices: Vec<DeviceInfo>,
    pub playback_devices: Vec<DeviceInfo>,
}

impl TerminalApp {
    pub fn new(params: Arc<EffectParams>) -> Self {
        let host = cpal::default_host();
        let capture_devices = get_capture_devices(&host);
        let playback_devices = get_playback_devices(&host);

        Self {
            params,
            selected_param: SelectedParam::OverdriveDrive,
            device_stage: DeviceSelectStage::None,
            selected_input_cursor: 0,
            selected_output_cursor: 0,
            chosen_input_device: None,
            status_message: None,
            host,
            capture_devices,
            playback_devices,
        }
    }

    pub fn run(&mut self, audio: &mut AudioEngine) -> Result<()> {
        terminal::enable_raw_mode()?;
        let mut out = stdout();
        execute!(out, terminal::EnterAlternateScreen, cursor::Hide)?;

        let res = self.event_loop(audio);

        execute!(out, cursor::Show, terminal::LeaveAlternateScreen)?;
        terminal::disable_raw_mode()?;

        res
    }

    fn event_loop(&mut self, audio: &mut AudioEngine) -> Result<()> {
        let mut out = stdout();

        loop {
            // 1. Render Screen
            execute!(out, cursor::MoveTo(0, 0), Clear(ClearType::All))?;
            match self.device_stage {
                DeviceSelectStage::SelectInput => self.render_select_input(&mut out)?,
                DeviceSelectStage::SelectOutput => self.render_select_output(&mut out)?,
                DeviceSelectStage::None => self.render_dashboard(&mut out, audio)?,
            }
            out.flush()?;

            // 2. Poll keyboard events at ~30 FPS (33ms)
            if event::poll(Duration::from_millis(33))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        match self.device_stage {
                            DeviceSelectStage::SelectInput => {
                                self.handle_input_device_key(key.code);
                            }
                            DeviceSelectStage::SelectOutput => {
                                self.handle_output_device_key(key.code, audio);
                            }
                            DeviceSelectStage::None => match key.code {
                                KeyCode::Char('q') | KeyCode::Char('Q') => break,
                                KeyCode::Char('d') | KeyCode::Char('D') => {
                                    self.capture_devices = get_capture_devices(&self.host);
                                    self.playback_devices = get_playback_devices(&self.host);
                                    self.selected_input_cursor = 0;
                                    self.selected_output_cursor = 0;
                                    self.chosen_input_device = None;
                                    self.device_stage = DeviceSelectStage::SelectInput;
                                }
                                KeyCode::Char('1') => {
                                    let cur = self.params.clean_enabled.load(Ordering::Relaxed);
                                    self.params.clean_enabled.store(!cur, Ordering::Relaxed);
                                }
                                KeyCode::Char('2') => {
                                    let cur = self.params.crunch_enabled.load(Ordering::Relaxed);
                                    self.params.crunch_enabled.store(!cur, Ordering::Relaxed);
                                }
                                KeyCode::Char('3') => {
                                    let cur = self.params.overdrive_enabled.load(Ordering::Relaxed);
                                    self.params.overdrive_enabled.store(!cur, Ordering::Relaxed);
                                }
                                KeyCode::Char('4') => {
                                    let cur =
                                        self.params.distortion_enabled.load(Ordering::Relaxed);
                                    self.params
                                        .distortion_enabled
                                        .store(!cur, Ordering::Relaxed);
                                }
                                KeyCode::Char('5') => {
                                    let cur = self.params.wah_enabled.load(Ordering::Relaxed);
                                    self.params.wah_enabled.store(!cur, Ordering::Relaxed);
                                }
                                KeyCode::Char('6') => {
                                    let cur = self.params.delay_enabled.load(Ordering::Relaxed);
                                    self.params.delay_enabled.store(!cur, Ordering::Relaxed);
                                }
                                KeyCode::Char('b') | KeyCode::Char('B') => {
                                    let cur = self.params.global_bypass.load(Ordering::Relaxed);
                                    self.params.global_bypass.store(!cur, Ordering::Relaxed);
                                }
                                KeyCode::Char('m') | KeyCode::Char('M') => {
                                    let cur = self.params.mute.load(Ordering::Relaxed);
                                    self.params.mute.store(!cur, Ordering::Relaxed);
                                }
                                KeyCode::Tab => {
                                    self.selected_param = self.selected_param.next();
                                }
                                KeyCode::Char('+') | KeyCode::Char('=') | KeyCode::Right => {
                                    self.adjust_param(1.0);
                                }
                                KeyCode::Char('-') | KeyCode::Left => {
                                    self.adjust_param(-1.0);
                                }
                                _ => {}
                            },
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn handle_input_device_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Esc => {
                self.device_stage = DeviceSelectStage::None;
            }
            KeyCode::Up => {
                if self.selected_input_cursor > 0 {
                    self.selected_input_cursor -= 1;
                }
            }
            KeyCode::Down => {
                if !self.capture_devices.is_empty()
                    && self.selected_input_cursor + 1 < self.capture_devices.len()
                {
                    self.selected_input_cursor += 1;
                }
            }
            KeyCode::Enter => {
                if let Some(dev) = self.capture_devices.get(self.selected_input_cursor) {
                    self.chosen_input_device = Some(dev.clone());
                    self.device_stage = DeviceSelectStage::SelectOutput;
                }
            }
            KeyCode::Char(c) if c.is_ascii_digit() => {
                let idx = (c as u8 - b'0') as usize;
                if let Some(dev) = self.capture_devices.get(idx) {
                    self.chosen_input_device = Some(dev.clone());
                    self.device_stage = DeviceSelectStage::SelectOutput;
                }
            }
            _ => {}
        }
    }

    fn handle_output_device_key(&mut self, code: KeyCode, audio: &mut AudioEngine) {
        match code {
            KeyCode::Esc => {
                self.device_stage = DeviceSelectStage::None;
            }
            KeyCode::Backspace | KeyCode::Left => {
                self.device_stage = DeviceSelectStage::SelectInput;
            }
            KeyCode::Up => {
                if self.selected_output_cursor > 0 {
                    self.selected_output_cursor -= 1;
                }
            }
            KeyCode::Down => {
                if !self.playback_devices.is_empty()
                    && self.selected_output_cursor + 1 < self.playback_devices.len()
                {
                    self.selected_output_cursor += 1;
                }
            }
            KeyCode::Enter => {
                if let Some(out_info) = self.playback_devices.get(self.selected_output_cursor) {
                    self.apply_device_switch(out_info.clone(), audio);
                }
            }
            KeyCode::Char(c) if c.is_ascii_digit() => {
                let idx = (c as u8 - b'0') as usize;
                if let Some(out_info) = self.playback_devices.get(idx) {
                    self.apply_device_switch(out_info.clone(), audio);
                }
            }
            _ => {}
        }
    }

    fn apply_device_switch(&mut self, out_info: DeviceInfo, audio: &mut AudioEngine) {
        if let Some(in_info) = self.chosen_input_device.take() {
            let in_dev = get_input_device_by_name(&self.host, &in_info.name);
            let out_dev = get_output_device_by_name(&self.host, &out_info.name);

            match (in_dev, out_dev) {
                (Some(in_d), Some(out_d)) => match audio.start(&in_d, &out_d) {
                    Ok(_) => {
                        self.status_message = Some(format!(
                            "Device berhasil diubah! [{}] -> [{}]",
                            in_info.name, out_info.name
                        ));
                    }
                    Err(err) => {
                        self.status_message = Some(format!("Gagal ganti audio stream: {}", err));
                    }
                },
                (None, _) => {
                    self.status_message =
                        Some(format!("Input device tidak ditemukan: {}", in_info.name));
                }
                (_, None) => {
                    self.status_message =
                        Some(format!("Output device tidak ditemukan: {}", out_info.name));
                }
            }
        }
        self.device_stage = DeviceSelectStage::None;
    }

    fn adjust_param(&self, dir: f32) {
        match self.selected_param {
            SelectedParam::CleanBright => {
                let cur = load_f32(&self.params.clean_bright);
                store_f32(
                    &self.params.clean_bright,
                    (cur + dir * 0.05).clamp(0.0, 1.0),
                );
            }
            SelectedParam::CleanWarmth => {
                let cur = load_f32(&self.params.clean_warmth);
                store_f32(
                    &self.params.clean_warmth,
                    (cur + dir * 0.05).clamp(0.0, 1.0),
                );
            }
            SelectedParam::CrunchDrive => {
                let cur = load_f32(&self.params.crunch_drive);
                store_f32(
                    &self.params.crunch_drive,
                    (cur + dir * 0.05).clamp(0.0, 1.0),
                );
            }
            SelectedParam::CrunchTone => {
                let cur = load_f32(&self.params.crunch_tone);
                store_f32(&self.params.crunch_tone, (cur + dir * 0.05).clamp(0.0, 1.0));
            }
            SelectedParam::OverdriveDrive => {
                let cur = load_f32(&self.params.overdrive_drive);
                store_f32(
                    &self.params.overdrive_drive,
                    (cur + dir * 0.05).clamp(0.0, 1.0),
                );
            }
            SelectedParam::OverdriveTone => {
                let cur = load_f32(&self.params.overdrive_tone);
                store_f32(
                    &self.params.overdrive_tone,
                    (cur + dir * 0.05).clamp(0.0, 1.0),
                );
            }
            SelectedParam::DistortionDrive => {
                let cur = load_f32(&self.params.distortion_drive);
                store_f32(
                    &self.params.distortion_drive,
                    (cur + dir * 0.05).clamp(0.0, 1.0),
                );
            }
            SelectedParam::DistortionTone => {
                let cur = load_f32(&self.params.distortion_tone);
                store_f32(
                    &self.params.distortion_tone,
                    (cur + dir * 0.05).clamp(0.0, 1.0),
                );
            }
            SelectedParam::WahDepth => {
                let cur = load_f32(&self.params.wah_depth);
                store_f32(&self.params.wah_depth, (cur + dir * 0.05).clamp(0.0, 1.0));
            }
            SelectedParam::WahResonance => {
                let cur = load_f32(&self.params.wah_resonance);
                store_f32(
                    &self.params.wah_resonance,
                    (cur + dir * 0.05).clamp(0.0, 1.0),
                );
            }
            SelectedParam::DelayTime => {
                let cur = load_f32(&self.params.delay_time_ms);
                store_f32(
                    &self.params.delay_time_ms,
                    (cur + dir * 20.0).clamp(10.0, 1000.0),
                );
            }
            SelectedParam::DelayFeedback => {
                let cur = load_f32(&self.params.delay_feedback);
                store_f32(
                    &self.params.delay_feedback,
                    (cur + dir * 0.05).clamp(0.0, 0.90),
                );
            }
            SelectedParam::DelayMix => {
                let cur = load_f32(&self.params.delay_mix);
                store_f32(&self.params.delay_mix, (cur + dir * 0.05).clamp(0.0, 1.0));
            }
        }
    }

    fn render_dashboard(&self, out: &mut std::io::Stdout, audio: &AudioEngine) -> Result<()> {
        let is_running = audio.is_running.load(Ordering::Relaxed);
        let bypass = self.params.global_bypass.load(Ordering::Relaxed);
        let mute = self.params.mute.load(Ordering::Relaxed);

        let status_str = if mute {
            "[MUTED]"
        } else if bypass {
            "[BYPASS]"
        } else if is_running {
            "[RUNNING]"
        } else {
            "[STOPPED]"
        };

        let status_color = if mute || bypass {
            Color::Yellow
        } else if is_running {
            Color::Green
        } else {
            Color::Red
        };

        // Header
        execute!(
            out,
            SetForegroundColor(Color::Cyan),
            Print("=========================================================\n"),
            Print("                     GUITARFX RUST                       \n"),
            Print("=========================================================\n\n"),
            ResetColor,
            Print(" STATUS : "),
            SetForegroundColor(status_color),
            Print(format!("{}\n\n", status_str)),
            ResetColor,
            Print(format!(" INPUT  : {}\n", audio.input_device_name)),
            Print(format!(" OUTPUT : {}\n\n", audio.output_device_name)),
            Print(format!(" SAMPLE RATE : {} Hz\n", audio.sample_rate)),
            Print(format!(
                " BUFFER SIZE : {} frames (Ultra-Low Latency ~5.3 ms)\n\n",
                audio.buffer_size
            )),
            SetForegroundColor(Color::Cyan),
            Print("=========================================================\n"),
            Print(" AUDIO METERS\n"),
            Print("=========================================================\n"),
            ResetColor
        )?;

        // Level Meters
        let in_peak = load_f32(&self.params.input_peak);
        let out_peak = load_f32(&self.params.output_peak);
        let in_db = if in_peak > 1e-5 {
            20.0 * in_peak.log10()
        } else {
            -60.0
        };
        let out_db = if out_peak > 1e-5 {
            20.0 * out_peak.log10()
        } else {
            -60.0
        };

        let in_bar = make_meter_bar(in_peak);
        let out_bar = make_meter_bar(out_peak);

        execute!(
            out,
            Print(format!(" INPUT  [{}] {:>5.1} dB\n", in_bar, in_db)),
            Print(format!(" OUTPUT [{}] {:>5.1} dB\n\n", out_bar, out_db)),
            SetForegroundColor(Color::Cyan),
            Print("=========================================================\n"),
            Print(" EFFECTS CHAIN\n"),
            Print("=========================================================\n"),
            ResetColor
        )?;

        // Effects Table
        let clean_on = self.params.clean_enabled.load(Ordering::Relaxed);
        let clean_b = (load_f32(&self.params.clean_bright) * 100.0) as u32;
        let clean_w = (load_f32(&self.params.clean_warmth) * 100.0) as u32;

        let crunch_on = self.params.crunch_enabled.load(Ordering::Relaxed);
        let crunch_d = (load_f32(&self.params.crunch_drive) * 100.0) as u32;
        let crunch_t = (load_f32(&self.params.crunch_tone) * 100.0) as u32;

        let od_on = self.params.overdrive_enabled.load(Ordering::Relaxed);
        let od_drive = (load_f32(&self.params.overdrive_drive) * 100.0) as u32;
        let od_tone = (load_f32(&self.params.overdrive_tone) * 100.0) as u32;

        let dist_on = self.params.distortion_enabled.load(Ordering::Relaxed);
        let dist_drive = (load_f32(&self.params.distortion_drive) * 100.0) as u32;
        let dist_tone = (load_f32(&self.params.distortion_tone) * 100.0) as u32;

        let wah_on = self.params.wah_enabled.load(Ordering::Relaxed);
        let wah_dp = (load_f32(&self.params.wah_depth) * 100.0) as u32;
        let wah_res = (load_f32(&self.params.wah_resonance) * 100.0) as u32;

        let del_on = self.params.delay_enabled.load(Ordering::Relaxed);
        let del_time = load_f32(&self.params.delay_time_ms) as u32;
        let del_fb = (load_f32(&self.params.delay_feedback) * 100.0) as u32;
        let del_mix = (load_f32(&self.params.delay_mix) * 100.0) as u32;

        print_effect_row(
            out,
            "[1] Clean Preamp",
            clean_on,
            &format!("Bright: {}% | Warmth: {}%", clean_b, clean_w),
        )?;
        print_effect_row(
            out,
            "[2] Crunch Amp",
            crunch_on,
            &format!("Drive: {}% | Tone: {}%", crunch_d, crunch_t),
        )?;
        print_effect_row(
            out,
            "[3] Overdrive",
            od_on,
            &format!("Drive: {}% | Tone: {}%", od_drive, od_tone),
        )?;
        print_effect_row(
            out,
            "[4] Distortion",
            dist_on,
            &format!("Drive: {}% | Tone: {}%", dist_drive, dist_tone),
        )?;
        print_effect_row(
            out,
            "[5] Auto-Wah",
            wah_on,
            &format!("Depth: {}% | Resonance: {}%", wah_dp, wah_res),
        )?;
        print_effect_row(
            out,
            "[6] Delay",
            del_on,
            &format!(
                "Time: {} ms | FB: {}% | Mix: {}%",
                del_time, del_fb, del_mix
            ),
        )?;

        if let Some(ref msg) = self.status_message {
            execute!(
                out,
                Print("\n"),
                SetForegroundColor(Color::Green),
                Print(format!(" [*] {}\n", msg)),
                ResetColor
            )?;
        }

        execute!(
            out,
            Print("\n"),
            SetForegroundColor(Color::Yellow),
            Print(format!(
                " Active Parameter: [{}] (Use [Tab] to cycle, [+/-] to adjust)\n\n",
                self.selected_param.name()
            )),
            ResetColor,
            SetForegroundColor(Color::Cyan),
            Print("=========================================================\n"),
            Print(" CONTROLS\n"),
            Print("=========================================================\n"),
            ResetColor,
            Print(" [D] Select Devices (2-Step Wizard)\n"),
            Print(
                " [1] Clean   [2] Crunch   [3] Overdrive   [4] Distortion   [5] Wah   [6] Delay\n"
            ),
            Print(" [+] / [-] Adjust Parameter    [Tab] Cycle Active Parameter\n"),
            Print(" [B] Bypass All          [M] Mute Output          [Q] Quit\n"),
            SetForegroundColor(Color::Cyan),
            Print("=========================================================\n"),
            ResetColor
        )?;

        Ok(())
    }

    fn render_select_input(&self, out: &mut std::io::Stdout) -> Result<()> {
        execute!(
            out,
            SetForegroundColor(Color::Yellow),
            Print("=========================================================\n"),
            Print("           [LANGKAH 1/2] PILIH INPUT DEVICE              \n"),
            Print("=========================================================\n\n"),
            ResetColor,
            Print(" Pilih input gitar (Vention USB / Sound Card / B1 / B2):\n"),
            SetForegroundColor(Color::Cyan),
            Print(" -> Tekan [ANGKA 0-9] langsung, atau [PANAH ATAS/BAWAH] + [ENTER]\n\n"),
            ResetColor
        )?;

        if self.capture_devices.is_empty() {
            execute!(
                out,
                SetForegroundColor(Color::Red),
                Print("  (Tidak ada audio input device terdeteksi di sistem)\n\n"),
                ResetColor
            )?;
        } else {
            for (i, d) in self.capture_devices.iter().enumerate() {
                let is_cursor = i == self.selected_input_cursor;
                let pointer = if is_cursor { ">" } else { " " };
                let def = if d.is_default { " (Default)" } else { "" };

                if is_cursor {
                    execute!(
                        out,
                        SetForegroundColor(Color::Green),
                        Print(format!("  {} [{}] {}{}\n", pointer, i, d.name, def)),
                        ResetColor
                    )?;
                } else {
                    execute!(
                        out,
                        Print(format!("  {} [{}] {}{}\n", pointer, i, d.name, def))
                    )?;
                }
            }
        }

        execute!(
            out,
            Print("\n"),
            SetForegroundColor(Color::DarkGrey),
            Print("=========================================================\n"),
            Print(" [ESC] Batal dan kembali ke Dashboard\n"),
            Print("=========================================================\n"),
            ResetColor
        )?;

        Ok(())
    }

    fn render_select_output(&self, out: &mut std::io::Stdout) -> Result<()> {
        let in_name = self
            .chosen_input_device
            .as_ref()
            .map(|d| d.name.as_str())
            .unwrap_or("Unknown");

        execute!(
            out,
            SetForegroundColor(Color::Yellow),
            Print("=========================================================\n"),
            Print("           [LANGKAH 2/2] PILIH OUTPUT DEVICE             \n"),
            Print("=========================================================\n\n"),
            ResetColor,
            SetForegroundColor(Color::Green),
            Print(format!(" Input Terpilih: [{}]\n\n", in_name)),
            ResetColor,
            Print(" Pilih output audio (VoiceMeeter AUX Input / Headphones):\n"),
            SetForegroundColor(Color::Cyan),
            Print(" -> Tekan [ANGKA 0-9] langsung, atau [PANAH ATAS/BAWAH] + [ENTER]\n\n"),
            ResetColor
        )?;

        if self.playback_devices.is_empty() {
            execute!(
                out,
                SetForegroundColor(Color::Red),
                Print("  (Tidak ada audio output device terdeteksi di sistem)\n\n"),
                ResetColor
            )?;
        } else {
            for (i, d) in self.playback_devices.iter().enumerate() {
                let is_cursor = i == self.selected_output_cursor;
                let pointer = if is_cursor { ">" } else { " " };
                let def = if d.is_default { " (Default)" } else { "" };

                if is_cursor {
                    execute!(
                        out,
                        SetForegroundColor(Color::Green),
                        Print(format!("  {} [{}] {}{}\n", pointer, i, d.name, def)),
                        ResetColor
                    )?;
                } else {
                    execute!(
                        out,
                        Print(format!("  {} [{}] {}{}\n", pointer, i, d.name, def))
                    )?;
                }
            }
        }

        execute!(
            out,
            Print("\n"),
            SetForegroundColor(Color::DarkGrey),
            Print("=========================================================\n"),
            Print(" [BACKSPACE] Kembali ke Input  |  [ESC] Batal\n"),
            Print("=========================================================\n"),
            ResetColor
        )?;

        Ok(())
    }
}

fn make_meter_bar(level: f32) -> String {
    let bars = ((level.clamp(0.0, 1.0)) * 14.0) as usize;
    let mut s = String::with_capacity(14);
    for _ in 0..bars {
        s.push('#');
    }
    for _ in bars..14 {
        s.push('-');
    }
    s
}

fn print_effect_row(
    out: &mut std::io::Stdout,
    name: &str,
    enabled: bool,
    params: &str,
) -> Result<()> {
    let (badge, color) = if enabled {
        ("[ON] ", Color::Green)
    } else {
        ("[OFF]", Color::DarkGrey)
    };

    execute!(
        out,
        SetForegroundColor(color),
        Print(format!(" {} ", badge)),
        ResetColor,
        Print(format!("{:<18} {}\n", name, params))
    )?;
    Ok(())
}
