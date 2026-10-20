"""Host tooling for math-viz2-rust: the harness that runs one script (tools/states) on the
math-viz2 reference, the web build and the native build, image differences, and font conversion.
Run through ../run; `./run --help` lists the commands."""

import argparse
import json
import sys
from pathlib import Path

from act import REPO, TARGETS, is_native, run, shown
from browser import ENGINES
from dom import FIXTURE, run_dom, run_fixture
from fonts import OUT as FONTS
from fonts import instance, page_faces
from gamut import run_gamut
from images import Box, changed_mask, count_set, difference, load_rgb, scaled_box
from judge import run_judge
from keys import OUT as KEYS_OUT
from keys import run_keys
from loop import run_loop
from raf import OUT as RAF_OUT
from raf import run_raf
from release import run_release
from samerun import same_run
from script import ScriptError, load
from shift import run_shift
from states import (
    DEFAULT_JOBS,
    STATES,
    all_states,
    is_milestone,
    milestone_states,
    run_states,
    state_exists,
)
from sweep import DEFAULT_OUT as SWEEP_OUT
from sweep import offsets_arg, run_sweep
from tabs import OUT as TABS_OUT
from tabs import run_tabs
from themes import generate as generate_themes
from themes import react_ref
from tokens import run_tokens
from view import run_view
from widths import OUT as WIDTHS
from widths import run_widths

SHOTS = REPO / ".local" / "shots"


def engines_list(text: str) -> list[str]:
    engines = [e for e in text.split(",") if e]
    if unknown := [e for e in engines if e not in ENGINES]:
        raise argparse.ArgumentTypeError(
            f"unknown engine {', '.join(unknown)}: {', '.join(ENGINES)}"
        )
    return engines


def box_arg(text: str) -> Box:
    x, y, w, h = (int(v) for v in text.split(","))
    return x, y, w, h


def act(args: argparse.Namespace) -> int:
    """Run a script on one surface: in each engine for ref and web."""
    try:
        script = load(args.script)
    except ScriptError as error:
        print(error)
        return 1
    out: Path = (args.out or SHOTS / "act" / args.script.stem).resolve()
    engines = args.engines if not is_native(args.target) else [""]
    failed = False
    for engine in engines:
        result = run(
            script,
            args.target,
            engine,
            out / (f"{args.target}-{engine}" if engine else args.target),
        )
        print(result.summary())
        for problem in result.problems:
            print(f"  {problem}")
        print(f"  {shown(result.out)}")
        failed |= not result.ok
    return 1 if failed else 0


def state_names(given: list[str]) -> list[str]:
    """States named, a milestone's (tools/states/milestones.toml), or `all` (every milestone's,
    each once); exits on one that's missing."""
    names: list[str] = []
    for name in given:
        if name == "all":
            names += all_states()
        else:
            names += milestone_states(name) if is_milestone(name) else [name]
    names = list(dict.fromkeys(names))
    if missing := [n for n in names if not state_exists(n)]:
        sys.exit(f"no such state in tools/states: {', '.join(missing)}")
    return names


def states(args: argparse.Namespace) -> int:
    names = state_names(args.names)
    first = args.names[0]
    named = is_milestone(first) or first == "all"
    out: Path = (args.out or SHOTS / (first.lower() if named else "states")).resolve()
    targets = [t for t in args.targets.split(",") if t]
    engines = {"ref": args.ref_engines, "web": args.engines}
    if args.jobs < 1:
        sys.exit("--jobs is at least 1")
    return run_states(names, targets, engines, out, args.jobs)


def judge(args: argparse.Namespace) -> int:
    """The judgment's numbers for a states run (tools/src/judge.py): each native shot against the
    reference in each engine, the flat pixels apart and where they fall; the port's surfaces
    against each other."""
    names = state_names(args.names) if args.names else []
    return run_judge(args.dir.resolve(), names, args.jobs)


