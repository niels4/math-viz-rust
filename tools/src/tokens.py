"""The theme fixture (`tools/run tokens`): every colour token of the six themes as the
reference resolves and draws it, written to crates/mv-theme/testdata/tokens.json, which
`cargo test -p mv-theme` checks the port's colour maths against.

The reference opens once per theme and engine, showing that theme (its class is on <body>). A
probe element under the body reads each colour's computed value; a swatch of the colour at full
alpha (relative colour syntax) is painted in the page and read back from a screenshot, which is
the pixel the engine draws, and the same colour filled into a 1x1 sRGB canvas must give the same
pixel. WebKit's values are the fixture (the harness's default engine); Chromium's and Firefox's
pixels are kept beside them.

The page's mixes are the port's own recurring mixes (theme.rs's ring, plate and badges): every
token they fade exists in the theme files, so the browser resolves them. The themes' other
values (radius, the card, the shadow, the glow, the HUD ticks, the label tracking, the readout
font) have no source in math-viz2's theme files: the fixture carries them from the previous
file, and the tests pin them there until the milestone that draws them re-sources them."""

import io
import json
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import cast

from PIL import Image
from playwright.sync_api import Page, sync_playwright

from act import REFERENCE_ROUTE, REPO, port
from browser import ENGINES, PageLog, engine_type
from themes import SLUGS, Theme, react_ref, read_themes

OUT = REPO / "crates" / "mv-theme" / "testdata" / "tokens.json"
FIXTURE_ENGINE = "webkit"

# The port's recurring mixes with transparent (theme.rs's ring, plate and badges), by the name
# of the function that makes each. Every token they fade exists in the theme files.
MIXES = {
    "ring": "color-mix(in oklch, var(--primary) 32%, transparent)",
    "plate": "color-mix(in oklch, var(--primary) 26%, transparent)",
    "badge-accent": "color-mix(in oklch, var(--accent) 24%, transparent)",
    "badge-warning": "color-mix(in oklch, var(--warning) 24%, transparent)",
}

SWATCH = 10  # CSS px, in a grid at the page's top left
COLUMNS = 40

COMPUTED = re.compile(
    r"oklch\(([-\d.e]+) ([-\d.e]+) ([-\d.e]+|none)(?: / ([\d.e]+))?\)"
    r"|rgba?\((\d+), (\d+), (\d+)(?:, ([\d.]+))?\)"
)

PROBE_JS = r"""
({ colours, size, columns }) => {
  const host = document.body;
  const probe = document.createElement('div');
  probe.style.cssText = 'position:absolute;visibility:hidden;pointer-events:none;';
  host.appendChild(probe);
  const grid = document.createElement('div');
  grid.style.cssText = `position:fixed;left:0;top:0;z-index:2147483647;display:grid;` +
    `grid-template-columns:repeat(${columns}, ${size}px);grid-auto-rows:${size}px;`;
  host.appendChild(grid);
  const canvas = document.createElement('canvas');
  canvas.width = canvas.height = 1;
  const ctx = canvas.getContext('2d', { colorSpace: 'srgb', willReadFrequently: true });
  // The computed colour at full alpha, for the canvas.
  const opaque = (c) => c.replace(/ \/ [\d.e-]+\)$/, ')').replace(/^rgba\((\d+), (\d+), (\d+), [\d.]+\)$/, 'rgb($1, $2, $3)');
  const out = [];
  for (const expr of colours) {
    probe.style.color = '';
    probe.style.color = expr;
    const computed = getComputedStyle(probe).color;
    const swatch = document.createElement('div');
    swatch.style.background = `oklch(from ${expr} l c h / 1)`;
    grid.appendChild(swatch);
    ctx.clearRect(0, 0, 1, 1);
    ctx.fillStyle = '#000';
    ctx.fillStyle = opaque(computed);
    ctx.fillRect(0, 0, 1, 1);
    const d = ctx.getImageData(0, 0, 1, 1).data;
    out.push({ computed, canvas: [d[0], d[1], d[2]] });
  }
  const body = getComputedStyle(document.body);
  probe.remove();
  return { out, background: body.getPropertyValue('--background').trim() };
}
"""


