# vendor

Third-party C libraries the port builds from, kept here at recorded versions so a clone
builds offline with only the Rust toolchain and Emscripten installed (emsdk is the one
toolchain fetched outside it). `crates/mv-sys/build.rs` compiles them with `cc` (`-O2
-ffp-contract=off`, so native and web lay out and rasterize with the same arithmetic;
`NVG_NO_STB`; the GL3 backend natively, GLES3 on Emscripten).

| Directory | What | Upstream source | Version |
|---|---|---|---|
| `clay/` | Clay's `clay.h` | github.com/nicbarker/clay | commit `e6cc369` (2026-05-20) |
| `nanovg/` | NanoVG (`nanovg.c/.h`, `nanovg_gl.h`, `nanovg_gl_utils.h`) and its bundled `fontstash.h` | github.com/memononen/nanovg | commit `ce3bf74` (2026-02-19) |
| `nanovg/stb_truetype.h` | stb_truetype, fontstash's rasterizer | github.com/nothings/stb | v1.26 (2021-08-28) |
| `nanovg/stb_rect_pack.h` | stb_rect_pack, vendored but compiled by no translation unit (nothing references it: fontstash packs the atlas with its own skyline allocator) | github.com/nothings/stb | v1.01 |
| `kb_text_shape/` | kb_text_shape (Unicode segmentation and OpenType shaper) | github.com/JimmyLefevre/kb | 2.21 |

Licences: Clay `clay/LICENSE.md` (zlib); NanoVG and Fontstash are zlib (in their headers);
stb is public domain or MIT (in each header); kb_text_shape `kb_text_shape/LICENSE` (zlib).
Altered source versions are marked `math-viz-rust:` in the code, as the licences require.

## What changed from upstream, and why

Every change is marked `math-viz-rust:` in the code, beside the `math-viz-odin:` mark in
the Odin port's `vendor/` it answers (file and line in the Odin port at `3d23626`).

- **`fontstash.h`: glyphs by index** (answers `fontstash.odin:38,75,83,465,498` —
  `SUBPIXEL_STEPS`, `Glyph.codepoint`/`subpixel`, `GetGlyphByIndex`, `__placeGlyph`).
  `fons__getGlyphByIndex` looks a glyph up by its index in the font at an em size (the
  size a shaper's font units scale by, where the codepoint path sizes the height from
  ascender to descender) and a horizontal offset of a quarter pixel
  (`FONS_SUBPIXEL_STEPS`, each glyph rasterized at the offset its pen falls on via
  `stbtt_MakeGlyphBitmapSubpixel`); `fons__glyphQuad` gives its atlas quad (answers
  `fontstash.odin:1041` — `GlyphQuad`). The atlas placement and rasterising moved from
  `fons__getGlyph` into `fons__placeGlyph`, which both paths use (the codepoint path draws
  the same pixels as before: offset 0), so a shaped run and a plain string share one
  rasterizer.
