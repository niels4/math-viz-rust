//! The flip animation: a damped spring turning the curve through flat.
//!
//! Ported from math-viz2 at `5cef052`
//! (`src/components/views/FunctionViewer/useShownTransform.ts`): the flip
//! steps through [`ShownTransform::step`], once per frame on the app's
//! clock, exactly like the hook's `requestAnimationFrame` loop. The coder
//! owns the `flip` value (h6-curve appends `App.state.flip:
//! Option<Flip>`) and wires the app's reduced-motion setting in as
//! `can_animate`, as `plot::Shown::switch_to` does.

use crate::transform_curves::{Transform, TransformParam};

/// How long the flip takes, in ms (`durationMs`).
pub const DURATION_MS: f64 = 1000.0;

/// The spring settles this far into the flip (`springSettleMs`): the curve
/// has rung through flat and rests on the new value by 702 ms of the
/// second it is given.
pub const SPRING_SETTLE_MS: f64 = 702.0;

/// The first frame of a flip counts as one frame in (`firstFrameMs`, as in
/// `plot.rs` and `camera.rs`): the rAF loop's first callback carries no
/// measured gap.
pub const FIRST_FRAME_MS: f64 = 1000.0 / 60.0;

/// The flip spring's stiffness (`stiffness`).
pub const STIFFNESS: f64 = 170.0;

/// The flip spring's damping (`damping`).
pub const DAMPING: f64 = 26.0;

/// A pending flip: the parameter, and the value it flips from and to
/// (`flipAtom`). Compared by value for the hook's `flip !== seenFlip`
/// check; two flips always differ in `from`/`to` (each click negates the
/// value), except a flip of `0` against `-0`, which shows nothing either
/// way.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Flip {
    pub param: TransformParam,
    pub from: f64,
    pub to: f64,
}

/// The damped spring's share of the way from `from` to `to` at `ms` after
/// the flip (`springAt`): 1 minus the decaying oscillation at
/// `t = ((ms / duration) * settle) / 1000`, with `decay = damping / 2` and
/// `ringing = sqrt(stiffness - decay²)`.
#[must_use]
pub fn spring_at(ms: f64) -> f64 {
    let decay = DAMPING / 2.0;
    let ringing = (STIFFNESS - decay * decay).sqrt();
    let t = ((ms / DURATION_MS) * SPRING_SETTLE_MS) / 1000.0;
    1.0 - (-decay * t).exp() * ((ringing * t).cos() + (decay / ringing) * (ringing * t).sin())
}

/// A flip in flight: what the plane showed when it started, what it turns
/// toward, and the ms since (`Jump`). Like `plot::Shown`, the next step is
/// the switch's first frame and carries [`FIRST_FRAME_MS`].
struct Jump {
    from: Transform,
    to: Transform,
    elapsed: f64,
    first_frame: bool,
}

/// What the flip animation shows: the pending flip it has seen (the hook's
/// `seenFlip`) and the jump in flight, if any. The live transform stays
/// with the caller (the query params); `step` blends the two.
#[derive(Default)]
pub struct ShownTransform {
    jump: Option<Jump>,
    seen: Option<Flip>,
}

impl ShownTransform {
    /// The steady state: no flip seen, nothing in flight.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether a flip is still turning the curve (the hook's rAF loop is
    /// still scheduled; the vitest's `frames.size`).
    #[must_use]
    pub fn is_animating(&self) -> bool {
        self.jump.is_some()
    }

    /// The transform one frame shows (`shownDuring(jump, transform)`): the
    /// live values, except each parameter still travelling toward the
    /// jump's target, which blends from `from` to `to` at the spring's
    /// share. A value typed mid-turn differs from the target and shows at
    /// once, as the TypeScript's `===` check does.
    fn shown(&self, transform: Transform) -> Transform {
        let Some(jump) = &self.jump else {
            return transform;
        };
        let share = spring_at(jump.elapsed);
        let mut out = transform;
        for param in [
            TransformParam::A,
            TransformParam::B,
            TransformParam::H,
            TransformParam::K,
        ] {
            if transform.get(param) == jump.to.get(param) {
                out = out.with(
                    param,
                    jump.from.get(param) + (jump.to.get(param) - jump.from.get(param)) * share,
                );
            }
        }
        out
    }

