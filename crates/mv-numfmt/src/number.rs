//! Ported from `src/numfmt/number.odin` (231 lines): the page's one number rule.
//!
//! Every public item names its Odin declaration below. This port keeps the
//! Odin code's JavaScript emulation as written: `shift_decimal` moves the
//! decimal point through shortest round-trip digits (so 1.005 shifted by 2 is
//! exactly 100.5), and `fixed` never formats a float with a precision
//! specifier directly — callers round through `round_to` first, because Rust's
//! `{:.N}` rounds ties to even while JavaScript's `toFixed` rounds them up.
//! Float-string work goes through `ftoa`, the stand-in for Odin's
//! `strconv.write_float`.

/// Mathematical minus (U+2212), never the keyboard hyphen.
///
/// Ported from `number.odin:14` (`MINUS :: "−"`).
pub const MINUS: &str = "\u{2212}";

/// Almost equal to (U+2248): the page's fonts take it from their symbols files.
///
/// Ported from `number.odin:17` (`APPROX :: "≈"`).
pub const APPROX: &str = "\u{2248}";

/// The dash a number with no value prints (U+2013).
///
/// Ported from `number.odin:20` (`NO_NUMBER :: "–"`).
pub const NO_NUMBER: &str = "\u{2013}";

/// Decimal places every value shows.
///
/// Ported from `number.odin:23` (`DISPLAY_DP :: 2`).
pub const DISPLAY_DP: i64 = 2;

/// The places a fine drag shows.
///
/// Ported from `number.odin:24` (`FINE_DP :: 3`).
pub const FINE_DP: i64 = 3;

/// Readouts go scientific from here up: fixed digits would outgrow their slots.
///
/// Ported from `number.odin:27` (`SCIENTIFIC_FROM :: 1e5`).
pub const SCIENTIFIC_FROM: f64 = 1e5;

/// How a readout prints a value with fixed decimals.
///
/// Ported from `number.odin:31` (`Fixed :: struct { text, exact }`).
/// `exact`: the text is the value, not a rounding of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fixed {
    pub text: String,
    pub exact: bool,
}

/// How a readout prints a scientific value: a mantissa and an exponent.
///
/// Ported from `number.odin:35` (`Sci :: struct { mantissa, exponent }`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sci {
    pub mantissa: String,
    pub exponent: String,
}

/// How a readout prints a value: fixed decimals, or a mantissa and an exponent.
///
/// Ported from `number.odin:40` (`Parts :: union { Fixed, Sci }`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Parts {
    Fixed(Fixed),
    Sci(Sci),
}

/// `v` as strconv writes it, without the "+" it puts before a positive number.
///
/// Ported from `number.odin:47` (`ftoa`, private). Odin's
/// `strconv.write_float` mirrors Go's `FormatFloat`: `'e'` always carries an
/// exponent, `'f'` with a negative precision prints the shortest digits with
/// no exponent. Rust's `{:e}` (no precision: shortest round-trip) and `{}`
/// (Display: shortest, never scientific) give the same values through the two
/// uses below — `shift_decimal` re-parses the digits, `precision` re-parses
/// the rounded value — so the Go padding details (`e+00`, `e-03`) are not
/// reproduced.
fn ftoa(v: f64, format: u8, prec: i32) -> String {
    match format {
        b'e' => {
            if prec < 0 {
                format!("{v:e}")
            } else {
                format!("{v:.prec$e}", prec = prec as usize)
            }
        }
        b'f' => {
            if prec < 0 {
                format!("{v}")
            } else {
                format!("{v:.prec$}", prec = prec as usize)
            }
        }
        _ => unreachable!("ftoa: unsupported format {format}"),
    }
}

/// `digits` with the sign: U+2212 for negatives.
///
/// Ported from `number.odin:53` (`with_sign`, private).
fn with_sign(negative: bool, digits: &str) -> String {
    if negative {
        format!("{MINUS}{digits}")
    } else {
        digits.to_string()
    }
}

/// `v`'s shortest round-trip digits with the decimal point moved `by` places,
/// read back: decimal arithmetic, so 1.005 shifted by 2 is 100.5.
///
/// Ported from `number.odin:60` (`shift_decimal`, private). Follows the Odin
/// code: shortest `'e'` digits, the exponent shifted by `by`, parsed back.
fn shift_decimal(v: f64, by: i64) -> f64 {
    let digits = ftoa(v, b'e', -1);
    match digits.find('e') {
        Some(pos) => {
            let mantissa = &digits[..pos];
            let exponent: i64 = digits[pos + 1..].parse().unwrap_or(0);
            format!("{mantissa}e{}", exponent + by).parse().unwrap_or(v)
        }
        None => v,
    }
}

