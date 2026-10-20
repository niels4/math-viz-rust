"""The six themes, from math-viz2's CSS through the reference (`tools/run themes`).

tools/states/tokens.toml lists (selector, token) pairs, one per theme token on the theme
element to start (H0); each milestone appends the pairs the port newly draws with. The generator
reads the theme files (src/style/themes/mathviz_*.module.css under REACT_REF) for every token's
value as written (an oklch colour, another token, or a mix in oklch), then loads the reference
once per theme in each system scheme (the storage keys seeded from the script's scheme/mode/theme
lines, then reload) and reads getComputedStyle(element).getPropertyValue(token) for every pair:
the browser's token stream must equal the file's, whitespace apart. It writes
crates/mv-theme/src/themes.rs (`// generated, do not edit`): the Token enum in the files' order
with each theme's values, in the shape the port resolves (theme.rs). Run it again rather than
editing the file.

The Definition lengths the file carries (radius, the card, the glow, the HUD ticks, the label
tracking, the readout font) have no source in math-viz2's theme files at 5cef052, and the
port's Theme type still carries them: they are carried from the copied file verbatim, and the
milestone that draws them re-sources them (h1-frame the page tokens). Tokens no theme file
defines are dropped, not carried (the driver's condition on the copy): math-viz's page tokens
(ParamA and friends, the chart points, the curve glow) are gone; the milestone that sources a
token adds it to tokens.toml and regenerates (h4-readout the --param-* rows from
SideBar.module.css's per-theme blocks, read by read_sidebar_params below).
theme.rs's tests and testdata/tokens.json still name the dropped tokens:
h1-frame regenerates them with tools/run tokens (named in the t0-harness return)."""

import json
import os
import re
import subprocess
import sys
import tomllib
from dataclasses import dataclass
from pathlib import Path
from typing import Any, cast

from playwright.sync_api import sync_playwright

from act import REPO, port
from browser import engine_type

OUT = REPO / "crates" / "mv-theme" / "src" / "themes.rs"
TOKENS = REPO / "tools" / "states" / "tokens.toml"

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
DARK_THEMES = SLUGS[:3]

# The system schemes every theme is applied in (the browser's color scheme).
SCHEMES = ("dark", "light")

DECLARATION = re.compile(r"(--[a-z0-9-]+)\s*:\s*([^;]+);", re.S)
OKLCH = re.compile(r"oklch\(([-\d.]+) ([-\d.]+) ([-\d.]+)(?: / ([\d.]+))?\)")
VAR = re.compile(r"var\((--[a-z0-9-]+)\)")
MIX = re.compile(
    r"color-mix\(in oklch, var\((--[a-z0-9-]+)\) ([\d.]+)%, (?:var\((--[a-z0-9-]+)\)|transparent)\)"
)


def resolve_vars(decls: dict[str, str], text: str) -> str:
    """The declaration with its `var(--x)` references replaced by the theme's
    own declarations, so `color-mix(in oklch, var(--foreground) 12%,
    transparent)` reads as the browser computes it (which resolves the
    reference). Single-level references iterate to a fixpoint."""

    def rep(match: re.Match[str]) -> str:
        return decls.get(match.group(1), match.group(0))

    prev = ""
    while prev != text:
        prev, text = text, VAR.sub(rep, text)
    return text


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
        return list(self.declarations)


def read_theme(ref: Path, slug: str) -> Theme:
    css = theme_file(ref, slug).read_text()
    block = css[css.index(".theme {") :]
    declarations = {m[1]: " ".join(m[2].split()) for m in DECLARATION.finditer(block)}
    return Theme(slug, declarations)


def read_themes(ref: Path) -> list[Theme]:
    """The six themes; every one must define the same tokens in the same order."""
    themes = [read_theme(ref, slug) for slug in SLUGS]
    first = themes[0].colour_tokens()
    for theme in themes[1:]:
        if theme.colour_tokens() != first:
            sys.exit(f"{theme.slug} defines other tokens than {themes[0].slug}")
    sidebar = read_sidebar_params(ref)
    points = read_sidebar_points(ref)
    for theme in themes:
        params = sidebar[theme.slug]
        # The per-theme point overrides ride with the base points below.
        points_in_block = {k: params.pop(k) for k in ("--point-p", "--point-q") if k in params}
        if list(params) != ["--param-a", "--param-k", "--param-b", "--param-h"]:
            sys.exit(f"{theme.slug}: sidebar params out of order: {list(params)}")
        merged = dict(params)
        merged["--point-p"] = points_in_block.get("--point-p", points["--point-p"])
        merged["--point-q"] = points_in_block.get("--point-q", points["--point-q"])
        theme.declarations.update(merged)
    return themes


SIDEBAR_THEME = re.compile(r'&\[data-theme="([a-z-]+)"\]\s*\{([^}]*)\}')


