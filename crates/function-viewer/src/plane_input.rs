//! The Cartesian plane's input wiring: SDL pointer and wheel facts routed by
//! the plane's hit region into the camera, and the camera's frame step with
//! the URL write on settle.
//!
//! A port of the wiring in `CartesianPlane.tsx` (`flushCamera` as `onSettle`,
//! the overlay button's flush-both-to-default click) and `useCamera.ts`'s
//! event handlers (`onWheel`, `onPointerDown/Move/Up`, `offsetFromCenter`) at
//! math-viz2 `c409aad`, plus the plane handles' hover and drag dispatch
//! (`onHover`/`onHoverEnd` order, the grab offset, `dragging`, the scrub
//! modes, no pan while dragging, the release flush) and the canvas cursor
//! (`data-cursor`: pan, grab-hover, grab-active) at `251c080`. The camera
//! itself (`camera.rs`, h2-p-camera) and the URL params (`url.rs`, h2-p-url)
//! own their state machines; this module only routes between them, the input
//! layer and the frame loop.
//!
//! Event flow: `input.rs` calls [`plane_down`]/[`plane_move`]/[`plane_up`]/
//! [`plane_wheel`] when the press or hover names [`HitKind::Plane`]
//! (h2-plane's canvas region). [`frame_step`] runs once per frame from
//! `app_loop::iterate`, before the draw: while the camera animates it steps
//! the camera with the frame's `dt`, publishes center/scale into the params
//! (debounced URL writes, as the hook's `setCenter`/`setScale`), and when the
//! step reports rest while settled it flushes both params (the hook's
//! `onSettle`); then every param's `step` (debounce) and `sync` (an external
//! hash edit flows into the live values) run, the camera adopts outside
//! changes while idle, and on the web the changed query mirrors into the page
//! URL. [`external_url_synced`] is the same sync-plus-adopt half for the
//! `UserCode::Hash` arm (an edit in the address bar, back/forward).
//!
//! Region-box convention (for h2-plane, which declares the region): the Plane
//! region's box is the canvas CONTENT box, inside its 1 px border, in window
//! logical px — the anchor math below subtracts no border itself, as
//! `offsetFromCenter` already removed `clientLeft`/`clientTop`.

use crate::app::App;
use crate::app_loop::{MS, Time};
use crate::camera::{self, PointerButton};
use crate::grid::{GridLayout, layout_grid};
use crate::input::{Cursor, Hit, HitKind, region_kept, region_of};
use crate::plane_geometry::{HitTarget, from_px, hit_test};
use crate::state::TransformParam;
use crate::url::{QueryParam, QueryParamDef, default_equals};

/// The SDL mouse pointer's id in camera coordinates. `input.rs` decodes only
/// mouse buttons (pen/touch arrive as no facts yet), so the plane holds one
/// pointer; the camera ignores a second id while one drags, as the hook does.
pub const MOUSE_POINTER_ID: u64 = 1;

/// `centerParam` (`queryParams.ts`): `toFixed(4)`, default `[0, 0]`, debounced
/// 200 ms. Parse and serialise delegate to the camera's pair, which pin
/// `Number` semantics and the four-decimal shape.
pub static CENTER_DEF: QueryParamDef<[f64; 2]> = QueryParamDef {
    key: "center",
    default: camera::DEFAULT_CENTER,
    debounce_ms: 200,
    parse: parse_center_param,
    serialize: serialize_center_param,
    equals: default_equals,
};

/// `scaleParam` (`queryParams.ts`): `String(Number(toPrecision(3)))`, default
/// 0.5, range 0.01..=100, debounced 200 ms.
pub static SCALE_DEF: QueryParamDef<f64> = QueryParamDef {
    key: "scale",
    default: camera::DEFAULT_SCALE,
    debounce_ms: 200,
    parse: parse_scale_param,
    serialize: serialize_scale_param,
    equals: default_equals,
};

fn parse_center_param(raw: Option<&str>) -> Option<[f64; 2]> {
    camera::parse_center(raw?)
}

#[allow(clippy::unnecessary_wraps)]
fn serialize_center_param(value: &[f64; 2]) -> Option<String> {
    Some(camera::format_center(*value))
}

fn parse_scale_param(raw: Option<&str>) -> Option<f64> {
    camera::parse_scale(raw?)
}

#[allow(clippy::unnecessary_wraps)]
fn serialize_scale_param(value: &f64) -> Option<String> {
    Some(camera::format_scale(*value))
}

/// The fact's time in ms, on the app's clock. `input.time` stands in while a
/// fact is handled (`input.rs`); the camera keeps event times, not handling
/// times, as the hook's samples keep `timeStamp`.
fn fact_ms(app: &App) -> f64 {
    app.input.time as f64 / MS as f64
}

/// The frame's whole milliseconds, for the params' debounce clock (which runs
/// in whole ms; the truncation matches between `set` and `step`, so a
/// deadline armed for `debounce_ms` fires exactly there).
fn frame_ms(now: Time) -> u64 {
    (now / MS).max(0) as u64
}

/// The pointer in camera px (window logical px; the drag only ever reads
/// differences, so the origin cancels, as `[clientX, clientY]` does there).
fn pointer_at(app: &App) -> [f64; 2] {
    [
        f64::from(app.input.pointer[0]),
        f64::from(app.input.pointer[1]),
    ]
}

/// Adopt externally set values (URL back/forward, the overlay reset) while the
/// camera is still. This runs before the pointer handlers in the same frame —
/// the hook reads its props at the top of `onWheel`/`onPointerDown`, and the
/// port moved the adopt out of `Camera` (h2-p-camera review finding 5) — so a
/// gesture starts from what the URL shows, never from a stale camera.
fn adopt_camera(app: &mut App) {
    let center = *app.center_param.get();
    let scale = *app.scale_param.get();
    app.camera.adopt_if_idle(center, scale);
}

/// The canvas content center in window logical px, as `offsetFromCenter`
/// measures it: the content-box origin plus `floor(size / 2)`. Falls back to
/// the window center when the plane declared no region (a layout mid-flight);
/// routing only reaches here through a Plane hit, which a region produced.
fn canvas_content_center(app: &App) -> [f64; 2] {
    let hit = if app.input.pressed.kind == HitKind::Plane {
        app.input.pressed
    } else {
        app.input.hover
    };
    let (region, found) = region_of(app, hit);
    if !found {
        return [f64::from(app.size[0]) / 2.0, f64::from(app.size[1]) / 2.0];
    }
    let bounds = region.r#box;
    [
        f64::from(bounds.x) + f64::from(bounds.width / 2.0).floor(),
        f64::from(bounds.y) + f64::from(bounds.height / 2.0).floor(),
    ]
}

/// Seed the camera and both params from the page query at start (`--query`
/// natively, the hash on the web). Runs once in each `build()` after `app.url`
/// is set; `App::zeroed` already holds the URL defaults for headless tests.
pub fn init_camera_from_url(app: &mut App) {
    app.center_param = QueryParam::new(&CENTER_DEF, app.url.query());
    app.scale_param = QueryParam::new(&SCALE_DEF, app.url.query());
    let center = *app.center_param.get();
    let scale = *app.scale_param.get();
    app.camera = camera::Camera::new(center, scale);
}

