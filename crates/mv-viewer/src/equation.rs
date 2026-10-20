//! The equation's tokens: the live and form streams the page sets.

#[cfg(test)]
use enum_map::EnumMap;
use enumset::EnumSet;

use super::form::{Base, PARAM_LETTERS, Param, Params, is_at_default};
use mv_numfmt::{MINUS, format_stored};

/// How the equation sets: live values or the form.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EquationMode {
    Live,
    Form,
}

/// A live hold: the kept and fine parameters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LiveHold {
    pub keep: EnumSet<Param>,
    pub fine: EnumSet<Param>,
}

/// A text token.
#[derive(Clone, Debug, PartialEq)]
pub struct TextToken {
    pub text: String,
}

/// A term token: a parameter's value in the equation.
#[derive(Clone, Debug, PartialEq)]
pub struct TermToken {
    pub param: Param,
    pub text: String,
    pub ghost: bool,
}

/// A fraction token: its numerator and denominator.
#[derive(Clone, Debug, PartialEq)]
pub struct FracToken {
    pub num: Vec<Token>,
    pub den: Vec<Token>,
}

/// A superscript token.
#[derive(Clone, Debug, PartialEq)]
pub struct SupToken {
    pub text: String,
}

/// A parenthesis token, big or small.
#[derive(Clone, Debug, PartialEq)]
pub struct ParenToken {
    pub text: String,
    pub big: bool,
}

/// An equation token: text, a term, a fraction, a superscript or parens.
#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    Text(TextToken),
    Term(TermToken),
    Frac(FracToken),
    Sup(SupToken),
    Paren(ParenToken),
}

/// A signed value in words.
fn signed(v: f64, fine: bool, positive: &str, negative: &str) -> String {
    format!(
        "{} {}",
        if v >= 0.0 { positive } else { negative },
        format_stored(v.abs(), fine)
    )
}

/// The equation's tokens for a function and parameters.
pub fn equation_tokens(
    func: Base,
    params: Params,
    mode: EquationMode,
    hold: LiveHold,
) -> Vec<Token> {
    let form = mode == EquationMode::Form;
    fn shown(params: Params, p: Param, form: bool, hold: LiveHold) -> bool {
        form || !is_at_default(params, p) || hold.keep.contains(p)
    }
    fn term(params: Params, p: Param, s: String, form: bool) -> Token {
        Token::Term(TermToken {
            param: p,
            text: s,
            ghost: form && is_at_default(params, p),
        })
    }
    fn value(params: Params, p: Param, hold: LiveHold) -> String {
        format_stored(params[p], hold.fine.contains(p))
    }

    let mut inner: Vec<Token> = vec![Token::Text(TextToken {
        text: "x".to_string(),
    })];
    let mut has_frac = false;
    for step in super::form::INNER_STEPS {
        if !shown(params, step, form, hold) {
            continue;
        }
        if step == Param::H {
            match inner.last_mut() {
                Some(Token::Text(t)) => t.text.push(' '),
                _ => inner.push(Token::Text(TextToken {
                    text: " ".to_string(),
                })),
            }
            if form {
                inner.push(Token::Text(TextToken {
                    text: " − ".to_string(),
                }));
                inner.push(term(params, Param::H, "h".to_string(), form));
            } else {
                let h = signed(params[Param::H], hold.fine.contains(Param::H), MINUS, "+");
                inner.push(term(params, Param::H, h, form));
            }
        } else {
            let den = vec![term(
                params,
                Param::B,
                if form {
                    "b".to_string()
                } else {
                    value(params, Param::B, hold)
                },
                form,
            )];
            let num = std::mem::take(&mut inner);
            inner.push(Token::Frac(FracToken { num, den }));
            has_frac = true;
        }
    }
    let bare = inner.len() == 1 && matches!(inner[0], Token::Text(_));

    let mut body: Vec<Token> = Vec::new();
    match func {
        Base::Sin => {
            body.push(Token::Text(TextToken {
                text: "sin".to_string(),
            }));
            body.push(Token::Paren(ParenToken {
                text: "(".to_string(),
                big: has_frac,
            }));
            body.extend(inner);
            body.push(Token::Paren(ParenToken {
                text: ")".to_string(),
                big: has_frac,
            }));
        }
        Base::X => {
            body.extend(inner);
        }
        Base::X2 | Base::X3 => {
            let exponent = if func == Base::X2 { "2" } else { "3" };
            if bare {
                body.push(Token::Text(TextToken {
                    text: "x".to_string(),
                }));
                body.push(Token::Sup(SupToken {
                    text: exponent.to_string(),
                }));
            } else {
                body.push(Token::Paren(ParenToken {
                    text: "(".to_string(),
                    big: has_frac,
                }));
                body.extend(inner);
                body.push(Token::Paren(ParenToken {
                    text: ")".to_string(),
                    big: has_frac,
                }));
                body.push(Token::Sup(SupToken {
                    text: exponent.to_string(),
                }));
            }
        }
    }

    let mut tokens: Vec<Token> = vec![Token::Text(TextToken {
        text: "f(x) = ".to_string(),
    })];
    if shown(params, Param::A, form, hold) {
        tokens.push(term(
            params,
            Param::A,
            if form {
                "a".to_string()
            } else {
                value(params, Param::A, hold)
            },
            form,
        ));
        if func == Base::Sin {
            tokens.push(Token::Text(TextToken {
                text: " ".to_string(),
            }));
        }
    }
    if func == Base::X && shown(params, Param::A, form, hold) && !bare {
        tokens.push(Token::Paren(ParenToken {
            text: "(".to_string(),
            big: has_frac,
        }));
        tokens.extend(body);
        tokens.push(Token::Paren(ParenToken {
            text: ")".to_string(),
            big: has_frac,
        }));
    } else {
        tokens.extend(body);
    }
    if shown(params, Param::K, form, hold) {
        let k = if form {
            "+ k".to_string()
        } else {
            signed(params[Param::K], hold.fine.contains(Param::K), "+", MINUS)
        };
        tokens.push(Token::Text(TextToken {
            text: " ".to_string(),
        }));
        tokens.push(term(params, Param::K, k, form));
    }
    tokens
}

