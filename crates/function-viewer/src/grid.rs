//! The Cartesian plane's grid in screen px, laid out from the camera: minor
//! lines on the 1-2-5 step that keeps them at least [`MIN_GRID_PX`] apart,
//! major lines on the next step up. No canvas here, so it tests without one.
//!
//! Ported from `src/components/views/CartesianPlane/grid.ts` at 251c080.
//! `unscaledUnitSize` (25) is confirmed from `constants.ts` at the same hash.
//! Since `1cfd34a` the layout carries the canvas width, height and scale it
//! was laid out for, so the label functions read them from the grid.

use mv_numfmt::precision;

use crate::camera::UNSCALED_UNIT_SIZE;

/// Minor grid lines stay at least this many px apart.
const MIN_GRID_PX: f64 = 20.0;

/// Neighbouring x labels closer than this read as crowded.
const MIN_LABEL_GAP: f64 = 12.0;

/// x labels end this far left of their line, and y labels this far left of the y-axis.
const LABEL_GAP_X: f64 = 6.0;

/// x labels' tops sit this far below the x-axis, and each y label's top this far below its line.
const LABEL_GAP_Y: f64 = 6.0;

/// With its axis near or past an edge, a label row stays this far inside it.
const LABEL_EDGE_PAD: f64 = 8.0;

// One plane unit in screen px at scale 1 is `camera::UNSCALED_UNIT_SIZE`
// (`constants.ts` single-sources it; `camera.rs` owns the port).

const EPS: f64 = 1e-9;

/// The largest integer an f64 holds exactly (JS `Number.MAX_SAFE_INTEGER`).
const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;

/// Minor and major grid steps in plane units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridSteps {
    pub minor: f64,
    pub major: f64,
}

/// A grid line at plane value `unit`, drawn at screen x or y `px`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridLine {
    pub unit: f64,
    pub px: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GridLayout {
    pub steps: GridSteps,
    pub px_per_unit: f64,
    /// Vertical lines by screen x, horizontal lines by screen y. The axes are
    /// drawn apart, so 0 is left out.
    pub minor_x: Vec<GridLine>,
    pub major_x: Vec<GridLine>,
    pub minor_y: Vec<GridLine>,
    pub major_y: Vec<GridLine>,
    /// Screen x of the y-axis (x = 0) and screen y of the x-axis (y = 0),
    /// `None` while off screen and past the margin.
    pub x_origin: Option<f64>,
    pub y_origin: Option<f64>,
    /// Screen x and y of the origin, on screen or not.
    pub origin: [f64; 2],
    /// The canvas width, height and camera scale the layout was computed
    /// for (`1cfd34a` lays the grid out once per camera update and hands the
    /// layout to the painters, the legend and the hit test).
    pub width: f64,
    pub height: f64,
    pub scale: f64,
}

/// Which axis a [`GridLabel`] belongs to (TS `"x" | "y"`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GridAxis {
    X,
    Y,
}

/// A label's axis and text, the screen x of its right edge and the screen y of its top.
#[derive(Clone, Debug, PartialEq)]
pub struct GridLabel {
    pub axis: GridAxis,
    pub text: String,
    pub right: f64,
    pub top: f64,
}

/// Rounds away float noise, so 3 x 0.2 reads as 0.6, not 0.6000000000000001
/// (JS `Number(v.toPrecision(12))`, through `mv-numfmt`).
fn clean(v: f64) -> f64 {
    precision(v, 12)
}

/// Smallest 1, 2 or 5 x 10^n at or above min.
fn step_125(min: f64) -> f64 {
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
    clean(factor * magnitude)
}

/// Minor 0.5 and major 1 at the default 50 px per unit.
fn grid_steps(px_per_unit: f64) -> GridSteps {
    let minor = step_125(MIN_GRID_PX / px_per_unit);
    // 1.5 minors rounds up to the next rung: 1 to 2, 2 to 5, 5 to 10.
    GridSteps {
        minor,
        major: step_125(minor * 1.5),
    }
}

