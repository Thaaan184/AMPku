#pragma once
#include "audio/AudioEngine.hpp"
#include "core/PresetManager.hpp"
#include "core/ConfigManager.hpp"
#include <atomic>

namespace guitarfx {

class TerminalUI {
public:
    TerminalUI(AudioEngine& engine, DSPChain& chain, PresetManager& presets, ConfigManager& config);
    ~TerminalUI();

    void run();
    void request_quit();

private:
    AudioEngine& engine_;
    DSPChain& chain_;
    PresetManager& presets_;
    ConfigManager& config_;

    std::atomic<bool> should_quit_{false};
};

} // namespace guitarfx
