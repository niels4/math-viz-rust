//! SVG path data ported from `svgpath.odin` (`Path_Op`, `Path_Step`,
//! `draw_path`, `read_path`, `arc_to`, `skip_separators`, `point`, `flag`,
//! `number`).
//!
//! Pure string parsing and geometry: no platform calls, so this module has
//! no external milestone needs. The emitter calls `mv_nvg::Context` path
//! methods, which `t0c` binds through `t0c-nanovg-sys`.

use mv_nvg::Context;

/// What a path step does. Ported from `Path_Op`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum PathOp {
    /// `M` / `m`: move the pen.
    Move,
    /// `L` / `l`, `H` / `h`, `V` / `v`: straight line.
    Line,
    /// `C` / `c`, `S` / `s`, `Q` / `q`, `A` / `a`: cubic curve.
    Cubic,
    /// `Z` / `z`: close the subpath.
    Close,
}

/// A path's pieces in absolute coordinates; a cubic's control points in `c1`
/// and `c2`. Ported from `Path_Step`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PathStep {
    /// What the step does.
    pub op: PathOp,
    /// Where the step ends (for `Close`, the subpath start to return to).
    pub to: [f32; 2],
    /// A cubic's first control point.
    pub c1: [f32; 2],
    /// A cubic's second control point.
    pub c2: [f32; 2],
}

/// SVG path data as NanoVG path calls. False when the data holds what
/// `read_path` doesn't read. Ported from `draw_path`; needs the `t0c` NanoVG
/// path bindings.
pub fn draw_path(vg: &mut Context, d: &str) -> bool {
    let mut steps = Vec::new();
    if !read_path(d, &mut steps) {
        return false;
    }
    for step in &steps {
        match step.op {
            PathOp::Move => vg.move_to(step.to[0], step.to[1]),
            PathOp::Line => vg.line_to(step.to[0], step.to[1]),
            PathOp::Cubic => vg.bezier_to(
                step.c1[0], step.c1[1], step.c2[0], step.c2[1], step.to[0], step.to[1],
            ),
            PathOp::Close => vg.close_path(),
        }
    }
    true
}

