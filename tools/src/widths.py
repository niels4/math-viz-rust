"""The width fixture (`tools/run widths`, plan section 4 M4): text the page sets, measured as the
reference's browsers lay it out, written to crates/mv-text/testdata/widths.json, which
`cargo test -p mv-text` checks the port's shaping, measuring and line boxes against.

Each sample is a run of the page's text in the face, size, letter spacing and features a rule of
the page gives it (`where`), in a line box of the rule's line height. It is set alone in a fresh
document of the reference's origin whose only styles are the page's own @font-face rules
(src/style/global.css, the fonts' URLs as the dev server serves them): one document per sample,
because WebKit paints a run like the one beside it (M4a: two runs that differ only in
`font-kerning` paint alike when they share a document, though each measures right). A Range over
the text gives the run's advance, letter spacing included; an empty inline-block at the line's
end sits on its baseline; the line's own box is its line box. WebKit's values are the fixture (the
harness's default engine; plan call 3 e: within 1 px), Chromium's and Firefox's sit beside them.

After the samples, the reference page itself is opened once per engine and the same text read
where the page sets it (the top bar, the rows' names, the cards' titles), so a run measured alone
is shown to measure as it does on the page."""

import json
import re
from dataclasses import dataclass
from pathlib import Path
from typing import cast

from playwright.sync_api import Browser, Page, Route, sync_playwright

from act import REFERENCE_ROUTE, REPO, port, shown
from browser import ENGINES, PageLog, engine_type
from themes import react_ref

OUT = REPO / "crates" / "mv-text" / "testdata" / "widths.json"
FIXTURE_ENGINE = "webkit"
NBSP = " "
MINUS = "−"


@dataclass(frozen=True)
class Sample:
    """A run of the page's text and the rule that sets it."""

    name: str
    family: str  # the CSS family: "Work Sans", "STIX Two Text", "Roboto Mono"
    weight: int
    size: float  # px
    text: str  # as drawn (text-transform applied)
    where: str  # the rule on the page that sets it
    line_height: float | None = None  # px; None for `normal`
    tracking: float = 0  # letter-spacing, px
    italic: bool = False
    tabular: bool = False  # font-variant-numeric: tabular-nums
    kerning: bool = True  # font-kerning: none when False
    check: bool = False  # a check of kerning, figures or ch, not one of the 30 strings
    ch: bool = False  # measure the ch unit (a 64ch box over 64) and a round(up, 5ch, 1px) slot


WS, STIX, MONO = "Work Sans", "STIX Two Text", "Roboto Mono"
CAPS = "FUNCTIONS · TRANSFORMATIONS"  # TopBar's subtitle, "Functions · transformations" in caps

