//! Zoom and pan maths: viewports, conversions, and zoom steps.

use super::faces::js_round;

/// The zoom that fits the default view.
pub const BASE_ZOOM: f64 = 50.0;

/// The scale bar's target length in px.
pub const SCALE_BAR_PX: f64 = 50.0;

/// The smallest grid unit, in math units.
pub const MIN_UNIT_SIZE: f64 = 0.25;

/// The largest grid unit, in math units.
pub const MAX_UNIT_SIZE: f64 = 100.0;

/// The default half height of the fitted y-range.
pub const DEFAULT_FIT_HALF_RANGE_Y: f64 = 5.0;

/// A math extent: the visible x and y ranges.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Extent {
    pub min_x: f64,
    pub max_x: f64,
    pub min_y: f64,
    pub max_y: f64,
}

/// A saved view: its zoom and extent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    pub zoom: f64,
    pub extent: Extent,
}

/// A view in motion: its zoom and pan.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewState {
    pub zoom: f64,
    pub pan_x: f64,
    pub pan_y: f64,
}

/// A laid-out viewport: its size, zoom and origin.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    pub width: f64,
    pub height: f64,
    pub zoom: f64,
    pub origin_x: f64,
    pub origin_y: f64,
}

/// Builds a viewport from a size, a density and a view state.
pub fn make_viewport(width: f64, height: f64, dpr: f64, view: ViewState) -> Viewport {
    let snap = |v: f64| js_round(v * dpr) / dpr;
    Viewport {
        width,
        height,
        zoom: view.zoom,
        origin_x: snap((width / 2.0).floor() + view.pan_x * view.zoom),
        origin_y: snap((height / 2.0).floor() - view.pan_y * view.zoom),
    }
}

/// Math x to screen px.
pub fn to_screen_x(vp: Viewport, x: f64) -> f64 {
    vp.origin_x + x * vp.zoom
}

/// Math y to screen px.
pub fn to_screen_y(vp: Viewport, y: f64) -> f64 {
    vp.origin_y - y * vp.zoom
}

/// Screen px to math x.
pub fn to_math_x(vp: Viewport, px: f64) -> f64 {
    (px - vp.origin_x) / vp.zoom
}

/// Screen py to math y.
pub fn to_math_y(vp: Viewport, py: f64) -> f64 {
    (vp.origin_y - py) / vp.zoom
}

/// The visible math extent of a viewport.
pub fn visible_extent(vp: Viewport) -> Extent {
    Extent {
        min_x: to_math_x(vp, 0.0),
        max_x: to_math_x(vp, vp.width),
        min_y: to_math_y(vp, vp.height),
        max_y: to_math_y(vp, 0.0),
    }
}

/// The viewport of a saved view.
pub fn view_of(view: View) -> Viewport {
    let e = view.extent;
    Viewport {
        width: (e.max_x - e.min_x) * view.zoom,
        height: (e.max_y - e.min_y) * view.zoom,
        zoom: view.zoom,
        origin_x: -e.min_x * view.zoom,
        origin_y: e.max_y * view.zoom,
    }
}

/// Pans a view state to bring a point into view.
pub fn pan_into_view(vp: Viewport, view: ViewState, point: [f64; 2]) -> ViewState {
    let towards = |s: f64, size: f64| {
        if s < 0.0 {
            size / 4.0 - s
        } else if s > size {
            size * 3.0 / 4.0 - s
        } else {
            0.0
        }
    };
    let sx = to_screen_x(vp, point[0]);
    let sy = to_screen_y(vp, point[1]);
    // Screen x follows pan_x; screen y moves against pan_y.
    ViewState {
        zoom: view.zoom,
        pan_x: view.pan_x + towards(sx, vp.width) / vp.zoom,
        pan_y: view.pan_y - towards(sy, vp.height) / vp.zoom,
    }
}

/// A zoom as a percent.
pub fn zoom_percent(zoom: f64) -> f64 {
    zoom / BASE_ZOOM * 100.0
}

/// A zoom as a label.
pub fn zoom_label(zoom: f64) -> String {
    let percent = zoom_percent(zoom);
    format!(
        "{}%",
        mv_numfmt::format_mark(percent, if percent < 10.0 { 1 } else { 0 })
    )
}

/// The stepped zooms, ascending.
pub static ZOOM_STOPS: [f64; 27] = [
    0.25, 0.315, 0.4, 0.5, 0.625, 0.8, 1.0, 1.25, 1.6, 2.0, 2.5, 3.15, 4.0, 5.0, 6.25, 8.0, 10.0,
    12.5, 16.0, 20.0, 25.0, 31.5, 40.0, 50.0, 62.5, 80.0, 100.0,
];

