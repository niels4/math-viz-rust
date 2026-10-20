//! The page's words: titles, names, hints and tips as rich copy.

use std::sync::LazyLock;

use enum_map::{Enum, EnumMap};

use super::form::{DEFAULT_PARAMS, PARAM_LETTERS, Param, ZERO_SCALE, is_scale};
use super::state::OffView;
use mv_numfmt::{DISPLAY_DP, format_mark};

/// The view's title.
pub const VIEW_TITLE: &str = "Function Viewer";

/// The view's subtitle.
pub const VIEW_SUBTITLE: &str = "Functions · transformations";

/// A panel section, in reading order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum Section {
    Function,
    Transform,
    Points,
}

/// The section titles.
pub static SECTION_TITLES: EnumMap<Section, &str> =
    EnumMap::from_array(["Function", "Transform", "Points"]);

/// A parameter's name.
pub static PARAM_NAMES: EnumMap<Param, &str> = EnumMap::from_array([
    "Vertical scale",
    "Vertical shift",
    "Horizontal scale",
    "Horizontal shift",
]);

/// The transform groups: the vertical pair and the horizontal pair.
pub const TRANSFORM_GROUPS: [[Param; 2]; 2] = [[Param::A, Param::K], [Param::B, Param::H]];

/// A point: pinned P or following Q.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum Point {
    P,
    Q,
}

/// The point names.
pub static POINT_NAMES: EnumMap<Point, &str> = EnumMap::from_array(["P", "Q"]);

/// P's role: pinned.
pub const P_ROLE: &str = "pinned";

/// Q's role while live: follows the pointer.
pub const Q_ROLE_LIVE: &str = "follows your pointer";

/// Q's role while empty: point at the plane.
pub const Q_ROLE_EMPTY: &str = "point at the plane";

/// P's role against the view: pinned, above or below.
pub static OFF_VIEW_ROLES: EnumMap<OffView, &str> =
    EnumMap::from_array([P_ROLE, "above the view", "below the view"]);

/// Words: plain text.
#[derive(Clone, Debug, PartialEq)]
pub struct Words(pub String);

/// Math: set in the maths face.
#[derive(Clone, Debug, PartialEq)]
pub struct Math(pub String);

/// Key: a key cap.
#[derive(Clone, Debug, PartialEq)]
pub struct Key(pub String);

/// Together: phrases kept on one line.
#[derive(Clone, Debug, PartialEq)]
pub struct Together(pub Vec<Phrase>);

/// A rich-copy phrase: words, maths, a key, or phrases kept together.
#[derive(Clone, Debug, PartialEq)]
pub enum Phrase {
    Words(Words),
    Math(Math),
    Key(Key),
    Together(Together),
}

/// The plain words of phrases, flattened.
pub fn phrase_words(phrases: &[Phrase]) -> Vec<String> {
    fn add(words: &mut Vec<String>, phrases: &[Phrase]) {
        for p in phrases {
            match p {
                Phrase::Words(w) => words.push(w.0.clone()),
                Phrase::Math(m) => words.push(m.0.clone()),
                Phrase::Key(k) => words.push(k.0.clone()),
                Phrase::Together(t) => add(words, &t.0),
            }
        }
    }
    let mut words = Vec::new();
    add(&mut words, phrases);
    words
}

/// What resetting everything says.
pub const RESET_ALL: &str = "Reset all";

/// The badge that takes a name's place mid-drag.
pub fn mode_badge(mode: mv_scrub::Mode, scale: bool) -> String {
    match mode {
        mv_scrub::Mode::Fine => "Fine ×0.1".to_string(),
        mv_scrub::Mode::Snap => {
            if scale {
                "Snap ¼".to_string()
            } else {
                "Snap 1".to_string()
            }
        }
        mv_scrub::Mode::Coarse => String::new(),
    }
}

/// The idle hint.
pub static HINT_IDLE: LazyLock<[Phrase; 3]> = LazyLock::new(|| {
    [
        Phrase::Words(Words(
            "Drag a ruler sideways to reshape the curve ·".to_string(),
        )),
        Phrase::Words(Words("point at the plane to read".to_string())),
        Phrase::Math(Math("f(x)".to_string())),
    ]
});

/// The fine-drag hint.
pub static HINT_FINE: LazyLock<[Phrase; 2]> = LazyLock::new(|| {
    [
        Phrase::Key(Key("Shift".to_string())),
        Phrase::Words(Words(
            "Fine: a tenth of the speed, steps of 0.001".to_string(),
        )),
    ]
});

