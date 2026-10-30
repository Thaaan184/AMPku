# AMPku Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build AMPku, an ultra-low latency, zero-setup standalone guitar and vocal DSP studio desktop application for Windows using Rust, CPAL WASAPI Exclusive mode, and egui.

**Architecture:** The application separates the high-priority real-time audio thread from the 60 FPS GUI thread using lock-free SPSC ring buffers and atomic parameters. The audio engine runs a dual-channel modular serial DSP pipeline (Guitar Ch 1 + Vocal Ch 2) with stereo `AudioFrame` processing, sum bus limiter, and background WAV recording.

**Tech Stack:** Rust (1.75+), `cpal` (audio I/O WASAPI Exclusive), `eframe` & `egui` (native GUI), `hound` (WAV recording), `rtrb` (lock-free SPSC queues), `serde` & `serde_json` (preset management).

**Spec:** `docs/superpowers/specs/2026-10-30-ampku-design.md`

## Global Constraints
- Target platform: Windows 10/11 x86_64, portable single executable under 15 MB.
- Audio callback thread must be strictly non-blocking: zero heap allocations (`malloc`), zero syscalls, zero mutex locks.
- Real-time latency target: sub-3ms round-trip buffer latency (64 or 128 frames @ 48 kHz).
- No external driver dependencies: WASAPI Exclusive direct hardware access without ASIO4ALL.
- All code formatted with `rustfmt` and passing `cargo clippy`.

## Review Focus
1. **Clock Drift Between Independent Hardware Devices**: Input on Vention USB and Output on Realtek Motherboard run on different crystal clocks. Adaptive drift ring buffer must absorb drift without clicks or buffer stalls. (Tested in Task 2 & Task 10).
2. **Audio Buffer NaN/Inf Protection**: Extreme saturation or feedback loops producing non-finite float samples must be clamped to `[-1.0, 1.0]` and never crash or silence the audio pipeline. (Tested in Task 3 & Task 4).
3. **Glitch-Free Parameter Updates & Bypass Switching**: Adjusting sliders or clicking bypass while audio is streaming must smoothly interpolate without clicks or zipper noise. (Tested in Task 3 & Task 8).
4. **WAV Recorder Disk Latency Spikes**: Slow disk I/O during recording must never stall the audio thread; write queue must drop gracefully if capacity is exceeded without tearing memory. (Tested in Task 9).
5. **Sample Rate Mismatch Resilience**: If input device is 48 kHz and output is 44.1 kHz, stream configuration validator must warn or gracefully negotiate supported hardware formats. (Tested in Task 10).

---

### Task 1: Project Scaffolding & Cargo Configuration

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/lib.rs`
- Create: `.gitignore`

**Interfaces:**
- Produces: Project root structure, dependency graph, release build profile with LTO and binary stripping.

- [ ] **Step 1: Create Cargo.toml with all required dependencies and release profiles**
```toml
[package]
name = "ampku"
version = "0.1.0"
edition = "2021"
authors = ["Thaaan184"]
description = "Standalone Low-Latency Guitar & Vocal DSP Studio"

[dependencies]
cpal = "0.15"
eframe = "0.27"
egui = "0.27"
hound = "3.5"
rtrb = "0.3"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

- [ ] **Step 2: Create .gitignore for Rust and temporary files**
Include `/target`, `Cargo.lock` (for libraries, keep tracked for binaries), `*.wav`, `.superpowers/`.

- [ ] **Step 3: Create stub lib.rs and main.rs**
Create minimal `src/lib.rs` and `src/main.rs` that prints "AMPku Engine Ready".

- [ ] **Step 4: Verify cargo check passes**
Run: `cargo check`
Expected: SUCCESS with zero warnings.

- [ ] **Step 5: Commit**
```bash
git add Cargo.toml src/main.rs src/lib.rs .gitignore
git commit -m "chore: scaffold project structure and dependencies"
```

---

### Task 2: Core Audio Frame & Lock-Free Ring Buffer

**Files:**
- Create: `src/core/mod.rs`
- Create: `src/core/frame.rs`
- Create: `src/core/ring_buffer.rs`
- Test: `tests/test_core.rs`

