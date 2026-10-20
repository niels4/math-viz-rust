//! The live driver: plays a script through SDL's own loop on the real clock.
//!
//! Native only. The script becomes timed steps posted by an SDL timer; the
//! main thread applies each step, and finishing writes live.json. Posted and
//! due timestamps are preserved so replays line up.

use std::sync::atomic::{AtomicPtr, Ordering};

use sdl3_main::AppResult;

use crate::app::App;
use crate::app_loop::{MS, Pace, Time, UserCode, push_user_event};
use crate::input::Input;

/// The live run, while it stands: the boxed `Live` of `live_start`,
/// reachable from the SDL timer thread. A raw pointer, never a borrow: the
/// main thread holds the only `&mut App` (in the transient `Harness`), while
/// the timer thread only advances `next`/`posted` and posts SDL events.
/// `live_finish` takes it back with `Box::from_raw`.
static LIVE_RUN: AtomicPtr<std::ffi::c_void> = AtomicPtr::new(std::ptr::null_mut());

/// One live run.
pub struct Live {
    /// The app. A raw pointer, not the `&mut` the surface sketch had: the
    /// run is reachable from the SDL timer thread while the main thread
    /// holds the borrow (in `Harness.app`), which safe Rust cannot name.
    /// The main thread's step handling takes `&mut App` as a parameter and
    /// never dereferences this; the timer thread reads `window_id` instead.
    pub app: *mut App,
    pub out: String,
    pub steps: Vec<LiveStep>,
    /// When each step was posted (SDL ticks).
    pub posted: Vec<Time>,
    /// The next step to post: SDL's timer thread's alone once the run
    /// starts.
    pub next: i64,
    /// When the timeline began (SDL ticks).
    pub start: Time,
    pub timer: u32,
    pub readback: Vec<u8>,
    /// The window the timer thread posts into, taken at start: reading it
    /// through the raw `app` from the timer thread would race the main
    /// thread's SDL calls.
    pub window_id: u32,
    /// The window's pointer, taken at start beside the id: the text step
    /// asks SDL whether text input is on, which takes the window (the spec
    /// reads it off the app on its timer thread). Never dereferenced here;
    /// the window outlives the run.
    pub window: *mut sdl3::sys::video::SDL_Window,
}

// The timer thread only reads `steps`/`start`, advances `next`/`posted` and
// posts SDL events (which are thread-safe); the main thread owns the rest.
// This matches the spec, where `live_run` is a plain global with the same
// sharing.
unsafe impl Send for Live {}
unsafe impl Sync for Live {}

/// One step: at how many ms after the start, and what.
#[derive(Debug)]
pub struct LiveStep {
    /// Ms after the start.
    pub at: i64,
    pub what: LiveWhat,
}

/// What a step does.
#[derive(Debug)]
pub enum LiveWhat {
    /// Posted as the SDL event that carries it.
    Input(Input),
    /// Typed text: posted only while SDL has text input on, as SDL sends text.
    Text(LiveText),
    Shot(LiveShot),
    Resize(LiveResize),
    Scheme(LiveScheme),
    End(LiveEnd),
}

/// Typed text: posted only while text input is on.
#[derive(Debug)]
pub struct LiveText {
    pub text: std::ffi::CString,
}

/// A screenshot step.
#[derive(Debug)]
pub struct LiveShot {
    pub name: String,
}

/// A resize step.
#[derive(Debug)]
pub struct LiveResize {
    pub size: [i64; 2],
}

/// A scheme step.
#[derive(Debug)]
pub struct LiveScheme {
    pub dark: bool,
}

/// The run's end.
#[derive(Debug)]
pub struct LiveEnd {}

