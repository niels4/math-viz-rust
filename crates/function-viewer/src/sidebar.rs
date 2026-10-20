//! The left panel: the function card with picker and equation, the transform
//! card with its rows, and the points card.
//!
//! The panel scrolls three cards; each parameter row carries its chip, name,
//! flip, reset, box, tip and ruler.

use enum_map::{Enum, EnumMap};
use mv_sys::clay;

use crate::app::App;
use crate::input::{Cursor, Hit, HitKind};

/// The panel width, padding and section gap.
pub const PANEL_WIDTH: f32 = 460.0;
pub const PANEL_PADDING: f32 = 18.0;
pub const SECTION_GAP: f32 = 12.0;

/// Each parameter's ink and band tokens.
pub static PARAM_INKS: EnumMap<mv_viewer::Param, mv_theme::Token> = EnumMap::from_array([
    mv_theme::Token::ParamA,
    mv_theme::Token::ParamK,
    mv_theme::Token::ParamB,
    mv_theme::Token::ParamH,
]);
/// Each parameter's band token.
pub static PARAM_BANDS: EnumMap<mv_viewer::Param, mv_theme::Token> = EnumMap::from_array([
    mv_theme::Token::ParamA18,
    mv_theme::Token::ParamK18,
    mv_theme::Token::ParamB18,
    mv_theme::Token::ParamH18,
]);

/// Declares the scrolling panel with its three cards and foot tip.
pub fn panel(app: &mut App, t: &mv_theme::Theme, stacked: bool) {
    let (background, border, radius) = crate::ui::card(t);
    let sizing = if stacked {
        clay::Sizing {
            width: mv_clay::sizing_grow(crate::top_bar::ZERO_MINMAX),
            height: mv_clay::sizing_fit(crate::top_bar::ZERO_MINMAX),
        }
    } else {
        clay::Sizing {
            width: mv_clay::sizing_fixed(PANEL_WIDTH),
            height: mv_clay::sizing_grow(crate::top_bar::ZERO_MINMAX),
        }
    };
    let mut panel_layout = crate::top_bar::layout_config(sizing);
    panel_layout.padding = mv_clay::padding_all((PANEL_PADDING + 1.0) as u16);
    panel_layout.childGap = SECTION_GAP as u16;
    panel_layout.layoutDirection = clay::LayoutDirection::TopToBottom;
    let mut panel_decl = crate::top_bar::decl(panel_layout);
    panel_decl.backgroundColor = background;
    panel_decl.border = border;
    panel_decl.cornerRadius = radius;
    panel_decl.clip = clay::ClipElementConfig {
        horizontal: false,
        vertical: !stacked,
        childOffset: crate::scroll::scrolled(app, crate::scroll::Scroller::Panel),
    };
    let panel_id = mv_clay::id(
        crate::scroll::SCROLLER_IDS[crate::scroll::Scroller::Panel],
        0,
    );
    crate::ui::ui_scope(app, panel_id, &panel_decl, |app| {
        if !stacked {
            app.input.clip = Some(crate::scroll::Scroller::Panel);
        }
        function_card(app, t);
        transform_card(app, t);
        crate::point_rows::points_card(app, t);
        if mv_viewer::hint_context(&app.viewer).kind == mv_viewer::HintKind::PlaneKeys {
            let width = if stacked {
                app.size[0] - 2.0 * crate::ui::PAGE_PADDING
            } else {
                PANEL_WIDTH
            };
            crate::tip::foot_tip(app, t, width - 2.0 * (PANEL_PADDING + 1.0));
        }
        if !stacked {
            app.input.clip = None;
        }
    });
}

/// A sub-card's element declaration.
pub fn sub_card(t: &mv_theme::Theme) -> clay::ElementDeclaration {
    let mut layout = crate::top_bar::layout_config(clay::Sizing {
        width: mv_clay::sizing_grow(crate::top_bar::ZERO_MINMAX),
        height: mv_clay::sizing_fit(crate::top_bar::ZERO_MINMAX),
    });
    layout.padding = clay::Padding {
        left: 14,
        right: 14,
        top: 12,
        bottom: 16,
    };
    layout.childGap = 12;
    layout.layoutDirection = clay::LayoutDirection::TopToBottom;
    let mut sub = crate::top_bar::decl(layout);
    sub.backgroundColor = crate::ui::ink(t, mv_theme::Token::Section);
    sub.cornerRadius = mv_clay::corner_radius_all(t.card_radius);
    sub
}

/// Declares a 28 px card title head.
pub fn card_head(app: &mut App, section: mv_viewer::Section) {
    let mut head = crate::top_bar::layout_config(clay::Sizing {
        width: mv_clay::sizing_grow(crate::top_bar::ZERO_MINMAX),
        height: mv_clay::sizing_fixed(28.0),
    });
    head.childAlignment = clay::ChildAlignment {
        x: clay::LayoutAlignmentX::Left,
        y: clay::LayoutAlignmentY::Center,
    };
    crate::ui::ui_scope(
        app,
        mv_clay::id("CardHead", section as u32),
        &crate::top_bar::decl(head),
        |app| {
            crate::typography::label(
                app,
                crate::typography::Type::CardTitle,
                mv_viewer::SECTION_TITLES[section],
                mv_theme::Token::Foreground,
            );
        },
    );
}

