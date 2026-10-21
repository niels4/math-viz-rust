//! The settings menu: the gear's popover under the top bar with the light/dark/system
//! radios and the two theme lists with swatches (`SettingsMenu.tsx` at `adb0534`).
//!
//! The menu's theme selections live in `crate::theme_state::ThemeState`; this module
//! owns the menu's own state (the open flag and keyboard focus), the layout, the
//! swatch painter, and the pointer/keyboard routing. The popover API becomes an
//! explicit `open` bool: the menu lays out only while open, an outside pointer-down
//! or Esc closes it, picking keeps it open, and the 160 ms open/close transition is
//! skipped (the menu appears at rest).

use std::fmt;

use mv_sys::clay;

use crate::app::App;
use crate::input::{Cursor, Hit, HitKind};
use crate::theme_state::{LightDark, SystemScheme, ThemeSlug, ThemeState};

/// `getLightDarkSelectionMessage`: with `system`, `"Your system is {dark|light}
/// right now."`, otherwise `"Always {dark|light}."`.
pub fn light_dark_message(light_dark: LightDark, system: SystemScheme) -> String {
    match light_dark {
        LightDark::System => format!(
            "Your system is {} right now.",
            match system {
                SystemScheme::Dark => "dark",
                SystemScheme::Light => "light",
            }
        ),
        option => format!("Always {}.", option.word()),
    }
}

/// Theme swatch geometry (`ThemeSwatch`): a 30x20 svg, the curve
/// `M6 4.63Q15 25.37 24 4.63` stroked 1.75 in chart-line, the point
/// `(21, 10.39)` of radius 2.25 filled chart-accent.
pub mod swatch {
    pub const WIDTH: f64 = 30.0;
    pub const HEIGHT: f64 = 20.0;
    pub const RADIUS: f32 = 5.0;
    pub const OUTLINE: f32 = 1.0;
    pub const CURVE: &str = "M6 4.63Q15 25.37 24 4.63";
    pub const STROKE_WIDTH: f32 = 1.75;
    pub const POINT: (f64, f64) = (21.0, 10.39);
    pub const POINT_RADIUS: f32 = 2.25;
}

/// Paints all three rows' swatches over the frame, from their laid-out boxes:
/// the theme's background rounded to 5 px with a 1 px border outline, the
/// curve in chart-line, the point in chart-accent. A post-pass like the gear
/// chrome, so the frame needs no second custom payload type beside the icons'.
pub fn paint_swatches(app: &mut App) {
    if !app.settings.open {
        return;
    }
    let scheme = app.theme.resolved_scheme();
    let slugs = shown_slugs(scheme);
    let mut boxes = [[0.0; 2]; 3];
    {
        let clay = app.clay.as_ref().expect("clay context initialised");
        for (i, spot) in boxes.iter_mut().enumerate() {
            let data = clay.get_element_data(mv_clay::id("MenuSwatch", i as u32));
            if data.found {
                *spot = [data.boundingBox.x, data.boundingBox.y];
            }
        }
    }
    for (at, slug) in boxes.iter().zip(slugs.iter()) {
        draw_swatch(app, *at, *slug);
    }
}

/// Paints one swatch in its box.
pub fn draw_swatch(app: &mut App, at: [f32; 2], slug: ThemeSlug) {
    let theme = &app.themes[port_slug(slug)];
    let bg = crate::draw::nvg_color(crate::ui::ink(theme, mv_theme::Token::Background));
    let border = crate::draw::nvg_color(crate::ui::ink(theme, mv_theme::Token::Border));
    let line = crate::draw::nvg_color(crate::ui::ink(theme, mv_theme::Token::ChartLine));
    let accent = crate::draw::nvg_color(crate::ui::ink(theme, mv_theme::Token::ChartAccent));
    let vg = &mut app.vg;
    vg.save();
    vg.translate(at[0], at[1]);
    vg.begin_path();
    vg.rounded_rect(
        0.0,
        0.0,
        swatch::WIDTH as f32,
        swatch::HEIGHT as f32,
        swatch::RADIUS,
    );
    vg.fill_color(bg);
    vg.fill();
    vg.begin_path();
    vg.rounded_rect(
        swatch::OUTLINE / 2.0,
        swatch::OUTLINE / 2.0,
        swatch::WIDTH as f32 - swatch::OUTLINE,
        swatch::HEIGHT as f32 - swatch::OUTLINE,
        swatch::RADIUS - swatch::OUTLINE / 2.0,
    );
    vg.stroke_color(border);
    vg.stroke_width(swatch::OUTLINE);
    vg.stroke();
    vg.begin_path();
    if !crate::svgpath::draw_path(vg, swatch::CURVE) {
        eprintln!(
            "math-viz2-rust: swatch path NanoVG can't draw: {:?}",
            swatch::CURVE
        );
    }
    vg.stroke_color(line);
    vg.stroke_width(swatch::STROKE_WIDTH);
    vg.line_cap(mv_nvg::LineCapType::ROUND);
    vg.stroke();
    vg.begin_path();
    vg.circle(
        swatch::POINT.0 as f32,
        swatch::POINT.1 as f32,
        swatch::POINT_RADIUS,
    );
    vg.fill_color(accent);
    vg.fill();
    vg.restore();
}

/// Keyboard focus within the open menu. The DOM gives each radio group one tab
/// stop (its checked option); the arrows move and check within the group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusSlot {
    LightDark(LightDark),
    Theme(usize),
}

impl fmt::Display for FocusSlot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FocusSlot::LightDark(option) => write!(f, "light-dark:{}", option.word()),
            FocusSlot::Theme(index) => write!(f, "theme:{index}"),
        }
    }
}

/// The menu's interactive state: the popover's open flag, plus whether hover
/// is held off. A pick drops hover like the reference's re-render does, and
/// Chromium does not re-hover without a mouse move, so the hold suppresses
/// the hover recompute until the pointer moves. Keyboard focus itself lives
/// in the shell's input state (`app.input.focus`); the menu maps it to stops
/// with `stop_of`, and Tab/Shift+Tab walk the group stops.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SettingsMenuState {
    pub open: bool,
    pub hover_held: bool,
}

