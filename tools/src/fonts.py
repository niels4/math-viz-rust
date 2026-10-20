"""Static TrueType instances of the React app's fonts (`tools/run font`): stb_truetype (through
NanoVG's fontstash) and kb_text_shape read TTF, not woff2, and only a variable font's default
instance, so each face the page draws in becomes a static TTF under assets/fonts/, which the app
embeds with include_bytes!. The faces are the page's own (FACES): every text node of the reference in
WebKit across the six themes and the states that show more text, plus the plane's canvas faces
(CartesianPlane/faces.ts), measured in M4a."""

# fontTools ships no type information.
# pyright: reportMissingTypeStubs=false, reportUnknownMemberType=false, reportUnknownVariableType=false

from dataclasses import dataclass
from pathlib import Path

from fontTools.ttLib import TTFont
from fontTools.varLib import instancer

from act import REPO

OUT = REPO / "assets" / "fonts"


@dataclass(frozen=True)
class Face:
    """One static face: a font file under $REACT_REF/src/style/fonts, its weight (None for a
    static source), and the TTF it becomes."""

    source: str
    weight: float | None
    out: str


FACES = (
    # Work Sans, the UI's face, upright only (no rule on the page sets italic).
    Face("work_sans/WorkSans-Variable.woff2", 400, "WorkSans-Regular.ttf"),
    Face("work_sans/WorkSans-Variable.woff2", 500, "WorkSans-Medium.ttf"),
    Face("work_sans/WorkSans-Variable.woff2", 600, "WorkSans-SemiBold.ttf"),
    Face("work_sans/WorkSans-Variable.woff2", 700, "WorkSans-Bold.ttf"),
    Face("work_sans/WorkSans-Variable.woff2", 800, "WorkSans-ExtraBold.ttf"),
    # STIX Two Text, the maths: 400 upright and italic (<var>), and 500 where Sage Editorial's
    # readout font is STIX (--sig-readout-font, the readouts' weight 500, in the DOM and on the
    # plane's canvas). STIX Two Math's ≈ stands in for the text face's missing one, static 400
    # used at every weight (global.css declares it 400 700, upright and italic).
    Face("stix_two_text/STIXTwoText-Variable.woff2", 400, "STIXTwoText-Regular.ttf"),
    Face("stix_two_text/STIXTwoText-Variable.woff2", 500, "STIXTwoText-Medium.ttf"),
    Face("stix_two_text/STIXTwoText-Italic-Variable.woff2", 400, "STIXTwoText-Italic.ttf"),
    Face("stix_two_text/STIXTwoMath-Symbols.woff2", None, "STIXTwoMath-Symbols.ttf"),
    # Roboto Mono, the numbers: 500 (readouts, ticks, the ruler) and 600 (key caps), with its ≈
    # at 500 (the tick labels' and readouts' weight).
    Face("roboto_mono/RobotoMono-Variable.woff2", 500, "RobotoMono-Medium.ttf"),
    Face("roboto_mono/RobotoMono-Variable.woff2", 600, "RobotoMono-SemiBold.ttf"),
    Face("roboto_mono/RobotoMono-Symbols-Variable.woff2", 500, "RobotoMono-Symbols-Medium.ttf"),
    # STIX Two Text 600: a term's tip names its parameter in it (TermTip.module.css .name sets the
    # weight and size but no family, so it inherits the equation's STIX; found in M8).
    Face("stix_two_text/STIXTwoText-Variable.woff2", 600, "STIXTwoText-SemiBold.ttf"),
)


def instance(source: Path, out: Path, axes: dict[str, float]) -> Path:
    """Pin every axis named in `axes` (the rest keep their defaults) and save a plain TTF; a static
    source given no axes is only unpacked (woff2 to TTF)."""
    font = TTFont(source, recalcTimestamp=False)  # the same input gives the same bytes
    if "fvar" in font:
        defaults = {axis.axisTag: axis.defaultValue for axis in font["fvar"].axes}
        font = instancer.instantiateVariableFont(font, defaults | axes, updateFontNames=True)
    elif axes:
        raise SystemExit(f"{source} is not a variable font: it has no axes to pin")
    font.flavor = None
    out.parent.mkdir(parents=True, exist_ok=True)
    font.save(out)
    return out


def page_faces(react_ref: Path, out_dir: Path = OUT) -> list[Path]:
    """Every face in FACES from the reference's font files into out_dir."""
    fonts = react_ref / "src" / "style" / "fonts"
    return [
        instance(
            fonts / face.source,
            out_dir / face.out,
            {} if face.weight is None else {"wght": face.weight},
        )
        for face in FACES
    ]
