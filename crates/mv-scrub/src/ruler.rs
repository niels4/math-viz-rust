//! Ported from `src/scrub/ruler.odin` (192 lines): the jog ruler's tape marks.
//!
//! The tape slides under a fixed index at its centre, so the value under the
//! index is the current one.

use crate::scrub::{Kind, STEP};

/// The tape's height (FV_TAPE_H).
///
/// Ported from `ruler.odin:16` (`RULER_HEIGHT :: 32`).
pub const RULER_HEIGHT: f64 = 32.0;

/// A tenth tick's length from the top edge.
///
/// Ported from `ruler.odin:19` (`TICK_MINOR :: 5`).
pub const TICK_MINOR: f64 = 5.0;

/// A half tick's length from the top edge.
///
/// Ported from `ruler.odin:20` (`TICK_MID :: 8`).
pub const TICK_MID: f64 = 8.0;

/// A labelled tick's length from the top edge.
///
/// Ported from `ruler.odin:21` (`TICK_MAJOR :: 11`).
pub const TICK_MAJOR: f64 = 11.0;

/// Labels: Roboto Mono 12, monospaced, so a label's width is its length × 0.6 em.
///
/// Ported from `ruler.odin:24` (`LABEL_CHAR_PX :: 12 * 0.6`).
pub const LABEL_CHAR_PX: f64 = 12.0 * 0.6;

/// The labels' top, from the tape's top edge.
///
/// Ported from `ruler.odin:26` (`LABEL_TOP :: 14`).
pub const LABEL_TOP: f64 = 14.0;

/// No label within this of the index.
///
/// Ported from `ruler.odin:29` (`LABEL_CLEAR_INDEX :: 16`, file-private).
const LABEL_CLEAR_INDEX: f64 = 16.0;

/// No label within this of either end, where the chevrons sit.
///
/// Ported from `ruler.odin:31` (`LABEL_CLEAR_END :: 20`, file-private).
const LABEL_CLEAR_END: f64 = 20.0;

/// A label keeps this far from the one before.
///
/// Ported from `ruler.odin:34` (`LABEL_GAP :: 6`, file-private).
const LABEL_GAP: f64 = 6.0;

/// A label on the tape: where its value sits, its centred left edge, its text.
///
/// Ported from `ruler.odin:37` (`Ruler_Label :: struct { x, left, text }`).
#[derive(Clone, Debug, PartialEq)]
pub struct RulerLabel {
    /// Where the label's value sits on the tape.
    pub x: f64,
    /// The label's left edge, whole px, centred on x as Figma places it.
    pub left: f64,
    pub text: String,
}

/// Ticks, labels and the home notch for a tape. Positions are px from the
/// tape's left edge.
///
/// Ported from `ruler.odin:44` (`Ruler_Marks`).
#[derive(Clone, Debug, PartialEq)]
pub struct RulerMarks {
    pub minor: Vec<f64>,
    pub mid: Vec<f64>,
    /// Ticks on the label lattice, labelled or not.
    pub major: Vec<f64>,
    pub labels: Vec<RulerLabel>,
    /// The default's notch (0 or 1), where a double-click returns; none when off
    /// the tape or under the index.
    pub home: Option<f64>,
}

/// Figma rounds an auto-width text box up to whole px; the text's length in characters.
///
/// Ported from `ruler.odin:54` (`label_width`, file-private).
fn label_width(text: &str) -> f64 {
    (text.chars().count() as f64 * LABEL_CHAR_PX).ceil()
}

/// The smallest step in the list that is at least `min`, and past its end the
/// 1-2-5 steps beyond it.
///
/// Ported from `ruler.odin:61` (`nice`, file-private).
fn nice(min: f64, list: &[f64]) -> f64 {
    for &v in list {
        if v >= min {
            return v;
        }
    }
    nice_step(min, list[list.len() - 1])
}

/// Whether `u` sits on the `q` lattice.
///
/// Ported from `ruler.odin:71` (`on_lattice`, file-private).
fn on_lattice(u: f64, q: f64) -> bool {
    (u / q - (u / q).round()).abs() < 1e-6
}

