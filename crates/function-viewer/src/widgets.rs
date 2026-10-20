//! Small widgets: note text, custom payloads, segmented controls, outlines.
//!
//! Ported from `widgets.odin`, plus `draw_outline` (which the Odin app also
//! keeps here) and the GL-free parts of layout declaration. Declaring goes
//! through the private `declare` shims at the bottom: they take no `App` so
//! the call sites do not care whether the Clay context ends up global (as
//! the C library keeps it) or on `App` (which would forbid the `region`
//! calls nested inside `declare` children).

use crate::app::App;
use crate::effects::{draw_ring, grow_box};
use crate::icons::{Icon, IconKind, draw_icon};
use crate::input::{Cursor, Hit, note_text, region};
use crate::track::draw_dashed_ring;
use crate::ui::{current_theme, ink};
use mv_sys::clay;
use mv_theme::Token;

/// A laid-out line of text as a Custom payload.
///
/// Ported from `widgets.odin:49` (`Typeset`).
#[derive(Clone, Debug)]
pub struct Typeset {
    /// The laid-out line to paint.
    pub line: mv_text::Line,
    /// Whether the line participates in text selection.
    pub selectable: bool,
}

/// The app's custom element payloads, one per custom widget.
///
/// Ported from `widgets.odin:21` (`Custom`).
#[derive(Clone, Debug)]
pub enum Custom {
    /// A plane canvas.
    PlaneCanvas(crate::plane_area::PlaneCanvas),
    /// A plane scale bar.
    ScaleBar(crate::plane_area::ScaleBar),
    /// A laid-out line of text.
    Typeset(Typeset),
    /// A ghost swatch in the sidebar.
    GhostSwatch(crate::plane_area::GhostSwatch),
    /// A laid-out line for text permits.
    TextLine(crate::typography::TextLine),
    /// A mode icon in the header.
    Icon(Icon),
    /// A laid-out equation.
    Equation(crate::equation::Equation),
    /// A number box.
    NumberBox(crate::number_box::NumberBox),
    /// A slider track.
    Track(crate::track::Track),
    /// A point mark on the plane.
    PointMark(PointMark),
    /// A flip glyph in the equation.
    FlipGlyph(FlipGlyph),
    /// A ruler tape.
    RulerTape(crate::ruler_tape::RulerTape),
    /// Settings tip items.
    TipItems(crate::tip::TipItems),
    /// A settings mode badge.
    ModeBadge(crate::sidebar::ModeBadge),
    /// A settings reset glyph.
    ResetGlyph(crate::sidebar::ResetGlyph),
    /// A settings readout.
    Readout(crate::point_rows::Readout),
    /// A term tip in the equation.
    TermTip(crate::equation::TermTip),
    /// A triangle bullet.
    Triangle(crate::point_rows::Triangle),
    /// An explainer callout caret.
    CalloutCaret(crate::explainer::CalloutCaret),
    /// An explainer plot.
    ExplainerPlot(crate::explainer::ExplainerPlot),
    /// An explainer snippet.
    ExplainerSnippet(crate::explainer::ExplainerSnippet),
    /// A settings swatch.
    Swatch(crate::settings::Swatch),
    /// A focus outline.
    Outline(Outline),
}

/// Paints every placed run of a note line.
///
/// Ported from `widgets.odin:55` (`note_line`).
pub fn note_line(app: &mut App, line: &mv_text::Line, x: f32, y: f32) {
    for p in &line.placed {
        note_text(
            app,
            clay::BoundingBox {
                x: x + p.x,
                y: y + p.top,
                width: p.width,
                height: p.height,
            },
        );
    }
}

/// Packs a Custom payload for the layout's user data.
///
/// The payload moves into a frame-lived store and the config points at the
/// stored clone, so callers can pass temporaries: the Odin code
/// temp-allocates a clone the same way. `draw_frame` frees the store before
/// the next layout. Each payload is separately boxed, so pushing more never
/// moves a payload an earlier config points at.
///
/// Ported from `widgets.odin:61` (`custom_data`).
pub fn custom_data(c: Custom) -> clay::CustomElementConfig {
    CUSTOMS.with(|store| {
        let mut store = store.borrow_mut();
        store.push(Box::new(c));
        let last = store.last().expect("payload just pushed");
        clay::CustomElementConfig {
            customData: &**last as *const Custom as *mut std::ffi::c_void,
        }
    })
}

