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
use crate::functions::{FUNCTION_SLUGS, FunctionSlug};
use crate::input::{Cursor, Hit, HitKind};

/// The card: `--section` (the inner surface since 82c2809's restyle;
/// `--card` before), radius `--radius-card`, padding
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
/// `overflow: hidden`; four 87 px cells fill the 348 px content.
pub const BUTTONS_HEIGHT: f32 = 36.0;
pub const BUTTONS_BORDER: f32 = 1.0;
pub const BUTTONS_RADIUS: f32 = 4.0;
pub const CELL_WIDTH: f32 = 87.0;

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

/// The run's text style: upright runs in the maths face, `x` in its italic.
#[must_use]
pub fn run_type(style: RunStyle) -> crate::typography::Type {
    match style {
        RunStyle::Up => crate::typography::Type::Maths,
        RunStyle::Italic => crate::typography::Type::MathsItalic,
    }
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
    card.backgroundColor = ink(mv_theme::Token::Section);
    card.cornerRadius = crate::ui::corner_radius_all(CARD_RADIUS);
    crate::ui::ui_scope(app, mv_clay::id(CARD_ID, 0), &card, |app| {
        legend(app, ink(mv_theme::Token::Foreground));
        buttons(app, current);
        // The readout's two lines, in their own column with the form's
        // margin-top between them (h4-readout; the card's gap covers the
        // readout's own margin-top above it).
        let mut lines = crate::ui::empty_decl();
        lines.layout.sizing = clay::Sizing {
            width: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
            height: mv_clay::sizing_fit(crate::ui::ZERO_MINMAX),
        };
        lines.layout.childGap = crate::readout::FORM_MARGIN_TOP as u16;
        lines.layout.layoutDirection = clay::LayoutDirection::TopToBottom;
        crate::ui::ui_scope(app, mv_clay::id("FunctionReadout", 0), &lines, |app| {
            let mut ids = crate::readout::LayoutIds::fresh();
            crate::readout::layout_value_line(app, current, &mut ids);
            crate::readout::layout_form_line(app, current, &mut ids);
        });
    });
}

