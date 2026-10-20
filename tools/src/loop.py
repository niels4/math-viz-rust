"""The loop and the platform on the real clock (plan section 4, M2): what a paused clock and a
script's clock can't show. The web build runs in each engine through Playwright with ?trace, so
the app logs each fact and timer to the console; the native build runs the scripts in tools/live
through SDL's own loop (--live), logging to stdout, and is sampled with ps and sample while it
waits. Each check prints a line of what was measured; report.json keeps them all."""

import json
import math
import re
import signal
import statistics
import subprocess
import threading
import time
from collections.abc import Callable
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import cast

from playwright.sync_api import Page, sync_playwright

from act import NATIVE, REPO, port, shown
from browser import PageLog, engine_type

LIVE = REPO / "tools" / "live"

# The C3 sidebar at 1440 x 900 (sidebar.rs, number_box.rs; the reference's DOM): the
# centre of each row's number box (a rest shows its tip, a click types into it), a's chip, a's flip
# toggle, the plane. The motion the frame-pacing checks count is a flip's value jump (FV 05: the
# sweep spring, 702 ms; motion_driver.rs logs it); the prefs checks pick the mode in the
# settings panel (settings.rs): the gear opens it, Light and then Dark are picked in its
# Appearance toggle, each written as it applies (the reference's DOM: the gear (1363, 28, 40 x 40),
# the toggle's options 91.33 x 34 from (1113, 112)); the panel's opening runs 643 ms.
BOXES = [(383, 424), (383, 468), (383, 529), (383, 573)]
FIELD = BOXES[0]
CHIP = (70, 424)
FLIP_A = (308, 424)
# The plane's empty background in every state the checks pass through, where its cursor is the
# crosshair and a drag pans it. The plane spans 488..1424 x 92..884 with the origin at its centre
# (956, 488), 50 px a unit; what the pointer can take hold of lies up and right of the origin (since
# M9: the anchor at the origin and the stretch grip at (1, a), 44 px boxes, which show the open
# hand; P at x 2, a 48 px box; the edge markers), and the chrome sits in the plane's corners. So
# (700, 700), the point (-5.12, -4.24), far below the curve for any a the checks set.
PLANE = (700, 700)
GEAR = (1383, 48)
MODE_LIGHT = (1251, 129)
MODE_DARK = (1343, 129)
PANEL_OPEN_MS = 700

# The view's first paint (FV 05 › Draw-on timeline) keeps the loop awake for its first 2.6 s.
FIRST_PAINT_MS = 2600

# A 702 ms value jump draws 0.702 x the refresh rate +- 3 frames, counted while it moves with its
# first frame included (M2's review: the plan's "40 +- 3" miscounted): at 60 Hz one at 0, 16.7,
# ... 700 ms, 43, and SDL adds a turn when its callback rate leaves waitevent.
TWEEN_FRAMES = (39, 45)
JUMP_MS = 702

# The tip's 400 ms timer, armed by a rest on each box in turn. Its wake comes late by two shares,
# which the app logs apart (loop.rs, woken). The app's own runs from the deadline to the time
# its timer was armed to fire, by the interval SDL hands back: the timer is armed after the turn's
# frame for the time left then (`schedule`), so this share is 0 unless the deadline had passed by
# then, and each wake's is held within APP_SHARE_MS (an app that arms 15 ms late shows 15). Armed
# instead from the turn's start, it was the frame's time, 2.3 to 3.4 ms, and 14 to 16 ms in runs
# where this login's load stretched the frames (M13b's review). The timer's own runs from then to
# the posted event, and it is this login's (CLAUDE.md: Harness notes): natively SDL's timer thread posts
# it 2.6 to 26 ms late, from one batch of runs to the next and not with the load average (the M7
# review); on the web Firefox's setTimeout follows the same login (0 to 16 ms recorded), WebKit's
# and Chromium's 0 to 7. So the timer's share is reported, its spread over --runs, and gated only at
# WAKE_LATE_MS, where a wake was lost. What the app controls is held to: its share, no wake before
# its deadline, and the turn after each wake firing the timer, natively at once (0.1 to 1.1 ms
# measured) and on the web at the next animation frame, which is the browser's to time (up to
# 24 ms recorded in Firefox; a loop that didn't resume on the wake would wait for the next input,
# 300 ms on). On the web SDL's clock is Date.now() in whole ms (SDL asks for CLOCK_MONOTONIC_RAW,
# which Emscripten's clock_gettime refuses, and falls back to gettimeofday), and Firefox jitters
# its clamped clock, so a punctual wake there may read up to WEB_CLOCK_MS early.
RESTS = 5
APP_SHARE_MS = 2.0
WAKE_LATE_MS = 50.0
NATIVE_TURN_MS = 2.0
WEB_TURN_MS = 50.0
WEB_CLOCK_MS = 1.0

# The tip's rest (NumberBox.tsx REST_MS): each tip timer falls due this long after the pointer's
# move onto the box, which the app counts from that move's own time (M9: SDL stamps an event as it
# is queued). The checks take the deadline from the run's own timeline, not from the app's word,
# since every lateness above is measured from the app's own deadline and a tip armed for 300 ms
# would pass them (the M7h review). Natively the timeline is live.json: the move's posted time (SDL
# stamps the event as the driver posts it, within µs), and the turn that fired the tip less how
# late that turn says it fired: 400.0 to 400.1 ms, held within NATIVE_REST_SLACK_MS. On the web it
# is the page's clock stamped as things happen (STAMPER): the move, by a listener ahead of SDL's on
# the same pointermove, and the app's console line firing the tip, less how late it says it fired.
# SDL's clock there reads whole ms (above), so the two agree within a ms each way, and
# WEB_REST_SLACK_MS allows Firefox's jittered clock too. The move is the first pointer event of the
# gesture: Firefox sends a first move's pointerover and pointerenter at once (SDL's enter handler
# sends the motion then) and its pointermove at the next refresh, 21 ms on. A rest counted from the
# app's own "set" line instead reads short, by however long the move waited for the loop's next
# turn (369 to 399 ms in M13b's review).
REST_MS = 400.0
NATIVE_REST_SLACK_MS = 1.0
WEB_REST_SLACK_MS = 3.0

# Tab from a's box, where text was typed: k's box, b's flip toggle, b's box; Shift+Tab back to the
# flip; Tab to b's box again (the stops in FV 07's reading order; a parameter's index: a 0, k 1,
# b 2, h 3). Enter on b's box then opens its field.
TAB_ORDER = [("Box", 1), ("Flip", 2), ("Box", 2), ("Flip", 2), ("Box", 2)]

