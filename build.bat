@echo off
rem Builds math-viz-rust on Windows: the native app (SDL3, OpenGL 3.3) and its release build, and
rem the web release when Emscripten is set up.
rem
rem UNTESTED. Written on macOS (i3-release) with no Windows machine to run it on: it follows
rem build.sh's steps for `native` and `release`, which are verified on macOS. Where the two differ,
rem build.sh is right. README: Platforms.
rem
rem   build.bat native    build\native\function-viewer.exe (debug), SDL3.dll beside it
rem   build.bat release   build\release\native\function-viewer.exe (optimised) with SDL3.dll beside
rem                       it, and build\release\web\ when emsdk is set up
rem
rem Needs: Rust 1.99.0 (rust-toolchain.toml pins it; the wasm target listed there) with Cargo;
rem Visual Studio 2022's C++ build tools, run from the "x64 Native Tools Command Prompt for VS 2022"
rem (cl builds the vendored C libraries through cargo's cc/cmake steps, and links); CMake on PATH
rem (SDL3 compiles from source: there is no system SDL3 on Windows, so both verbs turn on
rem cargo's bundled-sdl feature, like build.sh release does); for the web release, emsdk with
rem Emscripten 6.0.12 in %EMSDK% or %USERPROFILE%\emsdk (README: Setup), activated so emcc is on
rem PATH (cargo's cc step calls it for the wasm target's C objects). Inside a parenthesized block a
rem path is expanded late (!VAR!): a path with a parenthesis, such as C:\Program Files (x86)\...,
rem expanded early (%VAR%) would end the block.
setlocal EnableDelayedExpansion
cd /d "%~dp0"

set EMSDK_VERSION=6.0.12
if not defined EMSDK set "EMSDK=%USERPROFILE%\emsdk"

where cargo >nul 2>nul || (echo cargo is not on PATH: install Rust 1.99.0 ^(README: Setup^) & exit /b 1)
where cl >nul 2>nul || (echo cl is not on PATH: run from the x64 Native Tools Command Prompt for VS 2022 & exit /b 1)
where cmake >nul 2>nul || (echo cmake is not on PATH: install CMake ^(README: Setup^) & exit /b 1)

if /i "%~1"=="native" (
	call :native
) else if /i "%~1"=="release" (
	call :release
) else (
	findstr /b /c:"rem   build.bat" "%~f0"
	exit /b 2
)
exit /b %errorlevel%

rem The native app, debug: SDL3 compiled from the vendored sources (bundled-sdl), the DLL copied
rem beside the exe, where Windows loads it from. (build.sh's dev builds link Homebrew's SDL3;
rem Windows has no system SDL3, so native builds bundled too.)
:native
cargo build -p function-viewer --features bundled-sdl || exit /b 1
if not exist build\native mkdir build\native
copy /y target\debug\function-viewer.exe build\native\ >nul || exit /b 1
call :copydll target\debug build\native || exit /b 1
echo built build\native\function-viewer.exe
exit /b 0

rem The release: the native app optimised with the machine's paths remapped out of it (build.sh's
rem release RUSTFLAGS; %CD% is the worktree), SDL3.dll beside it, and the web release when emsdk
rem is there. build.sh release also fails on any path of this machine in the outputs; that check
rem has no counterpart here (no strings/tr pair on a stock Windows box).
:release
set "RELEASE_RUSTFLAGS=--remap-path-prefix=%CD%=. --remap-path-prefix=%USERPROFILE%=home --remap-path-prefix=%CARGO_HOME%=cargo-home"
if not defined CARGO_HOME set "RELEASE_RUSTFLAGS=--remap-path-prefix=%CD%=. --remap-path-prefix=%USERPROFILE%=home"
set RUSTFLAGS=!RELEASE_RUSTFLAGS!
cargo build --release -p function-viewer --features bundled-sdl || exit /b 1
set RUSTFLAGS=
if not exist build\release\native mkdir build\release\native
copy /y target\release\function-viewer.exe build\release\native\ >nul || exit /b 1
call :copydll target\release build\release\native || exit /b 1
echo built build\release\native\function-viewer.exe
if not exist "!EMSDK!\emsdk_env.bat" (
	echo the web release needs emsdk with Emscripten %EMSDK_VERSION% at !EMSDK! ^(README: Setup^): skipped
	exit /b 0
)
call "!EMSDK!\emsdk_env.bat" >nul 2>nul
set EMCC_VERSION=
for /f "delims=" %%v in ('emcc --version ^| findstr /b /c:"emcc"') do set "EMCC_VERSION=%%v"
if "!EMCC_VERSION: %EMSDK_VERSION% =!"=="!EMCC_VERSION!" (echo emcc is not %EMSDK_VERSION%: !EMCC_VERSION! & exit /b 1)
set RUSTFLAGS=!RELEASE_RUSTFLAGS!
cargo build -p function-viewer --release --target wasm32-unknown-emscripten || exit /b 1
set RUSTFLAGS=
if exist build\release\web rmdir /s /q build\release\web
mkdir build\release\web
copy /y target\wasm32-unknown-emscripten\release\function-viewer.js build\release\web\ >nul || exit /b 1
copy /y target\wasm32-unknown-emscripten\release\function_viewer.wasm build\release\web\ >nul || exit /b 1
copy /y web\index.html build\release\web\ >nul || exit /b 1
echo built build\release\web\
exit /b 0

rem Copies the SDL3.dll cargo's SDL3 build left under %1 (target\debug or target\release) next to
rem the exe in %2. cargo links the DLL but does not place it beside the binary on Windows.
:copydll
for /r "%~1" %%f in (SDL3.dll) do (
	copy /y "%%f" "%~2\" >nul || exit /b 1
	echo SDL3.dll beside the exe ^(from %%f^)
	exit /b 0
)
echo no SDL3.dll under %~1: the bundled-sdl build left none & exit /b 1
