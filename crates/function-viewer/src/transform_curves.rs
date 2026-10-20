//! The transform applied to the plane's curves, and the shift and scale
//! handles and the lit-parameter lines that go with it.
//!
//! Ported from math-viz2 at `5cef052`:
//!
//! - `src/components/views/FunctionViewer/transformCurves.ts`:
//!   [`Transform`], [`transform_curves`], [`transform_handles`],
//!   [`transform_lines`].
//! - `src/components/views/FunctionViewer/baseFunctions.ts` (`unitX`):
//!   the `u` argument stays a plain `f64` the caller passes;
//!   `crate::functions::unit_x` is the value the caller reads.
//!
//! The params type is `crate::state::TransformParam` (h4's `state.rs`).

use crate::functions::{FunctionSlug, base};
use crate::plot::{HandleKind, LineKind, PlotCurve, PlotFunction, PlotHandle, PlotLine, PlotPoint};
use crate::state::TransformParam;

/// The four transform values (`Transform` in `transformCurves.ts`): a
/// struct, not a map, since the keys are fixed and the tests write them as
/// literals.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub a: f64,
    pub b: f64,
    pub h: f64,
    pub k: f64,
}

impl Transform {
    /// The untouched transform (`{ a: 1, b: 1, h: 0, k: 0 }`), which passes
    /// curves through.
    pub const IDENTITY: Self = Self {
        a: 1.0,
        b: 1.0,
        h: 0.0,
        k: 0.0,
    };

    /// The parameter's value (`transform[p]`).
    #[must_use]
    pub fn get(&self, param: TransformParam) -> f64 {
        match param {
            TransformParam::A => self.a,
            TransformParam::B => self.b,
            TransformParam::H => self.h,
            TransformParam::K => self.k,
        }
    }

    /// A copy with the parameter set (`{ ...transform, [p]: value }`).
    #[must_use]
    pub fn with(&self, param: TransformParam, value: f64) -> Self {
        let mut out = *self;
        match param {
            TransformParam::A => out.a = value,
            TransformParam::B => out.b = value,
            TransformParam::H => out.h = value,
            TransformParam::K => out.k = value,
        }
        out
    }

    /// Whether the transform leaves every curve alone (`isIdentity`).
    #[must_use]
    pub fn is_identity(&self) -> bool {
        self.a == 1.0 && self.b == 1.0 && self.h == 0.0 && self.k == 0.0
    }
}

/// A base function under a transform: `a * g((x - h) / b) + k`
/// (`transformed` in `transformCurves.ts`). Data, not a closure: a
/// transformed curve cannot be a `fn` pointer, and closures never cross
/// module boundaries.
#[derive(Clone, Copy, Debug)]
pub struct TransformedFunction {
    pub base: PlotFunction,
    pub transform: Transform,
}

impl TransformedFunction {
    /// The transformed value at `x`.
    #[must_use]
    pub fn value(&self, x: f64) -> f64 {
        let t = &self.transform;
        t.a * (self.base)((x - t.h) / t.b) + t.k
    }
}

/// A curve as the plane shows it under a transform: what to draw, how far,
/// and whether it is the dashed original behind the transformed curve (the
/// `PlotCurve` with `dashed` that `transformCurves` returns; the flag rides
/// here because `plot::PlotCurve` carries a `fn` pointer, which a
/// transformed function cannot be).
#[derive(Clone, Copy, Debug)]
pub struct TransformedCurve {
    pub func: TransformedFunction,
    pub alpha: f64,
    pub draw_to: f64,
    pub dashed: bool,
}

/// The curves the plane graphs under a transform (`transformCurves`): with
/// no values set the curves pass through; otherwise each curve draws twice,
/// the untransformed original dashed behind the transformed one. Opacity
/// and draw-on share carry over untouched.
#[must_use]
pub fn transform_curves(curves: &[PlotCurve], transform: Transform) -> Vec<TransformedCurve> {
    curves
        .iter()
        .flat_map(|curve| {
            let drawn = TransformedCurve {
                func: TransformedFunction {
                    base: curve.func,
                    transform,
                },
                alpha: curve.alpha,
                draw_to: curve.draw_to,
                dashed: false,
            };
            if transform.is_identity() {
                vec![drawn]
            } else {
                // The original keeps its base function, untransformed, and
                // draws dashed behind (`{ ...curve, dashed: true }`).
                let original = TransformedCurve {
                    func: TransformedFunction {
                        base: curve.func,
                        transform: Transform::IDENTITY,
                    },
                    alpha: curve.alpha,
                    draw_to: curve.draw_to,
                    dashed: true,
                };
                vec![original, drawn]
            }
        })
        .collect()
}

/// The plane x where a base function shows its scale handle: 1 for the
/// monomials, π/2 (the peak) for `sin x`. Used here through the `u`
/// argument the caller passes.
fn unit_point(g: PlotFunction, u: f64, transform: Transform) -> PlotPoint {
    PlotPoint {
        x: transform.h + transform.b * u,
        y: transform.a * g(u) + transform.k,
    }
}