    /// One animation frame: adopt a flip the animation has not seen (the
    /// hook's render-phase step — `from` is what the plane shows with the
    /// flipped param at its old value, `to` is the live transform with it
    /// at its new value — unless the flip is null or motion is reduced),
    /// advance the jump by the frame's `dt`, and return the transform to
    /// show. Past the duration the flip ends and the live transform shows
    /// whole. At rest the state is untouched and the live transform
    /// returns, so stepping past the end is harmless (the hook's loop
    /// simply no longer runs).
    pub fn step(
        &mut self,
        flip: Option<Flip>,
        transform: Transform,
        dt: f64,
        can_animate: bool,
    ) -> Transform {
        if self.seen != flip {
            self.seen = flip;
            if let Some(pending) = flip
                && can_animate
            {
                let from = self.shown(transform).with(pending.param, pending.from);
                self.jump = Some(Jump {
                    from,
                    to: transform.with(pending.param, pending.to),
                    elapsed: 0.0,
                    first_frame: true,
                });
            }
        }
        let finished = if let Some(jump) = self.jump.as_mut() {
            let dt = if jump.first_frame { FIRST_FRAME_MS } else { dt };
            jump.first_frame = false;
            if jump.elapsed + dt >= DURATION_MS {
                true
            } else {
                jump.elapsed += dt;
                false
            }
        } else {
            false
        };
        if finished {
            self.jump = None;
        }
        self.shown(transform)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `toBeCloseTo(expected, 2)`: within half a hundredth (the `plot.rs`
    /// precedent).
    fn assert_close_to(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 0.005,
            "expected {actual} to be close to {expected}"
        );
    }

    /// The test rig: the live transform (the query params the box writes at
    /// once), the pending flip, and the last plane output. Frames run 1000/60
    /// ms apart, exactly like the vitest `runFrames`.
    struct Rig {
        live: Transform,
        flip: Option<Flip>,
        shown: ShownTransform,
        plane: Transform,
        can_animate: bool,
    }

    impl Rig {
        fn at_a2() -> Self {
            let live = Transform {
                a: 2.0,
                ..Transform::IDENTITY
            };
            Self {
                live,
                flip: None,
                shown: ShownTransform::new(),
                plane: live,
                can_animate: true,
            }
        }

        /// The flip button: the flip pends and the value writes through at
        /// once (the box shows the new value while the plane still turns).
        fn click_flip(&mut self) {
            let value = self.live.a;
            self.flip = Some(Flip {
                param: TransformParam::A,
                from: value,
                to: -value,
            });
            self.live.a = -value;
        }

        fn frames(&mut self, count: i32) {
            for _ in 0..count {
                self.plane =
                    self.shown
                        .step(self.flip, self.live, FIRST_FRAME_MS, self.can_animate);
            }
        }
    }

    #[test]
    fn flipping_a_2_turns_the_curve_over_through_flat_while_the_box_shows_minus_2_at_once() {
        let mut rig = Rig::at_a2();
        assert_close_to(rig.plane.a, 2.0);

        rig.click_flip();
        // The box shows the new value at once; the plane still shows the old.
        assert_eq!(rig.live.a, -2.0);
        assert_close_to(rig.plane.a, 2.0);

        rig.frames(6);
        assert_close_to(rig.plane.a, 1.07);

        rig.frames(6);
        assert_close_to(rig.plane.a, -0.19);

        rig.frames(31);
        assert_close_to(rig.plane.a, -1.96);

        rig.frames(18);
        assert_close_to(rig.plane.a, -2.0);
        assert!(!rig.shown.is_animating());
    }

    #[test]
    fn a_value_typed_while_the_curve_turns_over_shows_at_once() {
        let mut rig = Rig::at_a2();

        rig.click_flip();
        rig.frames(6);
        assert_close_to(rig.plane.a, 1.07);

        // Typing -1 and pressing Enter writes the live value at once.
        rig.live.a = -1.0;
        rig.frames(1);
        assert_close_to(rig.plane.a, -1.0);
    }

    #[test]
    fn with_reduced_motion_flipping_a_2_shows_minus_2_at_once() {
        let mut rig = Rig::at_a2();
        rig.can_animate = false;
        assert_close_to(rig.plane.a, 2.0);

        rig.click_flip();
        rig.frames(1);
        assert_close_to(rig.plane.a, -2.0);
        assert!(!rig.shown.is_animating());
    }
}
