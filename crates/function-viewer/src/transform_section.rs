//! The sidebar's Transform card: the four parameter rows.
//!
//! Ported from `SideBar/TransformSection.tsx` and
//! `TransformSection.module.css` at `b8a0e1d`: the card over the sidebar
//! column, the "Transform" legend with Reset all beside it while any value
//! is off its default, and the rows y scale/a, y shift/k, a rule, x scale/b,
//! x shift/h — the parameters outside `f( )` first (they move the curve
//! vertically), then the ones inside (they move it horizontally). The names
//! are the shortened ones (`999e0a0`).
//!
//! The card's box (background, radius, padding) stays the sidebar's
//! declaration, as the Function card's does; this module lays the title, the
//! rows and the rule inside it. Wiring into h3's column joins in phase 2.
//! Reset all's behaviour ([`crate::reset::any_changed`]) and geometry
//! ([`crate::reset`]'s `RESET_ALL_*`) wire into the head then.

use crate::transform_row::TransformParam;

/// The card: `sidebarStyle.card` — section background, `--radius-card` (4)
/// corners, `--space-inset` (12) padding, `--space-section` (16) at the
/// bottom.
pub const CARD_PAD_X: f32 = 12.0;
pub const CARD_PAD_TOP: f32 = 12.0;
pub const CARD_PAD_BOTTOM: f32 = 16.0;
pub const CARD_RADIUS: f32 = 4.0;

/// The legend's copy.
pub const TITLE: &str = "Transform";

/// The legend's block: `--text-heading` (17 px), `--weight-heading` (700),
/// 28 px line, `--space-inset` (12) under it (`card_title`). The head holds
/// the legend and, while any value is off its default, Reset all
/// (`align-items: flex-start`, `justify-content: space-between`); Reset all
/// is 20 px in a 28 px line, so the head stays 28 px tall either way and the
/// card's height never moves.
pub const TITLE_FONT_SIZE: f32 = 17.0;
pub const TITLE_LINE_HEIGHT: f32 = 28.0;
pub const TITLE_GAP: f32 = 12.0;

/// The rule between the vertical and horizontal rows: 1 px tall,
/// `background-muted`, 8 px margins.
pub const RULE_HEIGHT: f32 = 1.0;
pub const RULE_MARGIN: f32 = 8.0;

/// The rows in the source's order: the vertical pair, then the horizontal
/// pair (`TransformSection.tsx`).
pub const ROW_ORDER: [TransformParam; 4] = [
    TransformParam::A,
    TransformParam::K,
    TransformParam::B,
    TransformParam::H,
];

/// Each row's shortened name, by parameter (`999e0a0`).
#[must_use]
pub fn row_name(param: TransformParam) -> &'static str {
    match param {
        TransformParam::A => "y scale",
        TransformParam::K => "y shift",
        TransformParam::B => "x scale",
        TransformParam::H => "x shift",
    }
}

/// The card's measured height arithmetic (CSS px): top pad + title + gap +
/// four rows + the rule's margins and height + bottom pad. Reset all fits in
/// the title's line, so the height holds with it shown (probed 261 px with
/// `a=2`, `a=-2` and `a=2&h=3`).
#[must_use]
pub fn card_height() -> f32 {
    CARD_PAD_TOP
        + TITLE_LINE_HEIGHT
        + TITLE_GAP
        + 4.0 * crate::transform_row::ROW_HEIGHT
        + (2.0 * RULE_MARGIN + RULE_HEIGHT)
        + CARD_PAD_BOTTOM
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::functions::{A_PARAM, B_PARAM, H_PARAM, K_PARAM};
    use crate::number_field::display_value;
    use crate::url::{QueryParam, QueryParamDef};

    #[test]
    fn rows_stand_in_the_sources_order() {
        assert_eq!(
            ROW_ORDER.map(row_name),
            ["y scale", "y shift", "x scale", "x shift"]
        );
        assert_eq!(ROW_ORDER.map(TransformParam::as_str), ["a", "k", "b", "h"]);
    }

    #[test]
    fn card_arithmetic_matches_the_reference() {
        assert_eq!(CARD_PAD_TOP + TITLE_LINE_HEIGHT + TITLE_GAP, 52.0);
        assert_eq!(2.0 * RULE_MARGIN + RULE_HEIGHT, 17.0);
        assert_eq!(card_height(), 261.0);
    }

    fn def_of(param: TransformParam) -> &'static QueryParamDef<f64> {
        match param {
            TransformParam::A => &A_PARAM,
            TransformParam::K => &K_PARAM,
            TransformParam::B => &B_PARAM,
            TransformParam::H => &H_PARAM,
        }
    }

    #[test]
    fn each_row_shows_the_value_a_link_sets() {
        // The transform-card test: a link setting all four parameters shows
        // each value in its own row with two decimals.
        let query = "a=2&k=-1&b=0.5&h=3";
        let shown: Vec<String> = ROW_ORDER
            .iter()
            .map(|param| display_value(*QueryParam::new(def_of(*param), query).get()))
            .collect();
        assert_eq!(shown, vec!["2.00", "-1.00", "0.50", "3.00"]);
    }
}
