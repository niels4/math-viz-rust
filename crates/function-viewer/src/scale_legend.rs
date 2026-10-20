//! The plane's scale legend: the `0.50 u` plate in the canvas's
//! bottom-right corner.
//!
//! Ported from `src/components/views/CartesianPlane/drawScaleLegend.ts` at
//! 251c080: since `1cfd34a` the entry takes the grid (`{ width, height,
//! scale }`), like the painters. The geometry is pure ([`legend_geometry`],
//! tested against the reference's draw records); [`paint_legend`] draws it
//! through NanoVG and the text crate. The bar is always [`BAR_WIDTH`] wide:
//! it is `unscaledUnitSize`, never zoom-scaled.

use mv_text::{Face, Features, Style};

use crate::camera::UNSCALED_UNIT_SIZE;
use crate::grid::GridLayout;

/// The plate sits this far left of the canvas's right edge.
pub const PLATE_RIGHT: f64 = 28.0;

/// The plate sits this far above the canvas's bottom edge.
pub const PLATE_BOTTOM: f64 = 22.0;

/// The plate's padding, x and y.
pub const PAD_X: f64 = 14.0;

/// The plate's padding, x and y.
pub const PAD_Y: f64 = 8.0;

/// The plate's corner radius.
pub const RADIUS: f64 = 4.0;

/// Space between the bar and the label.
pub const TEXT_GAP: f64 = 8.0;

/// The label's line height in the plane font, which sets the plate's height.
pub const TEXT_LINE_HEIGHT: f64 = 18.0;

/// The bar's height.
pub const BAR_HEIGHT: f64 = 5.0;

/// The bar's width: `unscaledUnitSize` (`constants.ts` single-sources it
/// through `camera.rs`, as `grid.rs` does).
pub const BAR_WIDTH: f64 = UNSCALED_UNIT_SIZE;

/// The plate's height: the line height plus padding.
pub const WINDOW_HEIGHT: f64 = TEXT_LINE_HEIGHT + 2.0 * PAD_Y;

/// The legend's label: `formatFixed(2, scale)` plus `" u"` (`format.ts`).
pub fn legend_text(scale: f64) -> String {
    format!("{} u", mv_numfmt::to_fixed(scale, 2))
}

/// The legend's laid-out geometry in canvas px: the plate, the bar's four
/// bracket points (top-left, bottom-left, bottom-right, top-right), the label
/// and its pen (the left edge, the middle the baseline centres on).
#[derive(Clone, Debug, PartialEq)]
pub struct LegendGeometry {
    pub plate: [f64; 4],
    pub bar: [[f64; 2]; 4],
    pub text: String,
    pub text_x: f64,
    pub text_middle_y: f64,
}

/// Lays out the legend live: `text_width` is the label's measured advance.
/// The canvas dims and the scale come from the grid, which `1cfd34a` lays
/// out once per camera update.
pub fn legend_geometry(grid: &GridLayout, text_width: f64) -> LegendGeometry {
    let text = legend_text(grid.scale);
    let plate_width = 2.0 * PAD_X + BAR_WIDTH + TEXT_GAP + text_width;
    let x = grid.width - PLATE_RIGHT - plate_width;
    let y = grid.height - PLATE_BOTTOM - WINDOW_HEIGHT;
    let middle_y = y + WINDOW_HEIGHT / 2.0;
    let bar_x = x + PAD_X;
    let bar_y = middle_y - BAR_HEIGHT / 2.0;
    LegendGeometry {
        plate: [x, y, plate_width, WINDOW_HEIGHT],
        bar: [
            [bar_x, bar_y],
            [bar_x, bar_y + BAR_HEIGHT],
            [bar_x + BAR_WIDTH, bar_y + BAR_HEIGHT],
            [bar_x + BAR_WIDTH, bar_y],
        ],
        text,
        text_x: bar_x + BAR_WIDTH + TEXT_GAP,
        text_middle_y: middle_y,
    }
}

/// The colours the legend paints with: the plate's rim (`chartGrid`, which
/// carries `--border` in every theme) and fill (`background`), the bar
/// (`foreground`), the label (`foregroundMuted`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LegendColors {
    pub rim: [f32; 4],
    pub fill: [f32; 4],
    pub bar: [f32; 4],
    pub text: [f32; 4],
}

/// The legend's text style: the plane font.
pub fn legend_style() -> Style {
    Style {
        face: Face::RobotoMono500,
        size: crate::plane::PLANE_FONT_SIZE as f32,
        tracking: 0.0,
        features: Features::new(),
    }
}