/// Declares the plane canvas's kept region: the camera's hit target over the
/// canvas content box, as [`canvas_content_center`] reads it. Runs after the
/// layout (the box resolves from the Clay context through `PLANE_ID`) and
/// before the overlay button's region, so the button wins their overlap at
/// equal z. h2-plane's slot declares only the button; this side owns the
/// canvas. Not a focus stop (the canvas takes no keyboard focus).
pub fn declare_plane_region(app: &mut App) {
    let data = app
        .clay
        .as_ref()
        .expect("clay context initialised")
        .get_element_data(mv_clay::id(crate::plane::PLANE_ID, 0));
    if !data.found {
        return;
    }
    let content = crate::plane::canvas_content(&data.boundingBox);
    region_kept(
        app,
        Hit {
            kind: HitKind::Plane,
            index: 0,
        },
        content,
        // The canvas's own cursors (`CartesianPlain.module.css`): the
        // crosshair at rest, the closed hand while a press holds. The grab
        // over a handle comes from `plane_cursor`, not the region, because
        // the diamond-to-square jump leaves the cursor at pan while a
        // handle is still under the pointer (h7-plane's handle-hover-move).
        Cursor::Crosshair,
        Cursor::Grabbing,
        false,
        crate::ui::corner_radius_all(0.0),
    );
}

/// A plane handle: the diamond shifting h/k, the square scaling a/b (the
/// `shiftParams`/`scaleParams` lists at `251c080`, in build order so the
/// square wins a tie through `hit_test`'s last-index rule). Carried as
/// `Option<HandleId>` in `App::hovered`/`App::dragged`, so no press maps to
/// no handle rather than silently to the shift pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandleId {
    /// The diamond: sets h and k.
    Shift,
    /// The square: sets a and b.
    Scale,
    /// P's marker on the graph: sets p.
    PointP,
}

impl HandleId {
    /// The handle at the hit-test index (shift 0, scale 1); `None` for any
    /// other index, which the two-handle test never reports.
    fn from_index(index: usize) -> Option<HandleId> {
        match index {
            0 => Some(HandleId::Shift),
            1 => Some(HandleId::Scale),
            _ => None,
        }
    }

    /// The handle's index in the built pair (its hit-test slot).
    fn index(self) -> usize {
        match self {
            HandleId::Shift => 0,
            HandleId::Scale => 1,
            HandleId::PointP => 0,
        }
    }
}

/// The shift handle's two lit parameters, then the scale handle's (the
/// `shiftParams`/`scaleParams` lists at `251c080`).
fn handle_params(id: HandleId) -> &'static [TransformParam] {
    match id {
        HandleId::Shift => &[TransformParam::H, TransformParam::K],
        HandleId::Scale => &[TransformParam::A, TransformParam::B],
        HandleId::PointP => &[],
    }
}

