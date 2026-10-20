//! The plotted curves and the function-switch animation.
//!
//! Ported from math-viz2 at `3560eb2`:
//!
//! - `src/components/views/CartesianPlane/usePlotCurves.ts`: [`Shown`],
//!   [`step`], [`ease_draw`], the fade and draw durations.
//! - `src/components/views/CartesianPlane/drawCartesianPlane.ts`
//!   (`drawCurve`): [`sample_curve`], the polyline the painter strokes
//!   (paint itself stays with the h3 coder's `plane.rs`).

use crate::grid::GridLayout;

/// A function the plane graphs as y = f(x) (`PlotFunction` in
/// `drawCartesianPlane.ts`). Plain fn pointers, never closures: the hook
/// compares function identity (`!==`), and address comparison is exactly that.
pub type PlotFunction = fn(f64) -> f64;

/// How long the old curve takes to fade out, in ms (`fadeMs`).
pub const FADE_MS: f64 = 160.0;

/// How long the new curve takes to draw on, in ms (`drawMs`).
pub const DRAW_MS: f64 = 900.0;

/// The first frame of a switch counts as one frame in (`firstFrameMs`, as in
/// `camera.rs`): the rAF loop's first callback carries no measured gap.
pub const FIRST_FRAME_MS: f64 = 1000.0 / 60.0;

/// A function's curve as one frame shows it: its opacity, and how much of its
/// length inside the canvas is drawn, from the left (1 for all of it)
/// (`PlotCurve`).
#[derive(Clone, Copy, Debug)]
pub struct PlotCurve {
    /// `fn` in the TypeScript, renamed: `fn` is a Rust keyword.
    pub func: PlotFunction,
    pub alpha: f64,
    pub draw_to: f64,
}

// Function identity, as the hook's `===`: addresses, through the sanctioned
// comparison (a derived `PartialEq` would compare fn pointers with `==`,
// which the compiler lints).
impl PartialEq for PlotCurve {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::fn_addr_eq(self.func, other.func)
            && self.alpha == other.alpha
            && self.draw_to == other.draw_to
    }
}

/// The function shown, and while it switches, the one before it and the ms
/// since the switch began (`Shown`).
#[derive(Clone, Copy, Debug)]
pub struct Shown {
    pub func: PlotFunction,
    pub old: Option<PlotFunction>,
    pub elapsed: f64,
    /// The next [`step`] is the switch's first frame and carries
    /// [`FIRST_FRAME_MS`] instead of its measured gap (the rAF loop's
    /// `last === null` branch; cf. `camera.rs`'s `first_frame`).
    first_frame: bool,
}

impl Shown {
    /// The steady state: `func` shown whole
    /// (`useState({ fn, old: null, elapsed: 0 })`).
    #[must_use]
    pub fn new(func: PlotFunction) -> Self {
        Self {
            func,
            old: None,
            elapsed: 0.0,
            first_frame: false,
        }
    }

    /// The render-phase switch, set during render so no frame shows the new
    /// curve whole before it draws on: showing another function fades the
    /// curve shown, whole, and starts the draw-on again. A change during a
    /// switch fades the curve shown (not the fading one) whole. With reduced
    /// motion the new curve replaces at once; the coder passes the app's
    /// reduced-motion setting as `can_animate`.
    pub fn switch_to(&mut self, func: PlotFunction, can_animate: bool) {
        // Function identity, as the hook's `!==`: addresses, through the
        // sanctioned comparison (plain `==` on fn pointers is linted).
        if std::ptr::fn_addr_eq(self.func, func) {
            return;
        }
        // A switch from rest starts the frame clock (`first_frame`, as the
        // hook's loop starts); a change mid-switch keeps the running loop's
        // clock, as the hook's loop is keyed on `switching` and is not
        // restarted (cf. `camera.rs`'s `ensure_loop`).
        let fresh = self.old.is_none();
        let old = if can_animate { Some(self.func) } else { None };
        *self = Self {
            func,
            old,
            elapsed: 0.0,
            first_frame: fresh,
        };
    }

    /// Whether a switch is running: the frame loop the hook starts
    /// (`switching`). The coder steps only then.
    #[must_use]
    pub fn is_switching(&self) -> bool {
        self.old.is_some()
    }
}

/// Ease-in-out cubic, the curve behind the draw-on ease
/// (`easeDraw`, behind math-viz's `cubic-bezier(0.65, 0, 0.35, 1)`).
#[must_use]
pub fn ease_draw(t: f64) -> f64 {
    if t < 0.5 {
        4.0 * t.powi(3)
    } else {
        1.0 - (2.0 - 2.0 * t).powi(3) / 2.0
    }
}

/// The curves one frame draws (`usePlotCurves`'s return): while the old curve
/// is still visible it shows whole under the drawing new one, then the one
/// curve.
#[must_use]
pub fn curves(shown: &Shown) -> Vec<PlotCurve> {
    let Some(old) = shown.old else {
        return vec![PlotCurve {
            func: shown.func,
            alpha: 1.0,
            draw_to: 1.0,
        }];
    };
    let old_alpha = 1.0 - shown.elapsed / FADE_MS;
    let curve = PlotCurve {
        func: shown.func,
        alpha: 1.0,
        draw_to: ease_draw(shown.elapsed / DRAW_MS),
    };
    if old_alpha > 0.0 {
        vec![
            PlotCurve {
                func: old,
                alpha: old_alpha,
                draw_to: 1.0,
            },
            curve,
        ]
    } else {
        vec![curve]
    }
}

