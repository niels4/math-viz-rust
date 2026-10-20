//! Gesture maths: zoom clamping, wheel views, and the fling glide.

use super::grid::grid_steps;
use super::viewport::{MAX_UNIT_SIZE, MIN_UNIT_SIZE, ViewState};

/// The zoom rate per wheel delta.
pub const ZOOM_SPEED: f64 = 0.0015;

/// The fling's exponential decay, per second.
pub const INERTIA_DECAY: f64 = 4.5;

/// The slowest fling that still glides, in px per second.
pub const INERTIA_MIN_SPEED: f64 = 24.0;

/// The fastest fling, in px per second.
pub const INERTIA_MAX_SPEED: f64 = 8000.0;

/// The window a fling's velocity measures over, in ms.
pub const VELOCITY_WINDOW_MS: f64 = 120.0;

/// The longest glide step, in seconds.
pub const GLIDE_MAX_STEP_S: f64 = 0.05;

/// Clamps a zoom to the stops' range.
pub fn clamp_zoom(zoom: f64) -> f64 {
    MIN_UNIT_SIZE.max(MAX_UNIT_SIZE.min(zoom))
}

/// Zooms a view state around a cursor by a wheel delta.
pub fn wheel_view(
    view: ViewState,
    delta: f64,
    cursor: [f64; 2],
    center: [f64; 2],
) -> (ViewState, bool) {
    let next = clamp_zoom(view.zoom * (-delta * ZOOM_SPEED).exp());
    if next == view.zoom {
        return (view, false);
    }
    let k = 1.0 / next - 1.0 / view.zoom;
    (
        ViewState {
            zoom: next,
            pan_x: view.pan_x + (cursor[0] - center[0]) * k,
            pan_y: view.pan_y + (center[1] - cursor[1]) * k,
        },
        true,
    )
}

/// One drag sample: its point and time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DragSample {
    pub at: [f64; 2],
    pub t: f64,
}

/// Drops the samples older than the velocity window, keeping at least two.
pub fn prune_samples(samples: &mut Vec<DragSample>) {
    while samples.len() > 2
        && samples.last().expect("len > 2").t - samples.first().expect("len > 2").t
            > VELOCITY_WINDOW_MS
    {
        samples.remove(0);
    }
}

/// The velocity of recent samples, and whether there is one.
pub fn velocity_of_samples(samples: &[DragSample]) -> ([f64; 2], bool) {
    if samples.len() < 2 {
        return ([0.0, 0.0], false);
    }
    let last = samples[samples.len() - 1];
    let mut first = last;
    for s in samples {
        if last.t - s.t <= VELOCITY_WINDOW_MS {
            first = *s;
            break;
        }
    }
    if last.t <= first.t {
        return ([0.0, 0.0], false);
    }
    let dt = (last.t - first.t) / 1000.0;
    let v = [
        (last.at[0] - first.at[0]) / dt,
        (last.at[1] - first.at[1]) / dt,
    ];
    if (v[0] * v[0] + v[1] * v[1]).sqrt() < INERTIA_MIN_SPEED {
        return ([0.0, 0.0], false);
    }
    (v, true)
}

/// Clamps a launch velocity to the fling range.
pub fn clamp_launch_speed(v: [f64; 2]) -> [f64; 2] {
    let scale = (1.0f64).min(INERTIA_MAX_SPEED / (v[0] * v[0] + v[1] * v[1]).sqrt());
    [v[0] * scale, v[1] * scale]
}

/// Decays a velocity over a step.
pub fn decay_velocity(v: [f64; 2], dt: f64) -> [f64; 2] {
    let decay = (-INERTIA_DECAY * dt).exp();
    [v[0] * decay, v[1] * decay]
}

/// A fling in flight: its velocity and last step time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Glide {
    pub v: [f64; 2],
    pub last: f64,
}

/// Starts a glide from a velocity at now, and whether it glides.
pub fn glide_start(v: [f64; 2], now: f64) -> (Glide, bool) {
    if (v[0] * v[0] + v[1] * v[1]).sqrt() < INERTIA_MIN_SPEED {
        return (
            Glide {
                v: [0.0, 0.0],
                last: 0.0,
            },
            false,
        );
    }
    (
        Glide {
            v: clamp_launch_speed(v),
            last: now,
        },
        true,
    )
}

/// Steps a glide at now: its step and whether it keeps gliding.
pub fn glide_step(g: &mut Glide, now: f64, zoom: f64) -> ([f64; 2], bool) {
    let dt = GLIDE_MAX_STEP_S.min(0.0f64.max((now - g.last) / 1000.0));
    g.last = now;
    g.v = decay_velocity(g.v, dt);
    if (g.v[0] * g.v[0] + g.v[1] * g.v[1]).sqrt() < INERTIA_MIN_SPEED {
        return ([0.0, 0.0], false);
    }
    let mut step = [0.0, 0.0];
    if zoom > 0.0 && dt > 0.0 {
        step = [(g.v[0] * dt) / zoom, (g.v[1] * dt) / zoom];
    }
    (step, true)
}

