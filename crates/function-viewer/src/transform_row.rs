//! One transform parameter's row: its chip, name and scrub value box.
//!
//! Ported from `SideBar/TransformRow.tsx` and `TransformRow.module.css` at
//! `5714319`, with the box grown into `ScrubValue/` at `b8a0e1d` (the drag
//! with its modifiers, the wheel, the arrow keys, Esc, the double-click
//! reset, the tips — [`crate::scrub_value`] owns the machine, this module
//! the row's layout and paint) plus the flip toggle on the two scale rows
//! and the reset button in its kept 24 px slot: the 44 px row (`data-param`,
//! `data-lit`), the 26 px chip with the parameter's letter in its colour,
//! the name, and the 107×32 box with two decorative chevrons around the
//! field ([`crate::number_field`]). Pointing at the row or focusing its box
//! highlights the parameter everywhere it shows; the row lights up wherever
//! the highlight comes from.
//!
//! The box's press, keys, wheel, text and tip route through
//! [`crate::scrub_value`]; the buttons' behaviour (changed rule, labels,
//! visibility, flip) is [`crate::reset`], their icons paint through
//! [`crate::icons`] (`Reset`, `FlipVertical`, `FlipHorizontal`); the row
//! keeps the highlight sync state in h4-readout's [`crate::state`], the
//! layout constants and the chevron geometry.

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

/// The flip toggle on the two scale rows and the reset button: 24×24 grids,
/// no padding or border, `--radius-card` corners, pointer. The toggle sits
/// between the name and the slot, inked with the row's colour on
/// background-muted (secondary while hovered, the row's ink behind the
/// background ink while pressed, a 3 px primary-32 % ring on focus-visible);
/// the reset is transparent, foreground-muted (foreground on secondary while
/// hovered).
pub const FLIP_SIZE: f32 = 24.0;
pub const RESET_SIZE: f32 = 24.0;

/// The reset button's slot: flex-none, 24 px wide, kept whether or not the
/// button shows — and while a drag hides it — so the flip toggle holds still
/// (`8261442`, divergences.md "The flip toggle holds still").
pub const RESET_SLOT_WIDTH: f32 = 24.0;

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
pub const FLIP_ID: &str = "TransformFlip";
pub const FLIP_ICON_ID: &str = "TransformFlipIcon";
pub const RESET_SLOT_ID: &str = "TransformResetSlot";
pub const RESET_ID: &str = "TransformReset";
pub const RESET_ICON_ID: &str = "TransformResetIcon";
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

/// A scale row's flip toggle hit (a focus stop, in tab order).
#[must_use]
pub fn flip_hit(param: TransformParam) -> Hit {
    Hit {
        kind: HitKind::TransformFlip,
        index: param_index(param) as i64,
    }
}

/// A row's reset button hit (never a focus stop: `tabIndex={-1}`).
#[must_use]
pub fn reset_hit(param: TransformParam) -> Hit {
    Hit {
        kind: HitKind::TransformReset,
        index: param_index(param) as i64,
    }
}

/// The parameter a row, field, flip or reset hit names, if any. The buttons
/// sit on their row, so they name it: moving onto them never drops the
/// hover, and the highlight follows.
#[must_use]
pub fn hit_param(hit: Hit) -> Option<TransformParam> {
    match hit.kind {
        HitKind::TransformRow
        | HitKind::TransformField
        | HitKind::TransformFlip
        | HitKind::TransformReset => index_param(hit.index),
        _ => None,
    }
}

/// One row: the chip, the name, the flip toggle on the scale rows, the reset
/// slot, and the value box with its chevrons around the field. Lit rows tint
/// with the parameter's colour (`--row-tint`).
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
        let value = crate::transform_section::param_value(app, param);
        if crate::reset::is_scale(param) {
            flip(app, param, value, param_ink, index);
        }
        reset_slot(app, param, value, index);
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

