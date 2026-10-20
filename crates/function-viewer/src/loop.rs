//! Frame pacing, timers and SDL user events: the clock every motion reads.
//!
//! Time is nanoseconds (`Time`); `MS` is one millisecond. While a script runs,
//! the harness's clock replaces SDL's ticks.

use std::sync::atomic::AtomicU32;

use enum_map::{Enum, EnumMap};

use crate::app::App;
#[cfg(not(target_os = "emscripten"))]
use crate::platform_native::{
    platform_pace, platform_push_user, platform_ticks_ns, platform_timer_arm, platform_timer_remove,
};
#[cfg(target_os = "emscripten")]
use crate::platform_web::{
    platform_pace, platform_push_user, platform_ticks_ns, platform_timer_arm, platform_timer_remove,
};

/// Nanoseconds on the app's clock.
pub type Time = i64;

/// One millisecond in `Time`.
pub const MS: Time = 1_000_000;

/// The frame pacer.
pub struct Loop {
    /// The harness's clock, while a script runs.
    pub script_time: Option<Time>,
    pub pace: Pace,
    /// Frames keep coming: something moves.
    pub awake: bool,
    /// The deadline the wake timer is set for; 0 when none is set.
    pub wake_at: Time,
    /// The wake timer's id; 0 when none is armed. (The loop milestone settles
    /// how this maps onto the SDL timer.)
    pub wake_timer: u32,
    /// When the wake timer was armed, its interval counted from then.
    pub armed_at: Time,
    /// Turns of the loop, drawn or not.
    pub iterations: i64,
    /// Frames drawn.
    pub frames: i64,
    /// Keep each turn in `turns` (the live driver writes them out).
    pub recording: bool,
    pub turns: Vec<Turn>,
}

/// One turn of the loop.
pub struct Turn {
    pub at: Time,
    pub drawn: bool,
}

/// How frames are paced.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pace {
    /// The display: vsync on, SDL's callback rate at 0, so each swap waits
    /// for the display.
    Display,
    /// SDL's callback rate at 60 Hz and no vsync. A login with no active
    /// display never sends the vsync a swap waits for, so the app paces
    /// itself there.
    Fixed60,
}

/// The deadlines the app arms. The shell arms none; the page milestones append
/// their variants here (append-only, like `App`'s fields).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum Timer {
    /// The scrub box's rest tip: the pointer's 400 ms rest is over.
    ScrubRest,
    /// The return-to-centre tooltip: the pointer's 400 ms hover is over
    /// (h6-handles).
    OverlayTip,
    /// The moved badge's linger: a frame is due so the badge can clear
    /// (h8-points-row).
    MovedLinger,
}

/// One deadline per timer.
pub type Timers = EnumMap<Timer, Time>;

/// The SDL user-event type the app registered; 0 before registration.
pub static USER_EVENT_TYPE: AtomicU32 = AtomicU32::new(0);

/// What a user event carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum UserCode {
    /// A deadline came: data1 is the wake timer's id, data2 the interval SDL
    /// armed it for, in microseconds.
    Wake,
    /// The web's prefers-reduced-motion changed: data1 is 0 or 1.
    ReducedMotion,
    /// A step of the native live driver: data1 is its index.
    Live,
    /// A script's model line: data1 its model, data2 non-nil when it is to be
    /// freed.
    Model,
    /// Another tab stored the prefs: data1 the packed prefs word.
    Prefs,
    /// The page's hash changed under the app (the web `hashchange`
    /// subscription): re-read on the main thread. Appended by h2-p-url.
    Hash,
}

/// Builds an SDL user event and pushes it. Callable from any thread (the SDL
/// timer and the web exports call it); data words are raw by SDL's design.
///
/// The push itself is one `SDL_PushEvent` call in the platform file: the safe
/// `sdl3` crate's push needs an event-subsystem handle, which a contextless
/// timer callback has no way to reach.
pub fn push_user_event(
    code: UserCode,
    data1: *mut std::ffi::c_void,
    data2: *mut std::ffi::c_void,
) -> bool {
    platform_push_user(code as i32, data1, data2)
}

