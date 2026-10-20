//! The sidebar's Function card: the function picker.
//!
//! Ported from `FunctionViewer/SideBar/SideBar.tsx` and
//! `SideBar.module.css` at `3560eb2`: the card over the sidebar column, the
//! "Function" legend, and the four radios (x, x2, x3, sin x) as a segmented
//! grid with hover and checked styles. The picker's live value is the
//! `func` page param (`functions::FUNC_PARAM`); the click, arrow-key and
//! animation-step fns join in phase 2 with the `App` fields they need.
//!
//! The labels are maths: STIX Two Text 20 px, the `x` in italic, so each
//! option lays out as centred runs (`option_runs`). The superscript 2/3 are
//! literal characters (U+00B2/U+00B3), written as escapes.

use mv_sys::clay;

use crate::app::App;
use crate::functions::{FunctionSlug, FUNCTION_SLUGS};
use crate::input::{Cursor, Hit, HitKind};

/// The card: `--card`, radius `--radius-card`, padding
/// `--space-inset`/`--space-inset`/`--space-section` (12/12/16).
pub const CARD_PAD_X: f32 = 12.0;
pub const CARD_PAD_TOP: f32 = 12.0;
pub const CARD_PAD_BOTTOM: f32 = 16.0;
pub const CARD_RADIUS: f32 = 4.0;

/// The legend's block: 28 px tall (`line-height: 28px`) with
/// `--space-inset` (12) under it (`margin-bottom`).
pub const TITLE_HEIGHT: f32 = 28.0;
pub const TITLE_GAP: f32 = 12.0;

/// The legend's copy.
pub const TITLE: &str = "Function";

/// The segmented grid: 36 px tall, 1 px `--border`, radius 4,
/// `overflow: hidden`; four 83 px cells fill the 332 px content.
pub const BUTTONS_HEIGHT: f32 = 36.0;
pub const BUTTONS_BORDER: f32 = 1.0;
pub const BUTTONS_RADIUS: f32 = 4.0;
pub const CELL_WIDTH: f32 = 83.0;

/// A label's side padding (`padding: 0 var(--space-section)`).
pub const CELL_PAD_X: f32 = 16.0;

/// The divider between cells 2-4 (`border-left: 1px solid var(--border)`,
/// border-box): a nested 1 px fill child, because the shared Border arm
/// strokes only uniform borders and a one-sided border paints nothing.
/// A checked end cell keeps the container's inner corner (4 - 1).
pub const DIVIDER_WIDTH: f32 = 1.0;
pub const CELL_RADIUS: f32 = 3.0;

/// The `:active` mix: `--primary` at 26% over the cell's base.
pub const ACTIVE_MIX: f32 = 0.26;

/// The card's Clay id, for tests.
pub const CARD_ID: &str = "FunctionCard";

/// A label run's style: upright or italic `x`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunStyle {
    Up,
    Italic,
}

/// An option's label as centred runs: the `x` in italic, the rest upright.
/// `sin x` keeps its space as its own run with the font's advance.
#[must_use]
pub fn option_runs(slug: FunctionSlug) -> &'static [(RunStyle, &'static str)] {
    match slug {
        FunctionSlug::X1 => &[(RunStyle::Italic, "x")],
        FunctionSlug::X2 => &[(RunStyle::Italic, "x"), (RunStyle::Up, "\u{00B2}")],
        FunctionSlug::X3 => &[(RunStyle::Italic, "x"), (RunStyle::Up, "\u{00B3}")],
        FunctionSlug::SinX => &[
            (RunStyle::Up, "sin"),
            (RunStyle::Up, " "),
            (RunStyle::Italic, "x"),
        ],
    }
}

/// The run's text style.
// STANDIN (phase 2): `Type::Body` stands in for the appended types -
// `Heading` for the legend, `Maths`/`MathsItalic` for the runs - so the
// files compile before h1-settings merges. Fixed-height wrappers keep every
// asserted box exact meanwhile; only glyph paint differs.
#[must_use]
pub fn run_type(_style: RunStyle) -> crate::typography::Type {
    crate::typography::Type::Body
}

/// The option's index in picker order.
#[must_use]
pub fn option_index(slug: FunctionSlug) -> usize {
    FUNCTION_SLUGS
        .iter()
        .position(|option| *option == slug)
        .expect("a picker option")
}

