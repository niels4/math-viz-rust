//! The whole pointer/keyboard/focus/cursor/target layer.
//!
//! SDL events become `Input` facts; `handle_input` hit-tests the last frame's
//! regions and routes each fact to its widget. Regions and focus stops are
//! declared during layout, resolved into the last frame's targets, and kept in
//! `InputState` on the app.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use enum_map::{Enum, EnumMap};

use crate::app::App;
use crate::app_loop::{Time, USER_EVENT_TYPE, UserCode, clock_now};
use crate::parts::{apply_model, report_focus, report_hover};
#[cfg(not(target_os = "emscripten"))]
use crate::platform_native::{
    platform_reduced_motion, platform_set_cursor, platform_system_dark, platform_wheel_px,
};
#[cfg(target_os = "emscripten")]
use crate::platform_web::{
    platform_reduced_motion, platform_set_cursor, platform_system_dark, platform_wheel_px,
};
use mv_sys::clay;

/// One input fact.
#[derive(Clone, Debug)]
pub enum Input {
    Pointer(PointerEvent),
    Button(ButtonEvent),
    Wheel(WheelEvent),
    Pinch(PinchEvent),
    Key(KeyEvent),
    Text(TextEvent),
    Window(WindowEvent),
    Focus(FocusEvent),
    Scheme(SchemeEvent),
    ReducedMotion(ReducedMotionEvent),
    Model(ModelEvent),
}

/// The pointer moved or its buttons changed.
#[derive(Clone, Copy, Debug)]
pub struct PointerEvent {
    pub pos: [f32; 2],
    pub left: bool,
}

/// A mouse button went down or up. Buttons are SDL's numbering.
#[derive(Clone, Copy, Debug)]
pub struct ButtonEvent {
    pub button: u8,
    pub down: bool,
    pub clicks: i64,
    pub pos: [f32; 2],
}

/// The wheel moved.
#[derive(Clone, Copy, Debug)]
pub struct WheelEvent {
    pub delta: [f32; 2],
    pub pos: [f32; 2],
}

/// A pinch gesture scaled.
#[derive(Clone, Copy, Debug)]
pub struct PinchEvent {
    pub scale: f32,
}

/// A key went down or up.
#[derive(Clone, Debug)]
pub struct KeyEvent {
    pub key: sdl3::keyboard::Keycode,
    /// The modifiers held as the key went down or up.
    pub mods: sdl3::keyboard::Mod,
    pub down: bool,
    pub repeat: bool,
}

/// Text was typed. Valid while `handle_input` runs.
#[derive(Clone, Debug)]
pub struct TextEvent {
    pub text: String,
}

/// The window resized.
#[derive(Clone, Copy, Debug)]
pub struct WindowEvent {
    pub size: [f32; 2],
    pub pixels: [i32; 2],
}

/// The window's focus changed.
#[derive(Clone, Copy, Debug)]
pub struct FocusEvent {
    pub gained: bool,
}

/// The system's light/dark scheme changed.
#[derive(Clone, Copy, Debug)]
pub struct SchemeEvent {
    pub dark: bool,
}

/// The reduced-motion setting changed.
#[derive(Clone, Copy, Debug)]
pub struct ReducedMotionEvent {
    pub reduced: bool,
}

/// A script's model line arrived.
#[derive(Clone, Debug)]
pub struct ModelEvent {
    pub change: mv_script::Model,
}

/// What the pointer hit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Hit {
    pub kind: HitKind,
    pub index: i64,
}

/// The 24 targets a hit can name.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HitKind {
    #[default]
    None,
    /// The top bar's wordmark link.
    Wordmark,
    /// The settings button, which opens the settings panel.
    Gear,
    /// A function picker's option, by index.
    Picker,
    /// A transform's row, by parameter.
    Row,
    /// A row's letter chip.
    Chip,
    /// A scale's flip toggle.
    Flip,
    /// A row's number box.
    Box,
    /// A point's row: P's lights P's partners.
    Point,
    /// P's track.
    Track,
    Plane,
    /// The Original toggle in the plane's tools.
    Ghost,
    /// Return to origin.
    Origin,
    /// The zoom's `+` (0) and `-` (1).
    ZoomStep,
    /// A row's reset, by parameter, while its value is off its default.
    Reset,
    /// Transform's Reset all, while any value is off its default.
    ResetAll,
    /// The zoom control's capsule, its one tab stop.
    Zoom,
    /// The equation card, between its terms.
    Equation,
    /// A parameter's term in the equation.
    Term,
    /// P's x in its readout.
    PField,
    /// An open explainer's card, by parameter.
    Explainer,
    /// The settings panel's Appearance toggle: an option, by mode.
    SettingsMode,
    /// The settings panel's list: a theme's row, by its place.
    SettingsTheme,
    /// The settings panel, between its controls.
    SettingsPanel,
}

/// Whether a kind holds the pointer until release.
pub fn captures(kind: HitKind) -> bool {
    matches!(
        kind,
        HitKind::Box | HitKind::Term | HitKind::Track | HitKind::Plane
    )
}

/// The cursor shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum Cursor {
    Default,
    Pointer,
    Text,
    Crosshair,
    Grab,
    Grabbing,
    EwResize,
    Help,
}

/// Each cursor's CSS name, for the web.
pub static CURSOR_CSS: EnumMap<Cursor, &str> = EnumMap::from_array([
    "default",
    "pointer",
    "text",
    "crosshair",
    "grab",
    "grabbing",
    "ew-resize",
    "help",
]);

/// One declared target: a hit, its box, and its cursors.
#[derive(Clone, Copy, Debug)]
pub struct Region {
    pub hit: Hit,
    pub r#box: clay::BoundingBox,
    pub cursor: Cursor,
    /// While a press on it holds the pointer.
    pub pressed: Cursor,
    pub radius: clay::CornerRadius,
    pub clip: Option<clay::BoundingBox>,
    pub z: i16,
}

/// One selectable text box the I-beam shows over.
#[derive(Clone, Copy, Debug)]
pub struct TextBox {
    pub r#box: clay::BoundingBox,
    pub clip: Option<clay::BoundingBox>,
    pub z: i16,
}