# The 30 strings: every face the page draws in, at the sizes its rules give (the M4a census of
# every visible text node in WebKit, six themes, the states with more text), long runs first.
SAMPLES = (
    Sample("title", WS, 700, 20, "Function Viewer", "TopBar .title", 23),
    Sample("subtitle-2.8", WS, 600, 12, CAPS, "TopBar .subtitle, Arctic Ice", 14, tracking=2.8),
    Sample("subtitle-1.12", WS, 600, 12, CAPS, "TopBar .subtitle, other themes", 14, tracking=1.12),
    Sample("wordmark", WS, 700, 24, "MathViz", "TopBar .wordmark (0.01em)", 28, tracking=0.24),
    Sample("name-a", WS, 600, 15, "Vertical scale", "ParamRow .name", 18),
    Sample("name-k", WS, 600, 15, "Vertical shift", "ParamRow .name", 18),
    Sample("name-b", WS, 600, 15, "Horizontal scale", "ParamRow .name", 18),
    Sample("name-h", WS, 600, 15, "Horizontal shift", "ParamRow .name", 18),
    Sample("card-function", WS, 700, 17, "Function", "SubCard .title", 20),
    Sample("card-transform", WS, 700, 17, "Transform", "SubCard .title", 20),
    Sample("theme-chip", WS, 600, 14, "Arctic Ice", "ThemeChip .chip", 16),
    Sample("caption-label", WS, 600, 14, "PLANE", "PlaneChrome .caption_label", tracking=2.8),
    Sample("role", WS, 500, 13, "point at the plane", "PointRows .role", 15),
    Sample(
        "tip-foot",
        WS,
        400,
        14,
        f"Drag a number sideways to reshape the curve{NBSP}·",
        "C3Sidebar's foot tip, Phrases .text",
        16,
    ),
    Sample(
        "tip-refused", WS, 400, 14, "A scale of 0 squashes the curve flat.", "a refusal tip", 16
    ),
    Sample(
        "tip-pan", WS, 400, 14, f"drag to pan{NBSP}· scroll to zoom", "Q's tip, Phrases .text", 16
    ),
    Sample(
        "sentence",
        WS,
        400,
        15,
        "Multiplies every height by a. a = 2 makes the curve twice as tall;",
        "TransformExplainer .sentence, its first line",
        22,
    ),
    Sample(
        "settings-label", WS, 800, 11.52, "APPEARANCE", "SettingsMenu .settings_label", 14, 0.9216
    ),
    Sample("badge", WS, 700, 12, "Fine ×0.1", "ParamRow .badge", 14),
    Sample("explainer-name", WS, 700, 16, "Vertical scale", "TransformExplainer .name", 19),
    Sample("readout", MONO, 500, 18, f"{MINUS}26.21", "PointReadout .number", 24, tabular=True),
    Sample("plane-label", MONO, 500, 17, f"({MINUS}1.50, 2.25)", "faces.ts readout label"),
    Sample("tick-approx", MONO, 500, 14, "≈0", "faces.ts TICK_LABEL_FACE, ≈ from Symbols"),
    Sample("scale-label", MONO, 500, 13, "1.25 u", "PlaneChrome .scale_label", 17),
    Sample("keycap", MONO, 600, 12, "Shift", "Kbd .key", 16),
    Sample("picker-sin", STIX, 400, 20, "sin", "the picker's sin x"),
    Sample("readout-paren", STIX, 400, 18, ") =", "PointReadout .math", tabular=True),
    Sample(
        "readout-sage",
        STIX,
        500,
        18,
        "26.21",
        "PointReadout .number, Sage Editorial",
        24,
        tabular=True,
    ),
    Sample("point-letter", STIX, 400, 20, "P", "PointMark .letter", italic=True),
    Sample("relation", STIX, 400, 20, "≈", "PointReadout .rel, ≈ from STIX Two Math"),
    # Checks: the kerning in a row's name; tabular figures in the page's tabular faces (and Work
    # Sans with tnum, which the page doesn't set but the feature must reach); ch in the readout
    # faces (PointReadout's slots, plan call 3 c).
    Sample(
        "kern-off",
        WS,
        600,
        15,
        "Vertical scale",
        "name-a, font-kerning: none",
        18,
        kerning=False,
        check=True,
    ),
    Sample(
        "tnum-mono-1.00",
        MONO,
        500,
        18,
        "1.00",
        "NumberBox in Roboto Mono",
        tabular=True,
        check=True,
    ),
    Sample(
        "tnum-mono-1.11",
        MONO,
        500,
        18,
        "1.11",
        "NumberBox in Roboto Mono",
        tabular=True,
        check=True,
    ),
    Sample("tnum-stix-1.00", STIX, 500, 18, "1.00", "NumberBox in STIX", tabular=True, check=True),
    Sample("tnum-stix-1.11", STIX, 500, 18, "1.11", "NumberBox in STIX", tabular=True, check=True),
    Sample("tnum-ws-1.00", WS, 600, 15, "1.00", "Work Sans with tnum", tabular=True, check=True),
    Sample("tnum-ws-1.11", WS, 600, 15, "1.11", "Work Sans with tnum", tabular=True, check=True),
    Sample("pnum-ws-1.00", WS, 600, 15, "1.00", "Work Sans, proportional", check=True),
    Sample("pnum-ws-1.11", WS, 600, 15, "1.11", "Work Sans, proportional", check=True),
    Sample(
        "ch-mono", MONO, 500, 18, "0", "1ch in Roboto Mono 18", tabular=True, check=True, ch=True
    ),
    Sample("ch-stix", STIX, 500, 18, "0", "1ch in STIX 500 18", tabular=True, check=True, ch=True),
)

FONT_FACE = re.compile(r"@font-face\s*\{[^}]*\}")

MEASURE_JS = """() => {
  const line = document.getElementById("line");
  const run = document.getElementById("run");
  const range = document.createRange();
  range.selectNodeContents(run);
  const r = range.getBoundingClientRect();
  const box = line.getBoundingClientRect();
  const base = document.getElementById("base").getBoundingClientRect().bottom;
  const ch = document.getElementById("ch");
  const slot = document.getElementById("slot");
  return {
    width: ch ? ch.getBoundingClientRect().width / 64 : r.width,
    slot: slot ? slot.getBoundingClientRect().width : null,
    height: box.height,
    baseline: base - box.top,
    content: [r.top - box.top, r.height],
    family: getComputedStyle(run).fontFamily,
  };
}"""

