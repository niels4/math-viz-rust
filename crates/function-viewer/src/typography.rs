//! Type styles and labels ported from `typography.odin` (`Type`,
//! `TYPE_SELECTABLE`, `Type_Style`, `type_style`, `label`, `label_colored`,
//! `clay_type_style`, `measure_text`, `type_width`, `longest_word_width`,
//! `Text_Line`, `Overflow`, `text_line`, `draw_text_line`, `draw_text`).
//!
//! Style lookup is pure over the theme. Emitting and measuring text run the
//! `t0c` Clay bindings and the sibling `text` milestone's shapers.

use mv_sys::clay;
use mv_text::{Features, Style};
use mv_theme::{Color, Theme, Token};

use crate::app::App;

/// One text style of the page. Ported from `Type`; order matches the Odin
/// declaration so the `u16` ids packed into Clay font ids stay aligned.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, enum_map::Enum)]
#[repr(u8)]
pub enum Type {
    /// The header wordmark.
    Wordmark,
    /// The plane card heading.
    Title,
    /// The smaller heading under the wordmark and cards.
    Subtitle,
    /// A card heading.
    CardTitle,
    /// A parameter row name.
    RowName,
    /// A chip letter.
    ChipLetter,
    /// A point letter.
    PointLetter,
    /// A control role label.
    Role,
    /// A theme chip label.
    ThemeChip,
    /// A tip line.
    Tip,
    /// A scale label.
    ScaleLabel,
    /// A zoom step label.
    ZoomStep,
    /// A toggle label.
    ToggleLabel,
    /// A badge label.
    Badge,
    /// The reset-all label.
    ResetAllLabel,
    /// A moved readout label.
    MovedLabel,
    /// A moved readout value.
    MovedValue,
    /// An explainer name.
    ExplainerName,
    /// An explainer where line.
    ExplainerWhere,
    /// An explainer sentence.
    ExplainerSentence,
    /// A snippet label.
    SnippetLabel,
    /// A settings label.
    SettingsLabel,
    /// A settings option.
    SettingsOption,
    /// A settings help line.
    SettingsHelp,
    /// A settings theme name.
    SettingsTheme,
    /// A settings theme name, checked.
    SettingsThemeChecked,
}

/// Whether a style's text is one a browser shows its I-beam over. Ported
/// from `TYPE_SELECTABLE`.
pub const TYPE_SELECTABLE: [bool; 26] = [
    false, // Wordmark
    true,  // Title
    true,  // Subtitle
    true,  // CardTitle
    true,  // RowName
    true,  // ChipLetter
    true,  // PointLetter
    true,  // Role
    true,  // ThemeChip
    false, // Tip
    false, // ScaleLabel
    false, // ZoomStep
    false, // ToggleLabel
    true,  // Badge
    false, // ResetAllLabel
    true,  // MovedLabel
    true,  // MovedValue
    true,  // ExplainerName
    true,  // ExplainerWhere
    true,  // ExplainerSentence
    true,  // SnippetLabel
    true,  // SettingsLabel
    false, // SettingsOption
    true,  // SettingsHelp
    false, // SettingsTheme
    false, // SettingsThemeChecked
];

/// A style resolved for a theme. Ported from `Type_Style`.
#[derive(Clone, Copy, Debug)]
pub struct TypeStyle {
    /// The text style: face, size, tracking and features.
    pub style: Style,
    /// The line height in pixels.
    pub line_height: f32,
    /// Whether the text sets in capitals.
    pub caps: bool,
}