/// The input layer's state: the last frame's targets and this frame's facts.
pub struct InputState {
    pub pointer: [f32; 2],
    /// When the fact being handled happened (`handle_input`); 0 between facts.
    pub time: Time,
    /// The pointer is in the window.
    pub over: bool,
    /// The modifiers held, as the key events left them.
    pub mods: sdl3::keyboard::Mod,
    /// What the pointer is over.
    pub hover: Hit,
    /// What the held primary button went down on.
    pub pressed: Hit,
    /// The focused stop.
    pub focus: Hit,
    /// The focus's ring shows: the keyboard put it there, or a key came since.
    pub focus_visible: bool,
    /// The keyboard put the focus there.
    pub keyed: bool,
    /// The keyboard's focus is to come into view after the next layout.
    pub reveal: bool,
    /// The cursor shown.
    pub cursor: Cursor,
    /// The last frame's targets, in the order they were declared.
    pub regions: Vec<Region>,
    /// The last frame's focus stops, in reading order.
    pub stops: Vec<Hit>,
    /// This frame's, while the layout runs.
    pub declared: Vec<Declared>,
    /// The scroll container the layout is declaring in.
    pub clip: Option<super::scroll::Scroller>,
    /// The stacking layer the layout is declaring in.
    pub z: i16,
    /// The layout is declaring what takes no input.
    pub inert: bool,
    /// The stacking layers declared inert in the last layout.
    pub inert_layers: u64,
    /// The last frame's texts the I-beam shows over.
    pub texts: Vec<TextBox>,
    /// While the draw runs: the clip and layer it draws in.
    pub text_clip: Option<clay::BoundingBox>,
    pub text_z: i16,
}

/// One target declared during layout, resolved into regions after.
#[derive(Clone, Debug)]
pub struct Declared {
    pub id: clay::ElementId,
    pub region: Region,
    pub stop: bool,
    pub clip: Option<super::scroll::Scroller>,
}

/// A parameter index as its parameter, when it names one. Hits and stops only
/// ever hold valid indices (they are built from parameters), so anything else
/// reports none.
fn param_at(index: i64) -> Option<mv_viewer::Param> {
    match index {
        0 => Some(mv_viewer::Param::A),
        1 => Some(mv_viewer::Param::K),
        2 => Some(mv_viewer::Param::B),
        3 => Some(mv_viewer::Param::H),
        _ => None,
    }
}

/// Declares a target for this frame's layout, unless inert.
pub fn region(
    app: &mut App,
    id: clay::ElementId,
    hit: Hit,
    cursor: Cursor,
    pressed: Cursor,
    stop: bool,
    radius: clay::CornerRadius,
) {
    if app.input.inert {
        let z = app.input.z;
        if (0..64).contains(&z) {
            app.input.inert_layers |= 1u64 << z;
        }
        return;
    }
    let z = app.input.z;
    let clip = app.input.clip;
    app.input.declared.push(Declared {
        id,
        region: Region {
            hit,
            r#box: clay::BoundingBox {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
            },
            cursor,
            pressed: if pressed != Cursor::Default {
                pressed
            } else {
                cursor
            },
            radius,
            clip: None,
            z,
        },
        stop,
        clip,
    });
}

/// Resolves the declared boxes through the layout into regions and stops.
///
/// The layout data lives in the Clay context the frame laid out with, which
/// the caller (the layout pass) holds; the app keeps only the arena backing.
pub fn collect_regions(app: &mut App, clay: &mv_clay::Clay) {
    let (input, scroll) = (&mut app.input, &app.scroll);
    input.regions.clear();
    input.stops.clear();
    for d in &input.declared {
        let data = clay.get_element_data(d.id);
        if !data.found {
            continue;
        }
        let mut r = d.region;
        r.r#box = data.boundingBox;
        if let Some(s) = d.clip {
            // The container's padding box: it clips its content inside its border.
            let c = clay.get_element_data(mv_clay::id(crate::scroll::SCROLLER_IDS[s], 0));
            if c.found {
                r.clip = Some(crate::effects::grow_box(c.boundingBox, -scroll[s].border));
            }
        }
        input.regions.push(r);
        if d.stop {
            input.stops.push(r.hit);
        }
    }
    input.declared.clear();
    // The targets moved under a still pointer, or one appeared there.
    retarget(app);
}

