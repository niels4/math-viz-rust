// Odin builds _test files into every build, and core:testing can't build for the web.
#+build !js
package text

import "core:strings"
import "core:testing"

// A run's advance at a size of one px per font unit: in the font's own units.
@(private = "file")
units :: proc(fonts: ^Fonts, face: Face, text: string, features: Features = {}) -> f32 {
	style := Style {
		face     = face,
		size     = fonts.units_per_em[FACES[face].font],
		features = features,
	}
	return measure(fonts, style, text)
}

// The metrics CSS's line boxes take (hhea), as CartesianPlane/faces.ts writes them for STIX and
// Roboto Mono.
@(test)
metrics_are_the_fonts :: proc(t: ^testing.T) {
	fonts: Fonts
	if !testing.expect(t, init(&fonts)) {
		return
	}
	defer destroy(&fonts)
	testing.expect_value(t, metrics(&fonts, .Work_Sans_600), Metrics{0.93, 0.243, 0})
	testing.expect_value(t, metrics(&fonts, .STIX_400_Italic), Metrics{0.762, 0.238, 0.25})
	testing.expect_value(t, metrics(&fonts, .Roboto_Mono_500), Metrics{2146.0 / 2048, 555.0 / 2048, 0})
}

// The fonts' GPOS kerning, to the unit (Work Sans SemiBold's own pairs, read with fontTools), the
// pairs around a space included, as Chromium kerns them under the page's `font-kerning: auto`
// (plan call 3 c): a pair that ends with a space ('a ' -14) and one that starts with a space
// (' V' -31), so "x V" loses 'x ''s -20 and ' V''s -31. A space that starts the text joins the
// run after it.
@(test)
kerning_is_the_fonts :: proc(t: ^testing.T) {
	fonts: Fonts
	if !testing.expect(t, init(&fonts)) {
		return
	}
	defer destroy(&fonts)
	pairs := [?]struct {
		text: string,
		kern: f32,
	} {
		{"To", -99},
		{"Va", -60},
		{"AV", -68},
		{"Ty", -67},
		{"a ", -14},
		{"x V", -51},
		{" V", -31},
		{"67", -18},
	}
	for p in pairs {
		kerned := units(&fonts, .Work_Sans_600, p.text)
		plain := units(&fonts, .Work_Sans_600, p.text, {.No_Kerning})
		testing.expectf(
			t,
			kerned - plain == p.kern,
			"%q kerned %v, unkerned %v: the page kerns it by %v",
			p.text,
			kerned,
			plain,
			p.kern,
		)
	}
	// The hmtx advances summed, then every pair kerned, the three after a space too (' V' -31,
	// ' A' -29, ' T' -21).
	testing.expect_value(t, units(&fonts, .Work_Sans_600, "To Va AV Ty", {.No_Kerning}), 5918)
	testing.expect_value(t, units(&fonts, .Work_Sans_600, "To Va AV Ty"), 5498)
	testing.expect_value(t, units(&fonts, .Work_Sans_600, " V"), 306 + 670 - 31)
}

// tnum swaps Work Sans's proportional figures for its tabular ones (zero.tf..nine.tf, 604 units
// each); STIX's figures are tabular as they come (but its 7, 494.37 units at 500 in the variable
// font itself, which the static instance rounds to 494) and Roboto Mono is monospaced.
@(test)
tabular_figures_share_one_advance :: proc(t: ^testing.T) {
	fonts: Fonts
	if !testing.expect(t, init(&fonts)) {
		return
	}
	defer destroy(&fonts)
	digits := "0123456789"
	for d in 0 ..< 10 {
		digit := digits[d:d + 1]
		testing.expect_value(t, units(&fonts, .Work_Sans_600, digit, {.Tabular}), 604)
		testing.expect_value(t, units(&fonts, .STIX_400, digit), 495)
		testing.expect_value(t, units(&fonts, .STIX_500, digit, {.Tabular}), 494 if digit == "7" else 495)
		testing.expect_value(t, units(&fonts, .Roboto_Mono_500, digit, {.Tabular}), 1229)
	}
	testing.expect_value(t, units(&fonts, .Work_Sans_600, "0"), 636)
	testing.expect_value(t, units(&fonts, .Work_Sans_600, "1"), 416)
	testing.expect_value(t, units(&fonts, .Work_Sans_600, digits, {.Tabular}), 6040)
	testing.expect_value(t, units(&fonts, .Work_Sans_600, digits), 5853)
}