/// Looks up the style of one [`Type`]. Ported from `type_style`.
pub fn type_style(ty: Type, t: &Theme) -> TypeStyle {
    let face_size = |face: mv_text::Face, size: f32| Style {
        face,
        size,
        tracking: 0.0,
        features: Features::default(),
    };
    use mv_text::Face::*;
    match ty {
        Type::Wordmark => TypeStyle {
            style: Style {
                tracking: 0.24,
                ..face_size(WorkSans700, 24.0)
            },
            line_height: 28.0,
            caps: false,
        },
        Type::Title => TypeStyle {
            style: face_size(WorkSans700, 20.0),
            line_height: 23.0,
            caps: false,
        },
        Type::Subtitle => TypeStyle {
            style: Style {
                tracking: t.label_tracking,
                ..face_size(WorkSans600, 12.0)
            },
            line_height: 14.0,
            caps: true,
        },
        Type::CardTitle => TypeStyle {
            style: face_size(WorkSans700, 17.0),
            line_height: 20.0,
            caps: false,
        },
        Type::RowName => TypeStyle {
            style: face_size(WorkSans600, 15.0),
            line_height: 18.0,
            caps: false,
        },
        Type::ChipLetter => TypeStyle {
            style: face_size(Stix400Italic, 19.0),
            line_height: 24.0,
            caps: false,
        },
        Type::PointLetter => TypeStyle {
            style: face_size(Stix400Italic, 20.0),
            line_height: 25.0,
            caps: false,
        },
        Type::Role => TypeStyle {
            style: face_size(WorkSans500, 13.0),
            line_height: 15.0,
            caps: false,
        },
        Type::ThemeChip => TypeStyle {
            style: face_size(WorkSans600, 14.0),
            line_height: 16.0,
            caps: false,
        },
        Type::Tip => TypeStyle {
            style: face_size(WorkSans400, 13.0),
            line_height: 15.0,
            caps: false,
        },
        Type::ScaleLabel => TypeStyle {
            style: face_size(RobotoMono500, 13.0),
            line_height: 17.0,
            caps: false,
        },
        Type::ZoomStep => TypeStyle {
            style: face_size(WorkSans500, 20.0),
            line_height: 20.0,
            caps: false,
        },
        Type::ToggleLabel => TypeStyle {
            style: face_size(WorkSans600, 14.0),
            line_height: 16.0,
            caps: false,
        },
        Type::Badge => TypeStyle {
            style: face_size(WorkSans700, 12.0),
            line_height: 14.0,
            caps: false,
        },
        Type::ResetAllLabel => TypeStyle {
            style: face_size(WorkSans600, 13.0),
            line_height: 15.0,
            caps: false,
        },
        Type::MovedLabel => TypeStyle {
            style: face_size(WorkSans600, 13.0),
            line_height: 15.0,
            caps: false,
        },
        Type::MovedValue => TypeStyle {
            style: Style {
                face: crate::point_rows::readout_face(t, 500),
                size: 14.0,
                tracking: 0.0,
                features: mv_text::Feature::Tabular.into(),
            },
            line_height: 18.0,
            caps: false,
        },
        Type::ExplainerName => TypeStyle {
            style: face_size(WorkSans700, 16.0),
            line_height: 19.0,
            caps: false,
        },
        Type::ExplainerWhere => TypeStyle {
            style: face_size(WorkSans400, 13.0),
            line_height: 15.0,
            caps: false,
        },
        Type::ExplainerSentence => TypeStyle {
            style: face_size(WorkSans400, 15.0),
            line_height: 22.0,
            caps: false,
        },
        Type::SnippetLabel => TypeStyle {
            style: Style {
                tracking: t.label_tracking,
                ..face_size(WorkSans600, 12.0)
            },
            line_height: 14.0,
            caps: true,
        },
        Type::SettingsLabel => TypeStyle {
            style: Style {
                tracking: 0.9216,
                ..face_size(WorkSans800, 11.52)
            },
            line_height: 14.0,
            caps: true,
        },
        Type::SettingsOption => TypeStyle {
            style: face_size(WorkSans600, 14.0),
            line_height: 17.5,
            caps: false,
        },
        Type::SettingsHelp => TypeStyle {
            style: face_size(WorkSans400, 13.0),
            line_height: 18.0,
            caps: false,
        },
        Type::SettingsTheme => TypeStyle {
            style: face_size(WorkSans400, 16.0),
            line_height: 19.0,
            caps: false,
        },
        Type::SettingsThemeChecked => TypeStyle {
            style: face_size(WorkSans600, 16.0),
            line_height: 19.0,
            caps: false,
        },
    }
}

/// Sets one line of text in a [`Type`] style and a token colour. Ported from
/// `label`; needs the `t0c` Clay text binding.
pub fn label(app: &mut App, ty: Type, s: &str, ink_token: Token) {
    label_colored(app, ty, s, crate::ui::current_theme(app).colors[ink_token]);
}

/// Sets one line of text in a [`Type`] style with an explicit colour. Ported
/// from `label_colored`; needs the `t0c` Clay text binding.
pub fn label_colored(app: &mut App, ty: Type, s: &str, color: Color) {
    let t = crate::ui::current_theme(app);
    let slug = slug_id(t.slug);
    let style = type_style(ty, t);
    let shown;
    let text: &str = if style.caps {
        shown = s.to_uppercase();
        &shown
    } else {
        s
    };
    let config = clay::TextElementConfig {
        userData: text.len() as *mut std::ffi::c_void,
        textColor: [
            color[0] * 255.0,
            color[1] * 255.0,
            color[2] * 255.0,
            color[3] * 255.0,
        ],
        fontId: ty as u16 | slug << 8,
        fontSize: style.style.size as u16,
        letterSpacing: 0,
        lineHeight: style.line_height as u16,
        wrapMode: clay::TextWrapMode::None,
        textAlignment: clay::TextAlignment::Left,
    };
    let clay = app.clay.as_ref().expect("clay context initialised");
    clay.text(text, &config);
}

