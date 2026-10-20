//! Still illustrations: the mini plots the page draws off the plane.

use super::grid::{GridSteps, grid_lines};
use super::marks::Segment;
use super::paint::{Painter, paint_curve, paint_grid_lines, paint_line};
use super::scene::{Curve, Ink};
use super::viewport::{Viewport, to_screen_x, to_screen_y};

/// A mini arrow: from one point to another, in an ink.
#[derive(Clone, Debug, PartialEq)]
pub struct MiniArrow {
    pub from: [f64; 2],
    pub to: [f64; 2],
    pub ink: Ink,
    pub width: f64,
}

/// A mini dot: at a point, with a diameter, fill and optional ring.
#[derive(Clone, Debug, PartialEq)]
pub struct MiniDot {
    pub at: [f64; 2],
    pub diameter: f64,
    pub fill: Ink,
    pub ring: Option<MiniRing>,
}

/// A mini ring: an outline in an ink.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MiniRing {
    pub ink: Ink,
    pub width: f64,
}

/// A mini plot: its own viewport, steps, and curves with arrows and dots.
#[derive(Debug)]
pub struct MiniPlot {
    pub zoom: f64,
    pub origin: [f64; 2],
    pub steps: GridSteps,
    pub axis_width: f64,
    pub curves: Vec<Curve>,
    pub arrows: Vec<MiniArrow>,
    pub dots: Vec<MiniDot>,
}

/// Draws a mini plot in a box of a width and height.
pub fn draw_mini(mut p: Painter<'_>, plot: &MiniPlot, width: f64, height: f64) {
    let vp = Viewport {
        width,
        height,
        zoom: plot.zoom,
        origin_x: plot.origin[0],
        origin_y: plot.origin[1],
    };
    paint_grid_lines(
        p.reborrow(),
        &grid_lines(vp, plot.steps),
        width,
        height,
        plot.axis_width,
    );
    for curve in &plot.curves {
        paint_curve(p.reborrow(), vp, curve);
    }
    for arrow in &plot.arrows {
        let s = Segment {
            x1: to_screen_x(vp, arrow.from[0]),
            y1: to_screen_y(vp, arrow.from[1]),
            x2: to_screen_x(vp, arrow.to[0]),
            y2: to_screen_y(vp, arrow.to[1]),
        };
        let color = p.theme.colors[arrow.ink];
        paint_line(p.reborrow(), &s, arrow.width, color, None, true);
    }
    for dot in &plot.dots {
        let x = to_screen_x(vp, dot.at[0]) as f32;
        let y = to_screen_y(vp, dot.at[1]) as f32;
        let r = (dot.diameter / 2.0) as f32;
        let fill = p.theme.colors[dot.fill];
        p.vg.begin_path();
        p.vg.circle(x, y, r);
        p.vg.fill_color(fill);
        p.vg.fill();
        if let Some(ring) = &dot.ring {
            let ring_color = p.theme.colors[ring.ink];
            p.vg.begin_path();
            p.vg.circle(x, y, r - (ring.width as f32) / 2.0);
            p.vg.stroke_width(ring.width as f32);
            p.vg.stroke_color(ring_color);
            p.vg.stroke();
        }
    }
}