/// The snap-drag hint.
pub static HINT_SNAP: LazyLock<[Phrase; 3]> = LazyLock::new(|| {
    [
        Phrase::Key(Key("Ctrl".to_string())),
        Phrase::Words(Words("Snapping to whole numbers".to_string())),
        Phrase::Words(Words("(scales: quarters)".to_string())),
    ]
});

/// The editing hint.
pub static HINT_EDIT: LazyLock<[Phrase; 7]> = LazyLock::new(|| {
    [
        Phrase::Key(Key("Enter".to_string())),
        Phrase::Words(Words("apply ·".to_string())),
        Phrase::Key(Key("Esc".to_string())),
        Phrase::Words(Words("cancel ·".to_string())),
        Phrase::Key(Key("↑".to_string())),
        Phrase::Key(Key("↓".to_string())),
        Phrase::Words(Words("nudge 0.01".to_string())),
    ]
});

/// The plane hint.
pub static HINT_PLANE: LazyLock<[Phrase; 3]> = LazyLock::new(|| {
    [
        Phrase::Math(Math("Q".to_string())),
        Phrase::Words(Words("follows your pointer ·".to_string())),
        Phrase::Words(Words("drag to pan · scroll to zoom".to_string())),
    ]
});

/// The plane's keyboard hint.
pub static HINT_PLANE_KEYS: LazyLock<[Phrase; 4]> = LazyLock::new(|| {
    [
        Phrase::Together(Together(vec![
            Phrase::Key(Key("←".to_string())),
            Phrase::Key(Key("↑".to_string())),
            Phrase::Key(Key("↓".to_string())),
            Phrase::Key(Key("→".to_string())),
            Phrase::Words(Words("pan ·".to_string())),
        ])),
        Phrase::Together(Together(vec![
            Phrase::Key(Key("+".to_string())),
            Phrase::Key(Key("−".to_string())),
            Phrase::Words(Words("zoom ·".to_string())),
        ])),
        Phrase::Together(Together(vec![
            Phrase::Key(Key("[".to_string())),
            Phrase::Key(Key("]".to_string())),
            Phrase::Words(Words("move".to_string())),
            Phrase::Math(Math("P".to_string())),
            Phrase::Words(Words("·".to_string())),
        ])),
        Phrase::Together(Together(vec![
            Phrase::Key(Key("0".to_string())),
            Phrase::Words(Words("resets the view".to_string())),
        ])),
    ]
});

/// The point hint.
pub static HINT_POINT: LazyLock<[Phrase; 6]> = LazyLock::new(|| {
    [
        Phrase::Words(Words("Drag".to_string())),
        Phrase::Math(Math("P".to_string())),
        Phrase::Words(Words("along the curve ·".to_string())),
        Phrase::Key(Key("←".to_string())),
        Phrase::Key(Key("→".to_string())),
        Phrase::Words(Words("nudge 0.1".to_string())),
    ]
});

/// The handles hint.
pub static HINT_HANDLES: LazyLock<[Phrase; 2]> = LazyLock::new(|| {
    [
        Phrase::Words(Words("Drag the diamond to move the curve ·".to_string())),
        Phrase::Words(Words("the square to stretch it".to_string())),
    ]
});

/// The anchor hint.
pub static HINT_ANCHOR: LazyLock<[Phrase; 7]> = LazyLock::new(|| {
    [
        Phrase::Words(Words("Moving the anchor sets".to_string())),
        Phrase::Math(Math("h".to_string())),
        Phrase::Words(Words("and".to_string())),
        Phrase::Math(Math("k".to_string())),
        Phrase::Words(Words("·".to_string())),
        Phrase::Key(Key("Shift".to_string())),
        Phrase::Words(Words("locks one axis".to_string())),
    ]
});

/// The stretch hint.
pub static HINT_STRETCH: LazyLock<[Phrase; 7]> = LazyLock::new(|| {
    [
        Phrase::Words(Words("Stretching sets".to_string())),
        Phrase::Math(Math("a".to_string())),
        Phrase::Words(Words("(height) and".to_string())),
        Phrase::Math(Math("b".to_string())),
        Phrase::Words(Words("(width) ·".to_string())),
        Phrase::Key(Key("Shift".to_string())),
        Phrase::Words(Words("locks one of them".to_string())),
    ]
});