impl SettingsMenuState {
    pub fn closed() -> Self {
        Self {
            open: false,
            hover_held: false,
        }
    }

    pub fn open(&mut self) {
        self.open = true;
    }

    pub fn close(&mut self) {
        self.open = false;
        self.hover_held = false;
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
        if !self.open {
            self.hover_held = false;
        }
    }

    /// Tab from the current stop: gear, then the checked light-dark cell, then
    /// the checked theme row, then out. The menu stays open throughout.
    pub fn tab(
        &self,
        current: Option<FocusSlot>,
        light_dark: LightDark,
        theme_index: usize,
    ) -> Option<FocusSlot> {
        if !self.open {
            return None;
        }
        match current {
            None => Some(FocusSlot::LightDark(light_dark)),
            Some(FocusSlot::LightDark(_)) => Some(FocusSlot::Theme(theme_index)),
            Some(FocusSlot::Theme(_)) => None,
        }
    }

    /// Shift+Tab walks the stops backwards.
    pub fn shift_tab(
        &self,
        current: Option<FocusSlot>,
        light_dark: LightDark,
        theme_index: usize,
    ) -> Option<FocusSlot> {
        if !self.open {
            return None;
        }
        match current {
            None => Some(FocusSlot::Theme(theme_index)),
            Some(FocusSlot::Theme(_)) => Some(FocusSlot::LightDark(light_dark)),
            Some(FocusSlot::LightDark(_)) => None,
        }
    }
}

/// Closes the menu and drops keyboard focus from its stops, which are gone.
pub fn close_menu(app: &mut App) {
    app.settings.close();
    app.input.focus = Hit::default();
    app.input.focus_visible = false;
    app.dirty = true;
}

/// A pick that changes the theme drops hover, like the reference's re-render
/// does; a same-value pick changes nothing, so hover stays. The hold lasts
/// until the pointer moves.
pub fn drop_hover(app: &mut App) {
    app.settings.hover_held = true;
    app.input.hover = Hit::default();
    app.dirty = true;
}

/// The pointer moved: hover recomputes from the new position again.
pub fn pointer_moved(app: &mut App) {
    app.settings.hover_held = false;
}

/// An arrow key within a radio group: the new checked index, wrapping round.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arrow {
    Left,
    Right,
    Up,
    Down,
}

pub fn arrow_index(current: usize, len: usize, arrow: Arrow) -> usize {
    debug_assert!(len > 0);
    match arrow {
        Arrow::Left | Arrow::Up => current.checked_sub(1).unwrap_or(len - 1),
        Arrow::Right | Arrow::Down => (current + 1) % len,
    }
}

/// The menu's layout constants, from `SettingsMenu.module.css` at `adb0534`
/// (root font 16 px: the rem values below are the computed pixels).
pub mod layout {
    /// The menu frame: 19 rem wide, `space-inset` padding, 4 px radius, 1 px border.
    /// Clay's borders take no layout space (the browser's `border-box` does),
    /// so each bordered element's Clay padding is its CSS padding plus border.
    pub const MENU_WIDTH: f32 = 304.0;
    pub const MENU_PAD: f32 = 12.0;
    pub const MENU_BORDER: u16 = 1;
    pub const MENU_RADIUS: f32 = 4.0;
    /// The margin under the gear the anchor leaves (`space-inline`).
    pub const MENU_OFFSET_Y: f32 = 8.0;
    /// The title and legend gap (`space-inline` margin under the legend).
    pub const TITLE_GAP: f32 = 8.0;
    /// The segmented control: 36 px tall, 1 px bordered, 4 px radius.
    pub const SEG_HEIGHT: f32 = 36.0;
    /// Three 92 px cells fill the 278 px content (276 + 2 px of container border).
    pub const CELL_WIDTH: f32 = 92.0;
    pub const CELL_PAD_X: f32 = 8.0;
    pub const CELL_GAP: f32 = 8.0;
    /// The selected segment's outer corners stay concentric with the control.
    pub const CELL_RADIUS: f32 = 3.0;
    /// The option icon: 16 px.
    pub const CELL_ICON: f32 = 16.0;
    /// A theme row: 36 px tall, `space-inset` side padding, 8 px gaps.
    pub const ROW_HEIGHT: f32 = 36.0;
    /// Height the reference's menu carries for the excluded Developer pages
    /// row that the port does not render (ruling: dev pages out of scope):
    /// 36 px row + (16 px `--space-section` gap - 8 px `calc(--space-inline - --space-section)` margin) = 44 px.
    pub const MENU_DEV_LINK_DELTA: f32 = 44.0;
    pub const ROW_PAD_X: f32 = 12.0;
    pub const ROW_GAP: f32 = 8.0;
    /// The selected row's check: 14 px, `margin-left: auto`, 1 px down from centre.
    pub const CHECK: f32 = 14.0;
    pub const CHECK_TOP: f32 = 1.0;
    /// The menu's Clay ids, for regions and the layout test.
    pub const MENU_ID: &str = "Menu";
}

/// The menu's three light/dark cells in order.
const LIGHT_DARK_ORDER: [LightDark; 3] = [LightDark::System, LightDark::Dark, LightDark::Light];

/// The option icon for a light/dark cell.
fn cell_icon(option: LightDark) -> crate::icons::IconKind {
    match option {
        LightDark::System => crate::icons::IconKind::System,
        LightDark::Dark => crate::icons::IconKind::Moon,
        LightDark::Light => crate::icons::IconKind::Sun,
    }
}

fn port_slug(slug: ThemeSlug) -> mv_theme::Slug {
    match slug {
        ThemeSlug::ArcticIce => mv_theme::Slug::ArcticIce,
        ThemeSlug::TronCyan => mv_theme::Slug::TronCyan,
        ThemeSlug::MidnightViolet => mv_theme::Slug::MidnightViolet,
        ThemeSlug::CleanTeal => mv_theme::Slug::CleanTeal,
        ThemeSlug::EducationalWarm => mv_theme::Slug::EducationalWarm,
        ThemeSlug::SageEditorial => mv_theme::Slug::SageEditorial,
    }
}

