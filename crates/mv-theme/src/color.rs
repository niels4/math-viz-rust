//! Oklch colours, `color-mix()` in Oklch, and the conversion to the space a build draws in.
//!
//! The page writes its colours as CSS `oklch()` and `color-mix(in oklch, …)`; the
//! browser resolves them, and this module repeats that maths so the port draws
//! the same bytes. A colour converts through Oklab to linear RGB in the build's
//! space, clips, and gamma-encodes with the sRGB transfer function (which
//! Display P3 shares).
//!
//! Ported from `src/theme/color.odin` (113 lines): oklch() colours, color-mix()
//! in oklch, and the conversion to the space a build draws in.

use enum_map::{Enum, EnumMap};

/// The spaces a build draws in: Display P3 wherever the surface shows it,
/// sRGB elsewhere.
///
/// Ported from `color.odin:11` (`Space :: enum u8 { sRGB, Display_P3 }`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum Space {
    Srgb,
    DisplayP3,
}

/// The words for a space, as a script's `space` line writes it.
///
/// Ported from `color.odin:18` (`SPACE_WORDS := [Space]string`, `@(rodata)`).
pub static SPACE_WORDS: EnumMap<Space, &str> = EnumMap::from_array(["srgb", "p3"]);

/// A colour in Oklch, as CSS's `oklch()` writes it: lightness 0 to 1, chroma,
/// hue in degrees, alpha.
///
/// Ported from `color.odin:24` (`Oklch :: struct { l, c, h, alpha }`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Oklch {
    pub l: f64,
    pub c: f64,
    pub h: f64,
    pub alpha: f64,
}

/// CSS's `transparent`: black at no alpha. It has no chroma, so its hue is
/// powerless and a mix takes the other colour's.
///
/// Ported from `color.odin:30` (`TRANSPARENT :: Oklch{0, 0, 0, 0}`).
pub const TRANSPARENT: Oklch = Oklch {
    l: 0.0,
    c: 0.0,
    h: 0.0,
    alpha: 0.0,
};

/// `color-mix(in oklch, a p%, b)`, p a percentage: lightness and chroma
/// premultiplied by alpha, interpolated and divided by the interpolated alpha;
/// the hue taken the shorter way round, a powerless one (no chroma) taking the
/// other colour's. A mix with transparent is therefore the colour with p % of
/// its alpha.
///
/// Ported from `color.odin:36` (`mix`).
pub fn mix(a: Oklch, p: f64, b: Oklch) -> Oklch {
    let t = p / 100.0;
    let (mut ha, mut hb) = (a.h, b.h);
    if a.c == 0.0 {
        ha = hb;
    }
    if b.c == 0.0 {
        hb = ha;
    }
    if hb - ha > 180.0 {
        ha += 360.0;
    } else if hb - ha < -180.0 {
        hb += 360.0;
    }
    let alpha = a.alpha * t + b.alpha * (1.0 - t);
    let (mut l, mut c) = (
        a.l * a.alpha * t + b.l * b.alpha * (1.0 - t),
        a.c * a.alpha * t + b.c * b.alpha * (1.0 - t),
    );
    if alpha > 0.0 {
        l /= alpha;
        c /= alpha;
    }
    Oklch {
        l,
        c,
        h: (ha * t + hb * (1.0 - t)).rem_euclid(360.0),
        alpha,
    }
}

/// Oklab to the cone responses' cube roots.
///
/// Ported from `color.odin:63` (`OKLAB_TO_LMS :: matrix[3, 3]f64`).
pub const OKLAB_TO_LMS: [[f64; 3]; 3] = [
    [1.0, 0.3963377773761749, 0.2158037573099136],
    [1.0, -0.1055613458156586, -0.0638541728258133],
    [1.0, -0.0894841775298119, -1.2914855480194092],
];

/// The cubed responses to CIE XYZ (D65).
///
/// The literals stay verbatim from CSS Color 4: several carry more digits than
/// `f64` holds, which is exactly what the reference maths has.
///
/// Ported from `color.odin:69` (`LMS_TO_XYZ :: matrix[3, 3]f64`).
#[allow(clippy::excessive_precision)]
pub const LMS_TO_XYZ: [[f64; 3]; 3] = [
    [1.2268798758459243, -0.5578149944602171, 0.2813910456659647],
    [-0.0405757452148008, 1.1122868032803170, -0.0717110580655164],
    [-0.0763729366746601, -0.4214933324022432, 1.5869240198367816],
];

