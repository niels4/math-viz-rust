//! The welcome hero's backdrop: the graph paper and the two masked lights.
//!
//! `WelcomePage.module.css` at `68e1c7b` lays four translucent layers behind
//! the hero plane (DOM order, all `z-index: -1`): the vertical paper lines
//! (`.paper_x`), the horizontal ones (`.paper_y`), the gradient top light
//! (`.light_top`) and the single-colour body light (`.light_body`). The hero
//! plane's frame paints opaque over them.
//!
//! The browser composites every layer "over" in sRGB-encoded space while the
//! port's framebuffer blends in linear light (the h4-readout/h5-ruler
//! finding), so the whole backdrop is composited here on the CPU per pixel in
//! sRGB into one opaque RGBA image at device resolution, uploaded once per
//! theme, size and fit and drawn with an image pattern (phase 2). Colours
//! ride in as [`BackdropColors`] (extended sRGB, unclipped: `--param-a` sits
//! outside sRGB in every theme, and CSS interpolates the gradient before any
//! gamut map); geometry as [`BackdropGeometry`] in design px with the fit and
//! density (phase 1 runs at fit 1; h10-fit wires the real fit).
//!
//! The SVG masks are blurred-ellipse alpha (a `mask: url()` at a whole
//! document is an image mask: the alpha channel, not luminance): a binary
//! ellipse raster blurred by a true separable Gaussian at quarter resolution
//! (σ is 120/170 device px at density 2: smooth) and bilinearly upsampled.
//! Whether the true Gaussian or the SVG three-box approximation matches
//! WebKit best is decided by the phase-2 profile judgment.

/// The paper's tile: 1 px `--chart-grid` lines every 50 px, the tile opening
/// half a px before its line (`-0.5px 0 / 50px 100%`).
pub const PAPER_STEP: f64 = 50.0;
/// The line's width in CSS px.
pub const PAPER_LINE: f64 = 1.0;
/// The tile's offset: lines sit at band `50k - 0.5`.
pub const PAPER_OFFSET: f64 = -0.5;
/// `.paper_x` is 1600 px wide, centred on the hero (`left: calc(50% - 800px)`
/// puts its left at `hero.x + 420 - 800`).
pub const PAPER_X_WIDTH: f64 = 1600.0;
/// `.paper_y` is 1100 px tall, centred on the hero (`top: calc(50% - 550px)`
/// puts its top at `hero.y + 260 - 550`).
pub const PAPER_Y_HEIGHT: f64 = 1100.0;

/// `.paper_x`'s `mask-image` stops: `(px along the 1600 px band, alpha)`.
/// Double stops (750/850) are the 0.5 plateau; both ends are transparent.
pub const PAPER_X_STOPS: &[(f64, f64)] = &[
    (0.0, 0.0),
    (50.0, 0.08),
    (100.0, 0.14),
    (250.0, 0.28),
    (300.0, 0.31),
    (350.0, 0.35),
    (450.0, 0.41),
    (600.0, 0.47),
    (750.0, 0.5),
    (850.0, 0.5),
    (1000.0, 0.47),
    (1150.0, 0.41),
    (1250.0, 0.35),
    (1300.0, 0.31),
    (1350.0, 0.28),
    (1500.0, 0.14),
    (1550.0, 0.08),
    (1600.0, 0.0),
];

/// `.paper_y`'s `mask-image` stops along its 1100 px band; the plateau runs
/// 500 to 600 px.
pub const PAPER_Y_STOPS: &[(f64, f64)] = &[
    (0.0, 0.0),
    (50.0, 0.1),
    (100.0, 0.18),
    (250.0, 0.36),
    (350.0, 0.44),
    (500.0, 0.5),
    (600.0, 0.5),
    (750.0, 0.44),
    (850.0, 0.36),
    (1000.0, 0.18),
    (1050.0, 0.1),
    (1100.0, 0.0),
];

/// `.light_top`'s box relative to the hero's top-left (`top: -300px;
/// left: -240px`), 1320 x 640.
pub const LIGHT_TOP_AT: [f64; 2] = [-240.0, -300.0];
/// `.light_top`'s size.
pub const LIGHT_TOP_SIZE: [f64; 2] = [1320.0, 640.0];
/// `.light_body`'s box relative to the hero (`top: -300px; left: -380px`),
/// 1600 x 1240.
pub const LIGHT_BODY_AT: [f64; 2] = [-380.0, -300.0];
/// `.light_body`'s size.
pub const LIGHT_BODY_SIZE: [f64; 2] = [1600.0, 1240.0];

