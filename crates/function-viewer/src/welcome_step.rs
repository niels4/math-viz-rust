//! The welcome loop playing on the hero plane: the loop state and its frame
//! step.
//!
//! The tour itself (`loop_at`, `WelcomeClock`, `WELCOME_STILL`) is
//! h10-p-loop's [`welcome_loop`]; the hero's own switch animation is
//! [`plot`]'s `Shown` (the `usePlotCurves` hook); the paint reads the loop
//! state's values through `transform_curves` in [`welcome`]'s hero paint.
//! [`loop`]'s `iterate` steps this once per frame on the welcome route, and
//! `harness` writes its `loop` views line.
//!
//! [`loop`]: crate::app_loop
//! [`welcome`]: crate::welcome
//! [`welcome_loop`]: crate::welcome_loop

use crate::app::App;
use crate::app_loop::{MS, Time};
use crate::functions::FunctionSlug;
use crate::welcome::Route;
use crate::welcome_loop::{LoopState, WELCOME_STILL, WelcomeClock};

/// The welcome page's loop state: the tour clock, the hero's own switch
/// animation, the last tour state, and the stamp plus the hold that mirror
/// [`plot`]'s frame pattern (`plot_frame_at` + `plot_hold` in
/// `selector.rs`). The clock keeps its own last frame, so no second stamp
/// is needed for it.
///
/// [`plot`]: crate::plot
#[derive(Clone, Copy, Debug)]
pub struct WelcomeState {
    pub clock: WelcomeClock,
    pub shown: crate::plot::Shown,
    pub current: LoopState,
    /// The route-enter turn draws the mount unstepped (see [`welcome_step`]).
    pub entered: bool,
    step_at: Option<f64>,
    hold: bool,
}

impl WelcomeState {
    /// Fresh at startup: the clock at 0, x² shown whole (the tour's start).
    /// The first frame steps it: there is no entering turn at startup.
    #[must_use]
    pub fn fresh() -> Self {
        Self {
            clock: WelcomeClock::new(),
            shown: crate::plot::Shown::new(crate::functions::base(FunctionSlug::X2)),
            current: crate::welcome_loop::loop_at(0.0),
            entered: false,
            step_at: None,
            hold: false,
        }
    }

    /// Fresh when the welcome route is entered: as [`fresh`], but the
    /// entering turn draws the mount unstepped; under reduced motion the
    /// still, shown whole.
    ///
    /// [`fresh`]: Self::fresh
    #[must_use]
    pub fn fresh_entered(reduced_motion: bool) -> Self {
        if reduced_motion {
            Self {
                clock: WelcomeClock::new(),
                shown: crate::plot::Shown::new(crate::functions::base(WELCOME_STILL.func)),
                current: WELCOME_STILL,
                entered: true,
                step_at: None,
                hold: false,
            }
        } else {
            Self {
                entered: true,
                ..Self::fresh()
            }
        }
    }
}

/// Whether the welcome route keeps the loop awake: only while animation is
/// on (under reduced motion the page rests on the still and requests no
/// frames). The entering turn draws the mount unstepped but stays awake
/// through this arm, so the next frame starts the loop. `loop.rs`'s
/// `schedule` reads it.
#[must_use]
pub fn animating(app: &App) -> bool {
    Route::of(&app.url) == Route::Welcome && WelcomeClock::is_animating(!app.reduced_motion)
}

