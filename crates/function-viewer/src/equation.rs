//! The equation card: the live and form lines, fraction bars, term plates,
//! and the term tips.
//!
//! Tokens become runs, bars and terms; the live line fit-scales to the card's
//! width.

use std::f32::consts::PI;

use enum_map::Enum;
use mv_sys::clay;

use crate::app::App;
use crate::input::{Cursor, Hit, HitKind};

/// The STIX x-height fraction the fraction bars centre on.
pub const STIX_X_HEIGHT: f32 = 0.473;
/// The live and form sizes.
pub const LIVE_SIZE: f32 = 28.0;
pub const FORM_SIZE: f32 = 18.0;
/// The live row's minimum height.
pub const LIVE_MIN_HEIGHT: f32 = 60.0;
/// Gaps and padding around the lines.
pub const EQUATION_GAP: f32 = 4.0;
pub const EQUATION_PAD: f32 = 4.0;

/// One shaped run in a line.
#[derive(Clone, Debug)]
pub struct EqRun {
    pub shaped: mv_text::Shaped,
    pub size: f32,
    pub x: f32,
    pub baseline: f32,
    pub ink: mv_theme::Color,
}

/// One fraction bar.
#[derive(Clone, Copy, Debug)]
pub struct EqBar {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub ink: mv_theme::Color,
}

/// One hoverable term.
#[derive(Clone, Copy, Debug)]
pub struct EqTerm {
    pub param: mv_viewer::Param,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// One laid-out line.
#[derive(Clone, Debug)]
pub struct EqLine {
    pub runs: Vec<EqRun>,
    pub bars: Vec<EqBar>,
    pub terms: Vec<EqTerm>,
    pub width: f32,
    pub height: f32,
    pub baseline: f32,
}

/// Both lines and what is lit.
#[derive(Clone, Debug)]
pub struct Equation {
    pub live: EqLine,
    pub form: EqLine,
    /// The live row's height: at least 60.
    pub live_height: f32,
    pub lit: enumset::EnumSet<mv_viewer::Param>,
}

/// A laid-out extent: width and how far it reaches above and below.
struct Extent {
    width: f32,
    top: f32,
    bottom: f32,
}

/// The line builder's working state.
struct Setter<'a> {
    fonts: &'a mut mv_text::Fonts,
    t: &'a mv_theme::Theme,
    line_ink: mv_theme::Color,
    style: LineStyle,
    out: &'a mut EqLine,
}

/// How terms draw in a line.
#[derive(Clone, Copy, Debug)]
pub struct LineStyle {
    pub term_ink: Option<mv_theme::Color>,
    pub term_bottom: f32,
    pub whole_px: bool,
}

/// The equation line style: term plates with whole-px type.
pub const EQUATION_LINE: LineStyle = LineStyle {
    term_ink: None,
    term_bottom: TERM_PAD[1],
    whole_px: true,
};

/// The term plate's padding.
pub const TERM_PAD: [f32; 2] = [3.0, 1.0];

/// The token fitter's working state.
struct FitData<'a> {
    fonts: &'a mut mv_text::Fonts,
    t: &'a mv_theme::Theme,
    tokens: &'a [mv_viewer::Token],
}

/// The baseline split of a strut in a face at a size.
fn strut(
    fonts: &mut mv_text::Fonts,
    face: mv_text::Face,
    size: f32,
    line_height: f32,
) -> (f32, f32) {
    let below = mv_text::baseline(fonts, face, size, line_height);
    (-below, line_height - below)
}