/// Multiples of step from min to max. Past 2^53 n++ stops moving, so a URL
/// that pans that far gets no lines instead of a loop that never ends.
fn multiples(min: f64, max: f64, step: f64) -> Vec<f64> {
    let mut out = Vec::new();
    let mut n = (min / step - EPS).ceil();
    while n.fract() == 0.0 && n.abs() <= MAX_SAFE_INTEGER && n * step <= max + step * EPS {
        out.push(clean(n * step));
        n += 1.0;
    }
    out
}

/// Whether `u` sits on `step`. Rust and JS round a true halfway quotient to
/// different neighbours, but both read 0.5 there, never below the tolerance,
/// so the verdict agrees everywhere.
fn on_step(u: f64, step: f64) -> bool {
    (u / step - (u / step).round()).abs() < 1e-6
}

fn in_range(v: f64, lo: f64, hi: f64) -> Option<f64> {
    if v >= lo && v <= hi { Some(v) } else { None }
}

fn clamp(v: f64, lo: f64, hi: f64) -> f64 {
    v.min(hi).max(lo)
}

/// How a plane value prints in a label (JS `String(unit)`).
fn unit_text(unit: f64) -> String {
    format!("{unit}")
}

/// Splits units into minor and major lines, leaving out 0 where the axes are drawn.
fn split(units: &[f64], major: f64, to_px: impl Fn(f64) -> f64) -> (Vec<GridLine>, Vec<GridLine>) {
    let mut minor = Vec::new();
    let mut major_lines = Vec::new();
    for &unit in units {
        if unit == 0.0 {
            continue;
        }
        let line = GridLine {
            unit,
            px: to_px(unit),
        };
        if on_step(unit, major) {
            major_lines.push(line);
        } else {
            minor.push(line);
        }
    }
    (minor, major_lines)
}

/// The grid of a width by height canvas. Lines and axes up to margin px past
/// each edge are listed too, so a label beside a line just off screen still
/// shows. `center` is the plane point at the canvas pixel, as in `useCamera`.
pub fn layout_grid(
    center: [f64; 2],
    scale: f64,
    width: f64,
    height: f64,
    margin: f64,
) -> GridLayout {
    let px_per_unit = UNSCALED_UNIT_SIZE / scale;
    let steps = grid_steps(px_per_unit);

    let center_x = (width / 2.0).floor();
    let center_y = (height / 2.0).floor();
    let screen_x = |u: f64| center_x + (u - center[0]) * px_per_unit;
    // Screen y grows downward, plane y upward.
    let screen_y = |u: f64| center_y - (u - center[1]) * px_per_unit;

    let left = center[0] - (center_x + margin) / px_per_unit;
    let right = center[0] + (width - center_x + margin) / px_per_unit;
    let bottom = center[1] - (height - center_y + margin) / px_per_unit;
    let top = center[1] + (center_y + margin) / px_per_unit;
    let (minor_x, major_x) = split(&multiples(left, right, steps.minor), steps.major, screen_x);
    let (minor_y, major_y) = split(&multiples(bottom, top, steps.minor), steps.major, screen_y);

    GridLayout {
        steps,
        px_per_unit,
        minor_x,
        major_x,
        minor_y,
        major_y,
        x_origin: in_range(screen_x(0.0), -margin, width + margin),
        y_origin: in_range(screen_y(0.0), -margin, height + margin),
        origin: [screen_x(0.0), screen_y(0.0)],
        width,
        height,
        scale,
    }
}

/// The major lines that get an x label: all of them, or every other one once
/// the widest label of a line on the canvas leaves less than [`MIN_LABEL_GAP`]
/// before the next. Every other one means the multiples of twice the major
/// step, so the same labels stay as the plane pans.
pub fn x_label_lines(grid: &GridLayout, label_width: &dyn Fn(&str) -> f64) -> Vec<GridLine> {
    let spacing = grid.steps.major * grid.px_per_unit;
    // Lines past the edge are listed so their labels slide in, but a wide
    // label there isn't seen yet and mustn't thin the ones that are.
    let widest = grid
        .major_x
        .iter()
        .filter(|line| line.px >= 0.0 && line.px <= grid.width)
        .map(|line| label_width(&unit_text(line.unit)))
        .fold(0.0, f64::max);
    if widest + MIN_LABEL_GAP <= spacing {
        grid.major_x.clone()
    } else {
        grid.major_x
            .iter()
            .filter(|line| on_step(line.unit, 2.0 * grid.steps.major))
            .copied()
            .collect()
    }
}

