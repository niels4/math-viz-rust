//! The function readout: the value line and the form line as token runs.
//!
//! Ported from `src/components/views/FunctionViewer/SideBar/FunctionReadout.tsx`
//! and `FunctionReadout.module.css` at `5714319` (with the fit-to-width scale
//! from `src/components/hooks/useFitWidth.ts`): under the function picker,
//! the equation with the parameter values set (`f(x) = 2.00((x − 1.00)/2.00)²
//! − 3.00`) over the equation's form with its letters (`Form: f(x) =
//! a((x − h)/b)² + k`). Each parameter paints in its colour (`--param-a/k/b/h`
//! from `SideBar.module.css`, per theme) and lights wherever it shows while
//! the pointer or the focus is on it (`state.rs`).
//!
//! A component becomes data here: the builders return the lines as runs of
//! tokens with their roles (plain text, a parameter's run, a stacked
//! fraction, an exponent), and the layout/paint step (phase 2, on the sidebar
//! slot) reads them. The vitest tests' `spoken` helper becomes `spoken`: the
//! lines as a screen reader reads them (reader-only text included, the
//! painted sup digit left out).
//!
//! Out of scope, as recorded: the screen-reader live region (`5a11915`)
//! — SDL has no accessibility tree, so the port carries the text model and
//! no announcement role.

use crate::app::App;
use crate::functions::FunctionSlug;
use crate::state::TransformParam;
use mv_sys::clay;

/// The readout row: flex, vertically centred, at least 78 px tall with the
/// card's inset above it (`FunctionReadout.module.css`: `.readout` — 78 px
/// since `2e6069e`, reserving the division bar's space so a horizontal scale
/// stacking a fraction never moves the row; h5-buttons's named change).
pub const READOUT_MIN_HEIGHT: f32 = 78.0;
/// `margin-top: var(--space-inset)`.
pub const READOUT_MARGIN_TOP: f32 = 12.0;
/// The line's side padding (`.line`: `padding: 0 4px`).
pub const LINE_PADDING_X: f32 = 4.0;
/// `.line { font: 1.75rem var(--font-serif) }`: 28 px STIX Two Text.
pub const LINE_FONT_SIZE: f32 = 28.0;
/// `.line { line-height: round(up, 1.25em, 1px) }`: 35 px.
pub const LINE_HEIGHT: f32 = 35.0;
/// `.form { margin: 4px 0 0; padding: 0 4px }`.
pub const FORM_MARGIN_TOP: f32 = 4.0;
pub const FORM_PADDING_X: f32 = 4.0;
/// `.form { font: 1.25rem var(--font-serif) }`: 20 px STIX Two Text.
pub const FORM_FONT_SIZE: f32 = 20.0;
/// `.form { line-height: round(up, 1.25em, 1px) }`: 25 px.
pub const FORM_LINE_HEIGHT: f32 = 25.0;
/// A parameter's run (`.term`): `padding: 0 3px 1px; border-radius: 4px`.
pub const TERM_PADDING_X: f32 = 3.0;
pub const TERM_PADDING_BOTTOM: f32 = 1.0;
pub const TERM_RADIUS: f32 = 4.0;
/// The lit tint: `color-mix(in oklch, var(--term-ink) 18%, transparent)`.
pub const TERM_TINT: f32 = 0.18;
/// The `h` letter's minus keeps its gap (`.minus { padding-left: 3px }`).
pub const MINUS_PADDING_LEFT: f32 = 3.0;
/// The stacked fraction (`.frac`): `gap: 2px`, `font-size: round(0.8em)` =
/// 22 px at the line size, the text centred. The side padding is
/// `round(0.15em)` of the fraction's own font: 3 px on the value line's
/// 22 px, 2 px on the form line's 16 px.
pub const FRAC_GAP: f32 = 2.0;
pub const FRAC_FONT_SIZE: f32 = 22.0;

/// The fraction's side padding for the line: `round(0.15em)` of the
/// fraction's own font size, as the stylesheet rounds it.
fn frac_pad_x(form: bool) -> f32 {
    (0.15 * size_px(&RunSize::Frac, form)).round()
}
/// A fraction row's inner spans (`> span:not(.bar)`): `padding: 0 4px`.
pub const FRAC_ROW_PAD_X: f32 = 4.0;
/// The division bar (`.bar`): `height: max(1.5px, 0.06875em)`, currentcolor.
pub const BAR_MIN_HEIGHT: f32 = 1.5;
/// The exponent (`.sup`): `margin-left: 1px`, `font-size: round(0.68em)` =
/// 19 px at the line size, `line-height: 0`, raised `0.58em`.
pub const SUP_MARGIN_LEFT: f32 = 1.0;
pub const SUP_FONT_SIZE: f32 = 19.0;
/// Stacked parens (`.paren_big`): `font-size: round(1.42em)` = 40 px at the
/// line size, `line-height: 1`, sunk `0.0775em`.
pub const PAREN_BIG_FONT_SIZE: f32 = 40.0;
/// The readers' exponent for each painted digit (`EXPONENTS`).
pub const SUPERSCRIPT_2: char = '\u{00b2}';
pub const SUPERSCRIPT_3: char = '\u{00b3}';
/// The true minus the values print (`MINUS = "−"`, U+2212).
pub const MINUS_STR: &str = "\u{2212}";

/// A value with two decimals and true minus signs.
///
/// Ported from `FunctionReadout.tsx` (`formatValue`: `v.toFixed(2)` with `-`
/// → U+2212). Runs through `mv_numfmt::to_fixed(v, 2)`, the exact binary
/// `toFixed` (halves up on the binary value, as JavaScript rounds them: a
/// binary tie like 2.675 prints "2.67" on both sides), with the ASCII minus
/// swapped for the true minus the readout prints.
#[must_use]
pub fn format_value(v: f64) -> String {
    mv_numfmt::to_fixed(v, 2).replace('-', MINUS_STR)
}

/// A signed run: `positive`/`negative` then the magnitude with two decimals.
///
/// Ported from `FunctionReadout.tsx` (`signed`: `h` prints `− n`/`+ n`, `k`
/// prints `+ n`/`− n`).
#[must_use]
pub fn signed_value(v: f64, positive: &str, negative: &str) -> String {
    let sign = if v >= 0.0 { positive } else { negative };
    format!("{sign} {}", format_value(v.abs()))
}

/// Fits the line into its row: at most 1, with 1 px spare.
///
/// Ported from `useFitWidth.ts` (`Math.min(1, (outer.clientWidth − 1) /
/// inner.offsetWidth)`). `clientWidth`/`offsetWidth` round to whole px, so
/// the port rounds both inputs first; the painted line scales about its left
/// centre (`transform-origin: left center`). A zero-width line divides by
/// zero as the source does (Rust's float division matches: infinity, then
/// `min(1, …)` keeps 1 for a positive spare).
#[must_use]
pub fn fit_scale(available: f64, content: f64) -> f64 {
    let outer = available.round();
    let inner = content.round();
    ((outer - 1.0) / inner).min(1.0)
}

/// The size a run paints at: the line's, the fraction's, the sup's, or a
/// stacked paren's. Assigned at build (CSS sizes descendants); the painter
/// reads it per run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunSize {
    Line,
    Frac,
    Sup,
    ParenBig,
}

/// Plain text: upright, or italic for a `var` letter. `hidden` is a
/// screen-reader-only span (`Hidden`): painted as nothing, read aloud.
/// `pad_left` is the span's own left padding (the form `h` minus's 3 px
/// `.minus` span; everything else pads through its parent or not at all).
#[derive(Clone, Debug, PartialEq)]
pub struct TextRun {
    pub text: String,
    pub italic: bool,
    pub size: RunSize,
    pub hidden: bool,
    pub pad_left: f32,
}

/// A parameter's run (`Term`): painted in its colour, hoverable, lit through
/// `state.rs`. Values print upright; form letters are `var` italics. `prefix`
/// is an upright lead inside the same padded hit box (the form's `+ ` before
/// k, which the reference renders inside k's `Term`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TermRun {
    pub param: TransformParam,
    pub prefix: String,
    pub text: String,
    pub italic: bool,
    pub size: RunSize,
}