def read_sidebar_params(ref: Path) -> dict[str, dict[str, str]]:
    """The readout's per-parameter inks: `--param-a/k/b/h` from SideBar.module.css's
    per-theme blocks (h4-readout sources them; the theme files never define them).
    Values read as written (oklch colours), in the blocks' order."""
    css = (
        ref / "src" / "components" / "views" / "FunctionViewer" / "SideBar" / "SideBar.module.css"
    ).read_text()
    out: dict[str, dict[str, str]] = {}
    for match in SIDEBAR_THEME.finditer(css):
        slug, block = match.group(1), match.group(2)
        out[slug] = {m[1]: " ".join(m[2].split()) for m in DECLARATION.finditer(block)}
    if sorted(out) != sorted(SLUGS):
        sys.exit(f"sidebar params cover {sorted(out)}, want {sorted(SLUGS)}")
    return out


def read_sidebar_points(ref: Path) -> dict[str, str]:
    """P's and Q's default inks: `--point-p`/`--point-q` from SideBar.module.css's
    base `.sidebar` block (h8-plane sources them; per-theme blocks override one
    or the other). Values read as written (`var(--ordinal-NN)` refs)."""
    css = (
        ref / "src" / "components" / "views" / "FunctionViewer" / "SideBar" / "SideBar.module.css"
    ).read_text()
    head = css.split("&[data-theme", 1)[0]
    out = {m[1]: " ".join(m[2].split()) for m in DECLARATION.finditer(head)}
    points = {k: v for k, v in out.items() if k in ("--point-p", "--point-q")}
    if sorted(points) != ["--point-p", "--point-q"]:
        sys.exit(f"sidebar base points cover {sorted(points)}, want both point tokens")
    return points


def read_pairs() -> list[tuple[str, str]]:
    """tools/states/tokens.toml: the (selector, token) pairs in order."""
    manifest: dict[str, Any] = tomllib.loads(TOKENS.read_text())
    raw = manifest.get("pair", [])
    if not isinstance(raw, list) or not raw:
        sys.exit(f"{TOKENS} lists no [[pair]]s")
    out: list[tuple[str, str]] = []
    for entry in cast(list[Any], raw):
        if not isinstance(entry, dict):
            sys.exit(f"{TOKENS}: a [[pair]] is {{selector, token}}")
        item = cast(dict[str, Any], entry)
        selector, token = item.get("selector"), item.get("token")
        if not isinstance(selector, str) or not isinstance(token, str):
            sys.exit(f"{TOKENS}: a [[pair]] is {{selector, token}}")
        out.append((selector, token))
    return out


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


# The Definition lengths carried from the copied file verbatim: math-viz2's theme files at
# 5cef052 define no lengths, and the milestone that draws them re-sources them. The Rust
# expression of each field, per theme.
KEPT: dict[str, dict[str, str]] = {
    "arctic-ice": {
        "radius": "9.6,      // 0.6rem",
        "card_radius": "9.6, // var(--radius)",
        "card_shadow": "None,",
        "glow_radius": "8.0,",
        "hud_ticks": "0.0,",
        "label_tracking": "2.8,",
        "readout_font": "ReadoutFont::RobotoMono,",
    },
    "tron-cyan": {
        "radius": "9.6,      // 0.6rem",
        "card_radius": "4.0, // 4px",
        "card_shadow": "None,",
        "glow_radius": "10.0,",
        "hud_ticks": "1.0,",
        "label_tracking": "1.12,",
        "readout_font": "ReadoutFont::RobotoMono,",
    },
    "midnight-violet": {
        "radius": "9.6,      // 0.6rem",
        "card_radius": "9.6, // var(--radius)",
        "card_shadow": "None,",
        "glow_radius": "14.0,",
        "hud_ticks": "0.0,",
        "label_tracking": "1.12,",
        "readout_font": "ReadoutFont::RobotoMono,",
    },
    "clean-teal": {
        "radius": "9.6,      // 0.6rem",
        "card_radius": "9.6, // var(--radius)",
        "card_shadow": "None,",
        "glow_radius": "0.0,",
        "hud_ticks": "0.0,",
        "label_tracking": "1.12,",
        "readout_font": "ReadoutFont::RobotoMono,",
    },
    "educational-warm": {
        "radius": "9.6,       // 0.6rem",
        "card_radius": "18.0, // 18px",
        # Inserted into the output raw (HEADER.format substitutes values, never formats them).
        "card_shadow": """Some(ShadowDefinition {
                offset: [0.0, 6.0],
                blur: 18.0,
                color: Value::Mix(Mix {
                    a: Token::AccentForeground,
                    p: 12.0,
                    b: None,
                }),
            }),""",
        "glow_radius": "0.0,",
        "hud_ticks": "0.0,",
        "label_tracking": "1.12,",
        "readout_font": "ReadoutFont::RobotoMono,",
    },
    "sage-editorial": {
        "radius": "9.6,      // 0.6rem",
        "card_radius": "9.6, // var(--radius)",
        "card_shadow": "None,",
        "glow_radius": "0.0,",
        "hud_ticks": "0.0,",
        "label_tracking": "1.12,",
        "readout_font": "ReadoutFont::StixTwoText,",
    },
}


