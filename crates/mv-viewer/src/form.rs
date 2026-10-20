//! The base maths: the D1 transform, its base functions, and the anchor and
//! stretch handle maps.

use enum_map::{Enum, EnumMap};
use enumset::EnumSetType;
use mv_numfmt::quantize;

/// A base function g in y = a·g((x−h)/b)+k.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum Base {
    X,
    X2,
    X3,
    Sin,
}

/// A base function: its evaluator, names and unit input.
#[derive(Clone, Copy, Debug)]
pub struct BaseFunction {
    pub g: fn(f64) -> f64,
    pub label: &'static str,
    pub spoken: &'static str,
    pub unit_x: f64,
}

fn square(u: f64) -> f64 {
    u * u
}

fn cube(u: f64) -> f64 {
    u * u * u
}

/// The base functions, with their labels and unit inputs.
pub static BASE_FUNCTIONS: EnumMap<Base, BaseFunction> = EnumMap::from_array([
    BaseFunction {
        g: (|u: f64| u) as fn(f64) -> f64,
        label: "x",
        spoken: "x",
        unit_x: 1.0,
    },
    BaseFunction {
        g: square,
        label: "x²",
        spoken: "x squared",
        unit_x: 1.0,
    },
    BaseFunction {
        g: cube,
        label: "x³",
        spoken: "x cubed",
        unit_x: 1.0,
    },
    BaseFunction {
        g: f64::sin,
        label: "sin x",
        spoken: "sine of x",
        unit_x: std::f64::consts::FRAC_PI_2,
    },
]);

/// A transform parameter.
#[derive(Debug, Enum, EnumSetType)]
pub enum Param {
    A,
    K,
    B,
    H,
}

/// The four parameters, in order.
pub type Params = EnumMap<Param, f64>;

/// A parameter's letter.
pub static PARAM_LETTERS: EnumMap<Param, &str> = EnumMap::from_array(["a", "k", "b", "h"]);

/// The parameters at rest.
pub const DEFAULT_PARAMS: Params = EnumMap::from_array([1.0, 0.0, 1.0, 0.0]);

/// Whether a parameter sits at its default.
pub fn is_at_default(params: Params, p: Param) -> bool {
    params[p] == DEFAULT_PARAMS[p]
}

/// Whether the parameters show the base function unchanged.
pub fn is_identity(params: Params) -> bool {
    [Param::A, Param::K, Param::B, Param::H]
        .iter()
        .all(|&p| is_at_default(params, p))
}

/// Whether a parameter scales: a, b.
pub fn is_scale(p: Param) -> bool {
    matches!(p, Param::A | Param::B)
}

/// Whether a value is allowed: finite, and non-zero for scales.
pub fn accepts_value(p: Param, v: f64) -> bool {
    !v.is_nan() && !v.is_infinite() && !(is_scale(p) && v == 0.0)
}

/// What a refused zero scale reports.
pub const ZERO_SCALE: &str = "zero-scale";

/// The inner parameters in application order: h, then b.
pub const INNER_STEPS: [Param; 2] = [Param::H, Param::B];

/// The inner input g receives at x.
pub fn inner_input(params: Params, x: f64) -> f64 {
    let mut u = x;
    for step in INNER_STEPS {
        if step == Param::H {
            u -= params[Param::H];
        } else {
            u /= params[Param::B];
        }
    }
    u
}

/// The x reaching a unit input.
pub fn x_for_inner(params: Params, u: f64) -> f64 {
    let mut x = u;
    for &step in INNER_STEPS.iter().rev() {
        if step == Param::H {
            x += params[Param::H];
        } else {
            x *= params[Param::B];
        }
    }
    x
}

/// Evaluates the transform at x.
pub fn evaluate(g: fn(f64) -> f64, params: Params, x: f64) -> f64 {
    params[Param::A] * g(inner_input(params, x)) + params[Param::K]
}

/// The anchor point: the transform's fixed point at the origin.
pub fn anchor_point(params: Params) -> [f64; 2] {
    [x_for_inner(params, 0.0), params[Param::K]]
}