/// A snippet's tokens: one parameter's phrase.
pub fn snippet_tokens(param: Param, snippet: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let letter = PARAM_LETTERS[param];
    for (i, part) in snippet.split(letter).enumerate() {
        if i > 0 {
            tokens.push(Token::Term(TermToken {
                param,
                text: letter.to_string(),
                ghost: false,
            }));
        }
        if !part.is_empty() {
            tokens.push(Token::Text(TextToken {
                text: part.to_string(),
            }));
        }
    }
    tokens
}

/// The equation in plain words.
pub fn describe_equation(tokens: &[Token]) -> String {
    let mut b = String::new();
    plain(&mut b, tokens);
    let mut out = String::new();
    let mut space = false;
    for r in b.trim().chars() {
        if r.is_whitespace() {
            space = true;
            continue;
        }
        if space {
            out.push(' ');
            space = false;
        }
        out.push(r);
    }
    out
}

/// Writes tokens into a builder.
fn plain(b: &mut String, tokens: &[Token]) {
    fn group(b: &mut String, tokens: &[Token]) {
        let mut part = String::new();
        plain(&mut part, tokens);
        let s = part.trim();
        if s.contains(' ') || s.contains('+') || s.contains('−') {
            b.push('(');
            b.push_str(s);
            b.push(')');
        } else {
            b.push_str(s);
        }
    }
    for token in tokens {
        match token {
            Token::Text(t) => b.push_str(&t.text),
            Token::Term(t) => b.push_str(&t.text),
            Token::Paren(p) => b.push_str(&p.text),
            Token::Sup(s) => match s.text.as_str() {
                "2" => b.push('²'),
                "3" => b.push('³'),
                _ => {
                    b.push('^');
                    b.push_str(&s.text);
                }
            },
            Token::Frac(f) => {
                group(b, &f.num);
                b.push('/');
                group(b, &f.den);
            }
        }
    }
}

