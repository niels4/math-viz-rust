//! The native side of the platform interface: SDL hints, GL attributes,
//! cursors, prefs files, PNGs.
//!
//! Native only. The window-made, pace and colour-space queries defer to the
//! macOS specifics where they exist; signals come from the unix/windows
//! modules. Logging goes to stderr with the binary's prefix, as `main.rs`
//! does (there is no logger framework on this side).
//!
//! Ported from `src/platform_native.odin` (334 lines): same procedures, same
//! constants, same order. `unsafe` here touches only SDL's raw API (event
//! pushing, GL attributes, window properties) where the safe `sdl3` crate has
//! no wrapper; the pixel and file work is safe.

use std::cell::RefCell;

use enum_map::EnumMap;

use crate::app::App;
use crate::app_loop::Pace;
use crate::input::Cursor;

thread_local! {
    /// The cursor cache, one per cursor, on the main thread.
    static CURSORS: RefCell<EnumMap<Cursor, Option<sdl3::mouse::Cursor>>> =
        const { RefCell::new(EnumMap::from_array([None, None, None, None, None, None, None, None])) };
}

/// Sets up the default context.
///
/// Nothing to install on this side: Rust has no global logger (callers write
/// to stderr) and the system allocator needs no setup.
pub fn platform_context() {}

/// The command-line arguments, including the program name first, as the
/// harness's option parser expects.
pub fn platform_args() -> Vec<String> {
    std::env::args().collect()
}

/// SDL hints: callback pacing, scroll momentum, signal handling.
///
/// Runs before SDL's init, so SDL installs no signal handler of its own.
pub fn platform_hints() {
    // Iterate only when an event arrives, so an idle window costs nothing.
    sdl3::hint::set("SDL_MAIN_CALLBACK_RATE", "waitevent");
    // A trackpad's scroll keeps its momentum after the fingers lift, as
    // Safari's does; SDL drops it on macOS unless asked.
    sdl3::hint::set("SDL_MAC_SCROLL_MOMENTUM", "1");
    // Ctrl-C and SIGTERM quit an idle app at once.
    #[cfg(unix)]
    crate::platform_unix::platform_signals();
    #[cfg(windows)]
    crate::platform_windows::platform_signals();
}

/// GL 3.3 core, forward-compatible, with an 8-bit stencil.
///
/// Uses the raw API: this runs without a subsystem handle, and the attribute
/// call is process-global state in SDL.
pub fn platform_gl_attributes() {
    use sdl3::sys::video as sys;
    // SAFETY: plain integer attributes on SDL's global GL state; valid
    // before or after init, and the values are SDL's own constants.
    unsafe {
        sys::SDL_GL_SetAttribute(sys::SDL_GL_CONTEXT_MAJOR_VERSION, 3);
        sys::SDL_GL_SetAttribute(sys::SDL_GL_CONTEXT_MINOR_VERSION, 3);
        sys::SDL_GL_SetAttribute(
            sys::SDL_GL_CONTEXT_PROFILE_MASK,
            sys::SDL_GL_CONTEXT_PROFILE_CORE.0,
        );
        sys::SDL_GL_SetAttribute(
            sys::SDL_GL_CONTEXT_FLAGS,
            sys::SDL_GL_CONTEXT_FORWARD_COMPATIBLE_FLAG.0,
        );
        sys::SDL_GL_SetAttribute(sys::SDL_GL_STENCIL_SIZE, 8);
    }
}

/// Wheel pixels per notch: 10 on macOS, 100 elsewhere.
#[cfg(target_os = "macos")]
pub const PLATFORM_WHEEL_PX: f32 = 10.0;
#[cfg(not(target_os = "macos"))]
pub const PLATFORM_WHEEL_PX: f32 = 100.0;

/// Wheel pixels per tick: 40 on macOS, 100 elsewhere.
#[cfg(target_os = "macos")]
pub const PLATFORM_WHEEL_TICK_PX: f32 = 40.0;
#[cfg(not(target_os = "macos"))]
pub const PLATFORM_WHEEL_TICK_PX: f32 = 100.0;

