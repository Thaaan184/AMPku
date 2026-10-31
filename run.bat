@echo off
setlocal
title GuitarFX Rust - Terminal Guitar Effects Engine

:: 1. If guitarfx.exe already exists, launch directly
if exist "guitarfx.exe" (
    echo [*] Starting GuitarFX...
    guitarfx.exe
    goto :END
)

:: 2. If cargo target directory has it
if exist "target\release\guitarfx.exe" (
    echo [*] Starting GuitarFX from target\release...
    target\release\guitarfx.exe
    goto :END
)

:: 3. Otherwise compile and run via Cargo
where cargo >nul 2>&1
if %ERRORLEVEL% EQU 0 (
    echo [*] Building and running GuitarFX via Cargo...
    cargo run --release
    goto :END
)

echo [!] Error: guitarfx.exe not found and Cargo is not installed.
echo [*] Download the pre-built guitarfx.exe or install Rust from https://rustup.rs
pause

:END
if %ERRORLEVEL% NEQ 0 (
    echo.
    echo [-] GuitarFX exited with code %ERRORLEVEL%.
    pause
)
