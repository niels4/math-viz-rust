//! The sidebar function picker's options and the `func` page parameter.
//!
//! Ported from math-viz2 at `3560eb2`:
//!
//! - `src/components/views/FunctionViewer/baseFunctions.ts`: [`base`], the
//!   function each picker option graphs.
//! - `src/components/views/FunctionViewer/queryParams.ts`: [`FunctionSlug`],
//!   [`FUNCTION_SLUGS`], [`FUNC_PARAM`] (key `func`, default `x1`).
//! - `src/components/views/FunctionViewer/baseFunctions.ts` (at `5cef052`):
//!   [`unit_x`], the plane x where a function shows its scale handle.
//!
//! Extended at `5714319` from the same file's `transformParam` and
//! `transformParams`: [`A_PARAM`], [`B_PARAM`], [`H_PARAM`], [`K_PARAM`] (the
//! transform `y = a · f((x − h) / b) + k`; scales default 1, shifts 0, a
//! scale of 0 never reads).

use std::f64::consts::FRAC_PI_2;

use crate::plot::PlotFunction;
use crate::url::{DEFAULT_DEBOUNCE_MS, QueryParamDef, default_equals, parse_js_number};

/// The function picker's options, on the wire as `x1`, `x2`, `x3`, `sinx`
/// (`FunctionSlug` in `queryParams.ts`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FunctionSlug {
    X1,
    X2,
    X3,
    SinX,
}

// `Object.is` on slugs is `==` (no -0/NaN in an enum); the impl satisfies
// `default_equals`' bound for `FUNC_PARAM`.
impl crate::url::ObjectIs for FunctionSlug {
    fn object_is(&self, other: &Self) -> bool {
        self == other
    }
}

impl FunctionSlug {
    /// The wire string: the `FUNCTION_SLUGS` entry.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::X1 => "x1",
            Self::X2 => "x2",
            Self::X3 => "x3",
            Self::SinX => "sinx",
        }
    }
}

/// The picker's options in order (`FUNCTION_SLUGS`).
pub const FUNCTION_SLUGS: [FunctionSlug; 4] = [
    FunctionSlug::X1,
    FunctionSlug::X2,
    FunctionSlug::X3,
    FunctionSlug::SinX,
];

/// The function a picker option graphs (`baseFunctions`): `x`, `x²`, `x³`,
/// `sin x` (`Math.sin`).
#[must_use]
pub fn base(slug: FunctionSlug) -> PlotFunction {
    match slug {
        FunctionSlug::X1 => |x| x,
        FunctionSlug::X2 => |x| x * x,
        FunctionSlug::X3 => |x| x * x * x,
        FunctionSlug::SinX => f64::sin,
    }
}

/// The plane x where a picker option shows its scale handle (`unitX`): 1
/// for `x`, `x²` and `x³`, π/2 (the peak) for `sin x`.
#[must_use]
pub fn unit_x(slug: FunctionSlug) -> f64 {
    match slug {
        FunctionSlug::X1 | FunctionSlug::X2 | FunctionSlug::X3 => 1.0,
        FunctionSlug::SinX => FRAC_PI_2,
    }
}

/// `funcParam`'s parser: only a listed slug reads; anything else (`None` for
/// a missing key) falls back to the default, as the TS `undefined` does.
fn parse_func(raw: Option<&str>) -> Option<FunctionSlug> {
    match raw {
        Some("x1") => Some(FunctionSlug::X1),
        Some("x2") => Some(FunctionSlug::X2),
        Some("x3") => Some(FunctionSlug::X3),
        Some("sinx") => Some(FunctionSlug::SinX),
        _ => None,
    }
}

/// `funcParam`'s serialiser: the slug itself, always. The default `x1` stays
/// out of the URL through `url_value`'s serialise-equals-default deletion,
/// as the TS comment promises.
// Always `Some`: the `Option` satisfies the def's fn-pointer field type (a
// `None` would delete the key, which this param never does, as the TS
// `(v) => v` never returns `null`).
#[allow(clippy::unnecessary_wraps)]
fn serialize_func(slug: &FunctionSlug) -> Option<String> {
    Some(slug.as_str().to_owned())
}

/// `funcParam`: the function the plane graphs (key `func`, default `x1`).
pub static FUNC_PARAM: QueryParamDef<FunctionSlug> = QueryParamDef {
    key: "func",
    default: FunctionSlug::X1,
    debounce_ms: DEFAULT_DEBOUNCE_MS,
    parse: parse_func,
    serialize: serialize_func,
    equals: default_equals,
};

