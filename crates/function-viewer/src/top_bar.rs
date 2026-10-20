//! The top bar: the wordmark, the title, the settings gear.
//!
//! Ported from `TopBar/TopBar.tsx` and `TopBar.module.css` at `adb0534`: a
//! 64 px row in the page, the flake and wordmark left, a divider, the title
//! block, and the gear button right (its menu is h1-settings's). Layout
//! values are the CSS values; the probe record is in the progress file.

use mv_sys::clay;

use crate::app::App;
use crate::icons::{Icon, IconKind};
use crate::input::{Cursor, Hit, HitKind};

/// The bar: 64 px tall, 20 px side padding, 20 px gaps, radius 4.
pub const TOPBAR_HEIGHT: f32 = 64.0;
pub const TOPBAR_PAD: f32 = 20.0;
pub const TOPBAR_GAP: f32 = 20.0;
pub const TOPBAR_RADIUS: f32 = 4.0;
/// The bar's 1 px border. Clay's border takes no layout space (children sit
/// at the padding from the edge), while the browser's `border-box` puts the
/// content inside the border: the horizontal padding carries the border.
pub const TOPBAR_BORDER: f32 = 1.0;

/// The wordmark: 28 px flake, 8 px gap, 24 px word.
pub const WORDMARK_ICON: f32 = 28.0;
pub const WORDMARK_GAP: f32 = 8.0;
pub const WORDMARK: &str = "MathViz";

/// The divider: 1 px, 28 px tall.
pub const DIVIDER_WIDTH: f32 = 1.0;
pub const DIVIDER_HEIGHT: f32 = 28.0;

/// The gap between the title and the subtitle: `--space-text`.
pub const TITLE_GAP: f32 = 4.0;

/// The title and subtitle.
pub const TITLE: &str = "Function Viewer";
pub const SUBTITLE: &str = "Functions · Transformations";

/// The gear button: 40 px, round, 22 px icon.
pub const GEAR_SIZE: f32 = 40.0;
pub const GEAR_ICON: f32 = 22.0;

/// The gear's Clay id, for the settings menu's anchor and tests.
pub const GEAR_ID: &str = "Gear";

/// Declares the top bar's row in the page: wordmark, divider, title block,
/// a spacer taking the free width (the gear's `margin-left: auto`), and the
/// gear button. The button's hover and focus styles are h1-settings's: the
/// menu owns the button's states, and this paint keeps the resting border.
pub fn top_bar(app: &mut App) {
    let t = crate::ui::current_theme(app).clone();
    let ink = |token| crate::ui::ink(&t, token);
    let fg = ink(mv_theme::Token::Foreground);
    let gear = Hit {
        kind: HitKind::Gear,
        index: 0,
    };

    let mut bar = crate::ui::empty_decl();
    bar.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
        height: mv_clay::sizing_fixed(TOPBAR_HEIGHT),
    };
    bar.layout.padding = clay::Padding {
        left: (TOPBAR_PAD + TOPBAR_BORDER) as u16,
        right: (TOPBAR_PAD + TOPBAR_BORDER) as u16,
        top: 0,
        bottom: 0,
    };
    bar.layout.childGap = TOPBAR_GAP as u16;
    bar.layout.childAlignment = clay::ChildAlignment {
        x: clay::LayoutAlignmentX::Left,
        y: clay::LayoutAlignmentY::Center,
    };
    bar.backgroundColor = ink(mv_theme::Token::BackgroundMuted);
    bar.border = clay::BorderElementConfig {
        color: ink(mv_theme::Token::Border),
        width: mv_clay::border_outside(1),
    };
    bar.cornerRadius = crate::ui::corner_radius_all(TOPBAR_RADIUS);
    crate::ui::ui_scope(app, mv_clay::id("TopBar", 0), &bar, |app| {
        // The wordmark: flake and word, centred.
        let mut word = crate::ui::empty_decl();
        word.layout.sizing = clay::Sizing {
            width: mv_clay::sizing_fit(crate::ui::ZERO_MINMAX),
            height: mv_clay::sizing_fit(crate::ui::ZERO_MINMAX),
        };
        word.layout.childGap = WORDMARK_GAP as u16;
        word.layout.childAlignment = clay::ChildAlignment {
            x: clay::LayoutAlignmentX::Left,
            y: clay::LayoutAlignmentY::Center,
        };
        crate::ui::ui_scope(app, mv_clay::id("Wordmark", 0), &word, |app| {
            icon(
                app,
                mv_clay::id("WordmarkIcon", 0),
                IconKind::Logo,
                WORDMARK_ICON,
                t.colors[mv_theme::Token::Primary],
            );
            crate::typography::text(
                app,
                mv_clay::id("WordmarkText", 0),
                crate::typography::Type::Wordmark,
                WORDMARK,
                fg,
                false,
            );
        });
        // The divider: a 1 px border-left, 28 px tall.
        let mut divider = crate::ui::empty_decl();
        divider.layout.sizing = clay::Sizing {
            width: mv_clay::sizing_fixed(DIVIDER_WIDTH),
            height: mv_clay::sizing_fixed(DIVIDER_HEIGHT),
        };
        divider.backgroundColor = ink(mv_theme::Token::Border);
        crate::ui::ui_scope(app, mv_clay::id("Divider", 0), &divider, |_| {});
        // The title block: title over subtitle, 4 px apart.
        let mut block = crate::ui::empty_decl();
        block.layout.sizing = clay::Sizing {
            width: mv_clay::sizing_fit(crate::ui::ZERO_MINMAX),
            height: mv_clay::sizing_fit(crate::ui::ZERO_MINMAX),
        };
        block.layout.childGap = TITLE_GAP as u16;
        block.layout.layoutDirection = clay::LayoutDirection::TopToBottom;
        crate::ui::ui_scope(app, mv_clay::id("TitleBlock", 0), &block, |app| {
            // The h1 spans the block, like a block-level heading.
            crate::typography::text(
                app,
                mv_clay::id("Title", 0),
                crate::typography::Type::Title,
                TITLE,
                fg,
                true,
            );
            crate::typography::text(
                app,
                mv_clay::id("Subtitle", 0),
                crate::typography::Type::Subtitle,
                &SUBTITLE.to_uppercase(),
                ink(mv_theme::Token::ForegroundMuted),
                false,
            );
        });
        // The free width the gear's `margin-left: auto` absorbs.
        let mut spacer = crate::ui::empty_decl();
        spacer.layout.sizing = clay::Sizing {
            width: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
            height: mv_clay::sizing_fit(crate::ui::ZERO_MINMAX),
        };
        crate::ui::ui_scope(app, mv_clay::id("TopBarSpacer", 0), &spacer, |_| {});
        // The gear button: 40 px round, secondary, bordered.
        let mut button = crate::ui::empty_decl();
        button.layout.sizing = clay::Sizing {
            width: mv_clay::sizing_fixed(GEAR_SIZE),
            height: mv_clay::sizing_fixed(GEAR_SIZE),
        };
        button.layout.childAlignment = clay::ChildAlignment {
            x: clay::LayoutAlignmentX::Center,
            y: clay::LayoutAlignmentY::Center,
        };
        button.backgroundColor = ink(mv_theme::Token::Secondary);
        button.border = clay::BorderElementConfig {
            color: ink(mv_theme::Token::Border),
            width: mv_clay::border_outside(1),
        };
        button.cornerRadius = crate::ui::corner_radius_all(GEAR_SIZE / 2.0);
        crate::ui::ui_scope(app, mv_clay::id(GEAR_ID, 0), &button, |app| {
            icon(
                app,
                mv_clay::id("GearIcon", 0),
                IconKind::Gear,
                GEAR_ICON,
                t.colors[mv_theme::Token::Foreground],
            );
        });
        crate::input::region(
            app,
            mv_clay::id(GEAR_ID, 0),
            gear,
            Cursor::Pointer,
            Cursor::Pointer,
            true,
            crate::ui::corner_radius_all(GEAR_SIZE / 2.0),
        );
    });
}

