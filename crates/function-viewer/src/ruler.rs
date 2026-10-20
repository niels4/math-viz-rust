//! The scrub ruler: the tape under a dragged value box.
//!
//! Ported from `SideBar/TransformSection/ScrubValue/Ruler.tsx` and
//! `Ruler.module.css` at `b8a0e1d`: while a box scrubs, a card as wide as its
//! row floats 4 px under the row, and the tape inside (the row's tint wash,
//! rimmed in the drag mode's colour) slides its marks and labels with the live
//! value under a centre index, with chevrons at its ends. The marks and label
//! texts come from [`crate::ruler_marks`] (h5-p-scrub); this module owns the
//! card's geometry, the mode's rim, and the tape's paint.
//!
//! Layout is explicit, as the source's hooks are: [`layout_ruler`] declares
//! the floating card (called while the drag holds it open), and [`paint_ruler`]
//! paints the tape's contents in a post-pass from the laid-out content box,
//! the live value and the mode (this repo's settings swatch precedent: no
//! custom payload type). The index paints over the labels, as the source
//! draws it (math-viz2's divergences note).

use mv_sys::clay;
use mv_text::{Face, Features, Style};

use crate::app::App;
use crate::ruler_marks::{RulerMarks, ruler_marks};
use crate::scrub::ScrubMode;
use crate::state::TransformParam;

// stub until h5-box: `App.scrub` owns this section (the port of the press and
// drag state in `ScrubValue.tsx`). Names follow the spawn (`ScrubDrag {
// param, kind, start_value, dx, mode, row_box }`); h5-box has written no code
// yet, so `kind` is a guess. Delete this section at the merge and read
// h5-box's `ScrubDrag` instead; the layout and paint below take explicit
// arguments, so only their caller adapts.

/// Whether a drag moves a size by ratio or a shift by addition (`isScale`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrubKind {
    Scale,
    Shift,
}

/// A box's drag in flight: which parameter, of what kind, from what value,
/// how far, in what mode, under which row.
#[derive(Clone, Copy, Debug)]
pub struct ScrubDrag {
    pub param: TransformParam,
    pub kind: ScrubKind,
    pub start_value: f64,
    pub dx: f64,
    pub mode: ScrubMode,
    pub row_box: clay::BoundingBox,
}

/// The card's gap under the row (`.ruler`: `top: calc(100% + 4px)`).
pub const RULER_GAP: f32 = 4.0;
/// The card's padding (`.ruler`: `padding: 8px`).
pub const RULER_PAD: f32 = 8.0;
/// The card's rim (`.ruler`: `1px solid var(--border)`).
pub const RULER_BORDER: f32 = 1.0;
/// The card's corners (`.ruler`: `var(--radius-card)`, 4 px in every theme).
pub const RULER_RADIUS: f32 = 4.0;
/// The card's stack level (`.ruler`: `z-index: 1`).
pub const RULER_Z: i16 = 1;

/// The tape's height, border box (`.tape`: `height: 32px`).
pub const TAPE_HEIGHT: f32 = 32.0;
/// The tape's rim (`.tape`: `2px solid`, the mode's colour).
pub const TAPE_BORDER: f32 = 2.0;
/// The tape's corners (`.tape`: `var(--radius-card)`).
pub const TAPE_RADIUS: f32 = 4.0;

/// The tick lengths (minor 3, mid 6, major 9: `TICK_LENGTH`).
pub const TICK_MINOR_LEN: f32 = 3.0;
pub const TICK_MID_LEN: f32 = 6.0;
pub const TICK_MAJOR_LEN: f32 = 9.0;
/// The tick widths (minor and mid 1, major 1.5).
pub const TICK_MINOR_WIDTH: f32 = 1.0;
pub const TICK_MID_WIDTH: f32 = 1.0;
pub const TICK_MAJOR_WIDTH: f32 = 1.5;

/// The labels' alphabetic baseline in tape-content px (`LABEL_BASELINE`).
pub const LABEL_BASELINE: f32 = 25.0;
/// The labels' face and size (`font: 500 12px var(--font-mono)`).
pub const LABEL_FONT_SIZE: f32 = 12.0;

