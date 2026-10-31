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
   ├── Overdrive (Drive, Tone, Soft-Clipping)
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
 EFFECTS
=========================================
  [ON]  [1] Overdrive    Drive: 40% | Tone: 60%
  [OFF] [2] Distortion   Drive: 50% | Tone: 50%
  [ON]  [3] Delay        Time: 320 ms | FB: 35% | Mix: 25%

 Active Parameter: [Overdrive Drive] (Use [Tab] to cycle, [+/-] to adjust)

=========================================
 CONTROLS
=========================================
 [D] Select Device
 [1] Toggle Overdrive    [2] Toggle Distortion    [3] Toggle Delay
 [+] / [-] Adjust Parameter    [Tab] Cycle Active Parameter
 [B] Bypass All          [M] Mute Output          [Q] Quit
=========================================
```

### Tombol Kontrol:
- **`D`**: Buka daftar audio device untuk memilih input & output.
- **`1`**: Toggle efek Overdrive `[ON]` / `[OFF]`.
- **`2`**: Toggle efek Distortion `[ON]` / `[OFF]`.
- **`3`**: Toggle efek Delay `[ON]` / `[OFF]`.
- **`+` / `-`**: Naikkan / turunkan nilai parameter aktif (+/- 5%).
- **`Tab`**: Ganti parameter yang sedang dipilih (Drive, Tone, Time, Feedback, Mix).
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
