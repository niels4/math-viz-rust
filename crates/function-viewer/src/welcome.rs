//! The welcome page at the root route, and the page's second route.
//!
//! Ported from math-viz2 at `68e1c7b`:
//!
//! - `src/components/views/WelcomePage/WelcomePage.tsx` and
//!   `WelcomePage.module.css`: the page at the design size (1440 × 900, fit
//!   1): the top row (the mark, the settings gear), the centred frame holding
//!   the intro column (title, description, the button to `#function-viewer`,
//!   the hint with four parameter chips) beside the 840 × 520 hero.
//! - `src/components/views/WelcomePage/HeroPlane.tsx`: the hero's bare plane
//!   at the default camera with the still's curves, no legend, no handles,
//!   lines or markers. Until h10-seam's bare `paint_plane` lands, the hero
//!   composes `plane`'s parts (`paint_grid`, `paint_axes`, `paint_curves`,
//!   `paint_labels`); then it calls that.
//! - `src/components/router/Router.tsx`, `router-hooks.ts`, `router-util.ts`
//!   and `src/pages/_root.page.tsx`: the route (`""` the welcome page,
//!   `function-viewer` the viewer, any other word the welcome page; the
//!   template's not-found page and the dev theme-demo page are not ported:
//!   `decisions.md`'s h10 lines). `trim_slashes` is already `url`'s.
//!
//! The layout is written against a frame size and a fit argument (both the
//! window and 1 for now, with every position in design units), so h10-fit
//! lands as a transform and a zoom, not a rewrite. The backdrop (paper and
//! lights) is h10-hero's: [`paint_backdrop`] is the one named call between
//! the page background and the frame's content, painting nothing until it
//! merges. Entering a route is a fresh mount (plan §3.7 Routes): entering
//! the viewer resets its component state with the params and camera from
//! the URL, entering the welcome page closes its menu (h10-loop restarts its
//! loop clock).

use mv_sys::clay;

use crate::app::App;
use crate::input::{Cursor, Hit, HitKind};

/// The design size the page lays out at (`designWidth`, `designHeight`).
pub const DESIGN_WIDTH: f32 = 1440.0;
pub const DESIGN_HEIGHT: f32 = 900.0;

/// The viewer route's hash word (`function-viewer`).
pub const VIEWER_ROUTE: &str = "function-viewer";

/// The page's routes: `""` is the welcome page, `function-viewer` the
/// viewer, any other route the welcome page (the template's not-found page
/// is never in the port).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    Welcome,
    FunctionViewer,
}

impl Route {
    /// Reads the route from the app's URL, as math-viz2's router reads it
    /// from `location.hash` (trimmed, as `Router.tsx` looks the trimmed
    /// route up).
    #[must_use]
    pub fn of(url: &crate::url::Url) -> Self {
        match crate::url::trim_slashes(url.route()).as_str() {
            VIEWER_ROUTE => Self::FunctionViewer,
            _ => Self::Welcome,
        }
    }
}

/// The still the hero graphs (h10-loop's `welcomeStill`, through h10-p-loop's
/// `welcome_loop`: sin x at a 3, k 0, b 1.48, h 1). h10-loop replaces this
/// constant with the loop's state, one line.
pub const WELCOME_STILL: crate::transform_curves::Transform = crate::transform_curves::Transform {
    a: 3.0,
    b: 1.48,
    h: 1.0,
    k: 0.0,
};

/// The welcome page's mark: 28 px, stroked in `--primary`, at (32, 30).
pub const MARK_SIZE: f32 = 28.0;

/// The top row's gear: 40 px, as the viewer's (`top_bar.rs`).
pub const GEAR_SIZE: f32 = crate::top_bar::GEAR_SIZE;

/// The intro column: 380 wide, gap 28; the content row's gap 60.
pub const INTRO_WIDTH: f32 = 380.0;
pub const INTRO_GAP: u16 = 28;
pub const CONTENT_GAP: u16 = 60;

/// The actions column's gap 18; the button's padding 16 24 16 26 and gap 12;
/// the hint's gap 8; a chip 26 × 26.
pub const ACTIONS_GAP: u16 = 18;
pub const CTA_PAD_TOP: u16 = 16;
pub const CTA_PAD_RIGHT: u16 = 24;
pub const CTA_PAD_BOTTOM: u16 = 16;
pub const CTA_PAD_LEFT: u16 = 26;
pub const CTA_GAP: u16 = 12;
pub const CTA_ICON: f32 = 14.0;
pub const HINT_GAP: u16 = 8;
pub const CHIP_SIZE: f32 = 26.0;

/// The welcome radius (`--radius-card`, 4 px), as the sidebar's own 4 px
/// constant has it.
pub const WELCOME_RADIUS: f32 = 4.0;

