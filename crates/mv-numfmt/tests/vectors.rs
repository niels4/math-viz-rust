//! Conformance tables from Node (see `fixtures/gen_vectors.mjs`): every row
//! is `<input> <digits> <want>`, checked against [`mv_numfmt::to_fixed`]
//! and [`mv_numfmt::to_precision`].
//!
//! Inputs are decimal strings both sides parse with correct rounding, so
//! both read the same double; `-0`, `NaN`, `Infinity`, `-Infinity` are
//! specials. Read with `include_str!` so the wasm target stays file-free.

fn parse_input(text: &str) -> f64 {
    match text {
        "-0" => -0.0,
        "NaN" => f64::NAN,
        "Infinity" => f64::INFINITY,
        "-Infinity" => f64::NEG_INFINITY,
        _ => text
            .parse()
            .unwrap_or_else(|_| panic!("bad input {text:?}")),
    }
}

fn check_rows(table: &str, check: impl Fn(f64, u32) -> String) {
    let mut rows = 0;
    for line in table.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (input, rest) = line.split_once(' ').expect("input and digits");
        let (digits, want) = rest.split_once(' ').expect("digits and want");
        let digits: u32 = digits.parse().expect("digit count");
        let got = check(parse_input(input), digits);
        assert_eq!(got, want, "input {input} digits {digits}");
        rows += 1;
    }
    assert!(rows >= 2000, "only {rows} rows, want at least 2000");
}

#[test]
fn to_fixed_matches_node() {
    check_rows(include_str!("fixtures/to_fixed.txt"), mv_numfmt::to_fixed);
}

#[test]
fn to_precision_matches_node() {
    check_rows(
        include_str!("fixtures/to_precision.txt"),
        mv_numfmt::to_precision,
    );
}

/// `grid.rs`'s `clean` stands in for `Number(v.toPrecision(12))`: the
/// numeric twin [`mv_numfmt::precision`] must parse to the same double on
/// the table's 12-digit rows.
#[test]
fn precision_12_matches_node_numerically() {
    let mut rows = 0;
    for line in include_str!("fixtures/to_precision.txt").lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (input, rest) = line.split_once(' ').expect("input and digits");
        let (digits, want) = rest.split_once(' ').expect("digits and want");
        if digits != "12" {
            continue;
        }
        let v = parse_input(input);
        let want: f64 = want.parse().expect("want parses");
        let got = mv_numfmt::precision(v, 12);
        assert!(
            got == want || (got.is_nan() && want.is_nan()),
            "precision({input}, 12) = {got}, node wants {want}"
        );
        rows += 1;
    }
    assert!(rows > 50, "only {rows} 12-digit rows");
}
