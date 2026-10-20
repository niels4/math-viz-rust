//! Edge-marker arrivals: the slide-in of markers entering the view.

use enum_map::EnumMap;
use mv_motion::{Ease, FAST_MS, REDUCED_FADE_MS, ease_at};

use super::marks::MarksLayout;
use super::placement::Direction;
use super::plates::{Plate, PlateMotion};

/// How far an arriving marker slides, in px.
pub const EDGE_SLIDE_PX: f64 = 8.0;

/// An arriving edge marker: its id, when it arrived, and the motion level.
#[derive(Clone, Debug, PartialEq)]
pub struct EdgeArrival {
    pub id: String,
    pub since: f64,
    pub reduced: bool,
}

/// The outward slide per direction, in px.
static OUTWARD: EnumMap<Direction, [f64; 2]> =
    EnumMap::from_array([[0.0, -1.0], [0.0, 1.0], [-1.0, 0.0], [1.0, 0.0]]);

/// The arrival motion of an edge marker from a direction at an elapsed time.
pub fn edge_arrival(dir: Direction, elapsed: f64, reduced: bool) -> Option<PlateMotion> {
    let duration = if reduced { REDUCED_FADE_MS } else { FAST_MS };
    if elapsed >= duration {
        return None;
    }
    let t = 0.0f64.max(elapsed / duration);
    if reduced {
        return Some(PlateMotion {
            alpha: t,
            dx: 0.0,
            dy: 0.0,
        });
    }
    let e = ease_at(Ease::Enter, t);
    let off = EDGE_SLIDE_PX * (1.0 - e);
    Some(PlateMotion {
        alpha: 1.0f64.min(e),
        dx: OUTWARD[dir][0] * off,
        dy: OUTWARD[dir][1] * off,
    })
}

/// Applies a motion to a plate.
fn compose(plate: &Plate, m: PlateMotion) -> Plate {
    let under = plate.motion.unwrap_or(PlateMotion {
        alpha: 1.0,
        dx: 0.0,
        dy: 0.0,
    });
    let mut out = plate.clone();
    out.motion = Some(PlateMotion {
        alpha: under.alpha * m.alpha,
        dx: under.dx + m.dx,
        dy: under.dy + m.dy,
    });
    out
}

