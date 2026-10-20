//! The web platform: the Emscripten side of the interface.
//!
//! Web only. Arguments are nil, hints are a no-op, GL is ES 3.0, and prefs
//! live in local storage under the port's own key. The JavaScript imports
//! below are implemented by the web shell (`web/library.js`); the exports are
//! called from it. Logging goes to stderr, which the shell forwards to the
//! console as errors (so a panic or a logged error counts as one).
//!
//! Ported from `src/platform_web.odin` (304 lines): same procedures, same
//! exports, same order. `unsafe` here touches only the Emscripten imports and
//! the event push, as on the native side. Two deliberate differences: the
//! allocator pair is one function (`emscripten_alloc`; Rust's `System`
//! allocator is already `malloc`-backed, so there is no allocator model to
//! install) and there is no `app_context` global (the surface deferred it to
//! the wiring; the exports build owned values instead of borrowing it).

use enum_map::EnumMap;

use crate::app::App;
use crate::app_loop::Pace;
use crate::input::Cursor;

unsafe extern "C" {
    /// Reports the loop counts to the shell: `Module.iterations` stands
    /// still while the loop is idle, and the harness waits on
    /// `Module.frames`.
    pub fn mv_loop_counts(iterations: std::os::raw::c_int, frames: std::os::raw::c_int);
    /// Parks the main loop after this turn.
    pub fn mv_loop_idle();
    /// Wakes a parked main loop; resuming a running loop does nothing.
    pub fn mv_loop_wake();
    /// Runs after init: the paused-clock step, the model-line path, the
    /// pinch read, the storage watch and the cursor rule.
    pub fn mv_web_init();
    /// Whether `?trace` is in the address: log each fact and timer.
    pub fn mv_trace() -> bool;
    /// Whether `?panic-at-frame` is in the address: panic in frame 1, as
    /// `MV_PANIC_AT_FRAME=1` does natively, for the sweep's crash proof.
    pub fn mv_panic_at_frame() -> bool;
    /// The `prefers-reduced-motion` media query's answer.
    pub fn mv_reduced_motion() -> bool;
    /// Sets the canvas cursor by CSS name, in a rule SDL's own writes don't
    /// override.
    pub fn mv_set_cursor(name: *const u8, len: std::os::raw::c_int);
    /// Reads the prefs key into the buffer; returns the stored length in
    /// bytes (copied when the buffer holds it and its terminator), or -1 when
    /// nothing is stored or storage throws.
    pub fn mv_prefs_load(
        key: *const u8,
        key_len: std::os::raw::c_int,
        buf: *mut u8,
        cap: std::os::raw::c_int,
    ) -> std::os::raw::c_int;
    /// Stores the prefs key; false when storage throws.
    pub fn mv_prefs_save(
        key: *const u8,
        key_len: std::os::raw::c_int,
        text: *const u8,
        len: std::os::raw::c_int,
    ) -> bool;
    /// Whether the drawing buffer is Display P3 (the `?space=` rule: p3 or
    /// display-p3 forces it, srgb refuses it, else WebKit and Chromium take
    /// it and Firefox does not).
    pub fn mv_color_space() -> bool;
    /// Tags the drawing buffer's colour space from the next frame drawn;
    /// whether its buffer is Display P3 after (an engine without the
    /// attribute keeps sRGB).
    pub fn mv_tag_drawing_buffer(p3: bool) -> bool;
    /// Tells the shell whether a number field is being edited, so the page's
    /// key listeners keep Meta combos with the app while one is (the user's
    /// ruling 2026-10-18 on the web build's browser shortcuts). A flag the
    /// shell sets; no export is needed.
    pub fn mv_field_editing(editing: bool);
    /// The page's hash (`location.hash`), freshly allocated; the caller reads
    /// it once and frees it through `free`.
    pub fn mv_read_hash() -> *mut std::os::raw::c_char;
    /// Writes the page's hash through `history.replaceState` (no history
    /// entry), so the page behaves as math-viz2's does.
    pub fn mv_write_hash(hash: *const std::os::raw::c_char);
    /// The C allocator's memalign.
    pub fn memalign(alignment: usize, size: usize) -> *mut std::ffi::c_void;
    /// The C allocator's free.
    pub fn free(ptr: *mut std::ffi::c_void);
}

