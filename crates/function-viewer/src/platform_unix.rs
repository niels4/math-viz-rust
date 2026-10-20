//! Interrupt handling on Unix: SIGINT/SIGTERM become SDL quit events.
//!
//! A self-pipe carries the signal number out of the handler; a reader thread
//! pushes SDL quit for each byte. Only the handler's `write` runs on the
//! signal thread; everything else runs on ordinary threads. The setup runs
//! before SDL's own init (from [`crate::platform_native::platform_hints`]),
//! so SDL installs no handler of its own.

use std::cell::RefCell;
use std::sync::atomic::{AtomicI32, Ordering};

thread_local! {
    /// The self-pipe's fds, on the main thread.
    static QUIT_PIPE: RefCell<[std::os::raw::c_int; 2]> = const { RefCell::new([-1, -1]) };
}

/// The write end, visible to the signal handler (a thread-local is not: a
/// signal may land on any thread).
static QUIT_WRITE_FD: AtomicI32 = AtomicI32::new(-1);

/// Opens the pipe, starts the reader thread, takes the two signals, and tells
/// SDL not to handle signals itself.
///
/// Without a pipe or a thread SDL's own handling stays, and Ctrl-C waits for
/// the window's next event as before.
pub fn platform_signals() {
    let mut pipe = [-1; 2];
    // SAFETY: `pipe` points at two writable ints; on success both are open fds.
    if unsafe { libc::pipe(pipe.as_mut_ptr()) } != 0 {
        eprintln!(
            "function-viewer-rust: signals: no pipe; Ctrl-C waits for the window's next event"
        );
        return;
    }
    QUIT_PIPE.with(|slot| *slot.borrow_mut() = pipe);
    QUIT_WRITE_FD.store(pipe[1], Ordering::Release);
    let read_fd = pipe[0];
    if std::thread::Builder::new()
        .name("quit-pipe".to_string())
        .spawn(move || read_quit_pipe(read_fd))
        .is_err()
    {
        eprintln!(
            "function-viewer-rust: signals: no thread; Ctrl-C waits for the window's next event"
        );
        // SAFETY: both fds are open (the pipe call above succeeded).
        unsafe {
            libc::close(pipe[0]);
            libc::close(pipe[1]);
        }
        QUIT_PIPE.with(|slot| *slot.borrow_mut() = [-1, -1]);
        QUIT_WRITE_FD.store(-1, Ordering::Release);
        return;
    }
    // SAFETY: `action` is fully initialised below; the handler only writes
    // one byte to the pipe, which is async-signal-safe.
    unsafe {
        let mut action: libc::sigaction = std::mem::zeroed();
        action.sa_sigaction = on_quit_signal as *const () as usize;
        action.sa_flags = libc::SA_RESTART;
        libc::sigemptyset(&mut action.sa_mask);
        for signal in [libc::SIGINT, libc::SIGTERM] {
            if libc::sigaction(signal, &action, std::ptr::null_mut()) != 0 {
                eprintln!("function-viewer-rust: signals: {signal} not taken");
            }
        }
    }
    sdl3::hint::set("SDL_NO_SIGNAL_HANDLERS", "1");
}

/// The signal handler: writes the signal byte into the pipe.
///
/// Only async-signal-safe calls here (`write` to a pipe).
unsafe extern "C" fn on_quit_signal(signal: std::os::raw::c_int) {
    let fd = QUIT_WRITE_FD.load(Ordering::Acquire);
    if fd >= 0 {
        let byte = signal as u8;
        // SAFETY: `fd` is the pipe's open write end; a one-byte write is atomic.
        unsafe {
            libc::write(fd, (&raw const byte).cast::<std::ffi::c_void>(), 1);
        }
    }
}

/// Reads the pipe forever, pushing SDL quit per byte.
///
/// `read_fd` is the pipe's read end, owned by this thread.
fn read_quit_pipe(read_fd: std::os::raw::c_int) {
    loop {
        let mut byte = 0u8;
        // SAFETY: `read_fd` is the open read end; `byte` is a writable byte.
        let n = unsafe { libc::read(read_fd, (&raw mut byte).cast::<std::ffi::c_void>(), 1) };
        match n {
            1 => {
                // The signal's name, as the Odin app logs it (`quit: SIGINT`):
                // the loop check reads the name, not the number.
                let name = match byte as std::os::raw::c_int {
                    libc::SIGINT => "SIGINT",
                    libc::SIGTERM => "SIGTERM",
                    _ => "signal",
                };
                eprintln!("function-viewer-rust: quit: {name}");
                push_quit_event();
            }
            // The write end stays open for the life of the process, so 0
            // (all writers closed) and errors other than an interrupted read
            // mean the pipe is gone: stop the thread either way.
            _ => return,
        }
    }
}

/// Pushes SDL's quit event, waking the wait on events.
///
/// Uses the raw API: this runs on the reader thread with no subsystem handle,
/// and `SDL_PushEvent` is thread-safe by SDL's contract.
fn push_quit_event() {
    // SAFETY: a zeroed event with only the type set is what SDL documents
    // for a quit event; pushing it wakes the main loop's event wait.
    unsafe {
        let mut event: sdl3::sys::events::SDL_Event = std::mem::zeroed();
        event.r#type = sdl3::sys::events::SDL_EventType::QUIT.0;
        sdl3::sys::events::SDL_PushEvent(&mut event);
    }
}
