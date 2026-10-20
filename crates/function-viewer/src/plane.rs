//! The Cartesian plane's canvas: the grid, axes and labels through NanoVG.
//!
//! Ported from `src/components/views/CartesianPlane/CartesianPlane.tsx` and
//! `drawCartesianPlane.ts` at c409aad. The layouts come from [`grid`]
//! ([`layout_grid`] with [`LABEL_MARGIN`], [`layout_labels`] with the plane
//! font); this module only batches them into strokes and shaped text. Strokes
//! sit on the layout px exactly as Canvas 2D centres them, with butt caps
//! (the first port's technique); each colour and width batch is one path.
//!
//! The canvas starts cleared (transparent in the browser, showing the page
//! behind it), so the plane paints no background: [`PlaneColors::page`] is
//! what shows through, used for the rounded-corner cutouts.

use mv_sys::clay;
use mv_text::{Face, Features, Style};

use crate::grid::{GridLabel, GridLayout, layout_grid, layout_labels};
use crate::plane_geometry::{HandlePath, handle_path, line_path, to_px};
use crate::scale_legend::LegendColors;

/// The font size of every text on the plane (`constants.ts`).
pub const PLANE_FONT_SIZE: f64 = 14.0;

/// Ticks cross an axis at each major line, this far to each side.
pub const TICK_HALF: f64 = 5.0;

/// Lines this far past an edge still get their labels, so a label slides in
/// with its line instead of popping in (`drawCartesianPlane.ts`).
pub const LABEL_MARGIN: f64 = 64.0;

/// The canvas's CSS border, `1px solid var(--border)`.
pub const CANVAS_BORDER: f32 = 1.0;

/// The canvas's CSS radius, `var(--radius-card)`.
pub const CANVAS_RADIUS: f32 = 4.0;

/// The colours the plane paints with (resolved by h1-frame from the theme):
/// the grid (`chartGrid`, which carries `--border` in every theme, so it rims
/// the canvas and the legend's plate the way the reference's `var(--border)`
/// does), the major grid (`chartGridMajor`), the axes (`chartAxis`), the
/// labels (`foregroundMuted`) and the page behind the cleared canvas.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlaneColors {
    pub grid: [f32; 4],
    pub grid_major: [f32; 4],
    pub axis: [f32; 4],
    pub label: [f32; 4],
    pub page: [f32; 4],
    /// The scale legend's colours, drawn by the viewer after the bare plane
    /// (its `drawPlaneWithScale`), never by `paint_plane` itself.
    pub legend: LegendColors,
}

/// The canvas element's Clay id, for the slot's lookup and tests.
pub const PLANE_ID: &str = "Plane";

/// The canvas's content box: the border box inside its 1 px CSS border (the
/// page sets `box-sizing: border-box`, and the backing store is the content
/// box times the density).
pub fn canvas_content(border: &clay::BoundingBox) -> clay::BoundingBox {
    clay::BoundingBox {
        x: border.x + CANVAS_BORDER,
        y: border.y + CANVAS_BORDER,
        width: border.width - 2.0 * CANVAS_BORDER,
        height: border.height - 2.0 * CANVAS_BORDER,
    }
}

/// The labels' text style: the plane font (`constants.ts`).
pub fn label_style() -> Style {
    Style {
        face: Face::RobotoMono500,
        size: PLANE_FONT_SIZE as f32,
        tracking: 0.0,
        features: Features::new(),
    }
}

/// The camera the plane draws from: the page query's `center` and `scale`
/// through `camera.rs`'s parses, defaulting where a key is missing or
/// unparseable, as `queryParamAtom` falls back to its default. h2-input's
/// merge switches the slot to its live camera.
pub fn plane_camera(url: &crate::url::Url) -> ([f64; 2], f64) {
    let center = url
        .raw("center")
        .as_deref()
        .and_then(crate::camera::parse_center)
        .unwrap_or(crate::camera::DEFAULT_CENTER);
    let scale = url
        .raw("scale")
        .as_deref()
        .and_then(crate::camera::parse_scale)
        .unwrap_or(crate::camera::DEFAULT_SCALE);
    (center, scale)
}

/// The plane's colours from the resolved theme.
pub fn plane_colors(theme: &mv_theme::Theme) -> PlaneColors {
    PlaneColors {
        grid: theme.colors[mv_theme::Token::ChartGrid],
        grid_major: theme.colors[mv_theme::Token::ChartGridMajor],
        axis: theme.colors[mv_theme::Token::ChartAxis],
        label: theme.colors[mv_theme::Token::ForegroundMuted],
        page: theme.colors[mv_theme::Token::Background],
        legend: crate::scale_legend::LegendColors {
            rim: theme.colors[mv_theme::Token::ChartGrid],
            fill: theme.colors[mv_theme::Token::Background],
            bar: theme.colors[mv_theme::Token::Foreground],
            text: theme.colors[mv_theme::Token::ForegroundMuted],
        },
    }
}

