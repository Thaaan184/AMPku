# Changelog - GuitarFX

Semua perubahan penting dari proyek GuitarFX dicatat secara teratur di dokumen ini.
Format penomoran versi mengikuti [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0] - 2026-10-31

### Arsitektur Baru: C++20 Lightweight Terminal Audio Engine
- **Mesin Audio (miniaudio + WASAPI)**:
  - Mengganti arsitektur GUI berat dengan C++20 terminal engine interaktif (`FTXUI`).
  - Audio backend WASAPI via single-header `miniaudio.h` dengan latency ~10 ms (buffer 256 samples @ 48 kHz).
  - Mode duplex audio capture & playback dengan fallback silence saat underrun.
  - Enumerasi device dinamis via `AudioDeviceManager`.
- **Integrasi VoiceMeeter Banana**:
  - Dukungan fleksibel untuk input langsung dari Vention USB Sound Card atau bus VoiceMeeter Virtual Out (B1 / B2).
  - Output langsung terarah ke VoiceMeeter AUX Input (`Voicemeeter Aux VAIO`) untuk distribusi ke headphone A1 dan Discord/recording B1.
- **9 Efek DSP Kustom Mandiri (Zero VST / Zero External Dependency)**:
  - Input Gain (penguatan sinyal dengan smoothing parameter).
  - Noise Gate (threshold dB dengan 2 dB histeresis anti-chattering).
  - Overdrive (soft clipping `tanh` + tone lowpass filter).
  - Distortion (asymmetric diode clipping 1N4148 + 720 Hz pre-emphasis).
  - Amp Simulator (3 voicing Clean/Crunch/Lead + tone stack 3-band + 4.8 kHz cabinet filter).
  - Equalizer (Direct Form II Transposed biquad filters: Low Shelf, Peaking Mid, High Shelf).
  - Delay (stereo ping-pong delay line dengan feedback damping).
  - Reverb (Freeverb/Schroeder algorithmic reverb: 8 comb + 4 allpass per kanal).
  - Output Limiter (soft-knee lookahead brickwall peak limiter pada ceiling -0.3 dBFS).
- **Terminal UI Interaktif (FTXUI)**:
  - Layar Dashboard studio dengan meteran VU bar real-time, estimasi latensi, CPU, dan status XRUN.
  - Menu pemilihan audio input/output device interaktif (`[D]`).
  - Menu kontrol efek dan toggle `[ON]` / `[OFF]` (`[E]`, `Space`).
  - Menu preset tone gitar (`[P]`).
  - Hotkey bypass global (`[B]`) dan mute (`[M]`).
- **Sistem Preset JSON**:
  - 5 preset bawaan pabrik: `clean.json`, `blues.json`, `crunch.json`, `rock.json`, `ambient.json`.
- **Packaging & Build**:
  - Konfigurasi CMake 3.20+ terpadu dengan FetchContent FTXUI.
  - Script build Windows otomatis: `run.bat` dan `build.bat`.
  - Binary portable tunggal Windows x64 (`guitarfx.exe`, 1.7 MB).