TWEEN_LINE = re.compile(
    r"motion: (\d+) frames while the plane moved over (\d+) ms[^;]*; at rest ([\d.]+) ms"
)
TIMER_LINE = re.compile(r"timer Tip: fired ([\d.]+) ms after its deadline")
TIMER_SET_LINE = re.compile(r"timer Tip: set for ([\d.]+) ms")
WAKE_LINE = re.compile(
    r"wake: posted (-?[\d.]+) ms after its deadline, its timer armed to fire (-?[\d.]+) ms after it"
)
FOCUS_LINE = re.compile(r'focus Hit\{kind = "?(\w+)"?, index = (\d+)\}')
TEXT_LINE = re.compile(r'input text "(.*)"')
PINCH_LINE = re.compile(r"input pinch ([\d.]+)")
SCHEME_LINE = re.compile(r"scheme (Light|Dark)")
FIELD_LINE = re.compile(r'field closed: "(.*)"')
OPEN_LINE = re.compile(r"field open in box (\w+)")
PREFS_LINE = re.compile(r"prefs (read|written): (\{.*\})")


@dataclass
class Check:
    surface: str
    name: str
    ok: bool
    said: str  # what was measured, in words
    data: dict[str, object] = field(default_factory=lambda: {})

    def line(self) -> str:
        return f"{self.surface:13} {self.name:8} {'ok' if self.ok else 'FAILED':6} {self.said}"


def run_loop(engines: list[str], out: Path, native: bool, runs: int = 1) -> int:
    """The checks once into `out`, or `runs` times back to back into out/run-01 and on, then how
    many runs passed and the timers' spread over all of them (summary.json)."""
    if runs == 1:
        checks = loop_once(engines, out, native)
        return 0 if checks and all(c.ok for c in checks) else 1
    every: list[list[Check]] = []
    for run in range(1, runs + 1):
        print(f"run {run} of {runs}")
        every.append(loop_once(engines, out / f"run-{run:02}", native))
    return summarize(every, out)


def loop_once(engines: list[str], out: Path, native: bool) -> list[Check]:
    out.mkdir(parents=True, exist_ok=True)
    checks: list[Check] = []
    if native:
        checks += native_checks(out / "native")
    for engine in engines:
        checks += web_checks(engine, out / f"web-{engine}")
    for check in checks:
        print(check.line())
    (out / "report.json").write_text(json.dumps([asdict(c) for c in checks], indent=2))
    print(f"report: {shown(out / 'report.json')}")
    return checks


def summarize(every: list[list[Check]], out: Path) -> int:
    """Runs back to back: which passed every check, each check's pass count, and the tip timer's
    wakes, their two shares and the turns pooled per surface."""
    passed = [bool(checks) and all(c.ok for c in checks) for checks in every]
    print(f"{len(every)} runs back to back: {sum(passed)} passed every check")
    for run, checks in enumerate(every, 1):
        if failed := [f"{c.surface} {c.name}" for c in checks if not c.ok]:
            print(f"  run {run:02} FAILED: {', '.join(failed)}")
    names = list(dict.fromkeys((c.surface, c.name) for checks in every for c in checks))
    counts = {
        f"{surface} {name}": sum(
            c.ok for checks in every for c in checks if (c.surface, c.name) == (surface, name)
        )
        for surface, name in names
    }
    timers: dict[str, object] = {}
    print(
        "the tip's timer over every run: its wakes, ms after the 400 ms deadline, the app's share "
        "and the timer's; the turns after"
    )
    for surface in dict.fromkeys(s for s, name in names if name == "timer"):
        found = [c for checks in every for c in checks if (c.surface, c.name) == (surface, "timer")]

        def pooled(key: str, found: list[Check] = found) -> list[float]:
            return [v for c in found for v in cast(list[float], c.data.get(key, []))]

        wakes, turns = pooled("wake_ms"), pooled("turn_ms")
        app_share, timer_share = pooled("app_share_ms"), pooled("timer_share_ms")
        timers[surface] = {
            "wake_ms": wakes,
            "app_share_ms": app_share,
            "timer_share_ms": timer_share,
            "turn_ms": turns,
            "wake_spread": spread(wakes),
            "app_share_spread": spread(app_share),
            "timer_share_spread": spread(timer_share),
            "turn_spread": spread(turns),
            "wake_histogram": histogram(wakes),
        }
        print(f"  {surface:13} {len(wakes)} wakes: {spread_text(spread(wakes))}")
        print(f"  {'':13} by 5 ms: {histogram(wakes)}")
        print(f"  {'':13} the app's share: {spread_text(spread(app_share))}")
        print(f"  {'':13} the timer's: {spread_text(spread(timer_share))}")
        print(f"  {'':13} the turn: {spread_text(spread(turns))}")
    summary = {
        "runs": len(every),
        "passed": sum(passed),
        "passed_by_run": passed,
        "passes_by_check": counts,
        "timer": timers,
        "reports": [str(out / f"run-{run:02}" / "report.json") for run in range(1, len(every) + 1)],
    }
    (out / "summary.json").write_text(json.dumps(summary, indent=2))
    print(f"summary: {shown(out / 'summary.json')}")
    return 0 if all(passed) else 1


# Native: tools/live through SDL's own loop.


@dataclass
class LiveRun:
    code: int
    lines: list[str]
    record: dict[str, object]

    def found(self, pattern: re.Pattern[str]) -> list[re.Match[str]]:
        return [m for line in self.lines if (m := pattern.search(line))]


# Called with the live run's process id and its output so far, while it runs.
WhileRunning = Callable[[int, list[str]], None]


def live(
    script: str,
    out: Path,
    prefs: Path,
    while_running: WhileRunning | None = None,
) -> LiveRun:
    """Runs tools/live/<script>.act with --live at a fixed 60 Hz (the driver's measurement: swap
    interval 0, SDL's callback rate at 60), logging each fact, into `out`."""
    out.mkdir(parents=True, exist_ok=True)
    command = [
        str(NATIVE),
        *("--live", str(LIVE / f"{script}.act"), "--out", str(out)),
        *("--prefs", str(prefs), "--pace", "60", "--trace"),
    ]
    process = subprocess.Popen(
        command, cwd=REPO, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True
    )
    lines: list[str] = []

    def read() -> None:
        assert process.stdout is not None
        for line in process.stdout:
            lines.append(line.rstrip("\n"))

    reader = threading.Thread(target=read)
    reader.start()
    if while_running is not None:
        while_running(process.pid, lines)
    try:
        code = process.wait(timeout=120)
    except subprocess.TimeoutExpired:
        process.kill()
        code = -9
    reader.join()
    (out / "log.txt").write_text("\n".join(lines) + "\n")
    path = out / "live.json"
    record: dict[str, object] = json.loads(path.read_text()) if path.exists() else {}
    return LiveRun(code, lines, record)


