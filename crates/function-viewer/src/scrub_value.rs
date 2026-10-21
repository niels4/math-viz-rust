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
//! This module holds two halves. [`ScrubBox`] is the pure state machine:
//! every handler returns the [`Effect`]s the input layer performs, and the
//! vitest component tests stand against it directly. The `App` half below
//! (`press_down`, `release_up`, `pointer_moved`, `modifier_key`,
//! `field_key`, `type_text`, `wheel_over`, `focus_lost`, the rest timer, the
//! box text and the tip layout) drives one transient machine per fact: the
//! value's authority stays the row's [`crate::url::QueryParam`], the draft,
//! caret and restore point stay `App.drafts`, the press stays
//! `App.scrub_hold`, and [`ScrubDrag`] on `App.scrub` mirrors the drag for
//! `ruler.rs` (h5-ruler). What the DOM showed in the vitest tests becomes
//! what this state holds.
//!
//! `selectNothingUntilRelease` is dropped with reason: the app draws to a
//! canvas, so there is no DOM text to select mid-drag; the pointer capture
//! half stays as [`Effect::Capture`] (a repaint; the shell already recorded
//! the press).
//!
//! Layout constants live here for the box's own chrome; the tip's look
//! moved to [`crate::tip`] (h8-points-field), which this module's tip calls
//! into; the 107×32 box geometry stays in [`crate::transform_row`].

use mv_sys::clay;

use crate::app::App;
use crate::app_loop::{MS, Timer};
use crate::input::{Hit, HitKind};
use crate::number_field::{
    backspace, collapse_end, delete_forward, home_end, insert_text, move_caret, select_all,
};
use crate::plane_input::MOUSE_POINTER_ID;
use crate::scrub::{ScrubMode, nudged, scrub_mode, scrubbed, shown, wheel_dx};
use crate::state::TransformParam;

/// The px a press may travel before it becomes a drag (`DRAG_THRESHOLD`).
pub const DRAG_THRESHOLD_PX: f64 = 3.0;

/// How long the pointer rests on the box before the rest tip shows
/// (`REST_MS`, armed by the input layer while [`ScrubBox::pointer_on`]).
pub const REST_MS: u64 = 400;

/// The tip under the box: `top: calc(100% + 11px)`, right-flush.
pub const TIP_BELOW_PX: f32 = 11.0;

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
    /// The row's layout box, anchoring the ruler.
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
            Some(Tip::Typing(typing_parts()))
        } else if self.resting {
            Some(Tip::Rest(tip_words(
                crate::transform_section::param_def(self.param).default,
            )))
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
        let default = crate::transform_section::param_def(self.param).default;
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
        (crate::transform_section::param_def(self.param).parse)(text)
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

/// The typing tip's content: the key help (`Enter apply · Esc cancel ·
/// arrows nudge 0.1`), shared by the state model and the layout.
#[must_use]
pub fn typing_parts() -> Vec<TipPart> {
    vec![
        TipPart::Key("Enter".to_owned()),
        TipPart::Words("apply ·".to_owned()),
        TipPart::Key("Esc".to_owned()),
        TipPart::Words("cancel ·".to_owned()),
        TipPart::KeyIcon(ArrowDir::Up),
        TipPart::KeyIcon(ArrowDir::Down),
        TipPart::Words("nudge 0.1".to_owned()),
    ]
}

/// A box press in flight: a scrub, whose value follows the pointer, or a tap
/// — pressed while typed into, when the value never moves and the release
/// may reset (double click) or move the caret. Single-pointer (mouse), as
/// `input.rs` decodes only mouse buttons; the pointer id stays the camera's
/// [`MOUSE_POINTER_ID`](crate::plane_input::MOUSE_POINTER_ID) convention.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScrubHold {
    /// A live scrub press: the drag machine's state.
    Scrub {
        /// The parameter dragged.
        param: TransformParam,
        /// The press (`press` in `ScrubValue.tsx`).
        press: Press,
    },
    /// A press into the focused box: the down position gates the release
    /// (a double click resets, a plain click collapses the caret, a move
    /// away keeps typing as-is — the unmodelled text-select drag).
    Tap {
        /// The parameter typed into.
        param: TransformParam,
        /// The down position, in CSS px.
        pos: [f32; 2],
    },
}

/// The tip's drop below the box: the CSS `11px` measured from the padding
/// box, minus the 1 px border — the reference holds the tip 10 px under the
/// border box (probed), its right edge flush with the box's.
pub const TIP_DROP: f32 = TIP_BELOW_PX - crate::transform_row::BOX_BORDER;

/// The tip's element ids, one param per index (`param_index`, as the rows').
pub const TIP_ID: &str = "ScrubTip";
/// A tip word or key label.
pub const TIP_TEXT_ID: &str = "ScrubTipText";
/// A key cap box.
pub const TIP_KEY_ID: &str = "ScrubTipKey";
/// A key cap's arrow glyph.
pub const TIP_ARROW_ID: &str = "ScrubTipArrow";

/// The tip containers' hashed element ids, one param per index
/// (`param_index`, as the rows'). The page walks the commands inside a laid
/// out tip's box after the rows' post-pass rings, as the DOM's `z-index: 1`
/// paints the tip above the boxes beneath it (h5-fix-tip: k's inset ring
/// printed over a's tip). Only the containers match by id: Clay derives the
/// text and border commands' ids from the element id (`HashNumber` in
/// `clay.h`), so the layer is everything the container's box holds; the
/// arrow glyph scopes declare no commands (their paths paint in
/// [`paint_extras`], which runs after the tip walk).
#[must_use]
pub fn tip_box_ids() -> [u32; 4] {
    [0, 1, 2, 3].map(|index| mv_clay::id(TIP_ID, index).id)
}

