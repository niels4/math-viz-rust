//! The plane's painters: the frame pipeline and every layer drawn through
//! the painter backend.

use std::f64::consts::PI;

use mv_nvg::{LineCapType, Winding};
use mv_text::{Fonts, Style as TextStyle};
use mv_theme::{Color, Theme, Token};

use super::chrome::{BRACKET_PATHS, BRACKET_WIDTH, Corner, bracket_box, corner_boxes};
use super::dash::{MIN_PIECE_PX, dash_circle, dash_phase, dash_polyline};
use super::faces::{
    Face, Measure, ORIGIN_FACE, ReadoutFaces, TICK_LABEL_FACE, baseline_below_top, style,
};
use super::grid::{GridLayout, GridLines, TICK_HALF, layout_grid};
use super::marks::{
    AnnotationMarks, AnnotationStroke, HandleMark, Marker, MarksLayout, MarksOptions, Segment,
    WasMark, layout_marks,
};
use super::placement::Direction;
use super::plates::{Plate, PlateMotion, PlateRunWhat, TRIANGLE_INSET, TRIANGLE_SIZE};
use super::rect::Rect;
use super::scene::{AnnotationLayer, Curve, HandleShape, Ink, PointStyle, Scene};
use super::trace::{curve_trace, trace_up_to};
use super::viewport::Viewport;

/// The painter backend: the drawing calls the plane's painters make.
///
/// The production backend is the NanoVG context; the tests count through a
/// recording fake. A trait object keeps every painter signature unchanged
/// however the backend is provided.
pub trait PaintDevice {
    /// Pushes the state.
    fn save(&mut self);
    /// Pops the state.
    fn restore(&mut self);
    /// Sets the global alpha.
    fn global_alpha(&mut self, alpha: f32);
    /// Sets the cap of stroked ends.
    fn line_cap(&mut self, cap: LineCapType);
    /// Sets the join of stroked corners.
    fn line_join(&mut self, join: LineCapType);
    /// Sets the stroke colour.
    fn stroke_color(&mut self, color: Color);
    /// Sets the fill colour.
    fn fill_color(&mut self, color: Color);
    /// Sets the stroke width.
    fn stroke_width(&mut self, width: f32);
    /// Starts a path.
    fn begin_path(&mut self);
    /// Starts a subpath at x, y.
    fn move_to(&mut self, x: f32, y: f32);
    /// Continues the subpath to x, y.
    fn line_to(&mut self, x: f32, y: f32);
    /// Closes the subpath.
    fn close_path(&mut self);
    /// Adds a rounded rectangle at x, y of w by h with corner radius.
    fn rounded_rect(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32);
    /// Adds a circle at x, y of radius r.
    fn circle(&mut self, x: f32, y: f32, r: f32);
    /// Adds an arc at x, y of radius r from a0 to a1, going dir.
    fn arc(&mut self, x: f32, y: f32, r: f32, a0: f32, a1: f32, dir: Winding);
    /// Strokes the path.
    fn stroke(&mut self);
    /// Fills the path.
    fn fill(&mut self);
    /// Strokes the path feathered over feather px.
    fn stroke_feathered(&mut self, feather: f32);
    /// Draws shaped text in a style, its pen starting at x on the baseline y.
    fn draw_text(
        &mut self,
        fonts: &mut Fonts,
        style: TextStyle,
        x: f32,
        y: f32,
        s: &str,
        color: Color,
    );
}

/// The NanoVG backend: every call straight through.
impl PaintDevice for mv_nvg::Context {
    fn save(&mut self) {
        self.save();
    }
    fn restore(&mut self) {
        self.restore();
    }
    fn global_alpha(&mut self, alpha: f32) {
        self.global_alpha(alpha);
    }
    fn line_cap(&mut self, cap: LineCapType) {
        self.line_cap(cap);
    }
    fn line_join(&mut self, join: LineCapType) {
        self.line_join(join);
    }
    fn stroke_color(&mut self, color: Color) {
        self.stroke_color(color);
    }
    fn fill_color(&mut self, color: Color) {
        self.fill_color(color);
    }
    fn stroke_width(&mut self, width: f32) {
        self.stroke_width(width);
    }
    fn begin_path(&mut self) {
        self.begin_path();
    }
    fn move_to(&mut self, x: f32, y: f32) {
        self.move_to(x, y);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.line_to(x, y);
    }
    fn close_path(&mut self) {
        self.close_path();
    }
    fn rounded_rect(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32) {
        self.rounded_rect(x, y, w, h, radius);
    }
    fn circle(&mut self, x: f32, y: f32, r: f32) {
        self.circle(x, y, r);
    }
    fn arc(&mut self, x: f32, y: f32, r: f32, a0: f32, a1: f32, dir: Winding) {
        self.arc(x, y, r, a0, a1, dir);
    }
    fn stroke(&mut self) {
        self.stroke();
    }
    fn fill(&mut self) {
        self.fill();
    }
    fn stroke_feathered(&mut self, feather: f32) {
        self.stroke_feathered(feather);
    }
    fn draw_text(
        &mut self,
        fonts: &mut Fonts,
        style: TextStyle,
        x: f32,
        y: f32,
        s: &str,
        color: Color,
    ) {
        mv_text::draw(fonts, self, style, x, y, s, color);
    }
}

