//! The shared tip component: `Tip`, `Key` and `ArrowKey`.
//!
//! Ported from `src/components/ui/Tip.tsx` and `Tip.module.css` at `96f1e84`
//! (commit `37bcf5e` made the tip shared: `ScrubValue.tsx` renders both of
//! its tips through it with the same look it had, and the P row's edit tip
//! renders through it too). The tip is a floating plate under its anchor:
//! wrapping words and key caps in a 4 px gap, 7 px 10 px padding, 13 px on
//! 15 px sans, the foreground ink on the card with a 1 px border at the
//! card radius; the error tone inks the border and the words destructive
//! (key caps keep the foreground ink: `.key` sets its own colour).
//!
//! The look lives here; the content stays with the callers (a scrub tip's
//! words in [`crate::scrub_value`], the P edit tip's in the points section).
//! Each caller passes its placement ([`TipPlace`]: the laid-out parent, the
//! drop below it, the paint order) and its element ids ([`TipIds`]), so the
//! scrub tip (drop 10 px under the box) and the P tip (drop 4 px under the
//! head) share the one paint. Pointer-transparent: the tip
//! declares no regions.
//!
//! Phase 2 wires this in: `scrub_value.rs`'s tip layout and arrow paint move
//! onto [`layout_tip`] and [`paint_arrows`] with no pixel change (every H5
//! `box-tip-*` state passing is the guard), and the P row's edit tip calls
//! [`layout_tip`] with [`TipTone::Error`] when its field edit is invalid.

use mv_sys::clay;

use crate::app::App;

/// The tip's inner gap (4 px).
pub const TIP_GAP: f32 = 4.0;
/// The tip's horizontal padding (10 px; the 1 px border joins it, since
/// borders take no layout space).
pub const TIP_PAD_X: f32 = 10.0;
/// The tip's vertical padding (7 px).
pub const TIP_PAD_Y: f32 = 7.0;
/// The tip's text: sans 400, 13 px.
pub const TIP_FONT_SIZE: f32 = 13.0;
/// The tip's text line height (15 px).
pub const TIP_LINE_HEIGHT: f32 = 15.0;
/// The tip's border (1 px) and corner radius (the card radius, 4 px).
pub const TIP_BORDER: f32 = 1.0;
/// The tip's corner radius (the card radius, as the scrub box's).
pub const TIP_RADIUS: f32 = 4.0;

/// A key cap: 21 px tall, mono 600 12 px/16 px, 1 px 6 px 2 px padding (the
/// side borders join the horizontal padding; the fixed height holds the
/// vertical ones, as CSS `box-sizing` does), the secondary plate with a 1 px
/// border and a 2 px bottom, at the card radius.
pub const KEY_HEIGHT: f32 = 21.0;
/// A key cap's font size.
pub const KEY_FONT_SIZE: f32 = 12.0;
/// A key cap's line height.
pub const KEY_LINE_HEIGHT: f32 = 16.0;
/// A key cap's horizontal padding.
pub const KEY_PAD_X: f32 = 6.0;
/// A key cap's top padding.
pub const KEY_PAD_TOP: u16 = 1;
/// A key cap's bottom padding (the 2 px bottom border).
pub const KEY_PAD_BOTTOM: u16 = 2;

/// An arrow key's glyph (`ArrowKey`): 18 × 14, 1.6 px round stroke.
pub const ARROW_WIDTH: f32 = 18.0;
/// An arrow glyph's height.
pub const ARROW_HEIGHT: f32 = 14.0;
/// An arrow glyph's stroke width (the chevron stroke).
pub const ARROW_STROKE: f32 = 1.6;

/// The tip's tone (`tone?: "error"`): the default foreground look, or the
/// destructive error look for an edit that doesn't parse.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TipTone {
    Default,
    Error,
}

/// An arrow key's direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArrowDir {
    Up,
    Down,
}

/// One tip cell: a key cap (`Key`), an arrow glyph (`ArrowKey`), or plain
/// words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TipCell {
    Key(String),
    Arrow(ArrowDir),
    Words(String),
}

