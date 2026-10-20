// The Function Viewer shell: one SDL3 window drawing one cleared frame,
// natively and on the web.
//
// SDL3's main callbacks drive both builds: `#[app_impl]` assigns
// `app_init/iterate/event/quit` to SDL_AppInit/Iterate/Event/Quit, so SDL owns
// the loop natively and the browser does on the web. The shell state
// (`app::App`, built by `App::bare` and filled by `app_init` below) and one
// module per shell concern hang off this root. There is no `fn main`:
// `sdl3-main` generates it. A scripted run draws hidden (this login has no
// display, and a visible swap would wait on a vsync that never comes);
// an interactive run shows the window.
//
// Surface: the kept utilities (draw helpers, ui scope, scroll, regions, the
// icon painter) carry no page yet; the page milestones wire them.
#![allow(dead_code)]

pub mod app;
// `loop` is a keyword and cannot be a module name, so the `loop.rs` file
// is declared under this name.
#[path = "loop.rs"]
pub mod app_loop;
pub mod camera;
pub mod draw;
pub mod fixture;
pub mod flex;
pub mod grid;
#[cfg(not(target_os = "emscripten"))]
pub mod harness;
pub mod icons;
pub mod input;
#[cfg(not(target_os = "emscripten"))]
pub mod live;
pub mod page;
pub mod plane_input;
#[cfg(target_os = "macos")]
pub mod platform_darwin;
#[cfg(not(target_os = "emscripten"))]
pub mod platform_native;
#[cfg(unix)]
pub mod platform_unix;
#[cfg(target_os = "emscripten")]
pub mod platform_web;
#[cfg(windows)]
pub mod platform_windows;
pub mod prefs;
pub mod scroll;
pub mod svgpath;
pub mod theme_state;
pub mod top_bar;
pub mod transition;
pub mod typography;
pub mod ui;
pub mod url;

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use glow::HasContext as _;
use sdl3::event::Event;
use sdl3_main::{AppResult, AppResultWithState, MainThreadData, app_impl};

// t0a's window. The page's own size is WINDOW_SIZE below; the shell clears
// its frame at the script's size until the page is wired.
const WINDOW_TITLE: &str = "Function Viewer";

/// The page's window: 1440x900 logical, at least 420x560. These replace the
/// triangle's size above once the page is wired.
pub const WINDOW_SIZE: [i32; 2] = [1440, 900];
pub const WINDOW_MIN_SIZE: [i32; 2] = [420, 560];

/// Whether debug-level logs print: set from `--trace` in `app_init`.
///
/// Ports Odin's `logger.lowest_level = .Debug` there: debug (`log.debugf`
/// ports, via `crate::trace()`) prints only under `--trace`; info and above
/// always print, info to stdout and errors to stderr (on the web: console
/// log vs console error, which is what keeps scripted runs problem-free).
pub static TRACE: AtomicBool = AtomicBool::new(false);

/// Debug-level logging, for `log.debugf` ports: silent unless `--trace`.
pub fn trace() -> bool {
    TRACE.load(Ordering::Relaxed)
}

/// What the command line asked for: scripted runs, the live driver, and the
/// appearance overrides. The harness milestone owns the flags' parsing.
#[derive(Default)]
pub struct Options {
    pub script: Option<mv_script::Script>,
    /// Where the script's shots go.
    pub out: String,
    /// After the script, its last state redrawn this many times, each timed.
    pub time: i64,
    pub window: bool,
    pub live: bool,
    pub pace: Option<app_loop::Pace>,
    pub prefs: String,
    pub trace: bool,
    /// The page query at start (`App.url`): `--query` natively, the hash on
    /// the web. The harness writes it to `views.json` whenever it changes.
    pub query: String,
    pub slug: Option<mv_theme::Slug>,
    pub mode: Option<mv_theme::Mode>,
    pub space: Option<mv_theme::Space>,
    /// Panic in the first frame instead of drawing it: the sweep's crash
    /// proof (plan 3.4). Set by `--panic-at-frame` (the web's way in) or
    /// `MV_PANIC_AT_FRAME=1` (the native way); kept in the app, as t0a had it.
    pub panic_at_frame: bool,
}

