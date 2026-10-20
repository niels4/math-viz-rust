//! The settings menu: the gear's popover under the top bar with the light/dark/system
//! radios and the two theme lists with swatches (`SettingsMenu.tsx` at `adb0534`).
//!
//! The menu's theme selections live in h1-frame's `ThemeState` (the three persisted
//! picks plus the system scheme); this module owns the menu's own state (open flag
//! and keyboard focus) and the shell-independent menu logic. The `ThemeState` items
//! below are a local stub with h1-frame's names (marked); when h1-frame merges the
//! stub section goes and this module uses `crate::theme_state` instead.

use std::fmt;

// The stub section: h1-frame's `theme_state` items mirrored exactly (commit
// 1fda916 on claude-port-h1-frame, read-only). When h1-frame merges into
// `claude-port`, this section is deleted and the `use` below it takes over.
// Every name, default, word, slug, label and routing rule matches.

// stub until h1-frame: replaced by `crate::theme_state::SystemScheme`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SystemScheme {
    #[default]
    Dark,
    Light,
}

// stub until h1-frame: replaced by `crate::theme_state::LightDark`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LightDark {
    #[default]
    System,
    Dark,
    Light,
}

// stub until h1-frame: replaced by `crate::theme_state::DarkTheme`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DarkTheme {
    #[default]
    ArcticIce,
    TronCyan,
    MidnightViolet,
}

// stub until h1-frame: replaced by `crate::theme_state::LightTheme`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LightTheme {
    #[default]
    CleanTeal,
    EducationalWarm,
    SageEditorial,
}

// stub until h1-frame: replaced by `crate::theme_state::ThemeSlug`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ThemeSlug {
    #[default]
    ArcticIce,
    TronCyan,
    MidnightViolet,
    CleanTeal,
    EducationalWarm,
    SageEditorial,
}

// stub until h1-frame: replaced by `crate::theme_state::ALL_SLUGS`.
pub const ALL_SLUGS: [ThemeSlug; 6] = [
    ThemeSlug::ArcticIce,
    ThemeSlug::TronCyan,
    ThemeSlug::MidnightViolet,
    ThemeSlug::CleanTeal,
    ThemeSlug::EducationalWarm,
    ThemeSlug::SageEditorial,
];

impl LightDark {
    // stub until h1-frame.
    pub fn word(self) -> &'static str {
        match self {
            LightDark::System => "system",
            LightDark::Dark => "dark",
            LightDark::Light => "light",
        }
    }

    // stub until h1-frame.
    pub fn name(self) -> &'static str {
        match self {
            LightDark::System => "System",
            LightDark::Dark => "Dark",
            LightDark::Light => "Light",
        }
    }

    // stub until h1-frame.
    pub fn parse(text: &str) -> Option<LightDark> {
        match text {
            "system" => Some(LightDark::System),
            "dark" => Some(LightDark::Dark),
            "light" => Some(LightDark::Light),
            _ => None,
        }
    }
}

impl DarkTheme {
    // stub until h1-frame.
    pub fn slug(self) -> &'static str {
        ThemeSlug::from(self).slug()
    }

    // stub until h1-frame.
    pub fn name(self) -> &'static str {
        ThemeSlug::from(self).name()
    }

    // stub until h1-frame.
    pub fn parse(text: &str) -> Option<DarkTheme> {
        match text {
            "arctic-ice" => Some(DarkTheme::ArcticIce),
            "tron-cyan" => Some(DarkTheme::TronCyan),
            "midnight-violet" => Some(DarkTheme::MidnightViolet),
            _ => None,
        }
    }
}

impl LightTheme {
    // stub until h1-frame.
    pub fn slug(self) -> &'static str {
        ThemeSlug::from(self).slug()
    }

    // stub until h1-frame.
    pub fn name(self) -> &'static str {
        ThemeSlug::from(self).name()
    }

    // stub until h1-frame.
    pub fn parse(text: &str) -> Option<LightTheme> {
        match text {
            "clean-teal" => Some(LightTheme::CleanTeal),
            "educational-warm" => Some(LightTheme::EducationalWarm),
            "sage-editorial" => Some(LightTheme::SageEditorial),
            _ => None,
        }
    }
}