/// Past 2^53 a tenth can't be told from the next, and n += 1 stops moving.
///
/// Ported from `ruler.odin:77` (`is_safe_integer`, file-private).
fn is_safe_integer(n: f64) -> bool {
    n.floor() == n && n.abs() <= 9007199254740991.0
}

/// Ticks, labels and the home notch for a tape `width` px wide showing `value`
/// under its centre.
///
/// Ported from `ruler.odin:82` (`ruler_marks`; the allocator parameter is gone:
/// the owned `RulerMarks` return replaces the caller's allocator).
/// Appends the label for the value `u` at tape position `x`, unless it would sit
/// too close to the index, the chevrons, or the label before it.
fn push_label(marks: &mut RulerMarks, x: f64, u: f64, c: f64, width: f64) {
    let text = mv_numfmt::format_mark(u, mv_numfmt::DISPLAY_DP);
    let w = label_width(&text);
    let left = (x - w / 2.0).round();
    let n = marks.labels.len();
    let clear_of_before = n == 0
        || left >= marks.labels[n - 1].left + label_width(&marks.labels[n - 1].text) + LABEL_GAP;
    if left >= LABEL_CLEAR_END
        && left + w <= width - LABEL_CLEAR_END
        && (x - c).abs() >= LABEL_CLEAR_INDEX
        && clear_of_before
    {
        marks.labels.push(RulerLabel { x, left, text });
    }
}

pub fn ruler_marks(kind: Kind, value: f64, width: f64) -> RulerMarks {
    let mut marks = RulerMarks {
        minor: Vec::new(),
        mid: Vec::new(),
        major: Vec::new(),
        labels: Vec::new(),
        home: None,
    };
    let c = width / 2.0;
    let inside = |x: f64| x >= 1.0 && x <= width - 1.0;

    if kind == Kind::Additive {
        let ppu = 1.0 / STEP[Kind::Additive];
        let last = ((value + c / ppu) * 10.0).floor();
        // `n` counts tenths, so the loop walks whole floats and float error can't
        // drift the lattice; it ends where tenths stop being safe integers.
        let mut n = ((value - c / ppu) * 10.0).ceil();
        while is_safe_integer(n) && n <= last {
            let u = n / 10.0;
            let x = c + (u - value) * ppu;
            if inside(x) {
                // `math.mod(n, k) == 0` is exact here: `n` is a whole float, so the
                // remainder is only ever an exact zero or not.
                if n % 10.0 == 0.0 {
                    marks.major.push(x);
                    push_label(&mut marks, x, u, c, width);
                } else if n % 5.0 == 0.0 {
                    marks.mid.push(x);
                } else {
                    marks.minor.push(x);
                }
                if n == 0.0 && (x - c).abs() > 2.0 {
                    marks.home = Some(x);
                }
            }
            n += 1.0;
        }
        return marks;
    }

    // The log ruler: ticks no closer than about 3.5 px at the dense end, labels ≥ 38 px apart.
    let size = value.abs();
    if size > 0.0 && !size.is_infinite() {
        let k = 1.0 / (1.0 + STEP[Kind::Multiplicative]).ln();
        let lo = size * (-c / k).exp();
        let hi = size * (c / k).exp();
        if !hi.is_infinite() {
            let step = nice(3.5 * hi / k, &[0.01, 0.02, 0.05, 0.1, 0.2, 0.5, 1.0]);
            let mid = nice(step * 4.9, &[0.05, 0.1, 0.2, 0.25, 0.5, 1.0, 2.0, 5.0]);
            let lab = nice(
                38.0 * hi / k,
                &[0.01, 0.02, 0.05, 0.1, 0.2, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0],
            );
            let at = |u: f64| c + k * (u / size).ln();
            let mut n = (lo / step).ceil();
            while n * step <= hi + 1e-9 {
                let u = (n * step * 1e6).round() / 1e6;
                let x = at(u);
                // Ticks on the label lattice are drawn as labelled ones below.
                if u > 0.0 && !on_lattice(u, lab) && inside(x) {
                    if on_lattice(u, mid) {
                        marks.mid.push(x);
                    } else {
                        marks.minor.push(x);
                    }
                }
                n += 1.0;
            }
            // Labels on their own lattice: 0.25 never lands on a 0.02 tick grid.
            let mut n = (lo / lab).ceil();
            while n * lab <= hi + 1e-9 {
                let u = (n * lab * 1e6).round() / 1e6;
                let x = at(u);
                if u > 0.0 && inside(x) {
                    marks.major.push(x);
                    push_label(&mut marks, x, u, c, width);
                }
                n += 1.0;
            }
            if lo <= 1.0 && hi >= 1.0 && (at(1.0) - c).abs() > 2.0 {
                marks.home = Some(at(1.0));
            }
        }
    }
    marks
}