/// Lays out text/term/paren/sup/frac tokens left to right.
fn set_tokens(
    s: &mut Setter<'_>,
    tokens: &[mv_viewer::Token],
    size: f32,
    line_height: f32,
    x0: f32,
    in_part: bool,
) -> Extent {
    let (mut top, mut bottom) = strut(s.fonts, mv_text::Face::Stix400, size, line_height);
    let mut x = x0;
    for token in tokens {
        match token {
            mv_viewer::Token::Text(t) => {
                let core = t.text.trim();
                if core.is_empty() {
                    x += 1.0f32.max((0.22 * size * t.text.len() as f32).round());
                    continue;
                }
                let lead = t.text.len() - t.text.trim_start_matches(' ').len();
                let trail = t.text.len() - t.text.trim_end_matches(' ').len();
                x += (0.3 * size * lead as f32).round();
                let from = s.out.runs.len();
                let content = set_runs(s, core, size, x, in_part, s.line_ink);
                let boxed = if s.style.whole_px || in_part {
                    mv_text::round_up(content)
                } else {
                    content
                };
                if in_part {
                    for r in &mut s.out.runs[from..] {
                        r.x += (boxed - content) / 2.0;
                    }
                }
                x += boxed;
                x += (0.3 * size * trail as f32).round();
            }
            mv_viewer::Token::Term(t) => {
                let ink = s
                    .style
                    .term_ink
                    .unwrap_or(s.t.colors[crate::sidebar::PARAM_INKS[t.param]]);
                let content = set_runs(s, &t.text, size, x + TERM_PAD[0], in_part, ink);
                let mut width = TERM_PAD[0] * 2.0 + content;
                if !in_part && s.style.whole_px {
                    width = mv_text::round_up(width);
                }
                let (term_top, term_bottom) =
                    strut(s.fonts, mv_text::Face::Stix400, size, line_height);
                s.out.terms.push(EqTerm {
                    param: t.param,
                    x,
                    y: term_top,
                    width,
                    height: term_bottom + s.style.term_bottom - term_top,
                });
                bottom = bottom.max(term_bottom + s.style.term_bottom);
                x += width;
            }
            mv_viewer::Token::Paren(p) => {
                let paren = if p.big { (1.42 * size).round() } else { size };
                let drop = if p.big { 0.0775 * paren } else { 0.0 };
                let shaped = mv_text::shape(
                    s.fonts,
                    mv_text::Style {
                        face: mv_text::Face::Stix400,
                        size: paren,
                        tracking: 0.0,
                        features: enumset::EnumSet::empty(),
                    },
                    &p.text,
                );
                s.out.runs.push(EqRun {
                    shaped,
                    size: paren,
                    x,
                    baseline: drop,
                    ink: s.line_ink,
                });
                let (paren_top, paren_bottom) =
                    strut(s.fonts, mv_text::Face::Stix400, paren, paren);
                top = top.min(drop + paren_top);
                bottom = bottom.max(drop + paren_bottom);
                x += if s.style.whole_px {
                    mv_text::round_up(s.out.runs.last().map(|r| r.shaped.advance).unwrap_or(0.0))
                } else {
                    s.out.runs.last().map(|r| r.shaped.advance).unwrap_or(0.0)
                };
            }
            mv_viewer::Token::Sup(t) => {
                let sup = (0.68 * size).round();
                x += 1.0;
                let shaped = mv_text::shape(
                    s.fonts,
                    mv_text::Style {
                        face: mv_text::Face::Stix400,
                        size: sup,
                        tracking: 0.0,
                        features: mv_text::Features::only(mv_text::Feature::Tabular),
                    },
                    &t.text,
                );
                s.out.runs.push(EqRun {
                    shaped,
                    size: sup,
                    x,
                    baseline: -0.58 * sup,
                    ink: s.line_ink,
                });
                let (sup_top, _) = strut(s.fonts, mv_text::Face::Stix400, sup, 0.0);
                top = top.min(-0.58 * sup + sup_top);
                bottom = bottom.max(-0.58 * sup + sup_top);
                x += if s.style.whole_px {
                    mv_text::round_up(s.out.runs.last().map(|r| r.shaped.advance).unwrap_or(0.0))
                } else {
                    s.out.runs.last().map(|r| r.shaped.advance).unwrap_or(0.0)
                };
            }
            mv_viewer::Token::Frac(f) => {
                let mut frac_extent = Extent {
                    width: 0.0,
                    top,
                    bottom,
                };
                x += set_frac(s, f, size, x, &mut frac_extent);
                top = frac_extent.top;
                bottom = frac_extent.bottom;
            }
        }
    }
    Extent {
        width: x - x0,
        top,
        bottom,
    }
}

/// Shapes italic/non-italic math runs and appends them.
fn set_runs(
    s: &mut Setter<'_>,
    src: &str,
    size: f32,
    x0: f32,
    in_part: bool,
    ink: mv_theme::Color,
) -> f32 {
    let mut x = x0;
    for run in crate::math_text::math_runs(src) {
        let style = mv_text::Style {
            face: crate::math_text::math_face(&run),
            size,
            tracking: 0.0,
            features: mv_text::Features::only(mv_text::Feature::Tabular),
        };
        let shaped = mv_text::shape(s.fonts, style, &run.text);
        if in_part && run.italic {
            let boxed = mv_text::round_up(shaped.advance);
            s.out.runs.push(EqRun {
                x: x + (boxed - shaped.advance) / 2.0,
                shaped,
                size,
                baseline: 0.0,
                ink,
            });
            x += boxed;
        } else {
            s.out.runs.push(EqRun {
                shaped,
                size,
                x,
                baseline: 0.0,
                ink,
            });
            x += s.out.runs.last().map(|r| r.shaped.advance).unwrap_or(0.0);
        }
    }
    x - x0
}

