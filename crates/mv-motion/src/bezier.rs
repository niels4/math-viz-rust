//! Ported from `src/motion/bezier.odin` (86 lines): CSS's cubic-bezier() as a function.

/// Newton solver steps before falling back to bisection.
///
/// Ported from `bezier.odin:7` (`NEWTON_STEPS :: 8`, file-private).
const NEWTON_STEPS: i64 = 8;

/// Bisection steps once the slope is too flat for Newton.
///
/// Ported from `bezier.odin:9` (`BISECT_STEPS :: 40`, file-private).
const BISECT_STEPS: i64 = 40;

/// How close the solver must come.
///
/// Ported from `bezier.odin:11` (`EPSILON :: 1e-7`, file-private).
const EPSILON: f64 = 1e-7;

/// The curve in Bernstein form, its end points at (0, 0) and (1, 1).
///
/// Ported from `bezier.odin:15` (`Cubic_Bezier :: struct { ax, bx, cx, ay, by, cy }`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CubicBezier {
    pub ax: f64,
    pub bx: f64,
    pub cx: f64,
    pub ay: f64,
    pub by: f64,
    pub cy: f64,
}

/// The curve for CSS `cubic-bezier(x1, y1, x2, y2)`.
///
/// Ported from `bezier.odin:19` (`cubic_bezier`).
pub fn cubic_bezier(x1: f64, y1: f64, x2: f64, y2: f64) -> CubicBezier {
    let cx = 3.0 * x1;
    let bx = 3.0 * (x2 - x1) - cx;
    let ax = 1.0 - cx - bx;
    let cy = 3.0 * y1;
    let by = 3.0 * (y2 - y1) - cy;
    let ay = 1.0 - cy - by;
    CubicBezier {
        ax,
        bx,
        cx,
        ay,
        by,
        cy,
    }
}

/// The curve's x at parameter `t`.
///
/// Ported from `bezier.odin:30` (`sample_x`, file-private).
fn sample_x(b: CubicBezier, t: f64) -> f64 {
    ((b.ax * t + b.bx) * t + b.cx) * t
}

/// The curve's y at parameter `t`.
///
/// Ported from `bezier.odin:35` (`sample_y`, file-private).
fn sample_y(b: CubicBezier, t: f64) -> f64 {
    ((b.ay * t + b.by) * t + b.cy) * t
}

/// The curve's x slope at parameter `t`.
///
/// Ported from `bezier.odin:40` (`slope_x`, file-private).
fn slope_x(b: CubicBezier, t: f64) -> f64 {
    (3.0 * b.ax * t + 2.0 * b.bx) * t + b.cx
}

/// The curve parameter whose x is `x`: Newton first, bisection when the slope
/// is too flat for Newton to converge.
///
/// Ported from `bezier.odin:47` (`solve_t`, file-private).
fn solve_t(b: CubicBezier, x: f64) -> f64 {
    let mut t = x;
    for _ in 0..NEWTON_STEPS {
        let err = sample_x(b, t) - x;
        if err.abs() < EPSILON {
            return t;
        }
        let slope = slope_x(b, t);
        if slope.abs() < 1e-6 {
            break;
        }
        t -= err / slope;
    }
    let (mut lo, mut hi) = (0.0, 1.0);
    t = x;
    for _ in 0..BISECT_STEPS {
        let value = sample_x(b, t);
        if (value - x).abs() < EPSILON {
            return t;
        }
        if value < x {
            lo = t;
        } else {
            hi = t;
        }
        t = (lo + hi) / 2.0;
    }
    t
}