/// The painter: the backend, the fonts, the theme and the density.
pub struct Painter<'a> {
    pub vg: &'a mut dyn PaintDevice,
    pub fonts: &'a mut mv_text::Fonts,
    pub theme: &'a Theme,
    pub density: f64,
}

impl Painter<'_> {
    /// Lends the painter out again for one nested call.
    pub(crate) fn reborrow(&mut self) -> Painter<'_> {
        Painter {
            vg: &mut *self.vg,
            fonts: &mut *self.fonts,
            theme: self.theme,
            density: self.density,
        }
    }
}

/// A laid-out frame: the viewport, scene, grid and marks.
#[derive(Debug)]
pub struct Frame {
    pub vp: Viewport,
    pub scene: Scene,
    pub grid: GridLayout,
    pub marks: MarksLayout,
}

/// Lays out a frame from a scene in a viewport.
pub fn lay_out(
    scene: Scene,
    vp: Viewport,
    m: Measure<'_>,
    readout: ReadoutFaces,
    plates: &[Rect],
) -> Frame {
    let corners = corner_boxes(vp.width, vp.height);
    let mut opts = MarksOptions {
        measure: m,
        readout,
        plates: plates.to_vec(),
        corners: corners.to_vec(),
    };
    let marks = layout_marks(&scene, vp, &mut opts);
    let MarksOptions { mut measure, .. } = opts;
    let mut keep_out = Vec::with_capacity(plates.len() + corners.len());
    keep_out.extend_from_slice(plates);
    keep_out.extend_from_slice(&corners);
    let grid = layout_grid(vp, &mut measure, &keep_out);
    Frame {
        vp,
        scene,
        grid,
        marks,
    }
}

/// Draws a frame.
pub fn draw(mut p: Painter<'_>, f: &Frame) {
    let vp = f.vp;
    if vp.width == 0.0 || vp.height == 0.0 {
        return;
    }
    p.vg.save();
    p.vg.global_alpha(f.scene.grid_alpha.unwrap_or(1.0).clamp(0.0, 1.0) as f32);
    paint_grid(p.reborrow(), &f.grid, vp.width, vp.height);
    p.vg.restore();
    for back in [true, false] {
        if !back {
            for a in &f.marks.annotations {
                if a.layer == AnnotationLayer::Under {
                    paint_annotation(p.reborrow(), a);
                }
            }
        }
        for curve in &f.scene.curves {
            if curve.back == back {
                paint_curve(p.reborrow(), vp, curve);
            }
        }
    }
    paint_handle_halos(p.reborrow(), &f.marks.handles);
    for a in &f.marks.annotations {
        if a.layer == AnnotationLayer::Over {
            paint_annotation(p.reborrow(), a);
        }
    }
    paint_handles(p.reborrow(), &f.marks.handles);
    paint_guides(p.reborrow(), &f.marks, vp.height);
    for layer in &f.marks.points {
        if let Some(was) = &layer.was {
            paint_was(p.reborrow(), was);
        }
        if let Some(edge) = &layer.edge {
            paint_plate(p.reborrow(), edge);
        }
        if let Some(marker) = &layer.marker {
            paint_marker(p.reborrow(), marker);
        }
        if let Some(label) = &layer.label {
            paint_plate(p.reborrow(), label);
        }
    }
}

/// A theme token resolved to a drawing colour.
fn ink(p: &Painter<'_>, i: Ink) -> Color {
    p.theme.colors[i]
}

/// Grid lines at 1 px.
pub const GRID_WIDTH: f64 = 1.0;

/// Axes at 2 px.
pub const AXIS_WIDTH: f64 = 2.0;

/// Strokes a set of lines in a colour at a width.
fn stroke_lines(
    p: Painter<'_>,
    xs: &[f64],
    ys: &[f64],
    width: f64,
    height: f64,
    color: Color,
    line_width: f64,
) {
    if xs.is_empty() && ys.is_empty() {
        return;
    }
    p.vg.begin_path();
    for x in xs {
        p.vg.move_to(*x as f32, 0.0);
        p.vg.line_to(*x as f32, height as f32);
    }
    for y in ys {
        p.vg.move_to(0.0, *y as f32);
        p.vg.line_to(width as f32, *y as f32);
    }
    p.vg.stroke_color(color);
    p.vg.stroke_width(line_width as f32);
    p.vg.stroke();
}

/// Paints the grid lines of a layout.
pub fn paint_grid_lines(
    mut p: Painter<'_>,
    lines: &GridLines,
    width: f64,
    height: f64,
    axis_width: f64,
) {
    p.vg.save();
    p.vg.line_cap(LineCapType::BUTT);
    let minor = ink(&p, Token::ChartGrid);
    let major = ink(&p, Token::ChartGridMajor);
    let axis = ink(&p, Token::ChartAxis);
    stroke_lines(
        p.reborrow(),
        &lines.minor_x,
        &lines.minor_y,
        width,
        height,
        minor,
        GRID_WIDTH,
    );
    stroke_lines(
        p.reborrow(),
        &lines.major_x,
        &lines.major_y,
        width,
        height,
        major,
        GRID_WIDTH,
    );
    let mut axes_x = [0.0f64; 1];
    let mut axes_y = [0.0f64; 1];
    let mut nx = 0;
    let mut ny = 0;
    if let Some(x) = lines.axis_y {
        axes_x[0] = x;
        nx = 1;
    }
    if let Some(y) = lines.axis_x {
        axes_y[0] = y;
        ny = 1;
    }
    stroke_lines(
        p.reborrow(),
        &axes_x[..nx],
        &axes_y[..ny],
        width,
        height,
        axis,
        axis_width,
    );
    p.vg.restore();
}

