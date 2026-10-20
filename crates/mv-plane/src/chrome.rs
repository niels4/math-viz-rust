//! The plane's chrome: the four corner brackets.
//!
//! The port draws no HUD ticks since the 2026-10-18 ruling (the user dislikes
//! how they look): the geometry below is the brackets alone. The Odin app
//! draws the ticks from its own `hud_ticks` (chrome.odin:32) in
//! paint.odin:735-752, and React still shows them; the brackets stay by the
//! ruling's scope (decisions.md › The port's own calls).

use enum_map::{Enum, EnumMap};

use super::rect::Rect;

/// A corner bracket's box, in px.
pub const BRACKET_BOX: f64 = 28.0;

/// A corner bracket's inset, in px.
pub const BRACKET_INSET: f64 = 10.0;

/// A corner bracket's line width, in px.
pub const BRACKET_WIDTH: f64 = 2.5;

/// A viewport corner.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// The box of a corner's bracket.
pub fn bracket_box(corner: Corner, width: f64, height: f64) -> Rect {
    let far_x = width - BRACKET_INSET - BRACKET_BOX;
    let far_y = height - BRACKET_INSET - BRACKET_BOX;
    match corner {
        Corner::TopLeft => Rect {
            x: BRACKET_INSET,
            y: BRACKET_INSET,
            w: BRACKET_BOX,
            h: BRACKET_BOX,
        },
        Corner::TopRight => Rect {
            x: far_x,
            y: BRACKET_INSET,
            w: BRACKET_BOX,
            h: BRACKET_BOX,
        },
        Corner::BottomLeft => Rect {
            x: BRACKET_INSET,
            y: far_y,
            w: BRACKET_BOX,
            h: BRACKET_BOX,
        },
        Corner::BottomRight => Rect {
            x: far_x,
            y: far_y,
            w: BRACKET_BOX,
            h: BRACKET_BOX,
        },
    }
}

/// A corner bracket's path, in px from its box's corner.
pub(crate) static BRACKET_PATHS: EnumMap<Corner, [[f64; 2]; 3]> = EnumMap::from_array([
    [[2.0, 26.0], [2.0, 2.0], [26.0, 2.0]],
    [[2.0, 2.0], [26.0, 2.0], [26.0, 26.0]],
    [[2.0, 2.0], [2.0, 26.0], [26.0, 26.0]],
    [[26.0, 2.0], [26.0, 26.0], [2.0, 26.0]],
]);

