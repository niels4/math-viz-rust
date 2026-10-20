//! Ported from `src/script/script.odin` (698 lines): the action-language reader.
//!
//! A line is a verb and its arguments, and '#' starts a comment. Lines before
//! the first action say how the page opens; after it, size, scheme and
//! reduced-motion change it live. Positions are logical pixels, times
//! milliseconds.

use enum_map::{Enum, EnumArray, EnumMap};
use enumset::{EnumSet, EnumSetType};

/// A script: how the page opens, then its actions.
///
/// Ported from `script.odin:16` (`Script :: struct { start, actions }`; the
/// allocator-borrowed slice is an owned `Vec`).
#[derive(Clone, Debug, PartialEq)]
pub struct Script {
    pub start: Start,
    pub actions: Vec<Action>,
}

/// How the page opens: the window's logical size and pixel density, the
/// system's colour scheme and motion preference, the stored appearance, and
/// the colour space the port draws in.
///
/// Ported from `script.odin:25` (`Start :: struct { size, density, scheme,
/// mode, dark_theme, light_theme, reduced_motion, space }`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Start {
    pub size: [i64; 2],
    pub density: f32,
    pub scheme: mv_theme::Scheme,
    pub mode: mv_theme::Mode,
    pub dark_theme: mv_theme::Slug,
    pub light_theme: mv_theme::Slug,
    pub reduced_motion: bool,
    pub space: mv_theme::Space,
}

/// React's defaults, on a dark system at the size the judgments use.
///
/// Ported from `script.odin:37` (`DEFAULT_START :: Start{...}`).
pub const DEFAULT_START: Start = Start {
    size: [1440, 900],
    density: 2.0,
    scheme: mv_theme::Scheme::Dark,
    mode: mv_theme::DEFAULT_PREFS.mode,
    dark_theme: mv_theme::DEFAULT_PREFS.dark,
    light_theme: mv_theme::DEFAULT_PREFS.light,
    reduced_motion: false,
    space: mv_theme::Space::Srgb,
};

/// An action: a verb, the line it came from, and the surfaces it runs on.
///
/// Ported from `script.odin:47` (`Action :: struct { line, verb, surface }`).
#[derive(Clone, Debug, PartialEq)]
pub struct Action {
    /// In the script, for messages.
    pub line: i64,
    pub verb: Verb,
    /// The surfaces it runs on.
    pub surface: Surface,
}

/// `ref LINE` runs on the reference alone, `port LINE` on the builds alone.
///
/// Ported from `script.odin:57` (`Surface :: enum u8 { All, Ref, Port }`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum Surface {
    All,
    Ref,
    Port,
}

/// A line's verb: an input, time passing, a capture, a live change, or the
/// port's model set directly.
///
/// Ported from `script.odin:63` (`Verb :: union { Move, Press, ... }`, 15 members).
#[derive(Clone, Debug, PartialEq)]
pub enum Verb {
    Move(Move),
    Press(Press),
    Click(Click),
    Wheel(Wheel),
    Pinch(Pinch),
    Key(Key),
    Hold(Hold),
    Type(Type),
    Wait(Wait),
    Shot(Shot),
    Frames(Frames),
    Video(Video),
    Resize(Resize),
    SchemeChange(SchemeChange),
    MotionChange(MotionChange),
    Model(Model),
}

/// move X Y [over MS]: the pointer to (X, Y); with `over`, in a step every
/// frame for MS.
///
/// Ported from `script.odin:83` (`Move :: struct { to, over }`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Move {
    pub to: [f32; 2],
    pub over: i64,
}

/// down, up: the primary button at the pointer.
///
/// Ported from `script.odin:89` (`Press :: struct { down }`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Press {
    pub down: bool,
}

/// click X Y, dblclick X Y: the pointer to (X, Y), pressed and released
/// `count` times.
///
/// Ported from `script.odin:95` (`Click :: struct { at, count }`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Click {
    pub at: [f32; 2],
    pub count: i64,
}

