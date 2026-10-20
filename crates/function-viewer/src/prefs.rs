//! The stored appearance prefs: loading, saving and applying them.
//!
//! Natively the prefs live in a file; on the web in local storage. While a
//! script runs they stay in memory.

use crate::app::App;
#[cfg(not(target_os = "emscripten"))]
use crate::platform_native::{platform_load_prefs, platform_save_prefs};
#[cfg(target_os = "emscripten")]
use crate::platform_web::{platform_load_prefs, platform_save_prefs};

/// Loads the stored prefs, or stores the defaults when none are stored.
pub fn load_prefs(app: &mut App) {
    let (text, found) = platform_load_prefs(app);
    if !found {
        app.prefs = mv_theme::DEFAULT_PREFS;
        println!(
            "prefs: none stored; the defaults, {}",
            mv_theme::prefs_json(app.prefs)
        );
        save_prefs(app);
        return;
    }
    app.prefs = mv_theme::parse_prefs(&text);
    println!("prefs read: {}", mv_theme::prefs_json(app.prefs));
}

/// Serializes the prefs and saves them through the platform.
pub fn save_prefs(app: &mut App) {
    let text = mv_theme::prefs_json(app.prefs);
    if platform_save_prefs(app, &text) {
        println!("prefs written: {text}");
    }
}

/// Updates the prefs, marks dirty, and saves when they changed.
pub fn set_prefs(app: &mut App, prefs: mv_theme::Prefs) {
    if prefs == app.prefs {
        return;
    }
    app.prefs = prefs;
    app.dirty = true;
    save_prefs(app);
}

/// Adopts prefs stored elsewhere (another tab) without writing back.
pub fn prefs_stored_elsewhere(app: &mut App, prefs: mv_theme::Prefs) {
    if prefs != app.prefs {
        println!("prefs changed elsewhere: {}", mv_theme::prefs_json(prefs));
        app.prefs = prefs;
        app.dirty = true;
    }
}

/// Packs mode/dark/light into one word for the user event.
pub fn prefs_word(prefs: mv_theme::Prefs) -> usize {
    (prefs.mode as usize) | (prefs.dark as usize) << 8 | (prefs.light as usize) << 16
}

/// Unpacks the word into prefs.
pub fn prefs_of_word(word: usize) -> mv_theme::Prefs {
    mv_theme::Prefs {
        mode: mode_of_byte((word & 0xff) as u8),
        dark: slug_of_byte(((word >> 8) & 0xff) as u8, mv_theme::DEFAULT_PREFS.dark),
        light: slug_of_byte(((word >> 16) & 0xff) as u8, mv_theme::DEFAULT_PREFS.light),
    }
}

/// A prefs word only ever holds values the app packed itself, so every byte
/// is a valid discriminant; anything else falls back to the defaults rather
/// than failing, since the word crosses the event queue unchecked.
fn mode_of_byte(byte: u8) -> mv_theme::Mode {
    match byte {
        1 => mv_theme::Mode::Light,
        2 => mv_theme::Mode::Dark,
        _ => mv_theme::Mode::System,
    }
}

/// As above, for a theme slug.
fn slug_of_byte(byte: u8, fallback: mv_theme::Slug) -> mv_theme::Slug {
    match byte {
        0 => mv_theme::Slug::ArcticIce,
        1 => mv_theme::Slug::TronCyan,
        2 => mv_theme::Slug::MidnightViolet,
        3 => mv_theme::Slug::CleanTeal,
        4 => mv_theme::Slug::EducationalWarm,
        5 => mv_theme::Slug::SageEditorial,
        _ => fallback,
    }
}
