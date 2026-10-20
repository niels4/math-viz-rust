"""Motion at every phase (tools/run sweep): each state run with its inputs shifted 0 to 15 ms off the
16 ms grid, natively on the harness's clock and in a browser at real speed, to see that the app
survives every frame time a motion can land on. The paused clock draws a state's frames on the grid
at every run, so its motion lands on the same times each run; a frame between them can still
crash the app (M10's review: a dash or a draw-on piece a hair long, which NanoVG flattened to one
point and indexed before its start). A swept script loses its shots and video (the frames a
`frames` line would shoot are waited instead) and gains a `wait k` after its first wait; in a
browser it runs on the real clock. A run that exits non-zero, panics, traps or logs an error is a
crash, listed with its state and k. Nothing is compared: the states' own runs judge the frames."""

import json
import re
import time
from concurrent.futures import ProcessPoolExecutor
from dataclasses import asdict, dataclass
from pathlib import Path

from act import REPO, Run, run, shown
from script import Script, load
from states import STATES

# What a crash leaves in a run's console: the Odin oracle's bounds check or panic (natively on
# stderr, on the web through the console), Rust's panic texts (`panicked at src/main.rs:3:21:`,
# `index out of bounds: the len is 3 but the index is 9`, `attempt to ...`, measured from a
# scratch cargo panic; `panic` covers `panicked at`, `unreachable` covers `unreachable
# executed`) or the wasm trap (`RuntimeError`). Any problem a run reports counts too (a native
# exit status, a page error, a console error). Re-sync with t0a's recorded panic texts at its
# merge (plan section 5).
PANIC = re.compile(
    r"out of range|panic|index out of bounds|attempt to|RuntimeError|unreachable", re.IGNORECASE
)
FRAMES = re.compile(r"^frames\s+\S+\s+every\s+(\d+)\s+for\s+(\d+)\s*$")


@dataclass
class _RealTime(Script):
    """A script run on the real clock in a browser, as one with a video section is."""

    @property
    def real_time(self) -> bool:
        return True


@dataclass
class Swept:
    state: str
    k: int
    surface: str
    crashed: bool
    said: str  # the first line that told of the crash, or the run's summary
    seconds: float


def shifted(text: str, k: int) -> str:
    """The script with its inputs k ms later than the first wait puts them, its shots and video
    gone and each `frames` line waited through to the last frame it would shoot."""
    lines: list[str] = []
    delayed = False
    for line in text.splitlines():
        body = line.strip()
        word = body.split()[0] if body else ""
        if word in ("shot", "video"):
            continue
        if m := FRAMES.match(body):
            every, span = int(m[1]), int(m[2])
            lines.append(f"wait {span // every * every}")
            continue
        lines.append(line)
        if word == "wait" and not delayed:
            lines.append(f"wait {k}")
            delayed = True
    return "\n".join(lines) + "\n"


def _one(state: str, k: int, target: str, engine: str, out: Path) -> Swept:
    path = out / "scripts" / f"{state}-{k:02d}.act"
    script = load(path)
    surface = f"{target}-{engine}" if engine else target
    if target == "web":
        script = _RealTime(script.path, script.start, script.actions)
    began = time.monotonic()
    result: Run = run(script, target, engine, out / "runs" / surface / f"{state}-{k:02d}")
    told = next((line for line in result.console if PANIC.search(line)), None)
    told = told or next(iter(result.problems), None)
    crashed = told is not None or not result.ok
    said = told or result.summary()
    return Swept(state, k, surface, crashed, said.strip(), time.monotonic() - began)


def run_sweep(
    names: list[str],
    offsets: list[int],
    targets: list[str],
    engines: list[str],
    jobs: int,
    out: Path,
) -> int:
    (out / "scripts").mkdir(parents=True, exist_ok=True)
    for state in names:
        text = (STATES / f"{state}.act").read_text()
        for k in offsets:
            (out / "scripts" / f"{state}-{k:02d}.act").write_text(shifted(text, k))
    work = [
        (state, k, target, engine)
        for target in targets
        for engine in (engines if target == "web" else [""])
        for state in names
        for k in offsets
    ]
    print(
        f"sweep: {len(names)} states x {len(offsets)} offsets on {', '.join(targets)}: {len(work)} runs"
    )
    results: list[Swept] = []
    with ProcessPoolExecutor(max_workers=jobs) as pool:
        futures = [
            pool.submit(_one, state, k, target, engine, out) for state, k, target, engine in work
        ]
        for future in futures:
            swept = future.result()
            results.append(swept)
            if swept.crashed:
                print(f"  CRASH {swept.surface} {swept.state} k={swept.k}: {swept.said[-160:]}")
    failed = False
    for surface in sorted({r.surface for r in results}):
        mine = [r for r in results if r.surface == surface]
        crashes = [r for r in mine if r.crashed]
        failed |= bool(crashes)
        where = ", ".join(f"{r.state} k={r.k}" for r in crashes)
        print(f"{surface}: {len(mine)} runs, {len(crashes)} crashes{': ' + where if where else ''}")
    (out / "sweep.json").write_text(json.dumps([asdict(r) for r in results], indent=1))
    print(f"report: {shown(out / 'sweep.json')}")
    return 1 if failed else 0


def offsets_arg(text: str) -> list[int]:
    """0-15, or a comma list."""
    if "-" in text:
        lo, hi = (int(v) for v in text.split("-"))
        return list(range(lo, hi + 1))
    return [int(v) for v in text.split(",") if v]


DEFAULT_OUT = REPO / ".local" / "shots" / "sweep"
