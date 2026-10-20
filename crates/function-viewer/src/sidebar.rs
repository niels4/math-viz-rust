//! The sidebar column: the Function card now, the Transform card from h4.
//!
//! Ported from `FunctionViewer/SideBar/SideBar.tsx` at `3560eb2`. The
//! sidebar's box (background, border, `--space-card` padding) stays
//! `page.rs`'s `side` declaration, as the top bar's box stays `top_bar.rs`'s;
//! this column lays the cards inside it.

use mv_sys::clay;

use crate::app::App;
use crate::functions::FunctionSlug;

/// The gap between the sidebar's cards (`--space-inset`, measured at the
/// h4 hash; unobservable while the column holds one card, and h4 owns it
/// from its Transform card on).
pub const CARD_GAP: f32 = 12.0;

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
