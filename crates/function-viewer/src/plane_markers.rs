//! The P and Q markers on the plane: the painters.
//!
//! Ported from `src/components/views/CartesianPlane/draw/drawCartesianPlane.ts`
//! at `96f1e84` (`drawGuide`, `drawDropLines`, `drawMarker`, `drawLabel`,
//! `drawArrow` and the marker order in `drawCartesianPlane`). Positions come
//! from [`plane_geometry`](crate::plane_geometry) (h8-p-marker's: `guide_x`,
//! `drop_lines`, `marker_arcs`, `label_layout`, `arrow_path`); this module
//! only batches them into strokes, fills and shaped text, as `plane.rs` does
//! for the grid. Painted after the handles, before the legend: the guides of
//! the markers with `guide`, then the drop lines of those with `drop_lines`,
//! then each marker's arcs and, once the font has loaded, its label.
//!
//! Dashes go through h6-curve's [`dash_pieces`](crate::plane::dash_pieces),
//! never a second emulation. All positions are canvas-relative px; the caller
//! translates to the content box, as the other painters do.
//!
//! Draft (phase 1): h8-p-marker's geometry names are its prompt's contract;
//! the scratch-tree check carries spec-derived stubs until it merges, and
//! phase 2 reconciles any name the porter fixed differently.

use mv_text::{Face, Features, Style};

use crate::grid::GridLayout;
use crate::plane_geometry::{
    DROP_DASH, GUIDE_DASH, LabelLayout, MarkerArcs, MarkerShape, PlotMarker, arrow_path,
    drop_lines, guide_x, label_layout, marker_arcs,
};

/// The P marker's name face: STIX Two Text italic 400, 20 px
/// (`labelLetterFont`; `Type::MathsItalic`'s face).
pub fn marker_name_style() -> Style {
    Style {
        face: Face::Stix400Italic,
        size: 20.0,
        tracking: 0.0,
        features: Features::new(),
    }
}

/// The marker labels' coords face: Roboto Mono 500, 17 px
/// (`labelNumberFont`; `Type::FieldText`'s face).
pub fn marker_coords_style() -> Style {
    Style {
        face: Face::RobotoMono500,
        size: 17.0,
        tracking: 0.0,
        features: Features::new(),
    }
}

/// The guide's look (`drawGuide`): 1 px, `[4, 6]` dashes from half a dash in.
pub const GUIDE_WIDTH: f32 = 1.0;

/// Where the guide's pattern starts (`lineDashOffset`: half the first dash).
pub const GUIDE_OFFSET: f64 = 2.0;

/// The drop lines' look (`drawDropLines`): 1.5 px, `[5, 4]` dashes from half
/// a dash in.
pub const DROP_WIDTH: f32 = 1.5;

/// Where the drop lines' pattern starts (half the first dash).
pub const DROP_OFFSET: f64 = 2.5;

/// The label pill's border width, in px (`labelBorder`).
pub const LABEL_BORDER: f32 = 2.0;

/// The gap between the label's name and coords runs, in px (`labelRunGap`).
pub const LABEL_RUN_GAP: f64 = 6.0;

/// The colours the markers paint with, resolved from the theme: the two point
/// inks (`point_p`/`point_q`, the sidebar's `--point-p`/`--point-q`), the
/// marker ring and label text (`foreground`), the guide (`foregroundMuted`),
/// the marker's background fill (`background`) and the label pill (`card`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MarkerColors {
    pub point_p: [f32; 4],
    pub point_q: [f32; 4],
    pub foreground: [f32; 4],
    pub foreground_muted: [f32; 4],
    pub background: [f32; 4],
    pub card: [f32; 4],
}

/// The marker colours from the resolved theme.
pub fn marker_colors(theme: &mv_theme::Theme) -> MarkerColors {
    MarkerColors {
        point_p: theme.colors[mv_theme::Token::PointP],
        point_q: theme.colors[mv_theme::Token::PointQ],
        foreground: theme.colors[mv_theme::Token::Foreground],
        foreground_muted: theme.colors[mv_theme::Token::ForegroundMuted],
        background: theme.colors[mv_theme::Token::Background],
        card: theme.colors[mv_theme::Token::Card],
    }
}