/// The topmost region containing `pos`.
pub fn hit_at(regions: &[Region], pos: [f32; 2]) -> Hit {
    let mut hit = Hit::default();
    let mut top = i16::MIN;
    for r in regions.iter().rev() {
        if r.z > top
            && crate::scroll::inside(r.r#box, pos)
            && crate::scroll::inside(r.clip.unwrap_or(r.r#box), pos)
        {
            hit = r.hit;
            top = r.z;
        }
    }
    hit
}

/// The region for a hit, when one is declared.
pub fn region_of(app: &App, hit: Hit) -> (Region, bool) {
    for candidate in &app.input.regions {
        if candidate.hit == hit {
            return (*candidate, true);
        }
    }
    (
        Region {
            hit: Hit::default(),
            r#box: clay::BoundingBox {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
            },
            cursor: Cursor::Default,
            pressed: Cursor::Default,
            radius: clay::CornerRadius {
                topLeft: 0.0,
                topRight: 0.0,
                bottomLeft: 0.0,
                bottomRight: 0.0,
            },
            clip: None,
            z: 0,
        },
        false,
    )
}

/// Routes one input fact to its widget handler.
///
/// `time` is when the fact happened, on the app's clock, 0 for now. The
/// fact's own time stands in `input.time` while it is handled (a drag's
/// samples, a rest's tip, the caret's blink all count from it) and the
/// previous value is restored after, including on the early returns below.
pub fn handle_input(app: &mut App, input: Input, time: Time) {
    let outer = app.input.time;
    app.input.time = if time != 0 { time } else { clock_now(app) };
    match input {
        Input::Pointer(event) => {
            // Odin logs each fact (`log.debugf`); the port's other arms do,
            // this one didn't: gated, so scripted runs stay problem-free
            // (stdout: web stderr counts as a harness problem).
            if crate::trace() {
                println!("input {event:?}");
            }
            app.input.over = !event.left;
            if !event.left {
                app.input.pointer = event.pos;
                match app.input.pressed.kind {
                    HitKind::Box | HitKind::Term => crate::number_box::box_drag(app),
                    HitKind::Track => crate::track::track_drag(app),
                    HitKind::PField => crate::point_rows::p_field_drag(app),
                    HitKind::Plane => crate::plane_input::plane_moved(app),
                    _ => {}
                }
            }
            crate::plane_input::plane_pointer_moved(app);
            retarget(app);
            crate::plane_input::report_plane_pointer(app);
        }
        Input::Button(event) => {
            if crate::trace() {
                println!("input {event:?}");
            }
            app.input.pointer = event.pos;
            app.input.over = true;
            // Only the primary button acts, as the page's pointer handlers check `button === 0`.
            if event.button != 1 {
                app.input.time = outer;
                return;
            }
            if event.down {
                press(app, event.clicks);
            } else {
                release(app);
            }
            crate::plane_input::report_plane_pointer(app);
        }
        Input::Wheel(event) => {
            if crate::trace() {
                println!("input {event:?}");
            }
            app.input.pointer = event.pos;
            let hit = hit_at(app.input.regions.as_slice(), event.pos);
            if hit.kind == HitKind::Box
                && let Some(p) = param_at(hit.index)
            {
                let mode = crate::number_box::held_mode(app.input.mods);
                if crate::number_box::box_wheel(app, p, event.delta[1], mode) {
                    app.input.time = outer;
                    return;
                }
            }
            if app
                .input
                .mods
                .intersects(sdl3::keyboard::Mod::LCTRLMOD | sdl3::keyboard::Mod::RCTRLMOD)
            {
                // A browser hands a trackpad's pinch to the page as a wheel with Ctrl held, -100
                // ln(scale) px, which the web shell reads back as a pinch: the same
                // wheel goes the same way natively, so both builds read it alike, to the bit.
                pinch(app, ((-event.delta[1] as f64) / 100.0).exp() as f32);
            } else if hit.kind == HitKind::Plane {
                // The plane zooms on any wheel, about the pointer.
                crate::plane_input::plane_wheel(app, event.delta[1] as f64);
            } else {
                // The next frame lays the page out scrolled and finds what is under the pointer.
                let _ = crate::scroll::scroll_wheel(app, event.pos, event.delta);
            }
        }
        Input::Pinch(event) => {
            // Over a number box a pinch is a Ctrl + wheel, as a browser reports one, and snaps it.
            let hit = hit_at(app.input.regions.as_slice(), app.input.pointer);
            let snapped = hit.kind == HitKind::Box
                && param_at(hit.index).is_some_and(|p| {
                    crate::number_box::box_wheel(
                        app,
                        p,
                        -100.0 * event.scale.ln(),
                        mv_scrub::Mode::Snap,
                    )
                });
            if snapped {
                app.input.time = outer;
                return;
            }
            pinch(app, event.scale);
        }
        Input::Key(event) => {
            if crate::trace() {
                println!("input {event:?}");
            }
            app.input.mods = event.mods;
            if event.down {
                key_down(app, event.key, event.mods);
            }
            // A modifier pressed or let go mid-drag changes the drag's mode.
            crate::number_box::box_modifiers(app);
        }
        Input::Text(event) => {
            if crate::trace() {
                println!("input text {:?}", event.text);
            }
            crate::number_field::field_text(app, &event.text);
        }
        Input::Window(event) => {
            app.size = event.size;
            app.pixels = event.pixels;
            app.dirty = true;
        }
        Input::Focus(event) => {
            if crate::trace() {
                println!("input {event:?}");
            }
            if event.gained {
                // The system's motion preference is read again as the window comes back.
                let reduced = platform_reduced_motion();
                if reduced != app.reduced_motion {
                    handle_input(app, Input::ReducedMotion(ReducedMotionEvent { reduced }), 0);
                }
            } else {
                // The window's focus left: a drag ends, as the page's blur stoppers end it, and an
                // open field applies its number, as the input's blur does.
                match app.input.pressed.kind {
                    HitKind::Box | HitKind::Term => crate::number_box::box_cancel(app),
                    HitKind::Track => crate::track::track_release(app),
                    HitKind::Plane => crate::plane_input::plane_cancel(app),
                    _ => {}
                }
                if app.input.pressed.kind != HitKind::None {
                    app.input.pressed = Hit::default();
                    retarget(app);
                    crate::plane_input::report_plane_pointer(app);
                }
                crate::number_field::field_blur(app);
            }
        }
        Input::Scheme(event) => {
            app.system_scheme = if event.dark {
                mv_theme::Scheme::Dark
            } else {
                mv_theme::Scheme::Light
            };
            if crate::trace() {
                println!("scheme {:?}", app.system_scheme);
            }
            app.dirty = true;
        }
        Input::ReducedMotion(event) => {
            app.reduced_motion = event.reduced;
            if crate::trace() {
                println!("reduced motion {}", event.reduced);
            }
            app.dirty = true;
        }
        Input::Model(event) => {
            if crate::trace() {
                println!("input {event:?}");
            }
            apply_model(app, &event.change);
        }
    }
    app.input.time = outer;
}

/// Recomputes hover and cursor after the pointer or targets changed.
pub fn retarget(app: &mut App) {
    let over = app.input.over;
    let pointer = app.input.pointer;
    let pressed = app.input.pressed;
    let mut hit = if over {
        hit_at(app.input.regions.as_slice(), pointer)
    } else {
        Hit::default()
    };
    if captures(pressed.kind) {
        hit = pressed;
    }
    if hit != app.input.hover {
        let from = app.input.hover;
        app.input.hover = hit;
        crate::number_box::box_hover(app, from, hit);
        crate::explainer::chip_hover(app, from, hit);
        report_hover(app, from, hit);
        app.dirty = true;
    }
    update_cursor(app);
}

/// Picks the cursor from pressed/hover/plane/text.
pub fn update_cursor(app: &mut App) {
    let hover = app.input.hover;
    let pressed = app.input.pressed;
    let over = app.input.over;
    let (mut cursor, mut z) = (Cursor::Default, i16::MIN);
    if pressed.kind != HitKind::None {
        let (r, found) = region_of(app, pressed);
        if found {
            cursor = r.pressed;
            z = r.z;
        }
    } else if over {
        let (r, found) = region_of(app, hover);
        if found {
            cursor = r.cursor;
            z = r.z;
        }
    }
    // The plane's cursor says what a press on it would do there.
    if hover.kind == HitKind::Plane && (over || pressed.kind == HitKind::Plane) {
        cursor = crate::plane_input::plane_cursor(app);
    }
    if cursor == Cursor::Default && over && !captures(pressed.kind) && over_text(app, z) {
        cursor = Cursor::Text;
    }
    set_cursor(app, cursor);
}

/// Whether the pointer is over drawn text at layer `z`.
pub fn over_text(app: &App, z: i16) -> bool {
    let pos = app.input.pointer;
    for t in &app.input.texts {
        if t.z >= z
            && crate::scroll::inside(t.r#box, pos)
            && crate::scroll::inside(t.clip.unwrap_or(t.r#box), pos)
        {
            return true;
        }
    }
    false
}

/// Records a selectable text box.
pub fn note_text(app: &mut App, bounds: clay::BoundingBox) {
    let z = app.input.text_z;
    if bounds.width <= 0.0
        || bounds.height <= 0.0
        || ((0..64).contains(&z) && app.input.inert_layers & (1u64 << z) != 0)
    {
        return;
    }
    let clip = app.input.text_clip;
    app.input.texts.push(TextBox {
        r#box: bounds,
        clip,
        z,
    });
}

/// Shows `cursor` through the platform when it changed.
pub fn set_cursor(app: &mut App, cursor: Cursor) {
    if cursor != app.input.cursor {
        app.input.cursor = cursor;
        if crate::trace() {
            println!("cursor {cursor:?}");
        }
        platform_set_cursor(cursor);
    }
}

/// Captures the target, sets focus, routes to the widget press.
pub fn press(app: &mut App, clicks: i64) {
    let pointer = app.input.pointer;
    let hit = hit_at(app.input.regions.as_slice(), pointer);
    app.input.pressed = hit;
    crate::settings::settings_pressed(app, hit);
    // A press in the open field keeps its focus (P's x, which isn't a stop of its own).
    let target = if is_stop(app, hit) || crate::number_field::field_holds(app, hit) {
        hit
    } else {
        Hit::default()
    };
    set_focus(app, target, false, false);
    match hit.kind {
        HitKind::Box => {
            if let Some(p) = param_at(hit.index) {
                crate::number_box::box_press(app, p, clicks);
            }
        }
        HitKind::Term => {
            if let Some(p) = crate::equation::term_param(hit) {
                crate::number_box::term_press(app, p, clicks);
            }
        }
        HitKind::Track => crate::track::track_press(app),
        HitKind::PField => crate::point_rows::p_field_press(app),
        HitKind::Plane => crate::plane_input::plane_press(app),
        _ => {}
    }
    app.dirty = true;
    retarget(app);
}

/// Releases the capture: clicks or routes the release.
pub fn release(app: &mut App) {
    let pressed = app.input.pressed;
    app.input.pressed = Hit::default();
    match pressed.kind {
        HitKind::Box | HitKind::Term => crate::number_box::box_release(app),
        HitKind::Track => crate::track::track_release(app),
        HitKind::PField => {
            app.boxes.field.selecting = false;
        }
        HitKind::Plane => crate::plane_input::plane_release(app),
        HitKind::None => {}
        _ => {
            if hit_at(app.input.regions.as_slice(), app.input.pointer) == pressed {
                activate(app, pressed, false);
            }
        }
    }
    app.dirty = true;
    retarget(app);
}

/// Clicks a button target or a keyboard-activated stop.
pub fn activate(app: &mut App, hit: Hit, keyboard: bool) {
    match hit.kind {
        HitKind::Picker => crate::sidebar::picker_choose(app, hit.index, keyboard),
        HitKind::Flip => {
            if let Some(p) = param_at(hit.index) {
                crate::sidebar::flip_activate(app, p);
            }
        }
        HitKind::Reset => {
            if let Some(p) = param_at(hit.index) {
                crate::parts::dispatch(
                    app,
                    mv_viewer::Action::ResetParam(mv_viewer::ResetParam { param: p }),
                );
            }
        }
        HitKind::ResetAll => crate::sidebar::reset_all_activate(app),
        HitKind::Ghost | HitKind::Origin | HitKind::ZoomStep => {
            crate::plane_area::plane_tool_activate(app, hit);
        }
        HitKind::Chip => {
            if let Some(p) = param_at(hit.index) {
                crate::explainer::chip_click(app, p);
            }
        }
        HitKind::Gear => crate::settings::gear_activate(app, keyboard),
        HitKind::SettingsMode => {
            crate::settings::settings_mode_choose(app, hit.index, keyboard);
        }
        HitKind::SettingsTheme => {
            crate::settings::settings_theme_choose(app, hit.index, keyboard);
        }
        _ => {}
    }
}

/// Zooms the plane when over it.
pub fn pinch(app: &mut App, scale: f32) {
    if crate::trace() {
        println!("input pinch {scale:.4}");
    }
    if hit_at(app.input.regions.as_slice(), app.input.pointer).kind == HitKind::Plane {
        crate::plane_input::plane_pinch(app, scale);
    }
}

/// Routes a key to the focused widget, the plane, the explainer, Tab/activate.
pub fn key_down(app: &mut App, key: sdl3::keyboard::Keycode, mods: sdl3::keyboard::Mod) {
    let focus = app.input.focus;
    let shift = mods.intersects(sdl3::keyboard::Mod::LSHIFTMOD | sdl3::keyboard::Mod::RSHIFTMOD);
    // Any key but a modifier alone shows the focused part's ring, focused by a press or not:
    // `:focus-visible` comes to match an element the keyboard interacts with. No focus event
    // comes of it, so what the parts take from one stays as the press left it.
    if focus.kind != HitKind::None && !modifier_key(key) && !app.input.focus_visible {
        app.input.focus_visible = true;
        app.dirty = true;
    }
    match focus.kind {
        HitKind::Box => {
            if let Some(p) = param_at(focus.index) {
                if crate::number_field::field_open_on(app, p) {
                    if key == sdl3::keyboard::Keycode::Tab && shift {
                        // The box itself is the stop before its field: it takes the focus back, and the
                        // field applies what it holds as it loses the focus.
                        crate::number_field::field_blur(app);
                        set_focus(app, focus, true, true);
                        return;
                    }
                    // The field takes its keys; a character comes as text input, and Tab moves on.
                    if crate::number_field::field_key(app, key, mods)
                        || key != sdl3::keyboard::Keycode::Tab
                    {
                        return;
                    }
                } else if crate::number_box::box_key(app, p, key, mods) {
                    return;
                }
            }
        }
        HitKind::PField => {
            if crate::number_field::p_field_open(app) {
                if key == sdl3::keyboard::Keycode::Tab {
                    // The field sits just before P's track in reading order: Tab moves on to the track,
                    // Shift+Tab to the stop before it, and the field applies what it holds as it loses
                    // the focus.
                    crate::number_field::field_blur(app);
                    let track = Hit {
                        kind: HitKind::Track,
                        index: 0,
                    };
                    let next = if shift {
                        stop_beside(app, track, -1)
                    } else {
                        track
                    };
                    set_focus(app, next, true, false);
                    return;
                }
                // The field takes its keys; a character comes as text input.
                let _ = crate::number_field::field_key(app, key, mods);
                return;
            }
        }
        HitKind::Track => {
            if crate::track::track_key(app, key, mods) {
                return;
            }
        }
        HitKind::Picker => {
            if crate::sidebar::picker_key(app, key) {
                return;
            }
        }
        HitKind::Zoom => {
            if crate::plane_area::zoom_control_key(app, key, mods) {
                return;
            }
        }
        HitKind::Gear | HitKind::SettingsMode | HitKind::SettingsTheme
            if crate::settings::settings_key(app, focus, key) =>
        {
            // The open panel takes Esc first, and the view's own Esc waits for the next one.
            return;
        }
        _ => {}
    }
    if focus.kind == HitKind::Plane && crate::plane_input::plane_key(app, key, mods) {
        return;
    }
    if crate::explainer::explainer_key(app, key, mods) {
        return;
    }
    match key {
        sdl3::keyboard::Keycode::Tab => move_focus(app, if shift { -1 } else { 1 }),
        sdl3::keyboard::Keycode::Return | sdl3::keyboard::Keycode::Space
            if focus.kind != HitKind::None =>
        {
            activate(app, focus, true);
        }
        _ => {}
    }
}

/// Whether the key is a modifier or caps lock.
pub fn modifier_key(key: sdl3::keyboard::Keycode) -> bool {
    matches!(
        key,
        sdl3::keyboard::Keycode::LShift
            | sdl3::keyboard::Keycode::RShift
            | sdl3::keyboard::Keycode::LCtrl
            | sdl3::keyboard::Keycode::RCtrl
            | sdl3::keyboard::Keycode::LAlt
            | sdl3::keyboard::Keycode::RAlt
            | sdl3::keyboard::Keycode::LGui
            | sdl3::keyboard::Keycode::RGui
            | sdl3::keyboard::Keycode::CapsLock
    )
}

/// Whether the hit is a focus stop.
pub fn is_stop(app: &App, hit: Hit) -> bool {
    app.input.stops.contains(&hit)
}

/// Moves focus stepwise, wrapping out at the ends.
pub fn move_focus(app: &mut App, step: i64) {
    let count = app.input.stops.len();
    if count == 0 {
        return;
    }
    let mut at: i64 = -1;
    for (i, stop) in app.input.stops.iter().enumerate() {
        if *stop == app.input.focus {
            at = i as i64;
        }
    }
    let next = if at >= 0 {
        at + step
    } else if step > 0 {
        0
    } else {
        count as i64 - 1
    };
    if next < 0 || next >= count as i64 {
        set_focus(app, Hit::default(), false, false);
    } else {
        let hit = app.input.stops[next as usize];
        set_focus(app, hit, true, false);
    }
    if crate::trace() {
        // The harness's focus line (`tools/run loop` reads it): the Odin
        // shape `Hit{kind = Box, index = 1}`, not Rust's `{:?}` debug, whose
        // spaces and colons the loop's FOCUS_LINE does not match.
        println!(
            "focus Hit{{kind = {:?}, index = {}}}",
            app.input.focus.kind, app.input.focus.index
        );
    }
}

/// The neighbouring stop, or an empty hit.
pub fn stop_beside(app: &mut App, hit: Hit, step: i64) -> Hit {
    for (i, stop) in app.input.stops.iter().enumerate() {
        if *stop == hit {
            let next = i as i64 + step;
            if next >= 0 && next < app.input.stops.len() as i64 {
                return app.input.stops[next as usize];
            }
        }
    }
    Hit::default()
}

/// Moves focus, notifies, and marks reveal/dirty.
pub fn set_focus(app: &mut App, hit: Hit, visible: bool, again: bool) {
    let before = app.input.focus;
    app.input.focus = hit;
    app.input.focus_visible = visible;
    app.input.keyed = visible;
    if before != hit && crate::number_field::field_holds(app, before) {
        crate::number_field::field_blur(app);
    }
    crate::settings::settings_focus_moved(app, before, hit);
    report_focus(app, again || before != hit);
    // A focus the keyboard moves shows the part: its scroll container brings it into view
    // once the next layout has shown what the focus changes.
    if visible && before != hit {
        app.input.reveal = true;
    }
    app.dirty = true;
}

/// Pinch scales the raw-event watch has seen, keyed by the event's
/// timestamp, until the routed `Unknown` event takes its own.
///
/// A `static Mutex`, not a `thread_local!`: SDL may call the event watch on
/// another thread than the one pumping events, and a thread-local would then
/// hide the scale from the routing thread (or lose it at the thread's end).
/// The safe event keeps the queued event's timestamp but drops its
/// payload, so each scale waits here, as the wheel ticks wait in the
/// platform. Each routed pinch consumes its own entry; entries nothing
/// routes (a watch registered late, a queue never pumped) stay until
/// newer pinches evict them.
static PINCH_SCALES: Mutex<Vec<(u64, f32)>> = Mutex::new(Vec::new());

/// Registers the raw-event watch that recovers pinch scales.
///
/// SDL reports a trackpad pinch as `SDL_EVENT_PINCH_UPDATE`, whose scale the
/// safe event enum drops (it arrives as `Unknown` with no payload). The watch
/// reads the raw union as SDL queues each event — the point where the Odin
/// port's `input_from_event` reads `event.pinch.scale` — and the `Unknown`
/// arm joins the scale back by the timestamp both carry. Call once at startup
/// (next to the platform's own event watch); a second call is a no-op. The
/// web's pinch arrives the same way: `library.js` posts SDL's own pinch event
/// for a Ctrl+wheel, so this path serves both targets.
pub fn input_watch_pinch() {
    static REGISTERED: AtomicBool = AtomicBool::new(false);
    if REGISTERED.swap(true, Ordering::Relaxed) {
        return;
    }
    // SAFETY: the watch only reads the queued event and records a scale in a
    // `static Mutex`; it runs on whatever thread SDL notifies (the main
    // thread under SDL's main callbacks, possibly another), where the
    // mutex is valid. The lock is held only for the push.
    let added =
        unsafe { sdl3::sys::events::SDL_AddEventWatch(Some(pinch_watch), std::ptr::null_mut()) };
    if !added {
        REGISTERED.store(false, Ordering::Relaxed);
        eprintln!(
            "function-viewer-rust: input: no event watch: a trackpad pinch never becomes a fact"
        );
    }
}

/// The SDL event watch: notes pinch scales by their event's timestamp.
///
/// Matches `SDL_EventFilter` exactly (the surface's looser signature is
/// narrowed here so the watch registers without a trampoline).
unsafe extern "C" fn pinch_watch(
    _userdata: *mut std::ffi::c_void,
    event: *mut sdl3::sys::events::SDL_Event,
) -> bool {
    // SAFETY: SDL hands the filter a valid event; the union's type word is read first.
    if unsafe { (*event).r#type } != sdl3::sys::events::SDL_EventType::PINCH_UPDATE.0 {
        return true;
    }
    // SAFETY: the type was just checked, so the union's pinch member is live; both fields are plain data.
    let (timestamp, scale) = unsafe { ((*event).pinch.timestamp, (*event).pinch.scale) };
    {
        let mut scales = PINCH_SCALES.lock().unwrap_or_else(|e| e.into_inner());
        // A queue that is never pumped (or a watch registered late) must not
        // grow without bound; the oldest entry is the stalest either way.
        if scales.len() >= 64 {
            scales.remove(0);
        }
        scales.push((timestamp, scale));
    }
    true
}

/// The pinch the `Unknown` event carries, when the raw watch saw its scale.
///
/// Takes the entry: each routed pinch consumes its own. Nothing comes back
/// for a pinch the watch never saw (it runs from registration on; anything
/// queued before is dropped, as it is today), nor for any other event type.
fn pinch_from_unknown(type_: u32, timestamp: u64) -> Option<PinchEvent> {
    if sdl3::sys::events::SDL_EventType::PINCH_UPDATE != type_ {
        return None;
    }
    let mut scales = PINCH_SCALES.lock().unwrap_or_else(|e| e.into_inner());
    scales
        .iter()
        .position(|(t, _)| *t == timestamp)
        .map(|at| PinchEvent {
            scale: scales.remove(at).1,
        })
}

/// Converts an SDL event into an input fact, when it is one.
pub fn input_from_event(app: &mut App, event: &sdl3::event::Event) -> Option<Input> {
    match event {
        sdl3::event::Event::MouseMotion { x, y, .. } => Some(Input::Pointer(PointerEvent {
            pos: [*x, *y],
            left: false,
        })),
        sdl3::event::Event::Window {
            win_event: sdl3::event::WindowEvent::MouseLeave,
            ..
        } => Some(Input::Pointer(PointerEvent {
            pos: [0.0, 0.0],
            left: true,
        })),
        sdl3::event::Event::MouseButtonDown {
            mouse_btn,
            clicks,
            x,
            y,
            ..
        } => Some(Input::Button(ButtonEvent {
            button: *mouse_btn as u8,
            down: true,
            clicks: *clicks as i64,
            pos: [*x, *y],
        })),
        sdl3::event::Event::MouseButtonUp {
            mouse_btn,
            clicks,
            x,
            y,
            ..
        } => Some(Input::Button(ButtonEvent {
            button: *mouse_btn as u8,
            down: false,
            clicks: *clicks as i64,
            pos: [*x, *y],
        })),
        sdl3::event::Event::MouseWheel {
            x,
            y,
            direction,
            mouse_x,
            mouse_y,
            ..
        } => {
            // The bindings' notches, +y away from the user, in the page's px, +y down.
            let (mut nx, mut ny) = (*x, *y);
            if matches!(direction, sdl3::mouse::MouseWheelDirection::Flipped) {
                nx = -nx;
                ny = -ny;
            }
            let px = platform_wheel_px(event);
            Some(Input::Wheel(WheelEvent {
                delta: [nx * px, -ny * px],
                pos: [*mouse_x, *mouse_y],
            }))
        }
        sdl3::event::Event::KeyDown {
            keycode,
            keymod,
            repeat,
            ..
        } => {
            let key = (*keycode)?;
            Some(Input::Key(KeyEvent {
                key,
                mods: *keymod,
                down: true,
                repeat: *repeat,
            }))
        }
        sdl3::event::Event::KeyUp {
            keycode,
            keymod,
            repeat,
            ..
        } => {
            let key = (*keycode)?;
            Some(Input::Key(KeyEvent {
                key,
                mods: *keymod,
                down: false,
                repeat: *repeat,
            }))
        }
        sdl3::event::Event::TextInput { text, .. } => {
            Some(Input::Text(TextEvent { text: text.clone() }))
        }
        sdl3::event::Event::Window { win_event, .. } => match win_event {
            sdl3::event::WindowEvent::Resized(..)
            | sdl3::event::WindowEvent::PixelSizeChanged(..)
            | sdl3::event::WindowEvent::Exposed => Some(Input::Window(window_event(app.window()))),
            sdl3::event::WindowEvent::FocusGained => {
                Some(Input::Focus(FocusEvent { gained: true }))
            }
            sdl3::event::WindowEvent::FocusLost => Some(Input::Focus(FocusEvent { gained: false })),
            _ => None,
        },
        sdl3::event::Event::User {
            type_, code, data1, ..
        } => {
            if *type_ == USER_EVENT_TYPE.load(Ordering::Relaxed)
                && *code == UserCode::ReducedMotion as i32
            {
                Some(Input::ReducedMotion(ReducedMotionEvent {
                    reduced: !data1.is_null(),
                }))
            } else {
                None
            }
        }
        // The safe event enum has no pinch, system-theme or display-scale
        // variants: all three arrive as `Unknown`. The theme change carries
        // no payload and ports whole; the pinch scale waits in the raw-event
        // watch, joined by the timestamp both carry; the display-scale change
        // re-reads the window sizes, as the Odin window event does.
        sdl3::event::Event::Unknown { type_, timestamp } => {
            if sdl3::sys::events::SDL_EventType::SYSTEM_THEME_CHANGED == *type_ {
                Some(Input::Scheme(SchemeEvent {
                    dark: system_dark(),
                }))
            } else if let Some(pinch) = pinch_from_unknown(*type_, *timestamp) {
                Some(Input::Pinch(pinch))
            } else if sdl3::sys::events::SDL_EventType::WINDOW_DISPLAY_SCALE_CHANGED == *type_ {
                Some(Input::Window(window_event(app.window())))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Whether the system theme is dark (the platform's theme query; the safe
/// `sdl3` crate has none).
pub fn system_dark() -> bool {
    platform_system_dark()
}

/// Reads the window's logical and pixel sizes.
pub fn window_event(window: &sdl3::video::Window) -> WindowEvent {
    let (w, h) = window.size();
    let (pw, ph) = window.size_in_pixels();
    WindowEvent {
        size: [w as f32, h as f32],
        pixels: [pw as i32, ph as i32],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;

    /// Serializes the tests that push to and peep from SDL's global event
    /// queue: parallel `cargo test` threads otherwise race (a peep takes a
    /// sibling test's event, or a push lands after the peep), failing about
    /// 2 runs in 8. Added by i1-frame (driver's note); the shape follows
    /// `mv_clay::tests_lock`, but this is its own queue and its own lock.
    static SDL_QUEUE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// An app that runs no layout: the model and the focus, as the input
    /// handlers leave them. Every other field is the headless constructor's
    /// zero: no window, no GL, no NanoVG. Ported from `input_test.odin:15`.
    ///
    /// Exception to `motion_watch_fade`'s invariant (registered only after
    /// the App's last move): this test App is still moved once, by the
    /// return below, after `motion_init` stores the raw pointer to its Q
    /// row. The exception holds because no test here ever draws or reads
    /// the watch list — the stored pointer is never dereferenced — as the
    /// reference test's listing only touches the motion state the
    /// assertions below read.
    fn input_app() -> App {
        let mut app = crate::fixture::headless_app();
        app.viewer = mv_viewer::initial_state();
        app.r#loop.script_time = Some(0);
        crate::motion_driver::motion_init(&mut app);
        app
    }

    /// A parameter as its same-named part.
    fn part_of_param(p: mv_viewer::Param) -> mv_viewer::Part {
        match p {
            mv_viewer::Param::A => mv_viewer::Part::A,
            mv_viewer::Param::K => mv_viewer::Part::K,
            mv_viewer::Param::B => mv_viewer::Part::B,
            mv_viewer::Param::H => mv_viewer::Part::H,
        }
    }

    /// A key on a part a press focused shows its ring and no tip; a modifier
    /// alone shows no ring. The keyboard's own focus brings both.
    #[test]
    fn a_key_on_a_part_a_press_focused_shows_its_ring_and_no_tip() {
        let app = &mut input_app();
        // The reference test lists the driver's own fades here (`motion_init`);
        // the app-surface module for the motion driver omits that procedure,
        // so the call lands with app-plane's declaration (recorded). The
        // listing only touches the motion watch list, which none of the
        // assertions below read.
        let k = mv_viewer::Param::K;
        let bx = Hit {
            kind: HitKind::Box,
            index: k as i64,
        };
        let tip = |app: &mut App, p: mv_viewer::Param| -> Vec<mv_viewer::Phrase> {
            let ui = mv_viewer::part_ui(&app.viewer, part_of_param(p));
            let (phrases, _) = crate::sidebar::row_tip_phrases(app, p, &ui);
            phrases
        };
        set_focus(app, bx, false, false); // as a press leaves it
        assert!(!app.input.focus_visible);
        key_down(
            app,
            sdl3::keyboard::Keycode::LShift,
            sdl3::keyboard::Mod::LSHIFTMOD,
        );
        assert!(!app.input.focus_visible);
        key_down(
            app,
            sdl3::keyboard::Keycode::Right,
            sdl3::keyboard::Mod::empty(),
        );
        assert!(app.input.focus_visible);
        assert!(!app.input.keyed);
        assert_eq!(app.viewer.params[k], 0.01);
        assert!(tip(app, k).is_empty());
        // The flip (a button focused on click): a key it takes no action on shows its ring.
        let flip = Hit {
            kind: HitKind::Flip,
            index: mv_viewer::Param::A as i64,
        };
        set_focus(app, flip, false, false);
        key_down(
            app,
            sdl3::keyboard::Keycode::X,
            sdl3::keyboard::Mod::empty(),
        );
        assert!(app.input.focus_visible);
        assert_eq!(app.viewer.params[mv_viewer::Param::A], 1.0);
        // Tab's focus is the keyboard's: the ring and the tip.
        set_focus(app, bx, true, false);
        assert!(app.input.focus_visible && app.input.keyed);
        assert!(!tip(app, k).is_empty());
    }

    /// A synthetic pinch queued through the raw sys calls decodes to a pinch
    /// with its scale: the watch notes the raw union's scale as SDL queues
    /// the event, the safe conversion keeps it as `Unknown` with the queued
    /// timestamp, and the lookup joins them. Headless: the events subsystem
    /// only, no window.
    #[test]
    fn pinch_raw_event_yields_pinch_with_its_scale() {
        let _queue = SDL_QUEUE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        use sdl3::sys::{events as sys_events, init as sys_init};
        // SAFETY: starting the events subsystem headless; nothing here needs
        // video or a window, and the subsystem is never quit (sibling tests
        // own their SDL lifetimes).
        assert!(unsafe { sys_init::SDL_InitSubSystem(sys_init::SDL_INIT_EVENTS) });
        input_watch_pinch();
        const SCALE: f32 = 1.25;
        // A synthetic pinch, as the OS would queue one.
        let mut pushed = sys_events::SDL_Event {
            pinch: sys_events::SDL_PinchFingerEvent {
                r#type: sys_events::SDL_EventType::PINCH_UPDATE,
                reserved: 0,
                timestamp: 0,
                scale: SCALE,
                windowID: sdl3::sys::video::SDL_WindowID(0),
                ..Default::default()
            },
        };
        // SAFETY: the event is a plain data struct SDL copies on push.
        assert!(unsafe { sys_events::SDL_PushEvent(&mut pushed) });
        // Take exactly the pinch back out, leaving any other queued event (a
        // sibling test's) where it is.
        let mut found: [sys_events::SDL_Event; 1] = unsafe { std::mem::zeroed() };
        // SAFETY: the buffer holds one event; the type range names only pinches.
        let taken = unsafe {
            sys_events::SDL_PeepEvents(
                found.as_mut_ptr(),
                1,
                sys_events::SDL_EventAction::GETEVENT,
                sys_events::SDL_EventType::PINCH_UPDATE.0,
                sys_events::SDL_EventType::PINCH_UPDATE.0,
            )
        };
        assert_eq!(taken, 1);
        // SAFETY: an event was taken, so the buffer's pinch member is live.
        let scale = unsafe { found[0].pinch.scale };
        assert_eq!(scale, SCALE);
        // The safe conversion delivers it as `Unknown` with the queued timestamp...
        // SAFETY: the taken event is a plain data struct; it is read once here.
        let raw = unsafe { std::ptr::read(found.as_ptr()) };
        let decoded = sdl3::event::Event::from_ll(raw);
        let (type_, timestamp) = match decoded {
            sdl3::event::Event::Unknown { type_, timestamp } => (type_, timestamp),
            other => panic!("a pinch decodes as Unknown, got {other:?}"),
        };
        assert_eq!(type_, sys_events::SDL_EventType::PINCH_UPDATE.0);
        // ...and the watched scale joins it there.
        let pinch = pinch_from_unknown(type_, timestamp).expect("a watched pinch scale");
        assert_eq!(pinch.scale, SCALE);
        // Each routed pinch consumes its own scale: a second routing finds none.
        assert!(pinch_from_unknown(type_, timestamp).is_none());
    }

    /// The pinch store is shared across threads: SDL may call the event watch
    /// on another thread than the one routing events, so a scale pushed from
    /// a second thread must be visible where the routing runs. (The review
    /// asked for this proof when the store moved off `thread_local!`.)
    #[test]
    fn pinch_store_is_shared_across_threads() {
        const STAMP: u64 = u64::MAX - 7;
        const SCALE: f32 = 2.5;
        assert!(
            pinch_from_unknown(sdl3::sys::events::SDL_EventType::PINCH_UPDATE.0, STAMP).is_none()
        );
        std::thread::scope(|s| {
            s.spawn(|| {
                PINCH_SCALES
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .push((STAMP, SCALE));
            });
        });
        let pinch = pinch_from_unknown(sdl3::sys::events::SDL_EventType::PINCH_UPDATE.0, STAMP)
            .expect("a scale pushed from a second thread");
        assert_eq!(pinch.scale, SCALE);
    }

    /// A synthetic display-scale change queues and decodes as the `Unknown`
    /// type the re-read arm matches, which is the headless half of the fix:
    /// the arm itself re-reads live window sizes (as the Odin window event
    /// does), so its run waits for a real display-scale event under `i0`'s
    /// routing.
    #[test]
    fn display_scale_change_decodes_as_its_unknown_type() {
        let _queue = SDL_QUEUE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        use sdl3::sys::{events as sys_events, init as sys_init};
        // SAFETY: as in the pinch test.
        assert!(unsafe { sys_init::SDL_InitSubSystem(sys_init::SDL_INIT_EVENTS) });
        let mut pushed = sys_events::SDL_Event {
            window: sys_events::SDL_WindowEvent {
                r#type: sys_events::SDL_EventType::WINDOW_DISPLAY_SCALE_CHANGED,
                reserved: 0,
                timestamp: 0,
                windowID: sdl3::sys::video::SDL_WindowID(0),
                data1: 0,
                data2: 0,
                ..Default::default()
            },
        };
        // SAFETY: as in the pinch test.
        assert!(unsafe { sys_events::SDL_PushEvent(&mut pushed) });
        let mut found: [sys_events::SDL_Event; 1] = unsafe { std::mem::zeroed() };
        // SAFETY: the buffer holds one event; the type range names only the scale change.
        let taken = unsafe {
            sys_events::SDL_PeepEvents(
                found.as_mut_ptr(),
                1,
                sys_events::SDL_EventAction::GETEVENT,
                sys_events::SDL_EventType::WINDOW_DISPLAY_SCALE_CHANGED.0,
                sys_events::SDL_EventType::WINDOW_DISPLAY_SCALE_CHANGED.0,
            )
        };
        assert_eq!(taken, 1);
        // SAFETY: an event was taken; it is read once here.
        let raw = unsafe { std::ptr::read(found.as_ptr()) };
        match sdl3::event::Event::from_ll(raw) {
            sdl3::event::Event::Unknown { type_, .. } => {
                assert_eq!(
                    type_,
                    sys_events::SDL_EventType::WINDOW_DISPLAY_SCALE_CHANGED.0
                )
            }
            other => panic!("a display-scale change decodes as Unknown, got {other:?}"),
        }
    }
}
