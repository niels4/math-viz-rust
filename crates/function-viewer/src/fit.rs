//! The welcome page's fit to the window: the frame laid out at window / fit in
//! design units and drawn under a scale of fit, hit testing through the
//! inverse, and the top row scaled by zoom with its menu at natural size.
//!
//! Ported from `src/components/views/WelcomePage/WelcomePage.tsx`,
//! `WelcomePage.module.css` (`.frame`, `.top_bar`) and `useFitScale.ts` at
//! `68e1c7b`. The fit value itself (`fit_scale` + `paper_fit`, the 1440 × 900
//! design size and the 50 px paper step, including the whole-device-pixel
//! rounding) is h10-p-loop's: this module takes `fit` as an input and never
//! computes it. Wiring (the NanoVG scale at the root, the re-layout on
//! resize, the menu anchor) lands with h10-page's `welcome.rs`; the three
//! vitest fit cases become layout tests there.

/// The top row's offsets and sizes in design px (`.top_bar`).
pub const TOP_OFFSET: f64 = 24.0;
/// The top row's side insets in design px (`left`/`right: 32px`).
pub const TOP_SIDE: f64 = 32.0;
/// The top row's height in design px.
pub const TOP_HEIGHT: f64 = 40.0;

/// The settings menu's width in design px.
pub const MENU_WIDTH: f64 = 304.0;
/// The menu's gap below the gear's box in screen px.
pub const MENU_GAP: f64 = 8.0;

/// The size to lay the frame out at: window / fit in design units (`.frame {
/// width: calc(100% / var(--fit)); height: ... }`), drawn under a scale of
/// fit from the origin, so the content safe-centres inside the enlarged frame
/// and shrinks as one picture.
#[must_use]
pub fn frame_size(window_width: f64, window_height: f64, fit: f64) -> (f64, f64) {
    (window_width / fit, window_height / fit)
}

/// The design point a screen pointer hits: the inverse of the root scale (a
/// pointer at screen (x, y) hits the design point (x / fit, y / fit)).
#[must_use]
pub fn screen_to_design(screen_x: f64, screen_y: f64, fit: f64) -> (f64, f64) {
    (screen_x / fit, screen_y / fit)
}

/// A top-row length at its painted size: the row scales by `zoom:
/// var(--fit)` rather than the frame's transform, so its offsets 24 and 32,
/// its 40 px height and the 28 px mark all scale by fit.
#[must_use]
pub fn zoomed(design_px: f64, fit: f64) -> f64 {
    design_px * fit
}

/// The menu's anchor in screen px: the popover keeps its natural size
/// (`[popover] { zoom: calc(1 / var(--fit)) }`) under the zoomed gear — its
/// right edge at the window's width less the zoomed 32 px side inset, its top
/// the zoomed gear bottom (24 + 40) plus the 8 px gap. Returns
/// (right, top): the menu's right edge and top edge.
#[must_use]
pub fn menu_anchor(window_width: f64, fit: f64) -> (f64, f64) {
    (
        window_width - zoomed(TOP_SIDE, fit),
        zoomed(TOP_OFFSET + TOP_HEIGHT, fit) + MENU_GAP,
    )
}

#[cfg(test)]
mod tests {
    use super::{
        MENU_GAP, MENU_WIDTH, TOP_OFFSET, TOP_SIDE, frame_size, menu_anchor, screen_to_design,
        zoomed,
    };

    const EPS: f64 = 1e-9;

    fn close(actual: f64, expected: f64) -> bool {
        (actual - expected).abs() <= EPS
    }

    #[test]
    fn frame_is_window_over_fit() {
        // 1024 × 640 at the vitest fit 0.70.
        let (w, h) = frame_size(1024.0, 640.0, 0.70);
        assert!(close(w, 1024.0 / 0.70));
        assert!(close(h, 640.0 / 0.70));
        // 1280 × 1000 at the vitest fit 0.88.
        let (w, h) = frame_size(1280.0, 1000.0, 0.88);
        assert!(close(w, 1280.0 / 0.88));
        assert!(close(h, 1000.0 / 0.88));
        // At fit 1 the frame is the window.
        assert_eq!(frame_size(1440.0, 900.0, 1.0), (1440.0, 900.0));
    }

    #[test]
    fn hit_testing_inverts_the_root_scale() {
        let (x, y) = screen_to_design(512.0, 320.0, 0.70);
        assert!(close(x, 512.0 / 0.70));
        assert!(close(y, 320.0 / 0.70));
        // Round trip: design → screen → design.
        let (x, y) = screen_to_design(0.70 * 100.0, 0.70 * 200.0, 0.70);
        assert!(close(x, 100.0));
        assert!(close(y, 200.0));
    }

    #[test]
    fn title_scales_by_zoom_at_1024x640() {
        // The 96 px title's box reads 67.2 tall: 96 · 0.70.
        assert!(close(zoomed(96.0, 0.70), 67.2));
    }

    #[test]
    fn plane_scales_by_zoom() {
        // The 840 × 520 hero reads 588 × 364 at fit 0.70 ...
        assert!(close(zoomed(840.0, 0.70), 588.0));
        assert!(close(zoomed(520.0, 0.70), 364.0));
        // ... and 739.2 × 457.6 at fit 0.88.
        assert!(close(zoomed(840.0, 0.88), 739.2));
        assert!(close(zoomed(520.0, 0.88), 457.6));
    }

    #[test]
    fn gear_sits_in_the_window_corners_at_1280x1000() {
        // The vitest literals: the gear's top 21.12, its right inset 28.16.
        assert!(close(zoomed(TOP_OFFSET, 0.88), 21.12));
        assert!(close(zoomed(TOP_SIDE, 0.88), 28.16));
    }

    #[test]
    fn menu_anchors_to_the_zoomed_gear() {
        // Right edge at the window width less the zoomed 32 px inset, top 8
        // px below the zoomed gear bottom (24 + 40), at natural size.
        let (right, top) = menu_anchor(1024.0, 0.70);
        assert!(close(right, 1024.0 - 32.0 * 0.70));
        assert!(close(top, 64.0 * 0.70 + MENU_GAP));
        // The 304-wide menu stays inside a 1024 window.
        assert!(right - MENU_WIDTH >= 0.0);
        assert!(right <= 1024.0);
    }

    #[test]
    fn paper_step_stays_whole_device_pixels() {
        // The 1280 × 800 vitest case: (plane.width / 840) · 50 · dpr is whole
        // within 0.005. At dpr 1 the plane reads 840 · fit, so the check is
        // fit · 50 whole — the paper_fit rounding h10-p-loop owns; here the
        // zoomed width that feeds it.
        let per_unit = zoomed(50.0, 0.88);
        assert!((per_unit - per_unit.round()).abs() <= 0.005 + EPS);
    }
}
