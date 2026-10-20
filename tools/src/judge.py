"""The judgment's numbers for a states run (`tools/run judge DIR`): each shot of the native build
against the reference in each engine at threshold 0, the pixels apart that lie in flat areas of
both images (the 3 x 3 neighbourhood uniform in each: a colour or a part out of place, where an
anti-aliased edge the rasterisers draw differently is not) and those more than 2 levels apart, with
where they fall (clusters of 8 px tiles, the largest first, as boxes in logical px); and whether the
port's surfaces agree (native against the web build in each engine, at threshold 0). The numbers say
where to look: the judgment is a person reading the images beside them (README: The harness).
Writes judge.txt, judge.json and, for each shot with flat pixels more than 2 levels apart, a map
(orange: apart; red: flat and more than 2 levels apart) under DIR/judgment/.

A cluster of flat pixels apart is either a defect (a flat-area colour, a missing or extra element,
a box off by more than 1 device px, a wrong label, number or curve) or one of the plan's recorded
categories (section 1.9 item 3), named in the progress file's cluster table in these words:
(a) glyph ink, stb_truetype against the engine's rasteriser; (b) one device row or column at an
anti-aliased edge; (c) Canvas 2D against NanoVG stroke geometry at sub-pixel positions; (d) the
web only: Chromium's SwiftShader or Firefox's late frame; (e) an engine quirk the entry records
with its measurement."""

import json
from collections.abc import Iterator
from concurrent.futures import ProcessPoolExecutor
from dataclasses import asdict, dataclass, field
from pathlib import Path

import numpy as np
from numpy.typing import NDArray
from PIL import Image

from act import shown
from script import load
from states import STATES

LEVELS = 2  # a flat pixel further apart than this is a colour or a part out of place
TILE = 8  # logical px: clusters are of tiles this size that hold such pixels, touching
CLUSTERS = 4  # the largest clusters listed per comparison
REFERENCES = ("webkit", "chromium", "firefox")
PORTS = ("web-webkit", "web-firefox", "web-chromium")

type Pixels = NDArray[np.int16]
type Mask = NDArray[np.bool_]


@dataclass
class Cluster:
    pixels: int
    box: tuple[float, float, float, float]  # x, y, width, height in logical px
    max: int


@dataclass
class Versus:
    """One shot against one other image."""

    other: str
    changed: int = 0
    max: int = 0
    flat: int = 0
    apart: int = 0  # flat and more than LEVELS apart
    clusters: list[Cluster] = field(default_factory=lambda: [])
    map: str | None = None


@dataclass
class Shot:
    state: str
    shot: str
    versus: list[Versus] = field(default_factory=lambda: [])


def load_pixels(path: Path) -> Pixels:
    with Image.open(path) as image:
        return np.asarray(image.convert("RGB")).astype(np.int16)


def flat_mask(image: Pixels) -> Mask:
    """Pixels whose 3 x 3 neighbourhood is one colour."""
    pad = np.pad(image, ((1, 1), (1, 1), (0, 0)), mode="edge")
    h, w = image.shape[:2]
    same = np.ones((h, w), bool)
    for dy in range(3):
        for dx in range(3):
            same &= np.all(pad[dy : dy + h, dx : dx + w] == image, axis=2)
    return same