/// Arrives the edges of a laid-out marks: slides markers in, drops the gone.
pub fn arrive_edges(
    marks: &mut MarksLayout,
    arrivals: &mut Vec<EdgeArrival>,
    now: f64,
    reduced: bool,
) -> bool {
    let mut arriving = false;
    let mut shown: Vec<String> = Vec::with_capacity(marks.points.len());
    for layer in marks.points.iter_mut() {
        let (Some(edge), Some(dir)) = (layer.edge.clone(), layer.edge_dir) else {
            continue;
        };
        shown.push(layer.id.clone());
        let mut at: Option<usize> = None;
        for (i, a) in arrivals.iter().enumerate() {
            if a.id == layer.id {
                at = Some(i);
            }
        }
        let at = match at {
            Some(i) => i,
            None => {
                arrivals.push(EdgeArrival {
                    id: layer.id.clone(),
                    since: now,
                    reduced,
                });
                arrivals.len() - 1
            }
        };
        let arrival = arrivals[at].clone();
        let Some(m) = edge_arrival(dir, now - arrival.since, arrival.reduced) else {
            continue;
        };
        arriving = true;
        layer.edge = Some(compose(&edge, m));
    }
    for i in (0..arrivals.len()).rev() {
        if !shown.contains(&arrivals[i].id) {
            arrivals.remove(i);
        }
    }
    arriving
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::marks::{MarksLayout, PointLayer};
    use crate::rect::Rect;
    use mv_theme::Token;

    fn plate() -> Plate {
        Plate {
            r#box: Rect {
                x: 400.0,
                y: 22.0,
                w: 118.0,
                h: 39.0,
            },
            radius: 19.5,
            fill: Token::Card,
            border_ink: Token::ChartPoint1,
            border_width: 2.0,
            runs: Vec::new(),
            motion: None,
        }
    }

    fn layer_with(id: &str, edge: Option<Plate>) -> PointLayer {
        let dir = if edge.is_some() {
            Some(Direction::Up)
        } else {
            None
        };
        PointLayer {
            id: id.to_string(),
            was: None,
            marker: None,
            label: None,
            edge,
            edge_dir: dir,
        }
    }

    fn marks_with(layers: Vec<PointLayer>) -> MarksLayout {
        MarksLayout {
            annotations: Vec::new(),
            handles: Vec::new(),
            guides: Vec::new(),
            drop_lines: Vec::new(),
            points: layers,
            hits: Vec::new(),
        }
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 5e-10
    }

    #[test]
    fn an_edge_marker_slides_8_px_in_from_its_edge_on_the_enter_spring() {
        assert_eq!(
            edge_arrival(Direction::Up, 0.0, false),
            Some(PlateMotion {
                alpha: 0.0,
                dx: 0.0,
                dy: -8.0,
            })
        );
        let mid = edge_arrival(Direction::Down, 80.0, false).unwrap_or(PlateMotion {
            alpha: 0.0,
            dx: 0.0,
            dy: 0.0,
        });
        let e = ease_at(Ease::Enter, 0.5);
        assert!(close(mid.alpha, 1.0f64.min(e)));
        assert!(close(mid.dy, 8.0 * (1.0 - e)));
        let left = edge_arrival(Direction::Left, 40.0, false).unwrap_or(PlateMotion {
            alpha: 0.0,
            dx: 0.0,
            dy: 0.0,
        });
        let right = edge_arrival(Direction::Right, 40.0, false).unwrap_or(PlateMotion {
            alpha: 0.0,
            dx: 0.0,
            dy: 0.0,
        });
        assert!(left.dx < 0.0);
        assert!(right.dx > 0.0);
        assert_eq!(edge_arrival(Direction::Up, 160.0, false), None);
    }

    #[test]
    fn an_edge_marker_only_fades_under_reduced_motion() {
        assert_eq!(
            edge_arrival(Direction::Up, 60.0, true),
            Some(PlateMotion {
                alpha: 0.5,
                dx: 0.0,
                dy: 0.0,
            })
        );
        assert_eq!(edge_arrival(Direction::Up, 120.0, true), None);
    }

    #[test]
    fn arrivals_time_each_marker_from_when_it_appears_and_forget_it_once_gone() {
        let mut arrivals: Vec<EdgeArrival> = Vec::new();
        let mut first = marks_with(vec![layer_with("p", Some(plate())), layer_with("q", None)]);
        assert!(arrive_edges(&mut first, &mut arrivals, 1000.0, false));
        let edge = first.points[0].edge.clone().unwrap_or(plate());
        assert_eq!(
            edge.motion,
            Some(PlateMotion {
                alpha: 0.0,
                dx: 0.0,
                dy: -8.0,
            })
        );
        assert_eq!(edge.r#box, plate().r#box);
        assert!(first.points[1].edge.is_none());

        let mut later = marks_with(vec![layer_with("p", Some(plate()))]);
        let _ = arrive_edges(&mut later, &mut arrivals, 1100.0, false);
        let edge = later.points[0].edge.clone().unwrap_or(plate());
        let m = edge.motion.unwrap_or(PlateMotion {
            alpha: 0.0,
            dx: 0.0,
            dy: 0.0,
        });
        assert!(close(
            m.alpha,
            1.0f64.min(ease_at(Ease::Enter, 100.0 / 160.0))
        ));

        let mut rested = marks_with(vec![layer_with("p", Some(plate()))]);
        assert!(!arrive_edges(&mut rested, &mut arrivals, 1160.0, false));
        let at_rest = rested.points[0].edge.clone().unwrap_or(plate());
        assert!(at_rest.motion.is_none() && at_rest.r#box == plate().r#box);

        let mut gone = marks_with(vec![layer_with("p", None)]);
        let _ = arrive_edges(&mut gone, &mut arrivals, 1200.0, false);
        assert_eq!(arrivals.len(), 0);
        let mut again = marks_with(vec![layer_with("p", Some(plate()))]);
        assert!(arrive_edges(&mut again, &mut arrivals, 5000.0, false));
    }

    #[test]
    fn an_arrival_fades_on_top_of_the_points_own_fade() {
        let mut arrivals: Vec<EdgeArrival> = Vec::new();
        let mut faded_plate = plate();
        faded_plate.motion = Some(PlateMotion {
            alpha: 0.5,
            dx: 0.0,
            dy: 0.0,
        });
        let mut out = marks_with(vec![layer_with("p", Some(faded_plate.clone()))]);
        let _ = arrive_edges(&mut out, &mut arrivals, 0.0, true);
        let mut again = marks_with(vec![layer_with("p", Some(faded_plate.clone()))]);
        let _ = arrive_edges(&mut again, &mut arrivals, 0.0, true);
        let mut half = marks_with(vec![layer_with("p", Some(faded_plate.clone()))]);
        let _ = arrive_edges(&mut half, &mut arrivals, 60.0, true);
        let out_edge = out.points[0].edge.clone().unwrap_or(plate());
        let half_edge = half.points[0].edge.clone().unwrap_or(plate());
        assert_eq!(
            out_edge
                .motion
                .unwrap_or(PlateMotion {
                    alpha: 0.0,
                    dx: 0.0,
                    dy: 0.0,
                })
                .alpha,
            0.0
        );
        assert!(close(
            half_edge
                .motion
                .unwrap_or(PlateMotion {
                    alpha: 0.0,
                    dx: 0.0,
                    dy: 0.0,
                })
                .alpha,
            0.25
        ));
    }
}