/// Stacks a numerator/denominator pair at 0.8 size with its bar.
fn set_frac(
    s: &mut Setter<'_>,
    frac: &mv_viewer::FracToken,
    size: f32,
    x0: f32,
    e: &mut Extent,
) -> f32 {
    let part_size = (0.8 * size).round();
    let part_line = mv_text::round_up(1.25 * part_size);
    let pad = (0.15 * part_size).round();
    let bar = 1.5f32.max(0.06875 * part_size);

    let num_from = [s.out.runs.len(), s.out.bars.len(), s.out.terms.len()];
    let num = set_tokens(s, &frac.num, part_size, part_line, 0.0, true);
    let den_from = [s.out.runs.len(), s.out.bars.len(), s.out.terms.len()];
    let den = set_tokens(s, &frac.den, part_size, part_line, 0.0, true);
    let den_to = [s.out.runs.len(), s.out.bars.len(), s.out.terms.len()];

    let inner = num.width.max(den.width);
    let num_height = num.bottom - num.top;
    let den_height = den.bottom - den.top;
    let height = num_height + 2.0 + bar + 2.0 + den_height;
    let top = -STIX_X_HEIGHT * size / 2.0 - height / 2.0;

    let shift = |out: &mut EqLine, from: [usize; 3], to: [usize; 3], dx: f32, dy: f32| {
        for r in &mut out.runs[from[0]..to[0]] {
            r.x += dx;
            r.baseline += dy;
        }
        for b in &mut out.bars[from[1]..to[1]] {
            b.x += dx;
            b.y += dy;
        }
        for t in &mut out.terms[from[2]..to[2]] {
            t.x += dx;
            t.y += dy;
        }
    };
    let left = x0 + pad + 4.0;
    shift(
        s.out,
        num_from,
        den_from,
        left + (inner - num.width) / 2.0,
        top - num.top,
    );
    let den_top = top + num_height + 2.0 + bar + 2.0;
    shift(
        s.out,
        den_from,
        den_to,
        left + (inner - den.width) / 2.0,
        den_top - den.top,
    );
    s.out.bars.push(EqBar {
        x: x0 + pad,
        y: top + num_height + 2.0,
        width: inner + 8.0,
        height: bar,
        ink: s.line_ink,
    });

    e.top = e.top.min(top);
    e.bottom = e.bottom.max(top + height);
    2.0 * pad + inner + 8.0
}

/// Lays out a token line, shifted so its line-box top is y 0.
pub fn set_line(
    fonts: &mut mv_text::Fonts,
    t: &mv_theme::Theme,
    tokens: &[mv_viewer::Token],
    size: f32,
    ink: mv_theme::Color,
    style: LineStyle,
) -> EqLine {
    let mut line = EqLine {
        runs: Vec::new(),
        bars: Vec::new(),
        terms: Vec::new(),
        width: 0.0,
        height: 0.0,
        baseline: 0.0,
    };
    let e = {
        let mut s = Setter {
            fonts,
            t,
            line_ink: ink,
            style,
            out: &mut line,
        };
        set_tokens(
            &mut s,
            tokens,
            size,
            mv_text::round_up(1.25 * size),
            0.0,
            false,
        )
    };
    for r in &mut line.runs {
        r.baseline -= e.top;
    }
    for b in &mut line.bars {
        b.y -= e.top;
    }
    for term in &mut line.terms {
        term.y -= e.top;
    }
    line.width = e.width;
    line.height = e.bottom - e.top;
    line.baseline = -e.top;
    line
}

