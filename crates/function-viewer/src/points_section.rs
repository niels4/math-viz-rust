//! The sidebar's Points card: P's row and Q's row.
//!
//! Ported from math-viz2 at `96f1e84`:
//!
//! - `SideBar/PointsSection/PointsSection.tsx`: [`points_section`] (the
//!   third card: the `Points` title over the two rows, 16 px apart).
//! - `SideBar/PointsSection/PRow.tsx`: [`p_head`] (the 30 px head: the mark,
//!   the letter, the off-mark triangle, the role slot with the moved badge,
//!   the readout), [`off_side`] (the side P lies furthest past, in plot
//!   units, ties to the vertical), [`TRIANGLE_PATHS`].
//! - `SideBar/PointsSection/QRow.tsx`: [`q_head`] (the ring mark, Q, the
//!   role, the readout at the pointer's `q_x`).
//! - `SideBar/PointsSection/PointMark.tsx`: [`MARK_SIZE`], the bullseye and
//!   ring geometries ([`mark_radii`]).
//! - `SideBar/PointsSection/PointReadout.tsx`: [`readout_text`] (the `f(x)`
//!   readout with fixed-width slots, `≈` when inexact, scientific `m×10ⁿ`
//!   from 1e5, `–` when empty).
//!
//! The row owns P's head and leaves one call line each for the slider under
//! the head ([`crate::point_slider`], with an empty 20 px track as the
//! stand-in until h8-points-slider merges) and for the field in the
//! readout's x slot ([`crate::x_field`], the formatted number as the
//! stand-in until h8-points-field merges). `p`'s param ([`crate::functions::P_PARAM`),
//! the visible range ([`crate::visible_range`]), the number formatters
//! ([`crate::point_format`]) and the moved badge's state
//! ([`crate::moved_by`]) are the porters' names, as the plan lists them.

use mv_sys::clay;

use crate::app::App;
use crate::functions::FunctionSlug;
use crate::point_format::{ReadoutSlots, format_change, format_number, number_parts, readout_slots};
use crate::transform_curves::{Transform, TransformedFunction};
use crate::visible_range::VisibleRange;

/// The card's copy.
pub const TITLE: &str = "Points";

/// The rows' gap (`.points`: column, 16 px).
pub const ROWS_GAP: f32 = 16.0;

/// The head row: 30 px high, 8 px gaps, centred (`.head`).
pub const HEAD_HEIGHT: f32 = 30.0;
pub const HEAD_GAP: f32 = 8.0;

/// The point block: head over the slider, 6 px apart (`.point`).
pub const POINT_GAP: f32 = 6.0;

/// The slider's stand-in: an empty 20 px track under P's head until
/// h8-points-slider merges (its `point_slider.rs` replaces this scope
/// through [`SLIDER_ID`]).
pub const SLIDER_STAND_IN_HEIGHT: f32 = 20.0;

/// The mark: an 18 px svg (`.mark`, `SIZE`).
pub const MARK_SIZE: f32 = 18.0;

/// The letter: italic 20 px on 25 px serif (`.letter`).
pub const LETTER_SIZE: f32 = 20.0;
pub const LETTER_LINE: f32 = 25.0;

/// The role: 13 px 500 on 15 px (`.role`); muted when pinned, foreground
/// when off (`[data-off]`).
pub const ROLE_SIZE: f32 = 13.0;
pub const ROLE_LINE: f32 = 15.0;

/// The moved badge: a 24 px pill, radius 6, 6 px between label and change,
/// 8 px after it (`.moved`); the label 13 px 600 (`.moved_label`), the
/// change 14 px mono tabular (`.moved_value`).
pub const BADGE_HEIGHT: f32 = 24.0;
pub const BADGE_RADIUS: f32 = 6.0;
pub const BADGE_GAP: f32 = 6.0;
pub const BADGE_AFTER: f32 = 8.0;
pub const BADGE_LABEL_SIZE: f32 = 13.0;
pub const BADGE_VALUE_SIZE: f32 = 14.0;
pub const BADGE_PAD_X: f32 = 8.0;
pub const BADGE_PAD_Y: f32 = 2.0;

/// The readout's math size: 20 px on 25 px serif (`.math`); the fixed open
/// paren `round(up, 0.634em, 1px)`, the close `round(up, 0.343em, 1px)`,
/// the relation `round(up, 0.72em, 1px)` with 7 px side padding (`.rel`).
pub const MATH_SIZE: f32 = 20.0;
pub const MATH_LINE: f32 = 25.0;
pub const OPEN_EM: f32 = 0.634;
pub const CLOSE_EM: f32 = 0.343;
pub const REL_EM: f32 = 0.72;
pub const REL_PAD: f32 = 7.0;

/// The readout's numbers: 500 18 px on 24 px mono, tabular, right-aligned
/// (`.number`); the scientific exponent an 11 px run raised 8 px
/// (`.exponent`). The readout element's box is the 25 px math line,
/// centred in the 30 px head.
pub const NUMBER_SIZE: f32 = 18.0;
pub const NUMBER_LINE: f32 = 24.0;
pub const READOUT_HEIGHT: f32 = 25.0;
pub const EXPONENT_SIZE: f32 = 11.0;
pub const EXPONENT_RAISE: f32 = 8.0;

/// The empty number: U+2013 (`NO_NUMBER`), muted with `data-empty`.
pub const NO_NUMBER: &str = "–";

/// Clay ids for the card, the rows and the readout's runs, for the layout
/// test and the paint pass.
pub const CARD_ID: &str = "PointsCard";
pub const TITLE_ID: &str = "PointsTitle";
pub const LIST_ID: &str = "PointsList";
pub const P_POINT_ID: &str = "PPoint";
pub const P_HEAD_ID: &str = "PHead";
pub const P_MARK_ID: &str = "PMark";
pub const P_LETTER_ID: &str = "PLetter";
pub const P_OFF_ID: &str = "POff";
pub const P_ROLE_SLOT_ID: &str = "PRoleSlot";
pub const P_BADGE_ID: &str = "PMoved";
pub const P_ROLE_ID: &str = "PRole";
pub const P_READOUT_ID: &str = "PReadout";
pub const Q_HEAD_ID: &str = "QHead";
pub const Q_MARK_ID: &str = "QMark";
pub const Q_LETTER_ID: &str = "QLetter";
pub const Q_ROLE_ID: &str = "QRole";
pub const Q_READOUT_ID: &str = "QReadout";
/// The slider's scope (h8-points-slider's call line) and the readout x
/// slot's scope (h8-points-field's call line).
pub const SLIDER_ID: &str = "PointSlider";
pub const X_SLOT_ID: &str = "PXSlot";