def problems(run: LiveRun) -> list[str]:
    bad = [line for line in run.lines if "[ERROR]" in line or "[FATAL]" in line]
    if run.code != 0:
        bad.append(f"exit status {run.code}")
    return bad


def native_checks(out: Path) -> list[Check]:
    if not NATIVE.exists():
        return [Check("native", "build", False, f"no {shown(NATIVE)}: run ./build.sh native")]
    prefs = out / "prefs"
    for old in prefs.glob("*"):
        old.unlink()
    return [
        native_idle(out / "idle", prefs),
        native_tween(out / "tween", prefs),
        native_timer(out / "timer", prefs),
        *native_input(out / "input", prefs),
        native_prefs(out / "prefs-runs", out / "prefs-fresh"),
        native_signals(out / "signals", prefs),
    ]


class IdleSampler:
    """Waits for the live run's `awake` shot, then a second, then samples the process for 3 s:
    its CPU time before and after (ps) and its threads' stacks (sample)."""

    def __init__(self, out: Path) -> None:
        self.out = out
        self.cpu: list[float] = []
        self.span = 0.0
        self.states: list[str] = []

    def __call__(self, pid: int, lines: list[str]) -> None:
        deadline = time.monotonic() + 30
        while not any("awake.png" in line for line in lines) and time.monotonic() < deadline:
            time.sleep(0.05)
        time.sleep(1.0)
        began = time.monotonic()
        self.cpu.append(cpu_seconds(pid))
        self.states.append(ps_state(pid))
        subprocess.run(
            ["sample", str(pid), "3", "-mayDie", "-file", str(self.out / "sample.txt")],
            capture_output=True,
        )
        self.cpu.append(cpu_seconds(pid))
        self.states.append(ps_state(pid))
        self.span = time.monotonic() - began


def cpu_seconds(pid: int) -> float:
    """The process's CPU time so far, from ps (minutes:seconds.hundredths)."""
    text = subprocess.run(["ps", "-o", "time=", "-p", str(pid)], capture_output=True, text=True)
    minutes, _, seconds = text.stdout.strip().partition(":")
    return int(minutes) * 60 + float(seconds) if seconds else math.nan


def ps_state(pid: int) -> str:
    text = subprocess.run(["ps", "-o", "state=", "-p", str(pid)], capture_output=True, text=True)
    return text.stdout.strip()


# How `sample` heads the main thread's call graph: by its dispatch queue ("DispatchQueue_1:
# com.apple.main-thread  (serial)"), or, when the thread served more than one queue while it was
# sampled, "Main Thread   DispatchQueue_<multiple>" (the M7h review's run 04). The Rust binary's
# main thread carries the pthread name "main" (std names it; the Odin binary sets none), which
# `sample` prints instead of any queue label, so that heads it too (a thread named exactly
# "main" is the process main thread).
MAIN_THREAD_LABELS = ("com.apple.main-thread", "Main Thread")
# The Rust main thread's exact head (no queue label follows the name).
MAIN_THREAD_RUST = ": main"


def main_thread_stack(sample: Path) -> tuple[int, int, list[str]] | None:
    """From `sample`'s call graph: the main thread's sample count, how many of them sit in
    mach_msg2_trap under SDL_WaitEventTimeoutNS, and its heaviest path's frames; None when the
    file holds no main thread."""
    if not sample.exists():
        return None
    lines = sample.read_text().splitlines()
    start = next(
        (
            i
            for i, line in enumerate(lines)
            if " Thread_" in line
            and (
                any(label in line for label in MAIN_THREAD_LABELS)
                or line.rstrip().endswith(MAIN_THREAD_RUST)
            )
        ),
        None,
    )
    if start is None:
        return None
    total = int(lines[start].split()[0])
    frames: list[str] = []
    parked = 0
    waiting = False
    for line in lines[start + 1 :]:
        stripped = line.lstrip(" +!:|")
        if not stripped or not stripped[0].isdigit() or " Thread_" in line:
            break
        count, _, rest = stripped.partition(" ")
        name = rest.split("  (in ")[0]
        if int(count) * 2 < total:
            continue
        frames.append(f"{count} {name}")
        waiting |= "SDL_WaitEventTimeoutNS" in name
        if waiting and name.startswith("mach_msg2_trap"):
            parked = int(count)
    return total, parked, frames


def turns(record: dict[str, object]) -> list[tuple[float, bool]]:
    """The live run's turns of the loop: when each began (ms) and whether it drew."""
    raw = cast(list[list[float]], record.get("turns", []))
    return [(float(t[0]), bool(t[1])) for t in raw]


def steps(record: dict[str, object]) -> list[dict[str, object]]:
    """The live run's steps: when each was due (`at`), when it was posted, and what it was."""
    return cast(list[dict[str, object]], record.get("steps", []))


def posted(record: dict[str, object], what: str, after: float = -1) -> float:
    """When the first step whose description starts with `what` (after `after`) was posted, ms."""
    for step in steps(record):
        at = float(str(step["posted"]))
        if str(step["what"]).startswith(what) and at > after:
            return at
    return math.nan


