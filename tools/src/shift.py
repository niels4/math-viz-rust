"""The band check (plan section 1.9): two frames compared in bands of 60 device rows, the
second (the port's) moved up and down one device row. An edge one row off is invisible to a
flat-pixel judge and to `tools/run diff`'s whole-frame count, so where the differing pixels fall
matters: a frame whose content sits a row high differs along every horizontal edge, and moving it
down one row makes every band but the wrapped edge row identical."""

from pathlib import Path

import numpy as np
from numpy.typing import NDArray
from PIL import Image

from act import shown
from images import difference, load_rgb

BAND_ROWS = 60  # device rows per band


def pixels(path: Path) -> NDArray[np.int16]:
    with Image.open(path) as image:
        return np.asarray(image.convert("RGB")).astype(np.int16)


def band_changed(
    a: NDArray[np.int16], b: NDArray[np.int16], threshold: int, rows: int
) -> list[int]:
    """Changed pixels per band of `rows` device rows: a pixel changed when a channel differs by
    more than `threshold`."""
    changed = np.abs(a - b).max(axis=2) > threshold
    return [int(changed[y : y + rows].sum()) for y in range(0, a.shape[0], rows)]


def run_shift(a_path: Path, b_path: Path, threshold: int = 0, rows: int = BAND_ROWS) -> int:
    a_image, b_image = load_rgb(a_path), load_rgb(b_path)
    if a_image.size != b_image.size:
        print(
            f"sizes differ: {a_image.size[0]}x{a_image.size[1]} and {b_image.size[0]}x{b_image.size[1]}"
        )
        return 1
    a, b = pixels(a_path), pixels(b_path)
    height = a.shape[0]
    result = difference(a_image, b_image, threshold)
    print(f"{a_path.name} vs {b_path.name}: {result.summary()}")
    offsets = {
        0: b,
        1: np.roll(b, 1, axis=0),  # the port's frame moved down one device row
        -1: np.roll(b, -1, axis=0),  # moved up one device row
    }
    bands = {offset: band_changed(a, moved, threshold, rows) for offset, moved in offsets.items()}
    total = {offset: sum(counts) for offset, counts in bands.items()}
    count = len(bands[0])
    for i in range(count):
        if result.same and not bands[0][i]:
            continue
        if bands[0][i] or bands[1][i] or bands[-1][i]:
            top = i * rows
            print(
                f"  rows {top}-{min(top + rows, height)}: "
                f"{bands[0][i]} differ as is, "
                f"{bands[1][i]} moved down a row, {bands[-1][i]} moved up a row"
            )
    if result.same:
        print("the frames are identical: no row offset")
    else:
        best = min(total, key=lambda offset: total[offset])
        if best != 0 and total[best] < total[0]:
            direction = "down" if best == 1 else "up"
            edge = "high" if best == 1 else "low"
            print(
                f"b sits one row {edge}: moved {direction} a row, {total[best]} pixels differ "
                f"(of {total[0]} as is)"
            )
        else:
            print(f"no single-row offset explains the difference ({total[0]} pixels as is)")
    print(f"bands: {shown(a_path)} {shown(b_path)}")
    return 0
