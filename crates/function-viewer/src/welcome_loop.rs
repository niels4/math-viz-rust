//! The welcome page's animation loop: the 22-step transform tour and the
//! clock that feeds it.
//!
//! Ported from math-viz2 at `68e1c7b`:
//!
//! - `src/components/views/WelcomePage/welcomeLoop.ts`: [`LoopState`],
//!   [`Step`], [`FPS`], [`LOOP_MS`], [`WELCOME_STILL`], [`cubic_bezier`],
//!   [`ease_drag`], [`dragged`], [`loop_at`].
//! - `src/components/views/WelcomePage/useWelcomeLoop.ts`:
//!   [`WelcomeClock`] (the rAF accumulator as a step function for the
//!   coder's frame loop), [`MAX_FRAME_MS`].

use crate::functions::FunctionSlug;
use crate::plot::FIRST_FRAME_MS;
use crate::scrub::{SCALE_PER_PX, SHIFT_PER_PX, ScrubMode, scrubbed};
use crate::state::TransformParam;

/// One frame of the welcome tour: the shown function and its four
/// transform values (`LoopState` in `welcomeLoop.ts`: a flat struct, not
/// `{ func } & Transform`, since the tests write every field as a literal).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LoopState {
    pub func: FunctionSlug,
    pub a: f64,
    pub b: f64,
    pub h: f64,
    pub k: f64,
}

impl LoopState {
    /// The parameter's value, as `Transform::get` reads it.
    #[must_use]
    pub fn get(&self, param: TransformParam) -> f64 {
        match param {
            TransformParam::A => self.a,
            TransformParam::B => self.b,
            TransformParam::H => self.h,
            TransformParam::K => self.k,
        }
    }

    /// A copy with the parameter set, as `{ ...state, [param]: value }`
    /// writes it.
    #[must_use]
    pub fn with(&self, param: TransformParam, value: f64) -> Self {
        let mut out = *self;
        match param {
            TransformParam::A => out.a = value,
            TransformParam::B => out.b = value,
            TransformParam::H => out.h = value,
            TransformParam::K => out.k = value,
        }
        out
    }
}

/// One tour entry (`Step` in `welcomeLoop.ts`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Step {
    Hold {
        frames: u32,
    },
    Drag {
        frames: u32,
        param: TransformParam,
        to: f64,
    },
    Switch {
        frames: u32,
        func: FunctionSlug,
    },
}

impl Step {
    /// The entry's length in frames, as the walk reads `step.frames`.
    #[must_use]
    pub const fn frames(&self) -> u32 {
        match self {
            Self::Hold { frames } | Self::Drag { frames, .. } | Self::Switch { frames, .. } => {
                *frames
            }
        }
    }
}

/// Frames per second of the tour (`fps`).
pub const FPS: f64 = 24.0;

/// Where the tour starts: x² at rest (`start`).
const START: LoopState = LoopState {
    func: FunctionSlug::X2,
    a: 1.0,
    b: 1.0,
    h: 0.0,
    k: 0.0,
};

/// The tour in order: holds, coarse scrub drags and function switches
/// (`steps`).
pub const STEPS: [Step; 22] = [
    Step::Hold { frames: 29 },
    Step::Drag {
        frames: 29,
        param: TransformParam::K,
        to: -1.5,
    },
    Step::Hold { frames: 10 },
    Step::Drag {
        frames: 24,
        param: TransformParam::H,
        to: 1.0,
    },
    Step::Hold { frames: 14 },
    Step::Switch {
        frames: 26,
        func: FunctionSlug::SinX,
    },
    Step::Hold { frames: 14 },
    Step::Drag {
        frames: 34,
        param: TransformParam::A,
        to: 3.0,
    },
    Step::Hold { frames: 10 },
    Step::Drag {
        frames: 29,
        param: TransformParam::K,
        to: 0.0,
    },
    Step::Hold { frames: 10 },
    Step::Drag {
        frames: 29,
        param: TransformParam::B,
        to: 2.0,
    },
    Step::Hold { frames: 14 },
    Step::Switch {
        frames: 26,
        func: FunctionSlug::X3,
    },
    Step::Hold { frames: 14 },
    Step::Drag {
        frames: 24,
        param: TransformParam::H,
        to: 0.0,
    },
    Step::Hold { frames: 14 },
    Step::Switch {
        frames: 26,
        func: FunctionSlug::X2,
    },
    Step::Hold { frames: 10 },
    Step::Drag {
        frames: 29,
        param: TransformParam::A,
        to: 1.0,
    },
    Step::Hold { frames: 7 },
    Step::Drag {
        frames: 29,
        param: TransformParam::B,
        to: 1.0,
    },
];

