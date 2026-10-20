//! One transform parameter's row: its chip, name and scrub value box.
//!
//! Ported from `SideBar/TransformRow.tsx` and `TransformRow.module.css` at
//! `5714319`, with the box grown into `ScrubValue/` at `b8a0e1d` (the drag
//! with its modifiers, the wheel, the arrow keys, Esc, the double-click
//! reset, the tips — [`crate::scrub_value`] owns the machine, this module
//! the row's layout and paint): the 44 px row (`data-param`, `data-lit`),
//! the 26 px chip with the parameter's letter in its colour, the name, and
//! the 107×32 box with two decorative chevrons around the field
//! ([`crate::number_field`]). Pointing at the row or focusing its box
//! highlights the parameter everywhere it shows; the row lights up wherever
//! the highlight comes from.
//!
//! The box's press, keys, wheel, text and tip route through
//! [`crate::scrub_value`]; the row keeps the highlight sync state in
//! h4-readout's [`crate::state`], the layout constants and the chevron
//! geometry.

/// The row: flex, 44 px tall, `0 6px 0 8px` padding, `--space-inset` (12) gap,
/// `--radius-card` (4) corners; lit rows tint with the parameter's colour.
pub const ROW_HEIGHT: f32 = 44.0;
pub const ROW_PAD_LEFT: f32 = 8.0;
pub const ROW_PAD_RIGHT: f32 = 6.0;
pub const ROW_GAP: f32 = 12.0;
pub const ROW_RADIUS: f32 = 4.0;

/// The chip: 26×26 grid, serif 19 px/24 px, the letter on the parameter's
/// colour, `--radius-card` corners. The letter is a `var`: STIX Two Text
/// italic 400 (the face is `Stix400Italic`).
pub const CHIP_SIZE: f32 = 26.0;
pub const CHIP_FONT_SIZE: f32 = 19.0;
pub const CHIP_LINE_HEIGHT: f32 = 24.0;
pub const CHIP_RADIUS: f32 = 4.0;

/// The name: flex 1, 15 px, `--weight-strong` (600), foreground-muted —
/// foreground while lit.
pub const NAME_FONT_SIZE: f32 = 15.0;

/// The box: 107×32, `0 9.5px` padding, gap 6, mono 500 17 px/22 px
/// tabular-nums, the parameter's ink on background-muted, a 1 px ink border
/// plus an inset ink ring (0.5 rest, 1 lit or focused), and a 3 px primary
/// 32 % ring while `:focus-visible` is inside. Clay takes no border space
/// (the h1-frame rule), so its padding is the CSS padding plus the border;
/// the 9.5 px sides are fractional (Clay's padding and gaps are whole px),
/// so 3.5 px edge spacers plus the 6 px gap hold each side, as the picker's
/// dividers hold its 1 px (the Border arm strokes only uniform borders).
pub const BOX_WIDTH: f32 = 107.0;
pub const BOX_HEIGHT: f32 = 32.0;
pub const BOX_PAD_X: f32 = 9.5;
pub const BOX_BORDER: f32 = 1.0;
pub const BOX_GAP: f32 = 6.0;
/// The edge spacer: the side padding minus one gap (`BOX_PAD_X - BOX_GAP`).
pub const BOX_EDGE: f32 = 3.5;
pub const BOX_FONT_SIZE: f32 = 17.0;
pub const BOX_LINE_HEIGHT: f32 = 22.0;
pub const BOX_RADIUS: f32 = 4.0;
/// The inset ring's width at rest and while lit or focused.
pub const BOX_RING_REST: f32 = 0.5;
pub const BOX_RING_LIT: f32 = 1.0;
/// The `:focus-visible` ring's width (primary 32 %, `theme::ring`).
pub const BOX_FOCUS_RING: f32 = 3.0;

/// The field: flex 1, 22 px tall, right-aligned, transparent, no outline;
/// focused text runs in the foreground.
pub const FIELD_HEIGHT: f32 = 22.0;