/// The rest of the hints, after the lead.
fn gestures(lead: &[Phrase], param: Param) -> Vec<Phrase> {
    let steps = if is_scale(param) { "quarter" } else { "whole" };
    let last = format!(
        "{} steps · double-click: back to {}",
        steps,
        format_mark(DEFAULT_PARAMS[param], DISPLAY_DP)
    );
    let mut phrases = Vec::with_capacity(lead.len() + 4);
    phrases.extend(lead.iter().cloned());
    phrases.push(Phrase::Key(Key("Shift".to_string())));
    phrases.push(Phrase::Words(Words("fine ·".to_string())));
    phrases.push(Phrase::Key(Key("Ctrl".to_string())));
    phrases.push(Phrase::Words(Words(last)));
    phrases
}

/// How a transform drag reads.
pub fn hint_transform(param: Param) -> Vec<Phrase> {
    let lead = [
        Phrase::Words(Words("Drag to change".to_string())),
        Phrase::Math(Math(PARAM_LETTERS[param].to_string())),
        Phrase::Words(Words("·".to_string())),
    ];
    gestures(&lead, param)
}

/// What an explainer says, by parameter.
pub static EXPLAINER_SAY: EnumMap<Param, &str> = EnumMap::from_array([
    "Multiplies every height by a. a = 2 makes the curve twice as tall; between 0 and 1 it flattens; below 0 it flips upside down.",
    "Adds k to every height, so the whole curve moves up by k (down when k is negative).",
    "Divides x by b before f sees it. b = 2 makes the curve twice as wide; between 0 and 1 it narrows.",
    "Subtracts h from x before f sees it, so the curve moves right by h. Inside f the sign looks backwards: x − 2 moves right.",
]);

/// An explainer's snippet, by parameter.
pub static EXPLAINER_SNIPPETS: EnumMap<Param, &str> =
    EnumMap::from_array(["a · f(x)", "f(x) + k", "f(x / b)", "f(x − h)"]);

/// Where an explainer's head points.
pub const IN_THE_EQUATION: &str = "In the equation";

/// Where each transform group acts.
pub static GROUP_WHERE: [&str; 2] = ["outside", "inside"];

/// The equation shape the groups act on.
pub const GROUP_MATH: &str = "f( )";

/// Where a parameter acts: its group's word.
pub fn where_of(param: Param) -> String {
    for (i, group) in TRANSFORM_GROUPS.iter().enumerate() {
        for p in group {
            if *p == param {
                return GROUP_WHERE[i].to_string();
            }
        }
    }
    String::new()
}

/// What an explainer's gesture drags: the number.
pub static GESTURE_LEAD: LazyLock<[Phrase; 1]> =
    LazyLock::new(|| [Phrase::Words(Words("Drag the number ·".to_string()))]);

/// An explainer's gestures, after what to drag.
pub fn explainer_gestures(param: Param) -> Vec<Phrase> {
    gestures(&GESTURE_LEAD[..], param)
}

/// A refused edit's hint: the error and what it puts back.
pub fn hint_refused(error: &str, puts_back: &str) -> Vec<Phrase> {
    let mut phrases = Vec::new();
    if error == ZERO_SCALE {
        phrases.push(Phrase::Words(Words(
            "A scale of 0 squashes the curve flat.".to_string(),
        )));
        phrases.push(Phrase::Words(Words("Try 0.1 ·".to_string())));
    } else {
        phrases.push(Phrase::Words(Words(
            "Type a number such as 1.5 ·".to_string(),
        )));
    }
    phrases.push(Phrase::Key(Key("Esc".to_string())));
    phrases.push(Phrase::Words(Words(format!("puts back {puts_back}"))));
    phrases
}

/// How a drag tips: the value it resets to.
pub fn tip_drag(reset: f64) -> Vec<Phrase> {
    vec![
        Phrase::Words(Words("Drag sideways ·".to_string())),
        Phrase::Words(Words("Shift for fine ·".to_string())),
        Phrase::Words(Words(format!(
            "double-click: back to {}",
            format_mark(reset, DISPLAY_DP)
        ))),
    ]
}

/// How keys tip: the value they reset to.
pub fn tip_keys(reset: f64) -> Vec<Phrase> {
    vec![
        Phrase::Together(Together(vec![
            Phrase::Key(Key("←".to_string())),
            Phrase::Key(Key("→".to_string())),
            Phrase::Words(Words("nudge 0.01,".to_string())),
            Phrase::Key(Key("Shift".to_string())),
            Phrase::Words(Words("0.1 ·".to_string())),
        ])),
        Phrase::Together(Together(vec![
            Phrase::Key(Key("Enter".to_string())),
            Phrase::Words(Words("type ·".to_string())),
        ])),
        Phrase::Together(Together(vec![
            Phrase::Key(Key("Backspace".to_string())),
            Phrase::Words(Words(format!("back to {}", format_mark(reset, DISPLAY_DP)))),
        ])),
    ]
}