# The page's own runs, where it sets them in the default state, by the rule's selector.
ON_PAGE_JS = """() => {
  const read = (el) => {
    const range = document.createRange();
    range.selectNodeContents(el);
    return range.getBoundingClientRect().width;
  };
  const out = {};
  const first = (selector) => document.querySelector(selector);
  const title = first('header h1 span'), subtitle = first('header p span');
  if (title) out['title'] = read(title);
  if (subtitle) out['subtitle-2.8'] = read(subtitle);
  const wordmark = first('[class*="wordmark_text"]');
  if (wordmark) out['wordmark'] = read(wordmark);
  for (const p of ['a', 'k', 'b', 'h']) {
    const name = [...document.querySelectorAll(`[data-param="${p}"] span`)].find(
      (el) => /^_name_[a-z0-9]+_[0-9]+$/.test(el.className));
    if (name) out[`name-${p}`] = read(name);
  }
  for (const el of document.querySelectorAll('[class*="_title_"]')) {
    const text = el.textContent.trim();
    if (text === 'Function') out['card-function'] = read(el);
    if (text === 'Transform') out['card-transform'] = read(el);
  }
  return out;
}"""


def font_faces(react: Path) -> str:
    """The page's @font-face rules (src/style/global.css), each URL as the dev server serves it."""
    css = (react / "src" / "style" / "global.css").read_text()
    return "\n".join(
        rule.replace('url("./fonts/', 'url("/src/style/fonts/') for rule in FONT_FACE.findall(css)
    )


def document(faces: str, sample: Sample) -> str:
    style = (
        f"font-family: {json.dumps(sample.family)}; font-weight: {sample.weight}; "
        f"font-style: {'italic' if sample.italic else 'normal'}; font-size: {sample.size}px; "
        f"line-height: {f'{sample.line_height}px' if sample.line_height else 'normal'}; "
        f"letter-spacing: {f'{sample.tracking}px' if sample.tracking else 'normal'}; "
        f"font-kerning: {'auto' if sample.kerning else 'none'}; "
        f"font-variant-numeric: {'tabular-nums' if sample.tabular else 'normal'};"
    )
    text = sample.text.replace("&", "&amp;").replace("<", "&lt;")
    # 64ch, so the box's layout unit (1/64 px in WebKit and Chromium) leaves the unit itself to
    # 1/4096 px; and a readout's slot, PointReadout's round(up, n ch, 1px) at n = 5.
    ch = (
        '<span id="ch" style="display: inline-block; width: 64ch"></span>'
        '<span id="slot" style="display: inline-block; width: round(up, 5ch, 1px)"></span>'
        if sample.ch
        else ""
    )
    return f"""<!doctype html><meta charset="utf-8"><style>{faces}
body {{ margin: 0 }}
#line {{ position: absolute; left: 0; top: 0; white-space: pre; {style} }}
#base {{ display: inline-block; width: 0; height: 0 }}
</style><div id="line"><span id="run">{text}</span>{ch}<span id="base"></span></div>"""


def measure(page: Page, url: str, html: str, sample: Sample) -> dict[str, object]:
    def serve(route: Route) -> None:
        route.fulfill(status=200, content_type="text/html; charset=utf-8", body=html)

    page.route(url, serve)
    page.goto(url, wait_until="load")
    font = f"{'italic ' if sample.italic else ''}{sample.weight} {sample.size}px {json.dumps(sample.family)}"
    page.evaluate("([font, text]) => document.fonts.load(font, text)", [font, sample.text])
    page.evaluate("document.fonts.ready.then(() => true)")
    found = cast(dict[str, object], page.evaluate(MEASURE_JS))
    page.unroute(url, serve)
    return found


def round_values(found: dict[str, object]) -> dict[str, object]:
    content = cast(list[float], found["content"])
    return {
        "width": round(cast(float, found["width"]), 4),
        "height": round(cast(float, found["height"]), 4),
        "baseline": round(cast(float, found["baseline"]), 4),
        "content": [round(content[0], 4), round(content[1], 4)],
        **({"slot": found["slot"]} if found.get("slot") is not None else {}),
    }