/// `transformParam`'s parser: JS `Number(raw)` ([`parse_js_number`]), finite,
/// and never 0 for a scale (a scale of 0 would squash the curve flat, so the
/// URL can't set one). A missing key (`None`) falls back to the default, as
/// the TS `undefined` does. Note the empty text commits 0 for a shift
/// (`Number("") === 0`) and restores a scale.
const fn parse_transform(is_scale: bool) -> fn(Option<&str>) -> Option<f64> {
    if is_scale { parse_scale } else { parse_shift }
}

/// A scale's parser (a or b).
fn parse_scale(raw: Option<&str>) -> Option<f64> {
    let n = parse_js_number(raw?)?;
    (n.is_finite() && n != 0.0).then_some(n)
}

/// A shift's parser (h or k).
fn parse_shift(raw: Option<&str>) -> Option<f64> {
    let n = parse_js_number(raw?)?;
    n.is_finite().then_some(n)
}

/// `transformParam` serialiser: `String(Number(v.toFixed(3)))`, through
/// `mv-numfmt` (exact `to_fixed`, then `js_to_string` on the re-parse, nested
/// as the TS nests them: `format_scale` in `camera.rs` is the precedent).
// Always `Some`: the `Option` satisfies the def's fn-pointer field type (a
// `None` would delete the key, which this serialiser never does, as the TS
// `(v) => String(...)` never returns `null`).
#[allow(clippy::unnecessary_wraps)]
fn serialize_transform(value: &f64) -> Option<String> {
    // The fixed string always parses (digits with at most one `.`), so this
    // nests the two conversions as the TS does.
    let n: f64 = mv_numfmt::to_fixed(*value, 3).parse().unwrap_or(f64::NAN);
    Some(mv_numfmt::js_to_string(n))
}

/// One transform parameter's def: `transformParam(key, default, isScale)`.
macro_rules! transform_param {
    ($name:ident, $key:literal, $default:literal, $is_scale:literal) => {
        /// The transform parameter's page param (`transformParam`).
        pub static $name: QueryParamDef<f64> = QueryParamDef {
            key: $key,
            default: $default,
            debounce_ms: DEFAULT_DEBOUNCE_MS,
            parse: parse_transform($is_scale),
            serialize: serialize_transform,
            equals: default_equals,
        };
    };
}