/// A chevron beside the value: 6×12 svg, 1.6 px round stroke in the
/// parameter's colour, no fill. Decorative (`aria-hidden`): no handler.
pub const CHEVRON_WIDTH: f32 = 6.0;
pub const CHEVRON_HEIGHT: f32 = 12.0;
pub const CHEVRON_STROKE: f32 = 1.6;

/// The left chevron's path (`M5 2L1 6L5 10`).
pub const CHEVRON_LEFT: &str = "M5 2L1 6L5 10";
/// The right chevron's path (`M1 2L5 6L1 10`).
pub const CHEVRON_RIGHT: &str = "M1 2L5 6L1 10";

use mv_sys::clay;

use crate::app::App;
use crate::input::{Cursor, Hit, HitKind};
use crate::number_field::commit_value;
use crate::state::TransformParam;

/// Element ids, one per parameter row.
pub const ROW_ID: &str = "TransformRow";
pub const CHIP_ID: &str = "TransformChip";
pub const NAME_ID: &str = "TransformName";
pub const BOX_ID: &str = "TransformBox";
pub const CHEVRON_ID: &str = "TransformChevron";
pub const FIELD_ID: &str = "TransformField";
pub const FIELD_TEXT_ID: &str = "TransformFieldText";
/// The selection plate around the field's text run.
pub const FIELD_RUN_ID: &str = "TransformFieldRun";

/// The parameter's index: `Hit.index` for its row and field hits, in
/// `TransformParam` order.
#[must_use]
pub fn param_index(param: TransformParam) -> usize {
    match param {
        TransformParam::A => 0,
        TransformParam::B => 1,
        TransformParam::H => 2,
        TransformParam::K => 3,
    }
}

/// The parameter an index names, if any.
#[must_use]
pub fn index_param(index: i64) -> Option<TransformParam> {
    match index {
        0 => Some(TransformParam::A),
        1 => Some(TransformParam::B),
        2 => Some(TransformParam::H),
        3 => Some(TransformParam::K),
        _ => None,
    }
}

/// The row's hit (pointing here highlights the parameter).
#[must_use]
pub fn row_hit(param: TransformParam) -> Hit {
    Hit {
        kind: HitKind::TransformRow,
        index: param_index(param) as i64,
    }
}

/// The field's hit (the row's focus stop).
#[must_use]
pub fn field_hit(param: TransformParam) -> Hit {
    Hit {
        kind: HitKind::TransformField,
        index: param_index(param) as i64,
    }
}

/// The parameter a row or field hit names, if either.
#[must_use]
pub fn hit_param(hit: Hit) -> Option<TransformParam> {
    match hit.kind {
        HitKind::TransformRow | HitKind::TransformField => index_param(hit.index),
        _ => None,
    }
}

