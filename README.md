# math-viz-rust

The [MathViz](https://github.com/niels4/math-viz) Function Viewer, ported to Rust by AI agents. One codebase builds a native app for macOS, Windows and Linux (SDL3, OpenGL) and a WebAssembly app for the browser. A high-fidelity port of the original design and implementation.

### [Open the live app →](https://niels4.github.io/math-viz-rust/)

<a href="https://niels4.github.io/math-viz-rust/#function-viewer">
  <img alt="The Function Viewer: the function selector, the formula, the Transform card, the Points card, and the plane with a stretched parabola, its handles and the P marker" src=".github/readme/function-viewer-light.png">
</a>

<a href="https://niels4.github.io/math-viz-rust/">
  <img alt="The welcome page: the MathViz mark, the title, the Open the Function Viewer button, and the animated graph on the right" src=".github/readme/welcome-dark.png">
</a>

**[Function Viewer](https://niels4.github.io/math-viz-rust/#function-viewer):** pick a function, then drag its handles or scrub a, b, h and k to stretch and shift the curve. Pan and zoom the plane. Slide P along the curve and read its value.

## Why Clay, SDL and Rust

I had been playing around with [Odin](https://odin-lang.org/), building native apps with [Clay](https://github.com/nicbarker/clay) for layout and [SDL3](https://libsdl.org/) for the window, input and GPU, and liked how little stood between the code and the screen. Rust seems to be the popular language for automated app building, so I combined the two: the Odin-style native stack, written in Rust by AI agents.

| Layer | Library |
|---|---|
| Window, input, GPU context, main loop | [SDL3](https://libsdl.org/) |
| Layout | [Clay](https://github.com/nicbarker/clay) |
| Vector drawing | [NanoVG](https://github.com/memononen/nanovg) |
| Text shaping | [kb_text_shape](https://github.com/JimmyLefevre/kb) |
| Web build | [Emscripten](https://emscripten.org/) |

The C libraries are vendored and wrapped in thin Rust bindings; everything above them is plain Rust.

## How it was built

The original is a React app that grew commit by commit. Rather than hand the agents a spec, the orchestrator read the original's git history, cut it into milestones, and built the port in the same order, so the Rust app passed through the same states the original did. When the original gained commits during the run, they became the next milestones.

| Role | Did |
|---|---|
| **Driver** (Claude Opus) | Ran the whole thing: planned, cut branches, spawned every agent, merged in order, asked the few questions that needed a human |
| **Planner** (a larger Claude model) | Turned the original's history into milestones, briefs and agent definitions |
| **Porters** (a cheaper model, many at once) | Ported the pure modules, test for test and value for value |
| **Coders** (a cheaper model, several at once) | Wired each milestone into the running app, judged by pixels and behaviour against the original |
| **Reviewers** (three per milestone) | A pixel recount, a code and idiom read, and a trial merge against the sibling branches |
| **Operators** | Worktrees, reference servers, merges, whole-app check runs |

What kept it fast and honest:

- **Parallel by default.** Every milestone on its own branch and worktree, against its own pinned build of the original. The next milestone started before the previous one merged; only the wiring waited. Merges landed in history order.
- **The original as the oracle.** One script drives the original, the web build and the native build to the same state in three browser engines and compares the screenshots pixel for pixel. Native and web had to match exactly; differences from the original were measured region by region, with browser quirks recorded once.
- **Tests as the anchor.** The original's unit tests were ported one for one, and a drift scan compared every ported function with its source. The Rust could change shape, never behaviour.
- **Reviews beside merges.** No merge waited for a review; findings became fix-forward work.
- **A human, lightly.** I answered a handful of questions, reported two things I saw, and merged to main when I wanted to deploy.

The Function Viewer reached parity with the original in under nine hours; the welcome screen followed the same morning, and the welcome page's link into the viewer and the point dragged on the graph came after that. 340 commits, 644 unit tests and 237 scripted states across three browser engines and the native build. The first attempt, a port of an Odin version of the app, had taken 28 hours the day before; this run reused its libraries, harness and lessons.

The native app was verified on macOS and the web app in WebKit, Chromium and Firefox; the Windows and Linux builds come from the same sources and are untested so far. Deliberate differences from the original are listed in `decisions.md`.

## Where the port stands

At math-viz2's `c6754e7`: the whole Function Viewer and the welcome screen beside it, ported commit by commit through the original's own history (h1 to h11) and judged state by state against math-viz2 at each milestone's commit, in three browser engines and the native build. [BUILDING.md](BUILDING.md) has the detail, the checks you can run yourself and the known gaps.

## Running it

Needs Rust 1.99 with the `wasm32-unknown-emscripten` target, Emscripten 6.0.12 and SDL3 (`brew install sdl3` on macOS). [BUILDING.md](BUILDING.md) has the setup, every flag, and the harness.

The native app opens on the welcome page, as the browser's `/` does; `--route function-viewer` opens the viewer straight away. The web build has the two routes at `/` and `/#function-viewer`, and the viewer opens on x².

```sh
./build.sh native        # build/native/function-viewer
./build.sh web           # build/web/, serve it as static files
./build.sh check         # fmt, clippy, tests and both builds
./build.sh pages         # docs/: the web app for GitHub Pages
```

## The harness

`tools/` is a small Python project that puts the original, the web build and the native build in the same state from one script and compares them at threshold 0, in WebKit, Chromium and Firefox. It is what judged every milestone.

```sh
tools/setup                                      # the venv
tools/run states H1 --targets ref,web,native     # one state group on every target
tools/run states all --jobs 8                    # the whole catalogue
tools/run judge .local/shots/H1                  # pixels apart from the original, by region
tools/run sweep                                  # motion and input at off-grid timings
```

MIT licensed.
