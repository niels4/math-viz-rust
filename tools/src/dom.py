"""The reference's own layout where a state's script puts it (`tools/run dom`): at each `shot` the
script takes, every element the page shows, from the page's root down, with its box, the boxes of
its own text runs (a Range over each text node) and their baselines, and the computed styles the
port draws from, beside the screenshot.

Elements the page hides are left out with their subtrees and listed: `display: none`, and the
visually hidden lines kept for screen readers (1 x 1 boxes clipped by `clip-path: inset(50%)`),
which the port has no counterpart for.

`tools/run dom --fixture` writes crates/function-viewer/testdata/page.json: the named boxes of
tools/states/boxes.toml at every shot of the states the port's tests lay out (FIXTURE_STATES),
by shot name, in WebKit and Chromium, which `cargo test -p function-viewer` holds the port's
layout to."""

import json
import subprocess
from collections.abc import Callable
from pathlib import Path
from typing import Any, cast

from playwright.sync_api import Page

from act import REPO, run, shown
from script import load
from states import STATES
from themes import react_ref

# What the probe reads of each element's computed style.
STYLE_PROPS = (
    "display",
    "position",
    "fontFamily",
    "fontSize",
    "fontWeight",
    "fontStyle",
    "lineHeight",
    "letterSpacing",
    "textTransform",
    "whiteSpace",
    "textOverflow",
    "color",
    "backgroundColor",
    "borderTopWidth",
    "borderTopColor",
    "borderRadius",
    "boxShadow",
    "paddingTop",
    "paddingRight",
    "paddingBottom",
    "paddingLeft",
    "gap",
    "fontVariantNumeric",
    "cursor",
    "opacity",
    "textAlign",
    "overflowX",
    "overflowY",
    "fill",
    "stroke",
    "strokeWidth",
)

# A text node's run: its Range box and the baseline it sits on. The baseline is read from an empty
# inline-block set on it: the node is wrapped in a span with the probe before its text, so the two
# share a line even where the text is a flex container's own (the theme chip's label), whose
# anonymous item a probe appended to the container would not share. The DOM is put back at once.
RUN_JS = r"""
const run = (node) => {
  const range = document.createRange();
  range.selectNodeContents(node);
  const r = range.getBoundingClientRect();
  const wrap = document.createElement('span');
  const probe = document.createElement('span');
  probe.style.cssText = 'display:inline-block;width:0;height:0;vertical-align:baseline;';
  node.replaceWith(wrap);
  wrap.append(probe, node);
  const baseline = probe.getBoundingClientRect().bottom;
  wrap.replaceWith(node);
  return {text: node.textContent, x: r.x, y: r.y, width: r.width, height: r.height, baseline};
};
"""

DOM_JS = "(props) => {" + RUN_JS + r"""
  const root = (document.querySelector('header') ?? document.querySelector('#root')).parentElement;
  const out = [];
  const skipped = [];
  const visuallyHidden = (cs, r) => cs.clipPath === 'inset(50%)' && r.width <= 1 && r.height <= 1;
  const walk = (el, depth, parent) => {
    const cs = getComputedStyle(el);
    const r = el.getBoundingClientRect();
    if (cs.display === 'none' || visuallyHidden(cs, r)) {
      skipped.push({tag: el.tagName.toLowerCase(), class: el.getAttribute('class') ?? '',
        testid: el.getAttribute('data-testid') ?? '', parent,
        reason: cs.display === 'none' ? 'display none' : 'visually hidden', text: el.textContent});
      return;
    }
    const style = {};
    for (const p of props) style[p] = cs[p];
    const texts = [];
    for (const n of [...el.childNodes]) {
      if (n.nodeType === 3 && n.textContent.length > 0) texts.push(run(n));
    }
    let scroll = null;
    if (el.scrollHeight > el.clientHeight || el.scrollWidth > el.clientWidth) {
      scroll = {top: el.scrollTop, left: el.scrollLeft, height: el.scrollHeight,
        width: el.scrollWidth, clientHeight: el.clientHeight, clientWidth: el.clientWidth};
    }
    const attrs = {};
    for (const a of el.attributes) if (a.name !== 'style') attrs[a.name] = a.value;
    const me = out.length;
    out.push({id: me, parent, depth, tag: el.tagName.toLowerCase(), attrs,
      rect: [r.x, r.y, r.width, r.height], texts, scroll, style});
    for (const c of el.children) walk(c, depth + 1, me);
  };
  walk(root, 0, -1);
  const doc = document.scrollingElement;
  return {
    viewport: [innerWidth, innerHeight],
    scroll: {top: doc.scrollTop, height: doc.scrollHeight, clientHeight: doc.clientHeight},
    elements: out,
    skipped,
  };
}
"""