/// The hero: 840 × 520.
pub const HERO_WIDTH: f32 = 840.0;
pub const HERO_HEIGHT: f32 = 520.0;

/// The button's label and the hint's copy.
pub const CTA_LABEL: &str = "Open the Function Viewer";
pub const HINT_TEXT: &str = "Pick a function, then drag";

/// The title.
pub const TITLE_TEXT: &str = "MathViz";

/// The description's lines: the reference wraps the sentence to three lines
/// in 380 px (h10-page's probe: 367.49, 318.91 and 216.0 px wide at a 33 px
/// pitch); Clay text never wraps, so the port lays the breaks out.
pub const DESCRIPTION_LINES: [&str; 3] = [
    "Interactive math visualizations for",
    "the browser, built with React,",
    "TypeScript and Vite.",
];

/// The hint's chips, in order (`hintParams`: a k b h).
pub const HINT_PARAMS: [crate::state::TransformParam; 4] = [
    crate::state::TransformParam::A,
    crate::state::TransformParam::K,
    crate::state::TransformParam::B,
    crate::state::TransformParam::H,
];

/// The CTA focus ring: the UA ring on an anchor, measured on ref-h10
/// (Chromium: 1 px `rgb(0, 95, 204)` at a 1 px offset; Firefox draws its own
/// 1 px ring in the link's ink instead, a recorded engine quirk; WebKit
/// never shows it, skipping the link on Tab, the known quirk).
pub const CTA_RING_WIDTH: f32 = 1.0;
pub const CTA_RING_OFFSET: f32 = 1.0;
pub const CTA_RING_INK: mv_nvg::Color = [0.0, 95.0 / 255.0, 204.0 / 255.0, 1.0];

/// Lays out the welcome page in `app`'s Clay context against `size` at `fit`
/// (both the window and 1 for now) and returns the frame's render commands.
/// The caller opens the layout first and ends it after.
pub fn layout_welcome(app: &mut App, size: [f32; 2], fit: f32) -> Vec<clay::RenderCommand> {
    app.clay
        .as_mut()
        .expect("clay context initialised")
        .set_layout_dimensions(clay::Dimensions {
            width: size[0],
            height: size[1],
        });
    app.clay
        .as_mut()
        .expect("clay context initialised")
        .begin_layout();
    let frame = [size[0] / fit, size[1] / fit];
    let mut root = crate::ui::empty_decl();
    root.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_fixed(size[0]),
        height: mv_clay::sizing_fixed(size[1]),
    };
    crate::ui::ui_scope(app, mv_clay::id("WelcomeRoot", 0), &root, |app| {
        // DOM order: the top row, then the frame; the menu floats last.
        top_bar(app, size);
        content_frame(app, frame);
        // The settings menu floats under the gear, above the page, as on the
        // viewer: its regions ride one layer up, over the same pixels.
        app.input.z = 1;
        crate::settings::menu(app);
        app.input.z = 0;
    });
    app.clay
        .as_mut()
        .expect("clay context initialised")
        .end_layout()
        .to_vec()
}

