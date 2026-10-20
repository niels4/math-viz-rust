//! The plane's HTML overlay: the corner brackets and the return-to-centre
//! button over the canvas.
//!
//! Ported from `src/components/views/CartesianPlane/HtmlOverlay.tsx` and the
//! overlay half of `CartesianPlain.module.css` at c409aad. Geometry is pure
//! (tested against the reference's boxes); the brackets and the button paint
//! through NanoVG, and the page slot declares the button's kept region for
//! hover and the click. The click resets through [`App::reset_view`], which
//! h2-input wires to its live camera.
//!
//! At this hash the button renders on every view: `HtmlOverlay.tsx` holds no
//! visibility conditional (the reference shows it at the default camera too,
//! probed at `.local/probe/default.json`). `button_visible` records that.

use mv_sys::clay;

/// The overlay sits this far inside the plane's edges (`--space-inset`).
pub const INSET: f32 = 12.0;

/// A corner bracket's arm length and width.
pub const CORNER_LENGTH: f32 = 26.0;

/// A corner bracket's arm length and width.
pub const CORNER_WIDTH: f32 = 2.5;

/// The button's size.
pub const BUTTON_SIZE: f32 = 36.0;

/// The button's right edge sits this far left of the plane's right edge
/// (`calc(28.5px - var(--space-inset))` of the overlay).
pub const BUTTON_RIGHT: f32 = 28.5;

/// The button's bottom edge sits this far above the plane's bottom edge
/// (`calc(57.5px + 10px - var(--space-inset))` of the overlay).
pub const BUTTON_BOTTOM: f32 = 67.5;

/// The button's corner radius (`var(--radius-card)`).
pub const BUTTON_RADIUS: f32 = 4.0;

/// The button's icon renders this wide in its 36 px box (the svg's 20 px on
/// its 24-unit viewBox).
const ICON_SCALE: f32 = 20.0 / 24.0;

/// The overlay's box: the plane inset by [`INSET`] on every side.
pub fn overlay_box(plane: &clay::BoundingBox) -> clay::BoundingBox {
    clay::BoundingBox {
        x: plane.x + INSET,
        y: plane.y + INSET,
        width: plane.width - 2.0 * INSET,
        height: plane.height - 2.0 * INSET,
    }
}

/// The four corner brackets, top-left, top-right, bottom-left, bottom-right.
pub fn corner_boxes(overlay: &clay::BoundingBox) -> [clay::BoundingBox; 4] {
    [
        clay::BoundingBox {
            x: overlay.x,
            y: overlay.y,
            width: CORNER_LENGTH,
            height: CORNER_LENGTH,
        },
        clay::BoundingBox {
            x: overlay.x + overlay.width - CORNER_LENGTH,
            y: overlay.y,
            width: CORNER_LENGTH,
            height: CORNER_LENGTH,
        },
        clay::BoundingBox {
            x: overlay.x,
            y: overlay.y + overlay.height - CORNER_LENGTH,
            width: CORNER_LENGTH,
            height: CORNER_LENGTH,
        },
        clay::BoundingBox {
            x: overlay.x + overlay.width - CORNER_LENGTH,
            y: overlay.y + overlay.height - CORNER_LENGTH,
            width: CORNER_LENGTH,
            height: CORNER_LENGTH,
        },
    ]
}

/// The button's box: above the scale legend, near the plane's right edge.
pub fn overlay_button_box(plane: &clay::BoundingBox) -> clay::BoundingBox {
    clay::BoundingBox {
        x: plane.x + plane.width - BUTTON_RIGHT - BUTTON_SIZE,
        y: plane.y + plane.height - BUTTON_BOTTOM - BUTTON_SIZE,
        width: BUTTON_SIZE,
        height: BUTTON_SIZE,
    }
}

/// Whether the button shows: always at this hash (see the module docs).
pub fn button_visible() -> bool {
    true
}

/// The overlay's colours: the brackets (`foreground-muted`), the button's
/// fill (`card`, `secondary` hovered), rim (`border`) and ink (`foreground`),
/// the focus ring (`primary` at 32%).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OverlayColors {
    pub chrome: [f32; 4],
    pub button: [f32; 4],
    pub button_hover: [f32; 4],
    pub button_rim: [f32; 4],
    pub button_ink: [f32; 4],
    pub focus_ring: [f32; 4],
}

/// Paints the four corner brackets: two [`CORNER_WIDTH`] arms each, one path.
pub fn paint_corners(vg: &mut mv_nvg::Context, corners: &[clay::BoundingBox; 4], chrome: [f32; 4]) {
    vg.begin_path();
    for (i, corner) in corners.iter().enumerate() {
        let (top, left) = (i < 2, i % 2 == 0);
        let arm_y = if top {
            corner.y
        } else {
            corner.y + corner.height - CORNER_WIDTH
        };
        let arm_x = if left {
            corner.x
        } else {
            corner.x + corner.width - CORNER_WIDTH
        };
        vg.rect(corner.x, arm_y, corner.width, CORNER_WIDTH);
        vg.rect(arm_x, corner.y, CORNER_WIDTH, corner.height);
    }
    vg.fill_color(chrome);
    vg.fill();
}