/// The index bar's width (`INDEX.width`).
pub const INDEX_WIDTH: f32 = 3.0;
/// The index pointer's width and height below the tape's foot.
pub const POINTER_WIDTH: f32 = 8.6;
pub const POINTER_HEIGHT: f32 = 5.0;

/// The chevrons' inset from the tape's ends and their size (`CHEVRON`).
pub const CHEVRON_INSET: f32 = 6.0;
pub const CHEVRON_WIDTH: f32 = 5.0;
pub const CHEVRON_HEIGHT: f32 = 10.0;
/// The chevrons' stroke (`.chevrons`: `stroke-width: 2px`, round caps).
pub const CHEVRON_STROKE: f32 = 2.0;

/// Whether the parameter lays its tape out by ratio (`Ruler`'s `isScale`:
/// the `a` and `b` rows; `h` and `k` add).
#[must_use]
pub fn is_scale(param: TransformParam) -> bool {
    matches!(param, TransformParam::A | TransformParam::B)
}

/// Whose colour rims the tape (`.tape`'s border, and the box's while
/// dragging): the row's ink, or the accent while fine and the warning while
/// snap (`data-mode`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TapeStroke {
    RowInk,
    Accent,
    Warning,
}

/// The rim for a drag mode.
#[must_use]
pub fn tape_stroke(mode: ScrubMode) -> TapeStroke {
    match mode {
        ScrubMode::Coarse => TapeStroke::RowInk,
        ScrubMode::Fine => TapeStroke::Accent,
        ScrubMode::Snap => TapeStroke::Warning,
    }
}

/// A chevron's end: the tip sits at the tape's inset, the mouth opens inward.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChevronSide {
    Left,
    Right,
}

/// The tape's content width inside a row `row_width` wide: the card's border
/// and padding plus the tape's own border come off both sides (at 350: 328).
#[must_use]
pub fn tape_width(row_width: f32) -> f32 {
    row_width - 2.0 * (RULER_BORDER + RULER_PAD + TAPE_BORDER)
}

/// The card's height: its border and padding around the 32 px tape (at rest:
/// 50).
#[must_use]
pub fn ruler_height() -> f32 {
    TAPE_HEIGHT + 2.0 * (RULER_PAD + RULER_BORDER)
}

/// The tape border box's top-left inside the card (at rest: 9, 9).
#[must_use]
pub fn tape_origin() -> f32 {
    RULER_BORDER + RULER_PAD
}

/// The tape's content height: the tape inside its own border (at rest: 28).
#[must_use]
pub fn tape_content_height() -> f32 {
    TAPE_HEIGHT - 2.0 * TAPE_BORDER
}

/// The card's box under a row: as wide as the row, 4 px down (the probe's
/// `rulerSize` and `belowRow`).
#[must_use]
pub fn ruler_box(row: &clay::BoundingBox) -> clay::BoundingBox {
    clay::BoundingBox {
        x: row.x,
        y: row.y + row.height + RULER_GAP,
        width: row.width,
        height: ruler_height(),
    }
}

/// The tape's content box inside a laid-out card: past both borders and the
/// card's padding (the probe's svg box).
#[must_use]
pub fn tape_content_box(ruler: &clay::BoundingBox) -> clay::BoundingBox {
    let outside = RULER_BORDER + RULER_PAD + TAPE_BORDER;
    clay::BoundingBox {
        x: ruler.x + outside,
        y: ruler.y + outside,
        width: ruler.width - 2.0 * outside,
        height: tape_content_height(),
    }
}

/// The index bar's box over a tape content box: 3 px wide, full height, on
/// the centre (`INDEX`, `x = c - 1.5`).
#[must_use]
pub fn index_rect(centre: f32, height: f32) -> [f32; 4] {
    [centre - INDEX_WIDTH / 2.0, 0.0, INDEX_WIDTH, height]
}