/// The top row: the mark left, the gear right, 40 tall at top 24 with 32 px
/// sides (`top: 24, left/right 32, height 40, z-index 1`). A floating row, so
/// the frame below stays full-size for its safe centering; h10-fit's zoom
/// (not a second layout) shrinks it later.
fn top_bar(app: &mut App, size: [f32; 2]) {
    let t = crate::ui::current_theme(app).clone();
    let mut bar = crate::ui::empty_decl();
    bar.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_fixed(size[0] - 64.0),
        height: mv_clay::sizing_fixed(GEAR_SIZE),
    };
    bar.layout.childAlignment = clay::ChildAlignment {
        x: clay::LayoutAlignmentX::Left,
        y: clay::LayoutAlignmentY::Center,
    };
    bar.floating = clay::FloatingElementConfig {
        offset: [32.0, 24.0],
        expand: clay::Dimensions {
            width: 0.0,
            height: 0.0,
        },
        parentId: mv_clay::id("WelcomeRoot", 0).id,
        zIndex: 1,
        attachment: clay::FloatingAttachPoints {
            element: clay::FloatingAttachPointType::LeftTop,
            parent: clay::FloatingAttachPointType::LeftTop,
        },
        pointerCaptureMode: clay::PointerCaptureMode::Passthrough,
        attachTo: clay::FloatingAttachToElement::ElementWithId,
        clipTo: clay::FloatingClipToElement::None,
    };
    crate::ui::ui_scope(app, mv_clay::id("WelcomeTopBar", 0), &bar, |app| {
        // The mark (the welcome `Logo`, no link: only the viewer's wordmark
        // navigates).
        let mut mark = crate::ui::empty_decl();
        mark.layout.sizing = clay::Sizing {
            width: mv_clay::sizing_fixed(MARK_SIZE),
            height: mv_clay::sizing_fixed(MARK_SIZE),
        };
        mark.custom = crate::icons::custom_data(crate::icons::Icon {
            kind: crate::icons::IconKind::Logo,
            ink: t.colors[mv_theme::Token::Primary],
        });
        crate::ui::ui_scope(app, mv_clay::id("WelcomeMark", 0), &mark, |_| {});
        // The free width the gear's `margin-left: auto` absorbs.
        let mut spacer = crate::ui::empty_decl();
        spacer.layout.sizing = clay::Sizing {
            width: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
            height: mv_clay::sizing_fixed(GEAR_SIZE),
        };
        crate::ui::ui_scope(app, mv_clay::id("WelcomeSpacer", 0), &spacer, |_| {});
        // The gear button, as the viewer's: 40 px round, secondary, bordered,
        // the 22 px icon; `settings.rs`'s `GEAR_ID`, so `paint_chrome` and
        // the menu anchor under it exactly as on the viewer.
        let mut button = crate::ui::empty_decl();
        button.layout.sizing = clay::Sizing {
            width: mv_clay::sizing_fixed(GEAR_SIZE),
            height: mv_clay::sizing_fixed(GEAR_SIZE),
        };
        button.layout.childAlignment = clay::ChildAlignment {
            x: clay::LayoutAlignmentX::Center,
            y: clay::LayoutAlignmentY::Center,
        };
        let ink = |token| crate::ui::ink(&t, token);
        button.backgroundColor = ink(mv_theme::Token::Secondary);
        button.border = clay::BorderElementConfig {
            color: ink(mv_theme::Token::Border),
            width: mv_clay::border_outside(1),
        };
        button.cornerRadius = crate::ui::corner_radius_all(GEAR_SIZE / 2.0);
        crate::ui::ui_scope(
            app,
            mv_clay::id(crate::top_bar::GEAR_ID, 0),
            &button,
            |app| {
                icon(
                    app,
                    mv_clay::id("WelcomeGearIcon", 0),
                    crate::icons::IconKind::Gear,
                    crate::top_bar::GEAR_ICON,
                    t.colors[mv_theme::Token::Foreground],
                );
            },
        );
        crate::input::region(
            app,
            mv_clay::id(crate::top_bar::GEAR_ID, 0),
            Hit {
                kind: HitKind::Gear,
                index: 0,
            },
            Cursor::Pointer,
            Cursor::Pointer,
            true,
            crate::ui::corner_radius_all(GEAR_SIZE / 2.0),
        );
    });
}

/// The centred frame holding the content row: the intro column beside the
/// hero, 60 px apart (`align-items/justify-content: safe center`).
fn content_frame(app: &mut App, frame: [f32; 2]) {
    let mut outer = crate::ui::empty_decl();
    outer.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_fixed(frame[0]),
        height: mv_clay::sizing_fixed(frame[1]),
    };
    outer.layout.childGap = CONTENT_GAP;
    outer.layout.childAlignment = clay::ChildAlignment {
        x: clay::LayoutAlignmentX::Center,
        y: clay::LayoutAlignmentY::Center,
    };
    crate::ui::ui_scope(app, mv_clay::id("WelcomeFrame", 0), &outer, |app| {
        intro(app);
        hero(app);
    });
}

/// The intro column: 380 wide, gap 28.
fn intro(app: &mut App) {
    let t = crate::ui::current_theme(app).clone();
    let ink = |token| crate::ui::ink(&t, token);
    let mut column = crate::ui::empty_decl();
    column.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_fixed(INTRO_WIDTH),
        height: mv_clay::sizing_fit(crate::ui::ZERO_MINMAX),
    };
    column.layout.childGap = INTRO_GAP;
    column.layout.layoutDirection = clay::LayoutDirection::TopToBottom;
    crate::ui::ui_scope(app, mv_clay::id("WelcomeIntro", 0), &column, |app| {
        crate::typography::text(
            app,
            mv_clay::id("WelcomeTitle", 0),
            crate::typography::Type::WelcomeTitle,
            TITLE_TEXT,
            ink(mv_theme::Token::WelcomeTitle),
            true,
        );
        description(app, &t);
        actions(app, &t);
    });
}