/// The flip toggle: the 14 px axis mark centred in the 24 px button. The
/// background is the row's ink while pressed (`aria-pressed`), secondary
/// while hovered, background-muted at rest; the mark runs in the background
/// ink while pressed, the row's ink otherwise.
fn flip(app: &mut App, param: TransformParam, value: f64, param_ink: clay::Color, index: u32) {
    let theme = crate::ui::current_theme(app).clone();
    let pressed = crate::reset::flip_pressed(value);
    let hovered = app.input.hover == flip_hit(param);
    let mut decl = crate::ui::empty_decl();
    decl.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_fixed(FLIP_SIZE),
        height: mv_clay::sizing_fixed(FLIP_SIZE),
    };
    decl.layout.childAlignment = clay::ChildAlignment {
        x: clay::LayoutAlignmentX::Center,
        y: clay::LayoutAlignmentY::Center,
    };
    decl.backgroundColor = if pressed {
        param_ink
    } else if hovered {
        crate::ui::ink(&theme, mv_theme::Token::Secondary)
    } else {
        crate::ui::ink(&theme, mv_theme::Token::BackgroundMuted)
    };
    decl.cornerRadius = crate::ui::corner_radius_all(ROW_RADIUS);
    crate::ui::ui_scope(app, mv_clay::id(FLIP_ID, index), &decl, |app| {
        let mark = if pressed {
            crate::ui::ink(&theme, mv_theme::Token::Background)
        } else {
            param_ink
        };
        icon(
            app,
            mv_clay::id(FLIP_ICON_ID, index),
            crate::reset::flip_icon(param),
            crate::reset::FLIP_ICON_SIZE,
            mark,
        );
    });
    crate::input::region(
        app,
        mv_clay::id(FLIP_ID, index),
        flip_hit(param),
        Cursor::Pointer,
        Cursor::Pointer,
        true,
        crate::ui::corner_radius_all(ROW_RADIUS),
    );
}

/// The reset slot: 24 px kept whether or not the button shows, so the flip
/// toggle holds still. The button paints while the value is off its default
/// and no drag holds it; transparent at rest, secondary while hovered, the
/// mark foreground-muted (foreground while hovered).
fn reset_slot(app: &mut App, param: TransformParam, value: f64, index: u32) {
    let theme = crate::ui::current_theme(app).clone();
    let mut decl = crate::ui::empty_decl();
    decl.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_fixed(RESET_SLOT_WIDTH),
        height: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
    };
    decl.layout.childAlignment = clay::ChildAlignment {
        x: clay::LayoutAlignmentX::Center,
        y: clay::LayoutAlignmentY::Center,
    };
    crate::ui::ui_scope(app, mv_clay::id(RESET_SLOT_ID, index), &decl, |app| {
        let dragged = crate::reset::dragged_param(app) == Some(param);
        if !crate::reset::reset_shown(crate::reset::is_changed(param, value), dragged) {
            return;
        }
        let hovered = app.input.hover == reset_hit(param);
        let mut button = crate::ui::empty_decl();
        button.layout.sizing = clay::Sizing {
            width: mv_clay::sizing_fixed(RESET_SIZE),
            height: mv_clay::sizing_fixed(RESET_SIZE),
        };
        button.layout.childAlignment = clay::ChildAlignment {
            x: clay::LayoutAlignmentX::Center,
            y: clay::LayoutAlignmentY::Center,
        };
        if hovered {
            button.backgroundColor = crate::ui::ink(&theme, mv_theme::Token::Secondary);
        }
        button.cornerRadius = crate::ui::corner_radius_all(ROW_RADIUS);
        crate::ui::ui_scope(app, mv_clay::id(RESET_ID, index), &button, |app| {
            let mark = if hovered {
                crate::ui::ink(&theme, mv_theme::Token::Foreground)
            } else {
                crate::ui::ink(&theme, mv_theme::Token::ForegroundMuted)
            };
            icon(
                app,
                mv_clay::id(RESET_ICON_ID, index),
                crate::icons::IconKind::Reset,
                crate::reset::RESET_ICON_SIZE,
                mark,
            );
        });
        crate::input::region(
            app,
            mv_clay::id(RESET_ID, index),
            reset_hit(param),
            Cursor::Pointer,
            Cursor::Pointer,
            false,
            crate::ui::corner_radius_all(ROW_RADIUS),
        );
    });
}

