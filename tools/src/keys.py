"""The web build's browser shortcuts (web-keys): what the page prevents per case.

The user's ruling (2026-10-18): in the web app, browser keyboard shortcuts reach the browser as
they do on the React page. While a number field is edited, Cmd combos stay with the app, as in a
native text field; otherwise every Cmd+key combo goes to the browser untouched; Cmd+Opt combos
always go to the browser; the Meta key alone still reaches SDL (a Cmd-drag reads GUI as held).
web/library.js stops the pass-through combos before SDL's bubble-phase listener, so the browser
performs them; the field-edit flag (platform_web.rs's platform_field_editing, set when a field
opens and closes) tells the page when a field is edited.

This check records `defaultPrevented` per case on the Rust web build (`web`) and the Odin web
build (`odin-web`, the Odin record, prevented in every case), in each engine, routing each
target's web/ from disk (no WEB_PORT, no server): outside a field Meta+ArrowLeft, Meta+R and
Meta+Alt+I are not prevented on web; inside b's field Meta+ArrowLeft, Meta+A and Meta+ArrowRight
are prevented, each moving the caret as now, and the field's crop after each equals odin-web's
after the same gesture; after Escape Meta+ArrowLeft is not prevented again; a Cmd-drag on k's
box draws the snap badge and stroke identically to odin-web. The exit status is the web column's;
odin-web is reported, never judged."""

import json
from dataclasses import asdict, dataclass
from pathlib import Path

from playwright.sync_api import BrowserContext, Page, sync_playwright

from act import REPO, init_script, serve, web_dir
from browser import PageLog, engine_type
from images import difference, load_rgb, region
from script import Start
OUT = REPO / ".local" / "shots" / "keys"

# b's box (330, 513.5, 107 x 32 at 1440 x 900, field-arrows.act) and k's box centre (383, 468,
# drag-k-modes.act), in CSS px, as Playwright's mouse takes them.
B_BOX = (383, 529)
K_BOX = (383, 468)
# The field's crop in device px (density 2): b's row with room for the caret and selection.
FIELD_BOX = (620, 1000, 300, 110)

# Before any page script: a capture-phase recorder on window, so it sees every key event even
# when the page's own listener stops one (same node: stopPropagation spares same-node
# listeners). It keeps the event objects; Python reads `defaultPrevented` off them after the
# gesture, which is final then. (A microtask queued during dispatch does NOT run after every
# listener: measured in WebKit, it runs between the capture listeners, before the page's own
# and SDL's bubble listeners, so it reads too early. The prompt's microtask design is set aside
# for this lazy read, the reviewer's "after dispatch" point kept.)
RECORDER = """
window.__keysSeen = [];
for (const type of ["keydown", "keyup", "keypress"]) {
  window.addEventListener(type, (event) => {
    window.__keysSeen.push(event);
  }, {capture: true});
}
"""

# WebKit-internal noise when the browser acts on a pass-through shortcut (measured: a
# SecurityError from web-inspector://bootstrap.js as the page goes back): the browser's own UI
# failing headless, never app code. Named in the verdict, kept out of it.
BROWSER_NOISE = "web-inspector://"


def app_problems(log: PageLog, stacks: list[str]) -> list[str]:
    """The page's problems minus the browser's own shortcut noise (a pageerror whose stack leads
    into the browser's UI, e.g. its inspector bootstrap failing headless as it acts)."""
    noise = any(BROWSER_NOISE in stack for stack in stacks)
    return [
        p
        for p in log.problems()
        if not (noise and p == "pageerror: The operation is insecure.")
    ]


@dataclass
class Step:
    what: str
    ok: bool
    said: str


@dataclass
class KeyRecord:
    type: str
    key: str
    code: str
    meta: bool
    alt: bool
    prevented: bool