/// The `to right in srgb` gradient's colour stops along the top light's box:
/// `--param-a` at 180 px, `--param-b` at 708 px, `--param-h` at 1140 px, the
/// ends clamped to the nearer stop.
pub const TOP_STOP_A: f64 = 180.0;
/// See [`TOP_STOP_A`].
pub const TOP_STOP_B: f64 = 708.0;
/// See [`TOP_STOP_A`].
pub const TOP_STOP_H: f64 = 1140.0;

/// `.light_top`'s opacity; `.light_body`'s is [`LIGHT_BODY_OPACITY`].
pub const LIGHT_TOP_OPACITY: f64 = 0.3;
/// See [`LIGHT_TOP_OPACITY`].
pub const LIGHT_BODY_OPACITY: f64 = 0.35;

/// The hero plane's content box at fit 1: the 840 x 520 frame inside its 1 px
/// border, where the bare plane draws (the label test's box).
pub const HERO_CONTENT: [f64; 2] = [838.0, 518.0];

/// An SVG mask ellipse in its box's local CSS px (`light_top.svg`: cx 660
/// cy 320 rx 480 ry 140; `light_body.svg`: cx 800 cy 620 rx 540 ry 360).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MaskEllipse {
    /// The centre.
    pub at: [f64; 2],
    /// The radii.
    pub radii: [f64; 2],
}

/// The top light's mask ellipse.
pub const TOP_ELLIPSE: MaskEllipse = MaskEllipse {
    at: [660.0, 320.0],
    radii: [480.0, 140.0],
};
/// The top mask's `feGaussianBlur` standard deviation in CSS px.
pub const TOP_SIGMA: f64 = 60.0;
/// The body light's mask ellipse.
pub const BODY_ELLIPSE: MaskEllipse = MaskEllipse {
    at: [800.0, 620.0],
    radii: [540.0, 360.0],
};
/// The body mask's blur in CSS px.
pub const BODY_SIGMA: f64 = 85.0;

/// The mask grid's cell in screen px: the blur is computed at reduced
/// resolution (σ is 30/42.5 cells: smooth) and bilinearly upsampled.
pub const MASK_CELL: f64 = 2.0;
/// The Gaussian kernel's half-width in sigmas (renormalised, so the 3e-5
/// tail loss never shows).
pub const KERNEL_SIGMAS: f64 = 4.0;

/// A mask stop table's alpha at `x`: piecewise linear between stops, 0 past
/// either end (both tables end transparent).
pub fn mask_alpha(stops: &[(f64, f64)], x: f64) -> f64 {
    let mut below = None;
    for (edge, alpha) in stops {
        if x < *edge {
            if let Some((lo, lo_alpha)) = below {
                let span = edge - lo;
                return lo_alpha + (alpha - lo_alpha) * (x - lo) / span;
            }
            return 0.0;
        }
        below = Some((*edge, *alpha));
    }
    0.0
}

/// The sRGB "over" operator, as the browser composites it: per-channel
/// `src * alpha + dst * (1 - alpha)` on the encoded values, unclipped (the
/// gradient's out-of-gamut ends interpolate before any gamut map).
pub fn srgb_over(src: [f32; 3], alpha: f32, dst: [f32; 3]) -> [f32; 3] {
    [
        src[0] * alpha + dst[0] * (1.0 - alpha),
        src[1] * alpha + dst[1] * (1.0 - alpha),
        src[2] * alpha + dst[2] * (1.0 - alpha),
    ]
}

/// `.light_top`'s fill at `x` px along its box: the nearer end stop outside
/// 180/1140 px, the linear `in srgb` ramp between.
pub fn top_gradient(x: f64, a: [f32; 3], b: [f32; 3], h: [f32; 3]) -> [f32; 3] {
    if x <= TOP_STOP_A {
        return a;
    }
    if x >= TOP_STOP_H {
        return h;
    }
    let (from, to, lo, hi) = if x <= TOP_STOP_B {
        (a, b, TOP_STOP_A, TOP_STOP_B)
    } else {
        (b, h, TOP_STOP_B, TOP_STOP_H)
    };
    let t = ((x - lo) / (hi - lo)) as f32;
    [
        from[0] + (to[0] - from[0]) * t,
        from[1] + (to[1] - from[1]) * t,
        from[2] + (to[2] - from[2]) * t,
    ]
}

