//! The ruler tape under a dragged scrub box: tick positions and labels in the
//! box's px, centred on the live value.
//!
//! Ported from
//! `src/components/views/FunctionViewer/SideBar/TransformSection/ScrubValue/rulerMarks.ts`
//! at b8a0e1d. `Ruler.tsx` (h5-ruler) draws the three tick paths, the labels,
//! the notch and the chevrons from this; the tape width comes from its own
//! layout effect, so this module takes `width` as a plain argument.

use mv_numfmt::{MINUS, round_to};

/// Ticks stay at least this many px apart (`TICK_GAP`).
const TICK_GAP: f64 = 3.5;

/// Labels stay at least this many px apart (`LABEL_GAP`).
const LABEL_GAP: f64 = 38.0;

/// The largest integer an f64 holds exactly (JS `Number.MAX_SAFE_INTEGER`).
const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;

/// A label's screen x and text.
#[derive(Clone, Debug, PartialEq)]
pub struct RulerLabel {
    pub x: f64,
    pub text: String,
}

/// The tape's ticks and labels (`RulerMarks`).
#[derive(Clone, Debug, PartialEq)]
pub struct RulerMarks {
    pub minor: Vec<f64>,
    pub mid: Vec<f64>,
    pub major: Vec<f64>,
    pub labels: Vec<RulerLabel>,
}

/// The value-to-px tape both lattices share: screen x of plane value `u`,
/// the visible value range, and the px per plane unit at the tape's centre.
enum Tape {
    Shift {
        value: f64,
        centre: f64,
    },
    Scale {
        size: f64,
        centre: f64,
        px_per_e: f64,
    },
}

impl Tape {
    /// Plane value `u` at screen x (`at`).
    fn at(&self, u: f64) -> f64 {
        match *self {
            Tape::Shift { value, centre } => centre + (u - value) / super::scrub::SHIFT_PER_PX,
            Tape::Scale {
                size,
                centre,
                px_per_e,
            } => centre + px_per_e * (u / size).ln(),
        }
    }

    /// The visible value range (`lo`, `hi`).
    fn bounds(&self) -> (f64, f64) {
        match *self {
            Tape::Shift { value, centre } => (
                value - centre * super::scrub::SHIFT_PER_PX,
                value + centre * super::scrub::SHIFT_PER_PX,
            ),
            Tape::Scale {
                size,
                centre,
                px_per_e,
            } => (
                size * (-centre / px_per_e).exp(),
                size * (centre / px_per_e).exp(),
            ),
        }
    }

    /// Screen px per plane unit at the tape's centre (`pxPerUnit`).
    fn px_per_unit(&self) -> f64 {
        match *self {
            Tape::Shift { .. } => 1.0 / super::scrub::SHIFT_PER_PX,
            Tape::Scale {
                size: _,
                centre: _,
                px_per_e,
            } => {
                let (_, hi) = self.bounds();
                px_per_e / hi
            }
        }
    }
}

/// The smallest 1, 2 or 5 × 10^n at or above `min` (`niceStep`).
fn nice_step(min: f64) -> f64 {
    let decade = 10f64.powf(min.log10().floor());
    let factor = if decade >= min {
        1.0
    } else if 2.0 * decade >= min {
        2.0
    } else if 5.0 * decade >= min {
        5.0
    } else {
        10.0
    };
    factor * decade
}

/// Whether `n` is a safe lattice index (JS `Number.isSafeInteger`).
fn is_safe_integer(n: f64) -> bool {
    n.is_finite() && n.fract() == 0.0 && n.abs() <= MAX_SAFE_INTEGER
}

/// The lattice indices whose multiples fall in the visible range
/// (`multiples`).
///
/// The counter stays an f64, exactly as the TS loop does, so a range too
/// large to count in tenths (the `1e20` test) falls out empty through
/// identical float ops.
fn multiples(step: f64, lo: f64, hi: f64) -> Vec<f64> {
    let mut ns = Vec::new();
    let mut n = (lo / step).ceil();
    while n * step <= hi && is_safe_integer(n) {
        ns.push(n);
        n += 1.0;
    }
    ns
}

