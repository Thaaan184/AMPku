#pragma once
#include "dsp/Effect.hpp"
#include <algorithm>
#include <cmath>
#include <string>

namespace guitarfx {

enum class AmpVoicing {
    Clean = 0,
    Crunch = 1,
    Lead = 2
};

class AmpSimulator : public Effect {
public:
    AmpSimulator() = default;

    const char* get_name() const override { return "Amp Simulator"; }

    void process(float* left, float* right, size_t frames) override {
        if (!is_enabled()) return;

        const int voicing = voicing_.load(std::memory_order_relaxed);
        const float gain_param = gain_.load(std::memory_order_relaxed);
        const float bass = bass_.load(std::memory_order_relaxed);
        const float mid = mid_.load(std::memory_order_relaxed);
        const float treble = treble_.load(std::memory_order_relaxed);

        float drive_mult = 1.0f;
        if (voicing == 0) drive_mult = 1.2f + gain_param * 2.0f;       // Clean
        else if (voicing == 1) drive_mult = 3.0f + gain_param * 8.0f;  // Crunch
        else drive_mult = 8.0f + gain_param * 20.0f;                  // Lead

        for (size_t i = 0; i < frames; ++i) {
            left[i] = process_sample(left[i], drive_mult, voicing, bass, mid, treble, state_l_);
            right[i] = process_sample(right[i], drive_mult, voicing, bass, mid, treble, state_r_);
        }
    }

    void reset() override {
        state_l_ = ChannelState{};
        state_r_ = ChannelState{};
    }

    void set_voicing(AmpVoicing v) { voicing_.store(static_cast<int>(v), std::memory_order_relaxed); }
    AmpVoicing get_voicing() const { return static_cast<AmpVoicing>(voicing_.load(std::memory_order_relaxed)); }

    void set_gain(float g) { gain_.store(std::clamp(g, 0.0f, 1.0f), std::memory_order_relaxed); }
    float get_gain() const { return gain_.load(std::memory_order_relaxed); }

    void set_bass(float b) { bass_.store(std::clamp(b, 0.0f, 1.0f), std::memory_order_relaxed); }
    float get_bass() const { return bass_.load(std::memory_order_relaxed); }

    void set_mid(float m) { mid_.store(std::clamp(m, 0.0f, 1.0f), std::memory_order_relaxed); }
    float get_mid() const { return mid_.load(std::memory_order_relaxed); }

    void set_treble(float t) { treble_.store(std::clamp(t, 0.0f, 1.0f), std::memory_order_relaxed); }
    float get_treble() const { return treble_.load(std::memory_order_relaxed); }

    std::string get_param_summary() const override {
        switch (get_voicing()) {
            case AmpVoicing::Clean: return "CLEAN";
            case AmpVoicing::Crunch: return "CRUNCH";
            case AmpVoicing::Lead: return "LEAD";
        }
        return "CRUNCH";
    }

private:
    struct ChannelState {
        float bass_state{0.0f};
        float mid_state{0.0f};
        float treble_state{0.0f};
        float cab_lp1{0.0f};
        float cab_lp2{0.0f};
        float cab_hp{0.0f};
    };

    float process_sample(float in, float drive, int voicing, float bass, float mid, float treble, ChannelState& s) {
        float x = in * drive;

        // Tube preamp tube saturation curve
        float saturated = (x / (1.0f + std::abs(x))) * 1.2f;

        // 3-Band Tone Stack (Approximation)
        // Bass filter: 150 Hz
        s.bass_state += 0.04f * (saturated - s.bass_state);
        s.bass_state = flush_denormal(s.bass_state);
        float bass_comp = s.bass_state * (0.5f + bass);

        // Treble filter: high-pass above 2.5 kHz
        s.treble_state += 0.35f * (saturated - s.treble_state);
        s.treble_state = flush_denormal(s.treble_state);
        float treble_comp = (saturated - s.treble_state) * (0.5f + treble);

        // Mid filter: band between bass and treble
        float mid_comp = (s.treble_state - s.bass_state) * (0.5f + mid);

        float tone_shaped = bass_comp + mid_comp + treble_comp;

        // 2-Pole Speaker Cabinet Low-Pass Filter @ 4.8 kHz
        const float lp_alpha = 0.42f;
        s.cab_lp1 += lp_alpha * (tone_shaped - s.cab_lp1);
        s.cab_lp2 += lp_alpha * (s.cab_lp1 - s.cab_lp2);

        // Cabinet High-Pass sub-bass rumble filter @ 75 Hz
        const float hp_alpha = 0.015f;
        s.cab_hp += hp_alpha * (s.cab_lp2 - s.cab_hp);
        float cab_out = s.cab_lp2 - s.cab_hp;

        return flush_denormal(cab_out);
    }

    std::atomic<int> voicing_{1}; // Crunch by default
    std::atomic<float> gain_{0.5f};
    std::atomic<float> bass_{0.5f};
    std::atomic<float> mid_{0.6f};
    std::atomic<float> treble_{0.55f};

    ChannelState state_l_{};
    ChannelState state_r_{};
};

} // namespace guitarfx