#[cfg(test)]
pub(crate) fn params_with(a: f64, k: f64, b: f64, h: f64) -> Params {
    EnumMap::from_array([a, k, b, h])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::form::DEFAULT_PARAMS;

    const M: &str = MINUS;

    fn no_hold() -> LiveHold {
        LiveHold {
            keep: EnumSet::empty(),
            fine: EnumSet::empty(),
        }
    }

    fn says(func: Base, params: Params, mode: EquationMode) -> String {
        describe_equation(&equation_tokens(func, params, mode, no_hold()))
    }

    fn terms(tokens: &[Token], out: &mut Vec<TermToken>) {
        for token in tokens {
            match token {
                Token::Frac(f) => {
                    terms(&f.num, out);
                    terms(&f.den, out);
                }
                Token::Term(t) => out.push(t.clone()),
                _ => {}
            }
        }
    }

    #[test]
    fn the_live_line_hides_parameters_at_their_default() {
        assert_eq!(
            says(Base::X, DEFAULT_PARAMS, EquationMode::Live),
            "f(x) = x"
        );
        assert_eq!(
            says(Base::X2, DEFAULT_PARAMS, EquationMode::Live),
            "f(x) = x²"
        );
        assert_eq!(
            says(Base::X3, DEFAULT_PARAMS, EquationMode::Live),
            "f(x) = x³"
        );
        assert_eq!(
            says(Base::Sin, DEFAULT_PARAMS, EquationMode::Live),
            "f(x) = sin(x)"
        );
    }

    #[test]
    fn the_live_line_prints_the_key_states() {
        assert_eq!(
            says(
                Base::X2,
                params_with(2.0, 1.0, 1.0, -1.0),
                EquationMode::Live
            ),
            "f(x) = 2.00(x + 1.00)² + 1.00"
        );
        assert_eq!(
            says(
                Base::X2,
                params_with(-2.0, 4.0, 1.0, -1.0),
                EquationMode::Live
            ),
            "f(x) = ".to_owned() + M + "2.00(x + 1.00)² + 4.00"
        );
        assert_eq!(
            says(
                Base::X2,
                params_with(1.0, 0.0, 1.0, 1.0),
                EquationMode::Live
            ),
            "f(x) = (x ".to_owned() + M + " 1.00)²"
        );
        assert_eq!(
            says(
                Base::X2,
                params_with(1.0, 0.0, 2.0, 0.0),
                EquationMode::Live
            ),
            "f(x) = (x/2.00)²"
        );
        assert_eq!(
            says(
                Base::X2,
                params_with(1.0, 0.0, 2.0, 1.0),
                EquationMode::Live
            ),
            "f(x) = ((x ".to_owned() + M + " 1.00)/2.00)²"
        );
        let tokens = equation_tokens(
            Base::X2,
            params_with(1.0, 0.0, 2.0, 1.0),
            EquationMode::Live,
            no_hold(),
        );
        let is_frac = matches!(tokens[2], Token::Frac(_));
        assert!(is_frac, "the third token is a fraction");
        assert_eq!(
            says(Base::X, params_with(1.0, 0.0, 1.0, 0.5), EquationMode::Live),
            "f(x) = x ".to_owned() + M + " 0.50"
        );
        assert_eq!(
            says(
                Base::X,
                params_with(1.0, 0.0, 1.0, -0.5),
                EquationMode::Live
            ),
            "f(x) = x + 0.50"
        );
        assert_eq!(
            says(
                Base::X,
                params_with(1.0, -3.0, 1.0, 0.0),
                EquationMode::Live
            ),
            "f(x) = x ".to_owned() + M + " 3.00"
        );
        assert_eq!(
            says(
                Base::X2,
                params_with(-1.0, 0.0, 1.0, 0.0),
                EquationMode::Live
            ),
            "f(x) = ".to_owned() + M + "1.00x²"
        );
        assert_eq!(
            says(Base::X, params_with(3.0, 0.0, 1.0, 0.0), EquationMode::Live),
            "f(x) = 3.00x"
        );
        assert_eq!(
            says(Base::X, params_with(3.0, 0.0, 1.0, 1.0), EquationMode::Live),
            "f(x) = 3.00(x ".to_owned() + M + " 1.00)"
        );
        assert_eq!(
            says(
                Base::Sin,
                params_with(3.0, 0.0, 1.0, 0.0),
                EquationMode::Live
            ),
            "f(x) = 3.00 sin(x)"
        );
        assert_eq!(
            says(
                Base::X2,
                params_with(1.035, 0.0, -1.0, 0.0),
                EquationMode::Live
            ),
            "f(x) = 1.035(x/(".to_owned() + M + "1.00))²"
        );
    }

    #[test]
    fn the_live_line_holds_three_decimals_for_a_fine_drag() {
        let held = equation_tokens(
            Base::X2,
            params_with(2.0, 1.0, 1.0, -1.0),
            EquationMode::Live,
            LiveHold {
                keep: EnumSet::empty(),
                fine: EnumSet::only(Param::K) | Param::H,
            },
        );
        assert_eq!(describe_equation(&held), "f(x) = 2.00(x + 1.000)² + 1.000");
    }

    #[test]
    fn the_parentheses_grow_only_around_a_fraction() {
        fn big(tokens: &[Token]) -> Vec<bool> {
            let mut out = Vec::new();
            for token in tokens {
                if let Token::Paren(p) = token {
                    out.push(p.big);
                }
            }
            out
        }
        let plain = big(&equation_tokens(
            Base::X2,
            params_with(1.0, 0.0, 1.0, 1.0),
            EquationMode::Live,
            no_hold(),
        ));
        assert_eq!(plain.len(), 2);
        assert!(!plain[0] && !plain[1], "(x − 1.00)²'s parentheses stay");
        let grown = big(&equation_tokens(
            Base::Sin,
            params_with(1.0, 0.0, 2.0, 0.0),
            EquationMode::Live,
            no_hold(),
        ));
        assert_eq!(grown.len(), 2);
        assert!(grown[0] && grown[1], "sin((x)/2.00)'s parentheses grow");
    }

    #[test]
    fn each_value_is_its_own_term() {
        let mut live = Vec::new();
        terms(
            &equation_tokens(
                Base::X2,
                params_with(2.0, 1.0, 0.5, -1.0),
                EquationMode::Live,
                no_hold(),
            ),
            &mut live,
        );
        let want = [
            TermToken {
                param: Param::A,
                text: "2.00".to_string(),
                ghost: false,
            },
            TermToken {
                param: Param::H,
                text: "+ 1.00".to_string(),
                ghost: false,
            },
            TermToken {
                param: Param::B,
                text: "0.50".to_string(),
                ghost: false,
            },
            TermToken {
                param: Param::K,
                text: "+ 1.00".to_string(),
                ghost: false,
            },
        ];
        assert_eq!(live.len(), want.len());
        for (got, w) in live.iter().zip(want.iter()) {
            assert_eq!(got, w);
        }
    }

    #[test]
    fn the_form_line_prints_every_letter() {
        assert_eq!(
            says(Base::X2, DEFAULT_PARAMS, EquationMode::Form),
            "f(x) = a((x ".to_owned() + M + " h)/b)² + k"
        );
        assert_eq!(
            says(
                Base::Sin,
                params_with(2.0, 0.0, 1.0, 0.0),
                EquationMode::Form
            ),
            "f(x) = a sin((x ".to_owned() + M + " h)/b) + k"
        );
        assert_eq!(
            says(Base::X, DEFAULT_PARAMS, EquationMode::Form),
            "f(x) = a((x ".to_owned() + M + " h)/b) + k"
        );
    }

    #[test]
    fn the_form_line_marks_letters_at_their_default() {
        let mut form = Vec::new();
        terms(
            &equation_tokens(
                Base::X2,
                params_with(2.0, 1.0, 1.0, -1.0),
                EquationMode::Form,
                no_hold(),
            ),
            &mut form,
        );
        let mut ghosts = EnumSet::empty();
        for term in &form {
            if term.ghost {
                ghosts |= term.param;
            }
        }
        assert_eq!(ghosts, EnumSet::only(Param::B));
    }

    #[test]
    fn a_dragged_term_stays_at_its_default() {
        fn live(keep: EnumSet<Param>) -> String {
            describe_equation(&equation_tokens(
                Base::X2,
                DEFAULT_PARAMS,
                EquationMode::Live,
                LiveHold {
                    keep,
                    fine: EnumSet::empty(),
                },
            ))
        }
        assert_eq!(live(EnumSet::empty()), "f(x) = x²");
        assert_eq!(live(EnumSet::only(Param::K)), "f(x) = x² + 0.00");
        assert_eq!(
            live(EnumSet::only(Param::H) | Param::K),
            "f(x) = (x ".to_owned() + M + " 0.00)² + 0.00"
        );
        assert_eq!(
            live(EnumSet::only(Param::A) | Param::B),
            "f(x) = 1.00(x/1.00)²"
        );
    }
}