/// The shown theme list: the dark slugs under a dark scheme, the light slugs
/// under a light one (`DARK_THEME_SLUGS` / `LIGHT_THEME_SLUGS`).
fn shown_slugs(scheme: SystemScheme) -> [ThemeSlug; 3] {
    match scheme {
        SystemScheme::Dark => [
            ThemeSlug::ArcticIce,
            ThemeSlug::TronCyan,
            ThemeSlug::MidnightViolet,
        ],
        SystemScheme::Light => [
            ThemeSlug::CleanTeal,
            ThemeSlug::EducationalWarm,
            ThemeSlug::SageEditorial,
        ],
    }
}

/// The checked light-dark cell's index.
fn checked_cell(theme: &ThemeState) -> usize {
    LIGHT_DARK_ORDER
        .iter()
        .position(|option| *option == theme.light_dark)
        .expect("light/dark order covers the picks")
}

/// The checked theme row's index in the shown list.
fn checked_row(theme: &ThemeState) -> usize {
    let scheme = theme.resolved_scheme();
    shown_slugs(scheme)
        .iter()
        .position(|slug| *slug == theme.resolved())
        .expect("shown list holds the resolved theme")
}

/// The menu's focus stop for a hit, when the hit is a stop.
fn stop_of(hit: Hit) -> Option<FocusSlot> {
    match hit.kind {
        HitKind::SettingsLightDark if (hit.index as usize) < LIGHT_DARK_ORDER.len() => {
            Some(FocusSlot::LightDark(LIGHT_DARK_ORDER[hit.index as usize]))
        }
        HitKind::SettingsTheme => Some(FocusSlot::Theme(hit.index as usize)),
        _ => None,
    }
}

/// Clay padding for a `pad` CSS padding inside a 1 px border: the border
/// takes no layout space, so it joins the padding.
fn bordered(pad: f32) -> clay::Padding {
    let both = (pad as u16) + 1;
    clay::Padding {
        left: both,
        right: both,
        top: both,
        bottom: both,
    }
}

/// Lays out the open menu as a floating sibling under the gear: anchored
/// right to the gear's right bottom, 8 px below it, like the popover's
/// `top: anchor(bottom); right: anchor(right)`. Skipped while closed: the
/// hidden popover takes no part in the layout.
pub fn menu(app: &mut App) {
    menu_below(app, layout::MENU_OFFSET_Y);
}

/// Lays out the open menu with `dy` under the gear's box instead of the
/// usual 8 px: on the welcome page the menu keeps its natural size under the
/// zoomed gear (h10-fit: the popover's `zoom: calc(1 / var(--fit))`), so its
/// 8 screen px gap lays out as 8 / fit design px.
pub fn menu_below(app: &mut App, dy: f32) {
    if !app.settings.open {
        return;
    }
    let theme = app.theme;
    let scheme = theme.resolved_scheme();
    let slugs = shown_slugs(scheme);
    let mut decl = crate::ui::empty_decl();
    decl.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_fixed(layout::MENU_WIDTH),
        height: mv_clay::sizing_fit(crate::ui::ZERO_MINMAX),
    };
    decl.layout.padding = bordered(layout::MENU_PAD);
    let page = crate::ui::current_theme(app);
    let ink = |token| crate::ui::ink(page, token);
    decl.backgroundColor = ink(mv_theme::Token::Card);
    decl.border = clay::BorderElementConfig {
        color: ink(mv_theme::Token::Border),
        width: mv_clay::border_outside(1),
    };
    decl.cornerRadius = crate::ui::corner_radius_all(layout::MENU_RADIUS);
    decl.layout.childGap = layout::TITLE_GAP as u16;
    decl.layout.layoutDirection = clay::LayoutDirection::TopToBottom;
    decl.floating = clay::FloatingElementConfig {
        offset: [0.0, dy],
        expand: clay::Dimensions {
            width: 0.0,
            height: 0.0,
        },
        parentId: mv_clay::id(crate::top_bar::GEAR_ID, 0).id,
        zIndex: 1,
        attachment: clay::FloatingAttachPoints {
            element: clay::FloatingAttachPointType::RightTop,
            parent: clay::FloatingAttachPointType::RightBottom,
        },
        pointerCaptureMode: clay::PointerCaptureMode::Passthrough,
        attachTo: clay::FloatingAttachToElement::ElementWithId,
        clipTo: clay::FloatingClipToElement::None,
    };
    crate::ui::ui_scope(app, mv_clay::id(layout::MENU_ID, 0), &decl, |app| {
        appearance(app, &theme);
        // The inner column: the message over the theme list with a 16 px gap,
        // 8 px under the appearance section (the CSS `-8 px` top margin).
        let mut rest = crate::ui::empty_decl();
        rest.layout.sizing = clay::Sizing {
            width: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
            height: mv_clay::sizing_fit(crate::ui::ZERO_MINMAX),
        };
        rest.layout.childGap = 16;
        rest.layout.layoutDirection = clay::LayoutDirection::TopToBottom;
        crate::ui::ui_scope(app, mv_clay::id("MenuRest", 0), &rest, |app| {
            message(app, &theme);
            theme_list(app, &theme, scheme, &slugs);
        });
    });
}

/// The Appearance section: the caps legend over the three-cell segmented control.
fn appearance(app: &mut App, theme: &ThemeState) {
    let mut section = crate::ui::empty_decl();
    section.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
        height: mv_clay::sizing_fit(crate::ui::ZERO_MINMAX),
    };
    section.layout.layoutDirection = clay::LayoutDirection::TopToBottom;
    crate::ui::ui_scope(app, mv_clay::id("MenuAppearance", 0), &section, |app| {
        legend(app, "MenuAppearanceTitle", "APPEARANCE");
        segmented(app, theme);
    });
}

