//! Maths runs ported from `math_text.odin` (`Math_Run`, `math_runs`,
//! `math_face`).
//!
//! Pure string splitting: no platform calls, so this module has no external
//! milestone needs. The runs borrow nothing; the caller passes the text each
//! time, as the Odin caller hands its slice.

/// One run of maths text with a single typeface. Ported from `Math_Run`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MathRun {
    /// The run's text.
    pub text: String,
    /// Whether the run sets in the italic maths face.
    pub italic: bool,
}

/// Splits maths text into upright and single-letter-italic runs: bare ASCII
/// letters set italic except inside `sin`, `cos` and `tan`. Ported from
/// `math_runs`.
pub fn math_runs(s: &str) -> Vec<MathRun> {
    fn add(runs: &mut Vec<MathRun>, s: &str, start: usize, end: usize, italic: bool) {
        if start >= end {
            return;
        }
        if !italic
            && let Some(last) = runs.last_mut()
            && !last.italic
        {
            last.text.push_str(&s[start..end]);
            return;
        }
        runs.push(MathRun {
            text: s[start..end].to_string(),
            italic,
        });
    }

    let bytes = s.as_bytes();
    let mut runs: Vec<MathRun> = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        let tail = &s[i..];
        if tail.starts_with("sin") || tail.starts_with("cos") || tail.starts_with("tan") {
            add(&mut runs, s, i, i + 3, false);
            i += 3;
        } else if bytes[i].is_ascii_alphabetic() {
            add(&mut runs, s, i, i + 1, true);
            i += 1;
        } else {
            let mut end = i;
            while end < bytes.len() && !bytes[end].is_ascii_alphabetic() {
                end += 1;
            }
            add(&mut runs, s, i, end, false);
            i = end;
        }
    }
    runs
}

/// The face a maths run is set in: STIX Two Text, italic for a variable.
/// Ported from `math_face`.
pub fn math_face(run: &MathRun) -> mv_text::Face {
    if run.italic {
        mv_text::Face::Stix400Italic
    } else {
        mv_text::Face::Stix400
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // `ui/MathText.test.tsx`'s cases, as runs. Ported from
    // `math_runs_set_letters_italic`.
    #[test]
    fn math_runs_set_letters_italic() {
        let cases: &[(&str, &[(&str, bool)])] = &[
            (
                "f(x) =",
                &[("f", true), ("(", false), ("x", true), (") =", false)],
            ),
            ("sin x", &[("sin ", false), ("x", true)]),
            ("x²", &[("x", true), ("²", false)]),
            ("+ 1.00", &[("+ 1.00", false)]),
            ("+ k", &[("+ ", false), ("k", true)]),
            ("cos(2x)", &[("cos(2", false), ("x", true), (")", false)]),
            ("P", &[("P", true)]),
        ];
        for (text, want) in cases {
            let runs = math_runs(text);
            let got: Vec<(&str, bool)> = runs.iter().map(|r| (r.text.as_str(), r.italic)).collect();
            assert_eq!(got, *want, "{text:?}");
            for run in &runs {
                assert_eq!(
                    math_face(run),
                    if run.italic {
                        mv_text::Face::Stix400Italic
                    } else {
                        mv_text::Face::Stix400
                    }
                );
            }
        }
    }
}