/// The style a Clay text element carries in its font id. Ported from
/// `clay_type_style`.
pub fn clay_type_style(app: &App, font_id: u16) -> TypeStyle {
    let ty = type_from_id(font_id & 0xff);
    let slug = slug_from_id(font_id >> 8);
    type_style(ty, &app.themes[slug])
}

/// Clay's measure of text: a word of a text element at a time, with the whole
/// text's own kerning across its spaces, so the words and Clay's spaces add
/// up to the text's advance; a space alone is Clay's own, between words. The
/// height is the style's line box. Ported from `measure_text`; needs the
/// sibling `text` milestone's shapers.
pub fn measure_text(
    app: &mut App,
    slice: &clay::StringSlice,
    config: &clay::TextElementConfig,
) -> clay::Dimensions {
    let style = clay_type_style(app, config.fontId);
    // Clay hands slices of the text it lays out; both pointers stay valid
    // for the measuring call, and the app's texts are retained for the
    // frame, so the reads are always valid UTF-8.
    let word: &str = slice.as_str();
    let whole_length = config.userData as usize;
    if word == " " || whole_length == 0 {
        return clay::Dimensions {
            width: mv_text::measure(&mut app.fonts, style.style, word),
            height: style.line_height,
        };
    }
    // The whole text the word comes from: `userData` carries its length and
    // `baseChars` its start (typography.odin's `measure_text`).
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
            style.style,
            whole,
            start as i64,
            start as i64 + word.len() as i64,
        ),
        height: style.line_height,
    }
}

/// The width of `s` in a text style, caps included. Ported from `type_width`;
/// needs the sibling `text` milestone's shapers.
pub fn type_width(app: &mut App, ty: Type, s: &str) -> f32 {
    let style = type_style(ty, crate::ui::current_theme(app));
    let shown;
    let text: &str = if style.caps {
        shown = s.to_uppercase();
        &shown
    } else {
        s
    };
    mv_text::layout_width(mv_text::measure(&mut app.fonts, style.style, text))
}

/// The widest word of `s` in a text style: its min-content width, where the
/// text may break at its spaces. Ported from `longest_word_width`; needs the
/// sibling `text` milestone's shapers.
pub fn longest_word_width(app: &mut App, ty: Type, s: &str) -> f32 {
    let style = type_style(ty, crate::ui::current_theme(app));
    let shown;
    let text: &str = if style.caps {
        shown = s.to_uppercase();
        &shown
    } else {
        s
    };
    let mut widest = 0.0f32;
    for word in text.split(' ') {
        widest = widest.max(mv_text::layout_width(mv_text::measure(
            &mut app.fonts,
            style.style,
            word,
        )));
    }
    widest
}

/// How a custom text line treats text wider than its box. Ported from
/// `Overflow`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Overflow {
    /// Cut with an ellipsis.
    Ellipsis,
    /// Left out whole.
    Whole,
}

/// One line of text in one style as a custom element, for text whose box can
/// be narrower than it. Ported from `Text_Line`.
#[derive(Clone, Debug)]
pub struct TextLine {
    /// The style to set it in.
    pub ty: Type,
    /// The text, as given; the style's caps apply when it is drawn.
    pub text: String,
    /// The colour to draw it in.
    pub ink: Color,
    /// What happens when the box is narrower than the text.
    pub overflow: Overflow,
}

/// Declares a [`TextLine`] `width` wide (its own width when `None`) and its
/// line box tall. Ported from `text_line`; needs the `t0c` Clay element
/// binding.
pub fn text_line(
    app: &mut App,
    id: clay::ElementId,
    ty: Type,
    s: &str,
    ink_token: Token,
    overflow: Overflow,
    width: Option<clay::SizingAxis>,
) {
    let t = crate::ui::current_theme(app);
    let color = t.colors[ink_token];
    let style = type_style(ty, t);
    let sizing = width.unwrap_or_else(|| crate::ui::sizing_fixed(type_width(app, ty, s)));
    let mut decl = crate::ui::empty_decl();
    decl.layout = clay::LayoutConfig {
        sizing: clay::Sizing {
            width: sizing,
            height: crate::ui::sizing_fixed(style.line_height),
        },
        padding: clay::Padding {
            left: 0,
            right: 0,
            top: 0,
            bottom: 0,
        },
        childGap: 0,
        childAlignment: clay::ChildAlignment {
            x: clay::LayoutAlignmentX::Left,
            y: clay::LayoutAlignmentY::Top,
        },
        layoutDirection: clay::LayoutDirection::LeftToRight,
    };
    decl.custom = crate::widgets::custom_data(crate::widgets::Custom::TextLine(TextLine {
        ty,
        text: s.to_string(),
        ink: color,
        overflow,
    }));
    crate::ui::ui_scope(app, id, &decl, |_| {});
}

