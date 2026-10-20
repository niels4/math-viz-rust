//! Per-pixel curve sampling and shares along a trace.

use super::dash::MIN_PIECE_PX;
use super::scene::{CurveFn, curve_at};
use super::viewport::{Viewport, to_math_x, to_screen_y};

/// How far past the viewport a trace overshoots, in px.
const OVERSHOOT_PX: f64 = 10_000.0;

/// The extra samples past each viewport edge.
const EDGE_SAMPLES: i64 = 2;

/// One trace sample: its screen point, whether it starts a run, and its share.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TraceSample {
    pub x: f64,
    pub y: f64,
    pub start: bool,
    pub at: f64,
}

/// A sampled curve: its samples and their length.
#[derive(Clone, Debug, PartialEq)]
pub struct Trace {
    pub samples: Vec<TraceSample>,
    pub length: f64,
}

/// The share of the segment between y0 and y1 that lies within [0, height].
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

/// Samples a curve across a viewport, one sample per px.
pub fn curve_trace(vp: Viewport, func: &CurveFn) -> Trace {
    let mut samples = Vec::new();
    let mut at = 0.0;
    let mut prev: Option<TraceSample> = None;
    let mut px = -(EDGE_SAMPLES as f64);
    while px <= vp.width + EDGE_SAMPLES as f64 {
        let value = curve_at(func, to_math_x(vp, px));
        if value.is_nan() || value.is_infinite() {
            prev = None;
            px += 1.0;
            continue;
        }
        let y = (vp.height + OVERSHOOT_PX).min((-OVERSHOOT_PX).max(to_screen_y(vp, value)));
        if let Some(p) = prev {
            at += ((px - p.x) * (px - p.x) + (y - p.y) * (y - p.y)).sqrt()
                * inside_share(p.y, y, vp.height);
        }
        let sample = TraceSample {
            x: px,
            y,
            start: prev.is_none(),
            at,
        };
        samples.push(sample);
        prev = Some(sample);
        px += 1.0;
    }
    Trace {
        samples,
        length: at,
    }
}

/// The samples up to a share of a trace.
pub fn trace_up_to(trace: &Trace, share: f64) -> Vec<TraceSample> {
    if share >= 1.0 {
        return trace.samples.clone();
    }
    if share <= 0.0 || trace.length == 0.0 {
        return Vec::new();
    }
    let stop = share * trace.length;
    let mut drawn = Vec::new();
    for sample in &trace.samples {
        if sample.at <= stop {
            drawn.push(*sample);
            continue;
        }
        if !drawn.is_empty() && !sample.start {
            let last: TraceSample = drawn[drawn.len() - 1];
            let t = (stop - last.at) / (sample.at - last.at);
            let cut = [
                last.x + (sample.x - last.x) * t,
                last.y + (sample.y - last.y) * t,
            ];
            if ((cut[0] - last.x) * (cut[0] - last.x) + (cut[1] - last.y) * (cut[1] - last.y))
                .sqrt()
                >= MIN_PIECE_PX
            {
                drawn.push(TraceSample {
                    x: cut[0],
                    y: cut[1],
                    start: false,
                    at: stop,
                });
            }
        }
        break;
    }
    drawn
}

/// The share of a trace at a math x.
pub fn share_at_x(trace: &Trace, x: f64) -> f64 {
    if trace.length == 0.0 {
        return 1.0;
    }
    for s in &trace.samples {
        if s.x >= x {
            return s.at / trace.length;
        }
    }
    1.0
}

#[cfg(test)]
#[allow(clippy::float_cmp, clippy::excessive_precision)]
mod tests {
    use super::super::scene::CurveFn;
    use super::*;
    use std::f64::consts::SQRT_2;

    const SMALL: Viewport = Viewport {
        width: 100.0,
        height: 100.0,
        zoom: 10.0,
        origin_x: 50.0,
        origin_y: 50.0,
    };
    const R2: Viewport = Viewport {
        width: 936.0,
        height: 792.0,
        zoom: 50.0,
        origin_x: 468.0,
        origin_y: 396.0,
    };

    fn curve_fn_of(f: impl Fn(f64) -> f64 + 'static) -> CurveFn {
        CurveFn { f: Box::new(f) }
    }

    fn expect_floats(got: &[f64], want: &[f64]) {
        assert_eq!(got.len(), want.len(), "got {got:?}, want {want:?}");
        for (g, w) in got.iter().zip(want.iter()) {
            assert!((g - w).abs() < 1e-9, "got {got:?}, want {want:?}");
        }
    }

    fn square(x: f64) -> f64 {
        x * x
    }

