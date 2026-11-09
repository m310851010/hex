@echo off
rem One-click release build of Hex for Windows.
rem Uses the locally installed VS 2022, the user's CMake, and the Vulkan SDK
rem extract under D:\tmp\hex-build; produces hex.exe and hex.com under
rem target\release. Double-click from Explorer or run from any terminal.
rem Hex keeps its settings, history, logs, models, and downloads beside
rem hex.exe (set HEX_APPLICATION_SUPPORT_DIR to override).
setlocal
cd /d "%~dp0"

rem Build environment: explicit, so the script works in any fresh terminal
rem regardless of user-level environment variables.
set "CMAKE_GENERATOR=Visual Studio 17 2022"
set "VULKAN_SDK=D:\tmp\hex-build\vulkan\sdk309"
set "CMAKE_PREFIX_PATH=D:/tmp/hex-build/vulkan/sdk309/Lib/cmake"

where cargo >nul 2>nul || (echo cargo not found; install the Rust toolchain first & exit /b 1)

echo Building hex.exe and hex.com (release) ...
cargo build --locked --release -p hex@3.3.0 -p hex-console || goto :failed

rem hex.com is hex-console.exe by another name: it sits beside hex.exe and
rem finds it there, and PATH entries can run it from a terminal.
copy /y "target\release\hex-console.exe" "target\release\hex.com" >nul

echo.
echo Build succeeded:
"target\release\hex.exe" --version
echo   target\release\hex.exe
echo   target\release\hex.com
exit /b 0

:failed
echo.
echo Build FAILED. Check the cargo output above.
exit /b 1
