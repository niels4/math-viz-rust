// The t0c twin: an Odin program drawing the render scene through the Odin app's own
// ports ($ODIN_SRC/vendor, $ODIN_SRC/src/text) to a PNG, the oracle for the 24 C patches
// before any page exists. The Rust render test (crates/mv-nvg/tests/render.rs) draws the
// same scene through the Rust FFI; the two PNGs must be identical at threshold 0.
// Build and run with tools/twin/run (needs odin and SDL3, not the Rust toolchain).
package twin

import "base:runtime"
import "core:c"
import "core:fmt"
import "core:log"
import "core:os"

import gl "o2/vendor/gl"
import nvg "o2/vendor/nanovg"
import nvg_gl "o2/vendor/nanovg/gl"
import sdl "o2/vendor/sdl3"
import stbi "vendor:stb/image"
import text "o2/src/text"

LOGICAL_W :: 400
LOGICAL_H :: 200
DENSITY :: 2
PX_W :: LOGICAL_W * DENSITY
PX_H :: LOGICAL_H * DENSITY

LINE_48 :: "MathViz To Va AV Ty 123"

OFFSETS := [4]f32{0, 0.25, 0.5, 0.75}

// The same bytes the page embeds (o2 is a byte-identical copy of the oracle's tree).
SEMI_BYTES :: #load("o2/assets/fonts/WorkSans-SemiBold.ttf")
BOLD_BYTES :: #load("o2/assets/fonts/WorkSans-Bold.ttf")

app_context: runtime.Context

append_png :: proc "c" (png: rawptr, data: rawptr, size: c.int) {
	context = app_context
	append((^[dynamic]u8)(png), ..([^]u8)(data)[:size])
}

check :: proc(ok: bool, what: string) -> bool {
	if !ok {
		log.errorf("twin: %s failed: %s", what, sdl.GetError())
	}
	return ok
}

main :: proc() {
	app_context = runtime.default_context()
	app_context.logger = log.create_console_logger(.Info, {.Level})
	context = app_context

	out := "twin.png"
	for i := 1; i < len(os.args); i += 1 {
		if string(os.args[i]) == "--out" && i + 1 < len(os.args) {
			out = string(os.args[i + 1])
		}
	}

	if !check(sdl.Init({.VIDEO}), "SDL_Init") {
		os.exit(1)
	}
	defer sdl.Quit()
	sdl.GL_SetAttribute(.CONTEXT_MAJOR_VERSION, 3)
	sdl.GL_SetAttribute(.CONTEXT_MINOR_VERSION, 3)
	sdl.GL_SetAttribute(.CONTEXT_PROFILE_MASK, i32(transmute(u32)sdl.GL_CONTEXT_PROFILE_CORE))
	sdl.GL_SetAttribute(.CONTEXT_FLAGS, i32(transmute(u32)sdl.GL_CONTEXT_FORWARD_COMPATIBLE_FLAG))
	sdl.GL_SetAttribute(.STENCIL_SIZE, 8)

	window := sdl.CreateWindow(
		"t0c twin",
		PX_W,
		PX_H,
		{.OPENGL, .HIDDEN},
	)
	if !check(window != nil, "SDL_CreateWindow") {
		os.exit(1)
	}
	defer sdl.DestroyWindow(window)
	gl_context := sdl.GL_CreateContext(window)
	if !check(gl_context != nil, "SDL_GL_CreateContext") {
		os.exit(1)
	}
	defer sdl.GL_DestroyContext(gl_context)
	if missing := gl.load(sdl.gl_set_proc_address); missing != nil {
		log.errorf("twin: no %s in this GL", missing)
		os.exit(1)
	}


	w, h: c.int
	sdl.GetWindowSizeInPixels(window, &w, &h)
	fmt.printf("twin: drawable=%d x %d\n", w, h)
	fmt.printf("twin: gl=%s %s\n", gl.GetString(gl.VERSION), gl.GetString(gl.RENDERER))
	vg := nvg_gl.Create({.ANTI_ALIAS, .STENCIL_STROKES})
	defer nvg_gl.Destroy(vg)

	fonts: text.Fonts
	if !text.init(&fonts) {
		os.exit(1)
	}
	defer text.destroy(&fonts)

	// The two faces, from the same bytes the page embeds (kept alive for the context).
	semi := nvg.CreateFontMem(vg, "Work Sans 600", SEMI_BYTES, false)
	bold := nvg.CreateFontMem(vg, "Work Sans 700", BOLD_BYTES, false)
	fmt.printf("twin: font ids semi=%d bold=%d atlas=%d\n", semi, bold, vg.fontImageIdx)

	draw(vg, &fonts, semi, bold)

	px := make([]u8, PX_W * PX_H * 4, context.temp_allocator)
	gl.PixelStorei(gl.PACK_ALIGNMENT, 1)
	gl.ReadPixels(0, 0, PX_W, PX_H, gl.RGBA, gl.UNSIGNED_BYTE, raw_data(px))

	png := make([dynamic]u8, context.temp_allocator)
	stbi.flip_vertically_on_write(true)
	if stbi.write_png_to_func(append_png, &png, PX_W, PX_H, 4, raw_data(px), PX_W * 4) == 0 {
		log.errorf("twin: could not encode %s", out)
		os.exit(1)
	}
	if err := os.write_entire_file(out, png[:]); err != nil {
		log.errorf("twin: could not write %s: %v", out, err)
		os.exit(1)
	}
	fmt.printf("twin: wrote %s\n", out)
}

