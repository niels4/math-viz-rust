//! The theme state: math-viz2's `useAppTheme` atoms as one struct.
//!
//! Ported from `src/state/useAppTheme.ts` at `adb0534`: the three persisted
//! picks (light/dark selector, one dark theme, one light theme), the system
//! scheme tracked from the platform, and the resolved theme derived from
//! them. math-viz2 keeps the picks in three `atomWithStorage` atoms
//! (`mathviz-light-dark`, `mathviz-dark-theme`, `mathviz-light-theme`); the
//! port keeps them in one struct on `App`, stored under its own keys
//! (`prefs.rs`).

use crate::app::App;

/// Dark or light, as the system reports it.
///
/// Ported from `useAppTheme.ts` (`SystemColorScheme = "dark" | "light"`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SystemScheme {
    #[default]
    Dark,
    Light,
}

/// Which themes are in play: the system's, or always one scheme's.
///
/// Ported from `useAppTheme.ts` (`LightDarkOptionSlug`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LightDark {
    #[default]
    System,
    Dark,
    Light,
}

/// The three dark themes, in the settings menu's order.
///
/// Ported from `useAppTheme.ts` (`DARK_THEME_SLUGS`, `DarkThemeSlug`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DarkTheme {
    #[default]
    ArcticIce,
    TronCyan,
    MidnightViolet,
}

/// The three light themes, in the settings menu's order.
///
/// Ported from `useAppTheme.ts` (`LIGHT_THEME_SLUGS`, `LightThemeSlug`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LightTheme {
    #[default]
    CleanTeal,
    EducationalWarm,
    SageEditorial,
}

/// The six themes.
///
/// Ported from `useAppTheme.ts` (`ThemeSlug`).
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

/// The six themes in slug order.
pub const ALL_SLUGS: [ThemeSlug; 6] = [
    ThemeSlug::ArcticIce,
    ThemeSlug::TronCyan,
    ThemeSlug::MidnightViolet,
    ThemeSlug::CleanTeal,
    ThemeSlug::EducationalWarm,
    ThemeSlug::SageEditorial,
];

impl LightDark {
    /// The stored word, as `atomWithStorage` writes it.
    pub fn word(self) -> &'static str {
        match self {
            LightDark::System => "system",
            LightDark::Dark => "dark",
            LightDark::Light => "light",
        }
    }

    /// The menu's label, as `slugToLabel` spells it.
    ///
    /// Ported from `format.ts` (`slugToLabel`: dash-split, capitalised).
    pub fn name(self) -> &'static str {
        match self {
            LightDark::System => "System",
            LightDark::Dark => "Dark",
            LightDark::Light => "Light",
        }
    }

    /// Parses a stored word; `None` falls back to the default, as
    /// `storageFor` falls back to the initial value.
    ///
    /// Ported from `useAppTheme.ts` (`isLightDarkOptionSlug`).
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
    /// The stored slug.
    pub fn slug(self) -> &'static str {
        ThemeSlug::from(self).slug()
    }

    /// The menu's label, as `slugToLabel` spells it.
    pub fn name(self) -> &'static str {
        ThemeSlug::from(self).name()
    }

    /// Parses a stored slug; a light slug is not a dark pick, as
    /// `isDarkThemeSlug` says.
    ///
    /// Ported from `useAppTheme.ts` (`isDarkThemeSlug`).
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
    /// The stored slug.
    pub fn slug(self) -> &'static str {
        ThemeSlug::from(self).slug()
    }

    /// The menu's label, as `slugToLabel` spells it.
    pub fn name(self) -> &'static str {
        ThemeSlug::from(self).name()
    }

    /// Parses a stored slug; a dark slug is not a light pick, as
    /// `isLightThemeSlug` says.
    ///
    /// Ported from `useAppTheme.ts` (`isLightThemeSlug`).
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
    /// The stored slug.
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

    /// The menu's label, as `slugToLabel` spells it.
    ///
    /// Ported from `format.ts` (`slugToLabel("arctic-ice")` is "Arctic Ice").
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

    /// Parses a stored slug; `None` falls back to the default, as
    /// `storageFor` falls back to the initial value.
    ///
    /// Ported from `useAppTheme.ts` (`isThemeSlug`).
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

    /// The scheme this theme belongs to.
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

    /// The dark pick this slug names, if any.
    pub fn dark(self) -> Option<DarkTheme> {
        match self {
            ThemeSlug::ArcticIce => Some(DarkTheme::ArcticIce),
            ThemeSlug::TronCyan => Some(DarkTheme::TronCyan),
            ThemeSlug::MidnightViolet => Some(DarkTheme::MidnightViolet),
            ThemeSlug::CleanTeal | ThemeSlug::EducationalWarm | ThemeSlug::SageEditorial => None,
        }
    }

    /// The light pick this slug names, if any.
    pub fn light(self) -> Option<LightTheme> {
        match self {
            ThemeSlug::ArcticIce | ThemeSlug::TronCyan | ThemeSlug::MidnightViolet => None,
            ThemeSlug::CleanTeal => Some(LightTheme::CleanTeal),
            ThemeSlug::EducationalWarm => Some(LightTheme::EducationalWarm),
            ThemeSlug::SageEditorial => Some(LightTheme::SageEditorial),
        }
    }
}

