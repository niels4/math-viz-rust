//! Text measurement for Clay: the font families in `assets/fonts/` behind
//! Clay font ids.
//!
//! The page milestones rebuild the text styles from math-viz2's CSS on top of
//! this: a style per CSS class, with its face, size, tracking and line box.
//! What stays here is the family table (font id to `mv_text::Face`, in the
//! order below) and the measure callback `ui.rs` installs.

use mv_sys::clay;
use mv_text::{Face, Features, Style};

use crate::app::App;

/// The font id's face: the families and weights `assets/fonts/` ships, in
/// `mv_text::Face` order.
fn face_from_id(id: u16) -> Face {
    match id {
        0 => Face::WorkSans400,
        1 => Face::WorkSans500,
        2 => Face::WorkSans600,
        3 => Face::WorkSans700,
        4 => Face::WorkSans800,
        5 => Face::Stix400,
        6 => Face::Stix500,
        7 => Face::Stix400Italic,
        8 => Face::RobotoMono500,
        9 => Face::RobotoMono600,
        10 => Face::Stix600,
        _ => unreachable!("clay font id names a shipped face"),
    }
}

/// Clay's measure of text: a word of a text element at a time, with the whole
/// text's own kerning across its spaces, so the words and Clay's spaces add
/// up to the text's advance; a space alone is Clay's own, between words. The
/// height is the config's line box.
pub fn measure_text(
    app: &mut App,
    slice: &clay::StringSlice,
    config: &clay::TextElementConfig,
) -> clay::Dimensions {
    // The theme rides in the font id's high byte, as before; the measure
    // needs only the face, so the theme half is unread here.
    let style = Style {
        face: face_from_id(config.fontId & 0xff),
        size: f32::from(config.fontSize),
        tracking: 0.0,
        features: Features::default(),
    };
    let height = f32::from(config.lineHeight);
    // Clay hands slices of the text it lays out; both pointers stay valid
    // for the measuring call, and the app's texts are retained for the
    // frame, so the reads are always valid UTF-8.
    let word: &str = slice.as_str();
    let whole_length = config.userData as usize;
    if word == " " || whole_length == 0 {
        return clay::Dimensions {
            width: mv_text::measure(&mut app.fonts, style, word),
            height,
        };
    }
    // The whole text the word comes from: `userData` carries its length and
    // `baseChars` its start.
    let view = clay::StringSlice {
        length: whole_length as core::ffi::c_int,
        chars: slice.baseChars,
        baseChars: slice.baseChars,
    };
    let whole: &str = view.as_str();
    let start = (slice.chars as usize) - (slice.baseChars as usize);
    clay::Dimensions {
        width: mv_text::measure_word(
            &mut app.fonts,
            style,
            whole,
            start as i64,
            start as i64 + word.len() as i64,
        ),
        height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Clay measures text through the text package: a text element laid out
    /// by Clay is as wide as the text package's advance of the whole text and
    /// as tall as its line box, in every face.
    #[test]
    fn clay_measures_through_text() {
        let _guard = mv_clay::tests_lock();
        let mut app = crate::fixture::headless_app();
        assert!(mv_text::init(&mut app.fonts));
        crate::ui::init_clay(&mut app);

        let faces = [
            (Face::WorkSans400, "Function Viewer", 24.0),
            (Face::WorkSans600, "Horizontal shift", 15.0),
            (Face::Stix400Italic, "x", 19.0),
            (Face::RobotoMono500, "12.5", 13.0),
        ];
        app.clay
            .as_mut()
            .expect("clay context initialised")
            .set_layout_dimensions(clay::Dimensions {
                width: 1440.0,
                height: 900.0,
            });
        app.clay
            .as_mut()
            .expect("clay context initialised")
            .begin_layout();
        let mut decl = crate::ui::empty_decl();
        decl.layout.layoutDirection = clay::LayoutDirection::TopToBottom;
        crate::ui::ui_scope(&mut app, mv_clay::id("Texts", 0), &decl, |app| {
            for (face, text, size) in &faces {
                let config = clay::TextElementConfig {
                    userData: text.len() as *mut std::ffi::c_void,
                    textColor: [0.0, 0.0, 0.0, 255.0],
                    fontId: *face as u16,
                    fontSize: *size as u16,
                    letterSpacing: 0,
                    lineHeight: *size as u16,
                    wrapMode: clay::TextWrapMode::None,
                    textAlignment: clay::TextAlignment::Left,
                };
                let clay = app.clay.as_ref().expect("clay context initialised");
                clay.text(text, &config);
            }
        });
        let commands: Vec<clay::RenderCommand> = app
            .clay
            .as_mut()
            .expect("clay context initialised")
            .end_layout()
            .to_vec();
        let mut found = 0;
        for command in &commands {
            if command.commandType != clay::RenderCommandType::Text {
                continue;
            }
            let (face, text, size) = faces[found];
            found += 1;
            let style = Style {
                face,
                size,
                tracking: 0.0,
                features: Features::default(),
            };
            let want = mv_text::measure(&mut app.fonts, style, text);
            let b = command.boundingBox;
            assert!(
                (b.width - want).abs() < 1e-3 && b.height == size,
                "{face:?} {text:?}: Clay laid it out {} x {}, the text package measures {want} x {size}",
                b.width,
                b.height,
            );
        }
        assert_eq!(found, faces.len());
        crate::ui::release_clay(&mut app);
        mv_text::destroy(&mut app.fonts);
    }

    /// The face table covers every shipped face in order.
    #[test]
    fn table_covers_every_face_in_order() {
        use enum_map::Enum as _;
        let order = [
            Face::WorkSans400,
            Face::WorkSans500,
            Face::WorkSans600,
            Face::WorkSans700,
            Face::WorkSans800,
            Face::Stix400,
            Face::Stix500,
            Face::Stix400Italic,
            Face::RobotoMono500,
            Face::RobotoMono600,
            Face::Stix600,
        ];
        assert_eq!(Face::LENGTH, order.len());
        for (i, face) in order.iter().enumerate() {
            assert_eq!(*face as usize, i);
            assert_eq!(face_from_id(i as u16), *face);
        }
    }
}