/// The side P lies furthest past, as the plane's label points (`OffSide`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OffSide {
    Above,
    Below,
    Left,
    Right,
}

impl OffSide {
    /// The role's copy (`roles`).
    #[must_use]
    pub fn role(self) -> &'static str {
        match self {
            Self::Above => "above the view",
            Self::Below => "below the view",
            Self::Left => "left of the view",
            Self::Right => "right of the view",
        }
    }

    /// The triangle's direction (`arrows`).
    #[must_use]
    pub fn dir(self) -> TriangleDir {
        match self {
            Self::Above => TriangleDir::Up,
            Self::Below => TriangleDir::Down,
            Self::Left => TriangleDir::Left,
            Self::Right => TriangleDir::Right,
        }
    }
}

/// A triangle's direction (`Triangle`'s `dir`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriangleDir {
    Up,
    Down,
    Left,
    Right,
}

/// The triangle paths (`trianglePaths`): 10 × 9 up and down, 9 × 10 left
/// and right.
pub const TRIANGLE_UP: &str = "M5 0L10 9H0Z";
pub const TRIANGLE_DOWN: &str = "M0 0H10L5 9Z";
pub const TRIANGLE_LEFT: &str = "M9 0V10L0 5Z";
pub const TRIANGLE_RIGHT: &str = "M0 0L9 5L0 10Z";

/// The triangle's path for a direction.
#[must_use]
pub fn triangle_path(dir: TriangleDir) -> &'static str {
    match dir {
        TriangleDir::Up => TRIANGLE_UP,
        TriangleDir::Down => TRIANGLE_DOWN,
        TriangleDir::Left => TRIANGLE_LEFT,
        TriangleDir::Right => TRIANGLE_RIGHT,
    }
}

/// The triangle's box: 10 × 9 up and down, 9 × 10 left and right.
#[must_use]
pub fn triangle_size(dir: TriangleDir) -> [f32; 2] {
    match dir {
        TriangleDir::Up | TriangleDir::Down => [10.0, 9.0],
        TriangleDir::Left | TriangleDir::Right => [9.0, 10.0],
    }
}

/// The side P lies furthest past (`offSide`): the overshoot in plot units
/// per axis, a tie to the vertical. `None` while P is inside the range.
#[must_use]
pub fn off_side(x: f64, y: f64, range: &VisibleRange) -> Option<OffSide> {
    let over_x = if x < range.min_x {
        range.min_x - x
    } else if x > range.max_x {
        x - range.max_x
    } else {
        0.0
    };
    let over_y = if y < range.min_y {
        range.min_y - y
    } else if y > range.max_y {
        y - range.max_y
    } else {
        0.0
    };
    if over_x == 0.0 && over_y == 0.0 {
        return None;
    }
    if over_y >= over_x {
        Some(if y > range.max_y {
            OffSide::Above
        } else {
            OffSide::Below
        })
    } else if x < range.min_x {
        Some(OffSide::Left)
    } else {
        Some(OffSide::Right)
    }
}

/// The mark's shape (`PointMark`'s `shape`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkShape {
    Bullseye,
    Ring,
}

/// The mark's radii (`PointMark`): the bullseye a ring at r 8 stroked 2 px
/// with a core at r 4.5, the ring shape r 7.75 stroked 2.5 px with a centre
/// at r 2.25. Returns the ring radius, its stroke width and the core
/// radius.
#[must_use]
pub fn mark_radii(shape: MarkShape) -> (f32, f32, f32) {
    match shape {
        MarkShape::Bullseye => (8.0, 2.0, MARK_SIZE / 4.0),
        MarkShape::Ring => (7.75, 2.5, MARK_SIZE / 8.0),
    }
}

/// P's row model: what the head shows for `x` under `func` and the
/// transform. `moved` is the badge's change (`moved_by`'s), `None` while no
/// drag moves f(P).
pub struct PRowModel {
    pub x: f64,
    pub y: f64,
    pub off: Option<OffSide>,
    pub moved: Option<f64>,
    pub slots: Option<ReadoutSlots>,
}

impl PRowModel {
    /// The role's text: `pinned`, else the off side's (`roles`).
    #[must_use]
    pub fn role(&self) -> &'static str {
        self.off.map_or("pinned", OffSide::role)
    }

    /// The badge's text: `moved` beside the change through `format_change`.
    #[must_use]
    pub fn badge(&self) -> Option<String> {
        self.moved
            .map(|change| format!("moved {}", format_change(change)))
    }

    /// The readout's text, for the behaviour check: `f(x) = y` with the
    /// relation and the empty dashes.
    #[must_use]
    pub fn readout(&self) -> String {
        readout_text(Some(self.x), Some(self.y)).text
    }
}

/// Builds P's row model: f at the URL's a b h k (`PRow.tsx` line 48), not
/// the shown transform.
#[must_use]
pub fn p_row(
    func: FunctionSlug,
    transform: Transform,
    x: f64,
    range: Option<&VisibleRange>,
    moved: Option<f64>,
) -> PRowModel {
    let y = TransformedFunction {
        base: crate::functions::base(func),
        transform,
    }
    .value(x);
    PRowModel {
        x,
        y,
        off: range.and_then(|range| off_side(x, y, range)),
        moved,
        slots: range.map(readout_slots),
    }
}

/// Q's row model: the role and the readout at the pointer's `q_x`.
pub struct QRowModel {
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub slots: Option<ReadoutSlots>,
}

impl QRowModel {
    /// The role: `point at the plane` while the pointer is off it, `follows
    /// your pointer` while on it.
    #[must_use]
    pub fn role(&self) -> &'static str {
        if self.x.is_none() {
            "point at the plane"
        } else {
            "follows your pointer"
        }
    }

    /// The readout's text, for the behaviour check.
    #[must_use]
    pub fn readout(&self) -> String {
        readout_text(self.x, self.y).text
    }
}

/// Builds Q's row model: `(q_x, f(q_x))`, or the dashes.
#[must_use]
pub fn q_row(
    func: FunctionSlug,
    transform: Transform,
    x: Option<f64>,
    range: Option<&VisibleRange>,
) -> QRowModel {
    QRowModel {
        x,
        y: x.map(|x| {
            TransformedFunction {
                base: crate::functions::base(func),
                transform,
            }
            .value(x)
        }),
        slots: range.map(readout_slots),
    }
}