/// A pinch from the shell (a trackpad pinch reaches a page as wheel events
/// with Ctrl held): queued as SDL's own pinch event.
#[unsafe(no_mangle)]
pub extern "C" fn mv_pinch(scale: f32) {
    // SAFETY: a zeroed event with the type and scale set is what the plane
    // reads; pushing it is thread-safe by Emscripten's contract (the call
    // comes from the browser thread, as SDL's own events do).
    unsafe {
        let mut event = sdl3::sys::events::SDL_PinchFingerEvent {
            r#type: sdl3::sys::events::SDL_EventType::PINCH_UPDATE,
            scale,
            ..Default::default()
        };
        sdl3::sys::events::SDL_PushEvent((&raw mut event).cast::<sdl3::sys::events::SDL_Event>());
    }
}

/// Stored prefs from another tab (the storage event): the app re-reads its
/// keys on the main thread, so this only wakes it with the prefs user event.
///
/// # Safety
///
/// `stored` is the shell's storage text, read never; it only keeps the
/// export's shape the shell calls. Called from the shell's storage listener
/// on the browser thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mv_prefs_stored(stored: *const std::os::raw::c_char) {
    // The event carries one key's new value, while the app needs all three;
    // the main thread re-reads them (see the `Prefs` arm in `main.rs`), as
    // math-viz2's storage subscription re-reads its atoms.
    let _ = stored;
    crate::app_loop::push_user_event(
        crate::app_loop::UserCode::Prefs,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
    );
}

/// The media query changed: queued as the app's own event (0 or 1).
#[unsafe(no_mangle)]
pub extern "C" fn mv_reduced_motion_changed(reduced: bool) {
    crate::app_loop::push_user_event(
        crate::app_loop::UserCode::ReducedMotion,
        usize::from(reduced) as *mut std::ffi::c_void,
        std::ptr::null_mut(),
    );
}

/// A model line from the harness page (`Module.mvModelLine`, which the
/// harness calls for a script's `port model` line): the line read as
/// `src/script` reads it and queued as the fact a native run queues; false
/// when it isn't a model line.
///
/// # Safety
///
/// `line` must be a valid NUL-terminated string; it is only read during the
/// call. Called from the shell's `mvModelLine` on the browser thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mv_model_line(line: *const std::os::raw::c_char) -> bool {
    // SAFETY: the shell hands a NUL-terminated string it allocated; it is
    // only read here, never retained.
    let line = unsafe {
        if line.is_null() {
            return false;
        }
        std::ffi::CStr::from_ptr(line)
            .to_string_lossy()
            .into_owned()
    };
    // The script package's own words, so the copy keeps nothing of the line.
    let (run, err) = mv_script::parse(&format!("port {line}"));
    if let Some(err) = err {
        eprintln!("math-viz2-rust: model line {line:?}: {}", err.message);
        return false;
    }
    if run.actions.len() != 1 {
        return false;
    }
    let mv_script::Verb::Model(model) = &run.actions[0].verb else {
        return false;
    };
    // The model crosses the event as an owned clone; the app frees it when
    // the event's second word says so, as the native side does.
    let owned = Box::new(model.clone());
    crate::app_loop::push_user_event(
        crate::app_loop::UserCode::Model,
        Box::into_raw(owned).cast::<std::ffi::c_void>(),
        std::ptr::dangling_mut::<std::ffi::c_void>(),
    )
}

/// The page's hash changed under the app (an edit in the address bar,
/// back/forward): queued as the app's own event. The main thread re-reads
/// the hash there, as math-viz2's `hashchange` subscription re-reads its
/// route and search string. Called from the shell's `hashchange` listener
/// on the browser thread; our own writes go through `history.replaceState`,
/// which fires no `hashchange`, so they never loop back here.
#[unsafe(no_mangle)]
pub extern "C" fn mv_hash_changed() {
    crate::app_loop::push_user_event(
        crate::app_loop::UserCode::Hash,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
    );
}

/// Sets up the default context.
///
/// Nothing to install on this side: Rust's `System` allocator calls
/// `malloc`/`posix_memalign`, not the trapping `aligned_alloc`, so the Odin
/// side's allocator swap has no equivalent, and there is no logger framework
/// (callers write to stderr, which reaches the console).
pub fn platform_context() {}