/// One row: the chip, the name, and the value box with its chevrons around
/// the field. Lit rows tint with the parameter's colour (`--row-tint`).
pub fn row(app: &mut App, param: TransformParam) {
    let theme = crate::ui::current_theme(app).clone();
    let ink = |token| crate::ui::ink(&theme, token);
    let param_ink = ink(param.colour_token());
    let shown = crate::scrub_value::box_text(app, param);
    let lit = app.state.is_lit(param);
    let focused = app.input.focus == field_hit(param);
    let name_ink = if lit {
        ink(mv_theme::Token::Foreground)
    } else {
        ink(mv_theme::Token::ForegroundMuted)
    };
    let field_ink = if focused {
        ink(mv_theme::Token::Foreground)
    } else {
        param_ink
    };
    let mut decl = crate::ui::empty_decl();
    decl.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
        height: mv_clay::sizing_fixed(ROW_HEIGHT),
    };
    decl.layout.padding = clay::Padding {
        left: ROW_PAD_LEFT as u16,
        right: ROW_PAD_RIGHT as u16,
        top: 0,
        bottom: 0,
    };
    decl.layout.childGap = ROW_GAP as u16;
    decl.layout.childAlignment = clay::ChildAlignment {
        x: clay::LayoutAlignmentX::Left,
        y: clay::LayoutAlignmentY::Center,
    };
    if lit {
        let tint = mv_theme::band(&theme, param.colour_token());
        decl.backgroundColor = [
            tint[0] * 255.0,
            tint[1] * 255.0,
            tint[2] * 255.0,
            tint[3] * 255.0,
        ];
    }
    decl.cornerRadius = crate::ui::corner_radius_all(ROW_RADIUS);
    let index = param_index(param) as u32;
    // Declared before the children: `hit_at` prefers later declarations at
    // equal depth, so the field's region (declared inside) wins their overlap.
    crate::input::region(
        app,
        mv_clay::id(ROW_ID, index),
        row_hit(param),
        Cursor::Default,
        Cursor::Default,
        false,
        crate::ui::corner_radius_all(ROW_RADIUS),
    );
    crate::ui::ui_scope(app, mv_clay::id(ROW_ID, index), &decl, |app| {
        chip(app, param, param_ink, index);
        crate::typography::text(
            app,
            mv_clay::id(NAME_ID, index),
            crate::typography::Type::RowName,
            crate::transform_section::row_name(param),
            name_ink,
            true,
        );
        value_box(app, param, &shown, param_ink, field_ink, index);
    });
}

/// The chip: the parameter's letter on its colour.
fn chip(app: &mut App, param: TransformParam, param_ink: clay::Color, index: u32) {
    let mut decl = crate::ui::empty_decl();
    decl.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_fixed(CHIP_SIZE),
        height: mv_clay::sizing_fixed(CHIP_SIZE),
    };
    decl.layout.childAlignment = clay::ChildAlignment {
        x: clay::LayoutAlignmentX::Center,
        y: clay::LayoutAlignmentY::Center,
    };
    decl.backgroundColor = param_ink;
    decl.cornerRadius = crate::ui::corner_radius_all(CHIP_RADIUS);
    crate::ui::ui_scope(app, mv_clay::id(CHIP_ID, index), &decl, |app| {
        let theme = crate::ui::current_theme(app).clone();
        crate::typography::text(
            app,
            mv_clay::id("TransformChipLetter", index),
            crate::typography::Type::ChipLetter,
            param.as_str(),
            crate::ui::ink(&theme, mv_theme::Token::Background),
            false,
        );
    });
}

