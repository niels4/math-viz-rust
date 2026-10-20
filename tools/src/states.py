"""The judgment states (plan section 6): the scripts in tools/states run on every surface, each
shot compared between surfaces at threshold 0 and laid out on a contact sheet. A milestone's
states are listed in tools/states/milestones.toml; the judgment is a person reading the
images."""

import json
import re
import time
import tomllib
from concurrent.futures import Future, ProcessPoolExecutor, as_completed
from dataclasses import dataclass
from pathlib import Path
from typing import cast

from act import REPO, Run, run, shown
from images import Difference, contact_sheet, difference, load_rgb
from script import DARK_THEMES, THEMES, Script, load

STATES = REPO / "tools" / "states"
# How many (state, surface) runs go at once without --jobs: the smallest level within 10 %
# of the best wall time with zero differences, measured on 24 cores / 128 GB in
# .local/briefs/progress/t1-jobs.md (H1 H2, native and web-webkit: 1 job 80 s, 8 jobs 50 s,
# 24 jobs 50 s; 8 wins the tie).
DEFAULT_JOBS = 8
# The scripts of states in another theme (state_script), where the native build can read them.
VARIANTS = REPO / ".local" / "states-variants"
# The lines that say how a page opens, before a script's first action.
START_WORDS = (
    "size",
    "density",
    "scheme",
    "mode",
    "theme",
    "reduced-motion",
    "space",
    "query",
    "route",
)


def state_script(name: str) -> Script:
    """A state's script. `STATE@THEME` is the state shown in another theme: its opening `scheme`,
    `mode` and `theme` lines set to the theme's (a dark theme on a dark system in Dark, a light one
    on a light system in Light), the rest as written, kept in VARIANTS for the native build."""
    if "@" not in name:
        return load(STATES / f"{name}.act")
    base, slug = name.split("@", 1)
    if slug not in THEMES:
        raise SystemExit(f"no theme {slug}: {', '.join(THEMES)}")
    scheme = "dark" if slug in DARK_THEMES else "light"
    out: list[str] = []
    opening = True
    for line in (STATES / f"{base}.act").read_text().splitlines():
        word = line.split()[0] if line.strip() and not line.lstrip().startswith("#") else ""
        if opening and word and word not in START_WORDS:
            out += [f"scheme {scheme}", f"mode {scheme}", f"theme {slug}"]
            opening = False
        if opening and word in ("scheme", "mode", "theme"):
            continue
        out.append(line)
    VARIANTS.mkdir(parents=True, exist_ok=True)
    path = VARIANTS / f"{name}.act"
    path.write_text("\n".join(out) + "\n")
    return load(path)


def state_exists(name: str) -> bool:
    """A script in tools/states, or one in another theme (STATE@THEME)."""
    base, _, slug = name.partition("@")
    return (STATES / f"{base}.act").exists() and (not slug or slug in THEMES)


def is_milestone(name: str) -> bool:
    """A milestone's name, as milestones.toml keys them: H0, H1."""
    return re.fullmatch(r"[MmHh]\d+[a-z]?", name) is not None


def milestone_states(milestone: str) -> list[str]:
    manifest = tomllib.loads((STATES / "milestones.toml").read_text())
    listed = {key.upper(): value for key, value in manifest.items()}.get(milestone.upper())
    if not isinstance(listed, list):
        raise SystemExit(f"tools/states/milestones.toml lists no states for {milestone}")
    return [name for name in cast(list[object], listed) if isinstance(name, str)]


def all_states() -> list[str]:
    """Every milestone's states, each once, in the order milestones.toml lists them."""
    manifest = tomllib.loads((STATES / "milestones.toml").read_text())
    names: list[str] = []
    for listed in manifest.values():
        if isinstance(listed, list):
            names += [name for name in cast(list[object], listed) if isinstance(name, str)]
    return list(dict.fromkeys(names))


@dataclass
class Comparison:
    shot: str
    a: str
    b: str
    result: Difference
    image: Path | None  # the changed pixels, when there are any


def surfaces(targets: list[str], engines: dict[str, list[str]]) -> list[tuple[str, str]]:
    return [(target, engine) for target in targets for engine in engines.get(target, [""])]


def compare(runs: list[Run], shots: list[str], out: Path) -> list[Comparison]:
    """Each shot at threshold 0: the port's two builds against each other (native against each
    browser of the web build), and the web build against the reference in the same engine. The
    report holds each pair's threshold, changed pixels, max and bounds: the identity columns."""
    by_name = {r.name: r for r in runs}
    pairs: list[tuple[str, str]] = []
    if "native" in by_name:
        pairs += [("native", r.name) for r in runs if r.target == "web"]
    pairs += [
        (r.name, f"ref-{r.engine}")
        for r in runs
        if r.target == "web" and f"ref-{r.engine}" in by_name
    ]
    comparisons: list[Comparison] = []
    (out / "diffs").mkdir(parents=True, exist_ok=True)
    for shot in shots:
        for a, b in pairs:
            path_a, path_b = by_name[a].out / f"{shot}.png", by_name[b].out / f"{shot}.png"
            if not (path_a.exists() and path_b.exists()):
                continue
            image = out / "diffs" / f"{shot}-{a}-vs-{b}.png"
            result = difference(load_rgb(path_a), load_rgb(path_b), 0)
            if result.same:
                image.unlink(missing_ok=True)
                comparisons.append(Comparison(shot, a, b, result, None))
            else:
                difference(load_rgb(path_a), load_rgb(path_b), 0, out=image)
                comparisons.append(Comparison(shot, a, b, result, image))
    return comparisons