/// Paints the legend over the plane.
pub fn paint_legend(
    fonts: &mut mv_text::Fonts,
    vg: &mut mv_nvg::Context,
    geometry: &LegendGeometry,
    colors: &LegendColors,
) {
    let [x, y, w, h] = geometry.plate;
    vg.begin_path();
    vg.rounded_rect(x as f32, y as f32, w as f32, h as f32, RADIUS as f32);
    vg.fill_color(colors.fill);
    vg.fill();
    vg.stroke_width(1.0);
    vg.stroke_color(colors.rim);
    vg.stroke();

    vg.begin_path();
    vg.line_cap(mv_nvg::LineCapType::BUTT);
    let [[x0, y0], [x1, y1], [x2, y2], [x3, y3]] = geometry.bar;
    vg.move_to(x0 as f32, y0 as f32);
    vg.line_to(x1 as f32, y1 as f32);
    vg.line_to(x2 as f32, y2 as f32);
    vg.line_to(x3 as f32, y3 as f32);
    vg.stroke_width(2.0);
    vg.stroke_color(colors.bar);
    vg.stroke();

    // Canvas `textBaseline: "middle"`: the em box's vertical centre sits on
    // the anchor, so the baseline sits half the ascent-descent span below it.
    let style = legend_style();
    let metrics = mv_text::metrics(fonts, style.face);
    let baseline =
        geometry.text_middle_y as f32 + (metrics.ascent - metrics.descent) * style.size / 2.0;
    mv_text::draw(
        fonts,
        vg,
        style,
        geometry.text_x as f32,
        baseline,
        &geometry.text,
        colors.text,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::layout_grid;

    /// The legend's grid: the 990 by 786 probe canvas, no margin.
    fn probe_grid(scale: f64) -> GridLayout {
        layout_grid([0.0, 0.0], scale, 990.0, 786.0, 0.0)
    }

    /// The label reads `formatFixed(2, scale)` plus `" u"` at every H2 scale.
    #[test]
    fn legend_text_values() {
        assert_eq!(legend_text(0.5), "0.50 u");
        assert_eq!(legend_text(0.25), "0.25 u");
        assert_eq!(legend_text(1.25), "1.25 u");
        assert_eq!(legend_text(5.0), "5.00 u");
        assert_eq!(legend_text(0.01), "0.01 u");
        assert_eq!(legend_text(100.0), "100.00 u");
        // The binary value decides: 2.675 is 2.6749999... underneath.
        assert_eq!(legend_text(2.675), "2.67 u");
    }

    /// The default legend's geometry against the reference's draw record
    /// (WebKit, `.local/probe/default.json`): the plate, the bar's bracket
    /// and the label's pen. `"0.50 u"` measures 50.41 px there (6 chars at
    /// 8.4 px, the mono advance the grid tests pin).
    #[test]
    fn legend_geometry_matches_probe() {
        let geometry = legend_geometry(&probe_grid(0.5), 50.41);
        let [x, y, w, h] = geometry.plate;
        assert!((x - 850.59).abs() < 0.02, "plate x {x}");
        assert!((y - 730.0).abs() < 0.01, "plate y {y}");
        assert!((w - 111.41).abs() < 0.02, "plate w {w}");
        assert!((h - 34.0).abs() < 0.01, "plate h {h}");
        assert_eq!(geometry.text, "0.50 u");
        let [[x0, y0], [_, _], [x2, _], [_, _]] = geometry.bar;
        assert!((x0 - 864.59).abs() < 0.02, "bar x {x0}");
        assert!((y0 - 744.5).abs() < 0.01, "bar y {y0}");
        assert!((x2 - (x0 + 25.0)).abs() < 1e-9, "bar width {x2}");
        assert!((geometry.text_x - (x2 + 8.0)).abs() < 1e-9, "text x");
        assert!((geometry.text_middle_y - 747.0).abs() < 0.01, "middle y");
    }

    /// The scale-100 legend widens with its text (probe `scale-100.json`:
    /// plate x 833.79, w 128.21 for `"100.00 u"`).
    #[test]
    fn legend_geometry_grows_with_text() {
        let narrow = legend_geometry(&probe_grid(0.5), 50.41);
        let wide = legend_geometry(&probe_grid(100.0), 50.41 + 8.4 * 2.0);
        assert_eq!(wide.text, "100.00 u");
        assert!(
            (wide.plate[2] - narrow.plate[2] - 16.8).abs() < 1e-9,
            "plate grows"
        );
        assert!(
            (narrow.plate[0] - wide.plate[0] - 16.8).abs() < 1e-9,
            "plate stays right"
        );
        assert!((wide.plate[0] - 833.79).abs() < 0.02, "plate x");
    }
}
