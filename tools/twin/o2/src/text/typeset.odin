package text

// Runs of text in mixed faces set side by side on one line, each in a box of its own, as the
// page's flex rows of inline runs set them (the readouts, the equation's runs, the phrases with
// their key caps and maths): the box as wide as its text, or rounded up to whole px where the
// page rounds it in every engine (React's useWholePxRuns, `round(up, …, 1px)` rules), at least a
// slot's width, the text at its left or right; the line's pieces on one baseline, or each line box
// centred on the line.

// How a piece's box takes its text's width.
Box :: enum u8 {
	Exact, // the text's advance (CSS's max-content)
	Whole_Px, // the advance rounded up to whole px: round(up, …, 1px), useWholePxRuns
}

// Where a piece's text sits in a box wider than it.
Align :: enum u8 {
	Left,
	Right, // text-align: right (a number in its slot)
}

// How a line places its pieces' line boxes.
Vertical :: enum u8 {
	Baseline, // every piece on one baseline (inline flow, align-items: baseline)
	Center, // each piece's line box centred on the line (align-items: center)
}

// A run of text in one style, and its box.
Piece :: struct {
	style:       Style,
	text:        string,
	ink:         Color,
	line_height: f32, // px; 0 for `normal`
	box:         Box,
	min_width:   f32, // the box at least this wide, padding aside (a readout's slot)
	align:       Align,
	pad:         [2]f32, // px of padding left and right of the box's content
}

// A piece as the line placed it. Lengths are px from the line's left and top.
Placed :: struct {
	shaped:   Shaped,
	size:     f32, // the font size it is set at (the piece's, times the line's scale)
	ink:      Color,
	x:        f32, // its box's left
	width:    f32, // its box's width, padding included
	pen:      f32, // where its text starts
	top:      f32, // its line box's top
	height:   f32, // its line box's height
	baseline: f32,
}

// A line of pieces: their places, and the line's size. `baseline` is the line's own when it sets
// its pieces on one, the first piece's when it centres them.
Line :: struct {
	placed:   []Placed,
	width:    f32,
	height:   f32,
	baseline: f32,
}

// Sets `pieces` side by side, `gap` px apart, at `scale` times their font sizes (useFitWidth's
// --fit-scale: the fonts shrink, the line heights, padding, gaps and slots don't).
typeset :: proc(
	fonts: ^Fonts,
	pieces: []Piece,
	gap: f32 = 0,
	vertical := Vertical.Baseline,
	scale: f32 = 1,
	allocator := context.temp_allocator,
) -> (
	line: Line,
) {
	placed := make([]Placed, len(pieces), allocator)
	x: f32
	for piece, i in pieces {
		style := piece.style
		style.size *= scale
		shaped := shape(fonts, style, piece.text, allocator)
		content := round_up(shaped.advance) if piece.box == .Whole_Px else shaped.advance
		content = max(content, piece.min_width)
		pen := x + piece.pad[0]
		if piece.align == .Right {
			pen += content - shaped.advance
		}
		height := piece.line_height
		if height == 0 {
			height = line_height_normal(fonts, style.face, style.size)
		}
		placed[i] = {
			shaped   = shaped,
			size     = style.size,
			ink      = piece.ink,
			x        = x,
			width    = piece.pad[0] + content + piece.pad[1],
			pen      = pen,
			height   = height,
			baseline = baseline(fonts, style.face, style.size, height), // within its box, for now
		}
		x += placed[i].width
		if i < len(pieces) - 1 {
			x += gap
		}
	}
	line.placed = placed
	line.width = x

	switch vertical {
	case .Baseline:
		for p in placed {
			line.baseline = max(line.baseline, p.baseline)
		}
		for &p in placed {
			p.top = line.baseline - p.baseline
			p.baseline = line.baseline
			line.height = max(line.height, p.top + p.height)
		}
	case .Center:
		for p in placed {
			line.height = max(line.height, p.height)
		}
		for &p in placed {
			p.top = (line.height - p.height) / 2
			p.baseline += p.top
		}
		if len(placed) > 0 {
			line.baseline = placed[0].baseline
		}
	}
	return
}
