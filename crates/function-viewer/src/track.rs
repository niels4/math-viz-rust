//! P's track slider: press, drag, keys, the thumb, the tip, and the parked
//! dashed ring.
//!
//! The value maps linearly onto the track between lattice-clamped ends; past
//! either end the thumb parks with its dashed ring and the tip names the
//! value.

use mv_sys::clay;

use crate::app::App;
use crate::input::{Cursor, Hit, HitKind};
use crate::point_rows::POINT_MARK_SIZE;
use crate::widgets::{PointMark, PointMarkKind};

/// The track's state: its span, validity, colours, thumb and hover/focus/drag.
#[derive(Clone, Debug)]
pub struct Track {
    pub value: f64,
    pub min: f64,
    pub max: f64,
    pub valid: bool,
    pub line: mv_theme::Color,
    pub thumb: PointMark,
    pub hovered: bool,
    pub focused: bool,
    pub dragging: bool,
    pub halo: mv_theme::Color,
    pub plate: mv_theme::Color,
    pub words: mv_theme::Color,
}

/// The track height, inset, zero mark, lattice quantum and key steps.
pub const TRACK_HEIGHT: f32 = 20.0;
pub const TRACK_INSET: f32 = 9.0;
pub const TRACK_ZERO: f32 = 10.0;
/// The point lattice quantum, shared with the viewer.
pub const TRACK_QUANTUM: f64 = mv_viewer::POINT_QUANTUM;
pub const TRACK_STEP: f32 = 0.1;
pub const TRACK_STEP_SHIFT: f32 = 1.0;
pub const THUMB_HOVER_SCALE: f32 = 1.12;
pub const THUMB_HALO: f32 = 8.0;

/// The tip's height, padding, gap, radius, air above, triangle and symbol.
pub const TRACK_TIP_HEIGHT: f32 = 22.0;
pub const TRACK_TIP_PADDING: [f32; 2] = [7.0, 2.0];
pub const TRACK_TIP_GAP: f32 = 4.0;
pub const TRACK_TIP_RADIUS: f32 = 5.0;
pub const TRACK_TIP_ABOVE: f32 = 4.0;
pub const TRACK_TIP_TRIANGLE: [f32; 2] = [7.0, 10.0];
pub const TRACK_TIP_SYMBOL: &str = "x";

/// The tip symbol's style and line.
pub static TRACK_TIP_SYMBOL_STYLE: std::sync::LazyLock<mv_text::Style> =
    std::sync::LazyLock::new(|| mv_text::Style {
        face: mv_text::Face::Stix400Italic,
        size: 14.0,
        tracking: 0.0,
        features: enumset::EnumSet::empty(),
    });
pub const TRACK_TIP_SYMBOL_LINE: f32 = 18.0;
/// The tip value's style and line.
pub static TRACK_TIP_VALUE_STYLE: std::sync::LazyLock<mv_text::Style> =
    std::sync::LazyLock::new(|| mv_text::Style {
        face: mv_text::Face::RobotoMono500,
        size: 13.0,
        tracking: 0.0,
        features: enumset::EnumSet::only(mv_text::Feature::Tabular),
    });
pub const TRACK_TIP_VALUE_LINE: f32 = 17.0;

/// Builds the track state from the viewer, input and theme.
pub fn track_of(app: &mut App) -> Track {
    let t = crate::ui::current_theme(app).clone();
    let hit = Hit {
        kind: HitKind::Track,
        index: 0,
    };
    let (min, max, laid_out) = match &app.viewer.view {
        Some(view) => (view.extent.min_x, view.extent.max_x, true),
        None => (0.0, 0.0, false),
    };
    let mut track = Track {
        value: app.viewer.p_x,
        min,
        max,
        valid: false,
        line: t.colors[mv_theme::Token::ForegroundMuted],
        thumb: PointMark {
            kind: PointMarkKind::Bullseye,
            size: POINT_MARK_SIZE,
            ink: t.colors[crate::point_rows::POINT_INKS[mv_viewer::Point::P]],
            parked: false,
        },
        hovered: app.input.hover == hit,
        focused: app.input.focus_visible && app.input.focus == hit,
        dragging: app.input.pressed == hit,
        halo: mv_theme::plate(&t, mv_theme::Token::Primary),
        plate: t.colors[mv_theme::Token::Foreground],
        words: t.colors[mv_theme::Token::Background],
    };
    track.valid = laid_out && track_span_ok(&track);
    track
}

