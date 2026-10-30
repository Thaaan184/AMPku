# AMPku - Standalone Low-Latency Guitar & Vocal DSP Studio

## 1. Intent & The Why
Musicians and guitarists using budget USB soundcards (e.g., Vention, generic 2-channel audio interfaces) face friction when practicing, jamming, or recording quick takes on Windows:
- Professional DAWs (FL Studio, Reaper, Ableton) are heavy, slow to launch, and require complex project setup.
- Third-party virtual drivers like ASIO4ALL are finicky, frequently conflict with system audio, and confuse non-technical users.
- Hardware direct monitoring on entry-level cards yields only dry, raw, uninspired guitar signals without amplifier or pedal distortion.

**AMPku** solves this by providing an ultra-lightweight, standalone Windows desktop application (under 15 MB) written in Rust. It utilizes Windows WASAPI Exclusive Mode for sub-3ms round-trip latency without needing ASIO4ALL, delivering immediate guitar amplification, comprehensive multi-effects, dual-input processing (Guitar + Mic), and one-click direct WAV recording.

---

## 2. Goals, Non-Goals, and Anti-Goals

### 2.1 Goals
- **Instant Plug-and-Play**: Launch the single executable, select the Vention soundcard, and play immediately.
- **Ultra-Low Latency**: Leverage `cpal` WASAPI Exclusive mode (64 or 128 buffer frames @ 48 kHz / 44.1 kHz) achieving ~1.3 ms to 2.7 ms buffer latency.
- **Dual Independent Channels**:
  - Channel 1 (Left): Guitar with dedicated guitar DSP chain.
  - Channel 2 (Right): Vocal/Mic with dedicated voice optimization chain.
- **Modular Serial Effect Rack**:
  - Add, remove, bypass, and rearrange DSP effect nodes dynamically per channel.
  - Full parameter adjustment (sliders, knobs, wet/dry balance).
- **Comprehensive Guitar DSP Suite**:
  - Dynamics: Noise Gate, Sustainer Compressor, Parametric/Graphic EQ.
  - Saturation: Overdrive (soft clip), Distortion (hard clip), Fuzz (transistor saturation), Clean Boost.
  - Speaker/Amp Emulation: Cabinet Impulse Response (FIR convolution engine).
  - Modulation: Chorus, Flanger, Phaser, Tremolo.
  - Time & Space: Digital/Analog Delay, Reverb (Room, Hall, Shimmer).
- **Vocal DSP Suite**:
  - High-Pass Filter (80Hz cut), Vocal De-Esser, Noise Gate, Vocal Compressor, Ambience Reverb.
- **Master Bus & Direct WAV Recorder**:
  - Real-time stereo metering (peak & RMS VU meters).
  - Master limiter to prevent digital clipping/distortion.
  - Lossless 24-bit / 48 kHz WAV recording directly to disk via `hound`.
- **Preset Management**:
  - Load and save custom tone presets via structured JSON files.
  - Factory presets included (e.g., Clean Ambient, Texas Blues, High-Gain Lead, Acoustic Chime, Podcaster Mic).
- **Minimalist Studio Utility UI**:
  - Clean, dark-mode, high-contrast interface built with `egui` (eframe).
  - Ultra-low CPU/GPU rendering footprint to preserve CPU cycles for audio DSP.
- **Developer-Friendly & Extensible**:
  - Comprehensive `README.md` detailing how to clone, build, modify, add custom DSP effect traits, and customize presets.
  - Strict changelog (`CHANGELOG.md`) tracking minor and major revisions.

### 2.2 Non-Goals
- Multi-track timeline arrangements, MIDI sequencing, or piano-roll editing (users should use a dedicated DAW if complex multi-track arranging is needed).
- Hosting third-party VST3/CLAP plugins in version 1.0 (all DSP nodes are native, deterministic Rust code for stability and zero external crashes).
- Mobile or web deployment in version 1.0 (targeted specifically at Windows 10/11 desktop audio architecture).