/// The index pointer's triangle: 8.6 wide at the tape's foot, rising 5
/// (`M{c-4.3} {h}H{c+4.3}L{c} {h-5}Z`).
#[must_use]
pub fn pointer_triangle(centre: f32, height: f32) -> [[f32; 2]; 3] {
    let half = POINTER_WIDTH / 2.0;
    [
        [centre - half, height],
        [centre + half, height],
        [centre, height - POINTER_HEIGHT],
    ]
}

/// A chevron's three points: the open ends and the tip at the tape's middle
/// (`chevronPath`: `M{open} {mid-5}L{tip} {mid}L{open} {mid+5}`).
#[must_use]
pub fn chevron_points(width: f32, height: f32, side: ChevronSide) -> [[f32; 2]; 3] {
    let mid = height / 2.0;
    let half = CHEVRON_HEIGHT / 2.0;
    let (tip, open) = match side {
        ChevronSide::Left => (CHEVRON_INSET, CHEVRON_INSET + CHEVRON_WIDTH),
        ChevronSide::Right => (width - CHEVRON_INSET, width - CHEVRON_INSET - CHEVRON_WIDTH),
    };
    [[open, mid - half], [tip, mid], [open, mid + half]]
}

/// A tape position rounded to hundredths of a px, as the tick paths quantize
/// theirs (`x.toFixed(2)`; ties are unreachable on either lattice).
fn hundredths(x: f64) -> f32 {
    ((x * 100.0).round() / 100.0) as f32
}

/// The tape's marks for `value` in a row `row_width` wide: the measured tape
/// width plumbed into [`ruler_marks`] with the row's lattice.
#[must_use]
pub fn marks_for(value: f64, row_width: f32, param: TransformParam) -> RulerMarks {
    ruler_marks(value, f64::from(tape_width(row_width)), is_scale(param))
}

/// The labels' text style: the mono face at 12 px (`label_style`'s shape).
#[must_use]
pub fn label_style() -> Style {
    Style {
        face: Face::RobotoMono500,
        size: LABEL_FONT_SIZE,
        tracking: 0.0,
        features: Features::new(),
    }
}

/// The colours the ruler paints with, resolved by the caller: the card and
/// its rim (`Card`, `Border`), the tape's wash (the row's tint), the mode's
/// rim (the row's ink, `Accent`, `Warning`), the index (the row's ink) and
/// the ticks, labels and chevrons (`Foreground`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RulerColors {
    pub card: [f32; 4],
    pub rim: [f32; 4],
    pub fill: [f32; 4],
    pub stroke: [f32; 4],
    pub index: [f32; 4],
    pub ink: [f32; 4],
}

/// The ruler's colours from the resolved theme, the parameter and the drag
/// mode: the row's ink is `param.colour_token()`, its wash `theme::band` (the
/// pre-blended 18 % tint, as the lit rows paint it; the probe shows the
/// resolved wash as the ink's own oklch triple at `/ 0.18`).
#[must_use]
pub fn ruler_colors(
    theme: &mv_theme::Theme,
    param: TransformParam,
    mode: ScrubMode,
) -> RulerColors {
    let param_ink = theme.colors[param.colour_token()];
    let stroke = match tape_stroke(mode) {
        TapeStroke::RowInk => param_ink,
        TapeStroke::Accent => theme.colors[mv_theme::Token::Accent],
        TapeStroke::Warning => theme.colors[mv_theme::Token::Warning],
    };
    RulerColors {
        card: theme.colors[mv_theme::Token::Card],
        rim: theme.colors[mv_theme::Token::Border],
        fill: mv_theme::band(theme, param.colour_token()),
        stroke,
        index: param_ink,
        ink: theme.colors[mv_theme::Token::Foreground],
    }
}

/// A Clay colour (0 to 255 per channel) for a computed colour (the tint, the
/// row's ink) that no token names: the generic walk composites its alpha as
/// CSS does (`draw::css_background`).
fn clay_of(color: [f32; 4]) -> clay::Color {
    [
        color[0] * 255.0,
        color[1] * 255.0,
        color[2] * 255.0,
        color[3] * 255.0,
    ]
}