/// One piece of a line: text, a parameter's run, a stacked fraction (top over
/// the bar over bottom, read as `top/bottom`), or the exponent digit (read
/// as its superscript).
#[derive(Clone, Debug, PartialEq)]
pub enum Part {
    Text(TextRun),
    Term(TermRun),
    Frac { top: Vec<Part>, bottom: Vec<Part> },
    Sup { digit: char },
}

/// One equation line: the value line's span, or the form line's paragraph.
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub parts: Vec<Part>,
}

impl Line {
    fn new(parts: Vec<Part>) -> Self {
        Self { parts }
    }
}

/// The parameter values the equation sets: `None` is at its default and left
/// out (`a`/`b` at 1, `h`/`k` at 0). The query-to-`None` mapping is the
/// transform params' (h4-rows, `functions.rs`); the readout only reads it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ValueTerms {
    pub a: Option<f64>,
    pub b: Option<f64>,
    pub h: Option<f64>,
    pub k: Option<f64>,
}

/// The equation's four slots as built runs: `None` is left out.
pub struct EquationTerms {
    pub a: Option<Vec<Part>>,
    pub b: Option<Vec<Part>>,
    pub h: Option<Vec<Part>>,
    pub k: Option<Vec<Part>>,
}

fn text_run(content: &str, italic: bool, size: RunSize) -> TextRun {
    TextRun {
        text: content.to_owned(),
        italic,
        size,
        hidden: false,
        pad_left: 0.0,
    }
}

fn text(content: &str, italic: bool, size: RunSize) -> Part {
    Part::Text(text_run(content, italic, size))
}

/// A screen-reader-only span (`Hidden`): the `Form: ` prefix, the numerator's
/// parens, the fraction's slash, the exponents.
fn hidden_text(content: &str) -> Part {
    Part::Text(TextRun {
        text: content.to_owned(),
        italic: false,
        size: RunSize::Line,
        hidden: true,
        pad_left: 0.0,
    })
}

/// The form `h` term's minus span (`.minus`): the true minus with the span's
/// 3 px left padding, measured and painted as one run.
fn minus_text() -> Part {
    Part::Text(TextRun {
        pad_left: MINUS_PADDING_LEFT,
        ..text_run(MINUS_STR, false, RunSize::Line)
    })
}

fn term(param: TransformParam, prefix: &str, content: &str, italic: bool, size: RunSize) -> Part {
    Part::Term(TermRun {
        param,
        prefix: prefix.to_owned(),
        text: content.to_owned(),
        italic,
        size,
    })
}

fn var(letter: &str, size: RunSize) -> Part {
    text(letter, true, size)
}

/// Resizes a built run list (every `Text`/`Term` takes `size`; fractions
/// recurse, though the equation never nests them).
fn at_size(parts: &[Part], size: RunSize) -> Vec<Part> {
    parts
        .iter()
        .map(|part| match part {
            Part::Text(run) => Part::Text(TextRun {
                size,
                ..run.clone()
            }),
            Part::Term(run) => Part::Term(TermRun {
                size,
                ..run.clone()
            }),
            Part::Frac { top, bottom } => Part::Frac {
                top: at_size(top, size),
                bottom: at_size(bottom, size),
            },
            Part::Sup { digit } => Part::Sup { digit: *digit },
        })
        .collect()
}

/// The equation's slots with the value line's texts: set values as runs, the
/// rest left out.
fn value_terms(values: &ValueTerms) -> EquationTerms {
    EquationTerms {
        a: values.a.map(|v| {
            vec![term(
                TransformParam::A,
                "",
                &format_value(v),
                false,
                RunSize::Line,
            )]
        }),
        b: values.b.map(|v| {
            vec![term(
                TransformParam::B,
                "",
                &format_value(v),
                false,
                RunSize::Line,
            )]
        }),
        h: values.h.map(|v| {
            vec![term(
                TransformParam::H,
                "",
                &signed_value(v, MINUS_STR, "+"),
                false,
                RunSize::Line,
            )]
        }),
        k: values.k.map(|v| {
            vec![term(
                TransformParam::K,
                "",
                &signed_value(v, "+", MINUS_STR),
                false,
                RunSize::Line,
            )]
        }),
    }
}

/// The equation's slots with every letter (`LETTERS`): `a`, `b`, `− h`,
/// `+ k`, always shown.
fn letter_terms() -> EquationTerms {
    EquationTerms {
        a: Some(vec![term(TransformParam::A, "", "a", true, RunSize::Line)]),
        b: Some(vec![term(TransformParam::B, "", "b", true, RunSize::Line)]),
        h: Some(vec![
            minus_text(),
            text(" ", false, RunSize::Line),
            term(TransformParam::H, "", "h", true, RunSize::Line),
        ]),
        k: Some(vec![term(
            TransformParam::K,
            "+ ",
            "k",
            true,
            RunSize::Line,
        )]),
    }
}

/// Lays out the function with its four terms (`Equation`).
///
/// `stacked` shows `b` under a division bar; `bare` is an unshifted,
/// unscaled `x`; shifted `x` takes reader-only parens in the numerator; the
/// inner takes visual parens (big when stacked); `x1` scales bare `x`
/// without parens; `x2`/`x3` add the sup digit; `sinx` prefixes `sin`.
/// A null term is left out.
fn equation(func: FunctionSlug, terms: &EquationTerms) -> Vec<Part> {
    let stacked = terms.b.is_some();
    let bare = terms.h.is_none() && !stacked;

    // Unshifted, unscaled x; shifted x trails its term. The reader-only
    // parens below wrap the shifted numerator only inside a stacked
    // fraction: unstacked, the inner is the shifted run itself.
    let shifted: Vec<Part> = match &terms.h {
        None => vec![var("x", RunSize::Line)],
        Some(h) => {
            let mut shifted = vec![var("x", RunSize::Line), text(" ", false, RunSize::Line)];
            shifted.extend(h.iter().cloned());
            shifted
        }
    };
    let inner: Vec<Part> = match &terms.b {
        None => shifted,
        Some(b) => {
            // The reader-only parens wrap a shifted numerator; an unshifted
            // x stands bare over b (numerator is x itself when h hides).
            let numerator: Vec<Part> = match &terms.h {
                None => shifted,
                Some(_) => {
                    let mut wrapped = vec![hidden_text("(")];
                    wrapped.extend(shifted.iter().cloned());
                    wrapped.push(hidden_text(")"));
                    wrapped
                }
            };
            let mut bottom = vec![hidden_text("/")];
            bottom.extend(b.iter().cloned());
            vec![Part::Frac {
                top: at_size(&numerator, RunSize::Frac),
                bottom: at_size(&bottom, RunSize::Frac),
            }]
        }
    };
    let paren_size = if stacked {
        RunSize::ParenBig
    } else {
        RunSize::Line
    };
    let mut wrapped = vec![text("(", false, paren_size)];
    wrapped.extend(inner.iter().cloned());
    wrapped.push(text(")", false, paren_size));

    let mut body: Vec<Part> = match func {
        FunctionSlug::SinX => {
            let mut body = vec![text("sin", false, RunSize::Line)];
            body.extend(wrapped.iter().cloned());
            body
        }
        FunctionSlug::X1 if terms.a.is_some() && !bare => wrapped,
        FunctionSlug::X1 => inner,
        FunctionSlug::X2 | FunctionSlug::X3 => {
            let digit = match func {
                FunctionSlug::X2 => '2',
                _ => '3',
            };
            if bare {
                vec![var("x", RunSize::Line), Part::Sup { digit }]
            } else {
                let mut body = wrapped;
                body.push(Part::Sup { digit });
                body
            }
        }
    };
    if let Some(a) = &terms.a {
        let mut headed = a.clone();
        if func == FunctionSlug::SinX {
            headed.push(text(" ", false, RunSize::Line));
        }
        headed.append(&mut body);
        body = headed;
    }

    let mut line = vec![
        var("f", RunSize::Line),
        text("(", false, RunSize::Line),
        var("x", RunSize::Line),
        text(") = ", false, RunSize::Line),
    ];
    line.append(&mut body);
    if let Some(k) = &terms.k {
        line.push(text(" ", false, RunSize::Line));
        line.extend(k.iter().cloned());
    }
    line
}