/// No arguments on the web.
pub fn platform_args() -> Vec<String> {
    Vec::new()
}

/// No hints on the web.
pub fn platform_hints() {}

/// GLES 3.0 (WebGL2) with an 8-bit stencil.
///
/// Uses the raw API, as on the native side: no subsystem handle at this point.
pub fn platform_gl_attributes() {
    use sdl3::sys::video as sys;
    // SAFETY: plain integer attributes on SDL's global GL state; the values
    // are SDL's own constants.
    unsafe {
        sys::SDL_GL_SetAttribute(sys::SDL_GL_CONTEXT_MAJOR_VERSION, 3);
        sys::SDL_GL_SetAttribute(sys::SDL_GL_CONTEXT_MINOR_VERSION, 0);
        sys::SDL_GL_SetAttribute(
            sys::SDL_GL_CONTEXT_PROFILE_MASK,
            sys::SDL_GL_CONTEXT_PROFILE_ES.0,
        );
        sys::SDL_GL_SetAttribute(sys::SDL_GL_STENCIL_SIZE, 8);
    }
}

/// Wheel pixels per notch on the web: the port's wheel handler divides a
/// `WheelEvent`'s pixel deltas by 100, so the canvas scrolls by the DOM's own
/// px.
pub const PLATFORM_WHEEL_PX: f32 = 100.0;

/// Wheel pixels for one event on the web.
pub fn platform_wheel_px(_event: &sdl3::event::Event) -> f32 {
    PLATFORM_WHEEL_PX
}

/// The browser paces frames: SDL's port runs the loop on
/// `requestAnimationFrame`.
pub fn platform_default_pace() -> Pace {
    Pace::Display
}

/// Display P3 when the shell says the buffer is, else sRGB.
pub fn platform_default_space() -> mv_theme::Space {
    // SAFETY: a boolean query with no arguments.
    if unsafe { mv_color_space() } {
        mv_theme::Space::DisplayP3
    } else {
        mv_theme::Space::Srgb
    }
}

/// The shell owns the canvas: the space passes through unchanged (the drawing
/// buffer is tagged at init, once a GL context exists).
pub fn platform_window_made(
    _window: &sdl3::video::Window,
    space: mv_theme::Space,
) -> mv_theme::Space {
    space
}

/// Tags the drawing buffer, sets the swap interval, watches for wakeups, and
/// tells the shell init is done.
///
/// SDL's WebGL2 context is tagged with the space; an engine without the
/// attribute keeps sRGB, and the frames are drawn in the space the buffer
/// took.
pub fn platform_init(app: &mut App) {
    // Every event SDL queues, from the page's input or the app's own timers,
    // resumes the main loop if it was paused.
    // SAFETY: the watch only calls the shell's wake; it runs on the thread
    // pumping events (the browser thread under the port's callbacks).
    unsafe {
        sdl3::sys::events::SDL_AddEventWatch(Some(wake_on_event), std::ptr::null_mut());
    }
    crate::input::input_watch_pinch();
    let asked = app.space;
    // SAFETY: a boolean query taking a boolean; the buffer's tag follows.
    app.space = if unsafe { mv_tag_drawing_buffer(asked == mv_theme::Space::DisplayP3) } {
        mv_theme::Space::DisplayP3
    } else {
        mv_theme::Space::Srgb
    };
    println!(
        "math-viz2-rust: colour space: the WebGL2 drawing buffer's {}{}",
        BUFFER_SPACES[app.space],
        if app.space == asked {
            ""
        } else {
            " (the engine has no other)"
        },
    );
    // SAFETY: the GL context is current on this thread; the swap call only
    // sets an integer on SDL's global GL state.
    unsafe {
        sdl3::sys::video::SDL_GL_SetSwapInterval(1);
    }
    // SAFETY: no arguments; the shell wires the paused-clock step, the
    // model-line path, the pinch read and the storage watch.
    unsafe {
        mv_web_init();
    }
}

/// Each space's buffer tag (`drawingBufferColorSpace`'s words).
static BUFFER_SPACES: EnumMap<mv_theme::Space, &str> = EnumMap::from_array(["srgb", "display-p3"]);

/// Nothing to quit on the web.
pub fn platform_quit() {}

