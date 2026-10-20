//! The SVG icons: the wordmark's flake, the gear, the locate mark, the
//! appearance toggle's marks and the checked theme's check.
//!
//! Icons draw through the SVG path parser at their view box, stroked round.
//! `set_window_icon` renders the favicon off-screen and installs it as the
//! SDL window icon; the pixel readback (`gl_read_rgba`) and the surface fill
//! (`surface_fill`, a pitch-strided copy through the safe `with_lock_mut`)
//! live here beside it, and the `set_icon` call stays with that caller.

use mv_sys::clay;

use crate::app::App;
use crate::draw::gl_target;

/// The wordmark flake's two stroke groups.
pub const SNOWFLAKE_PATHS: [&str; 2] = [
    "M12 2v20M3.3 7l17.4 10M20.7 7L3.3 17",
    "M12 2l-2 2.5M12 2l2 2.5M12 22l-2-2.5M12 22l2-2.5",
];

/// The gear's outline and its hub radius.
pub const GEAR_PATH: &str = "M19.4 13.5a7.5 7.5 0 0 0 0-3l2-1.5-2-3.4-2.3 1a7.6 7.6 0 0 0-2.6-1.5L14 2h-4l-.5 2.6a7.6 7.6 0 0 0-2.6 1.5l-2.3-1-2 3.4 2 1.5a7.5 7.5 0 0 0 0 3l-2 1.5 2 3.4 2.3-1a7.6 7.6 0 0 0 2.6 1.5L10 22h4l.5-2.6a7.6 7.6 0 0 0 2.6-1.5l2.3 1 2-3.4-2-1.5z";
pub const GEAR_HUB_RADIUS: f32 = 3.2;

/// An icon in a colour.
#[derive(Clone, Copy, Debug)]
pub struct Icon {
    /// Which icon.
    pub kind: IconKind,
    /// The icon's ink.
    pub ink: mv_theme::Color,
}

/// The seven icons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconKind {
    /// The wordmark's mark.
    Snowflake,
    /// The settings button's, its joins round.
    Gear,
    /// Return to origin's.
    Locate,
    /// The Appearance toggle's marks.
    System,
    Sun,
    Moon,
    /// The checked theme's check in the settings list.
    Check,
}

/// The locate mark's ring, dot and ticks.
pub const LOCATE_RING_RADIUS: f32 = 6.5;
pub const LOCATE_DOT_RADIUS: f32 = 2.6;
pub const LOCATE_TICKS: &str = "M12 2.5V5.5M12 18.5V21.5M2.5 12H5.5M18.5 12H21.5";

/// The default icon metrics.
pub const ICON_VIEW_BOX: f32 = 24.0;
pub const ICON_STROKE: f32 = 1.8;

/// The toggle marks: x, y, width, height, corner radius; the stand; the sun's
/// radius and rays; the moon's path; the toggle marks' own metrics.
pub const SYSTEM_SCREEN: [f32; 5] = [1.8, 2.5, 12.4, 8.5, 1.6];
pub const SYSTEM_STAND: &str = "M8 11v2.75M5.25 13.75h5.5";
pub const SUN_RADIUS: f32 = 3.0;
pub const SUN_RAYS: &str = "M13.2 8h1.6M11.68 11.68l1.13 1.13M8 13.2v1.6M4.32 11.68l-1.13 1.13M2.8 8H1.2M4.32 4.32 3.19 3.19M8 2.8V1.2M11.68 4.32l1.13-1.13";
pub const MOON_PATH: &str = "M13.6 8.54A6 6 0 1 1 7.46 2.4 4.6 4.6 0 0 0 13.6 8.54Z";
pub const MODE_ICON_VIEW_BOX: f32 = 16.0;
pub const MODE_ICON_STROKE: f32 = 1.6;

/// The check's path and metrics.
pub const CHECK_PATH: &str = "M2.5 7.5L5.5 10.5L11.5 3.5";
pub const CHECK_VIEW_BOX: f32 = 14.0;
pub const CHECK_STROKE: f32 = 2.0;

/// The view box and stroke for a kind.
///
/// Ported from `icons.odin:73` (`icon_frame`).
pub fn icon_frame(kind: IconKind) -> (f32, f32) {
    match kind {
        IconKind::System | IconKind::Sun | IconKind::Moon => (MODE_ICON_VIEW_BOX, MODE_ICON_STROKE),
        IconKind::Check => (CHECK_VIEW_BOX, CHECK_STROKE),
        _ => (ICON_VIEW_BOX, ICON_STROKE),
    }
}

