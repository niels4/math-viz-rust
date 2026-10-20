//! The plane scene: what the state draws on the plane.

use enum_map::EnumMap;

use super::copy::{POINT_NAMES, Point};
use super::form::{
    BASE_FUNCTIONS, Base, DEFAULT_PARAMS, Param, Params, anchor_point, evaluate, is_identity,
    unit_point,
};
use super::selectors::{Active, active, handle_held, handle_lit, p_lit, p_was};
use super::state::{Handle, PlaneMark, State};
use mv_plane::{
    Annotation, AnnotationLayer, AnnotationLine, Curve, CurveFn, Guide, Handle as PlaneHandle,
    HandleShape, Ink, LabelPlace, Point as PlanePoint, PointStyle, Scene, Was,
};

/// The point inks, by point.
pub static POINT_INKS: EnumMap<Point, Ink> =
    EnumMap::from_array([Ink::ChartPoint1, Ink::ChartPoint2]);

/// The f curve's width, in px.
pub const CURVE_WIDTH: f64 = 3.5;

/// The ghost curve's width, in px.
pub const GHOST_WIDTH: f64 = 2.0;

/// The ghost curve's dash.
pub const GHOST_DASH: [f64; 2] = [6.0, 6.0];

/// The ghost curve's alpha.
pub const GHOST_ALPHA: f64 = 0.75;

/// The annotation ink.
const LINK_INK: Ink = Ink::Primary;

/// A dimension line's width, in px.
const DIMENSION_WIDTH: f64 = 2.5;

/// A dimension line's start tick, in px.
const DIMENSION_START_TICK: f64 = 9.0;

/// A box annotation's width, in px.
const BOX_WIDTH: f64 = 1.5;

/// A box annotation's dash.
const BOX_DASH: [f64; 2] = [5.0, 4.0];

/// A box annotation's alpha.
const BOX_ALPHA: f64 = 0.8;

/// A side line's width, in px.
const SIDE_WIDTH: f64 = 3.0;

/// A side line's end ticks, in px.
const SIDE_ENDS: f64 = 7.0;

/// A drop line's width, in px.
const DROP_WIDTH: f64 = 1.5;

/// A drop line's dash.
const DROP_DASH: [f64; 2] = [5.0, 4.0];

/// What the scene builder reads: the function, P and Q, the ghost, the lit
/// and held handles, and P's trail.
///
/// `active` is `None` where the state acts on nothing (the union's nil): then
/// the plane draws no annotation.
#[derive(Clone, Debug)]
pub struct PlaneSceneInput {
    pub func: Base,
    pub params: Params,
    pub p_x: f64,
    pub q_x: Option<f64>,
    pub ghost_on: bool,
    pub p_lit: bool,
    pub active: Option<Active>,
    pub handle_lit: Option<Handle>,
    pub handle_held: Option<Handle>,
    pub p_was: Option<f64>,
}

/// Reads the scene's input from the state.
pub fn plane_scene_input(state: &State) -> PlaneSceneInput {
    PlaneSceneInput {
        func: state.func,
        params: state.params,
        p_x: state.p_x,
        q_x: state.q_x,
        ghost_on: state.ghost_on,
        p_lit: p_lit(state),
        active: active(state),
        handle_lit: handle_lit(state),
        handle_held: handle_held(state),
        p_was: p_was(state),
    }
}

/// A curve closure over an evaluator and parameters.
pub(crate) fn curve_fn(g: fn(f64) -> f64, params: Params) -> CurveFn {
    CurveFn {
        f: Box::new(move |x| evaluate(g, params, x)),
    }
}

/// The box annotation's sides at an alpha.
fn box_sides(from: [f64; 2], to: [f64; 2], alpha: Option<f64>) -> [AnnotationLine; 4] {
    let corners = [from, [to[0], from[1]], to, [from[0], to[1]]];
    std::array::from_fn(|i| AnnotationLine {
        from: corners[i],
        to: corners[(i + 1) % 4],
        width: BOX_WIDTH,
        dash: Some(BOX_DASH),
        alpha,
        start_tick: None,
        end_ticks: None,
        arrow: false,
    })
}

