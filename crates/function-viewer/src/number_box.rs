//! The parameter number boxes: scrub-drag, wheel, keys, the rest tip, and the
//! open field.
//!
//! Each row's box shows its value; dragging scrubs it on its lattice,
//! double-click resets it, and Enter opens the typed field.

use mv_sys::clay;
use sdl3::keyboard::{Keycode, Mod};

use crate::app::App;
use crate::input::{Cursor, Hit, HitKind};

/// The rest tip's delay and the caret blink period, in ms.
pub const REST_MS: f64 = 400.0;
pub const CARET_BLINK_MS: f64 = 500.0;

/// The number boxes' own: a press or drag, a rest, and the open field.
pub struct Boxes {
    pub scrub: Option<Scrub>,
    pub resting: Option<mv_viewer::Param>,
    pub field: super::number_field::Field,
}

/// An active scrub: whose, from where, in what mode, at what value.
#[derive(Clone, Copy, Debug)]
pub struct Scrub {
    pub param: mv_viewer::Param,
    pub anchor_x: f32,
    pub anchor_value: f64,
    pub last_x: f32,
    pub mode: mv_scrub::Mode,
    pub value: f64,
    pub pending: bool,
    pub reset: bool,
}

/// The box geometry and the field line.
pub const NUMBER_BOX_HEIGHT: f32 = 32.0;
pub const NUMBER_BOX_RADIUS: f32 = 8.0;
pub const NUMBER_BOX_INSET: f32 = 10.5;
pub const NUMBER_BOX_SLOT: f32 = 62.0;
pub const CHEVRON_SIZE: [f32; 2] = [6.0, 12.0];
pub const CHEVRON_GAP: f32 = 6.0;
pub const FIELD_LINE: f32 = 22.0;

/// The field's inset: past the chevron.
pub const FIELD_INSET: f32 = NUMBER_BOX_INSET + CHEVRON_SIZE[0] + CHEVRON_GAP;

/// A parameter's part: the first four parts are the four parameters, in order.
pub(crate) fn part_of_param(p: mv_viewer::Param) -> mv_viewer::Part {
    match p {
        mv_viewer::Param::A => mv_viewer::Part::A,
        mv_viewer::Param::K => mv_viewer::Part::K,
        mv_viewer::Param::B => mv_viewer::Part::B,
        mv_viewer::Param::H => mv_viewer::Part::H,
    }
}

/// The box width from the text width.
pub fn number_box_width(text_width: f32) -> f32 {
    2.0 * FIELD_INSET + NUMBER_BOX_SLOT.max(text_width)
}

/// The readout-face tabular style the field shows.
pub fn field_style(t: &mv_theme::Theme) -> mv_text::Style {
    mv_text::Style {
        face: crate::point_rows::readout_face(t, 500),
        size: 17.0,
        tracking: 0.0,
        features: enumset::EnumSet::only(mv_text::Feature::Tabular),
    }
}

/// Scales scrub multiplicatively, else additively.
pub fn scrub_kind(p: mv_viewer::Param) -> mv_scrub::Kind {
    if mv_viewer::is_scale(p) {
        mv_scrub::Kind::Multiplicative
    } else {
        mv_scrub::Kind::Additive
    }
}

/// The drag mode Shift/Ctrl/GUI pick.
pub fn held_mode(mods: Mod) -> mv_scrub::Mode {
    let shift = mods.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD);
    let ctrl = mods.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD);
    let gui = mods.intersects(Mod::LGUIMOD | Mod::RGUIMOD);
    mv_scrub::mode_of(shift, ctrl, gui)
}

/// Starts/stops the rest timer and tip across a hover change.
pub fn box_hover(app: &mut App, from: Hit, to: Hit) {
    if from == to {
        return;
    }
    if from.kind == HitKind::Box {
        crate::app_loop::timer_stop(app, crate::app_loop::Timer::Tip);
        if app.boxes.resting.is_some() {
            app.boxes.resting = None;
            app.dirty = true;
        }
    }
    if to.kind == HitKind::Box {
        crate::app_loop::timer_start(app, crate::app_loop::Timer::Tip, REST_MS);
    }
}

/// Records the hovered box as resting, for its tip.
pub fn box_rested(app: &mut App) {
    if app.input.hover.kind == HitKind::Box {
        let index = app.input.hover.index;
        let param = match index {
            0 => mv_viewer::Param::A,
            1 => mv_viewer::Param::K,
            2 => mv_viewer::Param::B,
            _ => mv_viewer::Param::H,
        };
        app.boxes.resting = Some(param);
        app.dirty = true;
    }
}

