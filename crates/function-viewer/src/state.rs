//! The highlight model: which transform parameter is lit, and where.
//!
//! Ported from `src/components/views/FunctionViewer/state.ts` at `5714319`:
//! the parameter the pointer is on (in its row's control or in its part of
//! the equation), the parameter whose row has the keyboard's focus, and which
//! of the two moved last. math-viz2 keeps this in one `touchAtom`
//! (`{hover, focus, lead}`) with setter atoms and derives `litAtom` (the
//! lead's parameter, else the other's) plus one highlight atom per parameter;
//! the port keeps one struct on `App` (`App.state`, appended in phase 2) with
//! the derived answers as methods. Like the source, this state lives only
//! while the page is open: it is never in the URL or storage.
//!
//! The sibling contract (plan §3.2, h4-rows writes against these names):
//! `HighlightState { hover: Option<Touch>, focus: Option<Touch> }`,
//! `Touch { param: TransformParam, at: u64 }`,
//! `HighlightState::lit(&self) -> Option<TransformParam>`,
//! `set_hover(Option<TransformParam>)`, `set_focus(Option<TransformParam>)`,
//! and `TransformParam { A, B, H, K }` with `colour_token()`.

/// A transform parameter: the letter in the equation and on its row.
///
/// Ported from `state.ts` (`TransformParam = "a" | "b" | "h" | "k"`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, enum_map::Enum)]
pub enum TransformParam {
    #[default]
    A,
    B,
    H,
    K,
}

impl TransformParam {
    /// The letter as the URL, the equation and the rows spell it.
    ///
    /// Named as  and h4-rows' stub spell it, so the
    /// three transform-parameter definitions share the spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::A => "a",
            Self::B => "b",
            Self::H => "h",
            Self::K => "k",
        }
    }

    /// Reads a letter back; anything else is `None`.
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "a" => Some(Self::A),
            "b" => Some(Self::B),
            "h" => Some(Self::H),
            "k" => Some(Self::K),
            _ => None,
        }
    }

    /// The parameter's colour token: `--param-a/k/b/h` from
    /// `SideBar.module.css`, resolved per theme (added to `tokens.toml` and
    /// `themes.rs` when the readout is wired in phase 2).
    #[must_use]
    pub fn colour_token(self) -> mv_theme::Token {
        match self {
            Self::A => mv_theme::Token::ParamA,
            Self::B => mv_theme::Token::ParamB,
            Self::H => mv_theme::Token::ParamH,
            Self::K => mv_theme::Token::ParamK,
        }
    }
}

/// One side's touch: the parameter it is on, and when it moved there.
///
/// The `at` is the highlight state's own move sequence (one step per touch),
/// so the later of two touches always wins, exactly as the source's `lead`
/// (whoever moved last) does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Touch {
    pub param: TransformParam,
    pub at: u64,
}

/// Which parameter the pointer and the focus touch, and which moved last.
///
/// Ported from `state.ts` (`touchAtom` + `litAtom` + `highlightAtoms`).
/// `hover` is the parameter the pointer is on, in its row or in its part of
/// the equation; `focus` is the parameter whose row has the keyboard's focus.
/// Each setter follows the source's `touch` writer: touching sets the slot
/// and takes the lead; leaving takes the slot back to `None` (the port's
/// input layer sends `None` only when its side is on no parameter, so the
/// outcome equals the source's slot-specific off).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HighlightState {
    pub hover: Option<Touch>,
    pub focus: Option<Touch>,
    /// The move sequence: the next touch takes this number, then it advances.
    next: u64,
}

impl HighlightState {
    /// The untouched state: nothing hovered, nothing focused.
    #[must_use]
    pub fn touched_out() -> Self {
        Self::default()
    }

    /// The parameter the pointer or the focus moved to last, else the one the
    /// other is on: focusing a's row and pointing at k lights only k, while
    /// the lead's slot is empty the other still lights.
    ///
    /// Ported from `state.ts` (`litAtom`: the lead's slot, else the other).
    /// The present touch with the greater `at` is the lead; the lead cannot
    /// tie (every touch advances the sequence), so this matches the source
    /// in every combination, including the lead clearing first.
    #[must_use]
    pub fn lit(&self) -> Option<TransformParam> {
        match (self.hover, self.focus) {
            (None, None) => None,
            (Some(touch), None) | (None, Some(touch)) => Some(touch.param),
            (Some(hover), Some(focus)) => Some(if hover.at > focus.at {
                hover.param
            } else {
                focus.param
            }),
        }
    }