/// The clock: the script's time while a script runs, else SDL's ticks.
pub fn clock_now(app: &mut App) -> Time {
    if let Some(t) = app.r#loop.script_time {
        return t;
    }
    platform_ticks_ns() as Time
}

/// Arms `timer` `after_ms` from now.
pub fn timer_start(app: &mut App, timer: Timer, after_ms: f64) {
    let now = clock_now(app);
    let from = if app.input.time != 0 {
        app.input.time
    } else {
        now
    };
    let mut due = from + (after_ms * MS as f64) as Time;
    if due <= now {
        due = now + (after_ms * MS as f64) as Time;
    }
    app.timers[timer] = due;
    if crate::trace() {
        println!(
            "timer {timer:?}: set for {after_ms:.0} ms, {:.3} ms from now",
            (due - now) as f64 / MS as f64
        );
    }
}

/// Clears `timer`'s deadline.
pub fn timer_stop(app: &mut App, timer: Timer) {
    app.timers[timer] = 0;
}

/// One turn: fires due timers, draws when dirty, and schedules the next
/// deadline.
pub fn iterate(app: &mut App) -> bool {
    let now = clock_now(app);
    app.r#loop.iterations += 1;
    // The timers in order, re-read live: firing one may re-arm a later one,
    // which this same turn then sees.
    let order: Vec<Timer> = app.timers.iter().map(|(timer, _)| timer).collect();
    for timer in order {
        let due = app.timers[timer];
        if due != 0 && due <= now {
            let late = (now - due) as f64 / MS as f64;
            app.timers[timer] = 0;
            if crate::trace() {
                println!("timer {timer:?}: fired {late:.3} ms after its deadline");
            }
            // What a timer sets going counts from its deadline (timer_start).
            app.input.time = due;
            timer_fired(app, timer, now);
            app.input.time = 0;
        }
    }
    // The plane's camera step (h2-input): the camera's frame, the params'
    // debounce and external sync, the web hash mirror. Before the draw, so
    // the frame draws what the camera moved to.
    crate::plane_input::frame_step(app, now);
    // The picker's plot step (h3): the switch's frame, then an outside edit
    // flowing in. Before the draw, so the frame draws what the switch drew
    // to.
    crate::selector::plot_step(app, now);
    crate::selector::external_func_synced(app);
    // The transform params' debounce and external sync (h4-rows): a pending
    // write fires, an outside edit flows into the live values. Before the
    // draw, so the frame draws what the values moved to. A fired write
    // repaints: the readout reads the URL, so without this it would keep
    // showing the pre-debounce values on a quiet page.
    if crate::transform_section::params_step(app, (now / MS) as u64)
        | crate::point_slider::p_step(app, (now / MS) as u64)
    {
        app.dirty = true;
        #[cfg(target_os = "emscripten")]
        crate::platform_web::platform_write_hash(&app.url.full_hash());
    }
    // The flip's step (h6-curve, through h7-p-state's jump list): the
    // toggle's spring turns the curve through flat over a second, stepped
    // after the params sync so it reads the fresh live values, before the
    // draw so the frame draws what the flip turned to. The press's own turn
    // draws the adopted jump unstepped; a running flip repaints, as a switch
    // does.
    {
        let live = crate::flip::live_transform(app);
        crate::flip::flip_step(
            &mut app.flip,
            &app.state.jumps,
            live,
            now,
            !app.reduced_motion,
        );
        if app.flip.is_animating() {
            app.dirty = true;
        }
    }
    let mut drawn = false;
    // The theme fade redraws until done: no timer arms its frames, so the
    // turn does while one is in flight (its last frame clears the active).
    if app.transition.active.is_some() {
        app.dirty = true;
    }
    if app.dirty {
        app.dirty = false;
        crate::draw::draw_frame(app, now);
        app.r#loop.frames += 1;
        drawn = true;
    }
    schedule(app, now);
    trace_iteration(app, now, drawn);
    drawn
}

