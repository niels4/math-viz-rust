"""`tools/run gamut` (plan 1.10, call 2): the port's colour in Display P3. Each state is drawn
natively twice, in sRGB as `tools/run states` draws it and in Display P3 (the script with
`space p3`), and in Display P3 by the web build in each engine (Firefox too: the word wins over
its rule). Then, shot by shot:

- every pixel whose 3 x 3 neighbourhood is flat in the sRGB frame is held, in the P3 frame, to the
  P3 conversion of its sRGB pixel (the well-conditioned direction: a P3 capture is never converted
  back to sRGB to be compared): within 1/255 on a theme colour, within 3/255 where a translucent
  fill blends (blended in P3 instead of sRGB); a colour outside sRGB (Arctic Ice's a, Clean
  Teal's and Sage Editorial's k) is held to its oklch's own P3 bytes, and its bands and plates to
  their blend of those bytes, as is a translucent colour inside sRGB drawn over one of those
  blends (the focus ring over a lit row's band) or over a theme colour (the ring over the top bar:
  in Tron Cyan and Clean Teal it lies up to 6.5/255 from the conversion, M12); edge pixels are
  left out and counted;
- Chromium's and Firefox's P3 shots against the native P3 frame at threshold 0, since their
  screenshots pass a tagged buffer's bytes through (Chromium's SwiftShader edges apart, as ever);
- WebKit's P3 shots, which it converts to sRGB, against the harness's own sRGB conversion of the
  native P3 frame, within 1/255: both convert the same bytes;
- a web shot's pixel off native's is counted apart only by a row of ALLOWANCES, each bounded by
  engine, by whether the shot was taken mid-motion, by region and by level: where the engine's own
  sRGB shot differs from native's too (no colour's doing: Chromium's half-delta wheel, say), a
  level off beside those (an edge's pixel whose sRGB byte matched native's by chance), Chromium's
  anti-aliased edges, and the two mid-motion levels below;
- in a shot taken while anything moved (the native harness's motion.json, M10), a translucent part
  is drawn at an opacity no theme colour has (Q's label fading in, a term's flash fading over its
  plate), so blends are counted but not held there; the theme's colours and the colours outside
  sRGB are held as ever. There Chromium's SwiftShader may composite a fading layer a level apart,
  and WebKit samples a scaled layer (an overlay's opening or closing, M11) a level apart, as its
  sRGB shot shows on a few hundred of the layer's pixels: with the conversion's level, 2/255, and
  only on anti-aliased edges in the box of those pixels.

Native P3 shots carry the Display P3 profile as the app writes them; Chromium's and Firefox's get
it here, their bytes being P3. Output: <out>/<state>/<surface>/, <out>/report.json."""

import json
import math
from dataclasses import asdict, dataclass, field, replace
from pathlib import Path
from typing import cast

import numpy as np
from numpy.typing import NDArray
from PIL import Image

from act import Run, run, shown
from colour import P3_ICC, Bytes, Colour, convert, theme_colours, to_bytes
from script import Frames, SchemeChange, Script, Shot, load
from states import STATES

Mask = NDArray[np.bool_]
RGB = tuple[float, float, float]

# Where a flat pixel's expected P3 value comes from, and how far it may be from it, of 255.
THEME_COLOUR = "theme colour"  # a colour of the theme at full alpha, inside sRGB: its conversion
OUTSIDE = "outside sRGB"  # a colour outside sRGB at full alpha: its oklch's P3 bytes, exactly
OUTSIDE_BLEND = "blend outside sRGB"  # a band or plate of one over a theme colour, blended in P3
# A held modifier's badge over its row's band: two translucent fills, each composited as the port
# composites the page's (css_background: premultiplied in 8 bits, the frame rounding between them),
# in the space drawn. A blend of a blend strays furthest from the P3 conversion of its sRGB pixel
# (Snap's --warning over k's band: 3.9/255 in Arctic Ice), so it is predicted, not allowed for.
STACKED = "badge over a band"
# A translucent colour inside sRGB (the focus ring, a plate, a band) over a theme colour at full
# alpha, blended in P3 as NanoVG blends it: a saturated primary at 32 % (Tron Cyan's, Clean Teal's
# ring) lies up to 6.5/255 from the P3 conversion of its sRGB blend (M12), so it is predicted.
OVER = "translucent over a theme colour"
BLEND = "blend"  # anything else flat: a translucent fill blended in P3 instead of sRGB
TOLERANCE = {
    THEME_COLOUR: 1.0,
    OUTSIDE: 0.0,
    OUTSIDE_BLEND: 1.0,
    STACKED: 1.0,
    OVER: 1.0,
    BLEND: 3.0,
}


