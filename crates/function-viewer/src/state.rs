//! The highlight model: which transform parameters are lit, and where.
//!
//! Ported from `src/components/views/FunctionViewer/state.ts` at `251c080`:
//! the parameters the pointer is on (in their rows' controls, in their parts
//! of the equation, or on one of the plane's handles), the parameters whose
//! rows have the keyboard's focus, which of the two sides moved last, the
//! per-parameter scrub modes, and the jump list the plane animates. math-viz2
//! keeps these in a `touchAtom` (`{hover, focus, lead}` with setter atoms), a
//! `scrubModeAtoms` record and a `jumpsAtom` list; the port keeps one struct
//! on `App` (`App.state`) with the derived answers as methods. Like the
//! source, this state lives only while the page is open: it is never in the
//! URL or storage.
//!
//! The sibling contract (plan §3.2, h4-rows writes against these names, h7
//! extends them): `HighlightState { hover, focus }`, `Touch { param, at }`,
//! `HighlightState::lit(&self) -> Option<TransformParam>`,
//! `set_hover/set_focus(Option<TransformParam>)`, and `TransformParam { A, B,
//! H, K }` with `colour_token()`. h7 keeps every name and widens the shapes:
//! hover/focus hold ordered lists, `lit` returns the ordered lit list, the
//! per-parameter scrub modes and the jump list ride on the same struct (they
//! are separate atoms in the source, one `App.state` here).

use enum_map::EnumMap;

use crate::scrub::ScrubMode;

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
    /// Named as_str, as the readout's terms and h4-rows' rows spell it, so
    /// the transform-parameter definitions share the spelling.
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
    /// `themes.rs` when the readout was wired in phase 2).
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

/// Which side of the highlight moved last (the source's `Touch`: the
/// `touchAtom`'s `lead`, `"hover"` at rest).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Touch {
    /// The pointer's side moved last.
    #[default]
    Hover,
    /// The keyboard's side moved last.
    Focus,
}

/// One animated parameter reset: the parameter, the value it eases from and
/// the value it eases to (one element of the source's `jumpsAtom` list: the
/// flip sets one, a row's reset sets one, Reset all sets four).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Jump {
    pub param: TransformParam,
    pub from: f64,
    pub to: f64,
}

/// The jump list (the source's `jumpsAtom`): the whole list is set at once,
/// and `seq` tells the animation apart. Every set mints a new list in the
/// source (a fresh array identity), so every set here bumps `seq` even with
/// equal values, and `ShownTransform` compares `seq`, exactly like the
/// hook's `jumps !== seenJumps`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Jumps {
    pub list: Vec<Jump>,
    pub seq: u64,
}

impl Jumps {
    /// Sets the whole list (the flip's one jump, a row's one, Reset all's
    /// four in a-k-b-h order), minting a fresh identity.
    pub fn set(&mut self, list: Vec<Jump>) {
        self.list = list;
        self.seq = self.seq.saturating_add(1);
    }

    /// Sets a one-jump list, minting a fresh identity: the flip's call stays
    /// one line (`app.state.jumps.single(param, from, to)`). A method, not
    /// an associated function, so the fresh identity comes from the bump,
    /// never a global.
    pub fn single(&mut self, param: TransformParam, from: f64, to: f64) {
        self.set(vec![Jump { param, from, to }]);
    }
}

/// Whether two parameter lists hold the same parameters in the same order
/// (the source's `sameParams`).
#[must_use]
fn same_params(x: &[TransformParam], y: &[TransformParam]) -> bool {
    x.len() == y.len() && x.iter().zip(y.iter()).all(|(a, b)| a == b)
}

/// Which transform parameters are lit, and where.
///
/// Ported from `state.ts` (`touchAtom` + `litAtom` + `highlightAtoms` +
/// `scrubModeAtoms` + `jumpsAtom`). `hover` is the ordered list of
/// parameters the pointer is on (a row or term contributes its one, a plane
/// handle its two); `focus` is the ordered list whose rows have the
/// keyboard's focus; `lead` is the side that moved last. Each
/// `set_hover`/`set_focus` follows the source's `touch` writer: on stores
/// the list and takes the lead, off clears only when the stored list equals
/// the given one element for element. `scrub_mode` is the `scrubModeAtoms`
/// record (read and written by the boxes, and by the plane's handle drag);
/// `jumps` is the `jumpsAtom` list (set whole by the flip, a reset and
/// Reset all).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HighlightState {
    pub hover: Vec<TransformParam>,
    pub focus: Vec<TransformParam>,
    pub lead: Touch,
    pub scrub_mode: EnumMap<TransformParam, Option<ScrubMode>>,
    pub jumps: Jumps,
    /// P's x slider is being dragged (`pDraggingAtom`, h8).
    pub p_dragging: bool,
    /// The plane pointer's quantized x, while it is over the plane and off
    /// the handles (`qXAtom`, h8); `None` elsewhere.
    pub q_x: Option<f64>,
}

