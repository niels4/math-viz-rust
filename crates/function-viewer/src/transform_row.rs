//! One transform parameter's row: its chip, name and typed number field.
//!
//! Ported from `SideBar/TransformRow.tsx` and `TransformRow.module.css` at
//! `5714319`: the 44 px row (`data-param`, `data-lit`), the 26 px chip with
//! the parameter's letter in its colour, the name, and the 107×32 box with
//! two decorative chevrons around the field ([`crate::number_field`]).
//! Pointing at the row or focusing its box highlights the parameter
//! everywhere it shows; the row lights up wherever the highlight comes from.
//!
//! Layout and paint wire into the sidebar in phase 2 (h3's column, h4-readout's
//! readout); this module holds the CSS-derived constants, the highlight sync
//! state, and the chevron geometry. Hit kinds (`input.rs`) and the field's
//! commit wiring join with the `App` fields they need.

use enum_map::Enum;

// stub until h4-readout: `state.rs` owns this section (the port of
// `FunctionViewer/state.ts`). Names follow the plan (`TransformParam` with
// `A B H K`, `HighlightState` with hover/focus records, `lit()`, `set_hover`,
// `set_focus`); `colour_token()` joins with the `--param-*` tokens in phase 2.
// Delete this section at the merge and `use crate::state::{...}` instead.

/// A transform parameter, by its letter (`TransformParam` in `state.ts`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum TransformParam {
    A,
    B,
    H,
    K,
}

impl TransformParam {
    /// The letter: the chip's copy, `data-param`, and the URL key.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::A => "a",
            Self::B => "b",
            Self::H => "h",
            Self::K => "k",
        }
    }
}

/// Who moved last (`lead` in `touchAtom`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Touch {
    Hover,
    Focus,
}

/// The highlight model (`touchAtom` + `litAtom`): the parameter the pointer
/// is on, the one whose row has the keyboard's focus, and which moved last.
/// The readout's terms read the same state through [`HighlightState::lit`],
/// so a row and its equation light together.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HighlightState {
    hover: Option<TransformParam>,
    focus: Option<TransformParam>,
    lead: Touch,
}

impl Default for HighlightState {
    fn default() -> Self {
        Self::new()
    }
}

impl HighlightState {
    /// Nothing touched.
    #[must_use]
    pub fn new() -> Self {
        Self {
            hover: None,
            focus: None,
            lead: Touch::Hover,
        }
    }

    /// The pointer moved onto (`on`) or off a parameter (`touch("hover")`).
    /// Leaving one the pointer already moved past changes nothing.
    pub fn set_hover(&mut self, param: TransformParam, on: bool) {
        if on {
            self.hover = Some(param);
            self.lead = Touch::Hover;
        } else if self.hover == Some(param) {
            self.hover = None;
        }
    }

    /// The keyboard's focus entered or left a parameter's row (`touch("focus")`).
    pub fn set_focus(&mut self, param: TransformParam, on: bool) {
        if on {
            self.focus = Some(param);
            self.lead = Touch::Focus;
        } else if self.focus == Some(param) {
            self.focus = None;
        }
    }

    /// The lit parameter (`litAtom`): the lead's one, or else the other's —
    /// so focusing a's box and pointing at k lights only k.
    #[must_use]
    pub fn lit(self) -> Option<TransformParam> {
        match self.lead {
            Touch::Hover => self.hover.or(self.focus),
            Touch::Focus => self.focus.or(self.hover),
        }
    }

    /// Whether the parameter is highlighted, everywhere it shows
    /// (`highlightAtoms[param]`).
    #[must_use]
    pub fn is_lit(self, param: TransformParam) -> bool {
        self.lit() == Some(param)
    }
}

/// The row: flex, 44 px tall, `0 6px 0 8px` padding, `--space-inset` (12) gap,
/// `--radius-card` (4) corners; lit rows tint with the parameter's colour.
pub const ROW_HEIGHT: f32 = 44.0;
pub const ROW_PAD_LEFT: f32 = 8.0;
pub const ROW_PAD_RIGHT: f32 = 6.0;
pub const ROW_GAP: f32 = 12.0;
pub const ROW_RADIUS: f32 = 4.0;

/// The chip: 26×26 grid, serif 19 px/24 px, the letter on the parameter's
/// colour, `--radius-card` corners. The letter is a `var`: STIX Two Text
/// italic 400 (the face is `Stix400Italic`).
pub const CHIP_SIZE: f32 = 26.0;
pub const CHIP_FONT_SIZE: f32 = 19.0;
pub const CHIP_LINE_HEIGHT: f32 = 24.0;
pub const CHIP_RADIUS: f32 = 4.0;

/// The name: flex 1, 15 px, `--weight-strong` (600), foreground-muted —
/// foreground while lit.
pub const NAME_FONT_SIZE: f32 = 15.0;

