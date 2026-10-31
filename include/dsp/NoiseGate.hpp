#pragma once
#include "dsp/Effect.hpp"
#include <algorithm>
#include <iomanip>
#include <sstream>

namespace guitarfx {

class NoiseGate : public Effect {
public:
    NoiseGate() = default;

    const char* get_name() const override { return "Noise Gate"; }

    void process(float* left, float* right, size_t frames) override {
        if (!is_enabled()) return;

        const float thresh_db = threshold_db_.load(std::memory_order_relaxed);
        const float thresh_open = std::pow(10.0f, thresh_db / 20.0f);
        const float thresh_close = std::pow(10.0f, (thresh_db - 2.0f) / 20.0f); // 2 dB hysteresis

        const float attack_coeff = std::exp(-1.0f / (attack_ms_ * 0.001f * sample_rate_));
        const float release_coeff = std::exp(-1.0f / (release_ms_ * 0.001f * sample_rate_));

        for (size_t i = 0; i < frames; ++i) {
            float input_mag = std::max(std::abs(left[i]), std::abs(right[i]));

            // State machine with hysteresis
            if (gate_open_) {
                if (input_mag < thresh_close) {
                    gate_open_ = false;
                }
            } else {
                if (input_mag > thresh_open) {
                    gate_open_ = true;
                }
            }

            float target_gain = gate_open_ ? 1.0f : 0.0f;
            float coeff = (target_gain > current_gain_) ? attack_coeff : release_coeff;
            current_gain_ = target_gain + coeff * (current_gain_ - target_gain);
            current_gain_ = flush_denormal(current_gain_);

            left[i] *= current_gain_;
            right[i] *= current_gain_;
        }
    }

    void reset() override {
        current_gain_ = 0.0f;
        gate_open_ = false;
    }

    void set_threshold_db(float db) {
        threshold_db_.store(std::clamp(db, -80.0f, 0.0f), std::memory_order_relaxed);
    }

    float get_threshold_db() const { return threshold_db_.load(std::memory_order_relaxed); }

    std::string get_param_summary() const override {
        std::ostringstream ss;
        ss << std::fixed << std::setprecision(0) << get_threshold_db() << " dB";
        return ss.str();
    }

private:
    std::atomic<float> threshold_db_{-50.0f};
    float attack_ms_{2.0f};
    float release_ms_{80.0f};
    float current_gain_{0.0f};
    bool gate_open_{false};
};

} // namespace guitarfx