// ≈ comes from the symbols file behind the face (STIX Two Math's ≈ is 720 units wide, Roboto
// Mono's 1229), each glyph from the file that has it.
@(test)
symbols_fall_back :: proc(t: ^testing.T) {
	fonts: Fonts
	if !testing.expect(t, init(&fonts)) {
		return
	}
	defer destroy(&fonts)
	run := shape(&fonts, {face = .Roboto_Mono_500, size = 14}, "≈0")
	if testing.expect_value(t, len(run.glyphs), 2) {
		testing.expect_value(t, run.glyphs[0].font, Font_File.Roboto_Mono_Symbols)
		testing.expect_value(t, run.glyphs[1].font, Font_File.Roboto_Mono_Medium)
		testing.expect_value(t, run.glyphs[1].source, 3) // ≈ is three bytes of UTF-8
	}
	testing.expect_value(t, units(&fonts, .STIX_400, "≈"), 720)
	testing.expect_value(t, units(&fonts, .Roboto_Mono_500, "≈"), 1229)
}

// Letter spacing after every character, the last included: the subtitle's 27 characters at
// 2.8 px add 75.6 px.
@(test)
tracking_follows_every_character :: proc(t: ^testing.T) {
	fonts: Fonts
	if !testing.expect(t, init(&fonts)) {
		return
	}
	defer destroy(&fonts)
	caps := "FUNCTIONS · TRANSFORMATIONS"
	plain := measure(&fonts, {face = .Work_Sans_600, size = 12}, caps)
	tracked := measure(&fonts, {face = .Work_Sans_600, size = 12, tracking = 2.8}, caps)
	testing.expectf(t, abs(tracked - plain - 27 * 2.8) < 1e-3, "tracked %v, plain %v", tracked, plain)
}

// Clay adds a text's words up with a space of its own between them: the words' shares and the
// spaces come to the whole text's advance, its kerning across the spaces included.
@(test)
words_add_up_to_the_whole :: proc(t: ^testing.T) {
	fonts: Fonts
	if !testing.expect(t, init(&fonts)) {
		return
	}
	defer destroy(&fonts)
	samples := [?]struct {
		style: Style,
		text:  string,
	} {
		{{face = .Work_Sans_700, size = 20}, "Function Viewer"},
		{{face = .Work_Sans_600, size = 12, tracking = 2.8}, "FUNCTIONS · TRANSFORMATIONS"},
		{{face = .Work_Sans_600, size = 24}, "To Va AV Ty"},
		{{face = .STIX_400, size = 20}, ") = ≈ 2"},
	}
	for s in samples {
		space := measure(&fonts, s.style, " ")
		sum: f32
		start := 0
		for start <= len(s.text) {
			end := strings.index_byte(s.text[start:], ' ')
			end = len(s.text) if end < 0 else start + end
			sum += measure_word(&fonts, s.style, s.text, start, end)
			if end < len(s.text) {
				sum += space
			}
			start = end + 1
		}
		whole := measure(&fonts, s.style, s.text)
		testing.expectf(
			t,
			abs(sum - whole) < 1e-3,
			"%q: the words add up to %v, the whole is %v",
			s.text,
			sum,
			whole,
		)
	}
}

