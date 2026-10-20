//! The sidebar's Transform card: the four parameter rows.
//!
//! Ported from `SideBar/TransformSection.tsx` and
//! `TransformSection.module.css` at `5714319`: the card over the sidebar
//! column, the "Transform" legend, and the rows Vertical scale/a, Vertical
//! shift/k, a rule, Horizontal scale/b, Horizontal shift/h — the parameters
//! outside `f( )` first (they move the curve vertically), then the ones
//! inside (they move it horizontally).
//!
//! The card's box (background, radius, padding) stays the sidebar's
//! declaration, as the Function card's does; this module lays the title, the
//! rows and the rule inside it, and owns the rows' page params (their live
//! values, debounce and flush, as `url.rs` owns the state machine).

use mv_sys::clay;

use crate::app::App;
use crate::functions::{A_PARAM, B_PARAM, H_PARAM, K_PARAM};
use crate::state::TransformParam;
use crate::url::QueryParamDef;

/// The card: `sidebarStyle.card` — section background, `--radius-card` (4)
/// corners, `--space-inset` (12) padding, `--space-section` (16) at the
/// bottom.
pub const CARD_PAD_X: f32 = 12.0;
pub const CARD_PAD_TOP: f32 = 12.0;
pub const CARD_PAD_BOTTOM: f32 = 16.0;
pub const CARD_RADIUS: f32 = 4.0;

/// The legend's copy.
pub const TITLE: &str = "Transform";

/// The legend's block: `--text-heading` (17 px), `--weight-heading` (700),
/// 28 px line, `--space-inset` (12) under it (`card_title`).
pub const TITLE_FONT_SIZE: f32 = 17.0;
pub const TITLE_LINE_HEIGHT: f32 = 28.0;
pub const TITLE_GAP: f32 = 12.0;

/// The rule between the vertical and horizontal rows: 1 px tall,
/// `background-muted`, 8 px margins.
pub const RULE_HEIGHT: f32 = 1.0;
pub const RULE_MARGIN: f32 = 8.0;

/// The rows in the source's order: the vertical pair, then the horizontal
/// pair (`TransformSection.tsx`).
pub const ROW_ORDER: [TransformParam; 4] = [
    TransformParam::A,
    TransformParam::K,
    TransformParam::B,
    TransformParam::H,
];

/// Each row's name, by parameter.
#[must_use]
pub fn row_name(param: TransformParam) -> &'static str {
    match param {
        TransformParam::A => "Vertical scale",
        TransformParam::K => "Vertical shift",
        TransformParam::B => "Horizontal scale",
        TransformParam::H => "Horizontal shift",
    }
}

/// The card's measured height arithmetic (CSS px): top pad + title + gap +
/// four rows + the rule's margins and height + bottom pad.
#[must_use]
pub fn card_height() -> f32 {
    CARD_PAD_TOP
        + TITLE_LINE_HEIGHT
        + TITLE_GAP
        + 4.0 * crate::transform_row::ROW_HEIGHT
        + (2.0 * RULE_MARGIN + RULE_HEIGHT)
        + CARD_PAD_BOTTOM
}

/// Each transform parameter's page param (`transformParams[param]`).
#[must_use]
pub fn param_def(param: TransformParam) -> &'static QueryParamDef<f64> {
    match param {
        TransformParam::A => &A_PARAM,
        TransformParam::K => &K_PARAM,
        TransformParam::B => &B_PARAM,
        TransformParam::H => &H_PARAM,
    }
}

/// The parameter's live value.
#[must_use]
pub fn param_value(app: &App, param: TransformParam) -> f64 {
    match param {
        TransformParam::A => *app.a_param.get(),
        TransformParam::K => *app.k_param.get(),
        TransformParam::B => *app.b_param.get(),
        TransformParam::H => *app.h_param.get(),
    }
}

/// The params start from the page query (each `build()` re-seeds them, as
/// `init_func_from_url` re-seeds the picked function).
pub fn init_params_from_url(app: &mut App) {
    let query = app.url.query().to_owned();
    app.a_param = crate::url::QueryParam::new(&A_PARAM, &query);
    app.b_param = crate::url::QueryParam::new(&B_PARAM, &query);
    app.h_param = crate::url::QueryParam::new(&H_PARAM, &query);
    app.k_param = crate::url::QueryParam::new(&K_PARAM, &query);
}

