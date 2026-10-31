//! AMPku - Standalone Low-Latency Guitar & Vocal DSP Studio application entrypoint.

use ampku::gui::app::AmpkuApp;
use std::fs::OpenOptions;
use std::io::Write;
use std::panic;
use std::path::PathBuf;

/// Diagnostic logger writing to both stderr and `ampku_boot.log`.
fn log_boot(msg: &str) {
    let timestamp = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(d) => format!("[{}.{:03}s] ", d.as_secs() % 1000, d.subsec_millis()),
        Err(_) => String::from("[init] "),
    };
    let line = format!("{}{}\n", timestamp, msg);
    eprint!("{}", line);

    // Write to ampku_boot.log in current directory and %APPDATA%/AMPku if available
    let log_paths = [
        PathBuf::from("ampku_boot.log"),
        #[cfg(target_os = "windows")]
        std::env::var("APPDATA")
            .map(|a| PathBuf::from(a).join("AMPku").join("ampku_boot.log"))
            .unwrap_or_else(|_| PathBuf::from("ampku_boot.log")),
        #[cfg(not(target_os = "windows"))]
        PathBuf::from("/tmp/ampku_boot.log"),
    ];

    for path in &log_paths {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
            let _ = file.write_all(line.as_bytes());
        }
    }
}

/// Native Windows message box pop-up on panic or fatal error.
#[cfg(target_os = "windows")]
fn show_native_alert(title: &str, message: &str, is_error: bool) {
    use std::ffi::CString;
    #[link(name = "user32")]
    extern "system" {
        fn MessageBoxA(
            hwnd: *mut std::ffi::c_void,
            lpText: *const std::ffi::c_char,
            lpCaption: *const std::ffi::c_char,
            uType: u32,
        ) -> i32;
    }

    let title_c = CString::new(title).unwrap_or_default();
    let message_c = CString::new(message).unwrap_or_default();
    let u_type = if is_error { 0x00000010 } else { 0x00000040 }; // MB_ICONERROR or MB_ICONINFORMATION
    unsafe {
        MessageBoxA(
            std::ptr::null_mut(),
            message_c.as_ptr(),
            title_c.as_ptr(),
            u_type,
        );
    }
}

#[cfg(not(target_os = "windows"))]
fn show_native_alert(title: &str, message: &str, _is_error: bool) {
    eprintln!("[{}] {}", title, message);
}

fn setup_panic_hook() {
    panic::set_hook(Box::new(|info| {
        let panic_msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic payload".to_string()
        };

        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown location".to_string());

        let full_crash_report = format!(
            "AMPku crashed unexpectedly!\n\nLocation: {}\nMessage: {}\n\nPlease check 'ampku_boot.log' for diagnostic details.",
            location, panic_msg
        );

        log_boot(&format!("FATAL PANIC: {}", full_crash_report));

        // Write crash report to ampku_crash.log
        let _ = std::fs::write("ampku_crash.log", &full_crash_report);

        show_native_alert("AMPku - Fatal Crash", &full_crash_report, true);
    }));
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    setup_panic_hook();

    log_boot("=================================================");
    log_boot("AMPku v0.1.0 starting...");
    log_boot("Configuring eframe native window (WGPU DirectX 12/11 + Glow)...");

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 750.0])
            .with_min_inner_size([900.0, 600.0])
            .with_title("AMPku - Low-Latency Guitar & Vocal DSP Studio"),
        default_theme: eframe::Theme::Dark,
        hardware_acceleration: eframe::HardwareAcceleration::Preferred,
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    log_boot("Calling eframe::run_native...");

    let run_result = eframe::run_native(
        "AMPku - Low-Latency Guitar & Vocal DSP Studio",
        native_options,
        Box::new(|cc| {
            log_boot("eframe creation context ready. Initializing AmpkuApp...");
            let app = AmpkuApp::new(cc);
            log_boot("AmpkuApp successfully initialized. Entering main render loop.");
            Box::new(app)
        }),
    );

    if let Err(err) = run_result {
        let err_msg = format!("Failed to launch GUI application:\n\n{}", err);
        log_boot(&format!("FATAL ERROR: {}", err_msg));
        let _ = std::fs::write("ampku_crash.log", &err_msg);
        show_native_alert("AMPku - Launch Error", &err_msg, true);
        return Err(Box::new(err));
    }

    log_boot("AMPku exited cleanly.");
    Ok(())
}