def _run_surface(target: str, engine: str, state_dir: str, script: Script) -> Run:
    """One (state, surface) run in a worker process: the script on the surface into
    <state_dir>/<surface>, as run_states does serially. macOS spawns children that re-import
    this module, so the job function lives here (as sweep's _one does). The parent preloads
    every script, and with it every VARIANTS file, before the pool starts, so workers only
    read; each worker's browser and native runs are its own process's (act.run launches a
    browser per call and a native binary per script into the run's own directory)."""
    surface = f"{target}-{engine}" if engine else target
    return run(script, target, engine, Path(state_dir) / surface)


def _finish_state(
    name: str, script: Script, runs: list[Run], state_dir: Path
) -> tuple[list[str], dict[str, object], bool]:
    """A state's printed block, its report entry, and whether it failed: the comparisons and
    contact sheet over its surfaces' runs, whose order is the surfaces() order."""
    lines = [f"{name} ({script.path.relative_to(REPO)}):"]
    failed = False
    for result in runs:
        lines.append(f"  {result.summary()}")
        for problem in result.problems:
            lines.append(f"    {problem}")
        failed |= not result.ok
    shots = script.shot_names()
    comparisons = compare(runs, shots, state_dir)
    for c in comparisons:
        verdict = "same" if c.result.same else c.result.summary()
        lines.append(f"  {c.shot}: {c.a} vs {c.b}: {verdict}")
    if shots:
        rows = [
            (
                shot,
                [(r.name, p if (p := r.out / f"{shot}.png").exists() else None) for r in runs],
            )
            for shot in shots
        ]
        contact_sheet(rows, state_dir / "contact.png")
        lines.append(f"  contact sheet: {shown(state_dir / 'contact.png')}")
    entry: dict[str, object] = {
        "script": str(script.path.relative_to(REPO)),
        "runs": [
            {"surface": r.name, "ok": r.ok, "problems": r.problems, "out": str(r.out)} for r in runs
        ],
        "comparisons": [
            {
                "shot": c.shot,
                "a": c.a,
                "b": c.b,
                "threshold": c.result.threshold,
                "changed": c.result.changed,
                "max": c.result.max,
                "bounds": c.result.bounds,
                "image": str(c.image) if c.image else None,
            }
            for c in comparisons
        ],
        "videos": [str(r.out / f"{v}.webm") for r in runs for v in r.videos],
    }
    return lines, entry, failed


def _write_report(out: Path, names: list[str], finished: dict[str, dict[str, object]]) -> None:
    """report.json in milestones.toml order, over the states finished so far: a run killed
    midway leaves a readable partial report."""
    out.mkdir(parents=True, exist_ok=True)
    report = {name: finished[name] for name in names if name in finished}
    (out / "report.json").write_text(json.dumps(report, indent=2))


def _run_serial(
    names: list[str], scripts: dict[str, Script], order: list[tuple[str, str]], out: Path
) -> int:
    finished: dict[str, dict[str, object]] = {}
    failed = False
    for name in names:
        script, state_dir = scripts[name], out / name
        runs = [
            run(script, target, engine, state_dir / (f"{target}-{engine}" if engine else target))
            for target, engine in order
        ]
        lines, finished[name], state_failed = _finish_state(name, script, runs, state_dir)
        print("\n".join(lines))
        failed |= state_failed
    _write_report(out, names, finished)
    print(f"report: {shown(out / 'report.json')}")
    return 1 if failed else 0


def _run_parallel(
    names: list[str], scripts: dict[str, Script], order: list[tuple[str, str]], out: Path, jobs: int
) -> int:
    index = {
        f"{target}-{engine}" if engine else target: i for i, (target, engine) in enumerate(order)
    }
    finished: dict[str, dict[str, object]] = {}
    failed = False
    finish_seconds = 0.0
    print(f"states: {len(names)} states x {len(order)} surfaces, {jobs} jobs")
    with ProcessPoolExecutor(max_workers=jobs) as pool:
        future_of: dict[Future[Run], str] = {}
        for name in names:
            for target, engine in order:
                future_of[
                    pool.submit(_run_surface, target, engine, str(out / name), scripts[name])
                ] = name
        collected: dict[str, list[Run]] = {name: [] for name in names}
        for future in as_completed(future_of):
            name = future_of[future]
            collected[name].append(future.result())
            if len(collected[name]) == len(order):
                runs = sorted(collected[name], key=lambda r: index[r.name])
                began = time.monotonic()
                lines, finished[name], state_failed = _finish_state(
                    name, scripts[name], runs, out / name
                )
                finish_seconds += time.monotonic() - began
                print("\n".join(lines))
                failed |= state_failed
                _write_report(out, names, finished)
    _write_report(out, names, finished)
    print(f"report: {shown(out / 'report.json')}")
    print(
        f"finish (compare and contact sheets in the parent): {finish_seconds:.1f} s total, "
        f"{finish_seconds / max(len(names), 1):.2f} s a state"
    )
    return 1 if failed else 0


def run_states(
    names: list[str], targets: list[str], engines: dict[str, list[str]], out: Path, jobs: int = 1
) -> int:
    scripts = {name: state_script(name) for name in names}
    order = surfaces(targets, engines)
    if jobs == 1:
        return _run_serial(names, scripts, order, out)
    return _run_parallel(names, scripts, order, out, jobs)
