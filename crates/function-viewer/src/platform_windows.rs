//! Console handling on Windows: close and Ctrl+C/Break become SDL quit.
//!
//! The console's handler, which Windows runs on a thread of its own, posts
//! `SDL_EVENT_QUIT`, waking the app's wait on events. This module never
//! compiles on this machine (it is `#[cfg(windows)]`, and there is no Windows
//! toolchain here), so it is written against the documented signatures and
//! marked unverified, as the Odin file it ports is.
//!
//! Ported from `src/platform_windows.odin` (34 lines): the handler names and
//! the three events it answers are the spec; the `kernel32` declarations below
//! spell the `core:sys/windows` calls it uses.

/// Installs the console handler and tells SDL not to handle signals.
///
/// Before SDL's init, which then installs no handler of its own.
pub fn platform_signals() {
    // SAFETY: `SetConsoleCtrlHandler` with a valid function pointer and
    // `add = TRUE` registers the handler; it stays registered for the life
    // of the process. `BOOL` is a 4-byte `i32` (windows-sys spells it so;
    // Rust's 1-byte `bool` would be the wrong width at this boundary).
    let added = unsafe { SetConsoleCtrlHandler(Some(on_console_ctrl), 1) };
    if added == 0 {
        eprintln!(
            "function-viewer-rust: signals: no console handler; Ctrl-C waits for the window's next event"
        );
        return;
    }
    sdl3::hint::set("SDL_NO_SIGNAL_HANDLERS", "1");
}

/// The console handler: quit on Ctrl+C, Ctrl+Break and Close.
///
/// `kind` is a `DWORD`, the return a `BOOL`: 1 takes the event, 0 passes it on.
unsafe extern "system" fn on_console_ctrl(kind: u32) -> i32 {
    const CTRL_C_EVENT: u32 = 0;
    const CTRL_BREAK_EVENT: u32 = 1;
    const CTRL_CLOSE_EVENT: u32 = 2;
    match kind {
        CTRL_C_EVENT | CTRL_BREAK_EVENT | CTRL_CLOSE_EVENT => {
            push_quit_event();
            1
        }
        _ => 0,
    }
}

/// Pushes SDL's quit event, waking the wait on events.
///
/// Uses the raw API: this runs on Windows' handler thread with no subsystem
/// handle, and `SDL_PushEvent` is thread-safe by SDL's contract.
fn push_quit_event() {
    // SAFETY: a zeroed event with only the type set is what SDL documents
    // for a quit event; pushing it wakes the main loop's event wait.
    unsafe {
        let mut event: sdl3::sys::events::SDL_Event = std::mem::zeroed();
        event.type_ = sdl3::sys::events::SDL_EventType::QUIT.0;
        sdl3::sys::events::SDL_PushEvent(&mut event);
    }
}

#[link(name = "kernel32")]
unsafe extern "system" {
    /// Registers (`add = true`) or removes a console control handler.
    ///
    /// Mirrors `core:sys/windows.SetConsoleCtrlHandler`: the handler runs on
    /// a thread of Windows' own whenever the console closes or Ctrl+C or
    /// Ctrl+Break arrives.
    fn SetConsoleCtrlHandler(
        handler: Option<unsafe extern "system" fn(kind: u32) -> i32>,
        add: i32,
    ) -> i32;
}