/// Reads the flags into `Options`: the harness's view of the app (README).
/// `--script` runs a script (tools/states) on its own clock, writing its
/// shots to `--out`; `--png` writes one frame at `--size` and `--density`;
/// with `--time N` either then redraws its last state N times, timing each
/// redraw. `--live` runs a script through SDL's own loop on the real clock.
/// `--pace 60` paces frames at 60 Hz without vsync; `--prefs DIR` keeps the
/// prefs in DIR; `--trace` logs each fact and timer. `--theme SLUG` shows
/// that theme and `--mode` sets the stored mode; `--space srgb|p3` sets the
/// colour space the frames are drawn in (a script says all three itself).
/// `--query STRING` sets the page query at start (`App.url`).
/// Failures print `[ERROR]` (which the harness counts as a problem) and
/// return `false`.
fn parse_options(args: &[String]) -> (Options, bool) {
    let mut options = Options::default();
    let mut script_path = String::new();
    let mut live_path = String::new();
    let mut png = String::new();
    let mut size: Option<String> = None;
    let mut density: Option<String> = None;
    let mut i = 1;
    while i < args.len() {
        let arg = args[i].as_str();
        match arg {
            "--window" => {
                options.window = true;
                i += 1;
                continue;
            }
            "--trace" => {
                options.trace = true;
                i += 1;
                continue;
            }
            "--panic-at-frame" => {
                options.panic_at_frame = true;
                i += 1;
                continue;
            }
            _ => {}
        }
        if i + 1 == args.len() {
            eprintln!("[ERROR] --- [{}:{}] {arg} takes a value", file!(), line!());
            return (Options::default(), false);
        }
        i += 1;
        let value = args[i].as_str();
        match arg {
            "--script" => script_path = value.to_string(),
            "--live" => live_path = value.to_string(),
            "--out" => options.out = value.to_string(),
            "--png" => png = value.to_string(),
            "--size" => size = Some(value.to_string()),
            "--density" => density = Some(value.to_string()),
            "--time" => match value.parse::<i64>() {
                Ok(n) if n > 0 => options.time = n,
                _ => {
                    eprintln!(
                        "[ERROR] --- [{}:{}] --time takes a number of redraws, not {value:?}",
                        file!(),
                        line!()
                    );
                    return (Options::default(), false);
                }
            },
            "--prefs" => options.prefs = value.to_string(),
            "--query" => options.query = value.to_string(),
            "--theme" => match one_of(&[value], &mv_theme::SLUG_WORDS, arg) {
                Ok(slug) => options.slug = Some(slug),
                Err(message) => {
                    eprintln!(
                        "[ERROR] --- [{}:{}] {message}, not {value:?}",
                        file!(),
                        line!()
                    );
                    return (Options::default(), false);
                }
            },
            "--mode" => match one_of(&[value], &mv_theme::MODE_WORDS, arg) {
                Ok(mode) => options.mode = Some(mode),
                Err(message) => {
                    eprintln!(
                        "[ERROR] --- [{}:{}] {message}, not {value:?}",
                        file!(),
                        line!()
                    );
                    return (Options::default(), false);
                }
            },
            "--space" => match one_of(&[value], &mv_theme::SPACE_WORDS, arg) {
                Ok(space) => options.space = Some(space),
                Err(message) => {
                    eprintln!(
                        "[ERROR] --- [{}:{}] {message}, not {value:?}",
                        file!(),
                        line!()
                    );
                    return (Options::default(), false);
                }
            },
            "--pace" => match value {
                "display" => options.pace = Some(app_loop::Pace::Display),
                "60" => options.pace = Some(app_loop::Pace::Fixed60),
                _ => {
                    eprintln!(
                        "[ERROR] --- [{}:{}] --pace takes display or 60, not {value:?}",
                        file!(),
                        line!()
                    );
                    return (Options::default(), false);
                }
            },
            _ => {
                eprintln!(
                    "[ERROR] --- [{}:{}] unknown argument {arg:?}",
                    file!(),
                    line!()
                );
                return (Options::default(), false);
            }
        }
        i += 1;
    }

    let runs = [
        !script_path.is_empty(),
        !live_path.is_empty(),
        !png.is_empty(),
    ]
    .iter()
    .filter(|&&run| run)
    .count();
    if runs > 1 {
        eprintln!(
            "[ERROR] --- [{}:{}] one of --script, --live and --png",
            file!(),
            line!()
        );
        return (Options::default(), false);
    }
    if !script_path.is_empty() || !live_path.is_empty() {
        if size.is_some()
            || density.is_some()
            || options.slug.is_some()
            || options.mode.is_some()
            || options.space.is_some()
        {
            eprintln!(
                "[ERROR] --- [{}:{}] a script sets its own size, density, theme, mode and space",
                file!(),
                line!()
            );
            return (Options::default(), false);
        }
        if !script_path.is_empty() && options.out.is_empty() {
            eprintln!(
                "[ERROR] --- [{}:{}] --script needs --out DIR for its shots",
                file!(),
                line!()
            );
            return (Options::default(), false);
        }
        let path = if !script_path.is_empty() {
            &script_path
        } else {
            &live_path
        };
        let (data, ok) = platform_read_file(path);
        if !ok {
            return (Options::default(), false);
        }
        let Ok(text) = String::from_utf8(data) else {
            eprintln!(
                "[ERROR] --- [{}:{}] could not read {path}",
                file!(),
                line!()
            );
            return (Options::default(), false);
        };
        match mv_script::parse(&text) {
            (run, None) => options.script = Some(run),
            (_, Some(error)) => {
                eprintln!(
                    "[ERROR] --- [{}:{}] {path}:{}: {}",
                    file!(),
                    line!(),
                    error.line,
                    error.message
                );
                return (Options::default(), false);
            }
        }
        options.live = !live_path.is_empty();
    } else if !png.is_empty() {
        let mut start = mv_script::DEFAULT_START;
        if let Some(size) = size.as_deref() {
            match size.split_once('x') {
                Some((w, h)) => match (w.parse::<i64>(), h.parse::<i64>()) {
                    (Ok(width), Ok(height)) if width > 0 && height > 0 => {
                        start.size = [width, height];
                    }
                    _ => {
                        eprintln!(
                            "[ERROR] --- [{}:{}] --size takes WIDTHxHEIGHT, not {size:?}",
                            file!(),
                            line!()
                        );
                        return (Options::default(), false);
                    }
                },
                None => {
                    eprintln!(
                        "[ERROR] --- [{}:{}] --size takes WIDTHxHEIGHT, not {size:?}",
                        file!(),
                        line!()
                    );
                    return (Options::default(), false);
                }
            }
        }
        if let Some(density) = density.as_deref() {
            match density.parse::<f32>() {
                Ok(parsed) if parsed > 0.0 => start.density = parsed,
                _ => {
                    eprintln!(
                        "[ERROR] --- [{}:{}] --density takes a number above 0, not {density:?}",
                        file!(),
                        line!()
                    );
                    return (Options::default(), false);
                }
            }
        }
        let prefs = appearance(
            mv_theme::Prefs {
                mode: start.mode,
                dark: start.dark_theme,
                light: start.light_theme,
            },
            &options,
            start.scheme,
        );
        start.mode = prefs.mode;
        start.dark_theme = prefs.dark;
        start.light_theme = prefs.light;
        start.space = options.space.unwrap_or_else(platform_default_space);
        // One shot named for the file, in its directory, at rest: once the
        // first frames have run their course.
        let (dir, file) = match png.rfind('/') {
            Some(slash) => (png[..slash].to_string(), &png[slash + 1..]),
            None => (".".to_string(), png.as_str()),
        };
        options.out = dir;
        options.script = Some(mv_script::Script {
            start,
            actions: vec![
                mv_script::Action {
                    line: 0,
                    verb: mv_script::Verb::Wait(mv_script::Wait { ms: 2000 }),
                    surface: mv_script::Surface::All,
                },
                mv_script::Action {
                    line: 0,
                    verb: mv_script::Verb::Shot(mv_script::Shot {
                        name: file.strip_suffix(".png").unwrap_or(file).to_string(),
                    }),
                    surface: mv_script::Surface::All,
                },
            ],
        });
    } else if size.is_some() || density.is_some() {
        eprintln!(
            "[ERROR] --- [{}:{}] --size and --density go with --png",
            file!(),
            line!()
        );
        return (Options::default(), false);
    }
    if options.time > 0 && (options.script.is_none() || options.live) {
        eprintln!(
            "[ERROR] --- [{}:{}] --time goes with --script or --png (with or without --window), not --live",
            file!(),
            line!()
        );
        return (Options::default(), false);
    }
    (options, true)
}

