//! One transform row's value box as a scrub control.
//!
//! Ported from `SideBar/TransformSection/ScrubValue/ScrubValue.tsx` and
//! `ScrubValue.module.css` at `b8a0e1d`, with `selectNothing.ts` read for
//! the press lock. The box shows the parameter through [`crate::scrub`]'s
//! [`crate::scrub::shown`]; a press past [`DRAG_THRESHOLD_PX`] drags the
//! value by [`crate::scrub`]'s rules (Shift fine, Ctrl/⌘ snap, read live
//! mid-drag), the wheel steps it, a click opens the field with its text
//! selected, arrows nudge, Esc restores, double-click resets to the default,
//! and a tip shows at rest and while typing.
//!
//! This module is the state machine: [`ScrubBox`] holds the press, the
//! draft, the focus record and the hover record, and every handler returns
//! the [`Effect`]s the input layer performs (the debounced URL writes go
//! through the row's [`crate::url::QueryParam`], the focus and capture
//! through `input.rs`; phase 2 wires them). What the DOM showed in the
//! vitest tests becomes what this state holds. [`ScrubDrag`] is the shape
//! `ruler.rs` (h5-ruler) reads off [`ScrubBox::drag_state`].
//!
//! `selectNothingUntilRelease` is dropped with reason: the app draws to a
//! canvas, so there is no DOM text to select mid-drag; the pointer capture
//! half stays as [`Effect::Capture`].
//!
//! Layout constants live here for the box's own chrome (chevrons, tip, key
//! caps); the 107×32 box geometry stays in [`crate::transform_row`].

use mv_sys::clay;

use crate::functions::{A_PARAM, B_PARAM, H_PARAM, K_PARAM};
use crate::scrub::{ScrubMode, nudged, scrub_mode, scrubbed, wheel_dx};
use crate::state::TransformParam;
use crate::url::QueryParamDef;

/// The px a press may travel before it becomes a drag (`DRAG_THRESHOLD`).
pub const DRAG_THRESHOLD_PX: f64 = 3.0;

/// How long the pointer rests on the box before the rest tip shows
/// (`REST_MS`, armed by the input layer while [`ScrubBox::pointer_on`]).
pub const REST_MS: u64 = 400;

/// The box's left chevron (`Chevron dir="left"`): 6×12, round caps.
pub const CHEVRON_WIDTH: f32 = 6.0;
/// The box's chevron height.
pub const CHEVRON_HEIGHT: f32 = 12.0;
/// The chevron and tip-arrow stroke width (1.6 px).
pub const CHEVRON_STROKE: f32 = 1.6;

/// The tip under the box: `top: calc(100% + 11px)`, `right: -1px`.
pub const TIP_BELOW_PX: f32 = 11.0;
/// The tip's right overhang past the box's border (`right: -1px`).
pub const TIP_RIGHT_PX: f32 = -1.0;
/// The tip's inner gap (4 px) and padding (7 px 10 px).
pub const TIP_GAP: f32 = 4.0;
/// The tip's vertical padding.
pub const TIP_PAD_Y: f32 = 7.0;
/// The tip's horizontal padding.
pub const TIP_PAD_X: f32 = 10.0;
/// The tip's text: sans 400 13 px/15 px.
pub const TIP_FONT_SIZE: f32 = 13.0;
/// The tip's line height.
pub const TIP_LINE_HEIGHT: f32 = 15.0;

/// A key cap in the typing tip: 21 px tall, mono 600 12 px/16 px, padding
/// 1 px 6 px 2 px, 2 px bottom border.
pub const KEY_HEIGHT: f32 = 21.0;
/// A key cap's font size.
pub const KEY_FONT_SIZE: f32 = 12.0;
/// A key cap's line height.
pub const KEY_LINE_HEIGHT: f32 = 16.0;
/// A key cap's horizontal padding.
pub const KEY_PAD_X: f32 = 6.0;

/// The typing tip's arrow glyphs (`Arrow`): 18×14.
pub const TIP_ARROW_WIDTH: f32 = 18.0;
/// The typing tip's arrow height.
pub const TIP_ARROW_HEIGHT: f32 = 14.0;

/// Whether the parameter multiplies (`a`, `b`) or adds (`h`, `k`).
///
/// The porter's `ruler_marks` takes a bool; this enum names the same split
/// for [`ScrubDrag`] so h5-ruler never re-derives it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrubKind {
    Shift,
    Scale,
}

/// The kind from the parameter (`isScale`: `a` or `b`).
#[must_use]
pub fn scrub_kind(param: TransformParam) -> ScrubKind {
    match param {
        TransformParam::A | TransformParam::B => ScrubKind::Scale,
        TransformParam::H | TransformParam::K => ScrubKind::Shift,
    }
}

/// Whether the parameter multiplies.
#[must_use]
pub fn is_scale(param: TransformParam) -> bool {
    scrub_kind(param) == ScrubKind::Scale
}

/// The row's page-param def (`transformParams[param]` in `queryParams.ts`):
/// the default the double-click reset restores and the parser the field's
/// commit, arrows and Esc go through.
#[must_use]
pub fn param_def(param: TransformParam) -> &'static QueryParamDef<f64> {
    match param {
        TransformParam::A => &A_PARAM,
        TransformParam::B => &B_PARAM,
        TransformParam::H => &H_PARAM,
        TransformParam::K => &K_PARAM,
    }
}