/// The tip's element ids, one namespace per caller (the rows' `param_index`
/// times 16 in the scrub tip's case, as before).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TipIds {
    /// The tip plate.
    pub tip: &'static str,
    /// A word run or key label.
    pub text: &'static str,
    /// A key cap plate.
    pub key: &'static str,
    /// An arrow glyph box.
    pub arrow: &'static str,
}

/// Where the tip floats: right-flush under its laid-out parent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TipPlace {
    /// The parent element's id (`parentId`): the box for a scrub tip, the
    /// row's head for the P tip.
    pub parent: u32,
    /// The drop below the parent's bottom edge (10 px for a scrub tip: the
    /// CSS 11 px from the padding box minus the 1 px border; 4 px for the P
    /// tip: 34 px below the row over its 30 px head).
    pub drop: f32,
    /// The paint order (`z-index`: 1 for a scrub tip, 3 for the P tip).
    pub z: i16,
}

/// Lays out the shared tip plate under its anchor: the tone's border on the
/// card, the cells in a wrapping row. Call at the end of the anchor's scope,
/// as the scrub tip does in the box's.
pub fn layout_tip(
    app: &mut App,
    ids: &TipIds,
    index: u32,
    place: &TipPlace,
    tone: TipTone,
    cells: &[TipCell],
) {
    let theme = crate::ui::current_theme(app).clone();
    let ink = |token| crate::ui::ink(&theme, token);
    let mut decl = crate::ui::empty_decl();
    decl.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_fit(crate::ui::ZERO_MINMAX),
        height: mv_clay::sizing_fit(crate::ui::ZERO_MINMAX),
    };
    decl.layout.padding = clay::Padding {
        // The 1 px border takes no layout space, so it joins the padding
        // (the settings menu's `bordered` rule).
        left: TIP_PAD_X as u16 + 1,
        right: TIP_PAD_X as u16 + 1,
        top: TIP_PAD_Y as u16 + 1,
        bottom: TIP_PAD_Y as u16 + 1,
    };
    decl.layout.childGap = TIP_GAP as u16;
    decl.layout.childAlignment = clay::ChildAlignment {
        x: clay::LayoutAlignmentX::Left,
        y: clay::LayoutAlignmentY::Center,
    };
    decl.backgroundColor = ink(mv_theme::Token::Card);
    decl.border = clay::BorderElementConfig {
        color: match tone {
            TipTone::Default => ink(mv_theme::Token::Border),
            TipTone::Error => ink(mv_theme::Token::Destructive),
        },
        width: mv_clay::border_outside(1),
    };
    decl.cornerRadius = crate::ui::corner_radius_all(TIP_RADIUS);
    decl.floating = clay::FloatingElementConfig {
        // Right-flush under the parent (RightTop to RightBottom), as the
        // offsets below the box and the head both are; the CSS `right`
        // describes the same flush against the padding box.
        offset: [0.0, place.drop],
        expand: clay::Dimensions {
            width: 0.0,
            height: 0.0,
        },
        parentId: place.parent,
        zIndex: place.z,
        attachment: clay::FloatingAttachPoints {
            element: clay::FloatingAttachPointType::RightTop,
            parent: clay::FloatingAttachPointType::RightBottom,
        },
        pointerCaptureMode: clay::PointerCaptureMode::Passthrough,
        attachTo: clay::FloatingAttachToElement::ElementWithId,
        clipTo: clay::FloatingClipToElement::None,
    };
    crate::ui::ui_scope(app, mv_clay::id(ids.tip, index), &decl, |app| {
        for (k, cell) in cells.iter().enumerate() {
            tip_cell(app, ids, index + k as u32, tone, cell);
        }
    });
}

