//! Runs shaped through the page's fonts: kerning, tabular figures, the space
//! rules, and the one-word measure the layout shares.

use enum_map::{Enum, EnumMap};
use enumset::{EnumSet, EnumSetType};

use super::fonts::{FACES, Face, FontFile, Fonts};

/// What a rule of the page sets on a run of text: the face, the em size, the
/// tracking after every character, and the features it turns on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Style {
    pub face: Face,
    pub size: f32,
    pub tracking: f32,
    pub features: Features,
}

/// Departures from a run's defaults: kerning off, or tabular figures.
#[derive(Debug, Enum, EnumSetType)]
pub enum Feature {
    NoKerning,
    Tabular,
}

/// The features a run sets, as a set.
pub type Features = EnumSet<Feature>;

/// One shaped glyph: its font's glyph id, its pen offset and advance, and the
/// source byte it came from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Glyph {
    pub id: u16,
    pub font: FontFile,
    pub x: f32,
    pub y: f32,
    pub advance: f32,
    pub source: i32,
}

/// A shaped run: its glyphs and their total advance.
#[derive(Clone, Debug, PartialEq)]
pub struct Shaped {
    pub glyphs: Vec<Glyph>,
    pub advance: f32,
}

/// Shapes text in a style, tracking after every character, the last included.
///
/// Ported from `shape.odin:shape`: the empty-text guard (kb_text_shape
/// writes through a null pointer on empty text); the symbols file pushed
/// below the face's file, so the shaper tries the face first; left-to-right
/// shaping with `kern` off, `tnum` on and `liga`/`clig` off where the style
/// asks (spaced letters take no ligature, CSS Text 3 § 8.2); the text shaped
/// with source-index user ids; each run's glyphs read back with their
/// codepoint sources, scaled to the em size with tracking on every
/// unattached glyph, marks taking no spacing of their own. The feature pops
/// run in reverse push order over the pushed tags, as the spec's overrides
/// array does.
pub fn shape(fonts: &mut Fonts, style: Style, text: &str) -> Shaped {
    use mv_sys::kb::{direction, feature_tag, language, user_id_generation_mode};

    if text.is_empty() {
        return Shaped {
            glyphs: Vec::new(),
            advance: 0.0,
        };
    }
    let files = FACES[style.face];
    let ctx = fonts
        .ctx
        .as_mut()
        .expect("shape needs init: no shaping context");
    // kb_text_shape tries the font on top of its stack first: the symbols
    // file goes in below.
    if let Some(symbols) = files.symbols {
        ctx.push_font(&mut fonts.kb[symbols]);
    }
    ctx.push_font(&mut fonts.kb[files.font]);

    // The page is left to right (no dir attribute): with the paragraph's
    // direction given, a space that starts the text joins the run that
    // follows it rather than making one of its own.
    ctx.begin(direction::KBTS_DIRECTION_LTR, language::DONT_KNOW);
    let mut overrides = [feature_tag::kern; 4];
    let mut count = 0;
    if style.features.contains(Feature::NoKerning) {
        overrides[count] = feature_tag::kern;
        count += 1;
        ctx.push_feature(feature_tag::kern, 0);
    }
    if style.features.contains(Feature::Tabular) {
        overrides[count] = feature_tag::tnum;
        count += 1;
        ctx.push_feature(feature_tag::tnum, 1);
    }
    if style.tracking != 0.0 {
        overrides[count] = feature_tag::liga;
        overrides[count + 1] = feature_tag::clig;
        count += 2;
        ctx.push_feature(feature_tag::liga, 0);
        ctx.push_feature(feature_tag::clig, 0);
    }
    ctx.shape_utf8(text, user_id_generation_mode::SOURCE_INDEX);
    for i in (0..count).rev() {
        ctx.pop_feature(overrides[i]);
    }
    ctx.end();

    let mut glyphs: Vec<Glyph> = Vec::with_capacity(text.len());
    let mut pen = 0.0f32;
    while let Some(mut run) = ctx.shape_run() {
        let file = file_of(&fonts.kb, run.font());
        let scale = style.size / fonts.units_per_em[file];
        while let Some(g) = run.glyph_next() {
            let (source, _) = ctx.shape_codepoint(g.user_id_or_codepoint_index);
            let mut advance = g.advance_x as f32 * scale;
            // A mark attached to its base takes no spacing of its own: the
            // spacing follows the character it belongs to.
            if !g.attached {
                advance += style.tracking;
            }
            glyphs.push(Glyph {
                id: g.id,
                font: file,
                x: pen + g.offset_x as f32 * scale,
                y: -(g.offset_y as f32) * scale, // font units are y up
                advance,
                source: source.UserId,
            });
            pen += advance;
        }
    }
    ctx.pop_font();
    if files.symbols.is_some() {
        ctx.pop_font();
    }
    Shaped {
        glyphs,
        advance: pen,
    }
}