/// Traces vertical lines over the canvas's full height and horizontal lines
/// over its full width into one path, in canvas-relative px (the caller
/// translates to the content box).
fn trace_lines(
    vg: &mut mv_nvg::Context,
    width: f32,
    height: f32,
    vertical: &[crate::grid::GridLine],
    horizontal: &[crate::grid::GridLine],
) {
    for line in vertical {
        vg.move_to(line.px as f32, 0.0);
        vg.line_to(line.px as f32, height);
    }
    for line in horizontal {
        vg.move_to(0.0, line.px as f32);
        vg.line_to(width, line.px as f32);
    }
}

/// Paints the minor and major grid lines, 1 px each, in canvas-relative px.
pub fn paint_grid(
    vg: &mut mv_nvg::Context,
    width: f32,
    height: f32,
    grid: &GridLayout,
    colors: &PlaneColors,
) {
    vg.begin_path();
    trace_lines(vg, width, height, &grid.minor_x, &grid.minor_y);
    vg.stroke_color(colors.grid);
    vg.stroke_width(1.0);
    vg.stroke();

    vg.begin_path();
    trace_lines(vg, width, height, &grid.major_x, &grid.major_y);
    vg.stroke_color(colors.grid_major);
    vg.stroke_width(1.0);
    vg.stroke();
}

/// Paints the axes, 2 px, with a tick at every major line, in
/// canvas-relative px.
pub fn paint_axes(
    vg: &mut mv_nvg::Context,
    width: f32,
    height: f32,
    grid: &GridLayout,
    colors: &PlaneColors,
) {
    vg.begin_path();
    if let Some(x) = grid.x_origin {
        let x = x as f32;
        vg.move_to(x, 0.0);
        vg.line_to(x, height);
        for line in &grid.major_y {
            let y = line.px as f32;
            vg.move_to(x - TICK_HALF as f32, y);
            vg.line_to(x + TICK_HALF as f32, y);
        }
    }
    if let Some(y) = grid.y_origin {
        let y = y as f32;
        vg.move_to(0.0, y);
        vg.line_to(width, y);
        for line in &grid.major_x {
            let x = line.px as f32;
            vg.move_to(x, y - TICK_HALF as f32);
            vg.line_to(x, y + TICK_HALF as f32);
        }
    }
    vg.stroke_color(colors.axis);
    vg.stroke_width(2.0);
    vg.stroke();
}

/// The plotted curves' look: solid, 3 px (`drawCurve` at `3560eb2`).
pub const CURVE_WIDTH: f32 = 3.0;

/// The dashed original's look (`dashedLook` at `5cef052`): 2 px, `[6, 6]`
/// dashes from half a dash in (`lineDashOffset` is half the first dash),
/// 0.75 alpha, round caps and joins.
pub const DASHED_CURVE_WIDTH: f32 = 2.0;
/// The dashed original's pattern (`dash: [6, 6]`).
pub const DASHED_CURVE_DASH: [f64; 2] = [6.0, 6.0];
/// Where the pattern starts (`lineDashOffset`: half the first dash).
pub const DASHED_CURVE_OFFSET: f64 = 3.0;
/// The dashed original's alpha (`alpha: 0.75`, times the curve's own).
pub const DASHED_CURVE_ALPHA: f32 = 0.75;

/// The smallest dash piece that survives, in px (the first port's `dash.rs`
/// floor: a shorter piece never strokes).
const MIN_DASH_PIECE_PX: f64 = 0.01;

/// The dash on-intervals along a length: on/off pairs from an offset (the
/// first port's `dash.rs` intervals: the pattern runs from `-offset`, and a
/// piece shorter than [`MIN_DASH_PIECE_PX`] is dropped).
fn dash_intervals(length: f64, pattern: [f64; 2], offset: f64) -> Vec<[f64; 2]> {
    let period = pattern[0] + pattern[1];
    let mut out: Vec<[f64; 2]> = Vec::new();
    if period <= 0.0 || length <= 0.0 {
        return out;
    }
    let mut o = offset;
    while o > period {
        o -= period;
    }
    while o < 0.0 {
        o += period;
    }
    let mut pos = -o;
    while pos < length {
        let from = pos.max(0.0);
        let to = (pos + pattern[0]).min(length);
        if to - from >= MIN_DASH_PIECE_PX {
            out.push([from, to]);
        }
        pos += period;
    }
    out
}