/// One tip cell: a key cap, an arrow glyph in its cap, or plain words (the
/// error tone's destructive ink; key labels keep the foreground ink, as
/// `.key` sets its own colour).
fn tip_cell(app: &mut App, ids: &TipIds, id: u32, tone: TipTone, cell: &TipCell) {
    let theme = crate::ui::current_theme(app).clone();
    let ink = |token| crate::ui::ink(&theme, token);
    match cell {
        TipCell::Words(words) => {
            crate::typography::text(
                app,
                mv_clay::id(ids.text, id),
                crate::typography::Type::TipText,
                words,
                match tone {
                    TipTone::Default => ink(mv_theme::Token::Foreground),
                    TipTone::Error => ink(mv_theme::Token::Destructive),
                },
                false,
            );
        }
        TipCell::Key(label) => {
            key_cap(app, ids, id, &ink, |app| {
                crate::typography::text(
                    app,
                    mv_clay::id(ids.text, id),
                    crate::typography::Type::KeyText,
                    label,
                    ink(mv_theme::Token::Foreground),
                    false,
                );
            });
        }
        TipCell::Arrow(_) => {
            key_cap(app, ids, id, &ink, |app| {
                let mut glyph = crate::ui::empty_decl();
                glyph.layout.sizing = clay::Sizing {
                    width: mv_clay::sizing_fixed(ARROW_WIDTH),
                    height: mv_clay::sizing_fixed(ARROW_HEIGHT),
                };
                crate::ui::ui_scope(app, mv_clay::id(ids.arrow, id), &glyph, |_| {});
            });
        }
    }

    /// A key cap: the secondary plate with a 1 px border under the label.
    fn key_cap(
        app: &mut App,
        ids: &TipIds,
        id: u32,
        ink: &impl Fn(mv_theme::Token) -> clay::Color,
        child: impl FnOnce(&mut App),
    ) {
        let mut cap = crate::ui::empty_decl();
        cap.layout.sizing = clay::Sizing {
            width: mv_clay::sizing_fit(crate::ui::ZERO_MINMAX),
            height: mv_clay::sizing_fixed(KEY_HEIGHT),
        };
        cap.layout.padding = clay::Padding {
            // The 1 px side borders join the horizontal padding; the fixed
            // 21 px height already holds the vertical ones, as CSS
            // `box-sizing` does.
            left: KEY_PAD_X as u16 + 1,
            right: KEY_PAD_X as u16 + 1,
            top: KEY_PAD_TOP,
            bottom: KEY_PAD_BOTTOM,
        };
        cap.layout.childAlignment = clay::ChildAlignment {
            x: clay::LayoutAlignmentX::Center,
            y: clay::LayoutAlignmentY::Center,
        };
        cap.backgroundColor = ink(mv_theme::Token::Secondary);
        cap.border = clay::BorderElementConfig {
            color: ink(mv_theme::Token::Border),
            width: mv_clay::border_outside(1),
        };
        cap.cornerRadius = crate::ui::corner_radius_all(TIP_RADIUS);
        crate::ui::ui_scope(app, mv_clay::id(ids.key, id), &cap, child);
    }
}

/// An arrow glyph's chevron: three points.
pub type ArrowChevron = [(f32, f32); 3];
/// An arrow glyph's shaft: two points.
pub type ArrowShaft = [(f32, f32); 2];

/// An arrow glyph's two strokes (`M4 8L9 2L14 8M9 2V14` up,
/// `M4 8L9 14L14 8M9 14V2` down — the chevron then the shaft), as the two
/// segments the painter strokes.
#[must_use]
pub fn arrow_strokes(up: bool) -> (ArrowChevron, ArrowShaft) {
    // `M4 8L9 2L14 8` up / `M4 8L9 14L14 8` down, then `M9 2V14` / `M9 14V2`.
    let mid = if up { 2.0 } else { 14.0 };
    let end = if up { 14.0 } else { 2.0 };
    ([(4.0, 8.0), (9.0, mid), (14.0, 8.0)], [(9.0, mid), (9.0, end)])
}

