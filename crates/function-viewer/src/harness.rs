//! The scripted harness: plays `.act` scripts against the app on a 16 ms grid.
//!
//! Native only. Each verb becomes timestamped input facts applied in the frame
//! at their time; shots read the framebuffer back to PNGs and record a
//! `views.json` line beside them.

use enum_map::EnumMap;

use crate::app::App;
use crate::app_loop::MS;
use crate::input::{
    ButtonEvent, Input, KeyEvent, ModelEvent, PinchEvent, PointerEvent, ReducedMotionEvent,
    SchemeEvent, TextEvent, WheelEvent, WindowEvent,
};
use crate::live::{LiveResize, LiveScheme, LiveShot, LiveWhat, live_add};

/// One frame of the grid.
pub const FRAME_MS: i64 = 16;
/// The double-click window, in ms.
pub const DOUBLE_CLICK_MS: i64 = 500;
/// The double-click radius, in px.
pub const DOUBLE_CLICK_PX: f32 = 32.0;

/// Each named key's keycode.
pub static NAMED_KEYCODES: EnumMap<mv_script::NamedKey, sdl3::keyboard::Keycode> =
    EnumMap::from_array([
        sdl3::keyboard::Keycode::Up,
        sdl3::keyboard::Keycode::Down,
        sdl3::keyboard::Keycode::Left,
        sdl3::keyboard::Keycode::Right,
        sdl3::keyboard::Keycode::Return,
        sdl3::keyboard::Keycode::Escape,
        sdl3::keyboard::Keycode::Tab,
        sdl3::keyboard::Keycode::Backspace,
        sdl3::keyboard::Keycode::Delete,
        sdl3::keyboard::Keycode::Home,
        sdl3::keyboard::Keycode::End,
        sdl3::keyboard::Keycode::PageUp,
        sdl3::keyboard::Keycode::PageDown,
        sdl3::keyboard::Keycode::Space,
        sdl3::keyboard::Keycode::LShift,
        sdl3::keyboard::Keycode::LCtrl,
        sdl3::keyboard::Keycode::LAlt,
        sdl3::keyboard::Keycode::LGui,
    ]);

/// Each modifier's named key.
pub static MODIFIER_KEYS: EnumMap<mv_script::Modifier, mv_script::NamedKey> =
    EnumMap::from_array([
        mv_script::NamedKey::Shift,
        mv_script::NamedKey::Control,
        mv_script::NamedKey::Alt,
        mv_script::NamedKey::Meta,
    ]);

/// Each modifier's keymod flag.
pub static MODIFIER_MODS: EnumMap<mv_script::Modifier, sdl3::keyboard::Mod> =
    EnumMap::from_array([
        sdl3::keyboard::Mod::LSHIFTMOD,
        sdl3::keyboard::Mod::LCTRLMOD,
        sdl3::keyboard::Mod::LALTMOD,
        sdl3::keyboard::Mod::LGUIMOD,
    ]);

/// One fact with when it happened.
#[derive(Clone, Debug)]
pub struct Stamped {
    pub input: Input,
    pub time: crate::app_loop::Time,
}

/// A press tracked for double clicks.
#[derive(Clone, Copy, Debug)]
pub struct HarnessPress {
    pub time: i64,
    pub pos: [f32; 2],
    pub clicks: i64,
}

/// The harness run: the app, the framebuffer, the pending facts, and what the
/// shots record.
pub struct Harness<'a> {
    pub app: &'a mut App,
    /// The directory shots go to.
    pub out: String,
    /// Frames go to the visible window, not to `fb`.
    pub window: bool,
    /// The script goes through SDL's own loop: steps, not frames.
    pub live: Option<&'a mut super::live::Live>,
    /// `None` in windowed mode, which draws to the window instead. Rust
    /// structs have no zero value, so the framebuffer is optional rather
    /// than an empty struct as in the spec.
    pub fb: Option<mv_nvg::Framebuffer>,
    pub density: f32,
    /// Ms since the first frame.
    pub time: i64,
    /// This moment's facts, with when they happened, for its frame.
    pub pending: Vec<Stamped>,
    pub readback: Vec<u8>,
    /// Where the pointer starts.
    pub pointer: [f32; 2],
    /// The modifier keys held.
    pub mods: sdl3::keyboard::Mod,
    pub press: HarnessPress,
    /// Video sections, which record nothing natively.
    pub videos: i64,
    /// The plane's view at each shot, as JSON lines.
    pub views: Vec<u8>,
    /// The shots taken mid-motion.
    pub moving: Vec<String>,
    /// The page query as last written to `views.json`.
    pub url: String,
    /// The camera as last written to `views.json`.
    pub camera: String,
}