/// The description: the three reference line breaks, each a 33 px line in
/// the 380 px paragraph.
fn description(app: &mut App, t: &mv_theme::Theme) {
    let mut para = crate::ui::empty_decl();
    para.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_fixed(INTRO_WIDTH),
        height: mv_clay::sizing_fit(crate::ui::ZERO_MINMAX),
    };
    para.layout.layoutDirection = clay::LayoutDirection::TopToBottom;
    crate::ui::ui_scope(app, mv_clay::id("WelcomeDescription", 0), &para, |app| {
        for (index, line) in DESCRIPTION_LINES.iter().enumerate() {
            crate::typography::text(
                app,
                mv_clay::id("WelcomeDescriptionLine", index as u32),
                crate::typography::Type::WelcomeDescription,
                line,
                crate::ui::ink(t, mv_theme::Token::ForegroundMuted),
                true,
            );
        }
    });
}

/// The actions column: the button over the hint, 18 px apart, starting
/// where the column starts.
fn actions(app: &mut App, t: &mv_theme::Theme) {
    let mut column = crate::ui::empty_decl();
    column.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_fit(crate::ui::ZERO_MINMAX),
        height: mv_clay::sizing_fit(crate::ui::ZERO_MINMAX),
    };
    column.layout.childGap = ACTIONS_GAP;
    column.layout.layoutDirection = clay::LayoutDirection::TopToBottom;
    column.layout.childAlignment = clay::ChildAlignment {
        x: clay::LayoutAlignmentX::Left,
        y: clay::LayoutAlignmentY::Top,
    };
    crate::ui::ui_scope(app, mv_clay::id("WelcomeActions", 0), &column, |app| {
        cta(app, t);
        hint(app, t);
    });
}

/// The button: an inline-flex row to `#function-viewer` (padding 16 24 16
/// 26, 18 px / 600, gap 12, the 14 px arrow, radius 4, `--cta` on
/// `--cta-foreground`; no hover style).
fn cta(app: &mut App, t: &mv_theme::Theme) {
    let cta_ink = crate::ui::ink(t, mv_theme::Token::WelcomeCtaForeground);
    let mut button = crate::ui::empty_decl();
    button.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_fit(crate::ui::ZERO_MINMAX),
        height: mv_clay::sizing_fit(crate::ui::ZERO_MINMAX),
    };
    button.layout.padding = clay::Padding {
        left: CTA_PAD_LEFT,
        right: CTA_PAD_RIGHT,
        top: CTA_PAD_TOP,
        bottom: CTA_PAD_BOTTOM,
    };
    button.layout.childGap = CTA_GAP;
    button.layout.childAlignment = clay::ChildAlignment {
        x: clay::LayoutAlignmentX::Left,
        y: clay::LayoutAlignmentY::Center,
    };
    button.backgroundColor = crate::ui::ink(t, mv_theme::Token::WelcomeCta);
    button.cornerRadius = crate::ui::corner_radius_all(WELCOME_RADIUS);
    crate::ui::ui_scope(app, mv_clay::id("WelcomeCta", 0), &button, |app| {
        crate::typography::text(
            app,
            mv_clay::id("WelcomeCtaText", 0),
            crate::typography::Type::WelcomeCta,
            CTA_LABEL,
            cta_ink,
            false,
        );
        icon(
            app,
            mv_clay::id("WelcomeCtaIcon", 0),
            crate::icons::IconKind::Arrow,
            CTA_ICON,
            t.colors[mv_theme::Token::WelcomeCtaForeground],
        );
    });
    crate::input::region(
        app,
        mv_clay::id("WelcomeCta", 0),
        Hit {
            kind: HitKind::WelcomeCta,
            index: 0,
        },
        Cursor::Pointer,
        Cursor::Pointer,
        true,
        crate::ui::corner_radius_all(WELCOME_RADIUS),
    );
}

/// The hint: the 16 px copy with the four 26 px chips, gap 8.
fn hint(app: &mut App, t: &mv_theme::Theme) {
    let mut row = crate::ui::empty_decl();
    row.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_fit(crate::ui::ZERO_MINMAX),
        height: mv_clay::sizing_fit(crate::ui::ZERO_MINMAX),
    };
    row.layout.childGap = HINT_GAP;
    row.layout.childAlignment = clay::ChildAlignment {
        x: clay::LayoutAlignmentX::Left,
        y: clay::LayoutAlignmentY::Center,
    };
    crate::ui::ui_scope(app, mv_clay::id("WelcomeHint", 0), &row, |app| {
        crate::typography::text(
            app,
            mv_clay::id("WelcomeHintText", 0),
            crate::typography::Type::WelcomeHint,
            HINT_TEXT,
            crate::ui::ink(t, mv_theme::Token::ForegroundMuted),
            false,
        );
        for (index, param) in HINT_PARAMS.iter().enumerate() {
            let mut chip = crate::ui::empty_decl();
            chip.layout.sizing = clay::Sizing {
                width: mv_clay::sizing_fixed(CHIP_SIZE),
                height: mv_clay::sizing_fixed(CHIP_SIZE),
            };
            chip.layout.childAlignment = clay::ChildAlignment {
                x: clay::LayoutAlignmentX::Center,
                y: clay::LayoutAlignmentY::Center,
            };
            chip.backgroundColor = crate::ui::ink(t, param.colour_token());
            chip.cornerRadius = crate::ui::corner_radius_all(WELCOME_RADIUS);
            let id = match index {
                0 => "WelcomeChipA",
                1 => "WelcomeChipK",
                2 => "WelcomeChipB",
                _ => "WelcomeChipH",
            };
            crate::ui::ui_scope(app, mv_clay::id(id, 0), &chip, |app| {
                crate::typography::text(
                    app,
                    mv_clay::id("WelcomeChipLetter", index as u32),
                    crate::typography::Type::ChipLetter,
                    param.as_str(),
                    crate::ui::ink(t, mv_theme::Token::Background),
                    false,
                );
            });
        }
    });
}

