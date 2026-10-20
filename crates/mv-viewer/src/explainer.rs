//! The explainer: where its callout sits, and the example plot it shows.

use enum_map::EnumMap;

use super::form::{
    BASE_FUNCTIONS, Base, DEFAULT_PARAMS, Param, Params, anchor_point, is_scale, unit_point,
};
use super::plane_scene::curve_fn;
use mv_plane::{Curve, GridSteps, Ink, MiniArrow, MiniDot, MiniPlot, MiniRing, Rect};

/// The rest before an explainer opens, in ms.
pub const EXPLAINER_REST_MS: f64 = 400.0;

/// An explainer's width, in px.
pub const EXPLAINER_WIDTH: f64 = 300.0;

/// An explainer caret's reach, in px.
pub const EXPLAINER_CARET: f64 = 9.0;

/// An explainer tip's length, in px.
pub const EXPLAINER_TIP: f64 = 20.0;

/// An explainer's gap, in px.
pub const EXPLAINER_GAP: f64 = 4.0;

/// An explainer's margin, in px.
pub const EXPLAINER_MARGIN: f64 = 16.0;

/// Which side an explainer's caret leaves from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaretSide {
    Left,
    Top,
    Bottom,
}

/// An explainer's caret: its side and position along it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Caret {
    pub side: CaretSide,
    pub at: f64,
}

/// Where an explainer sits: its point and caret.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExplainerPlace {
    pub x: f64,
    pub y: f64,
    pub caret: Caret,
}

/// Clamps a value into a range, the low bound winning ties.
fn js_clamp(v: f64, lo: f64, hi: f64) -> f64 {
    lo.max(v.min(hi))
}

/// Places an explainer for a chip and panel at a height in a view.
pub fn place_explainer(chip: Rect, panel: Rect, height: f64, view: [f64; 2]) -> ExplainerPlace {
    let cx = chip.x + chip.w / 2.0;
    let cy = chip.y + chip.h / 2.0;
    let x = panel.x + panel.w + EXPLAINER_GAP + EXPLAINER_CARET;
    if x + EXPLAINER_WIDTH + EXPLAINER_MARGIN <= view[0] {
        let y = js_clamp(
            cy - EXPLAINER_TIP,
            EXPLAINER_MARGIN,
            view[1] - EXPLAINER_MARGIN - height,
        );
        return ExplainerPlace {
            x,
            y,
            caret: Caret {
                side: CaretSide::Left,
                at: js_clamp(cy - y, EXPLAINER_TIP, height - EXPLAINER_TIP),
            },
        };
    }
    let below = chip.y + chip.h + EXPLAINER_GAP + EXPLAINER_CARET;
    let above = chip.y - EXPLAINER_GAP - EXPLAINER_CARET - height;
    let room_below = view[1] - EXPLAINER_MARGIN - (below + height);
    let room_above = above - EXPLAINER_MARGIN;
    let under = room_below >= 0.0 || room_below >= room_above;
    let outward = if !under && cx < view[0] / 2.0 {
        cx + EXPLAINER_TIP - EXPLAINER_WIDTH
    } else {
        cx - EXPLAINER_TIP
    };
    let left = js_clamp(
        outward,
        EXPLAINER_MARGIN,
        view[0] - EXPLAINER_MARGIN - EXPLAINER_WIDTH,
    );
    let at = js_clamp(cx - left, EXPLAINER_TIP, EXPLAINER_WIDTH - EXPLAINER_TIP);
    if under {
        ExplainerPlace {
            x: left,
            y: below,
            caret: Caret {
                side: CaretSide::Top,
                at,
            },
        }
    } else {
        ExplainerPlace {
            x: left,
            y: above,
            caret: Caret {
                side: CaretSide::Bottom,
                at,
            },
        }
    }
}

/// The explainer plots' example function.
pub const EXPLAINER_EXAMPLE_FN: Base = Base::X2;

