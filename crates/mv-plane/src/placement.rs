//! Label placement: polylines along the curves, and boxes beside or around them.

use super::rect::{Rect, inflate, intersects};
use super::scene::{CurveFn, curve_at};
use super::viewport::{Viewport, to_math_x, to_screen_y};
use enum_map::Enum;

/// A polyline: screen points along one curve run.
pub type Polyline = Vec<[f64; 2]>;

/// Where an edge marker sits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

/// Samples per px when testing meetings, in px.
const SAMPLE_PX: f64 = 2.0;

/// The band around the viewport tested for meetings, in px.
const BAND_PX: f64 = 100.0;

/// The polylines along the curves through a viewport.
pub fn curve_polylines(vp: Viewport, func: &CurveFn) -> Vec<Polyline> {
    let mut lines: Vec<Polyline> = Vec::new();
    let mut line: Polyline = Vec::new();
    let mut px = 0.0;
    while px < vp.width + SAMPLE_PX {
        let x = px.min(vp.width);
        let y = curve_at(func, to_math_x(vp, x));
        if y.is_nan() || y.is_infinite() {
            if line.len() > 1 {
                lines.push(std::mem::take(&mut line));
            } else {
                line.clear();
            }
            px += SAMPLE_PX;
            continue;
        }
        line.push([
            x,
            (vp.height + BAND_PX).min((-BAND_PX).max(to_screen_y(vp, y))),
        ]);
        px += SAMPLE_PX;
    }
    if line.len() > 1 {
        lines.push(line);
    }
    lines
}

/// Whether a segment meets a rectangle.
pub fn segment_meets_rect(x1: f64, y1: f64, x2: f64, y2: f64, r: Rect) -> bool {
    let (dx, dy) = (x2 - x1, y2 - y1);
    let (mut t0, mut t1): (f64, f64) = (0.0, 1.0);
    let sides = [
        [-dx, x1 - r.x],
        [dx, r.x + r.w - x1],
        [-dy, y1 - r.y],
        [dy, r.y + r.h - y1],
    ];
    for side in sides {
        let (p, q) = (side[0], side[1]);
        if p == 0.0 {
            if q < 0.0 {
                return false;
            }
            continue;
        }
        let t = q / p;
        if p < 0.0 {
            t0 = t0.max(t);
        } else {
            t1 = t1.min(t);
        }
        if t0 > t1 {
            return false;
        }
    }
    true
}