/// The value box: the chevrons around the field, and the tip under it. The
/// 9.5 px side padding is fractional (Clay's padding is whole px), so
/// spacers hold it, as the picker's dividers hold its 1 px (the Border arm
/// strokes only uniform borders). The whole box is the row's focus stop and
/// scrub target: the field's inner element keeps no region of its own (the
/// DOM field takes no pointer events until it opens for typing). While a
/// scrub holds the box, its fill tints and its border follows the modifier
/// (accent for fine, warning for snap).
fn value_box(
    app: &mut App,
    param: TransformParam,
    shown: &str,
    param_ink: clay::Color,
    field_ink: clay::Color,
    index: u32,
) {
    let theme = crate::ui::current_theme(app).clone();
    let mode = crate::scrub_value::drag_mode(app, param);
    let background = if mode.is_some() {
        let tint = mv_theme::band(&theme, param.colour_token());
        [
            tint[0] * 255.0,
            tint[1] * 255.0,
            tint[2] * 255.0,
            tint[3] * 255.0,
        ]
    } else {
        crate::ui::ink(&theme, mv_theme::Token::BackgroundMuted)
    };
    let stroke = match mode {
        Some(crate::scrub::ScrubMode::Fine) => crate::ui::ink(&theme, mv_theme::Token::Accent),
        Some(crate::scrub::ScrubMode::Snap) => crate::ui::ink(&theme, mv_theme::Token::Warning),
        _ => param_ink,
    };
    let mut decl = crate::ui::empty_decl();
    decl.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_fixed(BOX_WIDTH),
        height: mv_clay::sizing_fixed(BOX_HEIGHT),
    };
    // CSS padding 0 plus the 1 px border: Clay's border takes no space.
    decl.layout.padding = clay::Padding {
        left: BOX_BORDER as u16,
        right: BOX_BORDER as u16,
        top: BOX_BORDER as u16,
        bottom: BOX_BORDER as u16,
    };
    decl.layout.childGap = BOX_GAP as u16;
    decl.layout.childAlignment = clay::ChildAlignment {
        x: clay::LayoutAlignmentX::Left,
        y: clay::LayoutAlignmentY::Center,
    };
    decl.backgroundColor = background;
    decl.border = clay::BorderElementConfig {
        color: stroke,
        width: mv_clay::border_outside(BOX_BORDER as u16),
    };
    decl.cornerRadius = crate::ui::corner_radius_all(BOX_RADIUS);
    crate::ui::ui_scope(app, mv_clay::id(BOX_ID, index), &decl, |app| {
        edge(app, index, 0);
        chevron(app, true, index);
        field(app, param, shown, field_ink, index);
        chevron(app, false, index);
        edge(app, index, 1);
        crate::scrub_value::tip(app, param, index);
    });
    let focused = app.input.focus == field_hit(param);
    crate::input::region(
        app,
        mv_clay::id(BOX_ID, index),
        field_hit(param),
        if focused {
            Cursor::Text
        } else {
            Cursor::EwResize
        },
        if focused {
            Cursor::Text
        } else {
            Cursor::EwResize
        },
        true,
        crate::ui::corner_radius_all(BOX_RADIUS),
    );
}

/// A side edge: `BOX_EDGE` (3.5 px) plus the gap holds the 9.5 px side.
fn edge(app: &mut App, index: u32, side: u32) {
    let mut decl = crate::ui::empty_decl();
    decl.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_fixed(BOX_EDGE),
        height: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
    };
    crate::ui::ui_scope(
        app,
        mv_clay::id("TransformBoxPad", index * 2 + side),
        &decl,
        |_| {},
    );
}

/// A chevron beside the value (painted in the post-pass).
fn chevron(app: &mut App, left: bool, index: u32) {
    let mut decl = crate::ui::empty_decl();
    decl.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_fixed(CHEVRON_WIDTH),
        height: mv_clay::sizing_fixed(CHEVRON_HEIGHT),
    };
    crate::ui::ui_scope(
        app,
        mv_clay::id(CHEVRON_ID, index * 2 + u32::from(!left)),
        &decl,
        |_| {},
    );
}

/// The field: the shown text right-aligned in the remaining width. While the
/// caret covers the whole text, the run carries the browser's selection
/// fill (scheme-picked, the text keeping the focused ink in both).
fn field(app: &mut App, param: TransformParam, shown: &str, field_ink: clay::Color, index: u32) {
    let mut decl = crate::ui::empty_decl();
    decl.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
        height: mv_clay::sizing_fit(crate::ui::ZERO_MINMAX),
    };
    decl.layout.childAlignment = clay::ChildAlignment {
        x: clay::LayoutAlignmentX::Right,
        y: clay::LayoutAlignmentY::Center,
    };
    crate::ui::ui_scope(app, mv_clay::id(FIELD_ID, index), &decl, |app| {
        // The selection shows only while focused: leaving the field (Esc,
        // reset, clicking away) keeps the caret but drops the paint, as the
        // blurred DOM input does.
        let selected = app.input.focus == field_hit(param) && app.drafts[param].is_selected(shown);
        let mut run = crate::ui::empty_decl();
        run.layout.sizing = clay::Sizing {
            width: mv_clay::sizing_fit(crate::ui::ZERO_MINMAX),
            height: mv_clay::sizing_fit(crate::ui::ZERO_MINMAX),
        };
        if selected {
            let dark = matches!(
                app.theme.resolved_scheme(),
                crate::theme_state::SystemScheme::Dark
            );
            run.backgroundColor = if dark {
                crate::scrub_value::SELECTION_DARK
            } else {
                crate::scrub_value::SELECTION_LIGHT
            };
        }
        crate::ui::ui_scope(app, mv_clay::id(FIELD_RUN_ID, index), &run, |app| {
            crate::typography::text(
                app,
                mv_clay::id(FIELD_TEXT_ID, index),
                crate::typography::Type::FieldText,
                shown,
                field_ink,
                false,
            );
        });
    });
}