/// The value line: the equation with the set values.
///
/// Ported from `FunctionReadout.tsx` (`FunctionReadout`'s readout div).
#[must_use]
pub fn value_line(func: FunctionSlug, values: &ValueTerms) -> Line {
    Line::new(equation(func, &value_terms(values)))
}

/// The form line: the equation with every letter (`LETTERS`), under the
/// reader-only `Form: ` prefix.
///
/// Ported from `FunctionReadout.tsx` (`FunctionReadout`'s form paragraph).
#[must_use]
pub fn form_line(func: FunctionSlug) -> Line {
    let mut parts = vec![hidden_text("Form: ")];
    parts.extend(equation(func, &letter_terms()));
    Line::new(parts)
}

/// A line as a screen reader reads it: reader-only text included, the painted
/// sup digit left out for its superscript, whitespace collapsed.
///
/// Ported from `FunctionReadout.test.tsx` (`readable`/`spoken`).
#[must_use]
pub fn spoken(line: &Line) -> String {
    fn readable(parts: &[Part], out: &mut String) {
        for part in parts {
            match part {
                Part::Text(run) => out.push_str(&run.text),
                Part::Term(run) => {
                    out.push_str(&run.prefix);
                    out.push_str(&run.text);
                }
                Part::Frac { top, bottom } => {
                    // The slash is the bottom's reader-only span; nothing added.
                    readable(top, out);
                    readable(bottom, out);
                }
                Part::Sup { digit } => {
                    out.push(match digit {
                        '2' => SUPERSCRIPT_2,
                        _ => SUPERSCRIPT_3,
                    });
                }
            }
        }
    }
    let mut raw = String::new();
    readable(&line.parts, &mut raw);
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::functions::FunctionSlug;

    fn values(a: Option<f64>, b: Option<f64>, h: Option<f64>, k: Option<f64>) -> ValueTerms {
        ValueTerms { a, b, h, k }
    }

    fn none() -> ValueTerms {
        values(None, None, None, None)
    }

    #[test]
    fn readout_row_reserves_the_division_bar_height() {
        // `2e6069e` (h5-buttons): the value line's row is at least 78 px
        // tall, so a stacking fraction (b=2 on x²) moves nothing below it
        // (the readout-b-height state pins the behaviour).
        assert_eq!(READOUT_MIN_HEIGHT, 78.0);
    }

    #[test]
    fn shows_the_bare_function_when_every_value_is_at_its_default() {
        // A fresh page graphs x, untransformed.
        let line = value_line(FunctionSlug::X1, &none());
        assert_eq!(spoken(&line), "f(x) = x");
    }

    #[test]
    fn prints_values_with_two_decimals_and_true_minus_signs() {
        // x² flipped and stretched by a, moved left by h and up by k.
        let line = value_line(
            FunctionSlug::X2,
            &values(Some(-1.5), None, Some(-2.0), Some(0.75)),
        );
        assert_eq!(
            spoken(&line),
            "f(x) = \u{2212}1.50(x + 2.00)\u{00b2} + 0.75"
        );
    }

    #[test]
    fn leaves_out_the_values_at_their_defaults() {
        // Only k is off its default, so x³ keeps its bare x.
        let line = value_line(FunctionSlug::X3, &values(None, None, None, Some(-1.0)));
        assert_eq!(spoken(&line), "f(x) = x\u{00b3} \u{2212} 1.00");
    }

    #[test]
    fn scales_a_bare_x_without_parentheses() {
        let line = value_line(FunctionSlug::X1, &values(Some(3.0), None, None, None));
        assert_eq!(spoken(&line), "f(x) = 3.00x");
    }

    #[test]
    fn puts_a_scaled_x_in_parentheses_when_it_is_shifted() {
        let line = value_line(FunctionSlug::X1, &values(Some(3.0), None, Some(1.0), None));
        assert_eq!(spoken(&line), "f(x) = 3.00(x \u{2212} 1.00)");
    }

    #[test]
    fn reads_x_over_b_without_parentheses() {
        let line = value_line(FunctionSlug::X1, &values(None, Some(2.0), None, None));
        assert_eq!(spoken(&line), "f(x) = x/2.00");
    }

    #[test]
    fn reads_a_stacked_fraction_as_top_over_bottom() {
        // On screen, x − 1.00 sits over 0.50; read aloud, it is (x − 1.00)/0.50.
        let line = value_line(
            FunctionSlug::SinX,
            &values(Some(2.0), Some(0.5), Some(1.0), Some(-1.0)),
        );
        assert_eq!(
            spoken(&line),
            "f(x) = 2.00 sin((x \u{2212} 1.00)/0.50) \u{2212} 1.00"
        );
    }

    #[test]
    fn leaves_announcing_changes_to_the_transform_controls() {
        // A screen reader reads the line when its user moves to it, not each
        // time it changes: the model carries no live region or role, only
        // text, so there is nothing that would re-announce a value change.
        // The ports' builders are pure functions of their inputs.
        let first = value_line(FunctionSlug::X1, &values(Some(2.0), None, None, None));
        let second = value_line(FunctionSlug::X1, &values(Some(2.0), None, None, None));
        assert_eq!(spoken(&first), "f(x) = 2.00x");
        assert_eq!(first, second);
    }

    #[test]
    fn shrinks_a_long_equation_to_fit_the_sidebar() {
        // Every value set, in a column as wide as the sidebar's card. The test
        // leaves func at its default (x1), so x1 carries the stacked fraction.
        let line = value_line(
            FunctionSlug::X1,
            &values(Some(20.0), Some(30.0), Some(10.0), Some(12.5)),
        );
        assert_eq!(
            spoken(&line),
            "f(x) = 20.00((x \u{2212} 10.00)/30.00) + 12.50"
        );
        // The 350 px row holds the 394 px line with 1 px spare, ending just
        // inside the column instead of running past it.
        let scale = fit_scale(350.0, 394.0);
        assert_eq!(scale, 349.0 / 394.0);
        let scaled = 394.0 * scale;
        assert!(scaled <= 349.0 + 1e-9);
        assert!(349.0 - scaled <= 3.0);
    }

    #[test]
    fn shows_every_letter_even_for_values_at_their_defaults() {
        // Only k is set, but the form under x² still shows a, h and b.
        let line = form_line(FunctionSlug::X2);
        assert_eq!(
            spoken(&line),
            "Form: f(x) = a((x \u{2212} h)/b)\u{00b2} + k"
        );
    }

    #[test]
    fn reads_an_unshifted_x_over_b_stacked_under_a_sup() {
        // b set on x2 without h: the numerator stands bare over b.
        let line = value_line(FunctionSlug::X2, &values(None, Some(2.0), None, None));
        assert_eq!(spoken(&line), "f(x) = (x/2.00)²");
    }

    #[test]
    fn forms_without_exponents_share_the_stacked_letters() {
        assert_eq!(
            spoken(&form_line(FunctionSlug::X1)),
            "Form: f(x) = a((x − h)/b) + k"
        );
        assert_eq!(
            spoken(&form_line(FunctionSlug::X3)),
            "Form: f(x) = a((x − h)/b)³ + k"
        );
    }

    #[test]
    fn the_plus_sits_inside_k_s_term() {
        // The reference renders `+ <var>k</var>` inside k's Term, so the `+`
        // paints in the parameter's ink and hovers as k (h4-readout review).
        let terms = letter_terms();
        let k = terms.k.expect("form always letters k");
        assert_eq!(k.len(), 1);
        let Part::Term(run) = &k[0] else {
            panic!("k's letter is one term");
        };
        assert_eq!(run.param, TransformParam::K);
        assert_eq!(run.prefix, "+ ");
        assert_eq!(run.text, "k");
        assert!(run.italic);
        // The prefix paints upright, the letter italic, as the source renders
        // `+ <var>k</var>`; the spoken line is unchanged.
        assert_eq!(
            spoken(&form_line(FunctionSlug::X1)),
            "Form: f(x) = a((x − h)/b) + k"
        );
    }

    #[test]
    fn puts_a_before_sin() {
        let line = form_line(FunctionSlug::SinX);
        assert_eq!(spoken(&line), "Form: f(x) = a sin((x \u{2212} h)/b) + k");
    }

    #[test]
    fn lights_its_term_in_both_lines_while_the_pointer_is_on_either_one() {
        use crate::state::HighlightState;
        // x² scaled by a = 2: a shows in both lines, h only in the form.
        let line_values = value_line(FunctionSlug::X2, &values(Some(2.0), None, None, None));
        let line_form = form_line(FunctionSlug::X2);
        assert_eq!(spoken(&line_values), "f(x) = 2.00x\u{00b2}");
        let mut state = HighlightState::touched_out();
        assert!(!state.is_lit(TransformParam::A));

        // Pointing at 2.00 lights a in both lines, and nothing else.
        state.set_hover(&[TransformParam::A], true);
        assert!(state.is_lit(TransformParam::A));
        assert!(!state.is_lit(TransformParam::H));

        // Moving off it puts both out.
        state.set_hover(&[TransformParam::A], false);
        assert!(!state.is_lit(TransformParam::A));
        assert!(!state.is_lit(TransformParam::H));

        // The form's h lights though the value line has no h.
        state.set_hover(&[TransformParam::H], true);
        assert!(state.is_lit(TransformParam::H));
        // Both lines carry a's term; only the form carries h's.
        assert!(has_term(&line_values, TransformParam::A));
        assert!(has_term(&line_form, TransformParam::A));
        assert!(!has_term(&line_values, TransformParam::H));
        assert!(has_term(&line_form, TransformParam::H));
    }

    /// Whether the line carries a run for the parameter, in any section.
    fn has_term(line: &Line, param: TransformParam) -> bool {
        fn hunt(parts: &[Part], param: TransformParam) -> bool {
            for part in parts {
                match part {
                    Part::Term(run) if run.param == param => return true,
                    Part::Frac { top, bottom } => {
                        if hunt(top, param) || hunt(bottom, param) {
                            return true;
                        }
                    }
                    Part::Text(_) | Part::Term(_) | Part::Sup { .. } => {}
                }
            }
            false
        }
        hunt(&line.parts, param)
    }

    #[test]
    fn fit_scale_rounds_layout_widths_like_the_browsers_widths() {
        // clientWidth/offsetWidth are whole px: fractional advances round first.
        assert_eq!(fit_scale(350.4, 394.4), fit_scale(350.0, 394.0));
        assert_eq!(fit_scale(350.0, 394.0), 349.0 / 394.0);
        // A short line is never scaled up.
        assert_eq!(fit_scale(350.0, 94.0), 1.0);
    }

    #[test]
    fn stacked_parens_paint_big_while_bare_runs_do_not() {
        fn paren_sizes(line: &Line) -> Vec<RunSize> {
            line.parts
                .iter()
                .filter_map(|part| match part {
                    Part::Text(run) if run.text == "(" || run.text == ")" => Some(run.size),
                    _ => None,
                })
                .collect()
        }
        let stacked = value_line(FunctionSlug::X2, &values(Some(2.0), Some(0.5), None, None));
        assert!(paren_sizes(&stacked).contains(&RunSize::ParenBig));
        let bare = value_line(FunctionSlug::X1, &none());
        assert!(paren_sizes(&bare).iter().all(|size| *size == RunSize::Line));
    }
}

