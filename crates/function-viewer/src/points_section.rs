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
//! The row owns P's head and leaves stand-ins for its siblings: an empty
//! 20 px track under the head for h8-points-slider's call line, and the
//! formatted number in the readout's x slot for h8-points-field (which
//! swaps its field in at its merge). `p`'s param (`P_PARAM`), the visible
//! range (`visible_range`), the number formatters (`point_format`) and the
//! badge's state machine (`MovedBy`) are h8-p-points's; the per-param scrub
//! modes the badge reads as its drag flag are h7-p-state's; `Token::PointP`
//! and `Token::PointQ` land with h8-plane's regeneration.

use mv_sys::clay;

use crate::app::App;
use crate::app_loop::MS;
use crate::functions::FunctionSlug;
use crate::point_format::{
    NumberParts, ReadoutSlots, format_change, format_number, number_parts, readout_slots,
};
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
/// change 14 px mono tabular (`.moved_value`). The pill is primary mixed
/// 26 % over the card (`BADGE_MIX`).
pub const BADGE_HEIGHT: f32 = 24.0;
pub const BADGE_RADIUS: f32 = 6.0;
pub const BADGE_GAP: f32 = 6.0;
pub const BADGE_AFTER: f32 = 8.0;
pub const BADGE_MIX: f32 = 0.26;
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
pub const Q_ROLE_SLOT_ID: &str = "QRoleSlot";
pub const Q_ROLE_ID: &str = "QRole";
pub const Q_READOUT_ID: &str = "QReadout";
/// The readout x slot's scope (h8-points-field's call line; the slider's
/// scope is h8-points-slider's `TRACK_ID`, the same "PointSlider").
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
        slots: readout_slots(range.copied()),
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
        slots: readout_slots(range.copied()),
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
        Some(NumberParts::Sci {
            mantissa,
            exponent,
            exact,
        }) => (
            if *exact { "=" } else { "≈" },
            format!("{mantissa}×10"),
            Some(exponent.clone()),
        ),
        Some(NumberParts::Fixed { text, exact }) => {
            (if *exact { "=" } else { "≈" }, text.clone(), None)
        }
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
        // The x-of-P slider under the head (h8-points-slider's call line):
        // the 20 px track with the thumb and tip.
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
        readout(
            app,
            mv_clay::id(P_READOUT_ID, 0),
            Some(model.x),
            Some(model.y),
        );
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
        // The role slot, as P's (without the badge): flex 1, so the
        // readout stands right.
        let mut slot = crate::ui::empty_decl();
        slot.layout.sizing = clay::Sizing {
            width: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
            height: mv_clay::sizing_fixed(HEAD_HEIGHT),
        };
        slot.layout.childAlignment = clay::ChildAlignment {
            x: clay::LayoutAlignmentX::Left,
            y: clay::LayoutAlignmentY::Center,
        };
        crate::ui::ui_scope(app, mv_clay::id(Q_ROLE_SLOT_ID, 0), &slot, |app| {
            let (_, role_w) = slot_shown(app, model.role(), None, "Q", None, model.x, model.y);
            if role_w > 0.0 {
                role_text(app, mv_clay::id(Q_ROLE_ID, 0), role_w);
            }
        });
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
    // Exact: flex sums fractional advances (the browser's P is 11.39 px),
    // unlike the CSS `round(up, …)` slots.
    let mut decl = crate::ui::empty_decl();
    decl.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_fixed(advance),
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

/// The head's content width: the card's content box (the sidebar column is
/// a fixed 400 px track with 12 px insets and 12 px card padding, so 350 px
/// at every window width, as the readout's row is).
pub const HEAD_CONTENT_WIDTH: f32 = 350.0;

