//! The hint: what the page says about the state.

use super::copy::{
    HINT_ANCHOR, HINT_EDIT, HINT_FINE, HINT_HANDLES, HINT_IDLE, HINT_PLANE, HINT_PLANE_KEYS,
    HINT_POINT, HINT_SNAP, HINT_STRETCH, Phrase, hint_refused, hint_transform,
};
use super::form::Param;
use super::state::{Lead, Part, PlaneMark, State, part_param};
use mv_numfmt::format_stored;

/// What the hint says: idle, transforms, drags, edits and errors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HintKind {
    Idle,
    Transform,
    Fine,
    Snap,
    Edit,
    Error,
    Plane,
    PlaneKeys,
    Point,
    Handles,
    Anchor,
    Stretch,
}

/// The hint's context: its kind and parameter, or its error.
#[derive(Clone, Debug, PartialEq)]
pub struct HintContext {
    pub kind: HintKind,
    pub param: Param,
    pub error: String,
    pub base: f64,
}

/// The idle context: nothing on a part.
fn idle_context() -> HintContext {
    HintContext {
        kind: HintKind::Idle,
        param: Param::A,
        error: String::new(),
        base: 0.0,
    }
}

/// A context without a payload.
fn plain_context(kind: HintKind) -> HintContext {
    HintContext {
        kind,
        param: Param::A,
        error: String::new(),
        base: 0.0,
    }
}

/// A part's hint context.
fn part_context(state: &State, part: Option<Part>) -> HintContext {
    let Some(p) = part else {
        return idle_context();
    };
    let (param, is_param) = part_param(p);
    if is_param {
        return HintContext {
            kind: HintKind::Transform,
            param,
            error: String::new(),
            base: 0.0,
        };
    }
    let kind = match p {
        Part::P => HintKind::Point,
        Part::Plane => match state.plane_over {
            None => HintKind::Plane,
            Some(PlaneMark::P) => HintKind::Point,
            _ => HintKind::Handles,
        },
        Part::Eq => match state.eq_over {
            Some(term) => {
                return HintContext {
                    kind: HintKind::Transform,
                    param: term,
                    error: String::new(),
                    base: 0.0,
                };
            }
            None => return idle_context(),
        },
        Part::Anchor => HintKind::Anchor,
        Part::Stretch => HintKind::Stretch,
        _ => return idle_context(),
    };
    plain_context(kind)
}

/// The state's hint context.
pub fn hint_context(state: &State) -> HintContext {
    if let Some(edit) = &state.edit {
        if edit.error.is_empty() {
            return plain_context(HintKind::Edit);
        }
        return HintContext {
            kind: HintKind::Error,
            param: Param::A,
            error: edit.error.clone(),
            base: edit.base,
        };
    }
    if let Some(drag) = &state.drag {
        match drag.mode {
            mv_scrub::Mode::Coarse => return part_context(state, Some(drag.part)),
            mv_scrub::Mode::Fine => return plain_context(HintKind::Fine),
            mv_scrub::Mode::Snap => return plain_context(HintKind::Snap),
        }
    }
    if let Some(e) = &state.explainer {
        return HintContext {
            kind: HintKind::Transform,
            param: e.param,
            error: String::new(),
            base: 0.0,
        };
    }
    let mut by_focus = state.lead == Lead::Focus;
    if by_focus && state.focus.is_none() {
        by_focus = false;
    } else if !by_focus && state.hover.is_none() {
        by_focus = state.focus.is_some();
    }
    if !by_focus && state.hover.is_none() {
        return idle_context();
    }
    if by_focus {
        if state.focus == Some(Part::Plane) {
            return plain_context(HintKind::PlaneKeys);
        }
        return part_context(state, state.focus);
    }
    part_context(state, state.hover)
}

/// A hint: its phrases, and whether it errors.
#[derive(Clone, Debug, PartialEq)]
pub struct Hint {
    pub phrases: Vec<Phrase>,
    pub error: bool,
}

