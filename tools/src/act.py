"""Runs a script (script.py) on one surface: the math-viz2 reference or the web build in a
browser through Playwright, or the native build through its --script flag. Each run writes its
shots, any video and a report.json into a directory of its own.

In a browser the script runs on Playwright's clock, paused from before the page loads. Time 0
is the page's first animation frame after it has loaded, and `wait` runs the clock forward, so
animation frames fall every 16 ms of script time on every run and a shot shows the same frame
each time. The native harness frames on the same grid. A script that records video runs on the
real clock instead."""

import json
import math
import os
import subprocess
import sys
import time
from collections.abc import Callable
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, cast

import av
from playwright.sync_api import BrowserContext
from playwright.sync_api import Error as PlaywrightError
from playwright.sync_api import Page, Route, sync_playwright

from browser import PageLog, engine_type
from script import (
    MODIFIERS,
    Click,
    Frames,
    Hash,
    Hold,
    Key,
    Model,
    MotionChange,
    Move,
    Pinch,
    Press,
    Resize,
    SchemeChange,
    Script,
    Shot,
    Start,
    Type,
    Verb,
    Video,
    Wait,
    Wheel,
)

REPO = Path(__file__).resolve().parents[2]
NATIVE = REPO / "build" / "native" / "function-viewer"
TARGETS = ("ref", "web", "native")

# A build given by its directory (build/, or ./build.sh release's build/release/) reaches the
# browser from its web/ directory through Playwright's request routing, at an origin no server
# listens on (port 2, off the browsers' blocked ports), so nothing else answers and nothing is
# started: a clone tests its own files. The web target is routed by default; WEB_PORT honoured
# when set serves it from a server instead, for a person looking at a build.
ROUTED_ORIGIN = "http://localhost:2"

# The math-viz2 page the port copies.
REFERENCE_ROUTE = "#function-viewer"

# The paused clock's wall time: 2026-10-12T00:00:00Z.
EPOCH_MS = 1_791_763_200_000
# Playwright's clock runs animation frames on multiples of 16 ms (`16 - ticks % 16`).
FRAME_MS = 16

# Called with the page and the shot's name after each shot in a browser (tools/run dom).
type OnShot = Callable[[Page, str], None]


def shown(path: Path) -> Path:
    """A path as the README writes it: from the repository when it lies inside."""
    return path.relative_to(REPO) if path.is_relative_to(REPO) else path


def port(name: str) -> int:
    value = os.environ.get(name)
    if value is None:
        sys.exit(f"{name} is not set: export the port your server listens on (README: The harness)")
    return int(value)


def url_for(target: str, query: str = "") -> str:
    """The page's URL: the reference served on REACT_REF_PORT at the Function Viewer route (with
    ?query appended when the script opens with a `query` line), or the web build on WEB_PORT (at
    the same route: the build reads only the hash, so a `?query` search would be dropped)."""
    if target == "ref":
        tail = f"?{query}" if query else ""
        return f"http://localhost:{port('REACT_REF_PORT')}/{REFERENCE_ROUTE}{tail}"
    tail = f"{REFERENCE_ROUTE}?{query}" if query else ""
    return f"http://localhost:{port('WEB_PORT')}/{tail}"


def is_web(target: str) -> bool:
    """A target drawn by the web build in a browser."""
    return target == "web"


def is_native(target: str) -> bool:
    """A target drawn by a native binary through its --script flag."""
    return target == "native"


def web_dir(build: Path | None) -> Path:
    """The web/ directory a web run is routed from: the given build's, else the build's own
    (WEB_PORT honoured when set serves it from a server instead)."""
    if build is not None:
        return build / "web"
    return REPO / "build" / "web"


def native_binary(build: Path | None) -> Path:
    """The native binary a native run runs: the given build's, else the build's own."""
    if build is not None:
        return build / "native" / "function-viewer"
    return NATIVE