/// A press in flight (`press` in `ScrubValue.tsx`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Press {
    /// The pointer that pressed, so a second one cannot hijack the drag.
    pub pointer_id: u64,
    /// The x the drag measures from: the down position, then the last
    /// rebase anchor (`changeMode` resets only this, never the value).
    pub x: f64,
    /// The value the drag measures from: the down value, then the live
    /// value at each modifier change (`changeMode` re-reads it — the
    /// pre-snap value, e.g. 0.6, not the snapped one).
    pub from: f64,
    /// The last x seen, the next rebase's anchor.
    pub last_x: f64,
    /// The modifier in force (`p.mode`), from the keys at the drag's start
    /// and re-read live.
    pub mode: ScrubMode,
    /// The last value sent.
    pub value: f64,
    /// Past the threshold yet.
    pub dragging: bool,
}

/// What a handler asks the input layer to do, in order.
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    /// Write the live value; the row's `QueryParam` debounces the URL.
    Live(f64),
    /// Flush the debounced write (letting go, blur, cancel).
    Flush,
    /// Write and flush at once (wheel commits, arrows, Esc, reset).
    LiveFlush(f64),
    /// Focus the field with its text selected (a click that never dragged).
    FocusSelect,
    /// Leave the field's focus nowhere (Esc, double-click reset).
    Blur,
    /// Capture the pointer to the box and lock selection out until release
    /// (`setPointerCapture` + `selectNothingUntilRelease`'s capture half).
    Capture,
}

/// What a wheel over the box does.
#[derive(Clone, Debug, PartialEq)]
pub enum WheelAction {
    /// The field is typed into: the box ignores the wheel and the page may
    /// scroll (the handler returns before `preventDefault`).
    PassThrough,
    /// The wheel is consumed (`preventDefault`); the value when it moved.
    Handled {
        /// The stepped value, or `None` when the step changed nothing.
        value: Option<f64>,
    },
}

/// The tip under the box.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Tip {
    /// Past [`REST_MS`] of rest: the drag help, one word per span
    /// (`tipWords`: `Drag sideways · Shift for fine · double-click: back
    /// to <default>`).
    Rest(Vec<String>),
    /// While the field has the focus: the key help.
    Typing(Vec<TipPart>),
}

/// One piece of the typing tip.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TipPart {
    /// A key cap (`Enter`, `Esc`).
    Key(String),
    /// An arrow cap (the up/down nudge glyphs).
    KeyIcon(ArrowDir),
    /// Plain words (`apply ·`, `cancel ·`, `nudge 0.1`).
    Words(String),
}

/// The typing tip's arrow direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArrowDir {
    Up,
    Down,
}

/// The drag h5-ruler draws under (`ScrubDrag` on `App.scrub`).
///
/// `start_value`/`dx` are the **rebased** anchor: the value and the x origin
/// at the last modifier change, so the live value is always
/// `scrubbed(start_value, dx, kind == Scale, mode)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrubDrag {
    /// The parameter dragged.
    pub param: TransformParam,
    /// Multiply or add.
    pub kind: ScrubKind,
    /// The value at the drag's (re)start.
    pub start_value: f64,
    /// The px since the (re)start.
    pub dx: f64,
    /// The modifier in force.
    pub mode: ScrubMode,
    /// The box's layout box, anchoring the ruler.
    pub row_box: clay::BoundingBox,
}

/// The value box's state: the live value, the draft, the focus and hover
/// records, and the press (`value`, `draft`, `typingFrom`, `pointerOn`,
/// `resting`, `press` in `ScrubValue.tsx`; the drag `mode` reads off the
/// press, exactly as `data-mode` is `null` until the drag starts).
#[derive(Clone, Debug, PartialEq)]
pub struct ScrubBox {
    /// The parameter this box edits.
    pub param: TransformParam,
    /// The live value (what the field shows through `shown`).
    pub value: f64,
    /// The text typed into the field, verbatim until Enter or blur.
    pub draft: Option<String>,
    /// The value from when the field took the focus; `Some` while typed
    /// into (`typingFrom`, also the typing tip's switch).
    pub typing_from: Option<f64>,
    /// The pointer is on the box (non-touch only).
    pub pointer_on: bool,
    /// The pointer rested past [`REST_MS`].
    pub resting: bool,
    /// The press in flight.
    pub press: Option<Press>,
}

impl ScrubBox {
    /// A box at rest showing `value`.
    #[must_use]
    pub fn new(param: TransformParam, value: f64) -> Self {
        Self {
            param,
            value,
            draft: None,
            typing_from: None,
            pointer_on: false,
            resting: false,
            press: None,
        }
    }

    /// The drag modifier in force (`data-mode`): the press's mode once it
    /// drags, else `None` — which is also when the ruler shows.
    #[must_use]
    pub fn mode(&self) -> Option<ScrubMode> {
        match self.press {
            Some(p) if p.dragging => Some(p.mode),
            _ => None,
        }
    }

    /// Whether the field has the keyboard's focus.
    #[must_use]
    pub fn typing(&self) -> bool {
        self.typing_from.is_some()
    }