/// The card's Clay id, one per parameter.
#[must_use]
pub fn ruler_id(param: TransformParam) -> clay::ElementId {
    mv_clay::id("Ruler", param as u32)
}

/// The tape content's Clay id, one per parameter: the paint lookup's key.
#[must_use]
pub fn tape_id(param: TransformParam) -> clay::ElementId {
    mv_clay::id("RulerTape", param as u32)
}

/// The row's Clay id for a parameter: the float's parent (the rows declare
/// `transform_row::ROW_ID` per `transform_row::param_index`).
#[must_use]
pub fn row_parent(param: TransformParam) -> clay::ElementId {
    mv_clay::id(
        crate::transform_row::ROW_ID,
        crate::transform_row::param_index(param) as u32,
    )
}

/// Lays the ruler card out under its row: full row width, 4 px down, the tape
/// inside with the card's padding, both with their borders and radii. Called
/// while the drag holds the ruler open, with the row's element id as the
/// float's parent. `pointer-events: none`: no region is declared, so the
/// pointer falls through to the rows under the card.
pub fn layout_ruler(
    app: &mut App,
    parent: clay::ElementId,
    param: TransformParam,
    row_width: f32,
    colors: &RulerColors,
) {
    // Clay's paddings join the CSS padding with the rim (borders take no
    // layout space, the h1-frame rule): 8 + 1 on the card, 0 + 2 on the tape.
    let mut card = crate::ui::empty_decl();
    card.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_fixed(row_width),
        height: mv_clay::sizing_fixed(ruler_height()),
    };
    card.layout.padding = clay::Padding {
        left: 9,
        right: 9,
        top: 9,
        bottom: 9,
    };
    card.layout.childGap = 0;
    card.layout.layoutDirection = clay::LayoutDirection::TopToBottom;
    card.backgroundColor = clay_of(colors.card);
    card.border = clay::BorderElementConfig {
        color: clay_of(colors.rim),
        width: mv_clay::border_outside(1),
    };
    card.cornerRadius = crate::ui::corner_radius_all(RULER_RADIUS);
    card.floating = clay::FloatingElementConfig {
        offset: [0.0, RULER_GAP],
        expand: clay::Dimensions {
            width: 0.0,
            height: 0.0,
        },
        parentId: parent.id,
        zIndex: RULER_Z,
        attachment: clay::FloatingAttachPoints {
            element: clay::FloatingAttachPointType::LeftTop,
            parent: clay::FloatingAttachPointType::LeftBottom,
        },
        pointerCaptureMode: clay::PointerCaptureMode::Passthrough,
        attachTo: clay::FloatingAttachToElement::ElementWithId,
        clipTo: clay::FloatingClipToElement::None,
    };
    crate::ui::ui_scope(app, ruler_id(param), &card, |app| {
        let mut tape = crate::ui::empty_decl();
        tape.layout.sizing = clay::Sizing {
            width: mv_clay::sizing_fixed(tape_width(row_width)),
            height: mv_clay::sizing_fixed(TAPE_HEIGHT),
        };
        tape.layout.padding = clay::Padding {
            left: 2,
            right: 2,
            top: 2,
            bottom: 2,
        };
        tape.backgroundColor = clay_of(colors.fill);
        tape.border = clay::BorderElementConfig {
            color: clay_of(colors.stroke),
            width: mv_clay::border_outside(2),
        };
        tape.cornerRadius = crate::ui::corner_radius_all(TAPE_RADIUS);
        crate::ui::ui_scope(
            app,
            mv_clay::id("RulerTapeInner", param as u32),
            &tape,
            |app| {
                let mut content = crate::ui::empty_decl();
                content.layout.sizing = clay::Sizing {
                    width: mv_clay::sizing_fixed(tape_width(row_width) - 2.0 * TAPE_BORDER),
                    height: mv_clay::sizing_fixed(tape_content_height()),
                };
                crate::ui::ui_scope(app, tape_id(param), &content, |_| {});
            },
        );
    });
}