def records_since(page: Page, count: int) -> list[KeyRecord] | None:
    """The recorder's events since `count`, with their final `defaultPrevented`, oldest first;
    None when the page left (a pass-through shortcut the browser acted on)."""
    try:
        raw = page.evaluate(
            f"""window.__keysSeen.slice({count}).map(e => ({{type: e.type, key: e.key,
                code: e.code, metaKey: e.metaKey, altKey: e.altKey,
                prevented: e.defaultPrevented}}))"""
        )
    except Exception:
        return None
    return [
        KeyRecord(
            type=str(r["type"]),
            key=str(r["key"]),
            code=str(r["code"]),
            meta=bool(r["metaKey"]),
            alt=bool(r["altKey"]),
            prevented=bool(r["prevented"]),
        )
        for r in raw
    ]


def seen_count(page: Page) -> int:
    """How many events the recorder holds; -1 when the page left."""
    try:
        return int(page.evaluate("window.__keysSeen.length"))
    except Exception:
        return -1


def keydown(records: list[KeyRecord], code: str) -> KeyRecord | None:
    """The first keydown record for `code` (key names wobble under modifiers; codes don't)."""
    return next((r for r in records if r.type == "keydown" and r.code == code), None)


def fresh_page(context: BrowserContext, target: str) -> tuple[Page, PageLog, list[str]]:
    """A page routed from the target's web/ with ?trace, at its first frame, with page-error
    stacks collected (to tell the browser's own shortcut noise from app errors)."""
    page = context.new_page()
    log = PageLog()
    log.watch(page)
    stacks: list[str] = []
    page.on("pageerror", lambda error: stacks.append(str(error.stack or error)))
    page.goto(serve(context, web_dir(target, None)) + "?trace", wait_until="load")
    page.wait_for_function("() => window.Module?.started === true", polling=50)
    page.wait_for_function("() => Module.frames >= 1")
    page.wait_for_timeout(300)
    return page, log, stacks


def wait_console(log: PageLog, start: int, text: str, page: Page, timeout_ms: int = 3000) -> bool:
    """Whether `text` reaches the console within the timeout."""
    waited = 0
    while waited < timeout_ms:
        if any(text in line for line in log.console[start:]):
            return True
        page.wait_for_timeout(100)
        waited += 100
    return False


def passthrough_case(
    context: BrowserContext, target: str, combo: str, code: str, name: str
) -> tuple[Step, PageLog]:
    """One pass-through combo on a freshly loaded page: was its keydown prevented?"""
    page, log, stacks = fresh_page(context, target)
    try:
        count = seen_count(page)
        page.keyboard.press(combo)
        page.wait_for_timeout(500)
        records = records_since(page, count)
        if records is None:
            try:
                url = page.url
            except Exception:
                url = "?"
            step = Step(
                f"{name} outside a field not prevented",
                url != "?",
                f"the page left ({url}): the browser acted on the shortcut"
                if url != "?"
                else "the page died: no keydown recorded",
            )
        else:
            found = keydown(records, code)
            if found is None:
                step = Step(
                    f"{name} outside a field not prevented", False, "no keydown recorded"
                )
            else:
                step = Step(
                    f"{name} outside a field not prevented",
                    not found.prevented,
                    f"defaultPrevented={found.prevented} (key {found.key})",
                )
        problems = app_problems(log, stacks)
        if problems:
            step = Step(
                step.what, False, f"{step.said}; {len(problems)} problems: {problems[:2]}"
            )
        return step, log
    finally:
        page.close()


