//! The typed value field: opening, committing, cancelling, the caret, and UTF-8
//! motion.
//!
//! Enter on a box or P's readout opens the field with the value selected;
//! commit parses and applies, Escape cancels, and the caret blinks while open.

use mv_sys::clay;
use sdl3::keyboard::{Keycode, Mod};

use crate::app::App;
use crate::input::{Hit, HitKind};
#[cfg(not(target_os = "emscripten"))]
use crate::platform_native::platform_field_editing;
#[cfg(target_os = "emscripten")]
use crate::platform_web::platform_field_editing;

/// The open field: whose, its bytes, caret, anchor, refusal and blink.
#[derive(Clone, Debug)]
pub struct Field {
    pub open: bool,
    pub part: mv_viewer::Part,
    pub text: [u8; 64],
    pub len: i64,
    pub caret: i64,
    pub anchor: i64,
    pub error: String,
    pub base: f64,
    pub blink_on: bool,
    pub selecting: bool,
}

/// P's field's padding and line.
pub const P_FIELD_PAD: f32 = 5.0;
pub const P_FIELD_LINE: f32 = 24.0;

/// Whether the field is open on a parameter's box.
pub fn field_open_on(app: &mut App, p: mv_viewer::Param) -> bool {
    app.boxes.field.open && app.boxes.field.part == crate::number_box::part_of_param(p)
}

/// Whether the field is open on P.
pub fn p_field_open(app: &mut App) -> bool {
    app.boxes.field.open && app.boxes.field.part == mv_viewer::Part::P
}

/// The part's field hit: its box, or P's field.
pub fn field_hit(part: mv_viewer::Part) -> Hit {
    if part == mv_viewer::Part::P {
        return Hit {
            kind: HitKind::PField,
            index: 0,
        };
    }
    Hit {
        kind: HitKind::Box,
        index: part_of_param_index(part),
    }
}

/// A part's parameter index for its box hit.
fn part_of_param_index(part: mv_viewer::Part) -> i64 {
    match part {
        mv_viewer::Part::A => 0,
        mv_viewer::Part::K => 1,
        mv_viewer::Part::B => 2,
        _ => 3,
    }
}

/// Whether the focus hit is the field's.
pub fn field_holds(app: &mut App, hit: Hit) -> bool {
    app.boxes.field.open && field_hit(app.boxes.field.part) == hit
}

/// The part's model value.
fn field_value(app: &mut App, part: mv_viewer::Part) -> f64 {
    let (p, is_param) = mv_viewer::part_param(part);
    if is_param {
        app.viewer.params[p]
    } else {
        app.viewer.p_x
    }
}

/// The value as the field shows it.
fn field_format(app: &mut App, part: mv_viewer::Part, value: f64) -> String {
    let (_, is_param) = mv_viewer::part_param(part);
    if is_param {
        mv_numfmt::format_stored(value, mv_viewer::part_ui(&app.viewer, part).fine)
    } else {
        mv_numfmt::format_number(value, mv_numfmt::DISPLAY_DP)
    }
}

/// The box or P field text style.
pub fn field_text_style(t: &mv_theme::Theme, part: mv_viewer::Part) -> mv_text::Style {
    if part == mv_viewer::Part::P {
        return mv_text::Style {
            face: crate::point_rows::readout_face(t, 500),
            size: crate::point_rows::READOUT_NUMBER_SIZE,
            tracking: 0.0,
            features: enumset::EnumSet::only(mv_text::Feature::Tabular),
        };
    }
    crate::number_box::field_style(t)
}

/// The right-aligned text edge in the box.
fn field_text_right(part: mv_viewer::Part, r#box: clay::BoundingBox) -> f32 {
    if part == mv_viewer::Part::P {
        return r#box.x + r#box.width - P_FIELD_PAD;
    }
    r#box.x + r#box.width - crate::number_box::FIELD_INSET
}

