//! The macOS specifics: wheel-event watching, display activity, reduced
//! motion, colour-space tagging, the ICC profile, appearances and cursors.
//!
//! macOS only. The AppKit and CoreGraphics calls behind these signatures
//! live in `csrc/platform_darwin.m` (compiled by `build.rs`); this module is
//! the safe boundary: it fetches the Cocoa window pointer through SDL's
//! properties API, converts the C strings and pixel buffers the `.m` file
//! hands back, and frees them. No SDL headers cross into the `.m` file.
//!
//! Ported from `src/platform_darwin.odin` (324 lines): same procedures, same
//! ring buffer, same report order. Two deliberate differences: the hand
//! cursors carry only their base image (the denser alternates are dropped;
//! the OS cursor never appears in a shot) and `darwin_cursor` returns the
//! finished cursor instead of a raw pointer.

use std::cell::RefCell;

use crate::input::Cursor;

thread_local! {
    /// The recent non-precise scroll timestamps' ring, on the main thread.
    static TICKS: RefCell<[u64; 8]> = const { RefCell::new([0; 8]) };
}

thread_local! {
    /// The ring's next slot.
    static TICKS_NEXT: RefCell<usize> = const { RefCell::new(0) };
}

thread_local! {
    /// What tagged the drawing buffer last, for the colour report.
    static TAGGED_BY: RefCell<String> = const { RefCell::new(String::new()) };
}

/// The space as the `.m` file takes it: 0 sRGB, 1 Display P3 (the Odin
/// `Space` order).
fn space_code(space: mv_theme::Space) -> std::os::raw::c_int {
    match space {
        mv_theme::Space::Srgb => 0,
        mv_theme::Space::DisplayP3 => 1,
    }
}

unsafe extern "C" {
    /// Whether any display is active (a background login has none).
    fn mv_darwin_display_active() -> bool;
    /// System Settings › Accessibility › Display › Reduce motion.
    fn mv_darwin_reduced_motion() -> bool;
    /// Whether AppKit's current event is a wheel event without precise
    /// scrolling deltas (a mouse wheel's ticks, not a trackpad).
    fn mv_darwin_current_wheel_is_tick() -> bool;
    /// Tags the Cocoa window with the space; reports the space set and a
    /// malloc'd account of who tagged it (freed by the caller).
    fn mv_darwin_tag_space(
        nswindow: *mut std::ffi::c_void,
        space: std::os::raw::c_int,
        set_space: *mut std::os::raw::c_int,
        tagged_by: *mut *mut std::os::raw::c_char,
    );
    /// The colour report's account of the window, its surface and its screen,
    /// malloc'd (freed by the caller).
    fn mv_darwin_color_report(
        nswindow: *mut std::ffi::c_void,
        tagged_by: *const std::os::raw::c_char,
    ) -> *mut std::os::raw::c_char;
    /// The space's ICC profile bytes, malloc'd into `out` (freed by the
    /// caller); false when AppKit has none.
    fn mv_darwin_icc_profile(
        space: std::os::raw::c_int,
        out: *mut *mut u8,
        len: *mut usize,
    ) -> bool;
    /// Sets the app's appearance to dark or light (the live driver's flip).
    fn mv_darwin_set_appearance(dark: bool);
    /// The open (grab) or closed (grabbing) hand's base image as malloc'd
    /// straight-alpha RGBA (freed by the caller); null when AppKit has none.
    /// The hotspot and size come back through the out-params.
    fn mv_darwin_hand_cursor(
        grabbing: bool,
        width: *mut std::os::raw::c_int,
        height: *mut std::os::raw::c_int,
        hot_x: *mut std::os::raw::c_int,
        hot_y: *mut std::os::raw::c_int,
    ) -> *mut u8;
}

/// The Cocoa window pointer behind an SDL window, or null.
fn cocoa_window(window: &sdl3::video::Window) -> *mut std::ffi::c_void {
    // SAFETY: the window is live; the properties id and the pointer read are
    // SDL's documented query, and a null default means no Cocoa window.
    unsafe {
        let props = sdl3::sys::video::SDL_GetWindowProperties(window.raw());
        sdl3::sys::properties::SDL_GetPointerProperty(
            props,
            sdl3::sys::video::SDL_PROP_WINDOW_COCOA_WINDOW_POINTER,
            std::ptr::null_mut(),
        )
    }
}

