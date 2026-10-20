//! Action dispatch: viewer actions in, model changes out.
//!
//! Widgets dispatch viewer actions through one choke point; hover and focus
//! changes report their model parts, and script model lines apply through the
//! same dispatches. (Declares against the viewer surface; wired when it lands.)

use mv_viewer::{
    Action, Base, EqPointer, Focus, Hover, Param, Part, SetDrag, SetFunction, SetP, SetParam,
};

use crate::app::App;
use crate::app_loop::{Timer, timer_start, timer_stop};
use crate::equation::term_param;
use crate::input::Hit;

/// P's ghost lingers this long after a drag lets go, in ms.
pub const P_GHOST_LINGER_MS: f64 = 600.0;

/// Applies a viewer action, marks dirty, and manages the P-ghost timer.
pub fn dispatch(app: &mut App, action: mv_viewer::Action) {
    let logged = format!("{action:?}");
    if !mv_viewer::apply(&mut app.viewer, action) {
        return;
    }
    // The Debug impls below print the action the way the app's log does.
    if crate::trace() {
        println!("action {logged}");
    }
    app.dirty = true;
    crate::motion_driver::motion_heard(app);
    // A transform's drag let go: P's ghost stays a moment, then the model clears it.
    let (drag_none, p_before_some) = {
        let viewer = &app.viewer;
        (viewer.drag.is_none(), viewer.p_before.is_some())
    };
    if drag_none && p_before_some {
        if app.timers[Timer::PGhost] == 0 {
            timer_start(app, Timer::PGhost, P_GHOST_LINGER_MS);
        }
    } else {
        timer_stop(app, Timer::PGhost);
    }
}

/// The hover hit's model part, when it has one.
fn hover_part(hit: Hit) -> Option<mv_viewer::Part> {
    match hit.kind {
        crate::input::HitKind::Row
        | crate::input::HitKind::Chip
        | crate::input::HitKind::Flip
        | crate::input::HitKind::Box
        | crate::input::HitKind::Reset => param_part(hit.index),
        crate::input::HitKind::Point
        | crate::input::HitKind::Track
        | crate::input::HitKind::PField => Some(Part::P),
        crate::input::HitKind::Equation | crate::input::HitKind::Term => Some(Part::Eq),
        _ => None,
    }
}

/// A parameter index as its part, when it names one.
fn param_part(index: i64) -> Option<Part> {
    match index {
        0 => Some(Part::A),
        1 => Some(Part::K),
        2 => Some(Part::B),
        3 => Some(Part::H),
        _ => None,
    }
}

/// The focused hit's model part: the plane's only when keyboard-driven.
fn focus_part(hit: Hit, keyboard: bool) -> Option<mv_viewer::Part> {
    if hit.kind == crate::input::HitKind::Plane {
        return if keyboard { Some(Part::Plane) } else { None };
    }
    hover_part(hit)
}

/// Dispatches the hover and equation-pointer changes between two hits.
pub fn report_hover(app: &mut App, from: Hit, to: Hit) {
    let (left, entered) = (hover_part(from), hover_part(to));
    if left == entered {
        if entered == Some(Part::Eq) {
            dispatch(
                app,
                Action::EqPointer(EqPointer {
                    over: term_param(to),
                }),
            );
        }
        return;
    }
    if let Some(part) = left {
        dispatch(app, Action::Hover(Hover { part, on: false }));
    }
    if let Some(part) = entered {
        if part == Part::Eq {
            dispatch(
                app,
                Action::EqPointer(EqPointer {
                    over: term_param(to),
                }),
            );
        } else {
            dispatch(app, Action::Hover(Hover { part, on: true }));
        }
    }
}

/// Dispatches the focus leave/enter.
pub fn report_focus(app: &mut App, entered: bool) {
    let part = focus_part(app.input.focus, app.input.focus_visible);
    let current = app.viewer.focus;
    if let Some(c) = current
        && current != part
    {
        dispatch(app, Action::Focus(Focus { part: c, on: false }));
    }
    if let Some(p) = part
        && (current != part || entered)
    {
        dispatch(app, Action::Focus(Focus { part: p, on: true }));
    }
}

/// Applies a script model line through dispatches and P moves.
pub fn apply_model(app: &mut App, m: &mv_script::Model) {
    match m.set {
        mv_script::ModelSet::Fn => {
            for (i, word) in mv_script::MODEL_FNS.iter().enumerate() {
                if m.name.as_str() == *word {
                    dispatch(app, Action::SetFunction(SetFunction { func: base_at(i) }));
                }
            }
        }
        mv_script::ModelSet::Param => {
            for (i, word) in mv_script::MODEL_PARAMS.iter().enumerate() {
                if m.name.as_str() == *word {
                    dispatch(
                        app,
                        Action::SetParam(SetParam {
                            param: param_at(i),
                            value: m.value[0],
                            jump: false,
                        }),
                    );
                }
            }
        }
        mv_script::ModelSet::P => {
            dispatch(app, Action::SetP(SetP { x: m.value[0] }));
        }
        mv_script::ModelSet::Drag => {
            if let Some(part) = app.viewer.drag.as_ref().map(|drag| drag.part) {
                dispatch(app, Action::SetDrag(SetDrag { part, mode: None }));
            }
            for (i, word) in mv_script::MODEL_PARTS.iter().enumerate() {
                if m.name.as_str() == *word && *word != "none" {
                    dispatch(
                        app,
                        Action::SetDrag(SetDrag {
                            part: MODEL_PART_OF[i],
                            mode: Some(mv_scrub::Mode::Coarse),
                        }),
                    );
                }
            }
        }
        mv_script::ModelSet::PBefore => {
            // Where a transform's drag found P: set as the drag that took it leaves it, moved.
            let viewer = &mut app.viewer;
            viewer.p_before = None;
            if m.name != "none" {
                viewer.p_before = Some(mv_viewer::PBefore {
                    x: m.value[0],
                    y: m.value[1],
                    moved: true,
                });
            }
            app.dirty = true;
        }
    }
}

/// The model function at a `MODEL_FNS` index.
fn base_at(index: usize) -> Base {
    match index {
        0 => Base::X,
        1 => Base::X2,
        2 => Base::X3,
        _ => Base::Sin,
    }
}

/// The parameter at a `MODEL_PARAMS` index.
fn param_at(index: usize) -> Param {
    match index {
        0 => Param::A,
        1 => Param::K,
        2 => Param::B,
        _ => Param::H,
    }
}

/// Each model drag part's viewer part.
static MODEL_PART_OF: [mv_viewer::Part; 8] = [
    mv_viewer::Part::A,
    mv_viewer::Part::K,
    mv_viewer::Part::B,
    mv_viewer::Part::H,
    mv_viewer::Part::P,
    mv_viewer::Part::Anchor,
    mv_viewer::Part::Stretch,
    mv_viewer::Part::A,
];