def native_idle(out: Path, prefs: Path) -> Check:
    sampler = IdleSampler(out)
    run = live("idle", out, prefs, sampler)
    record = run.record
    log = turns(record)
    shot_at = posted(record, "shot awake")
    wake_at = posted(record, "Pointer_Event", after=shot_at)
    # The shot holds the main thread while it writes its PNG; then nothing until the next move.
    after_shot = [t for t in log if shot_at <= t[0]]
    idle_from = after_shot[0][0] if after_shot else math.nan
    during = [t for t in log if idle_from < t[0] < wake_at]
    woken = [t for t in log if t[0] >= wake_at]
    stack = main_thread_stack(out / "sample.txt")
    total, parked, frames = stack or (0, 0, [])
    cpu = sampler.cpu[1] - sampler.cpu[0] if len(sampler.cpu) == 2 else math.nan
    percent = cpu / sampler.span * 100 if sampler.span else math.nan
    drawn_before = [t for t in log if t[0] < shot_at and t[1]]
    rests = native_rests(run)
    rested, rest_words = rests_ok(rests, 1, web=False)
    ok = (
        rested
        and not problems(run)
        and len(drawn_before) >= 3
        and not during
        and bool(woken)
        and woken[0][1]
        and cpu < 0.02
        and stack is not None
        and total > 0
        and parked >= total * 0.99
    )
    if stack is None:
        thread = (
            f"NO MAIN THREAD in {shown(out / 'sample.txt')}: no thread labelled "
            f"{' or '.join(repr(label) for label in MAIN_THREAD_LABELS)} (did sample run?)"
        )
    else:
        thread = f"main thread {parked} of {total} samples in mach_msg2_trap under SDL_WaitEventTimeoutNS"
    said = (
        f"woke for the move and the tip (frames at {', '.join(f'{t[0]:.0f}' for t in drawn_before)} ms), "
        f"{rest_words}; then {wake_at - idle_from:.0f} ms with {len(during)} turns of the loop; "
        f"{cpu:.2f} s of CPU over {sampler.span:.1f} s ({percent:.1f} %), ps state {'/'.join(sampler.states)}; "
        f"{thread}; woken again at {woken[0][0] if woken else math.nan:.0f} ms, a frame drawn"
    )
    return Check(
        "native",
        "idle",
        ok,
        said,
        {
            "turns": log,
            "rest_ms": rests,
            "idle_ms": wake_at - idle_from,
            "turns_while_idle": len(during),
            "cpu_s": cpu,
            "span_s": sampler.span,
            "ps_states": sampler.states,
            "main_thread": frames,
            "problems": problems(run),
            "out": str(out),
        },
    )


def native_tween(out: Path, prefs: Path) -> Check:
    run = live("tween", out, prefs)
    found = run.found(TWEEN_LINE)
    if not found:
        return Check("native", "tween", False, "no tween line", {"problems": problems(run)})
    # The last line is the flip's jump: the first paint's comes before it.
    frames, span, rest = int(found[-1][1]), int(found[-1][2]), float(found[-1][3])
    key = posted(run.record, "Button_Event")
    moving = [t for t in turns(run.record) if t[0] >= key and t[1]][: frames + 1]
    gaps = [b[0] - a[0] for a, b in zip(moving[1:], moving[2:], strict=False)]
    ok = not problems(run) and TWEEN_FRAMES[0] <= frames <= TWEEN_FRAMES[1] and span == JUMP_MS
    said = (
        f"{frames} frames while it moved over {span} ms, at rest {rest:.1f} ms after it began "
        f"(want {TWEEN_FRAMES[0]} to {TWEEN_FRAMES[1]}); the first two {moving[1][0] - moving[0][0]:.1f} ms apart "
        f"(SDL runs the turn after a rate change at once), then {min(gaps):.2f} to {max(gaps):.2f} ms "
        f"(mean {sum(gaps) / len(gaps):.2f})"
    )
    return Check(
        "native", "tween", ok, said, {"frames": frames, "gaps_ms": gaps, "problems": problems(run)}
    )


def rests_ok(rests: list[float], count: int, web: bool) -> tuple[bool, str]:
    """Each tip's deadline after the move that began its rest (REST_MS above): `count` of them,
    each at 400 ms, within NATIVE_REST_SLACK_MS or WEB_REST_SLACK_MS either way."""
    slack = WEB_REST_SLACK_MS if web else NATIVE_REST_SLACK_MS
    low, high = REST_MS - slack, REST_MS + slack
    ok = len(rests) == count and all(low <= r <= high for r in rests)
    began = "the pointer's move onto the box"
    values = ", ".join(f"{r:.1f}" for r in rests) or "none"
    want = f"{low:g} to {high:g}" if count == 1 else f"{count}, each {low:g} to {high:g}"
    noun = "the tip's deadline" if count == 1 else "the deadlines"
    said = f"{noun} {values} ms after {began} (want {want}: the rest is {REST_MS:.0f})"
    return ok, said


def native_rests(run: LiveRun) -> list[float]:
    """Each tip's deadline, in ms after the move that began its rest, from the live run's own
    timeline (REST_MS above): the move's posted time, and the turn that fired the tip, the first
    past half the rest after the turn that took the move (the loop sleeps through a rest: no turn
    between), its start less how late the app says it fired. The k-th fire is the k-th move's: the
    scripts that rest (tools/live/timer.act and idle.act) move onto the boxes, each held past its
    rest, before they move anywhere else."""
    log = turns(run.record)
    moves = [
        float(str(s["posted"]))
        for s in steps(run.record)
        if str(s["what"]).startswith("Pointer_Event")
    ]
    fired = [float(m[1]) for m in run.found(TIMER_LINE)]
    rests: list[float] = []
    for move, late in zip(moves, fired, strict=False):
        took = next((t for t, _ in log if t >= move), None)
        fire = next((t for t, _ in log if took is not None and t > took + REST_MS / 2), None)
        if took is None or fire is None:
            break
        rests.append(fire - late - move)
    return rests


# Before any of the page's scripts: every console line kept with the page's clock as it is written,
# and every pointer move and entry as it is dispatched (capture listeners on the window run ahead
# of SDL's own on the canvas, in the same dispatch), in window.__stamps: the rests' timeline on the
# web (REST_MS above).
MOVE_STAMP = "(the pointer moved)"
STAMPER = """(() => {
  window.__stamps = [];
  for (const name of ["log", "error"]) {
    const write = console[name];
    console[name] = function (...args) {
      window.__stamps.push([performance.now(), args.join(" ")]);
      return write.apply(this, args);
    };
  }
  for (const type of ["pointerover", "pointermove"]) {
    window.addEventListener(type, () => window.__stamps.push([performance.now(), MOVE]), true);
  }
})();""".replace("MOVE", json.dumps(MOVE_STAMP))


def stamps_since(page: Page) -> int:
    return int(page.evaluate("window.__stamps.length"))


def web_rests(page: Page, since: int) -> list[float]:
    """Each tip's deadline on the web, in ms after the move that began its rest, from what was
    stamped since `since` (STAMPER): a fire's stamp less how late it says it came, less the stamp
    of the gesture's first pointer event before the app's line setting the timer."""
    stamps = cast(list[tuple[float, str]], page.evaluate(f"window.__stamps.slice({since})"))
    rests: list[float] = []
    first: float | None = None
    began: float | None = None
    for at, text in stamps:
        if text == MOVE_STAMP:
            first = at if first is None else first
        elif TIMER_SET_LINE.search(text):
            began, first = first, None
        elif (fired := TIMER_LINE.search(text)) and began is not None:
            rests.append(at - float(fired[1]) - began)
            began = first = None
    return rests


