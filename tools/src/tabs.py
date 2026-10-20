"""The prefs across two tabs of the web build (M12): a pick in one tab's settings panel shows in the
other, as React's storage subscription takes another tab's write (useAppTheme.ts; useAppTheme.test.ts
"take another tab's change, checked"). Two pages of one browser context share localStorage, and a
write in one fires the storage event in the other: the web build hands the stored text to the app
(web/library.js mv_prefs_load), which checks it as a read is and shows it without writing it back
(src/prefs.rs prefs_stored_elsewhere, which logs "prefs changed elsewhere: ..."). On the real
clock in each engine: tab A picks Tron Cyan, tab B follows; tab B picks Light, tab A follows; tab B
stores an invalid pick and tab A reads it checked; a removed key reads as the defaults."""

import json
import re
from dataclasses import asdict, dataclass
from pathlib import Path

import numpy as np
from playwright.sync_api import Page, sync_playwright

from act import REPO, port
from browser import PageLog, engine_type
from images import load_rgb

OUT = REPO / ".local" / "shots" / "tabs"

# At 1440 x 900, from the reference's DOM with the panel open (tools/states/settings-open.act).
GEAR = (1383, 48)
TRON_CYAN = (1251, 265)
MODE_LIGHT = (1251, 129)
PANEL_OPEN_MS = 700
# The page's background beside the top bar, where each theme's --background shows.
BACKGROUND = (8, 450)

CHANGED = re.compile(r"prefs changed elsewhere: (\{.*\})")


@dataclass
class Step:
    what: str
    ok: bool
    said: str


def changes(log: PageLog, start: int) -> list[str]:
    return [m[1] for line in log.console[start:] if (m := CHANGED.search(line))]


def background(page: Page, out: Path, name: str) -> tuple[int, int, int]:
    """The page's background colour in a screenshot, in device px at density 2."""
    path = out / f"{name}.png"
    page.screenshot(path=path)
    pixels = np.asarray(load_rgb(path))
    r, g, b = (int(v) for v in pixels[BACKGROUND[1] * 2, BACKGROUND[0] * 2])
    return r, g, b


def run_engine(engine: str, out: Path) -> list[Step]:
    out.mkdir(parents=True, exist_ok=True)
    url = f"http://localhost:{port('WEB_PORT')}/?trace"
    steps: list[Step] = []
    with sync_playwright() as playwright:
        browser = engine_type(playwright, engine).launch()
        try:
            context = browser.new_context(
                viewport={"width": 1440, "height": 900}, device_scale_factor=2, color_scheme="dark"
            )
            logs = [PageLog(), PageLog()]
            pages = [context.new_page(), context.new_page()]
            for page, log in zip(pages, logs, strict=True):
                log.watch(page)
                page.goto(url, wait_until="load")
                page.wait_for_function(
                    "() => window.Module?.started === true && Module.frames >= 1"
                )
            a, b = pages
            a.wait_for_timeout(500)
            dark = background(b, out, "b-arctic-ice")

            # A picks Tron Cyan in its panel: B follows.
            start = len(logs[1].console)
            a.bring_to_front()
            a.mouse.click(*GEAR)
            a.wait_for_timeout(PANEL_OPEN_MS)
            a.mouse.click(*TRON_CYAN)
            b.wait_for_timeout(500)
            seen = changes(logs[1], start)
            tron = background(b, out, "b-tron-cyan")
            want = '{"mode":"system","dark":"tron-cyan","light":"clean-teal"}'
            steps.append(
                Step(
                    "A picks Tron Cyan, B follows",
                    seen == [want] and tron != dark,
                    f"B heard {seen}; its background {dark} -> {tron}",
                )
            )

            # B picks Light in its panel: A follows.
            start = len(logs[0].console)
            b.bring_to_front()
            b.mouse.click(*GEAR)
            b.wait_for_timeout(PANEL_OPEN_MS)
            b.mouse.click(*MODE_LIGHT)
            a.wait_for_timeout(500)
            seen = changes(logs[0], start)
            want = '{"mode":"light","dark":"tron-cyan","light":"clean-teal"}'
            light = background(a, out, "a-clean-teal")
            steps.append(
                Step(
                    "B picks Light, A follows",
                    seen == [want] and light != tron,
                    f"A heard {seen}; its background {light}",
                )
            )

            # Another tab's write is checked as a read is: a dark theme can't be the light pick.
            start = len(logs[0].console)
            b.evaluate("""localStorage.setItem('mathviz-function-viewer-rust',
                JSON.stringify({mode: 'dark', dark: 'midnight-violet', light: 'tron-cyan'}))""")
            a.wait_for_timeout(500)
            seen = changes(logs[0], start)
            want = '{"mode":"dark","dark":"midnight-violet","light":"clean-teal"}'
            steps.append(Step("a checked write", seen == [want], f"A heard {seen}"))

            # A removed key reads as the defaults.
            start = len(logs[0].console)
            b.evaluate("localStorage.removeItem('mathviz-function-viewer-rust')")
            a.wait_for_timeout(500)
            seen = changes(logs[0], start)
            want = '{"mode":"system","dark":"arctic-ice","light":"clean-teal"}'
            steps.append(Step("a removed key", seen == [want], f"A heard {seen}"))

            problems = logs[0].problems() + logs[1].problems()
            steps.append(
                Step("no errors", not problems, f"{len(problems)} problems: {problems[:3]}")
            )
            context.close()
        finally:
            browser.close()
    return steps


def run_tabs(engines: list[str], out: Path = OUT) -> int:
    report: dict[str, list[dict[str, object]]] = {}
    failed = False
    for engine in engines:
        steps = run_engine(engine, out / engine)
        report[engine] = [asdict(s) for s in steps]
        for s in steps:
            print(f"{engine}: {'ok  ' if s.ok else 'FAIL'} {s.what}: {s.said}")
            failed |= not s.ok
    out.mkdir(parents=True, exist_ok=True)
    (out / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    print(f"report: {out / 'report.json'}")
    return 1 if failed else 0
