//! The whole page state: every card, layer and driver the app owns.
//!
//! One `App` owns everything; events become input facts, input becomes viewer
//! actions through one dispatch, and each frame draws what is dirty or moving.

use enum_map::EnumMap;

use crate::app_loop;
use crate::draw::LayerBuffers;
use crate::input::InputState;
use crate::scroll::{Scroll, Scroller};

/// The whole page state.
///
/// Seven fields arrive with the crates that own their types (fonts, the
/// viewer's model, the plane view, the number boxes, the motion driver, the
/// explainer and the settings panel) and are appended at the end then.
pub struct App {
    /// The window; `None` in a test that never opens one (`headless_app`).
    ///
    /// Ported from the Odin app's nil-able window pointer: the harness and
    /// the platform own a live window, while a unit test has none (a
    /// `cargo test` thread cannot start SDL video). Readers go through
    /// [`App::window`], which panics on `None`.
    pub window: Option<sdl3::video::Window>,
    /// The window's GL context; `None` beside a `None` window.
    ///
    /// Ported from the Odin app's nil-able GL context pointer.
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
    /// Where the prefs live: a file natively, a storage key on the web; empty
    /// while a script runs, which keeps them in memory.
    pub prefs_store: String,
    pub system_scheme: mv_theme::Scheme,
    pub reduced_motion: bool,
    /// The equation card's width, as the last layout placed it.
    pub equation_width: f32,
    /// The page's and the panel's scroll, as the last layout left them.
    pub scroll: EnumMap<Scroller, Scroll>,
    /// Where an element's subtree is drawn as one layer.
    pub layers: LayerBuffers,
    /// Something changed since the last frame was drawn.
    pub dirty: bool,
    // Appended at the viewer/text/plane merge (fields are never reordered):
    // the page's faces, the view's model, the plane view, the number boxes,
    // what moves, the explainer, and the settings panel.
    /// The page's faces, shaped and drawn by the text crate.
    pub fonts: mv_text::Fonts,
    /// The view's model: the function, the parameters, P and Q.
    pub viewer: mv_viewer::State,
    /// The plane's view and what it last laid out.
    pub plane: crate::plane_area::PlaneState,
    /// The number boxes' own: a press or drag, a rest, the open field.
    pub boxes: crate::number_box::Boxes,
    /// What moves, since when.
    pub motion: crate::motion_driver::Motion,
    /// The explainer as the page shows it.
    pub explainer: crate::explainer::ExplainerView,
    /// The settings panel: open or not, and its list's fade.
    pub settings: crate::settings::Settings,
    /// The Clay context, set up by `ui::init_clay` (render-base).
    pub clay: Option<mv_clay::Clay>,
    /// The GL loader for the app's few direct GL calls (`draw.odin:44-46`
    /// and `244-246`, `icons.odin:167-176`, `harness.odin:462` and `585`),
    /// created by `i0` in `app_init` as `t0a`'s `main.rs:778` does; `None`
    /// in tests (`headless_app`), which never call GL.
    ///
    /// Appended at the end (fields are never reordered).
    pub gl: Option<glow::Context>,
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

    /// The app with a live window and GL, everything else as the zeroed
    /// memory Odin's `new(App)` starts from: `app_init` (`main.rs`, `i0`)
    /// fills the rest (colour space, themes, fonts, Clay, model, prefs,
    /// motion) before the first frame. Unit tests build theirs through
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