/// wheel DX DY `[ctrl]`: a wheel event at the pointer, in the DOM's pixel deltas.
///
/// Ported from `script.odin:101` (`Wheel :: struct { delta, ctrl }`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wheel {
    pub delta: [f32; 2],
    pub ctrl: bool,
}

/// pinch SCALE: a trackpad pinch at the pointer; above 1 the fingers spread.
///
/// Ported from `script.odin:107` (`Pinch :: struct { scale }`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pinch {
    pub scale: f32,
}

/// key NAME `[modifiers]`, or tab `[shift]`: a key pressed and released.
///
/// Ported from `script.odin:113` (`Key :: struct { key, mods }`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Key {
    pub key: KeyName,
    pub mods: Modifiers,
}

/// hold NAME, release NAME: a key held down until released.
///
/// Ported from `script.odin:119` (`Hold :: struct { key, down }`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hold {
    pub key: KeyName,
    pub down: bool,
}

/// type "TEXT": each character typed in turn.
///
/// Ported from `script.odin:125` (`Type :: struct { text }`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Type {
    pub text: String,
}

/// wait MS: time passes, and the frames that fall due in it are drawn.
///
/// Ported from `script.odin:130` (`Wait :: struct { ms }`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Wait {
    pub ms: i64,
}

/// shot NAME: what the page shows now, as NAME.png.
///
/// Ported from `script.odin:135` (`Shot :: struct { name }`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shot {
    pub name: String,
}

/// frames NAME every MS for MS: a shot through `span`.
///
/// Ported from `script.odin:141` (`Frames :: struct { name, every, span }`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frames {
    pub name: String,
    pub every: i64,
    pub span: i64,
}

/// video start `[NAME]`, video stop: a recording at real speed.
///
/// Ported from `script.odin:149` (`Video :: struct { start, name }`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Video {
    pub start: bool,
    pub name: String,
}

/// size W H after the first action: the window resized.
///
/// Ported from `script.odin:154` (`Resize :: struct { size }`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Resize {
    pub size: [i64; 2],
}

/// scheme dark|light after the first action: the system's scheme changes.
///
/// Ported from `script.odin:159` (`Scheme_Change :: struct { scheme }`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SchemeChange {
    pub scheme: mv_theme::Scheme,
}

/// reduced-motion on|off after the first action: the motion preference changes.
///
/// Ported from `script.odin:164` (`Motion_Change :: struct { reduced }`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MotionChange {
    pub reduced: bool,
}

/// model WHAT ...: the port's model set directly, on a `port` line only.
///
/// Ported from `script.odin:175` (`Model :: struct { set, name, value }`).
#[derive(Clone, Debug, PartialEq)]
pub struct Model {
    pub set: ModelSet,
    /// The function, parameter or part as the line names it; "none" for none.
    pub name: String,
    pub value: [f64; 2],
}

/// What a model line sets.
///
/// Ported from `script.odin:181` (`Model_Set :: enum u8`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum ModelSet {
    Fn,
    Param,
    P,
    Drag,
    PBefore,
}

/// The words for what a model line sets.
///
/// Ported from `script.odin:190` (`MODEL_WORDS := [Model_Set]string`, `@(rodata)`).
pub static MODEL_WORDS: EnumMap<ModelSet, &str> =
    EnumMap::from_array(["fn", "param", "p", "drag", "p-before"]);

/// The base functions a model line names.
///
/// Ported from `script.odin:198` (`MODEL_FNS :: [?]string{...}`).
pub const MODEL_FNS: [&str; 4] = ["x", "x2", "x3", "sin"];

/// The parameters a model line names.
///
/// Ported from `script.odin:199` (`MODEL_PARAMS :: [?]string{...}`).
pub const MODEL_PARAMS: [&str; 4] = ["a", "k", "b", "h"];

/// The drag parts a model line names.
///
/// Ported from `script.odin:200` (`MODEL_PARTS :: [?]string{...}`).
pub const MODEL_PARTS: [&str; 8] = ["a", "k", "b", "h", "p", "anchor", "stretch", "none"];

