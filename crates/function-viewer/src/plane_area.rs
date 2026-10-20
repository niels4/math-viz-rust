//! The plane card: its layout, caption, scale bar, tools, ghost toggle, and
//! the viewport the plane draws through.
//!
//! The card lays out the canvas with its floating chrome; after layout the
//! view follows the default zoom and the plane lays out its scene. (Declares
//! against the plane, text and viewer surfaces; wired when they land.)

use mv_sys::clay;

use crate::app::App;
use crate::input::{Cursor, Hit, HitKind};

/// The plane's view and what it last laid out.
pub struct PlaneState {
    pub view: mv_plane::ViewState,
    pub fit: f64,
    pub r#box: clay::BoundingBox,
    /// On the frame's temp allocator, between lay-out and draw.
    pub frame: mv_plane::Frame,
    pub hits: Vec<mv_plane::Hit>,
    pub grip: Option<super::plane_input::PlaneGrip>,
    /// The pan's last 120 ms, for the release's velocity.
    pub samples: Vec<mv_plane::DragSample>,
    pub glide: Option<mv_plane::Glide>,
    /// The last report stands, with this cursor, until the pointer moves.
    pub still: Option<Cursor>,
}

/// Fits the view on first layout.
pub fn plane_init(app: &mut App) {
    app.plane.view = mv_plane::ViewState {
        zoom: mv_plane::BASE_ZOOM,
        pan_x: 0.0,
        pan_y: 0.0,
    };
    app.plane.fit = mv_plane::default_zoom(0.0, mv_plane::DEFAULT_FIT_HALF_RANGE_Y);
}

/// The plane canvas custom element's payload: none.
#[derive(Clone, Copy, Debug)]
pub struct PlaneCanvas;

/// The chrome's stacking layer and offsets.
pub const PLANE_CHROME_Z: i16 = 1;
pub const PLANE_CHROME_X: f32 = 28.0;
pub const PLANE_CHROME_Y: f32 = 22.0;