/// Cut a polyline by a dash pattern from an offset (Canvas 2D's
/// `setLineDash` with `lineDashOffset`, as the first port's `dash.rs` cuts
/// it): the on-intervals as subpaths, since Canvas caps each dash with the
/// look's caps. `pub(crate)` for the line painters (h6-handles), whose
/// dashed and measured looks cut their shafts the same way.
pub(crate) fn dash_pieces(
    points: &[[f64; 2]],
    pattern: [f64; 2],
    offset: f64,
) -> Vec<Vec<[f64; 2]>> {
    if points.len() < 2 {
        return Vec::new();
    }
    // The distance along the polyline at each vertex.
    let mut along = vec![0.0; points.len()];
    for (i, pair) in points.windows(2).enumerate() {
        let dx = pair[1][0] - pair[0][0];
        let dy = pair[1][1] - pair[0][1];
        along[i + 1] = along[i] + dx.hypot(dy);
    }
    let length = along[points.len() - 1];
    let at = |seg: usize, d: f64| -> [f64; 2] {
        let span = along[seg + 1] - along[seg];
        let t = if span == 0.0 {
            0.0
        } else {
            (d - along[seg]) / span
        };
        let a = points[seg];
        let b = points[seg + 1];
        [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
    };
    let mut pieces: Vec<Vec<[f64; 2]>> = Vec::new();
    // The segment the walk is in; intervals come in order.
    let mut seg = 0;
    for interval in dash_intervals(length, pattern, offset) {
        while seg + 2 < points.len() && along[seg + 1] <= interval[0] {
            seg += 1;
        }
        let mut piece = vec![at(seg, interval[0])];
        let mut end_seg = seg;
        while end_seg + 2 < points.len() && along[end_seg + 1] < interval[1] {
            end_seg += 1;
            piece.push(points[end_seg]);
        }
        piece.push(at(end_seg, interval[1]));
        pieces.push(piece);
        seg = end_seg;
    }
    pieces
}

/// Paints the frame's curves over the canvas: each curve's sampled polyline
/// (`plot::sample_curve` through [`TransformedFunction::value`], so the
/// switch's draw-on and fade compose with the transform as the source
/// composes them), the dashed original behind its transformed curve in the
/// muted ink, each stroked in order so each draws over the ones before it.
/// Canvas 2D centres strokes on the layout px as NanoVG does with butt
/// caps; the dashed pieces carry the look's round caps, as Canvas caps each
/// dash.
///
/// [`TransformedFunction::value`]: crate::transform_curves::TransformedFunction::value
// Eight inputs: the pass predicate shares the dashed and solid passes'
// one signature so the dispatch reads as one call twice.
#[allow(clippy::too_many_arguments)]
pub fn paint_curves(
    vg: &mut mv_nvg::Context,
    width: f32,
    height: f32,
    grid: &GridLayout,
    curves: &[crate::transform_curves::TransformedCurve],
    line: [f32; 4],
    muted: [f32; 4],
    dashed_only: bool,
) {
    for curve in curves {
        // One pass paints the dashed originals, the next the solid curves;
        // the lines paint between them (the `drawCartesianPlane` order).
        if curve.dashed != dashed_only {
            continue;
        }
        let points = crate::plot::sample_curve(
            |x| curve.func.value(x),
            grid,
            f64::from(width),
            f64::from(height),
            curve.draw_to,
        );
        // NanoVG drops a path with no move-to (its points never join one),
        // where Canvas 2D reads the first line-to as one: the explicit move
        // strokes the same pixels on both.
        vg.begin_path();
        if curve.dashed {
            for piece in dash_pieces(&points, DASHED_CURVE_DASH, DASHED_CURVE_OFFSET) {
                let mut piece = piece.iter();
                if let Some([x, y]) = piece.next() {
                    vg.move_to(*x as f32, *y as f32);
                }
                for [x, y] in piece {
                    vg.line_to(*x as f32, *y as f32);
                }
            }
            vg.global_alpha(curve.alpha as f32 * DASHED_CURVE_ALPHA);
            vg.stroke_color(muted);
            vg.stroke_width(DASHED_CURVE_WIDTH);
            vg.line_cap(mv_nvg::LineCapType::ROUND);
        } else {
            let mut points = points.iter();
            if let Some([x, y]) = points.next() {
                vg.move_to(*x as f32, *y as f32);
            }
            for [x, y] in points {
                vg.line_to(*x as f32, *y as f32);
            }
            vg.global_alpha(curve.alpha as f32);
            vg.stroke_color(line);
            vg.stroke_width(CURVE_WIDTH);
            vg.line_cap(mv_nvg::LineCapType::BUTT);
        }
        vg.line_join(mv_nvg::LineCapType::ROUND);
        vg.stroke();
        vg.global_alpha(1.0);
    }
}

/// Paints laid-out labels: right-aligned at `right`, Canvas `textBaseline:
/// "top"` (the em box's top on `top`, so the baseline sits one ascent below).
pub fn paint_labels(
    fonts: &mut mv_text::Fonts,
    vg: &mut mv_nvg::Context,
    labels: &[GridLabel],
    ink: [f32; 4],
) {
    let style = label_style();
    let ascent = mv_text::metrics(fonts, style.face).ascent * style.size;
    for label in labels {
        let advance = mv_text::measure(fonts, style, &label.text);
        mv_text::draw(
            fonts,
            vg,
            style,
            label.right as f32 - advance,
            label.top as f32 + ascent,
            &label.text,
            ink,
        );
    }
}

/// Fills the canvas's corner triangles with the page colour: the canvas
/// bitmap paints square corners and the CSS radius rounds them after, so the
/// port cuts the same triangles out after painting its ink. Canvas-relative.
/// h10-page's hero shares it (its frame rounds the same way).
pub(crate) fn paint_corner_cutouts(
    vg: &mut mv_nvg::Context,
    width: f32,
    height: f32,
    page: [f32; 4],
) {
    /// The bezier control offset for a quarter circle of radius 1.
    const KAPPA: f32 = 0.552_284_8;
    let r = CANVAS_RADIUS;
    // Each corner: its sharp point and its arc centre.
    let corners = [
        ([0.0, 0.0], [r, r]),
        ([width, 0.0], [width - r, r]),
        ([0.0, height], [r, height - r]),
        ([width, height], [width - r, height - r]),
    ];
    vg.fill_color(page);
    for (c, a) in corners {
        // The arc's endpoints on the two edges.
        let e0 = [c[0], a[1]];
        let e1 = [a[0], c[1]];
        // The unit tangents pointing along the chord e1 -> e0: each radius's
        // perpendicular, flipped where it runs against the chord.
        let chord = [e0[0] - e1[0], e0[1] - e1[1]];
        let radius1 = [e1[0] - a[0], e1[1] - a[1]];
        let mut t = [-radius1[1] / r, radius1[0] / r];
        if t[0] * chord[0] + t[1] * chord[1] < 0.0 {
            t = [-t[0], -t[1]];
        }
        let radius0 = [e0[0] - a[0], e0[1] - a[1]];
        let mut u = [-radius0[1] / r, radius0[0] / r];
        if u[0] * chord[0] + u[1] * chord[1] < 0.0 {
            u = [-u[0], -u[1]];
        }
        let k = KAPPA * r;
        vg.begin_path();
        vg.move_to(e0[0], e0[1]);
        vg.line_to(c[0], c[1]);
        vg.line_to(e1[0], e1[1]);
        vg.bezier_to(
            e1[0] + k * t[0],
            e1[1] + k * t[1],
            e0[0] - k * u[0],
            e0[1] - k * u[1],
            e0[0],
            e0[1],
        );
        vg.close_path();
        vg.fill();
    }
}

/// The texts the plane shows once its font has loaded: the axis labels and
/// the legend's label. Before the font loads the plane draws its grid and
/// axes but no numbers and no legend (`CartesianPlane.test.tsx`); natively
/// fonts are always loaded, so the port always shows them.
pub fn plane_texts(
    center: [f64; 2],
    scale: f64,
    width: f64,
    height: f64,
    label_width: &dyn Fn(&str) -> f64,
    font_ready: bool,
) -> Vec<String> {
    if !font_ready {
        return Vec::new();
    }
    let grid = layout_grid(center, scale, width, height, LABEL_MARGIN);
    let mut texts: Vec<String> = layout_labels(&grid, PLANE_FONT_SIZE, label_width)
        .into_iter()
        .map(|label| label.text)
        .collect();
    texts.push(crate::scale_legend::legend_text(scale));
    texts
}

/// Paints the bare plane over the canvas's content box: the grid, the axes,
/// the curves, the handles, the point markers and, once the font has loaded,
/// the labels (the `drawCartesianPlane` order: the dashed originals, the lit
/// parameter's lines, the solid curves, the handles, the guides, the drop
/// lines, the markers with their labels). The scale legend is not painted
/// here: the function viewer draws it after the bare plane (its
/// `drawPlaneWithScale`), and the welcome page's hero draws the bare plane
/// alone. h6-curve paints its dashed originals behind each transformed curve;
/// the dispatch below splits its pass around the lines.
// Fifteen paint inputs (the plane's six, the frame's curves with their
// three inks, the handles and lines with theirs, the markers with their
// colours): a bundle struct would hide the draw order this signature
// documents.
#[allow(clippy::too_many_arguments)]
pub fn paint_plane(
    fonts: &mut mv_text::Fonts,
    vg: &mut mv_nvg::Context,
    content: &clay::BoundingBox,
    center: [f64; 2],
    scale: f64,
    colors: &PlaneColors,
    font_ready: bool,
    curves: &[crate::transform_curves::TransformedCurve],
    line: [f32; 4],
    muted: [f32; 4],
    handles: &[crate::plot::PlotHandle],
    lines: &[crate::plot::PlotLine],
    primary: [f32; 4],
    markers: &[crate::plane_geometry::PlotMarker],
    mcolors: &crate::plane_markers::MarkerColors,
) {
    let width = f64::from(content.width);
    let height = f64::from(content.height);
    let grid = layout_grid(center, scale, width, height, LABEL_MARGIN);
    vg.save();
    vg.scissor(content.x, content.y, content.width, content.height);
    // The layouts are canvas-relative; the frame draws in window px.
    vg.translate(content.x, content.y);
    vg.line_cap(mv_nvg::LineCapType::BUTT);
    paint_grid(vg, content.width, content.height, &grid, colors);
    paint_axes(vg, content.width, content.height, &grid, colors);
    // The labels wait for their font; the curves draw either way, between
    // them in draw order (as `drawCartesianPlane` draws the curves before
    // the font-gated text). The legend is the viewer's to draw after this
    // returns, so only the measured labels are kept here.
    let labels = if font_ready {
        let style = label_style();
        // `layout_labels` takes a shared measure while shaping borrows the
        // fonts mutably: the cell lends them out call by call, which is sound
        // because the layout never re-enters the measure. The cell holds a
        // reborrow, so the caller's `fonts` stays usable below.
        let cell = std::cell::RefCell::new(&mut *fonts);
        let measure = |text: &str| {
            let mut guard = cell.borrow_mut();
            let fonts: &mut mv_text::Fonts = &mut guard;
            f64::from(mv_text::measure(fonts, style, text))
        };
        let labels = layout_labels(&grid, PLANE_FONT_SIZE, &measure);
        let fonts = cell.into_inner();
        paint_labels(fonts, vg, &labels, colors.label);
        Some(fonts)
    } else {
        None
    };
    // The draw order (`drawCartesianPlane.ts`): the dashed originals, the
    // lit parameter's lines, the solid curves, the handles, the guides, the
    // drop lines, the markers with their labels, the legend. The two curve
    // passes filter on the `dashed` flag; each pass keeps the frame's order.
    paint_curves(
        vg,
        content.width,
        content.height,
        &grid,
        curves,
        line,
        muted,
        true,
    );
    paint_lines(vg, &grid, lines, primary);
    paint_curves(
        vg,
        content.width,
        content.height,
        &grid,
        curves,
        line,
        muted,
        false,
    );
    paint_handles(vg, &grid, handles, primary, colors.page);
    // The markers paint over the handles; their labels wait for the font
    // with the axis labels (the `labels` option carries the measured fonts
    // exactly when `font_ready` lent them).
    match labels {
        Some(fonts) => {
            crate::plane_markers::paint_markers(fonts, vg, &grid, markers, mcolors, true);
        }
        None => {
            crate::plane_markers::paint_markers(fonts, vg, &grid, markers, mcolors, false);
        }
    }
    vg.reset_scissor();
    paint_corner_cutouts(vg, content.width, content.height, colors.page);
    // The CSS border stays Clay's: the canvas element's Border command strokes
    // the border box inset half a px, the border's exact pixels.
    vg.restore();
}

/// A lit scale's dashed box side: 1.5 px, `[5, 4]`, 0.8 alpha
/// (`lineLooks.dashed`).
pub const DASHED_WIDTH: f32 = 1.5;

/// The dashed look's dash pattern, in px (`lineLooks.dashed.dash`).
pub const DASHED_DASH: [f64; 2] = [5.0, 4.0];

/// The dashed look's alpha (`lineLooks.dashed.alpha`).
pub const DASHED_ALPHA: f32 = 0.8;

/// A measured box side: 3 px, solid, full alpha (`lineLooks.measured`).
pub const MEASURED_WIDTH: f32 = 3.0;

/// Every line's ticks: 2 px (`tickWidth`).
pub const LINE_TICK_WIDTH: f32 = 2.0;

/// The shaft's look: width, dash pattern and alpha (`lineLooks`).
fn line_look(kind: crate::plot::LineKind) -> (f32, Option<[f64; 2]>, f32) {
    match kind {
        crate::plot::LineKind::Arrow => (crate::plane_geometry::ARROW_WIDTH as f32, None, 1.0),
        crate::plot::LineKind::Dashed => (DASHED_WIDTH, Some(DASHED_DASH), DASHED_ALPHA),
        crate::plot::LineKind::Measured => (MEASURED_WIDTH, None, 1.0),
    }
}

/// Paints the shift and scale handles: the diamond filled with the page
/// colour and stroked in primary, the rounded square filled with the page
/// colour with its inner rounded square stroked in primary (`drawHandle`,
/// `handleStroke` 2.5, round joins). Handles and lines paint in
/// `theme.primary` — not the parameter colours: the source sets
/// `strokeStyle`/`fillStyle` to primary for every handle and line (probed).
/// `background` is the page colour showing through the cleared canvas.
pub fn paint_handles(
    vg: &mut mv_nvg::Context,
    grid: &GridLayout,
    handles: &[crate::plot::PlotHandle],
    primary: [f32; 4],
    background: [f32; 4],
) {
    vg.line_cap(mv_nvg::LineCapType::BUTT);
    vg.line_join(mv_nvg::LineCapType::ROUND);
    vg.stroke_width(crate::plane_geometry::HANDLE_STROKE as f32);
    vg.stroke_color(primary);
    vg.fill_color(background);
    for handle in handles {
        let at = to_px(grid, handle.at);
        match handle_path(handle.kind, at) {
            HandlePath::Diamond { points } => {
                vg.begin_path();
                vg.move_to(points[0][0] as f32, points[0][1] as f32);
                for point in &points[1..] {
                    vg.line_to(point[0] as f32, point[1] as f32);
                }
                vg.close_path();
                vg.fill();
                vg.stroke();
            }
            HandlePath::Square {
                outer,
                outer_radius,
                inner,
                inner_radius,
            } => {
                vg.begin_path();
                vg.rounded_rect(
                    outer[0] as f32,
                    outer[1] as f32,
                    outer[2] as f32,
                    outer[3] as f32,
                    outer_radius as f32,
                );
                vg.fill();
                vg.begin_path();
                vg.rounded_rect(
                    inner[0] as f32,
                    inner[1] as f32,
                    inner[2] as f32,
                    inner[3] as f32,
                    inner_radius as f32,
                );
                vg.stroke();
            }
        }
    }
    // The source restores the context after each handle; the port leaves it
    // as the source leaves it (butt caps, miter joins) so the legend's bar
    // strokes as probed.
    vg.line_join(mv_nvg::LineCapType::MITER);
}

/// Paints one lit parameter's lines: the shaft (cut past the arrowhead, in
/// the look's width, dash and alpha), a tick across each end, and the filled
/// arrowhead (`drawLine`). Zero-length lines draw nothing (the source's
/// early return through `line_path`'s `None`). `label` stays undrawn: the
/// source carries and draws none at this hash.
pub fn paint_lines(
    vg: &mut mv_nvg::Context,
    grid: &GridLayout,
    lines: &[crate::plot::PlotLine],
    primary: [f32; 4],
) {
    vg.line_cap(mv_nvg::LineCapType::BUTT);
    vg.line_join(mv_nvg::LineCapType::MITER);
    vg.stroke_color(primary);
    vg.fill_color(primary);
    for line in lines {
        let from = to_px(grid, line.from);
        let to = to_px(grid, line.to);
        let Some(path) = line_path(line.kind, from, to) else {
            continue;
        };
        let (width, dash, alpha) = line_look(line.kind);
        vg.global_alpha(alpha);
        // The shaft, in one path (each dash piece its own subpath).
        vg.begin_path();
        match dash {
            None => {
                vg.move_to(path.shaft[0][0] as f32, path.shaft[0][1] as f32);
                vg.line_to(path.shaft[1][0] as f32, path.shaft[1][1] as f32);
            }
            Some(pattern) => {
                // `lineDashOffset` is half the dash on (`dash[0] / 2`).
                for piece in dash_pieces(&path.shaft, pattern, pattern[0] / 2.0) {
                    vg.move_to(piece[0][0] as f32, piece[0][1] as f32);
                    for point in &piece[1..] {
                        vg.line_to(point[0] as f32, point[1] as f32);
                    }
                }
            }
        }
        vg.stroke_width(width);
        vg.stroke();
        // A tick across each end, in `tickWidth` at the shaft's alpha.
        vg.begin_path();
        vg.move_to(path.start_tick[0][0] as f32, path.start_tick[0][1] as f32);
        vg.line_to(path.start_tick[1][0] as f32, path.start_tick[1][1] as f32);
        vg.move_to(path.end_tick[0][0] as f32, path.end_tick[0][1] as f32);
        vg.line_to(path.end_tick[1][0] as f32, path.end_tick[1][1] as f32);
        vg.stroke_width(LINE_TICK_WIDTH);
        vg.stroke();
        if let Some(head) = path.head {
            vg.begin_path();
            vg.move_to(head[0][0] as f32, head[0][1] as f32);
            vg.line_to(head[1][0] as f32, head[1][1] as f32);
            vg.line_to(head[2][0] as f32, head[2][1] as f32);
            vg.close_path();
            vg.fill();
        }
        vg.global_alpha(1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `[6, 6]` dash at offset 3 over 30 px: `[0, 3]`, `[9, 15]`,
    /// `[21, 27]` (h6-curve's review: the helper carries a test).
    #[test]
    fn dash_intervals_cut_a_6_6_dash_from_offset_3() {
        assert_eq!(
            dash_intervals(30.0, [6.0, 6.0], 3.0),
            vec![[0.0, 3.0], [9.0, 15.0], [21.0, 27.0]]
        );
    }

    /// A degenerate pattern or length cuts nothing.
    #[test]
    fn dash_intervals_cut_nothing_degenerate() {
        assert!(dash_intervals(30.0, [0.0, 0.0], 3.0).is_empty());
        assert!(dash_intervals(0.0, [6.0, 6.0], 3.0).is_empty());
        assert!(dash_pieces(&[[0.0, 0.0]], [6.0, 6.0], 3.0).is_empty());
    }

    /// The mono advance the grid tests pin: a char is 8.4 px at 14 px.
    fn probe_width(text: &str) -> f64 {
        text.len() as f64 * 8.4
    }

    /// The port's boxes against `testdata/page.json` (the reference at this
    /// hash, `tools/run dom --fixture`): the canvas and the overlay button
    /// within the plan's 0.5 CSS px, and the content and button boxes derived
    /// from the canvas exactly. At `5cef052` the button only renders off the
    /// default camera (the tooltip commit), so its box reads from the
    /// `overlay-tip` shot (which h6-handles' fixture state provides;
    /// `overlay-visible` carries the same box).
    #[test]
    fn boxes_match_fixture() {
        const SLACK: f64 = 0.5;
        let fixture: serde_json::Value =
            serde_json::from_str(crate::fixture::PAGE_FIXTURE).expect("page fixture parses");
        let boxes = &fixture["shots"]["default"]["webkit"]["boxes"];
        let canvas = boxes["canvas"].as_array().expect("canvas box");
        let canvas = clay::BoundingBox {
            x: canvas[0].as_f64().expect("x") as f32,
            y: canvas[1].as_f64().expect("y") as f32,
            width: canvas[2].as_f64().expect("w") as f32,
            height: canvas[3].as_f64().expect("h") as f32,
        };
        let content = canvas_content(&canvas);
        for (got, want) in [
            (f64::from(content.width), 990.0),
            (f64::from(content.height), 786.0),
        ] {
            assert!((got - want).abs() <= SLACK, "content {got} against {want}");
        }
        let button = crate::overlay::overlay_button_box(&canvas);
        let want = fixture["shots"]["overlay-tip"]["webkit"]["boxes"]["overlay_button"]
            .as_array()
            .expect("button box");
        for (i, (got, want)) in [
            f64::from(button.x),
            f64::from(button.y),
            f64::from(button.width),
            f64::from(button.height),
        ]
        .into_iter()
        .zip([
            want[0].as_f64().expect("x"),
            want[1].as_f64().expect("y"),
            want[2].as_f64().expect("w"),
            want[3].as_f64().expect("h"),
        ])
        .enumerate()
        {
            assert!(
                (got - want).abs() <= SLACK,
                "button box [{i}]: {got} against {want}"
            );
        }
    }

    /// The default grid against the reference's draw record (WebKit,
    /// `.local/probe/default.json` on the 990 x 786 content box): the steps,
    /// the origins and the first lines of each batch.
    #[test]
    fn default_grid_matches_probe() {
        let grid = layout_grid([0.0, 0.0], 0.5, 990.0, 786.0, LABEL_MARGIN);
        assert_eq!(grid.px_per_unit, 50.0);
        assert_eq!(grid.steps.minor, 0.5);
        assert_eq!(grid.steps.major, 1.0);
        assert_eq!(grid.x_origin, Some(495.0));
        assert_eq!(grid.y_origin, Some(393.0));
        assert_eq!(grid.minor_x[0].px, -30.0);
        assert_eq!(grid.minor_x[1].px, 20.0);
        assert_eq!(grid.major_x[0].px, -55.0);
        assert_eq!(grid.major_x[1].px, -5.0);
        assert_eq!(grid.minor_x.len(), 22);
        assert_eq!(grid.major_y.len(), 18);
    }

    /// The content box is the border box inside its 1 px border: the
    /// reference's canvas (432, 96, 992, 788) backs 990 x 786 px.
    #[test]
    fn content_insets_border() {
        let content = canvas_content(&clay::BoundingBox {
            x: 432.0,
            y: 96.0,
            width: 992.0,
            height: 788.0,
        });
        assert_eq!(
            [content.x, content.y, content.width, content.height],
            [433.0, 97.0, 990.0, 786.0]
        );
    }

    /// The font gate, ported from `CartesianPlane.test.tsx` against the
    /// state model: before the font loads no numbers and no legend show;
    /// after it the labels and the legend show, with `1`, `-1` and
    /// `0.50 u` among them.
    #[test]
    fn font_gate_holds_numbers_until_ready() {
        let dark: Vec<String> = plane_texts([0.0, 0.0], 0.5, 990.0, 786.0, &probe_width, false);
        assert!(dark.is_empty(), "no text before the font loads");
        let lit = plane_texts([0.0, 0.0], 0.5, 990.0, 786.0, &probe_width, true);
        assert!(lit.len() > 3, "labels and the legend after it loads");
        for text in ["1", "-1", "0", "0.50 u"] {
            assert!(lit.contains(&text.to_string()), "shows {text}");
        }
    }

    /// Off the default camera the labels slide with their lines: at
    /// center 100, 0 the y-axis is gone and the y labels run along the left
    /// edge, as the probe's `off-left.json` shows.
    #[test]
    fn off_left_labels_run_along_the_edge() {
        let grid = layout_grid([100.0, 0.0], 0.5, 990.0, 786.0, LABEL_MARGIN);
        assert_eq!(grid.x_origin, None);
        assert_eq!(grid.y_origin, Some(393.0));
        let labels = layout_labels(&grid, PLANE_FONT_SIZE, &probe_width);
        assert!(
            labels
                .iter()
                .any(|label| label.text == "100" || label.text == "89")
        );
        let right_of = |text: &str| {
            labels
                .iter()
                .find(|label| label.axis == crate::grid::GridAxis::Y && label.text == text)
                .unwrap_or_else(|| panic!("a {text} y label"))
                .right
        };
        // One- and two-char labels each end 8 px inside the edge: `right` is
        // the clamped edge plus the label's own width.
        assert!((right_of("9") - 16.4).abs() < 0.01, "one-char label");
        assert!((right_of("-1") - 24.8).abs() < 0.01, "two-char label");
    }

    /// Both axes near the bottom-left corner (center 7.2, 6.6, the user's
    /// view): the origin sits on the canvas at (135, 723), the x labels run
    /// below their axis and the y labels left of theirs, as the probe's
    /// `corner.json` shows.
    #[test]
    fn corner_labels_swap_sides() {
        let grid = layout_grid([7.2, 6.6], 0.5, 990.0, 786.0, LABEL_MARGIN);
        assert_eq!(grid.origin, [135.0, 723.0]);
        let labels = layout_labels(&grid, PLANE_FONT_SIZE, &probe_width);
        let top_of = |text: &str| {
            labels
                .iter()
                .find(|label| label.text == text)
                .unwrap_or_else(|| panic!("a {text} label"))
                .top
        };
        assert!(
            (top_of("-1") - 729.0).abs() < 0.01,
            "x labels below the axis"
        );
        let right_of_y = |text: &str| {
            labels
                .iter()
                .find(|label| label.axis == crate::grid::GridAxis::Y && label.text == text)
                .unwrap_or_else(|| panic!("a {text} y label"))
                .right
        };
        assert!(
            (right_of_y("14") - 129.0).abs() < 0.01,
            "y labels left of the axis"
        );
    }

    /// At scale 100 the major step thins to every other line (probe
    /// `scale-100.json` labels -2000, -1600, ...): the widest on-canvas
    /// label no longer fits the 50 px spacing.
    #[test]
    fn scale_100_thins_to_every_other_major() {
        let grid = layout_grid([0.0, 0.0], 100.0, 990.0, 786.0, LABEL_MARGIN);
        assert_eq!(grid.steps.major, 200.0);
        let lines = crate::grid::x_label_lines(&grid, &probe_width);
        assert!(!lines.is_empty(), "some lines keep labels");
        assert!(
            lines.iter().all(|line| (line.unit / 400.0).fract() == 0.0),
            "labels sit on multiples of twice the major step"
        );
        assert_eq!(crate::scale_legend::legend_text(100.0), "100.00 u");
    }

    /// The canvas slot fills the body cell: the laid-out `Plane` box matches
    /// the reference's canvas box within the plan's 0.5 CSS px.
    #[test]
    fn canvas_slot_matches_fixture() {
        const SLACK: f32 = 0.5;
        let _guard = mv_clay::tests_lock();
        let mut app = crate::fixture::headless_app();
        assert!(mv_text::init(&mut app.fonts));
        crate::ui::init_clay(&mut app);
        app.size = [1440.0, 900.0];
        app.pixels = [2880, 1800];
        app.url = crate::url::Url::new("#function-viewer");
        let _commands = crate::page::layout_page(&mut app);
        let clay = app.clay.as_ref().expect("clay context initialised");
        let data = clay.get_element_data(mv_clay::id(crate::plane::PLANE_ID, 0));
        assert!(data.found, "Plane element laid out");
        let fixture = crate::fixture::page_fixture().expect("page fixture parses");
        let shot =
            crate::fixture::fixture_shot(&fixture, "default").expect("default shot in fixture");
        let want = crate::fixture::fixture_box(&shot.webkit, "canvas").expect("canvas in fixture");
        let b = data.boundingBox;
        for (i, (got, want)) in [b.x, b.y, b.width, b.height]
            .iter()
            .zip(want.iter())
            .enumerate()
        {
            assert!(
                (got - want).abs() <= SLACK,
                "canvas[{i}]: laid out {got}, reference {want}"
            );
        }
        crate::ui::release_clay(&mut app);
        mv_text::destroy(&mut app.fonts);
    }
}
