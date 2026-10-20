//! The canvas geometry the handle and line painters stroke.
//!
//! Ported from math-viz2 at `5cef052`
//! (`src/components/views/CartesianPlane/drawCartesianPlane.ts`): `toPx`,
//! the overshoot clamp inside `drawCurve` (which arrived in `f1e069f`,
//! inside h6's stretch), `drawHandle`'s shapes and `drawLine`'s shaft,
//! ticks and arrowhead. h7-p-plane adds `fromPx` and `hitTest` at `251c080`.
//! Paint (colours, widths, dashes, alphas) stays with
//! the coders' `plane.rs`; this module is pure positions, which is what
//! the vitest cases pin.

use crate::grid::GridLayout;
use crate::plot::{HandleKind, LineKind, PlotHandle, PlotPoint};

/// How far past the canvas a sampled curve may overshoot before the stroke
/// clamps it, in px (`overshootPx`): WebKit still strokes a curve drawn
/// within 10,000 px of the canvas.
pub const OVERSHOOT_PX: f64 = 10_000.0;

/// The diamond handle's radius, in px (`diamondRadius`).
pub const DIAMOND_RADIUS: f64 = 10.0;

/// The square handle's side, in px (`squareSide`).
pub const SQUARE_SIDE: f64 = 16.0;

/// The square handle's corner radius, in px (`squareRadius`).
pub const SQUARE_RADIUS: f64 = 3.0;

/// The square handle's stroke width, in px (`handleStroke`): the inner
/// stroke rect insets by half of it.
pub const HANDLE_STROKE: f64 = 2.5;

/// The arrowhead's length along the shaft, in px (`arrowHeadLength`).
pub const ARROW_HEAD_LENGTH: f64 = 9.8;

/// The arrowhead's half-width across the shaft, in px (`arrowHeadHalf`).
pub const ARROW_HEAD_HALF: f64 = 4.9;

/// The line ticks' stroke width, in px (`tickWidth`): paint, pinned here
/// beside the ticks it strokes.
pub const TICK_WIDTH: f64 = 2.0;

/// Plane units to canvas px (`toPx`).
#[must_use]
pub fn to_px(grid: &GridLayout, point: PlotPoint) -> [f64; 2] {
    [
        grid.origin[0] + point.x * grid.px_per_unit,
        grid.origin[1] - point.y * grid.px_per_unit,
    ]
}

/// Canvas px back to plane units (`fromPx`): the inverse of [`to_px`].
#[must_use]
pub fn from_px(grid: &GridLayout, px: [f64; 2]) -> PlotPoint {
    PlotPoint {
        x: (px[0] - grid.origin[0]) / grid.px_per_unit,
        y: (grid.origin[1] - px[1]) / grid.px_per_unit,
    }
}

/// Clamp a sampled curve y to the strokeable band (`drawCurve`'s
/// `Math.min`/`Math.max` with `overshootPx`): the painter's sampling calls
/// this per column before accumulating the inside length. h6-curve wires it
/// into the paint path; `plot::sample_curve` (h3-p-plot's, from before the
/// clamp) is untouched.
#[must_use]
pub fn clamp_curve_y(y: f64, height: f64) -> f64 {
    (height + OVERSHOOT_PX).min(y.max(-OVERSHOOT_PX))
}

/// A handle's stroked shape in canvas px (`drawHandle` without the paint):
/// a diamond's four corners clockwise from the top, or a square's outer
/// and inner rounded rects (`[x, y, w, h]` with corner radius).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HandlePath {
    Diamond {
        points: [[f64; 2]; 4],
    },
    Square {
        outer: [f64; 4],
        outer_radius: f64,
        inner: [f64; 4],
        inner_radius: f64,
    },
}

/// A handle's path at its canvas position (`at` is already in px, through
/// [`to_px`]).
#[must_use]
pub fn handle_path(kind: HandleKind, at: [f64; 2]) -> HandlePath {
    let [x, y] = at;
    match kind {
        HandleKind::Diamond => HandlePath::Diamond {
            points: [
                [x, y - DIAMOND_RADIUS],
                [x + DIAMOND_RADIUS, y],
                [x, y + DIAMOND_RADIUS],
                [x - DIAMOND_RADIUS, y],
            ],
        },
        HandleKind::Square => {
            let half = SQUARE_SIDE / 2.0;
            let inset = HANDLE_STROKE / 2.0;
            HandlePath::Square {
                outer: [x - half, y - half, SQUARE_SIDE, SQUARE_SIDE],
                outer_radius: SQUARE_RADIUS,
                inner: [
                    x - half + inset,
                    y - half + inset,
                    SQUARE_SIDE - HANDLE_STROKE,
                    SQUARE_SIDE - HANDLE_STROKE,
                ],
                inner_radius: SQUARE_RADIUS - inset,
            }
        }
    }
}