// The render scene (plan 3.1): every number here is mirrored by the Rust render test.
draw :: proc(vg: ^nvg.Context, fonts: ^text.Fonts, semi, bold: int) {
	white := nvg.RGB(255, 255, 255)

	gl.ClearColor(30.0 / 255, 35.0 / 255, 48.0 / 255, 1)
	gl.Clear(gl.COLOR_BUFFER_BIT | gl.STENCIL_BUFFER_BIT)
	nvg.BeginFrame(vg, LOGICAL_W, LOGICAL_H, DENSITY)

	// Background.
	nvg.BeginPath(vg)
	nvg.Rect(vg, 0, 0, LOGICAL_W, LOGICAL_H)
	nvg.FillColor(vg, nvg.RGB(30, 35, 48))
	nvg.Fill(vg)

	// 48 px in the first face, overstruck at quarter-pixel pens.
	shaped48 := text.shape(fonts, {face = .Work_Sans_600, size = 48}, LINE_48)
	fmt.printf("twin: advance48=%f idx=%d\n", shaped48.advance, vg.fontImageIdx)
	placed48 := make([]nvg.Placed_Glyph, len(shaped48.glyphs), context.temp_allocator)
	for g, i in shaped48.glyphs {
		if g.font != .Work_Sans_SemiBold {
			log.errorf("twin: 48 px run fell back to %v", g.font)
			os.exit(1)
		}
		placed48[i] = {i32(g.id), g.x, g.y}
	}
	nvg.FontFaceId(vg, semi)
	nvg.FontSize(vg, 48)
	nvg.FillColor(vg, white)
	for off in OFFSETS {
		nvg.TextGlyphs(vg, 16 + off, 64, placed48)
	}
	fmt.printf("twin: after48 idx=%d\n", vg.fontImageIdx)

	// The second face, loaded after the first text: the atlas grows once here.
	shaped_bold := text.shape(fonts, {face = .Work_Sans_700, size = 48}, LINE_48)
	fmt.printf("twin: advance48bold=%f idx=%d\n", shaped_bold.advance, vg.fontImageIdx)
	placed_bold := make([]nvg.Placed_Glyph, len(shaped_bold.glyphs), context.temp_allocator)
	for g, i in shaped_bold.glyphs {
		if g.font != .Work_Sans_Bold {
			log.errorf("twin: bold run fell back to %v", g.font)
			os.exit(1)
		}
		placed_bold[i] = {i32(g.id), g.x, g.y}
	}
	nvg.FontFaceId(vg, bold)
	nvg.FontSize(vg, 48)
	nvg.FillColor(vg, white)
	for off in OFFSETS {
		nvg.TextGlyphs(vg, 16 + off, 132, placed_bold)
	}
	fmt.printf("twin: afterbold idx=%d\n", vg.fontImageIdx)

	// 15 px on one line.
	shaped15 := text.shape(fonts, {face = .Work_Sans_600, size = 15}, LINE_48)
	placed15 := make([]nvg.Placed_Glyph, len(shaped15.glyphs), context.temp_allocator)
	for g, i in shaped15.glyphs {
		placed15[i] = {i32(g.id), g.x, g.y}
	}
	nvg.FontFaceId(vg, semi)
	nvg.FontSize(vg, 15)
	nvg.FillColor(vg, nvg.RGB(200, 200, 200))
	nvg.TextGlyphs(vg, 16, 164, placed15)
	fmt.printf("twin: after15 idx=%d tritris=%d\n", vg.fontImageIdx, vg.textTriCount)

	// A filled rounded rectangle with a 1 px stroke at a half-pixel offset (the fringe).
	nvg.BeginPath(vg)
	nvg.RoundedRect(vg, 16.5, 172.5, 100, 20, 6)
	nvg.FillColor(vg, nvg.RGB(70, 130, 180))
	nvg.Fill(vg)
	nvg.StrokeColor(vg, white)
	nvg.StrokeWidth(vg, 1)
	nvg.Stroke(vg)

	// A scissor cutting a 3 px stroke midway (the scissor edge).
	nvg.Scissor(vg, 130, 174, 36, 16)
	nvg.BeginPath(vg)
	nvg.MoveTo(vg, 130, 182)
	nvg.LineTo(vg, 200, 182)
	nvg.StrokeColor(vg, nvg.RGB(255, 127, 80))
	nvg.StrokeWidth(vg, 3)
	nvg.Stroke(vg)
	nvg.ResetScissor(vg)

	// A stroke_feathered glow.
	nvg.BeginPath(vg)
	nvg.Circle(vg, 240, 182, 7)
	nvg.StrokeColor(vg, nvg.RGB(64, 224, 208))
	nvg.StrokeWidth(vg, 3)
	nvg.StrokeFeathered(vg, 1.5)

	// A path that merges into its first point, with sub-0.01 px pieces (the flatten guard).
	nvg.BeginPath(vg)
	nvg.MoveTo(vg, 290, 182)
	nvg.LineTo(vg, 290.00005, 182.00005)
	nvg.LineTo(vg, 290.00009, 182.00002)
	nvg.StrokeColor(vg, nvg.RGB(255, 80, 80))
	nvg.StrokeWidth(vg, 2)
	nvg.Stroke(vg)


	vp: [4]i32
	gl.GetIntegerv(0x0BA2, raw_data(vp[:]))
	fmt.printf("twin: viewport=%v\n", vp)
	nvg.EndFrame(vg)
}