/// Runs one fired timer's action. The shell arms no timers, so no arm fires.
pub fn timer_fired(app: &mut App, timer: Timer, _now: Time) {
    match timer {
        Timer::ScrubRest => crate::scrub_value::rest_fired(app),
        Timer::OverlayTip => crate::overlay::tip_fired(app),
        Timer::MovedLinger => crate::points_section::moved_linger_fired(app),
    }
}

/// Sets awake/pace and rearms the wake timer for the next deadline.
pub fn schedule(app: &mut App, _now: Time) {
    // A fade in flight keeps the turns coming, as its frames do: the last
    // one clears the active and lets the app sleep again. A camera in flight
    // does the same (h2-input): each turn steps it, and the settle stops the
    // loop — without this a glide stalls after its first step, since the draw
    // consumes the dirty its step set. A plot switch does the same (h3). A
    // debounced page-param write does the same (h5-box): each turn steps it,
    // and the write stops the loop — without this a typed or nudged value
    // never reaches the URL on a quiet page, since nothing else steps the
    // params once the frame settles. A flip in flight does the same
    // (h6-curve): each turn steps it, and the settle stops the loop —
    // without this the curve stalls after its first step, since the draw
    // consumes the dirty its step set.
    app.r#loop.awake = app.dirty
        || app.transition.active.is_some()
        || app.camera.animating()
        || app.shown.is_switching()
        || app.flip.is_animating()
        || crate::transform_section::params_pending(app)
        || app.p_param.has_pending();
    let awake = app.r#loop.awake;
    platform_pace(app, awake);
    let mut next: Time = 0;
    for due in app.timers.values() {
        if *due != 0 && (next == 0 || *due < next) {
            next = *due;
        }
    }
    if next == app.r#loop.wake_at {
        return;
    }
    if app.r#loop.wake_timer != 0 {
        platform_timer_remove(app.r#loop.wake_timer);
        app.r#loop.wake_timer = 0;
    }
    app.r#loop.wake_at = next;
    // Under a script the harness steps to the deadline itself. The timer is armed for the time left
    // now, not from the turn's start: the frame drawn before this point (up to 16 ms under load)
    // would otherwise make every wake that much late.
    if next != 0 && app.r#loop.script_time.is_none() {
        app.r#loop.armed_at = clock_now(app);
        let interval = (next - app.r#loop.armed_at).max(0) as u64;
        app.r#loop.wake_timer = platform_timer_arm(interval);
    }
}

/// Appends this turn when recording.
pub fn trace_iteration(app: &mut App, now: Time, drawn: bool) {
    if app.r#loop.recording {
        app.r#loop.turns.push(Turn { at: now, drawn });
    }
}

/// The SDL wake-timer callback: posts a `Wake` user event.
///
/// SDL's nanosecond timer passes the timer id and the armed interval in
/// nanoseconds; the callback is `SDL_NSTimerCallback`, so the interval is
/// `u64` (the surface's `u32` could not name the C type). The event carries
/// the interval in microseconds, rounded up, so the wake's log never puts
/// the timer before the deadline it was armed for.
///
/// `pub(crate)`: the platform files arm it (they own the raw timer call).
pub(crate) extern "C" fn wake(
    _userdata: *mut std::ffi::c_void,
    timer: sdl3::sys::timer::SDL_TimerID,
    interval: u64,
) -> u64 {
    let _ = push_user_event(
        UserCode::Wake,
        timer.0 as usize as *mut std::ffi::c_void,
        interval.div_ceil(1000) as usize as *mut std::ffi::c_void,
    );
    0
}

/// Clears the spent wake timer when it matches.
pub fn woken(app: &mut App, timer: u32, posted: Time, interval: Time) {
    if timer == app.r#loop.wake_timer {
        if crate::trace() {
            println!(
                "wake: posted {:.3} ms after its deadline, its timer armed to fire {:.3} ms after it",
                (posted - app.r#loop.wake_at) as f64 / MS as f64,
                (app.r#loop.armed_at + interval - app.r#loop.wake_at) as f64 / MS as f64
            );
        }
        platform_timer_remove(timer);
        app.r#loop.wake_timer = 0;
        app.r#loop.wake_at = 0;
    }
}
