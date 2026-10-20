package text

// The room fit keeps free: rounding to whole px can add a pixel per run (hooks/useFitWidth.ts).
FIT_SLACK :: 2

// The scale that keeps text no wider than a container `container` px wide, at most 1, as React's
// useFitWidth sets --fit-scale for the live equation's font: the text's width at full size, and
// if that doesn't fit, the container over it; then, since whole-px parts (a fraction's padding,
// rounded margins) don't shrink with the font, a second look at that scale tells the part that
// scales from the fixed part, and the scale is solved for the room. `width_at` gives the text's
// width at a scale, `data` its input.
fit :: proc(container: f32, width_at: proc(scale: f32, data: rawptr) -> f32, data: rawptr) -> f32 {
	room := container - FIT_SLACK
	natural := width_at(1, data)
	if room <= 0 || natural <= room {
		return 1
	}
	first := room / natural
	width := width_at(first, data)
	if width <= room {
		return first
	}
	scaling := (natural - width) / (1 - first)
	solved := (room - (natural - scaling)) / scaling
	return solved if solved > 0 else first
}
