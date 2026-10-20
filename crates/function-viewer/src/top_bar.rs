//! The top bar: the wordmark, the title, the theme chip and the gear.
//!
//! A responsive header row: narrow windows hide the theme chip's text, then
//! the wordmark's text, keeping the gear reachable.

use mv_sys::clay;

use crate::app::App;
use crate::icons::IconKind;

/// The bar's height, padding, gaps, icon and separator metrics.
pub const TOPBAR_HEIGHT: f32 = 64.0;
pub const TOPBAR_PADDING: f32 = 20.0;
pub const TOPBAR_GAP: f32 = 20.0;
pub const WORDMARK_ICON: f32 = 28.0;
pub const WORDMARK_GAP: f32 = 10.0;
pub const SEPARATOR_HEIGHT: f32 = 28.0;
pub const TITLE_GAP: f32 = 2.0;
pub const CHIP_PADDING: [f32; 2] = [12.0, 6.0];
pub const CHIP_DOT: f32 = 8.0;
pub const CHIP_GAP: f32 = 8.0;
pub const GEAR_SIZE: f32 = 40.0;
pub const GEAR_ICON: f32 = 22.0;

/// The widths below which the chip's text, then the wordmark's text, hide.
pub const CHIP_HIDDEN_AT: f32 = 640.0;
pub const WORDMARK_TEXT_HIDDEN_AT: f32 = 520.0;

/// The wordmark's word.
pub const WORDMARK: &str = "MathViz";

/// A zero min/max pair: grow/fit sizing with no constraint.
pub(crate) const ZERO_MINMAX: clay::SizingConstraintsMinMax =
    clay::SizingConstraintsMinMax { min: 0.0, max: 0.0 };

/// A layout with only its sizing set; the engine fills the rest with zeros.
///
/// Layout-construction helpers belong with the Clay setup; this local one
/// keeps the port compiling until that milestone provides them.
pub(crate) fn layout_config(sizing: clay::Sizing) -> clay::LayoutConfig {
    clay::LayoutConfig {
        sizing,
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
    }
}

