//! A transform row's typed number field.
//!
//! Ported from the field half of `SideBar/ScrubValue/ScrubValue.tsx` at
//! `b8a0e1d` (the box's input; the h4 `TransformRow.tsx` field became this
//! box): the text in the box while you type (`draft: Option<String>` —
//! partial numbers such as `"-"` or `"1."` stay here), shown as typed; Enter
//! or leaving the box commits (`commit` — the parse decides, text the URL
//! wouldn't accept, such as a scale of 0, puts the old value back); at rest
//! the box shows the scrub text through [`crate::scrub::shown`]. The chevrons
//! beside the value are decorative (no handler at any stretch commit):
//! nothing steps.
//!
//! The field carries its own caret and restore point: `caret` is the
//! selection as scalar indices into the draft (or into the shown value while
//! there is no draft), so Delete, the caret arrows, Home and End edit and
//! move where the browser's caret would — the box is still a text input at
//! `b8a0e1d`. `typing_from` is the value from when the field took the focus;
//! Esc puts it back. Hovering lights the row ([`crate::transform_row`]),
//! focus lives in `input.rs`, and the commit writes through the row's
//! [`crate::url::QueryParam`]. This module is the pure core: the draft slot,
//! the caret ops, the display, and the commit rule.
//!
//! Caret units (review decision, tested below): scalar indices, not UTF-16
//! units. The browser deletes one UTF-16 unit per Backspace, splitting an
//! astral character into a lone surrogate; SDL's text input only ever
//! delivers whole scalars (a lone surrogate cannot arrive), and a Rust
//! `String` cannot hold half of one — so the port deletes whole scalars. A
//! draft holding an astral character therefore differs from the browser by
//! design; number drafts never hold one through the states.

/// The text in the box while you type (`draft` in `ScrubValue.tsx`):
/// `None` at rest (the box shows the value), `Some` once typed into — kept
/// verbatim, however partial, until Enter or blur commits.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FieldState {
    /// The uncommitted text, if any.
    pub draft: Option<String>,
    /// The selection as scalar indices `[anchor, focus]` into the draft, or
    /// into the shown value while there is no draft. Collapsed (`[n, n]`)
    /// except after opening the field or select-all, which cover the text.
    pub caret: [usize; 2],
    /// The value from when the field took the focus; `Some` while typed
    /// into (`typingFrom`, also the typing tip's switch). Esc puts it back.
    pub typing_from: Option<f64>,
}

impl FieldState {
    /// A field at rest.
    #[must_use]
    pub fn new() -> Self {
        Self {
            draft: None,
            caret: [0, 0],
            typing_from: None,
        }
    }

    /// Keystrokes replace the draft whole (the DOM input owns the caret; a
    /// scripted `type` sets the text, a key appends to it — both land here).
    pub fn set_text(&mut self, text: String) {
        self.draft = Some(text);
    }

    /// Leaving the box or pressing Enter takes the draft out, committed or
    /// not (`setDraft(null)` always runs).
    pub fn take(&mut self) -> Option<String> {
        self.draft.take()
    }

    /// The text the caret indexes: the draft, else the shown value the
    /// caller passes.
    #[must_use]
    pub fn text(&self, shown: &str) -> String {
        self.draft.clone().unwrap_or_else(|| shown.to_owned())
    }

    /// The text's length in scalars.
    #[must_use]
    pub fn text_len(&self, shown: &str) -> usize {
        self.text(shown).chars().count()
    }

    /// The caret clamped into the text: edits from a stale caret (the value
    /// moved under it, as an arrow nudge does) land at the nearer end.
    #[must_use]
    pub fn clamped(&self, shown: &str) -> [usize; 2] {
        let len = self.text_len(shown);
        [self.caret[0].min(len), self.caret[1].min(len)]
    }

    /// Whether the caret covers the whole text (opening the field and
    /// select-all select; edits collapse).
    #[must_use]
    pub fn is_selected(&self, shown: &str) -> bool {
        let [a, b] = self.clamped(shown);
        a == 0 && b == self.text_len(shown)
    }
}

