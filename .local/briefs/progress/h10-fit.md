RESUME (F1 done): head `aa63cb2` on branch `claude-port-h10-fit`. HERO_FBO decision: direct vector path kept. Measurement at fit 0.71 (1024x640), 6x crop of hero centre (140x140 device px, 19600 px): FBO 3759 differing vs ref, direct 3202 differing; within 8 levels: FBO 3423, direct 1700. Full-frame differing: FBO 980864, direct 979422. Direct matches closer; HERO_FBO=false committed. FBO path remains gated behind HERO_FBO (dead-ish but harmless, left gated per spec). cargo test 565 passed; ./build.sh check green.

Milestone h10-fit of the math-viz2-rust run, part of h10 at math-viz2 `68e1c7b`.
Spec: `739e1e4` ("resize hero welcome page to fit screen") plus the fit lines of
`WelcomePage.tsx`, `WelcomePage.module.css` (`.page`, `.frame`, `.top_bar`,
`.hero_plane`), `useFitScale.ts`, the three vitest fit cases of
`WelcomePage.test.tsx`, and `useUpdateCanvas.ts` (bitmap at grid × dpr).
Worktree `/opt/dev/agent/worktrees/math-viz2-rust/claude-port-h10-fit`,
branch `claude-port-h10-fit`, base `8adaf58`. Reference `$REACT_REF`
(`/opt/dev/agent/worktrees/math-viz2/rust-port-ref-h10`, pinned at `68e1c7b`)
on port 5200. Merge order at h10: h10-p-loop, h10-t-route, h10-seam, h10-page,
h10-hero, h10-loop, then this milestone (last).

- [x] P1 fit.rs pure helpers with unit tests pinning the vitest literals
- [x] P1 reference probes on ref-h10 (fit values, boxes, bitmap, 6x crops)
- [ ] P2 merge claude-port (at h10-loop's merge), wire root transform + zoom + bitmap path
- [ ] P2 H10 states run and judged, boxes within 0.5 CSS px, bitmap path decided by measurement
- [ ] P2 sweep H10 at 0 crashes, evidence copied, CLAUDE.md block

## P1: architecture

`fit.rs` is pure geometry over a `fit: f64` it takes as input, never computes:
the fit itself (`fit_scale` + `paper_fit`) belongs to h10-p-loop's `fit_scale.rs`
(plan §3.7 owners), which has not landed — every sibling branch is still at the
base `8adaf58` (measured 2026-10-20). Nothing here names `DESIGN_*`: those
constants live with the porter. What this module owns, from the CSS at `68e1c7b`:

- `frame_size(window, fit)`: the frame is laid out at window / fit in design
  units (`.frame { width: calc(100% / var(--fit)); height: ... }`) and drawn
  under a NanoVG scale of fit from the origin (`transform: scale(var(--fit))`,
  `transform-origin: 0 0`), so the content safe-centres inside the enlarged
  frame and shrinks as one picture.
- `screen_to_design(point, fit)`: hit testing through the inverse — a pointer
  at screen (x, y) hits the design point (x / fit, y / fit).
- `zoomed(px, fit)`: the top row scales by `zoom: var(--fit)` rather than the
  frame's transform — its offsets 24 and 32, its 40 px height and the 28 px
  mark all scale by fit. The menu sits at natural size
  (`[popover] { zoom: calc(1 / var(--fit)) }`), anchored to the zoomed gear's
  box: right edge at window.width − 32·fit, bottom at 64·fit, the 304-wide menu
  8 px below, inside the window.
- `menu_anchor(window_width, fit)`: that anchor in screen px.

The hero plane's bitmap (spec §7 item 2): the reference keeps the canvas bitmap
at the device ratio of the design size (838·dpr × 518·dpr, `useUpdateCanvas`:
`canvas.width = round(grid.width · dpr)`) and lets the compositor shrink it by
fit. The port's plane is NanoVG paint; candidates (a) render at full bitmap
size into an offscreen framebuffer (`mv-nvg`'s `Framebuffer`:
`nvgluCreateFramebuffer`/`nvgluBindFramebuffer`, already in the wrapper — no
wrapper addition needed) and draw scaled with linear filtering, vs (b) direct
drawing under the scale. Decided in P2 by measurement at fit 0.71 and 0.88 on
6× crops; the planner expects (a). P1 records the reference side only.

The three vitest fit cases become layout tests in P2 (they need h10-page's
`welcome.rs`, not yet written): 1024×640 dpr 1 → fit 0.70, title 67.2 tall,
mark/gear/title/button/hint/plane inside the window, plane 588×364, no scroll;
1280×1000 → plane 739.2×457.6 centred at y 500, gear top 21.12, right inset
28.16; 1280×800 → (plane.width/840)·50·dpr within 0.005 of whole. P1 pins the
same literals as pure-geometry unit tests with fit passed in (see the table).

No new boxes (h10-page's welcome boxes cover the region); the fit states join
`dom.py`'s `FIXTURE_STATES` in P2 (appends only). No new tokens. States (all
route welcome, density 2 both sides, dark Arctic Ice unless said):
welcome-fit-1024x640, welcome-fit-1280x1000, welcome-fit-1280x800,
welcome-fit-768x1024, welcome-fit-1920x1200, welcome-fit-light-1024x640,
welcome-fit-menu-1024x640, welcome-fit-loop-1024x640, welcome-resize — all
created in P2 after h10-t-route's `route` verb and H10 group land (neither
exists yet: `script.py` has no `route`, `milestones.toml` has no H10 line,
`act.py` hardcodes `REFERENCE_ROUTE = "#function-viewer"`).

