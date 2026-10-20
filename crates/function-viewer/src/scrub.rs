//! The scrub control's value maths: how far a drag, a wheel notch or an arrow
//! key moves a transform parameter, and how the box prints it.
//!
//! Ported from
//! `src/components/views/FunctionViewer/SideBar/TransformSection/ScrubValue/scrub.ts`
//! at b8a0e1d. The drag state machine itself (`ScrubValue.tsx`) is h5-box's;
//! this module holds every pure function its tests pin by value.

use mv_numfmt::{quantize, round_to};

/// How far a shift parameter moves per dragged px (`SHIFT_PER_PX`).
pub const SHIFT_PER_PX: f64 = 0.02;

/// The per-px growth rate of a scale parameter (`SCALE_PER_PX`).
pub const SCALE_PER_PX: f64 = 0.002;

/// The drag modifier (`ScrubMode`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrubMode {
    Coarse,
    Fine,
    Snap,
}

/// The modifier from the keys held (`scrubMode`).
///
/// Plain bools: h4's rows are unmerged, so there is no key-state struct to
/// take yet.
pub fn scrub_mode(shift_key: bool, ctrl_key: bool, meta_key: bool) -> ScrubMode {
    if ctrl_key || meta_key {
        ScrubMode::Snap
    } else if shift_key {
        ScrubMode::Fine
    } else {
        ScrubMode::Coarse
    }
}

/// The value `dx` px into a drag from `from` (`scrubbed`).
///
/// A shift adds `rate·dx`; a scale multiplies by `(1+rate)^dx` and never
/// rests nearer 0 than one snap step. Fine drags at a tenth of the rate;
/// snap quantizes to a whole number (a quarter for scales).
pub fn scrubbed(from: f64, dx: f64, is_scale: bool, mode: ScrubMode) -> f64 {
    let rate = (if is_scale { SCALE_PER_PX } else { SHIFT_PER_PX })
        * (if mode == ScrubMode::Fine { 0.1 } else { 1.0 });
    let raw = if is_scale && from != 0.0 {
        from * (1.0 + rate).powf(dx)
    } else {
        from + rate * dx
    };
    let step = match mode {
        ScrubMode::Snap => {
            if is_scale {
                0.25
            } else {
                1.0
            }
        }
        ScrubMode::Fine => 0.001,
        ScrubMode::Coarse => 0.01,
    };
    let next = quantize(raw, step);
    // `next === 0 && raw !== 0`: the multiply rounded through zero.
    if is_scale && next == 0.0 && raw != 0.0 {
        raw.signum() * step
    } else {
        next
    }
}

/// The value after an arrow-key nudge by `delta` (`nudged`).
///
/// A shift adds `delta` outright; a scale moves its size, floored at 0.01 so
/// a scale never crosses or touches 0, keeping its sign.
pub fn nudged(value: f64, is_scale: bool, delta: f64) -> f64 {
    if !is_scale {
        let stepped = quantize(value + delta, 0.001);
        // `|| 0`: NaN and both zeroes read as 0.
        if stepped == 0.0 || stepped.is_nan() {
            0.0
        } else {
            stepped
        }
    } else {
        // `Math.min/max`; inputs are finite by construction, so NaN
        // propagation (where they differ) is unreachable.
        let size = f64::max(
            f64::min(value.abs(), 0.01),
            quantize(value.abs() + delta, 0.001),
        );
        if value < 0.0 { -size } else { size }
    }
}

/// The drag `dx` a wheel event with `delta_y` amounts to (`wheelDx`).
///
/// `delta_mode` is the DOM `deltaMode`: lines (1) count 16 px each, anything
/// else passes through as px — exactly the TS ternary, pages included.
pub fn wheel_dx(delta_y: f64, delta_mode: u32) -> f64 {
    -(if delta_mode == 1 {
        delta_y * 16.0
    } else {
        delta_y
    }) / 100.0
        * 15.0
}