@dataclass
class Run:
    """What one script did on one surface."""

    target: str
    engine: str  # "" for native
    out: Path
    clock: str  # "paused" or "real"
    problems: list[str] = field(default_factory=lambda: [])
    console: list[str] = field(default_factory=lambda: [])
    shots: list[str] = field(default_factory=lambda: [])
    videos: list[str] = field(default_factory=lambda: [])
    seconds: float = 0
    detail: dict[str, object] = field(default_factory=lambda: {})

    @property
    def name(self) -> str:
        return f"{self.target}-{self.engine}" if self.engine else self.target

    @property
    def ok(self) -> bool:
        return not self.problems

    def write_report(self) -> None:
        report = asdict(self)
        report["out"] = str(self.out)
        (self.out / "report.json").write_text(json.dumps(report, indent=2))

    def summary(self) -> str:
        status = "ok" if self.ok else "FAILED"
        text = f"{self.name}: {status}, {len(self.shots)} shots"
        if self.videos:
            text += f", {len(self.videos)} video"
        text += f", {self.seconds:.1f} s, {self.clock} clock"
        if gl := self.detail.get("gl"):
            text += f"; {gl}"
        return text


def run(
    script: Script,
    target: str,
    engine: str,
    out: Path,
    on_shot: OnShot | None = None,
    build: Path | None = None,
) -> Run:
    """Runs `script` on `target` (ref, web or native; `engine` for the browser ones) into `out`.
    In a browser, `on_shot` is called with the page after each shot is written (tools/run dom).
    The port runs from `build` when it is given (its native/function-viewer, and its web/
    through ROUTED_ORIGIN); else the web target is routed from its own build directory (WEB_PORT
    honoured when set), and the native target runs its own binary. A native run's views.json,
    motion.json, time.json and live.json are checked against their shapes (check_shapes), a
    mismatch counting as a problem."""
    out.mkdir(parents=True, exist_ok=True)
    for old in [*out.glob("*.png"), *out.glob("*.webm")]:
        old.unlink()
    if target == "ref" and script.start.space != "srgb":
        # Every headless capture of the reference is sRGB (plan section 0): P3 is the port's.
        result = Run(target, engine, out, "none")
        result.problems.append(
            f"space {script.start.space}: the reference is captured in sRGB only"
        )
    elif is_native(target):
        result = run_native(script, out, binary=native_binary(build), name=target)
        result.problems += check_shapes(out, script.shot_names())
    else:
        result = run_browser(script, target, engine, out, on_shot, build)
    result.write_report()
    return result


# The reference's CSS animations and transitions on the script's clock (a paused clock only): the
# page's rAF-driven motion follows Playwright's clock, but the browser's own animation timeline
# runs on real time, so a fade CSS starts (Q's row, an overlay) would run on while a script's time
# stands still. Each animation is held at its start when it is first seen (right after the input
# that started it, __harnessSeek, or at the next frame), and on every frame of the paused clock is
# set to the script time since, as a browser runs it from the rendering update after the change.
# React renders the update of a continuous event (a pointer move) in a task of its own scheduler,
# posted as a message: __harnessSeek yields a message's turn first, so it sees what that render
# started. A timer's update (a chip's rest opening an explainer) renders the same way, in the task
# the clock yields after each timer, which falls after that frame's callbacks: Actor.wait steps the
# clock a frame at a time and calls __harnessFrameSeek after each step, which holds what the step's
# timers set going at the step's last frame, as the port starts it (the first frame that draws the
# change), and sets the rest to that frame as the frame's own seek would: the DOM's motions stand on
# the frames, as the canvases' do.
SEEK_ANIMATIONS = """
(() => {
  const seek = (now) => {
    for (const animation of document.getAnimations()) {
      if (animation.__harnessStart === undefined) {
        animation.__harnessStart = now;
        animation.pause();
      }
      animation.currentTime = now - animation.__harnessStart;
    }
  };
  const afterMessages = (at) =>
    new Promise((done) => {
      const channel = new MessageChannel();
      channel.port1.onmessage = () => {
        seek(at());
        done(true);
      };
      channel.port2.postMessage(0);
    });
  window.__harnessSeek = () => afterMessages(() => performance.now());
  window.__harnessFrameSeek = () =>
    afterMessages(() => {
      const now = performance.now();
      return now - (now % __FRAME_MS__);
    });
  const frame = () => {
    seek(performance.now());
    requestAnimationFrame(frame);
  };
  requestAnimationFrame(frame);
})();""".replace("__FRAME_MS__", str(FRAME_MS))