/// Declares a square icon element painting `kind` in `ink`, as the settings
/// menu's icon does. The port's icons take 0-to-1 colours, so the 0-to-255
/// Clay ink converts at the boundary.
fn icon(
    app: &mut App,
    id: clay::ElementId,
    kind: crate::icons::IconKind,
    size: f32,
    ink: clay::Color,
) {
    let mut decl = crate::ui::empty_decl();
    decl.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_fixed(size),
        height: mv_clay::sizing_fixed(size),
    };
    decl.custom = crate::icons::custom_data(crate::icons::Icon {
        kind,
        ink: crate::draw::nvg_color(ink),
    });
    crate::ui::ui_scope(app, id, &decl, |_| {});
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
        // The toggle's `:focus-visible` ring: 3 px of primary at 32 % outside
        // the button, as the field's. The reset is never keyboard-focused
        // (`tabIndex={-1}`), so it paints no ring.
        if crate::reset::is_scale(param)
            && app.input.focus == flip_hit(param)
            && app.input.focus_visible
        {
            paint_focus_ring(app, &theme, density, mv_clay::id(FLIP_ID, index));
        }
    }
    crate::scrub_value::paint_extras(app);
}

/// Paints a 3 px primary-32 % ring outside a button's box (the `:focus-visible`
/// rule the toggle and Reset all share with the field).
pub fn paint_focus_ring(app: &mut App, theme: &mv_theme::Theme, density: f32, id: clay::ElementId) {
    let Some(frame) = element_box(app, id) else {
        return;
    };
    let primary = crate::ui::ink(theme, mv_theme::Token::Primary);
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
        crate::ui::corner_radius_all(ROW_RADIUS + BOX_FOCUS_RING / 2.0),
    );
    app.vg.stroke_color(crate::draw::nvg_color(halo));
    app.vg.stroke_width(BOX_FOCUS_RING);
    app.vg.stroke();
}