/// What a role slot shows: the badge's and the role's spacer widths (0 for
/// a hidden run). The badge and the role wrap to a clipped second line when
/// the room left is too narrow, so each shows whole or not at all: the
/// badge fits its slot, the role fits beside the badge (or alone when the
/// badge wrapped away). Hidden runs declare no scope, so no phantom gap
/// shifts a shown run and no unbreakable fixed run stretches the head past
/// the card.
fn slot_shown(
    app: &mut App,
    role: &str,
    moved: Option<f64>,
    letter: &str,
    tri: Option<TriangleDir>,
    x: Option<f64>,
    y: Option<f64>,
) -> (f32, f32) {
    let letter_w = mv_text::measure(
        &mut app.fonts,
        mv_text::Style {
            face: mv_text::Face::Stix400Italic,
            size: LETTER_SIZE,
            tracking: 0.0,
            features: mv_text::Features::new(),
        },
        letter,
    );
    let tri_w = tri.map_or(0.0, |dir| triangle_size(dir)[0] + HEAD_GAP);
    let readout_w = readout_width(app, x, y);
    let slot_avail = HEAD_CONTENT_WIDTH
        - (MARK_SIZE + HEAD_GAP + letter_w + HEAD_GAP + tri_w)
        - HEAD_GAP
        - readout_w;
    let badge_w = badge_width(app);
    let badge_shown = moved.is_some() && badge_w <= slot_avail;
    let role_w = role_advance(app, role);
    let role_shown = if badge_shown {
        badge_w + BADGE_AFTER + role_w <= slot_avail
    } else {
        role_w <= slot_avail
    };
    (
        if badge_shown { badge_w } else { 0.0 },
        if role_shown { role_w } else { 0.0 },
    )
}

/// The role slot (`.role_slot`): flex 1, 30 px, overflow hidden.
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
    let off = model.off.map(OffSide::dir);
    let (badge_w, role_w) = slot_shown(
        app,
        model.role(),
        model.moved,
        "P",
        off,
        Some(model.x),
        Some(model.y),
    );
    crate::ui::ui_scope(app, mv_clay::id(P_ROLE_SLOT_ID, 0), &slot, |app| {
        if badge_w > 0.0 {
            badge_spacer(app, badge_w);
        }
        if role_w > 0.0 {
            role_text(app, mv_clay::id(P_ROLE_ID, 0), role_w);
        }
    });
}

