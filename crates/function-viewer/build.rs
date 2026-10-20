// Links the macOS Objective-C helpers in `csrc/`. Only the macOS target
// compiles them: the script reads the target being built (`CARGO_CFG_TARGET_OS`),
// never the host, so cross-checks from macOS stay quiet.

fn main() {
    println!("cargo:rerun-if-changed=csrc/platform_darwin.m");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        cc::Build::new()
            .file("csrc/platform_darwin.m")
            .compile("platform_darwin");
        // What the `.m` file calls: AppKit (windows, cursors, workspaces),
        // Foundation (strings, data, replies), CoreGraphics (displays, colour
        // spaces). No SDL headers cross into the `.m` file.
        println!("cargo:rustc-link-lib=framework=AppKit");
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=framework=CoreGraphics");
    }
    // The web link's flags (deltas 1-4 in the workspace `.cargo/config.toml`):
    // the wasm spike's proven `-s` set plus `--js-library web/library.js`
    // and the `-sEXPORTED_FUNCTIONS` list naming the `mv_*` exports
    // `web/library.js` calls back into. They ride
    // `cargo:rustc-link-arg-bins=` so they reach only this package's binary
    // targets, never another crate's test binaries (a library crate's test
    // has no `_main` or `mv_pinch`, so workspace-wide rustflags broke
    // `cargo build --tests` for the wasm target: t0c-wire's finding 1). The
    // strings match the old rustflags except `-sEXIT_RUNTIME=0` (web-blend's
    // delta 5, below), and link-time paths
    // still resolve from the workspace root, which `./build.sh` runs from.
    // They live here rather than in `.cargo/config.toml` because an
    // `RUSTFLAGS` in the environment (which `./build.sh release` sets for
    // the remaps) replaces the config's `rustflags` instead of adding to
    // them: with the `-s` set in the config, the release web link silently
    // lost `MIN_WEBGL_VERSION=2` and created a WebGL1 context (measured,
    // progress/i3-release.md R4b). Emitted per-bin, the set applies to every
    // web link, check and release alike.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("emscripten") {
        for arg in [
            "-sALLOW_MEMORY_GROWTH=1",
            "-sMAX_WEBGL_VERSION=2",
            "-sMIN_WEBGL_VERSION=2",
            "-sFULL_ES3=1",
            "-sEXPORTED_RUNTIME_METHODS=['callMain','ccall','cwrap']",
            // Delta 5 (web-blend): EXIT_RUNTIME off, the Emscripten default
            // the Odin build links with. With `=1`, the idle design's
            // `MainLoop.pause()` pops the runtime's last keepalive and the
            // next user callback's `maybeExit` runs `_exit`, setting ABORT;
            // every later main-loop turn then silently no-ops, so input
            // after the first idle never reaches the app (measured in
            // progress/web-blend.md: ABORT true after a 2000 ms idle, the
            // lit-k probe's move lost; 0 px post-fix in all three engines).
            "-sEXIT_RUNTIME=0",
            "--js-library",
            "web/library.js",
            "-sEXPORTED_FUNCTIONS=_main,_mv_pinch,_mv_hash_changed,_mv_prefs_stored,_mv_reduced_motion_changed,_mv_model_line",
            // Delta 5 (i4-loop): the runtime stays alive while the main loop
            // is paused. The app idles by pausing Emscripten's main loop
            // (platform_web.rs's platform_pace through library.js's
            // mv_loop_idle, as the Odin app does); pausing pops the loop's
            // keepalive, and the runner's staleness check then calls
            // maybeExit, which exits(0) a runtime with nothing alive. With
            // the default (false) that kills the page's app at its first
            // idle: Module.iterations freezes, wake-ups never run, and
            // onExit(0) fires (measured in WebKit). The Odin app's web build
            // bakes in noExitRuntime=true and survives its pauses the same
            // way this flag makes this build survive them. The app never
            // exits on the web by design (its teardown leaks the SDL owners;
            // the FILL_DOCUMENT quit workaround), so keeping the runtime is
            // faithful, not a behaviour change.
            "-sNO_EXIT_RUNTIME=1",
        ] {
            println!("cargo:rustc-link-arg-bins={arg}");
        }
    }
}