def timer_check(
    surface: str, wakes: list[re.Match[str]], fires: list[float], rests: list[float], web: bool
) -> Check:
    """Each rest arms the tip's 400 ms timer: its wake event is posted (an SDL timer, a setTimeout
    on the web), and the next turn of the loop fires it. Natively that turn comes at once; on the
    web it waits for the next animation frame, as React's tip waits for the next paint. Each wake's
    line (WAKE_LINE) gives when it was posted and when its timer was armed to fire, both in ms after
    the deadline: the second is the app's share of the lateness, the rest the timer's. A turn is a
    fire less its wake. `rests` are the deadlines measured from when each rest began (REST_MS). The
    limits and their reasons are with RESTS above."""
    turn_limit = WEB_TURN_MS if web else NATIVE_TURN_MS
    early_limit = WEB_CLOCK_MS if web else 0.0
    posted = [float(m[1]) for m in wakes]
    app_share = [float(m[2]) for m in wakes]
    timer_share = [p - a for p, a in zip(posted, app_share, strict=True)]
    turns = [f - w for w, f in zip(posted, fires, strict=False)]
    rested, rest_words = rests_ok(rests, RESTS, web)
    ok = (
        rested
        and len(posted) == RESTS
        and len(fires) == RESTS
        and min(posted) >= -early_limit
        and min(app_share) >= -early_limit
        and max(app_share) <= APP_SHARE_MS
        and min(timer_share) >= -early_limit
        and max(timer_share) <= WAKE_LATE_MS
        and min(turns) >= 0
        and max(turns) <= turn_limit
    )
    wake_spread, turn_spread = spread(posted), spread(turns)
    app_spread, timer_spread = spread(app_share), spread(timer_share)
    said = (
        f"{rest_words}; {len(posted)} wakes posted {', '.join(f'{x:.1f}' for x in posted)} ms after "
        f"it ({spread_text(wake_spread)}; want {RESTS}, none before its deadline"
        + (f" by more than the clock's {WEB_CLOCK_MS:.0f} ms" if web else "")
        + f"), the app's share {', '.join(f'{x:.1f}' for x in app_share)} (want each within "
        f"{APP_SHARE_MS:.0f}: its timer armed for the time left) and the timer's "
        f"{', '.join(f'{x:.1f}' for x in timer_share)} ({spread_text(timer_spread)}; want each "
        f"within {WAKE_LATE_MS:.0f}: the login's); {len(fires)} fired, each by the turn that "
        "followed its wake"
        + (", at the next animation frame," if web else ",")
        + f" {turn_spread.get('min', math.nan):.1f} to {turn_spread.get('max', math.nan):.1f} ms "
        f"after it (want each within {turn_limit:.0f})"
    )
    return Check(
        surface,
        "timer",
        ok,
        said,
        {
            "rest_ms": rests,
            "wake_ms": posted,
            "app_share_ms": app_share,
            "timer_share_ms": timer_share,
            "fire_ms": fires,
            "turn_ms": turns,
            "wake_spread": wake_spread,
            "app_share_spread": app_spread,
            "timer_share_spread": timer_spread,
            "turn_spread": turn_spread,
            "limits_ms": {
                "rest": REST_MS,
                "rest_slack": WEB_REST_SLACK_MS if web else NATIVE_REST_SLACK_MS,
                "early": early_limit,
                "app_share": APP_SHARE_MS,
                "timer_share": WAKE_LATE_MS,
                "turn": turn_limit,
            },
        },
    )


def spread(values: list[float]) -> dict[str, float]:
    """The distribution of `values`, in ms: how many, the least, the quartiles, the 90th
    percentile, the most and the mean (inclusive quantiles)."""
    if not values:
        return {"n": 0}
    ordered = sorted(values)
    quartiles = statistics.quantiles(ordered, n=4, method="inclusive")
    deciles = statistics.quantiles(ordered, n=10, method="inclusive")
    return {
        "n": len(ordered),
        "min": ordered[0],
        "p25": quartiles[0],
        "median": quartiles[1],
        "p75": quartiles[2],
        "p90": deciles[8],
        "max": ordered[-1],
        "mean": statistics.fmean(ordered),
    }


def spread_text(s: dict[str, float]) -> str:
    if not s.get("n"):
        return "none"
    return (
        f"min {s['min']:.1f}, p25 {s['p25']:.1f}, median {s['median']:.1f}, p75 {s['p75']:.1f}, "
        f"p90 {s['p90']:.1f}, max {s['max']:.1f}"
    )


def histogram(values: list[float], width: float = 5.0) -> str:
    """How many of `values` fall in each `width` ms from 0 (any below 0 counted apart)."""
    early = sum(v < 0 for v in values)
    late = [v for v in values if v >= 0]
    bins = [0] * (math.floor(max(late) / width) + 1 if late else 0)
    for v in late:
        bins[math.floor(v / width)] += 1
    text = [f"{i * width:g}-{(i + 1) * width:g}: {n}" for i, n in enumerate(bins)]
    return ", ".join(([f"before 0: {early}"] if early else []) + text)


def native_timer(out: Path, prefs: Path) -> Check:
    run = live("timer", out, prefs)
    wakes = run.found(WAKE_LINE)
    fires = [float(m[1]) for m in run.found(TIMER_LINE)]
    check = timer_check("native", wakes, fires, native_rests(run), web=False)
    check.ok &= not problems(run)
    check.data["problems"] = problems(run)
    return check


def native_input(out: Path, prefs: Path) -> list[Check]:
    run = live("input", out, prefs)
    texts = [m[1] for m in run.found(TEXT_LINE)]
    closed = [m[1] for m in run.found(FIELD_LINE)]
    pinches = [float(m[1]) for m in run.found(PINCH_LINE)]
    schemes = [m[1] for m in run.found(SCHEME_LINE)]
    focus = [(m[1], int(m[2])) for m in run.found(FOCUS_LINE)]
    opened = [m[1] for m in run.found(OPEN_LINE)]
    bad = problems(run)
    return [
        Check(
            "native",
            "text",
            not bad and texts == ["1", ".", "5", "2", "5"] and closed == ["1.25"],
            f"text facts {texts}, the field closed on {closed}",
            {"texts": texts, "closed": closed, "problems": bad},
        ),
        Check(
            "native",
            "keys",
            not bad and focus == TAB_ORDER and opened == ["A", "B"],
            f"from a's box Tab, Tab, Tab, Shift+Tab, Tab: focus {focus}; the field opened in the "
            f"boxes {opened} (the click, then Enter)",
            {"focus": focus, "opened": opened, "problems": bad},
        ),
        Check(
            "native",
            "pinch",
            not bad and [round(p, 4) for p in pinches] == [1.25, round(math.exp(0.5), 4)],
            f"pinch facts {pinches}: SDL's pinch event 1.25, and a wheel of 50 px up with Ctrl held read as exp(0.5)",
            {"pinches": pinches, "problems": bad},
        ),
        Check(
            "native",
            "scheme",
            not bad and schemes == ["Light", "Dark"],
            f"NSApp's appearance set light then dark: SDL_EVENT_SYSTEM_THEME_CHANGED read as {schemes}",
            {"schemes": schemes, "problems": bad},
        ),
    ]


