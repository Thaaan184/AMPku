# GuitarFX 🎸 (Rust Edition)

> **Lightweight Real-Time Terminal Guitar Effects Engine in Pure Rust**  
> Pemroses efek gitar digital mandiri berbasis Rust (CPAL, ringbuf, crossterm). Tanpa DAW, tanpa VST eksternal, dan tanpa GUI berat.

---

## 1. Konsep & Alur Sinyal

GuitarFX menangkap sinyal gitar dari soundcard USB, memproses efek digital buatan sendiri secara real-time, dan mengirim hasilnya ke **VoiceMeeter Banana** untuk didistribusikan ke headphone, Discord, atau recording.

```
[ Gitar Elektrik ]
       │
       ▼
[ Vention USB Sound Card / Input Device ]
       │
       ▼
[ GuitarFX (Rust Engine) ]
   ├── Input Gain (Volume)
   ├── Auto-Wah (Dynamic Envelope State-Variable Filter)
   ├── Clean Preamp (Sparkle Bright & Low-Mid Warmth)
   ├── Crunch Amp (British Tube Edge-of-Breakup Saturation)
   ├── Overdrive (Drive, Tone, Smooth Tanh Clipping)
   ├── Distortion (Asymmetric Diode Clipping, 720Hz Pre-emphasis)
   ├── Delay (Stereo Ping-Pong, Feedback, Mix)
   └── Brickwall Limiter (Clamping -0.3 dBFS)
       │
       ▼ (Stereo 48 kHz)
[ VoiceMeeter AUX Input ]
       │
       ├── A1 ──> Headphone (Monitoring langsung tanpa latency)
       └── B1 ──> Discord / OBS / Voice Chat / Recording
```

---

## 2. Cara Pakai (Siap Jalan)

### Opsi A: Langsung Jalankan (.exe Portable)
1. Unduh file **`guitarfx.exe`** atau **[GuitarFX-v0.1.0-windows-x86_64.zip](https://github.com/Thaaan184/AMPku/releases/download/v0.1.0/GuitarFX-v0.1.0-windows-x86_64.zip)**.
2. Jalankan **`guitarfx.exe`** (atau double-click **`run.bat`**).
3. Terminal interaktif langsung terbuka di Command Prompt / Windows Terminal.

### Opsi B: Jalankan via Cargo
```bash
cargo run --release
```

---

## 3. Panduan Setup VoiceMeeter Banana

### A. Pengaturan Input & Output di GuitarFX
Saat aplikasi dibuka, GuitarFX otomatis mencari device berikut:
- **Input**: `Vention USB Audio` (atau soundcard default).
- **Output**: `VoiceMeeter AUX Input` (atau soundcard default).
- Jika ingin ganti device secara manual, tekan **`D`** di terminal.

### B. Pengaturan di VoiceMeeter Banana
1. Pada strip virtual **`VOICEMEETER AUX`**:
   - Aktifkan **`A1`**: Agar suara gitar basah (efek) terdengar di headphone kamu.
   - Aktifkan **`B1`**: Agar suara gitar masuk ke mic Discord atau recording.
2. **PENTING (Mencegah Double Audio / Echo)**:
   - Jangan aktifkan monitoring hardware (A1) pada input fisik gitar di VoiceMeeter. Biarkan hanya strip **AUX** yang menyalakan A1. Ini mencegah suara senar mentah (dry) bertumpuk dengan suara efek (wet).

---

## 4. Tampilan & Kontrol Terminal (Crossterm)

```
=========================================
             GUITARFX RUST               
=========================================

 STATUS : [RUNNING]

 INPUT  : Vention USB Audio
 OUTPUT : VoiceMeeter AUX Input

 SAMPLE RATE : 48000 Hz
 BUFFER SIZE : 256 frames

=========================================
 AUDIO METERS
=========================================
 INPUT  [######--------] -14.2 dB
 OUTPUT [########------]  -8.5 dB

=========================================
 EFFECTS CHAIN
=========================================
  [OFF] [1] Clean Preamp  Bright: 50% | Warmth: 40%
  [OFF] [2] Crunch Amp    Drive: 45% | Tone: 55%
  [ON]  [3] Overdrive     Drive: 40% | Tone: 60%
  [OFF] [4] Distortion    Drive: 50% | Tone: 50%
  [OFF] [5] Auto-Wah      Depth: 70% | Resonance: 65%
  [ON]  [6] Delay         Time: 320 ms | FB: 35% | Mix: 25%

 Active Parameter: [Overdrive: Drive] (Use [Tab] to cycle, [+/-] to adjust)

=========================================
 CONTROLS
=========================================
 [D] Select Audio Devices (2-Step Wizard)
 [1] Clean   [2] Crunch   [3] Overdrive   [4] Distortion   [5] Wah   [6] Delay
 [+] / [-] Adjust Parameter    [Tab] Cycle Active Parameter
 [B] Bypass All          [M] Mute Output          [Q] Quit
=========================================
```

### Tombol Kontrol:
- **`D`**: Buka wizard 2-langkah pemilihan input & output device.
- **`1`**: Toggle **Clean Preamp** `[ON]` / `[OFF]` (bright sparkle + warm body).
- **`2`**: Toggle **Crunch Amp** `[ON]` / `[OFF]` (edge-of-breakup tube amp).
- **`3`**: Toggle **Overdrive** `[ON]` / `[OFF]` (smooth blues/rock saturation).
- **`4`**: Toggle **Distortion** `[ON]` / `[OFF]` (diode clipping + 720 Hz pre-emphasis).
- **`5`**: Toggle **Auto-Wah** `[ON]` / `[OFF]` (envelope-driven SVF bandpass sweep).
- **`6`**: Toggle **Delay** `[ON]` / `[OFF]` (stereo ping-pong delay).
- **`+` / `-`**: Naikkan / turunkan nilai parameter aktif (+/- 5%).
- **`Tab`**: Ganti parameter aktif (Bright, Warmth, Drive, Tone, Depth, Res, Time, FB, Mix).
- **`B`**: Global Bypass (matikan semua efek, suara clean mentah).
- **`M`**: Mute audio output seketika.
- **`Q`**: Keluar dari aplikasi dengan bersih.

---

## 5. Command Line Flags

```bash
# Cek semua audio input & output yang terdeteksi Windows:
guitarfx.exe --list-devices

# Menampilkan versi software:
guitarfx.exe --version

# Menampilkan bantuan penggunaan:
guitarfx.exe --help
```

---

## 6. Struktur Kode

```
guitarfx/
├── Cargo.toml
├── src/
│   ├── main.rs       # Entrypoint & CLI dispatcher
│   ├── audio.rs      # CPAL capture, playback, dan SPSC ring buffer
│   ├── effects.rs    # Algoritma DSP (Gain, Overdrive, Distortion, Delay)
│   ├── devices.rs    # Enumerasi dan pencarian audio hardware
│   └── terminal.rs   # Terminal interactive dashboard (Crossterm)
└── README.md
```

---

## 📜 Lisensi
MIT License