@dataclass
class Colour:
    """One colour as an engine computes and draws it."""

    computed: str
    oklch: list[float]  # L, C, H, alpha; empty when the engine computes it as rgb()
    alpha: float
    rgb: list[int]  # the swatch's pixel: the colour at full alpha, as drawn in the page
    canvas: list[int]  # the same colour filled into a 1x1 sRGB canvas
    engines: dict[str, list[int]] = field(default_factory=lambda: {})

    def entry(self) -> dict[str, object]:
        return {
            "computed": self.computed,
            "oklch": self.oklch,
            "alpha": self.alpha,
            "rgb": self.rgb,
            "engines": self.engines,
        }


def parse_computed(text: str) -> tuple[list[float], float]:
    m = COMPUTED.fullmatch(text)
    if m is None:
        raise SystemExit(f"a computed colour the fixture can't read: {text}")
    if m[1] is not None:
        hue = 0.0 if m[3] == "none" else float(m[3])
        alpha = float(m[4]) if m[4] else 1.0
        return [float(m[1]), float(m[2]), hue, alpha], alpha
    return [], float(m[8]) if m[8] else 1.0


def theme_prefs(slug: str) -> dict[str, str]:
    """The storage seeding that shows `slug`: both shapes, since the reference reads the single
    `mathviz-theme` key before h1 and the three keys after (each reference reads its own)."""
    scheme = "dark" if SLUGS.index(slug) < 3 else "light"
    return {
        "theme": slug,
        "mode": scheme,
        "dark": slug if scheme == "dark" else "arctic-ice",
        "light": slug if scheme == "light" else "clean-teal",
    }


@dataclass
class Taken:
    """One theme in one engine."""

    colours: dict[str, Colour]