@dataclass(frozen=True)
class Expectation:
    kind: str
    p3: RGB  # the expected P3 value, 0..255
    colour: str = ""  # OUTSIDE and OUTSIDE_BLEND: which colour
    srgb: RGB | None = None  # OUTSIDE_BLEND: the sRGB blend the sRGB frame holds


@dataclass
class Seen:
    """A colour outside sRGB, or a blend of one, where a shot shows it."""

    pixels: int
    expected: list[float]
    drawn: list[list[int]]  # the distinct P3 values drawn


@dataclass
class Kind:
    """A shot's flat pixels of one kind against their expected P3 values."""

    pixels: int = 0
    worst: float = 0  # the largest channel difference, of 255
    worst_at: tuple[int, int] | None = None  # device px
    over: int = 0  # pixels beyond the kind's tolerance
    seen: dict[str, Seen] = field(default_factory=lambda: {})


# What a frame taken mid-motion still holds: its colours at full alpha.
HELD_IN_MOTION = (THEME_COLOUR, OUTSIDE)


@dataclass
class Flat:
    """A P3 frame's flat pixels against the P3 conversion of the sRGB frame's."""

    width: int
    height: int
    flat: int
    edges: int
    kinds: dict[str, Kind]
    moving: bool = False  # taken mid-motion: blends counted, not held

    @property
    def ok(self) -> bool:
        return all(
            k.over == 0
            for name, k in self.kinds.items()
            if not self.moving or name in HELD_IN_MOTION
        )


@dataclass
class Versus:
    """A web build's P3 shot against the native P3 frame: the pixels off it, those an allowance
    explains (by its cause, ALLOWANCES' order), and the rest."""

    engine: str
    how: str  # "exact" (Chromium, Firefox) or "converted" (WebKit's shot, converted to sRGB)
    differ: int  # pixels beyond the comparison's level: 0 when exact, 1/255 when converted
    by_one: int  # converted: pixels 1/255 off
    max: int
    explained: dict[str, int]  # the differing pixels each allowance takes, by its cause
    unexplained: int  # the rest
    bounds: tuple[int, int, int, int] | None
    moving: bool = False  # the shot was taken mid-motion

    @property
    def ok(self) -> bool:
        return self.unexplained == 0


# Where in a shot an allowance applies, from the engine's own sRGB shot against native's.
SRGB_DIFFERS = "srgb differs"  # where the engine's sRGB shot differs from native's
BESIDE = "beside"  # a pixel beside those whose own sRGB byte is native's
EDGES = "edges"  # anti-aliased edges of the native P3 frame (not flat)
ANYWHERE = "anywhere"
# The anti-aliased edges in the box of the sRGB differences, SRGB_BOX_MARGIN px wider each way: a
# scaled layer's sampling changes a pixel only where its texels differ (a flat area samples to
# itself), and its sRGB differences sample the layer sparsely (8 to 226 of the explainer card's
# pixels, M11), its P3 shot's landing on others, up to 5 px past their box in M11's states.
EDGES_IN_SRGB_BOX = "edges in srgb box"
SRGB_BOX_MARGIN = 8
# The anti-aliased edges inside a box the native frame composited a scaled layer into (views.json's
# "scaled", the settings panel's opening and closing, M12), SCALED_BOX_MARGIN px wider each way for
# the layer's own anti-aliased rim.
EDGES_IN_SCALED_BOX = "edges in scaled box"
SCALED_BOX_MARGIN = 2