/// A readout's text: the joined string for the behaviour check, with the
/// superscript exponent beside it (the DOM's `sup`).
pub struct ReadoutText {
    pub text: String,
    pub superscript: Option<String>,
}

/// The readout's text (`PointReadout`): `f(` in 20 px serif with the
/// fixed-width open paren, the x number, `)`, the relation (`≈` when y's
/// `number_parts` is inexact) and y: the fixed text, the scientific
/// `m×10` with the exponent as a superscript, or `–` with `data-empty`
/// when there is no point.
#[must_use]
pub fn readout_text(x: Option<f64>, y: Option<f64>) -> ReadoutText {
    // Lengths are characters (the TS `.length` counts UTF-16 units; the
    // readout's strings are ASCII plus U+2212, one unit each). The slots
    // only size the boxes, never the text, so they stay out of here.
    let x_text = x.map_or_else(|| NO_NUMBER.to_owned(), format_number);
    let parts = y.map(number_parts);
    let (rel, y_text, superscript) = match &parts {
        None => ("=", NO_NUMBER.to_owned(), None),
        Some(parts) if parts.kind() == crate::point_format::NumberKind::Sci => (
            if parts.exact() { "=" } else { "≈" },
            format!("{}×10", parts.mantissa()),
            Some(parts.exponent().to_owned()),
        ),
        Some(parts) => (
            if parts.exact() { "=" } else { "≈" },
            parts.text().to_owned(),
            None,
        ),
    };
    ReadoutText {
        text: format!("f({x_text}) {rel} {y_text}"),
        superscript,
    }
}

/// Round up to whole px (`round(up, …, 1px)`).
#[must_use]
pub fn ceil_px(v: f32) -> f32 {
    v.ceil()
}

/// Tabular figures, as the readout's numbers set them
/// (`font-variant-numeric: tabular-nums`).
fn tabular() -> mv_text::Features {
    let mut features = mv_text::Features::new();
    features.insert(mv_text::Feature::Tabular);
    features
}

/// The `1ch` advance: the `0` in the number face, for the fixed-width slots.
#[must_use]
pub fn ch_advance(app: &mut App) -> f32 {
    mv_text::measure(
        &mut app.fonts,
        mv_text::Style {
            face: mv_text::Face::RobotoMono500,
            size: NUMBER_SIZE,
            tracking: 0.0,
            features: tabular(),
        },
        "0",
    )
}

/// The card: the `Points` title over the two rows (`PointsSection`).
pub fn points_section(app: &mut App) {
    let theme = crate::ui::current_theme(app).clone();
    let mut card = crate::ui::empty_decl();
    card.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
        height: mv_clay::sizing_fit(crate::ui::ZERO_MINMAX),
    };
    card.layout.padding = clay::Padding {
        left: crate::transform_section::CARD_PAD_X as u16,
        right: crate::transform_section::CARD_PAD_X as u16,
        top: crate::transform_section::CARD_PAD_TOP as u16,
        bottom: crate::transform_section::CARD_PAD_BOTTOM as u16,
    };
    card.layout.layoutDirection = clay::LayoutDirection::TopToBottom;
    card.backgroundColor = crate::ui::ink(&theme, mv_theme::Token::Section);
    card.cornerRadius = crate::ui::corner_radius_all(crate::transform_section::CARD_RADIUS);
    crate::ui::ui_scope(app, mv_clay::id(CARD_ID, 0), &card, |app| {
        title(app);
        crate::ui::ui_scope(app, mv_clay::id(LIST_ID, 0), &list_decl(), |app| {
            p_point(app);
            q_head(app);
        });
    });
}

/// The card's title in the sidebar's `card_title` style, as the other cards'.
fn title(app: &mut App) {
    let theme = crate::ui::current_theme(app).clone();
    crate::typography::text(
        app,
        mv_clay::id(TITLE_ID, 0),
        crate::typography::Type::Heading,
        TITLE,
        crate::ui::ink(&theme, mv_theme::Token::Foreground),
        true,
    );
}

/// The rows' list: a column 16 px apart (`.points`).
fn list_decl() -> clay::ElementDeclaration {
    let mut list = crate::ui::empty_decl();
    list.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
        height: mv_clay::sizing_fit(crate::ui::ZERO_MINMAX),
    };
    list.layout.childGap = ROWS_GAP as u16;
    list.layout.layoutDirection = clay::LayoutDirection::TopToBottom;
    // The title's 12 px gap, as the Transform card's (`TITLE_GAP`).
    list.layout.padding = clay::Padding {
        left: 0,
        right: 0,
        top: crate::transform_section::TITLE_GAP as u16,
        bottom: 0,
    };
    list
}

/// P's point block: the head over the slider (`.point`, 6 px apart).
fn p_point(app: &mut App) {
    let mut point = crate::ui::empty_decl();
    point.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
        height: mv_clay::sizing_fit(crate::ui::ZERO_MINMAX),
    };
    point.layout.childGap = POINT_GAP as u16;
    point.layout.layoutDirection = clay::LayoutDirection::TopToBottom;
    crate::ui::ui_scope(app, mv_clay::id(P_POINT_ID, 0), &point, |app| {
        p_head(app);
        // h8-points-slider's call line: the x-of-P slider under the head.
        crate::point_slider::slider(app);
    });
}

/// P's head row (`.head`): the mark, P, the off triangle, the role slot
/// with the moved badge, and the readout.
fn p_head(app: &mut App) {
    let model = live_p_row(app);
    let mut head = crate::ui::empty_decl();
    head.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
        height: mv_clay::sizing_fixed(HEAD_HEIGHT),
    };
    head.layout.childGap = HEAD_GAP as u16;
    head.layout.childAlignment = clay::ChildAlignment {
        x: clay::LayoutAlignmentX::Left,
        y: clay::LayoutAlignmentY::Center,
    };
    let off = model.off.map(OffSide::dir);
    crate::ui::ui_scope(app, mv_clay::id(P_HEAD_ID, 0), &head, |app| {
        mark(app, mv_clay::id(P_MARK_ID, 0), MarkShape::Bullseye);
        letter(app, mv_clay::id(P_LETTER_ID, 0), "P");
        if let Some(dir) = off {
            triangle(app, mv_clay::id(P_OFF_ID, 0), dir);
        }
        role_slot(app, &model);
        readout(app, mv_clay::id(P_READOUT_ID, 0), Some(model.x), Some(model.y));
    });
}