/// Scales to the box and strokes/fills the kind.
///
/// Ported from `icons.odin:83` (`draw_icon`).
pub fn draw_icon(vg: &mut mv_nvg::Context, _box: clay::BoundingBox, icon: Icon) {
    let (view_box, stroke) = icon_frame(icon.kind);
    let scale = _box.width / view_box;
    vg.save();
    vg.translate(_box.x, _box.y);
    vg.scale(scale, scale);
    vg.stroke_color(icon.ink);
    vg.stroke_width(stroke);
    vg.line_cap(mv_nvg::LineCapType::ROUND);
    // `LineJoin ROUND` only where the Odin code sets it (Gear, System, Sun,
    // Moon, Check): no joints in the other paths, so no effect there.
    match icon.kind {
        IconKind::Snowflake => {
            for d in SNOWFLAKE_PATHS {
                vg.begin_path();
                let _ = crate::svgpath::draw_path(vg, d);
                vg.stroke();
            }
        }
        IconKind::Gear => {
            vg.line_join(mv_nvg::LineCapType::ROUND);
            vg.begin_path();
            vg.circle(view_box / 2.0, view_box / 2.0, GEAR_HUB_RADIUS);
            vg.stroke();
            vg.begin_path();
            let _ = crate::svgpath::draw_path(vg, GEAR_PATH);
            vg.stroke();
        }
        IconKind::Locate => {
            vg.begin_path();
            vg.circle(view_box / 2.0, view_box / 2.0, LOCATE_RING_RADIUS);
            vg.stroke();
            vg.begin_path();
            vg.circle(view_box / 2.0, view_box / 2.0, LOCATE_DOT_RADIUS);
            vg.fill_color(icon.ink);
            vg.fill();
            vg.begin_path();
            let _ = crate::svgpath::draw_path(vg, LOCATE_TICKS);
            vg.stroke();
        }
        IconKind::System => {
            vg.line_join(mv_nvg::LineCapType::ROUND);
            vg.begin_path();
            vg.rounded_rect(
                SYSTEM_SCREEN[0],
                SYSTEM_SCREEN[1],
                SYSTEM_SCREEN[2],
                SYSTEM_SCREEN[3],
                SYSTEM_SCREEN[4],
            );
            vg.stroke();
            vg.begin_path();
            let _ = crate::svgpath::draw_path(vg, SYSTEM_STAND);
            vg.stroke();
        }
        IconKind::Sun => {
            vg.line_join(mv_nvg::LineCapType::ROUND);
            vg.begin_path();
            vg.circle(view_box / 2.0, view_box / 2.0, SUN_RADIUS);
            vg.stroke();
            vg.begin_path();
            let _ = crate::svgpath::draw_path(vg, SUN_RAYS);
            vg.stroke();
        }
        IconKind::Moon => {
            vg.line_join(mv_nvg::LineCapType::ROUND);
            vg.begin_path();
            let _ = crate::svgpath::draw_path(vg, MOON_PATH);
            vg.stroke();
        }
        IconKind::Check => {
            vg.line_join(mv_nvg::LineCapType::ROUND);
            vg.begin_path();
            let _ = crate::svgpath::draw_path(vg, CHECK_PATH);
            vg.stroke();
        }
    }
    vg.restore();
}

/// The favicon's tile and mark colours.
pub const FAVICON_TILE: [u8; 4] = [0x10, 0x19, 0x21, 0xff];
pub const FAVICON_MARK: [u8; 4] = [0x70, 0xc0, 0xa0, 0xff];

/// The icon raster size: small on the web, full natively.
#[cfg(target_os = "emscripten")]
pub const ICON_PIXELS: u32 = 64;
#[cfg(not(target_os = "emscripten"))]
pub const ICON_PIXELS: u32 = 1024;

/// Renders the favicon, unpremultiplies it, and sets the SDL window icon.
///
/// Ported from `icons.odin:161` (`set_window_icon`).
pub fn set_window_icon(app: &mut App) {
    let n = ICON_PIXELS as i32;
    let fb = mv_nvg::Framebuffer::create(&mut app.vg, n, n, 0);
    fb.bind();
    gl_target(app, [n, n], [0.0, 0.0, 0.0, 0.0]);
    app.vg.begin_frame(24.0, 24.0, n as f32 / 24.0);
    draw_favicon(&mut app.vg);
    app.vg.end_frame();
    let pixels = gl_read_rgba(app, n);
    fb.delete();
    mv_nvg::Framebuffer::bind_default();
    let mut surface =
        match sdl3::surface::Surface::new(n as u32, n as u32, sdl3::pixels::PixelFormat::RGBA32) {
            Ok(surface) => surface,
            Err(e) => {
                eprintln!("function-viewer-rust: SDL_CreateSurface: {e}");
                return;
            }
        };
    let row = n as usize * 4;
    let mut top_down = vec![0u8; pixels.len()];
    let rows = n as usize;
    for y in 0..rows {
        top_down[y * row..(y + 1) * row]
            .copy_from_slice(&pixels[(rows - 1 - y) * row..(rows - y) * row]);
    }
    unpremultiply(&mut top_down);
    surface_fill(&mut surface, &top_down);
    if !app.window_mut().set_icon(&surface) {
        eprintln!(
            "function-viewer-rust: SDL_SetWindowIcon: {}",
            sdl3::get_error()
        );
    }
}

