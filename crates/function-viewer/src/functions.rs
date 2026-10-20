//! The sidebar function picker's options and the `func` page parameter.
//!
//! Ported from math-viz2 at `3560eb2`:
//!
//! - `src/components/views/FunctionViewer/baseFunctions.ts`: [`base`], the
//!   function each picker option graphs.
//! - `src/components/views/FunctionViewer/queryParams.ts`: [`FunctionSlug`],
//!   [`FUNCTION_SLUGS`], [`FUNC_PARAM`] (key `func`, default `x1`).

use crate::plot::PlotFunction;
use crate::url::{DEFAULT_DEBOUNCE_MS, QueryParamDef, default_equals};

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
}
