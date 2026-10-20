//! Reads of the state: what shows, what glows, and what P did.

use enumset::EnumSet;

use super::form::{Param, is_identity};
use super::state::{
    FieldEdit, HANDLE_PARAMS, Handle, Lead, Part, PlaneMark, State, curve_at, part_param,
};

/// Whether the transform differs from the base function.
pub fn is_transformed(state: &State) -> bool {
    !is_identity(state.params)
}

/// Whether the ghost curve shows.
pub fn ghost_visible(state: &State) -> bool {
    state.ghost_on && is_transformed(state)
}

/// Whether P lights up.
pub fn p_lit(state: &State) -> bool {
    if let Some(drag) = state.drag
        && drag.part == Part::P
    {
        return true;
    }
    state.hover == Some(Part::P)
        || state.focus == Some(Part::P)
        || (state.hover == Some(Part::Plane) && state.plane_over == Some(PlaneMark::P))
}

/// What a value or handle edit acts on: a parameter or a handle, if any.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Active {
    Param(Param),
    Handle(Handle),
}

/// What the state acts on: the edit, drag, explainer, or the lead's part.
fn active_of(state: &State, part: Option<Part>) -> Option<Active> {
    let p = part?;
    let (param, is_param) = part_param(p);
    if is_param {
        return Some(Active::Param(param));
    }
    match p {
        Part::Anchor => Some(Active::Handle(Handle::Anchor)),
        Part::Stretch => Some(Active::Handle(Handle::Stretch)),
        Part::Eq => state.eq_over.map(Active::Param),
        Part::Plane => mark_handle(state.plane_over).map(Active::Handle),
        _ => None,
    }
}

/// What the state acts on.
pub fn active(state: &State) -> Option<Active> {
    if let Some(edit) = &state.edit {
        if edit.error.is_empty() {
            return active_of(state, Some(edit.part));
        }
        return None;
    }
    if let Some(drag) = &state.drag {
        return active_of(state, Some(drag.part));
    }
    if let Some(e) = &state.explainer {
        return Some(Active::Param(e.param));
    }
    let (first, second) = if state.lead == Lead::Focus {
        (state.focus, state.hover)
    } else {
        (state.hover, state.focus)
    };
    active_of(state, first.or(second))
}

/// The parameters the state's action touches.
pub fn active_params(state: &State) -> EnumSet<Param> {
    match active(state) {
        Some(Active::Param(p)) => EnumSet::only(p),
        Some(Active::Handle(h)) => HANDLE_PARAMS[h],
        None => EnumSet::empty(),
    }
}

/// The parameters a part touches.
pub fn params_of(part: Part) -> EnumSet<Param> {
    let (param, is_param) = part_param(part);
    if is_param {
        return EnumSet::only(param);
    }
    if let Some(h) = handle_of(part) {
        return HANDLE_PARAMS[h];
    }
    EnumSet::empty()
}

/// The parameters a drag moves.
pub fn dragged_params(state: &State) -> EnumSet<Param> {
    match state.drag {
        Some(drag) => params_of(drag.part),
        None => EnumSet::empty(),
    }
}

/// The parameters a drag fines.
pub fn fine_params(state: &State) -> EnumSet<Param> {
    match state.drag {
        Some(drag) => drag.fine,
        None => EnumSet::empty(),
    }
}

/// How a part shows: hovered, lit, its drag mode, fineness and edit.
#[derive(Clone, Debug, PartialEq)]
pub struct PartUi {
    pub hovered: bool,
    pub lit: bool,
    pub mode: Option<mv_scrub::Mode>,
    pub fine: bool,
    pub edit: Option<FieldEdit>,
}

/// How a part shows.
pub fn part_ui(state: &State, part: Part) -> PartUi {
    let mut ui = PartUi {
        hovered: state.hover == Some(part),
        lit: false,
        mode: None,
        fine: false,
        edit: None,
    };
    let (param, is_param) = part_param(part);
    ui.lit = is_param && active_params(state).contains(param);
    if let Some(drag) = state.drag {
        if drag.part == part {
            ui.mode = Some(drag.mode);
        } else if is_param && handle_of(drag.part).is_some_and(|h| HANDLE_PARAMS[h].contains(param))
        {
            ui.mode = Some(mv_scrub::Mode::Coarse);
        }
        ui.fine = is_param && drag.fine.contains(param);
    }
    if let Some(edit) = &state.edit
        && edit.part == part
    {
        ui.edit = Some(FieldEdit {
            error: edit.error.clone(),
            base: edit.base,
        });
    }
    ui
}

/// The lit handle, if any.
pub fn handle_lit(state: &State) -> Option<Handle> {
    if let Some(drag) = state.drag {
        return handle_of(drag.part);
    }
    if state.hover == Some(Part::Plane) {
        return mark_handle(state.plane_over);
    }
    None
}

/// The held handle, if any.
pub fn handle_held(state: &State) -> Option<Handle> {
    state.drag.and_then(|drag| handle_of(drag.part))
}

/// A plane mark's handle, if it is one.
fn mark_handle(mark: Option<PlaneMark>) -> Option<Handle> {
    match mark {
        Some(PlaneMark::Anchor) => Some(Handle::Anchor),
        Some(PlaneMark::Stretch) => Some(Handle::Stretch),
        _ => None,
    }
}

/// A part's handle, if it is one.
fn handle_of(part: Part) -> Option<Handle> {
    match part {
        Part::Anchor => Some(Handle::Anchor),
        Part::Stretch => Some(Handle::Stretch),
        _ => None,
    }
}

/// Where P moved to, if it moved.
pub fn p_moved(state: &State) -> Option<f64> {
    let was = state.p_before?;
    if !was.moved || was.x != state.p_x {
        return None;
    }
    Some(curve_at(state, state.p_x) - was.y)
}

/// Where P was, if it left a trail.
pub fn p_was(state: &State) -> Option<f64> {
    match state.p_before {
        Some(before) if before.x == state.p_x => Some(before.y),
        _ => None,
    }
}