/// The advance of text in a style.
pub fn measure(fonts: &mut Fonts, style: Style, text: &str) -> f32 {
    shape(fonts, style, text).advance
}

/// The share of a word's advance Clay charges to a slice of it, kerning into
/// and out of the surrounding spaces kept.
pub fn measure_word(fonts: &mut Fonts, style: Style, whole: &str, start: i64, end: i64) -> f32 {
    if fonts.words.style != style || fonts.words.text != whole {
        forget_words(fonts);
        fonts.words.style = style;
        fonts.words.text = whole.to_owned();
        fonts.words.shaped = shape(fonts, style, whole);
    }
    let mut share = 0.0;
    for glyph in &fonts.words.shaped.glyphs {
        let source = glyph.source as i64;
        if source >= start && source < end {
            share += glyph.advance;
        } else if source == start - 1 && start > 0 && whole.as_bytes()[(start - 1) as usize] == b' '
        {
            let alone = fonts.space[glyph.font] as f32 * style.size
                / fonts.units_per_em[glyph.font]
                + style.tracking;
            share += glyph.advance - alone;
        }
    }
    share
}

/// Drops the cached word, so the next measure shapes again.
pub fn forget_words(fonts: &mut Fonts) {
    fonts.words.text = String::new();
    fonts.words.shaped = Shaped {
        glyphs: Vec::new(),
        advance: 0.0,
    };
}

/// Which embedded file a shaping font pointer came from.
///
/// Takes the loaded fonts rather than the whole `Fonts`: `shape` holds the
/// shaping context borrowed while it looks each run's font up.
fn file_of(kb: &EnumMap<FontFile, mv_sys::kb::font>, font: *const mv_sys::kb::font) -> FontFile {
    for (file, loaded) in kb {
        if core::ptr::eq(loaded, font) {
            return file;
        }
    }
    panic!("kb_text_shape shaped a run in a font it wasn't given")
}

#[cfg(test)]
pub(crate) mod fixture {
    use std::collections::HashMap;

    use serde::Deserialize;

    use super::*;
    use crate::fonts::Face;

    pub(crate) const FIXTURE: &str = include_str!("../testdata/widths.json");

    pub(crate) const TOLERANCE: f32 = 1.0;

    #[derive(Clone, Debug, Deserialize)]
    #[allow(dead_code)] // mirrors the fixture's shape; the tests read width and slot
    pub(crate) struct Measured {
        #[serde(default)]
        pub width: f32,
        #[serde(default)]
        pub height: f32,
        #[serde(default)]
        pub baseline: f32,
        #[serde(default)]
        pub content: [f32; 2],
        #[serde(default)]
        pub slot: f32,
    }