impl From<DarkTheme> for ThemeSlug {
    fn from(dark: DarkTheme) -> ThemeSlug {
        match dark {
            DarkTheme::ArcticIce => ThemeSlug::ArcticIce,
            DarkTheme::TronCyan => ThemeSlug::TronCyan,
            DarkTheme::MidnightViolet => ThemeSlug::MidnightViolet,
        }
    }
}

impl From<LightTheme> for ThemeSlug {
    fn from(light: LightTheme) -> ThemeSlug {
        match light {
            LightTheme::CleanTeal => ThemeSlug::CleanTeal,
            LightTheme::EducationalWarm => ThemeSlug::EducationalWarm,
            LightTheme::SageEditorial => ThemeSlug::SageEditorial,
        }
    }
}

/// The theme picks: the light/dark selector, one theme per scheme, and the
/// system's scheme.
///
/// Ported from `useAppTheme.ts` (the three `atomWithStorage` atoms plus the
/// `systemColorSchemeAtom`): defaults system, arctic-ice, clean-teal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThemeState {
    pub light_dark: LightDark,
    pub dark_theme: DarkTheme,
    pub light_theme: LightTheme,
    pub system: SystemScheme,
}

impl Default for ThemeState {
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
    /// The scheme in use: the pick's, or the system's under `System`.
    ///
    /// Ported from `useAppTheme.ts` (`appThemeAtom`'s `resolvedLightDark`).
    pub fn resolved_scheme(&self) -> SystemScheme {
        match self.light_dark {
            LightDark::System => self.system,
            LightDark::Dark => SystemScheme::Dark,
            LightDark::Light => SystemScheme::Light,
        }
    }

    /// The theme shown: the pick for the scheme in use.
    ///
    /// Ported from `useAppTheme.ts` (`appThemeAtom`).
    pub fn resolved(&self) -> ThemeSlug {
        if self.resolved_scheme() == SystemScheme::Dark {
            ThemeSlug::from(self.dark_theme)
        } else {
            ThemeSlug::from(self.light_theme)
        }
    }

    /// Picks the theme for the resolved scheme, as the menu does: a dark
    /// slug changes the dark pick, a light slug the light pick.
    ///
    /// Ported from `useAppTheme.ts` (`setThemeSlug`).
    pub fn set_theme_slug(&mut self, slug: ThemeSlug) {
        if self.resolved_scheme() == SystemScheme::Dark {
            if let Some(dark) = slug.dark() {
                self.dark_theme = dark;
            }
        } else if let Some(light) = slug.light() {
            self.light_theme = light;
        }
    }