    #[test]
    fn a_trace_measures_only_what_draws_inside_the_plane() {
        let trace = curve_trace(SMALL, &curve_fn_of(|x| x));
        assert!((trace.length - 100.0 * SQRT_2).abs() < 1e-9);
        assert_eq!(trace.samples.len(), 105);
        let first = trace.samples[0];
        assert!(first.x == -2.0 && first.y == 102.0 && first.at == 0.0 && first.start);
        assert!((trace.samples[trace.samples.len() - 1].at - 100.0 * SQRT_2).abs() < 1e-9);
    }

    #[test]
    fn a_trace_spends_no_length_above_the_plane() {
        let trace = curve_trace(R2, &curve_fn_of(square));
        assert!((share_at_x(&trace, 468.0) - 0.5).abs() < 5e-4);
        assert_eq!(share_at_x(&trace, 300.0), 0.0);
    }

    #[test]
    fn a_trace_lifts_the_pen_where_the_function_breaks() {
        let trace = curve_trace(
            SMALL,
            &curve_fn_of(|x| if x == 0.0 { f64::NAN } else { 1.0 / x }),
        );
        let starts: Vec<f64> = trace
            .samples
            .iter()
            .filter(|s| s.start)
            .map(|s| s.x)
            .collect();
        expect_floats(&starts, &[-2.0, 51.0]);
    }

    #[test]
    fn trace_up_to_cuts_the_pen_where_the_share_ends() {
        let line = curve_trace(SMALL, &curve_fn_of(|x| x));
        let half = trace_up_to(&line, 0.5);
        let last = half[half.len() - 1];
        assert!((last.x - 50.0).abs() < 1e-6 && (last.y - 50.0).abs() < 1e-6);
        assert!((last.at - 50.0 * SQRT_2).abs() < 1e-6);
        assert_eq!(trace_up_to(&line, 0.0).len(), 0);
        assert_eq!(trace_up_to(&line, 1.0).len(), line.samples.len());
    }

    #[test]
    fn trace_up_to_never_bridges_a_break() {
        let step = curve_trace(
            SMALL,
            &curve_fn_of(|x| {
                if x < 0.0 {
                    1.0
                } else if x > 0.0 {
                    -1.0
                } else {
                    f64::NAN
                }
            }),
        );
        assert_eq!(step.length, 102.0);
        for s in trace_up_to(&step, 0.25) {
            assert_eq!(s.y, 40.0);
        }
        let across = trace_up_to(&step, 0.75);
        let mut first_right = 0;
        let mut found = false;
        for (i, s) in across.iter().enumerate() {
            if s.y == 60.0 {
                first_right = i;
                found = true;
                break;
            }
        }
        assert!(found);
        assert!(first_right > 0);
        assert!(across[first_right].x == 51.0 && across[first_right].start);
        assert_eq!(across[first_right - 1].x, 49.0);
        let end = across[across.len() - 1];
        assert!(end.x == 76.5 && end.y == 60.0 && end.at == 76.5);
    }

    #[test]
    fn share_at_x_says_how_far_the_pen_has_drawn() {
        let line = curve_trace(SMALL, &curve_fn_of(|x| x));
        assert!((share_at_x(&line, 25.0) - 0.25).abs() < 1e-9);
        assert_eq!(share_at_x(&line, -50.0), 0.0);
        assert_eq!(share_at_x(&line, 500.0), 1.0);
        let above = curve_trace(SMALL, &curve_fn_of(|_| 1000.0));
        assert_eq!(above.length, 0.0);
        assert_eq!(share_at_x(&above, 50.0), 1.0);
        assert_eq!(trace_up_to(&above, 0.5).len(), 0);
    }

    #[test]
    fn a_draw_on_leaves_out_a_sliver_nanovg_would_merge_into_a_point() {
        let trace = curve_trace(R2, &curve_fn_of(f64::sin));
        let no_sliver = |drawn: &[TraceSample]| -> bool {
            for (i, s) in drawn.iter().enumerate() {
                if i > 0
                    && !s.start
                    && ((s.x - drawn[i - 1].x) * (s.x - drawn[i - 1].x)
                        + (s.y - drawn[i - 1].y) * (s.y - drawn[i - 1].y))
                        .sqrt()
                        < MIN_PIECE_PX
                {
                    return false;
                }
            }
            true
        };
        for share in [3.9061201148734744e-06, 9.7527220566048779e-07] {
            let drawn = trace_up_to(&trace, share);
            assert_eq!(drawn.len(), 1);
        }
        for (i, s) in trace.samples.iter().enumerate() {
            if i % 10 != 0 {
                continue;
            }
            for past in [1e-9, 1e-6, 1e-3, 0.02] {
                assert!(no_sliver(&trace_up_to(
                    &trace,
                    (s.at + past) / trace.length
                )));
            }
        }
    }
}