/// Plays the script into a live timeline, arms the SDL timer, and enables
/// recording.
pub fn live_start(app: &mut App, run: &mv_script::Script, out: &str) -> bool {
    if !out.is_empty() && !crate::platform_native::platform_make_dir(out) {
        return false;
    }
    if app.window().pixel_density() != run.start.density {
        println!(
            "the window's pixel density is {} and the script's {}; shots come out at {}",
            app.window().pixel_density(),
            run.start.density,
            app.window().pixel_density()
        );
    }
    let window_id = app.window().id();
    // The live window pointer, taken while the main thread owns the window;
    // the timer thread only hands it back to SDL's own text-input query,
    // which reads the window's state the same way the spec's timer thread
    // does, and the window outlives the run.
    let window = app.window().raw();
    let mut l = Box::new(Live {
        // SAFETY: a raw cast takes no borrow; the main thread's only `&mut`
        // stays in the `Harness` below, and nothing dereferences this while
        // the run stands (the timer path uses `window_id`, the main thread
        // takes `&mut App` as a parameter).
        app: app as *mut App,
        out: out.to_string(),
        steps: Vec::new(),
        posted: Vec::new(),
        next: 0,
        start: 0,
        timer: 0,
        readback: Vec::new(),
        window_id,
        window,
    });
    let total = {
        let mut h = crate::harness::Harness {
            app,
            out: String::new(),
            window: false,
            live: Some(&mut *l),
            fb: None,
            density: run.start.density,
            time: 0,
            pending: Vec::new(),
            readback: Vec::new(),
            pointer: [0.0, 0.0],
            mods: sdl3::keyboard::Mod::empty(),
            press: crate::harness::HarnessPress {
                time: -crate::harness::DOUBLE_CLICK_MS,
                pos: [0.0, 0.0],
                clicks: 0,
            },
            videos: 0,
            views: Vec::new(),
            moving: Vec::new(),
            url: String::new(),
        };
        if !crate::harness::play(&mut h, run) {
            return false;
        }
        h.time
    };
    live_add(&mut l, total, LiveWhat::End(LiveEnd {}));
    l.posted = vec![0; l.steps.len()];
    // SAFETY: `SDL_GetTicksNS` reads the global SDL clock; no pointers.
    l.start = unsafe { sdl3::sys::timer::SDL_GetTicksNS() } as Time;
    // SAFETY: the callback is a plain `extern "C"` fn; `userdata` (the boxed
    // run) outlives the timer, which `live_finish` removes before freeing it.
    l.timer = unsafe {
        sdl3::sys::timer::SDL_AddTimerNS(
            (l.steps[0].at as u64) * (MS as u64),
            Some(live_tick),
            (&mut *l as *mut Live).cast::<std::ffi::c_void>(),
        )
        .0
    };
    println!(
        "live: {} steps over {} ms through SDL's loop",
        l.steps.len(),
        l.steps.last().map_or(0, |s| s.at)
    );
    LIVE_RUN.store(
        Box::into_raw(l).cast::<std::ffi::c_void>(),
        Ordering::SeqCst,
    );
    app.r#loop.recording = true;
    true
}

/// A step at `at` ms. Strings are copied: the script's frame names and typed
/// characters are made on the temp allocator.
pub fn live_add(l: &mut Live, at: i64, what: LiveWhat) {
    let mut step = LiveStep { at, what };
    match &mut step.what {
        LiveWhat::Text(t) => {
            // The C string truncates at the first nul, as the spec's
            // `clone_to_cstring` read does.
            let bytes = t.text.to_bytes();
            let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
            t.text = std::ffi::CString::new(&bytes[..end]).unwrap_or_default();
        }
        LiveWhat::Input(Input::Text(t)) => {
            let bytes = t.text.as_bytes();
            let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
            step.what = LiveWhat::Text(LiveText {
                text: std::ffi::CString::new(&bytes[..end]).unwrap_or_default(),
            });
        }
        LiveWhat::Input(_)
        | LiveWhat::Shot(_)
        | LiveWhat::Resize(_)
        | LiveWhat::Scheme(_)
        | LiveWhat::End(_) => {}
    }
    l.steps.push(step);
}

/// SDL's timer thread: posts the steps that are due, and asks to be called
/// again at the next. Nanoseconds in and out, as `SDL_AddTimerNS` counts.
unsafe extern "C" fn live_tick(
    userdata: *mut std::ffi::c_void,
    _timer: sdl3::sys::timer::SDL_TimerID,
    _interval: u64,
) -> u64 {
    // SAFETY: `userdata` is the boxed run `live_start` registered; it lives
    // until `live_finish` removes this timer and frees it.
    let l = unsafe { &mut *(userdata as *mut Live) };
    // SAFETY: `SDL_GetTicksNS` reads the global SDL clock; no pointers.
    let now = unsafe { sdl3::sys::timer::SDL_GetTicksNS() } as Time;
    while l.next < l.steps.len() as i64 {
        let due = l.start + l.steps[l.next as usize].at * MS;
        if due > now {
            return (due - now) as u64;
        }
        l.posted[l.next as usize] = now;
        post(l, l.next);
        l.next += 1;
    }
    0
}