/// A mask ellipse's coverage at a point: 1 inside, 0 outside. Binary (the
/// SVG shape edge's own antialiasing is a sub-pixel step under σ 60/85).
pub fn ellipse_alpha(ellipse: &MaskEllipse, x: f64, y: f64) -> f64 {
    let dx = (x - ellipse.at[0]) / ellipse.radii[0];
    let dy = (y - ellipse.at[1]) / ellipse.radii[1];
    if dx * dx + dy * dy <= 1.0 { 1.0 } else { 0.0 }
}

/// A 1D Gaussian kernel for `sigma` (in cells), cut at [`KERNEL_SIGMAS`]
/// sigmas and renormalised to sum 1.
pub fn gaussian_kernel(sigma: f64) -> Vec<f64> {
    let radius = (KERNEL_SIGMAS * sigma).ceil() as usize;
    let mut kernel: Vec<f64> = (0..=2 * radius)
        .map(|i| {
            let d = i as f64 - radius as f64;
            (-d * d / (2.0 * sigma * sigma)).exp()
        })
        .collect();
    let sum: f64 = kernel.iter().sum();
    for k in &mut kernel {
        *k /= sum;
    }
    kernel
}

/// A separable Gaussian blur of a row-major `w` x `h` grid (horizontal pass,
/// then vertical, same kernel).
pub fn separable_blur(src: &[f32], w: usize, h: usize, sigma: f64) -> Vec<f32> {
    let kernel = gaussian_kernel(sigma);
    let radius = kernel.len() / 2;
    let mut mid = vec![0.0f32; src.len()];
    for (y, row) in mid.chunks_mut(w).enumerate() {
        for (x, cell) in row.iter_mut().enumerate() {
            let mut acc = 0.0;
            for (k, weight) in kernel.iter().enumerate() {
                let sx = x as isize + k as isize - radius as isize;
                let clamped = sx.clamp(0, w as isize - 1) as usize;
                acc += f64::from(src[y * w + clamped]) * weight;
            }
            *cell = acc as f32;
        }
    }
    let mut dst = vec![0.0f32; src.len()];
    for (y, row) in dst.chunks_mut(w).enumerate() {
        for (x, cell) in row.iter_mut().enumerate() {
            let mut acc = 0.0;
            for (k, weight) in kernel.iter().enumerate() {
                let sy = y as isize + k as isize - radius as isize;
                let clamped = sy.clamp(0, h as isize - 1) as usize;
                acc += f64::from(mid[clamped * w + x]) * weight;
            }
            *cell = acc as f32;
        }
    }
    dst
}

/// A blurred-ellipse alpha mask over a `box_w` x `box_h` box (either light's,
/// in screen px): the binary ellipse rasterised at [`MASK_CELL`] px cells,
/// blurred with `sigma` (screen px), sampled with [`sample_mask`].
#[derive(Clone, Debug, PartialEq)]
pub struct LightMask {
    /// Grid cells across/along.
    pub size: [usize; 2],
    /// The cell in screen px.
    pub cell: f64,
    /// Row-major blurred alpha.
    pub data: Vec<f32>,
}

/// See [`render_light_mask`].
pub fn render_light_mask(
    box_size: [f64; 2],
    ellipse: &MaskEllipse,
    sigma: f64,
    cell: f64,
) -> LightMask {
    let w = (box_size[0] / cell).ceil() as usize;
    let h = (box_size[1] / cell).ceil() as usize;
    // The caller passes the box and the ellipse in the same (screen) space;
    // the raster samples the ellipse at the cells' centres.
    let raster: Vec<f32> = (0..h)
        .flat_map(|y| {
            (0..w).map(move |x| {
                ellipse_alpha(ellipse, (x as f64 + 0.5) * cell, (y as f64 + 0.5) * cell) as f32
            })
        })
        .collect();
    let data = separable_blur(&raster, w, h, sigma / cell);
    LightMask {
        size: [w, h],
        cell,
        data,
    }
}

/// Scales a mask ellipse by the fit into screen px.
pub fn scale_ellipse(ellipse: &MaskEllipse, fit: f64) -> MaskEllipse {
    MaskEllipse {
        at: [ellipse.at[0] * fit, ellipse.at[1] * fit],
        radii: [ellipse.radii[0] * fit, ellipse.radii[1] * fit],
    }
}