/// The scene's annotations.
fn annotations_for(input: &PlaneSceneInput) -> Vec<Annotation> {
    let Some(active) = input.active else {
        return Vec::new();
    };
    let anchor = anchor_point(input.params);
    let base = &BASE_FUNCTIONS[input.func];
    let unit = unit_point(base, input.params);
    let corner = [unit[0], anchor[1]];
    let mut lines: Vec<AnnotationLine> = Vec::new();
    let mut layer = AnnotationLayer::Under;
    match active {
        Active::Handle(handle) => {
            layer = AnnotationLayer::Over;
            match handle {
                Handle::Anchor => {
                    lines.push(AnnotationLine {
                        from: [0.0, anchor[1]],
                        to: anchor,
                        width: DROP_WIDTH,
                        dash: Some(DROP_DASH),
                        alpha: None,
                        start_tick: None,
                        end_ticks: None,
                        arrow: false,
                    });
                    lines.push(AnnotationLine {
                        from: anchor,
                        to: [anchor[0], 0.0],
                        width: DROP_WIDTH,
                        dash: Some(DROP_DASH),
                        alpha: None,
                        start_tick: None,
                        end_ticks: None,
                        arrow: false,
                    });
                }
                Handle::Stretch => {
                    lines.extend(box_sides(anchor, unit, None));
                    lines.push(AnnotationLine {
                        from: anchor,
                        to: corner,
                        width: SIDE_WIDTH,
                        dash: None,
                        alpha: None,
                        start_tick: None,
                        end_ticks: None,
                        arrow: false,
                    });
                    lines.push(AnnotationLine {
                        from: corner,
                        to: unit,
                        width: SIDE_WIDTH,
                        dash: None,
                        alpha: None,
                        start_tick: None,
                        end_ticks: None,
                        arrow: false,
                    });
                }
            }
        }
        Active::Param(p) => match p {
            Param::K => {
                lines.push(AnnotationLine {
                    from: [anchor[0], 0.0],
                    to: anchor,
                    width: DIMENSION_WIDTH,
                    dash: None,
                    alpha: None,
                    start_tick: Some(DIMENSION_START_TICK),
                    end_ticks: None,
                    arrow: true,
                });
            }
            Param::H => {
                lines.push(AnnotationLine {
                    from: [0.0, anchor[1]],
                    to: anchor,
                    width: DIMENSION_WIDTH,
                    dash: None,
                    alpha: None,
                    start_tick: Some(DIMENSION_START_TICK),
                    end_ticks: None,
                    arrow: true,
                });
            }
            Param::A | Param::B => {
                lines.extend(box_sides(anchor, unit, Some(BOX_ALPHA)));
                let (from, to) = if p == Param::B {
                    (anchor, corner)
                } else {
                    (corner, unit)
                };
                lines.push(AnnotationLine {
                    from,
                    to,
                    width: SIDE_WIDTH,
                    dash: None,
                    alpha: None,
                    start_tick: None,
                    end_ticks: Some(SIDE_ENDS),
                    arrow: false,
                });
            }
        },
    }
    vec![Annotation {
        layer,
        ink: LINK_INK,
        lines,
    }]
}

