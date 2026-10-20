//! P's x field: the number in the readout's x slot.
//!
//! Ported from `SideBar/PointsSection/XField.tsx` and `XField.module.css`
//! at `96f1e84`. At rest the field is read-only, showing
//! [`format_number`](crate::point_format::format_number) of `p`; a
//! mouse-down, a focus, or Enter from the x-of-P slider opens the edit (the
//! text selected, the edit's base = `p`), and typing replaces the text. The
//! width follows the characters (at least one, 5 px padding while editing,
//! the card background with a 2 px primary inset ring; the destructive
//! colour and ring when the text doesn't parse). Enter commits
//! [`read`] (null keeps the edit open in the error tone), Escape puts the
//! base back, ArrowUp/Down nudge 0.01 from the parsed text (or `p`) and
//! write `p` live (debounced) with the text reformatted, and blur commits
//! the parsed text or the base. A commit or a cancel returns the focus to
//! the slider; a blur doesn't. The edit (`invalid`, `base`) is reported to
//! the row for the tip.
//!
//! This module is the pure state machine: every handler returns the
//! [`XEffect`]s the input layer performs (`p`'s debounced or flushed write,
//! the focus move), and the vitest typing tests stand against it directly.
//! The caret reuses [`crate::number_field`]'s scalar-index model (clicks
//! while editing collapse to the end, as the scrub box's do — a named
//! simplification of the browser moving the caret to the click). Phase 2
//! wires the App half (the `Focus` effect, SDL text input while editing,
//! the layout and paint, the slider's Enter opening the field and the
//! commit/cancel focus back) after h8-points-row and h8-points-slider
//! merge: it reads `p` through h8-p-points's `P_PARAM`, and the row reads
//! [`XField::edit`] for the tip.

use crate::number_field::{
    FieldState, backspace, collapse_end, delete_forward, home_end, insert_text, move_caret,
    select_all,
};

/// The live nudge per arrow key (`NUDGE`).
pub const NUDGE: f64 = 0.01;

/// The field's height (24 px, on a 24 px line).
pub const FIELD_HEIGHT: f32 = 24.0;
/// The field's text: mono 500, 18 px on a 24 px line, tabular numbers.
pub const FIELD_FONT_SIZE: f32 = 18.0;
/// The field's line height.
pub const FIELD_LINE_HEIGHT: f32 = 24.0;
/// The field's corner radius (6 px).
pub const FIELD_RADIUS: f32 = 6.0;
/// The editing padding each side (5 px; none at rest).
pub const FIELD_PAD_EDITING: f32 = 5.0;
/// The editing inset ring (2 px, primary; destructive when invalid).
pub const FIELD_RING: f32 = 2.0;

/// The edit reported to the row for the tip (`FieldEdit`): whether the text
/// parses, and the value the edit started from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FieldEdit {
    /// The text doesn't parse (`aria-invalid`, the error tone).
    pub invalid: bool,
    /// The `p` the edit opened from (what Escape puts back, and what the
    /// error tip names).
    pub base: f64,
}

/// How the edit ended (`FieldEnd`): a commit or a cancel returns the focus
/// to the slider, a blur doesn't.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldEnd {
    Commit,
    Cancel,
    Blur,
}

/// What a handler asks the input layer to do, in order.
#[derive(Clone, Debug, PartialEq)]
pub enum XEffect {
    /// Write `p` live (debounced): an arrow nudge.
    Value(f64),
    /// Write `p` and flush at once: Enter, Escape, blur.
    ValueFlush(f64),
    /// The edit ended; a commit or a cancel moves the focus back to the
    /// slider.
    End(FieldEnd),
    /// Focus the field with its text selected (opening the edit).
    Focus,
}

/// A caret key the read-only rest ignores and the open edit handles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum XKey {
    Enter,
    Escape,
    ArrowUp,
    ArrowDown,
    Left,
    Right,
    Home,
    End,
    Backspace,
    Delete,
}

/// P's x field: the edit (`text` + `base` + `invalid`) over `p`.
///
/// The text and caret ride [`FieldState`]: `draft` is `Some` while editing
/// (the text as typed, however partial), `None` at rest (the field shows
/// the formatted value). `base` is `Some` exactly while editing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct XField {
    field: FieldState,
    base: Option<f64>,
    invalid: bool,
}

impl XField {
    /// A field at rest showing the value.
    #[must_use]
    pub fn new() -> Self {
        Self {
            field: FieldState::new(),
            base: None,
            invalid: false,
        }
    }

    /// Whether the edit is open (`data-editing`).
    #[must_use]
    pub fn editing(&self) -> bool {
        self.base.is_some()
    }