/// The hero slot: 840 × 520; the plane frame (`--background` fill, 1 px
/// `--border`, radius 4) whose 838 × 518 content box the still paints in.
/// The frame carries no Clay paint: `paint_hero` fills its background, inks
/// and border in one pass, so the menu's opaque background (painted with the
/// commands after) covers the still where they overlap, as the popover
/// stacks above the page.
fn hero(app: &mut App) {
    let mut slot = crate::ui::empty_decl();
    slot.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_fixed(HERO_WIDTH),
        height: mv_clay::sizing_fixed(HERO_HEIGHT),
    };
    crate::ui::ui_scope(app, mv_clay::id("WelcomeHero", 0), &slot, |app| {
        let mut frame = crate::ui::empty_decl();
        frame.layout.sizing = clay::Sizing {
            width: mv_clay::sizing_fixed(HERO_WIDTH),
            height: mv_clay::sizing_fixed(HERO_HEIGHT),
        };
        crate::ui::ui_scope(app, mv_clay::id("WelcomeHeroPlane", 0), &frame, |_| {});
    });
}

/// Declares a square icon element painting `kind` in `ink`, as the top bar's
/// own helper does.
fn icon(
    app: &mut App,
    id: clay::ElementId,
    kind: crate::icons::IconKind,
    size: f32,
    ink: mv_nvg::Color,
) {
    let mut decl = crate::ui::empty_decl();
    decl.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_fixed(size),
        height: mv_clay::sizing_fixed(size),
    };
    decl.custom = crate::icons::custom_data(crate::icons::Icon { kind, ink });
    crate::ui::ui_scope(app, id, &decl, |_| {});
}

/// The hero's boxed content: the border box inside its 1 px border, as the
/// plane's own `canvas_content` does. `page.rs` shares it for the hero paint.
pub(crate) fn hero_content(border: &clay::BoundingBox) -> clay::BoundingBox {
    clay::BoundingBox {
        x: border.x + crate::plane::CANVAS_BORDER,
        y: border.y + crate::plane::CANVAS_BORDER,
        width: border.width - 2.0 * crate::plane::CANVAS_BORDER,
        height: border.height - 2.0 * crate::plane::CANVAS_BORDER,
    }
}

/// Paints the still into the hero frame: the `--background` fill, the bare
/// plane (the grid, axes and labels, then the still's curves — the dashed
/// original behind the solid one — with no handles, no lines and no
/// markers), the 1 px `--border` at radius 4, and the rounded cutouts — as
/// `HeroPlane.tsx` draws them through `drawCartesianPlane` with empty extras
/// and no legend.
pub fn paint_hero(app: &mut App, theme: &mv_theme::Theme, frame: &clay::BoundingBox, density: f32) {
    // The frame's background fill, under the ink (the DOM's canvas element
    // above its own background).
    app.vg.begin_path();
    crate::draw::rounded_rect(
        &mut app.vg,
        crate::draw::snap_box(*frame, density),
        crate::ui::corner_radius_all(WELCOME_RADIUS),
    );
    app.vg.fill_color(theme.colors[mv_theme::Token::Background]);
    app.vg.fill();
    let content = hero_content(frame);
    // The bare plane (h10-seam's split): the still's curves through the
    // transform, with empty extras.
    let colors = crate::plane::plane_colors(theme);
    let shown =
        crate::plot::Shown::new(crate::functions::base(crate::functions::FunctionSlug::SinX));
    let still =
        crate::transform_curves::transform_curves(&crate::plot::curves(&shown), WELCOME_STILL);
    let mcolors = crate::plane_markers::marker_colors(theme);
    crate::plane::paint_plane(
        &mut app.fonts,
        &mut app.vg,
        &content,
        crate::camera::DEFAULT_CENTER,
        crate::camera::DEFAULT_SCALE,
        &colors,
        true,
        &still,
        theme.colors[mv_theme::Token::ChartLine],
        theme.colors[mv_theme::Token::ForegroundMuted],
        &[],
        &[],
        theme.colors[mv_theme::Token::Primary],
        &[],
        &mcolors,
    );
    // The 1 px `--border` at radius 4 over the ink (the DOM's border above
    // the canvas): the command walk's border shape, stroked in the translated
    // pass — the frame box inset half a px, corners shrunk the same.
    let border_box = clay::BoundingBox {
        x: -crate::plane::CANVAS_BORDER,
        y: -crate::plane::CANVAS_BORDER,
        width: HERO_WIDTH,
        height: HERO_HEIGHT,
    };
    let width = crate::plane::CANVAS_BORDER;
    let inset = clay::BoundingBox {
        x: border_box.x + width / 2.0,
        y: border_box.y + width / 2.0,
        width: (border_box.width - width).max(0.0),
        height: (border_box.height - width).max(0.0),
    };
    app.vg.begin_path();
    crate::draw::rounded_rect(
        &mut app.vg,
        crate::draw::snap_box(inset, density),
        crate::draw::inset_radius(crate::ui::corner_radius_all(WELCOME_RADIUS), width / 2.0),
    );
    app.vg.stroke_color(theme.colors[mv_theme::Token::Border]);
    app.vg.stroke_width(width);
    app.vg.stroke();
    app.vg.restore();
}