/// Whether `u` sits on the `step` lattice (`onLattice`).
fn on_lattice(u: f64, step: f64) -> bool {
    (u / step - (u / step).round()).abs() < 1e-6
}

/// A label's text: `String(Number(u.toFixed(decimals)))`.
///
/// `round_to` plus the shortest `Display` is that value (both round the exact
/// binary value; ties past the kept decimals are unreachable — see the
/// progress file), printed plain in `[1e-6, 1e21)` and exponential with an
/// explicit `+` outside it, as ECMA-262 `Number::toString` does. The keyboard
/// hyphen becomes the true minus, first occurrence only, as the TS `replace`
/// with a string pattern does; `-0` reads as `0`.
fn label_text(u: f64, decimals: i32) -> String {
    let rounded = round_to(u, i64::from(decimals));
    if rounded == 0.0 {
        return "0".to_string();
    }
    let magnitude = rounded.abs();
    let text = if (1e-6..1e21).contains(&magnitude) {
        format!("{rounded}")
    } else {
        let exp = format!("{rounded:e}");
        match exp.find('e') {
            Some(i) if !exp[i + 1..].starts_with(['-', '+']) => {
                format!("{}e+{}", &exp[..i], &exp[i + 1..])
            }
            _ => exp,
        }
    };
    text.replacen('-', MINUS, 1)
}

/// The ruler marks for `value` on a tape `width` px wide (`rulerMarks`).
///
/// A shift lays whole numbers 50 px apart about the value; a scale lays
/// values out by ratio about the size. `width` is the measured tape width —
/// h5-ruler's layout effect reads it; the value passed in already includes
/// the drag (the tests call `rulerMarks` on the scrubbed value).
pub fn ruler_marks(value: f64, width: f64, is_scale: bool) -> RulerMarks {
    let mut marks = RulerMarks {
        minor: Vec::new(),
        mid: Vec::new(),
        major: Vec::new(),
        labels: Vec::new(),
    };
    let centre = width / 2.0;
    let tape = if is_scale {
        Tape::Scale {
            size: value.abs(),
            centre,
            px_per_e: 1.0 / super::scrub::SCALE_PER_PX.ln_1p(),
        }
    } else {
        Tape::Shift { value, centre }
    };
    let (lo, hi) = tape.bounds();
    let step = nice_step(TICK_GAP / tape.px_per_unit());
    let label_step = nice_step(LABEL_GAP / tape.px_per_unit());
    // `toFixed` takes the count; the degenerate value-0 scale tape holds
    // `Infinity` here in TS, but both steps are 0 there so the loops below
    // never run — the clamp only keeps the conversion finite.
    #[allow(clippy::cast_possible_truncation)]
    let decimals = (-label_step.log10().floor()).clamp(0.0, 100.0) as i32;
    let inside = |x: f64| x >= 0.0 && x <= width;

    for n in multiples(step, lo, hi) {
        let u = n * step;
        let x = tape.at(u);
        if inside(x) && !on_lattice(u, label_step) {
            if n % 5.0 == 0.0 {
                marks.mid.push(x);
            } else {
                marks.minor.push(x);
            }
        }
    }
    for n in multiples(label_step, lo, hi) {
        let u = n * label_step;
        let x = tape.at(u);
        if inside(x) {
            marks.major.push(x);
            marks.labels.push(RulerLabel {
                x,
                text: label_text(u, decimals),
            });
        }
    }
    marks
}

#[cfg(test)]
mod tests {
    use super::*;

    const TAPE_WIDTH: f64 = 328.0;

    /// The vitest `rounded` helper: tick positions to 2 dp.
    fn rounded(xs: &[f64]) -> Vec<f64> {
        xs.iter().map(|x| round_to(*x, 2)).collect()
    }

    fn texts(labels: &[RulerLabel]) -> Vec<&str> {
        labels.iter().map(|label| label.text.as_str()).collect()
    }

    mod shift_ruler {
        use super::*;

