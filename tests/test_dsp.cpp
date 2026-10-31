#include "dsp/DSPChain.hpp"
#include <cassert>
#include <iostream>
#include <vector>
#include <cmath>

int main() {
    std::cout << "[TEST] Running GuitarFX DSP verification suite...\n";

    guitarfx::DSPChain chain;
    chain.set_sample_rate(48000.0f);

    const size_t kBlockSize = 256;
    std::vector<float> in(kBlockSize, 0.0f);
    std::vector<float> out(kBlockSize * 2, 0.0f);

    // Test 1: Silence throughput
    chain.process_mono_to_stereo(in.data(), out.data(), kBlockSize);
    for (size_t i = 0; i < kBlockSize * 2; ++i) {
        assert(std::abs(out[i]) < 1e-6f && "Silence test failed");
    }
    std::cout << "  [PASS] Silence processing\n";

    // Test 2: Input Gain
    chain.input_gain().set_gain_db(6.0f);
    assert(chain.input_gain().get_gain_db() == 6.0f);
    std::cout << "  [PASS] Input Gain parameter setting\n";

    // Test 3: Limiter ceiling clamp
    // Generate hot signal (+12 dBFS)
    for (size_t i = 0; i < kBlockSize; ++i) {
        in[i] = 4.0f * std::sin(2.0f * 3.14159265f * 440.0f * i / 48000.0f);
    }
    chain.limiter().set_enabled(true);
    chain.limiter().set_ceiling_db(-0.3f);
    chain.process_mono_to_stereo(in.data(), out.data(), kBlockSize);

    float max_peak = 0.0f;
    for (size_t i = 0; i < kBlockSize * 2; ++i) {
        float abs_s = std::abs(out[i]);
        if (abs_s > max_peak) max_peak = abs_s;
    }
    // -0.3 dBFS is ~0.966 linear
    assert(max_peak <= 1.0f && "Limiter failed to clamp peak below 1.0");
    std::cout << "  [PASS] Output Limiter brickwall ceiling (" << max_peak << " <= 1.0)\n";

    // Test 4: Noise Gate attenuation
    // Low input (-60 dBFS)
    for (size_t i = 0; i < kBlockSize; ++i) {
        in[i] = 0.001f;
    }
    chain.noise_gate().set_enabled(true);
    chain.noise_gate().set_threshold_db(-40.0f);
    chain.reset();
    for (int block = 0; block < 10; ++block) {
        chain.process_mono_to_stereo(in.data(), out.data(), kBlockSize);
    }
    float gated_val = std::abs(out[0]);
    assert(gated_val < 0.0001f && "Noise gate failed to attenuate low level signal");
    std::cout << "  [PASS] Noise Gate attenuation below threshold\n";

    // Test 5: Global Mute
    chain.set_mute(true);
    chain.process_mono_to_stereo(in.data(), out.data(), kBlockSize);
    for (size_t i = 0; i < kBlockSize * 2; ++i) {
        assert(out[i] == 0.0f && "Mute failed");
    }
    std::cout << "  [PASS] Global Mute functionality\n";

    std::cout << "\n[ALL 5 DSP TESTS PASSED SUCCESSFULLY!]\n";
    return 0;
}
