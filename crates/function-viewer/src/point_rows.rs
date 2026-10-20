//! The P/Q point readout rows: heads, roles, moved badges, readouts with the
//! typed-x field, and the triangle mark.
//!
//! P's row carries the track and the editable x; Q's row fades between live
//! and placeholder.

use enum_map::EnumMap;
use mv_sys::clay;

use crate::app::App;
use crate::input::{Cursor, Hit, HitKind};

/// Each point's ink token.
pub static POINT_INKS: EnumMap<mv_viewer::Point, mv_theme::Token> =
    EnumMap::from_array([mv_theme::Token::ChartPoint1, mv_theme::Token::ChartPoint2]);

/// The mark size and the head height.
pub const POINT_MARK_SIZE: f32 = 18.0;
pub const POINT_HEAD_HEIGHT: f32 = 30.0;

/// The readout math and number sizes and lines.
pub const READOUT_MATH_SIZE: f32 = 20.0;
pub const READOUT_MATH_LINE: f32 = 25.0;
pub const READOUT_NUMBER_SIZE: f32 = 18.0;
pub const READOUT_NUMBER_LINE: f32 = 24.0;

/// P's tip hangs below the head.
pub const P_TIP_TOP: f32 = POINT_HEAD_HEIGHT + 4.0;

/// The off-view mark's size.
pub const OFF_MARK: [f32; 2] = [10.0, 9.0];

/// The moved badge's word, height, padding, gap, margin and radius.
pub const MOVED_LABEL: &str = "moved";
pub const MOVED_HEIGHT: f32 = 24.0;
pub const MOVED_PADDING: [u16; 2] = [8, 2];
pub const MOVED_GAP: f32 = 6.0;
pub const MOVED_MARGIN: f32 = 4.0;
pub const MOVED_RADIUS: f32 = 6.0;

/// Declares the Points card with the P and Q rows.
pub fn points_card(app: &mut App, t: &mv_theme::Theme) {
    let id = mv_clay::id("Card", mv_viewer::Section::Points as u32);
    crate::ui::ui_scope(app, id, &crate::sidebar::sub_card(t), |app| {
        crate::sidebar::card_head(app, mv_viewer::Section::Points);
        let slots = mv_viewer::readout_slots(app.viewer.view);
        let mut points = crate::top_bar::layout_config(clay::Sizing {
            width: mv_clay::sizing_grow(crate::top_bar::ZERO_MINMAX),
            height: mv_clay::sizing_fit(crate::top_bar::ZERO_MINMAX),
        });
        points.childGap = 16;
        points.layoutDirection = clay::LayoutDirection::TopToBottom;
        crate::ui::ui_scope(
            app,
            mv_clay::id("Points", 0),
            &crate::top_bar::decl(points),
            |app| {
                p_row(app, t, slots);
                q_row(app, t, slots);
            },
        );
    });
}