/// The explainer plots' example value.
pub const EXPLAINER_EXAMPLE_VALUE: f64 = 2.0;

/// An explainer plot's size: width and height.
pub const EXPLAINER_PLOT_SIZE: [f64; 2] = [268.0, 124.0];

/// An explainer plot's zoom.
const PLOT_ZOOM: f64 = 22.0;

/// An explainer plot's grid steps.
const PLOT_STEPS: GridSteps = GridSteps {
    minor: 1.0,
    major: 5.0,
};

/// An explainer plot's axis width.
const PLOT_AXIS_WIDTH: f64 = 1.5;

/// An explainer plot's origin, by parameter.
static PLOT_ORIGINS: EnumMap<Param, [f64; 2]> = EnumMap::from_array([
    [134.0, 112.0],
    [134.0, 104.0],
    [134.0, 112.0],
    [80.0, 112.0],
]);

/// What was there before, in px wide.
const BEFORE_WIDTH: f64 = 1.75;

/// What was there before, dashed.
const BEFORE_DASH: [f64; 2] = [5.0, 4.0];

/// What was there before, at this alpha.
const BEFORE_ALPHA: f64 = 0.8;

/// What is there now, in px wide.
const AFTER_WIDTH: f64 = 2.5;

/// A parameter's example parameters: before and after.
pub fn explainer_params(param: Param) -> Params {
    let mut params = DEFAULT_PARAMS;
    params[param] = EXPLAINER_EXAMPLE_VALUE;
    params
}

/// A parameter's carried points: from and to.
pub fn explainer_carried(param: Param) -> ([f64; 2], [f64; 2]) {
    let base = &BASE_FUNCTIONS[EXPLAINER_EXAMPLE_FN];
    let params = explainer_params(param);
    if is_scale(param) {
        return (unit_point(base, DEFAULT_PARAMS), unit_point(base, params));
    }
    (anchor_point(DEFAULT_PARAMS), anchor_point(params))
}