/// The cell's hit: `index` names the option in picker order.
#[must_use]
pub fn cell_hit(index: usize) -> Hit {
    Hit {
        kind: HitKind::Selector,
        index: index as i64,
    }
}

/// Source-over: `primary` at [`ACTIVE_MIX`] over the cell's base, as the
/// `:active` color-mix reads (a transient press state, never in a shot).
fn active_over(base: clay::Color, primary: clay::Color) -> clay::Color {
    // Clay colours run 0 to 255, as CSS hands them.
    let mix = |p: f32, b: f32| p * ACTIVE_MIX + b * (1.0 - ACTIVE_MIX);
    [
        mix(primary[0], base[0]),
        mix(primary[1], base[1]),
        mix(primary[2], base[2]),
        255.0,
    ]
}

/// Lays out the Function card in the open sidebar scope: the legend over the
/// segmented grid. `current` is the picked function; widths and heights are
/// the CSS values, so the card is 104 px tall at any width.
pub fn function_card(app: &mut App, current: FunctionSlug) {
    let theme = crate::ui::current_theme(app).clone();
    let ink = |token| crate::ui::ink(&theme, token);
    let mut card = crate::ui::empty_decl();
    card.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
        height: mv_clay::sizing_fit(crate::ui::ZERO_MINMAX),
    };
    card.layout.padding = clay::Padding {
        left: CARD_PAD_X as u16,
        right: CARD_PAD_X as u16,
        top: CARD_PAD_TOP as u16,
        bottom: CARD_PAD_BOTTOM as u16,
    };
    card.layout.childGap = TITLE_GAP as u16;
    card.layout.layoutDirection = clay::LayoutDirection::TopToBottom;
    card.backgroundColor = ink(mv_theme::Token::Card);
    card.cornerRadius = crate::ui::corner_radius_all(CARD_RADIUS);
    crate::ui::ui_scope(app, mv_clay::id(CARD_ID, 0), &card, |app| {
        legend(app, ink(mv_theme::Token::Foreground));
        buttons(app, current);
    });
}

/// The legend's block: a fixed 28 px wrap (the CSS line box) holding the
/// title text. The wrap goes away with the `Heading` type, whose own line
/// box is 28 px.
fn legend(app: &mut App, foreground: clay::Color) {
    let mut wrap = crate::ui::empty_decl();
    wrap.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
        height: mv_clay::sizing_fixed(TITLE_HEIGHT),
    };
    crate::ui::ui_scope(app, mv_clay::id("FunctionTitle", 0), &wrap, |app| {
        crate::typography::text(
            app,
            mv_clay::id("FunctionTitleText", 0),
            crate::typography::Type::Body,
            TITLE,
            foreground,
            false,
        );
    });
}

/// The segmented grid: the bordered 36 px row of option cells.
fn buttons(app: &mut App, current: FunctionSlug) {
    let theme = crate::ui::current_theme(app).clone();
    let ink = |token| crate::ui::ink(&theme, token);
    let mut grid = crate::ui::empty_decl();
    grid.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
        height: mv_clay::sizing_fixed(BUTTONS_HEIGHT),
    };
    // CSS padding 0 plus the 1 px border: Clay's border takes no space.
    grid.layout.padding = clay::Padding {
        left: BUTTONS_BORDER as u16,
        right: BUTTONS_BORDER as u16,
        top: BUTTONS_BORDER as u16,
        bottom: BUTTONS_BORDER as u16,
    };
    grid.backgroundColor = ink(mv_theme::Token::Card);
    grid.border = clay::BorderElementConfig {
        color: ink(mv_theme::Token::Border),
        width: mv_clay::border_outside(BUTTONS_BORDER as u16),
    };
    grid.cornerRadius = crate::ui::corner_radius_all(BUTTONS_RADIUS);
    crate::ui::ui_scope(app, mv_clay::id("FunctionButtons", 0), &grid, |app| {
        let at = option_index(current);
        for (i, slug) in FUNCTION_SLUGS.iter().enumerate() {
            cell(app, *slug, i, i == at);
        }
    });
}