/// The axis labels. x labels sit below the x-axis, right-aligned left of their
/// lines, or above it when there is no room below. y labels sit left of the
/// y-axis, right-aligned, below their lines, or right of it, left-aligned,
/// when there is no room left. While an axis is past an edge, its labels stay
/// along that edge.
pub fn layout_labels(
    grid: &GridLayout,
    label_height: f64,
    label_width: &dyn Fn(&str) -> f64,
) -> Vec<GridLabel> {
    let [origin_x, origin_y] = grid.origin;
    let width = grid.width;
    let height = grid.height;

    let below = origin_y + LABEL_GAP_Y;
    let above = origin_y - LABEL_GAP_Y - label_height;
    let x_top = clamp(
        if below + label_height <= height - LABEL_EDGE_PAD {
            below
        } else {
            above
        },
        LABEL_EDGE_PAD,
        height - LABEL_EDGE_PAD - label_height,
    );
    let mut labels: Vec<GridLabel> = x_label_lines(grid, label_width)
        .into_iter()
        .map(|line| GridLabel {
            axis: GridAxis::X,
            text: unit_text(line.unit),
            right: line.px - LABEL_GAP_X,
            top: x_top,
        })
        .collect();
    // The grid lists no line at 0, where the axes are. The origin's 0 shows
    // once, in the x labels.
    if let Some(x_origin) = grid.x_origin {
        labels.push(GridLabel {
            axis: GridAxis::X,
            text: "0".to_string(),
            right: x_origin - LABEL_GAP_X,
            top: x_top,
        });
    }

    // The y labels go left of the y-axis while the widest on the canvas fits there.
    let widest = grid
        .major_y
        .iter()
        .filter(|line| line.px >= 0.0 && line.px <= height)
        .map(|line| label_width(&unit_text(line.unit)))
        .fold(0.0, f64::max);
    let room_left = origin_x - LABEL_GAP_X - widest >= LABEL_EDGE_PAD;
    for line in &grid.major_y {
        let text = unit_text(line.unit);
        let top = line.px + LABEL_GAP_Y;
        // Where the x labels run along an edge, the y label beside them is left out.
        if (top - x_top).abs() < label_height {
            continue;
        }
        let w = label_width(&text);
        let right = if room_left {
            (origin_x - LABEL_GAP_X).min(width - LABEL_EDGE_PAD)
        } else {
            clamp(
                origin_x + LABEL_GAP_X,
                LABEL_EDGE_PAD,
                width - LABEL_EDGE_PAD - w,
            ) + w
        };
        labels.push(GridLabel {
            axis: GridAxis::Y,
            text,
            right,
            top,
        });
    }
    labels
}

#[cfg(test)]
mod tests {
    use super::*;

    fn units(lines: &[GridLine]) -> Vec<f64> {
        lines.iter().map(|line| line.unit).collect()
    }

    /// Screen px of every vertical line and the y-axis, left to right.
    fn x_positions(grid: &GridLayout) -> Vec<f64> {
        let mut px: Vec<f64> = grid
            .minor_x
            .iter()
            .chain(&grid.major_x)
            .map(|line| line.px)
            .chain(grid.x_origin)
            .collect();
        px.sort_by(f64::total_cmp);
        px
    }

    /// Screen px of every horizontal line and the x-axis, top to bottom.
    fn y_positions(grid: &GridLayout) -> Vec<f64> {
        let mut px: Vec<f64> = grid
            .minor_y
            .iter()
            .chain(&grid.major_y)
            .map(|line| line.px)
            .chain(grid.y_origin)
            .collect();
        px.sort_by(f64::total_cmp);
        px
    }

    // Roboto Mono at 14 px is 8.4 px a character: "-1600" is 42 px wide.
    fn label_width(text: &str) -> f64 {
        text.len() as f64 * 8.4
    }