/// A key by its DOM name, or the one character it types.
///
/// Ported from `script.odin:203` (`Key_Name :: union { Named_Key, rune }`; the
/// rune is a `char`: `key_name` only accepts one printable character).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyName {
    Named(NamedKey),
    Char(char),
}

/// A key by its DOM name.
///
/// Ported from `script.odin:208` (`Named_Key`, 18 variants).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum NamedKey {
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Enter,
    Escape,
    Tab,
    Backspace,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,
    Space,
    Shift,
    Control,
    Alt,
    Meta,
}

/// The DOM's names, which Playwright's keyboard takes as they are.
///
/// Ported from `script.odin:231` (`NAMED_KEYS := [Named_Key]string`, `@(rodata)`).
pub static NAMED_KEYS: EnumMap<NamedKey, &str> = EnumMap::from_array([
    "ArrowUp",
    "ArrowDown",
    "ArrowLeft",
    "ArrowRight",
    "Enter",
    "Escape",
    "Tab",
    "Backspace",
    "Delete",
    "Home",
    "End",
    "PageUp",
    "PageDown",
    "Space",
    "Shift",
    "Control",
    "Alt",
    "Meta",
]);

/// A modifier key.
///
/// Ported from `script.odin:252` (`Modifier :: enum u8`). The `EnumSetType`
/// derive supplies `Clone`, `Copy`, `PartialEq` and `Eq` itself; `Enum` serves
/// the `MODIFIER_WORDS` table.
#[derive(Debug, Enum, EnumSetType)]
pub enum Modifier {
    Shift,
    Control,
    Alt,
    Meta,
}

/// The modifiers held around a key.
///
/// Ported from `script.odin:259` (`Modifiers :: bit_set[Modifier; u8]`).
pub type Modifiers = EnumSet<Modifier>;

/// The words for the modifiers.
///
/// Ported from `script.odin:262` (`MODIFIER_WORDS := [Modifier]string`, `@(rodata)`).
pub static MODIFIER_WORDS: EnumMap<Modifier, &str> =
    EnumMap::from_array(["shift", "ctrl", "alt", "meta"]);

/// The modifier a held key is, if it is one.
///
/// Ported from `script.odin:270` (`key_modifier -> (modifier, ok)`; the second
/// return is folded into the `Option`).
pub fn key_modifier(key: KeyName) -> Option<Modifier> {
    let KeyName::Named(named) = key else {
        return None;
    };
    match named {
        NamedKey::Shift => Some(Modifier::Shift),
        NamedKey::Control => Some(Modifier::Control),
        NamedKey::Alt => Some(Modifier::Alt),
        NamedKey::Meta => Some(Modifier::Meta),
        _ => None,
    }
}

/// What went wrong on a line.
///
/// Ported from `script.odin:288` (`Error :: struct { line, message }`; the
/// temp-allocator message is owned).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    pub line: i64,
    pub message: String,
}

/// What went wrong on a line, or nil.
///
/// Ported from `script.odin:295` (`Failure :: Maybe(string)`, private).
type Failure = Option<String>;

/// Reads a script.
///
/// Ported from `script.odin:298` (`parse`; the allocator-borrowed strings are
/// owned, so the allocator parameter is gone and `text` need not outlive the
/// result).
pub fn parse(text: &str) -> (Script, Option<Error>) {
    let mut script = Script {
        start: DEFAULT_START,
        actions: Vec::new(),
    };
    let mut actions: Vec<Action> = Vec::new();
    // An action has been read: start lines now change the page live.
    let mut started = false;
    for (i, line) in text.lines().enumerate() {
        let number = i as i64 + 1;
        let (surface, body) = surface_of(line);
        let (verb, failure) = if surface == Surface::All {
            parse_line(&mut script.start, started, body, surface)
        } else {
            // A qualified line is an action: the page opens alike on every surface.
            let mut scratch = script.start;
            let (verb, failure) = parse_line(&mut scratch, started, body, surface);
            let mut failure = failure;
            if failure.is_none() {
                let gesture = verb.as_ref().is_some_and(is_gesture);
                if !gesture {
                    failure = Some(
                        "ref and port take an input or a model line: time, shots and the page's size and media run on every surface"
                            .to_string(),
                    );
                }
            }
            (verb, failure)
        };
        if let Some(message) = failure {
            return (
                Script {
                    start: DEFAULT_START,
                    actions: Vec::new(),
                },
                Some(Error {
                    line: number,
                    message,
                }),
            );
        }
        if let Some(verb) = verb {
            started = true;
            actions.push(Action {
                line: number,
                verb,
                surface,
            });
        }
    }
    script.actions = actions;
    (script, None)
}

