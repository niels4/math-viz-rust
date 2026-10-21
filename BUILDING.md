# Building and checking math-viz2-rust

The reference for building, running and checking the port. The README has the short version.

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

- **Native window:** `build/native/function-viewer`. The welcome page at 1440x900 logical (density 2): the top row, the intro column, the hero plane, drawn in Display P3 on macOS (the window is tagged with it before its GL context). `--route function-viewer` opens the viewer instead. `--space srgb` draws sRGB instead. It draws only when something changed or moves; idle, SDL's loop waits on events. `--route function-viewer` opens the viewer (welcome by default, as the browser's `/`). The appearance prefs live in `prefs.json` under SDL's preference path (on macOS `~/Library/Application Support/mathviz/math-viz2-rust/`), the port's own key, never another app's; `--prefs DIR` keeps them in DIR instead. Quit with ⌘Q, the window's close button, or Ctrl-C (SIGTERM) in its terminal, idle or not.
- **Native frame to PNG:** `build/native/function-viewer --png out.png --size 1440x900 --density 2` draws the page into a hidden window's offscreen framebuffer at that size, writes it, and quits. In Display P3 (the default on macOS) the PNG carries the Display P3 profile; `--space srgb` writes the sRGB frame the harness compares with the browsers.
- **Theme, mode and space:** `--theme clean-teal`, `--mode light`, `--space srgb` override the stored prefs for one run (a script sets all three itself).
- **Harness runs:** `--script tools/states/default.act --out DIR` plays a state script and writes shots, `views.json`, `motion.json` and `time.json`; `--window` draws to the visible window instead of the offscreen buffer; `--time 300` redraws the script's last state 300 more times and reports median, 95th percentile and slowest; `--trace` logs each input and timer. The harness passes the script's `route` line as `--route` on every run.
- **GitHub Pages:** `./build.sh pages` builds the web app with the release flags (no path of this machine in the bundle, checked) and writes it into `docs/` at the repo root (with `.nojekyll`), committed, so the repository hosts it from Pages (Settings › Pages › main, `/docs`); every URL in the bundle is relative, so it serves from the repository subpath, at `/` (the root route) and `/#function-viewer`. `tools/run pages --port <a free port>` proves it from that subpath: it serves `docs/` at `/math-viz2-rust/` on a local port and runs the smoke, welcome-route, default and url-roundtrip states in three engines (the pathname must stay unchanged).
- **Browser:** serve `build/web/` as static files and open it, for example `python3 -m http.server 8000 --directory build/web` and `http://localhost:8000/`. A browser won't load wasm from `file://`. The main loop pauses while nothing moves and any input resumes it; `Module.iterations` and `Module.frames` count its turns and its frames. The prefs are the port's own keys (`localStorage['math-viz2-rust-light-dark']`, `math-viz2-rust-dark-theme`, `math-viz2-rust-light-theme`). `?trace` in the address logs each input and timer to the console; `?space=p3` or `?space=srgb` overrides the colour space, and the console says which it took.
- **Panic switch:** `MV_PANIC_AT_FRAME=1` (or `--panic-at-frame`, the web's way in through `Module.arguments`) panics in the first frame, so the sweep can prove it sees a crash on each build. Natively the process exits non-zero with `panicked at <file>:<line>` on stderr (101 through the SDL loop: the first panic poisons the loop's lock and the quit path panics unwrapping it); on the web the same lines reach the console as errors.
- **Smoke:** `tools/setup && tools/run states H0 --targets ref,web,native` runs the first state group on every target (needs the export in Setup); the report's identity columns read identical at threshold 0.

## The harness

One script puts the math-viz2 reference, the web build and the native build in the same state;
output goes to `.local/shots/`. The reference is math-viz2 itself at the milestone's hash,
served by Vite in tmux session `math-viz2-rust-ref` (port 5190, page `#function-viewer`); each
history stretch has its own fixed reference worktree and port. Nothing in `tools/` hard-codes a port or a checkout path beyond a default: every
command reads the reference from the environment, `REACT_REF` (the checkout) and
`REACT_REF_PORT` (its port), and the web build's server from `WEB_PORT` where one is needed.

