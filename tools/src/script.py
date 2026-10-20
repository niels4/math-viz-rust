"""The harness's action language, read for Playwright: the scripts in tools/states that put the
React reference, the web build and the native build in the same state (README: The harness).
crates/mv-script/src/script.rs reads the same language for the native build: change the two
together.

A line is a verb and its arguments, and '#' starts a comment. Lines before the first action say
how the page opens; after it, size, scheme and reduced-motion change it live. Positions are
logical pixels, times milliseconds."""

from dataclasses import dataclass, field
from pathlib import Path

THEMES = (
    "arctic-ice",
    "tron-cyan",
    "midnight-violet",
    "clean-teal",
    "educational-warm",
    "sage-editorial",
)
DARK_THEMES = THEMES[:3]

# The colour spaces the port draws in (plan call 2): a script's `space`, sRGB unless it says p3.
SPACES = ("srgb", "p3")

# The DOM's key names, which Playwright's keyboard takes as they are.
NAMED_KEYS = (
    "ArrowUp",
    "ArrowDown",
    "ArrowLeft",
    "ArrowRight",
    "Enter",
    "Escape",
    "Tab",
    "Backspace",
    "Delete",
    "Home",
    "End",
    "PageUp",
    "PageDown",
    "Space",
    "Shift",
    "Control",
    "Alt",
    "Meta",
)

# The script's modifier words and the keys they hold, in the order they go down.
MODIFIERS = {"shift": "Shift", "ctrl": "Control", "alt": "Alt", "meta": "Meta"}


@dataclass
class Start:
    """How the page opens: React's defaults (state/useAppTheme.ts) on a dark system, the port
    drawing in sRGB (every capture of the reference is sRGB: plan 1.10)."""

    size: tuple[int, int] = (1440, 900)
    density: float = 2
    scheme: str = "dark"
    mode: str = "system"
    dark_theme: str = "arctic-ice"
    light_theme: str = "clean-teal"
    reduced_motion: bool = False
    space: str = "srgb"

    @property
    def theme_shown(self) -> str:
        """The theme the page shows: the pick for the mode's scheme, or the system's in System."""
        scheme = self.scheme if self.mode == "system" else self.mode
        return self.dark_theme if scheme == "dark" else self.light_theme


@dataclass(frozen=True)
class Move:
    to: tuple[float, float]
    over: int = 0  # ms; a step every frame or so


@dataclass(frozen=True)
class Press:
    down: bool


@dataclass(frozen=True)
class Click:
    at: tuple[float, float]
    count: int


@dataclass(frozen=True)
class Wheel:
    delta: tuple[float, float]  # the DOM's pixel deltas, +y scrolling down
    ctrl: bool


@dataclass(frozen=True)
class Pinch:
    scale: float


@dataclass(frozen=True)
class Key:
    key: str  # a DOM key name or one character
    mods: tuple[str, ...]  # modifier words, in the order given


@dataclass(frozen=True)
class Hold:
    key: str
    down: bool


@dataclass(frozen=True)
class Type:
    text: str


@dataclass(frozen=True)
class Wait:
    ms: int


@dataclass(frozen=True)
class Shot:
    name: str


@dataclass(frozen=True)
class Frames:
    name: str
    every: int
    span: int

    def times(self) -> list[int]:
        """The times of the shots, from 0 ms."""
        return list(range(0, self.span + 1, self.every))


@dataclass(frozen=True)
class Video:
    start: bool
    name: str = ""


@dataclass(frozen=True)
class Resize:
    size: tuple[int, int]


@dataclass(frozen=True)
class SchemeChange:
    scheme: str


@dataclass(frozen=True)
class MotionChange:
    reduced: bool


# model's forms, set on the port's model directly (a stand-in for a gesture the port can't make
# yet: only on a `port` line); crates/mv-script/src/script.rs reads them too.
MODEL_FNS = ("x", "x2", "x3", "sin")
MODEL_PARAMS = ("a", "k", "b", "h")
MODEL_PARTS = ("a", "k", "b", "h", "p", "anchor", "stretch", "none")


@dataclass(frozen=True)
class Model:
    text: str  # the line as the port reads it, "model ..."


Verb = (
    Move
    | Press
    | Click
    | Wheel
    | Pinch
    | Key
    | Hold
    | Type
    | Wait
    | Shot
    | Frames
    | Video
    | Resize
    | SchemeChange
    | MotionChange
    | Model
)

# What a `ref` or `port` line may hold: an input, or a model line.
GESTURES = (Move, Press, Click, Wheel, Pinch, Key, Hold, Type, Model)

# The surfaces an action runs on: every one, the React reference alone (`ref LINE`), or the web
# and native builds alone (`port LINE`). Where the port can't yet make a gesture the reference
# makes, a state reaches the same point on each side by its own means: the reference by the
# gesture, the port by `model` lines setting what the gesture leaves.
SURFACES = ("all", "ref", "port")


