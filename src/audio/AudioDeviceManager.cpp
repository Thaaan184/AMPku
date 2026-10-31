#include "audio/AudioDeviceManager.hpp"
#include "miniaudio/miniaudio.h"
#include <algorithm>
#include <iostream>

namespace guitarfx {

AudioDeviceManager::AudioDeviceManager() {
    auto* ctx = new ma_context();
    ma_context_config ctx_cfg = ma_context_config_init();
    if (ma_context_init(nullptr, 0, &ctx_cfg, ctx) == MA_SUCCESS) {
        context_ptr_ = ctx;
        refresh_devices();
    } else {
        delete ctx;
        context_ptr_ = nullptr;
    }
}

AudioDeviceManager::~AudioDeviceManager() {
    if (context_ptr_) {
        auto* ctx = static_cast<ma_context*>(context_ptr_);
        ma_context_uninit(ctx);
        delete ctx;
        context_ptr_ = nullptr;
    }
}

bool AudioDeviceManager::refresh_devices() {
    if (!context_ptr_) return false;
    auto* ctx = static_cast<ma_context*>(context_ptr_);

    ma_device_info* pPlaybackInfos = nullptr;
    ma_uint32 playbackCount = 0;
    ma_device_info* pCaptureInfos = nullptr;
    ma_uint32 captureCount = 0;

    if (ma_context_get_devices(ctx, &pPlaybackInfos, &playbackCount, &pCaptureInfos, &captureCount) != MA_SUCCESS) {
        return false;
    }

    capture_devices_.clear();
    for (ma_uint32 i = 0; i < captureCount; ++i) {
        DeviceInfo info;
        info.index = static_cast<int>(i);
        info.name = pCaptureInfos[i].name;
        info.is_default = (pCaptureInfos[i].isDefault != 0);
        capture_devices_.push_back(info);
    }

    playback_devices_.clear();
    for (ma_uint32 i = 0; i < playbackCount; ++i) {
        DeviceInfo info;
        info.index = static_cast<int>(i);
        info.name = pPlaybackInfos[i].name;
        info.is_default = (pPlaybackInfos[i].isDefault != 0);
        playback_devices_.push_back(info);
    }

    return true;
}

int AudioDeviceManager::find_capture_device_by_name(const std::string& query) const {
    if (query.empty()) return -1;
    std::string q_lower = query;
    std::transform(q_lower.begin(), q_lower.end(), q_lower.begin(), ::tolower);

    for (const auto& dev : capture_devices_) {
        std::string d_lower = dev.name;
        std::transform(d_lower.begin(), d_lower.end(), d_lower.begin(), ::tolower);
        if (d_lower.find(q_lower) != std::string::npos) {
            return dev.index;
        }
    }
    return -1;
}

int AudioDeviceManager::find_playback_device_by_name(const std::string& query) const {
    if (query.empty()) return -1;
    std::string q_lower = query;
    std::transform(q_lower.begin(), q_lower.end(), q_lower.begin(), ::tolower);

    for (const auto& dev : playback_devices_) {
        std::string d_lower = dev.name;
        std::transform(d_lower.begin(), d_lower.end(), d_lower.begin(), ::tolower);
        if (d_lower.find(q_lower) != std::string::npos) {
            return dev.index;
        }
    }
    return -1;
}

} // namespace guitarfx