/// Draws the tile and the scaled flake mark.
///
/// Ported from `icons.odin:196` (`draw_favicon`).
pub fn draw_favicon(vg: &mut mv_nvg::Context) {
    vg.begin_path();
    vg.rounded_rect(0.0, 0.0, 24.0, 24.0, 5.0);
    vg.fill_color(rgba8(FAVICON_TILE[0], FAVICON_TILE[1], FAVICON_TILE[2]));
    vg.fill();
    vg.save();
    vg.translate(12.0, 12.0);
    vg.scale(0.9, 0.9);
    vg.translate(-12.0, -12.0);
    vg.stroke_color(rgba8(FAVICON_MARK[0], FAVICON_MARK[1], FAVICON_MARK[2]));
    vg.stroke_width(2.4);
    vg.line_cap(mv_nvg::LineCapType::ROUND);
    for d in SNOWFLAKE_PATHS {
        vg.begin_path();
        if !crate::svgpath::draw_path(vg, d) {
            eprintln!("function-viewer-rust: path data NanoVG can't draw: {d:?}");
        }
        vg.stroke();
    }
    vg.restore();
}

/// Converts premultiplied RGBA bytes to straight alpha, in place.
///
/// Ported from `icons.odin:220` (`unpremultiply`).
pub fn unpremultiply(rgba: &mut [u8]) {
    for px in rgba.as_chunks_mut::<4>().0 {
        let a = px[3];
        if a == 0 || a == 255 {
            continue;
        }
        let a32 = u32::from(a);
        for b in px[..3].iter_mut() {
            *b = ((u32::from(*b) * 255 + a32 / 2) / a32).min(255) as u8;
        }
    }
}

/// Opaque RGB bytes as NanoVG takes them.
///
/// Ported from `icons.odin`'s `nvg.RGB` calls.
fn rgba8(r: u8, g: u8, b: u8) -> mv_nvg::Color {
    [
        f32::from(r) / 255.0,
        f32::from(g) / 255.0,
        f32::from(b) / 255.0,
        1.0,
    ]
}

/// Reads the current framebuffer back as packed bottom-up RGBA rows.
///
/// Ported from `icons.odin:176` (`set_window_icon`'s `gl.ReadPixels`): the
/// caller bound the icon framebuffer first. The contract is `n * n * 4`
/// bytes, rows bottom up as `glReadPixels` gives them (the caller flips to
/// top-down and unpremultiplies).
fn gl_read_rgba(app: &App, n: i32) -> Vec<u8> {
    use glow::HasContext as _;
    let gl = app.gl.as_ref().expect("no GL: headless App");
    let mut px = vec![0u8; n as usize * n as usize * 4];
    // SAFETY: the context is current on this thread (made current at start),
    // and the read targets the freshly sized buffer, exactly `n * n * 4`
    // bytes, as `glReadPixels` writes them.
    unsafe {
        gl.pixel_store_i32(glow::PACK_ALIGNMENT, 1);
        gl.read_pixels(
            0,
            0,
            n,
            n,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            glow::PixelPackData::Slice(Some(px.as_mut_slice())),
        );
    }
    px
}

/// Copies packed top-down RGBA rows into the SDL surface, honouring its pitch.
///
/// Ported from `icons.odin:186` (`set_window_icon`'s row loop): row `y` of
/// the surface takes row `y` of `rows`. The surface's `with_lock_mut` keeps
/// the pixel access safe (its `without_lock_mut` is the `unsafe` one, which
/// is why the surface fill was once assigned to `app-platform`); the
/// `set_icon` call stays with the caller (`set_window_icon` above).
fn surface_fill(surface: &mut sdl3::surface::Surface<'_>, rows: &[u8]) {
    let (w, h) = surface.size();
    let pitch = surface.pitch() as usize;
    let row_len = w as usize * 4;
    surface.with_lock_mut(|pixels| {
        for y in 0..h as usize {
            pixels[y * pitch..y * pitch + row_len]
                .copy_from_slice(&rows[y * row_len..(y + 1) * row_len]);
        }
    });
}