/// Opens the field with the value text selected, and sets the IME area.
pub fn field_open(app: &mut App, part: mv_viewer::Part) {
    if app.boxes.field.open {
        if app.boxes.field.part == part {
            return;
        }
        field_blur(app);
    }
    let value = field_value(app, part);
    let shown = field_format(app, part, value);
    {
        let f = &mut app.boxes.field;
        f.open = true;
        f.part = part;
        f.base = value;
        f.text = [0; 64];
        f.len = 0;
        f.caret = 0;
        f.anchor = 0;
        f.error = String::new();
        f.blink_on = false;
        f.selecting = false;
        let bytes = shown.as_bytes();
        f.text[..bytes.len()].copy_from_slice(bytes);
        f.len = bytes.len() as i64;
        f.anchor = 0;
        f.caret = f.len;
    }
    caret_restart(app);
    crate::parts::dispatch(
        app,
        mv_viewer::Action::SetEdit(mv_viewer::SetEdit {
            part,
            edit: Some(mv_viewer::FieldEdit {
                error: String::new(),
                base: value,
            }),
        }),
    );
    app.window().subsystem().text_input().start(app.window());
    // The page's key listeners keep Meta combos with the app while a field
    // is edited (the web build's browser shortcuts); natively compiled away.
    platform_field_editing(true);
    let (r, found) = crate::input::region_of(app, field_hit(part));
    if found {
        let area = sdl3::rect::Rect::new(
            r.r#box.x as i32,
            r.r#box.y as i32,
            r.r#box.width as u32,
            r.r#box.height as u32,
        );
        app.window()
            .subsystem()
            .text_input()
            .set_rect(app.window(), area, 0);
    }
    let active = app
        .window()
        .subsystem()
        .text_input()
        .is_active(app.window());
    let (p, is_param) = mv_viewer::part_param(part);
    // Odin `log.debugf`: silent unless `--trace`, and to stdout when it
    // prints, so scripted web runs stay problem-free (a console error
    // counts as a problem there).
    if is_param {
        if crate::trace() {
            println!("field open in box {p:?}, text input {active}");
        }
    } else {
        // P's input takes the focus as it opens.
        let hit = field_hit(part);
        let visible = app.input.focus_visible;
        crate::input::set_focus(app, hit, visible, false);
        if crate::trace() {
            println!("field open in P's readout, text input {active}");
        }
    }
    app.dirty = true;
}

/// How the field ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldEnd {
    Commit,
    Cancel,
    Blur,
}

/// Closes the field and restores focus to its owner.
pub fn field_end(app: &mut App, how: FieldEnd, ring: bool) {
    if !app.boxes.field.open {
        return;
    }
    if how == FieldEnd::Cancel {
        let base = app.boxes.field.base;
        field_apply(app, base);
    }
    let part = app.boxes.field.part;
    {
        let f = &mut app.boxes.field;
        f.open = false;
        f.selecting = false;
    }
    crate::app_loop::timer_stop(app, crate::app_loop::Timer::Caret);
    app.window().subsystem().text_input().stop(app.window());
    // The field is gone: Meta combos go back to the browser (see field_open).
    platform_field_editing(false);
    // Odin `log.debugf`: silent unless `--trace` (see `field_open`).
    if crate::trace() {
        println!(
            "field closed: {:?}",
            String::from_utf8_lossy(&app.boxes.field.text[..app.boxes.field.len as usize])
        );
    }
    crate::parts::dispatch(
        app,
        mv_viewer::Action::SetEdit(mv_viewer::SetEdit { part, edit: None }),
    );
    if how != FieldEnd::Blur {
        let owner = if part == mv_viewer::Part::P {
            Hit {
                kind: HitKind::Track,
                index: 0,
            }
        } else {
            Hit {
                kind: HitKind::Box,
                index: part_of_param_index(part),
            }
        };
        crate::input::set_focus(app, owner, ring, true);
    }
    app.dirty = true;
}

/// Applies the number, or restores the base, on focus loss.
pub fn field_blur(app: &mut App) {
    if !app.boxes.field.open {
        return;
    }
    let (value, error) = field_read(app);
    if error.is_empty() {
        field_apply(app, value);
    } else {
        let base = app.boxes.field.base;
        field_apply(app, base);
    }
    field_end(app, FieldEnd::Blur, true);
}

/// Parses and rounds the text, refusing scale zero.
fn field_read(app: &mut App) -> (f64, String) {
    let text =
        String::from_utf8_lossy(&app.boxes.field.text[..app.boxes.field.len as usize]).into_owned();
    let part = app.boxes.field.part;
    let (n, ok) = mv_numfmt::parse_number(&text);
    if !ok {
        return (0.0, "invalid".to_string());
    }
    let mut value = mv_numfmt::round_to(n, mv_numfmt::FINE_DP);
    if value == 0.0 {
        value = 0.0;
    }
    if !field_accepts(part, value) {
        return (value, mv_viewer::ZERO_SCALE.to_string());
    }
    (value, String::new())
}

/// Whether the part accepts the value (params refuse scale zero).
fn field_accepts(part: mv_viewer::Part, value: f64) -> bool {
    let (p, is_param) = mv_viewer::part_param(part);
    if !is_param {
        return true;
    }
    mv_viewer::accepts_value(p, value)
}

