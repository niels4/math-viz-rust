//! The scrub-mode ruler popover: the tape under the row while dragging.
//!
//! While a number box scrubs, a floating card shows the ruler tape with ticks,
//! the home mark, the index and the value labels.

use mv_sys::clay;

use crate::app::App;

/// The popover's stacking layer, gap, padding and radius.
pub const ROW_POPOVER_Z: i16 = 3;
pub const RULER_GAP: f32 = 4.0;
pub const RULER_PADDING: f32 = 8.0;
pub const RULER_RADIUS: f32 = 10.0;

/// The ruler tape: whose lattice, at what value, in what colours.
#[derive(Clone, Copy, Debug)]
pub struct RulerTape {
    pub kind: mv_scrub::Kind,
    pub value: f64,
    pub fill: mv_theme::Color,
    pub stroke: mv_theme::Color,
    pub index: mv_theme::Color,
    pub ink: mv_theme::Color,
    pub radius: f32,
}

/// Declares the floating ruler card with the tape under the row.
pub fn ruler_popover(
    app: &mut App,
    t: &mv_theme::Theme,
    p: mv_viewer::Param,
    mode: mv_scrub::Mode,
) {
    let id = mv_clay::id("Ruler", p as u32);
    let layout = clay::LayoutConfig {
        sizing: clay::Sizing {
            width: crate::tip::grow(),
            height: crate::tip::fit(),
        },
        padding: clay::Padding {
            left: (RULER_PADDING + 1.0) as u16,
            right: (RULER_PADDING + 1.0) as u16,
            top: (RULER_PADDING + 1.0) as u16,
            bottom: (RULER_PADDING + 1.0) as u16,
        },
        childGap: 0,
        childAlignment: clay::ChildAlignment {
            x: clay::LayoutAlignmentX::Left,
            y: clay::LayoutAlignmentY::Top,
        },
        layoutDirection: clay::LayoutDirection::LeftToRight,
    };
    let mut config = crate::tip::blank_declaration(layout);
    config.backgroundColor = crate::ui::ink(t, mv_theme::Token::Card);
    config.border = clay::BorderElementConfig {
        color: crate::ui::ink(t, mv_theme::Token::Border),
        width: clay::BorderWidth {
            left: 1,
            right: 1,
            top: 1,
            bottom: 1,
            betweenChildren: 0,
        },
    };
    config.cornerRadius = clay::CornerRadius {
        topLeft: RULER_RADIUS,
        topRight: RULER_RADIUS,
        bottomLeft: RULER_RADIUS,
        bottomRight: RULER_RADIUS,
    };
    config.floating = clay::FloatingElementConfig {
        offset: [0.0, RULER_GAP],
        expand: clay::Dimensions {
            width: 0.0,
            height: 0.0,
        },
        parentId: 0,
        zIndex: ROW_POPOVER_Z,
        attachment: clay::FloatingAttachPoints {
            element: clay::FloatingAttachPointType::LeftTop,
            parent: clay::FloatingAttachPointType::LeftBottom,
        },
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
    let mut stroke = t.colors[crate::sidebar::PARAM_INKS[p]];
    match mode {
        mv_scrub::Mode::Fine => stroke = t.colors[mv_theme::Token::Accent],
        mv_scrub::Mode::Snap => stroke = t.colors[mv_theme::Token::Warning],
        mv_scrub::Mode::Coarse => {}
    }
    let tape = RulerTape {
        kind: crate::number_box::scrub_kind(p),
        value: app.viewer.params[p],
        fill: t.colors[crate::sidebar::PARAM_BANDS[p]],
        stroke,
        index: t.colors[crate::sidebar::PARAM_INKS[p]],
        ink: t.colors[mv_theme::Token::Foreground],
        radius: t.card_radius,
    };
    let inner = mv_clay::id("Tape", p as u32);
    let inner_layout = clay::LayoutConfig {
        sizing: clay::Sizing {
            width: crate::tip::grow(),
            height: crate::tip::fixed(mv_scrub::RULER_HEIGHT as f32),
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
    let inner_payload = crate::widgets::Custom::RulerTape(tape);
    let mut inner_config = crate::tip::blank_declaration(inner_layout);
    inner_config.custom = crate::widgets::custom_data(inner_payload);
    crate::ui::ui_scope(app, id, &config, |app| {
        crate::ui::ui_scope(app, inner, &inner_config, |_| {});
    });
}

/// The tape labels' style and line.
pub static RULER_LABEL: std::sync::LazyLock<mv_text::Style> =
    std::sync::LazyLock::new(|| mv_text::Style {
        face: mv_text::Face::RobotoMono500,
        size: 12.0,
        tracking: 0.0,
        features: enumset::EnumSet::empty(),
    });
/// The tape labels' line height.
pub const RULER_LABEL_LINE: f32 = 16.0;

/// Draws the band, ticks, home, index, chevrons, labels and stroke.
pub fn draw_ruler_tape(app: &mut App, r#box: clay::BoundingBox, tape: &RulerTape) {
    let w = f64::from(r#box.width);
    let c = w / 2.0;
    let radii = crate::draw::css_radii(
        r#box,
        clay::CornerRadius {
            topLeft: tape.radius,
            topRight: tape.radius,
            bottomLeft: tape.radius,
            bottomRight: tape.radius,
        },
    );
    let r = radii.topLeft;
    app.vg.begin_path();
    crate::draw::rounded_rect(&mut app.vg, r#box, radii);
    app.vg.fill_color(crate::draw::css_background(tape.fill));
    app.vg.fill();

    let marks = mv_scrub::ruler_marks(tape.kind, tape.value, w);
    app.vg.stroke_color(tape.ink);
    app.vg.line_cap(mv_nvg::LineCapType::BUTT);
    for x in &marks.minor {
        ruler_tick(&mut app.vg, r#box, r, *x, mv_scrub::TICK_MINOR as f32, 1.0);
    }
    for x in &marks.mid {
        ruler_tick(&mut app.vg, r#box, r, *x, mv_scrub::TICK_MID as f32, 1.0);
    }
    for x in &marks.major {
        ruler_tick(&mut app.vg, r#box, r, *x, mv_scrub::TICK_MAJOR as f32, 1.5);
    }
    if let Some(home) = marks.home {
        let h = home as f32;
        app.vg.begin_path();
        app.vg.move_to(r#box.x + px_hundredths(h - 4.0), r#box.y);
        app.vg.line_to(r#box.x + px_hundredths(h + 4.0), r#box.y);
        app.vg.line_to(r#box.x + px_hundredths(h), r#box.y + 6.0);
        app.vg.close_path();
        app.vg.fill_color(tape.ink);
        app.vg.fill();
    }
    // The index, held: 3 px wide, full height, and its foot.
    let centre = r#box.x + c as f32;
    crate::draw::fill_rect(
        &mut app.vg,
        [centre - 1.5, r#box.y, 3.0, mv_scrub::RULER_HEIGHT as f32],
        tape.index,
    );
    app.vg.begin_path();
    app.vg
        .move_to(centre - 6.0, r#box.y + mv_scrub::RULER_HEIGHT as f32);
    app.vg
        .line_to(centre + 6.0, r#box.y + mv_scrub::RULER_HEIGHT as f32);
    app.vg
        .line_to(centre, r#box.y + mv_scrub::RULER_HEIGHT as f32 - 7.0);
    app.vg.close_path();
    app.vg.fill_color(tape.index);
    app.vg.fill();
    // Left and right chevrons at the ends.
    app.vg.begin_path();
    app.vg.move_to(r#box.x + 13.0, r#box.y + 11.0);
    app.vg.line_to(r#box.x + 8.0, r#box.y + 16.0);
    app.vg.line_to(r#box.x + 13.0, r#box.y + 21.0);
    app.vg.move_to(r#box.x + r#box.width - 13.0, r#box.y + 11.0);
    app.vg.line_to(r#box.x + r#box.width - 8.0, r#box.y + 16.0);
    app.vg.line_to(r#box.x + r#box.width - 13.0, r#box.y + 21.0);
    app.vg.stroke_color(tape.ink);
    app.vg.stroke_width(2.0);
    app.vg.line_cap(mv_nvg::LineCapType::ROUND);
    app.vg.line_join(mv_nvg::LineCapType::ROUND);
    app.vg.stroke();
    let baseline = r#box.y
        + mv_scrub::LABEL_TOP as f32
        + mv_text::baseline(
            &mut app.fonts,
            RULER_LABEL.face,
            RULER_LABEL.size,
            RULER_LABEL_LINE,
        );
    for label in &marks.labels {
        mv_text::draw(
            &mut app.fonts,
            &mut app.vg,
            *RULER_LABEL,
            r#box.x + label.left as f32,
            baseline,
            &label.text,
            tape.ink,
        );
    }
    // The stroke, drawn over the tape as the frame draws a stroke over its
    // children.
    crate::effects::draw_inset(&mut app.vg, r#box, radii, 2.0, tape.stroke);
}

/// A vertical tick from the top edge, its top cut where the tape's rounded
/// corner is.
fn ruler_tick(
    vg: &mut mv_nvg::Context,
    r#box: clay::BoundingBox,
    r: f32,
    at: f64,
    length: f32,
    width: f32,
) {
    let x = ((at * 100.0).round() / 100.0) as f32;
    let mut top = 0.0f32;
    let from_edge = x.min(r#box.width - x);
    if from_edge < r {
        let d = r - from_edge;
        top = r - (r * r - d * d).max(0.0).sqrt();
    }
    if top >= length {
        return;
    }
    vg.begin_path();
    vg.move_to(r#box.x + x, r#box.y + top);
    vg.line_to(r#box.x + x, r#box.y + length);
    vg.stroke_width(width);
    vg.stroke();
}

/// Rounds a position to hundredths of a px.
fn px_hundredths(v: f32) -> f32 {
    (v * 100.0).round() / 100.0
}