/// The value whose word `fields` holds, as the only field: the script
/// package's `one_of` for the command line, whose words live here. `Err`
/// holds the failure's message.
fn one_of<E>(fields: &[&str], words: &enum_map::EnumMap<E, &str>, what: &str) -> Result<E, String>
where
    E: for<'a> enum_map::EnumArray<&'a str>,
{
    if fields.len() == 1 {
        for (e, word) in words {
            if fields[0] == *word {
                return Ok(e);
            }
        }
    }
    let mut list = String::new();
    for (_, word) in words {
        if !list.is_empty() {
            list.push_str(", ");
        }
        list.push_str(word);
    }
    Err(format!("{what} takes one of {list}"))
}

/// Applies the `--theme`/`--mode` overrides to the stored prefs.
fn appearance(
    prefs: mv_theme::Prefs,
    options: &Options,
    system: mv_theme::Scheme,
) -> mv_theme::Prefs {
    // The prefs with --theme and --mode applied: the mode as given, and the
    // theme picked for its scheme and shown (showing_theme) unless a mode
    // was given too.
    let mut out = prefs;
    if let Some(slug) = options.slug {
        out = if options.mode.is_some() {
            mv_theme::with_theme(out, slug)
        } else {
            mv_theme::showing_theme(out, slug, system)
        };
    }
    if let Some(mode) = options.mode {
        out.mode = mode;
    }
    out
}

#[cfg(not(target_os = "emscripten"))]
use crate::platform_native::{
    platform_args, platform_context, platform_default_pace, platform_default_space,
    platform_gl_attributes, platform_hints, platform_init, platform_load_prefs,
    platform_loop_counts, platform_prefs_store, platform_quit, platform_read_file,
    platform_reduced_motion, platform_window_made,
};
#[cfg(target_os = "emscripten")]
use crate::platform_web::{
    mv_panic_at_frame, mv_trace, platform_args, platform_context, platform_default_pace,
    platform_default_space, platform_gl_attributes, platform_hints, platform_init,
    platform_load_prefs, platform_loop_counts, platform_prefs_store, platform_quit,
    platform_read_file, platform_read_hash, platform_reduced_motion, platform_window_made,
};

#[cfg(test)]
mod options_tests {
    use super::*;

    fn args(words: &[&str]) -> Vec<String> {
        words.iter().map(|w| w.to_string()).collect()
    }

    #[test]
    fn empty_means_an_interactive_run() {
        let (options, ok) = parse_options(&args(&["function-viewer"]));
        assert!(ok);
        assert!(options.script.is_none());
        assert!(!options.window && !options.live && !options.trace);
        assert_eq!(options.time, 0);
        assert!(options.pace.is_none() && options.slug.is_none());
    }

    #[test]
    fn window_and_trace_are_flags() {
        let (options, ok) = parse_options(&args(&["function-viewer", "--window", "--trace"]));
        assert!(ok);
        assert!(options.window && options.trace);
    }

    #[test]
    fn panic_at_frame_is_a_flag() {
        let (options, ok) = parse_options(&args(&["function-viewer", "--panic-at-frame"]));
        assert!(ok);
        assert!(options.panic_at_frame);
        let (options, ok) = parse_options(&args(&["function-viewer"]));
        assert!(ok);
        assert!(!options.panic_at_frame);
    }

    #[test]
    fn script_flag_loads_the_file() {
        // The app-harness blocker, proved headless: `--script` reads and
        // parses the file (platform_read_file + mv_script::parse are both
        // real now), so the binary takes `--script` once `app_init` calls
        // this parser instead of t0a's.
        let (options, ok) = parse_options(&args(&[
            "function-viewer",
            "--script",
            "../../tools/states/default.act",
            "--out",
            "/tmp/i0-script-out",
        ]));
        assert!(ok);
        let script = options.script.expect("a script");
        assert_eq!(script.start.size, [1440, 900]);
        assert_eq!(options.out, "/tmp/i0-script-out");
    }

    #[test]
    fn a_flag_without_its_value_fails() {
        let (_, ok) = parse_options(&args(&["function-viewer", "--out"]));
        assert!(!ok);
    }

    #[test]
    fn time_takes_a_count() {
        let (options, ok) = parse_options(&args(&[
            "function-viewer",
            "--png",
            "a.png",
            "--space",
            "srgb",
            "--time",
            "10",
        ]));
        assert!(ok);
        assert_eq!(options.time, 10);
        for bad in ["0", "-3", "many"] {
            let (_, ok) = parse_options(&args(&["function-viewer", "--time", bad]));
            assert!(!ok, "for {bad}");
        }
    }

    #[test]
    fn pace_takes_two_words() {
        let (display, ok) = parse_options(&args(&["function-viewer", "--pace", "display"]));
        assert!(ok);
        assert_eq!(display.pace, Some(app_loop::Pace::Display));
        let (sixty, ok) = parse_options(&args(&["function-viewer", "--pace", "60"]));
        assert!(ok);
        assert_eq!(sixty.pace, Some(app_loop::Pace::Fixed60));
        let (_, ok) = parse_options(&args(&["function-viewer", "--pace", "fast"]));
        assert!(!ok);
    }

