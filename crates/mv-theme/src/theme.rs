//! Resolved themes: every token computed as the browser computes it and encoded
//! in the build's space, plus the recurring mixes.
//!
//! A theme file writes each token as an Oklch colour, another token, or a mix
//! of two in Oklch (`themes.rs`, generated). Resolving follows the aliases
//! and makes the mixes in Oklch, then encodes every token in the build's space.
//!
//! Ported from `src/theme/theme.odin` (141 lines): resolved themes and the
//! recurring mixes.

use enum_map::{Enum, EnumMap};

use crate::color::{Oklch as Resolved, Space, mix, to_space};
use crate::prefs::Slug;
use crate::themes::{DEFINITIONS, Definition, Token, Value};

/// A colour as drawn: each channel and the alpha 0 to 1, the channels encoded
/// in the theme's space, the alpha straight.
///
/// Ported from `theme.odin:11` (`Color :: [4]f32`).
pub type Color = [f32; 4];

/// A theme with every token resolved: what the page draws with.
///
/// Ported from `theme.odin:14` (`Theme :: struct { slug, space, colors, ... }`).
#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    pub slug: Slug,
    /// The space its colours are encoded in.
    pub space: Space,
    pub colors: EnumMap<Token, Color>,
    /// `--radius`, px.
    pub radius: f32,
    /// `--sig-card-radius`.
    pub card_radius: f32,
    /// `--sig-card-shadow`: none, or Educational Warm's paper shadow.
    pub card_shadow: Option<Shadow>,
    /// `--sig-glow-radius`: the blur radius of the curve's glow.
    pub glow_radius: f32,
    /// `--sig-hud-ticks`: the opacity of the plane frame's HUD ticks.
    /// The port draws none since the 2026-10-18 ruling; the table keeps
    /// React's token (1.0 for tron-cyan) and the fixture test proves the mirror.
    pub hud_ticks: f32,
    /// `--sig-label-tracking`: the letter spacing of caps labels, px.
    pub label_tracking: f32,
    /// `--sig-readout-font`: the readouts' numerals.
    pub readout_font: crate::themes::ReadoutFont,
}

/// A box-shadow without spread: offset and blur in px.
///
/// Ported from `theme.odin:28` (`Shadow :: struct { offset, blur, color }`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shadow {
    pub offset: [f32; 2],
    pub blur: f32,
    pub color: Color,
}

/// The theme `slug` with each token resolved as the browser computes it
/// (aliases followed and mixes made in Oklch) and encoded in `space` as the
/// build draws it.
///
/// Ported from `theme.odin:73` (`resolve`).
pub fn resolve(slug: Slug, space: Space) -> Theme {
    let word = crate::prefs::SLUG_WORDS[slug];
    let def = &DEFINITIONS
        .iter()
        .find(|t| t.slug == word)
        .unwrap_or_else(|| panic!("no theme for {slug:?}"))
        .def;
    let mut colors = [[0.0f32; 4]; Token::LENGTH];
    for &(token, value) in def.colors {
        colors[token.into_usize()] = to_space(value_oklch(def, value), space);
    }
    Theme {
        slug,
        space,
        colors: EnumMap::from_array(colors),
        radius: def.radius,
        card_radius: def.card_radius,
        card_shadow: def.card_shadow.map(|s| Shadow {
            offset: s.offset,
            blur: s.blur,
            color: to_space(value_oklch(def, s.color), space),
        }),
        glow_radius: def.glow_radius,
        hud_ticks: def.hud_ticks,
        label_tracking: def.label_tracking,
        readout_font: def.readout_font,
    }
}

/// A token's computed value: an Oklch colour, as `getComputedStyle` reports it.
///
/// Ported from `theme.odin:93` (`token_oklch`).
pub fn token_oklch(d: &Definition, token: Token) -> Resolved {
    let (_, value) = d
        .colors
        .iter()
        .find(|(t, _)| *t == token)
        .copied()
        .unwrap_or_else(|| panic!("theme has no {token:?}"));
    value_oklch(d, value)
}

/// A written value's computed colour.
///
/// Ported from `theme.odin:97` (`value_oklch`).
pub fn value_oklch(d: &Definition, value: Value) -> Resolved {
    match value {
        Value::Oklch(o) => Resolved {
            l: o.l,
            c: o.c,
            h: o.h,
            alpha: o.alpha,
        },
        Value::Ref(token) => token_oklch(d, token),
        Value::Mix(m) => {
            let b = match m.b {
                Some(other) => token_oklch(d, other),
                None => crate::color::TRANSPARENT,
            };
            mix(token_oklch(d, m.a), m.p, b)
        }
    }
}

