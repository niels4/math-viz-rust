//! Ported from `src/scrub/scrub.odin` (96 lines): relative jog maths, no min/max.
//!
//! Every gesture comes down to a horizontal pixel delta from an anchor, and the
//! next value derives from the anchor's value, never accumulated.

use enum_map::{Enum, EnumMap};

/// Shifts add (a linear ruler), scales multiply (a log ruler).
///
/// Ported from `scrub.odin:12` (`Kind :: enum u8`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum Kind {
    Additive,
    Multiplicative,
}

/// A plain drag, Shift for fine, Ctrl or ⌘ to snap, which wins over fine.
///
/// Ported from `scrub.odin:18` (`Mode :: enum u8`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum Mode {
    Coarse,
    Fine,
    Snap,
}

/// Value change per px: 0.02 units for shifts, ×1.002 per px for scales.
///
/// Ported from `scrub.odin:26` (`STEP := [Kind]f64`, `@(rodata)`).
pub static STEP: EnumMap<Kind, f64> = EnumMap::from_array([0.02, 0.002]);

/// Shift scales the rate and the lattice by this.
///
/// Ported from `scrub.odin:32` (`FINE_SCALE :: 0.1`).
pub const FINE_SCALE: f64 = 0.1;

/// Stored = shown (numfmt): a drag lands on 0.01, a fine drag on 0.001.
///
/// Ported from `scrub.odin:35` (`QUANTUM :: 0.01`).
pub const QUANTUM: f64 = 0.01;

/// Ctrl snaps (decision D9): shifts to whole numbers, scales to quarters.
///
/// Ported from `scrub.odin:39` (`SNAP_STEP := [Kind]f64`, `@(rodata)`).
pub static SNAP_STEP: EnumMap<Kind, f64> = EnumMap::from_array([1.0, 0.25]);

/// Arrow keys on a ruler (FV 07): one display step.
///
/// Ported from `scrub.odin:45` (`NUDGE_STEP :: 0.01`).
pub const NUDGE_STEP: f64 = 0.01;

/// Arrow keys on a ruler with Shift held: ten display steps.
///
/// Ported from `scrub.odin:46` (`NUDGE_STEP_SHIFT :: 0.1`).
pub const NUDGE_STEP_SHIFT: f64 = 0.1;

/// How far a press on a number box travels before it drags (px).
///
/// Ported from `scrub.odin:49` (`DRAG_THRESHOLD :: 3`).
pub const DRAG_THRESHOLD: f64 = 3.0;

/// The mode Shift/Ctrl/⌘ pick: snap wins over fine.
///
/// Ported from `scrub.odin:51` (`mode_of`).
pub fn mode_of(shift: bool, ctrl: bool, meta: bool) -> Mode {
    if ctrl || meta {
        Mode::Snap
    } else if shift {
        Mode::Fine
    } else {
        Mode::Coarse
    }
}

/// A mode's lattice: what a drag lands on.
///
/// Ported from `scrub.odin:56` (`lattice`, private).
fn lattice(kind: Kind, mode: Mode) -> f64 {
    match mode {
        Mode::Snap => SNAP_STEP[kind],
        Mode::Fine => QUANTUM * FINE_SCALE,
        Mode::Coarse => QUANTUM,
    }
}

/// The value a drag of `dx` px from `value` gives, on its mode's lattice.
///
/// Ported from `scrub.odin:68` (`delta`).
pub fn delta(value: f64, dx: f64, kind: Kind, mode: Mode) -> f64 {
    let rate = STEP[kind] * if mode == Mode::Fine { FINE_SCALE } else { 1.0 };
    // Multiplicative scrubbing can't leave 0 (0 times anything is 0), so a scale at 0 is pushed
    // additively until it is off 0. Otherwise the factor is positive, so a scale keeps its sign.
    let raw = if kind == Kind::Multiplicative && value != 0.0 {
        value * (1.0 + rate).powf(dx)
    } else {
        value + dx * rate
    };
    let step = lattice(kind, mode);
    let next = mv_numfmt::quantize(raw, step);
    // A scale never lands on 0, which would flatten the curve (D9): it stops one lattice step
    // short, on its own side.
    if kind == Kind::Multiplicative && next == 0.0 && raw != 0.0 {
        raw.signum() * step
    } else {
        next
    }
}

/// An arrow key on a ruler: `by` along the ruler's reading.
///
/// Ported from `scrub.odin:82` (`nudge`).
pub fn nudge(value: f64, kind: Kind, by: f64) -> f64 {
    if kind == Kind::Additive {
        let next = mv_numfmt::round_to(value + by, mv_numfmt::FINE_DP);
        // `0` here is +0: a shift nudged onto zero loses its negative sign.
        return if next == 0.0 { 0.0 } else { next };
    }
    let size = value.abs();
    let next = size
        .min(QUANTUM)
        .max(mv_numfmt::round_to(size + by, mv_numfmt::FINE_DP));
    if value < 0.0 { -next } else { next }
}

/// A wheel notch as scrub px: up increases.
///
/// Ported from `scrub.odin:93` (`wheel_dx`; the default `notch_px := 15.0`
/// is explicit here: Rust has no default arguments).
pub fn wheel_dx(delta_y: f64, delta_mode: i64, notch_px: f64) -> f64 {
    let d = if delta_mode == 1 {
        delta_y * 16.0
    } else {
        delta_y
    };
    (-d / 100.0) * notch_px
}