/// One animation frame: add its time to the switch's elapsed and return the
/// curves the frame draws. Past the draw duration the switch ends
/// (`{ old: null, elapsed: 0 }`) and the curve shows whole. At rest the state
/// is untouched and the whole curve returns, so stepping past the end is
/// harmless (the hook's loop simply no longer runs).
pub fn step(shown: &mut Shown, dt: f64) -> Vec<PlotCurve> {
    if shown.old.is_some() {
        let dt = if shown.first_frame {
            FIRST_FRAME_MS
        } else {
            dt
        };
        shown.first_frame = false;
        if shown.elapsed + dt >= DRAW_MS {
            shown.old = None;
            shown.elapsed = 0.0;
        } else {
            shown.elapsed += dt;
        }
    }
    curves(shown)
}

/// The share of the segment from y0 to y1 that lies between the canvas's top
/// and bottom edges (`insideShare`).
fn inside_share(y0: f64, y1: f64, height: f64) -> f64 {
    let lo = y0.min(y1);
    let hi = y0.max(y1);
    if hi < 0.0 || lo > height {
        return 0.0;
    }
    if hi == lo {
        return 1.0;
    }
    (hi.min(height) - lo.max(0.0)) / (hi - lo)
}

/// y = f(x) across the canvas, one point per px column (`drawCurve`'s
/// sampling): the polyline the painter strokes. A curve drawn partway stops
/// once it has drawn its share `draw_to` of the length inside the canvas, so
/// the pen spends no time above or below the canvas.
///
/// `grid` is the canvas's [`GridLayout`]; it stores no canvas dims, so
/// `width` and `height` ride along. `draw_to` is the curve's share from the
/// left (1 for all of it); the curve's `alpha` is paint, not sampling.
#[must_use]
pub fn sample_curve(
    func: PlotFunction,
    grid: &GridLayout,
    width: f64,
    height: f64,
    draw_to: f64,
) -> Vec<[f64; 2]> {
    let [origin_x, origin_y] = grid.origin;
    let px_per_unit = grid.px_per_unit;
    // Each point with the length inside the canvas from the left edge to it.
    let mut points: Vec<([f64; 2], f64)> = Vec::new();
    let mut at = 0.0;
    let cols = width as usize;
    for i in 0..=cols {
        let px = i as f64;
        let y = origin_y - func((px - origin_x) / px_per_unit) * px_per_unit;
        if let Some(&(prev, _)) = points.last() {
            at += (px - prev[0]).hypot(y - prev[1]) * inside_share(prev[1], y, height);
        }
        points.push(([px, y], at));
    }
    let stop = draw_to * at;

    let mut line = Vec::new();
    let mut prev: Option<([f64; 2], f64)> = None;
    for &(point, point_at) in &points {
        // The pen stops partway along this segment.
        if let Some((prev_point, prev_at)) = prev
            && point_at > stop
        {
            let t = (stop - prev_at) / (point_at - prev_at);
            line.push([
                prev_point[0] + (point[0] - prev_point[0]) * t,
                prev_point[1] + (point[1] - prev_point[1]) * t,
            ]);
            break;
        }
        line.push(point);
        prev = Some((point, point_at));
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::layout_grid;

    // Functions from the sidebar's Function picker.
    fn line(x: f64) -> f64 {
        x
    }
    fn square(x: f64) -> f64 {
        x * x
    }
    fn cube(x: f64) -> f64 {
        x * x * x
    }

    /// `toBeCloseTo(expected, 2)`: within half a hundredth.
    fn assert_close_to(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 0.005,
            "expected {actual} to be close to {expected}"
        );
    }

    /// Animation frames run only when a test advances them, 1000/60 ms apart
    /// as on a 60 Hz screen: the frame clock behind the TS `runFrames`. Time
    /// accumulates and each frame diffs it, so `elapsed` sums exactly as the
    /// hook's `now - last` does.
    struct Frames {
        time: f64,
        last: Option<f64>,
    }

    impl Frames {
        fn new() -> Self {
            Self {
                time: 0.0,
                last: None,
            }
        }

        fn frame(&mut self, shown: &mut Shown) -> Vec<PlotCurve> {
            self.time += FIRST_FRAME_MS;
            let dt = self.last.map_or(FIRST_FRAME_MS, |last| self.time - last);
            self.last = Some(self.time);
            step(shown, dt)
        }
    }

    #[test]
    fn picking_x2_fades_y_equals_x_out_while_x2_draws_on_from_the_left() {
        let mut shown = Shown::new(line);
        // y = x shows whole.
        assert_eq!(
            curves(&shown),
            vec![PlotCurve {
                func: line,
                alpha: 1.0,
                draw_to: 1.0
            }]
        );

        // Picking x²: until the next frame, y = x shows whole and none of x² shows.
        let mut frames = Frames::new();
        shown.switch_to(square, true);
        assert_eq!(
            curves(&shown),
            vec![
                PlotCurve {
                    func: line,
                    alpha: 1.0,
                    draw_to: 1.0
                },
                PlotCurve {
                    func: square,
                    alpha: 1.0,
                    draw_to: 0.0
                },
            ]
        );

        // 83 ms in: y = x is about half faded and x² has barely started.
        let mut current = Vec::new();
        for _ in 0..5 {
            current = frames.frame(&mut shown);
        }
        assert_eq!(current.len(), 2);
        let [fading, drawing] = current.as_slice() else {
            panic!("expected the fading and drawing curves");
        };
        assert!(std::ptr::fn_addr_eq(fading.func, line as PlotFunction));
        assert_close_to(fading.alpha, 0.48);
        assert!(std::ptr::fn_addr_eq(drawing.func, square as PlotFunction));
        assert!(drawing.draw_to > 0.0);
        assert!(drawing.draw_to < 0.01);

        // 450 ms in: y = x is gone and x² is drawn halfway.
        for _ in 0..22 {
            current = frames.frame(&mut shown);
        }
        assert_eq!(current.len(), 1);
        assert!(std::ptr::fn_addr_eq(
            current[0].func,
            square as PlotFunction
        ));
        assert_close_to(current[0].draw_to, 0.5);

        // Just past 900 ms: x² shows whole and the frames stop.
        for _ in 0..28 {
            current = frames.frame(&mut shown);
        }
        assert_eq!(
            current,
            vec![PlotCurve {
                func: square,
                alpha: 1.0,
                draw_to: 1.0
            }]
        );
        assert!(!shown.is_switching());
    }

    #[test]
    fn picking_x3_while_x2_draws_on_fades_x2_out_whole_and_draws_x3_from_the_start() {
        let mut shown = Shown::new(line);
        let mut frames = Frames::new();
        // Picking x², then waiting 450 ms: x² is drawn halfway.
        shown.switch_to(square, true);
        let mut current = Vec::new();
        for _ in 0..27 {
            current = frames.frame(&mut shown);
        }
        assert_close_to(current[0].draw_to, 0.5);

        // Picking x³: the half-drawn x² shows whole, to fade, and none of x³ shows.
        shown.switch_to(cube, true);
        assert_eq!(
            curves(&shown),
            vec![
                PlotCurve {
                    func: square,
                    alpha: 1.0,
                    draw_to: 1.0
                },
                PlotCurve {
                    func: cube,
                    alpha: 1.0,
                    draw_to: 0.0
                },
            ]
        );

        // 83 ms later: x² is about half faded and x³ has barely started.
        for _ in 0..5 {
            current = frames.frame(&mut shown);
        }
        assert_close_to(current[0].alpha, 0.48);
        assert!(current[1].draw_to < 0.01);
    }

    #[test]
    fn with_reduced_motion_picking_x2_shows_it_whole_at_once() {
        let mut shown = Shown::new(line);
        // Picking x²: it replaces y = x with no frames to run.
        shown.switch_to(square, false);
        assert_eq!(
            curves(&shown),
            vec![PlotCurve {
                func: square,
                alpha: 1.0,
                draw_to: 1.0
            }]
        );
        assert!(!shown.is_switching());
    }

    /// Draws one curve on a 400 by 300 px canvas at scale 0.5: 50 px per unit,
    /// the origin at (200, 150). `layout_grid`'s margin is
    /// `drawCartesianPlane.ts`'s `labelMargin` (64), owned by the h3 painter;
    /// the origin and px-per-unit the sampler reads do not depend on it.
    fn sampled(func: PlotFunction, draw_to: f64) -> Vec<[f64; 2]> {
        let grid = layout_grid([0.0, 0.0], 0.5, 400.0, 300.0, 64.0);
        sample_curve(func, &grid, 400.0, 300.0, draw_to)
    }

    #[test]
    fn graphs_y_equals_x2_through_0_5_0_25_and_1_1() {
        let points = sampled(square, 1.0);

        // (0.5, 0.25) is 25 px right of the origin and 12.5 px above it.
        assert!(points.contains(&[225.0, 137.5]));
        // (1, 1) is 50 px right and 50 px above.
        assert!(points.contains(&[250.0, 100.0]));
    }

    #[test]
    fn a_quarter_of_the_way_through_drawing_y_equals_x3_the_pen_is_on_the_canvas() {
        // y = x³ comes up through the bottom edge near x = -1.44 (128 px) and
        // leaves through the top near x = 1.44 (272 px). A quarter of the way
        // across the canvas, at x = -2, it is 250 px below the bottom edge.
        let points = sampled(cube, 0.25);

        // A quarter of what shows is on the part left of and below the origin.
        let &[x, y] = points.last().expect("a part-drawn curve strokes points");
        assert!(x > 128.0);
        assert!(x < 200.0);
        assert!(y > 150.0);
        assert!(y < 300.0);
    }
}
