#pragma once
#include "dsp/DSPChain.hpp"
#include <string>
#include <vector>
#include <filesystem>

namespace guitarfx {

class PresetManager {
public:
    PresetManager(std::string presets_dir = "presets");

    void ensure_factory_presets();
    std::vector<std::string> list_presets() const;

    bool load_preset(const std::string& name_or_path, DSPChain& chain);
    bool save_preset(const std::string& name, const DSPChain& chain, const std::string& description = "");

    std::string get_current_preset_name() const { return current_preset_name_; }

private:
    std::string presets_dir_;
    std::string current_preset_name_{"crunch"};
};

} // namespace guitarfx