/// Reads SVG path data into `steps`: M L H V C S Q A Z, absolute and
/// relative, a command's numbers repeating it, numbers after a move drawing
/// lines, S reflecting the last cubic's second control point, Q raised to a
/// cubic, A as cubics. False at anything else (T) or at numbers out of place.
/// Ported from `read_path`.
pub fn read_path(d: &str, steps: &mut Vec<PathStep>) -> bool {
    let mut rest = d;
    let mut command: u8 = 0;
    let mut pen = [0.0f32; 2];
    let mut start = [0.0f32; 2];
    // The second control point of the cubic just drawn, for S.
    let mut last_c2: Option<[f32; 2]> = None;
    loop {
        skip_separators(&mut rest);
        if rest.is_empty() {
            return true;
        }
        let c = rest.as_bytes()[0];
        if c.is_ascii_alphabetic() {
            command = c;
            rest = &rest[1..];
        } else if command == 0 || command == b'Z' || command == b'z' {
            return false;
        }
        let relative = command >= b'a';
        let base = if relative { pen } else { [0.0; 2] };
        let mut cubic_c2: Option<[f32; 2]> = None;
        match command {
            b'M' | b'm' | b'L' | b'l' => {
                let Some(p) = point(&mut rest) else {
                    return false;
                };
                let p = [base[0] + p[0], base[1] + p[1]];
                if command == b'M' || command == b'm' {
                    steps.push(PathStep {
                        op: PathOp::Move,
                        to: p,
                        c1: [0.0; 2],
                        c2: [0.0; 2],
                    });
                    start = p;
                    command = if relative { b'l' } else { b'L' };
                } else {
                    steps.push(PathStep {
                        op: PathOp::Line,
                        to: p,
                        c1: [0.0; 2],
                        c2: [0.0; 2],
                    });
                }
                pen = p;
            }
            b'H' | b'h' => {
                let (x, ok) = number(&mut rest);
                if !ok {
                    return false;
                }
                pen[0] = if relative { pen[0] + x } else { x };
                steps.push(PathStep {
                    op: PathOp::Line,
                    to: pen,
                    c1: [0.0; 2],
                    c2: [0.0; 2],
                });
            }
            b'V' | b'v' => {
                let (y, ok) = number(&mut rest);
                if !ok {
                    return false;
                }
                pen[1] = if relative { pen[1] + y } else { y };
                steps.push(PathStep {
                    op: PathOp::Line,
                    to: pen,
                    c1: [0.0; 2],
                    c2: [0.0; 2],
                });
            }
            b'C' | b'c' => {
                let (c1, c2, to) = (point(&mut rest), point(&mut rest), point(&mut rest));
                let (Some(c1), Some(c2), Some(to)) = (c1, c2, to) else {
                    return false;
                };
                let (c1, c2, to) = (
                    [base[0] + c1[0], base[1] + c1[1]],
                    [base[0] + c2[0], base[1] + c2[1]],
                    [base[0] + to[0], base[1] + to[1]],
                );
                steps.push(PathStep {
                    op: PathOp::Cubic,
                    to,
                    c1,
                    c2,
                });
                pen = to;
                cubic_c2 = Some(c2);
            }
            b'S' | b's' => {
                // The first control point reflects the last cubic's second
                // about the pen, or is the pen after anything else.
                let mut c1 = pen;
                if let Some(c2) = last_c2 {
                    c1 = [2.0 * pen[0] - c2[0], 2.0 * pen[1] - c2[1]];
                }
                let (Some(c2), Some(to)) = (point(&mut rest), point(&mut rest)) else {
                    return false;
                };
                let (c2, to) = (
                    [base[0] + c2[0], base[1] + c2[1]],
                    [base[0] + to[0], base[1] + to[1]],
                );
                steps.push(PathStep {
                    op: PathOp::Cubic,
                    to,
                    c1,
                    c2,
                });
                pen = to;
                cubic_c2 = Some(c2);
            }
            b'Q' | b'q' => {
                let (Some(q), Some(to)) = (point(&mut rest), point(&mut rest)) else {
                    return false;
                };
                let (q, to) = (
                    [base[0] + q[0], base[1] + q[1]],
                    [base[0] + to[0], base[1] + to[1]],
                );
                steps.push(PathStep {
                    op: PathOp::Cubic,
                    to,
                    c1: [
                        pen[0] + (q[0] - pen[0]) * 2.0 / 3.0,
                        pen[1] + (q[1] - pen[1]) * 2.0 / 3.0,
                    ],
                    c2: [
                        to[0] + (q[0] - to[0]) * 2.0 / 3.0,
                        to[1] + (q[1] - to[1]) * 2.0 / 3.0,
                    ],
                });
                pen = to;
            }
            b'A' | b'a' => {
                let (rx, ok) = number(&mut rest);
                if !ok {
                    return false;
                }
                let (ry, ok) = number(&mut rest);
                if !ok {
                    return false;
                }
                let (rotation, ok) = number(&mut rest);
                if !ok {
                    return false;
                }
                let (large, ok) = flag(&mut rest);
                if !ok {
                    return false;
                }
                let (sweep, ok) = flag(&mut rest);
                if !ok {
                    return false;
                }
                let Some(to) = point(&mut rest) else {
                    return false;
                };
                let to = [base[0] + to[0], base[1] + to[1]];
                arc_to(steps, pen, [rx, ry], rotation, large, sweep, to);
                pen = to;
            }
            b'Z' | b'z' => {
                steps.push(PathStep {
                    op: PathOp::Close,
                    to: start,
                    c1: [0.0; 2],
                    c2: [0.0; 2],
                });
                pen = start;
            }
            _ => return false,
        }
        last_c2 = cubic_c2;
    }
}