/// Declares the picker and the equation card with its term overlays.
pub fn function_card(app: &mut App, t: &mv_theme::Theme) {
    let id = mv_clay::id("Card", mv_viewer::Section::Function as u32);
    crate::ui::ui_scope(app, id, &sub_card(t), |app| {
        card_head(app, mv_viewer::Section::Function);
        function_picker(app, t);
        let eq = crate::equation::set_equation(
            &mut app.fonts,
            t,
            &app.viewer,
            app.equation_width,
            mv_viewer::active_params(&app.viewer),
        );
        // The card's region goes first, its terms' lie over it (sidebar.odin:
        // the region takes the equation's id, not the card's, so the picker's
        // options above it keep the pointer).
        let eq_id = mv_clay::id("Equation", 0);
        crate::input::region(
            app,
            eq_id,
            Hit {
                kind: HitKind::Equation,
                index: 0,
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
        let height = crate::equation::equation_height(&eq);
        let custom = crate::widgets::Custom::Equation(eq);
        let mut eq_decl = crate::top_bar::decl(crate::top_bar::layout_config(clay::Sizing {
            width: mv_clay::sizing_grow(crate::top_bar::ZERO_MINMAX),
            height: mv_clay::sizing_fixed(height),
        }));
        eq_decl.custom = crate::widgets::custom_data(custom.clone());
        crate::ui::ui_scope(app, eq_id, &eq_decl, |app| {
            if let crate::widgets::Custom::Equation(ref eq) = custom {
                crate::equation::equation_terms(app, t, eq);
            }
        });
    });
}

/// Builds the segmented control from the base functions.
pub fn function_picker(app: &mut App, t: &mv_theme::Theme) {
    let mut options = Vec::with_capacity(4);
    for (i, base) in [
        mv_viewer::Base::X,
        mv_viewer::Base::X2,
        mv_viewer::Base::X3,
        mv_viewer::Base::Sin,
    ]
    .iter()
    .enumerate()
    {
        options.push(crate::widgets::Segment {
            line: crate::widgets::math_line(
                &mut app.fonts,
                mv_viewer::BASE_FUNCTIONS[*base].label,
                PICKER_MATH_SIZE,
                0.0,
                [0.0; 4],
            ),
            icon: None,
            hit: Hit {
                kind: HitKind::Picker,
                index: i as i64,
            },
            picked: *base == app.viewer.func,
        });
    }
    crate::widgets::segmented_control(
        app,
        t,
        "Picker",
        &options,
        crate::widgets::SegmentFocus::Ring,
    );
}

/// The picker's math size.
pub const PICKER_MATH_SIZE: f32 = 20.0;

/// Focuses the wrapped option and changes the function.
pub fn picker_choose(app: &mut App, index: i64, keyboard: bool) {
    let n = mv_viewer::Base::LENGTH as i64;
    let i = (index % n + n) % n;
    crate::input::set_focus(
        app,
        Hit {
            kind: HitKind::Picker,
            index: i,
        },
        keyboard,
        false,
    );
    let func = mv_viewer::Base::from_usize(i as usize);
    if func != app.viewer.func {
        crate::parts::dispatch(
            app,
            mv_viewer::Action::SetFunction(mv_viewer::SetFunction { func }),
        );
    }
}

/// Moves the picker selection by arrow/Home/End.
pub fn picker_key(app: &mut App, key: sdl3::keyboard::Keycode) -> bool {
    let (to, ok) =
        crate::widgets::radio_target(key, app.viewer.func as i64, mv_viewer::Base::LENGTH as i64);
    if !ok {
        return false;
    }
    picker_choose(app, to, true);
    true
}

/// Declares the transform groups with their rule and Reset all.
pub fn transform_card(app: &mut App, t: &mv_theme::Theme) {
    let id = mv_clay::id("Card", mv_viewer::Section::Transform as u32);
    crate::ui::ui_scope(app, id, &sub_card(t), |app| {
        let transformed = mv_viewer::is_transformed(&app.viewer);
        let reset_all = mv_clay::id("ResetAll", 0);
        let mut head = crate::top_bar::layout_config(clay::Sizing {
            width: mv_clay::sizing_grow(crate::top_bar::ZERO_MINMAX),
            height: mv_clay::sizing_fixed(28.0),
        });
        head.childAlignment = clay::ChildAlignment {
            x: clay::LayoutAlignmentX::Left,
            y: clay::LayoutAlignmentY::Center,
        };
        crate::ui::ui_scope(
            app,
            mv_clay::id("CardHead", mv_viewer::Section::Transform as u32),
            &crate::top_bar::decl(head),
            |app| {
                crate::typography::label(
                    app,
                    crate::typography::Type::CardTitle,
                    mv_viewer::SECTION_TITLES[mv_viewer::Section::Transform],
                    mv_theme::Token::Foreground,
                );
                if transformed {
                    let gap = crate::top_bar::decl(crate::top_bar::layout_config(clay::Sizing {
                        width: mv_clay::sizing_grow(crate::top_bar::ZERO_MINMAX),
                        height: mv_clay::sizing_fit(crate::top_bar::ZERO_MINMAX),
                    }));
                    crate::ui::ui_scope(app, mv_clay::id("CardHeadGap", 0), &gap, |_| {});
                    reset_all_button(app, t, reset_all);
                }
            },
        );
        let mut rows = crate::top_bar::layout_config(clay::Sizing {
            width: mv_clay::sizing_grow(crate::top_bar::ZERO_MINMAX),
            height: mv_clay::sizing_fit(crate::top_bar::ZERO_MINMAX),
        });
        rows.childGap = 2;
        rows.layoutDirection = clay::LayoutDirection::TopToBottom;
        crate::ui::ui_scope(
            app,
            mv_clay::id("Rows", 0),
            &crate::top_bar::decl(rows),
            |app| {
                for (i, group) in mv_viewer::TRANSFORM_GROUPS.iter().enumerate() {
                    if i > 0 {
                        let mut rule = crate::top_bar::layout_config(clay::Sizing {
                            width: mv_clay::sizing_grow(crate::top_bar::ZERO_MINMAX),
                            height: mv_clay::sizing_fit(crate::top_bar::ZERO_MINMAX),
                        });
                        rule.padding = clay::Padding {
                            left: 8,
                            right: 8,
                            top: 6,
                            bottom: 6,
                        };
                        crate::ui::ui_scope(
                            app,
                            mv_clay::id("Rule", 0),
                            &crate::top_bar::decl(rule),
                            |app| {
                                let mut line = crate::top_bar::decl(crate::top_bar::layout_config(
                                    clay::Sizing {
                                        width: mv_clay::sizing_grow(crate::top_bar::ZERO_MINMAX),
                                        height: mv_clay::sizing_fixed(1.0),
                                    },
                                ));
                                line.backgroundColor =
                                    crate::ui::ink(t, mv_theme::Token::BackgroundMuted);
                                crate::ui::ui_scope(app, mv_clay::id("RuleLine", 0), &line, |_| {});
                            },
                        );
                    }
                    let group_layout = crate::top_bar::layout_config(clay::Sizing {
                        width: mv_clay::sizing_grow(crate::top_bar::ZERO_MINMAX),
                        height: mv_clay::sizing_fit(crate::top_bar::ZERO_MINMAX),
                    });
                    let mut group_layout = group_layout;
                    group_layout.layoutDirection = clay::LayoutDirection::TopToBottom;
                    crate::ui::ui_scope(
                        app,
                        mv_clay::id("Group", i as u32),
                        &crate::top_bar::decl(group_layout),
                        |app| {
                            for p in group.iter() {
                                param_row(app, t, *p);
                            }
                        },
                    );
                }
            },
        );
        if transformed {
            crate::input::region(
                app,
                reset_all,
                Hit {
                    kind: HitKind::ResetAll,
                    index: 0,
                },
                Cursor::Pointer,
                Cursor::Pointer,
                true,
                mv_clay::corner_radius_all(6.0),
            );
        }
    });
}

/// Declares the Reset all button.
pub fn reset_all_button(app: &mut App, t: &mv_theme::Theme, id: clay::ElementId) {
    let hit = Hit {
        kind: HitKind::ResetAll,
        index: 0,
    };
    let hot = app.input.hover == hit || (app.input.focus_visible && app.input.focus == hit);
    let words = if hot {
        mv_theme::Token::Foreground
    } else {
        mv_theme::Token::ForegroundMuted
    };
    let mut layout = crate::top_bar::layout_config(clay::Sizing {
        width: mv_clay::sizing_fit(crate::top_bar::ZERO_MINMAX),
        height: mv_clay::sizing_fixed(20.0),
    });
    layout.padding = clay::Padding {
        left: 4,
        right: 4,
        top: 2,
        bottom: 2,
    };
    layout.childGap = 5;
    layout.childAlignment = clay::ChildAlignment {
        x: clay::LayoutAlignmentX::Left,
        y: clay::LayoutAlignmentY::Center,
    };
    let mut button = crate::top_bar::decl(layout);
    if hot {
        button.backgroundColor = crate::ui::ink(t, mv_theme::Token::Secondary);
    }
    button.cornerRadius = mv_clay::corner_radius_all(6.0);
    crate::ui::ui_scope(app, id, &button, |app| {
        let custom = crate::widgets::Custom::ResetGlyph(ResetGlyph {
            ink: t.colors[words],
        });
        let mut glyph = crate::top_bar::decl(crate::top_bar::layout_config(clay::Sizing {
            width: mv_clay::sizing_fixed(RESET_GLYPH_SIZE),
            height: mv_clay::sizing_fixed(RESET_GLYPH_SIZE),
        }));
        glyph.custom = crate::widgets::custom_data(custom);
        crate::ui::ui_scope(app, mv_clay::id("ResetAllGlyph", 0), &glyph, |_| {});
        crate::typography::text_line(
            app,
            mv_clay::id("ResetAllLabel", 0),
            crate::typography::Type::ResetAllLabel,
            mv_viewer::RESET_ALL,
            words,
            crate::typography::Overflow::Whole,
            None,
        );
    });
}

/// Resets all params and moves focus to a's box.
pub fn reset_all_activate(app: &mut App) {
    let focused = app.input.focus
        == Hit {
            kind: HitKind::ResetAll,
            index: 0,
        };
    let visible = app.input.focus_visible;
    crate::parts::dispatch(app, mv_viewer::Action::ResetAll(mv_viewer::ResetAll));
    if focused {
        crate::input::set_focus(
            app,
            Hit {
                kind: HitKind::Box,
                index: mv_viewer::Param::A.into_usize() as i64,
            },
            visible,
            false,
        );
    }
}

/// Dispatches a scale flip.
pub fn flip_activate(app: &mut App, p: mv_viewer::Param) {
    crate::parts::dispatch(app, mv_viewer::Action::Flip(mv_viewer::Flip { param: p }));
}

/// Whether a parameter is the active one in the model.
pub fn row_lit(app: &mut App, p: mv_viewer::Param) -> bool {
    mv_viewer::active_params(&app.viewer).contains(p)
}

/// Declares one chip/name/flip/reset/box/tip/ruler row.
pub fn param_row(app: &mut App, t: &mv_theme::Theme, p: mv_viewer::Param) {
    let ui = mv_viewer::part_ui(&app.viewer, mv_viewer::Part::from_usize(p.into_usize()));
    let mode = ui.mode;
    let dragging = mode.is_some();
    let editing_error = matches!(&ui.edit, Some(edit) if !edit.error.is_empty());
    let banded = (ui.lit && !editing_error) || dragging;
    let id = mv_clay::id("Row", p.into_usize() as u32);
    crate::input::region(
        app,
        id,
        Hit {
            kind: HitKind::Row,
            index: p.into_usize() as i64,
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
        height: mv_clay::sizing_fixed(ROW_HEIGHT),
    });
    row.padding = clay::Padding {
        left: 8,
        right: 6,
        top: 0,
        bottom: 0,
    };
    row.childGap = 10;
    row.childAlignment = clay::ChildAlignment {
        x: clay::LayoutAlignmentX::Left,
        y: clay::LayoutAlignmentY::Center,
    };
    let mut row_decl = crate::top_bar::decl(row);
    if banded {
        row_decl.backgroundColor = crate::ui::ink(t, PARAM_BANDS[p]);
    }
    row_decl.cornerRadius = mv_clay::corner_radius_all(ROW_RADIUS);
    crate::ui::ui_scope(app, id, &row_decl, |app| {
        let chip = mv_clay::id("Chip", p.into_usize() as u32);
        let mut chip_layout = crate::top_bar::layout_config(clay::Sizing {
            width: mv_clay::sizing_fixed(26.0),
            height: mv_clay::sizing_fixed(26.0),
        });
        chip_layout.childAlignment = clay::ChildAlignment {
            x: clay::LayoutAlignmentX::Center,
            y: clay::LayoutAlignmentY::Center,
        };
        let mut chip_decl = crate::top_bar::decl(chip_layout);
        chip_decl.backgroundColor = crate::ui::ink(t, PARAM_INKS[p]);
        chip_decl.cornerRadius = mv_clay::corner_radius_all(6.0);
        crate::ui::ui_scope(app, chip, &chip_decl, |app| {
            crate::typography::label(
                app,
                crate::typography::Type::ChipLetter,
                mv_viewer::PARAM_LETTERS[p],
                mv_theme::Token::Background,
            );
        });
        crate::input::region(
            app,
            chip,
            Hit {
                kind: HitKind::Chip,
                index: p.into_usize() as i64,
            },
            Cursor::Help,
            Cursor::Help,
            false,
            clay::CornerRadius {
                topLeft: 0.0,
                topRight: 0.0,
                bottomLeft: 0.0,
                bottomRight: 0.0,
            },
        );
        let mut slot = crate::top_bar::layout_config(clay::Sizing {
            width: mv_clay::sizing_grow(crate::top_bar::ZERO_MINMAX),
            height: mv_clay::sizing_fixed(ROW_HEIGHT),
        });
        slot.childAlignment = clay::ChildAlignment {
            x: clay::LayoutAlignmentX::Left,
            y: clay::LayoutAlignmentY::Center,
        };
        crate::ui::ui_scope(
            app,
            mv_clay::id("NameSlot", p.into_usize() as u32),
            &crate::top_bar::decl(slot),
            |app| {
                // The badge shows while dragging in a stepped mode; a dragging-Coarse
                // row on a wide window shows the name instead (sidebar.odin:316,330).
                let badge_mode = mode.filter(|m| dragging && *m != mv_scrub::Mode::Coarse);
                if let Some(mode) = badge_mode {
                    let held = if mode == mv_scrub::Mode::Fine {
                        mv_theme::Token::Accent
                    } else {
                        mv_theme::Token::Warning
                    };
                    let badge = ModeBadge {
                        text: mv_viewer::mode_badge(mode, mv_viewer::is_scale(p)),
                        ink: t.colors[held],
                        fill: mv_theme::badge(t, held),
                        words: t.colors[mv_theme::Token::Foreground],
                    };
                    let custom = crate::widgets::Custom::ModeBadge(badge);
                    let mut badge_decl =
                        crate::top_bar::decl(crate::top_bar::layout_config(clay::Sizing {
                            width: mv_clay::sizing_grow(crate::top_bar::ZERO_MINMAX),
                            height: mv_clay::sizing_fixed(BADGE_HEIGHT),
                        }));
                    badge_decl.custom = crate::widgets::custom_data(custom);
                    crate::ui::ui_scope(
                        app,
                        mv_clay::id("Badge", p.into_usize() as u32),
                        &badge_decl,
                        |_| {},
                    );
                } else if app.size[0] > NAMES_HIDDEN_AT {
                    crate::typography::text_line(
                        app,
                        mv_clay::id("Name", p.into_usize() as u32),
                        crate::typography::Type::RowName,
                        mv_viewer::PARAM_NAMES[p],
                        if banded {
                            mv_theme::Token::Foreground
                        } else {
                            mv_theme::Token::ForegroundMuted
                        },
                        crate::typography::Overflow::Whole,
                        Some(mv_clay::sizing_grow(crate::top_bar::ZERO_MINMAX)),
                    );
                }
            },
        );
        if mv_viewer::is_scale(p) {
            let flip = mv_clay::id("Flip", p.into_usize() as u32);
            let on = app.viewer.params[p] < 0.0;
            let hot = app.input.hover
                == Hit {
                    kind: HitKind::Flip,
                    index: p.into_usize() as i64,
                };
            let well = if on {
                PARAM_INKS[p]
            } else if hot {
                mv_theme::Token::Secondary
            } else {
                mv_theme::Token::BackgroundMuted
            };
            let mut flip_layout = crate::top_bar::layout_config(clay::Sizing {
                width: mv_clay::sizing_fixed(24.0),
                height: mv_clay::sizing_fixed(24.0),
            });
            flip_layout.childAlignment = clay::ChildAlignment {
                x: clay::LayoutAlignmentX::Center,
                y: clay::LayoutAlignmentY::Center,
            };
            let mut flip_decl = crate::top_bar::decl(flip_layout);
            flip_decl.backgroundColor = crate::ui::ink(t, well);
            flip_decl.cornerRadius = mv_clay::corner_radius_all(6.0);
            crate::ui::ui_scope(app, flip, &flip_decl, |app| {
                let custom = crate::widgets::Custom::FlipGlyph(crate::widgets::FlipGlyph {
                    vertical: p == mv_viewer::Param::A,
                    ink: t.colors[if on {
                        mv_theme::Token::Background
                    } else {
                        PARAM_INKS[p]
                    }],
                });
                let mut fg = crate::top_bar::decl(crate::top_bar::layout_config(clay::Sizing {
                    width: mv_clay::sizing_fixed(14.0),
                    height: mv_clay::sizing_fixed(14.0),
                }));
                fg.custom = crate::widgets::custom_data(custom);
                crate::ui::ui_scope(
                    app,
                    mv_clay::id("FlipGlyph", p.into_usize() as u32),
                    &fg,
                    |_| {},
                );
            });
            crate::input::region(
                app,
                flip,
                Hit {
                    kind: HitKind::Flip,
                    index: p.into_usize() as i64,
                },
                Cursor::Pointer,
                Cursor::Pointer,
                true,
                mv_clay::corner_radius_all(6.0),
            );
        }
        if !mv_viewer::is_at_default(app.viewer.params, p) && !dragging {
            reset_button(app, t, p);
        }
        crate::number_box::number_box(app, t, p, &ui);
        let (phrases, error) = row_tip_phrases(app, p, &ui);
        if !phrases.is_empty() {
            let mut row_width = ROW_WIDTH;
            let (region, found) = crate::input::region_of(
                app,
                Hit {
                    kind: HitKind::Row,
                    index: p.into_usize() as i64,
                },
            );
            if found {
                row_width = region.r#box.width;
            }
            crate::tip::row_tip(app, t, p, &phrases, error, row_width);
        }
        if let Some(mode) = mode {
            crate::ruler_tape::ruler_popover(app, t, p, mode);
        }
    });
}

/// Selects the editing/drag/focus/rest tip phrases for a row.
pub fn row_tip_phrases(
    app: &mut App,
    p: mv_viewer::Param,
    ui: &mv_viewer::PartUi,
) -> (Vec<mv_viewer::Phrase>, bool) {
    let defaults = mv_viewer::DEFAULT_PARAMS;
    if let Some(edit) = &ui.edit {
        if edit.error.is_empty() {
            return (mv_viewer::HINT_EDIT.to_vec(), false);
        }
        return (
            mv_viewer::hint_refused(&edit.error, &mv_numfmt::format_stored(edit.base, false)),
            true,
        );
    }
    if ui.mode.is_some() {
        return (Vec::new(), false);
    }
    let hit = Hit {
        kind: HitKind::Box,
        index: p.into_usize() as i64,
    };
    if app.input.focus == hit && app.input.keyed && !crate::number_field::field_open_on(app, p) {
        return (mv_viewer::tip_keys(defaults[p]), false);
    }
    if app.boxes.resting == Some(p) {
        return (mv_viewer::tip_drag(defaults[p]), false);
    }
    (Vec::new(), false)
}

/// Declares a row's reset button.
pub fn reset_button(app: &mut App, t: &mv_theme::Theme, p: mv_viewer::Param) {
    let id = mv_clay::id("Reset", p.into_usize() as u32);
    let hot = app.input.hover
        == Hit {
            kind: HitKind::Reset,
            index: p.into_usize() as i64,
        };
    let mut layout = crate::top_bar::layout_config(clay::Sizing {
        width: mv_clay::sizing_fixed(24.0),
        height: mv_clay::sizing_fixed(24.0),
    });
    layout.childAlignment = clay::ChildAlignment {
        x: clay::LayoutAlignmentX::Center,
        y: clay::LayoutAlignmentY::Center,
    };
    let mut reset = crate::top_bar::decl(layout);
    if hot {
        reset.backgroundColor = crate::ui::ink(t, mv_theme::Token::Secondary);
    }
    reset.cornerRadius = mv_clay::corner_radius_all(6.0);
    crate::ui::ui_scope(app, id, &reset, |app| {
        let custom = crate::widgets::Custom::ResetGlyph(ResetGlyph {
            ink: t.colors[if hot {
                mv_theme::Token::Foreground
            } else {
                mv_theme::Token::ForegroundMuted
            }],
        });
        let mut glyph = crate::top_bar::decl(crate::top_bar::layout_config(clay::Sizing {
            width: mv_clay::sizing_fixed(RESET_GLYPH_SIZE),
            height: mv_clay::sizing_fixed(RESET_GLYPH_SIZE),
        }));
        glyph.custom = crate::widgets::custom_data(custom);
        crate::ui::ui_scope(
            app,
            mv_clay::id("ResetGlyph", p.into_usize() as u32),
            &glyph,
            |_| {},
        );
    });
    crate::input::region(
        app,
        id,
        Hit {
            kind: HitKind::Reset,
            index: p.into_usize() as i64,
        },
        Cursor::Pointer,
        Cursor::Pointer,
        false,
        mv_clay::corner_radius_all(6.0),
    );
}

/// The reset glyph: a stroked arc with a filled head.
#[derive(Clone, Copy, Debug)]
pub struct ResetGlyph {
    pub ink: mv_theme::Color,
}

/// The reset glyph's size.
pub const RESET_GLYPH_SIZE: f32 = 16.0;

/// Strokes the arc and fills the arrow head.
pub fn draw_reset_glyph(vg: &mut mv_nvg::Context, at: [f32; 2], g: ResetGlyph) {
    vg.save();
    vg.translate(at[0], at[1]);
    vg.begin_path();
    let _ = crate::svgpath::draw_path(vg, "M12.213 4.465A5.5 5.5 0 1 1 4.465 3.787");
    vg.stroke_color(g.ink);
    vg.stroke_width(1.75);
    vg.line_cap(mv_nvg::LineCapType::ROUND);
    vg.stroke();
    vg.begin_path();
    let _ = crate::svgpath::draw_path(vg, "M9.013 0.865L14.813 2.065L11.813 6.865Z");
    vg.fill_color(g.ink);
    vg.fill();
    vg.restore();
}

/// A held modifier's badge.
#[derive(Clone, Debug)]
pub struct ModeBadge {
    pub text: String,
    pub ink: mv_theme::Color,
    pub fill: mv_theme::Color,
    pub words: mv_theme::Color,
}

/// The badge height and padding.
pub const BADGE_HEIGHT: f32 = 21.0;
pub const BADGE_PADDING: [f32; 2] = [7.5, 2.5];

/// Draws the modifier badge whole-or-nothing at the slot's left.
pub fn draw_mode_badge(app: &mut App, bounds: clay::BoundingBox, b: &ModeBadge) {
    let style = crate::typography::type_style(
        crate::typography::Type::Badge,
        crate::ui::current_theme(app),
    );
    let width = mv_text::layout_width(mv_text::measure(&mut app.fonts, style.style, &b.text))
        + 2.0 * BADGE_PADDING[0]
        + 2.0;
    if width > bounds.width + 1e-3 {
        return;
    }
    let r = clay::BoundingBox {
        x: bounds.x,
        y: bounds.y + (bounds.height - BADGE_HEIGHT) / 2.0,
        width,
        height: BADGE_HEIGHT,
    };
    crate::input::note_text(app, r);
    let radius = mv_clay::corner_radius_all(4.0);
    let vg = &mut app.vg;
    vg.begin_path();
    crate::draw::rounded_rect(vg, r, radius);
    vg.fill_color(crate::draw::css_background(b.fill));
    vg.fill();
    crate::effects::draw_inset(vg, r, radius, 1.0, b.ink);
    crate::effects::draw_inset(
        vg,
        crate::effects::grow_box(r, -1.0),
        crate::draw::inset_radius(radius, 1.0),
        0.5,
        b.ink,
    );
    let baseline = r.y
        + 1.0
        + BADGE_PADDING[1]
        + mv_text::baseline(
            &mut app.fonts,
            style.style.face,
            style.style.size,
            style.line_height,
        );
    mv_text::draw(
        &mut app.fonts,
        &mut app.vg,
        style.style,
        r.x + 1.0 + BADGE_PADDING[0],
        baseline,
        &b.text,
        b.words,
    );
}

/// The row height, radius, width, and the width hiding the names.
pub const ROW_HEIGHT: f32 = 44.0;
pub const ROW_RADIUS: f32 = 10.0;
pub const ROW_WIDTH: f32 = 394.0;
pub const NAMES_HIDDEN_AT: f32 = 469.0;

/// The sidebar's typeset parts against the reference's own boxes.
#[cfg(all(test, not(target_os = "emscripten")))]
pub(crate) mod tests {
    use super::*;
    use crate::equation::{EqLine, equation_height, set_equation};
    use crate::fixture::fixture_box;
    use crate::point_rows::{point_readout, point_readout_empty};

    const TOLERANCE: f32 = 0.02;

    fn near(a: f32, b: f32) -> bool {
        (a - b).abs() <= TOLERANCE
    }

    /// The line's runs against the fixture's runs of `name`, each from the
    /// line's box.
    fn expect_runs(
        name: &str,
        line: &EqLine,
        m: &crate::fixture::FixtureMeasures,
        bounds: [f32; 4],
    ) {
        let want = &m.runs[name];
        assert_eq!(
            line.runs.len(),
            want.len(),
            "{name}: {} runs, the reference's {}",
            line.runs.len(),
            want.len()
        );
        for (i, (r, w)) in line.runs.iter().zip(want.iter()).enumerate() {
            let x = w.x - bounds[0];
            let baseline = w.baseline - bounds[1];
            assert!(
                near(r.x, x) && near(r.baseline, baseline),
                "{name} run {i}: at {} on a baseline {} down; the reference's {:?} at {x} on {baseline}",
                r.x,
                r.baseline,
                w.text
            );
        }
    }

    /// The test app, its fixture shot and its plane view. Ported from
    /// `sidebar_test.odin:59` (`setup`): the headless constructor with fonts
    /// loaded, every theme resolved, the dark default prefs, the initial
    /// state, and the plane's view at 1440 x 900 (its box at its default
    /// zoom, centred).
    pub(crate) fn setup() -> (App, crate::fixture::FixtureShot) {
        let page = crate::fixture::page_fixture().expect("page fixture");
        let fixture = crate::fixture::fixture_shot(&page, "default").expect("default shot");
        let mut app = crate::fixture::headless_app();
        assert!(mv_text::init(&mut app.fonts));
        for i in 0..mv_theme::Slug::LENGTH {
            let slug = mv_theme::Slug::from_usize(i);
            app.themes[slug] = mv_theme::resolve(slug, mv_theme::Space::Srgb);
        }
        app.system_scheme = mv_theme::Scheme::Dark;
        app.prefs = mv_theme::DEFAULT_PREFS;
        app.viewer = mv_viewer::initial_state();
        let b = fixture_box(&fixture.webkit, "plane").expect("plane box");
        let zoom = mv_plane::default_zoom(b[3] as f64, mv_plane::DEFAULT_FIT_HALF_RANGE_Y);
        let half = [b[2] as f64 / 2.0 / zoom, b[3] as f64 / 2.0 / zoom];
        app.viewer.view = Some(mv_plane::View {
            zoom,
            extent: mv_plane::Extent {
                min_x: -half[0],
                max_x: half[0],
                min_y: -half[1],
                max_y: half[1],
            },
        });
        (app, fixture)
    }

    pub(crate) fn teardown(mut app: App) {
        mv_text::destroy(&mut app.fonts);
    }

    fn theme_of(app: &App) -> mv_theme::Theme {
        app.themes[mv_theme::shown(app.prefs, app.system_scheme)].clone()
    }

    #[test]
    fn equation_is_the_references() {
        let _guard = mv_clay::tests_lock();
        let (mut app, fixture) = setup();
        let m = &fixture.webkit;
        let card = fixture_box(m, "equation").expect("equation box");
        let live = fixture_box(m, "live_line").expect("live line box");
        let form = fixture_box(m, "form_line").expect("form line box");
        let bar = fixture_box(m, "bar").expect("bar box");
        let theme = theme_of(&app);
        let eq = set_equation(
            &mut app.fonts,
            &theme,
            &app.viewer,
            card[2],
            // The Odin test relies on the default `lit = {}` (equation.odin:319);
            // the initial state has no active param, so this equals active_params.
            enumset::EnumSet::<mv_viewer::Param>::empty(),
        );
        assert_eq!(eq.live.width, live[2]);
        assert_eq!(eq.live.height, live[3]);
        expect_runs("live", &eq.live, m, live);
        assert_eq!(eq.form.width, form[2]);
        assert_eq!(eq.form.height, form[3]);
        expect_runs("form", &eq.form, m, form);
        assert_eq!(eq.form.bars.len(), 1);
        let b = &eq.form.bars[0];
        assert_eq!(
            [b.x, b.y, b.width, b.height],
            [bar[0] - form[0], bar[1] - form[1], bar[2], bar[3]]
        );
        assert_eq!(equation_height(&eq), card[3]);
        teardown(app);
    }

    #[test]
    fn readouts_are_the_references() {
        let _guard = mv_clay::tests_lock();
        let (mut app, fixture) = setup();
        let wk = &fixture.webkit;
        let cr = &fixture.chromium;
        let by_name =
            |m: &crate::fixture::FixtureMeasures, name: &str, from: [f32; 4]| -> [f32; 4] {
                let b = &m.boxes[name];
                [b[0] - from[0], b[1] - from[1], b[2], b[3]]
            };
        let run_x =
            // Past the end yields -1, as the Odin helper does (sidebar_test.odin:145-148).
            |m: &crate::fixture::FixtureMeasures, line: &str, i: usize, from: [f32; 4]| -> f32 {
                m.runs[line].get(i).map(|r| r.x - from[0]).unwrap_or(-1.0)
            };
        let theme = theme_of(&app);
        let slots = mv_viewer::readout_slots(app.viewer.view);
        let p = point_readout(&mut app, &theme, 2.0, 4.0, slots, None).line;
        let rp = fixture_box(wk, "readout_p").expect("readout_p box");
        assert_eq!(p.width, rp[2]);
        assert_eq!(p.height, rp[3]);
        assert_eq!(p.placed.len(), 6);
        let x_box = by_name(cr, "readout_p_x", rp);
        let field = by_name(wk, "readout_p_field", rp);
        assert!(
            near(p.placed[0].pen, run_x(cr, "readout_p", 0, rp)),
            "f at {}",
            p.placed[0].pen
        );
        assert!(
            near(p.placed[1].pen, run_x(cr, "readout_p", 1, rp)),
            "( at {}",
            p.placed[1].pen
        );
        assert_eq!(p.placed[2].x, x_box[0]);
        assert_eq!(p.placed[2].width, x_box[2]);
        assert_eq!(p.placed[2].top, x_box[1]);
        assert!(
            near(p.placed[2].pen, field[0]),
            "x's text at {}, the field at {}",
            p.placed[2].pen,
            field[0]
        );
        assert_eq!(field, by_name(cr, "readout_p_field", rp));
        let close = by_name(wk, "readout_p_close", rp);
        assert_eq!(p.placed[3].x, close[0]);
        assert_eq!(p.placed[3].width, close[2]);
        let rel = by_name(wk, "readout_p_rel", rp);
        assert_eq!(p.placed[4].x, rel[0]);
        assert_eq!(p.placed[4].width, rel[2]);
        assert!(
            near(p.placed[4].pen, run_x(wk, "readout_p", 3, rp)),
            "= at {}",
            p.placed[4].pen
        );
        let y = by_name(wk, "readout_p_y", rp);
        assert_eq!(p.placed[5].x, y[0]);
        assert_eq!(p.placed[5].width, y[2]);
        assert!(
            near(p.placed[5].pen, run_x(wk, "readout_p", 4, rp)),
            "y's text at {}",
            p.placed[5].pen
        );
        let theme = theme_of(&app);
        let q = point_readout_empty(&mut app, &theme).line;
        let rq = fixture_box(wk, "readout_q").expect("readout_q box");
        assert_eq!(q.width, rq[2]);
        assert_eq!(q.placed.len(), 6);
        assert_eq!(q.placed[2].x, by_name(cr, "readout_q_x", rq)[0]);
        assert_eq!(q.placed[5].width, by_name(wk, "readout_q_y", rq)[2]);
        teardown(app);
    }
}