/// Paints an open ruler: the tape's contents at its laid-out content box for
/// the live `value` in `mode`. Reads nothing but the content box, the theme
/// and the arguments, so a missing box (the drag just ended) paints nothing.
pub fn paint_ruler(app: &mut App, param: TransformParam, value: f64, mode: ScrubMode) {
    let content = {
        let clay = app.clay.as_ref().expect("clay context initialised");
        let data = clay.get_element_data(tape_id(param));
        if !data.found {
            return;
        }
        data.boundingBox
    };
    let theme = crate::ui::current_theme(app).clone();
    let colors = ruler_colors(&theme, param, mode);
    let marks = ruler_marks(value, f64::from(content.width), is_scale(param));
    paint_tape(&mut app.fonts, &mut app.vg, &content, &marks, &colors);
}

/// Paints the tape's contents in its content box: the three tick batches, the
/// labels, the index over them, and the chevrons (`Marks`). Ticks hang from
/// the top edge; labels sit middle-anchored on baseline 25; the bar and its
/// pointer cover whatever labels they cross. Scissored to the content box, as
/// the tape's `overflow: hidden` clips it.
pub fn paint_tape(
    fonts: &mut mv_text::Fonts,
    vg: &mut mv_nvg::Context,
    content: &clay::BoundingBox,
    marks: &RulerMarks,
    colors: &RulerColors,
) {
    vg.scissor(content.x, content.y, content.width, content.height);
    vg.stroke_color(colors.ink);
    vg.line_cap(mv_nvg::LineCapType::BUTT);
    for (xs, length, width) in [
        (&marks.minor, TICK_MINOR_LEN, TICK_MINOR_WIDTH),
        (&marks.mid, TICK_MID_LEN, TICK_MID_WIDTH),
        (&marks.major, TICK_MAJOR_LEN, TICK_MAJOR_WIDTH),
    ] {
        vg.begin_path();
        for x in xs {
            tick(vg, content, hundredths(*x), length);
        }
        vg.stroke_width(width);
        vg.stroke();
    }
    let style = label_style();
    for label in &marks.labels {
        let advance = mv_text::measure(fonts, style, &label.text);
        mv_text::draw(
            fonts,
            vg,
            style,
            content.x + hundredths(label.x) - advance / 2.0,
            content.y + LABEL_BASELINE,
            &label.text,
            colors.ink,
        );
    }
    let centre = content.x + content.width / 2.0;
    let bar = index_rect(centre, content.height);
    crate::draw::fill_rect(
        vg,
        [bar[0], content.y + bar[1], bar[2], bar[3]],
        colors.index,
    );
    let pointer = pointer_triangle(centre, content.height);
    vg.begin_path();
    vg.move_to(pointer[0][0], content.y + pointer[0][1]);
    vg.line_to(pointer[1][0], content.y + pointer[1][1]);
    vg.line_to(pointer[2][0], content.y + pointer[2][1]);
    vg.close_path();
    vg.fill_color(colors.index);
    vg.fill();
    vg.begin_path();
    for side in [ChevronSide::Left, ChevronSide::Right] {
        let chevron = chevron_points(content.width, content.height, side);
        vg.move_to(content.x + chevron[0][0], content.y + chevron[0][1]);
        vg.line_to(content.x + chevron[1][0], content.y + chevron[1][1]);
        vg.line_to(content.x + chevron[2][0], content.y + chevron[2][1]);
    }
    vg.stroke_color(colors.ink);
    vg.stroke_width(CHEVRON_STROKE);
    vg.line_cap(mv_nvg::LineCapType::ROUND);
    vg.line_join(mv_nvg::LineCapType::ROUND);
    vg.stroke();
    vg.reset_scissor();
}