/// Builds the plane scene from its input.
pub fn build_plane_scene(input: &PlaneSceneInput) -> Scene {
    let base = &BASE_FUNCTIONS[input.func];
    let f = curve_fn(base.g, input.params);
    let p_y = mv_plane::curve_at(&f, input.p_x);
    let q_y = input.q_x.map(|x| mv_plane::curve_at(&f, x));
    let mut curves: Vec<Curve> = Vec::with_capacity(2);
    if input.ghost_on && !is_identity(input.params) {
        curves.push(Curve {
            id: "original".to_string(),
            func: curve_fn(base.g, DEFAULT_PARAMS),
            ink: Ink::ForegroundMuted,
            width: GHOST_WIDTH,
            dash: Some(GHOST_DASH),
            alpha: Some(GHOST_ALPHA),
            avoid: false,
            back: true,
            draw_to: None,
            glow: false,
        });
    }
    curves.push(Curve {
        id: "f".to_string(),
        func: f,
        ink: Ink::ChartLine,
        width: CURVE_WIDTH,
        dash: None,
        alpha: None,
        avoid: false,
        back: false,
        draw_to: None,
        glow: true,
    });

    let anchor = anchor_point(input.params);
    let unit = unit_point(base, input.params);
    let mut handles: Vec<PlaneHandle> = Vec::with_capacity(2);
    for handle in [Handle::Anchor, Handle::Stretch] {
        let at = if handle == Handle::Anchor {
            anchor
        } else {
            unit
        };
        handles.push(PlaneHandle {
            id: HANDLE_IDS[handle].to_string(),
            x: at[0],
            y: at[1],
            shape: if handle == Handle::Anchor {
                HandleShape::Diamond
            } else {
                HandleShape::Square
            },
            ink: LINK_INK,
            halo: input.handle_lit == Some(handle),
            held: input.handle_held == Some(handle),
            alpha: None,
        });
    }

    let mut points: Vec<PlanePoint> = Vec::with_capacity(2);
    let mut p = PlanePoint {
        id: POINT_IDS[Point::P].to_string(),
        x: input.p_x,
        y: p_y,
        style: PointStyle::Bullseye,
        ink: POINT_INKS[Point::P],
        name: POINT_NAMES[Point::P].to_string(),
        label_place: LabelPlace::Fixed,
        focus: input.p_lit,
        drop_lines: input.p_lit,
        edge_marker: true,
        draggable: true,
        was: None,
        alpha: None,
        label_alpha: None,
        label_rise: 0.0,
        reach: None,
    };
    if let Some(y) = input.p_was {
        p.was = Some(Was {
            x: input.p_x,
            y,
            ink: LINK_INK,
        });
    }
    points.push(p);
    let mut guides: Vec<Guide> = Vec::new();
    if let (Some(q_x), Some(q_y)) = (input.q_x, q_y) {
        points.push(PlanePoint {
            id: POINT_IDS[Point::Q].to_string(),
            x: q_x,
            y: q_y,
            style: PointStyle::Ring,
            ink: POINT_INKS[Point::Q],
            name: POINT_NAMES[Point::Q].to_string(),
            label_place: LabelPlace::Fixed,
            focus: false,
            drop_lines: true,
            edge_marker: true,
            draggable: false,
            was: None,
            alpha: None,
            label_alpha: None,
            label_rise: 0.0,
            reach: None,
        });
        guides.push(Guide {
            x: q_x,
            alpha: None,
        });
    }
    Scene {
        grid_alpha: None,
        curves,
        points,
        guides,
        handles,
        annotations: annotations_for(input),
    }
}

/// The point ids, by point.
pub static POINT_IDS: EnumMap<Point, &str> = EnumMap::from_array(["p", "q"]);

/// The handle ids, by handle.
pub static HANDLE_IDS: EnumMap<Handle, &str> = EnumMap::from_array(["anchor", "stretch"]);

