//! The Transform card's reset button, the flip toggles and Reset all.
//!
//! Ported from `SideBar/TransformSection/useReset.ts`, `ResetIcon.tsx`,
//! `FlipIcon.tsx`, and the reset/flip/Reset-all parts of `TransformRow.tsx`,
//! `TransformSection.tsx` and their CSS modules at `b8a0e1d`: each row shows
//! its reset button while its value is off its default, in a 24 px slot the
//! row keeps whether or not the button shows (so the flip toggle holds
//! still); the two scale rows carry a flip toggle showing its pressed state
//! while the value is negative; the card's head shows Reset all while any
//! value is off its default. The readout's reserved 78 px row (`2e6069e`) and
//! the shortened labels (`999e0a0`) live in [`crate::readout`] and
//! [`crate::transform_section`]; the drag that hides a reset
//! (`.row:has([data-mode])`) reads h5-box's drag state at the wiring.
//!
//! This module is the pure core: the defaults, the changed rule, the labels,
//! the visibility rules, and the button and icon geometry. Layout and paint
//! wire into the rows in phase 2 (h4-rows's `transform_row.rs`/
//! `transform_section.rs` as merged); the icon paths move to [`crate::icons`]
//! as `Reset`/`Flip` kinds then.

use enum_map::EnumMap;

use crate::functions::{A_PARAM, B_PARAM, H_PARAM, K_PARAM};
use crate::transform_row::TransformParam;
use crate::url::QueryParamDef;

/// The parameter's page-param def (`transformParams[param]`): the record as a
/// match. `useReset` reads its `defaultValue` off this def.
#[must_use]
pub fn def_of(param: TransformParam) -> &'static QueryParamDef<f64> {
    match param {
        TransformParam::A => &A_PARAM,
        TransformParam::K => &K_PARAM,
        TransformParam::B => &B_PARAM,
        TransformParam::H => &H_PARAM,
    }
}

/// Whether the row shows its reset button's condition (`changed` in
/// `useReset`): the value is not the default. Exact float equality, as the
/// TS `!==` is.
#[must_use]
pub fn is_changed(param: TransformParam, value: f64) -> bool {
    value != def_of(param).default
}

/// The value the reset puts back (`reset`: `setValue(defaultValue, flush)`).
#[must_use]
pub fn reset_value(param: TransformParam) -> f64 {
    def_of(param).default
}

/// How a default prints in the reset's name (`Reset y scale to 1`): JS
/// `String(1)` is `"1"`, `String(0)` is `"0"` — no decimals, ASCII.
fn default_text(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1e21 {
        format!("{value:.0}")
    } else {
        format!("{value}")
    }
}

/// The reset button's accessible name (`Reset ${name} to ${defaultValue}`),
/// with the shortened row name (`999e0a0`).
#[must_use]
pub fn reset_label(param: TransformParam) -> String {
    format!(
        "Reset {} to {}",
        crate::transform_section::row_name(param),
        default_text(reset_value(param))
    )
}

/// Whether the row carries a flip toggle: the two scale rows only (`a`, `b`).
#[must_use]
pub fn is_scale(param: TransformParam) -> bool {
    matches!(param, TransformParam::A | TransformParam::B)
}

/// Which way the toggle flips (`FlipIcon`'s `axis` prop): a turns upside
/// down, b mirrors left to right.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlipAxis {
    Vertical,
    Horizontal,
}

/// The toggle's axis by row.
#[must_use]
pub fn flip_axis(param: TransformParam) -> FlipAxis {
    if param == TransformParam::A {
        FlipAxis::Vertical
    } else {
        FlipAxis::Horizontal
    }
}

/// The toggle's accessible name.
#[must_use]
pub fn flip_label(param: TransformParam) -> &'static str {
    if param == TransformParam::A {
        "Flip upside down"
    } else {
        "Mirror left to right"
    }
}

/// Whether the toggle shows pressed (`aria-pressed`): the value is negative.
#[must_use]
pub fn flip_pressed(value: f64) -> bool {
    value < 0.0
}

/// The value after the toggle (`setValue((v) => -v, flush)`): the sign flips.
#[must_use]
pub fn flipped(value: f64) -> f64 {
    -value
}

/// Whether the reset button paints: shown while the value is off its default
/// and hidden while the row's box is dragged (`.row:has([data-mode])` —
/// `dragged` is whether h5-box's `ScrubDrag` names this parameter). The slot
/// keeps its 24 px either way (`8261442`).
#[must_use]
pub fn reset_shown(changed: bool, dragged: bool) -> bool {
    changed && !dragged
}

/// Whether the head shows Reset all: any of the four values off its default.
#[must_use]
pub fn any_changed(values: &EnumMap<TransformParam, f64>) -> bool {
    values
        .iter()
        .any(|(param, value)| is_changed(param, *value))
}