/// Pushes the SDL event, user event or text for one step's kind. Called from
/// the timer thread: only `SDL_PushEvent` (which is thread-safe) and the
/// step data are touched.
fn post(l: &Live, index: i64) {
    use sdl3::sys::events::*;
    use sdl3::sys::keycode::SDL_Keycode;

    let id = l.window_id;
    // SAFETY: every event below is zero-initialised and only its own union
    // member is written; `SDL_PushEvent` copies the event into SDL's queue
    // (it is thread-safe), except `text`, whose `CString` lives in `steps`
    // for the whole run. `SDL_WindowID` is a plain `u32` newtype.
    unsafe {
        match &l.steps[index as usize].what {
            LiveWhat::Input(input) => match input {
                Input::Pointer(e) => {
                    let mut ev: SDL_Event = std::mem::zeroed();
                    if e.left {
                        ev.window = SDL_WindowEvent {
                            r#type: SDL_EventType::WINDOW_MOUSE_LEAVE,
                            windowID: sdl3::sys::video::SDL_WindowID(id),
                            ..Default::default()
                        };
                    } else {
                        ev.motion = SDL_MouseMotionEvent {
                            r#type: SDL_EventType::MOUSE_MOTION,
                            windowID: sdl3::sys::video::SDL_WindowID(id),
                            x: e.pos[0],
                            y: e.pos[1],
                            ..Default::default()
                        };
                    }
                    SDL_PushEvent(&mut ev);
                }
                Input::Button(e) => {
                    let mut ev: SDL_Event = std::mem::zeroed();
                    ev.button = SDL_MouseButtonEvent {
                        r#type: if e.down {
                            SDL_EventType::MOUSE_BUTTON_DOWN
                        } else {
                            SDL_EventType::MOUSE_BUTTON_UP
                        },
                        windowID: sdl3::sys::video::SDL_WindowID(id),
                        button: e.button,
                        down: e.down,
                        clicks: e.clicks as u8,
                        x: e.pos[0],
                        y: e.pos[1],
                        ..Default::default()
                    };
                    SDL_PushEvent(&mut ev);
                }
                Input::Wheel(e) => {
                    let mut ev: SDL_Event = std::mem::zeroed();
                    ev.wheel = SDL_MouseWheelEvent {
                        r#type: SDL_EventType::MOUSE_WHEEL,
                        windowID: sdl3::sys::video::SDL_WindowID(id),
                        // The page's px as SDL's notches, which
                        // input_from_event turns back into px.
                        x: e.delta[0] / crate::platform_native::PLATFORM_WHEEL_PX,
                        y: -e.delta[1] / crate::platform_native::PLATFORM_WHEEL_PX,
                        direction: sdl3::sys::mouse::SDL_MouseWheelDirection::NORMAL,
                        mouse_x: e.pos[0],
                        mouse_y: e.pos[1],
                        ..Default::default()
                    };
                    SDL_PushEvent(&mut ev);
                }
                Input::Pinch(e) => {
                    let mut ev: SDL_Event = std::mem::zeroed();
                    ev.pinch = SDL_PinchFingerEvent {
                        r#type: SDL_EventType::PINCH_UPDATE,
                        scale: e.scale,
                        windowID: sdl3::sys::video::SDL_WindowID(id),
                        ..Default::default()
                    };
                    SDL_PushEvent(&mut ev);
                }
                Input::Key(e) => {
                    let code = SDL_Keycode::from(e.key);
                    let mut ev: SDL_Event = std::mem::zeroed();
                    ev.key = SDL_KeyboardEvent {
                        r#type: if e.down {
                            SDL_EventType::KEY_DOWN
                        } else {
                            SDL_EventType::KEY_UP
                        },
                        windowID: sdl3::sys::video::SDL_WindowID(id),
                        scancode: sdl3::sys::keyboard::SDL_GetScancodeFromKey(
                            code,
                            std::ptr::null_mut(),
                        ),
                        key: code,
                        r#mod: sdl3::sys::keycode::SDL_Keymod(e.mods.bits()),
                        down: e.down,
                        repeat: e.repeat,
                        ..Default::default()
                    };
                    SDL_PushEvent(&mut ev);
                }
                Input::Focus(e) => {
                    let mut ev: SDL_Event = std::mem::zeroed();
                    ev.window = SDL_WindowEvent {
                        r#type: if e.gained {
                            SDL_EventType::WINDOW_FOCUS_GAINED
                        } else {
                            SDL_EventType::WINDOW_FOCUS_LOST
                        },
                        windowID: sdl3::sys::video::SDL_WindowID(id),
                        ..Default::default()
                    };
                    SDL_PushEvent(&mut ev);
                }
                Input::ReducedMotion(e) => {
                    // No script can flip the system's setting: the change is
                    // posted as web/library.js posts prefers-reduced-motion's.
                    push_user_event(
                        UserCode::ReducedMotion,
                        std::ptr::null_mut(),
                        e.reduced as usize as *mut std::ffi::c_void,
                    );
                }
                Input::Model(_) => {
                    // Applied on the main thread when SDL's loop takes the
                    // step (live_step), which reads the model from the step.
                    push_user_event(
                        UserCode::Live,
                        index as *mut std::ffi::c_void,
                        std::ptr::null_mut(),
                    );
                }
                Input::Text(_) | Input::Window(_) | Input::Scheme(_) => {
                    // Made into Live_Text, Live_Resize and Live_Scheme.
                }
            },
            LiveWhat::Text(t) => {
                // The window pointer, not the app: the spec asks SDL about
                // its own window on its timer thread. Safe here: taken at
                // start while the main thread owns the window; SDL only
                // reads the window's state, and the window outlives the run.
                if sdl3::sys::keyboard::SDL_TextInputActive(l.window) {
                    let mut ev: SDL_Event = std::mem::zeroed();
                    ev.text = SDL_TextInputEvent {
                        r#type: SDL_EventType::TEXT_INPUT,
                        windowID: sdl3::sys::video::SDL_WindowID(id),
                        text: t.text.as_ptr().cast::<::core::ffi::c_char>(),
                        ..Default::default()
                    };
                    SDL_PushEvent(&mut ev);
                }
            }
            LiveWhat::Shot(_) | LiveWhat::Resize(_) | LiveWhat::Scheme(_) | LiveWhat::End(_) => {
                push_user_event(
                    UserCode::Live,
                    index as *mut std::ffi::c_void,
                    std::ptr::null_mut(),
                );
            }
        }
    }
}

