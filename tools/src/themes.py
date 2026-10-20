"""The six themes, from math-viz's CSS to Rust (`tools/run themes`). Each theme file
(src/style/themes/mathviz_*.module.css under $REACT_REF) is one `.theme` block of custom
properties; every colour token is kept as the CSS writes it (an oklch colour, another token, or a
mix in oklch) and written into crates/mv-theme/src/themes.rs, with the theme's lengths, shadow,
glow and readout font. The port resolves the tokens itself (crates/mv-theme/src/color.rs);
tokens.py takes the browser's resolved values for the same names, which the Rust tests check it
against."""

import os
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

from act import REPO

OUT = REPO / "crates" / "mv-theme" / "src" / "themes.rs"

# The themes in the port's order (theme::Slug): the dark ones, then the light ones, as
# useAppTheme.ts lists them.
SLUGS = (
    "arctic-ice",
    "tron-cyan",
    "midnight-violet",
    "clean-teal",
    "educational-warm",
    "sage-editorial",
)

# The tokens that are not colours, each a field of theme::Definition. The gradients are left out:
# none of the page's stylesheets reads one.
NOT_COLOURS = {
    "--radius": "radius",
    "--sig-card-radius": "card_radius",
    "--sig-card-shadow": "card_shadow",
    "--sig-glow-radius": "glow_radius",
    "--sig-hud-ticks": "hud_ticks",
    "--sig-label-tracking": "label_tracking",
    "--sig-readout-font": "readout_font",
}

# The root font size the page's rem lengths are in (no rule on the page changes it).
REM_PX = 16.0

DECLARATION = re.compile(r"(--[a-z0-9-]+)\s*:\s*([^;]+);", re.S)
OKLCH = re.compile(r"oklch\(([-\d.]+) ([-\d.]+) ([-\d.]+)(?: / ([\d.]+))?\)")
VAR = re.compile(r"var\((--[a-z0-9-]+)\)")
MIX = re.compile(
    r"color-mix\(in oklch, var\((--[a-z0-9-]+)\) ([\d.]+)%, (?:var\((--[a-z0-9-]+)\)|transparent)\)"
)
LENGTH = re.compile(r"(-?[\d.]+)(px|rem)?")
FAMILIES = {"Roboto Mono": "RobotoMono", "STIX Two Text": "StixTwoText"}


def react_ref() -> Path:
    value = os.environ.get("REACT_REF")
    if value is None:
        sys.exit(
            "REACT_REF is not set: export the React reference's checkout (README: The harness)"
        )
    return Path(value)


def theme_file(ref: Path, slug: str) -> Path:
    return ref / "src" / "style" / "themes" / f"mathviz_{slug.replace('-', '_')}.module.css"


@dataclass
class Theme:
    slug: str
    declarations: dict[str, str]  # in the file's order

    def colour_tokens(self) -> list[str]:
        return [
            name
            for name in self.declarations
            if name not in NOT_COLOURS and not name.startswith("--gradient-")
        ]


def read_theme(ref: Path, slug: str) -> Theme:
    css = theme_file(ref, slug).read_text()
    block = css[css.index(".theme {") :]
    declarations = {m[1]: " ".join(m[2].split()) for m in DECLARATION.finditer(block)}
    return Theme(slug, declarations)


def read_themes(ref: Path) -> list[Theme]:
    """The six themes; every one must define the same colour tokens in the same order."""
    themes = [read_theme(ref, slug) for slug in SLUGS]
    first = themes[0].colour_tokens()
    for theme in themes[1:]:
        if theme.colour_tokens() != first:
            sys.exit(f"{theme.slug} defines other colour tokens than {themes[0].slug}")
    return themes


def rust_variant(css_name: str) -> str:
    """--chart-point-1 → ChartPoint1 (the plan's mapping: Ada_Case to CamelCase)."""
    return "".join(part.capitalize() for part in css_name.removeprefix("--").split("-"))


def rust_slug(slug: str) -> str:
    """arctic-ice → ArcticIce."""
    return "".join(part.capitalize() for part in slug.split("-"))


INT = re.compile(r"-?\d+")


def num(text: str) -> str:
    """A float literal: a whole number gains a decimal point, so the generated Rust compiles
    where a float type is expected (rustc E0308 otherwise)."""
    return f"{text}.0" if INT.fullmatch(text) else text


def rust_value(text: str) -> str:
    """A colour token's value as a theme::Value expression."""
    if text == "transparent":
        return "Value::Oklch(Oklch { l: 0.0, c: 0.0, h: 0.0, alpha: 0.0 })"
    if m := OKLCH.fullmatch(text):
        light, chroma, hue = (num(m[i]) for i in (1, 2, 3))
        alpha = num(m[4]) if m[4] else "1.0"
        return f"Value::Oklch(Oklch {{ l: {light}, c: {chroma}, h: {hue}, alpha: {alpha} }})"
    if m := VAR.fullmatch(text):
        return f"Value::Ref(Token::{rust_variant(m[1])})"
    if m := MIX.fullmatch(text):
        other = f"Some(Token::{rust_variant(m[3])})" if m[3] else "None"
        return (
            f"Value::Mix(Mix {{ a: Token::{rust_variant(m[1])}, " f"p: {num(m[2])}, b: {other} }})"
        )
    sys.exit(f"a colour the port can't read: {text}")