/// Whether any curve passes within a margin of a box.
pub fn curves_meet(r#box: Rect, curves: &[Polyline], margin: f64) -> bool {
    let r = inflate(r#box, margin);
    for line in curves {
        for i in 0..line.len().saturating_sub(1) {
            let (a, b) = (line[i], line[i + 1]);
            if segment_meets_rect(a[0], a[1], b[0], b[1], r) {
                return true;
            }
        }
    }
    false
}

/// The search directions around an anchor, in order.
const DIRECTIONS: [[f64; 2]; 8] = [
    [1.0, -1.0],
    [-1.0, -1.0],
    [1.0, 1.0],
    [-1.0, 1.0],
    [1.0, 0.0],
    [-1.0, 0.0],
    [0.0, -1.0],
    [0.0, 1.0],
];

/// The search rings around an anchor, in px.
const RINGS: [f64; 3] = [0.0, 26.0, 60.0];

/// How far down each ring reaches, as a share.
const RING_Y: f64 = 0.6;

/// The margin an edge box keeps from the viewport edge, in px.
const EDGE_MARGIN: f64 = 12.0;

/// The clearance a label keeps from the curves, in px.
const CURVE_CLEAR: f64 = 6.0;

/// What a placement avoids: curves, obstacle boxes, and its own size.
#[derive(Clone, Debug)]
pub struct PlaceOptions {
    pub gap: f64,
    pub curves: Vec<Polyline>,
    pub obstacles: Vec<Rect>,
    pub width: f64,
    pub height: f64,
}

/// Places a box beside an anchor, clear of curves and obstacles.
pub fn place_beside(cx: f64, cy: f64, w: f64, h: f64, opts: &PlaceOptions) -> Rect {
    for extra in RINGS {
        let ring = opts.gap + extra;
        for d in DIRECTIONS {
            let b = Rect {
                x: if d[0] > 0.0 {
                    cx + ring
                } else if d[0] < 0.0 {
                    cx - ring - w
                } else {
                    cx - w / 2.0
                },
                y: if d[1] > 0.0 {
                    cy + ring * RING_Y
                } else if d[1] < 0.0 {
                    cy - ring * RING_Y - h
                } else {
                    cy - h / 2.0
                },
                w,
                h,
            };
            let inside = b.x >= EDGE_MARGIN
                && b.y >= EDGE_MARGIN
                && b.x + w <= opts.width - EDGE_MARGIN
                && b.y + h <= opts.height - EDGE_MARGIN;
            if inside
                && !curves_meet(b, &opts.curves, CURVE_CLEAR)
                && !meets_any(b, &opts.obstacles)
            {
                return b;
            }
        }
    }
    Rect {
        x: clamp_low(cx + opts.gap, EDGE_MARGIN, opts.width - EDGE_MARGIN - w),
        y: clamp_low(
            cy - opts.gap - h,
            EDGE_MARGIN,
            opts.height - EDGE_MARGIN - h,
        ),
        w,
        h,
    }
}

/// Whether a box meets any obstacle.
fn meets_any(r#box: Rect, others: &[Rect]) -> bool {
    others.iter().any(|o| intersects(r#box, *o))
}

/// Clamps a value into a range, the low bound winning ties.
fn clamp_low(v: f64, lo: f64, hi: f64) -> f64 {
    v.max(lo).min(lo.max(hi))
}

/// Places a box at a fixed anchor, clamped into the viewport clear of chrome.
#[allow(clippy::too_many_arguments)]
pub fn place_fixed(
    cx: f64,
    cy: f64,
    w: f64,
    h: f64,
    gap: f64,
    chrome: &[Rect],
    width: f64,
    height: f64,
) -> Rect {
    let inside = |b: Rect| -> Rect {
        Rect {
            x: clamp_low(b.x, EDGE_MARGIN, width - EDGE_MARGIN - b.w),
            y: clamp_low(b.y, EDGE_MARGIN, height - EDGE_MARGIN - b.h),
            w: b.w,
            h: b.h,
        }
    };
    let mut b = inside(Rect {
        x: cx + gap,
        y: cy - gap * RING_Y - h,
        w,
        h,
    });
    for plate in chrome {
        if !intersects(b, *plate) {
            continue;
        }
        if plate.w >= plate.h {
            b.y = if plate.y + plate.h / 2.0 < height / 2.0 {
                plate.y + plate.h
            } else {
                plate.y - h
            };
        } else {
            b.x = if plate.x + plate.w / 2.0 < width / 2.0 {
                plate.x + plate.w
            } else {
                plate.x - w
            };
        }
    }
    inside(b)
}

/// Which edge of the viewport a point leans to, if any.
pub fn edge_direction(sx: f64, sy: f64, width: f64, height: f64) -> Option<Direction> {
    let over_x = if sx < 0.0 {
        -sx
    } else if sx > width {
        sx - width
    } else {
        0.0
    };
    let over_y = if sy < 0.0 {
        -sy
    } else if sy > height {
        sy - height
    } else {
        0.0
    };
    if over_x == 0.0 && over_y == 0.0 {
        return None;
    }
    if over_y >= over_x {
        Some(if sy < 0.0 {
            Direction::Up
        } else {
            Direction::Down
        })
    } else {
        Some(if sx < 0.0 {
            Direction::Left
        } else {
            Direction::Right
        })
    }
}

/// How far into a corner an edge marker sits, in px.
const EDGE_CORNER: f64 = 24.0;

/// How far from the edge an edge marker sits, in px.
const EDGE_INSET: f64 = 22.0;

/// The gap edge markers keep from the chrome, in px.
const EDGE_CHROME_GAP: f64 = 26.0;

/// The box of an edge marker in a direction at a point.
pub fn edge_marker_box(
    dir: Direction,
    at: [f64; 2],
    size: [f64; 2],
    width: f64,
    height: f64,
    chrome: &[Rect],
) -> Rect {
    let (w, h) = (size[0], size[1]);
    let along_x = clamp_low(at[0] - w / 2.0, EDGE_CORNER, width - EDGE_CORNER - w);
    let along_y = clamp_low(at[1] - h / 2.0, EDGE_CORNER, height - EDGE_CORNER - h);
    let mut b = match dir {
        Direction::Up => Rect {
            x: along_x,
            y: EDGE_INSET,
            w,
            h,
        },
        Direction::Down => Rect {
            x: along_x,
            y: height - EDGE_INSET - h,
            w,
            h,
        },
        Direction::Left => Rect {
            x: EDGE_INSET,
            y: along_y,
            w,
            h,
        },
        Direction::Right => Rect {
            x: width - EDGE_INSET - w,
            y: along_y,
            w,
            h,
        },
    };
    for _ in chrome {
        let (plate, found) = first_meeting(b, chrome);
        if !found {
            break;
        }
        match dir {
            Direction::Up => b.y = plate.y + plate.h + EDGE_CHROME_GAP,
            Direction::Down => b.y = plate.y - EDGE_CHROME_GAP - h,
            Direction::Left => b.x = plate.x + plate.w + EDGE_CHROME_GAP,
            Direction::Right => b.x = plate.x - EDGE_CHROME_GAP - w,
        }
    }
    b
}

/// The first obstacle a box meets, if any.
fn first_meeting(r#box: Rect, others: &[Rect]) -> (Rect, bool) {
    for o in others {
        if intersects(r#box, *o) {
            return (*o, true);
        }
    }
    (
        Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        },
        false,
    )
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::super::faces::js_round;
    use super::super::scene::CurveFn;
    use super::*;

    const R2: Viewport = Viewport {
        width: 936.0,
        height: 792.0,
        zoom: 50.0,
        origin_x: 468.0,
        origin_y: 396.0,
    };
    const W: f64 = 936.0;
    const H: f64 = 792.0;

    fn square(x: f64) -> f64 {
        x * x
    }

    fn curve_fn_of(f: impl Fn(f64) -> f64 + 'static) -> CurveFn {
        CurveFn { f: Box::new(f) }
    }

    #[test]
    fn curve_polylines_sample_every_2_px_clamped_near_the_view() {
        let lines = curve_polylines(R2, &curve_fn_of(square));
        let line = &lines[0];
        assert_eq!(line[0], [0.0, -100.0]);
        let mut found = false;
        for pt in line {
            if pt[0] == 568.0 {
                assert_eq!(*pt, [568.0, 196.0]);
                found = true;
            }
        }
        assert!(found);
        assert_eq!(line[line.len() - 1][0], 936.0);
        assert_eq!(line[line.len() - 2][0], 934.0);
    }

    #[test]
    fn curve_polylines_break_where_the_curve_has_no_value() {
        let lines = curve_polylines(
            R2,
            &curve_fn_of(|x| if x > 0.0 { x.ln() } else { f64::NAN }),
        );
        assert_eq!(lines.len(), 1);
        assert!(lines[0][0][0] > 468.0);
    }

    #[test]
    fn segment_meets_rect_finds_what_crosses_and_misses_what_passes() {
        let b = Rect {
            x: 10.0,
            y: 10.0,
            w: 10.0,
            h: 10.0,
        };
        assert!(segment_meets_rect(0.0, 15.0, 30.0, 15.0, b));
        assert!(segment_meets_rect(0.0, 0.0, 15.0, 15.0, b));
        assert!(segment_meets_rect(12.0, 12.0, 14.0, 14.0, b));
        assert!(!segment_meets_rect(0.0, 0.0, 30.0, 5.0, b));
        assert!(!segment_meets_rect(0.0, 30.0, 30.0, 21.0, b));
        assert!(!segment_meets_rect(25.0, 0.0, 25.0, 30.0, b));
    }

    fn beside_opts(curves: Vec<Polyline>, obstacles: Vec<Rect>) -> PlaceOptions {
        PlaceOptions {
            gap: 22.0,
            curves,
            obstacles,
            width: W,
            height: H,
        }
    }

    #[test]
    fn place_beside_puts_r2s_labels_where_fvplace_does() {
        let x2 = curve_polylines(R2, &curve_fn_of(square));
        let p = place_beside(568.0, 196.0, 108.0, 41.0, &beside_opts(x2.clone(), vec![]));
        assert_eq!(p.x, 590.0);
        assert!((p.y - 141.8).abs() < 1e-9);
        let q = place_beside(393.0, 283.5, 172.0, 41.0, &beside_opts(x2, vec![]));
        assert_eq!(q.x, 199.0);
        assert!((q.y - 296.7).abs() < 1e-9);
    }

    #[test]
    fn place_beside_keeps_clear_of_obstacles_and_the_edges() {
        let blocked = Rect {
            x: 580.0,
            y: 100.0,
            w: 150.0,
            h: 60.0,
        };
        let b = place_beside(
            568.0,
            196.0,
            108.0,
            41.0,
            &beside_opts(vec![], vec![blocked]),
        );
        assert_eq!(b.x, 568.0 - 22.0 - 108.0);
        let corner = place_beside(20.0, 20.0, 108.0, 41.0, &beside_opts(vec![], vec![]));
        assert!(corner.x >= 12.0 && corner.y >= 12.0);
        let all = Rect {
            x: 0.0,
            y: 0.0,
            w: W,
            h: H,
        };
        assert_eq!(
            place_beside(900.0, 30.0, 108.0, 41.0, &beside_opts(vec![], vec![all])),
            Rect {
                x: W - 12.0 - 108.0,
                y: 12.0,
                w: 108.0,
                h: 41.0
            },
        );
    }

    fn fixed(cx: f64, cy: f64, w: f64, chrome: &[Rect]) -> Rect {
        place_fixed(cx, cy, w, 41.0, 22.0, chrome, W, H)
    }

    fn tenths(v: f64) -> f64 {
        js_round(v * 10.0) / 10.0
    }

    #[test]
    fn place_fixed_holds_up_right_of_the_point() {
        let r6 = curve_polylines(R2, &curve_fn_of(|x| 2.0 * (x - 1.5) * (x - 1.5) + 1.0));
        assert_eq!(
            place_beside(493.0, 246.0, 169.0, 41.0, &beside_opts(r6, vec![])).x,
            302.0
        );
        let b = fixed(493.0, 246.0, 169.0, &[]);
        assert!(b.x == 515.0 && tenths(b.y) == 191.8 && b.w == 169.0 && b.h == 41.0);
        let wide = fixed(568.0, 196.0, 230.0, &[]);
        let narrow = fixed(568.0, 196.0, 169.0, &[]);
        assert!(wide.x == narrow.x && wide.y == narrow.y && wide.w == 230.0);
    }

    #[test]
    fn place_fixed_moves_only_as_far_as_the_edges_make_it() {
        let a = fixed(850.0, 400.0, 169.0, &[]);
        assert!(a.x == W - 12.0 - 169.0 && tenths(a.y) == 345.8);
        let b = fixed(300.0, 30.0, 169.0, &[]);
        assert!(b.x == 322.0 && b.y == 12.0);
        let c = fixed(900.0, 20.0, 169.0, &[]);
        assert!(c.x == 755.0 && c.y == 12.0);
        let d = fixed(2.0, 790.0, 169.0, &[]);
        assert!(d.x == 24.0 && tenths(d.y) == 735.8);
    }

    #[test]
    fn place_fixed_moves_off_a_chrome_plate_across_its_thin_side() {
        let caption = Rect {
            x: 20.0,
            y: 14.0,
            w: 162.0,
            h: 51.0,
        };
        let tools = Rect {
            x: 589.0,
            y: 726.0,
            w: 327.0,
            h: 52.0,
        };
        let a = fixed(60.0, 100.0, 169.0, &[caption, tools]);
        assert!(a.x == 82.0 && a.y == 65.0);
        let b = fixed(200.0, 100.0, 169.0, &[caption, tools]);
        assert!(b.x == 222.0 && tenths(b.y) == 45.8);
        let c = fixed(700.0, 780.0, 169.0, &[caption, tools]);
        assert!(c.x == 722.0 && c.y == 726.0 - 41.0);
        let tall = Rect {
            x: 860.0,
            y: 560.0,
            w: 56.0,
            h: 218.0,
        };
        let d = fixed(700.0, 700.0, 169.0, &[tall]);
        assert!(d.x == 860.0 - 169.0 && tenths(d.y) == 645.8);
    }

    #[test]
    fn curves_meet_counts_the_curve_within_the_margin() {
        let x2 = curve_polylines(R2, &curve_fn_of(square));
        let b = Rect {
            x: 500.0,
            y: 240.0,
            w: 48.0,
            h: 20.0,
        };
        assert!(curves_meet(b, &x2, 6.0));
        assert!(!curves_meet(b, &x2, 0.0));
    }

    #[test]
    fn edge_markers_point_the_way_a_point_lies_furthest() {
        assert!(edge_direction(500.0, 300.0, W, H).is_none());
        assert_eq!(edge_direction(568.0, -204.0, W, H).unwrap(), Direction::Up);
        assert_eq!(edge_direction(568.0, 900.0, W, H).unwrap(), Direction::Down);
        assert_eq!(edge_direction(-50.0, 300.0, W, H).unwrap(), Direction::Left);
        assert_eq!(
            edge_direction(1000.0, -10.0, W, H).unwrap(),
            Direction::Right
        );
        assert_eq!(edge_direction(-50.0, -60.0, W, H).unwrap(), Direction::Up);
    }

    #[test]
    fn edge_markers_sit_on_their_edge_24_px_from_the_corners() {
        let size = [108.0, 39.0];
        assert_eq!(
            edge_marker_box(Direction::Up, [568.0, -204.0], size, W, H, &[]),
            Rect {
                x: 514.0,
                y: 22.0,
                w: 108.0,
                h: 39.0
            },
        );
        assert_eq!(
            edge_marker_box(Direction::Up, [5.0, -204.0], size, W, H, &[]).x,
            24.0
        );
        let down = edge_marker_box(Direction::Down, [930.0, 900.0], size, W, H, &[]);
        assert!(down.x == W - 24.0 - 108.0 && down.y == 731.0);
        assert_eq!(
            edge_marker_box(Direction::Left, [-40.0, 100.0], size, W, H, &[]),
            Rect {
                x: 22.0,
                y: 80.5,
                w: 108.0,
                h: 39.0
            },
        );
        let right = edge_marker_box(Direction::Right, [990.0, 790.0], size, W, H, &[]);
        assert!(right.x == 806.0 && right.y == H - 24.0 - 39.0);
        let tools = Rect {
            x: 597.0,
            y: 734.0,
            w: 311.0,
            h: 36.0,
        };
        let b = edge_marker_box(Direction::Down, [700.0, 900.0], size, W, H, &[tools]);
        assert_eq!(b.y + b.h, H - 84.0);
    }
}