def native_prefs(out: Path, fresh: Path) -> Check:
    fresh.mkdir(parents=True, exist_ok=True)
    for old in fresh.glob("*"):
        old.unlink()
    first = live("prefs", out / "1", fresh)
    second = live("prefs", out / "2", fresh)
    first_found = [(m[1], m[2]) for m in first.found(PREFS_LINE)]
    second_found = [(m[1], m[2]) for m in second.found(PREFS_LINE)]
    stored = (fresh / "prefs.json").read_text() if (fresh / "prefs.json").exists() else ""
    none_first = any("prefs: none stored" in line for line in first.lines)
    ok = (
        not problems(first)
        and not problems(second)
        and none_first
        and bool(first_found)
        and bool(second_found)
        and second_found[0] == ("read", first_found[-1][1])
        and stored == second_found[-1][1]
    )
    said = (
        f"run 1 found none and wrote {[p for _, p in first_found]}; run 2 read "
        f"{second_found[0][1] if second_found else None} and wrote {[p for k, p in second_found if k == 'written']}; "
        f"{fresh.name}/prefs.json holds {stored}"
    )
    return Check(
        "native", "prefs", ok, said, {"first": first_found, "second": second_found, "file": stored}
    )


# Ctrl-C (SIGINT) and SIGTERM quit the idle native app at once (platform_unix.rs): its own
# handler writes the signal to a pipe whose reader posts SDL's quit event, which wakes the loop's
# wait. With SDL's handler, which only marks the quit until the loop next pumps events, the app
# idled on (M13b's fresh clone). The app is launched plainly (not --live: no script's events to wake
# it), left to idle past its first paint, its CPU read over IDLE_PROBE_S to show it idle, then sent
# the signal: it exits with status 0 within SIGNAL_EXIT_S, logging the signal it quit on.
SIGNAL_EXIT_S = 1.0
IDLE_PROBE_S = 1.0
QUIT_LINE = re.compile(r"quit: (SIGINT|SIGTERM)")


def native_signals(out: Path, prefs: Path) -> Check:
    out.mkdir(parents=True, exist_ok=True)
    found: dict[str, dict[str, object]] = {}
    for sig in (signal.SIGINT, signal.SIGTERM):
        process = subprocess.Popen(
            [str(NATIVE), "--prefs", str(prefs), "--pace", "60", "--trace"],
            cwd=REPO,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
        )
        lines: list[str] = []

        def read(process: subprocess.Popen[str] = process, lines: list[str] = lines) -> None:
            assert process.stdout is not None
            for line in process.stdout:
                lines.append(line.rstrip("\n"))

        reader = threading.Thread(target=read)
        reader.start()
        time.sleep(FIRST_PAINT_MS / 1000 + 1.0)
        cpu_before = cpu_seconds(process.pid)
        time.sleep(IDLE_PROBE_S)
        cpu = cpu_seconds(process.pid) - cpu_before
        state = ps_state(process.pid)
        sent = time.monotonic()
        process.send_signal(sig)
        try:
            code: int | None = process.wait(timeout=10)
            took: float | None = time.monotonic() - sent
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()
            code, took = None, None
        reader.join()
        (out / f"{sig.name}.txt").write_text("\n".join(lines) + "\n")
        quit_on = [m[1] for line in lines if (m := QUIT_LINE.search(line))]
        bad = [line for line in lines if "[ERROR]" in line or "[FATAL]" in line]
        found[sig.name] = {
            "idle_cpu_s": cpu,
            "ps_state": state,
            "exit": code,
            "took_s": took,
            "quit_on": quit_on,
            "problems": bad,
        }
    ok = all(
        f["exit"] == 0
        and f["took_s"] is not None
        and cast(float, f["took_s"]) <= SIGNAL_EXIT_S
        and f["quit_on"] == [name]
        and cast(float, f["idle_cpu_s"]) < 0.02
        and not f["problems"]
        for name, f in found.items()
    )

    def told(name: str, f: dict[str, object]) -> str:
        if f["took_s"] is None:
            return f"{name}: still running 10 s after it (killed)"
        took = cast(float, f["took_s"]) * 1000
        return (
            f"{name}: exit {f['exit']} {took:.0f} ms after it, logged {f['quit_on'] or 'nothing'} "
            f"(idle before: {cast(float, f['idle_cpu_s']):.2f} s of CPU over {IDLE_PROBE_S:.0f} s, "
            f"ps state {f['ps_state']})"
        )

    said = "; ".join(told(name, f) for name, f in found.items())
    return Check("native", "signals", ok, said, {"runs": found, "out": str(out)})


# Web: each engine through Playwright on the real clock. Waits are page.wait_for_timeout, never
# time.sleep: Playwright's sync API hands over console messages only while one of its calls runs.


def web_url() -> str:
    return f"http://localhost:{port('WEB_PORT')}/?trace"


def open_page(page: Page) -> None:
    page.goto(web_url())
    page.wait_for_function("() => window.Module?.started === true && Module.frames >= 1")


def counts(page: Page) -> tuple[int, int]:
    value = page.evaluate("[Module.iterations, Module.frames]")
    return int(value[0]), int(value[1])


def console_after(log: PageLog, start: int, pattern: re.Pattern[str]) -> list[re.Match[str]]:
    return [m for line in log.console[start:] if (m := pattern.search(line))]


