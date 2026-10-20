//! The x-of-P slider: the track over the plane's visible range, the drag
//! that moves P, its keys, and its paint.
//!
//! Ported from
//! `src/components/views/FunctionViewer/SideBar/PointsSection/PointSlider.tsx`
//! and `PointSlider.module.css` at `96f1e84`, with `selectNothing.ts` read
//! for the press lock. The slider sits under P's row head (h8-points-row's
//! `p_point` calls [`slider`]) and spans the plane's visible x range: a
//! 20 px track (pointer cursor, no outline, `touch-action: none`) with its
//! line from `INSET` 9 to `width − INSET` at `LINE_Y` 10 (2 px, foreground
//! muted) and a 10 px zero tick (1.5 px) where x = 0 lies inside the range; a
//! thumb (an 18 px bullseye, h8-points-row's `PointMark`) at `INSET + t · run`,
//! scaled 1.12 on hover, a 3 px primary ring at 32% on focus-visible, an 8 px
//! primary halo at 26% while dragging; parked at an end with the dashed
//! parked mark when P lies past the visible range; a 22 px tip above the
//! thumb on hover, focus or drag (`x` italic 14 px serif, `= value` 13 px mono
//! tabular, 4 px above the track, clamped inside it; a 7×10 triangle when
//! parked).
//!
//! Behaviour: a left-button press (range valid) starts a drag
//! (`p_dragging`, pointer capture, no text selection until release) and jumps
//! P to the pointer (`t = clamp((x − left − INSET) / run, 0, 1)`, clamped to
//! the inward bounds, quantized to 0.01); moves while captured send the same;
//! release, cancel and the window's blur stop (`p_dragging` cleared, `p`
//! flushed). Keys on the focused track: arrows ±0.1 (×10 with Shift), Home/End
//! to the inward bounds, Enter flagging the field open (h8-points-field
//! consumes [`App::slider_edit`]). Aria: role slider "x of P", valuemin
//! `low`, valuemax `high`, valuenow the value, valuetext `format_number`.
//!
//! The pure half maps every value as a function of the visible range and the
//! track width; the `App` half below routes the SDL facts, owns the drag and
//! the edit flag, and paints the track post-pass from its laid-out box (the
//! thumb and tip are positioned at computed offsets, like the overlay button
//! is). `QUANTUM`, `quantize` and `format_number` are h8-p-points'
//! `point_format`.

use crate::app::App;
use crate::app_loop::MS;
use crate::input::{Cursor, Hit, HitKind, KeyEvent};
use crate::point_format::{QUANTUM, format_number, quantize};

/// An arrow step (`STEP`).
pub const STEP: f64 = 0.1;
/// An arrow step with Shift held (`STEP_SHIFT`).
pub const STEP_SHIFT: f64 = 1.0;

/// The track's end inset (`INSET`): the line runs `INSET..width − INSET` and
/// the thumb parks at `INSET` / `width − INSET`.
pub const TRACK_INSET: f64 = 9.0;
/// The track's height (20 px).
pub const TRACK_HEIGHT: f32 = 20.0;
/// The line's height inside the track (`LINE_Y`).
pub const TRACK_LINE_Y: f32 = 10.0;
/// The zero tick's height (`ZERO_MARK`).
pub const ZERO_TICK_HEIGHT: f32 = 10.0;
/// The track line's width (`.line`, 2 px).
pub const TRACK_LINE_WIDTH: f32 = 2.0;
/// The zero tick's width (`.zero`, 1.5 px).
pub const ZERO_TICK_WIDTH: f32 = 1.5;

/// The Clay scope of the track (h8-points-row's `SLIDER_ID`, under P's head).
pub const TRACK_ID: &str = "PointSlider";

/// The thumb's size: an 18 px mark (h8-points-row's `PointMark`, `SIZE`).
pub const THUMB_SIZE: f32 = 18.0;
/// The bullseye's ring radius and stroke (`mark_radii` for `Bullseye`).
pub const THUMB_RING_R: f32 = 8.0;
/// The bullseye's ring stroke width.
pub const THUMB_RING_W: f32 = 2.0;
/// The bullseye's core radius (`SIZE / 4`).
pub const THUMB_CORE_R: f32 = 4.5;
/// The thumb's hover scale (`.track:hover .thumb`, 1.12).
pub const THUMB_HOVER_SCALE: f32 = 1.12;
/// The focus ring's width (`.track:focus-visible .thumb`, 3 px of primary at
/// 32%, as `theme::ring` is).
pub const THUMB_RING_WIDTH: f32 = 3.0;
/// The drag halo's spread (`.track[data-dragging] .thumb::before`,
/// `inset: -8px`, primary at 26%, as `theme::plate` is).
pub const THUMB_HALO_SPREAD: f32 = 8.0;
/// The parked ring's dash pattern (`.parked`, `stroke-dasharray: 3 2`).
pub const PARKED_DASH: [f64; 2] = [3.0, 2.0];

/// The tip's height (`.tip`, 22 px).
pub const TIP_HEIGHT: f32 = 22.0;
/// The tip's horizontal padding (`.tip`, `2px 7px`).
pub const TIP_PAD_X: f32 = 7.0;
/// The tip's vertical padding.
pub const TIP_PAD_Y: f32 = 2.0;
/// The tip's gap above the track (`bottom: calc(100% + 4px)`).
pub const TIP_ABOVE: f32 = 4.0;
/// The tip's inner gap (`.tip`, `gap: 4px`).
pub const TIP_GAP: f32 = 4.0;
/// The tip's corner radius (`.tip`, 5 px).
pub const TIP_RADIUS: f32 = 5.0;
/// The tip's `x`: italic 14 px serif on an 18 px line (`var`).
pub const TIP_X_SIZE: f32 = 14.0;
/// The tip's `x` line height.
pub const TIP_X_LINE: f32 = 18.0;
/// The tip's value: 13 px mono tabular on a 17 px line (`.tip_value`).
pub const TIP_VALUE_SIZE: f32 = 13.0;
/// The tip's value line height.
pub const TIP_VALUE_LINE: f32 = 17.0;
/// The parked triangle's size (the tip's `Triangle`, 7×10).
pub const TIP_TRIANGLE_SIZE: [f32; 2] = [7.0, 10.0];
/// The parked-left triangle's path (`M7 0L0 5L7 10Z`).
pub const TIP_TRIANGLE_LEFT: &str = "M7 0L0 5L7 10Z";
/// The parked-right triangle's path (`M0 0L7 5L0 10Z`).
pub const TIP_TRIANGLE_RIGHT: &str = "M0 0L7 5L0 10Z";

