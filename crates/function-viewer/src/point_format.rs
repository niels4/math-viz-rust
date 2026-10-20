//! The Points card's number formatting: two-decimal readouts with a true minus,
//! change badges, typed-field parsing, and the fixed/scientific readout split.
//!
//! Ported from
//! `src/components/views/FunctionViewer/SideBar/PointsSection/format.ts` at
//! `96f1e84`. All digit work goes through `mv-numfmt` (exact JS `toFixed` /
//! `toPrecision`); `round2` is `Number(v.toFixed(2))` nested as the source
//! nests it, never `round_to` (whose ties go away from zero while `toFixed`
//! rounds them up).

/// The minus the readouts print: U+2212, kept as bytes.
const MINUS: &str = "\u{2212}";

/// P's x snaps to this step under the pointer (`QUANTUM`).
pub const QUANTUM: f64 = 0.01;

/// Values at or above this magnitude print in scientific notation.
const SCIENTIFIC_FROM: f64 = 1e5;

/// Snaps `v` to [`QUANTUM`] without float noise (`quantize`).
///
/// `Number(sign(v) · round(|v| / QUANTUM) · QUANTUM).toPrecision(12)) || 0`:
/// the rounding runs on the non-negative quotient, so Rust's `round` agrees
/// with `Math.round`; the trailing `|| 0` maps `NaN` and either zero to
/// `0.0`, while infinities pass through as the source's do.
#[must_use]
pub fn quantize(v: f64) -> f64 {
    if v.is_nan() {
        return 0.0;
    }
    let snapped = mv_numfmt::precision(v.signum() * (v.abs() / QUANTUM).round() * QUANTUM, 12);
    if snapped == 0.0 { 0.0 } else { snapped }
}

/// `v` with two decimals and a true minus (`formatNumber`): `|v|.toFixed(2)`,
/// prefixed with U+2212 when `v` is negative — except when the digits read
/// `"0.00"`, so `-0.001` prints `"0.00"`, never `"−0.00"`.
#[must_use]
pub fn format_number(v: f64) -> String {
    let digits = mv_numfmt::to_fixed(v.abs(), 2);
    if v < 0.0 && digits != "0.00" {
        format!("{MINUS}{digits}")
    } else {
        digits
    }
}

/// A signed change for the moved badge (`formatChange`): `"+"` before a
/// positive value that doesn't round to zero, else the plain readout.
#[must_use]
pub fn format_change(v: f64) -> String {
    let text = format_number(v);
    if v > 0.0 && text != "0.00" {
        format!("+{text}")
    } else {
        text
    }
}

/// Reads a number typed with either minus (`parseNumber`): trims, maps U+2212
/// to `-`, accepts `^[-+]?(\d+\.?\d*|\.\d+)(e[-+]?\d+)?$`
/// case-insensitively and finite values only. Anything else — `""`, `"abc"`,
/// `"1.0.4"`, `"0x10"`, `"Infinity"`, `"1e400"` — is `None`.
#[must_use]
pub fn parse_number(text: &str) -> Option<f64> {
    let s = text.trim().replace(MINUS, "-");
    if !matches_number(&s) {
        return None;
    }
    // The regex gate already rejected every spelling `parse` would widen
    // (`"inf"`, `"NaN"`, hex): what reaches here is digits with an optional
    // point and exponent, so a failure only guards overflow to infinity.
    let n: f64 = s.parse().ok()?;
    n.is_finite().then_some(n)
}

/// The regex gate of [`parse_number`], hand-rolled (no regex crate):
/// `^[-+]?(\d+\.?\d*|\.\d+)(e[-+]?\d+)?$`, case-insensitive on the `e`.
fn matches_number(s: &str) -> bool {
    let b = s.as_bytes();
    let mut i = 0;
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        i += 1;
    }
    let mut int_digits = 0;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
        int_digits += 1;
    }
    if int_digits > 0 {
        if i < b.len() && b[i] == b'.' {
            i += 1;
            while i < b.len() && b[i].is_ascii_digit() {
                i += 1;
            }
        }
    } else {
        if i >= b.len() || b[i] != b'.' {
            return false;
        }
        i += 1;
        let mut frac_digits = 0;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
            frac_digits += 1;
        }
        if frac_digits == 0 {
            return false;
        }
    }
    if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        i += 1;
        if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
            i += 1;
        }
        let mut exp_digits = 0;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
            exp_digits += 1;
        }
        if exp_digits == 0 {
            return false;
        }
    }
    i == b.len()
}

/// `Number(v.toFixed(2))`, nested as the source nests it: the fixed string
/// always parses, so a failure falls back to the value itself.
fn round2(v: f64) -> f64 {
    mv_numfmt::to_fixed(v, 2).parse().unwrap_or(v)
}

/// How a readout value prints (`NumberParts`): fixed with two decimals below
/// 1e5, scientific with a two-decimal mantissa from 1e5.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NumberParts {
    /// `formatNumber(v)` with whether it shows `v` exactly.
    Fixed { text: String, exact: bool },
    /// `formatNumber` of the mantissa, the bare decimal exponent, exactness.
    Sci {
        mantissa: String,
        exponent: String,
        exact: bool,
    },
}