/// A mask's alpha at screen point (`x`, `y`) from the box's top-left:
/// bilinear over the grid, 0 outside.
pub fn sample_mask(mask: &LightMask, x: f64, y: f64) -> f32 {
    let gx = x / mask.cell - 0.5;
    let gy = y / mask.cell - 0.5;
    let [w, h] = mask.size;
    let bilinear = |x0: isize, y0: isize, fx: f64, fy: f64| {
        let at = |dx: isize, dy: isize| {
            let sx = (x0 + dx).clamp(0, w as isize - 1) as usize;
            let sy = (y0 + dy).clamp(0, h as isize - 1) as usize;
            f64::from(mask.data[sy * w + sx])
        };
        at(0, 0) * (1.0 - fx) * (1.0 - fy)
            + at(1, 0) * fx * (1.0 - fy)
            + at(0, 1) * (1.0 - fx) * fy
            + at(1, 1) * fx * fy
    };
    if gx < -1.0 || gy < -1.0 || gx > w as f64 || gy > h as f64 {
        return 0.0;
    }
    bilinear(
        gx.floor() as isize,
        gy.floor() as isize,
        gx.fract(),
        gy.fract(),
    ) as f32
}

/// The colours the composite folds, as extended sRGB triplets in 0..1 (the
/// theme's resolution; `--chart-glow` carries its `/ 0.4`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BackdropColors {
    /// The page background (`--background`).
    pub background: [f32; 3],
    /// The paper ink (`--chart-grid`).
    pub grid: [f32; 3],
    /// The body light's fill (`--chart-glow` with alpha).
    pub glow: [f32; 4],
    /// The top gradient's stops (`--param-a/b/h`, unclipped).
    pub param_a: [f32; 3],
    /// See [`BackdropColors::param_a`].
    pub param_b: [f32; 3],
    /// See [`BackdropColors::param_a`].
    pub param_h: [f32; 3],
}

/// Where the backdrop draws: the hero's box in design px with the window's
/// screen size, the fit and the density.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BackdropGeometry {
    /// The hero (plane frame) box in design px.
    pub hero: [f64; 4],
    /// The window in screen px.
    pub viewport: [f64; 2],
    /// The page scale (`--fit`; 1 until h10-fit wires it).
    pub fit: f64,
    /// The device pixels per screen px.
    pub dpr: f64,
}

/// The composited backdrop: opaque sRGB bytes at device resolution for the
/// visible window, ready to upload through `mv-nvg`'s image API.
#[derive(Clone, Debug, PartialEq)]
pub struct BackdropImage {
    /// Device pixels across/along.
    pub size: [usize; 2],
    /// Row-major opaque RGBA.
    pub rgba: Vec<u8>,
}

/// A 1 px paper line's device columns: the design span `[lo, hi)` at `s`
/// device px per design px covers every column it overlaps (the fit is built
/// so the paper step lands on whole device px, keeping boundaries exact).
pub fn line_device_span(lo: f64, hi: f64, s: f64) -> core::ops::Range<i64> {
    let start = (lo * s).floor() as i64;
    let end = (hi * s - 1e-9).floor() as i64;
    start..end + 1
}