/// Declares P's head, track and editing tip.
fn p_row(app: &mut App, t: &mv_theme::Theme, slots: Option<mv_viewer::ReadoutSlots>) {
    let p_x = app.viewer.p_x;
    let off = mv_viewer::p_off_view(&app.viewer);
    let curve = mv_viewer::curve_at(&app.viewer, p_x);
    let moved = mv_viewer::p_moved(&app.viewer);
    let edit = mv_viewer::part_ui(&app.viewer, mv_viewer::Part::P).edit;
    let id = mv_clay::id("Point", mv_viewer::Point::P as u32);
    crate::input::region(
        app,
        id,
        Hit {
            kind: HitKind::Point,
            index: mv_viewer::Point::P as i64,
        },
        Cursor::Default,
        Cursor::Default,
        false,
        clay::CornerRadius {
            topLeft: 0.0,
            topRight: 0.0,
            bottomLeft: 0.0,
            bottomRight: 0.0,
        },
    );
    let mut row = crate::top_bar::layout_config(clay::Sizing {
        width: mv_clay::sizing_grow(crate::top_bar::ZERO_MINMAX),
        height: mv_clay::sizing_fit(crate::top_bar::ZERO_MINMAX),
    });
    row.childGap = 6;
    row.layoutDirection = clay::LayoutDirection::TopToBottom;
    crate::ui::ui_scope(app, id, &crate::top_bar::decl(row), |app| {
        let field = app.boxes.field.clone();
        let readout = p_readout(app, t, p_x, curve, slots, &field);
        let head = PointHead {
            point: mv_viewer::Point::P,
            role: mv_viewer::OFF_VIEW_ROLES[off].to_string(),
            role_ink: if off == mv_viewer::OffView::InView {
                mv_theme::Token::ForegroundMuted
            } else {
                mv_theme::Token::Foreground
            },
            off,
            moved,
            fade: None,
        };
        point_head(app, t, &head, &readout);
        crate::track::p_track(app);
        if let Some(edit) = edit {
            let phrases = if edit.error.is_empty() {
                mv_viewer::HINT_EDIT.to_vec()
            } else {
                mv_viewer::hint_refused(&edit.error, &mv_numfmt::format_stored(edit.base, false))
            };
            let mut row_width = crate::sidebar::ROW_WIDTH;
            let (region, found) = crate::input::region_of(
                app,
                Hit {
                    kind: HitKind::Point,
                    index: mv_viewer::Point::P as i64,
                },
            );
            if found {
                row_width = region.r#box.width;
            }
            crate::tip::hang_tip(
                app,
                t,
                mv_clay::id("PointTip", 0),
                &phrases,
                !edit.error.is_empty(),
                row_width,
                clay::FloatingAttachPoints {
                    element: clay::FloatingAttachPointType::RightTop,
                    parent: clay::FloatingAttachPointType::RightTop,
                },
                [0.0, P_TIP_TOP],
            );
        }
    });
}

/// Declares Q's head, live or placeholder with its fade.
fn q_row(app: &mut App, t: &mv_theme::Theme, slots: Option<mv_viewer::ReadoutSlots>) {
    let id = mv_clay::id("Point", mv_viewer::Point::Q as u32);
    let mut row = crate::top_bar::layout_config(clay::Sizing {
        width: mv_clay::sizing_grow(crate::top_bar::ZERO_MINMAX),
        height: mv_clay::sizing_fit(crate::top_bar::ZERO_MINMAX),
    });
    row.layoutDirection = clay::LayoutDirection::TopToBottom;
    crate::ui::ui_scope(app, id, &crate::top_bar::decl(row), |app| {
        let q_x = app.viewer.q_x;
        let live = q_x.is_some();
        let readout = match q_x {
            Some(x) => {
                let curve = mv_viewer::curve_at(&app.viewer, x);
                Readout {
                    line: point_readout(app, t, x, curve, slots, None).line,
                    input: [0.0; 4],
                    field: None,
                }
            }
            None => Readout {
                line: point_readout_empty(app, t).line,
                input: [0.0; 4],
                field: None,
            },
        };
        let role = if live {
            mv_viewer::Q_ROLE_LIVE.to_string()
        } else {
            mv_viewer::Q_ROLE_EMPTY.to_string()
        };
        let fade = crate::motion_driver::q_row_fade(app, live);
        point_head(
            app,
            t,
            &PointHead {
                point: mv_viewer::Point::Q,
                role,
                role_ink: mv_theme::Token::ForegroundMuted,
                off: mv_viewer::OffView::InView,
                moved: None,
                fade: Some(fade),
            },
            &readout,
        );
    });
}

/// A point's head: whose, its role, off-view state, moved value and fade.
#[derive(Clone, Debug)]
pub struct PointHead {
    pub point: mv_viewer::Point,
    pub role: String,
    pub role_ink: mv_theme::Token,
    pub off: mv_viewer::OffView,
    pub moved: Option<f64>,
    pub fade: Option<f32>,
}