/// An element declaration with only its layout set; everything else zero.
pub(crate) fn decl(layout: clay::LayoutConfig) -> clay::ElementDeclaration {
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
            offset: [0.0, 0.0],
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
            childOffset: [0.0, 0.0],
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

/// A floating config attached to the parent at an offset, above its siblings.
pub(crate) fn floating(
    offset: [f32; 2],
    element: clay::FloatingAttachPointType,
    parent: clay::FloatingAttachPointType,
    z: i16,
) -> clay::FloatingElementConfig {
    clay::FloatingElementConfig {
        offset,
        expand: clay::Dimensions {
            width: 0.0,
            height: 0.0,
        },
        parentId: 0,
        zIndex: z,
        attachment: clay::FloatingAttachPoints { element, parent },
        pointerCaptureMode: clay::PointerCaptureMode::Capture,
        attachTo: clay::FloatingAttachToElement::Parent,
        clipTo: clay::FloatingClipToElement::AttachedParent,
    }
}

/// Declares the responsive header row with wordmark, title and actions.
pub fn top_bar(app: &mut App, t: &mv_theme::Theme, window: f32, width: f32) {
    let show_name = window > WORDMARK_TEXT_HIDDEN_AT;
    let show_chip = window > CHIP_HIDDEN_AT;
    let chip_label = mv_theme::SLUG_LABELS[mv_theme::shown(app.prefs, app.system_scheme)];

    let mut wordmark_width = WORDMARK_ICON;
    if show_name {
        wordmark_width += WORDMARK_GAP
            + crate::typography::type_width(app, crate::typography::Type::Wordmark, WORDMARK);
    }
    let chip_padding = 2.0 * (CHIP_PADDING[0] + 1.0) + CHIP_DOT + CHIP_GAP;
    let chip_width = chip_padding
        + crate::typography::type_width(app, crate::typography::Type::ThemeChip, chip_label);
    let mut actions = crate::flex::FlexItem {
        base: GEAR_SIZE,
        min: GEAR_SIZE,
        none: false,
    };
    if show_chip {
        actions.base += chip_width + TOPBAR_GAP;
        actions.min += chip_padding
            + crate::typography::longest_word_width(
                app,
                crate::typography::Type::ThemeChip,
                chip_label,
            )
            + TOPBAR_GAP;
    }
    let title =
        crate::typography::type_width(app, crate::typography::Type::Title, mv_viewer::VIEW_TITLE);
    let subtitle = crate::typography::type_width(
        app,
        crate::typography::Type::Subtitle,
        mv_viewer::VIEW_SUBTITLE,
    );
    let items = [
        crate::flex::FlexItem {
            base: wordmark_width,
            min: wordmark_width,
            none: false,
        },
        crate::flex::FlexItem {
            base: 1.0,
            min: 1.0,
            none: true,
        },
        crate::flex::FlexItem {
            base: title.max(subtitle),
            min: 0.0,
            none: false,
        },
        actions,
    ];
    let mut sizes = [0.0f32; 4];
    let room = width - 2.0 * (TOPBAR_PADDING + 1.0) - (items.len() as f32 - 1.0) * TOPBAR_GAP;
    crate::flex::flex_shrink(&items, room, &mut sizes);
    let block = sizes[2];
    let slot = room - sizes[0] - sizes[1] - block;

    let (background, border, radius) = crate::ui::card(t);
    let mut top = layout_config(clay::Sizing {
        width: mv_clay::sizing_grow(ZERO_MINMAX),
        height: mv_clay::sizing_fixed(TOPBAR_HEIGHT),
    });
    top.padding = clay::Padding {
        left: (TOPBAR_PADDING + 1.0) as u16,
        right: (TOPBAR_PADDING + 1.0) as u16,
        top: 0,
        bottom: 0,
    };
    top.childGap = TOPBAR_GAP as u16;
    top.childAlignment = clay::ChildAlignment {
        x: clay::LayoutAlignmentX::Left,
        y: clay::LayoutAlignmentY::Center,
    };
    let mut top_decl = decl(top);
    top_decl.backgroundColor = background;
    top_decl.border = border;
    top_decl.cornerRadius = radius;
    crate::ui::ui_scope(app, mv_clay::id("TopBar", 0), &top_decl, |app| {
        let wordmark = mv_clay::id("Wordmark", 0);
        let mut word_decl = layout_config(clay::Sizing {
            width: mv_clay::sizing_fixed(wordmark_width),
            height: mv_clay::sizing_fixed(WORDMARK_ICON),
        });
        word_decl.childGap = WORDMARK_GAP as u16;
        word_decl.childAlignment = clay::ChildAlignment {
            x: clay::LayoutAlignmentX::Left,
            y: clay::LayoutAlignmentY::Center,
        };
        crate::ui::ui_scope(app, wordmark, &decl(word_decl), |app| {
            icon(
                app,
                mv_clay::id("WordmarkIcon", 0),
                IconKind::Snowflake,
                WORDMARK_ICON,
                mv_theme::Token::Primary,
            );
            if show_name {
                crate::typography::label(
                    app,
                    crate::typography::Type::Wordmark,
                    WORDMARK,
                    mv_theme::Token::Foreground,
                );
            }
        });
        crate::input::region(
            app,
            wordmark,
            crate::input::Hit {
                kind: crate::input::HitKind::Wordmark,
                index: 0,
            },
            crate::input::Cursor::Pointer,
            crate::input::Cursor::Pointer,
            true,
            mv_clay::corner_radius_all(6.0),
        );

        let sep = layout_config(clay::Sizing {
            width: mv_clay::sizing_fixed(1.0),
            height: mv_clay::sizing_fixed(SEPARATOR_HEIGHT),
        });
        let mut sep_decl = decl(sep);
        sep_decl.backgroundColor = crate::ui::ink(t, mv_theme::Token::Border);
        crate::ui::ui_scope(app, mv_clay::id("Separator", 0), &sep_decl, |_| {});

        let mut title_block = layout_config(clay::Sizing {
            width: mv_clay::sizing_fixed(block),
            height: mv_clay::sizing_fit(ZERO_MINMAX),
        });
        title_block.childGap = TITLE_GAP as u16;
        title_block.layoutDirection = clay::LayoutDirection::TopToBottom;
        crate::ui::ui_scope(
            app,
            mv_clay::id("TitleBlock", 0),
            &decl(title_block),
            |app| {
                crate::typography::text_line(
                    app,
                    mv_clay::id("Title", 0),
                    crate::typography::Type::Title,
                    mv_viewer::VIEW_TITLE,
                    mv_theme::Token::Foreground,
                    crate::typography::Overflow::Ellipsis,
                    Some(mv_clay::sizing_fixed(title.min(block))),
                );
                crate::typography::text_line(
                    app,
                    mv_clay::id("Subtitle", 0),
                    crate::typography::Type::Subtitle,
                    mv_viewer::VIEW_SUBTITLE,
                    mv_theme::Token::ForegroundMuted,
                    crate::typography::Overflow::Ellipsis,
                    Some(mv_clay::sizing_fixed(subtitle.min(block))),
                );
            },
        );

        let mut slot_layout = layout_config(clay::Sizing {
            width: mv_clay::sizing_fixed(slot),
            height: mv_clay::sizing_fit(ZERO_MINMAX),
        });
        slot_layout.childAlignment = clay::ChildAlignment {
            x: clay::LayoutAlignmentX::Right,
            y: clay::LayoutAlignmentY::Center,
        };
        crate::ui::ui_scope(
            app,
            mv_clay::id("ActionsSlot", 0),
            &decl(slot_layout),
            |app| {
                let mut acts = layout_config(clay::Sizing {
                    width: mv_clay::sizing_fit(ZERO_MINMAX),
                    height: mv_clay::sizing_fit(ZERO_MINMAX),
                });
                acts.childGap = TOPBAR_GAP as u16;
                acts.childAlignment = clay::ChildAlignment {
                    x: clay::LayoutAlignmentX::Left,
                    y: clay::LayoutAlignmentY::Center,
                };
                crate::ui::ui_scope(app, mv_clay::id("Actions", 0), &decl(acts), |app| {
                    if show_chip {
                        theme_chip(app, t, chip_label, chip_width);
                    }
                    gear(app, t);
                });
            },
        );
    });
}

/// Declares the theme label chip with its dot.
pub fn theme_chip(app: &mut App, t: &mv_theme::Theme, label_text: &str, width: f32) {
    let mut layout = layout_config(clay::Sizing {
        width: mv_clay::sizing_fixed(width),
        height: mv_clay::sizing_fit(ZERO_MINMAX),
    });
    layout.padding = clay::Padding {
        left: (CHIP_PADDING[0] + 1.0) as u16,
        right: (CHIP_PADDING[0] + 1.0) as u16,
        top: (CHIP_PADDING[1] + 1.0) as u16,
        bottom: (CHIP_PADDING[1] + 1.0) as u16,
    };
    layout.childGap = CHIP_GAP as u16;
    layout.childAlignment = clay::ChildAlignment {
        x: clay::LayoutAlignmentX::Left,
        y: clay::LayoutAlignmentY::Center,
    };
    let mut chip = decl(layout);
    chip.backgroundColor = crate::ui::ink(t, mv_theme::Token::Card);
    chip.border = clay::BorderElementConfig {
        color: crate::ui::ink(t, mv_theme::Token::Border),
        width: mv_clay::border_outside(1),
    };
    chip.cornerRadius = mv_clay::corner_radius_all(999.0);
    crate::ui::ui_scope(app, mv_clay::id("ThemeChip", 0), &chip, |app| {
        let mut dot = decl(layout_config(clay::Sizing {
            width: mv_clay::sizing_fixed(CHIP_DOT),
            height: mv_clay::sizing_fixed(CHIP_DOT),
        }));
        dot.backgroundColor = crate::ui::ink(t, mv_theme::Token::Primary);
        dot.cornerRadius = mv_clay::corner_radius_all(CHIP_DOT / 2.0);
        crate::ui::ui_scope(app, mv_clay::id("ChipDot", 0), &dot, |_| {});
        crate::typography::label(
            app,
            crate::typography::Type::ThemeChip,
            label_text,
            mv_theme::Token::Foreground,
        );
    });
}

/// Declares the settings gear button and its panel.
pub fn gear(app: &mut App, t: &mv_theme::Theme) {
    let id = mv_clay::id("Gear", 0);
    let hit = crate::input::Hit {
        kind: crate::input::HitKind::Gear,
        index: 0,
    };
    let lit = app.settings.open
        || app.input.hover == hit
        || (app.input.focus_visible && app.input.focus == hit);
    let mut layout = layout_config(clay::Sizing {
        width: mv_clay::sizing_fixed(GEAR_SIZE),
        height: mv_clay::sizing_fixed(GEAR_SIZE),
    });
    layout.childAlignment = clay::ChildAlignment {
        x: clay::LayoutAlignmentX::Center,
        y: clay::LayoutAlignmentY::Center,
    };
    let mut gear_decl = decl(layout);
    gear_decl.backgroundColor = crate::ui::ink(t, mv_theme::Token::Secondary);
    gear_decl.border = clay::BorderElementConfig {
        color: crate::ui::ink(
            t,
            if lit {
                mv_theme::Token::Primary
            } else {
                mv_theme::Token::Border
            },
        ),
        width: mv_clay::border_outside(1),
    };
    gear_decl.cornerRadius = mv_clay::corner_radius_all(GEAR_SIZE / 2.0);
    crate::ui::ui_scope(app, id, &gear_decl, |app| {
        icon(
            app,
            mv_clay::id("GearIcon", 0),
            IconKind::Gear,
            GEAR_ICON,
            mv_theme::Token::Foreground,
        );
    });
    crate::input::region(
        app,
        id,
        hit,
        crate::input::Cursor::Pointer,
        crate::input::Cursor::Pointer,
        true,
        mv_clay::corner_radius_all(GEAR_SIZE / 2.0),
    );
    crate::settings::settings_panel(app, t, id);
}

/// Declares a square icon element.
pub fn icon(
    app: &mut App,
    id: clay::ElementId,
    kind: IconKind,
    size: f32,
    ink_token: mv_theme::Token,
) {
    let custom = crate::widgets::Custom::Icon(crate::icons::Icon {
        kind,
        ink: crate::ui::current_theme(app).colors[ink_token],
    });
    let mut icon_decl = decl(layout_config(clay::Sizing {
        width: mv_clay::sizing_fixed(size),
        height: mv_clay::sizing_fixed(size),
    }));
    icon_decl.custom = crate::widgets::custom_data(custom);
    crate::ui::ui_scope(app, id, &icon_decl, |_| {});
}
