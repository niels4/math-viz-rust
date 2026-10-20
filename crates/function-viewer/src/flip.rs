//! The flip button's wiring: the pending flip and its step on the app's clock.
//!
//! Ported from math-viz2 at `5cef052`:
//!
//! - `SideBar/TransformSection/TransformRow.tsx` (the flip toggle's
//!   `onClick`): [`press_flip`] sets the pending flip (`{ param, from:
//!   value, to: -value }`) and the caller writes the negated value through
//!   at once, as `setFlip` + `setValue(-value, { flush: true })` do. The
//!   press identity ([`Flip::seq`]) mints from a counter advancing on every
//!   press, so a second press restarts the spring even with equal values.
//! - `FunctionViewer/useShownTransform.ts` (the render-phase step and the
//!   rAF loop): the press's own turn draws the adopted jump at elapsed 0
//!   (the plane still showing the old value), and [`flip_step`] advances
//!   from the turn after, the first step carrying [`FIRST_FRAME_MS`].
//!
//! The animation state machine itself ([`ShownTransform`], [`Flip`]) is
//! `shown_transform.rs` (h6-p-transform); this module owns the atom slot,
//! the press counter and the frame clock, and stays `App`-free so its tests
//! run headless: the caller passes the live transform and writes the
//! returned value through its params.
//!
//! Phase-2 wiring (after `claude-port` brings the porter modules, h4's
//! params and h5's rows): one call line from the flip button in
//! `transform_row.rs`, one step line in the frame loop, the draw reading
//! [`shown_transform`], the `views.json` shown line in `harness.rs`.
//!
//! Needs from `shown_transform.rs` (named change, applied at the merge):
//! `ShownTransform::adopt` (the render-phase half of `step`: adopt an
//! unseen flip without advancing) and `shown` made `pub` (the draw's read
//! without stepping).

use crate::shown_transform::{Flip, ShownTransform};
use crate::transform_curves::{Transform, TransformParam};

/// The first frame of a flip carries the hook's first-frame gap, as
/// `plot.rs`'s switch does (re-exported for the tests' frame rig).
pub use crate::shown_transform::FIRST_FRAME_MS;

/// The flip button's state: the pending flip (the `flipAtom` value), the
/// animation showing it, the press counter minting [`Flip::seq`], and when
/// the animation last stepped, on the app's clock (`None` while at rest and
/// on a press's own turn, as `plot.rs`'s `plot_frame_at` is).
#[derive(Default)]
pub struct FlipState {
    pending: Option<Flip>,
    shown: ShownTransform,
    presses: u64,
    frame_at: Option<crate::app_loop::Time>,
}

impl FlipState {
    /// The untouched state: no flip pending, nothing in flight.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether a flip is still turning the curve (the hook's rAF loop is
    /// still scheduled).
    #[must_use]
    pub fn is_animating(&self) -> bool {
        self.shown.is_animating()
    }

    /// The pending flip, if one was pressed.
    #[must_use]
    pub fn pending(&self) -> Option<Flip> {
        self.pending
    }
}

/// Presses the flip toggle for `param`: the pending flip is `{ param,
/// from: live value, to: its negation }` with a fresh press identity, and
/// the jump is adopted at elapsed 0 without advancing, so the press's own
/// turn still draws the old value. Returns the negated value for the caller
/// to write through its param at once (flushed, as the hook's `setValue`
/// with `{ flush: true }` does).
pub fn press_flip(
    state: &mut FlipState,
    live: Transform,
    param: TransformParam,
    can_animate: bool,
) -> f64 {
    let from = live.get(param);
    let to = -from;
    state.presses += 1;
    let flip = Flip {
        param,
        from,
        to,
        seq: state.presses,
    };
    state.pending = Some(flip);
    state.shown.adopt(Some(flip), live, can_animate);
    // The press's own turn draws the adopted jump, unstepped (the hook's
    // render runs before the next animation frame, not in one).
    state.frame_at = None;
    to
}

/// The transform one frame draws: the live values with the flip's spring
/// blended in ([`press_flip`]'s render phase, then [`flip_step`]'s
/// advances). Reads without stepping, so the draw calls this, never
/// [`flip_step`].
#[must_use]
pub fn shown_transform(state: &FlipState, live: Transform) -> Transform {
    state.shown.shown(live)
}