/// Dispatches the value as a jump or a P set.
fn field_apply(app: &mut App, next: f64) {
    let part = app.boxes.field.part;
    if next == field_value(app, part) {
        return;
    }
    let (p, is_param) = mv_viewer::part_param(part);
    if is_param {
        crate::parts::dispatch(
            app,
            mv_viewer::Action::SetParam(mv_viewer::SetParam {
                param: p,
                value: next,
                jump: true,
            }),
        );
    } else {
        crate::parts::dispatch(app, mv_viewer::Action::SetP(mv_viewer::SetP { x: next }));
    }
}

/// Reports the refusal to the model.
fn field_mark(app: &mut App, error: &str) {
    if app.boxes.field.error != error {
        let part = app.boxes.field.part;
        let base = app.boxes.field.base;
        app.boxes.field.error = error.to_string();
        crate::parts::dispatch(
            app,
            mv_viewer::Action::SetEdit(mv_viewer::SetEdit {
                part,
                edit: Some(mv_viewer::FieldEdit {
                    error: error.to_string(),
                    base,
                }),
            }),
        );
    }
}

/// Replaces the selection with the typed text.
pub fn field_text(app: &mut App, typed: &str) {
    if !app.boxes.field.open {
        return;
    }
    let f = &mut app.boxes.field;
    let lo = f.anchor.min(f.caret) as usize;
    let hi = f.anchor.max(f.caret) as usize;
    let len = f.len as usize;
    let bytes = typed.as_bytes();
    if len - (hi - lo) + bytes.len() > f.text.len() {
        return;
    }
    f.text.copy_within(hi..len, lo + bytes.len());
    f.text[lo..lo + bytes.len()].copy_from_slice(bytes);
    f.len = (len + bytes.len() - (hi - lo)) as i64;
    f.caret = (lo + bytes.len()) as i64;
    f.anchor = f.caret;
    field_edited(app);
}

/// Clears the refusal and restarts the caret blink.
fn field_edited(app: &mut App) {
    field_mark(app, "");
    caret_restart(app);
    app.dirty = true;
}

/// Shows the caret and starts its blink timer.
fn caret_restart(app: &mut App) {
    app.boxes.field.blink_on = true;
    crate::app_loop::timer_start(
        app,
        crate::app_loop::Timer::Caret,
        crate::number_box::CARET_BLINK_MS,
    );
    app.dirty = true;
}

/// Toggles the caret blink phase.
pub fn caret_blinked(app: &mut App) {
    if app.boxes.field.open {
        app.boxes.field.blink_on = !app.boxes.field.blink_on;
        crate::app_loop::timer_start(
            app,
            crate::app_loop::Timer::Caret,
            crate::number_box::CARET_BLINK_MS,
        );
        app.dirty = true;
    }
}

/// The prior rune boundary.
fn step_back(s: &str, at: i64) -> i64 {
    if at <= 0 {
        return 0;
    }
    // Snap down first: `at` can sit inside a rune (a caret the draw snaps
    // to a boundary but never stores back), and slicing there panics.
    let mut end = (at as usize).min(s.len());
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    let back = s[..end]
        .chars()
        .next_back()
        .map(|c| c.len_utf8())
        .unwrap_or(0);
    (end - back) as i64
}

/// The next rune boundary.
fn step_on(s: &str, at: i64) -> i64 {
    if at >= s.len() as i64 {
        return s.len() as i64;
    }
    // Snap down first, as above: slicing inside a rune panics.
    let mut start = (at as usize).min(s.len());
    while !s.is_char_boundary(start) {
        start -= 1;
    }
    let on = s[start..].chars().next().map(|c| c.len_utf8()).unwrap_or(0);
    (start + on) as i64
}

/// Moves the caret, collapsing the selection unless extending.
fn field_move(f: &mut Field, to: i64, extend: bool) {
    f.caret = to;
    if !extend {
        f.anchor = to;
    }
}