/// The SDL event watch: wakes the parked loop.
///
/// Matches `SDL_EventFilter` exactly (the surface's looser signature is
/// narrowed here so the watch registers without a trampoline).
unsafe extern "C" fn wake_on_event(
    _userdata: *mut std::ffi::c_void,
    _event: *mut sdl3::sys::events::SDL_Event,
) -> bool {
    // SAFETY: no arguments; waking a running loop does nothing.
    unsafe {
        mv_loop_wake();
    }
    true
}

/// Wakes the loop when awake, else parks it when no events wait.
///
/// While anything moves the main loop runs on every animation frame; idle, it
/// pauses after this frame, unless an event came in meanwhile.
pub fn platform_pace(_app: &mut App, awake: bool) {
    if awake {
        // SAFETY: no arguments; waking a running loop does nothing.
        unsafe {
            mv_loop_wake();
        }
    } else {
        // SAFETY: a pure query over SDL's event queue.
        let waiting = unsafe {
            sdl3::sys::events::SDL_HasEvents(
                sdl3::sys::events::SDL_EventType::FIRST.0,
                sdl3::sys::events::SDL_EventType::LAST.0,
            )
        };
        if !waiting {
            // SAFETY: no arguments.
            unsafe {
                mv_loop_idle();
            }
        }
    }
}

/// Forwards the loop counts to the shell: what the harness waits on.
pub fn platform_loop_counts(_iterations: i64, _frames: i64) {
    // SAFETY: two small integers to the shell's counter.
    unsafe {
        mv_loop_counts(
            _iterations as std::os::raw::c_int,
            _frames as std::os::raw::c_int,
        );
    }
}

/// SDL's nanosecond clock: the app clock's ticks when no script runs.
///
/// Same raw call as the native side (the safe `sdl3` crate has no ns clock);
/// on the web SDL's clock reads whole ms (Emscripten falls back to
/// `gettimeofday`), which the loop check allows for.
pub fn platform_ticks_ns() -> u64 {
    // SAFETY: a pure clock read with no arguments and no retained state.
    unsafe { sdl3::sys::timer::SDL_GetTicksNS() }
}

/// Pushes one of the app's SDL user events (`UserCode` as `i32`, two data
/// words). Same raw call as the native side: the safe `sdl3` crate's push
/// needs an event-subsystem handle, which a contextless timer callback and the
/// shell's exports have no way to reach.
pub fn platform_push_user(
    code: i32,
    data1: *mut std::ffi::c_void,
    data2: *mut std::ffi::c_void,
) -> bool {
    // SAFETY: the event is a plain data struct the queue copies on push; the
    // data words travel opaquely and are never dereferenced here.
    unsafe {
        let mut event = sdl3::sys::events::SDL_Event {
            user: sdl3::sys::events::SDL_UserEvent {
                r#type: crate::app_loop::USER_EVENT_TYPE.load(std::sync::atomic::Ordering::Relaxed),
                reserved: 0,
                timestamp: 0,
                windowID: sdl3::sys::video::SDL_WindowID(0),
                code,
                data1,
                data2,
                ..Default::default()
            },
        };
        sdl3::sys::events::SDL_PushEvent(&mut event)
    }
}

/// Arms the wake timer to fire `interval_ns` from now, posting through the
/// loop's `wake` callback; returns SDL's timer id. Same raw call as the
/// native side (on the web SDL's timer is a `setTimeout`).
pub fn platform_timer_arm(interval_ns: u64) -> u32 {
    // SAFETY: `wake` is a contextless callback taking only its id and
    // interval; it pushes a user event and keeps no state.
    unsafe {
        sdl3::sys::timer::SDL_AddTimerNS(
            interval_ns,
            Some(crate::app_loop::wake),
            std::ptr::null_mut(),
        )
        .0
    }
}

/// Removes a wake timer by the id `platform_timer_arm` returned. On the web
/// SDL keeps a spent timer until it is removed, so the wake's removal lives
/// here rather than only in `woken`.
pub fn platform_timer_remove(id: u32) {
    // SAFETY: the id came from `platform_timer_arm`; removing it only stops
    // the callback from firing.
    unsafe {
        sdl3::sys::timer::SDL_RemoveTimer(sdl3::sys::timer::SDL_TimerID(id));
    }
}

/// The web has no refresh-rate query: the browser paces frames by its
/// animation frames and tells a page no rate.
pub fn platform_refresh_rate(_app: &mut App) -> f32 {
    0.0
}

