//! The appearance panel: the mode toggle, the theme rows with swatches, and
//! the panel's focus and key handling.
//!
//! The gear opens the floating panel; picking a mode or theme stores the prefs
//! and swaps the shown theme with a fade. (Declares against the motion
//! driver's overlay origin; wired with it.)

use enum_map::{Enum as _, EnumMap};
use mv_sys::clay;
use sdl3::keyboard::Keycode;

use crate::app::App;
use crate::effects::Layer;
use crate::icons::IconKind;
use crate::input::{Cursor, Hit, HitKind};
use crate::motion_driver::OverlayFrom;
use crate::typography::Type;

/// The settings panel: open or not, and its list's fade.
pub struct Settings {
    pub open: bool,
    pub scheme: mv_theme::Scheme,
    pub swapped: bool,
    pub swap: mv_motion::FadeIn,
}

/// The panel geometry, rows, swatch, check and stacking layer.
pub const SETTINGS_WIDTH: f32 = 304.0;
pub const SETTINGS_PADDING: f32 = 12.0;
pub const SETTINGS_DROP: f32 = 8.0;
pub const SETTINGS_GAP: f32 = 16.0;
pub const SETTINGS_SECTION_GAP: f32 = 8.0;
pub const SETTINGS_INSET: f32 = 10.0;
pub const SETTINGS_ROW_HEIGHT: f32 = 36.0;
pub const SETTINGS_ROW_GAP: f32 = 10.0;
pub const SETTINGS_ROW_CORNER: f32 = 4.0;
pub const SWATCH_SIZE: [f32; 2] = [30.0, 20.0];
pub const SWATCH_RADIUS: f32 = 5.0;
pub const CHECK_SIZE: f32 = 14.0;
pub const SETTINGS_Z: i16 = 20;

/// The panel's overlay origin: from under the gear.
pub const SETTINGS_FROM: OverlayFrom = OverlayFrom {
    scale: 0.9,
    origin: [SETTINGS_WIDTH, 0.0],
    offset: [0.0, -SETTINGS_DROP],
};

/// The Appearance section's title.
pub const SETTINGS_APPEARANCE: &str = "Appearance";

/// Each mode's label and icon.
pub static MODE_LABELS: EnumMap<mv_theme::Mode, &str> =
    EnumMap::from_array(["System", "Light", "Dark"]);
/// Each mode's icon.
pub static MODE_ICONS: EnumMap<mv_theme::Mode, IconKind> =
    EnumMap::from_array([IconKind::System, IconKind::Sun, IconKind::Moon]);
/// Each scheme's row label.
pub static THEME_LABELS: EnumMap<mv_theme::Scheme, &str> =
    EnumMap::from_array(["Dark theme", "Light theme"]);

/// Opens the Clay element `id` (or an anonymous one), configures it, runs
/// the children and closes it: the `UI(id)(config) { ... }` scope.
///
/// Ported from `settings.odin`'s `UI` scopes (`clay.odin:527-536`): through
/// `mv-clay`'s free functions on Clay's current context (which the layout
/// left open).
///
/// `explainer.rs` carries its own copy until `mv-clay` exposes element
/// scopes without an owner instance (its `t0c` API); the two stay identical.
fn ui(id: Option<clay::ElementId>, config: &clay::ElementDeclaration, children: impl FnOnce()) {
    match id {
        Some(id) => mv_clay::open_element(id),
        None => mv_clay::open_element_auto(),
    }
    mv_clay::configure_open_element(config);
    children();
    mv_clay::close_element();
}

/// Hashes an element id from a label and an index.
///
/// Ported from `settings.odin`'s `ID` uses (`clay.odin:595`).
fn element_id(label: &str, index: u32) -> clay::ElementId {
    mv_clay::id(label, index)
}

/// This layout's data for the element with `id`.
///
/// Ported from `settings.odin`'s layout reads (`clay.odin:493`
/// `GetElementData`): through `mv-clay`'s free function on the laid-out
/// frame.
fn element_data(id: clay::ElementId) -> clay::ElementData {
    mv_clay::element_data(id)
}

/// A zero sizing axis: the `{}` an unsized axis is.
fn axis_zero() -> clay::SizingAxis {
    clay::SizingAxis {
        constraints: clay::SizingConstraints {
            sizeMinMax: clay::SizingConstraintsMinMax { min: 0.0, max: 0.0 },
        },
        r#type: clay::SizingType::Fit,
    }
}