/// One option cell: 83 px wide filling the grid's height; cells 2-4 lead
/// with their divider, then the centred runs. The checked cell paints
/// primary (over `:hover` and `:active`, as the reference measures); a held
/// cell mixes primary over its base; a hovered cell paints secondary.
fn cell(app: &mut App, slug: FunctionSlug, index: usize, checked: bool) {
    let theme = crate::ui::current_theme(app).clone();
    let ink = |token| crate::ui::ink(&theme, token);
    let hit = cell_hit(index);
    let hovered = app.input.hover == hit;
    let pressed = app.input.pressed == hit;
    let card = ink(mv_theme::Token::Card);
    let background = if checked {
        ink(mv_theme::Token::Primary)
    } else if pressed {
        let base = if hovered {
            ink(mv_theme::Token::Secondary)
        } else {
            card
        };
        active_over(base, ink(mv_theme::Token::Primary))
    } else if hovered {
        ink(mv_theme::Token::Secondary)
    } else {
        card
    };
    let mut decl = crate::ui::empty_decl();
    decl.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_fixed(CELL_WIDTH),
        height: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
    };
    decl.backgroundColor = background;
    if checked {
        // The grid clips to its radius; the end cells keep the outer corners.
        let (left, right) = match index {
            0 => (CELL_RADIUS, 0.0),
            3 => (0.0, CELL_RADIUS),
            _ => (0.0, 0.0),
        };
        decl.cornerRadius = clay::CornerRadius {
            topLeft: left,
            topRight: right,
            bottomLeft: left,
            bottomRight: right,
        };
    }
    let foreground = if checked {
        ink(mv_theme::Token::PrimaryForeground)
    } else {
        ink(mv_theme::Token::Foreground)
    };
    crate::ui::ui_scope(
        app,
        mv_clay::id("FunctionCell", index as u32),
        &decl,
        |app| {
            if index > 0 {
                let mut bar = crate::ui::empty_decl();
                bar.layout.sizing = clay::Sizing {
                    width: mv_clay::sizing_fixed(DIVIDER_WIDTH),
                    height: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
                };
                bar.backgroundColor = ink(mv_theme::Token::Border);
                crate::ui::ui_scope(
                    app,
                    mv_clay::id("FunctionDivider", index as u32),
                    &bar,
                    |_| {},
                );
            }
            runs(app, slug, index, foreground);
        },
    );
    crate::input::region(
        app,
        mv_clay::id("FunctionCell", index as u32),
        hit,
        Cursor::Default,
        Cursor::Default,
        checked,
        crate::ui::corner_radius_all(0.0),
    );
}