// The frame-lived Custom payloads `custom_data` keeps, freed by
// `draw_frame` before the next layout.
std::thread_local! {
    // Each payload is separately boxed so pushing more never moves a payload
    // an earlier config points at; the lint's plain `Vec` would reallocate.
    #[allow(clippy::vec_box)]
    static CUSTOMS: std::cell::RefCell<Vec<Box<Custom>>> = const {
        std::cell::RefCell::new(Vec::new())
    };
}

/// Frees the frame-lived Custom payloads before the next layout.
pub(crate) fn clear_customs() {
    CUSTOMS.with(|store| store.borrow_mut().clear());
}

/// Paints whatever custom payload a Custom command carries.
///
/// Ported from `widgets.odin:65` (`draw_custom`).
pub fn draw_custom(app: &mut App, _box: clay::BoundingBox, c: &Custom, radius: clay::CornerRadius) {
    let b = [_box.x, _box.y, _box.width, _box.height];
    match c {
        Custom::PlaneCanvas(_) => crate::plane_area::draw_plane(app, _box, radius),
        Custom::ScaleBar(v) => crate::plane_area::draw_scale_bar(&mut app.vg, _box, *v),
        Custom::Typeset(v) => {
            mv_text::draw_line(&mut app.fonts, &mut app.vg, &v.line, _box.x, _box.y);
            if v.selectable {
                note_line(app, &v.line, _box.x, _box.y);
            }
        }
        Custom::GhostSwatch(v) => crate::plane_area::draw_ghost_swatch(&mut app.vg, _box, *v),
        Custom::TextLine(v) => crate::typography::draw_text_line(app, _box, v),
        Custom::Icon(v) => draw_icon(&mut app.vg, _box, *v),
        Custom::Equation(v) => crate::equation::draw_equation(app, b, v),
        Custom::NumberBox(v) => crate::number_box::draw_number_box(app, b, v),
        Custom::Track(v) => crate::track::draw_track(app, b, v),
        Custom::PointMark(v) => {
            let theme = current_theme(app).clone();
            draw_point_mark(
                &mut app.vg,
                [_box.x + v.size / 2.0, _box.y + v.size / 2.0],
                *v,
                &theme,
            );
        }
        Custom::FlipGlyph(v) => draw_flip_glyph(&mut app.vg, [_box.x, _box.y], *v),
        Custom::RulerTape(v) => crate::ruler_tape::draw_ruler_tape(app, _box, v),
        Custom::TipItems(v) => crate::tip::draw_tip_items(app, _box, v),
        Custom::ModeBadge(v) => crate::sidebar::draw_mode_badge(app, _box, v),
        Custom::ResetGlyph(v) => {
            crate::sidebar::draw_reset_glyph(&mut app.vg, [_box.x, _box.y], *v)
        }
        Custom::Readout(v) => crate::point_rows::draw_readout(app, _box, v),
        Custom::TermTip(v) => crate::equation::draw_term_tip(app, _box, v),
        Custom::Triangle(v) => crate::point_rows::draw_triangle(&mut app.vg, _box, *v),
        Custom::CalloutCaret(v) => crate::explainer::draw_callout_caret(&mut app.vg, *v),
        Custom::ExplainerPlot(v) => crate::explainer::draw_explainer_plot(app, _box, *v),
        Custom::ExplainerSnippet(v) => crate::explainer::draw_explainer_snippet(app, _box, v),
        Custom::Swatch(v) => crate::settings::draw_swatch(app, _box, *v),
        Custom::Outline(v) => draw_outline(&mut app.vg, v),
    }
}

/// Counts the runes in a string.
///
/// Ported from `widgets.odin:121` (`rune_count`).
pub fn rune_count(s: &str) -> i64 {
    s.chars().count() as i64
}

/// Lays out one math run at the given size, inked in one colour.
///
/// Ported from `widgets.odin:127` (`math_line`).
pub fn math_line(
    fonts: &mut mv_text::Fonts,
    s: &str,
    size: f32,
    line_height: f32,
    ink: mv_theme::Color,
) -> mv_text::Line {
    let runs = crate::math_text::math_runs(s);
    let mut pieces = Vec::with_capacity(runs.len());
    for run in runs {
        pieces.push(mv_text::Piece {
            style: mv_text::Style {
                face: crate::math_text::math_face(&run),
                size,
                tracking: 0.0,
                features: mv_text::Features::new(),
            },
            text: run.text,
            ink,
            line_height,
            r#box: mv_text::Box::Exact,
            min_width: 0.0,
            align: mv_text::Align::Left,
            pad: [0.0, 0.0],
        });
    }
    mv_text::typeset(fonts, &pieces, 0.0, mv_text::Vertical::Baseline, 1.0)
}