/// Wheel pixels for one event: a mouse wheel's ticks on macOS tell from
/// precise units (the darwin watch notes them by timestamp); elsewhere
/// precise units.
pub fn platform_wheel_px(event: &sdl3::event::Event) -> f32 {
    #[cfg(target_os = "macos")]
    if let sdl3::event::Event::MouseWheel { timestamp, .. } = event
        && crate::platform_darwin::darwin_wheel_ticked(*timestamp)
    {
        return PLATFORM_WHEEL_TICK_PX;
    }
    let _ = event;
    PLATFORM_WHEEL_PX
}

/// The default pace: fixed 60 Hz where no display is active (a background
/// login's swaps would wait for a vsync that never comes), else the display.
pub fn platform_default_pace() -> Pace {
    #[cfg(target_os = "macos")]
    if !crate::platform_darwin::darwin_display_active() {
        return Pace::Fixed60;
    }
    Pace::Display
}

/// The default colour space: Display P3 on macOS, sRGB on Windows and Linux,
// where SDL exposes no window colour space.
pub fn platform_default_space() -> mv_theme::Space {
    #[cfg(target_os = "macos")]
    return mv_theme::Space::DisplayP3;
    #[cfg(not(target_os = "macos"))]
    return mv_theme::Space::Srgb;
}

/// Tags the window's colour space between SDL's tag and the GL context: on
/// macOS the window is tagged with `space` now, before the GL context makes
/// the surface that takes the window's colour space and keeps it. Windows
/// and Linux draw in sRGB.
pub fn platform_window_made(
    window: &sdl3::video::Window,
    space: mv_theme::Space,
) -> mv_theme::Space {
    #[cfg(target_os = "macos")]
    return crate::platform_darwin::darwin_tag_space(window, space);
    #[cfg(not(target_os = "macos"))]
    {
        if space != mv_theme::Space::Srgb {
            println!(
                "function-viewer-rust: colour space: SDL can't tag a window {} here; drawing in sRGB",
                mv_theme::color::SPACE_WORDS[space],
            );
        }
        return mv_theme::Space::Srgb;
    }
}

/// Watches the wheel, sets the swap interval, and logs the pace.
///
/// `app.window` exists and the GL context is current when this runs.
pub fn platform_init(app: &mut App) {
    #[cfg(target_os = "macos")]
    {
        crate::platform_darwin::darwin_watch_wheel();
        crate::platform_darwin::darwin_color_report(app.window());
    }
    crate::input::input_watch_pinch();
    // SAFETY: the GL context is current on this thread; the swap call only
    // sets an integer on SDL's global GL state.
    unsafe {
        sdl3::sys::video::SDL_GL_SetSwapInterval(i32::from(app.r#loop.pace != Pace::Display));
    }
    println!(
        "function-viewer-rust: frames paced {}; the window's display refreshes at {:.0} Hz",
        if app.r#loop.pace == Pace::Display {
            "by the display"
        } else {
            "at 60 Hz by SDL's callback rate, no vsync"
        },
        platform_refresh_rate(app),
    );
}

/// Destroys the cached cursors.
pub fn platform_quit() {
    CURSORS.with(|cache| {
        for (_, slot) in cache.borrow_mut().iter_mut() {
            *slot = None;
        }
    });
}

/// How the callbacks are paced.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Rate {
    WaitEvent,
    Display,
    Hz60,
}

thread_local! {
    /// The last rate set, so hints change only on change.
    static PACE_RATE: RefCell<Option<Rate>> = const { RefCell::new(None) };
}

