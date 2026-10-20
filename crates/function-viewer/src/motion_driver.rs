//! The per-frame animation driver: what moves, since when, and what the plane
//! shows now.
//!
//! Model changes heard since launch become timed layers; fades, transitions,
//! overlays, flashes and edge arrivals all read the frame's time. (Declares
//! against the motion, plane and viewer surfaces; wired when they land.)

use enum_map::Enum;
use enum_map::EnumMap;

use crate::app::App;
use crate::app_loop::MS;
use crate::app_loop::Time;
use crate::effects::Layer;

/// What moves, since when.
pub struct Motion {
    pub layers: mv_viewer::MotionLayers,
    /// The model's values as last heard: none before launch.
    pub seen: Option<Seen>,
    /// Each changed term's flash, since when.
    pub flashes: EnumMap<mv_viewer::Param, Option<f64>>,
    /// The frame's time, in s.
    pub now: f64,
    /// The plane's edge markers, since they appeared.
    pub arrivals: Vec<mv_plane::EdgeArrival>,
    /// An edge marker is still arriving.
    pub arriving: bool,
    /// Q's role and readout, faded in as the pointer enters or leaves.
    pub q_row: mv_motion::FadeIn,
    /// What Q's row showed when last laid out.
    pub q_live: Option<bool>,
    /// The overlays on their way in and out.
    pub overlays: EnumMap<OverlayKind, mv_motion::Overlay>,
    /// The parts' own fades and transitions.
    pub watched: [Watched; WATCHED_MAX],
    pub watching: i64,
    /// Anything moved at the last frame.
    pub moving: bool,
    /// The plane's layers, counted for the log.
    pub plane: MotionCount,
}

/// One watched fade or transition and whether it is translucent.
#[derive(Clone, Copy, Debug)]
pub struct Watched {
    pub motion: WatchedMotion,
    pub translucent: bool,
}

/// A watched fade or transition, by pointer.
#[derive(Clone, Copy, Debug)]
pub enum WatchedMotion {
    Fade(*mut mv_motion::FadeIn),
    Transition(*mut mv_motion::Transition),
}

/// How many watched motions fit.
pub const WATCHED_MAX: usize = 8;

/// The model's watched values.
#[derive(Clone, Debug)]
pub struct Seen {
    pub r#fn: mv_viewer::Base,
    pub params: mv_viewer::Params,
    pub p_x: f64,
    pub q_x: Option<f64>,
    pub ghost_on: bool,
}

/// The plane's layer count: since when, over what span, for how many frames.
#[derive(Clone, Copy, Debug)]
pub struct MotionCount {
    pub since: Time,
    pub span: f64,
    pub frames: i64,
}

/// Projects the state to its watched values.
fn seen_of(state: &mv_viewer::State) -> Seen {
    Seen {
        r#fn: state.func,
        params: state.params,
        p_x: state.p_x,
        q_x: state.q_x,
        ghost_on: state.ghost_on,
    }
}

/// Starts the driver's own fades at launch: Q's row.
///
/// Ported from `motion_driver.odin:84` (`motion_init`).
pub fn motion_init(app: &mut App) {
    let q_row = std::ptr::addr_of_mut!(app.motion.q_row);
    motion_watch_fade(app, q_row);
}

/// Starts or stops the driver's global motions: a paint sweep, a function
/// switch and a parameter jump all begin here.
///
/// Ported from `motion_driver.odin:134` (`motion_launch`).
pub fn motion_launch(app: &mut App, at: Time) {
    let level = motion_level(app);
    app.motion.seen = Some(seen_of(&app.viewer));
    app.motion.layers = mv_viewer::AT_REST;
    app.motion.flashes = EnumMap::from_array([None, None, None, None]);
    app.motion.plane = MotionCount {
        since: 0,
        span: 0.0,
        frames: 0,
    };
    if level == mv_motion::Level::Full {
        app.motion.layers.paint = Some(seconds(at));
        app.motion.plane = MotionCount {
            since: at,
            span: mv_viewer::FIRST_PAINT_END,
            frames: 0,
        };
    }
    app.dirty = true;
}