use crate::input::{Cursor, Hit, HitKind};
use mv_text::{Face, Feature, Features, Style};

/// The readout row's, the value line span's and the form line's Clay ids,
/// for the layout test and the paint pass.
pub const READOUT_ROW_ID: &str = "ReadoutRow";
/// The row's width: the card's content box, fixed. The sidebar column is a
/// fixed 400 px grid track, the sidebar takes its 1 px border with 12 px
/// inset padding, the card its 12 px side padding: 350 px at every window
/// width. Fixed (not grow) so the unscaled line span, which overflows the
/// row where the equation is long, cannot stretch the row, the card and the
/// form the way a flex minimum would: the browser's block row stays 350 and
/// the span overflows it, and the fit-to-width scale paints it back.
pub const READOUT_ROW_WIDTH: f32 = 350.0;
pub const READOUT_LINE_ID: &str = "ReadoutLine";
pub const READOUT_FORM_ID: &str = "ReadoutForm";
/// One id namespace for every parameter's padded term box (the hover
/// target): index counts the terms in document order, value line first.
pub const READOUT_TERM_ID: &str = "ReadoutTerm";

/// The sup's rise and the stacked paren's sink, in em of the line's font
/// (vertical-align 0.58em and -0.0775em).
pub const SUP_RAISE_EM: f32 = 0.58;
pub const PAREN_SINK_EM: f32 = 0.0775;
/// The fraction bar's height, in CSS px (max(1.5px, 0.06875em) reads 1.5px
/// on the reference at both line sizes).
pub const BAR_HEIGHT: f32 = 1.5;

/// A run's paint style: upright or italic STIX Two Text at its px size, with
/// tabular figures on the value line (font-variant-numeric).
fn run_style(size: f32, italic: bool, tabular: bool) -> Style {
    let mut features = Features::new();
    if tabular {
        features.insert(Feature::Tabular);
    }
    Style {
        face: if italic {
            Face::Stix400Italic
        } else {
            Face::Stix400
        },
        size,
        tracking: 0.0,
        features,
    }
}

/// A run's advance, shaped at its size: what the spacer measures and the
/// paint cursor steps by. One spacer per DOM run, so the sums match the
/// reference's per-element advances exactly (h3's no-cross-run-kerning).
fn run_advance(app: &mut App, text: &str, size: f32, italic: bool, tabular: bool) -> f32 {
    mv_text::measure(&mut app.fonts, run_style(size, italic, tabular), text)
}

/// The values the readout shows, from the page query: None is at its
/// default and left out (a/b at 1, h/k at 0). The parse mirrors h4-rows'
/// transformParam (finite; a/b never 0); h4-rows switches this source to
/// their QueryParams at their merge, keeping the default-to-None rule here.
#[must_use]
/// The live values for the lines: the rows' page params, not the URL text.
/// The reference's readout reads the hooks' live values (immediate on every
/// `setValue`); the URL string lags behind the 200 ms debounce, so reading
/// it leaves the readout a drag behind — a held scrub would show the
/// pre-drag values while the box and the reference move on.
///
/// Fix-forward context (h5-box, named in its return): h4-readout owns this
/// file; their states never hold a pending write at shot time, so the two
/// sources never visibly differed there.
pub fn live_param_values(app: &crate::app::App) -> ValueTerms {
    fn live(app: &crate::app::App, param: TransformParam, default: f64) -> Option<f64> {
        let value = crate::transform_section::param_value(app, param);
        (value != default).then_some(value)
    }
    ValueTerms {
        a: live(app, TransformParam::A, 1.0),
        b: live(app, TransformParam::B, 1.0),
        h: live(app, TransformParam::H, 0.0),
        k: live(app, TransformParam::K, 0.0),
    }
}

/// The live values parsed out of a URL string: `live_param_values` for text
/// without an app (tests, fixtures). The parse keeps `Number` semantics, so
/// a scale of 0 and non-numbers read as absent, as the page params do.
pub fn live_values(url: &crate::url::Url) -> ValueTerms {
    fn live(url: &crate::url::Url, key: &str, default: f64, scale: bool) -> Option<f64> {
        let raw = url.raw(key)?;
        let n = crate::url::parse_js_number(raw.as_str())?;
        if !n.is_finite() || (scale && n == 0.0) {
            return None;
        }
        (n != default).then_some(n)
    }
    ValueTerms {
        a: live(url, "a", 1.0, true),
        b: live(url, "b", 1.0, true),
        h: live(url, "h", 0.0, false),
        k: live(url, "k", 0.0, false),
    }
}