/// Whether the span and value are finite and non-empty.
fn track_span_ok(track: &Track) -> bool {
    let span = track.max - track.min;
    !track.value.is_nan() && !track.value.is_infinite() && span > 0.0 && !span.is_infinite()
}

/// Which end an off-track value lies past.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Parked {
    No,
    Left,
    Right,
}

/// Which end the value parks past, if either.
pub fn parked(track: &Track) -> Parked {
    if !track.valid {
        return Parked::No;
    }
    if track.value < track.min {
        Parked::Left
    } else if track.value > track.max {
        Parked::Right
    } else {
        Parked::No
    }
}

/// Declares the track element with its region stop.
pub fn p_track(app: &mut App) {
    let id = mv_clay::id("Track", 0);
    let payload = crate::widgets::Custom::Track(track_of(app));
    let layout = clay::LayoutConfig {
        sizing: clay::Sizing {
            width: crate::tip::grow(),
            height: crate::tip::fixed(TRACK_HEIGHT),
        },
        padding: clay::Padding {
            left: 0,
            right: 0,
            top: 0,
            bottom: 0,
        },
        childGap: 0,
        childAlignment: clay::ChildAlignment {
            x: clay::LayoutAlignmentX::Left,
            y: clay::LayoutAlignmentY::Top,
        },
        layoutDirection: clay::LayoutDirection::LeftToRight,
    };
    let mut config = crate::tip::blank_declaration(layout);
    config.custom = crate::widgets::custom_data(payload);
    let hit = Hit {
        kind: HitKind::Track,
        index: 0,
    };
    crate::ui::ui_scope(app, id, &config, |app| {
        crate::input::region(
            app,
            id,
            hit,
            Cursor::Pointer,
            Cursor::Default,
            true,
            clay::CornerRadius {
                topLeft: 0.0,
                topRight: 0.0,
                bottomLeft: 0.0,
                bottomRight: 0.0,
            },
        );
    });
}

/// Snaps a range end inward to the lattice.
fn inward(v: f64, low: bool) -> f64 {
    let q = TRACK_QUANTUM;
    let n = if low {
        (v / q - 1e-9).ceil()
    } else {
        (v / q + 1e-9).floor()
    };
    mv_numfmt::quantize(n * q, q)
}

/// The lattice-clamped range ends.
pub fn track_ends(track: &Track) -> (f64, f64) {
    (inward(track.min, true), inward(track.max, false))
}

/// Quantizes to the lattice without negative zero.
pub fn track_quantize(v: f64) -> f64 {
    let q = mv_numfmt::quantize(v, TRACK_QUANTUM);
    if q == 0.0 { 0.0 } else { q }
}