/// Paints one chevron in its box: the 1.6 px round path in the ink. A
/// chevron fully behind a visible tip stays unpainted: the tip floats above
/// the rows (as the DOM stacks it) while the chevrons paint in this
/// post-pass, so an unskipped chevron would print its strokes over the tip's
/// text (h5-box pixels review F1: k's gold chevrons over a's tip).
fn paint_chevron(app: &mut App, ink: mv_nvg::Color, density: f32, index: u32, left: bool) {
    let Some(frame) = element_box(app, mv_clay::id(CHEVRON_ID, index * 2 + u32::from(!left)))
    else {
        return;
    };
    for tip in 0..4_u32 {
        if let Some(tip) = element_box(app, mv_clay::id(crate::scrub_value::TIP_ID, tip))
            && box_contains(tip, frame)
        {
            return;
        }
    }
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

/// Whether `inner` sits fully inside `outer` (layout space): the tip-cover
/// test for post-pass glyphs.
#[must_use]
fn box_contains(outer: clay::BoundingBox, inner: clay::BoundingBox) -> bool {
    outer.x <= inner.x
        && outer.y <= inner.y
        && inner.x + inner.width <= outer.x + outer.width
        && inner.y + inner.height <= outer.y + outer.height
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

/// Routes a click: the release whose down-hit and up-hit agree, as the
/// buttons' `onClick` needs both. The flip negates its value, the reset puts
/// its default back; both blur the button (`b6457b7`: a mouse click focuses
/// it, and the focus would keep the row lit after the pointer leaves).
/// Reset all puts every default back and keeps its focus (the source blurs
/// nothing there).
pub fn click(app: &mut App, down: Hit) {
    let up = app.input.hover;
    if down != up {
        return;
    }
    match down.kind {
        HitKind::TransformFlip => {
            if let Some(param) = index_param(down.index) {
                let value = crate::transform_section::param_value(app, param);
                store_param(app, param, crate::reset::flipped(value));
                blur_button(app);
            }
        }
        HitKind::TransformReset => {
            if let Some(param) = index_param(down.index) {
                store_param(app, param, crate::reset::reset_value(param));
                blur_button(app);
            }
        }
        HitKind::TransformResetAll => {
            crate::transform_section::reset_all_params(app);
        }
        _ => {}
    }
}

/// The click's blur: the button unfocuses, so the row lights from the hover
/// alone and drops it once the pointer moves off.
fn blur_button(app: &mut App) {
    app.input.focus = Hit::default();
    app.input.focus_visible = false;
    app.state.set_focus(None);
    app.dirty = true;
}

/// Routes a key to the focused control. Space and Enter flip the focused
/// toggle or reset all (a keyboard click has `detail` 0, so nothing blurs).
/// The scrub box's own keys — the nudge and caret arrows, Home/End,
/// Backspace/Delete, Esc — run first ([`crate::scrub_value::field_key`]);
/// Tab from a toggle or Reset all walks on, Tab from a field commits and
/// walks with the caret armed ([`crate::scrub_value`] owns the field arms,
/// which do not overlap these). Returns whether consumed.
pub fn key(app: &mut App, event: &crate::input::KeyEvent) -> bool {
    use sdl3::keyboard::Keycode;
    if !event.down || event.repeat {
        return false;
    }
    // A keyboard click on a toggle or Reset all (`detail` 0, so nothing
    // blurs): the value flips or every default comes back, keeping the focus.
    if matches!(
        event.key,
        Keycode::Space | Keycode::Return | Keycode::KpEnter
    ) && matches!(
        app.input.focus.kind,
        HitKind::TransformFlip | HitKind::TransformResetAll
    ) {
        let focus = app.input.focus;
        match focus.kind {
            HitKind::TransformFlip => {
                if let Some(param) = index_param(focus.index) {
                    let value = crate::transform_section::param_value(app, param);
                    store_param(app, param, crate::reset::flipped(value));
                }
            }
            _ => {
                crate::transform_section::reset_all_params(app);
            }
        }
        return true;
    }
    if crate::scrub_value::field_key(app, event) {
        return true;
    }
    // Tab walks on from a toggle or Reset all without committing (fields
    // commit in their own arm below). Any other stop walks on too (the
    // browser's default when no milestone owns the key): the highlight
    // follows, clearing the parameter when the focus leaves the rows.
    if event.key == Keycode::Tab && app.input.focus.kind != HitKind::TransformField {
        match app.input.focus.kind {
            HitKind::TransformFlip | HitKind::TransformResetAll => {}
            _ => {
                if app.settings.open {
                    return false;
                }
            }
        }
        let shift = event
            .mods
            .intersects(sdl3::keyboard::Mod::LSHIFTMOD | sdl3::keyboard::Mod::RSHIFTMOD);
        crate::input::move_focus(app, if shift { -1 } else { 1 });
        sync_focus_state(app);
        // Landing on a field arms its restore point and parks the caret at
        // the end, as focusing the DOM input would (the field's own Tab arm
        // does the same coming from a field).
        if app.input.focus.kind == HitKind::TransformField
            && let Some(landed) = index_param(app.input.focus.index)
        {
            let value = crate::transform_section::param_value(app, landed);
            app.drafts[landed].typing_from = Some(value);
            let shown = crate::scrub_value::box_text(app, landed);
            let len = shown.chars().count();
            app.drafts[landed].caret = [len, len];
        }
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
            // caret at the end, as focusing the DOM input would. A toggle
            // landing arms nothing: the field never took the focus, so no
            // typing tip shows (h5-buttons flip-focus).
            if app.input.focus.kind == HitKind::TransformField
                && let Some(landed) = index_param(app.input.focus.index)
            {
                let value = crate::transform_section::param_value(app, landed);
                app.drafts[landed].typing_from = Some(value);
                let shown = crate::scrub_value::box_text(app, landed);
                let len = shown.chars().count();
                app.drafts[landed].caret = [len, len];
            }
            true
        }
        Keycode::Backspace => {
            // Fallback: the box's `field_key` consumes Backspace first (the
            // caret-aware delete); this pops the shown text's last scalar.
            let mut shown = crate::scrub_value::box_text(app, param);
            shown.pop();
            app.drafts[param].draft = Some(shown);
            app.dirty = true;
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

/// Stores a parameter's value, flushed to the URL at once (the hook's `set`
/// with `{ flush: true }`).
fn store_param(app: &mut App, param: TransformParam, value: f64) {
    match param {
        TransformParam::A => app.a_param.set_flushed(&mut app.url, value),
        TransformParam::K => app.k_param.set_flushed(&mut app.url, value),
        TransformParam::B => app.b_param.set_flushed(&mut app.url, value),
        TransformParam::H => app.h_param.set_flushed(&mut app.url, value),
    }
    app.dirty = true;
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
        store_param(app, param, value);
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
        assert_eq!(
            (FLIP_SIZE, RESET_SIZE, RESET_SLOT_WIDTH),
            (24.0, 24.0, 24.0)
        );
    }

    #[test]
    fn flip_keeps_its_place_while_the_reset_shows_and_hides() {
        // The row test: the flip toggle's left edge never moves — the name
        // takes what the flip, the kept slot, the box and the fixed chrome
        // leave, so showing, hiding or drag-hiding the reset changes nothing.
        // Five children, four gaps: 8 + 26 + 12 + name + 12 + 24 + 12 + 24
        // + 12 + 107 + 6 = 350.
        let name = 350.0
            - (ROW_PAD_LEFT
                + CHIP_SIZE
                + FLIP_SIZE
                + RESET_SLOT_WIDTH
                + BOX_WIDTH
                + ROW_PAD_RIGHT
                + 4.0 * ROW_GAP);
        assert_eq!(name, 107.0);
        // The probed flip stands 165 px into its row: 8 + 26 + 12 + 107 + 12.
        assert_eq!(ROW_PAD_LEFT + CHIP_SIZE + ROW_GAP + name + ROW_GAP, 165.0);
    }

    #[test]
    fn clicking_a_button_blur_puts_the_rows_light_out() {
        // The highlight-stuck fix (`b6457b7`): the reset and flip clicks blur
        // the button, so once the pointer moves off, the row is unlit. The
        // click moves no touch itself; the blur drops the row's focus and the
        // pointer-off drops its hover.
        let mut state = HighlightState::touched_out();
        state.set_hover(Some(TransformParam::A));
        state.set_focus(Some(TransformParam::A));
        assert!(state.is_lit(TransformParam::A));

        // The click's blur.
        state.set_focus(None);
        // The pointer moving off after.
        state.set_hover(None);
        assert!(!state.is_lit(TransformParam::A));
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

    #[test]
    fn tab_walks_a_focused_field_onto_the_next_toggle() {
        // flip-focus: k's field focused, Tab moves the focus to b's toggle
        // with the highlight following (blur-then-focus).
        use sdl3::keyboard::{Keycode, Mod};

        let _guard = mv_clay::tests_lock();
        let mut app = crate::fixture::headless_app();
        assert!(mv_text::init(&mut app.fonts));
        crate::ui::init_clay(&mut app);
        app.url = crate::url::Url::from_query("a=2");
        crate::transform_section::init_params_from_url(&mut app);
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
        crate::input::collect_regions(&mut app);

        app.input.focus = field_hit(TransformParam::K);
        app.input.focus_visible = true;
        app.state.set_focus(Some(TransformParam::K));
        let tab = crate::input::KeyEvent {
            key: Keycode::Tab,
            mods: Mod::empty(),
            down: true,
            repeat: false,
        };
        assert!(key(&mut app, &tab));
        assert_eq!(app.input.focus, flip_hit(TransformParam::B));
        assert_eq!(app.state.lit(), Some(TransformParam::B));
        // Tabbing on into the field parks the caret at the end with the
        // restore point armed, as focusing the DOM input would.
        assert!(key(&mut app, &tab));
        assert_eq!(app.input.focus, field_hit(TransformParam::B));
        assert_eq!(app.state.lit(), Some(TransformParam::B));
        assert_eq!(app.drafts[TransformParam::B].typing_from, Some(1.0));
        let len = crate::scrub_value::box_text(&app, TransformParam::B)
            .chars()
            .count();
        assert_eq!(app.drafts[TransformParam::B].caret, [len, len]);
    }

    #[test]
    fn tip_cover_hides_only_whole_chevrons() {
        // The tip-cover test (h5-box pixels review F1): a chevron fully
        // behind a visible tip stays unpainted; a merely touching one still
        // paints (its visible strokes stay).
        let tip = mv_sys::clay::BoundingBox {
            x: 27.0,
            y: 488.0,
            width: 357.0,
            height: 31.0,
        };
        let inside = mv_sys::clay::BoundingBox {
            x: 288.0,
            y: 490.0,
            width: 6.0,
            height: 12.0,
        };
        assert!(box_contains(tip, inside));
        let outside = mv_sys::clay::BoundingBox {
            x: 10.0,
            y: 490.0,
            width: 6.0,
            height: 12.0,
        };
        assert!(!box_contains(tip, outside));
        let straddling = mv_sys::clay::BoundingBox {
            x: 25.0,
            y: 490.0,
            width: 6.0,
            height: 12.0,
        };
        assert!(!box_contains(tip, straddling));
    }
}
