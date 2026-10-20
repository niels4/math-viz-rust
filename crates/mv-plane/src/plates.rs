//! Plates: the label and edge-marker cards, laid out from runs.

use super::faces::{Face, Measure, baseline_below_top, measure, text_box_height};
use super::placement::Direction;
use super::rect::Rect;
use super::scene::Ink;
use mv_theme::Token;

/// A text run on a plate: its text in a face.
#[derive(Clone, Debug, PartialEq)]
pub struct TextRun {
    pub text: String,
    pub face: Face,
}

/// A triangle run on a plate: an arrowhead in a direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TriangleRun {
    pub dir: Direction,
}

/// What a plate run holds: text or a triangle.
#[derive(Clone, Debug, PartialEq)]
pub enum PlateRunWhat {
    Text(TextRun),
    Triangle(TriangleRun),
}

/// One plate run: what it holds, in an ink, at an offset.
#[derive(Clone, Debug, PartialEq)]
pub struct PlateRun {
    pub what: PlateRunWhat,
    pub ink: Ink,
    pub x: f64,
    pub y: f64,
}

/// How a plate arrives: its alpha and slide.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlateMotion {
    pub alpha: f64,
    pub dx: f64,
    pub dy: f64,
}

/// A laid-out plate: its box, inks, runs and arrival.
#[derive(Clone, Debug, PartialEq)]
pub struct Plate {
    pub r#box: Rect,
    pub radius: f64,
    pub fill: Ink,
    pub border_ink: Ink,
    pub border_width: f64,
    pub runs: Vec<PlateRun>,
    pub motion: Option<PlateMotion>,
}

/// A plate style: padding, run gap, pill shape and border.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlateStyle {
    pub pad: [f64; 4],
    pub gap: f64,
    pub pill: bool,
    pub border: f64,
}

/// The outline width plates draw, in px.
const OUTLINE: f64 = 2.0;

/// A triangle run's box, in px.
const TRIANGLE_BOX: f64 = 12.0;

/// How far a triangle sits inside its box, in px.
pub const TRIANGLE_INSET: f64 = 1.0;

/// A triangle run's size, in px.
pub const TRIANGLE_SIZE: f64 = 10.0;

/// The label plate style.
pub const LABEL_STYLE: PlateStyle = PlateStyle {
    pad: [6.0, 12.0, 6.0, 12.0],
    gap: 6.0,
    pill: true,
    border: OUTLINE,
};

/// The edge-marker plate style.
pub const EDGE_STYLE: PlateStyle = PlateStyle {
    pad: [6.0, 12.0, 6.0, 8.0],
    gap: 8.0,
    pill: true,
    border: OUTLINE,
};

/// Lays out a plate from its runs in a style and ink.
pub fn layout_plate(runs: &[PlateRun], style: PlateStyle, ink: Ink, m: &mut Measure<'_>) -> Plate {
    let (top, right, bottom, left) = (style.pad[0], style.pad[1], style.pad[2], style.pad[3]);
    let edge = style.border;
    let mut sizes = vec![[0.0; 2]; runs.len()];
    let mut row_h: f64 = 0.0;
    for (i, run) in runs.iter().enumerate() {
        sizes[i] = match &run.what {
            PlateRunWhat::Text(r) => [
                measure(&mut *m, r.face, &r.text).ceil(),
                text_box_height(r.face),
            ],
            PlateRunWhat::Triangle(_) => [TRIANGLE_BOX, TRIANGLE_BOX],
        };
        row_h = row_h.max(sizes[i][1]);
    }
    let mut placed = runs.to_vec();
    let mut x = edge + left;
    for (i, run) in runs.iter().enumerate() {
        let box_top = edge + top + (row_h - sizes[i][1]) / 2.0;
        placed[i].x = x;
        placed[i].y = box_top;
        if let PlateRunWhat::Text(r) = &run.what {
            placed[i].y = box_top + baseline_below_top(r.face);
        }
        x += sizes[i][0] + style.gap;
    }
    let w = x - (if runs.is_empty() { 0.0 } else { style.gap }) + right + edge;
    let h = 2.0 * edge + top + bottom + row_h;
    Plate {
        r#box: Rect {
            x: 0.0,
            y: 0.0,
            w,
            h,
        },
        radius: if style.pill { h / 2.0 } else { 0.0 },
        fill: Token::Card,
        border_ink: ink,
        border_width: edge,
        runs: placed,
        motion: None,
    }
}

/// Moves a plate to x, y.
pub fn at(plate: &Plate, x: f64, y: f64) -> Plate {
    let mut out = plate.clone();
    out.r#box.x = x;
    out.r#box.y = y;
    out
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::super::faces::{EDGE_LETTER_FACE, LABEL_LETTER_FACE, readout_faces};
    use super::*;
    use mv_text::Face as TextFace;

    fn board_measure(face: Face, s: &str) -> f64 {
        let per = if face.face == TextFace::Stix400Italic {
            0.57
        } else {
            0.6001
        };
        per * face.size * s.chars().count() as f64
    }

    fn text_run(s: &str, face: Face, ink: Ink) -> PlateRun {
        PlateRun {
            what: PlateRunWhat::Text(TextRun {
                text: s.to_string(),
                face,
            }),
            ink,
            x: 0.0,
            y: 0.0,
        }
    }

    #[test]
    fn a_plate_builds_point_label_fv() {
        let readout = readout_faces(mv_theme::ReadoutFont::RobotoMono);
        let mut m = Measure {
            f: Box::new(board_measure),
        };
        let plate = layout_plate(
            &[
                text_run("P", LABEL_LETTER_FACE, Ink::Foreground),
                text_run("(2, 4)", readout.label, Ink::Foreground),
            ],
            LABEL_STYLE,
            Ink::ChartPoint1,
            &mut m,
        );
        assert_eq!(
            plate.r#box,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 108.0,
                h: 41.0
            }
        );
        assert_eq!(plate.radius, 20.5);
        assert_eq!(plate.runs[0].x, 14.0);
        assert_eq!(plate.runs[1].x, 32.0);
        assert!((plate.runs[0].y - (8.0 + 20.0 * (0.125 + 0.762))).abs() < 1e-9);
        assert!((plate.runs[1].y - (9.5 + 17.0 * (2146.0 / 2048.0))).abs() < 1e-9);
        assert_eq!(plate.fill, Token::Card);
        assert_eq!(plate.border_ink, Token::ChartPoint1);
        assert_eq!(plate.border_width, 2.0);
    }

    #[test]
    fn a_plate_builds_edge_marker_fv() {
        let readout = readout_faces(mv_theme::ReadoutFont::RobotoMono);
        let mut m = Measure {
            f: Box::new(board_measure),
        };
        let plate = layout_plate(
            &[
                PlateRun {
                    what: PlateRunWhat::Triangle(TriangleRun { dir: Direction::Up }),
                    ink: Ink::ChartPoint1,
                    x: 0.0,
                    y: 0.0,
                },
                text_run("P", EDGE_LETTER_FACE, Ink::Foreground),
                text_run("(−3, 10.56)", readout.small, Ink::ForegroundMuted),
            ],
            EDGE_STYLE,
            Ink::ChartPoint1,
            &mut m,
        );
        assert_eq!(
            plate.r#box,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 163.0,
                h: 39.0
            }
        );
        assert!(plate.runs[0].x == 10.0 && plate.runs[0].y == 13.5);
        assert_eq!(plate.runs[1].x, 30.0);
        assert_eq!(plate.runs[2].x, 49.0);
    }
}
