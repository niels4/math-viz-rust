//! The plane's scene: the owner-agnostic model of curves, points, guides,
//! handles and annotations the painters draw.

use enum_map::Enum;

/// A drawing ink: a theme token resolved at paint time.
pub type Ink = mv_theme::Token;

/// A curve over math x: a closure the trace samples.
pub struct CurveFn {
    pub f: Box<dyn Fn(f64) -> f64>,
}

impl std::fmt::Debug for CurveFn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CurveFn(..)")
    }
}

/// Evaluates a curve at x.
pub fn curve_at(f: &CurveFn, x: f64) -> f64 {
    (f.f)(x)
}

/// One plotted curve.
#[derive(Debug)]
pub struct Curve {
    pub id: String,
    pub func: CurveFn,
    pub ink: Ink,
    pub width: f64,
    pub glow: bool,
    pub dash: Option<[f64; 2]>,
    pub alpha: Option<f64>,
    pub avoid: bool,
    pub back: bool,
    pub draw_to: Option<f64>,
}

/// How a point marks itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointStyle {
    Bullseye,
    Ring,
}

/// Where a point's label sits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LabelPlace {
    Clear,
    Fixed,
}

/// A past position a point left behind.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Was {
    pub x: f64,
    pub y: f64,
    pub ink: Ink,
}

/// A pinned or followed point on a curve.
#[derive(Debug)]
pub struct Point {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub style: PointStyle,
    pub ink: Ink,
    pub name: String,
    pub label_place: LabelPlace,
    pub focus: bool,
    pub drop_lines: bool,
    pub edge_marker: bool,
    pub draggable: bool,
    pub was: Option<Was>,
    pub alpha: Option<f64>,
    pub label_alpha: Option<f64>,
    pub label_rise: f64,
    pub reach: Option<f64>,
}

/// A vertical guide at a math x.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Guide {
    pub x: f64,
    pub alpha: Option<f64>,
}

/// How a handle draws itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum HandleShape {
    Diamond,
    Square,
}

/// A drag handle on the curve.
#[derive(Debug)]
pub struct Handle {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub shape: HandleShape,
    pub ink: Ink,
    pub halo: bool,
    pub held: bool,
    pub alpha: Option<f64>,
}

/// One annotation line: a dimension, a drop or an arrow.
#[derive(Clone, Debug, PartialEq)]
pub struct AnnotationLine {
    pub from: [f64; 2],
    pub to: [f64; 2],
    pub width: f64,
    pub dash: Option<[f64; 2]>,
    pub alpha: Option<f64>,
    pub start_tick: Option<f64>,
    pub end_ticks: Option<f64>,
    pub arrow: bool,
}

/// Whether an annotation draws under the curves or over them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnnotationLayer {
    Under,
    Over,
}

/// A group of annotation lines in one ink and layer.
#[derive(Debug)]
pub struct Annotation {
    pub layer: AnnotationLayer,
    pub ink: Ink,
    pub lines: Vec<AnnotationLine>,
}

/// Everything on the plane: the curves and their marks.
#[derive(Debug)]
pub struct Scene {
    pub grid_alpha: Option<f64>,
    pub curves: Vec<Curve>,
    pub points: Vec<Point>,
    pub guides: Vec<Guide>,
    pub handles: Vec<Handle>,
    pub annotations: Vec<Annotation>,
}
