// Package text sets the page's text: the static faces (assets/fonts, embedded), runs shaped by
// kb_text_shape with the fonts' kerning and the features the page asks for, the advances Clay and
// the layout measure with, CSS's line boxes and baselines, runs of mixed faces on one line
// (typeset), the scale that fits a line to a width (fit), and the glyphs drawn through NanoVG.
// It is the only package that touches kb_text_shape or NanoVG's fonts. Lengths are logical px
// (CSS px); a size is the em size, CSS's font-size.
package text

import "base:runtime"
import "core:log"

import kbts "../../vendor/kb_text_shape"
import nvg "../../vendor/nanovg"

// The font files: static instances of the page's variable fonts at the weights it uses
// (`tools/run font`), and the two symbols files that hold its ≈ (U+2248).
Font_File :: enum u8 {
	Work_Sans_Regular,
	Work_Sans_Medium,
	Work_Sans_SemiBold,
	Work_Sans_Bold,
	Work_Sans_ExtraBold,
	STIX_Regular,
	STIX_Medium,
	STIX_Italic,
	STIX_Math_Symbols,
	Roboto_Mono_Medium,
	Roboto_Mono_SemiBold,
	Roboto_Mono_Symbols,
	STIX_SemiBold,
}

@(rodata)
FONT_DATA := [Font_File][]u8 {
	.Work_Sans_Regular    = #load("../../assets/fonts/WorkSans-Regular.ttf"),
	.Work_Sans_Medium     = #load("../../assets/fonts/WorkSans-Medium.ttf"),
	.Work_Sans_SemiBold   = #load("../../assets/fonts/WorkSans-SemiBold.ttf"),
	.Work_Sans_Bold       = #load("../../assets/fonts/WorkSans-Bold.ttf"),
	.Work_Sans_ExtraBold  = #load("../../assets/fonts/WorkSans-ExtraBold.ttf"),
	.STIX_Regular         = #load("../../assets/fonts/STIXTwoText-Regular.ttf"),
	.STIX_Medium          = #load("../../assets/fonts/STIXTwoText-Medium.ttf"),
	.STIX_Italic          = #load("../../assets/fonts/STIXTwoText-Italic.ttf"),
	.STIX_Math_Symbols    = #load("../../assets/fonts/STIXTwoMath-Symbols.ttf"),
	.Roboto_Mono_Medium   = #load("../../assets/fonts/RobotoMono-Medium.ttf"),
	.Roboto_Mono_SemiBold = #load("../../assets/fonts/RobotoMono-SemiBold.ttf"),
	.Roboto_Mono_Symbols  = #load("../../assets/fonts/RobotoMono-Symbols-Medium.ttf"),
	.STIX_SemiBold        = #load("../../assets/fonts/STIXTwoText-SemiBold.ttf"),
}

// A face: what a family, weight and style in CSS pick on the page (the M4a census: every face
// the reference draws text in). Work Sans is upright only, STIX Two Text italic only at 400.
Face :: enum u8 {
	Work_Sans_400,
	Work_Sans_500,
	Work_Sans_600,
	Work_Sans_700,
	Work_Sans_800,
	STIX_400,
	STIX_500,
	STIX_400_Italic,
	Roboto_Mono_500,
	Roboto_Mono_600,
	STIX_600, // a term's tip's name (TermTip.module.css .name, in the equation's STIX)
}

// The file a face draws from, and the symbols file behind it for what it lacks, as the page's
// @font-face rules add one with `unicode-range: U+2248` (src/style/global.css). The STIX Two
// Math file serves every STIX weight and style, the Roboto Mono symbols at 500 both weights.
Face_Files :: struct {
	font:    Font_File,
	symbols: Maybe(Font_File),
}

@(rodata)
FACES := [Face]Face_Files {
	.Work_Sans_400   = {.Work_Sans_Regular, nil},
	.Work_Sans_500   = {.Work_Sans_Medium, nil},
	.Work_Sans_600   = {.Work_Sans_SemiBold, nil},
	.Work_Sans_700   = {.Work_Sans_Bold, nil},
	.Work_Sans_800   = {.Work_Sans_ExtraBold, nil},
	.STIX_400        = {.STIX_Regular, .STIX_Math_Symbols},
	.STIX_500        = {.STIX_Medium, .STIX_Math_Symbols},
	.STIX_400_Italic = {.STIX_Italic, .STIX_Math_Symbols},
	.Roboto_Mono_500 = {.Roboto_Mono_Medium, .Roboto_Mono_Symbols},
	.Roboto_Mono_600 = {.Roboto_Mono_SemiBold, .Roboto_Mono_Symbols},
	.STIX_600        = {.STIX_SemiBold, .STIX_Math_Symbols},
}