    /// Whether the open text parses (else `aria-invalid` and the error tip).
    #[must_use]
    pub fn invalid(&self) -> bool {
        self.invalid
    }

    /// The edit for the row's tip: `None` at rest.
    #[must_use]
    pub fn edit(&self) -> Option<FieldEdit> {
        self.base.map(|base| FieldEdit {
            invalid: self.invalid,
            base,
        })
    }

    /// What the field shows: the text as typed, else the formatted value.
    #[must_use]
    pub fn display(&self, value: f64) -> String {
        match self.field.draft.as_deref() {
            Some(text) => text.to_owned(),
            None => crate::point_format::format_number(value),
        }
    }

    /// The `--chars` width: the text's length, at least 1.
    #[must_use]
    pub fn chars(&self, value: f64) -> usize {
        // JS `text.length` counts UTF-16 units; scalars agree on every text
        // the field shows or accepts (digits and one true minus sign — the
        // number field's named decision).
        self.display(value).chars().count().max(1)
    }

    /// Opens the edit on `value` (`open`): the text selected, the base = the
    /// value. A no-op while open (focus lands on an already-open field).
    pub fn open(&mut self, value: f64) -> Vec<XEffect> {
        if self.editing() {
            return Vec::new();
        }
        let shown = crate::point_format::format_number(value);
        self.field.set_text(shown.clone());
        select_all(&mut self.field, &shown);
        self.base = Some(value);
        self.invalid = false;
        vec![XEffect::Focus]
    }

    /// The mouse went down on the field (`onMouseDown`): a closed field
    /// opens without the browser's default (which would move the focus
    /// first); an open one keeps its caret.
    pub fn mouse_down(&mut self, value: f64) -> Vec<XEffect> {
        self.open(value)
    }

    /// The field took the focus (`onFocus`): a closed field opens.
    pub fn focus_in(&mut self, value: f64) -> Vec<XEffect> {
        self.open(value)
    }

    /// A key (`onKeyDown`). Rest answers only Enter (which opens); the edit
    /// answers Enter (commit), Escape (cancel), ArrowUp/Down (live nudge)
    /// and the caret keys.
    pub fn key_down(&mut self, key: XKey, value: f64) -> Vec<XEffect> {
        if !self.editing() {
            if key == XKey::Enter {
                return self.open(value);
            }
            return Vec::new();
        }
        let text = self.text();
        match key {
            XKey::Enter => match read(&text) {
                None => {
                    self.invalid = true;
                    Vec::new()
                }
                Some(next) => self.finish(XEffect::ValueFlush(next), FieldEnd::Commit),
            },
            XKey::Escape => {
                let base = self.base.unwrap_or(value);
                self.finish(XEffect::ValueFlush(base), FieldEnd::Cancel)
            }
            XKey::ArrowUp | XKey::ArrowDown => {
                let delta = if key == XKey::ArrowUp { NUDGE } else { -NUDGE };
                let next = rounded(read(&text).unwrap_or(value) + delta);
                let shown = crate::point_format::format_number(next);
                self.field.set_text(shown);
                self.invalid = false;
                vec![XEffect::Value(next)]
            }
            XKey::Left => {
                move_caret(&mut self.field, &text, -1);
                Vec::new()
            }
            XKey::Right => {
                move_caret(&mut self.field, &text, 1);
                Vec::new()
            }
            XKey::Home => {
                home_end(&mut self.field, &text, false);
                Vec::new()
            }
            XKey::End => {
                home_end(&mut self.field, &text, true);
                Vec::new()
            }
            XKey::Backspace => {
                backspace(&mut self.field, &text);
                Vec::new()
            }
            XKey::Delete => {
                delete_forward(&mut self.field, &text);
                Vec::new()
            }
        }
    }

    /// Typed text (`onChange` past the first keystroke, and the harness's
    /// text facts): the text as typed, valid again until Enter says
    /// otherwise.
    pub fn type_text(&mut self, value: f64, text: &str) {
        if !self.editing() {
            return;
        }
        let shown = self.display(value);
        insert_text(&mut self.field, &shown, text);
        self.invalid = false;
    }

    /// The field lost the focus (`onBlur`): the parsed text commits, else
    /// the base does, flushed; the focus stays where it went.
    pub fn blur_out(&mut self, value: f64) -> Vec<XEffect> {
        if !self.editing() {
            return Vec::new();
        }
        let text = self.text();
        let next = read(&text).unwrap_or(self.base.unwrap_or(value));
        self.finish(XEffect::ValueFlush(next), FieldEnd::Blur)
    }