TS-to-Rust table (spec items → Rust):

| TypeScript/CSS at 68e1c7b | Rust | Note |
|---|---|---|
| `useFitScale` (`min(1, clientW/w, clientH/h)`, ResizeObserver) | h10-p-loop's `fit_scale` (porter) + frame re-layout on SDL resize / `Resize` verb (P2 wiring) | not this module; re-measured at every resize as the observer does |
| `fit = fitted<1 ? floor(fitted·50·dpr+1e-9)/(50·dpr) : 1` | h10-p-loop's `paper_fit` (porter) | the whole-device-pixel paper rounding |
| `.frame` size + `transform: scale(fit)`, origin 0 0 | `frame_size` + `vg.scale(fit, fit)` at the root (P2) | NanoVG `scale` exists in the wrapper |
| inverse hit test (no TS: the page has no canvas hit-testing under fit) | `screen_to_design` (P2: applied at the input boundary) | keeps every region's design coords |
| `.top_bar { zoom: var(--fit) }`, `[popover] { zoom: 1/fit }` | `zoomed` + menu at natural size anchored by `menu_anchor` (P2) | Firefox ignores transforms for anchor positioning — hence zoom, not transform |
| `canvas.width = round(grid.width·dpr)` | bitmap path (a) or (b), P2 by measurement | reference keeps bitmap at device ratio |
| three vitest fit cases | P1 unit tests here; P2 layout tests with headless app | dpr 1 pins layout only; states run at density 2 |

## P1 commits

- `1f9c1b6` h10-fit: fit.rs frame, inverse-hit and zoom helpers with
  vitest-literal tests (7 unit tests green; `./build.sh check` green).

## P1 judgment: reference probes (2026-10-20, ref-h10 at 68e1c7b, :5200)

Probe `.local/probe-ref-fit.py` (standalone Playwright: `act.py` hardcodes
`REFERENCE_ROUTE = "#function-viewer"`, so the welcome root route is opened
directly; seeding mirrors `init_script`'s mathviz-* keys, canvases forced to
sRGB). Evidence `.local/shots/h10-fit/ref-probe/` (PNG + JSON per case, 6x
crops in `crops6x/`).

- Fit values exact in all three engines: 1024×640 → 0.71, 1280×1000 → 0.88,
  1280×800 → 0.88, 768×1024 → 0.53, 1920×1200 → 1 (density 2; the dpr-1
  vitest literals 0.70/0.88 fall out of the same formula). Canvas bitmap is
  constant 1676×1036 = 838·2 × 518·2 at every fit: the bitmap stays at the
  device ratio of the design size, as `useUpdateCanvas` says.
- 1024×640: title 68.16 tall (= 96·0.71; the 67.2 vitest literal is the dpr-1
  fit-0.70 sibling), plane 596.4×369.2 (= 840·0.71 × 520·0.71), every part
  (mark, gear, title, button, hint, plane) inside the window, scroll ==
  window (1024×640). Mark at (22.72, 21.28) = (32·0.71, 30·0.71), 19.88 =
  28·0.71. Gear right edge 1001.28 = 1024 − 32·0.71.
- 1280×1000: plane 739.2×457.6, centre y 500.0, gear top 21.11 (≈ 24·0.88 =
  21.12), right inset 28.16 (= 32·0.88). Composition centred (title x 76.79 =
  87·0.88, the frame margin).
- Menu at 1024×640: right edge 1001.28 (= window − 32·fit), top 53.42 (≈
  64·fit + 8 = 53.44), 304 wide at natural size, inside the window — exactly
  what `menu_anchor` predicts.
- Paper-step case: (739.2/840)·50·2 = 88.0 whole. Chromium and Firefox agree
  with WebKit on fit, canvas size, bitmap and scroll in both probed sizes.
- 6x crops (centre: grid crossing + both axes + transformed solid curve +
  dashed original — the still is sinx with a = 3, so the dashed original
  shows; xlabel/ylabel: the axis labels) show the browser-shrunk full bitmap:
  axes, 1 px grid lines, STIX labels and the curve all sharp. The port-side
  (a)-vs-(b) measurement runs in P2 against these.
- Full-frame eyeball (1024×640, menu-open): composition centred, top row in
  the window's corners, paper and lights filling above and below, menu at
  natural size under the gear — matches the M5 judgment's picture.

## P2 resume pointer (blocked on siblings)

Nothing of P2 can start: all six sibling branches are still at the base
`8adaf58` (no commits), so `welcome.rs`, `welcome_backdrop.rs`,
`fit_scale.rs`, the `route` verb, the H10 group and the welcome boxes do not
exist anywhere. When the progress index shows h10-loop merged: `git merge
claude-port`, then (a) wire `frame_size` + `vg.scale(fit, fit)` at
`welcome.rs`'s root, the top-row `zoomed` layout with the menu at natural size
via `menu_anchor`, the resize re-measure (SDL resize + `Resize` verb); (b) run
the (a)-vs-(b) bitmap measurement at fit 0.71/0.88 on 6× crops and keep the
closer path; (c) add the three vitest fit cases as layout tests at dpr 1,
append the nine states to the H10 line (h10-t-route's group) and the fixture
states to `dom.py`; (d) `tools/run states H10 --targets ref,web,native
--ref-engines webkit,chromium,firefox --jobs 8`, judge, sweep H10 with the
panic run. State scripts cannot be drafted until h10-t-route's `route` verb
exists; layout tests cannot be drafted until h10-page's `welcome.rs` exists.