/// Paints a laid-out grid.
pub fn paint_grid(mut p: Painter<'_>, grid: &GridLayout, width: f64, height: f64) {
    paint_grid_lines(
        p.reborrow(),
        &GridLines {
            minor_x: grid.minor_x.clone(),
            major_x: grid.major_x.clone(),
            minor_y: grid.minor_y.clone(),
            major_y: grid.major_y.clone(),
            axis_x: grid.axis_x,
            axis_y: grid.axis_y,
        },
        width,
        height,
        AXIS_WIDTH,
    );
    p.vg.save();
    p.vg.line_cap(LineCapType::BUTT);
    p.vg.begin_path();
    if let Some(axis) = grid.axis_x {
        for x in &grid.ticks_x {
            p.vg.move_to(*x as f32, (axis - TICK_HALF) as f32);
            p.vg.line_to(*x as f32, (axis + TICK_HALF) as f32);
        }
    }
    if let Some(axis) = grid.axis_y {
        for y in &grid.ticks_y {
            p.vg.move_to((axis - TICK_HALF) as f32, *y as f32);
            p.vg.line_to((axis + TICK_HALF) as f32, *y as f32);
        }
    }
    let axis_color = ink(&p, Token::ChartAxis);
    p.vg.stroke_color(axis_color);
    p.vg.stroke_width(AXIS_WIDTH as f32);
    p.vg.stroke();

    let muted = ink(&p, Token::ForegroundMuted);
    let label_baseline = baseline_below_top(TICK_LABEL_FACE);
    for label in &grid.labels {
        fill_text(
            p.reborrow(),
            TICK_LABEL_FACE,
            &label.text,
            label.r#box.x,
            label.r#box.y + label_baseline,
            muted,
        );
    }
    if let Some(o) = &grid.origin {
        fill_text(
            p.reborrow(),
            ORIGIN_FACE,
            "O",
            o.x,
            o.y + baseline_below_top(ORIGIN_FACE),
            muted,
        );
    }
    p.vg.restore();
}

/// Fills a string in a face and colour.
fn fill_text(p: Painter<'_>, face: Face, s: &str, x: f64, y: f64, color: Color) {
    let st = style(face);
    p.vg.draw_text(p.fonts, st, x as f32, y as f32, s, color);
}

/// Paints a curve through a viewport.
pub fn paint_curve(mut p: Painter<'_>, vp: Viewport, curve: &Curve) {
    let samples = trace_up_to(&curve_trace(vp, &curve.func), curve.draw_to.unwrap_or(1.0));
    if samples.len() < 2 {
        return;
    }
    let mut runs: Vec<Vec<[f64; 2]>> = Vec::new();
    let mut run: Vec<[f64; 2]> = Vec::new();
    for s in &samples {
        if s.start && !run.is_empty() {
            if run.len() >= 2 {
                runs.push(std::mem::take(&mut run));
            } else {
                run.clear();
            }
        }
        run.push([s.x, s.y]);
    }
    if run.len() >= 2 {
        runs.push(run);
    }
    if runs.is_empty() {
        return;
    }

    p.vg.save();
    if let Some(alpha) = curve.alpha {
        p.vg.global_alpha(alpha as f32);
    }
    let glow = ink(&p, Token::SigCurveGlow);
    let glow_radius = f64::from(p.theme.glow_radius);
    if curve.glow && glow_radius > 0.0 && glow[3] > 0.0 {
        paint_glow(
            p.reborrow(),
            &runs,
            curve.width,
            glow,
            glow_radius / 2.0,
            vp.height,
        );
    }
    p.vg.begin_path();
    for r in &runs {
        add_polyline(&mut *p.vg, r, curve.dash);
    }
    let stroke_color = ink(&p, curve.ink);
    p.vg.stroke_color(stroke_color);
    p.vg.stroke_width(curve.width as f32);
    p.vg.line_cap(LineCapType::ROUND);
    p.vg.line_join(LineCapType::ROUND);
    p.vg.stroke();
    p.vg.restore();
}

/// Adds a dashed or plain polyline to the path.
pub fn add_polyline(vg: &mut dyn PaintDevice, points: &[[f64; 2]], dash: Option<[f64; 2]>) {
    match dash {
        None => move_through(vg, points),
        Some(pattern) => {
            for piece in dash_polyline(points, pattern, dash_phase(pattern)) {
                move_through(vg, &piece);
            }
        }
    }
}

/// Moves through points, closing the gaps between runs.
fn move_through(vg: &mut dyn PaintDevice, points: &[[f64; 2]]) {
    for (i, pt) in points.iter().enumerate() {
        if i == 0 {
            vg.move_to(pt[0] as f32, pt[1] as f32);
        } else {
            vg.line_to(pt[0] as f32, pt[1] as f32);
        }
    }
}

/// The glow's stacked strokes step, in px.
pub const GLOW_STEP: f64 = 2.0;

