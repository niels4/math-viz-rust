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

/// The page's text styles, rebuilt from math-viz2's CSS: one per class that
/// sets type (TopBar's wordmark, title and subtitle at this hash; the
/// settings menu's come with h1-settings).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Type {
    /// `TopBar.module.css .wordmark_text`: Work Sans 700, 24 px, 0.01 em.
    Wordmark,
    /// `TopBar.module.css .title`: Work Sans 700, 20 px.
    Title,
    /// `TopBar.module.css .subtitle`: Work Sans 600, 12 px, 0.08 em, uppercase.
    Subtitle,
    /// The sidebar stub's copy at this hash: Work Sans 400, 16 px.
    Body,
}

/// A style's face, em size, tracking and line box, as its CSS class sets them.
pub struct TypeStyle {
    pub face: Face,
    pub size: f32,
    pub tracking: f32,
    pub line_height: f32,
}

/// The style a type sets: sizes and tracking from the CSS above (0.01 em of
/// 24 px is 0.24 px; 0.08 em of 12 px is 0.96 px), line boxes from the
/// reference's measured boxes (28, 24 and 14 px).
pub fn type_style(ty: Type) -> TypeStyle {
    match ty {
        Type::Wordmark => TypeStyle {
            face: Face::WorkSans700,
            size: 24.0,
            tracking: 0.24,
            line_height: 28.0,
        },
        Type::Title => TypeStyle {
            face: Face::WorkSans700,
            size: 20.0,
            tracking: 0.0,
            line_height: 24.0,
        },
        Type::Subtitle => TypeStyle {
            face: Face::WorkSans600,
            size: 12.0,
            tracking: 0.96,
            line_height: 14.0,
        },
        // The stub's `normal` line box, measured on the reference.
        Type::Body => TypeStyle {
            face: Face::WorkSans400,
            size: 16.0,
            tracking: 0.0,
            line_height: 19.0,
        },
    }
}

fn type_from_id(id: u16) -> Type {
    match id {
        0 => Type::Wordmark,
        1 => Type::Title,
        2 => Type::Subtitle,
        3 => Type::Body,
        _ => unreachable!("clay font id names a text style"),
    }
}

/// Clay's measure of text: a word of a text element at a time, with the whole
/// text's own kerning across its spaces, so the words and Clay's spaces add
/// up to the text's advance; a space alone is Clay's own, between words. The
/// height is the style's line box.
pub fn measure_text(
    app: &mut App,
    slice: &clay::StringSlice,
    config: &clay::TextElementConfig,
) -> clay::Dimensions {
    // The font id names the text style; the style owns the face, size and
    // tracking, so the config's own numbers stay unread here.
    let typed = type_style(type_from_id(config.fontId & 0xff));
    let style = Style {
        face: typed.face,
        size: typed.size,
        tracking: typed.tracking,
        features: Features::default(),
    };
    let height = typed.line_height;
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

/// Lays out one line of text in a style: the Clay text element with the
/// style's id, size and line box, in `ink` (a Clay 0-to-255 colour).
pub fn text(app: &mut App, id: clay::ElementId, ty: Type, content: &str, ink: clay::Color) {
    let styled = type_style(ty);
    let mut sizing = crate::ui::empty_decl();
    sizing.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_fit(crate::ui::ZERO_MINMAX),
        height: mv_clay::sizing_fit(crate::ui::ZERO_MINMAX),
    };
    crate::ui::ui_scope(app, id, &sizing, |app| {
        let config = clay::TextElementConfig {
            userData: content.len() as *mut std::ffi::c_void,
            textColor: ink,
            fontId: ty as u16,
            fontSize: styled.size as u16,
            letterSpacing: 0,
            lineHeight: styled.line_height as u16,
            wrapMode: clay::TextWrapMode::None,
            textAlignment: clay::TextAlignment::Left,
        };
        let clay = app.clay.as_ref().expect("clay context initialised");
        clay.text(content, &config);
    });
}

/// Paints a Clay text command in its style's face and box: the shaped run at
/// the box's left, on the style's baseline.
pub fn draw_text(app: &mut App, command: &clay::RenderCommand) {
    // The walk only calls this on text commands, so the payload is there.
    let data = mv_clay::text_of(command).expect("text command holds text");
    let b = command.boundingBox;
    let styled = type_style(type_from_id(data.fontId & 0xff));
    let style = Style {
        face: styled.face,
        size: styled.size,
        tracking: styled.tracking,
        features: Features::default(),
    };
    // Clay hands a slice of the text it laid out, valid for the draw.
    let line: &str = data.stringContents.as_str();
    let y = b.y + mv_text::baseline(&mut app.fonts, styled.face, styled.size, styled.line_height);
    mv_text::draw(
        &mut app.fonts,
        &mut app.vg,
        style,
        b.x,
        y,
        line,
        crate::draw::nvg_color(data.textColor),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Clay measures text through the text package: a text element laid out
    /// by Clay is as wide as the text package's advance of the whole text and
    /// as tall as its style's line box, in every page style.
    #[test]
    fn clay_measures_through_text() {
        let _guard = mv_clay::tests_lock();
        let mut app = crate::fixture::headless_app();
        assert!(mv_text::init(&mut app.fonts));
        crate::ui::init_clay(&mut app);

        let lines = [
            (Type::Wordmark, "MathViz"),
            (Type::Title, "Function Viewer"),
            (Type::Subtitle, "FUNCTIONS · TRANSFORMATIONS"),
            (Type::Body, "Sidebar!"),
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
            for (ty, text) in &lines {
                let styled = type_style(*ty);
                let config = clay::TextElementConfig {
                    userData: text.len() as *mut std::ffi::c_void,
                    textColor: [0.0, 0.0, 0.0, 255.0],
                    fontId: *ty as u16,
                    fontSize: styled.size as u16,
                    letterSpacing: 0,
                    lineHeight: styled.line_height as u16,
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
            let (ty, text) = lines[found];
            found += 1;
            let styled = type_style(ty);
            let style = Style {
                face: styled.face,
                size: styled.size,
                tracking: styled.tracking,
                features: Features::default(),
            };
            let want = mv_text::measure(&mut app.fonts, style, text);
            let b = command.boundingBox;
            assert!(
                (b.width - want).abs() < 1e-3 && b.height == styled.line_height,
                "{ty:?} {text:?}: Clay laid it out {} x {}, the text package measures {want} x {}",
                b.width,
                b.height,
                styled.line_height,
            );
        }
        assert_eq!(found, lines.len());
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