/// The reset button's size, the slot's kept width and the flip toggle's size
/// live with the row's layout ([`crate::transform_row::RESET_SIZE`],
/// [`crate::transform_row::RESET_SLOT_WIDTH`],
/// [`crate::transform_row::FLIP_SIZE`]).
/// Reset all in the card's head: flex, gap 5, 20 px tall, margin-top 4,
/// padding 2×4, 13 px/15 px strong, `--radius-card` corners.
pub const RESET_ALL_HEIGHT: f32 = 20.0;
pub const RESET_ALL_FONT_SIZE: f32 = 13.0;
pub const RESET_ALL_LINE_HEIGHT: f32 = 15.0;
pub const RESET_ALL_GAP: f32 = 5.0;
pub const RESET_ALL_PAD_X: f32 = 4.0;
pub const RESET_ALL_PAD_Y: f32 = 2.0;
pub const RESET_ALL_MARGIN_TOP: f32 = 4.0;

/// The reset icon's viewBox (`ResetIcon.tsx`, 16). The arc's stroke is 1.75
/// round; the head fills.
pub const RESET_VIEWBOX: f32 = 16.0;
pub const RESET_ARC: &str = "M12.213 4.465A5.5 5.5 0 1 1 4.465 3.787";
pub const RESET_ARC_STROKE: f32 = 1.75;
pub const RESET_HEAD: &str = "M9.013 0.865L14.813 2.065L11.813 6.865Z";