/// The topmost handle under a canvas point (`hitTest` at `251c080`): the
/// last handle whose shape contains it — a diamond where
/// `|dx| + |dy| <= diamondRadius`, a square where
/// `max(|dx|, |dy|) <= squareSide / 2` — so the scale square wins where the
/// handles overlap. `None` when no handle does (the source's -1). The
/// source also skips a handle lacking the action's handler; both handles
/// carry all five handlers at this hash, so the filter is left out, and the
/// handlers themselves stay the app's dispatch.
#[must_use]
pub fn hit_test(grid: &GridLayout, handles: &[PlotHandle], at: [f64; 2]) -> Option<usize> {
    handles.iter().rposition(|handle| {
        let [x, y] = to_px(grid, handle.at);
        let dx = (at[0] - x).abs();
        let dy = (at[1] - y).abs();
        match handle.kind {
            HandleKind::Diamond => dx + dy <= DIAMOND_RADIUS,
            HandleKind::Square => dx.max(dy) <= SQUARE_SIDE / 2.0,
        }
    })
}

/// The tick half-lengths across the start and end of each line look
/// (`lineLooks`' `startTick`/`endTick`): the single source h6-handles'
/// painter reads, so the ticks it strokes match the ticks the tests pin.
#[must_use]
pub fn line_tick_halves(kind: LineKind) -> (f64, f64) {
    match kind {
        LineKind::Arrow => (9.0, 0.0),
        LineKind::Dashed => (0.0, 0.0),
        LineKind::Measured => (7.0, 7.0),
    }
}

/// A line's stroked segments in canvas px (`drawLine` without the paint):
/// the shaft (shortened past the arrowhead, if any), a tick across each
/// end, and the arrowhead's three corners starting at the tip.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinePath {
    pub shaft: [[f64; 2]; 2],
    pub start_tick: [[f64; 2]; 2],
    pub end_tick: [[f64; 2]; 2],
    pub head: Option<[[f64; 2]; 3]>,
}

/// A tick across `(x, y)`, half `half` to each side along the perpendicular
/// (`tick` in `drawLine`).
fn tick(x: f64, y: f64, ux: f64, uy: f64, half: f64) -> [[f64; 2]; 2] {
    [
        [x + uy * half, y - ux * half],
        [x - uy * half, y + ux * half],
    ]
}