/// What a `ref` or `port` line may hold: an input, or a model line.
///
/// Ported from `script.odin:334` (`is_gesture`, private).
fn is_gesture(verb: &Verb) -> bool {
    matches!(
        verb,
        Verb::Move(_)
            | Verb::Press(_)
            | Verb::Click(_)
            | Verb::Wheel(_)
            | Verb::Pinch(_)
            | Verb::Key(_)
            | Verb::Hold(_)
            | Verb::Type(_)
            | Verb::Model(_)
    )
}

/// The surfaces a line runs on, and the line without its `ref` or `port`.
///
/// Ported from `script.odin:344` (`surface_of`, private).
fn surface_of(line: &str) -> (Surface, &str) {
    let trimmed = line.trim_start();
    for (word, surface) in [("ref", Surface::Ref), ("port", Surface::Port)] {
        if let Some(rest) = trimmed.strip_prefix(word)
            && (rest.is_empty() || rest.starts_with(' ') || rest.starts_with('\t'))
        {
            return (surface, rest);
        }
    }
    (Surface::All, line)
}

/// One line: a start setting, an action, or nothing.
///
/// Ported from `script.odin:359` (`parse_line`, private).
// Odin's `or_return` on a `(value, Failure)` pair: the failure becomes the
// line's, the value unwraps.
macro_rules! or_fail {
    ($e:expr) => {
        match $e {
            (Some(v), _) => v,
            (None, f) => return (None, f),
        }
    };
}