/// The text-selection fill, measured on the reference: the browser's default
/// selection swaps with the colour scheme (dark `(83,123,165)`, light
/// `(173,208,245)`); the text keeps the focused ink in both. sRGB bytes, as
/// the headless captures that judge them.
pub const SELECTION_DARK: clay::Color = [83.0, 123.0, 165.0, 255.0];
/// The light-scheme selection fill.
pub const SELECTION_LIGHT: clay::Color = [173.0, 208.0, 245.0, 255.0];

/// The box's shown text: the draft as typed, else `shown` — three decimals
/// while a fine drag holds this parameter, else two (or three for a value
/// with a third decimal, as `shown` decides).
#[must_use]
pub fn box_text(app: &App, param: TransformParam) -> String {
    if let Some(text) = app.drafts[param].draft.as_deref() {
        return text.to_owned();
    }
    shown(
        crate::transform_section::param_value(app, param),
        drag_mode(app, param) == Some(ScrubMode::Fine),
    )
}

/// The drag modifier holding this parameter, if one does: the shared
/// per-parameter scrub mode (`scrubModeAtoms`), set by this box's own drag
/// and by a handle drag on the plane alike, as the readout's `mode` is.
#[must_use]
pub fn drag_mode(app: &App, param: TransformParam) -> Option<ScrubMode> {
    app.state.scrub_mode[param]
}

/// Whether a scrub holds the pointer: moves route to the drag instead of
/// retargeting, as `setPointerCapture` keeps the hook's moves coming. A tap
/// never captures (the DOM returned early, so hover tracks normally).
#[must_use]
pub fn captured(app: &App) -> bool {
    matches!(app.scrub_hold, Some(ScrubHold::Scrub { .. }))
}

/// Routes a pointer press on the rows (`transform_row::press_down`
/// delegates): a press outside any box commits the focused draft and leaves
/// the focus nowhere, as `onBlur={commit}` does; a press into the focused
/// box keeps focus and records a tap; any other box press blurs the old
/// field first (the DOM blurs on mousedown, before the new press) and starts
/// a scrub. The rest tip always ends (`endRest` runs on every down).
pub fn press_down(app: &mut App) {
    clear_rest(app);
    let pressed = app.input.pressed;
    let pos = app.input.pointer;
    let param = match pressed.kind {
        HitKind::TransformField => crate::transform_row::index_param(pressed.index),
        _ => None,
    };
    let Some(param) = param else {
        let focused = app.state.focus.first().copied();
        if let Some(old) = focused {
            crate::transform_row::commit_param(app, old);
            app.drafts[old].typing_from = None;
        }
        app.state.set_focus(&[], true);
        crate::transform_row::stop_text_input(app);
        return;
    };
    if app.state.focus.first().copied() == Some(param) {
        crate::input::set_focus(app, crate::transform_row::field_hit(param), true);
        app.scrub_hold = Some(ScrubHold::Tap { param, pos });
        app.dirty = true;
        return;
    }
    if let Some(old) = app.state.focus.first().copied() {
        crate::transform_row::commit_param(app, old);
        app.drafts[old].typing_from = None;
    }
    app.state.set_focus(&[], true);
    crate::transform_row::stop_text_input(app);
    let value = crate::transform_section::param_value(app, param);
    let x = f64::from(pos[0]);
    app.scrub_hold = Some(ScrubHold::Scrub {
        param,
        press: Press {
            pointer_id: MOUSE_POINTER_ID,
            x,
            from: value,
            last_x: x,
            mode: ScrubMode::Coarse,
            value,
            dragging: false,
        },
    });
    mirror_scrub(app, param);
    app.dirty = true;
}

/// Continues the held scrub at the pointer (`pointer_move` with the button
/// held, routed here while [`captured`]).
pub fn pointer_moved(app: &mut App) {
    let Some(ScrubHold::Scrub { param, .. }) = app.scrub_hold else {
        return;
    };
    let mut box_ = machine(app, param);
    let x = f64::from(app.input.pointer[0]);
    let (shift, ctrl, meta) = mods_now(app);
    let fx = box_.pointer_move(MOUSE_POINTER_ID, x, true, shift, ctrl, meta);
    apply(app, param, fx);
    store(app, param, &box_);
}

/// Ends the held press at the release (`pointer_up`).
///
/// A scrub release drags to the release point, then flushes when it dragged
/// or opens the field with its text selected when it never did (the click).
/// A tap release is positional (no capture held it): over the same box
/// within a twitch, a double click resets to the default and a plain click
/// collapses the caret to the end; a move away keeps typing as-is.
pub fn release_up(app: &mut App, down: Hit, pos: [f32; 2], clicks: i64) {
    match app.scrub_hold {
        Some(ScrubHold::Scrub { param, .. }) => {
            if crate::transform_row::hit_param(down) != Some(param) {
                app.scrub_hold = None;
                mirror_scrub(app, param);
                return;
            }
            let mut box_ = machine(app, param);
            let x = f64::from(pos[0]);
            let (shift, ctrl, meta) = mods_now(app);
            let fx = box_.pointer_up(MOUSE_POINTER_ID, x, shift, ctrl, meta);
            apply(app, param, fx);
            store(app, param, &box_);
        }
        Some(ScrubHold::Tap { param, pos: from }) => {
            app.scrub_hold = None;
            if box_at(app, pos) != Some(param) {
                return;
            }
            let dx = f64::from(pos[0] - from[0]);
            let dy = f64::from(pos[1] - from[1]);
            if dx.hypot(dy) >= DRAG_THRESHOLD_PX {
                return;
            }
            if clicks == 2 {
                let mut box_ = machine(app, param);
                let fx = box_.double_click();
                apply(app, param, fx);
                store(app, param, &box_);
            } else if clicks == 1 {
                let shown = box_text(app, param);
                collapse_end(&mut app.drafts[param], &shown);
                app.dirty = true;
            }
        }
        None => {}
    }
}

