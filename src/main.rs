use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait};
use std::sync::Arc;

use guitarfx::audio::AudioEngine;
use guitarfx::devices::{find_device_by_name, get_capture_devices, get_playback_devices};
use guitarfx::effects::EffectParams;
use guitarfx::terminal::TerminalApp;

fn print_help() {
    println!("GuitarFX v0.1.0 - Lightweight Terminal Guitar Effects Engine in Rust");
    println!("\nUsage: guitarfx [OPTIONS]");
    println!("\nOptions:");
    println!("  -l, --list-devices    List available audio input and output devices");
    println!("  -v, --version         Print version information");
    println!("  -h, --help            Print help information");
}

fn print_version() {
    println!("GuitarFX v0.1.0");
}

fn list_devices() {
    let host = cpal::default_host();
    println!("Audio Host: {:?}", host.id());

    println!("\n=== Capture Devices (Input) ===");
    let capture_devices = get_capture_devices(&host);
    if capture_devices.is_empty() {
        println!("  (No capture devices found)");
    } else {
        for dev in &capture_devices {
            let default_mark = if dev.is_default { " [DEFAULT]" } else { "" };
            println!("  [{}] {}{}", dev.index, dev.name, default_mark);
        }
    }

    println!("\n=== Playback Devices (Output) ===");
    let playback_devices = get_playback_devices(&host);
    if playback_devices.is_empty() {
        println!("  (No playback devices found)");
    } else {
        for dev in &playback_devices {
            let default_mark = if dev.is_default { " [DEFAULT]" } else { "" };
            println!("  [{}] {}{}", dev.index, dev.name, default_mark);
        }
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if let Some(arg) = args.get(1) {
        match arg.as_str() {
            "-v" | "--version" => {
                print_version();
                return Ok(());
            }
            "-h" | "--help" => {
                print_help();
                return Ok(());
            }
            "-l" | "--list-devices" => {
                list_devices();
                return Ok(());
            }
            unknown => {
                eprintln!("Unknown argument: {}", unknown);
                print_help();
                std::process::exit(1);
            }
        }
    }

    // Default: Run Interactive Terminal Application
    let host = cpal::default_host();
    let capture_devices = get_capture_devices(&host);
    let playback_devices = get_playback_devices(&host);

    // 1. Resolve preferred input (prefer Vention USB, fallback to default)
    let in_dev = find_device_by_name(&capture_devices, "vention")
        .or_else(|| capture_devices.iter().find(|d| d.is_default).cloned())
        .or_else(|| capture_devices.first().cloned())
        .and_then(|info| {
            host.input_devices().ok().and_then(|mut devs| {
                devs.find(|d| d.name().map(|n| n == info.name).unwrap_or(false))
            })
        });

    // 2. Resolve preferred output (prefer VoiceMeeter AUX or VoiceMeeter, fallback to default)
    let out_dev = find_device_by_name(&playback_devices, "aux")
        .or_else(|| find_device_by_name(&playback_devices, "voicemeeter"))
        .or_else(|| playback_devices.iter().find(|d| d.is_default).cloned())
        .or_else(|| playback_devices.first().cloned())
        .and_then(|info| {
            host.output_devices().ok().and_then(|mut devs| {
                devs.find(|d| d.name().map(|n| n == info.name).unwrap_or(false))
            })
        });

    let params = Arc::new(EffectParams::default());
    let mut audio = AudioEngine::new(Arc::clone(&params));

    if let (Some(ref in_d), Some(ref out_d)) = (&in_dev, &out_dev) {
        if let Err(err) = audio.start(in_d, out_d) {
            eprintln!(
                "[Warning] Could not start audio stream automatically: {}",
                err
            );
        }
    }

    let mut app = TerminalApp::new(Arc::clone(&params));
    let res = app.run(&mut audio);

    audio.stop();
    res
}