# The page's boxes by name, from tools/states/boxes.toml (name = CSS selector): each
# selector's first shown element's box, in CSS px, or the name in `missing` when no shown
# element matches. A layout the reference hides (a media query's display: none) is left out.
FIXTURE_JS = """(pairs) => {
  const boxes = {}, missing = [];
  for (const [name, selector] of pairs) {
    const el = document.querySelector(selector);
    if (el === null || getComputedStyle(el).display === 'none' ||
        el.getClientRects().length === 0) {
      missing.push(name);
      continue;
    }
    const r = el.getBoundingClientRect();
    boxes[name] = [r.x, r.y, r.width, r.height];
  }
  return {viewport: [innerWidth, innerHeight], boxes, missing};
}
"""

Dom = dict[str, Any]

# The layouts the port's tests lay out, each a state in tools/states and the shot that shows
# it. H0's page has one state; each milestone appends the layouts it lays out.
# h1-settings: the menu's boxes only exist with the menu open, so the fixture runs the
# open menu in both schemes (dark options at settings-open, light options at
# settings-light-dark-system). h4-readout: the readout's boxes at values (the
# only state with terms, the fraction, the sup and the scale in both lines).
# h4-rows' rows states join the fixture: the rows' regions resolve here, so the
# layout tests hold the port's boxes to the reference's (h4-readout's readout
# states are already here).
FIXTURE_STATES = (
    "default",
    "settings-open",
    "settings-light-dark-system",
    "readout-values",
    "rows-default",
    "rows-hover-a",
    "rows-focus-k",
    "rows-typed",
    "rows-enter",
    "rows-zero-scale",
    "rows-arrows",
    "box-tip-rest",
    "box-tip-typing",
    # h5-ruler: the ruler and tape only exist mid-drag, so the fixture runs
    # ruler-a-30 (a's box held 30 px in).
    "ruler-a-30",
    "reset-visible",
    # h6 at 5cef052: the return-to-centre button only renders off the default
    # camera (the tooltip commit), so the fixture runs overlay-visible for
    # its box; the button's own visibility and tip are h6-handles', so the
    # fixture also runs overlay-tip (off default, pointer on the button).
    "overlay-visible",
    "overlay-tip",
    # h7-sidebar: the h and k rulers only exist mid-drag, so the fixture runs
    # rows-lit-hk-drag (the shift handle held).
    "rows-lit-hk-drag",
    # h8-points-slider: the track and thumb at rest (slider-default), the tip
    # on hover (slider-hover; display:none at rest).
    "slider-default",
    "slider-hover",
    # h8-points-field: the P edit tip and the open field box only exist
    # while the field edits, so the fixture runs field-open; field-invalid
    # pins the error tone's wider tip.
    "field-open",
    "field-invalid",
)
FIXTURE_ENGINES = ("webkit", "chromium")
FIXTURE = REPO / "crates" / "function-viewer" / "testdata" / "page.json"


def read_dom(page: Page) -> Dom:
    """The page's layout as it is now (DOM_JS)."""
    return cast(Dom, page.evaluate(DOM_JS, list(STYLE_PROPS)))


def _num(v: float) -> str:
    return f"{v:.2f}".rstrip("0").rstrip(".")


def _short_class(name: str) -> str:
    """A CSS module's class as its source names it: `_title_1x2y3_45` reads `title`."""
    parts = name.split("_")
    return parts[1] if name.startswith("_") and len(parts) >= 3 else name