/// The flip icon's viewBox (`FlipIcon.tsx`, 14 in the 24 px toggle). Each
/// axis pairs a 1.6 round shaft stroke with two filled heads.
pub const FLIP_VIEWBOX: f32 = 14.0;
pub const FLIP_ICON_SIZE: f32 = 14.0;
pub const FLIP_SHAFT_STROKE: f32 = 1.6;
pub const FLIP_SHAFT_VERTICAL: &str = "M7 2V12";
pub const FLIP_HEAD_VERTICAL: &str = "M3.5 5L7 0.5L10.5 5ZM3.5 9L7 13.5L10.5 9Z";
pub const FLIP_SHAFT_HORIZONTAL: &str = "M2 7H12";
pub const FLIP_HEAD_HORIZONTAL: &str = "M5 3.5L0.5 7L5 10.5ZM9 3.5L13.5 7L9 10.5Z";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transform_row::{FLIP_SIZE, RESET_SIZE, RESET_SLOT_WIDTH};
    use crate::url::{QueryParam, Url};

    #[test]
    fn reset_shows_while_off_default_and_puts_the_default_back() {
        // The row test: a link with a=2 shows "2.00" with a reset named
        // "Reset y scale to 1"; clicking it shows "1.00", drops a from the
        // URL, and takes the button away (the slot stays: reset_shown).
        let mut url = Url::from_query("a=2");
        let mut param = QueryParam::new(&A_PARAM, "a=2");
        assert!(is_changed(TransformParam::A, *param.get()));
        assert_eq!(reset_label(TransformParam::A), "Reset y scale to 1");
        assert!(reset_shown(true, false));

        param.set_flushed(&mut url, reset_value(TransformParam::A));
        assert_eq!(*param.get(), 1.0);
        assert_eq!(url.query(), "");
        assert!(!is_changed(TransformParam::A, *param.get()));

        // A shift resets to 0: "Reset x shift to 0".
        assert_eq!(reset_value(TransformParam::H), 0.0);
        assert_eq!(reset_label(TransformParam::H), "Reset x shift to 0");
    }

    #[test]
    fn reset_hides_while_a_drag_holds_the_value() {
        // The row test: with a=2 the reset paints; from the drag's first
        // move to its release it paints nothing; after release it is back.
        // `dragged` is h5-box's per-param drag marker (`data-mode`).
        assert!(reset_shown(true, false));
        assert!(!reset_shown(true, true));
        assert!(reset_shown(true, false));
        // At the default there is no button to hide.
        assert!(!reset_shown(false, false));
        assert!(!reset_shown(false, true));
    }

    #[test]
    fn flip_flips_the_sign_and_presses_while_negative() {
        // The row test: at a=2 the toggle reads pressed false; clicking it
        // shows "-2.00", reads pressed true and writes a=-2; clicking again
        // restores 2.00, false and a=2.
        let mut url = Url::from_query("a=2");
        let mut param = QueryParam::new(&A_PARAM, "a=2");
        assert!(!flip_pressed(*param.get()));

        param.set_flushed(&mut url, flipped(*param.get()));
        assert_eq!(*param.get(), -2.0);
        assert!(flip_pressed(*param.get()));
        assert_eq!(url.query(), "a=-2");

        param.set_flushed(&mut url, flipped(*param.get()));
        assert_eq!(*param.get(), 2.0);
        assert!(!flip_pressed(*param.get()));
        assert_eq!(url.query(), "a=2");
    }

    #[test]
    fn flip_toggles_stand_on_the_scale_rows_only() {
        // The section test: exactly the a and b rows carry a toggle, with
        // their own axis and name; k and h carry none.
        assert!(is_scale(TransformParam::A));
        assert!(is_scale(TransformParam::B));
        assert!(!is_scale(TransformParam::K));
        assert!(!is_scale(TransformParam::H));
        assert_eq!(flip_axis(TransformParam::A), FlipAxis::Vertical);
        assert_eq!(flip_axis(TransformParam::B), FlipAxis::Horizontal);
        assert_eq!(flip_label(TransformParam::A), "Flip upside down");
        assert_eq!(flip_label(TransformParam::B), "Mirror left to right");
    }

    #[test]
    fn reset_all_shows_while_any_value_is_off_and_restores_all() {
        // The section test: a link with a=2&h=3 shows Reset all; clicking it
        // puts every default back, drops both keys, and hides the button.
        let mut url = Url::from_query("a=2&h=3");
        let mut params = EnumMap::from_fn(|param| *QueryParam::new(def_of(param), "a=2&h=3").get());
        assert!(any_changed(&params));
        assert!(is_changed(TransformParam::A, params[TransformParam::A]));
        assert!(!is_changed(TransformParam::K, params[TransformParam::K]));
        assert!(!is_changed(TransformParam::B, params[TransformParam::B]));
        assert!(is_changed(TransformParam::H, params[TransformParam::H]));

        for param in [
            TransformParam::A,
            TransformParam::K,
            TransformParam::B,
            TransformParam::H,
        ] {
            let mut bound = QueryParam::new(def_of(param), url.query());
            bound.set_flushed(&mut url, reset_value(param));
            params[param] = *bound.get();
        }
        assert_eq!(
            (params[TransformParam::A], params[TransformParam::H]),
            (1.0, 0.0)
        );
        assert_eq!(url.query(), "");
        assert!(!any_changed(&params));
    }

    #[test]
    fn reset_all_and_icon_geometry_match_the_css() {
        assert_eq!(
            (
                RESET_ALL_HEIGHT,
                RESET_ALL_FONT_SIZE,
                RESET_ALL_LINE_HEIGHT,
                RESET_ALL_GAP,
                RESET_ALL_PAD_X,
                RESET_ALL_PAD_Y,
                RESET_ALL_MARGIN_TOP
            ),
            (20.0, 13.0, 15.0, 5.0, 4.0, 2.0, 4.0)
        );
        assert_eq!((RESET_VIEWBOX, RESET_ARC_STROKE), (16.0, 1.75));
        assert_eq!(RESET_ARC, "M12.213 4.465A5.5 5.5 0 1 1 4.465 3.787");
        assert_eq!(RESET_HEAD, "M9.013 0.865L14.813 2.065L11.813 6.865Z");
        assert_eq!(
            (FLIP_VIEWBOX, FLIP_ICON_SIZE, FLIP_SHAFT_STROKE),
            (14.0, 14.0, 1.6)
        );
        assert_eq!(
            (FLIP_SHAFT_VERTICAL, FLIP_HEAD_VERTICAL),
            ("M7 2V12", "M3.5 5L7 0.5L10.5 5ZM3.5 9L7 13.5L10.5 9Z")
        );
        assert_eq!(
            (FLIP_SHAFT_HORIZONTAL, FLIP_HEAD_HORIZONTAL),
            ("M2 7H12", "M5 3.5L0.5 7L5 10.5ZM9 3.5L13.5 7L9 10.5Z")
        );
    }

    #[test]
    fn buttons_keep_the_rows_layout() {
        // The keeps-place test's arithmetic: the flip stands at
        // pad + chip + gap + name, where the name takes what the flip, the
        // slot, the box and the fixed chrome leave — so showing or hiding
        // the reset (or the drag hiding it) never moves the flip.
        // Row: 8 + 26 + 12 + name + 24 + 12 + 24 + 12 + 107 + 6 = 350.
        let name = 350.0
            - (crate::transform_row::ROW_PAD_LEFT
                + crate::transform_row::CHIP_SIZE
                + crate::transform_row::ROW_GAP
                + FLIP_SIZE
                + crate::transform_row::ROW_GAP
                + RESET_SLOT_WIDTH
                + crate::transform_row::ROW_GAP
                + crate::transform_row::BOX_WIDTH
                + crate::transform_row::ROW_PAD_RIGHT);
        assert_eq!(name, 119.0);
        assert_eq!(RESET_SIZE, FLIP_SIZE);
        assert_eq!(RESET_SLOT_WIDTH, RESET_SIZE);
    }
}
