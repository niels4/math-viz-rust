//! The canvas geometry the handle and line painters stroke.
//!
//! Ported from math-viz2 at `5cef052`
//! (`src/components/views/CartesianPlane/drawCartesianPlane.ts`): `toPx`,
//! the overshoot clamp inside `drawCurve` (which arrived in `f1e069f`,
//! inside h6's stretch), `drawHandle`'s shapes and `drawLine`'s shaft,
//! ticks and arrowhead. h7-p-plane adds `fromPx` and `hitTest` at `251c080`.
//! h8-p-marker adds the P/Q marker geometry (`PlotMarker`, the looks, the
//! guide and drop lines, the label layout and the off-frame arrow) at
//! `96f1e84`. Paint (colours, widths, dashes, alphas) stays with
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

/// The arrow shaft's width, in px (`look.width`, owned by h6-handles'
/// painter): the shaft shortens its own width past the head.
pub const ARROW_WIDTH: f64 = 2.5;

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
/// What a hit test returns: a handle slot or a marker slot. Markers are
/// tested after the handles (so a marker beats a coincident handle, as
/// `findLastIndex` over `[...handles, ...markers]` does).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HitTarget {
    Handle(usize),
    Marker(usize),
}

/// The marker hit box in px (`markerHitBox`): a marker is hit when
/// `max(|dx|, |dy|) <= markerHitBox / 2`.
pub const MARKER_HIT_BOX: f64 = 48.0;

#[must_use]
pub fn hit_test(
    grid: &GridLayout,
    handles: &[PlotHandle],
    markers: &[PlotMarker],
    at: [f64; 2],
) -> Option<HitTarget> {
    let mut result = None;
    for (i, handle) in handles.iter().enumerate() {
        let [x, y] = to_px(grid, handle.at);
        let dx = (at[0] - x).abs();
        let dy = (at[1] - y).abs();
        let hit = match handle.kind {
            HandleKind::Diamond => dx + dy <= DIAMOND_RADIUS,
            HandleKind::Square => dx.max(dy) <= SQUARE_SIDE / 2.0,
        };
        if hit {
            result = Some(HitTarget::Handle(i));
        }
    }
    for (i, marker) in markers.iter().enumerate() {
        if !marker.draggable {
            continue;
        }
        let [x, y] = to_px(grid, marker.point);
        let dx = (at[0] - x).abs();
        let dy = (at[1] - y).abs();
        if dx.max(dy) <= MARKER_HIT_BOX / 2.0 {
            result = Some(HitTarget::Marker(i));
        }
    }
    result
}

/// Clamp a canvas value into a range (`clamp` in `grid.ts`, which the draw
/// file imports): `Math.min(hi, Math.max(lo, v))`, so a range gone negative
/// on a narrow canvas pins to its top — `f64::clamp` would panic there.
/// (`grid.rs` keeps its own private copy; this file is h7-p-plane's
/// neighbour, not its owner, so the one-line duplication stands.)
fn clamp(v: f64, lo: f64, hi: f64) -> f64 {
    v.min(hi).max(lo)
}

/// A marker's label: a name and its pre-formatted coords string (`PlotLabel`).
/// The coords string is the app's, built by the painter through
/// `point_format::format_number` as `"(x, y)"`; geometry only measures the
/// two runs it is given.
#[derive(Clone, Debug, PartialEq)]
pub struct PlotLabel {
    pub name: String,
    pub coords: String,
}

/// Which marker a plane point draws (`PlotMarker["shape"]`): P's bullseye or
/// Q's ring. The source leaves the field unset for P, so the default is the
/// bullseye.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MarkerShape {
    #[default]
    Bullseye,
    Ring,
}