def field_case(
    context: BrowserContext, target: str, out: Path
) -> tuple[list[Step], dict[str, Path], PageLog]:
    """b's field: Meta+arrows/A prevented with the caret moved; Escape ends it."""
    steps: list[Step] = []
    crops: dict[str, Path] = {}
    page, log, stacks = fresh_page(context, target)
    try:
        out.mkdir(parents=True, exist_ok=True)
        start = len(log.console)
        page.mouse.click(*B_BOX)
        opened = wait_console(log, start, "field open in box", page)
        steps.append(
            Step(
                "field opens on b's box",
                opened,
                "trace shows the open" if opened else "no open line",
            )
        )
        if not opened:
            return steps, crops, log
        page.wait_for_timeout(400)

        def crop(name: str) -> Path:
            # A body selection from an unprevented Select All tints the composited page (measured
            # on odin-web: WebKit selects the body, an overlay over every pixel); the crops compare
            # the app's pixels, so the selection is cleared first (a no-op where nothing selected).
            page.evaluate("getSelection().removeAllRanges()")
            # The compositor needs a frame to drop the overlay before the shot.
            page.wait_for_timeout(300)
            shot = out / f"full-{name}.png"
            page.screenshot(path=shot)
            path = out / f"field-{name}.png"
            region(load_rgb(shot), FIELD_BOX).save(path)
            crops[name] = path
            return path

        def prevented(combo: str, code: str, what: str) -> KeyRecord | None:
            count = seen_count(page)
            page.keyboard.press(combo)
            page.wait_for_timeout(400)
            records = records_since(page, count)
            found = None if records is None else keydown(records, code)
            steps.append(
                Step(
                    what,
                    found is not None and found.prevented,
                    "the page left: the browser acted, nothing stayed with the app"
                    if records is None
                    else "defaultPrevented="
                    + ("?" if found is None else str(found.prevented).lower()),
                )
            )
            return found

        def differs(before: str, after: str, what: str) -> None:
            changed = difference(load_rgb(crops[before]), load_rgb(crops[after])).changed
            steps.append(Step(what, changed > 0, f"{changed} px differ"))

        crop("before")
        prevented("Meta+ArrowLeft", "ArrowLeft", "Meta+ArrowLeft in the field prevented")
        crop("after-left")
        differs("before", "after-left", "the caret moved to the start (crop differs)")
        prevented("Meta+KeyA", "KeyA", "Meta+A in the field prevented")
        crop("after-a")
        differs("after-left", "after-a", "all selected (crop differs)")
        prevented("Meta+ArrowRight", "ArrowRight", "Meta+ArrowRight in the field prevented")
        crop("after-right")
        differs("after-a", "after-right", "the caret moved to the end (crop differs)")

        # Measurement for the record: which keypress, if any, an engine fires for a Cmd combo.
        firing = page.evaluate(
            """[...new Set(window.__keysSeen.filter(e => e.type === "keypress" && e.metaKey)
                .map(e => e.code))]"""
        )
        steps.append(
            Step("keypress with Meta observed", True, f"codes: {list(firing)}")
        )

        start = len(log.console)
        page.keyboard.press("Escape")
        closed = wait_console(log, start, "field closed:", page)
        steps.append(
            Step(
                "field closes on Escape",
                closed,
                "trace shows the close" if closed else "no close line",
            )
        )
        page.wait_for_timeout(300)
        count = seen_count(page)
        page.keyboard.press("Meta+ArrowLeft")
        page.wait_for_timeout(500)
        records = records_since(page, count)
        found = None if records is None else keydown(records, "ArrowLeft")
        steps.append(
            Step(
                "Meta+ArrowLeft not prevented again",
                (records is None) or (found is not None and not found.prevented),
                "the page left: the browser acted on the shortcut again"
                if records is None
                else "defaultPrevented="
                + ("?" if found is None else str(found.prevented).lower()),
            )
        )
        problems = app_problems(log, stacks)
        steps.append(
            Step(
                "no errors",
                not problems,
                "none" if not problems else f"{len(problems)} problems: {problems[:2]}",
            )
        )
        return steps, crops, log
    finally:
        page.close()


def drag_case(context: BrowserContext, target: str, out: Path) -> tuple[Step, Path | None, PageLog]:
    """A Cmd-drag on k's box: Meta alone reaches SDL, so the drag snaps."""
    page, log, stacks = fresh_page(context, target)
    try:
        out.mkdir(parents=True, exist_ok=True)
        page.mouse.move(*K_BOX)
        page.wait_for_timeout(200)
        page.mouse.down()
        page.keyboard.down("Meta")
        for k in range(1, 11):
            page.mouse.move(K_BOX[0] + 5 * k, K_BOX[1])
            page.wait_for_timeout(16)
        page.wait_for_timeout(400)
        shot = out / "snap.png"
        page.screenshot(path=shot)
        page.keyboard.up("Meta")
        page.mouse.up()
        problems = app_problems(log, stacks)
        step = Step(
            "Cmd-drag on k's box snaps",
            not problems,
            f"shot {shot.name}" + ("" if not problems else f"; {len(problems)} problems"),
        )
        return step, shot, log
    finally:
        page.close()