/// Builds the live and form token lines, fit-scaling the live line to width.
pub fn set_equation(
    fonts: &mut mv_text::Fonts,
    t: &mv_theme::Theme,
    state: &mv_viewer::State,
    width: f32,
    lit: enumset::EnumSet<mv_viewer::Param>,
) -> Equation {
    let hold = mv_viewer::LiveHold {
        keep: mv_viewer::dragged_params(state),
        fine: mv_viewer::fine_params(state),
    };
    let live = mv_viewer::equation_tokens(
        state.func,
        state.params,
        mv_viewer::EquationMode::Live,
        hold,
    );
    // The form line never holds its terms: Odin omits the argument, taking the
    // default `hold := Live_Hold{}` (equation.odin:326; viewer/equation.odin:78).
    let form = mv_viewer::equation_tokens(
        state.func,
        state.params,
        mv_viewer::EquationMode::Form,
        mv_viewer::LiveHold {
            keep: enumset::EnumSet::empty(),
            fine: enumset::EnumSet::empty(),
        },
    );
    let ink = t.colors[mv_theme::Token::Foreground];
    // `fit` takes `&dyn Fn` while shaping needs `&mut Fonts`; the cell bridges
    // them until `fit` takes `FnMut`.
    let fonts_cell = std::cell::RefCell::new(fonts);
    let scale = if width > 0.0 {
        mv_text::fit(width - 2.0 * EQUATION_PAD, &|s| {
            let mut fonts_ref = fonts_cell.borrow_mut();
            set_line(&mut fonts_ref, t, &live, LIVE_SIZE * s, ink, EQUATION_LINE).width
        })
    } else {
        1.0
    };
    let fonts: &mut mv_text::Fonts = fonts_cell.into_inner();
    let live_line = set_line(fonts, t, &live, LIVE_SIZE * scale, ink, EQUATION_LINE);
    let form_line = set_line(
        fonts,
        t,
        &form,
        FORM_SIZE,
        t.colors[mv_theme::Token::ForegroundMuted],
        EQUATION_LINE,
    );
    let live_height = LIVE_MIN_HEIGHT.max(live_line.height);
    Equation {
        live: live_line,
        form: form_line,
        live_height,
        lit,
    }
}

/// Padding plus the live row plus the gap plus the form height.
pub fn equation_height(eq: &Equation) -> f32 {
    EQUATION_PAD + eq.live_height + EQUATION_GAP + eq.form.height
}

/// Draws the live line centred in its row, then the form line beneath.
pub fn draw_equation(app: &mut App, bounds: [f32; 4], eq: &Equation) {
    let x = bounds[0] + EQUATION_PAD;
    let top = bounds[1] + EQUATION_PAD;
    draw_eq_line(
        app,
        &eq.live,
        x,
        top + (eq.live_height - eq.live.height) / 2.0,
        eq.lit,
        mv_viewer::EquationMode::Live,
    );
    draw_eq_line(
        app,
        &eq.form,
        x,
        top + eq.live_height + EQUATION_GAP,
        eq.lit,
        mv_viewer::EquationMode::Form,
    );
}

/// Draws term plates and flashes, then runs, then fraction bars.
pub fn draw_eq_line(
    app: &mut App,
    line: &EqLine,
    x: f32,
    y: f32,
    lit: enumset::EnumSet<mv_viewer::Param>,
    mode: mv_viewer::EquationMode,
) {
    for term in &line.terms {
        let (tint, ink) = {
            let t = crate::ui::current_theme(app);
            (
                t.colors[crate::sidebar::PARAM_BANDS[term.param]],
                t.colors[crate::sidebar::PARAM_INKS[term.param]],
            )
        };
        let bounds = [x + term.x, y + term.y, term.width, term.height];
        let flash = mv_theme::fade(
            ink,
            TERM_FLASH * crate::motion_driver::term_flash(app, term.param) as f32,
        );
        if lit.contains(term.param) {
            draw_term_plate(&mut app.vg, bounds, tint, ink, flash, mode);
        } else if flash[3] > 0.0 {
            let vg = &mut app.vg;
            vg.begin_path();
            vg.rounded_rect(bounds[0], bounds[1], bounds[2], bounds[3], TERM_RADIUS);
            vg.fill_color(crate::draw::css_background(flash));
            vg.fill();
        }
    }
    for r in &line.runs {
        mv_text::draw_shaped(
            &mut app.fonts,
            &mut app.vg,
            r.size,
            x + r.x,
            y + r.baseline,
            &r.shaped,
            r.ink,
        );
        note_run(app, r, x, y);
    }
    for b in &line.bars {
        crate::draw::fill_rect(&mut app.vg, [x + b.x, y + b.y, b.width, b.height], b.ink);
    }
}