/// Launches motion, sizes the framebuffer or window, draws the first frame,
/// plays the script, and writes the views and times.
pub fn run_harness(
    app: &mut App,
    run: &mv_script::Script,
    out: &str,
    window: bool,
    time: i64,
) -> bool {
    let mut h = Harness {
        app,
        out: out.to_string(),
        window,
        live: None,
        fb: None,
        density: run.start.density,
        time: 0,
        pending: Vec::new(),
        readback: Vec::new(),
        pointer: [0.0, 0.0],
        mods: sdl3::keyboard::Mod::empty(),
        press: HarnessPress {
            time: -DOUBLE_CLICK_MS,
            pos: [0.0, 0.0],
            clicks: 0,
        },
        videos: 0,
        views: Vec::new(),
        moving: Vec::new(),
        url: String::new(),
        camera: String::new(),
    };
    if !crate::platform_native::platform_make_dir(out) {
        return false;
    }
    h.app.r#loop.script_time = Some(0);
    // The view appeared a frame before its first frame, as the web build loads
    // under Playwright's paused clock (its init at 0, its first animation
    // frame at 16 ms), where React's view mounts a frame before the script's
    // time 0: its first paint is 16 ms in at the first frame.
    let size = [run.start.size[0] as f32, run.start.size[1] as f32];
    let mut pixels = device_pixels(size, h.density);
    if window {
        if h.app.window().pixel_density() != h.density {
            eprintln!(
                "[ERROR] --- [{}:{}] the window's pixel density is {} and the script's {}",
                file!(),
                line!(),
                h.app.window().pixel_density(),
                h.density
            );
            return false;
        }
        pixels = crate::input::window_event(h.app.window()).pixels;
        // Frames follow the script's clock, not the display's; and with no
        // display (a background login) a swap that waits for vsync never
        // returns.
        //
        // SAFETY: `SDL_GL_SetSwapInterval` touches only global GL state and
        // runs on the main thread before any frame.
        unsafe {
            sdl3::sys::video::SDL_GL_SetSwapInterval(0);
        }
    } else {
        h.fb = Some(mv_nvg::Framebuffer::create(
            &mut h.app.vg,
            pixels[0],
            pixels[1],
            0,
        ));
    }
    crate::input::handle_input(h.app, Input::Window(WindowEvent { size, pixels }), 0);
    harness_frame(&mut h); // the first frame, at 0 ms

    if !play(&mut h, run) {
        return false;
    }
    if !h.views.is_empty()
        && !crate::platform_native::platform_write_file(
            &format!("{}/views.json", h.out),
            std::str::from_utf8(&h.views).unwrap_or(""),
        )
    {
        eprintln!(
            "[ERROR] --- [{}:{}] could not write {}/views.json",
            file!(),
            line!(),
            h.out
        );
        return false;
    }
    if time > 0 && !redraw_times(&mut h, time) {
        return false;
    }
    if h.videos > 0 {
        println!(
            "{} video section(s): natively the harness records none; judge motion by frames",
            h.videos
        );
    }
    write_moving(&mut h)
}

/// Plays the script's actions in turn.
pub fn play(h: &mut Harness<'_>, run: &mv_script::Script) -> bool {
    for action in &run.actions {
        // The reference's own lines: the port reaches their state by its `port` lines.
        if action.surface == mv_script::Surface::Ref {
            continue;
        }
        if !act(h, &action.verb) {
            eprintln!(
                "[ERROR] --- [{}:{}] the script stopped at its line {}",
                file!(),
                line!(),
                action.line
            );
            return false;
        }
        inputs_now(h);
    }
    true
}

/// Draws a frame when non-live facts are pending.
fn inputs_now(h: &mut Harness<'_>) {
    if h.live.is_none() && !h.pending.is_empty() {
        harness_frame(h);
    }
}