/// A zero layout config: the `{}` a layout with no set fields is.
fn layout_zero() -> clay::LayoutConfig {
    clay::LayoutConfig {
        sizing: clay::Sizing {
            width: axis_zero(),
            height: axis_zero(),
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
    }
}

/// The zero element declaration: the `{}` the layout literals leave the other
/// fields at.
fn decl_zero() -> clay::ElementDeclaration {
    clay::ElementDeclaration {
        layout: layout_zero(),
        backgroundColor: [0.0, 0.0, 0.0, 0.0],
        overlayColor: [0.0, 0.0, 0.0, 0.0],
        cornerRadius: clay::CornerRadius {
            topLeft: 0.0,
            topRight: 0.0,
            bottomLeft: 0.0,
            bottomRight: 0.0,
        },
        aspectRatio: clay::AspectRatioElementConfig { aspectRatio: 0.0 },
        image: clay::ImageElementConfig {
            imageData: core::ptr::null_mut(),
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
            customData: core::ptr::null_mut(),
        },
        clip: clay::ClipElementConfig {
            horizontal: false,
            vertical: false,
            childOffset: [0.0, 0.0],
        },
        border: clay::BorderElementConfig {
            color: [0.0, 0.0, 0.0, 0.0],
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
        userData: core::ptr::null_mut(),
    }
}

/// The zero effect: the `{}` a layer config without one is.
fn effect_zero() -> crate::effects::Effect {
    crate::effects::Effect {
        shadow: None,
        inset: 0.0,
        inset_ink: [0.0, 0.0, 0.0, 0.0],
        layer: None,
    }
}

/// The Appearance help line for a mode and scheme.
pub fn settings_help(mode: mv_theme::Mode, scheme: mv_theme::Scheme) -> String {
    match mode {
        mv_theme::Mode::System => {
            if scheme == mv_theme::Scheme::Dark {
                "Your system is dark right now.".to_string()
            } else {
                "Your system is light right now.".to_string()
            }
        }
        mv_theme::Mode::Light => "Always light.".to_string(),
        mv_theme::Mode::Dark => "Always dark.".to_string(),
    }
}

/// Declares the overlay panel with the mode toggle and theme rows.
pub fn settings_panel(app: &mut App, t: &mv_theme::Theme, gear: clay::ElementId) {
    let scheme = mv_theme::scheme_for(app.prefs.mode, app.system_scheme);
    if scheme != app.settings.scheme {
        app.settings.scheme = scheme;
        app.settings.swapped = app.settings.open;
        if app.settings.open {
            let now = app.motion.now;
            let ms = if app.reduced_motion {
                mv_motion::REDUCED_FADE_MS
            } else {
                mv_motion::FAST_MS
            };
            // `fade_in_start` before `motion_watch_fade`: the spec lists the
            // watch first, but both complete before `overlay_frame` reads them.
            mv_motion::fade_in_start(&mut app.settings.swap, now, ms, mv_motion::Ease::Linear);
            let swap = core::ptr::addr_of_mut!(app.settings.swap);
            crate::motion_driver::motion_watch_fade(app, swap);
        }
    }
    let (presence, layer) = crate::motion_driver::overlay_frame(
        app,
        crate::motion_driver::OverlayKind::Settings,
        app.settings.open,
        SETTINGS_FROM,
    );
    if presence == mv_motion::Presence::Closed {
        return;
    }
    let checked = settings_checked(app);
    if app.input.focus.kind == HitKind::SettingsTheme {
        app.input.focus = checked;
    }
    let (z, inert) = (app.input.z, app.input.inert);
    app.input.z = SETTINGS_Z;
    app.input.inert = presence == mv_motion::Presence::Closing;
    let mut swap = crate::effects::LAYER_REST;
    if app.settings.swapped {
        swap.alpha = mv_motion::fade_in_at(app.settings.swap, app.motion.now) as f32;
    }
    let id = element_id("SettingsPanel", 0);
    // Odin `settings.odin:129`: `layer_config(layer, {shadow = t.card_shadow})`.
    let (overlay, user_data) = crate::effects::layer_config(
        layer,
        crate::effects::Effect {
            shadow: t.card_shadow,
            inset: 0.0,
            inset_ink: [0.0, 0.0, 0.0, 0.0],
            layer: None,
        },
    );
    ui(
        Some(id),
        &clay::ElementDeclaration {
            layout: clay::LayoutConfig {
                sizing: clay::Sizing {
                    width: mv_clay::sizing_fixed(SETTINGS_WIDTH),
                    height: axis_zero(),
                },
                padding: mv_clay::padding_all((SETTINGS_PADDING + 1.0) as u16),
                childGap: SETTINGS_GAP as u16,
                layoutDirection: clay::LayoutDirection::TopToBottom,
                ..layout_zero()
            },
            backgroundColor: crate::ui::ink(t, mv_theme::Token::Card),
            border: clay::BorderElementConfig {
                color: crate::ui::ink(t, mv_theme::Token::Border),
                width: mv_clay::border_outside(1),
            },
            cornerRadius: mv_clay::corner_radius_all(t.card_radius),
            floating: clay::FloatingElementConfig {
                offset: [0.0, SETTINGS_DROP],
                expand: clay::Dimensions {
                    width: 0.0,
                    height: 0.0,
                },
                parentId: gear.id,
                zIndex: SETTINGS_Z,
                attachment: clay::FloatingAttachPoints {
                    element: clay::FloatingAttachPointType::RightTop,
                    parent: clay::FloatingAttachPointType::RightBottom,
                },
                pointerCaptureMode: clay::PointerCaptureMode::Capture,
                attachTo: clay::FloatingAttachToElement::ElementWithId,
                clipTo: clay::FloatingClipToElement::None,
            },
            overlayColor: overlay,
            userData: user_data,
            ..decl_zero()
        },
        || {
            crate::input::region(
                app,
                id,
                Hit {
                    kind: HitKind::SettingsPanel,
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
            let section = clay::ElementDeclaration {
                layout: clay::LayoutConfig {
                    sizing: clay::Sizing {
                        width: mv_clay::sizing_grow(clay::SizingConstraintsMinMax {
                            min: 0.0,
                            max: 0.0,
                        }),
                        height: axis_zero(),
                    },
                    childGap: SETTINGS_SECTION_GAP as u16,
                    layoutDirection: clay::LayoutDirection::TopToBottom,
                    ..layout_zero()
                },
                ..decl_zero()
            };
            ui(None, &section, || {
                settings_line(
                    app,
                    element_id("SettingsAppearance", 0),
                    Type::SettingsLabel,
                    SETTINGS_APPEARANCE,
                    crate::effects::LAYER_REST,
                );
                mode_toggle(app, t);
                settings_line(
                    app,
                    element_id("SettingsHelp", 0),
                    Type::SettingsHelp,
                    &settings_help(app.prefs.mode, scheme),
                    swap,
                );
            });
            ui(None, &section, || {
                settings_line(
                    app,
                    element_id("SettingsThemeLabel", 0),
                    Type::SettingsLabel,
                    THEME_LABELS[scheme],
                    swap,
                );
                ui(
                    Some(element_id("SettingsThemes", 0)),
                    &clay::ElementDeclaration {
                        layout: clay::LayoutConfig {
                            sizing: clay::Sizing {
                                width: mv_clay::sizing_grow(clay::SizingConstraintsMinMax {
                                    min: 0.0,
                                    max: 0.0,
                                }),
                                height: axis_zero(),
                            },
                            layoutDirection: clay::LayoutDirection::TopToBottom,
                            ..layout_zero()
                        },
                        ..decl_zero()
                    },
                    || {
                        for (row, slug) in mv_theme::SCHEME_SLUGS[scheme].into_iter().enumerate() {
                            theme_row(app, t, slug, row as i64, row as i64 == checked.index, swap);
                        }
                    },
                );
            });
        },
    );
    app.input.z = z;
    app.input.inert = inert;
}

/// Declares one faded label/help line.
fn settings_line(app: &mut App, id: clay::ElementId, ty: Type, s: &str, look: Layer) {
    let style = crate::typography::type_style(ty, crate::ui::current_theme(app));
    ui(
        Some(id),
        &clay::ElementDeclaration {
            layout: clay::LayoutConfig {
                sizing: clay::Sizing {
                    width: mv_clay::sizing_grow(clay::SizingConstraintsMinMax {
                        min: 0.0,
                        max: 0.0,
                    }),
                    height: mv_clay::sizing_fixed(style.line_height),
                },
                padding: clay::Padding {
                    left: SETTINGS_INSET as u16,
                    right: SETTINGS_INSET as u16,
                    top: 0,
                    bottom: 0,
                },
                ..layout_zero()
            },
            ..decl_zero()
        },
        || {
            let (overlay, user_data) = crate::effects::layer_config(look, effect_zero());
            ui(
                None,
                &clay::ElementDeclaration {
                    overlayColor: overlay,
                    userData: user_data,
                    ..decl_zero()
                },
                || {
                    crate::typography::label(app, ty, s, mv_theme::Token::ForegroundMuted);
                },
            );
        },
    );
}

/// Declares the System/Light/Dark segmented control.
fn mode_toggle(app: &mut App, t: &mv_theme::Theme) {
    let style = crate::typography::type_style(Type::SettingsOption, t);
    let options: [crate::widgets::Segment; mv_theme::Mode::LENGTH] = std::array::from_fn(|i| {
        let mode = mv_theme::Mode::from_usize(i);
        crate::widgets::Segment {
            line: mv_text::typeset(
                &mut app.fonts,
                &[mv_text::Piece {
                    style: style.style,
                    text: MODE_LABELS[mode].to_string(),
                    ink: [0.0, 0.0, 0.0, 0.0],
                    line_height: style.line_height,
                    r#box: mv_text::Box::Exact,
                    min_width: 0.0,
                    align: mv_text::Align::Left,
                    pad: [0.0, 0.0],
                }],
                0.0,
                mv_text::Vertical::Baseline,
                1.0,
            ),
            icon: Some(MODE_ICONS[mode]),
            hit: Hit {
                kind: HitKind::SettingsMode,
                index: i as i64,
            },
            picked: mode == app.prefs.mode,
        }
    });
    crate::widgets::segmented_control(
        app,
        t,
        "SettingsMode",
        &options,
        crate::widgets::SegmentFocus::Line,
    );
}

/// Declares one theme row with swatch, name, check and focus outline.
fn theme_row(
    app: &mut App,
    t: &mv_theme::Theme,
    slug: mv_theme::Slug,
    row: i64,
    checked: bool,
    look: Layer,
) {
    let id = element_id("SettingsTheme", row as u32);
    let hit = Hit {
        kind: HitKind::SettingsTheme,
        index: row,
    };
    let focused = app.input.focus_visible && app.input.focus == hit;
    let hot = !app.input.inert && (app.input.hover == hit || focused);
    let radius = mv_clay::corner_radius_all((t.card_radius - SETTINGS_ROW_CORNER).max(0.0));
    ui(
        Some(id),
        &clay::ElementDeclaration {
            layout: clay::LayoutConfig {
                sizing: clay::Sizing {
                    width: mv_clay::sizing_grow(clay::SizingConstraintsMinMax {
                        min: 0.0,
                        max: 0.0,
                    }),
                    height: mv_clay::sizing_fixed(SETTINGS_ROW_HEIGHT),
                },
                padding: clay::Padding {
                    left: SETTINGS_INSET as u16,
                    right: SETTINGS_INSET as u16,
                    top: 0,
                    bottom: 0,
                },
                childAlignment: clay::ChildAlignment {
                    x: clay::LayoutAlignmentX::Left,
                    y: clay::LayoutAlignmentY::Center,
                },
                ..layout_zero()
            },
            backgroundColor: if hot {
                crate::ui::ink(t, mv_theme::Token::Secondary)
            } else {
                [0.0, 0.0, 0.0, 0.0]
            },
            cornerRadius: radius,
            ..decl_zero()
        },
        || {
            let (overlay, user_data) = crate::effects::layer_config(look, effect_zero());
            ui(
                Some(element_id("SettingsThemeContent", row as u32)),
                &clay::ElementDeclaration {
                    layout: clay::LayoutConfig {
                        sizing: clay::Sizing {
                            width: mv_clay::sizing_grow(clay::SizingConstraintsMinMax {
                                min: 0.0,
                                max: 0.0,
                            }),
                            height: axis_zero(),
                        },
                        childGap: SETTINGS_ROW_GAP as u16,
                        childAlignment: clay::ChildAlignment {
                            x: clay::LayoutAlignmentX::Left,
                            y: clay::LayoutAlignmentY::Center,
                        },
                        ..layout_zero()
                    },
                    overlayColor: overlay,
                    userData: user_data,
                    ..decl_zero()
                },
                || {
                    ui(
                        Some(element_id("SettingsSwatch", row as u32)),
                        &clay::ElementDeclaration {
                            layout: clay::LayoutConfig {
                                sizing: clay::Sizing {
                                    width: mv_clay::sizing_fixed(SWATCH_SIZE[0]),
                                    height: mv_clay::sizing_fixed(SWATCH_SIZE[1]),
                                },
                                ..layout_zero()
                            },
                            custom: crate::widgets::custom_data(crate::widgets::Custom::Swatch(
                                Swatch {
                                    slug,
                                    outline: t.colors[mv_theme::Token::Border],
                                },
                            )),
                            ..decl_zero()
                        },
                        || {},
                    );
                    let name = crate::typography::type_style(Type::SettingsTheme, t);
                    ui(
                        Some(element_id("SettingsThemeName", row as u32)),
                        &clay::ElementDeclaration {
                            layout: clay::LayoutConfig {
                                sizing: clay::Sizing {
                                    width: mv_clay::sizing_grow(clay::SizingConstraintsMinMax {
                                        min: 0.0,
                                        max: 0.0,
                                    }),
                                    height: mv_clay::sizing_fixed(name.line_height),
                                },
                                ..layout_zero()
                            },
                            ..decl_zero()
                        },
                        || {
                            crate::typography::label(
                                app,
                                if checked {
                                    Type::SettingsThemeChecked
                                } else {
                                    Type::SettingsTheme
                                },
                                mv_theme::SLUG_LABELS[slug],
                                mv_theme::Token::Foreground,
                            );
                        },
                    );
                    let mut check = clay::ElementDeclaration {
                        layout: clay::LayoutConfig {
                            sizing: clay::Sizing {
                                width: mv_clay::sizing_fixed(CHECK_SIZE),
                                height: mv_clay::sizing_fixed(if checked {
                                    CHECK_SIZE
                                } else {
                                    0.0
                                }),
                            },
                            ..layout_zero()
                        },
                        ..decl_zero()
                    };
                    if checked {
                        check.custom = crate::widgets::custom_data(crate::widgets::Custom::Icon(
                            crate::icons::Icon {
                                kind: IconKind::Check,
                                ink: t.colors[mv_theme::Token::Primary],
                            },
                        ));
                    }
                    ui(Some(element_id("SettingsCheck", row as u32)), &check, || {});
                },
            );
            if focused {
                crate::widgets::outline(&crate::widgets::Outline {
                    of: id,
                    offset: -crate::widgets::OUTLINE_OFFSET,
                    width: crate::widgets::OUTLINE_WIDTH,
                    ink: t.colors[mv_theme::Token::Primary],
                    radius,
                });
            }
        },
    );
    crate::input::region(
        app,
        id,
        hit,
        Cursor::Pointer,
        Cursor::Default,
        checked,
        radius,
    );
}

/// A theme's swatch: whose, and its outline.
#[derive(Clone, Copy, Debug)]
pub struct Swatch {
    pub slug: mv_theme::Slug,
    pub outline: mv_theme::Color,
}

/// The swatch curve, point and stroke.
pub const SWATCH_CURVE: &str = "M6 4.63Q15 25.37 24 4.63";
pub const SWATCH_POINT: [f32; 3] = [21.0, 10.39, 2.25];
pub const SWATCH_STROKE: f32 = 1.75;

/// Draws the theme swatch page, curve and point with its border.
pub fn draw_swatch(app: &mut App, r#box: clay::BoundingBox, s: Swatch) {
    let t = app.themes[s.slug].clone();
    let radius = mv_clay::corner_radius_all(SWATCH_RADIUS);
    crate::effects::draw_ring(&mut app.vg, r#box, radius, 1.0, s.outline);
    let vg = &mut app.vg;
    vg.begin_path();
    crate::draw::rounded_rect(&mut *vg, r#box, radius);
    vg.fill_color(t.colors[mv_theme::Token::Background]);
    vg.fill();
    vg.save();
    vg.translate(r#box.x, r#box.y);
    vg.begin_path();
    let _ = crate::svgpath::draw_path(&mut *vg, SWATCH_CURVE);
    vg.stroke_color(t.colors[mv_theme::Token::ChartLine]);
    vg.stroke_width(SWATCH_STROKE);
    vg.line_cap(mv_sys::nvg::LineCapType::ROUND);
    vg.stroke();
    vg.begin_path();
    vg.circle(SWATCH_POINT[0], SWATCH_POINT[1], SWATCH_POINT[2]);
    vg.fill_color(t.colors[mv_theme::Token::ChartPoint1]);
    vg.fill();
    vg.restore();
}

/// The checked theme's row: the list's one stop, where opening puts the focus.
pub fn settings_checked(app: &mut App) -> Hit {
    Hit {
        kind: HitKind::SettingsTheme,
        index: mv_theme::slug_row(mv_theme::shown(app.prefs, app.system_scheme)),
    }
}

/// Closes the panel.
pub fn settings_close(app: &mut App) {
    app.settings.open = false;
    app.dirty = true;
}

/// Toggles the panel, focusing the checked theme or the gear.
pub fn gear_activate(app: &mut App, keyboard: bool) {
    if app.settings.open {
        settings_close(app);
        crate::input::set_focus(
            app,
            Hit {
                kind: HitKind::Gear,
                index: 0,
            },
            false,
            false,
        );
        return;
    }
    app.settings.open = true;
    app.settings.swapped = false;
    app.dirty = true;
    let checked = settings_checked(app);
    crate::input::set_focus(app, checked, keyboard, false);
}

/// Focuses a mode option and stores the mode.
pub fn settings_mode_choose(app: &mut App, index: i64, keyboard: bool) {
    crate::input::set_focus(
        app,
        Hit {
            kind: HitKind::SettingsMode,
            index,
        },
        keyboard,
        false,
    );
    let mut prefs = app.prefs;
    prefs.mode = mv_theme::Mode::from_usize(index as usize);
    crate::prefs::set_prefs(app, prefs);
}

/// Focuses a theme row and stores the themed prefs.
pub fn settings_theme_choose(app: &mut App, row: i64, keyboard: bool) {
    let scheme = mv_theme::scheme_for(app.prefs.mode, app.system_scheme);
    crate::input::set_focus(
        app,
        Hit {
            kind: HitKind::SettingsTheme,
            index: row,
        },
        keyboard,
        false,
    );
    let prefs = mv_theme::with_theme(app.prefs, mv_theme::SCHEME_SLUGS[scheme][row as usize]);
    crate::prefs::set_prefs(app, prefs);
}

/// Esc/Enter/arrows on the gear and panel.
pub fn settings_key(app: &mut App, focus: Hit, key: Keycode) -> bool {
    if !app.settings.open {
        return false;
    }
    if focus.kind == HitKind::Gear {
        if key == Keycode::Escape {
            settings_close(app);
            return true;
        }
        return false;
    }
    if key == Keycode::Escape || key == Keycode::Return {
        settings_close(app);
        crate::input::set_focus(
            app,
            Hit {
                kind: HitKind::Gear,
                index: 0,
            },
            true,
            false,
        );
        return true;
    }
    if focus.kind == HitKind::SettingsMode {
        let (to, ok) = crate::widgets::radio_target(
            key,
            app.prefs.mode.into_usize() as i64,
            mv_theme::Mode::LENGTH as i64,
        );
        if !ok {
            return false;
        }
        settings_mode_choose(app, to, true);
    } else {
        let (to, ok) = crate::widgets::radio_target(
            key,
            settings_checked(app).index,
            mv_theme::SCHEME_SLUGS[mv_theme::Scheme::Dark].len() as i64,
        );
        if !ok {
            return false;
        }
        settings_theme_choose(app, to, true);
    }
    true
}

/// Closes the open panel on an outside press.
pub fn settings_pressed(app: &mut App, hit: Hit) {
    if app.settings.open && !settings_wrap(hit.kind) {
        settings_close(app);
    }
}

/// Closes the panel when focus leaves the gear and panel.
pub fn settings_focus_moved(app: &mut App, before: Hit, after: Hit) {
    if app.settings.open
        && settings_wrap(before.kind)
        && after.kind != HitKind::None
        && !settings_wrap(after.kind)
    {
        settings_close(app);
    }
}

/// Whether the kind belongs to the gear/panel focus ring.
pub fn settings_wrap(kind: HitKind) -> bool {
    matches!(
        kind,
        HitKind::Gear | HitKind::SettingsMode | HitKind::SettingsTheme | HitKind::SettingsPanel
    )
}

/// Whether the kind takes the panel's internal line focus.
pub fn settings_line_focus(kind: HitKind) -> bool {
    kind == HitKind::SettingsMode || kind == HitKind::SettingsTheme
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdl3::keyboard::Keycode;

    const SLACK: f32 = 0.02;
    const BASELINE_SLACK: f32 = 1.0 / 64.0;

    /// A blank app: the headless constructor, as the Odin tests' `new(App)`.
    fn blank_app() -> App {
        crate::fixture::headless_app()
    }

    /// Inlines `motion_init`: its spec body, since the surface lacks it
    /// (app-plane adds it with the motion driver's bodies).
    fn watch_q_row(app: &mut App) {
        let q_row = core::ptr::addr_of_mut!(app.motion.q_row);
        crate::motion_driver::motion_watch_fade(app, q_row);
    }

    fn menu_app(scheme: mv_theme::Scheme, prefs: mv_theme::Prefs) -> App {
        let mut app = blank_app();
        app.viewer = mv_viewer::initial_state();
        app.system_scheme = scheme;
        app.prefs = prefs;
        app.r#loop.script_time = Some(0);
        watch_q_row(&mut app);
        app
    }

    fn rows_as_read(app: &mut App) -> [String; 3] {
        let scheme = mv_theme::scheme_for(app.prefs.mode, app.system_scheme);
        let checked = settings_checked(app).index;
        let mut out = [String::new(), String::new(), String::new()];
        for (row, slug) in mv_theme::SCHEME_SLUGS[scheme].into_iter().enumerate() {
            out[row] = format!(
                "{}{}",
                mv_theme::SLUG_LABELS[slug],
                if row as i64 == checked { " ✓" } else { "" }
            );
        }
        out
    }

    fn lay_out_menu(app: &mut App, size: [f32; 2]) -> Vec<clay::RenderCommand> {
        app.size = size;
        app.pixels = [(size[0] * 2.0) as i32, (size[1] * 2.0) as i32];
        crate::draw::settle_layout(app, size)
    }

    fn layout_app() -> App {
        let mut app = blank_app();
        assert!(mv_text::init(&mut app.fonts));
        for i in 0..mv_theme::Slug::LENGTH {
            let slug = mv_theme::Slug::from_usize(i);
            app.themes[slug] = mv_theme::resolve(slug, mv_theme::Space::Srgb);
        }
        // No `init_clay` here: it captures `&mut app` for the measure
        // callback, and returning `app` by value would move it out from
        // under that pointer. Each test inits after the final placement.
        app.system_scheme = mv_theme::Scheme::Dark;
        app.prefs = mv_theme::DEFAULT_PREFS;
        app.viewer = mv_viewer::initial_state();
        crate::plane_area::plane_init(&mut app);
        watch_q_row(&mut app);
        app
    }

    fn release_layout_app(app: &mut App) {
        crate::ui::release_clay(app);
        mv_text::destroy(&mut app.fonts);
    }

    fn region_of_found(app: &mut App, hit: Hit) -> bool {
        crate::input::region_of(app, hit).1
    }

    #[test]
    fn the_help_line_and_the_list_follow_the_mode_and_the_system() {
        struct Case {
            os: mv_theme::Scheme,
            mode: mv_theme::Mode,
            help: &'static str,
            label: &'static str,
        }
        let cases = [
            Case {
                os: mv_theme::Scheme::Dark,
                mode: mv_theme::Mode::System,
                help: "Your system is dark right now.",
                label: "Dark theme",
            },
            Case {
                os: mv_theme::Scheme::Light,
                mode: mv_theme::Mode::System,
                help: "Your system is light right now.",
                label: "Light theme",
            },
            Case {
                os: mv_theme::Scheme::Dark,
                mode: mv_theme::Mode::Light,
                help: "Always light.",
                label: "Light theme",
            },
            Case {
                os: mv_theme::Scheme::Light,
                mode: mv_theme::Mode::Dark,
                help: "Always dark.",
                label: "Dark theme",
            },
        ];
        for c in cases {
            let mut app = menu_app(
                c.os,
                mv_theme::Prefs {
                    mode: c.mode,
                    dark: mv_theme::Slug::ArcticIce,
                    light: mv_theme::Slug::CleanTeal,
                },
            );
            let scheme = mv_theme::scheme_for(c.mode, c.os);
            assert_eq!(settings_help(c.mode, scheme), c.help);
            assert_eq!(THEME_LABELS[scheme], c.label);
            let want = if scheme == mv_theme::Scheme::Dark {
                [
                    "Arctic Ice ✓".to_string(),
                    "Tron Cyan".to_string(),
                    "Midnight Violet".to_string(),
                ]
            } else {
                [
                    "Clean Teal ✓".to_string(),
                    "Educational Warm".to_string(),
                    "Sage Editorial".to_string(),
                ]
            };
            assert_eq!(rows_as_read(&mut app), want);
        }
        let want_labels: EnumMap<mv_theme::Mode, &str> =
            EnumMap::from_array(["System", "Light", "Dark"]);
        assert_eq!(MODE_LABELS, want_labels);
        let want_icons: EnumMap<mv_theme::Mode, IconKind> =
            EnumMap::from_array([IconKind::System, IconKind::Sun, IconKind::Moon]);
        assert_eq!(MODE_ICONS, want_icons);
    }

    #[test]
    fn a_pick_applies_at_once_and_the_panel_stays() {
        fn shown(app: &mut App) -> mv_theme::Slug {
            mv_theme::shown(app.prefs, app.system_scheme)
        }
        let mut app = menu_app(mv_theme::Scheme::Dark, mv_theme::DEFAULT_PREFS);
        gear_activate(&mut app, false);
        assert!(app.settings.open);
        settings_theme_choose(&mut app, 1, false);
        assert_eq!(shown(&mut app), mv_theme::Slug::TronCyan);
        assert_eq!(
            rows_as_read(&mut app),
            [
                "Arctic Ice".to_string(),
                "Tron Cyan ✓".to_string(),
                "Midnight Violet".to_string()
            ]
        );
        settings_mode_choose(&mut app, mv_theme::Mode::Light.into_usize() as i64, false);
        assert_eq!(shown(&mut app), mv_theme::Slug::CleanTeal);
        assert_eq!(
            rows_as_read(&mut app),
            [
                "Clean Teal ✓".to_string(),
                "Educational Warm".to_string(),
                "Sage Editorial".to_string()
            ]
        );
        settings_theme_choose(&mut app, 2, false);
        settings_mode_choose(&mut app, mv_theme::Mode::Dark.into_usize() as i64, false);
        assert_eq!(shown(&mut app), mv_theme::Slug::TronCyan);
        settings_mode_choose(&mut app, mv_theme::Mode::Light.into_usize() as i64, false);
        assert_eq!(shown(&mut app), mv_theme::Slug::SageEditorial);
        assert!(app.settings.open);
        assert_eq!(
            mv_theme::prefs_json(app.prefs),
            "{\"mode\":\"light\",\"dark\":\"tron-cyan\",\"light\":\"sage-editorial\"}"
        );
    }

    #[test]
    fn the_focus_goes_to_the_checked_theme_and_the_arrows_pick_and_wrap() {
        struct Step {
            key: Keycode,
            slug: mv_theme::Slug,
        }
        let mut app = menu_app(mv_theme::Scheme::Dark, mv_theme::DEFAULT_PREFS);
        gear_activate(&mut app, true);
        assert_eq!(
            app.input.focus,
            Hit {
                kind: HitKind::SettingsTheme,
                index: 0
            }
        );
        assert!(app.input.focus_visible);
        let steps = [
            Step {
                key: Keycode::Down,
                slug: mv_theme::Slug::TronCyan,
            },
            Step {
                key: Keycode::Right,
                slug: mv_theme::Slug::MidnightViolet,
            },
            Step {
                key: Keycode::Down,
                slug: mv_theme::Slug::ArcticIce,
            },
            Step {
                key: Keycode::Up,
                slug: mv_theme::Slug::MidnightViolet,
            },
            Step {
                key: Keycode::Left,
                slug: mv_theme::Slug::TronCyan,
            },
            Step {
                key: Keycode::Home,
                slug: mv_theme::Slug::ArcticIce,
            },
            Step {
                key: Keycode::End,
                slug: mv_theme::Slug::MidnightViolet,
            },
        ];
        for s in steps {
            let focus = app.input.focus;
            assert!(settings_key(&mut app, focus, s.key));
            assert_eq!(app.prefs.dark, s.slug);
            assert_eq!(
                app.input.focus,
                Hit {
                    kind: HitKind::SettingsTheme,
                    index: mv_theme::slug_row(s.slug)
                }
            );
        }
        crate::input::set_focus(
            &mut app,
            Hit {
                kind: HitKind::SettingsMode,
                index: mv_theme::Mode::System.into_usize() as i64,
            },
            true,
            false,
        );
        let focus = app.input.focus;
        assert!(settings_key(&mut app, focus, Keycode::Right));
        assert_eq!(
            app.input.focus,
            Hit {
                kind: HitKind::SettingsMode,
                index: mv_theme::Mode::Light.into_usize() as i64,
            }
        );
        assert_eq!(
            mv_theme::shown(app.prefs, app.system_scheme),
            mv_theme::Slug::CleanTeal
        );
        let focus = app.input.focus;
        let _ = settings_key(&mut app, focus, Keycode::Right);
        let focus = app.input.focus;
        let _ = settings_key(&mut app, focus, Keycode::Right);
        assert_eq!(
            app.input.focus,
            Hit {
                kind: HitKind::SettingsMode,
                index: mv_theme::Mode::System.into_usize() as i64,
            }
        );
        let focus = app.input.focus;
        let _ = settings_key(&mut app, focus, Keycode::End);
        assert_eq!(
            app.input.focus,
            Hit {
                kind: HitKind::SettingsMode,
                index: mv_theme::Mode::Dark.into_usize() as i64,
            }
        );
        let focus = app.input.focus;
        let _ = settings_key(&mut app, focus, Keycode::Home);
        assert_eq!(
            app.input.focus,
            Hit {
                kind: HitKind::SettingsMode,
                index: mv_theme::Mode::System.into_usize() as i64,
            }
        );
        let focus = app.input.focus;
        assert!(!settings_key(&mut app, focus, Keycode::Tab));
        let focus = app.input.focus;
        assert!(!settings_key(&mut app, focus, Keycode::M));
        assert!(app.settings.open);
        gear_activate(&mut app, false);
        gear_activate(&mut app, false);
        assert_eq!(
            app.input.focus,
            Hit {
                kind: HitKind::SettingsTheme,
                index: 2
            }
        );
        assert!(!app.input.focus_visible);
    }

    #[test]
    fn esc_and_enter_close_it_to_the_gear() {
        for key in [Keycode::Escape, Keycode::Return] {
            let mut app = menu_app(mv_theme::Scheme::Dark, mv_theme::DEFAULT_PREFS);
            gear_activate(&mut app, false);
            let focus = app.input.focus;
            assert!(settings_key(&mut app, focus, key));
            assert!(!app.settings.open);
            assert_eq!(
                app.input.focus,
                Hit {
                    kind: HitKind::Gear,
                    index: 0
                }
            );
            assert!(app.input.focus_visible);
            gear_activate(&mut app, true);
            let focus = app.input.focus;
            let _ = settings_key(&mut app, focus, Keycode::Down);
            let focus = app.input.focus;
            assert!(settings_key(&mut app, focus, key));
            assert_eq!(
                mv_theme::shown(app.prefs, app.system_scheme),
                mv_theme::Slug::TronCyan
            );
        }
        let mut app = menu_app(mv_theme::Scheme::Dark, mv_theme::DEFAULT_PREFS);
        gear_activate(&mut app, false);
        crate::input::set_focus(
            &mut app,
            Hit {
                kind: HitKind::Gear,
                index: 0,
            },
            true,
            false,
        );
        assert!(app.settings.open);
        assert!(settings_key(
            &mut app,
            Hit {
                kind: HitKind::Gear,
                index: 0
            },
            Keycode::Escape
        ));
        assert!(!app.settings.open);
        assert!(!settings_key(
            &mut app,
            Hit {
                kind: HitKind::Gear,
                index: 0
            },
            Keycode::Escape
        ));
    }

    #[test]
    fn the_gear_a_press_outside_and_the_focus_leaving_close_it() {
        let mut app = menu_app(mv_theme::Scheme::Dark, mv_theme::DEFAULT_PREFS);
        gear_activate(&mut app, false);
        gear_activate(&mut app, false);
        assert!(!app.settings.open);
        assert_eq!(
            app.input.focus,
            Hit {
                kind: HitKind::Gear,
                index: 0
            }
        );
        assert!(!app.input.focus_visible);

        gear_activate(&mut app, false);
        settings_pressed(
            &mut app,
            Hit {
                kind: HitKind::SettingsPanel,
                index: 0,
            },
        );
        assert!(app.settings.open);
        settings_pressed(
            &mut app,
            Hit {
                kind: HitKind::Plane,
                index: 0,
            },
        );
        assert!(!app.settings.open);

        gear_activate(&mut app, false);
        crate::input::set_focus(
            &mut app,
            Hit {
                kind: HitKind::Gear,
                index: 0,
            },
            true,
            false,
        );
        assert!(app.settings.open);
        crate::input::set_focus(
            &mut app,
            Hit {
                kind: HitKind::None,
                index: 0,
            },
            false,
            false,
        );
        assert!(app.settings.open);
        let checked = settings_checked(&mut app);
        crate::input::set_focus(&mut app, checked, true, false);
        crate::input::set_focus(
            &mut app,
            Hit {
                kind: HitKind::Picker,
                index: 0,
            },
            true,
            false,
        );
        assert!(!app.settings.open);
        crate::input::set_focus(
            &mut app,
            Hit {
                kind: HitKind::Gear,
                index: 0,
            },
            true,
            false,
        );
        crate::input::set_focus(
            &mut app,
            Hit {
                kind: HitKind::Wordmark,
                index: 0,
            },
            true,
            false,
        );
        assert!(!app.settings.open);
    }

    #[test]
    fn it_comes_out_of_the_gear_and_goes_back() {
        fn look(app: &mut App, ms: f64) -> (mv_motion::Presence, Layer) {
            app.motion.now = ms / 1000.0;
            let open = app.settings.open;
            crate::motion_driver::overlay_frame(
                app,
                crate::motion_driver::OverlayKind::Settings,
                open,
                SETTINGS_FROM,
            )
        }
        fn in_by(l: Layer) -> f32 {
            (l.scale - 0.9) / 0.1
        }
        let mut app = menu_app(mv_theme::Scheme::Dark, mv_theme::DEFAULT_PREFS);
        app.settings.open = true;
        let (_, l) = look(&mut app, 0.0);
        assert_eq!(l.alpha, 0.0);
        assert_eq!(l.scale, 0.9);
        let (_, l) = look(&mut app, 50.0);
        assert!(l.alpha < 0.35 && in_by(l) < 0.3);
        let (_, l) = look(&mut app, 100.0);
        assert!(l.alpha > 0.5 && l.alpha < 0.75);
        assert!(in_by(l) > 0.5 && in_by(l) < 0.7);
        let (_, l) = look(&mut app, 150.0);
        assert!(l.alpha > 0.9 && in_by(l) > 0.8 && in_by(l) < 0.95);
        let (_, l) = look(&mut app, 200.0);
        assert!(in_by(l) > 0.98);
        assert!(
            (l.offset[1] - -8.0 * (1.0 - in_by(l))).abs() < 1e-4,
            "8 px up, {:?}",
            l.offset
        );
        assert_eq!(l.origin, [SETTINGS_WIDTH, 0.0]);
        let (_, l) = look(&mut app, 700.0);
        assert!(
            l.alpha == 1.0 && l.scale == 1.0 && l.offset == [0.0, 0.0],
            "in place by 643 ms"
        );
        for (i, reduced) in [false, true].into_iter().enumerate() {
            let base = 1000.0 + 10_000.0 * i as f64;
            app.reduced_motion = reduced;
            app.settings.open = true;
            let _ = look(&mut app, base);
            let _ = look(&mut app, base + 1000.0);
            app.settings.open = false;
            let ms: f64 = if reduced { 120.0 } else { 200.0 };
            let (presence, _) = look(&mut app, base + 2000.0);
            assert_eq!(presence, mv_motion::Presence::Closing);
            let (presence, _) = look(&mut app, base + 2000.0 + ms - 1.0);
            assert_eq!(presence, mv_motion::Presence::Closing);
            let (presence, _) = look(&mut app, base + 2000.0 + ms + 0.001);
            assert_eq!(presence, mv_motion::Presence::Closed);
        }
    }

    #[test]
    fn a_new_scheme_fades_the_lists_content_in() {
        let _guard = mv_clay::tests_lock();
        // The spec sets the Clay callback context here; the surface removed
        // the global, so there is nothing to set.
        let mut app = layout_app();
        // After the final placement: `init_clay` keeps `&mut app` for the
        // measure callback, so it must run after `layout_app`'s return move.
        crate::ui::init_clay(&mut app);
        let size = [1440.0, 900.0];
        app.motion.now = 1.0;
        let _ = lay_out_menu(&mut app, size);
        gear_activate(&mut app, true);
        let _ = lay_out_menu(&mut app, size);
        assert!(!app.settings.swapped);
        assert!(region_of_found(
            &mut app,
            Hit {
                kind: HitKind::SettingsTheme,
                index: 0
            }
        ));
        let focus = app.input.focus;
        assert!(settings_key(&mut app, focus, Keycode::Down));
        app.motion.now = 2.0;
        let _ = lay_out_menu(&mut app, size);
        app.system_scheme = mv_theme::Scheme::Light;
        app.motion.now = 3.0;
        let _ = lay_out_menu(&mut app, size);
        assert!(app.settings.swapped);
        assert_eq!(app.settings.swap.start, 3.0);
        assert_eq!(app.settings.swap.duration, 0.16);
        assert!((mv_motion::fade_in_at(app.settings.swap, 3.08) - 0.5).abs() < 1e-9);
        assert_eq!(
            app.input.focus,
            Hit {
                kind: HitKind::SettingsTheme,
                index: 0
            }
        );
        assert!(crate::motion_driver::motion_moving(&mut app));
        let focus = app.input.focus;
        assert!(settings_key(&mut app, focus, Keycode::Escape));
        gear_activate(&mut app, false);
        app.motion.now = 4.0;
        let _ = lay_out_menu(&mut app, size);
        assert!(!app.settings.swapped);
        app.reduced_motion = true;
        app.system_scheme = mv_theme::Scheme::Dark;
        app.motion.now = 5.0;
        let _ = lay_out_menu(&mut app, size);
        assert_eq!(app.settings.swap.duration, 0.12);
        release_layout_app(&mut app);
    }

    #[test]
    fn settings_panel_is_the_references() {
        struct Entry {
            name: String,
            id: clay::ElementId,
        }
        fn near(b: clay::BoundingBox, want: [f32; 4], slack: f32) -> bool {
            (b.x - want[0]).abs() <= slack
                && (b.y - want[1]).abs() <= slack
                && (b.width - want[2]).abs() <= slack
                && (b.height - want[3]).abs() <= slack
        }
        fn clay_words(
            app: &mut App,
            commands: &[clay::RenderCommand],
            panel: clay::BoundingBox,
            s: &str,
        ) -> (f32, f32, bool) {
            for c in commands {
                if c.commandType != clay::RenderCommandType::Text
                    || !crate::scroll::inside(panel, [c.boundingBox.x, c.boundingBox.y])
                {
                    continue;
                }
                // SAFETY: `c` comes from Clay's own laid-out command array;
                // the slice borrows its string for the comparison below.
                let data = unsafe { c.renderData.text };
                let bytes = unsafe {
                    std::slice::from_raw_parts(
                        data.stringContents.chars as *const u8,
                        data.stringContents.length as usize,
                    )
                };
                if std::str::from_utf8(bytes).unwrap_or("") != s {
                    continue;
                }
                let style = crate::typography::clay_type_style(app, data.fontId);
                let b = c.boundingBox;
                return (
                    b.x,
                    b.y + mv_text::baseline(
                        &mut app.fonts,
                        style.style.face,
                        style.style.size,
                        b.height,
                    ),
                    true,
                );
            }
            (0.0, 0.0, false)
        }
        fn add(texts: &mut Vec<Text>, name: &str, source: &str, x: f32, base: f32, ok: bool) {
            texts.push(Text {
                name: name.to_string(),
                source: source.to_string(),
                x,
                base,
                ok,
            });
        }
        struct Text {
            name: String,
            source: String,
            x: f32,
            base: f32,
            ok: bool,
        }

        let page = crate::fixture::page_fixture().expect("page fixture");
        let shot = crate::fixture::fixture_shot(&page, "open").unwrap();
        let _guard = mv_clay::tests_lock();
        // The spec sets the Clay callback context here; the surface removed
        // the global, so there is nothing to set.
        let mut app = layout_app();
        // After the final placement, as above.
        crate::ui::init_clay(&mut app);
        app.settings.open = true;
        app.motion.now = 1.0;
        let commands = lay_out_menu(&mut app, shot.size);
        let (wk, cr) = (&shot.webkit, &shot.chromium);

        let mut entries = Vec::new();
        entries.push(Entry {
            name: "settings_panel".to_string(),
            id: element_id("SettingsPanel", 0),
        });
        entries.push(Entry {
            name: "settings_appearance".to_string(),
            id: element_id("SettingsAppearance", 0),
        });
        entries.push(Entry {
            name: "settings_mode".to_string(),
            id: element_id("SettingsMode", 0),
        });
        for (mode, word) in mv_theme::MODE_WORDS.iter() {
            let i = mode.into_usize() as u32;
            entries.push(Entry {
                name: format!("settings_mode_{word}"),
                id: element_id("SettingsModeOption", i),
            });
            entries.push(Entry {
                name: format!("settings_mode_{word}_label"),
                id: element_id("SettingsModeLabel", i),
            });
            entries.push(Entry {
                name: format!("settings_mode_{word}_icon"),
                id: element_id("SettingsModeIcon", i),
            });
        }
        entries.push(Entry {
            name: "settings_help".to_string(),
            id: element_id("SettingsHelp", 0),
        });
        entries.push(Entry {
            name: "settings_theme_label".to_string(),
            id: element_id("SettingsThemeLabel", 0),
        });
        entries.push(Entry {
            name: "settings_themes".to_string(),
            id: element_id("SettingsThemes", 0),
        });
        for row in 0..3 {
            let i = row as u32;
            entries.push(Entry {
                name: format!("settings_row_{row}"),
                id: element_id("SettingsTheme", i),
            });
            entries.push(Entry {
                name: format!("settings_swatch_{row}"),
                id: element_id("SettingsSwatch", i),
            });
            entries.push(Entry {
                name: format!("settings_name_{row}"),
                id: element_id("SettingsThemeName", i),
            });
            entries.push(Entry {
                name: format!("settings_check_{row}"),
                id: element_id("SettingsCheck", i),
            });
        }
        let (mut checked, mut passed) = (0, 0);
        for b in &entries {
            let want_wk = match crate::fixture::fixture_box(wk, &b.name) {
                Ok(found) => found,
                Err(e) => panic!("{e}"),
            };
            let want_cr = cr.boxes.get(b.name.as_str()).copied().unwrap_or([0.0; 4]);
            let data = element_data(b.id);
            checked += 1;
            if !data.found {
                panic!("no {} in the port", b.name);
            }
            let got = data.boundingBox;
            let good = near(got, want_wk, SLACK) || near(got, want_cr, SLACK);
            if good {
                passed += 1;
            }
            assert!(
                good,
                "{} at ({:.3}, {:.3}, {:.3} x {:.3}); WebKit's ({:.3}, {:.3}, {:.3} x {:.3})",
                b.name,
                got.x,
                got.y,
                got.width,
                got.height,
                want_wk[0],
                want_wk[1],
                want_wk[2],
                want_wk[3],
            );
        }

        let mut texts = Vec::new();
        {
            let panel = element_data(element_id("SettingsPanel", 0)).boundingBox;
            let (x, base, drawn) = clay_words(&mut app, &commands, panel, "APPEARANCE");
            add(
                &mut texts,
                "settings_appearance_text",
                SETTINGS_APPEARANCE,
                x,
                base,
                drawn,
            );
            let help = settings_help(mv_theme::Mode::System, mv_theme::Scheme::Dark);
            let (x, base, drawn) = clay_words(&mut app, &commands, panel, &help);
            add(&mut texts, "settings_help_text", &help, x, base, drawn);
            let (x, base, drawn) = clay_words(&mut app, &commands, panel, "DARK THEME");
            add(
                &mut texts,
                "settings_theme_label_text",
                THEME_LABELS[mv_theme::Scheme::Dark],
                x,
                base,
                drawn,
            );
            for (row, slug) in mv_theme::SCHEME_SLUGS[mv_theme::Scheme::Dark]
                .into_iter()
                .enumerate()
            {
                let name = mv_theme::SLUG_LABELS[slug];
                let (x, base, drawn) = clay_words(&mut app, &commands, panel, name);
                add(
                    &mut texts,
                    &format!("settings_name_{row}"),
                    name,
                    x,
                    base,
                    drawn,
                );
            }
            let style = crate::typography::type_style(
                crate::typography::Type::SettingsOption,
                crate::ui::current_theme(&app),
            );
            for (mode, word) in MODE_LABELS.iter() {
                let data = element_data(element_id("SettingsModeText", mode.into_usize() as u32));
                let b = data.boundingBox;
                let on = b.y
                    + mv_text::baseline(
                        &mut app.fonts,
                        style.style.face,
                        style.style.size,
                        style.line_height,
                    );
                add(
                    &mut texts,
                    &format!("settings_mode_{}_text", mv_theme::MODE_WORDS[mode]),
                    word,
                    b.x,
                    on,
                    data.found,
                );
            }
        }
        for c in &texts {
            let Some(want) = wk.texts.get(c.name.as_str()) else {
                panic!("testdata/page.json has no text {:?}", c.name);
            };
            if !c.ok {
                panic!("{} isn't drawn in the port", c.name);
            }
            assert_eq!(
                c.source, want.text,
                "{} reads {:?}; the reference's {:?}",
                c.name, c.source, want.text,
            );
            checked += 1;
            let good = cr.texts.get(c.name.as_str()).is_some_and(|want_cr| {
                (c.x - want.x).abs() <= SLACK || (c.x - want_cr.x).abs() <= SLACK
            }) && (c.base - want.baseline).abs() <= BASELINE_SLACK;
            if good {
                passed += 1;
            }
            assert!(
                good,
                "{} at {:.3} on {:.3}; WebKit's at {:.3} on {:.3}, Chromium's at {:.3}",
                c.name,
                c.x,
                c.base,
                want.x,
                want.baseline,
                cr.texts.get(c.name.as_str()).map_or(0.0, |t| t.x),
            );
        }
        eprintln!(
            "the settings panel at {} x {}: {} of {} boxes and texts at the reference's",
            shot.size[0], shot.size[1], passed, checked,
        );
        release_layout_app(&mut app);
    }
}