/// One flip frame on the app's clock: while a flip runs, advance it. The
/// press's own turn holds the adopted jump (elapsed 0); the loop steps from
/// the turn after, the first step carrying the hook's first-frame gap.
/// Returns the transform the frame draws.
pub fn flip_step(
    state: &mut FlipState,
    live: Transform,
    now: crate::app_loop::Time,
    can_animate: bool,
) -> Transform {
    // A press's own turn draws the adopted jump without stepping, keeping
    // the running clock for the next frame on a mid-turn re-press (as
    // `plot.rs`'s `plot_hold` does for a mid-switch re-pick).
    let Some(last) = state.frame_at else {
        state.frame_at = Some(now);
        return state.shown.shown(live);
    };
    let dt = (now - last) as f64 / crate::app_loop::MS as f64;
    let shown = state.shown.step(state.pending, live, dt, can_animate);
    state.frame_at = Some(now);
    if !state.shown.is_animating() {
        state.frame_at = None;
    }
    shown
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

    /// The live transform behind the vitest rig: only `a` off default.
    fn live_a(a: f64) -> Transform {
        Transform {
            a,
            b: 1.0,
            h: 0.0,
            k: 0.0,
        }
    }

    /// Animation frames on the app's clock, 1000/60 ms apart as on a 60 Hz
    /// screen (the `plot.rs` `Frames` precedent): time accumulates from the
    /// press turn, and each step diffs it, so `elapsed` sums as the hook's
    /// `now - last` does. The press turn itself holds (see [`flip_step`]);
    /// every call here is an advancing frame after it.
    struct Frames {
        time: crate::app_loop::Time,
    }

    impl Frames {
        fn new() -> Self {
            Self { time: 0 }
        }

        fn frame(&mut self, state: &mut FlipState, live: Transform) -> Transform {
            self.time += (FIRST_FRAME_MS * crate::app_loop::MS as f64) as crate::app_loop::Time;
            flip_step(state, live, self.time, true)
        }
    }

    /// `useShownTransform.test.tsx`: flipping a = 2 turns the curve over
    /// through flat while the box shows −2 at once — ported against the
    /// state model (the box is the value `press_flip` returns; the plane is
    /// what the steps show).
    #[test]
    fn flipping_a_2_turns_the_curve_over_through_flat_while_the_box_shows_minus_2_at_once() {
        let mut state = FlipState::new();
        // The plane shows a = 2.00 at rest.
        assert_eq!(shown_transform(&state, live_a(2.0)).a, 2.0);

        // The press: the box takes −2.00, and the press's own turn still
        // shows 2.00 (the render-phase jump at elapsed 0).
        let boxed = press_flip(&mut state, live_a(2.0), TransformParam::A, true);
        assert_eq!(boxed, -2.0);
        assert_eq!(
            state.pending(),
            Some(Flip {
                param: TransformParam::A,
                from: 2.0,
                to: -2.0,
                seq: 1,
            })
        );
        assert_eq!(shown_transform(&state, live_a(-2.0)).a, 2.0);

        // Six frames in: the curve is on its way down through flat.
        // The press's own turn holds the adopted jump; each frame after
        // advances, as the hook's render runs before its first callback.
        let mut frames = Frames::new();
        flip_step(&mut state, live_a(-2.0), 0, true);
        let mut shown = live_a(2.0);
        for _ in 0..6 {
            shown = frames.frame(&mut state, live_a(-2.0));
        }
        assert_close_to(shown.a, 1.07);

        // Twelve frames in: through flat.
        for _ in 0..6 {
            shown = frames.frame(&mut state, live_a(-2.0));
        }
        assert_close_to(shown.a, -0.19);

        // 43 frames in (past the 702 ms settle): rung through, resting on
        // the new value.
        for _ in 0..31 {
            shown = frames.frame(&mut state, live_a(-2.0));
        }
        assert_close_to(shown.a, -1.96);

        // Past the second: the flip ends on the live transform whole, and
        // no frames are scheduled.
        for _ in 0..18 {
            shown = frames.frame(&mut state, live_a(-2.0));
        }
        assert_eq!(shown.a, -2.0);
        assert!(!state.is_animating());
    }

    /// `useShownTransform.test.tsx`: a value typed while the curve turns
    /// over shows at once.
    #[test]
    fn a_value_typed_while_the_curve_turns_over_shows_at_once() {
        let mut state = FlipState::new();
        press_flip(&mut state, live_a(2.0), TransformParam::A, true);
        let mut frames = Frames::new();
        flip_step(&mut state, live_a(-2.0), 0, true);
        for _ in 0..6 {
            frames.frame(&mut state, live_a(-2.0));
        }
        assert_close_to(shown_transform(&state, live_a(-2.0)).a, 1.07);

        // Typing −1: the live value differs from the jump's target, so it
        // shows whole at once, as the TypeScript's `===` check does.
        assert_eq!(shown_transform(&state, live_a(-1.0)).a, -1.0);
    }

    /// `useShownTransform.test.tsx`: with reduced motion, flipping a = 2
    /// shows a = −2 at once — ported against the state model (no jump, no
    /// frames to run).
    #[test]
    fn with_reduced_motion_flipping_a_2_shows_minus_2_at_once() {
        let mut state = FlipState::new();
        let boxed = press_flip(&mut state, live_a(2.0), TransformParam::A, false);
        assert_eq!(boxed, -2.0);
        assert_eq!(shown_transform(&state, live_a(-2.0)).a, -2.0);
        assert!(!state.is_animating());
    }

    /// Every press mints a fresh identity, so a second press restarts the
    /// spring even with equal values (the hook's `flip !== seenFlip`).
    #[test]
    fn every_press_mints_a_fresh_identity() {
        let mut state = FlipState::new();
        press_flip(&mut state, live_a(2.0), TransformParam::A, true);
        press_flip(&mut state, live_a(-2.0), TransformParam::B, true);
        assert_eq!(
            state.pending(),
            Some(Flip {
                param: TransformParam::B,
                from: 1.0,
                to: -1.0,
                seq: 2,
            })
        );
    }

    /// A re-press mid-turn restarts from the displayed position: the new
    /// jump blends from what the plane shows, so the curve never jumps.
    #[test]
    fn a_re_press_mid_turn_restarts_from_the_displayed_position() {
        let mut state = FlipState::new();
        press_flip(&mut state, live_a(2.0), TransformParam::A, true);
        let mut frames = Frames::new();
        flip_step(&mut state, live_a(-2.0), 0, true);
        for _ in 0..6 {
            frames.frame(&mut state, live_a(-2.0));
        }
        // The box holds −2.00, so the re-press flips back toward +2.00 —
        // from the 1.07 the plane displays.
        let boxed = press_flip(&mut state, live_a(-2.0), TransformParam::A, true);
        assert_eq!(boxed, 2.0);
        assert_close_to(shown_transform(&state, live_a(2.0)).a, 1.07);
    }
}