@dataclass(frozen=True)
class Allowance:
    """A difference from native the engine's own drawing explains, not a colour: its cause, the
    engines it applies in, whether only in a shot taken mid-motion, where in the shot, and how
    many levels past the comparison's own a pixel may be (None: any; the comparison allows 0 in
    Chromium and Firefox, whose P3 bytes are compared as they are, and 1 in WebKit, which converts
    its P3 shot to sRGB)."""

    cause: str
    engines: tuple[str, ...]
    moving_only: bool
    region: str
    levels: int | None
    # WebKit: instead of `levels`, the pixel is within the comparison's level of the conversion of
    # some P3 value up to this many levels from native's in each channel (a scaled layer sampled a
    # level apart in P3, then converted: near a zero channel of a saturated colour one P3 level
    # makes several sRGB ones, plan section 0's ill-conditioning).
    p3_levels: int = 0


ENGINES = ("webkit", "chromium", "firefox")

# A differing pixel takes the first row that fits it; any left over fails the shot.
# The allowances start empty: each milestone records the rows its judgment needs, in the
# order they apply, with the measurement behind each.
ALLOWANCES: tuple[Allowance, ...] = ()


def p3_script(script: Script, out: Path) -> Script:
    """The state's script with `space p3` before its first line, written beside its runs."""
    out.mkdir(parents=True, exist_ok=True)
    path = out / f"{script.path.stem}.p3.act"
    path.write_text(
        f"# tools/run gamut: {shown(script.path)} drawn in Display P3\nspace p3\n"
        + script.path.read_text()
    )
    return load(path)


def themes_at_shots(script: Script) -> dict[str, str]:
    """The theme each shot shows: the start's, following the system's scheme in System mode."""
    start = script.start
    scheme = start.scheme
    shots: dict[str, str] = {}
    for action in script.actions:
        mode = scheme if start.mode == "system" else start.mode
        theme = start.dark_theme if mode == "dark" else start.light_theme
        match action.verb:
            case SchemeChange(changed):
                scheme = changed
            case Shot(name):
                shots[name] = theme
            case Frames(name) as frames:
                for t in frames.times():
                    shots[f"{name}-{t:04d}"] = theme
            case _:
                pass
    return shots


def shown_themes(native_out: Path) -> dict[str, str]:
    """The theme each shot showed, as the native harness recorded it (its views.json, "theme"): a
    pick in the settings panel (M12) changes it mid-script, which the script's start doesn't say."""
    path = native_out / "views.json"
    if not path.exists():
        return {}
    shots: dict[str, str] = {}
    for line in path.read_text().splitlines():
        view = cast(dict[str, object], json.loads(line))
        if isinstance(view.get("theme"), str):
            shots[str(view["shot"])] = str(view["theme"])
    return shots


def scaled_boxes(native_out: Path, density: float) -> dict[str, list[tuple[int, int, int, int]]]:
    """The boxes each shot's frame composited a scaled layer into (views.json's "scaled", logical
    px), in device px, rounded out to whole pixels."""
    path = native_out / "views.json"
    if not path.exists():
        return {}
    shots: dict[str, list[tuple[int, int, int, int]]] = {}
    for line in path.read_text().splitlines():
        view = cast(dict[str, object], json.loads(line))
        if "shot" not in view:
            # A standalone `url` line (the shell's contract: written whenever
            # the page query changes): no frame, no boxes.
            continue
        boxes: list[tuple[int, int, int, int]] = []
        for b in cast(list[list[float]], view.get("scaled") or []):
            x0, y0 = math.floor(b[0] * density), math.floor(b[1] * density)
            x1, y1 = math.ceil((b[0] + b[2]) * density), math.ceil((b[1] + b[3]) * density)
            boxes.append((x0, y0, x1 - x0, y1 - y0))
        shots[str(view["shot"])] = boxes
    return shots