/// Which end P lies past (`data-parked`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParkSide {
    Left,
    Right,
}

/// The slider's range: the plane's visible x extent and its inward 0.01
/// bounds (`low` the first 0.01 multiple at or above `min_x`, `high` the last
/// at or below `max_x`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SliderRange {
    pub min_x: f64,
    pub max_x: f64,
    pub low: f64,
    pub high: f64,
}

impl SliderRange {
    /// The range over the visible extent, or `None` when it is not a positive
    /// finite span (`valid` in the source).
    #[must_use]
    pub fn from_visible(min_x: f64, max_x: f64) -> Option<Self> {
        let span = max_x - min_x;
        if !span.is_finite() || span <= 0.0 {
            return None;
        }
        Some(Self {
            min_x,
            max_x,
            low: inward_low(min_x),
            high: inward_high(max_x),
        })
    }

    /// The visible span (`maxX − minX`).
    #[must_use]
    pub fn span(self) -> f64 {
        self.max_x - self.min_x
    }
}

/// The first 0.01 multiple at or above `min_x` (`inward(v, "low")`).
#[must_use]
pub fn inward_low(min_x: f64) -> f64 {
    quantize((min_x / QUANTUM - 1e-9).ceil() * QUANTUM)
}

/// The last 0.01 multiple at or below `max_x` (`inward(v, "high")`).
#[must_use]
pub fn inward_high(max_x: f64) -> f64 {
    quantize((max_x / QUANTUM + 1e-9).floor() * QUANTUM)
}

/// CSS-px rounding (`px`).
#[must_use]
pub fn px(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

/// The thumb travel (`run = width − 2 · INSET`).
#[must_use]
pub fn run(width: f64) -> f64 {
    width - 2.0 * TRACK_INSET
}

/// Which end P lies past, if any (`parked`).
#[must_use]
pub fn parked(value: f64, range: SliderRange) -> Option<ParkSide> {
    if value < range.min_x {
        Some(ParkSide::Left)
    } else if value > range.max_x {
        Some(ParkSide::Right)
    } else {
        None
    }
}

/// The thumb's `left` in track px (`thumbX`), px-rounded, or `None` when the
/// track has no travel.
#[must_use]
pub fn thumb_x(value: f64, range: SliderRange, width: f64) -> Option<f64> {
    if run(width) <= 0.0 {
        return None;
    }
    Some(px(match parked(value, range) {
        Some(ParkSide::Left) => TRACK_INSET,
        Some(ParkSide::Right) => width - TRACK_INSET,
        None => TRACK_INSET + (value - range.min_x) / range.span() * run(width),
    }))
}

/// The zero tick's x in track px (`zeroX`), px-rounded, or `None` when x = 0
/// lies outside the range or the track has no travel.
#[must_use]
pub fn zero_x(range: SliderRange, width: f64) -> Option<f64> {
    if run(width) <= 0.0 || range.min_x > 0.0 || range.max_x < 0.0 {
        return None;
    }
    Some(px(TRACK_INSET + (-range.min_x) / range.span() * run(width)))
}

/// P for a pointer at `client_x` over a track at `track_left` of `width` px
/// (`sendFromClientX`): the clamped, inward-bounded, quantized value, or
/// `None` when the track is too narrow to hold the insets.
#[must_use]
pub fn value_from_client_x(
    client_x: f64,
    track_left: f64,
    width: f64,
    range: SliderRange,
) -> Option<f64> {
    if width <= 2.0 * TRACK_INSET {
        return None;
    }
    let t = ((client_x - track_left - TRACK_INSET) / run(width)).clamp(0.0, 1.0);
    Some(quantize(
        (range.min_x + t * range.span()).clamp(range.low, range.high),
    ))
}

/// A key the track handles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SliderKey {
    ArrowRight,
    ArrowUp,
    ArrowLeft,
    ArrowDown,
    Home,
    End,
    Enter,
}

/// What a handled key does: send a value (`setValue`), or open the x field
/// (`onEditRequest`, h8-points-field's `open`, one call). Home/End off a
/// valid range are `Ignored`, as the source guards them on `valid`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum KeyOutcome {
    Send(f64),
    OpenField,
    Ignored,
}

/// The key's effect on the value (`onKeyDown`).
#[must_use]
pub fn apply_key(
    key: SliderKey,
    shift: bool,
    value: f64,
    range: Option<SliderRange>,
) -> KeyOutcome {
    match key {
        SliderKey::ArrowRight | SliderKey::ArrowUp => {
            KeyOutcome::Send(quantize(value + direction(1.0, shift)))
        }
        SliderKey::ArrowLeft | SliderKey::ArrowDown => {
            KeyOutcome::Send(quantize(value + direction(-1.0, shift)))
        }
        SliderKey::Home => match range {
            Some(range) => KeyOutcome::Send(range.low),
            None => KeyOutcome::Ignored,
        },
        SliderKey::End => match range {
            Some(range) => KeyOutcome::Send(range.high),
            None => KeyOutcome::Ignored,
        },
        SliderKey::Enter => KeyOutcome::OpenField,
    }
}

/// An arrow step: ±`STEP`, ×10 with Shift (`STEP_SHIFT`).
fn direction(sign: f64, shift: bool) -> f64 {
    sign * if shift { STEP_SHIFT } else { STEP }
}