/// An elliptical arc from `from` to `to` as cubics of a quarter turn or less:
/// the endpoint parameters to the centre's, out-of-range radii scaled up, a
/// zero radius a line. Ported from `arc_to`; runs in `f64` exactly like the
/// Odin original.
#[allow(clippy::too_many_arguments)]
fn arc_to(
    steps: &mut Vec<PathStep>,
    from: [f32; 2],
    radii: [f32; 2],
    rotation_deg: f32,
    large: bool,
    sweep: bool,
    to: [f32; 2],
) {
    if from == to {
        return;
    }
    let (mut rx, mut ry) = (f64::from(radii[0]).abs(), f64::from(radii[1]).abs());
    if rx == 0.0 || ry == 0.0 {
        steps.push(PathStep {
            op: PathOp::Line,
            to,
            c1: [0.0; 2],
            c2: [0.0; 2],
        });
        return;
    }
    let phi = f64::from(rotation_deg) * std::f64::consts::PI / 180.0;
    let (sin_phi, cos_phi) = phi.sin_cos();
    let (p0, p1) = (
        [f64::from(from[0]), f64::from(from[1])],
        [f64::from(to[0]), f64::from(to[1])],
    );
    let half = [(p0[0] - p1[0]) / 2.0, (p0[1] - p1[1]) / 2.0];
    let x1 = cos_phi * half[0] + sin_phi * half[1];
    let y1 = -sin_phi * half[0] + cos_phi * half[1];
    let lambda = x1 * x1 / (rx * rx) + y1 * y1 / (ry * ry);
    if lambda > 1.0 {
        let s = lambda.sqrt();
        rx *= s;
        ry *= s;
    }
    let num = rx * rx * ry * ry - rx * rx * y1 * y1 - ry * ry * x1 * x1;
    let den = rx * rx * y1 * y1 + ry * ry * x1 * x1;
    let mut k = (num / den).max(0.0).sqrt();
    if large == sweep {
        k = -k;
    }
    let (cx1, cy1) = (k * rx * y1 / ry, -k * ry * x1 / rx);
    let mid = [(p0[0] + p1[0]) / 2.0, (p0[1] + p1[1]) / 2.0];
    let centre = [
        cos_phi * cx1 - sin_phi * cy1 + mid[0],
        sin_phi * cx1 + cos_phi * cy1 + mid[1],
    ];
    let angle =
        |u: [f64; 2], v: [f64; 2]| (u[0] * v[1] - u[1] * v[0]).atan2(u[0] * v[0] + u[1] * v[1]);
    let u = [(x1 - cx1) / rx, (y1 - cy1) / ry];
    let v = [(-x1 - cx1) / rx, (-y1 - cy1) / ry];
    let theta = angle([1.0, 0.0], u);
    let mut delta = angle(u, v);
    if !sweep && delta > 0.0 {
        delta -= 2.0 * std::f64::consts::PI;
    } else if sweep && delta < 0.0 {
        delta += 2.0 * std::f64::consts::PI;
    }
    let at = |t: f64| -> ([f64; 2], [f64; 2]) {
        let (st, ct) = t.sin_cos();
        let p = [
            centre[0] + rx * ct * cos_phi - ry * st * sin_phi,
            centre[1] + rx * ct * sin_phi + ry * st * cos_phi,
        ];
        let tangent = [
            -rx * st * cos_phi - ry * ct * sin_phi,
            -rx * st * sin_phi + ry * ct * cos_phi,
        ];
        (p, tangent)
    };
    let segments = ((delta.abs() / (std::f64::consts::PI / 2.0) - 1e-9).ceil() as usize).max(1);
    let step = delta / segments as f64;
    let alpha = 4.0 / 3.0 * (step / 4.0).tan();
    for i in 0..segments {
        let t0 = theta + i as f64 * step;
        let (a, ta) = at(t0);
        let (b, tb) = at(t0 + step);
        let end = if i == segments - 1 {
            to
        } else {
            [b[0] as f32, b[1] as f32]
        };
        let c1 = [a[0] + alpha * ta[0], a[1] + alpha * ta[1]];
        let c2 = [b[0] - alpha * tb[0], b[1] - alpha * tb[1]];
        steps.push(PathStep {
            op: PathOp::Cubic,
            to: end,
            c1: [c1[0] as f32, c1[1] as f32],
            c2: [c2[0] as f32, c2[1] as f32],
        });
    }
}