/// Sets the main-callback rate from awake/pace: waiting on events while idle;
/// while anything moves, at the display's pace (vsync does it, so the rate is
/// 0) or at a fixed 60 Hz. SDL reads the hint again as it changes.
pub fn platform_pace(app: &mut App, awake: bool) {
    let want = if !awake {
        Rate::WaitEvent
    } else if app.r#loop.pace == Pace::Display {
        Rate::Display
    } else {
        Rate::Hz60
    };
    let changed = PACE_RATE.with(|rate| {
        let mut rate = rate.borrow_mut();
        if *rate == Some(want) {
            return false;
        }
        *rate = Some(want);
        true
    });
    if changed {
        let value = match want {
            Rate::WaitEvent => "waitevent",
            Rate::Display => "0",
            Rate::Hz60 => "60",
        };
        sdl3::hint::set("SDL_MAIN_CALLBACK_RATE", value);
        if crate::trace() {
            println!("function-viewer-rust: loop: callback rate {value}");
        }
    }
}

/// Forwards the loop counts; native needs none.
pub fn platform_loop_counts(_iterations: i64, _frames: i64) {}

/// SDL's nanosecond clock: the app clock's ticks when no script runs.
///
/// The safe `sdl3` crate has no ns clock, so this is the raw call; pure read,
/// no arguments, no retained state.
pub fn platform_ticks_ns() -> u64 {
    // SAFETY: as above.
    unsafe { sdl3::sys::timer::SDL_GetTicksNS() }
}

/// Pushes one of the app's SDL user events (`UserCode` as `i32`, two data
/// words). Callable from any thread: SDL's timer callback and the web exports
/// call in through here; the words travel opaquely and are never dereferenced.
///
/// The safe `sdl3` crate's push needs an event-subsystem handle, which a
/// contextless timer callback has no way to reach, so this is the raw call:
/// the event is a plain data struct the queue copies on push.
pub fn platform_push_user(
    code: i32,
    data1: *mut std::ffi::c_void,
    data2: *mut std::ffi::c_void,
) -> bool {
    // SAFETY: as above.
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
/// loop's `wake` callback; returns SDL's timer id.
///
/// The safe `sdl3` crate has no timer by id, so this is the raw call. The
/// interval is counted from now (the arming's own time), which is what keeps
/// the app's share of a wake's lateness at 0.
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

/// Removes a wake timer by the id `platform_timer_arm` returned; removing a
/// spent id finds nothing.
///
/// The safe `sdl3` crate has no timer by id, so this is the raw call.
pub fn platform_timer_remove(id: u32) {
    // SAFETY: the id came from `platform_timer_arm`; removing it only stops
    // the callback from firing.
    unsafe {
        sdl3::sys::timer::SDL_RemoveTimer(sdl3::sys::timer::SDL_TimerID(id));
    }
}

/// The refresh rate of the display the window is on, in Hz; 0 when SDL
/// doesn't know it.
pub fn platform_refresh_rate(app: &mut App) -> f32 {
    if app.window.is_none() {
        // Headless (`headless_app`): no window, so no display, so SDL knows
        // no rate. The Odin code's `else 0` path: with its tests' nil window,
        // `GetDisplayForWindow` yields no mode and the rate is 0.
        return 0.0;
    }
    // SAFETY: `GetDisplayForWindow` takes the live window pointer; the mode
    // pointer is only read while SDL owns it, and the rate is copied out.
    unsafe {
        let id = sdl3::sys::video::SDL_GetDisplayForWindow(app.window().raw());
        let mode = sdl3::sys::video::SDL_GetCurrentDisplayMode(id);
        if mode.is_null() {
            return 0.0;
        }
        (*mode).refresh_rate
    }
}

/// Shows the cursor: hand images on macOS (where SDL has none and macOS none
/// of its own for help), else the system cursor map (SDL has no help cursor;
/// the pointing hand stands in; the move cursor stands in for both hands).
pub fn platform_set_cursor(cursor: Cursor) {
    CURSORS.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache[cursor].is_none() {
            #[cfg(target_os = "macos")]
            if let Some(hand) = crate::platform_darwin::darwin_cursor(cursor) {
                cache[cursor] = Some(hand);
            }
            if cache[cursor].is_none() {
                let system = match cursor {
                    Cursor::Default => sdl3::mouse::SystemCursor::Arrow,
                    Cursor::Pointer | Cursor::Help => sdl3::mouse::SystemCursor::Hand,
                    Cursor::Text => sdl3::mouse::SystemCursor::IBeam,
                    Cursor::Crosshair => sdl3::mouse::SystemCursor::Crosshair,
                    Cursor::Grab | Cursor::Grabbing => sdl3::mouse::SystemCursor::SizeAll,
                    Cursor::EwResize => sdl3::mouse::SystemCursor::SizeWE,
                };
                if let Ok(system) = sdl3::mouse::Cursor::from_system(system) {
                    cache[cursor] = Some(system);
                }
            }
        }
        if let Some(current) = &cache[cursor] {
            current.set();
        }
    });
}