/// A drag in flight: the pointer that pressed, so no other one hijacks it
/// (`pointerId` in the source). Capture, the select-nothing lock and the
/// window-blur stop ride the input layer (h5-box's model).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SliderDrag {
    pub pointer_id: u64,
}

impl SliderDrag {
    /// A drag starts: the press takes the pointer.
    #[must_use]
    pub fn begin(pointer_id: u64) -> Self {
        Self { pointer_id }
    }

    /// Whether the fact's pointer is the drag's (every move/up arm checks
    /// `pointerId.current !== e.pointerId` first).
    #[must_use]
    pub fn accepts(self, pointer_id: u64) -> bool {
        self.pointer_id == pointer_id
    }
}

/// The track's hit: the whole 20 px box (the thumb takes no pointer events).
#[must_use]
pub fn slider_hit() -> Hit {
    Hit {
        kind: HitKind::PointSlider,
        index: 0,
    }
}

/// The slider's live range: the plane's visible x extent over the last
/// frame's content box at the live camera, as `visibleRangeAtom` reads the
/// canvas size (`planeSizeAtom`) with the centre and scale. `None` before
/// the first layout or off a degenerate span, when the source's press
/// returns early.
#[must_use]
pub fn slider_range(app: &App) -> Option<SliderRange> {
    crate::visible_range::live(app)
        .and_then(|range| SliderRange::from_visible(range.min_x, range.max_x))
}

/// Whether the pointer hovers the track.
#[must_use]
pub fn slider_hovered(app: &App) -> bool {
    app.input.hover == slider_hit()
}

/// Whether the keyboard put the focus on the track.
#[must_use]
pub fn slider_focused(app: &App) -> bool {
    app.input.focus == slider_hit()
}

/// Whether a slider drag holds the pointer: moves route to the drag instead
/// of retargeting, as `setPointerCapture` keeps the hook's moves coming.
/// Every press starts a drag or nothing (no tap arm: the press already
/// jumped), so the hold alone is the capture.
#[must_use]
pub fn slider_captured(app: &App) -> bool {
    app.slider_hold.is_some()
}

/// The track's laid-out box, for the drag mapping and the paint pass.
fn laid_out(app: &App) -> Option<mv_sys::clay::BoundingBox> {
    let data = app
        .clay
        .as_ref()
        .expect("clay context initialised")
        .get_element_data(mv_clay::id(TRACK_ID, 0));
    data.found.then_some(data.boundingBox)
}

/// The x-of-P slider under P's head: a grow-wide 20 px track declaring the
/// pointer region and the tab stop (h8-points-row's `p_point` calls this;
/// the stand-in comment there names this scope).
pub fn slider(app: &mut App) {
    let mut decl = crate::ui::empty_decl();
    decl.layout.sizing = mv_sys::clay::Sizing {
        width: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
        height: mv_clay::sizing_fixed(TRACK_HEIGHT),
    };
    crate::ui::ui_scope(app, mv_clay::id(TRACK_ID, 0), &decl, |_| {});
    crate::input::region(
        app,
        mv_clay::id(TRACK_ID, 0),
        slider_hit(),
        Cursor::Pointer,
        Cursor::Pointer,
        true,
        crate::ui::corner_radius_all(0.0),
    );
}

/// Sends the pointer's value: the live write, repainting on a change (the
/// hook's `send`, debounced through the loop's shared param step).
fn send_at(app: &mut App, range: SliderRange, track_x: f64, track_w: f64, client_x: f64) {
    let Some(next) = value_from_client_x(client_x, track_x, track_w, range) else {
        return;
    };
    live_p(app, next);
}

/// Writes the live value (debounced), repainting on a change.
fn live_p(app: &mut App, next: f64) {
    if *app.p_param.get() == next {
        return;
    }
    let now_ms = (app.input.time / MS) as u64;
    app.p_param.set(next, now_ms);
    app.dirty = true;
}

/// Routes a press on the track (`press_down`): a press elsewhere is not
/// ours. The track takes the keyboard focus without the ring (a mouse press
/// focuses but never matches `:focus-visible`); a valid range starts the
/// drag and jumps P to the pointer, as the press both captures and sends.
pub fn press_down(app: &mut App) {
    if app.input.pressed.kind != HitKind::PointSlider {
        return;
    }
    crate::input::set_focus(app, slider_hit(), false);
    let (Some(range), Some(track)) = (slider_range(app), laid_out(app)) else {
        return;
    };
    if app.slider_hold.is_some() {
        return;
    }
    app.slider_hold = Some(SliderDrag::begin(crate::plane_input::MOUSE_POINTER_ID));
    app.state.p_dragging = true;
    send_at(
        app,
        range,
        f64::from(track.x),
        f64::from(track.width),
        f64::from(app.input.pointer[0]),
    );
    app.dirty = true;
}

/// Continues the held drag at the pointer (a move while captured).
pub fn pointer_moved(app: &mut App) {
    let Some(hold) = app.slider_hold else {
        return;
    };
    if !hold.accepts(crate::plane_input::MOUSE_POINTER_ID) {
        return;
    }
    let (Some(range), Some(track)) = (slider_range(app), laid_out(app)) else {
        return;
    };
    send_at(
        app,
        range,
        f64::from(track.x),
        f64::from(track.width),
        f64::from(app.input.pointer[0]),
    );
}

/// Ends the held drag at the release: `p_dragging` clears and `p` flushes
/// (the source's `stop`: the release sends nothing, the last move did).
/// With pointer capture the release lands on the track wherever the pointer
/// is, so any release ends an active hold.
pub fn release_up(app: &mut App) {
    if app.slider_hold.is_none() {
        return;
    }
    app.slider_hold = None;
    app.state.p_dragging = false;
    app.p_param.flush(&mut app.url);
    app.dirty = true;
}

