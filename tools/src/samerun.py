"""Shot-for-shot equality of two states runs (tools/run same-run): every state in both
reports, every comparison's changed/max/bounds, every surface run's ok and problems, and
every shot PNG the two runs share identical at threshold 0. A state missing from one
report is a difference; a PNG that differs is a flake to diagnose, never to average away
(the par-states milestone: rerun that state alone with --jobs 1, and see which run the
reshoot matches). React's own render wobbles run to run (i5's run-1-vs-run-2: 122
reference-column pairs), so a difference in a `vs ref-*` pair alone says the reference
moved, not the port."""

import json
from pathlib import Path
from typing import Any, cast

from images import difference, load_rgb


def _report(out: Path) -> dict[str, Any]:
    path = out / "report.json"
    if not path.exists():
        raise SystemExit(f"no report in {out}: run states into it first")
    data = json.loads(path.read_text())
    if not isinstance(data, dict):
        raise SystemExit(f"{path} holds no state report")
    return cast(dict[str, Any], data)


def _entry(report: dict[str, Any], state: str) -> dict[str, Any]:
    entry = report.get(state)
    if not isinstance(entry, dict):
        raise SystemExit(f"report holds no object for {state}")
    return cast(dict[str, Any], entry)


def _runs(entry: dict[str, Any]) -> dict[str, dict[str, Any]]:
    """The state's surface runs by surface name."""
    runs = entry.get("runs")
    if not isinstance(runs, list):
        raise SystemExit("report entry holds no runs list")
    by_surface: dict[str, dict[str, Any]] = {}
    items = cast(list[Any], runs)
    for raw in items:
        if not isinstance(raw, dict):
            raise SystemExit("report entry holds a run that is no object")
        item = cast(dict[str, Any], raw)
        surface = item.get("surface")
        if not isinstance(surface, str):
            raise SystemExit("report entry holds a run with no surface")
        by_surface[surface] = item
    return by_surface


def _pairs(entry: dict[str, Any]) -> dict[tuple[str, str, str], dict[str, Any]]:
    """The state's comparisons by (shot, a, b)."""
    comparisons = entry.get("comparisons")
    if not isinstance(comparisons, list):
        raise SystemExit("report entry holds no comparisons list")
    by_key: dict[tuple[str, str, str], dict[str, Any]] = {}
    found = cast(list[Any], comparisons)
    for raw in found:
        if not isinstance(raw, dict):
            raise SystemExit("report entry holds a comparison that is no object")
        item = cast(dict[str, Any], raw)
        key = (item.get("shot"), item.get("a"), item.get("b"))
        if not all(isinstance(part, str) for part in key):
            raise SystemExit("report entry holds a comparison with no shot/a/b")
        by_key[(str(key[0]), str(key[1]), str(key[2]))] = item
    return by_key


def _shots(state_dir: Path) -> dict[str, dict[str, Path]]:
    """Every shot PNG by surface and shot name, leaving out the diffs and contact sheet."""
    found: dict[str, dict[str, Path]] = {}
    if not state_dir.is_dir():
        return found
    for surface_dir in sorted(state_dir.iterdir()):
        if not surface_dir.is_dir() or surface_dir.name == "diffs":
            continue
        shots = {
            path.stem: path
            for path in sorted(surface_dir.glob("*.png"))
            if path.name != "contact.png"
        }
        if shots:
            found[surface_dir.name] = shots
    return found


def same_run(dir_a: Path, dir_b: Path, names: list[str]) -> int:
    """Compares two states runs shot for shot; exit 1 on any difference, with a per-state
    line for each one and a summary."""
    report_a, report_b = _report(dir_a), _report(dir_b)
    states = names or sorted(set(report_a) | set(report_b))
    differences = 0
    for state in states:
        lines: list[str] = []
        if state not in report_a:
            lines.append(f"only in {dir_b}")
        elif state not in report_b:
            lines.append(f"only in {dir_a}")
        else:
            runs_a = _runs(_entry(report_a, state))
            runs_b = _runs(_entry(report_b, state))
            for surface in sorted(set(runs_a) | set(runs_b)):
                if surface not in runs_a:
                    lines.append(f"{surface}: only in {dir_b}")
                elif surface not in runs_b:
                    lines.append(f"{surface}: only in {dir_a}")
                else:
                    a, b = runs_a[surface], runs_b[surface]
                    if a["ok"] != b["ok"]:
                        lines.append(f"{surface}: ok {a['ok']} vs {b['ok']}")
                    if a["problems"] != b["problems"]:
                        lines.append(f"{surface}: problems {a['problems']} vs {b['problems']}")
            pairs_a = _pairs(_entry(report_a, state))
            pairs_b = _pairs(_entry(report_b, state))
            for key in sorted(set(pairs_a) | set(pairs_b)):
                if key not in pairs_a:
                    lines.append(f"{key[0]}: {key[1]} vs {key[2]} only in {dir_b}")
                elif key not in pairs_b:
                    lines.append(f"{key[0]}: {key[1]} vs {key[2]} only in {dir_a}")
                else:
                    a, b = pairs_a[key], pairs_b[key]
                    for column in ("changed", "max", "bounds"):
                        if a[column] != b[column]:
                            lines.append(
                                f"{key[0]}: {key[1]} vs {key[2]} {column} "
                                f"{a[column]} vs {b[column]}"
                            )
            shots_a, shots_b = _shots(dir_a / state), _shots(dir_b / state)
            for surface in sorted(set(shots_a) | set(shots_b)):
                for shot in sorted(set(shots_a.get(surface, {})) | set(shots_b.get(surface, {}))):
                    path_a = shots_a.get(surface, {}).get(shot)
                    path_b = shots_b.get(surface, {}).get(shot)
                    if path_a is None:
                        lines.append(f"{surface}/{shot}.png: only in {dir_b}")
                    elif path_b is None:
                        lines.append(f"{surface}/{shot}.png: only in {dir_a}")
                    else:
                        result = difference(load_rgb(path_a), load_rgb(path_b), 0)
                        if not result.same:
                            lines.append(
                                f"{surface}/{shot}.png: differs "
                                f"(changed {result.changed}, max {result.max})"
                            )
        if lines:
            differences += len(lines)
            print(f"{state}:")
            for line in lines:
                print(f"  {line}")
    print(f"same-run: {len(states)} states, {differences} differences")
    return 1 if differences else 0