/// The hover target of a parameter's runs: index is the parameter in
/// A B H K order, in both lines.
#[must_use]
pub fn term_hit(param: TransformParam) -> Hit {
    Hit {
        kind: HitKind::ReadoutTerm,
        index: param as i64,
    }
}

/// The parameter a term hit names, if any.
#[must_use]
pub fn hit_param(hit: Hit) -> Option<TransformParam> {
    match hit.kind {
        HitKind::ReadoutTerm => match hit.index {
            0 => Some(TransformParam::A),
            1 => Some(TransformParam::B),
            2 => Some(TransformParam::H),
            3 => Some(TransformParam::K),
            _ => None,
        },
        // h4-rows' rows light the same model (the row and field hits carry
        // the parameter in `TransformParam` order); h5-buttons' flip and
        // reset sit on their row, so they name it too.
        HitKind::TransformRow
        | HitKind::TransformField
        | HitKind::TransformFlip
        | HitKind::TransformReset => crate::transform_row::index_param(hit.index),
        _ => None,
    }
}

/// Follows the pointer for the highlight model: on a parameter's runs the
/// hovered parameter takes the lead, anywhere else the pointer's side drops.
/// h4-rows extends the kept side with its row kinds at its merge.
pub fn sync_hover(app: &mut App) {
    match hit_param(app.input.hover) {
        Some(param) => app.state.set_hover(&[param], true),
        // On nothing: the pointer's side goes empty (taking the lead with an
        // empty list lights what the source's off leaves: the focus side,
        // if any — the exception is the source's off with a stale list,
        // which is a no-op where this clears; see `set_hover`: the lit
        // answer is the same either way).
        None => app.state.set_hover(&[], true),
    }
}

/// The font px of a run's size class on the line: the line's own size, the
/// fraction's 0.8em, the sup's 0.68em, the stacked paren's 1.42em (each
/// rounded to whole px, as the reference computes them).
fn size_px(size: &RunSize, form: bool) -> f32 {
    let em = if form { FORM_FONT_SIZE } else { LINE_FONT_SIZE };
    match size {
        RunSize::Line => em,
        RunSize::Frac => (0.8 * em).round(),
        RunSize::Sup => (0.68 * em).round(),
        RunSize::ParenBig => (1.42 * em).round(),
    }
}

/// The line box a run lays out in: the line-height box, except a stacked
/// paren's own 1.42em box (which never wins the line: stacked lines always
/// carry the taller fraction).
fn run_box_height(size: &RunSize, form: bool) -> f32 {
    match size {
        RunSize::ParenBig => size_px(size, form),
        RunSize::Line | RunSize::Frac | RunSize::Sup => {
            if form {
                FORM_LINE_HEIGHT
            } else {
                LINE_HEIGHT
            }
        }
    }
}

/// The text width of a run list: the paint cursor's step and the spacer sum.
fn runs_width(app: &mut App, parts: &[Part], form: bool) -> f32 {
    let mut width = 0.0;
    for part in parts {
        match part {
            Part::Text(run) if run.hidden => {}
            Part::Text(run) => {
                width += run.pad_left
                    + run_advance(app, &run.text, size_px(&run.size, form), run.italic, !form);
            }
            Part::Term(run) => {
                width += run_advance(app, &run.prefix, size_px(&run.size, form), false, !form)
                    + run_advance(app, &run.text, size_px(&run.size, form), run.italic, !form)
                    + 2.0 * TERM_PADDING_X;
            }
            Part::Frac { top, bottom } => {
                width += frac_row_width(app, top, form).max(frac_row_width(app, bottom, form))
                    + 2.0 * frac_pad_x(form);
            }
            Part::Sup { digit } => {
                let sup = size_px(&RunSize::Sup, form);
                width += SUP_MARGIN_LEFT + run_advance(app, &digit.to_string(), sup, false, false);
            }
        }
    }
    width
}

/// Element ids counted in document order: every run spacer takes one, every
/// parameter's padded term box takes one of its own. The paint pass never
/// reads run boxes (it accumulates the same advances), so only the term ids
/// matter across the two passes, and both walks visit terms in the same
/// order.
pub struct LayoutIds {
    run: u32,
    term: u32,
}

impl LayoutIds {
    pub fn fresh() -> Self {
        Self { run: 0, term: 0 }
    }
}

/// The strut: the line-height box the baselines hang from (35 px value,
/// 25 px form).
fn strut_h(form: bool) -> f32 {
    if form { FORM_LINE_HEIGHT } else { LINE_HEIGHT }
}

/// The line font's px size.
fn line_px(form: bool) -> f32 {
    if form { FORM_FONT_SIZE } else { LINE_FONT_SIZE }
}

/// Whether a run list holds a parameter's run (its padding-bottom grows the
/// row by 1 px, as the reference measures).
fn row_has_term(parts: &[Part]) -> bool {
    parts.iter().any(|part| matches!(part, Part::Term(_)))
}

/// A fraction row's width: its runs with the row spans' side padding.
fn frac_row_width(app: &mut App, parts: &[Part], form: bool) -> f32 {
    runs_width(app, parts, form) + 2.0 * FRAC_ROW_PAD_X
}

/// A frac's height: both rows with their term padding, the 2 px gaps and
/// the 1.5 px bar.
fn frac_height(top: &[Part], bottom: &[Part], form: bool) -> f32 {
    let row = |parts: &[Part]| strut_h(form) + if row_has_term(parts) { 1.0 } else { 0.0 };
    row(top) + FRAC_GAP + BAR_HEIGHT + FRAC_GAP + row(bottom)
}

/// Lays out the value line's row in the open Function card: the row (at
/// least 78 px tall, content centred) with the span holding one run per
/// part. The card's own gap carries the readout's margin-top above it.
/// Reads the live values and the highlight from the app.
pub fn layout_value_line(app: &mut App, func: FunctionSlug, ids: &mut LayoutIds) {
    let values = live_param_values(app);
    let line = value_line(func, &values);
    let mut row = crate::ui::empty_decl();
    row.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_fixed(READOUT_ROW_WIDTH),
        height: mv_clay::sizing_fit(clay::SizingConstraintsMinMax {
            min: READOUT_MIN_HEIGHT,
            max: f32::MAX,
        }),
    };
    row.layout.childAlignment = clay::ChildAlignment {
        x: clay::LayoutAlignmentX::Left,
        y: clay::LayoutAlignmentY::Center,
    };
    crate::ui::ui_scope(app, mv_clay::id(READOUT_ROW_ID, 0), &row, |app| {
        let mut span = crate::ui::empty_decl();
        span.layout.sizing = clay::Sizing {
            width: mv_clay::sizing_fit(crate::ui::ZERO_MINMAX),
            height: mv_clay::sizing_fit(crate::ui::ZERO_MINMAX),
        };
        span.layout.padding = clay::Padding {
            left: LINE_PADDING_X as u16,
            right: LINE_PADDING_X as u16,
            top: 0,
            bottom: 0,
        };
        span.layout.layoutDirection = clay::LayoutDirection::LeftToRight;
        span.layout.childAlignment = clay::ChildAlignment {
            x: clay::LayoutAlignmentX::Left,
            y: clay::LayoutAlignmentY::Center,
        };
        crate::ui::ui_scope(app, mv_clay::id(READOUT_LINE_ID, 0), &span, |app| {
            layout_parts(app, &line.parts, false, ids);
        });
    });
}