/// Declares the mark/letter/off-mark/role/moved/readout row with the P field
/// overlay.
pub fn point_head(app: &mut App, t: &mv_theme::Theme, head: &PointHead, readout: &Readout) {
    let point = head.point;
    let mut row = crate::top_bar::layout_config(clay::Sizing {
        width: mv_clay::sizing_grow(crate::top_bar::ZERO_MINMAX),
        height: mv_clay::sizing_fixed(POINT_HEAD_HEIGHT),
    });
    row.childGap = 8;
    row.childAlignment = clay::ChildAlignment {
        x: clay::LayoutAlignmentX::Left,
        y: clay::LayoutAlignmentY::Center,
    };
    crate::ui::ui_scope(
        app,
        mv_clay::id("PointHead", point as u32),
        &crate::top_bar::decl(row),
        |app| {
            let mark = crate::widgets::PointMark {
                kind: if point == mv_viewer::Point::P {
                    crate::widgets::PointMarkKind::Bullseye
                } else {
                    crate::widgets::PointMarkKind::Ring
                },
                size: POINT_MARK_SIZE,
                ink: t.colors[POINT_INKS[point]],
                parked: false,
            };
            let custom = crate::widgets::Custom::PointMark(mark);
            let mut mark_decl = crate::top_bar::decl(crate::top_bar::layout_config(clay::Sizing {
                width: mv_clay::sizing_fixed(POINT_MARK_SIZE),
                height: mv_clay::sizing_fixed(POINT_MARK_SIZE),
            }));
            mark_decl.custom = crate::widgets::custom_data(custom);
            crate::ui::ui_scope(app, mv_clay::id("Mark", point as u32), &mark_decl, |_| {});
            crate::typography::label(
                app,
                crate::typography::Type::PointLetter,
                mv_viewer::POINT_NAMES[point],
                mv_theme::Token::Foreground,
            );
            if head.off != mv_viewer::OffView::InView {
                let custom = crate::widgets::Custom::Triangle(Triangle {
                    up: head.off == mv_viewer::OffView::Above,
                    ink: t.colors[mv_theme::Token::Warning],
                });
                let mut off_decl =
                    crate::top_bar::decl(crate::top_bar::layout_config(clay::Sizing {
                        width: mv_clay::sizing_fixed(OFF_MARK[0]),
                        height: mv_clay::sizing_fixed(OFF_MARK[1]),
                    }));
                off_decl.custom = crate::widgets::custom_data(custom);
                crate::ui::ui_scope(app, mv_clay::id("OffMark", 0), &off_decl, |_| {});
            }
            let mut fade = crate::effects::LAYER_REST;
            fade.alpha = head.fade.unwrap_or(1.0);
            let (fade_overlay, fade_data) = crate::effects::layer_config(
                fade,
                crate::effects::Effect {
                    shadow: None,
                    inset: 0.0,
                    inset_ink: [0.0; 4],
                    layer: None,
                },
            );
            let mut role_slot = crate::top_bar::layout_config(clay::Sizing {
                width: mv_clay::sizing_grow(crate::top_bar::ZERO_MINMAX),
                height: mv_clay::sizing_fixed(POINT_HEAD_HEIGHT),
            });
            role_slot.childAlignment = clay::ChildAlignment {
                x: clay::LayoutAlignmentX::Left,
                y: clay::LayoutAlignmentY::Center,
            };
            let mut role_decl = crate::top_bar::decl(role_slot);
            role_decl.overlayColor = fade_overlay;
            role_decl.userData = fade_data;
            crate::ui::ui_scope(
                app,
                mv_clay::id("RoleSlot", point as u32),
                &role_decl,
                |app| {
                    crate::typography::text_line(
                        app,
                        mv_clay::id("Role", point as u32),
                        crate::typography::Type::Role,
                        &head.role,
                        head.role_ink,
                        crate::typography::Overflow::Whole,
                        Some(mv_clay::sizing_grow(crate::top_bar::ZERO_MINMAX)),
                    );
                },
            );
            if let Some(dy) = head.moved {
                moved_badge(app, t, dy);
            }
            let readout_id = mv_clay::id("Readout", point as u32);
            let mut readout_decl =
                crate::top_bar::decl(crate::top_bar::layout_config(clay::Sizing {
                    width: mv_clay::sizing_fixed(readout.line.width),
                    height: mv_clay::sizing_fixed(readout.line.height),
                }));
            readout_decl.custom =
                crate::widgets::custom_data(crate::widgets::Custom::Readout(readout.clone()));
            readout_decl.overlayColor = fade_overlay;
            readout_decl.userData = fade_data;
            crate::ui::ui_scope(app, readout_id, &readout_decl, |app| {
                if point == mv_viewer::Point::P {
                    let input = readout.input;
                    let field_id = mv_clay::id("PointField", 0);
                    let mut field_decl =
                        crate::top_bar::decl(crate::top_bar::layout_config(clay::Sizing {
                            width: mv_clay::sizing_fixed(input[2]),
                            height: mv_clay::sizing_fixed(input[3]),
                        }));
                    field_decl.floating = crate::top_bar::floating(
                        [input[0], input[1]],
                        clay::FloatingAttachPointType::LeftTop,
                        clay::FloatingAttachPointType::LeftTop,
                        0,
                    );
                    crate::ui::ui_scope(app, field_id, &field_decl, |_| {});
                    crate::input::region(
                        app,
                        field_id,
                        Hit {
                            kind: HitKind::PField,
                            index: 0,
                        },
                        Cursor::Text,
                        Cursor::Text,
                        false,
                        clay::CornerRadius {
                            topLeft: 0.0,
                            topRight: 0.0,
                            bottomLeft: 0.0,
                            bottomRight: 0.0,
                        },
                    );
                }
            });
        },
    );
}

