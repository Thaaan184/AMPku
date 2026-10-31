#pragma once
#include <string>
#include <atomic>
#include <cmath>

namespace guitarfx {

/// Base interface for all real-time DSP effects.
class Effect {
public:
    virtual ~Effect() = default;

    /// Process a block of stereo frames in-place or from in buffer to out buffer.
    /// Audio frames are interleaved or provided as separate L/R pointers.
    virtual void process(float* left, float* right, size_t frames) = 0;

    /// Reset internal state buffers (e.g. delay lines, filter history).
    virtual void reset() = 0;

    /// Update sample rate (called when audio stream starts or changes).
    virtual void set_sample_rate(float sample_rate) {
        sample_rate_ = sample_rate;
        reset();
    }

    /// Effect identifier name.
    virtual const char* get_name() const = 0;

    /// Primary display parameter summary string for terminal UI.
    virtual std::string get_param_summary() const = 0;

    /// Toggle enabled/bypassed state.
    bool is_enabled() const { return enabled_.load(std::memory_order_relaxed); }
    void set_enabled(bool e) { enabled_.store(e, std::memory_order_relaxed); }

protected:
    float sample_rate_{48000.0f};
    std::atomic<bool> enabled_{true};

    /// Flush IEEE-754 denormal numbers to zero to prevent FPU microcode slowdowns.
    static inline float flush_denormal(float val) {
        return (std::abs(val) < 1e-15f) ? 0.0f : val;
    }
};

} // namespace guitarfx
