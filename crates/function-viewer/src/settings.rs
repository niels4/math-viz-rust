//! The settings menu: the gear's popover under the top bar with the light/dark/system
//! radios and the two theme lists with swatches (`SettingsMenu.tsx` at `adb0534`).
//!
//! The menu's theme selections live in h1-frame's `ThemeState` (the three persisted
//! picks plus the system scheme); this module owns the menu's own state (open flag
//! and keyboard focus) and the shell-independent menu logic. The `ThemeState` items
//! below are a local stub with h1-frame's names (marked); when h1-frame merges the
//! stub section goes and this module uses `crate::theme_state` instead.

use std::fmt;

// stub until h1-frame: replaced by `crate::theme_state::SystemScheme`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemScheme {
    Dark,
    Light,
}

// stub until h1-frame: replaced by `crate::theme_state::LightDark`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightDark {
    System,
    Dark,
    Light,
}

// stub until h1-frame: replaced by `crate::theme_state::DarkTheme`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DarkTheme {
    ArcticIce,
    TronCyan,
    MidnightViolet,
}

// stub until h1-frame: replaced by `crate::theme_state::LightTheme`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightTheme {
    CleanTeal,
    EducationalWarm,
    SageEditorial,
}

// stub until h1-frame: replaced by `crate::theme_state::ThemeSlug`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeSlug {
    ArcticIce,
    TronCyan,
    MidnightViolet,
    CleanTeal,
    EducationalWarm,
    SageEditorial,
}

impl ThemeSlug {
    // stub until h1-frame.
    pub fn name(self) -> &'static str {
        match self {
            ThemeSlug::ArcticIce => "arctic-ice",
            ThemeSlug::TronCyan => "tron-cyan",
            ThemeSlug::MidnightViolet => "midnight-violet",
            ThemeSlug::CleanTeal => "clean-teal",
            ThemeSlug::EducationalWarm => "educational-warm",
            ThemeSlug::SageEditorial => "sage-editorial",
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

// stub until h1-frame: replaced by `crate::theme_state::ThemeState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThemeState {
    pub light_dark: LightDark,
    pub dark_theme: DarkTheme,
    pub light_theme: LightTheme,
    pub system: SystemScheme,
}

impl ThemeState {
    // stub until h1-frame: the resolved scheme (`appThemeAtom`'s first step).
    pub fn resolved_scheme(self) -> SystemScheme {
        match self.light_dark {
            LightDark::System => self.system,
            LightDark::Dark => SystemScheme::Dark,
            LightDark::Light => SystemScheme::Light,
        }
    }

    // stub until h1-frame: `appThemeAtom`'s resolved theme.
    pub fn resolved(self) -> ThemeSlug {
        match self.resolved_scheme() {
            SystemScheme::Dark => match self.dark_theme {
                DarkTheme::ArcticIce => ThemeSlug::ArcticIce,
                DarkTheme::TronCyan => ThemeSlug::TronCyan,
                DarkTheme::MidnightViolet => ThemeSlug::MidnightViolet,
            },
            SystemScheme::Light => match self.light_theme {
                LightTheme::CleanTeal => ThemeSlug::CleanTeal,
                LightTheme::EducationalWarm => ThemeSlug::EducationalWarm,
                LightTheme::SageEditorial => ThemeSlug::SageEditorial,
            },
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

    /// `setThemeSlug`: the picked slug is written to the dark or the light slot
    /// according to the *resolved* scheme, never both.
    // stub until h1-frame (kept here: the routing is the menu's own rule).
    pub fn set_theme_slug(&mut self, slug: ThemeSlug) {
        match self.resolved_scheme() {
            SystemScheme::Dark => match slug {
                ThemeSlug::ArcticIce => self.dark_theme = DarkTheme::ArcticIce,
                ThemeSlug::TronCyan => self.dark_theme = DarkTheme::TronCyan,
                ThemeSlug::MidnightViolet => self.dark_theme = DarkTheme::MidnightViolet,
                ThemeSlug::CleanTeal | ThemeSlug::EducationalWarm | ThemeSlug::SageEditorial => {}
            },
            SystemScheme::Light => match slug {
                ThemeSlug::CleanTeal => self.light_theme = LightTheme::CleanTeal,
                ThemeSlug::EducationalWarm => {
                    self.light_theme = LightTheme::EducationalWarm;
                }
                ThemeSlug::SageEditorial => self.light_theme = LightTheme::SageEditorial,
                ThemeSlug::ArcticIce | ThemeSlug::TronCyan | ThemeSlug::MidnightViolet => {}
            },
        }
    }
}

/// `slugToLabel` (`format.ts`): `"arctic-ice"` maps to `"Arctic Ice"`.
pub fn slug_to_label(slug: &str) -> String {
    slug.split('-')
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn light_dark_slug(light_dark: LightDark) -> &'static str {
    match light_dark {
        LightDark::System => "system",
        LightDark::Dark => "dark",
        LightDark::Light => "light",
    }
}

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
        option => format!("Always {}.", light_dark_slug(option)),
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
            FocusSlot::LightDark(option) => write!(f, "light-dark:{}", light_dark_slug(*option)),
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
    fn slug_to_label_capitalises_each_word() {
        assert_eq!(slug_to_label("arctic-ice"), "Arctic Ice");
        assert_eq!(slug_to_label("midnight-violet"), "Midnight Violet");
        assert_eq!(slug_to_label("clean-teal"), "Clean Teal");
        assert_eq!(slug_to_label("system"), "System");
        assert_eq!(slug_to_label("dark"), "Dark");
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
            ThemeSlug::slugs_for(SystemScheme::Dark).map(ThemeSlug::name),
            ["arctic-ice", "tron-cyan", "midnight-violet"]
        );
        assert_eq!(
            ThemeSlug::slugs_for(SystemScheme::Light).map(ThemeSlug::name),
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