/// Hears one state change and starts, adjusts or stops the layers it moves.
/// Runs from `dispatch`, before the layout, so the layers already move on the
/// first frame after the change.
///
/// Ported from `motion_driver.odin:152` (`motion_heard`).
pub fn motion_heard(app: &mut App) {
    let state = app.viewer.clone();
    let at = if app.input.time != 0 {
        app.input.time
    } else {
        crate::app_loop::clock_now(app)
    };
    let now = seconds(at);
    let level = motion_level(app);
    let seen = match app.motion.seen.clone() {
        Some(seen) => seen,
        None => return,
    };
    let was = mv_viewer::is_moving(&app.motion.layers);
    let mut next = mv_viewer::settle(app.motion.layers, now, level);
    let mut began = false;
    if state.func != seen.r#fn {
        let shown = mv_viewer::shown_params(seen.params, next, now);
        next.paint = None;
        next.jump = None;
        next.fn_switch = None;
        if level == mv_motion::Level::Full {
            next.fn_switch = Some(mv_viewer::SwitchLayer {
                func: seen.r#fn,
                params: shown,
                start: now,
            });
            began = true;
        }
    } else if state.params != seen.params {
        const N_PARAMS: usize = mv_viewer::Param::LENGTH;
        let mut changed = [false; N_PARAMS];
        for (i, slot) in changed.iter_mut().enumerate() {
            let p = mv_viewer::Param::from_usize(i);
            if state.params[p] != seen.params[p] {
                *slot = true;
            }
        }
        next.paint = None;
        if state.change == mv_viewer::Change::Jump && level == mv_motion::Level::Full {
            next.jump = Some(mv_viewer::JumpLayer {
                from: mv_viewer::shown_params(seen.params, next, now),
                to: state.params,
                start: now,
            });
            began = true;
        } else if let Some(mut j) = next.jump {
            for (i, is_changed) in changed.iter().enumerate() {
                if *is_changed {
                    let p = mv_viewer::Param::from_usize(i);
                    j.from[p] = state.params[p];
                    j.to[p] = state.params[p];
                }
            }
            next.jump = Some(j);
        }
        if state.drag.is_none() {
            for (i, is_changed) in changed.iter().enumerate() {
                if *is_changed {
                    app.motion.flashes[mv_viewer::Param::from_usize(i)] = Some(now);
                }
            }
        }
    }
    if state.p_x != seen.p_x {
        next.paint = None;
    }
    if state.q_x.is_none() != seen.q_x.is_none() {
        let mut alpha = if seen.q_x.is_none() { 0.0 } else { 1.0 };
        if next.q.is_some() {
            alpha = mv_viewer::q_look_at(&next, now, level).alpha;
        }
        if let Some(x) = state.q_x {
            next.q = Some(mv_viewer::QLayer {
                dir: mv_viewer::QDir::In,
                x,
                from: alpha,
                start: now,
            });
        } else {
            next.q = Some(mv_viewer::QLayer {
                dir: mv_viewer::QDir::Out,
                x: seen.q_x.unwrap_or(0.0),
                from: alpha,
                start: now,
            });
        }
    }
    if state.ghost_on != seen.ghost_on {
        let mut from = if seen.ghost_on { 1.0 } else { 0.0 };
        if let Some(g) = next.ghost {
            from = mv_viewer::ghost_share_at(now - g.start, g.from, g.to, level);
        }
        next.ghost = Some(mv_viewer::GhostLayer {
            from,
            to: if state.ghost_on { 1.0 } else { 0.0 },
            start: now,
        });
    }
    app.motion.layers = next;
    app.motion.seen = Some(seen_of(&state));
    if began || (!was && mv_viewer::is_moving(&next)) {
        app.motion.plane = MotionCount {
            since: at,
            span: plane_span(&next, level),
            frames: 0,
        };
    }
}