/// Rounds half away from zero at `dp` decimals, in decimal.
///
/// Ported from `number.odin:75` (`round_to`).
pub fn round_to(v: f64, dp: i64) -> f64 {
    if v.is_nan() || v.is_infinite() || v == 0.0 {
        return v;
    }
    v.signum() * shift_decimal(shift_decimal(v.abs(), dp).round(), -dp)
}

/// `v`'s exact decimal digits: every binary fraction terminates, so the
/// correctly-rounded `{:.1074}` expansion is exact (1074 is the lowest
/// subnormal's place count). Returns the digits without leading zeros and the
/// power of ten they carry: `v = digits × 10^exp`.
fn exact_digits(v: f64) -> (Vec<u8>, i32) {
    debug_assert!(v.is_finite() && v > 0.0);
    let text = format!("{v:.1074}");
    let (int, frac) = text.split_once('.').unwrap_or((text.as_str(), ""));
    let all = format!("{int}{}", frac.trim_end_matches('0'));
    let leading = all.bytes().take_while(|&b| b == b'0').count();
    let trailing = all.bytes().rev().take_while(|&b| b == b'0').count();
    let digits = all
        .bytes()
        .skip(leading)
        .take(all.len() - leading - trailing)
        .collect::<Vec<u8>>();
    debug_assert!(!digits.is_empty());
    let exp = int.len() as i32 - all.len() as i32 + trailing as i32;
    (digits, exp)
}

/// Adds one to a digit string, growing it on overflow (`b"999"` → `b"1000"`).
fn increment(digits: &[u8]) -> Vec<u8> {
    let mut out = digits.to_vec();
    for digit in out.iter_mut().rev() {
        if *digit == b'9' {
            *digit = b'0';
        } else {
            *digit += 1;
            return out;
        }
    }
    let mut grown = Vec::with_capacity(out.len() + 1);
    grown.push(b'1');
    grown.extend_from_slice(&out);
    grown
}

/// `digits × 10^exp` rounded to a whole number, halves up (the larger `n` on
/// an exact tie, as both `toFixed` and `toPrecision` pick it).
fn round_half_up(digits: &[u8], exp: i32) -> Vec<u8> {
    if exp >= 0 {
        let mut out = digits.to_vec();
        out.extend(std::iter::repeat_n(b'0', exp as usize));
        return out;
    }
    let drop = (-exp) as usize;
    if drop > digits.len() {
        // Below half a unit: no digit reaches the halfway place.
        return vec![b'0'];
    }
    let (kept, dropped) = digits.split_at(digits.len() - drop);
    if dropped[0] < b'5' {
        if kept.is_empty() {
            vec![b'0']
        } else {
            kept.to_vec()
        }
    } else if kept.is_empty() {
        vec![b'1']
    } else {
        increment(kept)
    }
}

/// JavaScript's `Number.prototype.toFixed(f)`: `f` decimals of the exact
/// binary value, halves up, ASCII `-` for negatives, `String(x)` from 1e21 up.
///
/// Panics when `f > 100`, mirroring the `RangeError`.
pub fn to_fixed(x: f64, f: u32) -> String {
    assert!(f <= 100, "to_fixed: {f} fraction digits, at most 100");
    if x.is_nan() {
        return "NaN".to_string();
    }
    if x.is_infinite() {
        return if x > 0.0 {
            "Infinity".to_string()
        } else {
            "-Infinity".to_string()
        };
    }
    if x.abs() >= 1e21 {
        return js_to_string(x);
    }
    let sign = if x < 0.0 { "-" } else { "" };
    // Zero has no digits to expand; both zeros print unsigned.
    let n = if x == 0.0 {
        vec![b'0']
    } else {
        let (digits, exp) = exact_digits(x.abs());
        round_half_up(&digits, exp + f as i32)
    };
    let text = String::from_utf8(n).unwrap_or_default();
    if f == 0 {
        return format!("{sign}{text}");
    }
    let f = f as usize;
    if text.len() <= f {
        format!("{sign}0.{:0>width$}{text}", "", width = f - text.len())
    } else {
        let (int, frac) = text.split_at(text.len() - f);
        format!("{sign}{int}.{frac}")
    }
}

