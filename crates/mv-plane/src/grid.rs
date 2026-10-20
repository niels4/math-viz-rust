//! Grid lines, ticks and labels laid out for a viewport.

use super::faces::{Measure, ORIGIN_FACE, TICK_LABEL_FACE, js_round, line_box, measure};
use super::rect::{Rect, inside_box, intersects};
use super::viewport::{Viewport, to_screen_x, to_screen_y, visible_extent};
use mv_numfmt::{format_tick, precision};

/// The closest grid lines may come, in px.
pub const MIN_GRID_PX: f64 = 20.0;

/// Half the tick length in px.
pub const TICK_HALF: f64 = 5.0;

/// The gap between a tick and its label, in px.
const LABEL_GAP_X: f64 = 9.0;

/// The gap between a tick and its label, in px.
const LABEL_GAP_Y: f64 = 10.0;

/// How far labels keep from the vertical edges, in px.
const LABEL_EDGE_X: f64 = 16.0;

/// How far labels keep from the horizontal edges, in px.
const LABEL_EDGE_Y: f64 = 14.0;

/// The origin label's gap in x, in px.
const ORIGIN_GAP_X: f64 = 8.0;

/// The origin label's gap in y, in px.
const ORIGIN_GAP_Y: f64 = 5.0;

/// The padding labels keep from the edges, in px.
const EDGE_PAD: f64 = 8.0;

/// What counts as on a lattice line.
const EPS: f64 = 1e-9;

/// The grid steps at a zoom: minor and major.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridSteps {
    pub minor: f64,
    pub major: f64,
}

/// A grid axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
}

/// One grid label: its axis, text and box.
#[derive(Clone, Debug, PartialEq)]
pub struct GridLabel {
    pub axis: Axis,
    pub text: String,
    pub r#box: Rect,
}

/// The grid lines of a viewport.
#[derive(Clone, Debug, PartialEq)]
pub struct GridLines {
    pub minor_x: Vec<f64>,
    pub major_x: Vec<f64>,
    pub minor_y: Vec<f64>,
    pub major_y: Vec<f64>,
    pub axis_x: Option<f64>,
    pub axis_y: Option<f64>,
}

/// A laid-out grid: its lines, steps, ticks, labels and origin.
#[derive(Clone, Debug, PartialEq)]
pub struct GridLayout {
    pub minor_x: Vec<f64>,
    pub major_x: Vec<f64>,
    pub minor_y: Vec<f64>,
    pub major_y: Vec<f64>,
    pub axis_x: Option<f64>,
    pub axis_y: Option<f64>,
    pub steps: GridSteps,
    pub ticks_x: Vec<f64>,
    pub ticks_y: Vec<f64>,
    pub labels: Vec<GridLabel>,
    pub origin: Option<Rect>,
}

/// The smallest 1, 2 or 5 x 10^n at or above a minimum.
fn step125(min: f64) -> f64 {
    let magnitude = 10f64.powf(min.log10().floor());
    let m = min / magnitude;
    let factor = if m <= 1.0 + EPS {
        1.0
    } else if m <= 2.0 + EPS {
        2.0
    } else if m <= 5.0 + EPS {
        5.0
    } else {
        10.0
    };
    precision(factor * magnitude, 12)
}

/// The grid steps at a zoom.
pub fn grid_steps(zoom: f64) -> GridSteps {
    let minor = step125(MIN_GRID_PX / zoom);
    GridSteps {
        minor,
        major: step125(minor * 1.5),
    }
}

/// The multiples of a step in a range.
fn multiples(lo: f64, hi: f64, step: f64) -> Vec<f64> {
    const MAX_SAFE: f64 = 9007199254740991.0;
    let mut out = Vec::new();
    let mut n = (lo / step - EPS).ceil();
    while n.abs() <= MAX_SAFE && n * step <= hi + step * EPS {
        out.push(precision(n * step, 12));
        n += 1.0;
    }
    out
}

/// Whether a value sits on a step's lattice.
fn on_step(u: f64, step: f64) -> bool {
    (u / step - js_round(u / step)).abs() < 1e-6
}

/// Whether a value falls strictly between two bounds.
fn between(v: f64, lo: f64, hi: f64) -> bool {
    v >= lo && v <= hi
}

/// The lattice lines through a viewport, major and minor.
struct Lattice {
    xs: Vec<f64>,
    ys: Vec<f64>,
    major_xs: Vec<f64>,
    major_ys: Vec<f64>,
}

/// The lattice lines through a viewport at grid steps.
fn lattice(vp: Viewport, steps: GridSteps) -> Lattice {
    let extent = visible_extent(vp);
    let xs = without_zero(multiples(extent.min_x, extent.max_x, steps.minor));
    let ys = without_zero(multiples(extent.min_y, extent.max_y, steps.minor));
    let major_xs = keep(xs.clone(), steps.major, true);
    let major_ys = keep(ys.clone(), steps.major, true);
    Lattice {
        xs,
        ys,
        major_xs,
        major_ys,
    }
}

