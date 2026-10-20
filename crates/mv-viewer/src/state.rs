//! The view state: everything the page shows, and its curve.

use std::sync::LazyLock;

use enum_map::{Enum, EnumMap};
use enumset::EnumSet;
use mv_numfmt::{DISPLAY_DP, format_number};

use super::form::{BASE_FUNCTIONS, Base, DEFAULT_PARAMS, Param, Params, evaluate};
use mv_plane::View;

/// The state the page shows.
#[derive(Clone, Debug, PartialEq)]
pub struct State {
    pub func: Base,
    pub params: Params,
    pub p_x: f64,
    pub q_x: Option<f64>,
    pub ghost_on: bool,
    pub view: Option<View>,
    pub hover: Option<Part>,
    pub plane_over: Option<PlaneMark>,
    pub eq_over: Option<Param>,
    pub focus: Option<Part>,
    pub lead: Lead,
    pub drag: Option<Drag>,
    pub edit: Option<Edit>,
    pub p_before: Option<PBefore>,
    pub explainer: Option<Explainer>,
    pub change: Change,
}

/// The function the page opens with.
pub const DEFAULT_FUNCTION: Base = Base::X2;

/// The P the page opens with.
pub const DEFAULT_P_X: f64 = 2.0;

/// The ghost curve shows at first.
pub const DEFAULT_GHOST_ON: bool = true;

/// The initial state.
pub fn initial_state() -> State {
    State {
        func: DEFAULT_FUNCTION,
        params: DEFAULT_PARAMS,
        p_x: DEFAULT_P_X,
        q_x: None,
        ghost_on: DEFAULT_GHOST_ON,
        view: None,
        hover: None,
        plane_over: None,
        eq_over: None,
        focus: None,
        lead: Lead::Hover,
        drag: None,
        edit: None,
        p_before: None,
        explainer: None,
        change: Change::Direct,
    }
}

/// What the pointer or keyboard acts on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum Part {
    A,
    K,
    B,
    H,
    P,
    Plane,
    Eq,
    Anchor,
    Stretch,
}

/// A part's parameter, if it has one.
pub fn part_param(part: Part) -> (Param, bool) {
    match part {
        Part::A => (Param::A, true),
        Part::K => (Param::K, true),
        Part::B => (Param::B, true),
        Part::H => (Param::H, true),
        _ => (Param::A, false),
    }
}

/// A curve handle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum Handle {
    Anchor,
    Stretch,
}

/// Each handle's parameters.
pub static HANDLE_PARAMS: LazyLock<EnumMap<Handle, EnumSet<Param>>> = LazyLock::new(|| {
    EnumMap::from_array([
        EnumSet::only(Param::H) | Param::K,
        EnumSet::only(Param::A) | Param::B,
    ])
});

/// Each handle's part.
pub static HANDLE_PARTS: EnumMap<Handle, Part> = EnumMap::from_array([Part::Anchor, Part::Stretch]);

/// A plane mark the pointer hovers: P or a handle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum PlaneMark {
    P,
    Anchor,
    Stretch,
}

/// Whose hover or focus shows: pointer or keyboard.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum Lead {
    Hover,
    Focus,
}

/// A drag in flight: its part, mode and fine parameters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Drag {
    pub part: Part,
    pub mode: mv_scrub::Mode,
    pub fine: EnumSet<Param>,
}

/// What changed last: a direct set, a jump, or a function switch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum Change {
    Direct,
    Jump,
    Switch,
}

/// An edit in flight: its part, error and base.
#[derive(Clone, Debug, PartialEq)]
pub struct Edit {
    pub part: Part,
    pub error: String,
    pub base: f64,
}

/// A field's edit: its error and base.
#[derive(Clone, Debug, PartialEq)]
pub struct FieldEdit {
    pub error: String,
    pub base: f64,
}

/// How an explainer opens: hover, tap or key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum ExplainBy {
    Hover,
    Tap,
    Key,
}

/// An open explainer: its parameter and how it opened.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Explainer {
    pub param: Param,
    pub by: ExplainBy,
}

/// Where P was before a drag: its point and whether it moved.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PBefore {
    pub x: f64,
    pub y: f64,
    pub moved: bool,
}

/// The transform's value at x.
pub fn curve_at(state: &State, x: f64) -> f64 {
    evaluate(BASE_FUNCTIONS[state.func].g, state.params, x)
}

/// Where P falls against the view: inside, above or below.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum OffView {
    InView,
    Above,
    Below,
}

/// Where P falls against the view.
pub fn p_off_view(state: &State) -> OffView {
    let Some(view) = state.view else {
        return OffView::InView;
    };
    let y = curve_at(state, state.p_x);
    if y > view.extent.max_y {
        OffView::Above
    } else if y < view.extent.min_y {
        OffView::Below
    } else {
        OffView::InView
    }
}

/// The readout slots a view fits: how many x and y labels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReadoutSlots {
    pub x: i64,
    pub y: i64,
}

/// The readout slots a view fits, if there is a view.
pub fn readout_slots(view: Option<View>) -> Option<ReadoutSlots> {
    fn widest(lo: f64, hi: f64) -> i64 {
        (format_number(lo, DISPLAY_DP)
            .chars()
            .count()
            .max(format_number(hi, DISPLAY_DP).chars().count())) as i64
    }
    let v = view?;
    let e = v.extent;
    Some(ReadoutSlots {
        x: widest(e.min_x, e.max_x),
        y: widest(e.min_y, e.max_y),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use mv_plane::{Extent, View};

    #[test]
    fn readout_slots_keep_the_widest_value_in_view() {
        fn slots(min_x: f64, max_x: f64, min_y: f64, max_y: f64) -> Option<ReadoutSlots> {
            readout_slots(Some(View {
                zoom: 50.0,
                extent: Extent {
                    min_x,
                    max_x,
                    min_y,
                    max_y,
                },
            }))
        }
        assert_eq!(
            slots(-9.36, 9.36, -7.92, 7.92),
            Some(ReadoutSlots { x: 5, y: 5 })
        );
        assert_eq!(
            slots(-15.7, 15.7, -5.1, 5.1),
            Some(ReadoutSlots { x: 6, y: 5 })
        );
        assert_eq!(
            slots(0.5, 19.2, -120.5, -2.0),
            Some(ReadoutSlots { x: 5, y: 7 })
        );
        assert_eq!(readout_slots(None), None);
    }

    #[test]
    fn p_lies_in_or_off_the_view() {
        let mut state = initial_state();
        assert_eq!(p_off_view(&state), OffView::InView);
        state.view = Some(View {
            zoom: 50.0,
            extent: Extent {
                min_x: -9.36,
                max_x: 9.36,
                min_y: -7.92,
                max_y: 7.92,
            },
        });
        assert_eq!(curve_at(&state, state.p_x), 4.0);
        assert_eq!(p_off_view(&state), OffView::InView);
        state.p_x = 3.0;
        assert_eq!(p_off_view(&state), OffView::Above);
        state.params[Param::A] = -1.0;
        assert_eq!(p_off_view(&state), OffView::Below);
    }
}
