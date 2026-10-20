//! The reducer: every action that changes the state.

use enumset::EnumSet;

use super::form::{
    BASE_FUNCTIONS, Base, DEFAULT_PARAMS, Param, accepts_value, drag_anchor, drag_stretch,
};
use super::selectors::params_of;
use super::state::{
    Change, Drag, Edit, ExplainBy, Explainer, FieldEdit, Handle, Lead, PBefore, Part, PlaneMark,
    State, curve_at, part_param,
};

/// Points drag on the display lattice.
pub const POINT_QUANTUM: f64 = 0.01;

/// Sets the base function.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SetFunction {
    pub func: Base,
}

/// Sets a parameter, jumping or sliding to it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SetParam {
    pub param: Param,
    pub value: f64,
    pub jump: bool,
}

/// Resets a parameter to its default.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResetParam {
    pub param: Param,
}

/// Resets everything.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResetAll;

/// Flips a parameter's sign.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Flip {
    pub param: Param,
}

/// Pins P at x.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SetP {
    pub x: f64,
}

/// Moves the pointer over the plane.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlanePointer {
    pub x: Option<f64>,
    pub over: Option<PlaneMark>,
    pub panning: bool,
}

/// Shows or hides the ghost curve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SetGhost {
    pub on: bool,
}

/// Sets the saved view.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SetView {
    pub view: mv_plane::View,
}

/// Hovers a part.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hover {
    pub part: Part,
    pub on: bool,
}

/// Focuses a part.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Focus {
    pub part: Part,
    pub on: bool,
}

/// Starts or stops a drag.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SetDrag {
    pub part: Part,
    pub mode: Option<mv_scrub::Mode>,
}

/// Starts or stops an edit.
#[derive(Clone, Debug, PartialEq)]
pub struct SetEdit {
    pub part: Part,
    pub edit: Option<FieldEdit>,
}

/// Hovers the equation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EqPointer {
    pub over: Option<Param>,
}

/// Drags a handle to a point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DragHandle {
    pub handle: Handle,
    pub to: [f64; 2],
}

/// Clears P's trail.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClearPBefore;

/// Explains a parameter, opening, closing or toggling.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Explain {
    pub param: Param,
    pub by: ExplainBy,
    pub open: ExplainOpen,
}

/// How an explanation opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExplainOpen {
    Close,
    Open,
    Toggle,
}

/// Escapes: closes what is open.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Escape;

/// An action: everything that changes the state.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    SetFunction(SetFunction),
    SetParam(SetParam),
    ResetParam(ResetParam),
    ResetAll(ResetAll),
    Flip(Flip),
    SetP(SetP),
    PlanePointer(PlanePointer),
    SetGhost(SetGhost),
    SetView(SetView),
    Hover(Hover),
    Focus(Focus),
    SetDrag(SetDrag),
    SetEdit(SetEdit),
    EqPointer(EqPointer),
    DragHandle(DragHandle),
    ClearPBefore(ClearPBefore),
    Explain(Explain),
    Escape(Escape),
}

/// Applies an action to the state, reporting whether it changed.
pub fn apply(state: &mut State, action: Action) -> bool {
    let before = state.clone();
    let next = track_p(
        &before,
        tag_change(&before, &action, transition(before.clone(), &action)),
    );
    let changed = next != before;
    *state = next;
    changed
}

/// Whether a value is finite.
fn finite(v: f64) -> bool {
    !v.is_nan() && !v.is_infinite()
}

/// Whether a part drags the transform.
fn is_transform_drag(part: Part) -> bool {
    let (_, is_param) = part_param(part);
    is_param || part == Part::Anchor || part == Part::Stretch
}

/// A parameter's control part: the inverse of `part_param` over the parameters.
fn param_part(param: Param) -> Part {
    match param {
        Param::A => Part::A,
        Param::K => Part::K,
        Param::B => Part::B,
        Param::H => Part::H,
    }
}

