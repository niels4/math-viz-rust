package text

import "core:unicode/utf8"

// text-overflow: ellipsis (CSS Overflow 3 § 3.3): a line wider than its box keeps the longest start
// of its text, whole characters, that leaves room for the ellipsis, and the ellipsis follows it in
// the line's own style, letter spacing included. The page truncates one text this way: the top
// bar's title block, whose lines give way first when the bar is too narrow (TopBar.module.css).

ELLIPSIS :: "…"

// `text` cut to fit `width` with the ellipsis after it: the start that is kept, and whether it was
// cut at all (when it was not, `start` is all of it). The ellipsis goes at the start's advance.
truncate :: proc(fonts: ^Fonts, style: Style, text: string, width: f32) -> (start: string, cut: bool) {
	if measure(fonts, style, text) <= width {
		return text, false
	}
	room := width - measure(fonts, style, ELLIPSIS)
	end := len(text)
	for end > 0 {
		_, size := utf8.decode_last_rune(text[:end])
		end -= size
		if measure(fonts, style, text[:end]) <= room {
			break
		}
	}
	return text[:end], true
}