/// Drops the zero line from a set of values.
fn without_zero(values: Vec<f64>) -> Vec<f64> {
    values.into_iter().filter(|v| *v != 0.0).collect()
}

/// Keeps the lines a filter leaves: major lines, or minor lines when on.
fn keep(values: Vec<f64>, step: f64, on: bool) -> Vec<f64> {
    values
        .into_iter()
        .filter(|v| on_step(*v, step) == on)
        .collect()
}

/// The grid lines of a viewport at grid steps.
pub fn grid_lines(vp: Viewport, steps: GridSteps) -> GridLines {
    let l = lattice(vp, steps);
    let screen = |vp: Viewport, us: &[f64], y: bool| -> Vec<f64> {
        us.iter()
            .map(|u| {
                if y {
                    to_screen_y(vp, *u)
                } else {
                    to_screen_x(vp, *u)
                }
            })
            .collect()
    };
    GridLines {
        minor_x: screen(vp, &keep(l.xs.clone(), steps.major, false), false),
        major_x: screen(vp, &l.major_xs, false),
        minor_y: screen(vp, &keep(l.ys.clone(), steps.major, false), true),
        major_y: screen(vp, &l.major_ys, true),
        axis_x: if between(vp.origin_y, 0.0, vp.height) {
            Some(vp.origin_y)
        } else {
            None
        },
        axis_y: if between(vp.origin_x, 0.0, vp.width) {
            Some(vp.origin_x)
        } else {
            None
        },
    }
}