/// One frame's debounce, then an outside edit flowing in: each param's
/// `step` (a pending write fires) and `sync` (a hash edit flows into the
/// live values, as `syncFromUrl` does). Reports whether the query changed,
/// for the web hash mirror. Runs in `app_loop::iterate` before the draw.
pub fn params_step(app: &mut App, now_ms: u64) -> bool {
    let wrote = app.a_param.step(&mut app.url, now_ms)
        | app.b_param.step(&mut app.url, now_ms)
        | app.h_param.step(&mut app.url, now_ms)
        | app.k_param.step(&mut app.url, now_ms);
    external_params_synced(app);
    wrote
}

/// An outside edit (the hash arm, back/forward) flows into the live values.
/// Idempotent: a second call with no new hash finds every key's text
/// unchanged and reports false. Returns whether anything changed.
pub fn external_params_synced(app: &mut App) -> bool {
    let changed = app.a_param.sync(&app.url)
        | app.b_param.sync(&app.url)
        | app.h_param.sync(&app.url)
        | app.k_param.sync(&app.url);
    if changed {
        app.dirty = true;
    }
    changed
}

/// The card over the sidebar column: the "Transform" legend, the rows in
/// the source's order, and the rule between the vertical and horizontal
/// pairs. The column owns the gap between the cards (`sidebar.rs`).
pub fn section(app: &mut App) {
    let theme = crate::ui::current_theme(app).clone();
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
    card.layout.layoutDirection = clay::LayoutDirection::TopToBottom;
    card.backgroundColor = crate::ui::ink(&theme, mv_theme::Token::Section);
    card.cornerRadius = crate::ui::corner_radius_all(CARD_RADIUS);
    crate::ui::ui_scope(app, mv_clay::id("TransformCard", 0), &card, |app| {
        title(app);
        for param in ROW_ORDER {
            if param == TransformParam::B {
                rule(app);
            }
            crate::transform_row::row(app, param);
        }
    });
}

/// The legend block: the copy with `--space-inset` under it (`card_title`).
fn title(app: &mut App) {
    let theme = crate::ui::current_theme(app).clone();
    let mut block = crate::ui::empty_decl();
    block.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
        height: mv_clay::sizing_fit(crate::ui::ZERO_MINMAX),
    };
    block.layout.padding = clay::Padding {
        left: 0,
        right: 0,
        top: 0,
        bottom: TITLE_GAP as u16,
    };
    crate::ui::ui_scope(app, mv_clay::id("TransformTitle", 0), &block, |app| {
        crate::typography::text(
            app,
            mv_clay::id("TransformTitleText", 0),
            crate::typography::Type::Heading,
            TITLE,
            crate::ui::ink(&theme, mv_theme::Token::Foreground),
            true,
        );
    });
}

