#include "core/ConfigManager.hpp"
#include "nlohmann/json.hpp"
#include <fstream>
#include <filesystem>

using json = nlohmann::json;

namespace guitarfx {

ConfigManager::ConfigManager(std::string config_path)
    : config_path_(std::move(config_path)) {}

bool ConfigManager::load(AudioConfig& audio_cfg) {
    if (!std::filesystem::exists(config_path_)) {
        return false;
    }

    std::ifstream f(config_path_);
    if (!f.is_open()) return false;

    try {
        json j;
        f >> j;
        if (j.contains("audio")) {
            const auto& a = j["audio"];
            audio_cfg.sample_rate = a.value("sample_rate", 48000);
            audio_cfg.buffer_size = a.value("buffer_size", 256);
            audio_cfg.capture_device_name = a.value("capture_device", "");
            audio_cfg.playback_device_name = a.value("playback_device", "");
        }
        return true;
    } catch (...) {
        return false;
    }
}

bool ConfigManager::save(const AudioConfig& audio_cfg) {
    std::filesystem::path p(config_path_);
    if (p.has_parent_path()) {
        std::filesystem::create_directories(p.parent_path());
    }

    json j = {
        {"audio", {
            {"sample_rate", audio_cfg.sample_rate},
            {"buffer_size", audio_cfg.buffer_size},
            {"capture_device", audio_cfg.capture_device_name},
            {"playback_device", audio_cfg.playback_device_name}
        }}
    };

    std::ofstream f(config_path_);
    if (!f.is_open()) return false;
    f << j.dump(2);
    return true;
}

} // namespace guitarfx