- **Targets:** `ref`, `web`, `native` (`tools/run act --target ...`). The web build needs no
  server (`act`, `states`, `release` and `keys` route `build/web/` from disk at an origin no
  server listens on); `web`, `tabs`, `raf` and `loop` on the web target need one on `WEB_PORT`,
  which a milestone starts in its own tmux window on a free port from 5200 and stops after.
- **Scripts** (`tools/states/*.act`): lines before the first action say how the page opens
  (`size`, `density`, `scheme`, `mode`, `theme`, `reduced-motion`, `space`, `query`, `route`); after it,
  `size`, `scheme` and `reduced-motion` change it live. Positions are logical pixels, times
  milliseconds (the paused clock: time 0 is the first animation frame, `wait` runs the clock
  forward, animation frames fall every 16 ms). `query center=1,2&scale=2` is the page's hash
  query: appended to the reference and web URLs, passed as `--query` natively (mv-script's
  twin parses it too: change `tools/src/script.py` and `crates/mv-script/src/script.rs`
  together). `route welcome|function-viewer` is the page route it opens on (function-viewer
  unless the script says welcome: the web URL always carries the hash, including the empty
  route; natively it rides as `--route`, welcome by hand). `hash center=3,1&scale=2` edits that
  query mid-run on every surface, keeping the script's route, through
  the page's own hashchange path, as an address-bar edit while it runs.
- **Storage keys:** the reference is seeded with math-viz2's keys (`mathviz-light-dark`,
  `mathviz-dark-theme`, `mathviz-light-theme`, one JSON string each, and `mathviz-theme`, the
  single key the reference reads before h1's three-key model: each reference reads its own);
  the web build with the port's own (`math-viz2-rust-light-dark`, `math-viz2-rust-dark-theme`,
  `math-viz2-rust-light-theme`). The port never shares runtime state with math-viz2.
- **Serving from a subpath:** `WEB_BASE` (e.g. `/math-viz2-rust/`) tells the harness where the
  web build is served; it must start and end with `/`. `tools/run pages` sets it for the
  GitHub Pages proof; every other run leaves it at `/`.
- **The three toml files** (`tools/states/`): `milestones.toml` groups states by milestone
  (`H0 = ["default"]`); `tokens.toml` lists the (selector, token) pairs the port draws with
  (the 35 theme tokens on `body` to start); `boxes.toml` lists the named boxes
  `tools/run dom --fixture` writes to `crates/function-viewer/testdata/page.json`. Every
  milestone appends its states, tokens and boxes. A group is any key of
  `milestones.toml` (`H0` to `H11`, and `FIXCURSOR` for the groups that are not a stretch
  of the history): `tools/run states FIXCURSOR` finds it.
- **Writes:** each run writes its shots, any video and `report.json` (`views.json`,
  `motion.json`, `time.json` and `live.json` are checked against their shapes, a mismatch
  counting as a problem). `views.json` holds one object per shot with the exact 10 keys
  (`shot,time,zoom,pan_x,pan_y,origin_x,origin_y,cursor,theme,scaled`); a line holding only
  `url` is the page's hash query, written whenever it changes (`--query` writes one at frame 0,
  `hash` whenever it changes it; runs without one write none, which reads as the empty query). A line
  holding only `route` is the page's hash route (`""` on the welcome page, `"function-viewer"` on the
  viewer), written at start and whenever the route changes. Behaviour checks compare the
  last `url` line with the reference's `location.hash` query after every scripted input, and the last
  `route` line with its hash route the same way.
  `tools/run states` lays the surfaces' shots on a contact sheet and compares every pair at
  threshold 0; `tools/run judge` counts the native shot's pixels apart from the reference per
  engine (flat and more than 2 levels apart, clustered); `tools/run themes` regenerates
  `crates/mv-theme/src/themes.rs` through the reference; `tools/run font` instances math-viz2's
  variable woff2 into `assets/fonts/`; `tools/run gamut` checks Display P3 (its allowances
  start empty).