/// The reduced-motion setting; only macOS has one.
pub fn platform_reduced_motion() -> bool {
    #[cfg(target_os = "macos")]
    return crate::platform_darwin::darwin_reduced_motion();
    #[cfg(not(target_os = "macos"))]
    return false;
}

/// Whether the system theme is dark.
///
/// The safe `sdl3` crate has no theme query, so this is the raw call: a pure
/// setting read with no arguments and no retained state.
pub fn platform_system_dark() -> bool {
    // SAFETY: as above.
    unsafe { sdl3::sys::video::SDL_GetSystemTheme() != sdl3::sys::video::SDL_SYSTEM_THEME_LIGHT }
}

/// No page listens on the native side: the field-edit flag is a web-shell
/// concern, compiled away here through the `platform_web`/`platform_native`
/// split.
pub fn platform_field_editing(_editing: bool) {}

/// Where the prefs live: `prefs.json` in `dir` when one is given (`--prefs`),
/// else in SDL's place for the port's own preferences: `…/mathviz/`
/// `function-viewer-rust/prefs.json` (the Rust app's own path, never the Odin
/// app's).
pub fn platform_prefs_store(dir: &str) -> String {
    if !dir.is_empty() {
        if let Err(err) = std::fs::create_dir_all(dir) {
            eprintln!("function-viewer-rust: could not make {dir}: {err}");
        }
        let dir = dir.trim_end_matches('/');
        return format!("{dir}/prefs.json");
    }
    match sdl3::filesystem::get_pref_path("mathviz", "function-viewer-rust") {
        Ok(base) => base.join("prefs.json").to_string_lossy().into_owned(),
        Err(err) => {
            eprintln!("function-viewer-rust: SDL_GetPrefPath: {err}");
            String::new()
        }
    }
}

/// Reads the whole prefs file; empty and not found when there is none.
pub fn platform_load_prefs(app: &mut App) -> (String, bool) {
    if app.prefs_store.is_empty() {
        return (String::new(), false);
    }
    match std::fs::read(&app.prefs_store) {
        Ok(data) => (String::from_utf8_lossy(&data).into_owned(), true),
        Err(_) => (String::new(), false),
    }
}

/// Writes the prefs file.
pub fn platform_save_prefs(app: &mut App, text: &str) -> bool {
    if app.prefs_store.is_empty() {
        return false;
    }
    if let Err(err) = std::fs::write(&app.prefs_store, text) {
        eprintln!(
            "function-viewer-rust: could not write {}: {err}",
            app.prefs_store
        );
        return false;
    }
    true
}

/// Reads a whole file (the harness's script and shots).
pub fn platform_read_file(path: &str) -> (Vec<u8>, bool) {
    match std::fs::read(path) {
        Ok(data) => (data, true),
        Err(err) => {
            eprintln!("function-viewer-rust: could not read {path}: {err}");
            (Vec::new(), false)
        }
    }
}

/// Makes a directory and its parents (the harness's shot directory).
pub fn platform_make_dir(path: &str) -> bool {
    if let Err(err) = std::fs::create_dir_all(path) {
        eprintln!("function-viewer-rust: could not make {path}: {err}");
        return false;
    }
    true
}

