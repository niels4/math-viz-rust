//! The pointer/keyboard/focus/cursor/target layer.
//!
//! SDL events become `Input` facts; `handle_input` updates the shell state
//! (pointer, modifiers, size, scheme) and marks dirty. The page milestones
//! route the facts to their handlers from here. Regions and focus stops are
//! declared during layout, resolved into the last frame's targets, and kept in
//! `InputState` on the app.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use enum_map::{Enum, EnumMap};

use crate::app::App;
use crate::app_loop::{Time, USER_EVENT_TYPE, UserCode, clock_now};
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

/// The targets a hit can name. The shell names none; the page milestones
/// append their kinds here (append-only, like `App`'s fields).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HitKind {
    #[default]
    None,
    /// The top bar's settings gear.
    Gear,
    /// A light/dark/system cell; `Hit.index` names the cell (0 system, 1 dark, 2 light).
    SettingsLightDark,
    /// A theme row; `Hit.index` names the row in the shown list (0..3).
    SettingsTheme,
    /// The plane's return-to-centre button (h2-plane's overlay).
    ReturnToCenter,
    /// The Cartesian plane's canvas (h2-input; h2-input declares the region).
    Plane,
    /// A function-picker option; `Hit.index` names the option in picker
    /// order (h3's selector declares the regions).
    Selector,
    /// A readout term; `Hit.index` names the parameter in A B H K order, in
    /// both lines (h4-readout's readout declares the regions).
    ReadoutTerm,
    /// A transform row; `Hit.index` names the parameter in `TransformParam`
    /// order (h4-rows' transform_row declares the regions).
    TransformRow,
    /// A transform row's number field; `Hit.index` names the parameter
    /// (h4-rows; the row's focus stop).
    TransformField,
    /// A scale row's flip toggle; `Hit.index` names the parameter in
    /// `TransformParam` order (h5-buttons; a focus stop, in tab order).
    TransformFlip,
    /// A row's reset button; `Hit.index` names the parameter (h5-buttons;
    /// never a focus stop: `tabIndex={-1}`).
    TransformReset,
    /// The Transform card's Reset all (h5-buttons; a focus stop, `index` 0).
    TransformResetAll,
    /// The top bar's wordmark link to the root page (h8-plane; `index` 0).
    /// A hot region with the pointer cursor and a no-op click on both
    /// targets: the root page is the template's placeholder card, never in
    /// the port, which has one route (the planner's call, `decisions.md`).
    /// Never a focus stop: the port's tab walk is the milestones' own.
    Wordmark,
}

/// Whether a kind holds the pointer until release. The shell captures
/// nothing; the page milestones name their capturing kinds here.
pub fn captures(kind: HitKind) -> bool {
    matches!(kind, HitKind::Plane)
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
    /// Keep the declared box: a painted target no Clay element resolves.
    pub keep_box: bool,
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
        keep_box: false,
    });
}

/// Declares a painted target: the layout computed its box (no Clay element
/// resolves it), so `collect_regions` keeps the declared box. The plane's
/// overlay button uses this; Clay-laid targets use `region`.
pub fn region_kept(
    app: &mut App,
    hit: Hit,
    r#box: clay::BoundingBox,
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
        // Never resolved: `collect_regions` keeps the declared box.
        id: mv_clay::id("KeptBox", 0),
        region: Region {
            hit,
            r#box,
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
        keep_box: true,
    });
}

