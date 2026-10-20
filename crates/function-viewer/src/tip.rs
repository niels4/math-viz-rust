//! Tips: row tips, hanging tips, the foot tip, and the key caps.
//!
//! Phrases become words/math/key pieces laid out into greedy flex lines; the
//! row tip hangs under its parameter row, others float at their attachment,
//! and the foot tip pins the plane keys to the panel foot.

use mv_sys::clay;

use crate::app::App;

/// A declaration with the page's zero defaults: only the layout is set, every
/// other field the Clay C default a zeroed declaration carries.
pub(crate) fn blank_declaration(layout: clay::LayoutConfig) -> clay::ElementDeclaration {
    clay::ElementDeclaration {
        layout,
        backgroundColor: [0.0; 4],
        overlayColor: [0.0; 4],
        cornerRadius: clay::CornerRadius {
            topLeft: 0.0,
            topRight: 0.0,
            bottomLeft: 0.0,
            bottomRight: 0.0,
        },
        aspectRatio: clay::AspectRatioElementConfig { aspectRatio: 0.0 },
        image: clay::ImageElementConfig {
            imageData: std::ptr::null_mut(),
        },
        floating: clay::FloatingElementConfig {
            offset: [0.0; 2],
            expand: clay::Dimensions {
                width: 0.0,
                height: 0.0,
            },
            parentId: 0,
            zIndex: 0,
            attachment: clay::FloatingAttachPoints {
                element: clay::FloatingAttachPointType::LeftTop,
                parent: clay::FloatingAttachPointType::LeftTop,
            },
            pointerCaptureMode: clay::PointerCaptureMode::Capture,
            attachTo: clay::FloatingAttachToElement::None,
            clipTo: clay::FloatingClipToElement::None,
        },
        custom: clay::CustomElementConfig {
            customData: std::ptr::null_mut(),
        },
        clip: clay::ClipElementConfig {
            horizontal: false,
            vertical: false,
            childOffset: [0.0; 2],
        },
        border: clay::BorderElementConfig {
            color: [0.0; 4],
            width: clay::BorderWidth {
                left: 0,
                right: 0,
                top: 0,
                bottom: 0,
                betweenChildren: 0,
            },
        },
        transition: clay::TransitionElementConfig {
            handler: None,
            duration: 0.0,
            properties: 0,
            interactionHandling:
                clay::TransitionInteractionHandlingType::DisableInteractionsWhileTransitioningPosition,
            enter: clay::TransitionEnterConfig {
                setInitialState: None,
                trigger: clay::TransitionEnterTriggerType::SkipOnFirstParentFrame,
            },
            exit: clay::TransitionExitConfig {
                setFinalState: None,
                trigger: clay::TransitionExitTriggerType::SkipWhenParentExits,
                siblingOrdering: clay::ExitTransitionSiblingOrdering::UnderneathSiblings,
            },
        },
        userData: std::ptr::null_mut(),
    }
}

/// A fixed sizing axis of `size` px, as the C `SizingFixed` reads it.
pub(crate) fn fixed(size: f32) -> clay::SizingAxis {
    clay::SizingAxis {
        r#type: clay::SizingType::Fixed,
        constraints: clay::SizingConstraints {
            sizeMinMax: clay::SizingConstraintsMinMax {
                min: size,
                max: size,
            },
        },
    }
}

/// A grow sizing axis, as the C `SizingGrow` reads it.
pub(crate) fn grow() -> clay::SizingAxis {
    clay::SizingAxis {
        r#type: clay::SizingType::Grow,
        constraints: clay::SizingConstraints {
            sizeMinMax: clay::SizingConstraintsMinMax { min: 0.0, max: 0.0 },
        },
    }
}

/// A fit sizing axis, as the C `SizingFit` reads it.
pub(crate) fn fit() -> clay::SizingAxis {
    clay::SizingAxis {
        r#type: clay::SizingType::Fit,
        constraints: clay::SizingConstraints {
            sizeMinMax: clay::SizingConstraintsMinMax { min: 0.0, max: 0.0 },
        },
    }
}

/// The tip gap, padding, radius and math size/line.
pub const TIP_GAP: f32 = 4.0;
pub const TIP_PADDING: [f32; 2] = [10.0, 7.0];
pub const TIP_RADIUS: f32 = 8.0;
pub const TIP_MATH_SIZE: f32 = 15.0;
pub const TIP_MATH_LINE: f32 = 18.0;

