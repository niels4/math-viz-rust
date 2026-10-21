//! The shell's app state: the window and GL owners, the harness state, and
//! the clock.
//!
//! One `App` owns everything; events become input facts, and each frame draws
//! what is dirty. The page milestones append their fields at the end (fields
//! are never reordered).

use enum_map::EnumMap;

use crate::app_loop;
use crate::input::InputState;
use crate::plane_input::HandleId;
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
    /// The settings menu's own state (open flag and keyboard focus); h1-settings
    /// appends it here, h1-frame's `ThemeState` holds the theme picks.
    pub settings: crate::settings::SettingsMenuState,
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
    /// The plotted function switch: the shown and fading functions with the
    /// ms since the switch began (h3; `plot.rs` owns the state machine).
    pub shown: crate::plot::Shown,
    /// The `func` page param: the live value with its flush (h3; `url.rs`
    /// owns the state machine; the pick writes it at once, as the hook's
    /// `setFunc` with `{ flush: true }` does).
    pub func_param: crate::url::QueryParam<crate::functions::FunctionSlug>,
    /// When the switch last stepped, on the app's clock; `None` while at
    /// rest and on a switch-from-rest's own turn (h3; the turn draws the
    /// render-phase curves, the loop steps from the turn after).
    pub plot_frame_at: Option<crate::app_loop::Time>,
    /// Skip the plot step on a mid-switch re-pick's own turn, keeping the
    /// running clock (h3; the hook's loop is keyed on `switching`, so a
    /// re-pick never restarts it: the turn draws the reset curves, the next
    /// frame diffs from the last callback, not the click).
    pub plot_hold: bool,
    /// The highlight model: the hovered and focused parameters and which
    /// moved last (h4-readout's `state.rs` owns the state machine; both h4
    /// milestones read and write it. h4-rows appends the field its rows need;
    /// h4-readout's merge keeps this line).
    pub state: crate::state::HighlightState,
    /// The `a` page param: the live value with its flush (h4-rows; `url.rs`
    /// owns the state machine; Enter writes it at once, as the hook's
    /// `setValue` with `{ flush: true }` does).
    pub a_param: crate::url::QueryParam<f64>,
    /// The `b` page param, as above.
    pub b_param: crate::url::QueryParam<f64>,
    /// The `h` page param, as above.
    pub h_param: crate::url::QueryParam<f64>,
    /// The `k` page param, as above.
    pub k_param: crate::url::QueryParam<f64>,
    /// P's x page param: the live value with its flush (h8-plane; `url.rs`
    /// owns the state machine; the slider writes it on release, as the
    /// hook's `setValue` does. h8-points-slider writes through this field).
    pub p_param: crate::url::QueryParam<f64>,
    /// The uncommitted text per parameter: `None` at rest (the box shows the
    /// value), `Some` once typed into (h4-rows; `number_field.rs` owns the
    /// display and commit rules).
    pub drafts: EnumMap<crate::state::TransformParam, crate::number_field::FieldState>,
    /// The drag the ruler draws: while a scrub holds a box, the rebased
    /// anchor off the row's laid-out box (h5-box mirrors it every drag fact;
    /// h5-ruler reads it).
    pub scrub: Option<crate::scrub_value::ScrubDrag>,
    /// A box press in flight: a scrub or a tap (h5-box; `scrub_value.rs`
    /// owns the machine).
    pub scrub_hold: Option<crate::scrub_value::ScrubHold>,
    /// The rest tip's armed parameter and arming time, on the app's clock
    /// (h5-box; `loop::Timer::ScrubRest` fires it).
    pub scrub_rest: Option<(crate::state::TransformParam, crate::app_loop::Time)>,
    /// The parameter whose rest tip shows (resolved when the timer fires).
    pub scrub_resting: Option<crate::state::TransformParam>,
    /// When the return-to-centre hover began, on the app's clock; `None`
    /// while the pointer is elsewhere (h6-handles; the tooltip shows past
    /// its 400 ms delay).
    pub overlay_tip_since: Option<crate::app_loop::Time>,
    /// The flip toggle's state: the pending flip, its spring, the press
    /// counter and the frame clock (h6-curve; `flip.rs` owns the machine).
    pub flip: crate::flip::FlipState,
    /// The last hovered handle (`None` for none), the `lastHoveredIndex`
    /// ref of `useMouseHandles.ts` (h7-plane; `plane_input.rs` owns the
    /// dispatch). It carries what the hook's `hovering` state did: the
    /// cursor rule reads it directly as "the pointer is on a handle"
    /// (`ea29aa9`, which deleted the separate state).
    pub hovered: Option<HandleId>,
    /// A plane handle drag in flight, with the press's grab offset in plane
    /// units (the `dragging` state and the `grab` ref at `251c080`).
    pub dragging: bool,
    pub grab: [f64; 2],
    /// The pressed handle while a drag flies (the hook's closed-over
    /// handle: the drag's formula pair never changes mid-drag, while the
    /// hover index follows the moves).
    pub dragged: Option<HandleId>,
    /// The moved badge's state machine (h8-points-row; `moved_by.rs` owns
    /// it): fed `(dragging, x, y)` every frame P's row draws.
    ///
    /// Appended at the end (fields are never reordered).
    pub moved_by: crate::moved_by::MovedBy,
    /// The plane's visible plot range for the Points rows' slots
    /// (h8-points-row): the sidebar lays out before the plane, so the
    /// layout reads the previous frame's range (at rest — every judged
    /// shot — it equals the current one). Refreshed in `paint_points`,
    /// dirtying on change so the first frame's `None` redraws converged
    /// instead of photographing stale.
    ///
    /// Appended at the end (fields are never reordered).
    pub visible_range: Option<crate::visible_range::VisibleRange>,
    /// P's x field: the edit over `p` (`x_field.rs` owns it), `None`-shaped
    /// at rest (h8-points-field).
    ///
    /// Appended at the end (fields are never reordered).
    pub x_field: crate::x_field::XField,
    /// An x-of-P slider drag in flight (h8-points-slider;
    /// `point_slider.rs` owns the machine; `state.p_dragging` mirrors it).
    pub slider_hold: Option<crate::point_slider::SliderDrag>,
    /// The x field was asked open from the slider's Enter (h8-points-field
    /// consumes it in its wiring, opening the field and clearing the flag).
    pub slider_edit: bool,
    /// The welcome backdrop's uploaded composite with its cache key
    /// (h10-hero; `welcome_backdrop.rs` owns the composite, `welcome.rs`
    /// paints it; `None` until the first welcome frame).
    ///
    /// Appended at the end (fields are never reordered).
    pub backdrop: Option<crate::welcome_backdrop::BackdropCache>,
    /// The welcome loop's state: the tour clock and the hero's own switch
    /// animation (h10-loop; `welcome_step.rs` owns the machine). Fresh at
    /// startup, restarted whenever the welcome route is entered.
    ///
    /// Appended at the end (fields are never reordered).
    pub welcome: crate::welcome_step::WelcomeState,
    /// The hero plane's offscreen bitmap at the device ratio (h10-fit;
    /// `welcome.rs` owns the composite, `page.rs` paints it; `None` until
    /// the first welcome frame).
    ///
    /// Appended at the end (fields are never reordered).
    pub hero_plane: Option<crate::welcome::HeroPlaneCache>,
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
                hover_fresh: false,
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
            timers: EnumMap::default(),
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
            settings: crate::settings::SettingsMenuState::closed(),
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
            // The picker graphs x at the URL default; each `build()`
            // re-seeds the param and the shown function from the real page
            // query (`selector::init_func_from_url`).
            func_param: crate::url::QueryParam::new(&crate::functions::FUNC_PARAM, ""),
            shown: crate::plot::Shown::new(crate::functions::base(
                crate::functions::FUNC_PARAM.default,
            )),
            plot_frame_at: None,
            plot_hold: false,
            // Nothing touched; the transform params at the URL defaults; no
            // uncommitted text. Each `build()` re-seeds the params from the
            // real page query (`transform_section::init_params_from_url`).
            state: crate::state::HighlightState::touched_out(),
            // The transform params at the URL defaults; no uncommitted text.
            // Each `build()` re-seeds the params from the real page query
            // (`transform_section::init_params_from_url`).
            a_param: crate::url::QueryParam::new(&crate::functions::A_PARAM, ""),
            b_param: crate::url::QueryParam::new(&crate::functions::B_PARAM, ""),
            h_param: crate::url::QueryParam::new(&crate::functions::H_PARAM, ""),
            k_param: crate::url::QueryParam::new(&crate::functions::K_PARAM, ""),
            p_param: crate::url::QueryParam::new(&crate::functions::P_PARAM, ""),
            drafts: EnumMap::default(),
            // At rest: no scrub holds a box, no rest tip is armed or showing.
            scrub: None,
            scrub_hold: None,
            scrub_rest: None,
            scrub_resting: None,
            overlay_tip_since: None,
            flip: crate::flip::FlipState::new(),
            hovered: None,
            dragging: false,
            grab: [0.0, 0.0],
            dragged: None,
            moved_by: crate::moved_by::MovedBy::new(),
            visible_range: None,
            x_field: crate::x_field::XField::new(),
            slider_hold: None,
            slider_edit: false,
            backdrop: None,
            welcome: crate::welcome_step::WelcomeState::fresh(),
            hero_plane: None,
        }
    }
}
