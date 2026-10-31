# GuitarFX (Rust Edition) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task with dedicated implementer and curator/QA subagents per phase.

**Goal:** Build a lightweight Windows terminal interactive guitar effects processor (`guitarfx.exe`) in pure Rust, capturing input from a USB soundcard (Vention) or VoiceMeeter B1/B2, processing audio with 9 custom native DSP effects, and routing output to VoiceMeeter Banana AUX Input.

**Architecture:** Lock-free duplex audio processing pipeline (`cpal` WASAPI + `rtrb` SPSC ring buffer) decoupled from an interactive terminal UI (`ratatui` + `crossterm`) with JSON preset persistence (`serde_json`). Zero GUI driver dependencies, zero VST/DAW overhead, sub-10ms buffer latency.

**Tech Stack:** Rust 2021 edition, `cpal` (WASAPI), `rtrb` (lock-free SPSC buffer), `ratatui` + `crossterm` (terminal dashboard), `serde` + `serde_json` (presets).

---

## Global Constraints
- Target platform: Windows 10/11 x64 standalone executable (`guitarfx.exe`).
- Terminal-only interface: No graphical window (no egui, no DirectX/OpenGL requirements).
- Audio backend: WASAPI shared mode via `cpal` for universal VoiceMeeter and physical soundcard compatibility.
- Real-time safety: Zero memory allocation (`Vec::push`, `Box`) and zero blocking locks inside the audio callback loop.
- VoiceMeeter integration: Configurable input (Vention USB / VoiceMeeter Out B1/B2) and output (`VoiceMeeter AUX Input`).
- All 9 core effects implemented from scratch in Rust (no external VST plugins).

---

## Phases & Tasks

### Phase 1: Project Foundation & Device Enumeration
- [ ] Initialize Rust Cargo project with `Cargo.toml` (`cpal`, `rtrb`, `ratatui`, `crossterm`, `serde`, `serde_json`).
- [ ] Implement `src/audio/device.rs` to enumerate capture and playback devices with indices and names.
- [ ] Implement CLI flags in `src/main.rs`: `--list-devices`, `--version`, `--help`.
- [ ] Unit test: Verify device enumeration and CLI output.

### Phase 2: Audio Engine & Lock-Free Ring Buffer
- [ ] Implement `src/audio/ring_buffer.rs` wrapping `rtrb` for lock-free audio frame exchange.
- [ ] Implement `src/audio/engine.rs` managing WASAPI capture and playback streams via `cpal`.
- [ ] Implement clean audio passthrough from mono capture to stereo playback with silence fallback on underrun.
- [ ] Unit & integration tests: SPSC ring buffer concurrency, underrun counting, and buffer lifecycle.

### Phase 3: Custom DSP Engine (The 9 Core Effects)
- [ ] Implement `src/dsp/traits.rs` defining the `AudioEffect` trait.
- [ ] Implement dynamics & gain: `InputGain` (with smoothing) and `NoiseGate` (with hysteresis).
- [ ] Implement saturation: `Overdrive` (`tanh` soft-clipping) and `Distortion` (diode clipping + 720 Hz pre-emphasis).
- [ ] Implement amp modeling: `AmpSimulator` (Clean, Crunch, Lead voicings + 3-band tone stack + cabinet filter).
- [ ] Implement filtering: `Equalizer` (Direct Form II Transposed biquad Low Shelf, Mid Peaking, High Shelf).
- [ ] Implement spatial: `Delay` (stereo ping-pong + damping) and `Reverb` (Freeverb algorithmic comb/allpass).
- [ ] Implement mastering: `Limiter` (lookahead brickwall peak limiter @ -0.3 dBFS).
- [ ] Implement `src/dsp/chain.rs` (`DSPChain`) coordinating the 9 effects, atomic peak/RMS metering, bypass, and mute.
- [ ] Unit tests: Verify mathematical bounds, limiter ceiling clamp, and noise gate attenuation in `tests/test_dsp.rs`.

### Phase 4: Preset Management & Configuration
- [ ] Implement `src/core/preset.rs` (`PresetManager`) managing JSON serialization of effect parameters.
- [ ] Generate 5 built-in factory presets: `clean.json`, `blues.json`, `crunch.json`, `rock.json`, `ambient.json`.
- [ ] Implement `src/core/config.rs` (`ConfigManager`) managing `config/default.json` for persistent device selection.
- [ ] Unit tests: Validate preset schema, parameter boundaries, and atomic file persistence.

### Phase 5: Interactive Terminal UI (Ratatui + Crossterm)
- [ ] Implement `src/ui/dashboard.rs`: Header (status, sample rate, buffer size, latency badge, XRUNs, active preset).
- [ ] Implement live ASCII VU meter bars for input level and output level.
- [ ] Implement Effect Chain status table (`[ON]` / `[OFF]` badges and parameter summaries).
- [ ] Implement navigation tabs & modals:
  - `[D]` Devices menu (select input Vention/VoiceMeeter B1/B2, output VoiceMeeter AUX).
  - `[E]` Effects menu (toggle bypass, parameter adjustments).
  - `[P]` Presets menu (load factory tones).
  - `[B]` Global bypass, `[M]` Mute, `[Q]` Quit.
- [ ] Non-blocking 30 FPS terminal event loop decoupled from the audio thread.

### Phase 6: Packaging, Release Build & Documentation
- [ ] Cross-compile release binary `guitarfx.exe` for Windows target `x86_64-pc-windows-gnu`.
- [ ] Create 1-click launcher `run.bat`.
- [ ] Package `GuitarFX-v0.1.0-windows-x86_64.zip`.
- [ ] Revamp `README.md` with complete VoiceMeeter Banana routing guide, diagrams, and usage controls.
- [ ] Verify all automated tests pass 100%. Push changes to GitHub.