def rgb_array(path: Path) -> Bytes:
    with Image.open(path) as image:
        return np.asarray(image.convert("RGB"), dtype=np.uint8)


def flat_mask(frame: Bytes) -> Mask:
    """True where a pixel and its eight neighbours are one colour; never on the border."""
    h, w, _ = frame.shape
    inner = frame[1 : h - 1, 1 : w - 1]
    flat = np.ones((h - 2, w - 2), dtype=np.bool_)
    for dy in (-1, 0, 1):
        for dx in (-1, 0, 1):
            if dy or dx:
                flat &= np.all(frame[1 + dy : h - 1 + dy, 1 + dx : w - 1 + dx] == inner, axis=2)
    mask = np.zeros((h, w), dtype=np.bool_)
    mask[1 : h - 1, 1 : w - 1] = flat
    return mask


def packed(pixels: Bytes) -> NDArray[np.uint32]:
    p = pixels.astype(np.uint32)
    return ((p[..., 0] << 16) | (p[..., 1] << 8) | p[..., 2]).astype(np.uint32)


def to_p3(rgb: tuple[int, int, int]) -> RGB:
    v = convert(np.array(rgb, dtype=np.uint8), "srgb", "p3")
    return float(v[0]), float(v[1]), float(v[2])


def expectations(theme: str) -> tuple[dict[tuple[int, int, int], Expectation], list[Expectation]]:
    """What a flat sRGB pixel of `theme` is expected to be in P3: by value for the theme's opaque
    colours; and the blends of its translucent colours outside sRGB (bands, plates) as NanoVG
    blends them in either space, over each opaque colour and over one such blend (a lit row's band
    under a tween's), and of its translucent colours inside sRGB (the focus ring, a plate) over one
    such blend (the ring round a box on a lit row), found by nearness in sRGB."""
    colours = theme_colours(theme)
    opaque: dict[tuple[int, int, int], Expectation] = {}
    bases: list[Base] = []
    for c in colours:
        if c.alpha != 1:
            continue
        r, g, b = c.p3_bytes
        bases.append(Base(c.name, c.srgb, (r, g, b)))
        if c.outside_srgb:
            opaque[c.srgb] = Expectation(OUTSIDE, (float(r), float(g), float(b)), c.name)
        elif c.srgb not in opaque:
            opaque[c.srgb] = Expectation(THEME_COLOUR, to_p3(c.srgb))
    translucent = [t for t in colours if 0 < t.alpha < 1 and t.outside_srgb]
    once = [blend(t, under) for t in translucent for under in bases]
    twice = [blend(t, drawn(under)) for t in translucent for under in once]
    inside = [t for t in colours if 0 < t.alpha < 1 and not t.outside_srgb]
    over = [blend(t, drawn(under)) for t in inside for under in once]
    over_theme = [replace(blend(t, under), kind=OVER) for t in inside for under in bases]
    # A row's band lies on its sub card (--section), and a held modifier's badge on the band.
    bands = [t for t in colours if t.name.startswith("--param-") and t.name.endswith("-18")]
    badges = [t for t in colours if t.name.startswith("badge-")]
    cards = [b for b in bases if b.name == "--section"]
    stacked = [stack(badge, band, card) for badge in badges for band in bands for card in cards]
    return opaque, once + twice + over + stacked + over_theme


def composite(colour: RGB, alpha: float, under: RGB) -> RGB:
    """A translucent fill over a pixel as the port draws the page's (css_background, WebKit's
    compositing): its alpha on the nearest of 255 steps, each channel premultiplied and rounded to
    8 bits, then blended by the GPU over the pixel; 0..255, before the frame rounds it."""
    a = math.floor(alpha * 255 + 0.5)
    out = [math.floor(colour[i] * a / 255 + 0.5) + under[i] * (1 - a / 255) for i in range(3)]
    return out[0], out[1], out[2]