/// The state an action moves to.
fn transition(state: State, action: &Action) -> State {
    let mut next = state.clone();
    match action {
        Action::SetFunction(a) => {
            next.func = a.func;
        }
        Action::SetParam(a) => {
            // No hidden digits: a stored value prints whole at 3 decimals, never -0;
            // a scale never stores 0.
            if !finite(a.value) {
                return next;
            }
            let mut value = mv_numfmt::round_to(a.value, mv_numfmt::FINE_DP);
            // `|| 0`: round_to keeps a sign on 0.
            if value == 0.0 {
                value = 0.0;
            }
            if accepts_value(a.param, value) {
                next.params[a.param] = value;
            }
        }
        Action::ResetParam(a) => {
            next.params[a.param] = DEFAULT_PARAMS[a.param];
        }
        Action::ResetAll(_) => {
            next.params = DEFAULT_PARAMS;
        }
        Action::Flip(a) => {
            let flipped = -state.params[a.param];
            next.params[a.param] = if flipped == 0.0 { 0.0 } else { flipped };
        }
        Action::SetP(a) => {
            if finite(a.x) {
                next.p_x = mv_numfmt::quantize(a.x, POINT_QUANTUM);
            }
        }
        Action::PlanePointer(a) => match a.x {
            Some(x) if finite(x) => {
                next.q_x = None;
                if a.over.is_none() && !a.panning {
                    next.q_x = Some(mv_numfmt::quantize(x, POINT_QUANTUM));
                }
                next.plane_over = a.over;
                next.hover = Some(Part::Plane);
                next.lead = Lead::Hover;
            }
            _ => {
                next.q_x = None;
                next.plane_over = None;
                if state.hover == Some(Part::Plane) {
                    next.hover = None;
                }
                return next;
            }
        },
        Action::SetGhost(a) => {
            next.ghost_on = a.on;
        }
        Action::SetView(a) => {
            next.view = Some(a.view);
        }
        Action::Hover(a) => {
            if a.on {
                next.hover = Some(a.part);
                next.lead = Lead::Hover;
            } else if state.hover == Some(a.part) {
                next.hover = None;
                if a.part == Part::Eq {
                    next.eq_over = None;
                }
            }
        }
        Action::EqPointer(a) => {
            next.hover = Some(Part::Eq);
            next.eq_over = a.over;
            next.lead = Lead::Hover;
        }
        Action::DragHandle(a) => {
            if !finite(a.to[0]) || !finite(a.to[1]) {
                return next;
            }
            if a.handle == Handle::Anchor {
                next.params = drag_anchor(state.params, a.to);
            } else {
                next.params = drag_stretch(&BASE_FUNCTIONS[state.func], state.params, a.to);
            }
        }
        Action::ClearPBefore(_) => {
            if state.drag.is_none() {
                next.p_before = None;
            }
        }
        Action::Focus(a) => {
            if a.on {
                next.focus = Some(a.part);
                next.lead = Lead::Focus;
            } else if state.focus == Some(a.part) {
                next.focus = None;
                // The focus leaving a control ends the explainer its ? opened.
                if let Some(e) = state.explainer
                    && e.by == ExplainBy::Key
                    && param_part(e.param) == a.part
                {
                    next.explainer = None;
                }
            }
        }
        Action::Explain(a) => {
            let current = state.explainer;
            let is_open = current.is_some_and(|e| e.param == a.param);
            let open = if a.open == ExplainOpen::Toggle {
                !is_open
            } else {
                a.open == ExplainOpen::Open
            };
            if open {
                if !is_open || current.is_some_and(|e| e.by != a.by) {
                    next.explainer = Some(Explainer {
                        param: a.param,
                        by: a.by,
                    });
                }
            } else if is_open
                && (a.open == ExplainOpen::Toggle || current.is_some_and(|e| e.by == a.by))
            {
                next.explainer = None;
            }
        }
        Action::Escape(_) => {
            next.explainer = None;
        }
        Action::SetDrag(a) => {
            let current = state.drag;
            match a.mode {
                None => {
                    if let Some(held) = current
                        && held.part == a.part
                    {
                        next.drag = None;
                    }
                    return next;
                }
                Some(mode) => {
                    if let Some(held) = current
                        && held.part == a.part
                    {
                        // Gone fine: what it moves keeps 3 decimals until it lets go.
                        let fine = if mode == mv_scrub::Mode::Fine {
                            params_of(a.part)
                        } else {
                            held.fine
                        };
                        next.drag = Some(Drag {
                            part: a.part,
                            mode,
                            fine,
                        });
                        return next;
                    }
                    // A new drag holds 3 decimals for the values it finds with a third (all of
                    // them if it starts fine); a transform's takes P where it is now.
                    let mut fine = EnumSet::new();
                    for p in params_of(a.part) {
                        if mode == mv_scrub::Mode::Fine
                            || mv_numfmt::stored_dp(state.params[p], false) == mv_numfmt::FINE_DP
                        {
                            fine.insert(p);
                        }
                    }
                    next.drag = Some(Drag {
                        part: a.part,
                        mode,
                        fine,
                    });
                    next.p_before = None;
                    if is_transform_drag(a.part) {
                        next.p_before = Some(PBefore {
                            x: state.p_x,
                            y: curve_at(&state, state.p_x),
                            moved: false,
                        });
                    }
                }
            }
        }
        Action::SetEdit(a) => match &a.edit {
            None => {
                if let Some(current) = &state.edit
                    && current.part == a.part
                {
                    next.edit = None;
                }
                return next;
            }
            Some(edit) => {
                next.edit = Some(Edit {
                    part: a.part,
                    error: edit.error.clone(),
                    base: edit.base,
                });
            }
        },
    }
    next
}

/// How an action changes the curve, for the plane's motion.
fn change_of(action: &Action) -> Change {
    match action {
        Action::SetFunction(_) => Change::Switch,
        Action::ResetParam(_) | Action::ResetAll(_) | Action::Flip(_) => Change::Jump,
        Action::SetParam(a) => {
            if a.jump {
                Change::Jump
            } else {
                Change::Direct
            }
        }
        _ => Change::Direct,
    }
}

/// Says how the curve changed, when it did.
fn tag_change(before: &State, action: &Action, next: State) -> State {
    if next.params == before.params && next.func == before.func {
        return next;
    }
    let mut tagged = next;
    tagged.change = change_of(action);
    tagged
}

