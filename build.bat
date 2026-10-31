@echo off
setlocal
echo ===================================================
echo   Building GuitarFX Engine (C++20, CMake, MSVC/MinGW)
echo ===================================================

where cmake >nul 2>&1
if %ERRORLEVEL% NEQ 0 (
    echo [-] Error: CMake is required to build from source.
    echo [*] Please install CMake or Visual Studio 2022 C++ Tools.
    pause
    exit /b 1
)

mkdir build 2>nul
cmake -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build --config Release -j4

if exist "build\Release\guitarfx.exe" (
    copy "build\Release\guitarfx.exe" "guitarfx.exe"
    echo [OK] guitarfx.exe built successfully!
) else if exist "build\guitarfx.exe" (
    copy "build\guitarfx.exe" "guitarfx.exe"
    echo [OK] guitarfx.exe built successfully!
)
