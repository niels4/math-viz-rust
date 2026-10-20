//! The window icon: the favicon rendered off-screen and installed as the
//! SDL window icon.
//!
//! The pixel readback (`gl_read_rgba`) and the surface fill (`surface_fill`,
//! a pitch-strided copy through the safe `with_lock_mut`) live here beside
//! it, and the `set_icon` call stays with that caller. The page's own icons
//! (wordmark, gear, locate, toggle marks, check) are their milestone's.

use crate::app::App;
use crate::draw::gl_target;

/// The wordmark flake's two stroke groups, as the favicon's mark.
pub const SNOWFLAKE_PATHS: [&str; 2] = [
    "M12 2v20M3.3 7l17.4 10M20.7 7L3.3 17",
    "M12 2l-2 2.5M12 2l2 2.5M12 22l-2-2.5M12 22l2-2.5",
];

/// The favicon's tile and mark colours.
pub const FAVICON_TILE: [u8; 4] = [0x10, 0x19, 0x21, 0xff];
pub const FAVICON_MARK: [u8; 4] = [0x70, 0xc0, 0xa0, 0xff];

/// The icon raster size: small on the web, full natively.
#[cfg(target_os = "emscripten")]
pub const ICON_PIXELS: u32 = 64;
#[cfg(not(target_os = "emscripten"))]
pub const ICON_PIXELS: u32 = 1024;

/// Renders the favicon, unpremultiplies it, and sets the SDL window icon.
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
pub fn unpremultiply(rgba: &mut [u8]) {
    for px in rgba.as_chunks_mut::<4>().0 {
        let a = px[3];
        if a == 0 || a == 255 {
            continue;
        }
        let a32 = u32::from(a);
        for b in &mut px[..3] {
            *b = ((u32::from(*b) * 255 + a32 / 2) / a32).min(255) as u8;
        }
    }
}

/// Opaque RGB bytes as NanoVG takes them.
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
/// The caller bound the icon framebuffer first. The contract is `n * n * 4`
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
/// Row `y` of the surface takes row `y` of `rows`. The surface's
/// `with_lock_mut` keeps the pixel access safe (its `without_lock_mut` is the
/// `unsafe` one); the `set_icon` call stays with the caller
/// (`set_window_icon` above).
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