/// The unit point: where the unit input lands.
pub fn unit_point(base: &BaseFunction, params: Params) -> [f64; 2] {
    [
        x_for_inner(params, base.unit_x),
        params[Param::A] * (base.g)(base.unit_x) + params[Param::K],
    ]
}

/// Drags quantize to the display step.
pub const HANDLE_QUANTUM: f64 = mv_scrub::QUANTUM;

/// The bracketing values around a drag.
fn around(params: Params, step: Param, x: f64, u: f64) -> (f64, f64) {
    let mut at = 0;
    for (i, s) in INNER_STEPS.iter().enumerate() {
        if *s == step {
            at = i;
        }
    }
    let mut before = x;
    for s in &INNER_STEPS[..at] {
        if *s == Param::H {
            before -= params[Param::H];
        } else {
            before /= params[Param::B];
        }
    }
    let mut after = u;
    for s in INNER_STEPS[at + 1..].iter().rev() {
        if *s == Param::H {
            after += params[Param::H];
        } else {
            after *= params[Param::B];
        }
    }
    (before, after)
}

/// JavaScript's sign: -1, 0 or 1.
fn js_sign(v: f64) -> f64 {
    if v > 0.0 {
        1.0
    } else if v < 0.0 {
        -1.0
    } else {
        0.0
    }
}

/// Snaps a raw value onto the lattice from a previous one.
fn on_lattice(raw: f64, scale: bool, was: f64) -> f64 {
    let mut v = quantize(raw, HANDLE_QUANTUM);
    if v.is_nan() {
        v = 0.0;
    }
    if !scale || v != 0.0 {
        return v;
    }
    let mut side = js_sign(raw);
    if side == 0.0 {
        side = js_sign(was);
    }
    if side == 0.0 {
        side = 1.0;
    }
    side * HANDLE_QUANTUM
}

/// Drags the anchor to a point.
pub fn drag_anchor(params: Params, to: [f64; 2]) -> Params {
    let (before, after) = around(params, Param::H, to[0], 0.0);
    let mut next = params;
    next[Param::H] = on_lattice(before - after, false, params[Param::H]);
    next[Param::K] = on_lattice(to[1], false, params[Param::K]);
    next
}

