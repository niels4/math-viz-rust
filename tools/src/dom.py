"""The reference's own layout where a state's script puts it (`tools/run dom`): at each `shot` the
script takes, every element the page shows, from the page's root down, with its box, the boxes of
its own text runs (a Range over each text node) and their baselines, and the computed styles the
port draws from, beside the screenshot.

Elements the page hides are left out with their subtrees and listed: `display: none` (the layouts'
media queries drop the theme chip, the wordmark's text and the rows' names) and the visually hidden
lines kept for screen readers (C3Sidebar's spoken hint, the equation's spoken form, the theme
chip's "Theme: ", the zoom's status: 1 x 1 boxes clipped by `clip-path: inset(50%)`), which the
port has no counterpart for (plan 1.12).

`tools/run dom --fixture` writes crates/function-viewer/testdata/page.json: named measures of
the page (the top bar's parts, the panel, the plane, the cards, the rows, the points, the
equation's runs and the readouts' parts) at every shot of the states the port's tests lay out
(FIXTURE_STATES), by shot name, in WebKit and Chromium, which `cargo test -p function-viewer`
holds the port's layout to (the layout and sidebar tests)."""

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
  const root = document.querySelector('header').parentElement;
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

# The page's parts by name, as the port's tests call them: boxes, texts (a text node's run), runs
# (every visible text node under an element, in document order) and scroll positions. A part the
# layout hides (display: none) is left out.
FIXTURE_JS = "() => {" + RUN_JS + r"""
  const shown = (el) => el != null && getComputedStyle(el).display !== 'none' &&
    el.getClientRects().length > 0;
  const box = (el) => { const r = el.getBoundingClientRect(); return [r.x, r.y, r.width, r.height]; };
  const own = (el) => [...el.childNodes].filter((n) => n.nodeType === 3 && n.textContent.trim() !== '');
  const runs = (el) => {
    const nodes = [];
    const walker = document.createTreeWalker(el, NodeFilter.SHOW_TEXT);
    for (let n = walker.nextNode(); n !== null; n = walker.nextNode()) {
      if (n.textContent.trim() !== '' && shown(n.parentElement)) nodes.push(n);
    }
    return nodes.map(run);
  };
  const boxes = {}, texts = {}, lines = {}, scroll = {};
  const part = (name, el) => { if (shown(el)) boxes[name] = box(el); return el; };
  const text = (name, el) => { if (shown(el) && own(el).length) texts[name] = run(own(el)[0]); };
  const tid = (id) => document.querySelector(`[data-testid="${id}"]`);

  const header = part('topbar', document.querySelector('header'));
  const [wordmark, separator, titleBlock, actions] = header.children;
  part('wordmark', wordmark);
  part('wordmark_icon', wordmark.children[0]);
  text('wordmark_text', wordmark.children[1]);
  part('separator', separator);
  part('title_block', titleBlock);
  text('title_text', part('title', titleBlock.querySelector('h1')).firstElementChild);
  text('subtitle_text', part('subtitle', titleBlock.querySelector('p')).firstElementChild);
  part('actions', actions);
  const chip = part('chip', tid('theme-chip'));
  if (shown(chip)) {
    part('chip_dot', chip.children[0]);
    text('chip_text', chip);
  }
  const gear = part('gear', tid('settings-button'));
  part('gear_icon', gear.querySelector('svg'));

  const panel = part('panel', document.querySelector('aside'));
  scroll.panel = {top: panel.scrollTop, height: panel.scrollHeight, client: panel.clientHeight};
  part('plane', document.querySelector('main'));
  const doc = document.scrollingElement;
  scroll.page = {top: doc.scrollTop, height: doc.scrollHeight, client: doc.clientHeight};

  for (const s of ['function', 'transform', 'points']) {
    const card = part(`card_${s}`, tid(`fv-section-${s}`));
    text(`card_title_${s}`, card.querySelector('h2'));
  }
  part('picker', tid('fv-fn-x').parentElement);
  ['x', 'x2', 'x3', 'sin'].forEach((f, i) => part(`option_${i}`, tid(`fv-fn-${f}`)));
  part('equation', tid('fv-equation'));
  const live = part('live_line', tid('func-readout').firstElementChild);
  lines.live = runs(live);
  const form = part('form_line', [...tid('fv-form').children].find((c) => shown(c) &&
    !c.className.includes('_hidden_')));
  lines.form = runs(form);
  part('bar', form.querySelector('[class*="_bar_"]'));

  for (const p of ['a', 'k', 'b', 'h']) {
    const row = part(`row_${p}`, tid(`fv-param-${p}`));
    const chipEl = part(`chip_${p}`, tid(`fv-param-${p}-chip`));
    text(`chip_letter_${p}`, chipEl.querySelector('var'));
    const slot = part(`name_slot_${p}`, row.children[1]);
    text(`name_${p}`, slot.firstElementChild);
    part(`flip_${p}`, tid(`fv-param-${p}-flip`));
    part(`box_${p}`, tid(`fv-param-${p}-box`));
  }

  for (const p of ['p', 'q']) {
    const point = part(`point_${p}`, tid(`fv-point-${p}`));
    const head = part(`head_${p}`, point.firstElementChild);
    part(`mark_${p}`, head.children[0]);
    text(`letter_${p}`, part(`letter_${p}`, head.children[1]));
    const slot = part(`role_slot_${p}`, head.children[2]);
    text(`role_${p}`, slot.firstElementChild);
    const readout = part(`readout_${p}`, tid(`fv-${p}-readout`));
    lines[`readout_${p}`] = runs(readout);
    const [lead, close, rel, y] = readout.children;
    part(`readout_${p}_lead`, lead);
    part(`readout_${p}_open`, lead.children[0]);
    part(`readout_${p}_x`, lead.children[1]);
    part(`readout_${p}_close`, close);
    part(`readout_${p}_rel`, rel);
    part(`readout_${p}_y`, y);
  }
  part('readout_p_field', tid('fv-p-field'));
  part('track', tid('fv-p-scrubber'));
  part('thumb', tid('fv-p-scrubber-thumb'));

  // The settings panel, where a state has it open (settings-open): its two settings, the toggle's
  // options with their marks and words, the help line, the theme's label and the rows' parts.
  const menu = tid('settings-panel');
  if (shown(menu)) {
    part('settings_panel', menu);
    const [appearance, themes] = menu.children;
    text('settings_appearance_text', part('settings_appearance', appearance.children[0]));
    part('settings_mode', appearance.children[1]);
    for (const m of ['system', 'light', 'dark']) {
      const option = part(`settings_mode_${m}`, tid(`settings-mode-${m}`));
      part(`settings_mode_${m}_label`, option.firstElementChild);
      part(`settings_mode_${m}_icon`, option.querySelector('svg'));
      text(`settings_mode_${m}_text`, option.firstElementChild);
    }
    text('settings_help_text', part('settings_help', tid('settings-help')).firstElementChild);
    text('settings_theme_label_text', part('settings_theme_label', themes.children[0]).firstElementChild);
    [...part('settings_themes', tid('settings-themes')).children].forEach((row, i) => {
      part(`settings_row_${i}`, row);
      const [swatch, name, check] = row.firstElementChild.children;
      part(`settings_swatch_${i}`, swatch);
      text(`settings_name_${i}`, part(`settings_name_${i}`, name));
      part(`settings_check_${i}`, check);
    });
  }
  return {viewport: [innerWidth, innerHeight], boxes, texts, runs: lines, scroll};
}
"""

Dom = dict[str, Any]

# The layouts the port's tests lay out, each a state in tools/states and the shot that shows it.
FIXTURE_STATES = ("default", "stacked", "short", "narrow", "topbar", "settings-open")
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
    """crates/function-viewer/testdata/page.json: the named measures (FIXTURE_JS) at every shot of
    the fixture's states, by shot name, in WebKit and Chromium, with the viewport each was taken
    at."""
    shots: dict[str, Dom] = {}
    failed = False
    for name in FIXTURE_STATES:
        script = load(STATES / f"{name}.act")
        for engine in FIXTURE_ENGINES:
            measured: dict[str, Dom] = {}

            def measure(page: Page, shot: str, measured: dict[str, Dom] = measured) -> None:
                measured[shot] = cast(Dom, page.evaluate(FIXTURE_JS))

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
    fixture = {"reference": f"math-viz {_reference_commit()}", "shots": shots}
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(fixture, indent=1) + "\n")
    print(f"wrote {shown(out)}")
    return 0
