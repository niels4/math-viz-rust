//! Truncation: how much text fits a width, and whether any was cut.

use super::fonts::Fonts;
use super::shape::{Style, measure};

/// What a cut run ends in.
pub const ELLIPSIS: &str = "…";

/// The longest head of text that fits a width, and whether text was cut. The
/// cut lands on a character boundary.
pub fn truncate(fonts: &mut Fonts, style: Style, text: &str, width: f32) -> (String, bool) {
    if measure(fonts, style, text) <= width {
        return (text.to_owned(), false);
    }
    let room = width - measure(fonts, style, ELLIPSIS);
    let mut end = text.len();
    while end > 0 {
        let step = text[..end].chars().next_back().map_or(1, |c| c.len_utf8());
        end -= step;
        if measure(fonts, style, &text[..end]) <= room {
            break;
        }
    }
    (text[..end].to_owned(), true)
}