/// The rule between the pairs: 1 px `background-muted` with 8 px margins
/// (Clay has no margins; the wrapper's padding holds them).
fn rule(app: &mut App) {
    let theme = crate::ui::current_theme(app).clone();
    let mut wrap = crate::ui::empty_decl();
    wrap.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
        height: mv_clay::sizing_fit(crate::ui::ZERO_MINMAX),
    };
    wrap.layout.padding = clay::Padding {
        left: RULE_MARGIN as u16,
        right: RULE_MARGIN as u16,
        top: RULE_MARGIN as u16,
        bottom: RULE_MARGIN as u16,
    };
    crate::ui::ui_scope(app, mv_clay::id("TransformRule", 0), &wrap, |app| {
        let mut bar = crate::ui::empty_decl();
        bar.layout.sizing = clay::Sizing {
            width: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
            height: mv_clay::sizing_fixed(RULE_HEIGHT),
        };
        bar.backgroundColor = crate::ui::ink(&theme, mv_theme::Token::BackgroundMuted);
        crate::ui::ui_scope(app, mv_clay::id("TransformRuleBar", 0), &bar, |_| {});
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::number_field::display_value;
    use crate::url::{QueryParam, QueryParamDef};

    #[test]
    fn rows_stand_in_the_sources_order() {
        assert_eq!(
            ROW_ORDER.map(row_name),
            [
                "Vertical scale",
                "Vertical shift",
                "Horizontal scale",
                "Horizontal shift"
            ]
        );
        assert_eq!(ROW_ORDER.map(TransformParam::wire), ["a", "k", "b", "h"]);
    }

    #[test]
    fn card_arithmetic_matches_the_reference() {
        assert_eq!(CARD_PAD_TOP + TITLE_LINE_HEIGHT + TITLE_GAP, 52.0);
        assert_eq!(2.0 * RULE_MARGIN + RULE_HEIGHT, 17.0);
        assert_eq!(card_height(), 261.0);
    }

    fn def_of(param: TransformParam) -> &'static QueryParamDef<f64> {
        param_def(param)
    }

    #[test]
    fn each_row_shows_the_value_a_link_sets() {
        // The transform-card test: a link setting all four parameters shows
        // each value in its own row with two decimals.
        let query = "a=2&k=-1&b=0.5&h=3";
        let shown: Vec<String> = ROW_ORDER
            .iter()
            .map(|param| display_value(*QueryParam::new(def_of(*param), query).get()))
            .collect();
        assert_eq!(shown, vec!["2.00", "-1.00", "0.50", "3.00"]);
    }

    /// The card's boxes against the reference's (webkit probe at 1440x900,
    /// `.local/drafts/probe-rows-webkit.json`): card-relative, so the page's
    /// placement stays the page test's. The card is 374 wide: the sidebar's
    /// 400 px border box minus its 1 px border and 12 px padding, both sides.
    #[test]
    fn card_lays_out_like_the_reference() {
        use mv_sys::clay;

        let _guard = mv_clay::tests_lock();
        let mut app = crate::fixture::headless_app();
        assert!(mv_text::init(&mut app.fonts));
        crate::ui::init_clay(&mut app);
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
            section(app);
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
        // x, y, w, h card-relative: card [29,358.5,374,261] on the reference.
        // Rows stand in source order (a, k, rule, b, h); boxes sit row-relative.
        for (id, want) in [
            (mv_clay::id("TransformCard", 0), [0.0, 0.0, 374.0, 261.0]),
            (mv_clay::id("TransformTitle", 0), [12.0, 12.0, 350.0, 40.0]),
            (mv_clay::id("TransformRow", 0), [12.0, 52.0, 350.0, 44.0]),
            (mv_clay::id("TransformRow", 3), [12.0, 96.0, 350.0, 44.0]),
            (
                mv_clay::id("TransformRuleBar", 0),
                [20.0, 148.0, 334.0, 1.0],
            ),
            (mv_clay::id("TransformRow", 1), [12.0, 157.0, 350.0, 44.0]),
            (mv_clay::id("TransformRow", 2), [12.0, 201.0, 350.0, 44.0]),
            (mv_clay::id("TransformBox", 0), [249.0, 58.0, 107.0, 32.0]),
            (mv_clay::id("TransformBox", 3), [249.0, 102.0, 107.0, 32.0]),
            (mv_clay::id("TransformBox", 1), [249.0, 163.0, 107.0, 32.0]),
            (mv_clay::id("TransformBox", 2), [249.0, 207.0, 107.0, 32.0]),
            (mv_clay::id("TransformField", 0), [271.5, 63.0, 62.0, 22.0]),
            (mv_clay::id("TransformChevron", 0), [259.5, 68.0, 6.0, 12.0]),
            (mv_clay::id("TransformChevron", 1), [339.5, 68.0, 6.0, 12.0]),
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
        // The chip and name stand row-relative: chip [8,9,26,26], name
        // [46,13,179,18] in every row.
        for index in [0, 3, 1, 2] {
            let row = at(mv_clay::id("TransformRow", index));
            let chip = at(mv_clay::id("TransformChip", index));
            assert!((chip.x - row.x - 8.0).abs() <= 0.5, "chip x");
            assert!((chip.y - row.y - 9.0).abs() <= 0.5, "chip y");
            assert!((chip.width - 26.0).abs() <= 0.5, "chip w");
            assert!((chip.height - 26.0).abs() <= 0.5, "chip h");
            let name = at(mv_clay::id("TransformName", index));
            assert!((name.x - row.x - 46.0).abs() <= 0.5, "name x");
            assert!((name.y - row.y - 13.0).abs() <= 0.5, "name y");
            assert!((name.width - 179.0).abs() <= 0.5, "name w");
            assert!((name.height - 18.0).abs() <= 0.5, "name h");
        }
        // The field's text runs right-aligned: its right edge meets the
        // field's, in the 22 px line box.
        let field = at(mv_clay::id("TransformField", 0));
        let run = at(mv_clay::id("TransformFieldText", 0));
        assert!(
            ((run.x + run.width) - (field.x + field.width)).abs() <= 0.5,
            "field text right edge"
        );
        assert!((run.height - 22.0).abs() <= 0.5, "field text line");
    }
}
