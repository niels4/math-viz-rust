# math-viz2-rust

**Live app:** https://niels4.github.io/math-viz-rust/

A port of the Function Viewer from [math-viz](https://github.com/niels4/math-viz) (React, TypeScript, Canvas 2D) to Rust: one codebase that builds a native desktop app on SDL3 and, compiled to WebAssembly, the browser app linked above. The port follows math-viz2's development history commit by commit; math-viz2 at each milestone's commit is the reference the harness compares against.

## Toolchain

| Tool | Version | Used for |
|---|---|---|
| Rust | `1.99.0` (`rust-toolchain.toml`; `wasm32-unknown-emscripten` target) | the app; `cargo` builds both targets |
| SDL3 | 3.4.18 (Homebrew `sdl3` natively for dev/check builds; compiled from source for the release and always on the web) | window, input, GL context, main callbacks |
| Emscripten | 6.0.12 through emsdk in `~/emsdk` | linking the web build |

Clay (`e6cc369`), NanoVG, Fontstash, stb_truetype, kb_text_shape (2.21) are vendored in `vendor/`, which says where each came from and what changed; emsdk is the one toolchain fetched outside it.

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
export REACT_REF_PORT=5190   # math-viz2 serves the page here
```

## Build and run

```sh
./build.sh native        # build/native/function-viewer
./build.sh web           # build/web/: index.html, function-viewer.js, function_viewer.wasm
./build.sh               # both
./build.sh test          # cargo test for the workspace
./build.sh check         # fmt, clippy for both targets, tests, both builds; exits non-zero on any failure
./build.sh release       # build/release/: native/function-viewer and web/, optimised (Release)
./build.sh pages         # docs/: the release web app as a self-contained static site (below)
```

- **Native window:** `build/native/function-viewer`. The welcome page at 1440x900 logical (density 2): the top row with the wordmark and the gear, the intro column, the hero plane, drawn in Display P3 on macOS (the window is tagged with it before its GL context). `--space srgb` draws sRGB instead. It draws only when something changed or moves; idle, SDL's loop waits on events. `--route function-viewer` opens the viewer directly (welcome by default, as the browser's `/`). The appearance prefs live in `prefs.json` under SDL's preference path (on macOS `~/Library/Application Support/mathviz/math-viz2-rust/`), the port's own key, never another app's; `--prefs DIR` keeps them in DIR instead. Quit with ⌘Q, the window's close button, or Ctrl-C (SIGTERM) in its terminal, idle or not.
- **Native frame to PNG:** `build/native/function-viewer --png out.png --size 1440x900 --density 2` draws the page into a hidden window's offscreen framebuffer at that size, writes it, and quits. In Display P3 (the default on macOS) the PNG carries the Display P3 profile; `--space srgb` writes the sRGB frame the harness compares with the browsers.
- **Theme, mode and space:** `--theme clean-teal`, `--mode light`, `--space srgb` override the stored prefs for one run (a script sets all three itself).
- **Harness runs:** `--script tools/states/default.act --out DIR` plays a state script and writes shots, `views.json`, `motion.json` and `time.json`; `--window` draws to the visible window instead of the offscreen buffer; `--time 300` redraws the script's last state 300 more times and reports median, 95th percentile and slowest; `--trace` logs each input and timer. The harness passes the script's `route` line as `--route` on every run.
- **GitHub Pages:** `./build.sh pages` builds the web app with the release flags (no path of this machine in the bundle, checked) and writes it into `docs/` at the repo root (with `.nojekyll`), committed, so the repository hosts it from Pages (Settings › Pages › main, `/docs`); every URL in the bundle is relative, so it serves from the repository subpath, at `/` (the root route) and `/#function-viewer`. Proved by `tools/run pages [--port P] [--base /math-viz2-rust/] [--engines ...]`: it serves `docs/` from the subpath on a local port and runs the web smoke plus the `welcome-route`, `default` and `url-roundtrip` states in each engine, every report at zero problems and the path unmoved. Rebuilt only by `./build.sh pages`, as the last step of a finished tree and by hand before a deploy — never by `./build.sh check` or a feature branch (a stale `docs/` in between is expected).
- **Browser:** serve `build/web/` as static files and open it, for example `python3 -m http.server 8000 --directory build/web` and `http://localhost:8000/` (the root route) or `http://localhost:8000/#function-viewer` (the viewer). A browser won't load wasm from `file://`. The main loop pauses while nothing moves and any input resumes it; `Module.iterations` and `Module.frames` count its turns and its frames. The prefs are the port's own keys (`localStorage['math-viz2-rust-light-dark']`, `math-viz2-rust-dark-theme`, `math-viz2-rust-light-theme`). `?trace` in the address logs each input and timer to the console; `?space=p3` or `?space=srgb` overrides the colour space, and the console says which it took.
- **Panic switch:** `MV_PANIC_AT_FRAME=1` (or `--panic-at-frame`, the web's way in through `Module.arguments`) panics in the first frame, so the sweep can prove it sees a crash on each build. Natively the process exits non-zero with `panicked at <file>:<line>` on stderr (101 through the SDL loop: the first panic poisons the loop's lock and the quit path panics unwrapping it); on the web the same lines reach the console as errors.
- **Smoke:** `tools/setup && tools/run states H0 --targets ref,web,native` runs the first state group on every target (needs the export in Setup); the report's identity columns read identical at threshold 0.

## The harness

One script puts the math-viz2 reference, the web build and the native build in the same state; output goes to `.local/shots/`. The reference is math-viz2 itself, served by Vite (page `#function-viewer` for the viewer); each history stretch has its own fixed reference worktree and port. Nothing in `tools/` hard-codes a port or a checkout path beyond a default: every command reads the reference from the environment, `REACT_REF` (the checkout) and `REACT_REF_PORT` (its port), and the web build's server from `WEB_PORT` where one is needed, served from the subpath in `WEB_BASE` (default `/`) where one is served from it.

- **Targets:** `ref`, `web`, `native` (`tools/run act --target ...`). The web build needs no server (`act`, `states`, `release` and `keys` route `build/web/` from disk at an origin no server listens on); `web`, `tabs`, `raf` and `loop` on the web target need one on `WEB_PORT`, started in a tmux window on a free port from 5200 and stopped after; `pages` serves `docs/` from a subpath on a server it starts and stops itself.
- **Scripts** (`tools/states/*.act`): lines before the first action say how the page opens (`size`, `density`, `scheme`, `mode`, `theme`, `reduced-motion`, `space`, `query`, `route`); after it, `size`, `scheme` and `reduced-motion` change it live. Positions are logical pixels, times milliseconds (the paused clock: time 0 is the first animation frame, `wait` runs the clock forward, animation frames fall every 16 ms). `query center=1,2&scale=2` is the page's hash query: appended to the reference and web URLs, passed as `--query` natively (the native twin parses it too: change `tools/src/script.py` and `crates/mv-script/src/script.rs` together). `route welcome|function-viewer` is the page route it opens on (function-viewer unless the script says welcome: the web URL always carries the hash, including the empty route; natively it rides as `--route`, welcome by hand). `hash center=3,1&scale=2` edits that query mid-run on every surface, keeping the script's route, through the page's own hashchange path, as an address-bar edit while it runs.
- **Storage keys:** the reference is seeded with math-viz2's keys (`mathviz-light-dark`, `mathviz-dark-theme`, `mathviz-light-theme`, one JSON string each); the web build with the port's own (`math-viz2-rust-light-dark`, `math-viz2-rust-dark-theme`, `math-viz2-rust-light-theme`). The port never shares runtime state with math-viz2.
- **The three toml files** (`tools/states/`): `milestones.toml` groups states by stretch; `tokens.toml` lists the (selector, token) pairs the port draws with; `boxes.toml` lists the named boxes `tools/run dom --fixture` writes to `crates/function-viewer/testdata/page.json`. Every stretch appends its states, tokens and boxes.
- **Writes:** each run writes its shots, any video and `report.json` (`views.json`, `motion.json`, `time.json` and `live.json` are checked against their shapes, a mismatch counting as a problem). `views.json` holds one object per shot with the exact keys (`shot,time,zoom,pan_x,pan_y,origin_x,origin_y,cursor,theme,scaled`); a line holding only `url` is the page's hash query, written whenever it changes (`--query` writes one at frame 0, `hash` whenever it changes it; runs without one write none, which reads as the empty query). A line holding only `route` is the page's hash route (`""` on the welcome page, `"function-viewer"` on the viewer), written at start and whenever the route changes. Behaviour checks compare the last `url` line with the reference's `location.hash` query after every scripted input, and the last `route` line with its hash route the same way. `tools/run states` lays the surfaces' shots on a contact sheet and compares every pair at threshold 0; `tools/run judge` counts the native shot's pixels apart from the reference per engine (flat and more than 2 levels apart, clustered); `tools/run themes` regenerates `crates/mv-theme/src/themes.rs` through the reference; `tools/run font` instances math-viz2's variable woff2 into `assets/fonts/`; `tools/run gamut` checks Display P3 (its allowances start empty).
- **Parallelism:** `tools/run states --jobs N` runs N state-surface pairs at once (default 8: the smallest level within 10 % of the best wall time with zero differences, measured on 24 cores / 128 GB, natively and in WebKit):

  | jobs | wall | load (1 min, at end) | `same-run` differences |
  |---|---|---|---|
  | 1 | 80 s | 6.75 | — (first) |
  | 8 | 50 s | 20.07 | 0 vs jobs 1 |
  | 24 | 50 s | 12.71 | 0 vs jobs 8 |

## Layout

The app lives in one binary crate (`crates/function-viewer/src/`), one module per domain; the leaf crates are copied from the first port and re-pinned to math-viz2 (`crates/mv-text`, `mv-theme`, `mv-numfmt`, `mv-script`, plus the FFI crates `mv-sys`, `mv-clay`, `mv-nvg`).

| Module | Holds |
|---|---|
| `main.rs`, `app.rs`, `app_loop.rs`, `loop.rs` | SDL main callbacks, the `App` state, the frame loop with the paused script clock |
| `page.rs`, `sidebar.rs`, `draw.rs`, `ui.rs` | the page dispatch per route, the sidebar column, the draw-command walk, Clay scopes and colours |
| `theme_state.rs`, `transition.rs`, `prefs.rs` | the three theme selections with system scheme, prefs round-trip, the 250 ms linear cross-fade |
| `top_bar.rs`, `settings.rs`, `icons.rs` | the top bar with wordmark and gear, the settings menu, the NanoVG icon paths |
| `url.rs`, `harness.rs`, `live.rs` | the hash query as app state, the script harness (`--script`, `--png`, `views.json`), the `--live` timeline |
| `camera.rs`, `plane_input.rs`, `plane.rs`, `grid.rs` | drag/flick/glide/wheel camera, its SDL wiring, the plane painters, the grid layout |
| `scale_legend.rs`, `overlay.rs` | the scale legend, the return-to-centre button and tooltip |
| `input.rs`, `fixture.rs` | SDL event decoding with hit kinds and regions, the headless test `App` |
| `functions.rs`, `plot.rs` | the four base functions, the plotted switch animation |
| `selector.rs` | the function picker card |
| `readout.rs`, `state.rs` | the value and form lines as token runs with fit-to-width, hover/focus/lit |
| `transform_section.rs`, `transform_row.rs`, `number_field.rs` | the Transform card, its rows with typed fields, the caret model |
| `scrub.rs`, `ruler_marks.rs` | the scrub maths, the ruler tape lattice |
| `scrub_value.rs`, `ruler.rs`, `reset.rs` | the value-box drag with tips, the ruler tape, the reset buttons |
| `transform_curves.rs`, `shown_transform.rs`, `flip.rs` | the transform application, the flip spring, the flip state |
| `plane_geometry.rs`, `plane_markers.rs` | canvas positions (handles, lines, markers), the P/Q marker painters |
| `points_section.rs`, `point_slider.rs`, `x_field.rs`, `tip.rs` | the Points card with readout and badge, the x-of-P slider, the x field, the shared tip |
| `point_format.rs`, `visible_range.rs`, `moved_by.rs` | the P readout formatting, the visible plot range, the moved badge |
| `welcome.rs`, `welcome_loop.rs`, `welcome_backdrop.rs`, `fit_scale.rs`, `fit.rs` | the welcome page, its 22-step loop, the paper-and-lights backdrop, the fit-to-window scale |
| `typography.rs`, `svgpath.rs`, `draw.rs` | font registration and faces, SVG path drawing, NanoVG helpers |
| `platform_native.rs`, `platform_web.rs`, `platform_darwin.rs`, `platform_unix.rs`, `platform_windows.rs` | per-platform glue (prefs path, cursors, navigation, appearance) |

## Release

```sh
./build.sh release                         # build/release/native/function-viewer and build/release/web/
build/release/native/function-viewer --script tools/states/default.act --out /tmp/t --time 300
```

`./build.sh release` builds both targets optimised with `bundled-sdl` natively: SDL compiled from source, `libSDL3.0.dylib` beside the binary with an `@loader_path` rpath, so it runs with no Homebrew on the machine. Panics stay `unwind` with source locations by file name, but the outputs hold no path of the machine that built them: `RUSTFLAGS` remaps the worktree, the cargo home, `$HOME` and emsdk out of every Rust object, `CFLAGS`/`CXXFLAGS` do the same for the C objects, and the build then fails if a byte scan of the outputs finds any of those roots or any other absolute path but the dynamic linker's load commands, the devices' (`/dev/std*`, `/proc/*`) and, in the JavaScript, Emscripten's in-browser file system. To look yourself:

```sh
strings -a build/web/function_viewer.wasm | grep -c -F -e "$PWD" -e "$HOME"    # the check build: its registry panic paths
strings -a build/release/web/function_viewer.wasm | grep -c -F -e "$PWD" -e "$HOME"    # the release: 0
LC_ALL=C tr -c '[:print:]' '\n' < build/release/native/function-viewer |
  grep -oE '/(Users|home|opt|private|var|tmp|usr|System|Library|Applications|Volumes|nix|root|mnt|srv|etc|dev|proc|bin|sbin|lib|lib64)(/[A-Za-z0-9._+@~%$-]+)+' | sort -u
# the load commands (the system's libraries, the bundled SDL) and /dev/std*: 48 lines, each allowed
```

The release takes every option the check build takes and draws the same frames: `tools/run release` runs states on both builds, natively and in WebKit, Chromium and Firefox, loading both builds' `web/` through request routing (no server), and fails on a pair that differs at threshold 0 or on a console error. The two builds' web outputs are not byte-for-byte reproducible (the embedded path strings differ by construction), so builds are compared by their frames.

**The redraw on the release build.** The budget for a full redraw at density 2 is 4 ms, judged on the release build. `--time 300` redraws the script's last state 300 more times and reports the median, the 95th percentile and the slowest. Measured on the release build (medians are load-robust, tails are not): 1.47/1.57 ms offscreen at 1440x900 (95th 3.15, slowest ~6.5), 2.32 ms at 5K. The debug build `check` makes is over the budget everywhere.

## Platforms

macOS (native) and the browser (WebKit, Chromium, Firefox) are verified here. Windows and Linux are kept buildable, every platform call behind one interface, and neither has been built or run, so what follows is unverified. Both draw in sRGB, since SDL tags no window's colour space there.

### Windows (unverified)

`build.bat` takes `build.sh`'s steps for `native` and `release`. It has never been run: there is no Windows machine here. It needs Rust 1.99.0 on `PATH`, Visual Studio 2022's C++ build tools run from the "x64 Native Tools Command Prompt" (`cl` builds the vendored C libraries and SDL3 from source: Windows has no system SDL3), CMake on `PATH`, and for the web release emsdk with Emscripten 6.0.12. The prefs live in `%APPDATA%\mathviz\math-viz2-rust\prefs.json`, the port's own key.

### Linux (unverified)

`build.sh` builds on Linux as on macOS, with `cc`/`cmake` compiling the vendored C libraries and SDL3 (install the X11/Wayland/EGL development packages, `tools/setup` for the harness, Rust 1.99.0 on `PATH`). `./build.sh release` works as on macOS (its `bundled-sdl` compiles SDL3 from source, so no system SDL3 is needed). The prefs live in `~/.local/share/mathviz/math-viz2-rust/prefs.json`.

## Where the port stands

At math-viz2's `68e1c7b`: the page shell with the top bar and settings menu, the Cartesian plane with pan, flick, wheel zoom and the URL camera, the sidebar with the function picker, the readout and transform rows with scrub boxes, rulers and flip, the damped flip and the shift/scale handles with their lines, the P/Q points with markers, drop lines and slider, and the welcome screen at the root route with its loop and fit-to-window — the stretch from the first history milestone (h1) to the welcome screen (h10), judged state by state against math-viz2 at each milestone's commit.

## Your own checks

What the harness can't measure, because it runs headless with synthesised input: no display, no real pointer or keyboard, Playwright's WebKit instead of Safari.

- **A real trackpad and mouse on the plane:** drag, flick (a fling glides on and eases to a stop), wheel and two-finger scroll zoom about the cursor, pinch to zoom. Check: in the native window and in Safari, beside math-viz2 in Safari.
- **The scrub drag with a real mouse:** a value box drag (moves after 3 px; Shift fine at a tenth, Ctrl snaps; the ruler rides under k's row), the wheel over a box, arrow nudges, Esc revert, double-click reset, both tips. Check: in the native window and in Safari.
- **The handle and slider drags with a real mouse:** the shift diamond and scale square (both params lit, rulers out while held), the x-of-P slider (thumb jumps to the pointer, arrows/Home/End, Enter opens the field). Check: in the native window and in Safari.
- **The welcome loop at real speed:** the 22-step tour through scrubs and switches, natively and in Safari (the harness judges paused-clock frames, not motion). Check: `build/release/native/function-viewer` left running on the welcome page, and the web build in Safari.
- **The fit in a real window:** resize a Safari window across 1024x640, 1280x1000, 1280x800 and 768x1024 (every part inside, no scrolling; the menu under the gear at natural size in a small window; the plane sharp on a 2x and a 1x screen). Check: the web build in a real Safari window.
- **Safari itself:** the web build beside math-viz2 at 1440x900. Tab walks the DOM order, stopping at fields in Safari only with "Press Tab to highlight each item" on (buttons never take focus on click there); the port walks the same DOM order everywhere.
- **The composited colour on your display:** Digital Color Meter in Display P3 on a param chip in the native window, in the web build in Safari, and in math-viz2 in Safari — the same.
- **Text weight under macOS smoothing:** the browsers' text carries more ink than the port's at small sizes; kept as drawn. Check: the title and the readout beside math-viz2 in Safari.
- **The system theme flipping the running app:** Appearance Light/Dark in System with the menu open and closed (the page follows at once with its 250 ms fade); Reduce motion on (switches land at once, no frames).
- **The view transition's feel:** the 250 ms linear cross-fade on a theme pick, real trackpad and display vs the paused clock.
- **Prefs across a real restart:** pick a theme and a mode, quit, launch again: the same (`prefs.json` under the app's own path); in Safari a reload keeps them (the port's own keys); math-viz2's own settings untouched either way.
- **The web build's URL:** reload and back/forward across `/` and `/#function-viewer` (history entries from the wordmark and the button), a theme pick recolours both routes.
- **The popover and menu feel:** the settings menu's open/close (its 160 ms animation is skipped in the port), the macOS focus ring vs the port's ring, the rest tip's 400 ms delay, the typing tip while typing.
- **The Arctic Ice Secondary by eye:** the recolored Secondary against the card background, dark room and bright room, in a live window.
- **A Windows and a Linux build:** build and run each (Platforms), and its `--script` frames against macOS's (`tools/run diff`).

## Known gaps

What the port leaves out, one line each with how you check it yourself.

- **Screen readers.** SDL and Clay expose no accessibility tree, so a screen reader finds nothing on the page; the keyboard model (Tab, focus rings, every key) is what remains. Check: VoiceOver over the native window and the web build.
- **The not-found page and the dev pages.** The template's not-found route and the developer theme-demo page are not ported; an unknown hash shows the welcome page. Check: any other `#word` in the web build.
- **The font-loading state.** Natively fonts are always loaded, so labels always show; a cold web load may flash fallback faces before the fonts arrive, unmeasured. Check: a cold load with the cache off.
- **Click-to-caret placement.** Clicking into a typing field collapses the caret to the end; only whole scalars delete. Check: click mid-text while typing, natively.
- **Touch.** One finger may act as the mouse, but pinch and multi-touch are untested on a phone or tablet. Check: the web build in Safari on an iPhone and an iPad.
- **Static text doesn't select.** Field text selects while editing, but static labels never do. Check: drag across a row name in the port.