/// Commit/cancel/nudge/edit/navigation keys.
pub fn field_key(app: &mut App, key: Keycode, mods: Mod) -> bool {
    let shift = mods.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD);
    let command = mods.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD | Mod::LGUIMOD | Mod::RGUIMOD);
    match key {
        Keycode::Return | Keycode::KpEnter => {
            let (value, error) = field_read(app);
            if !error.is_empty() {
                field_mark(app, &error);
                return true;
            }
            field_apply(app, value);
            field_end(app, FieldEnd::Commit, true);
        }
        Keycode::Escape => {
            field_end(app, FieldEnd::Cancel, true);
        }
        Keycode::Up | Keycode::Down => {
            // Up/down nudges 0.01 and applies at once; a nudge onto a refused
            // value (a scale onto 0) does nothing.
            let (value, error) = field_read(app);
            let part = app.boxes.field.part;
            let from = if error.is_empty() {
                value
            } else {
                field_value(app, part)
            };
            let delta = if key == Keycode::Up { 0.01 } else { -0.01 };
            let mut next = mv_numfmt::round_to(from + delta, mv_numfmt::FINE_DP);
            if next == 0.0 {
                next = 0.0;
            }
            if !field_accepts(part, next) {
                return true;
            }
            let shown = field_format(app, part, next);
            {
                let f = &mut app.boxes.field;
                let bytes = shown.as_bytes();
                f.text[..bytes.len()].copy_from_slice(bytes);
                f.len = bytes.len() as i64;
                f.anchor = f.len;
                f.caret = f.len;
            }
            field_mark(app, "");
            field_apply(app, next);
            caret_restart(app);
        }
        Keycode::Backspace => {
            let (lo, hi) = {
                let f = &app.boxes.field;
                (f.anchor.min(f.caret), f.anchor.max(f.caret))
            };
            if lo != hi {
                field_text(app, "");
            } else {
                let caret = app.boxes.field.caret;
                if caret > 0 {
                    let text = String::from_utf8_lossy(
                        &app.boxes.field.text[..app.boxes.field.len as usize],
                    )
                    .into_owned();
                    let back = step_back(&text, caret);
                    app.boxes.field.anchor = back;
                    field_text(app, "");
                }
            }
        }
        Keycode::Delete => {
            let (lo, hi, len) = {
                let f = &app.boxes.field;
                (f.anchor.min(f.caret), f.anchor.max(f.caret), f.len)
            };
            if lo != hi {
                field_text(app, "");
            } else {
                let caret = app.boxes.field.caret;
                if caret < len {
                    let text = String::from_utf8_lossy(
                        &app.boxes.field.text[..app.boxes.field.len as usize],
                    )
                    .into_owned();
                    let on = step_on(&text, caret);
                    app.boxes.field.anchor = on;
                    field_text(app, "");
                }
            }
        }
        Keycode::Left => {
            let (lo, hi) = {
                let f = &app.boxes.field;
                (f.anchor.min(f.caret), f.anchor.max(f.caret))
            };
            if command {
                field_move(&mut app.boxes.field, 0, shift);
            } else if !shift && lo != hi {
                field_move(&mut app.boxes.field, lo, false);
            } else {
                let caret = app.boxes.field.caret;
                let text =
                    String::from_utf8_lossy(&app.boxes.field.text[..app.boxes.field.len as usize])
                        .into_owned();
                let back = step_back(&text, caret);
                field_move(&mut app.boxes.field, back, shift);
            }
            caret_restart(app);
        }
        Keycode::Right => {
            let (lo, hi, len) = {
                let f = &app.boxes.field;
                (f.anchor.min(f.caret), f.anchor.max(f.caret), f.len)
            };
            if command {
                field_move(&mut app.boxes.field, len, shift);
            } else if !shift && lo != hi {
                field_move(&mut app.boxes.field, hi, false);
            } else {
                let caret = app.boxes.field.caret;
                let text =
                    String::from_utf8_lossy(&app.boxes.field.text[..app.boxes.field.len as usize])
                        .into_owned();
                let on = step_on(&text, caret);
                field_move(&mut app.boxes.field, on, shift);
            }
            caret_restart(app);
        }
        Keycode::Home => {
            field_move(&mut app.boxes.field, 0, shift);
            caret_restart(app);
        }
        Keycode::End => {
            let len = app.boxes.field.len;
            field_move(&mut app.boxes.field, len, shift);
            caret_restart(app);
        }
        Keycode::A => {
            if !command {
                return false;
            }
            let len = app.boxes.field.len;
            {
                let f = &mut app.boxes.field;
                f.anchor = 0;
                f.caret = len;
            }
            caret_restart(app);
        }
        _ => {
            return false;
        }
    }
    app.dirty = true;
    true
}

