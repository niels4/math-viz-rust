# Decisions

What the port leaves out, and the calls it stands on. The milestones'
sections were consolidated here at the end.

## What is not ported, and why

- **The template's not-found page** (`_not_found.page.tsx`, the router's fallback for unknown routes) **and the dev theme-demo page** (`dev/theme-demo.page.tsx`, a developer route): SDL has no use for either. The port's router maps every unknown hash word to the welcome page.
- **The root route's history.** Before the welcome screen reached math-viz2's main, the root route (`#`, the template's placeholder card) was never in the port and the wordmark was a hot region with a no-op click. Since the welcome screen landed, the root route IS ported (the welcome page) and the wordmark navigates to it; the earlier no-op reading is superseded, kept here for the record.
- **The screen-reader live region** (`5a11915`): SDL has no accessibility tree, so the announcements have nowhere to go (a known gap in the README; the port keeps a text model for the readout, tested through `spoken()`).

## The run's own calls (overturnable by the user)

- **The build follows math-viz2's history.** Main from the first commit to `5cef052`, then the committed history of the user's `claude/canvas-mouse-handles` branch past main (h7), then main again after the user merged it (h8), then h9, then the welcome screen's merge (h10 at `68e1c7b`), where the run stops and reports. A later main commit that touches the app itself is a new milestone; the run waits at `68e1c7b`. Merges never wait for reviews; nothing paces against the machine.
- **A static site for GitHub Pages.** `docs/` at the repo root holds the release wasm app as a self-contained site (relative paths, `.nojekyll`), committed, rebuilt by `./build.sh pages` as the last step of a finished tree and by hand before a deploy — never by `./build.sh check` or a feature branch. Proved from a subpath by `tools/run pages`.
- **The native app opens on the welcome page**, as the browser does at `/`; `--route function-viewer` opens the viewer directly. The harness passes every existing script's route as `--route`, so no existing state changes.
- **The URL is the app state.** The camera (`center`, `scale`), the function (`func`), the four parameters (`a`, `b`, `h`, `k`) and the point (`p`) live in the page's hash query with a 200 ms debounce and no history entry; the hash route is the page route. On the web the build reads `location.hash` and writes through `history.replaceState` (`hashchange` back in); natively `--query` seeds it and `views.json` carries it.
- **The port's own prefs and keys.** App name `math-viz2-rust` (`mathviz/math-viz2-rust/prefs.json` under SDL's path); web keys `math-viz2-rust-light-dark`, `math-viz2-rust-dark-theme`, `math-viz2-rust-light-theme`. Building and testing against math-viz2 is unrestricted; nothing else is shared.
- **The layout slack is 0.5 CSS px** (not the first port's 0.02: this port writes its layout from CSS rather than from an Odin page that already matched).
- **The Chromium cap is 15/255 at anti-aliased edges** (0 on any flat pixel).
- **The theme change is a 250 ms linear cross-fade**, measured on the reference (Chromium runs it; WebKit runs an ease-like UA default, so mid-fade the port sits ~0.2 progress behind there with equal endpoints — one build can't match both, and the port keeps the measured linear).
- **No cap on agents or builds, no pacing**; milestones merge in history order; each milestone's reference is pinned at its end hash.

## Deliberate differences from math-viz2 (each with the state that shows it)

The port copies what math-viz2 means; where the stack cannot do what the DOM does, the port does the closest thing and records it here. Layout geometry is exact (boxes within 0.5 CSS px); these are behaviour and paint-model calls.

- **Popover animation skipped** (settings open states): the 160 ms scale/translate plays in CSS only; the port opens at once.
- **Hover is positional but sticky-dropped** (settings-hover-option): a pick that changes the theme drops hover until the next pointer move (measured: Chromium does not re-hover without a mouse move after the re-render); a same-value pick keeps it.
- **No half-pixel offset on the plane** (plane-default): Canvas 2D and NanoVG centre strokes alike, so grid/axis strokes sit on integer layout px with butt caps, one path per colour+width batch.
- **The canvas bitmap is the content box** (plane-default): the CSS radius rounds afterwards; the port scissors the content box, cuts the corners back and strokes the rim on the border box.
- **Overlay button always visible** (overlay-visible): probed unconditional at this hash, so no rule is ported.
- **Wheel takes DOM px** (zoom-wheel-3): `dx` ignored; web ×100 restores DOM px, native macOS notch 40 / precise 10.
- **Value terms print through `toFixed`** (readout-values): exact binary `toFixed` with the ASCII minus swapped for U+2212 (never `-0`); ties round half up on the binary value.
- **The fit is `min(1, (outer-1)/inner)` on whole-px inputs** (readout-narrow), about the line's left centre; no window width narrows the sidebar (the grid column is fixed).
- **Chevrons are not buttons** (rows-arrows): the arrows row state shows rest visuals only; display is `toFixed(2)` while the URL serialises `toFixed(3)`.
- **Scrub mode changes rebase** (box-drag-shift): `changeMode` sets `p.from` to the pre-snap value and resets only `p.x`; a mode-changing move sends once after the rebase.
- **Capture is conditional** (box-drag-out): a scrub holds the hover, a tap into the focused box never does.
- **Clicking while typing collapses the caret** (box-click-open); only whole scalars delete; click-to-caret placement is the accepted gap.
- **A refused commit still repaints** (rows-enter): taking the draft always dirties.
- **Text input runs on SDL_StartTextInput** (rows-typed): the harness's tap only queues text while it is active; arrows collapse the caret (React's value write resets the selection); Ctrl/Command+A is a `selected` flag the next text replaces.
- **The reset/Reset-all animation is a jump list** (reset-frames): one jump, or four in a/k/b/h order — the press turn still draws the old values while the boxes show the new ones, exactly like the flip.
- **Scrub modes are shared per parameter** (rows-lit-hk-drag): box drags sync the map; the ruler opens one card per mode-held param.
- **The diamond→square cursor quirk is preserved** (handle-drag states): `onHover` runs before the old handle's `onHoverEnd`, as the reference does; states approach the square from empty plane.
- **Q is quantized and settles** (q-hover): scripted hovers land one step stale, settle, then read; a trailing 1 px nudge leaves Q 0.02 off, by the source's quantize.
- **The wordmark navigates** (viewer-wordmark-click): a pointer-cursor region driving the route, since the welcome screen landed (the earlier no-op is history, above).
- **The P x field commits to 3 decimals** (field-enter): Esc restores, arrows nudge live, blur commits, unparseable text takes the error tone.
- **The minus span keeps its 3 px** (readout-lit-hk): `.minus { padding-left: 3px }` rides the minus run; the form fraction pads `round(0.15em)` of its own font (2 px form, 3 px value); the sup digit keeps its 1 px left margin; term glyphs start past the 3 px box padding (wrap's readout fix: layout and paint now accumulate the same advances, verified to 0.03 px run by run).
- **Value bars follow the 4 px radius** (readout-lit-hk): the three 1 px value-line shadows paint as radius-following strips, like the form's inset underline.
- **The welcome loop is a 451-frame tour** (welcome-loop-frames): holds and coarse scrub drags eased by cubic-bezier(0.4,0,0.2,1), switches drawn on; the still under reduced motion; the fit rounds down to whole device pixels per 50 px paper step.
- **Fresh mounts hover nothing** (route-roundtrip): hover holds off until the pointer acts, because the reference's post-navigation hover is racy engine to engine; both sides photograph unlit deterministically.
- **The CTA activates on Enter only** (welcome-cta-click): an anchor activates on Enter, never Space.
- **Selector cells keep the pointer** (cursor-cells-hover): the user's deliberate departure — the reference is the arrow everywhere (auto in WebKit, default in Chromium/Firefox from the covering radio input); no cursor rule on the cells at `68e1c7b` or main.
- **Menu cells and rows keep the pointer** (cursor-menu-cell): a consistency call, overturnable — the reference is default in all three engines (its labels carry no cursor rule), and matching it is a two-line revert in `settings.rs`.
- **The WebKit link quirk is recorded, not matched** (cursor-cta-click): this WebKit computes `auto` on links (its UA sheet lacks the rule); real Safari shows the hand and the port keeps pointer.
- **Cursors never screenshot**: changing a region cursor cannot move a pixel (proved by same-run 0 differences on H1+H3 before/after).

## Measured engine quirks (category e: the engines disagree, the port holds)

- **SwiftShader edges (Chromium, web only):** at most 15/255 at anti-aliased and glyph edges; isolated single pixels peak higher (41 at (1599,331) and 40 at (1730,817) on drifted dash ends; 42/36/35 on the hero curve; 41-57 on 3-7 welcome pixels per state; 5353 on box-wheel; 127k uniform-noise px on home-end at fractional camera). Native and web-Chromium are the same binary pixels through different compositors.
- **Firefox's late frame (web only):** theme fades capture all-final; the zoom-then-drag gap is reference-side; moves past the viewport edge get broken coordinates (both sides agree); selection paints opaque `#3399FF` with white text (the port reuses the scrub dark blue (83,123,165), 2 levels).
- **WebKit skips buttons on Tab** (no Full Keyboard Access in the launched browser): flip-focus, settings-focus-radio and welcome-tab-cta land on the next field/box with select-all and the tip bar, while the port, Chromium and Firefox stop on the control. The port keeps the tested DOM order; one walk can't match an engine-split reference (overturnable, question for the user). The WebKit bar clusters are category (e). The flip-focus/settings-focus-radio growth vs h8 (+3487/+3546) is entirely ref-side (the reference paints its focused-box tip caps Secondary, 8140 px in the c0 box); the port paints 0 px Secondary there because focus never enters a field on its walk.
- **WebKit's long-path dash drift:** ~+2.5 device px down-path on the ~9900 px dashed original (90 px in 58 micro-clusters, max 117 on the steep runs) while the port, Blink and Gecko agree to ≤1 px — the port keeps the spec math.
- **WebKit-only (X,space) kerns** ((space,t) −12u, (space,V) −31u apply in kb, in no browser; (x,space) −20u and friends apply in WebKit and kb, Chromium/Firefox skip most): the port shapes each word with its trailing space. nbsp pairs disagree up to 1.2 px among engines (left as the tests pin them).
- **Reference-side 1 px ticks:** chromium/firefox draw one axis tick 1 device px off WebKit (13-17 px slivers; the port matches WebKit); the plane labels sit 9 device px higher there (y-axis 5: 413 vs 422; x-axis 4: 913 vs 922; 22 rows vs WebKit's 20).
- **Light-theme 1-level seams:** the top bar's shadow fade (page background equal); the full-width hairline rows and the uniform +1 light background tone on the three light themes (b-shaped single-device rows at ≤2 levels with blend rounding; positions exact; earliest showing H5-era, absent from H1's archived shots) — not a defect.
- **Display P3 rounding:** curve edges at most 1 px in P3 conversions (SwiftShader); the wheel/trackpad zoom-divergence P3 quirk.
- **Density-2 Chromium rapid notches:** two −100 notches read 1.04 there vs 1.06 everywhere else (input quirk, both sides).
- **Firefox menu swatch/label islands** (0.4-1k px on settings-open/pick-light): subpixel swatch/glyph fringe where the references disagree among themselves; the port sides with WebKit+Chromium.
- **Firefox light-background rounding:** 1 level over ~20 rows on sage-editorial's canvas top (port matches WebKit).
- **`parse_center`/`parse_scale` reject radix literals beyond u64** where JS rounds to f64 (unreachable through the URL UI; recorded, not fixed).
- **The field-arrows cold-profile race:** one WebKit run showed 2.00 (arrows lost); flaky cold start, to re-verify with a settle wait if it recurs.

## The bar's categories (plan §1.9 item 3), with the largest named cluster of each

- **(a) glyph ink** (stb_truetype vs the engines' rasterisers, edges and weight): hundreds of px per state at most (e.g. ~448 px readout clusters, 2.2k px Chromium tab ring); positions and boxes exact.
- **(b) one device row or column at an edge** (stroke, curve, corner, box): tick slivers, hairline rows, the h plate's right column (64 px), plate top corners.
- **(c) Canvas 2D vs NanoVG stroke geometry** (caps, joins, sub-pixel positions): curve arms, gridline subpixel caps (235 px slivers), dash caps.
- **(d) Chromium's SwiftShader and Firefox's late frame, web only**: the single-pixel 41s, hero 42/36/35, welcome 41-57s, box-wheel 5353; Firefox all-final fades and #3399FF selection.
- **(e) a recorded engine quirk**: WebKit dash drift (117 max), the 34-68k px Tab-walk splits, the 9 px label offset, Firefox menu islands, the (X,space) kerns, the density-2 wheel notch. The union after the last milestone recounts every cluster against the reference in three engines.
