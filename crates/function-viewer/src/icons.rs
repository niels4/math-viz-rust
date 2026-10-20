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

/// The page's own icons: the wordmark flake and the settings gear.
///
/// Ported from math-viz2's `src/components/icons/icons.tsx` (`Logo`,
/// `GearIcon`) and `TopBar/icons.tsx`: the flake's two stroke groups are the
/// favicon's mark above; the gear is a 3.2 hub with its toothed ring path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconKind {
    Logo,
    Gear,
}

/// An icon to paint: its kind and its ink, already resolved to a colour.
#[derive(Clone, Copy, Debug)]
pub struct Icon {
    pub kind: IconKind,
    pub ink: mv_nvg::Color,
}

/// The gear's toothed ring, as `GearIcon` writes it (viewBox 24).
pub const GEAR_PATH: &str = "M19.4 13.5a7.5 7.5 0 0 0 0-3l2-1.5-2-3.4-2.3 1a7.6 7.6 0 0 0-2.6-1.5L14 2h-4l-.5 2.6a7.6 7.6 0 0 0-2.6 1.5l-2.3-1-2 3.4 2 1.5a7.5 7.5 0 0 0 0 3l-2 1.5 2 3.4 2.3-1a7.6 7.6 0 0 0 2.6 1.5L10 22h4l.5-2.6a7.6 7.6 0 0 0 2.6-1.5l2.3 1 2-3.4-2-1.5z";

/// The gear hub's radius, as `GearIcon` writes it.
pub const GEAR_HUB_RADIUS: f32 = 3.2;

/// The icons' viewBox, as both components write it.
pub const ICON_VIEWBOX: f32 = 24.0;

/// The icons' stroke width, as both components write it.
pub const ICON_STROKE: f32 = 1.8;

/// Paints an icon in a `size` box at `at`: the viewBox scaled to the box,
/// stroked round in its ink.
pub fn draw_icon(vg: &mut mv_nvg::Context, at: [f32; 2], size: f32, icon: &Icon) {
    vg.save();
    vg.translate(at[0], at[1]);
    let scale = size / ICON_VIEWBOX;
    vg.scale(scale, scale);
    vg.stroke_color(icon.ink);
    vg.stroke_width(ICON_STROKE);
    vg.line_cap(mv_nvg::LineCapType::ROUND);
    match icon.kind {
        IconKind::Logo => {
            for d in SNOWFLAKE_PATHS {
                vg.begin_path();
                if !crate::svgpath::draw_path(vg, d) {
                    eprintln!("math-viz2-rust: icon path NanoVG can't draw: {d:?}");
                }
                vg.stroke();
            }
        }
        IconKind::Gear => {
            vg.line_join(mv_nvg::LineCapType::ROUND);
            vg.begin_path();
            vg.circle(ICON_VIEWBOX / 2.0, ICON_VIEWBOX / 2.0, GEAR_HUB_RADIUS);
            vg.stroke();
            vg.begin_path();
            if !crate::svgpath::draw_path(vg, GEAR_PATH) {
                eprintln!("math-viz2-rust: icon path NanoVG can't draw: {GEAR_PATH:?}");
            }
            vg.stroke();
        }
    }
    vg.restore();
}

// Frame-lived icon payloads: layout boxes an `Icon` per custom element, and
// the paint reads it back. The boxes outlive the call, so they live in a
// frame store cleared by the frame's draw.
thread_local! {
    // Each icon boxed: pushing to the `Vec` may move it, while the boxed
    // icon the Clay config points at never moves.
    #[allow(clippy::vec_box)]
    static CUSTOMS: std::cell::RefCell<Vec<Box<Icon>>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// Empties the frame's icon store; the frame's draw calls it first.
pub fn clear_customs() {
    CUSTOMS.with(|store| store.borrow_mut().clear());
}

/// The Clay custom config painting `icon`.
pub fn custom_data(icon: Icon) -> mv_sys::clay::CustomElementConfig {
    let ptr = CUSTOMS.with(|store| {
        let mut store = store.borrow_mut();
        store.push(Box::new(icon));
        std::ptr::from_ref::<Icon>(store.last().expect("icon just pushed"))
            .cast_mut()
            .cast::<std::ffi::c_void>()
    });
    mv_sys::clay::CustomElementConfig { customData: ptr }
}

/// The icon a custom command paints, if it is one.
pub fn custom_of(
    command: &mv_sys::clay::RenderCommand,
) -> Option<(&Icon, mv_sys::clay::CornerRadius)> {
    mv_clay::custom_of::<Icon>(command)
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
                eprintln!("math-viz2-rust: SDL_CreateSurface: {e}");
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
        eprintln!("math-viz2-rust: SDL_SetWindowIcon: {}", sdl3::get_error());
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
            eprintln!("math-viz2-rust: path data NanoVG can't draw: {d:?}");
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
