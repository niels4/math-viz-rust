// The six themes as math-viz2 writes them at af434dc (src/style/themes/mathviz_*.module.css),
// written from those files and the reference by `tools/run themes`: run it again rather than
// editing this file.
// Token is every colour token the files define, in their order; each theme gives each token as
// its CSS does: an oklch colour, another token's value, or a mix of two in oklch. resolve
// (theme.rs) turns them into colours.
// generated, do not edit

use enum_map::Enum;

/// Every colour token the theme files define, in their order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Enum)]
pub enum Token {
    Background,
    BackgroundMuted,
    Card,
    Foreground,
    ForegroundMuted,
    Primary,
    PrimaryForeground,
    Secondary,
    SecondaryForeground,
    Border,
    Accent,
    AccentForeground,
    Success,
    Warning,
    Destructive,
    ChartGrid,
    ChartLine,
    ChartAccent,
    ChartGlow,
    Ordinal01,
    Ordinal02,
    Ordinal03,
    Ordinal04,
    Ordinal05,
    Ordinal06,
    Ordinal07,
    Ordinal08,
    Ordinal09,
    Ordinal10,
    Ordinal11,
    Ordinal12,
}

/// An oklch colour as a theme file writes it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Oklch {
    pub l: f64,
    pub c: f64,
    pub h: f64,
    pub alpha: f64,
}

/// A mix of two tokens' colours in oklch: `color-mix(in oklch, var(--a) p%, var(--b))`,
/// or transparent where `b` is None.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mix {
    pub a: Token,
    pub p: f64,
    pub b: Option<Token>,
}

/// A colour token's value as its theme file writes it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Value {
    Oklch(Oklch),
    Ref(Token),
    Mix(Mix),
}

/// A theme as its file writes it: every token's value with the lengths, shadow, glow and
/// readout font. `colors` is a slice rather than a map so the generated table stays literal;
// `resolve` builds the token map from it.
#[derive(Clone, Copy, Debug)]
pub struct Definition {
    pub colors: &'static [(Token, Value)],
    pub radius: f32,
    pub card_radius: f32,
    pub card_shadow: Option<ShadowDefinition>,
    pub glow_radius: f32,
    pub hud_ticks: f32,
    pub label_tracking: f32,
    pub readout_font: ReadoutFont,
}

/// A box-shadow without spread: offset and blur in px.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShadowDefinition {
    pub offset: [f32; 2],
    pub blur: f32,
    pub color: Value,
}

/// The families the themes set for the readouts (--sig-readout-font's first family).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReadoutFont {
    RobotoMono,
    StixTwoText,
}

/// One theme's slug with its definition.
#[derive(Clone, Copy, Debug)]
pub struct ThemeDef {
    pub slug: &'static str,
    pub def: Definition,
}

