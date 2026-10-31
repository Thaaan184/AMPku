#include "audio/AudioEngine.hpp"
#include "miniaudio/miniaudio.h"
#include <iostream>
#include <cstring>

namespace guitarfx {

static void audio_data_callback(ma_device* pDevice, void* pOutput, const void* pInput, ma_uint32 frameCount) {
    auto* self = static_cast<AudioEngine*>(pDevice->pUserData);
    if (!self) return;

    auto& dsp = self->get_dsp_chain_ref();
    const float* in_mono = static_cast<const float*>(pInput);
    float* out_stereo = static_cast<float*>(pOutput);

    if (in_mono == nullptr) {
        // Fallback silence on input starvation
        if (out_stereo) {
            std::memset(out_stereo, 0, frameCount * 2 * sizeof(float));
        }
        dsp.record_underrun();
        return;
    }

    if (out_stereo == nullptr) {
        dsp.record_overrun();
        return;
    }

    dsp.process_mono_to_stereo(in_mono, out_stereo, frameCount);
}

AudioEngine::AudioEngine(DSPChain& dsp_chain)
    : dsp_chain_(dsp_chain) {
    device_ptr_ = new ma_device();
}

AudioEngine::~AudioEngine() {
    shutdown();
    if (device_ptr_) {
        delete static_cast<ma_device*>(device_ptr_);
        device_ptr_ = nullptr;
    }
}

bool AudioEngine::init() {
    return apply_config(config_);
}

void AudioEngine::shutdown() {
    stop();
    if (is_initialized_.load()) {
        auto* dev = static_cast<ma_device*>(device_ptr_);
        ma_device_uninit(dev);
        is_initialized_.store(false);
    }
}

bool AudioEngine::start() {
    if (!is_initialized_.load()) {
        if (!init()) return false;
    }

    auto* dev = static_cast<ma_device*>(device_ptr_);
    if (ma_device_start(dev) == MA_SUCCESS) {
        is_running_.store(true);
        return true;
    }
    return false;
}

void AudioEngine::stop() {
    if (is_running_.load()) {
        auto* dev = static_cast<ma_device*>(device_ptr_);
        ma_device_stop(dev);
        is_running_.store(false);
    }
}

bool AudioEngine::apply_config(const AudioConfig& config) {
    stop();
    if (is_initialized_.load()) {
        auto* dev = static_cast<ma_device*>(device_ptr_);
        ma_device_uninit(dev);
        is_initialized_.store(false);
    }

    config_ = config;
    dsp_chain_.set_sample_rate(static_cast<float>(config_.sample_rate));

    auto* ctx = static_cast<ma_context*>(device_manager_.get_context_ptr());
    if (!ctx) return false;

    ma_device_info* pPlaybackInfos = nullptr;
    ma_uint32 playbackCount = 0;
    ma_device_info* pCaptureInfos = nullptr;
    ma_uint32 captureCount = 0;
    ma_context_get_devices(ctx, &pPlaybackInfos, &playbackCount, &pCaptureInfos, &captureCount);

    ma_device_id* pCaptureID = nullptr;
    if (config_.capture_device_index >= 0 && static_cast<ma_uint32>(config_.capture_device_index) < captureCount) {
        pCaptureID = &pCaptureInfos[config_.capture_device_index].id;
        config_.capture_device_name = pCaptureInfos[config_.capture_device_index].name;
    } else {
        config_.capture_device_name = "Default Windows Capture";
    }

    ma_device_id* pPlaybackID = nullptr;
    if (config_.playback_device_index >= 0 && static_cast<ma_uint32>(config_.playback_device_index) < playbackCount) {
        pPlaybackID = &pPlaybackInfos[config_.playback_device_index].id;
        config_.playback_device_name = pPlaybackInfos[config_.playback_device_index].name;
    } else {
        config_.playback_device_name = "Default Windows Playback";
    }

    ma_device_config dev_cfg = ma_device_config_init(ma_device_type_duplex);
    dev_cfg.capture.pDeviceID = pCaptureID;
    dev_cfg.capture.format = ma_format_f32;
    dev_cfg.capture.channels = 1; // Mono guitar input
    dev_cfg.capture.shareMode = ma_share_mode_shared;

    dev_cfg.playback.pDeviceID = pPlaybackID;
    dev_cfg.playback.format = ma_format_f32;
    dev_cfg.playback.channels = 2; // Stereo output
    dev_cfg.playback.shareMode = ma_share_mode_shared;

    dev_cfg.sampleRate = config_.sample_rate;
    dev_cfg.periodSizeInFrames = config_.buffer_size;
    dev_cfg.dataCallback = audio_data_callback;
    dev_cfg.pUserData = this;

    auto* dev = static_cast<ma_device*>(device_ptr_);
    if (ma_device_init(ctx, &dev_cfg, dev) != MA_SUCCESS) {
        return false;
    }

    is_initialized_.store(true);
    return true;
}

float AudioEngine::get_estimated_latency_ms() const {
    if (config_.sample_rate == 0) return 0.0f;
    // Buffer latency (one-way capture + one-way playback)
    float buffer_ms = (static_cast<float>(config_.buffer_size) / static_cast<float>(config_.sample_rate)) * 1000.0f;
    return buffer_ms * 2.0f;
}

std::string AudioEngine::get_active_capture_name() const {
    return config_.capture_device_name.empty() ? "Default Capture" : config_.capture_device_name;
}

std::string AudioEngine::get_active_playback_name() const {
    return config_.playback_device_name.empty() ? "Default Playback" : config_.playback_device_name;
}

// Internal helper for callback access
DSPChain& AudioEngine::get_dsp_chain_ref() {
    return dsp_chain_;
}

} // namespace guitarfx