/// JavaScript's `Number.prototype.toPrecision(p)`: `p` significant digits of
/// the exact binary value, halves up on ties, the exponential form when the
/// decimal exponent is below -6 or at least `p`.
///
/// Panics when `p` is outside 1..=100, mirroring the `RangeError`.
pub fn to_precision(x: f64, p: u32) -> String {
    assert!((1..=100).contains(&p), "to_precision: {p} digits, 1 to 100");
    if x.is_nan() {
        return "NaN".to_string();
    }
    if x.is_infinite() {
        return if x > 0.0 {
            "Infinity".to_string()
        } else {
            "-Infinity".to_string()
        };
    }
    let sign = if x < 0.0 { "-" } else { "" };
    if x == 0.0 {
        // Both zeros print unsigned with `p - 1` decimals (`(0).toPrecision(3)`
        // is `"0.00"`).
        let zeros = "0".repeat(p as usize - 1);
        return if p == 1 {
            "0".to_string()
        } else {
            format!("0.{zeros}")
        };
    }
    let (digits, exp) = exact_digits(x.abs());
    let p_len = p as usize;
    let len = digits.len();
    // The decimal exponent: `digits` has `len` places, so the value sits in
    // `[10^(exp+len-1), 10^(exp+len))`.
    let mut e = exp + len as i32 - 1;
    let mut n = if len >= p_len {
        let (head, rest) = digits.split_at(p_len);
        // An empty rest is exact: nothing to round up.
        if rest.first().is_some_and(|&b| b >= b'5') {
            increment(head)
        } else {
            head.to_vec()
        }
    } else {
        // Moved, not cloned: `len` above holds the length past the move.
        let mut padded = digits;
        padded.extend(std::iter::repeat_n(b'0', p_len - len));
        padded
    };
    if n.len() > p_len {
        // The rounding carried into a new leading digit (`999` → `1000`).
        n.pop();
        e += 1;
    }
    let text = String::from_utf8(n).unwrap_or_default();
    if e < -6 || e >= p as i32 {
        let (head, tail) = text.split_at(1);
        let mantissa = if p_len == 1 {
            head.to_string()
        } else {
            format!("{head}.{tail}")
        };
        let exp_sign = if e >= 0 { "+" } else { "-" };
        return format!("{sign}{mantissa}e{exp_sign}{}", e.abs());
    }
    if e >= 0 {
        let (int, frac) = text.split_at(e as usize + 1);
        if frac.is_empty() {
            format!("{sign}{int}")
        } else {
            format!("{sign}{int}.{frac}")
        }
    } else {
        format!("{sign}0.{:0>width$}{text}", "", width = (-(e + 1)) as usize)
    }
}

/// JavaScript's `String(x)` for a number: shortest digits that round-trip,
/// plain decimals near 1, otherwise one leading digit with `e+`/`e-`.
///
/// `(-0).toString()` is `"0"`, as here: only `x < 0.0` takes the minus.
pub fn js_to_string(x: f64) -> String {
    if x.is_nan() {
        return "NaN".to_string();
    }
    if x.is_infinite() {
        return if x > 0.0 {
            "Infinity".to_string()
        } else {
            "-Infinity".to_string()
        };
    }
    if x == 0.0 {
        return "0".to_string();
    }
    let sign = if x < 0.0 { "-" } else { "" };
    // Rust's Display is shortest-round-trip and never scientific, so its
    // digits are the spec's shortest `s`; trailing zeros never round-trip
    // shorter than without them.
    let text = format!("{}", x.abs());
    let (int, frac) = text.split_once('.').unwrap_or((text.as_str(), ""));
    let all = format!("{int}{frac}");
    let stripped = all.trim_matches('0');
    // A nonzero value keeps a nonzero digit.
    let digits = if stripped.is_empty() { "0" } else { stripped };
    // `n` in the spec's `s × 10^(n-k)`: the point stood after `int.len()`
    // digits, and each stripped trailing zero moves it one place up.
    let n = int.len() as i32 - all.len() as i32
        + (all.len() - all.trim_end_matches('0').len()) as i32
        + digits.len() as i32;
    let k = digits.len() as i32;
    if k <= n && n <= 21 {
        format!("{sign}{digits}{}", "0".repeat((n - k) as usize))
    } else if 0 < n && n <= 21 {
        let (head, tail) = digits.split_at(n as usize);
        format!("{sign}{head}.{tail}")
    } else if -6 < n && n <= 0 {
        format!("{sign}0.{}{digits}", "0".repeat((-n) as usize))
    } else {
        let (head, tail) = digits.split_at(1);
        let mantissa = if tail.is_empty() {
            head.to_string()
        } else {
            format!("{head}.{tail}")
        };
        let e = n - 1;
        let exp_sign = if e >= 0 { "+" } else { "-" };
        format!("{sign}{mantissa}e{exp_sign}{}", e.abs())
    }
}