/// Paints a glow: a curve's runs stacked as feathered strokes.
pub fn paint_glow(
    p: Painter<'_>,
    runs: &[Vec<[f64; 2]>],
    width: f64,
    color: Color,
    sigma: f64,
    height: f64,
) {
    fn phi(x: f64) -> f64 {
        0.5 * (1.0 + libm::erf(x / std::f64::consts::SQRT_2))
    }
    fn profile(d: f64, width: f64, sigma: f64, alpha: f64) -> f64 {
        alpha * (phi((d + width / 2.0) / sigma) - phi((d - width / 2.0) / sigma))
    }
    let alpha = f64::from(color[3]);
    let mut knots: Vec<f64> = Vec::new();
    let mut d = width / 2.0 - GLOW_STEP / 2.0;
    loop {
        knots.push(profile(d, width, sigma, alpha));
        if *knots.last().expect("pushed this iteration") < 0.5 / 255.0 {
            break;
        }
        d += GLOW_STEP;
    }
    let reach = width / 2.0 + knots.len() as f64 * GLOW_STEP;
    let near = |pt: &[f64; 2], reach: f64, height: f64| pt[1] >= -reach && pt[1] <= height + reach;
    let mut pieces: Vec<&[[f64; 2]]> = Vec::new();
    for r in runs {
        let mut from: Option<usize> = None;
        for i in 0..=r.len() {
            let keep = i < r.len()
                && (near(&r[i], reach, height)
                    || (i > 0 && near(&r[i - 1], reach, height))
                    || (i + 1 < r.len() && near(&r[i + 1], reach, height)));
            if keep && from.is_none() {
                from = Some(i);
            } else if !keep && from.is_some() {
                pieces.push(&r[from.expect("set when keep started")..i]);
                from = None;
            }
        }
    }
    p.vg.save();
    p.vg.line_cap(LineCapType::ROUND);
    p.vg.line_join(LineCapType::ROUND);
    let n = knots.len() - 1;
    let mut outer = 0.0f64;
    for j in (0..n).rev() {
        let a = 1.0 - (1.0 - knots[j]) / (1.0 - outer);
        outer = knots[j];
        if a <= 0.0 {
            continue;
        }
        let edge = width / 2.0 - GLOW_STEP / 2.0 + (j as f64 + 0.5) * GLOW_STEP;
        p.vg.begin_path();
        for piece in &pieces {
            move_through(&mut *p.vg, piece);
        }
        let mut c = color;
        c[3] = a as f32;
        p.vg.stroke_color(c);
        p.vg.stroke_width((2.0 * edge) as f32);
        p.vg.stroke_feathered(GLOW_STEP as f32);
    }
    p.vg.restore();
}

/// A tick's width, in px.
const TICK_WIDTH: f64 = 2.0;

/// An arrowhead's length, in px.
const ARROW_LENGTH: f64 = 9.8;

/// An arrowhead's half width, in px.
const ARROW_HALF: f64 = 4.9;

/// A segment's unit vector and length.
fn unit(s: &Segment) -> (f64, f64, f64) {
    let length = (s.x2 - s.x1).hypot(s.y2 - s.y1);
    if length == 0.0 {
        return (0.0, 0.0, 0.0);
    }
    ((s.x2 - s.x1) / length, (s.y2 - s.y1) / length, length)
}

/// Paints an arrowhead at a segment's end.
pub fn paint_arrowhead(p: Painter<'_>, s: &Segment, color: Color) {
    let (ux, uy, length) = unit(s);
    if length == 0.0 {
        return;
    }
    let (bx, by) = (s.x2 - ux * ARROW_LENGTH, s.y2 - uy * ARROW_LENGTH);
    p.vg.begin_path();
    p.vg.move_to(s.x2 as f32, s.y2 as f32);
    p.vg.line_to((bx - uy * ARROW_HALF) as f32, (by + ux * ARROW_HALF) as f32);
    p.vg.line_to((bx + uy * ARROW_HALF) as f32, (by - ux * ARROW_HALF) as f32);
    p.vg.close_path();
    p.vg.fill_color(color);
    p.vg.fill();
}

/// Paints a segment wide, dashed or arrowed.
pub fn paint_line(
    mut p: Painter<'_>,
    s: &Segment,
    width: f64,
    color: Color,
    dash: Option<[f64; 2]>,
    arrow: bool,
) {
    let (ux, uy, length) = unit(s);
    let short = if arrow {
        length.min(width.max(2.0))
    } else {
        0.0
    };
    p.vg.save();
    p.vg.line_cap(LineCapType::BUTT);
    p.vg.stroke_width(width as f32);
    p.vg.stroke_color(color);
    if length - short >= MIN_PIECE_PX {
        p.vg.begin_path();
        add_polyline(
            &mut *p.vg,
            &[[s.x1, s.y1], [s.x2 - ux * short, s.y2 - uy * short]],
            dash,
        );
        p.vg.stroke();
    }
    p.vg.restore();
    if arrow {
        paint_arrowhead(p.reborrow(), s, color);
    }
}

/// Ticks across a point along a direction.
fn tick(vg: &mut dyn PaintDevice, x: f64, y: f64, ux: f64, uy: f64, half: f64) {
    let (nx, ny) = if ux == 0.0 && uy == 0.0 {
        (0.0, 1.0)
    } else {
        (-uy, ux)
    };
    vg.move_to((x - nx * half) as f32, (y - ny * half) as f32);
    vg.line_to((x + nx * half) as f32, (y + ny * half) as f32);
}