def stack(badge: Colour, band: Colour, under: Base) -> Expectation:
    """A badge's fill over a band's over `under`, each composited in turn (the frame rounding the
    band before the badge goes over it): in sRGB from the tokens' sRGB pixels, in P3 from their
    oklch's P3 values over under's P3 pixel."""

    def rounded(v: RGB) -> RGB:
        return (
            float(math.floor(v[0] + 0.5)),
            float(math.floor(v[1] + 0.5)),
            float(math.floor(v[2] + 0.5)),
        )

    def floats(v: tuple[int, int, int]) -> RGB:
        return float(v[0]), float(v[1]), float(v[2])

    srgb_band = rounded(composite(floats(band.srgb), band.alpha, floats(under.srgb)))
    p3_band = rounded(composite(band.p3, band.alpha, floats(under.p3)))
    return Expectation(
        STACKED,
        composite(badge.p3, badge.alpha, p3_band),
        f"{badge.name} over {band.name} over {under.name}",
        composite(floats(badge.srgb), badge.alpha, srgb_band),
    )


@dataclass(frozen=True)
class Base:
    """What a translucent fill is drawn over: its pixel in the sRGB frame and in the P3 frame."""

    name: str
    srgb: tuple[int, int, int]
    p3: tuple[int, int, int]


def blend(t: Colour, under: Base) -> Expectation:
    """`t` over `under` as NanoVG blends it in the frame's encoded values: in sRGB t's clip over
    under's sRGB pixel, in P3 t's oklch over under's P3 pixel."""
    a = t.alpha
    srgb = [t.srgb[i] * a + under.srgb[i] * (1 - a) for i in range(3)]
    p3 = [t.p3[i] * a + under.p3[i] * (1 - a) for i in range(3)]
    return Expectation(
        OUTSIDE_BLEND,
        (p3[0], p3[1], p3[2]),
        f"{t.name} over {under.name}",
        (srgb[0], srgb[1], srgb[2]),
    )


def drawn(e: Expectation) -> Base:
    """A predicted blend as the frames hold it, to draw another over."""
    assert e.srgb is not None
    srgb = [int(v) for v in to_bytes(np.array(e.srgb))]
    p3 = [int(v) for v in to_bytes(np.array(e.p3))]
    return Base(e.colour, (srgb[0], srgb[1], srgb[2]), (p3[0], p3[1], p3[2]))


def blend_for(rgb: tuple[int, int, int], blends: list[Expectation]) -> Expectation | None:
    """The predicted blend whose sRGB value lies within 1/255 of `rgb` in each channel, nearest."""
    best: tuple[float, Expectation] | None = None
    for e in blends:
        assert e.srgb is not None
        off = max(abs(rgb[i] - e.srgb[i]) for i in range(3))
        if off <= 1 and (best is None or off < best[0]):
            best = (off, e)
    return None if best is None else best[1]