/// Settles the layers to `now` (each frame, after the scripted state lands):
/// time passes, flashes fade, and the end of a run is logged.
///
/// Ported from `motion_driver.odin:249` (`motion_settle`).
pub fn motion_settle(app: &mut App, now: Time) {
    let now_s = seconds(now);
    let level = motion_level(app);
    let was = mv_viewer::is_moving(&app.motion.layers);
    app.motion.layers = mv_viewer::settle(app.motion.layers, now_s, level);
    app.motion.now = now_s;
    for (_, flash) in app.motion.flashes.iter_mut() {
        if let Some(start) = *flash
            && now_s - start >= mv_viewer::FLASH_END
        {
            *flash = None;
        }
    }
    if was && !mv_viewer::is_moving(&app.motion.layers) {
        let frames = app.motion.plane.frames;
        let span_ms = app.motion.plane.span * 1000.0;
        let rested_ms = (now - app.motion.plane.since) as f64 / MS as f64;
        let pace = pace_said(app, span_ms);
        println!(
            "motion: {frames} frames while the plane moved over {span_ms:.0} ms (its first included); at rest {rested_ms:.1} ms after it began; {pace}"
        );
    }
    let moving = app.motion.moving;
    if moving && !motion_moving(app) {
        app.dirty = true;
    }
}

/// Reports the frame pacing for the settled log line.
///
/// Ported from `motion_driver.odin:392` (`pace_said`).
pub fn pace_said(app: &mut App, mss: f64) -> String {
    if app.r#loop.pace == crate::app_loop::Pace::Fixed60 {
        return format!(
            "frames paced at 60 Hz by SDL's callback rate, {:.0} in {:.0} ms",
            0.06 * mss,
            mss
        );
    }
    #[cfg(not(target_os = "emscripten"))]
    let hz = crate::platform_native::platform_refresh_rate(app);
    #[cfg(target_os = "emscripten")]
    let hz = crate::platform_web::platform_refresh_rate(app);
    if hz <= 0.0 {
        return "frames paced by the display, its refresh rate unknown".to_string();
    }
    format!(
        "frames paced by the display, which refreshes at {:.0} Hz: {:.0} in {:.0} ms",
        hz,
        f64::from(hz) * mss / 1000.0,
        mss
    )
}

/// Wakes or rests the driver: whether anything still moves.
///
/// Ported from `motion_driver.odin:277` (`motion_awake`).
pub fn motion_awake(app: &mut App) -> bool {
    app.motion.moving = motion_moving(app);
    app.motion.moving
}

/// Whether anything still moves: a layer, an arrival, a watched part, a
/// flash or an overlay.
///
/// Ported from `motion_driver.odin:284` (`motion_moving`).
pub fn motion_moving(app: &mut App) -> bool {
    let m = &app.motion;
    if mv_viewer::is_moving(&m.layers) || m.arriving {
        return true;
    }
    for w in m.watched[..m.watching as usize].iter() {
        if watched_running(*w, m.now) {
            return true;
        }
    }
    for o in m.overlays.values() {
        if mv_motion::overlay_moving(*o, m.now) {
            return true;
        }
    }
    for (_, flash) in m.flashes.iter() {
        if flash.is_some() {
            return true;
        }
    }
    false
}

/// Whether anything translucent still moves: the paint until its label lands,
/// a switch or the Q, an arrival, a watched translucent part, a flash or an
/// overlay.
///
/// Ported from `motion_driver.odin:312` (`motion_translucent`).
pub fn motion_translucent(app: &mut App) -> bool {
    let m = &app.motion;
    let l = m.layers;
    if let Some(start) = l.paint
        && m.now - start < mv_viewer::FIRST_PAINT_LABEL[1]
    {
        return true;
    }
    if l.fn_switch.is_some() || l.q.is_some() || l.ghost.is_some() || m.arriving {
        return true;
    }
    for w in m.watched[..m.watching as usize].iter() {
        if w.translucent && watched_running(*w, m.now) {
            return true;
        }
    }
    for (_, flash) in m.flashes.iter() {
        if flash.is_some() {
            return true;
        }
    }
    for o in m.overlays.values() {
        if mv_motion::overlay_moving(*o, m.now) {
            return true;
        }
    }
    false
}

/// Counts the frame for the settled log line when the plane moved.
///
/// Ported from `motion_driver.odin:340` (`motion_drawn`).
pub fn motion_drawn(app: &mut App) {
    if mv_viewer::is_moving(&app.motion.layers) {
        app.motion.plane.frames += 1;
    }
}

/// The plane the motion layers see at the driver's time.
///
/// Ported from `motion_driver.odin:347` (`plane_scene_now`).
pub fn plane_scene_now(app: &mut App) -> mv_plane::Scene {
    let level = motion_level(app);
    let input = mv_viewer::plane_scene_input(&app.viewer);
    mv_viewer::motion_scene(
        &input,
        app.motion.layers,
        app.motion.now,
        app.viewer.view,
        level,
    )
}

