//! Word wrap: splitting text on spaces to fit a width.

use super::fonts::Fonts;
use super::shape::{Style, measure};

/// Splits text on spaces to fit a width. An overwide word stands alone; the
/// empty text breaks to nothing.
pub fn break_lines(fonts: &mut Fonts, style: Style, text: &str, width: f32) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    if text.is_empty() {
        return lines;
    }
    let mut start = 0; // where the line begins
    let mut end = 0; // where its last word ends
    let mut words = 0; // the words on it
    let mut i = 0;
    while i <= text.len() {
        let mut j = i;
        while j < text.len() && text.as_bytes()[j] != b' ' {
            j += 1;
        }
        if words > 0 && measure(fonts, style, &text[start..j]) > width {
            lines.push(text[start..end].to_owned());
            start = i;
            words = 0;
        }
        end = j;
        words += 1;
        i = j + 1;
    }
    lines.push(text[start..end].to_owned());
    lines
}