/// A point marker on the plane (`PlotMarker`): where it sits in plane units,
/// which theme token its ink is (P's `point_p`, Q's `point_q` — the painter
/// reads `theme.colors[ink]` as the source reads `theme[marker.ink]`), and
/// what rides on it. Everything but the point defaults to off, as the source
/// tests each flag against `true`.
#[derive(Clone, Debug, PartialEq)]
pub struct PlotMarker {
    pub point: PlotPoint,
    pub ink: mv_theme::Token,
    pub shape: MarkerShape,
    pub label: Option<PlotLabel>,
    pub drop_lines: bool,
    pub guide: bool,
    /// Whether the marker can be grabbed and dragged (`PlotMarker["handlers"]`
    /// present at `7686c89`): P carries it, Q never does.
    pub draggable: bool,
}

/// A marker's radius in px (`markerRadius`): the background fill, and the
/// ring's outer edge.
pub const MARKER_RADIUS: f64 = 12.0;

/// A marker look's ring stroke width and core radius in px (`markerLooks`):
/// a bullseye's ring is thin around a wide core, a ring's is wide around a
/// small centre.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MarkerLook {
    pub ring: f64,
    pub core: f64,
}

/// A marker shape's look (`markerLooks[shape ?? "bullseye"]`).
#[must_use]
pub const fn marker_look(shape: MarkerShape) -> MarkerLook {
    match shape {
        MarkerShape::Bullseye => MarkerLook {
            ring: 2.5,
            core: 6.0,
        },
        MarkerShape::Ring => MarkerLook {
            ring: 3.0,
            core: 3.0,
        },
    }
}

/// A guide line's width in px (`guideWidth`): Q's full-height line. The
/// painter owns the width; the dash offset (half the first dash) rides with
/// it.
pub const GUIDE_WIDTH: f64 = 1.0;

/// A guide line's dash in px (`guideDash`), offset by half its first dash.
pub const GUIDE_DASH: [f64; 2] = [4.0, 6.0];

/// A drop line's width in px (`dropWidth`): P's lines to the axes while
/// dragged. The painter owns the width; the dash offset (half the first
/// dash) rides with it.
pub const DROP_WIDTH: f64 = 1.5;

/// A drop line's dash in px (`dropDash`), offset by half its first dash.
pub const DROP_DASH: [f64; 2] = [5.0, 4.0];

/// One of the label's two text faces as a line box in px: Figma's ascent
/// plus descent plus line gap as the height, and the alphabetic baseline
/// measured from the box top (`labelLetter`/`labelNumber`; the fonts stay the
/// painter's). Used as numbers, never measured.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LabelFace {
    pub height: f64,
    pub baseline: f64,
}

/// The label name's face (`labelLetter`).
pub const LABEL_LETTER: LabelFace = LabelFace {
    height: 25.0,
    baseline: 17.74,
};

/// The label coords' face (`labelNumber`).
pub const LABEL_NUMBER: LabelFace = LabelFace {
    height: 22.0,
    baseline: 17.81,
};

/// The label's row height in px (`labelRow`): the larger face height.
pub const LABEL_ROW: f64 = 25.0;

/// The label's horizontal padding in px (`labelPadX`): the inset beside the
/// border on each side.
pub const LABEL_PAD_X: f64 = 12.0;

/// The label's vertical padding in px (`labelPadY`): the inset above and
/// below the row.
pub const LABEL_PAD_Y: f64 = 6.0;

/// The label pill's border width in px (`labelBorder`).
pub const LABEL_BORDER: f64 = 2.0;

/// The gap between the name and coords runs in px (`labelRunGap`).
pub const LABEL_RUN_GAP: f64 = 6.0;

/// How far right of the point the label sits in px (`labelGap`), and how far
/// above it (`labelGap * labelRise`).
pub const LABEL_GAP: f64 = 22.0;

/// The fraction of the label gap the label rises above the point
/// (`labelRise`): 13.2 px.
pub const LABEL_RISE: f64 = 0.6;

/// How far inside the frame the label clamps in px (`labelEdgeMargin`).
pub const LABEL_EDGE_MARGIN: f64 = 12.0;

/// The off-frame arrow's box side in px (`arrowBox`).
pub const ARROW_BOX: f64 = 12.0;