/// The window lost the focus: a held drag ends, flushed (the window's blur
/// listener runs the source's `stop`).
pub fn focus_lost(app: &mut App) {
    release_up(app);
}

/// Routes a key to the focused track. Arrows and Home/End send through the
/// debounce; Enter flags the x field open for h8-points-field
/// ([`App::slider_edit`]); Tab walks on in the input layer's chain. Returns
/// whether consumed.
pub fn key(app: &mut App, event: &KeyEvent) -> bool {
    use sdl3::keyboard::Keycode;
    if !event.down || event.repeat {
        return false;
    }
    if app.input.focus != slider_hit() {
        return false;
    }
    let outcome = match event.key {
        Keycode::Right => apply_key(
            SliderKey::ArrowRight,
            shift_held(event),
            *app.p_param.get(),
            slider_range(app),
        ),
        Keycode::Up => apply_key(
            SliderKey::ArrowUp,
            shift_held(event),
            *app.p_param.get(),
            slider_range(app),
        ),
        Keycode::Left => apply_key(
            SliderKey::ArrowLeft,
            shift_held(event),
            *app.p_param.get(),
            slider_range(app),
        ),
        Keycode::Down => apply_key(
            SliderKey::ArrowDown,
            shift_held(event),
            *app.p_param.get(),
            slider_range(app),
        ),
        Keycode::Home => apply_key(
            SliderKey::Home,
            false,
            *app.p_param.get(),
            slider_range(app),
        ),
        Keycode::End => apply_key(SliderKey::End, false, *app.p_param.get(), slider_range(app)),
        Keycode::Return | Keycode::KpEnter => KeyOutcome::OpenField,
        _ => return false,
    };
    match outcome {
        KeyOutcome::Send(next) => live_p(app, next),
        KeyOutcome::OpenField => {
            app.slider_edit = true;
            app.dirty = true;
        }
        // Off a valid range Home/End do nothing: the source guards them on
        // `valid`, so the key falls through unconsumed.
        KeyOutcome::Ignored => return false,
    }
    true
}

/// Whether Shift is held (the ×10 modifier).
fn shift_held(event: &KeyEvent) -> bool {
    event
        .mods
        .intersects(sdl3::keyboard::Mod::LSHIFTMOD | sdl3::keyboard::Mod::RSHIFTMOD)
}

/// The thumb's centre in window px, or `None` off a track with no travel.
#[must_use]
pub fn thumb_centre(
    track: &mv_sys::clay::BoundingBox,
    range: SliderRange,
    value: f64,
) -> Option<[f32; 2]> {
    let left = thumb_x(value, range, f64::from(track.width))?;
    Some([track.x + left as f32, track.y + TRACK_LINE_Y])
}

/// The thumb's box: the 18 px mark centred on the thumb's centre (the span
/// shrink-wraps the svg, translated −50%).
#[must_use]
pub fn thumb_box(
    track: &mv_sys::clay::BoundingBox,
    range: SliderRange,
    value: f64,
) -> Option<mv_sys::clay::BoundingBox> {
    let [cx, cy] = thumb_centre(track, range, value)?;
    Some(mv_sys::clay::BoundingBox {
        x: cx - THUMB_SIZE / 2.0,
        y: cy - THUMB_SIZE / 2.0,
        width: THUMB_SIZE,
        height: THUMB_SIZE,
    })
}

/// The tip's box and its text pen: the measured width centred on the thumb
/// and clamped inside the track (the `translate: clamp(...)` rule), 4 px
/// above the track, 22 px tall.
pub struct TipGeom {
    pub r#box: mv_sys::clay::BoundingBox,
    /// The pen x of the `x` run (after the left padding and triangle).
    pub x_pen: f32,
    /// The pen x of the `= value` run.
    pub value_pen: f32,
    /// The baselines of the `x` and value runs.
    pub x_base: f32,
    pub value_base: f32,
    /// The triangle's box, when parked.
    pub triangle: Option<mv_sys::clay::BoundingBox>,
}

/// The tip's text width: padding, the `x` and `= value` runs with their gap,
/// and the parked triangle with its gap.
fn tip_text_width(app: &mut App, value: f64, side: Option<ParkSide>) -> (f32, f32, f32) {
    let x_adv = mv_text::measure(
        &mut app.fonts,
        mv_text::Style {
            face: mv_text::Face::Stix400Italic,
            size: TIP_X_SIZE,
            tracking: 0.0,
            features: mv_text::Features::new(),
        },
        "x",
    );
    let mut features = mv_text::Features::new();
    features.insert(mv_text::Feature::Tabular);
    let value_adv = mv_text::measure(
        &mut app.fonts,
        mv_text::Style {
            face: mv_text::Face::RobotoMono500,
            size: TIP_VALUE_SIZE,
            tracking: 0.0,
            features,
        },
        &format!("= {}", format_number(value)),
    );
    let mut width = 2.0 * TIP_PAD_X + x_adv + TIP_GAP + value_adv;
    if side.is_some() {
        width += TIP_TRIANGLE_SIZE[0] + TIP_GAP;
    }
    (width, x_adv, value_adv)
}