    /// Whether the parameter is lit, everywhere it shows (both lines, its
    /// row): the per-parameter answer the source's `highlightAtoms` give.
    #[must_use]
    pub fn is_lit(&self, param: TransformParam) -> bool {
        self.lit() == Some(param)
    }

    /// Touches or untouches the pointer's side.
    ///
    /// Ported from `state.ts` (`hoverAtom`): `Some` puts the pointer on the
    /// parameter and takes the lead; `None` takes the pointer off.
    pub fn set_hover(&mut self, param: Option<TransformParam>) {
        self.hover = self.touch(param);
    }

    /// Touches or untouches the focus's side.
    ///
    /// Ported from `state.ts` (`focusAtom`): `Some` focuses the parameter's
    /// row and takes the lead; `None` takes the focus off.
    pub fn set_focus(&mut self, param: Option<TransformParam>) {
        self.focus = self.touch(param);
    }

    /// One touch: a stamped slot, or empty. Every touch advances the sequence,
    /// so re-touching the parameter the lead already holds still moves last,
    /// as the source's setter always rewrites `lead`.
    fn touch(&mut self, param: Option<TransformParam>) -> Option<Touch> {
        let param = param?;
        let at = self.next;
        self.next = self.next.saturating_add(1);
        Some(Touch { param, at })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_spell_a_b_h_k() {
        assert_eq!(TransformParam::A.as_str(), "a");
        assert_eq!(TransformParam::B.as_str(), "b");
        assert_eq!(TransformParam::H.as_str(), "h");
        assert_eq!(TransformParam::K.as_str(), "k");
    }

    #[test]
    fn parse_reads_the_letters_back() {
        assert_eq!(TransformParam::parse("a"), Some(TransformParam::A));
        assert_eq!(TransformParam::parse("b"), Some(TransformParam::B));
        assert_eq!(TransformParam::parse("h"), Some(TransformParam::H));
        assert_eq!(TransformParam::parse("k"), Some(TransformParam::K));
        assert_eq!(TransformParam::parse("x"), None);
        assert_eq!(TransformParam::parse(""), None);
    }

    #[test]
    fn colour_tokens_follow_the_param_names() {
        assert_eq!(TransformParam::A.colour_token(), mv_theme::Token::ParamA);
        assert_eq!(TransformParam::B.colour_token(), mv_theme::Token::ParamB);
        assert_eq!(TransformParam::H.colour_token(), mv_theme::Token::ParamH);
        assert_eq!(TransformParam::K.colour_token(), mv_theme::Token::ParamK);
    }

    #[test]
    fn untouched_state_lights_nothing() {
        assert_eq!(HighlightState::touched_out().lit(), None);
    }

    #[test]
    fn hover_lights_its_parameter() {
        let mut state = HighlightState::touched_out();
        state.set_hover(Some(TransformParam::A));
        assert_eq!(state.lit(), Some(TransformParam::A));
        assert!(state.is_lit(TransformParam::A));
        assert!(!state.is_lit(TransformParam::H));
    }

    #[test]
    fn leaving_takes_the_light_back_out() {
        let mut state = HighlightState::touched_out();
        state.set_hover(Some(TransformParam::A));
        state.set_hover(None);
        assert_eq!(state.lit(), None);
        state.set_focus(Some(TransformParam::K));
        state.set_focus(None);
        assert_eq!(state.lit(), None);
    }

    #[test]
    fn whoever_moved_last_wins() {
        // Focusing a's row, then pointing at k, lights only k.
        let mut state = HighlightState::touched_out();
        state.set_focus(Some(TransformParam::A));
        assert_eq!(state.lit(), Some(TransformParam::A));
        state.set_hover(Some(TransformParam::K));
        assert_eq!(state.lit(), Some(TransformParam::K));
        assert!(!state.is_lit(TransformParam::A));
        // Moving the focus again takes the lead back.
        state.set_focus(Some(TransformParam::H));
        assert_eq!(state.lit(), Some(TransformParam::H));
    }

    #[test]
    fn the_other_side_lights_once_the_lead_clears() {
        // Hover moved last, then leaves: the focused parameter lights.
        let mut state = HighlightState::touched_out();
        state.set_focus(Some(TransformParam::A));
        state.set_hover(Some(TransformParam::K));
        state.set_hover(None);
        assert_eq!(state.lit(), Some(TransformParam::A));
        // Focus moved last, then leaves: the hovered parameter lights.
        state.set_focus(Some(TransformParam::H));
        state.set_focus(None);
        assert_eq!(state.lit(), None);
        state.set_hover(Some(TransformParam::B));
        state.set_focus(Some(TransformParam::H));
        state.set_focus(None);
        assert_eq!(state.lit(), Some(TransformParam::B));
    }
}
