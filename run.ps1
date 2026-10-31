# AMPku - Windows PowerShell Launcher & Auto-Installer
$ErrorActionPreference = "Stop"

Write-Host "===================================================" -ForegroundColor Cyan
Write-Host "  AMPku 🎸🎙️ - Launcher & Auto-Installer" -ForegroundColor Cyan
Write-Host "  Zero-Setup Low-Latency DSP Studio for Windows" -ForegroundColor Cyan
Write-Host "===================================================" -ForegroundColor Cyan
Write-Host ""

# 1. Check Cargo in PATH or standard user profile
$cargoCmd = Get-Command cargo -ErrorAction SilentlyContinue
if (-not $cargoCmd) {
    $cargoDefault = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
    if (Test-Path $cargoDefault) {
        $env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
    } else {
        Write-Host "[!] Rust/Cargo belum terdeteksi di sistem." -ForegroundColor Yellow
        Write-Host "[*] Mengunduh dan memasang Rust via rustup-init..." -ForegroundColor Green
        
        $installer = Join-Path $env:TEMP "rustup-init.exe"
        Invoke-WebRequest -Uri "https://win.rustup.rs/x86_64" -OutFile $installer
        Start-Process -FilePath $installer -ArgumentList "-y" -Wait
        $env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
    }
}

Write-Host "[*] Rust/Cargo terdeteksi:" -ForegroundColor Green
cargo --version

Write-Host ""
Write-Host "===================================================" -ForegroundColor Cyan
Write-Host "  [*] Memulai AMPku (cargo run --release)..." -ForegroundColor Cyan
Write-Host "===================================================" -ForegroundColor Cyan
Write-Host ""

cargo run --release