fn parse_line(
    start: &mut Start,
    started: bool,
    text: &str,
    surface: Surface,
) -> (Option<Verb>, Failure) {
    let line = text.trim();
    let end = line.find([' ', '\t']);
    let (word, args) = match end {
        Some(i) => (&line[..i], &line[i..]),
        None => (line, ""),
    };
    if word.is_empty() || word.starts_with('#') {
        if surface != Surface::All {
            return (None, Some("ref and port take an action".to_string()));
        }
        return (None, None);
    }
    if word == "type" {
        return parse_type(args);
    }
    let mut fields: Vec<&str> = args.split_whitespace().collect();
    if let Some(i) = fields.iter().position(|f| f.starts_with('#')) {
        fields.truncate(i);
    }

    match word {
        "size" => {
            let pair = or_fail!(ints2(&fields));
            if pair[0] <= 0 || pair[1] <= 0 {
                return (
                    None,
                    Some("size takes a width and a height above 0".to_string()),
                );
            }
            if started {
                return (Some(Verb::Resize(Resize { size: pair })), None);
            }
            start.size = pair;
        }
        "scheme" => {
            let scheme = or_fail!(one_of(&fields, &mv_theme::SCHEME_WORDS, "scheme"));
            if started {
                return (Some(Verb::SchemeChange(SchemeChange { scheme })), None);
            }
            start.scheme = scheme;
        }
        "reduced-motion" => {
            let on = or_fail!(on_off(&fields, "reduced-motion"));
            if started {
                return (Some(Verb::MotionChange(MotionChange { reduced: on })), None);
            }
            start.reduced_motion = on;
        }
        "density" => {
            if let Some(message) = before_actions(started, word) {
                return (None, Some(message));
            }
            let density = or_fail!(f32s::<1>(&fields));
            if density[0] <= 0.0 {
                return (None, Some("density takes a number above 0".to_string()));
            }
            start.density = density[0];
        }
        "mode" => {
            if let Some(message) = before_actions(started, word) {
                return (None, Some(message));
            }
            start.mode = or_fail!(one_of(&fields, &mv_theme::MODE_WORDS, "mode"));
        }
        "theme" => {
            if let Some(message) = before_actions(started, word) {
                return (None, Some(message));
            }
            let slug = or_fail!(one_of(&fields, &mv_theme::SLUG_WORDS, "theme"));
            if mv_theme::slug_scheme(slug) == mv_theme::Scheme::Dark {
                start.dark_theme = slug;
            } else {
                start.light_theme = slug;
            }
        }
        "space" => {
            if let Some(message) = before_actions(started, word) {
                return (None, Some(message));
            }
            start.space = or_fail!(one_of(&fields, &mv_theme::SPACE_WORDS, "space"));
        }
        "move" => {
            if fields.len() != 2 && !(fields.len() == 4 && fields[2] == "over") {
                return (None, Some("move takes X Y [over MS]".to_string()));
            }
            let to = or_fail!(f32s::<2>(&fields[..2]));
            let mut over = 0;
            if fields.len() == 4 {
                over = or_fail!(ms(fields[3]));
            }
            return (Some(Verb::Move(Move { to, over })), None);
        }
        "down" | "up" => {
            if !fields.is_empty() {
                return (None, Some(format!("{word} takes nothing")));
            }
            return (
                Some(Verb::Press(Press {
                    down: word == "down",
                })),
                None,
            );
        }
        "click" | "dblclick" => {
            let at = or_fail!(f32s::<2>(&fields));
            return (
                Some(Verb::Click(Click {
                    at,
                    count: if word == "click" { 1 } else { 2 },
                })),
                None,
            );
        }
        "wheel" => {
            if fields.len() != 2 && !(fields.len() == 3 && fields[2] == "ctrl") {
                return (None, Some("wheel takes DX DY [ctrl]".to_string()));
            }
            let delta = or_fail!(f32s::<2>(&fields[..2]));
            return (
                Some(Verb::Wheel(Wheel {
                    delta,
                    ctrl: fields.len() == 3,
                })),
                None,
            );
        }
        "pinch" => {
            let scale = or_fail!(f32s::<1>(&fields));
            if scale[0] <= 0.0 {
                return (None, Some("pinch takes a scale above 0".to_string()));
            }
            return (Some(Verb::Pinch(Pinch { scale: scale[0] })), None);
        }
        "key" | "tab" => {
            let mut key = KeyName::Named(NamedKey::Tab);
            let mut words = fields.as_slice();
            if word == "key" {
                if fields.is_empty() {
                    return (
                        None,
                        Some("key takes a key name, then modifiers".to_string()),
                    );
                }
                key = or_fail!(key_name(fields[0]));
                words = &fields[1..];
            }
            let mut mods = Modifiers::empty();
            for name in words {
                mods.insert(or_fail!(one_of(&[*name], &MODIFIER_WORDS, "a modifier")));
            }
            return (Some(Verb::Key(Key { key, mods })), None);
        }
        "hold" | "release" => {
            if fields.len() != 1 {
                return (None, Some(format!("{word} takes a key name")));
            }
            let key = or_fail!(key_name(fields[0]));
            return (
                Some(Verb::Hold(Hold {
                    key,
                    down: word == "hold",
                })),
                None,
            );
        }
        "wait" => {
            if fields.len() != 1 {
                return (None, Some("wait takes MS".to_string()));
            }
            return (
                Some(Verb::Wait(Wait {
                    ms: or_fail!(ms(fields[0])),
                })),
                None,
            );
        }
        "shot" => {
            if fields.len() != 1 {
                return (None, Some("shot takes a name".to_string()));
            }
            return (
                Some(Verb::Shot(Shot {
                    name: or_fail!(file_name(fields[0])),
                })),
                None,
            );
        }
        "frames" => {
            if fields.len() != 5 || fields[1] != "every" || fields[3] != "for" {
                return (None, Some("frames takes NAME every MS for MS".to_string()));
            }
            let name = or_fail!(file_name(fields[0]));
            let every = or_fail!(ms(fields[2]));
            let span = or_fail!(ms(fields[4]));
            if every == 0 {
                return (None, Some("frames takes a step above 0 ms".to_string()));
            }
            return (Some(Verb::Frames(Frames { name, every, span })), None);
        }
        "video" => {
            if fields.len() == 1 && fields[0] == "stop" {
                return (
                    Some(Verb::Video(Video {
                        start: false,
                        name: String::new(),
                    })),
                    None,
                );
            }
            if fields.is_empty() || fields.len() > 2 || fields[0] != "start" {
                return (None, Some("video takes start [NAME] or stop".to_string()));
            }
            let mut name = "video".to_string();
            if fields.len() == 2 {
                name = or_fail!(file_name(fields[1]));
            }
            return (Some(Verb::Video(Video { start: true, name })), None);
        }
        "model" => {
            if surface != Surface::Port {
                return (
                    None,
                    Some("model sets the port's model: it goes on a port line".to_string()),
                );
            }
            return parse_model(&fields);
        }
        _ => {
            return (None, Some(format!("unknown verb {word:?}")));
        }
    }
    (None, None)
}

