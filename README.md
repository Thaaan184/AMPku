# AMPku 🎸🎙️
> **Zero-Setup, Ultra-Low Latency Guitar & Vocal DSP Studio for Windows**  
> *Dibuat khusus untuk musisi yang ingin colok gitar langsung ke soundcard USB (Vention, dll) tanpa ribet setting driver ASIO4ALL atau membuka DAW berat.*

---

## ⚡ Kenapa AMPku? (The Why)
Main gitar listrik colok soundcard USB di Windows seringkali menyebalkan:
- **DAW terlalu berat & lambat**: Membuka FL Studio, Ableton, atau Reaper hanya untuk latihan 15 menit terasa berlebihan dan memakan RAM besar.
- **ASIO4ALL bikin pusing**: Sering conflict dengan browser YouTube/Spotify, setting buffer membingungkan, dan sering bisu (audio drop).
- **Direct Monitoring soundcard suaranya mentah**: Soundcard budget (seperti Vention) hanya punya direct monitor sinyal *dry* (suara senar mentah tanpa distorsi/amp simulator).

**AMPku** memotong semua keribetan itu:
1. **WASAPI Exclusive Low Latency**: Langsung akses hardware audio Windows dengan buffer 64 sample (~1.3 ms) tanpa perlu driver pihak ketiga.
2. **Dual Input Independen**: Channel 1 untuk Gitar, Channel 2 untuk Mic Vokal (masing-masing punya fader dan modular rack efek terpisah).
3. **Dual Device I/O**: Bebas gunakan input soundcard USB (Vention) dan output speaker motherboard (Realtek) secara bersamaan dengan kompensasi clock drift otomatis.
4. **10 Modular DSP Effects**: Overdrive, Distortion, Fuzz, Noise Gate, Compressor, 4 Factory Cabinet IRs, Delay, Reverb, EQ, dan Vocal Suite/De-Esser.
5. **Direct WAV Recorder**: Rekam jamming atau ide lagu dalam format 24-bit PCM WAV lossless dengan satu tombol klik.

---

## 🚀 Cara Menjalankan AMPku (1-Click Run & Auto-Install)

AMPku menyediakan script launcher otomatis untuk memasang dependensi, meng-compile, dan menjalankan aplikasi secara lokal di Windows.

### Cara Menjalankan di Windows:

1. **Download / Clone Repository ini:**
   - Klik tombol hijau **`Code`** ➔ **`Download ZIP`** di GitHub (atau gunakan `git clone https://github.com/Thaaan184/AMPku.git`).
   - Ekstrak file zip ke folder mana saja di komputer kamu.
2. **Double-click file `run.bat`:**
   - Cukup **double-click** file **`run.bat`** (atau buka PowerShell lalu jalankan `./run.ps1`).
3. **Selesai!** Script otomatis:
   - Memeriksa instalasi Rust/Cargo di Windows kamu.
   - Jika Rust belum terpasang, script akan otomatis mengunduh installer resmi Rust (`rustup-init.exe`) dan memasangnya.
   - Mengunduh dependensi DSP & GUI, lalu meng-compile binary lokal dengan akselerasi hardware DirectX 12/11 (`WGPU`).
   - Membuka jendela aplikasi **AMPku Studio GUI**.

*(Catatan: Kompilasi pertama kali membutuhkan waktu sekitar 1–2 menit untuk download crate. Jalankan berikutnya akan instan dalam 2 detik).*

---

## 🎛️ Quick Start Panduan Penggunaan
1. Colok soundcard USB (misal Vention) ke PC Windows 10/11.
2. Colok kabel gitar ke Input 1 (Left), dan mic ke Input 2 (Right).
3. Jalankan `run.bat`.
4. Pada bar atas (**Transport Bar**), pilih:
   - **Input Device**: *Vention USB Audio*
   - **Output Device**: *Speakers / Headphone (Realtek / USB)*
   - **Buffer Size**: *64 samples* (untuk latensi terendah ~1.3 ms) atau *128 samples*.
5. Pilih preset favorit (misal: `Texas Blues` atau `Clean Ambient`) dan mulai main!

---

## 📊 Matriks Fitur & Karakteristik Latensi (v0.1.0)

### 1. Karakteristik Latensi Terverifikasi (@ 48 kHz)
| Buffer Size | One-Way Buffer Latency | Estimasi Roundtrip Latency | Kategori Penggunaan |
|---|---|---|---|
| **64 samples** | **1.33 ms** | **~3.2 – 4.5 ms** | Real-time shredding, lead guitar, ultra-fast response |
| **128 samples** | **2.67 ms** | **~5.5 – 7.0 ms** | Standard playing & vocal monitoring (sangat aman) |
| **256 samples** | **5.33 ms** | **~11.0 – 13.0 ms** | Sesi santai / CPU hemat daya |
| **512 samples** | **10.67 ms** | **~21.0 – 24.0 ms** | High stability buffer |