    #[test]
    fn appearance_words_resolve() {
        let (options, ok) = parse_options(&args(&[
            "function-viewer",
            "--theme",
            "arctic-ice",
            "--mode",
            "light",
            "--space",
            "p3",
        ]));
        assert!(ok);
        assert_eq!(options.slug, Some(mv_theme::Slug::ArcticIce));
        assert_eq!(options.mode, Some(mv_theme::Mode::Light));
        assert_eq!(options.space, Some(mv_theme::Space::DisplayP3));
        for bad in [
            ["--theme", "lorange"],
            ["--mode", "sepia"],
            ["--space", "cmyk"],
        ] {
            let argv = [&["function-viewer"], &bad[..]].concat();
            let (_, ok) = parse_options(&args(&argv));
            assert!(!ok, "for {bad:?}");
        }
    }

    #[test]
    fn one_run_at_most() {
        let (_, ok) = parse_options(&args(&[
            "function-viewer",
            "--script",
            "a.act",
            "--png",
            "a.png",
            "--out",
            "d",
        ]));
        assert!(!ok);
        let (_, ok) = parse_options(&args(&[
            "function-viewer",
            "--live",
            "a.act",
            "--script",
            "a.act",
            "--out",
            "d",
        ]));
        assert!(!ok);
    }

    #[test]
    fn scripts_bring_their_own_page() {
        let (_, ok) = parse_options(&args(&[
            "function-viewer",
            "--script",
            "a.act",
            "--out",
            "d",
            "--theme",
            "arctic-ice",
        ]));
        assert!(!ok);
        let (_, ok) = parse_options(&args(&["function-viewer", "--script", "a.act"]));
        assert!(!ok);
    }

    #[test]
    fn size_and_density_go_with_png() {
        let (_, ok) = parse_options(&args(&["function-viewer", "--size", "800x600"]));
        assert!(!ok);
    }

    #[test]
    fn png_synthesises_its_wait_and_shot() {
        let (options, ok) = parse_options(&args(&[
            "function-viewer",
            "--png",
            "/tmp/shots/page.png",
            "--space",
            "p3",
            "--size",
            "800x600",
        ]));
        assert!(ok);
        assert_eq!(options.out, "/tmp/shots");
        let script = options.script.expect("a script");
        assert_eq!(script.start.size, [800, 600]);
        assert_eq!(script.start.space, mv_theme::Space::DisplayP3);
        assert_eq!(script.actions.len(), 2);
        match &script.actions[0].verb {
            mv_script::Verb::Wait(wait) => assert_eq!(wait.ms, 2000),
            verb => panic!("a wait first, not {verb:?}"),
        }
        match &script.actions[1].verb {
            mv_script::Verb::Shot(shot) => assert_eq!(shot.name, "page"),
            verb => panic!("a shot second, not {verb:?}"),
        }
    }

    #[test]
    fn png_without_a_dir_shoots_here() {
        let (options, ok) = parse_options(&args(&[
            "function-viewer",
            "--png",
            "page.png",
            "--space",
            "srgb",
        ]));
        assert!(ok);
        assert_eq!(options.out, ".");
        match &options.script.expect("a script").actions[1].verb {
            mv_script::Verb::Shot(shot) => assert_eq!(shot.name, "page"),
            verb => panic!("a shot second, not {verb:?}"),
        }
    }

    #[test]
    fn png_rejects_bad_geometry() {
        for bad in [
            ["--size", "800"],
            ["--size", "800x"],
            ["--size", "0x600"],
            ["--size", "800x600x2"],
            ["--density", "0"],
            ["--density", "wide"],
        ] {
            let argv = [
                &["function-viewer", "--png", "a.png", "--space", "srgb"][..],
                &bad[..],
            ]
            .concat();
            let (_, ok) = parse_options(&args(&argv));
            assert!(!ok, "for {bad:?}");
        }
    }

    #[test]
    fn query_sets_the_page_query() {
        let (options, ok) = parse_options(&args(&[
            "function-viewer",
            "--png",
            "a.png",
            "--space",
            "srgb",
            "--query",
            "center=1,2&scale=2",
        ]));
        assert!(ok);
        assert_eq!(options.query, "center=1,2&scale=2");
        let (options, ok) = parse_options(&args(&["function-viewer"]));
        assert!(ok);
        assert!(options.query.is_empty());
    }

    #[test]
    fn unknown_arguments_fail() {
        let (_, ok) = parse_options(&args(&["function-viewer", "--zoom", "2"]));
        assert!(!ok);
    }

    #[test]
    fn time_goes_with_a_scripted_run() {
        // --time alone names no run; --time with --png names one. (--time
        // with --live needs the file read, which is app-platform's.)
        let (_, ok) = parse_options(&args(&["function-viewer", "--time", "3"]));
        assert!(!ok);
        let (_, ok) = parse_options(&args(&[
            "function-viewer",
            "--png",
            "a.png",
            "--space",
            "srgb",
            "--time",
            "3",
        ]));
        assert!(ok);
    }

    #[test]
    fn theme_applies_without_a_mode() {
        let stored = mv_theme::Prefs {
            mode: mv_theme::Mode::System,
            dark: mv_theme::Slug::ArcticIce,
            light: mv_theme::Slug::CleanTeal,
        };
        let options = Options {
            slug: Some(mv_theme::Slug::TronCyan),
            ..Options::default()
        };
        let out = appearance(stored, &options, mv_theme::Scheme::Dark);
        assert_eq!(out.dark, mv_theme::Slug::TronCyan);
        assert_eq!(out.mode, mv_theme::Mode::System);
    }

