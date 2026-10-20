"""Colour maths for the harness as CSS Color 4 states it and crates/mv-theme/src/color.rs
computes it: oklch to sRGB and to Display P3, and pixels converted between the two (plan 1.10,
call 2). Display P3 shares sRGB's transfer curve; the matrices are CSS Color 4's sample code's."""

import json
from dataclasses import dataclass
from functools import cache
from pathlib import Path
from typing import cast

import numpy as np
from numpy.typing import NDArray

Floats = NDArray[np.float64]
Bytes = NDArray[np.uint8]

REPO = Path(__file__).resolve().parents[2]
TOKENS = REPO / "crates" / "mv-theme" / "testdata" / "tokens.json"

# The profile macOS gives Display P3, which a PNG of P3 bytes carries (iCCP) so a viewer shows them
# as the colours they are; the native build writes the same bytes (platform_native's PNG writer).
P3_ICC = Path("/System/Library/ColorSync/Profiles/Display P3.icc")

OKLAB_TO_LMS = np.array(
    [
        [1, 0.3963377773761749, 0.2158037573099136],
        [1, -0.1055613458156586, -0.0638541728258133],
        [1, -0.0894841775298119, -1.2914855480194092],
    ]
)
LMS_TO_XYZ = np.array(
    [
        [1.2268798758459243, -0.5578149944602171, 0.2813910456659647],
        [-0.0405757452148008, 1.1122868032803170, -0.0717110580655164],
        [-0.0763729366746601, -0.4214933324022432, 1.5869240198367816],
    ]
)
LINEAR_SRGB_TO_XYZ = np.array(
    [
        [506752 / 1228815, 87881 / 245763, 12673 / 70218],
        [87098 / 409605, 175762 / 245763, 12673 / 175545],
        [7918 / 409605, 87881 / 737289, 1001167 / 1053270],
    ]
)
XYZ_TO_LINEAR_SRGB = np.array(
    [
        [12831 / 3959, -329 / 214, -1974 / 3959],
        [-851781 / 878810, 1648619 / 878810, 36519 / 878810],
        [705 / 12673, -2585 / 12673, 705 / 667],
    ]
)
LINEAR_P3_TO_XYZ = np.array(
    [
        [608311 / 1250200, 189793 / 714400, 198249 / 1000160],
        [35783 / 156275, 247089 / 357200, 198249 / 2500400],
        [0, 32229 / 714400, 5220557 / 5000800],
    ]
)
XYZ_TO_LINEAR_P3 = np.array(
    [
        [446124 / 178915, -333277 / 357830, -72051 / 178915],
        [-14852 / 17905, 63121 / 35810, 423 / 17905],
        [11844 / 330415, -50337 / 660830, 316169 / 330415],
    ]
)
TO_XYZ = {"srgb": LINEAR_SRGB_TO_XYZ, "p3": LINEAR_P3_TO_XYZ}
FROM_XYZ = {"srgb": XYZ_TO_LINEAR_SRGB, "p3": XYZ_TO_LINEAR_P3}


def decode(encoded: Floats) -> Floats:
    """Encoded 0..1 to linear light: the sRGB transfer function's inverse."""
    return np.where(encoded <= 0.04045, encoded / 12.92, ((encoded + 0.055) / 1.055) ** 2.4)


def encode(linear: Floats) -> Floats:
    """Linear light to encoded 0..1, each channel clipped to 0..1 first (no engine maps by chroma)."""
    v = np.clip(linear, 0.0, 1.0)
    return np.where(v <= 0.0031308, 12.92 * v, 1.055 * v ** (1 / 2.4) - 0.055)


def oklch_linear(oklch: tuple[float, float, float], space: str) -> Floats:
    """An oklch colour's linear RGB in `space` ("srgb" or "p3"), unclipped."""
    lightness, chroma, hue = oklch
    lab = np.array([lightness, chroma * np.cos(np.radians(hue)), chroma * np.sin(np.radians(hue))])
    lms = OKLAB_TO_LMS @ lab
    return FROM_XYZ[space] @ (LMS_TO_XYZ @ (lms**3))


def oklch_value(oklch: tuple[float, float, float], space: str) -> Floats:
    """An oklch colour encoded in `space`, 0..255 unrounded: what the port hands the GPU."""
    return encode(oklch_linear(oklch, space)) * 255


def to_bytes(values: Floats) -> Bytes:
    """0..255 values as the GPU stores them: rounded to nearest."""
    return np.floor(values + 0.5).astype(np.uint8)


def convert(pixels: Bytes, source: str, target: str) -> Floats:
    """Pixels (..., 3) encoded in `source` as the same colours encoded in `target`, 0..255
    unrounded, clipped to the target's gamut."""
    linear = decode(pixels.astype(np.float64) / 255)
    matrix = FROM_XYZ[target] @ TO_XYZ[source]
    return encode(linear @ matrix.T) * 255


@dataclass(frozen=True)
class Colour:
    """A colour of a theme as the fixture holds it (WebKit's computed oklch and drawn sRGB pixel),
    and as the port encodes it in Display P3."""

    name: str
    alpha: float
    srgb: tuple[int, int, int]  # WebKit's pixel at full alpha: the clip when outside sRGB
    p3: tuple[float, float, float]  # the oklch in Display P3, 0..255 unrounded
    outside_srgb: bool

    @property
    def p3_bytes(self) -> tuple[int, int, int]:
        r, g, b = (int(v) for v in to_bytes(np.array(self.p3)))
        return r, g, b


@cache
def theme_colours(theme: str) -> list[Colour]:
    """Every colour token and page mix of a theme with a colour (transparent left out)."""
    fixture = json.loads(TOKENS.read_text())
    taken = fixture["themes"][theme]
    found: list[Colour] = []
    for group in ("colors", "mixes"):
        for name, entry in cast(dict[str, dict[str, object]], taken[group]).items():
            alpha = float(cast(float, entry["alpha"]))
            oklch = cast(list[float], entry["oklch"])
            if alpha == 0 or len(oklch) != 4:
                continue
            lch = (oklch[0], oklch[1], oklch[2])
            linear = oklch_linear(lch, "srgb")
            outside = bool(np.any(linear < -0.0005) or np.any(linear > 1.0005))
            r, g, b = (int(v) for v in cast(list[int], entry["rgb"]))
            p3 = oklch_value(lch, "p3")
            found.append(Colour(name, alpha, (r, g, b), (p3[0], p3[1], p3[2]), outside))
    return found