/// Selects the whole text, as opening the field and select-all do.
pub fn select_all(field: &mut FieldState, shown: &str) {
    let len = field.text_len(shown);
    field.caret = [0, len];
}

/// Collapses the caret to the text's end, as a click into typed text does.
/// (The browser moves the caret to the click; the port keeps it at the end,
/// where the clearing clicks land — a named simplification.)
pub fn collapse_end(field: &mut FieldState, shown: &str) {
    let len = field.text_len(shown);
    field.caret = [len, len];
}

/// Inserts text at the caret, replacing the selection (`onChange` past the
/// first keystroke, and the harness's `Text` facts).
pub fn insert_text(field: &mut FieldState, shown: &str, text: &str) {
    let base = field.text(shown);
    let [a, b] = field.clamped(shown);
    let (lo, hi) = (a.min(b), a.max(b));
    let mut chars: Vec<char> = base.chars().collect();
    let insert: Vec<char> = text.chars().collect();
    chars.splice(lo..hi, insert.iter().copied());
    let at = lo + insert.len();
    field.draft = Some(chars.into_iter().collect());
    field.caret = [at, at];
}

/// Deletes the selection, else the scalar before the caret (Backspace).
pub fn backspace(field: &mut FieldState, shown: &str) {
    let base = field.text(shown);
    let [a, b] = field.clamped(shown);
    let (lo, hi) = (a.min(b), a.max(b));
    let mut chars: Vec<char> = base.chars().collect();
    if lo == hi {
        if lo > 0 {
            chars.remove(lo - 1);
        }
        field.draft = Some(chars.into_iter().collect());
        field.caret = [lo.saturating_sub(1), lo.saturating_sub(1)];
    } else {
        chars.drain(lo..hi);
        field.draft = Some(chars.into_iter().collect());
        field.caret = [lo, lo];
    }
}

/// Deletes the selection, else the scalar after the caret (Delete).
pub fn delete_forward(field: &mut FieldState, shown: &str) {
    let base = field.text(shown);
    let [a, b] = field.clamped(shown);
    let (lo, hi) = (a.min(b), a.max(b));
    let mut chars: Vec<char> = base.chars().collect();
    if lo == hi {
        if lo < chars.len() {
            chars.remove(lo);
        }
        field.draft = Some(chars.into_iter().collect());
        field.caret = [lo, lo];
    } else {
        chars.drain(lo..hi);
        field.draft = Some(chars.into_iter().collect());
        field.caret = [lo, lo];
    }
}

/// Moves the caret by scalars, collapsing the selection (the caret arrows).
/// A positive step moves right, a negative one left.
pub fn move_caret(field: &mut FieldState, shown: &str, step: i64) {
    let len = field.text_len(shown) as i64;
    let [a, b] = field.clamped(shown);
    let at = if step < 0 { a.min(b) } else { a.max(b) };
    let at = (at as i64 + step).clamp(0, len) as usize;
    field.caret = [at, at];
}

/// Moves the caret to the start or end (Home and End).
pub fn home_end(field: &mut FieldState, shown: &str, to_end: bool) {
    let at = if to_end { field.text_len(shown) } else { 0 };
    field.caret = [at, at];
}

/// What the box shows: the draft as typed, else `value.toFixed(2)` — exact
/// through [`mv_numfmt::to_fixed`], ASCII minus as `toFixed` prints.
#[must_use]
pub fn display_value(value: f64) -> String {
    display_draft(None, value)
}

/// What the box shows for the field's full state.
#[must_use]
pub fn display_draft(draft: Option<&str>, value: f64) -> String {
    if let Some(text) = draft {
        return text.to_owned();
    }
    mv_numfmt::to_fixed(value, 2)
}