/// Paints an annotation stroke.
fn paint_stroke(mut p: Painter<'_>, stroke: &AnnotationStroke, color: Color) {
    p.vg.save();
    if let Some(alpha) = stroke.alpha {
        p.vg.global_alpha(alpha as f32);
    }
    let segment = Segment {
        x1: stroke.x1,
        y1: stroke.y1,
        x2: stroke.x2,
        y2: stroke.y2,
    };
    paint_line(
        p.reborrow(),
        &segment,
        stroke.width,
        color,
        stroke.dash,
        stroke.arrow,
    );
    let (ux, uy, _) = unit(&segment);
    p.vg.line_cap(LineCapType::BUTT);
    p.vg.stroke_width(TICK_WIDTH as f32);
    p.vg.stroke_color(color);
    p.vg.begin_path();
    if let Some(half) = stroke.start_tick {
        tick(&mut *p.vg, stroke.x1, stroke.y1, ux, uy, half);
    }
    if let Some(half) = stroke.end_ticks {
        tick(&mut *p.vg, stroke.x1, stroke.y1, ux, uy, half);
        tick(&mut *p.vg, stroke.x2, stroke.y2, ux, uy, half);
    }
    p.vg.stroke();
    p.vg.restore();
}

/// Paints an annotation's marks.
pub fn paint_annotation(mut p: Painter<'_>, marks: &AnnotationMarks) {
    let color = ink(&p, marks.ink);
    for stroke in &marks.strokes {
        paint_stroke(p.reborrow(), stroke, color);
    }
}

/// A guide's width, in px.
const GUIDE_WIDTH: f64 = 1.0;

/// A guide's dash pattern.
const GUIDE_DASH: [f64; 2] = [4.0, 6.0];

/// A drop line's width, in px.
const DROP_WIDTH: f64 = 1.5;

/// A drop line's dash pattern.
const DROP_DASH: [f64; 2] = [5.0, 4.0];

/// Paints the guides and drop lines of a layout.
pub fn paint_guides(p: Painter<'_>, marks: &MarksLayout, height: f64) {
    p.vg.save();
    p.vg.line_cap(LineCapType::BUTT);
    let muted = ink(&p, Token::ForegroundMuted);
    p.vg.stroke_color(muted);
    p.vg.stroke_width(GUIDE_WIDTH as f32);
    for guide in &marks.guides {
        p.vg.global_alpha(guide.alpha.unwrap_or(1.0) as f32);
        p.vg.begin_path();
        add_polyline(
            &mut *p.vg,
            &[[guide.x, 0.0], [guide.x, height]],
            Some(GUIDE_DASH),
        );
        p.vg.stroke();
    }
    p.vg.stroke_width(DROP_WIDTH as f32);
    for line in &marks.drop_lines {
        if line.x1 == line.x2 && line.y1 == line.y2 {
            continue;
        }
        p.vg.global_alpha(line.alpha.unwrap_or(1.0) as f32);
        let line_color = ink(&p, line.ink);
        p.vg.stroke_color(line_color);
        p.vg.begin_path();
        add_polyline(
            &mut *p.vg,
            &[[line.x1, line.y1], [line.x2, line.y2]],
            Some(DROP_DASH),
        );
        p.vg.stroke();
    }
    p.vg.restore();
}

/// A handle halo's radius, in px.
const HANDLE_HALO_R: f64 = 22.0;

/// A handle halo's alpha.
const HALO_ALPHA: f64 = 0.26;

/// A diamond handle's radius, in px.
const DIAMOND_R: f64 = 10.0;

/// A square handle's side, in px.
const SQUARE: f64 = 16.0;

/// A square handle's corner radius, in px.
const SQUARE_RADIUS: f64 = 3.0;

/// A handle's stroke width, in px.
const HANDLE_STROKE: f64 = 2.5;

/// Paints the halos of held handles.
pub fn paint_handle_halos(p: Painter<'_>, handles: &[HandleMark]) {
    p.vg.save();
    for h in handles {
        if !h.halo {
            continue;
        }
        p.vg.global_alpha((HALO_ALPHA * h.alpha.unwrap_or(1.0)) as f32);
        let halo_color = ink(&p, h.ink);
        p.vg.begin_path();
        p.vg.circle(h.x as f32, h.y as f32, HANDLE_HALO_R as f32);
        p.vg.fill_color(halo_color);
        p.vg.fill();
    }
    p.vg.restore();
}

/// Paints the handles.
pub fn paint_handles(p: Painter<'_>, handles: &[HandleMark]) {
    p.vg.save();
    p.vg.stroke_width(HANDLE_STROKE as f32);
    for h in handles {
        p.vg.global_alpha(h.alpha.unwrap_or(1.0) as f32);
        let color = ink(&p, h.ink);
        let background = ink(&p, Token::Background);
        p.vg.fill_color(if h.held { color } else { background });
        p.vg.stroke_color(color);
        let (x, y) = (h.x as f32, h.y as f32);
        p.vg.begin_path();
        match h.shape {
            HandleShape::Diamond => {
                p.vg.line_join(LineCapType::ROUND);
                p.vg.move_to(x, y - DIAMOND_R as f32);
                p.vg.line_to(x + DIAMOND_R as f32, y);
                p.vg.line_to(x, y + DIAMOND_R as f32);
                p.vg.line_to(x - DIAMOND_R as f32, y);
                p.vg.close_path();
                p.vg.fill();
                p.vg.stroke();
            }
            HandleShape::Square => {
                let half = SQUARE / 2.0;
                let inset = HANDLE_STROKE / 2.0;
                p.vg.rounded_rect(
                    x - half as f32,
                    y - half as f32,
                    SQUARE as f32,
                    SQUARE as f32,
                    SQUARE_RADIUS as f32,
                );
                p.vg.fill();
                p.vg.begin_path();
                p.vg.rounded_rect(
                    x - half as f32 + inset as f32,
                    y - half as f32 + inset as f32,
                    (SQUARE - HANDLE_STROKE) as f32,
                    (SQUARE - HANDLE_STROKE) as f32,
                    (SQUARE_RADIUS - inset) as f32,
                );
                p.vg.stroke();
            }
        }
    }
    p.vg.restore();
}