/// The box text for a value (`shown`).
///
/// Three decimals while fine or while the value has a third decimal, else
/// two — through `round_to` plus a plain print (never `format_number`): the
/// field shows the keyboard hyphen (`toHaveValue("-0.05")`), not the
/// readout's true minus. `-0` prints as `0.00`, as `(−0).toFixed` does.
pub fn shown(v: f64, fine: bool) -> String {
    let dp = if fine || quantize(v, 0.01) != quantize(v, 0.001) {
        3
    } else {
        2
    };
    let rounded = round_to(v, dp);
    // Only an input zero prints as `0.00`: `toFixed` keeps a nonzero
    // input's sign (`(-0.0004).toFixed(2)` is `"-0.00"`).
    let plain = if v == 0.0 { 0.0 } else { rounded };
    match dp {
        3 => format!("{plain:.3}"),
        _ => format!("{plain:.2}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // `scrubMode` has no direct vitest test; the component tests exercise it
    // through the drag, so these four pin it directly (extra, no counterpart).
    #[test]
    fn mode_is_coarse_with_no_keys() {
        assert_eq!(scrub_mode(false, false, false), ScrubMode::Coarse);
    }

    #[test]
    fn mode_is_fine_with_shift() {
        assert_eq!(scrub_mode(true, false, false), ScrubMode::Fine);
    }

    #[test]
    fn mode_is_snap_with_ctrl() {
        assert_eq!(scrub_mode(false, true, false), ScrubMode::Snap);
    }

    #[test]
    fn mode_snap_wins_over_shift_and_meta_snaps() {
        assert_eq!(scrub_mode(true, true, false), ScrubMode::Snap);
        assert_eq!(scrub_mode(false, false, true), ScrubMode::Snap);
    }

    // "moves a shift 0.02 per px and writes the URL on letting go": the
    // value half (the URL write, the drag threshold and the selection lock
    // are h5-box's state machine).
    #[test]
    fn moves_a_shift_0_02_per_px_and_writes_the_url_on_letting_go() {
        assert_eq!(scrubbed(0.0, 30.0, false, ScrubMode::Coarse), 0.6);
        assert_eq!(shown(0.6, false), "0.60");
        assert_eq!(scrubbed(0.0, 50.0, false, ScrubMode::Coarse), 1.0);
        assert_eq!(shown(1.0, false), "1.00");
    }

    // "multiplies a scale, and stops it at 0.01 short of 0".
    #[test]
    fn multiplies_a_scale_and_stops_it_at_0_01_short_of_0() {
        assert_eq!(scrubbed(2.0, 347.0, true, ScrubMode::Coarse), 4.0);
        assert_eq!(shown(4.0, false), "4.00");
        assert_eq!(scrubbed(2.0, -3000.0, true, ScrubMode::Coarse), 0.01);
    }

    // "moves a tenth as fast with Shift, to a third decimal, and holds still
    // when Shift is let go": the values (the hold-still is h5-box's rebase).
    #[test]
    fn moves_a_tenth_as_fast_with_shift_to_a_third_decimal_and_holds_still_when_shift_is_let_go() {
        assert_eq!(scrubbed(0.0, 30.0, false, ScrubMode::Fine), 0.06);
        assert_eq!(shown(0.06, true), "0.060");
        assert_eq!(scrubbed(0.0, 33.0, false, ScrubMode::Fine), 0.066);
        assert_eq!(shown(0.066, false), "0.066");
        assert_eq!(scrubbed(0.066, 10.0, false, ScrubMode::Coarse), 0.27);
    }

    // "snaps a shift to a whole number as soon as Ctrl is pressed": the
    // snap-on-press value (the continued drags need the rebase state).
    #[test]
    fn snaps_a_shift_to_a_whole_number_as_soon_as_ctrl_is_pressed() {
        assert_eq!(scrubbed(0.6, 0.0, false, ScrubMode::Snap), 1.0);
        assert_eq!(shown(1.0, false), "1.00");
    }

    // "snaps a scale to a quarter with ⌘".
    #[test]
    fn snaps_a_scale_to_a_quarter_with_meta() {
        assert_eq!(scrubbed(1.0, 100.0, true, ScrubMode::Snap), 1.25);
        assert_eq!(shown(1.25, false), "1.25");
    }

    // "moves a shift as a 15 px drag per notch, and scrolls the page while
    // typing": the notch values (scroll-through while typing is h5-box's).
    #[test]
    fn moves_a_shift_as_a_15_px_drag_per_notch_and_scrolls_the_page_while_typing() {
        assert_eq!(wheel_dx(-100.0, 0), 15.0);
        assert_eq!(scrubbed(0.0, 15.0, false, ScrubMode::Coarse), 0.3);
        assert_eq!(shown(0.3, false), "0.30");
        assert_eq!(scrubbed(0.3, 15.0, false, ScrubMode::Coarse), 0.6);
        assert_eq!(scrubbed(0.6, 15.0, false, ScrubMode::Coarse), 0.9);
    }

    // "nudge a shift 0.1 from the text typed so far, and 0.01 with Shift".
    #[test]
    fn nudge_a_shift_0_1_from_the_text_typed_so_far_and_0_01_with_shift() {
        assert_eq!(nudged(2.5, false, 0.1), 2.6);
        assert_eq!(shown(2.6, false), "2.60");
        assert_eq!(nudged(2.6, false, 0.01), 2.61);
        assert_eq!(shown(2.61, false), "2.61");
    }

    // "nudge a scale's size and stop it at 0.01".
    #[test]
    fn nudge_a_scale_s_size_and_stop_it_at_0_01() {
        assert_eq!(shown(-0.05, false), "-0.05");
        assert_eq!(nudged(-0.05, true, -0.01), -0.04);
        assert_eq!(shown(-0.04, false), "-0.04");
        assert_eq!(nudged(-0.04, true, -0.1), -0.01);
        assert_eq!(nudged(-0.01, true, 0.01), -0.02);
    }

    // "puts back the value from before the box opened, over nudges and typed
    // text, and closes the box": the nudge value (restore and close are
    // h5-box's).
    #[test]
    fn puts_back_the_value_from_before_the_box_opened_over_nudges_and_typed_text_and_closes_the_box()
     {
        assert_eq!(shown(2.0, false), "2.00");
        assert_eq!(nudged(5.0, false, 0.1), 5.1);
        assert_eq!(shown(5.1, false), "5.10");
    }

    // Extra (no counterpart): the wheel's line mode and the ASCII hyphen/−0
    // rules the vitest tests only touch in passing.
    #[test]
    fn wheel_lines_count_16_px_each() {
        assert_eq!(wheel_dx(-100.0, 1), 240.0);
    }

    #[test]
    fn shown_uses_the_hyphen_and_never_negative_zero() {
        assert_eq!(shown(-2.5, false), "-2.50");
        assert_eq!(shown(-0.0, false), "0.00");
        // A nonzero value keeps its sign through a rounding to zero.
        assert_eq!(shown(-0.0004, false), "-0.00");
    }

    // Extra (no counterpart): the TS rounds the absolute value, so negative
    // exact halves snap away from zero — `mv-numfmt::quantize` does the same.
    #[test]
    fn snap_rounds_the_absolute_value() {
        assert_eq!(scrubbed(-2.5, 0.0, false, ScrubMode::Snap), -3.0);
        assert_eq!(scrubbed(-0.125, 0.0, true, ScrubMode::Snap), -0.25);
    }
}