/// Records the I-beam hover box from the font ascent/descent, unless glyphless.
pub fn note_run(app: &mut App, r: &EqRun, x: f32, y: f32) {
    if r.shaped.glyphs.is_empty() {
        return;
    }
    let m = app.fonts.metrics[r.shaped.glyphs[0].font];
    let top = y + r.baseline - m.ascent * r.size;
    crate::input::note_text(
        app,
        clay::BoundingBox {
            x: x + r.x,
            y: top,
            width: r.shaped.advance,
            height: (m.ascent + m.descent) * r.size,
        },
    );
}

/// The term plate's corner radius and flash opacity.
pub const TERM_RADIUS: f32 = 4.0;
pub const TERM_FLASH: f32 = 0.18;

/// Fills the tinted rounded rect plus flash plus foot underline as one contour.
pub fn draw_term_plate(
    vg: &mut mv_nvg::Context,
    b: [f32; 4],
    tint: mv_theme::Color,
    ink: mv_theme::Color,
    flash: mv_theme::Color,
    mode: mv_viewer::EquationMode,
) {
    let out = if mode == mv_viewer::EquationMode::Live {
        1.0
    } else {
        0.0
    };
    let r = TERM_RADIUS;
    vg.begin_path();
    vg.rounded_rect(b[0], b[1] - out, b[2], b[3] + out, r);
    vg.fill_color(crate::draw::css_background(tint));
    vg.fill();
    if flash[3] > 0.0 {
        vg.begin_path();
        vg.rounded_rect(b[0], b[1], b[2], b[3], r);
        vg.fill_color(crate::draw::css_background(flash));
        vg.fill();
    }
    let x0 = b[0];
    let x1 = b[0] + b[2];
    let bottom = b[1] + b[3] + out;
    let inner = bottom - 2.0;
    let from = 20.0 * PI / 180.0;
    let side = inner - r + r * from.sin();
    vg.save();
    vg.intersect_scissor(x0, b[1], b[2], b[3] + out);
    vg.begin_path();
    vg.move_to(x0, side);
    vg.line_to(x0, bottom - r);
    vg.arc(x0 + r, bottom - r, r, PI, PI / 2.0, mv_nvg::Winding::CCW);
    vg.line_to(x1 - r, bottom);
    vg.arc(x1 - r, bottom - r, r, PI / 2.0, 0.0, mv_nvg::Winding::CCW);
    vg.line_to(x1, side);
    vg.arc(x1 - r, inner - r, r, from, PI / 2.0, mv_nvg::Winding::CW);
    vg.line_to(x0 + r, inner);
    vg.arc(
        x0 + r,
        inner - r,
        r,
        PI / 2.0,
        PI - from,
        mv_nvg::Winding::CW,
    );
    vg.close_path();
    vg.fill_color(ink);
    vg.fill();
    vg.restore();
}

/// The live-centred and form-under offsets from the card's top-left, live
/// then form. (A map by mode; the mode enum has no map derive, so the order
/// is positional.)
fn line_origins(eq: &Equation) -> [[f32; 2]; 2] {
    [
        [
            EQUATION_PAD,
            EQUATION_PAD + (eq.live_height - eq.live.height) / 2.0,
        ],
        [EQUATION_PAD, EQUATION_PAD + eq.live_height + EQUATION_GAP],
    ]
}

/// Encodes a param plus the form offset into a term hit.
pub fn term_hit(p: mv_viewer::Param, mode: mv_viewer::EquationMode) -> Hit {
    Hit {
        kind: HitKind::Term,
        index: p.into_usize() as i64
            + if mode == mv_viewer::EquationMode::Form {
                mv_viewer::Param::LENGTH as i64
            } else {
                0
            },
    }
}

/// Decodes a term hit's index into its param, else nothing.
pub fn term_param(hit: Hit) -> Option<mv_viewer::Param> {
    if hit.kind != HitKind::Term {
        return None;
    }
    Some(mv_viewer::Param::from_usize(
        (hit.index % mv_viewer::Param::LENGTH as i64) as usize,
    ))
}