/// Drags the stretch handle to a point.
pub fn drag_stretch(base: &BaseFunction, params: Params, to: [f64; 2]) -> Params {
    let (before, after) = around(params, Param::B, to[0], base.unit_x);
    let b = on_lattice(before / after, true, params[Param::B]);
    let a = on_lattice(
        (to[1] - params[Param::K]) / (base.g)(base.unit_x),
        true,
        params[Param::A],
    );
    if a.is_nan() || a.is_infinite() || b.is_nan() || b.is_infinite() {
        return params;
    }
    let mut next = params;
    next[Param::A] = a;
    next[Param::B] = b;
    next
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::equation::params_with;

    const R3: Params = EnumMap::from_array([2.0, 1.0, 1.0, -1.0]);
    const R8: Params = EnumMap::from_array([-2.0, 4.0, 1.0, -1.0]);

    fn close_to(a: f64, b: f64, digits: i64) -> bool {
        (a - b).abs() < 10f64.powi(-(digits as i32)) / 2.0
    }

    #[test]
    fn evaluate_is_d1() {
        let square = BASE_FUNCTIONS[Base::X2].g;
        assert_eq!(evaluate(square, R3, 0.5), 5.5);
        assert_eq!(evaluate(square, R8, 0.5), -0.5);
        for base in [Base::X, Base::X2, Base::X3, Base::Sin] {
            let g = BASE_FUNCTIONS[base].g;
            assert_eq!(evaluate(g, DEFAULT_PARAMS, 0.7), g(0.7));
        }
        fn old(x: f64) -> f64 {
            let v = x / 1.035 - 0.258;
            1.02 * v * v + 0.482
        }
        let now = params_with(1.02, 0.482, 1.035, 1.035 * 0.258);
        for x in [-3.0, -0.5, 0.0, 0.267, 1.0, 4.2] {
            assert!(close_to(evaluate(square, now, x), old(x), 12), "x = {x}");
        }
        assert!(close_to(anchor_point(now)[0], 0.267, 3));
    }

    #[test]
    fn the_anchor_and_the_unit_point() {
        let p = params_with(3.0, -2.0, 0.5, 1.5);
        assert_eq!(anchor_point(p), [1.5, -2.0]);
        for base in [Base::X, Base::X2, Base::X3, Base::Sin] {
            assert!(
                close_to(evaluate(BASE_FUNCTIONS[base].g, p, 1.5), -2.0, 12),
                "{base:?}"
            );
        }
        assert_eq!(unit_point(&BASE_FUNCTIONS[Base::X2], R3), [0.0, 3.0]);
        let sin_unit = unit_point(&BASE_FUNCTIONS[Base::Sin], params_with(2.0, 1.0, 2.0, 1.0));
        assert!(
            close_to(sin_unit[0], 1.0 + std::f64::consts::PI, 12) && close_to(sin_unit[1], 3.0, 12)
        );
        let q = params_with(-1.5, 0.5, 0.75, -2.0);
        for base in [Base::X, Base::X2, Base::X3, Base::Sin] {
            let f = BASE_FUNCTIONS[base];
            let unit = unit_point(&f, q);
            assert!(close_to(evaluate(f.g, q, unit[0]), unit[1], 12), "{base:?}");
        }
        let r = params_with(1.0, 0.0, -0.5, 2.0);
        assert_eq!(inner_input(r, 3.0), -2.0);
        assert_eq!(x_for_inner(r, inner_input(r, 3.0)), 3.0);
    }

    #[test]
    fn defaults_and_values() {
        assert!(is_at_default(R3, Param::B));
        assert!(!is_at_default(R3, Param::A));
        assert!(is_at_default(params_with(1.0, -0.0, 1.0, 0.0), Param::K));
        assert!(is_identity(DEFAULT_PARAMS));
        assert!(!is_identity(params_with(1.0, 0.0, 1.0, -1.0)));
        assert!(accepts_value(Param::H, 0.0));
        assert!(accepts_value(Param::K, -12.0));
        assert!(accepts_value(Param::A, -2.0));
        assert!(!accepts_value(Param::A, 0.0));
        assert!(!accepts_value(Param::B, 0.0));
        assert!(!accepts_value(Param::K, f64::NAN));
    }

    #[test]
    fn the_handles_inverse_maps() {
        assert_eq!(drag_anchor(R3, [1.5, 1.0]), params_with(2.0, 1.0, 1.0, 1.5));
        let moved = drag_anchor(R3, [0.3333, -2.004]);
        assert_eq!(anchor_point(moved), [0.33, -2.0]);

        let x2 = BASE_FUNCTIONS[Base::X2];
        let next = drag_stretch(&x2, R3, [0.5, 3.0]);
        assert_eq!(next, params_with(2.0, 1.0, 1.5, -1.0));
        assert_eq!(unit_point(&x2, next), [0.5, 3.0]);

        let x3 = BASE_FUNCTIONS[Base::X3];
        let flipped = drag_stretch(&x3, R3, [-2.5, -0.5]);
        assert_eq!(flipped[Param::A], -1.5);
        assert_eq!(flipped[Param::B], -1.5);
        assert!(close_to(evaluate(x3.g, flipped, -2.5), -0.5, 9));

        assert_eq!(drag_stretch(&x2, R3, [0.0, 1.002])[Param::A], 0.01);
        assert_eq!(
            drag_stretch(&x2, params_with(-2.0, 1.0, 1.0, -1.0), [0.0, 1.0])[Param::A],
            -0.01
        );
        assert_eq!(drag_stretch(&x2, R3, [-1.001, 3.0])[Param::B], -0.01);

        assert_eq!(
            drag_stretch(
                &BASE_FUNCTIONS[Base::Sin],
                DEFAULT_PARAMS,
                [std::f64::consts::PI, 2.0]
            ),
            params_with(2.0, 0.0, 2.0, 0.0)
        );
    }
}
