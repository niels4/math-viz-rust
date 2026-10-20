package text

import "core:math"

// CSS's line boxes, as WebKit lays them out (plan call 3 d: the formula with the face's ascent
// and descent kept fractional; Chromium rounds each to whole px and Firefox rounds each up, which
// moves a glyph by up to half a pixel).

// How far below the top of a line box `line_height` px tall the baseline of text in `face` at
// `size` sits: the half-leading, the line height less the content area (ascent and descent),
// split above and below, then the ascent. Fractional, as WebKit keeps it (its layout floors it to
// 1/64 px; the glyphs' baseline then snaps to the device pixel where they are drawn).
baseline :: proc(fonts: ^Fonts, face: Face, size, line_height: f32) -> f32 {
	m := metrics(fonts, face)
	ascent, descent := size * m.ascent, size * m.descent
	return (line_height - (ascent + descent)) / 2 + ascent
}

// The height `line-height: normal` gives text in `face` at `size`: the ascent, the descent and
// the line gap, each rounded to whole px, halves up, as WebKit and Chromium make it (Firefox
// rounds the first two up and keeps the gap: plan call 3 b). Work Sans 600 at 14: 13 + 3 + 0 = 16;
// STIX Two Text at 18: 14 + 4 + 5 = 23.
line_height_normal :: proc(fonts: ^Fonts, face: Face, size: f32) -> f32 {
	m := metrics(fonts, face)
	return half_up(size * m.ascent) + half_up(size * m.descent) + half_up(size * m.line_gap)
}

// The `ch` unit in `style`: the advance of its "0", features included, letter spacing not. In
// WebKit and Chromium it is the font's own advance (10.8018 px for Roboto Mono 500 at 18), so
// `round(up, 5ch, 1px)` is 55 px; Firefox holds it to its 1/60 px unit (10.8 px, 54): plan
// call 3 c.
ch :: proc(fonts: ^Fonts, style: Style) -> f32 {
	plain := style
	plain.tracking = 0
	return measure(fonts, plain, "0")
}

// The width a box takes from the text in it (fit-content: the wordmark, the theme chip, the title
// block): the text's advance rounded up to the 1/64 px WebKit and Chromium lay out in, so
// "MathViz" at 24 (97.909 px) makes a box 97.921875 wide in both, and "Arctic Ice" (65.495) one
// 65.5 wide. A thousandth of a px below a step counts as that step, as round_up's whole px do.
layout_width :: proc(advance: f32) -> f32 {
	return math.ceil(advance * 64 - 1e-3) / 64
}

// CSS's round(up, v, 1px): the whole px at or above v. A thousandth of a px below a whole px
// counts as that px, as React's useWholePxRuns rounds a measured width, so float noise in a sum
// of advances doesn't add a pixel.
round_up :: proc(v: f32) -> f32 {
	return math.ceil(v - 1e-3)
}

@(private = "file")
half_up :: proc(v: f32) -> f32 {
	return math.floor(v + 0.5)
}