/// A marker's ink: `PlotMarker.ink` is always one of the two point tokens.
pub fn marker_ink(colors: &MarkerColors, marker: &PlotMarker) -> [f32; 4] {
    if marker.ink == mv_theme::Token::PointQ {
        colors.point_q
    } else {
        colors.point_p
    }
}

/// A marker's ring and core inks: a bullseye's ring is the foreground and its
/// core the marker's ink, a ring's the other way round (`drawMarker`).
pub fn marker_arc_inks(
    shape: MarkerShape,
    ink: [f32; 4],
    foreground: [f32; 4],
) -> ([f32; 4], [f32; 4]) {
    match shape {
        MarkerShape::Bullseye => (foreground, ink),
        MarkerShape::Ring => (ink, foreground),
    }
}

/// Strokes dashed pieces with the look already on the context: one path per
/// piece, since Canvas caps each dash with the look's caps.
fn stroke_pieces(vg: &mut mv_nvg::Context, pieces: &[Vec<[f64; 2]>]) {
    for piece in pieces {
        vg.begin_path();
        for (i, [x, y]) in piece.iter().enumerate() {
            // NanoVG drops a path with no move-to (`plane.rs`'s curves).
            if i == 0 {
                vg.move_to(*x as f32, *y as f32);
            } else {
                vg.line_to(*x as f32, *y as f32);
            }
        }
        vg.stroke();
    }
}

/// Paints a marker's guide: its px over the frame's full height (`drawGuide`).
pub fn paint_guide(
    vg: &mut mv_nvg::Context,
    grid: &GridLayout,
    marker: &PlotMarker,
    colors: &MarkerColors,
) {
    let Some(px) = guide_x(grid, marker.point) else {
        return;
    };
    vg.line_cap(mv_nvg::LineCapType::BUTT);
    vg.stroke_color(colors.foreground_muted);
    vg.stroke_width(GUIDE_WIDTH);
    stroke_pieces(
        vg,
        &crate::plane::dash_pieces(
            &[[f64::from(px), 0.0], [f64::from(px), grid.height]],
            GUIDE_DASH,
            GUIDE_OFFSET,
        ),
    );
}

/// Paints a marker's drop lines to the axes, clamped at the frame
/// (`drawDropLines`).
pub fn paint_drop_lines(
    vg: &mut mv_nvg::Context,
    grid: &GridLayout,
    marker: &PlotMarker,
    colors: &MarkerColors,
) {
    vg.line_cap(mv_nvg::LineCapType::BUTT);
    vg.stroke_color(marker_ink(colors, marker));
    vg.stroke_width(DROP_WIDTH);
    for segment in drop_lines(grid, marker.point) {
        stroke_pieces(
            vg,
            &crate::plane::dash_pieces(&segment, DROP_DASH, DROP_OFFSET),
        );
    }
}

/// Paints a marker's three arcs at its canvas position: the background fill,
/// the ring stroke, the core fill (`drawMarker`).
pub fn paint_marker(
    vg: &mut mv_nvg::Context,
    arcs: &MarkerArcs,
    ring_ink: [f32; 4],
    core_ink: [f32; 4],
    background: [f32; 4],
) {
    let [x, y] = arcs.center;
    vg.begin_path();
    vg.circle(x as f32, y as f32, arcs.back_r as f32);
    vg.fill_color(background);
    vg.fill();
    vg.begin_path();
    vg.circle(x as f32, y as f32, arcs.ring_r as f32);
    vg.stroke_color(ring_ink);
    vg.stroke_width(arcs.ring_width as f32);
    vg.stroke();
    vg.begin_path();
    vg.circle(x as f32, y as f32, arcs.core_r as f32);
    vg.fill_color(core_ink);
    vg.fill();
}