/// Queues the facts for one verb: moves, presses, keys, waits, shots, and the
/// rest.
fn act(h: &mut Harness<'_>, verb: &mv_script::Verb) -> bool {
    match verb {
        mv_script::Verb::Move(m) => r#move(h, m),
        mv_script::Verb::Press(p) => press(h, p.down),
        mv_script::Verb::Click(c) => {
            point(h, c.at);
            for _ in 0..c.count {
                press(h, true);
                press(h, false);
            }
        }
        mv_script::Verb::Wheel(w) => {
            // The script's deltas are the page's px, as the app's wheel facts are.
            if w.ctrl {
                hold(
                    h,
                    &mv_script::KeyName::Named(mv_script::NamedKey::Control),
                    true,
                );
            }
            queue(
                h,
                Input::Wheel(WheelEvent {
                    delta: w.delta,
                    pos: h.pointer,
                }),
            );
            if w.ctrl {
                hold(
                    h,
                    &mv_script::KeyName::Named(mv_script::NamedKey::Control),
                    false,
                );
            }
        }
        mv_script::Verb::Pinch(p) => {
            // In a browser the harness pinches as a trackpad does, by a wheel
            // event with Ctrl held, and the app sees Ctrl go down and up
            // around it, so it does here too.
            hold(
                h,
                &mv_script::KeyName::Named(mv_script::NamedKey::Control),
                true,
            );
            queue(h, Input::Pinch(PinchEvent { scale: p.scale }));
            hold(
                h,
                &mv_script::KeyName::Named(mv_script::NamedKey::Control),
                false,
            );
        }
        mv_script::Verb::Key(k) => {
            // As Playwright presses "Shift+ArrowUp": the modifiers down, the
            // key down and up, the modifiers up in reverse.
            let mut held = [mv_script::Modifier::Shift; 4];
            let mut count = 0usize;
            for modifier in k.mods {
                hold(h, &mv_script::KeyName::Named(MODIFIER_KEYS[modifier]), true);
                held[count] = modifier;
                count += 1;
            }
            tap(h, &k.key, "");
            for modifier in held[..count].iter().rev() {
                hold(
                    h,
                    &mv_script::KeyName::Named(MODIFIER_KEYS[*modifier]),
                    false,
                );
            }
        }
        mv_script::Verb::Hold(v) => hold(h, &v.key, v.down),
        mv_script::Verb::Type(t) => {
            for c in t.text.chars() {
                let mut buf = [0u8; 4];
                let typed = c.encode_utf8(&mut buf);
                tap(h, &mv_script::KeyName::Char(c), typed);
            }
        }
        mv_script::Verb::Wait(w) => advance(h, w.ms),
        mv_script::Verb::Shot(s) => {
            if !shot(h, &s.name) {
                return false;
            }
        }
        mv_script::Verb::Frames(f) => {
            let mut t = 0i64;
            loop {
                if !shot(h, &format!("{}-{:04}", f.name, t)) {
                    return false;
                }
                if t + f.every > f.span {
                    break;
                }
                advance(h, f.every);
                t += f.every;
            }
        }
        mv_script::Verb::Video(v) => {
            if v.start {
                h.videos += 1;
            }
        }
        mv_script::Verb::Resize(r) => {
            if !resize_window(h, r.size) {
                return false;
            }
        }
        mv_script::Verb::SchemeChange(s) => {
            if let Some(live) = h.live.as_mut() {
                let at = h.time;
                live_add(
                    live,
                    at,
                    LiveWhat::Scheme(LiveScheme {
                        dark: s.scheme == mv_theme::Scheme::Dark,
                    }),
                );
            } else {
                queue(
                    h,
                    Input::Scheme(SchemeEvent {
                        dark: s.scheme == mv_theme::Scheme::Dark,
                    }),
                );
            }
        }
        mv_script::Verb::MotionChange(m) => {
            queue(
                h,
                Input::ReducedMotion(ReducedMotionEvent { reduced: m.reduced }),
            );
        }
        mv_script::Verb::Model(m) => {
            queue(h, Input::Model(ModelEvent { change: m.clone() }));
        }
    }
    true
}

/// A fact SDL would deliver now: held for this moment's frame (inputs_now),
/// or a step of the live run.
fn queue(h: &mut Harness<'_>, input: Input) {
    if let Some(live) = h.live.as_mut() {
        let at = h.time;
        live_add(live, at, LiveWhat::Input(input));
        return;
    }
    h.pending.push(Stamped {
        input,
        time: h.time * MS,
    });
}

/// Time passes. Each frame on the grid takes the facts waiting and draws:
/// every 16 ms while the app is awake; while it's idle, only the first frame
/// after a fact or a deadline.
fn advance(h: &mut Harness<'_>, ms: i64) {
    let end = h.time + ms;
    if h.live.is_some() {
        h.time = end;
        return;
    }
    for next in grid_steps(h.time, ms) {
        let (awake, waiting, wake_at) = (
            h.app.r#loop.awake,
            !h.pending.is_empty(),
            h.app.r#loop.wake_at,
        );
        let due = wake_at != 0 && wake_at < next * MS;
        if awake || waiting || due {
            h.time = next;
            harness_frame(h);
        }
    }
    h.time = end;
}

/// The frame times `advance` visits going `ms` from `time`: the 16 ms grid
/// points strictly after `time` up to and including the end. The gating
/// (awake, pending facts, due deadlines) lives in `advance`; this is the
/// grid alone, so the paused-clock math is testable without the app.
fn grid_steps(time: i64, ms: i64) -> impl Iterator<Item = i64> {
    let end = time + ms;
    (time / FRAME_MS + 1..)
        .map(|k| k * FRAME_MS)
        .take_while(move |&n| n <= end)
}

/// One harness frame: sets the script time, handles the facts, binds the
/// framebuffer, and iterates (swapping when windowed).
fn harness_frame(h: &mut Harness<'_>) {
    h.app.r#loop.script_time = Some(h.time * MS);
    let facts = std::mem::take(&mut h.pending);
    for fact in facts {
        crate::input::handle_input(h.app, fact.input, fact.time);
    }
    if h.window {
        mv_nvg::Framebuffer::bind_default();
    } else if let Some(fb) = &h.fb {
        fb.bind();
    }
    if crate::app_loop::iterate(h.app) && h.window {
        h.app.window().gl_swap_window();
    }
    // The page query, whenever it changed: `--query` at start, then whatever
    // the page milestones route through `App.url`.
    if h.app.url.query() != h.url {
        h.url = h.app.url.query().to_string();
        h.views.extend_from_slice(url_line(&h.url).as_bytes());
    }
    // The camera, whenever it changed: center, scale, whether a frame is
    // scheduled (h2-input; the behaviour checks read these against the
    // reference's URL and canvas labels).
    let camera = camera_line(
        h.app.camera.center,
        h.app.camera.scale,
        h.app.camera.animating(),
    );
    if camera != h.camera {
        h.camera = camera;
        h.views.extend_from_slice(h.camera.as_bytes());
    }
}

