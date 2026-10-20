//! Ported from `src/script/script_test.odin` (227 lines): the five `@(test)`s.
//!
//! `state_scripts_parse` reads `tools/states/` and `tools/live/` (`r0`'s
//! copies) through `CARGO_MANIFEST_DIR`, as the Odin test does with
//! `#load_directory`. File reads stay out of the library for the
//! `wasm32-unknown-emscripten` build.
#![cfg(not(target_os = "emscripten"))]

use mv_script::*;
use mv_theme::{Mode, Scheme, Slug, Space};

#[test]
fn empty_script_opens_with_the_defaults() {
    let (script, err) = parse("# nothing but a comment\n\n");
    assert!(err.is_none());
    assert_eq!(script.start, DEFAULT_START);
    assert_eq!(script.actions.len(), 0);
}

// Reaches `mv_theme::slug_scheme`, whose body merges with theme.
#[test]
fn every_verb() {
    let text = "size 800 600
density 1.5
scheme light
mode dark
theme midnight-violet
theme sage-editorial
reduced-motion on
space p3
move 10 20
\tmove 30.5 40 over 160  # a comment
down
up
click 1 2
dblclick 3 4
wheel 0 -120
wheel 0 100 ctrl
pinch 1.25
key ArrowUp shift
key + ctrl alt
tab
tab shift
hold Shift
release Shift
type \"1.5 # not a comment\"   # but this is
wait 80
shot rest
frames first-paint every 80 for 400
video start sweep
video stop
video start
size 460 900
scheme dark
reduced-motion off
";
    let (script, err) = parse(text);
    assert!(err.is_none(), "unexpected error: {:?}", err);
    assert_eq!(
        script.start,
        Start {
            size: [800, 600],
            density: 1.5,
            scheme: Scheme::Light,
            mode: Mode::Dark,
            dark_theme: Slug::MidnightViolet,
            light_theme: Slug::SageEditorial,
            reduced_motion: true,
            space: Space::DisplayP3,
            query: String::new(),
        }
    );
    let expected = vec![
        Verb::Move(Move {
            to: [10.0, 20.0],
            over: 0,
        }),
        Verb::Move(Move {
            to: [30.5, 40.0],
            over: 160,
        }),
        Verb::Press(Press { down: true }),
        Verb::Press(Press { down: false }),
        Verb::Click(Click {
            at: [1.0, 2.0],
            count: 1,
        }),
        Verb::Click(Click {
            at: [3.0, 4.0],
            count: 2,
        }),
        Verb::Wheel(Wheel {
            delta: [0.0, -120.0],
            ctrl: false,
        }),
        Verb::Wheel(Wheel {
            delta: [0.0, 100.0],
            ctrl: true,
        }),
        Verb::Pinch(Pinch { scale: 1.25 }),
        Verb::Key(Key {
            key: KeyName::Named(NamedKey::ArrowUp),
            mods: Modifier::Shift.into(),
        }),
        Verb::Key(Key {
            key: KeyName::Char('+'),
            mods: Modifier::Control | Modifier::Alt,
        }),
        Verb::Key(Key {
            key: KeyName::Named(NamedKey::Tab),
            mods: Modifiers::empty(),
        }),
        Verb::Key(Key {
            key: KeyName::Named(NamedKey::Tab),
            mods: Modifier::Shift.into(),
        }),
        Verb::Hold(Hold {
            key: KeyName::Named(NamedKey::Shift),
            down: true,
        }),
        Verb::Hold(Hold {
            key: KeyName::Named(NamedKey::Shift),
            down: false,
        }),
        Verb::Type(Type {
            text: "1.5 # not a comment".to_string(),
        }),
        Verb::Wait(Wait { ms: 80 }),
        Verb::Shot(Shot {
            name: "rest".to_string(),
        }),
        Verb::Frames(Frames {
            name: "first-paint".to_string(),
            every: 80,
            span: 400,
        }),
        Verb::Video(Video {
            start: true,
            name: "sweep".to_string(),
        }),
        Verb::Video(Video {
            start: false,
            name: String::new(),
        }),
        Verb::Video(Video {
            start: true,
            name: "video".to_string(),
        }),
        Verb::Resize(Resize { size: [460, 900] }),
        Verb::SchemeChange(SchemeChange {
            scheme: Scheme::Dark,
        }),
        Verb::MotionChange(MotionChange { reduced: false }),
    ];
    assert_eq!(
        script.actions.len(),
        expected.len(),
        "actions: {:?}",
        script.actions
    );
    for (i, verb) in expected.into_iter().enumerate() {
        assert_eq!(script.actions[i].verb, verb, "action {i}");
    }
    assert_eq!(script.actions[0].line, 9);
}