/// Skips spaces, commas and tabs. Ported from `skip_separators`.
fn skip_separators(rest: &mut &str) {
    let mut n = 0usize;
    for c in rest.as_bytes() {
        if matches!(c, b' ' | b',' | b'\n' | b'\t') {
            n += 1;
        } else {
            break;
        }
    }
    *rest = &rest[n..];
}

/// One `x,y` point. Ported from `point`.
fn point(rest: &mut &str) -> Option<[f32; 2]> {
    let (x, ok) = number(rest);
    if !ok {
        return None;
    }
    let (y, ok) = number(rest);
    if !ok {
        return None;
    }
    Some([x, y])
}

/// An arc's flag: one character, 0 or 1, which may run into the number after
/// it. Ported from `flag`.
fn flag(rest: &mut &str) -> (bool, bool) {
    skip_separators(rest);
    match rest.as_bytes().first() {
        Some(b'0') => {
            *rest = &rest[1..];
            (false, true)
        }
        Some(b'1') => {
            *rest = &rest[1..];
            (true, true)
        }
        _ => (false, false),
    }
}

/// The next number in path data. A sign starts a new number unless it is an
/// exponent's, and so does a second point. Ported from `number`; the scan
/// only moves past the token on success.
fn number(rest: &mut &str) -> (f32, bool) {
    skip_separators(rest);
    let bytes = rest.as_bytes();
    let mut end = 0usize;
    let mut seen_point = false;
    while end < bytes.len() {
        match bytes[end] {
            b'0'..=b'9' | b'e' | b'E' => end += 1,
            b'-' | b'+' => {
                if end > 0 && bytes[end - 1] != b'e' && bytes[end - 1] != b'E' {
                    break;
                }
                end += 1;
            }
            b'.' => {
                if seen_point {
                    break;
                }
                seen_point = true;
                end += 1;
            }
            _ => break,
        }
    }
    match rest[..end].parse::<f32>() {
        Ok(v) => {
            *rest = &rest[end..];
            (v, true)
        }
        Err(_) => (0.0, false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icons::SNOWFLAKE_PATHS;

    // Parser test data: path strings with arcs, implicit commands and decimals.
    const GEAR_PATH: &str = "M19.4 13.5a7.5 7.5 0 0 0 0-3l2-1.5-2-3.4-2.3 1a7.6 7.6 0 0 0-2.6-1.5L14 2h-4l-.5 2.6a7.6 7.6 0 0 0-2.6 1.5l-2.3-1-2 3.4 2 1.5a7.5 7.5 0 0 0 0 3l-2 1.5 2 3.4 2.3-1a7.6 7.6 0 0 0 2.6 1.5L10 22h4l.5-2.6a7.6 7.6 0 0 0 2.6-1.5l2.3 1 2-3.4-2-1.5z";
    const SYSTEM_STAND: &str = "M8 11v2.75M5.25 13.75h5.5";
    const SUN_RAYS: &str = "M13.2 8h1.6M11.68 11.68l1.13 1.13M8 13.2v1.6M4.32 11.68l-1.13 1.13M2.8 8H1.2M4.32 4.32 3.19 3.19M8 2.8V1.2M11.68 4.32l1.13-1.13";
    const MOON_PATH: &str = "M13.6 8.54A6 6 0 1 1 7.46 2.4 4.6 4.6 0 0 0 13.6 8.54Z";
    const CHECK_PATH: &str = "M2.5 7.5L5.5 10.5L11.5 3.5";

    // A step as the tests write it: what it does and where it ends.
    struct Step {
        op: PathOp,
        to: [f32; 2],
    }

    fn expect_path(d: &str, want: &[Step]) {
        let mut steps = Vec::new();
        assert!(read_path(d, &mut steps), "{d:?} didn't read");
        assert_eq!(steps.len(), want.len(), "{d:?}: {} steps", steps.len());
        for (step, w) in steps.iter().zip(want.iter()) {
            let close = (step.to[0] - w.to[0]).abs() < 1e-5 && (step.to[1] - w.to[1]).abs() < 1e-5;
            assert!(
                step.op == w.op && close,
                "{d:?}: {step:?}, want {:?} at {:?}",
                w.op,
                w.to
            );
        }
    }

    fn near_point(a: [f32; 2], b: [f32; 2]) -> bool {
        (a[0] - b[0]).abs() < 1e-4 && (a[1] - b[1]).abs() < 1e-4
    }

    // The snowflake's two paths, absolute and relative.
    #[test]
    fn snowflake_paths_read() {
        expect_path(
            SNOWFLAKE_PATHS[0],
            &[
                Step {
                    op: PathOp::Move,
                    to: [12.0, 2.0],
                },
                Step {
                    op: PathOp::Line,
                    to: [12.0, 22.0],
                },
                Step {
                    op: PathOp::Move,
                    to: [3.3, 7.0],
                },
                Step {
                    op: PathOp::Line,
                    to: [20.7, 17.0],
                },
                Step {
                    op: PathOp::Move,
                    to: [20.7, 7.0],
                },
                Step {
                    op: PathOp::Line,
                    to: [3.3, 17.0],
                },
            ],
        );
        expect_path(
            SNOWFLAKE_PATHS[1],
            &[
                Step {
                    op: PathOp::Move,
                    to: [12.0, 2.0],
                },
                Step {
                    op: PathOp::Line,
                    to: [10.0, 4.5],
                },
                Step {
                    op: PathOp::Move,
                    to: [12.0, 2.0],
                },
                Step {
                    op: PathOp::Line,
                    to: [14.0, 4.5],
                },
                Step {
                    op: PathOp::Move,
                    to: [12.0, 22.0],
                },
                Step {
                    op: PathOp::Line,
                    to: [10.0, 19.5],
                },
                Step {
                    op: PathOp::Move,
                    to: [12.0, 22.0],
                },
                Step {
                    op: PathOp::Line,
                    to: [14.0, 19.5],
                },
            ],
        );
    }

    // Numbers run together as SVG lets them, a move's extra numbers draw
    // lines, and Z goes back to where the subpath began.
    #[test]
    fn path_numbers_and_commands() {
        expect_path(
            "M1-2-3.5.5",
            &[
                Step {
                    op: PathOp::Move,
                    to: [1.0, -2.0],
                },
                Step {
                    op: PathOp::Line,
                    to: [-3.5, 0.5],
                },
            ],
        );
        expect_path(
            "m1,1 2,0 0,2z",
            &[
                Step {
                    op: PathOp::Move,
                    to: [1.0, 1.0],
                },
                Step {
                    op: PathOp::Line,
                    to: [3.0, 1.0],
                },
                Step {
                    op: PathOp::Line,
                    to: [3.0, 3.0],
                },
                Step {
                    op: PathOp::Close,
                    to: [1.0, 1.0],
                },
            ],
        );
        expect_path(
            "M0 0H5V5h-5z",
            &[
                Step {
                    op: PathOp::Move,
                    to: [0.0, 0.0],
                },
                Step {
                    op: PathOp::Line,
                    to: [5.0, 0.0],
                },
                Step {
                    op: PathOp::Line,
                    to: [5.0, 5.0],
                },
                Step {
                    op: PathOp::Line,
                    to: [0.0, 5.0],
                },
                Step {
                    op: PathOp::Close,
                    to: [0.0, 0.0],
                },
            ],
        );
        expect_path(
            "M1e1 2E0",
            &[Step {
                op: PathOp::Move,
                to: [10.0, 2.0],
            }],
        );
        let mut steps = Vec::new();
        assert!(!read_path("M0 0T1 1", &mut steps), "T isn't read");
        assert!(!read_path("1 1", &mut steps), "numbers before any command");
    }

    // Cubics as given; S reflects the last cubic's second control point about
    // the pen, or starts at the pen after anything else; Q raised to a cubic.
    #[test]
    fn path_curves() {
        let mut steps = Vec::new();
        assert!(read_path(
            "M0 0C1 2 3 4 5 6s7 8 9 10L0 0S1 1 2 2",
            &mut steps
        ));
        assert_eq!(steps.len(), 5);
        assert_eq!(
            steps[1],
            PathStep {
                op: PathOp::Cubic,
                to: [5.0, 6.0],
                c1: [1.0, 2.0],
                c2: [3.0, 4.0]
            }
        );
        assert_eq!(
            steps[2],
            PathStep {
                op: PathOp::Cubic,
                to: [14.0, 16.0],
                c1: [7.0, 8.0],
                c2: [12.0, 14.0]
            }
        );
        assert_eq!(
            steps[4],
            PathStep {
                op: PathOp::Cubic,
                to: [2.0, 2.0],
                c1: [0.0, 0.0],
                c2: [1.0, 1.0]
            }
        );
        steps.clear();
        assert!(read_path("M0 0Q3 3 6 0", &mut steps));
        assert_eq!(steps.len(), 2);
        assert_eq!(
            steps[1],
            PathStep {
                op: PathOp::Cubic,
                to: [6.0, 0.0],
                c1: [2.0, 2.0],
                c2: [4.0, 2.0]
            }
        );
    }

    // Arcs as cubics of a quarter turn or less: a half circle in two, and the
    // gear's outline, whose steps land where the SVG's do.
    #[test]
    fn path_arcs() {
        let mut steps = Vec::new();
        assert!(read_path("M0 0A1 1 0 0 1 2 0", &mut steps));
        assert_eq!(steps.len(), 3);
        assert_eq!(steps[2].to, [2.0, 0.0]);
        // The first quarter ends at the top, (1, -1), its control points
        // 0.5523 along the tangents.
        #[allow(clippy::excessive_precision)]
        const K: f32 = 0.55228475;
        assert!(
            near_point(steps[1].to, [1.0, -1.0])
                && near_point(steps[1].c1, [0.0, -K])
                && near_point(steps[1].c2, [1.0 - K, -1.0]),
            "the first quarter: {:?}",
            steps[1]
        );
        steps.clear();
        assert!(read_path(GEAR_PATH, &mut steps));
        // The outline's corners, absolute: each arc's end, each line's.
        let want: &[[f32; 2]] = &[
            [19.4, 13.5],
            [19.4, 10.5],
            [21.4, 9.0],
            [19.4, 5.6],
            [17.1, 6.6],
            [14.5, 5.1],
            [14.0, 2.0],
            [10.0, 2.0],
            [9.5, 4.6],
            [6.9, 6.1],
            [4.6, 5.1],
            [2.6, 8.5],
            [4.6, 10.0],
            [4.6, 13.0],
            [2.6, 14.5],
            [4.6, 17.9],
            [6.9, 16.9],
            [9.5, 18.4],
            [10.0, 22.0],
            [14.0, 22.0],
            [14.5, 19.4],
            [17.1, 17.9],
            [19.4, 18.9],
            [21.4, 15.5],
            [19.4, 14.0],
        ];
        let corners: Vec<[f32; 2]> = steps
            .iter()
            .filter(|s| s.op != PathOp::Close)
            .map(|s| s.to)
            .collect();
        assert_eq!(corners.len(), want.len());
        for (i, (c, w)) in corners.iter().zip(want.iter()).enumerate() {
            assert!(
                near_point(*c, *w),
                "corner {i} at {c:?}, the SVG's at {w:?}"
            );
        }
        assert_eq!(steps.last().unwrap().op, PathOp::Close);
    }

    // The settings menu's marks: the sun's rays as eight lines, one of them a
    // move's extra numbers; the moon's two arcs end where its path says, the
    // second closing on the first's start.
    #[test]
    fn settings_icon_paths_read() {
        expect_path(
            SYSTEM_STAND,
            &[
                Step {
                    op: PathOp::Move,
                    to: [8.0, 11.0],
                },
                Step {
                    op: PathOp::Line,
                    to: [8.0, 13.75],
                },
                Step {
                    op: PathOp::Move,
                    to: [5.25, 13.75],
                },
                Step {
                    op: PathOp::Line,
                    to: [10.75, 13.75],
                },
            ],
        );
        expect_path(
            SUN_RAYS,
            &[
                Step {
                    op: PathOp::Move,
                    to: [13.2, 8.0],
                },
                Step {
                    op: PathOp::Line,
                    to: [14.8, 8.0],
                },
                Step {
                    op: PathOp::Move,
                    to: [11.68, 11.68],
                },
                Step {
                    op: PathOp::Line,
                    to: [12.81, 12.81],
                },
                Step {
                    op: PathOp::Move,
                    to: [8.0, 13.2],
                },
                Step {
                    op: PathOp::Line,
                    to: [8.0, 14.8],
                },
                Step {
                    op: PathOp::Move,
                    to: [4.32, 11.68],
                },
                Step {
                    op: PathOp::Line,
                    to: [3.19, 12.81],
                },
                Step {
                    op: PathOp::Move,
                    to: [2.8, 8.0],
                },
                Step {
                    op: PathOp::Line,
                    to: [1.2, 8.0],
                },
                Step {
                    op: PathOp::Move,
                    to: [4.32, 4.32],
                },
                Step {
                    op: PathOp::Line,
                    to: [3.19, 3.19],
                },
                Step {
                    op: PathOp::Move,
                    to: [8.0, 2.8],
                },
                Step {
                    op: PathOp::Line,
                    to: [8.0, 1.2],
                },
                Step {
                    op: PathOp::Move,
                    to: [11.68, 4.32],
                },
                Step {
                    op: PathOp::Line,
                    to: [12.81, 3.19],
                },
            ],
        );
        expect_path(
            CHECK_PATH,
            &[
                Step {
                    op: PathOp::Move,
                    to: [2.5, 7.5],
                },
                Step {
                    op: PathOp::Line,
                    to: [5.5, 10.5],
                },
                Step {
                    op: PathOp::Line,
                    to: [11.5, 3.5],
                },
            ],
        );
        let mut steps = Vec::new();
        assert!(read_path(MOON_PATH, &mut steps));
        assert!(steps[0].op == PathOp::Move && near_point(steps[0].to, [13.6, 8.54]));
        assert_eq!(steps.last().unwrap().op, PathOp::Close);
        // The big arc (radius 6, the long way) ends at (7.46, 2.4); the small
        // one (4.6) back at the start.
        let (mut found_mid, mut found_end) = (false, false);
        for step in &steps {
            if step.op == PathOp::Cubic {
                found_mid = found_mid || near_point(step.to, [7.46, 2.4]);
                found_end = found_end || near_point(step.to, [13.6, 8.54]);
            }
        }
        assert!(found_mid, "the big arc ends at (7.46, 2.4)");
        assert!(found_end, "the small arc ends back at (13.6, 8.54)");
    }
}
