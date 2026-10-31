@echo off
setlocal enabledelayedexpansion
title AMPku - Low-Latency Guitar & Vocal DSP Studio

echo ===================================================
echo   AMPku 🎸🎙️ - Launcher & Auto-Installer
echo   Zero-Setup Low-Latency DSP Studio for Windows
echo ===================================================
echo.

:: 1. Check if cargo is in PATH
where cargo >nul 2>&1
if %ERRORLEVEL% EQU 0 goto :CHECK_TOOLCHAIN

:: 2. Check in standard default cargo path %USERPROFILE%\.cargo\bin
if exist "%USERPROFILE%\.cargo\bin\cargo.exe" (
    set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
    goto :CHECK_TOOLCHAIN
)

:: 3. Rust not found -> Auto-install via winget or rustup-init
echo [!] Rust/Cargo belum terdeteksi di komputer ini.
echo [*] Memulai proses instalasi otomatis Rust Toolchain...
echo.

where winget >nul 2>&1
if %ERRORLEVEL% EQU 0 (
    echo [*] Menginstall Rust via Windows Package Manager (winget)...
    winget install --id Rustlang.Rustup -e --accept-source-agreements --accept-package-agreements
    if %ERRORLEVEL% EQU 0 (
        set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
        goto :CHECK_TOOLCHAIN
    )
)

echo [*] Mengunduh installer resmi rustup-init.exe...
powershell -Command "Invoke-WebRequest -Uri 'https://win.rustup.rs/x86_64' -OutFile '%TEMP%\rustup-init.exe'"
if not exist "%TEMP%\rustup-init.exe" (
    echo [-] Gagal mengunduh rustup-init.exe secara otomatis.
    echo [!] Silakan unduh dan pasang manual melalui: https://rustup.rs
    pause
    exit /b 1
)

echo [*] Memasang Rust Toolchain secara otomatis...
"%TEMP%\rustup-init.exe" -y
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"

:CHECK_TOOLCHAIN
echo.
echo [*] Memeriksa versi Rust & Cargo...
cargo --version
if %ERRORLEVEL% NEQ 0 (
    echo [-] Cargo belum aktif di PATH saat ini.
    echo [!] Silakan tutup command prompt ini dan jalankan run.bat kembali.
    pause
    exit /b 1
)

echo.
echo ===================================================
echo   [*] Menjalankan AMPku Studio (cargo run --release)
echo   [*] Dependencies akan di-resolve dan di-compile otomatis...
echo ===================================================
echo.

cargo run --release

if %ERRORLEVEL% NEQ 0 (
    echo.
    echo [-] Terjadi kendala saat menjalankan AMPku.
    echo [*] Memeriksa catatan log...
    if exist ampku_boot.log (
        echo.
        echo --- [Log Diagnostik: ampku_boot.log] ---
        type ampku_boot.log
    )
    if exist ampku_crash.log (
        echo.
        echo --- [Log Crash: ampku_crash.log] ---
        type ampku_crash.log
    )
    echo.
    pause
    exit /b %ERRORLEVEL%
)

echo.
echo [*] AMPku ditutup dengan aman.
pause