def flat_check(srgb: Bytes, p3: Bytes, theme: str, moving: bool = False) -> Flat:
    h, w, _ = srgb.shape
    mask = flat_mask(srgb)
    ys, xs = np.nonzero(mask)
    values = p3[mask].astype(np.float64)
    unique, inverse = np.unique(packed(srgb[mask]), return_inverse=True)
    opaque, blends = expectations(theme)
    found: list[Expectation] = []
    for key in unique.tolist():
        rgb = ((key >> 16) & 255, (key >> 8) & 255, key & 255)
        found.append(opaque.get(rgb) or blend_for(rgb, blends) or Expectation(BLEND, to_p3(rgb)))
    expected = np.array([e.p3 for e in found])[inverse]
    tolerance = np.array([TOLERANCE[e.kind] for e in found])[inverse]
    off = np.abs(values - expected).max(axis=1)
    order = np.argsort(inverse, kind="stable")
    starts = np.searchsorted(inverse[order], np.arange(len(found) + 1))
    kinds: dict[str, Kind] = {}
    for i, e in enumerate(found):
        rows = order[starts[i] : starts[i + 1]]
        k = kinds.setdefault(e.kind, Kind())
        k.pixels += len(rows)
        worst = rows[np.argmax(off[rows])]
        if k.worst_at is None or off[worst] > k.worst:
            k.worst, k.worst_at = float(off[worst]), (int(xs[worst]), int(ys[worst]))
        k.over += int(np.count_nonzero(off[rows] > tolerance[rows]))
        if e.kind in (OUTSIDE, OUTSIDE_BLEND, STACKED):
            drawn = np.unique(values[rows].astype(np.uint8), axis=0).tolist()
            seen = k.seen.setdefault(e.colour, Seen(0, [round(float(v), 3) for v in e.p3], []))
            seen.pixels += len(rows)
            seen.drawn = sorted({*map(tuple, seen.drawn), *map(tuple, drawn)})  # type: ignore[arg-type]
    return Flat(w, h, int(mask.sum()), h * w - int(mask.sum()), kinds, moving)


def moving_shots(native_out: Path) -> set[str]:
    """The shots the native harness took while anything moved (its motion.json)."""
    path = native_out / "motion.json"
    if not path.exists():
        return set()
    return set(cast(list[str], json.loads(path.read_text())["moving"]))


def tag_p3(path: Path) -> None:
    """Gives a PNG of P3 bytes the Display P3 profile, its pixels untouched."""
    with Image.open(path) as image:
        image.load()
        image.save(path, icc_profile=P3_ICC.read_bytes())


def versus(
    native_p3: Bytes,
    web_p3: Bytes,
    native_srgb: Bytes,
    web_srgb: Bytes,
    engine: str,
    moving: bool = False,
    scaled: list[tuple[int, int, int, int]] | None = None,
) -> Versus:
    srgb_differs = np.any(web_srgb != native_srgb, axis=2)
    if engine == "webkit":
        unique, inverse = np.unique(packed(native_p3).reshape(-1), return_inverse=True)
        colours = np.stack([(unique >> 16) & 255, (unique >> 8) & 255, unique & 255], axis=1)
        converted = to_bytes(convert(colours.astype(np.uint8), "p3", "srgb"))[inverse]
        off = np.abs(web_p3.reshape(-1, 3).astype(np.int16) - converted.astype(np.int16)).max(
            axis=1
        )
        off = off.reshape(native_p3.shape[:2])
        how, base, by_one = "converted", 1, int((off == 1).sum())
    else:
        off = np.abs(web_p3.astype(np.int16) - native_p3.astype(np.int16)).max(axis=2)
        how, base, by_one = "exact", 0, 0
    differs = off > base

    def region(name: str) -> Mask:
        if name == SRGB_DIFFERS:
            return srgb_differs
        if name == BESIDE:
            return grown(srgb_differs, 1) & ~srgb_differs
        if name == EDGES:
            return ~flat_mask(native_p3)
        if name == EDGES_IN_SRGB_BOX:
            return box_of(srgb_differs, SRGB_BOX_MARGIN) & ~flat_mask(native_p3)
        if name == EDGES_IN_SCALED_BOX:
            inside = np.zeros_like(srgb_differs)
            for x, y, w, h in scaled or []:
                m = SCALED_BOX_MARGIN
                inside[max(y - m, 0) : y + h + m, max(x - m, 0) : x + w + m] = True
            return inside & ~flat_mask(native_p3)
        return np.ones_like(srgb_differs)

    left = differs.copy()
    explained: dict[str, int] = {}
    for allowance in ALLOWANCES:
        if engine not in allowance.engines or (allowance.moving_only and not moving):
            continue
        fits = left & region(allowance.region)
        if allowance.levels is not None:
            fits &= off <= base + allowance.levels
        if allowance.p3_levels:
            fits &= sampled(native_p3, web_p3, fits, allowance.p3_levels, base)
        explained[allowance.cause] = int(fits.sum())
        left &= ~fits
    return Versus(
        engine,
        how,
        int(differs.sum()),
        by_one,
        int(off.max()),
        explained,
        int(left.sum()),
        bounds(differs),
        moving,
    )


