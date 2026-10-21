//! The welcome page's fit to the window: the frame laid out at window / fit in
//! design units and drawn under a scale of fit, hit testing through the
//! inverse, and the top row scaled by zoom with its menu at natural size.
//!
//! Ported from `src/components/views/WelcomePage/WelcomePage.tsx`,
//! `WelcomePage.module.css` (`.frame`, `.top_bar`) and `useFitScale.ts` at
//! `68e1c7b`. The fit value itself (`fit_scale` + `paper_fit`, the 1440 × 900
//! design size and the 50 px paper step, including the whole-device-pixel
//! rounding) is h10-p-loop's `fit_scale.rs`; the geometry here takes `fit` as
//! an input, and `current_fit` below is the one call that reads it from the
//! window (re-measured every frame, as the `ResizeObserver` re-measures at
//! every resize).

use mv_sys::clay;

use crate::app::App;

/// The top row's offsets and sizes in design px (`.top_bar`).
pub const TOP_OFFSET: f64 = 24.0;
/// The top row's side insets in design px (`left`/`right: 32px`).
pub const TOP_SIDE: f64 = 32.0;
/// The top row's height in design px.
pub const TOP_HEIGHT: f64 = 40.0;

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

/// Where the settings menu sits, in the two spaces the welcome route draws
/// it in: the frame's design space (where Clay lays it out and where the
/// pointer is mapped back into) and the screen (where it is painted, at its
/// natural size under the zoomed gear).
///
/// The menu keeps its natural size (`[popover] { zoom: calc(1 / var(--fit)) }`)
/// while the frame around it is scaled by the fit, so the menu's box moves
/// by `offset` screen px away from where the frame's layout put it. The
/// drawing translates the menu's commands by that offset; the hit test maps
/// a drawn box back into the design space with [`MenuSpace::hit`]. One
/// source for both, so a pointer over a drawn cell hits that cell at every
/// fit.
#[derive(Clone, Copy, Debug)]
pub(crate) struct MenuSpace {
    /// The menu's move from the frame's design space to the screen, in screen px.
    pub(crate) offset: [f32; 2],
    /// The frame's scale: the design space is the screen's, divided by it.
    pub(crate) scale: f32,
}

impl MenuSpace {
    /// The viewer route's space: no move and no scale, the menu where Clay
    /// laid it out.
    pub(crate) const IDENTITY: Self = Self {
        offset: [0.0, 0.0],
        scale: 1.0,
    };

    /// The box a menu element's design box is drawn at, in screen px: the
    /// menu keeps its natural size, so only the move applies.
    ///
    /// A test helper: the drawing translates by `offset` and the hit test
    /// maps through `region`, so no production path needs the box.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn drawn(&self, box_: clay::BoundingBox) -> clay::BoundingBox {
        clay::BoundingBox {
            x: box_.x + self.offset[0],
            y: box_.y + self.offset[1],
            width: box_.width,
            height: box_.height,
        }
    }

    /// The design-space box a menu element's region takes, so a pointer over
    /// its drawn box lands in it: the drawn box divided by the frame's
    /// scale, because the pointer arrives in the design space (the root
    /// scale's inverse).
    #[must_use]
    pub(crate) fn region(&self, box_: clay::BoundingBox) -> clay::BoundingBox {
        clay::BoundingBox {
            x: (box_.x + self.offset[0]) / self.scale,
            y: (box_.y + self.offset[1]) / self.scale,
            width: box_.width / self.scale,
            height: box_.height / self.scale,
        }
    }
}

/// The menu's space on the welcome route: the natural-size anchor under the
/// zoomed gear, the frame's scale being the fit.
///
/// `plate` is the menu's laid-out box in the frame's design space.
#[must_use]
pub(crate) fn menu_space(window_width: f64, fit: f64, plate: clay::BoundingBox) -> MenuSpace {
    let (right, top) = menu_anchor(window_width, fit);
    MenuSpace {
        // `as` at the f64/f32 boundary: the layout is f32 throughout.
        // The menu's width is the plate's own laid-out width, the same
        // number the layout sizes the menu with (`settings::layout::
        // MENU_WIDTH`), so there is no second constant to drift.
        offset: [right as f32 - plate.width - plate.x, top as f32 - plate.y],
        scale: fit as f32,
    }
}