/// Tracks P's trail across the change.
fn track_p(before: &State, next: State) -> State {
    let Some(was) = next.p_before else {
        return next;
    };
    if next == *before {
        return next;
    }
    let curve_changed = next.params != before.params || next.func != before.func;
    let dragging = next.drag.is_some_and(|drag| is_transform_drag(drag.part));
    if !next.drag.is_some() || !dragging || next.p_x != before.p_x {
        if !curve_changed && next.p_x == before.p_x {
            return next;
        }
        let mut cleared = next;
        cleared.p_before = None;
        return cleared;
    }
    if curve_changed && !was.moved && (curve_at(&next, next.p_x) - was.y).abs() > 1e-9 {
        let mut moved = next;
        moved.p_before = Some(PBefore {
            x: was.x,
            y: was.y,
            moved: true,
        });
        return moved;
    }
    next
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::form::{Params, ZERO_SCALE};
    use crate::selectors::{
        Active, PartUi, active, active_params, dragged_params, fine_params, ghost_visible,
        handle_held, handle_lit, is_transformed, p_lit, p_moved, part_ui,
    };
    use crate::state::{FieldEdit, OffView, PlaneMark, initial_state, p_off_view};
    use mv_plane::{Extent, View};
    use mv_scrub::Mode;

    /// Applies each action in turn, starting from the initial state.
    fn run(actions: Vec<Action>) -> State {
        let mut state = initial_state();
        for action in actions {
            apply(&mut state, action);
        }
        state
    }

    /// R3: the shared transform the tests build on.
    const R3: [Action; 3] = [
        Action::SetParam(SetParam {
            param: Param::A,
            value: 2.0,
            jump: false,
        }),
        Action::SetParam(SetParam {
            param: Param::H,
            value: -1.0,
            jump: false,
        }),
        Action::SetParam(SetParam {
            param: Param::K,
            value: 1.0,
            jump: false,
        }),
    ];

    /// Heads a shared prefix onto more actions.
    fn with(head: &[Action], tail: Vec<Action>) -> Vec<Action> {
        let mut all = head.to_vec();
        all.extend(tail);
        all
    }

    /// Parameters in call order: a, b, h, k.
    fn params(a: f64, b: f64, h: f64, k: f64) -> Params {
        let mut m = DEFAULT_PARAMS;
        m[Param::A] = a;
        m[Param::B] = b;
        m[Param::H] = h;
        m[Param::K] = k;
        m
    }

    /// R5's start: R3 with P at 0.5 and k's box dragging.
    const R5_START: [Action; 4] = [
        Action::SetParam(SetParam {
            param: Param::A,
            value: 2.0,
            jump: false,
        }),
        Action::SetParam(SetParam {
            param: Param::H,
            value: -1.0,
            jump: false,
        }),
        Action::SetP(SetP { x: 0.5 }),
        Action::SetDrag(SetDrag {
            part: Part::K,
            mode: Some(Mode::Coarse),
        }),
    ];

    /// Opens an explainer by one means.
    fn open(param: Param, by: ExplainBy) -> Action {
        Action::Explain(Explain {
            param,
            by,
            open: ExplainOpen::Open,
        })
    }

    #[test]
    fn starts_at_x_squared_untransformed() {
        let s = initial_state();
        assert_eq!(s.func, Base::X2);
        assert_eq!(s.params, DEFAULT_PARAMS);
        assert_eq!(s.p_x, 2.0);
        assert!(s.ghost_on);
        assert_eq!(s.q_x, None);
        assert_eq!(s.change, Change::Direct);
    }

    #[test]
    fn reaches_r3() {
        let s = run(with(&R3, vec![Action::SetP(SetP { x: 0.5 })]));
        assert_eq!(curve_at(&s, s.p_x), 5.5);
        assert_eq!(curve_at(&s, -1.0), 1.0);
        assert!(curve_at(&s, -1.5) > 1.0);
        assert!(curve_at(&s, -0.5) > 1.0);
    }

    #[test]
    fn stores_at_most_3_decimals() {
        assert_eq!(
            run(vec![Action::SetParam(SetParam {
                param: Param::B,
                value: 1.03511,
                jump: false,
            })])
            .params[Param::B],
            1.035
        );
        assert_eq!(
            run(vec![Action::SetParam(SetParam {
                param: Param::H,
                value: 0.0004,
                jump: false,
            })])
            .params[Param::H],
            0.0
        );
        let k = run(vec![Action::SetParam(SetParam {
            param: Param::K,
            value: -0.0001,
            jump: false,
        })])
        .params[Param::K];
        assert!(k == 0.0 && !k.is_sign_negative(), "no -0 in the state");
    }

    #[test]
    fn never_stores_a_scale_of_0() {
        let mut s = initial_state();
        assert!(!apply(
            &mut s,
            Action::SetParam(SetParam {
                param: Param::B,
                value: 0.0,
                jump: false,
            })
        ));
        assert!(!apply(
            &mut s,
            Action::SetParam(SetParam {
                param: Param::A,
                value: 0.0004,
                jump: false,
            })
        ));
        assert_eq!(s, initial_state());
        assert_eq!(
            run(vec![
                Action::SetParam(SetParam {
                    param: Param::K,
                    value: 1.0,
                    jump: false,
                }),
                Action::SetParam(SetParam {
                    param: Param::K,
                    value: 0.0,
                    jump: false,
                }),
            ])
            .params[Param::K],
            0.0
        );
    }

    #[test]
    fn ignores_values_that_are_not_numbers() {
        let mut s = initial_state();
        assert!(!apply(
            &mut s,
            Action::SetParam(SetParam {
                param: Param::A,
                value: f64::NAN,
                jump: false,
            })
        ));
        assert!(!apply(
            &mut s,
            Action::SetParam(SetParam {
                param: Param::H,
                value: f64::NAN,
                jump: false,
            })
        ));
        assert!(!apply(&mut s, Action::SetP(SetP { x: f64::INFINITY })));
    }

    #[test]
    fn flips_a_scale_and_resets_one_or_all() {
        let flipped = run(with(&R3, vec![Action::Flip(Flip { param: Param::A })]));
        assert_eq!(flipped.params[Param::A], -2.0);
        let mut one = flipped.clone();
        apply(&mut one, Action::ResetParam(ResetParam { param: Param::A }));
        assert_eq!(one.params, params(1.0, 1.0, -1.0, 1.0));
        let mut all = flipped;
        apply(&mut all, Action::ResetAll(ResetAll));
        assert_eq!(all.params, DEFAULT_PARAMS);
    }

    #[test]
    fn keeps_p_x_through_every_transform() {
        let mut s = run(vec![Action::SetP(SetP { x: 0.5 })]);
        for action in with(
            &R3,
            vec![
                Action::SetParam(SetParam {
                    param: Param::B,
                    value: 0.5,
                    jump: false,
                }),
                Action::Flip(Flip { param: Param::A }),
                Action::Flip(Flip { param: Param::B }),
                Action::ResetParam(ResetParam { param: Param::H }),
                Action::SetFunction(SetFunction { func: Base::Sin }),
                Action::ResetAll(ResetAll),
            ],
        ) {
            apply(&mut s, action);
            assert_eq!(s.p_x, 0.5);
        }
    }

    #[test]
    fn puts_p_and_q_on_the_lattice() {
        assert_eq!(run(vec![Action::SetP(SetP { x: 0.503 })]).p_x, 0.5);
        assert_eq!(
            run(vec![Action::PlanePointer(PlanePointer {
                x: Some(-1.4987),
                over: None,
                panning: false,
            })])
            .q_x,
            Some(-1.5)
        );
        assert_eq!(
            run(vec![
                Action::PlanePointer(PlanePointer {
                    x: Some(2.0),
                    over: None,
                    panning: false,
                }),
                Action::PlanePointer(PlanePointer {
                    x: None,
                    over: None,
                    panning: false,
                }),
            ])
            .q_x,
            None
        );
    }

    #[test]
    fn nothing_changes_when_nothing_changes() {
        let mut s = run(vec![Action::PlanePointer(PlanePointer {
            x: Some(1.234),
            over: None,
            panning: false,
        })]);
        assert!(!apply(
            &mut s,
            Action::PlanePointer(PlanePointer {
                x: Some(1.229),
                over: None,
                panning: false,
            })
        ));
        assert!(!apply(
            &mut s,
            Action::Hover(Hover {
                part: Part::Plane,
                on: true,
            })
        ));
        assert!(!apply(
            &mut s,
            Action::Hover(Hover {
                part: Part::K,
                on: false,
            })
        ));
        assert!(!apply(
            &mut s,
            Action::SetParam(SetParam {
                param: Param::A,
                value: 1.0,
                jump: false,
            })
        ));
        assert!(!apply(
            &mut s,
            Action::SetFunction(SetFunction { func: Base::X2 })
        ));
        let mut view = View {
            zoom: 50.0,
            extent: Extent {
                min_x: -9.36,
                max_x: 9.36,
                min_y: -7.92,
                max_y: 7.92,
            },
        };
        assert!(apply(&mut s, Action::SetView(SetView { view })));
        assert!(!apply(&mut s, Action::SetView(SetView { view })));
        view.zoom = 40.0;
        assert!(apply(&mut s, Action::SetView(SetView { view })));
        assert_eq!(s.view.as_ref().unwrap().zoom, 40.0);
    }

    #[test]
    fn ghost_shows_while_transformed_and_on() {
        assert!(!is_transformed(&initial_state()));
        assert!(!ghost_visible(&initial_state()));
        let mut moved = run(vec![Action::SetParam(SetParam {
            param: Param::K,
            value: 1.0,
            jump: false,
        })]);
        assert!(ghost_visible(&moved));
        apply(&mut moved, Action::SetGhost(SetGhost { on: false }));
        assert!(!ghost_visible(&moved));
    }

    #[test]
    fn q_follows_the_pointer_on_the_plane() {
        let mut on = run(vec![
            Action::Hover(Hover {
                part: Part::K,
                on: true,
            }),
            Action::Hover(Hover {
                part: Part::K,
                on: false,
            }),
            Action::PlanePointer(PlanePointer {
                x: Some(-1.5),
                over: None,
                panning: false,
            }),
        ]);
        assert_eq!(on.q_x, Some(-1.5));
        assert_eq!(on.hover, Some(Part::Plane));
        assert_eq!(on.lead, Lead::Hover);
        apply(
            &mut on,
            Action::PlanePointer(PlanePointer {
                x: None,
                over: None,
                panning: false,
            }),
        );
        assert_eq!(on.q_x, None);
        assert_eq!(on.hover, None);
    }

    #[test]
    fn q_hides_over_p_while_dragged_and_panning() {
        let over_p = run(vec![Action::PlanePointer(PlanePointer {
            x: Some(1.99),
            over: Some(PlaneMark::P),
            panning: false,
        })]);
        assert_eq!(over_p.q_x, None);
        assert_eq!(over_p.plane_over, Some(PlaneMark::P));
        assert_eq!(over_p.hover, Some(Part::Plane));
        let panning = run(vec![Action::PlanePointer(PlanePointer {
            x: Some(1.0),
            over: None,
            panning: true,
        })]);
        assert_eq!(panning.q_x, None);
        assert_eq!(panning.plane_over, None);
        // Back on the empty plane, Q follows again; off the plane, nothing is over.
        let mut back = over_p.clone();
        apply(
            &mut back,
            Action::PlanePointer(PlanePointer {
                x: Some(1.0),
                over: None,
                panning: false,
            }),
        );
        assert_eq!(back.q_x, Some(1.0));
        assert_eq!(back.plane_over, None);
        let mut off = over_p;
        apply(
            &mut off,
            Action::PlanePointer(PlanePointer {
                x: None,
                over: None,
                panning: false,
            }),
        );
        assert_eq!(off.plane_over, None);
        assert_eq!(off.hover, None);
    }

    #[test]
    fn lights_p_from_its_row_its_drag_or_its_marker() {
        assert!(!p_lit(&initial_state()));
        assert!(p_lit(&run(vec![Action::Hover(Hover {
            part: Part::P,
            on: true,
        })])));
        assert!(p_lit(&run(vec![Action::Focus(Focus {
            part: Part::P,
            on: true,
        })])));
        assert!(p_lit(&run(vec![Action::SetDrag(SetDrag {
            part: Part::P,
            mode: Some(Mode::Coarse),
        })])));
        assert!(p_lit(&run(vec![Action::PlanePointer(PlanePointer {
            x: Some(2.0),
            over: Some(PlaneMark::P),
            panning: false,
        })])));
        assert!(!p_lit(&run(vec![Action::PlanePointer(PlanePointer {
            x: Some(2.0),
            over: None,
            panning: false,
        })])));
        assert!(!p_lit(&run(vec![Action::Hover(Hover {
            part: Part::K,
            on: true,
        })])));
    }

    #[test]
    fn a_leave_clears_only_its_own_part() {
        let mut s = run(vec![
            Action::Hover(Hover {
                part: Part::A,
                on: true,
            }),
            Action::Hover(Hover {
                part: Part::B,
                on: true,
            }),
        ]);
        assert!(!apply(
            &mut s,
            Action::Hover(Hover {
                part: Part::A,
                on: false,
            })
        ));
        assert_eq!(
            run(vec![
                Action::PlanePointer(PlanePointer {
                    x: Some(1.0),
                    over: None,
                    panning: false,
                }),
                Action::Hover(Hover {
                    part: Part::P,
                    on: true,
                }),
                Action::PlanePointer(PlanePointer {
                    x: None,
                    over: None,
                    panning: false,
                }),
            ])
            .hover,
            Some(Part::P)
        );
    }

    #[test]
    fn hover_and_focus_each_hold_one_part() {
        let mut focused = run(vec![
            Action::Hover(Hover {
                part: Part::Plane,
                on: true,
            }),
            Action::Focus(Focus {
                part: Part::K,
                on: true,
            }),
        ]);
        assert_eq!(focused.hover, Some(Part::Plane));
        assert_eq!(focused.focus, Some(Part::K));
        assert_eq!(focused.lead, Lead::Focus);
        apply(
            &mut focused,
            Action::PlanePointer(PlanePointer {
                x: Some(0.5),
                over: None,
                panning: false,
            }),
        );
        assert_eq!(focused.lead, Lead::Hover);
        apply(
            &mut focused,
            Action::Focus(Focus {
                part: Part::K,
                on: false,
            }),
        );
        assert_eq!(focused.focus, None);
    }

    #[test]
    fn holds_one_drag_and_one_edit() {
        let dragging = run(vec![
            Action::SetDrag(SetDrag {
                part: Part::H,
                mode: Some(Mode::Coarse),
            }),
            Action::SetDrag(SetDrag {
                part: Part::H,
                mode: Some(Mode::Fine),
            }),
        ]);
        assert_eq!(
            dragging.drag,
            Some(Drag {
                part: Part::H,
                mode: Mode::Fine,
                fine: EnumSet::only(Param::H),
            })
        );
        let mut s = dragging;
        assert!(!apply(
            &mut s,
            Action::SetDrag(SetDrag {
                part: Part::K,
                mode: None,
            })
        ));
        apply(
            &mut s,
            Action::SetDrag(SetDrag {
                part: Part::H,
                mode: None,
            }),
        );
        assert_eq!(s.drag, None);
        let mut refused = run(vec![Action::SetEdit(SetEdit {
            part: Part::B,
            edit: Some(FieldEdit {
                error: ZERO_SCALE.to_string(),
                base: 1.04,
            }),
        })]);
        assert_eq!(
            refused.edit,
            Some(Edit {
                part: Part::B,
                error: ZERO_SCALE.to_string(),
                base: 1.04,
            })
        );
        assert!(!apply(
            &mut refused,
            Action::SetEdit(SetEdit {
                part: Part::B,
                edit: Some(FieldEdit {
                    error: ZERO_SCALE.to_string(),
                    base: 1.04,
                }),
            })
        ));
        apply(
            &mut refused,
            Action::SetEdit(SetEdit {
                part: Part::B,
                edit: None,
            }),
        );
        assert_eq!(refused.edit, None);
    }

    #[test]
    fn each_part_has_its_own_hover_drag_and_edit() {
        let s = run(vec![
            Action::Hover(Hover {
                part: Part::A,
                on: true,
            }),
            Action::SetDrag(SetDrag {
                part: Part::P,
                mode: Some(Mode::Coarse),
            }),
            Action::SetEdit(SetEdit {
                part: Part::K,
                edit: Some(FieldEdit {
                    error: String::new(),
                    base: 0.0,
                }),
            }),
        ]);
        // The open field leads: only k is lit.
        assert_eq!(
            part_ui(&s, Part::A),
            PartUi {
                hovered: true,
                lit: false,
                mode: None,
                fine: false,
                edit: None,
            }
        );
        assert_eq!(
            part_ui(&s, Part::P),
            PartUi {
                hovered: false,
                lit: false,
                mode: Some(Mode::Coarse),
                fine: false,
                edit: None,
            }
        );
        assert_eq!(
            part_ui(&s, Part::K),
            PartUi {
                hovered: false,
                lit: true,
                mode: None,
                fine: false,
                edit: Some(FieldEdit {
                    error: String::new(),
                    base: 0.0,
                }),
            }
        );
    }

    #[test]
    fn a_drag_holds_3_decimals_once_it_goes_fine() {
        let mut s = run(vec![Action::SetDrag(SetDrag {
            part: Part::K,
            mode: Some(Mode::Coarse),
        })]);
        assert_eq!(fine_params(&s), EnumSet::new());
        apply(
            &mut s,
            Action::SetDrag(SetDrag {
                part: Part::K,
                mode: Some(Mode::Fine),
            }),
        );
        assert_eq!(fine_params(&s), EnumSet::only(Param::K));
        assert!(part_ui(&s, Part::K).fine);
        apply(
            &mut s,
            Action::SetDrag(SetDrag {
                part: Part::K,
                mode: Some(Mode::Coarse),
            }),
        );
        assert_eq!(fine_params(&s), EnumSet::only(Param::K));
        apply(
            &mut s,
            Action::SetDrag(SetDrag {
                part: Part::K,
                mode: None,
            }),
        );
        assert_eq!(fine_params(&s), EnumSet::new());
        assert_eq!(
            fine_params(&run(vec![Action::SetDrag(SetDrag {
                part: Part::H,
                mode: Some(Mode::Fine),
            })])),
            EnumSet::only(Param::H)
        );
    }

    #[test]
    fn a_drag_holds_the_third_decimal_its_value_starts_with() {
        fn held(state: &State, part: Part) -> EnumSet<Param> {
            let mut s = state.clone();
            apply(
                &mut s,
                Action::SetDrag(SetDrag {
                    part,
                    mode: Some(Mode::Coarse),
                }),
            );
            fine_params(&s)
        }
        let typed = run(vec![Action::SetParam(SetParam {
            param: Param::A,
            value: 1.035,
            jump: false,
        })]);
        assert_eq!(held(&typed, Part::A), EnumSet::only(Param::A));
        assert_eq!(held(&typed, Part::Stretch), EnumSet::only(Param::A));
        assert_eq!(held(&typed, Part::Anchor), EnumSet::new());
        assert_eq!(held(&typed, Part::P), EnumSet::new());
    }

    #[test]
    fn the_active_value_is_whichever_moved_last() {
        assert_eq!(active(&initial_state()), None);
        assert_eq!(
            active(&run(vec![Action::Hover(Hover {
                part: Part::A,
                on: true,
            })])),
            Some(Active::Param(Param::A))
        );
        let mut both = run(vec![
            Action::Hover(Hover {
                part: Part::A,
                on: true,
            }),
            Action::Focus(Focus {
                part: Part::K,
                on: true,
            }),
        ]);
        assert_eq!(active(&both), Some(Active::Param(Param::K)));
        assert_eq!(active_params(&both), EnumSet::only(Param::K));
        // The pointer moving onto the empty plane hands the highlight over: nothing is lit.
        apply(
            &mut both,
            Action::PlanePointer(PlanePointer {
                x: Some(1.0),
                over: None,
                panning: false,
            }),
        );
        assert_eq!(active(&both), None);
    }

    #[test]
    fn a_term_lights_while_the_pointer_is_on_it() {
        let over = run(vec![Action::EqPointer(EqPointer {
            over: Some(Param::H),
        })]);
        assert_eq!(over.hover, Some(Part::Eq));
        assert_eq!(active(&over), Some(Active::Param(Param::H)));
        let mut between = over.clone();
        apply(&mut between, Action::EqPointer(EqPointer { over: None }));
        assert_eq!(active(&between), None);
        let mut left = over;
        apply(
            &mut left,
            Action::Hover(Hover {
                part: Part::Eq,
                on: false,
            }),
        );
        assert_eq!(left.hover, None);
        assert_eq!(left.eq_over, None);
    }

    #[test]
    fn a_handle_lights_both_its_values() {
        let over = run(vec![Action::PlanePointer(PlanePointer {
            x: Some(0.0),
            over: Some(PlaneMark::Anchor),
            panning: false,
        })]);
        assert_eq!(active(&over), Some(Active::Handle(Handle::Anchor)));
        assert_eq!(active_params(&over), EnumSet::only(Param::H) | Param::K);
        assert_eq!(over.q_x, None);
        assert_eq!(handle_lit(&over), Some(Handle::Anchor));
        assert_eq!(handle_held(&over), None);
        let mut held = over;
        apply(
            &mut held,
            Action::SetDrag(SetDrag {
                part: Part::Stretch,
                mode: Some(Mode::Coarse),
            }),
        );
        assert_eq!(active_params(&held), EnumSet::only(Param::A) | Param::B);
        assert_eq!(handle_lit(&held), Some(Handle::Stretch));
        assert_eq!(handle_held(&held), Some(Handle::Stretch));
        // The boxes of a dragged handle's values show its drag.
        assert_eq!(part_ui(&held, Part::A).mode, Some(Mode::Coarse));
        assert_eq!(part_ui(&held, Part::H).mode, None);
        assert_eq!(dragged_params(&held), EnumSet::only(Param::A) | Param::B);
    }

    #[test]
    fn nothing_lights_while_a_field_holds_refused_text() {
        assert_eq!(
            active(&run(vec![
                Action::Hover(Hover {
                    part: Part::A,
                    on: true,
                }),
                Action::SetEdit(SetEdit {
                    part: Part::B,
                    edit: Some(FieldEdit {
                        error: ZERO_SCALE.to_string(),
                        base: 1.0,
                    }),
                }),
            ])),
            None
        );
    }

    #[test]
    fn the_anchor_moves_the_curve() {
        let s = run(with(
            &R3,
            vec![
                Action::SetDrag(SetDrag {
                    part: Part::Anchor,
                    mode: Some(Mode::Coarse),
                }),
                Action::DragHandle(DragHandle {
                    handle: Handle::Anchor,
                    to: [1.5, 1.0],
                }),
            ],
        ));
        assert_eq!(s.params, params(2.0, 1.0, 1.5, 1.0));
    }

    #[test]
    fn the_grip_stretches_and_flips_below_the_anchor() {
        let s = run(with(
            &R3,
            vec![Action::DragHandle(DragHandle {
                handle: Handle::Stretch,
                to: [0.5, -1.0],
            })],
        ));
        assert_eq!(s.params, params(-2.0, 1.5, -1.0, 1.0));
    }

    #[test]
    fn a_transform_drag_takes_p_where_it_found_it() {
        let start = run(R5_START.to_vec());
        assert_eq!(
            start.p_before,
            Some(PBefore {
                x: 0.5,
                y: 4.5,
                moved: false,
            })
        );
        assert_eq!(p_moved(&start), None);
        let mut moved = start;
        apply(
            &mut moved,
            Action::SetParam(SetParam {
                param: Param::K,
                value: 1.0,
                jump: false,
            }),
        );
        assert_eq!(p_moved(&moved), Some(1.0));
        // Back where it started, it still says so: "moved 0".
        apply(
            &mut moved,
            Action::SetParam(SetParam {
                param: Param::K,
                value: 0.0,
                jump: false,
            }),
        );
        assert_eq!(p_moved(&moved), Some(0.0));
    }

    #[test]
    fn the_ghost_stays_after_the_release_until_cleared() {
        let mut moved = run(with(
            &R5_START,
            vec![Action::SetParam(SetParam {
                param: Param::K,
                value: 1.0,
                jump: false,
            })],
        ));
        assert!(!apply(&mut moved, Action::ClearPBefore(ClearPBefore)));
        let mut released = moved;
        apply(
            &mut released,
            Action::SetDrag(SetDrag {
                part: Part::K,
                mode: None,
            }),
        );
        assert_eq!(p_moved(&released), Some(1.0));
        apply(&mut released, Action::ClearPBefore(ClearPBefore));
        assert_eq!(released.p_before, None);
    }

    #[test]
    fn the_ghost_starts_over_on_the_next_drag() {
        let released = run(with(
            &R5_START,
            vec![
                Action::SetParam(SetParam {
                    param: Param::K,
                    value: 1.0,
                    jump: false,
                }),
                Action::SetDrag(SetDrag {
                    part: Part::K,
                    mode: None,
                }),
            ],
        ));
        assert_eq!(run(R5_START.to_vec()).p_before.unwrap().y, 4.5);
        let mut again = released.clone();
        apply(
            &mut again,
            Action::SetDrag(SetDrag {
                part: Part::A,
                mode: Some(Mode::Fine),
            }),
        );
        assert_eq!(
            again.p_before,
            Some(PBefore {
                x: 0.5,
                y: 5.5,
                moved: false,
            })
        );
        let mut reset = released.clone();
        apply(&mut reset, Action::ResetAll(ResetAll));
        assert_eq!(reset.p_before, None);
        let mut p_moved_by_hand = released;
        apply(&mut p_moved_by_hand, Action::SetP(SetP { x: 1.0 }));
        assert_eq!(p_moved_by_hand.p_before, None);
        // A mode change mid-drag keeps it.
        let fine = run(with(
            &R5_START,
            vec![Action::SetDrag(SetDrag {
                part: Part::K,
                mode: Some(Mode::Fine),
            })],
        ));
        assert_eq!(
            fine.p_before,
            Some(PBefore {
                x: 0.5,
                y: 4.5,
                moved: false,
            })
        );
    }

    #[test]
    fn no_ghost_for_p_own_drag_or_a_hover() {
        assert_eq!(
            run(vec![Action::SetDrag(SetDrag {
                part: Part::P,
                mode: Some(Mode::Coarse),
            })])
            .p_before,
            None
        );
        assert_eq!(
            run(vec![Action::Hover(Hover {
                part: Part::K,
                on: true,
            })])
            .p_before,
            None
        );
    }

    #[test]
    fn p_off_the_view() {
        let view = SetView {
            view: View {
                zoom: 50.0,
                extent: Extent {
                    min_x: -9.36,
                    max_x: 9.36,
                    min_y: -7.92,
                    max_y: 7.92,
                },
            },
        };
        assert_eq!(p_off_view(&initial_state()), OffView::InView);
        assert_eq!(
            p_off_view(&run(vec![Action::SetView(view)])),
            OffView::InView
        );
        // x3 at P = 2 is 8, above R2's 7.92.
        assert_eq!(
            p_off_view(&run(vec![
                Action::SetView(view),
                Action::SetFunction(SetFunction { func: Base::X3 }),
            ])),
            OffView::Above
        );
        assert_eq!(
            p_off_view(&run(vec![
                Action::SetView(view),
                Action::SetParam(SetParam {
                    param: Param::K,
                    value: -12.0,
                    jump: false,
                }),
            ])),
            OffView::Below
        );
    }

    #[test]
    fn a_jump_for_typing_resets_and_flips() {
        assert_eq!(
            run(vec![Action::SetParam(SetParam {
                param: Param::A,
                value: 2.0,
                jump: true,
            })])
            .change,
            Change::Jump
        );
        assert_eq!(
            run(with(
                &R3,
                vec![Action::ResetParam(ResetParam { param: Param::K })],
            ))
            .change,
            Change::Jump
        );
        assert_eq!(
            run(with(&R3, vec![Action::ResetAll(ResetAll)]),).change,
            Change::Jump
        );
        assert_eq!(
            run(vec![Action::Flip(Flip { param: Param::A })]).change,
            Change::Jump
        );
    }

    #[test]
    fn direct_for_drags_and_keys_a_switch_for_a_new_function() {
        let mut s = run(vec![Action::SetParam(SetParam {
            param: Param::A,
            value: 2.0,
            jump: true,
        })]);
        for action in R3 {
            apply(&mut s, action);
        }
        assert_eq!(s.change, Change::Direct);
        assert_eq!(
            run(vec![Action::DragHandle(DragHandle {
                handle: Handle::Anchor,
                to: [1.0, 2.0],
            })])
            .change,
            Change::Direct
        );
        assert_eq!(
            run(vec![Action::SetFunction(SetFunction { func: Base::Sin })]).change,
            Change::Switch
        );
    }

    #[test]
    fn nothing_new_when_the_curve_stays() {
        let typed = run(vec![Action::SetParam(SetParam {
            param: Param::A,
            value: 2.0,
            jump: true,
        })]);
        let mut same = typed;
        assert!(!apply(
            &mut same,
            Action::SetParam(SetParam {
                param: Param::A,
                value: 2.0,
                jump: false,
            })
        ));
        apply(&mut same, Action::SetP(SetP { x: 1.0 }));
        assert_eq!(same.change, Change::Jump);
        assert_eq!(run(vec![Action::ResetAll(ResetAll)]).change, Change::Direct);
    }

    #[test]
    fn explainers_open_one_at_a_time() {
        // A close ends only what the same means opened.
        assert_eq!(
            run(vec![open(Param::A, ExplainBy::Hover)]).explainer,
            Some(Explainer {
                param: Param::A,
                by: ExplainBy::Hover,
            })
        );
        assert_eq!(
            run(vec![
                open(Param::A, ExplainBy::Hover),
                open(Param::K, ExplainBy::Key),
            ])
            .explainer,
            Some(Explainer {
                param: Param::K,
                by: ExplainBy::Key,
            })
        );
        // Leaving the chip doesn't close what ? opened.
        assert_eq!(
            run(vec![
                open(Param::A, ExplainBy::Key),
                Action::Explain(Explain {
                    param: Param::A,
                    by: ExplainBy::Hover,
                    open: ExplainOpen::Close,
                }),
            ])
            .explainer,
            Some(Explainer {
                param: Param::A,
                by: ExplainBy::Key,
            })
        );
        assert_eq!(
            run(vec![
                open(Param::A, ExplainBy::Hover),
                Action::Explain(Explain {
                    param: Param::A,
                    by: ExplainBy::Hover,
                    open: ExplainOpen::Close,
                }),
            ])
            .explainer,
            None
        );
        // A close for another parameter changes nothing.
        assert_eq!(
            run(vec![
                open(Param::A, ExplainBy::Hover),
                Action::Explain(Explain {
                    param: Param::K,
                    by: ExplainBy::Hover,
                    open: ExplainOpen::Close,
                }),
            ])
            .explainer,
            Some(Explainer {
                param: Param::A,
                by: ExplainBy::Hover,
            })
        );
        // The same again changes nothing at all.
        let mut s = run(vec![open(Param::A, ExplainBy::Hover)]);
        assert!(!apply(&mut s, open(Param::A, ExplainBy::Hover)));
    }

    #[test]
    fn explainers_toggle_from_a_tap_or_the_key() {
        fn toggle(by: ExplainBy) -> Action {
            Action::Explain(Explain {
                param: Param::B,
                by,
                open: ExplainOpen::Toggle,
            })
        }
        // Whatever opened it.
        assert_eq!(
            run(vec![toggle(ExplainBy::Tap)]).explainer,
            Some(Explainer {
                param: Param::B,
                by: ExplainBy::Tap,
            })
        );
        assert_eq!(
            run(vec![toggle(ExplainBy::Tap), toggle(ExplainBy::Tap)]).explainer,
            None
        );
        assert_eq!(
            run(vec![
                open(Param::B, ExplainBy::Hover),
                toggle(ExplainBy::Key)
            ])
            .explainer,
            None
        );
    }

    #[test]
    fn explainers_close_on_esc_and_the_focus_leaving() {
        assert_eq!(
            run(vec![open(Param::H, ExplainBy::Tap), Action::Escape(Escape),]).explainer,
            None
        );
        let focused = Focus {
            part: Part::H,
            on: true,
        };
        let blurred = Focus {
            part: Part::H,
            on: false,
        };
        // The focus leaving the control whose ? opened it.
        assert_eq!(
            run(vec![
                Action::Focus(focused),
                open(Param::H, ExplainBy::Key),
                Action::Focus(blurred),
            ])
            .explainer,
            None
        );
        assert_eq!(
            run(vec![
                Action::Focus(focused),
                open(Param::H, ExplainBy::Hover),
                Action::Focus(blurred),
            ])
            .explainer,
            Some(Explainer {
                param: Param::H,
                by: ExplainBy::Hover,
            })
        );
        // Esc with none open changes nothing.
        let mut s = initial_state();
        assert!(!apply(&mut s, Action::Escape(Escape)));
    }

    #[test]
    fn explainers_light_their_value() {
        assert_eq!(
            active(&run(vec![open(Param::A, ExplainBy::Tap)])),
            Some(Active::Param(Param::A))
        );
        assert_eq!(
            active(&run(vec![
                open(Param::A, ExplainBy::Tap),
                Action::Hover(Hover {
                    part: Part::K,
                    on: true,
                }),
            ])),
            Some(Active::Param(Param::A))
        );
        assert_eq!(
            active(&run(vec![
                open(Param::A, ExplainBy::Tap),
                Action::SetDrag(SetDrag {
                    part: Part::K,
                    mode: Some(Mode::Coarse),
                }),
            ])),
            Some(Active::Param(Param::K))
        );
        assert!(part_ui(&run(vec![open(Param::A, ExplainBy::Tap)]), Part::A).lit);
    }
}
