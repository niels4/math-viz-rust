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

use crate::app::App;
use crate::app_loop::MS;
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

    /// Whether the caret covers the whole shown text (the browser's
    /// selection paint condition, with the focus).
    #[must_use]
    pub fn is_selected(&self, shown: &str) -> bool {
        self.field.is_selected(shown)
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

// ---------------------------------------------------------------------------
// The app half: the card's slot, tip, paint and input.
// ---------------------------------------------------------------------------

/// P's field hit (never a Tab stop: `tabIndex={-1}`).
#[must_use]
pub fn field_hit() -> crate::input::Hit {
    crate::input::Hit {
        kind: crate::input::HitKind::PointsField,
        index: 0,
    }
}

/// P's live value.
#[must_use]
pub fn p_value(app: &App) -> f64 {
    *app.p_param.get()
}

/// What the field shows: the edit text, else the formatted p.
#[must_use]
pub fn display(app: &App) -> String {
    app.x_field.display(p_value(app))
}

/// The display text with its `--chars` count, for the readout and the slot.
#[must_use]
pub fn display_chars(app: &App) -> (String, usize) {
    let text = display(app);
    let chars = app.x_field.chars(p_value(app));
    (text, chars)
}

/// The field's horizontal padding: 5 px while editing, none at rest.
fn pad(app: &App) -> f32 {
    if app.x_field.editing() {
        FIELD_PAD_EDITING
    } else {
        0.0
    }
}

/// The field's box width: the text's chars at the `1ch` advance with the
/// editing padding (`width: calc(var(--chars) * 1ch + 2 * var(--pad))`).
fn field_width(app: &mut App) -> (f32, f32) {
    let chars = app.x_field.chars(p_value(app));
    let ch = crate::points_section::ch_advance(app);
    (chars as f32 * ch + 2.0 * pad(app), ch)
}

/// The x slot's scope: the field's box, right-aligned in the x run, holding
/// the hit target. A floating child of the readout scope (out of flow, like
/// the tip), attached to its top-left. Called from the P readout's layout
/// ([`X_SLOT_ID`] stays the paint and hit anchor).
pub fn slot(app: &mut App, readout: mv_sys::clay::ElementId) {
    // The slider's Enter asks for the field (h8-points-slider sets
    // `App::slider_edit`): open on set and clear the flag, in the same
    // frame the track's key turn drew.
    if app.slider_edit {
        app.slider_edit = false;
        open_field(app, true);
    }
    let (text, n) = display_chars(app);
    let (x_slot, _) = crate::points_section::x_slot(app, &text, n, true);
    let (field_w, _) = field_width(app);
    let open = crate::points_section::ceil_px(
        crate::points_section::OPEN_EM * crate::points_section::MATH_SIZE,
    );
    let mut decl = crate::ui::empty_decl();
    decl.layout.sizing = mv_sys::clay::Sizing {
        width: mv_clay::sizing_fixed(field_w),
        height: mv_clay::sizing_fixed(FIELD_HEIGHT),
    };
    decl.floating = mv_sys::clay::FloatingElementConfig {
        offset: [
            open + x_slot - field_w,
            (crate::points_section::READOUT_HEIGHT - FIELD_HEIGHT) / 2.0,
        ],
        expand: mv_sys::clay::Dimensions {
            width: 0.0,
            height: 0.0,
        },
        parentId: readout.id,
        zIndex: 0,
        attachment: mv_sys::clay::FloatingAttachPoints {
            element: mv_sys::clay::FloatingAttachPointType::LeftTop,
            parent: mv_sys::clay::FloatingAttachPointType::LeftTop,
        },
        pointerCaptureMode: mv_sys::clay::PointerCaptureMode::Capture,
        attachTo: mv_sys::clay::FloatingAttachToElement::ElementWithId,
        clipTo: mv_sys::clay::FloatingClipToElement::None,
    };
    crate::ui::ui_scope(
        app,
        mv_clay::id(crate::points_section::X_SLOT_ID, 0),
        &decl,
        |_| {},
    );
    crate::input::region(
        app,
        mv_clay::id(crate::points_section::X_SLOT_ID, 0),
        field_hit(),
        crate::input::Cursor::Text,
        crate::input::Cursor::Text,
        false,
        crate::ui::corner_radius_all(FIELD_RADIUS),
    );
}

/// The tip's shared element ids.
fn tip_ids() -> crate::tip::TipIds {
    crate::tip::TipIds {
        tip: P_TIP_ID,
        text: P_TIP_TEXT_ID,
        key: P_TIP_KEY_ID,
        arrow: P_TIP_ARROW_ID,
    }
}

/// The P tip's container id, for the page's tip layer (the plate's walk).
#[must_use]
pub fn tip_id() -> u32 {
    mv_clay::id(P_TIP_ID, 0).id
}

/// The P tip's element ids (h8-points-field's namespace).
pub const P_TIP_ID: &str = "PFieldTip";
/// A P tip word or key label.
pub const P_TIP_TEXT_ID: &str = "PFieldTipText";
/// A P tip key cap plate.
pub const P_TIP_KEY_ID: &str = "PFieldTipKey";
/// A P tip arrow glyph box.
pub const P_TIP_ARROW_ID: &str = "PFieldTipArrow";

/// The P edit tip's cells: the key help, or the error tone naming the base
/// (`PRow.tsx` lines 94 to 112).
fn tip_cells(edit: &FieldEdit) -> (crate::tip::TipTone, Vec<crate::tip::TipCell>) {
    use crate::tip::{ArrowDir, TipCell, TipTone};
    if edit.invalid {
        (
            TipTone::Error,
            vec![
                TipCell::Words("Type a number such as 1.5 \u{00b7}".to_owned()),
                TipCell::Key("Esc".to_owned()),
                TipCell::Words(format!(
                    "puts back {}",
                    crate::point_format::format_number(edit.base)
                )),
            ],
        )
    } else {
        (
            TipTone::Default,
            vec![
                TipCell::Key("Enter".to_owned()),
                TipCell::Words("apply \u{00b7}".to_owned()),
                TipCell::Arrow(ArrowDir::Up),
                TipCell::Arrow(ArrowDir::Down),
                TipCell::Words("nudge 0.01".to_owned()),
            ],
        )
    }
}

/// Lays out the P edit tip under the head while the field edits (the head
/// is the `.point` block's top 30 px, so top 34 px is 4 px below it, right
/// flush, `z-index: 3`). Called at the end of the point block.
pub fn edit_tip(app: &mut App) {
    let Some(edit) = app.x_field.edit() else {
        return;
    };
    let (tone, cells) = tip_cells(&edit);
    crate::tip::layout_tip(
        app,
        &tip_ids(),
        0,
        0,
        &crate::tip::TipPlace {
            parent: mv_clay::id(crate::points_section::P_HEAD_ID, 0).id,
            drop: P_TIP_DROP,
            z: 3,
        },
        tone,
        &cells,
    );
}

/// The P tip's drop below the head: top 34 px over the 30 px head.
pub const P_TIP_DROP: f32 = 4.0;

/// Paints the P tip's arrow glyphs after the tip layer walks (paths, not
/// text, as the scrub tip's). Call from `draw_page` after the tip layer.
pub fn paint_tip_arrows(app: &mut App) {
    let Some(edit) = app.x_field.edit() else {
        return;
    };
    let (_, cells) = tip_cells(&edit);
    crate::tip::paint_arrows(app, &tip_ids(), 0, &cells);
}

/// The number style: 500 18 px mono, tabular, right-aligned (`.number`).
fn number_style() -> mv_text::Style {
    let mut features = mv_text::Features::new();
    features.insert(mv_text::Feature::Tabular);
    mv_text::Style {
        face: mv_text::Face::RobotoMono500,
        size: FIELD_FONT_SIZE,
        tracking: 0.0,
        features,
    }
}

/// Paints the x run's field: the text right-aligned in the slot, with the
/// card background and the primary inset ring while editing (destructive
/// when invalid), the scheme selection over a fully selected text while
/// focused, and destructive text when invalid. `readout_x` is the readout
/// scope's x, `open`/`x_slot` its lead geometry, `number_base` the number
/// line's baseline, `ink` the run's ink. Called from the P readout's paint.
pub fn paint_field(
    app: &mut App,
    theme: &mv_theme::Theme,
    readout_x: f32,
    open: f32,
    x_slot: f32,
    number_base: f32,
    ink: mv_sys::clay::Color,
) {
    let data = app
        .clay
        .as_ref()
        .expect("clay context initialised")
        .get_element_data(mv_clay::id(crate::points_section::X_SLOT_ID, 0));
    if !data.found {
        return;
    };
    let fbox = data.boundingBox;
    let display = display(app);
    let editing = app.x_field.editing();
    let invalid = app.x_field.invalid();
    let density = if app.size[0] > 0.0 {
        app.pixels[0] as f32 / app.size[0]
    } else {
        1.0
    };
    if editing {
        let frame = crate::draw::snap_box(fbox, density);
        app.vg.begin_path();
        crate::draw::rounded_rect(
            &mut app.vg,
            frame,
            crate::ui::corner_radius_all(FIELD_RADIUS),
        );
        app.vg.fill_color(crate::draw::nvg_color(crate::ui::ink(
            theme,
            mv_theme::Token::Card,
        )));
        app.vg.fill();
        let ring = crate::draw::nvg_color(crate::ui::ink(
            theme,
            if invalid {
                mv_theme::Token::Destructive
            } else {
                mv_theme::Token::Primary
            },
        ));
        app.vg.begin_path();
        crate::draw::rounded_rect(
            &mut app.vg,
            mv_sys::clay::BoundingBox {
                x: frame.x + FIELD_RING / 2.0,
                y: frame.y + FIELD_RING / 2.0,
                width: frame.width - FIELD_RING,
                height: frame.height - FIELD_RING,
            },
            crate::draw::inset_radius(crate::ui::corner_radius_all(FIELD_RADIUS), FIELD_RING / 2.0),
        );
        app.vg.stroke_color(ring);
        app.vg.stroke_width(FIELD_RING);
        app.vg.stroke();
    }
    // The selection shows only while focused, over the fully selected text,
    // as the scrub box's does: the plate covers the text area (the field
    // minus its editing padding) less 2 px top and bottom.
    let selected = app.input.focus == field_hit() && app.x_field.is_selected(&display);
    if selected {
        let dark = matches!(
            app.theme.resolved_scheme(),
            crate::theme_state::SystemScheme::Dark
        );
        let fill = if dark {
            crate::scrub_value::SELECTION_DARK
        } else {
            crate::scrub_value::SELECTION_LIGHT
        };
        let pad = pad(app);
        crate::draw::fill_rect(
            &mut app.vg,
            [
                fbox.x + pad,
                fbox.y + 2.0,
                fbox.width - 2.0 * pad,
                fbox.height - 4.0,
            ],
            crate::draw::nvg_color(fill),
        );
    }
    let style = number_style();
    let text_w = mv_text::measure(&mut app.fonts, style, &display);
    let text_ink = if invalid {
        crate::draw::nvg_color(crate::ui::ink(theme, mv_theme::Token::Destructive))
    } else {
        crate::draw::nvg_color(ink)
    };
    mv_text::draw(
        &mut app.fonts,
        &mut app.vg,
        style,
        readout_x + open + x_slot - text_w,
        number_base,
        &display,
        text_ink,
    );
}

/// Performs a fact's effects through `p`'s page param.
fn apply(app: &mut App, effects: Vec<XEffect>) {
    let now_ms = (app.input.time / MS) as u64;
    for fx in effects {
        match fx {
            XEffect::Value(next) => {
                if p_value(app) != next {
                    app.p_param.set(next, now_ms);
                    app.dirty = true;
                }
            }
            XEffect::ValueFlush(next) => {
                app.p_param.set_flushed(&mut app.url, next);
                app.dirty = true;
            }
            XEffect::End(FieldEnd::Commit | FieldEnd::Cancel) => focus_slider(app),
            XEffect::End(FieldEnd::Blur) => {}
            XEffect::Focus => {
                crate::input::set_focus(app, field_hit(), true);
                crate::transform_row::start_text_input(app);
                app.dirty = true;
            }
        }
    }
}

/// Opens the edit with its text selected (a mouse-down, a focus, or Enter
/// from the x-of-P slider): the focus, the selection and the platform's
/// text input, as a focused DOM input takes keys. `visible` marks
/// keyboard-opened focus for the ring.
pub fn open_field(app: &mut App, visible: bool) {
    let value = p_value(app);
    let fx = app.x_field.open(value);
    for effect in fx {
        match effect {
            XEffect::Focus => {
                crate::input::set_focus(app, field_hit(), visible);
                crate::transform_row::start_text_input(app);
                app.dirty = true;
            }
            other => apply(app, vec![other]),
        }
    }
}

/// Returns the focus to the x-of-P slider after a commit or a cancel (the
/// row's `onEditEnd`: every end but a blur). Until h8-points-slider merges
/// nothing holds the stop and the focus clears.
fn focus_slider(app: &mut App) {
    let slider = crate::point_slider::slider_hit();
    let target = if app.input.stops.contains(&slider) {
        slider
    } else {
        crate::input::Hit::default()
    };
    crate::input::set_focus(app, target, true);
    crate::transform_row::stop_text_input(app);
    app.dirty = true;
}

/// Routes a pointer press: a press on the field opens it (or collapses the
/// caret to the end while open, as the scrub box does); anything else
/// blur-commits the open edit, as a mousedown blurs the DOM input. Call
/// after `transform_row::press_down`.
pub fn press_down(app: &mut App) {
    if app.input.pressed.kind == crate::input::HitKind::PointsField {
        if app.x_field.editing() {
            crate::input::set_focus(app, field_hit(), false);
            let shown = display(app);
            collapse_end(&mut app.x_field.field, &shown);
            app.dirty = true;
        } else {
            open_field(app, false);
        }
        return;
    }
    if app.x_field.editing() {
        let value = p_value(app);
        let fx = app.x_field.blur_out(value);
        apply(app, fx);
    }
}

/// Routes a key to the focused field: Enter commits, Escape cancels, the
/// arrows nudge live or move the caret, Tab commits and walks on (the field
/// is never a Tab stop). Call before `transform_row::key`, whose Tab arm
/// would otherwise walk off with the edit open. Returns whether consumed.
pub fn key(app: &mut App, event: &crate::input::KeyEvent) -> bool {
    use sdl3::keyboard::Keycode;
    if !event.down || event.repeat {
        return false;
    }
    if app.input.focus.kind != crate::input::HitKind::PointsField {
        return false;
    }
    let value = p_value(app);
    match event.key {
        Keycode::Return | Keycode::KpEnter => {
            let fx = app.x_field.key_down(XKey::Enter, value);
            apply(app, fx);
        }
        Keycode::Escape => {
            let fx = app.x_field.key_down(XKey::Escape, value);
            apply(app, fx);
        }
        Keycode::Up => {
            let fx = app.x_field.key_down(XKey::ArrowUp, value);
            apply(app, fx);
        }
        Keycode::Down => {
            let fx = app.x_field.key_down(XKey::ArrowDown, value);
            apply(app, fx);
        }
        Keycode::Left => {
            let fx = app.x_field.key_down(XKey::Left, value);
            apply(app, fx);
        }
        Keycode::Right => {
            let fx = app.x_field.key_down(XKey::Right, value);
            apply(app, fx);
        }
        Keycode::Home => {
            let fx = app.x_field.key_down(XKey::Home, value);
            apply(app, fx);
        }
        Keycode::End => {
            let fx = app.x_field.key_down(XKey::End, value);
            apply(app, fx);
        }
        Keycode::Backspace => {
            let fx = app.x_field.key_down(XKey::Backspace, value);
            apply(app, fx);
        }
        Keycode::Delete => {
            let fx = app.x_field.key_down(XKey::Delete, value);
            apply(app, fx);
        }
        Keycode::Tab => {
            tab_out(
                app,
                event
                    .mods
                    .intersects(sdl3::keyboard::Mod::LSHIFTMOD | sdl3::keyboard::Mod::RSHIFTMOD),
            );
        }
        Keycode::A
            if event.mods.intersects(
                sdl3::keyboard::Mod::LCTRLMOD
                    | sdl3::keyboard::Mod::RCTRLMOD
                    | sdl3::keyboard::Mod::LGUIMOD
                    | sdl3::keyboard::Mod::RGUIMOD,
            ) =>
        {
            let shown = display(app);
            select_all(&mut app.x_field.field, &shown);
            app.dirty = true;
        }
        _ => return false,
    }
    true
}

/// Tab leaves the field (never a Tab stop): the edit blur-commits, and the
/// focus walks on — forward to the slider, back to the stop before it (the
/// DOM order around the input).
fn tab_out(app: &mut App, shift: bool) {
    let value = p_value(app);
    let fx = app.x_field.blur_out(value);
    apply(app, fx);
    let slider = crate::point_slider::slider_hit();
    let target = if shift {
        crate::input::stop_beside(app, slider, -1)
    } else if app.input.stops.contains(&slider) {
        slider
    } else {
        crate::input::Hit::default()
    };
    crate::input::set_focus(app, target, true);
    app.dirty = true;
}

/// Routes typed text to the focused field, replacing the selection.
pub fn text(app: &mut App, text: &str) {
    if text.is_empty() || app.input.focus.kind != crate::input::HitKind::PointsField {
        return;
    }
    let value = p_value(app);
    app.x_field.type_text(value, text);
    app.dirty = true;
}

/// Lays the full page out twice, converged on the visible range, as the
/// card's layout test does (the sidebar lays out before the plane, so the
/// second pass reads the current range). Takes `&mut`: the measure closure
/// keeps the app's address from `init_clay`, so the app must never move
/// again — returning it by value would dangle that pointer into the
/// helper's frame and fail the next measure.
#[cfg(test)]
fn laid_out_page(app: &mut crate::app::App, query: &str) {
    // Callers hold `mv_clay::tests_lock` for the whole test: the layout
    // context is global, and one lock covers every test that lays out.
    assert!(mv_text::init(&mut app.fonts));
    crate::ui::init_clay(app);
    app.size = [1440.0, 900.0];
    app.pixels = [2880, 1800];
    app.url = crate::url::Url::from_query(query);
    crate::transform_section::init_params_from_url(app);
    let _commands = crate::page::layout_page(app);
    app.visible_range = crate::visible_range::live(app);
    assert!(
        app.visible_range.is_some(),
        "the laid out plane shows a range"
    );
}

/// The field and tip boxes against the reference's (`page.json` at 96f1e84,
/// webkit): closed in the rest state, open with the tip while editing.
#[cfg(test)]
mod layout_tests {
    use super::*;

    const SLACK: f32 = 0.5;

    fn check(shot: &str, name: &str, id: mv_sys::clay::ElementId) {
        let _guard = mv_clay::tests_lock();
        let mut app = crate::fixture::headless_app();
        laid_out_page(&mut app, "");
        let _commands = crate::page::layout_page(&mut app);
        check_in(&app, shot, name, id);
    }

    fn check_in(app: &App, shot: &str, name: &str, id: mv_sys::clay::ElementId) {
        let clay = app.clay.as_ref().expect("clay context initialised");
        let data = clay.get_element_data(id);
        assert!(data.found, "{name}: no such element");
        let b = data.boundingBox;
        let fixture = crate::fixture::page_fixture().expect("page fixture parses");
        let shot = crate::fixture::fixture_shot(&fixture, shot).expect("shot in fixture");
        let want = crate::fixture::fixture_box(&shot.webkit, name).expect("box in fixture");
        for (i, (got, want)) in [b.x, b.y, b.width, b.height]
            .iter()
            .zip(want.iter())
            .enumerate()
        {
            assert!(
                (got - want).abs() <= SLACK,
                "{name}[{i}]: laid out {got}, reference {want}"
            );
        }
    }

    #[test]
    fn closed_field_matches_the_reference() {
        check(
            "rows-default",
            "field_p",
            mv_clay::id(crate::points_section::X_SLOT_ID, 0),
        );
    }

    #[test]
    fn open_field_and_tip_match_the_reference() {
        let _guard = mv_clay::tests_lock();
        let mut app = crate::fixture::headless_app();
        laid_out_page(&mut app, "");
        app.x_field.open(p_value(&app));
        let _commands = crate::page::layout_page(&mut app);
        check_in(
            &app,
            "field-open",
            "field_p",
            mv_clay::id(crate::points_section::X_SLOT_ID, 0),
        );
        check_in(&app, "field-open", "tip_p", mv_clay::id(P_TIP_ID, 0));
    }

    #[test]
    fn error_tip_matches_the_reference() {
        let _guard = mv_clay::tests_lock();
        let mut app = crate::fixture::headless_app();
        laid_out_page(&mut app, "");
        app.x_field.open(p_value(&app));
        app.x_field.type_text(p_value(&app), "abc");
        let _ = app.x_field.key_down(XKey::Enter, p_value(&app));
        assert!(app.x_field.invalid());
        let _commands = crate::page::layout_page(&mut app);
        check_in(
            &app,
            "field-invalid",
            "field_p",
            mv_clay::id(crate::points_section::X_SLOT_ID, 0),
        );
        check_in(&app, "field-invalid", "tip_p", mv_clay::id(P_TIP_ID, 0));
    }
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