/// Declares the floating targets over each term.
pub fn equation_terms(app: &mut App, t: &mv_theme::Theme, eq: &Equation) {
    let origins = line_origins(eq);
    let tipped = match term_param(app.input.hover) {
        Some(tip) => {
            app.viewer.hover == Some(mv_viewer::Part::Eq) && app.viewer.eq_over == Some(tip)
        }
        None => false,
    };
    for mode in [mv_viewer::EquationMode::Live, mv_viewer::EquationMode::Form] {
        let line = if mode == mv_viewer::EquationMode::Live {
            &eq.live
        } else {
            &eq.form
        };
        for term in &line.terms {
            let hit = term_hit(term.param, mode);
            let id = mv_clay::id("Term", hit.index as u32);
            let layout = crate::top_bar::layout_config(clay::Sizing {
                width: mv_clay::sizing_fixed(term.width),
                height: mv_clay::sizing_fixed(term.height),
            });
            let floating = crate::top_bar::floating(
                [
                    origins[mode as usize][0] + term.x,
                    origins[mode as usize][1] + term.y,
                ],
                clay::FloatingAttachPointType::LeftTop,
                clay::FloatingAttachPointType::LeftTop,
                0,
            );
            let mut term_decl = crate::top_bar::decl(layout);
            term_decl.floating = floating;
            crate::ui::ui_scope(app, id, &term_decl, |app| {
                if tipped && hit == app.input.hover {
                    term_tip(app, t, term.param);
                }
            });
            crate::input::region(
                app,
                id,
                hit,
                Cursor::EwResize,
                Cursor::EwResize,
                false,
                clay::CornerRadius {
                    topLeft: 0.0,
                    topRight: 0.0,
                    bottomLeft: 0.0,
                    bottomRight: 0.0,
                },
            );
        }
    }
}

/// A term's inverted tip: its letter, value and name lines and its plate.
#[derive(Clone, Debug)]
pub struct TermTip {
    pub letter: mv_text::Line,
    pub value: mv_text::Line,
    pub name: mv_text::Line,
    pub plate: mv_theme::Color,
}

/// The tip's caret, height, padding, gap, radius and text metrics.
pub const TERM_TIP_CARET: [f32; 2] = [12.0, 6.0];
pub const TERM_TIP_HEIGHT: f32 = 28.0;
pub const TERM_TIP_PADDING: [f32; 2] = [8.0, 4.0];
pub const TERM_TIP_GAP: f32 = 6.0;
pub const TERM_TIP_RADIUS: f32 = 6.0;
pub const TERM_TIP_Z: i16 = 2;
pub const TERM_TIP_MATH_SIZE: f32 = 16.0;
pub const TERM_TIP_MATH_LINE: f32 = 20.0;
pub const TERM_TIP_NAME_LINE: f32 = 16.0;

/// Builds the inverted tip lines and declares the floating tip element.
fn term_tip(app: &mut App, t: &mv_theme::Theme, p: mv_viewer::Param) {
    let words = t.colors[mv_theme::Token::Background];
    let fine = mv_viewer::fine_params(&app.viewer).contains(p);
    let value = format!("= {}", mv_numfmt::format_stored(app.viewer.params[p], fine));
    let name = format!("· {}", mv_viewer::PARAM_NAMES[p].to_lowercase());
    let tip = TermTip {
        letter: crate::widgets::math_line(
            &mut app.fonts,
            mv_viewer::PARAM_LETTERS[p],
            TERM_TIP_MATH_SIZE,
            TERM_TIP_MATH_LINE,
            words,
        ),
        value: term_tip_math(app, &value, words),
        name: {
            let piece = mv_text::Piece {
                style: mv_text::Style {
                    face: mv_text::Face::Stix600,
                    size: 14.0,
                    tracking: 0.0,
                    features: enumset::EnumSet::empty(),
                },
                text: name,
                ink: words,
                line_height: TERM_TIP_NAME_LINE,
                r#box: mv_text::Box::Exact,
                min_width: 0.0,
                align: mv_text::Align::Left,
                pad: [0.0, 0.0],
            };
            mv_text::typeset(
                &mut app.fonts,
                &[piece],
                0.0,
                mv_text::Vertical::Baseline,
                1.0,
            )
        },
        plate: t.colors[mv_theme::Token::Foreground],
    };
    let width = term_tip_width(&tip);
    let custom = crate::widgets::Custom::TermTip(tip);
    let mut tip_decl = crate::top_bar::decl(crate::top_bar::layout_config(clay::Sizing {
        width: mv_clay::sizing_fixed(width),
        height: mv_clay::sizing_fixed(TERM_TIP_CARET[1] + TERM_TIP_HEIGHT),
    }));
    tip_decl.floating = crate::top_bar::floating(
        [0.0, 4.0],
        clay::FloatingAttachPointType::CenterTop,
        clay::FloatingAttachPointType::CenterBottom,
        TERM_TIP_Z,
    );
    tip_decl.custom = crate::widgets::custom_data(custom);
    crate::ui::ui_scope(app, mv_clay::id("TermTip", 0), &tip_decl, |_| {});
}