/// Splits `v` for the readout (`numberParts`): non-finite values and anything
/// whose two-decimal rounding stays below 1e5 print fixed (exact when the
/// rounding is within 1e-9 of `v`); the rest print scientific with the
/// exponent raised by one when the rounded mantissa reaches 10 (exact when
/// `mantissa · 10^exponent` is within a relative 1e-9 of `v`).
#[must_use]
pub fn number_parts(v: f64) -> NumberParts {
    if !v.is_finite() || round2(v).abs() < SCIENTIFIC_FROM {
        return NumberParts::Fixed {
            text: format_number(v),
            exact: (round2(v) - v).abs() < 1e-9,
        };
    }
    let mut exponent = v.abs().log10().floor() as i32;
    if (round2(v / 10_f64.powi(exponent))).abs() >= 10.0 {
        exponent += 1;
    }
    let mantissa = round2(v / 10_f64.powi(exponent));
    NumberParts::Sci {
        mantissa: format_number(mantissa),
        exponent: exponent.to_string(),
        exact: (mantissa * 10_f64.powi(exponent) - v).abs() <= 1e-9 * v.abs(),
    }
}

/// The fixed-width slots the Points readout reserves per axis (`ReadoutSlots`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReadoutSlots {
    /// The widest `formatNumber` of either x edge, in chars.
    pub x: usize,
    /// The widest `formatNumber` of either y edge, in chars.
    pub y: usize,
}

/// Fits the widest number each axis of the visible range prints
/// (`readoutSlots`): `None` for a `None` range. Widths count chars (U+2212 is
/// one UTF-16 unit in the source's `.length`, one `char` here — never bytes).
#[must_use]
pub fn readout_slots(range: Option<crate::visible_range::VisibleRange>) -> Option<ReadoutSlots> {
    let range = range?;
    let widest = |lo: f64, hi: f64| {
        format_number(lo)
            .chars()
            .count()
            .max(format_number(hi).chars().count())
    };
    Some(ReadoutSlots {
        x: widest(range.min_x, range.max_x),
        y: widest(range.min_y, range.max_y),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prints_two_decimals_with_a_true_minus_and_never_minus_zero() {
        assert_eq!(format_number(2.0), "2.00");
        assert_eq!(format_number(-1.5), "−1.50");
        assert_eq!(format_number(-0.001), "0.00");
    }

    #[test]
    fn signs_a_change_either_way_and_leaves_one_that_rounds_to_0_unsigned() {
        assert_eq!(format_change(1.25), "+1.25");
        assert_eq!(format_change(-0.5), "−0.50");
        assert_eq!(format_change(0.001), "0.00");
    }

    #[test]
    fn reads_a_number_typed_with_either_minus() {
        assert_eq!(parse_number(" 1.5 "), Some(1.5));
        assert_eq!(parse_number("−2"), Some(-2.0));
        assert_eq!(parse_number("-.25"), Some(-0.25));
        assert_eq!(parse_number("1e2"), Some(100.0));
    }

    #[test]
    fn refuses_anything_else() {
        for text in ["", "abc", "1.0.4", "0x10", "Infinity", "1e400"] {
            assert_eq!(parse_number(text), None, "{text:?}");
        }
    }

    #[test]
    fn prints_fixed_digits_below_100000() {
        assert_eq!(
            number_parts(99999.99),
            NumberParts::Fixed {
                text: "99999.99".to_owned(),
                exact: true,
            }
        );
        assert_eq!(
            number_parts(-12.345),
            NumberParts::Fixed {
                text: "−12.35".to_owned(),
                exact: false,
            }
        );
    }

    #[test]
    fn goes_scientific_from_100000_with_two_decimals_in_the_mantissa() {
        assert_eq!(
            number_parts(123456.0),
            NumberParts::Sci {
                mantissa: "1.23".to_owned(),
                exponent: "5".to_owned(),
                exact: false,
            }
        );
        assert_eq!(
            number_parts(-1e6),
            NumberParts::Sci {
                mantissa: "−1.00".to_owned(),
                exponent: "6".to_owned(),
                exact: true,
            }
        );
    }

    #[test]
    fn carries_a_mantissa_that_rounds_up_to_10_into_the_exponent() {
        assert_eq!(
            number_parts(99999.996),
            NumberParts::Sci {
                mantissa: "1.00".to_owned(),
                exponent: "5".to_owned(),
                exact: false,
            }
        );
        assert_eq!(
            number_parts(9.999e7),
            NumberParts::Sci {
                mantissa: "1.00".to_owned(),
                exponent: "8".to_owned(),
                exact: false,
            }
        );
    }

    #[test]
    fn is_one_that_shows_the_value_exactly_not_a_rounding_of_it() {
        let exact = |v: f64| match number_parts(v) {
            NumberParts::Fixed { exact, .. } | NumberParts::Sci { exact, .. } => exact,
        };
        assert!(exact(2.25));
        assert!(exact(0.1 + 0.2));
        assert!(!exact(f64::sin(1.0)));
        assert!(exact(2.5e6));
        assert!(!exact(2.505e6));
    }

    #[test]
    fn fits_the_widest_number_each_axis_of_the_visible_range_prints() {
        assert_eq!(
            readout_slots(Some(crate::visible_range::VisibleRange {
                min_x: -12.5,
                max_x: 3.0,
                min_y: -4.0,
                max_y: 150.0,
            })),
            Some(ReadoutSlots { x: 6, y: 6 })
        );
        assert_eq!(readout_slots(None), None);
    }
}