/// Strokes one tick hanging from the content box's top edge, cut where the
/// tape's rounded corner clips it (the first port's `ruler_tick`, radius 4).
fn tick(vg: &mut mv_nvg::Context, content: &clay::BoundingBox, x: f32, length: f32) {
    let mut top = 0.0f32;
    let from_edge = x.min(content.width - x);
    if from_edge < TAPE_RADIUS {
        let cut = TAPE_RADIUS - from_edge;
        top = TAPE_RADIUS - (TAPE_RADIUS * TAPE_RADIUS - cut * cut).max(0.0).sqrt();
    }
    if top >= length {
        return;
    }
    vg.move_to(content.x + x, content.y + top);
    vg.line_to(content.x + x, content.y + length);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The probe's row `a` at 1440×900: [41, 428.5, 350, 44].
    fn row_a() -> clay::BoundingBox {
        clay::BoundingBox {
            x: 41.0,
            y: 428.5,
            width: 350.0,
            height: 44.0,
        }
    }

    #[test]
    fn card_arithmetic_matches_the_reference() {
        assert_eq!(tape_width(350.0), 328.0);
        assert_eq!(ruler_height(), 50.0);
        assert_eq!(tape_origin(), 9.0);
        assert_eq!(tape_content_height(), 28.0);
    }

    #[test]
    fn ruler_sits_four_px_under_the_row() {
        // The probe's ruler_a: full row width, 4 px below the row, 50 tall.
        let ruler = ruler_box(&row_a());
        assert_eq!(
            [ruler.x, ruler.y, ruler.width, ruler.height],
            [41.0, 476.5, 350.0, 50.0]
        );
        // The probe's tape_a: the card inside its 1 px rim and 8 px padding.
        let content = tape_content_box(&ruler);
        assert_eq!(
            [content.x, content.y, content.width, content.height],
            [52.0, 487.5, 328.0, 28.0]
        );
    }

    /// The port's boxes against `testdata/page.json` (the reference at this
    /// hash, `tools/run dom --fixture`): the ruler card and tape at the
    /// ruler-a-30 shot within the plan's 0.5 CSS px.
    #[test]
    fn boxes_match_fixture() {
        const SLACK: f64 = 0.5;
        let fixture: serde_json::Value =
            serde_json::from_str(crate::fixture::PAGE_FIXTURE).expect("page fixture parses");
        let boxes = &fixture["shots"]["ruler-a-30"]["webkit"]["boxes"];
        let read = |name: &str| -> [f64; 4] {
            let got = boxes[name].as_array().expect("box");
            [
                got[0].as_f64().expect("x"),
                got[1].as_f64().expect("y"),
                got[2].as_f64().expect("w"),
                got[3].as_f64().expect("h"),
            ]
        };
        let row = read("row_a");
        let ruler = ruler_box(&clay::BoundingBox {
            x: row[0] as f32,
            y: row[1] as f32,
            width: row[2] as f32,
            height: row[3] as f32,
        });
        for (got, want) in [
            (f64::from(ruler.x), read("ruler_a")[0]),
            (f64::from(ruler.y), read("ruler_a")[1]),
            (f64::from(ruler.width), read("ruler_a")[2]),
            (f64::from(ruler.height), read("ruler_a")[3]),
        ] {
            assert!((got - want).abs() <= SLACK, "ruler {got} against {want}");
        }
        // The tape's border box: the card inside its 1 px rim and 8 px pad.
        let tape = read("tape_a");
        let ruler_want = read("ruler_a");
        for (got, want) in [
            (tape[0], ruler_want[0] + f64::from(tape_origin())),
            (tape[1], ruler_want[1] + f64::from(tape_origin())),
            (tape[2], ruler_want[2] - 2.0 * f64::from(tape_origin())),
            (tape[3], f64::from(TAPE_HEIGHT)),
        ] {
            assert!((got - want).abs() <= SLACK, "tape {got} against {want}");
        }
    }

    #[test]
    fn scale_marks_match_the_probed_drag() {
        // `a` held 30 px in: `scrubbed(1.0, 30.0, scale, coarse)` is 1.06,
        // and the probe's majors sit at these px with these labels.
        let marks = marks_for(1.06, 350.0, TransformParam::A);
        let major: Vec<f64> = marks
            .major
            .iter()
            .map(|x| mv_numfmt::round_to(*x, 2))
            .collect();
        assert_eq!(major, vec![23.15, 134.84, 226.09, 303.24]);
        let texts: Vec<&str> = marks
            .labels
            .iter()
            .map(|label| label.text.as_str())
            .collect();
        assert_eq!(texts, vec!["0.8", "1", "1.2", "1.4"]);
        // Each label sits on its major tick.
        for (label, x) in marks.labels.iter().zip(marks.major.iter()) {
            assert_eq!(label.x, *x);
        }
    }

    #[test]
    fn shift_marks_match_the_probed_drag() {
        // `k` held 30 px in: the value is 0.6, whole numbers 50 px apart.
        let marks = marks_for(0.6, 350.0, TransformParam::K);
        let major: Vec<f64> = marks
            .major
            .iter()
            .map(|x| mv_numfmt::round_to(*x, 2))
            .collect();
        assert_eq!(major, vec![34.0, 84.0, 134.0, 184.0, 234.0, 284.0]);
        let texts: Vec<&str> = marks
            .labels
            .iter()
            .map(|label| label.text.as_str())
            .collect();
        assert_eq!(texts, ["\u{2212}2", "\u{2212}1", "0", "1", "2", "3"]);
    }

    #[test]
    fn index_matches_the_probed_notch() {
        // The probe's bar: x 162.5, full 28 px height; the pointer triangle.
        assert_eq!(index_rect(164.0, 28.0), [162.5, 0.0, 3.0, 28.0]);
        let pointer = pointer_triangle(164.0, 28.0);
        for (got, want) in pointer
            .iter()
            .zip([[159.7, 28.0], [168.3, 28.0], [164.0, 23.0]])
        {
            assert!(
                (got[0] - want[0]).abs() < 1e-4 && (got[1] - want[1]).abs() < 1e-4,
                "pointer {got:?} against {want:?}"
            );
        }
    }

    #[test]
    fn chevrons_match_the_probed_paths() {
        // `M11 9L6 14L11 19M317 9L322 14L317 19` at 328×28.
        assert_eq!(
            chevron_points(328.0, 28.0, ChevronSide::Left),
            [[11.0, 9.0], [6.0, 14.0], [11.0, 19.0]]
        );
        assert_eq!(
            chevron_points(328.0, 28.0, ChevronSide::Right),
            [[317.0, 9.0], [322.0, 14.0], [317.0, 19.0]]
        );
    }

    #[test]
    fn rim_follows_the_mode() {
        assert_eq!(tape_stroke(ScrubMode::Coarse), TapeStroke::RowInk);
        assert_eq!(tape_stroke(ScrubMode::Fine), TapeStroke::Accent);
        assert_eq!(tape_stroke(ScrubMode::Snap), TapeStroke::Warning);
    }

    #[test]
    fn scales_lay_out_by_ratio() {
        assert!(is_scale(TransformParam::A));
        assert!(is_scale(TransformParam::B));
        assert!(!is_scale(TransformParam::H));
        assert!(!is_scale(TransformParam::K));
    }

    #[test]
    fn colors_resolve_around_the_rows_ink() {
        let theme = mv_theme::resolve(mv_theme::Slug::ArcticIce, mv_theme::Space::Srgb);
        let ink = theme.colors[mv_theme::Token::ParamA];
        let colors = ruler_colors(&theme, TransformParam::A, ScrubMode::Coarse);
        assert_eq!(colors.card, theme.colors[mv_theme::Token::Card]);
        assert_eq!(colors.rim, theme.colors[mv_theme::Token::Border]);
        // The wash is the pre-blended tint: the ink at 18 % alpha.
        assert_eq!(colors.fill, [ink[0], ink[1], ink[2], ink[3] * 0.18]);
        assert_eq!(colors.stroke, ink);
        assert_eq!(colors.index, ink);
        assert_eq!(colors.ink, theme.colors[mv_theme::Token::Foreground]);
        assert_eq!(
            ruler_colors(&theme, TransformParam::A, ScrubMode::Fine).stroke,
            theme.colors[mv_theme::Token::Accent]
        );
        assert_eq!(
            ruler_colors(&theme, TransformParam::A, ScrubMode::Snap).stroke,
            theme.colors[mv_theme::Token::Warning]
        );
    }
}
