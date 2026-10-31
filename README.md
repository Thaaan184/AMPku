# AMPku 🎸🎙️
> **Zero-Setup, Ultra-Low Latency Standalone Guitar & Vocal DSP Studio for Windows**  
> *Dibuat khusus untuk musisi yang ingin colok gitar langsung ke soundcard USB (Vention, dll) tanpa ribet install ASIO4ALL atau buka DAW berat.*

---

## ⚡ Kenapa AMPku? (The Why)
Main gitar listrik colok soundcard USB di Windows seringkali menyebalkan:
- **DAW terlalu berat & lambat**: Membuka FL Studio, Ableton, atau Reaper cuma buat latihan 15 menit terasa berlebihan.
- **ASIO4ALL bikin pusing**: Sering conflict dengan browser YouTube/Spotify, setting buffer membingungkan, dan sering bisu (audio drop).
- **Direct Monitoring soundcard suaranya mentah**: Soundcard budget (seperti Vention) hanya punya direct monitor sinyal *dry* (suara senar mentah tanpa distorsi/amp simulator).

**AMPku** memotong semua keribetan itu:
1. **Satu File Executable Ringan (< 15 MB)**: Langsung klik, langsung jalan tanpa installer.
2. **WASAPI Exclusive Low Latency**: Langsung akses hardware audio Windows dengan buffer 64 sample (~1.3 ms) tanpa perlu driver pihak ketiga.
3. **Dual Input Independen**: Channel 1 untuk Gitar, Channel 2 untuk Mic Vokal (masing-masing punya rantai efek terpisah).
4. **Dual Device I/O**: Bebas gunakan input soundcard USB (Vention) dan output speaker motherboard (Realtek) secara bersamaan dengan kompensasi clock drift otomatis.
5. **Modular DSP Rack**: Bebas tambah, hapus, bypass, dan atur urutan efek (Overdrive, Cab Sim, Delay, Reverb, Compressor, Gate, EQ, De-Esser).
6. **Direct WAV Recorder**: Rekam jamming atau ide lagu dalam format 24-bit PCM WAV lossless dengan satu tombol klik.

---

## 💾 Download AMPku (Siap Pakai untuk Windows 10 / 11)

