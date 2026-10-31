#pragma once
#include "dsp/Effect.hpp"
#include <algorithm>
#include <cmath>
#include <iomanip>
#include <sstream>

namespace guitarfx {

class Equalizer : public Effect {
public:
    Equalizer() {
        update_coefficients();
    }

    const char* get_name() const override { return "Equalizer"; }

    void set_sample_rate(float sample_rate) override {
        Effect::set_sample_rate(sample_rate);
        update_coefficients();
    }

    void process(float* left, float* right, size_t frames) override {
        if (!is_enabled()) return;

        for (size_t i = 0; i < frames; ++i) {
            float l = left[i];
            float r = right[i];

            l = low_l_.process(l);
            l = mid_l_.process(l);
            l = high_l_.process(l);

            r = low_r_.process(r);
            r = mid_r_.process(r);
            r = high_r_.process(r);

            left[i] = flush_denormal(l);
            right[i] = flush_denormal(r);
        }
    }

    void reset() override {
        low_l_.reset(); low_r_.reset();
        mid_l_.reset(); mid_r_.reset();
        high_l_.reset(); high_r_.reset();
    }

    void set_low_db(float db) {
        low_db_.store(std::clamp(db, -15.0f, +15.0f), std::memory_order_relaxed);
        update_coefficients();
    }
    float get_low_db() const { return low_db_.load(std::memory_order_relaxed); }

    void set_mid_db(float db) {
        mid_db_.store(std::clamp(db, -15.0f, +15.0f), std::memory_order_relaxed);
        update_coefficients();
    }
    float get_mid_db() const { return mid_db_.load(std::memory_order_relaxed); }

    void set_high_db(float db) {
        high_db_.store(std::clamp(db, -15.0f, +15.0f), std::memory_order_relaxed);
        update_coefficients();
    }
    float get_high_db() const { return high_db_.load(std::memory_order_relaxed); }

    std::string get_param_summary() const override {
        std::ostringstream ss;
        ss << "L:" << static_cast<int>(get_low_db()) << " M:" << static_cast<int>(get_mid_db()) << " H:" << static_cast<int>(get_high_db());
        return ss.str();
    }

private:
    struct Biquad {
        float b0{1.0f}, b1{0.0f}, b2{0.0f};
        float a1{0.0f}, a2{0.0f};
        float s1{0.0f}, s2{0.0f};

        float process(float in) {
            float out = b0 * in + s1;
            s1 = b1 * in - a1 * out + s2;
            s2 = b2 * in - a2 * out;
            return out;
        }

        void reset() {
            s1 = 0.0f;
            s2 = 0.0f;
        }

        void make_peaking(float f0, float fs, float gain_db, float q) {
            float a = std::pow(10.0f, gain_db / 40.0f);
            float w0 = 2.0f * 3.14159265f * f0 / fs;
            float alpha = std::sin(w0) / (2.0f * q);
            float cos_w = std::cos(w0);

            float a0 = 1.0f + alpha / a;
            b0 = (1.0f + alpha * a) / a0;
            b1 = (-2.0f * cos_w) / a0;
            b2 = (1.0f - alpha * a) / a0;
            a1 = (-2.0f * cos_w) / a0;
            a2 = (1.0f - alpha / a) / a0;
        }

        void make_low_shelf(float f0, float fs, float gain_db) {
            float a = std::pow(10.0f, gain_db / 40.0f);
            float w0 = 2.0f * 3.14159265f * f0 / fs;
            float cos_w = std::cos(w0);
            float sin_w = std::sin(w0);
            float alpha = sin_w / 2.0f * std::sqrt(2.0f);

            float a0 = (a + 1.0f) + (a - 1.0f) * cos_w + 2.0f * std::sqrt(a) * alpha;
            b0 = (a * ((a + 1.0f) - (a - 1.0f) * cos_w + 2.0f * std::sqrt(a) * alpha)) / a0;
            b1 = (2.0f * a * ((a - 1.0f) - (a + 1.0f) * cos_w)) / a0;
            b2 = (a * ((a + 1.0f) - (a - 1.0f) * cos_w - 2.0f * std::sqrt(a) * alpha)) / a0;
            a1 = (-2.0f * ((a - 1.0f) + (a + 1.0f) * cos_w)) / a0;
            a2 = ((a + 1.0f) + (a - 1.0f) * cos_w - 2.0f * std::sqrt(a) * alpha) / a0;
        }

        void make_high_shelf(float f0, float fs, float gain_db) {
            float a = std::pow(10.0f, gain_db / 40.0f);
            float w0 = 2.0f * 3.14159265f * f0 / fs;
            float cos_w = std::cos(w0);
            float sin_w = std::sin(w0);
            float alpha = sin_w / 2.0f * std::sqrt(2.0f);

            float a0 = (a + 1.0f) - (a - 1.0f) * cos_w + 2.0f * std::sqrt(a) * alpha;
            b0 = (a * ((a + 1.0f) + (a - 1.0f) * cos_w + 2.0f * std::sqrt(a) * alpha)) / a0;
            b1 = (-2.0f * a * ((a - 1.0f) + (a + 1.0f) * cos_w)) / a0;
            b2 = (a * ((a + 1.0f) + (a - 1.0f) * cos_w - 2.0f * std::sqrt(a) * alpha)) / a0;
            a1 = (2.0f * ((a - 1.0f) - (a + 1.0f) * cos_w)) / a0;
            a2 = ((a + 1.0f) - (a - 1.0f) * cos_w - 2.0f * std::sqrt(a) * alpha) / a0;
        }
    };

    void update_coefficients() {
        float fs = sample_rate_;
        low_l_.make_low_shelf(120.0f, fs, low_db_.load(std::memory_order_relaxed));
        low_r_.make_low_shelf(120.0f, fs, low_db_.load(std::memory_order_relaxed));

        mid_l_.make_peaking(1000.0f, fs, mid_db_.load(std::memory_order_relaxed), 1.2f);
        mid_r_.make_peaking(1000.0f, fs, mid_db_.load(std::memory_order_relaxed), 1.2f);

        high_l_.make_high_shelf(3500.0f, fs, high_db_.load(std::memory_order_relaxed));
        high_r_.make_high_shelf(3500.0f, fs, high_db_.load(std::memory_order_relaxed));
    }

    std::atomic<float> low_db_{0.0f};
    std::atomic<float> mid_db_{2.0f};
    std::atomic<float> high_db_{1.0f};

    Biquad low_l_, low_r_;
    Biquad mid_l_, mid_r_;
    Biquad high_l_, high_r_;
};

} // namespace guitarfx
