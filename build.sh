#!/usr/bin/env bash
# Builds math-viz-rust's two targets from crates/: a native app (SDL3,
# OpenGL 3.3) and a web build (Emscripten, SDL3 compiled from source, WebGL2)
# served from build/web/. The README has the toolchain, its versions and the
# setup commands.
#
#   ./build.sh native    build/native/function-viewer
#   ./build.sh web       build/web/: index.html, function-viewer.js,
#                        function_viewer.wasm (the emitted names, kept: the JS
#                        fetches its baked wasm name, so no rename)
#   ./build.sh test      cargo test for the workspace
#   ./build.sh check     fmt, clippy for both targets, tests, both builds
#   ./build.sh release   build/release/: native/function-viewer and web/,
#                        optimised, without Homebrew, with no path of this
#                        machine in them (i3-release hardens the check)
#   ./build.sh           native and web
#
# Every step prints its status; the script exits non-zero when any step fails
# (set -euo pipefail, and each step echoes what it runs). t0a proved the
# non-zero exit by breaking a step once (progress/t0a.md).
set -euo pipefail
cd "$(dirname "$0")"

EMSDK_VERSION=6.0.12 # the Emscripten the web build is verified with (README: Setup)
EMSDK_DIR="${EMSDK:-$HOME/emsdk}"

# The emcc output names cargo emits for the binary target. The pair is split:
# cargo names the `.js` after the package (`function-viewer.js`) while emcc
# derives the `.wasm` from the object file's stem (`function_viewer.wasm`),
# and the generated JS fetches that baked name (emcc 6.0.12 reads neither the
# script URL nor `Module.wasmBinaryFile` for it: measured, progress/t0a.md).
# So build/web/ keeps both emitted names and index.html loads function-viewer.js.
WEB_JS=target/wasm32-unknown-emscripten/release/function-viewer.js
WEB_WASM=target/wasm32-unknown-emscripten/release/function_viewer.wasm

emsdk_env() {
	if [[ ! -f $EMSDK_DIR/emsdk_env.sh ]]; then
		echo "no emsdk at $EMSDK_DIR: install Emscripten $EMSDK_VERSION (README: Setup) or set EMSDK" >&2
		exit 1
	fi
	# shellcheck disable=SC1091
	EMSDK_QUIET=1 source "$EMSDK_DIR/emsdk_env.sh" >/dev/null
	local version
	version=$(emcc --version | head -1 | sed -E 's/.* ([0-9]+\.[0-9]+\.[0-9]+) .*/\1/')
	if [[ $version != "$EMSDK_VERSION" ]]; then
		echo "emcc is $version; this build is verified with $EMSDK_VERSION (README: Setup)" >&2
		exit 1
	fi
}

native() {
	echo "build.sh native: cargo build -p function-viewer"
	cargo build -p function-viewer
	mkdir -p build/native
	cp target/debug/function-viewer build/native/function-viewer
	echo "built build/native/function-viewer"
}

# The web build is always `--release` (plan 1.6): a debug wasm is slow to
# load and the harness drives the web build at speed.
web() {
	emsdk_env
	echo "build.sh web: cargo build -p function-viewer --release --target wasm32-unknown-emscripten"
	cargo build -p function-viewer --release --target wasm32-unknown-emscripten
	# Assembled fresh: a stale index.js beside a new index.wasm throws at load
	# (the Odin run's M7 gamut), so the directory is rebuilt, not merged into.
	rm -rf build/web
	mkdir -p build/web
	cp "$WEB_JS" build/web/function-viewer.js
	cp "$WEB_WASM" build/web/function_viewer.wasm
	cp web/index.html build/web/index.html
	echo "built build/web/ (index.html, function-viewer.js, function_viewer.wasm)"
}

tests() {
	echo "build.sh test: cargo test --workspace"
	cargo test --workspace
}