/// Declares the moved badge with its signed value.
fn moved_badge(app: &mut App, t: &mv_theme::Theme, dy: f64) {
    let sign = if mv_numfmt::round_to(dy, mv_numfmt::DISPLAY_DP) > 0.0 {
        "+"
    } else {
        ""
    };
    let value = format!(
        "{sign}{}",
        mv_numfmt::format_number(dy, mv_numfmt::DISPLAY_DP)
    );
    let mut slot = crate::top_bar::layout_config(clay::Sizing {
        width: mv_clay::sizing_fit(crate::top_bar::ZERO_MINMAX),
        height: mv_clay::sizing_fit(crate::top_bar::ZERO_MINMAX),
    });
    slot.padding = clay::Padding {
        left: 0,
        right: MOVED_MARGIN as u16,
        top: 0,
        bottom: 0,
    };
    crate::ui::ui_scope(
        app,
        mv_clay::id("MovedSlot", 0),
        &crate::top_bar::decl(slot),
        |app| {
            let mut layout = crate::top_bar::layout_config(clay::Sizing {
                width: mv_clay::sizing_fit(crate::top_bar::ZERO_MINMAX),
                height: mv_clay::sizing_fixed(MOVED_HEIGHT),
            });
            layout.padding = clay::Padding {
                left: (MOVED_PADDING[0] as f32 + 1.0) as u16,
                right: (MOVED_PADDING[0] as f32 + 1.0) as u16,
                top: (MOVED_PADDING[1] as f32 + 1.0) as u16,
                bottom: (MOVED_PADDING[1] as f32 + 1.0) as u16,
            };
            layout.childGap = MOVED_GAP as u16;
            layout.childAlignment = clay::ChildAlignment {
                x: clay::LayoutAlignmentX::Left,
                y: clay::LayoutAlignmentY::Center,
            };
            let mut moved_decl = crate::top_bar::decl(layout);
            let plate = mv_theme::plate(t, mv_theme::Token::Primary);
            moved_decl.backgroundColor = [
                plate[0] * 255.0,
                plate[1] * 255.0,
                plate[2] * 255.0,
                plate[3] * 255.0,
            ];
            moved_decl.border = clay::BorderElementConfig {
                color: crate::ui::ink(t, mv_theme::Token::Primary),
                width: mv_clay::border_outside(1),
            };
            moved_decl.cornerRadius = mv_clay::corner_radius_all(MOVED_RADIUS);
            crate::ui::ui_scope(app, mv_clay::id("Moved", 0), &moved_decl, |app| {
                crate::typography::label(
                    app,
                    crate::typography::Type::MovedLabel,
                    MOVED_LABEL,
                    mv_theme::Token::Foreground,
                );
                crate::typography::label(
                    app,
                    crate::typography::Type::MovedValue,
                    &value,
                    mv_theme::Token::Foreground,
                );
            });
        },
    );
}