def length(theme: Theme, text: str) -> str:
    """A length in px, from px, rem or another token."""
    if m := VAR.fullmatch(text):
        return length(theme, theme.declarations[m[1]])
    m = LENGTH.fullmatch(text)
    if m is None:
        sys.exit(f"{theme.slug}: a length the port can't read: {text}")
    value = float(m[1]) * (REM_PX if m[2] == "rem" else 1)
    return num(f"{value:g}")


def shadow(theme: Theme, text: str) -> str:
    """box-shadow's `x y blur colour`, or None for none."""
    if text == "none":
        return "None"
    m = re.fullmatch(r"(\S+) (\S+) (\S+) (.+)", text)
    if m is None or not m[4].startswith(("color-mix(", "oklch(", "var(")):
        sys.exit(f"{theme.slug}: a shadow the port can't read: {text}")
    x, y, blur = (length(theme, m[i]) for i in (1, 2, 3))
    return (
        f"Some(ShadowDefinition {{ offset: [{x}, {y}], blur: {blur}, "
        f"color: {rust_value(m[4])} }})"
    )


def readout_font(theme: Theme, text: str) -> str:
    first = text.split(",")[0].strip().strip('"')
    if first not in FAMILIES:
        sys.exit(f"{theme.slug}: a readout font the port doesn't ship: {text}")
    return f"ReadoutFont::{FAMILIES[first]}"


def definition(theme: Theme) -> str:
    d = theme.declarations
    lines = [
        "    ThemeDef {",
        f'        slug: "{theme.slug}",',
        "        def: Definition {",
        "            colors: &[",
    ]
    for name in theme.colour_tokens():
        lines.append(f"                (Token::{rust_variant(name)}, {rust_value(d[name])}),")
    lines.append("            ],")
    # Each field with the CSS it came from where that isn't the same number.
    fields = {
        "radius": (length(theme, d["--radius"]), d["--radius"]),
        "card_radius": (length(theme, d["--sig-card-radius"]), d["--sig-card-radius"]),
        "card_shadow": (shadow(theme, d["--sig-card-shadow"]), ""),
        "glow_radius": (length(theme, d["--sig-glow-radius"]), ""),
        "hud_ticks": (length(theme, d["--sig-hud-ticks"]), ""),
        "label_tracking": (length(theme, d["--sig-label-tracking"]), ""),
        "readout_font": (readout_font(theme, d["--sig-readout-font"]), ""),
    }
    for field, (value, css) in fields.items():
        comment = f" // {css}" if css and css != f"{value}px" else ""
        lines.append(f"            {field}: {value},{comment}")
    lines.append("        },")
    lines.append("    },")
    return "\n".join(lines)


HEADER = """\
// The six themes as math-viz writes them at {commit} (src/style/themes/mathviz_*.module.css),
// written from those files by `tools/run themes`: run it again rather than editing this file.
// Token is every colour token the files define, in their order, the gradients left out (the
// page reads none); each theme gives each token as its CSS does: an oklch colour, another
// token's value, or a mix of two in oklch. resolve (theme.rs) turns them into colours.
// generated, do not edit

use enum_map::Enum;

/// Every colour token the theme files define, in their order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Enum)]
pub enum Token {{
{tokens}
}}

/// An oklch colour as a theme file writes it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Oklch {{
    pub l: f64,
    pub c: f64,
    pub h: f64,
    pub alpha: f64,
}}

/// A mix of two tokens' colours in oklch: `color-mix(in oklch, var(--a) p%, var(--b))`,
/// or transparent where `b` is None.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mix {{
    pub a: Token,
    pub p: f64,
    pub b: Option<Token>,
}}

/// A colour token's value as its theme file writes it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Value {{
    Oklch(Oklch),
    Ref(Token),
    Mix(Mix),
}}

/// A theme as its file writes it: every token's value with the lengths, shadow, glow and
/// readout font. `colors` is a slice rather than a map so the generated table stays literal;
// `resolve` builds the token map from it.
#[derive(Clone, Copy, Debug)]
pub struct Definition {{
    pub colors: &'static [(Token, Value)],
    pub radius: f32,
    pub card_radius: f32,
    pub card_shadow: Option<ShadowDefinition>,
    pub glow_radius: f32,
    pub hud_ticks: f32,
    pub label_tracking: f32,
    pub readout_font: ReadoutFont,
}}

/// A box-shadow without spread: offset and blur in px.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShadowDefinition {{
    pub offset: [f32; 2],
    pub blur: f32,
    pub color: Value,
}}

/// The families the themes set for the readouts (--sig-readout-font's first family).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReadoutFont {{
    RobotoMono,
    StixTwoText,
}}

/// One theme's slug with its definition.
#[derive(Clone, Copy, Debug)]
pub struct ThemeDef {{
    pub slug: &'static str,
    pub def: Definition,
}}

pub static DEFINITIONS: &[ThemeDef] = &[
{definitions}
];
"""


def generate(ref: Path, out: Path = OUT) -> Path:
    themes = read_themes(ref)
    commit = subprocess.run(
        ["git", "-C", str(ref), "rev-parse", "--short", "HEAD"],
        capture_output=True,
        text=True,
        check=True,
    ).stdout.strip()
    tokens = "\n".join(f"    {rust_variant(name)}," for name in themes[0].colour_tokens())
    text = HEADER.format(
        commit=commit, tokens=tokens, definitions="\n".join(definition(t) for t in themes)
    )
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(text)
    subprocess.run(["rustfmt", "--edition", "2024", str(out)], check=True)
    return out