/// A marker knockout's radius, in px.
const KNOCKOUT_R: f64 = 12.0;

/// A marker halo's radius, in px.
const MARKER_HALO_R: f64 = 24.0;

/// Discs a filled circle.
fn disc(vg: &mut dyn PaintDevice, x: f64, y: f64, r: f64, color: Color) {
    vg.begin_path();
    vg.circle(x as f32, y as f32, r as f32);
    vg.fill_color(color);
    vg.fill();
}

/// Rings a circle outline.
fn ring(vg: &mut dyn PaintDevice, x: f64, y: f64, r: f64, width: f64, color: Color) {
    vg.begin_path();
    vg.circle(x as f32, y as f32, (r - width / 2.0) as f32);
    vg.stroke_width(width as f32);
    vg.stroke_color(color);
    vg.stroke();
}

/// Paints a point marker.
pub fn paint_marker(p: Painter<'_>, m: &Marker) {
    let alpha = m.alpha.unwrap_or(1.0);
    p.vg.save();
    let primary = ink(&p, Token::Primary);
    let background = ink(&p, Token::Background);
    let foreground = ink(&p, Token::Foreground);
    if m.focus {
        p.vg.global_alpha((alpha * HALO_ALPHA) as f32);
        disc(&mut *p.vg, m.x, m.y, MARKER_HALO_R, primary);
    }
    p.vg.global_alpha(alpha as f32);
    disc(&mut *p.vg, m.x, m.y, KNOCKOUT_R, background);
    let dot = ink(&p, m.ink);
    match m.style {
        PointStyle::Bullseye => {
            ring(&mut *p.vg, m.x, m.y, KNOCKOUT_R, 2.5, foreground);
            disc(&mut *p.vg, m.x, m.y, 6.0, dot);
        }
        PointStyle::Ring => {
            ring(&mut *p.vg, m.x, m.y, KNOCKOUT_R, 3.0, dot);
            disc(&mut *p.vg, m.x, m.y, 3.0, foreground);
        }
    }
    p.vg.restore();
}

/// A past position's radius, in px.
const WAS_R: f64 = 11.0;

/// A past position's line width, in px.
const WAS_WIDTH: f64 = 2.0;

/// A past position's dash count.
const WAS_DASHES: f64 = 12.0;

/// A past position's arrow width, in px.
const WAS_ARROW_WIDTH: f64 = 2.0;

/// Paints a past position.
pub fn paint_was(mut p: Painter<'_>, was: &WasMark) {
    let r = WAS_R - WAS_WIDTH / 2.0;
    let d = PI * r / WAS_DASHES;
    p.vg.save();
    p.vg.line_cap(LineCapType::BUTT);
    p.vg.begin_path();
    for arc in dash_circle(r, [d, d], dash_phase([d, d])) {
        p.vg.move_to(
            (was.x + r * arc[0].cos()) as f32,
            (was.y + r * arc[0].sin()) as f32,
        );
        p.vg.arc(
            was.x as f32,
            was.y as f32,
            r as f32,
            arc[0] as f32,
            arc[1] as f32,
            Winding::CW,
        );
    }
    p.vg.stroke_width(WAS_WIDTH as f32);
    let was_color = ink(&p, was.ink);
    p.vg.stroke_color(was_color);
    p.vg.stroke();
    p.vg.restore();
    if let Some(arrow) = &was.arrow {
        let arrow_color = ink(&p, arrow.ink);
        paint_line(
            p.reborrow(),
            &Segment {
                x1: arrow.x1,
                y1: arrow.y1,
                x2: arrow.x2,
                y2: arrow.y2,
            },
            WAS_ARROW_WIDTH,
            arrow_color,
            None,
            true,
        );
    }
}