- **Parallelism:** `tools/run states --jobs N` runs N state-surface pairs at once
  (default 8: the smallest level within 10 % of the best wall time with zero
  differences, t1-jobs on 24 cores / 128 GB, H1 H2 natively and in WebKit):

  | jobs | wall | load (1 min, at end) | `same-run` differences |
  |---|---|---|---|
  | 1 | 80 s | 6.75 | — (first) |
  | 8 | 50 s | 20.07 | 0 vs jobs 1 |
  | 24 | 50 s | 12.71 | 0 vs jobs 8 |

## Layout

One binary crate, `crates/function-viewer`, holds the app; the library crates
are the copied ones (`mv-sys`, `mv-clay`, `mv-nvg`, `mv-text`, `mv-theme`,
`mv-numfmt`, `mv-script`). The modules follow the domain, not the component
tree. One line each, as the plan's module map (§1.2) stands after h11:

| Rust module | What it owns |
|---|---|
| `app.rs` | the `App` state (the fields the whole app reads) |
| `app_loop.rs` | the frame loop and the clock |
| `camera.rs` | the camera: drag, flick, glide, wheel zoom, the scale limits, the URL write |
| `draw.rs` | the draw helpers |
| `fit.rs`, `fit_scale.rs` | the welcome page's fit to the window (the frame, the inverse hit test, the zoomed top row, the menu's anchor) |
| `fixture.rs` | the headless `App` the tests build |
| `flex.rs` | the flex layout helpers |
| `flip.rs` | the flip's jump list |
| `functions.rs` | the function slugs, their bases and the URL serialisers |
| `grid.rs` | the plane's grid, its labels |
| `harness.rs` | the scripted run: the shots, `views.json`, `motion.json`, the `loop` line |
| `icons.rs` | the SVG icons |
| `input.rs` | the pointer, wheel and key events, the hit regions, the cursors |
| `live.rs` | the `--live` script runner |
| `moved_by.rs` | who moved last (the moved badge) |
| `number_field.rs` | the typed value field |
| `overlay.rs` | the return-to-centre button, the overlay tip |
| `page.rs` | the page: the top bar, the sidebar column, the plane, the welcome page, the viewer's grid |
| `plane.rs` | the plane's painters through NanoVG |
| `plane_geometry.rs` | the plane's geometry and the hit test |
| `plane_input.rs` | the plane's pointer and wheel handling |
| `plane_markers.rs` | the P and Q markers' geometry and paint |
| `point_format.rs` | the point's readout slots |
| `point_slider.rs` | the x-of-P slider |
| `points_section.rs` | the Points card's rows |
| `prefs.rs` | the appearance prefs (the port's own key) |
| `readout.rs` | the value line and the form line as token runs |
| `reset.rs` | the reset and flip buttons' animation |
| `ruler.rs`, `ruler_marks.rs` | the scrub ruler |
| `scale_legend.rs` | the plane's scale legend |
| `scroll.rs` | the scroll container (the sidebar's Panel) |
| `scrub.rs`, `scrub_value.rs` | the scrub's modes, the value box's drag, wheel, keys and tips |
| `selector.rs` | the function picker |
| `settings.rs` | the settings menu |
| `shown_transform.rs` | the transform's spring |
| `sidebar.rs` | the sidebar's cards |
| `state.rs` | hover, focus, lit, highlight |
| `svgpath.rs` | the SVG path parser for the icons |
| `theme_state.rs` | the theme selections, the system scheme, the resolved theme |
| `tip.rs` | the tips' boxes and glyphs |
| `top_bar.rs` | the top bar |
| `transform_curves.rs`, `transform_row.rs`, `transform_section.rs` | the transform's curves, its rows and its card |
| `transition.rs` | the theme cross-fade |
| `typography.rs` | the type styles |
| `ui.rs` | the layout scope and the regions |
| `url.rs` | the URL as app state (`QueryParam<T>`) |
| `visible_range.rs` | the visible x range |
| `welcome.rs`, `welcome_backdrop.rs`, `welcome_loop.rs`, `welcome_step.rs` | the welcome page's layout, its backdrop, the tour and the frame step |
| `x_field.rs` | the P x field |

Not ported: the router itself, `_root.page.tsx`, `_not_found.page.tsx`, the dev
theme-demo page and the dev pages' links, `main.tsx`'s mounting, `useHotReload`,
the test harness under `src/test/`, `scripts/`, the `docs/` build, and the
screen-reader live region (`decisions.md` has the reasons).

## Your own checks

What the harness can't measure, because it runs headless with synthesised input: no display, no real pointer or keyboard, Playwright's WebKit instead of Safari.

- **A real trackpad and mouse on the plane:** drag, flick (a fling glides on and eases to a stop), wheel and two-finger scroll zoom about the cursor, pinch to zoom. Check: in the native window and in Safari, beside math-viz2 in Safari.
- **The scrub drag with a real mouse:** a value box drag (moves after 3 px; Shift fine at a tenth, Ctrl snaps; the ruler rides under k's row), the wheel over a box, arrow nudges, Esc revert, double-click reset, both tips. Check: in the native window and in Safari.
- **The handle and slider drags with a real mouse:** the shift diamond and scale square (both params lit, rulers out while held), the x-of-P slider (thumb jumps to the pointer, arrows/Home/End, Enter opens the field). Check: in the native window and in Safari.
- **Dragging P along the curve with a real mouse:** the open hand over P, the closed one while it is held, and the slider's thumb showing its dragging look. Check: in the native window and in Safari, beside math-viz2 in Safari.
- **Clicking the hero mid-loop:** the welcome page's graph is a link into the viewer, so a click on it navigates and the cursor is a hand over it. Check: the native window and Safari, while the welcome loop plays.
- **The welcome loop at real speed:** the tour through scrubs and switches, natively and in Safari (the harness judges paused-clock frames, not motion). Check: `build/release/native/function-viewer` left running on the welcome page, and the web build in Safari.
- **The fit in a real window:** resize a Safari window across 1024x640, 1280x1000, 1280x800 and 768x1024 (every part inside, no scrolling; the menu under the gear at natural size in a small window; the plane sharp on a 2x and a 1x screen). Check: the web build in a real Safari window.
- **The sidebar's scroll:** the sidebar's content is taller than a short window, so it scrolls; a wheel over it scrolls the panel, a wheel over the plane zooms. Check: the native window and Safari at 1024x640.
- **Safari itself:** the web build beside math-viz2 at 1440x900. Tab walks fields in Safari only with "Press Tab to highlight each item" on (the port follows the Chromium order); buttons don't take focus on click there.
- **The composited colour on your display:** Digital Color Meter in Display P3 on a param chip in the native window, in the web build in Safari, and in math-viz2 in Safari -- the same.
- **Text weight under macOS smoothing:** the browsers' text carries more ink than the port's at small sizes; kept as drawn. Check: the title and the readout beside math-viz2 in Safari.
- **The system theme flipping the running app:** Appearance Light/Dark in System with the menu open and closed (the page follows at once with its 250 ms fade); Reduce motion on (switches land at once, no frames).
- **The view transition's feel:** the 250 ms linear cross-fade on a theme pick, real trackpad and display vs the paused clock.
- **Prefs across a real restart:** pick a theme and a mode, quit, launch again: the same (`prefs.json` under the app's own path); in Safari a reload keeps them (the port's own keys); math-viz2's own settings untouched either way.
- **The web build's URL:** reload and back/forward across `/` and `/#function-viewer` (history entries from the wordmark, the button and the hero), a theme pick recolours both routes.
- **The popover and menu feel:** the settings menu's open/close (its 160 ms animation is skipped in the port), the macOS focus ring vs the port's ring, the rest tip's 400 ms delay, the typing tip while typing.
- **The Arctic Ice Secondary by eye:** the recolored Secondary against the card background, dark room and bright room, in a live window.
- **A Windows and a Linux build:** build and run each, and its `--script` frames against macOS's (`tools/run diff`).

## Known gaps

What the port leaves out, one line each with how you check it yourself.

- **Screen readers.** SDL and Clay expose no accessibility tree, so a screen reader finds nothing on the page; the keyboard model (Tab, focus rings, every key) is what remains. Check: VoiceOver over the native window and the web build.
- **The not-found page and the dev pages.** The template's not-found route, the developer theme-demo page and the two links that point at them are not ported (the user's ruling of 2026-10-20: the dev pages and their links are out of scope); an unknown hash shows the welcome page. The port's settings menu is **44 CSS px shorter** than math-viz2's, because the excluded "Developer pages" row (36 px plus its 8 px gap) is not drawn; the row's own box (`menu_dev_link`, 278 x 36 CSS) and the 44 px of card it leaves below the port's card bottom are the run's category (f), 11 506 of 31 580 px at 1024 x 640. Check: any other `#word` in the web build, and the foot of the settings menu.
- **The font-loading state.** Natively fonts are always loaded, so labels always show; a cold web load may flash fallback faces before the fonts arrive, unmeasured. Check: a cold load with the cache off.
- **Click-to-caret placement.** Clicking into a typing field collapses the caret to the end; only whole scalars delete. Check: click mid-text while typing, natively.
- **Touch.** One finger may act as the mouse, but pinch and multi-touch are untested on a phone or tablet. Check: the web build in Safari on an iPhone and an iPad.
- **Static text doesn't select.** Field text selects while editing, but static labels never do. Check: drag across a row name in the port.


## Release

`./build.sh release` builds `build/release/native/function-viewer` and
`build/release/web/` (the release wasm app) with the release flags, remaps
this machine's paths out of every object (`--remap-path-prefix` for Rust,
`-ffile-prefix-map` for the C objects through `CFLAGS`/`CXXFLAGS`) and fails
on any path of this machine left in the outputs. **48 lines are allowed to
remain**: the dynamic linker's load commands, `/dev/*`, `/proc/*` and
Emscripten's in-browser file system. The native release runs with no
Homebrew (`@rpath/libSDL3.0.dylib`, `LC_ID` likewise, the bundled SDL beside
the binary). A release panic still names `crates/…/main.rs:<line>` (relative
through the remap; the exit is 101 through the SDL loop).

`tools/run release` checks the release build against the debug one; `--time
300` redraws the script's last state 300 more times and reports the median,
the 95th percentile and the slowest. On the release build the medians hold
the 4 ms budget at density 2 (1.47/1.57 ms offscreen at 1440x900, 2.32 ms at
5K); the debug build `check` makes is over the budget everywhere.

## Platforms

macOS (native) and the browser (WebKit, Chromium, Firefox) are verified here.
Windows and Linux are kept buildable, every platform call behind one
interface, and neither has been built or run, so what follows is unverified.
Both draw in sRGB, since SDL tags no window's colour space there.

### Windows (unverified)

`build.bat` takes `build.sh`'s steps for `native` and `release`. It has never
been run: there is no Windows machine here. It needs Rust 1.99.0 on `PATH`,
Visual Studio 2022's C++ build tools run from the "x64 Native Tools Command
Prompt" (`cl` builds the vendored C libraries and SDL3 from source: Windows
has no system SDL3), CMake on `PATH`, and for the web release emsdk with
Emscripten 6.0.12. The prefs live in `%APPDATA%\mathviz\math-viz2-rust\prefs.json`,
the port's own key.

### Linux (unverified)

`build.sh` builds on Linux as on macOS, with `cc`/`cmake` compiling the
vendored C libraries and SDL3 (install the X11/Wayland/EGL development
packages, `tools/setup` for the harness, Rust 1.99.0 on `PATH`).
`./build.sh release` works as on macOS (its `bundled-sdl` compiles SDL3 from
source, so no system SDL3 is needed). The prefs live in
`~/.local/share/mathviz/math-viz2-rust/prefs.json`.