/// A readout: its line, its input box, and its field when open.
#[derive(Clone, Debug)]
pub struct Readout {
    pub line: mv_text::Line,
    pub input: [f32; 4],
    pub field: Option<ReadoutField>,
}

/// The open field's look: its shown text, style, pen and colours.
#[derive(Clone, Debug)]
pub struct ReadoutField {
    pub shown: String,
    pub style: mv_text::Style,
    pub pen: f32,
    pub look: super::number_field::FieldLook,
    pub fill: mv_theme::Color,
    pub stroke: mv_theme::Color,
    pub select: mv_theme::Color,
    pub caret: mv_theme::Color,
}

/// The P field's radius and stroke.
pub const P_FIELD_RADIUS: f32 = 6.0;
pub const P_FIELD_STROKE: f32 = 2.0;

/// Builds P's readout with the open-field input box and look.
fn p_readout(
    app: &mut App,
    t: &mv_theme::Theme,
    x: f64,
    y: f64,
    slots: Option<mv_viewer::ReadoutSlots>,
    field: &crate::number_field::Field,
) -> Readout {
    let open = field.open && field.part == mv_viewer::Part::P;
    let mut typed = None;
    if open {
        typed = Some(TypedX {
            text: std::str::from_utf8(&field.text[..field.len as usize])
                .unwrap_or("")
                .to_string(),
            ink: t.colors[if field.error.is_empty() {
                mv_theme::Token::Foreground
            } else {
                mv_theme::Token::Destructive
            }],
        });
    }
    let mut out = Readout {
        line: point_readout(app, t, x, y, slots, typed).line,
        input: [0.0; 4],
        field: None,
    };
    let x_piece = out.line.placed[2].clone();
    let style = crate::number_field::field_text_style(t, mv_viewer::Part::P);
    let text_w = mv_text::layout_width(x_piece.shaped.advance);
    let input_w = text_w
        + if open {
            2.0 * crate::number_field::P_FIELD_PAD
        } else {
            0.0
        };
    out.input = [
        x_piece.x + x_piece.width - input_w,
        x_piece.top,
        input_w,
        crate::number_field::P_FIELD_LINE,
    ];
    if open {
        out.field = Some(ReadoutField {
            shown: std::str::from_utf8(&field.text[..field.len as usize])
                .unwrap_or("")
                .to_string(),
            style,
            pen: x_piece.pen,
            look: crate::number_field::field_look(field),
            fill: t.colors[mv_theme::Token::Card],
            stroke: t.colors[if field.error.is_empty() {
                mv_theme::Token::Primary
            } else {
                mv_theme::Token::Destructive
            }],
            select: mv_theme::plate(t, mv_theme::Token::Primary),
            caret: t.colors[mv_theme::Token::Primary],
        });
    }
    out
}

/// The text typed into P's field, in its ink.
#[derive(Clone, Debug)]
pub struct TypedX {
    pub text: String,
    pub ink: mv_theme::Color,
}

/// Formats a point readout with its slots and an optional typed x.
pub fn point_readout(
    app: &mut App,
    t: &mv_theme::Theme,
    x: f64,
    y: f64,
    slots: Option<mv_viewer::ReadoutSlots>,
    typed: Option<TypedX>,
) -> super::widgets::Typeset {
    let x_text = mv_numfmt::format_number(x, mv_numfmt::DISPLAY_DP);
    let y_text = match mv_numfmt::number_parts(y) {
        mv_numfmt::Parts::Fixed(f) => f.text,
        _ => mv_numfmt::NO_NUMBER.to_string(),
    };
    let (s, laid_out) = match slots {
        Some(s) => (s, true),
        None => (mv_viewer::ReadoutSlots { x: 0, y: 0 }, false),
    };
    let x_chars = crate::widgets::rune_count(&x_text).max(if laid_out { s.x } else { 0 });
    let y_chars = crate::widgets::rune_count(&y_text).max(if laid_out { s.y } else { 0 });
    readout_line(
        app,
        t,
        &x_text,
        &mv_numfmt::relation(y),
        &y_text,
        x_chars,
        y_chars,
        mv_theme::Token::Foreground,
        typed,
    )
}

