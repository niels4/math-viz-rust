"""The plane's view at each shot of a state (`tools/run view`): on the React reference, the canvas's
data-zoom, data-origin-x and data-origin-y (the zoom in px per unit, and where the maths origin
lands in the plane's px, on the device pixel grid) in each engine; natively, the same from the
build's views.json, which the native harness writes at each shot. A pan, a zoom, a fling's frames and
a key's step are judged by these numbers, the port's beside the reference's, before any pixel: Q
and the plane's motion (M10's) differ in the frames meanwhile. The web build draws native's pixels
(each state's diffs show it), so its view is native's. Beside them, the cursor each shows: on the
reference the one the browser shows over the element under the pointer (the deepest :hover): its
computed cursor, or for `auto` the pointing hand in a link (WebKit computes `auto` there and shows
the hand; Chromium computes `pointer`), the I-beam over one of its text nodes a press can start a
selection in (its used user-select not `none`; the pointer by the script's moves), else the arrow;
on the web build in WebKit its canvas's (web/library.js sets it), and natively the one the app set.

The port is held to the first engine's numbers (WebKit's by default): the origin exactly, the zoom
within 1e-7 of it (natively a trackpad's pinch comes as SDL's scale, a float, which the browsers
hand over as whole px of a wheel: the two zooms differ in their eighth digit, a millionth of a px
across the plane). The others are printed beside it; Chromium hands a wheel over at half its delta
under a device scale of 2 (Playwright's input, the reference's as much as the port's web build's),
so its numbers differ wherever a state turns the wheel."""

import json
from pathlib import Path
from typing import Any

from playwright.sync_api import Page

from act import run, run_native, shown
from script import Click, Frames, Move, Script, Shot, load
from states import STATES

VIEW_JS = """([x, y]) => {
  const canvas = document.querySelector('[data-testid="cartesian-canvas"]');
  const hovered = [...document.querySelectorAll(':hover')].pop();
  const selects = (el) => {
    for (let e = el; e; e = e.parentElement) {
      const style = getComputedStyle(e);
      const value = style.userSelect || style.webkitUserSelect;
      if (value && value !== 'auto') return value !== 'none';
    }
    return true;
  };
  const overText = (el) => {
    if (el !== document.elementFromPoint(x, y) || !selects(el)) return false;
    for (const node of el.childNodes) {
      if (node.nodeType !== Node.TEXT_NODE || !node.textContent.trim()) continue;
      const range = document.createRange();
      range.selectNodeContents(node);
      for (const r of range.getClientRects()) {
        if (x >= r.left && x < r.right && y >= r.top && y < r.bottom) return true;
      }
    }
    return false;
  };
  let cursor = hovered ? getComputedStyle(hovered).cursor : 'auto';
  if (cursor === 'auto') {
    cursor = hovered?.closest('a[href]') ? 'pointer' : hovered && overText(hovered) ? 'text' : 'default';
  }
  return {
    zoom: Number(canvas.dataset.zoom),
    origin_x: Number(canvas.dataset.originX),
    origin_y: Number(canvas.dataset.originY),
    cursor,
  };
}"""

# The web build's cursor: the rule web/library.js writes for SDL's canvas.
WEB_CURSOR_JS = """() => {
  const cursor = getComputedStyle(document.getElementById('canvas')).cursor;
  return cursor === 'auto' ? 'default' : cursor;
}"""

type View = dict[str, Any]


def same_view(a: View, b: View) -> bool:
    return (
        abs(a["zoom"] - b["zoom"]) <= 1e-7 * abs(b["zoom"])
        and a["origin_x"] == b["origin_x"]
        and a["origin_y"] == b["origin_y"]
    )


def shown_view(v: View | None) -> str:
    if v is None:
        return "-"
    return f"{v['zoom']:.6g} ({v['origin_x']:g}, {v['origin_y']:g})"


