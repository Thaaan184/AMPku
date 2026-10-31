# GuitarFX 🎸

> **Lightweight Real-Time Terminal Guitar Effects Engine for Windows**  
> Pemroses efek gitar digital mandiri berbasis C++20 dan WASAPI. Tanpa DAW, tanpa VST eksternal, dan tanpa GUI berat.

---

## 1. Konsep & Alur Kerja

GuitarFX dirancang khusus untuk musisi yang ingin colok gitar listrik langsung ke PC dengan latensi rendah, memproses efek digital buatan sendiri, dan mendistribusikan audio secara bersih ke headphone, Discord, atau recording melalui **VoiceMeeter Banana**.

```
[ Gitar Elektrik ]
       │
       ▼
[ Vention USB Sound Card / Input Device ]
       │
       ▼
[ GuitarFX (Terminal Engine) ]
   ├── 01. Input Gain
   ├── 02. Noise Gate
   ├── 03. Overdrive
   ├── 04. Distortion
   ├── 05. Amp Simulator (Clean / Crunch / Lead)
   ├── 06. 3-Band EQ
   ├── 07. Stereo Delay
   ├── 08. Algorithmic Reverb
   └── 09. Output Brickwall Limiter
       │
       ▼ (Stereo Output)
[ VoiceMeeter AUX Input ]
       │
       ├── A1 ──> Headphone (Monitoring langsung tanpa latency)
       └── B1 ──> Discord / OBS / Voice Chat
```

---

## 2. Download & Cara Pakai (Siap Jalan)