    #[derive(Clone, Debug, Deserialize)]
    #[allow(dead_code)] // mirrors the fixture's shape; `ch` marks check rows the tests skip
    pub(crate) struct Sample {
        #[serde(default)]
        pub name: String,
        #[serde(default)]
        pub family: String,
        #[serde(default)]
        pub weight: i64,
        #[serde(default)]
        pub italic: bool,
        #[serde(default)]
        pub size: f32,
        #[serde(default)]
        pub tracking: f32,
        #[serde(default)]
        pub tabular: bool,
        #[serde(default)]
        pub kerning: bool,
        #[serde(default)]
        pub line_height: Option<f32>,
        #[serde(default)]
        pub text: String,
        #[serde(default)]
        pub check: bool,
        #[serde(default)]
        pub ch: bool,
        #[serde(default)]
        pub width: f32,
        #[serde(default)]
        pub height: f32,
        #[serde(default)]
        pub baseline: f32,
        #[serde(default)]
        pub slot: f32,
        #[serde(default)]
        pub engines: HashMap<String, Measured>,
    }

    #[derive(Clone, Debug, Deserialize)]
    #[allow(dead_code)] // mirrors the fixture's shape; the tests read samples
    pub(crate) struct Fixture {
        #[serde(default)]
        pub engine: String,
        #[serde(default)]
        pub samples: Vec<Sample>,
    }

    pub(crate) fn load_fixture() -> Fixture {
        serde_json::from_str(FIXTURE).expect("testdata/widths.json parses")
    }

    pub(crate) fn face_of(s: &Sample) -> Option<Face> {
        match s.family.as_str() {
            "Work Sans" => {
                if s.italic {
                    return None;
                }
                match s.weight {
                    400 => Some(Face::WorkSans400),
                    500 => Some(Face::WorkSans500),
                    600 => Some(Face::WorkSans600),
                    700 => Some(Face::WorkSans700),
                    800 => Some(Face::WorkSans800),
                    _ => None,
                }
            }
            "STIX Two Text" => match (s.italic, s.weight) {
                (true, 400) => Some(Face::Stix400Italic),
                (false, 400) => Some(Face::Stix400),
                (false, 500) => Some(Face::Stix500),
                _ => None,
            },
            "Roboto Mono" => match s.weight {
                500 => Some(Face::RobotoMono500),
                600 => Some(Face::RobotoMono600),
                _ => None,
            },
            _ => None,
        }
    }

    pub(crate) fn style_of(s: &Sample) -> Option<Style> {
        let face = face_of(s)?;
        let mut features = Features::new();
        if s.tabular {
            features |= Feature::Tabular;
        }
        if !s.kerning {
            features |= Feature::NoKerning;
        }
        Some(Style {
            face,
            size: s.size,
            tracking: s.tracking,
            features,
        })
    }

