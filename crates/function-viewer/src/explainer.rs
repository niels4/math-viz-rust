//! The letter-chip explainer card: hover, rest and click open it; it places
//! itself from the chip, panel and card boxes after layout.
//!
//! The card shows the head, the sentence, a mini-plot, the snippet, the rule
//! and the gestures under a callout caret. (Declares against the plane, text
//! and viewer surfaces; wired when they land.)

use enum_map::Enum as _;
use mv_sys::clay;

use crate::app::App;
use crate::input::{Cursor, Hit, HitKind};
use crate::motion_driver::OverlayFrom;

/// The explainer's stacking layer, padding and part gap.
pub const EXPLAINER_Z: i16 = 30;
pub const EXPLAINER_PADDING: f32 = 16.0;
pub const EXPLAINER_PART_GAP: f32 = 10.0;

/// The head row's height, the chip's size and radius, and the where-acts math.
pub const EXPLAINER_HEAD: f32 = 26.0;
pub const EXPLAINER_CHIP: f32 = 26.0;
pub const EXPLAINER_CHIP_RADIUS: f32 = 6.0;
pub const WHERE_GAP: f32 = 4.0;
pub const WHERE_MATH_SIZE: f32 = 15.0;
pub const WHERE_MATH_LINE: f32 = 19.0;

/// The snippet row's height, gap, size and term bottom.
pub const SNIPPET_ROW: f32 = 28.0;
pub const SNIPPET_GAP: f32 = 10.0;
pub const SNIPPET_SIZE: f32 = 20.0;
pub const SNIPPET_TERM_BOTTOM: f32 = 3.0;

/// The gestures rows' gap.
pub const GESTURES_ROW_GAP: f32 = 2.0;

/// The example plate's inset, padding, radius, gap, size and line.
pub const EXAMPLE_INSET: f32 = 8.0;
pub const EXAMPLE_PADDING: [f32; 2] = [6.0, 2.0];
pub const EXAMPLE_RADIUS: f32 = 4.0;
pub const EXAMPLE_GAP: f32 = 4.0;
pub const EXAMPLE_SIZE: f32 = 15.0;
pub const EXAMPLE_LINE: f32 = 19.0;

/// The callout origin's scale and distance.
pub const CALLOUT_FROM_SCALE: f32 = 0.9;
pub const CALLOUT_FROM_DISTANCE: f32 = 80.0;

/// The explainer as the page shows it.
#[derive(Clone, Copy, Debug)]
pub struct ExplainerView {
    pub shown: Option<mv_viewer::Param>,
    pub placed: Option<PlacedExplainer>,
    pub resting: Option<mv_viewer::Param>,
}

/// The placed explainer: where, how tall, how big, and the scroll then.
#[derive(Clone, Copy, Debug)]
pub struct PlacedExplainer {
    pub place: mv_viewer::ExplainerPlace,
    pub height: f32,
    pub size: [f32; 2],
    pub scroll: enum_map::EnumMap<crate::scroll::Scroller, f32>,
}

/// Opens the Clay element `id` (or an anonymous one), configures it, runs
/// the children and closes it: the `UI(id)(config) { ... }` scope.
///
/// Ported from `explainer.odin`'s `UI` scopes (`clay.odin:527-536`): through
/// `mv-clay`'s free functions on Clay's current context (which the layout
/// left open).
///
/// `settings.rs` carries its own copy until `mv-clay` exposes element
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
/// Ported from `explainer.odin`'s `ID` uses (`clay.odin:595`).
fn element_id(label: &str, index: u32) -> clay::ElementId {
    mv_clay::id(label, index)
}

/// This layout's data for the element with `id`.
///
/// Ported from `explainer.odin`'s layout reads (`clay.odin:493`
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

/// The zero overlay: the `{}` an overlay reset to is.
fn overlay_zero() -> mv_motion::Overlay {
    let transition = mv_motion::Transition {
        from: 0.0,
        to: 0.0,
        start: 0.0,
        duration: 0.0,
        ease: mv_motion::Ease::Linear,
        reversing_from: 0.0,
        shortening: 0.0,
    };
    mv_motion::Overlay {
        open: false,
        shown: false,
        since: 0.0,
        exit: 0.0,
        alpha: transition,
        shape: transition,
    }
}

/// Closes the hover-opened explainer on chip leave; starts the rest timer on
/// enter.
pub fn chip_hover(app: &mut App, from: Hit, to: Hit) {
    if from == to {
        return;
    }
    if from.kind == HitKind::Chip {
        crate::app_loop::timer_stop(app, crate::app_loop::Timer::ChipRest);
        app.explainer.resting = None;
        crate::parts::dispatch(
            app,
            mv_viewer::Action::Explain(mv_viewer::Explain {
                param: mv_viewer::Param::from_usize(from.index as usize),
                by: mv_viewer::ExplainBy::Hover,
                open: mv_viewer::ExplainOpen::Close,
            }),
        );
    }
    if to.kind == HitKind::Chip {
        app.explainer.resting = Some(mv_viewer::Param::from_usize(to.index as usize));
        crate::app_loop::timer_start(
            app,
            crate::app_loop::Timer::ChipRest,
            mv_viewer::EXPLAINER_REST_MS,
        );
    }
}

/// Opens the hover explainer when still resting on the chip.
pub fn chip_rested(app: &mut App) {
    if let Some(p) = app.explainer.resting {
        app.explainer.resting = None;
        crate::parts::dispatch(
            app,
            mv_viewer::Action::Explain(mv_viewer::Explain {
                param: p,
                by: mv_viewer::ExplainBy::Hover,
                open: mv_viewer::ExplainOpen::Open,
            }),
        );
    }
}

/// Stops the rest timer and opens the hover explainer at once.
pub fn chip_click(app: &mut App, p: mv_viewer::Param) {
    crate::app_loop::timer_stop(app, crate::app_loop::Timer::ChipRest);
    app.explainer.resting = None;
    crate::parts::dispatch(
        app,
        mv_viewer::Action::Explain(mv_viewer::Explain {
            param: p,
            by: mv_viewer::ExplainBy::Hover,
            open: mv_viewer::ExplainOpen::Open,
        }),
    );
}