/// `v` to `digits` significant digits (JavaScript's `Number(v.toPrecision(digits))`); never -0.
///
/// Ported from `number.odin:83` (`precision`).
pub fn precision(v: f64, digits: i64) -> f64 {
    if v.is_nan() || v.is_infinite() {
        return v;
    }
    let text = ftoa(v, b'e', (digits - 1) as i32);
    let out: f64 = text.parse().unwrap_or(v);
    if out == 0.0 { 0.0 } else { out }
}

/// Snaps to a multiple of `step` without float noise; ties away from zero; never -0.
///
/// Ported from `number.odin:95` (`quantize`).
pub fn quantize(v: f64, step: f64) -> f64 {
    precision(v.signum() * (v.abs() / step).round() * step, 12)
}

/// `v` with exactly `dp` decimals (JavaScript's toFixed), for a value already rounded there.
///
/// Ported from `number.odin:101` (`fixed`, private). The value arrives rounded
/// through `round_to`, so Rust's `{:.N}` only prints digits; the tie direction
/// (Rust rounds ties to even, `toFixed` rounds them up) was settled one step
/// earlier, in decimal.
fn fixed(v: f64, dp: i64) -> String {
    ftoa(v, b'f', dp as i32)
}

/// A value: exactly `dp` decimals, U+2212 for negatives, never -0.
///
/// Ported from `number.odin:107` (`format_number`; the default `dp := DISPLAY_DP`
/// is explicit here: Rust has no default arguments).
pub fn format_number(v: f64, dp: i64) -> String {
    let rounded = round_to(v, dp);
    with_sign(rounded < 0.0, &fixed(rounded.abs(), dp))
}

/// The decimals a stored value shows: 3 when it has a third, else 2.
///
/// Ported from `number.odin:114` (`stored_dp`; the default `fine := false`
/// is explicit here).
pub fn stored_dp(v: f64, fine: bool) -> i64 {
    if fine || round_to(v, DISPLAY_DP) != round_to(v, FINE_DP) {
        FINE_DP
    } else {
        DISPLAY_DP
    }
}

/// A stored value at its decimals (stored_dp).
///
/// Ported from `number.odin:119` (`format_stored`).
pub fn format_stored(v: f64, fine: bool) -> String {
    format_number(v, stored_dp(v, fine))
}

/// Typed text back to a number: what a value field accepts.
///
/// Ported from `number.odin:126` (`parse_number -> (value, ok)`). Either minus
/// (U+2212 as printed, or the keyboard's hyphen), no other text; false when
/// the text is not one finite number ("", "1.0.4", "Infinity").
pub fn parse_number(text: &str) -> (f64, bool) {
    let trimmed = text.trim();
    let s = trimmed.replace(MINUS, "-");
    let b = s.as_bytes();
    // [-+]?(\d+\.?\d*|\.\d+)(e[-+]?\d+)?, case-insensitive, matched whole.
    let mut i = 0;
    if i < b.len() && (b[i] == b'-' || b[i] == b'+') {
        i += 1;
    }
    let int_digits = digits_at(&s, &mut i);
    let mut frac_digits = 0;
    if i < b.len() && b[i] == b'.' {
        i += 1;
        frac_digits = digits_at(&s, &mut i);
    }
    if int_digits == 0 && frac_digits == 0 {
        return (0.0, false);
    }
    if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        i += 1;
        if i < b.len() && (b[i] == b'-' || b[i] == b'+') {
            i += 1;
        }
        if digits_at(&s, &mut i) == 0 {
            return (0.0, false);
        }
    }
    if i != b.len() {
        return (0.0, false);
    }
    match s.parse::<f64>() {
        Ok(value) if value.is_finite() => (value, true),
        _ => (0.0, false),
    }
}

