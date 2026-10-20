# math-viz2-rust

A port of math-viz2, the user's React Function Viewer, to Rust in math-viz-rust's stack: SDL3, Clay, NanoVG, kb_text_shape through FFI, native and wasm. The plan and brief live in `.local/briefs/` (`math-viz2-rust.plan.md` and `math-viz2-rust.md`, with the progress index `math-viz2-rust.progress.md` and one record per milestone under `progress/`).

## The stack and the web build

t0a owns this section (t0c and i3-release append under their headings).

Rust 1.99.0 (`rust-toolchain.toml` pins the channel and the `wasm32-unknown-emscripten` target; `rustup target add wasm32-unknown-emscripten` installs it into the home directory). `sdl3 0.20.0` + `sdl3-sys 0.7.2+SDL-3.4.18` + `sdl3-main 0.7.0`, `glow 0.18.0`, `png 0.18.1`. Native dev/check builds link Homebrew's SDL3 3.4.18 through sdl3-sys's default `/opt/homebrew/lib` search (3 s clean, no setup); `bundled-sdl` (a `function-viewer` feature forwarding `sdl3/build-from-source`) compiles SDL from source for the release, with `-C link-args=-Wl,-rpath,@loader_path` and the dylib beside the binary; the web build always compiles SDL from source with emcc and links it statically (Emscripten's `-sUSE_SDL=3` port is not used). `source ~/emsdk/emsdk_env.sh` runs under `bash`; `zsh` mangles the script, so `build.sh` is bash.

- **The loop is SDL's main callbacks on both targets** (plan call 3): `#[sdl3_main::app_impl]` on the `App` in `function-viewer/src/main.rs` (`app_init/iterate/event`; `app_quit` omitted until the app needs it). SDL's objects are main-thread-only, so they ride in `MainThreadData`; `Box<Mutex<App>>` is the state. Measured on both targets in t0a (native hidden GL 4.1 Metal, web GLES3/WebGL2 in all three engines); no fallback needed. The fallback, if callbacks ever fail a target, is a hand-written loop (`SDL_WaitEvent`/`PollEvent` natively, `emscripten_set_main_loop` on the web) behind the same functions.
- **`.cargo/config.toml` starts from the wasm spike's flags byte for byte** (`ALLOW_MEMORY_GROWTH`, WebGL2 min/max, `FULL_ES3`, `EXPORTED_RUNTIME_METHODS`, `EXIT_RUNTIME`); every delta is numbered there with its three-engine evidence in `progress/t0a.md`. Delta 1: `--js-library web/library.js`. Kept, not dropped: `EXIT_RUNTIME`/`callMain` work under `sdl3-main` (three engines green, no console errors). emcc 6.0.12 bakes `function_viewer.wasm` into the JS (reads neither the script URL nor `Module.wasmBinaryFile`), so `build/web/` keeps cargo/emcc's emitted names and `index.html` loads `function-viewer.js`.
- **WebGL canvas rules, measured in t0a** (SDL 3.4.18 compiled from source, not Emscripten's port): the canvas composites on swap (no swap: black page); quitting zeroes the canvas (`Emscripten_DestroyWindow`: "We can't destroy the canvas, so resize it to zero instead"), so the web build idles after its frame instead of quitting; `--png` reads the back buffer before the swap (after it: all black). `index.html` merges a pre-seeded `Module` (`Object.assign(globalThis.Module || {}, …)`), because redefining it drops the harness's `Module.arguments`; the canvas keeps its `width`/`height` attributes matching the window.
- **Panics**: `panic = "unwind"` everywhere (the default); no `panic = "abort"`, no `-fwasm-exceptions` on the link (no C objects yet; t0c revisits). `MV_PANIC_AT_FRAME=1` (or `--panic-at-frame`, the web's way in via `Module.arguments`) panics in the first frame. Natively: `thread 'main' (<id>) panicked at <file>:<line>:` + message on stderr, exit **1**, not 101 (the unwind passes through SDL's C loop under `sdl3-main`). On the web the same lines reach the console as errors, no pageerror. Exact texts in `progress/t0a.md` for t0b's sweep regex.
- **Allocator**: Rust's default (`System`: `malloc` on emscripten). Floating point: Rust never contracts `a*b+c`; the C libraries get `-ffp-contract=off` (t0c's `build.rs`).
- **`./build.sh web` assembles `build/web/` fresh** (`rm -rf` first: a stale `.js` beside a new `.wasm` throws at load) and the web build is always `--release` (a debug wasm is slow to load).
- **i3-release's release facts**: `./build.sh release` remaps the worktree, the cargo home, `$HOME` and emsdk out of every object (`--remap-path-prefix` for Rust, `-ffile-prefix-map` for the C objects through `CFLAGS`/`CXXFLAGS`) and fails on any path of this machine left in the outputs (48 allowed remain: the dynamic linker's load commands, `/dev/*`, `/proc/*`, Emscripten's in-browser FS), proved failing once on the check build. The native release runs with no Homebrew (`@rpath/libSDL3.0.dylib`, LC_ID likewise, `DYLD_PRINT_LIBRARIES` shows the bundled dylib). A release panic still names `crates/…/main.rs:<line>` (relative through the remap; exit is 101 through the SDL loop, which re-panics on the poisoned lock). All wasm `-s` flags live in `function-viewer/build.rs` as `cargo:rustc-link-arg-bins=` (byte-identical strings), because an `RUSTFLAGS` in the environment replaces the config's `rustflags`: with them in `.cargo/config.toml` the release web silently lost `MIN_WEBGL_VERSION=2` and created a WebGL1 context (measured). `tools/run release` on the M3 six is 24/24 identical at threshold 0; `--time 300` medians on the release hold the 4 ms budget (1.47/1.57 offscreen at 1440x900, 2.32 at 5K). `build.bat` (CRLF, kept by `.gitattributes`) and the Linux note are unverified; `platform_windows.rs`'s `BOOL`s are 4-byte `i32` per Microsoft and windows-sys. Details in `progress/i3-release.md`.

## The Rust mapping

s-core owns this section (each porter appends examples under its crate's bullet). One table, applied everywhere (plan §1.3): math-viz2's names kept, idiomatic Rust inside; `unsafe` in `mv-sys`, the wrappers and the platform files only; every struct crossing the FFI gets a size test against the C side.
- function-viewer (shell ui): the `UI()(){children}` scope is `crate::ui::ui_scope`, which aliases `&mut App` for the children through raw pointers at that one call site (the safe `Clay::ui` cannot lend it); layout bodies call it, never `Clay::ui` directly. Clay constructor helpers used as pure data (`SizingGrow`, `SizingFixed`, `CornerRadiusAll`, blank declarations) are mirrored as real-data `pub(crate)` fns in `ui.rs` (`sizing_grow`, `sizing_fixed`, `corner_radius_all`, `empty_decl`, `ZERO_MINMAX`); `init_clay` passes arena capacity 0 because `Clay::new` folds `MinMemorySize` in, i.e. 0 asks for exactly the minimum.
- **mv-text** (text-shape): the kb call sequence runs through `mv-sys::kb_safe` (`ShapeContext`, `font_from_memory/is_valid/info/free`, `Run`, `ShapedGlyph`), so this `forbid(unsafe_code)` crate holds the context as `Option<ShapeContext>` (`None` until `init`; `pub fn uninit()` is the constructor the tests use) and every shaping borrow stays inside disjoint `Fonts` fields (`ctx` borrowed while `kb`/`units_per_em` read — never a whole-`Fonts` reborrow beside it, which is why `file_of` takes `&EnumMap`); a `Run` borrows its context (`shape_run`/`shape_codepoint` take `&self`), so the use-after-free the t0c-b review found (a `Copy` run kept past `end`/drop through the old free `glyph_next`, now removed) cannot compile.
- **mv-clay** (t0c-wire): the C error/measure callbacks keep fat pointers, which must point into heap boxes that outlive the call (`&param` dangles; the measure closure `Box::leak`s for want of an unregister hook); callers without a `&Clay` use the free functions on Clay's current context (a layout must be open), and payload readers key on `commandType` first (rectangle/border/image are owned copies, `custom_of::<T>` borrows the caller's `T` behind `customData`, `effect_of::<T: Copy>` copies out of `userData`, `command_at` OOB fires the error handler and segfaults with no live context); `Clay::text` copies into a frame thread-local store cleared at the next `begin_layout`, because Clay stores the pointer and re-measures it at `EndLayout` (the frame-local scratch shape); dropping a `Clay` clears a current-context pointing at it, since the C context lives in the freed arena.
- `function-viewer`: keyword names stay spelled (`app.r#loop`, `loop.rs` as `app_loop`); input handlers use `app.input.*` per statement with owned locals before `&mut App` calls (a held `&mut app.input` poisons the interleaved `app.dirty`/handler calls); raw SDL goes through `sdl3::sys` only where the safe crate cannot express it (`SDL_PushEvent` handle-free, `SDL_GetTicksNS`, `SDL_AddTimerNS`/`SDL_RemoveTimer` by id, `SDL_GetSystemTheme`), each in an `unsafe` block that dereferences nothing; undecoded SDL events land in `Event::Unknown` (match by the sys `SDL_EventType` consts); SDL `bitflags` have no combined `CTRLMOD`/`SHIFTMOD` (compose `LCTRLMOD | RCTRLMOD`); `collect_regions` takes the caller's `&mv_clay::Clay` (element data is its method, and no `App` field holds one).
- **mv-numfmt** (numfmt): `format!("{v:e}")` (no precision) is shortest-round-trip and always carries an `e`, so it stands in for `write_float` `'e'`/`-1` wherever digits are re-parsed; never format an unrounded float with `{:.N}` (Rust rounds ties to even, JS `toFixed` rounds them up — safe only after decimal `round_to`); spec-pinned literals like `3.14` keep `#[allow(clippy::approx_constant)]` per test with a comment naming its origin.
- **mv-script** (script): `EnumMap::iter()` yields variants in declaration order — `one_of` relies on it for the `takes one of a, b, …` lists (pinned by tests); Bit-set literals on modifiers are Rust `x | y` via enumset's `BitOr`.
- **mv-theme** (theme): `themes.rs` is generated by `tools/run themes` against the reference (`// generated, do not edit`, committed; a rerun changes nothing), tokens in CSS order.
- function-viewer (input-pinch): payloads the safe event enum drops are recovered by an `SDL_AddEventWatch` that records them keyed by the event timestamp (the safe `Unknown` keeps the timestamp: the join key), following the `darwin_watch_wheel` precedent; the store is a `static Mutex` (SDL may call the watch on another thread than the router: a `thread_local!` would hide it); the safe `EventType` lacks pinch and display-scale variants, which is why both arrive as `Unknown`; `SDL_PeepEvents` with a single-type range takes one event type back without disturbing other queued events.
- **function-viewer** (test-app): unit tests build `App` with `fixture::headless_app()` (the zeroed `App`: `window`/`gl_context`/`gl` are `None`, `vg` is `Context::null()`) and never open a window — a `cargo test` thread cannot start SDL video (measured 2026-10-18: `sdl3::init()` refuses off the main thread, and with one thread SDL answers "No available video device"). Window readers go through `App::window()`/`window_mut()`, which panic on `None`; `platform_refresh_rate` returns 0.0 headless.
- function-viewer (h2-p-url): the URL query codec is hand-rolled (no new dependency): `application/x-www-form-urlencoded` encode/decode as `URLSearchParams` does (space `+`, stray `%` literal, lossy UTF-8), pairs in a `Vec` so key order matches, then the literal-comma restore (`%2C`/`%2c` back to `,`) applied after the whole search string is built, exactly where the TS `replaceAll` sits; `trim_slashes` is split/filter/join (the regex's three alternatives remove exactly the empty segments — the 13 cases pin it); the debounce is one pending write per param armed in whole ms on the app's frame clock (callers pass `(now / MS) as u64`), fired by an explicit `step`, absorbed by an explicit `sync` with the `lastRaw` skip; test serialises print `format!("{:.N}", round_to(v, N))` for ASCII `-` (never bare `{:.N}` per the numfmt bullet, never `format_number`'s U+2212); the `hashchange` listener calls the `mv_hash_changed` export (on `EXPORTED_FUNCTIONS`; `replaceState` fires no `hashchange`, so our own writes never loop back), which pushes `UserCode::Hash` for the main thread to re-read.

## Harness notes

t0b owns this section (app-harness, i4-loop, i5 append). t0a's entries: windows open in the `agent` account's background login, which has no display, so every native result is a PNG the app writes (`--png` reads the back buffer before the swap); the web build needs no server (the harness serves `build/web/` by request routing; t0a's own shots used a plain `http.server`); never start servers of your own — a stray one holds a port and outlives you.
Only `act`, `states`, `release` and `keys` route the web build from disk; `web`,
`tabs`, `raf` and `loop` on the web target still need a server on `WEB_PORT`
(t0b's finding, restated: a milestone that runs them starts one in its own
tmux window on a free port and stops it after). Emscripten's main loop must
never be paused without `noExitRuntime=true` (the link bakes it in): pausing
pops the loop's keepalive and the runner's staleness check calls `maybeExit`,
which exits(0) a runtime with nothing alive — the page's app dies at its
first idle with `onExit(0)` and no later resume ever runs (i4-loop measured
it in WebKit). `sample` heads the Rust main
thread as `Thread_<id>: main` (std names it), and `tools/run loop` knows that head. `touch <rs> &&
./build.sh web` does not relink when only `web/library.js` changed (cargo
doesn't track the `--js-library` file): verify the new symbol lands in
`build/web/function-viewer.js` before probing. `MV_PANIC_AT_FRAME=1` panics
the native binary in frame 1 (scripted runs: `data_panics` in `app_init`;
interactive: `app_iterate`) and reaches the web build as `?panic-at-frame`
(`mv_panic_at_frame`, target `web` only); gate `./build.sh check` on its own exit code,
never on a pipe (`| tail` reports tail's status).
t0-harness's entries: math-viz2's storage keys are three JSON strings (`mathviz-light-dark`,
`mathviz-dark-theme`, `mathviz-light-theme`, one per key; no tour key), the port's three
under `math-viz2-rust-*`; the theme class is on `document.body` (main.tsx sets it); the
reference at af434dc renders the not-found page (no canvas), so `act`/`dom`/`raf` wait for
the body's class, not a canvas, until h1's plane; the theme files define the same tokens in
the same order in all six themes (31 at af434dc; --section, --chart-axis, --chart-grid-major
and --shadow-color join by 5cef052), shared values identical to the copied file's;
`tools/run themes` reads every token's stream through the reference in each scheme and fails
on the first mismatch, and its output drops math-viz's page tokens (ParamA and friends, the
chart points, the curve glow: theme.rs's tests and testdata/tokens.json still name them,
h1-frame's fix-forward). `views.json`'s `url`-line contract (kept from t0-shell-a): a line
holding only `url` is written whenever the page query changes (`--query` writes one at frame
0); no line reads as the empty query, and behaviour checks compare the last `url` line with
the reference's `location.hash` query.

## The references and the bar

The bar is the plan's §1.9 (`math-viz2-rust.plan.md` in `.local/briefs/`).

## Checking your work

The one check: `./build.sh check`.

## What the port settled

One block per milestone, appended in merge order; empty until the first milestone lands.

### h2-plane (the Cartesian plane's painters)

- The reference's canvas draw is fully characterised without screenshots: wrap
  `fillText`/`moveTo`/`lineTo`/`roundRect`/`stroke` in an init script and read
  every segment with its width+colour and every text with its font+pen
  (`.local/probe/` pattern). Canvas 2D centres strokes exactly like NanoVG,
  so grid/axis strokes sit on integer layout px with butt caps, one path per
  colour+width batch; there is no half-pixel offset at this hash.
- math-viz2's canvas bitmap is its content box (border-box minus the 1 px
  border); the CSS radius rounds afterwards. The port scissors the content
  box, paints, then cuts the four rounded triangles with page-coloured
  quarter-circle bezier cutouts and strokes the rim on the border box inset
  half a px (radius 3.5).
- `layout_labels` takes `&dyn Fn`: the `&mut Fonts` measure adapts through a
  local `RefCell` (never by touching the merged grid module).
- Canvas `textBaseline: "top"` puts the em-box top on the pen: baseline =
  top + face ascent at size; `"middle"` centres the ascent-descent span.
  Both rest on `mv_text::metrics` and are judged through the harness.
- The return-to-centre button renders on every view at this hash (probed in
  all fourteen states); `HtmlOverlay.tsx` holds no conditional.
- Eyeballing contact PNGs misleads at display scale: the default origin
  looked bottom-left but measured exactly centred. Measure, don't eyeball.
- `tools/live/` must exist for `mv-script`'s twin test only as a missing dir
  holding zero scripts (git keeps no empty directory); `tools/states/`
  scripts need no such guard while they hold states.

## Long runs

t0a owns this paragraph. This repo is built by many parallel milestones under the `long-run` skill: the brief (`math-viz2-rust.md`), the plan (`math-viz2-rust.plan.md`) and the progress index (`math-viz2-rust.progress.md`) live in `.local/briefs/` (gitignored scratch); each milestone's record is `.local/briefs/progress/<slug>.md`. Commit on the branch at each verified step (`./build.sh check && git add <paths> && git commit`, paths staged by name, joined with `&&` only). Edit only the files the plan's §5 assigns yours; a change needed in another milestone's file goes in the return, not the file. When a dependency merges into `claude-port`, merge it into the branch at a clean point (`git merge claude-port`, never a rebase).