/// The label texts' pens from a laid-out label: the name at the layout's left
/// on its baseline, the coords past the name's measured width
/// (`textLeft + nameWidth + labelRunGap`).
pub fn label_text_xs(layout: &LabelLayout, name_width: f64) -> (f32, f32) {
    let name_x = layout.text_left as f32;
    let coords_x = (layout.text_left + name_width + LABEL_RUN_GAP) as f32;
    (name_x, coords_x)
}

/// Paints a marker's label: the pill in the card colour with a 2 px border in
/// the marker's ink, the off-frame arrow in the ink, the name and coords at
/// their literal baselines (`drawLabel`, `drawArrow`). `name_width` and
/// `coords_width` are the ceiled measures `label_layout` laid out with.
// Eight paint inputs (the marker and its runs with their layout, widths and
// inks): the call reads in draw order, as `paint_plane`'s does.
#[allow(clippy::too_many_arguments)]
pub fn paint_label(
    fonts: &mut mv_text::Fonts,
    vg: &mut mv_nvg::Context,
    marker: &PlotMarker,
    name: &str,
    coords: &str,
    layout: &LabelLayout,
    name_width: f64,
    colors: &MarkerColors,
) {
    let ink = marker_ink(colors, marker);
    let [x, y, w, h] = layout.rect;
    // The pill and its border, stroked inset by half the border.
    vg.begin_path();
    vg.rounded_rect(x as f32, y as f32, w as f32, h as f32, (h / 2.0) as f32);
    vg.fill_color(colors.card);
    vg.fill();
    let half = f64::from(LABEL_BORDER) / 2.0;
    vg.line_cap(mv_nvg::LineCapType::BUTT);
    vg.stroke_color(ink);
    vg.stroke_width(LABEL_BORDER);
    vg.begin_path();
    vg.rounded_rect(
        (x + half) as f32,
        (y + half) as f32,
        (w - f64::from(LABEL_BORDER)) as f32,
        (h - f64::from(LABEL_BORDER)) as f32,
        (h / 2.0 - half) as f32,
    );
    vg.stroke();
    // The off-frame arrow, filled in the ink.
    if let (Some(dir), Some(at)) = (layout.dir, layout.arrow) {
        vg.begin_path();
        let mut corners = arrow_path(dir, at).into_iter();
        if let Some([first_x, first_y]) = corners.next() {
            vg.move_to(first_x as f32, first_y as f32);
            for [corner_x, corner_y] in corners {
                vg.line_to(corner_x as f32, corner_y as f32);
            }
            vg.close_path();
            vg.fill_color(ink);
            vg.fill();
        }
    }
    // The runs at their literal baselines.
    let (name_x, coords_x) = label_text_xs(layout, name_width);
    mv_text::draw(
        fonts,
        vg,
        marker_name_style(),
        name_x,
        layout.name_baseline as f32,
        name,
        colors.foreground,
    );
    mv_text::draw(
        fonts,
        vg,
        marker_coords_style(),
        coords_x,
        layout.coords_baseline as f32,
        coords,
        colors.foreground,
    );
}