pub static DEFINITIONS: &[ThemeDef] = &[
    ThemeDef {
        slug: "arctic-ice",
        def: Definition {
            colors: &[
                (
                    Token::Background,
                    Value::Oklch(Oklch {
                        l: 0.155,
                        c: 0.0147,
                        h: 242.71,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::BackgroundMuted,
                    Value::Oklch(Oklch {
                        l: 0.2086,
                        c: 0.0208,
                        h: 245.84,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Card,
                    Value::Oklch(Oklch {
                        l: 0.2301,
                        c: 0.0203,
                        h: 245.67,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Foreground,
                    Value::Oklch(Oklch {
                        l: 0.9724,
                        c: 0.0053,
                        h: 197.07,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::ForegroundMuted,
                    Value::Oklch(Oklch {
                        l: 0.717,
                        c: 0.0246,
                        h: 213.67,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Primary,
                    Value::Oklch(Oklch {
                        l: 0.7472,
                        c: 0.0924,
                        h: 166.32,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::PrimaryForeground,
                    Value::Oklch(Oklch {
                        l: 0.1307,
                        c: 0.0129,
                        h: 241.44,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Secondary,
                    Value::Oklch(Oklch {
                        l: 0.2885,
                        c: 0.0208,
                        h: 248.55,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::SecondaryForeground,
                    Value::Oklch(Oklch {
                        l: 0.9514,
                        c: 0.0096,
                        h: 197.0,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Border,
                    Value::Oklch(Oklch {
                        l: 0.2885,
                        c: 0.0208,
                        h: 248.55,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Accent,
                    Value::Oklch(Oklch {
                        l: 0.8532,
                        c: 0.0589,
                        h: 177.1,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::AccentForeground,
                    Value::Oklch(Oklch {
                        l: 0.1307,
                        c: 0.0129,
                        h: 241.44,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Success,
                    Value::Oklch(Oklch {
                        l: 0.7729,
                        c: 0.1535,
                        h: 163.22,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Warning,
                    Value::Oklch(Oklch {
                        l: 0.8369,
                        c: 0.1644,
                        h: 84.43,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Destructive,
                    Value::Oklch(Oklch {
                        l: 0.7106,
                        c: 0.1661,
                        h: 22.22,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::ChartGrid,
                    Value::Oklch(Oklch {
                        l: 0.2885,
                        c: 0.0208,
                        h: 248.55,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::ChartLine,
                    Value::Oklch(Oklch {
                        l: 0.7472,
                        c: 0.0924,
                        h: 166.32,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::ChartAccent,
                    Value::Oklch(Oklch {
                        l: 0.8532,
                        c: 0.0589,
                        h: 177.1,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::ChartGlow,
                    Value::Oklch(Oklch {
                        l: 0.7472,
                        c: 0.0924,
                        h: 166.32,
                        alpha: 0.4,
                    }),
                ),
                (
                    Token::Ordinal01,
                    Value::Oklch(Oklch {
                        l: 0.7472,
                        c: 0.0924,
                        h: 166.32,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal02,
                    Value::Oklch(Oklch {
                        l: 0.8532,
                        c: 0.0589,
                        h: 177.1,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal03,
                    Value::Oklch(Oklch {
                        l: 0.8276,
                        c: 0.1013,
                        h: 230.32,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal04,
                    Value::Oklch(Oklch {
                        l: 0.8112,
                        c: 0.1013,
                        h: 293.57,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal05,
                    Value::Oklch(Oklch {
                        l: 0.8228,
                        c: 0.1095,
                        h: 346.02,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal06,
                    Value::Oklch(Oklch {
                        l: 0.8077,
                        c: 0.1035,
                        h: 19.57,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal07,
                    Value::Oklch(Oklch {
                        l: 0.879,
                        c: 0.1534,
                        h: 91.61,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal08,
                    Value::Oklch(Oklch {
                        l: 0.8712,
                        c: 0.1363,
                        h: 154.45,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal09,
                    Value::Oklch(Oklch {
                        l: 0.8549,
                        c: 0.1251,
                        h: 181.07,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal10,
                    Value::Oklch(Oklch {
                        l: 0.8091,
                        c: 0.0956,
                        h: 251.81,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal11,
                    Value::Oklch(Oklch {
                        l: 0.833,
                        c: 0.1322,
                        h: 321.43,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal12,
                    Value::Oklch(Oklch {
                        l: 0.9842,
                        c: 0.0034,
                        h: 247.86,
                        alpha: 1.0,
                    }),
                ),
            ],
            radius: 9.6,      // 0.6rem
            card_radius: 9.6, // var(--radius)
            card_shadow: None,
            glow_radius: 8.0,
            hud_ticks: 0.0,
            label_tracking: 2.8,
            readout_font: ReadoutFont::RobotoMono,
        },
    },
    ThemeDef {
        slug: "tron-cyan",
        def: Definition {
            colors: &[
                (
                    Token::Background,
                    Value::Oklch(Oklch {
                        l: 0.137,
                        c: 0.02,
                        h: 228.74,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::BackgroundMuted,
                    Value::Oklch(Oklch {
                        l: 0.1817,
                        c: 0.0213,
                        h: 224.39,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Card,
                    Value::Oklch(Oklch {
                        l: 0.2073,
                        c: 0.0257,
                        h: 228.29,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Foreground,
                    Value::Oklch(Oklch {
                        l: 0.9753,
                        c: 0.0206,
                        h: 211.76,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::ForegroundMuted,
                    Value::Oklch(Oklch {
                        l: 0.7083,
                        c: 0.0317,
                        h: 220.75,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Primary,
                    Value::Oklch(Oklch {
                        l: 0.8442,
                        c: 0.1457,
                        h: 209.29,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::PrimaryForeground,
                    Value::Oklch(Oklch {
                        l: 0.2481,
                        c: 0.0422,
                        h: 216.06,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Secondary,
                    Value::Oklch(Oklch {
                        l: 0.3242,
                        c: 0.049,
                        h: 216.41,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::SecondaryForeground,
                    Value::Oklch(Oklch {
                        l: 0.9483,
                        c: 0.042,
                        h: 213.24,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Border,
                    Value::Oklch(Oklch {
                        l: 0.3396,
                        c: 0.0423,
                        h: 220.01,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Accent,
                    Value::Oklch(Oklch {
                        l: 0.7487,
                        c: 0.173,
                        h: 53.99,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::AccentForeground,
                    Value::Oklch(Oklch {
                        l: 0.2187,
                        c: 0.0475,
                        h: 59.25,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Success,
                    Value::Oklch(Oklch {
                        l: 0.7729,
                        c: 0.1535,
                        h: 163.22,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Warning,
                    Value::Oklch(Oklch {
                        l: 0.8537,
                        c: 0.1572,
                        h: 84.13,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Destructive,
                    Value::Oklch(Oklch {
                        l: 0.7045,
                        c: 0.1885,
                        h: 17.64,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::ChartGrid,
                    Value::Oklch(Oklch {
                        l: 0.3396,
                        c: 0.0423,
                        h: 220.01,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::ChartLine,
                    Value::Oklch(Oklch {
                        l: 0.8442,
                        c: 0.1457,
                        h: 209.29,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::ChartAccent,
                    Value::Oklch(Oklch {
                        l: 0.7487,
                        c: 0.173,
                        h: 53.99,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::ChartGlow,
                    Value::Oklch(Oklch {
                        l: 0.8442,
                        c: 0.1457,
                        h: 209.29,
                        alpha: 0.4,
                    }),
                ),
                (
                    Token::Ordinal01,
                    Value::Oklch(Oklch {
                        l: 0.8442,
                        c: 0.1457,
                        h: 209.29,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal02,
                    Value::Oklch(Oklch {
                        l: 0.7487,
                        c: 0.173,
                        h: 53.99,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal03,
                    Value::Oklch(Oklch {
                        l: 0.6816,
                        c: 0.2415,
                        h: 353.18,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal04,
                    Value::Oklch(Oklch {
                        l: 0.8493,
                        c: 0.2073,
                        h: 128.85,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal05,
                    Value::Oklch(Oklch {
                        l: 0.6056,
                        c: 0.2189,
                        h: 292.72,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal06,
                    Value::Oklch(Oklch {
                        l: 0.7729,
                        c: 0.1535,
                        h: 163.22,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal07,
                    Value::Oklch(Oklch {
                        l: 0.8788,
                        c: 0.1628,
                        h: 91.08,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal08,
                    Value::Oklch(Oklch {
                        l: 0.6623,
                        c: 0.1786,
                        h: 264.64,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal09,
                    Value::Oklch(Oklch {
                        l: 0.7116,
                        c: 0.1812,
                        h: 22.84,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal10,
                    Value::Oklch(Oklch {
                        l: 0.7845,
                        c: 0.1325,
                        h: 181.91,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal11,
                    Value::Oklch(Oklch {
                        l: 0.7477,
                        c: 0.207,
                        h: 322.16,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal12,
                    Value::Oklch(Oklch {
                        l: 0.9683,
                        c: 0.0069,
                        h: 247.9,
                        alpha: 1.0,
                    }),
                ),
            ],
            radius: 9.6,      // 0.6rem
            card_radius: 4.0, // 4px
            card_shadow: None,
            glow_radius: 10.0,
            hud_ticks: 1.0,
            label_tracking: 1.12,
            readout_font: ReadoutFont::RobotoMono,
        },
    },
    ThemeDef {
        slug: "midnight-violet",
        def: Definition {
            colors: &[
                (
                    Token::Background,
                    Value::Oklch(Oklch {
                        l: 0.1566,
                        c: 0.0759,
                        h: 270.51,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::BackgroundMuted,
                    Value::Oklch(Oklch {
                        l: 0.1663,
                        c: 0.0262,
                        h: 269.37,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Card,
                    Value::Oklch(Oklch {
                        l: 0.2489,
                        c: 0.0695,
                        h: 274.42,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Foreground,
                    Value::Oklch(Oklch {
                        l: 0.96,
                        c: 0.019,
                        h: 276.35,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::ForegroundMuted,
                    Value::Oklch(Oklch {
                        l: 0.7166,
                        c: 0.0559,
                        h: 276.73,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Primary,
                    Value::Oklch(Oklch {
                        l: 0.6056,
                        c: 0.2189,
                        h: 292.72,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::PrimaryForeground,
                    Value::Oklch(Oklch {
                        l: 0.1718,
                        c: 0.0688,
                        h: 284.78,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Secondary,
                    Value::Oklch(Oklch {
                        l: 0.2778,
                        c: 0.1093,
                        h: 283.05,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::SecondaryForeground,
                    Value::Oklch(Oklch {
                        l: 0.9383,
                        c: 0.0299,
                        h: 279.62,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Border,
                    Value::Oklch(Oklch {
                        l: 0.3327,
                        c: 0.0827,
                        h: 274.63,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Accent,
                    Value::Oklch(Oklch {
                        l: 0.664,
                        c: 0.2355,
                        h: 338.51,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::AccentForeground,
                    Value::Oklch(Oklch {
                        l: 0.1755,
                        c: 0.0472,
                        h: 342.88,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Success,
                    Value::Oklch(Oklch {
                        l: 0.7729,
                        c: 0.1535,
                        h: 163.22,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Warning,
                    Value::Oklch(Oklch {
                        l: 0.8369,
                        c: 0.1644,
                        h: 84.43,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Destructive,
                    Value::Oklch(Oklch {
                        l: 0.7192,
                        c: 0.169,
                        h: 13.43,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::ChartGrid,
                    Value::Oklch(Oklch {
                        l: 0.3327,
                        c: 0.0827,
                        h: 274.63,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::ChartLine,
                    Value::Oklch(Oklch {
                        l: 0.6056,
                        c: 0.2189,
                        h: 292.72,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::ChartAccent,
                    Value::Oklch(Oklch {
                        l: 0.664,
                        c: 0.2355,
                        h: 338.51,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::ChartGlow,
                    Value::Oklch(Oklch {
                        l: 0.6056,
                        c: 0.2189,
                        h: 292.72,
                        alpha: 0.4,
                    }),
                ),
                (
                    Token::Ordinal01,
                    Value::Oklch(Oklch {
                        l: 0.6056,
                        c: 0.2189,
                        h: 292.72,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal02,
                    Value::Oklch(Oklch {
                        l: 0.664,
                        c: 0.2355,
                        h: 338.51,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal03,
                    Value::Oklch(Oklch {
                        l: 0.8442,
                        c: 0.1457,
                        h: 209.29,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal04,
                    Value::Oklch(Oklch {
                        l: 0.8131,
                        c: 0.165,
                        h: 75.04,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal05,
                    Value::Oklch(Oklch {
                        l: 0.7729,
                        c: 0.1535,
                        h: 163.22,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal06,
                    Value::Oklch(Oklch {
                        l: 0.7116,
                        c: 0.1812,
                        h: 22.84,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal07,
                    Value::Oklch(Oklch {
                        l: 0.7137,
                        c: 0.1434,
                        h: 254.62,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal08,
                    Value::Oklch(Oklch {
                        l: 0.8493,
                        c: 0.2073,
                        h: 128.85,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal09,
                    Value::Oklch(Oklch {
                        l: 0.7487,
                        c: 0.173,
                        h: 53.99,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal10,
                    Value::Oklch(Oklch {
                        l: 0.7845,
                        c: 0.1325,
                        h: 181.91,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal11,
                    Value::Oklch(Oklch {
                        l: 0.9683,
                        c: 0.0069,
                        h: 247.9,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal12,
                    Value::Oklch(Oklch {
                        l: 0.908,
                        c: 0.1757,
                        h: 98.54,
                        alpha: 1.0,
                    }),
                ),
            ],
            radius: 9.6,      // 0.6rem
            card_radius: 9.6, // var(--radius)
            card_shadow: None,
            glow_radius: 14.0,
            hud_ticks: 0.0,
            label_tracking: 1.12,
            readout_font: ReadoutFont::RobotoMono,
        },
    },
    ThemeDef {
        slug: "clean-teal",
        def: Definition {
            colors: &[
                (
                    Token::Background,
                    Value::Oklch(Oklch {
                        l: 0.992,
                        c: 0.0034,
                        h: 67.78,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::BackgroundMuted,
                    Value::Oklch(Oklch {
                        l: 0.9478,
                        c: 0.008,
                        h: 73.74,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Card,
                    Value::Oklch(Oklch {
                        l: 1.0,
                        c: 0.0,
                        h: 0.0,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Foreground,
                    Value::Oklch(Oklch {
                        l: 0.2681,
                        c: 0.0299,
                        h: 209.98,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::ForegroundMuted,
                    Value::Oklch(Oklch {
                        l: 0.5283,
                        c: 0.0245,
                        h: 218.88,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Primary,
                    Value::Oklch(Oklch {
                        l: 0.5262,
                        c: 0.0935,
                        h: 178.34,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::PrimaryForeground,
                    Value::Oklch(Oklch {
                        l: 1.0,
                        c: 0.0,
                        h: 0.0,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Secondary,
                    Value::Oklch(Oklch {
                        l: 0.941,
                        c: 0.0153,
                        h: 171.04,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::SecondaryForeground,
                    Value::Oklch(Oklch {
                        l: 0.3846,
                        c: 0.0605,
                        h: 182.1,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Border,
                    Value::Oklch(Oklch {
                        l: 0.9009,
                        c: 0.0133,
                        h: 71.32,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Accent,
                    Value::Oklch(Oklch {
                        l: 0.7001,
                        c: 0.1508,
                        h: 68.7,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::AccentForeground,
                    Value::Oklch(Oklch {
                        l: 0.2226,
                        c: 0.0327,
                        h: 85.74,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Success,
                    Value::Oklch(Oklch {
                        l: 0.5273,
                        c: 0.1371,
                        h: 150.07,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Warning,
                    Value::Oklch(Oklch {
                        l: 0.5553,
                        c: 0.1455,
                        h: 49.0,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Destructive,
                    Value::Oklch(Oklch {
                        l: 0.5771,
                        c: 0.2152,
                        h: 27.33,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::ChartGrid,
                    Value::Oklch(Oklch {
                        l: 0.9009,
                        c: 0.0133,
                        h: 71.32,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::ChartLine,
                    Value::Oklch(Oklch {
                        l: 0.5262,
                        c: 0.0935,
                        h: 178.34,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::ChartAccent,
                    Value::Oklch(Oklch {
                        l: 0.7001,
                        c: 0.1508,
                        h: 68.7,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::ChartGlow,
                    Value::Oklch(Oklch {
                        l: 0.5262,
                        c: 0.0935,
                        h: 178.34,
                        alpha: 0.4,
                    }),
                ),
                (
                    Token::Ordinal01,
                    Value::Oklch(Oklch {
                        l: 0.5262,
                        c: 0.0935,
                        h: 178.34,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal02,
                    Value::Oklch(Oklch {
                        l: 0.7001,
                        c: 0.1508,
                        h: 68.7,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal03,
                    Value::Oklch(Oklch {
                        l: 0.5106,
                        c: 0.2301,
                        h: 276.97,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal04,
                    Value::Oklch(Oklch {
                        l: 0.5771,
                        c: 0.2152,
                        h: 27.33,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal05,
                    Value::Oklch(Oklch {
                        l: 0.5876,
                        c: 0.1389,
                        h: 241.97,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal06,
                    Value::Oklch(Oklch {
                        l: 0.5575,
                        c: 0.2525,
                        h: 302.32,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal07,
                    Value::Oklch(Oklch {
                        l: 0.5273,
                        c: 0.1371,
                        h: 150.07,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal08,
                    Value::Oklch(Oklch {
                        l: 0.5916,
                        c: 0.218,
                        h: 0.58,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal09,
                    Value::Oklch(Oklch {
                        l: 0.4882,
                        c: 0.2172,
                        h: 264.38,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal10,
                    Value::Oklch(Oklch {
                        l: 0.6482,
                        c: 0.1754,
                        h: 131.68,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal11,
                    Value::Oklch(Oklch {
                        l: 0.6461,
                        c: 0.1943,
                        h: 41.12,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal12,
                    Value::Oklch(Oklch {
                        l: 0.518,
                        c: 0.2258,
                        h: 323.95,
                        alpha: 1.0,
                    }),
                ),
            ],
            radius: 9.6,      // 0.6rem
            card_radius: 9.6, // var(--radius)
            card_shadow: None,
            glow_radius: 0.0,
            hud_ticks: 0.0,
            label_tracking: 1.12,
            readout_font: ReadoutFont::RobotoMono,
        },
    },
    ThemeDef {
        slug: "educational-warm",
        def: Definition {
            colors: &[
                (
                    Token::Background,
                    Value::Oklch(Oklch {
                        l: 0.9945,
                        c: 0.0057,
                        h: 84.57,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::BackgroundMuted,
                    Value::Oklch(Oklch {
                        l: 0.9693,
                        c: 0.0209,
                        h: 72.15,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Card,
                    Value::Oklch(Oklch {
                        l: 1.0,
                        c: 0.0,
                        h: 0.0,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Foreground,
                    Value::Oklch(Oklch {
                        l: 0.287,
                        c: 0.0419,
                        h: 281.49,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::ForegroundMuted,
                    Value::Oklch(Oklch {
                        l: 0.4891,
                        c: 0.0274,
                        h: 277.44,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Primary,
                    Value::Oklch(Oklch {
                        l: 0.5106,
                        c: 0.195,
                        h: 276.97,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::PrimaryForeground,
                    Value::Oklch(Oklch {
                        l: 1.0,
                        c: 0.0,
                        h: 0.0,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Secondary,
                    Value::Oklch(Oklch {
                        l: 0.939,
                        c: 0.0163,
                        h: 278.5,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::SecondaryForeground,
                    Value::Oklch(Oklch {
                        l: 0.287,
                        c: 0.0419,
                        h: 281.49,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Border,
                    Value::Oklch(Oklch {
                        l: 0.9095,
                        c: 0.0236,
                        h: 82.12,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Accent,
                    Value::Oklch(Oklch {
                        l: 0.7049,
                        c: 0.1867,
                        h: 47.6,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::AccentForeground,
                    Value::Oklch(Oklch {
                        l: 0.3239,
                        c: 0.06,
                        h: 47.6,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Success,
                    Value::Oklch(Oklch {
                        l: 0.6002,
                        c: 0.1038,
                        h: 184.7,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Warning,
                    Value::Oklch(Oklch {
                        l: 0.6658,
                        c: 0.1574,
                        h: 58.32,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Destructive,
                    Value::Oklch(Oklch {
                        l: 0.5771,
                        c: 0.18,
                        h: 27.33,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::ChartGrid,
                    Value::Oklch(Oklch {
                        l: 0.9095,
                        c: 0.0236,
                        h: 82.12,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::ChartLine,
                    Value::Oklch(Oklch {
                        l: 0.5106,
                        c: 0.195,
                        h: 276.97,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::ChartAccent,
                    Value::Oklch(Oklch {
                        l: 0.7049,
                        c: 0.1867,
                        h: 47.6,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::ChartGlow,
                    Value::Oklch(Oklch {
                        l: 0.5106,
                        c: 0.195,
                        h: 276.97,
                        alpha: 0.4,
                    }),
                ),
                (
                    Token::Ordinal01,
                    Value::Oklch(Oklch {
                        l: 0.5106,
                        c: 0.2301,
                        h: 276.97,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal02,
                    Value::Oklch(Oklch {
                        l: 0.7049,
                        c: 0.1867,
                        h: 47.6,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal03,
                    Value::Oklch(Oklch {
                        l: 0.6002,
                        c: 0.1038,
                        h: 184.7,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal04,
                    Value::Oklch(Oklch {
                        l: 0.5771,
                        c: 0.2152,
                        h: 27.33,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal05,
                    Value::Oklch(Oklch {
                        l: 0.5538,
                        c: 0.1207,
                        h: 66.44,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal06,
                    Value::Oklch(Oklch {
                        l: 0.5413,
                        c: 0.2466,
                        h: 293.01,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal07,
                    Value::Oklch(Oklch {
                        l: 0.5273,
                        c: 0.1371,
                        h: 150.07,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal08,
                    Value::Oklch(Oklch {
                        l: 0.5461,
                        c: 0.2152,
                        h: 262.88,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal09,
                    Value::Oklch(Oklch {
                        l: 0.5916,
                        c: 0.218,
                        h: 0.58,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal10,
                    Value::Oklch(Oklch {
                        l: 0.5322,
                        c: 0.1405,
                        h: 131.59,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal11,
                    Value::Oklch(Oklch {
                        l: 0.5198,
                        c: 0.0936,
                        h: 223.13,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal12,
                    Value::Oklch(Oklch {
                        l: 0.518,
                        c: 0.2258,
                        h: 323.95,
                        alpha: 1.0,
                    }),
                ),
            ],
            radius: 9.6,       // 0.6rem
            card_radius: 18.0, // 18px
            card_shadow: Some(ShadowDefinition {
                offset: [0.0, 6.0],
                blur: 18.0,
                color: Value::Mix(Mix {
                    a: Token::AccentForeground,
                    p: 12.0,
                    b: None,
                }),
            }),
            glow_radius: 0.0,
            hud_ticks: 0.0,
            label_tracking: 1.12,
            readout_font: ReadoutFont::RobotoMono,
        },
    },
    ThemeDef {
        slug: "sage-editorial",
        def: Definition {
            colors: &[
                (
                    Token::Background,
                    Value::Oklch(Oklch {
                        l: 0.9813,
                        c: 0.004,
                        h: 106.47,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::BackgroundMuted,
                    Value::Oklch(Oklch {
                        l: 0.9502,
                        c: 0.0054,
                        h: 117.93,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Card,
                    Value::Oklch(Oklch {
                        l: 0.9943,
                        c: 0.0028,
                        h: 84.56,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Foreground,
                    Value::Oklch(Oklch {
                        l: 0.2408,
                        c: 0.0119,
                        h: 145.19,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::ForegroundMuted,
                    Value::Oklch(Oklch {
                        l: 0.4996,
                        c: 0.0179,
                        h: 145.3,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Primary,
                    Value::Oklch(Oklch {
                        l: 0.4352,
                        c: 0.0769,
                        h: 149.96,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::PrimaryForeground,
                    Value::Oklch(Oklch {
                        l: 0.9813,
                        c: 0.004,
                        h: 106.47,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Secondary,
                    Value::Oklch(Oklch {
                        l: 0.9102,
                        c: 0.0177,
                        h: 131.88,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::SecondaryForeground,
                    Value::Oklch(Oklch {
                        l: 0.2408,
                        c: 0.0119,
                        h: 145.19,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Border,
                    Value::Oklch(Oklch {
                        l: 0.8683,
                        c: 0.0042,
                        h: 121.57,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Accent,
                    Value::Oklch(Oklch {
                        l: 0.5655,
                        c: 0.1453,
                        h: 44.65,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::AccentForeground,
                    Value::Oklch(Oklch {
                        l: 0.9943,
                        c: 0.0028,
                        h: 84.56,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Success,
                    Value::Oklch(Oklch {
                        l: 0.5266,
                        c: 0.1168,
                        h: 149.47,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Warning,
                    Value::Oklch(Oklch {
                        l: 0.5446,
                        c: 0.1159,
                        h: 70.23,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Destructive,
                    Value::Oklch(Oklch {
                        l: 0.5013,
                        c: 0.1783,
                        h: 28.7,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::ChartGrid,
                    Value::Oklch(Oklch {
                        l: 0.8683,
                        c: 0.0042,
                        h: 121.57,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::ChartLine,
                    Value::Oklch(Oklch {
                        l: 0.4352,
                        c: 0.0769,
                        h: 149.96,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::ChartAccent,
                    Value::Oklch(Oklch {
                        l: 0.5655,
                        c: 0.1453,
                        h: 44.65,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::ChartGlow,
                    Value::Oklch(Oklch {
                        l: 0.4352,
                        c: 0.0769,
                        h: 149.96,
                        alpha: 0.4,
                    }),
                ),
                (
                    Token::Ordinal01,
                    Value::Oklch(Oklch {
                        l: 0.4352,
                        c: 0.0769,
                        h: 149.96,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal02,
                    Value::Oklch(Oklch {
                        l: 0.5655,
                        c: 0.1453,
                        h: 44.65,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal03,
                    Value::Oklch(Oklch {
                        l: 0.4244,
                        c: 0.1809,
                        h: 265.64,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal04,
                    Value::Oklch(Oklch {
                        l: 0.5538,
                        c: 0.1207,
                        h: 66.44,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal05,
                    Value::Oklch(Oklch {
                        l: 0.5109,
                        c: 0.0861,
                        h: 186.39,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal06,
                    Value::Oklch(Oklch {
                        l: 0.4907,
                        c: 0.2412,
                        h: 292.58,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal07,
                    Value::Oklch(Oklch {
                        l: 0.5322,
                        c: 0.1405,
                        h: 131.59,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal08,
                    Value::Oklch(Oklch {
                        l: 0.5143,
                        c: 0.1978,
                        h: 16.93,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal09,
                    Value::Oklch(Oklch {
                        l: 0.45,
                        c: 0.0771,
                        h: 224.28,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal10,
                    Value::Oklch(Oklch {
                        l: 0.4519,
                        c: 0.1922,
                        h: 324.59,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal11,
                    Value::Oklch(Oklch {
                        l: 0.4732,
                        c: 0.1247,
                        h: 46.2,
                        alpha: 1.0,
                    }),
                ),
                (
                    Token::Ordinal12,
                    Value::Oklch(Oklch {
                        l: 0.4455,
                        c: 0.0374,
                        h: 257.28,
                        alpha: 1.0,
                    }),
                ),
            ],
            radius: 9.6,      // 0.6rem
            card_radius: 9.6, // var(--radius)
            card_shadow: None,
            glow_radius: 0.0,
            hud_ticks: 0.0,
            label_tracking: 1.12,
            readout_font: ReadoutFont::StixTwoText,
        },
    },
];