    /// Picks the light/dark selector.
    pub fn set_light_dark(&mut self, value: LightDark) {
        self.light_dark = value;
    }

    /// Picks the dark theme.
    pub fn set_dark_theme(&mut self, value: DarkTheme) {
        self.dark_theme = value;
    }

    /// Picks the light theme.
    pub fn set_light_theme(&mut self, value: LightTheme) {
        self.light_theme = value;
    }

    /// The storage mirror: the mode, and one theme per scheme.
    pub fn prefs(&self) -> mv_theme::Prefs {
        mv_theme::Prefs {
            mode: match self.light_dark {
                LightDark::System => mv_theme::Mode::System,
                LightDark::Dark => mv_theme::Mode::Dark,
                LightDark::Light => mv_theme::Mode::Light,
            },
            dark: match self.dark_theme {
                DarkTheme::ArcticIce => mv_theme::Slug::ArcticIce,
                DarkTheme::TronCyan => mv_theme::Slug::TronCyan,
                DarkTheme::MidnightViolet => mv_theme::Slug::MidnightViolet,
            },
            light: match self.light_theme {
                LightTheme::CleanTeal => mv_theme::Slug::CleanTeal,
                LightTheme::EducationalWarm => mv_theme::Slug::EducationalWarm,
                LightTheme::SageEditorial => mv_theme::Slug::SageEditorial,
            },
        }
    }

    /// The state for stored prefs: each field checked, a missing or foreign
    /// one taking its default, as `storageFor` checks each key.
    pub fn from_prefs(prefs: mv_theme::Prefs) -> ThemeState {
        ThemeState {
            light_dark: match prefs.mode {
                mv_theme::Mode::System => LightDark::System,
                mv_theme::Mode::Dark => LightDark::Dark,
                mv_theme::Mode::Light => LightDark::Light,
            },
            dark_theme: match prefs.dark {
                mv_theme::Slug::ArcticIce => DarkTheme::ArcticIce,
                mv_theme::Slug::TronCyan => DarkTheme::TronCyan,
                mv_theme::Slug::MidnightViolet => DarkTheme::MidnightViolet,
                mv_theme::Slug::CleanTeal
                | mv_theme::Slug::EducationalWarm
                | mv_theme::Slug::SageEditorial => DarkTheme::ArcticIce,
            },
            light_theme: match prefs.light {
                mv_theme::Slug::CleanTeal => LightTheme::CleanTeal,
                mv_theme::Slug::EducationalWarm => LightTheme::EducationalWarm,
                mv_theme::Slug::SageEditorial => LightTheme::SageEditorial,
                mv_theme::Slug::ArcticIce
                | mv_theme::Slug::TronCyan
                | mv_theme::Slug::MidnightViolet => LightTheme::CleanTeal,
            },
            ..ThemeState::default()
        }
    }
}

/// The three colours the menu's swatch shows for a theme: its background,
/// its curve ink, its point ink.
///
/// Ported from `SettingsMenu.tsx` (`ThemeSwatch` paints `var(--background)`,
/// `var(--chart-line)`, `var(--chart-accent)` under the row's theme class).
pub fn swatch_colours(_slug: ThemeSlug) -> [mv_theme::Token; 3] {
    use mv_theme::Token::{Background, ChartAccent, ChartLine};
    [Background, ChartLine, ChartAccent]
}

impl App {
    /// Picks the light/dark selector, saves it, and starts the transition.
    pub fn set_light_dark(&mut self, value: LightDark) {
        self.theme.set_light_dark(value);
        crate::prefs::save_theme(self);
        crate::transition::start(self);
    }

    /// Picks the dark theme, saves it, and starts the transition.
    pub fn set_dark_theme(&mut self, value: DarkTheme) {
        self.theme.set_dark_theme(value);
        crate::prefs::save_theme(self);
        crate::transition::start(self);
    }