/// The line re-inked in one colour.
///
/// Ported from `widgets.odin:147` (`inked`).
pub fn inked(line: mv_text::Line, color: mv_theme::Color) -> mv_text::Line {
    let mut out = line;
    for p in &mut out.placed {
        p.ink = color;
    }
    out
}

/// The icon diameter inside a segmented option.
///
/// Ported from `widgets.odin:166` (`SEGMENT_ICON`).
pub const SEGMENT_ICON: f32 = 16.0;

/// The gap between a segmented option's icon and its label.
///
/// Ported from `widgets.odin:167` (`SEGMENT_ICON_GAP`).
pub const SEGMENT_ICON_GAP: f32 = 6.0;

/// One segmented-control option.
///
/// Ported from `widgets.odin:159` (`Segment`).
#[derive(Clone, Debug)]
pub struct Segment {
    /// The label line, in the option's resting ink.
    pub line: mv_text::Line,
    /// The option's leading icon, if it has one.
    pub icon: Option<IconKind>,
    /// The hit selecting this option.
    pub hit: Hit,
    /// Whether this option is the selected one.
    pub picked: bool,
}

/// How a segmented control shows the keyboard's focus on an option.
///
/// Ported from `widgets.odin:170` (`Segment_Focus`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SegmentFocus {
    /// No outline here: the page's 3 px ring draws over the frame
    /// (`draw_overlays`).
    Ring,
    /// A 2 px `--primary` line 2 px outside the focused option's element,
    /// over its neighbours (the settings panel's toggle).
    Line,
}