/// The box: 107×32, `0 9.5px` padding, gap 6, mono 500 17 px/22 px
/// tabular-nums, the parameter's ink on background-muted, a 1 px ink border
/// plus an inset ink ring (0.5 rest, 1 lit or focused), and a 3 px primary
/// 32 % ring while `:focus-visible` is inside. Clay takes no border space
/// (the h1-frame rule), so its padding is the CSS padding plus the border.
pub const BOX_WIDTH: f32 = 107.0;
pub const BOX_HEIGHT: f32 = 32.0;
pub const BOX_PAD_X: f32 = 9.5;
pub const BOX_BORDER: f32 = 1.0;
pub const BOX_GAP: f32 = 6.0;
pub const BOX_FONT_SIZE: f32 = 17.0;
pub const BOX_LINE_HEIGHT: f32 = 22.0;
pub const BOX_RADIUS: f32 = 4.0;
/// The inset ring's width at rest and while lit or focused.
pub const BOX_RING_REST: f32 = 0.5;
pub const BOX_RING_LIT: f32 = 1.0;
/// The `:focus-visible` ring's width (primary 32 %, `theme::ring`).
pub const BOX_FOCUS_RING: f32 = 3.0;

/// The field: flex 1, 22 px tall, right-aligned, transparent, no outline;
/// focused text runs in the foreground.
pub const FIELD_HEIGHT: f32 = 22.0;

/// A chevron beside the value: 6×12 svg, 1.6 px round stroke in the
/// parameter's colour, no fill. Decorative (`aria-hidden`): no handler.
pub const CHEVRON_WIDTH: f32 = 6.0;
pub const CHEVRON_HEIGHT: f32 = 12.0;
pub const CHEVRON_STROKE: f32 = 1.6;

/// The left chevron's path (`M5 2L1 6L5 10`).
pub const CHEVRON_LEFT: &str = "M5 2L1 6L5 10";
/// The right chevron's path (`M1 2L5 6L1 10`).
pub const CHEVRON_RIGHT: &str = "M1 2L5 6L1 10";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_match_the_url_keys() {
        assert_eq!(
            [
                TransformParam::A,
                TransformParam::B,
                TransformParam::H,
                TransformParam::K
            ]
            .map(TransformParam::as_str),
            ["a", "b", "h", "k"]
        );
    }

    #[test]
    fn layout_matches_the_css() {
        assert_eq!(ROW_HEIGHT, 44.0);
        assert_eq!((ROW_PAD_LEFT, ROW_PAD_RIGHT, ROW_GAP), (8.0, 6.0, 12.0));
        assert_eq!(CHIP_SIZE, 26.0);
        assert_eq!(
            (BOX_WIDTH, BOX_HEIGHT, BOX_PAD_X, BOX_GAP),
            (107.0, 32.0, 9.5, 6.0)
        );
        assert_eq!(
            (CHEVRON_WIDTH, CHEVRON_HEIGHT, CHEVRON_STROKE),
            (6.0, 12.0, 1.6)
        );
        assert_eq!(FIELD_HEIGHT, 22.0);
    }

    #[test]
    fn hovering_a_row_lights_it_and_its_equation_together() {
        // The highlight test: a fresh page shows a's row unlit.
        let mut state = HighlightState::new();
        assert!(!state.is_lit(TransformParam::A));

        // Pointing at the row lights it (the form's term reads the same state).
        state.set_hover(TransformParam::A, true);
        assert!(state.is_lit(TransformParam::A));

        // Moving off it puts it out.
        state.set_hover(TransformParam::A, false);
        assert!(!state.is_lit(TransformParam::A));
    }

    #[test]
    fn focus_keeps_a_lit_after_the_pointer_leaves() {
        // The highlight test: focusing a's box lights a.
        let mut state = HighlightState::new();
        state.set_focus(TransformParam::A, true);
        assert!(state.is_lit(TransformParam::A));

        // Pointing at the row and moving off leaves a lit (focus leads now).
        state.set_hover(TransformParam::A, true);
        state.set_hover(TransformParam::A, false);
        assert!(state.is_lit(TransformParam::A));

        // Leaving the box puts it out.
        state.set_focus(TransformParam::A, false);
        assert!(!state.is_lit(TransformParam::A));
    }

    #[test]
    fn only_the_last_moved_parameter_lights() {
        // The highlight test: focusing a's box lights a, not k.
        let mut state = HighlightState::new();
        state.set_focus(TransformParam::A, true);
        assert!(state.is_lit(TransformParam::A));
        assert!(!state.is_lit(TransformParam::K));

        // Pointing at k lights k and puts a out.
        state.set_hover(TransformParam::K, true);
        assert!(state.is_lit(TransformParam::K));
        assert!(!state.is_lit(TransformParam::A));

        // Moving off k gives the light back to a, which still has the focus.
        state.set_hover(TransformParam::K, false);
        assert!(!state.is_lit(TransformParam::K));
        assert!(state.is_lit(TransformParam::A));
    }

    #[test]
    fn leaving_a_superseded_touch_changes_nothing() {
        // Off for a parameter the touch already moved past is a no-op.
        let mut state = HighlightState::new();
        state.set_hover(TransformParam::A, true);
        state.set_hover(TransformParam::K, true);
        state.set_hover(TransformParam::A, false);
        assert!(state.is_lit(TransformParam::K));
    }
}