/// A parameter's example plot.
pub fn explainer_plot(param: Param) -> MiniPlot {
    let base = &BASE_FUNCTIONS[EXPLAINER_EXAMPLE_FN];
    let (from, to) = explainer_carried(param);
    MiniPlot {
        zoom: PLOT_ZOOM,
        origin: PLOT_ORIGINS[param],
        steps: PLOT_STEPS,
        axis_width: PLOT_AXIS_WIDTH,
        curves: vec![
            Curve {
                id: "before".to_string(),
                func: curve_fn(base.g, DEFAULT_PARAMS),
                ink: Ink::ForegroundMuted,
                width: BEFORE_WIDTH,
                dash: Some(BEFORE_DASH),
                alpha: Some(BEFORE_ALPHA),
                avoid: false,
                back: false,
                draw_to: None,
                glow: false,
            },
            Curve {
                id: "after".to_string(),
                func: curve_fn(base.g, explainer_params(param)),
                ink: Ink::ChartLine,
                width: AFTER_WIDTH,
                dash: None,
                alpha: None,
                avoid: false,
                back: false,
                draw_to: None,
                glow: false,
            },
        ],
        arrows: vec![MiniArrow {
            from,
            to,
            ink: Ink::Primary,
            width: 2.0,
        }],
        dots: vec![
            MiniDot {
                at: from,
                diameter: 7.0,
                fill: Ink::Background,
                ring: Some(MiniRing {
                    ink: Ink::ForegroundMuted,
                    width: 1.5,
                }),
            },
            MiniDot {
                at: to,
                diameter: 8.0,
                fill: Ink::Primary,
                ring: None,
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::copy::{
        EXPLAINER_SNIPPETS, PARAM_NAMES, explainer_gestures, phrase_words, where_of,
    };
    use crate::equation::{TermToken, TextToken, Token, snippet_tokens};
    use mv_plane::curve_at;

    const PANEL: Rect = Rect {
        x: 16.0,
        y: 92.0,
        w: 460.0,
        h: 792.0,
    };
    const VIEW: [f64; 2] = [1440.0, 900.0];

    const STACKED: Rect = Rect {
        x: 16.0,
        y: 92.0,
        w: 768.0,
        h: 900.0,
    };
    const TALL: [f64; 2] = [800.0, 900.0];

    fn chip(x: f64, y: f64) -> Rect {
        Rect {
            x,
            y,
            w: 26.0,
            h: 26.0,
        }
    }

    #[test]
    fn opens_right_of_the_panel_on_the_chip() {
        assert_eq!(
            place_explainer(chip(35.0, 390.5), PANEL, 389.0, VIEW),
            ExplainerPlace {
                x: 489.0,
                y: 383.5,
                caret: Caret {
                    side: CaretSide::Left,
                    at: 20.0,
                },
            }
        );
    }

    #[test]
    fn stays_16_px_inside_the_window() {
        let place = place_explainer(chip(255.0, 600.0), PANEL, 389.0, [1440.0, 720.0]);
        assert_eq!(place.y, 720.0 - 16.0 - 389.0);
        assert_eq!(
            place.caret,
            Caret {
                side: CaretSide::Left,
                at: 613.0 - place.y,
            }
        );
    }

    #[test]
    fn hangs_under_the_chip_when_stacked() {
        assert_eq!(
            place_explainer(chip(35.0, 400.0), STACKED, 389.0, TALL),
            ExplainerPlace {
                x: 28.0,
                y: 439.0,
                caret: Caret {
                    side: CaretSide::Top,
                    at: 20.0,
                },
            }
        );
    }

    #[test]
    fn stands_above_a_low_chip_reaching_away_from_the_middle() {
        assert_eq!(
            place_explainer(chip(35.0, 500.0), STACKED, 389.0, TALL),
            ExplainerPlace {
                x: 16.0,
                y: 98.0,
                caret: Caret {
                    side: CaretSide::Bottom,
                    at: 32.0,
                },
            }
        );
        assert_eq!(
            place_explainer(chip(409.0, 500.0), STACKED, 389.0, TALL),
            ExplainerPlace {
                x: 402.0,
                y: 98.0,
                caret: Caret {
                    side: CaretSide::Bottom,
                    at: 20.0,
                },
            }
        );
    }

    #[test]
    fn takes_the_roomier_side_where_neither_fits() {
        let place = place_explainer(
            chip(35.0, 300.0),
            Rect {
                x: 16.0,
                y: 92.0,
                w: 768.0,
                h: 600.0,
            },
            389.0,
            [800.0, 640.0],
        );
        assert_eq!(place.caret.side, CaretSide::Top);
    }

    #[test]
    fn carries_the_unit_point_or_the_anchor() {
        let want: [[[f64; 2]; 2]; 4] = [
            [[1.0, 1.0], [1.0, 2.0]],
            [[0.0, 0.0], [0.0, 2.0]],
            [[1.0, 1.0], [2.0, 1.0]],
            [[0.0, 0.0], [2.0, 0.0]],
        ];
        for p in [Param::A, Param::K, Param::B, Param::H] {
            let plot = explainer_plot(p);
            assert_eq!(plot.arrows.len(), 1);
            assert_eq!(
                [plot.arrows[0].from, plot.arrows[0].to],
                want[p as usize],
                "{p:?}"
            );
        }
    }

    #[test]
    fn draws_the_original_dashed_and_the_example_solid() {
        let h = explainer_plot(Param::H);
        let (before, after) = (&h.curves[0], &h.curves[1]);
        assert_eq!(before.ink, Ink::ForegroundMuted);
        assert_eq!(before.width, 1.75);
        assert_eq!(before.dash, Some([5.0, 4.0]));
        assert_eq!(before.alpha.unwrap_or(1.0), 0.8);
        assert_eq!(curve_at(&before.func, 3.0), 9.0);
        assert_eq!(after.ink, Ink::ChartLine);
        assert_eq!(after.width, 2.5);
        assert_eq!(curve_at(&after.func, 2.0), 0.0);
        assert_eq!(curve_at(&explainer_plot(Param::A).curves[1].func, 1.0), 2.0);
        assert_eq!(curve_at(&explainer_plot(Param::B).curves[1].func, 2.0), 1.0);
        assert_eq!(curve_at(&explainer_plot(Param::K).curves[1].func, 0.0), 2.0);
    }

    #[test]
    fn says_what_each_transform_does() {
        assert_eq!(PARAM_NAMES[Param::A], "Vertical scale");
        assert!(crate::copy::EXPLAINER_SAY[Param::A].starts_with("Multiplies every height by a."));
        assert_eq!(
            crate::copy::EXPLAINER_SAY[Param::K],
            "Adds k to every height, so the whole curve moves up by k (down when k is negative)."
        );
        assert_eq!(PARAM_NAMES[Param::H], "Horizontal shift");
        assert_eq!(where_of(Param::A), "outside");
        assert_eq!(where_of(Param::K), "outside");
        assert_eq!(where_of(Param::B), "inside");
        assert_eq!(where_of(Param::H), "inside");
        let words = phrase_words(&explainer_gestures(Param::H)).join(" ");
        assert_eq!(
            words,
            "Drag the number · Shift fine · Ctrl whole steps · double-click: back to 0"
        );
        let words = phrase_words(&explainer_gestures(Param::A)).join(" ");
        assert_eq!(
            words,
            "Drag the number · Shift fine · Ctrl quarter steps · double-click: back to 1"
        );
    }

    fn same_token(a: &Token, b: &Token) -> bool {
        match (a, b) {
            (Token::Text(x), Token::Text(y)) => x == y,
            (Token::Term(x), Token::Term(y)) => x == y,
            _ => false,
        }
    }

    #[test]
    fn the_snippet_lights_its_letter() {
        let wants: [(Param, Vec<Token>); 4] = [
            (
                Param::A,
                vec![
                    Token::Term(TermToken {
                        param: Param::A,
                        text: "a".to_string(),
                        ghost: false,
                    }),
                    Token::Text(TextToken {
                        text: " · f(x)".to_string(),
                    }),
                ],
            ),
            (
                Param::K,
                vec![
                    Token::Text(TextToken {
                        text: "f(x) + ".to_string(),
                    }),
                    Token::Term(TermToken {
                        param: Param::K,
                        text: "k".to_string(),
                        ghost: false,
                    }),
                ],
            ),
            (
                Param::B,
                vec![
                    Token::Text(TextToken {
                        text: "f(x / ".to_string(),
                    }),
                    Token::Term(TermToken {
                        param: Param::B,
                        text: "b".to_string(),
                        ghost: false,
                    }),
                    Token::Text(TextToken {
                        text: ")".to_string(),
                    }),
                ],
            ),
            (
                Param::H,
                vec![
                    Token::Text(TextToken {
                        text: "f(x − ".to_string(),
                    }),
                    Token::Term(TermToken {
                        param: Param::H,
                        text: "h".to_string(),
                        ghost: false,
                    }),
                    Token::Text(TextToken {
                        text: ")".to_string(),
                    }),
                ],
            ),
        ];
        for (param, tokens) in &wants {
            let got = snippet_tokens(*param, EXPLAINER_SNIPPETS[*param]);
            if got.len() != tokens.len() {
                panic!("{param:?}: got {} tokens, want {}", got.len(), tokens.len());
            }
            for (i, token) in got.iter().enumerate() {
                assert!(
                    same_token(token, &tokens[i]),
                    "{param:?}: {token:?}, not {:?}",
                    tokens[i]
                );
            }
        }
    }
}