/// Resolves the declared boxes through the layout into regions and stops.
///
/// Clay-laid targets resolve their boxes from their elements; painted targets
/// declared through [`region_kept`] keep their declared boxes. The layout
/// data lives in the Clay context the frame laid out with, which the caller
/// (the layout pass) holds; the app keeps only the arena backing.
///
/// Callers hold `&mut App` whole, beside the Clay context they read: the
/// context outlives the call on this thread and no layout runs beside it
/// (the same shape as [`crate::ui::ui_scope`]'s aliasing).
pub fn collect_regions(app: &mut App) {
    let clay = app.clay.as_ref().expect("clay context initialised") as *const mv_clay::Clay;
    // SAFETY: the frame lays out and collects on this thread while the app
    // lives; the context is read (element boxes) while the input state is
    // written, disjoint fields no layout touches beside this call.
    let clay = unsafe { &*clay };
    let (input, scroll) = (&mut app.input, &app.scroll);
    input.regions.clear();
    input.stops.clear();
    // The last frame's texts the I-beam shows over: like the regions, they
    // are re-noted every layout (h4-readout's runs note theirs post-pass).
    input.texts.clear();
    for d in &input.declared {
        let mut r = d.region;
        if !d.keep_box {
            let data = clay.get_element_data(d.id);
            if !data.found {
                continue;
            }
            r.r#box = data.boundingBox;
        }
        if let Some(s) = d.clip {
            // The container's padding box: it clips its content inside its border.
            let c = clay.get_element_data(mv_clay::id(crate::scroll::SCROLLER_IDS[s], 0));
            if c.found {
                r.clip = Some(crate::scroll::grow_box(c.boundingBox, -scroll[s].border));
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

/// Routes one input fact to the shell state.
///
/// `time` is when the fact happened, on the app's clock, 0 for now. The
/// fact's own time stands in `input.time` while it is handled and the
/// previous value is restored after, including on the early returns below.
pub fn handle_input(app: &mut App, input: Input, time: Time) {
    let outer = app.input.time;
    app.input.time = if time != 0 { time } else { clock_now(app) };
    match input {
        Input::Pointer(event) => {
            if crate::trace() {
                println!("input {event:?}");
            }
            app.input.over = !event.left;
            if !event.left {
                if event.pos != app.input.pointer {
                    // A real move re-arms hover: a pick's hold lasts only
                    // until the pointer moves, as Chromium re-hovers then.
                    crate::settings::pointer_moved(app);
                }
                app.input.pointer = event.pos;
            }
            // A drag the plane holds keeps panning under capture, wherever
            // the pointer is, as `setPointerCapture` keeps the hook's moves
            // coming; a scrub the value box holds keeps scrubbing the same
            // way; anything else retargets, and an unpressed move over the
            // plane runs the handles' hover dispatch after it (h7-plane:
            // the retarget's highlight syncs run first, as the DOM's
            // leave-then-enter does, then the plane's hover order).
            if app.input.pressed.kind == HitKind::Plane && !event.left {
                crate::plane_input::plane_move(app);
            } else if crate::scrub_value::captured(app) && !event.left {
                crate::scrub_value::pointer_moved(app);
            } else {
                retarget(app);
                crate::plane_input::plane_hover(app);
                crate::plane_input::plane_cursor(app);
            }
        }
        Input::Button(event) => {
            if crate::trace() {
                println!("input {event:?}");
            }
            if event.pos != app.input.pointer {
                crate::settings::pointer_moved(app);
            }
            app.input.pointer = event.pos;
            app.input.over = true;
            // Only the primary button acts, as the page's pointer handlers check `button === 0`.
            if event.button != 1 {
                app.input.time = outer;
                return;
            }
            if event.down {
                press(app);
                crate::settings::press_down(app);
                crate::transform_row::press_down(app);
                if app.input.pressed.kind == HitKind::Plane {
                    // SDL's numbering starts at 1; the camera takes the DOM's,
                    // where the primary button is 0 (only 0 drags, there).
                    crate::plane_input::plane_down(
                        app,
                        crate::camera::PointerButton::Mouse(u32::from(event.button) - 1),
                    );
                }
            } else {
                let down = app.input.pressed;
                if down.kind == HitKind::Plane {
                    crate::plane_input::plane_up(app);
                }
                // A box press ends: a scrub flushes or opens the field, a
                // tap may reset or move the caret. Before the release clears
                // the press the down hit names.
                crate::scrub_value::release_up(app, down, event.pos, event.clicks);
                release(app);
                // The release cleared the press the cursor rule read: over
                // the plane the hover rule's cursor wins back (h7-plane: a
                // release onto a handle shows the open hand, as the press's
                // `hovering` never cleared).
                if down.kind == HitKind::Plane {
                    crate::plane_input::plane_cursor(app);
                }
                crate::settings::click(app, down);
                crate::selector::click(app, down);
                // The transform buttons' click (h5-buttons: the press started
                // and the release lands on the toggle, reset or Reset all).
                crate::transform_row::click(app, down);
                // The overlay button's click (the press started and the
                // release lands on it, as the button's `onClick` needs both):
                // the camera back to its defaults, flushed.
                if down.kind == HitKind::ReturnToCenter
                    && app.input.hover.kind == HitKind::ReturnToCenter
                {
                    app.reset_view();
                }
            }
        }
        Input::Wheel(event) => {
            if crate::trace() {
                println!("input {event:?}");
            }
            // A wheel carries the pointer's position: it is in the window,
            // as a motion would have it (without this a wheel before any
            // motion finds no hover and the plane never zooms).
            app.input.pointer = event.pos;
            app.input.over = true;
            retarget(app);
            // A wheel over a value box steps it (ignored while typed into,
            // when the page may scroll instead).
            if app.input.hover.kind == HitKind::TransformField {
                crate::scrub_value::wheel_over(app, event.delta[1]);
            }
            // The plane zooms about the pointer; a held drag keeps the hover
            // on the plane through capture, so a wheel mid-drag zooms, as the
            // hook's wheel and drag run on one loop.
            if app.input.hover.kind == HitKind::Plane {
                crate::plane_input::plane_wheel(app, event.delta);
            }
        }
        Input::Pinch(event) => {
            if crate::trace() {
                println!("input pinch {:.4}", event.scale);
            }
        }
        Input::Key(event) => {
            if crate::trace() {
                println!("input {event:?}");
            }
            app.input.mods = event.mods;
            // The page milestones route their keys from here.
            if !crate::settings::key(app, &event) && !crate::selector::key(app, &event) {
                crate::transform_row::key(app, &event);
            }
            // A modifier anywhere re-reads a held scrub's mode live
            // mid-drag, as the window key hook does (down and up, never a
            // repeat, which cannot change the mode; acts only then).
            if !event.repeat {
                crate::scrub_value::modifier_key(app);
            }
        }
        Input::Text(event) => {
            if crate::trace() {
                println!("input text {:?}", event.text);
            }
            // The focused transform field takes the typed text (h4-rows).
            crate::transform_row::text(app, &event.text);
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
            } else if app.input.pressed.kind != HitKind::None {
                // The window's focus left: a held press ends. On the plane
                // that ends the drag first, as `pointercancel` ends the
                // hook's (its samples are stale by now, so no glide starts).
                if app.input.pressed.kind == HitKind::Plane {
                    crate::plane_input::plane_up(app);
                }
                // A held scrub ends too, flushing when it dragged, as the
                // window release stops the box's press.
                crate::scrub_value::focus_lost(app);
                app.input.pressed = Hit::default();
                retarget(app);
            }
        }
        Input::Scheme(event) => {
            app.system_scheme = if event.dark {
                mv_theme::Scheme::Dark
            } else {
                mv_theme::Scheme::Light
            };
            // The theme state tracks the system, as `systemColorSchemeAtom`
            // tracks matchMedia; stored picks are untouched.
            app.theme.system = if event.dark {
                crate::theme_state::SystemScheme::Dark
            } else {
                crate::theme_state::SystemScheme::Light
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
            // The page milestones route the model to their state from here.
            if crate::trace() {
                println!("input {event:?}");
            }
        }
    }
    app.input.time = outer;
}

/// Recomputes hover and cursor after the pointer or targets changed. While
/// the settings menu holds hover off after a pick, the hover stays dropped
/// (only a pointer move re-arms it), matching Chromium, which does not
/// re-hover without a mouse move after the re-render.
pub fn retarget(app: &mut App) {
    let over = app.input.over;
    let pointer = app.input.pointer;
    let pressed = app.input.pressed;
    let mut hit = if over && !app.settings.hover_held {
        hit_at(app.input.regions.as_slice(), pointer)
    } else {
        Hit::default()
    };
    if captures(pressed.kind) || crate::scrub_value::captured(app) {
        hit = pressed;
    }
    if hit != app.input.hover {
        let before = app.input.hover;
        app.input.hover = hit;
        // The highlight follows the pointer across the transform rows (leave
        // then enter, as the DOM fires them); h4-rows owns this sync.
        crate::transform_row::sync_hover(app, before, hit);
        // The highlight model follows the hover: terms and rows light (or
        // drop) as the pointer moves across them (h4-readout; the row kinds
        // map in `readout::hit_param` too). Only on a change: re-stamping
        // the hover every frame would keep it ahead of the focus forever,
        // and a Tabbed focus would never take the lead (h5-buttons).
        crate::readout::sync_hover(app);
        app.dirty = true;
    }
    update_cursor(app);
}

/// Picks the cursor from pressed/hover/text.
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

/// Captures the target under the pointer. The page milestones route the press
/// to their handlers from here.
pub fn press(app: &mut App) {
    let pointer = app.input.pointer;
    app.input.pressed = hit_at(app.input.regions.as_slice(), pointer);
    app.input.focus = Hit::default();
    app.input.focus_visible = false;
    app.dirty = true;
    retarget(app);
}

/// Releases the capture.
pub fn release(app: &mut App) {
    app.input.pressed = Hit::default();
    app.dirty = true;
    retarget(app);
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
        set_focus(app, Hit::default(), false);
    } else {
        let hit = app.input.stops[next as usize];
        set_focus(app, hit, true);
    }
    if crate::trace() {
        // The harness's focus line (`tools/run loop` reads it): the shape
        // `Hit{kind = None, index = 0}`, not Rust's `{:?}` debug, whose
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

/// Moves focus and marks dirty. The page milestones notify their handlers of
/// the move from here.
pub fn set_focus(app: &mut App, hit: Hit, visible: bool) {
    let before = app.input.focus;
    app.input.focus = hit;
    app.input.focus_visible = visible;
    app.input.keyed = visible;
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
/// reads the raw union as SDL queues each event and the `Unknown`
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
        eprintln!("math-viz2-rust: input: no event watch: a trackpad pinch never becomes a fact");
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

    /// Serializes the tests that push to and peep from SDL's global event
    /// queue: parallel `cargo test` threads otherwise race (a peep takes a
    /// sibling test's event, or a push lands after the peep), failing about
    /// 2 runs in 8. The shape follows `mv_clay::tests_lock`, but this is its
    /// own queue and its own lock.
    static SDL_QUEUE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

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
    /// a second thread must be visible where the routing runs.
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
    /// the arm itself re-reads live window sizes, so its run waits for a real
    /// display-scale event under the routing.
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
                );
            }
            other => {
                panic!("a display-scale change decodes as Unknown, got {other:?}");
            }
        }
    }

    /// A painted target keeps its declared box through the resolve: the
    /// overlay button's box (no Clay element resolves it) lands in the
    /// regions as declared, with its stop.
    #[test]
    fn kept_region_resolves_to_its_declared_box() {
        let _guard = mv_clay::tests_lock();
        let mut app = crate::fixture::headless_app();
        crate::ui::init_clay(&mut app);
        let button = mv_sys::clay::BoundingBox {
            x: 1359.5,
            y: 780.5,
            width: 36.0,
            height: 36.0,
        };
        region_kept(
            &mut app,
            Hit {
                kind: HitKind::ReturnToCenter,
                index: 0,
            },
            button,
            Cursor::Pointer,
            Cursor::Pointer,
            true,
            crate::ui::corner_radius_all(4.0),
        );
        collect_regions(&mut app);
        assert_eq!(app.input.regions.len(), 1);
        let region = app.input.regions[0];
        assert_eq!(region.hit.kind, HitKind::ReturnToCenter);
        assert_eq!(
            [
                region.r#box.x,
                region.r#box.y,
                region.r#box.width,
                region.r#box.height
            ],
            [1359.5, 780.5, 36.0, 36.0]
        );
        assert_eq!(app.input.stops, vec![region.hit]);
        crate::ui::release_clay(&mut app);
    }
}
