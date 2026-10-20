//! The line box: baselines, normal line heights, and the whole-pixel rounding
//! the page's runs use in every engine.

use super::fonts::Face;
use super::fonts::Fonts;
use super::fonts::metrics;
use super::shape::Style;
use super::shape::measure;

/// The baseline below the line box's top, at a size and line height.
pub fn baseline(fonts: &mut Fonts, face: Face, size: f32, line_height: f32) -> f32 {
    let m = metrics(fonts, face);
    let (ascent, descent) = (size * m.ascent, size * m.descent);
    (line_height - (ascent + descent)) / 2.0 + ascent
}

/// The `normal` line height at a size: halves round up, as in WebKit.
pub fn line_height_normal(fonts: &mut Fonts, face: Face, size: f32) -> f32 {
    let m = metrics(fonts, face);
    half_up(size * m.ascent) + half_up(size * m.descent) + half_up(size * m.line_gap)
}

/// The advance of one `ch` in a style: the `0` advance, tracking dropped.
pub fn ch(fonts: &mut Fonts, style: Style) -> f32 {
    let plain = Style {
        tracking: 0.0,
        ..style
    };
    measure(fonts, plain, "0")
}

/// The whole-pixel width a run's advance takes: the ceiling of the advance
/// less a hair, so exact pixels stay put.
pub fn layout_width(advance: f32) -> f32 {
    (advance * 64.0 - 1e-3).ceil() / 64.0
}

/// Rounds up to the whole pixel, exact pixels kept.
pub fn round_up(v: f32) -> f32 {
    (v - 1e-3).ceil()
}

/// Rounds halves up.
fn half_up(v: f32) -> f32 {
    (v + 0.5).floor()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fonts::{destroy, init, uninit};
    use crate::shape::fixture::{load_fixture, style_of};

    #[test]
    fn line_boxes_are_webkits() {
        let fixture = load_fixture();
        let mut fonts = uninit();
        assert!(init(&mut fonts));
        for s in &fixture.samples {
            let Some(style) = style_of(s) else { continue };
            let height = match s.line_height {
                Some(set) => set,
                None => line_height_normal(&mut fonts, style.face, style.size),
            };
            assert!(
                height == s.height,
                "{}: a line box {height} px tall, WebKit's {}",
                s.name,
                s.height
            );
            let below = baseline(&mut fonts, style.face, style.size, height);
            assert!(
                below - s.baseline >= -1e-4 && below - s.baseline < 1.0 / 64.0 + 1e-4,
                "{}: the baseline {below:.4} px below the line box's top, WebKit's {:.4}",
                s.name,
                s.baseline
            );
        }
        destroy(&mut fonts);
    }
}