/// Lays out the tip over the track: centred on the thumb, clamped inside.
#[must_use]
pub fn tip_geom(
    app: &mut App,
    track: &mv_sys::clay::BoundingBox,
    range: SliderRange,
    value: f64,
) -> Option<TipGeom> {
    let left = thumb_x(value, range, f64::from(track.width))?;
    let side = parked(value, range);
    let (width, x_adv, value_adv) = tip_text_width(app, value, side);
    if width > track.width {
        return None;
    }
    let centre = track.x + left as f32;
    let x = (centre - width / 2.0).clamp(track.x, track.x + track.width - width);
    let y = track.y - TIP_ABOVE - TIP_HEIGHT;
    let content_top = y + TIP_PAD_Y;
    let mut pen = x + TIP_PAD_X;
    let triangle_left = if side == Some(ParkSide::Left) {
        let r#box = mv_sys::clay::BoundingBox {
            x: pen,
            y: content_top + (TIP_X_LINE - TIP_TRIANGLE_SIZE[1]) / 2.0,
            width: TIP_TRIANGLE_SIZE[0],
            height: TIP_TRIANGLE_SIZE[1],
        };
        pen += TIP_TRIANGLE_SIZE[0] + TIP_GAP;
        Some(r#box)
    } else {
        None
    };
    let x_pen = pen;
    let x_base = content_top
        + mv_text::baseline(
            &mut app.fonts,
            mv_text::Face::Stix400Italic,
            TIP_X_SIZE,
            TIP_X_LINE,
        );
    pen += x_adv + TIP_GAP;
    let value_pen = pen;
    let value_base = content_top
        + (TIP_X_LINE - TIP_VALUE_LINE) / 2.0
        + mv_text::baseline(
            &mut app.fonts,
            mv_text::Face::RobotoMono500,
            TIP_VALUE_SIZE,
            TIP_VALUE_LINE,
        );
    pen += value_adv;
    let triangle_right = if side == Some(ParkSide::Right) {
        pen += TIP_GAP;
        Some(mv_sys::clay::BoundingBox {
            x: pen,
            y: content_top + (TIP_X_LINE - TIP_TRIANGLE_SIZE[1]) / 2.0,
            width: TIP_TRIANGLE_SIZE[0],
            height: TIP_TRIANGLE_SIZE[1],
        })
    } else {
        None
    };
    Some(TipGeom {
        r#box: mv_sys::clay::BoundingBox {
            x,
            y,
            width,
            height: TIP_HEIGHT,
        },
        x_pen,
        value_pen,
        x_base,
        value_base,
        triangle: triangle_left.or(triangle_right),
    })
}

/// Whether the tip shows: on hover, keyboard focus or while dragging (the
/// three selectors showing `.tip`).
#[must_use]
pub fn tip_shows(app: &App) -> bool {
    slider_hovered(app) || slider_focused(app) || app.state.p_dragging
}

/// Paints the slider over its laid-out track: the line, the zero tick, the
/// thumb (halo, ring, mark) and the tip (pill, runs, triangle).
pub fn paint_slider(app: &mut App) {
    let theme = crate::ui::current_theme(app).clone();
    let (Some(track), Some(range)) = (laid_out(app), slider_range(app)) else {
        return;
    };
    let value = *app.p_param.get();
    paint_track(app, &theme, &track, range);
    paint_thumb(app, &theme, &track, range, value);
    if tip_shows(app) {
        paint_tip(app, &theme, &track, range, value);
    }
}

/// The track's line and zero tick (`.marks`).
fn paint_track(
    app: &mut App,
    theme: &mv_theme::Theme,
    track: &mv_sys::clay::BoundingBox,
    range: SliderRange,
) {
    let ink = crate::draw::nvg_color(crate::ui::ink(theme, mv_theme::Token::ForegroundMuted));
    let y = track.y + TRACK_LINE_Y;
    app.vg.begin_path();
    app.vg.move_to(track.x + TRACK_INSET as f32, y);
    app.vg
        .line_to(track.x + track.width - TRACK_INSET as f32, y);
    app.vg.stroke_width(TRACK_LINE_WIDTH);
    app.vg.stroke_color(ink);
    app.vg.stroke();
    if let Some(zx) = zero_x(range, f64::from(track.width)) {
        let x = track.x + zx as f32;
        app.vg.begin_path();
        app.vg.move_to(x, y - ZERO_TICK_HEIGHT / 2.0);
        app.vg.line_to(x, y + ZERO_TICK_HEIGHT / 2.0);
        app.vg.stroke_width(ZERO_TICK_WIDTH);
        app.vg.stroke_color(ink);
        app.vg.stroke();
    }
}

/// The thumb: the drag halo, the focus ring and the bullseye (the parked
/// dashed ring with no core when P lies past the range).
fn paint_thumb(
    app: &mut App,
    theme: &mv_theme::Theme,
    track: &mv_sys::clay::BoundingBox,
    range: SliderRange,
    value: f64,
) {
    let Some([cx, cy]) = thumb_centre(track, range, value) else {
        return;
    };
    let dragging = app.state.p_dragging;
    let scale = if slider_hovered(app) && !dragging {
        THUMB_HOVER_SCALE
    } else {
        1.0
    };
    // Primary at 26%/32% over the card, as `theme::plate`/`theme::ring`
    // hold it: `ui::ink` first (0..1 to 0..255), as every other `nvg_color`
    // call site does — the helpers' raw output is not a NanoVG colour.
    let primary = crate::ui::ink(theme, mv_theme::Token::Primary);
    if dragging {
        app.vg.begin_path();
        app.vg.circle(cx, cy, THUMB_SIZE / 2.0 + THUMB_HALO_SPREAD);
        app.vg.fill_color(crate::draw::nvg_color([
            primary[0],
            primary[1],
            primary[2],
            primary[3] * 0.26,
        ]));
        app.vg.fill();
    }
    if slider_focused(app) && app.input.focus_visible {
        app.vg.begin_path();
        app.vg
            .circle(cx, cy, THUMB_SIZE / 2.0 + THUMB_RING_WIDTH / 2.0);
        app.vg.stroke_width(THUMB_RING_WIDTH);
        app.vg.stroke_color(crate::draw::nvg_color([
            primary[0],
            primary[1],
            primary[2],
            primary[3] * 0.32,
        ]));
        app.vg.stroke();
    }
    let side = parked(value, range);
    let ring_r = THUMB_RING_R * scale;
    let ring_w = THUMB_RING_W * scale;
    // The mark fills its background, as `PointMark` does.
    app.vg.begin_path();
    app.vg.circle(cx, cy, ring_r);
    app.vg.fill_color(crate::draw::nvg_color(crate::ui::ink(
        theme,
        mv_theme::Token::Background,
    )));
    app.vg.fill();
    if side.is_some() {
        paint_dashed_ring(app, theme, cx, cy, ring_r, ring_w);
        return;
    }
    app.vg.begin_path();
    app.vg.circle(cx, cy, ring_r);
    app.vg.stroke_width(ring_w);
    app.vg.stroke_color(crate::draw::nvg_color(crate::ui::ink(
        theme,
        mv_theme::Token::Foreground,
    )));
    app.vg.stroke();
    app.vg.begin_path();
    app.vg.circle(cx, cy, THUMB_CORE_R * scale);
    app.vg
        .fill_color(crate::draw::nvg_color(thumb_point_ink(theme)));
    app.vg.fill();
}