/// A caps section legend: full-bleed text with 12 px side padding and an 8 px
/// margin under it.
fn legend(app: &mut App, id: &str, text: &str) {
    // Scoped: the borrow ends before the layout takes `app` mutably.
    let muted = {
        let page = crate::ui::current_theme(app);
        crate::ui::ink(page, mv_theme::Token::ForegroundMuted)
    };
    let mut wrap = crate::ui::empty_decl();
    wrap.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
        height: mv_clay::sizing_fit(crate::ui::ZERO_MINMAX),
    };
    wrap.layout.padding = clay::Padding {
        left: layout::ROW_PAD_X as u16,
        right: layout::ROW_PAD_X as u16,
        top: 0,
        bottom: layout::TITLE_GAP as u16,
    };
    crate::ui::ui_scope(app, mv_clay::id(id, 0), &wrap, |app| {
        crate::typography::text(
            app,
            mv_clay::id(id, 1),
            crate::typography::Type::MenuLabel,
            text,
            muted,
            false,
        );
    });
}

/// The segmented light/dark/system control: three 92 px cells in a 36 px
/// bordered row, dividers as left borders on the second and third.
fn segmented(app: &mut App, theme: &ThemeState) {
    let page = crate::ui::current_theme(app);
    let ink = |token| crate::ui::ink(page, token);
    let mut grid = crate::ui::empty_decl();
    grid.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
        height: mv_clay::sizing_fixed(layout::SEG_HEIGHT),
    };
    grid.backgroundColor = ink(mv_theme::Token::Card);
    grid.border = clay::BorderElementConfig {
        color: ink(mv_theme::Token::Border),
        width: mv_clay::border_outside(1),
    };
    grid.cornerRadius = crate::ui::corner_radius_all(layout::MENU_RADIUS);
    grid.layout.padding = bordered(0.0);
    crate::ui::ui_scope(app, mv_clay::id("MenuSegmented", 0), &grid, |app| {
        let checked = checked_cell(theme);
        for (i, option) in LIGHT_DARK_ORDER.iter().enumerate() {
            cell(app, *option, i, i == checked);
        }
    });
}

/// One segmented cell: the 16 px option icon over the label, centred.
fn cell(app: &mut App, option: LightDark, index: usize, checked: bool) {
    let page = crate::ui::current_theme(app);
    let ink = |token| crate::ui::ink(page, token);
    let hovered = app.input.hover
        == (Hit {
            kind: HitKind::SettingsLightDark,
            index: index as i64,
        });
    let mut decl = crate::ui::empty_decl();
    decl.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_fixed(layout::CELL_WIDTH),
        height: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
    };
    decl.layout.padding = clay::Padding {
        left: layout::CELL_PAD_X as u16,
        right: layout::CELL_PAD_X as u16,
        top: 0,
        bottom: 0,
    };
    decl.layout.childGap = layout::CELL_GAP as u16;
    decl.layout.childAlignment = clay::ChildAlignment {
        x: clay::LayoutAlignmentX::Center,
        y: clay::LayoutAlignmentY::Center,
    };
    decl.backgroundColor = if checked {
        ink(mv_theme::Token::Primary)
    } else if hovered {
        ink(mv_theme::Token::Secondary)
    } else {
        ink(mv_theme::Token::Card)
    };
    if index > 0 {
        decl.border = clay::BorderElementConfig {
            color: ink(mv_theme::Token::Border),
            width: clay::BorderWidth {
                left: 1,
                right: 0,
                top: 0,
                bottom: 0,
                betweenChildren: 0,
            },
        };
    }
    if checked {
        // The control clips to its radius; the end cells keep the outer corners.
        let (left, right) = match index {
            0 => (layout::CELL_RADIUS, 0.0),
            2 => (0.0, layout::CELL_RADIUS),
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
    let kind = cell_icon(option);
    crate::ui::ui_scope(app, mv_clay::id("MenuCell", index as u32), &decl, |app| {
        icon(
            app,
            mv_clay::id("MenuCellIcon", index as u32),
            kind,
            layout::CELL_ICON,
            foreground,
        );
        crate::typography::text(
            app,
            mv_clay::id("MenuCellText", index as u32),
            crate::typography::Type::OptionText,
            option.name(),
            foreground,
            false,
        );
    });
    // Pointer on the cells (fix-cursor): claude0's consistency call in the
    // user's place (2026-10-20, overturnable) — the same kind of control as
    // the selector cells, which the user wants with the pointer. The
    // reference measures default in all three engines (its labels carry no
    // cursor rule), so matching it is a two-line revert; see decisions.md.
    crate::input::region(
        app,
        mv_clay::id("MenuCell", index as u32),
        Hit {
            kind: HitKind::SettingsLightDark,
            index: index as i64,
        },
        Cursor::Pointer,
        Cursor::Pointer,
        checked,
        crate::ui::corner_radius_all(0.0),
    );
}

/// The selection message under the appearance section.
fn message(app: &mut App, theme: &ThemeState) {
    // Scoped: the borrow ends before the layout takes `app` mutably.
    let muted = {
        let page = crate::ui::current_theme(app);
        crate::ui::ink(page, mv_theme::Token::ForegroundMuted)
    };
    let text = light_dark_message(theme.light_dark, theme.system);
    let mut wrap = crate::ui::empty_decl();
    wrap.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
        height: mv_clay::sizing_fit(crate::ui::ZERO_MINMAX),
    };
    wrap.layout.padding = clay::Padding {
        left: layout::ROW_PAD_X as u16,
        right: layout::ROW_PAD_X as u16,
        top: 0,
        bottom: 0,
    };
    crate::ui::ui_scope(app, mv_clay::id("MenuMessage", 0), &wrap, |app| {
        crate::typography::text(
            app,
            mv_clay::id("MenuMessage", 1),
            crate::typography::Type::MenuHelp,
            &text,
            muted,
            false,
        );
    });
}

/// The theme list: the scheme legend over the three rows.
fn theme_list(app: &mut App, theme: &ThemeState, scheme: SystemScheme, slugs: &[ThemeSlug; 3]) {
    let mut list = crate::ui::empty_decl();
    list.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
        height: mv_clay::sizing_fit(crate::ui::ZERO_MINMAX),
    };
    list.layout.layoutDirection = clay::LayoutDirection::TopToBottom;
    crate::ui::ui_scope(app, mv_clay::id("MenuThemeList", 0), &list, |app| {
        let title = match scheme {
            SystemScheme::Dark => "DARK THEME",
            SystemScheme::Light => "LIGHT THEME",
        };
        legend(app, "MenuThemeTitle", title);
        let resolved = theme.resolved();
        for (i, slug) in slugs.iter().enumerate() {
            row(app, *slug, i, *slug == resolved);
        }
    });
}

