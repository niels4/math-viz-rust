//! Plane geometry: rectangles in device px.

/// A rectangle: its top-left corner and its size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// Whether two rectangles overlap.
pub fn intersects(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
}

/// Whether a rectangle fits inside a box of a width and height.
pub fn inside_box(r: Rect, width: f64, height: f64) -> bool {
    r.x >= 0.0 && r.y >= 0.0 && r.x + r.w <= width && r.y + r.h <= height
}

/// Grows a rectangle by a margin on every side.
pub fn inflate(r: Rect, by: f64) -> Rect {
    Rect {
        x: r.x - by,
        y: r.y - by,
        w: r.w + 2.0 * by,
        h: r.h + 2.0 * by,
    }
}

/// Whether a point falls inside a rectangle.
pub fn contains(r: Rect, x: f64, y: f64) -> bool {
    x >= r.x && x <= r.x + r.w && y >= r.y && y <= r.y + r.h
}

/// The square of a size centred on a point.
pub fn centred(x: f64, y: f64, size: f64) -> Rect {
    Rect {
        x: x - size / 2.0,
        y: y - size / 2.0,
        w: size,
        h: size,
    }
}