/// Draws a [`TextLine`] on its line box's baseline, cut with an ellipsis or
/// left out whole when the box is narrower. Ported from `draw_text_line`;
/// needs the sibling `text` milestone's shapers and `app-input`'s
/// `note_text`.
pub fn draw_text_line(app: &mut App, b: clay::BoundingBox, line: &TextLine) {
    let style = type_style(line.ty, crate::ui::current_theme(app));
    let shown;
    let text: &str = if style.caps {
        shown = line.text.to_uppercase();
        &shown
    } else {
        &line.text
    };
    let y = b.y
        + mv_text::baseline(
            &mut app.fonts,
            style.style.face,
            style.style.size,
            style.line_height,
        );
    // Clay's sums of px can come out a hair under the box's own width.
    let room = b.width + 1e-3;
    let width = mv_text::measure(&mut app.fonts, style.style, text);
    if mv_text::layout_width(width) <= room {
        mv_text::draw(
            &mut app.fonts,
            &mut app.vg,
            style.style,
            b.x,
            y,
            text,
            line.ink,
        );
        if TYPE_SELECTABLE[line.ty as usize] {
            crate::input::note_text(
                app,
                clay::BoundingBox {
                    x: b.x,
                    y: b.y,
                    width,
                    height: b.height,
                },
            );
        }
        return;
    }
    if line.overflow == Overflow::Whole {
        return;
    }
    let (start, _) = mv_text::truncate(&mut app.fonts, style.style, text, room);
    mv_text::draw(
        &mut app.fonts,
        &mut app.vg,
        style.style,
        b.x,
        y,
        &start,
        line.ink,
    );
    let at = b.x + mv_text::measure(&mut app.fonts, style.style, &start);
    mv_text::draw(
        &mut app.fonts,
        &mut app.vg,
        style.style,
        at,
        y,
        mv_text::ELLIPSIS,
        line.ink,
    );
    if TYPE_SELECTABLE[line.ty as usize] {
        crate::input::note_text(app, b);
    }
}

/// Draws a line of a Clay text element. Ported from `draw_text`; needs the
/// sibling `text` milestone's shapers and `app-input`'s `note_text`.
pub fn draw_text(app: &mut App, command: &clay::RenderCommand) {
    // The walk only calls this on text commands, so the payload is there.
    let data = mv_clay::text_of(command).expect("text command holds text");
    let b = command.boundingBox;
    let style = clay_type_style(app, data.fontId);
    // Clay hands a slice of the text it laid out, valid for the draw.
    let line: &str = data.stringContents.as_str();
    let y = b.y + mv_text::baseline(&mut app.fonts, style.style.face, style.style.size, b.height);
    mv_text::draw(
        &mut app.fonts,
        &mut app.vg,
        style.style,
        b.x,
        y,
        line,
        crate::draw::nvg_color(data.textColor),
    );
    if TYPE_SELECTABLE[type_from_id(data.fontId & 0xff) as usize] {
        crate::input::note_text(app, b);
    }
}

fn type_from_id(id: u16) -> Type {
    match id {
        0 => Type::Wordmark,
        1 => Type::Title,
        2 => Type::Subtitle,
        3 => Type::CardTitle,
        4 => Type::RowName,
        5 => Type::ChipLetter,
        6 => Type::PointLetter,
        7 => Type::Role,
        8 => Type::ThemeChip,
        9 => Type::Tip,
        10 => Type::ScaleLabel,
        11 => Type::ZoomStep,
        12 => Type::ToggleLabel,
        13 => Type::Badge,
        14 => Type::ResetAllLabel,
        15 => Type::MovedLabel,
        16 => Type::MovedValue,
        17 => Type::ExplainerName,
        18 => Type::ExplainerWhere,
        19 => Type::ExplainerSentence,
        20 => Type::SnippetLabel,
        21 => Type::SettingsLabel,
        22 => Type::SettingsOption,
        23 => Type::SettingsHelp,
        24 => Type::SettingsTheme,
        25 => Type::SettingsThemeChecked,
        _ => unreachable!("clay font id names a type"),
    }
}