/// The backdrop's call: paper and lights behind the frame (h10-hero paints
/// it here, after the page background and before the frame's content; the
/// top row paints above it, the menu last). Nothing until it merges.
pub fn paint_backdrop(_app: &mut App) {}

/// Goes to the route with the query, as the anchors do: the button to
/// `#function-viewer` with an empty query, the wordmark to `#`. On the web
/// the hash is assigned (a history entry; the existing `hashchange` path
/// re-reads the route and query), natively the URL is set and the same
/// route-entered step runs.
pub fn navigate(app: &mut App, route: &str, query: &str) {
    let hash = if query.is_empty() {
        route.to_string()
    } else {
        format!("{route}?{query}")
    };
    #[cfg(target_os = "emscripten")]
    {
        let _ = app;
        crate::platform_web::platform_navigate_hash(&hash);
    }
    #[cfg(not(target_os = "emscripten"))]
    {
        let prev = app.url.route().to_string();
        app.url.sync_from_hash(&format!("#{hash}"));
        if prev != app.url.route() {
            enter_route(app);
        }
        app.dirty = true;
    }
}

/// Enters the route `app.url` now holds: a fresh mount. Entering the viewer
/// resets its component state (drafts, scrub and holds, the open menu, hover
/// and cursor, the flip and plot animations adopted whole at the URL's
/// values, the camera's glide stopped, the overlay tip, `moved_by`, the
/// slider and the field) with the params and camera from the URL (bare
/// `#function-viewer`: every default); the view atoms (the lit, hover and
/// focus lists, jumps, `p_dragging`, `q_x`) keep their values, as jotai's
/// do. Entering the welcome page closes its menu (h10-loop restarts its
/// loop clock).
pub fn enter_route(app: &mut App) {
    match Route::of(&app.url) {
        Route::FunctionViewer => {
            crate::settings::close_menu(app);
            app.drafts = Default::default();
            app.scrub = None;
            app.scrub_hold = None;
            app.scrub_rest = None;
            app.scrub_resting = None;
            app.overlay_tip_since = None;
            app.timers[crate::app_loop::Timer::OverlayTip] = 0;
            app.moved_by = crate::moved_by::MovedBy::new();
            app.slider_hold = None;
            app.slider_edit = false;
            app.x_field = crate::x_field::XField::new();
            app.flip = crate::flip::FlipState::new();
            app.input.hover = Hit::default();
            app.input.focus = Hit::default();
            app.input.focus_visible = false;
            app.camera_frame_at = 0;
            crate::plane_input::init_camera_from_url(app);
            crate::transform_section::init_params_from_url(app);
            crate::selector::init_func_from_url(app);
            app.dirty = true;
        }
        Route::Welcome => {
            crate::settings::close_menu(app);
            app.input.hover = Hit::default();
            app.input.focus = Hit::default();
            app.input.focus_visible = false;
            app.dirty = true;
        }
    }
}

/// Routes a click on the welcome route: the release whose down-hit and
/// up-hit agree on the button navigates, as the anchor would.
pub fn click(app: &mut App, down: Hit) {
    let up = app.input.hover;
    if down != up {
        return;
    }
    if down.kind == HitKind::WelcomeCta {
        navigate(app, VIEWER_ROUTE, "");
    }
}