/// One theme row: the swatch, the label, a spacer, and the check when selected.
fn row(app: &mut App, slug: ThemeSlug, index: usize, selected: bool) {
    // Borrowed (never cloned): `Theme` cannot move out of the `EnumMap`
    // index, so both themes stay borrows, and the two inks the post-pass
    // needs are hoisted as owned colours before the layout takes `app`.
    let page = crate::ui::current_theme(app);
    let row_theme = &app.themes[port_slug(slug)];
    let hovered = app.input.hover
        == (Hit {
            kind: HitKind::SettingsTheme,
            index: index as i64,
        });
    let mut decl = crate::ui::empty_decl();
    decl.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
        height: mv_clay::sizing_fixed(layout::ROW_HEIGHT),
    };
    decl.layout.padding = clay::Padding {
        left: layout::ROW_PAD_X as u16,
        right: layout::ROW_PAD_X as u16,
        top: 0,
        bottom: 0,
    };
    decl.layout.childGap = layout::ROW_GAP as u16;
    decl.layout.childAlignment = clay::ChildAlignment {
        x: clay::LayoutAlignmentX::Left,
        y: clay::LayoutAlignmentY::Center,
    };
    // The checked row has no background of its own (only weight and check);
    // hover paints the row's own secondary whether selected or not, as the
    // theme class on the row resolves it (`:hover` has no `:not(.selected)`).
    decl.backgroundColor = if hovered {
        crate::ui::ink(row_theme, mv_theme::Token::Secondary)
    } else {
        crate::ui::ink(page, mv_theme::Token::Card)
    };
    let foreground = crate::ui::ink(page, mv_theme::Token::Foreground);
    let check_ink = crate::ui::ink(row_theme, mv_theme::Token::Primary);
    crate::ui::ui_scope(app, mv_clay::id("MenuRow", index as u32), &decl, |app| {
        // The swatch box: painted by the post-pass from this element's data.
        let mut swatch_decl = crate::ui::empty_decl();
        swatch_decl.layout.sizing = clay::Sizing {
            width: mv_clay::sizing_fixed(swatch::WIDTH as f32),
            height: mv_clay::sizing_fixed(swatch::HEIGHT as f32),
        };
        crate::ui::ui_scope(
            app,
            mv_clay::id("MenuSwatch", index as u32),
            &swatch_decl,
            |_| {},
        );
        crate::typography::text(
            app,
            mv_clay::id("MenuRowText", index as u32),
            if selected {
                crate::typography::Type::StrongBody
            } else {
                crate::typography::Type::Body
            },
            slug.name(),
            foreground,
            false,
        );
        // The spacer the check's `margin-left: auto` absorbs.
        let mut spacer = crate::ui::empty_decl();
        spacer.layout.sizing = clay::Sizing {
            width: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
            height: mv_clay::sizing_fit(crate::ui::ZERO_MINMAX),
        };
        crate::ui::ui_scope(
            app,
            mv_clay::id("MenuRowSpacer", index as u32),
            &spacer,
            |_| {},
        );
        if selected {
            let mut check = crate::ui::empty_decl();
            check.layout.sizing = clay::Sizing {
                width: mv_clay::sizing_fixed(layout::CHECK),
                height: mv_clay::sizing_fit(crate::ui::ZERO_MINMAX),
            };
            check.layout.padding = clay::Padding {
                left: 0,
                right: 0,
                top: layout::CHECK_TOP as u16,
                bottom: 0,
            };
            crate::ui::ui_scope(app, mv_clay::id("MenuCheck", index as u32), &check, |app| {
                icon(
                    app,
                    mv_clay::id("MenuCheckIcon", index as u32),
                    crate::icons::IconKind::Check,
                    layout::CHECK,
                    check_ink,
                );
            });
        }
    });
    // Pointer on the rows (fix-cursor): as the cells above, claude0's
    // consistency call (overturnable); the reference measures default.
    // See decisions.md.
    crate::input::region(
        app,
        mv_clay::id("MenuRow", index as u32),
        Hit {
            kind: HitKind::SettingsTheme,
            index: index as i64,
        },
        Cursor::Pointer,
        Cursor::Pointer,
        selected,
        crate::ui::corner_radius_all(0.0),
    );
}

/// Declares a square icon element painting `kind` in `ink`.
/// Declares a square icon element painting `kind` in `ink`. The port's icons
/// take 0-to-1 colours, so the 0-to-255 Clay ink converts at the boundary.
fn icon(
    app: &mut App,
    id: clay::ElementId,
    kind: crate::icons::IconKind,
    size: f32,
    ink: clay::Color,
) {
    let mut decl = crate::ui::empty_decl();
    decl.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_fixed(size),
        height: mv_clay::sizing_fixed(size),
    };
    decl.custom = crate::icons::custom_data(crate::icons::Icon {
        kind,
        ink: crate::draw::nvg_color(ink),
    });
    crate::ui::ui_scope(app, id, &decl, |_| {});
}