def sampled(native_p3: Bytes, web: Bytes, where: Mask, levels: int, base: int) -> Mask:
    """Where WebKit's (sRGB) pixel is within `base` of the conversion of some P3 value up to
    `levels` from native's in each channel."""
    out = np.zeros_like(where)
    ys, xs = np.nonzero(where)
    if len(ys) == 0:
        return out
    steps = np.arange(-levels, levels + 1)
    offsets = np.stack(np.meshgrid(steps, steps, steps, indexing="ij"), axis=-1).reshape(-1, 3)
    near = native_p3[ys, xs].astype(np.int16)[:, None, :] + offsets[None, :, :]
    near = np.clip(near, 0, 255).astype(np.uint8)
    converted = to_bytes(convert(near.reshape(-1, 3), "p3", "srgb")).reshape(near.shape)
    off = np.abs(converted.astype(np.int16) - web[ys, xs].astype(np.int16)[:, None, :]).max(axis=2)
    out[ys, xs] = (off <= base).any(axis=1)
    return out


def grown(mask: Mask, by: int) -> Mask:
    """The mask grown by `by` px each way: true where a pixel within `by` of it in x and y is."""
    out = mask.copy()
    for _ in range(by):
        rows = out.copy()
        rows[1:] |= out[:-1]
        rows[:-1] |= out[1:]
        out = rows.copy()
        out[:, 1:] |= rows[:, :-1]
        out[:, :-1] |= rows[:, 1:]
    return out


def box_of(mask: Mask, margin: int) -> Mask:
    """The box of the mask's pixels, `margin` px wider each way; none when the mask is empty."""
    out = np.zeros_like(mask)
    if (b := bounds(mask)) is not None:
        x, y, w, h = b
        out[max(y - margin, 0) : y + h + margin, max(x - margin, 0) : x + w + margin] = True
    return out


def bounds(mask: Mask) -> tuple[int, int, int, int] | None:
    ys, xs = np.nonzero(mask)
    if len(xs) == 0:
        return None
    return int(xs.min()), int(ys.min()), int(xs.max() - xs.min() + 1), int(ys.max() - ys.min() + 1)


def describe(flat: Flat, density: float) -> list[str]:
    lines = [
        f"{flat.flat:,} flat pixels of {flat.width * flat.height:,} "
        f"({flat.edges:,} edge pixels left out)"
    ]
    for name in (THEME_COLOUR, OUTSIDE, OUTSIDE_BLEND, STACKED, OVER, BLEND):
        if (k := flat.kinds.get(name)) is None:
            continue
        at = ""
        if k.worst_at is not None:
            x, y = k.worst_at
            at = f" at ({x}, {y}), logical ({x / density:.1f}, {y / density:.1f})"
        verdict = "ok" if k.over == 0 else f"{k.over:,} BEYOND {TOLERANCE[name]:g}/255"
        if k.over and flat.moving and name not in HELD_IN_MOTION:
            verdict = f"{k.over:,} beyond {TOLERANCE[name]:g}/255, not held: taken mid-motion"
        lines.append(
            f"{name}: {k.pixels:,} pixels within {k.worst:.3f}/255 of the expected{at}: {verdict}"
        )
        for colour, seen in k.seen.items():
            lines.append(
                f"  {colour}: {seen.pixels:,} pixels drawn {seen.drawn}, expected {seen.expected}"
            )
    return lines


