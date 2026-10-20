"""The settings panel's motion on the real clock, frame by frame (tools/run raf, h1-settings):
what a recording at 25 frames a second can't show (AGENTS.md: values sampled every animation
frame, timed with performance.now() inside the callback). On the reference, the panel's computed
opacity and scale each frame from the gear's click (its opening), and the new content's opacity
each frame from a pick of Light (the list's swap); on the web build, whether each animation frame
from the click (or the pick) drew a frame (Module.frames), and until when. The web build's frames
are the same functions of time as the reference's (the paused clock's states show them frame by
frame), so a frame on every animation frame means the same motion at real speed. The coordinates
and test ids name the settings panel; h1-settings keeps them with the page."""

import json
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import cast

from playwright.sync_api import Page, sync_playwright

from act import REPO, port
from browser import engine_type

OUT = REPO / ".local" / "shots" / "raf"

# At 1440 x 900 from the reference's DOM (tools/states/settings-open.act): the gear, the toggle's
# Light, the gap between the two settings.
GEAR = (1383, 48)
MODE_LIGHT = (1251, 129)
GAP = (1250, 181)

INIT = """localStorage.setItem('mathviz-light-dark',
  JSON.stringify("system"));
localStorage.setItem('mathviz-dark-theme',
  JSON.stringify("arctic-ice"));
localStorage.setItem('mathviz-light-theme',
  JSON.stringify("clean-teal"));"""

# Each animation frame for `ms`: performance.now() since the first, and what `read` reads then.
SAMPLER = """([ms, read]) => {
  const f = new Function('return (' + read + ')()');
  window.__samples = new Promise((done) => {
    const out = [];
    const t0 = performance.now();
    const tick = () => {
      const t = performance.now() - t0;
      out.push([t, f()]);
      if (t < ms) requestAnimationFrame(tick); else done(out);
    };
    requestAnimationFrame(tick);
  });
  return true;
}"""

PANEL = """() => {
  const p = document.querySelector('[data-testid="settings-panel"]');
  if (!p) return null;
  const cs = getComputedStyle(p);
  const m = new DOMMatrixReadOnly(cs.transform === 'none' ? undefined : cs.transform);
  return [Number(cs.opacity), m.a];
}"""

CONTENT = """() => {
  const row = document.querySelector('[data-testid="settings-theme-clean-teal"]');
  return row ? Number(getComputedStyle(row.firstElementChild).opacity) : null;
}"""

FRAMES = "() => Module.frames"


@dataclass
class Sampled:
    engine: str
    target: str
    motion: str
    said: str
    ok: bool


def open_page(page: Page, target: str) -> None:
    url = (
        f"http://localhost:{port('REACT_REF_PORT')}/#function-viewer"
        if target == "ref"
        else f"http://localhost:{port('WEB_PORT')}/"
    )
    page.goto(url, wait_until="load")
    if target == "ref":
        # React has mounted once the theme class is on the body (h1-settings's plane adds a
        # canvas to wait for instead).
        page.wait_for_function("() => document.body.className.length > 0")
    else:
        page.wait_for_function("() => window.Module?.started === true && Module.frames >= 1")
    page.wait_for_timeout(3000)  # the first paint is over


def sample(page: Page, ms: int, read: str, act: tuple[float, float]) -> list[tuple[float, object]]:
    page.evaluate(SAMPLER, [ms, read])
    page.mouse.click(*act)
    return [(float(t), v) for t, v in page.evaluate("() => window.__samples")]