def tree(dom: Dom, full: bool = False) -> str:
    """The layout as an indented tree, one element a line: its tag and classes, its test id, its
    box, its own texts with their Range boxes and baselines, and for text its font."""
    lines: list[str] = []
    lines.append(
        f"viewport {dom['viewport'][0]}x{dom['viewport'][1]}, the page scrolled "
        f"{_num(dom['scroll']['top'])} of {_num(dom['scroll']['height'])}"
    )
    for e in cast(list[Dom], dom["elements"]):
        attrs = cast(dict[str, str], e["attrs"])
        classes = ".".join(_short_class(c) for c in attrs.get("class", "").split())
        testid = attrs.get("data-testid", "")
        r = cast(list[float], e["rect"])
        line = "  " * e["depth"] + e["tag"] + (f".{classes}" if classes else "")
        line += f" #{testid}" if testid else ""
        line += f" [{_num(r[0])},{_num(r[1])} {_num(r[2])}x{_num(r[3])}]"
        for t in cast(list[Dom], e["texts"]):
            line += (
                f" {t['text']!r}@[{_num(t['x'])},{_num(t['y'])} {_num(t['width'])}x"
                f"{_num(t['height'])}] base {_num(t['baseline'])}"
            )
        if e["scroll"] is not None:
            s = e["scroll"]
            line += f" scroll {_num(s['top'])} of {_num(s['height'])} in {_num(s['clientHeight'])}"
        style = cast(dict[str, str], e["style"])
        if full or e["texts"]:
            family = style["fontFamily"].split(",")[0].strip('"')
            line += (
                f" | {family} {style['fontWeight']} {style['fontSize']}/{style['lineHeight']}"
                f"{' ' + style['fontStyle'] if style['fontStyle'] != 'normal' else ''}"
                f" ls {style['letterSpacing']} col {style['color']}"
            )
        lines.append(line)
    for s in cast(list[Dom], dom["skipped"]):
        classes = ".".join(_short_class(c) for c in s["class"].split())
        lines.append(f"left out ({s['reason']}): {s['tag']}.{classes} {s['text']!r}")
    return "\n".join(lines) + "\n"


def write_dom(page: Page, out: Path, name: str) -> None:
    dom = read_dom(page)
    (out / f"{name}.json").write_text(json.dumps(dom, indent=1))
    (out / f"{name}.txt").write_text(tree(dom))


def _writer(out: Path) -> Callable[[Page, str], None]:
    def write(page: Page, name: str) -> None:
        write_dom(page, out, name)

    return write


def run_dom(script_path: Path, engines: list[str], out: Path) -> int:
    """Runs a state's script on the reference in each engine, writing at each shot the screenshot,
    the layout as JSON and as a tree: <out>/<engine>/<shot>.png, .json, .txt."""
    script = load(script_path)
    failed = False
    for engine in engines:
        where = out / engine
        result = run(script, "ref", engine, where, on_shot=_writer(where))
        print(result.summary())
        for problem in result.problems:
            print(f"  {problem}")
        for name in result.shots:
            print(f"  {shown(where / name)}.json, .txt, .png")
        failed |= not result.ok
    return 1 if failed else 0


def _reference_commit() -> str:
    done = subprocess.run(
        ["git", "-C", str(react_ref()), "rev-parse", "--short", "HEAD"],
        capture_output=True,
        text=True,
        check=False,
    )
    return done.stdout.strip() or "unknown"


def run_fixture(out: Path, scratch: Path) -> int:
    """crates/function-viewer/testdata/page.json: the named boxes (tools/states/boxes.toml) at
    every shot of the fixture's states, by shot name, in WebKit and Chromium, with the viewport
    each was taken at and the names no shown element matched."""
    import tomllib

    boxes_toml = REPO / "tools" / "states" / "boxes.toml"
    pairs = [
        (name, selector)
        for name, selector in tomllib.loads(boxes_toml.read_text()).items()
        if isinstance(selector, str)
    ]
    if not pairs:
        print(f"{boxes_toml} lists no boxes")
        return 1
    shots: dict[str, Dom] = {}
    failed = False
    for name in FIXTURE_STATES:
        script = load(STATES / f"{name}.act")
        for engine in FIXTURE_ENGINES:
            measured: dict[str, Dom] = {}

            def measure(page: Page, shot: str, measured: dict[str, Dom] = measured) -> None:
                measured[shot] = cast(Dom, page.evaluate(FIXTURE_JS, pairs))

            result = run(script, "ref", engine, scratch / name / engine, on_shot=measure)
            if not result.ok or not measured:
                print(f"{name} {engine}: {result.summary()}")
                failed = True
                continue
            for shot, m in measured.items():
                entry = shots.setdefault(shot, {"size": m.pop("viewport")})
                m.pop("viewport", None)
                entry[engine] = m
                print(f"{shot} {engine}: {len(m['boxes'])} boxes at {entry['size']}")
    if failed:
        return 1
    fixture = {"reference": f"math-viz2 {_reference_commit()}", "shots": shots}
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(fixture, indent=1) + "\n")
    print(f"wrote {shown(out)}")
    return 0