def run_target(
    context: BrowserContext, target: str, out: Path
) -> tuple[list[Step], dict[str, Path], Path | None]:
    """Every case on one target: the steps, the field crops, the snap shot."""
    out.mkdir(parents=True, exist_ok=True)
    steps: list[Step] = []
    for combo, code, name in [
        ("Meta+ArrowLeft", "ArrowLeft", "Meta+ArrowLeft"),
        ("Meta+KeyR", "KeyR", "Meta+R"),
        ("Meta+Alt+KeyI", "KeyI", "Meta+Alt+I"),
    ]:
        step, _ = passthrough_case(context, target, combo, code, name)
        steps.append(step)
    field_steps, crops, _ = field_case(context, target, out)
    steps.extend(field_steps)
    drag_step, shot, _ = drag_case(context, target, out)
    steps.append(drag_step)
    return steps, crops, shot


def run_engine(engine: str, targets: list[str], out: Path) -> dict[str, list[Step]]:
    """One engine: each target's cases, then the web column judged against odin-web's."""
    report: dict[str, list[Step]] = {}
    crops: dict[str, dict[str, Path]] = {}
    shots: dict[str, Path | None] = {}
    with sync_playwright() as playwright:
        browser = engine_type(playwright, engine).launch()
        try:
            for target in targets:
                out_target = out / engine / target
                context = browser.new_context(
                    viewport={"width": 1440, "height": 900},
                    device_scale_factor=2,
                    color_scheme="dark",
                    reduced_motion="no-preference",
                    locale="en-US",
                    timezone_id="UTC",
                )
                context.add_init_script(init_script(Start(), target, False))
                context.add_init_script(RECORDER)
                try:
                    steps, field_crops, shot = run_target(context, target, out_target)
                finally:
                    context.close()
                report[target] = steps
                crops[target] = field_crops
                shots[target] = shot
        finally:
            browser.close()
    # The web column against the Odin record: each in-field crop and the snap
    # shot identical at threshold 0 to odin-web's after the same gesture.
    if "web" in report and "odin-web" in report:
        web_steps = report["web"]
        for name, what in [
            ("after-left", "Meta+ArrowLeft"),
            ("after-a", "Meta+A"),
            ("after-right", "Meta+ArrowRight"),
        ]:
            a, b = crops["web"].get(name), crops["odin-web"].get(name)
            if a is None or b is None:
                web_steps.append(
                    Step(f"field crop after {what} equals odin-web's", False, "crop missing")
                )
            else:
                diff = difference(load_rgb(a), load_rgb(b))
                web_steps.append(
                    Step(
                        f"field crop after {what} equals odin-web's",
                        diff.changed == 0,
                        f"{diff.changed} px differ at threshold 0",
                    )
                )
        a, b = shots["web"], shots["odin-web"]
        if a is None or b is None:
            web_steps.append(Step("Cmd-drag shot identical to odin-web's", False, "shot missing"))
        else:
            diff = difference(load_rgb(a), load_rgb(b))
            web_steps.append(
                Step(
                    "Cmd-drag shot identical to odin-web's",
                    diff.changed == 0,
                    f"{diff.changed} px differ at threshold 0",
                )
            )
    return report


def run_keys(engines: list[str], targets: list[str], out: Path = OUT) -> int:
    """The key check in each engine: reports per engine and target, the web column judged."""
    failed = False
    for engine in engines:
        report = run_engine(engine, targets, out)
        for target, steps in report.items():
            judged = target == "web"
            for step in steps:
                print(
                    f"{engine}/{target}: {'ok  ' if step.ok else 'FAIL'} {step.what}: {step.said}"
                )
                failed |= judged and not step.ok
            (out / engine / target).mkdir(parents=True, exist_ok=True)
            (out / engine / target / "report.json").write_text(
                json.dumps([asdict(s) for s in steps], indent=2) + "\n"
            )
        print(f"report: {out / engine}")
    return 1 if failed else 0