    #[test]
    fn mode_pins_what_theme_would_change() {
        let stored = mv_theme::Prefs {
            mode: mv_theme::Mode::System,
            dark: mv_theme::Slug::ArcticIce,
            light: mv_theme::Slug::CleanTeal,
        };
        let options = Options {
            slug: Some(mv_theme::Slug::TronCyan),
            mode: Some(mv_theme::Mode::Dark),
            ..Options::default()
        };
        let out = appearance(stored, &options, mv_theme::Scheme::Dark);
        assert_eq!(out.dark, mv_theme::Slug::TronCyan);
        assert_eq!(out.mode, mv_theme::Mode::Dark);
    }
}

// The running app: SDL's objects (main-thread-only, held by `MainThreadData`
// with a runtime thread check) and the page state they serve.
//
// Ported from `main.odin:261` (`app_init`, `app_iterate`, `app_event`,
// `app_quit`): the window hidden under a script, the colour space tagged
// before the GL context, GL through glow, NanoVG, fonts, Clay, themes
// resolved in the space, model, plane, prefs, motion, then the harness (or
// the loop, interactively). t0a's triangle is gone; every frame draws the
// page.
struct MainData {
    // Dropped last: SDL quits after the window and the GL context are gone.
    // (Construction fills these out of order; drop follows declaration.)
    app: crate::app::App,
    _video: sdl3::VideoSubsystem,
    _sdl: sdl3::Sdl,
    panic_at_frame: bool,
    frames: u64,
}

struct Main {
    main: MainThreadData<MainData>,
}

/// A scripted run the harness executes inside `app_init`, natively only: the
/// script, where its shots go, and the run's shape. `None` runs
/// interactively (or under Playwright, on the web).
#[cfg(not(target_os = "emscripten"))]
struct ScriptedRun {
    script: mv_script::Script,
    out: String,
    window: bool,
    time: i64,
    live: bool,
}

/// Whether the app asked to panic in its first frame (the sweep's crash
/// proof): read through the main lock, as `app_init`'s harness branch does.
#[cfg(not(target_os = "emscripten"))]
fn data_panics(main: &std::sync::Mutex<Main>) -> bool {
    main.lock()
        .expect("app_init: panic lock")
        .main
        .assert_get_mut()
        .panic_at_frame
}

/// GL through SDL's loader: one `glow::Context` per call (glow's context is
/// not `Clone`, so the app's direct calls and NanoVG's `finish`/`read_pixels`
/// each load their own handle and share the one current context).
///
/// SAFETY: the context is current on this thread, and every loader call runs
/// before any other GL use. (t0a's loader at `main.rs:778`.)
fn load_gl(video: &sdl3::VideoSubsystem) -> glow::Context {
    unsafe {
        glow::Context::from_loader_function(|name| {
            video
                .gl_get_proc_address(name)
                .map_or(std::ptr::null(), |f| f as *const std::ffi::c_void)
        })
    }
}