def dom(args: argparse.Namespace) -> int:
    """The reference's own layout at each shot of a state's script: every element's box, its text
    runs' boxes and baselines, its computed style (tools/src/dom.py); with --fixture, the named
    measures the port's tests read, in crates/function-viewer/testdata/page.json."""
    if args.fixture:
        return run_fixture((args.out or FIXTURE).resolve(), REPO / ".local" / "dom" / "fixture")
    if args.state is None:
        sys.exit("dom needs a state, or --fixture")
    path: Path = args.state if args.state.suffix == ".act" else STATES / f"{args.state}.act"
    if not path.exists():
        sys.exit(f"no such state: {path}")
    out: Path = (args.out or REPO / ".local" / "dom" / path.stem).resolve()
    return run_dom(path, args.engines, out)


def view(args: argparse.Namespace) -> int:
    """The plane's view at each shot of a state on the reference in each engine and natively
    (tools/src/view.py): the zoom and where the origin lands, the port's beside the reference's."""
    names = state_names(args.names)
    return run_view(names, args.engines, (args.out or SHOTS / "view").resolve())


def gamut(args: argparse.Namespace) -> int:
    """The port's colour in Display P3 (plan 1.10): the states drawn natively in sRGB and P3 and
    by the web build in P3 in each engine, each P3 shot held to the conversion of the sRGB one."""
    names = state_names(args.names)
    first = args.names[0]
    default = f"{first.lower()}-gamut" if is_milestone(first) else "gamut"
    return run_gamut(names, args.engines, (args.out or SHOTS / default).resolve())


# The hover check's row: row a of the Transform card, (49, 402.5, 394, 44) at 1440 x 900, held in
# whole px (tools/states/hover-row-1.act moves onto its name). Lit, it lights a's partners too
# (FV 04): its term in the equation's form line (a's at (101, 288.8, 16, 24) in the reference) and,
# since M6, a's meaning on the plane: the unit box from the origin (956, 488) to the unit point
# (1006, 438) at 50 px a unit, dashed, with its measured side's ticks and arrow out to x 1012.5.
HOVER_STATE = "hover-row-1"
HOVER_ROW: Box = (49, 402, 394, 45)
HOVER_TERM: Box = (101, 288, 16, 25)
HOVER_UNIT_BOX: Box = (954, 435, 61, 55)


def web(args: argparse.Namespace) -> int:
    """The web build's smoke check: the hover state's script in each engine, with no console
    error, page error or failed request, and the hover changing the row, its term in the equation
    and a's unit box on the plane, and nothing else."""
    script = load(STATES / f"{HOVER_STATE}.act")
    out: Path = args.out_dir.resolve()
    failed = False
    report: dict[str, object] = {}
    for engine in args.engines:
        result = run(script, "web", engine, out / f"web-{engine}")
        hover: dict[str, object] = {}
        answered = False
        if result.ok:
            row = scaled_box(HOVER_ROW, script.start.density)
            term = scaled_box(HOVER_TERM, script.start.density)
            unit_box = scaled_box(HOVER_UNIT_BOX, script.start.density)
            rest, lit = load_rgb(result.out / "rest.png"), load_rgb(result.out / "hover.png")
            change = difference(rest, lit, 0)
            mask = changed_mask(rest, lit, 0)
            in_row, in_term, in_plane = (
                count_set(mask.crop(_corners(b))) for b in (row, term, unit_box)
            )
            outside = mask.copy()
            for b in (row, term, unit_box):
                outside.paste(0, _corners(b))
            elsewhere = count_set(outside)
            row_pixels = row[2] * row[3]
            answered = elsewhere == 0 and in_row > 0 and in_term > 0 and in_plane > 0
            hover = {
                "row": HOVER_ROW,
                "term": HOVER_TERM,
                "threshold": change.threshold,
                "changed_in_row": in_row,
                "share_of_row": in_row / row_pixels,
                "changed_in_term": in_term,
                "unit_box": HOVER_UNIT_BOX,
                "changed_in_unit_box": in_plane,
                "changed_elsewhere": elsewhere,
                "max": change.max,
                "answered": answered,
            }
            print(
                f"{result.summary()}; hover at threshold 0: {in_row} pixels in the row "
                f"({in_row / row_pixels * 100:.1f}%), {in_term} in its term, {in_plane} in a's unit "
                f"box on the plane, {elsewhere} elsewhere"
            )
        else:
            print(result.summary())
        for problem in result.problems:
            print(f"  {problem}")
        failed |= not (result.ok and answered)
        report[engine] = {
            "ok": result.ok and answered,
            "out": str(result.out),
            "gl": result.detail.get("gl"),
            "hover": hover,
            "problems": result.problems,
            "console": result.console,
        }
    out.mkdir(parents=True, exist_ok=True)
    (out / "web-report.json").write_text(json.dumps(report, indent=2))
    return 1 if failed else 0