/// Lays out the form line's paragraph: the runs and the always-stacked
/// fraction at the muted size. The lines column's gap carries the form's
/// margin-top above it.
pub fn layout_form_line(app: &mut App, func: FunctionSlug, ids: &mut LayoutIds) {
    let line = form_line(func);
    let mut para = crate::ui::empty_decl();
    para.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_grow(crate::ui::ZERO_MINMAX),
        height: mv_clay::sizing_fit(crate::ui::ZERO_MINMAX),
    };
    para.layout.padding = clay::Padding {
        left: FORM_PADDING_X as u16,
        right: FORM_PADDING_X as u16,
        top: 0,
        bottom: 0,
    };
    para.layout.layoutDirection = clay::LayoutDirection::LeftToRight;
    para.layout.childAlignment = clay::ChildAlignment {
        x: clay::LayoutAlignmentX::Left,
        y: clay::LayoutAlignmentY::Center,
    };
    crate::ui::ui_scope(app, mv_clay::id(READOUT_FORM_ID, 0), &para, |app| {
        layout_parts(app, &line.parts, true, ids);
    });
}

/// Lays out one run list: a spacer per run, padded hover boxes with regions
/// for the parameters, the fraction as a centred column, the sup as a run.
fn layout_parts(app: &mut App, parts: &[Part], form: bool, ids: &mut LayoutIds) {
    let line_h = strut_h(form);
    for part in parts {
        match part {
            Part::Text(run) if run.hidden => {}
            Part::Text(run) => {
                let size = size_px(&run.size, form);
                let width = run.pad_left + run_advance(app, &run.text, size, run.italic, !form);
                run_spacer(app, width, run_box_height(&run.size, form), ids);
            }
            Part::Term(run) => {
                let size = size_px(&run.size, form);
                let width = run_advance(app, &run.prefix, size, false, !form)
                    + run_advance(app, &run.text, size, run.italic, !form);
                let mut outer = crate::ui::empty_decl();
                outer.layout.sizing = clay::Sizing {
                    width: mv_clay::sizing_fit(crate::ui::ZERO_MINMAX),
                    height: mv_clay::sizing_fit(crate::ui::ZERO_MINMAX),
                };
                outer.layout.padding = clay::Padding {
                    left: TERM_PADDING_X as u16,
                    right: TERM_PADDING_X as u16,
                    top: 0,
                    bottom: TERM_PADDING_BOTTOM as u16,
                };
                outer.cornerRadius = crate::ui::corner_radius_all(TERM_RADIUS);
                // No Clay background: the lit tint paints opaque and
                // pre-blended in the post-pass (a translucent Clay fill
                // would blend twice: once here, once there).
                let n = ids.term;
                ids.term += 1;
                crate::ui::ui_scope(app, mv_clay::id(READOUT_TERM_ID, n), &outer, |app| {
                    run_spacer(app, width, line_h, ids);
                });
                crate::input::region(
                    app,
                    mv_clay::id(READOUT_TERM_ID, n),
                    term_hit(run.param),
                    Cursor::Text,
                    Cursor::Text,
                    false,
                    crate::ui::corner_radius_all(0.0),
                );
            }
            Part::Frac { top, bottom } => {
                let top_w = frac_row_width(app, top, form);
                let bottom_w = frac_row_width(app, bottom, form);
                let content = top_w.max(bottom_w);
                let mut col = crate::ui::empty_decl();
                col.layout.sizing = clay::Sizing {
                    width: mv_clay::sizing_fit(crate::ui::ZERO_MINMAX),
                    height: mv_clay::sizing_fit(crate::ui::ZERO_MINMAX),
                };
                col.layout.padding = clay::Padding {
                    left: frac_pad_x(form) as u16,
                    right: frac_pad_x(form) as u16,
                    top: 0,
                    bottom: 0,
                };
                col.layout.childGap = FRAC_GAP as u16;
                col.layout.layoutDirection = clay::LayoutDirection::TopToBottom;
                col.layout.childAlignment = clay::ChildAlignment {
                    x: clay::LayoutAlignmentX::Center,
                    y: clay::LayoutAlignmentY::Top,
                };
                crate::ui::ui_scope(app, mv_clay::id("ReadoutFrac", frac_no(ids)), &col, |app| {
                    frac_row(app, top, form, ids);
                    let mut bar = crate::ui::empty_decl();
                    bar.layout.sizing = clay::Sizing {
                        width: mv_clay::sizing_fixed(content),
                        height: mv_clay::sizing_fixed(BAR_HEIGHT),
                    };
                    crate::ui::ui_scope(app, mv_clay::id("ReadoutBar", bar_no(ids)), &bar, |_| {});
                    frac_row(app, bottom, form, ids);
                });
            }
            Part::Sup { digit } => {
                let sup = size_px(&RunSize::Sup, form);
                let width =
                    SUP_MARGIN_LEFT + run_advance(app, &digit.to_string(), sup, false, false);
                run_spacer(app, width, line_h, ids);
            }
        }
    }
}

/// One frac row: its own padded width, its runs spread from the padding.
fn frac_row(app: &mut App, parts: &[Part], form: bool, ids: &mut LayoutIds) {
    let mut row = crate::ui::empty_decl();
    row.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_fit(crate::ui::ZERO_MINMAX),
        height: mv_clay::sizing_fit(crate::ui::ZERO_MINMAX),
    };
    row.layout.padding = clay::Padding {
        left: FRAC_ROW_PAD_X as u16,
        right: FRAC_ROW_PAD_X as u16,
        top: 0,
        bottom: 0,
    };
    row.layout.layoutDirection = clay::LayoutDirection::LeftToRight;
    row.layout.childAlignment = clay::ChildAlignment {
        x: clay::LayoutAlignmentX::Left,
        y: clay::LayoutAlignmentY::Center,
    };
    crate::ui::ui_scope(
        app,
        mv_clay::id("ReadoutFracRow", frac_no(ids)),
        &row,
        |app| {
            layout_parts(app, parts, form, ids);
        },
    );
}

/// A fixed run spacer.
fn run_spacer(app: &mut App, width: f32, height: f32, ids: &mut LayoutIds) {
    let mut decl = crate::ui::empty_decl();
    decl.layout.sizing = clay::Sizing {
        width: mv_clay::sizing_fixed(width),
        height: mv_clay::sizing_fixed(height),
    };
    let n = ids.run;
    ids.run += 1;
    crate::ui::ui_scope(app, mv_clay::id("ReadoutRun", n), &decl, |_| {});
}

/// Frac element ids: unique per element in the frame.
fn frac_no(ids: &mut LayoutIds) -> u32 {
    let n = ids.run;
    ids.run += 1;
    n
}

/// Bar ids: unique per bar in the frame.
fn bar_no(ids: &mut LayoutIds) -> u32 {
    let n = ids.run;
    ids.run += 1;
    n
}

/// The laid-out box of an element, if the frame laid it out.
fn laid_out(app: &App, id: clay::ElementId) -> Option<clay::BoundingBox> {
    let data = app
        .clay
        .as_ref()
        .expect("clay context initialised")
        .get_element_data(id);
    data.found.then_some(data.boundingBox)
}

/// Paints both lines post-pass, after the frame's commands: every run from
/// the token model at its measured advance, scaled by the fit about the
/// line's left centre (the CSS transform). Hidden spans paint nothing.
pub fn paint_readout(app: &mut App) {
    let theme = crate::ui::current_theme(app).clone();
    let func = *app.func_param.get();
    let values = live_param_values(app);
    paint_line(app, &theme, &value_line(func, &values), false);
    paint_line(app, &theme, &form_line(func), true);
}

/// The line's ink: foreground for values, muted for the form.
fn line_token(form: bool) -> mv_theme::Token {
    if form {
        mv_theme::Token::ForegroundMuted
    } else {
        mv_theme::Token::Foreground
    }
}

