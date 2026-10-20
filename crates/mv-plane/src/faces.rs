//! Font metrics for the plane's own text: tick labels, origins and letters.

use mv_text::{Face as TextFace, Fonts, Style as TextStyle};

/// A face at an em size with its metrics in ems.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Face {
    pub face: TextFace,
    pub size: f64,
    pub ascent: f64,
    pub descent: f64,
    pub line_gap: f64,
}

/// STIX metrics in ems: ascent, descent, line gap.
const STIX_METRICS: [f64; 3] = [0.762, 0.238, 0.25];

/// Roboto Mono metrics in ems: ascent, descent, line gap.
const MONO_METRICS: [f64; 3] = [2146.0 / 2048.0, 555.0 / 2048.0, 0.0];

/// Builds a face at an em size.
pub fn make_face(face: TextFace, size: f64) -> Face {
    let m = if is_stix(face) {
        STIX_METRICS
    } else {
        MONO_METRICS
    };
    Face {
        face,
        size,
        ascent: m[0],
        descent: m[1],
        line_gap: m[2],
    }
}

/// Whether a face sets in STIX.
fn is_stix(face: TextFace) -> bool {
    matches!(
        face,
        TextFace::Stix400 | TextFace::Stix500 | TextFace::Stix400Italic
    )
}

/// The tick-label face: Roboto Mono at 14.
pub const TICK_LABEL_FACE: Face = Face {
    face: TextFace::RobotoMono500,
    size: 14.0,
    ascent: 2146.0 / 2048.0,
    descent: 555.0 / 2048.0,
    line_gap: 0.0,
};

/// The origin-label face: STIX italic at 15.
pub const ORIGIN_FACE: Face = Face {
    face: TextFace::Stix400Italic,
    size: 15.0,
    ascent: 0.762,
    descent: 0.238,
    line_gap: 0.25,
};

/// The label-letter face: STIX italic at 20.
pub const LABEL_LETTER_FACE: Face = Face {
    face: TextFace::Stix400Italic,
    size: 20.0,
    ascent: 0.762,
    descent: 0.238,
    line_gap: 0.25,
};

/// The edge-letter face: STIX italic at 18.
pub const EDGE_LETTER_FACE: Face = Face {
    face: TextFace::Stix400Italic,
    size: 18.0,
    ascent: 0.762,
    descent: 0.238,
    line_gap: 0.25,
};

/// The readout faces: its label face and its small face.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReadoutFaces {
    pub label: Face,
    pub small: Face,
}

/// The readout faces for a readout font.
pub fn readout_faces(font: mv_theme::ReadoutFont) -> ReadoutFaces {
    let face = if font == mv_theme::ReadoutFont::StixTwoText {
        TextFace::Stix500
    } else {
        TextFace::RobotoMono500
    };
    ReadoutFaces {
        label: make_face(face, 17.0),
        small: make_face(face, 15.0),
    }
}

/// The line box of a face: its full height at its size.
pub fn line_box(face: Face) -> f64 {
    face.size * (face.ascent + face.descent + face.line_gap)
}

/// The text box height of a face: ascent plus descent at its size.
pub fn text_box_height(face: Face) -> f64 {
    js_round(line_box(face))
}

/// The baseline below the text box's top.
pub fn baseline_below_top(face: Face) -> f64 {
    face.size * (face.line_gap / 2.0 + face.ascent)
}

/// The shaping style of a face at its size.
pub fn style(face: Face) -> TextStyle {
    TextStyle {
        face: face.face,
        size: face.size as f32,
        tracking: 0.0,
        features: mv_text::Features::empty(),
    }
}

/// A text measure: a closure over the shaping fonts.
pub struct Measure<'a> {
    pub f: MeasureFn<'a>,
}

/// A measure's closure.
type MeasureFn<'a> = Box<dyn FnMut(Face, &str) -> f64 + 'a>;

/// Measures a string in a face.
pub fn measure(m: &mut Measure<'_>, face: Face, s: &str) -> f64 {
    (m.f)(face, s)
}

/// Builds a measure over the shaping fonts.
pub fn text_measure(fonts: &mut Fonts) -> Measure<'_> {
    Measure {
        f: Box::new(move |face: Face, s: &str| mv_text::measure(fonts, style(face), s) as f64),
    }
}

/// JavaScript's Math.round: halves up, as the page rounds.
pub fn js_round(v: f64) -> f64 {
    (v + 0.5).floor()
}
