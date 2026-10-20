//! The plane's pointer, wheel, pinch, key and glide handling.
//!
//! Presses grab marks, start pans, or pan edges into view; releases fling from
//! the pan samples; keys move P, pan square, step zoom and reset. (Declares
//! against the plane and viewer surfaces; wired when they land.)

use sdl3::keyboard::Keycode;
use sdl3::keyboard::Mod;

use crate::app::App;
use crate::app_loop::Time;
use crate::input::Cursor;

/// The key pan step.
pub const P_KEY_STEP: f32 = 0.1;

/// What the pointer holds: a pan or a mark drag.
#[derive(Clone, Debug)]
pub enum PlaneGrip {
    Pan(PanGrip),
    Mark(MarkGrip),
}

/// A pan: its last position.
#[derive(Clone, Copy, Debug)]
pub struct PanGrip {
    pub last: [f64; 2],
}

/// A mark drag: whose, which handle, from where, at where.
#[derive(Clone, Copy, Debug)]
pub struct MarkGrip {
    pub mark: mv_viewer::PlaneMark,
    pub handle: bool,
    pub from: [f64; 2],
    pub at: [f64; 2],
}

/// Time in ms.
fn ms(t: Time) -> f64 {
    t as f64 / crate::app_loop::MS as f64
}

/// The pointer less the plane box origin.
fn plane_pointer(app: &mut App) -> [f64; 2] {
    let b = app.plane.r#box;
    [
        f64::from(app.input.pointer[0]) - f64::from(b.x),
        f64::from(app.input.pointer[1]) - f64::from(b.y),
    ]
}

