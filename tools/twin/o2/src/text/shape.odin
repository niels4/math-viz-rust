package text

import "core:strings"

import kbts "../../vendor/kb_text_shape"

// What a rule of the page sets on a run of text: CSS's font family, weight and style (the face),
// font-size, letter-spacing and the features it turns on or off.
Style :: struct {
	face:     Face,
	size:     f32, // px, the em size
	tracking: f32, // px after every character, the last included, as CSS's letter-spacing
	features: Features,
}

// Departures from a run's defaults: kerning is on unless turned off (font-kerning: none);
// figures are proportional unless tabular (font-variant-numeric: tabular-nums).
Feature :: enum u8 {
	No_Kerning,
	Tabular,
}
Features :: bit_set[Feature;u8]

// A glyph of a shaped run.
Glyph :: struct {
	id:      u16, // its index in `font`
	font:    Font_File,
	x, y:    f32, // where it is drawn from the run's origin on the baseline, px, y down
	advance: f32, // how far it moves the pen, px: kerning and letter spacing in
	source:  i32, // the byte offset in the text of the character it draws
}

// A run shaped: its glyphs in order, and its advance (the width CSS gives it).
Shaped :: struct {
	glyphs:  []Glyph,
	advance: f32,
}

// `text` shaped in `style`, left to right: each glyph from the face's file, or from its symbols
// file where the face lacks the character, kerned by the font's GPOS pairs, with tabular figures
// where asked, letter spacing after every character, and no ligature where letters are spaced
// (CSS Text 3 § 8.2, as the engines do).
//
// Every pair the font holds is kerned, the pairs around a space included, as Chromium kerns them
// under the page's `font-kerning: auto` (plan call 3 c: WebKit kerns a pair into a space but not
// one out of it, Firefox neither).
shape :: proc(
	fonts: ^Fonts,
	style: Style,
	text: string,
	allocator := context.temp_allocator,
) -> (
	shaped: Shaped,
) {
	// kb_text_shape 2.21 writes through a null pointer (kbts__UpdateBreaks) when it shapes an
	// empty text: natively a SIGSEGV, on wasm a silent write at address 0.
	if len(text) == 0 {
		return
	}
	files := FACES[style.face]
	// kb_text_shape tries the font on top of its stack first: the symbols file goes in below.
	if symbols, has := files.symbols.?; has {
		_ = kbts.ShapePushFont(fonts.ctx, &fonts.kb[symbols])
	}
	_ = kbts.ShapePushFont(fonts.ctx, &fonts.kb[files.font])
	defer {
		_ = kbts.ShapePopFont(fonts.ctx)
		if files.symbols != nil {
			_ = kbts.ShapePopFont(fonts.ctx)
		}
	}

	// The page is left to right (no dir attribute): with the paragraph's direction given, a
	// space that starts the text joins the run that follows it rather than making one of its own.
	kbts.ShapeBegin(fonts.ctx, .KBTS_DIRECTION_LTR, .DONT_KNOW)
	overrides: [4]kbts.feature_tag
	count := 0
	if .No_Kerning in style.features {
		overrides[count] = .kern
		count += 1
		kbts.ShapePushFeature(fonts.ctx, .kern, 0)
	}
	if .Tabular in style.features {
		overrides[count] = .tnum
		count += 1
		kbts.ShapePushFeature(fonts.ctx, .tnum, 1)
	}
	if style.tracking != 0 {
		overrides[count] = .liga
		overrides[count + 1] = .clig
		count += 2
		kbts.ShapePushFeature(fonts.ctx, .liga, 0)
		kbts.ShapePushFeature(fonts.ctx, .clig, 0)
	}
	kbts.ShapeUtf8(fonts.ctx, text, .SOURCE_INDEX)
	for i := count - 1; i >= 0; i -= 1 {
		_ = kbts.ShapePopFeature(fonts.ctx, overrides[i])
	}
	kbts.ShapeEnd(fonts.ctx)

	glyphs := make([dynamic]Glyph, 0, len(text), allocator)
	pen: f32
	for {
		run := kbts.ShapeRun(fonts.ctx) or_break
		file := file_of(fonts, run.Font)
		scale := style.size / fonts.units_per_em[file]
		for {
			g := kbts.GlyphIteratorNext(&run.Glyphs) or_break
			source: kbts.shape_codepoint
			_ = kbts.ShapeGetShapeCodepoint(fonts.ctx, g.UserIdOrCodepointIndex, &source)
			advance := f32(g.AdvanceX) * scale
			// A mark attached to its base takes no spacing of its own: the spacing follows the
			// character it belongs to.
			if g.AttachGlyph == nil {
				advance += style.tracking
			}
			append(
				&glyphs,
				Glyph {
					id      = g.Id,
					font    = file,
					x       = pen + f32(g.OffsetX) * scale,
					y       = -f32(g.OffsetY) * scale, // font units are y up
					advance = advance,
					source  = source.UserId,
				},
			)
			pen += advance
		}
	}
	return {glyphs[:], pen}
}

// The width CSS gives `text` in `style`: its advance, letter spacing after the last character
// included.
measure :: proc(fonts: ^Fonts, style: Style, text: string) -> f32 {
	return shape(fonts, style, text).advance
}

// A word's share of the width of the whole text it is in, for a measurer that adds up a text's
// words with a space measured alone between them (Clay): the advance of the word's glyphs as the
// whole text shapes them, so a kerning pair from its last letter into the space after it counts,
// and what kerning gave the space before it, since a pair out of a space is kerned too (shape)
// and the measurer's space is the space alone. So the shares and the spaces add up to the whole
// text's advance. `start` and `end` are the word's byte offsets in `whole`.
measure_word :: proc(fonts: ^Fonts, style: Style, whole: string, start, end: int) -> (share: f32) {
	if fonts.words.style != style || fonts.words.text != whole {
		forget_words(fonts)
		fonts.words.style = style
		fonts.words.text = strings.clone(whole, fonts.allocator)
		fonts.words.shaped = shape(fonts, style, whole, fonts.allocator)
	}
	for glyph in fonts.words.shaped.glyphs {
		source := int(glyph.source)
		if source >= start && source < end {
			share += glyph.advance
		} else if source == start - 1 && whole[source] == ' ' {
			alone :=
				f32(fonts.space[glyph.font]) * style.size / fonts.units_per_em[glyph.font] + style.tracking
			share += glyph.advance - alone
		}
	}
	return
}

// Lets go of the text measure_word keeps.
forget_words :: proc(fonts: ^Fonts) {
	delete(fonts.words.text, fonts.allocator)
	delete(fonts.words.shaped.glyphs, fonts.allocator)
	fonts.words.text = ""
	fonts.words.shaped = {}
}

// The file a run of kb_text_shape's came from.
@(private = "file")
file_of :: proc(fonts: ^Fonts, font: ^kbts.font) -> Font_File {
	for file in Font_File {
		if &fonts.kb[file] == font {
			return file
		}
	}
	panic("kb_text_shape shaped a run in a font it wasn't given")
}