    fn gaps(px: &[f64]) -> Vec<f64> {
        px.windows(2).map(|pair| pair[1] - pair[0]).collect()
    }

    /// vitest `toBeCloseTo` with its default 5 digits.
    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 5e-6,
            "expected {actual} to be close to {expected}"
        );
    }

    fn decimals(s: &str) -> usize {
        s.split('.').nth(1).map_or(0, str::len)
    }

    /// The default view's 800 by 600 canvas, 14 px labels, 64 px margin.
    fn labels_at(center: [f64; 2]) -> Vec<GridLabel> {
        let grid = layout_grid(center, 0.5, 800.0, 600.0, 64.0);
        layout_labels(&grid, 14.0, &label_width)
    }

    fn of_axis(labels: &[GridLabel], axis: GridAxis) -> Vec<GridLabel> {
        labels
            .iter()
            .filter(|label| label.axis == axis)
            .cloned()
            .collect()
    }

    #[test]
    fn at_each_scale_spaces_minor_lines_apart_and_majors_further() {
        let cases = [
            (0.01, 2500.0, 0.01, 0.02, 25.0),
            (0.1, 250.0, 0.1, 0.2, 25.0),
            (0.25, 100.0, 0.2, 0.5, 20.0),
            (0.5, 50.0, 0.5, 1.0, 25.0),
            (1.0, 25.0, 1.0, 2.0, 25.0),
            (1.25, 20.0, 1.0, 2.0, 20.0),
            (2.0, 12.5, 2.0, 5.0, 25.0),
            (5.0, 5.0, 5.0, 10.0, 25.0),
            (10.0, 2.5, 10.0, 20.0, 25.0),
            (100.0, 0.25, 100.0, 200.0, 25.0),
        ];
        for (scale, px_per_unit, minor, major, minor_px) in cases {
            let grid = layout_grid([0.0, 0.0], scale, 800.0, 600.0, 0.0);
            assert_eq!(grid.px_per_unit, px_per_unit, "scale {scale}");
            assert_eq!(grid.steps, GridSteps { minor, major }, "scale {scale}");
            assert_close(grid.steps.minor * grid.px_per_unit, minor_px);
        }
    }

    #[test]
    fn draws_no_grid_line_on_either_axis() {
        // The default view: the origin at the canvas center, minors every 0.5, majors every 1.
        let grid = layout_grid([0.0, 0.0], 0.5, 800.0, 600.0, 0.0);
        assert_eq!(grid.x_origin, Some(400.0));
        assert_eq!(grid.y_origin, Some(300.0));

        // The lines nearest the axes sit half a unit (25 px) to each side of them.
        assert!(grid.minor_x.contains(&GridLine {
            unit: 0.5,
            px: 425.0
        }));
        assert!(grid.minor_x.contains(&GridLine {
            unit: -0.5,
            px: 375.0
        }));
        assert!(grid.minor_y.contains(&GridLine {
            unit: 0.5,
            px: 275.0
        }));
        assert!(grid.minor_y.contains(&GridLine {
            unit: -0.5,
            px: 325.0
        }));

        // Nothing is listed at 0, where the axes are drawn.
        assert!(
            !grid
                .minor_x
                .iter()
                .chain(&grid.major_x)
                .chain(&grid.minor_y)
                .chain(&grid.major_y)
                .any(|line| line.unit == 0.0)
        );
        assert!(
            !grid
                .minor_x
                .iter()
                .chain(&grid.major_x)
                .any(|line| line.px == 400.0)
        );
        assert!(
            !grid
                .minor_y
                .iter()
                .chain(&grid.major_y)
                .any(|line| line.px == 300.0)
        );
    }

    #[test]
    fn puts_the_grid_where_the_camera_says() {
        // The canvas center shows (1.3, -0.7) at 50 px per unit.
        let grid = layout_grid([1.3, -0.7], 0.5, 800.0, 600.0, 0.0);

        // x = 1 is 0.3 u left of the center: 400 - 15 px.
        let x1 = grid
            .major_x
            .iter()
            .find(|line| line.unit == 1.0)
            .map(|line| line.px);
        assert_close(x1.unwrap(), 385.0);
        // y = -1 is 0.3 u below the center, and screen y grows downward: 300 + 15 px.
        let y1 = grid
            .major_y
            .iter()
            .find(|line| line.unit == -1.0)
            .map(|line| line.px);
        assert_close(y1.unwrap(), 315.0);
        // The axes are 1.3 u left of and 0.7 u above the center.
        assert_close(grid.x_origin.unwrap(), 335.0);
        assert_close(grid.y_origin.unwrap(), 265.0);
    }

    #[test]
    fn keeps_minor_lines_between_20_and_50_px_apart_at_every_scale() {
        // Sweep the whole zoom range, 0.01 to 100 in steps of 1%, with the origin off center.
        let mut scale = 0.01;
        while scale <= 100.0 {
            let grid = layout_grid([0.37, -2.1], scale, 800.0, 600.0, 0.0);
            // With the axis in the place of the line at 0, every line is one minor step from the next.
            for gap in gaps(&x_positions(&grid))
                .into_iter()
                .chain(gaps(&y_positions(&grid)))
            {
                assert!(gap >= 20.0 - 1e-6, "scale {scale}: gap {gap} below 20");
                assert!(gap < 50.0, "scale {scale}: gap {gap} reaches 50");
            }
            scale *= 1.01;
        }
    }

    #[test]
    fn reports_the_axes_off_screen_when_the_center_is_far_away() {
        // At 50 px per unit the 800 px canvas spans x = 92 to 108.
        let grid = layout_grid([100.0, 100.0], 0.5, 800.0, 600.0, 0.0);
        assert_eq!(grid.x_origin, None);
        assert_eq!(grid.y_origin, None);
        assert_eq!(
            units(&grid.major_x),
            (92..=108).map(f64::from).collect::<Vec<_>>()
        );
        assert_eq!(
            units(&grid.major_y),
            (94..=106).map(f64::from).collect::<Vec<_>>()
        );
    }

    #[test]
    fn lists_lines_and_axes_up_to_the_margin_past_each_edge() {
        // At 50 px per unit with a 64 px margin, the 800 by 600 canvas lists x = -9
        // to 9 (-50 to 850 px) and y = -7 to 7 (650 to -50 px).
        let grid = layout_grid([0.0, 0.0], 0.5, 800.0, 600.0, 64.0);
        assert_eq!(
            grid.major_x.first(),
            Some(&GridLine {
                unit: -9.0,
                px: -50.0
            })
        );
        assert_eq!(
            grid.major_x.last(),
            Some(&GridLine {
                unit: 9.0,
                px: 850.0
            })
        );
        assert_eq!(
            grid.major_y.first(),
            Some(&GridLine {
                unit: -7.0,
                px: 650.0
            })
        );
        assert_eq!(
            grid.major_y.last(),
            Some(&GridLine {
                unit: 7.0,
                px: -50.0
            })
        );

        // Panned so the y-axis is 50 px left of the canvas, it is still listed.
        let panned = layout_grid([9.0, 0.0], 0.5, 800.0, 600.0, 64.0);
        assert_eq!(panned.x_origin, Some(-50.0));
        assert_eq!(
            layout_grid([9.0, 0.0], 0.5, 800.0, 600.0, 0.0).x_origin,
            None
        );
    }

    #[test]
    fn labels_read_as_short_decimals_not_float_noise() {
        for (scale, major) in [(0.01, 0.02), (0.1, 0.2), (0.25, 0.5)] {
            // Far from the origin, where n x 0.2 gives values like 12.600000000000001.
            let grid = layout_grid([12.3, -45.6], scale, 800.0, 600.0, 0.0);
            let labels: Vec<String> = grid
                .major_x
                .iter()
                .chain(&grid.major_y)
                .map(|line| unit_text(line.unit))
                .collect();
            assert!(!labels.is_empty(), "scale {scale}");
            // No label has more decimals than the step: 12.14 with majors every 0.02.
            let step_decimals = decimals(&format!("{major}"));
            for label in &labels {
                assert!(
                    decimals(label) <= step_decimals,
                    "scale {scale}: {label} longer than step {major}"
                );
            }
        }
    }

    #[test]
    fn draws_no_vertical_lines_instead_of_freezing_when_panned_past_2_to_53() {
        // Past 2^53, x and the next minor step are the same number.
        let grid = layout_grid([1e20, 0.0], 0.5, 800.0, 600.0, 0.0);
        assert_eq!(grid.minor_x, vec![]);
        assert_eq!(grid.major_x, vec![]);
        // y is still near 0, so the horizontal lines and the x-axis are drawn as usual.
        assert!(!grid.major_y.is_empty());
        assert_eq!(grid.y_origin, Some(300.0));
    }

    #[test]
    fn shows_every_x_label_while_they_fit() {
        // The default view: labels like -8, 50 px apart.
        let grid = layout_grid([0.0, 0.0], 0.5, 800.0, 600.0, 0.0);
        assert_eq!(x_label_lines(&grid, &label_width), grid.major_x);
    }

    #[test]
    fn shows_every_other_x_label_where_the_minus_sign_makes_negatives_crowd() {
        // At scale 100 the labels are 200 u, 50 px, apart. Panned right, the
        // widest is 4600: 34 px, which leaves 16 px before the next.
        let positive = layout_grid([3000.0, 0.0], 100.0, 800.0, 600.0, 0.0);
        assert_eq!(
            units(&x_label_lines(&positive, &label_width)),
            units(&positive.major_x)
        );

        // Panned left, -4600 is 42 px and leaves only 8 px, so every other label
        // shows: the multiples of 400.
        let negative = layout_grid([-3000.0, 0.0], 100.0, 800.0, 600.0, 0.0);
        assert!(units(&negative.major_x).contains(&-4200.0));
        assert_eq!(
            units(&x_label_lines(&negative, &label_width)),
            vec![
                -4400.0, -4000.0, -3600.0, -3200.0, -2800.0, -2400.0, -2000.0, -1600.0
            ]
        );
    }

    #[test]
    fn keeps_every_x_label_when_only_a_label_past_the_edge_is_too_wide() {
        // At scale 6.2 the labels are 10 u, 40 px, apart, and the 800 px canvas
        // shows x = -99 to 99: labels up to -90, which fit. The 64 px margin
        // lists -110 and -100 too, which wouldn't, but they aren't on screen.
        let grid = layout_grid([0.0, 0.0], 6.2, 800.0, 600.0, 64.0);
        assert!(units(&grid.major_x).contains(&-110.0));
        assert_eq!(x_label_lines(&grid, &label_width), grid.major_x);
    }

    #[test]
    fn keeps_the_same_x_labels_as_the_plane_pans() {
        // Pans of less than one label step and of an odd number of steps: the
        // labels that show are always the multiples of 400 in view.
        for x in [-3000.0, -2950.0, -2800.0, -2633.0, -2200.0] {
            let grid = layout_grid([x, 0.0], 100.0, 800.0, 600.0, 0.0);
            let multiples_of_400: Vec<f64> = units(&grid.major_x)
                .into_iter()
                .filter(|u| u % 400.0 == 0.0)
                .collect();
            assert_eq!(units(&x_label_lines(&grid, &label_width)), multiples_of_400);
        }
    }

    #[test]
    fn puts_x_labels_below_the_x_axis_and_y_labels_left_of_the_y_axis() {
        // The origin is at (400, 300) and a unit is 50 px.
        let labels = labels_at([0.0, 0.0]);
        // x labels end 6 px left of their line, 6 px below the axis.
        assert!(of_axis(&labels, GridAxis::X).contains(&GridLabel {
            axis: GridAxis::X,
            text: "1".to_string(),
            right: 444.0,
            top: 306.0,
        }));
        // y labels end 6 px left of the axis, 6 px below their line.
        assert!(of_axis(&labels, GridAxis::Y).contains(&GridLabel {
            axis: GridAxis::Y,
            text: "1".to_string(),
            right: 394.0,
            top: 256.0,
        }));
        // The origin's 0 shows once, among the x labels.
        let zeros: Vec<GridLabel> = labels
            .into_iter()
            .filter(|label| label.text == "0")
            .collect();
        assert_eq!(
            zeros,
            vec![GridLabel {
                axis: GridAxis::X,
                text: "0".to_string(),
                right: 394.0,
                top: 306.0,
            }]
        );
    }

    #[test]
    fn keeps_the_x_labels_along_the_top_edge_while_the_x_axis_is_above() {
        // y = 0 is 100 u, 5000 px, above the canvas center.
        for label in of_axis(&labels_at([0.0, -100.0]), GridAxis::X) {
            assert_eq!(label.top, 8.0);
        }
    }

    #[test]
    fn keeps_the_x_labels_along_the_bottom_edge_while_the_x_axis_is_below() {
        // 14 px labels whose bottoms stay 8 px above the 600 px canvas's edge.
        for label in of_axis(&labels_at([0.0, 100.0]), GridAxis::X) {
            assert_eq!(label.top, 578.0);
        }
    }

    #[test]
    fn keeps_the_y_labels_along_the_left_edge_while_the_y_axis_is_left() {
        let labels = of_axis(&labels_at([100.0, 0.0]), GridAxis::Y);
        assert!(!labels.is_empty());
        for label in &labels {
            assert_close(label.right - label_width(&label.text), 8.0);
        }
    }

    #[test]
    fn keeps_the_y_labels_along_the_right_edge_while_the_y_axis_is_right() {
        for label in of_axis(&labels_at([-100.0, 0.0]), GridAxis::Y) {
            assert_eq!(label.right, 792.0);
        }
    }

    #[test]
    fn puts_the_x_labels_above_the_x_axis_when_too_near_the_bottom_edge() {
        // y = 0 is 10 px above the bottom edge: 590 px. Below it, 14 px labels
        // would run past the edge, so their tops sit 6 + 14 px above the axis.
        for label in of_axis(&labels_at([0.0, 5.8]), GridAxis::X) {
            assert_close(label.top, 570.0);
        }
    }

    #[test]
    fn puts_the_y_labels_right_of_the_y_axis_when_too_near_the_left_edge() {
        // x = 0 is 15 px from the left edge, too near for -6, 16.8 px wide, so
        // the labels start 6 px right of the axis.
        let labels = of_axis(&labels_at([7.7, 0.0]), GridAxis::Y);
        assert!(!labels.is_empty());
        for label in &labels {
            assert_close(label.right - label_width(&label.text), 21.0);
        }
    }

    #[test]
    fn leaves_out_the_y_label_that_would_overlap_the_x_labels() {
        // The x-axis is far above the canvas, so the x labels run along the top
        // edge, 8 px down. The canvas center shows y = -100, so y = -94 is at
        // 0 px and its label at 6 px would overlap them; y = -95 is 50 px lower.
        let labels = labels_at([0.0, -100.0]);
        assert_eq!(
            of_axis(&labels, GridAxis::X).first().map(|label| label.top),
            Some(8.0)
        );
        let y_texts: Vec<String> = of_axis(&labels, GridAxis::Y)
            .into_iter()
            .map(|label| label.text)
            .collect();
        assert!(!y_texts.contains(&"-94".to_string()));
        assert!(y_texts.contains(&"-95".to_string()));
    }

    /// `1cfd34a` lays the grid out once per camera update: the layout carries
    /// the canvas width, height and scale it was computed for, so the label
    /// functions and the legend read them from the grid.
    #[test]
    fn layout_grid_stores_width_height_and_scale() {
        let grid = layout_grid([0.0, 0.0], 0.5, 800.0, 600.0, 64.0);
        assert_eq!(grid.width, 800.0);
        assert_eq!(grid.height, 600.0);
        assert_eq!(grid.scale, 0.5);
    }

    #[test]
    fn clean_matches_to_precision_12() {
        // Pins mv-numfmt's `precision` to math-viz2's `Number(v.toPrecision(12))`,
        // which `clean` stands in for: the comment's own example, and its neighbour.
        assert_eq!(clean(3.0 * 0.2), 0.6);
        assert_eq!(clean(0.1 + 0.2), 0.3);
        assert_eq!(clean(2.0 * 0.1), 0.2);
        assert_eq!(clean(10.0 * 0.001), 0.01);
    }
}
