#pragma once
#include "dsp/Effect.hpp"
#include <algorithm>
#include <cmath>
#include <iomanip>
#include <sstream>

namespace guitarfx {

class Distortion : public Effect {
public:
    Distortion() = default;

    const char* get_name() const override { return "Distortion"; }

    void process(float* left, float* right, size_t frames) override {
        if (!is_enabled()) return;

        const float drive = drive_.load(std::memory_order_relaxed);
        const float tone = tone_.load(std::memory_order_relaxed);
        const float level = level_.load(std::memory_order_relaxed);

        const float gain = 2.0f + drive * 40.0f;
        // Tone cutoff varies from 1000 Hz to 6500 Hz
        const float tone_cutoff = 1000.0f + tone * 5500.0f;
        const float alpha = std::clamp(2.0f * 3.14159265f * tone_cutoff / sample_rate_, 0.01f, 0.99f);

        for (size_t i = 0; i < frames; ++i) {
            left[i] = process_channel(left[i], gain, alpha, tone_state_l_, pre_state_l_);
            right[i] = process_channel(right[i], gain, alpha, tone_state_r_, pre_state_r_);
            left[i] *= level;
            right[i] *= level;
        }
    }

    void reset() override {
        tone_state_l_ = 0.0f;
        tone_state_r_ = 0.0f;
        pre_state_l_ = 0.0f;
        pre_state_r_ = 0.0f;
    }

    void set_drive(float d) { drive_.store(std::clamp(d, 0.0f, 1.0f), std::memory_order_relaxed); }
    float get_drive() const { return drive_.load(std::memory_order_relaxed); }

    void set_tone(float t) { tone_.store(std::clamp(t, 0.0f, 1.0f), std::memory_order_relaxed); }
    float get_tone() const { return tone_.load(std::memory_order_relaxed); }

    void set_level(float l) { level_.store(std::clamp(l, 0.0f, 2.0f), std::memory_order_relaxed); }
    float get_level() const { return level_.load(std::memory_order_relaxed); }

    std::string get_param_summary() const override {
        std::ostringstream ss;
        ss << "Drive: " << static_cast<int>(get_drive() * 100.0f) << "%";
        return ss.str();
    }

private:
    float process_channel(float in, float gain, float alpha, float& tone_state, float& pre_state) {
        // High-pass pre-emphasis at ~720 Hz to tighten low end before clipping
        const float pre_alpha = 0.12f;
        pre_state += pre_alpha * (in - pre_state);
        pre_state = flush_denormal(pre_state);
        float tightened = in - pre_state;

        float x = tightened * gain;
        // Asymmetric diode clipping model (1N4148 back-to-back diode pair)
        float clipped;
        if (x > 0.0f) {
            clipped = 1.0f - std::exp(-2.5f * x);
        } else {
            clipped = -1.0f + std::exp(1.8f * x);
        }

        // Tone filter
        tone_state += alpha * (clipped - tone_state);
        tone_state = flush_denormal(tone_state);
        return tone_state;
    }

    std::atomic<float> drive_{0.50f};
    std::atomic<float> tone_{0.50f};
    std::atomic<float> level_{0.70f};
    float tone_state_l_{0.0f};
    float tone_state_r_{0.0f};
    float pre_state_l_{0.0f};
    float pre_state_r_{0.0f};
};

} // namespace guitarfx
