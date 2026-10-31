#pragma once
#include <string>
#include <vector>

namespace guitarfx {

struct DeviceInfo {
    int index{-1};
    std::string name;
    std::string id_str;
    bool is_default{false};
};

class AudioDeviceManager {
public:
    AudioDeviceManager();
    ~AudioDeviceManager();

    bool refresh_devices();

    const std::vector<DeviceInfo>& get_capture_devices() const { return capture_devices_; }
    const std::vector<DeviceInfo>& get_playback_devices() const { return playback_devices_; }

    int find_capture_device_by_name(const std::string& name_query) const;
    int find_playback_device_by_name(const std::string& name_query) const;

    void* get_context_ptr() { return context_ptr_; }

private:
    void* context_ptr_{nullptr};
    std::vector<DeviceInfo> capture_devices_;
    std::vector<DeviceInfo> playback_devices_;
};

} // namespace guitarfx