/// A line's path between two canvas points. `None` for a zero-length line,
/// which the painter skips (the TypeScript's early return).
#[must_use]
pub fn line_path(kind: LineKind, from: [f64; 2], to: [f64; 2]) -> Option<LinePath> {
    let [x1, y1] = from;
    let [x2, y2] = to;
    let length = (x2 - x1).hypot(y2 - y1);
    if length == 0.0 {
        return None;
    }
    let ux = (x2 - x1) / length;
    let uy = (y2 - y1) / length;
    // The arrow shaft stops its own width short of the tip (`look.width`,
    // owned by h6-handles' painter); every other look runs end to end.
    let short = match kind {
        LineKind::Arrow => length.min(2.5),
        LineKind::Dashed | LineKind::Measured => 0.0,
    };
    let (start_half, end_half) = line_tick_halves(kind);
    let head = match kind {
        LineKind::Arrow => {
            let base_x = x2 - ux * ARROW_HEAD_LENGTH;
            let base_y = y2 - uy * ARROW_HEAD_LENGTH;
            Some([
                [x2, y2],
                [base_x - uy * ARROW_HEAD_HALF, base_y + ux * ARROW_HEAD_HALF],
                [base_x + uy * ARROW_HEAD_HALF, base_y - ux * ARROW_HEAD_HALF],
            ])
        }
        LineKind::Dashed | LineKind::Measured => None,
    };
    Some(LinePath {
        shaft: [[x1, y1], [x2 - ux * short, y2 - uy * short]],
        start_tick: tick(x1, y1, ux, uy, start_half),
        end_tick: tick(x2, y2, ux, uy, end_half),
        head,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::layout_grid;

    /// The 400 by 300 px canvas at scale 0.5 the vitest draws on: 50 px per
    /// unit, the origin at (200, 150) (the `plot.rs` precedent; the margin
    /// is the painter's `labelMargin` and moves neither).
    fn grid_400x300() -> GridLayout {
        layout_grid([0.0, 0.0], 0.5, 400.0, 300.0, 64.0)
    }

    #[test]
    fn plane_units_reach_canvas_px_through_the_grid_origin() {
        let grid = grid_400x300();
        // y = x² through (0.5, 0.25) and (1, 1).
        assert_eq!(to_px(&grid, PlotPoint { x: 0.5, y: 0.25 }), [225.0, 137.5]);
        assert_eq!(to_px(&grid, PlotPoint { x: 1.0, y: 1.0 }), [250.0, 100.0]);
    }

    #[test]
    fn a_curve_far_off_screen_clamps_within_10000px_of_the_canvas() {
        // y = (x / 0.001)³, one clamped sample per px column.
        let grid = grid_400x300();
        let [origin_x, origin_y] = grid.origin;
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        for px in 0..=400 {
            let y = origin_y
                - ((px as f64 - origin_x) / grid.px_per_unit / 0.001).powi(3) * grid.px_per_unit;
            let clamped = clamp_curve_y(y, 300.0);
            min = min.min(clamped);
            max = max.max(clamped);
        }
        assert_eq!(min, -10_000.0);
        assert_eq!(max, 10_300.0);
    }

    #[test]
    fn a_shift_handle_is_a_diamond_around_its_point() {
        // A shift handle at (1, 1), through the grid: (250, 100).
        let at = to_px(&grid_400x300(), PlotPoint { x: 1.0, y: 1.0 });
        assert_eq!(at, [250.0, 100.0]);
        assert_eq!(
            handle_path(HandleKind::Diamond, at),
            HandlePath::Diamond {
                points: [
                    [250.0, 90.0],
                    [260.0, 100.0],
                    [250.0, 110.0],
                    [240.0, 100.0]
                ],
            }
        );
    }

    #[test]
    fn an_arrow_starts_at_a_tick_across_its_tail_and_points_at_its_tip() {
        // From (0, 0) to (1, 0): (200, 150) to (250, 150).
        let path = line_path(LineKind::Arrow, [200.0, 150.0], [250.0, 150.0])
            .expect("a nonzero arrow draws");
        assert_eq!(path.start_tick, [[200.0, 141.0], [200.0, 159.0]]);
        let head = path.head.expect("an arrow draws its head");
        assert_eq!(head[0], [250.0, 150.0]);
        assert_eq!(head.len(), 3);
    }

    #[test]
    fn a_measured_line_has_a_tick_across_each_end() {
        // From (0, 0) to (0, 1): (200, 150) to (200, 100).
        let path = line_path(LineKind::Measured, [200.0, 150.0], [200.0, 100.0])
            .expect("a nonzero line draws");
        assert_eq!(path.start_tick, [[193.0, 150.0], [207.0, 150.0]]);
        assert_eq!(path.end_tick, [[193.0, 100.0], [207.0, 100.0]]);
        assert_eq!(path.head, None);
    }

    /// The shift diamond and the scale square at (1, 1): canvas (250, 100).
    fn shift_handle() -> PlotHandle {
        PlotHandle {
            kind: HandleKind::Diamond,
            at: PlotPoint { x: 1.0, y: 1.0 },
        }
    }

    fn scale_handle() -> PlotHandle {
        PlotHandle {
            kind: HandleKind::Square,
            at: PlotPoint { x: 1.0, y: 1.0 },
        }
    }

    /// The port's own tests: `hitTest` and `fromPx` have no vitest tests,
    /// so these pin the literals (`diamondRadius` 10, `squareSide / 2` 8,
    /// `findLastIndex`) and the inverse.
    #[test]
    fn canvas_px_converts_back_to_plane_units() {
        let grid = grid_400x300();
        assert_eq!(from_px(&grid, [250.0, 100.0]), PlotPoint { x: 1.0, y: 1.0 });
        assert_eq!(from_px(&grid, [200.0, 150.0]), PlotPoint { x: 0.0, y: 0.0 });
        // Round-trips `to_px`, up to float noise.
        let point = PlotPoint { x: 0.37, y: -2.1 };
        let back = from_px(&grid, to_px(&grid, point));
        assert!((back.x - point.x).abs() < 1e-12, "x round-trips");
        assert!((back.y - point.y).abs() < 1e-12, "y round-trips");
    }

    #[test]
    fn a_point_on_the_diamond_edge_hits_and_outside_misses() {
        let grid = grid_400x300();
        let handles = [shift_handle()];
        // |dx| + |dy| = 10 still hits.
        assert_eq!(hit_test(&grid, &handles, [255.0, 105.0]), Some(0));
        assert_eq!(hit_test(&grid, &handles, [260.0, 100.0]), Some(0));
        // |dx| + |dy| = 11 misses.
        assert_eq!(hit_test(&grid, &handles, [256.0, 105.0]), None);
    }

    #[test]
    fn a_point_on_the_square_corner_hits_and_outside_misses() {
        let grid = grid_400x300();
        let handles = [scale_handle()];
        // max(|dx|, |dy|) = 8 still hits.
        assert_eq!(hit_test(&grid, &handles, [258.0, 108.0]), Some(0));
        // max(|dx|, |dy|) = 9 misses.
        assert_eq!(hit_test(&grid, &handles, [258.0, 109.0]), None);
    }

    #[test]
    fn the_last_handle_wins_where_they_overlap() {
        // Both handles at (1, 1): the scale square beats the shift diamond.
        let grid = grid_400x300();
        let handles = [shift_handle(), scale_handle()];
        assert_eq!(hit_test(&grid, &handles, [250.0, 100.0]), Some(1));
    }

    #[test]
    fn a_point_near_no_handle_misses() {
        let grid = grid_400x300();
        assert_eq!(hit_test(&grid, &[shift_handle()], [0.0, 0.0]), None);
        assert_eq!(hit_test(&grid, &[], [250.0, 100.0]), None);
    }
}