/// The pointer through the current viewport to maths x/y.
fn plane_math(app: &mut App) -> [f64; 2] {
    let vp = crate::plane_area::plane_viewport(app, app.plane.r#box);
    let at = plane_pointer(app);
    [
        mv_plane::to_math_x(vp, at[0]),
        mv_plane::to_math_y(vp, at[1]),
    ]
}

/// Assigns the view when it changed, and dirties.
pub fn plane_set_view(app: &mut App, view: &mv_plane::ViewState) {
    if *view != app.plane.view {
        app.plane.view = *view;
        app.dirty = true;
    }
}

/// Clears the glide.
pub fn plane_stop_glide(app: &mut App) {
    app.plane.glide = None;
}

/// Whether a glide runs.
pub fn plane_gliding(app: &mut App) -> bool {
    app.plane.glide.is_some()
}

/// Steps the glide by the elapsed time, pans the view, ends below the floor.
pub fn plane_glide(app: &mut App, now: Time) {
    let Some(mut glide) = app.plane.glide.take() else {
        return;
    };
    let (step, still) = mv_plane::glide_step(&mut glide, ms(now), app.plane.view.zoom);
    if !still {
        // Odin `log.debug`: silent unless `--trace`, and to stdout when it
        // prints (a console error counts as a problem on a scripted web run).
        if crate::trace() {
            println!("plane: the glide stopped");
        }
        return;
    }
    app.plane.glide = Some(glide);
    let mut view = app.plane.view;
    view.pan_x += step[0];
    view.pan_y -= step[1];
    plane_set_view(app, &view);
}

/// The scene point or handle for an id.
fn mark_position(app: &mut App, id: &str) -> ([f64; 2], bool) {
    let scene = mv_viewer::build_plane_scene(&mv_viewer::plane_scene_input(&app.viewer));
    for p in &scene.points {
        if p.id.as_str() == id {
            return ([p.x, p.y], true);
        }
    }
    for h in &scene.handles {
        if h.id.as_str() == id {
            return ([h.x, h.y], true);
        }
    }
    ([0.0, 0.0], false)
}

/// A mark's model part.
fn mark_part(mark: mv_viewer::PlaneMark) -> mv_viewer::Part {
    match mark {
        mv_viewer::PlaneMark::P => mv_viewer::Part::P,
        mv_viewer::PlaneMark::Anchor => mv_viewer::Part::Anchor,
        mv_viewer::PlaneMark::Stretch => mv_viewer::Part::Stretch,
    }
}

/// Edge pans into view, a mark grabs, else a pan starts; stops the glide.
pub fn plane_press(app: &mut App) {
    plane_stop_glide(app);
    let at = plane_pointer(app);
    if let (hit, true) = mv_plane::hit_test(&app.plane.hits, at[0], at[1]) {
        if hit.kind == mv_plane::HitKind::Edge {
            let vp = crate::plane_area::plane_viewport(app, app.plane.r#box);
            let view = mv_plane::pan_into_view(vp, app.plane.view, hit.target);
            plane_set_view(app, &view);
            app.plane.still = Some(Cursor::Pointer);
            return;
        }
        let is_mark = mv_viewer::plane_mark(&hit.id).is_some();
        let (position, has) = mark_position(app, &hit.id);
        if is_mark && has {
            let mark = mv_viewer::plane_mark(&hit.id).unwrap_or(mv_viewer::PlaneMark::P);
            app.plane.grip = Some(PlaneGrip::Mark(MarkGrip {
                mark,
                handle: hit.kind == mv_plane::HitKind::Handle,
                from: plane_math(app),
                at: position,
            }));
            crate::parts::dispatch(
                app,
                mv_viewer::Action::SetDrag(mv_viewer::SetDrag {
                    part: mark_part(mark),
                    mode: Some(mv_scrub::Mode::Coarse),
                }),
            );
            return;
        }
    }
    let pointer = [
        f64::from(app.input.pointer[0]),
        f64::from(app.input.pointer[1]),
    ];
    app.plane.grip = Some(PlaneGrip::Pan(PanGrip { last: pointer }));
    app.plane.samples.clear();
    app.plane.samples.push(mv_plane::DragSample {
        at: pointer,
        t: ms(app.input.time),
    });
}

/// The pan follows the pointer with samples; a mark drags and dispatches.
pub fn plane_moved(app: &mut App) {
    let grip = app.plane.grip.clone();
    match grip {
        Some(PlaneGrip::Pan(mut g)) => {
            let pointer = [
                f64::from(app.input.pointer[0]),
                f64::from(app.input.pointer[1]),
            ];
            let d = [pointer[0] - g.last[0], pointer[1] - g.last[1]];
            g.last = pointer;
            app.plane.grip = Some(PlaneGrip::Pan(g));
            let t = ms(app.input.time);
            app.plane
                .samples
                .push(mv_plane::DragSample { at: pointer, t });
            mv_plane::prune_samples(&mut app.plane.samples);
            let mut view = app.plane.view;
            view.pan_x += d[0] / view.zoom;
            view.pan_y -= d[1] / view.zoom;
            plane_set_view(app, &view);
        }
        Some(PlaneGrip::Mark(g)) => {
            let lock = g.handle && app.input.mods.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD);
            let to = mv_plane::drag_to(g.from, g.at, plane_math(app), lock);
            let action = match g.mark {
                mv_viewer::PlaneMark::P => mv_viewer::Action::SetP(mv_viewer::SetP { x: to[0] }),
                mv_viewer::PlaneMark::Anchor => {
                    mv_viewer::Action::DragHandle(mv_viewer::DragHandle {
                        handle: mv_viewer::Handle::Anchor,
                        to,
                    })
                }
                mv_viewer::PlaneMark::Stretch => {
                    mv_viewer::Action::DragHandle(mv_viewer::DragHandle {
                        handle: mv_viewer::Handle::Stretch,
                        to,
                    })
                }
            };
            crate::parts::dispatch(app, action);
        }
        None => {}
    }
}

/// The pan may fling from its samples; the mark ends; clears the grip.
pub fn plane_release(app: &mut App) {
    let grip = app.plane.grip.clone();
    match grip {
        Some(PlaneGrip::Pan(_)) => {
            if app.plane.samples.len() >= 2 {
                let pointer = [
                    f64::from(app.input.pointer[0]),
                    f64::from(app.input.pointer[1]),
                ];
                let t = ms(app.input.time);
                app.plane
                    .samples
                    .push(mv_plane::DragSample { at: pointer, t });
                let (v, fling) = mv_plane::velocity_of_samples(&app.plane.samples);
                if fling {
                    let (glide, launched) = mv_plane::glide_start(v, ms(app.input.time));
                    if launched {
                        app.plane.glide = Some(glide);
                        // Odin `log.debugf`: silent unless `--trace`, stdout
                        // when it prints (see above).
                        if crate::trace() {
                            println!(
                                "plane: a fling at {:.1}, {:.1} px/s",
                                glide.v[0], glide.v[1]
                            );
                        }
                    }
                }
            }
            app.plane.samples.clear();
        }
        Some(PlaneGrip::Mark(g)) => {
            crate::parts::dispatch(
                app,
                mv_viewer::Action::SetDrag(mv_viewer::SetDrag {
                    part: mark_part(g.mark),
                    mode: None,
                }),
            );
            app.plane.still = Some(Cursor::Grab);
        }
        None => {
            app.plane.still = None;
        }
    }
    app.plane.grip = None;
}

/// Ends the mark drag and clears grip, still and samples, with no fling.
pub fn plane_cancel(app: &mut App) {
    let grip = app.plane.grip.clone();
    if let Some(PlaneGrip::Mark(g)) = grip {
        crate::parts::dispatch(
            app,
            mv_viewer::Action::SetDrag(mv_viewer::SetDrag {
                part: mark_part(g.mark),
                mode: None,
            }),
        );
    }
    app.plane.grip = None;
    app.plane.still = None;
    app.plane.samples.clear();
}

/// Clears the still report.
pub fn plane_pointer_moved(app: &mut App) {
    app.plane.still = None;
}

/// Stops the glide and zooms about the pointer.
pub fn plane_wheel(app: &mut App, delta: f64) {
    plane_stop_glide(app);
    let b = app.plane.r#box;
    let center = [
        (f64::from(b.width) / 2.0).floor(),
        (f64::from(b.height) / 2.0).floor(),
    ];
    let at = plane_pointer(app);
    let (next, changed) = mv_plane::wheel_view(app.plane.view, delta, at, center);
    if changed {
        plane_set_view(app, &next);
    }
}

/// A positive scale zooms by its log distance.
pub fn plane_pinch(app: &mut App, scale: f32) {
    if scale > 0.0 {
        plane_wheel(app, -100.0 * f64::from(scale).ln());
    }
}

/// Brackets move P, arrows pan square, +/− step zoom, 0 resets; stops the
/// glide.
pub fn plane_key(app: &mut App, key: Keycode, mods: Mod) -> bool {
    if mods.intersects(
        Mod::LCTRLMOD | Mod::RCTRLMOD | Mod::LGUIMOD | Mod::RGUIMOD | Mod::LALTMOD | Mod::RALTMOD,
    ) {
        return false;
    }
    match key {
        Keycode::LeftBracket => {
            crate::parts::dispatch(
                app,
                mv_viewer::Action::SetP(mv_viewer::SetP {
                    x: app.viewer.p_x - f64::from(P_KEY_STEP),
                }),
            );
            return true;
        }
        Keycode::RightBracket => {
            crate::parts::dispatch(
                app,
                mv_viewer::Action::SetP(mv_viewer::SetP {
                    x: app.viewer.p_x + f64::from(P_KEY_STEP),
                }),
            );
            return true;
        }
        _ => {}
    }
    let mut view = app.plane.view;
    let square = mv_plane::pan_square(view.zoom);
    match key {
        Keycode::Left => view.pan_x += square,
        Keycode::Right => view.pan_x -= square,
        Keycode::Up => view.pan_y -= square,
        Keycode::Down => view.pan_y += square,
        Keycode::Plus | Keycode::Equals | Keycode::KpPlus => {
            view.zoom = mv_plane::step_zoom(view.zoom, 1)
        }
        Keycode::Minus | Keycode::Underscore | Keycode::KpMinus => {
            view.zoom = mv_plane::step_zoom(view.zoom, -1)
        }
        Keycode::_0 | Keycode::Kp0 => {
            view = mv_plane::ViewState {
                zoom: app.plane.fit,
                pan_x: 0.0,
                pan_y: 0.0,
            }
        }
        _ => return false,
    }
    plane_stop_glide(app);
    plane_set_view(app, &view);
    true
}

/// Steps the zoom stop at the centre; no-op at the limit; stops the glide.
pub fn plane_zoom_step(app: &mut App, direction: i64) {
    let mut view = app.plane.view;
    view.zoom = mv_plane::step_zoom(view.zoom, direction);
    if view.zoom == app.plane.view.zoom {
        return;
    }
    plane_stop_glide(app);
    plane_set_view(app, &view);
}

/// Zeroes the pan, keeps the zoom; stops the glide.
pub fn plane_to_origin(app: &mut App) {
    plane_stop_glide(app);
    let mut view = app.plane.view;
    view.pan_x = 0.0;
    view.pan_y = 0.0;
    plane_set_view(app, &view);
}

/// Grabbing while held, the still cursor, edges and marks, else crosshair.
pub fn plane_cursor(app: &mut App) -> Cursor {
    if app.plane.grip.is_some() {
        return Cursor::Grabbing;
    }
    if let Some(cursor) = app.plane.still {
        return cursor;
    }
    let at = plane_pointer(app);
    if let (hit, true) = mv_plane::hit_test(&app.plane.hits, at[0], at[1]) {
        if hit.kind == mv_plane::HitKind::Edge {
            return Cursor::Pointer;
        }
        return Cursor::Grab;
    }
    Cursor::Crosshair
}

/// Dispatches the pointer report (panning, mark, hover, empty) unless still.
pub fn report_plane_pointer(app: &mut App) {
    if app.plane.still.is_some() {
        return;
    }
    let grip = app.plane.grip.clone();
    match grip {
        Some(PlaneGrip::Pan(_)) => {
            let x = plane_math(app)[0];
            crate::parts::dispatch(
                app,
                mv_viewer::Action::PlanePointer(mv_viewer::PlanePointer {
                    x: Some(x),
                    over: None,
                    panning: true,
                }),
            );
            return;
        }
        Some(PlaneGrip::Mark(g)) => {
            let x = plane_math(app)[0];
            crate::parts::dispatch(
                app,
                mv_viewer::Action::PlanePointer(mv_viewer::PlanePointer {
                    x: Some(x),
                    over: Some(g.mark),
                    panning: false,
                }),
            );
            return;
        }
        None => {}
    }
    if !app.input.over || app.input.hover.kind != crate::input::HitKind::Plane {
        crate::parts::dispatch(
            app,
            mv_viewer::Action::PlanePointer(mv_viewer::PlanePointer {
                x: None,
                over: None,
                panning: false,
            }),
        );
        return;
    }
    let at = plane_pointer(app);
    let mut over = None;
    if let (hit, true) = mv_plane::hit_test(&app.plane.hits, at[0], at[1])
        && hit.kind != mv_plane::HitKind::Edge
    {
        over = mv_viewer::plane_mark(&hit.id);
    }
    let x = plane_math(app)[0];
    crate::parts::dispatch(
        app,
        mv_viewer::Action::PlanePointer(mv_viewer::PlanePointer {
            x: Some(x),
            over,
            panning: false,
        }),
    );
}