    pub(crate) fn find<'a>(fixture: &'a Fixture, name: &str) -> &'a Sample {
        fixture
            .samples
            .iter()
            .find(|s| s.name == name)
            .unwrap_or_else(|| panic!("no sample {name:?} in testdata/widths.json"))
    }

    pub(crate) fn px(v: f32, signed: bool) -> String {
        if signed {
            format!("{v:+.3}")
        } else {
            format!("{v:.3}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fixture::*;
    use super::*;
    use crate::fonts::{FACES, Face, Fonts, destroy, init, uninit};

    fn units(fonts: &mut Fonts, face: Face, text: &str, features: Features) -> f32 {
        let style = Style {
            face,
            size: fonts.units_per_em[FACES[face].font],
            tracking: 0.0,
            features,
        };
        measure(fonts, style, text)
    }

    #[test]
    fn kerning_is_the_fonts() {
        let mut fonts = uninit();
        assert!(init(&mut fonts));
        let pairs = [
            ("To", -99.0),
            ("Va", -60.0),
            ("AV", -68.0),
            ("Ty", -67.0),
            ("a ", -14.0),
            ("x V", -51.0),
            (" V", -31.0),
            ("67", -18.0),
        ];
        for (text, kern) in pairs {
            let kerned = units(&mut fonts, Face::WorkSans600, text, Features::new());
            let plain = units(
                &mut fonts,
                Face::WorkSans600,
                text,
                Feature::NoKerning.into(),
            );
            assert!(
                kerned - plain == kern,
                "{text:?} kerned {kerned}, unkerned {plain}: the page kerns it by {kern}"
            );
        }
        assert_eq!(
            units(
                &mut fonts,
                Face::WorkSans600,
                "To Va AV Ty",
                Feature::NoKerning.into()
            ),
            5918.0
        );
        assert_eq!(
            units(
                &mut fonts,
                Face::WorkSans600,
                "To Va AV Ty",
                Features::new()
            ),
            5498.0
        );
        assert_eq!(
            units(&mut fonts, Face::WorkSans600, " V", Features::new()),
            306.0 + 670.0 - 31.0
        );
        destroy(&mut fonts);
    }

    #[test]
    fn tabular_figures_share_one_advance() {
        let mut fonts = uninit();
        assert!(init(&mut fonts));
        let digits = "0123456789";
        for d in 0..10 {
            let digit = &digits[d..d + 1];
            assert_eq!(
                units(
                    &mut fonts,
                    Face::WorkSans600,
                    digit,
                    Feature::Tabular.into()
                ),
                604.0
            );
            assert_eq!(
                units(&mut fonts, Face::Stix400, digit, Features::new()),
                495.0
            );
            assert_eq!(
                units(&mut fonts, Face::Stix500, digit, Feature::Tabular.into()),
                if digit == "7" { 494.0 } else { 495.0 }
            );
            assert_eq!(
                units(
                    &mut fonts,
                    Face::RobotoMono500,
                    digit,
                    Feature::Tabular.into()
                ),
                1229.0
            );
        }
        assert_eq!(
            units(&mut fonts, Face::WorkSans600, "0", Features::new()),
            636.0
        );
        assert_eq!(
            units(&mut fonts, Face::WorkSans600, "1", Features::new()),
            416.0
        );
        assert_eq!(
            units(
                &mut fonts,
                Face::WorkSans600,
                digits,
                Feature::Tabular.into()
            ),
            6040.0
        );
        assert_eq!(
            units(&mut fonts, Face::WorkSans600, digits, Features::new()),
            5853.0
        );
        destroy(&mut fonts);
    }

    #[test]
    fn symbols_fall_back() {
        let mut fonts = uninit();
        assert!(init(&mut fonts));
        let run = shape(
            &mut fonts,
            Style {
                face: Face::RobotoMono500,
                size: 14.0,
                tracking: 0.0,
                features: Features::new(),
            },
            "≈0",
        );
        assert_eq!(run.glyphs.len(), 2);
        assert_eq!(run.glyphs[0].font, FontFile::RobotoMonoSymbols);
        assert_eq!(run.glyphs[1].font, FontFile::RobotoMonoMedium);
        assert_eq!(run.glyphs[1].source, 3);
        assert_eq!(
            units(&mut fonts, Face::Stix400, "≈", Features::new()),
            720.0
        );
        assert_eq!(
            units(&mut fonts, Face::RobotoMono500, "≈", Features::new()),
            1229.0
        );
        destroy(&mut fonts);
    }

    #[test]
    fn tracking_follows_every_character() {
        let mut fonts = uninit();
        assert!(init(&mut fonts));
        let caps = "FUNCTIONS · TRANSFORMATIONS";
        let plain = measure(
            &mut fonts,
            Style {
                face: Face::WorkSans600,
                size: 12.0,
                tracking: 0.0,
                features: Features::new(),
            },
            caps,
        );
        let tracked = measure(
            &mut fonts,
            Style {
                face: Face::WorkSans600,
                size: 12.0,
                tracking: 2.8,
                features: Features::new(),
            },
            caps,
        );
        assert!(
            (tracked - plain - 27.0 * 2.8).abs() < 1e-3,
            "tracked {tracked}, plain {plain}"
        );
        destroy(&mut fonts);
    }

    #[test]
    fn words_add_up_to_the_whole() {
        let mut fonts = uninit();
        assert!(init(&mut fonts));
        let samples = [
            (
                Style {
                    face: Face::WorkSans700,
                    size: 20.0,
                    tracking: 0.0,
                    features: Features::new(),
                },
                "Function Viewer",
            ),
            (
                Style {
                    face: Face::WorkSans600,
                    size: 12.0,
                    tracking: 2.8,
                    features: Features::new(),
                },
                "FUNCTIONS · TRANSFORMATIONS",
            ),
            (
                Style {
                    face: Face::WorkSans600,
                    size: 24.0,
                    tracking: 0.0,
                    features: Features::new(),
                },
                "To Va AV Ty",
            ),
            (
                Style {
                    face: Face::Stix400,
                    size: 20.0,
                    tracking: 0.0,
                    features: Features::new(),
                },
                ") = ≈ 2",
            ),
        ];
        for (style, text) in samples {
            let space = measure(&mut fonts, style, " ");
            let mut sum = 0.0;
            let mut start: usize = 0;
            while start <= text.len() {
                let rest = &text[start..];
                let end = match rest.find(' ') {
                    Some(i) => start + i,
                    None => text.len(),
                };
                sum += measure_word(&mut fonts, style, text, start as i64, end as i64);
                if end < text.len() {
                    sum += space;
                }
                start = end + 1;
            }
            let whole = measure(&mut fonts, style, text);
            assert!(
                (sum - whole).abs() < 1e-3,
                "{text:?}: the words add up to {sum}, the whole is {whole}"
            );
        }
        destroy(&mut fonts);
    }

    #[test]
    fn empty_text_shapes_to_nothing() {
        let mut fonts = uninit();
        assert!(init(&mut fonts));
        let run = shape(
            &mut fonts,
            Style {
                face: Face::RobotoMono500,
                size: 17.0,
                tracking: 0.0,
                features: Features::new(),
            },
            "",
        );
        assert_eq!(run.glyphs.len(), 0);
        assert_eq!(run.advance, 0.0);
        assert_eq!(
            units(&mut fonts, Face::WorkSans600, "", Features::new()),
            0.0
        );
        assert_eq!(
            units(&mut fonts, Face::WorkSans600, "To", Features::new()),
            635.0 + 599.0 - 99.0
        );
        destroy(&mut fonts);
    }

    #[test]
    fn widths_are_the_references() {
        let fixture = load_fixture();
        let mut fonts = uninit();
        assert!(init(&mut fonts));

        let mut strings_count = 0;
        let mut within = [0; 3];
        let mut worst: [f32; 2] = [0.0; 2];
        let mut worst_name = [String::new(), String::new()];
        let mut table = String::new();
        table.push_str(&format!(
            "{:<16} {:>9} {:>9} {:>9} {:>9} {:>8} {:>8}\n",
            "sample", "port", "webkit", "chromium", "firefox", "-webkit", "-chrom."
        ));
        for s in &fixture.samples {
            if s.check {
                continue;
            }
            strings_count += 1;
            let Some(style) = style_of(s) else { continue };
            let port = measure(&mut fonts, style, &s.text);
            let chromium = s.engines["chromium"].width;
            let off = [port - s.width, port - chromium];
            assert!(
                off[0].abs() <= TOLERANCE && off[1].abs() <= TOLERANCE,
                "{} {:?}: the port's advance {:.3} px, WebKit's {:.3} ({:+.3}), Chromium's {:.3} ({:+.3}); Firefox {:.3}",
                s.name,
                s.text,
                port,
                s.width,
                off[0],
                chromium,
                off[1],
                s.engines["firefox"].width
            );
            let nearer = off[0].abs().min(off[1].abs());
            for (limit, count) in [0.1, 0.25, 0.5].iter().zip(within.iter_mut()) {
                if nearer <= *limit {
                    *count += 1;
                }
            }
            for engine in 0..2 {
                if off[engine].abs() > worst[engine].abs() {
                    worst[engine] = off[engine];
                    worst_name[engine] = s.name.clone();
                }
            }
            table.push_str(&format!(
                "{:<16} {:>9} {:>9} {:>9} {:>9} {:>8} {:>8}\n",
                s.name,
                px(port, false),
                px(s.width, false),
                px(chromium, false),
                px(s.engines["firefox"].width, false),
                px(off[0], true),
                px(off[1], true)
            ));
        }
        assert_eq!(strings_count, 30);
        println!(
            "{} strings: every one within {} px of WebKit's advance and of Chromium's; within 0.1 px of the nearer {}, 0.25 px {}, 0.5 px {}; the furthest from WebKit {}, {:+.3} px, from Chromium {}, {:+.3} px\n{}",
            strings_count,
            TOLERANCE,
            within[0],
            within[1],
            within[2],
            worst_name[0],
            worst[0],
            worst_name[1],
            worst[1],
            table
        );
        destroy(&mut fonts);
    }

    #[test]
    fn kerning_is_the_references() {
        let fixture = load_fixture();
        let mut fonts = uninit();
        assert!(init(&mut fonts));
        let kerned = find(&fixture, "name-a");
        let plain = find(&fixture, "kern-off");
        let Some(style) = style_of(kerned) else {
            destroy(&mut fonts);
            return;
        };
        let unkerned = Style {
            features: style.features | Feature::NoKerning,
            ..style
        };
        let port =
            measure(&mut fonts, style, &kerned.text) - measure(&mut fonts, unkerned, &plain.text);
        let reference = kerned.width - plain.width;
        assert!(
            port < -1.0 && (port - reference).abs() <= 0.05,
            "{:?}: kerning takes {:.3} px off in the port, {:.3} in WebKit",
            kerned.text,
            port,
            reference
        );
        destroy(&mut fonts);
    }

    #[test]
    fn tabular_figures_are_the_references() {
        let fixture = load_fixture();
        let mut fonts = uninit();
        assert!(init(&mut fonts));
        for prefix in ["tnum-mono", "tnum-stix", "tnum-ws", "pnum-ws"] {
            let zeros = find(&fixture, &format!("{prefix}-1.00"));
            let ones = find(&fixture, &format!("{prefix}-1.11"));
            let Some(style) = style_of(zeros) else {
                continue;
            };
            let (a, b) = (
                measure(&mut fonts, style, &zeros.text),
                measure(&mut fonts, style, &ones.text),
            );
            assert!(
                (a - zeros.width).abs() <= 0.05 && (b - ones.width).abs() <= 0.05,
                "{prefix}: 1.00 {a:.3} and 1.11 {b:.3} px, WebKit {:.3} and {:.3}",
                zeros.width,
                ones.width
            );
            if prefix == "pnum-ws" {
                assert!(
                    b < a - 5.0,
                    "Work Sans's proportional 1.11 ({b:.3}) as wide as 1.00 ({a:.3})"
                );
            } else {
                assert_eq!(a, b, "{prefix}: 1.00 {a:.4} px, 1.11 {b:.4}");
            }
        }
        destroy(&mut fonts);
    }

    #[test]
    fn ch_is_the_references() {
        use crate::line::{ch, round_up};

        let fixture = load_fixture();
        let mut fonts = uninit();
        assert!(init(&mut fonts));
        for name in ["ch-mono", "ch-stix"] {
            let s = find(&fixture, name);
            let Some(style) = style_of(s) else { continue };
            let unit = ch(&mut fonts, style);
            assert!(
                (unit - s.width).abs() < 1e-3,
                "{name}: ch {unit:.4} px, WebKit {:.4}",
                s.width
            );
            assert_eq!(
                round_up(5.0 * unit),
                s.slot,
                "{name}: round(up, 5ch, 1px) {} px, WebKit {}",
                round_up(5.0 * unit),
                s.slot
            );
        }
        assert_eq!(find(&fixture, "ch-mono").slot, 55.0);
        assert_eq!(find(&fixture, "ch-mono").engines["firefox"].slot, 54.0);
        destroy(&mut fonts);
    }
}