/// Writes a whole file (the harness's JSON and the PNG).
pub fn platform_write_file(path: &str, text: &str) -> bool {
    if let Err(err) = std::fs::write(path, text) {
        eprintln!("function-viewer-rust: could not write {path}: {err}");
        return false;
    }
    true
}

/// Writes `rgba`, rows bottom to top as `glReadPixels` gives them, to a PNG
/// at `path`. A frame drawn in Display P3 carries the space's ICC profile, so
/// Preview and the browsers show its bytes as the colours they are (a PNG
/// without one is read as sRGB); an sRGB frame's PNG has none, as before.
///
/// The PNG itself comes from the `png` crate; the `iCCP` chunk is written by
/// hand after the header, exactly as the Odin side writes it (the crate gives
/// no way to name the profile, and it would write `_` where the Odin file has
/// `Display P3`).
pub fn platform_write_png(path: &str, size: [i32; 2], rgba: &[u8], space: mv_theme::Space) -> bool {
    let Some(png) = encode_png(size, rgba) else {
        eprintln!("function-viewer-rust: could not write {path}");
        return false;
    };
    let out = {
        #[cfg(target_os = "macos")]
        if space == mv_theme::Space::DisplayP3 {
            let profile = crate::platform_darwin::darwin_icc_profile(space);
            with_icc_profile(&png, "Display P3", &profile)
        } else {
            png
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = space;
            png
        }
    };
    if let Err(err) = std::fs::write(path, &out) {
        eprintln!("function-viewer-rust: could not write {path}: {err}");
        return false;
    }
    true
}

