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
use crate::devices::{get_capture_devices, get_playback_devices, DeviceInfo};
use crate::effects::{load_f32, store_f32, EffectParams};

pub enum SelectedParam {
    OverdriveDrive,
    OverdriveTone,
    DistortionDrive,
    DistortionTone,
    DelayTime,
    DelayFeedback,
    DelayMix,
}

impl SelectedParam {
    pub fn next(&self) -> Self {
        match self {
            Self::OverdriveDrive => Self::OverdriveTone,
            Self::OverdriveTone => Self::DistortionDrive,
            Self::DistortionDrive => Self::DistortionTone,
            Self::DistortionTone => Self::DelayTime,
            Self::DelayTime => Self::DelayFeedback,
            Self::DelayFeedback => Self::DelayMix,
            Self::DelayMix => Self::OverdriveDrive,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::OverdriveDrive => "Overdrive Drive",
            Self::OverdriveTone => "Overdrive Tone",
            Self::DistortionDrive => "Distortion Drive",
            Self::DistortionTone => "Distortion Tone",
            Self::DelayTime => "Delay Time",
            Self::DelayFeedback => "Delay Feedback",
            Self::DelayMix => "Delay Mix",
        }
    }
}

pub struct TerminalApp {
    pub params: Arc<EffectParams>,
    pub selected_param: SelectedParam,
    pub selecting_device: bool,
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
            selecting_device: false,
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
            // 1. Render Dashboard
            execute!(out, cursor::MoveTo(0, 0), Clear(ClearType::All))?;
            if self.selecting_device {
                self.render_device_selection(&mut out)?;
            } else {
                self.render_dashboard(&mut out, audio)?;
            }
            out.flush()?;

            // 2. Poll keyboard events at ~30 FPS (33ms)
            if event::poll(Duration::from_millis(33))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        if self.selecting_device {
                            if self.handle_device_key(key.code, audio)? {
                                self.selecting_device = false;
                            }
                        } else {
                            match key.code {
                                KeyCode::Char('q') | KeyCode::Char('Q') => break,
                                KeyCode::Char('d') | KeyCode::Char('D') => {
                                    self.capture_devices = get_capture_devices(&self.host);
                                    self.playback_devices = get_playback_devices(&self.host);
                                    self.selecting_device = true;
                                }
                                KeyCode::Char('1') => {
                                    let cur = self.params.overdrive_enabled.load(Ordering::Relaxed);
                                    self.params.overdrive_enabled.store(!cur, Ordering::Relaxed);
                                }
                                KeyCode::Char('2') => {
                                    let cur =
                                        self.params.distortion_enabled.load(Ordering::Relaxed);
                                    self.params
                                        .distortion_enabled
                                        .store(!cur, Ordering::Relaxed);
                                }
                                KeyCode::Char('3') => {
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
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn adjust_param(&self, dir: f32) {
        match self.selected_param {
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
            Print("=========================================\n"),
            Print("             GUITARFX RUST               \n"),
            Print("=========================================\n\n"),
            ResetColor,
            Print(" STATUS : "),
            SetForegroundColor(status_color),
            Print(format!("{}\n\n", status_str)),
            ResetColor,
            Print(format!(" INPUT  : {}\n", audio.input_device_name)),
            Print(format!(" OUTPUT : {}\n\n", audio.output_device_name)),
            Print(format!(" SAMPLE RATE : {} Hz\n", audio.sample_rate)),
            Print(format!(" BUFFER SIZE : {} frames\n\n", audio.buffer_size)),
            SetForegroundColor(Color::Cyan),
            Print("=========================================\n"),
            Print(" AUDIO METERS\n"),
            Print("=========================================\n"),
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
            Print("=========================================\n"),
            Print(" EFFECTS\n"),
            Print("=========================================\n"),
            ResetColor
        )?;

        // Effects Table
        let od_on = self.params.overdrive_enabled.load(Ordering::Relaxed);
        let od_drive = (load_f32(&self.params.overdrive_drive) * 100.0) as u32;
        let od_tone = (load_f32(&self.params.overdrive_tone) * 100.0) as u32;

        let dist_on = self.params.distortion_enabled.load(Ordering::Relaxed);
        let dist_drive = (load_f32(&self.params.distortion_drive) * 100.0) as u32;
        let dist_tone = (load_f32(&self.params.distortion_tone) * 100.0) as u32;

        let del_on = self.params.delay_enabled.load(Ordering::Relaxed);
        let del_time = load_f32(&self.params.delay_time_ms) as u32;
        let del_fb = (load_f32(&self.params.delay_feedback) * 100.0) as u32;
        let del_mix = (load_f32(&self.params.delay_mix) * 100.0) as u32;

        print_effect_row(
            out,
            "[1] Overdrive",
            od_on,
            &format!("Drive: {}% | Tone: {}%", od_drive, od_tone),
        )?;
        print_effect_row(
            out,
            "[2] Distortion",
            dist_on,
            &format!("Drive: {}% | Tone: {}%", dist_drive, dist_tone),
        )?;
        print_effect_row(
            out,
            "[3] Delay",
            del_on,
            &format!(
                "Time: {} ms | FB: {}% | Mix: {}%",
                del_time, del_fb, del_mix
            ),
        )?;

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
            Print("=========================================\n"),
            Print(" CONTROLS\n"),
            Print("=========================================\n"),
            ResetColor,
            Print(" [D] Select Device\n"),
            Print(" [1] Toggle Overdrive    [2] Toggle Distortion    [3] Toggle Delay\n"),
            Print(" [+] / [-] Adjust Parameter    [Tab] Cycle Active Parameter\n"),
            Print(" [B] Bypass All          [M] Mute Output          [Q] Quit\n"),
            SetForegroundColor(Color::Cyan),
            Print("=========================================\n"),
            ResetColor
        )?;

        Ok(())
    }

    fn render_device_selection(&self, out: &mut std::io::Stdout) -> Result<()> {
        execute!(
            out,
            SetForegroundColor(Color::Yellow),
            Print("=========================================\n"),
            Print("        SELECT AUDIO DEVICES             \n"),
            Print("=========================================\n\n"),
            ResetColor,
            SetForegroundColor(Color::Green),
            Print("INPUT DEVICES (Vention / Guitar):\n"),
            ResetColor
        )?;

        for (i, d) in self.capture_devices.iter().enumerate() {
            let def = if d.is_default { " (Default)" } else { "" };
            execute!(out, Print(format!("  [{}] {}{}\n", i, d.name, def)))?;
        }

        execute!(
            out,
            Print("\n"),
            SetForegroundColor(Color::Green),
            Print("OUTPUT DEVICES (VoiceMeeter AUX / Headphones):\n"),
            ResetColor
        )?;

        for (i, d) in self.playback_devices.iter().enumerate() {
            let def = if d.is_default { " (Default)" } else { "" };
            execute!(out, Print(format!("  [{}] {}{}\n", i, d.name, def)))?;
        }

        execute!(
            out,
            Print("\n"),
            SetForegroundColor(Color::Cyan),
            Print("Press [ESC] to cancel and return to dashboard.\n"),
            ResetColor
        )?;

        Ok(())
    }

    fn handle_device_key(&mut self, code: KeyCode, _audio: &mut AudioEngine) -> Result<bool> {
        match code {
            KeyCode::Esc => Ok(true),
            _ => Ok(false),
        }
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
        Print(format!("{:<16} {}\n", name, params))
    )?;
    Ok(())
}
