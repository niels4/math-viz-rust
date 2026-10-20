//! A transform row's typed number field.
//!
//! Ported from `SideBar/TransformRow.tsx` at `5714319`: the text in the box
//! while you type (`draft: string | null` — partial numbers such as `"-"` or
//! `"1."` stay here), shown as typed; Enter or leaving the box commits
//! (`commit` — the parse decides, text the URL wouldn't accept, such as a
//! scale of 0, puts the old value back); at rest the box shows
//! `value.toFixed(2)`. The chevrons beside the value are decorative (no
//! handler at any stretch commit): nothing steps.
//!
//! The field holds no pointer or focus state: hovering lights the row
//! ([`crate::transform_row`]), focus lives in `input.rs`, and the commit
//! writes through the row's [`crate::url::QueryParam`] in phase 2. This
//! module is the pure core: the draft slot, the display, and the commit rule.

/// The text in the box while you type (`draft` in `TransformRow.tsx`):
/// `None` at rest (the box shows the value), `Some` once typed into — kept
/// verbatim, however partial, until Enter or blur commits. `selected` is the
/// browser's select-all: the next typed text replaces the box's text.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FieldState {
    /// The uncommitted text, if any.
    pub draft: Option<String>,
    /// The box's text stands selected (Ctrl/Command+A).
    pub selected: bool,
}

impl FieldState {
    /// A field at rest.
    #[must_use]
    pub fn new() -> Self {
        Self {
            draft: None,
            selected: false,
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
}

/// What the box shows: the draft as typed, else `value.toFixed(2)` — exact
/// through [`mv_numfmt::to_fixed`], ASCII minus as `toFixed` prints.
#[must_use]
pub fn display_value(value: f64) -> String {
    display_draft(&None, value)
}

/// What the box shows for the field's full state.
#[must_use]
pub fn display_draft(draft: &Option<String>, value: f64) -> String {
    if let Some(text) = draft {
        return text.clone();
    }
    mv_numfmt::to_fixed(value, 2)
}

/// Enter or blur with the taken draft: the commit rule (`commit`).
/// `parse` is the row's param parser. `Some(next)` sets the value with a
/// flush; `None` keeps the old one (no draft, or text the URL wouldn't
/// accept, such as a scale of 0) — the box shows the value again either way.
#[must_use]
pub fn commit_value(draft: &Option<String>, parse: fn(Option<&str>) -> Option<f64>) -> Option<f64> {
    parse(draft.as_deref())
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
        assert_eq!(display_draft(&field.draft, *param.get()), "2.5");
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
        if let Some(next) = commit_value(&draft, A_PARAM.parse) {
            param.set_flushed(&mut url, next);
        }
        assert_eq!(field.draft, None);
        assert_eq!(url.query(), "a=2.5");
        assert_eq!(display_draft(&field.draft, *param.get()), "2.50");
    }

    #[test]
    fn a_scale_of_zero_puts_the_old_value_back() {
        // The row test: a link with a=2 shows "2.00"; typing 0 and leaving
        // the box brings back "2.00", and the URL keeps a=2.
        let mut field = FieldState::new();
        let mut url = Url::from_query("a=2");
        let mut param = QueryParam::new(&A_PARAM, "a=2");
        assert_eq!(display_draft(&field.draft, *param.get()), "2.00");
        field.set_text("0".to_owned());
        assert_eq!(display_draft(&field.draft, *param.get()), "0");
        let draft = field.take();
        if let Some(next) = commit_value(&draft, A_PARAM.parse) {
            param.set_flushed(&mut url, next);
        }
        assert_eq!(display_draft(&field.draft, *param.get()), "2.00");
        assert_eq!(url.query(), "a=2");
    }

    #[test]
    fn commit_keeps_the_old_value_without_a_draft_or_a_parse() {
        assert_eq!(commit_value(&None, A_PARAM.parse), None);
        assert_eq!(commit_value(&Some("0".to_owned()), A_PARAM.parse), None);
        assert_eq!(commit_value(&Some("x".to_owned()), A_PARAM.parse), None);
        assert_eq!(
            commit_value(&Some("0".to_owned()), H_PARAM.parse),
            Some(0.0)
        );
        assert_eq!(commit_value(&Some("".to_owned()), K_PARAM.parse), Some(0.0));
        assert_eq!(commit_value(&Some("".to_owned()), B_PARAM.parse), None);
    }
}