        // "puts whole numbers 50 px apart, labelled, with the value at the
        // centre".
        #[test]
        fn puts_whole_numbers_50_px_apart_labelled_with_the_value_at_the_centre() {
            let marks = ruler_marks(0.0, TAPE_WIDTH, false);
            assert_eq!(
                rounded(&marks.major),
                [14.0, 64.0, 114.0, 164.0, 214.0, 264.0, 314.0]
            );
            assert_eq!(
                texts(&marks.labels),
                ["\u{2212}3", "\u{2212}2", "\u{2212}1", "0", "1", "2", "3"]
            );
            assert_eq!(
                rounded(&marks.mid),
                [39.0, 89.0, 139.0, 189.0, 239.0, 289.0]
            );
            assert_eq!(rounded(&marks.minor)[..4], [4.0, 9.0, 19.0, 24.0]);
        }

        // "slides its marks as far as the pointer moves, the other way".
        #[test]
        fn slides_its_marks_as_far_as_the_pointer_moves_the_other_way() {
            let before = ruler_marks(0.6, TAPE_WIDTH, false);
            assert_eq!(
                rounded(&before.major),
                [34.0, 84.0, 134.0, 184.0, 234.0, 284.0]
            );

            let after = ruler_marks(
                crate::scrub::scrubbed(0.6, 10.0, false, crate::scrub::ScrubMode::Coarse),
                TAPE_WIDTH,
                false,
            );
            assert_eq!(
                rounded(&after.major),
                [24.0, 74.0, 124.0, 174.0, 224.0, 274.0, 324.0]
            );
            assert_eq!(
                texts(&after.labels),
                ["\u{2212}2", "\u{2212}1", "0", "1", "2", "3", "4"]
            );
        }

        // "draws nothing for a shift too large to count in tenths".
        #[test]
        fn draws_nothing_for_a_shift_too_large_to_count_in_tenths() {
            assert_eq!(
                ruler_marks(1e20, TAPE_WIDTH, false),
                RulerMarks {
                    minor: Vec::new(),
                    mid: Vec::new(),
                    major: Vec::new(),
                    labels: Vec::new(),
                }
            );
        }
    }

    mod scale_ruler {
        use super::*;

        // "lays values out by ratio, so 1.5 sits as far left of 2 as 0.75
        // of 1".
        #[test]
        fn lays_values_out_by_ratio_so_1_5_sits_as_far_left_of_2_as_0_75_of_1() {
            let one = ruler_marks(1.0, TAPE_WIDTH, true);
            assert_eq!(texts(&one.labels), ["0.8", "1", "1.2"]);
            assert_eq!(one.minor.len() + one.mid.len() + one.major.len(), 66);

            let two = ruler_marks(2.0, TAPE_WIDTH, true);
            assert_eq!(texts(&two.labels), ["1.5", "2", "2.5"]);
            assert_eq!(
                rounded(&two.labels.iter().map(|label| label.x).collect::<Vec<_>>()),
                [20.02, 164.0, 275.68]
            );
            assert!(rounded(&one.mid).contains(&20.02));
        }

        // "reads a negative scale's size".
        #[test]
        fn reads_a_negative_scale_s_size() {
            assert_eq!(
                ruler_marks(-2.0, TAPE_WIDTH, true),
                ruler_marks(2.0, TAPE_WIDTH, true)
            );
        }
    }

    // Extra (no counterpart): the degenerate value-0 scale tape is empty
    // rather than NaN, and far-out labels keep JS `String(Number())` form.
    #[test]
    fn scale_at_zero_draws_nothing() {
        let marks = ruler_marks(0.0, TAPE_WIDTH, true);
        assert!(marks.minor.is_empty() && marks.mid.is_empty());
        assert!(marks.major.is_empty() && marks.labels.is_empty());
    }

    #[test]
    fn far_out_labels_keep_js_number_form() {
        assert_eq!(label_text(8e299, 0), "8e+299");
        // The TS `replace` swaps the first hyphen, which for a positive
        // exponent is the exponent's own.
        assert_eq!(label_text(8e-10, 10), "8e\u{2212}10");
        assert_eq!(label_text(-5e-7, 7), "\u{2212}5e-7");
        assert_eq!(label_text(-0.0, 1), "0");
    }
}