def run_widths(engines: list[str], out: Path = OUT) -> int:
    faces = font_faces(react_ref())
    origin = f"http://localhost:{port('REACT_REF_PORT')}"
    order = [FIXTURE_ENGINE, *(e for e in engines if e != FIXTURE_ENGINE)]
    taken: dict[str, dict[str, dict[str, object]]] = {}
    on_page: dict[str, dict[str, float]] = {}
    versions: dict[str, str] = {}
    problems: list[str] = []
    with sync_playwright() as playwright:
        for engine in order:
            browser = engine_type(playwright, engine).launch()
            versions[engine] = browser.version
            try:
                context = browser.new_context(viewport={"width": 1440, "height": 900})
                for i, sample in enumerate(SAMPLES):
                    page = context.new_page()  # a fresh document for each sample
                    log = PageLog()
                    log.watch(page)
                    found = measure(
                        page, f"{origin}/__widths/{i}.html", document(faces, sample), sample
                    )
                    if str(found["family"]).split(",")[0].strip("\"' ") != sample.family:
                        problems.append(f"{engine} {sample.name}: set in {found['family']}")
                    taken.setdefault(engine, {})[sample.name] = round_values(found)
                    problems += [f"{engine} {sample.name}: {p}" for p in log.problems()]
                    page.close()
                context.close()
                on_page[engine] = read_page(browser, f"{origin}/{REFERENCE_ROUTE}")
            finally:
                browser.close()
            print(f"{engine} {versions[engine]}: {len(SAMPLES)} samples")

    samples: list[dict[str, object]] = []
    for sample in SAMPLES:
        entry: dict[str, object] = {
            "name": sample.name,
            "family": sample.family,
            "weight": sample.weight,
            "italic": sample.italic,
            "size": sample.size,
            "tracking": sample.tracking,
            "tabular": sample.tabular,
            "kerning": sample.kerning,
            "line_height": sample.line_height,
            "text": sample.text,
            "check": sample.check,
            "ch": sample.ch,
            "where": sample.where,
            **taken[FIXTURE_ENGINE][sample.name],
            "engines": {e: taken[e][sample.name] for e in order[1:]},
        }
        samples.append(entry)

    fixture: dict[str, object] = {
        "about": (
            "Runs of the page's text measured in the reference's browsers (tools/src/widths.py; "
            "`tools/run widths` writes this file). width: the run's advance in px (a Range over "
            "it, letter spacing included; for ch, the unit, from a 64ch box, and slot, a round(up, 5ch, 1px) box); height: its line box; baseline: "
            "the baseline below the line box's top; content: the content area's top and height. "
            "engines: the other engines' values."
        ),
        "engine": FIXTURE_ENGINE,
        "browsers": versions,
        "samples": samples,
    }
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(dump(fixture))
    print(f"wrote {shown(out.resolve())}")

    print(f"{'sample':16s} " + " ".join(f"{e:>9s}" for e in order) + "  (width, px)")
    for sample in SAMPLES:
        widths = [cast(float, taken[e][sample.name]["width"]) for e in order]
        spread = max(widths) - min(widths)
        print(
            f"{sample.name:16s} " + " ".join(f"{w:9.3f}" for w in widths) + f"  spread {spread:.3f}"
        )
    print("on the page against alone (width, px):")
    for engine in order:
        for name, width in sorted(on_page[engine].items()):
            alone = cast(float, taken[engine][name]["width"])
            print(
                f"  {engine:8s} {name:16s} page {width:9.3f}  alone {alone:9.3f}  {width - alone:+.3f}"
            )
            if abs(width - alone) > 0.01:
                problems.append(f"{engine} {name}: {width:.3f} on the page, {alone:.3f} alone")
    for problem in problems:
        print(f"  problem: {problem}")
    return 1 if problems else 0


def read_page(browser: Browser, url: str) -> dict[str, float]:
    """The page's own runs in the default state (Arctic Ice, so the subtitle's 2.8 px)."""
    context = browser.new_context(
        viewport={"width": 1440, "height": 900}, device_scale_factor=2, color_scheme="dark"
    )
    prefs = {"mode": "dark", "dark": "arctic-ice", "light": "clean-teal"}
    context.add_init_script(
        "localStorage.setItem('mathviz-fv-tour', 'true');"
        f"localStorage.setItem('mathviz-appearance', {json.dumps(json.dumps(prefs))});"
    )
    page = context.new_page()
    page.goto(url, wait_until="load")
    page.evaluate("document.fonts.ready.then(() => true)")
    page.wait_for_selector("canvas")
    found = cast(dict[str, float], page.evaluate(ON_PAGE_JS))
    context.close()
    return found


def dump(fixture: dict[str, object]) -> str:
    """JSON with each sample on a line of its own, so a change reads as a one-line diff."""
    lines = ["{"]
    keys = list(fixture)
    for i, key in enumerate(keys):
        comma = "," if i < len(keys) - 1 else ""
        if key != "samples":
            lines.append(
                f"  {json.dumps(key)}: {json.dumps(fixture[key], ensure_ascii=False)}{comma}"
            )
            continue
        lines.append('  "samples": [')
        samples = cast(list[dict[str, object]], fixture[key])
        for s, sample in enumerate(samples):
            end = "," if s < len(samples) - 1 else ""
            lines.append(f"    {json.dumps(sample, ensure_ascii=False)}{end}")
        lines.append("  ]" + comma)
    lines.append("}")
    return "\n".join(lines) + "\n"


__all__ = ["ENGINES", "SAMPLES", "run_widths"]
