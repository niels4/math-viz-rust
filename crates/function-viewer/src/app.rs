//! The shell's app state: the window and GL owners, the harness state, and
//! the clock.
//!
//! One `App` owns everything; events become input facts, and each frame draws
//! what is dirty. The page milestones append their fields at the end (fields
//! are never reordered).

use enum_map::EnumMap;

use crate::app_loop;
use crate::input::InputState;
use crate::scroll::{Scroll, Scroller};
use crate::url::Url;

/// The shell's app state.
pub struct App {
    /// The window; `None` in a test that never opens one (`headless_app`).
    ///
    /// The harness and the platform own a live window, while a unit test has
    /// none (a `cargo test` thread cannot start SDL video). Readers go through
    /// [`App::window`], which panics on `None`.
    pub window: Option<sdl3::video::Window>,
    /// The window's GL context; `None` beside a `None` window.
    pub gl_context: Option<sdl3::video::GLContext>,
    pub vg: mv_nvg::Context,
    /// Clay's arena backing.
    pub clay_memory: Vec<u8>,
    /// The window's logical size.
    pub size: [f32; 2],
    /// Its framebuffer, in device pixels.
    pub pixels: [i32; 2],
    pub input: InputState,
    pub r#loop: app_loop::Loop,
    pub timers: app_loop::Timers,
    pub prefs: mv_theme::Prefs,
    /// The colour space the frames are drawn in, settled at start: Display P3
    /// where the surface shows it, else sRGB.
    pub space: mv_theme::Space,
    /// Every theme, resolved at start in `space`.
    pub themes: EnumMap<mv_theme::Slug, mv_theme::Theme>,
    /// Where the prefs live: a file natively, storage keys on the web; empty
    /// while a script runs, which keeps them in memory.
    pub prefs_store: String,
    pub system_scheme: mv_theme::Scheme,
    pub reduced_motion: bool,
    /// The scroll containers, as the last layout left them.
    pub scroll: EnumMap<Scroller, Scroll>,
    /// Something changed since the last frame was drawn.
    pub dirty: bool,
    /// The page's faces, shaped and drawn by the text crate.
    pub fonts: mv_text::Fonts,
    /// The Clay context, set up by `ui::init_clay`.
    pub clay: Option<mv_clay::Clay>,
    /// The page query: `--query` at start natively, the hash on the web. The
    /// harness writes it to `views.json` whenever it changes.
    ///
    /// The query string plus its route (`url::Url`); the page milestones'
    /// params parse from it and write back through it.
    pub url: Url,
    /// The GL loader for the app's few direct GL calls, `None` in tests
    /// (`headless_app`), which never call GL.
    ///
    /// Appended at the end (fields are never reordered).
    pub gl: Option<glow::Context>,
    /// The theme picks: the light/dark selector, one theme per scheme, and
    /// the system's scheme. The page draws the resolved theme.
    ///
    /// Appended at the end (fields are never reordered).
    pub theme: crate::theme_state::ThemeState,
    /// The theme cross-fade in flight, with the last stable frame's pixels.
    ///
    /// Appended at the end (fields are never reordered).
    pub transition: crate::transition::Transition,
    /// The Cartesian plane's camera: drag to pan, flick to glide, wheel to
    /// zoom (h2-input; `camera.rs` owns the state machine).
    pub camera: crate::camera::Camera,
    /// The `center` page param: the live value, its debounce and its flush
    /// (h2-input; `url.rs` owns the state machine).
    pub center_param: crate::url::QueryParam<[f64; 2]>,
    /// The `scale` page param, as above.
    pub scale_param: crate::url::QueryParam<f64>,
    /// When the camera last stepped, on the app's clock; 0 when it is still
    /// (h2-input; the next step's `dt` measures from here).
    pub camera_frame_at: crate::app_loop::Time,
}

impl App {
    /// The window; panics in a test that never opens one.
    ///
    /// Every reader of `app.window` goes through here, so a headless `App`
    /// fails loudly at the read instead of threading `Option` handling.
    pub fn window(&self) -> &sdl3::video::Window {
        self.window.as_ref().expect("no window: headless App")
    }

    /// The window, mutably; panics in a test that never opens one.
    ///
    /// The two resizes (`set_icon`, `set_size`) take `&mut`, so they go
    /// through here.
    pub fn window_mut(&mut self) -> &mut sdl3::video::Window {
        self.window.as_mut().expect("no window: headless App")
    }