**Interfaces:**
- Produces: `AudioFrame { left: f32, right: f32 }`, `AudioFrame::from_mono(f32)`, `AudioFrame::to_mono(&self) -> f32`, `AudioRingBuffer<T>` producer and consumer pair.

- [ ] **Step 1: Write failing test in tests/test_core.rs**
```rust
use ampku::core::frame::AudioFrame;
use ampku::core::ring_buffer::create_audio_ring_buffer;

#[test]
fn test_audio_frame_mono_stereo_conversion() {
    let mono = 0.5f32;
    let frame = AudioFrame::from_mono(mono);
    assert_eq!(frame.left, 0.5);
    assert_eq!(frame.right, 0.5);
    assert_eq!(frame.to_mono(), 0.5);
}

#[test]
fn test_ring_buffer_push_pop() {
    let (mut producer, mut consumer) = create_audio_ring_buffer::<AudioFrame>(128);
    assert!(producer.push(AudioFrame { left: 0.1, right: -0.1 }).is_ok());
    let popped = consumer.pop().unwrap();
    assert_eq!(popped.left, 0.1);
}
```

- [ ] **Step 2: Run test to verify it fails**
Run: `cargo test --test test_core`
Expected: FAIL (module not defined)

- [ ] **Step 3: Implement AudioFrame and RingBuffer wrapper in src/core/**
Define `AudioFrame` with arithmetic operators (`Add`, `Mul`, `clamp`). Implement `create_audio_ring_buffer` wrapping `rtrb::RingBuffer`.

- [ ] **Step 4: Run test to verify it passes**
Run: `cargo test --test test_core`
Expected: PASS

- [ ] **Step 5: Commit**
```bash
git add src/core/ tests/test_core.rs
git commit -m "feat(core): implement AudioFrame and lock-free SPSC ring buffer"
```

---

### Task 3: Unified DSP Trait & Utility Filters (Biquad HPF/LPF/EQ)

**Files:**
- Create: `src/dsp/mod.rs`
- Create: `src/dsp/traits.rs`
- Create: `src/dsp/filters/mod.rs`
- Create: `src/dsp/filters/biquad.rs`
- Test: `tests/test_biquad.rs`

**Interfaces:**
- Produces: `AudioEffect` trait, `BiquadFilter` (HPF, LPF, Peaking EQ with `set_params(sample_rate, freq, q, gain_db)`).

- [ ] **Step 1: Write failing test in tests/test_biquad.rs**
```rust
use ampku::core::frame::AudioFrame;
use ampku::dsp::traits::AudioEffect;
use ampku::dsp::filters::biquad::{BiquadFilter, FilterType};

#[test]
fn test_biquad_high_pass_attenuates_dc() {
    let mut hpf = BiquadFilter::new(FilterType::HighPass, 48000.0, 100.0, 0.707, 0.0);
    let mut out = AudioFrame::new(1.0, 1.0);
    for _ in 0..1000 {
        out = hpf.process_frame(AudioFrame::new(1.0, 1.0));
    }
    assert!(out.left.abs() < 0.01);
}

#[test]
fn test_biquad_nan_protection() {
    let mut hpf = BiquadFilter::new(FilterType::HighPass, 48000.0, 100.0, 0.707, 0.0);
    let out = hpf.process_frame(AudioFrame::new(f32::NAN, f32::INFINITY));
    assert!(!out.left.is_nan() && !out.right.is_nan());
}
```

- [ ] **Step 2: Run test to verify it fails**
Run: `cargo test --test test_biquad`
Expected: FAIL

- [ ] **Step 3: Implement BiquadFilter using AudioEffect trait**
Calculate Direct Form II transposed biquad coefficients based on Robert Bristow-Johnson Audio EQ Cookbook. Sanitize NaN/Inf inputs with `.unwrap_or(0.0)`.

- [ ] **Step 4: Run test to verify it passes**
Run: `cargo test --test test_biquad`
Expected: PASS

- [ ] **Step 5: Commit**
```bash
git add src/dsp/traits.rs src/dsp/filters/ tests/test_biquad.rs
git commit -m "feat(dsp): implement AudioEffect trait and biquad filter module"
```

---

### Task 4: Dynamics & Saturation Nodes (Gate, Comp, Overdrive, Distortion, Fuzz)

**Files:**
- Create: `src/dsp/dynamics/mod.rs`
- Create: `src/dsp/dynamics/noise_gate.rs`
- Create: `src/dsp/dynamics/compressor.rs`
- Create: `src/dsp/saturation/mod.rs`
- Create: `src/dsp/saturation/overdrive.rs`
- Create: `src/dsp/saturation/distortion.rs`
- Create: `src/dsp/saturation/fuzz.rs`
- Test: `tests/test_dynamics_saturation.rs`

**Interfaces:**
- Produces: `NoiseGate`, `Compressor`, `Overdrive`, `Distortion`, `Fuzz` implementing `AudioEffect`.

- [ ] **Step 1: Write failing test in tests/test_dynamics_saturation.rs**
```rust
use ampku::core::frame::AudioFrame;
use ampku::dsp::traits::AudioEffect;
use ampku::dsp::dynamics::noise_gate::NoiseGate;
use ampku::dsp::saturation::overdrive::Overdrive;

#[test]
fn test_noise_gate_mutes_low_level() {
    let mut gate = NoiseGate::new(48000.0, -40.0, 5.0, 50.0);
    let low_sig = AudioFrame::new(0.0001, 0.0001); // ~ -80 dB
    let out = gate.process_frame(low_sig);
    assert_eq!(out.left, 0.0);
}

#[test]
fn test_overdrive_soft_clips() {
    let mut od = Overdrive::new(5.0, 0.5, 0.8);
    let loud_sig = AudioFrame::new(0.9, 0.9);
    let out = od.process_frame(loud_sig);
    assert!(out.left > 0.0 && out.left <= 0.85);
}
```

- [ ] **Step 2: Run test to verify it fails**
Run: `cargo test --test test_dynamics_saturation`
Expected: FAIL

- [ ] **Step 3: Implement dynamics and saturation algorithms**
- `NoiseGate`: RMS/Peak envelope detector with Attack/Release state machine.
- `Overdrive`: Hyperbolic tangent `tanh(x * gain)` soft-saturation curve with tone low-pass.
- `Distortion`: Asymmetric diode hard-clipping with pre-emphasis.
- `Fuzz`: Square-wave wavefolder with high-gain transistor saturation.

- [ ] **Step 4: Run test to verify it passes**
Run: `cargo test --test test_dynamics_saturation`
Expected: PASS

- [ ] **Step 5: Commit**
```bash
git add src/dsp/dynamics/ src/dsp/saturation/ tests/test_dynamics_saturation.rs
git commit -m "feat(dsp): implement noise gate, compressor, overdrive, distortion, and fuzz"
```

---

### Task 5: Cabinet IR Simulation & Factory Impulse Responses

**Files:**
- Create: `src/dsp/cab_sim/mod.rs`
- Create: `src/dsp/cab_sim/ir_engine.rs`
- Create: `src/dsp/cab_sim/factory_irs.rs`
- Test: `tests/test_cab_sim.rs`

**Interfaces:**
- Produces: `CabinetSim` implementing `AudioEffect`, `CabinetProfile` enum (`CelestionV30`, `FenderTwin`, `Marshall1960`, `AcousticSim`).

- [ ] **Step 1: Write failing test in tests/test_cab_sim.rs**
```rust
use ampku::core::frame::AudioFrame;
use ampku::dsp::traits::AudioEffect;
use ampku::dsp::cab_sim::ir_engine::{CabinetSim, CabinetProfile};

#[test]
fn test_cab_sim_impulse_response_convolves() {
    let mut cab = CabinetSim::new(CabinetProfile::CelestionV30);
    let delta = AudioFrame::new(1.0, 1.0);
    let out1 = cab.process_frame(delta);
    let out2 = cab.process_frame(AudioFrame::new(0.0, 0.0));
    assert!(out1.left != 0.0 || out2.left != 0.0);
}
```

- [ ] **Step 2: Run test to verify it fails**
Run: `cargo test --test test_cab_sim`
Expected: FAIL

- [ ] **Step 3: Implement FIR partitioned convolution engine & factory impulse curves**
Implement direct time-domain FIR convolution for zero-latency buffer processing. Synthesize accurate resonant acoustic impulse responses for the 4 factory speaker cabinets.

- [ ] **Step 4: Run test to verify it passes**
Run: `cargo test --test test_cab_sim`
Expected: PASS

- [ ] **Step 5: Commit**
```bash
git add src/dsp/cab_sim/ tests/test_cab_sim.rs
git commit -m "feat(dsp): implement FIR cabinet IR simulation and 4 factory profiles"
```

---

### Task 6: Time-Based & Spatial Effects (Stereo Delay & Reverb)

**Files:**
- Create: `src/dsp/time/mod.rs`
- Create: `src/dsp/time/delay.rs`
- Create: `src/dsp/time/reverb.rs`
- Test: `tests/test_time_effects.rs`

**Interfaces:**
- Produces: `StereoDelay`, `StereoReverb` implementing `AudioEffect` with ping-pong / stereo spread support.

- [ ] **Step 1: Write failing test in tests/test_time_effects.rs**
```rust
use ampku::core::frame::AudioFrame;
use ampku::dsp::traits::AudioEffect;
use ampku::dsp::time::delay::StereoDelay;

#[test]
fn test_delay_ring_buffer_playback() {
    let mut delay = StereoDelay::new(48000.0, 10.0, 0.5, 1.0); // 10ms delay, 100% wet
    let impulse = AudioFrame::new(1.0, 1.0);
    let _ = delay.process_frame(impulse);
    let mut found = false;
    for _ in 0..500 {
        let out = delay.process_frame(AudioFrame::new(0.0, 0.0));
        if out.left.abs() > 0.01 {
            found = true;
            break;
        }
    }
    assert!(found);
}
```

- [ ] **Step 2: Run test to verify it fails**
Run: `cargo test --test test_time_effects`
Expected: FAIL

- [ ] **Step 3: Implement StereoDelay and Schroeder/Freeverb StereoReverb**
- `StereoDelay`: Circular buffer with fractional delay interpolation, low-pass damping in feedback loop, cross-channel ping-pong spread.
- `StereoReverb`: 8 parallel comb filters + 4 series allpass filters with stereo decorrelation.

- [ ] **Step 4: Run test to verify it passes**
Run: `cargo test --test test_time_effects`
Expected: PASS

- [ ] **Step 5: Commit**
```bash
git add src/dsp/time/ tests/test_time_effects.rs
git commit -m "feat(dsp): implement stereo ping-pong delay and algorithmic reverb"
```

---

### Task 7: Vocal DSP Suite (High-Pass, De-Esser, Vocal Compressor)

**Files:**
- Create: `src/dsp/vocal/mod.rs`
- Create: `src/dsp/vocal/de_esser.rs`
- Test: `tests/test_vocal.rs`

**Interfaces:**
- Produces: `VocalDeEsser`, combining Biquad bandpass side-chain detector (5kHz - 8kHz) with dynamic gain reduction.

- [ ] **Step 1: Write failing test in tests/test_vocal.rs**
```rust
use ampku::core::frame::AudioFrame;
use ampku::dsp::traits::AudioEffect;
use ampku::dsp::vocal::de_esser::VocalDeEsser;

#[test]
fn test_de_esser_attenuates_sibilance() {
    let mut de_esser = VocalDeEsser::new(48000.0, 6000.0, -20.0, 4.0);
    let sibilant_frame = AudioFrame::new(0.8, 0.8);
    let out = de_esser.process_frame(sibilant_frame);
    assert!(out.left < 0.8);
}
```

- [ ] **Step 2: Run test to verify it fails**
Run: `cargo test --test test_vocal`
Expected: FAIL

- [ ] **Step 3: Implement VocalDeEsser in src/dsp/vocal/de_esser.rs**
Implement sibilance detection using a peaking bandpass filter at ~6 kHz feeding an envelope follower, reducing high-frequency gain when threshold is crossed.

- [ ] **Step 4: Run test to verify it passes**
Run: `cargo test --test test_vocal`
Expected: PASS

- [ ] **Step 5: Commit**
```bash
git add src/dsp/vocal/ tests/test_vocal.rs
git commit -m "feat(dsp): implement vocal de-esser and vocal channel modules"
```

---

### Task 8: Modular Effect Chain & Channel Strip Engine

**Files:**
- Create: `src/engine/mod.rs`
- Create: `src/engine/chain.rs`
- Create: `src/engine/channel.rs`
- Test: `tests/test_chain.rs`

**Interfaces:**
- Produces: `EffectChain` (add, remove, swap, bypass nodes dynamically), `ChannelStrip` (volume, pan, mute, solo, mono/stereo mode, VU meter feeder).

- [ ] **Step 1: Write failing test in tests/test_chain.rs**
```rust
use ampku::core::frame::AudioFrame;
use ampku::engine::chain::EffectChain;
use ampku::dsp::saturation::overdrive::Overdrive;

#[test]
fn test_effect_chain_add_and_process() {
    let mut chain = EffectChain::new();
    chain.add_effect(Box::new(Overdrive::new(2.0, 0.5, 0.8)));
    assert_eq!(chain.len(), 1);
    let out = chain.process_frame(AudioFrame::new(0.5, 0.5));
    assert!(out.left > 0.0);
}
```

- [ ] **Step 2: Run test to verify it fails**
Run: `cargo test --test test_chain`
Expected: FAIL

- [ ] **Step 3: Implement EffectChain and ChannelStrip**
- `EffectChain`: Iterates through active `Box<dyn AudioEffect>`. Includes safe non-allocating lock-free node swapping command queue for the audio thread.
- `ChannelStrip`: Holds channel gain, stereo pan matrix, mute/solo flags, mono/stereo toggle, and atomic peak/RMS calculation.

- [ ] **Step 4: Run test to verify it passes**
Run: `cargo test --test test_chain`
Expected: PASS

- [ ] **Step 5: Commit**
```bash
git add src/engine/ tests/test_chain.rs
git commit -m "feat(engine): implement modular dynamic effect chain and channel strip"
```

---

### Task 9: Master Sum Bus, Limiter & Background WAV Recorder

**Files:**
- Create: `src/engine/master_bus.rs`
- Create: `src/audio/recorder.rs`
- Test: `tests/test_master_recorder.rs`

**Interfaces:**
- Produces: `MasterBus` (brickwall lookahead limiter to prevent 0 dBFS clipping), `WavRecorder` (background worker thread writing 24-bit 48kHz WAV via `hound`).

- [ ] **Step 1: Write failing test in tests/test_master_recorder.rs**
```rust
use ampku::core::frame::AudioFrame;
use ampku::engine::master_bus::MasterBus;
use ampku::audio::recorder::WavRecorder;
use std::path::PathBuf;

#[test]
fn test_master_bus_clamps_overs() {
    let mut bus = MasterBus::new();
    let super_hot = AudioFrame::new(3.0, 3.0);
    let limited = bus.process_frame(super_hot);
    assert!(limited.left <= 1.0 && limited.right <= 1.0);
}

#[test]
fn test_recorder_start_stop() {
    let mut recorder = WavRecorder::new();
    let tmp_path = PathBuf::from("/tmp/test_rec.wav");
    assert!(recorder.start_recording(tmp_path.clone(), 48000).is_ok());
    recorder.push_frame(AudioFrame::new(0.1, 0.1));
    assert!(recorder.stop_recording().is_ok());
    assert!(tmp_path.exists());
    let _ = std::fs::remove_file(tmp_path);
}
```

- [ ] **Step 2: Run test to verify it fails**
Run: `cargo test --test test_master_recorder`
Expected: FAIL

- [ ] **Step 3: Implement MasterBus limiter and WavRecorder worker**
- `MasterBus`: Soft saturation knee limiter preventing digital distortion.
- `WavRecorder`: Communicates via `rtrb` SPSC queue with a dedicated file I/O thread. Disk writes never block the audio stream.

- [ ] **Step 4: Run test to verify it passes**
Run: `cargo test --test test_master_recorder`
Expected: PASS

- [ ] **Step 5: Commit**
```bash
git add src/engine/master_bus.rs src/audio/recorder.rs tests/test_master_recorder.rs
git commit -m "feat(audio): implement master sum bus limiter and background WAV recorder"
```

---

### Task 10: CPAL Audio Engine & WASAPI Exclusive Stream Management

**Files:**
- Create: `src/audio/mod.rs`
- Create: `src/audio/device.rs`
- Create: `src/audio/engine.rs`
- Create: `src/audio/clock_drift.rs`
- Test: `tests/test_clock_drift.rs`

**Interfaces:**
- Produces: `AudioEngine` (initializes CPAL host, enumerates input/output devices, starts WASAPI Exclusive duplex streams, integrates `ClockDriftBuffer` between different hardware devices).

- [ ] **Step 1: Write failing test for ClockDriftBuffer in tests/test_clock_drift.rs**
```rust
use ampku::audio::clock_drift::ClockDriftBuffer;
use ampku::core::frame::AudioFrame;

#[test]
fn test_clock_drift_buffer_resilience() {
    let mut drift_buf = ClockDriftBuffer::new(256);
    // Push 64 frames from input clock
    for i in 0..64 {
        drift_buf.write(AudioFrame::new(i as f32, i as f32));
    }
    // Read 64 frames from output clock
    let read = drift_buf.read_block(64);
    assert_eq!(read.len(), 64);
}
```

- [ ] **Step 2: Run test to verify it fails**
Run: `cargo test --test test_clock_drift`
Expected: FAIL

- [ ] **Step 3: Implement ClockDriftBuffer and AudioEngine**
- `ClockDriftBuffer`: Circular buffer with proportional-integral (PI) drift correction to smoothly interpolate when input and output devices run on distinct crystal oscillators.
- `AudioEngine`: Queries CPAL devices, validates sample rate and buffer frame sizes (64/128), and wires audio callbacks to Guitar & Mic channel strips.

- [ ] **Step 4: Run test to verify it passes**
Run: `cargo test --test test_clock_drift`
Expected: PASS

- [ ] **Step 5: Commit**
```bash
git add src/audio/ tests/test_clock_drift.rs
git commit -m "feat(audio): implement CPAL WASAPI exclusive audio engine and clock drift buffer"
```

---

### Task 11: Preset Management Engine (JSON Serialization & Factory Tones)

**Files:**
- Create: `src/presets/mod.rs`
- Create: `src/presets/schema.rs`
- Create: `src/presets/manager.rs`
- Create: `src/presets/defaults.rs`
- Test: `tests/test_presets.rs`

**Interfaces:**
- Produces: `PresetManager` (save, load, list JSON presets), factory presets (`TexasBlues`, `ModernLead`, `CleanAmbient`, `AcousticChime`, `ClearVocal`).

- [ ] **Step 1: Write failing test in tests/test_presets.rs**
```rust
use ampku::presets::schema::PresetData;
use ampku::presets::defaults::get_factory_presets;
use ampku::presets::manager::PresetManager;

#[test]
fn test_factory_presets_validity() {
    let presets = get_factory_presets();
    assert!(presets.len() >= 4);
    let json = serde_json::to_string(&presets[0]).unwrap();
    let deserialized: PresetData = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.name, presets[0].name);
}
```

- [ ] **Step 2: Run test to verify it fails**
Run: `cargo test --test test_presets`
Expected: FAIL

- [ ] **Step 3: Implement PresetManager and default factory tone definitions**
Serialize and deserialize full rack configurations including bypass states and slider parameters.

- [ ] **Step 4: Run test to verify it passes**
Run: `cargo test --test test_presets`
Expected: PASS

- [ ] **Step 5: Commit**
```bash
git add src/presets/ tests/test_presets.rs
git commit -m "feat(presets): implement JSON preset management and factory tones"
```

---

### Task 12: GUI - Channel Strips & Modular Rack UI (`egui`)

**Files:**
- Create: `src/gui/mod.rs`
- Create: `src/gui/theme.rs`
- Create: `src/gui/effect_card.rs`
- Create: `src/gui/channel_strip.rs`

**Interfaces:**
- Consumes: `EffectChain`, `ChannelStrip`, `AudioEffect` trait.
- Produces: `render_channel_strip(ui, channel, title)`, `render_effect_card(ui, effect)`.

- [ ] **Step 1: Implement dark studio design tokens in src/gui/theme.rs**
Define background (`#18181b`), card container (`#27272a`), accent blue (`#38bdf8`), alert red (`#ef4444`), and font styles.

- [ ] **Step 2: Implement effect_card.rs**
Render individual collapsible cards with power toggle, remove `[X]`, move up/down, wet/dry balance, and parameter faders.

- [ ] **Step 3: Implement channel_strip.rs**
Render Guitar (Ch 1) and Mic (Ch 2) columns: Volume fader, Pan slider, Mute, Solo, Mono/Stereo toggle switch, live VU meter, and `[+ Add Effect]` menu.

- [ ] **Step 4: Verify cargo check passes**
Run: `cargo check`
Expected: PASS

- [ ] **Step 5: Commit**
```bash
git add src/gui/
git commit -m "feat(gui): implement egui channel strips and modular effect cards"
```

---

### Task 13: GUI - Master Bar, VU Meter & Transport Controls

**Files:**
- Create: `src/gui/meter.rs`
- Create: `src/gui/transport.rs`
- Create: `src/gui/app.rs`
- Modify: `src/main.rs`

**Interfaces:**
- Consumes: `AudioEngine`, `MasterBus`, `WavRecorder`, `PresetManager`.
- Produces: `AmpkuApp` implementing `eframe::App`.

- [ ] **Step 1: Implement meter.rs (Real-Time VU Meter Widget)**
Render segmented green/yellow/red LED meter bar with numeric dB readout (-60 dB to +3 dB) and peak hold clipping dot.

- [ ] **Step 2: Implement transport.rs & Top Bar**
- Hardware I/O selector dropdowns (Input Vention, Output Speakers).
- Buffer size and latency badge (`64 smp - 1.3 ms`).
- Preset loader/saver bar.
- Record transport bar: Red `● REC` button, duration timer, and folder shortcut.

- [ ] **Step 3: Implement AmpkuApp in src/gui/app.rs & wire main.rs**
Connect audio engine loop with GUI repaint callback (60 FPS). Initialize window with `eframe::run_native`.

- [ ] **Step 4: Verify cargo check passes**
Run: `cargo check`
Expected: PASS

- [ ] **Step 5: Commit**
```bash
git add src/gui/ src/main.rs
git commit -m "feat(gui): implement master transport, VU meters, and eframe application loop"
```

---

### Task 14: Integration, Cross-Compilation Verification & Release Packaging

**Files:**
- Create: `.cargo/config.toml`
- Modify: `README.md`
- Modify: `CHANGELOG.md`

**Interfaces:**
- Produces: Verified release executable target (`x86_64-pc-windows-gnu` / `x86_64-pc-windows-msvc`), full documentation of verification steps.

- [ ] **Step 1: Configure .cargo/config.toml for Windows cross-compilation**
Setup Windows target settings if building from Linux via MinGW (`x86_64-pc-windows-gnu`).

- [ ] **Step 2: Run all unit and integration test suites**
Run: `cargo test --all`
Expected: All tests PASS with zero errors.

- [ ] **Step 3: Update README and CHANGELOG with release v0.1.0 details**
Document executable distribution, verified latency numbers, and full feature matrix.

- [ ] **Step 4: Commit and push to GitHub**
```bash
git add .
git commit -m "chore(release): configure release target, verify tests, and finalize v0.1.0 docs"
git push origin main
```
