//! The page's static text faces: the embedded TTFs, the shaping contexts and
//! fonts, the per-face metrics, and the one-word measure cache.

use enum_map::{Enum, EnumMap};

use super::shape::{Shaped, Style, forget_words};

/// One embedded font file, as the shaping contexts keep it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum FontFile {
    WorkSansRegular,
    WorkSansMedium,
    WorkSansSemiBold,
    WorkSansBold,
    WorkSansExtraBold,
    StixRegular,
    StixMedium,
    StixItalic,
    StixMathSymbols,
    RobotoMonoMedium,
    RobotoMonoSemiBold,
    RobotoMonoSymbols,
    StixSemiBold,
}

/// The embedded bytes of every font file, in face order.
pub static FONT_DATA: EnumMap<FontFile, &'static [u8]> = EnumMap::from_array([
    include_bytes!("../../../assets/fonts/WorkSans-Regular.ttf") as &[u8],
    include_bytes!("../../../assets/fonts/WorkSans-Medium.ttf") as &[u8],
    include_bytes!("../../../assets/fonts/WorkSans-SemiBold.ttf") as &[u8],
    include_bytes!("../../../assets/fonts/WorkSans-Bold.ttf") as &[u8],
    include_bytes!("../../../assets/fonts/WorkSans-ExtraBold.ttf") as &[u8],
    include_bytes!("../../../assets/fonts/STIXTwoText-Regular.ttf") as &[u8],
    include_bytes!("../../../assets/fonts/STIXTwoText-Medium.ttf") as &[u8],
    include_bytes!("../../../assets/fonts/STIXTwoText-Italic.ttf") as &[u8],
    include_bytes!("../../../assets/fonts/STIXTwoMath-Symbols.ttf") as &[u8],
    include_bytes!("../../../assets/fonts/RobotoMono-Medium.ttf") as &[u8],
    include_bytes!("../../../assets/fonts/RobotoMono-SemiBold.ttf") as &[u8],
    include_bytes!("../../../assets/fonts/RobotoMono-Symbols-Medium.ttf") as &[u8],
    include_bytes!("../../../assets/fonts/STIXTwoText-SemiBold.ttf") as &[u8],
]);

/// A face the page sets: a main file with its symbols fallback.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum Face {
    WorkSans400,
    WorkSans500,
    WorkSans600,
    WorkSans700,
    WorkSans800,
    Stix400,
    Stix500,
    Stix400Italic,
    RobotoMono500,
    RobotoMono600,
    Stix600,
}

/// The files behind a face: its font file and its symbols file, if any.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FaceFiles {
    pub font: FontFile,
    pub symbols: Option<FontFile>,
}

/// The files behind every face.
pub static FACES: EnumMap<Face, FaceFiles> = EnumMap::from_array([
    FaceFiles {
        font: FontFile::WorkSansRegular,
        symbols: None,
    },
    FaceFiles {
        font: FontFile::WorkSansMedium,
        symbols: None,
    },
    FaceFiles {
        font: FontFile::WorkSansSemiBold,
        symbols: None,
    },
    FaceFiles {
        font: FontFile::WorkSansBold,
        symbols: None,
    },
    FaceFiles {
        font: FontFile::WorkSansExtraBold,
        symbols: None,
    },
    FaceFiles {
        font: FontFile::StixRegular,
        symbols: Some(FontFile::StixMathSymbols),
    },
    FaceFiles {
        font: FontFile::StixMedium,
        symbols: Some(FontFile::StixMathSymbols),
    },
    FaceFiles {
        font: FontFile::StixItalic,
        symbols: Some(FontFile::StixMathSymbols),
    },
    FaceFiles {
        font: FontFile::RobotoMonoMedium,
        symbols: Some(FontFile::RobotoMonoSymbols),
    },
    FaceFiles {
        font: FontFile::RobotoMonoSemiBold,
        symbols: Some(FontFile::RobotoMonoSymbols),
    },
    FaceFiles {
        font: FontFile::StixSemiBold,
        symbols: Some(FontFile::StixMathSymbols),
    },
]);

/// Ascent, descent and line gap of a face at one em, in ems.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Metrics {
    pub ascent: f32,
    pub descent: f32,
    pub line_gap: f32,
}

/// The single cached word the layout measures against.
#[derive(Clone, Debug, PartialEq)]
pub struct WordsCache {
    pub style: Style,
    pub text: String,
    pub shaped: Shaped,
}

/// Everything shaping needs: the shaping context, the loaded fonts, their
/// metrics, and the one-word measure cache.
///
/// `ctx` is `None` until [`init`] creates it; the safe context owns the C
/// context (this crate is `forbid(unsafe_code)`, so the raw pointer the Odin
/// `Fonts` keeps cannot be touched here).
pub struct Fonts {
    pub ctx: Option<mv_sys::kb_safe::ShapeContext>,
    pub kb: EnumMap<FontFile, mv_sys::kb::font>,
    pub units_per_em: EnumMap<FontFile, f32>,
    pub metrics: EnumMap<FontFile, Metrics>,
    pub space: EnumMap<FontFile, i32>,
    pub vg: EnumMap<FontFile, i32>,
    pub words: WordsCache,
}

/// The display name of every font file, for diagnostics.
static FONT_NAMES: EnumMap<FontFile, &'static str> = EnumMap::from_array([
    "Work Sans 400",
    "Work Sans 500",
    "Work Sans 600",
    "Work Sans 700",
    "Work Sans 800",
    "STIX Two Text 400",
    "STIX Two Text 500",
    "STIX Two Text 400 italic",
    "STIX Two Math symbols",
    "Roboto Mono 500",
    "Roboto Mono 600",
    "Roboto Mono symbols 500",
    "STIX Two Text 600",
]);

