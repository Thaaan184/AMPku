@echo off
setlocal
title GuitarFX - Lightweight Terminal Guitar Effects Engine

:: If guitarfx.exe already exists, launch immediately
if exist "guitarfx.exe" (
    echo [*] Launching GuitarFX...
    guitarfx.exe
    goto :END
)

:: If build directory has it
if exist "build\guitarfx.exe" (
    echo [*] Launching GuitarFX from build...
    build\guitarfx.exe
    goto :END
)

if exist "build\Release\guitarfx.exe" (
    echo [*] Launching GuitarFX from build\Release...
    build\Release\guitarfx.exe
    goto :END
)

echo [!] guitarfx.exe not found. Running build.bat...
call build.bat
if exist "build\Release\guitarfx.exe" (
    build\Release\guitarfx.exe
) else if exist "build\guitarfx.exe" (
    build\guitarfx.exe
)

:END
if %ERRORLEVEL% NEQ 0 (
    echo.
    echo [-] GuitarFX exited with error code %ERRORLEVEL%.
    pause
)