/// The off-frame arrow's triangle side in px (`arrowSize`).
pub const ARROW_SIZE: f64 = 10.0;

/// How far inside the pill the arrow box sits in px (`arrowPadLeft`).
pub const ARROW_PAD_LEFT: f64 = 8.0;

/// The gap between the arrow box and the name run in px (`arrowGap`).
pub const ARROW_GAP: f64 = 8.0;

/// Which frame edge a point lies furthest past (`Direction` in the draw
/// file): the larger overshoot wins, a tie going vertical
/// (`overY >= overX`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

/// Which edge a canvas point lies past (`offEdge`): `None` on the frame
/// (edges included), else the side of the larger overshoot — up or down on
/// a tie.
#[must_use]
pub fn off_edge(grid: &GridLayout, px: f64, py: f64) -> Option<Direction> {
    let over_x = if px < 0.0 {
        -px
    } else if px > grid.width {
        px - grid.width
    } else {
        0.0
    };
    let over_y = if py < 0.0 {
        -py
    } else if py > grid.height {
        py - grid.height
    } else {
        0.0
    };
    if over_x == 0.0 && over_y == 0.0 {
        return None;
    }
    if over_y >= over_x {
        Some(if py < 0.0 {
            Direction::Up
        } else {
            Direction::Down
        })
    } else if px < 0.0 {
        Some(Direction::Left)
    } else {
        Some(Direction::Right)
    }
}

/// The canvas x of a marker's guide line (`drawGuide`'s guard): `Some` while
/// the marker's x lies on the plane (`0 <= px <= width`), so the line runs
/// the plane's full height whatever the marker's y is; `None` past either
/// edge.
#[must_use]
pub fn guide_x(grid: &GridLayout, point: PlotPoint) -> Option<f64> {
    let [px, _] = to_px(grid, point);
    (0.0 <= px && px <= grid.width).then_some(px)
}

/// A marker's drop lines to the axes in canvas px (`drawDropLines` without
/// the paint): the vertical runs from the marker (clamped onto the frame) to
/// the x-axis when the marker's x is on the plane, the horizontal from the
/// marker (likewise clamped) to the y-axis when its y is. Each is `None`
/// exactly when the source strokes nothing for it; the painter strokes both
/// in one path, vertical first.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DropLines {
    pub vertical: Option<[[f64; 2]; 2]>,
    pub horizontal: Option<[[f64; 2]; 2]>,
}

/// A marker's drop lines in canvas px (`drawDropLines` without the paint).
#[must_use]
pub fn drop_lines(grid: &GridLayout, point: PlotPoint) -> DropLines {
    let [px, py] = to_px(grid, point);
    let [origin_x, origin_y] = grid.origin;
    DropLines {
        vertical: (0.0 <= px && px <= grid.width)
            .then_some([[px, clamp(py, 0.0, grid.height)], [px, origin_y]]),
        horizontal: (0.0 <= py && py <= grid.height)
            .then_some([[clamp(px, 0.0, grid.width), py], [origin_x, py]]),
    }
}

/// Whose ink one of a marker's three arcs takes: the painter resolves
/// `Background` to the plane background, `Foreground` to the foreground,
/// and `Marker` to `theme.colors[marker.ink]` — the source's bullseye/ring
/// swap, kept beside the looks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkerInk {
    Background,
    Foreground,
    Marker,
}

/// One of a marker's three concentric arcs: its centre, its radius, and whose
/// ink it takes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MarkerArc {
    pub at: [f64; 2],
    pub r: f64,
    pub ink: MarkerInk,
}