    /// The tip under the box, if one shows.
    #[must_use]
    pub fn tip(&self) -> Option<Tip> {
        if self.typing() {
            Some(Tip::Typing(vec![
                TipPart::Key("Enter".to_owned()),
                TipPart::Words("apply ·".to_owned()),
                TipPart::Key("Esc".to_owned()),
                TipPart::Words("cancel ·".to_owned()),
                TipPart::KeyIcon(ArrowDir::Up),
                TipPart::KeyIcon(ArrowDir::Down),
                TipPart::Words("nudge 0.1".to_owned()),
            ]))
        } else if self.resting {
            Some(Tip::Rest(tip_words(param_def(self.param).default)))
        } else {
            None
        }
    }

    /// The drag for the ruler, while one holds the value.
    #[must_use]
    pub fn drag_state(&self, row_box: clay::BoundingBox) -> Option<ScrubDrag> {
        match self.press {
            Some(p) if p.dragging => Some(ScrubDrag {
                param: self.param,
                kind: scrub_kind(self.param),
                start_value: p.from,
                dx: p.last_x - p.x,
                mode: p.mode,
                row_box,
            }),
            _ => None,
        }
    }

    /// The pointer went down on the box (`onPointerDown`): the rest ends,
    /// and a left press while not typing starts a press with a capture.
    pub fn pointer_down(
        &mut self,
        pointer_id: u64,
        x: f64,
        button: u32,
        typing: bool,
    ) -> Vec<Effect> {
        self.end_rest();
        if button != 0 || typing {
            return Vec::new();
        }
        self.press = Some(Press {
            pointer_id,
            x,
            from: self.value,
            last_x: x,
            mode: ScrubMode::Coarse,
            value: self.value,
            dragging: false,
        });
        vec![Effect::Capture]
    }

    /// The pointer moved (`onPointerMove`): without a held button the press
    /// stops (flushing when it dragged, exactly as the shared `stop`);
    /// otherwise the press's pointer drags.
    pub fn pointer_move(
        &mut self,
        pointer_id: u64,
        x: f64,
        held: bool,
        shift_key: bool,
        ctrl_key: bool,
        meta_key: bool,
    ) -> Vec<Effect> {
        if !held {
            return self.stop();
        }
        self.drag(pointer_id, x, shift_key, ctrl_key, meta_key)
    }

    /// The pointer went up (`onPointerUp`): the press's pointer drags to
    /// the release point, then the press stops — flushing when it dragged,
    /// else opening the field with its text selected.
    pub fn pointer_up(
        &mut self,
        pointer_id: u64,
        x: f64,
        shift_key: bool,
        ctrl_key: bool,
        meta_key: bool,
    ) -> Vec<Effect> {
        if self
            .press
            .as_ref()
            .is_none_or(|p| p.pointer_id != pointer_id)
        {
            return Vec::new();
        }
        let mut fx = self.drag(pointer_id, x, shift_key, ctrl_key, meta_key);
        match self.press.take() {
            Some(p) if p.dragging => fx.push(Effect::Flush),
            _ => fx.push(Effect::FocusSelect),
        }
        fx
    }

    /// A modifier key went down or up anywhere (`onWindowKey`): a dragging
    /// press whose modifier changed rebases (`changeMode`).
    pub fn key(&mut self, shift_key: bool, ctrl_key: bool, meta_key: bool) -> Vec<Effect> {
        let next = scrub_mode(shift_key, ctrl_key, meta_key);
        if self
            .press
            .as_ref()
            .is_some_and(|p| p.dragging && p.mode != next)
        {
            self.change_mode(next)
        } else {
            Vec::new()
        }
    }

    /// The press ended elsewhere: window release, cancel, capture loss, or
    /// blur (`onWindowRelease`, `onPointerCancel`, `onLostPointerCapture`,
    /// `blur`): the press clears, flushing when it dragged.
    pub fn stop(&mut self) -> Vec<Effect> {
        match self.press.take() {
            Some(p) if p.dragging => vec![Effect::Flush],
            _ => Vec::new(),
        }
    }

    /// Double-click (`onDoubleClick`): the field blurs (committing typed
    /// text first, as the blur handler runs inside `blur()`), then the
    /// value goes back to the default, flushed, with the focus nowhere.
    pub fn double_click(&mut self) -> Vec<Effect> {
        let mut fx = self.commit();
        fx.push(Effect::Blur);
        self.typing_from = None;
        let default = param_def(self.param).default;
        self.value = default;
        fx.push(Effect::LiveFlush(default));
        fx
    }

    /// The wheel over the box (`onWheel`): ignored while typing (the page
    /// may scroll); else the value steps by the wheel's drag `dx` in the
    /// keys' modifier, live (the URL follows through the debounce).
    pub fn wheel(
        &mut self,
        delta_y: f64,
        delta_mode: u32,
        shift_key: bool,
        ctrl_key: bool,
        meta_key: bool,
        typing: bool,
    ) -> WheelAction {
        if typing {
            return WheelAction::PassThrough;
        }
        let mode = scrub_mode(shift_key, ctrl_key, meta_key);
        let next = scrubbed(
            self.value,
            wheel_dx(delta_y, delta_mode),
            is_scale(self.param),
            mode,
        );
        if next == self.value {
            WheelAction::Handled { value: None }
        } else {
            self.value = next;
            WheelAction::Handled { value: Some(next) }
        }
    }