/// A step for the main thread, from app_event.
pub fn live_step(app: &mut App, index: i64) -> AppResult {
    let ptr = LIVE_RUN.load(Ordering::SeqCst);
    if ptr.is_null() {
        return AppResult::Continue;
    }
    // SAFETY: the run stands from `live_start` to `live_finish`, both on the
    // main thread; the timer thread only advances `next`/`posted` and reads
    // `steps`, which this step's index has already passed.
    let l = unsafe { &mut *(ptr as *mut Live) };
    enum Action {
        Shot(String),
        Resize([i64; 2]),
        Scheme(bool),
        End,
        Model(mv_script::Model),
        Nothing,
    }
    let action = match &l.steps[index as usize].what {
        LiveWhat::Shot(s) => Action::Shot(s.name.clone()),
        LiveWhat::Resize(r) => Action::Resize(r.size),
        LiveWhat::Scheme(s) => Action::Scheme(s.dark),
        LiveWhat::End(_) => Action::End,
        LiveWhat::Input(input) => match input {
            Input::Model(m) => Action::Model(m.change.clone()),
            _ => Action::Nothing,
        },
        LiveWhat::Text(_) => Action::Nothing,
    };
    match action {
        Action::Shot(name) => {
            if l.out.is_empty() {
                eprintln!(
                    "[ERROR] --- [{}:{}] live: shot {name} needs --out",
                    file!(),
                    line!()
                );
            } else {
                mv_nvg::Framebuffer::bind_default();
                // SAFETY: `SDL_GetTicksNS` reads the global SDL clock.
                let now = unsafe { sdl3::sys::timer::SDL_GetTicksNS() } as Time;
                if crate::harness::write_shot(app, &mut l.readback, &l.out, &name, true) {
                    println!(
                        "live: shot {}/{name}.png at {:.1} ms",
                        l.out,
                        ms_since(l, now)
                    );
                }
            }
        }
        Action::Resize(size) => {
            let _ = app.window_mut().set_size(size[0] as u32, size[1] as u32);
        }
        Action::Scheme(dark) => {
            #[cfg(target_os = "macos")]
            crate::platform_darwin::darwin_set_appearance(dark);
            #[cfg(not(target_os = "macos"))]
            {
                let _ = dark;
                println!("live: the system's scheme can't be flipped from here on this platform");
            }
        }
        Action::End => {
            // SAFETY: `SDL_GetTicksNS` reads the global SDL clock.
            let now = unsafe { sdl3::sys::timer::SDL_GetTicksNS() } as Time;
            println!(
                "live: done at {:.1} ms, {} turns of the loop, {} frames",
                ms_since(l, now),
                app.r#loop.iterations,
                app.r#loop.frames
            );
            return AppResult::Success;
        }
        Action::Model(change) => {
            crate::input::handle_input(app, Input::Model(crate::input::ModelEvent { change }), 0);
        }
        Action::Nothing => {}
    }
    AppResult::Continue
}