/// Paints the card's rings and chevrons after the command walk (the rings
/// are box-shadows Clay has no arm for; the chevrons are paths, not text).
/// Call from `draw_page` after `draw_commands`, as the settings chrome is.
pub fn paint_rows(app: &mut App) {
    let theme = crate::ui::current_theme(app).clone();
    let density = density_of(app);
    for param in crate::transform_section::ROW_ORDER {
        let index = param_index(param) as u32;
        let Some(frame) = element_box(app, mv_clay::id(BOX_ID, index)) else {
            continue;
        };
        let ink = crate::draw::nvg_color(crate::ui::ink(&theme, param.colour_token()));
        // The inset ring follows the scrub modifier (accent for fine, warning
        // for snap), as `--box-stroke` does; the chevrons keep the ink.
        let ring_ink = match crate::scrub_value::drag_mode(app, param) {
            Some(crate::scrub::ScrubMode::Fine) => {
                crate::draw::nvg_color(crate::ui::ink(&theme, mv_theme::Token::Accent))
            }
            Some(crate::scrub::ScrubMode::Snap) => {
                crate::draw::nvg_color(crate::ui::ink(&theme, mv_theme::Token::Warning))
            }
            _ => ink,
        };
        // The inset ring: 1 px while lit or focused, 0.5 px at rest. CSS
        // paints the inset shadow inside the border (the reference holds two
        // full border rows plus one ring row), so the path sits a border
        // inside the box, not on its edge.
        let lit = app.state.is_lit(param);
        let focused = app.input.focus == field_hit(param);
        let ring = if lit || focused {
            BOX_RING_LIT
        } else {
            BOX_RING_REST
        };
        let edge = BOX_BORDER + ring / 2.0;
        // The frame snaps first (as the Border arm snaps); the ring rect
        // itself stays exact — snapping its quarter-device offsets would drag
        // the hairline half a device pixel off its row.
        let frame = crate::draw::snap_box(frame, density);
        app.vg.begin_path();
        crate::draw::rounded_rect(
            &mut app.vg,
            clay::BoundingBox {
                x: frame.x + edge,
                y: frame.y + edge,
                width: frame.width - 2.0 * edge,
                height: frame.height - 2.0 * edge,
            },
            crate::ui::corner_radius_all(BOX_RADIUS - edge),
        );
        app.vg.stroke_color(ring_ink);
        app.vg.stroke_width(ring);
        app.vg.stroke();
        // The `:focus-visible` ring: 3 px of primary at 32 % outside the box.
        if focused && app.input.focus_visible {
            let primary = crate::ui::ink(&theme, mv_theme::Token::Primary);
            let halo = [primary[0], primary[1], primary[2], primary[3] * 0.32];
            app.vg.begin_path();
            crate::draw::rounded_rect(
                &mut app.vg,
                crate::draw::snap_box(
                    clay::BoundingBox {
                        x: frame.x - BOX_FOCUS_RING / 2.0,
                        y: frame.y - BOX_FOCUS_RING / 2.0,
                        width: frame.width + BOX_FOCUS_RING,
                        height: frame.height + BOX_FOCUS_RING,
                    },
                    density,
                ),
                crate::ui::corner_radius_all(BOX_RADIUS + BOX_FOCUS_RING / 2.0),
            );
            app.vg.stroke_color(crate::draw::nvg_color(halo));
            app.vg.stroke_width(BOX_FOCUS_RING);
            app.vg.stroke();
        }
        paint_chevron(app, ink, density, index, true);
        paint_chevron(app, ink, density, index, false);
    }
    crate::scrub_value::paint_extras(app);
}