/// The tour's frame count, summed as `steps.reduce` sums it (451).
const fn total_frames() -> u32 {
    let mut sum = 0;
    let mut i = 0;
    while i < STEPS.len() {
        sum += STEPS[i].frames();
        i += 1;
    }
    sum
}

/// The tour's length in ms (`loopMs`: the frame sum times 1000 over `fps`,
/// 18 791.666…).
pub const LOOP_MS: f64 = total_frames() as f64 * 1000.0 / FPS;

/// What reduced motion shows: sin x grown tall over a lifted wave
/// (`welcomeStill`).
pub const WELCOME_STILL: LoopState = LoopState {
    func: FunctionSlug::SinX,
    a: 3.0,
    b: 1.48,
    h: 1.0,
    k: 0.0,
};

/// A cubic-bezier easing built once and evaluated many times: a closure is
/// neither `const`-buildable nor `Copy`, so the four control values ride in
/// this struct and [`ease_drag`] evaluates it.
#[derive(Clone, Copy, Debug)]
struct Easing {
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
}

impl Easing {
    /// The eased y for progress `x`: the 60-step bisection on x exactly as
    /// the source writes it (`** 2`/`** 3` as `powi`, same op order, so the
    /// values round the same).
    fn at(&self, x: f64) -> f64 {
        fn curve(t: f64, p1: f64, p2: f64) -> f64 {
            // `3 * p1 * t * (1 - t) ** 2 + 3 * p2 * t ** 2 * (1 - t) + t ** 3`
            3.0 * p1 * t * (1.0 - t).powi(2) + 3.0 * p2 * t.powi(2) * (1.0 - t) + t.powi(3)
        }
        let mut lo = 0.0;
        let mut hi = 1.0;
        for _ in 0..60 {
            let mid = (lo + hi) / 2.0;
            if curve(mid, self.x1, self.x2) < x {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        curve((lo + hi) / 2.0, self.y1, self.y2)
    }
}

/// The cubic-bezier easing over a value (`(x1, y1, x2, y2) => (x) => y`).
/// Closures are already `#[must_use]`, so no attribute here.
pub fn cubic_bezier(x1: f64, y1: f64, x2: f64, y2: f64) -> impl Fn(f64) -> f64 {
    let easing = Easing { x1, y1, x2, y2 };
    move |x| easing.at(x)
}

/// The drag easing (`easeDrag`: 0.4, 0, 0.2, 1).
#[must_use]
pub fn ease_drag(progress: f64) -> f64 {
    Easing {
        x1: 0.4,
        y1: 0.0,
        x2: 0.2,
        y2: 1.0,
    }
    .at(progress)
}

/// The value `progress` (0..1) through a drag from `from` to `to`
/// (`dragged`): `to` exactly at 1, else a coarse [`scrubbed`] drag eased
/// along the travel in px — `ln(to / from) / ln(1 + SCALE_PER_PX)` for
/// scales, `(to - from) / SHIFT_PER_PX` for shifts.
#[must_use]
pub fn dragged(from: f64, to: f64, is_scale: bool, progress: f64) -> f64 {
    if progress >= 1.0 {
        return to;
    }
    let travel = if is_scale {
        (to / from).ln() / (1.0 + SCALE_PER_PX).ln()
    } else {
        (to - from) / SHIFT_PER_PX
    };
    scrubbed(
        from,
        ease_drag(progress) * travel,
        is_scale,
        ScrubMode::Coarse,
    )
}

/// The tour state `ms` after the mount (`loopAt`).
///
/// `frame = ((ms % LOOP_MS) * FPS) / 1000`, walked through the steps: a
/// switch applies at its step's start, a drag at
/// `min(1, frame / step.frames)`, and the step returns while
/// `frame <= step.frames`. `%` is Rust's float remainder, which matches the
/// source's JS `%` for the non-negative `ms` the clock feeds it (both keep
/// the dividend's sign); a negative `ms` would wrap differently than
/// `rem_euclid`, as it would in the source.
#[must_use]
pub fn loop_at(ms: f64) -> LoopState {
    let mut frame = ((ms % LOOP_MS) * FPS) / 1000.0;
    let mut state = START;
    for step in STEPS {
        match step {
            Step::Switch { func, .. } => {
                state.func = func;
            }
            Step::Drag { param, to, .. } => {
                let progress = (frame / f64::from(step.frames())).min(1.0);
                let is_scale = matches!(param, TransformParam::A | TransformParam::B);
                state = state.with(param, dragged(state.get(param), to, is_scale, progress));
            }
            Step::Hold { .. } => {}
        }
        if frame <= f64::from(step.frames()) {
            return state;
        }
        frame -= f64::from(step.frames());
    }
    state
}

/// The longest frame the accumulator keeps (`maxFrameMs` in
/// `useWelcomeLoop.ts`: a backgrounded tab's rAF gap never jumps the tour).
pub const MAX_FRAME_MS: f64 = 50.0;

/// The rAF accumulator as a step function for the coder's frame loop
/// (`useWelcomeLoop`'s `elapsed`/`last` state): `first_frame` adds
/// [`FIRST_FRAME_MS`] regardless of the gap (cf. `plot.rs`'s `Shown` and
/// `camera.rs`), later frames add `min(now - last, 50)`.
#[derive(Clone, Copy, Debug)]
pub struct WelcomeClock {
    elapsed: f64,
    last: Option<f64>,
}

impl WelcomeClock {
    /// A fresh mount's clock: nothing elapsed, no frame seen.
    #[must_use]
    pub fn new() -> Self {
        Self {
            elapsed: 0.0,
            last: None,
        }
    }

    /// The rAF callback at `now_ms`: advance the tour by the frame's `dt`.
    pub fn step(&mut self, now_ms: f64) {
        let dt = self
            .last
            .map_or(FIRST_FRAME_MS, |last| (now_ms - last).min(MAX_FRAME_MS));
        self.last = Some(now_ms);
        self.elapsed += dt;
    }

    /// Forgets the last frame's stamp, so the next [`step`](Self::step) adds
    /// [`FIRST_FRAME_MS`] again.
    ///
    /// The hook's `last` lives in the effect, and the effect is keyed on
    /// `animate`: every time the animation turns off and on again, the
    /// effect restarts with `last === null` and the first frame adds
    /// `firstFrameMs`. The port's clock is one state that outlives the
    /// effect, so the still path drops the stamp to stand for the effect's
    /// restart. The elapsed time is the hook's `useState` and stays.
    pub fn forget_stamp(&mut self) {
        self.last = None;
    }

    /// The shown state: the tour at the elapsed time, or the still when
    /// animation is off (`canAnimate`, from the reduced-motion setting).
    #[must_use]
    pub fn state(&self, can_animate: bool) -> LoopState {
        if can_animate {
            loop_at(self.elapsed)
        } else {
            WELCOME_STILL
        }
    }

    /// The elapsed tour time in ms, for the coder's `loop` views line.
    #[must_use]
    pub fn elapsed(&self) -> f64 {
        self.elapsed
    }

    /// Whether the coder's frame loop should keep scheduling frames: only
    /// while animation is on (under reduced motion the page rests on the
    /// still and requests no frames).
    #[must_use]
    pub fn is_animating(can_animate: bool) -> bool {
        can_animate
    }
}

impl Default for WelcomeClock {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // `frameEnd(frame) = (frame + 1) * 1000 / 24` in `welcomeLoop.test.ts`.
    fn frame_end(frame: u32) -> f64 {
        (f64::from(frame) + 1.0) * 1000.0 / 24.0
    }

    const AT_REST: LoopState = LoopState {
        func: FunctionSlug::X2,
        a: 1.0,
        b: 1.0,
        h: 0.0,
        k: 0.0,
    };

    #[test]
    fn starts_on_x2_at_rest() {
        assert_eq!(loop_at(0.0), AT_REST);
        assert_eq!(loop_at(frame_end(28)), AT_REST);
    }

    #[test]
    fn moves_x2_down_to_k_minus_1_5_fast_at_first_and_slow_onto_the_value() {
        assert_eq!(loop_at(frame_end(29)), AT_REST);
        assert_eq!(
            loop_at(frame_end(43)),
            LoopState {
                k: -1.19,
                ..AT_REST
            }
        );
        assert_eq!(loop_at(frame_end(57)), LoopState { k: -1.5, ..AT_REST });
    }

    #[test]
    fn switches_to_sin_x_at_the_click_keeping_the_transform() {
        assert_eq!(
            loop_at(frame_end(105)),
            LoopState {
                func: FunctionSlug::X2,
                k: -1.5,
                h: 1.0,
                ..AT_REST
            }
        );
        assert_eq!(
            loop_at(frame_end(106)),
            LoopState {
                func: FunctionSlug::SinX,
                k: -1.5,
                h: 1.0,
                ..AT_REST
            }
        );
    }

    #[test]
    fn grows_the_wave_to_a_3() {
        assert_eq!(
            loop_at(frame_end(163)),
            LoopState {
                func: FunctionSlug::SinX,
                a: 2.44,
                k: -1.5,
                h: 1.0,
                ..AT_REST
            }
        );
        assert_eq!(
            loop_at(frame_end(179)),
            LoopState {
                func: FunctionSlug::SinX,
                a: 3.0,
                k: -1.5,
                h: 1.0,
                ..AT_REST
            }
        );
    }

    #[test]
    fn widens_the_wave_to_b_2_passing_the_still_half_way() {
        assert_eq!(loop_at(frame_end(239)), WELCOME_STILL);
        assert_eq!(
            loop_at(frame_end(243)),
            LoopState {
                func: FunctionSlug::SinX,
                a: 3.0,
                k: 0.0,
                b: 1.74,
                h: 1.0,
            }
        );
        assert_eq!(
            loop_at(frame_end(257)),
            LoopState {
                func: FunctionSlug::SinX,
                a: 3.0,
                k: 0.0,
                b: 2.0,
                h: 1.0,
            }
        );
    }

    #[test]
    fn brings_x2_back_to_rest_by_shrinking_a_then_b() {
        assert_eq!(
            loop_at(frame_end(349)),
            LoopState {
                func: FunctionSlug::X3,
                a: 3.0,
                k: 0.0,
                b: 2.0,
                h: 0.0,
            }
        );
        assert_eq!(
            loop_at(frame_end(350)),
            LoopState {
                func: FunctionSlug::X2,
                a: 3.0,
                k: 0.0,
                b: 2.0,
                h: 0.0,
            }
        );
        assert_eq!(
            loop_at(frame_end(400)),
            LoopState {
                a: 1.25,
                b: 2.0,
                ..AT_REST
            }
        );
        assert_eq!(loop_at(frame_end(414)), LoopState { b: 2.0, ..AT_REST });
        assert_eq!(loop_at(frame_end(436)), LoopState { b: 1.15, ..AT_REST });
        assert_eq!(loop_at(frame_end(450)), AT_REST);
    }

    #[test]
    fn starts_over_after_18_79_s() {
        assert!((LOOP_MS - 18_791.67).abs() < 0.005);
        assert_eq!(loop_at(frame_end(449)), LoopState { b: 1.0, ..AT_REST });
        assert_eq!(loop_at(18_792.0), AT_REST);
        assert_eq!(
            loop_at(LOOP_MS + frame_end(43)),
            LoopState {
                k: -1.19,
                ..AT_REST
            }
        );
    }

    // `useWelcomeLoop.test.ts`: the hook's two tests against the step
    // function (the rAF stub calls each scheduled callback with
    // `frameTime += 1000/60`; the first frame's dt is the constant, not the
    // gap).
    fn run_frames(clock: &mut WelcomeClock, now_ms: &mut f64, count: u32) {
        for _ in 0..count {
            *now_ms += 1000.0 / 60.0;
            clock.step(*now_ms);
        }
    }

    #[test]
    fn rests_on_x2_for_1_2_s_then_moves_it_down() {
        let mut clock = WelcomeClock::new();
        let mut now_ms = 0.0;
        assert_eq!(clock.state(true), AT_REST);

        run_frames(&mut clock, &mut now_ms, 72);
        assert_eq!(clock.state(true), AT_REST);

        run_frames(&mut clock, &mut now_ms, 38);
        assert_eq!(
            clock.state(true),
            LoopState {
                k: -1.19,
                ..AT_REST
            }
        );

        run_frames(&mut clock, &mut now_ms, 35);
        assert_eq!(clock.state(true), LoopState { k: -1.5, ..AT_REST });
    }

    #[test]
    fn with_reduced_motion_shows_the_still_and_requests_no_frames() {
        let clock = WelcomeClock::new();
        assert_eq!(clock.state(false), WELCOME_STILL);
        assert!(!WelcomeClock::is_animating(false));
        assert!(WelcomeClock::is_animating(true));
    }
}
