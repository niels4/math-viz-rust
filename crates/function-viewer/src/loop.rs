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
pub enum Timer {}

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
    let mut drawn = false;
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
pub fn timer_fired(_app: &mut App, timer: Timer, _now: Time) {
    match timer {}
}

/// Sets awake/pace and rearms the wake timer for the next deadline.
pub fn schedule(app: &mut App, _now: Time) {
    app.r#loop.awake = app.dirty;
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