/// The pointer to `at`. SDL reports no motion that moves nothing.
fn point(h: &mut Harness<'_>, at: [f32; 2]) {
    if at != h.pointer {
        h.pointer = at;
        queue(
            h,
            Input::Pointer(PointerEvent {
                pos: at,
                left: false,
            }),
        );
    }
}

/// A move straight to the target, with `over`: a step at each frame or so,
/// whole pixels between, the last on the target, time passing as it goes.
/// tools/src/act.py steps the same way.
fn r#move(h: &mut Harness<'_>, m: &mv_script::Move) {
    if m.over == 0 {
        point(h, m.to);
        return;
    }
    let steps = std::cmp::max(1, (m.over + FRAME_MS / 2) / FRAME_MS);
    let from = h.pointer;
    for k in 1..=steps {
        let mut at = m.to;
        if k < steps {
            for axis in 0..2 {
                let along = f64::from(from[axis])
                    + f64::from(m.to[axis] - from[axis]) * k as f64 / steps as f64;
                at[axis] = (along + 0.5).floor() as f32;
            }
        }
        point(h, at);
        inputs_now(h);
        advance(h, m.over * k / steps - m.over * (k - 1) / steps);
    }
}

fn press(h: &mut Harness<'_>, down: bool) {
    if down {
        let moved = [h.pointer[0] - h.press.pos[0], h.pointer[1] - h.press.pos[1]];
        let far = moved[0].abs() > DOUBLE_CLICK_PX || moved[1].abs() > DOUBLE_CLICK_PX;
        if h.time - h.press.time >= DOUBLE_CLICK_MS || far {
            h.press.clicks = 0;
        }
        h.press.time = h.time;
        h.press.pos = h.pointer;
        h.press.clicks += 1;
    }
    queue(
        h,
        Input::Button(ButtonEvent {
            // SDL's numbering: 1 is the primary button.
            button: sdl3::sys::mouse::SDL_BUTTON_LEFT as u8,
            down,
            clicks: h.press.clicks,
            pos: h.pointer,
        }),
    );
}

fn hold(h: &mut Harness<'_>, key: &mv_script::KeyName, down: bool) {
    if let Some(modifier) = mv_script::key_modifier(*key) {
        if down {
            h.mods |= MODIFIER_MODS[modifier];
        } else {
            h.mods &= !MODIFIER_MODS[modifier];
        }
    }
    queue(
        h,
        Input::Key(KeyEvent {
            key: keycode(key),
            mods: h.mods,
            down,
            repeat: false,
        }),
    );
}

/// A key pressed and released. A key that types a character gives text
/// between, as SDL does while text input is on and neither Ctrl nor Command
/// is held.
fn tap(h: &mut Harness<'_>, key: &mv_script::KeyName, typed: &str) {
    let mut encoded = [0u8; 4];
    let text = if typed.is_empty() {
        match *key {
            mv_script::KeyName::Char(c) => c.encode_utf8(&mut encoded),
            mv_script::KeyName::Named(mv_script::NamedKey::Space) => " ",
            mv_script::KeyName::Named(_) => "",
        }
    } else {
        typed
    };
    let code = keycode(key);
    queue(
        h,
        Input::Key(KeyEvent {
            key: code,
            mods: h.mods,
            down: true,
            repeat: false,
        }),
    );
    // SDL sends text only while text input is on: asked here as the key
    // arrives, or in a live run as the key is posted (live.rs).
    let ctrl_gui = sdl3::keyboard::Mod::LCTRLMOD
        | sdl3::keyboard::Mod::RCTRLMOD
        | sdl3::keyboard::Mod::LGUIMOD
        | sdl3::keyboard::Mod::RGUIMOD;
    if !text.is_empty() && (h.mods & ctrl_gui).is_empty() {
        // SAFETY: `SDL_TextInputActive` reads global SDL state through a raw
        // window pointer it never retains; the window outlives the call.
        let texting = h.live.is_some()
            || unsafe { sdl3::sys::keyboard::SDL_TextInputActive(h.app.window().raw()) };
        if texting {
            queue(
                h,
                Input::Text(TextEvent {
                    text: text.to_string(),
                }),
            );
        }
    }
    queue(
        h,
        Input::Key(KeyEvent {
            key: code,
            mods: h.mods,
            down: false,
            repeat: false,
        }),
    );
}

/// SDL's keycode for a key: a key that types one character is that character,
/// as SDL's Emscripten port reads the DOM's `key`.
pub fn keycode(key: &mv_script::KeyName) -> sdl3::keyboard::Keycode {
    match *key {
        mv_script::KeyName::Named(named) => NAMED_KEYCODES[named],
        mv_script::KeyName::Char(c) => {
            sdl3::keyboard::Keycode::from_u32(c as u32).unwrap_or(sdl3::keyboard::Keycode::Unknown)
        }
    }
}