/// Enter or blur with the taken draft: the commit rule (`commit`).
/// `parse` is the row's param parser. `Some(next)` sets the value with a
/// flush; `None` keeps the old one (no draft, or text the URL wouldn't
/// accept, such as a scale of 0) — the box shows the value again either way.
#[must_use]
pub fn commit_value(draft: Option<&str>, parse: fn(Option<&str>) -> Option<f64>) -> Option<f64> {
    parse(draft)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::functions::{A_PARAM, B_PARAM, H_PARAM, K_PARAM};
    use crate::url::{QueryParam, Url};

    #[test]
    fn at_rest_each_box_shows_its_value_with_two_decimals() {
        assert_eq!(display_value(1.0), "1.00");
        assert_eq!(display_value(0.0), "0.00");
        assert_eq!(display_value(2.0), "2.00");
        assert_eq!(display_value(-1.0), "-1.00");
        assert_eq!(display_value(0.5), "0.50");
        assert_eq!(display_value(3.0), "3.00");
        assert_eq!(display_value(2.5), "2.50");
    }

    #[test]
    fn display_matches_tofixed2_around_zero() {
        // A negative value rounding to zero keeps its minus, as `toFixed` does.
        assert_eq!(display_value(-0.001), "-0.00");
        // Exact negative zero prints plain, as `(-0).toFixed(2)` does.
        assert_eq!(display_value(-0.0), "0.00");
        assert_eq!(display_value(0.001), "0.00");
    }

    #[test]
    fn typed_text_shows_as_typed_and_leaves_the_value_alone() {
        // The row test: typing 2.5 shows "2.5" while the param stays 1.
        let mut field = FieldState::new();
        let url = Url::from_query("");
        let param = QueryParam::new(&A_PARAM, "");
        field.set_text("2.5".to_owned());
        assert_eq!(display_draft(field.draft.as_deref(), *param.get()), "2.5");
        assert_eq!(*param.get(), 1.0);
        assert_eq!(url.query(), "");
    }

    #[test]
    fn enter_sets_the_value_and_shows_two_decimals() {
        // The row test: Enter writes a=2.5 and shows "2.50".
        let mut field = FieldState::new();
        let mut url = Url::from_query("");
        let mut param = QueryParam::new(&A_PARAM, "");
        field.set_text("2.5".to_owned());
        let draft = field.take();
        if let Some(next) = commit_value(draft.as_deref(), A_PARAM.parse) {
            param.set_flushed(&mut url, next);
        }
        assert_eq!(field.draft, None);
        assert_eq!(url.query(), "a=2.5");
        assert_eq!(display_draft(field.draft.as_deref(), *param.get()), "2.50");
    }

    #[test]
    fn a_scale_of_zero_puts_the_old_value_back() {
        // The row test: a link with a=2 shows "2.00"; typing 0 and leaving
        // the box brings back "2.00", and the URL keeps a=2.
        let mut field = FieldState::new();
        let mut url = Url::from_query("a=2");
        let mut param = QueryParam::new(&A_PARAM, "a=2");
        assert_eq!(display_draft(field.draft.as_deref(), *param.get()), "2.00");
        field.set_text("0".to_owned());
        assert_eq!(display_draft(field.draft.as_deref(), *param.get()), "0");
        let draft = field.take();
        if let Some(next) = commit_value(draft.as_deref(), A_PARAM.parse) {
            param.set_flushed(&mut url, next);
        }
        assert_eq!(display_draft(field.draft.as_deref(), *param.get()), "2.00");
        assert_eq!(url.query(), "a=2");
    }

    #[test]
    fn commit_keeps_the_old_value_without_a_draft_or_a_parse() {
        assert_eq!(commit_value(None, A_PARAM.parse), None);
        assert_eq!(commit_value(Some("0"), A_PARAM.parse), None);
        assert_eq!(commit_value(Some("x"), A_PARAM.parse), None);
        assert_eq!(commit_value(Some("0"), H_PARAM.parse), Some(0.0));
        assert_eq!(commit_value(Some(""), K_PARAM.parse), Some(0.0));
        assert_eq!(commit_value(Some(""), B_PARAM.parse), None);
    }

    // Opening the field selects the whole shown text.
    #[test]
    fn opening_selects_all() {
        let mut field = FieldState::new();
        select_all(&mut field, "1.00");
        assert_eq!(field.caret, [0, 4]);
        assert!(field.is_selected("1.00"));
        assert_eq!(field.draft, None);
    }

    // Typing over the selection replaces it and collapses the caret.
    #[test]
    fn typing_replaces_the_selection() {
        let mut field = FieldState::new();
        select_all(&mut field, "1.00");
        insert_text(&mut field, "1.00", "2.5");
        assert_eq!(field.draft.as_deref(), Some("2.5"));
        assert_eq!(field.caret, [3, 3]);
        assert!(!field.is_selected("2.5"));
    }

    // Typing mid-text inserts at the caret.
    #[test]
    fn typing_inserts_at_a_collapsed_caret() {
        let mut field = FieldState::new();
        field.set_text("25".to_owned());
        field.caret = [1, 1];
        insert_text(&mut field, "25", ".");
        assert_eq!(field.draft.as_deref(), Some("2.5"));
        assert_eq!(field.caret, [2, 2]);
    }

    // Backspace deletes the selection, else the scalar before the caret.
    #[test]
    fn backspace_deletes_selection_then_scalars() {
        let mut field = FieldState::new();
        select_all(&mut field, "1.00");
        backspace(&mut field, "1.00");
        assert_eq!(field.draft.as_deref(), Some(""));
        assert_eq!(field.caret, [0, 0]);

        field.set_text("2.5".to_owned());
        field.caret = [3, 3];
        backspace(&mut field, "2.5");
        assert_eq!(field.draft.as_deref(), Some("2."));
        assert_eq!(field.caret, [2, 2]);

        // At the start nothing happens, and the draft still clears the way
        // edits do (the box repaints from the draft).
        backspace(&mut field, "2.");
        backspace(&mut field, "2.");
        backspace(&mut field, "2.");
        assert_eq!(field.draft.as_deref(), Some(""));
    }

    // Backspace pops a Unicode scalar where the browser deletes one UTF-16
    // unit (the named decision): an astral character goes whole, where the
    // browser would leave its lead surrogate behind.
    #[test]
    fn backspace_pops_a_whole_scalar() {
        let mut field = FieldState::new();
        field.set_text("1😀".to_owned());
        field.caret = [2, 2];
        backspace(&mut field, "1😀");
        assert_eq!(field.draft.as_deref(), Some("1"));
        assert_eq!(field.caret, [1, 1]);
    }

    // Delete mirrors forward, and the caret arrows walk scalars.
    #[test]
    fn delete_and_arrows_walk_scalars() {
        let mut field = FieldState::new();
        field.set_text("2.5".to_owned());
        field.caret = [0, 0];
        delete_forward(&mut field, "2.5");
        assert_eq!(field.draft.as_deref(), Some(".5"));
        assert_eq!(field.caret, [0, 0]);

        move_caret(&mut field, ".5", 1);
        move_caret(&mut field, ".5", 1);
        assert_eq!(field.caret, [2, 2]);
        move_caret(&mut field, ".5", -1);
        assert_eq!(field.caret, [1, 1]);
        // Past either end clamps.
        move_caret(&mut field, ".5", -5);
        assert_eq!(field.caret, [0, 0]);
        move_caret(&mut field, ".5", 9);
        assert_eq!(field.caret, [2, 2]);

        home_end(&mut field, ".5", false);
        assert_eq!(field.caret, [0, 0]);
        home_end(&mut field, ".5", true);
        assert_eq!(field.caret, [2, 2]);
    }

    // A stale caret clamps into the shorter text.
    #[test]
    fn stale_carets_clamp() {
        let mut field = FieldState::new();
        field.set_text("2.61".to_owned());
        field.caret = [9, 9];
        insert_text(&mut field, "2.61", "0");
        assert_eq!(field.draft.as_deref(), Some("2.610"));
        collapse_end(&mut field, "2.610");
        assert_eq!(field.caret, [5, 5]);
    }
}
