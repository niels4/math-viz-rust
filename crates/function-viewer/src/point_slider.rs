//! The x-of-P slider's state model: the track geometry, the drag mapping and
//! the keys.
//!
//! Ported from
//! `src/components/views/FunctionViewer/SideBar/PointsSection/PointSlider.tsx`
//! and `PointSlider.module.css` at `96f1e84`, with `selectNothing.ts` read
//! for the press lock. The slider sits under P's row head and spans the
//! plane's visible x range (`visibleRangeAtom` in `CartesianPlane/state.ts`):
//! a 20 px track (pointer cursor, no outline, `touch-action: none`) with its
//! line from `INSET` 9 to `width − INSET` at `LINE_Y` 10 (2 px, foreground
//! muted) and a 10 px zero tick (1.5 px) where x = 0 lies inside the range; a
//! thumb (h8-points-row's `PointMark`) at `INSET + t · run`, scaled 1.12 on
//! hover, a 3 px primary ring at 32% on focus-visible, an 8 px primary halo at
//! 26% while dragging; parked at an end with the dashed parked mark when P
//! lies past the visible range; a 22 px tip above the thumb on hover, focus
//! or drag (`x` italic 14 px serif, `= value` 13 px mono tabular, 4 px above
//! the track, clamped inside it; a 7×10 triangle when parked).
//!
//! Behaviour: a left-button press (range valid) starts a drag (`p_dragging`,
//! pointer capture, no text selection until release) and jumps P to the
//! pointer (`t = clamp((x − left − INSET) / run, 0, 1)`, clamped to the
//! inward bounds, quantized to 0.01); moves while captured send the same (a
//! move reporting no buttons stops); release, cancel, lost capture and the
//! window's blur stop (`p_dragging` cleared, `p` flushed). Keys on the focused
//! track: arrows ±0.1 (×10 with Shift), Home/End to the inward bounds, Enter
//! opening the field (h8-points-field). Aria: role slider "x of P", valuemin
//! `low`, valuemax `high`, valuenow the value, valuetext `format_number`.
//!
//! This module holds the pure half: every mapping as a function of the
//! visible range and the track width, and the drag identity. The `App` half
//! (phase 2, after h7-sidebar merges: `p`'s `QueryParam` writes, `p_dragging`
//! on `App`, the Clay mount, the painters, the `p` views.json line) calls
//! these. `PointMark` is h8-points-row's painter; `QUANTUM`, `quantize` and
//! `format_number` come from h8-p-points' `point_format` (identical values to
//! the `mv-numfmt` calls used here until the merge — see the progress file).

use mv_numfmt::{format_number, quantize};

/// The quantum P moves in (`QUANTUM` in `format.ts`).
pub const QUANTUM: f64 = 0.01;

/// An arrow step (`STEP`).
pub const STEP: f64 = 0.1;
/// An arrow step with Shift held (`STEP_SHIFT`).
pub const STEP_SHIFT: f64 = 1.0;

/// The track's end inset (`INSET`): the line runs `INSET..width − INSET` and
/// the thumb parks at `INSET` / `width − INSET`.
pub const TRACK_INSET: f64 = 9.0;
/// The track's height (20 px).
pub const TRACK_HEIGHT: f32 = 20.0;
/// The line's height inside the track (`LINE_Y`): the svg is `LINE_Y * 2` tall.
pub const TRACK_LINE_Y: f32 = 10.0;
/// The zero tick's height (`ZERO_MARK`).
pub const ZERO_TICK_HEIGHT: f32 = 10.0;

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
    quantize((min_x / QUANTUM - 1e-9).ceil() * QUANTUM, QUANTUM)
}

/// The last 0.01 multiple at or below `max_x` (`inward(v, "high")`).
#[must_use]
pub fn inward_high(max_x: f64) -> f64 {
    quantize((max_x / QUANTUM + 1e-9).floor() * QUANTUM, QUANTUM)
}

/// P quantized to the slider's quantum (`send`'s `quantize(next)`).
#[must_use]
pub fn quantize_p(value: f64) -> f64 {
    quantize(value, QUANTUM)
}

/// P as the aria value text and the tip show it (`formatNumber`).
#[must_use]
pub fn format_p(value: f64) -> String {
    format_number(value, 2)
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
    Some(quantize_p(
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
            KeyOutcome::Send(quantize_p(value + direction(1.0, shift)))
        }
        SliderKey::ArrowLeft | SliderKey::ArrowDown => {
            KeyOutcome::Send(quantize_p(value + direction(-1.0, shift)))
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
/// (`pointerId` in the source; `p_dragging` lives on `App` in phase 2).
/// Capture, the select-nothing lock and the window-blur stop ride the input
/// layer (h5-box's model); a move reporting no buttons stops, as the source's
/// `e.buttons === 0` arm does.
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
        // App layer, phase 2).
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
        assert_eq!(
            value_from_client_x(100.0, 41.0, 350.0, range),
            Some(-6.92)
        );
    }

    #[test]
    fn drag_30px_from_the_thumb() {
        // Pressing the thumb centre (track 41 + 208.54) holds ≈2; +30 px
        // gives 3.79 with the URL unwritten until release.
        let range = SliderRange::from_visible(-9.9, 9.9).expect("span");
        assert_eq!(
            value_from_client_x(249.54, 41.0, 350.0, range),
            Some(2.0)
        );
        assert_eq!(
            value_from_client_x(279.54, 41.0, 350.0, range),
            Some(3.79)
        );
    }

    #[test]
    fn aria_text_uses_the_true_minus() {
        assert_eq!(format_p(2.0), "2.00");
        assert_eq!(format_p(-50.0), "\u{2212}50.00");
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
        assert_eq!(
            SliderRange::from_visible(f64::NAN, 5.0),
            None
        );
    }

    #[test]
    fn enter_opens_the_field() {
        assert_eq!(
            apply_key(SliderKey::Enter, false, 2.0, Some(fixture())),
            KeyOutcome::OpenField
        );
    }
}