/// The window resized: a new framebuffer at the new size, and the app told at
/// the next frame. A shot before that frame shows the empty framebuffer, as a
/// browser shows a resized canvas.
fn resize_window(h: &mut Harness<'_>, size: [i64; 2]) -> bool {
    if let Some(live) = h.live.as_mut() {
        let at = h.time;
        live_add(live, at, LiveWhat::Resize(LiveResize { size }));
        return true;
    }
    if h.window {
        eprintln!(
            "[ERROR] --- [{}:{}] size after the first action: --window keeps the window's size",
            file!(),
            line!()
        );
        return false;
    }
    let logical = [size[0] as f32, size[1] as f32];
    let pixels = device_pixels(logical, h.density);
    if let Some(fb) = h.fb.take() {
        fb.delete();
    }
    h.fb = Some(mv_nvg::Framebuffer::create(
        &mut h.app.vg,
        pixels[0],
        pixels[1],
        0,
    ));
    queue(
        h,
        Input::Window(WindowEvent {
            size: logical,
            pixels,
        }),
    );
    true
}

/// The cost of a full redraw: the state the script left drawn `count` more
/// times into the offscreen framebuffer, each from the layout to the GPU's
/// last pixel (glFinish), no swap and so no vsync; with --window into the
/// visible window instead, each presented by a swap at interval 0
/// (run_harness), so the frame's whole cost uncapped by a display. The
/// median, the 95th percentile and the slowest, logged and written to
/// <out>/time.json. The first redraw, which warms NanoVG's buffers and the
/// glyph atlas, is left out.
fn redraw_times(h: &mut Harness<'_>, count: i64) -> bool {
    if h.window {
        mv_nvg::Framebuffer::bind_default();
    } else if let Some(fb) = &h.fb {
        fb.bind();
    }
    let mut ms: Vec<f64> = Vec::with_capacity(count as usize);
    for i in -1..count {
        // SAFETY: `SDL_GetTicksNS` reads the global SDL clock; no pointers.
        let start = unsafe { sdl3::sys::timer::SDL_GetTicksNS() };
        h.app.dirty = true;
        if crate::app_loop::iterate(h.app) && h.window {
            h.app.window().gl_swap_window();
        }
        gl_finish(&h.app.vg);
        if i >= 0 {
            ms.push((unsafe { sdl3::sys::timer::SDL_GetTicksNS() } - start) as f64 / 1e6);
        }
    }
    ms.sort_by(|a, b| a.total_cmp(b));
    let median = ms[count as usize / 2];
    let p95 = ms[std::cmp::min(count as usize - 1, count as usize * 95 / 100)];
    let slowest = ms[count as usize - 1];
    let into = if h.window {
        "the window, each presented"
    } else {
        "the offscreen framebuffer"
    };
    println!(
        "redraw at {}x{} px into {}: {} redraws, median {:.2} ms, 95th percentile {:.2} ms, slowest {:.2} ms",
        h.app.pixels[0], h.app.pixels[1], into, count, median, p95, slowest
    );
    let report = time_json(
        [h.app.pixels[0], h.app.pixels[1]],
        h.window,
        count,
        median,
        p95,
        slowest,
    );
    if !crate::platform_native::platform_write_file(&format!("{}/time.json", h.out), &report) {
        eprintln!(
            "[ERROR] --- [{}:{}] could not write {}/time.json",
            file!(),
            line!(),
            h.out
        );
        return false;
    }
    true
}

/// The GPU's last pixel: `glFinish` through the NanoVG context.
///
/// Ported from `harness.odin:462` (`gl.Finish()`): through `mv-nvg`'s
/// `Context::finish`, which uses the loaded `glow` context when `set_gl`
/// ran and NanoVG's own `glFinish` helper otherwise.
fn gl_finish(vg: &mv_nvg::Context) {
    vg.finish();
}

/// The last frame drawn, read back and written to <out>/<name>.png.
fn shot(h: &mut Harness<'_>, name: &str) -> bool {
    if let Some(live) = h.live.as_mut() {
        let at = h.time;
        live_add(
            live,
            at,
            LiveWhat::Shot(LiveShot {
                name: name.to_string(),
            }),
        );
        return true;
    }
    if h.window {
        mv_nvg::Framebuffer::bind_default();
    } else if let Some(fb) = &h.fb {
        fb.bind();
    }
    let out = h.out.clone();
    if !write_shot(h.app, &mut h.readback, &out, name, h.window) {
        return false;
    }
    println!("shot {out}/{name}.png at {} ms", h.time);
    // The camera as drawn: the scale is the zoom, the center the pan (there
    // is no separate origin in this port). The cursor and the theme are
    // already real; no layer ever scales, so `scaled` stays empty until a
    // milestone lays one out.
    let line = views_line(
        name,
        h.time,
        h.app.camera.scale,
        h.app.camera.center[0],
        h.app.camera.center[1],
        0.0,
        0.0,
        crate::input::CURSOR_CSS[h.app.input.cursor],
        mv_theme::SLUG_WORDS[mv_theme::shown(h.app.prefs, h.app.system_scheme)],
        "",
    );
    h.views.extend_from_slice(line.as_bytes());
    true
}

