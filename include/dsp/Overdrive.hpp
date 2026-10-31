#pragma once
#include "dsp/Effect.hpp"
#include <algorithm>
#include <cmath>
#include <iomanip>
#include <sstream>

namespace guitarfx {

class Overdrive : public Effect {
public:
    Overdrive() = default;

    const char* get_name() const override { return "Overdrive"; }

    void process(float* left, float* right, size_t frames) override {
        if (!is_enabled()) return;

        const float drive = drive_.load(std::memory_order_relaxed);
        const float tone = tone_.load(std::memory_order_relaxed);
        const float level = level_.load(std::memory_order_relaxed);

        const float gain = 1.0f + drive * 24.0f;
        // Tone cutoff varies from 800 Hz to 7000 Hz
        const float tone_cutoff = 800.0f + tone * 6200.0f;
        const float alpha = std::clamp(2.0f * 3.14159265f * tone_cutoff / sample_rate_, 0.01f, 0.99f);

        for (size_t i = 0; i < frames; ++i) {
            left[i] = process_channel(left[i], gain, alpha, tone_state_l_);
            right[i] = process_channel(right[i], gain, alpha, tone_state_r_);
            left[i] *= level;
            right[i] *= level;
        }
    }

    void reset() override {
        tone_state_l_ = 0.0f;
        tone_state_r_ = 0.0f;
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
    float process_channel(float in, float gain, float alpha, float& tone_state) {
        float x = in * gain;
        // Soft-clipping via hyperbolic tangent with subtle asymmetrical bias
        float clipped = std::tanh(x + 0.05f * x * x);
        // 1-pole tone filter
        tone_state += alpha * (clipped - tone_state);
        tone_state = flush_denormal(tone_state);
        return tone_state;
    }

    std::atomic<float> drive_{0.35f};
    std::atomic<float> tone_{0.50f};
    std::atomic<float> level_{0.80f};
    float tone_state_l_{0.0f};
    float tone_state_r_{0.0f};
};

} // namespace guitarfx