fn paint_line(app: &mut App, theme: &mv_theme::Theme, line: &Line, form: bool) {
    let span_id = if form {
        READOUT_FORM_ID
    } else {
        READOUT_LINE_ID
    };
    let Some(span) = laid_out(app, mv_clay::id(span_id, 0)) else {
        return;
    };
    let avail = if form {
        span.width
    } else {
        laid_out(app, mv_clay::id(READOUT_ROW_ID, 0)).map_or(span.width, |row| row.width)
    };
    let content = runs_width(app, &line.parts, form) + 2.0 * LINE_PADDING_X;
    let scale = fit_scale(f64::from(avail), f64::from(content));
    let zoom = scale as f32;
    // The strut baseline the line-level runs hang from: the strut centred
    // against a taller fraction, at the top otherwise.
    let frac_h = line_frac_height(&line.parts, form);
    let strut = strut_h(form);
    let drop = if frac_h > strut {
        (frac_h - strut) / 2.0
    } else {
        0.0
    };
    let ascent = mv_text::baseline(&mut app.fonts, Face::Stix400, line_px(form), strut);
    let base = span.y + drop + ascent;
    let ink = crate::draw::nvg_color(crate::ui::ink(theme, line_token(form)));
    let ox = span.x;
    let oy = span.y + span.height / 2.0;
    app.vg.save();
    app.vg.translate(ox, oy);
    app.vg.scale(zoom, zoom);
    app.vg.translate(-ox, -oy);
    let mut x = span.x + LINE_PADDING_X;
    paint_parts(
        app,
        theme,
        ink,
        &line.parts,
        form,
        &mut x,
        PaintCtx {
            top: span.y,
            base,
            strut_drop: drop,
            span_h: span.height,
        },
    );
    app.vg.restore();
}

/// Paint coordinates carried down the walk: the line top, its baseline,
/// the strut drop (terms ride the strut down in frac lines) and the span
/// height (the fraction column centres in it).
#[derive(Clone, Copy)]
struct PaintCtx {
    top: f32,
    base: f32,
    strut_drop: f32,
    span_h: f32,
}

/// The frac height of a line, or 0 without one.
fn line_frac_height(parts: &[Part], form: bool) -> f32 {
    let mut height: f32 = 0.0;
    for part in parts {
        if let Part::Frac { top, bottom } = part {
            height = height.max(frac_height(top, bottom, form));
        }
    }
    height
}

#[allow(clippy::too_many_arguments)]
fn paint_parts(
    app: &mut App,
    theme: &mv_theme::Theme,
    ink: mv_nvg::Color,
    parts: &[Part],
    form: bool,
    x: &mut f32,
    ctx: PaintCtx,
) {
    let tabular = !form;
    for part in parts {
        match part {
            Part::Text(run) if run.hidden => {}
            Part::Text(run) => {
                let size = size_px(&run.size, form);
                let style = run_style(size, run.italic, tabular);
                let at = if run.size == RunSize::ParenBig {
                    ctx.base + PAREN_SINK_EM * line_px(form)
                } else {
                    ctx.base
                };
                let adv = mv_text::measure(&mut app.fonts, style, &run.text);
                // The span's own left padding (the form `h` minus): drawn
                // text starts past it, the cursor steps over both.
                let at_x = *x + run.pad_left;
                mv_text::draw(&mut app.fonts, &mut app.vg, style, at_x, at, &run.text, ink);
                note_run(app, style, at_x, at, adv);
                *x += run.pad_left + adv;
            }
            Part::Term(run) => {
                let size = size_px(&run.size, form);
                let param_ink =
                    crate::draw::nvg_color(crate::ui::ink(theme, run.param.colour_token()));
                // The upright prefix (the form's `+ ` before k) measures
                // first, in the same ink under the same padding. The tint
                // paints before either glyph (below): it is opaque, so
                // drawing the prefix first would cover it whenever the term
                // is lit.
                // The cursor is the padded box's left (the layout's term
                // outer): the ink starts past the left padding, the plate
                // spans the box, and the cursor leaves past the right one.
                let mut at = *x + TERM_PADDING_X;
                let prefix_adv = if !run.prefix.is_empty() {
                    let style = run_style(size, false, tabular);
                    mv_text::measure(&mut app.fonts, style, &run.prefix)
                } else {
                    0.0
                };
                let style = run_style(size, run.italic, tabular);
                let adv = mv_text::measure(&mut app.fonts, style, &run.text);
                let outer = clay::BoundingBox {
                    x: *x,
                    y: ctx.top + ctx.strut_drop,
                    width: prefix_adv + adv + 2.0 * TERM_PADDING_X,
                    height: strut_h(form) + TERM_PADDING_BOTTOM,
                };
                if app.state.is_lit(run.param) {
                    paint_lit(app, theme, run.param, outer, form);
                }
                if !run.prefix.is_empty() {
                    let style = run_style(size, false, tabular);
                    mv_text::draw(
                        &mut app.fonts,
                        &mut app.vg,
                        style,
                        at,
                        ctx.base,
                        &run.prefix,
                        param_ink,
                    );
                    at += prefix_adv;
                }
                mv_text::draw(
                    &mut app.fonts,
                    &mut app.vg,
                    style,
                    at,
                    ctx.base,
                    &run.text,
                    param_ink,
                );
                *x = outer.x + outer.width;
            }
            Part::Frac { top, bottom } => {
                paint_frac(app, theme, ink, top, bottom, form, x, ctx);
            }
            Part::Sup { digit } => {
                let sup = size_px(&RunSize::Sup, form);
                let style = run_style(sup, false, false);
                let text = digit.to_string();
                let adv = mv_text::measure(&mut app.fonts, style, &text);
                // The digit's `margin-left: 1px`: drawn text starts past
                // it, the cursor steps over both.
                let at_x = *x + SUP_MARGIN_LEFT;
                let at = ctx.base - SUP_RAISE_EM * line_px(form);
                mv_text::draw(&mut app.fonts, &mut app.vg, style, at_x, at, &text, ink);
                note_run(app, style, at_x, at, adv);
                *x += SUP_MARGIN_LEFT + adv;
            }
        }
    }
}

/// Paints a stacked fraction: the rows' runs on their row baselines, the
/// division bar between them in the line's ink. The narrower row centres
/// under the wider one, as the column's centred rows do.
#[allow(clippy::too_many_arguments)]
fn paint_frac(
    app: &mut App,
    theme: &mv_theme::Theme,
    ink: mv_nvg::Color,
    top: &[Part],
    bottom: &[Part],
    form: bool,
    x: &mut f32,
    ctx: PaintCtx,
) {
    let strut = strut_h(form);
    let top_h = strut + if row_has_term(top) { 1.0 } else { 0.0 };
    let bottom_h = strut + if row_has_term(bottom) { 1.0 } else { 0.0 };
    let pad_x = frac_pad_x(form);
    let col_x = *x + pad_x;
    let top_w = frac_row_width(app, top, form);
    let bottom_w = frac_row_width(app, bottom, form);
    let content = top_w.max(bottom_w);
    let col_h = top_h + FRAC_GAP + BAR_HEIGHT + FRAC_GAP + bottom_h;
    // The column defines the line top (it is the tallest child); the span
    // around it is the same height, so the centring is a no-op.
    let col_top = ctx.top + (ctx.span_h - col_h) / 2.0;
    paint_frac_row(
        app,
        theme,
        ink,
        top,
        form,
        col_x + (content - top_w) / 2.0,
        col_top,
        top_h,
    );
    let bar_y = col_top + top_h + FRAC_GAP;
    app.vg.begin_path();
    app.vg.rect(col_x, bar_y, content, BAR_HEIGHT);
    app.vg.fill_color(ink);
    app.vg.fill();
    paint_frac_row(
        app,
        theme,
        ink,
        bottom,
        form,
        col_x + (content - bottom_w) / 2.0,
        bar_y + BAR_HEIGHT + FRAC_GAP,
        bottom_h,
    );
    *x += content + 2.0 * pad_x;
}

/// Paints one frac row's runs on the row's strut baseline, starting inside
/// the row spans' side padding.
#[allow(clippy::too_many_arguments)]
fn paint_frac_row(
    app: &mut App,
    theme: &mv_theme::Theme,
    ink: mv_nvg::Color,
    parts: &[Part],
    form: bool,
    row_x: f32,
    row_top: f32,
    row_h: f32,
) {
    let base = row_top
        + mv_text::baseline(
            &mut app.fonts,
            Face::Stix400,
            size_px(&RunSize::Frac, form),
            row_h,
        );
    let row_ctx = PaintCtx {
        top: row_top,
        base,
        strut_drop: 0.0,
        span_h: row_h,
    };
    let mut x = row_x + FRAC_ROW_PAD_X;
    paint_parts(app, theme, ink, parts, form, &mut x, row_ctx);
}

