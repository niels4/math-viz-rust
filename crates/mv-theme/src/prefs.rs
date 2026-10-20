//! The stored preferences that pick the theme shown, and the logic around them.
//!
//! What is stored: the appearance mode, and one theme per scheme. The
//! platform layer reads and writes the stored text and reports the system's
//! scheme; this module says what follows.
//!
//! Ported from `src/theme/prefs.odin` (189 lines): the stored preferences that
//! pick the theme shown, and the logic around them.

use enum_map::{Enum, EnumArray, EnumMap};

/// What the Appearance toggle sets: follow the system, or always light or dark.
///
/// Ported from `prefs.odin:10` (`Mode :: enum u8 { System, Light, Dark }`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum Mode {
    System,
    Light,
    Dark,
}

/// Light or dark: what the mode gives, or in System what the system gives.
///
/// Ported from `prefs.odin:17` (`Scheme :: enum u8 { Dark, Light }`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum Scheme {
    Dark,
    Light,
}

/// The six themes, the three dark ones first.
///
/// Ported from `prefs.odin:23` (`Slug :: enum u8`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum Slug {
    ArcticIce,
    TronCyan,
    MidnightViolet,
    CleanTeal,
    EducationalWarm,
    SageEditorial,
}

/// The words for a mode, as stored.
///
/// Ported from `prefs.odin:33` (`MODE_WORDS := [Mode]string`, `@(rodata)`).
pub static MODE_WORDS: EnumMap<Mode, &str> = EnumMap::from_array(["system", "light", "dark"]);

/// The words for a scheme, as stored.
///
/// Ported from `prefs.odin:40` (`SCHEME_WORDS := [Scheme]string`, `@(rodata)`).
pub static SCHEME_WORDS: EnumMap<Scheme, &str> = EnumMap::from_array(["dark", "light"]);

/// The words for a theme, as stored.
///
/// Ported from `prefs.odin:46` (`SLUG_WORDS := [Slug]string`, `@(rodata)`).
pub static SLUG_WORDS: EnumMap<Slug, &str> = EnumMap::from_array([
    "arctic-ice",
    "tron-cyan",
    "midnight-violet",
    "clean-teal",
    "educational-warm",
    "sage-editorial",
]);

/// What the settings menu and the theme chip call each theme.
///
/// Ported from `prefs.odin:57` (`SLUG_LABELS := [Slug]string`, `@(rodata)`).
pub static SLUG_LABELS: EnumMap<Slug, &str> = EnumMap::from_array([
    "Arctic Ice",
    "Tron Cyan",
    "Midnight Violet",
    "Clean Teal",
    "Educational Warm",
    "Sage Editorial",
]);

/// The scheme a theme belongs to.
///
/// Ported from `prefs.odin:67` (`slug_scheme`; the enum-order comparison
/// `slug <= .Midnight_Violet` becomes an explicit match).
pub fn slug_scheme(slug: Slug) -> Scheme {
    match slug {
        Slug::ArcticIce | Slug::TronCyan | Slug::MidnightViolet => Scheme::Dark,
        Slug::CleanTeal | Slug::EducationalWarm | Slug::SageEditorial => Scheme::Light,
    }
}

/// Each scheme's three themes in the settings menu's order: the list's rows,
/// which keep their places while the scheme changes.
///
/// Ported from `prefs.odin:74` (`SCHEME_SLUGS := [Scheme][3]Slug`, `@(rodata)`).
pub static SCHEME_SLUGS: EnumMap<Scheme, [Slug; 3]> = EnumMap::from_array([
    [Slug::ArcticIce, Slug::TronCyan, Slug::MidnightViolet],
    [Slug::CleanTeal, Slug::EducationalWarm, Slug::SageEditorial],
]);

/// The row a theme takes in its scheme's list.
///
/// Ported from `prefs.odin:80` (`slug_row -> int`).
pub fn slug_row(slug: Slug) -> i64 {
    for (row, s) in SCHEME_SLUGS[slug_scheme(slug)].iter().enumerate() {
        if *s == slug {
            return row as i64;
        }
    }
    unreachable!("every theme takes a row in its scheme's list");
}

/// The scheme in use: the mode's, or in System the system's.
///
/// Ported from `prefs.odin:90` (`scheme_for`).
pub fn scheme_for(mode: Mode, system: Scheme) -> Scheme {
    match mode {
        Mode::Light => Scheme::Light,
        Mode::Dark => Scheme::Dark,
        Mode::System => system,
    }
}

/// The theme shown: the pick for the scheme in use.
///
/// Ported from `prefs.odin:102` (`shown`).
pub fn shown(prefs: Prefs, system: Scheme) -> Slug {
    if scheme_for(prefs.mode, system) == Scheme::Dark {
        prefs.dark
    } else {
        prefs.light
    }
}