/// Starts a pending drag, or handles the field caret, focus and double-click
/// reset.
pub fn box_press(app: &mut App, p: mv_viewer::Param, clicks: i64) {
    crate::app_loop::timer_stop(app, crate::app_loop::Timer::Tip);
    app.boxes.resting = None;
    let editing = crate::number_field::field_open_on(app, p);
    let x = app.input.pointer[0];
    if !editing {
        let value = app.viewer.params[p];
        app.boxes.scrub = Some(Scrub {
            param: p,
            anchor_x: x,
            anchor_value: value,
            last_x: x,
            mode: held_mode(app.input.mods),
            value,
            pending: true,
            reset: false,
        });
    }
    if clicks >= 2 {
        if editing {
            crate::number_field::field_end(app, crate::number_field::FieldEnd::Cancel, false);
        }
        crate::parts::dispatch(
            app,
            mv_viewer::Action::ResetParam(mv_viewer::ResetParam { param: p }),
        );
    } else if editing {
        let hit = Hit {
            kind: HitKind::Box,
            index: p as i64,
        };
        let (r, found) = crate::input::region_of(app, hit);
        if found && crate::scroll::inside(r.r#box, app.input.pointer) {
            crate::number_field::field_point(app, r.r#box, x, false);
            app.boxes.field.selecting = true;
        } else {
            crate::number_field::field_blur(app);
        }
    }
    app.dirty = true;
}

/// Promotes a past-threshold press to a drag, following modifiers, and sends
/// the value.
pub fn box_drag(app: &mut App) {
    let x = app.input.pointer[0];
    if app.boxes.field.selecting {
        let part = app.boxes.field.part;
        let (r, found) = crate::input::region_of(app, crate::number_field::field_hit(part));
        if found {
            crate::number_field::field_point(app, r.r#box, x, true);
        }
        return;
    }
    if app.boxes.scrub.is_none() {
        return;
    }
    if app.boxes.scrub.as_ref().expect("scrub held").pending {
        let anchor_x = app.boxes.scrub.as_ref().expect("scrub held").anchor_x;
        if (x - anchor_x).abs() < mv_scrub::DRAG_THRESHOLD as f32 {
            return;
        }
        // The drag shows from here, measured from the press, so nothing jumps.
        let mode = held_mode(app.input.mods);
        app.boxes.scrub.as_mut().expect("scrub held").mode = mode;
        scrub_begin(app);
    } else {
        let next = held_mode(app.input.mods);
        if next != app.boxes.scrub.as_ref().expect("scrub held").mode {
            scrub_mode(app, next);
        }
    }
    let next = {
        let s = app.boxes.scrub.as_mut().expect("scrub held");
        s.last_x = x;
        mv_scrub::delta(
            s.anchor_value,
            f64::from(x - s.anchor_x),
            scrub_kind(s.param),
            s.mode,
        )
    };
    scrub_send(app, next);
}

/// Reports the drag mode and sends the snap value.
fn scrub_begin(app: &mut App) {
    let (param, mode, anchor_value) = {
        let s = app.boxes.scrub.as_mut().expect("scrub held");
        s.pending = false;
        (s.param, s.mode, s.anchor_value)
    };
    crate::parts::dispatch(
        app,
        mv_viewer::Action::SetDrag(mv_viewer::SetDrag {
            part: part_of_param(param),
            mode: Some(mode),
        }),
    );
    if mode == mv_scrub::Mode::Snap {
        scrub_send(
            app,
            mv_scrub::delta(anchor_value, 0.0, scrub_kind(param), mv_scrub::Mode::Snap),
        );
    }
}

/// Starts an immediate equation-term drag, flagging double-click reset.
pub fn term_press(app: &mut App, p: mv_viewer::Param, clicks: i64) {
    let x = app.input.pointer[0];
    let value = app.viewer.params[p];
    app.boxes.scrub = Some(Scrub {
        param: p,
        anchor_x: x,
        anchor_value: value,
        last_x: x,
        mode: held_mode(app.input.mods),
        value,
        pending: false,
        reset: clicks >= 2,
    });
    scrub_begin(app);
}

/// Re-anchors the drag when modifiers change mid-drag.
pub fn box_modifiers(app: &mut App) {
    if app.boxes.scrub.is_none() {
        return;
    }
    let next = held_mode(app.input.mods);
    let switch = {
        let s = app.boxes.scrub.as_ref().expect("scrub held");
        !s.pending && next != s.mode
    };
    if switch {
        scrub_mode(app, next);
    }
}

/// Re-anchors the drag and reports the new mode.
fn scrub_mode(app: &mut App, next: mv_scrub::Mode) {
    let (param, value) = {
        let s = app.boxes.scrub.as_mut().expect("scrub held");
        s.anchor_x = s.last_x;
        s.anchor_value = s.value;
        s.mode = next;
        (s.param, s.value)
    };
    crate::parts::dispatch(
        app,
        mv_viewer::Action::SetDrag(mv_viewer::SetDrag {
            part: part_of_param(param),
            mode: Some(next),
        }),
    );
    if next == mv_scrub::Mode::Snap {
        scrub_send(
            app,
            mv_scrub::delta(value, 0.0, scrub_kind(param), mv_scrub::Mode::Snap),
        );
    }
}

/// Dispatches the new value when it changed.
fn scrub_send(app: &mut App, next: f64) {
    let param = app.boxes.scrub.as_ref().expect("scrub held").param;
    if next != app.boxes.scrub.as_ref().expect("scrub held").value {
        app.boxes.scrub.as_mut().expect("scrub held").value = next;
        crate::parts::dispatch(
            app,
            mv_viewer::Action::SetParam(mv_viewer::SetParam {
                param,
                value: next,
                jump: false,
            }),
        );
    }
}

/// Ends the drag, or opens the field on a tap; handles term reset.
pub fn box_release(app: &mut App) {
    app.boxes.field.selecting = false;
    let s = app.boxes.scrub.take();
    let Some(s) = s else {
        return;
    };
    if s.pending {
        crate::number_field::field_open(app, part_of_param(s.param));
    } else {
        crate::parts::dispatch(
            app,
            mv_viewer::Action::SetDrag(mv_viewer::SetDrag {
                part: part_of_param(s.param),
                mode: None,
            }),
        );
    }
    if s.reset {
        crate::parts::dispatch(
            app,
            mv_viewer::Action::ResetParam(mv_viewer::ResetParam { param: s.param }),
        );
    }
}

/// Ends the active drag on focus loss.
pub fn box_cancel(app: &mut App) {
    app.boxes.field.selecting = false;
    let held = matches!(app.boxes.scrub, Some(s) if !s.pending);
    if held {
        let param = app.boxes.scrub.as_ref().expect("scrub held").param;
        crate::parts::dispatch(
            app,
            mv_viewer::Action::SetDrag(mv_viewer::SetDrag {
                part: part_of_param(param),
                mode: None,
            }),
        );
    }
    app.boxes.scrub = None;
}

/// Scrubs the value by wheel notch, unless typing.
pub fn box_wheel(app: &mut App, p: mv_viewer::Param, delta_y: f32, mode: mv_scrub::Mode) -> bool {
    if crate::number_field::field_open_on(app, p) {
        return false;
    }
    let from = app.viewer.params[p];
    let next = mv_scrub::delta(
        from,
        mv_scrub::wheel_dx(f64::from(delta_y), 0, 15.0),
        scrub_kind(p),
        mode,
    );
    if next != from {
        crate::parts::dispatch(
            app,
            mv_viewer::Action::SetParam(mv_viewer::SetParam {
                param: p,
                value: next,
                jump: false,
            }),
        );
    }
    true
}

/// Arrows nudge, Enter opens the field, Backspace/Delete resets.
pub fn box_key(app: &mut App, p: mv_viewer::Param, key: Keycode, mods: Mod) -> bool {
    match key {
        Keycode::Right | Keycode::Up | Keycode::Left | Keycode::Down => {
            let direction = if key == Keycode::Right || key == Keycode::Up {
                1.0
            } else {
                -1.0
            };
            let step = if mods.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD) {
                mv_scrub::NUDGE_STEP_SHIFT
            } else {
                mv_scrub::NUDGE_STEP
            };
            let value = app.viewer.params[p];
            let next = mv_scrub::nudge(value, scrub_kind(p), direction * step);
            if next != value {
                crate::parts::dispatch(
                    app,
                    mv_viewer::Action::SetParam(mv_viewer::SetParam {
                        param: p,
                        value: next,
                        jump: false,
                    }),
                );
            }
            true
        }
        Keycode::Return | Keycode::KpEnter => {
            crate::number_field::field_open(app, part_of_param(p));
            true
        }
        Keycode::Backspace | Keycode::Delete => {
            crate::parts::dispatch(
                app,
                mv_viewer::Action::ResetParam(mv_viewer::ResetParam { param: p }),
            );
            true
        }
        _ => false,
    }
}

/// The focus ring shows unless dragging or editing.
pub fn box_ring_shows(app: &mut App, p: mv_viewer::Param) -> bool {
    mv_viewer::part_ui(&app.viewer, part_of_param(p))
        .mode
        .is_none()
        && !crate::number_field::field_open_on(app, p)
}

/// The inner field box from the number-box bounds.
pub fn field_box(r#box: clay::BoundingBox) -> clay::BoundingBox {
    clay::BoundingBox {
        x: r#box.x + FIELD_INSET,
        y: r#box.y + (r#box.height - FIELD_LINE) / 2.0,
        width: r#box.width - 2.0 * FIELD_INSET,
        height: FIELD_LINE,
    }
}

/// A number box's look: its value, inks, fill, stroke and field.
#[derive(Clone, Debug)]
pub struct NumberBox {
    pub value: String,
    pub style: mv_text::Style,
    pub ink: mv_theme::Color,
    pub stroke_ink: mv_theme::Color,
    pub value_ink: mv_theme::Color,
    pub fill: mv_theme::Color,
    pub stroke: f32,
    pub field: super::number_field::FieldLook,
    pub select_ink: mv_theme::Color,
    pub caret_ink: mv_theme::Color,
}

/// Declares the box with its value/field look and region.
pub fn number_box(app: &mut App, t: &mv_theme::Theme, p: mv_viewer::Param, ui: &mv_viewer::PartUi) {
    let hit = Hit {
        kind: HitKind::Box,
        index: p as i64,
    };
    let editing = crate::number_field::field_open_on(app, p);
    let dragging = ui.mode.is_some();
    let ink = t.colors[crate::sidebar::PARAM_INKS[p]];
    let mut look = NumberBox {
        value: String::new(),
        style: field_style(t),
        ink,
        stroke_ink: ink,
        value_ink: ink,
        fill: t.colors[mv_theme::Token::BackgroundMuted],
        stroke: 1.5,
        // The field look only while this box holds the open field (number_box.odin
        // sets it inside `if editing`); otherwise the zero value draws nothing.
        // Every box carrying the open selection would slice a shorter box's
        // text with the field's indices (number_field.rs's draw panics on the
        // out-of-bounds end: i4-loop measured it on flip-then-type).
        field: crate::number_field::FieldLook {
            selection: None,
            caret: None,
        },
        select_ink: mv_theme::plate(t, mv_theme::Token::Primary),
        caret_ink: t.colors[mv_theme::Token::Primary],
    };
    let focused = app.input.focus == hit && app.input.focus_visible && !editing;
    let hot = ui.lit && ui.edit.is_none();
    if app.input.hover == hit || hot || focused || dragging || editing {
        look.stroke = 2.0;
    }
    if dragging {
        look.fill = mv_theme::band(t, crate::sidebar::PARAM_INKS[p]);
        look.value_ink = t.colors[mv_theme::Token::Foreground];
        if let Some(mv_scrub::Mode::Fine) = ui.mode {
            look.stroke_ink = t.colors[mv_theme::Token::Accent];
        } else if let Some(mv_scrub::Mode::Snap) = ui.mode {
            look.stroke_ink = t.colors[mv_theme::Token::Warning];
        }
    }
    if editing {
        let f = &app.boxes.field;
        look.value = String::from_utf8_lossy(&f.text[..f.len as usize]).into_owned();
        look.value_ink = t.colors[mv_theme::Token::Foreground];
        if !f.error.is_empty() {
            look.stroke_ink = t.colors[mv_theme::Token::Destructive];
            look.value_ink = t.colors[mv_theme::Token::Destructive];
        }
        look.field = crate::number_field::field_look(f);
    } else {
        look.value = mv_numfmt::format_stored(app.viewer.params[p], ui.fine);
    }
    let width = number_box_width(mv_text::measure(&mut app.fonts, look.style, &look.value));
    let id = mv_clay::id("Box", p as u32);
    let payload = crate::widgets::Custom::NumberBox(look);
    let layout = clay::LayoutConfig {
        sizing: clay::Sizing {
            width: crate::tip::fixed(width),
            height: crate::tip::fixed(NUMBER_BOX_HEIGHT),
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
    let cursor = if editing {
        Cursor::Text
    } else {
        Cursor::EwResize
    };
    let cursor2 = cursor;
    crate::ui::ui_scope(app, id, &config, |app| {
        crate::input::region(
            app,
            id,
            hit,
            cursor,
            cursor2,
            true,
            clay::CornerRadius {
                topLeft: NUMBER_BOX_RADIUS,
                topRight: NUMBER_BOX_RADIUS,
                bottomLeft: NUMBER_BOX_RADIUS,
                bottomRight: NUMBER_BOX_RADIUS,
            },
        );
    });
}

/// Draws the fill, inset stroke, chevrons, value, selection and caret.
pub fn draw_number_box(app: &mut App, b: [f32; 4], r#box: &NumberBox) {
    let bounds = clay::BoundingBox {
        x: b[0],
        y: b[1],
        width: b[2],
        height: b[3],
    };
    let radius = clay::CornerRadius {
        topLeft: NUMBER_BOX_RADIUS,
        topRight: NUMBER_BOX_RADIUS,
        bottomLeft: NUMBER_BOX_RADIUS,
        bottomRight: NUMBER_BOX_RADIUS,
    };
    app.vg.begin_path();
    crate::draw::rounded_rect(&mut app.vg, bounds, radius);
    app.vg.fill_color(crate::draw::css_background(r#box.fill));
    app.vg.fill();
    // The stroke as the page draws it: a 1 px border, and inside it an inset
    // shadow for the rest, each with its own anti-aliased edge, so their
    // corners meet with the browsers' faint seam.
    crate::effects::draw_inset(&mut app.vg, bounds, radius, 1.0, r#box.stroke_ink);
    crate::effects::draw_inset(
        &mut app.vg,
        crate::effects::grow_box(bounds, -1.0),
        crate::draw::inset_radius(radius, 1.0),
        r#box.stroke - 1.0,
        r#box.stroke_ink,
    );

    let chevron_y = b[1] + (b[3] - CHEVRON_SIZE[1]) / 2.0;
    draw_chevron(
        &mut app.vg,
        [b[0] + NUMBER_BOX_INSET, chevron_y],
        false,
        r#box.ink,
    );
    draw_chevron(
        &mut app.vg,
        [b[0] + b[2] - NUMBER_BOX_INSET - CHEVRON_SIZE[0], chevron_y],
        true,
        r#box.ink,
    );

    // The value right-aligned in its slot, on the field's line box centred in
    // the box.
    let slot_right = b[0] + b[2] - FIELD_INSET;
    let shaped = mv_text::shape(&mut app.fonts, r#box.style, &r#box.value);
    let top = b[1] + (b[3] - FIELD_LINE) / 2.0;
    let baseline = top
        + mv_text::baseline(
            &mut app.fonts,
            r#box.style.face,
            r#box.style.size,
            FIELD_LINE,
        );
    let pen = slot_right - shaped.advance;
    crate::number_field::draw_field_selection(
        app,
        &r#box.style,
        &r#box.value,
        pen,
        top,
        FIELD_LINE,
        r#box.field,
        r#box.select_ink,
    );
    mv_text::draw_shaped(
        &mut app.fonts,
        &mut app.vg,
        r#box.style.size,
        pen,
        baseline,
        &shaped,
        r#box.value_ink,
    );
    crate::number_field::draw_field_caret(
        app,
        &r#box.style,
        &r#box.value,
        pen,
        top,
        FIELD_LINE,
        r#box.field,
        r#box.caret_ink,
    );
}

/// Strokes one 6x12 chevron.
pub fn draw_chevron(vg: &mut mv_nvg::Context, at: [f32; 2], right: bool, color: mv_theme::Color) {
    vg.save();
    vg.translate(at[0], at[1]);
    vg.begin_path();
    let _ = crate::svgpath::draw_path(
        vg,
        if right {
            "M1 2L5 6L1 10"
        } else {
            "M5 2L1 6L5 10"
        },
    );
    vg.stroke_color(color);
    vg.stroke_width(1.6);
    vg.line_cap(mv_nvg::LineCapType::ROUND);
    vg.line_join(mv_nvg::LineCapType::ROUND);
    vg.stroke();
    vg.restore();
}