/// Toggles the row explainer on `?`, closes the open one on Esc.
pub fn explainer_key(
    app: &mut App,
    key: sdl3::keyboard::Keycode,
    mods: sdl3::keyboard::Mod,
) -> bool {
    if key == sdl3::keyboard::Keycode::Question
        || (key == sdl3::keyboard::Keycode::Slash
            && mods.intersects(sdl3::keyboard::Mod::LSHIFTMOD | sdl3::keyboard::Mod::RSHIFTMOD))
    {
        let focus = app.input.focus;
        if focus.kind != HitKind::Box && focus.kind != HitKind::Flip {
            return false;
        }
        let p = mv_viewer::Param::from_usize(focus.index as usize);
        if crate::number_field::field_open_on(app, p) {
            return false;
        }
        crate::parts::dispatch(
            app,
            mv_viewer::Action::Explain(mv_viewer::Explain {
                param: p,
                by: mv_viewer::ExplainBy::Key,
                open: mv_viewer::ExplainOpen::Toggle,
            }),
        );
        return true;
    }
    if key == sdl3::keyboard::Keycode::Escape {
        if app.viewer.explainer.is_none() {
            return false;
        }
        crate::parts::dispatch(app, mv_viewer::Action::Escape(mv_viewer::Escape));
        return true;
    }
    false
}