impl HighlightState {
    /// The untouched state: nothing hovered, nothing focused, no modes, no
    /// jumps.
    #[must_use]
    pub fn touched_out() -> Self {
        Self::default()
    }

    /// The lit parameters, in order: the scrubbed parameters (a-b-h-k) while
    /// any parameter's scrub mode is set, else the lead touch's list while
    /// it is non-empty, else the other's (a side's empty list falls through
    /// to the other, as `first.length > 0 ? first : second` does).
    ///
    /// Ported from `state.ts` (`litAtom`).
    #[must_use]
    pub fn lit(&self) -> Vec<TransformParam> {
        const ORDER: [TransformParam; 4] = [
            TransformParam::A,
            TransformParam::B,
            TransformParam::H,
            TransformParam::K,
        ];
        let scrubbed: Vec<TransformParam> = ORDER
            .iter()
            .copied()
            .filter(|param| self.scrub_mode[*param].is_some())
            .collect();
        if !scrubbed.is_empty() {
            return scrubbed;
        }
        let (first, second) = match self.lead {
            Touch::Hover => (&self.hover, &self.focus),
            Touch::Focus => (&self.focus, &self.hover),
        };
        if first.is_empty() {
            second.clone()
        } else {
            first.clone()
        }
    }

    /// Whether the parameter is lit, everywhere it shows (both lines, its
    /// row, the plane's lines): the per-parameter answer the source's
    /// `highlightAtoms` give.
    #[must_use]
    pub fn is_lit(&self, param: TransformParam) -> bool {
        self.lit().contains(&param)
    }

    /// Whether any transform parameter is being scrubbed (`transformDragAtom`,
    /// h8): a scrub mode set on any of a, b, h or k. The moved badge reads it
    /// as its drag flag.
    #[must_use]
    pub fn transform_drag(&self) -> bool {
        self.scrub_mode.values().any(|mode| mode.is_some())
    }

    /// Touches or untouches the pointer's side.
    ///
    /// Ported from `state.ts` (`hoverAtom`): `on` puts the pointer on the
    /// parameters and takes the lead; off takes the pointer off only when
    /// the stored list equals the given one element for element. To drop
    /// the side unconditionally (the pointer is on nothing), pass an empty
    /// list with `on`: the emptied side reads as absent whichever side
    /// leads. That is the source's off except in one case: the source's
    /// off with a stale list is a no-op (it keeps the old list and the old
    /// lead), while on-with-empty clears unconditionally and takes the
    /// lead. The lit answer is the same either way (an emptied side falls
    /// through to the other side), and the only stale off the source ever
    /// fires is the plane's handle-to-handle hover-end, which the plane's
    /// dispatch replicates element for element (h7-plane's handle-hover-move
    /// proves it: `pan` with a and b lit).
    pub fn set_hover(&mut self, params: &[TransformParam], on: bool) {
        if on {
            self.hover = params.to_vec();
            self.lead = Touch::Hover;
        } else if same_params(&self.hover, params) {
            self.hover.clear();
        }
    }

    /// Touches or untouches the focus's side.
    ///
    /// Ported from `state.ts` (`focusAtom`): `on` focuses the parameters'
    /// rows and takes the lead; off unfocuses only when the stored list
    /// equals the given one element for element.
    pub fn set_focus(&mut self, params: &[TransformParam], on: bool) {
        if on {
            self.focus = params.to_vec();
            self.lead = Touch::Focus;
        } else if same_params(&self.focus, params) {
            self.focus.clear();
        }
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
        assert_eq!(HighlightState::touched_out().lit(), Vec::new());
    }

    #[test]
    fn hover_lights_its_parameter() {
        let mut state = HighlightState::touched_out();
        state.set_hover(&[TransformParam::A], true);
        assert_eq!(state.lit(), vec![TransformParam::A]);
        assert!(state.is_lit(TransformParam::A));
        assert!(!state.is_lit(TransformParam::H));
    }

    #[test]
    fn leaving_takes_the_light_back_out() {
        let mut state = HighlightState::touched_out();
        state.set_hover(&[TransformParam::A], true);
        state.set_hover(&[TransformParam::A], false);
        assert_eq!(state.lit(), Vec::new());
        state.set_focus(&[TransformParam::K], true);
        state.set_focus(&[TransformParam::K], false);
        assert_eq!(state.lit(), Vec::new());
    }

