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
//! never a second emulation. Canvas continues a dash pattern across the
//! subpaths of one path (probed in `.local/probe/dash_subpath.py`: the second
//! drop segment dashes from the first's phase), so the two drop segments are
//! cut with a carried offset. All positions are canvas-relative px; the caller
//! translates to the content box, as the other painters do.

use mv_text::{Face, Features, Style};

use crate::grid::GridLayout;
use crate::plane_geometry::{
    DROP_DASH, DROP_WIDTH, GUIDE_DASH, GUIDE_WIDTH, LABEL_BORDER, LABEL_RUN_GAP, LabelLayout,
    MarkerInk, PlotLabel, PlotMarker, arrow_path, drop_lines, guide_x, label_layout, marker_arcs,
    marker_look,
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

/// Where the guide's pattern starts (`lineDashOffset`: half the first dash).
pub const GUIDE_OFFSET: f64 = 2.0;

/// Where the drop lines' pattern starts (half the first dash).
pub const DROP_OFFSET: f64 = 2.5;

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

/// Whose ink a marker arc takes: the plane background, the foreground, or the
/// marker's own ink (`MarkerInk`, resolved here as the source resolves
/// `theme[...]` in `drawMarker`).
pub fn arc_ink(colors: &MarkerColors, ink: MarkerInk, marker: &PlotMarker) -> [f32; 4] {
    match ink {
        MarkerInk::Background => colors.background,
        MarkerInk::Foreground => colors.foreground,
        MarkerInk::Marker => marker_ink(colors, marker),
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
    vg.stroke_width(GUIDE_WIDTH as f32);
    stroke_pieces(
        vg,
        &crate::plane::dash_pieces(&[[px, 0.0], [px, grid.height]], GUIDE_DASH, GUIDE_OFFSET),
    );
}

/// Paints a marker's drop lines to the axes, clamped at the frame
/// (`drawDropLines`): both segments in one path, vertical first, the dash
/// phase carried across them as Canvas carries it.
pub fn paint_drop_lines(
    vg: &mut mv_nvg::Context,
    grid: &GridLayout,
    marker: &PlotMarker,
    colors: &MarkerColors,
) {
    let lines = drop_lines(grid, marker.point);
    vg.line_cap(mv_nvg::LineCapType::BUTT);
    vg.stroke_color(marker_ink(colors, marker));
    vg.stroke_width(DROP_WIDTH as f32);
    // The dash distance Canvas has consumed so far along the path.
    let mut consumed = 0.0;
    for segment in [lines.vertical, lines.horizontal].into_iter().flatten() {
        stroke_pieces(
            vg,
            &crate::plane::dash_pieces(&segment, DROP_DASH, DROP_OFFSET + consumed),
        );
        let [[x0, y0], [x1, y1]] = segment;
        consumed += (x1 - x0).hypot(y1 - y0);
    }
}

/// Paints a marker's three arcs at its canvas position: the background fill,
/// the ring stroke, the core fill (`drawMarker`).
pub fn paint_marker(
    vg: &mut mv_nvg::Context,
    marker: &PlotMarker,
    at: [f64; 2],
    colors: &MarkerColors,
) {
    let arcs = marker_arcs(at, marker.shape);
    let ring_width = marker_look(marker.shape).ring as f32;
    // The background fill.
    let [x, y] = arcs[0].at;
    vg.begin_path();
    vg.circle(x as f32, y as f32, arcs[0].r as f32);
    vg.fill_color(arc_ink(colors, arcs[0].ink, marker));
    vg.fill();
    // The ring stroke.
    let [x, y] = arcs[1].at;
    vg.begin_path();
    vg.circle(x as f32, y as f32, arcs[1].r as f32);
    vg.stroke_color(arc_ink(colors, arcs[1].ink, marker));
    vg.stroke_width(ring_width);
    vg.stroke();
    // The core fill.
    let [x, y] = arcs[2].at;
    vg.begin_path();
    vg.circle(x as f32, y as f32, arcs[2].r as f32);
    vg.fill_color(arc_ink(colors, arcs[2].ink, marker));
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
/// their literal baselines (`drawLabel`, `drawArrow`). `name_width` is the
/// ceiled name measure `label_layout` laid out with.
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
    // The pill and its border, stroked inset by half the border.
    vg.begin_path();
    vg.rounded_rect(
        layout.x as f32,
        layout.y as f32,
        layout.w as f32,
        layout.h as f32,
        (layout.h / 2.0) as f32,
    );
    vg.fill_color(colors.card);
    vg.fill();
    let half = LABEL_BORDER / 2.0;
    vg.line_cap(mv_nvg::LineCapType::BUTT);
    vg.stroke_color(ink);
    vg.stroke_width(LABEL_BORDER as f32);
    vg.begin_path();
    vg.rounded_rect(
        (layout.x + half) as f32,
        (layout.y + half) as f32,
        (layout.w - LABEL_BORDER) as f32,
        (layout.h - LABEL_BORDER) as f32,
        (layout.h / 2.0 - half) as f32,
    );
    vg.stroke();
    // The off-frame arrow, filled in the ink.
    if let (Some(dir), Some([ax, ay])) = (layout.dir, layout.arrow) {
        vg.begin_path();
        let mut corners = arrow_path(dir, ax, ay).into_iter();
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

/// The marker's label this frame: `None` until the plane's fonts have loaded
/// (`drawCartesianPlane.test.ts`: no label shows until then), else the
/// marker's own label.
fn labels(marker: &PlotMarker, font_ready: bool) -> Option<&PlotLabel> {
    if font_ready { marker.label.as_ref() } else { None }
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
        paint_marker(vg, marker, at, colors);
        if let Some(label) = labels(marker, font_ready) {
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
    use crate::plane_geometry::{MarkerShape, PlotLabel};
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

    /// A bullseye's ring takes the foreground and its core the marker's ink;
    /// a ring's go the other way round (`marker_arcs`' swap, resolved here).
    #[test]
    fn arc_inks_swap_by_shape() {
        let at = [250.0, 100.0];
        let bullseye = marker_arcs(at, MarkerShape::Bullseye);
        assert_eq!(
            [bullseye[0].ink, bullseye[1].ink, bullseye[2].ink],
            [
                MarkerInk::Background,
                MarkerInk::Foreground,
                MarkerInk::Marker
            ]
        );
        let ring = marker_arcs(at, MarkerShape::Ring);
        assert_eq!(
            [ring[0].ink, ring[1].ink, ring[2].ink],
            [
                MarkerInk::Background,
                MarkerInk::Marker,
                MarkerInk::Foreground
            ]
        );
        assert_eq!([ring[0].r, ring[1].r, ring[2].r], [12.0, 10.5, 3.0]);
    }

    /// `MarkerInk` resolves against the marker's own ink for `Marker`.
    #[test]
    fn marker_ink_resolves_point_tokens() {
        let colors = MarkerColors {
            point_p: [0.0, 1.0, 0.0, 1.0],
            point_q: [0.0, 0.0, 1.0, 1.0],
            foreground: [0.0, 0.0, 0.0, 1.0],
            foreground_muted: [0.5, 0.5, 0.5, 1.0],
            background: [1.0, 1.0, 1.0, 1.0],
            card: [0.2, 0.2, 0.2, 1.0],
        };
        let point = PlotPoint { x: 1.0, y: 1.0 };
        let p = marker(point);
        assert_eq!(arc_ink(&colors, MarkerInk::Marker, &p), colors.point_p);
        let q = PlotMarker {
            ink: mv_theme::Token::PointQ,
            ..marker(point)
        };
        assert_eq!(arc_ink(&colors, MarkerInk::Marker, &q), colors.point_q);
        assert_eq!(
            arc_ink(&colors, MarkerInk::Foreground, &p),
            colors.foreground
        );
        assert_eq!(
            arc_ink(&colors, MarkerInk::Background, &p),
            colors.background
        );
    }

    /// P's label sits 22 px right of the point, its bottom 13.2 px above it:
    /// the pill at (272, ~45.8), 114 by 41, "P" at x 286 and the coords at
    /// 332 under the harness's measureText stub (every width 40).
    #[test]
    fn label_sits_right_of_the_point() {
        let grid = grid_400x300();
        let layout = label_layout(&grid, 250.0, 100.0, 40.0, 40.0);
        assert_eq!([layout.x, layout.w, layout.h], [272.0, 114.0, 41.0]);
        assert!((layout.y - 45.8).abs() < 0.05, "label y {}", layout.y);
        assert_eq!(layout.dir, None);
        let (name_x, coords_x) = label_text_xs(&layout, 40.0);
        assert_eq!([name_x, coords_x], [286.0, 332.0]);
    }

    /// The label's coords run starts past the name's measured width plus the
    /// run gap, on the layout's left.
    #[test]
    fn coords_run_follows_the_name() {
        let layout = LabelLayout {
            x: 272.0,
            y: 45.8,
            w: 114.0,
            h: 41.0,
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

    /// Q's guide runs while its x lies on the plane, whatever its y is; none
    /// shows past either edge.
    #[test]
    fn guide_runs_while_x_is_on_the_plane() {
        let grid = grid_400x300();
        assert_eq!(guide_x(&grid, PlotPoint { x: 1.0, y: 20.0 }), Some(250.0));
        assert_eq!(guide_x(&grid, PlotPoint { x: 20.0, y: 1.0 }), None);
    }

    /// A marker's drop lines run to the axes: vertical first, then
    /// horizontal; above the view the vertical comes in from the top edge.
    #[test]
    fn drop_lines_run_vertical_first() {
        let grid = grid_400x300();
        let lines = drop_lines(&grid, PlotPoint { x: 1.0, y: 1.0 });
        assert_eq!(
            [lines.vertical, lines.horizontal],
            [
                Some([[250.0, 100.0], [250.0, 150.0]]),
                Some([[250.0, 100.0], [200.0, 100.0]]),
            ]
        );
        let above = drop_lines(&grid, PlotPoint { x: 1.0, y: 20.0 });
        assert_eq!(
            [above.vertical, above.horizontal],
            [Some([[250.0, 0.0], [250.0, 150.0]]), None]
        );
    }

    /// No label shows until the plane's fonts have loaded
    /// (`drawCartesianPlane.test.ts`): the gate holds with a label on the
    /// marker, and lifts with it.
    #[test]
    fn no_label_shows_until_the_fonts_have_loaded() {
        let mut marked = marker(PlotPoint { x: 1.0, y: 1.0 });
        marked.label = Some(PlotLabel {
            name: "P".to_string(),
            coords: "(1.00, 1.00)".to_string(),
        });
        assert_eq!(labels(&marked, false), None);
        assert_eq!(labels(&marked, true), marked.label.as_ref());
        assert_eq!(labels(&marker(PlotPoint { x: 1.0, y: 1.0 }), true), None);
    }
}