impl Main {
    /// Builds the window, the GL context and the real `App`: the window and
    /// its colour space, GL through glow, NanoVG, the themes resolved in the
    /// space, fonts, Clay's measure (after the last move, in `app_init`),
    /// prefs, then the harness (or the loop, interactively). Returns the app
    /// with what to do with it: a scripted run the caller executes (natively
    /// only), else an interactive app.
    #[cfg(not(target_os = "emscripten"))]
    fn build() -> Result<(Self, Option<ScriptedRun>), String> {
        platform_context();
        let args = platform_args();
        let (options, ok) = parse_options(&args);
        if !ok {
            return Err("bad options (see [ERROR] above)".to_string());
        }
        TRACE.store(options.trace, Ordering::SeqCst);
        let panic_at_frame =
            options.panic_at_frame || std::env::var("MV_PANIC_AT_FRAME").as_deref() == Ok("1");
        platform_hints();
        let sdl = sdl3::init().map_err(|e| format!("SDL_Init: {e}"))?;
        let video = sdl.video().map_err(|e| format!("SDL video: {e}"))?;
        // SAFETY: reserves one SDL user-event number; the number comes back.
        let user_event_type = unsafe { sdl3::sys::events::SDL_RegisterEvents(1) };
        crate::app_loop::USER_EVENT_TYPE
            .store(user_event_type, std::sync::atomic::Ordering::SeqCst);
        platform_gl_attributes();
        // The platform's raw attributes are the whole story (the Odin order):
        // t0a's safe `gl_attr` block set the same GL 3.3 core values and is
        // gone now that frames prove the platform path on both targets
        // (i1-frame: native "GL 4.1 Metal", identical frames in three engines).

        let scripted = options.script.is_some();
        let (width, height) = match options.script.as_ref() {
            Some(run) => (
                run.start.size[0].max(1) as u32,
                run.start.size[1].max(1) as u32,
            ),
            None => (WINDOW_SIZE[0] as u32, WINDOW_SIZE[1] as u32),
        };
        let mut builder = video.window(WINDOW_TITLE, width, height);
        builder.opengl();
        builder.high_pixel_density();
        builder.resizable();
        if scripted && !options.window && !options.live {
            builder.hidden();
        }
        let mut window = builder
            .build()
            .map_err(|e| format!("SDL_CreateWindow: {e}"))?;
        let _ = window.set_minimum_size(WINDOW_MIN_SIZE[0] as u32, WINDOW_MIN_SIZE[1] as u32);
        // The colour space, tagged before the GL context makes the surface
        // that keeps it: the script's (sRGB unless it says p3), else --space
        // or the platform's.
        let space = match options.script.as_ref() {
            Some(run) => run.start.space,
            None => options.space.unwrap_or_else(platform_default_space),
        };
        let space = platform_window_made(&window, space);
        let gl_context = window
            .gl_create_context()
            .map_err(|e| format!("SDL_GL_CreateContext: {e}"))?;
        window
            .gl_make_current(&gl_context)
            .map_err(|e| format!("SDL_GL_MakeCurrent: {e}"))?;

        let gl = load_gl(&video);
        println!(
            "math-viz2-rust: GL {}",
            // SAFETY: `VERSION` returns a live string while the context stands.
            unsafe { gl.get_parameter_string(glow::VERSION) }
        );

        let flags =
            mv_nvg::CreateFlag::ANTI_ALIAS as i32 | mv_nvg::CreateFlag::STENCIL_STROKES as i32;
        let mut vg = mv_nvg::Context::gl3(flags);
        // NanoVG's `finish`/`read_pixels` run through this handle (they
        // expect `set_gl` first); the app's direct calls use `gl` above.
        vg.set_gl(load_gl(&video));
        let mut app = crate::app::App::bare(window, gl_context, gl, vg);
        app.space = space;
        app.r#loop.pace = options.pace.unwrap_or_else(platform_default_pace);
        platform_init(&mut app);

        for slug in [
            mv_theme::Slug::ArcticIce,
            mv_theme::Slug::TronCyan,
            mv_theme::Slug::MidnightViolet,
            mv_theme::Slug::CleanTeal,
            mv_theme::Slug::EducationalWarm,
            mv_theme::Slug::SageEditorial,
        ] {
            app.themes[slug] = mv_theme::resolve(slug, app.space);
        }
        if !mv_text::init(&mut app.fonts) {
            return Err("text.init failed".to_string());
        }
        mv_text::load_into(&mut app.fonts, &mut app.vg);
        // No `init_clay` here: Clay's measure closure keeps a `*mut App`,
        // and `app` moves once more into its box below. `app_init` runs it
        // after the last move.
        crate::icons::set_window_icon(&mut app);

        // What the platform says, or under a script what the script says.
        app.system_scheme = if crate::input::system_dark() {
            mv_theme::Scheme::Dark
        } else {
            mv_theme::Scheme::Light
        };
        app.reduced_motion = platform_reduced_motion();
        if scripted && !options.live {
            let run = options.script.as_ref().expect("scripted");
            app.system_scheme = run.start.scheme;
            app.reduced_motion = run.start.reduced_motion;
            app.prefs = mv_theme::Prefs {
                mode: run.start.mode,
                dark: run.start.dark_theme,
                light: run.start.light_theme,
            };
            app.prefs_store = if options.prefs.is_empty() {
                String::new()
            } else {
                platform_prefs_store(&options.prefs)
            };
        } else {
            app.prefs_store = platform_prefs_store(&options.prefs);
            crate::prefs::load_prefs(&mut app);
            app.prefs = appearance(app.prefs, &options, app.system_scheme);
        }
        // The theme state takes the stored triple and the platform's scheme.
        crate::prefs::take_prefs_themes(&mut app);
        // The page query at start: `--query` (empty without it).
        app.url = crate::url::Url::from_query(&options.query);
        // The camera and its params start from that query (h2-input).
        crate::plane_input::init_camera_from_url(&mut app);
        println!(
            "math-viz2-rust: system {}, motion {}; showing {}",
            mv_theme::SCHEME_WORDS[app.system_scheme],
            if app.reduced_motion {
                "reduced"
            } else {
                "full"
            },
            mv_theme::SLUG_WORDS[mv_theme::shown(app.prefs, app.system_scheme)],
        );
        app.input.pointer = [-1.0, -1.0];
        let first = crate::input::Input::Window(crate::input::window_event(app.window()));
        crate::input::handle_input(&mut app, first, 0);

        let run = if scripted {
            let run = options.script.expect("scripted");
            Some(ScriptedRun {
                script: run,
                out: options.out,
                window: options.window,
                time: options.time,
                live: options.live,
            })
        } else {
            None
        };
        Ok((
            Self {
                main: MainThreadData::assert_new(MainData {
                    app,
                    _video: video,
                    _sdl: sdl,
                    panic_at_frame,
                    frames: 0,
                }),
            },
            run,
        ))
    }