/// Progress 0 … 1 → the eased value CSS draws, clamped outside 0 … 1.
///
/// Ported from `bezier.odin:78` (`bezier_at`).
pub fn bezier_at(b: CubicBezier, progress: f64) -> f64 {
    if progress <= 0.0 {
        return 0.0;
    }
    if progress >= 1.0 {
        return 1.0;
    }
    sample_y(b, solve_t(b, progress))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spring::spring_at;
    use crate::tokens::{
        CSS_LINEAR_SAMPLES, DRAW_MS, EASE_ENTER, Ease, FAST_MS, LEAVE_MS, Level, SPRING_ENTER,
        css_linear_at, css_linear_stop, ease_at, ease_draw, overlay_leave_ms,
    };

    fn percent(v: f64) -> i64 {
        (v * 100.0).round() as i64
    }

    #[test]
    fn the_draw_ease_matches_the_designs_draw_on_filmstrip() {
        let want = [0, 3, 15, 50, 85, 100];
        for (i, ms) in [0.0, 160.0, 300.0, 450.0, 600.0, 900.0].iter().enumerate() {
            assert_eq!(percent(ease_draw(ms / 900.0)), want[i]);
        }
    }

    #[test]
    fn a_straight_bezier_is_the_identity() {
        let line = cubic_bezier(0.0, 0.0, 1.0, 1.0);
        for p in [0.1, 0.33, 0.5, 0.9] {
            assert!((bezier_at(line, p) - p).abs() < 5e-7);
        }
    }

    #[test]
    fn a_bezier_agrees_with_css_ease() {
        assert!((bezier_at(cubic_bezier(0.25, 0.1, 0.25, 1.0), 0.5) - 0.8024).abs() < 5e-4);
    }

    #[test]
    fn a_bezier_clamps_outside_0_to_1() {
        assert_eq!(ease_draw(-1.0), 0.0);
        assert_eq!(ease_draw(2.0), 1.0);
    }

    #[test]
    fn the_tokens() {
        assert_eq!(LEAVE_MS, 120.0);
        assert_eq!(FAST_MS, 160.0);
        assert_eq!(DRAW_MS, 900.0);
        assert_eq!(overlay_leave_ms(Level::Full), 200.0);
        assert_eq!(overlay_leave_ms(Level::Reduced), 120.0);
    }

    const REACT_EASE_ENTER: &str = "linear(0, 0.0452, 0.1545, 0.2959, 0.446, 0.589, 0.7155, 0.8208, 0.9035, 0.9648, 1.0071, 1.0337, 1.048, 1.0531, 1.052, 1.0468, 1.0396, 1.0316, 1.0238, 1.0168, 1.0109, 1.0062, 1.0026, 1.0001, 0.9985, 0.9976, 0.9972, 0.9972, 0.9974, 0.9978, 0.9982, 0.9986, 1)";

    #[test]
    fn an_ease_as_css_linear_keeps_its_stops_and_overshoot() {
        let stops: Vec<&str> = REACT_EASE_ENTER
            .strip_prefix("linear(")
            .unwrap()
            .strip_suffix(')')
            .unwrap()
            .split(", ")
            .collect();
        assert_eq!(stops.len(), (CSS_LINEAR_SAMPLES + 1) as usize);
        let mut peak = 0.0f64;
        for (i, s) in stops.iter().enumerate() {
            let want: f64 = s.parse().unwrap();
            assert_eq!(
                css_linear_stop(*EASE_ENTER, i as i64, CSS_LINEAR_SAMPLES),
                want
            );
            peak = peak.max(want);
        }
        assert!(peak > 1.0);
        // Between stops, straight: halfway from the first stop to the second.
        assert!(
            (css_linear_at(*EASE_ENTER, 0.5 / 32.0, CSS_LINEAR_SAMPLES) - 0.0226).abs() < 1e-12
        );
        assert_eq!(css_linear_at(*EASE_ENTER, 0.0, CSS_LINEAR_SAMPLES), 0.0);
        assert_eq!(css_linear_at(*EASE_ENTER, 1.0, CSS_LINEAR_SAMPLES), 1.0);
        assert_eq!(ease_at(Ease::CssEnter, 13.0 / 32.0), 1.0531);
        assert!((ease_at(Ease::Enter, 0.25) - spring_at(SPRING_ENTER, 0.25 * 0.643)).abs() < 1e-15);
        assert!((ease_at(Ease::Linear, 0.3) - 0.3).abs() < 1e-15);
    }
}
