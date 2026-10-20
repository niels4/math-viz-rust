//! The welcome page's fit-to-window scale.
//!
//! Ported from math-viz2 at `68e1c7b`:
//!
//! - `src/components/views/WelcomePage/useFitScale.ts`: the hook measuring
//!   `min(1, clientWidth / width, clientHeight / height)`.
//! - `src/components/views/WelcomePage/WelcomePage.tsx` (the fit lines at
//!   its top): `designWidth` 1440, `designHeight` 900, `paperStep` 50, and
//!   the rounding down to whole device pixels per paper step.

/// The design size the page shrinks from (`designWidth`, `designHeight`).
pub const DESIGN: [f64; 2] = [1440.0, 900.0];

/// The graph paper's spacing in px (`paperStep`): the fit rounds down to a
/// whole number of device pixels per step, so the paper's lines stay on the
/// hero's (WebKit rounds each tile of a tiled background to whole device
/// pixels under a transform).
pub const PAPER_STEP: f64 = 50.0;

/// The shrink that fits `client` into `design` (`useFitScale`'s measure):
/// `min(1, w / design_w, h / design_h)`, never growing past the design.
///
/// Returns `1.0` when a client side is not positive: the hook only calls
/// `setFit` while both `clientWidth` and `clientHeight` are above 0 and
/// otherwise keeps its previous value (initially `1`), so the caller keeps
/// its stored fit on a zero measure and calls this once laid out.
#[must_use]
pub fn fit_scale(client: [f64; 2], design: [f64; 2]) -> f64 {
    if client[0] <= 0.0 || client[1] <= 0.0 {
        return 1.0;
    }
    1.0f64.min(client[0] / design[0]).min(client[1] / design[1])
}

/// The paper-step rounding of a fit (`WelcomePage.tsx`'s `fit`): below 1,
/// down to whole device pixels per `step` paper px —
/// `floor(fit * step * dpr + 1e-9) / (step * dpr)` — else exactly 1.
#[must_use]
pub fn paper_fit(fit: f64, step: f64, dpr: f64) -> f64 {
    if fit < 1.0 {
        (fit * step * dpr + 1e-9).floor() / (step * dpr)
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The three `WelcomePage.test.tsx` fit cases at dpr 1, plus the same
    // windows at dpr 2, the design size, and a narrow window. The expected
    // values are what the formula gives (verified by running it): at dpr 1
    // 1024 × 640 → 0.70, 1280 × 1000 → 0.88, 1280 × 800 → 0.88; at dpr 2
    // the 1024 × 640 window rounds one step finer (0.71), the 1280 windows
    // stay 0.88; the design size never scales; 768 × 1024 is 0.52 at dpr 1
    // (the brief's 0.53 is the dpr-2 value: 53/100).
    #[test]
    fn fit_1024x640_is_0_70_at_dpr_1() {
        let fitted = fit_scale([1024.0, 640.0], DESIGN);
        assert_eq!(paper_fit(fitted, PAPER_STEP, 1.0), 0.70);
    }

    #[test]
    fn fit_1280x1000_is_0_88_at_dpr_1() {
        let fitted = fit_scale([1280.0, 1000.0], DESIGN);
        assert_eq!(paper_fit(fitted, PAPER_STEP, 1.0), 0.88);
    }

    #[test]
    fn fit_1280x800_is_0_88_at_dpr_1() {
        let fitted = fit_scale([1280.0, 800.0], DESIGN);
        assert_eq!(paper_fit(fitted, PAPER_STEP, 1.0), 0.88);
    }

    #[test]
    fn fit_at_dpr_2_rounds_one_step_finer() {
        assert_eq!(
            paper_fit(fit_scale([1024.0, 640.0], DESIGN), PAPER_STEP, 2.0),
            0.71
        );
        assert_eq!(
            paper_fit(fit_scale([1280.0, 1000.0], DESIGN), PAPER_STEP, 2.0),
            0.88
        );
        assert_eq!(
            paper_fit(fit_scale([1280.0, 800.0], DESIGN), PAPER_STEP, 2.0),
            0.88
        );
    }

    #[test]
    fn fit_at_design_size_and_narrow_windows() {
        assert_eq!(fit_scale(DESIGN, DESIGN), 1.0);
        assert_eq!(paper_fit(1.0, PAPER_STEP, 1.0), 1.0);
        assert_eq!(paper_fit(1.0, PAPER_STEP, 2.0), 1.0);
        assert_eq!(
            paper_fit(fit_scale([768.0, 1024.0], DESIGN), PAPER_STEP, 1.0),
            0.52
        );
        assert_eq!(
            paper_fit(fit_scale([768.0, 1024.0], DESIGN), PAPER_STEP, 2.0),
            0.53
        );
    }

    #[test]
    fn fit_keeps_1_until_laid_out() {
        assert_eq!(fit_scale([0.0, 640.0], DESIGN), 1.0);
        assert_eq!(fit_scale([1024.0, 0.0], DESIGN), 1.0);
    }

    // The vitest literals' pixel arithmetic, pinned as the layout tests
    // will read it: the plane 840 × 520 at 0.70 is 588 × 364, the title 96
    // at 0.70 is 67.2; the plane at 0.88 is 739.2 × 457.6; the gear's top
    // 24 at 0.88 is 21.12, its right inset 32 at 0.88 is 28.16.
    // vitest's toBeCloseTo with its digit count: within half a unit there.
    #[track_caller]
    fn assert_close(actual: f64, expected: f64, digits: i32) {
        let tol = 10f64.powi(-digits) / 2.0;
        assert!(
            (actual - expected).abs() < tol,
            "{actual} is not close to {expected} within {digits} digits"
        );
    }

    #[test]
    fn parts_scale_with_the_fit() {
        let fit = paper_fit(fit_scale([1024.0, 640.0], DESIGN), PAPER_STEP, 1.0);
        assert_close(840.0 * fit, 588.0, 1);
        assert_close(520.0 * fit, 364.0, 1);
        assert_close(96.0 * fit, 67.2, 1);
        let fit = paper_fit(fit_scale([1280.0, 1000.0], DESIGN), PAPER_STEP, 1.0);
        assert_close(840.0 * fit, 739.2, 1);
        assert_close(520.0 * fit, 457.6, 1);
        assert_close(24.0 * fit, 21.12, 1);
        assert_close(32.0 * fit, 28.16, 1);
    }
}
