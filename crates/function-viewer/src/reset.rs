//! The Transform card's reset button, the flip toggles and Reset all.
//!
//! Ported from `SideBar/TransformSection/useReset.ts`, `ResetIcon.tsx`,
//! `FlipIcon.tsx`, and the reset/flip/Reset-all parts of `TransformRow.tsx`,
//! `TransformSection.tsx` and their CSS modules at `b8a0e1d`: each row shows
//! its reset button while its value is off its default, in a 24 px slot the
//! row keeps whether or not the button shows (so the flip toggle holds
//! still); the two scale rows carry a flip toggle showing its pressed state
//! while the value is negative; the card's head shows Reset all while any
//! value is off its default.
//!
//! This module is the pure core: the changed rule, the labels, the
//! visibility rules, and Reset all's geometry. The row's buttons geometry
//! lives with the row's layout ([`crate::transform_row`]); the icons paint
//! through [`crate::icons`] (`Reset`, `FlipVertical`, `FlipHorizontal`).
//! Layout, paint and input route through `transform_row.rs` /
//! `transform_section.rs`.

use enum_map::EnumMap;

use crate::state::TransformParam;

/// Whether the row's reset condition holds (`changed` in `useReset`): the
/// value is not the parameter's default. Exact float equality, as the TS
/// `!==` is.
#[must_use]
pub fn is_changed(param: TransformParam, value: f64) -> bool {
    value != crate::transform_section::param_def(param).default
}

/// The value the reset puts back (`reset`: `setValue(defaultValue, flush)`).
#[must_use]
pub fn reset_value(param: TransformParam) -> f64 {
    crate::transform_section::param_def(param).default
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

/// The toggle's icon kind by row.
#[must_use]
pub fn flip_icon(param: TransformParam) -> crate::icons::IconKind {
    if param == TransformParam::A {
        crate::icons::IconKind::FlipVertical
    } else {
        crate::icons::IconKind::FlipHorizontal
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
/// `dragged` is whether h5-box's drag names this parameter; see
/// [`dragged_param`]). The slot keeps its 24 px either way (`8261442`).
#[must_use]
pub fn reset_shown(changed: bool, dragged: bool) -> bool {
    changed && !dragged
}

/// The parameter whose box h5-box's drag holds, if any. h5-box owns the drag
/// state (`App.scrub`, plan's `ScrubDrag`); until it merges this answers
/// `None`, so the wiring stays green and the rule stays unit-tested pure
/// above. Replace the body with the `App.scrub` read at h5-box's merge.
#[must_use]
pub fn dragged_param(_app: &crate::app::App) -> Option<TransformParam> {
    None
}

/// Whether the head shows Reset all: any of the four values off its default.
#[must_use]
pub fn any_changed(values: &EnumMap<TransformParam, f64>) -> bool {
    values
        .iter()
        .any(|(param, value)| is_changed(param, *value))
}

/// Reset all in the card's head: flex, gap 5, 20 px tall, margin-top 4,
/// padding 2×4, 13 px/15 px strong, `--radius-card` corners.
pub const RESET_ALL_HEIGHT: f32 = 20.0;
pub const RESET_ALL_FONT_SIZE: f32 = 13.0;
pub const RESET_ALL_LINE_HEIGHT: f32 = 15.0;
pub const RESET_ALL_GAP: f32 = 5.0;
pub const RESET_ALL_PAD_X: f32 = 4.0;
pub const RESET_ALL_PAD_Y: f32 = 2.0;
pub const RESET_ALL_MARGIN_TOP: f32 = 4.0;

/// Reset all's copy.
pub const RESET_ALL_LABEL: &str = "Reset all";
/// The reset icon's svg size in the 24 px buttons (`ResetIcon.tsx`, 16).
pub const RESET_ICON_SIZE: f32 = 16.0;
/// The flip icon's svg size in the 24 px toggles (`FlipIcon.tsx`, 14).
pub const FLIP_ICON_SIZE: f32 = 14.0;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::functions::A_PARAM;
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
        // their own axis, icon and name; k and h carry none.
        assert!(is_scale(TransformParam::A));
        assert!(is_scale(TransformParam::B));
        assert!(!is_scale(TransformParam::K));
        assert!(!is_scale(TransformParam::H));
        assert_eq!(flip_axis(TransformParam::A), FlipAxis::Vertical);
        assert_eq!(flip_axis(TransformParam::B), FlipAxis::Horizontal);
        assert_eq!(
            flip_icon(TransformParam::A),
            crate::icons::IconKind::FlipVertical
        );
        assert_eq!(
            flip_icon(TransformParam::B),
            crate::icons::IconKind::FlipHorizontal
        );
        assert_eq!(flip_label(TransformParam::A), "Flip upside down");
        assert_eq!(flip_label(TransformParam::B), "Mirror left to right");
    }

    #[test]
    fn reset_all_shows_while_any_value_is_off_and_restores_all() {
        // The section test: a link with a=2&h=3 shows Reset all; clicking it
        // puts every default back, drops both keys, and hides the button.
        let mut url = Url::from_query("a=2&h=3");
        let mut params = EnumMap::from_fn(|param| {
            *QueryParam::new(crate::transform_section::param_def(param), "a=2&h=3").get()
        });
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
            let mut bound =
                QueryParam::new(crate::transform_section::param_def(param), url.query());
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
        assert_eq!(RESET_ALL_LABEL, "Reset all");
        assert_eq!((RESET_ICON_SIZE, FLIP_ICON_SIZE), (16.0, 14.0));
    }

    #[test]
    fn buttons_keep_the_rows_layout() {
        // The keeps-place test's arithmetic: the flip stands at
        // pad + chip + gap + name + gap, where the name takes what the flip,
        // the slot, the box and the fixed chrome leave — so showing or hiding
        // the reset (or the drag hiding it) never moves the flip.
        // Five children, four gaps: 8 + 26 + 12 + name + 12 + 24 + 12 + 24
        // + 12 + 107 + 6 = 350.
        let name = 350.0
            - (crate::transform_row::ROW_PAD_LEFT
                + crate::transform_row::CHIP_SIZE
                + FLIP_SIZE
                + RESET_SLOT_WIDTH
                + crate::transform_row::BOX_WIDTH
                + crate::transform_row::ROW_PAD_RIGHT
                + 4.0 * crate::transform_row::ROW_GAP);
        assert_eq!(name, 107.0);
        assert_eq!(RESET_SIZE, FLIP_SIZE);
        assert_eq!(RESET_SLOT_WIDTH, RESET_SIZE);
    }
}