/// Q's head row: the ring mark, Q, the role and the readout.
fn q_head(app: &mut App) {
    let model = live_q_row(app);
    let mut head = crate::ui::empty_decl();
    head.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
        height: mv_clay::sizing_fixed(HEAD_HEIGHT),
    };
    head.layout.childGap = HEAD_GAP as u16;
    head.layout.childAlignment = clay::ChildAlignment {
        x: clay::LayoutAlignmentX::Left,
        y: clay::LayoutAlignmentY::Center,
    };
    crate::ui::ui_scope(app, mv_clay::id(Q_HEAD_ID, 0), &head, |app| {
        mark(app, mv_clay::id(Q_MARK_ID, 0), MarkShape::Ring);
        letter(app, mv_clay::id(Q_LETTER_ID, 0), "Q");
        role_text(app, mv_clay::id(Q_ROLE_ID, 0), model.role(), false);
        readout(app, mv_clay::id(Q_READOUT_ID, 0), model.x, model.y);
    });
}

/// The mark's spacer: a fixed 18 px box, painted post-pass.
fn mark(app: &mut App, id: clay::ElementId, _shape: MarkShape) {
    let mut decl = crate::ui::empty_decl();
    decl.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_fixed(MARK_SIZE),
        height: mv_clay::sizing_fixed(MARK_SIZE),
    };
    crate::ui::ui_scope(app, id, &decl, |_| {});
}

/// The letter's spacer: P or Q on the 25 px serif line, painted post-pass.
fn letter(app: &mut App, id: clay::ElementId, _letter: &str) {
    let advance = mv_text::measure(
        &mut app.fonts,
        mv_text::Style {
            face: mv_text::Face::Stix400Italic,
            size: LETTER_SIZE,
            tracking: 0.0,
            features: mv_text::Features::new(),
        },
        _letter,
    );
    let mut decl = crate::ui::empty_decl();
    decl.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_fixed(advance.ceil()),
        height: mv_clay::sizing_fixed(LETTER_LINE),
    };
    crate::ui::ui_scope(app, id, &decl, |_| {});
}

/// The off-mark triangle's spacer: 10 × 9 up and down, 9 × 10 left and
/// right, painted post-pass.
fn triangle(app: &mut App, id: clay::ElementId, dir: TriangleDir) {
    let [w, h] = triangle_size(dir);
    let mut decl = crate::ui::empty_decl();
    decl.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_fixed(w),
        height: mv_clay::sizing_fixed(h),
    };
    crate::ui::ui_scope(app, id, &decl, |_| {});
}

/// The role slot (`.role_slot`): flex 1, 30 px, overflow hidden. The badge
/// and the role wrap to a clipped second line when the room left is too
/// narrow, so each shows whole or not at all: the badge paints when it
/// fits, the role when both fit.
fn role_slot(app: &mut App, model: &PRowModel) {
    let mut slot = crate::ui::empty_decl();
    slot.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
        height: mv_clay::sizing_fixed(HEAD_HEIGHT),
    };
    slot.layout.childGap = HEAD_GAP as u16;
    slot.layout.childAlignment = clay::ChildAlignment {
        x: clay::LayoutAlignmentX::Left,
        y: clay::LayoutAlignmentY::Center,
    };
    crate::ui::ui_scope(app, mv_clay::id(P_ROLE_SLOT_ID, 0), &slot, |app| {
        if model.moved.is_some() {
            badge_spacer(app);
        }
        role_text(app, mv_clay::id(P_ROLE_ID, 0), model.role(), model.off.is_some());
    });
}

/// The moved badge's spacer, measured from its label and change.
fn badge_spacer(app: &mut App) {
    let width = badge_width(app);
    let mut decl = crate::ui::empty_decl();
    decl.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_fixed(width),
        height: mv_clay::sizing_fixed(BADGE_HEIGHT),
    };
    crate::ui::ui_scope(app, mv_clay::id(P_BADGE_ID, 0), &decl, |_| {});
}

/// The badge's width: the label and change advances with the 6 px gap and
/// the 8 px side padding.
#[must_use]
pub fn badge_width(app: &mut App) -> f32 {
    let label = mv_text::measure(
        &mut app.fonts,
        mv_text::Style {
            face: mv_text::Face::WorkSans600,
            size: BADGE_LABEL_SIZE,
            tracking: 0.0,
            features: mv_text::Features::new(),
        },
        "moved",
    );
    (label + BADGE_GAP + change_advance(app, "+0.00") + 2.0 * BADGE_PAD_X).ceil()
}

/// The change's advance at its paint size (14 px mono tabular).
fn change_advance(app: &mut App, text: &str) -> f32 {
    mv_text::measure(
        &mut app.fonts,
        mv_text::Style {
            face: mv_text::Face::RobotoMono500,
            size: BADGE_VALUE_SIZE,
            tracking: 0.0,
            features: tabular(),
        },
        text,
    )
}

/// A role's spacer, measured at 13 px.
fn role_text(app: &mut App, id: clay::ElementId, _role: &str, _off: bool) {
    let advance = mv_text::measure(
        &mut app.fonts,
        mv_text::Style {
            face: mv_text::Face::WorkSans500,
            size: ROLE_SIZE,
            tracking: 0.0,
            features: mv_text::Features::new(),
        },
        _role,
    );
    let mut decl = crate::ui::empty_decl();
    decl.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_fixed(advance.ceil()),
        height: mv_clay::sizing_fixed(HEAD_HEIGHT),
    };
    crate::ui::ui_scope(app, id, &decl, |_| {});
}

/// The readout's spacer: the fixed-width runs summed, 25 px tall like the
/// math line, painted post-pass. The x run is h8-points-field's slot
/// ([`X_SLOT_ID`]): until it merges the formatted number paints there.
fn readout(app: &mut App, id: clay::ElementId, x: Option<f64>, y: Option<f64>) {
    let width = readout_width(app, x, y);
    let mut decl = crate::ui::empty_decl();
    decl.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_fixed(width),
        height: mv_clay::sizing_fixed(READOUT_HEIGHT),
    };
    crate::ui::ui_scope(app, id, &decl, |app| {
        // h8-points-field's call line: the x field in the readout's x slot.
        crate::x_field::slot(app, mv_clay::id(X_SLOT_ID, 0));
    });
}

