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
4. **Modular DSP Rack**: Bebas tambah, hapus, bypass, dan atur urutan efek (Overdrive, Cab Sim, Delay, Reverb, Compressor, Gate, EQ).
5. **Direct WAV Recorder**: Rekam jamming atau ide lagu dalam format 24-bit PCM WAV lossless dengan satu tombol klik.

---

## 🚀 Quick Start (Pengguna Siap Pakai)
1. Colok soundcard USB kamu (misal Vention) ke PC Windows 10/11.
2. Colok kabel gitar ke Input 1 (Left), dan mic ke Input 2 (Right).
3. Jalankan `AMPku.exe`.
4. Pada bar atas, pilih:
   - **Input Device**: *Vention USB Audio*
   - **Output Device**: *Speakers / Headphone (Realtek / USB)*
   - **Buffer**: *64 samples* (untuk latensi terendah) atau *128 samples*.
5. Pilih preset favorit (misal: `Texas Blues Crunch` atau `Clean Ambient`) dan mulai main!

---

## 🛠️ Panduan Developer: Mengambil & Membangun Source Code

### 1. Prasyarat Sistem
- **Sistem Operasi**: Windows 10/11 (atau cross-compile dari Linux dengan MinGW).
- **Rust Toolchain**: Rust versi stabil 1.75+ (install via [rustup.rs](https://rustup.rs/)).
- **Git**: Untuk cloning repository.

### 2. Clone Repository
```bash
git clone https://github.com/Thaaan184/AMPku.git
cd AMPku
```

### 3. Kompilasi & Menjalankan (Development Mode)
```bash
# Menjalankan langsung dengan live reload / debugger
cargo run
```

### 4. Membangun Binary Standalone Release (Production)
```bash
# Kompilasi binary teroptimasi maksimal (LTO + strip)
cargo build --release
```
File executable mandiri siap pakai akan berada di:
`target/release/AMPku.exe` (ukuran di bawah 15 MB, bebas dependensi eksternal).

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

---

## 🎛️ Cara Mengubah & Menambah Efek Baru (Custom DSP)

Semua efek di AMPku menerapkan trait tunggal `AudioEffect` yang berada di `src/dsp/traits.rs`. Trait ini mendukung pemrosesan frame stereo (`AudioFrame { left, right }`), sehingga efek spasial seperti Reverb dan Delay dapat menghasilkan persebaran stereo yang lebar:

### Contoh: Membuat Efek Distortion Baru
Buat file baru di `src/dsp/saturation/my_distortion.rs`:

```rust
use crate::dsp::traits::{AudioEffect, AudioFrame};

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

    fn set_parameter(&mut self, id: u32, value: f32) {
        match id {
            0 => self.gain = value,
            1 => self.threshold = value,
            _ => {}
        }
    }

    fn get_parameter(&self, id: u32) -> f32 {
        match id {
            0 => self.gain,
            1 => self.threshold,
            _ => 0.0,
        }
    }

    fn is_bypassed(&self) -> bool {
        self.bypass
    }

    fn set_bypass(&mut self, bypass: bool) {
        self.bypass = bypass;
    }

    fn reset(&mut self) {}
}
```

Setelah itu daftarkan modul efek kamu di catalog `src/dsp/mod.rs` dan UI selector di `src/gui/effect_card.rs`.

---

## 📁 Struktur Folder Project

```
AMPku/
├── Cargo.toml                  # Konfigurasi dependensi Rust & profil rilis
├── README.md                   # Dokumentasi lengkap produk & developer guide
├── CHANGELOG.md                # Catatan rilis minor & major
├── docs/
│   └── superpowers/specs/      # Dokumen arsitektur teknis mendalam
├── src/
│   ├── main.rs                 # Entry point aplikasi & startup window
│   ├── audio/                  # Engine audio cpal, WASAPI routing, recorder
│   │   ├── engine.rs           # Inisialisasi stream & buffer callback
│   │   ├── ring_buffer.rs      # SPSC ring buffer lock-free
│   │   └── recorder.rs         # Background WAV file writer (hound)
│   ├── dsp/                    # Kumpulan algoritma efek audio
│   │   ├── traits.rs           # Trait AudioEffect
│   │   ├── chain.rs            # Modular node execution pipeline
│   │   ├── dynamics/           # Noise gate, sustainer compressor
│   │   ├── saturation/         # Overdrive, distortion, fuzz, booster
│   │   ├── filters/            # Biquad EQ, high-pass filter
│   │   ├── time/               # Delay, reverb
│   │   └── cab_sim/            # FIR Cabinet Impulse Response convolution
│   ├── gui/                    # Antarmuka grafis desktop (egui)
│   │   ├── app.rs              # Root application view
│   │   ├── channel_strip.rs    # Kolom mixer Gitar & Mic
│   │   ├── effect_card.rs      # Tampilan modular card untuk efek
│   │   └── meter.rs            # Widget VU meter stereo & desibel
│   └── presets/                # Sistem preset JSON
│       ├── manager.rs          # Serializer & deserializer
│       └── defaults.rs         # Preset bawaan pabrik (blues, lead, clean)
```

---

## 📄 Format Preset (JSON)
Setiap preset disimpan dalam file JSON manusiawi yang mudah diedit:
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
