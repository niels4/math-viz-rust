//! Runs of mixed faces set side by side on one line, each in a box of its own.

use super::draw::Color;
use super::fonts::Fonts;
use super::line::{baseline, line_height_normal, round_up};
use super::shape::{Shaped, Style, shape};

/// How a piece's box takes its text's width.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Box {
    /// The text's advance.
    Exact,
    /// The advance rounded up to whole px, as the page rounds it.
    WholePx,
}

/// Where a piece's text sits in a box wider than it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Left,
    /// Text at the right: a number in its slot.
    Right,
}

/// Where the line's pieces sit against each other.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Vertical {
    /// The pieces share one baseline.
    Baseline,
    /// Each line box centres on the line.
    Center,
}

/// One run on the line: its style and text, its ink, and its box.
#[derive(Clone, Debug, PartialEq)]
pub struct Piece {
    pub style: Style,
    pub text: String,
    pub ink: Color,
    /// The line height in px; 0 is `normal`.
    pub line_height: f32,
    pub r#box: Box,
    /// The box at least this wide, padding aside.
    pub min_width: f32,
    pub align: Align,
    /// Px of padding left and right of the box's content.
    pub pad: [f32; 2],
}

/// A piece as the line placed it, in px from the line's left and top.
#[derive(Clone, Debug, PartialEq)]
pub struct Placed {
    pub shaped: Shaped,
    pub size: f32,
    pub ink: Color,
    pub x: f32,
    pub width: f32,
    pub pen: f32,
    pub top: f32,
    pub height: f32,
    pub baseline: f32,
}

/// A set line: its pieces, its width and height, and its baseline.
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub placed: Vec<Placed>,
    pub width: f32,
    pub height: f32,
    pub baseline: f32,
}

/// Sets pieces side by side on one line.
pub fn typeset(
    fonts: &mut Fonts,
    pieces: &[Piece],
    gap: f32,
    vertical: Vertical,
    scale: f32,
) -> Line {
    let mut placed = Vec::with_capacity(pieces.len());
    let mut x = 0.0;
    for (i, piece) in pieces.iter().enumerate() {
        let style = Style {
            size: piece.style.size * scale,
            ..piece.style
        };
        let shaped = shape(fonts, style, &piece.text);
        let content = match piece.r#box {
            Box::WholePx => round_up(shaped.advance),
            Box::Exact => shaped.advance,
        };
        let content = content.max(piece.min_width);
        let mut pen = x + piece.pad[0];
        if piece.align == Align::Right {
            pen += content - shaped.advance;
        }
        let height = if piece.line_height == 0.0 {
            line_height_normal(fonts, style.face, style.size)
        } else {
            piece.line_height
        };
        placed.push(Placed {
            baseline: baseline(fonts, style.face, style.size, height),
            shaped,
            size: style.size,
            ink: piece.ink,
            x,
            width: piece.pad[0] + content + piece.pad[1],
            pen,
            top: 0.0,
            height,
        });
        x += placed[i].width;
        if i + 1 < pieces.len() {
            x += gap;
        }
    }
    let mut line = Line {
        placed,
        width: x,
        height: 0.0,
        baseline: 0.0,
    };
    match vertical {
        Vertical::Baseline => {
            for p in &line.placed {
                line.baseline = line.baseline.max(p.baseline);
            }
            for p in &mut line.placed {
                p.top = line.baseline - p.baseline;
                p.baseline = line.baseline;
                line.height = line.height.max(p.top + p.height);
            }
        }
        Vertical::Center => {
            for p in &line.placed {
                line.height = line.height.max(p.height);
            }
            for p in &mut line.placed {
                p.top = (line.height - p.height) / 2.0;
                p.baseline += p.top;
            }
            if let Some(first) = line.placed.first() {
                line.baseline = first.baseline;
            }
        }
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fonts::{Face, destroy, init, uninit};
    use crate::line::{baseline, ch, round_up};
    use crate::shape::{Feature, Features, Style, measure};

    #[test]
    fn typeset_places_boxes() {
        let mut fonts = uninit();
        assert!(init(&mut fonts));
        let math = Style {
            face: Face::Stix400,
            size: 20.0,
            tracking: 0.0,
            features: Features::new(),
        };
        let number = Style {
            face: Face::RobotoMono500,
            size: 18.0,
            tracking: 0.0,
            features: Feature::Tabular.into(),
        };
        let slot = round_up(5.0 * ch(&mut fonts, number));
        let pieces = [
            Piece {
                style: math,
                text: "(".to_owned(),
                ink: [0.0; 4],
                line_height: 25.0,
                r#box: Box::WholePx,
                min_width: 0.0,
                align: Align::Left,
                pad: [0.0, 0.0],
            },
            Piece {
                style: number,
                text: "2.00".to_owned(),
                ink: [0.0; 4],
                line_height: 24.0,
                r#box: Box::Exact,
                min_width: slot,
                align: Align::Right,
                pad: [0.0, 0.0],
            },
            Piece {
                style: math,
                text: ")".to_owned(),
                ink: [0.0; 4],
                line_height: 25.0,
                r#box: Box::WholePx,
                min_width: 0.0,
                align: Align::Left,
                pad: [0.0, 7.0],
            },
        ];
        let line = typeset(&mut fonts, &pieces, 2.0, Vertical::Baseline, 1.0);
        let open = round_up(measure(&mut fonts, math, "("));
        let digits = measure(&mut fonts, number, "2.00");
        assert_eq!(slot, 55.0);
        assert_eq!(line.placed[0].width, open);
        assert_eq!(line.placed[1].x, open + 2.0);
        assert_eq!(line.placed[1].width, 55.0);
        assert!(
            (line.placed[1].pen - (open + 2.0 + 55.0 - digits)).abs() < 1e-4,
            "the number starts at {}",
            line.placed[1].pen
        );
        assert_eq!(
            line.placed[2].width,
            round_up(measure(&mut fonts, math, ")")) + 7.0
        );
        assert_eq!(line.width, line.placed[2].x + line.placed[2].width);
        let stix_base = baseline(&mut fonts, Face::Stix400, 20.0, 25.0);
        let mono_base = baseline(&mut fonts, Face::RobotoMono500, 18.0, 24.0);
        assert_eq!(line.baseline, stix_base.max(mono_base));
        for p in &line.placed {
            assert_eq!(p.baseline, line.baseline);
        }
        assert_eq!(line.placed[1].top, line.baseline - mono_base);
        let centred = typeset(&mut fonts, &pieces, 0.0, Vertical::Center, 1.0);
        assert_eq!(centred.height, 25.0);
        assert_eq!(centred.placed[1].top, 0.5);
        assert_eq!(centred.placed[1].baseline, 0.5 + mono_base);
        assert_eq!(centred.placed[0].baseline, stix_base);
        destroy(&mut fonts);
    }
}
