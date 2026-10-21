# h11-p-drag progress

Port of math-viz2's `ea29aa9` (hovering derived from the hovered index) then `7686c89`
(P's x dragged on the graph), the two app commits of h11's §3.9 entry that are not the
welcome hero link or the x² default. Worktree
`/opt/dev/agent/worktrees/math-viz2-rust/claude-port-h11-p-drag`, branch
`claude-port-h11-p-drag`, base `287b7a3` (h1 to h10 but h10-fit).

Reference: `/opt/dev/agent/worktrees/math-viz2/rust-port-ref-h11` detached at `c6754e7`,
served disk-only on 5201 in tmux `math-viz2-rust-ref-h11` (not mine to restart).
`REACT_REF=/opt/dev/agent/worktrees/math-viz2/rust-port-ref-h11`, `REACT_REF_PORT=5201`.

## Checklist

- [x] Read the spec, the plan's §1.9/§3.9, the idiom skill, the brief's rulings, the two
      commits, the TS at `c6754e7`, the port's `plane_input.rs`/`plane_geometry.rs`/`page.rs`.
- [x] P's geometry probed on ref-h11 (see "Geometry" below).
- [x] Phase 1 commit 1: `ea29aa9` — `App.hovering` deleted, `plane_cursor` reads
      `app.hovered.is_some()`, the H7 state and its test flipped, `./build.sh check`.
      Commit `c7589fd`; `cargo test` 553 passed before and after (the renamed
      test replaced the old one 1:1; 0 deleted).
- [x] Phase 1 commit 2: `7686c89` — `hit_test` over handles + handler markers, `PlotMarker`
      gains `handlers`, `HandleId::PointP`, the P hover/press/drag/release arms, the
      component test and the hit tests, `./build.sh check`.
- [ ] States drafted and probed on ref-h11 (`act --target ref`, three engines) and run
      natively as self-checks. Not judged in phase 1 (the base's default function is x,
      so a P state differs from ref-h11 by the whole curve until h11-default merges).
- [ ] Same-run proof of (1): H7 natively before and after, every shot byte-identical.
- [ ] Phase 2 (after h11-default and h11-hero merge into claude-port): merge `claude-port`,
      `./build.sh check`, the judged run `H7 H8 H11 --targets ref,web,native
      --ref-engines webkit,chromium,firefox --jobs 8`, `tools/run judge`, sweep `H7 H8 H11`.
- [ ] `CLAUDE.md` block, the supersession note in claude-port's copy of
      `progress/h7-plane.md`, the shots and reports in
      `/opt/dev/tmp/math-viz2-rust/h11-p-drag/`.

## Geometry (probed on ref-h11, 2026-10-20)

The canvas's box at 1440 × 900 is (432, 96, 992, 788) CSS (page.json's `canvas`, WebKit),
so the content box is (433, 97, 990, 786) and the grid's origin is
(433 + 990/2, 97 + 786/2) = (928, 490) CSS at the default camera (center [0,0], scale
0.5, so 50 CSS px a unit).

The default function is x² (6ba8b85) and P's default x is 2, so P is at plane (2, 4),
i.e. CSS (928 + 100, 490 − 200) = **(1028, 290)**.

Measured from the reference's own pixels: a probe state with the pointer moved to
(1028, 290) and settled (probe-geom.act, WebKit) gives the P label's pink
(`--point-p` = rgb(249, 168, 212), the bullseye's ring and core) blob centroid at
device (2268.7, 512.0). The blob spans device x 2100–2437 and y 472–552, i.e. the
marker plus its label pill. The prompt's own estimate (1028, 290) is confirmed by the
marker's own centre: the pink blob's left edge at device 2100 = CSS 1050 is the marker's
left edge (P at 1028 + the 12 px radius = 1040, the pill's pad making it 1050), and the
y span 472–552 device = CSS 236–276 brackets 290 with the pill above. Re-probe on the
states themselves before judging (the settle-and-nudge pattern the q-hover states use).

## Phase 1, commit 1: ea29aa9 — `c7589fd` (2026-10-20)

`App.hovering` is deleted; `plane_cursor` reads `app.hovered.is_some()`. The `set_hover`
calls keep their order (the lit list after the diamond-to-square jump is unchanged: a, b
lit). The old test
`hovering_straight_to_the_square_leaves_pan_with_a_b_lit` becomes the new expectation
(grab-hover with a, b lit) and is renamed to
`hovering_straight_to_the_square_gives_the_open_hand_with_a_b_lit`.
`tools/states/handle-hover-move.act` keeps its inputs and gets its comment and expected
cursor rewritten (grab-hover). `rows-lit-ab-hover.act`'s comment is corrected: it is
approached from empty plane, not a jump, so nothing there changes.

`cargo test` 553 passed before and after the edit (the renamed test replaced the old
one 1:1; 0 tests deleted). `./build.sh check` green (clippy web clean, `tools/check`
0 errors).

The supersession note went into claude-port's copy of
`.local/briefs/progress/h7-plane.md` (that file's `hovering`-state and pan-cursor
paragraphs are stale from `c6754e7`).

## Phase 1, commit 2: 7686c89

`hit_test` runs over the handles then the markers that carry a handler flag, last index
wins. `PlotMarker` gains `handlers: Option<PlotHandlers>` (P only). `HandleId::PointP`
appended. The P hover lights nothing and clears Q through the existing gate; the press
sets `grab.x = p − point.x`, `state.p_dragging = true`, `dragging = true`,
`dragged = Some(PointP)`; each move `p_param.set(quantize(point.x + grab.x), now_ms)`;
the release clears `p_dragging` and flushes `p_param`.

## Open questions

- None yet.