// useFitWidth.test.tsx: an element that fits stays at full size; one 200 px that scale and 20 that
// don't, in 120 px of room, shrinks to half, the whole-px part included.
@(test)
fit_is_use_fit_width :: proc(t: ^testing.T) {
	Part :: struct {
		scaling, fixed: f32,
	}
	width_at :: proc(scale: f32, data: rawptr) -> f32 {
		part := (^Part)(data)
		return part.scaling * scale + part.fixed
	}
	fits := Part{100, 20}
	testing.expect_value(t, fit(300 + FIT_SLACK, width_at, &fits), 1)
	wide := Part{200, 20}
	scale := fit(120 + FIT_SLACK, width_at, &wide)
	testing.expectf(t, abs(scale - 0.5) < 1e-5, "scale %v, not 0.5", scale)
	testing.expectf(
		t,
		abs(width_at(scale, &wide) - 120) < 1e-3,
		"%v px wide, not 120",
		width_at(scale, &wide),
	)
}

// A readout's runs on one line (PointReadout): "(" in STIX in its whole-px box, the number in
// Roboto Mono right-aligned in a 5ch slot, ")" after; on one baseline, and centred line box by
// line box.
@(test)
typeset_places_boxes :: proc(t: ^testing.T) {
	fonts: Fonts
	if !testing.expect(t, init(&fonts)) {
		return
	}
	defer destroy(&fonts)
	math := Style {
		face = .STIX_400,
		size = 20,
	}
	number := Style {
		face     = .Roboto_Mono_500,
		size     = 18,
		features = {.Tabular},
	}
	slot := round_up(5 * ch(&fonts, number))
	pieces := [?]Piece {
		{style = math, text = "(", line_height = 25, box = .Whole_Px},
		{style = number, text = "2.00", line_height = 24, min_width = slot, align = .Right},
		{style = math, text = ")", line_height = 25, box = .Whole_Px, pad = {0, 7}},
	}
	line := typeset(&fonts, pieces[:], gap = 2)
	open := round_up(measure(&fonts, math, "("))
	digits := measure(&fonts, number, "2.00")
	testing.expect_value(t, slot, 55)
	testing.expect_value(t, line.placed[0].width, open)
	testing.expect_value(t, line.placed[1].x, open + 2)
	testing.expect_value(t, line.placed[1].width, 55)
	testing.expectf(
		t,
		abs(line.placed[1].pen - (open + 2 + 55 - digits)) < 1e-4,
		"the number starts at %v",
		line.placed[1].pen,
	)
	testing.expect_value(t, line.placed[2].width, round_up(measure(&fonts, math, ")")) + 7)
	testing.expect_value(t, line.width, line.placed[2].x + line.placed[2].width)
	// One baseline: Roboto Mono's sits lower in its box (18.99 px down its 24) than STIX's in
	// its (17.74 down 25), so the parentheses' boxes start lower than the number's.
	stix_base := baseline(&fonts, .STIX_400, 20, 25)
	mono_base := baseline(&fonts, .Roboto_Mono_500, 18, 24)
	testing.expect_value(t, line.baseline, max(stix_base, mono_base))
	for p in line.placed {
		testing.expect_value(t, p.baseline, line.baseline)
	}
	testing.expect_value(t, line.placed[1].top, line.baseline - mono_base)
	// Centred, each box in the line's height: the boxes' baselines differ by their own offsets.
	centred := typeset(&fonts, pieces[:], vertical = .Center)
	testing.expect_value(t, centred.height, 25)
	testing.expect_value(t, centred.placed[1].top, 0.5)
	testing.expect_value(t, centred.placed[1].baseline, 0.5 + mono_base)
	testing.expect_value(t, centred.placed[0].baseline, stix_base)
}

// An empty text shapes to nothing (kb_text_shape itself crashes on one).
@(test)
empty_text_shapes_to_nothing :: proc(t: ^testing.T) {
	fonts: Fonts
	if !testing.expect(t, init(&fonts)) {
		return
	}
	defer destroy(&fonts)
	run := shape(&fonts, {face = .Roboto_Mono_500, size = 17}, "")
	testing.expect_value(t, len(run.glyphs), 0)
	testing.expect_value(t, run.advance, 0)
	// And shaping goes on as before after it: T 635, o 599, kerned -99.
	testing.expect_value(t, units(&fonts, .Work_Sans_600, ""), 0)
	testing.expect_value(t, units(&fonts, .Work_Sans_600, "To"), 635 + 599 - 99)
}