/// One `views.json` line: the shot, the script ms, the view (zoom, pan,
/// origin), the cursor, the theme, the scaled boxes. Pure, so the shape the
/// harness writes is testable without the app.
#[allow(clippy::too_many_arguments)]
fn views_line(
    name: &str,
    time: i64,
    zoom: f64,
    pan_x: f64,
    pan_y: f64,
    origin_x: f64,
    origin_y: f64,
    cursor: &str,
    theme: &str,
    scaled: &str,
) -> String {
    format!(
        "{{\"shot\": {}, \"time\": {time}, \"zoom\": {}, \"pan_x\": {}, \"pan_y\": {}, \"origin_x\": {}, \"origin_y\": {}, \"cursor\": {}, \"theme\": {}, \"scaled\": [{scaled}]}}\n",
        json_string(name),
        fmt_g17(zoom),
        fmt_g17(pan_x),
        fmt_g17(pan_y),
        fmt_g17(origin_x),
        fmt_g17(origin_y),
        json_string(cursor),
        json_string(theme),
    )
}

/// One `url` line: the page query as the frame left it. Pure, like
/// `views_line` above, so the shape is testable without the app.
fn url_line(url: &str) -> String {
    format!("{{\"url\": {}}}\n", json_string(url))
}

/// One `camera` line: the plane's center and scale with whether a frame is
/// scheduled, as the frame left them. Pure, like `views_line` above, so the
/// shape is testable without the app.
fn camera_line(center: [f64; 2], scale: f64, animating: bool) -> String {
    format!(
        "{{\"camera\": {{\"center\": [{}, {}], \"scale\": {}, \"animating\": {animating}}}}}\n",
        fmt_g17(center[0]),
        fmt_g17(center[1]),
        fmt_g17(scale),
    )
}

/// The shots taken mid-motion, as <out>/motion.json when there are any. The
/// shell never moves, so this stays empty until a page milestone's motion
/// records into `Harness.moving` again.
fn write_moving(h: &mut Harness<'_>) -> bool {
    if h.moving.is_empty() {
        return true;
    }
    let report = motion_json(&h.moving);
    let path = format!("{}/motion.json", h.out);
    if !crate::platform_native::platform_write_file(&path, &report) {
        eprintln!(
            "[ERROR] --- [{}:{}] could not write {path}",
            file!(),
            line!()
        );
        return false;
    }
    true
}

/// `<out>/motion.json`: the shots taken mid-motion-fade. Pure, so the shape
/// is testable without the app.
fn motion_json(moving: &[String]) -> String {
    let mut names: Vec<String> = Vec::with_capacity(moving.len());
    for name in moving {
        names.push(json_string(name));
    }
    format!("{{\"moving\": [{}]}}\n", names.join(", "))
}

/// `<out>/time.json`: pixels, the window flag, the redraw count, the median,
/// p95 and slowest ms. Pure, so the shape is testable without the app.
fn time_json(
    pixels: [i32; 2],
    window: bool,
    redraws: i64,
    median_ms: f64,
    p95_ms: f64,
    slowest_ms: f64,
) -> String {
    format!(
        "{{\"pixels\": [{}, {}], \"window\": {window}, \"redraws\": {redraws}, \"median_ms\": {:.3}, \"p95_ms\": {:.3}, \"slowest_ms\": {:.3}}}\n",
        pixels[0], pixels[1], median_ms, p95_ms, slowest_ms
    )
}

/// A string as JSON quotes it. Shot names, cursors and theme slugs are
/// ASCII, where this and Go's `%q` agree.
pub(crate) fn json_string(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| format!("{s:?}"))
}

/// C `%.17g`: 17 significant digits, fixed while `-4 <= exp < 17`, exponential
/// otherwise, no trailing zeros. The spec's `%.17g` verbs need byte parity
/// with the C libraries' formatting (views.json lines compare equal), and the
/// spec's `%g` was verified against C's on 20 values through the Odin
/// compiler; Rust has no `%g`, so it is reimplemented here over the
/// correctly-rounded `{:.16e}`.
fn fmt_g17(v: f64) -> String {
    if v == 0.0 {
        return if v.is_sign_negative() {
            "-0".to_string()
        } else {
            "0".to_string()
        };
    }
    if v.is_nan() {
        return "NaN".to_string();
    }
    if v.is_infinite() {
        return if v.is_sign_negative() {
            "-inf".to_string()
        } else {
            "inf".to_string()
        };
    }
    let sign = if v.is_sign_negative() { "-" } else { "" };
    // Seventeen significant digits, correctly rounded.
    let exp = format!("{:.16e}", v.abs());
    let (mantissa, exponent) = exp.split_once('e').unwrap_or((exp.as_str(), "0"));
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    debug_assert_eq!(digits.len(), 17);
    let e: i32 = exponent.parse().unwrap_or(0);
    if (-4..17).contains(&e) {
        // Fixed: the point after the first e+1 digits, (17-1-e) behind it.
        // Below zero the point sits before the digits: "0." then zeros.
        if e >= 0 {
            let (head, tail) = digits.split_at(e as usize + 1);
            let tail = tail.trim_end_matches('0');
            if tail.is_empty() {
                return format!("{sign}{head}");
            }
            return format!("{sign}{head}.{tail}");
        }
        let mut frac = "0".repeat(-e as usize - 1);
        frac.push_str(&digits);
        let frac = frac.trim_end_matches('0');
        return format!("{sign}0.{frac}");
    }
    // Exponential: one digit, the point, the rest stripped, e±XX.
    let (head, tail) = digits.split_at(1);
    let tail = tail.trim_end_matches('0');
    let mantissa = if tail.is_empty() {
        head.to_string()
    } else {
        format!("{head}.{tail}")
    };
    let exponent = if e < 0 {
        format!("e-{:02}", e.abs())
    } else {
        format!("e+{e:02}")
    };
    format!("{sign}{mantissa}{exponent}")
}