/// The prefs with `slug` picked for its scheme: the settings menu's pick, which
/// lists only the themes of the scheme in use.
///
/// Ported from `prefs.odin:108` (`with_theme`).
pub fn with_theme(prefs: Prefs, slug: Slug) -> Prefs {
    let mut picked = prefs;
    if slug_scheme(slug) == Scheme::Dark {
        picked.dark = slug;
    } else {
        picked.light = slug;
    }
    picked
}

/// `slug` picked and shown, for whatever lists every theme (the command line's
/// `--theme` here). The mode changes only when the pick would otherwise stay
/// hidden, so System survives a pick that matches the system.
///
/// Ported from `prefs.odin:121` (`showing_theme`).
pub fn showing_theme(prefs: Prefs, slug: Slug, system: Scheme) -> Prefs {
    let mut picked = with_theme(prefs, slug);
    let scheme = slug_scheme(slug);
    if scheme != scheme_for(prefs.mode, system) {
        picked.mode = if scheme == Scheme::Dark {
            Mode::Dark
        } else {
            Mode::Light
        };
    }
    picked
}

/// What is stored: the mode, and one theme per scheme.
///
/// Ported from `prefs.odin:131` (`Prefs :: struct { mode, dark, light }`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Prefs {
    pub mode: Mode,
    /// One of the dark themes.
    pub dark: Slug,
    /// One of the light themes.
    pub light: Slug,
}

/// System; Arctic Ice for dark; Clean Teal for light.
///
/// Ported from `prefs.odin:138` (`DEFAULT_PREFS :: Prefs{...}`).
pub const DEFAULT_PREFS: Prefs = Prefs {
    mode: Mode::System,
    dark: Slug::ArcticIce,
    light: Slug::CleanTeal,
};

/// Where the Rust app keeps the prefs in storage. The web build keeps one
/// value per key, as math-viz2 does (`useAppTheme.ts`): the mode under the
/// light-dark key, the dark pick under the dark-theme key, the light pick
/// under the light-theme key, each a JSON string (`"system"`, a slug).
/// The native build keeps the same three values as one JSON object in a file
/// (`prefs_json` below).
pub const PREFS_KEY_LIGHT_DARK: &str = "math-viz2-rust-light-dark";
pub const PREFS_KEY_DARK_THEME: &str = "math-viz2-rust-dark-theme";
pub const PREFS_KEY_LIGHT_THEME: &str = "math-viz2-rust-light-theme";

/// Stored prefs as read: each field checked, a missing or unknown one taking
/// its default. Text that isn't JSON gives the defaults, as a storage read
/// that throws does.
///
/// Ported from `prefs.odin:146` (`parse_prefs`).
pub fn parse_prefs(text: &str) -> Prefs {
    let mut prefs = DEFAULT_PREFS;
    let Ok(value): Result<serde_json::Value, _> = serde_json::from_str(text) else {
        return prefs;
    };
    let Some(fields) = value.as_object() else {
        return prefs;
    };
    if let Some(mode) = fields.get("mode").and_then(|v| word_in(v, &MODE_WORDS).0) {
        prefs.mode = mode;
    }
    if let Some(dark) = fields.get("dark").and_then(|v| word_in(v, &SLUG_WORDS).0)
        && slug_scheme(dark) == Scheme::Dark
    {
        prefs.dark = dark;
    }
    if let Some(light) = fields.get("light").and_then(|v| word_in(v, &SLUG_WORDS).0)
        && slug_scheme(light) == Scheme::Light
    {
        prefs.light = light;
    }
    prefs
}

/// The prefs as stored: JSON of `{ mode, dark, light }`.
///
/// Ported from `prefs.odin:169` (`prefs_json`).
pub fn prefs_json(prefs: Prefs) -> String {
    format!(
        "{{\"mode\":\"{}\",\"dark\":\"{}\",\"light\":\"{}\"}}",
        MODE_WORDS[prefs.mode], SLUG_WORDS[prefs.dark], SLUG_WORDS[prefs.light],
    )
}

