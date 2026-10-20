//! The page fixture: the reference layout every layout test measures against.
//!
//! The JSON is embedded at compile time; helpers parse it and look up shots
//! and boxes, failing the test with a message when either is missing. Layout
//! tests serialize on `mv_clay::tests_lock`: the layout context is global,
//! and one lock covers every test that lays out (a second mutex would let
//! two layouts interleave).

use std::collections::HashMap;

/// The embedded reference measurements.
pub const PAGE_FIXTURE: &str = include_str!("../testdata/page.json");

/// One measured text.
#[derive(Clone, Debug, serde::Deserialize)]
pub struct FixtureText {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub baseline: f32,
}

/// One measured scroll container.
#[derive(Clone, Debug, serde::Deserialize)]
pub struct FixtureScroll {
    /// scrollTop.
    pub top: f32,
    /// scrollHeight.
    pub height: f32,
    /// clientHeight.
    pub client: f32,
}

/// One engine's measures: boxes, texts, runs and scrolls by name.
#[derive(Clone, Debug, serde::Deserialize)]
pub struct FixtureMeasures {
    pub boxes: HashMap<String, [f32; 4]>,
    pub texts: HashMap<String, FixtureText>,
    pub runs: HashMap<String, Vec<FixtureText>>,
    pub scroll: HashMap<String, FixtureScroll>,
}

/// One shot: its size and both engines' measures.
#[derive(Clone, Debug, serde::Deserialize)]
pub struct FixtureShot {
    pub size: [f32; 2],
    pub webkit: FixtureMeasures,
    pub chromium: FixtureMeasures,
}

/// The whole fixture: what reference, and its shots by name.
#[derive(Clone, Debug, serde::Deserialize)]
pub struct PageFixture {
    pub reference: String,
    pub shots: HashMap<String, FixtureShot>,
}

/// Parses the embedded fixture.
pub fn page_fixture() -> Result<PageFixture, String> {
    serde_json::from_str(PAGE_FIXTURE).map_err(|e| format!("page fixture: {e}"))
}

/// The named shot, or an error when it is missing.
pub fn fixture_shot(fixture: &PageFixture, name: &str) -> Result<FixtureShot, String> {
    fixture
        .shots
        .get(name)
        .cloned()
        .ok_or_else(|| format!("page fixture: no shot {name:?}"))
}

/// The named box, or an error when it is missing.
pub fn fixture_box(m: &FixtureMeasures, name: &str) -> Result<[f32; 4], String> {
    m.boxes
        .get(name)
        .copied()
        .ok_or_else(|| format!("page fixture: no box {name:?}"))
}

#[cfg(test)]
/// The Rust `new(App)`: every field at its zero or resting value.
///
/// The Odin tests build `new(App)` (zeroed, window and GL context nil) and
/// set the fields they need; this is that constructor for the Rust tests.
/// No test opens a window: a `cargo test` thread cannot start SDL video
/// (measured 2026-10-18: `sdl3::init()` refuses off the main thread, and
/// with one thread SDL answers "No available video device"), and no Odin
/// test draws. `viewer` stays the zero state here: each test sets
/// `mv_viewer::initial_state()` itself, as the Odin tests do.
pub(crate) fn headless_app() -> crate::app::App {
    // One body with `App::bare` (`app.rs::zeroed`): the headless tests
    // spell the window and GL owners `None`, `app_init` spells them live.
    crate::app::App::zeroed(None, None, None, mv_nvg::Context::null())
}
