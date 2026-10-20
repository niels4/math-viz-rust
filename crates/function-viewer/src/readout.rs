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

use crate::functions::FunctionSlug;
use crate::state::TransformParam;

/// The readout row: flex, vertically centred, at least 60 px tall with the
/// card's inset above it (`FunctionReadout.module.css`: `.readout`).
pub const READOUT_MIN_HEIGHT: f32 = 60.0;
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
pub const TERM_TINT: f64 = 0.18;
/// The `h` letter's minus keeps its gap (`.minus { padding-left: 3px }`).
pub const MINUS_PADDING_LEFT: f32 = 3.0;
/// The stacked fraction (`.frac`): `gap: 2px`, `font-size: round(0.8em)` =
/// 22 px at the line size, side padding `round(0.15em)` = 4 px.
pub const FRAC_GAP: f32 = 2.0;
pub const FRAC_FONT_SIZE: f32 = 22.0;
pub const FRAC_PADDING_X: f32 = 4.0;
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextRun {
    pub text: String,
    pub italic: bool,
    pub size: RunSize,
    pub hidden: bool,
}

/// A parameter's run (`Term`): painted in its colour, hoverable, lit through
/// `state.rs`. Values print upright; form letters are `var` italics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TermRun {
    pub param: TransformParam,
    pub text: String,
    pub italic: bool,
    pub size: RunSize,
}

/// One piece of a line: text, a parameter's run, a stacked fraction (top over
/// the bar over bottom, read as `top/bottom`), or the exponent digit (read
/// as its superscript).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Part {
    Text(TextRun),
    Term(TermRun),
    Frac { top: Vec<Part>, bottom: Vec<Part> },
    Sup { digit: char },
}

/// One equation line: the value line's span, or the form line's paragraph.
#[derive(Clone, Debug, PartialEq, Eq)]
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

fn text(content: &str, italic: bool, size: RunSize) -> Part {
    Part::Text(TextRun {
        text: content.to_owned(),
        italic,
        size,
        hidden: false,
    })
}

/// A screen-reader-only span (`Hidden`): the `Form: ` prefix, the numerator's
/// parens, the fraction's slash, the exponents.
fn hidden_text(content: &str) -> Part {
    Part::Text(TextRun {
        text: content.to_owned(),
        italic: false,
        size: RunSize::Line,
        hidden: true,
    })
}

fn term(param: TransformParam, content: &str, italic: bool, size: RunSize) -> Part {
    Part::Term(TermRun {
        param,
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
                &format_value(v),
                false,
                RunSize::Line,
            )]
        }),
        b: values.b.map(|v| {
            vec![term(
                TransformParam::B,
                &format_value(v),
                false,
                RunSize::Line,
            )]
        }),
        h: values.h.map(|v| {
            vec![term(
                TransformParam::H,
                &signed_value(v, MINUS_STR, "+"),
                false,
                RunSize::Line,
            )]
        }),
        k: values.k.map(|v| {
            vec![term(
                TransformParam::K,
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
        a: Some(vec![term(TransformParam::A, "a", true, RunSize::Line)]),
        b: Some(vec![term(TransformParam::B, "b", true, RunSize::Line)]),
        h: Some(vec![
            text(MINUS_STR, false, RunSize::Line),
            text(" ", false, RunSize::Line),
            term(TransformParam::H, "h", true, RunSize::Line),
        ]),
        k: Some(vec![
            text("+", false, RunSize::Line),
            text(" ", false, RunSize::Line),
            term(TransformParam::K, "k", true, RunSize::Line),
        ]),
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
                Part::Term(run) => out.push_str(&run.text),
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
        state.set_hover(Some(TransformParam::A));
        assert!(state.is_lit(TransformParam::A));
        assert!(!state.is_lit(TransformParam::H));

        // Moving off it puts both out.
        state.set_hover(None);
        assert!(!state.is_lit(TransformParam::A));
        assert!(!state.is_lit(TransformParam::H));

        // The form's h lights though the value line has no h.
        state.set_hover(Some(TransformParam::H));
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