### Opsi A: Langsung Jalankan (.exe Portable)
1. Unduh **[GuitarFX-v0.1.0-windows-x86_64.zip](https://github.com/Thaaan184/AMPku/releases/download/v0.1.0/GuitarFX-v0.1.0-windows-x86_64.zip)** atau file **`guitarfx.exe`**.
2. Ekstrak file zip ke folder mana saja di komputer kamu.
3. Jalankan **`guitarfx.exe`** (atau double-click **`run.bat`**).
4. Terminal interaktif langsung terbuka di Command Prompt / Windows Terminal.

---

## 3. Panduan Setup Audio & VoiceMeeter Banana

GuitarFX dirancang fleksibel untuk routing capture dan playback:

### A. Konfigurasi Input Device
Tekan tombol **`D`** di terminal untuk membuka menu audio device:
- **Pilihan 1 (Rekomendasi Direct)**: Pilih **`Vention USB Audio`** (atau nama soundcard USB kamu).
- **Pilihan 2 (VoiceMeeter In)**: Jika gitar kamu sudah masuk ke strip VoiceMeeter dan di-route ke bus B1/B2, pilih **`VoiceMeeter Out B1`** atau **`VoiceMeeter Out B2`**.

### B. Konfigurasi Output Device
- Pada daftar Playback Device, pilih **`VoiceMeeter AUX Input`** (atau **`Voicemeeter Aux VAIO`**).
- Tekan **`Apply Selected Devices`**.

### C. Pengaturan di VoiceMeeter Banana
1. Pada strip virtual **`VOICEMEETER AUX`**:
   - Aktifkan **`A1`**: Agar suara gitar basah (dengan distorsi/efek) terdengar di headphone kamu.
   - Aktifkan **`B1`**: Agar suara gitar masuk ke mic Discord, Zoom, atau aplikasi streaming.
2. **PENTING (Mencegah Double Audio / Echo)**:
   - Jangan aktifkan monitoring hardware (A1) pada input fisik gitar di VoiceMeeter. Biarkan hanya strip **AUX** yang menyalakan A1. Ini mencegah suara senar mentah (dry) bertumpuk dengan suara efek (wet).

---

## 4. Tampilan & Navigasi Terminal (FTXUI)

Saat aplikasi berjalan, layar terminal menampilkan mixer studio digital:

```
========================================================
                   GUITARFX ENGINE                      
========================================================
 STATUS : RUNNING                 CPU : 1.8%
 INPUT  : Vention USB Audio       48000 Hz / MONO
 OUTPUT : VoiceMeeter AUX Input   48000 Hz / STEREO
 BUFFER : 256 FRAMES (~10 ms)     XRUNS : 0   PRESET : crunch
--------------------------------------------------------
 INPUT LEVEL  [########--------] -14 dB
 OUTPUT LEVEL [##########------] -8 dB
--------------------------------------------------------
 EFFECT CHAIN
--------------------------------------------------------
 [ON]  01. Input Gain              +1.0 dB
 [ON]  02. Noise Gate              -45 dB
 [ON]  03. Overdrive               Drive: 50%
 [OFF] 04. Distortion              Drive: 40%
 [ON]  05. Amp Simulator           CRUNCH
 [ON]  06. Equalizer               L:1 M:2 H:0
 [OFF] 07. Delay                   300 ms
 [ON]  08. Reverb                  15%
 [ON]  09. Output Limiter          -0.3 dBFS
--------------------------------------------------------
 [D] Devices  [E] Effects  [P] Presets  [B] Bypass  [M] Mute  [Q] Quit
========================================================
```

### Tombol Navigasi:
| Tombol | Fungsi |
|---|---|
| **D** | Buka / tutup menu pemilihan audio input & output device |
| **E** | Buka menu daftar efek untuk toggle bypass per-efek |
| **P** | Buka daftar preset untuk memuat tone gitar |
| **Space** | Toggle `[ON]` / `[OFF]` efek yang sedang dipilih di menu Effects |
| **B** | Global bypass (matikan semua efek, suara clean mentah) |
| **M** | Mute audio output seketika |
| **Esc** | Kembali ke tampilan utama Dashboard |
| **Q** | Keluar dari aplikasi dengan aman |

---

## 5. Rantai Efek DSP (Modular C++20)

Seluruh algoritma efek ditulis langsung menggunakan C++20 tanpa VST pihak ketiga:

1. **Input Gain**: Kontrol penguatan sinyal awal (-24 dB s/d +24 dB) dengan parameter smoothing.
2. **Noise Gate**: Menghilangkan dengung desis pickup single-coil dengan threshold desibel dan histeresis 2 dB.
3. **Overdrive**: Saturasi soft-clipping berbasis non-linear hyperbolic tangent (`tanh`) dan filter tone.
4. **Distortion**: Asymmetric diode clipping (model 1N4148) dengan filter pre-emphasis 720 Hz untuk karakter rock/metal modern.
5. **Amp Simulator**: Pemodelan tabung preamp dengan 3 voicing (**Clean**, **Crunch**, **Lead**), tone stack 3-band, dan filter kabinet speaker 4.8 kHz.
6. **Equalizer**: Filter biquad Direct Form II Transposed untuk Low Shelf (120 Hz), Peaking Mid (1 kHz, Q=1.2), dan High Shelf (3.5 kHz).
7. **Delay**: Digital delay stereo ping-pong dengan feedback damping analog dan stereo spread.
8. **Reverb**: Algoritma Freeverb/Schroeder (8 comb filters paralel + 4 allpass filters seri per kanal) dengan decay luas.
9. **Output Limiter**: Lookahead soft-knee brickwall peak limiter pada ceiling -0.3 dBFS untuk mencegah digital clipping pada output VoiceMeeter.

---

## 6. Preset Bawaan (Format JSON)

Preset tersimpan otomatis di folder `presets/*.json`:
- **`clean.json`**: Nada clean jernih, bright EQ, dan reverb hangat.
- **`blues.json`**: Overdrive hangat, voicing tube crunch, dan slapback delay pendek.
- **`crunch.json`**: Rhythm crunch berbobot dengan dorongan mid-range.
- **`rock.json`**: Distorsi high-gain, amp lead, noise gate aktif, dan stereo delay.
- **`ambient.json`**: Clean tone dengan delay panjang 480ms dan cathedral reverb lebar.

---

## 7. Command Line Flags

GuitarFX juga bisa dijalankan dengan opsi command line:
```bash
# Cek semua audio input & output yang terdeteksi Windows:
guitarfx.exe --list-devices

# Menampilkan versi software:
guitarfx.exe --version

# Menampilkan bantuan penggunaan:
guitarfx.exe --help
```

---

## 8. Kompilasi dari Source Code

### Prasyarat:
- C++20 compliant compiler (MSVC Visual Studio 2022 / Clang / GCC MinGW).
- CMake 3.20+.

### Langkah Build di Windows:
```cmd
# 1. Generate project build
cmake -B build -DCMAKE_BUILD_TYPE=Release

# 2. Compile binary release
cmake --build build --config Release -j4

# Executable siap pakai akan berada di build/Release/guitarfx.exe
```

Atau cukup jalankan script otomatis **`build.bat`**.

---

## 📜 Lisensi
MIT License - Bebas digunakan dan dimodifikasi untuk kebutuhan musisi dan developer.