/// The shift handle (a diamond on the vertex) and the scale handle (a
/// square on the curve at `u`) for one function (`transformHandles`).
#[must_use]
pub fn transform_handles(slug: FunctionSlug, u: f64, transform: Transform) -> Vec<PlotHandle> {
    vec![
        PlotHandle {
            kind: HandleKind::Diamond,
            at: PlotPoint {
                x: transform.h,
                y: transform.k,
            },
        },
        PlotHandle {
            kind: HandleKind::Square,
            at: unit_point(base(slug), u, transform),
        },
    ]
}

/// The dashed box joining the shift handle to the unit point (`dashedBox`).
fn dashed_box(from: PlotPoint, to: PlotPoint) -> Vec<PlotLine> {
    let corners = [
        from,
        PlotPoint { x: to.x, y: from.y },
        to,
        PlotPoint { x: from.x, y: to.y },
    ];
    corners
        .iter()
        .enumerate()
        .map(|(i, corner)| PlotLine {
            kind: LineKind::Dashed,
            from: *corner,
            to: corners[(i + 1) % corners.len()],
            label: None,
        })
        .collect()
}

/// The lines while a parameter is lit (`transformLines`): an arrow from the
/// axis for a shift, a dashed box with one measured side for a scale,
/// nothing while nothing is lit. `label` stays `None` (the TypeScript
/// carries no label); h6-handles decides what, if anything, fills it.
#[must_use]
pub fn transform_lines(
    lit: Option<TransformParam>,
    slug: FunctionSlug,
    u: f64,
    transform: Transform,
) -> Vec<PlotLine> {
    let shift = PlotPoint {
        x: transform.h,
        y: transform.k,
    };
    let unit = unit_point(base(slug), u, transform);
    let corner = PlotPoint {
        x: unit.x,
        y: shift.y,
    };
    let arrow = |from: PlotPoint| PlotLine {
        kind: LineKind::Arrow,
        from,
        to: shift,
        label: None,
    };
    let measured = |from: PlotPoint, to: PlotPoint| PlotLine {
        kind: LineKind::Measured,
        from,
        to,
        label: None,
    };
    match lit {
        None => Vec::new(),
        Some(TransformParam::K) => vec![arrow(PlotPoint { x: shift.x, y: 0.0 })],
        Some(TransformParam::H) => vec![arrow(PlotPoint { x: 0.0, y: shift.y })],
        Some(TransformParam::A) => {
            let mut lines = dashed_box(shift, unit);
            lines.push(measured(corner, unit));
            lines
        }
        Some(TransformParam::B) => {
            let mut lines = dashed_box(shift, unit);
            lines.push(measured(shift, corner));
            lines
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::functions::unit_x;

    fn square() -> PlotCurve {
        PlotCurve {
            func: base(FunctionSlug::X2),
            alpha: 1.0,
            draw_to: 1.0,
        }
    }

    fn untouched() -> Transform {
        Transform::IDENTITY
    }

    #[test]
    fn with_no_values_set_the_plane_shows_y_equals_x2_alone() {
        let curves = transform_curves(&[square()], untouched());

        assert_eq!(curves.len(), 1);
        assert!(!curves[0].dashed);
        assert_eq!(curves[0].func.value(2.0), 4.0);
    }

    #[test]
    fn scaled_and_shifted_x2_has_its_vertex_at_1_3_with_the_original_dashed_behind_it() {
        let curves = transform_curves(
            &[square()],
            Transform {
                a: 2.0,
                b: 1.0,
                h: 1.0,
                k: 3.0,
            },
        );

        assert_eq!(curves.len(), 2);
        assert!(curves[0].dashed);
        assert_eq!(curves[0].func.value(1.0), 1.0);
        assert!(!curves[1].dashed);
        assert_eq!(curves[1].func.value(1.0), 3.0);
        assert_eq!(curves[1].func.value(2.0), 5.0);
    }

    #[test]
    fn b_2_makes_y_equals_x2_twice_as_wide_so_it_passes_through_2_1() {
        let curves = transform_curves(
            &[square()],
            Transform {
                b: 2.0,
                ..untouched()
            },
        );

        assert_eq!(curves[1].func.value(2.0), 1.0);
    }

    #[test]
    fn while_x2_replaces_y_equals_x_each_fades_or_draws_on_with_its_dashed_original() {
        let line = PlotCurve {
            func: base(FunctionSlug::X1),
            alpha: 0.5,
            draw_to: 1.0,
        };
        let drawing = PlotCurve {
            func: base(FunctionSlug::X2),
            alpha: 1.0,
            draw_to: 0.25,
        };

        let curves = transform_curves(
            &[line, drawing],
            Transform {
                k: 1.0,
                ..untouched()
            },
        );

        // The mapped `{ alpha, drawTo, dashed }` triples, in order.
        let triples: Vec<(f64, f64, bool)> = curves
            .iter()
            .map(|curve| (curve.alpha, curve.draw_to, curve.dashed))
            .collect();
        assert_eq!(
            triples,
            vec![
                (0.5, 1.0, true),
                (0.5, 1.0, false),
                (1.0, 0.25, true),
                (1.0, 0.25, false),
            ]
        );
        assert_eq!(curves[1].func.value(2.0), 3.0);
        assert_eq!(curves[3].func.value(2.0), 5.0);
    }

    #[test]
    fn with_no_values_set_shift_handle_at_origin_and_scale_handle_at_1_1() {
        assert_eq!(
            transform_handles(FunctionSlug::X2, unit_x(FunctionSlug::X2), untouched()),
            vec![
                PlotHandle {
                    kind: HandleKind::Diamond,
                    at: PlotPoint { x: 0.0, y: 0.0 },
                },
                PlotHandle {
                    kind: HandleKind::Square,
                    at: PlotPoint { x: 1.0, y: 1.0 },
                },
            ]
        );
    }

    #[test]
    fn scaled_and_shifted_curve_has_its_handles_on_the_vertex_and_on_the_curve() {
        assert_eq!(
            transform_handles(
                FunctionSlug::X2,
                unit_x(FunctionSlug::X2),
                Transform {
                    a: 2.0,
                    b: 1.0,
                    h: 1.0,
                    k: 3.0,
                },
            ),
            vec![
                PlotHandle {
                    kind: HandleKind::Diamond,
                    at: PlotPoint { x: 1.0, y: 3.0 },
                },
                PlotHandle {
                    kind: HandleKind::Square,
                    at: PlotPoint { x: 2.0, y: 5.0 },
                },
            ]
        );
    }

    #[test]
    fn sinx_scale_handle_sits_on_its_peak_which_b_2_moves_to_pi() {
        let handles =
            transform_handles(FunctionSlug::SinX, unit_x(FunctionSlug::SinX), untouched());
        assert_eq!(
            handles[1],
            PlotHandle {
                kind: HandleKind::Square,
                at: PlotPoint {
                    x: std::f64::consts::FRAC_PI_2,
                    y: 1.0,
                },
            }
        );

        let wide = transform_handles(
            FunctionSlug::SinX,
            unit_x(FunctionSlug::SinX),
            Transform {
                b: 2.0,
                ..untouched()
            },
        );
        assert_eq!(
            wide[1],
            PlotHandle {
                kind: HandleKind::Square,
                at: PlotPoint {
                    x: std::f64::consts::PI,
                    y: 1.0,
                },
            }
        );
    }

    fn lines_on_square(lit: Option<TransformParam>) -> Vec<PlotLine> {
        transform_lines(
            lit,
            FunctionSlug::X2,
            unit_x(FunctionSlug::X2),
            Transform {
                a: 2.0,
                b: 1.0,
                h: 1.0,
                k: 3.0,
            },
        )
    }

    fn dashed(from: (f64, f64), to: (f64, f64)) -> PlotLine {
        PlotLine {
            kind: LineKind::Dashed,
            from: PlotPoint {
                x: from.0,
                y: from.1,
            },
            to: PlotPoint { x: to.0, y: to.1 },
            label: None,
        }
    }

    fn line_box() -> Vec<PlotLine> {
        vec![
            dashed((1.0, 3.0), (2.0, 3.0)),
            dashed((2.0, 3.0), (2.0, 5.0)),
            dashed((2.0, 5.0), (1.0, 5.0)),
            dashed((1.0, 5.0), (1.0, 3.0)),
        ]
    }

    #[test]
    fn while_k_is_lit_an_arrow_rises_from_the_x_axis_to_the_shift_handle() {
        assert_eq!(
            lines_on_square(Some(TransformParam::K)),
            vec![PlotLine {
                kind: LineKind::Arrow,
                from: PlotPoint { x: 1.0, y: 0.0 },
                to: PlotPoint { x: 1.0, y: 3.0 },
                label: None,
            }]
        );
    }

    #[test]
    fn while_h_is_lit_an_arrow_runs_from_the_y_axis_to_the_shift_handle() {
        assert_eq!(
            lines_on_square(Some(TransformParam::H)),
            vec![PlotLine {
                kind: LineKind::Arrow,
                from: PlotPoint { x: 0.0, y: 3.0 },
                to: PlotPoint { x: 1.0, y: 3.0 },
                label: None,
            }]
        );
    }

    #[test]
    fn while_a_is_lit_a_dashed_box_joins_the_handles_and_its_right_side_measures_a() {
        let mut expected = line_box();
        expected.push(PlotLine {
            kind: LineKind::Measured,
            from: PlotPoint { x: 2.0, y: 3.0 },
            to: PlotPoint { x: 2.0, y: 5.0 },
            label: None,
        });
        assert_eq!(lines_on_square(Some(TransformParam::A)), expected);
    }

    #[test]
    fn while_b_is_lit_a_dashed_box_joins_the_handles_and_its_bottom_side_measures_b() {
        let mut expected = line_box();
        expected.push(PlotLine {
            kind: LineKind::Measured,
            from: PlotPoint { x: 1.0, y: 3.0 },
            to: PlotPoint { x: 2.0, y: 3.0 },
            label: None,
        });
        assert_eq!(lines_on_square(Some(TransformParam::B)), expected);
    }

    #[test]
    fn while_nothing_is_lit_no_line_shows() {
        assert_eq!(lines_on_square(None), Vec::new());
    }
}