#[test]
fn lines_for_one_side() {
    let text = "size 1440 900
ref click 383 424
ref type \"2\"
\tport model param a 2
port model param k -1.5
port model p 0.5
port model fn sin
port model drag k
port model drag none
port model p-before 2 4
port model p-before none
wait 16
";
    let (script, err) = parse(text);
    assert!(err.is_none(), "unexpected error: {:?}", err);
    let expected = vec![
        Action {
            line: 2,
            verb: Verb::Click(Click {
                at: [383.0, 424.0],
                count: 1,
            }),
            surface: Surface::Ref,
        },
        Action {
            line: 3,
            verb: Verb::Type(Type {
                text: "2".to_string(),
            }),
            surface: Surface::Ref,
        },
        Action {
            line: 4,
            verb: Verb::Model(Model {
                set: ModelSet::Param,
                name: "a".to_string(),
                value: [2.0, 0.0],
            }),
            surface: Surface::Port,
        },
        Action {
            line: 5,
            verb: Verb::Model(Model {
                set: ModelSet::Param,
                name: "k".to_string(),
                value: [-1.5, 0.0],
            }),
            surface: Surface::Port,
        },
        Action {
            line: 6,
            verb: Verb::Model(Model {
                set: ModelSet::P,
                name: String::new(),
                value: [0.5, 0.0],
            }),
            surface: Surface::Port,
        },
        Action {
            line: 7,
            verb: Verb::Model(Model {
                set: ModelSet::Fn,
                name: "sin".to_string(),
                value: [0.0, 0.0],
            }),
            surface: Surface::Port,
        },
        Action {
            line: 8,
            verb: Verb::Model(Model {
                set: ModelSet::Drag,
                name: "k".to_string(),
                value: [0.0, 0.0],
            }),
            surface: Surface::Port,
        },
        Action {
            line: 9,
            verb: Verb::Model(Model {
                set: ModelSet::Drag,
                name: "none".to_string(),
                value: [0.0, 0.0],
            }),
            surface: Surface::Port,
        },
        Action {
            line: 10,
            verb: Verb::Model(Model {
                set: ModelSet::PBefore,
                name: String::new(),
                value: [2.0, 4.0],
            }),
            surface: Surface::Port,
        },
        Action {
            line: 11,
            verb: Verb::Model(Model {
                set: ModelSet::PBefore,
                name: "none".to_string(),
                value: [0.0, 0.0],
            }),
            surface: Surface::Port,
        },
        Action {
            line: 12,
            verb: Verb::Wait(Wait { ms: 16 }),
            surface: Surface::All,
        },
    ];
    assert_eq!(
        script.actions.len(),
        expected.len(),
        "actions: {:?}",
        script.actions
    );
    for (i, action) in expected.into_iter().enumerate() {
        assert_eq!(script.actions[i], action, "action {i}");
    }
    assert_eq!(script.start.size, [1440, 900]);
}

#[test]
fn query_opens_the_page_with_the_hash_query() {
    let (script, err) = parse("query center=1,2&scale=2\nwait 16\nshot rest\n");
    assert!(err.is_none(), "unexpected error: {:?}", err);
    assert_eq!(script.start.query, "center=1,2&scale=2");
    assert_eq!(script.actions.len(), 2);

    for text in [
        "query\nwait 1\n",
        "query a=1 b=2\nwait 1\n",
        "wait 1\nquery a=1\n",
        "ref query a=1\nwait 1\n",
    ] {
        let (_, err) = parse(text);
        assert!(err.is_some(), "no error for {text:?}");
    }
}