/// Composites the four layers over the page background in sRGB, in DOM
/// order: paper_x, paper_y, light_top, light_body. Device pixel centres map
/// back through the density and the fit into design space; the paper lines
/// are evaluated on the device grid, the masks bilinearly from their reduced
/// grids.
pub fn composite_backdrop(geom: &BackdropGeometry, colors: &BackdropColors) -> BackdropImage {
    let s = geom.dpr * geom.fit;
    let width = (geom.viewport[0] * geom.dpr).round() as usize;
    let height = (geom.viewport[1] * geom.dpr).round() as usize;
    let [hx, hy, hw, hh] = geom.hero;
    // The paper bands in design px: paper_x's left centres the 1600 px band
    // on the hero; its top and bottom hang 50vh/fit past the hero. paper_y's
    // top centres its 1100 px band; its sides run past the window.
    let band_x_left = hx + hw / 2.0 - PAPER_X_WIDTH / 2.0;
    let half_window = geom.viewport[1] / 2.0 / geom.fit;
    let band_x_top = hy - half_window;
    let band_x_bottom = hy + hh + half_window;
    let band_y_top = hy + hh / 2.0 - PAPER_Y_HEIGHT / 2.0;
    // The lights in screen px (design scaled by the fit).
    let top_box = scale_box(
        [
            hx + LIGHT_TOP_AT[0],
            hy + LIGHT_TOP_AT[1],
            LIGHT_TOP_SIZE[0],
            LIGHT_TOP_SIZE[1],
        ],
        geom.fit,
    );
    let body_box = scale_box(
        [
            hx + LIGHT_BODY_AT[0],
            hy + LIGHT_BODY_AT[1],
            LIGHT_BODY_SIZE[0],
            LIGHT_BODY_SIZE[1],
        ],
        geom.fit,
    );
    let top_mask = render_light_mask(
        [top_box[2], top_box[3]],
        &scale_ellipse(&TOP_ELLIPSE, geom.fit),
        TOP_SIGMA * geom.fit,
        MASK_CELL,
    );
    let body_mask = render_light_mask(
        [body_box[2], body_box[3]],
        &scale_ellipse(&BODY_ELLIPSE, geom.fit),
        BODY_SIGMA * geom.fit,
        MASK_CELL,
    );
    let bg = colors.background;
    let mut rgba = Vec::with_capacity(width * height * 4);
    for py in 0..height {
        // The pixel's design position (through its centre).
        let screen_y = (py as f64 + 0.5) / geom.dpr;
        let design_y = screen_y / geom.fit;
        // The paper_y line (if any) crossing this row, in device rows.
        let band_y = design_y - band_y_top;
        let row_line = ((band_y - PAPER_OFFSET) / PAPER_STEP).floor() as i64;
        let row_lo = band_y_top + row_line as f64 * PAPER_STEP + PAPER_OFFSET;
        let row_span = line_device_span(row_lo, row_lo + PAPER_LINE, s);
        let on_row_line = row_span.contains(&(py as i64))
            && (PAPER_OFFSET..=PAPER_Y_HEIGHT - PAPER_OFFSET).contains(&band_y);
        for px in 0..width {
            let screen_x = (px as f64 + 0.5) / geom.dpr;
            let design_x = screen_x / geom.fit;
            let mut c = bg;
            // (1) paper_x: the full window height over the band's width.
            let band_x = design_x - band_x_left;
            if (0.0..=PAPER_X_WIDTH).contains(&band_x)
                && design_y >= band_x_top
                && design_y <= band_x_bottom
            {
                let col_line = ((band_x - PAPER_OFFSET) / PAPER_STEP).floor() as i64;
                let col_lo = band_x_left + col_line as f64 * PAPER_STEP + PAPER_OFFSET;
                if line_device_span(col_lo, col_lo + PAPER_LINE, s).contains(&(px as i64)) {
                    c = srgb_over(colors.grid, mask_alpha(PAPER_X_STOPS, band_x) as f32, c);
                }
            }
            // (2) paper_y: the full window width over the band's height.
            if on_row_line && design_y >= band_y_top && design_y <= band_y_top + PAPER_Y_HEIGHT {
                c = srgb_over(colors.grid, mask_alpha(PAPER_Y_STOPS, band_y) as f32, c);
            }
            // (3) light_top: the srgb gradient under its blurred mask.
            if screen_x >= top_box[0]
                && screen_x < top_box[0] + top_box[2]
                && screen_y >= top_box[1]
                && screen_y < top_box[1] + top_box[3]
            {
                let g = top_gradient(
                    design_x - (hx + LIGHT_TOP_AT[0]),
                    colors.param_a,
                    colors.param_b,
                    colors.param_h,
                );
                let alpha = sample_mask(&top_mask, screen_x - top_box[0], screen_y - top_box[1])
                    * LIGHT_TOP_OPACITY as f32;
                c = srgb_over(g, alpha, c);
            }
            // (4) light_body: --chart-glow under its mask (the token carries
            // its own alpha, folded with the layer opacity).
            if screen_x >= body_box[0]
                && screen_x < body_box[0] + body_box[2]
                && screen_y >= body_box[1]
                && screen_y < body_box[1] + body_box[3]
            {
                let alpha = colors.glow[3]
                    * sample_mask(&body_mask, screen_x - body_box[0], screen_y - body_box[1])
                    * LIGHT_BODY_OPACITY as f32;
                c = srgb_over([colors.glow[0], colors.glow[1], colors.glow[2]], alpha, c);
            }
            rgba.extend_from_slice(&[
                (c[0] * 255.0).round().clamp(0.0, 255.0) as u8,
                (c[1] * 255.0).round().clamp(0.0, 255.0) as u8,
                (c[2] * 255.0).round().clamp(0.0, 255.0) as u8,
                255,
            ]);
        }
    }
    BackdropImage {
        size: [width, height],
        rgba,
    }
}

