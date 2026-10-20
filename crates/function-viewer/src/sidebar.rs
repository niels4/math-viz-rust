//! The sidebar column: the Function card and the Transform card.
//!
//! Ported from `FunctionViewer/SideBar/SideBar.tsx` at `3560eb2` (the column;
//! the Transform card joins it at `5714319`). The
//! sidebar's box (background, border, `--space-inset` padding at the h4 hash)
//! stays `page.rs`'s `side` declaration, as the top bar's box stays
//! `top_bar.rs`'s; this column lays the cards inside it.

use mv_sys::clay;

use crate::app::App;
use crate::functions::FunctionSlug;

/// The gap between the sidebar's cards (`--space-inset`).
pub const CARD_GAP: f32 = 12.0;

/// The sidebar's inner padding (`--space-inset` at the h4 hash; `--space-card`
/// before): `page.rs`'s `side` declaration reads it here, beside `CARD_GAP`.
pub const SIDE_PAD: f32 = 12.0;

/// Lays out the sidebar's cards in the open sidebar scope. `current` is the
/// picked function.
pub fn sidebar(app: &mut App, current: FunctionSlug) {
    let mut col = crate::ui::empty_decl();
    col.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
        height: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
    };
    col.layout.childGap = CARD_GAP as u16;
    col.layout.layoutDirection = clay::LayoutDirection::TopToBottom;
    crate::ui::ui_scope(app, mv_clay::id("SidebarColumn", 0), &col, |app| {
        crate::selector::function_card(app, current);
        // The Transform card (h4-rows' `transform_section`).
        crate::transform_section::section(app);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cards_stand_an_inset_apart() {
        assert_eq!(CARD_GAP, 12.0);
    }
}