    /// The zeroed memory Odin's `new(App)` starts from, with the window
    /// and GL owners the caller hands in (`None` in a headless test App).
    /// `bare` (above) is `app_init`'s spelling with a live window;
    /// `fixture::headless_app` is the tests' spelling with none: one body,
    /// so the two constructors cannot drift apart (i5 dedupes i0's pair).
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
        let zero_transition = mv_motion::Transition {
            from: 0.0,
            to: 0.0,
            start: 0.0,
            duration: 0.0,
            ease: mv_motion::Ease::Linear,
            reversing_from: 0.0,
            shortening: 0.0,
        };
        let zero_overlay = mv_motion::Overlay {
            open: false,
            shown: false,
            since: 0.0,
            exit: 0.0,
            alpha: zero_transition,
            shape: zero_transition,
        };
        let zero_fade = mv_motion::FadeIn {
            start: 0.0,
            duration: 0.0,
            ease: mv_motion::Ease::Linear,
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
            timers: EnumMap::from_array([0, 0, 0, 0]),
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
            equation_width: 0.0,
            scroll: EnumMap::from_array([ZERO_SCROLL, ZERO_SCROLL]),
            layers: crate::draw::LayerBuffers {
                buffers: [None, None],
                pixels: [[0, 0], [0, 0]],
                open: [
                    crate::draw::OpenLayer {
                        layer: crate::effects::LAYER_REST,
                        target: 0,
                    },
                    crate::draw::OpenLayer {
                        layer: crate::effects::LAYER_REST,
                        target: 0,
                    },
                ],
                depth: 0,
                skipped: 0,
                warned: false,
                scaled: [ZERO_BOX, ZERO_BOX, ZERO_BOX, ZERO_BOX],
                scaled_count: 0,
            },
            dirty: false,
            // Uninitialised shaping fonts; `app_init` runs `mv_text::init`
            // (text-shape's constructor, which also tracks the kb context's
            // lifetime as `Run<'ctx>`).
            fonts: mv_text::uninit(),
            viewer: mv_viewer::State {
                func: mv_viewer::Base::X,
                params: EnumMap::from_array([0.0, 0.0, 0.0, 0.0]),
                p_x: 0.0,
                q_x: None,
                ghost_on: false,
                view: None,
                hover: None,
                plane_over: None,
                eq_over: None,
                focus: None,
                lead: mv_viewer::Lead::Hover,
                drag: None,
                edit: None,
                p_before: None,
                explainer: None,
                change: mv_viewer::Change::Direct,
            },
            plane: crate::plane_area::PlaneState {
                view: mv_plane::ViewState {
                    zoom: 0.0,
                    pan_x: 0.0,
                    pan_y: 0.0,
                },
                fit: 0.0,
                r#box: ZERO_BOX,
                frame: mv_plane::Frame {
                    vp: mv_plane::Viewport {
                        width: 0.0,
                        height: 0.0,
                        zoom: 0.0,
                        origin_x: 0.0,
                        origin_y: 0.0,
                    },
                    scene: mv_plane::Scene {
                        grid_alpha: None,
                        curves: Vec::new(),
                        points: Vec::new(),
                        guides: Vec::new(),
                        handles: Vec::new(),
                        annotations: Vec::new(),
                    },
                    grid: mv_plane::GridLayout {
                        minor_x: Vec::new(),
                        major_x: Vec::new(),
                        minor_y: Vec::new(),
                        major_y: Vec::new(),
                        axis_x: None,
                        axis_y: None,
                        steps: mv_plane::GridSteps {
                            minor: 0.0,
                            major: 0.0,
                        },
                        ticks_x: Vec::new(),
                        ticks_y: Vec::new(),
                        labels: Vec::new(),
                        origin: None,
                    },
                    marks: mv_plane::MarksLayout {
                        annotations: Vec::new(),
                        handles: Vec::new(),
                        guides: Vec::new(),
                        drop_lines: Vec::new(),
                        points: Vec::new(),
                        hits: Vec::new(),
                    },
                },
                hits: Vec::new(),
                grip: None,
                samples: Vec::new(),
                glide: None,
                still: None,
            },
            boxes: crate::number_box::Boxes {
                scrub: None,
                resting: None,
                field: crate::number_field::Field {
                    open: false,
                    part: mv_viewer::Part::A,
                    text: [0; 64],
                    len: 0,
                    caret: 0,
                    anchor: 0,
                    error: String::new(),
                    base: 0.0,
                    blink_on: false,
                    selecting: false,
                },
            },
            motion: crate::motion_driver::Motion {
                layers: mv_viewer::AT_REST,
                seen: None,
                flashes: EnumMap::from_array([None, None, None, None]),
                now: 0.0,
                arrivals: Vec::new(),
                arriving: false,
                q_row: zero_fade,
                q_live: None,
                overlays: EnumMap::from_array([zero_overlay, zero_overlay]),
                watched: [crate::motion_driver::Watched {
                    motion: crate::motion_driver::WatchedMotion::Fade(std::ptr::null_mut()),
                    translucent: false,
                }; crate::motion_driver::WATCHED_MAX],
                watching: 0,
                moving: false,
                plane: crate::motion_driver::MotionCount {
                    since: 0,
                    span: 0.0,
                    frames: 0,
                },
            },
            explainer: crate::explainer::ExplainerView {
                shown: None,
                placed: None,
                resting: None,
            },
            settings: crate::settings::Settings {
                open: false,
                scheme: mv_theme::Scheme::Dark,
                swapped: false,
                swap: zero_fade,
            },
        }
    }
}
