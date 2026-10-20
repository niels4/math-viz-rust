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
use crate::scale_legend::{LegendColors, legend_geometry, paint_legend};

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
    /// The scale legend's colours, painted with the plane.
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

/// Paints the frame's curves over the canvas: each curve's sampled polyline
/// (`plot::sample_curve`: the `drawCurve` sampling, one point per px column
/// with the partial segment of a draw-on), stroked in `line` (`chartLine`)
/// at the curve's alpha with round joins, in order so each draws over the
/// ones before it. Canvas 2D defaults to butt caps and the grid paint sets
/// `BUTT`, which persists here as there.
pub fn paint_curves(
    vg: &mut mv_nvg::Context,
    width: f32,
    height: f32,
    grid: &GridLayout,
    curves: &[crate::plot::PlotCurve],
    line: [f32; 4],
) {
    for curve in curves {
        let points = crate::plot::sample_curve(
            curve.func,
            grid,
            f64::from(width),
            f64::from(height),
            curve.draw_to,
        );
        // NanoVG drops a path with no move-to (its points never join one),
        // where Canvas 2D reads the first line-to as one: the explicit move
        // strokes the same pixels on both.
        vg.begin_path();
        for (i, [x, y]) in points.iter().enumerate() {
            if i == 0 {
                vg.move_to(*x as f32, *y as f32);
            } else {
                vg.line_to(*x as f32, *y as f32);
            }
        }
        vg.global_alpha(curve.alpha as f32);
        vg.stroke_color(line);
        vg.stroke_width(CURVE_WIDTH);
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
fn paint_corner_cutouts(vg: &mut mv_nvg::Context, width: f32, height: f32, page: [f32; 4]) {
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
    let mut texts: Vec<String> = layout_labels(&grid, width, height, PLANE_FONT_SIZE, label_width)
        .into_iter()
        .map(|label| label.text)
        .collect();
    texts.push(crate::scale_legend::legend_text(scale));
    texts
}

/// Paints the plane over the canvas's content box: the grid, the axes, the
/// curves, and, once the font has loaded, the labels and the scale legend
/// (the `drawCartesianPlane` order: grid, curves, legend).
// Nine paint inputs (the plane's six plus the frame's curves with their
// ink): a bundle struct would hide the draw order this signature documents.
#[allow(clippy::too_many_arguments)]
pub fn paint_plane(
    fonts: &mut mv_text::Fonts,
    vg: &mut mv_nvg::Context,
    content: &clay::BoundingBox,
    center: [f64; 2],
    scale: f64,
    colors: &PlaneColors,
    font_ready: bool,
    curves: &[crate::plot::PlotCurve],
    line: [f32; 4],
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
    // The labels and the legend wait for their font; the curves draw either
    // way, between them in draw order (as `drawCartesianPlane` draws the
    // curves before the font-gated legend).
    let legend = if font_ready {
        let style = label_style();
        // `layout_labels` takes a shared measure while shaping borrows the
        // fonts mutably: the cell lends them out call by call, which is sound
        // because the layout never re-enters the measure.
        let cell = std::cell::RefCell::new(fonts);
        let measure = |text: &str| {
            let mut guard = cell.borrow_mut();
            let fonts: &mut mv_text::Fonts = &mut guard;
            f64::from(mv_text::measure(fonts, style, text))
        };
        let labels = layout_labels(&grid, width, height, PLANE_FONT_SIZE, &measure);
        let legend_label = crate::scale_legend::legend_text(scale);
        let legend_width = measure(&legend_label);
        let geometry = legend_geometry(width, height, legend_width, scale);
        let fonts = cell.into_inner();
        paint_labels(fonts, vg, &labels, colors.label);
        Some((fonts, geometry))
    } else {
        None
    };
    paint_curves(vg, content.width, content.height, &grid, curves, line);
    if let Some((fonts, geometry)) = legend {
        paint_legend(fonts, vg, &geometry, &colors.legend);
    }
    vg.reset_scissor();
    paint_corner_cutouts(vg, content.width, content.height, colors.page);
    // The CSS border stays Clay's: the canvas element's Border command strokes
    // the border box inset half a px, the border's exact pixels.
    vg.restore();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The mono advance the grid tests pin: a char is 8.4 px at 14 px.
    fn probe_width(text: &str) -> f64 {
        text.len() as f64 * 8.4
    }

    /// The port's boxes against `testdata/page.json` (the reference at this
    /// hash, `tools/run dom --fixture`): the canvas and the overlay button
    /// within the plan's 0.5 CSS px, and the content and button boxes derived
    /// from the canvas exactly.
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
        let want = boxes["overlay_button"].as_array().expect("button box");
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
        let labels = layout_labels(&grid, 990.0, 786.0, PLANE_FONT_SIZE, &probe_width);
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
        let labels = layout_labels(&grid, 990.0, 786.0, PLANE_FONT_SIZE, &probe_width);
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
        let lines = crate::grid::x_label_lines(&grid, 990.0, &probe_width);
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