/// A theme's id in declaration order, as packed into Clay font ids.
fn slug_id(slug: mv_theme::Slug) -> u16 {
    match slug {
        mv_theme::Slug::ArcticIce => 0,
        mv_theme::Slug::TronCyan => 1,
        mv_theme::Slug::MidnightViolet => 2,
        mv_theme::Slug::CleanTeal => 3,
        mv_theme::Slug::EducationalWarm => 4,
        mv_theme::Slug::SageEditorial => 5,
    }
}

fn slug_from_id(id: u16) -> mv_theme::Slug {
    match id {
        0 => mv_theme::Slug::ArcticIce,
        1 => mv_theme::Slug::TronCyan,
        2 => mv_theme::Slug::MidnightViolet,
        3 => mv_theme::Slug::CleanTeal,
        4 => mv_theme::Slug::EducationalWarm,
        5 => mv_theme::Slug::SageEditorial,
        _ => unreachable!("clay font id names a theme"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use enum_map::Enum as _;

    /// Clay measures text through the text package: a text element laid out
    /// by Clay is as wide as the text package's advance of the whole text and
    /// as tall as its style's line box, in every theme. Ported from
    /// `typography_test.odin:19` (`clay_measures_through_text`).
    #[test]
    fn clay_measures_through_text() {
        let _guard = mv_clay::tests_lock();
        let mut app = crate::fixture::headless_app();
        assert!(mv_text::init(&mut app.fonts));
        for i in 0..mv_theme::Slug::LENGTH {
            let slug = mv_theme::Slug::from_usize(i);
            app.themes[slug] = mv_theme::resolve(slug, mv_theme::Space::Srgb);
        }
        crate::ui::init_clay(&mut app);

        let samples = [
            (Type::Title, "Function Viewer"),
            (Type::Subtitle, "Functions · transformations"),
            (Type::RowName, "Horizontal shift"),
            (Type::CardTitle, "Transform"),
            (Type::Tip, "Rested 400 ms"),
        ];
        for i in 0..mv_theme::Slug::LENGTH {
            let slug = mv_theme::Slug::from_usize(i);
            let scheme = mv_theme::slug_scheme(slug);
            app.system_scheme = scheme;
            app.prefs = mv_theme::showing_theme(mv_theme::DEFAULT_PREFS, slug, scheme);
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
                for s in &samples {
                    label(app, s.0, s.1, Token::Foreground);
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
                let (ty, text) = samples[found];
                found += 1;
                let style = type_style(ty, &app.themes[slug]);
                let shown = if style.caps {
                    text.to_uppercase()
                } else {
                    text.to_string()
                };
                let want = mv_text::measure(&mut app.fonts, style.style, &shown);
                let b = command.boundingBox;
                assert!(
                    (b.width - want).abs() < 1e-3 && b.height == style.line_height,
                    "{slug:?} {ty:?} {shown:?}: Clay laid it out {} x {}, the text package measures {want} x {}",
                    b.width,
                    b.height,
                    style.line_height,
                );
            }
            assert_eq!(found, samples.len());
        }
        crate::ui::release_clay(&mut app);
        mv_text::destroy(&mut app.fonts);
    }

    #[test]
    fn table_covers_every_type_in_order() {
        let order = [
            Type::Wordmark,
            Type::Title,
            Type::Subtitle,
            Type::CardTitle,
            Type::RowName,
            Type::ChipLetter,
            Type::PointLetter,
            Type::Role,
            Type::ThemeChip,
            Type::Tip,
            Type::ScaleLabel,
            Type::ZoomStep,
            Type::ToggleLabel,
            Type::Badge,
            Type::ResetAllLabel,
            Type::MovedLabel,
            Type::MovedValue,
            Type::ExplainerName,
            Type::ExplainerWhere,
            Type::ExplainerSentence,
            Type::SnippetLabel,
            Type::SettingsLabel,
            Type::SettingsOption,
            Type::SettingsHelp,
            Type::SettingsTheme,
            Type::SettingsThemeChecked,
        ];
        for (i, ty) in order.iter().enumerate() {
            assert_eq!(*ty as usize, i);
            assert_eq!(type_from_id(i as u16), *ty);
        }
        assert_eq!(TYPE_SELECTABLE.len(), order.len());
    }
}