### 2. Feature Matrix
| Kategori | Fitur | Status | Keterangan |
|---|---|---|---|
| **Audio Core** | WASAPI Exclusive / Direct | ✅ Selesai | Akses hardware direct stream via `cpal` |
| | Dual Physical Device I/O | ✅ Selesai | Input & output pada perangkat fisik terpisah |
| | Adaptive Clock Drift Resampler | ✅ Selesai | PI controller + Hermite cubic interpolation |
| | Lock-Free Concurrency | ✅ Selesai | `rtrb` SPSC ring buffers & `AtomicF32` parameter sync |
| | Direct 24-bit WAV Recorder | ✅ Selesai | Background writer thread via `hound`, non-blocking I/O |
| **DSP Rack** | Stereo Processing Architecture | ✅ Selesai | `AudioFrame { left, right }` dengan spatial imaging |
| | Noise Gate | ✅ Selesai | Threshold, attack, release, hysteresis, zero bleed |
| | Sustainer Compressor | ✅ Selesai | Threshold, ratio, attack, release, makeup gain |
| | Overdrive | ✅ Selesai | Soft-clipping diode saturation + tone lowpass filter |
| | Distortion | ✅ Selesai | Asymmetric hard-clipping 1N4148 waveshaping |
| | Fuzz | ✅ Selesai | Transistor wavefolding saturation |
| | FIR Cabinet Simulator | ✅ Selesai | 4 Embedded Factory IRs (Celestion V30, Fender Twin, Marshall 1960A, Acoustic Sim) |
| | Stereo Delay | ✅ Selesai | Ping-pong stereo, fractional cubic delay, damping |
| | Studio Reverb | ✅ Selesai | Freeverb stereo decorrelation, room size, damping |
| | 3-Band Parametric EQ | ✅ Selesai | Low shelf, peaking mid (Q-factor), high shelf biquad |
| | Vocal Suite & De-Esser | ✅ Selesai | 80Hz HPF, split-band / wideband sibilance reduction |
| **Mixer & GUI** | Dual Channel Strips | ✅ Selesai | Channel 1 (Gitar) & Channel 2 (Mic), fader, pan, mute, solo |
| | Real-Time Stereo VU Meters | ✅ Selesai | Segmented LED bar (-60 to +3 dBFS), clip indicator |
| | Master Bus & Brickwall Limiter | ✅ Selesai | Master fader + soft-knee lookahead peak limiter |
| | Modular Card Reordering | ✅ Selesai | Reorder up/down, add from dropdown, delete, individual bypass |
| | Dark Studio Theme | ✅ Selesai | egui/eframe 60 FPS non-blocking repaint loop |
| **Presets** | Preset Engine (JSON) | ✅ Selesai | 5 Factory Presets, user preset save/load/delete |
| | Security & Integrity | ✅ Selesai | Path traversal rejection, safe atomic rename writes |
| **Testing** | 154 Test Cases Passing | ✅ Selesai | 100% pass rate di semua unit dan integration tests |

---

## 🛠️ Panduan Developer: Menjalankan Manual & Modifikasi

### Menjalankan Manual via Terminal:
```bash
# Menjalankan langsung aplikasi:
cargo run --release

# Menjalankan seluruh test suite otomatis:
cargo test --all-targets

# Menjalankan linter kode:
cargo clippy --all-targets
```

### Tutorial: Menambahkan Efek DSP Kustom Baru
Semua efek di AMPku menerapkan trait tunggal `AudioEffect` di `src/dsp/traits.rs`. Anda bisa membuat modul efek baru dengan sangat mudah:

```rust
use crate::core::frame::AudioFrame;
use crate::dsp::traits::AudioEffect;
use std::any::Any;

pub struct HardClipDistortion {
    pub gain: f32,
    pub threshold: f32,
    pub bypass: bool,
}

impl AudioEffect for HardClipDistortion {
    fn name(&self) -> &'static str {
        "My Custom Distortion"
    }

    fn process_frame(&mut self, input: AudioFrame) -> AudioFrame {
        if self.bypass {
            return input;
        }

        AudioFrame {
            left: (input.left * self.gain).clamp(-self.threshold, self.threshold),
            right: (input.right * self.gain).clamp(-self.threshold, self.threshold),
        }
    }

    fn reset(&mut self) {}

    fn is_enabled(&self) -> bool {
        !self.bypass
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.bypass = !enabled;
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
```

Daftarkan modul efek baru di `src/dsp/mod.rs` dan selector menu di `src/gui/effect_card.rs`.

---

## 🧠 Arsitektur Sistem Real-Time (Dual-Thread Lock-Free)

```
[ GUI Thread (egui / 60 FPS) ]
       │  ▲
       │  │ (Lock-free Ring Buffer / Atomics)
       ▼  │
[ Audio Callback Thread (cpal / WASAPI Exclusive) ]
       │
       ├── Channel 1: Input Gitar ──> [ Modular DSP Chain Gitar ] ──┐
       │                                                            ├──> [ Master Sum & Limiter ] ──> Output Speaker
       └── Channel 2: Input Mic   ──> [ Modular DSP Chain Mic ]   ──┘                 │
                                                                                      └──> [ WAV Recorder (hound) ]
```

- **Zero Allocation**: Audio loop callback tidak pernah melakukan heap allocation (`Vec::push`, `Box`, `String`).
- **Zero Blocking**: Komunikasi GUI ke thread audio menggunakan non-blocking try-lock dan register atomik (`AtomicF32`).
- **Clock Drift Adaptive**: Menghilangkan audio crackle/xrun jika input soundcard USB dan output motherboard memiliki crystal oscillator fisik berbeda.

---

## 📜 Lisensi
MIT License - Bebas digunakan, dimodifikasi, dan didistribusikan.