impl ThemeSlug {
    // stub until h1-frame.
    pub fn slug(self) -> &'static str {
        match self {
            ThemeSlug::ArcticIce => "arctic-ice",
            ThemeSlug::TronCyan => "tron-cyan",
            ThemeSlug::MidnightViolet => "midnight-violet",
            ThemeSlug::CleanTeal => "clean-teal",
            ThemeSlug::EducationalWarm => "educational-warm",
            ThemeSlug::SageEditorial => "sage-editorial",
        }
    }

    // stub until h1-frame.
    pub fn name(self) -> &'static str {
        match self {
            ThemeSlug::ArcticIce => "Arctic Ice",
            ThemeSlug::TronCyan => "Tron Cyan",
            ThemeSlug::MidnightViolet => "Midnight Violet",
            ThemeSlug::CleanTeal => "Clean Teal",
            ThemeSlug::EducationalWarm => "Educational Warm",
            ThemeSlug::SageEditorial => "Sage Editorial",
        }
    }

    // stub until h1-frame.
    pub fn parse(text: &str) -> Option<ThemeSlug> {
        match text {
            "arctic-ice" => Some(ThemeSlug::ArcticIce),
            "tron-cyan" => Some(ThemeSlug::TronCyan),
            "midnight-violet" => Some(ThemeSlug::MidnightViolet),
            "clean-teal" => Some(ThemeSlug::CleanTeal),
            "educational-warm" => Some(ThemeSlug::EducationalWarm),
            "sage-editorial" => Some(ThemeSlug::SageEditorial),
            _ => None,
        }
    }

    // stub until h1-frame.
    pub fn scheme(self) -> SystemScheme {
        match self {
            ThemeSlug::ArcticIce | ThemeSlug::TronCyan | ThemeSlug::MidnightViolet => {
                SystemScheme::Dark
            }
            ThemeSlug::CleanTeal | ThemeSlug::EducationalWarm | ThemeSlug::SageEditorial => {
                SystemScheme::Light
            }
        }
    }

    // stub until h1-frame.
    pub fn dark(self) -> Option<DarkTheme> {
        match self {
            ThemeSlug::ArcticIce => Some(DarkTheme::ArcticIce),
            ThemeSlug::TronCyan => Some(DarkTheme::TronCyan),
            ThemeSlug::MidnightViolet => Some(DarkTheme::MidnightViolet),
            ThemeSlug::CleanTeal | ThemeSlug::EducationalWarm | ThemeSlug::SageEditorial => None,
        }
    }

    // stub until h1-frame.
    pub fn light(self) -> Option<LightTheme> {
        match self {
            ThemeSlug::ArcticIce | ThemeSlug::TronCyan | ThemeSlug::MidnightViolet => None,
            ThemeSlug::CleanTeal => Some(LightTheme::CleanTeal),
            ThemeSlug::EducationalWarm => Some(LightTheme::EducationalWarm),
            ThemeSlug::SageEditorial => Some(LightTheme::SageEditorial),
        }
    }

    // stub until h1-frame: the dark list is `DARK_THEME_SLUGS`, the light list
    // `LIGHT_THEME_SLUGS` in `useAppTheme.ts`.
    pub fn slugs_for(scheme: SystemScheme) -> [ThemeSlug; 3] {
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
}

impl From<DarkTheme> for ThemeSlug {
    // stub until h1-frame.
    fn from(dark: DarkTheme) -> ThemeSlug {
        match dark {
            DarkTheme::ArcticIce => ThemeSlug::ArcticIce,
            DarkTheme::TronCyan => ThemeSlug::TronCyan,
            DarkTheme::MidnightViolet => ThemeSlug::MidnightViolet,
        }
    }
}

impl From<LightTheme> for ThemeSlug {
    // stub until h1-frame.
    fn from(light: LightTheme) -> ThemeSlug {
        match light {
            LightTheme::CleanTeal => ThemeSlug::CleanTeal,
            LightTheme::EducationalWarm => ThemeSlug::EducationalWarm,
            LightTheme::SageEditorial => ThemeSlug::SageEditorial,
        }
    }
}

// stub until h1-frame: replaced by `crate::theme_state::ThemeState`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThemeState {
    pub light_dark: LightDark,
    pub dark_theme: DarkTheme,
    pub light_theme: LightTheme,
    pub system: SystemScheme,
}

impl Default for ThemeState {
    // stub until h1-frame.
    fn default() -> ThemeState {
        ThemeState {
            light_dark: LightDark::System,
            dark_theme: DarkTheme::ArcticIce,
            light_theme: LightTheme::CleanTeal,
            system: SystemScheme::Dark,
        }
    }
}

impl ThemeState {
    // stub until h1-frame.
    pub fn resolved_scheme(&self) -> SystemScheme {
        match self.light_dark {
            LightDark::System => self.system,
            LightDark::Dark => SystemScheme::Dark,
            LightDark::Light => SystemScheme::Light,
        }
    }

    // stub until h1-frame.
    pub fn resolved(&self) -> ThemeSlug {
        if self.resolved_scheme() == SystemScheme::Dark {
            ThemeSlug::from(self.dark_theme)
        } else {
            ThemeSlug::from(self.light_theme)
        }
    }

    // stub until h1-frame.
    pub fn set_theme_slug(&mut self, slug: ThemeSlug) {
        if self.resolved_scheme() == SystemScheme::Dark {
            if let Some(dark) = slug.dark() {
                self.dark_theme = dark;
            }
        } else if let Some(light) = slug.light() {
            self.light_theme = light;
        }
    }

    // stub until h1-frame.
    pub fn set_light_dark(&mut self, value: LightDark) {
        self.light_dark = value;
    }

    // stub until h1-frame.
    pub fn set_dark_theme(&mut self, value: DarkTheme) {
        self.dark_theme = value;
    }