/// Declares a square icon element painting `kind` in `ink`.
fn icon(app: &mut App, id: clay::ElementId, kind: IconKind, size: f32, ink: mv_nvg::Color) {
    let mut decl = crate::ui::empty_decl();
    decl.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_fixed(size),
        height: mv_clay::sizing_fixed(size),
    };
    decl.custom = crate::icons::custom_data(Icon { kind, ink });
    crate::ui::ui_scope(app, id, &decl, |_| {});
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_are_the_css_values() {
        assert_eq!(TOPBAR_HEIGHT, 64.0);
        assert_eq!(TOPBAR_PAD, 20.0);
        assert_eq!(TOPBAR_GAP, 20.0);
        assert_eq!(TOPBAR_RADIUS, 4.0);
        assert_eq!(WORDMARK_ICON, 28.0);
        assert_eq!(WORDMARK_GAP, 8.0);
        assert_eq!(DIVIDER_HEIGHT, 28.0);
        assert_eq!(TITLE_GAP, 4.0);
        assert_eq!(GEAR_SIZE, 40.0);
        assert_eq!(GEAR_ICON, 22.0);
        assert_eq!(SUBTITLE.to_uppercase(), "FUNCTIONS · TRANSFORMATIONS");
    }

    /// The layout slack: boxes within half a CSS pixel of the reference's.
    const SLACK: f32 = 0.5;

    /// The top bar's boxes match the reference's, in CSS pixels.
    #[test]
    fn top_bar_matches_reference() {
        let _guard = mv_clay::tests_lock();
        let mut app = crate::fixture::headless_app();
        assert!(mv_text::init(&mut app.fonts));
        crate::ui::init_clay(&mut app);
        app.size = [1440.0, 900.0];
        app.pixels = [2880, 1800];
        let _commands = crate::page::layout_page(&mut app);
        let clay = app.clay.as_ref().expect("clay context initialised");
        let fixture = crate::fixture::page_fixture().expect("page fixture parses");
        let shot =
            crate::fixture::fixture_shot(&fixture, "default").expect("default shot in fixture");
        for (id, name) in [("TopBar", "header"), ("Title", "title"), (GEAR_ID, "gear")] {
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
}