/// The key caps' height, padding, style, line and arrow size.
pub const KEY_HEIGHT: f32 = 21.0;
pub const KEY_PADDING: f32 = 6.0;
pub static KEY_STYLE: std::sync::LazyLock<mv_text::Style> =
    std::sync::LazyLock::new(|| mv_text::Style {
        face: mv_text::Face::RobotoMono600,
        size: 12.0,
        tracking: 0.0,
        features: enumset::EnumSet::empty(),
    });
pub const KEY_LINE: f32 = 16.0;
pub const ARROW_SIZE: [f32; 2] = [18.0, 14.0];

/// Each arrow key's SVG path.
pub static ARROW_PATHS: [(&str, &str); 4] = [
    ("←", "M9 2L3 7L9 12M3 7H15"),
    ("→", "M9 2L15 7L9 12M15 7H3"),
    ("↑", "M4 8L9 2L14 8M9 2V14"),
    ("↓", "M4 8L9 14L14 8M9 14V2"),
];

/// Looks up an arrow key's SVG path.
pub fn arrow_path(key: &str) -> (&'static str, bool) {
    for (name, path) in ARROW_PATHS {
        if name == key {
            return (path, true);
        }
    }
    ("", false)
}

/// A tip piece's kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TipPieceKind {
    Words,
    Math,
    Key,
}