/// A marker's three arcs in canvas px (`drawMarker` without the paint): the
/// background fill at the marker radius, the ring at the radius less half
/// its width, and the filled core. A bullseye's ring takes the foreground
/// and its core the marker's ink; a ring's go the other way round.
#[must_use]
pub fn marker_arcs(center_px: [f64; 2], shape: MarkerShape) -> [MarkerArc; 3] {
    let look = marker_look(shape);
    let (ring_ink, core_ink) = match shape {
        MarkerShape::Bullseye => (MarkerInk::Foreground, MarkerInk::Marker),
        MarkerShape::Ring => (MarkerInk::Marker, MarkerInk::Foreground),
    };
    [
        MarkerArc {
            at: center_px,
            r: MARKER_RADIUS,
            ink: MarkerInk::Background,
        },
        MarkerArc {
            at: center_px,
            r: MARKER_RADIUS - look.ring / 2.0,
            ink: ring_ink,
        },
        MarkerArc {
            at: center_px,
            r: look.core,
            ink: core_ink,
        },
    ]
}

/// A marker label's pill in canvas px (`drawLabel` without the paint and
/// without measuring): its box, which way the marker lies when off the
/// frame, the text's left edge and two alphabetic baselines, and the
/// off-frame arrow's box top-left (`None` while the marker is in view). The
/// two text runs' widths arrive measured (`Math.ceil`ed, as the source ceils
/// `measureText`); the painter resolves the pill fill, border, arrow and
/// text inks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LabelLayout {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub dir: Option<Direction>,
    pub text_left: f64,
    pub name_baseline: f64,
    pub coords_baseline: f64,
    pub arrow: Option<[f64; 2]>,
}

/// A marker label's pill in canvas px (`drawLabel` without the paint).
#[must_use]
pub fn label_layout(
    grid: &GridLayout,
    px: f64,
    py: f64,
    name_width: f64,
    coords_width: f64,
) -> LabelLayout {
    fn baseline(face: LabelFace, row_top: f64) -> f64 {
        row_top + (LABEL_ROW - face.height) / 2.0 + face.baseline
    }

    let dir = off_edge(grid, px, py);
    let inset = LABEL_BORDER + LABEL_PAD_X;
    let lead = match dir {
        None => 0.0,
        Some(_) => ARROW_PAD_LEFT - LABEL_PAD_X + ARROW_BOX + ARROW_GAP,
    };
    let w = 2.0 * inset + lead + name_width + LABEL_RUN_GAP + coords_width;
    let h = 2.0 * (LABEL_BORDER + LABEL_PAD_Y) + LABEL_ROW;
    let x = clamp(
        px + LABEL_GAP,
        LABEL_EDGE_MARGIN,
        grid.width - LABEL_EDGE_MARGIN - w,
    );
    let y = clamp(
        py - LABEL_GAP * LABEL_RISE - h,
        LABEL_EDGE_MARGIN,
        grid.height - LABEL_EDGE_MARGIN - h,
    );
    let row_top = y + LABEL_BORDER + LABEL_PAD_Y;
    let arrow = dir.map(|_| {
        let in_box = (ARROW_BOX - ARROW_SIZE) / 2.0;
        [
            x + LABEL_BORDER + ARROW_PAD_LEFT + in_box,
            row_top + (LABEL_ROW - ARROW_BOX) / 2.0 + in_box,
        ]
    });
    LabelLayout {
        x,
        y,
        w,
        h,
        dir,
        text_left: x + inset + lead,
        name_baseline: baseline(LABEL_LETTER, row_top),
        coords_baseline: baseline(LABEL_NUMBER, row_top),
        arrow,
    }
}