    /// Picks the light theme, saves it, and starts the transition.
    pub fn set_light_theme(&mut self, value: LightTheme) {
        self.theme.set_light_theme(value);
        crate::prefs::save_theme(self);
        crate::transition::start(self);
    }

    /// Picks the theme for the resolved scheme, saves it, and starts the
    /// transition.
    pub fn set_theme_slug(&mut self, slug: ThemeSlug) {
        self.theme.set_theme_slug(slug);
        crate::prefs::save_theme(self);
        crate::transition::start(self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_system_arctic_clean_teal() {
        let state = ThemeState::default();
        assert_eq!(state.light_dark, LightDark::System);
        assert_eq!(state.dark_theme, DarkTheme::ArcticIce);
        assert_eq!(state.light_theme, LightTheme::CleanTeal);
    }

    #[test]
    fn words_parse_and_spell() {
        for (word, want) in [
            ("system", Some(LightDark::System)),
            ("dark", Some(LightDark::Dark)),
            ("light", Some(LightDark::Light)),
            ("dusk", None),
            ("", None),
            ("arctic-ice", None),
        ] {
            assert_eq!(LightDark::parse(word), want, "{word:?}");
        }
        assert_eq!(LightDark::System.word(), "system");
        assert_eq!(LightDark::System.name(), "System");
        assert_eq!(LightDark::Dark.name(), "Dark");
        assert_eq!(LightDark::Light.name(), "Light");
    }

    #[test]
    fn slugs_parse_spell_and_label() {
        let cases = [
            (DarkTheme::ArcticIce, "arctic-ice", "Arctic Ice"),
            (DarkTheme::TronCyan, "tron-cyan", "Tron Cyan"),
            (
                DarkTheme::MidnightViolet,
                "midnight-violet",
                "Midnight Violet",
            ),
        ];
        for (dark, slug, label) in cases {
            assert_eq!(DarkTheme::parse(slug), Some(dark));
            assert_eq!(dark.slug(), slug);
            assert_eq!(dark.name(), label);
            let theme = ThemeSlug::from(dark);
            assert_eq!(ThemeSlug::parse(slug), Some(theme));
            assert_eq!(theme.slug(), slug);
            assert_eq!(theme.name(), label);
            assert_eq!(theme.scheme(), SystemScheme::Dark);
            assert_eq!(theme.dark(), Some(dark));
            assert_eq!(theme.light(), None);
        }
        let cases = [
            (LightTheme::CleanTeal, "clean-teal", "Clean Teal"),
            (
                LightTheme::EducationalWarm,
                "educational-warm",
                "Educational Warm",
            ),
            (
                LightTheme::SageEditorial,
                "sage-editorial",
                "Sage Editorial",
            ),
        ];
        for (light, slug, label) in cases {
            assert_eq!(LightTheme::parse(slug), Some(light));
            assert_eq!(light.slug(), slug);
            assert_eq!(light.name(), label);
            let theme = ThemeSlug::from(light);
            assert_eq!(ThemeSlug::parse(slug), Some(theme));
            assert_eq!(theme.name(), label);
            assert_eq!(theme.scheme(), SystemScheme::Light);
            assert_eq!(theme.dark(), None);
            assert_eq!(theme.light(), Some(light));
        }
        // A slug of the other scheme parses for themes but not for the pick.
        assert_eq!(DarkTheme::parse("clean-teal"), None);
        assert_eq!(LightTheme::parse("arctic-ice"), None);
        assert_eq!(ThemeSlug::parse("neon"), None);
        assert_eq!(ThemeSlug::parse(""), None);
    }

    #[test]
    fn resolved_follows_scheme_then_pick() {
        // Under System the system's scheme picks the theme.
        let state = ThemeState::default();
        assert_eq!(state.resolved_scheme(), SystemScheme::Dark);
        assert_eq!(state.resolved(), ThemeSlug::ArcticIce);
        let mut light_system = state;
        light_system.system = SystemScheme::Light;
        assert_eq!(light_system.resolved(), ThemeSlug::CleanTeal);
        // Dark and Light hold their scheme whatever the system says.
        let dark = ThemeState {
            light_dark: LightDark::Dark,
            system: SystemScheme::Light,
            ..state
        };
        assert_eq!(dark.resolved(), ThemeSlug::ArcticIce);
        let light = ThemeState {
            light_dark: LightDark::Light,
            system: SystemScheme::Dark,
            ..state
        };
        assert_eq!(light.resolved(), ThemeSlug::CleanTeal);
    }

    #[test]
    fn set_theme_slug_writes_the_resolved_schemes_pick() {
        // As `setThemeSlug`: under a dark resolution a light slug changes
        // nothing, and under a light resolution a dark slug changes nothing.
        let mut state = ThemeState::default();
        state.set_theme_slug(ThemeSlug::TronCyan);
        assert_eq!(state.dark_theme, DarkTheme::TronCyan);
        assert_eq!(state.light_theme, LightTheme::CleanTeal);
        state.set_theme_slug(ThemeSlug::CleanTeal);
        assert_eq!(state.dark_theme, DarkTheme::TronCyan);
        let mut state = ThemeState {
            system: SystemScheme::Light,
            ..ThemeState::default()
        };
        state.set_theme_slug(ThemeSlug::SageEditorial);
        assert_eq!(state.light_theme, LightTheme::SageEditorial);
        state.set_theme_slug(ThemeSlug::MidnightViolet);
        assert_eq!(state.dark_theme, DarkTheme::ArcticIce);
        assert_eq!(state.resolved(), ThemeSlug::SageEditorial);
    }

    #[test]
    fn prefs_mirror_is_exact() {
        let state = ThemeState::default();
        let prefs = state.prefs();
        assert_eq!(prefs.mode, mv_theme::Mode::System);
        assert_eq!(
            mv_theme::prefs_json(prefs),
            "{\"mode\":\"system\",\"dark\":\"arctic-ice\",\"light\":\"clean-teal\"}"
        );
        assert_eq!(ThemeState::from_prefs(prefs), state);
        for slug in ALL_SLUGS {
            let mut state = ThemeState {
                system: slug.scheme(),
                ..ThemeState::default()
            };
            state.set_theme_slug(slug);
            // The system scheme is platform state, never stored: the mirror
            // round-trips the picks, and the system stays the default.
            let mut want = state;
            want.system = SystemScheme::Dark;
            assert_eq!(ThemeState::from_prefs(state.prefs()), want);
        }
        // A pick of the wrong scheme falls back, as `storageFor` falls back.
        let mut prefs = mv_theme::DEFAULT_PREFS;
        prefs.dark = mv_theme::Slug::CleanTeal;
        prefs.light = mv_theme::Slug::TronCyan;
        let state = ThemeState::from_prefs(prefs);
        assert_eq!(state.dark_theme, DarkTheme::ArcticIce);
        assert_eq!(state.light_theme, LightTheme::CleanTeal);
    }

    #[test]
    fn swatch_is_background_curve_point() {
        use mv_theme::Token::{Background, ChartAccent, ChartLine};
        for slug in ALL_SLUGS {
            assert_eq!(swatch_colours(slug), [Background, ChartLine, ChartAccent]);
        }
    }

    #[test]
    fn app_setters_save_and_resolve() {
        let mut app = crate::fixture::headless_app();
        app.set_light_dark(LightDark::Dark);
        assert_eq!(app.theme.light_dark, LightDark::Dark);
        assert_eq!(app.prefs.mode, mv_theme::Mode::Dark);
        assert_eq!(app.theme.resolved(), ThemeSlug::ArcticIce);
        app.set_theme_slug(ThemeSlug::MidnightViolet);
        assert_eq!(app.theme.dark_theme, DarkTheme::MidnightViolet);
        assert_eq!(app.prefs.dark, mv_theme::Slug::MidnightViolet);
        assert!(app.transition.active.is_some());
        assert!(app.dirty);
    }
}