/// Declares the floating overlay card with head, sentence, plot, snippet,
/// rule, gestures and caret.
pub fn explainer(app: &mut App, t: &mv_theme::Theme) {
    if let Some(model) = app.viewer.explainer
        && app.explainer.shown.is_none_or(|p| p != model.param)
    {
        app.explainer.shown = Some(model.param);
        app.explainer.placed = None;
        app.motion.overlays[crate::motion_driver::OverlayKind::Explainer] = overlay_zero();
    }
    let Some(p) = app.explainer.shown else {
        return;
    };
    let placed = app.explainer.placed;
    let has_place = placed.is_some();
    let place = placed.map_or(
        mv_viewer::ExplainerPlace {
            x: 0.0,
            y: 0.0,
            caret: mv_viewer::Caret {
                side: mv_viewer::CaretSide::Left,
                at: mv_viewer::EXPLAINER_TIP,
            },
        },
        |prev| prev.place,
    );
    let open = app.viewer.explainer.is_some();
    let height = placed.map_or(0.0, |prev| prev.height);
    let (presence, mut layer) = crate::motion_driver::overlay_frame(
        app,
        crate::motion_driver::OverlayKind::Explainer,
        open,
        callout_from(place.caret, height),
    );
    if presence == mv_motion::Presence::Closed {
        app.explainer.shown = None;
        app.explainer.placed = None;
        return;
    }
    if !has_place {
        layer.alpha = 0.0;
    }
    let (overlay, layered) = crate::effects::layer_config(
        layer,
        crate::effects::Effect {
            shadow: None,
            inset: 0.0,
            inset_ink: [0.0, 0.0, 0.0, 0.0],
            layer: None,
        },
    );
    let card_id = element_id("ExplainerCard", 0);
    ui(
        Some(element_id("Explainer", 0)),
        &clay::ElementDeclaration {
            layout: clay::LayoutConfig {
                sizing: clay::Sizing {
                    width: mv_clay::sizing_fit(clay::SizingConstraintsMinMax {
                        min: 0.0,
                        max: 0.0,
                    }),
                    height: mv_clay::sizing_fit(clay::SizingConstraintsMinMax {
                        min: 0.0,
                        max: 0.0,
                    }),
                },
                ..layout_zero()
            },
            floating: clay::FloatingElementConfig {
                offset: [place.x as f32, place.y as f32],
                expand: clay::Dimensions {
                    width: 0.0,
                    height: 0.0,
                },
                parentId: 0,
                zIndex: EXPLAINER_Z,
                attachment: clay::FloatingAttachPoints {
                    element: clay::FloatingAttachPointType::LeftTop,
                    parent: clay::FloatingAttachPointType::LeftTop,
                },
                pointerCaptureMode: clay::PointerCaptureMode::Capture,
                attachTo: clay::FloatingAttachToElement::Root,
                clipTo: clay::FloatingClipToElement::None,
            },
            overlayColor: overlay,
            userData: layered,
            ..decl_zero()
        },
        || {
            ui(
                Some(card_id),
                &clay::ElementDeclaration {
                    layout: clay::LayoutConfig {
                        sizing: clay::Sizing {
                            width: mv_clay::sizing_fixed(mv_viewer::EXPLAINER_WIDTH as f32),
                            height: mv_clay::sizing_fit(clay::SizingConstraintsMinMax {
                                min: 0.0,
                                max: 0.0,
                            }),
                        },
                        padding: mv_clay::padding_all((EXPLAINER_PADDING + 1.0) as u16),
                        childGap: EXPLAINER_PART_GAP as u16,
                        layoutDirection: clay::LayoutDirection::TopToBottom,
                        ..layout_zero()
                    },
                    backgroundColor: crate::ui::ink(t, mv_theme::Token::Card),
                    border: clay::BorderElementConfig {
                        color: crate::ui::ink(t, mv_theme::Token::Border),
                        width: mv_clay::border_outside(1),
                    },
                    cornerRadius: mv_clay::corner_radius_all(t.card_radius),
                    userData: crate::effects::effect(crate::effects::Effect {
                        shadow: t.card_shadow,
                        inset: 0.0,
                        inset_ink: [0.0, 0.0, 0.0, 0.0],
                        layer: None,
                    }),
                    ..decl_zero()
                },
                || {
                    let content =
                        mv_viewer::EXPLAINER_WIDTH as f32 - 2.0 * (EXPLAINER_PADDING + 1.0);
                    explainer_head(app, t, p);
                    let sentence = crate::typography::type_style(
                        crate::typography::Type::ExplainerSentence,
                        t,
                    );
                    ui(
                        Some(element_id("ExplainerSentence", 0)),
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
                            for line in mv_text::break_lines(
                                &mut app.fonts,
                                sentence.style,
                                mv_viewer::EXPLAINER_SAY[p],
                                content,
                            ) {
                                crate::typography::label(
                                    app,
                                    crate::typography::Type::ExplainerSentence,
                                    &line,
                                    mv_theme::Token::Foreground,
                                );
                            }
                        },
                    );
                    let size = mv_viewer::EXPLAINER_PLOT_SIZE;
                    ui(
                        Some(element_id("ExplainerPlot", 0)),
                        &clay::ElementDeclaration {
                            layout: clay::LayoutConfig {
                                sizing: clay::Sizing {
                                    width: mv_clay::sizing_fixed(size[0] as f32),
                                    height: mv_clay::sizing_fixed(size[1] as f32),
                                },
                                ..layout_zero()
                            },
                            custom: crate::widgets::custom_data(
                                crate::widgets::Custom::ExplainerPlot(ExplainerPlot {
                                    param: p,
                                    radius: t.card_radius,
                                }),
                            ),
                            ..decl_zero()
                        },
                        || {},
                    );
                    explainer_snippet(app, t, p);
                    ui(
                        Some(element_id("ExplainerRule", 0)),
                        &clay::ElementDeclaration {
                            layout: clay::LayoutConfig {
                                sizing: clay::Sizing {
                                    width: mv_clay::sizing_grow(clay::SizingConstraintsMinMax {
                                        min: 0.0,
                                        max: 0.0,
                                    }),
                                    height: mv_clay::sizing_fixed(1.0),
                                },
                                ..layout_zero()
                            },
                            backgroundColor: crate::ui::ink(t, mv_theme::Token::Border),
                            ..decl_zero()
                        },
                        || {},
                    );
                    let gestures = mv_viewer::explainer_gestures(p);
                    let mut items = crate::tip::tip_items(
                        app,
                        &gestures,
                        content,
                        t.colors[mv_theme::Token::ForegroundMuted],
                        GESTURES_ROW_GAP,
                    );
                    items.selectable = true;
                    ui(
                        Some(element_id("ExplainerGestures", 0)),
                        &clay::ElementDeclaration {
                            layout: clay::LayoutConfig {
                                sizing: clay::Sizing {
                                    width: mv_clay::sizing_grow(clay::SizingConstraintsMinMax {
                                        min: 0.0,
                                        max: 0.0,
                                    }),
                                    height: mv_clay::sizing_fixed(items.height),
                                },
                                ..layout_zero()
                            },
                            custom: crate::widgets::custom_data(crate::widgets::Custom::TipItems(
                                items,
                            )),
                            ..decl_zero()
                        },
                        || {},
                    );
                },
            );
            ui(
                Some(element_id("ExplainerCaret", 0)),
                &clay::ElementDeclaration {
                    custom: crate::widgets::custom_data(crate::widgets::Custom::CalloutCaret(
                        CalloutCaret {
                            card: [
                                place.x as f32,
                                place.y as f32,
                                mv_viewer::EXPLAINER_WIDTH as f32,
                                height,
                            ],
                            caret: place.caret,
                            fill: t.colors[mv_theme::Token::Card],
                            edge: t.colors[mv_theme::Token::Border],
                        },
                    )),
                    ..decl_zero()
                },
                || {},
            );
        },
    );
    if has_place && presence == mv_motion::Presence::Open {
        let z = app.input.z;
        app.input.z = EXPLAINER_Z;
        crate::input::region(
            app,
            card_id,
            Hit {
                kind: HitKind::Explainer,
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
        app.input.z = z;
    }
}

/// Maps the caret side to the scale origin out from the card.
pub fn callout_from(caret: mv_viewer::Caret, height: f32) -> OverlayFrom {
    let at = caret.at as f32;
    let origin = match caret.side {
        mv_viewer::CaretSide::Left => [-CALLOUT_FROM_DISTANCE, at],
        mv_viewer::CaretSide::Top => [at, -CALLOUT_FROM_DISTANCE],
        mv_viewer::CaretSide::Bottom => [at, height + CALLOUT_FROM_DISTANCE],
    };
    OverlayFrom {
        scale: CALLOUT_FROM_SCALE,
        origin,
        offset: [0.0, 0.0],
    }
}

/// Lays out the chip plus name plus where-acts math.
fn explainer_head(app: &mut App, t: &mv_theme::Theme, p: mv_viewer::Param) {
    ui(
        Some(element_id("ExplainerHead", 0)),
        &clay::ElementDeclaration {
            layout: clay::LayoutConfig {
                sizing: clay::Sizing {
                    width: mv_clay::sizing_grow(clay::SizingConstraintsMinMax {
                        min: 0.0,
                        max: 0.0,
                    }),
                    height: mv_clay::sizing_fixed(EXPLAINER_HEAD),
                },
                childGap: 10,
                childAlignment: clay::ChildAlignment {
                    x: clay::LayoutAlignmentX::Left,
                    y: clay::LayoutAlignmentY::Center,
                },
                ..layout_zero()
            },
            ..decl_zero()
        },
        || {
            ui(
                Some(element_id("ExplainerChip", 0)),
                &clay::ElementDeclaration {
                    layout: clay::LayoutConfig {
                        sizing: clay::Sizing {
                            width: mv_clay::sizing_fixed(EXPLAINER_CHIP),
                            height: mv_clay::sizing_fixed(EXPLAINER_CHIP),
                        },
                        childAlignment: clay::ChildAlignment {
                            x: clay::LayoutAlignmentX::Center,
                            y: clay::LayoutAlignmentY::Center,
                        },
                        ..layout_zero()
                    },
                    backgroundColor: crate::ui::ink(t, crate::sidebar::PARAM_INKS[p]),
                    cornerRadius: mv_clay::corner_radius_all(EXPLAINER_CHIP_RADIUS),
                    ..decl_zero()
                },
                || {
                    crate::typography::label(
                        app,
                        crate::typography::Type::ChipLetter,
                        mv_viewer::PARAM_LETTERS[p],
                        mv_theme::Token::Background,
                    );
                },
            );
            ui(
                Some(element_id("ExplainerName", 0)),
                &clay::ElementDeclaration {
                    layout: clay::LayoutConfig {
                        sizing: clay::Sizing {
                            width: mv_clay::sizing_grow(clay::SizingConstraintsMinMax {
                                min: 0.0,
                                max: 0.0,
                            }),
                            height: axis_zero(),
                        },
                        childAlignment: clay::ChildAlignment {
                            x: clay::LayoutAlignmentX::Left,
                            y: clay::LayoutAlignmentY::Center,
                        },
                        ..layout_zero()
                    },
                    ..decl_zero()
                },
                || {
                    crate::typography::label(
                        app,
                        crate::typography::Type::ExplainerName,
                        mv_viewer::PARAM_NAMES[p],
                        mv_theme::Token::Foreground,
                    );
                },
            );
            ui(
                Some(element_id("ExplainerWhere", 0)),
                &clay::ElementDeclaration {
                    layout: clay::LayoutConfig {
                        sizing: clay::Sizing {
                            width: axis_zero(),
                            height: axis_zero(),
                        },
                        childGap: WHERE_GAP as u16,
                        childAlignment: clay::ChildAlignment {
                            x: clay::LayoutAlignmentX::Left,
                            y: clay::LayoutAlignmentY::Center,
                        },
                        ..layout_zero()
                    },
                    ..decl_zero()
                },
                || {
                    crate::typography::label(
                        app,
                        crate::typography::Type::ExplainerWhere,
                        &mv_viewer::where_of(p),
                        mv_theme::Token::ForegroundMuted,
                    );
                    let line = crate::widgets::math_line(
                        &mut app.fonts,
                        mv_viewer::GROUP_MATH,
                        WHERE_MATH_SIZE,
                        WHERE_MATH_LINE,
                        t.colors[mv_theme::Token::ForegroundMuted],
                    );
                    ui(
                        Some(element_id("ExplainerWhereMath", 0)),
                        &clay::ElementDeclaration {
                            layout: clay::LayoutConfig {
                                sizing: clay::Sizing {
                                    width: mv_clay::sizing_fixed(line.width),
                                    height: mv_clay::sizing_fixed(line.height),
                                },
                                ..layout_zero()
                            },
                            custom: crate::widgets::custom_data(crate::widgets::Custom::Typeset(
                                crate::widgets::Typeset {
                                    line,
                                    selectable: true,
                                },
                            )),
                            ..decl_zero()
                        },
                        || {},
                    );
                },
            );
        },
    );
}

/// Lays out the caps label plus the lit-letter form snippet line.
fn explainer_snippet(app: &mut App, t: &mv_theme::Theme, p: mv_viewer::Param) {
    ui(
        Some(element_id("ExplainerSnippetRow", 0)),
        &clay::ElementDeclaration {
            layout: clay::LayoutConfig {
                sizing: clay::Sizing {
                    width: mv_clay::sizing_grow(clay::SizingConstraintsMinMax {
                        min: 0.0,
                        max: 0.0,
                    }),
                    height: mv_clay::sizing_fixed(SNIPPET_ROW),
                },
                childGap: SNIPPET_GAP as u16,
                childAlignment: clay::ChildAlignment {
                    x: clay::LayoutAlignmentX::Left,
                    y: clay::LayoutAlignmentY::Center,
                },
                ..layout_zero()
            },
            ..decl_zero()
        },
        || {
            let style = crate::typography::type_style(crate::typography::Type::SnippetLabel, t);
            let width = crate::typography::type_width(
                app,
                crate::typography::Type::SnippetLabel,
                mv_viewer::IN_THE_EQUATION,
            ) - style.style.tracking;
            ui(
                Some(element_id("ExplainerSnippetLabel", 0)),
                &clay::ElementDeclaration {
                    layout: clay::LayoutConfig {
                        sizing: clay::Sizing {
                            width: mv_clay::sizing_fixed(width),
                            height: mv_clay::sizing_fixed(style.line_height),
                        },
                        ..layout_zero()
                    },
                    ..decl_zero()
                },
                || {
                    crate::typography::label(
                        app,
                        crate::typography::Type::SnippetLabel,
                        mv_viewer::IN_THE_EQUATION,
                        mv_theme::Token::ForegroundMuted,
                    );
                },
            );
            let tokens = mv_viewer::snippet_tokens(p, mv_viewer::EXPLAINER_SNIPPETS[p]);
            let line = crate::equation::set_line(
                &mut app.fonts,
                t,
                &tokens,
                SNIPPET_SIZE,
                t.colors[mv_theme::Token::ForegroundMuted],
                crate::equation::LineStyle {
                    term_ink: Some(t.colors[mv_theme::Token::Foreground]),
                    term_bottom: SNIPPET_TERM_BOTTOM,
                    whole_px: false,
                },
            );
            ui(
                Some(element_id("ExplainerSnippet", 0)),
                &clay::ElementDeclaration {
                    layout: clay::LayoutConfig {
                        sizing: clay::Sizing {
                            width: mv_clay::sizing_fixed(line.width),
                            height: mv_clay::sizing_fixed(line.height),
                        },
                        ..layout_zero()
                    },
                    custom: crate::widgets::custom_data(crate::widgets::Custom::ExplainerSnippet(
                        ExplainerSnippet { line, param: p },
                    )),
                    ..decl_zero()
                },
                || {},
            );
        },
    );
}

/// Places the explainer from the chip/panel/card boxes on size/scroll change.
pub fn explainer_after_layout(app: &mut App) -> bool {
    let Some(p) = app.explainer.shown else {
        return false;
    };
    let Some(model) = app.viewer.explainer else {
        return false;
    };
    if model.param != p {
        return false;
    }
    let mut scroll = enum_map::EnumMap::<crate::scroll::Scroller, f32>::from_array([0.0, 0.0]);
    for (s, slot) in scroll.iter_mut() {
        *slot = app.scroll[s].offset;
    }
    let placed = app.explainer.placed;
    if let Some(prev) = placed
        && prev.size == app.size
        && prev.scroll == scroll
    {
        return false;
    }
    let card = element_data(element_id("ExplainerCard", 0));
    let chip = element_data(element_id("Chip", p.into_usize() as u32));
    let panel = element_data(element_id(
        crate::scroll::SCROLLER_IDS[crate::scroll::Scroller::Panel],
        0,
    ));
    if !card.found || !chip.found || !panel.found {
        return false;
    }
    let rect = |b: clay::BoundingBox| mv_plane::Rect {
        x: f64::from(b.x),
        y: f64::from(b.y),
        w: f64::from(b.width),
        h: f64::from(b.height),
    };
    let height = card.boundingBox.height;
    let view = [f64::from(app.size[0]), f64::from(app.size[1])];
    let place = mv_viewer::place_explainer(
        rect(chip.boundingBox),
        rect(panel.boundingBox),
        f64::from(height),
        view,
    );
    app.explainer.placed = Some(PlacedExplainer {
        place,
        height,
        size: app.size,
        scroll,
    });
    if let Some(prev) = placed
        && prev.place == place
        && prev.height == height
    {
        return false;
    }
    app.dirty = true;
    true
}

/// The callout caret: the card's box in the window and its fill and edge.
#[derive(Clone, Copy, Debug)]
pub struct CalloutCaret {
    /// The card's box in the window: x, y, width, height.
    pub card: [f32; 4],
    pub caret: mv_viewer::Caret,
    pub fill: mv_theme::Color,
    pub edge: mv_theme::Color,
}

/// The triangle points on the card's edge.
pub fn caret_points(card: [f32; 4], caret: mv_viewer::Caret) -> [[f32; 2]; 3] {
    let (x, y, height) = (card[0], card[1], card[3]);
    let at = caret.at as f32;
    match caret.side {
        mv_viewer::CaretSide::Left => [[x, y + at - 8.0], [x - 8.0, y + at], [x, y + at + 8.0]],
        mv_viewer::CaretSide::Top => [[x + at - 8.0, y], [x + at, y - 8.0], [x + at + 8.0, y]],
        mv_viewer::CaretSide::Bottom => [
            [x + at - 8.0, y + height],
            [x + at, y + height + 8.0],
            [x + at + 8.0, y + height],
        ],
    }
}

/// Fills the caret triangle, then strokes its two outer edges round-joined.
pub fn draw_callout_caret(vg: &mut mv_nvg::Context, c: CalloutCaret) {
    let points = caret_points(c.card, c.caret);
    vg.begin_path();
    vg.move_to(points[0][0], points[0][1]);
    vg.line_to(points[1][0], points[1][1]);
    vg.line_to(points[2][0], points[2][1]);
    vg.close_path();
    vg.fill_color(c.fill);
    vg.fill();
    vg.save();
    vg.begin_path();
    vg.move_to(points[0][0], points[0][1]);
    vg.line_to(points[1][0], points[1][1]);
    vg.line_to(points[2][0], points[2][1]);
    vg.stroke_color(c.edge);
    vg.stroke_width(1.0);
    vg.line_cap(mv_sys::nvg::LineCapType::BUTT);
    vg.line_join(mv_sys::nvg::LineCapType::ROUND);
    vg.stroke();
    vg.restore();
}

/// The explainer's mini-plot: whose, and how large.
#[derive(Clone, Copy, Debug)]
pub struct ExplainerPlot {
    pub param: mv_viewer::Param,
    pub radius: f32,
}

/// Paints the mini-plane plot clipped to the frame, then the example plate
/// plus border.
pub fn draw_explainer_plot(app: &mut App, r#box: clay::BoundingBox, v: ExplainerPlot) {
    let t = crate::ui::current_theme(app).clone();
    let ratio = crate::plane_area::device_ratio(app);
    let radius = mv_clay::corner_radius_all(v.radius);
    let vg = &mut app.vg;
    vg.begin_path();
    crate::draw::rounded_rect(&mut *vg, r#box, radius);
    vg.fill_color(t.colors[mv_theme::Token::Background]);
    vg.fill();

    let snap = |v: f32, ratio: f64| ((f64::from(v) * ratio).round() / ratio) as f32;
    vg.save();
    vg.intersect_scissor(r#box.x, r#box.y, r#box.width, r#box.height);
    vg.translate(snap(r#box.x, ratio), snap(r#box.y, ratio));
    let size = mv_viewer::EXPLAINER_PLOT_SIZE;
    mv_plane::draw_mini(
        mv_plane::Painter {
            vg: &mut *vg,
            fonts: &mut app.fonts,
            theme: &t,
            density: ratio,
        },
        &mv_viewer::explainer_plot(v.param),
        size[0],
        size[1],
    );
    vg.restore();

    draw_example_plate(app, r#box, v.param, &t);
    crate::plane_area::clip_corners(
        &mut app.vg,
        r#box,
        crate::draw::css_radii(r#box, radius),
        t.colors[mv_theme::Token::Card],
    );
    crate::effects::draw_inset(
        &mut app.vg,
        r#box,
        radius,
        1.0,
        t.colors[mv_theme::Token::Border],
    );
}

/// The example's value plate, 8 px in from the frame's top right: its letter
/// in maths italic, then "= 2".
fn draw_example_plate(
    app: &mut App,
    frame: clay::BoundingBox,
    p: mv_viewer::Param,
    t: &mv_theme::Theme,
) {
    let words = t.colors[mv_theme::Token::Foreground];
    let letter = crate::widgets::math_line(
        &mut app.fonts,
        mv_viewer::PARAM_LETTERS[p],
        EXAMPLE_SIZE,
        EXAMPLE_LINE,
        words,
    );
    let value = crate::widgets::math_line(
        &mut app.fonts,
        &format!("= {}", mv_viewer::EXPLAINER_EXAMPLE_VALUE),
        EXAMPLE_SIZE,
        EXAMPLE_LINE,
        words,
    );
    let width = 2.0 * (EXAMPLE_PADDING[0] + 1.0) + letter.width + EXAMPLE_GAP + value.width;
    let height = 2.0 * (EXAMPLE_PADDING[1] + 1.0) + EXAMPLE_LINE;
    let plate = clay::BoundingBox {
        x: frame.x + frame.width - EXAMPLE_INSET - width,
        y: frame.y + EXAMPLE_INSET,
        width,
        height,
    };
    let radius = mv_clay::corner_radius_all(EXAMPLE_RADIUS);
    let vg = &mut app.vg;
    vg.begin_path();
    crate::draw::rounded_rect(&mut *vg, plate, radius);
    vg.fill_color(t.colors[mv_theme::Token::Card]);
    vg.fill();
    crate::effects::draw_inset(
        &mut *vg,
        plate,
        radius,
        1.0,
        t.colors[mv_theme::Token::Border],
    );
    let (x, y) = (
        plate.x + 1.0 + EXAMPLE_PADDING[0],
        plate.y + 1.0 + EXAMPLE_PADDING[1],
    );
    mv_text::draw_line(&mut app.fonts, &mut *vg, &letter, x, y);
    mv_text::draw_line(
        &mut app.fonts,
        &mut *vg,
        &value,
        x + letter.width + EXAMPLE_GAP,
        y,
    );
    crate::widgets::note_line(app, &letter, x, y);
    crate::widgets::note_line(app, &value, x + letter.width + EXAMPLE_GAP, y);
}

/// The explainer's snippet: whose, and its line.
#[derive(Clone, Debug)]
pub struct ExplainerSnippet {
    pub line: super::equation::EqLine,
    pub param: mv_viewer::Param,
}

/// Draws the lit term plates, then the snippet runs.
pub fn draw_explainer_snippet(app: &mut App, r#box: clay::BoundingBox, v: &ExplainerSnippet) {
    let token = crate::sidebar::PARAM_INKS[v.param];
    let (tint, ink) = {
        let t = crate::ui::current_theme(app);
        (mv_theme::plate(t, token), t.colors[token])
    };
    for term in &v.line.terms {
        let b = [r#box.x + term.x, r#box.y + term.y, term.width, term.height];
        crate::equation::draw_term_plate(
            &mut app.vg,
            b,
            tint,
            ink,
            [0.0, 0.0, 0.0, 0.0],
            mv_viewer::EquationMode::Form,
        );
    }
    for r in &v.line.runs {
        mv_text::draw_shaped(
            &mut app.fonts,
            &mut app.vg,
            r.size,
            r#box.x + r.x,
            r#box.y + r.baseline,
            &r.shaped,
            r.ink,
        );
        crate::equation::note_run(app, r, r#box.x, r#box.y);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_loop::Timer;
    use mv_viewer::{ExplainBy, Explainer, Param};
    use sdl3::keyboard::{Keycode, Mod};

    const SLACK: f32 = 0.02;
    const TEXT_SLACK: f32 = 0.1;
    const TRACKING: f32 = 2.8;

    /// A blank app: the headless constructor, as the Odin tests' `new(App)`.
    fn blank_app() -> App {
        crate::fixture::headless_app()
    }

    fn zero_scroll() -> crate::scroll::Scroll {
        crate::scroll::Scroll {
            active: false,
            offset: 0.0,
            max: 0.0,
            r#box: clay::BoundingBox {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
            },
            border: 0.0,
        }
    }

    struct Part {
        name: &'static str,
        id: clay::ElementId,
        webkit: [f32; 4],
        chromium: [f32; 4],
        slack: f32,
    }

    fn parts() -> Vec<Part> {
        vec![
            Part {
                name: "callout",
                id: element_id("ExplainerCard", 0),
                webkit: [489.0, 404.5, 300.0, 389.0],
                chromium: [489.0, 404.5, 300.0, 389.0],
                slack: SLACK,
            },
            Part {
                name: "head",
                id: element_id("ExplainerHead", 0),
                webkit: [506.0, 421.5, 266.0, 26.0],
                chromium: [506.0, 421.5, 266.0, 26.0],
                slack: SLACK,
            },
            Part {
                name: "chip",
                id: element_id("ExplainerChip", 0),
                webkit: [506.0, 421.5, 26.0, 26.0],
                chromium: [506.0, 421.5, 26.0, 26.0],
                slack: SLACK,
            },
            Part {
                name: "name",
                id: element_id("ExplainerName", 0),
                webkit: [542.0, 425.0, 150.578, 19.0],
                chromium: [542.0, 425.0, 150.562, 19.0],
                slack: TEXT_SLACK,
            },
            Part {
                name: "where",
                id: element_id("ExplainerWhere", 0),
                webkit: [702.578, 425.0, 69.422, 19.0],
                chromium: [702.562, 425.0, 69.438, 19.0],
                slack: TEXT_SLACK,
            },
            Part {
                name: "where_math",
                id: element_id("ExplainerWhereMath", 0),
                webkit: [753.812, 425.0, 18.188, 19.0],
                chromium: [753.797, 425.0, 18.203, 19.0],
                slack: TEXT_SLACK,
            },
            Part {
                name: "sentence",
                id: element_id("ExplainerSentence", 0),
                webkit: [506.0, 457.5, 266.0, 88.0],
                chromium: [506.0, 457.5, 266.0, 88.0],
                slack: SLACK,
            },
            Part {
                name: "plot",
                id: element_id("ExplainerPlot", 0),
                webkit: [506.0, 555.5, 268.0, 124.0],
                chromium: [506.0, 555.5, 268.0, 124.0],
                slack: SLACK,
            },
            Part {
                name: "snippet_row",
                id: element_id("ExplainerSnippetRow", 0),
                webkit: [506.0, 689.5, 266.0, 28.0],
                chromium: [506.0, 689.5, 266.0, 28.0],
                slack: SLACK,
            },
            Part {
                name: "snippet_label",
                id: element_id("ExplainerSnippetLabel", 0),
                webkit: [506.0, 696.5, 146.766 - TRACKING, 14.0],
                chromium: [506.0, 696.5, 146.516 - TRACKING, 14.0],
                slack: TEXT_SLACK,
            },
            Part {
                name: "snippet",
                id: element_id("ExplainerSnippet", 0),
                webkit: [659.969, 689.5, 61.641, 28.0],
                chromium: [659.719, 689.5, 61.672, 28.0],
                slack: TEXT_SLACK,
            },
            Part {
                name: "rule",
                id: element_id("ExplainerRule", 0),
                webkit: [506.0, 727.5, 266.0, 1.0],
                chromium: [506.0, 727.5, 266.0, 1.0],
                slack: SLACK,
            },
            Part {
                name: "gestures",
                id: element_id("ExplainerGestures", 0),
                webkit: [506.0, 738.5, 266.0, 38.0],
                chromium: [506.0, 738.5, 266.0, 38.0],
                slack: SLACK,
            },
        ]
    }

    fn near(b: clay::BoundingBox, want: [f32; 4], slack: f32) -> bool {
        (b.x - want[0]).abs() <= slack
            && (b.y - want[1]).abs() <= slack
            && (b.width - want[2]).abs() <= slack
            && (b.height - want[3]).abs() <= slack
    }

    #[test]
    fn explainer_is_the_references() {
        struct Case {
            name: &'static str,
            size: [f32; 2],
            shift: [f32; 2],
            place: mv_viewer::ExplainerPlace,
            polygon: [f32; 4],
        }
        let _guard = mv_clay::tests_lock();
        // The spec sets the Clay callback context here; the surface removed
        // the global, so there is nothing to set.
        let mut app = blank_app();
        assert!(mv_text::init(&mut app.fonts));
        for i in 0..mv_theme::Slug::LENGTH {
            let slug = mv_theme::Slug::from_usize(i);
            app.themes[slug] = mv_theme::resolve(slug, mv_theme::Space::Srgb);
        }
        crate::ui::init_clay(&mut app);
        app.system_scheme = mv_theme::Scheme::Dark;
        app.prefs = mv_theme::Prefs {
            mode: mv_theme::Mode::Dark,
            dark: mv_theme::Slug::ArcticIce,
            light: mv_theme::Slug::CleanTeal,
        };
        app.viewer = mv_viewer::initial_state();
        crate::plane_area::plane_init(&mut app);

        let cases = [
            Case {
                name: "explainer-a",
                size: [1440.0, 900.0],
                shift: [0.0, 0.0],
                place: mv_viewer::ExplainerPlace {
                    x: 489.0,
                    y: 404.5,
                    caret: mv_viewer::Caret {
                        side: mv_viewer::CaretSide::Left,
                        at: 20.0,
                    },
                },
                polygon: [481.0, 416.5, 8.0, 16.0],
            },
            Case {
                name: "explainer-stacked",
                size: [800.0, 900.0],
                shift: [-439.0, 46.0],
                place: mv_viewer::ExplainerPlace {
                    x: 50.0,
                    y: 450.5,
                    caret: mv_viewer::Caret {
                        side: mv_viewer::CaretSide::Top,
                        at: 20.0,
                    },
                },
                polygon: [62.0, 442.5, 16.0, 8.0],
            },
        ];
        for c in &cases {
            app.size = c.size;
            app.pixels = [(c.size[0] * 2.0) as i32, (c.size[1] * 2.0) as i32];
            app.viewer.view = None;
            app.equation_width = 0.0;
            app.scroll = enum_map::EnumMap::from_array([zero_scroll(), zero_scroll()]);
            app.explainer = ExplainerView {
                shown: None,
                placed: None,
                resting: None,
            };
            app.motion.overlays = enum_map::EnumMap::from_array([overlay_zero(), overlay_zero()]);
            app.viewer.explainer = Some(Explainer {
                param: Param::A,
                by: ExplainBy::Hover,
            });
            let _ = crate::draw::settle_layout(&mut app, c.size);

            let Some(placed) = app.explainer.placed else {
                panic!("{}: the explainer isn't placed", c.name);
            };
            assert_eq!(
                placed.place, c.place,
                "{}: placed at {:?}; the reference's {:?}",
                c.name, placed.place, c.place,
            );
            let (mut checked, mut passed) = (0, 0);
            for part in parts() {
                let data = element_data(part.id);
                checked += 1;
                assert!(data.found, "{}: no {} in the port", c.name, part.name);
                let wk = [
                    part.webkit[0] + c.shift[0],
                    part.webkit[1] + c.shift[1],
                    part.webkit[2],
                    part.webkit[3],
                ];
                let cr = [
                    part.chromium[0] + c.shift[0],
                    part.chromium[1] + c.shift[1],
                    part.chromium[2],
                    part.chromium[3],
                ];
                let b = data.boundingBox;
                let ok = near(b, wk, part.slack) || near(b, cr, part.slack);
                if ok {
                    passed += 1;
                }
                assert!(
                    ok,
                    "{}: {} at ({:.3}, {:.3}, {:.3} x {:.3}); WebKit's ({:.3}, {:.3}, {:.3} x {:.3}), Chromium's ({:.3}, {:.3}, {:.3} x {:.3})",
                    c.name,
                    part.name,
                    b.x,
                    b.y,
                    b.width,
                    b.height,
                    wk[0],
                    wk[1],
                    wk[2],
                    wk[3],
                    cr[0],
                    cr[1],
                    cr[2],
                    cr[3],
                );
            }
            let points = caret_points(
                [
                    placed.place.x as f32,
                    placed.place.y as f32,
                    mv_viewer::EXPLAINER_WIDTH as f32,
                    placed.height,
                ],
                placed.place.caret,
            );
            let (mut lo, mut hi) = (points[0], points[0]);
            for p in points {
                lo = [lo[0].min(p[0]), lo[1].min(p[1])];
                hi = [hi[0].max(p[0]), hi[1].max(p[1])];
            }
            let bbox = [lo[0], lo[1], hi[0] - lo[0], hi[1] - lo[1]];
            checked += 1;
            let ok = bbox == c.polygon;
            if ok {
                passed += 1;
            }
            assert!(
                ok,
                "{}: the caret spans {:?}; the reference's polygon {:?}",
                c.name, bbox, c.polygon,
            );
            eprintln!(
                "{} at {} x {}: {} of {} boxes at the reference's",
                c.name, c.size[0], c.size[1], passed, checked,
            );
        }
        crate::ui::release_clay(&mut app);
        mv_text::destroy(&mut app.fonts);
    }

    #[test]
    fn sentences_break_as_the_browsers_break_them() {
        let mut fonts = mv_text::uninit();
        let fonts: &mut mv_text::Fonts = &mut fonts;
        assert!(mv_text::init(&mut *fonts));
        let t_ = mv_theme::resolve(mv_theme::Slug::ArcticIce, mv_theme::Space::Srgb);
        let style =
            crate::typography::type_style(crate::typography::Type::ExplainerSentence, &t_).style;
        let want: enum_map::EnumMap<Param, Vec<&str>> = enum_map::EnumMap::from_array([
            vec![
                "Multiplies every height by a. a = 2",
                "makes the curve twice as tall;",
                "between 0 and 1 it flattens; below 0",
                "it flips upside down.",
            ],
            vec![
                "Adds k to every height, so the",
                "whole curve moves up by k (down",
                "when k is negative).",
            ],
            vec![
                "Divides x by b before f sees it. b = 2",
                "makes the curve twice as wide;",
                "between 0 and 1 it narrows.",
            ],
            vec![
                "Subtracts h from x before f sees it,",
                "so the curve moves right by h.",
                "Inside f the sign looks backwards:",
                "x − 2 moves right.",
            ],
        ]);
        let content = mv_viewer::EXPLAINER_WIDTH as f32 - 2.0 * (EXPLAINER_PADDING + 1.0);
        for i in 0..Param::LENGTH {
            let p = Param::from_usize(i);
            let lines =
                mv_text::break_lines(&mut *fonts, style, mv_viewer::EXPLAINER_SAY[p], content);
            assert_eq!(
                lines.len(),
                want[p].len(),
                "{p:?}: {} lines, not {}: {lines:?}",
                lines.len(),
                want[p].len(),
            );
            for (i, line) in lines.iter().enumerate() {
                assert_eq!(
                    line,
                    want[p][i],
                    "{p:?}, line {}: {line:?}, not {:?}",
                    i + 1,
                    want[p][i],
                );
            }
        }
        mv_text::destroy(&mut *fonts);
    }

    #[test]
    fn the_explainer_opens_and_closes_by_its_means() {
        fn at(app: &mut App, ms: i64) {
            app.r#loop.script_time = Some(ms * crate::app_loop::MS);
        }
        fn opened(app: &App) -> Option<Explainer> {
            app.viewer.explainer
        }

        let mut app = blank_app();
        app.viewer = mv_viewer::initial_state();
        let a = Hit {
            kind: HitKind::Chip,
            index: Param::A.into_usize() as i64,
        };
        let k = Hit {
            kind: HitKind::Chip,
            index: Param::K.into_usize() as i64,
        };
        let no_hit = Hit {
            kind: HitKind::None,
            index: 0,
        };

        at(&mut app, 2016);
        app.input.time = 2000 * crate::app_loop::MS;
        chip_hover(&mut app, no_hit, a);
        app.input.time = 0;
        assert_eq!(app.timers[Timer::ChipRest], 2400 * crate::app_loop::MS);
        assert!(opened(&app).is_none());
        at(&mut app, 2400);
        app.input.time = 2400 * crate::app_loop::MS;
        crate::app_loop::timer_fired(&mut app, Timer::ChipRest, 2400 * crate::app_loop::MS);
        app.input.time = 0;
        assert_eq!(
            opened(&app),
            Some(Explainer {
                param: Param::A,
                by: ExplainBy::Hover
            })
        );
        chip_hover(&mut app, a, k);
        assert!(opened(&app).is_none());
        assert!(app.timers[Timer::ChipRest] != 0);
        chip_hover(&mut app, k, no_hit);
        assert_eq!(app.timers[Timer::ChipRest], 0);
        assert!(opened(&app).is_none());

        chip_click(&mut app, Param::H);
        assert_eq!(
            opened(&app),
            Some(Explainer {
                param: Param::H,
                by: ExplainBy::Hover
            })
        );
        assert!(explainer_key(&mut app, Keycode::Escape, Mod::empty()));
        assert!(opened(&app).is_none());
        assert!(!explainer_key(&mut app, Keycode::Escape, Mod::empty()));

        crate::input::set_focus(
            &mut app,
            Hit {
                kind: HitKind::Box,
                index: Param::K.into_usize() as i64,
            },
            true,
            false,
        );
        assert!(explainer_key(&mut app, Keycode::Question, Mod::empty()));
        assert_eq!(
            opened(&app),
            Some(Explainer {
                param: Param::K,
                by: ExplainBy::Key
            })
        );
        assert!(explainer_key(&mut app, Keycode::Slash, Mod::LSHIFTMOD));
        assert!(opened(&app).is_none());
        assert!(!explainer_key(&mut app, Keycode::Slash, Mod::empty()));
        assert!(explainer_key(&mut app, Keycode::Question, Mod::empty()));
        chip_hover(&mut app, k, no_hit);
        assert_eq!(
            opened(&app),
            Some(Explainer {
                param: Param::K,
                by: ExplainBy::Key
            })
        );
        crate::input::set_focus(
            &mut app,
            Hit {
                kind: HitKind::Box,
                index: Param::B.into_usize() as i64,
            },
            true,
            false,
        );
        assert!(opened(&app).is_none());
        crate::input::set_focus(
            &mut app,
            Hit {
                kind: HitKind::Flip,
                index: Param::A.into_usize() as i64,
            },
            true,
            false,
        );
        assert!(explainer_key(&mut app, Keycode::Question, Mod::empty()));
        assert_eq!(
            opened(&app),
            Some(Explainer {
                param: Param::A,
                by: ExplainBy::Key
            })
        );
        crate::input::set_focus(
            &mut app,
            Hit {
                kind: HitKind::Plane,
                index: 0,
            },
            true,
            false,
        );
        assert!(opened(&app).is_none());
        assert!(!explainer_key(&mut app, Keycode::Question, Mod::empty()));
    }
}
