# math-viz2-rust

A port of math-viz2, the user's React Function Viewer, to Rust in math-viz-rust's stack: one codebase that builds a native desktop app and, compiled to WebAssembly, a browser app. The reference of record is math-viz2 itself at each milestone's hash, served on port 5190. The rest of this README is wrap's.

## Toolchain

| Tool | Version | Used for |
|---|---|---|
| Rust | `1.99.0` (`rust-toolchain.toml`; `wasm32-unknown-emscripten` target) | the app; `cargo` builds both targets |
| SDL3 | 3.4.18 (Homebrew `sdl3` natively for dev/check builds; compiled from source for the release and always on the web) | window, input, GL context, main callbacks |
| Emscripten | 6.0.12 through emsdk in `~/emsdk` | linking the web build |
| Python | 3.14 (Homebrew), the `tools/` venv (t0b) | the harness |
| Playwright browsers | chromium 1243, firefox 1543, webkit 2359 in `~/Library/Caches/ms-playwright` | screenshots of the web build and the reference |

Clay (`e6cc369`), NanoVG, Fontstash, stb_truetype, kb_text_shape (2.21) are vendored in `vendor/` (t0c), which says where each came from and what changed; emsdk is the one toolchain fetched outside it.

## Setup

```sh
rustup target add wasm32-unknown-emscripten   # home directory, no admin needed

git clone https://github.com/emscripten-core/emsdk.git ~/emsdk
~/emsdk/emsdk install 6.0.12
~/emsdk/emsdk activate 6.0.12
```

`build.sh` sources `~/emsdk/emsdk_env.sh` itself (set `EMSDK` for another place; source it under `bash`: `zsh` mangles the script) and stops if `emcc` isn't 6.0.12. Native dev builds link Homebrew's SDL3 (`brew install sdl3`; needs an admin account) with no other setup.

The harness compares against the math-viz2 reference: it needs the reference server's port:

```sh
export REACT_REF_PORT=5190   # math-viz2 serves #function-viewer here
```

## Build and run

```sh
./build.sh native        # build/native/function-viewer
./build.sh web           # build/web/: index.html, function-viewer.js, function_viewer.wasm
./build.sh               # both
./build.sh test          # cargo test for the workspace
./build.sh check         # fmt, clippy for both targets, tests, both builds; exits non-zero on any failure
./build.sh release       # build/release/: native/function-viewer and web/, optimised (Release)
```

- **Native window:** `build/native/function-viewer`. The page at 1440x900 logical (density 2): the top bar, the sidebar cards, the plane, drawn in Display P3 on macOS (the window is tagged with it before its GL context). `--space srgb` draws sRGB instead. It draws only when something changed or moves; idle, SDL's loop waits on events. The appearance prefs live in `prefs.json` under SDL's preference path (on macOS `~/Library/Application Support/mathviz/function-viewer-rust/`), the port's own key, never another app's; `--prefs DIR` keeps them in DIR instead. Quit with ⌘Q, the window's close button, or Ctrl-C (SIGTERM) in its terminal, idle or not.
- **Native frame to PNG:** `build/native/function-viewer --png out.png --size 1440x900 --density 2` draws the page into a hidden window's offscreen framebuffer at that size, writes it, and quits. In Display P3 (the default on macOS) the PNG carries the Display P3 profile; `--space srgb` writes the sRGB frame the harness compares with the browsers.
- **Theme, mode and space:** `--theme clean-teal`, `--mode light`, `--space srgb` override the stored prefs for one run (a script sets all three itself).
- **Harness runs:** `--script tools/states/default.act --out DIR` plays a state script and writes shots, `views.json`, `motion.json` and `time.json`; `--window` draws to the visible window instead of the offscreen buffer; `--time 300` redraws the script's last state 300 more times and reports median, 95th percentile and slowest; `--trace` logs each input and timer.
- **Browser:** serve `build/web/` as static files and open it, for example `python3 -m http.server 8000 --directory build/web` and `http://localhost:8000/`. A browser won't load wasm from `file://`. The main loop pauses while nothing moves and any input resumes it; `Module.iterations` and `Module.frames` count its turns and its frames. The prefs are `localStorage['mathviz-function-viewer-rust']`, the port's own key. `?trace` in the address logs each input and timer to the console; `?space=p3` or `?space=srgb` overrides the colour space, and the console says which it took.
- **Panic switch:** `MV_PANIC_AT_FRAME=1` (or `--panic-at-frame`, the web's way in through `Module.arguments`) panics in the first frame, so the sweep can prove it sees a crash on each build. Natively the process exits non-zero with `panicked at <file>:<line>` on stderr (101 through the SDL loop: the first panic poisons the loop's lock and the quit path panics unwrapping it); on the web the same lines reach the console as errors.
- **Smoke:** `tools/setup && tools/run states H0 --targets ref,web,native` runs the first state group on every target (needs the export in Setup); the report's identity columns read identical at threshold 0.

## The harness

t0-harness writes this section with `tools/`. One script puts the math-viz2 reference, the web build and the native build in the same state; output goes to `.local/shots/`.