/// One measured piece.
#[derive(Clone, Debug)]
pub struct TipPiece {
    pub kind: TipPieceKind,
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// A laid-out tip: its pieces, size, ink and selectability.
#[derive(Clone, Debug)]
pub struct TipItems {
    pub pieces: Vec<TipPiece>,
    pub width: f32,
    pub height: f32,
    pub ink: mv_theme::Color,
    pub selectable: bool,
}

/// One wrap item's working state.
struct Item {
    pieces: Vec<TipPiece>,
    width: f32,
    height: f32,
}

/// Joins continuations with a non-breaking space.
fn keep_separators(s: &str) -> String {
    s.replace(" ·", "\u{a0}·")
}

/// Measures one words/math/key piece.
fn piece(app: &mut App, kind: TipPieceKind, s: &str) -> TipPiece {
    let mut p = TipPiece {
        kind,
        text: s.to_string(),
        x: 0.0,
        y: 0.0,
        width: 0.0,
        height: 0.0,
    };
    match kind {
        TipPieceKind::Words => {
            let style = crate::typography::type_style(
                crate::typography::Type::Tip,
                crate::ui::current_theme(app),
            );
            p.width = mv_text::round_up(mv_text::measure(&mut app.fonts, style.style, s));
            p.height = style.line_height;
        }
        TipPieceKind::Math => {
            p.width = mv_text::round_up(
                crate::widgets::math_line(
                    &mut app.fonts,
                    s,
                    TIP_MATH_SIZE,
                    TIP_MATH_LINE,
                    [0.0; 4],
                )
                .width,
            );
            p.height = TIP_MATH_LINE;
        }
        TipPieceKind::Key => {
            let (_, is_arrow) = arrow_path(s);
            let content = if is_arrow {
                ARROW_SIZE[0]
            } else {
                mv_text::layout_width(mv_text::measure(&mut app.fonts, *KEY_STYLE, s))
            };
            p.width = content + 2.0 * KEY_PADDING + 2.0;
            p.height = KEY_HEIGHT;
        }
    }
    p
}

/// Converts phrases to wrap items.
fn add_items(app: &mut App, items: &mut Vec<Item>, phrases: &[mv_viewer::Phrase], words: bool) {
    for phrase in phrases {
        match phrase {
            mv_viewer::Phrase::Words(w) => {
                let s = keep_separators(&w.0);
                if !words {
                    let p = piece(app, TipPieceKind::Words, &s);
                    items.push(Item {
                        pieces: vec![p.clone()],
                        width: p.width,
                        height: p.height,
                    });
                    continue;
                }
                for word in s.split(' ') {
                    if word.is_empty() {
                        continue;
                    }
                    let p = piece(app, TipPieceKind::Words, word);
                    items.push(Item {
                        pieces: vec![p.clone()],
                        width: p.width,
                        height: p.height,
                    });
                }
            }
            mv_viewer::Phrase::Math(m) => {
                let p = piece(app, TipPieceKind::Math, &m.0);
                items.push(Item {
                    pieces: vec![p.clone()],
                    width: p.width,
                    height: p.height,
                });
            }
            mv_viewer::Phrase::Key(k) => {
                let p = piece(app, TipPieceKind::Key, &k.0);
                items.push(Item {
                    pieces: vec![p.clone()],
                    width: p.width,
                    height: p.height,
                });
            }
            mv_viewer::Phrase::Together(inner) => {
                // Phrases that wrap as one item, spaced as the tip spaces its
                // items, centred.
                let mut sub = Vec::new();
                add_items(app, &mut sub, &inner.0, false);
                let mut group = Item {
                    pieces: Vec::new(),
                    width: 0.0,
                    height: 0.0,
                };
                for (i, it) in sub.iter().enumerate() {
                    group.height = group.height.max(it.height);
                    if i > 0 {
                        group.width += TIP_GAP;
                    }
                    for p in &it.pieces {
                        let mut placed = p.clone();
                        placed.x += group.width;
                        group.pieces.push(placed);
                    }
                    group.width += it.width;
                }
                items.push(group);
            }
        }
    }
}

/// Lays out items into greedy flex lines.
pub fn tip_items(
    app: &mut App,
    phrases: &[mv_viewer::Phrase],
    max_width: f32,
    ink: mv_theme::Color,
    row_gap: f32,
) -> TipItems {
    let mut items = Vec::new();
    add_items(app, &mut items, phrases, true);
    let mut total = 0.0f32;
    for (i, it) in items.iter().enumerate() {
        total += it.width + if i > 0 { TIP_GAP } else { 0.0 };
    }
    let mut tip = TipItems {
        pieces: Vec::new(),
        width: total.min(max_width),
        height: 0.0,
        ink,
        selectable: false,
    };
    // Lines, greedily: an item that doesn't fit after the one before starts
    // the next.
    let mut start = 0usize;
    let mut y = 0.0f32;
    while start < items.len() {
        let mut end = start + 1;
        let mut used = items[start].width;
        while end < items.len() && used + TIP_GAP + items[end].width <= max_width {
            used += TIP_GAP + items[end].width;
            end += 1;
        }
        let mut line = 0.0f32;
        for it in &items[start..end] {
            line = line.max(it.height);
        }
        let mut x = 0.0f32;
        for it in &items[start..end] {
            for p in &it.pieces {
                let mut placed = p.clone();
                placed.x += x;
                placed.y = y + (line - p.height) / 2.0;
                tip.pieces.push(placed);
            }
            x += it.width + TIP_GAP;
        }
        y += line + row_gap;
        start = end;
    }
    tip.height = (y - row_gap).max(0.0);
    tip
}

/// Hangs the tip under a parameter row.
pub fn row_tip(
    app: &mut App,
    t: &mv_theme::Theme,
    p: mv_viewer::Param,
    phrases: &[mv_viewer::Phrase],
    error: bool,
    row_width: f32,
) {
    hang_tip(
        app,
        t,
        mv_clay::id("Tip", p as u32),
        phrases,
        error,
        row_width - 12.0,
        clay::FloatingAttachPoints {
            element: clay::FloatingAttachPointType::RightTop,
            parent: clay::FloatingAttachPointType::RightBottom,
        },
        [-6.0, 4.0],
    );
}

/// Declares a floating tip card at an attachment and offset.
#[allow(clippy::too_many_arguments)]
pub fn hang_tip(
    app: &mut App,
    t: &mv_theme::Theme,
    id: clay::ElementId,
    phrases: &[mv_viewer::Phrase],
    error: bool,
    max_width: f32,
    attachment: clay::FloatingAttachPoints,
    offset: [f32; 2],
) {
    let words = t.colors[if error {
        mv_theme::Token::Destructive
    } else {
        mv_theme::Token::Foreground
    }];
    // Inside the border, which Clay draws without laying it out.
    let padding = [TIP_PADDING[0] + 1.0, TIP_PADDING[1] + 1.0];
    let items = tip_items(app, phrases, max_width - 2.0 * padding[0], words, TIP_GAP);
    let layout = clay::LayoutConfig {
        sizing: clay::Sizing {
            width: fixed(items.width + 2.0 * padding[0]),
            height: fixed(items.height + 2.0 * padding[1]),
        },
        padding: clay::Padding {
            left: padding[0] as u16,
            right: padding[0] as u16,
            top: padding[1] as u16,
            bottom: padding[1] as u16,
        },
        childGap: 0,
        childAlignment: clay::ChildAlignment {
            x: clay::LayoutAlignmentX::Left,
            y: clay::LayoutAlignmentY::Top,
        },
        layoutDirection: clay::LayoutDirection::LeftToRight,
    };
    let mut config = blank_declaration(layout);
    config.backgroundColor = crate::ui::ink(t, mv_theme::Token::Card);
    config.border = clay::BorderElementConfig {
        color: crate::ui::ink(
            t,
            if error {
                mv_theme::Token::Destructive
            } else {
                mv_theme::Token::Border
            },
        ),
        width: clay::BorderWidth {
            left: 1,
            right: 1,
            top: 1,
            bottom: 1,
            betweenChildren: 0,
        },
    };
    config.cornerRadius = clay::CornerRadius {
        topLeft: TIP_RADIUS,
        topRight: TIP_RADIUS,
        bottomLeft: TIP_RADIUS,
        bottomRight: TIP_RADIUS,
    };
    config.floating = clay::FloatingElementConfig {
        offset,
        expand: clay::Dimensions {
            width: 0.0,
            height: 0.0,
        },
        parentId: 0,
        zIndex: crate::ruler_tape::ROW_POPOVER_Z,
        attachment,
        pointerCaptureMode: clay::PointerCaptureMode::Capture,
        attachTo: clay::FloatingAttachToElement::Parent,
        clipTo: clay::FloatingClipToElement::AttachedParent,
    };
    config.userData = crate::effects::effect(crate::effects::Effect {
        shadow: t.card_shadow,
        inset: 0.0,
        inset_ink: [0.0; 4],
        layer: None,
    });
    // No custom on the card: the pieces draw once, from the inner element's
    // box (tip.odin's `hang_tip` sets `custom` only on `TipItems`).
    let inner = mv_clay::id("TipItems", id.id);
    let inner_layout = clay::LayoutConfig {
        sizing: clay::Sizing {
            width: fixed(items.width),
            height: fixed(items.height),
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
    let inner_payload = crate::widgets::Custom::TipItems(items);
    let mut inner_config = blank_declaration(inner_layout);
    inner_config.custom = crate::widgets::custom_data(inner_payload);
    crate::ui::ui_scope(app, id, &config, |app| {
        crate::ui::ui_scope(app, inner, &inner_config, |_| {});
    });
}

/// Declares the plane-keys tip pinned to the panel foot.
pub fn foot_tip(app: &mut App, t: &mv_theme::Theme, width: f32) {
    // Inside the border, which Clay draws without laying it out.
    let padding = [TIP_PADDING[0] + 1.0, TIP_PADDING[1] + 1.0];
    let items = tip_items(
        app,
        &mv_viewer::HINT_PLANE_KEYS[..],
        width - 2.0 * padding[0],
        t.colors[mv_theme::Token::Foreground],
        TIP_GAP,
    );
    let slot = mv_clay::id("FootTipSlot", 0);
    let slot_layout = clay::LayoutConfig {
        sizing: clay::Sizing {
            width: grow(),
            height: grow(),
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
            y: clay::LayoutAlignmentY::Bottom,
        },
        layoutDirection: clay::LayoutDirection::LeftToRight,
    };
    let card = mv_clay::id("FootTip", 0);
    let card_layout = clay::LayoutConfig {
        sizing: clay::Sizing {
            width: fixed(items.width + 2.0 * padding[0]),
            height: fixed(items.height + 2.0 * padding[1]),
        },
        padding: clay::Padding {
            left: padding[0] as u16,
            right: padding[0] as u16,
            top: padding[1] as u16,
            bottom: padding[1] as u16,
        },
        childGap: 0,
        childAlignment: clay::ChildAlignment {
            x: clay::LayoutAlignmentX::Left,
            y: clay::LayoutAlignmentY::Top,
        },
        layoutDirection: clay::LayoutDirection::LeftToRight,
    };
    let mut card_config = blank_declaration(card_layout);
    card_config.backgroundColor = crate::ui::ink(t, mv_theme::Token::Card);
    card_config.border = clay::BorderElementConfig {
        color: crate::ui::ink(t, mv_theme::Token::Border),
        width: clay::BorderWidth {
            left: 1,
            right: 1,
            top: 1,
            bottom: 1,
            betweenChildren: 0,
        },
    };
    card_config.cornerRadius = clay::CornerRadius {
        topLeft: TIP_RADIUS,
        topRight: TIP_RADIUS,
        bottomLeft: TIP_RADIUS,
        bottomRight: TIP_RADIUS,
    };
    card_config.userData = crate::effects::effect(crate::effects::Effect {
        shadow: t.card_shadow,
        inset: 0.0,
        inset_ink: [0.0; 4],
        layer: None,
    });
    let inner = mv_clay::id("FootTipItems", 0);
    let inner_layout = clay::LayoutConfig {
        sizing: clay::Sizing {
            width: fixed(items.width),
            height: fixed(items.height),
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
    let inner_payload = crate::widgets::Custom::TipItems(items);
    let mut inner_config = blank_declaration(inner_layout);
    inner_config.custom = crate::widgets::custom_data(inner_payload);
    crate::ui::ui_scope(app, slot, &blank_declaration(slot_layout), |app| {
        crate::ui::ui_scope(app, card, &card_config, |app| {
            crate::ui::ui_scope(app, inner, &inner_config, |_| {});
        });
    });
}

/// Draws the words/math/key pieces.
pub fn draw_tip_items(app: &mut App, r#box: clay::BoundingBox, tip: &TipItems) {
    let words =
        crate::typography::type_style(crate::typography::Type::Tip, crate::ui::current_theme(app));
    for p in &tip.pieces {
        let (x, y) = (r#box.x + p.x, r#box.y + p.y);
        if tip.selectable {
            crate::input::note_text(
                app,
                clay::BoundingBox {
                    x,
                    y,
                    width: p.width,
                    height: p.height,
                },
            );
        }
        match p.kind {
            TipPieceKind::Words => {
                let baseline = y + mv_text::baseline(
                    &mut app.fonts,
                    words.style.face,
                    words.style.size,
                    words.line_height,
                );
                mv_text::draw(
                    &mut app.fonts,
                    &mut app.vg,
                    words.style,
                    x,
                    baseline,
                    &p.text,
                    tip.ink,
                );
            }
            TipPieceKind::Math => {
                let line = crate::widgets::math_line(
                    &mut app.fonts,
                    &p.text,
                    TIP_MATH_SIZE,
                    TIP_MATH_LINE,
                    tip.ink,
                );
                mv_text::draw_line(&mut app.fonts, &mut app.vg, &line, x, y);
            }
            TipPieceKind::Key => {
                let theme = crate::ui::current_theme(app).clone();
                draw_key_cap(app, [x, y, p.width, p.height], &p.text, &theme);
            }
        }
    }
}

/// Draws one key cap with its name or arrow.
fn draw_key_cap(app: &mut App, b: [f32; 4], name: &str, t: &mv_theme::Theme) {
    let outer = clay::BoundingBox {
        x: b[0],
        y: b[1],
        width: b[2],
        height: b[3],
    };
    app.vg.begin_path();
    crate::draw::rounded_rect(
        &mut app.vg,
        outer,
        clay::CornerRadius {
            topLeft: 4.0,
            topRight: 4.0,
            bottomLeft: 4.0,
            bottomRight: 4.0,
        },
    );
    app.vg.fill_color(t.colors[mv_theme::Token::Border]);
    app.vg.fill();
    let inner = clay::BoundingBox {
        x: b[0] + 1.0,
        y: b[1] + 1.0,
        width: b[2] - 2.0,
        height: b[3] - 3.0,
    };
    app.vg.begin_path();
    crate::draw::rounded_rect(
        &mut app.vg,
        inner,
        clay::CornerRadius {
            topLeft: 3.0,
            topRight: 3.0,
            bottomLeft: 2.0,
            bottomRight: 2.0,
        },
    );
    app.vg.fill_color(t.colors[mv_theme::Token::Secondary]);
    app.vg.fill();
    let ink = t.colors[mv_theme::Token::Foreground];
    // The content box: inside the edge and the padding (1 6 2), 15 px tall.
    let content_top = b[1] + 2.0;
    let content_height = b[3] - 6.0;
    if let (path, true) = arrow_path(name) {
        app.vg.save();
        app.vg.translate(
            b[0] + 1.0 + KEY_PADDING,
            content_top + (content_height - ARROW_SIZE[1]) / 2.0,
        );
        app.vg.begin_path();
        let _ = crate::svgpath::draw_path(&mut app.vg, path);
        app.vg.stroke_color(ink);
        app.vg.stroke_width(1.6);
        app.vg.line_cap(mv_nvg::LineCapType::ROUND);
        app.vg.line_join(mv_nvg::LineCapType::ROUND);
        app.vg.stroke();
        app.vg.restore();
        return;
    }
    // The name's 16 px line centred in the 15 px content box, as inline-flex
    // centres it.
    let top = content_top + (content_height - KEY_LINE) / 2.0;
    let baseline =
        top + mv_text::baseline(&mut app.fonts, KEY_STYLE.face, KEY_STYLE.size, KEY_LINE);
    mv_text::draw(
        &mut app.fonts,
        &mut app.vg,
        *KEY_STYLE,
        b[0] + 1.0 + KEY_PADDING,
        baseline,
        name,
        ink,
    );
}