#[test]
// One case (`ref theme tron-cyan`) reaches `mv_theme::slug_scheme`.
fn errors_name_their_line() {
    struct Case {
        text: &'static str,
        line: i64,
        says: &'static str,
    }
    let cases = vec![
        Case {
            text: "jump 1 2",
            line: 1,
            says: "unknown verb",
        },
        Case {
            text: "move 1",
            line: 1,
            says: "move takes",
        },
        Case {
            text: "\nwait",
            line: 2,
            says: "wait takes",
        },
        Case {
            text: "wait -5",
            line: 1,
            says: "milliseconds",
        },
        Case {
            text: "shot a/b",
            line: 1,
            says: "a name takes",
        },
        Case {
            text: "move 1 2\ndensity 2",
            line: 2,
            says: "before the first action",
        },
        Case {
            text: "move 1 2\ntheme tron-cyan",
            line: 2,
            says: "before the first action",
        },
        Case {
            text: "move 1 2\nspace p3",
            line: 2,
            says: "before the first action",
        },
        Case {
            text: "space display-p3",
            line: 1,
            says: "space takes one of srgb, p3",
        },
        Case {
            text: "theme nope",
            line: 1,
            says: "theme takes one of arctic-ice, tron-cyan",
        },
        Case {
            text: "type \"open",
            line: 1,
            says: "no closing quote",
        },
        Case {
            text: "type \"a\" b",
            line: 1,
            says: "after the closing quote",
        },
        Case {
            text: "key Arrowup",
            line: 1,
            says: "unknown key",
        },
        Case {
            text: "key a shiftt",
            line: 1,
            says: "a modifier takes one of shift, ctrl, alt, meta",
        },
        Case {
            text: "frames f every 0 for 10",
            line: 1,
            says: "above 0 ms",
        },
        Case {
            text: "video start a b",
            line: 1,
            says: "video takes",
        },
        Case {
            text: "size 0 900",
            line: 1,
            says: "above 0",
        },
        Case {
            text: "model p 1",
            line: 1,
            says: "goes on a port line",
        },
        Case {
            text: "ref model p 1",
            line: 1,
            says: "goes on a port line",
        },
        Case {
            text: "port model q 1",
            line: 1,
            says: "model takes one of fn, param, p, drag, p-before",
        },
        Case {
            text: "port model param z 1",
            line: 1,
            says: "model param takes one of a, k, b, h",
        },
        Case {
            text: "port model param a",
            line: 1,
            says: "a parameter and a value",
        },
        Case {
            text: "port model drag q",
            line: 1,
            says: "model drag takes one of",
        },
        Case {
            text: "port model p-before 1",
            line: 1,
            says: "expected 2 numbers",
        },
        Case {
            text: "ref theme tron-cyan",
            line: 1,
            says: "take an input or a model line",
        },
        Case {
            text: "port density 2",
            line: 1,
            says: "take an input or a model line",
        },
        Case {
            text: "move 1 2\nref wait 10",
            line: 2,
            says: "take an input or a model line",
        },
        Case {
            text: "move 1 2\nport shot a",
            line: 2,
            says: "take an input or a model line",
        },
        Case {
            text: "ref",
            line: 1,
            says: "take an action",
        },
    ];
    for c in cases {
        let (_, err) = parse(c.text);
        match err {
            Some(e) => assert!(
                e.line == c.line && e.message.contains(c.says),
                "{:?}: got line {} {:?}, want line {} {:?}",
                c.text,
                e.line,
                e.message,
                c.line,
                c.says,
            ),
            None => panic!("{:?}: expected an error, parsed clean", c.text),
        }
    }
}

// Every script in tools/states and tools/live parses, so a mistake shows
// here before a run. Reaches `mv_theme::slug_scheme` through the scripts'
// `theme` lines.
#[test]
fn state_scripts_parse() {
    expect_parse("tools/states");
    expect_parse("tools/live");
}

fn expect_parse(dir: &str) {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let mut scripts = 0;
    let entries = std::fs::read_dir(format!("{root}/{dir}")).unwrap();
    for entry in entries {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("act") {
            continue;
        }
        scripts += 1;
        let text = std::fs::read_to_string(&path).unwrap();
        let (_, err) = parse(&text);
        assert!(
            err.is_none(),
            "{}/{}: {:?}",
            dir,
            path.file_name().unwrap().to_string_lossy(),
            err.map(|e| (e.line, e.message)),
        );
    }
    assert!(scripts > 0, "no scripts in {dir}");
}