/// `color` at `amount` of its alpha: `color-mix(in oklch, color p%,
/// transparent)`, which keeps the colour and scales its alpha by p.
///
/// Ported from `theme.odin:115` (`fade`).
pub fn fade(color: Color, amount: f32) -> Color {
    [color[0], color[1], color[2], color[3] * amount]
}

/// The 3 px focus ring: primary at 32 %.
///
/// Ported from `theme.odin:122` (`ring`).
pub fn ring(t: &Theme) -> Color {
    fade(t.colors[Token::Primary], 0.32)
}

/// A plate or tint at 26 %, of primary unless an ink is given: the picker's
/// selection, a lit term's plate, the moved value, a strip's fill, the
/// explainer's chip in its parameter's ink.
///
/// Ported from `theme.odin:128` (`plate`; the default `ink := Token.Primary`
/// is explicit here).
pub fn plate(t: &Theme, ink: Token) -> Color {
    fade(t.colors[ink], 0.26)
}

/// A held modifier's badge: accent (fine) or warning (snap) at 24 %.
///
/// Ported from `theme.odin:133` (`badge`).
pub fn badge(t: &Theme, ink: Token) -> Color {
    fade(t.colors[ink], 0.24)
}

/// A lit row's band and a dragged box's fill: the parameter's colour at 18 %.
///
/// Ported from `theme.odin:139` (`band`).
pub fn band(t: &Theme, ink: Token) -> Color {
    fade(t.colors[ink], 0.18)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::tests::ALL_SLUGS;
    use crate::prefs::SLUG_WORDS;

    const FIXTURE: &str = include_str!("../testdata/tokens.json");

    #[derive(Debug, serde::Deserialize)]
    struct FixtureColour {
        #[allow(dead_code)]
        computed: String,
        #[serde(default)]
        oklch: Vec<f64>,
        alpha: f64,
        rgb: [i32; 3],
        #[serde(default)]
        #[allow(dead_code)]
        engines: std::collections::HashMap<String, [i32; 3]>,
    }

    #[derive(Debug, serde::Deserialize)]
    struct FixtureShadow {
        computed: String,
        #[serde(default)]
        offset: [f32; 2],
        #[serde(default)]
        blur: f32,
        #[serde(default)]
        spread: f32,
        #[serde(default)]
        color: Option<FixtureColour>,
    }

    #[derive(Debug, serde::Deserialize)]
    struct FixtureValues {
        #[serde(rename = "--radius")]
        radius: String,
        #[serde(rename = "--sig-card-radius")]
        card_radius: String,
        #[serde(rename = "--sig-card-shadow")]
        card_shadow: FixtureShadow,
        #[serde(rename = "--sig-glow-radius")]
        glow_radius: String,
        #[serde(rename = "--sig-hud-ticks")]
        hud_ticks: String,
        #[serde(rename = "--sig-label-tracking")]
        label_tracking: String,
        #[serde(rename = "--sig-readout-font")]
        readout_font: String,
    }

    #[derive(Debug, serde::Deserialize)]
    struct FixtureTheme {
        colors: std::collections::HashMap<String, FixtureColour>,
        mixes: std::collections::HashMap<String, FixtureColour>,
        values: FixtureValues,
    }

    #[derive(Debug, serde::Deserialize)]
    struct Fixture {
        engine: String,
        themes: std::collections::HashMap<String, FixtureTheme>,
    }

    fn load_fixture() -> Fixture {
        let fixture: Fixture = serde_json::from_str(FIXTURE).expect("testdata/tokens.json parses");
        assert_eq!(fixture.themes.len(), ALL_SLUGS.len());
        fixture
    }

    // A token's CSS name: ParamA18 is --param-a-18.
    fn css_name(token: Token) -> String {
        let mut name = String::from("--");
        let mut prev_letter = false;
        for (i, ch) in format!("{token:?}").chars().enumerate() {
            if ch.is_ascii_uppercase() {
                if i > 0 {
                    name.push('-');
                }
                name.push(ch.to_ascii_lowercase());
                prev_letter = true;
            } else {
                if ch.is_ascii_digit() && prev_letter {
                    name.push('-');
                }
                name.push(ch);
                prev_letter = ch.is_ascii_alphabetic();
            }
        }
        name
    }

    // How far a drawn colour is from the browser's pixel, in 255ths: the
    // largest channel's and the alpha's difference.
    fn off_by(ours: Color, theirs: &FixtureColour) -> (f64, f64) {
        let mut rgb: f64 = 0.0;
        for (o, t) in ours.iter().zip(theirs.rgb.iter()) {
            rgb = rgb.max((f64::from(*o) * 255.0 - f64::from(*t)).abs());
        }
        (rgb, (f64::from(ours[3]) - theirs.alpha).abs() * 255.0)
    }

    fn same_pixel(ours: Color, theirs: &FixtureColour) -> bool {
        ours.iter()
            .zip(theirs.rgb.iter())
            .all(|(o, t)| (f64::from(*o) * 255.0 + 0.5) as i32 == *t)
    }

    #[test]
    fn every_token_is_drawn_as_the_browser_draws_it() {
        use crate::color::Space;
        let fixture = load_fixture();
        let mut worst = 0.0;
        let mut worst_at = String::new();
        let (mut same, mut total) = (0, 0);
        for slug in ALL_SLUGS {
            let taken = fixture.themes.get(SLUG_WORDS[slug]).unwrap_or_else(|| {
                panic!("no {} in the fixture", SLUG_WORDS[slug]);
            });
            // The fixture holds every colour token the theme files define, and
            // the port has each.
            assert_eq!(taken.colors.len(), Token::LENGTH);
            let theme = resolve(slug, Space::Srgb);
            for token in ALL_TOKENS {
                let name = css_name(token);
                let theirs = taken.colors.get(&name).unwrap_or_else(|| {
                    panic!("{}: no {name} in the fixture", SLUG_WORDS[slug]);
                });
                let ours = theme.colors[token];
                let (rgb, alpha) = off_by(ours, theirs);
                assert!(
                    rgb <= 1.0 && alpha <= 1.0,
                    "{} {name}: the port draws {:?}, {} draws {:?} at alpha {:?} ({:.2}/255 off)",
                    SLUG_WORDS[slug],
                    ours.map(|c| c * 255.0),
                    fixture.engine,
                    theirs.rgb,
                    theirs.alpha,
                    rgb.max(alpha),
                );
                if rgb > worst {
                    worst = rgb;
                    worst_at = format!("{} {name}", SLUG_WORDS[slug]);
                }
                same += usize::from(same_pixel(ours, theirs));
                total += 1;
            }
        }
        eprintln!(
            "{total} tokens in {} themes: {same} drawn to {}'s very pixel; \
             the largest channel difference {worst:.3}/255 ({worst_at})",
            ALL_SLUGS.len(),
            fixture.engine,
        );
    }

    struct Outside {
        slug: Slug,
        names: &'static [&'static str],
        p3: [i32; 3],
    }

    struct P3Tally {
        inside: usize,
        very: usize,
        outside: usize,
        transparent: usize,
        worst: f64,
        worst_at: String,
        outside_at: Vec<String>,
    }

    #[allow(clippy::too_many_arguments)]
    fn check_p3(
        tally: &mut P3Tally,
        outside: &[Outside],
        slug: Slug,
        name: &str,
        ours: Color,
        oklch: Resolved,
        theirs: &FixtureColour,
    ) {
        use crate::color::{Space, linear};
        let at = format!("{} {name}", SLUG_WORDS[slug]);
        if theirs.alpha == 0.0 {
            assert_eq!(
                ours,
                [0.0, 0.0, 0.0, 0.0],
                "{at}: the browser computes transparent, the port draws {ours:?}"
            );
            tally.transparent += 1;
            return;
        }
        assert!(
            (f64::from(ours[3]) - theirs.alpha).abs() < 1e-6,
            "{at}: alpha {:?}, the browser's {:?}",
            ours[3],
            theirs.alpha
        );
        let drawn = [
            (f64::from(ours[0]) * 255.0 + 0.5) as i32,
            (f64::from(ours[1]) * 255.0 + 0.5) as i32,
            (f64::from(ours[2]) * 255.0 + 0.5) as i32,
        ];
        let lin = linear(oklch, Space::Srgb);
        let mut in_srgb = true;
        for v in lin {
            // The themes' direct tokens reach 1.00035 from their 4-decimal
            // values.
            in_srgb = in_srgb && v > -0.0005 && v < 1.0005;
        }
        for o in outside {
            if o.slug != slug {
                continue;
            }
            for n in o.names {
                if *n == name {
                    assert!(!in_srgb, "{at} is inside sRGB: linear {lin:?}");
                    assert_eq!(
                        drawn, o.p3,
                        "{at}: the port draws {drawn:?} in Display P3, \
                         its oklch's bytes are {:?}",
                        o.p3
                    );
                    tally.outside += 1;
                    tally.outside_at.push(at);
                    return;
                }
            }
        }
        assert!(
            in_srgb,
            "{at} is outside sRGB (linear {lin:?}) and not among the colours named"
        );
        let converted = srgb_pixel_in_p3(theirs.rgb);
        let mut off: f64 = 0.0;
        let mut very = true;
        for (d, c) in drawn.iter().zip(converted.iter()) {
            off = off.max((f64::from(*d) - c).abs());
            very = very && f64::from(*d) == (c + 0.5).floor();
        }
        assert!(
            off <= 1.0,
            "{at}: the port draws {drawn:?} in Display P3; the browser's pixel {:?} \
             is {converted:?} there ({off:.2}/255 off)",
            theirs.rgb
        );
        if off > tally.worst {
            tally.worst = off;
            tally.worst_at = at;
        }
        tally.inside += 1;
        tally.very += usize::from(very);
    }

    // Linear sRGB to linear Display P3 as it is usually published, to seven
    // places: the two spaces' primaries composed through XYZ (D65). A route of
    // its own, so the test shares no constant with `to_space`'s XYZ matrices.
    const LINEAR_SRGB_TO_P3: [[f64; 3]; 3] = [
        [0.8224621, 0.1775380, 0.0000000],
        [0.0331941, 0.9668058, 0.0000000],
        [0.0170827, 0.0723974, 0.9105199],
    ];

    // An sRGB pixel as Display P3 encodes the same colour, each channel 0 to
    // 255, unrounded.
    fn srgb_pixel_in_p3(rgb: [i32; 3]) -> [f64; 3] {
        use crate::color::gamma;
        let mut lin = [0.0; 3];
        for (i, v) in rgb.iter().enumerate() {
            let e = f64::from(*v) / 255.0;
            lin[i] = if e <= 0.04045 {
                e / 12.92
            } else {
                ((e + 0.055) / 1.055).powf(2.4)
            };
        }
        let mut p3 = [0.0; 3];
        for (i, row) in LINEAR_SRGB_TO_P3.iter().enumerate() {
            p3[i] = f64::from(gamma(row[0] * lin[0] + row[1] * lin[1] + row[2] * lin[2])) * 255.0;
        }
        p3
    }

    // The regenerated Token set (themes.rs): every token the theme files define.
    const ALL_TOKENS: [Token; Token::LENGTH] = [
        Token::Background,
        Token::BackgroundMuted,
        Token::Card,
        Token::Section,
        Token::Foreground,
        Token::ForegroundMuted,
        Token::Primary,
        Token::PrimaryForeground,
        Token::Secondary,
        Token::SecondaryForeground,
        Token::Border,
        Token::Accent,
        Token::AccentForeground,
        Token::Success,
        Token::Warning,
        Token::Destructive,
        Token::ChartGrid,
        Token::ChartLine,
        Token::ChartAccent,
        Token::ChartGlow,
        Token::ChartAxis,
        Token::ChartGridMajor,
        Token::Ordinal01,
        Token::Ordinal02,
        Token::Ordinal03,
        Token::Ordinal04,
        Token::Ordinal05,
        Token::Ordinal06,
        Token::Ordinal07,
        Token::Ordinal08,
        Token::Ordinal09,
        Token::Ordinal10,
        Token::Ordinal11,
        Token::Ordinal12,
        Token::ShadowColor,
        Token::ParamA,
        Token::ParamK,
        Token::ParamB,
        Token::ParamH,
    ];

    #[test]
    fn every_colour_in_display_p3_is_the_browsers_colour() {
        use crate::color::Space;
        let fixture = load_fixture();
        // Every colour of the six themes at this pin lies inside sRGB (linear sRGB of each
        // fixture oklch within -0.0005..1.0005); nothing is named outside.
        let outside: &[Outside] = &[
            // The readout's vivid parameter inks, outside sRGB but inside the
            // Display P3 the port draws in (h4-readout sources them from
            // SideBar.module.css): the port's P3 bytes, measured here.
            Outside {
                slug: Slug::ArcticIce,
                names: &["--param-a"],
                p3: [251, 177, 144],
            },
            Outside {
                slug: Slug::CleanTeal,
                names: &["--param-k"],
                p3: [119, 104, 0],
            },
            Outside {
                slug: Slug::SageEditorial,
                names: &["--param-k"],
                p3: [115, 68, 13],
            },
        ];
        let mut tokens = P3Tally {
            inside: 0,
            very: 0,
            outside: 0,
            transparent: 0,
            worst: 0.0,
            worst_at: String::new(),
            outside_at: Vec::new(),
        };
        let mut mixes = P3Tally {
            inside: 0,
            very: 0,
            outside: 0,
            transparent: 0,
            worst: 0.0,
            worst_at: String::new(),
            outside_at: Vec::new(),
        };
        for slug in ALL_SLUGS {
            let taken = &fixture.themes[SLUG_WORDS[slug]];
            let theme = resolve(slug, Space::DisplayP3);
            let d = &DEFINITIONS
                .iter()
                .find(|t| t.slug == SLUG_WORDS[slug])
                .unwrap_or_else(|| panic!("no theme for {slug:?}"))
                .def;
            for token in ALL_TOKENS {
                let name = css_name(token);
                check_p3(
                    &mut tokens,
                    outside,
                    slug,
                    &name,
                    theme.colors[token],
                    token_oklch(d, token),
                    &taken.colors[&name],
                );
            }
            let cases: [(&str, Color, Token); 4] = [
                ("ring", ring(&theme), Token::Primary),
                ("plate", plate(&theme, Token::Primary), Token::Primary),
                ("badge-accent", badge(&theme, Token::Accent), Token::Accent),
                (
                    "badge-warning",
                    badge(&theme, Token::Warning),
                    Token::Warning,
                ),
            ];
            for (name, ours, ink) in cases {
                check_p3(
                    &mut mixes,
                    outside,
                    slug,
                    name,
                    ours,
                    token_oklch(d, ink),
                    &taken.mixes[name],
                );
            }
        }
        // The expected count: 228 tokens with a colour (201 plus the sidebar's
        // --section and 21 inside-sRGB --param-* inks), all inside sRGB; the
        // three dark shadow colours transparent; three --param-* inks outside
        // sRGB at their oklch's bytes (h4-readout); the 24 mixes inside too.
        assert_eq!(tokens.inside, 228);
        assert_eq!(tokens.outside, 3);
        assert_eq!(tokens.transparent, 3);
        assert_eq!(mixes.outside, 0);
        for (what, tally) in [("tokens", &tokens), ("the page's mixes", &mixes)] {
            eprintln!(
                "{what} in Display P3: {} inside sRGB within 1/255 of the browser's \
                 pixel converted ({} to its very byte, the largest difference {:.3}/255, {}); \
                 {} outside sRGB at their oklch's bytes ({}); {} transparent",
                tally.inside,
                tally.very,
                tally.worst,
                tally.worst_at,
                tally.outside,
                tally.outside_at.join(", "),
                tally.transparent,
            );
        }
    }

    #[test]
    fn every_token_resolves_as_the_browser_computes_it() {
        let fixture = load_fixture();
        for slug in ALL_SLUGS {
            let taken = &fixture.themes[SLUG_WORDS[slug]];
            let d = &DEFINITIONS
                .iter()
                .find(|t| t.slug == SLUG_WORDS[slug])
                .unwrap_or_else(|| panic!("no theme for {slug:?}"))
                .def;
            for token in ALL_TOKENS {
                let theirs = &taken.colors[&css_name(token)];
                let ours = token_oklch(d, token);
                if theirs.oklch.len() != 4 {
                    // rgba(0, 0, 0, 0): the browser computes transparent so.
                    assert!(
                        theirs.alpha == 0.0 && ours.alpha == 0.0 && ours.l == 0.0 && ours.c == 0.0,
                        "{} {token:?}: {} computes {}, the port gives {ours:?}",
                        SLUG_WORDS[slug],
                        fixture.engine,
                        theirs.computed,
                    );
                    continue;
                }
                // The browser keeps the value as float32 (242.710007 for
                // 242.71): 1e-5 in lightness and chroma, 1e-3 degrees of hue,
                // which a colour without chroma doesn't have.
                let (l, c, h, a) = (
                    theirs.oklch[0],
                    theirs.oklch[1],
                    theirs.oklch[2],
                    theirs.oklch[3],
                );
                let mut hue_off = (ours.h - h).abs();
                hue_off = hue_off.min(360.0 - hue_off);
                assert!(
                    (ours.l - l).abs() < 1e-5
                        && (ours.c - c).abs() < 1e-5
                        && (c < 1e-4 || hue_off < 1e-3)
                        && (ours.alpha - a).abs() < 1e-6,
                    "{} {token:?}: {} computes {}, the port gives {ours:?}",
                    SLUG_WORDS[slug],
                    fixture.engine,
                    theirs.computed,
                );
            }
        }
    }

    #[test]
    fn the_pages_mixes_are_the_browsers() {
        use crate::color::Space;
        let fixture = load_fixture();
        for slug in ALL_SLUGS {
            let name = SLUG_WORDS[slug];
            let taken = &fixture.themes[name];
            let theme = resolve(slug, Space::Srgb);
            let cases: [(&str, Color); 4] = [
                ("ring", ring(&theme)),
                ("plate", plate(&theme, Token::Primary)),
                ("badge-accent", badge(&theme, Token::Accent)),
                ("badge-warning", badge(&theme, Token::Warning)),
            ];
            assert_eq!(taken.mixes.len(), cases.len());
            for (name, ours) in cases {
                let theirs = taken.mixes.get(name).unwrap_or_else(|| {
                    panic!("{}: no mix {name} in the fixture", SLUG_WORDS[slug]);
                });
                let (rgb, alpha) = off_by(ours, theirs);
                assert!(
                    rgb <= 1.0 && alpha <= 1.0,
                    "{} {name}: the port draws {:?}, {} draws {:?} at alpha {:?}",
                    SLUG_WORDS[slug],
                    ours.map(|c| c * 255.0),
                    fixture.engine,
                    theirs.rgb,
                    theirs.alpha,
                );
            }
        }
    }

    // "9.6px" or "0" as a number.
    fn px(text: &str) -> f32 {
        text.strip_suffix("px")
            .unwrap_or(text)
            .parse()
            .unwrap_or(-1.0)
    }

    #[test]
    fn the_themes_other_values_are_the_browsers() {
        use crate::color::Space;
        let fixture = load_fixture();
        for slug in ALL_SLUGS {
            let v = &fixture.themes[SLUG_WORDS[slug]].values;
            let theme = resolve(slug, Space::Srgb);
            let name = SLUG_WORDS[slug];
            assert!(
                (theme.radius - px(&v.radius)).abs() < 1e-4,
                "{name} --radius {}",
                v.radius
            );
            assert!(
                (theme.card_radius - px(&v.card_radius)).abs() < 1e-4,
                "{name} card radius {}",
                v.card_radius
            );
            assert_eq!(
                theme.glow_radius,
                px(&v.glow_radius),
                "{name} glow radius {}",
                v.glow_radius
            );
            assert_eq!(
                theme.hud_ticks,
                px(&v.hud_ticks),
                "{name} HUD ticks {}",
                v.hud_ticks
            );
            assert!(
                (theme.label_tracking - px(&v.label_tracking)).abs() < 1e-4,
                "{name} label tracking {}",
                v.label_tracking
            );
            let family = v
                .readout_font
                .split(',')
                .next()
                .unwrap_or("")
                .trim()
                .trim_matches('"');
            assert!(
                family == "Roboto Mono" || family == "STIX Two Text",
                "{name} readout font {}",
                v.readout_font
            );
            assert_eq!(
                theme.readout_font,
                if family == "STIX Two Text" {
                    crate::themes::ReadoutFont::StixTwoText
                } else {
                    crate::themes::ReadoutFont::RobotoMono
                }
            );
            if v.card_shadow.computed == "none" {
                assert!(
                    theme.card_shadow.is_none(),
                    "{name}: the browser has no card shadow, the port draws {:?}",
                    theme.card_shadow
                );
                continue;
            }
            let shadow = theme.card_shadow.unwrap_or_else(|| {
                panic!(
                    "{name}: the browser's card shadow is {}",
                    v.card_shadow.computed
                );
            });
            let (rgb, alpha) = off_by(
                shadow.color,
                v.card_shadow.color.as_ref().unwrap_or_else(|| {
                    panic!("{name}: the browser's card shadow has no colour");
                }),
            );
            assert!(
                shadow.offset == v.card_shadow.offset
                    && shadow.blur == v.card_shadow.blur
                    && v.card_shadow.spread == 0.0
                    && rgb <= 1.0
                    && alpha <= 1.0,
                "{name} card shadow: the port draws {shadow:?}, the browser {}",
                v.card_shadow.computed
            );
        }
    }
}