/// Paints one chevron in its box: the 1.6 px round path in the ink.
fn paint_chevron(app: &mut App, ink: mv_nvg::Color, density: f32, index: u32, left: bool) {
    let Some(frame) = element_box(app, mv_clay::id(CHEVRON_ID, index * 2 + u32::from(!left)))
    else {
        return;
    };
    let snapped = crate::draw::snap_box(frame, density);
    // `M5 2L1 6L5 10` (left) / `M1 2L5 6L1 10` (right) in the 6x12 box.
    let (ax, bx) = if left { (5.0, 1.0) } else { (1.0, 5.0) };
    app.vg.save();
    app.vg.translate(snapped.x, snapped.y);
    app.vg.begin_path();
    app.vg.move_to(ax, 2.0);
    app.vg.line_to(bx, 6.0);
    app.vg.line_to(ax, 10.0);
    app.vg.stroke_color(ink);
    app.vg.stroke_width(CHEVRON_STROKE);
    app.vg.line_cap(mv_nvg::LineCapType::ROUND);
    app.vg.line_join(mv_nvg::LineCapType::ROUND);
    app.vg.stroke();
    app.vg.restore();
}

/// A laid-out element's box, when the frame laid it out.
fn element_box(app: &App, id: clay::ElementId) -> Option<clay::BoundingBox> {
    let data = app
        .clay
        .as_ref()
        .expect("clay context initialised")
        .get_element_data(id);
    data.found.then_some(data.boundingBox)
}

/// The density the frame draws at (settings' `paint_chrome` rule).
fn density_of(app: &App) -> f32 {
    if app.size[0] > 0.0 {
        app.pixels[0] as f32 / app.size[0]
    } else {
        1.0
    }
}

/// Syncs the highlight with the pointer's move across the rows: leaving a
/// row takes the pointer off its parameter, entering one puts it on
/// (`onPointerLeave` / `onPointerEnter`). Runs in `retarget` on a hover
/// change only, and skips moves within one parameter's row and field, so a
/// move inside never re-stamps the touch (the DOM fires enter once).
pub fn sync_hover(app: &mut App, before: Hit, now: Hit) {
    // The rest tip arms on box enters first: a move from row padding into
    // the box enters the box with the parameter unchanged, which the lead
    // sync below skips.
    crate::scrub_value::hover_moved(app, before, now);
    let was = hit_param(before);
    let is = hit_param(now);
    if was == is {
        return;
    }
    if was.is_some() {
        app.state.set_hover(None);
    }
    if let Some(param) = is {
        app.state.set_hover(Some(param));
    }
}

/// Routes a pointer press: the scrub box owns box presses (starting a scrub,
/// keeping a tap, or blurring the old field first); anything else blurs, as
/// below. Call after `settings::press_down`, as the Button arm chains the
/// milestones.
pub fn press_down(app: &mut App) {
    crate::scrub_value::press_down(app);
}