/// Paints a plate.
pub fn paint_plate(mut p: Painter<'_>, plate: &Plate) {
    let motion = plate.motion.unwrap_or(PlateMotion {
        alpha: 1.0,
        dx: 0.0,
        dy: 0.0,
    });
    let (x, y) = (plate.r#box.x + motion.dx, plate.r#box.y + motion.dy);
    let (w, h) = (plate.r#box.w, plate.r#box.h);
    p.vg.save();
    p.vg.global_alpha(motion.alpha as f32);
    p.vg.begin_path();
    p.vg.rounded_rect(x as f32, y as f32, w as f32, h as f32, plate.radius as f32);
    let fill = ink(&p, plate.fill);
    p.vg.fill_color(fill);
    p.vg.fill();
    let half = plate.border_width / 2.0;
    p.vg.begin_path();
    p.vg.rounded_rect(
        (x + half) as f32,
        (y + half) as f32,
        (w - plate.border_width) as f32,
        (h - plate.border_width) as f32,
        (0.0f64.max(plate.radius - half)) as f32,
    );
    p.vg.stroke_width(plate.border_width as f32);
    let border = ink(&p, plate.border_ink);
    p.vg.stroke_color(border);
    p.vg.stroke();
    for run in &plate.runs {
        let color = ink(&p, run.ink);
        match &run.what {
            PlateRunWhat::Text(t) => {
                fill_text(p.reborrow(), t.face, &t.text, x + run.x, y + run.y, color)
            }
            PlateRunWhat::Triangle(t) => paint_triangle(
                &mut *p.vg,
                t.dir,
                x + run.x + TRIANGLE_INSET,
                y + run.y + TRIANGLE_INSET,
                color,
            ),
        }
    }
    p.vg.restore();
}

/// Paints a triangle run in a direction.
pub fn paint_triangle(vg: &mut dyn PaintDevice, dir: Direction, x: f64, y: f64, color: Color) {
    let s = TRIANGLE_SIZE;
    let pts: [[f64; 2]; 3] = match dir {
        Direction::Up => [[s / 2.0, 0.0], [s, s], [0.0, s]],
        Direction::Down => [[0.0, 0.0], [s, 0.0], [s / 2.0, s]],
        Direction::Left => [[s, 0.0], [s, s], [0.0, s / 2.0]],
        Direction::Right => [[0.0, 0.0], [s, s / 2.0], [0.0, s]],
    };
    vg.begin_path();
    for (i, pt) in pts.iter().enumerate() {
        if i == 0 {
            vg.move_to((x + pt[0]) as f32, (y + pt[1]) as f32);
        } else {
            vg.line_to((x + pt[0]) as f32, (y + pt[1]) as f32);
        }
    }
    vg.close_path();
    vg.fill_color(color);
    vg.fill();
}

/// Draws the chrome around a width and height: the four corner brackets.
///
/// The port draws no HUD ticks since the 2026-10-18 ruling: the Odin app draws
/// the ticks in paint.odin:735-752 from chrome.odin:32's geometry, and React
/// still shows them, but the user dislikes how they look, so the brackets
/// stand alone (decisions.md › The port's own calls).
pub fn draw_chrome(p: Painter<'_>, width: f64, height: f64) {
    p.vg.save();
    p.vg.line_cap(LineCapType::SQUARE);
    p.vg.line_join(LineCapType::MITER);
    p.vg.stroke_width(BRACKET_WIDTH as f32);
    let muted = ink(&p, Token::ForegroundMuted);
    p.vg.stroke_color(muted);
    for corner in [
        Corner::TopLeft,
        Corner::TopRight,
        Corner::BottomLeft,
        Corner::BottomRight,
    ] {
        let b = bracket_box(corner, width, height);
        p.vg.begin_path();
        for (i, pt) in BRACKET_PATHS[corner].iter().enumerate() {
            if i == 0 {
                p.vg.move_to((b.x + pt[0]) as f32, (b.y + pt[1]) as f32);
            } else {
                p.vg.line_to((b.x + pt[0]) as f32, (b.y + pt[1]) as f32);
            }
        }
        p.vg.stroke();
    }
    p.vg.restore();
}

#[cfg(test)]
mod tests {
    use super::*;
    use enum_map::Enum as _;
    use enum_map::EnumMap;
    use mv_theme::{ReadoutFont, Slug, Space};

    /// The counted backend: records strokes, fills and surviving paths.
    ///
    /// A subpath survives when some point lies at least `MIN_PIECE_PX` from
    /// its first point, the painters' sliver rule; closed shapes always
    /// survive. The frame calls only reset the counters.
    struct CountingDevice {
        strokes: usize,
        fills: usize,
        paths: usize,
        pending: usize,
        cur: Vec<[f32; 2]>,
    }

    impl CountingDevice {
        fn new() -> Self {
            CountingDevice {
                strokes: 0,
                fills: 0,
                paths: 0,
                pending: 0,
                cur: Vec::new(),
            }
        }

        fn flush(&mut self) {
            if self.cur.len() >= 2 {
                let [x0, y0] = self.cur[0];
                if self
                    .cur
                    .iter()
                    .any(|[x, y]| (x - x0).hypot(y - y0) >= MIN_PIECE_PX as f32)
                {
                    self.pending += 1;
                }
            }
            self.cur.clear();
        }

        fn finish(&mut self, fills: bool) {
            self.flush();
            if fills {
                self.fills += 1;
            } else {
                self.strokes += 1;
            }
            self.paths += self.pending;
            self.pending = 0;
        }
    }

    impl PaintDevice for CountingDevice {
        fn save(&mut self) {}
        fn restore(&mut self) {}
        fn global_alpha(&mut self, _alpha: f32) {}
        fn line_cap(&mut self, _cap: LineCapType) {}
        fn line_join(&mut self, _join: LineCapType) {}
        fn stroke_color(&mut self, _color: Color) {}
        fn fill_color(&mut self, _color: Color) {}
        fn stroke_width(&mut self, _width: f32) {}
        fn begin_path(&mut self) {
            self.cur.clear();
            self.pending = 0;
        }
        fn move_to(&mut self, x: f32, y: f32) {
            self.flush();
            self.cur.push([x, y]);
        }
        fn line_to(&mut self, x: f32, y: f32) {
            self.cur.push([x, y]);
        }
        fn close_path(&mut self) {
            self.flush();
        }
        fn rounded_rect(&mut self, _x: f32, _y: f32, _w: f32, _h: f32, _radius: f32) {
            self.flush();
            self.pending += 1;
        }
        fn circle(&mut self, _x: f32, _y: f32, _r: f32) {
            self.flush();
            self.pending += 1;
        }
        fn arc(&mut self, _x: f32, _y: f32, _r: f32, _a0: f32, _a1: f32, _dir: Winding) {
            self.flush();
            self.pending += 1;
        }
        fn stroke(&mut self) {
            self.finish(false);
        }
        fn fill(&mut self) {
            self.finish(true);
        }
        fn stroke_feathered(&mut self, _feather: f32) {
            self.finish(false);
        }
        fn draw_text(
            &mut self,
            _fonts: &mut Fonts,
            _style: TextStyle,
            _x: f32,
            _y: f32,
            _s: &str,
            _color: Color,
        ) {
            self.fills += 1;
        }
    }

    #[test]
    fn a_stroke_that_merges_into_one_point_draws_nothing() {
        let mut vg = CountingDevice::new();
        vg.begin_path();
        vg.move_to(100.0, 100.0);
        vg.line_to(100.001, 100.0);
        vg.stroke_width(1.5);
        vg.stroke();
        assert_eq!(vg.strokes, 1);
        assert_eq!(vg.paths, 0);
        vg.begin_path();
        vg.move_to(100.0, 100.0);
        vg.line_to(100.001, 100.0);
        vg.move_to(200.0, 100.0);
        vg.line_to(210.0, 100.0);
        vg.stroke();
        assert_eq!(vg.paths, 1);
    }

    /// Builds a painter over a test backend, the zero theme and spare fonts.
    fn painter<'a>(
        vg: &'a mut CountingDevice,
        fonts: &'a mut Fonts,
        theme: &'a Theme,
    ) -> Painter<'a> {
        Painter {
            vg,
            fonts,
            theme,
            density: 2.0,
        }
    }

    #[test]
    // The coordinates stay verbatim from the spec whatever clippy thinks
    // of their digits.
    #[allow(clippy::excessive_precision, clippy::unreadable_literal)]
    fn the_strokes_that_trapped_in_m10s_review_leave_their_slivers_out() {
        use super::super::scene::{Curve, CurveFn};
        use super::super::viewport::Viewport;

        let mut vg = CountingDevice::new();
        let mut fonts = mv_text::uninit();
        assert!(mv_text::init(&mut fonts));
        let theme = Theme {
            slug: Slug::ArcticIce,
            space: Space::DisplayP3,
            colors: EnumMap::from_array([[0.0f32; 4]; Token::LENGTH]),
            radius: 0.0,
            card_radius: 0.0,
            card_shadow: None,
            glow_radius: 0.0,
            hud_ticks: 0.0,
            label_tracking: 0.0,
            readout_font: ReadoutFont::RobotoMono,
        };
        let white: Color = [1.0, 1.0, 1.0, 1.0];
        // The unit box's dashed sides during Reset all's jump: 60.5002 px at
        // 5 4 from half a dash in (the last dash 0.0002 px), and 51.5048 px
        // (0.0048 px).
        paint_line(
            painter(&mut vg, &mut fonts, &theme),
            &Segment {
                x1: 512.7498907323851,
                y1: 385.49978146477025,
                x2: 512.7498907323851,
                y2: 324.9995629295405,
            },
            1.5,
            white,
            Some([5.0, 4.0]),
            false,
        );
        paint_line(
            painter(&mut vg, &mut fonts, &theme),
            &Segment {
                x1: 464.99040607063381,
                y1: 392.99040607063381,
                x2: 516.49520303531688,
                y2: 392.99040607063381,
            },
            1.5,
            white,
            Some([5.0, 4.0]),
            false,
        );
        // k's shift arrow 0.0039 px longer than its head takes, as term-drag-k's
        // spring brought it back.
        paint_line(
            painter(&mut vg, &mut fonts, &theme),
            &Segment {
                x1: 468.0,
                y1: 396.0,
                x2: 468.0,
                y2: 393.49609540537239,
            },
            2.5,
            white,
            None,
            true,
        );
        // sin x drawing in in R2's plane at the switch's frames that trapped:
        // its first 0.0045 and 0.0011 px.
        let vp = Viewport {
            width: 936.0,
            height: 792.0,
            zoom: 50.0,
            origin_x: 468.0,
            origin_y: 396.0,
        };
        let strokes = vg.strokes;
        for share in [3.9061201148734744e-06, 9.7527220566048779e-07] {
            paint_curve(
                painter(&mut vg, &mut fonts, &theme),
                vp,
                &Curve {
                    id: String::new(),
                    func: CurveFn {
                        f: Box::new(|x: f64| x.sin()),
                    },
                    ink: Token::ChartLine,
                    width: 2.5,
                    glow: false,
                    dash: None,
                    alpha: None,
                    avoid: false,
                    back: false,
                    draw_to: Some(share),
                },
            );
        }
        assert_eq!(vg.strokes, strokes);
        paint_curve(
            painter(&mut vg, &mut fonts, &theme),
            vp,
            &Curve {
                id: String::new(),
                func: CurveFn {
                    f: Box::new(|x: f64| x.sin()),
                },
                ink: Token::ChartLine,
                width: 2.5,
                glow: false,
                dash: None,
                alpha: None,
                avoid: false,
                back: false,
                draw_to: Some(0.5),
            },
        );
        assert_eq!(vg.strokes, strokes + 1);
        mv_text::destroy(&mut fonts);
    }
}