/// The thumb's point ink (`PointP`, h8-plane's token).
fn thumb_point_ink(theme: &mv_theme::Theme) -> mv_sys::clay::Color {
    crate::ui::ink(theme, mv_theme::Token::PointP)
}

/// The parked thumb's ring: point-p ink dashed 3 on 2 (`stroke-dasharray`),
/// sampled round the circle and cut into on-intervals (the plane's dashed
/// curves cut their polylines the same way).
fn paint_dashed_ring(app: &mut App, theme: &mv_theme::Theme, cx: f32, cy: f32, r: f32, w: f32) {
    const SEGMENTS: usize = 72;
    let mut points = Vec::with_capacity(SEGMENTS + 1);
    for k in 0..=SEGMENTS {
        let a = k as f64 / SEGMENTS as f64 * std::f64::consts::TAU;
        points.push([
            f64::from(cx) + f64::from(r) * a.cos(),
            f64::from(cy) + f64::from(r) * a.sin(),
        ]);
    }
    let ink = crate::draw::nvg_color(thumb_point_ink(theme));
    for piece in crate::plane::dash_pieces(&points, PARKED_DASH, 0.0) {
        let mut piece = piece.into_iter();
        let Some(first) = piece.next() else {
            continue;
        };
        app.vg.begin_path();
        app.vg.move_to(first[0] as f32, first[1] as f32);
        for p in piece {
            app.vg.line_to(p[0] as f32, p[1] as f32);
        }
        app.vg.stroke_width(w);
        app.vg.stroke_color(ink);
        app.vg.stroke();
    }
}

/// The tip above the thumb: the foreground pill, the `x` and `= value`
/// runs, and the parked triangle in the background ink.
fn paint_tip(
    app: &mut App,
    theme: &mv_theme::Theme,
    track: &mv_sys::clay::BoundingBox,
    range: SliderRange,
    value: f64,
) {
    let Some(geom) = tip_geom(app, track, range, value) else {
        return;
    };
    app.vg.begin_path();
    crate::draw::rounded_rect(
        &mut app.vg,
        geom.r#box,
        crate::ui::corner_radius_all(TIP_RADIUS),
    );
    app.vg.fill_color(crate::draw::nvg_color(crate::ui::ink(
        theme,
        mv_theme::Token::Foreground,
    )));
    app.vg.fill();
    let ink = crate::draw::nvg_color(crate::ui::ink(theme, mv_theme::Token::Background));
    mv_text::draw(
        &mut app.fonts,
        &mut app.vg,
        mv_text::Style {
            face: mv_text::Face::Stix400Italic,
            size: TIP_X_SIZE,
            tracking: 0.0,
            features: mv_text::Features::new(),
        },
        geom.x_pen,
        geom.x_base,
        "x",
        ink,
    );
    let mut features = mv_text::Features::new();
    features.insert(mv_text::Feature::Tabular);
    mv_text::draw(
        &mut app.fonts,
        &mut app.vg,
        mv_text::Style {
            face: mv_text::Face::RobotoMono500,
            size: TIP_VALUE_SIZE,
            tracking: 0.0,
            features,
        },
        geom.value_pen,
        geom.value_base,
        &format!("= {}", format_number(value)),
        ink,
    );
    if let Some(tri) = geom.triangle {
        let parked_left = parked(value, range) == Some(ParkSide::Left);
        let corners: [[f32; 2]; 3] = if parked_left {
            [[7.0, 0.0], [0.0, 5.0], [7.0, 10.0]]
        } else {
            [[0.0, 0.0], [7.0, 5.0], [0.0, 10.0]]
        };
        app.vg.begin_path();
        app.vg.move_to(tri.x + corners[0][0], tri.y + corners[0][1]);
        app.vg.line_to(tri.x + corners[1][0], tri.y + corners[1][1]);
        app.vg.line_to(tri.x + corners[2][0], tri.y + corners[2][1]);
        app.vg.close_path();
        app.vg.fill_color(ink);
        app.vg.fill();
    }
}