/// The bound framebuffer, or with `front` the window's front buffer (what the
/// last swap presented), read back and written to <out>/<name>.png.
pub fn write_shot(
    app: &mut App,
    readback: &mut Vec<u8>,
    out: &str,
    name: &str,
    front: bool,
) -> bool {
    let pixels = app.pixels;
    readback.resize(pixels[0] as usize * pixels[1] as usize * 4, 0);
    read_back(app, readback, front);
    crate::platform_native::platform_write_png(
        &format!("{out}/{name}.png"),
        pixels,
        readback,
        app.space,
    )
}

/// The pixels of the bound framebuffer (or the window's front buffer) into
/// `readback`, rows bottom to top as `glReadPixels` gives them.
///
/// Ported from `harness.odin:585` (`write_shot`'s `gl.ReadPixels` leg): the
/// caller bound the target first (`shot` binds the default framebuffer or
/// the harness's `fb`); with `front` the window's front buffer is selected
/// first and restored to back afterwards, as the Odin code does.
fn read_back(app: &App, readback: &mut Vec<u8>, front: bool) {
    use glow::HasContext as _;
    let gl = app.gl.as_ref().expect("no GL: headless App");
    let [w, h] = app.pixels;
    assert_eq!(readback.len(), w as usize * h as usize * 4);
    // SAFETY: the context is current on this thread (made current at start),
    // and the read targets the caller-sized buffer, exactly `w * h * 4`
    // bytes (asserted above), as `glReadPixels` writes them.
    unsafe {
        if front {
            gl.read_buffer(glow::FRONT);
        }
        gl.pixel_store_i32(glow::PACK_ALIGNMENT, 1);
        gl.read_pixels(
            0,
            0,
            w,
            h,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            glow::PixelPackData::Slice(Some(readback.as_mut_slice())),
        );
        if front {
            gl.read_buffer(glow::BACK);
        }
    }
}

