// Odin builds _test files into every build, and core:testing can't build for the web.
#+build !js
package text

import "core:encoding/json"
import "core:fmt"
import "core:log"
import "core:strings"
import "core:testing"

// The port's text against the reference's (plan § 4 M4): runs of the page's text in the faces,
// sizes, letter spacing and features its rules give them, measured in WebKit at 0f4bdab, each
// alone in a document with the page's @font-face rules (testdata/widths.json, written by
// `tools/run widths`, with Chromium's and Firefox's values). WebKit and Chromium, the reference
// engines (plan call 3), differ by up to 0.9 px on the page's longest line, all of it the pairs
// out of a space that Chromium kerns and WebKit doesn't (call 3 c, which the port follows); the
// port is held to 1 px of both engines' advances (call 3 h), to WebKit's line boxes exactly and
// to its baselines within its 1/64 px (call 3 d).

@(private = "file")
FIXTURE :: #load("testdata/widths.json", string)

TOLERANCE :: 1 // px of WebKit's advance and of Chromium's (plan call 3 h)

@(private = "file")
Measured :: struct {
	width:    f32,
	height:   f32,
	baseline: f32,
	content:  [2]f32,
	slot:     f32,
}

@(private = "file")
Sample :: struct {
	name:        string,
	family:      string,
	weight:      int,
	italic:      bool,
	size:        f32,
	tracking:    f32,
	tabular:     bool,
	kerning:     bool,
	line_height: Maybe(f32), // nil for `normal`
	text:        string,
	check:       bool, // a check of kerning, figures or ch, not one of the 30 strings
	ch:          bool,
	// WebKit's measures, then the other engines'.
	width:       f32,
	height:      f32,
	baseline:    f32,
	slot:        f32,
	engines:     map[string]Measured,
}

@(private = "file")
Fixture :: struct {
	engine:  string,
	samples: []Sample,
}

@(private = "file")
load_fixture :: proc(t: ^testing.T) -> (fixture: Fixture, ok: bool) {
	err := json.unmarshal_string(FIXTURE, &fixture, allocator = context.temp_allocator)
	testing.expectf(t, err == nil, "testdata/widths.json: %v", err)
	return fixture, err == nil
}

// The face CSS picks for a sample's family, weight and style.
@(private = "file")
face_of :: proc(s: Sample) -> (face: Face, ok: bool) {
	switch s.family {
	case "Work Sans":
		if s.italic {
			return
		}
		switch s.weight {
		case 400:
			return .Work_Sans_400, true
		case 500:
			return .Work_Sans_500, true
		case 600:
			return .Work_Sans_600, true
		case 700:
			return .Work_Sans_700, true
		case 800:
			return .Work_Sans_800, true
		}
	case "STIX Two Text":
		switch {
		case s.italic && s.weight == 400:
			return .STIX_400_Italic, true
		case s.weight == 400:
			return .STIX_400, true
		case s.weight == 500:
			return .STIX_500, true
		}
	case "Roboto Mono":
		switch s.weight {
		case 500:
			return .Roboto_Mono_500, true
		case 600:
			return .Roboto_Mono_600, true
		}
	}
	return
}

@(private = "file")
style_of :: proc(t: ^testing.T, s: Sample) -> (style: Style, ok: bool) {
	face, found := face_of(s)
	testing.expectf(
		t,
		found,
		"%s: no face for %s %d%s",
		s.name,
		s.family,
		s.weight,
		" italic" if s.italic else "",
	)
	style = {
		face     = face,
		size     = s.size,
		tracking = s.tracking,
	}
	if s.tabular {
		style.features += {.Tabular}
	}
	if !s.kerning {
		style.features += {.No_Kerning}
	}
	return style, found
}

// A length for the table: three decimals, padded with spaces (fmt pads numbers with zeros).
@(private = "file")
px :: proc(v: f32, signed := false) -> string {
	return fmt.tprintf("%+.3f", v) if signed else fmt.tprintf("%.3f", v)
}

