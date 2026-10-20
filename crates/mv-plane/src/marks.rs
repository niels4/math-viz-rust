//! The marks over the plane: drop lines, markers, annotations, handles,
//! guides, and their hit boxes.

use enum_map::{Enum, EnumMap};
use mv_numfmt::{DISPLAY_DP, format_number};
use mv_theme::Token;

use super::faces::{EDGE_LETTER_FACE, Face, LABEL_LETTER_FACE, Measure, ReadoutFaces};
use super::placement::{
    Direction, PlaceOptions, Polyline, curve_polylines, edge_direction, edge_marker_box,
    place_beside, place_fixed,
};
use super::plates::{
    EDGE_STYLE, LABEL_STYLE, Plate, PlateRun, PlateRunWhat, TextRun, TriangleRun, at, layout_plate,
};
use super::rect::{Rect, centred, contains, inflate};
use super::scene::{AnnotationLayer, HandleShape, Ink, LabelPlace, PointStyle, Scene};
use super::viewport::{Viewport, to_screen_x, to_screen_y};

/// A drop line from a point to an axis, in an ink.
#[derive(Clone, Debug, PartialEq)]
pub struct DropLine {
    pub ink: Ink,
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
    pub alpha: Option<f64>,
}

/// A point marker on the curve.
#[derive(Clone, Debug, PartialEq)]
pub struct Marker {
    pub x: f64,
    pub y: f64,
    pub style: PointStyle,
    pub ink: Ink,
    pub focus: bool,
    pub alpha: Option<f64>,
}

/// A segment: a straight line between two points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Segment {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

/// A past position's mark: where the point was, and its arrow.
#[derive(Clone, Debug, PartialEq)]
pub struct WasMark {
    pub x: f64,
    pub y: f64,
    pub ink: Ink,
    pub arrow: Option<WasArrow>,
}

/// The arrow of a past position: a segment in an ink.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WasArrow {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
    pub ink: Ink,
}

/// One point's layer: its marker, label, edge and past position.
#[derive(Clone, Debug, PartialEq)]
pub struct PointLayer {
    pub id: String,
    pub was: Option<WasMark>,
    pub marker: Option<Marker>,
    pub label: Option<Plate>,
    pub edge: Option<Plate>,
    pub edge_dir: Option<Direction>,
}

/// An annotation stroke: a segment drawn wide, dashed or arrowed.
#[derive(Clone, Debug, PartialEq)]
pub struct AnnotationStroke {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
    pub width: f64,
    pub dash: Option<[f64; 2]>,
    pub alpha: Option<f64>,
    pub start_tick: Option<f64>,
    pub end_ticks: Option<f64>,
    pub arrow: bool,
}

/// An annotation's marks: its strokes in a layer and ink.
#[derive(Clone, Debug, PartialEq)]
pub struct AnnotationMarks {
    pub layer: AnnotationLayer,
    pub ink: Ink,
    pub strokes: Vec<AnnotationStroke>,
}

/// A handle's mark: where it sits and how it shows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HandleMark {
    pub x: f64,
    pub y: f64,
    pub shape: HandleShape,
    pub ink: Ink,
    pub halo: bool,
    pub held: bool,
    pub alpha: Option<f64>,
}

/// A guide's mark: its x and alpha.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GuideMark {
    pub x: f64,
    pub alpha: Option<f64>,
}

/// What a hit box hits: a point, a handle or an edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum HitKind {
    Point,
    Handle,
    Edge,
}

/// One hit box: what it hits, its id, box and target.
#[derive(Clone, Debug, PartialEq)]
pub struct Hit {
    pub kind: HitKind,
    pub id: String,
    pub r#box: Rect,
    pub target: [f64; 2],
}

/// The laid-out marks of a scene.
#[derive(Clone, Debug, PartialEq)]
pub struct MarksLayout {
    pub annotations: Vec<AnnotationMarks>,
    pub handles: Vec<HandleMark>,
    pub guides: Vec<GuideMark>,
    pub drop_lines: Vec<DropLine>,
    pub points: Vec<PointLayer>,
    pub hits: Vec<Hit>,
}

/// What laying out needs: the measure, the readout faces, and the boxes to
/// keep out of.
pub struct MarksOptions<'a> {
    pub measure: Measure<'a>,
    pub readout: ReadoutFaces,
    pub plates: Vec<Rect>,
    pub corners: Vec<Rect>,
}

/// A point hit box, in px.
const HIT_BOX: f64 = 48.0;

/// A handle hit box, in px.
const HANDLE_HIT: f64 = 44.0;

/// A marker box, in px.
const MARK_BOX: f64 = 28.0;

/// The gap a label keeps from its point, in px.
const LABEL_GAP: f64 = 22.0;

/// The clearance marks keep from the chrome, in px.
const CHROME_CLEAR: f64 = 8.0;

/// A past position's trail starts here, in px behind.
const WAS_START: f64 = 13.0;

/// A past position's arrow tip, in px.
const WAS_TIP: f64 = 14.0;

/// The shortest trail that draws an arrow, in px.
const WAS_ARROW_MIN: f64 = 26.0;

/// The clearance a trail keeps, in px.
const WAS_CLEAR: f64 = 14.0;

/// The clearance a handle keeps, in px, by shape.
static HANDLE_CLEAR: EnumMap<HandleShape, f64> = EnumMap::from_array([28.0, 24.0]);

/// The coordinates of a point as a label.
pub fn coords(x: f64, y: f64) -> String {
    format!(
        "({}, {})",
        format_number(x, DISPLAY_DP),
        format_number(y, DISPLAY_DP)
    )
}

/// An alpha that hides the plate when opaque.
fn faded(alpha: f64) -> Option<f64> {
    if alpha < 1.0 {
        Some(alpha.max(0.0))
    } else {
        None
    }
}

