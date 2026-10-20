//! The frame: the GL clear and the NanoVG helpers the page painters share.
//!
//! The shell draws one frame cleared to one fixed colour. The page milestones
//! lay out with Clay and paint through the helpers below: the pixel snapping,
//! the colour conversions, the CSS corner radii, the rounded-rect path, and
//! the GL target/viewport shims (every `glow` call sits behind these, since
//! `glow` 0.18 methods are `unsafe fn`).

use crate::app::App;
use crate::app_loop::Time;
use crate::input::Region;
use mv_sys::clay;

/// The shell frame's clear colour: opaque `#070d12`, the web shell's page
/// background behind the canvas.
pub const SHELL_CLEAR: [f32; 4] = [
    0x07 as f32 / 255.0,
    0x0d as f32 / 255.0,
    0x12 as f32 / 255.0,
    1.0,
];

/// Paints one frame: the shell's clear, then the page over it.
pub fn draw_frame(app: &mut App, now: Time) {
    gl_target(app, app.pixels, SHELL_CLEAR);
    crate::page::draw_page(app, now);
}

/// Rounds a layout box out to whole backing pixels at the given density.
pub fn snap_box(_box: clay::BoundingBox, density: f32) -> clay::BoundingBox {
    fn snap(v: f32, density: f32) -> f32 {
        (v * density + 0.5).floor() / density
    }
    let x0 = snap(_box.x, density);
    let y0 = snap(_box.y, density);
    let x1 = snap(_box.x + _box.width, density);
    let y1 = snap(_box.y + _box.height, density);
    clay::BoundingBox {
        x: x0,
        y: y0,
        width: x1 - x0,
        height: y1 - y0,
    }
}

/// A Clay colour (0 to 255 per channel) as NanoVG takes it (0 to 1).
pub fn nvg_color(c: clay::Color) -> mv_nvg::Color {
    [c[0] / 255.0, c[1] / 255.0, c[2] / 255.0, c[3] / 255.0]
}

/// Corner radii shrunk to fit their box, as CSS takes them: each side's pair
/// scaled down by one factor until it fits (CSS Backgrounds 3 § 5.5), which
/// is also exactly how Clay fits them.
pub fn css_radii(_box: clay::BoundingBox, r: clay::CornerRadius) -> clay::CornerRadius {
    let mut f = 1.0;
    let mut fit = |length: f32, sum: f32| {
        if sum > 0.0 && length < sum * f {
            f = length.max(0.0) / sum;
        }
    };
    fit(_box.width, r.topLeft + r.topRight);
    fit(_box.width, r.bottomLeft + r.bottomRight);
    fit(_box.height, r.topLeft + r.bottomLeft);
    fit(_box.height, r.topRight + r.bottomRight);
    clay::CornerRadius {
        topLeft: r.topLeft * f,
        topRight: r.topRight * f,
        bottomLeft: r.bottomLeft * f,
        bottomRight: r.bottomRight * f,
    }
}

/// Each corner radius moved inward by `by`, floored at zero.
pub fn inset_radius(r: clay::CornerRadius, by: f32) -> clay::CornerRadius {
    clay::CornerRadius {
        topLeft: (r.topLeft - by).max(0.0),
        topRight: (r.topRight - by).max(0.0),
        bottomLeft: (r.bottomLeft - by).max(0.0),
        bottomRight: (r.bottomRight - by).max(0.0),
    }
}

/// A theme colour as CSS takes it: partial alpha premultiplies the channels.
pub fn css_background(c: mv_theme::Color) -> mv_theme::Color {
    if c[3] <= 0.0 || c[3] >= 1.0 {
        return c;
    }
    let a = (c[3] * 255.0).round();
    let mut out = [0.0, 0.0, 0.0, a / 255.0];
    for i in 0..3 {
        out[i] = (c[i] * a).round() / a;
    }
    out
}

/// Emits the box's path with per-corner radii, without painting.
///
/// Callers open the path and fill or stroke it themselves.
pub fn rounded_rect(vg: &mut mv_nvg::Context, _box: clay::BoundingBox, radius: clay::CornerRadius) {
    let r = css_radii(_box, radius);
    vg.rounded_rect_varying(
        _box.x,
        _box.y,
        _box.width,
        _box.height,
        r.topLeft,
        r.topRight,
        r.bottomRight,
        r.bottomLeft,
    );
}

/// Fills a plain rectangle.
pub fn fill_rect(vg: &mut mv_nvg::Context, r: [f32; 4], ink: mv_nvg::Color) {
    vg.begin_path();
    vg.rect(r[0], r[1], r[2], r[3]);
    vg.fill_color(ink);
    vg.fill();
}

/// Clips NanoVG to the region's clip box, or clears the scissor without one.
pub fn clip_to(vg: &mut mv_nvg::Context, r: &Region) {
    if let Some(clip) = r.clip {
        vg.scissor(clip.x, clip.y, clip.width, clip.height);
    } else {
        vg.reset_scissor();
    }
}

/// The GL backing store for `pixels`, cleared to `clear`.
///
/// The viewport over the whole store, then the clear colour and a
/// colour+stencil clear. Runs through the `glow::Context` on `App`; the
/// context is current on this thread whenever a frame draws.
pub(crate) fn gl_target(app: &App, pixels: [i32; 2], clear: [f32; 4]) {
    use glow::HasContext as _;
    let gl = app.gl.as_ref().expect("GL context: the real app");
    // SAFETY: the context is current on this thread (made current at start),
    // and the viewport/clear take plain integers and floats.
    unsafe {
        gl.viewport(0, 0, pixels[0], pixels[1]);
        gl.clear_color(clear[0], clear[1], clear[2], clear[3]);
        gl.clear(glow::COLOR_BUFFER_BIT | glow::STENCIL_BUFFER_BIT);
    }
}

/// The GL viewport for `pixels`, without touching the clear state.
pub(crate) fn gl_viewport(app: &App, pixels: [i32; 2]) {
    use glow::HasContext as _;
    let gl = app.gl.as_ref().expect("GL context: the real app");
    // SAFETY: as in `gl_target`.
    unsafe {
        gl.viewport(0, 0, pixels[0], pixels[1]);
    }
}