/// Paints the gear's hover border and focus ring over the frame: the button
/// keeps its resting border in the layout, this post-pass strokes the primary
/// border while hovered or pressed and the 3 px primary ring while keyboard
/// focused. The `:active` scale is skipped (no rest state shows it).
pub fn paint_chrome(app: &mut App) {
    let data = app
        .clay
        .as_ref()
        .expect("clay context initialised")
        .get_element_data(mv_clay::id(crate::top_bar::GEAR_ID, 0));
    if !data.found {
        return;
    }
    // Scoped: the borrow ends before the paint takes `app.vg` mutably.
    // The ring is the primary at 32% (the rows' `theme::ring` shape).
    let (primary, ring) = {
        let page = crate::ui::current_theme(app);
        let primary = crate::ui::ink(page, mv_theme::Token::Primary);
        let ring = [primary[0], primary[1], primary[2], primary[3] * 0.32];
        (primary, ring)
    };
    let b = data.boundingBox;
    let density = density_of(app);
    let hovered = matches!(app.input.hover.kind, HitKind::Gear)
        || matches!(app.input.pressed.kind, HitKind::Gear);
    if hovered {
        app.vg.begin_path();
        crate::draw::rounded_rect(
            &mut app.vg,
            crate::draw::snap_box(b, density),
            crate::ui::corner_radius_all(crate::top_bar::GEAR_SIZE / 2.0),
        );
        app.vg.stroke_color(crate::draw::nvg_color(primary));
        app.vg.stroke_width(1.0);
        app.vg.stroke();
    }
    if app.input.focus.kind == HitKind::Gear && app.input.focus_visible {
        let at = clay::BoundingBox {
            x: b.x - 1.5,
            y: b.y - 1.5,
            width: b.width + 3.0,
            height: b.height + 3.0,
        };
        app.vg.begin_path();
        crate::draw::rounded_rect(
            &mut app.vg,
            crate::draw::snap_box(at, density),
            crate::ui::corner_radius_all(crate::top_bar::GEAR_SIZE / 2.0 + 1.5),
        );
        app.vg.stroke_color(crate::draw::nvg_color(ring));
        app.vg.stroke_width(3.0);
        app.vg.stroke();
    }
}

fn density_of(app: &App) -> f32 {
    if app.size[0] > 0.0 {
        app.pixels[0] as f32 / app.size[0]
    } else {
        1.0
    }
}

/// Routes a pointer press: an outside pointer-down light-dismisses the open
/// menu, as the popover does. The gear and the menu's own hits stay for the
/// click on release.
pub fn press_down(app: &mut App) {
    if !app.settings.open {
        return;
    }
    match app.input.pressed.kind {
        HitKind::Gear | HitKind::SettingsLightDark | HitKind::SettingsTheme => {}
        // An outside press closes the menu (the reference closes on outside
        // click; the down half lands here, the up half in `click`): the
        // plane's canvas and button are both outside it, as are h3's picker,
        // h4-readout's readout terms and h4-rows' transform rows (with
        // h5-buttons' flip, reset and Reset all on them).
        HitKind::None
        | HitKind::ReturnToCenter
        | HitKind::Plane
        | HitKind::Selector
        | HitKind::ReadoutTerm
        | HitKind::TransformRow
        | HitKind::TransformField
        | HitKind::TransformFlip
        | HitKind::TransformReset
        | HitKind::TransformResetAll
        | HitKind::Wordmark
        | HitKind::WelcomeCta
        | HitKind::WelcomeHero
        | HitKind::PointsField
        | HitKind::PointSlider => {
            close_menu(app);
        }
    }
}

/// Routes a click: the release whose down-hit and up-hit agree. The gear
/// toggles the menu; a cell or row picks through the `App` setters (which
/// save and start the transition); anything else is no change. A pick drops
/// hover, as the reference's re-render does: the view transition replaces
/// the hovered nodes, and Chromium does not re-hover without a mouse move.
pub fn click(app: &mut App, down: Hit) {
    let up = app.input.hover;
    if down != up {
        return;
    }
    match down.kind {
        HitKind::Gear => {
            app.settings.toggle();
            app.input.focus = Hit::default();
            app.dirty = true;
        }
        // The wordmark link to the welcome page (h10-page; the root page is
        // now a real route, plan §3.7 Routes): the press started and the
        // release lands on it, as a button's `onClick` needs both, and the
        // page navigates home with an empty query, as the anchor does.
        HitKind::Wordmark => {
            crate::welcome::navigate(app, "", "");
        }
        HitKind::SettingsLightDark if (down.index as usize) < LIGHT_DARK_ORDER.len() => {
            let option = LIGHT_DARK_ORDER[down.index as usize];
            let changed = option != app.theme.light_dark;
            app.set_light_dark(option);
            app.input.focus = down;
            app.input.focus_visible = false;
            if changed {
                drop_hover(app);
            }
        }
        HitKind::SettingsTheme => {
            let scheme = app.theme.resolved_scheme();
            if let Some(slug) = shown_slugs(scheme).get(down.index as usize) {
                let before = app.theme.resolved();
                app.set_theme_slug(*slug);
                app.input.focus = down;
                app.input.focus_visible = false;
                if app.theme.resolved() != before {
                    drop_hover(app);
                }
            }
        }
        _ => {}
    }
}

