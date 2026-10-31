fn print_help() {
    println!("GuitarFX v0.1.0 - Lightweight Terminal Guitar Effects Engine");
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
    let capture_devices = guitarfx::audio::query_capture_devices(&host);
    if capture_devices.is_empty() {
        println!("  (No capture devices found)");
    } else {
        for dev in &capture_devices {
            let default_mark = if dev.is_default { " [DEFAULT]" } else { "" };
            println!("  [{}] {}{}", dev.index, dev.name, default_mark);
        }
    }

    println!("\n=== Playback Devices (Output) ===");
    let playback_devices = guitarfx::audio::query_playback_devices(&host);
    if playback_devices.is_empty() {
        println!("  (No playback devices found)");
    } else {
        for dev in &playback_devices {
            let default_mark = if dev.is_default { " [DEFAULT]" } else { "" };
            println!("  [{}] {}{}", dev.index, dev.name, default_mark);
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(String::as_str) {
        None => {
            println!("GuitarFX v0.1.0 - Lightweight Terminal Guitar Effects Engine");
            println!("Run with --help for options or --list-devices to view audio devices.");
        }
        Some("-v" | "--version") => {
            print_version();
        }
        Some("-h" | "--help") => {
            print_help();
        }
        Some("-l" | "--list-devices") => {
            list_devices();
        }
        Some(unknown) => {
            eprintln!("Unknown argument: {}", unknown);
            print_help();
            std::process::exit(1);
        }
    }
}