/// Routes a key to the focused field. Enter commits (keeping the focus, as
/// the reference keeps editing); Tab commits and moves focus stepwise, as
/// blur-then-focus does; Ctrl/Command+A selects the box's text for the next
/// typed text to replace, as the browser does. The scrub box's own keys —
/// the nudge and caret arrows, Home/End, Backspace/Delete, Esc — run first
/// ([`crate::scrub_value::field_key`], which does not overlap these arms).
/// Returns whether consumed.
pub fn key(app: &mut App, event: &crate::input::KeyEvent) -> bool {
    use sdl3::keyboard::Keycode;
    if !event.down || event.repeat {
        return false;
    }
    if crate::scrub_value::field_key(app, event) {
        return true;
    }
    if app.input.focus.kind != HitKind::TransformField {
        return false;
    }
    let Some(param) = index_param(app.input.focus.index) else {
        return false;
    };
    match event.key {
        Keycode::Return | Keycode::KpEnter => {
            commit_param(app, param);
            true
        }
        Keycode::Tab => {
            commit_param(app, param);
            app.drafts[param].typing_from = None;
            let shift = event
                .mods
                .intersects(sdl3::keyboard::Mod::LSHIFTMOD | sdl3::keyboard::Mod::RSHIFTMOD);
            crate::input::move_focus(app, if shift { -1 } else { 1 });
            sync_focus_state(app);
            // Tabbing into a field arms its restore point and parks the
            // caret at the end, as focusing the DOM input would.
            if let Some(landed) = hit_param(app.input.focus) {
                let value = crate::transform_section::param_value(app, landed);
                app.drafts[landed].typing_from = Some(value);
                let shown = crate::scrub_value::box_text(app, landed);
                let len = shown.chars().count();
                app.drafts[landed].caret = [len, len];
            }
            true
        }
        Keycode::A if event.mods.intersects(ctrl_mods()) => {
            let shown = crate::scrub_value::box_text(app, param);
            crate::number_field::select_all(&mut app.drafts[param], &shown);
            app.dirty = true;
            true
        }
        _ => false,
    }
}

/// Ctrl or Command: the modifiers that select all with A.
fn ctrl_mods() -> sdl3::keyboard::Mod {
    sdl3::keyboard::Mod::LCTRLMOD
        | sdl3::keyboard::Mod::RCTRLMOD
        | sdl3::keyboard::Mod::LGUIMOD
        | sdl3::keyboard::Mod::RGUIMOD
}

/// Routes typed text to the focused field, inserting at the caret
/// ([`crate::scrub_value::type_text`]).
pub fn text(app: &mut App, text: &str) {
    crate::scrub_value::type_text(app, text);
}

/// The focused draft commits: Enter's and blur's shared half (`commit`).
/// The caret parks at the end of the committed text, as the DOM input moves
/// it when its value is set.
pub fn commit_param(app: &mut App, param: TransformParam) {
    let draft = app.drafts[param].take();
    // Taking the draft always repaints: the box falls back to the value
    // whether the parse takes (the value moves) or refuses (the old value
    // comes back) — without this a refused Enter leaves its text stranded.
    if draft.is_some() {
        app.dirty = true;
    }
    let next = commit_value(
        draft.as_deref(),
        crate::transform_section::param_def(param).parse,
    );
    if let Some(value) = next {
        match param {
            TransformParam::A => app.a_param.set_flushed(&mut app.url, value),
            TransformParam::K => app.k_param.set_flushed(&mut app.url, value),
            TransformParam::B => app.b_param.set_flushed(&mut app.url, value),
            TransformParam::H => app.h_param.set_flushed(&mut app.url, value),
        }
        app.dirty = true;
    }
    let len = crate::scrub_value::box_text(app, param).chars().count();
    app.drafts[param].caret = [len, len];
}

/// Focuses a field: the keyboard stop, the highlight, and the platform's
/// text input (as a focused DOM input takes keys). The selection and
/// restore point are the opener's (click selects, Tab parks at the end).
pub(crate) fn focus_field(app: &mut App, param: TransformParam) {
    crate::input::set_focus(app, field_hit(param), true);
    app.state.set_focus(Some(param));
    start_text_input(app);
}

/// The highlight follows the keyboard's focus (Tab moves, presses clear).
fn sync_focus_state(app: &mut App) {
    let param = hit_param(app.input.focus);
    app.state.set_focus(param);
}