def describe_web(v: Versus) -> str:
    verdict = "ok" if v.ok else f"{v.unexplained:,} UNEXPLAINED"
    if v.how == "converted":
        head = (
            f"web-p3-{v.engine} (converted to sRGB by the engine) against the harness's sRGB "
            f"conversion of native P3: {v.differ:,} pixels beyond 1/255, {v.by_one:,} at 1/255, "
            f"max {v.max}/255"
        )
    else:
        head = (
            f"web-p3-{v.engine} against native P3 at threshold 0: {v.differ:,} pixels differ, "
            f"max {v.max}/255"
        )
    if v.bounds:
        head += f", all within {v.bounds}"
    if v.differ:
        taken = [f"{n:,} {cause}" for cause, n in v.explained.items() if n]
        head += "; " + (", ".join(taken) if taken else "none explained")
    return f"{head}: {verdict}"


def run_gamut(names: list[str], engines: list[str], out: Path) -> int:
    report: dict[str, object] = {}
    failed = False
    for name in names:
        script = load(STATES / f"{name}.act")
        state_dir = out / name
        print(f"{name} ({shown(script.path)}):")
        if script.start.space != "srgb":
            print("  its script sets a space; gamut draws the state in both")
            failed = True
            continue
        p3 = p3_script(script, state_dir)
        runs: dict[str, Run] = {
            "native-srgb": run(script, "native", "", state_dir / "native-srgb"),
            "native-p3": run(p3, "native", "", state_dir / "native-p3"),
        }
        for engine in engines:
            runs[f"web-srgb-{engine}"] = run(
                script, "web", engine, state_dir / f"web-srgb-{engine}"
            )
            result = run(p3, "web", engine, state_dir / f"web-p3-{engine}")
            if engine != "webkit":
                for shot in result.shots:
                    tag_p3(result.out / f"{shot}.png")
            runs[f"web-p3-{engine}"] = result
        for surface, result in runs.items():
            print(f"  {surface}: {result.summary()}")
            for problem in result.problems:
                print(f"    {problem}")
            failed |= not result.ok
        themes = themes_at_shots(script) | shown_themes(runs["native-srgb"].out)
        moving = moving_shots(runs["native-p3"].out)
        scaled = scaled_boxes(runs["native-p3"].out, script.start.density)
        shots: dict[str, object] = {}
        for shot in script.shot_names():
            paths = {surface: r.out / f"{shot}.png" for surface, r in runs.items()}
            if not (paths["native-srgb"].exists() and paths["native-p3"].exists()):
                continue
            native_p3, native_srgb = rgb_array(paths["native-p3"]), rgb_array(paths["native-srgb"])
            flat = flat_check(native_srgb, native_p3, themes[shot], shot in moving)
            print(f"  {shot} ({themes[shot]}): native P3 against the P3 conversion of native sRGB:")
            for line in describe(flat, script.start.density):
                print(f"    {line}")
            failed |= not flat.ok
            web: list[Versus] = []
            for engine in engines:
                p3_shot, srgb_shot = paths[f"web-p3-{engine}"], paths[f"web-srgb-{engine}"]
                if p3_shot.exists() and srgb_shot.exists():
                    web.append(
                        versus(
                            native_p3,
                            rgb_array(p3_shot),
                            native_srgb,
                            rgb_array(srgb_shot),
                            engine,
                            shot in moving,
                            scaled.get(shot),
                        )
                    )
                    print(f"    {describe_web(web[-1])}")
                    failed |= not web[-1].ok
            shots[shot] = {
                "theme": themes[shot],
                "flat": asdict(flat) | {"ok": flat.ok},
                "web": [asdict(v) | {"ok": v.ok} for v in web],
            }
        report[name] = {
            "script": str(shown(script.path)),
            "runs": {
                s: {"ok": r.ok, "problems": r.problems, "out": str(r.out)} for s, r in runs.items()
            },
            "shots": shots,
        }
    out.mkdir(parents=True, exist_ok=True)
    (out / "report.json").write_text(json.dumps(report, indent=2))
    print(f"report: {shown(out / 'report.json')}")
    print("gamut: ok" if not failed else "gamut: FAILED")
    return 1 if failed else 0
