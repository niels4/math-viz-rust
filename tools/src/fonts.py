"""Static TrueType instances of math-viz2's fonts (`tools/run font`): stb_truetype (through
NanoVG's fontstash) and kb_text_shape read TTF, not woff2, and only a variable font's default
instance, so each face the page draws in becomes a static TTF under assets/fonts/, which the app
embeds with include_bytes!. The faces are the page's own (FACES): the weights the page's CSS uses
(style_vars.module.css and the modules at the ported hash), the plane's canvas faces and the
Symbols faces for U+2248."""

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
    # Work Sans, the UI's face, upright only (no rule on the page sets italic): the weights
    # style_vars.module.css and the modules use (400 for the tips, 600 strong, 700 headings,
    # 800 labels), with 500 for the scale's steps.
    Face("work_sans/WorkSans-Variable.woff2", 400, "WorkSans-Regular.ttf"),
    Face("work_sans/WorkSans-Variable.woff2", 500, "WorkSans-Medium.ttf"),
    Face("work_sans/WorkSans-Variable.woff2", 600, "WorkSans-SemiBold.ttf"),
    Face("work_sans/WorkSans-Variable.woff2", 700, "WorkSans-Bold.ttf"),
    Face("work_sans/WorkSans-Variable.woff2", 800, "WorkSans-ExtraBold.ttf"),
    # STIX Two Text, the maths: 400 upright and italic, and 500 where the readout needs it;
    # STIX Two Math's ≈ stands in for the text face's missing one, static 400 used at every
    # weight (global.css declares it 400 700, upright and italic).
    Face("stix_two_text/STIXTwoText-Variable.woff2", 400, "STIXTwoText-Regular.ttf"),
    Face("stix_two_text/STIXTwoText-Variable.woff2", 500, "STIXTwoText-Medium.ttf"),
    Face("stix_two_text/STIXTwoText-Italic-Variable.woff2", 400, "STIXTwoText-Italic.ttf"),
    Face("stix_two_text/STIXTwoMath-Symbols.woff2", None, "STIXTwoMath-Symbols.ttf"),
    # Roboto Mono, the numbers: 500 (the plane's labels, the scrub values, the ruler) and 600,
    # with its ≈ at 500.
    Face("roboto_mono/RobotoMono-Variable.woff2", 500, "RobotoMono-Medium.ttf"),
    Face("roboto_mono/RobotoMono-Variable.woff2", 600, "RobotoMono-SemiBold.ttf"),
    Face("roboto_mono/RobotoMono-Symbols-Variable.woff2", 500, "RobotoMono-Symbols-Medium.ttf"),
    # STIX Two Text 600: the transform rows' names (TransformRow.module.css sets the weight but
    # no family, so they inherit the serif).
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