/// XYZ to linear sRGB.
///
/// Ported from `color.odin:75` (`XYZ_TO_LINEAR_SRGB :: matrix[3, 3]f64`).
pub const XYZ_TO_LINEAR_SRGB: [[f64; 3]; 3] = [
    [12831.0 / 3959.0, -329.0 / 214.0, -1974.0 / 3959.0],
    [
        -851781.0 / 878810.0,
        1648619.0 / 878810.0,
        36519.0 / 878810.0,
    ],
    [705.0 / 12673.0, -2585.0 / 12673.0, 705.0 / 667.0],
];

/// XYZ to linear Display P3.
///
/// Ported from `color.odin:81` (`XYZ_TO_LINEAR_P3 :: matrix[3, 3]f64`).
pub const XYZ_TO_LINEAR_P3: [[f64; 3]; 3] = [
    [
        446124.0 / 178915.0,
        -333277.0 / 357830.0,
        -72051.0 / 178915.0,
    ],
    [-14852.0 / 17905.0, 63121.0 / 35810.0, 423.0 / 17905.0],
    [11844.0 / 330415.0, -50337.0 / 660830.0, 316169.0 / 330415.0],
];

fn mat_vec(m: &[[f64; 3]; 3], v: &[f64; 3]) -> [f64; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

/// The colour in `space`, as the page draws it: Oklab, then the space's linear
/// RGB, each channel clipped to 0..1 and gamma-encoded. Clipped, because no
/// engine maps a colour by chroma.
///
/// Ported from `color.odin:93` (`to_space`).
pub fn to_space(c: Oklch, space: Space) -> crate::theme::Color {
    let rgb = linear(c, space);
    [gamma(rgb[0]), gamma(rgb[1]), gamma(rgb[2]), c.alpha as f32]
}

/// The colour's linear-light RGB in `space`, unclipped: a channel lies outside
/// 0..1 where the colour lies outside the space.
///
/// Ported from `color.odin:101` (`linear`, private).
pub(crate) fn linear(c: Oklch, space: Space) -> [f64; 3] {
    let h = c.h.to_radians();
    let lms = mat_vec(&OKLAB_TO_LMS, &[c.l, c.c * h.cos(), c.c * h.sin()]);
    let xyz = mat_vec(
        &LMS_TO_XYZ,
        &[
            lms[0] * lms[0] * lms[0],
            lms[1] * lms[1] * lms[1],
            lms[2] * lms[2] * lms[2],
        ],
    );
    match space {
        Space::Srgb => mat_vec(&XYZ_TO_LINEAR_SRGB, &xyz),
        Space::DisplayP3 => mat_vec(&XYZ_TO_LINEAR_P3, &xyz),
    }
}

/// A linear channel, clipped, gamma-encoded: the sRGB transfer function, which
/// Display P3 shares.
///
/// Ported from `color.odin:110` (`gamma`, private).
pub(crate) fn gamma(v: f64) -> f32 {
    let c = v.clamp(0.0, 1.0);
    (if c <= 0.0031308 {
        12.92 * c
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }) as f32
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::prefs::Slug;
    use crate::theme::resolve;
    use crate::themes::DEFINITIONS;

    pub(crate) const ALL_SPACES: [Space; 2] = [Space::Srgb, Space::DisplayP3];
    pub(crate) const ALL_SLUGS: [Slug; 6] = [
        Slug::ArcticIce,
        Slug::TronCyan,
        Slug::MidnightViolet,
        Slug::CleanTeal,
        Slug::EducationalWarm,
        Slug::SageEditorial,
    ];

    fn bytes(c: crate::theme::Color) -> [i32; 3] {
        [
            (c[0] * 255.0 + 0.5) as i32,
            (c[1] * 255.0 + 0.5) as i32,
            (c[2] * 255.0 + 0.5) as i32,
        ]
    }

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn black_and_white_convert_exactly() {
        for space in ALL_SPACES {
            assert_eq!(
                to_space(
                    Oklch {
                        l: 0.0,
                        c: 0.0,
                        h: 0.0,
                        alpha: 1.0
                    },
                    space
                ),
                [0.0, 0.0, 0.0, 1.0]
            );
            let white = to_space(
                Oklch {
                    l: 1.0,
                    c: 0.0,
                    h: 0.0,
                    alpha: 1.0,
                },
                space,
            );
            for channel in white {
                assert!(
                    (channel - 1.0).abs() < 1e-6,
                    "oklch(1 0 0) gives {white:?} in {space:?}"
                );
            }
            // Out of the space, each channel is clipped, as every engine draws
            // it (no gamut mapping).
            let vivid = to_space(
                Oklch {
                    l: 0.7,
                    c: 0.4,
                    h: 30.0,
                    alpha: 1.0,
                },
                space,
            );
            assert!(
                vivid[0] == 1.0 && vivid[2] == 0.0,
                "oklch(0.7 0.4 30) gives {vivid:?} in {space:?}"
            );
        }
    }

    #[test]
    fn srgb_red_in_p3() {
        let red = Oklch {
            l: 0.6279553606145516,
            c: 0.25768330773615683,
            h: 29.2338851923426,
            alpha: 1.0,
        };
        let (srgb, p3) = (to_space(red, Space::Srgb), to_space(red, Space::DisplayP3));
        assert_eq!(
            bytes(srgb),
            [255, 0, 0],
            "in sRGB {:?}",
            srgb.map(|c| c * 255.0)
        );
        assert_eq!(
            bytes(p3),
            [234, 51, 35],
            "in Display P3 {:?}",
            p3.map(|c| c * 255.0)
        );
    }

    #[test]
    fn a_mix_with_transparent_keeps_the_colour_and_scales_its_alpha() {
        let color = Oklch {
            l: 0.7472,
            c: 0.0924,
            h: 166.32,
            alpha: 1.0,
        };
        for p in [12.0, 18.0, 24.0, 26.0, 32.0, 60.0] {
            let mixed = mix(color, p, TRANSPARENT);
            assert!(
                near(mixed.l, color.l)
                    && near(mixed.c, color.c)
                    && near(mixed.h, color.h)
                    && near(mixed.alpha, p / 100.0),
                "{p}% gives {mixed:?}"
            );
        }
        let half = mix(
            Oklch {
                l: 0.5,
                c: 0.1,
                h: 40.0,
                alpha: 0.5,
            },
            50.0,
            TRANSPARENT,
        );
        assert!(
            near(half.l, 0.5) && near(half.alpha, 0.25),
            "gives {half:?}"
        );
    }

    #[test]
    fn hues_mix_the_shorter_way_round() {
        let across = mix(
            Oklch {
                l: 0.5,
                c: 0.1,
                h: 350.0,
                alpha: 1.0,
            },
            50.0,
            Oklch {
                l: 0.5,
                c: 0.1,
                h: 10.0,
                alpha: 1.0,
            },
        );
        assert!(near(across.h, 0.0), "350 and 10 give {:?}", across.h);
        let back = mix(
            Oklch {
                l: 0.5,
                c: 0.1,
                h: 10.0,
                alpha: 1.0,
            },
            25.0,
            Oklch {
                l: 0.5,
                c: 0.1,
                h: 340.0,
                alpha: 1.0,
            },
        );
        assert!(near(back.h, 347.5), "10 at 25% and 340 give {:?}", back.h);
        // A colour with no chroma has no hue to give: the other's is kept.
        let grey = mix(
            Oklch {
                l: 0.5,
                c: 0.1,
                h: 120.0,
                alpha: 1.0,
            },
            50.0,
            Oklch {
                l: 1.0,
                c: 0.0,
                h: 0.0,
                alpha: 1.0,
            },
        );
        assert!(
            near(grey.h, 120.0) && near(grey.l, 0.75) && near(grey.c, 0.05),
            "gives {grey:?}"
        );
    }

    #[test]
    fn every_token_of_every_theme_resolves() {
        use crate::prefs::SLUG_WORDS;
        use enum_map::Enum as _;
        for slug in ALL_SLUGS {
            let def = &DEFINITIONS
                .iter()
                .find(|t| t.slug == SLUG_WORDS[slug])
                .unwrap_or_else(|| panic!("no theme for {slug:?}"))
                .def;
            assert_eq!(def.colors.len(), crate::themes::Token::LENGTH);
            for space in ALL_SPACES {
                let theme = resolve(slug, space);
                assert_eq!(theme.space, space);
                for color in theme.colors.values() {
                    let ok = color.iter().all(|c| !c.is_nan() && *c >= 0.0 && *c <= 1.0);
                    assert!(ok, "{slug:?} {color:?} resolves out of range in {space:?}");
                }
            }
        }
    }
}