    /// Text typed into the field (`onChange`): kept verbatim as the draft.
    pub fn type_text(&mut self, text: &str) {
        self.draft = Some(text.to_owned());
    }

    /// Enter in the field: the draft commits, flushed; the focus stays.
    pub fn commit_key(&mut self) -> Vec<Effect> {
        self.commit()
    }

    /// An arrow key in the field: the nudge measures from the text typed
    /// so far when it parses, else from the value; Shift steps 0.01, else
    /// 0.1 (`b8a0e1d`'s order). The draft clears; the write is live.
    pub fn arrow_key(&mut self, up: bool, shift_key: bool) -> Vec<Effect> {
        let draft = self.draft.take();
        let from = draft
            .as_deref()
            .and_then(|text| self.parse(Some(text)))
            .unwrap_or(self.value);
        let delta = (if up { 1.0 } else { -1.0 }) * (if shift_key { 0.01 } else { 0.1 });
        let next = nudged(from, is_scale(self.param), delta);
        self.value = next;
        vec![Effect::Live(next)]
    }

    /// Esc in the field: the draft commits first (the blur handler runs
    /// inside `blur()`), then the value goes back to when the field took
    /// the focus, flushed, with the focus nowhere. Idle unless typed into.
    pub fn escape_key(&mut self) -> Vec<Effect> {
        let Some(from) = self.typing_from else {
            return Vec::new();
        };
        let mut fx = self.commit();
        fx.push(Effect::Blur);
        self.typing_from = None;
        self.value = from;
        fx.push(Effect::LiveFlush(from));
        fx
    }

    /// The field took the focus (`onFocus`): the restore point is the value.
    pub fn field_focus(&mut self) {
        self.typing_from = Some(self.value);
    }

    /// The field lost the focus (`onBlur`): the draft commits, flushed on a
    /// parse; the typing record clears either way.
    pub fn field_blur(&mut self) -> Vec<Effect> {
        let fx = self.commit();
        self.typing_from = None;
        fx
    }

    /// The pointer entered the box (`onPointerEnter`): touch never rests.
    pub fn pointer_enter(&mut self, touch: bool) {
        if !touch {
            self.pointer_on = true;
        }
    }

    /// The pointer left the box (`onPointerLeave`): the rest ends.
    pub fn pointer_leave(&mut self) {
        self.end_rest();
    }

    /// The rest timer fired: the tip shows while the pointer is still on.
    pub fn rest(&mut self) {
        if self.pointer_on {
            self.resting = true;
        }
    }

    /// The rest ends: the pointer left, or a press began (`endRest`).
    fn end_rest(&mut self) {
        self.pointer_on = false;
        self.resting = false;
    }

    /// The drag step (`drag`): past the threshold the press starts dragging
    /// in the event's modifier; a changed modifier rebases (`changeMode`:
    /// the anchor moves to the last x, `from` re-reads the live value, and
    /// snap sends `scrubbed(from, 0)` before the move's own send); then the
    /// value sends `scrubbed(from, x − anchor)`.
    fn drag(
        &mut self,
        pointer_id: u64,
        x: f64,
        shift_key: bool,
        ctrl_key: bool,
        meta_key: bool,
    ) -> Vec<Effect> {
        let next = scrub_mode(shift_key, ctrl_key, meta_key);
        enum Step {
            Ignore,
            Begin,
            Switch,
            Move,
        }
        let step = match self.press.as_mut() {
            None => Step::Ignore,
            Some(p) if p.pointer_id != pointer_id => Step::Ignore,
            Some(p) if !p.dragging => {
                if (x - p.x).abs() < DRAG_THRESHOLD_PX {
                    Step::Ignore
                } else {
                    p.dragging = true;
                    p.mode = next;
                    Step::Begin
                }
            }
            Some(p) if p.mode != next => Step::Switch,
            Some(_) => Step::Move,
        };
        match step {
            Step::Ignore => Vec::new(),
            Step::Begin | Step::Move => self.send_at(x),
            Step::Switch => {
                let mut fx = self.change_mode(next);
                fx.extend(self.send_at(x));
                fx
            }
        }
    }

    /// Re-read the modifier mid-drag (`changeMode`): the anchor moves to
    /// the last x, `from` re-reads the live (pre-snap) value, and snap
    /// quantizes the value in place.
    fn change_mode(&mut self, next: ScrubMode) -> Vec<Effect> {
        let anchor = match self.press.as_mut() {
            Some(p) => {
                p.x = p.last_x;
                p.from = p.value;
                p.mode = next;
                Some((p.from, is_scale(self.param)))
            }
            None => None,
        };
        match anchor {
            Some((from, scale)) if next == ScrubMode::Snap => {
                self.send(scrubbed(from, 0.0, scale, next))
            }
            _ => Vec::new(),
        }
    }

    /// Send the value `dx` px from the anchor (`send` at the move's point).
    fn send_at(&mut self, x: f64) -> Vec<Effect> {
        let next = match self.press.as_mut() {
            Some(p) => {
                p.last_x = x;
                scrubbed(p.from, x - p.x, is_scale(self.param), p.mode)
            }
            None => return Vec::new(),
        };
        self.send(next)
    }

