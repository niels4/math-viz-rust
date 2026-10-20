"""The GitHub Pages proof (t-pages): serve the committed docs/ from a subpath on a local
port and run the harness's web smoke plus the route states against it, so wrap and the
reviewer rerun the deploy check the same way. Copies docs/ to `.local/pages-root/<base>/`,
starts `python -m http.server` on 127.0.0.1 (stopped at exit), sets WEB_PORT and WEB_BASE
for its own runs, and from that server runs, in each engine: the web smoke (main.py's web
check, as a subprocess so the command is byte-identical), `welcome-route` (the root route,
/<base>#), `default` (#function-viewer) and `url-roundtrip` (the app writes the hash while
it runs). Each run's report.json must hold zero problems, and `location.pathname` must
still be the base after each run's last action (the app's hash writes go through
history.replaceState, which keeps the path). Writes .local/shots/pages/report.json and
exits non-zero on any failure."""

import http.client
import json
import os
import shutil
import socket
import subprocess
import sys
import time
from pathlib import Path
from typing import Any, cast

from playwright.sync_api import Page

from act import REPO, OnShot, Run, run
from script import Script, load

OUT = REPO / ".local" / "shots" / "pages"
ROOT = REPO / ".local" / "pages-root"
STATES = REPO / "tools" / "states"
DOCS = REPO / "docs"

# The states after the smoke: the root route, the viewer route, and a run that writes the
# hash mid-run. Each ends in a shot, so the pathname read rides on the last action.
STATE_NAMES = ("welcome-route", "default", "url-roundtrip")


def free_port(start: int = 5200) -> int:
    """A free local port at or above `start` (bound to 127.0.0.1, as the server is)."""
    probe = start
    while True:
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
            try:
                sock.bind(("127.0.0.1", probe))
            except OSError:
                probe += 1
                continue
            return probe


def stage(base: str) -> Path:
    """Copies the committed docs/ under the pages root at `base`; returns the root."""
    if not DOCS.is_dir():
        sys.exit(f"no {DOCS}: run ./build.sh pages (wrap's last step writes it)")
    shutil.rmtree(ROOT, ignore_errors=True)
    rel = base.strip("/")
    dest = ROOT / rel if rel else ROOT
    shutil.copytree(DOCS, dest)
    return ROOT


def wait_ready(port: int, base: str, timeout_s: float = 15.0) -> None:
    """Until the server answers the base's index, or exit (its log says why)."""
    end = time.monotonic() + timeout_s
    while time.monotonic() < end:
        try:
            conn = http.client.HTTPConnection("127.0.0.1", port, timeout=1)
            conn.request("GET", f"{base}index.html")
            status = conn.getresponse().status
            conn.close()
            if status == 200:
                return
        except OSError:
            pass
        time.sleep(0.2)
    sys.exit(f"the pages server on {port} never answered {base}index.html")


def path_reader() -> tuple[dict[str, tuple[str, str]], OnShot]:
    """An on_shot callback reading (pathname, href) after each shot, keyed by shot name."""
    seen: dict[str, tuple[str, str]] = {}

    def on_shot(page: Page, shot: str) -> None:
        seen[shot] = (
            str(page.evaluate("location.pathname")),
            str(page.evaluate("location.href")),
        )

    return seen, on_shot


def run_state(name: str, engine: str, out: Path) -> tuple[Run, dict[str, tuple[str, str]]]:
    """Runs one proof state on the served build; returns the run and its pathname reads."""
    script: Script = load(STATES / f"{name}.act")
    seen, on_shot = path_reader()
    result = run(script, "web", engine, out, on_shot)
    return result, seen


def streaming_note(consoles: list[list[str]]) -> str:
    """How the wasm instantiated: python's http.server maps .wasm to application/wasm, so
    the engines should stream-compile; an engine falling back to ArrayBuffer instantiation
    logs a console warning, which is recorded here (a note, never a failure)."""
    for console in consoles:
        for line in console:
            if "ArrayBuffer" in line:
                return f"ArrayBuffer fallback: {line[:200]}"
    return "no ArrayBuffer fallback warning: streamed"


def run_pages(
    port: int | None, base: str, engines: list[str], out: Path
) -> int:
    """Serves docs/ at `base` on `port` (a free one from 5200 when None) and runs the proof.
    Returns 0 with every report zero problems and every pathname still the base."""
    if not base.startswith("/") or not base.endswith("/"):
        print(f"base is {base!r}: it must start and end with '/'")
        return 1
    if port is None:
        port = free_port()
    out = out.resolve()
    root = stage(base)
    log = out / "server.log"
    out.mkdir(parents=True, exist_ok=True)
    proc = subprocess.Popen(
        [
            sys.executable,
            "-m",
            "http.server",
            "--directory",
            str(root),
            "--bind",
            "127.0.0.1",
            str(port),
        ],
        stdout=log.open("wb"),
        stderr=subprocess.STDOUT,
        cwd=REPO,
    )
    try:
        wait_ready(port, base)
        env = dict(os.environ, WEB_PORT=str(port), WEB_BASE=base)
        return _runs(port, base, engines, out, env)
    finally:
        proc.terminate()
        proc.wait(timeout=10)