/// Typesets tabular math runs at the tip size.
fn term_tip_math(app: &mut App, s: &str, ink: mv_theme::Color) -> mv_text::Line {
    let mut pieces = Vec::new();
    for run in crate::math_text::math_runs(s) {
        pieces.push(mv_text::Piece {
            style: mv_text::Style {
                face: crate::math_text::math_face(&run),
                size: TERM_TIP_MATH_SIZE,
                tracking: 0.0,
                features: mv_text::Features::only(mv_text::Feature::Tabular),
            },
            text: run.text,
            ink,
            line_height: TERM_TIP_MATH_LINE,
            r#box: mv_text::Box::Exact,
            min_width: 0.0,
            align: mv_text::Align::Left,
            pad: [0.0, 0.0],
        });
    }
    mv_text::typeset(
        &mut app.fonts,
        &pieces,
        0.0,
        mv_text::Vertical::Baseline,
        1.0,
    )
}

/// Padding plus the three run widths plus gaps.
fn term_tip_width(tip: &TermTip) -> f32 {
    let runs = [tip.letter.width, tip.value.width, tip.name.width];
    let mut width = 2.0 * TERM_TIP_PADDING[0] + 2.0 * TERM_TIP_GAP;
    for r in runs {
        width += mv_text::layout_width(r);
    }
    width
}

/// Draws the caret, the rounded body and the three centred runs.
pub fn draw_term_tip(app: &mut App, bounds: clay::BoundingBox, tip: &TermTip) {
    let vg = &mut app.vg;
    let mid = bounds.x + bounds.width / 2.0;
    vg.begin_path();
    vg.move_to(mid, bounds.y);
    vg.line_to(mid + TERM_TIP_CARET[0] / 2.0, bounds.y + TERM_TIP_CARET[1]);
    vg.line_to(mid - TERM_TIP_CARET[0] / 2.0, bounds.y + TERM_TIP_CARET[1]);
    vg.close_path();
    vg.rounded_rect(
        bounds.x,
        bounds.y + TERM_TIP_CARET[1],
        bounds.width,
        TERM_TIP_HEIGHT,
        TERM_TIP_RADIUS,
    );
    vg.fill_color(tip.plate);
    vg.fill();
    let content = bounds.y + TERM_TIP_CARET[1] + TERM_TIP_PADDING[1];
    let inner = TERM_TIP_HEIGHT - 2.0 * TERM_TIP_PADDING[1];
    let mut x = bounds.x + TERM_TIP_PADDING[0];
    for line in [&tip.letter, &tip.value, &tip.name] {
        mv_text::draw_line(
            &mut app.fonts,
            &mut app.vg,
            line,
            x,
            content + (inner - line.height) / 2.0,
        );
        x += mv_text::layout_width(line.width) + TERM_TIP_GAP;
    }
}

/// The equation's term targets and P's field against the reference's DOM.
#[cfg(all(test, not(target_os = "emscripten")))]
mod tests {
    use super::*;

    const SLACK: f32 = 0.02;

    struct Target {
        name: &'static str,
        id: clay::ElementId,
        chromium: [f32; 4],
        webkit: [f32; 4],
    }