@dataclass
class Action:
    line: int
    verb: Verb
    surface: str = "all"

    def runs_on(self, target: str) -> bool:
        """Whether the action runs on a target (ref, web or native)."""
        if self.surface == "ref":
            return target == "ref"
        if self.surface == "port":
            return target != "ref"
        return True


@dataclass
class Script:
    path: Path
    start: Start = field(default_factory=Start)
    actions: list[Action] = field(default_factory=lambda: [])

    @property
    def real_time(self) -> bool:
        """A script that records video runs on the real clock; the rest on a paused one."""
        return any(isinstance(a.verb, Video) for a in self.actions)

    def shot_names(self) -> list[str]:
        names: list[str] = []
        for action in self.actions:
            match action.verb:
                case Shot(name):
                    names.append(name)
                case Frames() as frames:
                    names += [f"{frames.name}-{t:04d}" for t in frames.times()]
                case _:
                    pass
        return names


class ScriptError(Exception):
    def __init__(self, path: Path, line: int, message: str) -> None:
        super().__init__(f"{path}:{line}: {message}")


class _LineError(Exception):
    pass


def load(path: Path) -> Script:
    return parse(path.read_text(), path)


def parse(text: str, path: Path = Path("<script>")) -> Script:
    script = Script(path)
    started = False
    for number, line in enumerate(text.splitlines(), start=1):
        surface, body = _surface(line)
        try:
            if surface == "all":
                verb = _line(script.start, started, body, surface)
            else:
                # A qualified line is an action: the page opens alike on every surface.
                scratch = Start(**vars(script.start))
                verb = _line(scratch, started, body, surface)
                if not isinstance(verb, GESTURES):
                    raise _LineError(
                        "ref and port take an input or a model line: time, shots and the page's "
                        "size and media run on every surface"
                    )
        except _LineError as failure:
            raise ScriptError(path, number, str(failure)) from None
        if verb is not None:
            started = True
            script.actions.append(Action(number, verb, surface))
    return script


def _surface(line: str) -> tuple[str, str]:
    """The surfaces a line runs on, and the line without its `ref` or `port`."""
    trimmed = line.lstrip()
    for word in ("ref", "port"):
        rest = trimmed[len(word) :]
        if trimmed.startswith(word) and (rest == "" or rest[0] in " \t"):
            return word, rest
    return "all", line


def _line(start: Start, started: bool, text: str, surface: str = "all") -> Verb | None:
    line = text.strip()
    end = next((i for i, ch in enumerate(line) if ch in " \t"), len(line))
    word, args = line[:end], line[end:]
    if word == "" or word.startswith("#"):
        if surface != "all":
            raise _LineError("ref and port take an action")
        return None
    if word == "type":
        return _type(args)
    fields = args.split()
    for i, item in enumerate(fields):
        if item.startswith("#"):
            fields = fields[:i]
            break

    match word:
        case "size":
            w, h = _ints2(fields)
            if w <= 0 or h <= 0:
                raise _LineError("size takes a width and a height above 0")
            if started:
                return Resize((w, h))
            start.size = (w, h)
        case "scheme":
            scheme = _one_of(fields, ("dark", "light"), "scheme")
            if started:
                return SchemeChange(scheme)
            start.scheme = scheme
        case "reduced-motion":
            on = _one_of(fields, ("on", "off"), "reduced-motion") == "on"
            if started:
                return MotionChange(on)
            start.reduced_motion = on
        case "density":
            _before_actions(started, word)
            (density,) = _floats(fields, 1)
            if density <= 0:
                raise _LineError("density takes a number above 0")
            start.density = density
        case "mode":
            _before_actions(started, word)
            start.mode = _one_of(fields, ("system", "light", "dark"), "mode")
        case "theme":
            _before_actions(started, word)
            theme = _one_of(fields, THEMES, "theme")
            if theme in DARK_THEMES:
                start.dark_theme = theme
            else:
                start.light_theme = theme
        case "space":
            _before_actions(started, word)
            start.space = _one_of(fields, SPACES, "space")
        case "move":
            if len(fields) != 2 and not (len(fields) == 4 and fields[2] == "over"):
                raise _LineError("move takes X Y [over MS]")
            x, y = _floats(fields[:2], 2)
            return Move((x, y), _ms(fields[3]) if len(fields) == 4 else 0)
        case "down" | "up":
            if fields:
                raise _LineError(f"{word} takes nothing")
            return Press(word == "down")
        case "click" | "dblclick":
            x, y = _floats(fields, 2)
            return Click((x, y), 1 if word == "click" else 2)
        case "wheel":
            if len(fields) != 2 and not (len(fields) == 3 and fields[2] == "ctrl"):
                raise _LineError("wheel takes DX DY [ctrl]")
            dx, dy = _floats(fields[:2], 2)
            return Wheel((dx, dy), len(fields) == 3)
        case "pinch":
            (scale,) = _floats(fields, 1)
            if scale <= 0:
                raise _LineError("pinch takes a scale above 0")
            return Pinch(scale)
        case "key" | "tab":
            key, words = "Tab", fields
            if word == "key":
                if not fields:
                    raise _LineError("key takes a key name, then modifiers")
                key, words = _key(fields[0]), fields[1:]
            mods: list[str] = []
            for name in words:
                modifier = _one_of([name], tuple(MODIFIERS), "a modifier")
                if modifier not in mods:
                    mods.append(modifier)
            order = list(MODIFIERS)
            return Key(key, tuple(sorted(mods, key=order.index)))
        case "hold" | "release":
            if len(fields) != 1:
                raise _LineError(f"{word} takes a key name")
            return Hold(_key(fields[0]), word == "hold")
        case "wait":
            if len(fields) != 1:
                raise _LineError("wait takes MS")
            return Wait(_ms(fields[0]))
        case "shot":
            if len(fields) != 1:
                raise _LineError("shot takes a name")
            return Shot(_file_name(fields[0]))
        case "frames":
            if len(fields) != 5 or fields[1] != "every" or fields[3] != "for":
                raise _LineError("frames takes NAME every MS for MS")
            name, every, span = _file_name(fields[0]), _ms(fields[2]), _ms(fields[4])
            if every == 0:
                raise _LineError("frames takes a step above 0 ms")
            return Frames(name, every, span)
        case "video":
            if fields == ["stop"]:
                return Video(False)
            if not fields or len(fields) > 2 or fields[0] != "start":
                raise _LineError("video takes start [NAME] or stop")
            return Video(True, _file_name(fields[1]) if len(fields) == 2 else "video")
        case "model":
            if surface != "port":
                raise _LineError("model sets the port's model: it goes on a port line")
            return _model(fields)
        case _:
            raise _LineError(f'unknown verb "{word}"')
    return None