/// Builds the shaping context and loads every embedded font.
///
/// Ported from `fonts.odin:init`: the context through
/// `ShapeContext::new`, each file through `font_from_memory` with the
/// validity check, the metrics from `font_info` in ems, and the space
/// advance shaped alone. On a load failure returns false after dropping the
/// context, as the spec returns false without freeing (the embedded fonts
/// always load; the branch is defensive).
pub fn init(fonts: &mut Fonts) -> bool {
    use mv_sys::kb_safe::{ShapeContext, font_from_memory, font_info, font_is_valid};

    fonts.ctx = ShapeContext::new();
    if fonts.ctx.is_none() {
        return false;
    }
    for (file, data) in &FONT_DATA {
        fonts.kb[file] = font_from_memory(data, 0);
        if !font_is_valid(&mut fonts.kb[file]) {
            fonts.ctx = None;
            return false;
        }
        let info = font_info(&mut fonts.kb[file]);
        let em = info.UnitsPerEm as f32;
        fonts.units_per_em[file] = em;
        fonts.metrics[file] = Metrics {
            ascent: info.Ascent as f32 / em,
            descent: -(info.Descent as f32) / em,
            line_gap: info.LineGap as f32 / em,
        };
        fonts.space[file] = space_advance(fonts, file);
    }
    true
}

/// Frees the shaping context.
///
/// Ported from `fonts.odin:destroy`: the cached word goes first, then each
/// loaded font, then the context (dropping the `None` assignment destroys
/// it, as `DestroyShapeContext` does).
pub fn destroy(fonts: &mut Fonts) {
    use mv_sys::kb_safe::free_font;

    forget_words(fonts);
    for (_, font) in &mut fonts.kb {
        free_font(font);
    }
    fonts.ctx = None;
}

/// Creates the NanoVG handles for the loaded fonts.
pub fn load_into(fonts: &mut Fonts, vg: &mut mv_nvg::Context) {
    for (file, data) in &FONT_DATA {
        fonts.vg[file] = vg.create_font_mem(FONT_NAMES[file], data, false);
    }
}

/// The metrics of a face.
pub fn metrics(fonts: &mut Fonts, face: Face) -> Metrics {
    fonts.metrics[FACES[face].font]
}

/// The advance of a space glyph in a font file, in font units: a space
/// shaped alone.
///
/// Ported from `fonts.odin:space_advance`: the file's font pushed,
/// left-to-right shaping begun, one space shaped with source-index ids,
/// every run's advances summed, the font popped.
fn space_advance(fonts: &mut Fonts, file: FontFile) -> i32 {
    use mv_sys::kb::{direction, language, user_id_generation_mode};

    let ctx = fonts
        .ctx
        .as_mut()
        .expect("space_advance runs during init, after the context is created");
    ctx.push_font(&mut fonts.kb[file]);
    ctx.begin(direction::KBTS_DIRECTION_LTR, language::DONT_KNOW);
    ctx.shape_utf8(" ", user_id_generation_mode::SOURCE_INDEX);
    ctx.end();
    let mut advance = 0;
    while let Some(mut run) = ctx.shape_run() {
        while let Some(g) = run.glyph_next() {
            advance += g.advance_x;
        }
    }
    ctx.pop_font();
    advance
}

/// A `Fonts` before [`init`]: no context, zeroed fonts and metrics, an
/// empty word cache. What the Odin tests' `new(Fonts)` is; tests call
/// [`init`] on it.
pub fn uninit() -> Fonts {
    Fonts {
        ctx: None,
        kb: enum_map::EnumMap::from_array(
            [mv_sys::kb::font {
                Allocator: None,
                AllocatorData: core::ptr::null_mut(),
                Blob: core::ptr::null_mut(),
                Cmap: core::ptr::null_mut(),
                Cmap14: core::ptr::null_mut(),
                ShapingTables: [core::ptr::null_mut(), core::ptr::null_mut()],
                UserData: core::ptr::null_mut(),
                Error: mv_sys::kb::load_font_error::NONE,
            }; 13],
        ),
        units_per_em: enum_map::EnumMap::from_array([0.0; 13]),
        metrics: enum_map::EnumMap::from_array(
            [Metrics {
                ascent: 0.0,
                descent: 0.0,
                line_gap: 0.0,
            }; 13],
        ),
        space: enum_map::EnumMap::from_array([0; 13]),
        vg: enum_map::EnumMap::from_array([0; 13]),
        words: WordsCache {
            style: Style {
                face: Face::WorkSans400,
                size: 0.0,
                tracking: 0.0,
                features: enumset::EnumSet::new(),
            },
            text: String::new(),
            shaped: Shaped {
                glyphs: Vec::new(),
                advance: 0.0,
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_are_the_fonts() {
        let mut fonts = uninit();
        assert!(init(&mut fonts));
        assert_eq!(
            metrics(&mut fonts, Face::WorkSans600),
            Metrics {
                ascent: 0.93,
                descent: 0.243,
                line_gap: 0.0
            }
        );
        assert_eq!(
            metrics(&mut fonts, Face::Stix400Italic),
            Metrics {
                ascent: 0.762,
                descent: 0.238,
                line_gap: 0.25
            }
        );
        assert_eq!(
            metrics(&mut fonts, Face::RobotoMono500),
            Metrics {
                ascent: 2146.0 / 2048.0,
                descent: 555.0 / 2048.0,
                line_gap: 0.0
            }
        );
        destroy(&mut fonts);
    }
}