transform_param!(A_PARAM, "a", 1.0, true);
transform_param!(B_PARAM, "b", 1.0, true);
transform_param!(H_PARAM, "h", 0.0, false);
transform_param!(K_PARAM, "k", 0.0, false);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::url::{QueryParam, Url};

    #[test]
    fn slug_wire_strings_follow_function_slugs_order() {
        let wires: Vec<&str> = FUNCTION_SLUGS.iter().map(|slug| slug.as_str()).collect();
        assert_eq!(wires, vec!["x1", "x2", "x3", "sinx"]);
    }

    #[test]
    fn base_graphs_each_option() {
        assert_eq!(base(FunctionSlug::X1)(0.5), 0.5);
        assert_eq!(base(FunctionSlug::X2)(-2.0), 4.0);
        assert_eq!(base(FunctionSlug::X3)(2.0), 8.0);
        assert_eq!(base(FunctionSlug::SinX)(0.0), 0.0);
        assert!((base(FunctionSlug::SinX)(std::f64::consts::FRAC_PI_2) - 1.0).abs() < 1e-15);
    }

    #[test]
    fn func_param_reads_x1_by_default_and_falls_back_to_it() {
        assert_eq!(*QueryParam::new(&FUNC_PARAM, "").get(), FunctionSlug::X1);
        assert_eq!(
            *QueryParam::new(&FUNC_PARAM, "func=x3").get(),
            FunctionSlug::X3
        );
        assert_eq!(
            *QueryParam::new(&FUNC_PARAM, "func=sinx").get(),
            FunctionSlug::SinX
        );
        assert_eq!(
            *QueryParam::new(&FUNC_PARAM, "func=x9").get(),
            FunctionSlug::X1
        );
        assert_eq!(
            *QueryParam::new(&FUNC_PARAM, "func=").get(),
            FunctionSlug::X1
        );
    }

    #[test]
    fn func_param_keeps_the_default_out_of_the_url() {
        let mut url = Url::from_query("");
        let mut param = QueryParam::new(&FUNC_PARAM, "");
        param.set_flushed(&mut url, FunctionSlug::X1);
        assert_eq!(url.query(), "");
        param.set_flushed(&mut url, FunctionSlug::X2);
        assert_eq!(url.query(), "func=x2");
    }

    #[test]
    fn unit_x_is_1_except_for_the_sine_peak() {
        assert_eq!(unit_x(FunctionSlug::X1), 1.0);
        assert_eq!(unit_x(FunctionSlug::X2), 1.0);
        assert_eq!(unit_x(FunctionSlug::X3), 1.0);
        assert_eq!(unit_x(FunctionSlug::SinX), FRAC_PI_2);
    }

    #[test]
    fn transform_params_default_to_the_identity() {
        assert_eq!(*QueryParam::new(&A_PARAM, "").get(), 1.0);
        assert_eq!(*QueryParam::new(&B_PARAM, "").get(), 1.0);
        assert_eq!(*QueryParam::new(&H_PARAM, "").get(), 0.0);
        assert_eq!(*QueryParam::new(&K_PARAM, "").get(), 0.0);
    }

    #[test]
    fn transform_params_read_a_link_and_reject_scales_of_zero() {
        assert_eq!(*QueryParam::new(&A_PARAM, "a=2").get(), 2.0);
        assert_eq!(*QueryParam::new(&K_PARAM, "k=-1").get(), -1.0);
        assert_eq!(*QueryParam::new(&B_PARAM, "b=0.5").get(), 0.5);
        assert_eq!(*QueryParam::new(&H_PARAM, "h=3").get(), 3.0);
        // A scale of 0 squashes the curve flat, so the URL can't set one.
        assert_eq!(*QueryParam::new(&A_PARAM, "a=0").get(), 1.0);
        assert_eq!(*QueryParam::new(&B_PARAM, "b=0").get(), 1.0);
        assert_eq!(*QueryParam::new(&H_PARAM, "h=0").get(), 0.0);
        // Unparseable text falls back to the default, as `queryParamAtom` does.
        assert_eq!(*QueryParam::new(&A_PARAM, "a=x").get(), 1.0);
        assert_eq!(*QueryParam::new(&K_PARAM, "k=2.5.1").get(), 0.0);
        assert_eq!(*QueryParam::new(&H_PARAM, "h=Infinity").get(), 0.0);
    }

    #[test]
    fn transform_params_parse_js_numbers() {
        assert_eq!(parse_scale(Some("0x10")), Some(16.0));
        assert_eq!(parse_shift(Some("1e3")), Some(1000.0));
        assert_eq!(parse_shift(Some(" 2 ")), Some(2.0));
        // The empty text commits 0 for a shift and restores a scale.
        assert_eq!(parse_shift(Some("")), Some(0.0));
        assert_eq!(parse_scale(Some("")), None);
        assert_eq!(parse_shift(None), None);
    }

    #[test]
    fn transform_params_serialize_like_string_of_tofixed3() {
        assert_eq!(serialize_transform(&2.5), Some("2.5".to_owned()));
        assert_eq!(serialize_transform(&2.0), Some("2".to_owned()));
        assert_eq!(serialize_transform(&-1.0), Some("-1".to_owned()));
        assert_eq!(serialize_transform(&0.5), Some("0.5".to_owned()));
        // `Number(v.toFixed(3))`: three-decimal rounding, then shortest.
        assert_eq!(serialize_transform(&(0.1 + 0.2)), Some("0.3".to_owned()));
        // Ties round up, as `toFixed` does (Rust's `{:.3}` alone would not).
        assert_eq!(serialize_transform(&2.000_5), Some("2.001".to_owned()));
        // `-0.000` stringifies to `"0"`, as `String(-0)` does.
        assert_eq!(serialize_transform(&-0.000_1), Some("0".to_owned()));
    }

    #[test]
    fn transform_params_keep_the_identity_out_of_the_url() {
        let mut url = Url::from_query("");
        let mut a = QueryParam::new(&A_PARAM, "");
        a.set_flushed(&mut url, 1.0);
        assert_eq!(url.query(), "");
        a.set_flushed(&mut url, 2.5);
        assert_eq!(url.query(), "a=2.5");
        let mut h = QueryParam::new(&H_PARAM, "");
        h.set_flushed(&mut url, 0.0);
        assert_eq!(url.query(), "a=2.5");
    }
}