/// Moves the caret/selection to the nearest boundary at x.
pub fn field_point(app: &mut App, r#box: clay::BoundingBox, x: f32, extend: bool) {
    let part = app.boxes.field.part;
    let s =
        String::from_utf8_lossy(&app.boxes.field.text[..app.boxes.field.len as usize]).into_owned();
    let len = s.len() as i64;
    let style = field_text_style(crate::ui::current_theme(app), part);
    let right = field_text_right(part, r#box);
    let start = right - mv_text::measure(&mut app.fonts, style, &s);
    let mut nearest = 0i64;
    let mut distance = f32::MAX;
    let mut at = 0i64;
    loop {
        let prefix = s.get(..(at as usize).min(s.len())).unwrap_or("");
        let d = (start + mv_text::measure(&mut app.fonts, style, prefix) - x).abs();
        if d < distance {
            nearest = at;
            distance = d;
        }
        if at >= len {
            break;
        }
        at = step_on(&s, at);
    }
    {
        let f = &mut app.boxes.field;
        f.caret = nearest;
        if !extend {
            f.anchor = nearest;
        }
    }
    caret_restart(app);
}

/// What the field shows: a selection, or the blink-gated caret.
#[derive(Clone, Copy, Debug)]
pub struct FieldLook {
    pub selection: Option<[i64; 2]>,
    pub caret: Option<i64>,
}

/// Derives the selection or the blink-gated caret.
pub fn field_look(f: &Field) -> FieldLook {
    if f.anchor != f.caret {
        FieldLook {
            selection: Some([f.anchor.min(f.caret), f.anchor.max(f.caret)]),
            caret: None,
        }
    } else if f.blink_on {
        FieldLook {
            selection: None,
            caret: Some(f.caret),
        }
    } else {
        FieldLook {
            selection: None,
            caret: None,
        }
    }
}

/// The text up to a caret or selection end, snapped down to a rune
/// boundary: Odin's caret positions are byte offsets that always land on
/// boundaries (its `step_back`/`step_on` step whole runes), so the snap is
/// identity on every position the field's own ops produce, and only keeps
/// a stale one from slicing inside a code point (U+2212's three bytes).
fn field_prefix(s: &str, at: i64) -> &str {
    let mut end = at.clamp(0, s.len() as i64) as usize;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// Fills the selected text range.
#[allow(clippy::too_many_arguments)]
pub fn draw_field_selection(
    app: &mut App,
    style: &mv_text::Style,
    s: &str,
    pen: f32,
    top: f32,
    line: f32,
    look: FieldLook,
    ink: mv_theme::Color,
) {
    if let Some(selected) = look.selection {
        let x0 = pen + mv_text::measure(&mut app.fonts, *style, field_prefix(s, selected[0]));
        let x1 = pen + mv_text::measure(&mut app.fonts, *style, field_prefix(s, selected[1]));
        crate::draw::fill_rect(
            &mut app.vg,
            [x0, top, x1 - x0, line],
            crate::draw::css_background(ink),
        );
    }
}

/// Draws the 1 px caret on device pixels.
#[allow(clippy::too_many_arguments)]
pub fn draw_field_caret(
    app: &mut App,
    style: &mv_text::Style,
    s: &str,
    pen: f32,
    top: f32,
    line: f32,
    look: FieldLook,
    ink: mv_theme::Color,
) {
    if let Some(caret) = look.caret {
        let density = app.pixels[0] as f32 / app.size[0];
        let at = pen + mv_text::measure(&mut app.fonts, *style, field_prefix(s, caret));
        let x = ((at - 0.5) * density + 0.5).floor() / density;
        crate::draw::fill_rect(&mut app.vg, [x, top.round(), 1.0, line], ink);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A caret or selection end never slices inside a code point: "−1.00"
    // starts with U+2212 (three bytes), so byte offsets 1 and 2 sit inside
    // it, and 8 past the end. Every position snaps down to a boundary.
    #[test]
    fn caret_and_selection_ends_never_split_a_code_point() {
        let s = "\u{2212}1.00";
        assert_eq!(s.len(), 7);
        let want = [
            "",
            "",
            "",
            "\u{2212}",
            "\u{2212}1",
            "\u{2212}1.",
            "\u{2212}1.0",
            "\u{2212}1.00",
        ];
        for (caret, prefix) in want.iter().enumerate() {
            assert_eq!(field_prefix(s, caret as i64), *prefix, "caret {caret}");
        }
        // Past the end clamps to the whole string.
        assert_eq!(field_prefix(s, 8), s);
        assert_eq!(field_prefix(s, 100), s);
        // Negative clamps to empty.
        assert_eq!(field_prefix(s, -1), "");
        // Every snapped prefix is a boundary, so the draw calls' slicing
        // cannot panic at any caret or selection end.
        for at in -1..=8i64 {
            let prefix = field_prefix(s, at);
            assert!(s.is_char_boundary(prefix.len()), "end {at}");
        }
    }
}
