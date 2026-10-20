# math-viz-rust

The [MathViz](https://github.com/niels4/math-viz) Function Viewer ported to Rust: one codebase that runs as a native desktop app on macOS, Windows and Linux (SDL3, OpenGL 3.3) and in the browser as WebAssembly (WebGL2). A high-fidelity port of the original design and implementation, built almost entirely by AI agents.

### [Open the live app →](https://niels4.github.io/math-viz-rust/)

<a href="https://niels4.github.io/math-viz-rust/#function-viewer">
  <img alt="The Function Viewer: the function selector, the formula, the Transform card with a, k, b and h, the Points card, and the plane with a stretched parabola, its handles and the P marker" src=".github/readme/function-viewer-light.png">
</a>

**[Function Viewer](https://niels4.github.io/math-viz-rust/#function-viewer):** pick a function, then drag its handles or scrub a, b, h and k to stretch and shift the curve. Pan and zoom the plane. Move P along the curve and read its value. The same `#function-viewer?…` URL state as the original.

## Why Clay and SDL, and why Rust

I had been playing around with [Odin](https://odin-lang.org/), building native apps with [Clay](https://github.com/nicbarker/clay) for layout and [SDL3](https://libsdl.org/) for the window, input and GPU context, and liked how little stood between the code and the screen. Rust seems to be the popular language for automated app building, so I combined the two ideas: the Odin-style native stack, written and wired by AI agents in Rust. The result is this app, a port that keeps the original's design and behaviour rather than a reinterpretation of it.

The stack is small and all of it ships in the repository: SDL3 for the platform, Clay for layout, [NanoVG](https://github.com/memononen/nanovg) for vector drawing, [kb_text_shape](https://github.com/JimmyLefevre/kb) for text shaping, each a vendored C library behind a thin Rust binding (`crates/mv-sys`, `mv-clay`, `mv-nvg`), and Emscripten for the web build. The app itself is plain Rust on top: the state, the layout, the painters, the input.

## How it was built

The original is a React and TypeScript app, developed commit by commit with its own design process. The port followed that history instead of a spec: an orchestrating agent read the original repository's git log, cut it into milestones (the page shell and themes, the plane and camera, the sidebar and plotted function, the readout and transform rows, the scrub boxes and ruler, the transforms and handles, the points, the welcome screen), and built them in the same order, so the Rust app passed through the same states the original did. When the original gained new commits during the run, they became the next milestones.

The agents, all Claude Code sessions and subagents:

| Role | Model | Did |
|---|---|---|
| Driver | Claude Opus | Ran the whole thing: opened the run, called the planner, cut branches, spawned every agent, merged in order, kept the run record, asked the two or three questions that needed a human |
| Planner | a larger Claude model | Read the original's history and the previous Rust port, wrote the plan, the milestone briefs and the agent definitions, and re-planned as the original moved |
| Porters | a cheaper model, many at once | Each ported one pure module and its tests, test for test and value for value, with a drift scan comparing the Rust to the TypeScript function by function |
| Coders | a cheaper model, several at once | Each wired one milestone into the running app, judged by pixels and behaviour against the original |
| Reviewers | a cheaper model, three per milestone | Three lenses on every merged milestone: a pixel recount at threshold 0, a code and idiom read, and a trial merge against the sibling branches |
| Operators and scanners | a cheaper model | Worktrees, reference servers, merges, union runs; long logs condensed |

What made it fast, and what kept it honest:

- **Everything in parallel that could be.** Every milestone ran on its own branch in its own git worktree, against its own pinned build of the original (a detached worktree of the TypeScript app at that milestone's commit, served on its own port). A milestone's porters, and the reading and layout work of its coders, started before the previous milestone had merged; only the wiring waited. Merges went back in history order, so the integration branch always matched a state of the original.
- **The original as the oracle.** A harness puts the original, the WebAssembly build and the native build in the same state from one script, in WebKit, Chromium and Firefox, and compares the screenshots pixel for pixel. The port against itself (native against web, engine by engine) had to be identical; the port against the original was measured region by region, with browser engine quirks recorded once and never counted twice. Behaviour was checked the same way: after every scripted input, the page's URL state had to equal the original's.
- **Tests as the anchor of intent.** The original's unit tests were ported one for one, with their names and expected values, and a drift scan compared every ported function with its source, literal by literal. The Rust was allowed to change shape (enums for tagged unions, iterators for index loops, messages for callbacks) but never behaviour.
- **Reviews beside the merge, not before it.** A merge never waited for a review; findings became fix-forward work on the next branch. Each review recounted the milestone's own evidence rather than rerunning it.
- **A union after every level, a sweep at the end.** After each milestone level, the whole catalogue of states ran on the integration branch to catch anything a merge broke; motion and input ran a sweep at timings off the 16 ms grid to catch what on-grid checks miss.
- **A human in the loop, lightly.** I answered a handful of questions (what to do when the original moved, which cursor to show, where the port should stop), reported two things I saw, and merged to main when I wanted to deploy. The run took care of the rest.

The numbers, for this port: the Function Viewer reached feature parity with the original in under nine hours of wall-clock time, the welcome screen landed the same morning, and the result is about 230 commits, 600 unit tests and 190 scripted states over three browser engines and the native build. The first attempt at the stack, a port of an Odin version of the same app, had taken 28 hours a day earlier; this run reused its libraries, harness and lessons.

Where the port deliberately differs from the original is listed in `decisions.md`. The native build was built and verified on macOS and the web build in all three browser engines; the Windows and Linux builds come from the same sources and are untested as of this writing.

## Toolchain

| Tool | Version | Used for |
|---|---|---|
| Rust | `1.99.0` (`rust-toolchain.toml`; `wasm32-unknown-emscripten` target) | the app; `cargo` builds both targets |
| SDL3 | 3.4.18 (Homebrew `sdl3` natively for dev/check builds; compiled from source for the release and always on the web) | window, input, GL context, main callbacks |
| Emscripten | 6.0.12 through emsdk in `~/emsdk` | linking the web build |

Clay (`e6cc369`), NanoVG, Fontstash, stb_truetype and kb_text_shape (2.21) are vendored in `vendor/`, which says where each came from and what changed; emsdk is the one toolchain fetched outside it.

## Setup

```sh
rustup target add wasm32-unknown-emscripten   # home directory, no admin needed

git clone https://github.com/emscripten-core/emsdk.git ~/emsdk
~/emsdk/emsdk install 6.0.12
~/emsdk/emsdk activate 6.0.12
```

`build.sh` sources `~/emsdk/emsdk_env.sh` itself (set `EMSDK` for another place; it runs under `bash`) and stops if `emcc` isn't 6.0.12. Native dev builds link Homebrew's SDL3 (`brew install sdl3`) with no other setup.

## Build and run

```sh
./build.sh native        # build/native/function-viewer
./build.sh web           # build/web/: index.html, function-viewer.js, function_viewer.wasm
./build.sh               # both
./build.sh test          # cargo test for the workspace
./build.sh check         # fmt, clippy for both targets, tests, both builds; exits non-zero on any failure
./build.sh release       # build/release/: native/function-viewer and web/, optimised, no paths of the build machine
./build.sh pages         # docs/: the web app for GitHub Pages (release build, relative URLs, .nojekyll)
```

- **Native window:** `build/native/function-viewer`. The page at 1440×900 logical pixels (density 2), drawn in Display P3 on macOS; `--space srgb` draws sRGB instead. It draws only when something changed or moves.
- **Native frame to PNG:** `build/native/function-viewer --png out.png --size 1440x900 --density 2` draws the page into an offscreen framebuffer, writes it and quits.
- **Theme, mode, space and route:** `--theme clean-teal`, `--mode light`, `--space srgb`, `--route function-viewer` override the stored preferences for one run; `--query 'func=x2&a=2'` opens a viewer state, the same query the web app carries in its hash.
- **Browser:** serve `build/web/` as static files, for example `python3 -m http.server 8000 --directory build/web`, and open `http://localhost:8000/`. A browser won't load wasm from `file://`.
- **GitHub Pages:** `./build.sh pages` writes the release web build into `docs/` at the repository root; the repository hosts it from there (Settings › Pages › `main`, `/docs`). Every URL in the bundle is relative, so it serves from the repository subpath, at `/` and at `/#function-viewer`.

## The harness

`tools/` is a small Python project (`tools/setup` makes its venv) that drives the original, the web build and the native build from one state script and compares them.

```sh
export REACT_REF=/path/to/math-viz REACT_REF_PORT=5190   # the original, served by Vite
tools/run states H1 --targets ref,web,native             # one state group on every target
tools/run states all --jobs 8                            # the whole catalogue, 8 at once
tools/run judge .local/shots/H1                          # pixels apart from the original, per engine and region
tools/run sweep                                          # motion and input at off-grid timings
tools/run gamut                                          # Display P3 against sRGB
tools/run themes                                         # regenerate the theme tokens from the original's CSS
```

A state script (`tools/states/*.act`) says how the page opens (size, density, theme, mode, reduced motion, colour space, query, route) and then what happens to it (pointer and keyboard input, waits on a paused clock where animation frames fall every 16 ms, hash edits). Each run writes its screenshots and a `report.json`, with the page's view state, URL and route after every step, and `tools/run states` lays the surfaces side by side on a contact sheet and compares every pair at threshold 0. The port keeps its own preferences and browser storage keys (`math-viz2-rust-*`) and never shares runtime state with the original.

MIT licensed.