/// Steps the arrivals to the driver's time, reporting whether any still move.
///
/// Ported from `motion_driver.odin:355` (`plane_arrivals`).
pub fn plane_arrivals(app: &mut App) {
    let now_ms = app.motion.now * 1000.0;
    let reduced = app.reduced_motion;
    let arriving = mv_plane::arrive_edges(
        &mut app.plane.frame.marks,
        &mut app.motion.arrivals,
        now_ms,
        reduced,
    );
    app.motion.arriving = arriving;
}

/// The terminal's flash for one parameter, or none.
///
/// Ported from `motion_driver.odin:361` (`term_flash`).
pub fn term_flash(app: &mut App, p: mv_viewer::Param) -> f64 {
    match app.motion.flashes[p] {
        Some(start) => mv_viewer::flash_at(app.motion.now - start, motion_level(app)),
        None => 0.0,
    }
}

/// Fades the Q row in or out as its content changes, reporting its opacity.
///
/// Ported from `motion_driver.odin:373` (`q_row_fade`).
pub fn q_row_fade(app: &mut App, live: bool) -> f32 {
    let was_shown = app.motion.q_live;
    app.motion.q_live = Some(live);
    if let Some(was) = was_shown
        && was != live
    {
        match (app.reduced_motion, live) {
            (true, _) => mv_motion::fade_in_start(
                &mut app.motion.q_row,
                app.motion.now,
                mv_motion::REDUCED_FADE_MS,
                mv_motion::Ease::Linear,
            ),
            (false, true) => mv_motion::fade_in_start(
                &mut app.motion.q_row,
                app.motion.now,
                mv_motion::FAST_MS,
                mv_motion::Ease::CssEnter,
            ),
            (false, false) => mv_motion::fade_in_start(
                &mut app.motion.q_row,
                app.motion.now,
                mv_motion::LEAVE_MS,
                mv_motion::Ease::Linear,
            ),
        }
    }
    mv_motion::fade_in_at(app.motion.q_row, app.motion.now) as f32
}

/// The two overlays.
#[derive(Clone, Copy, Debug, PartialEq, Eq, enum_map::Enum)]
pub enum OverlayKind {
    Settings,
    Explainer,
}

/// An overlay's origin: scale about a point, offset from it.
#[derive(Clone, Copy, Debug)]
pub struct OverlayFrom {
    pub scale: f32,
    pub origin: [f32; 2],
    pub offset: [f32; 2],
}

/// Opens, shuts or rests one overlay at the driver's time, reporting its
/// presence and its layer.
///
/// Ported from `motion_driver.odin:431` (`overlay_frame`).
pub fn overlay_frame(
    app: &mut App,
    kind: OverlayKind,
    open: bool,
    from: OverlayFrom,
) -> (mv_motion::Presence, Layer) {
    let now = app.motion.now;
    let level = motion_level(app);
    let o = &mut app.motion.overlays[kind];
    mv_motion::overlay_set(o, open, now, level);
    let (alpha, shape) = mv_motion::overlay_look(*o, now);
    let k = shape as f32;
    let layer = Layer {
        alpha: alpha as f32,
        scale: from.scale + (1.0 - from.scale) * k,
        origin: from.origin,
        offset: [from.offset[0] * (1.0 - k), from.offset[1] * (1.0 - k)],
    };
    (mv_motion::overlay_presence(*o, now), layer)
}

/// Reduced motion shows the state as-is; full motion runs the layers.
///
/// Ported from `motion_driver.odin:74` (`motion_level`).
pub fn motion_level(app: &mut App) -> mv_motion::Level {
    if app.reduced_motion {
        mv_motion::Level::Reduced
    } else {
        mv_motion::Level::Full
    }
}

/// Starts watching the fade.
///
/// The entry keeps a raw pointer to the caller's fade (the Odin shape:
/// `motion_watch` takes `^Fade_In`). SAFETY invariant on every entry: the
/// pointer is live while it is listed — a part's fade outlives the frames
/// that read it — and entries into the App itself (Q's row, via
/// `motion_init`) are registered only after the App's last move (`app_init`
/// runs `motion_init` beside `init_clay`, after boxing); the list drops
/// with the App, so no entry outlives it.
///
/// Ported from `motion_driver.odin:91` (`motion_watch_fade`, a
/// `motion_watch` overload).
pub fn motion_watch_fade(app: &mut App, fade: *mut mv_motion::FadeIn) {
    watch(
        app,
        Watched {
            motion: WatchedMotion::Fade(fade),
            translucent: true,
        },
    );
}