/// Re-reads a modifier key anywhere mid-drag (`onWindowKey`): a dragging
/// press whose modifier changed rebases. Runs on key down and up (never a
/// repeat, which cannot change the mode), before the menu and field chains.
pub fn modifier_key(app: &mut App) {
    let Some(ScrubHold::Scrub { param, .. }) = app.scrub_hold else {
        return;
    };
    let mut box_ = machine(app, param);
    let (shift, ctrl, meta) = mods_now(app);
    let fx = box_.key(shift, ctrl, meta);
    apply(app, param, fx);
    store(app, param, &box_);
}

/// Routes a key to the focused field: the nudge arrows (Up/Down, Shift for
/// 0.01), the caret arrows, Home/End, Backspace/Delete, and Esc (restore and
/// leave). Returns whether consumed. Runs from `transform_row::key` before
/// its Enter/Tab/select arms, which do not overlap.
pub fn field_key(app: &mut App, event: &crate::input::KeyEvent) -> bool {
    use sdl3::keyboard::Keycode;
    if !event.down || event.repeat {
        return false;
    }
    if app.input.focus.kind != HitKind::TransformField {
        return false;
    }
    let Some(param) = crate::transform_row::index_param(app.input.focus.index) else {
        return false;
    };
    match event.key {
        Keycode::Up | Keycode::Down => {
            let mut box_ = machine(app, param);
            let fx = box_.arrow_key(event.key == Keycode::Up, shift_held(app));
            apply(app, param, fx);
            store(app, param, &box_);
            let len = box_text(app, param).chars().count();
            app.drafts[param].caret = [len, len];
            app.dirty = true;
            true
        }
        Keycode::Escape => {
            let mut box_ = machine(app, param);
            let fx = box_.escape_key();
            apply(app, param, fx);
            store(app, param, &box_);
            true
        }
        Keycode::Left | Keycode::Right => {
            let shown = box_text(app, param);
            move_caret(
                &mut app.drafts[param],
                &shown,
                if event.key == Keycode::Right { 1 } else { -1 },
            );
            app.dirty = true;
            true
        }
        Keycode::Home | Keycode::End => {
            let shown = box_text(app, param);
            home_end(&mut app.drafts[param], &shown, event.key == Keycode::End);
            app.dirty = true;
            true
        }
        Keycode::Backspace => {
            let shown = box_text(app, param);
            backspace(&mut app.drafts[param], &shown);
            app.dirty = true;
            true
        }
        Keycode::Delete => {
            let shown = box_text(app, param);
            delete_forward(&mut app.drafts[param], &shown);
            app.dirty = true;
            true
        }
        _ => false,
    }
}

/// Routes typed text to the focused field, replacing the selection
/// (`transform_row::text` delegates).
pub fn type_text(app: &mut App, text: &str) {
    if text.is_empty() || app.input.focus.kind != HitKind::TransformField {
        return;
    }
    let Some(param) = crate::transform_row::index_param(app.input.focus.index) else {
        return;
    };
    let shown = box_text(app, param);
    insert_text(&mut app.drafts[param], &shown, text);
    app.dirty = true;
}

/// Steps the hovered box's value by the wheel (`onWheel`): ignored while
/// typed into (the page may scroll), else a drag `dx` in the keys' modifier,
/// live (the URL follows through the debounce). Deltas arrive in page px on
/// both targets (the platform converts), so the DOM line mode never
/// applies here — scripted notches match the reference's px notches exactly.
///
/// Whether the wheel was taken: the box's handler calls `preventDefault`
/// (the page does not scroll), except while the field is typed into, when
/// the handler returns before it and the page may scroll (fix-viewer-size
/// phase 2: the sidebar's `overflow: auto`).
pub fn wheel_over(app: &mut App, delta_y: f32) -> bool {
    let param = match app.input.hover.kind {
        HitKind::TransformField => crate::transform_row::index_param(app.input.hover.index),
        _ => None,
    };
    let Some(param) = param else {
        return false;
    };
    let mut box_ = machine(app, param);
    let typing = app.state.focus.contains(&param);
    let (shift, ctrl, meta) = mods_now(app);
    match box_.wheel(f64::from(delta_y), 0, shift, ctrl, meta, typing) {
        WheelAction::Handled { value: Some(next) } => {
            live_value(app, param, next, (app.input.time / MS) as u64);
            true
        }
        WheelAction::Handled { value: None } => true,
        WheelAction::PassThrough => false,
    }
}

/// The window lost the focus: a held scrub ends (flushing when it dragged,
/// as `onWindowRelease` stops it), taps and the rest tip drop.
pub fn focus_lost(app: &mut App) {
    let param = match app.scrub_hold {
        Some(ScrubHold::Scrub { param, .. }) => Some(param),
        _ => None,
    };
    match app.scrub_hold.take() {
        Some(ScrubHold::Scrub { param, press }) if press.dragging => flush_value(app, param),
        _ => {}
    }
    clear_rest(app);
    if let Some(param) = param {
        mirror_scrub(app, param);
    }
}