    /// Record the value when it moved (`send`): the press and the box follow.
    fn send(&mut self, next: f64) -> Vec<Effect> {
        match self.press.as_mut() {
            Some(p) if p.value != next => {
                p.value = next;
                self.value = next;
                vec![Effect::Live(next)]
            }
            _ => Vec::new(),
        }
    }

    /// Enter or blur with the taken draft (`commit`): the row's parser
    /// decides; text it rejects (a scale of 0, non-numbers) keeps the value
    /// while the draft clears either way.
    fn commit(&mut self) -> Vec<Effect> {
        let draft = self.draft.take();
        match self.parse(draft.as_deref()) {
            Some(next) => {
                self.value = next;
                vec![Effect::LiveFlush(next)]
            }
            None => Vec::new(),
        }
    }

    /// The row's parser over the draft text.
    fn parse(&self, text: Option<&str>) -> Option<f64> {
        (param_def(self.param).parse)(text)
    }
}

/// The rest tip's words (`tipWords`): `Drag sideways · Shift for fine ·
/// double-click: back to <default>`, one span per word.
#[must_use]
pub fn tip_words(default_value: f64) -> Vec<String> {
    format!("Drag sideways · Shift for fine · double-click: back to {default_value}")
        .split(' ')
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shift_box(value: f64) -> ScrubBox {
        ScrubBox::new(TransformParam::K, value)
    }

    fn scale_box(value: f64) -> ScrubBox {
        ScrubBox::new(TransformParam::A, value)
    }

    fn down(box_: &mut ScrubBox, id: u64, x: f64) -> Vec<Effect> {
        box_.pointer_down(id, x, 0, false)
    }

    fn drag_to(box_: &mut ScrubBox, id: u64, x: f64) -> Vec<Effect> {
        box_.pointer_move(id, x, true, false, false, false)
    }

    // "moves a shift 0.02 per px and writes the URL on letting go".
    #[test]
    fn drag_moves_a_shift_0_02_per_px_and_flushes_on_release() {
        let mut box_ = shift_box(0.0);
        assert_eq!(down(&mut box_, 1, 100.0), vec![Effect::Capture]);
        assert_eq!(box_.mode(), None);

        // A 1 px twitch is below the threshold: no drag, no ruler.
        assert_eq!(drag_to(&mut box_, 1, 101.0), Vec::new());
        assert_eq!(box_.value, 0.0);
        assert_eq!(box_.mode(), None);
        assert_eq!(box_.drag_state(zero_box()), None);

        assert_eq!(drag_to(&mut box_, 1, 130.0), vec![Effect::Live(0.6)]);
        assert_eq!(box_.value, 0.6);
        assert_eq!(box_.mode(), Some(ScrubMode::Coarse));

        // The ruler reads the rebased anchor: from 0, 30 px in.
        let drag = box_.drag_state(zero_box()).expect("dragging");
        assert_eq!(drag.param, TransformParam::K);
        assert_eq!(drag.kind, ScrubKind::Shift);
        assert_eq!(drag.start_value, 0.0);
        assert_eq!(drag.dx, 30.0);
        assert_eq!(drag.mode, ScrubMode::Coarse);

        assert_eq!(
            box_.pointer_up(1, 150.0, false, false, false),
            vec![Effect::Live(1.0), Effect::Flush]
        );
        assert_eq!(box_.value, 1.0);
        assert_eq!(box_.mode(), None);
    }

    // "multiplies a scale, and stops it at 0.01 short of 0".
    #[test]
    fn drag_multiplies_a_scale() {
        let mut box_ = scale_box(2.0);
        down(&mut box_, 1, 1000.0);
        assert_eq!(drag_to(&mut box_, 1, 1347.0), vec![Effect::Live(4.0)]);
        assert_eq!(box_.value, 4.0);
        let drag = box_.drag_state(zero_box()).expect("dragging");
        assert_eq!(drag.kind, ScrubKind::Scale);
        assert_eq!(drag.start_value, 2.0);
        assert_eq!(drag.dx, 347.0);
    }

    // "moves a tenth as fast with Shift, to a third decimal, and holds still
    // when Shift is let go".
    #[test]
    fn shift_drag_is_fine_and_holds_still_on_release() {
        let mut box_ = shift_box(0.0);
        box_.pointer_down(1, 100.0, 0, false);
        assert_eq!(
            box_.pointer_move(1, 130.0, true, true, false, false),
            vec![Effect::Live(0.06)]
        );
        assert_eq!(box_.mode(), Some(ScrubMode::Fine));
        assert_eq!(
            box_.pointer_move(1, 133.0, true, true, false, false),
            vec![Effect::Live(0.066)]
        );

        // Letting go of Shift rebases (from 0.066) without moving the value.
        assert_eq!(box_.key(false, false, false), Vec::new());
        assert_eq!(box_.value, 0.066);
        assert_eq!(box_.mode(), Some(ScrubMode::Coarse));

        assert_eq!(
            box_.pointer_move(1, 143.0, true, false, false, false),
            vec![Effect::Live(0.27)]
        );
        assert_eq!(
            box_.pointer_up(1, 143.0, false, false, false),
            vec![Effect::Flush]
        );
        assert_eq!(box_.value, 0.27);
    }

    // "snaps a shift to a whole number as soon as Ctrl is pressed", and the
    // continued drags off the pre-snap rebase (0.6, not 1).
    #[test]
    fn ctrl_mid_drag_snaps_and_continues_from_the_pre_snap_value() {
        let mut box_ = shift_box(0.0);
        down(&mut box_, 1, 100.0);
        assert_eq!(drag_to(&mut box_, 1, 130.0), vec![Effect::Live(0.6)]);

        // Ctrl down: the anchor moves to 130 with from 0.6, and 0.6 snaps
        // to 1 in place.
        assert_eq!(box_.key(false, true, false), vec![Effect::Live(1.0)]);
        assert_eq!(box_.value, 1.0);
        assert_eq!(box_.mode(), Some(ScrubMode::Snap));

        // +30 px off the 0.6 rebase still reads 1; +50 reads 2.
        assert_eq!(
            box_.pointer_move(1, 160.0, true, false, true, false),
            Vec::new()
        );
        assert_eq!(box_.value, 1.0);
        assert_eq!(
            box_.pointer_move(1, 180.0, true, false, true, false),
            vec![Effect::Live(2.0)]
        );
        assert_eq!(box_.value, 2.0);
        assert_eq!(
            box_.pointer_up(1, 180.0, false, true, false),
            vec![Effect::Flush]
        );
    }

    // "snaps a scale to a quarter with ⌘".
    #[test]
    fn meta_drag_snaps_a_scale_to_a_quarter() {
        let mut box_ = scale_box(1.0);
        box_.pointer_down(1, 100.0, 0, false);
        assert_eq!(
            box_.pointer_move(1, 200.0, true, false, false, true),
            vec![Effect::Live(1.25)]
        );
        assert_eq!(box_.mode(), Some(ScrubMode::Snap));
        assert_eq!(
            box_.pointer_up(1, 200.0, false, false, true),
            vec![Effect::Flush]
        );
        assert_eq!(box_.value, 1.25);
    }

    // "ends when the window loses focus".
    #[test]
    fn window_release_ends_the_drag_and_later_moves_are_dead() {
        let mut box_ = shift_box(0.0);
        down(&mut box_, 1, 100.0);
        assert_eq!(drag_to(&mut box_, 1, 150.0), vec![Effect::Live(1.0)]);

        // Blur (or pointerup/cancel outside): the press clears, flushing.
        assert_eq!(box_.stop(), vec![Effect::Flush]);
        assert_eq!(box_.mode(), None);

        assert_eq!(drag_to(&mut box_, 1, 200.0), Vec::new());
        assert_eq!(box_.value, 1.0);
    }

    // A move with no button held stops a press that never dragged, silently.
    #[test]
    fn move_without_a_button_cancels_a_click_press_silently() {
        let mut box_ = shift_box(0.0);
        down(&mut box_, 1, 100.0);
        assert_eq!(
            box_.pointer_move(1, 120.0, false, false, false, false),
            Vec::new()
        );
        assert_eq!(box_.press, None);
        // The release finds no press: no flush, and no field opens.
        assert_eq!(box_.pointer_up(1, 120.0, false, false, false), Vec::new());
    }

    // "shows the ruler while the drag holds the value".
    #[test]
    fn ruler_gate_follows_the_drag() {
        let mut box_ = shift_box(0.0);
        assert_eq!(box_.drag_state(zero_box()), None);
        down(&mut box_, 1, 100.0);
        assert_eq!(drag_to(&mut box_, 1, 101.0), Vec::new());
        assert_eq!(box_.drag_state(zero_box()), None);
        drag_to(&mut box_, 1, 130.0);
        assert!(box_.drag_state(zero_box()).is_some());
        box_.pointer_up(1, 130.0, false, false, false);
        assert_eq!(box_.drag_state(zero_box()), None);
    }

    // "opens the box for typing on a click that doesn't drag".
    #[test]
    fn click_without_a_drag_opens_the_field() {
        let mut box_ = shift_box(0.0);
        down(&mut box_, 1, 100.0);
        assert_eq!(drag_to(&mut box_, 1, 101.0), Vec::new());
        assert_eq!(
            box_.pointer_up(1, 101.0, false, false, false),
            vec![Effect::FocusSelect]
        );
        // The focus event records the restore point; the draft is empty.
        box_.field_focus();
        assert_eq!(box_.typing_from, Some(0.0));
        assert_eq!(box_.draft, None);

        // A press while typing never starts: the drag stays dead.
        assert_eq!(box_.pointer_down(2, 100.0, 0, true), Vec::new());
        assert_eq!(box_.press, None);
        assert_eq!(
            box_.pointer_move(2, 200.0, true, false, false, false),
            Vec::new()
        );
        assert_eq!(box_.value, 0.0);
        assert_eq!(box_.pointer_up(2, 200.0, false, false, false), Vec::new());
    }

    // A right press never starts either, and the rest ends on any down.
    #[test]
    fn non_left_press_starts_nothing() {
        let mut box_ = shift_box(0.0);
        box_.pointer_enter(false);
        box_.rest();
        assert!(box_.tip().is_some());
        assert_eq!(box_.pointer_down(1, 100.0, 2, false), Vec::new());
        assert_eq!(box_.press, None);
        assert_eq!(box_.tip(), None);
    }

    // A pointer that never pressed is ignored everywhere.
    #[test]
    fn other_pointer_is_ignored() {
        let mut box_ = shift_box(0.0);
        down(&mut box_, 1, 100.0);
        assert_eq!(drag_to(&mut box_, 2, 150.0), Vec::new());
        assert_eq!(box_.value, 0.0);
        assert_eq!(box_.pointer_up(2, 150.0, false, false, false), Vec::new());
        assert!(box_.press.is_some());
    }

    // "moves a shift as a 15 px drag per notch, and scrolls the page while
    // typing".
    #[test]
    fn wheel_steps_and_passes_through_while_typing() {
        let mut box_ = shift_box(0.0);
        match box_.wheel(-100.0, 0, false, false, false, false) {
            WheelAction::Handled { value } => assert_eq!(value, Some(0.3)),
            WheelAction::PassThrough => panic!("plain wheel is handled"),
        }
        assert_eq!(box_.value, 0.3);
        match box_.wheel(-100.0, 0, false, false, false, false) {
            WheelAction::Handled { value } => assert_eq!(value, Some(0.6)),
            WheelAction::PassThrough => panic!("plain wheel is handled"),
        }
        match box_.wheel(-100.0, 0, false, false, false, false) {
            WheelAction::Handled { value } => assert_eq!(value, Some(0.9)),
            WheelAction::PassThrough => panic!("plain wheel is handled"),
        }
        assert_eq!(box_.value, 0.9);

        // While typing the wheel passes through to the page, value held.
        box_.field_focus();
        assert_eq!(
            box_.wheel(-100.0, 0, false, false, false, true),
            WheelAction::PassThrough
        );
        assert_eq!(box_.value, 0.9);
    }

    // "nudge a shift 0.1 from the text typed so far, and 0.01 with Shift".
    #[test]
    fn arrows_nudge_from_the_typed_text() {
        let mut box_ = shift_box(0.0);
        box_.field_focus();
        box_.type_text("2.5");
        assert_eq!(box_.arrow_key(true, false), vec![Effect::Live(2.6)]);
        assert_eq!(box_.value, 2.6);
        assert_eq!(box_.draft, None);
        assert_eq!(box_.arrow_key(true, true), vec![Effect::Live(2.61)]);
        assert_eq!(box_.value, 2.61);
        // Still typing after Enter-less nudges: the tip stays on keys.
        assert!(box_.typing());
    }

    // "nudge a scale's size and stop it at 0.01".
    #[test]
    fn arrows_nudge_a_scale_size() {
        let mut box_ = scale_box(-0.05);
        box_.field_focus();
        assert_eq!(box_.arrow_key(false, true), vec![Effect::Live(-0.04)]);
        assert_eq!(box_.arrow_key(false, false), vec![Effect::Live(-0.01)]);
        assert_eq!(box_.arrow_key(true, true), vec![Effect::Live(-0.02)]);
    }

    // Arrows fall back to the value over unparseable text.
    #[test]
    fn arrows_fall_back_to_the_value_over_rejected_text() {
        let mut box_ = scale_box(2.0);
        box_.field_focus();
        box_.type_text("0");
        assert_eq!(box_.arrow_key(true, false), vec![Effect::Live(2.1)]);
    }

    // "puts back the value from before the box opened, over nudges and typed
    // text, and closes the box".
    #[test]
    fn esc_restores_the_focus_value_and_leaves_nowhere() {
        let mut box_ = shift_box(2.0);
        // A click opens the field: the restore point is 2.
        down(&mut box_, 1, 100.0);
        assert_eq!(
            box_.pointer_up(1, 100.0, false, false, false),
            vec![Effect::FocusSelect]
        );
        box_.field_focus();
        box_.type_text("5");
        assert_eq!(box_.arrow_key(true, false), vec![Effect::Live(5.1)]);
        box_.type_text("7");
        // The blur inside Esc commits the 7 first, then the restore wins.
        assert_eq!(
            box_.escape_key(),
            vec![Effect::LiveFlush(7.0), Effect::Blur, Effect::LiveFlush(2.0)]
        );
        assert_eq!(box_.value, 2.0);
        assert_eq!(box_.draft, None);
        assert!(!box_.typing());
    }

    // Esc over committable text commits first, then restores (the blur
    // handler runs inside `blur()` before the restore write).
    #[test]
    fn esc_commits_before_restoring() {
        let mut box_ = shift_box(2.0);
        box_.field_focus();
        box_.type_text("7");
        assert_eq!(
            box_.escape_key(),
            vec![Effect::LiveFlush(7.0), Effect::Blur, Effect::LiveFlush(2.0)]
        );
        assert_eq!(box_.value, 2.0);
    }

    // Esc with the focus nowhere is idle.
    #[test]
    fn esc_without_typing_is_idle() {
        let mut box_ = shift_box(2.0);
        assert_eq!(box_.escape_key(), Vec::new());
    }

    // "puts a shift back to 0 and closes the box".
    #[test]
    fn double_click_resets_a_shift_to_0() {
        let mut box_ = shift_box(2.0);
        down(&mut box_, 1, 100.0);
        box_.pointer_up(1, 100.0, false, false, false);
        box_.field_focus();
        assert_eq!(
            box_.double_click(),
            vec![Effect::Blur, Effect::LiveFlush(0.0)]
        );
        assert_eq!(box_.value, 0.0);
        assert!(!box_.typing());
    }

    // "puts a scale back to 1 over text typed into it".
    #[test]
    fn double_click_resets_a_scale_over_typed_text() {
        let mut box_ = scale_box(2.0);
        down(&mut box_, 1, 100.0);
        box_.pointer_up(1, 100.0, false, false, false);
        box_.field_focus();
        box_.type_text("3");
        // The blur inside the reset commits the typed 3 first.
        assert_eq!(
            box_.double_click(),
            vec![Effect::LiveFlush(3.0), Effect::Blur, Effect::LiveFlush(1.0)]
        );
        assert_eq!(box_.value, 1.0);
    }

    // Enter commits the draft, flushed, and keeps the focus.
    #[test]
    fn enter_commits_and_keeps_typing() {
        let mut box_ = scale_box(1.0);
        box_.field_focus();
        box_.type_text("2.5");
        assert_eq!(box_.commit_key(), vec![Effect::LiveFlush(2.5)]);
        assert_eq!(box_.value, 2.5);
        assert!(box_.typing());
    }

    // Blur commits the draft and ends the typing.
    #[test]
    fn blur_commits_and_ends_typing() {
        let mut box_ = scale_box(2.0);
        box_.field_focus();
        box_.type_text("0");
        // A scale of 0 parses to nothing: the value stays, the draft goes.
        assert_eq!(box_.field_blur(), Vec::new());
        assert_eq!(box_.value, 2.0);
        assert_eq!(box_.draft, None);
        assert!(!box_.typing());
    }

    // "shows how to drag once a mouse rests on the box, and the keys while
    // the box is typed into".
    #[test]
    fn tips_show_at_rest_and_while_typing() {
        let mut box_ = shift_box(0.0);
        assert_eq!(box_.tip(), None);

        // Touch never rests.
        box_.pointer_enter(true);
        box_.rest();
        assert_eq!(box_.tip(), None);

        // Leaving and returning before the timer: no tip.
        box_.pointer_leave();
        box_.pointer_enter(false);
        assert_eq!(box_.tip(), None);

        // Past REST_MS the words show, with the shift default of 0.
        box_.rest();
        assert_eq!(
            box_.tip(),
            Some(Tip::Rest(
                vec![
                    "Drag",
                    "sideways",
                    "·",
                    "Shift",
                    "for",
                    "fine",
                    "·",
                    "double-click:",
                    "back",
                    "to",
                    "0",
                ]
                .into_iter()
                .map(str::to_owned)
                .collect()
            ))
        );

        // A press ends the rest; the click opens the field and the keys show.
        down(&mut box_, 1, 100.0);
        assert_eq!(box_.tip(), None);
        box_.pointer_up(1, 100.0, false, false, false);
        box_.field_focus();
        assert_eq!(
            box_.tip(),
            Some(Tip::Typing(vec![
                TipPart::Key("Enter".to_owned()),
                TipPart::Words("apply ·".to_owned()),
                TipPart::Key("Esc".to_owned()),
                TipPart::Words("cancel ·".to_owned()),
                TipPart::KeyIcon(ArrowDir::Up),
                TipPart::KeyIcon(ArrowDir::Down),
                TipPart::Words("nudge 0.1".to_owned()),
            ]))
        );

        box_.escape_key();
        assert_eq!(box_.tip(), None);
    }

    // The rest tip names the scale default of 1.
    #[test]
    fn rest_tip_names_a_scale_default_of_1() {
        let mut box_ = scale_box(2.0);
        box_.pointer_enter(false);
        box_.rest();
        match box_.tip() {
            Some(Tip::Rest(words)) => {
                assert_eq!(words.last().map(String::as_str), Some("1"));
                assert_eq!(
                    words.join(" "),
                    "Drag sideways · Shift for fine · double-click: back to 1"
                );
            }
            other => panic!("expected the rest tip, got {other:?}"),
        }
    }

    // The fine hold-still: Shift down then up with no move changes nothing.
    #[test]
    fn fine_hold_still_changes_nothing() {
        let mut box_ = shift_box(0.0);
        down(&mut box_, 1, 100.0);
        assert_eq!(
            box_.pointer_move(1, 130.0, true, true, false, false),
            vec![Effect::Live(0.06)]
        );
        assert_eq!(box_.key(true, false, false), Vec::new());
        assert_eq!(
            box_.pointer_move(1, 130.0, true, true, false, false),
            Vec::new()
        );
        assert_eq!(box_.value, 0.06);
    }

    // Kinds split scales from shifts.
    #[test]
    fn kinds_split_scales_from_shifts() {
        assert_eq!(scrub_kind(TransformParam::A), ScrubKind::Scale);
        assert_eq!(scrub_kind(TransformParam::B), ScrubKind::Scale);
        assert_eq!(scrub_kind(TransformParam::H), ScrubKind::Shift);
        assert_eq!(scrub_kind(TransformParam::K), ScrubKind::Shift);
        assert!(is_scale(TransformParam::A));
        assert!(!is_scale(TransformParam::K));
    }

    fn zero_box() -> clay::BoundingBox {
        clay::BoundingBox {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        }
    }
}