def _model(fields: list[str]) -> Model:
    if not fields:
        raise _LineError("model takes fn, param, p, drag or p-before")
    what = _one_of(fields[:1], ("fn", "param", "p", "drag", "p-before"), "model")
    args = fields[1:]
    match what:
        case "fn":
            _one_of(args, MODEL_FNS, "model fn")
        case "param":
            if len(args) != 2:
                raise _LineError("model param takes a parameter and a value")
            _one_of(args[:1], MODEL_PARAMS, "model param")
            _floats(args[1:], 1)
        case "p":
            _floats(args, 1)
        case "drag":
            _one_of(args, MODEL_PARTS, "model drag")
        case _:
            if args != ["none"]:
                _floats(args, 2)
    return Model(" ".join(["model", *fields]))


def _type(text: str) -> Type:
    args = text.strip()
    if not args.startswith('"'):
        raise _LineError('type takes "TEXT"')
    end = args.find('"', 1)
    if end < 0:
        raise _LineError("type: no closing quote")
    after = args[end + 1 :].strip()
    if after and not after.startswith("#"):
        raise _LineError("type: words after the closing quote")
    return Type(args[1:end])


def _before_actions(started: bool, word: str) -> None:
    if started:
        raise _LineError(f"{word} goes before the first action: the page opens with it")


def _key(word: str) -> str:
    if word in NAMED_KEYS or (len(word) == 1 and word > " "):
        return word
    raise _LineError(f'unknown key "{word}": a DOM key name such as ArrowUp, or one character')


def _one_of(fields: list[str], words: tuple[str, ...], what: str) -> str:
    if len(fields) == 1 and fields[0] in words:
        return fields[0]
    raise _LineError(f"{what} takes one of {', '.join(words)}")


def _ints2(fields: list[str]) -> tuple[int, int]:
    try:
        if len(fields) == 2:
            return int(fields[0]), int(fields[1])
    except ValueError:
        pass
    raise _LineError("expected two whole numbers")


def _floats(fields: list[str], count: int) -> list[float]:
    if len(fields) != count:
        raise _LineError(f"expected {count} numbers")
    values: list[float] = []
    for item in fields:
        try:
            values.append(float(item))
        except ValueError:
            raise _LineError(f'"{item}" is not a number') from None
    return values


def _ms(item: str) -> int:
    try:
        value = int(item)
    except ValueError:
        value = -1
    if value < 0:
        raise _LineError(f'"{item}" is not a whole number of milliseconds')
    return value


def _file_name(word: str) -> str:
    allowed = set("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_.")
    if not set(word) <= allowed:
        raise _LineError(f"\"{word}\": a name takes letters, digits, '-', '_' and '.'")
    if word == "" or word.startswith("."):
        raise _LineError(f'"{word}" is not a file name')
    return word