def init_script(start: Start, target: str, paused: bool) -> str:
    """Before any page script: the appearance prefs in math-viz2's keys and JSON shape
    (src/state/useAppTheme.ts: atomWithStorage under createJSONStorage, one JSON string per key;
    math-viz2 has no tour key) and, for the web build, the same prefs under the port's own keys
    (math-viz2-rust-*: the port never shares runtime state with math-viz2); the reference before
    h1's three-key model reads the single `mathviz-theme` key instead, so both shapes are seeded
    and each reference reads its own; on the reference 2D canvases in sRGB, since a headless
    screenshot shows a display-p3 canvas's values as sRGB, and under a paused clock its CSS
    animations on the script's clock (SEEK_ANIMATIONS); on the web build the script's colour
    space, which wins over the engine's rule (web/library.js)."""
    mode = json.dumps(json.dumps(start.mode))
    dark = json.dumps(json.dumps(start.dark_theme))
    light = json.dumps(json.dumps(start.light_theme))
    shown = json.dumps(json.dumps(start.theme_shown))
    lines = [
        f"localStorage.setItem('mathviz-theme', {shown});",
        f"localStorage.setItem('mathviz-light-dark', {mode});",
        f"localStorage.setItem('mathviz-dark-theme', {dark});",
        f"localStorage.setItem('mathviz-light-theme', {light});",
    ]
    if target == "web":
        lines += [
            f"localStorage.setItem('math-viz2-rust-light-dark', {mode});",
            f"localStorage.setItem('math-viz2-rust-dark-theme', {dark});",
            f"localStorage.setItem('math-viz2-rust-light-theme', {light});",
        ]
    if is_web(target):
        space = "display-p3" if start.space == "p3" else "srgb"
        lines.append(f"window.Module = {{ colorSpace: {json.dumps(space)} }};")
    if target == "ref":
        lines.append("""
const getContext = HTMLCanvasElement.prototype.getContext;
HTMLCanvasElement.prototype.getContext = function (type, attributes) {
  if (type === "2d") attributes = { ...attributes, colorSpace: "srgb" };
  return getContext.call(this, type, attributes);
};""")
        if paused:
            lines.append(SEEK_ANIMATIONS)
    return "\n".join(lines)


def run_browser(
    script: Script,
    target: str,
    engine: str,
    out: Path,
    on_shot: OnShot | None = None,
    build: Path | None = None,
) -> Run:
    paused = not script.real_time
    result = Run(target, engine, out, "paused" if paused else "real")
    start = script.start
    began = time.monotonic()
    log = PageLog()
    with sync_playwright() as playwright:
        browser = engine_type(playwright, engine).launch()
        try:
            context = browser.new_context(
                viewport={"width": start.size[0], "height": start.size[1]},
                device_scale_factor=start.density,
                color_scheme="dark" if start.scheme == "dark" else "light",
                reduced_motion="reduce" if start.reduced_motion else "no-preference",
                locale="en-US",
                timezone_id="UTC",
            )
            context.add_init_script(init_script(start, target, paused))
            if paused:
                # pause_at alone, not install first: a page replays the clock's log, and between
                # an install and a pause it counts the real ms that passed, so the page would
                # load at 1 to 3 ms of ticks, a little different each run. Paused alone, it loads
                # at 0 and every run's motion starts from the same time.
                context.clock.pause_at(EPOCH_MS)
            page = context.new_page()
            log.watch(page)
            # The reference is the dev server, never a routed build; the web build is routed
            # from disk unless WEB_PORT serves it. The script's `query` line opens on both.
            query = script.start.query
            if target == "ref" or os.environ.get("WEB_PORT"):
                url = url_for(target, query)
            else:
                url = serve(context, web_dir(build))
                if query:
                    url += f"{REFERENCE_ROUTE}?{query}"
            if target == "web" and os.environ.get("MV_PANIC_AT_FRAME") == "1":
                # The sweep's crash proof on the web build: the page reads
                # `?panic-at-frame` through `mv_panic_at_frame` (web/library.js)
                # and panics in frame 1. A search flag, so it goes before any hash.
                head, mark, tail = url.partition("#")
                head += ("&" if "?" in head else "?") + "panic-at-frame"
                url = head + mark + tail
            page.goto(url, wait_until="load")
            first_frame(page, target, paused)
            actor = Actor(page, target, paused, out, result, on_shot)
            for action in script.actions:
                if not action.runs_on(target):
                    continue
                try:
                    actor.act(action.verb)
                except PlaywrightError as error:
                    raise PlaywrightError(f"line {action.line}: {error.message}") from None
            if target == "web":
                result.detail["gl"] = page.evaluate(GL_INFO)
                result.detail["frames"] = page.evaluate("Module.frames")
            context.close()
        except PlaywrightError as error:
            log.page_errors.append(f"harness: {error.message.splitlines()[0]}")
        finally:
            browser.close()
    result.problems = log.problems()
    result.console = log.console
    result.seconds = time.monotonic() - began
    return result