    // stub until h1-frame.
    pub fn set_light_theme(&mut self, value: LightTheme) {
        self.light_theme = value;
    }
}

/// The three colours the menu's swatch shows for a theme: its background,
/// its curve ink, its point ink.
// stub until h1-frame: replaced by `crate::theme_state::swatch_colours`.
pub fn swatch_colours(_slug: ThemeSlug) -> [mv_theme::Token; 3] {
    use mv_theme::Token::{Background, ChartAccent, ChartLine};
    [Background, ChartLine, ChartAccent]
}

/// `slugToLabel` (`format.ts`) lives on h1-frame's `name()` methods
/// (`LightDark::name`, `ThemeSlug::name`); this module spells labels through them.
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
    pub const CURVE_START: (f64, f64) = (6.0, 4.63);
    pub const CURVE_CONTROL: (f64, f64) = (15.0, 25.37);
    pub const CURVE_END: (f64, f64) = (24.0, 4.63);
    pub const STROKE_WIDTH: f64 = 1.75;
    pub const POINT: (f64, f64) = (21.0, 10.39);
    pub const POINT_RADIUS: f64 = 2.25;
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

/// The menu's interactive state: the popover's open flag plus where keyboard
/// focus sits (`None` means focus is on the gear button or outside the menu).
/// Tab/Shift+Tab walk the group stops; Esc or an outside click closes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SettingsMenuState {
    pub open: bool,
    pub focus: Option<FocusSlot>,
}

impl SettingsMenuState {
    pub fn closed() -> Self {
        Self {
            open: false,
            focus: None,
        }
    }

    pub fn open(&mut self) {
        self.open = true;
        self.focus = None;
    }

    pub fn close(&mut self) {
        self.open = false;
        self.focus = None;
    }

    /// Tab from the current stop: gear, then the checked light-dark cell, then
    /// the checked theme row, then out. The menu stays open throughout.
    pub fn tab(&self, light_dark: LightDark, theme_index: usize) -> Option<FocusSlot> {
        if !self.open {
            return None;
        }
        match self.focus {
            None => Some(FocusSlot::LightDark(light_dark)),
            Some(FocusSlot::LightDark(_)) => Some(FocusSlot::Theme(theme_index)),
            Some(FocusSlot::Theme(_)) => None,
        }
    }

    /// Shift+Tab walks the stops backwards.
    pub fn shift_tab(&self, light_dark: LightDark, theme_index: usize) -> Option<FocusSlot> {
        if !self.open {
            return None;
        }
        match self.focus {
            None => Some(FocusSlot::Theme(theme_index)),
            Some(FocusSlot::Theme(_)) => Some(FocusSlot::LightDark(light_dark)),
            Some(FocusSlot::LightDark(_)) => None,
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

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
            ThemeSlug::slugs_for(SystemScheme::Dark).map(ThemeSlug::slug),
            ["arctic-ice", "tron-cyan", "midnight-violet"]
        );
        assert_eq!(
            ThemeSlug::slugs_for(SystemScheme::Light).map(ThemeSlug::slug),
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
        // A slug from the other scheme's list never lands.
        state.set_theme_slug(ThemeSlug::ArcticIce);
        assert_eq!(state.light_theme, LightTheme::SageEditorial);
    }

    #[test]
    fn tab_walks_group_to_group_then_out() {
        let state = SettingsMenuState {
            open: true,
            focus: None,
        };
        // Probe: gear, then system (checked), then arctic-ice (checked), then out.
        let first = state.tab(LightDark::System, 0);
        assert_eq!(first, Some(FocusSlot::LightDark(LightDark::System)));
        let state = SettingsMenuState {
            open: true,
            focus: first,
        };
        assert_eq!(state.tab(LightDark::System, 0), Some(FocusSlot::Theme(0)));
        let state = SettingsMenuState {
            open: true,
            focus: Some(FocusSlot::Theme(0)),
        };
        assert_eq!(state.tab(LightDark::System, 0), None);
    }

    #[test]
    fn tab_starts_at_the_checked_options() {
        let state = SettingsMenuState {
            open: true,
            focus: None,
        };
        assert_eq!(
            state.tab(LightDark::Dark, 2),
            Some(FocusSlot::LightDark(LightDark::Dark))
        );
        assert_eq!(
            state.shift_tab(LightDark::Dark, 2),
            Some(FocusSlot::Theme(2))
        );
    }

    #[test]
    fn closed_menu_takes_no_tab_stop() {
        let state = SettingsMenuState::closed();
        assert_eq!(state.tab(LightDark::System, 0), None);
        assert_eq!(state.shift_tab(LightDark::System, 0), None);
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
        assert_eq!(swatch::CURVE_START, (6.0, 4.63));
        assert_eq!(swatch::CURVE_CONTROL, (15.0, 25.37));
        assert_eq!(swatch::CURVE_END, (24.0, 4.63));
        assert_eq!(swatch::STROKE_WIDTH, 1.75);
        assert_eq!(swatch::POINT, (21.0, 10.39));
        assert_eq!(swatch::POINT_RADIUS, 2.25);
    }
}