/// Lays out the grid for a viewport, keeping out of the given boxes.
pub fn layout_grid(vp: Viewport, m: &mut Measure<'_>, keep_out: &[Rect]) -> GridLayout {
    let (width, height) = (vp.width, vp.height);
    let (origin_x, origin_y) = (vp.origin_x, vp.origin_y);
    let steps = grid_steps(vp.zoom);
    let l = lattice(vp, steps);
    let lines = grid_lines(vp, steps);
    let clear = |b: Rect| -> bool { keep_out.iter().all(|r| !intersects(b, *r)) };
    let mut grid = GridLayout {
        minor_x: lines.minor_x,
        major_x: lines.major_x,
        minor_y: lines.minor_y,
        major_y: lines.major_y,
        axis_x: lines.axis_x,
        axis_y: lines.axis_y,
        steps,
        ticks_x: Vec::new(),
        ticks_y: Vec::new(),
        labels: Vec::new(),
        origin: None,
    };
    let box_h = line_box(TICK_LABEL_FACE);
    let below = origin_y + LABEL_GAP_X;
    let top = if origin_y < 0.0 {
        EDGE_PAD
    } else if origin_y > height {
        height - EDGE_PAD - box_h
    } else if below + box_h <= height - EDGE_PAD {
        below
    } else {
        origin_y - LABEL_GAP_X - box_h
    };
    let mut labels = Vec::new();
    for u in &l.major_xs {
        let cx = to_screen_x(vp, *u);
        if !between(cx, LABEL_EDGE_X, width - LABEL_EDGE_X) {
            continue;
        }
        let text = format_tick(*u);
        let w = measure(&mut *m, TICK_LABEL_FACE, &text);
        let label = GridLabel {
            axis: Axis::X,
            text,
            r#box: Rect {
                x: cx - w / 2.0,
                y: top,
                w,
                h: box_h,
            },
        };
        if clear(label.r#box) {
            labels.push(label);
        }
    }
    let x_labels = labels.len();

    struct YText {
        u: f64,
        text: String,
        width: f64,
    }
    let mut y_texts = Vec::new();
    let mut widest: f64 = 0.0;
    for u in &l.major_ys {
        if !between(to_screen_y(vp, *u), LABEL_EDGE_Y, height - LABEL_EDGE_Y) {
            continue;
        }
        let text = format_tick(*u);
        let w = measure(&mut *m, TICK_LABEL_FACE, &text);
        widest = widest.max(w);
        y_texts.push(YText {
            u: *u,
            text,
            width: w,
        });
    }
    let left_aligned =
        origin_x < 0.0 || (origin_x <= width && origin_x - LABEL_GAP_Y - widest < EDGE_PAD);
    let anchor = if origin_x < 0.0 {
        EDGE_PAD
    } else if origin_x > width {
        width - EDGE_PAD
    } else if left_aligned {
        origin_x + LABEL_GAP_Y
    } else {
        origin_x - LABEL_GAP_Y
    };
    'y_loop: for y in y_texts {
        let b = Rect {
            x: if left_aligned {
                anchor
            } else {
                anchor - y.width
            },
            y: to_screen_y(vp, y.u) - box_h / 2.0,
            w: y.width,
            h: box_h,
        };
        if !clear(b) {
            continue;
        }
        for x in labels.iter().take(x_labels) {
            if intersects(x.r#box, b) {
                continue 'y_loop;
            }
        }
        labels.push(GridLabel {
            axis: Axis::Y,
            text: y.text,
            r#box: b,
        });
    }
    grid.labels = labels;

    if grid.axis_x.is_some() {
        grid.ticks_x = l.major_xs.iter().map(|u| to_screen_x(vp, *u)).collect();
    }
    if grid.axis_y.is_some() {
        grid.ticks_y = l.major_ys.iter().map(|u| to_screen_y(vp, *u)).collect();
    }
    if grid.axis_x.is_some() && grid.axis_y.is_some() {
        let w = measure(&mut *m, ORIGIN_FACE, "O");
        let b = Rect {
            x: origin_x - ORIGIN_GAP_X - w,
            y: origin_y + ORIGIN_GAP_Y,
            w,
            h: line_box(ORIGIN_FACE),
        };
        if inside_box(b, width, height) && clear(b) {
            grid.origin = Some(b);
        }
    }
    grid
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::super::faces::{Face, Measure, ORIGIN_FACE};
    use super::super::viewport::{ViewState, make_viewport};
    use super::*;
    use mv_numfmt::{DISPLAY_DP, format_mark};

    fn board_measure(face: Face, s: &str) -> f64 {
        if face == ORIGIN_FACE {
            11.0
        } else {
            s.chars().count() as f64 * 8.4
        }
    }

    fn layout(
        width: f64,
        height: f64,
        zoom: f64,
        pan_x: f64,
        pan_y: f64,
        keep_out: &[Rect],
    ) -> GridLayout {
        let vp = make_viewport(width, height, 1.0, ViewState { zoom, pan_x, pan_y });
        let mut m = Measure {
            f: Box::new(board_measure),
        };
        layout_grid(vp, &mut m, keep_out)
    }

    fn texts(grid: &GridLayout, axis: Axis) -> Vec<String> {
        grid.labels
            .iter()
            .filter(|l| l.axis == axis)
            .map(|l| l.text.clone())
            .collect()
    }

    fn label(grid: &GridLayout, axis: Axis, text: &str) -> Option<GridLabel> {
        grid.labels
            .iter()
            .find(|l| l.axis == axis && l.text == text)
            .cloned()
    }

    fn numbers(from: i64, to: i64) -> Vec<String> {
        (from..=to)
            .filter(|n| *n != 0)
            .map(|n| format_mark(n as f64, DISPLAY_DP))
            .collect()
    }

    fn same(got: &[String], want: &[String]) {
        assert_eq!(got.join(","), want.join(","), "got {got:?}, want {want:?}");
    }

    fn has(values: &[f64], v: f64) -> bool {
        values.contains(&v)
    }

    #[test]
    fn grid_steps_are_on_the_1_2_5_steps() {
        assert_eq!(
            grid_steps(50.0),
            GridSteps {
                minor: 0.5,
                major: 1.0
            }
        );
        assert_eq!(
            grid_steps(40.0),
            GridSteps {
                minor: 0.5,
                major: 1.0
            }
        );
        assert_eq!(
            grid_steps(100.0),
            GridSteps {
                minor: 0.2,
                major: 0.5
            }
        );
        assert_eq!(
            grid_steps(20.0),
            GridSteps {
                minor: 1.0,
                major: 2.0
            }
        );
        assert_eq!(
            grid_steps(2.0),
            GridSteps {
                minor: 10.0,
                major: 20.0
            }
        );
        assert_eq!(
            grid_steps(0.25),
            GridSteps {
                minor: 100.0,
                major: 200.0
            }
        );
    }

    #[test]
    fn the_grid_lays_out_r2s_plane() {
        let grid = layout(936.0, 792.0, 50.0, 0.0, 0.0, &[]);
        assert_eq!(grid.axis_x.unwrap(), 396.0);
        assert_eq!(grid.axis_y.unwrap(), 468.0);
        same(&texts(&grid, Axis::X), &numbers(-9, 9));
        same(&texts(&grid, Axis::Y), &numbers(-7, 7));
        assert!(has(&grid.major_x, 518.0));
        assert!(has(&grid.minor_x, 493.0));
        assert!(!has(&grid.minor_x, 468.0));
        assert_eq!(grid.ticks_x.len(), 18);
        assert_eq!(grid.ticks_y.len(), 14);
    }

    #[test]
    fn labels_and_the_o_sit_where_the_design_puts_them() {
        let grid = layout(936.0, 792.0, 50.0, 0.0, 0.0, &[]);
        let x1 = label(&grid, Axis::X, "1").expect("x label 1");
        assert!((x1.r#box.x - (518.0 - 4.2)).abs() < 1e-9);
        assert_eq!(x1.r#box.y, 405.0);
        let ym7 = label(&grid, Axis::Y, &format_mark(-7.0, DISPLAY_DP)).expect("y label -7");
        assert_eq!(ym7.r#box.x + ym7.r#box.w, 458.0);
        assert!((ym7.r#box.y + ym7.r#box.h / 2.0 - 746.0).abs() < 1e-9);
        let o = grid.origin.expect("origin");
        assert_eq!(o.x, 449.0);
        assert_eq!(o.y, 401.0);
        assert_eq!(o.w, 11.0);
    }

    #[test]
    fn the_grid_lays_out_r9s_plane() {
        let grid = layout(1256.0, 408.0, 40.0, 0.0, 0.0, &[]);
        let xs = texts(&grid, Axis::X);
        assert_eq!(xs.len(), 30);
        assert_eq!(xs[0], format_mark(-15.0, DISPLAY_DP));
        same(&texts(&grid, Axis::Y), &numbers(-4, 4));
    }

    #[test]
    fn labels_under_a_keep_out_box_are_skipped() {
        let grid = layout(
            936.0,
            792.0,
            50.0,
            0.0,
            0.0,
            &[Rect {
                x: 364.5,
                y: 382.0,
                w: 57.0,
                h: 28.0,
            }],
        );
        let has_m2 = label(&grid, Axis::X, &format_mark(-2.0, DISPLAY_DP)).is_some();
        let has_m1 = label(&grid, Axis::X, &format_mark(-1.0, DISPLAY_DP)).is_some();
        let has_m3 = label(&grid, Axis::X, &format_mark(-3.0, DISPLAY_DP)).is_some();
        assert!(!has_m2 && !has_m1 && has_m3);
    }

    #[test]
    fn labels_keep_to_the_edge_nearest_an_axis_out_of_view() {
        let above = layout(936.0, 792.0, 50.0, 0.0, 10.0, &[]);
        assert!(above.axis_x.is_none());
        assert_eq!(above.ticks_x.len(), 0);
        assert!(above.origin.is_none());
        let x1 = label(&above, Axis::X, "1").expect("x label 1");
        assert_eq!(x1.r#box.y, 8.0);
        let left = layout(936.0, 792.0, 50.0, -12.0, 0.0, &[]);
        assert!(left.axis_y.is_none());
        let y3 = label(&left, Axis::Y, "3").expect("y label 3");
        assert_eq!(y3.r#box.x, 8.0);
    }

    #[test]
    fn x_labels_go_above_an_axis_along_the_bottom_edge() {
        let grid = layout(936.0, 792.0, 50.0, 0.0, -7.72, &[]);
        assert_eq!(grid.axis_x.unwrap(), 782.0);
        let x1 = label(&grid, Axis::X, "1").expect("x label 1");
        assert!((x1.r#box.y + x1.r#box.h - (782.0 - 9.0)).abs() < 1e-9);
    }

    #[test]
    fn a_mini_plot_draws_its_grid_on_fixed_steps() {
        let vp = Viewport {
            width: 268.0,
            height: 124.0,
            zoom: 22.0,
            origin_x: 134.0,
            origin_y: 112.0,
        };
        let lines = grid_lines(
            vp,
            GridSteps {
                minor: 1.0,
                major: 5.0,
            },
        );
        expect_floats(&lines.major_x, &[24.0, 244.0]);
        expect_floats(
            &lines.minor_x,
            &[
                2.0, 46.0, 68.0, 90.0, 112.0, 156.0, 178.0, 200.0, 222.0, 266.0,
            ],
        );
        expect_floats(&lines.minor_y, &[90.0, 68.0, 46.0, 24.0]);
        expect_floats(&lines.major_y, &[2.0]);
        assert_eq!(lines.axis_x.unwrap(), 112.0);
        assert_eq!(lines.axis_y.unwrap(), 134.0);
    }

    fn expect_floats(got: &[f64], want: &[f64]) {
        assert_eq!(got.len(), want.len(), "got {got:?}, want {want:?}");
        for (g, w) in got.iter().zip(want.iter()) {
            assert!((g - w).abs() < 1e-9, "got {got:?}, want {want:?}");
        }
    }

    #[test]
    fn the_grid_ends_far_from_the_origin() {
        for y in [1e17, -1e17, 1e34, 1e300] {
            let vp = make_viewport(
                800.0,
                600.0,
                1.0,
                ViewState {
                    zoom: 50.0,
                    pan_x: 0.0,
                    pan_y: -y,
                },
            );
            let lines = grid_lines(vp, grid_steps(50.0));
            assert!(lines.minor_y.len() + lines.major_y.len() <= 600 / 20 + 1);
            assert!(lines.minor_x.len() + lines.major_x.len() <= 800 / 20 + 1);
        }
    }
}