/// Paints a lit term: the tint box with its edge lines (value line) or the
/// inset underline (form line). The tint paints as the browser composites
/// it: the ink at 18% over the opaque section backdrop, in the encoded
/// space both sides paint. (The port's framebuffer blends linear while the
/// reference composites gamma, so a translucent fill reads ~30 levels
/// light; the backdrop is always the opaque card, so the composite paints
/// opaque and matches the reference's bytes.)
fn paint_lit(
    app: &mut App,
    theme: &mv_theme::Theme,
    param: TransformParam,
    outer: clay::BoundingBox,
    form: bool,
) {
    let ink = crate::draw::nvg_color(crate::ui::ink(theme, param.colour_token()));
    let backdrop = crate::draw::nvg_color(crate::ui::ink(theme, mv_theme::Token::Section));
    let tint = [
        ink[0] * TERM_TINT + backdrop[0] * (1.0 - TERM_TINT),
        ink[1] * TERM_TINT + backdrop[1] * (1.0 - TERM_TINT),
        ink[2] * TERM_TINT + backdrop[2] * (1.0 - TERM_TINT),
        1.0,
    ];
    app.vg.begin_path();
    crate::draw::rounded_rect(
        &mut app.vg,
        outer,
        crate::ui::corner_radius_all(TERM_RADIUS),
    );
    app.vg.fill_color(tint);
    app.vg.fill();
    if form {
        // The form's underline follows the panel's 4 px radius (an inset
        // shadow): ink the full rounded panel, then cover all but the bottom
        // 2 px with the tint panel, squared at its bottom. A square bar would
        // stick square ends past the rounded corners.
        app.vg.begin_path();
        crate::draw::rounded_rect(
            &mut app.vg,
            outer,
            crate::ui::corner_radius_all(TERM_RADIUS),
        );
        app.vg.fill_color(ink);
        app.vg.fill();
        app.vg.begin_path();
        crate::draw::rounded_rect(
            &mut app.vg,
            clay::BoundingBox {
                x: outer.x,
                y: outer.y,
                width: outer.width,
                height: outer.height - 2.0,
            },
            clay::CornerRadius {
                topLeft: TERM_RADIUS,
                topRight: TERM_RADIUS,
                bottomLeft: 0.0,
                bottomRight: 0.0,
            },
        );
        app.vg.fill_color(tint);
        app.vg.fill();
    } else {
        // The value line's edge lines follow the panel's 4 px radius (three
        // 1 px shadows: tint above, ink below and inside). Square bars would
        // stick square ends past the rounded corners, so each strip is a
        // panel of its own: the top strip repaints the plate's top row with
        // the plate's own tint (a no-op there), the bottom strip merges the
        // inset and outer ink lines, which sit contiguously.
        app.vg.begin_path();
        crate::draw::rounded_rect(
            &mut app.vg,
            clay::BoundingBox {
                x: outer.x,
                y: outer.y - 1.0,
                width: outer.width,
                height: 2.0,
            },
            clay::CornerRadius {
                topLeft: TERM_RADIUS,
                topRight: TERM_RADIUS,
                bottomLeft: 0.0,
                bottomRight: 0.0,
            },
        );
        app.vg.fill_color(tint);
        app.vg.fill();
        app.vg.begin_path();
        crate::draw::rounded_rect(
            &mut app.vg,
            clay::BoundingBox {
                x: outer.x,
                y: outer.y + outer.height - 1.0,
                width: outer.width,
                height: 2.0,
            },
            clay::CornerRadius {
                topLeft: 0.0,
                topRight: 0.0,
                bottomLeft: TERM_RADIUS,
                bottomRight: TERM_RADIUS,
            },
        );
        app.vg.fill_color(ink);
        app.vg.fill();
    }
}

/// Notes a painted run's ink box for the text cursor: the em box over its
/// baseline, as the reference reports text under the pointer.
fn note_run(app: &mut App, style: Style, x: f32, baseline: f32, advance: f32) {
    let m = mv_text::metrics(&mut app.fonts, style.face);
    crate::input::note_text(
        app,
        clay::BoundingBox {
            x,
            y: baseline - m.ascent * style.size,
            width: advance,
            height: (m.ascent + m.descent) * style.size,
        },
    );
}

#[cfg(test)]
mod layout_tests {
    use super::*;

    /// The port's readout boxes against `testdata/page.json` (the reference
    /// at 5714319, `tools/run dom --fixture`): the row and the form line
    /// within the plan's 0.5 CSS px, through the page's layout at values
    /// (terms, fraction, sup and scale in both lines). The value line's span
    /// carries the fit-to-width transform, so its fixture box is the scaled
    /// one: the test scales the laid-out box by the same fit and compares
    /// that (the scale itself is pinned by the shrink unit test).
    #[test]
    fn boxes_match_fixture() {
        const SLACK: f32 = 0.5;
        const FIT_SLACK: f32 = 0.5;
        let _guard = mv_clay::tests_lock();
        let mut app = crate::fixture::headless_app();
        assert!(mv_text::init(&mut app.fonts));
        crate::ui::init_clay(&mut app);
        app.size = [1440.0, 900.0];
        app.pixels = [2880, 1800];
        app.url = crate::url::Url::new("#function-viewer?func=x2&a=2&h=1&k=-3&b=2");
        crate::selector::init_func_from_url(&mut app);
        // The lines read the rows' live params (h5-box), so the params join
        // the URL in the seed, as each `build()` re-seeds them.
        crate::transform_section::init_params_from_url(&mut app);
        let _commands = crate::page::layout_page(&mut app);
        let clay = app.clay.as_ref().expect("clay context initialised");
        let fixture = crate::fixture::page_fixture().expect("page fixture parses");
        let shot = crate::fixture::fixture_shot(&fixture, "readout-values")
            .expect("readout-values shot in fixture");
        for (id, key) in [(READOUT_ROW_ID, "readout"), (READOUT_FORM_ID, "form_line")] {
            let data = clay.get_element_data(mv_clay::id(id, 0));
            assert!(data.found, "{key}: no such element");
            let b = data.boundingBox;
            let want = crate::fixture::fixture_box(&shot.webkit, key).expect("box in fixture");
            for (i, (got, want)) in [b.x, b.y, b.width, b.height]
                .iter()
                .zip(want.iter())
                .enumerate()
            {
                assert!(
                    (got - want).abs() <= SLACK,
                    "{key}[{i}]: laid out {got}, reference {want}"
                );
            }
        }
        // The scaled span: the layout's unscaled box through the fit, about
        // the span's own centre (the transform's fixed point, as CSS lays the
        // transform out).
        let (span, row_w) = {
            let clay = app.clay.as_ref().expect("clay context initialised");
            let span = clay.get_element_data(mv_clay::id(READOUT_LINE_ID, 0));
            assert!(span.found, "value_line: no such element");
            let row = clay.get_element_data(mv_clay::id(READOUT_ROW_ID, 0));
            assert!(row.found, "readout: no such element");
            (span.boundingBox, row.boundingBox.width)
        };
        let values = live_values(&app.url);
        let func = *app.func_param.get();
        let content =
            runs_width(&mut app, &value_line(func, &values).parts, false) + 2.0 * LINE_PADDING_X;
        let scale = fit_scale(f64::from(row_w), f64::from(content)) as f32;
        let b = span;
        let oy = b.y + b.height / 2.0;
        let want = crate::fixture::fixture_box(&shot.webkit, "value_line").expect("box in fixture");
        for (i, (got, want)) in [
            b.x,
            oy - b.height / 2.0 * scale,
            b.width * scale,
            b.height * scale,
        ]
        .iter()
        .zip(want.iter())
        .enumerate()
        {
            assert!(
                (got - want).abs() <= FIT_SLACK,
                "value_line[{i}]: laid out {got}, reference {want}"
            );
        }
    }
}