/// model's forms (Model).
///
/// Ported from `script.odin:529` (`parse_model`, private).
fn parse_model(fields: &[&str]) -> (Option<Verb>, Failure) {
    if fields.is_empty() {
        return (
            None,
            Some("model takes fn, param, p, drag or p-before".to_string()),
        );
    }
    let set = or_fail!(one_of(&fields[..1], &MODEL_WORDS, "model"));
    let args = &fields[1..];
    let mut m = Model {
        set,
        name: String::new(),
        value: [0.0, 0.0],
    };
    match set {
        ModelSet::Fn => {
            m.name = or_fail!(word_in(args, &MODEL_FNS, "model fn"));
        }
        ModelSet::Param => {
            if args.len() != 2 {
                return (
                    None,
                    Some("model param takes a parameter and a value".to_string()),
                );
            }
            m.name = or_fail!(word_in(&args[..1], &MODEL_PARAMS, "model param"));
            m.value[0] = or_fail!(f32s::<1>(&args[1..]))[0] as f64;
        }
        ModelSet::P => {
            m.value[0] = or_fail!(f32s::<1>(args))[0] as f64;
        }
        ModelSet::Drag => {
            m.name = or_fail!(word_in(args, &MODEL_PARTS, "model drag"));
        }
        ModelSet::PBefore => {
            if args.len() == 1 && args[0] == "none" {
                m.name = "none".to_string();
            } else {
                let xy = or_fail!(f32s::<2>(args));
                m.value = [xy[0] as f64, xy[1] as f64];
            }
        }
    }
    (Some(Verb::Model(m)), None)
}

/// The word `fields` holds, when it is one of `words`.
///
/// Ported from `script.odin:566` (`word_in`, private).
fn word_in(fields: &[&str], words: &[&str], what: &str) -> (Option<String>, Failure) {
    if fields.len() == 1 {
        for w in words {
            if fields[0] == *w {
                return (Some((*w).to_string()), None);
            }
        }
    }
    (
        None,
        Some(format!("{what} takes one of {}", words.join(", "))),
    )
}

/// type "TEXT" [# comment].
///
/// Ported from `script.odin:579` (`parse_type`, private).
fn parse_type(text: &str) -> (Option<Verb>, Failure) {
    let args = text.trim();
    if !args.starts_with('"') {
        return (None, Some("type takes \"TEXT\"".to_string()));
    }
    let rest = &args[1..];
    let Some(end) = rest.as_bytes().iter().position(|&b| b == b'"') else {
        return (None, Some("type: no closing quote".to_string()));
    };
    let after = rest[end + 1..].trim();
    if !after.is_empty() && !after.starts_with('#') {
        return (
            None,
            Some("type: words after the closing quote".to_string()),
        );
    }
    (
        Some(Verb::Type(Type {
            text: rest[..end].to_string(),
        })),
        None,
    )
}