/// The page's fit from the window's logical size and backing pixels
/// (`useFitScale`'s measure through `WelcomePage.tsx`'s paper rounding):
/// `paper_fit(fit_scale(size, DESIGN), PAPER_STEP, dpr)` with the device
/// ratio off the window. Re-measured every frame, so a resize re-lays out on
/// its next frame, as the `ResizeObserver` re-measures at every resize.
#[must_use]
pub fn current_fit(size: [f32; 2], pixels: [i32; 2]) -> f64 {
    let dpr = if size[0] > 0.0 {
        f64::from(pixels[0]) / f64::from(size[0])
    } else {
        1.0
    };
    crate::fit_scale::paper_fit(
        crate::fit_scale::fit_scale(
            [f64::from(size[0]), f64::from(size[1])],
            crate::fit_scale::DESIGN,
        ),
        crate::fit_scale::PAPER_STEP,
        dpr,
    )
}

/// Maps a screen pointer into the design units the welcome page lays out in:
/// on the welcome route the pointer hits (x / fit, y / fit), the inverse of
/// the root scale; any other route keeps screen pixels, as its regions do.
#[must_use]
pub fn design_pos(app: &App, pos: [f32; 2]) -> [f32; 2] {
    if !matches!(
        crate::welcome::Route::of(&app.url),
        crate::welcome::Route::Welcome
    ) {
        return pos;
    }
    let fit = current_fit(app.size, app.pixels);
    if fit > 0.0 {
        // `as` at the f64/f32 boundary: the layout is f32 throughout.
        let fit = fit as f32;
        [pos[0] / fit, pos[1] / fit]
    } else {
        pos
    }
}

#[cfg(test)]
mod tests {
    use super::{
        MENU_GAP, TOP_OFFSET, TOP_SIDE, current_fit, design_pos, frame_size, menu_anchor,
        screen_to_design, zoomed,
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
        // The 304-wide menu stays inside a 1024 window (the layout's own
        // width constant, the one the menu is sized with).
        assert!(right - f64::from(crate::settings::layout::MENU_WIDTH) >= 0.0);
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

    #[test]
    fn current_fit_matches_the_probed_reference() {
        // The P1 probe values on ref-h10 (density 2): 1024 × 640 → 0.71,
        // 1280 × 1000 and 1280 × 800 → 0.88, 768 × 1024 → 0.53, the design
        // size → 1.
        assert_eq!(current_fit([1024.0, 640.0], [2048, 1280]), 0.71);
        assert_eq!(current_fit([1280.0, 1000.0], [2560, 2000]), 0.88);
        assert_eq!(current_fit([1280.0, 800.0], [2560, 1600]), 0.88);
        assert_eq!(current_fit([768.0, 1024.0], [1536, 2048]), 0.53);
        assert_eq!(current_fit([1440.0, 900.0], [2880, 1800]), 1.0);
        // The dpr-1 vitest literals fall out of the same call.
        assert_eq!(current_fit([1024.0, 640.0], [1024, 640]), 0.70);
        assert_eq!(current_fit([1280.0, 800.0], [1280, 800]), 0.88);
    }

    #[test]
    fn design_pos_inverts_the_pointer_on_the_welcome_route() {
        let mut app = crate::fixture::headless_app();
        app.url = crate::url::Url::new("#");
        app.size = [1024.0, 640.0];
        app.pixels = [2048, 1280];
        // Fit 0.71: the gear's screen centre hits its design box.
        let at = design_pos(&app, [987.1, 31.2]);
        assert!((at[0] - 987.1 / 0.71).abs() <= 1e-4);
        assert!((at[1] - 31.2 / 0.71).abs() <= 1e-4);
        // Any other route keeps screen pixels.
        app.url = crate::url::Url::new("#function-viewer");
        assert_eq!(design_pos(&app, [987.1, 31.2]), [987.1, 31.2]);
    }
}