def _corners(box: Box) -> tuple[int, int, int, int]:
    """A box as PIL takes it: left, top, right, bottom."""
    x, y, w, h = box
    return x, y, x + w, y + h


def diff(args: argparse.Namespace) -> int:
    """Compare two images, or every PNG two directories share. With --same, fail unless every
    pair is identical; with --inside, fail if a pixel changed outside the box."""
    a: Path = args.a
    b: Path = args.b
    if a.is_dir() and b.is_dir():
        names_a = {p.relative_to(a) for p in a.rglob("*.png") if "diffs" not in p.parts}
        names_b = {p.relative_to(b) for p in b.rglob("*.png") if "diffs" not in p.parts}
        pairs = [(a / n, b / n, str(n)) for n in sorted(names_a & names_b)]
        only = sorted(names_a ^ names_b)
    else:
        pairs = [(a, b, f"{a.name} vs {b.name}")]
        only = []
    box = scaled_box(args.inside, args.density) if args.inside else None
    failed = False
    differing = 0
    for path_a, path_b, label in pairs:
        result = difference(load_rgb(path_a), load_rgb(path_b), args.threshold, box, args.out)
        if args.json:
            print(json.dumps({"pair": label, **result.__dict__}))
        elif len(pairs) == 1 or not result.same:
            print(f"{label}: {result.summary()}")
        differing += not result.same
        failed |= args.same and not result.same
        failed |= box is not None and (result.outside or 0) > 0
    if len(pairs) > 1:
        print(
            f"{len(pairs)} pairs at threshold {args.threshold}/255: {len(pairs) - differing} same, {differing} differ"
        )
    for name in only:
        print(f"only in one: {name}")
    failed |= args.same and bool(only)
    return 1 if failed else 0


def shift(args: argparse.Namespace) -> int:
    """Two frames in 60-row bands, the second moved up and down one device row: where the
    differing pixels fall, and whether a row offset explains them."""
    return run_shift(args.a, args.b, args.threshold, args.bands)


def same(args: argparse.Namespace) -> int:
    """Shot-for-shot equality of two states runs (tools/src/samerun.py): every comparison's
    changed/max/bounds, every surface run's ok and problems, every shared shot PNG identical
    at threshold 0."""
    return same_run(args.a.resolve(), args.b.resolve(), args.names)


def loop(args: argparse.Namespace) -> int:
    """The loop and the platform on the real clock: the web build in each engine, the native
    build through SDL's own loop (tools/live), each check's measurement printed."""
    out: Path = (args.out or SHOTS / "m2" / "loop").resolve()
    return run_loop(args.engines, out, not args.no_native, args.runs)


def sweep(args: argparse.Namespace) -> int:
    """Each state with its inputs shifted off the 16 ms grid, natively and at real speed in a
    browser: the runs that crash."""
    names = state_names(args.names)
    targets = [t for t in args.targets.split(",") if t]
    if unknown := [t for t in targets if t not in ("native", "web")]:
        print(f"sweep runs on native and web only, not {', '.join(unknown)}")
        return 1
    out: Path = (args.out or SWEEP_OUT).resolve()
    return run_sweep(names, args.offsets, targets, args.engines, args.jobs, out)


def release(args: argparse.Namespace) -> int:
    """The release build's frames beside the check build's, natively and in each engine."""
    names = state_names(args.names or ["default"])
    return run_release(names, args.engines, (args.out or SHOTS / "release").resolve())