/// density, mode, theme and space come before the first action.
///
/// Ported from `script.odin:597` (`before_actions`, private).
fn before_actions(started: bool, word: &str) -> Failure {
    if started {
        return Some(format!(
            "{word} goes before the first action: the page opens with it"
        ));
    }
    None
}

/// A DOM key name, or one printable character.
///
/// Ported from `script.odin:605` (`key_name`, private).
fn key_name(word: &str) -> (Option<KeyName>, Failure) {
    for (named, name) in &NAMED_KEYS {
        if word == *name {
            return (Some(KeyName::Named(named)), None);
        }
    }
    if word.chars().count() == 1 {
        let r = word.chars().next().unwrap();
        if r > ' ' && r != '\u{FFFD}' {
            return (Some(KeyName::Char(r)), None);
        }
    }
    (
        None,
        Some(format!(
            "unknown key {word:?}: a DOM key name such as ArrowUp, or one character"
        )),
    )
}

/// The value whose word `fields` holds, as the only field.
///
/// Ported from `script.odin:622` (`one_of`, private; Odin's generic `[$E]string`
/// table becomes a generic function: the exact bound is the body milestone's call).
fn one_of<E>(fields: &[&str], words: &EnumMap<E, &str>, what: &str) -> (Option<E>, Failure)
where
    E: for<'a> EnumArray<&'a str>,
{
    if fields.len() == 1 {
        for (e, word) in words {
            if fields[0] == *word {
                return (Some(e), None);
            }
        }
    }
    let mut list = String::new();
    for (_, word) in words {
        if !list.is_empty() {
            list.push_str(", ");
        }
        list.push_str(word);
    }
    (None, Some(format!("{what} takes one of {list}")))
}

/// on|off.
///
/// Ported from `script.odin:639` (`on_off`, private).
fn on_off(fields: &[&str], what: &str) -> (Option<bool>, Failure) {
    if fields.len() == 1 && (fields[0] == "on" || fields[0] == "off") {
        return (Some(fields[0] == "on"), None);
    }
    (None, Some(format!("{what} takes on or off")))
}

/// Two whole numbers.
///
/// Ported from `script.odin:647` (`ints2`, private).
fn ints2(fields: &[&str]) -> (Option<[i64; 2]>, Failure) {
    if fields.len() == 2
        && let (Ok(a), Ok(b)) = (fields[0].parse::<i64>(), fields[1].parse::<i64>())
    {
        return (Some([a, b]), None);
    }
    (None, Some("expected two whole numbers".to_string()))
}

/// Exactly N numbers.
///
/// Ported from `script.odin:660` (`f32s`, private; Odin's `$N` param becomes a
/// const generic).
fn f32s<const N: usize>(fields: &[&str]) -> (Option<[f32; N]>, Failure) {
    if fields.len() != N {
        return (None, Some(format!("expected {N} numbers")));
    }
    let mut values = [0.0f32; N];
    for (i, field) in fields.iter().enumerate() {
        match field.parse::<f32>() {
            Ok(v) => values[i] = v,
            Err(_) => return (None, Some(format!("{field:?} is not a number"))),
        }
    }
    (Some(values), None)
}

/// A whole number of milliseconds.
///
/// Ported from `script.odin:675` (`ms`, private).
fn ms(field: &str) -> (Option<i64>, Failure) {
    if let Ok(value) = field.parse::<i64>()
        && value >= 0
    {
        return (Some(value), None);
    }
    (
        None,
        Some(format!("{field:?} is not a whole number of milliseconds")),
    )
}

/// A shot or video name as a file name.
///
/// Ported from `script.odin:686` (`file_name`, private).
fn file_name(word: &str) -> (Option<String>, Failure) {
    for r in word.chars() {
        match r {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' => {}
            _ => {
                return (
                    None,
                    Some(format!(
                        "{word:?}: a name takes letters, digits, '-', '_' and '.'"
                    )),
                );
            }
        }
    }
    if word.is_empty() || word.starts_with('.') {
        return (None, Some(format!("{word:?} is not a file name")));
    }
    (Some(word.to_string()), None)
}