/// Watches wheel events for non-precise scroll ticks.
///
/// SDL's event watch runs as SDL takes each event; a tick's wheel event is
/// noted by its timestamp, which the event keeps in the queue (see
/// `platform_wheel_px`). The watch stays registered for the life of the
/// process, as the Odin one does.
pub fn darwin_watch_wheel() {
    // SAFETY: the filter only reads the event and records a timestamp in a
    // thread-local; it runs on the thread pumping events (the main thread
    // under SDL's main callbacks), where the thread-local is valid.
    let added =
        unsafe { sdl3::sys::events::SDL_AddEventWatch(Some(wheel_watch), std::ptr::null_mut()) };
    if !added {
        eprintln!("math-viz2-rust: wheel: no event watch: a wheel's ticks count as precise units");
    }
}

/// The SDL event watch: notes non-precise scroll timestamps.
///
/// Matches `SDL_EventFilter` exactly (the surface's looser signature is
/// narrowed here so the watch registers without a trampoline).
unsafe extern "C" fn wheel_watch(
    _userdata: *mut std::ffi::c_void,
    event: *mut sdl3::sys::events::SDL_Event,
) -> bool {
    // SAFETY: SDL hands the filter a valid event; only the common header is read.
    let is_wheel = unsafe { (*event).r#type == sdl3::sys::events::SDL_EVENT_MOUSE_WHEEL.0 };
    if !is_wheel {
        return true;
    }
    // SAFETY: a boolean query with no arguments.
    if unsafe { !sdl3::sys::init::SDL_IsMainThread() } {
        return true;
    }
    // SAFETY: the AppKit query runs on the main thread during event
    // delivery, where the current event is the scroll event itself.
    if unsafe { mv_darwin_current_wheel_is_tick() } {
        // SAFETY: a wheel event carries its SDL timestamp in the common header.
        let timestamp = unsafe { (*event).common.timestamp };
        TICKS.with(|ticks| {
            TICKS_NEXT.with(|next| {
                let mut ticks = ticks.borrow_mut();
                let mut next = next.borrow_mut();
                let slot = *next % ticks.len();
                ticks[slot] = timestamp;
                *next += 1;
            });
        });
    }
    true
}

/// Whether the timestamp is in the ticks ring.
pub fn darwin_wheel_ticked(timestamp: u64) -> bool {
    TICKS.with(|ticks| ticks.borrow().contains(&timestamp))
}

/// Whether any display is active. A background login has none (CoreGraphics
/// lists no active display while NSScreen, which SDL reads, still lists the
/// machine's screens), and a swap that waits for vsync there never returns.
pub fn darwin_display_active() -> bool {
    // SAFETY: a boolean query with no arguments.
    unsafe { mv_darwin_display_active() }
}

/// The system's reduce-motion setting.
pub fn darwin_reduced_motion() -> bool {
    // SAFETY: a boolean query with no arguments.
    unsafe { mv_darwin_reduced_motion() }
}

/// Tags the window's colour space between SDL's tag and the GL context.
///
/// The window's frames are drawn in `space`, Display P3 by default, and macOS
/// colour-matches them from that space to the display. Only the colour space
/// is set: the window's and the app's appearance stay the system's, which
/// SDL reads for System mode. Returns the space the window is in.
pub fn darwin_tag_space(window: &sdl3::video::Window, space: mv_theme::Space) -> mv_theme::Space {
    let ns = cocoa_window(window);
    if ns.is_null() {
        eprintln!("math-viz2-rust: colour space: no Cocoa window to tag; SDL's sRGB stands");
        TAGGED_BY.with(|tagged| *tagged.borrow_mut() = "nobody".to_string());
        return mv_theme::Space::Srgb;
    }
    let mut set_space = 0;
    let mut tagged_by: *mut std::os::raw::c_char = std::ptr::null_mut();
    // SAFETY: `ns` is the live Cocoa window; the out-params point at valid
    // locals; the returned string is malloc'd and freed below.
    unsafe {
        mv_darwin_tag_space(ns, space_code(space), &mut set_space, &mut tagged_by);
    }
    if !tagged_by.is_null() {
        // SAFETY: the `.m` file hands back a NUL-terminated string it
        // malloc'd; it is read once, then freed.
        let account = unsafe {
            std::ffi::CStr::from_ptr(tagged_by)
                .to_string_lossy()
                .into_owned()
        };
        TAGGED_BY.with(|tagged| *tagged.borrow_mut() = account);
        // SAFETY: same allocation as above.
        unsafe {
            libc::free(tagged_by.cast::<std::ffi::c_void>());
        }
    }
    if set_space == 1 {
        mv_theme::Space::DisplayP3
    } else {
        mv_theme::Space::Srgb
    }
}

/// The ICC profile of `space` as macOS has it (Display P3's is the 536 bytes
/// of `/System/Library/ColorSync/Profiles/Display P3.icc`), for the PNGs.
pub fn darwin_icc_profile(space: mv_theme::Space) -> Vec<u8> {
    let mut out: *mut u8 = std::ptr::null_mut();
    let mut len = 0usize;
    // SAFETY: the out-params point at valid locals; on true the buffer is
    // `len` malloc'd bytes, copied out and freed below.
    let ok = unsafe { mv_darwin_icc_profile(space_code(space), &mut out, &mut len) };
    if !ok || out.is_null() {
        return Vec::new();
    }
    // SAFETY: as above; the slice is exactly the handed-back bytes.
    let profile = unsafe { std::slice::from_raw_parts(out, len) }.to_vec();
    // SAFETY: the same allocation the `.m` file malloc'd.
    unsafe {
        libc::free(out.cast::<std::ffi::c_void>());
    }
    profile
}

/// Logs the colour spaces of the window, of the surface its frames go to,
/// and of the screen it is on. A login with no active display has no screen
/// to show the frames on, so the line says so.
pub fn darwin_color_report(window: &sdl3::video::Window) {
    let ns = cocoa_window(window);
    if ns.is_null() {
        return;
    }
    let tagged = TAGGED_BY.with(|tagged| tagged.borrow().clone());
    let tagged_c = std::ffi::CString::new(tagged).unwrap_or_default();
    // SAFETY: `ns` is the live Cocoa window and `tagged_c` a valid C string;
    // the report is malloc'd and freed below.
    let report = unsafe { mv_darwin_color_report(ns, tagged_c.as_ptr()) };
    if report.is_null() {
        return;
    }
    // SAFETY: the `.m` file hands back a NUL-terminated string it malloc'd;
    // it is read once, then freed.
    let text = unsafe {
        std::ffi::CStr::from_ptr(report)
            .to_string_lossy()
            .into_owned()
    };
    eprintln!("math-viz2-rust: colour space: {text}");
    // SAFETY: the same allocation as above.
    unsafe {
        libc::free(report.cast::<std::ffi::c_void>());
    }
}

/// Sets the app's appearance, which SDL observes and reports as
/// `SDL_EVENT_SYSTEM_THEME_CHANGED`. Only the live driver calls it.
pub fn darwin_set_appearance(dark: bool) {
    // SAFETY: a setter with a plain boolean argument.
    unsafe {
        mv_darwin_set_appearance(dark);
    }
}

/// A hand cursor image for grab/grabbing, else nothing (the platform falls
/// back to the system cursor map).
///
/// The base image only: SDL copies the surface's pixels at creation (the
/// Odin side frees its surfaces right after), so the surface is dropped here.
pub fn darwin_cursor(cursor: Cursor) -> Option<sdl3::mouse::Cursor> {
    let grabbing = match cursor {
        Cursor::Grab => false,
        Cursor::Grabbing => true,
        _ => return None,
    };
    let (mut width, mut height, mut hot_x, mut hot_y) = (0, 0, 0, 0);
    // SAFETY: the out-params point at valid locals; on non-null the buffer
    // is `4*width*height` malloc'd bytes, copied out and freed below.
    let pixels =
        unsafe { mv_darwin_hand_cursor(grabbing, &mut width, &mut height, &mut hot_x, &mut hot_y) };
    if pixels.is_null() || width <= 0 || height <= 0 {
        return None;
    }
    // SAFETY: as above; the handed-back bytes are exactly the image.
    let mut rgba =
        unsafe { std::slice::from_raw_parts(pixels, 4 * width as usize * height as usize) }
            .to_vec();
    // SAFETY: the same allocation the `.m` file malloc'd.
    unsafe {
        libc::free(pixels.cast::<std::ffi::c_void>());
    }
    let surface = sdl3::surface::Surface::from_data(
        &mut rgba,
        width as u32,
        height as u32,
        4 * width as u32,
        sdl3::pixels::PixelFormat::RGBA32,
    )
    .ok()?;
    sdl3::mouse::Cursor::from_surface(&surface, hot_x, hot_y).ok()
}