def clusters(apart: Mask, diff: NDArray[np.int16], density: float) -> list[Cluster]:
    """The pixels in `apart`, grouped by touching tiles of TILE logical px, the largest first."""
    tile = max(1, round(TILE * density))
    h, w = apart.shape
    th, tw = -(-h // tile), -(-w // tile)
    padded = np.zeros((th * tile, tw * tile), bool)
    padded[:h, :w] = apart
    counts = padded.reshape(th, tile, tw, tile).sum(axis=(1, 3))
    seen = np.zeros_like(counts, bool)
    found: list[Cluster] = []
    for ty, tx in zip(*np.nonzero(counts), strict=True):
        if seen[ty, tx]:
            continue
        stack: list[tuple[int, int]] = [(int(ty), int(tx))]
        members: list[tuple[int, int]] = []
        seen[ty, tx] = True
        while stack:
            y, x = stack.pop()
            members.append((y, x))
            for dy in (-1, 0, 1):
                for dx in (-1, 0, 1):
                    ny, nx = y + dy, x + dx
                    if 0 <= ny < th and 0 <= nx < tw and counts[ny, nx] and not seen[ny, nx]:
                        seen[ny, nx] = True
                        stack.append((ny, nx))
        ys = [y for y, _ in members]
        xs = [x for _, x in members]
        y0, y1 = min(ys) * tile, min((max(ys) + 1) * tile, h)
        x0, x1 = min(xs) * tile, min((max(xs) + 1) * tile, w)
        window = apart[y0:y1, x0:x1]
        py, px = np.nonzero(window)
        if len(py) == 0:
            continue
        box = (
            (x0 + int(px.min())) / density,
            (y0 + int(py.min())) / density,
            (int(px.max()) - int(px.min()) + 1) / density,
            (int(py.max()) - int(py.min()) + 1) / density,
        )
        found.append(Cluster(len(py), box, int(diff[y0:y1, x0:x1][window].max())))
    found.sort(key=lambda c: -c.pixels)
    return found[:CLUSTERS]


def versus(a: Pixels, b: Pixels, other: str, density: float, map_path: Path | None) -> Versus:
    v = Versus(other)
    if a.shape != b.shape:
        v.changed = -1
        return v
    diff = np.abs(a - b).max(axis=2)
    changed = diff > 0
    v.changed = int(changed.sum())
    v.max = int(diff.max())
    if not v.changed:
        return v
    flat = changed & flat_mask(a) & flat_mask(b)
    apart = flat & (diff > LEVELS)
    v.flat = int(flat.sum())
    v.apart = int(apart.sum())
    if v.apart:
        v.clusters = clusters(apart, diff, density)
        if map_path is not None:
            picture = np.full(a.shape, 255, np.uint8)
            picture[changed] = (255, 160, 0)
            picture[apart] = (220, 0, 0)
            map_path.parent.mkdir(parents=True, exist_ok=True)
            Image.fromarray(picture).save(map_path)
            v.map = str(map_path)
    return v


def judge_state(root: Path, state: str) -> list[Shot]:
    """Each shot the native build took in `state`, against the port's web build in each engine and
    the reference in each engine that ran."""
    path = STATES / f"{state.partition('@')[0]}.act"
    density = load(path).start.density if path.exists() else 2.0
    native = root / state / "native"
    shots: list[Shot] = []
    for png in sorted(native.glob("*.png")):
        a = load_pixels(png)
        shot = Shot(state, png.stem)
        for port in PORTS:
            other = root / state / port / png.name
            if other.exists():
                shot.versus.append(versus(a, load_pixels(other), port, density, None))
        for engine in REFERENCES:
            other = root / state / f"ref-{engine}" / png.name
            if other.exists():
                map_path = root / "judgment" / "maps" / f"{state}-{png.stem}-{engine}.png"
                shot.versus.append(versus(a, load_pixels(other), engine, density, map_path))
        shots.append(shot)
    return shots


def told(v: Versus) -> str:
    if v.changed < 0:
        return f"{v.other}: sizes differ"
    if v.changed == 0:
        return f"{v.other}: same"
    text = f"{v.other}: {v.changed} px apart (max {v.max}), {v.flat} flat, {v.apart} flat >{LEVELS}"
    for c in v.clusters:
        x, y, w, h = c.box
        text += f"\n      {c.pixels} px at ({x:.1f}, {y:.1f}) {w:.1f} x {h:.1f}, max {c.max}"
    return text


def lines_of(shot: Shot) -> Iterator[str]:
    ports = [v for v in shot.versus if v.other in PORTS]
    same = [v.other for v in ports if v.changed == 0]
    yield f"{shot.state}/{shot.shot}: native = " + (" = ".join(same) if same else "no web shot")
    for v in ports:
        if v.changed != 0:
            yield f"  {told(v)}"
    for v in shot.versus:
        if v.other in REFERENCES:
            yield f"  vs {told(v)}"


def run_judge(root: Path, names: list[str], jobs: int = 8) -> int:
    states = names or sorted(p.name for p in root.iterdir() if (p / "native").is_dir())
    with ProcessPoolExecutor(max_workers=jobs) as pool:
        results = list(pool.map(judge_state, [root] * len(states), states))
    shots = [s for state in results for s in state]
    lines = [line for s in shots for line in lines_of(s)]
    out = root / "judgment"
    out.mkdir(parents=True, exist_ok=True)
    legend = (
        "The port against itself is the exact bar (plan 1.9 item 1): native = every web engine at "
        "threshold 0 over the whole frame. Against the reference, a cluster of flat pixels apart "
        "is a defect or one of the recorded categories, named in these words: (a) glyph ink, "
        "(b) one device row or column at an anti-aliased edge, (c) Canvas 2D against NanoVG "
        "stroke geometry, (d) Chromium's SwiftShader or Firefox's late frame, web only, (e) a "
        "recorded engine quirk."
    )
    (out / "judge.txt").write_text(legend + "\n\n" + "\n".join(lines) + "\n")
    (out / "judge.json").write_text(json.dumps([asdict(s) for s in shots], indent=1))
    # Where to look first: per state, the most flat pixels apart in a shot against each engine.
    summary = [f"{'state':28} {'shots':>5}  " + "  ".join(f"{e:>9}" for e in REFERENCES)]
    for state, found in zip(states, results, strict=True):
        worst = {
            e: max((v.apart for s in found for v in s.versus if v.other == e), default=-1)
            for e in REFERENCES
        }
        cells = "  ".join(f"{worst[e]:>9}" if worst[e] >= 0 else f"{'-':>9}" for e in REFERENCES)
        summary.append(f"{state:28} {len(found):>5}  {cells}")
    (out / "summary.txt").write_text("\n".join(summary) + "\n")
    print("\n".join(summary))
    print(f"{shown(out / 'judge.txt')}, {shown(out / 'summary.txt')}")
    return 0