/// One frame on the welcome route: the clock advances every frame, the
/// hero's switch follows the tour's function, and the switch steps toward
/// whole. Returns whether this frame animated: the caller dirties while it
/// does, so the loop stays awake at the display's pace; under reduced
/// motion (or off the route) it sleeps.
pub fn welcome_step(app: &mut App, now: Time) -> bool {
    if Route::of(&app.url) != Route::Welcome {
        return false;
    }
    let can_animate = !app.reduced_motion;
    let state = &mut app.welcome;
    if !can_animate {
        // The still, adopted whole when it changed (reduced motion shows
        // the new curve at once, as `switch_to` without animation does).
        if state.current != WELCOME_STILL {
            state.current = WELCOME_STILL;
            state
                .shown
                .switch_to(crate::functions::base(WELCOME_STILL.func), false);
        }
        state.entered = false;
        state.hold = false;
        state.step_at = None;
        return false;
    }
    // The entering turn draws the mount (`loopAt(0)` = rest, as the
    // reference's mount render does); the loop starts on the next frame, so
    // the first step carries `FIRST_FRAME_MS` exactly like the hook's first
    // rAF callback.
    if state.entered {
        state.entered = false;
        return true;
    }
    let now_ms = now as f64 / MS as f64;
    state.clock.step(now_ms);
    let next = state.clock.state(true);
    if next.func != state.current.func {
        // The render-phase switch, as `selector.rs`'s `begin_switch`: from
        // rest the (always `None` outside a switch) stamp path holds this
        // frame; mid-switch the running clock persists behind one held
        // frame (the hook's loop is keyed on `switching`, never restarted).
        if state.shown.is_switching() {
            state.hold = true;
        }
        state
            .shown
            .switch_to(crate::functions::base(next.func), true);
        state.current = next;
    }
    if !state.shown.is_switching() {
        return true;
    }
    if state.hold {
        state.hold = false;
        return true;
    }
    let Some(last) = state.step_at else {
        state.step_at = Some(now_ms);
        return true;
    };
    crate::plot::step(&mut state.shown, now_ms - last);
    state.step_at = Some(now_ms);
    if !state.shown.is_switching() {
        state.step_at = None;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::headless_app;

    /// A headless app on the welcome route (`#`, the empty route).
    fn welcome_app() -> App {
        let mut app = headless_app();
        app.url.sync_from_hash("#");
        assert_eq!(Route::of(&app.url), Route::Welcome);
        app
    }

    /// Script ms as the app's clock.
    fn at(ms: i64) -> Time {
        ms * MS
    }

    /// One grid frame, 16 ms on from `now`.
    fn frame(app: &mut App, now: &mut Time) -> bool {
        *now += 16 * MS;
        welcome_step(app, *now)
    }

    #[test]
    fn entering_draws_rest_unstepped_then_steps_each_frame() {
        let mut app = welcome_app();
        app.welcome = WelcomeState::fresh_entered(false);
        // The entering turn draws the mount with no step.
        assert!(welcome_step(&mut app, at(0)));
        assert_eq!(app.welcome.clock.elapsed(), 0.0);
        assert_eq!(app.welcome.current.func, FunctionSlug::X2);
        // Each frame after steps once: FIRST_FRAME_MS, then the 16 ms grid.
        let mut now = 0;
        for _ in 0..70 {
            assert!(frame(&mut app, &mut now));
        }
        let elapsed = crate::plot::FIRST_FRAME_MS + 69.0 * 16.0;
        assert!((app.welcome.clock.elapsed() - elapsed).abs() < 1e-9);
        assert_eq!(app.welcome.current, crate::welcome_loop::loop_at(elapsed));
    }

    #[test]
    fn still_under_reduced_motion_shows_whole_and_sleeps() {
        // `useWelcomeLoop.test.ts`: the still, and no frames requested.
        let mut app = welcome_app();
        app.reduced_motion = true;
        app.welcome = WelcomeState::fresh_entered(true);
        assert!(!welcome_step(&mut app, at(16)));
        assert_eq!(app.welcome.current, WELCOME_STILL);
        assert_eq!(
            crate::plot::curves(&app.welcome.shown),
            vec![crate::plot::PlotCurve {
                func: crate::functions::base(FunctionSlug::SinX),
                alpha: 1.0,
                draw_to: 1.0,
            }]
        );
        assert!(!app.welcome.shown.is_switching());
        assert_eq!(app.welcome.clock.elapsed(), 0.0);
        assert!(!welcome_step(&mut app, at(5000)));
        assert_eq!(app.welcome.clock.elapsed(), 0.0);
        assert!(!animating(&app));
    }

    #[test]
    fn a_switch_mid_loop_fades_the_old_curve_whole() {
        // To the sin x switch (loop frame 106 = 4416.67 ms), then the
        // hero's switch: the switch turn draws the reset curves unstepped,
        // and after the stamp-skip plus 5 steps on the grid x² is about
        // half faded while sin x has barely started.
        let mut app = welcome_app();
        app.welcome = WelcomeState::fresh_entered(false);
        assert!(welcome_step(&mut app, at(0)));
        let mut now = 0;
        for _ in 0..276 {
            assert!(frame(&mut app, &mut now));
        }
        assert_eq!(app.welcome.current.func, FunctionSlug::X2);
        assert!(frame(&mut app, &mut now));
        assert_eq!(app.welcome.current.func, FunctionSlug::SinX);
        assert_eq!(
            crate::plot::curves(&app.welcome.shown),
            vec![
                crate::plot::PlotCurve {
                    func: crate::functions::base(FunctionSlug::X2),
                    alpha: 1.0,
                    draw_to: 1.0,
                },
                crate::plot::PlotCurve {
                    func: crate::functions::base(FunctionSlug::SinX),
                    alpha: 1.0,
                    draw_to: 0.0,
                },
            ]
        );
        for _ in 0..5 {
            assert!(frame(&mut app, &mut now));
        }
        let showing = crate::plot::curves(&app.welcome.shown);
        assert_eq!(showing.len(), 2);
        assert!((showing[0].alpha - 0.50).abs() < 0.005);
        assert!(showing[1].draw_to > 0.0);
        assert!(showing[1].draw_to < 0.01);
    }

    #[test]
    fn switch_from_rest_carries_the_first_frame_gap() {
        // On an odd (24 ms) grid the frame after the switch turn still
        // steps FIRST_FRAME_MS, not the measured gap.
        let mut app = welcome_app();
        app.welcome = WelcomeState::fresh_entered(false);
        assert!(welcome_step(&mut app, at(0)));
        let mut now = 0;
        for _ in 0..300 {
            now += 24 * MS;
            assert!(welcome_step(&mut app, now));
            if app.welcome.current.func == FunctionSlug::SinX {
                break;
            }
        }
        assert_eq!(app.welcome.current.func, FunctionSlug::SinX);
        assert_eq!(app.welcome.shown.elapsed, 0.0);
        now += 24 * MS;
        assert!(welcome_step(&mut app, now));
        assert!((app.welcome.shown.elapsed - crate::plot::FIRST_FRAME_MS).abs() < 1e-9);
    }

    #[test]
    fn off_the_welcome_route_never_steps() {
        let mut app = headless_app();
        app.url.sync_from_hash("#function-viewer");
        app.welcome = WelcomeState::fresh_entered(false);
        assert!(!welcome_step(&mut app, at(16)));
        assert_eq!(app.welcome.clock.elapsed(), 0.0);
        assert!(!animating(&app));
    }

    #[test]
    fn animating_needs_the_route_and_motion() {
        let mut app = welcome_app();
        // Awake from the entering turn on (the mount draws unstepped but
        // the loop must start on the next frame).
        app.welcome = WelcomeState::fresh_entered(false);
        assert!(animating(&app));
        app.reduced_motion = true;
        assert!(!animating(&app));
    }
}