/// The smallest 1-2-5 number of at least `min`, never below `floor`.
///
/// Ported from `ruler.odin:181` (`nice_step`).
pub fn nice_step(min: f64, floor: f64) -> f64 {
    if min <= floor {
        return floor;
    }
    let decade = 10f64.powf(min.log10().floor());
    for f in [1.0, 2.0, 5.0] {
        if f * decade >= min {
            return f * decade;
        }
    }
    10.0 * decade
}

// Ported from `src/scrub/ruler_test.odin`: ui/scrubRuler.test.ts, case by case.
// The transform grid's columns are 204 px wide (fvTransformGrid), and every
// expected position is read off the key-states snapshot
// (fv-rec-key-states.json, R2, R3, R5), relative to the tape's left edge.
#[cfg(test)]
mod tests {
    use super::*;

    const W: f64 = 204.0;

    const M: &str = mv_numfmt::MINUS;

    fn texts(marks: &RulerMarks) -> Vec<String> {
        marks.labels.iter().map(|l| l.text.clone()).collect()
    }

    fn lefts(marks: &RulerMarks) -> Vec<f64> {
        marks.labels.iter().map(|l| l.left).collect()
    }

    fn expect_texts(kind: Kind, value: f64, want: &[&str]) {
        let got = texts(&ruler_marks(kind, value, W));
        let want: Vec<String> = want.iter().map(|s| s.to_string()).collect();
        assert_eq!(got, want, "labels at {value}");
    }

    fn close_to(got: f64, want: f64, within: f64) {
        assert!((got - want).abs() < within, "{got}, want {want}");
    }

    #[test]
    fn a_linear_ruler_labels_the_whole_numbers_it_has_room_for() {
        expect_texts(Kind::Additive, 0.0, &[&(M.to_string() + "1"), "1"]);
        expect_texts(Kind::Additive, 1.0, &["0", "2"]);
        expect_texts(Kind::Additive, -1.0, &[&(M.to_string() + "2"), "0"]);
        expect_texts(Kind::Additive, 1.5, &["0", "1", "2", "3"]);
    }

    #[test]
    fn r3_k_and_h_labels_land_on_the_snapshot() {
        assert_eq!(
            lefts(&ruler_marks(Kind::Additive, 1.0, W)),
            vec![48.0, 148.0]
        );
        assert_eq!(
            lefts(&ruler_marks(Kind::Additive, -1.0, W)),
            vec![45.0, 148.0]
        );
    }

    #[test]
    fn ticks_every_tenth_at_50_px_per_unit() {
        let marks = ruler_marks(Kind::Additive, 1.0, W);
        assert_eq!(marks.major, vec![2.0, 52.0, 102.0, 152.0, 202.0]);
        assert_eq!(marks.mid, vec![27.0, 77.0, 127.0, 177.0]);
        assert_eq!(marks.minor.len(), 32);
        assert_eq!(marks.minor[0], 7.0);
    }

    #[test]
    fn notches_the_default_off_the_index() {
        assert_eq!(ruler_marks(Kind::Additive, 1.0, W).home, Some(52.0));
        assert_eq!(ruler_marks(Kind::Additive, -1.0, W).home, Some(152.0));
        assert_eq!(ruler_marks(Kind::Additive, 0.0, W).home, None);
        assert_eq!(ruler_marks(Kind::Additive, 5.0, W).home, None);
    }

    #[test]
    fn follows_a_fine_value_between_ticks() {
        let marks = ruler_marks(Kind::Additive, -1.013, W);
        close_to(marks.home.unwrap(), 152.65, 1e-10);
        expect_texts(Kind::Additive, -1.013, &[&(M.to_string() + "2"), "0"]);
    }