    #[test]
    fn whoever_moved_last_wins() {
        // Focusing a's row, then pointing at k, lights only k.
        let mut state = HighlightState::touched_out();
        state.set_focus(&[TransformParam::A], true);
        assert_eq!(state.lit(), vec![TransformParam::A]);
        state.set_hover(&[TransformParam::K], true);
        assert_eq!(state.lit(), vec![TransformParam::K]);
        assert!(!state.is_lit(TransformParam::A));
        // Moving the focus again takes the lead back.
        state.set_focus(&[TransformParam::H], true);
        assert_eq!(state.lit(), vec![TransformParam::H]);
    }

    #[test]
    fn the_other_side_lights_once_the_lead_clears() {
        // Hover moved last, then leaves: the focused parameter lights.
        let mut state = HighlightState::touched_out();
        state.set_focus(&[TransformParam::A], true);
        state.set_hover(&[TransformParam::K], true);
        state.set_hover(&[TransformParam::K], false);
        assert_eq!(state.lit(), vec![TransformParam::A]);
        // Focus moved last, then leaves: the hovered parameter lights.
        state.set_focus(&[TransformParam::H], true);
        state.set_focus(&[TransformParam::H], false);
        assert_eq!(state.lit(), Vec::new());
        state.set_hover(&[TransformParam::B], true);
        state.set_focus(&[TransformParam::H], true);
        state.set_focus(&[TransformParam::H], false);
        assert_eq!(state.lit(), vec![TransformParam::B]);
    }

    #[test]
    fn hover_off_leaves_a_newer_list_lit() {
        // The plane's hover-end rule (`251c080`): the shift handle's end
        // fires after the pointer is already on the scale handle, and must
        // not clear the newer list.
        let mut state = HighlightState::touched_out();
        state.set_hover(&[TransformParam::H, TransformParam::K], true);
        assert_eq!(state.lit(), vec![TransformParam::H, TransformParam::K]);
        state.set_hover(&[TransformParam::A, TransformParam::B], true);
        state.set_hover(&[TransformParam::H, TransformParam::K], false);
        assert_eq!(state.lit(), vec![TransformParam::A, TransformParam::B]);
        // Its own end clears it.
        state.set_hover(&[TransformParam::A, TransformParam::B], false);
        assert_eq!(state.lit(), Vec::new());
    }

    #[test]
    fn scrubbed_params_light_in_a_b_h_k_order_while_any_mode_is_set() {
        // The plane-drag light (`606fc30`): setting h and k coarse lights
        // h and k, not a — and a ruler is due for exactly the params whose
        // mode is set (`ScrubValue.tsx` renders the ruler while mode is set).
        let mut state = HighlightState::touched_out();
        state.scrub_mode[TransformParam::H] = Some(ScrubMode::Coarse);
        state.scrub_mode[TransformParam::K] = Some(ScrubMode::Coarse);
        assert_eq!(state.lit(), vec![TransformParam::H, TransformParam::K]);
        assert!(state.is_lit(TransformParam::H));
        assert!(state.is_lit(TransformParam::K));
        assert!(!state.is_lit(TransformParam::A));
        for param in [
            TransformParam::A,
            TransformParam::B,
            TransformParam::H,
            TransformParam::K,
        ] {
            assert_eq!(
                state.scrub_mode[param].is_some(),
                param == TransformParam::H || param == TransformParam::K
            );
        }
        // The modes win over the touches, and clearing them falls back.
        state.set_hover(&[TransformParam::A], true);
        assert_eq!(state.lit(), vec![TransformParam::H, TransformParam::K]);
        state.scrub_mode[TransformParam::H] = None;
        state.scrub_mode[TransformParam::K] = None;
        assert_eq!(state.lit(), vec![TransformParam::A]);
    }

    #[test]
    fn every_jumps_set_mints_a_fresh_identity() {
        // The hook compares lists by identity, never by value: two equal
        // sets still restart the animation.
        let mut jumps = Jumps::default();
        let first = jumps.seq;
        jumps.single(TransformParam::A, 2.0, -2.0);
        assert_eq!(
            jumps.list,
            vec![Jump {
                param: TransformParam::A,
                from: 2.0,
                to: -2.0
            }]
        );
        jumps.single(TransformParam::A, 2.0, -2.0);
        assert_ne!(jumps.seq, first);
        assert_eq!(jumps.list.len(), 1);
    }
}