/// The tip's left edge in track px: centred on the thumb, clamped inside.
#[must_use]
pub fn tip_left(thumb_left: f64, track_width: f64, tip_width: f64) -> f64 {
    (thumb_left - tip_width / 2.0).clamp(0.0, track_width - tip_width)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The 500 × 400 fixture shows x from −5 to 5 (default camera).
    fn fixture() -> SliderRange {
        SliderRange::from_visible(-5.0, 5.0).expect("fixture span")
    }

    #[test]
    fn spans_the_planes_visible_x_range_and_follows_the_camera() {
        // −5..5 at the 500 × 400 fixture; −10..10 at scale 1; −7..13 at
        // centre [3, 0] (scale 1 still on: the sequential test state).
        let range = fixture();
        assert_eq!((range.low, range.high), (-5.0, 5.0));
        let zoomed = SliderRange::from_visible(-10.0, 10.0).expect("zoomed span");
        assert_eq!((zoomed.low, zoomed.high), (-10.0, 10.0));
        let panned = SliderRange::from_visible(-7.0, 13.0).expect("panned span");
        assert_eq!((panned.low, panned.high), (-7.0, 13.0));
    }

    #[test]
    fn jumps_p_to_the_pointer_follows_the_drag_and_writes_the_url_on_letting_go() {
        // The track's 218 px leave a 200 px run between its 9 px insets,
        // 20 px per unit. A press at x 169 gives 3 and starts the drag; a
        // move to 59 gives −2.5 with the URL unwritten; to −400 gives −5;
        // the release writes p=−5 and clears p_dragging (URL + flag at the
        // App layer, below).
        let range = fixture();
        assert_eq!(value_from_client_x(169.0, 0.0, 218.0, range), Some(3.0));
        assert_eq!(value_from_client_x(59.0, 0.0, 218.0, range), Some(-2.5));
        assert_eq!(value_from_client_x(-400.0, 0.0, 218.0, range), Some(-5.0));
        let drag = SliderDrag::begin(1);
        assert!(drag.accepts(1));
        assert!(!drag.accepts(2));
    }

    #[test]
    fn steps_0_1_with_the_arrow_keys_and_1_with_shift() {
        let range = Some(fixture());
        assert_eq!(
            apply_key(SliderKey::ArrowRight, false, 2.0, range),
            KeyOutcome::Send(2.1)
        );
        assert_eq!(
            apply_key(SliderKey::ArrowDown, true, 2.1, range),
            KeyOutcome::Send(1.1)
        );
    }

    #[test]
    fn jumps_to_the_ends_of_the_visible_range_with_home_and_end() {
        // Centre 0.123 at the fixture: −4.877..5.123, inward −4.87..5.12.
        let range = SliderRange::from_visible(-4.877, 5.123).expect("span");
        assert_eq!((range.low, range.high), (-4.87, 5.12));
        assert_eq!(
            apply_key(SliderKey::Home, false, 2.0, Some(range)),
            KeyOutcome::Send(-4.87)
        );
        assert_eq!(
            apply_key(SliderKey::End, false, 2.0, Some(range)),
            KeyOutcome::Send(5.12)
        );
        assert_eq!(
            apply_key(SliderKey::Home, false, 2.0, None),
            KeyOutcome::Ignored
        );
    }

    #[test]
    fn parks_the_thumb_on_the_end_p_lies_past() {
        // Query p=8 on −5..5 parks right; at scale 1 (−10..10) it unparks.
        let range = fixture();
        assert_eq!(parked(8.0, range), Some(ParkSide::Right));
        assert_eq!(thumb_x(8.0, range, 218.0), Some(209.0));
        let zoomed = SliderRange::from_visible(-10.0, 10.0).expect("span");
        assert_eq!(parked(8.0, zoomed), None);
        assert_eq!(
            thumb_x(8.0, zoomed, 218.0),
            Some(px(TRACK_INSET + (8.0 + 10.0) / 20.0 * run(218.0)))
        );
    }

    #[test]
    fn thumb_centres_on_the_value_at_the_scripted_frame() {
        // WebKit probe at 1440×900, default camera: ±9.9 over a 350 px
        // track; P=2 sits at left 208.54px.
        let range = SliderRange::from_visible(-9.9, 9.9).expect("span");
        assert_eq!((range.low, range.high), (-9.9, 9.9));
        assert_eq!(thumb_x(2.0, range, 350.0), Some(208.54));
    }

    #[test]
    fn zero_tick_sits_where_x_is_zero() {
        let range = SliderRange::from_visible(-9.9, 9.9).expect("span");
        assert_eq!(zero_x(range, 350.0), Some(175.0));
        let shifted = SliderRange::from_visible(1.0, 11.0).expect("span");
        assert_eq!(zero_x(shifted, 350.0), None);
    }

    #[test]
    fn parked_left_thumb_sits_at_the_inset() {
        // WebKit probe: ?p=−50 parks left, thumb at 9px.
        let range = SliderRange::from_visible(-9.9, 9.9).expect("span");
        assert_eq!(parked(-50.0, range), Some(ParkSide::Left));
        assert_eq!(thumb_x(-50.0, range, 350.0), Some(9.0));
    }

    #[test]
    fn press_jump_mapping_at_the_scripted_frame() {
        // A press at clientX 100 over the track at x=41 (350 px) lands at
        // −6.92.
        let range = SliderRange::from_visible(-9.9, 9.9).expect("span");
        assert_eq!(value_from_client_x(100.0, 41.0, 350.0, range), Some(-6.92));
    }

    #[test]
    fn drag_30px_from_the_thumb() {
        // Pressing the thumb centre (track 41 + 208.54) holds ≈2; +30 px
        // gives 3.79 with the URL unwritten until release.
        let range = SliderRange::from_visible(-9.9, 9.9).expect("span");
        assert_eq!(value_from_client_x(249.54, 41.0, 350.0, range), Some(2.0));
        assert_eq!(value_from_client_x(279.54, 41.0, 350.0, range), Some(3.79));
    }

    #[test]
    fn aria_text_uses_the_true_minus() {
        assert_eq!(format_number(2.0), "2.00");
        assert_eq!(format_number(-50.0), "\u{2212}50.00");
    }

    #[test]
    fn narrow_tracks_hold_no_drag() {
        let range = fixture();
        assert_eq!(value_from_client_x(9.0, 0.0, 18.0, range), None);
        assert_eq!(thumb_x(2.0, range, 18.0), None);
    }

    #[test]
    fn degenerate_ranges_are_invalid() {
        assert_eq!(SliderRange::from_visible(0.0, 0.0), None);
        assert_eq!(SliderRange::from_visible(5.0, -5.0), None);
        assert_eq!(SliderRange::from_visible(f64::NAN, 5.0), None);
    }

    #[test]
    fn enter_opens_the_field() {
        assert_eq!(
            apply_key(SliderKey::Enter, false, 2.0, Some(fixture())),
            KeyOutcome::OpenField
        );
    }

    #[test]
    fn layout_matches_the_css() {
        assert_eq!(
            (TRACK_INSET, TRACK_LINE_Y, ZERO_TICK_HEIGHT),
            (9.0, 10.0, 10.0)
        );
        assert_eq!((TRACK_LINE_WIDTH, ZERO_TICK_WIDTH), (2.0, 1.5));
        assert_eq!(TRACK_HEIGHT, 20.0);
        assert_eq!((STEP, STEP_SHIFT), (0.1, 1.0));
        assert_eq!(
            (TIP_HEIGHT, TIP_PAD_X, TIP_PAD_Y, TIP_ABOVE, TIP_GAP),
            (22.0, 7.0, 2.0, 4.0, 4.0)
        );
        assert_eq!(
            (TIP_X_SIZE, TIP_X_LINE, TIP_VALUE_SIZE, TIP_VALUE_LINE),
            (14.0, 18.0, 13.0, 17.0)
        );
        assert_eq!(
            (THUMB_SIZE, THUMB_RING_R, THUMB_RING_W, THUMB_CORE_R),
            (18.0, 8.0, 2.0, 4.5)
        );
        assert_eq!(
            (THUMB_HOVER_SCALE, THUMB_RING_WIDTH, THUMB_HALO_SPREAD),
            (1.12, 3.0, 8.0)
        );
        assert_eq!(PARKED_DASH, [3.0, 2.0]);
        assert_eq!(TIP_TRIANGLE_SIZE, [7.0, 10.0]);
        assert_eq!(
            (TIP_TRIANGLE_LEFT, TIP_TRIANGLE_RIGHT),
            ("M7 0L0 5L7 10Z", "M0 0L7 5L0 10Z")
        );
    }

    #[test]
    fn thumb_box_wraps_the_mark_on_its_centre() {
        // The thumb span shrink-wraps the 18 px svg about the centre: the
        // WebKit thumb box (240.53, 738.5, 18, 18) at left 208.54px.
        let track = mv_sys::clay::BoundingBox {
            x: 41.0,
            y: 737.5,
            width: 350.0,
            height: 20.0,
        };
        let range = SliderRange::from_visible(-9.9, 9.9).expect("span");
        let r#box = thumb_box(&track, range, 2.0).expect("thumb");
        for (got, want) in [
            (r#box.x, 240.54),
            (r#box.y, 738.5),
            (r#box.width, 18.0),
            (r#box.height, 18.0),
        ] {
            assert!((got - want).abs() < 1e-4, "thumb {got} against {want}");
        }
        let centre = thumb_centre(&track, range, 2.0).expect("centre");
        assert!((centre[0] - 249.54).abs() < 1e-4, "centre {}", centre[0]);
        assert!((centre[1] - 747.5).abs() < 1e-4, "centre {}", centre[1]);
    }

    #[test]
    fn tip_clamps_inside_the_track() {
        // Centred on the thumb, held inside the track: the hover tip at
        // 208.54 sits at 172.96; the parked-left tip clamps to the track's
        // left edge; the parked-right tip to its right edge.
        for (got, want) in [
            (tip_left(208.54, 350.0, 71.16), 172.96),
            (tip_left(9.0, 350.0, 97.77), 0.0),
            (tip_left(341.0, 350.0, 82.16), 267.84),
        ] {
            assert!((got - want).abs() < 1e-9, "tip_left {got} against {want}");
        }
    }

    #[test]
    fn boxes_match_fixture() {
        const SLACK: f64 = 0.5;
        let fixture: serde_json::Value =
            serde_json::from_str(crate::fixture::PAGE_FIXTURE).expect("page fixture parses");
        let shots = &fixture["shots"];
        let read = |shot: &str, name: &str| -> [f64; 4] {
            let boxes = &shots[shot]["webkit"]["boxes"];
            let got = boxes[name].as_array().expect("box");
            [
                got[0].as_f64().expect("x"),
                got[1].as_f64().expect("y"),
                got[2].as_f64().expect("w"),
                got[3].as_f64().expect("h"),
            ]
        };
        // The track's box at rest, within the plan's slack.
        let track = read("slider-default", "slider_p");
        for (got, want) in [
            (track[0], 41.0),
            (track[1], 737.5),
            (track[2], 350.0),
            (track[3], 20.0),
        ] {
            assert!((got - want).abs() <= SLACK, "track {got} against {want}");
        }
        // The thumb's box wraps the mark on its centre.
        let thumb = read("slider-default", "thumb_p");
        let range = SliderRange::from_visible(-9.9, 9.9).expect("span");
        let laid = mv_sys::clay::BoundingBox {
            x: track[0] as f32,
            y: track[1] as f32,
            width: track[2] as f32,
            height: track[3] as f32,
        };
        let mine = thumb_box(&laid, range, 2.0).expect("thumb");
        for (got, want) in [
            (f64::from(mine.x), thumb[0]),
            (f64::from(mine.y), thumb[1]),
            (f64::from(mine.width), thumb[2]),
            (f64::from(mine.height), thumb[3]),
        ] {
            assert!((got - want).abs() <= SLACK, "thumb {got} against {want}");
        }
        // The tip's box on hover: centred on the thumb, 4 px above.
        let tip = read("slider-hover", "slider_tip_p");
        assert!((tip[3] - 22.0).abs() <= SLACK, "tip height");
        assert!(
            (tip[1] - (track[1] - 4.0 - 22.0)).abs() <= SLACK,
            "tip above"
        );
        // The tip's laid-out box with real fonts: x, width and height.
        let _guard = mv_clay::tests_lock();
        let mut app = crate::fixture::headless_app();
        assert!(mv_text::init(&mut app.fonts));
        let geom = tip_geom(&mut app, &laid, range, 2.0).expect("tip");
        for (got, want) in [
            (f64::from(geom.r#box.x), tip[0]),
            (f64::from(geom.r#box.width), tip[2]),
            (f64::from(geom.r#box.height), tip[3]),
        ] {
            assert!((got - want).abs() <= SLACK, "tip {got} against {want}");
        }
    }
}