### 2.3 Anti-Goals
- **No External Driver Dependencies**: Must never force users to download ASIO4ALL, Jack, or complex virtual audio cables.
- **No Real-Time Audio Dropouts**: The audio callback thread must never allocate memory (`malloc`), make system calls, or lock mutexes that could cause buffer underruns (xruns).
- **No Bloat / Slop**: No heavy Electron/Chromium runtime, no intrusive analytics, no background telemetry.

---

## 3. System Architecture & Audio Pipeline

### 3.1 Audio Thread vs UI Thread Separation
The application is split into two isolated, communicating worlds:
```
+-------------------------------------------------------------------+
|                        GUI Thread (egui)                          |
|  - Renders 60 FPS controls, faders, meters, preset dialogs        |
|  - Communicates with Audio Thread via Lock-Free RingBuffer/Atomics|
+---------------------------------+---------------------------------+
                                  | Parameters / Commands / Peak RMS
                                  v
+-------------------------------------------------------------------+
|                   Audio Engine Thread (cpal)                      |
|  - High-priority real-time thread (WASAPI Exclusive)              |
|  - Fixed buffer callback (e.g., 64 samples @ 48 kHz)              |
|  - Zero heap allocation during processing loop                    |
+---------------------------------+---------------------------------+
                                  |
            +---------------------+---------------------+
            v                                           v
  [Input Ch 1: Guitar]                        [Input Ch 2: Mic]
            |                                           |
  +---------v---------+                       +---------v---------+
  | Modular Guitar    |                       | Modular Mic       |
  | DSP Node Chain    |                       | DSP Node Chain    |
  | (Gate, OD, Cab...) |                       | (HPF, Gate, Comp) |
  +---------+---------+                       +---------+---------+
            |                                           |
            \---------------------+---------------------/
                                  |
                                  v
                         [Stereo Sum Bus]
                                  |
                     +------------+------------+
                     |                         |
                     v                         v
            [Master Limiter]          [WAV Record Buffer]
                     |                         |
                     v                         v
            [WASAPI Output Stream]       [Disk File (.wav)]
```

### 3.2 Lock-Free Communication
- Parameters changed in the UI (e.g., Gain slider moved from 5.0 to 7.0) are communicated to DSP nodes using atomic floating-point values (`AtomicF32` or smoothed parameter transitions) to eliminate zipper noise without locking the audio thread.
- VU meter readouts and recording audio streams are pushed to a lock-free Single-Producer Single-Consumer (SPSC) ring buffer (`rtrb` or `crossbeam-channel`).

### 3.3 DSP Effect Trait Architecture
Every effect in AMPku implements the unified `AudioEffect` trait:
```rust
pub trait AudioEffect: Send {
    fn name(&self) -> &'static str;
    fn process_sample(&mut self, input: f32) -> f32;
    fn process_block(&mut self, buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            *sample = self.process_sample(*sample);
        }
    }
    fn set_parameter(&mut self, id: u32, value: f32);
    fn get_parameter(&self, id: u32) -> f32;
    fn is_bypassed(&self) -> bool;
    fn set_bypass(&mut self, bypass: bool);
    fn reset(&mut self);
}
```

---

## 4. User Interface Specification (Minimalist Studio Utility)

The UI follows Option A (Dark Modern Studio Utility):
1. **Header / Audio Hardware Bar**:
   - Audio Input Device Selector (e.g., `Microphone (Vention USB Audio)`).
   - Audio Output Device Selector (e.g., `Speakers (Realtek High Definition Audio)`).
   - Sample Rate selector (`44100 Hz`, `48000 Hz`).
   - Buffer Size selector (`64 samples (1.3ms)`, `128 samples (2.7ms)`, `256 samples (5.3ms)`).
   - Status badge: Active / Latency indicator.
2. **Preset Bar**:
   - Preset selector dropdown.
   - `[Save Preset]`, `[Save As...]`, `[Load...]`, `[Reset Default]` buttons.
