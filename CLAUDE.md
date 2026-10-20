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
- **mv-numfmt** (t1-numfmt): exact JS `toFixed`/`toPrecision` are `to_fixed`/`to_precision` (sign is `x < 0.0` only — node: `(-0.00001).toFixed(4)` is `"-0.0000"`, `(-0).toFixed(4)` is `"0.0000"`; halves up; `|x| >= 1e21` falls back to `js_to_string`), built on `exact_digits` (`format!("{v:.1074}")` is the exact binary expansion, every binary fraction terminating within 1074 places) plus `round_half_up`; `js_to_string` is JS `String(x)` (Rust Display shortest digits through the spec's n/k branch); out-of-range digits `panic!` (the `RangeError` mirror); conformance tables in `crates/mv-numfmt/tests/fixtures/` (`gen_vectors.mjs` + `node-version.txt`, `include_str!` so wasm stays file-free).
- **mv-script** (script): `EnumMap::iter()` yields variants in declaration order — `one_of` relies on it for the `takes one of a, b, …` lists (pinned by tests); Bit-set literals on modifiers are Rust `x | y` via enumset's `BitOr`.
- **mv-theme** (theme): `themes.rs` is generated by `tools/run themes` against the reference (`// generated, do not edit`, committed; a rerun changes nothing), tokens in CSS order.
- function-viewer (input-pinch): payloads the safe event enum drops are recovered by an `SDL_AddEventWatch` that records them keyed by the event timestamp (the safe `Unknown` keeps the timestamp: the join key), following the `darwin_watch_wheel` precedent; the store is a `static Mutex` (SDL may call the watch on another thread than the router: a `thread_local!` would hide it); the safe `EventType` lacks pinch and display-scale variants, which is why both arrive as `Unknown`; `SDL_PeepEvents` with a single-type range takes one event type back without disturbing other queued events.
- **function-viewer** (test-app): unit tests build `App` with `fixture::headless_app()` (the zeroed `App`: `window`/`gl_context`/`gl` are `None`, `vg` is `Context::null()`) and never open a window — a `cargo test` thread cannot start SDL video (measured 2026-10-18: `sdl3::init()` refuses off the main thread, and with one thread SDL answers "No available video device"). Window readers go through `App::window()`/`window_mut()`, which panic on `None`; `platform_refresh_rate` returns 0.0 headless.
- function-viewer (h2-p-url): the URL query codec is hand-rolled (no new dependency): `application/x-www-form-urlencoded` encode/decode as `URLSearchParams` does (space `+`, stray `%` literal, lossy UTF-8), pairs in a `Vec` so key order matches, then the literal-comma restore (`%2C`/`%2c` back to `,`) applied after the whole search string is built, exactly where the TS `replaceAll` sits; `trim_slashes` is split/filter/join (the regex's three alternatives remove exactly the empty segments — the 13 cases pin it); the debounce is one pending write per param armed in whole ms on the app's frame clock (callers pass `(now / MS) as u64`), fired by an explicit `step`, absorbed by an explicit `sync` with the `lastRaw` skip; test serialises print `to_fixed(v, N)` (exact JS `toFixed`, ASCII `-`; never `format_number`'s U+2212); the `hashchange` listener calls the `mv_hash_changed` export (on `EXPORTED_FUNCTIONS`; `replaceState` fires no `hashchange`, so our own writes never loop back), which pushes `UserCode::Hash` for the main thread to re-read.
- function-viewer (h3-p-plot): the plot switch keeps the hook's shape as data (`Shown { func, old: Option, elapsed }`, `PlotCurve { func, alpha, draw_to }` — the TS field `fn` is renamed `func`, a keyword); graphed functions are plain `fn(f64) -> f64` pointers and identity is address comparison through `std::ptr::fn_addr_eq` (plain `==` on fn pointers is compiler-linted, including inside a derived `PartialEq`, so `PlotCurve` implements it by hand); the rAF loop is `switch_to` (the render-phase transition, `canAnimate()` in as a `bool` the coder wires) plus `step` (first frame carries `FIRST_FRAME_MS`, as `camera.rs`'s `first_frame`, but only on a rest-to-switch transition — a mid-switch change keeps the running clock, as the hook's loop is keyed on `switching`); `sample_curve` is `drawCurve`'s sampling without the paint (per-column points, `insideShare`, cumulative inside-length, interpolated stop), taking the canvas dims beside the grid, which stores none; vitest `toBeCloseTo(x, 2)` is `|diff| < 0.005`, and frame-driven tests accumulate time and diff it exactly like the hook's `now - last` so `elapsed` sums bit-identically.
- function-viewer (h5-p-scrub): the scrub maths keep the TS shapes (`ScrubMode` enum, `scrub_mode`/`wheel_dx` plain args since h4's rows are unmerged); `quantize` is `mv-numfmt::quantize` exactly (the TS rounds `|v|` then re-applies the sign — including negative exact halves — which is what that helper does, so no local rounding); `shown` formats via `round_to` plus a plain print (the field shows the keyboard hyphen, never `format_number`'s U+2212), `-0` normalised to `0`; the tape is an enum (`Shift`/`Scale`, never a `kind` field), `multiples` keeps an f64 counter exactly like the TS loop (the `1e20` case falls out empty through identical float ops), and label text is `String(Number(toFixed))` including the ECMA-262 exponential rule outside `[1e-6, 1e21)` and the first-hyphen-only true-minus replace (`−5e-7` keeps its exponent hyphen).
- function-viewer (h6-p-transform): the transform is `Transform { a, b, h, k }` (a struct — the fixed keys read as the TS literals do — with `get`/`with` for the two indexed uses, not an `EnumMap`); a transformed function is data (`TransformedFunction { base, transform }` with `value(x)`), never a closure, since a transformed curve cannot be a `fn` pointer; `transform_curves` returns `TransformedCurve { func, alpha, draw_to, dashed }` (the dashed flag rides the new curve type, `plot::PlotCurve` untouched) with the original untransformed under `Transform::IDENTITY`; the flip is `ShownTransform { jump, seen }` with `step(flip, transform, dt, can_animate)` (adoption split into pub `adopt`, which the press turn calls before any frame passes, plus `pub shown` for that turn's draw; the first-frame `FIRST_FRAME_MS`, the `durationMs` finish, the per-param `===` passthrough for typed-mid values; `Flip.seq` is the press identity behind the hook's `flip !== seenFlip`, set from a counter where the flip is created); the params type is `state::TransformParam` (h4's; this module's own copy deleted at the h6 merge); `plane_geometry` is pure canvas positions (`to_px`, `clamp_curve_y` with `OVERSHOOT_PX`, `handle_path`, `line_path` with `None` for zero length, `line_tick_halves` as the painter's single source) while widths, dashes and alphas stay the painters' (h6-handles); `PlotLine.label` is always `None` from `transform_lines` (the TS carries no label).
- function-viewer (h7-p-state): the highlight holds ordered lists (`hover`/`focus: Vec<TransformParam>`, `lead: Touch::{Hover, Focus}` defaulting to hover): `lit()` is the scrubbed params in a-b-h-k order while any `scrub_mode` entry is `Some`, else the lead side's list while non-empty, else the other's; `set_hover/set_focus(&[params], on)` store-and-take-lead on `on` and clear only on element-for-element equality off (`same_params`), so a handle's stale end never clears a newer list; an unconditional drop passes `&[]` with `on` (the emptied side is unreadable whichever side leads, exactly as the source's off leaves it). `scrub_mode: EnumMap<TransformParam, Option<ScrubMode>>` is the `scrubModeAtoms` record; `Jump { param, from, to }` is the atom's element and `Jumps { list, seq }` the `jumpsAtom` list, every `set`/`single` bumping `seq` even with equal values (the hook's fresh-array identity), so the flip's call stays one line (`app.state.jumps.single(param, from, to)`). The flip animation is unchanged apart from the identity: `ShownTransform { jump, seen: u64 }` with `step(&Jumps, transform, dt, can_animate)` folding every jump over the transform for the `from`/`to` ends; `transform_lines(lit: &[T], g, u, transform)` draws the dashed box once while a or b is lit, then one line per lit param in list order; `unit_point` is public; `transformHandles` is gone (the plane builds its own handles).

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
t1-hash's entries: `hash <query>` edits the page's hash mid-run on every surface (`act.py`
sets `location.hash` on ref/web through the page's own hashchange path; `harness.rs`
`sync_from_hash`es it plus a frame natively, since scripted frames bypass SDL's event queue
and a pushed `UserCode::Hash` would sit unhandled); the web build opens a script's `query`
at `#function-viewer?<query>` (a `?query` search was silently dropped: nothing reads it),
`?panic-at-frame` inserted before any hash, and base `run_native` passes `--query` (the same
4 lines as h2-input's branch: the merge keeps either); `hash` in a `--live` run fails naming
`LiveWhat::Hash` (the loop milestone's); dev tools pinned `==` in `tools/pyproject.toml`
(black 26.10.1, isort 9.0.2, ruff 0.17.1, pyright 1.1.414: newest with a minimal canonical
reflow — black 25.12 drifts 7 files, 25.1.0 23; bisect in scratch, never in the worktree).

## The references and the bar

The bar is the plan's §1.9 (`math-viz2-rust.plan.md` in `.local/briefs/`).

## Checking your work

The one check: `./build.sh check`.

## What the port settled

One block per milestone, appended in merge order; empty until the first milestone lands.

### h1-frame (from its progress file; its branch merged first)

- The theme change is a 250 ms LINEAR cross-fade, measured on the reference (getAnimations during a click; the plan's §1.4 guessed ease).
- Clay borders take no layout space (the browser's `border-box` does), so each bordered element's Clay padding is its CSS padding plus its border.
- `ThemeState`'s setters are pure; the side-effecting `App::set_*` wrappers save through prefs and start the transition.
- The bar shadow paints from `ShadowColor` at 0/6/18, skipped where transparent (the dark themes).

### h1-settings

- The settings menu is a floating Clay sibling anchored to the gear (RightTop to RightBottom, 8 px below), laid out only while open; the hidden popover takes no part in the layout. The closed menu still matches `display:flex` selectors in the DOM, so the layout test reads the open shots.
- Hover is positional but sticky-dropped: a pick that changes the theme drops hover until the next pointer move (measured: Chromium does not re-hover without a mouse move after the re-render); a same-value pick keeps it. Enforced by `hover_held` in `retarget`, cleared on pointer (or button-pos) move. Keyboard focus lives separately in `input.focus`.
- One `HitKind::Gear` region (h1-frame's) serves the click; the menu never declares its own gear region. Theme rows resolve hover/secondary/swatch tokens under the ROW's theme class, not the page theme.
- Icons take 0-to-1 colours: a settings icon converts its 0-to-255 Clay ink with `nvg_color` at the layout boundary (unconverted inks clamp to white).
- The transition snapshot must be captured after `end_frame` (mid-frame `read_pixels` reads the unflushed backbuffer), and the loop must keep turns coming + stay awake while a transition is active, or the fade never animates past its first frame.
- Menu rows/labels hover with their own secondary; the checked row has no background (weight + check only); `:hover` beats `.selected` on rows while `:has(:checked)` beats `:hover` on cells (both measured).
- Chromium/Firefox don't capture the timed fade under the harness like WebKit: chromium runs ~180 ms ahead of script time, firefox shows all-final frames (engine quirk, measured per-frame). The port's fade is exactly linear-250 on the script clock.

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

### h4-rows (the Transform card's rows)
- The 9.5 px box side padding is fractional (Clay's padding and gaps are
  whole px): 3.5 px edge spacers plus the 6 px gap hold each side, as the
  picker's dividers hold its 1 px. Lay the card out and read every element's
  box before painting anything: the layout test decides, not the eye.
- A missing inset ring is a paint-order bug, not a missing stroke: CSS
  paints the inset shadow inside the border (two full border rows plus one
  ring row, measured), so the ring path sits a border inside the box. And a
  snapped hairline rect is a moved hairline: snapping quarter-device offsets
  drags the stroke half a device pixel off its row — snap the box, keep the
  ring rect exact.
- Later declarations win `hit_at` ties: declare a row's region before its
  children so the field's region wins their overlap.
- A refused commit still repaints: taking the draft always dirties, or the
  rejected text sits stranded until the next interaction redraws.
- Text input runs on SDL_StartTextInput while a field holds the focus
  (native and web; the harness's `tap` only queues `Text` while it is
  active); headless tests guard the window. Ctrl/Command+A is a `selected`
  flag the next text replaces; the first text starts the draft from the
  shown value.
- The hover highlight syncs in `retarget` on a hover change (leave then
  enter, as the DOM fires them; moves within one parameter skip so the
  touch never re-stamps); the focus side syncs at its explicit setters.
  One sync aware of every producer hit beats two partial syncs that fight.
- Eyeballing downscaled screenshots misleads (a correct pair once read
  swapped): md5 identity of reruns plus DOM reads settle it. Measure, don't
  eyeball.

### h5-buttons (the reset and flip buttons, Reset all)

- A kept slot beats a conditional element: the reset's 24 px slot stays in
  every row whether or not the button shows, so the flip toggle and the box
  never move (name 107 wide on scale rows, 143 on shift rows — five children
  means four gaps, not three).
- Mouse clicks blur, keyboard clicks don't: the flip and reset blur on
  `detail > 0`, Reset all never blurs, and a keyboard Space/Enter keeps the
  focus (and its ring). The blur is what lets the row go unlit once the
  pointer moves off.
- Tab must fall through to the milestones when the settings menu is closed:
  `settings::key` consuming Tab broke every Tab state (focus moved, the
  highlight never followed). The milestones walk their own stops and sync,
  with a default arm that walks on from any other stop.
- The hover must stamp on change only: re-stamping it every frame in
  `retarget` keeps it ahead of the focus forever, so a Tabbed focus never
  takes the lead. Move the unconditional sync inside the change.
- WebKit skips buttons on Tab (no Full Keyboard Access in the launched
  browser): a flip-focus state lands on the box in ref-webkit and on the
  toggle everywhere else. The port implements the DOM order; webkit is the
  recorded engine quirk, chromium/firefox the check.
- Opaque-then-glyph, never glyph-then-opaque: a lit tint panel covers
  whatever paints before it (the form's `+` prefix went missing). Compute
  the term's outer box first, paint the tint, then draw.
- An inset underline follows the radius: ink the full rounded panel, cover
  all but the band with the tint (top corners rounded for the cover), and
  the bar wraps the corner exactly.
- The readout's 78 px moves everything below it 18 px: every absolute-y
  script coordinate in later states goes stale (h4's row clicks missed their
  fields and a Backspace navigated WebKit back). Re-probe, don't trust.

### h5-box (the scrub value box)

- The scrub rebase (`changeMode`) moves the x anchor to the last x and
  re-reads `from` as the live (pre-snap) value: `scrubbed(0.6, 30, snap)` is
  1 and `scrubbed(0.6, 50, snap)` is 2. Rebase-then-send is the whole drag;
  probed live, not just read.
- `stop()` flushes whenever the press dragged — including the `buttons == 0`
  move path, which shares the function rather than cancelling silently.
- The box's whole 107×32 carries the field hit (the DOM field takes no
  pointer events until it opens); row padding keeps the row hit. Hover syncs
  treat both as the parameter, so padding-to-box moves never re-stamp.
- Capture is conditional: a scrub holds the hover, a tap into the focused
  box never does (the DOM returned early, so hover tracks). `captures()` stays
  plane-only; the scrub's condition lives beside it in `retarget`.
- `press()` clears the focus stop before routing, so a tap restores it from
  the highlight state's still-set focus side; a scrub onto another box
  commits the old draft first (DOM order: mousedown blurs, then the press).
- Double clicks synthesize from the up fact's click count (SDL pairs by
  time and distance on both targets, as the browser pairs `dblclick`): a tap
  release with clicks == 2 inside a twitch resets, with clicks == 1 it
  collapses the caret. A scrub release ignores clicks (a drag can end near
  a paired down).
- Debounced URL writes need two loop properties, or quiet pages never flush:
  pending writes keep the loop awake (`params_pending`, as an animating
  camera does) and a fired write repaints (the readout reads live params,
  not URL text — h4-readout's `live_values(url)` goes stale mid-drag).
- The rest tip arms on box-hit transitions (row padding into the box counts,
  as the DOM fires the box's enter) with `input.time` (never the layout's 0
  between facts) through `Timer::ScrubRest`, so the harness steps to the
  deadline itself; a press clears it, and it never surfaces while typing.
- The tip is a floating Clay sibling (RightTop to the box's RightBottom,
  offset [0, 10]: the CSS 11 px measured from the padding box minus the 1 px
  border, right edge flush). Borders take no layout space: fit boxes join
  the border into the padding (the menu's `bordered` rule), fixed boxes
  already hold it. Caps nest their label inside the cap scope, or the text
  lays out as a sibling and the tip runs ~16 px wide.
- The browser's selection swaps with the colour scheme (dark `(83,123,165)`,
  light `(173,208,245)`, probed Chromium; Firefox paints `#3399FF` with
  white text) while the text keeps the focused ink; it paints only while
  focused (blur keeps the caret, drops the paint). No caret bar appears in
  headless screenshots (3/3 absent), so none paints.
- Under the harness's density-2 pages, Chromium under-applies wheel notches
  (two -100 notches read 1.04 there, 1.06 in every other engine/target pair
  including dsf-1 Chromium probes; a page-level listener still reports -100
  everywhere — unrooted, past the page): ref and port agree per engine, so
  web-chromium judges against ref-chromium directly and native-vs-web-chromium
  carries the digit as a recorded input quirk, not a defect.
- A shot never iterates: debounced state needs a wait before the shot, or
  the framebuffer predates the deadline while the URL line already moved on.
- The caret is scalar indices (a browser deletes one UTF-16 unit; SDL only
  delivers whole scalars, and a `String` cannot hold half of one — named,
  tested with an astral character). Clicks while typing collapse to the end;
  click-to-caret placement is the accepted gap.

### h2-input: the plane's input wiring

`plane_input.rs` routes the plane's SDL facts into `camera.rs` and the URL
params into the page URL: `plane_down` adopts outside values BEFORE the
pointer handlers in the same frame (the hook's `adoptIfIdle` tops, moved out
of `Camera`); `plane_move` dirties only on center change (the hook's publish);
`plane_wheel` takes DOM px with `dx` ignored (no `deltaMode` read at this
hash — the skill's ×16 rule belongs to h5's scrub); `frame_step` runs in
`iterate` before the draw (camera step → publish-vs-live → settle flush →
param step/sync → adopt → web hash write); the Hash arm shares
`external_url_synced`; `App.reset_view` (the only copy; h2-plane's
`overlay.rs` twin deleted at the merge) flushes both params to defaults (a
mid-glide reset is rewritten by the ongoing glide, as the reference is).
`HitKind::Plane` captures; a wheel asserts pointer presence; the canvas keeps
the default cursor (no CSS rule). Wheel px: web ×100 restores DOM px, native
macOS notch 40 / precise 10 (scripted states queue page px on both targets).
A fact at script t=0 takes the wall clock (states open with `wait 500`).
views.json: shot lines carry `zoom=scale, pan=center`; native `camera` lines
(center/scale/animating) on change (`check_views` allows them); `run_native`
passes the script's `query` as `--query`. The Plane region is declared here
(`declare_plane_region` over the canvas content box; `collect_regions(app)`
reads `app.clay` in place). The return-to-centre release routes to
`reset_view` when the press started and the release lands on
`ReturnToCenter`; a press on it with the settings menu open closes the menu
(settings.rs ruling). `default_equals` is `Object.is` (`-0` ≠ `0`);
`write_pair` set collapses duplicate keys. Don't run repo-wide
`tools/format` (pinned versions only).

### h3: the sidebar's function picker and the plotted switch

`selector.rs` lays out the Function card (`function_card(app, current)`:
state as a param, the settings segmented-cell precedent): legend
(`Type::Heading`, WS700 17/28), the 36 px bordered grid of four 83 px cells
(`Type::Maths`/`MathsItalic`, STIX 20/25, x italic, literal ²/³ as
`\u{00B2}`/`\u{00B3}` escapes, `sin x` as three runs with the font's own
space advance — no cross-run kerning, so separate runs are exact). Cells 2-4
carry their divider as a nested 1 px `--border` child (the shared Border arm
strokes uniform top-width borders only; a one-sided border paints nothing —
NanoVG drops moveto-less paths and zero-width strokes alike). Checked beats
hover and active on background (probed specificity); active mixes primary at
26% over the base; cursor stays Default (no CSS rule); stop is the checked
cell only (the DOM's one tab stop per group). `sidebar.rs` opens the column
(`CARD_GAP` 12, h4-rows' `--space-inset` measurement; unobservable with one
card). `pick_function` flushes `func` at once (`set_flushed`, default stays
out), switches unless reduced motion shows it whole; arrows wrap and check;
`plot_step` holds the switch's own turn (render-phase curves) and a
mid-switch re-pick's turn keeping the running clock (the hook's loop is keyed
on `switching`: the next frame diffs from the last callback, not the click —
a re-anchor loses the 8 ms the click turn consumed, measured at 2922 flat px
before the fix); `external_func_synced` adopts hash edits the same way.
`App` appends `shown`, `func_param`, `plot_frame_at`, `plot_hold`; the loop
stays awake while switching; `paint_curves` (width 3, round joins, chartLine
at the curve's alpha, move-to first) runs between labels and legend in
`paint_plane` (the TS draw order). The sidebar has the bar's `--shadow-card`
shape and radius 4 (`--radius-card` is defined at this hash, unlike h1's
`--radius-lg`). No new views.json line (the function rides the URL line) and
no new tokens (all eight tabled).

### h4-readout (from its progress file; merges after h3, before h4-rows)

- `state.rs` owns `TransformParam` (A B H K + `as_str`, `colour_token` to the
  `--param-*` tokens); h4-rows' and h6-p-transform's copies delete at their
  merges and use this one. `HighlightState{hover, focus}` holds
  `Touch{param, at}` records with an internal move sequence; `lit()` is the
  present touch with the greater `at` (equals the source's lead/fallback
  rules); `set_hover/set_focus(None)` clears unconditionally (the input layer
  sends it only when its side is on no parameter). (h7-p-state widens all of
  this to ordered lists with a `lead` side — see the h7-p-state bullet above;
  the unconditional drop is now `set_*(&[], true)`.)
- `readout.rs` builds the value and form lines as token runs (`Part`: plain
  text, parameter term, stacked fraction, sup digit; reader-only spans feed
  `spoken()` only). Values print through `mv_numfmt::to_fixed(v, 2)` with the
  ASCII minus swapped for U+2212 (exact binary toFixed; the never-`-0`
  `format_number` diverges on ties). The fit is `min(1, (outer-1)/inner)`
  on whole-px inputs, about the line's left centre.
- Layout is Clay spacers for geometry plus a post-pass paint: one spacer per
  run (frac rows carry the row spans' 4 px side padding; the column 3 px),
  term outers with `ReadoutTerm` regions (text cursor), paint from the same
  model accumulating advances, scaled by the fit through a NanoVG transform
  about the span's left centre. Baselines hang from a centred strut
  (half the frac overflow above the 35/25 px line box, else the top); sup
  rises 0.58em, stacked parens sink 0.0775em; tint and bars are exact rects
  from the run boxes. The readout row is fixed 350 px wide so an unscaled
  span overflows it the way the browser's block row does (a grow row would
  stretch the card).
- A lit term's tint paints opaque and pre-blended (ink at 18% over the
  section backdrop, in the encoded space): the framebuffer blends linear
  while the reference composites gamma, so a translucent fill reads ~30
  levels light (measured 107 vs 74). Backdrop is always the opaque card.
- The Function card paints `--section` (added to the themes at 5714319 by
  the inner-background fix), the grid `--card`; sidebar padding is 12+1
  (the selector split changed it from 20+1); option cells are 87 px on the
  374 px card. `views.json` gains a `lit` line (null or the letter) whenever
  the highlight moves; `sync_hover` runs at the end of `retarget`. Text runs
  note their ink boxes for the I-beam (the reference reports text under the
  pointer); the port's `note_text` had no callers.
- Reference forensics that paid: DOM boxes inside a scaled line need
  unscaling before comparing advances (the 0.98 fit masquerades as a
  per-run shortfall); term boxes extend 3 px above the text runs (inline
  block) and 1 px below (padding), rows with terms are 36/26 px; the form
  frac is always 57.5 px; `div:has(> [data-testid=...])` selects the card
  through CSS-module hashes.

### h5-ruler (the scrub ruler)

- The ruler's card must paint in the post-pass, not through Clay: the rows
  beneath paint their chrome after `draw_commands`, and the next row's box
  border and chevrons show through the tape's translucent wash (measured:
  the next param's ink at the tape's right end). The layout declarations
  carry no paint; the post-pass fills the opaque card, both rims and the
  wash over everything beneath, then the contents.
- The wash goes through `draw::css_background`, as the rows' own tint does:
  an unpremultiplied fill composites a level off on some inks (measured: a
  31k-pixel 1-level blanket on the k/b/h tapes, gone after).
- The tint needs no token: the probe resolves `--row-tint` as the ink's own
  oklch triple at `/ 0.18`, so `theme::band` (alpha × 0.18) is exactly it.
- Judging downscaled full frames by eye misleads twice in one milestone
  (a "swapped modes" scare and a "missing element" scare, both refuted by
  crops and counts): verify programmatically, and mosaic small crops into
  one image instead of reading sequential full frames.
- `tools/run dom --fixture` at a new hash reruns every fixture state on the
  new reference: h4's typing states clicked 18 px above their boxes (the
  card moved) and Backspace navigated back, failing webkit with 0 shots.

### h5-fix-ruler (tape width + mirrored drag)

- Size the tape element at the card's content width (332 at a 350 row), not
  the tape's own client width: the tape fills the card, and `clientWidth`
  (328, what `rulerMarks` takes) is the tape inside its own 2 px rims. A 4 px
  narrower tape moves the centre 2 px and can flip `ceil(lo / step)` at a
  lattice boundary — measured: at the quantized box-drag value 1.11 the 0.8
  label sits 0.08 px inside the 328 tape but falls out of a 324 one, dropping
  the edge sliver with no filter logic differing. The review's suggested fix
  (widen label inclusion past the TS filter) would have diverged from the
  source; the debug print of value + content width + label xs in the native
  run found the real cause. Verify pixel findings against the TS arithmetic
  before changing inclusion rules.
- `open_drag` reads `App.scrub` (mirrored on every hold mutation via `store`,
  so it tracks moves and rebases) instead of rebuilding the drag from the
  hold plus a fresh row-box lookup: one derivation feeds the ruler.
  States must track the reference's layout per hash; the fix is a coord
  shift, verified behaviour-identical on the reference.

### h5-fix-tip (tip above the post-pass rings; flip-focus verdict)

- The scrub tip is the DOM's `z-index: 1` layer: `draw_page` partitions the
  frame's commands and walks everything a laid-out tip's box holds after
  the rows' post-pass rings (which print over it otherwise — measured: k's
  inset ring over a's tip, 304 saturated-gold px in the tip box vs 5 in the
  reference). Clay order holds within each walk; a neighbour's text fully
  inside the tip keeps its place ahead of the tip's plate in both, so it
  paints the same.
- Only a Rectangle command carries its element's id: Clay derives Border
  ids (`HashNumber(elementId, children.length)`) and Text ids
  (`HashNumber(lineIndex, elementId)`) from it (`clay.h`'s
  `Clay__CalculateFinalLayout`). Matching a layer by id set alone moves the
  plate and buries its border and words under it (measured: +3827 px);
  anchor on the id-matched plate Rectangle's box and take everything the
  box holds. The arrow glyph scopes declare no commands; their paths paint
  in `paint_extras`, which moved from the end of `paint_rows` to after the
  tip walk.
- flip-focus settles from the CSS/TS at `b8a0e1d`, not the pixels: `.box`
  digits are unconditionally `var(--row-ink)` (the lit state drives row
  tint, inset width and `data-lit`, never digit colour), and the toggle's
  focus lights b's row through `onFocus` → `focusAtom` → `litAtom`
  (lead-wins). Measured in ref-Chromium and ref-Firefox alike: b's toggle
  focused with its ring, only b lit, all digits full param colour — the
  port matches on every target, so the union's gold-vs-peach claim does not
  reproduce and there is no defect and no new quirk; the only engine split
  stays the known WebKit Tab quirk (focuses b's box, buttons skipped).
### h6-curve (the transform's wiring)

- The flip's render phase needs the adoption without the advance: the
  press's own turn must draw the jump at elapsed 0 (a=2.00), so
  `ShownTransform` splits `adopt` (render-phase, called from the press)
  from `step` (adopt + advance, called from the frame loop). Without the
  split the press turn draws one frame in (a≈1.96), a 4-device-px curve
  defect. The loop's first step after the press carries `FIRST_FRAME_MS`
  through the jump's `first_frame` flag, as the hook's first rAF callback
  carries no measured gap.
- A transformed curve cannot stay a `fn` pointer: `plot::sample_curve`
  takes `impl Fn(f64) -> f64` (plain curves pass their pointer, transformed
  ones `|x| curve.func.value(x)`), and the overshoot clamp
  (`plane_geometry::clamp_curve_y`) sits inside the shared sampler, exactly
  where `drawCurve` clamps before pushing the point. Identity transforms
  sample bit-equal through `TransformedFunction::value` (`1.0 * g + 0.0` is
  exact), so one paint path serves all curves.
- Canvas dashes emulate by cutting the polyline into on-interval subpaths
  (`plane::dash_pieces`, the first port's `dash.rs` technique) stroked with
  the look's caps: Canvas caps each dash, which is what makes round-capped
  `[6, 6]` from offset 3 match. The pattern phase is the offset into the
  period (`(d + offset) % period`), not a trimmed first dash.
- Never pipe a check into a commit chain: `./build.sh check | tail`
  exits 0 through `tail`, so one commit landed while a test was red. Chain
  the check and the commit with `&&` only and no pipe.
- At a new hash, re-read every shared artifact the reference may have
  renamed: the 5cef052 tooltip commit renamed the overlay button's
  aria-label (fixture selector) and hid the button at the default camera
  (fixture shot + box test source). The button's own visibility rule stays
  its owner's (h6-handles).
### h6-handles (the shift/scale handles, the lit-parameter lines, the overlay tooltip)

- Handles and lines paint in `theme.primary`, never the parameter colours:
  the source sets `strokeStyle`/`fillStyle` to primary for every handle and
  line (probed on every op), with the page colour as the handle fill.
- NanoVG inherits the context the last painter left, while Canvas 2D
  `save`/`restores` each curve and line back to butt caps and miter joins:
  the handle and line painters set butt + round (handles) and butt + miter
  (lines) explicitly and restore miter after, so the legend's bar strokes as
  probed whatever the curve painters leave.
- The dashes are `dash_polyline` pieces, not an API (`mv-nvg` has none):
  the first port's `dash.rs` technique with its 0.01 px cutoff, one subpath
  per piece, `lineDashOffset = dash[0] / 2`; solid shafts stroke directly.
  The shared `pub(crate) dash_pieces` in `plane.rs` is h6-curve's for the
  dashed original too.
- The hover redraw lands a task late on the reference (a hover lights the
  row at once but the canvas repaints after): every hover state is its own
  fresh page with `wait 1000` after the move, as the user's
  `transform-lines.mjs` reloads per param. Settled shots are exact.
- The tooltip needs a timer, not a timestamp: the app sleeps between input
  turns, so arming `overlay_tip_since` in the draw never fires — the delay
  is `loop::Timer::OverlayTip` (+400 ms, armed with the hover, cancelled on
  leave, `tip_fired` dirties while still hovered), mirroring the scrub rest
  tip. Forgetting the button gate the first time painted it at the default
  view (a 3335 px max-235 cluster on the button box in every state); the
  gate covers the region, the paint and the tip together.
- The fixture's WebKit and Chromium tip boxes disagree by 0.58 px (their
  shapers differ); the port's stb advance matches Chromium to 0.01. The tip
  geometry test pins the math from a given advance (the legend precedent),
  not the port's measure against WebKit.
- Judge boxes from `judge.json` are CSS px, not device px: a 34x32 "mystery
  cluster" at (1360, 781) is the button box, and element boxes need the
  canvas offset (433, 97) added — costs an hour when missed.
### h7-p-plane (the grid carried on the layout)

- Since `1cfd34a` the grid is laid out once per camera update: `GridLayout`
  stores the `width`, `height` and `scale` it was computed for (appended
  fields, not TS order), and `x_label_lines` / `layout_labels` / the legend
  entry read them from the grid instead of taking them as args.
  `layout_grid` keeps its `(center, scale, width, height, margin)` arg order.
- `labelMargin` moved from a local in `drawCartesianPlane.ts` to an export
  in `constants.ts` in the stretch; in Rust it stays `plane::LABEL_MARGIN`
  (already public) — a deliberate non-move.
- `hitTest`'s handler filter (`handlers?.[action] === undefined` skips the
  handle) is vacuous at this hash — both handles carry all five handlers —
  and is left out; `plot::PlotHandle { kind, at }` stays, handlers dispatch
  in the app.

### fix-theme-switch (union H1-H6 finding: theme-switch-frames frame 0)

- A floating overlay needs its stacking layer, not just its paint order:
  the plane's canvas region (declared after the layout) shadowed the open
  menu's cells/rows at equal z=0, so row clicks light-dismissed the menu
  instead of switching the theme. `app.input.z` is the designed mechanism
  (nothing had ever assigned it); the menu now lays out at layer 1. Hit
  regressions from shadowing show as cursor `default` + theme stuck in
  views.json, and a headless test (layout + declare_plane_region +
  collect_regions, then `hit_at` on the click point) pins the layer.
- The plane paints under the page's Clay content: grid/curves printed over
  the open menu. `draw_plane` runs before `draw_commands`, with only the
  menu shadow moved after the plane (bar/sidebar shadows keep their place
  under it). Nothing else overlaps the canvas, so no other state moves.
- Mid-fade whole-frame diffs are the transition's shape, not its timing:
  math-viz2 leaves the view transition uncustomised, so each engine runs
  its UA default — Chromium's is linear (h1's probe), WebKit's ease-like
  (measured per-frame: the port's linear fade sits ~0.2 progress behind
  mid-fade, endpoints equal). One build can't match both; the port stays
  linear (Chromium-measured, h1's call). The fade's start can lag the click
  up to one 16 ms grid frame (facts run on the grid), worth ≤6 levels at
  the end — inherent, like the last-grid-draw note.
- WebKit doesn't focus buttons on click (macOS convention): after a gear
  click focus stays on body, so Tab lands on the first page stop (the "a"
  field, with selection + hint bar), while the port clears focus to the
  menu's system cell. Chromium agrees with the port. That is the
  `settings-focus-radio` remainder (engine quirk (d) + h6's new stops),
  left for the tab-order owner — the menu paint half of it is fixed here.

### h7-plane: the plane's handles made live

- No new HitKinds and no handle regions: the press and the hover both name
  the Plane hit, and `plane_input` hit-tests inside (`plane_geometry`'s
  `hit_test`/`from_px` over `page.rs::build_handles`, the FunctionViewer
  build order). `App` appends `hovering`, `hovered` (None = the source's
  -1), `dragging`, `grab` and `dragged` (the press's handle index: the drag
  formula's pair never changes mid-drag while the hover index follows the
  moves).
- The cursor is state, not regions: `plane_cursor` (`dragging` or a Plane
  press → Grabbing, else `hovering` → Grab, else Crosshair) runs after the
  dispatch, the retarget, the press and the release, and at the end of
  `draw_page` — the per-frame win that matters, because
  `collect_regions` → `retarget` → `update_cursor` re-resolves the region
  cursor after every layout and would otherwise stomp the hover override
  (found by trace: Grab set at the input turn, Crosshair back before the
  shot). The diamond-to-square single jump lands pan with a and b lit, by
  the source's statement order (new hover first, then the old end).
- `harness.rs` writes a `{"cursor": "pan"|"grab-hover"|"grab-active"}`
  line on change between the plane's three only (other cursors write no
  line, so off-plane states keep byte-identical views); `act.py` allows the
  shape and reads `data-cursor`/`[data-param][data-lit]` for the behaviour
  checks. Shot lines keep the CSS cursor name (`grab`/`crosshair`); the
  single lines carry the reference's words.
- Scripting rule (measured on the reference, all engines): a scripted
  pointermove hit-tests the last render's handles, one step stale, so
  drags step at most 5 px with the press on the handle centre, and every
  decisive hover read is preceded by a settle plus a 1 px nudge. The drag
  past the canvas edge keeps following (pointer capture; offsetX works past
  the width). (Superseded by h7-fix-capture: the port's web glue now
  captures too, so handle-drag-out ends at x=1500 with h=11.44 on native,
  web-webkit, web-chromium and the reference; the past-the-viewport stop
  was a glue gap, not a stack limit.)
  Chromium draws 2 fixed dash-edge pixels at 41/255 where native draws them
  (same spots every same-camera state; everything else ≤15, 0 flat).
- Harness gotcha: never pipe `./tools/run` through head/tail — the SIGPIPE
  kills the venv's pip mid-install and the run proceeds without deps.

### h7-fix-capture: canvas pointer capture on the web; HandleId

- The web past-the-viewport stop was a glue gap, not a stack limit:
  `useCamera.ts:180-182` captures the canvas on every press
  (`setPointerCapture`, implicit release — the hook never calls release)
  and the port's `web/library.js` never did, so SDL's canvas listeners
  lost the moves past the viewport edge. `mv_web_init` now captures on
  canvas pointerdown (try/catch, `Module.canvas ??
  document.getElementById("canvas")`), mirroring the hook exactly.
  handle-drag-out is back at x=1500: h=11.44 on native, web-webkit,
  web-chromium, ref-webkit and ref-chromium.
- Harness/engine limit, measured: Playwright's Firefox dispatches broken
  coordinates for moves past the viewport edge, and the reference follows
  them too (ref-firefox h=-18.56, k=11.5 — the canvas maps the last move
  to window ~(0,-85)); the port lands on the identical values, so judge
  Firefox port-vs-ref directly on input-driven states.
- `plane_input` carries handles as `HandleId { Shift, Scale }`
  (`Option<HandleId>` in `App::hovered`/`dragged`, `from_index`/`index`
  helpers at the hit-test boundary): `handle_params`, `drag_move` and
  `plane_up` match both arms with a `None => {}` no-op (nothing may set
  or flush without a press — the old `_` fallback flushed h/k).

### h7-sidebar (the handles' sidebar side)

- The reset/Reset-all animation is jump-list adopt, not a value write: set
  `jumps` (one, or four in section order a/k/b/h with from=live), adopt at
  elapsed 0 (`flip::adopt_jumps`), then flush — the press's own turn still
  draws the old values while the boxes show the new ones, exactly like the
  flip's `press_flip` + `store_param` order.
- The shared scrub-mode map is the single source: box drags sync it in
  `mirror_scrub` (was/held transition on the box's own mirrored drag only —
  a handle drag's modes never pass through `store`), `drag_mode()` reads the
  map, and the ruler opens one card per mode-held param at the live query
  value (the box drag's card is never doubled). Rows, readout terms and the
  box chrome all follow the same slot.
- The diamond→square single-jump cursor-pan quirk is reference behaviour at
  this hash (`onHover` before the old handle's `onHoverEnd` clears
  `hovering`): keep the dispatch order, approach the square from empty plane
  in states.
- Ruler boxes only exist mid-drag: the dom fixture runs the held-drag state
  (`rows-lit-hk-drag`) for them, and the layout test reads them at that shot
  — the pattern for any drag-only geometry.

### h8-p-marker (the P/Q marker geometry)

- `plane_geometry` holds the markers as pure positions: `PlotMarker { point,
  ink: mv_theme::Token, shape, label, drop_lines, guide }` (the TS `keyof
  ThemeVars` is the `Token` enum; `PointP`/`PointQ` arrive with h8-plane's
  regeneration), `MarkerShape::{Bullseye, Ring}` defaulting to bullseye,
  `PlotLabel`'s coords pre-formatted by the painter; `marker_arcs` returns
  the three concentric arcs with `MarkerInk::{Background, Foreground,
  Marker}` so the bullseye/ring ink swap lives beside `marker_look`;
  `guide_x` is `Some(px)` iff on the plane; `drop_lines` gives the vertical
  then the horizontal segment, each `None` exactly when the TS strokes
  nothing; `label_layout` takes the `Math.ceil`ed run widths and returns the
  pill, baselines, and the arrow box (`None` in view); `arrow_path` the
  tip-first triangle. The label faces are literal `LabelFace` line boxes
  (Figma metrics, never measured); the TS `clamp` import is a private local
  (one-line duplication of `grid.rs`'s, which h7-p-plane owns) because
  `f64::clamp` panics on the negative range a narrow canvas makes.

### h8-plane (the P/Q markers on the plane)

- `plane_markers.rs` paints `&[PlotMarker]` through the geometry: the guide
  (1 px `[4,6]`), the drop lines (1.5 px `[5,4]`, both dash offsets half the
  first dash) cut with h6-curve's `dash_pieces` and a carried offset, because
  Canvas continues dashes across `moveTo` subpaths (probed live, not read).
  The label pill reuses `rounded_rect`; its runs draw at the geometry's
  literal baselines through `Type::MathsItalic`/`Type::FieldText` faces.
  Widths are `ceil(measure)` per run, as the source ceils `measureText`.
- Q's source is `hover_q`/`clear_q` in `plane_input.rs`: quantized x on moves
  that hit no handle with nothing dragged and no plane press, cleared on
  leave and on any panning move. Behaviour reads through a `{"q": ...}`
  views line and `act.py`'s `plane_point` (hash + Q row role/readout).
  Scripted hovers land on target+1, settle, then land on the target: a
  trailing 1 px nudge leaves Q 0.02 off (quantized, as the first probe
  showed).
- `--point-p`/`--point-q` resolve per theme on the sidebar (probed equal to
  the source's ordinals 6/6); `tools/run themes` merges the base block's
  defaults under each theme block's override (`read_sidebar_points`), and
  `tools/run tokens` + the mv-theme `ALL_TOKENS`/tally carry the two tokens.
- The wordmark is a `HitKind::Wordmark` region (Pointer, no focus stop) with
  a no-op click arm; the root page stays a `decisions.md` line. The shift
  diamond draws at the live h, k while the square rides the shown unit point
  (h7-sidebar fix-forward); paint and hit test share `build_handles`.
- Judging a milestone whose card is still unported: the siblings' Points
  card is one giant cluster per state (232k px at (29.5,650) 373×169);
  everything else in the canvas is named at 6x. Reference-side engine
  quirks seen: chromium/firefox draw one axis tick 1 device px off webkit
  (13–17 px slivers, port == webkit); firefox rounds sage's canvas-top
  background a level (port == webkit).

### h8-fix-values (WebKit dash-length drift; no app-code change)

- WebKit's Canvas drifts dash phase on long paths: on the ~9900 px dashed
  original (2 px `[6,6]` from 3), WebKit renders every dash ~+2.5 CSS px
  down-path of the spec position at along-path ~4550 (tangent-ray profiles
  at dash [4557,4563]; 90 canvas flat>2 px in 58 1–4 px micro-clusters, max
  117, all on the steep runs), while the port, Blink and Gecko agree to
  ≤1 px (native-vs-ref-chromium blob ends identical 10/10; rc-vs-rf 0
  flat). Same JS in all engines, yet ref-webkit-vs-ref-chromium is 347
  flat px apart (max 221): the engines differ on identical content, so the
  residual is category (e), recorded here with the measurement. Short-path
  dashes (drop lines `[5,4]`) match everywhere, so offsets are honored and
  the drift is length-dependent (~6e-4 relative), not an ignored offset;
  f32-vs-f64 accumulation was measured at 0.006 px and ruled out. Matching
  WebKit would break the spec/Blink/Gecko match, so the port keeps the
  spec math. h6-curve's 27 px dash-phase cluster at CSS (788.5,102.5) is
  the top of this same strip.
- SwiftShader AA at curve-stroke edges peaks at 41 on isolated pixels
  (marker-p-values: exactly 2 px >15 whole-frame — (1599,331) at 41,
  (1730,817) at 40, both single AA pixels on drifted dash ends, 0 flat),
  the h7-plane-recorded (d) family (their 2 px at 41; h6 accepted to 33).

### h8-points-row (the Points card's rows)

- The readout's x slot is the closed field's input, not a span: the input
  is only the value's chars wide (`--chars: max(1, text.length)`,
  unrounded) and the lead right-aligns the whole [open paren + input]
  group (`justify-content: flex-end`), so `f(` sits the slot's leftover
  right of the lead's left edge (11.8 px at the default slots — measured,
  not eyeballed). The stand-in paints that closed look until
  h8-points-field swaps its field in.
- The role slot wraps badge and role to a clipped second line, each whole
  or not at all: hidden runs declare no scope and take no gap, or an
  unbreakable fixed run stretches the head past the card (sci showed a
  392 px card). Shown widths come from one `slot_shown` predicate shared
  by layout and paint.
- The badge's pill is primary pre-blended opaque over the card (the
  framebuffer blends linear while the reference composites gamma: a
  translucent fill reads ~30 levels light, measured). The linger needs a
  `Timer::MovedLinger` keep-awake: shots photograph the buffer without
  drawing, so without it the app sleeps after the release's URL flush and
  the badge never clears. Feed `MovedBy` the frame clock (`clock_now`),
  never `input.time`, which is 0 outside input handling.
- The first frame always lays out stale (no plane box yet): refresh the
  cached range in the paint and dirty on change, or query states
  photograph `p = 2` forever. P's `p` rides the shared param plumbing
  (`init_params_from_url`, `params_step`, `external_params_synced`,
  `params_pending`).
- Playwright's `add_init_script` takes a program, not a function value —
  an arrow-function wrapper silently never runs (two probe rounds kept
  Arctic Ice). Seed each theme in its own slot; the three-key model reads
  `dark-theme` in dark mode.

### h8-points-slider (the x-of-P slider)

- The track is a grow-wide 20 px Clay scope with the region and tab stop;
  thumb and tip are post-pass paint from the laid-out track (no Clay scopes,
  like the overlay button): thumb centre = track.x + px-rounded left,
  tip = measured width centred on the thumb and clamped inside. The layout
  test asserts the arithmetic boxes against the fixture (track, thumb, full
  tip box with real fonts); the dom fixture runs slider-default (track,
  thumb) and slider-hover (tip, `display:none` at rest).
- Focus entries are click-h's-box + one Tab (the flip-focus pattern):
  absolute Tab×5 differs per engine (WebKit skips radios/buttons without
  Full Keyboard Access; chromium/firefox do not), while h-box → slider is
  next-stop in every engine (the h toggle precedes the h box in DOM order).
  A click on the track would jump P, so scripts never click it for focus.
- A left-button press focuses the track without the ring
  (`set_focus(hit, false)`: DOM focus, no `:focus-visible`), then starts the
  drag only with a live range and laid-out track. Moves route by a
  `slider_hold` capture predicate beside the scrub's (the shell's
  `captures()` stays plane-only); release/up/blur flush `p` and clear
  `p_dragging`. SDL has no buttons field, cancel or capture-loss events, so
  the buttons==0/cancel/lost-capture arms are accepted gaps. Enter sets
  `App.slider_edit` for h8-points-field (which merges later); the readout
  field merges its own focus stop past the slider.
- P rides the shared param plumbing (`params_step` steps and syncs it;
  the loop's separate `p_step` was deleted as a duplicate at the row
  merge). The `p` views line (`fmt_g17`) is validated in `act.py` beside
  the `q` line.
- The hover thumb scales 1.12 (radii × 1.12 about the centre — the fixture
  thumb boxes 20.16 px on hover); the parked thumb is a dashed ring
  (circle sampled to 72 segments, cut with the plane's `dash_pieces` at
  `[3,2]` offset 0) with no core. Halo is `theme::plate(primary)` (26%),
  the focus ring `theme::ring` (32%), both plain-alpha strokes like the
  rows' rings. Tip text baselines hang from a centred content strut
  (readout shape): `x` Stix400Italic 14/18, value RobotoMono500 13 tabular
  on a 17 px line offset 0.5 px.
- Fractions floor in WebKit mouse input (press at 249.54 reads clientX 249):
  scripts use whole px and the unit tests pin the integer-coord values
  (press 250 → 2.03, +30 → 3.82, click 100 → −6.92).
- `mv_theme`'s `ring`/`plate`/`band`/`badge` return 0..1 theme colors;
  `draw::nvg_color` takes 0..255 Clay colors (divides by 255). Wrap with
  `ui::ink` first, as every other `nvg_color` call site does — passing a
  helper's output straight in paints at alpha ~0.001 (an invisible halo and
  focus ring, found by judging slider-focus at 6x and proved by a radial
  profile against the reference).

### h8-points-field (the P x field and the shared tip's P edit tip)

- `x_field.rs` owns the edit (`XField`: `FieldState` text+caret, `base`,
  `invalid`) with the h4 `number_field` caret model; `tip.rs` owns the
  shared look (`TipTone`, `TipCell`, `TipIds`/`TipPlace`, `layout_tip`,
  `paint_arrows`). `scrub_value.rs`'s tip paint moved onto `tip.rs`
  (content stays there) with byte-identical PNGs pre/post on
  box-tip-{rest,typing}, box-click-open (native+webkit+firefox at
  threshold 0).
- The field's layout: a floating `X_SLOT_ID` box (value-width
  `chars*ch + 2*pad`, right-aligned in the x run) + hit region, declared
  from the P readout; the tip floats under `P_HEAD_ID` (drop 4, z 3).
  The readout width/paint share one `x_slot` formula; the lead
  right-aligns the group. Layout tests pin field_p + tip_p in
  rows-default/field-open/field-invalid within 0.5 CSS px.
- Paint post-pass: card bg + 2 px inset ring (primary/destructive),
  scheme selection plate over fully selected focused text (field minus
  pad, less 2 px top/bottom), right-aligned text. Key bug class:
  `mv_text::draw` takes the style's ink as NanoVG 0..1 — passing the
  readout's already-converted ink through `nvg_color` again paints
  near-black (invisible text); pass it straight through.
- Input: key chain settings > selector > x_field > transform_row, then
  the slider only if nobody consumed (first-consumer-wins, or a commit
  that moves focus to the slider falls through to the slider's Enter and
  reopens the field). A consumed key always dirties (a refused Enter
  returns no effects; without dirty the loop sleeps and the shot
  photographs the pre-Enter frame). Arrows collapse the caret to the end
  (React's value write resets the selection). Slider Enter sets
  `App.slider_edit`, consumed in `slot()` the same frame.
- Test-helper rule: never move the `App` after `init_clay` — the measure
  closure keeps its address, so a `laid_out_page` returning the app by
  value dangles it and the next measure aborts ("shape needs init").
  Take `&mut` and build the app in the test.
- Engine notes: WebKit dark selection (85,125,167) vs Chromium
  (83,123,165) the port paints (2 levels, accepted); saturated canvas
  inks shift vs CSS (primary ring (112,192,160) vs (122,190,216)) — the
  shared P3/gamut story (`ui::ink` everywhere; neutrals byte-identical),
  for the union `gamut all`, not a defect.

### h9 (Arctic Ice Secondary recolor)

- One CSS line (`--secondary` `oklch(0.2885 0.0208 248.55)` →
  `oklch(0.34 0.05 242)`, Arctic Ice only): no Rust change; `tools/run
  themes` + `tools/run tokens` + `tools/run dom --fixture` against ref-h9
  touch exactly three files (themes.rs header hash + Secondary l/c/h;
  tokens.json arctic-ice `--secondary` computed/oklch/rgb/engines;
  page.json `"reference"` header). Consumers all resolve
  `Token::Secondary` at run time (gear plate, hovered settings
  row/swatch, hovered selector row, hovered value box, flip/reset
  hovers, Reset-all hover, overlay button hover, tip key caps).
- Measured draws, all engines agree: old plate rgb (35,44,53), new
  (31,59,80) (tokens.json engines rows chromium+firefox identical;
  sampled off the PNGs). Fail-pass on `default` vs ref-h9 WebKit: before
  2847 flat px apart in the gear box (4636 changed), after 0 flat
  (1107 changed, stroke/AA edges only). Base-vs-new native diff on
  `default`: 4452 px changed, all inside the gear box, max 27.
- Census vs union-h8 (the level's evidence): all 87 H1/H3/H4/H5/H6
  states identical in chromium+firefox; webkit identical except
  flip-focus +3546 and settings-focus-radio +3487 inside their
  pre-existing focus-divergence clusters (same box/max) and
  theme-switch-frames mids +0.5% (fade-timing family). Triangulation
  (new port vs ref-h8 on 5198 + direct ref-h8-vs-ref-h9 diff) proves the
  growth is ref-side: the reference paints its focused-box tip caps
  Secondary (8140 px at CSS (54,485.5,310.5,503)), recolored h8→h9.
- Finding (open, sibling-owned, not fixed here): in those two states the
  port's focused-box key-help bar paints 0 px Secondary (section/muted
  family in both builds; the typing-variant tip follows the token —
  rows-typed paints 9164 px Secondary in the same box), so its cap
  plates don't resolve `Token::Secondary` although `tip.rs key_cap`
  sets it. Pre-existing at union-h8 (same shape), exposed by the
  recolor. Owner: h8-points-field's tip/input code (wrap/field settles
  the mechanism — suspect a second bar path beside `layout_tip`, since
  `scrub_value::tip` early-returns unless typing/resting).

### h10-p-loop (the welcome loop and the fit-to-window scale)

- `welcome_loop.rs` holds the 22-step tour (`Step::{Hold, Drag, Switch}`,
  `STEPS` in source order, 451 frames) with `LOOP_MS` computed as the
  source computes it (`total_frames * 1000 / FPS` via `const fn`, so a
  step edit moves the constant). `LoopState` is flat (`func, a, b, h, k`),
  not `{ func } & Transform`; `Easing` is a four-value struct evaluated
  by `ease_drag` because a closure is neither const-buildable nor `Copy`
  (clippy's `double_must_use` also bars `#[must_use]` on
  `cubic_bezier`, whose `impl Fn` return is already must-use).
  `dragged` delegates to `scrub::scrubbed` (no re-rounding; the −1.19 /
  2.44 / 1.74 / 1.15 literals fall out of its `quantize`).
- JS `%` on a non-negative float is Rust's `%` (both keep the dividend's
  sign); `loop_at` keeps `%` as the source has it and documents the
  negative-`ms` case rather than switching to `rem_euclid`.
- `WelcomeClock` is the rAF accumulator as a step function in the same
  file (first frame adds `plot::FIRST_FRAME_MS` regardless of the gap,
  later frames `min(now - last, 50)`); `state(false)` is the still and
  `is_animating(false)` is false, so the coder's frame loop schedules no
  frames under reduced motion.
- `fit_scale.rs`: `fit_scale` returns `1.0` on a non-positive side — the
  hook instead keeps its previous value, so the caller retains its stored
  fit and only calls this once laid out (said in the doc). `paper_fit`
  is the `WelcomePage.tsx` line verbatim, `1e-9` included. Verified
  against the formula: the brief's 768 × 1024 → 0.53 is the dpr-2 value
  (53/100); at dpr 1 it is 0.52.

## Long runs

t0a owns this paragraph. This repo is built by many parallel milestones under the `long-run` skill: the brief (`math-viz2-rust.md`), the plan (`math-viz2-rust.plan.md`) and the progress index (`math-viz2-rust.progress.md`) live in `.local/briefs/` (gitignored scratch); each milestone's record is `.local/briefs/progress/<slug>.md`. Commit on the branch at each verified step (`./build.sh check && git add <paths> && git commit`, paths staged by name, joined with `&&` only). Edit only the files the plan's §5 assigns yours; a change needed in another milestone's file goes in the return, not the file. When a dependency merges into `claude-port`, merge it into the branch at a clean point (`git merge claude-port`, never a rebase).