@(private = "file")
find :: proc(fixture: Fixture, name: string) -> Sample {
	for s in fixture.samples {
		if s.name == name {
			return s
		}
	}
	panic(fmt.tprintf("no sample %q in testdata/widths.json", name))
}

// Every one of the 30 strings within 1 px of WebKit's advance and of Chromium's. The table beside
// it shows where the port falls among the three engines.
@(test)
widths_are_the_references :: proc(t: ^testing.T) {
	fixture, ok := load_fixture(t)
	if !ok {
		return
	}
	fonts: Fonts
	if !testing.expect(t, init(&fonts)) {
		return
	}
	defer destroy(&fonts)

	strings_count, within := 0, [3]int{}
	worst: [2]f32
	worst_name: [2]string
	table: strings.Builder
	strings.builder_init(&table, context.temp_allocator)
	fmt.sbprintf(
		&table,
		"%-16s %9s %9s %9s %9s %8s %8s\n",
		"sample",
		"port",
		"webkit",
		"chromium",
		"firefox",
		"-webkit",
		"-chrom.",
	)
	for s in fixture.samples {
		if s.check {
			continue
		}
		strings_count += 1
		style := style_of(t, s) or_continue
		port := measure(&fonts, style, s.text)
		chromium := s.engines["chromium"].width
		off := [2]f32{port - s.width, port - chromium}
		testing.expectf(
			t,
			abs(off[0]) <= TOLERANCE && abs(off[1]) <= TOLERANCE,
			"%s %q: the port's advance %.3f px, WebKit's %.3f (%+.3f), Chromium's %.3f (%+.3f); Firefox %.3f",
			s.name,
			s.text,
			port,
			s.width,
			off[0],
			chromium,
			off[1],
			s.engines["firefox"].width,
		)
		nearer := min(abs(off[0]), abs(off[1]))
		for limit, i in ([3]f32{0.1, 0.25, 0.5}) {
			if nearer <= limit {
				within[i] += 1
			}
		}
		for engine in 0 ..< 2 {
			if abs(off[engine]) > abs(worst[engine]) {
				worst[engine], worst_name[engine] = off[engine], s.name
			}
		}
		fmt.sbprintf(
			&table,
			"%-16s %9s %9s %9s %9s %8s %8s\n",
			s.name,
			px(port),
			px(s.width),
			px(chromium),
			px(s.engines["firefox"].width),
			px(off[0], signed = true),
			px(off[1], signed = true),
		)
	}
	testing.expect_value(t, strings_count, 30)
	log.infof(
		"%d strings: every one within %v px of WebKit's advance and of Chromium's; within 0.1 px of the nearer %d, 0.25 px %d, 0.5 px %d; the furthest from WebKit %s, %+.3f px, from Chromium %s, %+.3f px\n%s",
		strings_count,
		TOLERANCE,
		within[0],
		within[1],
		within[2],
		worst_name[0],
		worst[0],
		worst_name[1],
		worst[1],
		strings.to_string(table),
	)
}

// Every sample's line box as WebKit makes it: a px line height kept, `normal` as round(A) +
// round(D) + round(gap); the baseline where the CSS formula puts it, which WebKit floors to its
// 1/64 px layout unit.
@(test)
line_boxes_are_webkits :: proc(t: ^testing.T) {
	fixture, ok := load_fixture(t)
	if !ok {
		return
	}
	fonts: Fonts
	if !testing.expect(t, init(&fonts)) {
		return
	}
	defer destroy(&fonts)
	for s in fixture.samples {
		style := style_of(t, s) or_continue
		height, set := s.line_height.?
		if !set {
			height = line_height_normal(&fonts, style.face, style.size)
		}
		testing.expectf(
			t,
			height == s.height,
			"%s: a line box %v px tall, WebKit's %v",
			s.name,
			height,
			s.height,
		)
		below := baseline(&fonts, style.face, style.size, height)
		testing.expectf(
			t,
			below - s.baseline >= -1e-4 && below - s.baseline < 1.0 / 64 + 1e-4,
			"%s: the baseline %.4f px below the line box's top, WebKit's %.4f",
			s.name,
			below,
			s.baseline,
		)
	}
}

