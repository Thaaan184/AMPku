#pragma once
#include "dsp/Effect.hpp"
#include <iomanip>
#include <sstream>

namespace guitarfx {

class InputGain : public Effect {
public:
    InputGain() = default;

    const char* get_name() const override { return "Input Gain"; }

    void process(float* left, float* right, size_t frames) override {
        if (!is_enabled()) return;
        const float target = linear_gain_.load(std::memory_order_relaxed);
        
        // Parameter smoothing (simple 1-pole lowpass on gain parameter)
        for (size_t i = 0; i < frames; ++i) {
            current_gain_ += 0.05f * (target - current_gain_);
            left[i] *= current_gain_;
            right[i] *= current_gain_;
        }
    }

    void reset() override {
        current_gain_ = linear_gain_.load(std::memory_order_relaxed);
    }

    void set_gain_db(float db) {
        db = std::clamp(db, -24.0f, +24.0f);
        gain_db_.store(db, std::memory_order_relaxed);
        linear_gain_.store(std::pow(10.0f, db / 20.0f), std::memory_order_relaxed);
    }

    float get_gain_db() const { return gain_db_.load(std::memory_order_relaxed); }

    std::string get_param_summary() const override {
        std::ostringstream ss;
        ss << std::fixed << std::setprecision(1) << (get_gain_db() >= 0 ? "+" : "") << get_gain_db() << " dB";
        return ss.str();
    }

private:
    std::atomic<float> gain_db_{0.0f};
    std::atomic<float> linear_gain_{1.0f};
    float current_gain_{1.0f};
};

} // namespace guitarfx
