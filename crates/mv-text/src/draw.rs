//! Drawing shaped runs through NanoVG, one font run at a time.

use super::fonts::Fonts;
use super::shape::{Shaped, Style, shape};
use super::typeset::Line;

/// A colour as NanoVG takes it: sRGB, each channel and the alpha 0 to 1.
pub type Color = [f32; 4];

/// Draws text in a style, its pen starting at x on the baseline y.
pub fn draw(
    fonts: &mut Fonts,
    vg: &mut mv_nvg::Context,
    style: Style,
    x: f32,
    y: f32,
    text: &str,
    ink: Color,
) {
    let shaped = shape(fonts, style, text);
    draw_shaped(fonts, vg, style.size, x, y, &shaped, ink);
}

/// Draws a set line with its top left at x, y.
pub fn draw_line(fonts: &mut Fonts, vg: &mut mv_nvg::Context, line: &Line, x: f32, y: f32) {
    for p in &line.placed {
        draw_shaped(
            fonts,
            vg,
            p.size,
            x + p.pen,
            y + p.baseline,
            &p.shaped,
            p.ink,
        );
    }
}

/// Draws a shaped run at a size, its pen starting at x on the baseline y.
pub fn draw_shaped(
    fonts: &mut Fonts,
    vg: &mut mv_nvg::Context,
    size: f32,
    x: f32,
    y: f32,
    shaped: &Shaped,
    ink: Color,
) {
    let glyphs = &shaped.glyphs;
    if glyphs.is_empty() {
        return;
    }
    vg.font_size(size);
    vg.fill_color(ink);
    let placed: Vec<mv_nvg::PlacedGlyph> = glyphs
        .iter()
        .map(|g| mv_nvg::PlacedGlyph {
            index: g.id as i32,
            x: g.x,
            y: g.y,
        })
        .collect();
    // Each file's glyphs go to NanoVG together: the face's, and the symbols
    // file's where a run fell back to it.
    let mut start = 0;
    let mut i = 1;
    while i <= glyphs.len() {
        if i < glyphs.len() && glyphs[i].font == glyphs[start].font {
            i += 1;
            continue;
        }
        vg.font_face_id(fonts.vg[glyphs[start].font]);
        vg.text_glyphs(x, y, &placed[start..i]);
        start = i;
        i += 1;
    }
}