def ref_rows(samples: list[tuple[float, object]], what: str) -> tuple[str, bool]:
    shown = [(t, v) for t, v in samples if v is not None]
    if not shown:
        return "no panel", False
    t0 = shown[0][0]
    if what == "open":
        looks = [(t, cast(list[float], v)) for t, v in shown]
        rows = [(round(t - t0), round(v[0], 3), round(v[1], 4)) for t, v in looks]
        full = next((t for t, a, _ in rows if a >= 1), None)
        peak = max(rows, key=lambda r: r[2])
        said = f"opacity 1 at {full} ms, the scale's peak {peak[2]} at {peak[0]} ms; " + ", ".join(
            f"{t}: {a}/{s}" for t, a, s in rows[:12]
        )
        return said, full is not None and 140 <= full <= 190 and 1.003 <= peak[2] <= 1.008
    rows2 = [(round(t - t0), round(cast(float, v), 3)) for t, v in shown]
    full = next((t for t, a in rows2 if a >= 1), None)
    said = f"the new content's opacity 1 at {full} ms; " + ", ".join(
        f"{t}: {a}" for t, a in rows2[:12]
    )
    return said, full is not None and 140 <= full <= 190


# An animation frame, ms: the sampler sees the web build's frame count change up to a frame after the
# frame that changed it (the two callbacks in either order), at the motion's start and at its end,
# so the frames it sees span the motion less up to a frame (M13a: 634 to 667 ms over the 643 ms
# opening in WebKit, every animation frame drawn).
FRAME_MS = 17


def web_rows(samples: list[tuple[float, object]], longest_ms: float) -> tuple[str, bool]:
    frames = [cast(int, v) for _, v in samples]
    times = [t for t, _ in samples]
    start = next((k for k in range(1, len(frames)) if frames[k] > frames[0]), None)
    if start is None:
        return "no frame drawn", False
    drew = [frames[k] - frames[k - 1] for k in range(start, len(frames))]
    last = max(k for k in range(len(drew)) if drew[k] > 0)
    gaps = [round(times[start + k] - times[start]) for k in range(last + 1) if drew[k] == 0]
    span = times[start + last] - times[start]
    said = (
        f"{sum(drew[: last + 1])} frames on {last + 1} animation frames from {round(times[start])} ms "
        f"to {round(times[start + last])} ms; none drawn at {gaps}"
    )
    return said, not gaps and span >= longest_ms - FRAME_MS


def run_engine(engine: str) -> list[Sampled]:
    out: list[Sampled] = []
    with sync_playwright() as playwright:
        browser = engine_type(playwright, engine).launch()
        try:
            for target in ("ref", "web"):
                context = browser.new_context(
                    viewport={"width": 1440, "height": 900},
                    device_scale_factor=2,
                    color_scheme="dark",
                )
                context.add_init_script(INIT)
                page = context.new_page()
                open_page(page, target)
                page.mouse.move(760, 48)
                page.wait_for_timeout(300)
                # The opening: 643 ms on the enter spring, its opacity over 160.
                samples = sample(page, 800, PANEL if target == "ref" else FRAMES, GEAR)
                said, ok = ref_rows(samples, "open") if target == "ref" else web_rows(samples, 640)
                out.append(Sampled(engine, target, "opening", said, ok))
                page.mouse.move(*GAP)
                page.wait_for_timeout(300)
                # The swap: Light picked, the light themes' content fading in over 160 ms.
                samples = sample(page, 400, CONTENT if target == "ref" else FRAMES, MODE_LIGHT)
                said, ok = ref_rows(samples, "swap") if target == "ref" else web_rows(samples, 150)
                out.append(Sampled(engine, target, "swap", said, ok))
                context.close()
        finally:
            browser.close()
    return out


def run_raf(engines: list[str], out: Path = OUT) -> int:
    report: list[dict[str, object]] = []
    failed = False
    for engine in engines:
        for s in run_engine(engine):
            print(
                f"{s.engine:9s} {s.target:4s} {s.motion:8s} {'ok  ' if s.ok else 'FAIL'} {s.said}"
            )
            report.append(asdict(s))
            failed |= not s.ok
    out.mkdir(parents=True, exist_ok=True)
    (out / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    print(f"report: {out / 'report.json'}")
    return 1 if failed else 0