/// The label's centred runs.
fn runs(app: &mut App, slug: FunctionSlug, index: usize, foreground: clay::Color) {
    let mut box_ = crate::ui::empty_decl();
    box_.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
        height: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
    };
    box_.layout.padding = clay::Padding {
        left: CELL_PAD_X as u16,
        right: CELL_PAD_X as u16,
        top: 0,
        bottom: 0,
    };
    box_.layout.childGap = 0;
    box_.layout.childAlignment = clay::ChildAlignment {
        x: clay::LayoutAlignmentX::Center,
        y: clay::LayoutAlignmentY::Center,
    };
    crate::ui::ui_scope(
        app,
        mv_clay::id("FunctionRuns", index as u32),
        &box_,
        |app| {
            for (j, (style, text)) in option_runs(slug).iter().enumerate() {
                crate::typography::text(
                    app,
                    mv_clay::id("FunctionRun", (index * 8 + j) as u32),
                    run_type(*style),
                    text,
                    foreground,
                    false,
                );
            }
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_are_the_css_values() {
        assert_eq!(CARD_PAD_X, 12.0);
        assert_eq!(CARD_PAD_TOP, 12.0);
        assert_eq!(CARD_PAD_BOTTOM, 16.0);
        assert_eq!(CARD_RADIUS, 4.0);
        assert_eq!(TITLE_HEIGHT, 28.0);
        assert_eq!(TITLE_GAP, 12.0);
        assert_eq!(TITLE, "Function");
        assert_eq!(BUTTONS_HEIGHT, 36.0);
        assert_eq!(BUTTONS_BORDER, 1.0);
        assert_eq!(BUTTONS_RADIUS, 4.0);
        assert_eq!(CELL_WIDTH, 83.0);
        assert_eq!(CELL_PAD_X, 16.0);
        assert_eq!(DIVIDER_WIDTH, 1.0);
        assert_eq!(CELL_RADIUS, 3.0);
        assert_eq!(
            CARD_PAD_TOP + TITLE_HEIGHT + TITLE_GAP + BUTTONS_HEIGHT + CARD_PAD_BOTTOM,
            104.0
        );
    }

    #[test]
    fn options_read_in_picker_order() {
        assert_eq!(
            FUNCTION_SLUGS.map(option_runs),
            [
                &[(RunStyle::Italic, "x")][..],
                &[(RunStyle::Italic, "x"), (RunStyle::Up, "\u{00B2}")][..],
                &[(RunStyle::Italic, "x"), (RunStyle::Up, "\u{00B3}")][..],
                &[
                    (RunStyle::Up, "sin"),
                    (RunStyle::Up, " "),
                    (RunStyle::Italic, "x")
                ][..],
            ]
        );
        assert_eq!(option_index(FunctionSlug::SinX), 3);
        assert_eq!(
            cell_hit(2),
            Hit {
                kind: HitKind::Selector,
                index: 2
            }
        );
    }

    /// The card's boxes against the reference's (Chromium probe at 1440x900,
    /// `.local/drafts/probe.json`): card-relative, so the page's placement
    /// stays the page test's. Run text boxes are the phase-2 type's.
    #[test]
    fn card_lays_out_like_the_reference() {
        let _guard = mv_clay::tests_lock();
        let mut app = crate::fixture::headless_app();
        assert!(mv_text::init(&mut app.fonts));
        crate::ui::init_clay(&mut app);
        app.size = [358.0, 900.0];
        app.pixels = [716, 1800];
        app.clay
            .as_mut()
            .expect("clay context initialised")
            .set_layout_dimensions(clay::Dimensions {
                width: 358.0,
                height: 900.0,
            });
        app.clay
            .as_mut()
            .expect("clay context initialised")
            .begin_layout();
        let mut root = crate::ui::empty_decl();
        root.layout.sizing = clay::Sizing {
            width: mv_clay::sizing_fixed(358.0),
            height: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
        };
        crate::ui::ui_scope(&mut app, mv_clay::id("CardScope", 0), &root, |app| {
            function_card(app, FunctionSlug::X1);
        });
        app.clay
            .as_mut()
            .expect("clay context initialised")
            .end_layout();
        let clay = app.clay.as_ref().expect("clay context initialised");
        let at = |id: clay::ElementId| {
            let data = clay.get_element_data(id);
            assert!(data.found, "element laid out");
            data.boundingBox
        };
        // x, y, w, h card-relative: card [37,117,358,104] on the reference.
        for (id, want) in [
            (mv_clay::id(CARD_ID, 0), [0.0, 0.0, 358.0, 104.0]),
            (mv_clay::id("FunctionTitle", 0), [12.0, 12.0, 334.0, 28.0]),
            (mv_clay::id("FunctionButtons", 0), [12.0, 52.0, 334.0, 36.0]),
            (mv_clay::id("FunctionCell", 0), [13.0, 53.0, 83.0, 34.0]),
            (mv_clay::id("FunctionCell", 1), [96.0, 53.0, 83.0, 34.0]),
            (mv_clay::id("FunctionCell", 2), [179.0, 53.0, 83.0, 34.0]),
            (mv_clay::id("FunctionCell", 3), [262.0, 53.0, 83.0, 34.0]),
            (mv_clay::id("FunctionDivider", 1), [96.0, 53.0, 1.0, 34.0]),
            (mv_clay::id("FunctionDivider", 2), [179.0, 53.0, 1.0, 34.0]),
            (mv_clay::id("FunctionDivider", 3), [262.0, 53.0, 1.0, 34.0]),
            (mv_clay::id("FunctionRuns", 0), [13.0, 53.0, 83.0, 34.0]),
            (mv_clay::id("FunctionRuns", 1), [97.0, 53.0, 82.0, 34.0]),
        ] {
            let b = at(id);
            for (i, (got, want)) in [b.x, b.y, b.width, b.height]
                .iter()
                .zip(want.iter())
                .enumerate()
            {
                assert!(
                    (got - want).abs() <= 0.5,
                    "box [{i}]: laid out {got}, reference {want}"
                );
            }
        }
        // Every option lays its runs out, and the lone space holds its
        // advance (Clay must not collapse it).
        for (i, slug) in FUNCTION_SLUGS.iter().enumerate() {
            let mut width = 0.0;
            for j in 0..option_runs(*slug).len() {
                let b = at(mv_clay::id("FunctionRun", (i * 8 + j) as u32));
                assert!(b.width > 0.0, "run {i}/{j} has width");
                assert!(b.height > 0.0, "run {i}/{j} has height");
                width += b.width;
            }
            assert!(width > 0.0, "option {i} spans width");
        }
    }
}
