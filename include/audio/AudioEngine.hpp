#pragma once
#include "audio/AudioDeviceManager.hpp"
#include "dsp/DSPChain.hpp"
#include <memory>
#include <string>
#include <atomic>

namespace guitarfx {

struct AudioConfig {
    uint32_t sample_rate{48000};
    uint32_t buffer_size{256};
    int capture_device_index{-1};   // -1 = default
    int playback_device_index{-1};  // -1 = default
    std::string capture_device_name;
    std::string playback_device_name;
};

class AudioEngine {
public:
    explicit AudioEngine(DSPChain& dsp_chain);
    ~AudioEngine();

    bool init();
    void shutdown();

    bool start();
    void stop();

    bool apply_config(const AudioConfig& config);
    const AudioConfig& get_config() const { return config_; }

    AudioDeviceManager& get_device_manager() { return device_manager_; }

    bool is_running() const { return is_running_.load(std::memory_order_relaxed); }
    float get_estimated_latency_ms() const;

    std::string get_active_capture_name() const;
    std::string get_active_playback_name() const;

    DSPChain& get_dsp_chain_ref();

private:
    DSPChain& dsp_chain_;
    AudioDeviceManager device_manager_;
    AudioConfig config_;

    void* device_ptr_{nullptr};
    std::atomic<bool> is_running_{false};
    std::atomic<bool> is_initialized_{false};
};

} // namespace guitarfx