/// Starts watching the transition, translucent or not.
///
/// Ported from `motion_driver.odin:96` (`motion_watch_transition`, a
/// `motion_watch` overload).
pub fn motion_watch_transition(
    app: &mut App,
    transition: *mut mv_motion::Transition,
    translucent: bool,
) {
    watch(
        app,
        Watched {
            motion: WatchedMotion::Transition(transition),
            translucent,
        },
    );
}

/// How long after a change the plane still moves: the longest layer's span.
///
/// Ported from `motion_driver.odin:230` (`plane_span`, file-private).
fn plane_span(layers: &mv_viewer::MotionLayers, level: mv_motion::Level) -> f64 {
    let mut span: f64 = 0.0;
    if layers.paint.is_some() {
        span = span.max(mv_viewer::FIRST_PAINT_END);
    }
    if layers.jump.is_some() {
        span = span.max(*mv_viewer::JUMP_S);
    }
    if layers.fn_switch.is_some() {
        span = span.max(mv_viewer::motion::SWITCH_END_S);
    }
    let fade = if level == mv_motion::Level::Full {
        mv_viewer::Q_ENTER
    } else {
        mv_motion::REDUCED_FADE_MS / 1000.0
    };
    if layers.q.is_some() || layers.ghost.is_some() {
        span = span.max(fade);
    }
    span
}

/// Whether the watched part still moves.
///
/// Ported from `motion_driver.odin:120` (`watched_running`, file-private).
fn watched_running(w: Watched, now: f64) -> bool {
    match w.motion {
        // SAFETY: `motion_watch_fade`'s invariant: the pointer is live
        // while listed (a part's own fade, or Q's row registered after the
        // App's last move). Listed = live.
        WatchedMotion::Fade(f) => unsafe { mv_motion::fade_in_running(*f, now) },
        WatchedMotion::Transition(t) => unsafe { mv_motion::transition_running(*t, now) },
    }
}

/// Lists one part motion to watch, once.
///
/// Ported from `motion_driver.odin:106` (`watch`, file-private).
fn watch(app: &mut App, w: Watched) {
    for listed in app.motion.watched[..app.motion.watching as usize].iter() {
        let same = match (listed.motion, w.motion) {
            (WatchedMotion::Fade(a), WatchedMotion::Fade(b)) => a == b,
            (WatchedMotion::Transition(a), WatchedMotion::Transition(b)) => a == b,
            _ => false,
        };
        if same {
            return;
        }
    }
    assert!(
        app.motion.watching < WATCHED_MAX as i64,
        "motion_watch: more parts' motions than WATCHED_MAX",
    );
    app.motion.watched[app.motion.watching as usize] = w;
    app.motion.watching += 1;
}

/// Time in seconds.
fn seconds(t: Time) -> f64 {
    t as f64 / 1e9
}
// The tests below port motion_driver_test.odin one for one.
// Ported from `motion_driver_test.odin:15` (`driver_app`): the headless
// constructor plus the model, the reduced-motion flag, the harness clock and
// the driver's launch, as the Odin setup does. No window, no GL, no NanoVG:
// `test_window` is deleted (a `cargo test` thread cannot start SDL video).

#[cfg(test)]
pub(crate) fn driver_app(reduced: bool) -> App {
    let mut app = crate::fixture::headless_app();
    app.viewer = mv_viewer::initial_state();
    app.reduced_motion = reduced;
    app.r#loop.script_time = Some(0);
    motion_init(&mut app);
    motion_launch(&mut app, 0);
    app
}

#[cfg(test)]
pub(crate) fn at(app: &mut App, mss: i64) {
    app.r#loop.script_time = Some(mss * MS);
}

#[cfg(test)]
mod tests {
    use super::*;
    use enum_map::EnumMap;

    // The tests below port motion_driver_test.odin one for one.

