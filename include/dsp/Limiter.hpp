#pragma once
#include "dsp/Effect.hpp"
#include <algorithm>
#include <cmath>
#include <iomanip>
#include <sstream>

namespace guitarfx {

class Limiter : public Effect {
public:
    Limiter() = default;

    const char* get_name() const override { return "Output Limiter"; }

    void process(float* left, float* right, size_t frames) override {
        if (!is_enabled()) return;

        const float ceiling_linear = std::pow(10.0f, ceiling_db_.load(std::memory_order_relaxed) / 20.0f);
        const float release_coeff = std::exp(-1.0f / (release_ms_ * 0.001f * sample_rate_));

        for (size_t i = 0; i < frames; ++i) {
            float peak = std::max(std::abs(left[i]), std::abs(right[i]));
            
            float target_gain = 1.0f;
            if (peak > ceiling_linear) {
                target_gain = ceiling_linear / peak;
            }

            if (target_gain < envelope_gain_) {
                // Instant attack
                envelope_gain_ = target_gain;
            } else {
                // Smooth exponential release
                envelope_gain_ = target_gain + release_coeff * (envelope_gain_ - target_gain);
            }
            envelope_gain_ = flush_denormal(envelope_gain_);

            left[i] *= envelope_gain_;
            right[i] *= envelope_gain_;

            // Hard clamp safety to absolute ceiling
            left[i] = std::clamp(left[i], -ceiling_linear, ceiling_linear);
            right[i] = std::clamp(right[i], -ceiling_linear, ceiling_linear);
        }
    }

    void reset() override {
        envelope_gain_ = 1.0f;
    }

    void set_ceiling_db(float db) { ceiling_db_.store(std::clamp(db, -6.0f, 0.0f), std::memory_order_relaxed); }
    float get_ceiling_db() const { return ceiling_db_.load(std::memory_order_relaxed); }

    std::string get_param_summary() const override {
        std::ostringstream ss;
        ss << std::fixed << std::setprecision(1) << get_ceiling_db() << " dBFS";
        return ss.str();
    }

private:
    std::atomic<float> ceiling_db_{-0.3f};
    float release_ms_{40.0f};
    float envelope_gain_{1.0f};
};

} // namespace guitarfx