/// Forwards the cursor's CSS name to the shell, in a rule SDL's own cursor
/// writes don't override.
pub fn platform_set_cursor(_cursor: Cursor) {
    let name = crate::input::CURSOR_CSS[_cursor];
    // SAFETY: the CSS name is a live `&str`; the shell copies it into the
    // rule before returning.
    unsafe {
        mv_set_cursor(name.as_ptr(), name.len() as std::os::raw::c_int);
    }
}

/// The media query's answer.
pub fn platform_reduced_motion() -> bool {
    // SAFETY: a boolean query with no arguments.
    unsafe { mv_reduced_motion() }
}

/// Whether the system theme is dark.
///
/// Same raw call as the native side (the safe `sdl3` crate has no theme
/// query): SDL's port answers from the page's colour scheme.
pub fn platform_system_dark() -> bool {
    // SAFETY: a pure setting read with no arguments and no retained state.
    unsafe { sdl3::sys::video::SDL_GetSystemTheme() != sdl3::sys::video::SDL_SYSTEM_THEME_LIGHT }
}

/// Tells the shell whether a number field is being edited, so the page's key
/// listeners keep Meta combos with the app while one is (number_field.rs
/// calls it when a field opens and closes).
pub fn platform_field_editing(editing: bool) {
    // SAFETY: a boolean flag set with no arguments retained.
    unsafe {
        mv_field_editing(editing);
    }
}

/// The prefs live under the port's own storage keys (never math-viz2's). The
/// string only records that prefs persist on this side (localStorage); the
/// three keys ride in the load/save below. Empty while a script runs, which
/// keeps prefs in memory.
pub fn platform_prefs_store(_dir: &str) -> String {
    "localStorage".to_string()
}

/// Reads the three prefs keys from local storage; empty and not found when
/// nothing is stored or storage throws (as in a private window). Each value
/// is checked as math-viz2 checks it (a missing or unknown one taking its
/// default); the triple returns as the one JSON object the native file
/// holds, so both sides share the parse.
pub fn platform_load_prefs(app: &mut App) -> (String, bool) {
    if app.prefs_store.is_empty() {
        return (String::new(), false);
    }
    let mut prefs = mv_theme::DEFAULT_PREFS;
    let mut found = false;
    if let Some(text) = prefs_key_load(mv_theme::PREFS_KEY_LIGHT_DARK) {
        found = true;
        if let Some(mode) = stored_mode(&text) {
            prefs.mode = mode;
        }
    }
    if let Some(text) = prefs_key_load(mv_theme::PREFS_KEY_DARK_THEME) {
        found = true;
        if let Some(dark) = stored_slug(&text, mv_theme::Scheme::Dark) {
            prefs.dark = dark;
        }
    }
    if let Some(text) = prefs_key_load(mv_theme::PREFS_KEY_LIGHT_THEME) {
        found = true;
        if let Some(light) = stored_slug(&text, mv_theme::Scheme::Light) {
            prefs.light = light;
        }
    }
    (mv_theme::prefs_json(prefs), found)
}

/// Saves the prefs, one value per key; false when storage throws.
pub fn platform_save_prefs(app: &mut App, text: &str) -> bool {
    if app.prefs_store.is_empty() {
        return false;
    }
    let prefs = mv_theme::parse_prefs(text);
    prefs_key_save(
        mv_theme::PREFS_KEY_LIGHT_DARK,
        &stored_json(mv_theme::MODE_WORDS[prefs.mode]),
    ) && prefs_key_save(
        mv_theme::PREFS_KEY_DARK_THEME,
        &stored_json(mv_theme::SLUG_WORDS[prefs.dark]),
    ) && prefs_key_save(
        mv_theme::PREFS_KEY_LIGHT_THEME,
        &stored_json(mv_theme::SLUG_WORDS[prefs.light]),
    )
}

/// One value as stored: the JSON string, as math-viz2 writes it.
fn stored_json(word: &str) -> String {
    serde_json::to_string(word).unwrap_or_else(|_| format!("{word:?}"))
}

/// A stored mode word as read: `None` for a missing or unknown one.
fn stored_mode(text: &str) -> Option<mv_theme::Mode> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    let word = value.as_str()?;
    for (mode, w) in &mv_theme::MODE_WORDS {
        if *w == word {
            return Some(mode);
        }
    }
    None
}