fn device_pixels(size: [f32; 2], density: f32) -> [i32; 2] {
    [
        (size[0] * density).round() as i32,
        (size[1] * density).round() as i32,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_ten_frames_in_160_ms() {
        let frames: Vec<i64> = grid_steps(0, 160).collect();
        assert_eq!(frames, vec![16, 32, 48, 64, 80, 96, 112, 128, 144, 160]);
    }

    #[test]
    fn grid_counts_against_its_edges() {
        assert_eq!(grid_steps(0, 2000).count(), 125);
        assert_eq!(grid_steps(0, 0).count(), 0);
        assert_eq!(grid_steps(0, 15).count(), 0);
        assert_eq!(grid_steps(0, 16).collect::<Vec<_>>(), vec![16]);
        assert_eq!(grid_steps(7, 10).collect::<Vec<_>>(), vec![16]);
        assert_eq!(grid_steps(16, 16).collect::<Vec<_>>(), vec![32]);
    }

    #[test]
    fn pixels_round_half_away() {
        assert_eq!(device_pixels([1440.0, 900.0], 2.0), [2880, 1800]);
        assert_eq!(device_pixels([100.0, 100.0], 1.5), [150, 150]);
        assert_eq!(device_pixels([1.0, 1.0], 2.5), [3, 3]);
    }

    #[test]
    // The table's literals are the ground truth: exact `%g` spellings, π to
    // 17 digits among them.
    #[allow(clippy::approx_constant, clippy::excessive_precision)]
    fn g17_matches_c() {
        // Python's `%.17g` (C's) for each value; the spec's `%g` was verified
        // to print these same strings through the Odin compiler.
        let cases: &[f64] = &[
            1.0,
            2.0,
            0.1,
            0.5,
            123.45678901234568,
            1440.0,
            900.0,
            1e21,
            1e-5,
            0.0001,
            0.00001,
            3.141592653589793,
            2.6,
            2600.0,
            -1.5,
            100.0,
            0.3333333333333333,
            1.2345678901234567e-7,
            9999999999999999.0,
            0.30000000000000004,
            1e16,
            1e17,
            123456789012345680.0,
            5e-324,
            1.7976931348623157e308,
            2.2250738585072014e-308,
            0.00010000000000000001,
            999.9999999999999,
            1000.0000000000001,
            12345.678901234567,
            999999.9999999999,
            1.0000000000000002,
            2.0000000000000004,
            10.0 / 3.0,
            2.0 / 3.0,
            1.5e-5,
            9.9999999999999998e-5,
            1.23456789012345678e15,
            1234567.8901234567,
            0.12345678901234568,
            100000.00000000001,
            0.001,
            12.0,
            123.0,
            1234.0,
            12340.0,
            1000000.0,
            123456789.0,
            987654321.1234567,
            1.1,
            2.2,
            6.62607015e-34,
            299792458.0,
            1.602176634e-19,
            0.007,
            70.0,
            700.0,
            7000.0,
            70000.0,
            700000.0,
            7000000.0,
            7e-6,
            7e-7,
        ];
        let expected: &[&str] = &[
            "1",
            "2",
            "0.10000000000000001",
            "0.5",
            "123.45678901234568",
            "1440",
            "900",
            "1e+21",
            "1.0000000000000001e-05",
            "0.0001",
            "1.0000000000000001e-05",
            "3.1415926535897931",
            "2.6000000000000001",
            "2600",
            "-1.5",
            "100",
            "0.33333333333333331",
            "1.2345678901234566e-07",
            "10000000000000000",
            "0.30000000000000004",
            "10000000000000000",
            "1e+17",
            "1.2345678901234568e+17",
            "4.9406564584124654e-324",
            "1.7976931348623157e+308",
            "2.2250738585072014e-308",
            "0.0001",
            "999.99999999999989",
            "1000.0000000000001",
            "12345.678901234567",
            "999999.99999999988",
            "1.0000000000000002",
            "2.0000000000000004",
            "3.3333333333333335",
            "0.66666666666666663",
            "1.5e-05",
            "9.9999999999999991e-05",
            "1234567890123456.8",
            "1234567.8901234567",
            "0.12345678901234568",
            "100000.00000000001",
            "0.001",
            "12",
            "123",
            "1234",
            "12340",
            "1000000",
            "123456789",
            "987654321.12345672",
            "1.1000000000000001",
            "2.2000000000000002",
            "6.6260701499999998e-34",
            "299792458",
            "1.6021766339999999e-19",
            "0.0070000000000000001",
            "70",
            "700",
            "7000",
            "70000",
            "700000",
            "7000000",
            "6.9999999999999999e-06",
            "6.9999999999999997e-07",
        ];
        assert_eq!(cases.len(), expected.len());
        for (value, want) in cases.iter().zip(expected.iter()) {
            assert_eq!(&fmt_g17(*value), want, "for {value}");
        }
    }

    #[test]
    fn views_line_has_the_harness_shape() {
        let line = views_line(
            "default",
            2000,
            2.0,
            0.0,
            -0.0,
            1440.0,
            0.5,
            "default",
            "arctic-ice",
            "[1.000, 2.000, 3.000, 4.000]",
        );
        assert_eq!(
            line,
            "{\"shot\": \"default\", \"time\": 2000, \"zoom\": 2, \"pan_x\": 0, \"pan_y\": -0, \"origin_x\": 1440, \"origin_y\": 0.5, \"cursor\": \"default\", \"theme\": \"arctic-ice\", \"scaled\": [[1.000, 2.000, 3.000, 4.000]]}\n"
        );
        let bare = views_line("a", 0, 1.0, 0.0, 0.0, 0.0, 0.0, "c", "t", "");
        assert!(bare.ends_with("\"scaled\": []}\n"), "{bare}");
    }

    #[test]
    fn url_line_has_its_shape() {
        assert_eq!(
            url_line("center=1,2&scale=2"),
            "{\"url\": \"center=1,2&scale=2\"}\n"
        );
        assert_eq!(url_line(""), "{\"url\": \"\"}\n");
    }

    #[test]
    fn camera_line_has_its_shape() {
        assert_eq!(
            camera_line([1.5, -2.0], 2.0, false),
            "{\"camera\": {\"center\": [1.5, -2], \"scale\": 2, \"animating\": false}}\n"
        );
        assert_eq!(
            camera_line([0.0, 0.0], 0.5, true),
            "{\"camera\": {\"center\": [0, 0], \"scale\": 0.5, \"animating\": true}}\n"
        );
    }

    #[test]
    fn motion_and_time_lines_have_their_shapes() {
        assert_eq!(
            motion_json(&["a".to_string(), "b".to_string()]),
            "{\"moving\": [\"a\", \"b\"]}\n"
        );
        assert_eq!(
            time_json([2880, 1800], false, 10, 1.23456, 2.0, 3.0),
            "{\"pixels\": [2880, 1800], \"window\": false, \"redraws\": 10, \"median_ms\": 1.235, \"p95_ms\": 2.000, \"slowest_ms\": 3.000}\n"
        );
    }

    #[test]
    fn keycodes_follow_sdl() {
        use mv_script::{KeyName, NamedKey};
        assert_eq!(
            keycode(&KeyName::Named(NamedKey::ArrowUp)),
            sdl3::keyboard::Keycode::Up
        );
        assert_eq!(
            keycode(&KeyName::Named(NamedKey::Space)),
            sdl3::keyboard::Keycode::Space
        );
        assert_eq!(
            keycode(&KeyName::Char('a')),
            sdl3::keyboard::Keycode::from_u32(97).unwrap()
        );
    }
}