/// Drags from an anchor: the mark follows the pointer, locked to an axis.
pub fn drag_to(from: [f64; 2], mark: [f64; 2], pointer: [f64; 2], lock: bool) -> [f64; 2] {
    let mut d = [pointer[0] - from[0], pointer[1] - from[1]];
    if lock {
        if d[0].abs() >= d[1].abs() {
            d[1] = 0.0;
        } else {
            d[0] = 0.0;
        }
    }
    [mark[0] + d[0], mark[1] + d[1]]
}

/// The pan quantum at a zoom.
pub fn pan_square(zoom: f64) -> f64 {
    grid_steps(zoom).minor
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::super::viewport::{
        MAX_UNIT_SIZE, MIN_UNIT_SIZE, ViewState, Viewport, make_viewport, to_math_x, to_math_y,
        to_screen_x, to_screen_y,
    };
    use super::*;

    fn close(got: f64, want: f64) {
        assert!(
            (got - want).abs() < 10f64.powf(-8.0) / 2.0,
            "got {got}, want {want}"
        );
    }

    fn dynamic_of(samples: &[DragSample]) -> Vec<DragSample> {
        samples.to_vec()
    }

    #[test]
    fn each_wheel_notch_zooms_from_where_the_last_left_the_view() {
        let mut view = ViewState {
            zoom: 50.0,
            pan_x: 0.0,
            pan_y: 0.0,
        };
        let center = [300.0, 300.0];
        let cursor = [450.0, 150.0];
        let unsnapped = |view: ViewState| -> Viewport {
            Viewport {
                width: 600.0,
                height: 600.0,
                zoom: view.zoom,
                origin_x: 300.0 + view.pan_x * view.zoom,
                origin_y: 300.0 - view.pan_y * view.zoom,
            }
        };
        for _ in 0..3 {
            let before = unsnapped(view);
            let held = [to_math_x(before, cursor[0]), to_math_y(before, cursor[1])];
            let (next, changed) = wheel_view(view, -100.0, cursor, center);
            assert!(changed);
            assert_eq!(next.zoom, view.zoom * 0.15f64.exp());
            let after = unsnapped(next);
            close(to_screen_x(after, held[0]), cursor[0]);
            close(to_screen_y(after, held[1]), cursor[1]);
            view = next;
        }
        close(view.zoom, 50.0 * 0.45f64.exp());
        let top = ViewState {
            zoom: MAX_UNIT_SIZE,
            pan_x: 1.0,
            pan_y: 2.0,
        };
        let (same, changed) = wheel_view(top, -100.0, cursor, center);
        assert!(!changed);
        assert_eq!(same, top);
        let (_, out) = wheel_view(
            ViewState {
                zoom: MIN_UNIT_SIZE,
                pan_x: 0.0,
                pan_y: 0.0,
            },
            100.0,
            cursor,
            center,
        );
        assert!(!out);
        let (clamped, _) = wheel_view(
            ViewState {
                zoom: 99.0,
                pan_x: 0.0,
                pan_y: 0.0,
            },
            -100.0,
            cursor,
            center,
        );
        assert_eq!(clamped.zoom, MAX_UNIT_SIZE);
    }

    #[test]
    fn a_release_flings_at_its_velocity_over_the_last_120_ms() {
        let flick = [
            DragSample {
                at: [0.0, 0.0],
                t: 1000.0,
            },
            DragSample {
                at: [90.0, 0.0],
                t: 1030.0,
            },
            DragSample {
                at: [180.0, 0.0],
                t: 1060.0,
            },
            DragSample {
                at: [180.0, 0.0],
                t: 1090.0,
            },
        ];
        let (v, ok) = velocity_of_samples(&flick);
        assert!(ok);
        close(v[0], 180.0 / 0.09);
        assert_eq!(v[1], 0.0);
        let mut samples = dynamic_of(&[
            DragSample {
                at: [0.0, 0.0],
                t: 0.0,
            },
            DragSample {
                at: [10.0, 0.0],
                t: 50.0,
            },
            DragSample {
                at: [20.0, 0.0],
                t: 100.0,
            },
            DragSample {
                at: [30.0, 0.0],
                t: 200.0,
            },
        ]);
        prune_samples(&mut samples);
        assert_eq!(samples.len(), 2);
        assert_eq!(samples[0].t, 100.0);
        let mut held = dynamic_of(&[
            DragSample {
                at: [0.0, 0.0],
                t: 0.0,
            },
            DragSample {
                at: [50.0, 0.0],
                t: 500.0,
            },
        ]);
        prune_samples(&mut held);
        assert_eq!(held.len(), 2);
        let (_, one) = velocity_of_samples(&flick[..1]);
        assert!(!one);
        let (_, still) = velocity_of_samples(&[
            DragSample {
                at: [0.0, 0.0],
                t: 10.0,
            },
            DragSample {
                at: [50.0, 0.0],
                t: 10.0,
            },
        ]);
        assert!(!still);
        let (_, late) = velocity_of_samples(&[
            DragSample {
                at: [0.0, 0.0],
                t: 0.0,
            },
            DragSample {
                at: [40.0, 0.0],
                t: 16.0,
            },
            DragSample {
                at: [40.0, 0.0],
                t: 300.0,
            },
        ]);
        assert!(!late);
        let (_, slow) = velocity_of_samples(&[
            DragSample {
                at: [0.0, 0.0],
                t: 0.0,
            },
            DragSample {
                at: [2.0, 0.0],
                t: 100.0,
            },
        ]);
        assert!(!slow);
        let fast = clamp_launch_speed([30000.0, 40000.0]);
        close(fast[0], 4800.0);
        close(fast[1], 6400.0);
        let (_, slow_start) = glide_start([20.0, 10.0], 0.0);
        assert!(!slow_start);
    }

    static REFERENCE_FLING: &[f64] = &[
        568.0, 583.5, 598.0, 611.5, 624.0, 635.5, 646.5, 656.5, 666.0, 674.5, 682.5, 690.0, 697.0,
        703.5, 710.0, 715.5, 720.5, 725.5, 730.0, 734.5, 738.5, 742.0, 745.5, 748.5, 751.5, 754.5,
        757.0, 759.5, 761.5, 763.5, 765.5, 767.5, 769.0, 770.5, 772.0, 773.5, 774.5, 775.5, 777.0,
        778.0, 778.5, 779.5, 780.5, 781.0, 782.0, 782.5, 783.0, 783.5, 784.0, 784.5, 785.0, 785.5,
        786.0,
    ];

    #[test]
    fn a_fling_glides_frame_by_frame_as_the_reference_does() {
        let mut samples = dynamic_of(&[DragSample {
            at: [700.0, 600.0],
            t: 2032.0,
        }]);
        let mut view = ViewState {
            zoom: 50.0,
            pan_x: 0.0,
            pan_y: 0.0,
        };
        let mut last = 700.0;
        for (i, x) in [717.0, 733.0, 750.0, 767.0, 783.0, 800.0]
            .iter()
            .enumerate()
        {
            samples.push(DragSample {
                at: [*x, 600.0],
                t: 2032.0 + 16.0 * i as f64,
            });
            prune_samples(&mut samples);
            view.pan_x += (x - last) / view.zoom;
            last = *x;
        }
        samples.push(DragSample {
            at: [800.0, 600.0],
            t: 2128.0,
        });
        let (v, ok) = velocity_of_samples(&samples);
        assert!(ok);
        let (mut glide, launched) = glide_start(v, 2128.0);
        assert!(launched);
        let origin = |view: ViewState| -> f64 { make_viewport(936.0, 792.0, 2.0, view).origin_x };
        assert_eq!(origin(view), REFERENCE_FLING[0]);
        let mut now = 2128.0;
        let mut frames = 0;
        loop {
            now += 16.0;
            let (step, gliding) = glide_step(&mut glide, now, view.zoom);
            if !gliding {
                break;
            }
            view.pan_x += step[0];
            view.pan_y -= step[1];
            frames += 1;
            if frames < REFERENCE_FLING.len() {
                assert!(
                    origin(view) == REFERENCE_FLING[frames],
                    "frame {frames}: origin {}, the reference's {}",
                    origin(view),
                    REFERENCE_FLING[frames],
                );
            }
        }
        assert_eq!(frames, REFERENCE_FLING.len() - 1);
        assert_eq!(origin(view), REFERENCE_FLING[REFERENCE_FLING.len() - 1]);
        let mut g = Glide {
            v: [1000.0, 0.0],
            last: 0.0,
        };
        let (step, _) = glide_step(&mut g, 500.0, 50.0);
        close(
            step[0],
            1000.0 * (-INERTIA_DECAY * 0.05).exp() * 0.05 / 50.0,
        );
    }

    #[test]
    fn a_dragged_mark_moves_as_far_as_the_pointer() {
        let vp = make_viewport(
            600.0,
            600.0,
            1.0,
            ViewState {
                zoom: 50.0,
                pan_x: 0.0,
                pan_y: 0.0,
            },
        );
        let at =
            |vp: Viewport, px: f64, py: f64| -> [f64; 2] { [to_math_x(vp, px), to_math_y(vp, py)] };
        assert_eq!(
            drag_to(
                at(vp, 354.0, 246.0),
                [1.0, 1.0],
                at(vp, 404.0, 246.0),
                false
            ),
            [2.0, 1.0]
        );
        let from = at(vp, 370.0, 250.0);
        assert_eq!(
            drag_to(from, [1.0, 1.0], at(vp, 395.0, 225.0), false),
            [1.5, 1.5]
        );
        assert_eq!(
            drag_to(from, [1.0, 1.0], at(vp, 420.0, 240.0), true),
            [2.0, 1.0]
        );
        assert_eq!(
            drag_to(from, [1.0, 1.0], at(vp, 380.0, 150.0), true),
            [1.0, 3.0]
        );
    }

    #[test]
    fn the_arrows_pan_one_minor_grid_square() {
        assert_eq!(pan_square(50.0), 0.5);
        assert_eq!(pan_square(62.5), 0.5);
        assert_eq!(pan_square(40.0), 0.5);
        assert_eq!(pan_square(2.0), 10.0);
    }
}
