#pragma once
#include "dsp/InputGain.hpp"
#include "dsp/NoiseGate.hpp"
#include "dsp/Overdrive.hpp"
#include "dsp/Distortion.hpp"
#include "dsp/AmpSimulator.hpp"
#include "dsp/Equalizer.hpp"
#include "dsp/Delay.hpp"
#include "dsp/Reverb.hpp"
#include "dsp/Limiter.hpp"
#include <vector>
#include <memory>
#include <atomic>

namespace guitarfx {

class DSPChain {
public:
    DSPChain() {
        effects_.push_back(&input_gain_);
        effects_.push_back(&noise_gate_);
        effects_.push_back(&overdrive_);
        effects_.push_back(&distortion_);
        effects_.push_back(&amp_sim_);
        effects_.push_back(&eq_);
        effects_.push_back(&delay_);
        effects_.push_back(&reverb_);
        effects_.push_back(&limiter_);
    }

    void set_sample_rate(float sample_rate) {
        sample_rate_ = sample_rate;
        for (auto* effect : effects_) {
            effect->set_sample_rate(sample_rate);
        }
    }

    /// Process a block of mono input converted to stereo output.
    /// in: pointer to mono float buffer
    /// out_interleaved: pointer to interleaved L/R float buffer
    void process_mono_to_stereo(const float* in, float* out_interleaved, size_t frames) {
        if (frames == 0) return;

        // Ensure stack/temporary buffer capacity
        if (scratch_l_.size() < frames) {
            scratch_l_.resize(frames);
            scratch_r_.resize(frames);
        }

        // Calculate input metering
        float in_peak = 0.0f;
        float in_sum_sq = 0.0f;
        for (size_t i = 0; i < frames; ++i) {
            float s = in[i];
            float abs_s = std::abs(s);
            if (abs_s > in_peak) in_peak = abs_s;
            in_sum_sq += s * s;
            scratch_l_[i] = s;
            scratch_r_[i] = s;
        }
        input_peak_.store(in_peak, std::memory_order_relaxed);
        input_rms_.store(std::sqrt(in_sum_sq / frames), std::memory_order_relaxed);

        if (mute_.load(std::memory_order_relaxed)) {
            for (size_t i = 0; i < frames; ++i) {
                out_interleaved[i * 2] = 0.0f;
                out_interleaved[i * 2 + 1] = 0.0f;
            }
            output_peak_.store(0.0f, std::memory_order_relaxed);
            output_rms_.store(0.0f, std::memory_order_relaxed);
            return;
        }

        if (!global_bypass_.load(std::memory_order_relaxed)) {
            // Process through serial effect chain
            for (auto* effect : effects_) {
                effect->process(scratch_l_.data(), scratch_r_.data(), frames);
            }
        }

        // Calculate output metering & interleave to output stream
        float out_peak = 0.0f;
        float out_sum_sq = 0.0f;
        for (size_t i = 0; i < frames; ++i) {
            float l = scratch_l_[i];
            float r = scratch_r_[i];

            out_interleaved[i * 2] = l;
            out_interleaved[i * 2 + 1] = r;

            float max_s = std::max(std::abs(l), std::abs(r));
            if (max_s > out_peak) out_peak = max_s;
            out_sum_sq += 0.5f * (l * l + r * r);
        }
        output_peak_.store(out_peak, std::memory_order_relaxed);
        output_rms_.store(std::sqrt(out_sum_sq / frames), std::memory_order_relaxed);
    }

    void reset() {
        for (auto* effect : effects_) {
            effect->reset();
        }
    }

    // Effect accessors
    InputGain& input_gain() { return input_gain_; }
    NoiseGate& noise_gate() { return noise_gate_; }
    Overdrive& overdrive() { return overdrive_; }
    Distortion& distortion() { return distortion_; }
    AmpSimulator& amp_sim() { return amp_sim_; }
    Equalizer& eq() { return eq_; }
    Delay& delay() { return delay_; }
    Reverb& reverb() { return reverb_; }
    Limiter& limiter() { return limiter_; }

    const std::vector<Effect*>& get_effects() const { return effects_; }

    // Telemetry and flags
    void set_global_bypass(bool b) { global_bypass_.store(b, std::memory_order_relaxed); }
    bool is_global_bypass() const { return global_bypass_.load(std::memory_order_relaxed); }

    void set_mute(bool m) { mute_.store(m, std::memory_order_relaxed); }
    bool is_muted() const { return mute_.load(std::memory_order_relaxed); }

    float get_input_peak_db() const { return to_db(input_peak_.load(std::memory_order_relaxed)); }
    float get_input_rms_db() const { return to_db(input_rms_.load(std::memory_order_relaxed)); }
    float get_output_peak_db() const { return to_db(output_peak_.load(std::memory_order_relaxed)); }
    float get_output_rms_db() const { return to_db(output_rms_.load(std::memory_order_relaxed)); }

    void record_underrun() { underruns_.fetch_add(1, std::memory_order_relaxed); }
    uint64_t get_underruns() const { return underruns_.load(std::memory_order_relaxed); }

    void record_overrun() { overruns_.fetch_add(1, std::memory_order_relaxed); }
    uint64_t get_overruns() const { return overruns_.load(std::memory_order_relaxed); }

private:
    static inline float to_db(float linear) {
        if (linear <= 0.000001f) return -96.0f;
        return 20.0f * std::log10(linear);
    }

    float sample_rate_{48000.0f};

    InputGain input_gain_;
    NoiseGate noise_gate_;
    Overdrive overdrive_;
    Distortion distortion_;
    AmpSimulator amp_sim_;
    Equalizer eq_;
    Delay delay_;
    Reverb reverb_;
    Limiter limiter_;

    std::vector<Effect*> effects_;

    std::vector<float> scratch_l_{1024, 0.0f};
    std::vector<float> scratch_r_{1024, 0.0f};

    std::atomic<bool> global_bypass_{false};
    std::atomic<bool> mute_{false};

    std::atomic<float> input_peak_{0.0f};
    std::atomic<float> input_rms_{0.0f};
    std::atomic<float> output_peak_{0.0f};
    std::atomic<float> output_rms_{0.0f};

    std::atomic<uint64_t> underruns_{0};
    std::atomic<uint64_t> overruns_{0};
};

} // namespace guitarfx