def serve(context: BrowserContext, root: Path) -> str:
    """Serves the files under `root` to the context's pages at ROUTED_ORIGIN; returns its URL."""

    def fulfill(route: Route) -> None:
        name = route.request.url.removeprefix(ROUTED_ORIGIN).split("?")[0].lstrip("/")
        file = root / (name or "index.html")
        if file.is_file():
            route.fulfill(path=file)
        else:
            route.fulfill(status=404, body="")

    context.route(f"{ROUTED_ORIGIN}/**", fulfill)
    return f"{ROUTED_ORIGIN}/"


def first_frame(page: Page, target: str, paused: bool) -> None:
    """Waits for the page to load, then for its first animation frame: script time 0."""
    if target == "ref":
        # React has mounted once main.tsx's effect set the theme class on the body (h1's
        # plane adds a canvas to wait for instead).
        page.wait_for_function("() => document.body.className.length > 0")
        page.evaluate("document.fonts.ready.then(() => true)")
    else:
        # web/index.html sets Module.started once the app's init has run; its first frame comes
        # with the next animation frame.
        page.wait_for_function("() => window.Module?.started === true", polling=50)
    if paused:
        # Run the clock to the next frame boundary (from 0, the first: 16 ms).
        ticks = float(page.evaluate("performance.now()"))
        page.clock.run_for(math.ceil(FRAME_MS - ticks % FRAME_MS))
    elif target == "ref":
        page.evaluate("new Promise((done) => requestAnimationFrame(() => done(true)))")
    if is_web(target):
        if paused and page.evaluate("Module.frames") != 1:
            raise PlaywrightError("the web build drew no first frame")
        page.wait_for_function("() => Module.frames >= 1")