# Plan 1.5: the release runs without Homebrew's SDL3. `bundled-sdl` compiles
# SDL 3.4.18 from source (cmake); the dylib rides beside the binary and the
# rpath finds it there. `--remap-path-prefix` keeps the worktree's path out
# of the binary (cargo has no per-profile rustflags on stable, so these ride
# RUSTFLAGS for this build only), and the web release is the web build under
# build/release/web/.
#
# i3-release hardened this: RUSTFLAGS remaps the worktree, the cargo home
# (the registry sources' panic locations), $HOME and emsdk out of every Rust
# object, and CFLAGS/CXXFLAGS map the same prefixes for the C objects (the
# vendored libraries natively and through emcc, SDL3's cmake build), because
# Rust embeds source paths in panic locations and `__FILE__` can do the same
# in C. `release_paths` then reads every printable run of the outputs' bytes
# (Xcode's `strings` skips a Mach-O's load commands, so it would miss the
# dylib reference) and fails the build on any path of this machine, allowing
# only the dynamic linker's load commands, the devices' paths (`/dev/std*`,
# `/proc/*`: SDL's own sources name them; they are the same on every
# machine) and, in the JS, Emscripten's in-browser file system. What remains
# is printed with why it is allowed.
release() {
	echo "build.sh release (native): bundled SDL3, rpath, remapped paths"
	: "${CARGO_HOME:=$HOME/.cargo}"
	RELEASE_RUSTFLAGS="--remap-path-prefix=$PWD=. --remap-path-prefix=$CARGO_HOME=cargo-home --remap-path-prefix=$HOME=home --remap-path-prefix=$EMSDK_DIR=emsdk"
	RELEASE_CFLAGS="-ffile-prefix-map=$PWD=. -ffile-prefix-map=$CARGO_HOME=cargo-home -ffile-prefix-map=$HOME=home -ffile-prefix-map=$EMSDK_DIR=emsdk"
	RUSTFLAGS="-C link-args=-Wl,-rpath,@loader_path $RELEASE_RUSTFLAGS" \
		CFLAGS="$RELEASE_CFLAGS" CXXFLAGS="$RELEASE_CFLAGS" \
		cargo build --release -p function-viewer --features bundled-sdl
	mkdir -p build/release/native
	cp target/release/function-viewer build/release/native/function-viewer
	cp target/release/libSDL3.0.dylib build/release/native/libSDL3.0.dylib
	echo "built build/release/native/ (function-viewer, libSDL3.0.dylib)"
	(
		emsdk_env
		echo "build.sh release (web)"
		RUSTFLAGS="$RELEASE_RUSTFLAGS" \
			CFLAGS="$RELEASE_CFLAGS" CXXFLAGS="$RELEASE_CFLAGS" \
			cargo build -p function-viewer --release --target wasm32-unknown-emscripten
		rm -rf build/release/web
		mkdir -p build/release/web
		cp "$WEB_JS" build/release/web/function-viewer.js
		cp "$WEB_WASM" build/release/web/function_viewer.wasm
		cp web/index.html build/release/web/index.html
		echo "built build/release/web/ (index.html, function-viewer.js, function_viewer.wasm)"
	)
	release_paths build/release/native/function-viewer build/release/native/libSDL3.0.dylib build/release/web/*
}

# No path of this machine may ship in the release outputs (adapted from the
# Odin app's build.sh `release_paths`, which this section mirrors: the
# release fails with the offending paths listed). Roots are the source tree,
# the home directory, the cargo home (registry sources) and emsdk. Allowed:
# the dynamic linker's load commands (the system's libraries and the bundled
# SDL beside the binary: otool's list), the devices' paths, and in JavaScript
# Emscripten's in-browser file system.
release_paths() {
	local file path found=() roots=("$PWD" "$HOME" "${CARGO_HOME:-$HOME/.cargo}" "$EMSDK_DIR") root load=""
	if [[ $(uname -s) == Darwin ]]; then
		load=$(otool -L build/release/native/function-viewer | tail -n +2 | awk '{print $1}')
		load+=$'\n/usr/lib/dyld'
	fi
	local total=0
	for file in "$@"; do
		for root in "${roots[@]}"; do
			[[ -z $root ]] && continue
			if LC_ALL=C grep -a -q -F "$root" "$file"; then
				found+=("$file: holds $root")
			fi
		done
		while IFS= read -r path; do
			[[ -z $path ]] && continue
			total=$((total + 1))
			case $path in
			/dev/* | /proc/* | /usr/lib/* | /System/Library/* | /lib/* | /lib64/*) continue ;;
			/home/web_user | /tmp/filedrop*) [[ $file == *.js ]] && continue ;;
			esac
			grep -qxF -- "$path" <<<"$load" && continue
			found+=("$file: $path")
		done < <(LC_ALL=C tr -c '[:print:]' '\n' <"$file" |
			grep -oE '/(Users|home|opt|private|var|tmp|usr|System|Library|Applications|Volumes|nix|root|mnt|srv|etc|dev|proc|bin|sbin|lib|lib64)(/[A-Za-z0-9._+@~%$-]+)+' |
			sort -u)
	done
	if [[ ${#found[@]} -gt 0 ]]; then
		printf '%s\n' "${found[@]}" >&2
		echo "release: paths of this machine in the outputs (above)" >&2
		return 1
	fi
	echo "release: no path of this machine in build/release/ ($total absolute paths, each the dynamic linker's, a device's or Emscripten's in-browser file system's)"
}

check() {
	echo "build.sh check: cargo fmt --all --check"
	cargo fmt --all --check
	echo "fmt: clean"
	echo "build.sh check: cargo clippy --workspace --all-targets -- -D warnings"
	cargo clippy --workspace --all-targets -- -D warnings
	echo "clippy (native): clean"
	# The mv-nvg twin test judges its render against the twin PNG, which is
	# gitignored (built by the Odin toolchain, not a dependency of the port):
	# a fresh clone has no copy, so the test would fail with "run
	# tools/twin/run first". Generate it when missing (web-blend; ~4 s warm),
	# so the check passes with no manual step.
	if [[ ! -f .local/shots/t0c-b/twin.png ]]; then
		echo "build.sh check: tools/twin/run (twin.png missing)"
		tools/twin/run
	fi
	tests
	native
	(
		emsdk_env
		echo "build.sh check: cargo clippy --workspace --target wasm32-unknown-emscripten -- -D warnings"
		cargo clippy --workspace --target wasm32-unknown-emscripten -- -D warnings
		echo "clippy (web): clean"
		web
	)
	if [[ -x tools/check ]]; then
		echo "build.sh check: tools/check"
		tools/check
	else
		echo "build.sh check: tools/check absent (t0b owns tools/), skipped"
	fi
	echo "check passed"
}

case "${1:-all}" in
native) native ;;
web) web ;;
test) tests ;;
release) release ;;
check) check ;;
all)
	native
	web
	;;
*)
	sed -n '2,16p' "$0"
	exit 2
	;;
esac