def pointer_at_shots(script: Script) -> dict[str, tuple[float, float]]:
    """Where the reference's pointer is at each shot: its moves and clicks before it."""
    at: tuple[float, float] = (-1, -1)
    found: dict[str, tuple[float, float]] = {}
    for action in script.actions:
        if not action.runs_on("ref"):
            continue
        match action.verb:
            case Move(to):
                at = to
            case Click(where):
                at = where
            case Shot(name):
                found[name] = at
            case Frames() as frames:
                found |= {f"{frames.name}-{t:04d}": at for t in frames.times()}
            case _:
                pass
    return found


def reference_views(path: Path, engine: str, out: Path) -> tuple[dict[str, View], list[str]]:
    views: dict[str, View] = {}
    script = load(path)
    pointer = pointer_at_shots(script)

    def on_shot(page: Page, name: str) -> None:
        views[name] = page.evaluate(VIEW_JS, list(pointer.get(name, (-1, -1))))

    result = run(script, "ref", engine, out, on_shot)
    return views, result.problems


def web_cursors(path: Path, engine: str, out: Path) -> tuple[dict[str, str], list[str]]:
    cursors: dict[str, str] = {}

    def on_shot(page: Page, name: str) -> None:
        cursors[name] = page.evaluate(WEB_CURSOR_JS)

    result = run(load(path), "web", engine, out, on_shot)
    return cursors, result.problems


def native_views(path: Path, out: Path) -> tuple[dict[str, View], list[str]]:
    result = run_native(load(path), out)
    result.write_report()
    views: dict[str, View] = {}
    file = out / "views.json"
    if file.exists():
        for line in file.read_text().splitlines():
            view: View = json.loads(line)
            views[view["shot"]] = view
    return views, result.problems


def run_view(names: list[str], engines: list[str], out: Path) -> int:
    """Each state's views on the reference in each engine and natively, compared shot by shot."""
    failed = False
    report: dict[str, object] = {}
    lines: list[str] = []
    for name in names:
        path = STATES / f"{name}.act"
        problems: list[str] = []
        refs: dict[str, dict[str, View]] = {}
        for engine in engines:
            views, trouble = reference_views(path, engine, out / name / f"ref-{engine}")
            refs[engine] = views
            problems += [f"ref-{engine}: {p}" for p in trouble]
        port, trouble = native_views(path, out / name / "native")
        problems += [f"native: {p}" for p in trouble]
        web, trouble = web_cursors(path, engines[0], out / name / f"web-{engines[0]}")
        problems += [f"web-{engines[0]}: {p}" for p in trouble]
        first = refs[engines[0]]
        shots = list(first)
        differ = [s for s in shots if s not in port or not same_view(port[s], first[s])]
        cursors_differ = [
            s
            for s in shots
            if port.get(s, {}).get("cursor") != first[s]["cursor"]
            or web.get(s) != first[s]["cursor"]
        ]
        lines.append(
            f"{name}: {len(shots) - len(differ)} of {len(shots)} shots as {engines[0]}'s, "
            f"{len(shots) - len(cursors_differ)} with its cursor"
        )
        for shot in shots:
            mark = "  " if shot not in differ and shot not in cursors_differ else "!="
            cells = [shown_view(refs[e].get(shot)) for e in engines]
            cursor = (
                f"cursor ref {first[shot]['cursor']}, web {web.get(shot, '-')}, "
                f"native {port.get(shot, {}).get('cursor', '-')}"
            )
            lines.append(
                f"  {mark} {shot:<14} native {shown_view(port.get(shot)):<26} "
                + "  ".join(f"{e} {c:<26}" for e, c in zip(engines, cells, strict=True))
                + f"  {cursor}"
            )
        for problem in problems:
            lines.append(f"  problem: {problem}")
        failed |= bool(differ) or bool(cursors_differ) or bool(problems)
        report[name] = {
            "native": port,
            f"web-{engines[0]}-cursors": web,
            **{f"ref-{e}": refs[e] for e in engines},
            "differ": differ,
            "cursors_differ": cursors_differ,
            "problems": problems,
        }
    out.mkdir(parents=True, exist_ok=True)
    (out / "view.json").write_text(json.dumps(report, indent=1))
    (out / "view.txt").write_text("\n".join(lines) + "\n")
    print("\n".join(lines))
    print(f"{shown(out / 'view.txt')}")
    return 1 if failed else 0