/// Routes a key on the welcome route with the menu closed (open, the
/// settings menu walks its own stops): Tab walks the gear and the link, and
/// Space or Enter on the link navigates. Returns whether the key was
/// consumed.
pub fn welcome_key(app: &mut App, event: &crate::input::KeyEvent) -> bool {
    use sdl3::keyboard::Keycode;
    if !event.down || event.repeat {
        return false;
    }
    let shift = event
        .mods
        .intersects(sdl3::keyboard::Mod::LSHIFTMOD | sdl3::keyboard::Mod::RSHIFTMOD);
    match event.key {
        Keycode::Tab => {
            crate::input::move_focus(app, if shift { -1 } else { 1 });
            true
        }
        Keycode::Space | Keycode::Return if app.input.focus.kind == HitKind::WelcomeCta => {
            navigate(app, VIEWER_ROUTE, "");
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::TransformParam;
    use crate::url::Url;

    #[test]
    fn route_reads_from_the_url() {
        // `""` is the welcome page, `function-viewer` the viewer, any other
        // word the welcome page (the template's not-found page is not
        // ported); slashes trim as the router trims them.
        assert_eq!(Route::of(&Url::new("")), Route::Welcome);
        assert_eq!(Route::of(&Url::new("#")), Route::Welcome);
        assert_eq!(Route::of(&Url::new("#/")), Route::Welcome);
        assert_eq!(Route::of(&Url::from_query("")), Route::Welcome);
        assert_eq!(
            Route::of(&Url::new("#function-viewer")),
            Route::FunctionViewer
        );
        assert_eq!(
            Route::of(&Url::new("#function-viewer?center=1,2&scale=2")),
            Route::FunctionViewer
        );
        assert_eq!(
            Route::of(&Url::new("#function-viewer/")),
            Route::FunctionViewer
        );
        assert_eq!(Route::of(&Url::new("#theme-demo")), Route::Welcome);
    }

    #[test]
    fn still_is_the_reference_still() {
        // `welcomeStill` at 68e1c7b: sin x at a 3, k 0, b 1.48, h 1.
        assert_eq!(
            (
                WELCOME_STILL.a,
                WELCOME_STILL.b,
                WELCOME_STILL.h,
                WELCOME_STILL.k
            ),
            (3.0, 1.48, 1.0, 0.0)
        );
    }

    #[test]
    fn description_breaks_match_the_reference() {
        // The reference wraps the sentence greedily in 380 px (h10-page's
        // probe: three lines, 367.49, 318.91 and 216.0 px wide at a 33 px
        // pitch); Clay text never wraps, so the port lays the breaks out.
        // The breaks are the greedy fit in 380 px: the longest prefix each
        // line holds, widest word well clear of the 380 px edge, so the
        // browser's shaper (within a level of the port's) breaks the same
        // way. Widths smoke-match the reference within a pixel (the port's
        // shaper reads about 0.3 % narrower on the third line: 215.2 vs
        // 216.0, edges either way).
        let _guard = mv_clay::tests_lock();
        let mut app = crate::fixture::headless_app();
        assert!(mv_text::init(&mut app.fonts));
        let style = mv_text::Style {
            face: mv_text::Face::WorkSans400,
            size: 22.0,
            tracking: 0.0,
            features: mv_text::Features::default(),
        };
        let words: Vec<&str> = DESCRIPTION_LINES
            .iter()
            .flat_map(|line| line.split(' '))
            .collect();
        let mut lines: Vec<String> = Vec::new();
        let mut current = String::new();
        for word in words {
            let longer = if current.is_empty() {
                word.to_string()
            } else {
                format!("{current} {word}")
            };
            if mv_text::measure(&mut app.fonts, style, &longer) <= INTRO_WIDTH {
                current = longer;
            } else {
                lines.push(current);
                current = word.to_string();
            }
        }
        lines.push(current);
        assert_eq!(
            lines,
            DESCRIPTION_LINES
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        );
        for (line, want) in DESCRIPTION_LINES.iter().zip([367.49, 318.91, 216.0]) {
            let width = mv_text::measure(&mut app.fonts, style, line);
            assert!(
                (width - want).abs() <= 1.0,
                "{line:?} measures {width}, the reference {want}"
            );
            assert!(width <= INTRO_WIDTH, "{line:?} overflows the 380 px column");
        }
    }

    #[test]
    fn entering_the_viewer_resets_its_component_state() {
        let mut app = crate::fixture::headless_app();
        app.url = Url::new("#function-viewer?center=1.5000,-2.0000&scale=2.0000&a=2.000");
        // A lived-in viewer: the menu open, hover and focus set, the camera
        // flown, a param set but not flushed, a draft typed, the tip armed.
        app.settings.open();
        app.input.hover = Hit {
            kind: HitKind::Gear,
            index: 0,
        };
        app.input.focus = Hit {
            kind: HitKind::Gear,
            index: 0,
        };
        app.input.focus_visible = true;
        app.camera = crate::camera::Camera::new([5.0, 5.0], 3.0);
        app.a_param.set(9.0, 0);
        app.drafts[TransformParam::A] = crate::number_field::FieldState {
            draft: Some("9".to_string()),
            caret: [0, 1],
            typing_from: Some(2.0),
        };
        app.overlay_tip_since = Some(1);
        enter_route(&mut app);
        // Fresh mount: the menu closed, hover and focus dropped, the camera
        // and params from the URL (bare defaults where missing), the flip
        // and plot adopted whole, no pending writes, no drafts, no tip.
        assert!(!app.settings.open);
        assert_eq!(app.input.hover, Hit::default());
        assert_eq!(app.input.focus, Hit::default());
        assert!(!app.input.focus_visible);
        assert_eq!(app.camera.center, [1.5, -2.0]);
        assert_eq!(app.camera.scale, 2.0);
        assert!(!app.camera.animating());
        assert_eq!(*app.a_param.get(), 2.0);
        assert!(!app.a_param.has_pending());
        assert_eq!(
            crate::flip::shown_transform(&app.flip, crate::flip::live_transform(&app)),
            crate::flip::live_transform(&app)
        );
        assert!(!app.flip.is_animating());
        assert_eq!(app.drafts[TransformParam::A].draft, None);
        assert_eq!(app.overlay_tip_since, None);
        assert!(app.dirty);
        // The view atoms keep their values: nothing here stamps them.
        assert!(app.state.lit().is_empty());
    }

    #[test]
    fn entering_the_welcome_page_closes_its_menu() {
        let mut app = crate::fixture::headless_app();
        app.url = Url::new("#");
        app.settings.open();
        app.input.hover = Hit {
            kind: HitKind::Gear,
            index: 0,
        };
        enter_route(&mut app);
        assert!(!app.settings.open);
        assert_eq!(app.input.hover, Hit::default());
        assert!(app.dirty);
    }

    #[test]
    fn navigation_sets_the_route_and_enters_it() {
        // Native only (the web build assigns `location.hash` instead): the
        // button to `#function-viewer` with an empty query, the wordmark to
        // `#`.
        let mut app = crate::fixture::headless_app();
        assert_eq!(Route::of(&app.url), Route::Welcome);
        navigate(&mut app, VIEWER_ROUTE, "");
        assert_eq!(app.url.route(), "function-viewer");
        assert_eq!(app.url.query(), "");
        navigate(&mut app, "", "");
        assert_eq!(app.url.route(), "");
        assert_eq!(app.url.query(), "");
    }
}

#[cfg(test)]
mod layout_tests {
    use super::*;

    /// The layout slack: boxes within half a CSS pixel of the reference's.
    const SLACK: f32 = 0.5;

    /// The welcome page's boxes match the reference's, in CSS pixels: the
    /// top row, the intro column and the hero's plane at the design size.
    #[test]
    fn boxes_match_fixture() {
        let _guard = mv_clay::tests_lock();
        let mut app = crate::fixture::headless_app();
        assert!(mv_text::init(&mut app.fonts));
        crate::ui::init_clay(&mut app);
        app.size = [DESIGN_WIDTH, DESIGN_HEIGHT];
        app.pixels = [2880, 1800];
        let size = app.size;
        let _commands = layout_welcome(&mut app, size, 1.0);
        let clay = app.clay.as_ref().expect("clay context initialised");
        let fixture = crate::fixture::page_fixture().expect("page fixture parses");
        let shot = crate::fixture::fixture_shot(&fixture, "welcome-default")
            .expect("welcome-default shot in fixture");
        for (id, name) in [
            (mv_clay::id("WelcomeMark", 0), "mark"),
            (mv_clay::id(crate::top_bar::GEAR_ID, 0), "gear"),
            (mv_clay::id("WelcomeTitle", 0), "title"),
            (mv_clay::id("WelcomeDescription", 0), "description"),
            (mv_clay::id("WelcomeCta", 0), "cta"),
            (mv_clay::id("WelcomeHint", 0), "hint"),
            (mv_clay::id("WelcomeChipA", 0), "welcome_chip_a"),
            (mv_clay::id("WelcomeChipK", 0), "welcome_chip_k"),
            (mv_clay::id("WelcomeChipB", 0), "welcome_chip_b"),
            (mv_clay::id("WelcomeChipH", 0), "welcome_chip_h"),
            (mv_clay::id("WelcomeHeroPlane", 0), "hero_plane"),
        ] {
            let data = clay.get_element_data(id);
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
                    "{name}[{i}]: the port lays out {got}, the reference {want}"
                );
            }
        }
    }
}