/// Tracks the rest tip across hover moves: entering a box arms the
/// [`REST_MS`] timer, leaving one (or pressing) drops the armed and showing
/// tip. A move between a row and its own box is not an enter — but moving
/// from row padding into the box is, as the DOM fires the box's enter.
/// Runs from `transform_row::sync_hover` on every hover change.
pub fn hover_moved(app: &mut App, before: Hit, now: Hit) {
    fn boxed(hit: Hit) -> Option<TransformParam> {
        match hit.kind {
            HitKind::TransformField => crate::transform_row::index_param(hit.index),
            _ => None,
        }
    }
    if boxed(before) == boxed(now) {
        return;
    }
    clear_rest(app);
    if let Some(param) = boxed(now)
        && app.scrub_hold.is_none()
        && app.input.time != 0
    {
        app.scrub_rest = Some((param, app.input.time));
        app.timers[Timer::ScrubRest] = app.input.time + REST_MS as i64 * MS;
    }
}

/// The rest timer fired: the tip shows while the pointer is still on the
/// box, nothing holds it, and the field is not typed into (the typing tip
/// owns the box then; a stale armed tip never surfaces after Esc).
pub fn rest_fired(app: &mut App) {
    if let Some((param, _)) = app.scrub_rest {
        let still_on = boxed_hover(app) == Some(param);
        let typing = app.drafts[param].typing_from.is_some();
        if still_on && app.scrub_hold.is_none() && !typing && app.scrub_resting != Some(param) {
            app.scrub_resting = Some(param);
            app.dirty = true;
        }
    }
}

/// The box the pointer hovers, if one.
fn boxed_hover(app: &App) -> Option<TransformParam> {
    match app.input.hover.kind {
        HitKind::TransformField => crate::transform_row::index_param(app.input.hover.index),
        _ => None,
    }
}

/// The box at a position, if one (the positional tap release).
fn box_at(app: &App, pos: [f32; 2]) -> Option<TransformParam> {
    let hit = crate::input::hit_at(&app.input.regions, pos);
    match hit.kind {
        HitKind::TransformField => crate::transform_row::index_param(hit.index),
        _ => None,
    }
}

/// Drops the armed and showing rest tip and its timer.
fn clear_rest(app: &mut App) {
    app.scrub_rest = None;
    app.scrub_resting = None;
    app.timers[Timer::ScrubRest] = 0;
}

/// The modifiers held, as the key events left them: Shift for fine,
/// Ctrl or ⌘ for snap (`scrubMode`).
fn mods_now(app: &App) -> (bool, bool, bool) {
    use sdl3::keyboard::Mod;
    let mods = app.input.mods;
    (
        shift_held(app),
        mods.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD),
        mods.intersects(Mod::LGUIMOD | Mod::RGUIMOD),
    )
}

/// Whether Shift is held.
fn shift_held(app: &App) -> bool {
    use sdl3::keyboard::Mod;
    app.input.mods.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD)
}

/// The transient machine for a fact: the live value, the draft and restore
/// point from the rows' state, the press from the hold.
fn machine(app: &App, param: TransformParam) -> ScrubBox {
    let mut box_ = ScrubBox::new(param, crate::transform_section::param_value(app, param));
    box_.draft = app.drafts[param].draft.clone();
    box_.typing_from = app.drafts[param].typing_from;
    if let Some(ScrubHold::Scrub { param: held, press }) = app.scrub_hold
        && held == param
    {
        box_.press = Some(press);
    }
    box_
}

/// Stores a fact's machine back: the draft rejoins the rows' state and the
/// press rejoins the hold (a spent press clears a scrub hold for its
/// parameter; a tap hold is never stored through here). The restore point is
/// not stored: the machine only reads it (Esc idles or restores from it),
/// while the app side owns it — storing it here would clobber the restore
/// point opening the field just set (its `FocusSelect` runs inside `apply`,
/// before this store).
fn store(app: &mut App, param: TransformParam, box_: &ScrubBox) {
    app.drafts[param].draft = box_.draft.clone();
    match box_.press {
        Some(press) => {
            app.scrub_hold = Some(ScrubHold::Scrub { param, press });
        }
        None => {
            if matches!(app.scrub_hold, Some(ScrubHold::Scrub { param: held, .. }) if held == param)
            {
                app.scrub_hold = None;
            }
        }
    }
    mirror_scrub(app, param);
}

/// Performs a fact's effects through the row's page param.
fn apply(app: &mut App, param: TransformParam, effects: Vec<Effect>) {
    let now_ms = (app.input.time / MS) as u64;
    for fx in effects {
        match fx {
            Effect::Live(next) => live_value(app, param, next, now_ms),
            Effect::Flush => flush_value(app, param),
            Effect::LiveFlush(next) => write_value(app, param, next),
            Effect::FocusSelect => open_field(app, param),
            Effect::Blur => blur_field(app, param),
            Effect::Capture => {
                app.dirty = true;
            }
        }
    }
}

/// Writes the live value (debounced), repainting on a change.
fn live_value(app: &mut App, param: TransformParam, next: f64, now_ms: u64) {
    let changed = crate::transform_section::param_value(app, param) != next;
    match param {
        TransformParam::A => app.a_param.set(next, now_ms),
        TransformParam::K => app.k_param.set(next, now_ms),
        TransformParam::B => app.b_param.set(next, now_ms),
        TransformParam::H => app.h_param.set(next, now_ms),
    }
    if changed {
        app.dirty = true;
    }
}

/// Flushes the debounced write (letting go, blur, cancel).
fn flush_value(app: &mut App, param: TransformParam) {
    match param {
        TransformParam::A => app.a_param.flush(&mut app.url),
        TransformParam::K => app.k_param.flush(&mut app.url),
        TransformParam::B => app.b_param.flush(&mut app.url),
        TransformParam::H => app.h_param.flush(&mut app.url),
    }
    app.dirty = true;
}