/// The legend: a block spanning the card, 28 px tall, in the heading style.
fn legend(app: &mut App, foreground: clay::Color) {
    let mut block = crate::ui::empty_decl();
    block.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
        height: mv_clay::sizing_fit(crate::ui::ZERO_MINMAX),
    };
    crate::ui::ui_scope(app, mv_clay::id("FunctionTitle", 0), &block, |app| {
        crate::typography::text(
            app,
            mv_clay::id("FunctionTitleText", 0),
            crate::typography::Type::Heading,
            TITLE,
            foreground,
            true,
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

/// Seeds the picker's param and shown function from the page query at start
/// (`--query` natively, the hash on the web): a link's function shows whole,
/// with no animation. Runs once in each `build()` after `app.url` is set;
/// `App::zeroed` already holds the URL defaults for headless tests.
pub fn init_func_from_url(app: &mut App) {
    app.func_param = crate::url::QueryParam::new(&crate::functions::FUNC_PARAM, app.url.query());
    let slug = *app.func_param.get();
    app.shown = crate::plot::Shown::new(crate::functions::base(slug));
    app.plot_frame_at = None;
    app.plot_hold = false;
}

/// Picks the option: the URL at once (the hook's `setFunc` with
/// `{ flush: true }`), the switch animation unless reduced motion shows the
/// new curve whole, and keyboard focus on the cell (unseen, as the
/// reference's outline on its 0x0 input is).
pub fn pick_function(app: &mut App, slug: FunctionSlug, index: usize) {
    app.func_param.set_flushed(&mut app.url, slug);
    let next = crate::functions::base(slug);
    // `switch_to` no-ops on the shown function; only a real switch holds a
    // turn (a no-op mid-switch must not pause the running one).
    if !std::ptr::fn_addr_eq(next, app.shown.func) {
        begin_switch(app, next);
        #[cfg(target_os = "emscripten")]
        crate::platform_web::platform_write_hash(&app.url.full_hash());
    }
    crate::input::set_focus(app, cell_hit(index), false);
    app.dirty = true;
}

/// Starts the switch to `next`, holding the turn: from rest the next step
/// carries the hook's first-frame gap; mid-switch the running clock persists
/// (the hook's loop is keyed on `switching`, so a re-pick never restarts it:
/// the turn draws the reset curves, the next frame diffs from the last
/// callback, not the click).
fn begin_switch(app: &mut App, next: crate::plot::PlotFunction) {
    let mid = app.shown.is_switching();
    app.shown.switch_to(next, !app.reduced_motion);
    if app.shown.is_switching() {
        if mid {
            app.plot_hold = true;
        } else {
            app.plot_frame_at = None;
        }
    }
}

/// Routes a click: the release whose down-hit and up-hit agree on a cell.
pub fn click(app: &mut App, down: Hit) {
    let up = app.input.hover;
    if down != up || down.kind != HitKind::Selector {
        return;
    }
    let index = down.index as usize;
    if let Some(slug) = FUNCTION_SLUGS.get(index) {
        pick_function(app, *slug, index);
    }
}

/// An arrow key within the group: the neighbouring option, wrapping round.
#[must_use]
pub fn arrow_slug(current: usize, len: usize, left: bool) -> usize {
    if left {
        current.checked_sub(1).unwrap_or(len - 1)
    } else {
        (current + 1) % len
    }
}

/// Routes a key: the arrows move and check within the focused group, as the
/// DOM radio group does. Space and Enter on a radio change nothing (arrows
/// check; there is nothing to confirm). Returns whether consumed.
pub fn key(app: &mut App, event: &crate::input::KeyEvent) -> bool {
    use sdl3::keyboard::Keycode;
    if !event.down || event.repeat {
        return false;
    }
    if app.input.focus.kind != HitKind::Selector {
        return false;
    }
    let at = app.input.focus.index as usize;
    if at >= FUNCTION_SLUGS.len() {
        return false;
    }
    match event.key {
        Keycode::Left | Keycode::Up | Keycode::Right | Keycode::Down => {
            let left = matches!(event.key, Keycode::Left | Keycode::Up);
            let next = arrow_slug(at, FUNCTION_SLUGS.len(), left);
            pick_function(app, FUNCTION_SLUGS[next], next);
            crate::input::set_focus(app, cell_hit(next), true);
            true
        }
        _ => false,
    }
}

/// One plot frame on the app's clock: while a switch runs, advance it. The
/// switch's own turn holds the render-phase curves (elapsed 0, as
/// `switch_to` leaves them for the first paint); the loop steps from the
/// turn after, the first step carrying the hook's first-frame gap.
pub fn plot_step(app: &mut App, now: crate::app_loop::Time) {
    if !app.shown.is_switching() {
        return;
    }
    // A mid-switch re-pick's own turn draws the reset curves without
    // stepping, keeping the running clock for the next frame.
    if app.plot_hold {
        app.plot_hold = false;
        return;
    }
    let Some(last) = app.plot_frame_at else {
        app.plot_frame_at = Some(now);
        return;
    };
    let dt = (now - last) as f64 / crate::app_loop::MS as f64;
    crate::plot::step(&mut app.shown, dt);
    app.plot_frame_at = Some(now);
    if !app.shown.is_switching() {
        app.plot_frame_at = None;
    }
    app.dirty = true;
}

/// An outside edit flowing in (the `hash` verb, the web `hashchange`): the
/// param syncs, and a newly named function is adopted as a switch. Focus
/// stays where it was, as a hash edit moves no DOM focus.
pub fn external_func_synced(app: &mut App) -> bool {
    if !app.func_param.sync(&app.url) {
        return false;
    }
    let next = crate::functions::base(*app.func_param.get());
    if !std::ptr::fn_addr_eq(next, app.shown.func) {
        begin_switch(app, next);
    }
    app.dirty = true;
    true
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
        assert_eq!(CELL_WIDTH, 87.0);
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
        app.size = [374.0, 900.0];
        app.pixels = [748, 1800];
        app.clay
            .as_mut()
            .expect("clay context initialised")
            .set_layout_dimensions(clay::Dimensions {
                width: 374.0,
                height: 900.0,
            });
        app.clay
            .as_mut()
            .expect("clay context initialised")
            .begin_layout();
        let mut root = crate::ui::empty_decl();
        root.layout.sizing = clay::Sizing {
            width: mv_clay::sizing_fixed(374.0),
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
        // x, y, w, h card-relative: card [29,109,374,255.5] on the reference
        // at the h5 hash (the readout reserves 78 px since `2e6069e`, 18 over
        // h4's 60 — h5-buttons's named change; the buttons and cells above it
        // stand where they did).
        for (id, want) in [
            (mv_clay::id(CARD_ID, 0), [0.0, 0.0, 374.0, 255.5]),
            (mv_clay::id("FunctionTitle", 0), [12.0, 12.0, 350.0, 28.0]),
            (mv_clay::id("FunctionButtons", 0), [12.0, 52.0, 350.0, 36.0]),
            (mv_clay::id("FunctionCell", 0), [13.0, 53.0, 87.0, 34.0]),
            (mv_clay::id("FunctionCell", 1), [100.0, 53.0, 87.0, 34.0]),
            (mv_clay::id("FunctionCell", 2), [187.0, 53.0, 87.0, 34.0]),
            (mv_clay::id("FunctionCell", 3), [274.0, 53.0, 87.0, 34.0]),
            (mv_clay::id("FunctionDivider", 1), [100.0, 53.0, 1.0, 34.0]),
            (mv_clay::id("FunctionDivider", 2), [187.0, 53.0, 1.0, 34.0]),
            (mv_clay::id("FunctionDivider", 3), [274.0, 53.0, 1.0, 34.0]),
            (mv_clay::id("FunctionRuns", 0), [13.0, 53.0, 87.0, 34.0]),
            (mv_clay::id("FunctionRuns", 1), [101.0, 53.0, 86.0, 34.0]),
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
        // The runs' rows: 25 px line boxes (the maths types), adjacent runs
        // with no gap, the span centred in the runs box's content.
        let runs_origin = |i: usize| {
            let content = at(mv_clay::id("FunctionRuns", i as u32));
            (content.x + CELL_PAD_X, content.width - 2.0 * CELL_PAD_X)
        };
        for (i, slug) in FUNCTION_SLUGS.iter().enumerate() {
            let (left, content) = runs_origin(i);
            let mut width = 0.0;
            for (j, _) in option_runs(*slug).iter().enumerate() {
                let b = at(mv_clay::id("FunctionRun", (i * 8 + j) as u32));
                assert!(
                    (b.height - 25.0).abs() <= 0.5,
                    "run {i}/{j}: line box {h}, reference 25",
                    h = b.height
                );
                if j > 0 {
                    let prev = at(mv_clay::id("FunctionRun", (i * 8 + j - 1) as u32));
                    assert!(
                        (b.x - (prev.x + prev.width)).abs() <= 0.5,
                        "run {i}/{j} starts where run {k} ends",
                        k = j - 1
                    );
                }
                width += b.width;
            }
            let first = at(mv_clay::id("FunctionRun", (i * 8) as u32));
            assert!(
                (first.x - (left + (content - width) / 2.0)).abs() <= 0.5,
                "option {i}: span starts at {x}, centred {c}",
                x = first.x,
                c = left + (content - width) / 2.0
            );
        }
    }

    /// Ports `SideBar.test.tsx`'s "puts the picked function in the URL, and
    /// leaves x out of it": a fresh page graphs x; picking x2 checks it and
    /// writes `func=x2`; picking x again takes `func` out of the URL.
    #[test]
    fn picking_x2_writes_func_and_picking_x_removes_it() {
        let _guard = mv_clay::tests_lock();
        let mut app = crate::fixture::headless_app();
        assert!(mv_text::init(&mut app.fonts));
        crate::ui::init_clay(&mut app);
        // A fresh page graphs x.
        assert_eq!(*app.func_param.get(), FunctionSlug::X1);
        assert_eq!(app.url.query(), "");
        // Picking x2 checks it and writes func=x2, starting the switch.
        pick_function(&mut app, FunctionSlug::X2, 1);
        assert_eq!(*app.func_param.get(), FunctionSlug::X2);
        assert_eq!(app.url.query(), "func=x2");
        assert!(app.shown.is_switching());
        assert_eq!(app.input.focus, cell_hit(1));
        // Picking x again takes func out of the URL.
        pick_function(&mut app, FunctionSlug::X1, 0);
        assert_eq!(*app.func_param.get(), FunctionSlug::X1);
        assert_eq!(app.url.query(), "");
    }

    /// Ports `SideBar.test.tsx`'s "checks the function a link names": a link
    /// with `func=sinx` opens on sin x, whole, with no switch to run.
    #[test]
    fn a_link_naming_sinx_checks_sin_x() {
        let _guard = mv_clay::tests_lock();
        let mut app = crate::fixture::headless_app();
        assert!(mv_text::init(&mut app.fonts));
        crate::ui::init_clay(&mut app);
        app.url = crate::url::Url::from_query("func=sinx");
        init_func_from_url(&mut app);
        assert_eq!(*app.func_param.get(), FunctionSlug::SinX);
        assert!(!app.shown.is_switching());
        assert!(std::ptr::fn_addr_eq(
            app.shown.func,
            crate::functions::base(FunctionSlug::SinX)
        ));
    }

    /// The arrows wrap round the group, as the DOM radio group does.
    #[test]
    fn arrows_wrap_round_the_group() {
        assert_eq!(arrow_slug(0, 4, true), 3);
        assert_eq!(arrow_slug(3, 4, false), 0);
        assert_eq!(arrow_slug(1, 4, false), 2);
        assert_eq!(arrow_slug(2, 4, true), 1);
    }

    /// An arrow key moves and checks within the focused group at once.
    #[test]
    fn arrow_right_from_x_checks_x2() {
        use sdl3::keyboard::{Keycode, Mod};
        let _guard = mv_clay::tests_lock();
        let mut app = crate::fixture::headless_app();
        assert!(mv_text::init(&mut app.fonts));
        crate::ui::init_clay(&mut app);
        crate::input::set_focus(&mut app, cell_hit(0), true);
        let right = crate::input::KeyEvent {
            key: Keycode::Right,
            mods: Mod::empty(),
            down: true,
            repeat: false,
        };
        assert!(key(&mut app, &right));
        assert_eq!(*app.func_param.get(), FunctionSlug::X2);
        assert_eq!(app.url.query(), "func=x2");
        assert_eq!(app.input.focus, cell_hit(1));
        // Off the group the arrows go elsewhere.
        crate::input::set_focus(&mut app, Hit::default(), false);
        assert!(!key(&mut app, &right));
    }

    /// The switch's own turn holds the render-phase curves; the loop steps
    /// from the turn after, ending whole past the draw duration.
    #[test]
    fn plot_step_holds_the_switch_turn_then_advances() {
        let _guard = mv_clay::tests_lock();
        let mut app = crate::fixture::headless_app();
        assert!(mv_text::init(&mut app.fonts));
        crate::ui::init_clay(&mut app);
        pick_function(&mut app, FunctionSlug::X2, 1);
        // The switch's turn: held, the new curve still at draw 0.
        plot_step(&mut app, 16_000_000);
        assert!(app.shown.is_switching());
        assert_eq!(crate::plot::curves(&app.shown)[1].draw_to, 0.0);
        // The next turn steps with the hook's first-frame gap.
        plot_step(&mut app, 32_000_000);
        assert_eq!(app.shown.elapsed, crate::plot::FIRST_FRAME_MS);
        // Past the draw duration the switch ends whole.
        let mut now = 32_000_000;
        while app.shown.is_switching() {
            now += 16_000_000;
            plot_step(&mut app, now);
        }
        assert_eq!(
            crate::plot::curves(&app.shown),
            vec![crate::plot::PlotCurve {
                func: crate::functions::base(FunctionSlug::X2),
                alpha: 1.0,
                draw_to: 1.0,
            }]
        );
        assert!(app.plot_frame_at.is_none());
    }

    /// A mid-switch re-pick holds its own turn without restarting the clock:
    /// the turn draws the reset curves, and the next frame diffs from the
    /// last callback, as the hook's `switching`-keyed loop does.
    #[test]
    fn mid_switch_repick_holds_the_turn_keeping_the_clock() {
        let _guard = mv_clay::tests_lock();
        let mut app = crate::fixture::headless_app();
        assert!(mv_text::init(&mut app.fonts));
        crate::ui::init_clay(&mut app);
        pick_function(&mut app, FunctionSlug::X2, 1);
        plot_step(&mut app, 16_000_000);
        plot_step(&mut app, 32_000_000);
        // Mid-switch, the clock anchored at the last step.
        assert_eq!(app.plot_frame_at, Some(32_000_000));
        pick_function(&mut app, FunctionSlug::X3, 2);
        // The re-pick's turn: held, the reset curves drawn, the anchor kept.
        plot_step(&mut app, 48_000_000);
        assert_eq!(app.plot_frame_at, Some(32_000_000));
        assert_eq!(app.shown.elapsed, 0.0);
        // The next turn diffs from the last step, not the re-pick.
        plot_step(&mut app, 64_000_000);
        assert_eq!(app.shown.elapsed, 32.0);
    }

    /// The port's boxes against `testdata/page.json` (the reference at this
    /// hash, `tools/run dom --fixture`): the sidebar, the card and the four
    /// option cells within the plan's 0.5 CSS px, through the page's layout.
    #[test]
    fn boxes_match_fixture() {
        const SLACK: f32 = 0.5;
        let _guard = mv_clay::tests_lock();
        let mut app = crate::fixture::headless_app();
        assert!(mv_text::init(&mut app.fonts));
        crate::ui::init_clay(&mut app);
        app.size = [1440.0, 900.0];
        app.pixels = [2880, 1800];
        app.url = crate::url::Url::new("#function-viewer");
        let _commands = crate::page::layout_page(&mut app);
        let clay = app.clay.as_ref().expect("clay context initialised");
        let fixture = crate::fixture::page_fixture().expect("page fixture parses");
        let shot =
            crate::fixture::fixture_shot(&fixture, "default").expect("default shot in fixture");
        let mut names: Vec<(&str, u32, &str)> = vec![(crate::page::SIDEBAR_ID, 0, "sidebar")];
        names.push((CARD_ID, 0, "card_function"));
        for i in 0..4 {
            names.push(("FunctionCell", i, "radio"));
        }
        for (id, n, name) in names {
            let data = clay.get_element_data(mv_clay::id(id, n));
            assert!(data.found, "{name}: no such element");
            let b = data.boundingBox;
            let key = if name == "radio" {
                format!("radio_{n}")
            } else {
                name.to_string()
            };
            let want = crate::fixture::fixture_box(&shot.webkit, &key).expect("box in fixture");
            for (i, (got, want)) in [b.x, b.y, b.width, b.height]
                .iter()
                .zip(want.iter())
                .enumerate()
            {
                assert!(
                    (got - want).abs() <= SLACK,
                    "{key}[{i}]: laid out {got}, reference {want}"
                );
            }
        }
    }

    /// An outside edit naming another function is adopted as a switch.
    #[test]
    fn external_hash_edit_adopts_the_named_function() {
        let _guard = mv_clay::tests_lock();
        let mut app = crate::fixture::headless_app();
        assert!(mv_text::init(&mut app.fonts));
        crate::ui::init_clay(&mut app);
        app.url = crate::url::Url::from_query("func=x3");
        assert!(external_func_synced(&mut app));
        assert_eq!(*app.func_param.get(), FunctionSlug::X3);
        assert!(app.shown.is_switching());
        // Nothing new: no adoption, no dirtying.
        app.dirty = false;
        assert!(!external_func_synced(&mut app));
        assert!(!app.dirty);
    }
}