/// A stored theme slug as read: `None` for a missing one, an unknown one, or
/// one of the other scheme's (each key keeps its scheme's pick).
fn stored_slug(text: &str, scheme: mv_theme::Scheme) -> Option<mv_theme::Slug> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    let word = value.as_str()?;
    for (slug, w) in &mv_theme::SLUG_WORDS {
        if *w == word && mv_theme::slug_scheme(slug) == scheme {
            return Some(slug);
        }
    }
    None
}

/// Reads one prefs key from local storage; `None` when nothing is stored or
/// storage throws.
fn prefs_key_load(key: &str) -> Option<String> {
    // SAFETY: the key is a live `&str`; a null buffer asks for the length.
    let n = unsafe {
        mv_prefs_load(
            key.as_ptr(),
            key.len() as std::os::raw::c_int,
            std::ptr::null_mut(),
            0,
        )
    };
    if n < 0 {
        return None;
    }
    let mut buf = vec![0u8; (n + 1) as usize];
    // SAFETY: `buf` holds `n + 1` bytes; the shell copies at most that many.
    unsafe {
        mv_prefs_load(
            key.as_ptr(),
            key.len() as std::os::raw::c_int,
            buf.as_mut_ptr(),
            n + 1,
        );
    }
    Some(String::from_utf8_lossy(&buf[..n as usize]).into_owned())
}

/// Saves one prefs key to local storage; false when storage throws.
fn prefs_key_save(key: &str, text: &str) -> bool {
    // SAFETY: the key and text are live; the shell copies both before
    // returning.
    unsafe {
        mv_prefs_save(
            key.as_ptr(),
            key.len() as std::os::raw::c_int,
            text.as_ptr(),
            text.len() as std::os::raw::c_int,
        )
    }
}

/// The page's hash at start: `location.hash` through `mv_read_hash`, whole
/// (`#function-viewer?center=1,2&scale=2`). Callers parse it with `Url`
/// (`Url::new`, `sync_from_hash`): cutting the query here dropped it, since
/// those split the full hash again and read an empty query (h2-plane found
/// the web build rendering every query state at its defaults).
pub fn platform_read_hash() -> String {
    // SAFETY: the shell hands a fresh NUL-terminated string; it is read once
    // here, then freed through the shell's `free`.
    unsafe {
        let ptr = mv_read_hash();
        if ptr.is_null() {
            return String::new();
        }
        let hash = std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned();
        free(ptr.cast::<std::ffi::c_void>());
        hash
    }
}

/// Writes the page's hash through `history.replaceState` (no history entry),
/// so the page behaves as math-viz2's does. Takes the full hash
/// (`Url::full_hash`); a hash with an interior NUL is never written. The
/// milestones call this on the frame step once their params' writes change
/// the query.
pub fn platform_write_hash(hash: &str) {
    let Ok(hash) = std::ffi::CString::new(hash) else {
        return;
    };
    // SAFETY: the NUL-terminated hash is live for the call; the shell copies
    // it before returning.
    unsafe {
        mv_write_hash(hash.as_ptr());
    }
}

/// No files on the web.
pub fn platform_read_file(path: &str) -> (Vec<u8>, bool) {
    eprintln!("math-viz2-rust: no files on the web: {path}");
    (Vec::new(), false)
}

/// Allocates `size` bytes aligned to `alignment`, zeroed when asked.
///
/// `memalign` rather than `aligned_alloc`: Emscripten's `aligned_alloc`
/// returns null unless the size is a multiple of the alignment.
pub fn emscripten_alloc(size: usize, alignment: usize, zero: bool) -> *mut std::ffi::c_void {
    let alignment = alignment.max(size_of::<*mut std::ffi::c_void>());
    let size = size.max(1);
    // SAFETY: a non-zero size and a pointer alignment; null means out of
    // memory, and the bytes are fresh `malloc` memory.
    let ptr = unsafe { memalign(alignment, size) };
    if ptr.is_null() {
        return ptr;
    }
    if zero {
        // SAFETY: `size` fresh bytes starting at `ptr`.
        unsafe {
            std::ptr::write_bytes(ptr.cast::<u8>(), 0, size);
        }
    }
    ptr
}
