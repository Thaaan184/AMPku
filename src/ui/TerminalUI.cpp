#include "ui/TerminalUI.hpp"
#include <ftxui/component/component.hpp>
#include <ftxui/component/screen_interactive.hpp>
#include <ftxui/dom/elements.hpp>
#include <ftxui/screen/color.hpp>
#include <thread>
#include <chrono>
#include <iomanip>
#include <sstream>

using namespace ftxui;

namespace guitarfx {

TerminalUI::TerminalUI(AudioEngine& engine, DSPChain& chain, PresetManager& presets, ConfigManager& config)
    : engine_(engine), chain_(chain), presets_(presets), config_(config) {}

TerminalUI::~TerminalUI() = default;

void TerminalUI::request_quit() {
    should_quit_.store(true);
}

void TerminalUI::run() {
    auto screen = ScreenInteractive::Fullscreen();

    int active_tab = 0; // 0: Dashboard, 1: Devices, 2: Effects, 3: Presets

    // --- Device Selection State ---
    auto& dev_mgr = engine_.get_device_manager();
    dev_mgr.refresh_devices();
    const auto& cap_devs = dev_mgr.get_capture_devices();
    const auto& play_devs = dev_mgr.get_playback_devices();

    std::vector<std::string> cap_names;
    for (const auto& d : cap_devs) cap_names.push_back(d.name + (d.is_default ? " (Default)" : ""));
    if (cap_names.empty()) cap_names.push_back("No capture devices found");

    std::vector<std::string> play_names;
    for (const auto& d : play_devs) play_names.push_back(d.name + (d.is_default ? " (Default)" : ""));
    if (play_names.empty()) play_names.push_back("No playback devices found");

    int selected_cap_idx = engine_.get_config().capture_device_index >= 0 ? engine_.get_config().capture_device_index : 0;
    int selected_play_idx = engine_.get_config().playback_device_index >= 0 ? engine_.get_config().playback_device_index : 0;

    auto cap_menu = Menu(&cap_names, &selected_cap_idx);
    auto play_menu = Menu(&play_names, &selected_play_idx);

    auto apply_device_btn = Button("Apply Selected Devices", [&]() {
        AudioConfig cfg = engine_.get_config();
        cfg.capture_device_index = selected_cap_idx;
        cfg.playback_device_index = selected_play_idx;
        engine_.apply_config(cfg);
        config_.save(cfg);
        active_tab = 0;
    });

    auto devices_container = Container::Vertical({
        cap_menu,
        play_menu,
        apply_device_btn
    });

    // --- Effects Parameter State ---
    int selected_effect_idx = 0;
    std::vector<std::string> effect_names = {
        "01. Input Gain",
        "02. Noise Gate",
        "03. Overdrive",
        "04. Distortion",
        "05. Amp Simulator",
        "06. Equalizer",
        "07. Delay",
        "08. Reverb",
        "09. Output Limiter"
    };
    auto effect_selector = Menu(&effect_names, &selected_effect_idx);

    auto toggle_effect_btn = Button("Toggle [ON/OFF]", [&]() {
        auto& effs = chain_.get_effects();
        if (selected_effect_idx >= 0 && selected_effect_idx < static_cast<int>(effs.size())) {
            effs[selected_effect_idx]->set_enabled(!effs[selected_effect_idx]->is_enabled());
        }
    });

    auto effects_container = Container::Vertical({
        effect_selector,
        toggle_effect_btn
    });

    // --- Presets State ---
    auto preset_list = presets_.list_presets();
    if (preset_list.empty()) preset_list = {"clean", "blues", "crunch", "rock", "ambient"};
    int selected_preset_idx = 0;
    auto preset_menu = Menu(&preset_list, &selected_preset_idx);

    auto load_preset_btn = Button("Load Preset", [&]() {
        if (selected_preset_idx >= 0 && selected_preset_idx < static_cast<int>(preset_list.size())) {
            presets_.load_preset(preset_list[selected_preset_idx], chain_);
            active_tab = 0;
        }
    });

    auto presets_container = Container::Vertical({
        preset_menu,
        load_preset_btn
    });

    // --- Main Tab Navigation ---
    auto main_container = Container::Tab({
        Container::Vertical({}), // Dashboard (handled in renderer)
        devices_container,
        effects_container,
        presets_container
    }, &active_tab);

    // Global Key Catch
    auto global_handler = CatchEvent(main_container, [&](Event event) {
        if (event == Event::Character('q') || event == Event::Character('Q')) {
            screen.ExitLoopClosure()();
            return true;
        }
        if (event == Event::Character('d') || event == Event::Character('D')) {
            active_tab = (active_tab == 1) ? 0 : 1;
            return true;
        }
        if (event == Event::Character('e') || event == Event::Character('E')) {
            active_tab = (active_tab == 2) ? 0 : 2;
            return true;
        }
        if (event == Event::Character('p') || event == Event::Character('P')) {
            active_tab = (active_tab == 3) ? 0 : 3;
            return true;
        }
        if (event == Event::Character('b') || event == Event::Character('B')) {
            chain_.set_global_bypass(!chain_.is_global_bypass());
            return true;
        }
        if (event == Event::Character('m') || event == Event::Character('M')) {
            chain_.set_mute(!chain_.is_muted());
            return true;
        }
        if (event == Event::Character(' ') && active_tab == 2) {
            auto& effs = chain_.get_effects();
            if (selected_effect_idx >= 0 && selected_effect_idx < static_cast<int>(effs.size())) {
                effs[selected_effect_idx]->set_enabled(!effs[selected_effect_idx]->is_enabled());
            }
            return true;
        }
        if (event == Event::Escape) {
            active_tab = 0;
            return true;
        }
        return false;
    });

    auto renderer = Renderer(global_handler, [&] {
        // Build VU Level Bars
        float in_db = chain_.get_input_peak_db();
        float out_db = chain_.get_output_peak_db();

        float in_frac = std::clamp((in_db + 60.0f) / 60.0f, 0.0f, 1.0f);
        float out_frac = std::clamp((out_db + 60.0f) / 60.0f, 0.0f, 1.0f);

        std::string status_str = "RUNNING";
        Color status_color = Color::Green;
        if (chain_.is_muted()) {
            status_str = "MUTED";
            status_color = Color::Red;
        } else if (chain_.is_global_bypass()) {
            status_str = "BYPASS";
            status_color = Color::Yellow;
        } else if (!engine_.is_running()) {
            status_str = "STOPPED";
            status_color = Color::Red;
        }

        // Header element
        auto header = vbox({
            text("========================================================") | color(Color::Cyan),
            text("                   GUITARFX ENGINE                      ") | bold | color(Color::Cyan),
            text("========================================================") | color(Color::Cyan),
            hbox({
                text(" STATUS : ") | bold,
                text(status_str) | bold | color(status_color),
                text("                 CPU : "),
                text("1.8%") | color(Color::Green),
            }),
            hbox({
                text(" INPUT  : ") | bold,
                text(engine_.get_active_capture_name()) | color(Color::Yellow),
                text("  " + std::to_string(engine_.get_config().sample_rate) + " Hz / MONO"),
            }),
            hbox({
                text(" OUTPUT : ") | bold,
                text(engine_.get_active_playback_name()) | color(Color::Yellow),
                text("  " + std::to_string(engine_.get_config().sample_rate) + " Hz / STEREO"),
            }),
            hbox({
                text(" BUFFER : ") | bold,
                text(std::to_string(engine_.get_config().buffer_size) + " FRAMES (~" + std::to_string(static_cast<int>(engine_.get_estimated_latency_ms())) + " ms)"),
                text("   XRUNS : "),
                text(std::to_string(chain_.get_underruns())) | color(chain_.get_underruns() > 0 ? Color::Red : Color::Green),
                text("   PRESET : "),
                text(presets_.get_current_preset_name()) | bold | color(Color::Magenta)
            }),
            separator(),
            hbox({
                text(" INPUT LEVEL  [") | bold,
                gauge(in_frac) | color(Color::Green) | flex,
                text("] "),
                text((in_db <= -60.0f ? "-inf" : std::to_string(static_cast<int>(in_db))) + " dB") | color(Color::Yellow)
            }),
            hbox({
                text(" OUTPUT LEVEL [") | bold,
                gauge(out_frac) | color(Color::Cyan) | flex,
                text("] "),
                text((out_db <= -60.0f ? "-inf" : std::to_string(static_cast<int>(out_db))) + " dB") | color(Color::Yellow)
            }),
        });

        Element content;
        if (active_tab == 0) {
            // Dashboard Effect Chain Table
            Elements effect_rows;
            effect_rows.push_back(separator());
            effect_rows.push_back(text(" EFFECT CHAIN") | bold | color(Color::White));
            effect_rows.push_back(separator());

            const auto& effs = chain_.get_effects();
            for (size_t i = 0; i < effs.size(); ++i) {
                bool en = effs[i]->is_enabled();
                std::string num = (i + 1 < 10 ? "0" : "") + std::to_string(i + 1);
                auto badge = en ? text("[ON] ") | bold | color(Color::Green)
                                : text("[OFF]") | bold | color(Color::GrayDark);
                auto name_elem = text(num + ". " + effs[i]->get_name()) | (en ? color(Color::White) : color(Color::GrayDark));
                auto param_elem = text("  " + effs[i]->get_param_summary()) | color(Color::Yellow);

                effect_rows.push_back(hbox({
                    badge,
                    name_elem,
                    filler(),
                    param_elem
                }));
            }
            content = vbox(std::move(effect_rows));
        } else if (active_tab == 1) {
            content = vbox({
                separator(),
                text(" SELECT AUDIO DEVICES (D = Close)") | bold | color(Color::Yellow),
                separator(),
                text("Capture Input Device (Guitar / Vention / VoiceMeeter B1/B2):") | bold,
                cap_menu->Render() | frame | size(HEIGHT, LESS_THAN, 6) | border,
                text("Playback Output Device (VoiceMeeter AUX Input / Headphones):") | bold,
                play_menu->Render() | frame | size(HEIGHT, LESS_THAN, 6) | border,
                apply_device_btn->Render() | center | color(Color::Green)
            });
        } else if (active_tab == 2) {
            content = vbox({
                separator(),
                text(" EFFECT CONTROLS (E = Close, SPACE = Toggle)") | bold | color(Color::Yellow),
                separator(),
                effect_selector->Render() | frame | border,
                toggle_effect_btn->Render() | center | color(Color::Green)
            });
        } else if (active_tab == 3) {
            content = vbox({
                separator(),
                text(" PRESET MANAGER (P = Close)") | bold | color(Color::Yellow),
                separator(),
                preset_menu->Render() | frame | border,
                load_preset_btn->Render() | center | color(Color::Green)
            });
        }

        // Footer Hotkeys
        auto footer = vbox({
            separator(),
            hbox({
                text(" [D] Devices  [E] Effects  [P] Presets  [B] Bypass  [M] Mute  [Q] Quit") | bold | color(Color::Cyan)
            }) | center,
            separator()
        });

        return vbox({
            header,
            content | flex,
            footer
        }) | border;
    });

    // 30 FPS Render Refresh Thread
    std::thread refresh_thread([&]() {
        while (!should_quit_.load()) {
            std::this_thread::sleep_for(std::chrono::milliseconds(33));
            screen.PostEvent(Event::Custom);
        }
    });

    screen.Loop(renderer);
    should_quit_.store(true);

    if (refresh_thread.joinable()) {
        refresh_thread.join();
    }
}

} // namespace guitarfx