/// The four corner boxes of a width and height.
pub fn corner_boxes(width: f64, height: f64) -> [Rect; 4] {
    [
        bracket_box(Corner::TopLeft, width, height),
        bracket_box(Corner::TopRight, width, height),
        bracket_box(Corner::BottomLeft, width, height),
        bracket_box(Corner::BottomRight, width, height),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dash::{MIN_PIECE_PX, dash_circle, dash_intervals, dash_phase, dash_polyline};
    use crate::faces::{
        EDGE_LETTER_FACE, LABEL_LETTER_FACE, ORIGIN_FACE, TICK_LABEL_FACE, readout_faces,
        text_box_height,
    };

    fn expect_floats(got: &[f64], want: &[f64]) {
        let ok = got.len() == want.len() && got.iter().zip(want).all(|(g, w)| (g - w).abs() < 1e-9);
        assert!(ok, "got {got:?}, want {want:?}");
    }

    #[test]
    fn the_brackets_sit_where_the_reference_puts_them() {
        // The reference's DOM at 1440 x 900: the plane at (488, 92), its brackets at (498, 102),
        // (1386, 102), (498, 846) and (1386, 846), 28 x 28.
        let boxes = corner_boxes(936.0, 792.0);
        assert_eq!(
            boxes[Corner::TopLeft as usize],
            Rect {
                x: 10.0,
                y: 10.0,
                w: 28.0,
                h: 28.0,
            }
        );
        assert_eq!(
            boxes[Corner::TopRight as usize],
            Rect {
                x: 898.0,
                y: 10.0,
                w: 28.0,
                h: 28.0,
            }
        );
        assert_eq!(
            boxes[Corner::BottomLeft as usize],
            Rect {
                x: 10.0,
                y: 754.0,
                w: 28.0,
                h: 28.0,
            }
        );
        assert_eq!(
            boxes[Corner::BottomRight as usize],
            Rect {
                x: 898.0,
                y: 754.0,
                w: 28.0,
                h: 28.0,
            }
        );
    }

    #[test]
    fn the_faces_are_the_fonts() {
        let mut fonts = mv_text::uninit();
        let fonts: &mut mv_text::Fonts = &mut fonts;
        assert!(mv_text::init(&mut *fonts));
        let faces = [
            TICK_LABEL_FACE,
            ORIGIN_FACE,
            LABEL_LETTER_FACE,
            EDGE_LETTER_FACE,
            readout_faces(mv_theme::ReadoutFont::RobotoMono).label,
            readout_faces(mv_theme::ReadoutFont::StixTwoText).small,
        ];
        for face in faces {
            let m = mv_text::metrics(&mut *fonts, face.face);
            assert!(
                (m.ascent as f64 - face.ascent).abs() < 1e-6
                    && (m.descent as f64 - face.descent).abs() < 1e-6
                    && (m.line_gap as f64 - face.line_gap).abs() < 1e-6,
                "{face:?}: the font's {m:?}",
            );
        }
        // Figma's AUTO boxes: STIX 20 → 25, Roboto Mono 17 → 22, STIX 18 → 23 (22.5 rounds up).
        assert_eq!(text_box_height(LABEL_LETTER_FACE), 25.0);
        assert_eq!(
            text_box_height(readout_faces(mv_theme::ReadoutFont::RobotoMono).label),
            22.0
        );
        assert_eq!(text_box_height(EDGE_LETTER_FACE), 23.0);
        mv_text::destroy(&mut *fonts);
    }

    #[test]
    fn dashes_are_cut_as_the_html_spec_cuts_them() {
        // A drop line of 20 px, 5 4 from half a dash before its start: 2.5 on, 4 off, 5 on, 4 off, 4.5.
        let got = dash_intervals(20.0, [5.0, 4.0], dash_phase([5.0, 4.0]));
        let want = [[0.0, 2.5], [6.5, 11.5], [15.5, 20.0]];
        assert_eq!(got.len(), want.len());
        for (i, w) in want.into_iter().enumerate() {
            assert!((got[i][0] - w[0]).abs() < 1e-9 && (got[i][1] - w[1]).abs() < 1e-9);
        }
        // An offset past a whole pattern comes back into it.
        let again = dash_intervals(20.0, [5.0, 4.0], dash_phase([5.0, 4.0]) + 9.0);
        for (i, w) in want.into_iter().enumerate() {
            assert!((again[i][0] - w[0]).abs() < 1e-9 && (again[i][1] - w[1]).abs() < 1e-9);
        }
        // Along a bent polyline a dash keeps the bend: 4 6 from (0, 0) to (3, 0) to (3, 10).
        let pieces = dash_polyline(
            &[[0.0, 0.0], [3.0, 0.0], [3.0, 10.0]],
            [4.0, 6.0],
            dash_phase([4.0, 6.0]),
        );
        assert_eq!(pieces.len(), 2);
        assert_eq!(pieces[0].len(), 2); // 0 … 2 along the first leg
        expect_floats(&[pieces[0][1][0], pieces[0][1][1]], &[2.0, 0.0]);
        // The second from 8 to 12: (3, 5) to (3, 9).
        expect_floats(
            &[
                pieces[1][0][0],
                pieces[1][0][1],
                pieces[1][1][0],
                pieces[1][1][1],
            ],
            &[3.0, 5.0, 3.0, 9.0],
        );
        let bent = dash_polyline(&[[0.0, 0.0], [3.0, 0.0], [3.0, 10.0]], [6.0, 2.0], 0.0);
        // 0 … 6 turns the corner: (0, 0), (3, 0), (3, 3).
        assert_eq!(bent[0].len(), 3);
        expect_floats(&[bent[0][2][0], bent[0][2][1]], &[3.0, 3.0]);
    }

    #[test]
    fn a_dash_cut_to_a_sliver_at_a_path_end_is_left_out() {
        // M10's review: the unit box's dashed sides as Reset all's jump moved them, 5 4 from half a
        // dash in. At 60.5002 px the last dash began at 60.5 and ran 0.0002 px, at 51.5048 px 0.0048
        // px: NanoVG merged each into a point and trapped. A dash under MIN_PIECE_PX is left out.
        for side in [[60.50021853522977, 56.5], [51.504796964683071, 47.5]] {
            let got = dash_intervals(side[0], [5.0, 4.0], dash_phase([5.0, 4.0]));
            for d in &got {
                assert!(d[1] - d[0] >= MIN_PIECE_PX);
            }
            assert_eq!(got[got.len() - 1][1], side[1]); // the last whole dash ends the line
        }
        // One a little longer stays.
        let kept = dash_intervals(60.52, [5.0, 4.0], dash_phase([5.0, 4.0]));
        assert_eq!(kept[kept.len() - 1][0], 60.5);
    }

    #[test]
    fn the_was_ring_has_twelve_even_dashes_one_on_each_compass_point() {
        let r = 10.0;
        let d = std::f64::consts::PI * r / 12.0;
        let arcs = dash_circle(r, [d, d], dash_phase([d, d]));
        assert_eq!(arcs.len(), 12);
        // The first dash across angle 0, the rest every 30°.
        for (i, arc) in arcs.iter().enumerate() {
            let mid = (arc[0] + arc[1]) / 2.0;
            assert!((mid - i as f64 * std::f64::consts::PI / 6.0).abs() < 1e-9);
            assert!((arc[1] - arc[0] - d / r).abs() < 1e-9);
        }
    }
}
