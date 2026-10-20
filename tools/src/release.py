"""The release build (./build.sh release) through the harness: a state's frames from build/release/
beside the check build's from build/, natively and in each engine. The release changes how the code
is compiled (optimised, source locations by file name, the web's typeid workaround), not what it
draws, so every pair must be the same at threshold 0; and the release's web build must load and run
with no console error, page error or failed request. Both builds' web/ reach the browser through
request routing (act.ROUTED_ORIGIN), so no server is needed and a clone tests its own files. M13b
checks it; M13a reruns it once the last features are in (plan 5.1.5 (6))."""

import json
from pathlib import Path

from act import REPO, Run, run, shown
from images import difference, load_rgb
from script import load
from states import STATES

BUILDS = {"check": REPO / "build", "release": REPO / "build" / "release"}


def run_release(names: list[str], engines: list[str], out: Path) -> int:
    report: dict[str, object] = {}
    failed = False
    for name in names:
        script = load(STATES / f"{name}.act")
        print(f"{name} ({shown(script.path)}):")
        pairs: list[dict[str, object]] = []
        for target, engine in [("native", ""), *(("web", e) for e in engines)]:
            surface = f"{target}-{engine}" if engine else target
            runs: dict[str, Run] = {}
            for label, build in BUILDS.items():
                result = run(script, target, engine, out / name / surface / label, build=build)
                runs[label] = result
                for problem in result.problems:
                    print(f"  {surface} {label}: {problem}")
                failed |= not result.ok
            check, release = runs["check"], runs["release"]
            for shot in script.shot_names():
                a, b = check.out / f"{shot}.png", release.out / f"{shot}.png"
                if not (a.exists() and b.exists()):
                    failed = True
                    print(
                        f"  {surface} {shot}: missing from {'check' if not a.exists() else 'release'}"
                    )
                    continue
                diffs = out / name / "diffs"
                diffs.mkdir(parents=True, exist_ok=True)
                image = diffs / f"{surface}-{shot}.png"
                result = difference(load_rgb(a), load_rgb(b), 0, out=image)
                failed |= not result.same
                verdict = "the same" if result.same else result.summary()
                print(f"  {surface} {shot}: release against check: {verdict}")
                pairs.append(
                    {
                        "surface": surface,
                        "shot": shot,
                        "same": result.same,
                        "threshold": result.threshold,
                        "changed": result.changed,
                        "max": result.max,
                        "check_ok": check.ok,
                        "release_ok": release.ok,
                        "release_problems": release.problems,
                        "release_gl": release.detail.get("gl"),
                        "image": str(image),
                    }
                )
        report[name] = pairs
    out.mkdir(parents=True, exist_ok=True)
    (out / "report.json").write_text(json.dumps(report, indent=2))
    print(f"report: {shown(out / 'report.json')}")
    return 1 if failed else 0
