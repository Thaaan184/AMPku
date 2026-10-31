#pragma once
#include "audio/AudioEngine.hpp"
#include <string>

namespace guitarfx {

class ConfigManager {
public:
    explicit ConfigManager(std::string config_path = "config/default.json");

    bool load(AudioConfig& audio_cfg);
    bool save(const AudioConfig& audio_cfg);

private:
    std::string config_path_;
};

} // namespace guitarfx
