#pragma once
#include "dsp/Effect.hpp"
#include <algorithm>
#include <vector>
#include <iomanip>
#include <sstream>

namespace guitarfx {

class Delay : public Effect {
public:
    Delay() {
        // Pre-allocate 2.5 seconds at 96 kHz max
        const size_t max_samples = 96000 * 3;
        buffer_l_.assign(max_samples, 0.0f);
        buffer_r_.assign(max_samples, 0.0f);
    }

    const char* get_name() const override { return "Delay"; }

    void process(float* left, float* right, size_t frames) override {
        if (!is_enabled()) return;

        const float time_ms = time_ms_.load(std::memory_order_relaxed);
        const float feedback = feedback_.load(std::memory_order_relaxed);
        const float mix = mix_.load(std::memory_order_relaxed);

        const size_t delay_samples_l = std::clamp(static_cast<size_t>(time_ms * 0.001f * sample_rate_), static_cast<size_t>(1), buffer_l_.size() - 1);
        // Stereo spread for wide image (1.15x delay on Right channel)
        const size_t delay_samples_r = std::clamp(static_cast<size_t>(delay_samples_l * 1.15f), static_cast<size_t>(1), buffer_r_.size() - 1);

        const size_t buf_len = buffer_l_.size();

        for (size_t i = 0; i < frames; ++i) {
            float in_l = left[i];
            float in_r = right[i];

            size_t read_idx_l = (write_idx_ + buf_len - delay_samples_l) % buf_len;
            size_t read_idx_r = (write_idx_ + buf_len - delay_samples_r) % buf_len;

            float delayed_l = buffer_l_[read_idx_l];
            float delayed_r = buffer_r_[read_idx_r];

            // 1-pole high-cut filter in feedback loop to simulate warm tape/analog delay
            damp_l_ += 0.35f * (delayed_l - damp_l_);
            damp_r_ += 0.35f * (delayed_r - damp_r_);
            damp_l_ = flush_denormal(damp_l_);
            damp_r_ = flush_denormal(damp_r_);

            buffer_l_[write_idx_] = in_l + damp_l_ * feedback;
            buffer_r_[write_idx_] = in_r + damp_r_ * feedback;

            write_idx_ = (write_idx_ + 1) % buf_len;

            left[i] = in_l * (1.0f - mix) + delayed_l * mix;
            right[i] = in_r * (1.0f - mix) + delayed_r * mix;
        }
    }

    void reset() override {
        std::fill(buffer_l_.begin(), buffer_l_.end(), 0.0f);
        std::fill(buffer_r_.begin(), buffer_r_.end(), 0.0f);
        write_idx_ = 0;
        damp_l_ = 0.0f;
        damp_r_ = 0.0f;
    }

    void set_time_ms(float ms) { time_ms_.store(std::clamp(ms, 10.0f, 1500.0f), std::memory_order_relaxed); }
    float get_time_ms() const { return time_ms_.load(std::memory_order_relaxed); }

    void set_feedback(float fb) { feedback_.store(std::clamp(fb, 0.0f, 0.95f), std::memory_order_relaxed); }
    float get_feedback() const { return feedback_.load(std::memory_order_relaxed); }

    void set_mix(float m) { mix_.store(std::clamp(m, 0.0f, 1.0f), std::memory_order_relaxed); }
    float get_mix() const { return mix_.load(std::memory_order_relaxed); }

    std::string get_param_summary() const override {
        std::ostringstream ss;
        ss << static_cast<int>(get_time_ms()) << " ms";
        return ss.str();
    }

private:
    std::atomic<float> time_ms_{320.0f};
    std::atomic<float> feedback_{0.35f};
    std::atomic<float> mix_{0.25f};

    std::vector<float> buffer_l_;
    std::vector<float> buffer_r_;
    size_t write_idx_{0};
    float damp_l_{0.0f};
    float damp_r_{0.0f};
};

} // namespace guitarfx
