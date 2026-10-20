//! The reset and flip animation: a damped spring turning the curve through
//! flat and back.
//!
//! Ported from math-viz2 at `251c080`
//! (`src/components/views/FunctionViewer/useShownTransform.ts`): the jumps
//! step through [`ShownTransform::step`], once per frame on the app's clock,
//! exactly like the hook's `requestAnimationFrame` loop. The coder owns the
//! jump list (h7-sidebar sets `App.state.jumps` from the flip, a row's reset
//! and Reset all) and wires the app's reduced-motion setting in as
//! `can_animate`, as `plot::Shown::switch_to` does.

use crate::state::{Jumps, TransformParam};
use crate::transform_curves::Transform;

/// How long a jump takes, in ms (`durationMs`).
pub const DURATION_MS: f64 = 1000.0;

/// The spring settles this far into the jump (`springSettleMs`): the curve
/// has rung through flat and rests on the new value by 702 ms of the second
/// it is given.
pub const SPRING_SETTLE_MS: f64 = 702.0;

/// The first frame of a jump counts as one frame in (`firstFrameMs`, as in
/// `plot.rs` and `camera.rs`): the rAF loop's first callback carries no
/// measured gap.
pub const FIRST_FRAME_MS: f64 = 1000.0 / 60.0;

/// The jump spring's stiffness (`stiffness`).
pub const STIFFNESS: f64 = 170.0;

/// The jump spring's damping (`damping`).
pub const DAMPING: f64 = 26.0;

/// The damped spring's share of the way from `from` to `to` at `ms` after
/// the jump (`springAt`): 1 minus the decaying oscillation at
/// `t = ((ms / duration) * settle) / 1000`, with `decay = damping / 2` and
/// `ringing = sqrt(stiffness - decay²)`.
#[must_use]
pub fn spring_at(ms: f64) -> f64 {
    let decay = DAMPING / 2.0;
    let ringing = (STIFFNESS - decay * decay).sqrt();
    let t = ((ms / DURATION_MS) * SPRING_SETTLE_MS) / 1000.0;
    1.0 - (-decay * t).exp() * ((ringing * t).cos() + (decay / ringing) * (ringing * t).sin())
}

/// A jump in flight: what the plane showed when it started, what it turns
/// toward, and the ms since (`Jump`). Like `plot::Shown`, the next step is
/// the switch's first frame and carries [`FIRST_FRAME_MS`].
struct Jump {
    from: Transform,
    to: Transform,
    elapsed: f64,
    first_frame: bool,
}

/// What the plane animation shows: the jumps identity it has seen (the
/// hook's `seenJumps`) and the jump in flight, if any. The live transform
/// stays with the caller (the query params); `step` blends the two.
#[derive(Default)]
pub struct ShownTransform {
    jump: Option<Jump>,
    seen: u64,
}