/// Scales a design box into screen px.
fn scale_box(design: [f64; 4], fit: f64) -> [f64; 4] {
    [
        design[0] * fit,
        design[1] * fit,
        design[2] * fit,
        design[3] * fit,
    ]
}

/// The hero plane's labels: the bare plane's axis texts at the default camera
/// in the content box, with no scale legend (the hero draws through
/// `drawCartesianPlane` without the viewer's legend wrapper). Ported from
/// `WelcomePage.test.tsx`'s first case against the label list.
pub fn hero_label_texts(
    center: [f64; 2],
    scale: f64,
    label_width: &dyn Fn(&str) -> f64,
) -> Vec<String> {
    let grid = crate::grid::layout_grid(
        center,
        scale,
        HERO_CONTENT[0],
        HERO_CONTENT[1],
        crate::plane::LABEL_MARGIN,
    );
    crate::grid::layout_labels(&grid, crate::plane::PLANE_FONT_SIZE, label_width)
        .into_iter()
        .map(|label| label.text)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The mono advance the plane's grid tests pin: a char is 8.4 px at 14 px.
    fn probe_width(text: &str) -> f64 {
        text.len() as f64 * 8.4
    }

    /// The stop tables read back at their stops, between them and past them.
    #[test]
    fn mask_tables_interpolate_piecewise_linear() {
        for (stops, end) in [(PAPER_X_STOPS, 1600.0), (PAPER_Y_STOPS, 1100.0)] {
            assert_eq!(mask_alpha(stops, 0.0), 0.0, "transparent at the start");
            assert_eq!(mask_alpha(stops, end), 0.0, "transparent at the end");
            assert_eq!(mask_alpha(stops, -1.0), 0.0, "0 past the start");
            assert_eq!(mask_alpha(stops, end + 1.0), 0.0, "0 past the end");
        }
        assert_eq!(mask_alpha(PAPER_X_STOPS, 750.0), 0.5, "plateau opens");
        assert_eq!(mask_alpha(PAPER_X_STOPS, 800.0), 0.5, "plateau holds");
        assert_eq!(mask_alpha(PAPER_X_STOPS, 850.0), 0.5, "plateau closes");
        let mid = mask_alpha(PAPER_X_STOPS, 875.0);
        assert!(
            (mid - 0.495).abs() < 1e-12,
            "875 between 850/0.5 and 1000/0.47: {mid}"
        );
        assert!(
            (mask_alpha(PAPER_X_STOPS, 49.5) - 0.0792).abs() < 1e-12,
            "first line"
        );
        assert_eq!(mask_alpha(PAPER_Y_STOPS, 500.0), 0.5, "y plateau opens");
        assert_eq!(mask_alpha(PAPER_Y_STOPS, 600.0), 0.5, "y plateau closes");
        assert!(
            (mask_alpha(PAPER_Y_STOPS, 149.5) - 0.2394).abs() < 1e-9,
            "y at 149.5"
        );
    }

    /// The sRGB over operator against hand-computed values.
    #[test]
    fn srgb_over_blends_in_encoded_space() {
        assert_eq!(
            srgb_over([1.0, 0.0, 0.0], 0.5, [0.0, 0.0, 1.0]),
            [0.5, 0.0, 0.5]
        );
        assert_eq!(
            srgb_over([0.2, 0.4, 0.6], 0.0, [0.7, 0.8, 0.9]),
            [0.7, 0.8, 0.9]
        );
        assert_eq!(
            srgb_over([0.2, 0.4, 0.6], 1.0, [0.7, 0.8, 0.9]),
            [0.2, 0.4, 0.6]
        );
        // Unclipped: the gradient's out-of-gamut ends interpolate before any
        // gamut map, so the operator never clips.
        assert_eq!(
            srgb_over([2.0, 0.0, 0.0], 0.5, [0.0, 0.0, 0.0]),
            [1.0, 0.0, 0.0]
        );
    }

    /// The top gradient clamps its ends and ramps linearly between.
    #[test]
    fn top_gradient_clamps_and_ramps() {
        let (a, b, h) = ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]);
        assert_eq!(top_gradient(0.0, a, b, h), a, "before the first stop");
        assert_eq!(top_gradient(180.0, a, b, h), a, "at the first stop");
        assert_eq!(top_gradient(708.0, a, b, h), b, "at the middle stop");
        assert_eq!(top_gradient(1140.0, a, b, h), h, "at the last stop");
        assert_eq!(top_gradient(1320.0, a, b, h), h, "past the last stop");
        assert_eq!(
            top_gradient(444.0, a, b, h),
            [0.5, 0.5, 0.0],
            "midway a to b"
        );
    }

    /// The mask's alpha at the ellipse's centre and far outside.
    #[test]
    fn mask_alpha_is_one_at_the_centre_and_zero_far_out() {
        assert_eq!(ellipse_alpha(&TOP_ELLIPSE, 660.0, 320.0), 1.0, "top centre");
        assert_eq!(
            ellipse_alpha(&BODY_ELLIPSE, 800.0, 620.0),
            1.0,
            "body centre"
        );
        assert_eq!(ellipse_alpha(&TOP_ELLIPSE, 0.0, 0.0), 0.0, "top corner");
        assert_eq!(ellipse_alpha(&BODY_ELLIPSE, 0.0, 0.0), 0.0, "body corner");
        assert_eq!(
            ellipse_alpha(&TOP_ELLIPSE, 660.0, 0.0),
            0.0,
            "past the minor radius"
        );
    }

    /// A blurred delta keeps its sum and peaks at the Gaussian's height.
    #[test]
    fn gaussian_blur_keeps_the_sum() {
        let (w, h) = (21, 21);
        let mut delta = vec![0.0f32; w * h];
        delta[10 * w + 10] = 1.0;
        let out = separable_blur(&delta, w, h, 2.0);
        let sum: f32 = out.iter().sum();
        assert!(
            (sum - 1.0).abs() < 1e-5,
            "renormalised kernel keeps the sum: {sum}"
        );
        let peak = out[10 * w + 10];
        let height = 1.0 / (2.0 * core::f64::consts::PI * 4.0);
        assert!(
            (f64::from(peak) - height).abs() < 1e-3,
            "peak at 1/2πσ²: {peak}"
        );
    }

    /// A 1 px design line at density 2 covers exactly two device columns.
    #[test]
    fn line_span_covers_its_device_columns() {
        assert_eq!(
            line_device_span(189.5, 190.5, 2.0),
            379..381,
            "first paper_x line"
        );
        assert_eq!(
            line_device_span(49.5, 50.5, 2.0),
            99..101,
            "paper_y line row"
        );
    }

    /// The hero's labels, ported from `WelcomePage.test.tsx`'s first case:
    /// the bare plane at the default camera in the 838 x 518 content box
    /// numbers -8 to 8 across and -5 to 5 up, with no scale legend text.
    #[test]
    fn hero_plane_labels_run_minus8_to_8_with_no_legend() {
        let texts = hero_label_texts([0.0, 0.0], 0.5, &probe_width);
        for text in ["-8", "-5", "-1", "0", "1", "5", "8"] {
            assert!(texts.contains(&text.to_string()), "shows {text}: {texts:?}");
        }
        assert!(
            !texts.iter().any(|text| text.ends_with(" u")),
            "no scale legend on the hero: {texts:?}"
        );
    }

    /// The composite is opaque and deterministic on a small window.
    #[test]
    fn composite_is_opaque_and_deterministic() {
        let geom = BackdropGeometry {
            hero: [40.0, 30.0, 120.0, 80.0],
            viewport: [160.0, 100.0],
            fit: 1.0,
            dpr: 1.0,
        };
        let colors = BackdropColors {
            background: [0.03, 0.05, 0.07],
            grid: [0.14, 0.17, 0.21],
            glow: [0.44, 0.75, 0.63, 0.4],
            param_a: [1.0, 0.68, 0.54],
            param_b: [0.77, 0.71, 0.99],
            param_h: [0.48, 0.65, 0.84],
        };
        let first = composite_backdrop(&geom, &colors);
        assert_eq!(first.size, [160, 100]);
        assert_eq!(first.rgba.len(), 160 * 100 * 4);
        assert!(
            first
                .rgba
                .iter()
                .skip(3)
                .step_by(4)
                .all(|alpha| *alpha == 255),
            "opaque"
        );
        let second = composite_backdrop(&geom, &colors);
        assert_eq!(first, second, "deterministic");
    }
}
