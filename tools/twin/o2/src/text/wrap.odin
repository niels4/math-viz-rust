package text

// A paragraph's lines (CSS Text 3 § 5, white-space: normal with text-wrap: wrap): broken at spaces
// only, each line taking words while they fit, the space after its last word hanging past the
// line's end, so a line fits when its words do; a word too wide for a line of its own keeps one. A
// no-break space joins its two words. Clay's own word wrap counts each word's trailing space
// against the width, which breaks a line that fits within a space of its box one word early (an
// explainer's sentence, M11: a's third line is 264.39 px in its 266).

// `text`'s lines in a box `width` wide, in `style`: slices of `text`.
break_lines :: proc(
	fonts: ^Fonts,
	style: Style,
	text: string,
	width: f32,
	allocator := context.temp_allocator,
) -> []string {
	lines := make([dynamic]string, allocator)
	if text == "" {
		return lines[:]
	}
	start := 0 // where the line begins
	end := 0 // where its last word ends
	words := 0 // the words on it
	for i := 0; i <= len(text); {
		j := i
		for j < len(text) && text[j] != ' ' {
			j += 1
		}
		if words > 0 && measure(fonts, style, text[start:j]) > width {
			append(&lines, text[start:end])
			start, words = i, 0
		}
		end = j
		words += 1
		i = j + 1
	}
	append(&lines, text[start:end])
	return lines[:]
}