/// The hint for a context.
pub fn hint_for(c: &HintContext) -> Hint {
    match c.kind {
        HintKind::Idle => Hint {
            phrases: HINT_IDLE.to_vec(),
            error: false,
        },
        HintKind::Transform => Hint {
            phrases: hint_transform(c.param),
            error: false,
        },
        HintKind::Fine => Hint {
            phrases: HINT_FINE.to_vec(),
            error: false,
        },
        HintKind::Snap => Hint {
            phrases: HINT_SNAP.to_vec(),
            error: false,
        },
        HintKind::Edit => Hint {
            phrases: HINT_EDIT.to_vec(),
            error: false,
        },
        HintKind::Error => Hint {
            phrases: hint_refused(&c.error, &format_stored(c.base, false)),
            error: true,
        },
        HintKind::Plane => Hint {
            phrases: HINT_PLANE.to_vec(),
            error: false,
        },
        HintKind::PlaneKeys => Hint {
            phrases: HINT_PLANE_KEYS.to_vec(),
            error: false,
        },
        HintKind::Point => Hint {
            phrases: HINT_POINT.to_vec(),
            error: false,
        },
        HintKind::Handles => Hint {
            phrases: HINT_HANDLES.to_vec(),
            error: false,
        },
        HintKind::Anchor => Hint {
            phrases: HINT_ANCHOR.to_vec(),
            error: false,
        },
        HintKind::Stretch => Hint {
            phrases: HINT_STRETCH.to_vec(),
            error: false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apply::{
        Action, EqPointer, Explain, ExplainOpen, Focus, Hover, PlanePointer, SetDrag, SetEdit,
        apply,
    };
    use crate::copy::{mode_badge, phrase_words, tip_drag, tip_keys};
    use crate::state::{ExplainBy, FieldEdit, initial_state};
    use mv_scrub::Mode;

    fn read(actions: &[Action]) -> String {
        let mut state = initial_state();
        for action in actions {
            apply(&mut state, action.clone());
        }
        phrase_words(&hint_for(&hint_context(&state)).phrases).join(" ")
    }

    fn context_of(actions: &[Action]) -> HintContext {
        let mut state = initial_state();
        for action in actions {
            apply(&mut state, action.clone());
        }
        hint_context(&state)
    }

    const IDLE: &str =
        "Drag a ruler sideways to reshape the curve · point at the plane to read f(x)";
    const PLANE: &str = "Q follows your pointer · drag to pan · scroll to zoom";
    const KEYS: &str = "← ↑ ↓ → pan · + − zoom · [ ] move P · 0 resets the view";
    const POINT: &str = "Drag P along the curve · ← → nudge 0.1";
    const K_TRANSFORM: &str =
        "Drag to change k · Shift fine · Ctrl whole steps · double-click: back to 0";

    fn plane_at(x: f64) -> Action {
        Action::PlanePointer(PlanePointer {
            x: Some(x),
            over: None,
            panning: false,
        })
    }

    fn plane_off() -> Action {
        Action::PlanePointer(PlanePointer {
            x: None,
            over: None,
            panning: false,
        })
    }

    #[test]
    fn idle_with_nothing_on_a_part() {
        assert_eq!(read(&[]), IDLE);
    }

    #[test]
    fn the_plane_while_the_pointer_is_on_it() {
        assert_eq!(read(&[plane_at(-1.5)]), PLANE);
    }

    #[test]
    fn the_plane_keys_under_its_keyboard_focus() {
        let focused = Action::Focus(Focus {
            part: Part::Plane,
            on: true,
        });
        assert_eq!(read(std::slice::from_ref(&focused)), KEYS);
        assert_eq!(read(&[focused.clone(), plane_at(-1.5)]), PLANE);
        let hover_k = Action::Hover(Hover {
            part: Part::K,
            on: true,
        });
        assert_eq!(read(&[hover_k, focused.clone()]), KEYS);
        let unfocused = Action::Focus(Focus {
            part: Part::Plane,
            on: false,
        });
        assert_eq!(read(&[focused, unfocused]), IDLE);
    }

    #[test]
    fn p_while_the_pointer_is_on_its_marker_or_drags_it() {
        let on_mark = Action::PlanePointer(PlanePointer {
            x: Some(2.0),
            over: Some(PlaneMark::P),
            panning: false,
        });
        assert_eq!(read(std::slice::from_ref(&on_mark)), POINT);
        assert_eq!(
            read(&[
                on_mark,
                Action::SetDrag(SetDrag {
                    part: Part::P,
                    mode: Some(Mode::Coarse),
                }),
            ]),
            POINT
        );
    }

    #[test]
    fn a_shift_and_its_reset_value() {
        let hover_k = Action::Hover(Hover {
            part: Part::K,
            on: true,
        });
        assert_eq!(read(&[hover_k]), K_TRANSFORM);
        let drag_k = Action::SetDrag(SetDrag {
            part: Part::K,
            mode: Some(Mode::Coarse),
        });
        assert_eq!(read(&[drag_k]), K_TRANSFORM);
    }

    #[test]
    fn a_scale_its_reset_value_and_quarter_steps() {
        assert_eq!(
            read(&[Action::Focus(Focus {
                part: Part::A,
                on: true,
            })]),
            "Drag to change a · Shift fine · Ctrl quarter steps · double-click: back to 1"
        );
    }

    #[test]
    fn a_held_modifier_mid_drag() {
        assert_eq!(
            read(&[Action::SetDrag(SetDrag {
                part: Part::H,
                mode: Some(Mode::Fine),
            })]),
            "Shift Fine: a tenth of the speed, steps of 0.001"
        );
        assert_eq!(
            read(&[Action::SetDrag(SetDrag {
                part: Part::B,
                mode: Some(Mode::Snap),
            })]),
            "Ctrl Snapping to whole numbers (scales: quarters)"
        );
    }

    #[test]
    fn typing_keys_and_refusals() {
        assert_eq!(
            read(&[Action::SetEdit(SetEdit {
                part: Part::K,
                edit: Some(FieldEdit {
                    error: String::new(),
                    base: 0.0,
                }),
            })]),
            "Enter apply · Esc cancel · ↑ ↓ nudge 0.01"
        );
        let invalid = context_of(&[Action::SetEdit(SetEdit {
            part: Part::B,
            edit: Some(FieldEdit {
                error: "invalid".to_string(),
                base: 1.04,
            }),
        })]);
        assert_eq!(
            read(&[Action::SetEdit(SetEdit {
                part: Part::B,
                edit: Some(FieldEdit {
                    error: "invalid".to_string(),
                    base: 1.04,
                }),
            })]),
            "Type a number such as 1.5 · Esc puts back 1.04"
        );
        assert!(hint_for(&invalid).error, "an error's tone");
        assert_eq!(
            read(&[Action::SetEdit(SetEdit {
                part: Part::B,
                edit: Some(FieldEdit {
                    error: crate::form::ZERO_SCALE.to_string(),
                    base: 1.04,
                }),
            })]),
            "A scale of 0 squashes the curve flat. Try 0.1 · Esc puts back 1.04"
        );
    }

    #[test]
    fn p_while_its_row_is_hovered_focused_or_dragged() {
        assert_eq!(
            read(&[Action::Hover(Hover {
                part: Part::P,
                on: true,
            })]),
            POINT
        );
        assert_eq!(
            read(&[Action::Focus(Focus {
                part: Part::P,
                on: true,
            })]),
            POINT
        );
        assert_eq!(
            read(&[Action::SetDrag(SetDrag {
                part: Part::P,
                mode: Some(Mode::Coarse),
            })]),
            POINT
        );
    }

    #[test]
    fn an_open_field_first_then_a_drag() {
        let busy = [
            plane_at(1.0),
            Action::SetDrag(SetDrag {
                part: Part::H,
                mode: Some(Mode::Snap),
            }),
            Action::SetEdit(SetEdit {
                part: Part::H,
                edit: Some(FieldEdit {
                    error: String::new(),
                    base: 0.0,
                }),
            }),
        ];
        assert_eq!(context_of(&busy).kind, HintKind::Edit);
        assert_eq!(context_of(&busy[..2]).kind, HintKind::Snap);
    }

    #[test]
    fn whichever_moved_last_else_the_other() {
        let focus_k = Action::Focus(Focus {
            part: Part::K,
            on: true,
        });
        let tabbed = [plane_at(1.0), focus_k.clone()];
        assert_eq!(
            context_of(&tabbed),
            HintContext {
                kind: HintKind::Transform,
                param: Param::K,
                error: String::new(),
                base: 0.0,
            }
        );
        let moved = [plane_at(1.0), focus_k.clone(), plane_at(1.5)];
        assert_eq!(
            context_of(&moved),
            HintContext {
                kind: HintKind::Plane,
                param: Param::A,
                error: String::new(),
                base: 0.0,
            }
        );
        let off = [plane_at(1.0), focus_k, plane_at(1.5), plane_off()];
        assert_eq!(
            context_of(&off),
            HintContext {
                kind: HintKind::Transform,
                param: Param::K,
                error: String::new(),
                base: 0.0,
            }
        );
    }

    #[test]
    fn a_term_speaks_as_its_control() {
        assert_eq!(
            read(&[Action::EqPointer(EqPointer {
                over: Some(Param::K),
            })]),
            K_TRANSFORM
        );
        assert_eq!(read(&[Action::EqPointer(EqPointer { over: None })]), IDLE);
    }

    #[test]
    fn the_handles_and_what_a_dragged_one_sets() {
        assert_eq!(
            read(&[Action::PlanePointer(PlanePointer {
                x: Some(0.0),
                over: Some(PlaneMark::Stretch),
                panning: false,
            })]),
            "Drag the diamond to move the curve · the square to stretch it"
        );
        assert_eq!(
            read(&[Action::SetDrag(SetDrag {
                part: Part::Anchor,
                mode: Some(Mode::Coarse),
            })]),
            "Moving the anchor sets h and k · Shift locks one axis"
        );
        assert_eq!(
            read(&[Action::SetDrag(SetDrag {
                part: Part::Stretch,
                mode: Some(Mode::Coarse),
            })]),
            "Stretching sets a (height) and b (width) · Shift locks one of them"
        );
    }

    #[test]
    fn the_tips_name_each_default() {
        fn words(phrases: &[Phrase]) -> String {
            phrase_words(phrases).join(" ")
        }
        assert_eq!(
            words(&tip_drag(1.0)),
            "Drag sideways · Shift for fine · double-click: back to 1"
        );
        assert_eq!(
            words(&tip_keys(0.0)),
            "← → nudge 0.01, Shift 0.1 · Enter type · Backspace back to 0"
        );
        assert_eq!(mode_badge(Mode::Fine, false), "Fine ×0.1");
        assert_eq!(mode_badge(Mode::Snap, true), "Snap ¼");
        assert_eq!(mode_badge(Mode::Snap, false), "Snap 1");
    }

    #[test]
    fn speaks_about_an_open_explainers_control() {
        assert_eq!(
            read(&[
                Action::Explain(Explain {
                    param: Param::A,
                    by: ExplainBy::Tap,
                    open: ExplainOpen::Open,
                }),
                plane_at(1.0),
            ]),
            "Drag to change a · Shift fine · Ctrl quarter steps · double-click: back to 1"
        );
    }
}