// Kerning present: a row's name narrower kerned than not by what WebKit takes off ("Vertical
// scale": V-e, r-t and l-space; its space starts no pair the font kerns, so WebKit and Chromium
// agree on it).
@(test)
kerning_is_the_references :: proc(t: ^testing.T) {
	fixture, ok := load_fixture(t)
	if !ok {
		return
	}
	fonts: Fonts
	if !testing.expect(t, init(&fonts)) {
		return
	}
	defer destroy(&fonts)
	kerned, plain := find(fixture, "name-a"), find(fixture, "kern-off")
	style, found := style_of(t, kerned)
	if !found {
		return
	}
	unkerned := style
	unkerned.features += {.No_Kerning}
	port := measure(&fonts, style, kerned.text) - measure(&fonts, unkerned, plain.text)
	reference := kerned.width - plain.width
	testing.expectf(
		t,
		port < -1 && abs(port - reference) <= 0.05,
		"%q: kerning takes %.3f px off in the port, %.3f in WebKit",
		kerned.text,
		port,
		reference,
	)
}

// Tabular figures: "1.00" and "1.11" equal in the page's tabular faces, as in the browsers, and
// Work Sans's figures proportional without tnum.
@(test)
tabular_figures_are_the_references :: proc(t: ^testing.T) {
	fixture, ok := load_fixture(t)
	if !ok {
		return
	}
	fonts: Fonts
	if !testing.expect(t, init(&fonts)) {
		return
	}
	defer destroy(&fonts)
	for prefix in ([]string{"tnum-mono", "tnum-stix", "tnum-ws", "pnum-ws"}) {
		zeros := find(fixture, fmt.tprintf("%s-1.00", prefix))
		ones := find(fixture, fmt.tprintf("%s-1.11", prefix))
		style := style_of(t, zeros) or_continue
		a, b := measure(&fonts, style, zeros.text), measure(&fonts, style, ones.text)
		testing.expectf(
			t,
			abs(a - zeros.width) <= 0.05 && abs(b - ones.width) <= 0.05,
			"%s: 1.00 %.3f and 1.11 %.3f px, WebKit %.3f and %.3f",
			prefix,
			a,
			b,
			zeros.width,
			ones.width,
		)
		if prefix == "pnum-ws" {
			testing.expectf(
				t,
				b < a - 5,
				"Work Sans's proportional 1.11 (%.3f) as wide as 1.00 (%.3f)",
				b,
				a,
			)
		} else {
			testing.expectf(t, a == b, "%s: 1.00 %.4f px, 1.11 %.4f", prefix, a, b)
		}
	}
}

// ch as WebKit and Chromium measure it, and a readout's 5ch slot rounded up as they round it.
@(test)
ch_is_the_references :: proc(t: ^testing.T) {
	fixture, ok := load_fixture(t)
	if !ok {
		return
	}
	fonts: Fonts
	if !testing.expect(t, init(&fonts)) {
		return
	}
	defer destroy(&fonts)
	for name in ([]string{"ch-mono", "ch-stix"}) {
		s := find(fixture, name)
		style := style_of(t, s) or_continue
		unit := ch(&fonts, style)
		testing.expectf(t, abs(unit - s.width) < 1e-3, "%s: ch %.4f px, WebKit %.4f", name, unit, s.width)
		testing.expectf(
			t,
			round_up(5 * unit) == s.slot,
			"%s: round(up, 5ch, 1px) %v px, WebKit %v",
			name,
			round_up(5 * unit),
			s.slot,
		)
	}
	testing.expect_value(t, find(fixture, "ch-mono").slot, 55)
	testing.expect_value(t, find(fixture, "ch-mono").engines["firefox"].slot, 54)
}
