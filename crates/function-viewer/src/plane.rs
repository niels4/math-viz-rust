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

/// Traces vertical lines over the content's full height and horizontal lines
/// over its full width into one path.
fn trace_lines(
    vg: &mut mv_nvg::Context,
    content: &clay::BoundingBox,
    vertical: &[crate::grid::GridLine],
    horizontal: &[crate::grid::GridLine],
) {
    for line in vertical {
        vg.move_to(line.px as f32, content.y);
        vg.line_to(line.px as f32, content.y + content.height);
    }
    for line in horizontal {
        vg.move_to(content.x, line.px as f32);
        vg.line_to(content.x + content.width, line.px as f32);
    }
}

/// Paints the minor and major grid lines, 1 px each.
pub fn paint_grid(
    vg: &mut mv_nvg::Context,
    content: &clay::BoundingBox,
    grid: &GridLayout,
    colors: &PlaneColors,
) {
    vg.begin_path();
    trace_lines(vg, content, &grid.minor_x, &grid.minor_y);
    vg.stroke_color(colors.grid);
    vg.stroke_width(1.0);
    vg.stroke();

    vg.begin_path();
    trace_lines(vg, content, &grid.major_x, &grid.major_y);
    vg.stroke_color(colors.grid_major);
    vg.stroke_width(1.0);
    vg.stroke();
}

/// Paints the axes, 2 px, with a tick at every major line.
pub fn paint_axes(
    vg: &mut mv_nvg::Context,
    content: &clay::BoundingBox,
    grid: &GridLayout,
    axis: [f32; 4],
) {
    vg.begin_path();
    if let Some(x) = grid.x_origin {
        let x = x as f32;
        vg.move_to(x, content.y);
        vg.line_to(x, content.y + content.height);
        for line in &grid.major_y {
            let y = line.px as f32;
            vg.move_to(x - TICK_HALF as f32, y);
            vg.line_to(x + TICK_HALF as f32, y);
        }
    }
    if let Some(y) = grid.y_origin {
        let y = y as f32;
        vg.move_to(content.x, y);
        vg.line_to(content.x + content.width, y);
        for line in &grid.major_x {
            let x = line.px as f32;
            vg.move_to(x, y - TICK_HALF as f32);
            vg.line_to(x, y + TICK_HALF as f32);
        }
    }
    vg.stroke_color(axis);
    vg.stroke_width(2.0);
    vg.stroke();
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

/// Fills the content box's corner triangles with the page colour: the canvas
/// bitmap paints square corners and the CSS radius rounds them after, so the
/// port cuts the same triangles out after painting its ink.
fn paint_corner_cutouts(vg: &mut mv_nvg::Context, content: &clay::BoundingBox, page: [f32; 4]) {
    /// The bezier control offset for a quarter circle of radius 1.
    const KAPPA: f32 = 0.552_284_8;
    let r = CANVAS_RADIUS;
    // Each corner: its sharp point and its arc centre.
    let corners = [
        ([content.x, content.y], [content.x + r, content.y + r]),
        (
            [content.x + content.width, content.y],
            [content.x + content.width - r, content.y + r],
        ),
        (
            [content.x, content.y + content.height],
            [content.x + r, content.y + content.height - r],
        ),
        (
            [content.x + content.width, content.y + content.height],
            [
                content.x + content.width - r,
                content.y + content.height - r,
            ],
        ),
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

/// Paints the plane over the canvas's content box: the grid, the axes, and,
/// once the font has loaded, the labels and the scale legend.
pub fn paint_plane(
    fonts: &mut mv_text::Fonts,
    vg: &mut mv_nvg::Context,
    content: &clay::BoundingBox,
    center: [f64; 2],
    scale: f64,
    colors: &PlaneColors,
    font_ready: bool,
) {
    let width = f64::from(content.width);
    let height = f64::from(content.height);
    let grid = layout_grid(center, scale, width, height, LABEL_MARGIN);
    vg.save();
    vg.scissor(content.x, content.y, content.width, content.height);
    vg.line_cap(mv_nvg::LineCapType::BUTT);
    paint_grid(vg, content, &grid, colors);
    paint_axes(vg, content, &grid, colors.axis);
    if font_ready {
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
        let fonts = cell.into_inner();
        paint_labels(fonts, vg, &labels, colors.label);
        let geometry = legend_geometry(width, height, legend_width, scale);
        paint_legend(fonts, vg, &geometry, &colors.legend);
    }
    vg.reset_scissor();
    paint_corner_cutouts(vg, content, colors.page);
    // The CSS border, `1px solid var(--border)`: a 1 px stroke centred on the
    // border box inset half a px covers the border's exact pixels, with the
    // outer radius concentric (4 px outside, 3 px inside).
    vg.begin_path();
    vg.rounded_rect_varying(
        content.x - 0.5,
        content.y - 0.5,
        content.width + 1.0,
        content.height + 1.0,
        CANVAS_RADIUS - 0.5,
        CANVAS_RADIUS - 0.5,
        CANVAS_RADIUS - 0.5,
        CANVAS_RADIUS - 0.5,
    );
    vg.stroke_color(colors.grid);
    vg.stroke_width(1.0);
    vg.stroke();
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
}