/// The moved badge's spacer: the shown width (hidden declares no scope).
fn badge_spacer(app: &mut App, width: f32) {
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
    label + BADGE_GAP + change_advance(app, "+0.00") + 2.0 * BADGE_PAD_X
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

/// A role's spacer: the shown width (hidden declares no scope), 30 px tall
/// like the slot.
fn role_text(app: &mut App, id: clay::ElementId, width: f32) {
    let mut decl = crate::ui::empty_decl();
    decl.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_fixed(width),
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
    crate::ui::ui_scope(app, id, &decl, |_| {
        // h8-points-field swaps its `x_field` call in for the formatted
        // number's paint at its merge (the x slot's box stays).
    });
}

/// The readout's width: the open paren, the x slot, the close, the relation
/// with its side padding, and y — each the max of its `min-width` slot and
/// its content, as the browser sizes the spans.
#[must_use]
pub fn readout_width(app: &mut App, x: Option<f64>, y: Option<f64>) -> f32 {
    let slots = readout_slots(app.visible_range);
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
    // With no point the lead's `--chars` is the dash's own length: the x
    // slot never applies while empty (`PointReadout`'s `xChars`).
    let x_chars = match x {
        Some(_) => x_text.chars().count().max(slots.map_or(0, |slots| slots.x)),
        None => x_text.chars().count(),
    };
    let x_min = ceil_px(x_chars as f32 * ch);
    let x_slot = x_min.max(mv_text::measure(&mut app.fonts, number, &x_text));
    let y_slot = match y.map(number_parts) {
        None => mv_text::measure(&mut app.fonts, number, NO_NUMBER),
        Some(NumberParts::Sci {
            ref mantissa,
            ref exponent,
            ..
        }) => sci_advance(app, mantissa) + sup_advance(app, exponent),
        Some(NumberParts::Fixed { ref text, .. }) => {
            let min =
                ceil_px(text.chars().count().max(slots.map_or(0, |slots| slots.y)) as f32 * ch);
            min.max(mv_text::measure(&mut app.fonts, number, text))
        }
    };
    open + x_slot + close + rel + y_slot
}

/// A scientific y's mantissa run advance (`m×10` at 18 px mono).
fn sci_advance(app: &mut App, mantissa: &str) -> f32 {
    mv_text::measure(
        &mut app.fonts,
        mv_text::Style {
            face: mv_text::Face::RobotoMono500,
            size: NUMBER_SIZE,
            tracking: 0.0,
            features: tabular(),
        },
        &format!("{mantissa}×10"),
    )
}

/// A scientific y's exponent advance (11 px).
fn sup_advance(app: &mut App, exponent: &str) -> f32 {
    mv_text::measure(
        &mut app.fonts,
        mv_text::Style {
            face: mv_text::Face::RobotoMono500,
            size: EXPONENT_SIZE,
            tracking: 0.0,
            features: tabular(),
        },
        exponent,
    )
}

/// P's live row: f at the URL's a b h k with the live `p`, the visible
/// range and the badge's change. Feeds the badge's state machine this
/// frame's `(dragging, x, y)` first (`MovedBy::update` dedups identical
/// feeds, so the layout's and the paint's feeds agree).
#[must_use]
pub fn live_p_row(app: &mut App) -> PRowModel {
    let transform = Transform {
        a: *app.a_param.get(),
        b: *app.b_param.get(),
        h: *app.h_param.get(),
        k: *app.k_param.get(),
    };
    let x = *app.p_param.get();
    let y = TransformedFunction {
        base: crate::functions::base(*app.func_param.get()),
        transform,
    }
    .value(x);
    let dragging = app.state.scrub_mode.values().any(|mode| mode.is_some());
    // The frame's clock (`clock_now`: script time under the harness, wall
    // ticks live) — not `input.time`, which only stands while a fact is
    // handled and is 0 in the paint, freezing the linger.
    let now_ms = (crate::app_loop::clock_now(app).max(0) as u64) / (MS as u64);
    app.moved_by.update(dragging, x, y, now_ms);
    // While the badge lingers past the release, keep a frame due: the
    // linger clears lazily inside `update` (release-anchored, never
    // restarted), and the timer's repaint paints it gone. Without this the
    // app sleeps after the release's URL flush and the shot photographs the
    // stale badge, as the scrub rest tip's timer does for its tip.
    if app.moved_by.moved().is_some() && !dragging {
        app.timers[crate::app_loop::Timer::MovedLinger] =
            (now_ms + crate::moved_by::LINGER_MS) as i64 * MS;
    } else {
        app.timers[crate::app_loop::Timer::MovedLinger] = 0;
    }
    p_row(
        *app.func_param.get(),
        transform,
        x,
        app.visible_range.as_ref(),
        app.moved_by.moved(),
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
        app.state.q_x,
        app.visible_range.as_ref(),
    )
}

/// Paints the card's rows post-pass, after the frame's commands: the marks,
/// letters, triangles, badges, roles and readouts from the same model the
/// spacers measure. Hidden scopes paint nothing. The slider paints itself
/// from exactly one place: h8-points-slider's `paint_slider` call in
/// `page.rs`, beside this one (this module paints no slider pixels, only
/// the stand-in's empty layout scope).
pub fn paint_points(app: &mut App) {
    // Refresh the cached range from the just-laid-out plane: `live` reads
    // the current layout where the layout read the previous frame's. Dirty
    // on change so the first frame redraws converged (h8-p-points's `live`
    // is the shared contract; the cache is this card's).
    let range = crate::visible_range::live(app);
    if range != app.visible_range {
        app.visible_range = range;
        app.dirty = true;
    }
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
    if let Some(slot) = laid_out(app, mv_clay::id(Q_ROLE_SLOT_ID, 0)) {
        let (_, role_w) = slot_shown(app, q.role(), None, "Q", None, q.x, q.y);
        if role_w > 0.0 {
            paint_role(app, &theme, q.role(), false, slot.x, slot.y);
        }
    }
    paint_readout(
        app,
        &theme,
        mv_clay::id(P_READOUT_ID, 0),
        Some(p.x),
        Some(p.y),
    );
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
    let base = r#box.y
        + mv_text::baseline(
            &mut app.fonts,
            mv_text::Face::Stix400Italic,
            LETTER_SIZE,
            LETTER_LINE,
        );
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
    app.vg
        .move_to(r#box.x + corners[0][0], r#box.y + corners[0][1]);
    app.vg
        .line_to(r#box.x + corners[1][0], r#box.y + corners[1][1]);
    app.vg
        .line_to(r#box.x + corners[2][0], r#box.y + corners[2][1]);
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
    // The layout's predicate, so paint and layout agree run for run.
    let off = model.off.map(OffSide::dir);
    let (badge_w, role_w) = slot_shown(
        app,
        model.role(),
        model.moved,
        "P",
        off,
        Some(model.x),
        Some(model.y),
    );
    if badge_w > 0.0 {
        paint_badge(app, theme, model.moved, slot.x, slot.y);
    }
    if role_w > 0.0 {
        paint_role(
            app,
            theme,
            model.role(),
            model.off.is_some(),
            slot.x
                + if badge_w > 0.0 {
                    badge_w + BADGE_AFTER
                } else {
                    0.0
                },
            slot.y,
        );
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
}

/// A role: 13 px 500 on its 15 px line, centred in the 30 px slot — muted
/// when pinned, foreground when off (`[data-off]`).
fn paint_role(app: &mut App, theme: &mv_theme::Theme, role: &str, off: bool, x: f32, y: f32) {
    let token = if off {
        mv_theme::Token::Foreground
    } else {
        mv_theme::Token::ForegroundMuted
    };
    let ink = crate::draw::nvg_color(crate::ui::ink(theme, token));
    let base = y
        + (HEAD_HEIGHT - ROLE_LINE) / 2.0
        + mv_text::baseline(
            &mut app.fonts,
            mv_text::Face::WorkSans500,
            ROLE_SIZE,
            ROLE_LINE,
        );
    mv_text::draw(
        &mut app.fonts,
        &mut app.vg,
        mv_text::Style {
            face: mv_text::Face::WorkSans500,
            size: ROLE_SIZE,
            tracking: 0.0,
            features: mv_text::Features::new(),
        },
        x,
        base,
        role,
        ink,
    );
}

/// The moved badge: the `moved` label beside the change through
/// `format_change`, in a 24 px pill in primary at 26 % with a 1 px primary
/// border (`.moved`).
fn paint_badge(app: &mut App, theme: &mv_theme::Theme, moved: Option<f64>, x: f32, y: f32) {
    let Some(change) = moved else {
        return;
    };
    let width = badge_width(app);
    let pill_y = y + (HEAD_HEIGHT - BADGE_HEIGHT) / 2.0;
    let primary = crate::draw::nvg_color(crate::ui::ink(theme, mv_theme::Token::Primary));
    // Primary at 26 % over the card, opaque and pre-blended: the
    // framebuffer blends linear while the reference composites gamma, so a
    // translucent fill reads ~30 levels light (h4-readout's finding).
    let backdrop = crate::draw::nvg_color(crate::ui::ink(theme, mv_theme::Token::Section));
    let pill = [
        primary[0] * BADGE_MIX + backdrop[0] * (1.0 - BADGE_MIX),
        primary[1] * BADGE_MIX + backdrop[1] * (1.0 - BADGE_MIX),
        primary[2] * BADGE_MIX + backdrop[2] * (1.0 - BADGE_MIX),
        1.0,
    ];
    let rim = crate::draw::nvg_color(crate::ui::ink(theme, mv_theme::Token::Primary));
    let ink = crate::draw::nvg_color(crate::ui::ink(theme, mv_theme::Token::Foreground));
    app.vg.begin_path();
    crate::draw::rounded_rect(
        &mut app.vg,
        clay::BoundingBox {
            x,
            y: pill_y,
            width,
            height: BADGE_HEIGHT,
        },
        crate::ui::corner_radius_all(BADGE_RADIUS),
    );
    app.vg.fill_color(pill);
    app.vg.fill();
    app.vg.stroke_width(1.0);
    app.vg.stroke_color(rim);
    app.vg.stroke();
    let label_base = pill_y
        + (BADGE_HEIGHT - ROLE_LINE) / 2.0
        + mv_text::baseline(
            &mut app.fonts,
            mv_text::Face::WorkSans600,
            BADGE_LABEL_SIZE,
            ROLE_LINE,
        );
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
    let value_base = pill_y
        + (BADGE_HEIGHT - 18.0) / 2.0
        + mv_text::baseline(
            &mut app.fonts,
            mv_text::Face::RobotoMono500,
            BADGE_VALUE_SIZE,
            18.0,
        );
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
    let slots = readout_slots(app.visible_range);
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
    let math_base = r#box.y
        + mv_text::baseline(
            &mut app.fonts,
            mv_text::Face::Stix400,
            MATH_SIZE,
            READOUT_HEIGHT,
        );
    let number_base = r#box.y
        + (READOUT_HEIGHT - NUMBER_LINE) / 2.0
        + mv_text::baseline(
            &mut app.fonts,
            mv_text::Face::RobotoMono500,
            NUMBER_SIZE,
            NUMBER_LINE,
        );
    let ch = ch_advance(app);
    let open = ceil_px(OPEN_EM * MATH_SIZE);
    let close = ceil_px(CLOSE_EM * MATH_SIZE);
    let rel_w = ceil_px(REL_EM * MATH_SIZE) + 2.0 * REL_PAD;
    let x_chars = match x {
        Some(_) => x_text.chars().count().max(slots.map_or(0, |slots| slots.x)),
        None => x_text.chars().count(),
    };
    let x_slot = ceil_px(x_chars as f32 * ch);
    // The lead right-aligns its [open paren + x] group (`justify-content:
    // flex-end`): the field's input is only the value's chars wide
    // (`--chars: max(1, text.length)`, unrounded), so the group shifts right
    // by the slot's leftover. Q's dash span has no field and keeps the
    // slot's right alignment.
    let input_w = mv_text::measure(&mut app.fonts, number, &x_text);
    let push = if x.is_some() { x_slot - input_w } else { 0.0 };
    mv_text::draw(
        &mut app.fonts,
        &mut app.vg,
        math,
        r#box.x + push,
        math_base,
        "f(",
        ink,
    );
    mv_text::draw(
        &mut app.fonts,
        &mut app.vg,
        number,
        r#box.x + open + x_slot - input_w,
        number_base,
        &x_text,
        ink,
    );
    let rel_x = r#box.x + open + x_slot;
    mv_text::draw(
        &mut app.fonts,
        &mut app.vg,
        math,
        rel_x,
        math_base,
        ")",
        ink,
    );
    let rel = match &parts {
        None => "=",
        Some(NumberParts::Fixed { exact, .. }) | Some(NumberParts::Sci { exact, .. }) => {
            if *exact {
                "="
            } else {
                "≈"
            }
        }
    };
    mv_text::draw(
        &mut app.fonts,
        &mut app.vg,
        math,
        rel_x + close + REL_PAD,
        math_base,
        rel,
        ink,
    );
    let y_x = rel_x + close + rel_w;
    match &parts {
        None => {
            mv_text::draw(
                &mut app.fonts,
                &mut app.vg,
                number,
                y_x,
                number_base,
                NO_NUMBER,
                ink,
            );
        }
        Some(NumberParts::Sci {
            mantissa, exponent, ..
        }) => {
            let mantissa = format!("{mantissa}×10");
            mv_text::draw(
                &mut app.fonts,
                &mut app.vg,
                number,
                y_x,
                number_base,
                &mantissa,
                ink,
            );
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
                exponent,
                ink,
            );
        }
        Some(NumberParts::Fixed { text, .. }) => {
            mv_text::draw(
                &mut app.fonts,
                &mut app.vg,
                number,
                y_x,
                number_base,
                text,
                ink,
            );
        }
    }
}

/// The linger timer's action: repaint, so the paint's feed clears a spent
/// linger and the shot photographs it gone.
pub fn moved_linger_fired(app: &mut App) {
    app.dirty = true;
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

    /// The card's boxes against the fixture (`SideBar.test.tsx`'s points
    /// title case rides along: the card paints, so the title stands).
    /// The first pass lays the plane out; the second lays the card out with
    /// the range the shared `live` reads from it — at rest the app has
    /// converged the same way.
    #[test]
    fn boxes_match_fixture() {
        const SLACK: f32 = 0.5;
        assert_eq!(TITLE, "Points");
        let _guard = mv_clay::tests_lock();
        let mut app = crate::fixture::headless_app();
        assert!(mv_text::init(&mut app.fonts));
        crate::ui::init_clay(&mut app);
        app.size = [1440.0, 900.0];
        app.pixels = [2880, 1800];
        let _commands = crate::page::layout_page(&mut app);
        // The paint's refresh, as the app runs it: seed the cache from the
        // laid-out plane so the second pass lays the card out converged.
        app.visible_range = crate::visible_range::live(&app);
        assert!(
            app.visible_range.is_some(),
            "the laid-out plane shows a range"
        );
        let _commands = crate::page::layout_page(&mut app);
        let clay = app.clay.as_ref().expect("clay context initialised");
        let fixture = crate::fixture::page_fixture().expect("page fixture parses");
        let shot =
            crate::fixture::fixture_shot(&fixture, "default").expect("default shot in fixture");
        for (id, name) in [
            (CARD_ID, "card_points"),
            (P_HEAD_ID, "row_p"),
            (Q_HEAD_ID, "row_q"),
            (P_MARK_ID, "mark_p"),
            (P_ROLE_SLOT_ID, "role_p"),
            (P_READOUT_ID, "readout_p"),
            (Q_READOUT_ID, "readout_q"),
        ] {
            let data = clay.get_element_data(mv_clay::id(id, 0));
            assert!(data.found, "{name}: no such element");
            let b = data.boundingBox;
            let want = crate::fixture::fixture_box(&shot.webkit, name).expect("box in fixture");
            for (i, (got, want)) in [b.x, b.y, b.width, b.height]
                .iter()
                .zip(want.iter())
                .enumerate()
            {
                assert!(
                    (got - want).abs() <= SLACK,
                    "{name}[{i}]: laid out {got}, reference {want}"
                );
            }
        }
    }

    /// A long readout never stretches the head past the card: the role
    /// wraps away (whole or not at all) instead of forcing growth
    /// (`points-sci` showed a 392 px card before this).
    #[test]
    fn long_readouts_wrap_the_role_instead_of_stretching_the_head() {
        let _guard = mv_clay::tests_lock();
        let mut app = crate::fixture::headless_app();
        assert!(mv_text::init(&mut app.fonts));
        crate::ui::init_clay(&mut app);
        app.size = [1440.0, 900.0];
        app.pixels = [2880, 1800];
        app.url = crate::url::Url::from_query("func=x3&p=-123.4");
        crate::transform_section::init_params_from_url(&mut app);
        let _commands = crate::page::layout_page(&mut app);
        app.visible_range = crate::visible_range::live(&app);
        let _commands = crate::page::layout_page(&mut app);
        let clay = app.clay.as_ref().expect("clay context initialised");
        let card = clay.get_element_data(mv_clay::id(CARD_ID, 0)).boundingBox;
        let head = clay.get_element_data(mv_clay::id(P_HEAD_ID, 0)).boundingBox;
        let readout = clay
            .get_element_data(mv_clay::id(P_READOUT_ID, 0))
            .boundingBox;
        assert_eq!((card.width, head.width), (374.0, 350.0));
        assert!(
            readout.x + readout.width <= 391.0,
            "readout spills past the card: x {} w {}",
            readout.x,
            readout.width
        );
    }

    /// A headless app with its clock settable, for the moved cases (the
    /// moved cases drive `moved_by` through a coarse scrub mode on k, as
    /// `PRow.test.tsx` does).
    fn moved_app(query: &str) -> App {
        let mut app = crate::fixture::headless_app();
        // The live range reads the Clay context: initialise it (no layout,
        // so the range is `None`, as on the first frame).
        crate::ui::init_clay(&mut app);
        app.func_param = crate::url::QueryParam::new(&crate::functions::FUNC_PARAM, query);
        app
    }

    /// Feeds one frame at `now_ms` and reads the row's strings.
    fn live_at(app: &mut App, now_ms: u64) -> PRowModel {
        app.r#loop.script_time = Some(now_ms as i64 * MS);
        live_p_row(app)
    }

    #[test]
    fn moved_badge_tracks_a_coarse_k_drag_until_600ms_after_release() {
        use crate::scrub::ScrubMode;
        use crate::state::TransformParam;

        let _guard = mv_clay::tests_lock();
        let mut app = moved_app("func=x2");
        // At rest on x2 at p = 2 (y = 4): no badge.
        assert_eq!(live_at(&mut app, 0).badge(), None);

        // The drag's first change lands with its start: k = 1 moves f(P)
        // 4 → 5, read `moved +1.00` before `pinned`.
        app.state.scrub_mode[TransformParam::K] = Some(ScrubMode::Coarse);
        app.k_param = crate::url::QueryParam::new(&crate::functions::K_PARAM, "k=1");
        let row = live_at(&mut app, 100);
        assert_eq!(row.badge().as_deref(), Some("moved +1.00"));
        assert_eq!(row.role(), "pinned");

        // On to k = −0.5 (y = 3.5): `moved −0.50`.
        app.k_param = crate::url::QueryParam::new(&crate::functions::K_PARAM, "k=-0.5");
        let row = live_at(&mut app, 200);
        assert_eq!(row.badge().as_deref(), Some("moved −0.50"));

        // Released: the badge lingers 599 ms, then goes at 600.
        app.state.scrub_mode[TransformParam::K] = None;
        let row = live_at(&mut app, 300);
        assert_eq!(row.badge().as_deref(), Some("moved −0.50"));
        let row = live_at(&mut app, 899);
        assert_eq!(row.badge().as_deref(), Some("moved −0.50"));
        let row = live_at(&mut app, 900);
        assert_eq!(row.badge(), None);
    }

    #[test]
    fn moved_badge_stays_away_while_a_drag_leaves_f_p_where_it_was() {
        use crate::scrub::ScrubMode;
        use crate::state::TransformParam;

        let _guard = mv_clay::tests_lock();
        let mut app = moved_app("func=x2");
        assert_eq!(live_at(&mut app, 0).badge(), None);
        // An h drag with x and y unchanged never marks the badge.
        app.state.scrub_mode[TransformParam::H] = Some(ScrubMode::Coarse);
        assert_eq!(live_at(&mut app, 100).badge(), None);
        assert_eq!(live_at(&mut app, 200).badge(), None);
        app.state.scrub_mode[TransformParam::H] = None;
        assert_eq!(live_at(&mut app, 300).badge(), None);
    }

    #[test]
    fn moved_badge_goes_at_once_when_a_non_drag_changes_the_curve() {
        use crate::scrub::ScrubMode;
        use crate::state::TransformParam;

        let _guard = mv_clay::tests_lock();
        let mut app = moved_app("");
        assert_eq!(live_at(&mut app, 0).badge(), None);
        // A same-frame drag start and h change marks the badge at once.
        app.state.scrub_mode[TransformParam::H] = Some(ScrubMode::Coarse);
        app.h_param = crate::url::QueryParam::new(&crate::functions::H_PARAM, "h=1");
        let row = live_at(&mut app, 100);
        assert_eq!(row.badge().as_deref(), Some("moved −1.00"));

        // Released with the value kept: the badge lingers.
        app.state.scrub_mode[TransformParam::H] = None;
        let row = live_at(&mut app, 200);
        assert_eq!(row.badge().as_deref(), Some("moved −1.00"));

        // A y change outside the drag clears it at once, not lingering.
        app.k_param = crate::url::QueryParam::new(&crate::functions::K_PARAM, "k=2");
        assert_eq!(live_at(&mut app, 300).badge(), None);
    }
}