def tabs(args: argparse.Namespace) -> int:
    """The web build's prefs across two tabs: a pick in one tab's settings panel shows in the
    other, checked as a read is (M12)."""
    return run_tabs(args.engines, (args.out or TABS_OUT).resolve())


def keys(args: argparse.Namespace) -> int:
    """The web build's browser shortcuts: what the page prevents per case, in each engine,
    routing build/web/ from disk (web-keys)."""
    return run_keys(args.engines, (args.out or KEYS_OUT).resolve())


def raf(args: argparse.Namespace) -> int:
    """The settings panel's opening and swap on the real clock, frame by frame, on the reference
    and the web build (M12)."""
    return run_raf(args.engines, (args.out or RAF_OUT).resolve())


def themes(_: argparse.Namespace) -> int:
    print(f"wrote {shown(generate_themes(react_ref()))}")
    return 0


def tokens(args: argparse.Namespace) -> int:
    """The theme fixture: every colour token of the six themes as the reference draws it."""
    return run_tokens(args.engines, args.out)


def widths(args: argparse.Namespace) -> int:
    """The width fixture: runs of the page's text as the reference's browsers measure them."""
    return run_widths(args.engines, args.out)


def font(args: argparse.Namespace) -> int:
    """Every face the page draws in, from $REACT_REF's fonts into assets/fonts; or, given a source
    and an output, one static instance of that font."""
    if args.source is None:
        for path in page_faces(react_ref(), args.into):
            print(f"wrote {shown(path)}")
        return 0
    if args.out is None:
        print("font SOURCE needs an OUT path")
        return 1
    axes: dict[str, float] = {}
    for setting in args.axis:
        tag, _, value = setting.partition("=")
        axes[tag] = float(value)
    print(f"wrote {instance(args.source, args.out, axes)} at {axes}")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(prog="./run", description=__doc__)
    commands = parser.add_subparsers(required=True)

    a = commands.add_parser(
        "act",
        help="run a script on the reference, the web build or the native build",
    )
    a.add_argument("script", type=Path)
    a.add_argument("--target", choices=TARGETS, required=True)
    a.add_argument(
        "--engines",
        type=engines_list,
        default=["webkit"],
        help="comma-separated, for ref and web",
    )
    a.add_argument("--out", type=Path, help="default .local/shots/act/<script>")
    a.set_defaults(run=act)

    s = commands.add_parser(
        "states", help="run states (or a milestone's, e.g. M1) on every surface"
    )
    s.add_argument(
        "names", nargs="+", help="state names in tools/states, a milestone such as M1, or all"
    )
    s.add_argument(
        "--targets",
        default=",".join(TARGETS),
        help="comma-separated: ref, web, native",
    )
    s.add_argument("--engines", type=engines_list, default=list(ENGINES), help="for web")
    s.add_argument("--ref-engines", type=engines_list, default=["webkit"], help="for the reference")
    s.add_argument(
        "--jobs",
        type=int,
        default=DEFAULT_JOBS,
        help=f"state x surface runs at once ({DEFAULT_JOBS}: the measured default; 1 runs "
        "each state in order, as before)",
    )
    s.add_argument("--out", type=Path, help="default .local/shots/<milestone>")
    s.set_defaults(run=states)

    j = commands.add_parser(
        "judge",
        help="the judgment's numbers for a states run: each native shot against the reference in "
        "each engine, the flat pixels apart and where they fall",
    )
    j.add_argument("dir", type=Path, help="a states run's directory, e.g. .local/shots/all")
    j.add_argument("names", nargs="*", help="its states, a milestone's or all (default: every one)")
    j.add_argument("--jobs", type=int, default=8, help="states at once")
    j.set_defaults(run=judge)

    g = commands.add_parser(
        "gamut",
        help="the states in Display P3: native P3 against native sRGB converted, the web build "
        "in P3 in each engine against native P3",
    )
    g.add_argument(
        "names", nargs="+", help="state names in tools/states, or a milestone such as M3"
    )
    g.add_argument("--engines", type=engines_list, default=list(ENGINES), help="for the web build")
    g.add_argument("--out", type=Path, help="default .local/shots/<milestone>-gamut")
    g.set_defaults(run=gamut)

    dm = commands.add_parser(
        "dom",
        help="the reference's layout at each shot of a state: boxes, text runs, baselines, styles",
    )
    dm.add_argument(
        "state", type=Path, nargs="?", help="a state in tools/states, or a script's path"
    )
    dm.add_argument(
        "--engines", type=engines_list, default=["webkit", "chromium"], help="comma-separated"
    )
    dm.add_argument(
        "--out",
        type=Path,
        help="default .local/dom/<state>: <engine>/<shot>.json, .txt, .png; with --fixture "
        "crates/function-viewer/testdata/page.json",
    )
    dm.add_argument(
        "--fixture",
        action="store_true",
        help="write the named measures the port's tests read (needs REACT_REF for its commit)",
    )
    dm.set_defaults(run=dom)

    v = commands.add_parser(
        "view",
        help="the plane's zoom and origin at each shot of states on the reference and natively",
    )
    v.add_argument(
        "names", nargs="+", help="state names in tools/states, or a milestone such as M9"
    )
    v.add_argument(
        "--engines",
        type=engines_list,
        default=["webkit", "chromium"],
        help="for the reference; the port is held to the first",
    )
    v.add_argument("--out", type=Path, help="default .local/shots/view")
    v.set_defaults(run=view)

    w = commands.add_parser("web", help="check the web build at WEB_PORT in each engine")
    w.add_argument("--engines", type=engines_list, default=list(ENGINES))
    w.add_argument("--out-dir", type=Path, default=SHOTS / "web")
    w.set_defaults(run=web)

    d = commands.add_parser("diff", help="compare two images, or the PNGs two directories share")
    d.add_argument("a", type=Path)
    d.add_argument("b", type=Path)
    d.add_argument(
        "--threshold", type=int, default=0, help="a channel difference above this counts (of 255)"
    )
    d.add_argument(
        "--inside", type=box_arg, help="X,Y,W,H in logical pixels: fail on a change outside it"
    )
    d.add_argument("--density", type=float, default=2, help="device pixels per logical pixel (2)")
    d.add_argument("--same", action="store_true", help="fail unless every pair is identical")
    d.add_argument("--out", type=Path, help="write the changed pixels over a dimmed B here")
    d.add_argument("--json", action="store_true")
    d.set_defaults(run=diff)

    sh = commands.add_parser(
        "shift",
        help="two frames in 60-row bands, the second moved up and down one device row: where "
        "the differing pixels fall, and whether a row offset explains them",
    )
    sh.add_argument("a", type=Path, help="the first frame (the reference side)")
    sh.add_argument("b", type=Path, help="the second frame (the port side, moved +-1 row)")
    sh.add_argument(
        "--threshold", type=int, default=0, help="a channel difference above this counts (of 255)"
    )
    sh.add_argument("--bands", type=int, default=60, help="band height in device rows")
    sh.set_defaults(run=shift)

    sr = commands.add_parser(
        "same-run",
        help="shot-for-shot equality of two states runs: every comparison's changed/max/bounds, "
        "every surface run's ok and problems, every shared shot PNG identical at threshold 0",
    )
    sr.add_argument("a", type=Path, help="the first states run's directory")
    sr.add_argument("b", type=Path, help="the second states run's directory")
    sr.add_argument("names", nargs="*", help="states to compare (default: every state in both)")
    sr.set_defaults(run=same)

    lp = commands.add_parser(
        "loop", help="the loop, idle, timers, input and platform services on the real clock"
    )
    lp.add_argument("--engines", type=engines_list, default=list(ENGINES))
    lp.add_argument("--no-native", action="store_true", help="the web build only")
    lp.add_argument("--out", type=Path, help="default .local/shots/m2/loop")
    lp.add_argument(
        "--runs", type=int, default=1, help="back to back, each into OUT/run-NN; then a summary"
    )
    lp.set_defaults(run=loop)

    sw = commands.add_parser(
        "sweep",
        help="states with their inputs shifted 0 to 15 ms off the 16 ms grid, natively and at real "
        "speed in a browser: the runs that crash",
    )
    sw.add_argument(
        "names", nargs="+", help="state names in tools/states, or a milestone such as M10"
    )
    sw.add_argument(
        "--offsets", type=offsets_arg, default=list(range(16)), help="ms off the grid: 0-15 or 1,8"
    )
    sw.add_argument("--targets", default="native,web", help="comma-separated: native, web")
    sw.add_argument("--engines", type=engines_list, default=["webkit"], help="for the web build")
    sw.add_argument("--jobs", type=int, default=4, help="runs at once")
    sw.add_argument("--out", type=Path, help="default .local/shots/sweep")
    sw.set_defaults(run=sweep)

    rl = commands.add_parser(
        "release",
        help="states (default: default) on ./build.sh release's build beside the check build's, "
        "natively and in each engine, at threshold 0",
    )
    rl.add_argument("names", nargs="*", help="state names in tools/states, or a milestone")
    rl.add_argument("--engines", type=engines_list, default=list(ENGINES))
    rl.add_argument("--out", type=Path, help="default .local/shots/release")
    rl.set_defaults(run=release)

    tb = commands.add_parser(
        "tabs",
        help="the web build at WEB_PORT in two tabs: a pick in one tab's settings panel shows in "
        "the other",
    )
    tb.add_argument("--engines", type=engines_list, default=["webkit", "chromium", "firefox"])
    tb.add_argument("--out", type=Path, help="default .local/shots/tabs")
    tb.set_defaults(run=tabs)

    ky = commands.add_parser(
        "keys",
        help="the web build's browser shortcuts: what the page prevents per case, "
        "routing build/web/ from disk",
    )
    ky.add_argument("--engines", type=engines_list, default=["webkit", "chromium", "firefox"])
    ky.add_argument("--out", type=Path, help="default .local/shots/keys")
    ky.set_defaults(run=keys)

    rf = commands.add_parser(
        "raf",
        help="the settings panel's opening and swap on the real clock, each animation frame sampled: "
        "React's computed opacity and scale, the web build's frames",
    )
    rf.add_argument("--engines", type=engines_list, default=["webkit", "firefox"])
    rf.add_argument("--out", type=Path, help="default .local/shots/raf")
    rf.set_defaults(run=raf)

    th = commands.add_parser(
        "themes", help="write crates/mv-theme/src/themes.rs from the six theme files at REACT_REF"
    )
    th.set_defaults(run=themes)

    tk = commands.add_parser(
        "tokens",
        help="write the theme fixture (crates/mv-theme/testdata/tokens.json) from the reference",
    )
    tk.add_argument(
        "--engines", type=engines_list, default=list(ENGINES), help="webkit's is the fixture"
    )
    tk.add_argument(
        "--out",
        type=Path,
        default=REPO / "crates" / "mv-theme" / "testdata" / "tokens.json",
    )
    tk.set_defaults(run=tokens)

    wd = commands.add_parser(
        "widths",
        help="write the text fixture (crates/mv-text/testdata/widths.json) from the reference",
    )
    wd.add_argument(
        "--engines", type=engines_list, default=list(ENGINES), help="webkit's is the fixture"
    )
    wd.add_argument("--out", type=Path, default=WIDTHS)
    wd.set_defaults(run=widths)

    fnt = commands.add_parser(
        "font",
        help="write the page's faces as static TTFs into assets/fonts from REACT_REF's fonts, "
        "or one instance of SOURCE into OUT",
    )
    fnt.add_argument("source", type=Path, nargs="?", help="a font (ttf, otf, woff, woff2)")
    fnt.add_argument("out", type=Path, nargs="?", help="the TTF to write")
    fnt.add_argument("--into", type=Path, default=FONTS, help="the page's faces go here")
    fnt.add_argument("--axis", action="append", default=[], help="TAG=VALUE, e.g. wght=500")
    fnt.set_defaults(run=font)

    args = parser.parse_args()
    return args.run(args)


if __name__ == "__main__":
    sys.exit(main())