/// Counts the ASCII digits at `i`, advancing past them.
///
/// Ported from `number.odin:163` (`digits_at`, private). Byte-indexed like
/// the Odin code; a non-ASCII byte simply ends the run.
fn digits_at(s: &str, i: &mut usize) -> usize {
    let b = s.as_bytes();
    let mut count = 0;
    while *i < b.len() && b[*i].is_ascii_digit() {
        *i += 1;
        count += 1;
    }
    count
}

/// How a readout prints a value: fixed below 100000, scientific from there up.
///
/// Ported from `number.odin:174` (`number_parts`).
pub fn number_parts(v: f64) -> Parts {
    if v.is_nan() || v.is_infinite() {
        return Parts::Fixed(Fixed {
            text: NO_NUMBER.to_string(),
            exact: false,
        });
    }
    if round_to(v, DISPLAY_DP).abs() < SCIENTIFIC_FROM {
        return Parts::Fixed(Fixed {
            text: format_number(v, DISPLAY_DP),
            exact: (round_to(v, DISPLAY_DP) - v).abs() < 1e-9,
        });
    }
    let mut exponent = v.abs().log10().floor();
    let mut mantissa = round_to(v / 10f64.powf(exponent), DISPLAY_DP);
    // 9.999e7 rounds its mantissa up to 10: renormalised to 1.00 × 10⁸.
    if mantissa.abs() >= 10.0 {
        exponent += 1.0;
        mantissa = round_to(v / 10f64.powf(exponent), DISPLAY_DP);
    }
    Parts::Sci(Sci {
        mantissa: format_number(mantissa, DISPLAY_DP),
        exponent: with_sign(exponent < 0.0, &(exponent.abs() as i64).to_string()),
    })
}

/// "=" when the readout shows the value exactly, "≈" when it shows a rounding.
///
/// Ported from `number.odin:194` (`relation`).
pub fn relation(v: f64) -> String {
    match number_parts(v) {
        Parts::Fixed(f) if f.exact => "=".to_string(),
        _ => APPROX.to_string(),
    }
}

/// The short form, for numbers that never slide: `dp` decimals, zeros stripped.
///
/// Ported from `number.odin:204` (`format_mark`; the default `dp := DISPLAY_DP`
/// is explicit here).
pub fn format_mark(v: f64, dp: i64) -> String {
    let rounded = round_to(v, dp);
    if rounded == 0.0 {
        return "0".to_string();
    }
    let mut digits = fixed(rounded.abs(), dp);
    if digits.contains('.') {
        digits = digits.trim_end_matches('0').to_string();
        if let Some(stripped) = digits.strip_suffix('.') {
            digits = stripped.to_string();
        }
    }
    with_sign(rounded < 0.0, &digits)
}