    #[test]
    fn the_first_paint_begins_at_launch_with_full_motion_only() {
        let app = driver_app(false);
        assert_eq!(app.motion.layers.paint.unwrap_or(-1.0), 0.0);
        let mut reduced = driver_app(true);
        assert!(reduced.motion.layers.paint.is_none());
        assert!(!motion_moving(&mut reduced));
    }

    #[test]
    fn a_typed_value_jumps_from_where_the_curve_is_shown_and_flashes_its_term() {
        let mut app = driver_app(false);
        at(&mut app, 500);
        // The input happened at 484 ms; the frame handles it at 500
        // (handle_input's time).
        app.input.time = 484 * MS;
        crate::parts::dispatch(
            &mut app,
            mv_viewer::Action::SetParam(mv_viewer::SetParam {
                param: mv_viewer::Param::A,
                value: 2.0,
                jump: true,
            }),
        );
        app.input.time = 0;
        assert!(app.motion.layers.paint.is_none());
        let jump = app.motion.layers.jump.expect("jump");
        assert!(app.motion.layers.jump.is_some());
        assert_eq!(jump.start, 0.484);
        assert_eq!(jump.from[mv_viewer::Param::A], 1.0);
        assert_eq!(jump.to[mv_viewer::Param::A], 2.0);
        assert_eq!(
            app.motion.flashes[mv_viewer::Param::A].unwrap_or(-1.0),
            0.484
        );
        assert!(app.motion.flashes[mv_viewer::Param::K].is_none());
        // A second jump 100 ms in starts from where the first has taken the curve.
        at(&mut app, 600);
        crate::parts::dispatch(
            &mut app,
            mv_viewer::Action::SetParam(mv_viewer::SetParam {
                param: mv_viewer::Param::A,
                value: 3.0,
                jump: true,
            }),
        );
        let second = app.motion.layers.jump.expect("second jump");
        let shown = mv_viewer::jump_at(jump.from, jump.to, 0.6 - 0.484);
        assert_eq!(second.from[mv_viewer::Param::A], shown[mv_viewer::Param::A]);
        assert_eq!(second.to[mv_viewer::Param::A], 3.0);
        // It rests once the spring has settled; the frame after the last that
        // moved draws at rest.
        motion_settle(&mut app, (600 + 701) * MS);
        assert!(app.motion.layers.jump.is_some());
        let _ = motion_awake(&mut app);
        app.dirty = false;
        motion_settle(&mut app, (600 + 703) * MS);
        assert!(app.motion.layers.jump.is_none());
        assert!(app.dirty);
    }

    #[test]
    fn a_direct_change_takes_its_value_at_once_while_the_others_spring_and_a_drag_flashes_nothing()
    {
        let mut app = driver_app(false);
        at(&mut app, 100);
        // Nothing to reset: no change.
        crate::parts::dispatch(&mut app, mv_viewer::Action::ResetAll(mv_viewer::ResetAll));
        assert!(app.motion.layers.jump.is_none());
        crate::parts::dispatch(
            &mut app,
            mv_viewer::Action::SetParam(mv_viewer::SetParam {
                param: mv_viewer::Param::K,
                value: 1.0,
                jump: true,
            }),
        );
        app.motion.flashes = EnumMap::from_array([None, None, None, None]);
        at(&mut app, 200);
        crate::parts::dispatch(
            &mut app,
            mv_viewer::Action::SetDrag(mv_viewer::SetDrag {
                part: mv_viewer::Part::A,
                mode: Some(mv_scrub::Mode::Coarse),
            }),
        );
        crate::parts::dispatch(
            &mut app,
            mv_viewer::Action::SetParam(mv_viewer::SetParam {
                param: mv_viewer::Param::A,
                value: 1.5,
                jump: false,
            }),
        );
        let jump = app.motion.layers.jump.expect("drag jump");
        assert_eq!(jump.from[mv_viewer::Param::A], 1.5);
        assert_eq!(jump.to[mv_viewer::Param::A], 1.5);
        assert_eq!(jump.to[mv_viewer::Param::K], 1.0);
        assert!(app.motion.flashes[mv_viewer::Param::A].is_none());
    }

