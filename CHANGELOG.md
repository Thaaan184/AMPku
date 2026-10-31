# Changelog - AMPku

Semua perubahan penting, minor, dan major dari proyek AMPku dicatat secara teratur di dokumen ini.
Format penomoran versi mengikuti [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0] - 2026-10-31

### Added
- **Audio Core & Device Engine**:
  - Engine audio WASAPI Exclusive / Direct via `cpal` dengan dukungan buffer size 64 hingga 512 sample.
  - Latensi buffer terverifikasi hingga 1.33 ms (64 sample @ 48 kHz).
  - Dukungan Dual Physical Device I/O independen (misal: input USB soundcard Vention + output speaker motherboard Realtek).
  - Resampler kompensasi clock drift adaptif menggunakan PI controller dan interpolasi Hermite cubic tanpa dropout atau klik audio.
  - Komunikasi thread aman bebas lock menggunakan SPSC ring buffer (`rtrb`) dan atomics (`AtomicF32`).
  - Fitur Direct 24-bit 48 kHz WAV Recorder via crate `hound` pada background worker thread non-blocking.
- **DSP Modular Effects Suite**:
  - Trait modular `AudioEffect` dengan arsitektur stereo `AudioFrame { left, right }`.
  - Effect Chain dinamis: reorder posisi efek (up/down), tambah dari dropdown catalog, hapus, dan individual bypass.
  - 10 Efek DSP produksi bawaan:
    1. **Noise Gate**: Threshold, attack, release, dan hysteresis anti-chatter.
    2. **Sustainer Compressor**: Threshold, ratio, attack, release, makeup gain, dan metering gain reduction.
    3. **Overdrive**: Emulasi soft-clipping tabung dengan tone control filter.
    4. **Distortion**: Asymmetric hard-clipping untuk lead tone agresif.
    5. **Fuzz**: Saturasi wavefolding germanium klasik.
    6. **FIR Cabinet Simulator**: Konvolusi FIR impulse response dengan 4 factory profile (Celestion V30, Fender Twin, Marshall 1960A, Acoustic Sim) plus custom WAV IR loader.
    7. **Stereo Delay**: Ping-pong stereo spread, fractional cubic delay, and lowpass feedback damping.
    8. **Studio Reverb**: Freeverb stereo decorrelation dengan kontrol room size, damping, dan stereo width.
    9. **3-Band Parametric Biquad EQ**: Low shelf 80 Hz, peaking mid dengan Q-factor, dan high shelf 5 kHz.
    10. **Vocal Suite & De-Esser**: Low-cut 80 Hz rumble filter, split-band/wideband sibilance suppression, dan listen mode.
- **Mixer & Master Bus**:
  - Dual independent channel strips (Channel 1 Gitar, Channel 2 Vokal Mic) dengan kontrol Gain/Volume dB, Pan matrix, Mute, dan Solo.
  - Stereo VU Meter real-time terintegrasi (Peak & RMS, rentang -60 hingga 0 dBFS) dengan clip indicator.
  - Master Bus Summing dilengkapi brickwall lookahead peak limiter (-0.1 dBFS ceiling) untuk pencegahan clipping digital.
- **Preset System**:
  - 5 Preset pabrik esensial: *Texas Blues Crunch*, *Modern Metal Lead*, *Ambient Shimmer Clean*, *Acoustic Direct & Vocal*, dan *Heavy Rock Rhythm*.
  - Penyimpanan JSON terstruktur dengan proteksi path traversal dan atomic rename file writes (`.tmp` -> `.json`).
- **Modern Studio GUI**:
  - Antarmuka grafis desktop native 60 FPS menggunakan `egui` dan `eframe`.
  - Tema dark studio profesional terinspirasi dari Ableton Live dan Neural DSP.
  - Transport bar lengkap dengan device selector, buffer selector, badge estimasi latensi, serta tombol rekam & indikator durasi.
  - Modular collapsible effect cards dengan kontrol interaktif dan indikator visual.
- **Build & Verification**:
  - Konfigurasi `.cargo/config.toml` untuk cross-compilation Windows target (`x86_64-pc-windows-gnu` MinGW dan `x86_64-pc-windows-msvc`).
  - Verifikasi 166 unit dan integration test cases (100% pass rate).
  - Kebersihan kode 100%: zero warnings pada `cargo clippy --all-targets` dan `cargo fmt`.

---

## [0.0.1] - 2026-10-30

### Initial Architectural Design & Foundation (Milestone 1)
- **Specification**:
  - Diterbitkan spesifikasi desain arsitektur `docs/superpowers/specs/2026-10-30-ampku-design.md`.
  - Penetapan target arsitektur: Dual-thread zero-lock audio engine (Rust `cpal` WASAPI Exclusive + `egui` native GUI).
- **Core Features Designed**:
  - Dual Input Processing: Input Channel 1 (Gitar) & Channel 2 (Mic) independen.
  - Flexible Dual-Device I/O: Dukungan pemilihan input dan output perangkat fisik berbeda dengan adaptive clock drift compensation buffer.
  - Stereo Processing Architecture: Trait `AudioEffect` dengan struktur `AudioFrame { left, right }`.
  - Factory Cabinet IR, Modular DSP Chain, Direct Lossless Recorder, Preset Engine.
- **Documentation**:
  - `README.md` mendalam dengan panduan clone, build standalone release, arsitektur internal, dan tutorial membuat modul DSP efek baru.