    /// The text the caret indexes: the draft while editing.
    fn text(&self) -> String {
        self.field.draft.clone().unwrap_or_default()
    }

    /// Ends the edit, clearing it and reporting how (the `update(null)` +
    /// `onEditEnd` pair, in that order).
    fn finish(&mut self, effect: XEffect, end: FieldEnd) -> Vec<XEffect> {
        self.field.take();
        collapse_end(&mut self.field, "");
        self.base = None;
        self.invalid = false;
        vec![effect, XEffect::End(end)]
    }
}

/// `rounded`: `Number(n.toFixed(3)) || 0` — three decimals, exact through
/// [`mv_numfmt::to_fixed`], with falsy results (`0`, `-0`, `NaN`) as `0.0`.
#[must_use]
pub fn rounded(n: f64) -> f64 {
    let parsed: f64 = mv_numfmt::to_fixed(n, 3).parse().unwrap_or(f64::NAN);
    if parsed == 0.0 || parsed.is_nan() {
        0.0
    } else {
        parsed
    }
}

/// `read`: [`parse_number`](crate::point_format::parse_number), rounded to
/// three decimals — `None` when the text doesn't parse.
#[must_use]
pub fn read(text: &str) -> Option<f64> {
    crate::point_format::parse_number(text).map(rounded)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open() -> XField {
        let mut field = XField::new();
        assert_eq!(field.key_down(XKey::Enter, 2.0), vec![XEffect::Focus]);
        assert_eq!(field.display(2.0), "2.00");
        field
    }

    // `rounded` keeps three decimals, exact, with falsy results as 0.
    #[test]
    fn rounded_keeps_three_decimals() {
        assert_eq!(rounded(3.256), 3.256);
        assert_eq!(rounded(2.0004), 2.0);
        assert_eq!(rounded(1.23456), 1.235);
        assert_eq!(rounded(0.0), 0.0);
        // `Number("-0.000") || 0`: a negative zero comes back plain.
        assert!(!rounded(-0.0001).is_sign_negative());
        assert_eq!(rounded(-0.0001), 0.0);
    }

    // At rest the field shows the formatted value, read-only, with no edit
    // for the tip; only Enter opens it.
    #[test]
    fn at_rest_the_field_shows_the_value() {
        let mut field = XField::new();
        assert!(!field.editing());
        assert_eq!(field.edit(), None);
        assert_eq!(field.display(2.0), "2.00");
        assert_eq!(field.display(-1.5), "\u{2212}1.50");
        assert_eq!(field.chars(2.0), 4);
        assert_eq!(field.key_down(XKey::ArrowUp, 2.0), Vec::new());
        assert_eq!(field.key_down(XKey::Escape, 2.0), Vec::new());
        assert_eq!(field.blur_out(2.0), Vec::new());
    }

    // Opening selects the whole text, from any of the three paths.
    #[test]
    fn opening_selects_the_text() {
        let mut by_enter = XField::new();
        assert_eq!(by_enter.key_down(XKey::Enter, 2.0), vec![XEffect::Focus]);
        assert!(by_enter.editing());

        let mut by_mouse = XField::new();
        assert_eq!(by_mouse.mouse_down(2.0), vec![XEffect::Focus]);

        let mut by_focus = XField::new();
        assert_eq!(by_focus.focus_in(2.0), vec![XEffect::Focus]);

        // A second open lands on the already-open field.
        assert_eq!(by_focus.focus_in(2.0), Vec::new());
        assert_eq!(by_focus.mouse_down(2.0), Vec::new());
    }

    // "opens from Enter on the slider, applies on Enter and hands the focus
    // back".
    #[test]
    fn enter_opens_applies_and_hands_the_focus_back() {
        let mut field = open();
        assert_eq!(
            field.edit(),
            Some(FieldEdit {
                invalid: false,
                base: 2.0
            })
        );

        field.type_text(2.0, "\u{2212}1.5");
        assert_eq!(field.display(2.0), "\u{2212}1.5");
        assert_eq!(
            field.key_down(XKey::Enter, 2.0),
            vec![XEffect::ValueFlush(-1.5), XEffect::End(FieldEnd::Commit)]
        );
        assert!(!field.editing());
        assert_eq!(field.edit(), None);
        assert_eq!(field.display(-1.5), "\u{2212}1.50");
    }

    // "keeps refused text open, and puts the value back on Esc".
    #[test]
    fn refused_text_stays_open_until_esc_puts_the_base_back() {
        let mut field = open();

        field.type_text(2.0, "abc");
        assert_eq!(field.key_down(XKey::Enter, 2.0), Vec::<XEffect>::new());
        assert!(field.editing());
        assert!(field.invalid());
        assert_eq!(
            field.edit(),
            Some(FieldEdit {
                invalid: true,
                base: 2.0
            })
        );

        // Typing over the selected text replaces it and clears the error
        // (the vitest `type` helper sets the whole value, as a select-all
        // followed by keys does).
        crate::number_field::select_all(&mut field.field, "abc");
        field.type_text(2.0, "3");
        assert_eq!(field.display(2.0), "3");
        assert!(!field.invalid());

        assert_eq!(
            field.key_down(XKey::Escape, 2.0),
            vec![XEffect::ValueFlush(2.0), XEffect::End(FieldEnd::Cancel)]
        );
        assert!(!field.editing());
        assert_eq!(field.display(2.0), "2.00");
    }

    // "nudges 0.01 with the arrow keys at once, and Esc undoes the nudges".
    #[test]
    fn arrows_nudge_live_and_esc_undoes_the_nudges() {
        let mut field = open();

        assert_eq!(
            field.key_down(XKey::ArrowUp, 2.0),
            vec![XEffect::Value(2.01)]
        );
        assert_eq!(field.display(2.01), "2.01");
        assert_eq!(
            field.key_down(XKey::ArrowUp, 2.01),
            vec![XEffect::Value(2.02)]
        );
        assert_eq!(field.display(2.02), "2.02");
        assert_eq!(
            field.key_down(XKey::ArrowDown, 2.02),
            vec![XEffect::Value(2.01)]
        );
        assert_eq!(field.display(2.01), "2.01");

        assert_eq!(
            field.key_down(XKey::Escape, 2.01),
            vec![XEffect::ValueFlush(2.0), XEffect::End(FieldEnd::Cancel)]
        );
        assert_eq!(field.display(2.0), "2.00");
    }

    // "applies a number on leaving, and puts the value back when the text
    // isn't one".
    #[test]
    fn blur_applies_a_number_and_restores_the_base_for_garbage() {
        let mut field = open();

        field.type_text(2.0, "3.256");
        assert_eq!(
            field.blur_out(2.0),
            vec![XEffect::ValueFlush(3.256), XEffect::End(FieldEnd::Blur)]
        );
        assert_eq!(field.display(3.256), "3.26");

        assert_eq!(field.focus_in(3.256), vec![XEffect::Focus]);
        field.type_text(3.256, "1..2");
        assert_eq!(
            field.blur_out(3.256),
            vec![XEffect::ValueFlush(3.256), XEffect::End(FieldEnd::Blur)]
        );
        assert_eq!(field.display(3.256), "3.26");
    }

    // Arrows nudge from the parsed text when it parses, else from the value.
    #[test]
    fn arrows_nudge_from_the_text_or_the_value() {
        let mut field = open();
        field.type_text(2.0, "1.5");
        assert_eq!(
            field.key_down(XKey::ArrowUp, 2.0),
            vec![XEffect::Value(1.51)]
        );
        assert_eq!(field.display(1.51), "1.51");

        let mut garbage = open();
        garbage.type_text(2.0, "abc");
        assert_eq!(
            garbage.key_down(XKey::ArrowUp, 2.0),
            vec![XEffect::Value(2.01)]
        );
        assert_eq!(garbage.display(2.01), "2.01");
        assert!(!garbage.invalid());
    }

    // Clearing the field takes Backspaces from the selection, as the
    // scripts clear it (a click at the right edge, then Backspace per
    // character).
    #[test]
    fn backspace_clears_the_selected_text() {
        let mut field = open();
        assert_eq!(field.key_down(XKey::Backspace, 2.0), Vec::new());
        assert_eq!(field.display(2.0), "");
        assert_eq!(field.chars(2.0), 1);
        assert_eq!(field.key_down(XKey::Backspace, 2.0), Vec::new());
        assert_eq!(field.key_down(XKey::Enter, 2.0), Vec::<XEffect>::new());
        assert!(field.invalid());
    }

    // Caret keys walk the text while editing, and rest ignores them.
    #[test]
    fn caret_keys_walk_the_open_text() {
        let mut field = open();
        assert_eq!(field.key_down(XKey::Left, 2.0), Vec::new());
        assert_eq!(field.key_down(XKey::Right, 2.0), Vec::new());
        assert_eq!(field.key_down(XKey::Home, 2.0), Vec::new());
        assert_eq!(field.key_down(XKey::End, 2.0), Vec::new());
        assert_eq!(field.key_down(XKey::Delete, 2.0), Vec::new());
        assert_eq!(field.display(2.0), "2.00");
    }
}