/// A tick label: the short form, two significant figures below 0.01, ≈0 below 0.0001.
///
/// Ported from `number.odin:219` (`format_tick`).
pub fn format_tick(v: f64) -> String {
    let a = v.abs();
    let finite = v.is_finite();
    if !finite || a == 0.0 || a >= 0.01 {
        return if finite {
            format_mark(v, DISPLAY_DP)
        } else {
            NO_NUMBER.to_string()
        };
    }
    if a < 1e-4 {
        return format!("{APPROX}0");
    }
    let rounded = precision(v, 2);
    with_sign(rounded < 0.0, &ftoa(rounded.abs(), b'f', -1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_number_prints_fixed_decimals() {
        // The user's ruling (2026-10-06): fixed decimals, so the decimal point holds still while a
        // value slides (1 → 1.01 → 1.1 no longer changes length).
        assert_eq!(format_number(2.0, DISPLAY_DP), "2.00");
        assert_eq!(format_number(2.5, DISPLAY_DP), "2.50");
        assert_eq!(format_number(2.499, DISPLAY_DP), "2.50");
        assert_eq!(format_number(1234.5, DISPLAY_DP), "1234.50");
        assert_eq!(format_number(0.30000000000000004, DISPLAY_DP), "0.30");
        for v in [1.0, 1.01, 1.1, 1.11] {
            assert_eq!(format_number(v, DISPLAY_DP).len(), 4);
        }
        // The decimals asked for.
        assert_eq!(format_number(1.03511, 3), "1.035");
        assert_eq!(format_number(1.0, 3), "1.000");
        assert_eq!(format_number(99.6, 0), "100");
        // One stored value as one string.
        assert_eq!(format_number(1.03511, DISPLAY_DP), "1.04");
        // U+2212 for negatives, never −0.
        assert_eq!(format_number(-1.0, DISPLAY_DP), format!("{MINUS}1.00"));
        assert_eq!(format_number(-2.25, DISPLAY_DP), format!("{MINUS}2.25"));
        assert_eq!(format_number(-0.0, DISPLAY_DP), "0.00");
        assert_eq!(format_number(-0.004, DISPLAY_DP), "0.00");
        // Decimal halves away from zero.
        assert_eq!(format_number(1.005, DISPLAY_DP), "1.01");
        assert_eq!(format_number(-1.005, DISPLAY_DP), format!("{MINUS}1.01"));
        assert_eq!(format_number(0.125, DISPLAY_DP), "0.13");
        assert_eq!(format_number(-0.125, DISPLAY_DP), format!("{MINUS}0.13"));
    }

    #[test]
    fn format_stored_prints_a_third_decimal_when_held() {
        assert_eq!(format_stored(2.0, false), "2.00");
        assert_eq!(format_stored(1.04, false), "1.04");
        assert_eq!(format_stored(1.035, false), "1.035");
        assert_eq!(format_stored(-0.5, false), format!("{MINUS}0.50"));
        assert_eq!(format_stored(1.04 + 1e-12, false), "1.04");
        // 3 decimals while a fine drag holds them, whatever the value.
        assert_eq!(format_stored(2.0, true), "2.000");
        assert_eq!(format_stored(1.04, true), "1.040");
        assert_eq!(format_stored(1.035, true), "1.035");
    }

    fn expect_fixed(v: f64, text: &str, exact: bool) {
        match number_parts(v) {
            Parts::Fixed(f) => assert!(
                f.text == text && f.exact == exact,
                "number_parts({v}) = Fixed({f:?}), want fixed {text:?} exact {exact}"
            ),
            Parts::Sci(s) => {
                panic!("number_parts({v}) = Sci({s:?}), want fixed {text:?} exact {exact}")
            }
        }
    }

    fn expect_sci(v: f64, mantissa: &str, exponent: &str) {
        match number_parts(v) {
            Parts::Sci(s) => assert!(
                s.mantissa == mantissa && s.exponent == exponent,
                "number_parts({v}) = Sci({s:?}), want {mantissa} × 10^{exponent}"
            ),
            Parts::Fixed(f) => {
                panic!("number_parts({v}) = Fixed({f:?}), want {mantissa} × 10^{exponent}")
            }
        }
    }

    #[test]
    // 3.14 is the Odin test's literal (FV 10's f(3.14)), not an approximation of PI.
    #[allow(clippy::approx_constant)]
    fn number_parts_and_relation() {
        // The design's key-state readouts: R2 f(2) = 4, R3 f(0.5) = 5.5, R8 f(0.5) = −0.5.
        expect_fixed(4.0, "4.00", true);
        expect_fixed(5.5, "5.50", true);
        expect_fixed(-0.5, &format!("{MINUS}0.50"), true);
        assert_eq!(relation(4.0), "=");
        // Rounded readouts take ≈: FV 10's f(1.8) ≈ 5.83 on x³, Components' f(−3) ≈ 10.56.
        expect_fixed(1.8f64.powf(3.0), "5.83", false);
        expect_fixed(10.5625, "10.56", false);
        assert_eq!(relation(1.8f64.powf(3.0)), APPROX);
        // Fixed digits up to 100000: FV 10's f(30) = 27000 on x³.
        expect_fixed(30.0f64.powf(3.0), "27000.00", true);
        expect_fixed(99999.99, "99999.99", true);
        // Too small for 2 decimals: 0.00, rounded (FV 10's f(3.14) ≈ 0.0016 on sin x).
        expect_fixed(3.14f64.sin(), "0.00", false);
        expect_fixed(-0.0016, "0.00", false);
        expect_fixed(-2.5e-7, "0.00", false);
        assert_eq!(relation(0.001), APPROX);
        // Scientific from 100000, the mantissa at 2 decimals.
        expect_sci(1e5, "1.00", "5");
        expect_sci(125000.0, "1.25", "5");
        expect_sci(-2.5e7, &format!("{MINUS}2.50"), "7");
        assert_eq!(relation(125000.0), APPROX);
        // A mantissa that rounds up to 10 renormalises.
        expect_sci(99999.996, "1.00", "5");
        expect_sci(9.9999e7, "1.00", "8");
        // Zero exactly, a non-number as a dash.
        expect_fixed(0.0, "0.00", true);
        expect_fixed(f64::NAN, NO_NUMBER, false);
    }

    #[test]
    fn format_mark_keeps_the_short_form() {
        assert_eq!(format_mark(2.0, DISPLAY_DP), "2");
        assert_eq!(format_mark(0.9, DISPLAY_DP), "0.9");
        assert_eq!(format_mark(1.25, DISPLAY_DP), "1.25");
        assert_eq!(format_mark(-0.5, DISPLAY_DP), format!("{MINUS}0.5"));
        assert_eq!(format_mark(-0.004, DISPLAY_DP), "0");
        assert_eq!(format_mark(80.0, 0), "80");
        assert_eq!(format_mark(6.3, 1), "6.3");
        assert_eq!(format_mark(5.0, 1), "5");
    }

    #[test]
    fn format_tick_labels_the_axes() {
        assert_eq!(format_tick(2.0), "2");
        assert_eq!(format_tick(-1.5), format!("{MINUS}1.5"));
        assert_eq!(format_tick(250000.0), "250000");
        assert_eq!(format_tick(0.0), "0");
        // A deep zoom's ticks stay apart; tiny ones read ≈0.
        assert_eq!(format_tick(0.0016), "0.0016");
        assert_eq!(format_tick(-0.0005), format!("{MINUS}0.0005"));
        assert_eq!(format_tick(2e-6), format!("{APPROX}0"));
        assert_eq!(format_tick(-3e-5), format!("{APPROX}0"));
    }

    #[test]
    fn round_to_and_quantize() {
        // In decimal.
        assert_eq!(round_to(1.005, 2), 1.01);
        assert_eq!(round_to(-2.675, 2), -2.68);
        assert_eq!(round_to(1e-7, 2), 0.0);
        // To the step without float noise.
        assert_eq!(quantize(0.1 + 0.2, 0.01), 0.3);
        assert_eq!(quantize(1.2345, 0.01), 1.23);
        assert_eq!(quantize(1.0004, 0.001), 1.0);
        assert_eq!(quantize(0.4, 0.25), 0.5);
        assert_eq!(quantize(-0.126, 0.25), -0.25);
        // Ties away from zero, so a value and its negative snap alike; -0 never.
        assert_eq!(quantize(0.375, 0.25), 0.5);
        assert_eq!(quantize(-0.375, 0.25), -0.5);
        assert_eq!(quantize(-0.5, 1.0), -1.0);
        let zero = quantize(-0.001, 0.01);
        assert!(
            zero == 0.0 && !zero.is_sign_negative(),
            "quantize(-0.001, 0.01) = {zero}, want +0"
        );
    }

    #[test]
    fn exact_digits_reads_the_binary_value_not_the_shortest_text() {
        // 0.1's shortest text is "0.1"; its value carries the full tail.
        let (digits, exp) = exact_digits(0.1);
        assert_eq!(
            (String::from_utf8(digits).unwrap(), exp),
            (
                "1000000000000000055511151231257827021181583404541015625".to_string(),
                -55
            )
        );
        // An integer carries a positive exponent.
        let (digits, exp) = exact_digits(100.0);
        assert_eq!(
            (String::from_utf8(digits).unwrap(), exp),
            ("1".to_string(), 2)
        );
    }

    #[test]
    fn to_fixed_rounds_the_binary_value_halves_up() {
        // The binary value decides, not the decimal digits: 2.675 is
        // 2.6749999... underneath, 0.125 is exact.
        assert_eq!(to_fixed(2.675, 2), "2.67");
        assert_eq!(to_fixed(-2.675, 2), "-2.67");
        assert_eq!(to_fixed(1.0005, 3), "1.000");
        assert_eq!(to_fixed(0.125, 2), "0.13");
        assert_eq!(to_fixed(1.005, 2), "1.00");
        // Exact ties take the larger n; the sign stays on the outside.
        assert_eq!(to_fixed(2.5, 0), "3");
        assert_eq!(to_fixed(-2.5, 0), "-3");
        assert_eq!(to_fixed(0.5, 0), "1");
        // Zeros print unsigned, nonzero values keep their minus.
        assert_eq!(to_fixed(-0.0, 4), "0.0000");
        assert_eq!(to_fixed(-0.00001, 4), "-0.0000");
        assert_eq!(to_fixed(0.00004, 4), "0.0000");
        assert_eq!(to_fixed(0.0, 2), "0.00");
        // Plain cases and padding.
        assert_eq!(to_fixed(2.0, 2), "2.00");
        assert_eq!(to_fixed(99.6, 0), "100");
        assert_eq!(to_fixed(1.23e-7, 6), "0.000000");
        assert_eq!(
            to_fixed(123456789012345680000.0, 0),
            "123456789012345683968"
        );
        // From 1e21 up, `String(x)`.
        assert_eq!(to_fixed(1e21, 2), "1e+21");
        assert_eq!(to_fixed(-1e21, 0), "-1e+21");
        // Non-finite values print as `String` prints them.
        assert_eq!(to_fixed(f64::NAN, 2), "NaN");
        assert_eq!(to_fixed(f64::INFINITY, 2), "Infinity");
        assert_eq!(to_fixed(f64::NEG_INFINITY, 2), "-Infinity");
    }

    #[test]
    fn to_precision_counts_significant_digits_halves_up() {
        assert_eq!(to_precision(0.5, 1), "0.5");
        assert_eq!(to_precision(0.00001, 3), "0.0000100");
        assert_eq!(to_precision(123456.0, 3), "1.23e+5");
        assert_eq!(to_precision(999.9, 3), "1.00e+3");
        assert_eq!(to_precision(99.99, 3), "100");
        assert_eq!(to_precision(0.0000123, 2), "0.000012");
        assert_eq!(to_precision(1.005, 3), "1.00");
        assert_eq!(to_precision(1.0, 1), "1");
        assert_eq!(to_precision(100.0, 3), "100");
        assert_eq!(to_precision(0.01, 3), "0.0100");
        // A carry into a new leading digit moves the exponent.
        assert_eq!(to_precision(9.999, 3), "10.0");
        // Zeros print unsigned.
        assert_eq!(to_precision(0.0, 3), "0.00");
        assert_eq!(to_precision(-0.0, 3), "0.00");
        assert_eq!(to_precision(-0.0, 1), "0");
        // Non-finite values print as `String` prints them.
        assert_eq!(to_precision(f64::NAN, 3), "NaN");
        assert_eq!(to_precision(f64::INFINITY, 3), "Infinity");
        assert_eq!(to_precision(f64::NEG_INFINITY, 3), "-Infinity");
    }

    #[test]
    fn js_to_string_matches_number_to_string() {
        assert_eq!(js_to_string(0.0), "0");
        assert_eq!(js_to_string(-0.0), "0");
        assert_eq!(js_to_string(2.5), "2.5");
        assert_eq!(js_to_string(-2.5), "-2.5");
        assert_eq!(js_to_string(0.30000000000000004), "0.30000000000000004");
        assert_eq!(js_to_string(1e21), "1e+21");
        assert_eq!(js_to_string(-1.5e21), "-1.5e+21");
        assert_eq!(js_to_string(1.23e-7), "1.23e-7");
        assert_eq!(js_to_string(1e-6), "0.000001");
        assert_eq!(js_to_string(100.0), "100");
        assert_eq!(js_to_string(f64::NAN), "NaN");
        assert_eq!(js_to_string(f64::INFINITY), "Infinity");
        assert_eq!(js_to_string(f64::NEG_INFINITY), "-Infinity");
    }

    #[test]
    fn parse_number_reads_what_a_field_prints() {
        let minus_two = format!("{MINUS}2.00");
        let reads = [
            ("1.04", 1.04),
            (minus_two.as_str(), -2.0),
            (" -0.5 ", -0.5),
            (".5", 0.5),
            ("2.", 2.0),
            ("1e3", 1000.0),
        ];
        for (text, want) in reads {
            let (value, ok) = parse_number(text);
            assert!(
                ok && value == want,
                "parse_number({text:?}) = {value} {ok}, want {want}"
            );
        }
        for text in [
            "", " ", "-", "1.0.4", "abc", "0x10", "Infinity", "1e999", "1,5",
        ] {
            let (value, ok) = parse_number(text);
            assert!(!ok, "parse_number({text:?}) = {value}, want none");
        }
    }
}