/// An off-frame arrow's filled triangle (`drawArrow`): the three corners from
/// the arrow box's top-left, tip first in each direction's order.
#[must_use]
pub fn arrow_path(dir: Direction, x: f64, y: f64) -> [[f64; 2]; 3] {
    let s = ARROW_SIZE;
    match dir {
        Direction::Up => [[x + s / 2.0, y], [x + s, y + s], [x, y + s]],
        Direction::Down => [[x, y], [x + s, y], [x + s / 2.0, y + s]],
        Direction::Left => [[x + s, y], [x + s, y + s], [x, y + s / 2.0]],
        Direction::Right => [[x, y], [x + s, y + s / 2.0], [x, y + s]],
    }
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
        LineKind::Arrow => length.min(ARROW_WIDTH),
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
        assert_eq!(
            hit_test(&grid, &handles, &[], [255.0, 105.0]),
            Some(HitTarget::Handle(0))
        );
        assert_eq!(
            hit_test(&grid, &handles, &[], [260.0, 100.0]),
            Some(HitTarget::Handle(0))
        );
        // |dx| + |dy| = 11 misses.
        assert_eq!(hit_test(&grid, &handles, &[], [256.0, 105.0]), None);
    }

    #[test]
    fn a_point_on_the_square_corner_hits_and_outside_misses() {
        let grid = grid_400x300();
        let handles = [scale_handle()];
        // max(|dx|, |dy|) = 8 still hits.
        assert_eq!(
            hit_test(&grid, &handles, &[], [258.0, 108.0]),
            Some(HitTarget::Handle(0))
        );
        // max(|dx|, |dy|) = 9 misses.
        assert_eq!(hit_test(&grid, &handles, &[], [258.0, 109.0]), None);
    }

    #[test]
    fn the_last_handle_wins_where_they_overlap() {
        // Both handles at (1, 1): the scale square beats the shift diamond.
        let grid = grid_400x300();
        let handles = [shift_handle(), scale_handle()];
        assert_eq!(
            hit_test(&grid, &handles, &[], [250.0, 100.0]),
            Some(HitTarget::Handle(1))
        );
    }

    #[test]
    fn a_point_near_no_handle_misses() {
        let grid = grid_400x300();
        assert_eq!(hit_test(&grid, &[shift_handle()], &[], [0.0, 0.0]), None);
        assert_eq!(hit_test(&grid, &[], &[], [250.0, 100.0]), None);
    }

    /// A draggable marker (P) is hit within its 48 px box (half 24).
    #[test]
    fn a_point_inside_p_s_48_px_box_hits_point_p() {
        let grid = grid_400x300();
        let marker = PlotMarker {
            point: PlotPoint { x: 1.0, y: 1.0 },
            ink: mv_theme::Token::PointP,
            shape: MarkerShape::Bullseye,
            label: None,
            drop_lines: false,
            guide: false,
            draggable: true,
        };
        // center is (250, 100); 23 px off still inside (half 24).
        assert_eq!(
            hit_test(
                &grid,
                &[],
                std::slice::from_ref(&marker),
                [250.0 + 23.0, 100.0]
            ),
            Some(HitTarget::Marker(0))
        );
        // 25 px off misses.
        assert_eq!(hit_test(&grid, &[], &[marker], [250.0 + 25.0, 100.0]), None);
    }

    /// A handle still wins away from P.
    #[test]
    fn a_handle_wins_away_from_p() {
        let grid = grid_400x300();
        let marker = PlotMarker {
            point: PlotPoint { x: 0.0, y: 0.0 },
            ink: mv_theme::Token::PointP,
            shape: MarkerShape::Bullseye,
            label: None,
            drop_lines: false,
            guide: false,
            draggable: true,
        };
        // marker at (0,0) plane = (200,150) px, well outside its 24 px box
        // from the handle at (1,1); the handle's edge still hits.
        assert_eq!(
            hit_test(&grid, &[shift_handle()], &[marker], [260.0, 100.0]),
            Some(HitTarget::Handle(0))
        );
    }

    /// P wins where it coincides with a handle (last-index rule).
    #[test]
    fn p_wins_where_it_coincides_with_a_handle() {
        let grid = grid_400x300();
        let marker = PlotMarker {
            point: PlotPoint { x: 1.0, y: 1.0 },
            ink: mv_theme::Token::PointP,
            shape: MarkerShape::Bullseye,
            label: None,
            drop_lines: false,
            guide: false,
            draggable: true,
        };
        // both at (1,1) plane = (250,100) px; marker tested after handles -> Marker wins.
        assert_eq!(
            hit_test(&grid, &[shift_handle()], &[marker], [250.0, 100.0]),
            Some(HitTarget::Marker(0))
        );
    }

    /// A marker with P's ink at plane units, unset shape and nothing riding
    /// on it (the vitest draws `{ point, ink: "ordinal05" }`).
    fn plain_marker(x: f64, y: f64) -> PlotMarker {
        PlotMarker {
            point: PlotPoint { x, y },
            ink: mv_theme::Token::Ordinal05,
            shape: MarkerShape::default(),
            label: None,
            drop_lines: false,
            guide: false,
            draggable: false,
        }
    }

    /// `toBeCloseTo` without a precision is two decimal places.
    fn close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 0.005,
            "{actual} is not close to {expected}"
        );
    }

    #[test]
    fn p_at_1_1_is_a_bullseye_around_250_100_with_a_core_in_its_ink() {
        // The default shape is the bullseye.
        let grid = grid_400x300();
        let marker = plain_marker(1.0, 1.0);
        let center = to_px(&grid, marker.point);
        assert_eq!(center, [250.0, 100.0]);

        let arcs = marker_arcs(center, marker.shape);
        assert_eq!(arcs.map(|arc| arc.at), [center; 3]);
        assert_eq!(
            arcs[1],
            MarkerArc {
                at: center,
                r: 10.75,
                ink: MarkerInk::Foreground,
            }
        );
        assert_eq!(
            arcs[2],
            MarkerArc {
                at: center,
                r: 6.0,
                ink: MarkerInk::Marker,
            }
        );
    }

    #[test]
    fn p_s_label_sits_22_px_right_of_the_point_its_bottom_13_2_px_above_it() {
        // The harness's measureText stub returns width 40 for every run.
        let grid = grid_400x300();
        let [px, py] = to_px(&grid, PlotPoint { x: 1.0, y: 1.0 });
        let layout = label_layout(&grid, px, py, 40.0, 40.0);

        assert_eq!((layout.x, layout.w, layout.h), (272.0, 114.0, 41.0));
        close(layout.y, 45.8);
        assert_eq!(layout.dir, None);
        assert_eq!(layout.arrow, None);
        assert_eq!(layout.text_left, 286.0);
        assert_eq!(layout.text_left + 40.0 + LABEL_RUN_GAP, 332.0);
    }

    #[test]
    fn p_s_label_stays_12_px_inside_the_frame_on_each_side_keeping_its_place_along_that_side() {
        let grid = grid_400x300();
        let label_at = |x: f64, y: f64| {
            let [px, py] = to_px(&grid, PlotPoint { x, y });
            let layout = label_layout(&grid, px, py, 40.0, 40.0);
            (layout.x, layout.y)
        };

        assert_eq!(label_at(-1.0, 20.0), (172.0, 12.0));
        assert_eq!(label_at(-1.0, -20.0), (172.0, 247.0));
        let (left, top) = label_at(-20.0, 1.0);
        assert_eq!(left, 12.0);
        close(top, 45.8);
        let (right, top) = label_at(20.0, 1.0);
        assert_eq!(right, 258.0);
        close(top, 45.8);
    }

    #[test]
    fn p_s_label_leads_with_a_triangle_pointing_the_way_p_lies_furthest_past_the_view() {
        let grid = grid_400x300();
        let arrow_of = |x: f64, y: f64| {
            let [px, py] = to_px(&grid, PlotPoint { x, y });
            let layout = label_layout(&grid, px, py, 40.0, 40.0);
            let [ax, ay] = layout.arrow.expect("an off-frame label draws its arrow");
            let dir = layout.dir.expect("an off-frame label knows its side");
            arrow_path(dir, ax, ay).map(|[qx, qy]| [qx - layout.x, qy - layout.y])
        };

        assert_eq!(
            arrow_of(-1.0, 20.0),
            [[16.0, 15.5], [21.0, 25.5], [11.0, 25.5]]
        );
        assert_eq!(
            arrow_of(-1.0, -20.0),
            [[11.0, 15.5], [21.0, 15.5], [16.0, 25.5]]
        );
        assert_eq!(
            arrow_of(-20.0, 1.0),
            [[21.0, 15.5], [21.0, 25.5], [11.0, 20.5]]
        );
        assert_eq!(
            arrow_of(20.0, 1.0),
            [[11.0, 15.5], [21.0, 20.5], [11.0, 25.5]]
        );
        // The tie goes vertical: as far past in x as in y still points up.
        assert_eq!(arrow_of(20.0, 30.0), arrow_of(-1.0, 20.0));
    }

    #[test]
    fn p_s_label_has_no_triangle_while_p_is_in_view_and_makes_room_for_it_when_p_is_not() {
        let grid = grid_400x300();
        let labelled = |x: f64, y: f64| {
            let [px, py] = to_px(&grid, PlotPoint { x, y });
            label_layout(&grid, px, py, 40.0, 40.0)
        };

        let in_view = labelled(1.0, 1.0);
        assert_eq!(in_view.w, 114.0);
        let above = labelled(1.0, 20.0);
        assert_eq!(above.w, 130.0);
        assert_eq!(above.text_left, above.x + 30.0);
    }

    #[test]
    fn p_s_drop_lines_run_in_its_ink_from_1_1_to_each_axis() {
        // The ink is the painter's (`theme.colors[marker.ink]`); geometry
        // gives the two segments the painter strokes in one path.
        let grid = grid_400x300();
        let lines = drop_lines(&grid, PlotPoint { x: 1.0, y: 1.0 });
        assert_eq!(lines.vertical, Some([[250.0, 100.0], [250.0, 150.0]]));
        assert_eq!(lines.horizontal, Some([[250.0, 100.0], [200.0, 100.0]]));
    }

    #[test]
    fn above_the_view_p_s_drop_line_comes_in_from_the_top_edge_and_none_runs_off_the_canvas() {
        let grid = grid_400x300();
        let lines = drop_lines(&grid, PlotPoint { x: 1.0, y: 20.0 });
        assert_eq!(lines.vertical, Some([[250.0, 0.0], [250.0, 150.0]]));
        assert_eq!(lines.horizontal, None);
    }

    #[test]
    fn p_has_no_drop_lines_unless_asked() {
        // The painter only strokes a marker whose `drop_lines` is set; the
        // flag defaults to off.
        assert!(!plain_marker(1.0, 1.0).drop_lines);
    }

    #[test]
    fn q_at_1_1_is_a_ring_in_its_ink_around_250_100_with_a_centre_in_the_foreground() {
        let grid = grid_400x300();
        let center = to_px(&grid, PlotPoint { x: 1.0, y: 1.0 });
        assert_eq!(center, [250.0, 100.0]);

        let arcs = marker_arcs(center, MarkerShape::Ring);
        assert_eq!(
            arcs.map(|arc| [arc.at[0], arc.at[1], arc.r]),
            [
                [250.0, 100.0, 12.0],
                [250.0, 100.0, 10.5],
                [250.0, 100.0, 3.0],
            ]
        );
        // A ring's ring is the marker's ink, its core the foreground.
        assert_eq!(arcs[1].ink, MarkerInk::Marker);
        assert_eq!(arcs[2].ink, MarkerInk::Foreground);
    }

    #[test]
    fn q_s_guide_runs_down_the_plane_s_full_height_at_its_x_even_while_q_is_above_the_view() {
        let grid = grid_400x300();
        assert_eq!(guide_x(&grid, PlotPoint { x: 1.0, y: 20.0 }), Some(250.0));
    }

    #[test]
    fn q_has_no_guide_while_its_x_lies_past_the_view() {
        let grid = grid_400x300();
        assert_eq!(guide_x(&grid, PlotPoint { x: 20.0, y: 1.0 }), None);
    }
}