/// Routes a key. Tab walks the menu's stops while open (the DOM's one stop
/// per radio group) and falls through to the page milestones closed (they
/// walk their own stops); Esc closes the open
/// menu; the arrows move and check within the focused group; Space and Enter
/// toggle on the gear and confirm in the menu (already picked, so no change).
/// Returns whether the key was consumed.
pub fn key(app: &mut App, event: &crate::input::KeyEvent) -> bool {
    use sdl3::keyboard::Keycode;
    if !event.down || event.repeat {
        return false;
    }
    let shift = event
        .mods
        .intersects(sdl3::keyboard::Mod::LSHIFTMOD | sdl3::keyboard::Mod::RSHIFTMOD);
    match event.key {
        Keycode::Escape => {
            if app.settings.open {
                close_menu(app);
                return true;
            }
            false
        }
        Keycode::Tab => {
            if app.settings.open {
                let checked_row = checked_row(&app.theme);
                let current = stop_of(app.input.focus);
                let next = if shift {
                    app.settings
                        .shift_tab(current, app.theme.light_dark, checked_row)
                } else {
                    app.settings.tab(current, app.theme.light_dark, checked_row)
                };
                match next {
                    None => crate::input::set_focus(app, Hit::default(), false),
                    Some(FocusSlot::LightDark(option)) => {
                        let index = LIGHT_DARK_ORDER
                            .iter()
                            .position(|choice| *choice == option)
                            .expect("focus names a cell");
                        crate::input::set_focus(
                            app,
                            Hit {
                                kind: HitKind::SettingsLightDark,
                                index: index as i64,
                            },
                            true,
                        );
                    }
                    Some(FocusSlot::Theme(index)) => crate::input::set_focus(
                        app,
                        Hit {
                            kind: HitKind::SettingsTheme,
                            index: index as i64,
                        },
                        true,
                    ),
                }
                true
            } else {
                // Closed: Tab falls through to the page milestones, which
                // walk their own stops (the transform rows commit a focused
                // draft and sync the highlight as blur-then-focus does).
                false
            }
        }
        Keycode::Left | Keycode::Right | Keycode::Up | Keycode::Down if app.settings.open => {
            let arrow = match event.key {
                Keycode::Left => Arrow::Left,
                Keycode::Right => Arrow::Right,
                Keycode::Up => Arrow::Up,
                _ => Arrow::Down,
            };
            match stop_of(app.input.focus) {
                Some(FocusSlot::LightDark(option)) => {
                    let at = LIGHT_DARK_ORDER
                        .iter()
                        .position(|choice| *choice == option)
                        .expect("focus names a cell");
                    let next = arrow_index(at, LIGHT_DARK_ORDER.len(), arrow);
                    app.set_light_dark(LIGHT_DARK_ORDER[next]);
                    drop_hover(app);
                    crate::input::set_focus(
                        app,
                        Hit {
                            kind: HitKind::SettingsLightDark,
                            index: next as i64,
                        },
                        true,
                    );
                    true
                }
                Some(FocusSlot::Theme(index)) => {
                    let scheme = app.theme.resolved_scheme();
                    let slugs = shown_slugs(scheme);
                    let next = arrow_index(index, slugs.len(), arrow);
                    app.set_theme_slug(slugs[next]);
                    drop_hover(app);
                    crate::input::set_focus(
                        app,
                        Hit {
                            kind: HitKind::SettingsTheme,
                            index: next as i64,
                        },
                        true,
                    );
                    true
                }
                None => false,
            }
        }
        Keycode::Space | Keycode::Return if app.input.focus.kind == HitKind::Gear => {
            app.settings.toggle();
            app.dirty = true;
            true
        }
        Keycode::Space | Keycode::Return => false,
        _ => false,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme_state::{DarkTheme, LightTheme};

    #[test]
    fn labels_spell_like_slug_to_label() {
        assert_eq!(ThemeSlug::ArcticIce.name(), "Arctic Ice");
        assert_eq!(ThemeSlug::MidnightViolet.name(), "Midnight Violet");
        assert_eq!(ThemeSlug::CleanTeal.name(), "Clean Teal");
        assert_eq!(LightDark::System.name(), "System");
        assert_eq!(LightDark::Dark.name(), "Dark");
        assert_eq!(ThemeSlug::ArcticIce.slug(), "arctic-ice");
        assert_eq!(LightDark::System.word(), "system");
    }

    #[test]
    fn selection_message_names_the_system_scheme() {
        assert_eq!(
            light_dark_message(LightDark::System, SystemScheme::Dark),
            "Your system is dark right now."
        );
        assert_eq!(
            light_dark_message(LightDark::System, SystemScheme::Light),
            "Your system is light right now."
        );
        assert_eq!(
            light_dark_message(LightDark::Dark, SystemScheme::Light),
            "Always dark."
        );
        assert_eq!(
            light_dark_message(LightDark::Light, SystemScheme::Dark),
            "Always light."
        );
    }

    #[test]
    fn system_resolves_to_the_os_scheme() {
        let state = ThemeState {
            light_dark: LightDark::System,
            dark_theme: DarkTheme::ArcticIce,
            light_theme: LightTheme::CleanTeal,
            system: SystemScheme::Dark,
        };
        assert_eq!(state.resolved(), ThemeSlug::ArcticIce);
        let state = ThemeState {
            system: SystemScheme::Light,
            ..state
        };
        assert_eq!(state.resolved(), ThemeSlug::CleanTeal);
        let state = ThemeState {
            light_dark: LightDark::Dark,
            system: SystemScheme::Light,
            ..state
        };
        assert_eq!(state.resolved(), ThemeSlug::ArcticIce);
    }

    #[test]
    fn theme_lists_match_use_app_theme() {
        assert_eq!(
            shown_slugs(SystemScheme::Dark).map(ThemeSlug::slug),
            ["arctic-ice", "tron-cyan", "midnight-violet"]
        );
        assert_eq!(
            shown_slugs(SystemScheme::Light).map(ThemeSlug::slug),
            ["clean-teal", "educational-warm", "sage-editorial"]
        );
    }

    #[test]
    fn theme_picks_write_through_the_resolved_scheme() {
        let mut state = ThemeState {
            light_dark: LightDark::System,
            dark_theme: DarkTheme::ArcticIce,
            light_theme: LightTheme::CleanTeal,
            system: SystemScheme::Dark,
        };
        state.set_theme_slug(ThemeSlug::MidnightViolet);
        assert_eq!(state.dark_theme, DarkTheme::MidnightViolet);
        assert_eq!(state.light_theme, LightTheme::CleanTeal);
        state.system = SystemScheme::Light;
        state.set_theme_slug(ThemeSlug::SageEditorial);
        assert_eq!(state.light_theme, LightTheme::SageEditorial);
        assert_eq!(state.dark_theme, DarkTheme::MidnightViolet);
        // A slug from the other scheme's list never lands: the port's
        // deliberate hardening (the TS writes it blindly).
        state.set_theme_slug(ThemeSlug::ArcticIce);
        assert_eq!(state.light_theme, LightTheme::SageEditorial);
    }

    #[test]
    fn tab_walks_group_to_group_then_out() {
        let state = SettingsMenuState {
            open: true,
            hover_held: false,
        };
        // Probe: gear, then system (checked), then arctic-ice (checked), then out.
        let first = state.tab(None, LightDark::System, 0);
        assert_eq!(first, Some(FocusSlot::LightDark(LightDark::System)));
        assert_eq!(
            state.tab(first, LightDark::System, 0),
            Some(FocusSlot::Theme(0))
        );
        assert_eq!(
            state.tab(Some(FocusSlot::Theme(0)), LightDark::System, 0),
            None
        );
    }

    #[test]
    fn tab_starts_at_the_checked_options() {
        let state = SettingsMenuState {
            open: true,
            hover_held: false,
        };
        assert_eq!(
            state.tab(None, LightDark::Dark, 2),
            Some(FocusSlot::LightDark(LightDark::Dark))
        );
        assert_eq!(
            state.shift_tab(None, LightDark::Dark, 2),
            Some(FocusSlot::Theme(2))
        );
    }

    #[test]
    fn closed_menu_takes_no_tab_stop() {
        let state = SettingsMenuState::closed();
        assert_eq!(state.tab(None, LightDark::System, 0), None);
        assert_eq!(state.shift_tab(None, LightDark::System, 0), None);
    }

    /// A press outside the open menu light-dismisses it, as the popover
    /// does: the return-to-centre button is outside the menu, so pressing it
    /// closes the menu (its click then proceeds through its own region).
    #[test]
    fn press_on_return_to_centre_closes_the_open_menu() {
        let mut app = crate::fixture::headless_app();
        app.settings.open();
        app.input.pressed = Hit {
            kind: HitKind::ReturnToCenter,
            index: 0,
        };
        press_down(&mut app);
        assert!(!app.settings.open);
    }

    /// The open menu's rows keep their hits where they float over the
    /// plane's canvas: the canvas region (declared after the layout, over
    /// the same pixels) must not shadow them, or a row click
    /// light-dismisses the menu instead of switching the theme (the
    /// H1-H6 union's theme-switch-frames frame 0: menu closed, Arctic
    /// throughout). The Tron Cyan row's centre is where the state scripts
    /// click.
    #[test]
    fn open_menu_rows_win_their_hits_over_the_plane() {
        let _guard = mv_clay::tests_lock();
        let mut app = crate::fixture::headless_app();
        assert!(mv_text::init(&mut app.fonts));
        crate::ui::init_clay(&mut app);
        app.size = [1440.0, 900.0];
        app.pixels = [2880, 1800];
        app.theme.light_dark = LightDark::Dark;
        app.settings.open();
        app.url = crate::url::Url::new("#function-viewer");
        let _commands = crate::page::layout_page(&mut app);
        crate::plane_input::declare_plane_region(&mut app);
        crate::input::collect_regions(&mut app);
        assert!(
            app.input
                .regions
                .iter()
                .any(|r| r.hit.kind == HitKind::Plane),
            "the canvas region overlaps the menu in this setup"
        );
        assert_eq!(
            crate::input::hit_at(&app.input.regions, [1251.0, 262.0]),
            Hit {
                kind: HitKind::SettingsTheme,
                index: 1,
            }
        );
    }

    #[test]
    fn arrows_wrap_round_the_group() {
        assert_eq!(arrow_index(0, 3, Arrow::Right), 1);
        assert_eq!(arrow_index(2, 3, Arrow::Right), 0);
        assert_eq!(arrow_index(0, 3, Arrow::Left), 2);
        assert_eq!(arrow_index(1, 3, Arrow::Up), 0);
        assert_eq!(arrow_index(1, 3, Arrow::Down), 2);
    }

    #[test]
    fn swatch_geometry_matches_theme_swatch() {
        assert_eq!((swatch::WIDTH, swatch::HEIGHT), (30.0, 20.0));
        assert_eq!(swatch::RADIUS, 5.0);
        assert_eq!(swatch::OUTLINE, 1.0);
        assert_eq!(swatch::CURVE, "M6 4.63Q15 25.37 24 4.63");
        assert_eq!(swatch::STROKE_WIDTH, 1.75);
        assert_eq!(swatch::POINT, (21.0, 10.39));
        assert_eq!(swatch::POINT_RADIUS, 2.25);
    }

    /// The layout slack: boxes within half a CSS pixel of the reference's.
    const SLACK: f32 = 0.5;

    /// The open menu's boxes match the reference's, in CSS pixels, in both
    /// schemes: the dark list at `settings-open`, the light list at
    /// `settings-light-dark-system`.
    #[test]
    fn menu_matches_the_reference() {
        let _guard = mv_clay::tests_lock();
        let mut app = crate::fixture::headless_app();
        assert!(mv_text::init(&mut app.fonts));
        crate::ui::init_clay(&mut app);
        app.size = [1440.0, 900.0];
        app.pixels = [2880, 1800];
        app.settings.open();
        app.url = crate::url::Url::new("#function-viewer");
        let _commands = crate::page::layout_page(&mut app);
        let clay = app.clay.as_ref().expect("clay context initialised");
        let fixture = crate::fixture::page_fixture().expect("page fixture parses");
        let shot = crate::fixture::fixture_shot(&fixture, "settings-open")
            .expect("settings-open in fixture");
        for (id, index, name) in [
            ("Menu", 0, "menu"),
            ("MenuCell", 0, "radio_system"),
            ("MenuCell", 1, "radio_dark"),
            ("MenuCell", 2, "radio_light"),
            ("MenuRow", 0, "option_0"),
            ("MenuRow", 1, "option_1"),
            ("MenuRow", 2, "option_2"),
            ("MenuSwatch", 0, "swatch_0"),
        ] {
            let data = clay.get_element_data(mv_clay::id(id, index));
            assert!(data.found, "{name}: no such element");
            let b = data.boundingBox;
            let want = crate::fixture::fixture_box(&shot.webkit, name).expect("box in fixture");
            let wants = if name == "menu" {
                [
                    want[0],
                    want[1],
                    want[2],
                    want[3] - layout::MENU_DEV_LINK_DELTA,
                ]
            } else {
                [want[0], want[1], want[2], want[3]]
            };
            for (i, (got, w)) in [b.x, b.y, b.width, b.height]
                .iter()
                .zip(wants.iter())
                .enumerate()
            {
                assert!(
                    (got - w).abs() <= SLACK,
                    "{name}[{i}]: laid out {got}, reference {w}"
                );
            }
        }
        // The light scheme shows the light list in the same boxes.
        app.theme.system = crate::theme_state::SystemScheme::Light;
        let _commands = crate::page::layout_page(&mut app);
        let clay = app.clay.as_ref().expect("clay context initialised");
        let shot = crate::fixture::fixture_shot(&fixture, "settings-light-dark-system")
            .expect("settings-light-dark-system in fixture");
        for (index, name) in [(0, "option_3"), (1, "option_4"), (2, "option_5")] {
            let data = clay.get_element_data(mv_clay::id("MenuRow", index));
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