/// From app_quit: <out>/live.json, then the run is gone.
///
/// Inherited from the Odin app (`live.odin`, which removes its timer and
/// frees the run the same way): the timer is removed before the box is
/// reclaimed, but a callback already in flight on SDL's timer thread may
/// still hold the run's pointer while it posts its events. SDL never calls
/// the callback again after removal, and quitting follows at once, so the
/// window a late post could miss is shutting down either way.
pub fn live_finish(app: &mut App) {
    let ptr = LIVE_RUN.swap(std::ptr::null_mut(), Ordering::SeqCst);
    if ptr.is_null() {
        return;
    }
    // SAFETY: the pointer came from `Box::into_raw` in `live_start` and the
    // timer is removed before the box is reclaimed below.
    let live: Box<Live> = unsafe { Box::from_raw(ptr as *mut Live) };
    unsafe {
        sdl3::sys::timer::SDL_RemoveTimer(sdl3::sys::timer::SDL_TimerID(live.timer));
    }
    if !live.out.is_empty() {
        write_live_json(app, &live);
    }
    // `live` drops here: steps, posted, readback and out are freed.
}

/// `(t - start)` in ms.
fn ms_since(l: &Live, t: Time) -> f64 {
    (t - l.start) as f64 / MS as f64
}

/// Writes pace, turns and steps to live.json.
fn write_live_json(app: &mut App, l: &Live) {
    let report = live_json(
        app.r#loop.pace,
        l.start,
        &app.r#loop.turns,
        &l.steps,
        &l.posted,
    );
    let path = format!("{}/live.json", l.out);
    if !crate::platform_native::platform_write_file(&path, &report) {
        eprintln!(
            "[ERROR] --- [{}:{}] live: could not write {path}",
            file!(),
            line!()
        );
    }
}

/// `<out>/live.json`: pace, the loop's turns ([ms since start, drawn 0/1]),
/// and each step with when it was due, posted and what. Pure, so the shape
/// is testable without the app.
fn live_json(
    pace: Pace,
    start: Time,
    turns: &[crate::app_loop::Turn],
    steps: &[LiveStep],
    posted: &[Time],
) -> String {
    let at = |t: Time| (t - start) as f64 / MS as f64;
    let mut out = format!(
        "{{\"pace\":\"{}\",\"turns\":[",
        if pace == Pace::Display {
            "display"
        } else {
            "60"
        }
    );
    for (i, turn) in turns.iter().enumerate() {
        out.push_str(&format!(
            "{}[{:.3},{}]",
            if i > 0 { "," } else { "" },
            at(turn.at),
            turn.drawn as i64
        ));
    }
    out.push_str("],\"steps\":[");
    for (i, step) in steps.iter().enumerate() {
        let posted_ms = if posted[i] != 0 {
            format!("{:.3}", at(posted[i]))
        } else {
            "-1.000".to_string()
        };
        out.push_str(&format!(
            "{}{{\"at\":{},\"posted\":{},\"what\":{}}}",
            if i > 0 { "," } else { "" },
            step.at,
            posted_ms,
            crate::harness::json_string(&describe(&step.what))
        ));
    }
    out.push_str("]}\n");
    out
}