/// Writes and flushes at once (commits, nudges, Esc, reset).
fn write_value(app: &mut App, param: TransformParam, next: f64) {
    match param {
        TransformParam::A => app.a_param.set_flushed(&mut app.url, next),
        TransformParam::K => app.k_param.set_flushed(&mut app.url, next),
        TransformParam::B => app.b_param.set_flushed(&mut app.url, next),
        TransformParam::H => app.h_param.set_flushed(&mut app.url, next),
    }
    app.dirty = true;
}

/// Opens the field with its text selected (a click that never dragged):
/// the keyboard stop, the highlight, the restore point, the selection, and
/// the platform's text input — as the focused DOM input takes keys.
fn open_field(app: &mut App, param: TransformParam) {
    crate::transform_row::focus_field(app, param);
    let value = crate::transform_section::param_value(app, param);
    app.drafts[param].typing_from = Some(value);
    let shown = box_text(app, param);
    select_all(&mut app.drafts[param], &shown);
    app.dirty = true;
}

/// Leaves the field's focus nowhere (Esc, double-click reset).
fn blur_field(app: &mut App, param: TransformParam) {
    crate::input::set_focus(app, Hit::default(), false);
    app.state.set_focus(&[], true);
    app.drafts[param].typing_from = None;
    crate::transform_row::stop_text_input(app);
    app.dirty = true;
}

/// Mirrors the drag for the ruler and the shared scrub modes: while a scrub
/// holds the value, `App.scrub` carries the rebased anchor off the row's
/// laid-out box, and the shared mode for the dragged parameter is set (a
/// handle drag on the plane sets the same slot directly, so the rows light
/// and the rulers come out for every held parameter). The slot clears when
/// this parameter's own box drag retires; a handle drag's modes are the
/// plane's to clear, so a retiring box never touches a slot a handle holds.
fn mirror_scrub(app: &mut App, param: TransformParam) {
    let was = app.scrub.filter(|drag| drag.param == param);
    app.scrub = match app.scrub_hold {
        Some(ScrubHold::Scrub { param, press }) if press.dragging => {
            let (region, _) = crate::input::region_of(app, crate::transform_row::row_hit(param));
            Some(ScrubDrag {
                param,
                kind: scrub_kind(param),
                start_value: press.from,
                dx: press.last_x - press.x,
                mode: press.mode,
                row_box: region.r#box,
            })
        }
        _ => None,
    };
    let held = app
        .scrub
        .filter(|drag| drag.param == param)
        .map(|drag| drag.mode);
    match (was, held) {
        (_, Some(mode)) => {
            app.state.scrub_mode[param] = Some(mode);
        }
        // The box's own drag retired (`stop` clears when its press was
        // dragging). A handle drag's modes never pass through here: `was`
        // is only this box's mirrored drag.
        (Some(_), None) => {
            app.state.scrub_mode[param] = None;
        }
        _ => {}
    }
}

/// Lays out the tip under the box: the rest words once the pointer rested,
/// the key help while typed into. A floating sibling of the box's content
/// (`position: absolute; top: calc(100% + 11px); right: -1px`), so it takes
/// no part in the row's layout; pointer-transparent (no regions declared).
/// Call at the end of the box's scope, as the chevrons' elements are.
/// The tip's shared element ids ([`crate::tip::TipIds`]).
fn tip_ids() -> crate::tip::TipIds {
    crate::tip::TipIds {
        tip: TIP_ID,
        text: TIP_TEXT_ID,
        key: TIP_KEY_ID,
        arrow: TIP_ARROW_ID,
    }
}

/// The tip's shared placement ([`crate::tip::TipPlace`]): right-flush under
/// the box, 10 px below it.
fn tip_place(index: u32) -> crate::tip::TipPlace {
    crate::tip::TipPlace {
        parent: mv_clay::id(crate::transform_row::BOX_ID, index).id,
        drop: TIP_DROP,
        z: 1,
    }
}

/// A tip part as the shared tip's cell.
fn tip_cell(part: TipPart) -> crate::tip::TipCell {
    match part {
        TipPart::Key(label) => crate::tip::TipCell::Key(label),
        TipPart::KeyIcon(ArrowDir::Up) => crate::tip::TipCell::Arrow(crate::tip::ArrowDir::Up),
        TipPart::KeyIcon(ArrowDir::Down) => crate::tip::TipCell::Arrow(crate::tip::ArrowDir::Down),
        TipPart::Words(words) => crate::tip::TipCell::Words(words),
    }
}

/// Lays out the tip under the box: the rest words once the pointer rested,
/// the key help while typed into. A floating sibling of the box's content
/// (`position: absolute; top: calc(100% + 11px)`), so it takes no part in
/// the row's layout; pointer-transparent (no regions declared). The paint
/// lives in [`crate::tip`]; this only maps the content over. Call at the
/// end of the box's scope, as the chevrons' elements are.
pub fn tip(app: &mut App, param: TransformParam, index: u32) {
    let typing = app.drafts[param].typing_from.is_some();
    let resting = app.scrub_resting == Some(param);
    if !typing && !resting {
        return;
    }
    let base = crate::transform_row::param_index(param) as u32 * 16;
    if typing {
        let cells: Vec<crate::tip::TipCell> = typing_parts().into_iter().map(tip_cell).collect();
        crate::tip::layout_tip(
            app,
            &tip_ids(),
            index,
            base,
            &tip_place(index),
            crate::tip::TipTone::Default,
            &cells,
        );
    } else {
        let default = crate::transform_section::param_def(param).default;
        let cells: Vec<crate::tip::TipCell> = tip_words(default)
            .into_iter()
            .map(crate::tip::TipCell::Words)
            .collect();
        crate::tip::layout_tip(
            app,
            &tip_ids(),
            index,
            base,
            &tip_place(index),
            crate::tip::TipTone::Default,
            &cells,
        );
    }
}