impl ShownTransform {
    /// The steady state: no jumps seen, nothing in flight.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether a jump is still turning the curve (the hook's rAF loop is
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
    pub fn shown(&self, transform: Transform) -> Transform {
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

    /// Adopt jumps the animation has not seen, without advancing (the hook's
    /// render-phase step): the jump starts at elapsed 0 with its first frame
    /// pending, so the press's own turn still draws the old value. With no
    /// new jumps, or motion reduced, only the seen mark moves.
    pub fn adopt(&mut self, jumps: &Jumps, transform: Transform, can_animate: bool) {
        if self.seen != jumps.seq {
            self.seen = jumps.seq;
            if can_animate {
                let ends = |end: fn(&crate::state::Jump) -> f64| {
                    jumps.list.iter().fold(transform, |t, j| {
                        let value = end(j);
                        t.with(j.param, value)
                    })
                };
                let from = self.shown(ends(|j| j.from));
                self.jump = Some(Jump {
                    from,
                    to: ends(|j| j.to),
                    elapsed: 0.0,
                    first_frame: true,
                });
            }
        }
    }

    /// One animation frame: adopt through [`adopt`](Self::adopt), advance
    /// the jump by the frame's `dt`, and return the transform to show
    /// (`from` is what the plane shows with the jumped params at their old
    /// values, `to` is the live transform with them at their new values).
    /// Past the duration the jump ends and the live transform shows whole.
    /// At rest the state is untouched and the live transform returns, so
    /// stepping past the end is harmless (the hook's loop simply no longer
    /// runs).
    pub fn step(
        &mut self,
        jumps: &Jumps,
        transform: Transform,
        dt: f64,
        can_animate: bool,
    ) -> Transform {
        self.adopt(jumps, transform, can_animate);
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
    /// once), the pending jumps, and the last plane output. Frames run 1000/60
    /// ms apart, exactly like the vitest `runFrames`.
    struct Rig {
        live: Transform,
        jumps: Jumps,
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
                jumps: Jumps::default(),
                shown: ShownTransform::new(),
                plane: live,
                can_animate: true,
            }
        }

        fn at_a3_h2() -> Self {
            let live = Transform {
                a: 3.0,
                h: 2.0,
                ..Transform::IDENTITY
            };
            Self {
                live,
                jumps: Jumps::default(),
                shown: ShownTransform::new(),
                plane: live,
                can_animate: true,
            }
        }

        /// The flip button: the jump pends and the value writes through at
        /// once (the box shows the new value while the plane still turns).
        /// Every press mints a fresh jumps identity, as each click mints a
        /// fresh jumps array.
        fn click_flip(&mut self) {
            let value = self.live.a;
            self.jumps.single(TransformParam::A, value, -value);
            self.live.a = -value;
        }

        /// A row's reset: the jump pends and the default writes through at
        /// once (the box shows the new value while the plane still turns).
        fn click_reset(&mut self, param: TransformParam, default: f64) {
            let value = self.live.get(param);
            self.jumps.single(param, value, default);
            self.live = self.live.with(param, default);
        }

        /// Reset all: one jump per changed parameter, set whole at once.
        fn click_reset_all(&mut self, defaults: &[(TransformParam, f64)]) {
            let jumps: Vec<crate::state::Jump> = defaults
                .iter()
                .map(|(param, default)| crate::state::Jump {
                    param: *param,
                    from: self.live.get(*param),
                    to: *default,
                })
                .collect();
            self.jumps.set(jumps);
            for (param, default) in defaults {
                self.live = self.live.with(*param, *default);
            }
        }

        fn frames(&mut self, count: i32) {
            for _ in 0..count {
                self.plane =
                    self.shown
                        .step(&self.jumps, self.live, FIRST_FRAME_MS, self.can_animate);
            }
        }

        /// The press's own turn: adopt without advancing, then draw (the
        /// hook's render phase runs before any frame passes).
        fn press(&mut self) {
            self.shown.adopt(&self.jumps, self.live, self.can_animate);
            self.plane = self.shown.shown(self.live);
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

    #[test]
    fn the_press_turn_draws_2_00_before_any_frame_passes() {
        let mut rig = Rig::at_a2();

        rig.click_flip();
        // The box shows the new value at once; the press's own turn still
        // draws the old one, as the hook adopts the jump during render.
        assert_eq!(rig.live.a, -2.0);
        rig.press();
        assert_eq!(rig.plane.a, 2.0);
        assert!(rig.shown.is_animating());
    }

    #[test]
    fn a_re_flip_mid_spring_restarts_from_the_displayed_position() {
        let mut rig = Rig::at_a2();

        rig.click_flip();
        rig.frames(6);
        assert_close_to(rig.plane.a, 1.07);

        // Pressing flip again turns back from the displayed 1.07, not from
        // the box's -2: the hook blends the in-flight jump from the shown
        // value (`shownDuring(jump, ends("from"))`).
        rig.click_flip();
        assert_eq!(rig.live.a, 2.0);
        rig.frames(6);
        assert_close_to(rig.plane.a, 1.28);
        assert!(rig.shown.is_animating());
    }

    #[test]
    fn a_second_press_with_the_same_values_restarts_the_animation() {
        let mut rig = Rig::at_a2();

        rig.click_flip();
        rig.frames(61);
        assert_close_to(rig.plane.a, -2.0);
        assert!(!rig.shown.is_animating());

        // Typing the value back and pressing flip again pends the same
        // {param, from, to} with a fresh identity: the hook compares lists
        // by identity, not by value, so the animation restarts instead of
        // being swallowed.
        rig.live.a = 2.0;
        rig.click_flip();
        rig.frames(1);
        assert!(rig.shown.is_animating());
        assert_close_to(rig.plane.a, 1.96);
    }

    #[test]
    fn resetting_a_3_eases_the_curve_back_while_the_box_shows_1_at_once() {
        let mut rig = Rig::at_a3_h2();
        assert_close_to(rig.plane.a, 3.0);

        rig.click_reset(TransformParam::A, 1.0);
        // The box shows the default at once; the plane still shows the old.
        assert_eq!(rig.live.a, 1.0);
        assert_close_to(rig.plane.a, 3.0);

        rig.frames(6);
        assert_close_to(rig.plane.a, 2.53);

        rig.frames(6);
        assert_close_to(rig.plane.a, 1.91);

        rig.frames(31);
        assert_close_to(rig.plane.a, 1.02);

        rig.frames(18);
        assert_close_to(rig.plane.a, 1.0);
        assert!(!rig.shown.is_animating());
    }

    #[test]
    fn reset_all_eases_a_3_and_h_2_back_together_while_the_boxes_show_1_and_0_at_once() {
        let mut rig = Rig::at_a3_h2();
        assert_close_to(rig.plane.a, 3.0);
        assert_close_to(rig.plane.h, 2.0);

        rig.click_reset_all(&[(TransformParam::A, 1.0), (TransformParam::H, 0.0)]);
        // The boxes show the defaults at once; the plane still shows the old.
        assert_eq!(rig.live.a, 1.0);
        assert_eq!(rig.live.h, 0.0);
        assert_close_to(rig.plane.a, 3.0);
        assert_close_to(rig.plane.h, 2.0);

        rig.frames(6);
        assert_close_to(rig.plane.a, 2.53);
        assert_close_to(rig.plane.h, 1.53);

        rig.frames(6);
        assert_close_to(rig.plane.a, 1.91);
        assert_close_to(rig.plane.h, 0.91);

        rig.frames(31);
        assert_close_to(rig.plane.a, 1.02);
        assert_close_to(rig.plane.h, 0.02);

        rig.frames(18);
        assert_close_to(rig.plane.a, 1.0);
        assert_close_to(rig.plane.h, 0.0);
        assert!(!rig.shown.is_animating());
    }
}