/// Encodes `rgba` (bottom row first) as an 8-bit RGBA PNG, top row first.
fn encode_png(size: [i32; 2], rgba: &[u8]) -> Option<Vec<u8>> {
    let (width, height) = (size[0].max(0) as usize, size[1].max(0) as usize);
    if rgba.len() != width * height * 4 {
        return None;
    }
    let row_len = width * 4;
    let mut flipped = vec![0u8; rgba.len()];
    for y in 0..height {
        flipped[y * row_len..(y + 1) * row_len]
            .copy_from_slice(&rgba[(height - 1 - y) * row_len..(height - y) * row_len]);
    }
    let mut png = Vec::new();
    let mut encoder = png::Encoder::new(&mut png, width as u32, height as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    // No filtering: the default adaptive filter tries every filter per row,
    // which takes ~a second a shot in a debug build and burns the loop
    // check's idle sample window (the shot's encode lands in it). The filter
    // changes only compression, never pixels, and these PNGs are harness
    // artifacts decoded back to pixels.
    encoder.set_filter(png::Filter::NoFilter);
    encoder
        .write_header()
        .ok()?
        .write_image_data(&flipped)
        .ok()?;
    Some(png)
}

/// `png` with an `iCCP` chunk for `profile` after its header: the profile's
/// name, a zero, compression method 0, and the profile as a zlib stream, here
/// in stored blocks (zlib's header, blocks of at most 65535 bytes, the data's
/// Adler-32), which every decoder reads.
///
/// Ported from `platform_native.odin`'s `with_icc_profile` (PNG §11.3.3.3):
/// same layout, same order, same arithmetic.
fn with_icc_profile(png: &[u8], name: &str, profile: &[u8]) -> Vec<u8> {
    /// The signature, then IHDR's length, type, data and CRC.
    const IHDR_END: usize = 8 + 4 + 4 + 13 + 4;
    if png.len() < IHDR_END || profile.is_empty() {
        return png.to_vec();
    }
    fn be32(out: &mut Vec<u8>, v: u32) {
        out.push((v >> 24) as u8);
        out.push((v >> 16) as u8);
        out.push((v >> 8) as u8);
        out.push(v as u8);
    }
    let mut chunk = Vec::new();
    chunk.extend_from_slice(b"iCCP");
    chunk.extend_from_slice(name.as_bytes());
    chunk.push(0);
    chunk.push(0); // the name's end; compression method 0, zlib
    chunk.extend_from_slice(&[0x78, 0x01]); // deflate with a 32 KB window, no dictionary
    let mut rest = profile;
    loop {
        let n = rest.len().min(65535) as u16;
        let last = rest.len() == usize::from(n);
        chunk.push(u8::from(last));
        chunk.push(n as u8);
        chunk.push((n >> 8) as u8);
        chunk.push(!n as u8);
        chunk.push((!n >> 8) as u8);
        chunk.extend_from_slice(&rest[..usize::from(n)]);
        rest = &rest[usize::from(n)..];
        if last {
            break;
        }
    }
    be32(&mut chunk, adler32(profile));
    let mut out = Vec::with_capacity(png.len() + chunk.len() + 8);
    out.extend_from_slice(&png[..IHDR_END]);
    be32(&mut out, (chunk.len() - 4) as u32);
    out.extend_from_slice(&chunk);
    be32(&mut out, crc32(&chunk));
    out.extend_from_slice(&png[IHDR_END..]);
    out
}

/// Adler-32, as `hash.adler32` computes it.
fn adler32(data: &[u8]) -> u32 {
    const MOD: u32 = 65521;
    let mut s1: u32 = 1;
    let mut s2: u32 = 0;
    for &byte in data {
        s1 = (s1 + u32::from(byte)) % MOD;
        s2 = (s2 + s1) % MOD;
    }
    (s2 << 16) | s1
}

/// CRC-32 (ISO HDLC), as `hash.crc32` computes it.
fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xffff_ffff;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The default prefs path lives under the port's own app name.
    #[test]
    fn default_prefs_path_is_the_ports_own() {
        let store = platform_prefs_store("");
        eprintln!("default prefs store: {store}");
        assert!(
            store.ends_with("function-viewer-rust/prefs.json"),
            "default prefs store {store:?} ends in function-viewer-rust/prefs.json"
        );
        assert!(
            !store.contains("function-viewer/prefs.json")
                || store.contains("function-viewer-rust/prefs.json"),
            "default prefs store {store:?} is not the Odin app's path"
        );
    }

    /// An explicit dir is created and the store sits inside it.
    #[test]
    fn explicit_prefs_dir_is_created() {
        let dir = std::env::temp_dir().join(format!(
            "function-viewer-rust-platform-test-dir-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let store = platform_prefs_store(dir.to_str().unwrap());
        assert_eq!(
            store,
            format!("{}/prefs.json", dir.to_str().unwrap()),
            "explicit store is dir/prefs.json"
        );
        assert!(dir.is_dir(), "explicit dir is created");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The prefs JSON round-trips through the file layer exactly as
    /// `platform_save_prefs`/`platform_load_prefs` move it.
    #[test]
    fn prefs_file_round_trip() {
        let dir = std::env::temp_dir().join(format!(
            "function-viewer-rust-platform-test-file-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let store = platform_prefs_store(dir.to_str().unwrap());
        for prefs in [
            mv_theme::prefs::DEFAULT_PREFS,
            mv_theme::Prefs {
                mode: mv_theme::Mode::Dark,
                dark: mv_theme::Slug::TronCyan,
                light: mv_theme::Slug::SageEditorial,
            },
        ] {
            let text = mv_theme::prefs::prefs_json(prefs);
            assert!(platform_write_file(&store, &text));
            let (data, ok) = platform_read_file(&store);
            assert!(ok);
            assert_eq!(
                mv_theme::prefs::parse_prefs(std::str::from_utf8(&data).unwrap()),
                prefs,
                "prefs {text:?} round-trip"
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The `iCCP` chunk parses: `iCCP` right after IHDR, the profile's name,
    /// method 0, a stored-block zlib stream whose bytes and Adler-32 are the
    /// profile's, and a CRC over the chunk.
    #[test]
    fn iccp_chunk_parses() {
        let profile = vec![0x11u8; 70000];
        let png = encode_png([2, 1], &[9; 8]).unwrap();
        let out = with_icc_profile(&png, "Display P3", &profile);
        // Right after IHDR's chunk (8 signature + 25 IHDR bytes).
        let at = 8 + 25;
        let len = u32::from_be_bytes(out[at..at + 4].try_into().unwrap()) as usize;
        assert_eq!(&out[at + 4..at + 8], b"iCCP");
        let mut body = &out[at + 8..at + 8 + len];
        assert_eq!(&body[..11], b"Display P3\0");
        assert_eq!(body[11], 0, "compression method 0");
        body = &body[12..];
        assert_eq!(&body[..2], &[0x78, 0x01], "zlib header");
        // Stored blocks back to the profile.
        let mut blocks = &body[2..];
        let mut raw = Vec::new();
        loop {
            let last = blocks[0] == 1;
            let n = u16::from_le_bytes(blocks[1..3].try_into().unwrap());
            let check = u16::from_le_bytes(blocks[3..5].try_into().unwrap());
            assert_eq!(check, !n, "stored block length complement");
            let n = usize::from(n);
            raw.extend_from_slice(&blocks[5..5 + n]);
            blocks = &blocks[5 + n..];
            if last {
                break;
            }
        }
        assert_eq!(raw, profile);
        assert_eq!(
            u32::from_be_bytes(blocks[..4].try_into().unwrap()),
            adler32(&profile),
            "Adler-32 of the profile"
        );
        assert_eq!(
            u32::from_be_bytes(out[at + 8 + len..at + 12 + len].try_into().unwrap()),
            crc32(&out[at + 4..at + 8 + len]),
            "CRC over type and data"
        );
        // The rest of the file is untouched.
        assert_eq!(&out[at + 12 + len..], &png[8 + 25..]);
    }

    /// A P3 PNG written to disk carries the Display P3 profile and still
    /// decodes; an sRGB one carries no `iCCP`.
    #[test]
    #[cfg(target_os = "macos")]
    fn written_p3_png_carries_the_profile() {
        let profile = crate::platform_darwin::darwin_icc_profile(mv_theme::Space::DisplayP3);
        eprintln!("Display P3 profile: {} bytes", profile.len());
        assert!(!profile.is_empty(), "Display P3 profile is non-empty");
        let dir = std::env::temp_dir();
        let p3 = dir.join(format!("fv-platform-p3-{}.png", std::process::id()));
        let srgb = dir.join(format!("fv-platform-srgb-{}.png", std::process::id()));
        let rgba = [200u8; 4 * 2 * 4];
        assert!(platform_write_png(
            p3.to_str().unwrap(),
            [4, 2],
            &rgba,
            mv_theme::Space::DisplayP3
        ));
        assert!(platform_write_png(
            srgb.to_str().unwrap(),
            [4, 2],
            &rgba,
            mv_theme::Space::Srgb
        ));
        let p3_bytes = std::fs::read(&p3).unwrap();
        let srgb_bytes = std::fs::read(&srgb).unwrap();
        assert!(
            p3_bytes.windows(4).any(|w| w == b"iCCP"),
            "P3 PNG has an iCCP chunk"
        );
        assert!(
            !srgb_bytes.windows(4).any(|w| w == b"iCCP"),
            "sRGB PNG has no iCCP chunk"
        );
        // The decoder still reads the P3 file, with the profile attached.
        let decoded = png::Decoder::new(std::io::Cursor::new(&p3_bytes[..]))
            .read_info()
            .unwrap();
        assert_eq!(decoded.info().width, 4);
        assert_eq!(decoded.info().height, 2);
        assert_eq!(
            decoded.info().icc_profile.as_deref(),
            Some(profile.as_slice()),
            "decoded profile is the Display P3 one"
        );
        // A fixed copy for the run's evidence (parsed by a scratch script
        // into /opt/dev/tmp/rust-port/app-platform/).
        if let Ok(dir) = std::env::var("FV_PNG_EVIDENCE_DIR") {
            let _ = std::fs::write(format!("{dir}/platform-p3.png"), &p3_bytes);
        }
        let _ = std::fs::remove_file(&p3);
        let _ = std::fs::remove_file(&srgb);
    }
}
