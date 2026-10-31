#pragma once
#include "dsp/Effect.hpp"
#include <algorithm>
#include <vector>
#include <iomanip>
#include <sstream>

namespace guitarfx {

class Reverb : public Effect {
public:
    Reverb() {
        init_buffers();
    }

    const char* get_name() const override { return "Reverb"; }

    void set_sample_rate(float sample_rate) override {
        Effect::set_sample_rate(sample_rate);
        init_buffers();
    }

    void process(float* left, float* right, size_t frames) override {
        if (!is_enabled()) return;

        const float decay = decay_.load(std::memory_order_relaxed);
        const float damp = damping_.load(std::memory_order_relaxed);
        const float mix = mix_.load(std::memory_order_relaxed);

        const float room_size = 0.7f + decay * 0.28f;
        const float damp_factor = damp * 0.4f;

        for (size_t i = 0; i < frames; ++i) {
            float in_l = left[i];
            float in_r = right[i];
            float mono_in = (in_l + in_r) * 0.5f;

            // 1. Parallel Comb Filters
            float out_comb_l = 0.0f;
            for (auto& cf : combs_l_) {
                out_comb_l += cf.process(mono_in, room_size, damp_factor);
            }

            float out_comb_r = 0.0f;
            for (auto& cf : combs_r_) {
                out_comb_r += cf.process(mono_in, room_size, damp_factor);
            }

            // 2. Series Allpass Filters
            float out_allpass_l = out_comb_l;
            for (auto& ap : allpass_l_) {
                out_allpass_l = ap.process(out_allpass_l);
            }

            float out_allpass_r = out_comb_r;
            for (auto& ap : allpass_r_) {
                out_allpass_r = ap.process(out_allpass_r);
            }

            left[i] = in_l * (1.0f - mix) + flush_denormal(out_allpass_l) * mix * 0.4f;
            right[i] = in_r * (1.0f - mix) + flush_denormal(out_allpass_r) * mix * 0.4f;
        }
    }

    void reset() override {
        for (auto& cf : combs_l_) cf.reset();
        for (auto& cf : combs_r_) cf.reset();
        for (auto& ap : allpass_l_) ap.reset();
        for (auto& ap : allpass_r_) ap.reset();
    }

    void set_decay(float d) { decay_.store(std::clamp(d, 0.0f, 1.0f), std::memory_order_relaxed); }
    float get_decay() const { return decay_.load(std::memory_order_relaxed); }

    void set_damping(float d) { damping_.store(std::clamp(d, 0.0f, 1.0f), std::memory_order_relaxed); }
    float get_damping() const { return damping_.load(std::memory_order_relaxed); }

    void set_mix(float m) { mix_.store(std::clamp(m, 0.0f, 1.0f), std::memory_order_relaxed); }
    float get_mix() const { return mix_.load(std::memory_order_relaxed); }

    std::string get_param_summary() const override {
        std::ostringstream ss;
        ss << static_cast<int>(get_mix() * 100.0f) << "%";
        return ss.str();
    }

private:
    struct CombFilter {
        std::vector<float> buffer;
        size_t idx{0};
        float filter_state{0.0f};

        void init(size_t size) {
            buffer.assign(size, 0.0f);
            idx = 0;
            filter_state = 0.0f;
        }

        void reset() {
            std::fill(buffer.begin(), buffer.end(), 0.0f);
            idx = 0;
            filter_state = 0.0f;
        }

        float process(float in, float feedback, float damp) {
            float out = buffer[idx];
            filter_state = out * (1.0f - damp) + filter_state * damp;
            if (std::abs(filter_state) < 1e-15f) filter_state = 0.0f;
            buffer[idx] = in + filter_state * feedback;
            idx = (idx + 1) % buffer.size();
            return out;
        }
    };

    struct AllpassFilter {
        std::vector<float> buffer;
        size_t idx{0};

        void init(size_t size) {
            buffer.assign(size, 0.0f);
            idx = 0;
        }

        void reset() {
            std::fill(buffer.begin(), buffer.end(), 0.0f);
            idx = 0;
        }

        float process(float in) {
            const float feedback = 0.5f;
            float buf_out = buffer[idx];
            if (std::abs(buf_out) < 1e-15f) buf_out = 0.0f;
            buffer[idx] = in + buf_out * feedback;
            idx = (idx + 1) % buffer.size();
            return -in + buf_out;
        }
    };

    void init_buffers() {
        const float scale = sample_rate_ / 44100.0f;
        // Standard Freeverb delay line lengths
        const size_t comb_lengths[8] = {1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617};
        const size_t allpass_lengths[4] = {556, 441, 341, 225};
        const size_t stereo_spread = 23;

        combs_l_.resize(8);
        combs_r_.resize(8);
        for (size_t i = 0; i < 8; ++i) {
            combs_l_[i].init(static_cast<size_t>(comb_lengths[i] * scale));
            combs_r_[i].init(static_cast<size_t>((comb_lengths[i] + stereo_spread) * scale));
        }

        allpass_l_.resize(4);
        allpass_r_.resize(4);
        for (size_t i = 0; i < 4; ++i) {
            allpass_l_[i].init(static_cast<size_t>(allpass_lengths[i] * scale));
            allpass_r_[i].init(static_cast<size_t>((allpass_lengths[i] + stereo_spread) * scale));
        }
    }

    std::atomic<float> decay_{0.5f};
    std::atomic<float> damping_{0.4f};
    std::atomic<float> mix_{0.25f};

    std::vector<CombFilter> combs_l_;
    std::vector<CombFilter> combs_r_;
    std::vector<AllpassFilter> allpass_l_;
    std::vector<AllpassFilter> allpass_r_;
};

} // namespace guitarfx