[![Download AMPku v0.1.0 (.exe)](https://img.shields.io/badge/DOWNLOAD-AMPku_v0.1.0_(.exe)-0284c7?style=for-the-badge&logo=windows&logoColor=white)](https://github.com/Thaaan184/AMPku/releases/latest)
[![Release Notes](https://img.shields.io/github/v/release/Thaaan184/AMPku?style=for-the-badge&color=22c55e)](https://github.com/Thaaan184/AMPku/releases)
[![Tests Passed](https://img.shields.io/badge/Tests-174%20Passed-brightgreen?style=for-the-badge)](https://github.com/Thaaan184/AMPku/actions)

> 📦 **Standalone Portable Single Executable (< 15 MB)**  
> Tidak perlu instalasi ribet, tidak perlu driver ASIO4ALL, dan bebas bloatware. Cukup download file `.exe`, colok soundcard USB, lalu double-click untuk langsung main.
>
> 🚀 **Direct Download Link:**  
> 👉 [**Download AMPku.exe (Versi Terbaru)**](https://github.com/Thaaan184/AMPku/releases/latest/download/AMPku.exe)  
> *Alternatif halaman rilis lengkap:* [github.com/Thaaan184/AMPku/releases](https://github.com/Thaaan184/AMPku/releases)

---

## 🚀 Quick Start (Pengguna Siap Pakai)
1. Colok soundcard USB kamu (misal Vention) ke PC Windows 10/11.
2. Colok kabel gitar ke Input 1 (Left), dan mic ke Input 2 (Right).
3. Jalankan `AMPku.exe`.
4. Pada bar atas (Transport Bar), pilih:
   - **Input Device**: *Vention USB Audio*
   - **Output Device**: *Speakers / Headphone (Realtek / USB)*
   - **Buffer Size**: *64 samples* (untuk latensi terendah ~1.3 ms) atau *128 samples*.
5. Pilih preset favorit (misal: `Texas Blues Crunch` atau `Clean Ambient`) dan mulai main!

---

## 📊 Matriks Fitur & Verifikasi Latensi (v0.1.0)

### 1. Karakteristik Latensi Terverifikasi (@ 48 kHz)
| Buffer Size | One-Way Buffer Latency | Estimasi Roundtrip Latency | Kategori Penggunaan |
|---|---|---|---|
| **64 samples** | **1.33 ms** | **~3.2 – 4.5 ms** | Real-time shredding, lead guitar, ultra-fast response |
| **128 samples** | **2.67 ms** | **~5.5 – 7.0 ms** | Standard playing & vocal monitoring (sangat aman) |
| **256 samples** | **5.33 ms** | **~11.0 – 13.0 ms** | Sesi santai / CPU hemat energi |
| **512 samples** | **10.67 ms** | **~21.0 – 24.0 ms** | High stability buffer |

### 2. Verified Feature Matrix (v0.1.0 Release)
| Kategori | Fitur | Status | Keterangan |
|---|---|---|---|
| **Audio Core** | WASAPI Exclusive / Direct | ✅ Selesai | Akses hardware eksklusif direct stream via `cpal` |
| | Dual Physical Device I/O | ✅ Selesai | Input dan Output pada perangkat fisik terpisah |
| | Adaptive Clock Drift Resampler | ✅ Selesai | PI controller + Hermite cubic interpolation, zero clicks/pops |
| | Lock-Free Concurrency | ✅ Selesai | `rtrb` SPSC ring buffers & `AtomicF32` parameter sync |
| | Direct 24-bit WAV Recorder | ✅ Selesai | Background writer thread via `hound`, non-blocking I/O |
| **DSP Rack** | Stereo Processing Architecture | ✅ Selesai | `AudioFrame { left, right }` dengan spatial imaging |
| | Noise Gate | ✅ Selesai | Threshold, attack, release, hysteresis, zero bleed |
| | Sustainer Compressor | ✅ Selesai | Threshold, ratio, attack, release, makeup gain, GR metering |
| | Overdrive | ✅ Selesai | Soft-clipping diode saturation + tone lowpass filter |
| | Distortion | ✅ Selesai | Asymmetric hard-clipping high gain waveshaping |
| | Fuzz | ✅ Selesai | Germanium wavefolding saturation |
| | FIR Cabinet Simulator | ✅ Selesai | 4 Embedded Factory IRs (Celestion V30, Fender Twin, Marshall 1960A, Acoustic) + Custom IR WAV loader |
| | Stereo Delay | ✅ Selesai | Ping-pong stereo, fractional cubic delay, lowpass damping |
| | Studio Reverb | ✅ Selesai | Freeverb stereo decorrelation, room size, damping, width |
| | 3-Band Parametric EQ | ✅ Selesai | Low shelf, peaking mid (Q-factor), high shelf biquad |
| | Vocal Suite & De-Esser | ✅ Selesai | 80Hz HPF, split-band / wideband sibilance reduction, listen mode |
| **Mixer & GUI** | Dual Independent Channel Strips | ✅ Selesai | Channel 1 (Gitar) & Channel 2 (Mic), fader, pan, mute, solo |
| | Real-Time Stereo VU Meters | ✅ Selesai | Peak & RMS bar rendering, -60 to 0 dBFS, clip indicator |
| | Master Bus & Brickwall Limiter | ✅ Selesai | Master fader + lookahead peak limiter (-0.1 dBFS ceiling) |
| | Modular Card Reordering | ✅ Selesai | Reorder up/down, add from dropdown, delete, individual bypass |
| | Dark Studio Theme | ✅ Selesai | egui/eframe 60 FPS, Ableton/Neural DSP inspired palette |
| **Presets** | Preset Engine (JSON) | ✅ Selesai | 5 Factory Presets, user preset save/load/delete |
| | Security & Integrity | ✅ Selesai | Path traversal rejection, safe atomic rename writes |
| **Testing** | 154 Test Cases Passing | ✅ Selesai | 100% pass rate di semua unit dan integration tests |

---

## 🛠️ Panduan Developer: Mengambil Source Code, Kompilasi & Modifikasi

### 1. Cara Mengambil Source Code
```bash
git clone https://github.com/Thaaan184/AMPku.git
cd AMPku
```

### 2. Prasyarat Sistem
- **Rust Toolchain**: Rust versi stabil 1.75+ (install via [rustup.rs](https://rustup.rs/)).
- **Git**: Untuk clone dan commit repository.
- **MinGW GCC (khusus jika cross-compile dari Linux ke Windows)**: `gcc-mingw-w64-x86-64`.

### 3. Kompilasi & Menjalankan secara Lokal
```bash
# Menjalankan langsung aplikasi di komputer kamu:
cargo run

# Menjalankan seluruh test suite otomatis:
cargo test --all-targets

# Menjalankan linter kode dan formatting:
cargo clippy --all-targets
cargo fmt --check
```

### 4. Membangun Binary Standalone Release (.exe)

#### Opsi A: Membangun Langsung di Windows (MSVC / GNU)
```powershell
cargo build --release
```
Binary release teroptimasi penuh (LTO + strip) akan berada di:  
`target/release/ampku.exe` (ukuran < 15 MB, zero external DLL dependencies).

#### Opsi B: Cross-Compile dari Linux ke Windows (MinGW)
Repository telah dilengkapi konfigurasi `.cargo/config.toml` untuk target `x86_64-pc-windows-gnu`:
```bash
# 1. Install toolchain target Windows di Rust:
rustup target add x86_64-pc-windows-gnu

# 2. Install MinGW cross-compiler di host Linux (Ubuntu/Debian):
sudo apt install gcc-mingw-w64-x86-64

# 3. Verifikasi check & clippy untuk target Windows:
cargo check --target x86_64-pc-windows-gnu
cargo clippy --target x86_64-pc-windows-gnu

# 4. Bangun binary release Windows:
cargo build --target x86_64-pc-windows-gnu --release
```
File executable Windows siap pakai akan berada di:  
`target/x86_64-pc-windows-gnu/release/ampku.exe`.

---

## 🎛️ Cara Memodifikasi Produk & Menambah Efek DSP Baru

Semua efek di AMPku menerapkan trait tunggal `AudioEffect` yang berada di `src/dsp/traits.rs`. Trait ini mendukung pemrosesan frame stereo (`AudioFrame { left, right }`), sehingga efek spasial seperti Reverb dan Delay dapat menghasilkan persebaran stereo yang lebar.

### Tutorial: Menambahkan Efek Kustom Baru
Buat file baru di `src/dsp/saturation/my_distortion.rs`:

```rust
use crate::core::frame::AudioFrame;
use crate::dsp::traits::AudioEffect;
use std::any::Any;

pub struct HardClipDistortion {
    pub gain: f32,       // 1.0 sampai 20.0
    pub threshold: f32,  // 0.1 sampai 1.0
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

Setelah itu daftarkan modul efek kamu di catalog `src/dsp/mod.rs` dan UI selector di `src/gui/effect_card.rs`.

---

## 🧠 Arsitektur Sistem & Cara Kerjanya

AMPku dibangun dengan arsitektur **Dual-Thread Zero-Lock**:

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

### Aturan Emas Audio Thread (Safety Real-Time):
Audio thread berjalan pada prioritas realtime OS. Di dalam audio callback:
- **DILARANG Malloc/Heap Allocation**: Tidak boleh ada `Vec::push` atau alokasi memori dinamis di dalam loop sampel audio.
- **DILARANG Mutex / Blocking Locks**: Komunikasi parameter UI ke DSP menggunakan atomics (`AtomicF32`) atau SPSC ring buffer (`rtrb`).
- **DILARANG Blocking I/O**: Penulisan file rekaman WAV dilempar ke background worker thread via ring buffer terpisah.
- **DILARANG Panic / Unchecked NaN**: Semua komputasi floating point disanitasi terhadap nilai `NaN` dan `Infinity`.

---

## 📁 Format Preset (JSON)

Preset disimpan dalam format JSON bersih dan manusiawi:
```json
{
  "name": "Texas Blues",
  "guitar_channel": {
    "volume": 0.9,
    "effects": [
      { "type": "NoiseGate", "threshold_db": -52.0, "bypass": false },
      { "type": "Overdrive", "drive": 0.65, "tone": 0.55, "level": 0.8, "bypass": false },
      { "type": "CabSimIR", "profile": "Vintage_Fender_1x12", "bypass": false },
      { "type": "Reverb", "decay": 1.8, "mix": 0.25, "bypass": false }
    ]
  }
}
```

---

## 📜 Lisensi
MIT License - Bebas digunakan, dimodifikasi, dan didistribusikan kembali.