/// The readout's width: the open paren, the x slot, the close, the relation
/// with its side padding, and y — each the max of its `min-width` slot and
/// its content, as the browser sizes the spans.
#[must_use]
pub fn readout_width(app: &mut App, x: Option<f64>, y: Option<f64>) -> f32 {
    let range = crate::visible_range::live(app);
    let slots = range.as_ref().map(readout_slots);
    let number = mv_text::Style {
        face: mv_text::Face::RobotoMono500,
        size: NUMBER_SIZE,
        tracking: 0.0,
        features: tabular(),
    };
    let ch = ch_advance(app);
    let open = ceil_px(OPEN_EM * MATH_SIZE);
    let close = ceil_px(CLOSE_EM * MATH_SIZE);
    let rel = ceil_px(REL_EM * MATH_SIZE) + 2.0 * REL_PAD;
    let x_text = x.map_or_else(|| NO_NUMBER.to_owned(), format_number);
    let x_min = ceil_px(x_text.chars().count().max(slots.map_or(0, |slots| slots.x)) as f32 * ch);
    let x_slot = x_min.max(mv_text::measure(&mut app.fonts, number, &x_text));
    let y_slot = match y.map(number_parts) {
        None => mv_text::measure(&mut app.fonts, number, NO_NUMBER),
        Some(ref p) if p.kind() == crate::point_format::NumberKind::Sci => {
            sci_advance(app, p) + sup_advance(app, p)
        }
        Some(ref p) => {
            let min = ceil_px(p.text().chars().count().max(slots.map_or(0, |slots| slots.y)) as f32 * ch);
            min.max(mv_text::measure(&mut app.fonts, number, p.text()))
        }
    };
    open + x_slot + close + rel + y_slot
}

/// A scientific y's mantissa run advance (`m×10` at 18 px mono).
fn sci_advance(app: &mut App, parts: &crate::point_format::NumberParts) -> f32 {
    mv_text::measure(
        &mut app.fonts,
        mv_text::Style {
            face: mv_text::Face::RobotoMono500,
            size: NUMBER_SIZE,
            tracking: 0.0,
            features: tabular(),
        },
        &format!("{}×10", parts.mantissa()),
    )
}

/// A scientific y's exponent advance (11 px).
fn sup_advance(app: &mut App, parts: &crate::point_format::NumberParts) -> f32 {
    mv_text::measure(
        &mut app.fonts,
        mv_text::Style {
            face: mv_text::Face::RobotoMono500,
            size: EXPONENT_SIZE,
            tracking: 0.0,
            features: tabular(),
        },
        parts.exponent(),
    )
}

/// P's live row: f at the URL's a b h k with the live `p`, the visible
/// range and the badge's change.
#[must_use]
pub fn live_p_row(app: &App) -> PRowModel {
    p_row(
        *app.func_param.get(),
        Transform {
            a: *app.a_param.get(),
            b: *app.b_param.get(),
            h: *app.h_param.get(),
            k: *app.k_param.get(),
        },
        *app.p_param.get(),
        crate::visible_range::live(app).as_ref(),
        crate::moved_by::moved_by(app),
    )
}

/// Q's live row: the pointer's `q_x` with the same function and transform.
#[must_use]
pub fn live_q_row(app: &App) -> QRowModel {
    q_row(
        *app.func_param.get(),
        Transform {
            a: *app.a_param.get(),
            b: *app.b_param.get(),
            h: *app.h_param.get(),
            k: *app.k_param.get(),
        },
        app.q_x,
        crate::visible_range::live(app).as_ref(),
    )
}

/// Paints the card's rows post-pass, after the frame's commands: the marks,
/// letters, triangles, badges, roles and readouts from the same model the
/// spacers measure. Hidden scopes paint nothing.
pub fn paint_points(app: &mut App) {
    let theme = crate::ui::current_theme(app).clone();
    paint_mark(app, &theme, mv_clay::id(P_MARK_ID, 0), MarkShape::Bullseye);
    paint_mark(app, &theme, mv_clay::id(Q_MARK_ID, 0), MarkShape::Ring);
    let p = live_p_row(app);
    paint_letter(app, &theme, mv_clay::id(P_LETTER_ID, 0), "P");
    paint_letter(app, &theme, mv_clay::id(Q_LETTER_ID, 0), "Q");
    if let Some(dir) = p.off.map(OffSide::dir) {
        paint_triangle(app, &theme, mv_clay::id(P_OFF_ID, 0), dir);
    }
    paint_role_slot(app, &theme, &p);
    let q = live_q_row(app);
    paint_role(app, &theme, mv_clay::id(Q_ROLE_ID, 0), q.role(), false);
    paint_readout(app, &theme, mv_clay::id(P_READOUT_ID, 0), Some(p.x), Some(p.y));
    paint_readout(app, &theme, mv_clay::id(Q_READOUT_ID, 0), q.x, q.y);
}

/// The mark: a ring stroked over the background fill with its core
/// (`PointMark`). P's bullseye reads the foreground ring with the point_p
/// core; Q's ring the point_q ring with the foreground centre.
fn paint_mark(app: &mut App, theme: &mv_theme::Theme, id: clay::ElementId, shape: MarkShape) {
    let Some(r#box) = laid_out(app, id) else {
        return;
    };
    let (ring_r, ring_w, core_r) = mark_radii(shape);
    let cx = r#box.x + MARK_SIZE / 2.0;
    let cy = r#box.y + MARK_SIZE / 2.0;
    let (ring_ink, core_ink) = match shape {
        MarkShape::Bullseye => (
            crate::ui::ink(theme, mv_theme::Token::Foreground),
            crate::ui::ink(theme, mv_theme::Token::PointP),
        ),
        MarkShape::Ring => (
            crate::ui::ink(theme, mv_theme::Token::PointQ),
            crate::ui::ink(theme, mv_theme::Token::Foreground),
        ),
    };
    app.vg.begin_path();
    app.vg.circle(cx, cy, ring_r);
    app.vg.fill_color(crate::draw::nvg_color(crate::ui::ink(
        theme,
        mv_theme::Token::Background,
    )));
    app.vg.fill();
    app.vg.stroke_width(ring_w);
    app.vg.stroke_color(crate::draw::nvg_color(ring_ink));
    app.vg.stroke();
    app.vg.begin_path();
    app.vg.circle(cx, cy, core_r);
    app.vg.fill_color(crate::draw::nvg_color(core_ink));
    app.vg.fill();
}