/// Steps a zoom up or down the stops.
pub fn step_zoom(zoom: f64, direction: i64) -> f64 {
    const STOP_TOLERANCE: f64 = 1e-3;
    if direction > 0 {
        for stop in ZOOM_STOPS {
            if stop > zoom * (1.0 + STOP_TOLERANCE) {
                return stop;
            }
        }
    } else {
        for stop in ZOOM_STOPS.into_iter().rev() {
            if stop < zoom * (1.0 - STOP_TOLERANCE) {
                return stop;
            }
        }
    }
    zoom
}

/// The zoom that fits a half y-range to a height.
pub fn default_zoom(height: f64, fit_half_range_y: f64) -> f64 {
    if height <= 0.0 || fit_half_range_y <= 0.0 {
        return BASE_ZOOM;
    }
    let fit = BASE_ZOOM.min(height / (2.0 * fit_half_range_y));
    for stop in ZOOM_STOPS.into_iter().rev() {
        if stop <= fit * (1.0 + 1e-9) {
            return stop;
        }
    }
    MIN_UNIT_SIZE
}

/// The scale bar's units at a zoom.
pub fn scale_bar_units(zoom: f64) -> f64 {
    SCALE_BAR_PX / zoom
}

/// The scale bar's label at a zoom.
pub fn scale_label(zoom: f64) -> String {
    format!("{} u", mv_numfmt::format_mark(scale_bar_units(zoom), 2))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(zoom: f64, pan_x: f64, pan_y: f64) -> ViewState {
        ViewState { zoom, pan_x, pan_y }
    }

    #[test]
    fn the_origin_sits_at_the_floored_centre() {
        let vp = make_viewport(936.0, 792.0, 1.0, view(50.0, 0.0, 0.0));
        assert_eq!(vp.origin_x, 468.0);
        assert_eq!(vp.origin_y, 396.0);
        assert_eq!(to_screen_x(vp, 1.0), 518.0);
        assert_eq!(to_screen_y(vp, 1.0), 346.0);
        assert_eq!(to_math_x(vp, 518.0), 1.0);
        assert_eq!(to_math_y(vp, 346.0), 1.0);
    }

    #[test]
    fn the_visible_range_follows_zoom_and_pan() {
        assert_eq!(
            visible_extent(make_viewport(1000.0, 800.0, 1.0, view(50.0, 0.0, 0.0))),
            Extent {
                min_x: -10.0,
                max_x: 10.0,
                min_y: -8.0,
                max_y: 8.0,
            }
        );
        // The pan is the maths offset of the view's centre: +2 in x shows -12 … 8.
        assert_eq!(
            visible_extent(make_viewport(1000.0, 800.0, 1.0, view(50.0, 2.0, 1.0))),
            Extent {
                min_x: -12.0,
                max_x: 8.0,
                min_y: -9.0,
                max_y: 7.0,
            }
        );
        // Degenerate at zero size, before the first layout.
        assert_eq!(
            visible_extent(make_viewport(0.0, 0.0, 1.0, view(50.0, 0.0, 0.0))),
            Extent {
                min_x: 0.0,
                max_x: 0.0,
                min_y: 0.0,
                max_y: 0.0,
            }
        );
        // R2's plane at 1440 x 900: 936 x 792 at 50 px per unit.
        assert_eq!(
            visible_extent(make_viewport(936.0, 792.0, 2.0, view(50.0, 0.0, 0.0))),
            Extent {
                min_x: -9.36,
                max_x: 9.36,
                min_y: -7.92,
                max_y: 7.92,
            }
        );
    }

    #[test]
    fn the_origin_lands_on_whole_device_pixels() {
        assert_eq!(
            make_viewport(936.0, 792.0, 1.0, view(50.0, 0.013, 0.0)).origin_x,
            469.0
        );
        assert_eq!(
            make_viewport(936.0, 792.0, 2.0, view(50.0, 0.013, 0.0)).origin_x,
            468.5
        );
    }

    #[test]
    fn the_zoom_stops_are_the_r10_numbers() {
        // viewport.ts builds them: R10 per decade, as a percentage of BASE_ZOOM, within the limits.
        let r10 = [1.0, 1.25, 1.6, 2.0, 2.5, 3.2, 4.0, 5.0, 6.3, 8.0];
        let mut built = Vec::new();
        for decade in -1..=2 {
            for m in r10 {
                let zoom = mv_numfmt::precision(m * 10f64.powi(decade) * BASE_ZOOM / 100.0, 12);
                if (MIN_UNIT_SIZE..=MAX_UNIT_SIZE).contains(&zoom) {
                    built.push(zoom);
                }
            }
        }
        assert_eq!(built.len(), ZOOM_STOPS.len());
        for (i, stop) in ZOOM_STOPS.into_iter().enumerate() {
            assert_eq!(stop, built[i]);
        }
        assert_eq!(ZOOM_STOPS[0], 0.25);
        assert_eq!(ZOOM_STOPS[ZOOM_STOPS.len() - 1], 100.0);
        let mut percents = Vec::new();
        for stop in ZOOM_STOPS {
            let p = zoom_percent(stop).round();
            if p >= 50.0 {
                percents.push(p);
            }
        }
        assert_eq!(percents.len(), 7);
        for (i, p) in [50.0, 63.0, 80.0, 100.0, 125.0, 160.0, 200.0]
            .into_iter()
            .enumerate()
        {
            assert_eq!(percents[i], p);
        }
    }

    #[test]
    fn the_default_zoom_keeps_five_units_in_view() {
        assert_eq!(default_zoom(792.0, 5.0), 50.0);
        // R9's plane: 408 px tall, 40 px per unit, 80 %.
        assert_eq!(default_zoom(408.0, 5.0), 40.0);
        assert_eq!(zoom_percent(default_zoom(408.0, 5.0)), 80.0);
        assert_eq!(default_zoom(300.0, 5.0), 25.0);
        // 100 % before the first layout.
        assert_eq!(default_zoom(0.0, 5.0), 50.0);
    }

    #[test]
    fn the_zoom_steps_to_the_next_stop() {
        assert_eq!(step_zoom(50.0, 1), 62.5);
        assert_eq!(step_zoom(50.0, -1), 40.0);
        assert_eq!(step_zoom(45.0, 1), 50.0);
        assert_eq!(step_zoom(45.0, -1), 40.0);
        assert_eq!(step_zoom(100.0, 1), 100.0);
        assert_eq!(step_zoom(0.25, -1), 0.25);
    }

    #[test]
    fn the_labels_measure_the_scale_bar_and_print_the_zoom() {
        // 50 px of scale bar: 1 u at 100 %, 1.25 u at 80 % (R9).
        assert_eq!(scale_bar_units(50.0), 1.0);
        assert_eq!(scale_bar_units(40.0), 1.25);
        assert_eq!(scale_label(50.0), "1 u");
        assert_eq!(scale_label(40.0), "1.25 u");
        assert_eq!(scale_label(100.0), "0.5 u");
        assert_eq!(scale_label(37.0), "1.35 u");
        // The zoom in whole percent, one decimal below 10 %.
        assert_eq!(zoom_label(50.0), "100%");
        assert_eq!(zoom_label(40.0), "80%");
        assert_eq!(zoom_label(31.5), "63%");
        assert_eq!(zoom_label(3.15), "6.3%");
        assert_eq!(zoom_label(0.25), "0.5%");
    }

    #[test]
    fn pan_into_view_brings_a_point_a_quarter_in() {
        let vp = make_viewport(936.0, 792.0, 1.0, view(50.0, 0.0, 0.0));
        // FV 10 X1: x³'s P (2, 8) lies 4 px above the view.
        let next = pan_into_view(vp, view(50.0, 0.0, 0.0), [2.0, 8.0]);
        assert_eq!(next.pan_x, 0.0);
        let moved = make_viewport(936.0, 792.0, 1.0, next);
        assert!((moved.origin_y - 8.0 * 50.0 - 792.0 / 4.0).abs() < 1e-9);
        // Past the right edge: a quarter in from it, the other axis kept.
        let right = pan_into_view(vp, view(50.0, 0.0, 1.0), [12.0, 0.0]);
        assert_eq!(right.pan_y, 1.0);
        let moved = make_viewport(936.0, 792.0, 1.0, right);
        assert!((moved.origin_x + 12.0 * 50.0 - 936.0 * 3.0 / 4.0).abs() < 1e-9);
        let left = pan_into_view(vp, view(50.0, 0.0, 0.0), [-20.0, -9.0]);
        let both = make_viewport(936.0, 792.0, 1.0, left);
        assert!((both.origin_x - 20.0 * 50.0 - 936.0 / 4.0).abs() < 1e-9);
        assert!((both.origin_y + 9.0 * 50.0 - 792.0 * 3.0 / 4.0).abs() < 1e-9);
        // A point in view stays where it is.
        assert_eq!(
            pan_into_view(vp, view(50.0, 0.5, -1.0), [2.0, 4.0]),
            view(50.0, 0.5, -1.0)
        );
    }

    #[test]
    fn view_of_rebuilds_the_viewport_an_owner_hears_about() {
        for vp in [
            make_viewport(936.0, 792.0, 1.0, view(50.0, 0.0, 0.0)),
            make_viewport(1256.0, 408.0, 2.0, view(40.0, 1.3, -0.7)),
        ] {
            let back = view_of(View {
                zoom: vp.zoom,
                extent: visible_extent(vp),
            });
            assert!((back.width - vp.width).abs() < 1e-9);
            assert!((back.height - vp.height).abs() < 1e-9);
            assert!((back.origin_x - vp.origin_x).abs() < 1e-9);
            assert!((back.origin_y - vp.origin_y).abs() < 1e-9);
            assert_eq!(back.zoom, vp.zoom);
        }
    }
}