// Ported from `src/scrub/scrub_test.odin`: ui/scrub.test.ts, case by case.
#[cfg(test)]
mod tests {
    use super::*;

    // The Odin helpers carry defaults (`value := 1, dx := 0, mode := .Coarse`);
    // Rust has no default arguments, so every call site passes all three.
    fn offset(value: f64, dx: f64, mode: Mode) -> f64 {
        delta(value, dx, Kind::Additive, mode)
    }

    fn scale(value: f64, dx: f64, mode: Mode) -> f64 {
        delta(value, dx, Kind::Multiplicative, mode)
    }

    #[test]
    fn shift_is_fine_and_ctrl_snaps() {
        assert_eq!(mode_of(false, false, false), Mode::Coarse);
        assert_eq!(mode_of(true, false, false), Mode::Fine);
        assert_eq!(mode_of(false, true, false), Mode::Snap);
        assert_eq!(mode_of(true, false, true), Mode::Snap);
    }

    #[test]
    fn additive_scrubbing() {
        // 0.02 per px: 50 px per unit.
        assert_eq!(offset(1.0, 100.0, Mode::Coarse), 3.0);
        assert_eq!(offset(1.0, 50.0, Mode::Coarse), 2.0);
        // Shift scales the rate down.
        assert_eq!(offset(1.0, 100.0, Mode::Fine), 1.2);
        // It lands on 0.01, and on 0.001 when fine.
        assert_eq!(offset(0.0, 10.0, Mode::Coarse), 0.2);
        assert_eq!(offset(0.0, 0.6, Mode::Coarse), 0.01);
        assert_eq!(offset(0.0, 0.6, Mode::Fine), 0.001);
        // Ctrl snaps shifts to whole numbers.
        assert_eq!(offset(1.0, 100.0, Mode::Snap), 3.0);
        assert_eq!(offset(0.4, 0.0, Mode::Snap), 0.0);
        assert_eq!(offset(-1.6, 0.0, Mode::Snap), -2.0);
    }

    #[test]
    fn multiplicative_scrubbing() {
        // Compounds per px (1.002^px) and lands on 0.01.
        assert_eq!(scale(1.0, 100.0, Mode::Coarse), 1.22);
        assert_eq!(scale(1.0, -100.0, Mode::Coarse), 0.82);
        // On 0.001 when fine.
        assert_eq!(scale(1.0, 100.0, Mode::Fine), 1.02);
        assert_eq!(scale(1.035, 1.0, Mode::Fine), 1.035);
        // Ctrl snaps scales to quarters (D9), never to 0.
        assert_eq!(scale(0.4, 0.0, Mode::Snap), 0.5);
        assert_eq!(scale(1.3, 0.0, Mode::Snap), 1.25);
        assert_eq!(scale(-0.9, 0.0, Mode::Snap), -1.0);
        assert_eq!(scale(0.1, 0.0, Mode::Snap), 0.25);
        assert_eq!(scale(-0.1, 0.0, Mode::Snap), -0.25);
        // Never rounds a scale to 0, and keeps its sign.
        assert_eq!(scale(1.0, -10000.0, Mode::Coarse), 0.01);
        assert_eq!(scale(-2.0, -10000.0, Mode::Coarse), -0.01);
        assert_eq!(scale(1.0, -100000.0, Mode::Fine), 0.001);
        // Pushes additively out of zero.
        assert_eq!(scale(0.0, 100.0, Mode::Coarse), 0.2);
        assert_eq!(scale(0.0, -100.0, Mode::Coarse), -0.2);
    }

    #[test]
    fn nudge_steps_by_the_arrow_keys() {
        // A shift by 0.01 or 0.1, keeping a fine digit.
        assert_eq!(nudge(1.0, Kind::Additive, 0.01), 1.01);
        assert_eq!(nudge(1.0, Kind::Additive, -0.1), 0.9);
        assert_eq!(nudge(-1.013, Kind::Additive, 0.01), -1.003);
        let zero = nudge(-0.01, Kind::Additive, 0.01);
        assert!(
            zero == 0.0 && !zero.is_sign_negative(),
            "nudge(-0.01, +0.01) = {zero}, want +0"
        );
        // A scale's size, keeping its sign, never reaching 0.
        assert_eq!(nudge(2.0, Kind::Multiplicative, 0.01), 2.01);
        assert_eq!(nudge(-2.0, Kind::Multiplicative, 0.01), -2.01);
        assert_eq!(nudge(-2.0, Kind::Multiplicative, -0.1), -1.9);
        assert_eq!(nudge(0.05, Kind::Multiplicative, -0.1), 0.01);
        assert_eq!(nudge(0.01, Kind::Multiplicative, -0.01), 0.01);
        assert_eq!(nudge(-0.005, Kind::Multiplicative, -0.01), -0.005);
    }

    #[test]
    fn wheel_notches_as_scrub_px() {
        // Scroll up is positive. The Odin default `notch_px := 15` is explicit.
        assert_eq!(wheel_dx(-100.0, 0, 15.0), 15.0);
        // Line mode normalised.
        let d = wheel_dx(-3.0, 1, 15.0);
        assert!(
            (d - 7.2).abs() < 1e-10,
            "wheel_dx(-3, lines) = {d}, want 7.2"
        );
    }
}
