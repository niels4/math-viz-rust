package text

import nvg "../../vendor/nanovg"

// A colour as NanoVG takes it: sRGB, each channel and the alpha 0 to 1 (theme.Color).
Color :: [4]f32

// `text` in `style`, its pen starting at x on the baseline y.
draw :: proc(fonts: ^Fonts, vg: ^nvg.Context, style: Style, x, y: f32, text: string, ink: Color) {
	draw_shaped(fonts, vg, style.size, x, y, shape(fonts, style, text), ink)
}

// A typeset line with its top left at x, y.
draw_line :: proc(fonts: ^Fonts, vg: ^nvg.Context, line: Line, x, y: f32) {
	for p in line.placed {
		draw_shaped(fonts, vg, p.size, x + p.pen, y + p.baseline, p.shaped, p.ink)
	}
}

// A shaped run at `size` px, its pen starting at x on the baseline y. The one place glyphs reach
// the rasteriser (fontstash's stb_truetype, through nvg.TextGlyphs: each glyph's baseline on the
// device pixel, its pen on a quarter of one): a change to how the page's glyphs are rasterised,
// such as thickening them to the weight macOS's font smoothing gives the browsers' text (with the
// user, M4a's review), goes here.
draw_shaped :: proc(fonts: ^Fonts, vg: ^nvg.Context, size: f32, x, y: f32, shaped: Shaped, ink: Color) {
	glyphs := shaped.glyphs
	if len(glyphs) == 0 {
		return
	}
	nvg.FontSize(vg, size)
	nvg.FillColor(vg, ink)
	placed := make([]nvg.Placed_Glyph, len(glyphs), context.temp_allocator)
	for g, i in glyphs {
		placed[i] = {i32(g.id), g.x, g.y}
	}
	// Each file's glyphs go to NanoVG together: the face's, and the symbols file's where a run
	// fell back to it.
	start := 0
	for i in 1 ..= len(glyphs) {
		if i < len(glyphs) && glyphs[i].font == glyphs[start].font {
			continue
		}
		nvg.FontFaceId(vg, fonts.vg[glyphs[start].font])
		nvg.TextGlyphs(vg, x, y, placed[start:i])
		start = i
	}
}