def web_checks(engine: str, out: Path) -> list[Check]:
    out.mkdir(parents=True, exist_ok=True)
    surface = f"web-{engine}"
    checks: list[Check] = []
    with sync_playwright() as playwright:
        browser = engine_type(playwright, engine).launch()
        try:
            context = browser.new_context(
                viewport={"width": 1440, "height": 900}, device_scale_factor=2, color_scheme="dark"
            )
            context.add_init_script(STAMPER)
            page = context.new_page()
            log = PageLog()
            log.watch(page)
            open_page(page)
            try:
                checks.append(web_idle(page, surface))
                checks.append(web_tween(page, log, surface, out))
                checks.append(web_timer(page, log, surface))
                checks += web_input(page, log, surface, out)
                checks.append(web_prefs(page, log, surface))
                checks.append(web_page(page, log, surface))
            except Exception as err:
                # The page died mid-engine (a navigation from an owed
                # gesture's keypress, e.g. WebKit's Backspace with no field
                # open): fail the checks that never ran, with the crash as
                # their evidence, instead of aborting the batch and losing
                # the other engines. No check's definition changes; a dead
                # page never passes one.
                have = {c.name for c in checks}
                for name in (
                    "idle", "tween", "timer", "text", "keys", "pinch",
                    "scheme", "motion", "cursor", "prefs", "page",
                ):
                    if name not in have:
                        checks.append(
                            Check(
                                surface, name, False,
                                f"the page died before it ran: {err!s}"[:300],
                                {},
                            )
                        )
            context.close()
        finally:
            browser.close()
    return checks


def web_idle(page: Page, surface: str) -> Check:
    """The main loop pauses after the first paint's last frame and stays paused; an input resumes it
    for one turn; the tip's deadline wakes it once more; then it pauses again."""
    page.wait_for_timeout(FIRST_PAINT_MS + 500)
    rest = counts(page)
    page.wait_for_timeout(2000)
    later = counts(page)
    since = stamps_since(page)
    page.mouse.move(*BOXES[3])
    page.wait_for_timeout(250)
    moved = counts(page)
    page.wait_for_timeout(600)  # the tip, at 400 ms
    tip = counts(page)
    page.wait_for_timeout(2000)
    after = counts(page)
    page.mouse.move(*PLANE)
    page.wait_for_timeout(300)
    rests = web_rests(page, since)
    rested, rest_words = rests_ok(rests, 1, web=True)
    ok = (
        rested
        and rest == later
        and moved[0] == rest[0] + 1
        and tip[0] == moved[0] + 1
        and after == tip
    )
    said = (
        f"[turns, frames] {list(rest)} after the first frame, {list(later)} 2 s later; "
        f"a move {list(moved)}, the tip's wake {list(tip)}, {list(after)} 2 s later; {rest_words}"
    )
    return Check(
        surface, "idle", ok, said, {"counts": [rest, later, moved, tip, after], "rest_ms": rests}
    )


RAF_RECORDER = """() => {
  window.__frames = [];
  const tick = () => { window.__frames.push(Module.frames); if (window.__frames.length < 600) requestAnimationFrame(tick); };
  requestAnimationFrame(tick);
}"""


def web_tween(page: Page, log: PageLog, surface: str, out: Path) -> Check:
    """One frame per animation frame while a flip's jump moves the plane: a recorder reads
    Module.frames on every animation frame, and each animation frame of the jump adds one."""
    start = len(log.console)
    page.evaluate(RAF_RECORDER)
    page.wait_for_timeout(200)
    page.mouse.click(*FLIP_A)
    page.wait_for_timeout(1400)
    page.screenshot(path=out / "after-tween.png")
    found = console_after(log, start, TWEEN_LINE)
    recorded = [int(x) for x in page.evaluate("window.__frames")]
    steps_ = [b - a for a, b in zip(recorded, recorded[1:], strict=False)]
    if not found:
        return Check(surface, "tween", False, "no tween line", {"steps": steps_})
    frames = int(found[0][1])
    # The longest run of animation frames that each added one frame: the jump's, when every
    # animation frame while it moved drew once (its frames and the one at rest).
    run = longest = 0
    for step in steps_:
        run = run + 1 if step == 1 else 0
        longest = max(longest, run)
    others = sorted({s for s in steps_ if s != 1})
    ok = longest >= frames + 1 and all(s in (0, 1) for s in steps_)
    said = (
        f"{frames} frames while it moved over {JUMP_MS} ms; on {longest} animation frames in a row each "
        f"added one frame (want at least {frames + 1}: its frames and the one at rest); the others "
        f"added {others}"
    )
    span = steps_
    return Check(surface, "tween", ok, said, {"frames": frames, "per_animation_frame": span})


def web_timer(page: Page, log: PageLog, surface: str) -> Check:
    start = len(log.console)
    since = stamps_since(page)
    for box in [*BOXES, BOXES[0]]:
        page.mouse.move(*box)
        page.wait_for_timeout(700)
    page.mouse.move(*PLANE)
    page.wait_for_timeout(200)
    wakes = console_after(log, start, WAKE_LINE)
    fires = [float(m[1]) for m in console_after(log, start, TIMER_LINE)]
    return timer_check(surface, wakes, fires, web_rests(page, since), web=True)