/// The lattice value under pointer x, when in range.
pub fn track_value_at(track: &Track, r#box: clay::BoundingBox, x: f32) -> (f64, bool) {
    if r#box.width <= 2.0 * TRACK_INSET {
        return (0.0, false);
    }
    let run = f64::from(r#box.width - 2.0 * TRACK_INSET);
    let t = (f64::from(x - r#box.x - TRACK_INSET) / run).clamp(0.0, 1.0);
    let (low, high) = track_ends(track);
    let span_value = track.min + t * (track.max - track.min);
    (track_quantize(high.min(low.max(span_value))), true)
}

/// Dispatches the quantized value when it differs.
fn track_send(app: &mut App, next: f64) {
    if track_quantize(next) != app.viewer.p_x {
        crate::parts::dispatch(
            app,
            mv_viewer::Action::SetP(mv_viewer::SetP {
                x: track_quantize(next),
            }),
        );
    }
}

/// Sends the value under pointer x.
fn track_send_at(app: &mut App, track: &Track, r#box: clay::BoundingBox, x: f32) {
    if let (value, true) = track_value_at(track, r#box, x) {
        track_send(app, value);
    }
}

/// Starts the P drag and jumps to the pointer.
pub fn track_press(app: &mut App) {
    let track = track_of(app);
    let (r, found) = crate::input::region_of(
        app,
        Hit {
            kind: HitKind::Track,
            index: 0,
        },
    );
    if !track.valid || !found {
        return;
    }
    crate::parts::dispatch(
        app,
        mv_viewer::Action::SetDrag(mv_viewer::SetDrag {
            part: mv_viewer::Part::P,
            mode: Some(mv_scrub::Mode::Coarse),
        }),
    );
    track_send_at(app, &track, r.r#box, app.input.pointer[0]);
}

/// Moves P with the pointer while held.
pub fn track_drag(app: &mut App) {
    let track = track_of(app);
    let (r, found) = crate::input::region_of(
        app,
        Hit {
            kind: HitKind::Track,
            index: 0,
        },
    );
    if found && track.valid {
        track_send_at(app, &track, r.r#box, app.input.pointer[0]);
    }
}

/// Ends the P drag.
pub fn track_release(app: &mut App) {
    crate::parts::dispatch(
        app,
        mv_viewer::Action::SetDrag(mv_viewer::SetDrag {
            part: mv_viewer::Part::P,
            mode: None,
        }),
    );
}

/// Steps, Home/End, and Enter opening the field, on the track.
pub fn track_key(app: &mut App, key: sdl3::keyboard::Keycode, mods: sdl3::keyboard::Mod) -> bool {
    use sdl3::keyboard::Keycode;
    let track = track_of(app);
    match key {
        Keycode::Right | Keycode::Up | Keycode::Left | Keycode::Down => {
            let direction = if key == Keycode::Right || key == Keycode::Up {
                1.0
            } else {
                -1.0
            };
            // The surface keeps the steps as f32 geometry; the drag quantizes
            // the sum back onto the 0.01 lattice either way.
            let step = if mods
                .intersects(sdl3::keyboard::Mod::LSHIFTMOD | sdl3::keyboard::Mod::RSHIFTMOD)
            {
                f64::from(TRACK_STEP_SHIFT)
            } else {
                f64::from(TRACK_STEP)
            };
            track_send(app, track.value + direction * step);
            true
        }
        Keycode::Home | Keycode::End => {
            if !track.valid {
                return false;
            }
            let (low, high) = track_ends(&track);
            track_send(app, if key == Keycode::Home { low } else { high });
            true
        }
        Keycode::Return | Keycode::KpEnter => {
            crate::number_field::field_open(app, mv_viewer::Part::P);
            true
        }
        _ => false,
    }
}

/// The thumb centre from the left, in hundredths of a px.
pub fn thumb_x(track: &Track, width: f64) -> f64 {
    let span = track.max - track.min;
    let at = track.value.clamp(track.min, track.max);
    px2(f64::from(TRACK_INSET) + (at - track.min) / span * (width - 2.0 * f64::from(TRACK_INSET)))
}

/// The thumb box and the focus-ring width.
pub fn track_ring(app: &mut App, r#box: clay::BoundingBox) -> (clay::BoundingBox, f32) {
    let track = track_of(app);
    if !track.valid {
        return (
            clay::BoundingBox {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
            },
            0.0,
        );
    }
    let c = [
        r#box.x + thumb_x(&track, f64::from(r#box.width)) as f32,
        r#box.y + TRACK_HEIGHT / 2.0,
    ];
    let scale = thumb_scale(&track);
    let half = POINT_MARK_SIZE / 2.0 * scale;
    (
        clay::BoundingBox {
            x: c[0] - half,
            y: c[1] - half,
            width: 2.0 * half,
            height: 2.0 * half,
        },
        crate::effects::RING_WIDTH * scale,
    )
}

/// The zero-mark position, when the span crosses zero.
pub fn track_zero_x(track: &Track, width: f64) -> Option<f64> {
    let run = width - 2.0 * f64::from(TRACK_INSET);
    if !track.valid || run <= 0.0 || track.min > 0.0 || track.max < 0.0 {
        return None;
    }
    Some(px2(
        f64::from(TRACK_INSET) + (-track.min / (track.max - track.min)) * run
    ))
}

/// 112% when hovered and not dragging.
pub fn thumb_scale(track: &Track) -> f32 {
    if track.hovered && !track.dragging {
        THUMB_HOVER_SCALE
    } else {
        1.0
    }
}

/// Draws the line, zero mark, halo/thumb and value tip.
pub fn draw_track(app: &mut App, b: [f32; 4], track: &Track) {
    let width = f64::from(b[2]);
    if !track.valid || width - 2.0 * f64::from(TRACK_INSET) <= 0.0 {
        return;
    }
    let y = b[1] + TRACK_HEIGHT / 2.0;
    app.vg.begin_path();
    app.vg.move_to(b[0] + TRACK_INSET, y);
    app.vg
        .line_to(b[0] + px2(width - f64::from(TRACK_INSET)) as f32, y);
    app.vg.stroke_color(track.line);
    app.vg.stroke_width(2.0);
    app.vg.line_cap(mv_nvg::LineCapType::BUTT);
    app.vg.stroke();
    if let Some(zero_at) = track_zero_x(track, width) {
        let zero = b[0] + zero_at as f32;
        app.vg.begin_path();
        app.vg.move_to(zero, y - TRACK_ZERO / 2.0);
        app.vg.line_to(zero, y + TRACK_ZERO / 2.0);
        app.vg.stroke_width(1.5);
        app.vg.stroke();
    }
    let x = thumb_x(track, width) as f32;
    let centre = [b[0] + x, y];
    if track.dragging {
        app.vg.begin_path();
        app.vg
            .circle(centre[0], centre[1], POINT_MARK_SIZE / 2.0 + THUMB_HALO);
        app.vg.fill_color(crate::draw::css_background(track.halo));
        app.vg.fill();
    }
    let mut mark = track.thumb;
    mark.parked = parked(track) != Parked::No;
    app.vg.save();
    if thumb_scale(track) != 1.0 {
        let scale = thumb_scale(track);
        app.vg.translate(centre[0], centre[1]);
        app.vg.scale(scale, scale);
        app.vg.translate(-centre[0], -centre[1]);
    }
    let theme = crate::ui::current_theme(app).clone();
    crate::widgets::draw_point_mark(&mut app.vg, centre, mark, &theme);
    app.vg.restore();
    if track.hovered || track.focused || track.dragging {
        draw_track_tip(app, track, b, x);
    }
}

/// Draws the clamped `x = value` tip with its parked arrow.
fn draw_track_tip(app: &mut App, track: &Track, b: [f32; 4], x: f32) {
    let value = format!(
        "= {}",
        mv_numfmt::format_number(track.value, mv_numfmt::DISPLAY_DP)
    );
    let symbol_w = mv_text::layout_width(mv_text::measure(
        &mut app.fonts,
        *TRACK_TIP_SYMBOL_STYLE,
        TRACK_TIP_SYMBOL,
    ));
    let value_w = mv_text::layout_width(mv_text::measure(
        &mut app.fonts,
        *TRACK_TIP_VALUE_STYLE,
        &value,
    ));
    let side = parked(track);
    let mut width = 2.0 * TRACK_TIP_PADDING[0] + symbol_w + TRACK_TIP_GAP + value_w;
    if side != Parked::No {
        width += TRACK_TIP_GAP + TRACK_TIP_TRIANGLE[0];
    }
    // Centred on the thumb, but never past the track's ends. A min/max pair,
    // not a clamp: on a track narrower than the tip the low end wins over the
    // high one, as the reference's clamp does, where Rust's would panic.
    let w = px2(f64::from(b[2])) as f32;
    let left = b[0] + x + (-width / 2.0).max(-x).min(w - x - width);
    let top = b[1] - TRACK_TIP_ABOVE - TRACK_TIP_HEIGHT;
    app.vg.begin_path();
    app.vg
        .rounded_rect(left, top, width, TRACK_TIP_HEIGHT, TRACK_TIP_RADIUS);
    app.vg.fill_color(track.plate);
    app.vg.fill();
    let content = top + TRACK_TIP_PADDING[1];
    let inner = TRACK_TIP_HEIGHT - 2.0 * TRACK_TIP_PADDING[1];
    let mut pen = left + TRACK_TIP_PADDING[0];
    if side == Parked::Left {
        track_tip_triangle(
            &mut app.vg,
            [pen, content + (inner - TRACK_TIP_TRIANGLE[1]) / 2.0],
            false,
            track.words,
        );
        pen += TRACK_TIP_TRIANGLE[0] + TRACK_TIP_GAP;
    }
    let symbol_top = content + (inner - TRACK_TIP_SYMBOL_LINE) / 2.0;
    let symbol_base = mv_text::baseline(
        &mut app.fonts,
        TRACK_TIP_SYMBOL_STYLE.face,
        TRACK_TIP_SYMBOL_STYLE.size,
        TRACK_TIP_SYMBOL_LINE,
    );
    mv_text::draw(
        &mut app.fonts,
        &mut app.vg,
        *TRACK_TIP_SYMBOL_STYLE,
        pen,
        symbol_top + symbol_base,
        TRACK_TIP_SYMBOL,
        track.words,
    );
    pen += symbol_w + TRACK_TIP_GAP;
    let value_top = content + (inner - TRACK_TIP_VALUE_LINE) / 2.0;
    let value_base = mv_text::baseline(
        &mut app.fonts,
        TRACK_TIP_VALUE_STYLE.face,
        TRACK_TIP_VALUE_STYLE.size,
        TRACK_TIP_VALUE_LINE,
    );
    mv_text::draw(
        &mut app.fonts,
        &mut app.vg,
        *TRACK_TIP_VALUE_STYLE,
        pen,
        value_top + value_base,
        &value,
        track.words,
    );
    pen += value_w + TRACK_TIP_GAP;
    if side == Parked::Right {
        track_tip_triangle(
            &mut app.vg,
            [pen, content + (inner - TRACK_TIP_TRIANGLE[1]) / 2.0],
            true,
            track.words,
        );
    }
}

/// A 7x10 triangle pointing the way a parked thumb's value lies.
fn track_tip_triangle(vg: &mut mv_nvg::Context, at: [f32; 2], right: bool, ink: mv_theme::Color) {
    let (w, h) = (TRACK_TIP_TRIANGLE[0], TRACK_TIP_TRIANGLE[1]);
    vg.begin_path();
    if right {
        vg.move_to(at[0], at[1]);
        vg.line_to(at[0] + w, at[1] + h / 2.0);
        vg.line_to(at[0], at[1] + h);
    } else {
        vg.move_to(at[0] + w, at[1]);
        vg.line_to(at[0] + w, at[1] + h);
        vg.line_to(at[0], at[1] + h / 2.0);
    }
    vg.close_path();
    vg.fill_color(ink);
    vg.fill();
}

/// Rounds to hundredths of a px.
pub fn px2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// Strokes the parked thumb's dashed ring.
pub fn draw_dashed_ring(
    vg: &mut mv_nvg::Context,
    centre: [f32; 2],
    r: f32,
    width: f32,
    ink: mv_theme::Color,
) {
    vg.save();
    vg.line_cap(mv_nvg::LineCapType::BUTT);
    vg.begin_path();
    for arc in mv_plane::dash_circle(f64::from(r), [3.0, 2.0], 0.0) {
        vg.move_to(
            centre[0] + r * (arc[0] as f32).cos(),
            centre[1] + r * (arc[0] as f32).sin(),
        );
        vg.arc(
            centre[0],
            centre[1],
            r,
            arc[0] as f32,
            arc[1] as f32,
            mv_nvg::Winding::CW,
        );
    }
    vg.stroke_width(width);
    vg.stroke_color(ink);
    vg.stroke();
    vg.restore();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{ButtonEvent, FocusEvent, Input, PointerEvent, Region};

    /// A 394 px track at x 100 whose line's ends sit 9 px in.
    const LINE_TRACK: clay::BoundingBox = clay::BoundingBox {
        x: 100.0,
        y: 0.0,
        width: 394.0,
        height: TRACK_HEIGHT,
    };

    fn r2(value: f64) -> Track {
        Track {
            value,
            min: -9.36,
            max: 9.36,
            valid: true,
            line: [0.0; 4],
            thumb: PointMark {
                kind: PointMarkKind::Bullseye,
                size: POINT_MARK_SIZE,
                ink: [0.0; 4],
                parked: false,
            },
            hovered: false,
            focused: false,
            dragging: false,
            halo: [0.0; 4],
            plate: [0.0; 4],
            words: [0.0; 4],
        }
    }

    #[test]
    fn the_thumb_sits_on_the_inset_run() {
        // 9 + (2 + 9.36) / 18.72 of 376 px.
        assert_eq!(thumb_x(&r2(2.0), 394.0), 237.17);
        assert_eq!(parked(&r2(2.0)), Parked::No);
        // Parked on the line's ends, the end it lies past.
        assert_eq!(thumb_x(&r2(21.5), 394.0), 385.0);
        assert_eq!(parked(&r2(21.5)), Parked::Right);
        assert_eq!(thumb_x(&r2(-14.2), 394.0), 9.0);
        assert_eq!(parked(&r2(-14.2)), Parked::Left);
    }

    #[test]
    fn a_press_maps_the_inset_run() {
        // A quarter of the run along: -9.36 + 4.68.
        let (value, ok) = track_value_at(&r2(2.0), LINE_TRACK, 100.0 + 9.0 + 94.0);
        assert!(ok);
        assert_eq!(value, -4.68);
        // Past the line's start: its end, inside the range.
        let (value, _) = track_value_at(&r2(2.0), LINE_TRACK, 100.0 + 4.0);
        assert_eq!(value, -9.36);
        let (value, _) = track_value_at(&r2(2.0), LINE_TRACK, 100.0 + 2000.0);
        assert_eq!(value, 9.36);
        // A track with no run takes no press.
        let (_, ok) = track_value_at(
            &r2(2.0),
            clay::BoundingBox {
                x: 100.0,
                y: 0.0,
                width: 18.0,
                height: TRACK_HEIGHT,
            },
            109.0,
        );
        assert!(!ok);
    }

    #[test]
    fn home_and_end_land_inside_the_range() {
        let (low, high) = track_ends(&Track {
            value: 2.0,
            min: -9.364,
            max: 9.366,
            valid: true,
            ..r2(2.0)
        });
        assert_eq!(low, -9.36);
        assert_eq!(high, 9.36);
    }

    #[test]
    fn the_arrows_step_past_the_ends() {
        // From 9.3: right, Shift+left, down (no bounds).
        let v = track_quantize(9.3 + f64::from(TRACK_STEP));
        assert_eq!(v, 9.4);
        let v = track_quantize(v - f64::from(TRACK_STEP_SHIFT));
        assert_eq!(v, 8.4);
        let v = track_quantize(v - f64::from(TRACK_STEP));
        assert_eq!(v, 8.3);
        assert_eq!(track_quantize(-0.001), 0.0);
    }

    #[test]
    fn a_focused_part_scrolls_into_view() {
        // The short window (1100 x 720): the panel shows 93..703 and goes as
        // far as 109. P's track (711.5..731.5) shows none: centred, as far as
        // the panel goes.
        assert_eq!(
            crate::scroll::scroll_target(0.0, 109.0, [93.0, 703.0], [711.5, 731.5]),
            109.0
        );
        // h's box shows whole: nothing moves.
        assert_eq!(
            crate::scroll::scroll_target(109.0, 109.0, [93.0, 703.0], [448.5, 480.5]),
            109.0
        );
        // The picker, above the view at 109 (55..89): centred, back to the top.
        assert_eq!(
            crate::scroll::scroll_target(109.0, 109.0, [93.0, 703.0], [55.0, 89.0]),
            0.0
        );
        // The stacked page (800 x 900): once the plane's keys have grown the
        // panel by 49 px the plane is 873.5..1353.5 and the page goes as far as
        // 470; the plane shows in part, so its foot comes to the window's:
        // 453.5, a whole px halves up.
        assert_eq!(
            crate::scroll::scroll_target(0.0, 470.0, [0.0, 900.0], [873.5, 1353.5]),
            454.0
        );
        assert_eq!(
            crate::scroll::scroll_target(0.0, 421.0, [0.0, 900.0], [824.5, 1304.5]),
            405.0
        );
        // Back to the plane by Shift+Tab, the page at 421: from 452.5..932.5,
        // its foot to the window's again.
        assert_eq!(
            crate::scroll::scroll_target(421.0, 470.0, [0.0, 900.0], [452.5, 932.5]),
            454.0
        );
    }

    #[test]
    fn the_zero_mark_sits_where_0_is() {
        // R2 on the 394 px line, the mark at 9 + 188.
        let Some(zero) = track_zero_x(&r2(2.0), 394.0) else {
            panic!("zero shows on R2");
        };
        assert_eq!(zero, 197.0);
        // No mark where 0 is off the range (1..5), and with no range at all.
        assert!(
            track_zero_x(
                &Track {
                    value: 2.0,
                    min: 1.0,
                    max: 5.0,
                    valid: true,
                    ..r2(2.0)
                },
                394.0
            )
            .is_none()
        );
        assert!(
            track_zero_x(
                &Track {
                    value: 0.0,
                    min: 0.0,
                    max: 0.0,
                    valid: false,
                    ..r2(2.0)
                },
                394.0
            )
            .is_none()
        );
    }

    /// An App with P's track laid out as at 1440 x 900 (49, 711.5, 394 x 20)
    /// over the plane's view, its region the only one. Ported from
    /// `track_test.odin:111` (`track_app`).
    fn track_app(extent: mv_plane::Extent) -> App {
        let mut app = crate::fixture::headless_app();
        app.viewer = mv_viewer::initial_state();
        app.viewer.view = Some(mv_plane::View { zoom: 50.0, extent });
        app.r#loop.script_time = Some(0);
        app.size = [1440.0, 900.0];
        app.pixels = [2880, 1800];
        push_track_region(&mut app);
        app
    }

    const SDL_LEFT: u8 = 1;

    #[test]
    fn an_empty_range_takes_no_press() {
        // "shows no thumb and ignores presses over an empty range".
        let mut app = track_app(mv_plane::Extent {
            min_x: 0.0,
            max_x: 0.0,
            min_y: -7.92,
            max_y: 7.92,
        });
        assert!(!track_of(&mut app).valid);
        crate::input::handle_input(
            &mut app,
            Input::Button(ButtonEvent {
                button: SDL_LEFT,
                down: true,
                clicks: 1,
                pos: [49.0 + 150.0, 721.0],
            }),
            0,
        );
        assert_eq!(app.viewer.drag, None);
        assert_eq!(app.viewer.p_x, 2.0);
    }

    #[test]
    fn a_release_anywhere_or_the_window_leaving_ends_the_drag() {
        // "ends a drag on a release anywhere, and drops one whose release was
        // missed". The track keeps the pointer from its press (capture), so
        // its release ends the drag wherever it falls; a release the window
        // never sees is the window losing its focus here, which ends the drag
        // too.
        let mut app = track_app(mv_plane::Extent {
            min_x: -9.36,
            max_x: 9.36,
            min_y: -7.92,
            max_y: 7.92,
        });
        crate::input::handle_input(
            &mut app,
            Input::Button(ButtonEvent {
                button: SDL_LEFT,
                down: true,
                clicks: 1,
                pos: [49.0 + 198.0, 721.0],
            }),
            0,
        );
        let Some(drag) = &app.viewer.drag else {
            panic!("press starts P's drag");
        };
        assert!(drag.part == mv_viewer::Part::P && drag.mode == mv_scrub::Mode::Coarse);
        // The press jumps P to the pointer: 198 of 394 is 189 along the
        // 376 px run.
        assert_eq!(app.viewer.p_x, 0.05);
        crate::input::handle_input(
            &mut app,
            Input::Pointer(PointerEvent {
                pos: [900.0, 300.0],
                left: false,
            }),
            0,
        );
        // The range's end, inside its lattice.
        assert_eq!(app.viewer.p_x, 9.36);
        crate::input::handle_input(
            &mut app,
            Input::Button(ButtonEvent {
                button: SDL_LEFT,
                down: false,
                clicks: 1,
                pos: [900.0, 300.0],
            }),
            0,
        );
        assert_eq!(app.viewer.drag, None);
        crate::input::handle_input(
            &mut app,
            Input::Pointer(PointerEvent {
                pos: [49.0 + 98.0, 721.0],
                left: false,
            }),
            0,
        );
        // No button held: no drag.
        assert_eq!(app.viewer.p_x, 9.36);
        crate::input::handle_input(
            &mut app,
            Input::Button(ButtonEvent {
                button: SDL_LEFT,
                down: true,
                clicks: 1,
                pos: [49.0 + 98.0, 721.0],
            }),
            0,
        );
        assert!(app.viewer.drag.is_some());
        crate::input::handle_input(&mut app, Input::Focus(FocusEvent { gained: false }), 0);
        assert_eq!(app.viewer.drag, None);
        assert_eq!(
            app.input.pressed,
            Hit {
                kind: HitKind::None,
                index: 0,
            }
        );
    }

    #[allow(dead_code)]
    fn push_track_region(app: &mut App) {
        app.input.regions.push(Region {
            hit: Hit {
                kind: HitKind::Track,
                index: 0,
            },
            r#box: clay::BoundingBox {
                x: 49.0,
                y: 711.5,
                width: 394.0,
                height: TRACK_HEIGHT,
            },
            cursor: Cursor::Default,
            pressed: Cursor::Default,
            radius: clay::CornerRadius {
                topLeft: 0.0,
                topRight: 0.0,
                bottomLeft: 0.0,
                bottomRight: 0.0,
            },
            clip: None,
            z: 0,
        });
    }
}