/// Paints the frame's markers in the source's order: the guides of the
/// markers with `guide`, then the drop lines of those with `drop_lines`, then
/// each marker's arcs and, once the font has loaded, its label. Widths are
/// measured as `ceil(measureText.width)` through the port's text measure.
pub fn paint_markers(
    fonts: &mut mv_text::Fonts,
    vg: &mut mv_nvg::Context,
    grid: &GridLayout,
    markers: &[PlotMarker],
    colors: &MarkerColors,
    font_ready: bool,
) {
    for marker in markers.iter().filter(|marker| marker.guide) {
        paint_guide(vg, grid, marker, colors);
    }
    for marker in markers.iter().filter(|marker| marker.drop_lines) {
        paint_drop_lines(vg, grid, marker, colors);
    }
    for marker in markers {
        let at = crate::plane_geometry::to_px(grid, marker.point);
        let (ring_ink, core_ink) =
            marker_arc_inks(marker.shape, marker_ink(colors, marker), colors.foreground);
        paint_marker(
            vg,
            &marker_arcs(at, marker.shape),
            ring_ink,
            core_ink,
            colors.background,
        );
        if font_ready && let Some(label) = &marker.label {
            let name_width =
                f64::from(mv_text::measure(fonts, marker_name_style(), &label.name)).ceil();
            let coords_width = f64::from(mv_text::measure(
                fonts,
                marker_coords_style(),
                &label.coords,
            ))
            .ceil();
            let layout = label_layout(grid, at[0], at[1], name_width, coords_width);
            paint_label(
                fonts,
                vg,
                marker,
                &label.name,
                &label.coords,
                &layout,
                name_width,
                colors,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::layout_grid;
    use crate::plane_geometry::{Direction, PlotLabel};
    use crate::plot::PlotPoint;

    /// The 400 by 300 px canvas at scale 0.5 the vitest draws on
    /// (`plane_geometry.rs`'s precedent).
    fn grid_400x300() -> GridLayout {
        layout_grid([0.0, 0.0], 0.5, 400.0, 300.0, 64.0)
    }

    fn marker(point: PlotPoint) -> PlotMarker {
        PlotMarker {
            point,
            ink: mv_theme::Token::PointP,
            shape: MarkerShape::Bullseye,
            label: None,
            drop_lines: false,
            guide: false,
        }
    }

    /// The marker label's faces are the sidebar's own: the name in STIX Two
    /// Text italic 400 20 px (`Type::MathsItalic`), the coords in Roboto Mono
    /// 500 17 px (`Type::FieldText`).
    #[test]
    fn label_faces_match_the_type_scale() {
        let name = marker_name_style();
        assert_eq!(name.face, Face::Stix400Italic);
        assert_eq!(name.size, 20.0);
        let coords = marker_coords_style();
        assert_eq!(coords.face, Face::RobotoMono500);
        assert_eq!(coords.size, 17.0);
    }

    /// A bullseye's ring is the foreground and its core the marker's ink; a
    /// ring's the other way round.
    #[test]
    fn arc_inks_swap_by_shape() {
        let ink = [0.0, 1.0, 0.0, 1.0];
        let foreground = [0.0, 0.0, 0.0, 1.0];
        assert_eq!(
            marker_arc_inks(MarkerShape::Bullseye, ink, foreground),
            (foreground, ink)
        );
        assert_eq!(
            marker_arc_inks(MarkerShape::Ring, ink, foreground),
            (ink, foreground)
        );
    }

    /// P at (1, 1) is a bullseye around (250, 100) with a core in its ink.
    #[test]
    fn p_marker_is_a_bullseye_with_its_core_in_its_ink() {
        let at = crate::plane_geometry::to_px(&grid_400x300(), PlotPoint { x: 1.0, y: 1.0 });
        assert_eq!(at, [250.0, 100.0]);
        let arcs = marker_arcs(at, MarkerShape::Bullseye);
        assert_eq!(arcs.center, [250.0, 100.0]);
        assert_eq!(arcs.back_r, 12.0);
        assert_eq!(arcs.ring_r, 12.0 - 2.5 / 2.0);
        assert_eq!(arcs.core_r, 6.0);
    }

    /// Q at (1, 1) is a ring in its ink around (250, 100) with a centre in
    /// the foreground.
    #[test]
    fn q_marker_is_a_ring_with_a_foreground_core() {
        let at = crate::plane_geometry::to_px(&grid_400x300(), PlotPoint { x: 1.0, y: 1.0 });
        let arcs = marker_arcs(at, MarkerShape::Ring);
        assert_eq!([arcs.back_r, arcs.ring_r, arcs.core_r], [12.0, 10.5, 3.0]);
        assert_eq!(arcs.ring_width, 3.0);
    }

    /// P's label sits 22 px right of the point, its bottom 13.2 px above it:
    /// the pill at (272, ~45.8), 114 by 41, "P" at x 286 and the coords at
    /// 332 under the harness's measureText stub (every width 40).
    #[test]
    fn label_sits_right_of_the_point() {
        let grid = grid_400x300();
        let layout = label_layout(&grid, 250.0, 100.0, 40.0, 40.0);
        let [x, y, w, h] = layout.rect;
        assert_eq!([x, w, h], [272.0, 114.0, 41.0]);
        assert!((y - 45.8).abs() < 0.05, "label y {y}");
        assert_eq!(layout.dir, None);
        let (name_x, coords_x) = label_text_xs(&layout, 40.0);
        assert_eq!([name_x, coords_x], [286.0, 332.0]);
    }

    /// P's label stays 12 px inside the frame on each side, keeping its place
    /// along that side.
    #[test]
    fn label_clamps_inside_the_frame() {
        let grid = grid_400x300();
        let left_of = |x: f64, y: f64| label_layout(&grid, x, y, 40.0, 40.0).rect[0];
        let top_of = |x: f64, y: f64| label_layout(&grid, x, y, 40.0, 40.0).rect[1];
        // (-1, 20) and (-1, -20) in plot units through the grid.
        let hi = crate::plane_geometry::to_px(&grid, PlotPoint { x: -1.0, y: 20.0 });
        let lo = crate::plane_geometry::to_px(&grid, PlotPoint { x: -1.0, y: -20.0 });
        assert_eq!([left_of(hi[0], hi[1]), top_of(hi[0], hi[1])], [172.0, 12.0]);
        assert_eq!(
            [left_of(lo[0], lo[1]), top_of(lo[0], lo[1])],
            [172.0, 247.0]
        );
    }

    /// P's label leads with a triangle pointing the way P lies furthest past
    /// the view.
    #[test]
    fn off_view_label_leads_with_an_arrow() {
        let grid = grid_400x300();
        let layout = label_layout(&grid, 150.0, -850.0, 40.0, 40.0);
        assert_eq!(layout.dir, Some(Direction::Up));
        let [x, y, w, _] = layout.rect;
        assert_eq!(w, 130.0);
        let at = layout.arrow.expect("an off-view label carries its arrow");
        let corners = arrow_path(Direction::Up, at);
        assert_eq!(
            corners.map(|[px, py]| [px - x, py - y]),
            [[16.0, 15.5], [21.0, 25.5], [11.0, 25.5]]
        );
    }

    /// P's drop lines run in its ink from (1, 1) to each axis; above the view
    /// the vertical comes in from the top edge and none runs off the canvas.
    #[test]
    fn drop_lines_run_to_the_axes() {
        let grid = grid_400x300();
        assert_eq!(
            drop_lines(&grid, PlotPoint { x: 1.0, y: 1.0 }),
            vec![
                [[250.0, 100.0], [250.0, 150.0]],
                [[250.0, 100.0], [200.0, 100.0]],
            ]
        );
        assert_eq!(
            drop_lines(&grid, PlotPoint { x: 1.0, y: 20.0 }),
            vec![[[250.0, 0.0], [250.0, 150.0]]]
        );
    }

    /// Q's guide runs down the plane's full height at its x, even while Q is
    /// above the view; none shows while its x lies past the view.
    #[test]
    fn guide_runs_the_full_height() {
        let grid = grid_400x300();
        assert_eq!(guide_x(&grid, PlotPoint { x: 1.0, y: 20.0 }), Some(250.0));
        assert_eq!(guide_x(&grid, PlotPoint { x: 20.0, y: 1.0 }), None);
    }

    /// The label's coords run starts past the name's measured width plus the
    /// run gap, on the layout's left.
    #[test]
    fn coords_run_follows_the_name() {
        let layout = LabelLayout {
            rect: [272.0, 45.8, 114.0, 41.0],
            dir: None,
            text_left: 286.0,
            name_baseline: 63.0,
            coords_baseline: 63.0,
            arrow: None,
        };
        assert_eq!(label_text_xs(&layout, 40.0), (286.0, 332.0));
    }

    /// The label type carries its runs: the painter draws the name and the
    /// coords the page built as "(x, y)".
    #[test]
    fn label_carries_name_and_coords() {
        let label = PlotLabel {
            name: "P".to_string(),
            coords: "(1.00, 1.00)".to_string(),
        };
        assert_eq!(label.name, "P");
        assert_eq!(label.coords, "(1.00, 1.00)");
    }
}
