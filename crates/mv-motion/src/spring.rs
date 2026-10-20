//! Ported from `src/motion/spring.odin` (67 lines): pure damped-spring maths.

/// A damped spring (motion.dev's `{ mass, stiffness, damping }`).
///
/// Ported from `spring.odin:11` (`Spring :: struct { mass, stiffness, damping }`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spring {
    pub mass: f64,
    pub stiffness: f64,
    pub damping: f64,
}

/// Position at `t` seconds of a spring released at 0 towards 1 from rest.
///
/// Ported from `spring.odin:16` (`spring_at`).
pub fn spring_at(s: Spring, t: f64) -> f64 {
    if t <= 0.0 {
        return 0.0;
    }
    let w0 = (s.stiffness / s.mass).sqrt();
    let zeta = s.damping / (2.0 * (s.stiffness * s.mass).sqrt());
    if (zeta - 1.0).abs() < 1e-9 {
        return 1.0 - (-w0 * t).exp() * (1.0 + w0 * t);
    }
    if zeta < 1.0 {
        let wd = w0 * (1.0 - zeta * zeta).sqrt();
        let decay = zeta * w0;
        return 1.0 - (-decay * t).exp() * ((wd * t).cos() + (decay / wd) * (wd * t).sin());
    }
    let root = w0 * (zeta * zeta - 1.0).sqrt();
    let r1 = -zeta * w0 + root;
    let r2 = -zeta * w0 - root;
    1.0 + (r2 * (r1 * t).exp() - r1 * (r2 * t).exp()) / (r1 - r2)
}

/// The settle scan's step, seconds.
///
/// Ported from `spring.odin:37` (`SETTLE_STEP_S :: 0.001`, file-private).
const SETTLE_STEP_S: f64 = 0.001;

/// How far the settle scan looks, seconds.
///
/// Ported from `spring.odin:39` (`SETTLE_LIMIT_S :: 10`, file-private).
const SETTLE_LIMIT_S: f64 = 10.0;

/// Seconds until the spring stays within `epsilon` of its target, to the millisecond.
///
/// Ported from `spring.odin:43` (`settle_time`; the default `epsilon := 0.001`
/// is explicit here: Rust has no default arguments).
pub fn settle_time(s: Spring, epsilon: f64) -> f64 {
    let mut last_outside = 0.0;
    let mut t = SETTLE_STEP_S;
    while t < SETTLE_LIMIT_S {
        if (1.0 - spring_at(s, t)).abs() > epsilon {
            last_outside = t;
        }
        t += SETTLE_STEP_S;
    }
    ((last_outside + SETTLE_STEP_S) * 1000.0).round() / 1000.0
}

/// A spring as an ease over its settle time.
///
/// Ported from `spring.odin:55` (`Spring_Ease :: struct { spring, settle }`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpringEase {
    pub spring: Spring,
    /// The spring's own duration, s: `settle_time(spring)`.
    pub settle: f64,
}

/// A spring as an ease over its settle time.
///
/// Ported from `spring.odin:60` (`spring_ease`).
pub fn spring_ease(s: Spring) -> SpringEase {
    SpringEase {
        spring: s,
        settle: settle_time(s, 0.001),
    }
}

/// Progress 0 … 1 → position, overshoot kept.
///
/// Ported from `spring.odin:65` (`spring_ease_at`).
pub fn spring_ease_at(e: SpringEase, progress: f64) -> f64 {
    if progress >= 1.0 {
        1.0
    } else {
        spring_at(e.spring, progress * e.settle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokens::{EASE_ENTER, EASE_SWEEP, ENTER_SETTLE_MS, SPRING_ENTER, SPRING_SWEEP};

    fn percent(v: f64) -> i64 {
        (v * 100.0).round() as i64
    }

    #[test]
    fn the_sweep_spring_matches_the_designs_filmstrip() {
        let want = [0, 28, 62, 82, 92, 99];
        for (i, ms) in [0.0, 80.0, 160.0, 240.0, 320.0, 480.0].iter().enumerate() {
            assert_eq!(percent(spring_at(SPRING_SWEEP, ms / 1000.0)), want[i]);
        }
    }

    #[test]
    fn a_critically_damped_spring_never_overshoots() {
        for ms in (0..=2000).step_by(5) {
            assert!(spring_at(SPRING_SWEEP, ms as f64 / 1000.0) <= 1.0);
        }
    }

    #[test]
    fn the_enter_spring_overshoots_a_little() {
        let mut peak = 0.0;
        let mut peak_ms = 0;
        for ms in 0..=1000 {
            let v = spring_at(SPRING_ENTER, ms as f64 / 1000.0);
            if v > peak {
                peak = v;
                peak_ms = ms;
            }
        }
        assert!((peak - 1.053).abs() < 0.0005);
        assert_eq!(peak_ms, 266);
    }

    #[test]
    fn exactly_critical_and_overdamped_springs() {
        let critical = Spring {
            mass: 1.0,
            stiffness: 100.0,
            damping: 20.0,
        };
        assert!((spring_at(critical, 0.1) - (1.0 - (-1.0f64).exp() * 2.0)).abs() < 5e-11);
        let over = Spring {
            mass: 1.0,
            stiffness: 100.0,
            damping: 40.0,
        };
        assert_eq!(spring_at(over, 0.0), 0.0);
        assert!(spring_at(over, 0.05) < spring_at(critical, 0.05));
        assert!((spring_at(over, 5.0) - 1.0).abs() < 5e-5);
    }

    #[test]
    fn a_spring_settles_where_its_motion_is_done() {
        assert_eq!(settle_time(SPRING_SWEEP, 0.01), 0.506);
        assert_eq!(settle_time(SPRING_SWEEP, 0.001), 0.702);
        assert_eq!(settle_time(SPRING_ENTER, 0.001), 0.643);
        assert_eq!(spring_ease(SPRING_SWEEP), *EASE_SWEEP);
        assert_eq!(spring_ease(SPRING_ENTER), *EASE_ENTER);
        assert_eq!(ENTER_SETTLE_MS, (EASE_ENTER.settle * 1000.0).round());
    }

    #[test]
    fn a_spring_eases_over_its_settle_time_and_lands_exactly() {
        let e = *EASE_SWEEP;
        assert_eq!(spring_ease_at(e, 0.0), 0.0);
        assert_eq!(spring_ease_at(e, 1.0), 1.0);
        assert!((spring_ease_at(e, 0.08 / e.settle) - spring_at(SPRING_SWEEP, 0.08)).abs() < 5e-13);
        assert!((1.0 - spring_at(SPRING_SWEEP, e.settle)).abs() <= 0.001);
    }
}