class Actor:
    """Performs a script's verbs in a page, keeping the pointer and the script's time."""

    def __init__(
        self,
        page: Page,
        target: str,
        paused: bool,
        out: Path,
        result: Run,
        on_shot: OnShot | None = None,
    ) -> None:
        self.page = page
        self.target = target
        self.paused = paused
        self.out = out
        self.result = result
        self.on_shot = on_shot
        self.pointer = (0.0, 0.0)  # where Playwright's mouse starts
        self.time = 0  # ms since the first frame
        self.real_start = time.monotonic()
        self.recording: tuple[Path, float] | None = None  # the video and when it started

    def wait(self, ms: int) -> None:
        start = self.time
        self.time += ms
        if self.paused:
            if self.target != "ref":
                if ms > 0:
                    self.page.clock.run_for(ms)
                return
            # On the reference a frame at a time (script time 0 is a frame), each step seeking the
            # CSS animations (SEEK_ANIMATIONS): one that a timer in the step set going is held at
            # the step's last frame, not first seen a frame later, nor run on real time through a
            # shot's real frames.
            at = start
            while at < self.time:
                to = min(self.time, (at // FRAME_MS + 1) * FRAME_MS)
                self.page.clock.run_for(to - at)
                self.page.evaluate("window.__harnessFrameSeek?.()")
                at = to
            return
        # On the real clock, keep to the script's timeline however long each call took.
        ahead = self.real_start + self.time / 1000 - time.monotonic()
        if ahead > 0:
            self.page.wait_for_timeout(ahead * 1000)

    def shot(self, name: str) -> None:
        self.presented()
        self.page.screenshot(path=self.out / f"{name}.png")
        self.result.shots.append(name)
        if self.on_shot is not None:
            self.on_shot(self.page, name)

    def presented(self) -> None:
        """Waits two of the browser's real frames, so a canvas drawn under the paused clock has
        been composited: Firefox shows a WebGL frame only at its next refresh, and a screenshot
        taken before that shows the frame before. Playwright polls on the real animation frames,
        which its clock leaves alone, so the script's time stands still meanwhile."""
        self.page.evaluate("window.__harnessPolls = 0")
        self.page.wait_for_function(
            "() => ++window.__harnessPolls > 2", polling="raf", timeout=10_000
        )

    def media_changed(self) -> None:
        """WebKit tells a page that a media query changed, and Chromium that the window resized,
        during a real rendering update, which the paused clock never runs: two real frames pass
        first, so the page has heard of it before the script's clock moves on."""
        if self.paused:
            self.presented()

    def move(self, to: tuple[float, float]) -> None:
        self.page.mouse.move(*to)
        self.pointer = to

    def inputs_now(self) -> None:
        """Under the paused clock the inputs just dispatched take effect at this instant on every
        surface, as a browser renders after the input events it dispatches: React has updated its
        page already (the screenshot renders it), the web build runs one turn of its main loop here
        (Module.mvIterate: its events, a frame), and the reference's CSS animations that the input
        started are held at their start (__harnessSeek). The native harness draws a frame at the
        input's time likewise (crates/function-viewer/src/harness.rs)."""
        if not self.paused:
            return
        if is_web(self.target):
            self.page.evaluate("Module.mvIterate()")
        elif self.target == "ref":
            self.page.evaluate("window.__harnessSeek?.()")

    def act(self, verb: Verb) -> None:
        mouse, keyboard = self.page.mouse, self.page.keyboard
        match verb:
            case Move(to, 0):
                self.move(to)
                self.inputs_now()
            case Move(to, over) if self.paused:
                # As the native harness steps: a step every frame or so, whole pixels between.
                steps = max(1, (over + FRAME_MS // 2) // FRAME_MS)
                start = self.pointer
                for k in range(1, steps + 1):
                    self.move(to if k == steps else along(start, to, k, steps))
                    self.inputs_now()
                    self.wait(over * k // steps - over * (k - 1) // steps)
            case Move(to, over):
                # On the real clock a step lands where the pointer is due by then, a frame or more
                # apart, so a slow engine takes fewer steps rather than more time.
                start, began = self.pointer, time.monotonic()
                while (elapsed := (time.monotonic() - began) * 1000) < over:
                    self.move(along(start, to, elapsed, over))
                    due = math.ceil((time.monotonic() - began) * 1000 / FRAME_MS) * FRAME_MS
                    self.page.wait_for_timeout(max(0.0, due - (time.monotonic() - began) * 1000))
                self.move(to)
                self.time += over
            case Press(down):
                if down:
                    mouse.down()
                else:
                    mouse.up()
                self.inputs_now()
            case Click(at, count):
                (mouse.click if count == 1 else mouse.dblclick)(*at)
                self.pointer = at
                self.inputs_now()
            case Wheel(delta, ctrl):
                if ctrl:
                    keyboard.down("Control")
                mouse.wheel(*delta)
                if ctrl:
                    keyboard.up("Control")
                self.inputs_now()
            case Pinch(scale):
                # A trackpad pinch reaches a page as a wheel event with Ctrl held, -100 ln(scale)
                # pixels down (Chromium's convention).
                keyboard.down("Control")
                mouse.wheel(0, -100 * math.log(scale))
                keyboard.up("Control")
                self.inputs_now()
            case Key(key, mods):
                keyboard.press("+".join([*(MODIFIERS[m] for m in mods), key]))
                self.inputs_now()
            case Hold(key, down):
                if down:
                    keyboard.down(key)
                else:
                    keyboard.up(key)
                self.inputs_now()
            case Type(text):
                keyboard.type(text)
                self.inputs_now()
            case Wait(ms):
                self.wait(ms)
            case Hash(query):
                # A mid-run edit of the page's hash: the page's own hashchange
                # path runs on it (the reference's router, the port's shell
                # listener), as an edit in the address bar would.
                route = REFERENCE_ROUTE.lstrip("#")
                self.page.evaluate("(hash) => { location.hash = hash; }", f"#{route}?{query}")
                self.inputs_now()
            case Shot(name):
                self.shot(name)
            case Frames(name, every, span):
                for t in Frames(name, every, span).times():
                    self.shot(f"{name}-{t:04d}")
                    if t + every <= span:
                        self.wait(every)
            case Video(start, name):
                if start:
                    path = self.out / f"{name}.webm"
                    self.page.screencast.start(path=path, size=self.page.viewport_size)
                    self.recording = (path, time.monotonic())
                    self.result.videos.append(name)
                elif self.recording is not None:
                    path, began = self.recording
                    section = time.monotonic() - began
                    self.page.screencast.stop()
                    self.result.detail[path.stem] = {
                        "section_s": round(section, 3),
                        "file_s": round(trim(path, section), 3),
                    }
                    self.recording = None
            case Resize(size):
                self.page.set_viewport_size({"width": size[0], "height": size[1]})
                # Chromium dispatches the window's resize event at its next real rendering
                # update, which the paused clock never runs: without it SDL resizes (and so
                # clears) the canvas only after the script's frames have run, and a shot shows it
                # blank. Two real frames let every engine hear of it first.
                self.media_changed()
                self.inputs_now()
            case SchemeChange(scheme):
                self.page.emulate_media(color_scheme="dark" if scheme == "dark" else "light")
                self.media_changed()
                self.inputs_now()
            case MotionChange(reduced):
                self.page.emulate_media(reduced_motion="reduce" if reduced else "no-preference")
                self.media_changed()
                self.inputs_now()
            case Model(text):
                # The web build's model set as the native harness sets it (a `port` line: the
                # reference never sees one), queued for the next frame as an input is.
                if not self.page.evaluate("line => Module.mvModelLine(line)", text):
                    raise PlaywrightError(f'the web build refused "{text}"')
                self.inputs_now()


def trim(path: Path, seconds: float) -> float:
    """Cuts a recording to its first `seconds`, the section the script recorded: Playwright holds
    the last frame for most of a second after the screencast stops. The packets are copied, not
    encoded again. Returns the length of what is left, in seconds."""
    cut = path.with_suffix(".cut.webm")
    end = 0.0
    with av.open(str(path)) as source, av.open(str(cut), "w", format="webm") as target:
        stream = source.streams.video[0]
        copy = target.add_stream_from_template(stream)  # pyright: ignore[reportUnknownMemberType]
        for packet in source.demux(stream):
            if packet.pts is None:
                continue
            start = float(packet.pts * packet.time_base)
            if start >= seconds:
                break
            end = start + float((packet.duration or 0) * packet.time_base)
            packet.stream = copy
            target.mux(packet)  # pyright: ignore[reportUnknownMemberType]
    cut.replace(path)
    return end


def along(
    start: tuple[float, float], to: tuple[float, float], part: float, whole: float
) -> tuple[float, float]:
    """The whole pixel `part / whole` of the way from `start` to `to`, multiplied before divided
    and rounded half up, as the native harness computes it, so both builds see the same points."""
    x = math.floor(start[0] + (to[0] - start[0]) * part / whole + 0.5)
    y = math.floor(start[1] + (to[1] - start[1]) * part / whole + 0.5)
    return float(x), float(y)


# The WebGL context the page draws with, read from SDL's canvas. Chromium and WebKit mask
# RENDERER and unmask it through an extension that Firefox, which doesn't mask, deprecates.
GL_INFO = """() => {
    const gl = document.getElementById("canvas").getContext("webgl2");
    if (!gl) return "no webgl2 context";
    let renderer = gl.getParameter(gl.RENDERER);
    if (renderer === "WebKit WebGL") {
        const debug = gl.getExtension("WEBGL_debug_renderer_info");
        if (debug) renderer = gl.getParameter(debug.UNMASKED_RENDERER_WEBGL);
    }
    return `${gl.getParameter(gl.VERSION)}, ${renderer}`;
}"""


def run_native(
    script: Script, out: Path, window: bool = False, binary: Path = NATIVE, name: str = "native"
) -> Run:
    """The native build runs the script itself (the native harness), always on its own clock."""
    result = Run(name, "", out, "script")
    if not binary.exists():
        result.problems.append(f"no {shown(binary)}: run ./build.sh native (or release)")
        return result
    command = [str(binary), "--script", str(script.path), "--out", str(out)]
    if script.start.query:
        # The script's `query` line opens on every surface: --query natively
        # (main.rs seeds App.url from it), at the hash on ref and web.
        command += ["--query", script.start.query]
    if window:
        command.append("--window")
    began = time.monotonic()
    try:
        done = subprocess.run(command, capture_output=True, text=True, timeout=600, cwd=REPO)
    except subprocess.TimeoutExpired:
        result.problems.append("the native build ran past 600 s")
        return result
    result.seconds = time.monotonic() - began
    result.console = (done.stdout + done.stderr).splitlines()
    result.problems = [line for line in result.console if "[ERROR]" in line or "[FATAL]" in line]
    if done.returncode != 0:
        result.problems.append(f"exit status {done.returncode}")
    result.shots = [name for name in script.shot_names() if (out / f"{name}.png").exists()]
    if missing := [name for name in script.shot_names() if name not in result.shots]:
        result.problems.append(f"no shot for {', '.join(missing)}")
    return result


def _is_num(value: object) -> bool:
    return isinstance(value, (int, float)) and not isinstance(value, bool)


def _is_int(value: object) -> bool:
    return isinstance(value, int) and not isinstance(value, bool)


# The theme slugs a views.json line may show (script.py: the six themes).
THEMES_SHOWN = (
    "arctic-ice",
    "tron-cyan",
    "midnight-violet",
    "clean-teal",
    "educational-warm",
    "sage-editorial",
)

# Every key of a views.json line, in the harness's order.
VIEW_KEYS = (
    "shot",
    "time",
    "zoom",
    "pan_x",
    "pan_y",
    "origin_x",
    "origin_y",
    "cursor",
    "theme",
    "scaled",
)


def check_views(out: Path) -> list[str]:
    """Each line of <out>/views.json against the native harness's shape (research odin-harness.md
    section 5): shot (str), time (int ms on the harness's 16 ms grid), zoom (px per unit, above
    0), pan_x/pan_y (view units), origin_x/origin_y (plane px on the device grid), cursor (a CSS
    string), theme (a slug), scaled (layer boxes [x, y, w, h] in logical px). A line holding only
    `url` (str) is the page's hash query, written whenever it changes (--query writes one at
    frame 0)."""
    problems: list[str] = []
    try:
        lines = (out / "views.json").read_text().splitlines()
    except FileNotFoundError:
        return ["no views.json"]
    for number, line in enumerate(lines, start=1):
        try:
            parsed: Any = json.loads(line)
        except json.JSONDecodeError as error:
            problems.append(f"views.json line {number}: not JSON ({error})")
            continue
        if not isinstance(parsed, dict):
            problems.append(f"views.json line {number}: not an object")
            continue
        view = cast(dict[str, object], parsed)
        if set(view) == {"url"}:
            if not isinstance(view.get("url"), str):
                problems.append(f"views.json line {number}: 'url' is not a string")
            continue
        for key in VIEW_KEYS:
            if key not in view:
                problems.append(f"views.json line {number}: no {key!r}")
        for key in view:
            if key not in VIEW_KEYS:
                problems.append(f"views.json line {number}: unexpected {key!r}")
        if "shot" in view and not isinstance(view.get("shot"), str):
            problems.append(f"views.json line {number}: 'shot' is not a string")
        if "time" in view and not _is_int(view.get("time")):
            problems.append(f"views.json line {number}: 'time' is not int ms")
        for key in ("zoom", "pan_x", "pan_y", "origin_x", "origin_y"):
            if key in view and not _is_num(view.get(key)):
                problems.append(f"views.json line {number}: {key!r} is not a number")
        if "cursor" in view and not isinstance(view.get("cursor"), str):
            problems.append(f"views.json line {number}: 'cursor' is not a string")
        zoom = view.get("zoom")
        if isinstance(zoom, (int, float)) and not isinstance(zoom, bool) and zoom <= 0:
            problems.append(f"views.json line {number}: 'zoom' is not above 0")
        theme = view.get("theme")
        if "theme" in view and (not isinstance(theme, str) or theme not in THEMES_SHOWN):
            problems.append(f"views.json line {number}: 'theme' is not a theme slug")
        scaled = view.get("scaled")
        if not isinstance(scaled, list):
            problems.append(f"views.json line {number}: 'scaled' is not a list")
        else:
            for box in cast(list[object], scaled):
                if not isinstance(box, list):
                    problems.append(f"views.json line {number}: 'scaled' holds no [x, y, w, h]")
                    break
                cells = cast(list[object], box)
                if len(cells) != 4 or not all(_is_num(v) for v in cells):
                    problems.append(f"views.json line {number}: 'scaled' holds no [x, y, w, h]")
                    break
    return problems


def check_motion(out: Path) -> list[str]:
    """<out>/motion.json: {"moving": [shot names]}, the shots taken mid-motion-fade."""
    try:
        parsed: Any = json.loads((out / "motion.json").read_text())
    except FileNotFoundError:
        return []
    except json.JSONDecodeError as error:
        return [f"motion.json: not JSON ({error})"]
    if not isinstance(parsed, dict):
        return ["motion.json: not {'moving': [shot names]}"]
    found = cast(dict[str, object], parsed)
    if set(found) != {"moving"}:
        return ["motion.json: not {'moving': [shot names]}"]
    moving = found.get("moving")
    if not isinstance(moving, list) or not all(
        isinstance(name, str) for name in cast(list[object], moving)
    ):
        return ["motion.json: not {'moving': [shot names]}"]
    return []


def check_time(out: Path) -> list[str]:
    """<out>/time.json: pixels ([w, h] device px), window (bool), redraws (int), median_ms,
    p95_ms, slowest_ms (ms, at least 0)."""
    try:
        parsed: Any = json.loads((out / "time.json").read_text())
    except FileNotFoundError:
        return []
    except json.JSONDecodeError as error:
        return [f"time.json: not JSON ({error})"]
    problems: list[str] = []
    if not isinstance(parsed, dict):
        return ["time.json: not an object"]
    found = cast(dict[str, object], parsed)
    for key in ("pixels", "window", "redraws", "median_ms", "p95_ms", "slowest_ms"):
        if key not in found:
            problems.append(f"time.json: no {key!r}")
    for key in found:
        if key not in ("pixels", "window", "redraws", "median_ms", "p95_ms", "slowest_ms"):
            problems.append(f"time.json: unexpected {key!r}")
    pixels = found.get("pixels")
    if not isinstance(pixels, list):
        problems.append("time.json: 'pixels' is not [w, h] device px")
    else:
        cells = cast(list[object], pixels)
        if len(cells) != 2 or not all(_is_int(v) for v in cells):
            problems.append("time.json: 'pixels' is not [w, h] device px")
    if "window" in found and not isinstance(found["window"], bool):
        problems.append("time.json: 'window' is not a bool")
    if "redraws" in found and not _is_int(found["redraws"]):
        problems.append("time.json: 'redraws' is not an int")
    for key in ("median_ms", "p95_ms", "slowest_ms"):
        value = found.get(key)
        if key in found and (not _is_num(value) or cast(float, value) < 0):
            problems.append(f"time.json: {key!r} is not ms at least 0")
    return problems


def check_live(out: Path) -> list[str]:
    """<out>/live.json: pace ("display" or "60"), turns ([ms since start, drawn 0/1] pairs),
    steps ({"at": ms due, "posted": ms posted or -1, "what": str} each)."""
    try:
        parsed: Any = json.loads((out / "live.json").read_text())
    except FileNotFoundError:
        return []
    except json.JSONDecodeError as error:
        return [f"live.json: not JSON ({error})"]
    problems: list[str] = []
    if not isinstance(parsed, dict):
        return ["live.json: not an object"]
    found = cast(dict[str, object], parsed)
    for key in ("pace", "turns", "steps"):
        if key not in found:
            problems.append(f"live.json: no {key!r}")
    for key in found:
        if key not in ("pace", "turns", "steps"):
            problems.append(f"live.json: unexpected {key!r}")
    if "pace" in found and found["pace"] not in ("display", "60"):
        problems.append("live.json: 'pace' is not 'display' or '60'")
    turns = found.get("turns")
    if not isinstance(turns, list):
        if "turns" in found:
            problems.append("live.json: 'turns' is not a list")
    else:
        for turn in cast(list[object], turns):
            if not isinstance(turn, list):
                problems.append("live.json: a turn is not [ms, drawn]")
                break
            cells = cast(list[object], turn)
            if len(cells) != 2 or not _is_num(cells[0]) or not _is_int(cells[1]):
                problems.append("live.json: a turn is not [ms, drawn]")
                break
    steps = found.get("steps")
    if not isinstance(steps, list):
        if "steps" in found:
            problems.append("live.json: 'steps' is not a list")
    else:
        for step in cast(list[object], steps):
            if not isinstance(step, dict):
                problems.append("live.json: a step is not {'at', 'posted', 'what'}")
                break
            item = cast(dict[str, object], step)
            if (
                not _is_int(item.get("at"))
                or not _is_num(item.get("posted"))
                or not isinstance(item.get("what"), str)
            ):
                problems.append("live.json: a step is not {'at', 'posted', 'what'}")
                break
    return problems


def check_shapes(out: Path, shots: list[str] | None = None) -> list[str]:
    """<out>'s views.json, motion.json, time.json and live.json against their shapes above: the
    keys, types and units research odin-harness.md section 5 records. views.json must exist when
    the script took shots (the harness writes a line per shot; `shots` the script's shot names,
    else views.json is required); the rest are validated when present (--time and --live runs
    write them, motion.json only when a shot fell mid-motion)."""
    problems = check_motion(out) + check_time(out) + check_live(out)
    if (out / "views.json").exists():
        problems += check_views(out)
    elif shots is None or shots:
        problems.append("no views.json")
    return problems