    #[test]
    fn q_enters_and_leaves_and_the_original_fades() {
        let mut app = driver_app(false);
        at(&mut app, 3000);
        crate::parts::dispatch(
            &mut app,
            mv_viewer::Action::PlanePointer(mv_viewer::PlanePointer {
                x: Some(-1.5),
                over: None,
                panning: false,
            }),
        );
        let q = app.motion.layers.q.expect("q");
        assert_eq!(
            q,
            mv_viewer::QLayer {
                dir: mv_viewer::QDir::In,
                x: -1.5,
                from: 0.0,
                start: 3.0,
            }
        );
        // Leaving 60 ms in, from the opacity it had come to.
        at(&mut app, 3060);
        crate::parts::dispatch(
            &mut app,
            mv_viewer::Action::PlanePointer(mv_viewer::PlanePointer {
                x: None,
                over: None,
                panning: false,
            }),
        );
        let out = app.motion.layers.q.expect("q out");
        assert_eq!(out.dir, mv_viewer::QDir::Out);
        assert_eq!(out.x, -1.5);
        assert_eq!(
            out.from,
            mv_viewer::q_enter_at(0.06, mv_motion::Level::Full).alpha
        );
        crate::parts::dispatch(
            &mut app,
            mv_viewer::Action::SetParam(mv_viewer::SetParam {
                param: mv_viewer::Param::H,
                value: 1.0,
                jump: true,
            }),
        );
        at(&mut app, 4000);
        crate::parts::dispatch(
            &mut app,
            mv_viewer::Action::SetGhost(mv_viewer::SetGhost { on: false }),
        );
        let ghost = app.motion.layers.ghost.expect("ghost");
        assert_eq!(
            ghost,
            mv_viewer::GhostLayer {
                from: 1.0,
                to: 0.0,
                start: 4.0,
            }
        );
    }

    #[test]
    fn a_new_function_fades_the_curve_as_it_was_shown() {
        let mut app = driver_app(false);
        at(&mut app, 3000);
        crate::parts::dispatch(
            &mut app,
            mv_viewer::Action::SetParam(mv_viewer::SetParam {
                param: mv_viewer::Param::K,
                value: 1.0,
                jump: true,
            }),
        );
        at(&mut app, 3100);
        crate::parts::dispatch(
            &mut app,
            mv_viewer::Action::SetFunction(mv_viewer::SetFunction {
                func: mv_viewer::Base::Sin,
            }),
        );
        let sw = app.motion.layers.fn_switch.expect("switch");
        assert_eq!(sw.func, mv_viewer::Base::X2);
        // The old curve as it was shown, 100 ms into the jump to k 1.
        let to = EnumMap::from_array([1.0, 1.0, 1.0, 0.0]);
        let shown_k = mv_viewer::jump_at(mv_viewer::DEFAULT_PARAMS, to, 0.1)[mv_viewer::Param::K];
        assert!((sw.params[mv_viewer::Param::K] - shown_k).abs() < 1e-12);
        assert!(app.motion.layers.jump.is_none());
        // Under reduced motion a switch draws the new curve whole at once, and
        // a jump lands at once.
        let mut reduced = driver_app(true);
        at(&mut reduced, 100);
        crate::parts::dispatch(
            &mut reduced,
            mv_viewer::Action::SetFunction(mv_viewer::SetFunction {
                func: mv_viewer::Base::Sin,
            }),
        );
        crate::parts::dispatch(
            &mut reduced,
            mv_viewer::Action::SetParam(mv_viewer::SetParam {
                param: mv_viewer::Param::K,
                value: 1.0,
                jump: true,
            }),
        );
        assert!(!mv_viewer::is_moving(&reduced.motion.layers));
        assert_eq!(
            reduced.motion.flashes[mv_viewer::Param::K].unwrap_or(-1.0),
            0.1
        );
    }