/// Slides a plate by an arrival.
fn moved(plate: &Plate, alpha: f64, dy: f64) -> Plate {
    if alpha >= 1.0 && dy == 0.0 {
        return plate.clone();
    }
    let mut out = plate.clone();
    out.motion = Some(super::plates::PlateMotion {
        alpha: alpha.max(0.0),
        dx: 0.0,
        dy,
    });
    out
}

/// Whether a value falls within a range.
fn within(v: f64, lo: f64, hi: f64) -> bool {
    v >= lo && v <= hi
}

/// The box spanning two points by a margin.
fn spanning(a: [f64; 2], b: [f64; 2], by: f64) -> Rect {
    inflate(
        Rect {
            x: a[0].min(b[0]),
            y: a[1].min(b[1]),
            w: (a[0] - b[0]).abs(),
            h: (a[1] - b[1]).abs(),
        },
        by,
    )
}

/// Lays out the marks of a scene in a viewport.
pub fn layout_marks(scene: &Scene, vp: Viewport, opts: &mut MarksOptions<'_>) -> MarksLayout {
    let width = vp.width;
    let height = vp.height;
    if width == 0.0 || height == 0.0 {
        return MarksLayout {
            annotations: Vec::new(),
            handles: Vec::new(),
            guides: Vec::new(),
            drop_lines: Vec::new(),
            points: Vec::new(),
            hits: Vec::new(),
        };
    }
    let m = &mut opts.measure;
    let readout = opts.readout;
    let run_text = |s: String, face: Face, ink: Ink| PlateRun {
        what: PlateRunWhat::Text(TextRun { text: s, face }),
        ink,
        x: 0.0,
        y: 0.0,
    };

    // Each point on screen, and the edge it lies past (none in view).
    struct Located<'a> {
        p: &'a super::scene::Point,
        sx: f64,
        sy: f64,
        off: Option<Direction>,
    }
    let mut located: Vec<Located> = Vec::with_capacity(scene.points.len());
    for p in &scene.points {
        if p.x.is_nan() || p.x.is_infinite() || p.y.is_nan() {
            continue;
        }
        let sx = to_screen_x(vp, p.x);
        let sy = to_screen_y(vp, p.y);
        located.push(Located {
            p,
            sx,
            sy,
            off: edge_direction(sx, sy, width, height),
        });
    }

    let mut guides: Vec<GuideMark> = Vec::new();
    for g in &scene.guides {
        let x = to_screen_x(vp, g.x);
        if within(x, 0.0, width) {
            guides.push(GuideMark {
                x,
                alpha: faded(g.alpha.unwrap_or(1.0)),
            });
        }
    }
    let mut drop_lines: Vec<DropLine> = Vec::new();
    let mut edges: Vec<Rect> = Vec::new(); // the edge markers so far, for the next to keep clear
    let mut edge_plates: Vec<Option<Plate>> = vec![None; located.len()];
    let mut hits: Vec<Hit> = Vec::new();

    for (i, l) in located.iter().enumerate() {
        let p = l.p;
        let x_in = within(l.sx, 0.0, width);
        let y_in = within(l.sy, 0.0, height);
        if p.drop_lines {
            // In view: drop lines to both axes. Off the view one way, only to the axis it still
            // crosses: the line comes in from the edge. Mid-motion the lines reach part way.
            let reach = 1.0f64.min(0.0f64.max(p.reach.unwrap_or(1.0)));
            let alpha = faded(p.alpha.unwrap_or(1.0));
            if x_in {
                let y1 = if y_in {
                    l.sy
                } else if l.sy < 0.0 {
                    0.0
                } else {
                    height
                };
                drop_lines.push(DropLine {
                    ink: p.ink,
                    x1: l.sx,
                    y1,
                    x2: l.sx,
                    y2: y1 + (vp.origin_y - y1) * reach,
                    alpha,
                });
            }
            if y_in {
                let x1 = if x_in {
                    l.sx
                } else if l.sx < 0.0 {
                    0.0
                } else {
                    width
                };
                drop_lines.push(DropLine {
                    ink: p.ink,
                    x1,
                    y1: l.sy,
                    x2: x1 + (vp.origin_x - x1) * reach,
                    y2: l.sy,
                    alpha,
                });
            }
        }
        if let Some(off) = l.off
            && p.edge_marker
        {
            let mut runs = Vec::with_capacity(3);
            runs.push(PlateRun {
                what: PlateRunWhat::Triangle(TriangleRun { dir: off }),
                ink: p.ink,
                x: 0.0,
                y: 0.0,
            });
            if !p.name.is_empty() {
                runs.push(run_text(
                    p.name.clone(),
                    EDGE_LETTER_FACE,
                    Token::Foreground,
                ));
            }
            runs.push(run_text(
                coords(p.x, p.y),
                readout.small,
                Token::ForegroundMuted,
            ));
            let marker = layout_plate(&runs, EDGE_STYLE, p.ink, &mut *m);
            // Clear of the chrome's plates and of the edge markers before it.
            let mut taken: Vec<Rect> = Vec::with_capacity(opts.plates.len() + edges.len());
            taken.extend_from_slice(&opts.plates);
            taken.extend_from_slice(&edges);
            let b = edge_marker_box(
                off,
                [l.sx, l.sy],
                [marker.r#box.w, marker.r#box.h],
                width,
                height,
                &taken,
            );
            let placed = moved(&at(&marker, b.x, b.y), p.alpha.unwrap_or(1.0), 0.0);
            edges.push(placed.r#box);
            hits.push(Hit {
                kind: HitKind::Edge,
                id: p.id.clone(),
                r#box: placed.r#box,
                target: [p.x, p.y],
            });
            edge_plates[i] = Some(placed);
        }
    }

    struct LocatedHandle<'a> {
        h: &'a super::scene::Handle,
        x: f64,
        y: f64,
    }
    let mut handles: Vec<LocatedHandle> = Vec::with_capacity(scene.handles.len());
    for h in &scene.handles {
        if h.x.is_nan() || h.x.is_infinite() || h.y.is_nan() || h.y.is_infinite() {
            continue;
        }
        handles.push(LocatedHandle {
            h,
            x: to_screen_x(vp, h.x),
            y: to_screen_y(vp, h.y),
        });
    }
    let mut marker_boxes: Vec<Rect> = Vec::new();
    for l in &located {
        if l.off.is_none() {
            marker_boxes.push(centred(l.sx, l.sy, MARK_BOX));
        }
    }

    // Where each point was, and the room its ring and arrow take. Off the view there is nothing
    // to mark: the owner's readout says how far it went.
    let mut was: Vec<Option<WasMark>> = vec![None; located.len()];
    for (i, l) in located.iter().enumerate() {
        let Some(w) = l.p.was else { continue };
        if w.x.is_nan() || w.x.is_infinite() || w.y.is_nan() || w.y.is_infinite() {
            continue;
        }
        let wx = to_screen_x(vp, w.x);
        let wy = to_screen_y(vp, w.y);
        if edge_direction(wx, wy, width, height).is_some() {
            continue;
        }
        let dist = if l.off.is_none() {
            (l.sx - wx).hypot(l.sy - wy)
        } else {
            0.0
        };
        let ux = if dist == 0.0 { 0.0 } else { (l.sx - wx) / dist };
        let uy = if dist == 0.0 { 0.0 } else { (l.sy - wy) / dist };
        let mut mark = WasMark {
            x: wx,
            y: wy,
            ink: l.p.ink,
            arrow: None,
        };
        if dist > WAS_ARROW_MIN {
            mark.arrow = Some(WasArrow {
                x1: wx + ux * WAS_START,
                y1: wy + uy * WAS_START,
                x2: l.sx - ux * WAS_TIP,
                y2: l.sy - uy * WAS_TIP,
                ink: w.ink,
            });
        }
        was[i] = Some(mark);
    }

    let mut annotations: Vec<AnnotationMarks> = Vec::with_capacity(scene.annotations.len());
    for a in &scene.annotations {
        let mut strokes = Vec::with_capacity(a.lines.len());
        for line in &a.lines {
            strokes.push(AnnotationStroke {
                x1: to_screen_x(vp, line.from[0]),
                y1: to_screen_y(vp, line.from[1]),
                x2: to_screen_x(vp, line.to[0]),
                y2: to_screen_y(vp, line.to[1]),
                width: line.width,
                dash: line.dash,
                alpha: line.alpha,
                start_tick: line.start_tick,
                end_ticks: line.end_ticks,
                arrow: line.arrow,
            });
        }
        annotations.push(AnnotationMarks {
            layer: a.layer,
            ink: a.ink,
            strokes,
        });
    }

    // A label placed clear keeps clear of the chrome, the edge markers, the handles, where points
    // were, the curves that ask, every marker and the labels before it; a label held in place keeps
    // only off the chrome. Either way the labels after it keep clear of it.
    let mut chrome: Vec<Rect> = Vec::with_capacity(opts.plates.len());
    for r in &opts.plates {
        chrome.push(inflate(*r, CHROME_CLEAR));
    }
    let mut obstacles: Vec<Rect> = Vec::new();
    obstacles.extend_from_slice(&chrome);
    for r in &opts.corners {
        obstacles.push(inflate(*r, CHROME_CLEAR));
    }
    obstacles.extend_from_slice(&edges);
    for h in &handles {
        obstacles.push(centred(h.x, h.y, HANDLE_CLEAR[h.h.shape]));
    }
    for (i, l) in located.iter().enumerate() {
        if let Some(w) = &was[i] {
            let to = if w.arrow.is_none() {
                [w.x, w.y]
            } else {
                [l.sx, l.sy]
            };
            obstacles.push(spanning([w.x, w.y], to, WAS_CLEAR));
        }
    }
    let mut avoided: Vec<Polyline> = Vec::new();
    for c in &scene.curves {
        if c.avoid {
            avoided.extend(curve_polylines(vp, &c.func));
        }
    }

    let mut layers: Vec<PointLayer> = Vec::with_capacity(located.len());
    for (i, l) in located.iter().enumerate() {
        let p = l.p;
        let mut layer = PointLayer {
            id: p.id.clone(),
            was: was[i].clone(),
            marker: None,
            label: None,
            edge: None,
            edge_dir: None,
        };
        if l.off.is_none() {
            if !p.name.is_empty() {
                let content = layout_plate(
                    &[
                        run_text(p.name.clone(), LABEL_LETTER_FACE, Token::Foreground),
                        run_text(coords(p.x, p.y), readout.label, Token::Foreground),
                    ],
                    LABEL_STYLE,
                    p.ink,
                    &mut *m,
                );
                let (w, h) = (content.r#box.w, content.r#box.h);
                let b = if p.label_place == LabelPlace::Fixed {
                    place_fixed(l.sx, l.sy, w, h, LABEL_GAP, &chrome, width, height)
                } else {
                    let mut clear: Vec<Rect> =
                        Vec::with_capacity(obstacles.len() + marker_boxes.len());
                    clear.extend_from_slice(&obstacles);
                    clear.extend_from_slice(&marker_boxes);
                    place_beside(
                        l.sx,
                        l.sy,
                        w,
                        h,
                        &PlaceOptions {
                            gap: LABEL_GAP,
                            curves: avoided.clone(),
                            obstacles: clear,
                            width,
                            height,
                        },
                    )
                };
                let alpha = p.alpha.unwrap_or(1.0) * p.label_alpha.unwrap_or(1.0);
                layer.label = Some(moved(&at(&content, b.x, b.y), alpha, p.label_rise));
                obstacles.push(b);
            }
            layer.marker = Some(Marker {
                x: l.sx,
                y: l.sy,
                style: p.style,
                ink: p.ink,
                focus: p.focus,
                alpha: faded(p.alpha.unwrap_or(1.0)),
            });
        }
        layer.edge = edge_plates[i].clone();
        if layer.edge.is_some() {
            layer.edge_dir = l.off;
        }
        layers.push(layer);
    }

    // The topmost mark takes the pointer: points over handles, the last painted first.
    for l in located.iter().rev() {
        if l.off.is_none() && l.p.draggable {
            hits.push(Hit {
                kind: HitKind::Point,
                id: l.p.id.clone(),
                r#box: centred(l.sx, l.sy, HIT_BOX),
                target: [0.0, 0.0],
            });
        }
    }
    for h in handles.iter().rev() {
        hits.push(Hit {
            kind: HitKind::Handle,
            id: h.h.id.clone(),
            r#box: centred(h.x, h.y, HANDLE_HIT),
            target: [0.0, 0.0],
        });
    }

    let mut handle_marks = Vec::with_capacity(handles.len());
    for h in &handles {
        handle_marks.push(HandleMark {
            x: h.x,
            y: h.y,
            shape: h.h.shape,
            ink: h.h.ink,
            halo: h.h.halo,
            held: h.h.held,
            alpha: faded(h.h.alpha.unwrap_or(1.0)),
        });
    }
    MarksLayout {
        annotations,
        handles: handle_marks,
        guides,
        drop_lines,
        points: layers,
        hits,
    }
}

/// Hit-tests a point against the hit boxes, topmost first.
pub fn hit_test(hits: &[Hit], px: f64, py: f64) -> (Hit, bool) {
    for hit in hits {
        if contains(hit.r#box, px, py) {
            return (hit.clone(), true);
        }
    }
    (
        Hit {
            kind: HitKind::Point,
            id: String::new(),
            r#box: Rect {
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: 0.0,
            },
            target: [0.0, 0.0],
        },
        false,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::faces::{TICK_LABEL_FACE, js_round, readout_faces};
    use crate::plates::PlateMotion;
    use crate::rect::intersects;
    use crate::scene::{Annotation, AnnotationLine, Curve, CurveFn, Guide, Handle, Point, Was};
    use mv_text::Face as TextFace;
    use mv_theme::ReadoutFont;

    fn board_measure(face: Face, s: &str) -> f64 {
        if face.face == TextFace::Stix400Italic {
            let mut w = 0.0;
            for r in s.chars() {
                w += (if r == 'Q' { 0.72 } else { 0.57 }) * face.size;
            }
            return w;
        }
        s.chars().count() as f64 * 0.6001 * face.size
    }

    fn plates(tools_x: f64) -> Vec<Rect> {
        vec![
            Rect {
                x: 28.0,
                y: 22.0,
                w: 186.0,
                h: 35.0,
            },
            Rect {
                x: 28.0,
                y: 735.0,
                w: 112.0,
                h: 35.0,
            },
            Rect {
                x: tools_x,
                y: 734.0,
                w: 908.0 - tools_x,
                h: 36.0,
            },
        ]
    }

    static CORNERS: [Rect; 4] = [
        Rect {
            x: 10.0,
            y: 10.0,
            w: 28.0,
            h: 28.0,
        },
        Rect {
            x: 10.0,
            y: 754.0,
            w: 28.0,
            h: 28.0,
        },
        Rect {
            x: 898.0,
            y: 10.0,
            w: 28.0,
            h: 28.0,
        },
        Rect {
            x: 898.0,
            y: 754.0,
            w: 28.0,
            h: 28.0,
        },
    ];

    static R2_VP: Viewport = Viewport {
        width: 936.0,
        height: 792.0,
        zoom: 50.0,
        origin_x: 468.0,
        origin_y: 396.0,
    };

    fn p(x: f64, y: f64) -> Point {
        Point {
            id: "p".to_string(),
            x,
            y,
            style: PointStyle::Bullseye,
            ink: Token::ChartPoint1,
            name: "P".to_string(),
            label_place: LabelPlace::Clear,
            focus: false,
            drop_lines: false,
            edge_marker: true,
            draggable: true,
            was: None,
            alpha: None,
            label_alpha: None,
            label_rise: 0.0,
            reach: None,
        }
    }

    fn q(x: f64, y: f64) -> Point {
        Point {
            id: "q".to_string(),
            x,
            y,
            style: PointStyle::Ring,
            ink: Token::ChartPoint2,
            name: "Q".to_string(),
            label_place: LabelPlace::Clear,
            focus: false,
            drop_lines: true,
            edge_marker: true,
            draggable: false,
            was: None,
            alpha: None,
            label_alpha: None,
            label_rise: 0.0,
            reach: None,
        }
    }

    fn x2(x: f64) -> f64 {
        x * x
    }

    fn r3(x: f64) -> f64 {
        2.0 * (x + 1.0) * (x + 1.0) + 1.0
    }

    fn r6(x: f64) -> f64 {
        2.0 * (x - 1.5) * (x - 1.5) + 1.0
    }

    fn cube(x: f64) -> f64 {
        x * x * x
    }

    fn ghost_fn(x: f64) -> f64 {
        x * x
    }

    fn curve(f: fn(f64) -> f64) -> Curve {
        Curve {
            id: "f".to_string(),
            func: CurveFn { f: Box::new(f) },
            ink: Token::ChartLine,
            width: 3.5,
            glow: false,
            dash: None,
            alpha: None,
            avoid: true,
            back: false,
            draw_to: None,
        }
    }

    fn ghost() -> Curve {
        Curve {
            id: "original".to_string(),
            func: CurveFn {
                f: Box::new(ghost_fn),
            },
            ink: Token::ForegroundMuted,
            width: 2.0,
            glow: false,
            dash: Some([6.0, 6.0]),
            alpha: Some(0.75),
            avoid: false,
            back: true,
            draw_to: None,
        }
    }

    fn scene(f: fn(f64) -> f64, points: Vec<Point>, guide_x: Option<f64>) -> Scene {
        let guides = match guide_x {
            Some(x) => vec![Guide { x, alpha: None }],
            None => Vec::new(),
        };
        Scene {
            grid_alpha: None,
            curves: vec![curve(f)],
            points,
            guides,
            handles: Vec::new(),
            annotations: Vec::new(),
        }
    }

    fn layout(s: Scene, tools_x: f64) -> MarksLayout {
        let corners = CORNERS;
        layout_marks(
            &s,
            R2_VP,
            &mut MarksOptions {
                measure: Measure {
                    f: Box::new(board_measure),
                },
                readout: readout_faces(ReadoutFont::RobotoMono),
                plates: plates(tools_x),
                corners: corners.to_vec(),
            },
        )
    }

    fn no_plate() -> Plate {
        Plate {
            r#box: Rect {
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: 0.0,
            },
            radius: 0.0,
            fill: Token::Card,
            border_ink: Token::Card,
            border_width: 0.0,
            runs: Vec::new(),
            motion: None,
        }
    }

    // A plate's box with x and y to a tenth, as the React test rounds them.
    fn box_of(plate: Option<Plate>) -> Rect {
        let b = plate.unwrap_or(no_plate()).r#box;
        Rect {
            x: js_round(b.x * 10.0) / 10.0,
            y: js_round(b.y * 10.0) / 10.0,
            w: b.w,
            h: b.h,
        }
    }

    fn run_texts(plate: Option<Plate>) -> Vec<String> {
        plate
            .unwrap_or(no_plate())
            .runs
            .iter()
            .map(|run| match &run.what {
                PlateRunWhat::Text(t) => t.text.clone(),
                PlateRunWhat::Triangle(t) => DIRECTION_WORDS[t.dir].to_string(),
            })
            .collect()
    }

    static DIRECTION_WORDS: EnumMap<Direction, &str> =
        EnumMap::from_array(["up", "down", "left", "right"]);

    fn r3_handles() -> Vec<Handle> {
        vec![
            Handle {
                id: "anchor".to_string(),
                x: -1.0,
                y: 1.0,
                shape: HandleShape::Diamond,
                ink: Token::Primary,
                halo: false,
                held: false,
                alpha: None,
            },
            Handle {
                id: "stretch".to_string(),
                x: 0.0,
                y: 3.0,
                shape: HandleShape::Square,
                ink: Token::Primary,
                halo: false,
                held: false,
                alpha: None,
            },
        ]
    }

    #[test]
    fn marks_lay_out_r2_at_the_snapshots_places() {
        // The boxes are as wide as fixed decimals make them: "(2.00, 4.00)" where
        // the board prints "(2, 4)". A label left of its point keeps the board's right edge.
        let marks = layout(
            scene(x2, vec![p(2.0, 4.0), q(-1.5, 2.25)], Some(-1.5)),
            769.0,
        );
        let pt = &marks.points[0];
        let qt = &marks.points[1];
        assert_eq!(
            box_of(pt.label.clone()),
            Rect {
                x: 590.0,
                y: 141.8,
                w: 169.0,
                h: 41.0,
            }
        );
        assert_eq!(
            box_of(qt.label.clone()),
            Rect {
                x: 189.0,
                y: 296.7,
                w: 182.0,
                h: 41.0,
            }
        );
        assert_eq!(marks.guides.len(), 1);
        assert!(marks.guides[0].x == 393.0 && marks.guides[0].alpha.is_none());
        // Drop lines from Q to both axes, in Q's ink.
        assert_eq!(marks.drop_lines.len(), 2);
        assert_eq!(
            marks.drop_lines[0],
            DropLine {
                ink: Token::ChartPoint2,
                x1: 393.0,
                y1: 283.5,
                x2: 393.0,
                y2: 396.0,
                alpha: None,
            }
        );
        assert_eq!(
            marks.drop_lines[1],
            DropLine {
                ink: Token::ChartPoint2,
                x1: 393.0,
                y1: 283.5,
                x2: 468.0,
                y2: 283.5,
                alpha: None,
            }
        );
        assert_eq!(
            pt.marker.clone().unwrap(),
            Marker {
                x: 568.0,
                y: 196.0,
                style: PointStyle::Bullseye,
                ink: Token::ChartPoint1,
                focus: false,
                alpha: None,
            }
        );
    }

    #[test]
    fn marks_print_coordinates_by_the_number_rule() {
        let marks = layout(
            scene(x2, vec![p(2.0, 4.0), q(-1.5, 2.25)], Some(-1.5)),
            769.0,
        );
        let pt = run_texts(marks.points[0].label.clone());
        let qt = run_texts(marks.points[1].label.clone());
        assert!(pt.len() == 2 && pt[0] == "P" && pt[1] == "(2.00, 4.00)");
        assert!(qt.len() == 2 && qt[0] == "Q" && qt[1] == "(−1.50, 2.25)");
    }

    #[test]
    fn marks_lay_out_r3_and_r4() {
        // P's label clears the curve; Q's label goes up-left.
        assert_eq!(
            box_of(
                layout(scene(r3, vec![p(0.5, 5.5)], None), 597.0).points[0]
                    .label
                    .clone()
            ),
            Rect {
                x: 515.0,
                y: 66.8,
                w: 169.0,
                h: 41.0,
            }
        );
        let r4 = layout(
            scene(r3, vec![p(0.5, 5.5), q(-2.0, 3.0)], Some(-2.0)),
            597.0,
        );
        assert_eq!(
            box_of(r4.points[1].label.clone()),
            Rect {
                x: 164.0,
                y: 191.8,
                w: 182.0,
                h: 41.0,
            }
        );
    }

    #[test]
    fn a_focused_point_lights_and_drops_lines() {
        let mut pp = p(2.0, 4.0);
        pp.focus = true;
        pp.drop_lines = true;
        let marks = layout(scene(x2, vec![pp], None), 769.0);
        assert!(marks.points[0].marker.as_ref().unwrap().focus);
        assert_eq!(marks.drop_lines.len(), 2);
        assert_eq!(
            marks.drop_lines[0],
            DropLine {
                ink: Token::ChartPoint1,
                x1: 568.0,
                y1: 196.0,
                x2: 568.0,
                y2: 396.0,
                alpha: None,
            }
        );
        assert_eq!(
            marks.drop_lines[1],
            DropLine {
                ink: Token::ChartPoint1,
                x1: 568.0,
                y1: 196.0,
                x2: 468.0,
                y2: 196.0,
                alpha: None,
            }
        );
    }

    #[test]
    fn a_point_off_the_view_is_marked_on_its_edge() {
        // x³ at P = 2 is 8, above y = 7.92.
        let marks = layout(scene(cube, vec![p(2.0, 8.0)], None), 769.0);
        let pt = &marks.points[0];
        assert!(pt.marker.is_none() && pt.label.is_none());
        let texts = run_texts(pt.edge.clone());
        assert!(
            texts.len() == 3 && texts[0] == "up" && texts[1] == "P" && texts[2] == "(2.00, 8.00)"
        );
        // "▲ P (2.00, 8.00)": runs of 12, 11 and 109 px, 8 apart, inside 2 + 8 and 12 + 2:
        // 172 wide, centred on P's x at the top.
        assert_eq!(
            box_of(pt.edge.clone()),
            Rect {
                x: 482.0,
                y: 22.0,
                w: 172.0,
                h: 39.0,
            }
        );
        let mut found = false;
        for hit in &marks.hits {
            if hit.kind == HitKind::Edge && hit.id == "p" {
                found = hit.r#box == pt.edge.clone().unwrap().r#box && hit.target == [2.0, 8.0];
            }
        }
        assert!(found);
    }

    #[test]
    fn drop_lines_go_only_to_the_axis_an_off_view_point_still_crosses() {
        let marks = layout(
            scene(cube, vec![q(1.8, 5.832), q(2.5, 15.625)], Some(1.8)),
            769.0,
        );
        // The second Q is above the view: only a line from the top edge to the x-axis.
        assert_eq!(marks.drop_lines.len(), 3);
        assert_eq!(
            marks.drop_lines[2],
            DropLine {
                ink: Token::ChartPoint2,
                x1: 593.0,
                y1: 0.0,
                x2: 593.0,
                y2: 396.0,
                alpha: None,
            }
        );
    }

    #[test]
    fn a_draggable_point_in_view_has_its_48_px_hit_box() {
        let marks = layout(
            scene(x2, vec![p(2.0, 4.0), q(-1.5, 2.25)], Some(-1.5)),
            769.0,
        );
        assert_eq!(marks.hits.len(), 1);
        assert_eq!(
            marks.hits[0],
            Hit {
                kind: HitKind::Point,
                id: "p".to_string(),
                r#box: Rect {
                    x: 544.0,
                    y: 172.0,
                    w: 48.0,
                    h: 48.0,
                },
                target: [0.0, 0.0],
            }
        );
    }

    #[test]
    fn nothing_is_laid_out_before_the_plane_has_a_size() {
        let s = scene(x2, vec![p(2.0, 4.0)], None);
        let marks = layout_marks(
            &s,
            Viewport {
                width: 0.0,
                height: 0.0,
                zoom: 0.0,
                origin_x: 0.0,
                origin_y: 0.0,
            },
            &mut MarksOptions {
                measure: Measure {
                    f: Box::new(board_measure),
                },
                readout: ReadoutFaces {
                    label: LABEL_LETTER_FACE,
                    small: TICK_LABEL_FACE,
                },
                plates: Vec::new(),
                corners: Vec::new(),
            },
        );
        assert!(marks.points.is_empty() && marks.hits.is_empty());
    }

    #[test]
    fn each_handle_has_its_44_px_hit_box_under_the_points() {
        let mut s = scene(r3, vec![p(0.5, 5.5)], None);
        s.handles = r3_handles();
        let marks = layout(s, 769.0);
        assert_eq!(marks.hits.len(), 3);
        assert_eq!(
            marks.hits[0],
            Hit {
                kind: HitKind::Point,
                id: "p".to_string(),
                r#box: Rect {
                    x: 469.0,
                    y: 97.0,
                    w: 48.0,
                    h: 48.0,
                },
                target: [0.0, 0.0],
            }
        );
        assert_eq!(
            marks.hits[1],
            Hit {
                kind: HitKind::Handle,
                id: "stretch".to_string(),
                r#box: Rect {
                    x: 446.0,
                    y: 224.0,
                    w: 44.0,
                    h: 44.0,
                },
                target: [0.0, 0.0],
            }
        );
        assert_eq!(
            marks.hits[2],
            Hit {
                kind: HitKind::Handle,
                id: "anchor".to_string(),
                r#box: Rect {
                    x: 396.0,
                    y: 324.0,
                    w: 44.0,
                    h: 44.0,
                },
                target: [0.0, 0.0],
            }
        );
        assert_eq!(
            marks.handles[0],
            HandleMark {
                x: 418.0,
                y: 346.0,
                shape: HandleShape::Diamond,
                ink: Token::Primary,
                halo: false,
                held: false,
                alpha: None,
            }
        );
        assert_eq!(
            marks.handles[1],
            HandleMark {
                x: 468.0,
                y: 246.0,
                shape: HandleShape::Square,
                ink: Token::Primary,
                halo: false,
                held: false,
                alpha: None,
            }
        );
    }

    #[test]
    fn marks_lay_out_r5_ks_dimension_line_ps_ghost_and_arrow() {
        let k = Annotation {
            layer: AnnotationLayer::Under,
            ink: Token::Primary,
            lines: vec![AnnotationLine {
                from: [-1.0, 0.0],
                to: [-1.0, 1.0],
                width: 2.5,
                dash: None,
                alpha: None,
                start_tick: Some(9.0),
                end_ticks: None,
                arrow: true,
            }],
        };
        let mut pp = p(0.5, 5.5);
        pp.was = Some(Was {
            x: 0.5,
            y: 4.5,
            ink: Token::Primary,
        });
        let marks = layout(
            Scene {
                grid_alpha: None,
                curves: vec![ghost(), curve(r3)],
                points: vec![pp],
                guides: Vec::new(),
                handles: r3_handles(),
                annotations: vec![k],
            },
            597.0,
        );
        let strokes = &marks.annotations[0].strokes;
        assert_eq!(strokes.len(), 1);
        assert!(
            strokes[0].x1 == 418.0
                && strokes[0].y1 == 396.0
                && strokes[0].x2 == 418.0
                && strokes[0].y2 == 346.0
                && strokes[0].width == 2.5
        );
        assert_eq!(strokes[0].start_tick, Some(9.0));
        assert!(strokes[0].arrow);
        // "P before": Ø22 at (493, 171); "P moved" from 158 to the head's tip at 135.
        let was = marks.points[0].was.clone().unwrap();
        assert!(was.x == 493.0 && was.y == 171.0 && was.ink == Token::ChartPoint1);
        let arrow = was.arrow.unwrap();
        assert!(
            arrow.x1 == 493.0
                && arrow.y1 == 158.0
                && arrow.x2 == 493.0
                && arrow.y2 == 135.0
                && arrow.ink == Token::Primary
        );
        assert_eq!(
            box_of(marks.points[0].label.clone()),
            Rect {
                x: 515.0,
                y: 66.8,
                w: 169.0,
                h: 41.0,
            }
        );
    }

    #[test]
    fn marks_lay_out_r6_the_anchors_drop_lines_and_halo() {
        let anchor = Annotation {
            layer: AnnotationLayer::Over,
            ink: Token::Primary,
            lines: vec![
                AnnotationLine {
                    from: [1.5, 1.0],
                    to: [1.5, 0.0],
                    width: 1.5,
                    dash: Some([5.0, 4.0]),
                    alpha: None,
                    start_tick: None,
                    end_ticks: None,
                    arrow: false,
                },
                AnnotationLine {
                    from: [1.5, 1.0],
                    to: [0.0, 1.0],
                    width: 1.5,
                    dash: Some([5.0, 4.0]),
                    alpha: None,
                    start_tick: None,
                    end_ticks: None,
                    arrow: false,
                },
            ],
        };
        let handles = vec![
            Handle {
                id: "anchor".to_string(),
                x: 1.5,
                y: 1.0,
                shape: HandleShape::Diamond,
                ink: Token::Primary,
                halo: true,
                held: true,
                alpha: None,
            },
            Handle {
                id: "stretch".to_string(),
                x: 2.5,
                y: 3.0,
                shape: HandleShape::Square,
                ink: Token::Primary,
                halo: false,
                held: false,
                alpha: None,
            },
        ];
        let marks = layout(
            Scene {
                grid_alpha: None,
                curves: vec![ghost(), curve(r6)],
                points: vec![p(0.5, 3.0)],
                guides: Vec::new(),
                handles,
                annotations: vec![anchor],
            },
            597.0,
        );
        // From the anchor (543, 346) down to the x-axis and across to the y-axis.
        let strokes = &marks.annotations[0].strokes;
        assert!(
            strokes[0].x1 == 543.0
                && strokes[0].y1 == 346.0
                && strokes[0].x2 == 543.0
                && strokes[0].y2 == 396.0
        );
        assert!(
            strokes[1].x1 == 543.0
                && strokes[1].y1 == 346.0
                && strokes[1].x2 == 468.0
                && strokes[1].y2 == 346.0
        );
        assert!(marks.handles[0].halo && marks.handles[0].held);
        // P's label placed clear of the curve: left-up.
        assert_eq!(
            box_of(marks.points[0].label.clone()),
            Rect {
                x: 302.0,
                y: 191.8,
                w: 169.0,
                h: 41.0,
            }
        );
    }

    #[test]
    fn a_fixed_label_holds_whatever_crowds_it() {
        // R6, where fvPlace takes P's label left-up (302, 191.8): here P is lit (its drop
        // lines out), Q's marker sits in the label's spot, and the anchor's drop line is out.
        let mut pp = p(0.5, 3.0);
        pp.label_place = LabelPlace::Fixed;
        pp.focus = true;
        pp.drop_lines = true;
        let handles = vec![
            Handle {
                id: "anchor".to_string(),
                x: 1.5,
                y: 1.0,
                shape: HandleShape::Diamond,
                ink: Token::Primary,
                halo: true,
                held: true,
                alpha: None,
            },
            Handle {
                id: "stretch".to_string(),
                x: 2.5,
                y: 3.0,
                shape: HandleShape::Square,
                ink: Token::Primary,
                halo: false,
                held: false,
                alpha: None,
            },
        ];
        let annotation = Annotation {
            layer: AnnotationLayer::Over,
            ink: Token::Primary,
            lines: vec![AnnotationLine {
                from: [1.5, 1.0],
                to: [1.5, 0.0],
                width: 1.5,
                dash: Some([5.0, 4.0]),
                alpha: None,
                start_tick: None,
                end_ticks: None,
                arrow: false,
            }],
        };
        let marks = layout(
            Scene {
                grid_alpha: None,
                curves: vec![ghost(), curve(r6)],
                points: vec![pp, q(1.5, 3.6)],
                guides: vec![Guide {
                    x: 1.5,
                    alpha: None,
                }],
                handles,
                annotations: vec![annotation],
            },
            597.0,
        );
        assert_eq!(
            box_of(marks.points[0].label.clone()),
            Rect {
                x: 515.0,
                y: 191.8,
                w: 169.0,
                h: 41.0,
            }
        );
        // Q's, placed clear, keeps clear of it.
        assert!(!intersects(
            marks.points[0].label.clone().unwrap().r#box,
            marks.points[1].label.clone().unwrap().r#box,
        ));
    }

    #[test]
    fn a_fixed_label_holds_over_the_labels_before_it() {
        // R6's P and Q, both held: Q (1.5, 3.6) at (543, 216) keeps its label 22 px right of
        // it and 13.2 px above it, across P's.
        let mut pp = p(0.5, 3.0);
        pp.label_place = LabelPlace::Fixed;
        let mut qq = q(1.5, 3.6);
        qq.label_place = LabelPlace::Fixed;
        let marks = layout(
            Scene {
                grid_alpha: None,
                curves: vec![ghost(), curve(r6)],
                points: vec![pp, qq],
                guides: vec![Guide {
                    x: 1.5,
                    alpha: None,
                }],
                handles: Vec::new(),
                annotations: Vec::new(),
            },
            597.0,
        );
        assert_eq!(
            box_of(marks.points[0].label.clone()),
            Rect {
                x: 515.0,
                y: 191.8,
                w: 169.0,
                h: 41.0,
            }
        );
        let qb = box_of(marks.points[1].label.clone());
        assert!(qb.x == 565.0 && qb.y == 161.8 && qb.h == 41.0);
        assert!(intersects(
            marks.points[0].label.clone().unwrap().r#box,
            marks.points[1].label.clone().unwrap().r#box,
        ));
    }

    #[test]
    fn no_arrow_from_where_a_point_was_when_the_two_nearly_touch() {
        let mut pp = p(2.0, 4.0);
        pp.was = Some(Was {
            x: 2.0,
            y: 3.6,
            ink: Token::Primary,
        });
        assert!(
            layout(scene(x2, vec![pp], None), 597.0).points[0]
                .was
                .clone()
                .unwrap()
                .arrow
                .is_none()
        );
    }

    #[test]
    fn where_a_point_was_is_marked_only_in_view() {
        // Was above the view: nothing to mark (a full-height arrow would read as a stray line).
        let mut above = p(2.0, 4.0);
        above.was = Some(Was {
            x: 2.0,
            y: 13.5,
            ink: Token::Primary,
        });
        assert!(
            layout(scene(x2, vec![above], None), 597.0).points[0]
                .was
                .is_none()
        );
        // Now below the view: the ring stays, the edge marker says where the point went.
        let mut below = p(2.0, -9.0);
        below.was = Some(Was {
            x: 2.0,
            y: 4.0,
            ink: Token::Primary,
        });
        let was = layout(scene(x2, vec![below], None), 597.0).points[0]
            .was
            .clone()
            .unwrap();
        assert!(was.x == 568.0 && was.y == 196.0 && was.arrow.is_none());
    }

    #[test]
    fn mid_motion_a_points_parts_fade_and_move_at_paint_time() {
        let rest = layout(
            scene(x2, vec![p(2.0, 4.0), q(-1.5, 2.25)], Some(-1.5)),
            769.0,
        );
        let mut pp = p(2.0, 4.0);
        pp.alpha = Some(0.5);
        pp.label_alpha = Some(0.5);
        pp.label_rise = 8.0;
        let mut qq = q(-1.5, 2.25);
        qq.alpha = Some(0.4);
        qq.reach = Some(0.5);
        let mut s = scene(x2, vec![pp, qq], None);
        s.guides = vec![Guide {
            x: -1.5,
            alpha: Some(0.4),
        }];
        let moving = layout(s, 769.0);
        assert_eq!(moving.points[0].marker.as_ref().unwrap().alpha, Some(0.5));
        assert_eq!(
            moving.points[0].label.clone().unwrap().motion,
            Some(PlateMotion {
                alpha: 0.25,
                dx: 0.0,
                dy: 8.0,
            })
        );
        assert_eq!(
            box_of(moving.points[0].label.clone()),
            box_of(rest.points[0].label.clone())
        );
        assert_eq!(
            moving.points[1].label.clone().unwrap().motion,
            Some(PlateMotion {
                alpha: 0.4,
                dx: 0.0,
                dy: 0.0,
            })
        );
        assert!(moving.guides[0].x == 393.0 && moving.guides[0].alpha == Some(0.4));
        // Q's drop lines reach half way to the axes.
        assert_eq!(
            moving.drop_lines[0],
            DropLine {
                ink: Token::ChartPoint2,
                x1: 393.0,
                y1: 283.5,
                x2: 393.0,
                y2: 339.75,
                alpha: Some(0.4),
            }
        );
        assert_eq!(
            moving.drop_lines[1],
            DropLine {
                ink: Token::ChartPoint2,
                x1: 393.0,
                y1: 283.5,
                x2: 430.5,
                y2: 283.5,
                alpha: Some(0.4),
            }
        );
        assert_eq!(moving.hits.len(), rest.hits.len());
        for (i, hit) in moving.hits.iter().enumerate() {
            assert_eq!(hit, &rest.hits[i]);
        }
    }

    #[test]
    fn a_point_at_rest_is_as_it_was() {
        let mut pp = p(2.0, 4.0);
        pp.alpha = Some(1.0);
        pp.label_alpha = Some(1.0);
        pp.label_rise = 0.0;
        let marks = layout(scene(x2, vec![pp], None), 769.0);
        assert_eq!(
            marks.points[0].marker.clone().unwrap(),
            Marker {
                x: 568.0,
                y: 196.0,
                style: PointStyle::Bullseye,
                ink: Token::ChartPoint1,
                focus: false,
                alpha: None,
            }
        );
        assert!(marks.points[0].label.clone().unwrap().motion.is_none());
    }

    #[test]
    fn handles_fade_and_an_off_view_point_names_its_edge() {
        let mut pp = p(2.0, 9.0);
        pp.alpha = Some(0.5);
        let mut s = scene(x2, vec![pp], None);
        s.handles = vec![Handle {
            id: "anchor".to_string(),
            x: 0.0,
            y: 0.0,
            shape: HandleShape::Diamond,
            ink: Token::Primary,
            halo: false,
            held: false,
            alpha: Some(0.3),
        }];
        let marks = layout(s, 769.0);
        assert!((marks.handles[0].alpha.unwrap() - 0.3).abs() < 1e-12);
        assert_eq!(marks.points[0].edge_dir, Some(Direction::Up));
        assert_eq!(
            marks.points[0].edge.clone().unwrap().motion,
            Some(PlateMotion {
                alpha: 0.5,
                dx: 0.0,
                dy: 0.0,
            })
        );
    }
}
