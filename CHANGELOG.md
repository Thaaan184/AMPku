# Changelog - AMPku

Semua perubahan penting, minor, dan major dari proyek AMPku dicatat secara teratur di dokumen ini.
Format penomoran versi mengikuti [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased] - 2026-10-30

### Initial Architectural Design & Foundation (Milestone 1)
- **Specification**:
  - Diterbitkan spesifikasi desain arsitektur `docs/superpowers/specs/2026-10-30-ampku-design.md`.
  - Penetapan target arsitektur: Dual-thread zero-lock audio engine (Rust `cpal` WASAPI Exclusive + `egui` native GUI).
- **Core Features Designed**:
  - **Dual Input Processing**: Input Channel 1 (Gitar) & Channel 2 (Mic) independen.
  - **Flexible Dual-Device I/O**: Dukungan pemilihan input dan output perangkat fisik berbeda (Vention + Motherboard Realtek) dengan adaptive clock drift compensation buffer.
  - **Stereo Processing Architecture**: Trait `AudioEffect` dengan struktur `AudioFrame { left, right }` dan toggle `Mono / Stereo` untuk modul spasial (Delay/Reverb stereo spread).
  - **Embedded Factory Cabinet IR**: 4 profil IR esensial langsung di-embed ke dalam binary (Celestion V30, Fender Twin, Marshall 1960A, Acoustic Cab).
  - **Modular DSP Chain**: Serial node graph dengan trait `AudioEffect` yang dapat ditambah, dihapus, dan di-bypass secara dinamis.
  - **Comprehensive Effect Suite**:
    - Dynamics: Noise Gate, Sustainer Compressor.
    - Saturation: Overdrive (soft-clip), Distortion (hard-clip), Fuzz.
    - Cabinet IR: FIR Convolution untuk speaker box (Celestion V30, Fender 1x12).
    - Time-Based: Echo Delay, Room/Hall/Shimmer Reverb.
    - Vocal Suite: 80Hz High-Pass Filter, Vocal Gate, De-Esser, Vocal Comp.
  - **Direct Lossless Recorder**: Background writer thread 24-bit 48 kHz WAV via crate `hound`.
  - **Preset Engine**: Serialisasi state ke JSON manusiawi (`presets/manager.rs`).
- **Documentation**:
  - `README.md` mendalam dengan panduan clone, build standalone release, arsitektur internal, dan tutorial membuat modul DSP efek baru.