/// The letter P or Q: italic 20 px serif in the foreground (`.letter`).
fn paint_letter(app: &mut App, theme: &mv_theme::Theme, id: clay::ElementId, letter: &str) {
    let Some(r#box) = laid_out(app, id) else {
        return;
    };
    let ink = crate::draw::nvg_color(crate::ui::ink(theme, mv_theme::Token::Foreground));
    let base = r#box.y + mv_text::baseline(&mut app.fonts, mv_text::Face::Stix400Italic, LETTER_SIZE, LETTER_LINE);
    mv_text::draw(
        &mut app.fonts,
        &mut app.vg,
        mv_text::Style {
            face: mv_text::Face::Stix400Italic,
            size: LETTER_SIZE,
            tracking: 0.0,
            features: mv_text::Features::new(),
        },
        r#box.x,
        base,
        letter,
        ink,
    );
}

/// The off-mark triangle in the warning colour (`.off_mark`).
fn paint_triangle(app: &mut App, theme: &mv_theme::Theme, id: clay::ElementId, dir: TriangleDir) {
    let Some(r#box) = laid_out(app, id) else {
        return;
    };
    // `trianglePaths`' corners: up (5,0)(10,9)(0,9), down (0,0)(10,0)(5,9),
    // left (9,0)(9,10)(0,5), right (0,0)(9,5)(0,10).
    let corners: [[f32; 2]; 3] = match dir {
        TriangleDir::Up => [[5.0, 0.0], [10.0, 9.0], [0.0, 9.0]],
        TriangleDir::Down => [[0.0, 0.0], [10.0, 0.0], [5.0, 9.0]],
        TriangleDir::Left => [[9.0, 0.0], [9.0, 10.0], [0.0, 5.0]],
        TriangleDir::Right => [[0.0, 0.0], [9.0, 5.0], [0.0, 10.0]],
    };
    app.vg.begin_path();
    app.vg.move_to(r#box.x + corners[0][0], r#box.y + corners[0][1]);
    app.vg.line_to(r#box.x + corners[1][0], r#box.y + corners[1][1]);
    app.vg.line_to(r#box.x + corners[2][0], r#box.y + corners[2][1]);
    app.vg.close_path();
    app.vg.fill_color(crate::draw::nvg_color(crate::ui::ink(
        theme,
        mv_theme::Token::Warning,
    )));
    app.vg.fill();
}

/// P's role slot: the badge when it fits, the role when both fit (the wrap
/// to a clipped second line shows each whole or not at all).
fn paint_role_slot(app: &mut App, theme: &mv_theme::Theme, model: &PRowModel) {
    let Some(slot) = laid_out(app, mv_clay::id(P_ROLE_SLOT_ID, 0)) else {
        return;
    };
    let badge_w = if model.moved.is_some() {
        badge_width(app)
    } else {
        0.0
    };
    let role = model.role();
    let role_w = role_advance(app, role);
    let gap = if model.moved.is_some() { BADGE_AFTER } else { 0.0 };
    if model.moved.is_some() && badge_w <= slot.width {
        paint_badge(app, theme, model.moved, slot.x);
    }
    if badge_w + gap + role_w <= slot.width {
        paint_role(app, theme, mv_clay::id(P_ROLE_ID, 0), role, model.off.is_some());
    }
}

/// A role's advance at 13 px.
fn role_advance(app: &mut App, role: &str) -> f32 {
    mv_text::measure(
        &mut app.fonts,
        mv_text::Style {
            face: mv_text::Face::WorkSans500,
            size: ROLE_SIZE,
            tracking: 0.0,
            features: mv_text::Features::new(),
        },
        role,
    )
    .ceil()
}

/// A role: 13 px 500 on its 15 px line, centred in the 30 px slot — muted
/// when pinned, foreground when off (`[data-off]`).
fn paint_role(app: &mut App, theme: &mv_theme::Theme, id: clay::ElementId, role: &str, off: bool) {
    let Some(r#box) = laid_out(app, id) else {
        return;
    };
    let token = if off {
        mv_theme::Token::Foreground
    } else {
        mv_theme::Token::ForegroundMuted
    };
    let ink = crate::draw::nvg_color(crate::ui::ink(theme, token));
    let base = r#box.y + (HEAD_HEIGHT - ROLE_LINE) / 2.0
        + mv_text::baseline(&mut app.fonts, mv_text::Face::WorkSans500, ROLE_SIZE, ROLE_LINE);
    mv_text::draw(
        &mut app.fonts,
        &mut app.vg,
        mv_text::Style {
            face: mv_text::Face::WorkSans500,
            size: ROLE_SIZE,
            tracking: 0.0,
            features: mv_text::Features::new(),
        },
        r#box.x,
        base,
        role,
        ink,
    );
}

/// The moved badge: the `moved` label beside the change through
/// `format_change`, in a 24 px pill in primary at 26 % with a 1 px primary
/// border (`.moved`).
fn paint_badge(app: &mut App, theme: &mv_theme::Theme, moved: Option<f64>, x: f32) {
    let Some(change) = moved else {
        return;
    };
    let Some(r#box) = laid_out(app, mv_clay::id(P_BADGE_ID, 0)) else {
        return;
    };
    let primary = theme.colors[mv_theme::Token::Primary];
    let pill = crate::draw::nvg_color([primary[0], primary[1], primary[2], 0.26]);
    let rim = crate::draw::nvg_color(crate::ui::ink(theme, mv_theme::Token::Primary));
    let ink = crate::draw::nvg_color(crate::ui::ink(theme, mv_theme::Token::Foreground));
    app.vg.begin_path();
    crate::draw::rounded_rect(
        &mut app.vg,
        clay::BoundingBox {
            x,
            y: r#box.y,
            width: r#box.width,
            height: BADGE_HEIGHT,
        },
        crate::ui::corner_radius_all(BADGE_RADIUS),
    );
    app.vg.fill_color(pill);
    app.vg.fill();
    app.vg.stroke_width(1.0);
    app.vg.stroke_color(rim);
    app.vg.stroke();
    let label_base = r#box.y + (BADGE_HEIGHT - ROLE_LINE) / 2.0
        + mv_text::baseline(&mut app.fonts, mv_text::Face::WorkSans600, BADGE_LABEL_SIZE, ROLE_LINE);
    mv_text::draw(
        &mut app.fonts,
        &mut app.vg,
        mv_text::Style {
            face: mv_text::Face::WorkSans600,
            size: BADGE_LABEL_SIZE,
            tracking: 0.0,
            features: mv_text::Features::new(),
        },
        x + BADGE_PAD_X,
        label_base,
        "moved",
        ink,
    );
    let value = format_change(change);
    let label_w = mv_text::measure(
        &mut app.fonts,
        mv_text::Style {
            face: mv_text::Face::WorkSans600,
            size: BADGE_LABEL_SIZE,
            tracking: 0.0,
            features: mv_text::Features::new(),
        },
        "moved",
    );
    let value_base = r#box.y + (BADGE_HEIGHT - 18.0) / 2.0
        + mv_text::baseline(&mut app.fonts, mv_text::Face::RobotoMono500, BADGE_VALUE_SIZE, 18.0);
    mv_text::draw(
        &mut app.fonts,
        &mut app.vg,
        mv_text::Style {
            face: mv_text::Face::RobotoMono500,
            size: BADGE_VALUE_SIZE,
            tracking: 0.0,
            features: tabular(),
        },
        x + BADGE_PAD_X + label_w + BADGE_GAP,
        value_base,
        &value,
        ink,
    );
}

/// The readout: `f(` in 20 px serif, the x number in 18 px mono
/// right-aligned in its slot, `)`, the relation, then y — the fixed text,
/// the scientific mantissa with the 11 px exponent raised 8 px, or `–` in
/// the muted colour when there is no point (`data-empty`).
#[allow(clippy::too_many_lines)]
fn paint_readout(
    app: &mut App,
    theme: &mv_theme::Theme,
    id: clay::ElementId,
    x: Option<f64>,
    y: Option<f64>,
) {
    let Some(r#box) = laid_out(app, id) else {
        return;
    };
    let range = crate::visible_range::live(app);
    let slots = range.as_ref().map(readout_slots);
    let x_text = x.map_or_else(|| NO_NUMBER.to_owned(), format_number);
    let parts = y.map(number_parts);
    let empty = parts.is_none();
    let token = if empty {
        mv_theme::Token::ForegroundMuted
    } else {
        mv_theme::Token::Foreground
    };
    let ink = crate::draw::nvg_color(crate::ui::ink(theme, token));
    let math = mv_text::Style {
        face: mv_text::Face::Stix400,
        size: MATH_SIZE,
        tracking: 0.0,
        features: mv_text::Features::new(),
    };
    let number = mv_text::Style {
        face: mv_text::Face::RobotoMono500,
        size: NUMBER_SIZE,
        tracking: 0.0,
        features: tabular(),
    };
    let math_base = r#box.y + mv_text::baseline(&mut app.fonts, mv_text::Face::Stix400, MATH_SIZE, READOUT_HEIGHT);
    let number_base = r#box.y + (READOUT_HEIGHT - NUMBER_LINE) / 2.0
        + mv_text::baseline(&mut app.fonts, mv_text::Face::RobotoMono500, NUMBER_SIZE, NUMBER_LINE);
    let ch = ch_advance(app);
    let open = ceil_px(OPEN_EM * MATH_SIZE);
    let close = ceil_px(CLOSE_EM * MATH_SIZE);
    let rel_w = ceil_px(REL_EM * MATH_SIZE) + 2.0 * REL_PAD;
    let x_slot = ceil_px(x_text.chars().count().max(slots.map_or(0, |slots| slots.x)) as f32 * ch);
    // The lead holds the open paren and the right-aligned x number.
    mv_text::draw(&mut app.fonts, &mut app.vg, math, r#box.x, math_base, "f(", ink);
    let x_w = mv_text::measure(&mut app.fonts, number, &x_text);
    mv_text::draw(
        &mut app.fonts,
        &mut app.vg,
        number,
        r#box.x + open + (x_slot - x_w),
        number_base,
        &x_text,
        ink,
    );
    let rel_x = r#box.x + open + x_slot;
    mv_text::draw(&mut app.fonts, &mut app.vg, math, rel_x, math_base, ")", ink);
    let rel = match &parts {
        None => "=",
        Some(p) if p.exact() => "=",
        Some(_) => "≈",
    };
    mv_text::draw(&mut app.fonts, &mut app.vg, math, rel_x + close + REL_PAD, math_base, rel, ink);
    let y_x = rel_x + close + rel_w;
    match &parts {
        None => {
            mv_text::draw(&mut app.fonts, &mut app.vg, number, y_x, number_base, NO_NUMBER, ink);
        }
        Some(p) if p.kind() == crate::point_format::NumberKind::Sci => {
            let mantissa = format!("{}×10", p.mantissa());
            mv_text::draw(&mut app.fonts, &mut app.vg, number, y_x, number_base, &mantissa, ink);
            let mantissa_w = mv_text::measure(&mut app.fonts, number, &mantissa);
            mv_text::draw(
                &mut app.fonts,
                &mut app.vg,
                mv_text::Style {
                    face: mv_text::Face::RobotoMono500,
                    size: EXPONENT_SIZE,
                    tracking: 0.0,
                    features: tabular(),
                },
                y_x + mantissa_w,
                number_base - EXPONENT_RAISE,
                p.exponent(),
                ink,
            );
        }
        Some(p) => {
            mv_text::draw(&mut app.fonts, &mut app.vg, number, y_x, number_base, p.text(), ink);
        }
    }
}

/// A laid-out box by Clay id, for the paint pass.
fn laid_out(app: &App, id: clay::ElementId) -> Option<clay::BoundingBox> {
    let data = app
        .clay
        .as_ref()
        .expect("clay context initialised")
        .get_element_data(id);
    data.found.then_some(data.boundingBox)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The fixture's range: a 500 × 400 plane at the default camera shows
    /// x −5..5 and y −4..4 (`PRow.test.tsx`'s `beforeEach`).
    fn fixture_range() -> VisibleRange {
        VisibleRange {
            min_x: -5.0,
            max_x: 5.0,
            min_y: -4.0,
            max_y: 4.0,
        }
    }

    fn identity() -> Transform {
        Transform {
            a: 1.0,
            b: 1.0,
            h: 0.0,
            k: 0.0,
        }
    }

    #[test]
    fn card_title_is_points() {
        assert_eq!(TITLE, "Points");
    }

    #[test]
    fn mark_radii_follow_point_mark() {
        assert_eq!(mark_radii(MarkShape::Bullseye), (8.0, 2.0, 4.5));
        assert_eq!(mark_radii(MarkShape::Ring), (7.75, 2.5, 2.25));
    }

    #[test]
    fn triangle_paths_follow_p_row() {
        assert_eq!(triangle_path(TriangleDir::Up), "M5 0L10 9H0Z");
        assert_eq!(triangle_path(TriangleDir::Down), "M0 0H10L5 9Z");
        assert_eq!(triangle_path(TriangleDir::Left), "M9 0V10L0 5Z");
        assert_eq!(triangle_path(TriangleDir::Right), "M0 0L9 5L0 10Z");
        assert_eq!(triangle_size(TriangleDir::Up), [10.0, 9.0]);
        assert_eq!(triangle_size(TriangleDir::Left), [9.0, 10.0]);
    }

    #[test]
    fn head_row_reads_f_p_through_function_and_transform() {
        // `func=x2&p=1.5&k=-1`: pinned, f(1.50) = 1.25.
        let range = fixture_range();
        let row = p_row(
            FunctionSlug::X2,
            Transform {
                a: 1.0,
                b: 1.0,
                h: 0.0,
                k: -1.0,
            },
            1.5,
            Some(&range),
            None,
        );
        assert_eq!(row.role(), "pinned");
        assert_eq!(format_number(row.x), "1.50");
        assert_eq!(row.readout(), "f(1.50) = 1.25");
    }

    #[test]
    fn head_row_says_approx_when_decimals_round_f_p() {
        // `func=sinx&p=1`.
        let range = fixture_range();
        let row = p_row(FunctionSlug::SinX, identity(), 1.0, Some(&range), None);
        assert!(row.readout().contains("≈ 0.84"));
    }

    #[test]
    fn head_row_prints_huge_f_p_in_scientific_notation() {
        // `func=x3&p=-123.4`.
        let range = fixture_range();
        let row = p_row(FunctionSlug::X3, identity(), -123.4, Some(&range), None);
        assert!(row.readout().contains("≈ −1.88×10"));
        let text = readout_text(Some(-123.4), Some(row.y));
        assert_eq!(text.superscript.as_deref(), Some("6"));
    }

    #[test]
    fn head_row_says_when_f_p_lies_above_the_view() {
        // `func=x2&p=3`.
        let range = fixture_range();
        let row = p_row(FunctionSlug::X2, identity(), 3.0, Some(&range), None);
        assert_eq!(row.role(), "above the view");
        assert_ne!(row.role(), "pinned");
    }

    #[test]
    fn head_row_says_when_f_p_lies_below_the_view_with_a_true_minus() {
        // `p=-5`.
        let range = fixture_range();
        let row = p_row(FunctionSlug::X1, identity(), -5.0, Some(&range), None);
        assert_eq!(row.role(), "below the view");
        assert_eq!(format_number(row.x), "−5.00");
        assert!(row.readout().contains("= −5.00"));
    }

    #[test]
    fn head_row_says_when_p_lies_left_of_the_view() {
        // `p=-6&k=6`.
        let range = fixture_range();
        let row = p_row(
            FunctionSlug::X1,
            Transform {
                a: 1.0,
                b: 1.0,
                h: 0.0,
                k: 6.0,
            },
            -6.0,
            Some(&range),
            None,
        );
        assert_eq!(row.role(), "left of the view");
        assert_ne!(row.role(), "pinned");
    }

    #[test]
    fn head_row_says_when_p_lies_right_of_the_view() {
        // `p=6&k=-6`.
        let range = fixture_range();
        let row = p_row(
            FunctionSlug::X1,
            Transform {
                a: 1.0,
                b: 1.0,
                h: 0.0,
                k: -6.0,
            },
            6.0,
            Some(&range),
            None,
        );
        assert_eq!(row.role(), "right of the view");
        assert_ne!(row.role(), "pinned");
    }

    #[test]
    fn past_a_corner_names_the_side_p_lies_furthest_past() {
        // `p=9&k=-4`: right, not above.
        let range = fixture_range();
        let row = p_row(
            FunctionSlug::X1,
            Transform {
                a: 1.0,
                b: 1.0,
                h: 0.0,
                k: -4.0,
            },
            9.0,
            Some(&range),
            None,
        );
        assert_eq!(row.role(), "right of the view");
    }

    #[test]
    fn q_row_asks_for_the_pointer_and_shows_dashes_while_off_the_plane() {
        let range = fixture_range();
        let row = q_row(FunctionSlug::X1, identity(), None, Some(&range));
        assert_eq!(row.role(), "point at the plane");
        assert_eq!(row.readout(), "f(–) = –");
    }

    #[test]
    fn q_row_reads_f_q_through_function_and_transform() {
        // `func=x2&k=-1` with the pointer at 1.5.
        let range = fixture_range();
        let row = q_row(
            FunctionSlug::X2,
            Transform {
                a: 1.0,
                b: 1.0,
                h: 0.0,
                k: -1.0,
            },
            Some(1.5),
            Some(&range),
        );
        assert_eq!(row.role(), "follows your pointer");
        assert_eq!(row.readout(), "f(1.50) = 1.25");
    }

    #[test]
    fn q_row_says_approx_when_decimals_round_f_q() {
        // `func=sinx` with the pointer at 1.
        let range = fixture_range();
        let row = q_row(FunctionSlug::SinX, identity(), Some(1.0), Some(&range));
        assert!(row.readout().contains("≈ 0.84"));
    }

    #[test]
    fn q_row_goes_back_to_dashes_once_the_pointer_leaves() {
        let range = fixture_range();
        let row = q_row(FunctionSlug::X1, identity(), None, Some(&range));
        assert_eq!(row.role(), "point at the plane");
        assert_eq!(row.readout(), "f(–) = –");
    }

    #[test]
    fn badge_text_besides_the_change() {
        let row = PRowModel {
            x: 2.0,
            y: 4.0,
            off: None,
            moved: Some(0.5),
            slots: None,
        };
        assert_eq!(row.badge().as_deref(), Some("moved +0.50"));
        let row = PRowModel {
            x: 2.0,
            y: 4.0,
            off: None,
            moved: Some(-0.5),
            slots: None,
        };
        assert_eq!(row.badge().as_deref(), Some("moved −0.50"));
    }
}