    /// Builds the window, the GL context and the real `App`, on the web:
    /// Playwright drives the scripted runs from outside, so there is no
    /// harness branch here (`platform_init` installs `mvIterate`).
    #[cfg(target_os = "emscripten")]
    fn build() -> Result<(Self, ()), String> {
        platform_context();
        let args = platform_args();
        let (options, ok) = parse_options(&args);
        if !ok {
            return Err("bad options (see [ERROR] above)".to_string());
        }
        // The shell's `?trace` (library.js's `mv_trace`), as the Odin app's
        // `platform_context` reads it: the web build takes no arguments, so
        // `--trace` never arrives here (platform_args is empty).
        // SAFETY: a boolean query with no arguments.
        let trace = options.trace || unsafe { mv_trace() };
        TRACE.store(trace, Ordering::SeqCst);
        // The shell's `?panic-at-frame` (library.js's `mv_panic_at_frame`),
        // as above: the web build takes no arguments, and the environment
        // never carries the variable there.
        // SAFETY: a boolean query with no arguments.
        let panic_at_frame = options.panic_at_frame
            || unsafe { mv_panic_at_frame() }
            || std::env::var("MV_PANIC_AT_FRAME").as_deref() == Ok("1");
        platform_hints();
        let sdl = sdl3::init().map_err(|e| format!("SDL_Init: {e}"))?;
        let video = sdl.video().map_err(|e| format!("SDL video: {e}"))?;
        // SAFETY: reserves one SDL user-event number; the number comes back.
        let user_event_type = unsafe { sdl3::sys::events::SDL_RegisterEvents(1) };
        crate::app_loop::USER_EVENT_TYPE
            .store(user_event_type, std::sync::atomic::Ordering::SeqCst);
        platform_gl_attributes();
        // As on the native side: the platform's raw attributes alone (Odin's
        // GLES 3.0 call), t0a's duplicate gone with the frames proving it.

        let mut builder = video.window(WINDOW_TITLE, WINDOW_SIZE[0] as u32, WINDOW_SIZE[1] as u32);
        builder.opengl();
        builder.high_pixel_density();
        builder.resizable();
        builder.set_window_flags(
            builder.window_flags() | sdl3::video::WindowFlags::FILL_DOCUMENT.as_u32(),
        );
        let mut window = builder
            .build()
            .map_err(|e| format!("SDL_CreateWindow: {e}"))?;
        let _ = window.set_minimum_size(WINDOW_MIN_SIZE[0] as u32, WINDOW_MIN_SIZE[1] as u32);
        let space = options.space.unwrap_or_else(platform_default_space);
        let space = platform_window_made(&window, space);
        let gl_context = window
            .gl_create_context()
            .map_err(|e| format!("SDL_GL_CreateContext: {e}"))?;
        window
            .gl_make_current(&gl_context)
            .map_err(|e| format!("SDL_GL_MakeCurrent: {e}"))?;

        let gl = load_gl(&video);
        println!(
            "math-viz2-rust: GL {}",
            // SAFETY: `VERSION` returns a live string while the context stands.
            unsafe { gl.get_parameter_string(glow::VERSION) }
        );

        let flags =
            mv_nvg::CreateFlag::ANTI_ALIAS as i32 | mv_nvg::CreateFlag::STENCIL_STROKES as i32;
        let mut vg = mv_nvg::Context::gles3(flags);
        // As on the native side: NanoVG's `finish`/`read_pixels` handle.
        vg.set_gl(load_gl(&video));
        let mut app = crate::app::App::bare(window, gl_context, gl, vg);
        app.space = space;
        app.r#loop.pace = options.pace.unwrap_or_else(platform_default_pace);
        platform_init(&mut app);

        for slug in [
            mv_theme::Slug::ArcticIce,
            mv_theme::Slug::TronCyan,
            mv_theme::Slug::MidnightViolet,
            mv_theme::Slug::CleanTeal,
            mv_theme::Slug::EducationalWarm,
            mv_theme::Slug::SageEditorial,
        ] {
            app.themes[slug] = mv_theme::resolve(slug, app.space);
        }
        if !mv_text::init(&mut app.fonts) {
            return Err("text.init failed".to_string());
        }
        mv_text::load_into(&mut app.fonts, &mut app.vg);
        // No `init_clay` here: Clay's measure closure keeps a `*mut App`,
        // and `app` moves once more into its box below. `app_init` runs it
        // after the last move.
        crate::icons::set_window_icon(&mut app);

        // What the platform says (a script's overrides arrive per model line
        // on the web; see `mv_model_line`).
        app.system_scheme = if crate::input::system_dark() {
            mv_theme::Scheme::Dark
        } else {
            mv_theme::Scheme::Light
        };
        app.reduced_motion = platform_reduced_motion();
        app.prefs_store = platform_prefs_store(&options.prefs);
        crate::prefs::load_prefs(&mut app);
        app.prefs = appearance(app.prefs, &options, app.system_scheme);
        // The page query at start: the hash's query part, if the page was
        // loaded with one (`#function-viewer?center=1,2&scale=2`).
        app.url = crate::url::Url::new(&platform_read_hash());
        // The camera and its params start from that query (h2-input).
        crate::plane_input::init_camera_from_url(&mut app);
        println!(
            "math-viz2-rust: system {}, motion {}; showing {}",
            mv_theme::SCHEME_WORDS[app.system_scheme],
            if app.reduced_motion {
                "reduced"
            } else {
                "full"
            },
            mv_theme::SLUG_WORDS[mv_theme::shown(app.prefs, app.system_scheme)],
        );
        app.input.pointer = [-1.0, -1.0];
        let first = crate::input::Input::Window(crate::input::window_event(app.window()));
        crate::input::handle_input(&mut app, first, 0);

        Ok((
            Self {
                main: MainThreadData::assert_new(MainData {
                    app,
                    _video: video,
                    _sdl: sdl,
                    panic_at_frame,
                    frames: 0,
                }),
            },
            (),
        ))
    }
}

/// Tears the app down: the live run's file, the platform, NanoVG's fonts,
/// Clay, then the GL context and the window (SDL quits with the `Sdl`
/// handle's drop).
fn teardown(app: &mut crate::app::App) {
    #[cfg(not(target_os = "emscripten"))]
    crate::live::live_finish(app);
    platform_quit();
    mv_text::destroy(&mut app.fonts);
    crate::ui::release_clay(app);
    #[cfg(not(target_os = "emscripten"))]
    {
        app.gl = None;
        app.gl_context = None;
        app.window = None;
    }
    // On the web the SDL owners are leaked, never destroyed: destroying the
    // window zeroes the Emscripten canvas element, and a Rust panic (the
    // sweep's `MV_PANIC_AT_FRAME`, or any failure) unwinds through Drop glue
    // on the way out, so dropping these owners then blanks the page (t0c-b's
    // teardown lesson from its wasm example). Leaking is deterministic; the
    // page holds them for its life.
    #[cfg(target_os = "emscripten")]
    {
        std::mem::forget(app.gl.take());
        std::mem::forget(app.gl_context.take());
        std::mem::forget(app.window.take());
    }
}