- **`fontstash.h`: the codepoint path's scale.** `fons__tt_getPixelHeightScale` sizes the
  height from ascender to descender (`stbtt_ScaleForPixelHeight`), not the em size upstream
  scales by (the Odin port's `__getPixelHeightScale`), so measured and drawn codepoint
  text matches the page's text metrics.
- **`fontstash.h` + `nanovg.c`: atlas growth a run can span** (answers
  `nanovg.odin:312,2796,2932,2936` — the resize callback, `__renderTextImage`, the `image`
  tracking in `Text`/`TextGlyphs`). When the atlas fills, `fons__placeGlyph` grows it with
  `fonsExpandAtlas`, which calls the new `nvg__atlasResize` before copying the pixels:
  what the frame drew from the old texture is uploaded first and the larger texture takes
  the next slot until `EndFrame`. A run drawn across the growth draws its first glyphs
  from the texture they are in (`nvg__renderTextImage`, in `nvgText` and `nvgTextGlyphs`).
- **`nanovg.c`/`nanovg.h`: `nvgTextGlyphs`** (answers `nanovg.odin:2992,3000` —
  `Placed_Glyph`, `TextGlyphs`). Draws a run a shaper has shaped (`NVGplacedGlyph`: a
  glyph index and its pen position from the run's origin on the baseline), glyph by glyph,
  with the current font, blur and fill colour. Each glyph's baseline is snapped to the
  device pixel and its pen to a quarter of one, as browsers position text.
- **`nanovg.c`/`nanovg.h`: `nvgStrokeFeathered`** (answers `nanovg.odin:2560` —
  `StrokeFeathered`). Strokes with an anti-aliased edge `feather` px wide (in the frame's
  units, as the device pixel's fringe is) in place of a device pixel's, which stacked
  strokes compose into a blurred-stroke profile.
- **`nanovg.c`: `nvg__flattenPaths` drops degenerate paths** (answers `nanovg.odin:1515`).
  A path whose points all merged into its first (a piece shorter than the distance
  tolerance) has nothing to draw; below, its last point would equal its first and
  `pts[count-1]` would index before its start, which C NanoVG reads silently.
- **`nanovg.c`: zeroed font textures** (answers `nanovg.odin:331,338,2783` — the font
  texture from fontstash's buffer, `__createFontTexture`, the zeroed growth texture).
  `nvg__createFontTexture` (calloc) backs the atlas at creation and at every growth: a
  texture made without data is undefined until first use, which some GL implementations
  clear lazily and warn about.
- **`nanovg.h`: no image loading from paths** (answers `nanovg.odin:907`). With
  `NVG_NO_STB` (set by the build script) `nvgCreateImage`/`nvgCreateImageMem` are
  undeclared; images go through `nvgCreateImageRGBA`. Emscripten's libc clashes with the
  libc shim image loading needs.
- **`nanovg_gl.h`: the scissor's edge in device pixels** (answers `gl.odin:208,745,833` —
  `Context.pixelFringe`, the scissor edge, `__renderViewport`). `GLNVGcontext.pixelFringe`,
  set from the viewport, replaces the call's fringe in the scissor scale: a stroke
  feathered over 2 px would otherwise draw 1 px past the scissor and fade inside it. The
  1 x 1 texture bound for fills without an image is made from a zero byte, not from none
  (answers `gl.odin:400`).
- **`nanovg.c`: `1 - KAPPA90` rounded once** (answers `nanovg.odin:25,2451-2457` —
  `KAPPA`, `RoundedRectVarying`). The Odin port's `KAPPA` is an untyped constant,
  so its `rx*(1 - KAPPA)` rounds the exact decimal once, to `0x3ee53aef`. C's
  `(1 - NVG_KAPPA90)` rounds twice (the `f`-suffixed literal to f32, then the
  subtraction: `0x3ee53aee`), which moves large-radius rounded-rect control
  points by 1 ulp (pre-fix C vs Odin: `0x41c3917f` vs `0x41c39180`). `NVG_KAPPA90C`
  (`0.44771525f`, which rounds to `0x3ee53aef`) replaces `(1 - NVG_KAPPA90)` in
  `nvgRoundedRectVarying`, so the products match the Odin port's bit for bit.
  Direct `*NVG_KAPPA90` multiplies (Ellipse, Arc) already agree and are untouched.
- **`nanovg.c`: `tan` as `sin`/`cos`** (answers the `math.tan` the Odin port's
  `ArcTo` and skew procs call — `tan_f32(θ) = sin(θ)/cos(θ)` — rather than the
  C library's `tanf`). `nvg__tanf` divides `sinf` by `cosf`, so `ArcTo`'s
  tangent distance and the skew transforms match the Odin port bit for bit.
- **`nanovg.c`: `atan2` as the Odin port's software `atan2`** (answers Odin
  core:math `atan2_f64` — a port of Stephen Moshier's Cephes `atan2` —
  narrowed to f32 as the Odin port's `math.atan2(f32)` does). C's `atan2f`
  computes in f32 directly; the two differ by 1 ulp on some inputs, which
  moves round-join and arc vertices (fuzz cases 61/70/100, the plane curve).
  `nvg__odin_atan2` transliterates the Odin source (constants and evaluation
  order kept); `nvg__atan2f` narrows its result, and the round-join call sites
  go through the wrapper.
- **`nanovg.c`: `acos` left as libm `acosf`** (evaluated residual, answers Odin
  `core/math` `acos_f32 = f32(acos_f64)`, FreeBSD msun in f64 then narrowed).
  The two call sites — the round-join `ncap` (`nanovg.c:1574`) and the arc
  angle (`nanovg.c:2180`, via the trivial `nvg__acosf`) — still call libm.
  Raw-C-vs-Odin trig fuzz differs 1 ulp on ~1/200 inputs with no vertex
  effect on any of the 160 stroke + 160 fill fuzz cases (0 diffs), so the
  port leaves it unported; port if a case needs it.
- **Second residual: raw `sinf`/`cosf` vs Odin's `sin`/`cos`** (unobserved).
  The committed trig fuzz (`/opt/dev/tmp/rust-port/nvg-dust/trig-c.txt` vs
  `trig-odin.txt`, recomputed by the review and i5) shows 18 atan2 + 3 sin +
  3 cos differences of 200 inputs, all exactly 1 ulp; the `sin`/`cos` columns
  are identical everywhere else (`sqrt` identical throughout). The wrappers
  route `tan` through `sin`/`cos` and `atan2` through `nvg__odin_atan2`
  (0/200 vs Odin's), so the residual never reaches a vertex on any fuzz
  case; libm stays.
- **No C change needed:** `gl.odin:96` (`ShaderNum`) — the C `GLNVGfragUniforms` already
  declares the non-UBO fields `float`; and `gl.odin:142` (GL3 natively, GLES3 on the web)
  — a build-script selection (`NANOVG_GL3_IMPLEMENTATION` /
  `NANOVG_GLES3_IMPLEMENTATION`).