/// The plane mark an id names, if any.
pub fn plane_mark(id: &str) -> Option<PlaneMark> {
    if id == POINT_IDS[Point::P] {
        return Some(PlaneMark::P);
    }
    for handle in [Handle::Anchor, Handle::Stretch] {
        if id == HANDLE_IDS[handle] {
            return Some(if handle == Handle::Anchor {
                PlaneMark::Anchor
            } else {
                PlaneMark::Stretch
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{PlaneSceneInput, build_plane_scene, plane_mark};
    use crate::form::{Base, DEFAULT_PARAMS, Param, Params};
    use crate::selectors::Active;
    use crate::state::{Handle, PlaneMark};
    use mv_numfmt::{DISPLAY_DP, format_mark};
    use mv_plane::{
        AnnotationLayer, AnnotationLine, Axis, Face, HandleShape, Ink, LabelPlace, Marker,
        MarksLayout, MarksOptions, Measure, Plate, PlateRunWhat, PointStyle, Rect, Viewport,
        curve_at, intersects, layout_grid, layout_marks, readout_faces,
    };
    use mv_theme::ReadoutFont;

    const BASE: PlaneSceneInput = PlaneSceneInput {
        func: Base::X2,
        params: DEFAULT_PARAMS,
        p_x: 2.0,
        q_x: None,
        ghost_on: true,
        p_lit: false,
        active: None,
        handle_lit: None,
        handle_held: None,
        p_was: None,
    };

    const R3: Params = enum_map::EnumMap::from_array([2.0, 1.0, 1.0, -1.0]);

    const VP: Viewport = Viewport {
        width: 936.0,
        height: 792.0,
        zoom: 50.0,
        origin_x: 468.0,
        origin_y: 396.0,
    };

    fn layout(scene: &mv_plane::Scene) -> MarksLayout {
        let measure = Measure {
            f: Box::new(|face: Face, s: &str| s.chars().count() as f64 * 0.6 * face.size),
        };
        let mut opts = MarksOptions {
            measure,
            readout: readout_faces(ReadoutFont::RobotoMono),
            plates: Vec::new(),
            corners: Vec::new(),
        };
        layout_marks(scene, VP, &mut opts)
    }

    struct Drawn {
        texts: Vec<String>,
        ticks_x: Vec<String>,
        ticks_y: Vec<String>,
    }

    fn draw(scene: &mv_plane::Scene) -> Drawn {
        let mut measure = Measure {
            f: Box::new(|_: Face, s: &str| s.chars().count() as f64 * 8.0),
        };
        let marks = layout(scene);
        let grid = layout_grid(VP, &mut measure, &[]);
        let mut texts = Vec::new();
        let mut xs = Vec::new();
        let mut ys = Vec::new();
        for label in &grid.labels {
            texts.push(label.text.clone());
            if label.axis == Axis::X {
                xs.push(label.text.clone());
            } else {
                ys.push(label.text.clone());
            }
        }
        if grid.origin.is_some() {
            texts.push("O".to_string());
        }
        for layer in &marks.points {
            for p in [&layer.edge, &layer.label].into_iter().flatten() {
                for run in &p.runs {
                    if let PlateRunWhat::Text(t) = &run.what {
                        texts.push(t.text.clone());
                    }
                }
            }
        }
        Drawn {
            texts,
            ticks_x: xs,
            ticks_y: ys,
        }
    }

    fn ids(scene: &mv_plane::Scene) -> String {
        scene
            .curves
            .iter()
            .map(|c| c.id.clone())
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn expect_line(got: &AnnotationLine, want: AnnotationLine) {
        assert_eq!(got, &want, "got {got:?}, want {want:?}");
    }

    fn full_line(from: [f64; 2], to: [f64; 2], width: f64) -> AnnotationLine {
        AnnotationLine {
            from,
            to,
            width,
            dash: None,
            alpha: None,
            start_tick: None,
            end_ticks: None,
            arrow: false,
        }
    }

    #[test]
    fn the_scene_draws_r1_r2s_defaults() {
        let scene = build_plane_scene(&BASE);
        assert_eq!(ids(&scene), "f");
        let f = &scene.curves[0];
        assert!(f.ink == Ink::ChartLine && f.width == 3.5 && f.glow);
        assert_eq!(scene.points.len(), 1);
        let p = &scene.points[0];
        assert_eq!(p.id, "p");
        assert_eq!(p.x, 2.0);
        assert_eq!(p.y, 4.0);
        assert_eq!(p.style, PointStyle::Bullseye);
        assert_eq!(p.ink, Ink::ChartPoint1);
        assert_eq!(p.name, "P");
        assert_eq!(p.label_place, LabelPlace::Fixed);
        assert!(!p.focus);
        assert!(!p.drop_lines);
        assert!(p.edge_marker);
        assert!(p.draggable);
        assert_eq!(p.was, None);
        assert_eq!(p.alpha, None);
        assert_eq!(p.label_alpha, None);
        assert_eq!(p.label_rise, 0.0);
        assert_eq!(p.reach, None);
        assert_eq!(scene.guides.len(), 0);
    }

    #[test]
    fn the_scene_draws_the_ghost_once_a_transform_is_set() {
        let input = PlaneSceneInput {
            params: R3,
            p_x: 0.5,
            ..BASE.clone()
        };
        let scene = build_plane_scene(&input);
        assert_eq!(ids(&scene), "original f");
        let (ghost, f) = (&scene.curves[0], &scene.curves[1]);
        assert!(ghost.ink == Ink::ForegroundMuted && ghost.width == 2.0 && ghost.back);
        assert!(ghost.dash == Some([6.0, 6.0]) && ghost.alpha == Some(0.75));
        assert_eq!(curve_at(&ghost.func, 2.0), 4.0);
        assert_eq!(curve_at(&f.func, 0.5), 5.5);
        let input = PlaneSceneInput {
            params: R3,
            p_x: 0.5,
            ghost_on: false,
            ..BASE.clone()
        };
        assert_eq!(ids(&build_plane_scene(&input)), "f");
    }

    #[test]
    fn the_scene_follows_the_pointer_with_q() {
        let input = PlaneSceneInput {
            q_x: Some(-1.5),
            ..BASE.clone()
        };
        let scene = build_plane_scene(&input);
        assert!(scene.points.len() == 2 && scene.points[0].id == "p" && scene.points[1].id == "q");
        let q = &scene.points[1];
        assert!(
            q.x == -1.5 && q.y == 2.25 && q.style == PointStyle::Ring && q.ink == Ink::ChartPoint2
        );
        assert!(
            q.name == "Q" && q.label_place == LabelPlace::Fixed && q.drop_lines && q.edge_marker
        );
        assert!(!q.draggable);
        assert!(scene.guides.len() == 1 && scene.guides[0].x == -1.5);
    }

    #[test]
    fn the_scene_lights_p() {
        let input = PlaneSceneInput {
            p_lit: true,
            ..BASE.clone()
        };
        let p = &build_plane_scene(&input).points[0];
        assert!(p.focus && p.drop_lines);
    }

    #[test]
    fn the_scene_puts_the_handles_on_the_anchor_and_the_unit_point() {
        let input = PlaneSceneInput {
            params: R3,
            ..BASE.clone()
        };
        let handles = &build_plane_scene(&input).handles;
        assert_eq!(handles[0].id, "anchor");
        assert_eq!((handles[0].x, handles[0].y), (-1.0, 1.0));
        assert_eq!(handles[0].shape, HandleShape::Diamond);
        assert_eq!(handles[0].ink, Ink::Primary);
        assert_eq!(handles[1].id, "stretch");
        assert_eq!((handles[1].x, handles[1].y), (0.0, 3.0));
        assert_eq!(handles[1].shape, HandleShape::Square);
        assert_eq!(handles[1].ink, Ink::Primary);
        let sin = PlaneSceneInput {
            func: Base::Sin,
            handle_lit: Some(Handle::Anchor),
            handle_held: Some(Handle::Anchor),
            ..BASE.clone()
        };
        let s = build_plane_scene(&sin);
        assert!(s.handles[1].x == std::f64::consts::PI / 2.0 && s.handles[1].y == 1.0);
        assert!(s.handles[0].halo && s.handles[0].held);
    }

    #[test]
    fn the_scene_draws_nothing_for_a_value_while_none_is_active() {
        let input = PlaneSceneInput {
            params: R3,
            ..BASE.clone()
        };
        assert_eq!(build_plane_scene(&input).annotations.len(), 0);
    }

    #[test]
    fn the_scene_draws_k_and_h_as_shifts_under_the_curve() {
        let input = PlaneSceneInput {
            params: R3,
            active: Some(Active::Param(Param::K)),
            ..BASE.clone()
        };
        let k = &build_plane_scene(&input).annotations[0];
        assert!(k.layer == AnnotationLayer::Under && k.ink == Ink::Primary && k.lines.len() == 1);
        expect_line(
            &k.lines[0],
            AnnotationLine {
                from: [-1.0, 0.0],
                to: [-1.0, 1.0],
                width: 2.5,
                dash: None,
                alpha: None,
                start_tick: Some(9.0),
                end_ticks: None,
                arrow: true,
            },
        );
        let input = PlaneSceneInput {
            params: R3,
            active: Some(Active::Param(Param::H)),
            ..BASE.clone()
        };
        let h = &build_plane_scene(&input).annotations[0];
        assert_eq!(h.lines.len(), 1);
        expect_line(
            &h.lines[0],
            AnnotationLine {
                from: [0.0, 1.0],
                to: [-1.0, 1.0],
                width: 2.5,
                dash: None,
                alpha: None,
                start_tick: Some(9.0),
                end_ticks: None,
                arrow: true,
            },
        );
    }

    #[test]
    fn the_scene_draws_a_and_b_as_the_unit_boxs_sides() {
        let input = PlaneSceneInput {
            params: R3,
            active: Some(Active::Param(Param::A)),
            ..BASE.clone()
        };
        let a = &build_plane_scene(&input).annotations[0];
        assert_eq!(a.lines.len(), 5);
        for line in &a.lines[..4] {
            assert!(line.dash.is_some() && line.alpha == Some(0.8));
        }
        expect_line(
            &a.lines[4],
            AnnotationLine {
                from: [0.0, 1.0],
                to: [0.0, 3.0],
                width: 3.0,
                dash: None,
                alpha: None,
                start_tick: None,
                end_ticks: Some(7.0),
                arrow: false,
            },
        );
        let input = PlaneSceneInput {
            params: R3,
            active: Some(Active::Param(Param::B)),
            ..BASE.clone()
        };
        let b = &build_plane_scene(&input).annotations[0];
        expect_line(
            &b.lines[4],
            AnnotationLine {
                from: [-1.0, 1.0],
                to: [0.0, 1.0],
                width: 3.0,
                dash: None,
                alpha: None,
                start_tick: None,
                end_ticks: Some(7.0),
                arrow: false,
            },
        );
    }

    #[test]
    fn the_scene_drops_a_dragged_anchors_h_and_k_onto_the_axes() {
        let input = PlaneSceneInput {
            params: enum_map::EnumMap::from_array([2.0, 1.0, 1.0, 1.5]),
            active: Some(Active::Handle(Handle::Anchor)),
            ..BASE.clone()
        };
        let anchor = &build_plane_scene(&input).annotations[0];
        assert_eq!(anchor.layer, AnnotationLayer::Over);
        assert_eq!(anchor.lines.len(), 2);
        expect_line(
            &anchor.lines[0],
            AnnotationLine {
                from: [0.0, 1.0],
                to: [1.5, 1.0],
                width: 1.5,
                dash: Some([5.0, 4.0]),
                alpha: None,
                start_tick: None,
                end_ticks: None,
                arrow: false,
            },
        );
        expect_line(
            &anchor.lines[1],
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
        );
    }

    #[test]
    fn the_scene_draws_a_dragged_stretch_grips_box_with_both_sides() {
        let input = PlaneSceneInput {
            params: enum_map::EnumMap::from_array([2.0, 1.0, 1.5, -1.0]),
            active: Some(Active::Handle(Handle::Stretch)),
            ..BASE.clone()
        };
        let b = &build_plane_scene(&input).annotations[0];
        assert_eq!(b.layer, AnnotationLayer::Over);
        expect_line(&b.lines[4], full_line([-1.0, 1.0], [0.5, 1.0], 3.0));
        expect_line(&b.lines[5], full_line([0.5, 1.0], [0.5, 3.0], 3.0));
    }

    #[test]
    fn the_scene_marks_where_a_transforms_drag_found_p() {
        let input = PlaneSceneInput {
            params: R3,
            p_x: 0.5,
            p_was: Some(4.5),
            ..BASE.clone()
        };
        assert_eq!(
            build_plane_scene(&input).points[0].was,
            Some(mv_plane::Was {
                x: 0.5,
                y: 4.5,
                ink: Ink::Primary,
            })
        );
    }

    fn held(marker: &Marker, label_box: &Rect) -> bool {
        let rest = [marker.x + 22.0, marker.y - 13.2 - label_box.h];
        assert!((label_box.x - rest[0].min(936.0 - 12.0 - label_box.w)).abs() < 1e-9);
        assert!((label_box.y - rest[1].max(12.0)).abs() < 1e-9);
        label_box.x != rest[0] || (label_box.y - rest[1]).abs() > 1e-9
    }

    fn placed_clear(mut scene: mv_plane::Scene, id: &str) -> mv_plane::Scene {
        for c in &mut scene.curves {
            if c.id == "f" {
                c.avoid = true;
            }
        }
        for p in &mut scene.points {
            if p.id == id {
                p.label_place = LabelPlace::Clear;
            }
        }
        scene
    }

    fn elsewhere(before: Option<&Plate>, label_box: &Rect) -> bool {
        match before {
            Some(b) => b.r#box.x != label_box.x || b.r#box.y != label_box.y,
            None => false,
        }
    }

    #[test]
    fn p_s_label_holds_up_right_whatever_crowds_it() {
        let mut places = 0;
        let mut at_edge = 0;
        let mut moved_before = 0;
        for func in [Base::X, Base::X2, Base::X3, Base::Sin] {
            for params in [DEFAULT_PARAMS, R3] {
                for active in [None, Some(Active::Handle(Handle::Anchor))] {
                    for i in 0..=36 {
                        let p_x = -9.0 + 0.5 * (i as f64);
                        for q_x in [None, Some(p_x + 0.3)] {
                            let input = PlaneSceneInput {
                                func,
                                params,
                                p_x,
                                q_x,
                                ghost_on: true,
                                p_lit: q_x.is_none(),
                                active,
                                handle_lit: None,
                                handle_held: None,
                                p_was: None,
                            };
                            let scene = build_plane_scene(&input);
                            let p = &layout(&scene).points[0];
                            let (Some(marker), Some(label)) = (&p.marker, &p.label) else {
                                continue;
                            };
                            places += 1;
                            if held(marker, &label.r#box) {
                                at_edge += 1;
                            }
                            if elsewhere(
                                layout(&placed_clear(build_plane_scene(&input), "p")).points[0]
                                    .label
                                    .as_ref(),
                                &label.r#box,
                            ) {
                                moved_before += 1;
                            }
                        }
                    }
                }
            }
        }
        assert!(
            places > 500 && at_edge > 50,
            "places {places}, at the edge {at_edge}"
        );
        assert!(
            moved_before as f64 / places as f64 > 0.5,
            "moved before {moved_before} of {places}"
        );
    }

    #[test]
    fn lit_p_and_q_leave_the_axes_to_the_tick_numbers() {
        let input = PlaneSceneInput {
            q_x: Some(-1.5),
            p_lit: true,
            ..BASE.clone()
        };
        let scene = build_plane_scene(&input);
        let lines = &layout(&scene).drop_lines;
        assert_eq!(lines.len(), 4);
        let want = [
            (Ink::ChartPoint1, 568.0, 396.0),
            (Ink::ChartPoint1, 468.0, 196.0),
            (Ink::ChartPoint2, 393.0, 396.0),
            (Ink::ChartPoint2, 468.0, 283.5),
        ];
        for (w, line) in want.iter().zip(lines.iter()) {
            assert!(line.ink == w.0 && line.x2 == w.1 && line.y2 == w.2);
        }
        let drawn = draw(&scene);
        for tick in [
            format_mark(-2.0, DISPLAY_DP),
            format_mark(-1.0, DISPLAY_DP),
            "2".to_string(),
        ] {
            assert!(drawn.ticks_x.contains(&tick));
        }
        for tick in ["2".to_string(), "4".to_string()] {
            assert!(drawn.ticks_y.contains(&tick));
        }
    }

    #[test]
    fn no_axis_is_tagged_for_p_or_q() {
        let mut scenes = 0;
        let mut lines = 0;
        for func in [Base::X, Base::X2, Base::X3, Base::Sin] {
            for params in [DEFAULT_PARAMS, R3] {
                for i in 0..=72 {
                    let x = -9.0 + 0.25 * (i as f64);
                    for lit in [false, true] {
                        for q_x in [None, Some(x + 0.1)] {
                            let input = PlaneSceneInput {
                                func,
                                params,
                                p_x: x,
                                q_x,
                                ghost_on: true,
                                p_lit: lit,
                                active: None,
                                handle_lit: None,
                                handle_held: None,
                                p_was: None,
                            };
                            let marks = layout(&build_plane_scene(&input));
                            assert_eq!(marks.annotations.len(), 0);
                            lines += marks.drop_lines.len();
                            scenes += 1;
                        }
                    }
                }
            }
        }
        assert!(scenes > 2000);
        assert!(lines > 2000);
    }

    #[test]
    fn q_s_label_holds_up_right_over_ps_where_they_meet() {
        let mut places = 0;
        let mut at_edge = 0;
        let mut over_p = 0;
        let mut moved_before = 0;
        for func in [Base::X, Base::X2, Base::X3, Base::Sin] {
            for params in [DEFAULT_PARAMS, R3] {
                for p_x in [-2.0, 0.5, 2.0] {
                    for i in 0..=72 {
                        let mut input = PlaneSceneInput {
                            func,
                            params,
                            p_x,
                            q_x: Some(-9.0 + 0.25 * (i as f64)),
                            ghost_on: true,
                            p_lit: p_x == 0.5,
                            active: None,
                            handle_lit: None,
                            handle_held: None,
                            p_was: None,
                        };
                        if p_x == 2.0 {
                            input.active = Some(Active::Param(Param::K));
                        }
                        let scene = build_plane_scene(&input);
                        let points = &layout(&scene).points;
                        let (p, q) = (&points[0], &points[1]);
                        let (Some(marker), Some(label)) = (&q.marker, &q.label) else {
                            continue;
                        };
                        places += 1;
                        if held(marker, &label.r#box) {
                            at_edge += 1;
                        }
                        if let Some(pl) = &p.label
                            && intersects(pl.r#box, label.r#box)
                        {
                            over_p += 1;
                        }
                        if elsewhere(
                            layout(&placed_clear(build_plane_scene(&input), "q")).points[1]
                                .label
                                .as_ref(),
                            &label.r#box,
                        ) {
                            moved_before += 1;
                        }
                    }
                }
            }
        }
        assert!(
            places > 800 && at_edge > 100 && over_p > 100,
            "{places} places, {at_edge} at the edge, {over_p} over P"
        );
        assert!(
            moved_before as f64 / places as f64 > 0.5,
            "moved before {moved_before} of {places}"
        );
    }

    #[test]
    fn the_plane_prints_only_ticks_the_o_and_ps_and_qs_labels() {
        const ACTIVE: [Option<Active>; 7] = [
            None,
            Some(Active::Param(Param::A)),
            Some(Active::Param(Param::B)),
            Some(Active::Param(Param::H)),
            Some(Active::Param(Param::K)),
            Some(Active::Handle(Handle::Anchor)),
            Some(Active::Handle(Handle::Stretch)),
        ];
        let mut drawn_count = 0;
        for func in [Base::X, Base::X2, Base::X3, Base::Sin] {
            for params in [DEFAULT_PARAMS, R3] {
                for active in ACTIVE {
                    let mut input = PlaneSceneInput {
                        func,
                        params,
                        p_x: -1.0,
                        q_x: Some(0.25),
                        ghost_on: true,
                        p_lit: true,
                        active,
                        handle_lit: None,
                        handle_held: None,
                        p_was: None,
                    };
                    if let Some(Active::Handle(handle)) = active {
                        input.handle_lit = Some(handle);
                        input.handle_held = Some(handle);
                    }
                    let scene = build_plane_scene(&input);
                    assert_eq!(
                        scene.annotations.len(),
                        if active.is_none() { 0 } else { 1 }
                    );
                    let drawn = draw(&scene);
                    for text in &drawn.texts {
                        let tick = drawn.ticks_x.contains(text) || drawn.ticks_y.contains(text);
                        let letter = text == "O" || text == "P" || text == "Q";
                        let coords = text.starts_with('(') && text.ends_with(')');
                        assert!(tick || letter || coords, "{text:?} drawn on the plane");
                    }
                    assert!(drawn.texts.iter().any(|t| t == "P"));
                    assert!(drawn.texts.iter().any(|t| t == "Q"));
                    drawn_count += 1;
                }
            }
        }
        assert_eq!(drawn_count, 4 * 2 * ACTIVE.len());
    }

    #[test]
    fn the_tick_numbers_stay_where_a_dragged_anchors_tags_were() {
        let input = PlaneSceneInput {
            params: enum_map::EnumMap::from_array([1.0, 0.0, 1.0, 2.0]),
            active: Some(Active::Handle(Handle::Anchor)),
            handle_lit: Some(Handle::Anchor),
            handle_held: Some(Handle::Anchor),
            ..BASE.clone()
        };
        let drawn = draw(&build_plane_scene(&input));
        assert!(
            drawn.ticks_x.contains(&"2".to_string()) && drawn.ticks_y.contains(&"1".to_string())
        );
        let mut twos = 0;
        let mut ones = 0;
        for text in &drawn.texts {
            if text == "2" {
                twos += 1;
            }
            if text == "1" {
                ones += 1;
            }
        }
        assert!(twos == 2 && ones == 2);
    }

    #[test]
    fn plane_mark_names_p_and_the_handles() {
        assert_eq!(plane_mark("p"), Some(PlaneMark::P));
        assert_eq!(plane_mark("anchor"), Some(PlaneMark::Anchor));
        assert_eq!(plane_mark("stretch"), Some(PlaneMark::Stretch));
        assert_eq!(plane_mark("q"), None);
        assert_eq!(plane_mark(""), None);
    }
}