def definition(theme: Theme, tokens: list[str]) -> str:
    d = theme.declarations
    lines = [
        "    ThemeDef {",
        f'        slug: "{theme.slug}",',
        "        def: Definition {",
        "            colors: &[",
    ]
    for name in tokens:
        if name in d:
            lines.append(f"                (Token::{rust_variant(name)}, {rust_value(d[name])}),")
    lines.append("            ],")
    kept = KEPT[theme.slug]
    for field in (
        "radius",
        "card_radius",
        "card_shadow",
        "glow_radius",
        "hud_ticks",
        "label_tracking",
        "readout_font",
    ):
        lines.append(f"            {field}: {kept[field]}")
    lines.append("        },")
    lines.append("    },")
    return "\n".join(lines)


HEADER = """\
// The six themes as math-viz2 writes them at {commit} (src/style/themes/mathviz_*.module.css),
// written from those files and the reference by `tools/run themes`: run it again rather than
// editing this file.
// Token is every colour token the files define, in their order; each theme gives each token as
// its CSS does: an oklch colour, another token's value, or a mix of two in oklch. resolve
// (theme.rs) turns them into colours.
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


def theme_of(slug: str) -> str:
    """The scheme a theme shows in: dark for the first three slugs, light for the rest."""
    return "dark" if slug in DARK_THEMES else "light"


def verify(themes: list[Theme], pairs: list[tuple[str, str]]) -> None:
    """Every pair's token stream as the browser computes it, with the theme applied in each
    system scheme: the file's text, whitespace apart. Prints ten spot checks."""
    by_slug = {t.slug: t for t in themes}
    checked = 0
    shown: list[str] = []
    with sync_playwright() as playwright:
        for slug in SLUGS:
            scheme = theme_of(slug)
            mode = json.dumps(json.dumps(scheme))
            dark = json.dumps(json.dumps(slug if scheme == "dark" else "arctic-ice"))
            light = json.dumps(json.dumps(slug if scheme == "light" else "clean-teal"))
            single = json.dumps(json.dumps(slug))
            init = "\n".join(
                [
                    # Both storage shapes: the single key the reference reads before h1, the
                    # three keys after; each reference reads its own.
                    f"localStorage.setItem('mathviz-theme', {single});",
                    f"localStorage.setItem('mathviz-light-dark', {mode});",
                    f"localStorage.setItem('mathviz-dark-theme', {dark});",
                    f"localStorage.setItem('mathviz-light-theme', {light});",
                ]
            )
            for system in SCHEMES:
                browser = engine_type(playwright, "webkit").launch()
                try:
                    context = browser.new_context(
                        color_scheme="dark" if system == "dark" else "light",
                    )
                    context.add_init_script(init)
                    page = context.new_page()
                    page.goto(
                        f"http://localhost:{port('REACT_REF_PORT')}/#function-viewer",
                        wait_until="load",
                    )
                    page.wait_for_function("() => document.body.className.length > 0")
                    body_class = str(page.evaluate("document.body.className"))
                    decls = by_slug[slug].declarations
                    for selector, token in pairs:
                        if token not in decls:
                            continue  # a later milestone's token: h1 checks it
                        want = resolve_vars(
                            decls, " ".join(decls[token].split())
                        )
                        got = str(
                            page.evaluate(
                                "([s, t]) => getComputedStyle(document.querySelector(s))"
                                ".getPropertyValue(t)",
                                [selector, token],
                            )
                        )
                        got = " ".join(got.split())
                        if got != want:
                            sys.exit(
                                f"{slug} in a {system} system ({body_class}): {selector} "
                                f"{token} is {got!r}, the file says {want!r}"
                            )
                        checked += 1
                        if len(shown) < 10 and system == "dark":
                            shown.append(f"{slug} {token}: {got}")
                    context.close()
                finally:
                    browser.close()
    print(f"{checked} token streams equal the files'")
    for line in shown:
        print(f"  {line}")


def generate(ref: Path, out: Path = OUT) -> Path:
    themes = read_themes(ref)
    pairs = read_pairs()
    filed = [token for _, token in pairs]
    decls = themes[0].declarations
    # tokens.toml may lead the reference (h1's tokens, added to math-viz2 after its pin):
    # values come only from the files the reference has; the rest wait for its move.
    emittable = [t for t in filed if t in decls]
    if emittable != [t for t in themes[0].colour_tokens() if t in emittable]:
        sys.exit("tokens.toml's filed tokens differ from the theme files' order")
    if missing := [t for t in filed if t not in decls]:
        print(f"no source in the reference's files yet, skipped: {', '.join(missing)}")
    seen: list[str] = []
    for token in emittable:
        if token not in seen:
            seen.append(token)
    verify(themes, pairs)
    commit = subprocess.run(
        ["git", "-C", str(ref), "rev-parse", "--short", "HEAD"],
        capture_output=True,
        text=True,
        check=True,
    ).stdout.strip()
    tokens = "\n".join(f"    {rust_variant(name)}," for name in seen)
    text = HEADER.format(
        commit=commit,
        tokens=tokens,
        definitions="\n".join(definition(t, emittable) for t in themes),
    )
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(text)
    subprocess.run(["rustfmt", "--edition", "2024", str(out)], check=True)
    return out