    #[test]
    fn the_terms_and_ps_field_are_the_references() {
        let _guard = mv_clay::tests_lock();
        let (mut app, _) = super::super::sidebar::tests::setup();
        assert!(mv_text::init(&mut app.fonts));
        for slug in [
            mv_theme::Slug::ArcticIce,
            mv_theme::Slug::TronCyan,
            mv_theme::Slug::MidnightViolet,
            mv_theme::Slug::CleanTeal,
            mv_theme::Slug::EducationalWarm,
            mv_theme::Slug::SageEditorial,
        ] {
            app.themes[slug] = mv_theme::resolve(slug, mv_theme::Space::Srgb);
        }
        crate::ui::init_clay(&mut app);
        app.system_scheme = mv_theme::Scheme::Dark;
        app.prefs = mv_theme::Prefs {
            mode: mv_theme::Mode::Dark,
            dark: mv_theme::Slug::ArcticIce,
            light: mv_theme::Slug::CleanTeal,
        };
        app.viewer = mv_viewer::initial_state();
        crate::plane_area::plane_init(&mut app);
        let r3 = [
            mv_viewer::Action::SetParam(mv_viewer::SetParam {
                param: mv_viewer::Param::A,
                value: 2.0,
                jump: true,
            }),
            mv_viewer::Action::SetParam(mv_viewer::SetParam {
                param: mv_viewer::Param::K,
                value: 1.0,
                jump: true,
            }),
            mv_viewer::Action::SetParam(mv_viewer::SetParam {
                param: mv_viewer::Param::H,
                value: -1.0,
                jump: true,
            }),
            mv_viewer::Action::SetP(mv_viewer::SetP { x: 0.5 }),
        ];
        for a in r3 {
            mv_viewer::apply(&mut app.viewer, a);
        }
        app.size = [1440.0, 900.0];
        app.pixels = [2880, 1800];
        let size = app.size;
        let _ = crate::draw::settle_layout(&mut app, size);

        let term = |p: mv_viewer::Param, mode: mv_viewer::EquationMode| {
            mv_clay::id("Term", term_hit(p, mode).index as u32)
        };
        let targets = [
            Target {
                name: "live a",
                id: term(mv_viewer::Param::A, mv_viewer::EquationMode::Live),
                chromium: [128.0, 227.0, 55.0, 36.0],
                webkit: [128.0, 227.0, 55.0, 36.0],
            },
            Target {
                name: "live h",
                id: term(mv_viewer::Param::H, mv_viewer::EquationMode::Live),
                chromium: [214.0, 227.0, 82.0, 36.0],
                webkit: [214.0, 227.0, 82.0, 36.0],
            },
            Target {
                name: "live k",
                id: term(mv_viewer::Param::K, mv_viewer::EquationMode::Live),
                chromium: [323.0, 227.0, 82.0, 36.0],
                webkit: [323.0, 227.0, 82.0, 36.0],
            },
            Target {
                name: "form a",
                id: term(mv_viewer::Param::A, mv_viewer::EquationMode::Form),
                chromium: [101.0, 289.0, 16.0, 24.0],
                webkit: [101.0, 288.8, 16.0, 24.0],
            },
            Target {
                name: "form h",
                id: term(mv_viewer::Param::H, mv_viewer::EquationMode::Form),
                chromium: [162.0, 279.0, 14.0, 19.0],
                webkit: [162.0, 279.0, 14.0, 19.0],
            },
            Target {
                name: "form b",
                id: term(mv_viewer::Param::B, mv_viewer::EquationMode::Form),
                chromium: [147.5, 303.5, 13.0, 19.0],
                webkit: [147.5, 303.5, 13.0, 19.0],
            },
            Target {
                name: "form k",
                id: term(mv_viewer::Param::K, mv_viewer::EquationMode::Form),
                chromium: [202.0, 289.0, 33.0, 24.0],
                webkit: [202.0, 288.8, 33.0, 24.0],
            },
            Target {
                name: "P's field",
                id: mv_clay::id("PointField", 0),
                chromium: [308.78, 678.5, 43.22, 24.0],
                webkit: [308.78, 678.5, 43.22, 24.0],
            },
        ];
        let near = |b: clay::BoundingBox, want: [f32; 4]| -> bool {
            let got = [b.x, b.y, b.width, b.height];
            for i in 0..4 {
                if (got[i] - want[i]).abs() > SLACK {
                    return false;
                }
            }
            true
        };
        for target in &targets {
            let data = app
                .clay
                .as_ref()
                .expect("clay context initialised")
                .get_element_data(target.id);
            assert!(data.found, "no {} in the port", target.name);
            let b = data.boundingBox;
            assert!(
                near(b, target.chromium) || near(b, target.webkit),
                "{} at ({:.3}, {:.3}, {:.3} x {:.3}); Chromium's {:?}, WebKit's {:?}",
                target.name,
                b.x,
                b.y,
                b.width,
                b.height,
                target.chromium,
                target.webkit
            );
        }
        for target in &targets[..7] {
            let data = app
                .clay
                .as_ref()
                .expect("clay context initialised")
                .get_element_data(target.id);
            let centre = [
                data.boundingBox.x + data.boundingBox.width / 2.0,
                data.boundingBox.y + data.boundingBox.height / 2.0,
            ];
            assert_eq!(
                crate::input::hit_at(&app.input.regions, centre).kind,
                HitKind::Term,
                "{} takes no pointer",
                target.name
            );
        }
        crate::ui::release_clay(&mut app);
        // `teardown` destroys the fonts; destroying here too would free
        // every font twice (the Odin test destroys once, `defer
        // text.destroy`, with no sidebar teardown).
        super::super::sidebar::tests::teardown(app);
    }
}