    /// The app with a live window and GL, everything else zeroed: `app_init`
    /// (`main.rs`) fills the rest (colour space, themes, fonts, Clay, prefs)
    /// before the first frame. Unit tests build theirs through
    /// `fixture::headless_app` instead, which never opens a window.
    ///
    /// Appended at the end (fields are never reordered).
    pub fn bare(
        window: sdl3::video::Window,
        gl_context: sdl3::video::GLContext,
        gl: glow::Context,
        vg: mv_nvg::Context,
    ) -> App {
        Self::zeroed(Some(window), Some(gl_context), Some(gl), vg)
    }

    /// The zeroed app, with the window and GL owners the caller hands in
    /// (`None` in a headless test App). `bare` (above) is `app_init`'s
    /// spelling with a live window; `fixture::headless_app` is the tests'
    /// spelling with none: one body, so the two constructors cannot drift
    /// apart.
    ///
    /// Appended at the end (fields are never reordered).
    pub(crate) fn zeroed(
        window: Option<sdl3::video::Window>,
        gl_context: Option<sdl3::video::GLContext>,
        gl: Option<glow::Context>,
        vg: mv_nvg::Context,
    ) -> App {
        const ZERO_BOX: mv_sys::clay::BoundingBox = mv_sys::clay::BoundingBox {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        };
        const ZERO_SCROLL: crate::scroll::Scroll = crate::scroll::Scroll {
            active: false,
            offset: 0.0,
            max: 0.0,
            r#box: ZERO_BOX,
            border: 0.0,
        };
        App {
            window,
            gl_context,
            gl,
            vg,
            clay_memory: Vec::new(),
            clay: None,
            size: [0.0, 0.0],
            pixels: [0, 0],
            input: crate::input::InputState {
                pointer: [0.0, 0.0],
                time: 0,
                over: false,
                mods: sdl3::keyboard::Mod::empty(),
                hover: crate::input::Hit::default(),
                pressed: crate::input::Hit::default(),
                focus: crate::input::Hit::default(),
                focus_visible: false,
                keyed: false,
                reveal: false,
                cursor: crate::input::Cursor::Default,
                regions: Vec::new(),
                stops: Vec::new(),
                declared: Vec::new(),
                clip: None,
                z: 0,
                inert: false,
                inert_layers: 0,
                texts: Vec::new(),
                text_clip: None,
                text_z: 0,
            },
            r#loop: crate::app_loop::Loop {
                script_time: None,
                pace: crate::app_loop::Pace::Display,
                awake: false,
                wake_at: 0,
                wake_timer: 0,
                armed_at: 0,
                iterations: 0,
                frames: 0,
                recording: false,
                turns: Vec::new(),
            },
            timers: EnumMap::from_array([]),
            prefs: mv_theme::DEFAULT_PREFS,
            space: mv_theme::Space::Srgb,
            themes: EnumMap::from_array([
                mv_theme::resolve(mv_theme::Slug::ArcticIce, mv_theme::Space::Srgb),
                mv_theme::resolve(mv_theme::Slug::TronCyan, mv_theme::Space::Srgb),
                mv_theme::resolve(mv_theme::Slug::MidnightViolet, mv_theme::Space::Srgb),
                mv_theme::resolve(mv_theme::Slug::CleanTeal, mv_theme::Space::Srgb),
                mv_theme::resolve(mv_theme::Slug::EducationalWarm, mv_theme::Space::Srgb),
                mv_theme::resolve(mv_theme::Slug::SageEditorial, mv_theme::Space::Srgb),
            ]),
            prefs_store: String::new(),
            system_scheme: mv_theme::Scheme::Dark,
            reduced_motion: false,
            scroll: EnumMap::from_array([ZERO_SCROLL, ZERO_SCROLL]),
            dirty: false,
            // Uninitialised shaping fonts; `app_init` runs `mv_text::init`.
            fonts: mv_text::uninit(),
            url: Url::from_query(""),
            theme: crate::theme_state::ThemeState::default(),
            transition: crate::transition::Transition::default(),
            // The plane's camera at the URL defaults; each `build()` re-seeds
            // camera and params from the real page query
            // (`plane_input::init_camera_from_url`).
            camera: crate::camera::Camera::new(
                crate::camera::DEFAULT_CENTER,
                crate::camera::DEFAULT_SCALE,
            ),
            center_param: crate::url::QueryParam::new(&crate::plane_input::CENTER_DEF, ""),
            scale_param: crate::url::QueryParam::new(&crate::plane_input::SCALE_DEF, ""),
            camera_frame_at: 0,
        }
    }
}