/// The muted dashed readout for no point.
pub fn point_readout_empty(app: &mut App, t: &mv_theme::Theme) -> super::widgets::Typeset {
    let dash = mv_numfmt::NO_NUMBER;
    let chars = crate::widgets::rune_count(dash);
    readout_line(
        app,
        t,
        dash,
        "=",
        dash,
        chars,
        chars,
        mv_theme::Token::ForegroundMuted,
        None,
    )
}

/// Typesets the `f(x) relation y` pieces with their slots.
#[allow(clippy::too_many_arguments)]
fn readout_line(
    app: &mut App,
    t: &mv_theme::Theme,
    x_text: &str,
    relation: &str,
    y_text: &str,
    x_chars: i64,
    y_chars: i64,
    ink_token: mv_theme::Token,
    typed: Option<TypedX>,
) -> super::widgets::Typeset {
    let color = t.colors[ink_token];
    let math_style = mv_text::Style {
        face: mv_text::Face::Stix400,
        size: READOUT_MATH_SIZE,
        tracking: 0.0,
        features: enumset::EnumSet::empty(),
    };
    let number = mv_text::Style {
        face: readout_face(t, 500),
        size: READOUT_NUMBER_SIZE,
        tracking: 0.0,
        features: mv_text::Features::only(mv_text::Feature::Tabular),
    };
    let ch = mv_text::ch(&mut app.fonts, number);
    let open_box = mv_text::round_up(0.634 * READOUT_MATH_SIZE);
    let (x_shown, x_ink, x_pad) = match typed {
        Some(typing) => (
            typing.text,
            typing.ink,
            [
                crate::number_field::P_FIELD_PAD,
                crate::number_field::P_FIELD_PAD,
            ],
        ),
        None => (x_text.to_string(), color, [0.0, 0.0]),
    };
    let x_box =
        mv_text::round_up(mv_text::measure(&mut app.fonts, number, &x_shown)) + x_pad[0] + x_pad[1];
    let lead = open_box + mv_text::round_up(x_chars as f32 * ch);
    let mut f_style = math_style;
    f_style.face = mv_text::Face::Stix400Italic;
    let paren_style = math_style;
    let f = mv_text::measure(&mut app.fonts, f_style, "f");
    let paren = mv_text::measure(&mut app.fonts, paren_style, "(");
    let pieces = [
        mv_text::Piece {
            style: f_style,
            text: "f".to_string(),
            ink: color,
            line_height: READOUT_MATH_LINE,
            r#box: mv_text::Box::Exact,
            min_width: 0.0,
            align: mv_text::Align::Left,
            pad: [0.0f32.max(lead - open_box - x_box), 0.0],
        },
        mv_text::Piece {
            style: paren_style,
            text: "(".to_string(),
            ink: color,
            line_height: READOUT_MATH_LINE,
            r#box: mv_text::Box::Exact,
            min_width: 0.0,
            align: mv_text::Align::Left,
            pad: [0.0, 0.0f32.max(open_box - f - paren)],
        },
        mv_text::Piece {
            style: number,
            text: x_shown,
            ink: x_ink,
            line_height: READOUT_NUMBER_LINE,
            r#box: mv_text::Box::WholePx,
            min_width: 0.0,
            align: mv_text::Align::Right,
            pad: x_pad,
        },
        mv_text::Piece {
            style: paren_style,
            text: ")".to_string(),
            ink: color,
            line_height: READOUT_MATH_LINE,
            r#box: mv_text::Box::Exact,
            min_width: mv_text::round_up(0.343 * READOUT_MATH_SIZE),
            align: mv_text::Align::Left,
            pad: [0.0, 0.0],
        },
        mv_text::Piece {
            style: paren_style,
            text: relation.to_string(),
            ink: color,
            line_height: READOUT_MATH_LINE,
            r#box: mv_text::Box::Exact,
            min_width: mv_text::round_up(0.72 * READOUT_MATH_SIZE),
            align: mv_text::Align::Left,
            pad: [7.0, 7.0],
        },
        mv_text::Piece {
            style: number,
            text: y_text.to_string(),
            ink: color,
            line_height: READOUT_NUMBER_LINE,
            r#box: mv_text::Box::Exact,
            min_width: mv_text::round_up(y_chars as f32 * ch),
            align: mv_text::Align::Right,
            pad: [0.0, 0.0],
        },
    ];
    super::widgets::Typeset {
        line: mv_text::typeset(&mut app.fonts, &pieces, 0.0, mv_text::Vertical::Center, 1.0),
        selectable: false,
    }
}