// A face's vertical metrics in em: the ascent and descent above and below the baseline and the
// line gap, from the font's hhea table, as browsers on macOS read them (the faces' OS/2 typo
// metrics are the same numbers; CartesianPlane/faces.ts writes STIX's and Roboto Mono's).
Metrics :: struct {
	ascent:   f32,
	descent:  f32,
	line_gap: f32,
}

// The faces, loaded once: kb_text_shape's fonts and context, each file's metrics and space
// advance, and NanoVG's handles for the same bytes. It must not move after init (kb_text_shape's
// allocator keeps a pointer to `allocator`).
Fonts :: struct {
	allocator:    runtime.Allocator,
	ctx:          ^kbts.shape_context,
	kb:           [Font_File]kbts.font,
	units_per_em: [Font_File]f32,
	metrics:      [Font_File]Metrics,
	// The advance of the space glyph alone, in font units: the space a measurer adds between
	// words (measure_word).
	space:        [Font_File]i32,
	vg:           [Font_File]int, // NanoVG's handles (load_into)
	// The last text Clay asked about word by word (measure_word), shaped whole.
	words:        struct {
		style:  Style,
		text:   string, // a copy, in `allocator`
		shaped: Shaped, // its glyphs in `allocator`
	},
}

init :: proc(fonts: ^Fonts) -> bool {
	fonts.allocator = context.allocator
	alloc, data := kbts.AllocatorFromOdinAllocator(&fonts.allocator)
	fonts.ctx = kbts.CreateShapeContext(alloc, data)
	if fonts.ctx == nil {
		log.error("kb_text_shape: no shape context")
		return false
	}
	for file in Font_File {
		fonts.kb[file] = kbts.FontFromMemory(FONT_DATA[file], 0, alloc, data)
		if !kbts.FontIsValid(&fonts.kb[file]) {
			log.errorf("kb_text_shape: %v is not a font it can read", file)
			return false
		}
		info := kbts.font_info2_1 {
			Size = size_of(kbts.font_info2_1),
		}
		kbts.GetFontInfo2(&fonts.kb[file], &info.FontInfo2)
		em := f32(info.UnitsPerEm)
		fonts.units_per_em[file] = em
		fonts.metrics[file] = {f32(info.Ascent) / em, -f32(info.Descent) / em, f32(info.LineGap) / em}
		fonts.space[file] = space_advance(fonts, file)
	}
	return true
}

destroy :: proc(fonts: ^Fonts) {
	forget_words(fonts)
	for &font in fonts.kb {
		kbts.FreeFont(&font)
	}
	kbts.DestroyShapeContext(fonts.ctx)
	fonts.ctx = nil
}

// The faces into NanoVG, which draws the shaped glyphs from the same bytes kb_text_shape read.
load_into :: proc(fonts: ^Fonts, vg: ^nvg.Context) {
	for file in Font_File {
		name := FONT_NAMES[file]
		fonts.vg[file] = nvg.CreateFontMem(vg, name, FONT_DATA[file], false)
	}
}

@(rodata, private = "file")
FONT_NAMES := [Font_File]string {
	.Work_Sans_Regular    = "Work Sans 400",
	.Work_Sans_Medium     = "Work Sans 500",
	.Work_Sans_SemiBold   = "Work Sans 600",
	.Work_Sans_Bold       = "Work Sans 700",
	.Work_Sans_ExtraBold  = "Work Sans 800",
	.STIX_Regular         = "STIX Two Text 400",
	.STIX_Medium          = "STIX Two Text 500",
	.STIX_Italic          = "STIX Two Text 400 italic",
	.STIX_Math_Symbols    = "STIX Two Math symbols",
	.Roboto_Mono_Medium   = "Roboto Mono 500",
	.Roboto_Mono_SemiBold = "Roboto Mono 600",
	.Roboto_Mono_Symbols  = "Roboto Mono symbols 500",
	.STIX_SemiBold        = "STIX Two Text 600",
}

// A face's metrics: its own file's.
metrics :: proc(fonts: ^Fonts, face: Face) -> Metrics {
	return fonts.metrics[FACES[face].font]
}

// The space glyph's advance in `file`, in font units: a space shaped alone.
@(private = "file")
space_advance :: proc(fonts: ^Fonts, file: Font_File) -> (advance: i32) {
	_ = kbts.ShapePushFont(fonts.ctx, &fonts.kb[file])
	defer _ = kbts.ShapePopFont(fonts.ctx)
	kbts.ShapeBegin(fonts.ctx, .KBTS_DIRECTION_LTR, .DONT_KNOW)
	kbts.ShapeUtf8(fonts.ctx, " ", .SOURCE_INDEX)
	kbts.ShapeEnd(fonts.ctx)
	for {
		run := kbts.ShapeRun(fonts.ctx) or_break
		for {
			glyph := kbts.GlyphIteratorNext(&run.Glyphs) or_break
			advance += glyph.AdvanceX
		}
	}
	return
}
