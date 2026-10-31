cmake_minimum_required(VERSION 3.20)
project(GuitarFX VERSION 0.1.0 LANGUAGES C CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# 1. Fetch FTXUI for Terminal UI
include(FetchContent)
FetchContent_Declare(
  ftxui
  GIT_REPOSITORY https://github.com/ArthurSonzogni/ftxui
  GIT_TAG v5.0.0
)
set(FTXUI_BUILD_EXAMPLES OFF CACHE BOOL "" FORCE)
set(FTXUI_BUILD_DOCS OFF CACHE BOOL "" FORCE)
set(FTXUI_BUILD_TESTS OFF CACHE BOOL "" FORCE)
FetchContent_MakeAvailable(ftxui)

# 2. Include Directories
include_directories(
    ${CMAKE_CURRENT_SOURCE_DIR}/include
    ${CMAKE_CURRENT_SOURCE_DIR}/third_party
)

# 3. Source Files
set(SOURCES
    src/main.cpp
    src/audio/miniaudio_impl.cpp
    src/audio/AudioDeviceManager.cpp
    src/audio/AudioEngine.cpp
    src/core/PresetManager.cpp
    src/core/ConfigManager.cpp
    src/ui/TerminalUI.cpp
)

# 4. Executable
add_executable(guitarfx ${SOURCES})

target_link_libraries(guitarfx PRIVATE
    ftxui::screen
    ftxui::dom
    ftxui::component
)

# 5. Windows-specific libraries for audio and console
if (WIN32 OR CMAKE_SYSTEM_NAME STREQUAL "Windows")
    target_link_libraries(guitarfx PRIVATE
        ole32
        winmm
        ksuser
        uuid
    )
endif()

# 6. Optimization Flags
if (MSVC)
    target_compile_options(guitarfx PRIVATE /O2 /W3)
else()
    target_compile_options(guitarfx PRIVATE -O3 -Wall)
    target_link_options(guitarfx PRIVATE -s)
endif()

# 7. Unit Tests
add_executable(test_dsp tests/test_dsp.cpp)
target_include_directories(test_dsp PRIVATE include third_party)
if (MSVC)
    target_compile_options(test_dsp PRIVATE /O2)
else()
    target_compile_options(test_dsp PRIVATE -O3)
endif()