def take(page: Page, theme: Theme, expressions: dict[str, str]) -> Taken:
    names = list(expressions)
    found = cast(
        dict[str, object],
        page.evaluate(
            PROBE_JS,
            {"colours": [expressions[n] for n in names], "size": SWATCH, "columns": COLUMNS},
        ),
    )
    background = theme.declarations["--background"]
    if str(found["background"]) != background:
        raise SystemExit(f"{theme.slug}: the page shows --background {found['background']}")
    shot = Image.open(io.BytesIO(page.screenshot())).convert("RGB")
    colours: dict[str, Colour] = {}
    for i, (name, raw) in enumerate(
        zip(names, cast(list[dict[str, object]], found["out"]), strict=True)
    ):
        x = (i % COLUMNS) * SWATCH + SWATCH // 2
        y = (i // COLUMNS) * SWATCH + SWATCH // 2
        pixel = cast(tuple[int, int, int], shot.getpixel((x, y)))
        oklch, alpha = parse_computed(str(raw["computed"]))
        colours[name] = Colour(
            str(raw["computed"]), oklch, alpha, list(pixel), cast(list[int], raw["canvas"])
        )
    return Taken(colours)


def run_tokens(engines: list[str], out: Path = OUT) -> int:
    themes = read_themes(react_ref())
    url = f"http://localhost:{port('REACT_REF_PORT')}/{REFERENCE_ROUTE}"
    expressions = {name: f"var({name})" for name in themes[0].colour_tokens()} | MIXES
    order = [FIXTURE_ENGINE, *(e for e in engines if e != FIXTURE_ENGINE)]
    taken: dict[str, dict[str, Taken]] = {}
    problems: list[str] = []
    versions: dict[str, str] = {}
    with sync_playwright() as playwright:
        for engine in order:
            browser = engine_type(playwright, engine).launch()
            versions[engine] = browser.version
            try:
                for theme in themes:
                    prefs = theme_prefs(theme.slug)
                    context = browser.new_context(
                        viewport={"width": 800, "height": 600},
                        device_scale_factor=1,
                        color_scheme="dark" if prefs["mode"] == "dark" else "light",
                    )
                    context.add_init_script(
                        "localStorage.setItem('mathviz-theme', "
                        f"{json.dumps(json.dumps(prefs['theme']))});"
                        "localStorage.setItem('mathviz-light-dark', "
                        f"{json.dumps(json.dumps(prefs['mode']))});"
                        "localStorage.setItem('mathviz-dark-theme', "
                        f"{json.dumps(json.dumps(prefs['dark']))});"
                        "localStorage.setItem('mathviz-light-theme', "
                        f"{json.dumps(json.dumps(prefs['light']))});"
                    )
                    page = context.new_page()
                    log = PageLog()
                    log.watch(page)
                    page.goto(url, wait_until="load")
                    # React has mounted once the theme class is on the body (no canvas yet).
                    page.wait_for_function("() => document.body.className.length > 0")
                    taken.setdefault(engine, {})[theme.slug] = take(page, theme, expressions)
                    problems += [f"{engine} {theme.slug}: {p}" for p in log.problems()]
                    context.close()
            finally:
                browser.close()
            print(
                f"{engine} {versions[engine]}: {len(themes)} themes, {len(expressions)} colours each"
            )

    try:
        carried = cast(
            dict[str, dict[str, object]],
            cast(dict[str, object], json.loads(OUT.read_text()))["themes"],
        )
    except (FileNotFoundError, KeyError, ValueError):
        sys.exit(f"{OUT} holds no themes to carry the values from")
    fixture_themes: dict[str, object] = {}
    spread: list[str] = []
    for theme in themes:
        base = taken[FIXTURE_ENGINE][theme.slug]
        for engine in order:
            for name, colour in taken[engine][theme.slug].colours.items():
                if colour.rgb != colour.canvas:
                    problems.append(
                        f"{engine} {theme.slug} {name}: the page draws {colour.rgb}, a canvas {colour.canvas}"
                    )
        for name, colour in base.colours.items():
            for engine in order[1:]:
                other = taken[engine][theme.slug].colours[name].rgb
                colour.engines[engine] = other
                if other != colour.rgb:
                    spread.append(
                        f"{theme.slug} {name}: {FIXTURE_ENGINE} {colour.rgb}, {engine} {other}"
                    )
        colours = {n: c.entry() for n, c in base.colours.items() if n not in MIXES}
        mixes = {n: base.colours[n].entry() for n in MIXES}
        # No source in math-viz2's theme files: carried from the previous file, pinned until
        # the milestone that draws them re-sources them.
        try:
            values = carried[theme.slug]["values"]
        except KeyError:
            sys.exit(f"{OUT} holds no values for {theme.slug}")
        fixture_themes[theme.slug] = {"colors": colours, "mixes": mixes, "values": values}

    fixture: dict[str, object] = {
        "about": (
            "Every colour token of math-viz2's six themes as the reference computes and draws "
            "it (tools/src/tokens.py; `tools/run tokens` writes this file). computed: "
            "getComputedStyle's colour; oklch: it parsed; rgb: the pixel the page draws for the "
            "colour at full alpha; engines: the other engines' pixels. The themes' other values "
            "are carried, unsourced in math-viz2's files."
        ),
        "engine": FIXTURE_ENGINE,
        "browsers": versions,
        "themes": fixture_themes,
    }
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(dump(fixture))
    print(f"wrote {out.relative_to(REPO)}")
    print(f"pixels where an engine differs from {FIXTURE_ENGINE}: {len(spread)}")
    for line in spread:
        print(f"  {line}")
    for problem in problems:
        print(f"  problem: {problem}")
    return 1 if problems else 0


def dump(fixture: dict[str, object]) -> str:
    """JSON with each colour on a line of its own, so a change reads as a one-line diff."""
    lines = ["{"]
    for i, (key, value) in enumerate(fixture.items()):
        comma = "," if i < len(fixture) - 1 else ""
        if key != "themes":
            lines.append(f"  {json.dumps(key)}: {json.dumps(value)}{comma}")
            continue
        lines.append('  "themes": {')
        themes = cast(dict[str, dict[str, dict[str, object]]], value)
        for t, (slug, sections) in enumerate(themes.items()):
            lines.append(f"    {json.dumps(slug)}: {{")
            for s, (section, entries) in enumerate(sections.items()):
                lines.append(f"      {json.dumps(section)}: {{")
                items = list(entries.items())
                for e, (name, entry) in enumerate(items):
                    end = "," if e < len(items) - 1 else ""
                    lines.append(f"        {json.dumps(name)}: {json.dumps(entry)}{end}")
                lines.append("      }" + ("," if s < len(sections) - 1 else ""))
            lines.append("    }" + ("," if t < len(themes) - 1 else ""))
        lines.append("  }" + comma)
    lines.append("}")
    return "\n".join(lines) + "\n"


__all__ = ["ENGINES", "run_tokens"]