/// One human line describing a step's kind.
fn describe(what: &LiveWhat) -> String {
    match what {
        LiveWhat::Input(input) => match input {
            Input::Pointer(e) => {
                format!(
                    "Pointer_Event{{pos = [{}, {}], left = {}}}",
                    e.pos[0], e.pos[1], e.left
                )
            }
            Input::Button(e) => format!(
                "Button_Event{{button = {}, down = {}, clicks = {}, pos = [{}, {}]}}",
                e.button, e.down, e.clicks, e.pos[0], e.pos[1]
            ),
            Input::Wheel(e) => format!(
                "Wheel_Event{{delta = [{}, {}], pos = [{}, {}]}}",
                e.delta[0], e.delta[1], e.pos[0], e.pos[1]
            ),
            Input::Pinch(e) => format!("Pinch_Event{{scale = {}}}", e.scale),
            Input::Key(e) => format!(
                "Key_Event{{key = {:?}, mods = {:?}, down = {}, repeat = {}}}",
                e.key, e.mods, e.down, e.repeat
            ),
            Input::Text(e) => format!("Text_Event{{text = {:?}}}", e.text),
            Input::Window(e) => format!(
                "Window_Event{{size = [{}, {}], pixels = [{}, {}]}}",
                e.size[0], e.size[1], e.pixels[0], e.pixels[1]
            ),
            Input::Focus(e) => format!("Focus_Event{{gained = {}}}", e.gained),
            Input::Scheme(e) => format!("Scheme_Event{{dark = {}}}", e.dark),
            Input::ReducedMotion(e) => {
                format!("Reduced_Motion_Event{{reduced = {}}}", e.reduced)
            }
            Input::Model(e) => format!("Model_Event{{set = {:?}}}", e.change.set),
        },
        LiveWhat::Text(t) => format!("text {:?}", t.text.to_string_lossy()),
        LiveWhat::Shot(s) => format!("shot {}", s.name),
        LiveWhat::Resize(r) => format!("size {} {}", r.size[0], r.size[1]),
        LiveWhat::Scheme(s) => format!("scheme {}", if s.dark { "dark" } else { "light" }),
        LiveWhat::End(_) => "end".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_live() -> Live {
        Live {
            app: std::ptr::null_mut(),
            out: String::new(),
            steps: Vec::new(),
            posted: Vec::new(),
            next: 0,
            start: 0,
            timer: 0,
            readback: Vec::new(),
            window_id: 0,
            window: std::ptr::null_mut(),
        }
    }

    #[test]
    fn since_counts_ms() {
        let mut l = empty_live();
        l.start = 1_000_000_000;
        assert_eq!(ms_since(&l, 1_000_500_000), 0.5);
    }

    #[test]
    fn describe_names_each_kind() {
        assert_eq!(
            describe(&LiveWhat::Shot(LiveShot {
                name: "a".to_string()
            })),
            "shot a"
        );
        assert_eq!(
            describe(&LiveWhat::Resize(LiveResize { size: [1, 2] })),
            "size 1 2"
        );
        assert_eq!(
            describe(&LiveWhat::Scheme(LiveScheme { dark: true })),
            "scheme dark"
        );
        assert_eq!(describe(&LiveWhat::End(LiveEnd {})), "end");
    }

    #[test]
    fn json_helpers_stay_pure() {
        let report = live_json(
            Pace::Display,
            0,
            &[crate::app_loop::Turn {
                at: 16_000_000,
                drawn: true,
            }],
            &[LiveStep {
                at: 0,
                what: LiveWhat::End(LiveEnd {}),
            }],
            &[0],
        );
        assert_eq!(
            report,
            "{\"pace\":\"display\",\"turns\":[[16.000,1]],\"steps\":[{\"at\":0,\"posted\":-1.000,\"what\":\"end\"}]}\n"
        );
    }
}