/// The enum value whose word a JSON value holds.
///
/// Ported from `prefs.odin:181` (`word_in`, private; Odin's generic `[$E]string`
/// table becomes a generic function over the word-map enum: the exact bound is
/// the body milestone's call).
fn word_in<E>(value: &serde_json::Value, words: &EnumMap<E, &str>) -> (Option<E>, bool)
where
    E: for<'a> EnumArray<&'a str>,
{
    let Some(text) = value.as_str() else {
        return (None, false);
    };
    for (e, word) in words {
        if *word == text {
            return (Some(e), true);
        }
    }
    (None, false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::tests::ALL_SLUGS;

    const ALL_MODES: [Mode; 3] = [Mode::System, Mode::Light, Mode::Dark];
    const ALL_SCHEMES: [Scheme; 2] = [Scheme::Dark, Scheme::Light];
    const DARK_SLUGS: [Slug; 3] = [Slug::ArcticIce, Slug::TronCyan, Slug::MidnightViolet];
    const LIGHT_SLUGS: [Slug; 3] = [Slug::CleanTeal, Slug::EducationalWarm, Slug::SageEditorial];

    #[test]
    fn each_stored_field_is_checked() {
        let valid = Prefs {
            mode: Mode::Light,
            dark: Slug::TronCyan,
            light: Slug::SageEditorial,
        };
        let cases: [(&str, Prefs); 14] = [
            (
                "{\"mode\":\"light\",\"dark\":\"tron-cyan\",\"light\":\"sage-editorial\"}",
                valid,
            ),
            (
                "{\"mode\":\"dusk\",\"dark\":\"tron-cyan\",\"light\":\"sage-editorial\"}",
                Prefs {
                    mode: Mode::System,
                    dark: Slug::TronCyan,
                    light: Slug::SageEditorial,
                },
            ),
            // A light theme can't be the dark pick, nor the other way round.
            (
                "{\"mode\":\"light\",\"dark\":\"clean-teal\",\"light\":\"sage-editorial\"}",
                Prefs {
                    mode: Mode::Light,
                    dark: Slug::ArcticIce,
                    light: Slug::SageEditorial,
                },
            ),
            (
                "{\"mode\":\"light\",\"dark\":\"tron-cyan\",\"light\":\"midnight-violet\"}",
                Prefs {
                    mode: Mode::Light,
                    dark: Slug::TronCyan,
                    light: Slug::CleanTeal,
                },
            ),
            (
                "{\"mode\":\"dark\"}",
                Prefs {
                    mode: Mode::Dark,
                    dark: Slug::ArcticIce,
                    light: Slug::CleanTeal,
                },
            ),
            (
                "{\"mode\":\"light\",\"dark\":\"tron-cyan\",\"light\":\"sage-editorial\",\"extra\":1}",
                valid,
            ),
            (
                "{\"mode\":\"dark\",\"dark\":\"tron-cyan\",\"light\":\"neon\"}",
                Prefs {
                    mode: Mode::Dark,
                    dark: Slug::TronCyan,
                    light: Slug::CleanTeal,
                },
            ),
            // Junk gives the defaults: null, a bare theme, a number, an array,
            // fields of the wrong types.
            ("null", DEFAULT_PREFS),
            ("\"arctic-ice\"", DEFAULT_PREFS),
            ("7", DEFAULT_PREFS),
            ("[]", DEFAULT_PREFS),
            ("{\"mode\":1,\"dark\":null,\"light\":{}}", DEFAULT_PREFS),
            // Not JSON at all: a storage read that throws gives the defaults.
            ("{not json", DEFAULT_PREFS),
            ("", DEFAULT_PREFS),
        ];
        for (stored, want) in cases {
            assert_eq!(
                parse_prefs(stored),
                want,
                "{stored}: got {:?}",
                parse_prefs(stored)
            );
        }
    }

    #[test]
    fn prefs_are_written_as_react_writes_them() {
        assert_eq!(
            prefs_json(DEFAULT_PREFS),
            "{\"mode\":\"system\",\"dark\":\"arctic-ice\",\"light\":\"clean-teal\"}"
        );
        for mode in ALL_MODES {
            for dark in DARK_SLUGS {
                for light in LIGHT_SLUGS {
                    let prefs = Prefs { mode, dark, light };
                    assert_eq!(parse_prefs(&prefs_json(prefs)), prefs);
                }
            }
        }
    }

    #[test]
    fn system_shows_the_pick_for_the_systems_scheme() {
        assert_eq!(
            DEFAULT_PREFS,
            Prefs {
                mode: Mode::System,
                dark: Slug::ArcticIce,
                light: Slug::CleanTeal
            }
        );
        assert_eq!(scheme_for(DEFAULT_PREFS.mode, Scheme::Dark), Scheme::Dark);
        assert_eq!(shown(DEFAULT_PREFS, Scheme::Dark), Slug::ArcticIce);
        assert_eq!(
            SLUG_LABELS[shown(DEFAULT_PREFS, Scheme::Dark)],
            "Arctic Ice"
        );
        assert_eq!(scheme_for(DEFAULT_PREFS.mode, Scheme::Light), Scheme::Light);
        assert_eq!(shown(DEFAULT_PREFS, Scheme::Light), Slug::CleanTeal);
        // "read stored prefs field by field, a bad one taking its default":
        // what that shows.
        assert_eq!(
            shown(
                parse_prefs("{\"mode\":\"dark\",\"dark\":\"tron-cyan\",\"light\":\"neon\"}"),
                Scheme::Dark
            ),
            Slug::TronCyan
        );
    }

    #[test]
    fn only_system_follows_the_system() {
        let prefs = DEFAULT_PREFS;
        assert_eq!(shown(prefs, Scheme::Dark), Slug::ArcticIce);
        assert_eq!(shown(prefs, Scheme::Light), Slug::CleanTeal);
        assert_eq!(shown(prefs, Scheme::Dark), Slug::ArcticIce);
        let mut prefs = prefs;
        prefs.mode = Mode::Light;
        assert_eq!(shown(prefs, Scheme::Light), Slug::CleanTeal);
        assert_eq!(shown(prefs, Scheme::Dark), Slug::CleanTeal);
        for mode in ALL_MODES {
            for system in ALL_SCHEMES {
                let want = if mode == Mode::System {
                    system
                } else if mode == Mode::Light {
                    Scheme::Light
                } else {
                    Scheme::Dark
                };
                assert_eq!(scheme_for(mode, system), want);
            }
        }
    }

    #[test]
    fn each_scheme_keeps_its_pick() {
        let system = Scheme::Dark;
        let mut prefs = DEFAULT_PREFS;
        prefs.mode = Mode::Light;
        prefs = with_theme(prefs, Slug::SageEditorial);
        prefs.mode = Mode::Dark;
        assert_eq!(shown(prefs, system), Slug::ArcticIce);
        prefs = with_theme(prefs, Slug::MidnightViolet);
        prefs.mode = Mode::Light;
        assert_eq!(shown(prefs, system), Slug::SageEditorial);
        prefs.mode = Mode::System;
        assert_eq!(shown(prefs, system), Slug::MidnightViolet);
        assert_eq!(
            prefs_json(prefs),
            "{\"mode\":\"system\",\"dark\":\"midnight-violet\",\"light\":\"sage-editorial\"}"
        );
        // A pick never moves the other scheme's.
        for slug in ALL_SLUGS {
            let picked = with_theme(DEFAULT_PREFS, slug);
            assert_eq!(picked.mode, DEFAULT_PREFS.mode);
            if slug_scheme(slug) == Scheme::Dark {
                assert_eq!(
                    picked,
                    Prefs {
                        mode: Mode::System,
                        dark: slug,
                        light: Slug::CleanTeal
                    }
                );
            } else {
                assert_eq!(
                    picked,
                    Prefs {
                        mode: Mode::System,
                        dark: Slug::ArcticIce,
                        light: slug
                    }
                );
            }
        }
    }

    #[test]
    fn showing_a_theme_keeps_system_when_it_shows() {
        let prefs = showing_theme(DEFAULT_PREFS, Slug::TronCyan, Scheme::Dark);
        assert_eq!(
            prefs,
            Prefs {
                mode: Mode::System,
                dark: Slug::TronCyan,
                light: Slug::CleanTeal
            }
        );
        let prefs = showing_theme(prefs, Slug::EducationalWarm, Scheme::Dark);
        assert_eq!(
            prefs,
            Prefs {
                mode: Mode::Light,
                dark: Slug::TronCyan,
                light: Slug::EducationalWarm
            }
        );
        assert_eq!(shown(prefs, Scheme::Dark), Slug::EducationalWarm);
        assert_eq!(
            showing_theme(
                Prefs {
                    mode: Mode::Dark,
                    dark: Slug::ArcticIce,
                    light: Slug::CleanTeal
                },
                Slug::CleanTeal,
                Scheme::Light
            ),
            Prefs {
                mode: Mode::Light,
                dark: Slug::ArcticIce,
                light: Slug::CleanTeal
            }
        );
        // Whatever the start, the pick shows.
        for mode in ALL_MODES {
            for system in ALL_SCHEMES {
                for slug in ALL_SLUGS {
                    let start = Prefs {
                        mode,
                        dark: Slug::MidnightViolet,
                        light: Slug::SageEditorial,
                    };
                    assert_eq!(shown(showing_theme(start, slug, system), system), slug);
                }
            }
        }
    }

    #[test]
    fn each_scheme_lists_its_three_themes() {
        assert_eq!(
            SCHEME_SLUGS[Scheme::Dark],
            [Slug::ArcticIce, Slug::TronCyan, Slug::MidnightViolet]
        );
        assert_eq!(
            SCHEME_SLUGS[Scheme::Light],
            [Slug::CleanTeal, Slug::EducationalWarm, Slug::SageEditorial]
        );
        for scheme in ALL_SCHEMES {
            for (row, slug) in SCHEME_SLUGS[scheme].iter().enumerate() {
                assert_eq!(slug_scheme(*slug), scheme);
                assert_eq!(slug_row(*slug), row as i64);
            }
        }
    }
}