/// The plane's content box in window logical px, from the last frame's kept
/// region (the box only moves on a resize, so the input turn reads it).
fn plane_content(app: &App) -> Option<mv_sys::clay::BoundingBox> {
    let (region, found) = region_of(
        app,
        Hit {
            kind: HitKind::Plane,
            index: 0,
        },
    );
    found.then_some(region.r#box)
}

/// The pointer in canvas px (the hook's `offsetX`/`offsetY`, past the 1 px
/// border onto the content box). Past the canvas's edge the values run
/// negative or past the size, and `from_px` still maps them, so a drag
/// past the edge keeps following (the capture retargets the moves).
fn canvas_at(app: &App) -> Option<[f64; 2]> {
    let content = plane_content(app)?;
    Some([
        f64::from(app.input.pointer[0]) - f64::from(content.x),
        f64::from(app.input.pointer[1]) - f64::from(content.y),
    ])
}

/// The grid the hit test and `from_px` read: laid out for the live camera
/// over the last frame's content box (the paint lays out the same grid for
/// the same values, so a press hit-tests what the frame paints).
fn plane_grid(app: &App) -> Option<GridLayout> {
    let content = plane_content(app)?;
    Some(layout_grid(
        app.camera.center,
        app.camera.scale,
        f64::from(content.width),
        f64::from(content.height),
        crate::plane::LABEL_MARGIN,
    ))
}

/// Q's hover source (`onHoverPoint` in `FunctionViewer.tsx`'s `Plot`,
/// through `useMouseHandles`' move and leave, gated by `CartesianPlane` on
/// `dragging || panning`): the quantized x under the pointer when the move
/// hits no handle and no handle drags and the plane isn't panning, else
/// cleared. Runs on every move, even when the hovered index didn't change.
pub fn hover_q(app: &mut App, index: Option<HandleId>, point: crate::plot::PlotPoint) {
    let next = if index.is_none() && !app.dragging && app.input.pressed.kind != HitKind::Plane {
        Some(crate::point_format::quantize(point.x))
    } else {
        None
    };
    if app.state.q_x != next {
        app.state.q_x = next;
        app.dirty = true;
    }
}

/// Q cleared: the pointer left the canvas, or a pan started. No-op when
/// already clear, so quiet frames stay clean.
pub fn clear_q(app: &mut App) {
    if app.state.q_x.is_some() {
        app.state.q_x = None;
        app.dirty = true;
    }
}

/// The hover dispatch for an unpressed move over the plane
/// (`CartesianPlane.tsx`'s `onPointerMove`, through `useMouseHandles`): the
/// hit handle's hover (its two params lit) first, then, when the hovered
/// index changed, the previously hovered handle's hover-end (its params off
/// only when the stored list still equals them). The order is the source's,
/// so a jump straight from the diamond to the square lights a and b while h
/// and k drop. The hook used to set its own `hovering` state between the two
/// handlers, which left it false across that jump; `ea29aa9` derives it
/// after both, and this port reads `app.hovered` (the `lastHoveredIndex` ref)
/// in the cursor rule instead. Same-index moves and empty moves change
/// nothing, so the frame stays clean.
pub fn plane_hover(app: &mut App) {
    if app.input.hover.kind != HitKind::Plane {
        clear_q(app);
        return;
    }
    let (Some(grid), Some(at)) = (plane_grid(app), canvas_at(app)) else {
        clear_q(app);
        return;
    };
    let handles = crate::page::build_handles(app);
    let markers = crate::page::build_markers(app);
    let index = match hit_test(&grid, &handles, &markers, at) {
        Some(HitTarget::Handle(i)) => HandleId::from_index(i),
        Some(HitTarget::Marker(_)) => Some(HandleId::PointP),
        None => None,
    };
    hover_q(app, index, from_px(&grid, at));
    if index == app.hovered {
        return;
    }
    // The source's statement order: the hit handle's hover, then the previous
    // index's hover-end when the index changed.
    if let Some(next) = index {
        app.state.set_hover(handle_params(next), true);
    }
    if let Some(last) = app.hovered {
        app.state.set_hover(handle_params(last), false);
    }
    app.hovered = index;
    app.dirty = true;
}

/// The canvas's cursor from the hover and press state
/// (`CartesianPlane.tsx`'s rule: `dragging || panning` grabs closed, else
/// `hovering` grabs open, else the crosshair). The press on the plane or a
/// handle is the `panning` half (the hook sets it on any canvas press and
/// clears it on release); `app.dragging` is the `dragging` half. The open
/// hand reads `app.hovered`, the hook's `hovering = hoveredHandleIndex !== -1`
/// (`ea29aa9`: the state is gone, the index is the value). Only while the
/// pointer or press names the plane: anywhere else the regions' cursors stand.
/// Runs after the dispatch and the retarget, so its cursor wins. `pub` for the
/// input layer's unpressed-move path.
pub fn plane_cursor(app: &mut App) {
    let on_plane =
        app.input.hover.kind == HitKind::Plane || app.input.pressed.kind == HitKind::Plane;
    if !on_plane {
        return;
    }
    let cursor = if app.dragging || app.input.pressed.kind == HitKind::Plane {
        Cursor::Grabbing
    } else if app.hovered.is_some() {
        Cursor::Grab
    } else {
        Cursor::Crosshair
    };
    crate::input::set_cursor(app, cursor);
}

/// A primary press on the plane: adopt outside values first (above), then
/// the drag starts. Catching the plane ends a glide while a zoom carries on,
/// as the hook's `onPointerDown` does. The press always reaches the camera
/// (the hook's `panning`, which also shows the closed hand); a press on a
/// handle additionally starts the handle drag and no pan: the grab offset
/// in plane units, `dragging` on, and both params' scrub modes coarse (the
/// rows light and their rulers come out). Needs no draw unless a drag
/// started.
pub fn plane_down(app: &mut App, button: PointerButton) {
    adopt_camera(app);
    let at = pointer_at(app);
    app.camera
        .pointer_down(MOUSE_POINTER_ID, at, fact_ms(app), button);
    // The press's own hit test (a press on a handle starts a drag and no
    // pan; a miss pans as before). The press touches no hover state (the
    // hook's press sets only the grab, the drag and the modes; the hover
    // follows from the moves, as the React handler does): the dispatch in
    // the drag path picks the hover up from the first move.
    if let (Some(grid), Some(px)) = (plane_grid(app), canvas_at(app)) {
        let handles = crate::page::build_handles(app);
        let markers = crate::page::build_markers(app);
        let hit = hit_test(&grid, &handles, &markers, px);
        match hit {
            Some(HitTarget::Handle(i)) => {
                if let Some(index) = HandleId::from_index(i) {
                    let point = from_px(&grid, px);
                    let anchor = handles[index.index()].at;
                    app.grab = [anchor.x - point.x, anchor.y - point.y];
                    app.dragged = Some(index);
                    app.dragging = true;
                    let params = handle_params(index);
                    app.state.scrub_mode[params[0]] = Some(crate::scrub::ScrubMode::Coarse);
                    app.state.scrub_mode[params[1]] = Some(crate::scrub::ScrubMode::Coarse);
                    app.dirty = true;
                }
            }
            Some(HitTarget::Marker(_)) => {
                let point = from_px(&grid, px);
                let p = *app.p_param.get();
                app.grab = [p - point.x, 0.0];
                app.state.p_dragging = true;
                app.dragging = true;
                app.dragged = Some(HandleId::PointP);
                app.dirty = true;
            }
            None => {}
        }
    }
    plane_cursor(app);
}

/// A move while the plane holds the pointer (capture, as `setPointerCapture`
/// keeps the hook's moves coming): a handle drag sets through its params,
/// else the plane pans at once. The camera never sees a handle drag's moves
/// (the hook returns early while `dragging`: no pan, no samples). Marks
/// dirty when the drag or the center changed.
pub fn plane_move(app: &mut App) {
    if app.dragging {
        // The drag's sets first (the canvas's own move listener), then the
        // hover dispatch (the React prop, which bubbles after), exactly as
        // the hook's two listeners run them.
        drag_move(app);
        plane_hover(app);
        plane_cursor(app);
        return;
    }
    let before = app.camera.center;
    let at = pointer_at(app);
    app.camera.pointer_move(MOUSE_POINTER_ID, at, fact_ms(app));
    if app.camera.center != before {
        app.dirty = true;
    }
    // The press names the plane, so this move pans: Q stays null while
    // panning (`CartesianPlane`'s gate), and a pan started under a lingering
    // Q clears it.
    clear_q(app);
    crate::input::retarget(app);
    // The press names the plane (the `panning` half of the cursor rule);
    // the regions' cursors already agree, and this keeps headless regions
    // (which declare no cursors) on the same cursor.
    plane_cursor(app);
}

/// One held step of a handle drag (`onDrag`): the shift handle sets
/// `h = x + grab.x`, `k = y + grab.y`; the scale handle sets
/// `b = (x + grab.x − h) / unitX`, `a = (y + grab.y − k) / g(unitX)`
/// (`g(unitX)` is 1 for all four functions at this hash; the port divides
/// by the formula, not the value). Through the params, so the writes stay
/// debounced as the hook's `setH`/`setK`/`setA`/`setB` do.
fn drag_move(app: &mut App) {
    let (Some(grid), Some(px)) = (plane_grid(app), canvas_at(app)) else {
        return;
    };
    let point = from_px(&grid, px);
    let [gx, gy] = app.grab;
    let now_ms = (app.input.time / MS).max(0) as u64;
    match app.dragged {
        Some(HandleId::Scale) => {
            let slug = *app.func_param.get();
            let unit = crate::functions::unit_x(slug);
            let h = *app.h_param.get();
            let k = *app.k_param.get();
            app.b_param.set((point.x + gx - h) / unit, now_ms);
            app.a_param.set(
                (point.y + gy - k) / crate::functions::base(slug)(unit),
                now_ms,
            );
        }
        Some(HandleId::Shift) => {
            app.h_param.set(point.x + gx, now_ms);
            app.k_param.set(point.y + gy, now_ms);
        }
        Some(HandleId::PointP) => {
            let gx = app.grab[0];
            app.p_param
                .set(crate::point_format::quantize(point.x + gx), now_ms);
        }
        // No press without a drag: unreachable while `dragging` holds (set
        // together in `plane_down`), and nothing may flush here.
        None => {}
    }
    app.dirty = true;
}

/// A wheel notch about the cursor: `delta` in DOM px (`input.rs` already
/// converted SDL's units; the hook reads raw `deltaY` with no `deltaMode`
/// branch). `dx` is passed and ignored, as the hook reads only `deltaY`.
/// Marks dirty only when a zoom starts (at a scale limit nothing animates).
pub fn plane_wheel(app: &mut App, delta: [f32; 2]) {
    adopt_camera(app);
    let center = canvas_content_center(app);
    let at = [
        f64::from(app.input.pointer[0]) - center[0],
        f64::from(app.input.pointer[1]) - center[1],
    ];
    app.camera
        .wheel(f64::from(delta[0]), f64::from(delta[1]), at);
    if app.camera.animating() {
        app.dirty = true;
    }
}

/// The held pointer let go (or the window lost focus mid-drag, as
/// `pointercancel` ends the hook's drag): the next frame glides on, or
/// settles and writes the URL. A handle drag ends first: `dragging` off,
/// both params' modes cleared, both flushed (the URL written at once).
/// The hover stays where the last move left it (no move fires on release),
/// so the cursor and the lit list fall back to the pointer's position.
pub fn plane_up(app: &mut App) {
    app.camera.pointer_up(MOUSE_POINTER_ID, fact_ms(app));
    if app.camera.animating() {
        app.dirty = true;
    }
    if app.dragging {
        app.dragging = false;
        match app.dragged.take() {
            Some(HandleId::Scale) => {
                app.state.scrub_mode[TransformParam::A] = None;
                app.state.scrub_mode[TransformParam::B] = None;
                app.a_param.flush(&mut app.url);
                app.b_param.flush(&mut app.url);
            }
            Some(HandleId::Shift) => {
                app.state.scrub_mode[TransformParam::H] = None;
                app.state.scrub_mode[TransformParam::K] = None;
                app.h_param.flush(&mut app.url);
                app.k_param.flush(&mut app.url);
            }
            Some(HandleId::PointP) => {
                app.state.p_dragging = false;
                app.p_param.flush(&mut app.url);
            }
            // No press without a drag: unreachable while `dragging` holds
            // (set together in `plane_down`), and nothing may flush here.
            None => {}
        }
        app.dirty = true;
    }
    plane_cursor(app);
}

/// Copy an external query change into the param live values and adopt it into
/// a still camera (the hook's `syncFromUrl` plus its next gesture's
/// `adoptIfIdle`, at once). Idempotent: a second call with no new hash finds
/// every key's text unchanged and reports false. Returns whether anything was
/// adopted.
pub fn external_url_synced(app: &mut App) -> bool {
    let center = app.center_param.sync(&app.url);
    let scale = app.scale_param.sync(&app.url);
    if !(center || scale) {
        return false;
    }
    adopt_camera(app);
    app.dirty = true;
    true
}

/// Whether the next frame publishes without stepping the camera: the turn
/// that armed motion draws the gesture's own change, and the loop steps from
/// the turn after — as the hook's handlers arm `requestAnimationFrame` for a
/// later frame instead of stepping in the event's own task. The scripted
/// harnesses set this on the turns whose facts can arm motion (a button
/// press or release, a wheel); moves never set it, so a glide under a drag
/// keeps stepping, and production never sets it (its events and frames are
/// separate turns already). Always consumed by the next [`frame_step`], so a
/// hold never leaks into a later turn.
static HOLD_CAMERA_STEP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Holds the camera's next step for this turn's publish-and-draw (see
/// [`HOLD_CAMERA_STEP`]). The scripted frame sites call this on arming turns:
/// the native harness when its facts hold a button event or a wheel, the web
/// harness through `mv_hold_step` after the same verbs.
pub(crate) fn hold_camera_step() {
    HOLD_CAMERA_STEP.store(true, std::sync::atomic::Ordering::SeqCst);
}

/// One frame on the app's clock: the camera's step, the params' debounce and
/// external sync, the web hash mirror. Runs in `app_loop::iterate` before the
/// draw, so the frame draws what the camera moved to.
pub fn frame_step(app: &mut App, now: Time) {
    let now_ms = frame_ms(now);
    // An arming turn's hold, if any, is consumed here whether or not the
    // camera flies, so it never leaks into a later turn.
    let held = HOLD_CAMERA_STEP.swap(false, std::sync::atomic::Ordering::SeqCst);
    if app.camera.animating() {
        // The first frame after idle carries no measurable dt; the camera
        // holds its `FIRST_FRAME_DT` for exactly that frame.
        let dt_ms = if app.camera_frame_at == 0 {
            0.0
        } else {
            (now - app.camera_frame_at) as f64 / MS as f64
        };
        // A held turn publishes (below) but does not step: the gesture's own
        // change draws now; the loop's motion starts next turn.
        let still_going = if held { true } else { app.camera.step(dt_ms) };
        app.camera_frame_at = now;
        // The hook's `publish`: the handlers pan the camera at once while the
        // frame writes to React only what changed since the last publish, so
        // however many moves land in one frame, the live values update once.
        // (An exact compare is safe: both sides are copies of the same
        // values, as the camera rig's own write detection is.)
        if app.camera.center != *app.center_param.get()
            || app.camera.scale != *app.scale_param.get()
        {
            let center = app.camera.center;
            let scale = app.camera.scale;
            app.center_param.set(center, now_ms);
            app.scale_param.set(scale, now_ms);
            app.dirty = true;
        }
        if !still_going && app.camera.settled() {
            // The hook's `onSettle`: the camera came to rest, flush both.
            app.center_param.flush(&mut app.url);
            app.scale_param.flush(&mut app.url);
        }
    } else {
        app.camera_frame_at = 0;
    }
    // One param's `setTimeout` going off, then an outside edit flowing in.
    // Each `step` reports its own write; `sync` never touches the query, so
    // the two flags are the whole "the query changed" story with no
    // string compare per frame.
    let center_wrote = app.center_param.step(&mut app.url, now_ms);
    let scale_wrote = app.scale_param.step(&mut app.url, now_ms);
    let query_changed = center_wrote || scale_wrote;
    external_url_synced(app);
    if query_changed {
        #[cfg(target_os = "emscripten")]
        crate::platform_web::platform_write_hash(&app.url.full_hash());
    }
}

impl App {
    /// The overlay's return-to-centre click (`HtmlOverlay.tsx`): both params
    /// to their defaults with an immediate URL write (the `{ flush: true }`
    /// pair), the still camera following at once. While the camera moves the
    /// adopt is a no-op and the gesture carries on from its own values, as
    /// the hook's `cur` does until its next gesture's adopt.
    pub fn reset_view(&mut self) {
        self.center_param
            .set_flushed(&mut self.url, camera::DEFAULT_CENTER);
        self.scale_param
            .set_flushed(&mut self.url, camera::DEFAULT_SCALE);
        let center = *self.center_param.get();
        let scale = *self.scale_param.get();
        self.camera.adopt_if_idle(center, scale);
        self.dirty = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::headless_app;
    use crate::input::{ButtonEvent, Hit, Input, PointerEvent, Region, WheelEvent, handle_input};

    /// The plane's region over a content box at `origin` sized `size`, in
    /// window logical px. Pushed straight into the last frame's targets: the
    /// layout that declares it is h2-plane's, which is still running. The
    /// cursors are the production region's (the crosshair at rest, the
    /// closed hand while a press holds).
    fn declare_plane(app: &mut App, origin: [f32; 2], size: [f32; 2]) {
        app.input.regions.push(Region {
            hit: Hit {
                kind: HitKind::Plane,
                index: 0,
            },
            r#box: mv_sys::clay::BoundingBox {
                x: origin[0],
                y: origin[1],
                width: size[0],
                height: size[1],
            },
            cursor: crate::input::Cursor::Crosshair,
            pressed: crate::input::Cursor::Grabbing,
            radius: mv_sys::clay::CornerRadius {
                topLeft: 0.0,
                topRight: 0.0,
                bottomLeft: 0.0,
                bottomRight: 0.0,
            },
            clip: None,
            z: 0,
        });
    }

    fn left_button(pos: [f32; 2], down: bool) -> Input {
        Input::Button(ButtonEvent {
            button: sdl3::sys::mouse::SDL_BUTTON_LEFT as u8,
            down,
            clicks: 1,
            pos,
        })
    }

    fn pointer_to(pos: [f32; 2]) -> Input {
        Input::Pointer(PointerEvent { pos, left: false })
    }

    fn wheel_at(delta: [f32; 2], pos: [f32; 2]) -> Input {
        Input::Wheel(WheelEvent { delta, pos })
    }

    /// One frame at `t` ms on the app's clock.
    fn frame(app: &mut App, t: i64) {
        frame_step(app, t * MS);
    }

    /// Frames every 16 ms until the camera rests (at most `cap`).
    fn run_until_still(app: &mut App, cap: usize) {
        let mut t = 0i64;
        for _ in 0..cap {
            if !app.camera.animating() {
                return;
            }
            t += 16;
            frame(app, t);
        }
        panic!("the camera is still animating after {cap} frames");
    }

    #[test]
    fn starts_from_the_page_query() {
        let mut app = headless_app();
        app.url = crate::url::Url::from_query("center=1.5000,-2.0000&scale=2.0000");
        init_camera_from_url(&mut app);
        assert_eq!(app.camera.center, [1.5, -2.0]);
        assert_eq!(app.camera.scale, 2.0);
        assert_eq!(*app.center_param.get(), [1.5, -2.0]);
        assert_eq!(*app.scale_param.get(), 2.0);
    }

    #[test]
    fn a_drag_starts_from_values_a_hash_edit_set_in_the_same_frame() {
        // The ordering h2-p-camera's review asked for: external values enter
        // through `adopt_if_idle` before the pointer handlers in the same
        // frame, as `adoptIfIdle` tops the hook's handlers.
        let mut app = headless_app();
        declare_plane(&mut app, [0.0, 0.0], [800.0, 600.0]);
        app.url = crate::url::Url::from_query("center=3.0000,4.0000&scale=2.0000");
        init_camera_from_url(&mut app);
        // Back/forward lands new values; the params sync but no frame adopts
        // them yet, so the camera is stale on purpose here.
        app.url
            .sync_from_hash("#function-viewer?center=5.0000,6.0000&scale=3.0000");
        assert!(app.center_param.sync(&app.url));
        assert!(app.scale_param.sync(&app.url));
        assert_eq!(app.camera.center, [3.0, 4.0]);
        // The press adopts first: at scale 3 a unit is 25/3 px wide, so a
        // 25 px drag right moves the center 3 units left from [5, 6].
        handle_input(&mut app, left_button([400.0, 300.0], true), 0);
        handle_input(&mut app, pointer_to([425.0, 300.0]), 16 * MS);
        assert_eq!(app.camera.center, [2.0, 6.0]);
    }

    #[test]
    fn a_wheel_starts_from_values_a_hash_edit_set_in_the_same_frame() {
        let mut app = headless_app();
        declare_plane(&mut app, [0.0, 0.0], [800.0, 600.0]);
        app.url = crate::url::Url::from_query("center=3.0000,4.0000&scale=2.0000");
        init_camera_from_url(&mut app);
        app.url
            .sync_from_hash("#function-viewer?center=0.0000,0.0000&scale=3.0000");
        assert!(app.center_param.sync(&app.url));
        assert!(app.scale_param.sync(&app.url));
        // At the content center the zoom holds the center: one notch from
        // scale 3, not from the stale 2.
        handle_input(&mut app, wheel_at([0.0, 100.0], [400.0, 300.0]), 0);
        run_until_still(&mut app, 1000);
        assert!((app.camera.scale - 3.0 * 0.1f64.exp()).abs() < 1e-12);
        assert_eq!(app.camera.center, [0.0, 0.0]);
    }

    #[test]
    fn a_drag_pans_and_writes_the_url_when_it_settles() {
        let mut app = headless_app();
        declare_plane(&mut app, [0.0, 0.0], [800.0, 600.0]);
        init_camera_from_url(&mut app);
        // At scale 0.5 a unit is 50 px wide: 50 px right, 20 px down.
        handle_input(&mut app, left_button([100.0, 100.0], true), 0);
        handle_input(&mut app, pointer_to([150.0, 120.0]), 16 * MS);
        frame(&mut app, 16);
        assert_eq!(app.camera.center, [-1.0, 0.4]);
        // Held still past the flick window, then released: no glide, and the
        // release's frame flushes the URL at once (no 200 ms wait).
        frame(&mut app, 200);
        assert_eq!(app.url.query(), "");
        handle_input(&mut app, left_button([150.0, 120.0], false), 200 * MS);
        frame(&mut app, 216);
        assert!(!app.camera.animating());
        assert_eq!(app.url.query(), "center=-1.0000,0.4000");
    }

    #[test]
    fn a_flick_glides_then_writes_the_url_once() {
        let mut app = headless_app();
        declare_plane(&mut app, [0.0, 0.0], [800.0, 600.0]);
        init_camera_from_url(&mut app);
        // 20 px a frame for five frames at scale 0.5, then go: fast enough
        // to glide.
        handle_input(&mut app, left_button([300.0, 200.0], true), 0);
        let mut t = 0i64;
        for i in 1..=5 {
            t += 16;
            handle_input(
                &mut app,
                pointer_to([300.0 + 20.0 * f64::from(i) as f32, 200.0]),
                t * MS,
            );
            frame(&mut app, t);
        }
        t += 16;
        handle_input(&mut app, left_button([400.0, 200.0], false), t * MS);
        run_until_still(&mut app, 1000);
        // The drag ended 200 px right (4 units at 50 px a unit) and the glide
        // carried it further left.
        assert!(app.camera.center[0] < -4.0);
        assert_eq!(app.camera.center[1], 0.0);
        let query = app.url.query().to_string();
        assert!(query.starts_with("center="), "{query}");
        // The scale never left its default, so it stays out of the URL.
        assert!(!query.contains("scale="), "{query}");
    }

    #[test]
    fn a_glide_keeps_the_loop_awake_until_it_settles() {
        // `schedule` counts a camera in flight like a fade in flight: without
        // it the draw consumes the step's dirty and the glide stalls after
        // one step (every flick frame but the first showed a frozen camera).
        let mut app = headless_app();
        // On the viewer route: the headless default is the welcome route,
        // whose playing loop would keep the loop awake past the settle.
        app.url.sync_from_hash("#function-viewer");
        declare_plane(&mut app, [0.0, 0.0], [800.0, 600.0]);
        init_camera_from_url(&mut app);
        handle_input(&mut app, left_button([300.0, 200.0], true), 0);
        let mut t = 0i64;
        for i in 1..=5 {
            t += 16;
            handle_input(
                &mut app,
                pointer_to([300.0 + 20.0 * f64::from(i) as f32, 200.0]),
                t * MS,
            );
            frame(&mut app, t);
        }
        t += 16;
        handle_input(&mut app, left_button([400.0, 200.0], false), t * MS);
        assert!(app.camera.animating());
        app.dirty = false;
        crate::app_loop::schedule(&mut app, t * MS);
        assert!(app.r#loop.awake);
        run_until_still(&mut app, 1000);
        app.dirty = false;
        crate::app_loop::schedule(&mut app, 100_000 * MS);
        assert!(!app.r#loop.awake);
    }

    #[test]
    fn an_arming_turn_publishes_without_stepping() {
        // The scripted harnesses hold the turn whose facts arm motion: the
        // gesture's own change draws now, and the loop starts next turn — as
        // the reference's next animation frame starts it. Moves never hold.
        let mut app = headless_app();
        declare_plane(&mut app, [0.0, 0.0], [800.0, 600.0]);
        init_camera_from_url(&mut app);
        // A wheel arms a zoom; the held turn draws the unzoomed plane while
        // publishing nothing (nothing moved yet), and the next turn steps.
        handle_input(&mut app, wheel_at([0.0, 100.0], [400.0, 300.0]), 0);
        hold_camera_step();
        frame(&mut app, 16);
        assert!(app.camera.animating());
        assert_eq!(app.camera.scale, 0.5);
        frame(&mut app, 32);
        assert!(app.camera.scale > 0.5);
        // The hold is consumed whether or not the camera flies: this turn
        // steps nothing (the scale stands still) while the zoom stays
        // scheduled, and no hold leaks into the turn after.
        hold_camera_step();
        let held = app.camera.scale;
        frame(&mut app, 48);
        assert_eq!(app.camera.scale, held);
        assert!(app.camera.animating());
        frame(&mut app, 64);
        assert!(app.camera.scale > held);
    }

    #[test]
    fn the_debounce_writes_mid_drag_without_a_release() {
        // The TypeScript's `setTimeout` fires while the pointer is still
        // held; here the next frame after the deadline fires it.
        let mut app = headless_app();
        declare_plane(&mut app, [0.0, 0.0], [800.0, 600.0]);
        init_camera_from_url(&mut app);
        handle_input(&mut app, left_button([100.0, 100.0], true), 0);
        handle_input(&mut app, pointer_to([150.0, 100.0]), 16 * MS);
        frame(&mut app, 16);
        assert_eq!(app.url.query(), "");
        frame(&mut app, 300);
        assert_eq!(app.url.query(), "center=-1.0000,0.0000");
    }

    #[test]
    fn three_notches_keep_the_plane_point_under_the_cursor() {
        // `zoom-wheel-3` through the wiring: the canvas content box sits at
        // (100, 50), 400 by 300, so its center is at (300, 200) and the
        // cursor at (450, 90) anchors at (150, -110).
        let mut app = headless_app();
        declare_plane(&mut app, [100.0, 50.0], [400.0, 300.0]);
        app.url = crate::url::Url::from_query("center=2.0000,-1.0000&scale=1.0000");
        init_camera_from_url(&mut app);
        let unit = 25.0;
        let under = [2.0 + (450.0 - 300.0) / unit, -1.0 - (90.0 - 200.0) / unit];
        for t in [0i64, 16, 32] {
            handle_input(&mut app, wheel_at([0.0, 100.0], [450.0, 90.0]), t * MS);
        }
        run_until_still(&mut app, 1000);
        assert!((app.camera.scale - 0.3f64.exp()).abs() < 1e-12);
        let unit = 25.0 / app.camera.scale;
        let kept = [
            app.camera.center[0] + (450.0 - 300.0) / unit,
            app.camera.center[1] - (90.0 - 200.0) / unit,
        ];
        assert!((kept[0] - under[0]).abs() < 1e-9, "{kept:?} vs {under:?}");
        assert!((kept[1] - under[1]).abs() < 1e-9, "{kept:?} vs {under:?}");
        // Settled, so both flushed: the keys keep their positions, the scale
        // at three significant digits.
        assert_eq!(
            app.url.query(),
            format!(
                "center={}&scale=1.35",
                camera::format_center(app.camera.center)
            )
        );
    }

    #[test]
    fn a_horizontal_wheel_does_nothing() {
        let mut app = headless_app();
        declare_plane(&mut app, [0.0, 0.0], [800.0, 600.0]);
        init_camera_from_url(&mut app);
        handle_input(&mut app, wheel_at([200.0, 0.0], [400.0, 300.0]), 0);
        assert!(!app.camera.animating());
        assert_eq!(app.camera.center, [0.0, 0.0]);
        assert_eq!(app.camera.scale, 0.5);
    }

    #[test]
    fn tiny_trackpad_steps_add_up() {
        let mut app = headless_app();
        declare_plane(&mut app, [0.0, 0.0], [800.0, 600.0]);
        init_camera_from_url(&mut app);
        for t in 0..10 {
            handle_input(&mut app, wheel_at([0.0, 0.2], [400.0, 300.0]), t * MS);
        }
        run_until_still(&mut app, 1000);
        // Script deltas are f32: ten f32 0.2 steps sum in f32 before the
        // exp, so the target is that sum's zoom, not `0.002f64`'s.
        let dy = f64::from(0.2f32) * 10.0;
        assert!((app.camera.scale - 0.5 * (dy * 0.001).exp()).abs() < 1e-9);
    }

    #[test]
    fn reset_view_returns_to_the_defaults_flushed() {
        // The overlay button's click: both params to defaults with an
        // immediate write (defaults flush as deletions, so the query empties)
        // and the still camera following at once.
        let mut app = headless_app();
        declare_plane(&mut app, [0.0, 0.0], [800.0, 600.0]);
        init_camera_from_url(&mut app);
        handle_input(&mut app, left_button([100.0, 100.0], true), 0);
        handle_input(&mut app, pointer_to([200.0, 100.0]), 16 * MS);
        frame(&mut app, 16);
        // Held still past the flick window, then released: no glide, and the
        // release's frame flushes the URL at once.
        frame(&mut app, 200);
        handle_input(&mut app, left_button([200.0, 100.0], false), 200 * MS);
        frame(&mut app, 216);
        assert_eq!(app.url.query(), "center=-2.0000,0.0000");
        app.dirty = false;
        app.reset_view();
        assert_eq!(app.camera.center, [0.0, 0.0]);
        assert_eq!(app.camera.scale, 0.5);
        assert_eq!(app.url.query(), "");
        assert!(app.dirty);
    }

    #[test]
    fn a_hash_edit_flows_into_the_live_values_and_the_still_camera() {
        let mut app = headless_app();
        declare_plane(&mut app, [0.0, 0.0], [800.0, 600.0]);
        init_camera_from_url(&mut app);
        // What the `UserCode::Hash` arm runs: route and query in at once,
        // then each param's sync with the adopt.
        app.url
            .sync_from_hash("#function-viewer?center=7.0000,8.0000");
        assert!(external_url_synced(&mut app));
        assert_eq!(*app.center_param.get(), [7.0, 8.0]);
        assert_eq!(app.camera.center, [7.0, 8.0]);
        assert!(app.dirty);
        assert!(!external_url_synced(&mut app));
    }

    #[test]
    fn a_non_primary_button_never_drags() {
        let mut app = headless_app();
        declare_plane(&mut app, [0.0, 0.0], [800.0, 600.0]);
        init_camera_from_url(&mut app);
        handle_input(
            &mut app,
            Input::Button(ButtonEvent {
                button: 3,
                down: true,
                clicks: 1,
                pos: [400.0, 300.0],
            }),
            0,
        );
        assert_eq!(app.input.pressed.kind, HitKind::None);
        handle_input(&mut app, pointer_to([500.0, 300.0]), 16 * MS);
        assert_eq!(app.camera.center, [0.0, 0.0]);
        assert!(!app.camera.animating());
    }

    #[test]
    fn losing_focus_ends_the_drag_without_a_glide_from_stale_samples() {
        let mut app = headless_app();
        declare_plane(&mut app, [0.0, 0.0], [800.0, 600.0]);
        init_camera_from_url(&mut app);
        handle_input(&mut app, left_button([300.0, 200.0], true), 0);
        handle_input(&mut app, pointer_to([320.0, 200.0]), 16 * MS);
        frame(&mut app, 16);
        // Focus leaves long after the flick window: the release velocity
        // measures ~zero, so no glide starts.
        handle_input(
            &mut app,
            Input::Focus(crate::input::FocusEvent { gained: false }),
            500 * MS,
        );
        run_until_still(&mut app, 1000);
        assert!((app.camera.center[0] - -0.4).abs() < 1e-12);
    }

    /// Seeds the page query's function and parameters, as a `query` line
    /// does: the camera and the pick plus a, b, h, k.
    fn seed_view(app: &mut App, query: &str) {
        app.url = crate::url::Url::from_query(query);
        init_camera_from_url(app);
        app.func_param = QueryParam::new(&crate::functions::FUNC_PARAM, app.url.query());
        app.a_param = QueryParam::new(&crate::functions::A_PARAM, app.url.query());
        app.b_param = QueryParam::new(&crate::functions::B_PARAM, app.url.query());
        app.h_param = QueryParam::new(&crate::functions::H_PARAM, app.url.query());
        app.k_param = QueryParam::new(&crate::functions::K_PARAM, app.url.query());
        app.p_param = QueryParam::new(&crate::functions::P_PARAM, app.url.query());
    }

    /// The canvas px of a handle: the content box sits at the origin, 800 by
    /// 600, so the grid's origin is its center (400, 300) at 50 px a unit.
    fn handle_px(app: &App, index: usize) -> [f32; 2] {
        let grid = crate::grid::layout_grid(
            app.camera.center,
            app.camera.scale,
            800.0,
            600.0,
            crate::plane::LABEL_MARGIN,
        );
        let [x, y] = crate::plane_geometry::to_px(&grid, crate::page::build_handles(app)[index].at);
        [x as f32, y as f32]
    }

    #[test]
    fn pressing_empty_canvas_shows_the_closed_hand_until_release() {
        // The `CartesianPlane` closed-hand test's shape, against the cursor
        // state: no handles under the press (x1 at rest, the diamond sits
        // at the origin's px), so the plane pans.
        let mut app = headless_app();
        declare_plane(&mut app, [0.0, 0.0], [800.0, 600.0]);
        seed_view(&mut app, "func=x1");
        handle_input(&mut app, pointer_to([100.0, 100.0]), 0);
        assert_eq!(app.input.cursor, crate::input::Cursor::Crosshair);
        handle_input(&mut app, left_button([100.0, 100.0], true), 16 * MS);
        assert_eq!(app.input.cursor, crate::input::Cursor::Grabbing);
        assert!(!app.dragging);
        handle_input(&mut app, pointer_to([140.0, 100.0]), 32 * MS);
        assert_eq!(app.input.cursor, crate::input::Cursor::Grabbing);
        handle_input(&mut app, left_button([140.0, 100.0], false), 48 * MS);
        assert_eq!(app.input.cursor, crate::input::Cursor::Crosshair);
    }

    #[test]
    fn hovering_the_diamond_lights_h_and_k() {
        let mut app = headless_app();
        declare_plane(&mut app, [0.0, 0.0], [800.0, 600.0]);
        seed_view(&mut app, "func=x2&a=2&h=1&k=-3&p=5");
        let diamond = handle_px(&app, 0);
        assert_eq!(diamond, [450.0, 450.0]);
        handle_input(&mut app, pointer_to(diamond), 16 * MS);
        assert_eq!(app.hovered, Some(HandleId::Shift));
        assert_eq!(app.input.cursor, crate::input::Cursor::Grab);
        assert_eq!(app.state.lit(), vec![TransformParam::H, TransformParam::K]);
    }

    #[test]
    fn hovering_straight_to_the_square_gives_the_open_hand_with_a_b_lit() {
        // `ea29aa9`: the hover derives from the hovered index, so a jump
        // straight from the diamond to the square still shows the open hand
        // while a and b light and h and k drop.
        let mut app = headless_app();
        declare_plane(&mut app, [0.0, 0.0], [800.0, 600.0]);
        seed_view(&mut app, "func=x2&a=2&h=1&k=-3&p=5");
        let diamond = handle_px(&app, 0);
        let square = handle_px(&app, 1);
        handle_input(&mut app, pointer_to(diamond), 16 * MS);
        handle_input(&mut app, pointer_to(square), 32 * MS);
        assert_eq!(app.hovered, Some(HandleId::Scale));
        assert_eq!(app.input.cursor, crate::input::Cursor::Grab);
        assert_eq!(app.state.lit(), vec![TransformParam::A, TransformParam::B]);
    }

    #[test]
    fn moving_off_the_handles_drops_the_light() {
        let mut app = headless_app();
        declare_plane(&mut app, [0.0, 0.0], [800.0, 600.0]);
        seed_view(&mut app, "func=x2&a=2&h=1&k=-3");
        let diamond = handle_px(&app, 0);
        handle_input(&mut app, pointer_to(diamond), 16 * MS);
        handle_input(&mut app, pointer_to([100.0, 100.0]), 32 * MS);
        assert_eq!(app.hovered, None);
        assert_eq!(app.input.cursor, crate::input::Cursor::Crosshair);
        assert!(app.state.lit().is_empty());
    }

    /// Ported from `CartesianPlane.test.tsx` ("reports the point under the
    /// pointer, and null once the pointer leaves or pans the plane"): two
    /// moves 40 px apart quantize 0.8 apart at 50 px a unit, leaving the
    /// canvas clears, and a press and move pans with Q held null.
    #[test]
    fn hovering_the_plane_reports_the_quantized_point_until_it_leaves_or_pans() {
        let mut app = headless_app();
        declare_plane(&mut app, [0.0, 0.0], [800.0, 600.0]);
        seed_view(&mut app, "func=x1");
        // Off both handles (the diamond at the origin's px, the square at
        // (450, 250)): (500, 300) is plot (2, 0).
        handle_input(&mut app, pointer_to([500.0, 300.0]), 16 * MS);
        assert_eq!(app.state.q_x, Some(2.0));
        handle_input(&mut app, pointer_to([540.0, 300.0]), 32 * MS);
        assert_eq!(app.state.q_x, Some(2.8));
        // Off the canvas the hover leaves the plane: Q clears.
        handle_input(&mut app, pointer_to([-10.0, 300.0]), 48 * MS);
        assert_eq!(app.state.q_x, None);
        // Back on, then a press and move: panning holds Q null.
        handle_input(&mut app, pointer_to([540.0, 300.0]), 64 * MS);
        assert_eq!(app.state.q_x, Some(2.8));
        handle_input(&mut app, left_button([540.0, 300.0], true), 80 * MS);
        handle_input(&mut app, pointer_to([580.0, 300.0]), 96 * MS);
        assert_eq!(app.state.q_x, None);
    }

    /// Ported from `CartesianPlane.test.tsx` ("reports null while the pointer
    /// rests on a handle"): Q shows off the handles and clears onto one.
    #[test]
    fn hovering_a_handle_reports_no_point() {
        let mut app = headless_app();
        declare_plane(&mut app, [0.0, 0.0], [800.0, 600.0]);
        seed_view(&mut app, "func=x1");
        handle_input(&mut app, pointer_to([500.0, 300.0]), 16 * MS);
        assert_eq!(app.state.q_x, Some(2.0));
        let diamond = handle_px(&app, 0);
        handle_input(&mut app, pointer_to(diamond), 32 * MS);
        assert_eq!(app.hovered, Some(HandleId::Shift));
        assert_eq!(app.state.q_x, None);
    }

    #[test]
    fn dragging_the_diamond_sets_h_and_k_coarse_with_no_pan() {
        let mut app = headless_app();
        declare_plane(&mut app, [0.0, 0.0], [800.0, 600.0]);
        seed_view(&mut app, "func=x2&a=2&h=1&k=-3&p=5");
        let diamond = handle_px(&app, 0);
        handle_input(&mut app, left_button(diamond, true), 0);
        assert!(app.dragging);
        assert_eq!(app.input.cursor, crate::input::Cursor::Grabbing);
        assert_eq!(
            app.state.scrub_mode[TransformParam::H],
            Some(crate::scrub::ScrubMode::Coarse)
        );
        assert_eq!(
            app.state.scrub_mode[TransformParam::K],
            Some(crate::scrub::ScrubMode::Coarse)
        );
        // +50, +30 px in small steps: h = 2.0, k = -3.6, the camera still.
        let mut t = 0i64;
        for i in 1..=5 {
            t += 16;
            handle_input(
                &mut app,
                pointer_to([diamond[0] + 10.0 * i as f32, diamond[1] + 6.0 * i as f32]),
                t * MS,
            );
            frame(&mut app, t);
        }
        assert_eq!(*app.h_param.get(), 2.0);
        assert_eq!(*app.k_param.get(), -3.6);
        assert_eq!(app.camera.center, [0.0, 0.0]);
        assert_eq!(*app.center_param.get(), [0.0, 0.0]);
        // The drag's sets are still inside their 200 ms debounce: the URL
        // holds the opening query.
        assert_eq!(app.url.query(), "func=x2&a=2&h=1&k=-3&p=5");
        // The loop's param step (not the camera's frame step) fires the
        // debounce past its deadline.
        crate::transform_section::params_step(&mut app, 300);
        assert_eq!(app.url.query(), "func=x2&a=2&h=2&k=-3.6&p=5");
        // Letting go flushes both and clears the modes; the pointer sits on
        // the handle, so the open hand and the lit pair stay.
        t += 16;
        handle_input(&mut app, left_button([500.0, 480.0], false), t * MS);
        assert!(!app.dragging);
        assert_eq!(app.state.scrub_mode[TransformParam::H], None);
        assert_eq!(app.state.scrub_mode[TransformParam::K], None);
        assert_eq!(app.url.query(), "func=x2&a=2&h=2&k=-3.6&p=5");
        assert_eq!(app.input.cursor, crate::input::Cursor::Grab);
        assert_eq!(app.state.lit(), vec![TransformParam::H, TransformParam::K]);
    }

    #[test]
    fn dragging_the_square_sets_a_and_b() {
        let mut app = headless_app();
        declare_plane(&mut app, [0.0, 0.0], [800.0, 600.0]);
        seed_view(&mut app, "func=x2&a=2&h=1&k=-3&p=5");
        let square = handle_px(&app, 1);
        assert_eq!(square, [500.0, 350.0]);
        handle_input(&mut app, left_button(square, true), 0);
        assert!(app.dragging);
        // -40, +60 px: b = 0.2, a = 0.8.
        let mut t = 0i64;
        for i in 1..=5 {
            t += 16;
            handle_input(
                &mut app,
                pointer_to([square[0] - 8.0 * i as f32, square[1] + 12.0 * i as f32]),
                t * MS,
            );
            frame(&mut app, t);
        }
        assert!(
            (app.b_param.get() - 0.2).abs() < 1e-9,
            "b = {}",
            app.b_param.get()
        );
        assert!(
            (app.a_param.get() - 0.8).abs() < 1e-9,
            "a = {}",
            app.a_param.get()
        );
        assert_eq!(app.camera.center, [0.0, 0.0]);
        t += 16;
        handle_input(&mut app, left_button([460.0, 410.0], false), t * MS);
        assert_eq!(app.url.query(), "func=x2&a=0.8&h=1&k=-3&p=5&b=0.2");
        assert_eq!(app.state.scrub_mode[TransformParam::A], None);
        assert_eq!(app.state.scrub_mode[TransformParam::B], None);
    }

    #[test]
    fn a_handle_drag_past_the_edge_keeps_following() {
        // Past the canvas's right edge the grab still maps (the capture
        // retargets the moves): the values follow, the camera never moves.
        let mut app = headless_app();
        declare_plane(&mut app, [0.0, 0.0], [800.0, 600.0]);
        seed_view(&mut app, "func=x2&a=2&h=1&k=-3&p=5");
        let diamond = handle_px(&app, 0);
        handle_input(&mut app, left_button(diamond, true), 0);
        handle_input(&mut app, pointer_to([950.0, 450.0]), 16 * MS);
        frame(&mut app, 16);
        assert!(
            (app.h_param.get() - 11.0).abs() < 1e-9,
            "h = {}",
            app.h_param.get()
        );
        assert_eq!(*app.k_param.get(), -3.0);
        assert_eq!(app.camera.center, [0.0, 0.0]);
        assert_eq!(app.url.query(), "func=x2&a=2&h=1&k=-3&p=5");
    }
}
