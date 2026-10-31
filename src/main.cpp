#include "audio/AudioEngine.hpp"
#include "dsp/DSPChain.hpp"
#include "core/PresetManager.hpp"
#include "core/ConfigManager.hpp"
#include "ui/TerminalUI.hpp"
#include <iostream>
#include <string>

int main(int argc, char* argv[]) {
    // 1. Parse CLI arguments
    for (int i = 1; i < argc; ++i) {
        std::string arg = argv[i];
        if (arg == "--version" || arg == "-v") {
            std::cout << "GuitarFX v0.1.0 (Standalone C++20 Terminal DSP Engine)\n";
            return 0;
        }
        if (arg == "--help" || arg == "-h") {
            std::cout << "GuitarFX - Lightweight Terminal Guitar Effects Engine\n\n"
                      << "Usage:\n"
                      << "  guitarfx.exe                 Launch interactive terminal effects processor\n"
                      << "  guitarfx.exe --list-devices  Enumerate all audio input and output devices\n"
                      << "  guitarfx.exe --version       Show software version\n"
                      << "  guitarfx.exe --help          Show this help message\n\n"
                      << "Controls:\n"
                      << "  [D] Audio Devices     [E] Effect Chain\n"
                      << "  [P] Presets           [B] Global Bypass\n"
                      << "  [M] Output Mute       [Q] Quit\n";
            return 0;
        }
        if (arg == "--list-devices") {
            guitarfx::AudioDeviceManager mgr;
            mgr.refresh_devices();
            std::cout << "\n=== CAPTURE INPUT DEVICES ===\n";
            for (const auto& dev : mgr.get_capture_devices()) {
                std::cout << "  [" << dev.index << "] " << dev.name << (dev.is_default ? " (Default)" : "") << "\n";
            }
            std::cout << "\n=== PLAYBACK OUTPUT DEVICES ===\n";
            for (const auto& dev : mgr.get_playback_devices()) {
                std::cout << "  [" << dev.index << "] " << dev.name << (dev.is_default ? " (Default)" : "") << "\n";
            }
            std::cout << "\n";
            return 0;
        }
    }

    // 2. Initialize Core Systems
    guitarfx::DSPChain dsp_chain;
    guitarfx::PresetManager preset_mgr("presets");
    guitarfx::ConfigManager config_mgr("config/default.json");

    guitarfx::AudioEngine audio_engine(dsp_chain);

    // 3. Load or Configure Default Settings
    guitarfx::AudioConfig audio_cfg;
    if (!config_mgr.load(audio_cfg)) {
        // Defaults: 48 kHz, 256 samples buffer
        audio_cfg.sample_rate = 48000;
        audio_cfg.buffer_size = 256;

        // Auto-detect Vention USB or VoiceMeeter
        auto& mgr = audio_engine.get_device_manager();
        mgr.refresh_devices();

        int vention_idx = mgr.find_capture_device_by_name("vention");
        if (vention_idx >= 0) {
            audio_cfg.capture_device_index = vention_idx;
        } else {
            int vm_b1 = mgr.find_capture_device_by_name("voicemeeter out b1");
            if (vm_b1 >= 0) audio_cfg.capture_device_index = vm_b1;
        }

        int vm_aux_idx = mgr.find_playback_device_by_name("voicemeeter aux input");
        if (vm_aux_idx >= 0) {
            audio_cfg.playback_device_index = vm_aux_idx;
        }
    }

    // Apply audio config and load default preset (crunch)
    audio_engine.apply_config(audio_cfg);
    preset_mgr.load_preset("crunch", dsp_chain);

    // 4. Start Real-Time Audio Engine
    if (!audio_engine.start()) {
        std::cerr << "[!] Warning: Could not start audio stream automatically with current devices.\n"
                  << "    You can configure input and output devices inside the [D] Devices menu.\n";
    }

    // 5. Run Interactive Terminal UI
    guitarfx::TerminalUI ui(audio_engine, dsp_chain, preset_mgr, config_mgr);
    ui.run();

    // 6. Graceful Shutdown
    audio_engine.stop();
    config_mgr.save(audio_engine.get_config());

    std::cout << "\n[OK] GuitarFX closed cleanly.\n";
    return 0;
}