/// The theme's readout face at a weight.
pub fn readout_face(t: &mv_theme::Theme, weight: i64) -> mv_text::Face {
    if t.readout_font == mv_theme::ReadoutFont::StixTwoText {
        mv_text::Face::Stix500
    } else if weight >= 600 {
        mv_text::Face::RobotoMono600
    } else {
        mv_text::Face::RobotoMono500
    }
}

/// Draws the P field, the readout line, and the selection/caret.
pub fn draw_readout(app: &mut App, bounds: clay::BoundingBox, r: &Readout) {
    if let Some(field) = &r.field {
        let top = bounds.y + r.input[1];
        let b = clay::BoundingBox {
            x: bounds.x + r.input[0],
            y: top,
            width: r.input[2],
            height: r.input[3],
        };
        let radius = mv_clay::corner_radius_all(P_FIELD_RADIUS);
        let vg = &mut app.vg;
        vg.begin_path();
        crate::draw::rounded_rect(vg, b, radius);
        vg.fill_color(field.fill);
        vg.fill();
        crate::effects::draw_inset(vg, b, radius, P_FIELD_STROKE, field.stroke);
        crate::number_field::draw_field_selection(
            app,
            &field.style,
            &field.shown,
            bounds.x + field.pen,
            top,
            crate::number_field::P_FIELD_LINE,
            field.look,
            field.select,
        );
    }
    mv_text::draw_line(&mut app.fonts, &mut app.vg, &r.line, bounds.x, bounds.y);
    crate::widgets::note_line(app, &r.line, bounds.x, bounds.y);
    if let Some(field) = &r.field {
        crate::number_field::draw_field_caret(
            app,
            &field.style,
            &field.shown,
            bounds.x + field.pen,
            bounds.y + r.input[1],
            crate::number_field::P_FIELD_LINE,
            field.look,
            field.caret,
        );
    }
}

/// Opens the P field, or places the caret and starts the selection.
pub fn p_field_press(app: &mut App) {
    if !crate::number_field::p_field_open(app) {
        crate::number_field::field_open(app, mv_viewer::Part::P);
        return;
    }
    let (region, found) = crate::input::region_of(
        app,
        Hit {
            kind: HitKind::PField,
            index: 0,
        },
    );
    if found {
        crate::number_field::field_point(app, region.r#box, app.input.pointer[0], false);
        app.boxes.field.selecting = true;
    }
}

/// Extends the P field selection while dragging.
pub fn p_field_drag(app: &mut App) {
    if !app.boxes.field.selecting || !crate::number_field::p_field_open(app) {
        return;
    }
    let (region, found) = crate::input::region_of(
        app,
        Hit {
            kind: HitKind::PField,
            index: 0,
        },
    );
    if found {
        crate::number_field::field_point(app, region.r#box, app.input.pointer[0], true);
    }
}

/// A small triangle filling its box, pointing up or down, in its ink: P's
/// off-view mark.
#[derive(Clone, Copy, Debug)]
pub struct Triangle {
    pub up: bool,
    pub ink: mv_theme::Color,
}

/// Fills the up/down triangle in the box.
pub fn draw_triangle(vg: &mut mv_nvg::Context, bounds: clay::BoundingBox, tri: Triangle) {
    vg.begin_path();
    if tri.up {
        vg.move_to(bounds.x + bounds.width / 2.0, bounds.y);
        vg.line_to(bounds.x + bounds.width, bounds.y + bounds.height);
        vg.line_to(bounds.x, bounds.y + bounds.height);
    } else {
        vg.move_to(bounds.x, bounds.y);
        vg.line_to(bounds.x + bounds.width, bounds.y);
        vg.line_to(bounds.x + bounds.width / 2.0, bounds.y + bounds.height);
    }
    vg.close_path();
    vg.fill_color(tri.ink);
    vg.fill();
}