    #[test]
    fn an_overlay_comes_out_of_its_place_and_goes_back() {
        let mut app = driver_app(false);
        let from = OverlayFrom {
            scale: 0.9,
            origin: [304.0, 0.0],
            offset: [0.0, -8.0],
        };
        motion_settle(&mut app, 1000 * MS);
        let (presence, layer) = overlay_frame(&mut app, OverlayKind::Settings, true, from);
        assert_eq!(presence, mv_motion::Presence::Open);
        assert_eq!(layer.alpha, 0.0);
        assert_eq!(layer.scale, 0.9);
        assert_eq!(layer.origin, [304.0, 0.0]);
        assert_eq!(layer.offset, [0.0, -8.0]);
        assert!(motion_moving(&mut app));
        motion_settle(&mut app, 1700 * MS);
        let (_, layer) = overlay_frame(&mut app, OverlayKind::Settings, true, from);
        assert_eq!(layer.alpha, 1.0);
        assert_eq!(layer.scale, 1.0);
        assert_eq!(layer.origin, [304.0, 0.0]);
        assert_eq!(layer.offset, [0.0, 0.0]);
        let (overlay, _) = crate::effects::layer_config(
            layer,
            crate::effects::Effect {
                shadow: None,
                inset: 0.0,
                inset_ink: [0.0, 0.0, 0.0, 0.0],
                layer: None,
            },
        );
        // At rest no bracket.
        assert_eq!(overlay[3], 0.0);
        motion_settle(&mut app, 2000 * MS);
        let (presence, _) = overlay_frame(&mut app, OverlayKind::Settings, false, from);
        assert_eq!(presence, mv_motion::Presence::Closing);
        motion_settle(&mut app, 2100 * MS);
        let (_, layer) = overlay_frame(&mut app, OverlayKind::Settings, false, from);
        assert!((layer.alpha - 0.5).abs() < 1e-6 && (layer.offset[1] + 4.0).abs() < 1e-5);
        motion_settle(&mut app, 2200 * MS);
        let (presence, _) = overlay_frame(&mut app, OverlayKind::Settings, false, from);
        assert_eq!(presence, mv_motion::Presence::Closed);
    }

    #[test]
    fn a_parts_own_fade_or_transition_keeps_the_loop_awake_while_it_runs() {
        let mut app = driver_app(false);
        // The first paint over.
        motion_settle(&mut app, 3000 * MS);
        assert!(!motion_moving(&mut app));
        // A part lists its fade as it lays out, every frame: listed once.
        let mut fade = mv_motion::FadeIn {
            start: 0.0,
            duration: 0.0,
            ease: mv_motion::Ease::Linear,
        };
        let mut glide = mv_motion::Transition {
            from: 0.0,
            to: 0.0,
            start: 0.0,
            duration: 0.0,
            ease: mv_motion::Ease::Linear,
            reversing_from: 0.0,
            shortening: 0.0,
        };
        for _ in 0..3 {
            // The watch overloads are one member each; the combined call below
            // is what the parts run.
            motion_watch_fade(&mut app, std::ptr::addr_of_mut!(fade));
            motion_watch_transition(&mut app, std::ptr::addr_of_mut!(glide), false);
        }
        // Q's row, the fade, the glide.
        assert_eq!(app.motion.watching, 3);
        mv_motion::fade_in_start(&mut fade, 3.0, mv_motion::FAST_MS, mv_motion::Ease::Linear);
        assert!(motion_moving(&mut app) && motion_translucent(&mut app));
        motion_settle(&mut app, 3200 * MS);
        assert!(!motion_moving(&mut app));
        // A glide moves no opacity: awake, not translucent.
        mv_motion::transition_to(
            &mut glide,
            100.0,
            3.2,
            mv_motion::BASE_MS / 1000.0,
            mv_motion::Ease::CssEnter,
        );
        assert!(motion_moving(&mut app) && !motion_translucent(&mut app));
        motion_settle(&mut app, 3600 * MS);
        assert!(!motion_moving(&mut app));
    }

    #[test]
    fn the_watch_list_outlives_no_app() {
        // D2's invariant: the watch list keeps a raw pointer into the boxed
        // App (Q's row), registered after its last move as `app_init` does
        // beside `init_clay`. The entry must name Q's row while the App
        // lives, and dropping the App must leave no live list behind: a
        // fresh App starts with an empty list.
        let mut boxed = Box::new(crate::fixture::headless_app());
        motion_init(&mut boxed);
        assert_eq!(boxed.motion.watching, 1);
        let entry = boxed.motion.watched[0];
        assert!(entry.translucent);
        match entry.motion {
            WatchedMotion::Fade(f) => {
                assert_eq!(f, std::ptr::addr_of_mut!(boxed.motion.q_row))
            }
            WatchedMotion::Transition(_) => panic!("Q's row is watched as a fade"),
        }
        drop(boxed);
        let fresh = crate::fixture::headless_app();
        assert_eq!(fresh.motion.watching, 0);
    }
}