    #[test]
    fn a_log_ruler_labels_around_its_value() {
        expect_texts(Kind::Multiplicative, 1.0, &["0.9", "1.1"]);
        expect_texts(Kind::Multiplicative, 2.0, &["1.8", "2.2"]);
        assert_eq!(
            lefts(&ruler_marks(Kind::Multiplicative, 2.0, W)),
            vec![38.0, 139.0]
        );
    }

    #[test]
    fn a_flipped_scale_reads_its_size() {
        let a = ruler_marks(Kind::Multiplicative, -2.0, W);
        let b = ruler_marks(Kind::Multiplicative, 2.0, W);
        assert!(a.minor == b.minor && a.mid == b.mid);
        assert!(a.major == b.major && texts(&a) == texts(&b));
        assert_eq!(a.home, b.home);
    }

    #[test]
    fn r3_a_labelled_1_8_to_2_4() {
        let marks = ruler_marks(Kind::Multiplicative, 2.0, W);
        close_to(marks.major[0], 49.267, 0.005);
        close_to(marks.major[marks.major.len() - 1], 193.25, 0.005);
        close_to(marks.minor[0], 2.675, 0.005);
        close_to(marks.mid[0], 20.659, 0.005);
    }

    #[test]
    fn notches_1_on_the_tape_and_off_the_index() {
        assert_eq!(ruler_marks(Kind::Multiplicative, 1.0, W).home, None);
        assert_eq!(ruler_marks(Kind::Multiplicative, 2.0, W).home, None);
        close_to(
            ruler_marks(Kind::Multiplicative, 1.04, W).home.unwrap(),
            82.37,
            0.005,
        );
    }

    #[test]
    fn log_labels_38_px_apart_at_any_size() {
        for size in [0.01, 0.07, 0.3, 0.75, 1.5, 4.0, 12.0, 90.0] {
            let xs = ruler_marks(Kind::Multiplicative, size, W).major;
            for i in 1..xs.len() {
                assert!(
                    xs[i] - xs[i - 1] >= 38.0 - 1e-9,
                    "size {size}: {} apart",
                    xs[i] - xs[i - 1]
                );
            }
        }
    }

    #[test]
    fn nothing_for_a_size_of_0() {
        let marks = ruler_marks(Kind::Multiplicative, 0.0, W);
        assert!(marks.minor.len() + marks.mid.len() + marks.major.len() + marks.labels.len() == 0);
        assert_eq!(marks.home, None);
    }

    // A typed value can be any finite number, 10⁹ or 10³⁰⁰: the ruler keeps an ordinary ruler's few
    // marks, and every loop ends.
    #[test]
    fn huge_values_keep_a_ruler_few_marks() {
        fn ticks(m: &RulerMarks) -> usize {
            m.minor.len() + m.mid.len() + m.major.len()
        }
        for size in [1e3, 1e6, 3e6, 1e9, 1e17, 1e300] {
            for width in [W, 420.0] {
                let marks = ruler_marks(Kind::Multiplicative, size, width);
                assert!(
                    ticks(&marks) > 0 && ticks(&marks) as f64 <= width / 3.5 + 1.0,
                    "size {size}: {} ticks",
                    ticks(&marks)
                );
                for i in 1..marks.labels.len() {
                    let prev = &marks.labels[i - 1];
                    let next = &marks.labels[i];
                    assert!(next.x - prev.x >= 38.0 - 1e-9);
                    assert!(
                        next.left > prev.left + (prev.text.chars().count() as f64 * 7.2).ceil()
                    );
                }
            }
        }
        for value in [1e15, 1e17, 1e300, -1e300] {
            assert!(
                ticks(&ruler_marks(Kind::Additive, value, W)) as f64 <= W / 5.0 + 1.0,
                "shift {value}"
            );
        }
    }

    // ui/extentTicks.test.ts niceStep.
    #[test]
    fn nice_steps_are_1_2_5_from_a_floor() {
        assert_eq!(nice_step(0.19, 0.5), 0.5);
        assert_eq!(nice_step(0.6, 0.5), 1.0);
        assert_eq!(nice_step(1.51, 2.0), 2.0);
        assert_eq!(nice_step(3.98, 1.0), 5.0);
        assert_eq!(nice_step(15.1, 2.0), 20.0);
        assert_eq!(nice_step(120.0, 2.0), 200.0);
    }
}