#[app_impl]
impl Main {
    fn app_init() -> AppResultWithState<Box<Mutex<Self>>> {
        match Self::build() {
            Ok((main, run)) => {
                let main = Box::new(Mutex::new(main));
                {
                    // Clay after the App's last move: its measure closure
                    // holds a `*mut App`, which boxing would otherwise leave
                    // dangling. Everything `build` did needs no Clay; the
                    // first layout does.
                    let mut guard = main.lock().expect("app_init: clay lock");
                    let app = &mut guard.main.assert_get_mut().app;
                    crate::ui::init_clay(app);
                }
                #[cfg(not(target_os = "emscripten"))]
                if let Some(run) = run {
                    // The harness runs natively only: Playwright drives the
                    // web build from outside. The state rides along on every
                    // return: SDL still routes its quit callback through
                    // sdl3-main's state machinery, which asserts non-null, so
                    // a `None` here panics after the run (measured). The one
                    // teardown runs in `app_quit`, never here.
                    // The sweep's crash proof on the scripted paths too: a
                    // scripted run executes inside `app_init` and never
                    // reaches `app_iterate`, where t0a's switch fires, so
                    // without this a scripted run with the flag set would
                    // report 0 crashes.
                    if data_panics(&main) {
                        panic!("MV_PANIC_AT_FRAME: intentional panic in frame 1");
                    }
                    if run.live {
                        let mut guard = main.lock().expect("app_init: live lock");
                        let data = guard.main.assert_get_mut();
                        let ran = crate::live::live_start(&mut data.app, &run.script, &run.out);
                        drop(guard);
                        return if ran {
                            AppResultWithState::Continue(main)
                        } else {
                            AppResultWithState::Failure(Some(main))
                        };
                    }
                    let mut guard = main.lock().expect("app_init: harness lock");
                    let data = guard.main.assert_get_mut();
                    let ran = crate::harness::run_harness(
                        &mut data.app,
                        &run.script,
                        &run.out,
                        run.window,
                        run.time,
                    );
                    drop(guard);
                    return if ran {
                        AppResultWithState::Success(Some(main))
                    } else {
                        AppResultWithState::Failure(Some(main))
                    };
                }
                #[cfg(target_os = "emscripten")]
                let () = run;
                AppResultWithState::Continue(main)
            }
            Err(err) => {
                eprintln!("math-viz2-rust init: {err}");
                // No state exists to hand SDL's quit callback (which asserts
                // non-null), so exit outright: the `[ERROR]` lines above are
                // the harness's failure signal.
                std::process::exit(1);
            }
        }
    }

    fn app_iterate(&mut self) -> AppResult {
        let data = self.main.assert_get_mut();
        data.frames += 1;
        if data.panic_at_frame {
            panic!(
                "MV_PANIC_AT_FRAME: intentional panic in frame {}",
                data.frames
            );
        }
        // One turn of the loop: the timers that fell due, a frame when
        // anything changed or moves, and how the loop goes on from here.
        let app = &mut data.app;
        let drawn = crate::app_loop::iterate(app);
        if drawn {
            app.window().gl_swap_window();
        }
        let (iterations, frames) = (app.r#loop.iterations, app.r#loop.frames);
        platform_loop_counts(iterations, frames);
        // The web build idles here instead of quitting: quitting destroys the
        // SDL window, and SDL's emscripten backend resizes what it can't
        // destroy — the canvas — to zero, blanking the page. Natively the
        // app runs until its window closes; i4-loop teaches both to pause.
        AppResult::Continue
    }

    fn app_event(&mut self, event: &Event) -> AppResult {
        let data = self.main.assert_get_mut();
        let app = &mut data.app;
        match event {
            Event::Quit { .. } => return AppResult::Success,
            Event::User {
                type_,
                code,
                data1,
                data2,
                timestamp,
                ..
            } if *type_
                == crate::app_loop::USER_EVENT_TYPE.load(std::sync::atomic::Ordering::Relaxed) =>
            {
                use crate::app_loop::{UserCode, woken};
                if *code == UserCode::Wake as i32 {
                    // A deadline came: data1 is the wake timer's id, data2
                    // the interval SDL armed it for, in µs (`woken` takes ns).
                    woken(
                        app,
                        *data1 as usize as u32,
                        *timestamp as crate::app_loop::Time,
                        *data2 as usize as crate::app_loop::Time * 1000,
                    );
                    return AppResult::Continue;
                }
                if *code == UserCode::Model as i32 {
                    // SAFETY: data1 is the boxed `mv_script::Model` the
                    // platform's `mv_model_line` queued; it is read here and
                    // freed when the event's second word says so, as the
                    // native side frees its model.
                    let change = unsafe { (*(data1.cast::<mv_script::Model>())).clone() };
                    crate::input::handle_input(
                        app,
                        crate::input::Input::Model(crate::input::ModelEvent { change }),
                        *timestamp as crate::app_loop::Time,
                    );
                    if !data2.is_null() {
                        drop(unsafe { Box::from_raw(data1.cast::<mv_script::Model>()) });
                    }
                    return AppResult::Continue;
                }
                if *code == UserCode::Prefs as i32 {
                    if data1.is_null() {
                        // The web's storage event: another tab wrote under one
                        // of the prefs keys. The event carries one key's new
                        // value, while the app needs all three, so the keys
                        // are re-read here, as math-viz2's storage subscription
                        // re-reads its atoms.
                        let (text, _) = platform_load_prefs(app);
                        crate::prefs::prefs_stored_elsewhere(app, mv_theme::parse_prefs(&text));
                    } else {
                        crate::prefs::prefs_stored_elsewhere(
                            app,
                            crate::prefs::prefs_of_word(*data1 as usize),
                        );
                    }
                    return AppResult::Continue;
                }
                #[cfg(not(target_os = "emscripten"))]
                if *code == UserCode::Live as i32 {
                    return crate::live::live_step(app, *data1 as usize as i64);
                }
                #[cfg(target_os = "emscripten")]
                if *code == UserCode::Hash as i32 {
                    // The page's hash changed under the app (an edit in the
                    // address bar, back/forward): route and query in at once,
                    // then each param's sync with the still camera's adopt
                    // (h2-input owns the camera's half).
                    app.url.sync_from_hash(&platform_read_hash());
                    crate::plane_input::external_url_synced(app);
                    return AppResult::Continue;
                }
                // Anything else (notably `ReducedMotion`) falls through to
                // `input_from_event`, which decodes it by type and code.
            }
            _ => {}
        }
        if let Some(input) = crate::input::input_from_event(app, event) {
            crate::input::handle_input(app, input, event.get_timestamp() as crate::app_loop::Time);
        }
        AppResult::Continue
    }

    fn app_quit(&mut self, _result: AppResult) {
        let data = self.main.assert_get_mut();
        teardown(&mut data.app);
    }
}