def web_input(page: Page, log: PageLog, surface: str, out: Path) -> list[Check]:
    start = len(log.console)
    page.mouse.click(*FIELD)
    page.wait_for_timeout(100)
    page.keyboard.type("1.5")
    page.keyboard.press("Backspace")
    page.keyboard.type("25")
    page.wait_for_timeout(200)
    page.screenshot(path=out / "typed.png")
    cursor_field = canvas_cursor(page)
    for key in ["Enter", "Tab", "Tab", "Tab", "Shift+Tab", "Tab", "Enter"]:
        page.keyboard.press(key)
    page.wait_for_timeout(900)
    page.mouse.move(*PLANE)
    page.wait_for_timeout(100)
    cursor_plane = canvas_cursor(page)
    page.mouse.down()
    page.mouse.move(PLANE[0] + 40, PLANE[1] + 20)
    page.wait_for_timeout(100)
    cursor_drag = canvas_cursor(page)
    page.mouse.up()
    page.mouse.move(*BOXES[0])
    page.wait_for_timeout(100)
    cursor_box = canvas_cursor(page)
    page.mouse.move(*CHIP)
    page.wait_for_timeout(100)
    cursor_chip = canvas_cursor(page)
    page.mouse.move(*PLANE)
    # Playwright's pinch in a browser (tools/src/act.py): Ctrl + wheel of -100 ln(scale) pixels;
    # then a trackpad's, a wheel with ctrlKey and no Control key.
    page.keyboard.down("Control")
    page.mouse.wheel(0, -100 * math.log(1.25))
    page.keyboard.up("Control")
    page.wait_for_timeout(100)
    page.evaluate(
        """([x, y]) => document.getElementById("canvas").dispatchEvent(new WheelEvent("wheel",
            {deltaY: 20, ctrlKey: true, bubbles: true, cancelable: true, clientX: x, clientY: y}))""",
        list(PLANE),
    )
    page.wait_for_timeout(100)
    page.emulate_media(color_scheme="light")
    page.wait_for_timeout(200)
    page.emulate_media(color_scheme="dark")
    page.wait_for_timeout(200)
    page.emulate_media(reduced_motion="reduce")
    page.wait_for_timeout(200)
    reduced_start = len(log.console)
    # Q leaves the plane: under reduced motion only a 120 ms fade.
    page.mouse.move(*CHIP)
    page.wait_for_timeout(400)
    page.emulate_media(reduced_motion="no-preference")
    page.wait_for_timeout(200)
    texts = [m[1] for m in console_after(log, start, TEXT_LINE)]
    closed = [m[1] for m in console_after(log, start, FIELD_LINE)]
    opened = [m[1] for m in console_after(log, start, OPEN_LINE)]
    focus = [(m[1], int(m[2])) for m in console_after(log, start, FOCUS_LINE)]
    pinches = [float(m[1]) for m in console_after(log, start, PINCH_LINE)]
    schemes = [m[1] for m in console_after(log, start, SCHEME_LINE)]
    motion = [
        line.split("reduced motion ")[-1]
        for line in log.console[start:]
        if "reduced motion " in line
    ]
    reduced_tween = console_after(log, reduced_start, TWEEN_LINE)
    cursors: dict[str, object] = {
        "field": cursor_field,
        "plane": cursor_plane,
        "drag": cursor_drag,
        "box": cursor_box,
        "chip": cursor_chip,
    }
    return [
        # b's field, opened by the Enter with its value's text, applies it again as the press on the
        # plane takes the focus.
        Check(
            surface,
            "text",
            texts == ["1", ".", "5", "2", "5"] and closed == ["1.25", "1.00"],
            f"text facts {texts}, the fields closed on {closed}",
            {"texts": texts, "closed": closed},
        ),
        Check(
            surface,
            "keys",
            focus == TAB_ORDER and opened == ["A", "B"],
            f"from a's box Tab, Tab, Tab, Shift+Tab, Tab: focus {focus}; the field opened in the "
            f"boxes {opened} (the click, then Enter)",
            {"focus": focus, "opened": opened},
        ),
        Check(
            surface,
            "pinch",
            len(pinches) == 2 and pinches[0] > 1.05 and abs(pinches[1] - math.exp(-0.2)) < 1e-3,
            f"pinch facts {pinches}: Playwright's Ctrl + wheel for 1.25, and a trackpad's wheel of 20 px "
            "with ctrlKey and no key, exp(-0.2)",
            {"pinches": pinches},
        ),
        Check(
            surface,
            "scheme",
            schemes == ["Light", "Dark"],
            f"emulateMedia light then dark: SDL_EVENT_SYSTEM_THEME_CHANGED read as {schemes}",
            {"schemes": schemes},
        ),
        Check(
            surface,
            "motion",
            motion == ["true", "false"] and bool(reduced_tween) and reduced_tween[0][2] == "120",
            f"emulateMedia reduce, then no-preference: {motion}; Q leaving the plane meanwhile "
            f"{reduced_tween[0][0] if reduced_tween else 'missing'}",
            {"motion": motion},
        ),
        Check(
            surface,
            "cursor",
            cursors
            == {
                "field": "text",
                "plane": "crosshair",
                "drag": "grabbing",
                "box": "ew-resize",
                "chip": "help",
            },
            f"the canvas's computed cursor {cursors}",
            cursors,
        ),
    ]


def canvas_cursor(page: Page) -> str:
    return str(page.evaluate("getComputedStyle(document.getElementById('canvas')).cursor"))


def reload_moving(page: Page) -> None:
    """Reloads while a flip's jump runs, so the main loop is running as the page unloads: SDL's quit
    then runs a turn first, and SDL 3.4.2 throws there when the canvas has a sibling (the wrapper in
    web/index.html; on an idle, paused page the quit never runs and nothing shows)."""
    page.wait_for_timeout(FIRST_PAINT_MS)
    page.mouse.click(*FLIP_A)
    page.wait_for_timeout(100)
    page.reload()
    page.wait_for_function("() => window.Module?.started === true && Module.frames >= 1")


def web_prefs(page: Page, log: PageLog, surface: str) -> Check:
    """The port's key and JSON in localStorage: written on change, read back after a reload. Every
    reload comes while the loop runs, and none may throw."""
    page_errors, console_errors = len(log.page_errors), len(log.console_errors)
    page.evaluate("localStorage.clear()")
    reload_moving(page)
    start = len(log.console)
    page.mouse.click(*GEAR)
    page.wait_for_timeout(PANEL_OPEN_MS)
    page.mouse.click(*MODE_LIGHT)
    page.wait_for_timeout(200)
    page.mouse.click(*MODE_DARK)
    page.wait_for_timeout(200)
    stored = str(page.evaluate("localStorage.getItem('mathviz-function-viewer-rust')"))
    reload_moving(page)
    page.wait_for_timeout(200)
    found = [(m[1], m[2]) for m in console_after(log, start, PREFS_LINE)]
    read = [p for k, p in found if k == "read"]
    reload_moving(page)
    page.wait_for_timeout(500)
    thrown = log.page_errors[page_errors:] + log.console_errors[console_errors:]
    ok = (
        read[:1] == [stored]
        and stored == '{"mode":"dark","dark":"arctic-ice","light":"clean-teal"}'
        and not thrown
    )
    said = (
        f"after a clear and Light then Dark picked in the settings panel localStorage holds {stored}; "
        f"the reload read {read[:1]}; "
        f"3 reloads while the loop ran, {len(thrown)} page or console errors"
    )
    return Check(surface, "prefs", ok, said, {"stored": stored, "found": found, "thrown": thrown})


def web_page(page: Page, log: PageLog, surface: str) -> Check:
    title = str(page.evaluate("document.title"))
    icons = [
        str(x)
        for x in page.evaluate("[...document.querySelectorAll('link[rel~=icon]')].map(l => l.href)")
    ]
    errors = log.problems()
    ok = title == "Function Viewer" and any(i.startswith("blob:") for i in icons) and not errors
    said = f"title {title!r}, favicon {[i[:30] for i in icons]}, {len(errors)} console errors or page errors"
    return Check(surface, "page", ok, said, {"title": title, "icons": icons, "problems": errors})