/// Paints the arrow glyphs after the command walk (paths, not text, as the
/// chevrons are). Call from the rows' paint, as the scrub tip's are.
pub fn paint_arrows(app: &mut App, ids: &TipIds, base: u32, cells: &[TipCell]) {
    let theme = crate::ui::current_theme(app).clone();
    let density = density_of(app);
    let ink = crate::draw::nvg_color(crate::ui::ink(&theme, mv_theme::Token::Foreground));
    for (k, cell) in cells.iter().enumerate() {
        let dir = match cell {
            TipCell::Arrow(ArrowDir::Up) => Some(true),
            TipCell::Arrow(ArrowDir::Down) => Some(false),
            _ => None,
        };
        let Some(up) = dir else {
            continue;
        };
        let data = app
            .clay
            .as_ref()
            .expect("clay context initialised")
            .get_element_data(mv_clay::id(ids.arrow, base + k as u32));
        if !data.found {
            continue;
        }
        let at = crate::draw::snap_box(data.boundingBox, density);
        let (chevron, shaft) = arrow_strokes(up);
        app.vg.save();
        app.vg.translate(at.x, at.y);
        app.vg.begin_path();
        app.vg.move_to(chevron[0].0, chevron[0].1);
        app.vg.line_to(chevron[1].0, chevron[1].1);
        app.vg.line_to(chevron[2].0, chevron[2].1);
        app.vg.move_to(shaft[0].0, shaft[0].1);
        app.vg.line_to(shaft[1].0, shaft[1].1);
        app.vg.stroke_color(ink);
        app.vg.stroke_width(ARROW_STROKE);
        app.vg.line_cap(mv_nvg::LineCapType::ROUND);
        app.vg.line_join(mv_nvg::LineCapType::ROUND);
        app.vg.stroke();
        app.vg.restore();
    }
}

/// The density the frame draws at (the rows' `paint_rows` rule).
fn density_of(app: &App) -> f32 {
    if app.size[0] > 0.0 {
        app.pixels[0] as f32 / app.size[0]
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The tip plate's look: Tip.module.css `.tip` to the pixel.
    #[test]
    fn plate_constants_match_the_css() {
        assert_eq!(TIP_GAP, 4.0);
        assert_eq!((TIP_PAD_X, TIP_PAD_Y), (10.0, 7.0));
        assert_eq!((TIP_FONT_SIZE, TIP_LINE_HEIGHT), (13.0, 15.0));
        assert_eq!(TIP_BORDER, 1.0);
        assert_eq!(TIP_RADIUS, 4.0);
    }

    // A key cap's look: `.key` to the pixel (21 px tall, 1 px 6 px 2 px
    // padding, 600 12 px/16 px).
    #[test]
    fn key_constants_match_the_css() {
        assert_eq!(KEY_HEIGHT, 21.0);
        assert_eq!((KEY_FONT_SIZE, KEY_LINE_HEIGHT), (12.0, 16.0));
        assert_eq!(KEY_PAD_X, 6.0);
        assert_eq!((KEY_PAD_TOP, KEY_PAD_BOTTOM), (1, 2));
    }

    // An arrow glyph's look: `ArrowKey`'s 18 x 14 box, 1.6 px stroke.
    #[test]
    fn arrow_constants_match_the_css() {
        assert_eq!((ARROW_WIDTH, ARROW_HEIGHT), (18.0, 14.0));
        assert_eq!(ARROW_STROKE, 1.6);
    }

    // The arrow paths: `M4 8L9 2L14 8M9 2V14` up, `M4 8L9 14L14 8M9 14V2`
    // down.
    #[test]
    fn arrow_strokes_follow_the_svg_paths() {
        assert_eq!(
            arrow_strokes(true),
            ([(4.0, 8.0), (9.0, 2.0), (14.0, 8.0)], [(9.0, 2.0), (9.0, 14.0)])
        );
        assert_eq!(
            arrow_strokes(false),
            (
                [(4.0, 8.0), (9.0, 14.0), (14.0, 8.0)],
                [(9.0, 14.0), (9.0, 2.0)]
            )
        );
    }
}