/// Starts the platform's text input for the focused field; headless tests
/// have no window, so nothing starts there.
pub(crate) fn start_text_input(app: &mut App) {
    if app.window.is_none() {
        return;
    }
    app.window().subsystem().text_input().start(app.window());
}

/// Stops the platform's text input on blur; headless, nothing runs.
pub(crate) fn stop_text_input(app: &mut App) {
    if app.window.is_none() {
        return;
    }
    app.window().subsystem().text_input().stop(app.window());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::HighlightState;

    #[test]
    fn letters_match_the_url_keys() {
        assert_eq!(
            [
                TransformParam::A,
                TransformParam::B,
                TransformParam::H,
                TransformParam::K
            ]
            .map(TransformParam::as_str),
            ["a", "b", "h", "k"]
        );
    }

    #[test]
    fn layout_matches_the_css() {
        assert_eq!(ROW_HEIGHT, 44.0);
        assert_eq!((ROW_PAD_LEFT, ROW_PAD_RIGHT, ROW_GAP), (8.0, 6.0, 12.0));
        assert_eq!(CHIP_SIZE, 26.0);
        assert_eq!(
            (BOX_WIDTH, BOX_HEIGHT, BOX_PAD_X, BOX_GAP),
            (107.0, 32.0, 9.5, 6.0)
        );
        assert_eq!(
            (CHEVRON_WIDTH, CHEVRON_HEIGHT, CHEVRON_STROKE),
            (6.0, 12.0, 1.6)
        );
        assert_eq!(FIELD_HEIGHT, 22.0);
    }

    #[test]
    fn hovering_a_row_lights_it_and_its_equation_together() {
        // The highlight test: a fresh page shows a's row unlit.
        let mut state = HighlightState::touched_out();
        assert!(!state.is_lit(TransformParam::A));

        // Pointing at the row lights it (the form's term reads the same state).
        state.set_hover(Some(TransformParam::A));
        assert!(state.is_lit(TransformParam::A));

        // Moving off it puts it out.
        state.set_hover(None);
        assert!(!state.is_lit(TransformParam::A));
    }

    #[test]
    fn focus_keeps_a_lit_after_the_pointer_leaves() {
        // The highlight test: focusing a's box lights a.
        let mut state = HighlightState::touched_out();
        state.set_focus(Some(TransformParam::A));
        assert!(state.is_lit(TransformParam::A));

        // Pointing at the row and moving off leaves a lit (focus leads now).
        state.set_hover(Some(TransformParam::A));
        state.set_hover(None);
        assert!(state.is_lit(TransformParam::A));

        // Leaving the box puts it out.
        state.set_focus(None);
        assert!(!state.is_lit(TransformParam::A));
    }

    #[test]
    fn only_the_last_moved_parameter_lights() {
        // The highlight test: focusing a's box lights a, not k.
        let mut state = HighlightState::touched_out();
        state.set_focus(Some(TransformParam::A));
        assert!(state.is_lit(TransformParam::A));
        assert!(!state.is_lit(TransformParam::K));

        // Pointing at k lights k and puts a out.
        state.set_hover(Some(TransformParam::K));
        assert!(state.is_lit(TransformParam::K));
        assert!(!state.is_lit(TransformParam::A));

        // Moving off k gives the light back to a, which still has the focus.
        state.set_hover(None);
        assert!(!state.is_lit(TransformParam::K));
        assert!(state.is_lit(TransformParam::A));
    }

    #[test]
    fn clearing_a_side_hands_the_light_to_the_other() {
        // Hover moved last, then leaves: the focused parameter lights.
        let mut state = HighlightState::touched_out();
        state.set_focus(Some(TransformParam::A));
        state.set_hover(Some(TransformParam::K));
        state.set_hover(None);
        assert!(state.is_lit(TransformParam::A));
    }
}