3. **Dual Channel Strips (Split View)**:
   - **Left Column (Channel 1 - Guitar)**:
     - Channel Gain fader, Pan slider, Mute, Solo.
     - Live Input VU meter (dB scale with clipping indicator).
     - Modular FX Rack:
       - List of active effect cards (collapsible).
       - Card header: Effect Name, Power Toggle (Bypass), Remove `[X]`, Move Up `[^]`, Move Down `[v]`.
       - Card body: Parameter sliders with exact numeric text inputs (e.g., Drive: 6.5, Tone: 4.2 kHz).
       - Bottom button: `[+ Add Effect to Guitar]` popup listing all categories.
   - **Right Column (Channel 2 - Mic)**:
     - Channel Gain fader, Pan slider, Mute, Solo.
     - Live Input VU meter.
     - Modular FX Rack for vocal chain.
     - Bottom button: `[+ Add Effect to Mic]`.
4. **Master & Transport Footer**:
   - Master Volume Fader (-inf to +6 dB).
   - Stereo Master Output VU meter (Left & Right).
   - Direct WAV Recorder Control:
     - Red Record toggle button `[● REC]`.
     - Time counter (`00:02:45`).
     - Recording output path indicator and `[Open Folder]` button.

---

## 5. Persistence & Preset Schema
Presets are saved as human-readable JSON files in `%APPDATA%/AMPku/presets/` or relative to the executable:
```json
{
  "version": 1,
  "name": "British Crunch & Vocal",
  "guitar_channel": {
    "volume": 0.85,
    "pan": 0.0,
    "effects": [
      {
        "type": "NoiseGate",
        "bypass": false,
        "params": { "threshold_db": -50.0, "release_ms": 100.0 }
      },
      {
        "type": "Overdrive",
        "bypass": false,
        "params": { "drive": 0.65, "tone": 0.5, "level": 0.8 }
      },
      {
        "type": "CabSimIR",
        "bypass": false,
        "params": { "profile": "Celestion_V30_4x12", "mix": 1.0 }
      },
      {
        "type": "Delay",
        "bypass": true,
        "params": { "time_ms": 350.0, "feedback": 0.3, "mix": 0.2 }
      }
    ]
  },
  "mic_channel": {
    "volume": 0.9,
    "pan": 0.0,
    "effects": [
      {
        "type": "HighPassFilter",
        "bypass": false,
        "params": { "cutoff_hz": 80.0 }
      },
      {
        "type": "VocalCompressor",
        "bypass": false,
        "params": { "threshold_db": -18.0, "ratio": 3.0, "makeup_gain": 2.0 }
      }
    ]
  }
}
```

---

## 6. Project Layout & Repository Architecture
The repository will contain:
- `Cargo.toml`: Workspace and dependencies (`cpal`, `eframe`, `egui`, `hound`, `rtrb`, `serde`, `serde_json`).
- `src/`:
  - `main.rs`: Application entry point, window init, event loop.
  - `audio/`:
    - `engine.rs`: CPAL stream setup, WASAPI Exclusive routing.
    - `ring_buffer.rs`: SPSC lock-free audio buffer.
    - `recorder.rs`: Hound WAV writer background thread.
  - `dsp/`:
    - `traits.rs`: `AudioEffect` trait and parameter representations.
    - `chain.rs`: Modular serial node execution.
    - `filters/`: Biquad HPF, LPF, Peaking EQ.
    - `dynamics/`: Noise gate, compressor.
    - `saturation/`: Overdrive, distortion, fuzz.
    - `time/`: Delay, reverb.
    - `cab_sim/`: FIR IR convolution engine.
  - `gui/`:
    - `app.rs`: Egui main view and rendering pipeline.
    - `channel_strip.rs`: Strip UI component.
    - `effect_card.rs`: Modular card renderer with sliders.
    - `meter.rs`: Real-time VU meter widget.
  - `presets/`:
    - `manager.rs`: JSON serialization, preset loading/saving.
    - `defaults.rs`: Built-in factory presets.
- `README.md`: Comprehensive guide for users and developers (compiling from source, adding effects, architecture).
- `CHANGELOG.md`: Strict change tracking across minor and major milestones.

---

## 7. What is Left to the Builder
- Selection of specific FIR convolution math library or direct SIMD partitioned convolution for zero-latency IR processing.
- Exact styling theme parameters in `egui` (exact dark theme hex values and border radiuses).
- Default preset audio calibration curves.
