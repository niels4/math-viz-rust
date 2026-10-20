//! Ported from `src/motion/tokens.odin` (111 lines): the motion tokens and ease dispatch.

use std::sync::LazyLock;

use crate::bezier::{bezier_at, cubic_bezier};
use crate::spring::{Spring, SpringEase, spring_ease, spring_ease_at};

/// Hover, chip toggle, crosshair, fades.
///
/// Ported from `tokens.odin:9` (`FAST_MS :: 160`).
pub const FAST_MS: f64 = 160.0;

/// Badge and chrome entrances.
///
/// Ported from `tokens.odin:11` (`BASE_MS :: 320`).
pub const BASE_MS: f64 = 320.0;

/// The curve's draw-on (path trim 0 → 1).
///
/// Ported from `tokens.odin:13` (`DRAW_MS :: 900`).
pub const DRAW_MS: f64 = 900.0;

/// Between siblings in a staged entrance.
///
/// Ported from `tokens.odin:15` (`STAGGER_MS :: 80`).
pub const STAGGER_MS: f64 = 80.0;

/// Things leaving: Q and its row's values.
///
/// Ported from `tokens.odin:17` (`LEAVE_MS :: 120`).
pub const LEAVE_MS: f64 = 120.0;

/// Under reduced motion: fades at most this long.
///
/// Ported from `tokens.odin:20` (`REDUCED_FADE_MS :: 120`).
pub const REDUCED_FADE_MS: f64 = 120.0;

/// Value jumps and pans: critically damped, so a measurement never wobbles.
///
/// Ported from `tokens.odin:23` (`SPRING_SWEEP :: Spring{1, 170, 26}`).
pub const SPRING_SWEEP: Spring = Spring {
    mass: 1.0,
    stiffness: 170.0,
    damping: 26.0,
};

/// Things arriving (points, badges, chrome): ζ 0.68, a hint of overshoot.
///
/// Ported from `tokens.odin:25` (`SPRING_ENTER :: Spring{1, 260, 22}`).
pub const SPRING_ENTER: Spring = Spring {
    mass: 1.0,
    stiffness: 260.0,
    damping: 22.0,
};

/// The sweep spring as an ease over its settle time (0.702 s, held by the tests).
///
/// Ported from `tokens.odin:29` (`EASE_SWEEP`). Odin's comptime constant becomes
/// a `LazyLock`: `settle_time` calls non-const float maths, so Rust computes it
/// once at first use instead.
pub static EASE_SWEEP: LazyLock<SpringEase> = LazyLock::new(|| spring_ease(SPRING_SWEEP));

/// The enter spring as an ease over its settle time (0.643 s, held by the tests).
///
/// Ported from `tokens.odin:30` (`EASE_ENTER`); see `EASE_SWEEP`.
pub static EASE_ENTER: LazyLock<SpringEase> = LazyLock::new(|| spring_ease(SPRING_ENTER));

/// Ink being drawn: a symmetric ease-in-out, `cubic-bezier(0.65, 0, 0.35, 1)`.
///
/// Ported from `tokens.odin:33` (`EASE_DRAW_BEZIER :: [4]f64{0.65, 0, 0.35, 1}`).
pub const EASE_DRAW_BEZIER: [f64; 4] = [0.65, 0.0, 0.35, 1.0];

/// The enter spring at its own pace (643 ms), as FV 05 runs its springs.
///
/// Ported from `tokens.odin:36` (`ENTER_SETTLE_MS :: 643`).
pub const ENTER_SETTLE_MS: f64 = 643.0;

/// An overlay leaving: linear, long enough to be seen going.
///
/// Ported from `tokens.odin:39` (`OVERLAY_LEAVE_MS :: 200`).
pub const OVERLAY_LEAVE_MS: f64 = 200.0;

/// How much motion to show: full, or reduced.
///
/// Ported from `tokens.odin:44` (`Level :: enum u8 { Full, Reduced }`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Full,
    Reduced,
}

/// How long an overlay stays on its way out.
///
/// Ported from `tokens.odin:51` (`overlay_leave_ms`).
pub fn overlay_leave_ms(level: Level) -> f64 {
    if level == Level::Reduced {
        REDUCED_FADE_MS
    } else {
        OVERLAY_LEAVE_MS
    }
}

/// The timing functions the page's motions run on.
///
/// Ported from `tokens.odin:55` (`Ease :: enum u8`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ease {
    Linear,
    /// `cubic-bezier(0.65, 0, 0.35, 1)`.
    Draw,
    /// The sweep spring over its settle time.
    Sweep,
    /// The enter spring over its settle time.
    Enter,
    /// `--motion-ease-sweep`: the sweep spring as CSS's `linear()`.
    CssSweep,
    /// `--motion-ease-enter`: the enter spring as CSS's `linear()`.
    CssEnter,
}

/// An ease's output at `progress`.
///
/// Ported from `tokens.odin:64` (`ease_at`).
pub fn ease_at(e: Ease, progress: f64) -> f64 {
    match e {
        Ease::Linear => progress.clamp(0.0, 1.0),
        Ease::Draw => ease_draw(progress),
        Ease::Sweep => spring_ease_at(*EASE_SWEEP, progress),
        Ease::Enter => spring_ease_at(*EASE_ENTER, progress),
        Ease::CssSweep => css_linear_at(*EASE_SWEEP, progress, CSS_LINEAR_SAMPLES),
        Ease::CssEnter => css_linear_at(*EASE_ENTER, progress, CSS_LINEAR_SAMPLES),
    }
}

/// The draw ease's output at `progress`.
///
/// Ported from `tokens.odin:82` (`ease_draw`).
pub fn ease_draw(progress: f64) -> f64 {
    let b = EASE_DRAW_BEZIER;
    bezier_at(cubic_bezier(b[0], b[1], b[2], b[3]), progress)
}

/// How many stops CSS's `linear()` takes a spring's ease in.
///
/// Ported from `tokens.odin:90` (`CSS_LINEAR_SAMPLES :: 32`).
pub const CSS_LINEAR_SAMPLES: i64 = 32;

/// A stop of `e` in CSS's `linear()`: its value at `i / samples`, to 4 decimals.
///
/// Ported from `tokens.odin:94` (`css_linear_stop`; the default
/// `samples := CSS_LINEAR_SAMPLES` is explicit here).
pub fn css_linear_stop(e: SpringEase, i: i64, samples: i64) -> f64 {
    (spring_ease_at(e, i as f64 / samples as f64) * 10_000.0).round() / 10_000.0
}

/// CSS's `linear()` of `e`'s stops at a progress 0 … 1: straight between the two
/// stops on either side, the stops evenly spaced.
///
/// Ported from `tokens.odin:100` (`css_linear_at`).
pub fn css_linear_at(e: SpringEase, progress: f64, samples: i64) -> f64 {
    if progress <= 0.0 {
        return css_linear_stop(e, 0, samples);
    }
    if progress >= 1.0 {
        return css_linear_stop(e, samples, samples);
    }
    let at = progress * samples as f64;
    let i = at.floor() as i64;
    let a = css_linear_stop(e, i, samples);
    let b = css_linear_stop(e, i + 1, samples);
    a + (b - a) * (at - i as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The s-core return's recommendation: the `LazyLock` statics must hold the
    /// settle times the Odin comptime constants (and the ported tests) assert,
    /// 0.702 s and 0.643 s. Addition beside the ported tests.
    #[test]
    fn ease_statics_hold_the_tokens_settle_times() {
        assert_eq!(EASE_SWEEP.settle, 0.702);
        assert_eq!(EASE_ENTER.settle, 0.643);
    }
}