/// Paints the button: the rounded plate, the target icon (the ring, the dot
/// and the four ticks), and the focus ring where the keyboard put the focus.
pub fn paint_button(
    vg: &mut mv_nvg::Context,
    button: &clay::BoundingBox,
    colors: &OverlayColors,
    hovered: bool,
    focused: bool,
) {
    if focused {
        vg.begin_path();
        vg.rounded_rect(
            button.x - 1.5,
            button.y - 1.5,
            button.width + 3.0,
            button.height + 3.0,
            BUTTON_RADIUS + 1.5,
        );
        vg.stroke_color(colors.focus_ring);
        vg.stroke_width(3.0);
        vg.stroke();
    }
    vg.begin_path();
    vg.rounded_rect(
        button.x,
        button.y,
        button.width,
        button.height,
        BUTTON_RADIUS,
    );
    vg.fill_color(if hovered {
        colors.button_hover
    } else {
        colors.button
    });
    vg.fill();
    vg.stroke_color(colors.button_rim);
    vg.stroke_width(1.0);
    vg.stroke();

    // The target icon, drawn in the svg's units scaled by ICON_SCALE about
    // the button's centre; strokes scale the same way (1.8 units wide). The
    // round caps stay inside this save block: the app's strokes are butt
    // everywhere else, and the next frame's painters inherit the context.
    let cx = button.x + button.width / 2.0;
    let cy = button.y + button.height / 2.0;
    let ring = 6.5 * ICON_SCALE;
    vg.save();
    vg.stroke_color(colors.button_ink);
    vg.stroke_width(1.8 * ICON_SCALE);
    vg.line_cap(mv_nvg::LineCapType::ROUND);
    vg.begin_path();
    vg.rounded_rect(cx - ring, cy - ring, 2.0 * ring, 2.0 * ring, ring);
    vg.stroke();
    let dot = 2.6 * ICON_SCALE;
    vg.begin_path();
    vg.rounded_rect(cx - dot, cy - dot, 2.0 * dot, 2.0 * dot, dot);
    vg.fill_color(colors.button_ink);
    vg.fill();
    vg.begin_path();
    for ([x0, y0], [x1, y1]) in [
        ([12.0, 2.5], [12.0, 5.5]),
        ([12.0, 18.5], [12.0, 21.5]),
        ([2.5, 12.0], [5.5, 12.0]),
        ([18.5, 12.0], [21.5, 12.0]),
    ] {
        vg.move_to(cx + (x0 - 12.0) * ICON_SCALE, cy + (y0 - 12.0) * ICON_SCALE);
        vg.line_to(cx + (x1 - 12.0) * ICON_SCALE, cy + (y1 - 12.0) * ICON_SCALE);
    }
    vg.stroke();
    vg.restore();
}

/// The overlay's colours from the resolved theme.
pub fn overlay_colors(theme: &mv_theme::Theme) -> OverlayColors {
    let primary = theme.colors[mv_theme::Token::Primary];
    OverlayColors {
        chrome: theme.colors[mv_theme::Token::ForegroundMuted],
        button: theme.colors[mv_theme::Token::Card],
        button_hover: theme.colors[mv_theme::Token::Secondary],
        button_rim: theme.colors[mv_theme::Token::Border],
        button_ink: theme.colors[mv_theme::Token::Foreground],
        // `color-mix(in oklch, var(--primary) 32%, transparent)` as a plain
        // alpha: no state focuses the button, so the mix's exact oklch path
        // is unjudged.
        focus_ring: [primary[0], primary[1], primary[2], 0.32],
    }
}

// The return-to-centre click resets through `App::reset_view`, which lives
// in `plane_input.rs` (the only copy; this module's twin went away at
// h2-input's merge): both params to their defaults, flushed, with the live
// camera following a still one at once.
#[cfg(test)]
mod tests {
    use super::*;

    fn plane() -> clay::BoundingBox {
        clay::BoundingBox {
            x: 432.0,
            y: 96.0,
            width: 992.0,
            height: 788.0,
        }
    }

    /// The overlay is the plane inset by `--space-inset` (probe
    /// `default.json`: 444, 108, 968, 764).
    #[test]
    fn overlay_insets_twelve() {
        let overlay = overlay_box(&plane());
        assert_eq!(
            [overlay.x, overlay.y, overlay.width, overlay.height],
            [444.0, 108.0, 968.0, 764.0]
        );
    }

    /// The brackets sit in the overlay's corners, 26 px square (probe).
    #[test]
    fn corners_sit_in_the_overlay_corners() {
        let overlay = overlay_box(&plane());
        let corners = corner_boxes(&overlay);
        assert_eq!([corners[0].x, corners[0].y], [444.0, 108.0]);
        assert_eq!([corners[1].x, corners[1].y], [1386.0, 108.0]);
        assert_eq!([corners[2].x, corners[2].y], [444.0, 846.0]);
        assert_eq!([corners[3].x, corners[3].y], [1386.0, 846.0]);
        for corner in &corners {
            assert_eq!([corner.width, corner.height], [26.0, 26.0]);
        }
    }

    /// The button's box against the probe (page box 1359.5, 780.5, 36, 36):
    /// 28.5 px off the plane's right edge, 67.5 px off its bottom.
    #[test]
    fn button_box_matches_probe() {
        let button = overlay_button_box(&plane());
        assert_eq!(
            [button.x, button.y, button.width, button.height],
            [927.5 + 432.0, 684.5 + 96.0, 36.0, 36.0]
        );
    }

    /// At this hash the button shows on every view, including the default
    /// camera (probed: the button's box exists in all fourteen states).
    #[test]
    fn button_shows_on_every_view() {
        assert!(button_visible());
    }

    /// The click clears the camera's keys, deleting them at their defaults,
    /// and dirties the frame.
    #[test]
    fn reset_view_clears_the_camera_keys() {
        let mut app = crate::fixture::headless_app();
        app.url = crate::url::Url::from_query("center=1.5000,-2.0000&scale=2.00");
        crate::plane_input::init_camera_from_url(&mut app);
        app.dirty = false;
        app.reset_view();
        assert_eq!(app.url.query(), "");
        assert!(app.dirty);
    }
}