/// A layout with only its sizing set.
fn sizing_layout(width: clay::SizingAxis, height: clay::SizingAxis) -> clay::LayoutConfig {
    clay::LayoutConfig {
        sizing: clay::Sizing { width, height },
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

/// A zero-fit sizing axis: Clay's `SizingFit` with a zero min/max.
fn fit_zero() -> clay::SizingAxis {
    mv_clay::sizing_fit(clay::SizingConstraintsMinMax { min: 0.0, max: 0.0 })
}

/// A grow sizing axis with a minimum.
fn grow_min(min: f32) -> clay::SizingAxis {
    clay::SizingAxis {
        constraints: clay::SizingConstraints {
            sizeMinMax: clay::SizingConstraintsMinMax { min, max: 0.0 },
        },
        r#type: clay::SizingType::Grow,
    }
}

/// A laid-out frame with nothing in it: every field zero, as Odin's
/// `app.plane.frame = {}` leaves them when the Plane element is missing.
fn zero_frame() -> mv_plane::Frame {
    mv_plane::Frame {
        vp: mv_plane::Viewport {
            width: 0.0,
            height: 0.0,
            zoom: 0.0,
            origin_x: 0.0,
            origin_y: 0.0,
        },
        scene: mv_plane::Scene {
            grid_alpha: None,
            curves: Vec::new(),
            points: Vec::new(),
            guides: Vec::new(),
            handles: Vec::new(),
            annotations: Vec::new(),
        },
        grid: mv_plane::GridLayout {
            minor_x: Vec::new(),
            major_x: Vec::new(),
            minor_y: Vec::new(),
            major_y: Vec::new(),
            axis_x: None,
            axis_y: None,
            steps: mv_plane::GridSteps {
                minor: 0.0,
                major: 0.0,
            },
            ticks_x: Vec::new(),
            ticks_y: Vec::new(),
            labels: Vec::new(),
            origin: None,
        },
        marks: mv_plane::MarksLayout {
            annotations: Vec::new(),
            handles: Vec::new(),
            guides: Vec::new(),
            drop_lines: Vec::new(),
            points: Vec::new(),
            hits: Vec::new(),
        },
    }
}

/// A floating config on the parent at an offset.
fn floating_on_parent(
    element: clay::FloatingAttachPointType,
    parent: clay::FloatingAttachPointType,
    offset: [f32; 2],
) -> clay::FloatingElementConfig {
    clay::FloatingElementConfig {
        offset,
        expand: clay::Dimensions {
            width: 0.0,
            height: 0.0,
        },
        parentId: 0,
        zIndex: PLANE_CHROME_Z,
        attachment: clay::FloatingAttachPoints { element, parent },
        pointerCaptureMode: clay::PointerCaptureMode::Capture,
        attachTo: clay::FloatingAttachToElement::Parent,
        clipTo: clay::FloatingClipToElement::None,
    }
}

/// Declares the canvas custom element with its caption, scale and tools.
pub fn plane_area(app: &mut App, t: &mv_theme::Theme, min_height: f32) {
    let id = mv_clay::id("Plane", 0);
    let radius = crate::ui::corner_radius_all(t.card_radius);
    crate::input::region(
        app,
        id,
        Hit {
            kind: HitKind::Plane,
            index: 0,
        },
        Cursor::Crosshair,
        Cursor::Grabbing,
        true,
        radius,
    );
    let focused = app.input.focus_visible
        && app.input.focus
            == (Hit {
                kind: HitKind::Plane,
                index: 0,
            });
    let mut d = crate::ui::empty_decl();
    d.layout = sizing_layout(crate::ui::sizing_grow(), grow_min(min_height));
    d.cornerRadius = radius;
    d.border = clay::BorderElementConfig {
        color: crate::ui::ink(
            t,
            if focused {
                mv_theme::Token::Primary
            } else {
                mv_theme::Token::Border
            },
        ),
        width: mv_clay::border_outside(1),
    };
    d.custom = crate::widgets::custom_data(crate::widgets::Custom::PlaneCanvas(PlaneCanvas));
    crate::ui::ui_scope(app, id, &d, |app| {
        plane_caption(app, t);
        plane_scale(app, t);
        // The tools' box takes the pointer from the plane, between and beside
        // its buttons too, as the tools' element does over the canvas: no Q,
        // no pan, the default cursor. Declared before the buttons, which sit
        // above it.
        crate::input::region(
            app,
            mv_clay::id("PlaneTools", 0),
            Hit {
                kind: HitKind::None,
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
        plane_tools(app, t);
    });
}

/// The caption's label and math.
pub const PLANE_LABEL: &str = "Plane";
pub const PLANE_CAPTION_MATH: &str = "y = f(x)";

/// Lays out the floating caption plate with its label and math runs.
pub fn plane_caption(app: &mut App, t: &mv_theme::Theme) {
    let mut pieces = Vec::with_capacity(1 + crate::math_text::math_runs(PLANE_CAPTION_MATH).len());
    pieces.push(mv_text::Piece {
        style: mv_text::Style {
            face: mv_text::Face::WorkSans600,
            size: 14.0,
            tracking: t.label_tracking,
            features: Default::default(),
        },
        text: "PLANE".to_string(),
        ink: t.colors[mv_theme::Token::ForegroundMuted],
        line_height: 0.0,
        r#box: mv_text::Box::Exact,
        min_width: 0.0,
        align: mv_text::Align::Left,
        pad: [0.0, 10.0 - t.label_tracking],
    });
    for run in crate::math_text::math_runs(PLANE_CAPTION_MATH) {
        pieces.push(mv_text::Piece {
            style: mv_text::Style {
                face: crate::math_text::math_face(&run),
                size: 18.0,
                tracking: 0.0,
                features: Default::default(),
            },
            text: run.text,
            ink: t.colors[mv_theme::Token::Foreground],
            line_height: 0.0,
            r#box: mv_text::Box::Exact,
            min_width: 0.0,
            align: mv_text::Align::Left,
            pad: [0.0, 0.0],
        });
    }
    let line = mv_text::typeset(&mut app.fonts, &pieces, 0.0, mv_text::Vertical::Center, 1.0);
    let mut d = crate::ui::empty_decl();
    d.layout = sizing_layout(fit_zero(), fit_zero());
    d.layout.padding = clay::Padding {
        left: 12,
        right: 12,
        top: 6,
        bottom: 6,
    };
    d.backgroundColor = crate::ui::ink(t, mv_theme::Token::Background);
    d.cornerRadius = crate::ui::corner_radius_all(6.0);
    d.floating = floating_on_parent(
        clay::FloatingAttachPointType::LeftTop,
        clay::FloatingAttachPointType::LeftTop,
        [PLANE_CHROME_X, PLANE_CHROME_Y],
    );
    crate::ui::ui_scope(app, mv_clay::id("PlaneCaption", 0), &d, |app| {
        let mut inner = crate::ui::empty_decl();
        inner.layout = sizing_layout(
            crate::ui::sizing_fixed(line.width),
            crate::ui::sizing_fixed(line.height),
        );
        inner.custom =
            crate::widgets::custom_data(crate::widgets::Custom::Typeset(crate::widgets::Typeset {
                line,
                selectable: false,
            }));
        crate::ui::ui_scope(app, mv_clay::id("PlaneCaptionLine", 0), &inner, |_app| {});
    });
}

/// The floating scale-bar plate with its bar and zoom label.
pub fn plane_scale(app: &mut App, t: &mv_theme::Theme) {
    let mut d = crate::ui::empty_decl();
    d.layout = sizing_layout(fit_zero(), fit_zero());
    d.layout.padding = clay::Padding {
        left: 15,
        right: 15,
        top: 9,
        bottom: 9,
    };
    d.layout.childGap = 8;
    d.layout.childAlignment = clay::ChildAlignment {
        x: clay::LayoutAlignmentX::Left,
        y: clay::LayoutAlignmentY::Center,
    };
    d.backgroundColor = crate::ui::ink(t, mv_theme::Token::Background);
    d.border = clay::BorderElementConfig {
        color: crate::ui::ink(t, mv_theme::Token::Border),
        width: mv_clay::border_outside(1),
    };
    d.cornerRadius = crate::ui::corner_radius_all(t.card_radius);
    d.floating = floating_on_parent(
        clay::FloatingAttachPointType::LeftBottom,
        clay::FloatingAttachPointType::LeftBottom,
        [PLANE_CHROME_X, -PLANE_CHROME_Y],
    );
    crate::ui::ui_scope(app, mv_clay::id("PlaneScale", 0), &d, |app| {
        let mut inner = crate::ui::empty_decl();
        inner.layout = sizing_layout(
            crate::ui::sizing_fixed(mv_plane::SCALE_BAR_PX as f32),
            crate::ui::sizing_fixed(SCALE_BAR_HEIGHT),
        );
        inner.custom = crate::widgets::custom_data(crate::widgets::Custom::ScaleBar(ScaleBar {
            ink: t.colors[mv_theme::Token::Foreground],
        }));
        crate::ui::ui_scope(app, mv_clay::id("PlaneScaleBar", 0), &inner, |_app| {});
        crate::typography::label(
            app,
            crate::typography::Type::ScaleLabel,
            &mv_plane::scale_label(app.plane.view.zoom),
            mv_theme::Token::ForegroundMuted,
        );
    });
}

/// The scale bar's height.
pub const SCALE_BAR_HEIGHT: f32 = 10.0;

/// A scale bar in an ink.
#[derive(Clone, Copy, Debug)]
pub struct ScaleBar {
    pub ink: mv_theme::Color,
}

/// Strokes the U-shape butt/miter.
pub fn draw_scale_bar(vg: &mut mv_nvg::Context, b: clay::BoundingBox, bar: ScaleBar) {
    vg.save();
    vg.begin_path();
    vg.move_to(b.x + 1.0, b.y + 1.0);
    vg.line_to(b.x + 1.0, b.y + 9.0);
    vg.line_to(b.x + mv_plane::SCALE_BAR_PX as f32 - 1.0, b.y + 9.0);
    vg.line_to(b.x + mv_plane::SCALE_BAR_PX as f32 - 1.0, b.y + 1.0);
    vg.stroke_color(bar.ink);
    vg.stroke_width(2.0);
    vg.line_cap(mv_sys::nvg::LineCapType::BUTT);
    vg.line_join(mv_sys::nvg::LineCapType::MITER);
    vg.stroke();
    vg.restore();
}

/// The bottom-right floating row: tools, origin and zoom.
pub fn plane_tools(app: &mut App, t: &mv_theme::Theme) {
    let mut d = crate::ui::empty_decl();
    d.layout = sizing_layout(fit_zero(), fit_zero());
    d.layout.childGap = 10;
    d.layout.childAlignment = clay::ChildAlignment {
        x: clay::LayoutAlignmentX::Left,
        y: clay::LayoutAlignmentY::Bottom,
    };
    d.floating = floating_on_parent(
        clay::FloatingAttachPointType::RightBottom,
        clay::FloatingAttachPointType::RightBottom,
        [-PLANE_CHROME_X, -PLANE_CHROME_Y],
    );
    crate::ui::ui_scope(app, mv_clay::id("PlaneTools", 0), &d, |app| {
        let mut owner = crate::ui::empty_decl();
        owner.layout = sizing_layout(fit_zero(), fit_zero());
        owner.layout.childGap = 10;
        owner.layout.childAlignment = clay::ChildAlignment {
            x: clay::LayoutAlignmentX::Left,
            y: clay::LayoutAlignmentY::Center,
        };
        crate::ui::ui_scope(app, mv_clay::id("PlaneOwnerTools", 0), &owner, |app| {
            if mv_viewer::is_transformed(&app.viewer) {
                ghost_toggle(app, t);
            }
        });
        let mut controls = crate::ui::empty_decl();
        controls.layout = sizing_layout(fit_zero(), fit_zero());
        controls.layout.childGap = 10;
        controls.layout.layoutDirection = clay::LayoutDirection::TopToBottom;
        crate::ui::ui_scope(app, mv_clay::id("PlaneViewControls", 0), &controls, |app| {
            origin_button(app, t);
            zoom_control(app, t);
        });
    });
}

/// A 36 px card button with the locate icon and hover/focus states.
pub fn origin_button(app: &mut App, t: &mv_theme::Theme) {
    let id = mv_clay::id("PlaneOrigin", 0);
    let hit = Hit {
        kind: HitKind::Origin,
        index: 0,
    };
    let radius = crate::ui::corner_radius_all(t.card_radius);
    let focused = app.input.focus_visible && app.input.focus == hit;
    crate::input::region(app, id, hit, Cursor::Pointer, Cursor::Default, true, radius);
    let mut d = crate::ui::empty_decl();
    d.layout = sizing_layout(
        crate::ui::sizing_fixed(VIEW_CONTROL),
        crate::ui::sizing_fixed(VIEW_CONTROL),
    );
    d.layout.childAlignment = clay::ChildAlignment {
        x: clay::LayoutAlignmentX::Center,
        y: clay::LayoutAlignmentY::Center,
    };
    d.backgroundColor = crate::ui::ink(
        t,
        if app.input.hover == hit {
            mv_theme::Token::Secondary
        } else {
            mv_theme::Token::Card
        },
    );
    d.border = clay::BorderElementConfig {
        color: crate::ui::ink(
            t,
            if focused {
                mv_theme::Token::Primary
            } else {
                mv_theme::Token::Border
            },
        ),
        width: mv_clay::border_outside(1),
    };
    d.cornerRadius = radius;
    crate::ui::ui_scope(app, id, &d, |app| {
        let mut inner = crate::ui::empty_decl();
        inner.layout = sizing_layout(
            crate::ui::sizing_fixed(LOCATE_ICON),
            crate::ui::sizing_fixed(LOCATE_ICON),
        );
        inner.custom =
            crate::widgets::custom_data(crate::widgets::Custom::Icon(crate::icons::Icon {
                kind: crate::icons::IconKind::Locate,
                ink: t.colors[mv_theme::Token::Foreground],
            }));
        crate::ui::ui_scope(app, mv_clay::id("PlaneOriginIcon", 0), &inner, |_app| {});
    });
}

/// The control sizes.
pub const VIEW_CONTROL: f32 = 36.0;
pub const ZOOM_STEP: f32 = 34.0;
pub const LOCATE_ICON: f32 = 20.0;

/// The zoom capsule with + over a rule over − and its limit/disabled/hover
/// states.
pub fn zoom_control(app: &mut App, t: &mv_theme::Theme) {
    let radius = t.card_radius;
    let inner = (radius - 1.0).max(0.0);
    let capsule = mv_clay::id("PlaneZoom", 0);
    let focused = app.input.focus_visible
        && app.input.focus
            == (Hit {
                kind: HitKind::Zoom,
                index: 0,
            });
    crate::input::region(
        app,
        capsule,
        Hit {
            kind: HitKind::Zoom,
            index: 0,
        },
        Cursor::Default,
        Cursor::Default,
        true,
        crate::ui::corner_radius_all(radius),
    );
    let mut d = crate::ui::empty_decl();
    d.layout = sizing_layout(fit_zero(), fit_zero());
    d.layout.padding = mv_clay::padding_all(1);
    d.layout.layoutDirection = clay::LayoutDirection::TopToBottom;
    d.backgroundColor = crate::ui::ink(t, mv_theme::Token::Card);
    d.border = clay::BorderElementConfig {
        color: crate::ui::ink(
            t,
            if focused {
                mv_theme::Token::Primary
            } else {
                mv_theme::Token::Border
            },
        ),
        width: mv_clay::border_outside(1),
    };
    d.cornerRadius = crate::ui::corner_radius_all(radius);
    crate::ui::ui_scope(app, capsule, &d, |app| {
        for (i, direction) in [1i64, -1].iter().enumerate() {
            if i > 0 {
                let mut rule = crate::ui::empty_decl();
                rule.layout = sizing_layout(
                    crate::ui::sizing_fixed(ZOOM_STEP),
                    crate::ui::sizing_fixed(1.0),
                );
                rule.backgroundColor = crate::ui::ink(t, mv_theme::Token::Border);
                crate::ui::ui_scope(app, mv_clay::id("PlaneZoomRule", 0), &rule, |_app| {});
            }
            let id = mv_clay::id("PlaneZoomStep", i as u32);
            let hit = Hit {
                kind: HitKind::ZoomStep,
                index: i as i64,
            };
            let at_limit =
                mv_plane::step_zoom(app.plane.view.zoom, *direction) == app.plane.view.zoom;
            let mut corners = clay::CornerRadius {
                topLeft: 0.0,
                topRight: 0.0,
                bottomLeft: 0.0,
                bottomRight: 0.0,
            };
            if i == 0 {
                corners.topLeft = inner;
                corners.topRight = inner;
            } else {
                corners.bottomLeft = inner;
                corners.bottomRight = inner;
            }
            crate::input::region(
                app,
                id,
                hit,
                if at_limit {
                    Cursor::Default
                } else {
                    Cursor::Pointer
                },
                Cursor::Default,
                false,
                corners,
            );
            let mut step = crate::ui::empty_decl();
            step.layout = sizing_layout(
                crate::ui::sizing_fixed(ZOOM_STEP),
                crate::ui::sizing_fixed(ZOOM_STEP),
            );
            step.layout.childAlignment = clay::ChildAlignment {
                x: clay::LayoutAlignmentX::Center,
                y: clay::LayoutAlignmentY::Center,
            };
            step.backgroundColor = if app.input.hover == hit && !at_limit {
                crate::ui::ink(t, mv_theme::Token::Secondary)
            } else {
                [0.0; 4]
            };
            step.cornerRadius = corners;
            crate::ui::ui_scope(app, id, &step, |app| {
                crate::typography::label(
                    app,
                    crate::typography::Type::ZoomStep,
                    if *direction > 0 {
                        "+"
                    } else {
                        mv_numfmt::MINUS
                    },
                    if at_limit {
                        mv_theme::Token::ForegroundMuted
                    } else {
                        mv_theme::Token::Foreground
                    },
                );
            });
        }
    });
}

/// Arrows and +/− step the zoom unless held with Ctrl/GUI/Alt; stops the glide.
pub fn zoom_control_key(
    app: &mut App,
    key: sdl3::keyboard::Keycode,
    mods: sdl3::keyboard::Mod,
) -> bool {
    if mods.intersects(
        sdl3::keyboard::Mod::LCTRLMOD
            | sdl3::keyboard::Mod::RCTRLMOD
            | sdl3::keyboard::Mod::LGUIMOD
            | sdl3::keyboard::Mod::RGUIMOD
            | sdl3::keyboard::Mod::LALTMOD
            | sdl3::keyboard::Mod::RALTMOD,
    ) {
        return false;
    }
    let direction = match key {
        sdl3::keyboard::Keycode::Up
        | sdl3::keyboard::Keycode::Right
        | sdl3::keyboard::Keycode::Plus
        | sdl3::keyboard::Keycode::Equals
        | sdl3::keyboard::Keycode::KpPlus => 1,
        sdl3::keyboard::Keycode::Down
        | sdl3::keyboard::Keycode::Left
        | sdl3::keyboard::Keycode::Minus
        | sdl3::keyboard::Keycode::Underscore
        | sdl3::keyboard::Keycode::KpMinus => -1,
        _ => return false,
    };
    crate::plane_input::plane_stop_glide(app);
    crate::plane_input::plane_zoom_step(app, direction);
    true
}

/// Ghost toggle, origin and zoom-step dispatch on the hit kind.
pub fn plane_tool_activate(app: &mut App, hit: Hit) {
    match hit.kind {
        HitKind::Ghost => crate::parts::dispatch(
            app,
            mv_viewer::Action::SetGhost(mv_viewer::SetGhost {
                on: !app.viewer.ghost_on,
            }),
        ),
        HitKind::Origin => crate::plane_input::plane_to_origin(app),
        HitKind::ZoomStep => {
            crate::plane_input::plane_zoom_step(app, if hit.index == 0 { 1 } else { -1 })
        }
        _ => {}
    }
}

/// The ghost toggle's label and off alpha.
pub const ORIGINAL_LABEL: &str = "Original";
pub const GHOST_OFF_ALPHA: f32 = 0.45;

/// The toggle plate with swatch, label and math in its on/off/border states.
pub fn ghost_toggle(app: &mut App, t: &mv_theme::Theme) {
    let id = mv_clay::id("GhostToggle", 0);
    let hit = Hit {
        kind: HitKind::Ghost,
        index: 0,
    };
    let radius = crate::ui::corner_radius_all(t.card_radius);
    let on = app.viewer.ghost_on;
    let border: u16 = if on { 2 } else { 1 };
    let content: f32 = if on { 1.0 } else { GHOST_OFF_ALPHA };
    crate::input::region(app, id, hit, Cursor::Pointer, Cursor::Default, true, radius);
    let math = crate::widgets::math_line(
        &mut app.fonts,
        &original_math(mv_viewer::BASE_FUNCTIONS[app.viewer.func].label),
        GHOST_TOGGLE_MATH_SIZE,
        GHOST_TOGGLE_MATH_LINE,
        mv_theme::fade(t.colors[mv_theme::Token::ForegroundMuted], content),
    );
    let mut d = crate::ui::empty_decl();
    d.layout = sizing_layout(fit_zero(), crate::ui::sizing_fixed(VIEW_CONTROL));
    d.layout.padding = clay::Padding {
        left: 12 + border,
        right: 12 + border,
        top: 0,
        bottom: 0,
    };
    d.layout.childGap = 8;
    d.layout.childAlignment = clay::ChildAlignment {
        x: clay::LayoutAlignmentX::Left,
        y: clay::LayoutAlignmentY::Center,
    };
    d.backgroundColor = crate::ui::ink(
        t,
        if app.input.hover == hit {
            mv_theme::Token::Secondary
        } else {
            mv_theme::Token::Card
        },
    );
    d.border = clay::BorderElementConfig {
        color: crate::ui::ink(
            t,
            if on {
                mv_theme::Token::Primary
            } else {
                mv_theme::Token::Border
            },
        ),
        width: mv_clay::border_outside(border),
    };
    d.cornerRadius = radius;
    crate::ui::ui_scope(app, id, &d, |app| {
        let mut swatch = crate::ui::empty_decl();
        swatch.layout = sizing_layout(
            crate::ui::sizing_fixed(GHOST_SWATCH[0]),
            crate::ui::sizing_fixed(GHOST_SWATCH[1]),
        );
        swatch.custom =
            crate::widgets::custom_data(crate::widgets::Custom::GhostSwatch(GhostSwatch {
                ink: mv_theme::fade(t.colors[mv_theme::Token::ForegroundMuted], content),
            }));
        crate::ui::ui_scope(app, mv_clay::id("GhostSwatch", 0), &swatch, |_app| {});
        crate::typography::label_colored(
            app,
            crate::typography::Type::ToggleLabel,
            ORIGINAL_LABEL,
            mv_theme::fade(t.colors[mv_theme::Token::Foreground], content),
        );
        let mut math_box = crate::ui::empty_decl();
        math_box.layout = sizing_layout(
            crate::ui::sizing_fixed(math.width),
            crate::ui::sizing_fixed(math.height),
        );
        math_box.custom =
            crate::widgets::custom_data(crate::widgets::Custom::Typeset(crate::widgets::Typeset {
                line: math,
                selectable: false,
            }));
        crate::ui::ui_scope(app, mv_clay::id("GhostMath", 0), &math_box, |_app| {});
    });
}

/// The toggle math's size/line and the swatch size.
pub const GHOST_TOGGLE_MATH_SIZE: f32 = 17.0;
pub const GHOST_TOGGLE_MATH_LINE: f32 = 21.0;
pub const GHOST_SWATCH: [f32; 2] = [22.0, 4.0];

/// Prefixes the ghost function with `y = `.
pub fn original_math(g: &str) -> String {
    text_concat("y = ", g)
}

/// Concatenates two strings.
fn text_concat(a: &str, b: &str) -> String {
    let mut out = String::with_capacity(a.len() + b.len());
    out.push_str(a);
    out.push_str(b);
    out
}

/// A ghost swatch in an ink.
#[derive(Clone, Copy, Debug)]
pub struct GhostSwatch {
    pub ink: mv_theme::Color,
}

/// Strokes the dashed polyline.
pub fn draw_ghost_swatch(vg: &mut mv_nvg::Context, b: clay::BoundingBox, swatch: GhostSwatch) {
    vg.save();
    vg.translate(b.x, b.y);
    vg.begin_path();
    mv_plane::add_polyline(
        vg,
        &[[0.0, 2.0], [f64::from(GHOST_SWATCH[0]), 2.0]],
        Some([6.0, 4.0]),
    );
    vg.stroke_color(swatch.ink);
    vg.stroke_width(2.0);
    vg.line_cap(mv_sys::nvg::LineCapType::BUTT);
    vg.stroke();
    vg.restore();
}

/// The viewport for the box at the device ratio.
pub fn plane_viewport(app: &mut App, b: clay::BoundingBox) -> mv_plane::Viewport {
    mv_plane::make_viewport(
        f64::from(b.width),
        f64::from(b.height),
        device_ratio(app),
        app.plane.view,
    )
}

/// Pixels over logical size, or 1.
pub fn device_ratio(app: &mut App) -> f64 {
    if app.size[0] > 0.0 {
        f64::from(app.pixels[0]) / f64::from(app.size[0])
    } else {
        1.0
    }
}

/// Follows the default zoom and dispatches the view change; reports whether
/// the view or zoom changed.
pub fn plane_after_layout(app: &mut App, b: clay::BoundingBox) -> bool {
    let changed = follow_default_zoom(app, f64::from(b.height));
    let vp = plane_viewport(app, b);
    let before = app.viewer.view;
    let dirty = app.dirty;
    crate::parts::dispatch(
        app,
        mv_viewer::Action::SetView(mv_viewer::SetView {
            view: mv_plane::View {
                zoom: vp.zoom,
                extent: mv_plane::visible_extent(vp),
            },
        }),
    );
    app.dirty = dirty;
    changed || app.viewer.view != before
}

/// Updates the fit; an untouched view follows it. Reports zoom change.
pub fn follow_default_zoom(app: &mut App, height: f64) -> bool {
    let fit = mv_plane::default_zoom(height, mv_plane::DEFAULT_FIT_HALF_RANGE_Y);
    if fit == app.plane.fit {
        return false;
    }
    let untouched = app.plane.view.zoom == app.plane.fit
        && app.plane.view.pan_x == 0.0
        && app.plane.view.pan_y == 0.0;
    app.plane.fit = fit;
    if untouched {
        app.plane.view.zoom = fit;
    }
    untouched
}

/// Gathers the plate rects, lays out the plane, and keeps hits, arrivals and
/// cursor.
pub fn lay_out_plane(app: &mut App) {
    app.plane.hits.clear();
    app.plane.frame = zero_frame();
    let boxed = app
        .clay
        .as_ref()
        .expect("clay context initialised")
        .get_element_data(mv_clay::id("Plane", 0));
    if !boxed.found {
        return;
    }
    let b = boxed.boundingBox;
    app.plane.r#box = b;
    let readout = mv_plane::readout_faces(crate::ui::current_theme(app).readout_font);
    let mut plates = Vec::with_capacity(4);
    for name in [
        "PlaneCaption",
        "PlaneScale",
        "PlaneOwnerTools",
        "PlaneViewControls",
    ] {
        let part = app
            .clay
            .as_ref()
            .expect("clay context initialised")
            .get_element_data(mv_clay::id(name, 0));
        let r = part.boundingBox;
        if !part.found || r.width == 0.0 || r.height == 0.0 {
            continue;
        }
        plates.push(mv_plane::Rect {
            x: f64::from(r.x - b.x),
            y: f64::from(r.y - b.y),
            w: f64::from(r.width),
            h: f64::from(r.height),
        });
    }
    let scene = crate::motion_driver::plane_scene_now(app);
    let vp = plane_viewport(app, b);
    let measure = mv_plane::text_measure(&mut app.fonts);
    app.plane.frame = mv_plane::lay_out(scene, vp, measure, readout, &plates);
    let hits = app.plane.frame.marks.hits.clone();
    app.plane.hits.extend(hits);
    crate::motion_driver::plane_arrivals(app);
    if app.input.over
        && app.input.hover.kind == HitKind::Plane
        && app.input.pressed.kind == HitKind::None
    {
        let cursor = crate::plane_input::plane_cursor(app);
        crate::input::set_cursor(app, cursor);
    }
}

/// The scissored rounded background, the snapped canvas scene and chrome, and
/// the corner clips.
pub fn draw_plane(app: &mut App, b: clay::BoundingBox, radius: clay::CornerRadius) {
    let t = crate::ui::current_theme(app).clone();
    let ratio = device_ratio(app);
    app.vg.save();
    app.vg.scissor(b.x, b.y, b.width, b.height);
    app.vg.begin_path();
    crate::draw::rounded_rect(&mut app.vg, b, radius);
    app.vg.fill_color(t.colors[mv_theme::Token::Background]);
    app.vg.fill();
    app.vg.save();
    let snap = |v: f32, ratio: f64| ((f64::from(v) * ratio).round() / ratio) as f32;
    app.vg.translate(snap(b.x, ratio), snap(b.y, ratio));
    {
        let painter = mv_plane::Painter {
            vg: &mut app.vg,
            fonts: &mut app.fonts,
            theme: &t,
            density: ratio,
        };
        mv_plane::draw(painter, &app.plane.frame);
    }
    {
        let painter = mv_plane::Painter {
            vg: &mut app.vg,
            fonts: &mut app.fonts,
            theme: &t,
            density: ratio,
        };
        mv_plane::draw_chrome(painter, f64::from(b.width), f64::from(b.height));
    }
    app.vg.restore();
    clip_corners(
        &mut app.vg,
        b,
        crate::draw::css_radii(b, radius),
        t.colors[mv_theme::Token::Background],
    );
    // Odin's `defer nvg.Restore` runs here: without it the plane's scissor
    // leaks onto the card border strokes drawn after it (nvg-dust).
    app.vg.restore();
}

/// Fills the four outside-corner patches.
pub fn clip_corners(
    vg: &mut mv_nvg::Context,
    b: clay::BoundingBox,
    r: clay::CornerRadius,
    outside: mv_theme::Color,
) {
    fn corner(vg: &mut mv_nvg::Context, cx: f32, cy: f32, sx: f32, sy: f32, r: f32) {
        if r <= 0.0 {
            return;
        }
        vg.move_to(cx, cy);
        vg.line_to(cx + sx * r, cy);
        vg.arc_to(cx, cy, cx, cy + sy * r, r);
        vg.close_path();
    }
    vg.begin_path();
    corner(vg, b.x, b.y, 1.0, 1.0, r.topLeft);
    corner(vg, b.x + b.width, b.y, -1.0, 1.0, r.topRight);
    corner(vg, b.x, b.y + b.height, 1.0, -1.0, r.bottomLeft);
    corner(vg, b.x + b.width, b.y + b.height, -1.0, -1.0, r.bottomRight);
    vg.fill_color(outside);
    vg.fill();
}

#[cfg(test)]
mod tests {
    use super::*;
    use enum_map::Enum;

    // The test below ports plane_area_test.odin one for one. It reaches
    // bodies in crates whose milestones have not merged yet (text,
    // viewer-base, render-base, render-widgets, app-input), so it waits on
    // those slugs.

    const SLACK: f32 = 0.02;
    const TEXT_SLACK: f32 = 0.1;

    struct ChromeBox {
        name: &'static str,
        id: clay::ElementId,
        webkit: [f32; 4],
        chromium: [f32; 4],
        slack: f32,
    }

    fn near(b: clay::BoundingBox, want: [f32; 4], slack: f32) -> bool {
        (b.x - want[0]).abs() <= slack
            && (b.y - want[1]).abs() <= slack
            && (b.width - want[2]).abs() <= slack
            && (b.height - want[3]).abs() <= slack
    }

    fn check_boxes(app: &mut App, state: &str, boxes: &[ChromeBox]) {
        for b in boxes {
            let data = app
                .clay
                .as_ref()
                .expect("clay context initialised")
                .get_element_data(b.id);
            assert!(data.found, "{}: no {}", state, b.name);
            let r = data.boundingBox;
            let cr = if b.chromium != [0.0; 4] {
                b.chromium
            } else {
                b.webkit
            };
            assert!(
                near(r, b.webkit, b.slack) || near(r, cr, b.slack),
                "{}: {} at ({:.3}, {:.3}, {:.3} x {:.3}); WebKit's ({:.3}, {:.3}, {:.3} x {:.3}), Chromium's ({:.3}, {:.3}, {:.3} x {:.3})",
                state,
                b.name,
                r.x,
                r.y,
                r.width,
                r.height,
                b.webkit[0],
                b.webkit[1],
                b.webkit[2],
                b.webkit[3],
                cr[0],
                cr[1],
                cr[2],
                cr[3],
            );
        }
    }

    #[test]
    fn the_planes_chrome_is_the_references() {
        let _guard = mv_clay::tests_lock();
        // Clay's callbacks run in app context: set once the lock is held, so
        // a test waiting for it can't hand this one its own context
        // mid-layout. (The Rust port wires the context with the harness.)
        let mut app = crate::motion_driver::driver_app(false);
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
        plane_init(&mut app);
        app.size = [1440.0, 900.0];
        app.pixels = [2880, 1800];

        // Default: Arctic Ice, the label tracked 2.8 px; no owner's tools.
        let size = app.size;
        let _ = crate::draw::settle_layout(&mut app, size);
        check_boxes(
            &mut app,
            "default",
            &[
                ChromeBox {
                    name: "plane",
                    id: mv_clay::id("Plane", 0),
                    webkit: [488.0, 92.0, 936.0, 792.0],
                    chromium: [0.0; 4],
                    slack: SLACK,
                },
                ChromeBox {
                    name: "caption",
                    id: mv_clay::id("PlaneCaption", 0),
                    webkit: [516.0, 114.0, 146.47, 35.0],
                    chromium: [516.0, 114.0, 146.5, 35.0],
                    slack: TEXT_SLACK,
                },
                // The label's 16 px line box centred in the maths' 23: its
                // top at 123.5.
                ChromeBox {
                    name: "caption_line",
                    id: mv_clay::id("PlaneCaptionLine", 0),
                    webkit: [528.0, 120.0, 122.47, 23.0],
                    chromium: [528.0, 120.0, 122.5, 23.0],
                    slack: TEXT_SLACK,
                },
                ChromeBox {
                    name: "scale",
                    id: mv_clay::id("PlaneScale", 0),
                    webkit: [516.0, 827.0, 111.41, 35.0],
                    chromium: [0.0; 4],
                    slack: TEXT_SLACK,
                },
                ChromeBox {
                    name: "scale_bar",
                    id: mv_clay::id("PlaneScaleBar", 0),
                    webkit: [531.0, 839.5, 50.0, 10.0],
                    chromium: [0.0; 4],
                    slack: SLACK,
                },
                ChromeBox {
                    name: "tools",
                    id: mv_clay::id("PlaneTools", 0),
                    webkit: [1350.0, 745.0, 46.0, 117.0],
                    chromium: [0.0; 4],
                    slack: SLACK,
                },
                ChromeBox {
                    name: "owner",
                    id: mv_clay::id("PlaneOwnerTools", 0),
                    webkit: [1350.0, 862.0, 0.0, 0.0],
                    chromium: [0.0; 4],
                    slack: SLACK,
                },
                ChromeBox {
                    name: "view",
                    id: mv_clay::id("PlaneViewControls", 0),
                    webkit: [1360.0, 745.0, 36.0, 117.0],
                    chromium: [0.0; 4],
                    slack: SLACK,
                },
                ChromeBox {
                    name: "origin",
                    id: mv_clay::id("PlaneOrigin", 0),
                    webkit: [1360.0, 745.0, 36.0, 36.0],
                    chromium: [0.0; 4],
                    slack: SLACK,
                },
                ChromeBox {
                    name: "origin_icon",
                    id: mv_clay::id("PlaneOriginIcon", 0),
                    webkit: [1368.0, 753.0, 20.0, 20.0],
                    chromium: [0.0; 4],
                    slack: SLACK,
                },
                ChromeBox {
                    name: "zoom",
                    id: mv_clay::id("PlaneZoom", 0),
                    webkit: [1360.0, 791.0, 36.0, 71.0],
                    chromium: [0.0; 4],
                    slack: SLACK,
                },
                ChromeBox {
                    name: "zoom_in",
                    id: mv_clay::id("PlaneZoomStep", 0),
                    webkit: [1361.0, 792.0, 34.0, 34.0],
                    chromium: [0.0; 4],
                    slack: SLACK,
                },
                // The − button is 35 tall with its 1 px top border: the rule,
                // then the 34 px square.
                ChromeBox {
                    name: "zoom_rule",
                    id: mv_clay::id("PlaneZoomRule", 0),
                    webkit: [1361.0, 826.0, 34.0, 1.0],
                    chromium: [0.0; 4],
                    slack: SLACK,
                },
                ChromeBox {
                    name: "zoom_out",
                    id: mv_clay::id("PlaneZoomStep", 1),
                    webkit: [1361.0, 827.0, 34.0, 34.0],
                    chromium: [0.0; 4],
                    slack: SLACK,
                },
            ],
        );
        // The caption's label sits on its own 16 px line, the maths on its 23
        // px one, 3.5 px above.
        {
            let found = app
                .clay
                .as_ref()
                .expect("clay context initialised")
                .get_element_data(mv_clay::id("PlaneCaptionLine", 0))
                .found;
            if found {
                assert_eq!(
                    mv_text::line_height_normal(&mut app.fonts, mv_text::Face::WorkSans600, 14.0),
                    16.0
                );
                assert_eq!(
                    mv_text::line_height_normal(&mut app.fonts, mv_text::Face::Stix400, 18.0),
                    23.0
                );
            }
        }

        // Transformed: R3 in Midnight Violet, the label tracked 1.12 px, the
        // Original toggle on.
        app.prefs = mv_theme::Prefs {
            mode: mv_theme::Mode::Dark,
            dark: mv_theme::Slug::MidnightViolet,
            light: mv_theme::Slug::CleanTeal,
        };
        app.viewer.params = enum_map::EnumMap::from_array([2.0, 1.0, 1.0, -1.0]);
        app.viewer.p_x = 0.5;
        let size = app.size;
        let _ = crate::draw::settle_layout(&mut app, size);
        check_boxes(
            &mut app,
            "transformed",
            &[
                ChromeBox {
                    name: "caption",
                    id: mv_clay::id("PlaneCaption", 0),
                    webkit: [516.0, 114.0, 139.77, 35.0],
                    chromium: [0.0; 4],
                    slack: TEXT_SLACK,
                },
                ChromeBox {
                    name: "tools",
                    id: mv_clay::id("PlaneTools", 0),
                    webkit: [1189.14, 745.0, 206.86, 117.0],
                    chromium: [1189.12, 745.0, 206.88, 117.0],
                    slack: TEXT_SLACK,
                },
                ChromeBox {
                    name: "owner",
                    id: mv_clay::id("PlaneOwnerTools", 0),
                    webkit: [1189.14, 826.0, 160.86, 36.0],
                    chromium: [1189.12, 826.0, 160.88, 36.0],
                    slack: TEXT_SLACK,
                },
                ChromeBox {
                    name: "toggle",
                    id: mv_clay::id("GhostToggle", 0),
                    webkit: [1189.14, 826.0, 160.86, 36.0],
                    chromium: [1189.12, 826.0, 160.88, 36.0],
                    slack: TEXT_SLACK,
                },
                ChromeBox {
                    name: "swatch",
                    id: mv_clay::id("GhostSwatch", 0),
                    webkit: [1203.14, 842.0, 22.0, 4.0],
                    chromium: [1203.12, 842.0, 22.0, 4.0],
                    slack: TEXT_SLACK,
                },
                ChromeBox {
                    name: "toggle_math",
                    id: mv_clay::id("GhostMath", 0),
                    webkit: [1294.14, 833.5, 41.86, 21.0],
                    chromium: [1294.12, 833.5, 41.88, 21.0],
                    slack: TEXT_SLACK,
                },
                ChromeBox {
                    name: "view",
                    id: mv_clay::id("PlaneViewControls", 0),
                    webkit: [1360.0, 745.0, 36.0, 117.0],
                    chromium: [0.0; 4],
                    slack: SLACK,
                },
            ],
        );
        crate::ui::release_clay(&mut app);
        mv_text::destroy(&mut app.fonts);
        app.input.regions.clear();
        app.input.stops.clear();
        app.input.declared.clear();
        app.plane.hits.clear();
    }
}