/// Paints the typing tip's arrow glyphs after the command walk (paths, not
/// text, as the chevrons are). The paint lives in [`crate::tip`]. Call from
/// `draw_page` after the tip walk, as the chevrons are.
pub fn paint_extras(app: &mut App) {
    for param in crate::transform_section::ROW_ORDER {
        if app.drafts[param].typing_from.is_none() {
            continue;
        }
        let base = crate::transform_row::param_index(param) as u32 * 16;
        // `typing_parts` order: Enter, apply, Esc, cancel, up, down, nudge.
        let cells: Vec<crate::tip::TipCell> = typing_parts().into_iter().map(tip_cell).collect();
        crate::tip::paint_arrows(app, &tip_ids(), base, &cells);
    }
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

    // ---- App wiring (headless; regions pushed by hand) ----

    use crate::input::{Cursor, Region};
    use crate::transform_row::{field_hit, param_index};

    // Kinds split scales from shifts.
    #[test]
    fn kinds_split_scales_from_shifts() {
        assert_eq!(scrub_kind(TransformParam::A), ScrubKind::Scale);
        assert_eq!(scrub_kind(TransformParam::B), ScrubKind::Scale);
        assert_eq!(scrub_kind(TransformParam::H), ScrubKind::Shift);
        assert_eq!(scrub_kind(TransformParam::K), ScrubKind::Shift);
        assert!(is_scale(TransformParam::A));
        assert!(!is_scale(TransformParam::K));
        assert_eq!(param_index(TransformParam::B), 1);
    }

    // Box text covers the drag mirror; the zero box anchors the pure tests.
    fn zero_box() -> clay::BoundingBox {
        clay::BoundingBox {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        }
    }

    /// The tip containers' ids are one hashed id per parameter (h5-fix-tip:
    /// the page finds the laid-out tip's box through its plate command).
    #[test]
    fn tip_box_ids_cover_each_parameter() {
        let ids = super::tip_box_ids();
        assert_eq!(ids.len(), 4);
        for index in 0..4_u32 {
            assert_eq!(ids[index as usize], mv_clay::id(super::TIP_ID, index).id);
        }
    }

    /// The tip's box against the reference's (webkit `page.json` at b8a0e1d,
    /// card-relative: the card stands at [29, 376.5]): the rest tip hangs 10
    /// px under the box with its right edge flush, wider than the row; the
    /// typing tip is narrower and a key-cap taller. Both are single lines.
    #[test]
    fn tip_lays_out_like_the_reference() {
        for (typing, want) in [
            (false, [-1.390_625, 100.0, 357.390_63, 31.0]),
            (true, [13.078_125, 100.0, 342.921_88, 37.0]),
        ] {
            let _guard = mv_clay::tests_lock();
            let mut app = crate::fixture::headless_app();
            assert!(mv_text::init(&mut app.fonts));
            crate::ui::init_clay(&mut app);
            if typing {
                app.drafts[TransformParam::A].typing_from = Some(1.0);
            } else {
                app.scrub_resting = Some(TransformParam::A);
            }
            app.clay
                .as_mut()
                .expect("clay context initialised")
                .set_layout_dimensions(mv_sys::clay::Dimensions {
                    width: 374.0,
                    height: 900.0,
                });
            app.clay
                .as_mut()
                .expect("clay context initialised")
                .begin_layout();
            let mut root = crate::ui::empty_decl();
            root.layout.sizing = mv_sys::clay::Sizing {
                width: mv_clay::sizing_fixed(374.0),
                height: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
            };
            crate::ui::ui_scope(&mut app, mv_clay::id("CardScope", 0), &root, |app| {
                crate::transform_section::section(app);
            });
            app.clay
                .as_mut()
                .expect("clay context initialised")
                .end_layout();
            let clay = app.clay.as_ref().expect("clay context initialised");
            let data = clay.get_element_data(mv_clay::id(TIP_ID, 0));
            assert!(data.found, "tip laid out (typing {typing})");
            let b = data.boundingBox;
            for (i, (got, want)) in [b.x, b.y, b.width, b.height]
                .iter()
                .zip(want.iter())
                .enumerate()
            {
                assert!(
                    (got - want).abs() <= 0.5,
                    "tip [{i}] typing {typing}: laid out {got}, reference {want}"
                );
            }
        }
    }

    fn box_app(param: TransformParam, x: f32, y: f32) -> crate::app::App {
        let mut app = crate::fixture::headless_app();
        app.input.regions.push(Region {
            hit: field_hit(param),
            r#box: clay::BoundingBox {
                x,
                y,
                width: 107.0,
                height: 32.0,
            },
            cursor: Cursor::EwResize,
            pressed: Cursor::EwResize,
            radius: clay::CornerRadius {
                topLeft: 0.0,
                topRight: 0.0,
                bottomLeft: 0.0,
                bottomRight: 0.0,
            },
            clip: None,
            z: 0,
        });
        app
    }

    fn press_at(app: &mut crate::app::App, param: TransformParam, x: f32, y: f32) {
        app.input.pointer = [x, y];
        app.input.pressed = field_hit(param);
        // The shell's `press` clears the focus stop before routing.
        app.input.focus = Hit::default();
        app.input.focus_visible = false;
        press_down(app);
    }

    fn release_at(app: &mut crate::app::App, param: TransformParam, x: f32, y: f32, clicks: i64) {
        release_up(app, field_hit(param), [x, y], clicks);
    }

    fn open(app: &mut crate::app::App, param: TransformParam, x: f32, y: f32) {
        press_at(app, param, x, y);
        release_at(app, param, x, y, 1);
    }

    // A click that never drags opens the field with its text selected.
    #[test]
    fn app_click_opens_the_field_selected() {
        let mut app = box_app(TransformParam::A, 278.0, 434.5);
        open(&mut app, TransformParam::A, 331.5, 450.5);
        assert_eq!(app.input.focus, field_hit(TransformParam::A));
        assert_eq!(app.drafts[TransformParam::A].typing_from, Some(1.0));
        assert_eq!(app.drafts[TransformParam::A].caret, [0, 4]);
        assert_eq!(app.url.query(), "");
        assert!(app.scrub_hold.is_none());
    }

    // A 50 px drag writes the live value and flushes the URL on release.
    #[test]
    fn app_drag_writes_live_and_flushes_on_release() {
        let mut app = box_app(TransformParam::K, 278.0, 478.5);
        press_at(&mut app, TransformParam::K, 331.5, 494.5);
        app.input.pointer = [381.5, 494.5];
        pointer_moved(&mut app);
        assert_eq!(
            crate::transform_section::param_value(&app, TransformParam::K),
            1.0
        );
        assert_eq!(app.url.query(), "");
        let drag = app.scrub.expect("ruler mirror while held");
        assert_eq!(drag.param, TransformParam::K);
        assert_eq!(drag.start_value, 0.0);
        assert_eq!(drag.dx, 50.0);
        release_at(&mut app, TransformParam::K, 381.5, 494.5, 1);
        assert_eq!(app.url.query(), "k=1");
        assert!(app.scrub.is_none());
        assert!(app.scrub_hold.is_none());
        // A release that dragged never opens the field.
        assert_eq!(app.input.focus, Hit::default());
    }

    // A box drag holds its parameter's shared scrub mode and releases it,
    // so the rows light and the ruler comes out under the dragged row.
    #[test]
    fn app_box_drag_holds_and_releases_the_shared_mode() {
        let mut app = box_app(TransformParam::K, 278.0, 478.5);
        press_at(&mut app, TransformParam::K, 331.5, 494.5);
        app.input.pointer = [381.5, 494.5];
        pointer_moved(&mut app);
        assert_eq!(
            app.state.scrub_mode[TransformParam::K],
            Some(ScrubMode::Coarse)
        );
        assert_eq!(app.state.scrub_mode[TransformParam::A], None);
        release_at(&mut app, TransformParam::K, 381.5, 494.5, 1);
        assert_eq!(app.state.scrub_mode[TransformParam::K], None);
    }

    // The box text shows three decimals while a fine drag holds it.
    #[test]
    fn app_box_text_is_fine_while_a_fine_drag_holds() {
        let mut app = box_app(TransformParam::K, 278.0, 478.5);
        assert_eq!(box_text(&app, TransformParam::K), "0.00");
        press_at(&mut app, TransformParam::K, 100.0, 494.5);
        app.input.mods = sdl3::keyboard::Mod::LSHIFTMOD;
        app.input.pointer = [130.0, 494.5];
        pointer_moved(&mut app);
        assert_eq!(drag_mode(&app, TransformParam::K), Some(ScrubMode::Fine));
        assert_eq!(box_text(&app, TransformParam::K), "0.060");
    }

    // Ctrl pressed mid-drag rebases onto the pre-snap value and snaps.
    #[test]
    fn app_ctrl_mid_drag_snaps() {
        let mut app = box_app(TransformParam::K, 278.0, 478.5);
        press_at(&mut app, TransformParam::K, 100.0, 494.5);
        app.input.pointer = [130.0, 494.5];
        pointer_moved(&mut app);
        assert_eq!(
            crate::transform_section::param_value(&app, TransformParam::K),
            0.6
        );
        app.input.mods = sdl3::keyboard::Mod::LCTRLMOD;
        modifier_key(&mut app);
        assert_eq!(
            crate::transform_section::param_value(&app, TransformParam::K),
            1.0
        );
        assert_eq!(drag_mode(&app, TransformParam::K), Some(ScrubMode::Snap));
    }

    // A tap's double click resets to the default and leaves the focus nowhere.
    #[test]
    fn app_tap_double_click_resets() {
        let mut app = box_app(TransformParam::K, 278.0, 478.5);
        app.k_param = crate::url::QueryParam::new(&crate::functions::K_PARAM, "k=2");
        open(&mut app, TransformParam::K, 331.5, 494.5);
        assert_eq!(app.input.focus, field_hit(TransformParam::K));
        press_at(&mut app, TransformParam::K, 331.5, 494.5);
        release_at(&mut app, TransformParam::K, 331.5, 494.5, 2);
        assert_eq!(
            crate::transform_section::param_value(&app, TransformParam::K),
            0.0
        );
        assert_eq!(app.url.query(), "");
        assert_eq!(app.input.focus, Hit::default());
    }

    // A tap's plain click collapses the caret to the end.
    #[test]
    fn app_tap_click_collapses_the_caret() {
        let mut app = box_app(TransformParam::A, 278.0, 434.5);
        open(&mut app, TransformParam::A, 331.5, 450.5);
        assert_eq!(app.drafts[TransformParam::A].caret, [0, 4]);
        press_at(&mut app, TransformParam::A, 331.5, 450.5);
        release_at(&mut app, TransformParam::A, 331.5, 450.5, 1);
        assert_eq!(app.drafts[TransformParam::A].caret, [4, 4]);
        assert_eq!(app.input.focus, field_hit(TransformParam::A));
    }

    // A press on another box commits the focused draft first (DOM order).
    #[test]
    fn app_press_elsewhere_commits_first() {
        let mut app = box_app(TransformParam::A, 278.0, 434.5);
        app.input.regions.push(Region {
            hit: field_hit(TransformParam::K),
            r#box: clay::BoundingBox {
                x: 278.0,
                y: 478.5,
                width: 107.0,
                height: 32.0,
            },
            cursor: Cursor::EwResize,
            pressed: Cursor::EwResize,
            radius: clay::CornerRadius {
                topLeft: 0.0,
                topRight: 0.0,
                bottomLeft: 0.0,
                bottomRight: 0.0,
            },
            clip: None,
            z: 0,
        });
        open(&mut app, TransformParam::A, 331.5, 450.5);
        type_text(&mut app, "2.5");
        press_at(&mut app, TransformParam::K, 331.5, 494.5);
        assert_eq!(
            crate::transform_section::param_value(&app, TransformParam::A),
            2.5
        );
        assert!(app.scrub_hold.is_some());
        assert_eq!(app.input.focus, Hit::default());
    }

    // The wheel steps the hovered box live; while typing it passes through.
    #[test]
    fn app_wheel_steps_and_passes_through_while_typing() {
        let mut app = box_app(TransformParam::A, 278.0, 434.5);
        app.input.hover = field_hit(TransformParam::A);
        wheel_over(&mut app, -100.0);
        assert_eq!(
            crate::transform_section::param_value(&app, TransformParam::A),
            1.03
        );
        assert_eq!(app.url.query(), "");

        open(&mut app, TransformParam::A, 331.5, 450.5);
        app.input.hover = field_hit(TransformParam::A);
        wheel_over(&mut app, -100.0);
        assert_eq!(
            crate::transform_section::param_value(&app, TransformParam::A),
            1.03
        );
    }

    // Esc puts back the focus value over typed text and nudges.
    #[test]
    fn app_esc_restores() {
        use sdl3::keyboard::Keycode;
        let mut app = box_app(TransformParam::K, 278.0, 478.5);
        app.k_param = crate::url::QueryParam::new(&crate::functions::K_PARAM, "k=2");
        open(&mut app, TransformParam::K, 331.5, 494.5);
        type_text(&mut app, "7");
        let esc = crate::input::KeyEvent {
            key: Keycode::Escape,
            mods: sdl3::keyboard::Mod::empty(),
            down: true,
            repeat: false,
        };
        assert!(field_key(&mut app, &esc));
        assert_eq!(
            crate::transform_section::param_value(&app, TransformParam::K),
            2.0
        );
        assert_eq!(app.url.query(), "k=2");
        assert_eq!(app.input.focus, Hit::default());
    }

    // Arrows nudge from the typed text through the debounce.
    #[test]
    fn app_arrows_nudge() {
        use sdl3::keyboard::Keycode;
        let mut app = box_app(TransformParam::K, 278.0, 478.5);
        open(&mut app, TransformParam::K, 331.5, 494.5);
        type_text(&mut app, "2.5");
        let up = crate::input::KeyEvent {
            key: Keycode::Up,
            mods: sdl3::keyboard::Mod::empty(),
            down: true,
            repeat: false,
        };
        assert!(field_key(&mut app, &up));
        assert_eq!(
            crate::transform_section::param_value(&app, TransformParam::K),
            2.6
        );
        assert_eq!(app.url.query(), "");
        assert_eq!(app.input.focus, field_hit(TransformParam::K));
    }

    // The rest tip arms on entering the box and fires after 400 ms.
    #[test]
    fn app_rest_arms_and_fires() {
        let mut app = box_app(TransformParam::A, 278.0, 434.5);
        app.input.time = 1_000 * crate::app_loop::MS;
        app.input.hover = field_hit(TransformParam::A);
        hover_moved(&mut app, Hit::default(), field_hit(TransformParam::A));
        assert!(app.scrub_rest.is_some());
        rest_fired(&mut app);
        assert_eq!(app.scrub_resting, Some(TransformParam::A));
        // Leaving drops it.
        hover_moved(&mut app, field_hit(TransformParam::A), Hit::default());
        assert!(app.scrub_rest.is_none());
        assert!(app.scrub_resting.is_none());
    }

    // A debounced write keeps the loop awake until its deadline (or a typed
    // or nudged value would never reach the URL on a quiet page: nothing
    // else steps the params once the frame settles).
    #[test]
    fn app_pending_write_keeps_the_loop_awake() {
        use sdl3::keyboard::Keycode;
        let mut app = box_app(TransformParam::K, 278.0, 478.5);
        // On the viewer route: the headless default is the welcome route,
        // whose playing loop would keep the loop awake past the write.
        app.url.sync_from_hash("#function-viewer");
        open(&mut app, TransformParam::K, 331.5, 494.5);
        type_text(&mut app, "2.5");
        let up = crate::input::KeyEvent {
            key: Keycode::Up,
            mods: sdl3::keyboard::Mod::empty(),
            down: true,
            repeat: false,
        };
        assert!(field_key(&mut app, &up));
        app.dirty = false;
        crate::app_loop::schedule(&mut app, 0);
        assert!(app.r#loop.awake);
        crate::transform_section::params_step(&mut app, 10_000);
        assert_eq!(app.url.query(), "k=2.6");
        app.dirty = false;
        crate::app_loop::schedule(&mut app, 10_000 * crate::app_loop::MS);
        assert!(!app.r#loop.awake);
    }

    // The window losing focus ends a drag with a flush.
    #[test]
    fn app_blur_ends_the_drag() {
        let mut app = box_app(TransformParam::K, 278.0, 478.5);
        press_at(&mut app, TransformParam::K, 331.5, 494.5);
        app.input.pointer = [381.5, 494.5];
        pointer_moved(&mut app);
        focus_lost(&mut app);
        assert_eq!(app.url.query(), "k=1");
        assert!(app.scrub_hold.is_none());
        assert!(app.scrub.is_none());
    }
}