def _runs(port: int, base: str, engines: list[str], out: Path, env: dict[str, str]) -> int:
    report: dict[str, object] = {"port": port, "base": base, "runs": {}, "ok": False}
    runs = cast(dict[str, object], report["runs"])
    failed = False

    # The web smoke, as wrap and the reviewer run it by hand.
    smoke_out = out / "smoke"
    try:
        done = subprocess.run(
            [
                str(REPO / "tools" / "run"),
                "web",
                "--engines",
                ",".join(engines),
                "--out-dir",
                str(smoke_out),
            ],
            capture_output=True,
            text=True,
            cwd=REPO,
            env=env,
            timeout=1200,
        )
    except subprocess.TimeoutExpired:
        print("the web smoke ran past 1200 s")
        return 1
    print(done.stdout, end="")
    print(done.stderr, end="", file=sys.stderr)
    smoke_report: dict[str, Any] = {}
    try:
        with (smoke_out / "web-report.json").open() as handle:
            smoke_report = cast(dict[str, Any], json.load(handle))
    except (FileNotFoundError, json.JSONDecodeError) as error:
        print(f"no smoke report: {error}")
        failed = True
    smoke: dict[str, object] = {}
    runs["smoke"] = smoke
    consoles: dict[str, list[list[str]]] = {engine: [] for engine in engines}
    for engine in engines:
        entry = cast(dict[str, object], smoke_report.get(engine, {}))
        ok = bool(entry.get("ok")) and done.returncode == 0
        pathname = entry.get("pathname")
        if pathname != base:
            print(f"smoke {engine}: pathname {pathname!r} is not the base {base!r}")
            ok = False
        consoles[engine].append([str(line) for line in cast(list[object], entry.get("console", []))])
        smoke[engine] = {
            "ok": ok,
            "out": entry.get("out"),
            "gl": entry.get("gl"),
            "hover": entry.get("hover"),
            "pathname": pathname,
            "problems": entry.get("problems", ["no smoke report"]),
            "console": entry.get("console", []),
        }
        failed |= not ok

    # The route states, in-process so the pathname is read after the last action.
    states: dict[str, object] = {}
    runs["states"] = states
    for name in STATE_NAMES:
        per_engine: dict[str, object] = {}
        states[name] = per_engine
        for engine in engines:
            state_out = out / f"{name}-{engine}"
            result, seen = run_state(name, engine, state_out)
            print(result.summary())
            for problem in result.problems:
                print(f"  {problem}")
            ok = result.ok
            pathnames = [pathname for pathname, _ in seen.values()]
            if not pathnames:
                print(f"  {name} {engine}: no pathname read (no shots?)")
                ok = False
            elif any(pathname != base for pathname in pathnames):
                print(f"  {name} {engine}: pathname left the base: {pathnames!r}")
                ok = False
            else:
                print(f"  {name} {engine}: pathname still {base} after {len(pathnames)} shots")
            detail: dict[str, object] = {
                "ok": ok,
                "out": str(result.out),
                "gl": result.detail.get("gl"),
                "pathnames": pathnames,
                "hrefs": [href for _, href in seen.values()],
                "problems": result.problems,
                "console": result.console,
            }
            if name == "welcome-route":
                # The port draws the viewer at every route until h10-page merges
                # (h10-t-route's record), and the web target writes no views.json,
                # so the route reader has no route line: a check that can't run.
                detail["route"] = "can't run: no route line (web writes no views.json)"
                print(f"  {name} {engine}: draws the viewer until h10-page (route: can't run)")
            if name == "url-roundtrip" and seen:
                last_href = next(reversed(seen.values()))[1]
                mark = f"#function-viewer"
                if f"{base}{mark}" not in last_href:
                    print(f"  {name} {engine}: last href {last_href!r} holds no {mark} past the base")
                    ok = False
                    detail["ok"] = False
                else:
                    print(f"  {name} {engine}: ends at {last_href!r}")
            consoles[engine].append(result.console)
            per_engine[engine] = detail
            failed |= not ok

    streaming = {engine: streaming_note(consoles[engine]) for engine in engines}
    report["streaming"] = streaming
    for engine in engines:
        print(f"{engine}: {streaming[engine]}")
    report["ok"] = not failed
    (out / "report.json").write_text(json.dumps(report, indent=2))
    return 1 if failed else 0