/// A segmented control: one rounded card per option.
///
/// Ported from `widgets.odin:180` (`segmented_control`).
pub fn segmented_control(
    app: &mut App,
    t: &mv_theme::Theme,
    name: &str,
    options: &[Segment],
    focus: SegmentFocus,
) {
    let radius = t.card_radius;
    let inner = (radius - 1.0).max(0.0);
    let mut root = blank_element();
    root.layout = clay::LayoutConfig {
        sizing: clay::Sizing {
            width: mv_clay::sizing_grow(clay::SizingConstraintsMinMax { min: 0.0, max: 0.0 }),
            height: mv_clay::sizing_fixed(36.0),
        },
        padding: mv_clay::padding_all(1),
        childGap: 0,
        childAlignment: clay::ChildAlignment {
            x: clay::LayoutAlignmentX::Left,
            y: clay::LayoutAlignmentY::Top,
        },
        layoutDirection: clay::LayoutDirection::LeftToRight,
    };
    root.backgroundColor = ink(t, Token::Card);
    root.cornerRadius = mv_clay::corner_radius_all(radius);
    root.border = clay::BorderElementConfig {
        color: ink(t, Token::Border),
        width: mv_clay::border_outside(1),
    };
    let root_id = mv_clay::id(name, 0);
    declare(root_id, &root, || {
        let live = !app.input.inert;
        let any_picked = options.iter().any(|o| o.picked);
        let mut focused: Option<Outline> = None;
        for (i, o) in options.iter().enumerate() {
            if i > 0 {
                let mut divider = blank_element();
                divider.layout = clay::LayoutConfig {
                    sizing: clay::Sizing {
                        width: mv_clay::sizing_fixed(1.0),
                        height: mv_clay::sizing_grow(clay::SizingConstraintsMinMax {
                            min: 0.0,
                            max: 0.0,
                        }),
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
                divider.backgroundColor = ink(t, Token::Border);
                declare(
                    mv_clay::id(&format!("{name}Divider"), i as u32),
                    &divider,
                    || {},
                );
            }
            let hot = live
                && (app.input.hover == o.hit
                    || (app.input.focus_visible && app.input.focus == o.hit));
            let mut fill = [0.0; 4];
            if o.picked {
                fill = ink(t, Token::Primary);
            } else if live && app.input.pressed == o.hit {
                let plate = mv_theme::plate(t, Token::Primary);
                fill = [
                    plate[0] * 255.0,
                    plate[1] * 255.0,
                    plate[2] * 255.0,
                    plate[3] * 255.0,
                ];
            } else if hot {
                fill = ink(t, Token::Secondary);
            }
            let label_ink = if o.picked {
                t.colors[Token::PrimaryForeground]
            } else {
                t.colors[Token::Foreground]
            };
            let mut corners = clay::CornerRadius {
                topLeft: 0.0,
                topRight: 0.0,
                bottomLeft: 0.0,
                bottomRight: 0.0,
            };
            if i == 0 {
                corners.topLeft = inner;
                corners.bottomLeft = inner;
            }
            if i == options.len() - 1 {
                corners.topRight = inner;
                corners.bottomRight = inner;
            }
            let option_id = mv_clay::id(&format!("{name}Option"), i as u32);
            let mut option = blank_element();
            option.layout = clay::LayoutConfig {
                sizing: clay::Sizing {
                    width: mv_clay::sizing_grow(clay::SizingConstraintsMinMax {
                        min: 0.0,
                        max: 0.0,
                    }),
                    height: mv_clay::sizing_grow(clay::SizingConstraintsMinMax {
                        min: 0.0,
                        max: 0.0,
                    }),
                },
                padding: clay::Padding {
                    left: 0,
                    right: 0,
                    top: 0,
                    bottom: 0,
                },
                childGap: 0,
                childAlignment: clay::ChildAlignment {
                    x: clay::LayoutAlignmentX::Center,
                    y: clay::LayoutAlignmentY::Center,
                },
                layoutDirection: clay::LayoutDirection::LeftToRight,
            };
            option.backgroundColor = fill;
            option.cornerRadius = corners;
            let stop = o.picked || (!any_picked && i == 0);
            declare(option_id, &option, || {
                region(
                    app,
                    option_id,
                    o.hit,
                    Cursor::Pointer,
                    Cursor::Default,
                    stop,
                    corners,
                );
                let line = inked(o.line.clone(), label_ink);
                let (text_w, text_h) = (line.width, line.height);
                let mut label = blank_element();
                // Sizing is FIT, as the Odin label sets no sizing: only the
                // gap and the vertical centring are its own.
                label.layout = clay::LayoutConfig {
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
                    padding: clay::Padding {
                        left: 0,
                        right: 0,
                        top: 0,
                        bottom: 0,
                    },
                    childGap: SEGMENT_ICON_GAP as u16,
                    childAlignment: clay::ChildAlignment {
                        x: clay::LayoutAlignmentX::Left,
                        y: clay::LayoutAlignmentY::Center,
                    },
                    layoutDirection: clay::LayoutDirection::LeftToRight,
                };
                let label_id = mv_clay::id(&format!("{name}Label"), i as u32);
                declare(label_id, &label, || {
                    if let Some(kind) = o.icon {
                        let mut icon = blank_element();
                        icon.layout = clay::LayoutConfig {
                            sizing: clay::Sizing {
                                width: mv_clay::sizing_fixed(SEGMENT_ICON),
                                height: mv_clay::sizing_fixed(SEGMENT_ICON),
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
                        icon.custom = custom_data(Custom::Icon(Icon {
                            kind,
                            ink: label_ink,
                        }));
                        declare(mv_clay::id(&format!("{name}Icon"), i as u32), &icon, || {});
                    }
                    let mut text = blank_element();
                    text.custom = custom_data(Custom::Typeset(Typeset {
                        line,
                        selectable: false,
                    }));
                    text.layout = clay::LayoutConfig {
                        sizing: clay::Sizing {
                            width: mv_clay::sizing_fixed(text_w),
                            height: mv_clay::sizing_fixed(text_h),
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
                    declare(mv_clay::id(&format!("{name}Text"), i as u32), &text, || {});
                    if focus == SegmentFocus::Line
                        && live
                        && app.input.focus_visible
                        && app.input.focus == o.hit
                    {
                        // The focused option's line, over its neighbours:
                        // drawn after them, from the option's box once laid
                        // out. `of` is the option element, not the label.
                        focused = Some(Outline {
                            of: option_id,
                            offset: OUTLINE_OFFSET,
                            width: OUTLINE_WIDTH,
                            ink: t.colors[Token::Primary],
                            radius: corners,
                        });
                    }
                });
            });
        }
        if let Some(line) = focused {
            outline(&line);
        }
    });
}

/// Moves the radio focus one option in the arrow-key direction.
///
/// Ported from `widgets.odin:279` (`radio_target`).
pub fn radio_target(key: sdl3::keyboard::Keycode, from: i64, count: i64) -> (i64, bool) {
    use sdl3::keyboard::Keycode;
    let to = match key {
        Keycode::Left | Keycode::Up => from - 1,
        Keycode::Right | Keycode::Down => from + 1,
        Keycode::Home => 0,
        Keycode::End => count - 1,
        _ => return (0, false),
    };
    ((to % count + count) % count, true)
}

/// A focus outline: a ring around a laid-out element.
///
/// Ported from `widgets.odin:299` (`Outline`).
#[derive(Clone, Copy, Debug)]
pub struct Outline {
    /// The element to ring.
    pub of: clay::ElementId,
    /// How far outside the element the ring sits.
    pub offset: f32,
    /// The ring's width.
    pub width: f32,
    /// The ring's colour.
    pub ink: mv_theme::Color,
    /// The ring's corner radii.
    pub radius: clay::CornerRadius,
}

/// How far outside its element an outline ring sits.
///
/// Ported from `widgets.odin:310` (`OUTLINE_OFFSET`).
pub const OUTLINE_OFFSET: f32 = 2.0;

/// An outline ring's width.
///
/// Ported from `widgets.odin:309` (`OUTLINE_WIDTH`).
pub const OUTLINE_WIDTH: f32 = 2.0;

/// Declares a zero-size custom element carrying a focus outline.
///
/// Ported from `widgets.odin:312` (`outline`).
pub fn outline(o: &Outline) {
    let mut config = blank_element();
    config.custom = custom_data(Custom::Outline(*o));
    declare_auto(&config, || {});
}

/// Paints a focus outline around its element's layout box.
///
/// Ported from `widgets.odin:316` (`draw_outline`).
pub fn draw_outline(vg: &mut mv_nvg::Context, o: &Outline) {
    let data = element_data(o.of);
    if !data.found {
        return;
    }
    fn moved(r: f32, by: f32) -> f32 {
        if r > 0.0 { (r + by).max(0.0) } else { 0.0 }
    }
    let inner = clay::CornerRadius {
        topLeft: moved(o.radius.topLeft, o.offset),
        topRight: moved(o.radius.topRight, o.offset),
        bottomLeft: moved(o.radius.bottomLeft, o.offset),
        bottomRight: moved(o.radius.bottomRight, o.offset),
    };
    draw_ring(
        vg,
        grow_box(data.boundingBox, o.offset),
        inner,
        o.width,
        o.ink,
    );
}

/// A double-arrow glyph flipping between the horizontal and vertical.
///
/// Ported from `widgets.odin:336` (`Flip_Glyph`).
#[derive(Clone, Copy, Debug)]
pub struct FlipGlyph {
    /// Whether the arrows run vertically.
    pub vertical: bool,
    /// The glyph's ink.
    pub ink: mv_nvg::Color,
}

/// Paints a flip glyph at the tile origin.
///
/// Ported from `widgets.odin:341` (`draw_flip_glyph`).
pub fn draw_flip_glyph(vg: &mut mv_nvg::Context, at: [f32; 2], g: FlipGlyph) {
    vg.save();
    vg.translate(at[0], at[1]);
    vg.begin_path();
    let _ = crate::svgpath::draw_path(vg, if g.vertical { "M7 2V12" } else { "M2 7H12" });
    vg.stroke_color(g.ink);
    vg.stroke_width(1.6);
    vg.line_cap(mv_nvg::LineCapType::ROUND);
    vg.stroke();
    vg.begin_path();
    let _ = crate::svgpath::draw_path(
        vg,
        if g.vertical {
            "M3.5 5L7 0.5L10.5 5ZM3.5 9L7 13.5L10.5 9Z"
        } else {
            "M5 3.5L0.5 7L5 10.5ZM9 3.5L13.5 7L9 10.5Z"
        },
    );
    vg.fill_color(g.ink);
    vg.fill();
    vg.restore();
}

/// A point mark on the plane.
///
/// Ported from `widgets.odin:365` (`Point_Mark`).
#[derive(Clone, Copy, Debug)]
pub struct PointMark {
    /// The mark's diameter.
    pub size: f32,
    /// The mark's kind.
    pub kind: PointMarkKind,
    /// Whether the mark sits on a parked thumb.
    pub parked: bool,
    /// The ring ink.
    pub ink: mv_theme::Color,
}

/// Paints a point mark centred on the tile.
///
/// Ported from `widgets.odin:375` (`draw_point_mark`).
pub fn draw_point_mark(
    vg: &mut mv_nvg::Context,
    centre: [f32; 2],
    m: PointMark,
    t: &mv_theme::Theme,
) {
    let c = m.size / 2.0;
    let ring_w: f32 = if m.kind == PointMarkKind::Ring && !m.parked {
        2.5
    } else {
        2.0
    };
    let bg = t.colors[Token::Background];
    vg.begin_path();
    vg.circle(centre[0], centre[1], c - ring_w / 2.0);
    vg.fill_color(bg);
    vg.fill();
    if m.parked {
        draw_dashed_ring(vg, centre, c - ring_w / 2.0, ring_w, m.ink);
        return;
    }
    vg.stroke_color(if m.kind == PointMarkKind::Bullseye {
        t.colors[Token::Foreground]
    } else {
        m.ink
    });
    vg.stroke_width(ring_w);
    vg.stroke();
    vg.begin_path();
    vg.circle(
        centre[0],
        centre[1],
        if m.kind == PointMarkKind::Bullseye {
            m.size / 4.0
        } else {
            m.size / 8.0
        },
    );
    vg.fill_color(if m.kind == PointMarkKind::Bullseye {
        m.ink
    } else {
        t.colors[Token::Foreground]
    });
    vg.fill();
}

/// A point mark's kind: a filled bullseye for a pinned point, a ring probe.
///
/// Ported from `widgets.odin:365` (`Point_Mark.kind`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointMarkKind {
    /// A pinned point's bullseye.
    Bullseye,
    /// A probe's ring.
    Ring,
}

/// A blank element declaration: every site fills what it needs.
///
/// Odin's Clay literals leave unset fields zero; Rust has no `Default` for
/// these configs, so the zero stands here once instead of at each site.
/// Sizing's zero is FIT (`SizingType::Fit = 0`), so an element that sets no
/// sizing fits its contents, as the Odin label wrapper and `outline` do.
fn blank_element() -> clay::ElementDeclaration {
    clay::ElementDeclaration {
        layout: clay::LayoutConfig {
            sizing: clay::Sizing {
                width: mv_clay::sizing_fit(clay::SizingConstraintsMinMax { min: 0.0, max: 0.0 }),
                height: mv_clay::sizing_fit(clay::SizingConstraintsMinMax { min: 0.0, max: 0.0 }),
            },
            padding: mv_clay::padding_all(0),
            childGap: 0,
            childAlignment: clay::ChildAlignment {
                x: clay::LayoutAlignmentX::Left,
                y: clay::LayoutAlignmentY::Top,
            },
            layoutDirection: clay::LayoutDirection::LeftToRight,
        },
        backgroundColor: [0.0; 4],
        border: clay::BorderElementConfig {
            color: [0.0; 4],
            width: mv_clay::border_outside(0),
        },
        userData: std::ptr::null_mut(),
        clip: clay::ClipElementConfig {
            horizontal: false,
            vertical: false,
            childOffset: [0.0, 0.0],
        },
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
        aspectRatio: clay::AspectRatioElementConfig { aspectRatio: 0.0 },
        cornerRadius: clay::CornerRadius {
            topLeft: 0.0,
            topRight: 0.0,
            bottomLeft: 0.0,
            bottomRight: 0.0,
        },
        overlayColor: [0.0; 4],
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
                siblingOrdering:
                    clay::ExitTransitionSiblingOrdering::UnderneathSiblings,
            },
        },
    }
}

/// Declares one element with its children.
///
/// Ported from `widgets.odin`'s `UI(id)(config) { ... }` scopes: the open,
/// configure, children, close scope through `mv-clay`'s free functions on
/// Clay's current context (which the layout left open).
//
// Takes no `App` (see the module docs); `children` runs synchronously, so
// by-value borrows of the caller's locals are sound for the declare
// itself, though the payloads Clay keeps must outlive the frame.
fn declare(id: clay::ElementId, config: &clay::ElementDeclaration, children: impl FnOnce()) {
    mv_clay::open_element(id);
    mv_clay::configure_open_element(config);
    children();
    mv_clay::close_element();
}

/// Declares one element with an automatic id.
///
/// Ported from `widgets.odin`'s `UI()(config) { ... }` scopes: as in
/// `declare`, through the anonymous open.
fn declare_auto(config: &clay::ElementDeclaration, children: impl FnOnce()) {
    mv_clay::open_element_auto();
    mv_clay::configure_open_element(config);
    children();
    mv_clay::close_element();
}

/// Clay's layout data for an element id.
///
/// Ported from `widgets.odin`'s layout reads (`GetElementData`): through
/// `mv-clay`'s free function on the laid-out frame. Takes no `App` (see
/// the module docs).
fn element_data(id: clay::ElementId) -> clay::ElementData {
    mv_clay::element_data(id)
}
