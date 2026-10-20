//! The motion timelines: pure functions of time for the first paint, jumps,
//! switches, Q, the ghost and flashes, and the frame builder over them.

use std::sync::LazyLock;

use mv_motion::{Ease, Level, SPRING_SWEEP, ease_at, spring_at};
use mv_plane::{Curve, Scene, View};
use mv_plane::{Trace, Viewport, curve_trace, share_at_x, to_screen_x, view_of};

use super::copy::Point;
use super::form::{Base, Param, Params};
use super::plane_scene::{POINT_IDS, PlaneSceneInput, build_plane_scene};

/// A second of the fast motion.
const FAST_S: f64 = mv_motion::FAST_MS / 1000.0;

/// A second of the reduced motion.
const REDUCED_S: f64 = mv_motion::REDUCED_FADE_MS / 1000.0;

/// Clamps a share to 0 … 1.
#[allow(clippy::manual_clamp)] // `clamp` panics on NaN; the source's min/max does not.
fn clamp01(v: f64) -> f64 {
    v.min(1.0).max(0.0)
}

/// Moves along an ease between two values.
fn along(t: f64, from: f64, to: f64, ease: Ease) -> f64 {
    if t <= from {
        return 0.0;
    }
    if t >= to {
        return 1.0;
    }
    ease_at(ease, (t - from) / (to - from))
}

/// Inverts an ease: the progress reaching a value.
fn progress_of(ease: Ease, v: f64, up_to: f64) -> f64 {
    let mut lo = 0.0;
    let mut hi = up_to;
    for _ in 0..40 {
        let mid = (lo + hi) / 2.0;
        if ease_at(ease, mid) < v {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (lo + hi) / 2.0
}

/// The first paint's grid span: from grid 0 to 0.4.
pub const FIRST_PAINT_GRID: [f64; 2] = [0.0, 0.4];

/// The first paint's trim span.
pub const FIRST_PAINT_TRIM: [f64; 2] = [0.35, 1.25];

/// The first paint's P span.
pub const FIRST_PAINT_P: [f64; 2] = [1.15, 1.31];

/// The first paint's label span.
pub const FIRST_PAINT_LABEL: [f64; 2] = [1.3, 1.62];

/// The first paint's rise, in px.
pub const FIRST_PAINT_RISE: f64 = 8.0;

/// The first paint's end, in s.
pub const FIRST_PAINT_END: f64 = 2.6;

/// A nudge's start, in s.
pub const NUDGE_FROM: f64 = 1.7;

/// A nudge's out time, in s.
pub const NUDGE_OUT: f64 = 0.25;

/// A nudge's back time, in s.
pub const NUDGE_BACK: f64 = 0.35;

/// A nudge's stagger per step, in s.
pub const NUDGE_STAGGER: f64 = 0.06;

/// A nudge's travel, in px.
pub const NUDGE_PX: f64 = -10.0;

/// The first paint's look: its grid, trim, P, label and rise.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FirstPaint {
    pub grid: f64,
    pub trim: f64,
    pub p: f64,
    pub label: f64,
    pub rise: f64,
}

/// The first paint's look at a time.
pub fn first_paint_at(t: f64) -> FirstPaint {
    let label_in = along(t, FIRST_PAINT_LABEL[0], FIRST_PAINT_LABEL[1], Ease::Enter);
    FirstPaint {
        grid: along(t, FIRST_PAINT_GRID[0], FIRST_PAINT_GRID[1], Ease::Draw),
        trim: along(t, FIRST_PAINT_TRIM[0], FIRST_PAINT_TRIM[1], Ease::Draw),
        p: 1.0f64.min(along(t, FIRST_PAINT_P[0], FIRST_PAINT_P[1], Ease::Enter)),
        label: 1.0f64.min(label_in),
        rise: FIRST_PAINT_RISE * (1.0 - label_in),
    }
}

/// A nudge's offset at a time for a step.
pub fn nudge_at(t: f64, i: i64) -> f64 {
    let start = NUDGE_FROM + (i as f64) * NUDGE_STAGGER;
    if t <= start || t >= start + NUDGE_OUT + NUDGE_BACK {
        return 0.0;
    }
    if t < start + NUDGE_OUT {
        return NUDGE_PX * ease_at(Ease::Enter, (t - start) / NUDGE_OUT);
    }
    NUDGE_PX * (1.0 - ease_at(Ease::Enter, (t - start - NUDGE_OUT) / NUDGE_BACK))
}

/// A jump's duration, in s: the sweep's settle.
pub static JUMP_S: LazyLock<f64> = LazyLock::new(|| mv_motion::EASE_SWEEP.settle);

/// The jump between two parameters at a time.
pub fn jump_at(from: Params, to: Params, t: f64) -> Params {
    if t >= *JUMP_S {
        return to;
    }
    let k = spring_at(SPRING_SWEEP, t);
    let mut out = to;
    for p in [Param::A, Param::K, Param::B, Param::H] {
        out[p] = from[p] + (to[p] - from[p]) * k;
    }
    out
}

/// A switch's fade, in s.
pub const SWITCH_FADE: f64 = FAST_S;

/// A switch's draw, in s.
pub const SWITCH_DRAW: f64 = mv_motion::DRAW_MS / 1000.0;

/// A switch's landing, in s.
pub const SWITCH_LAND: f64 = FAST_S;

/// A switch's end, in s.
pub const SWITCH_END_S: f64 = SWITCH_DRAW + SWITCH_LAND;

/// When a share draws.
pub fn draw_time_of(share: f64) -> f64 {
    if share <= 0.0 {
        return 0.0;
    }
    if share >= 1.0 {
        return SWITCH_DRAW;
    }
    progress_of(Ease::Draw, share, 1.0) * SWITCH_DRAW
}

/// A share's landing at a time.
pub fn landing_at(t: f64, share: f64) -> f64 {
    let at = draw_time_of(share);
    1.0f64.min(along(t, at, at + SWITCH_LAND, Ease::Enter))
}

/// A switch's look: its old curve and trim shares.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SwitchLook {
    pub old: f64,
    pub trim: f64,
}

/// A switch's look at a time.
pub fn switch_at(t: f64) -> SwitchLook {
    SwitchLook {
        old: 1.0 - clamp01(t / SWITCH_FADE),
        trim: along(t, 0.0, SWITCH_DRAW, Ease::Draw),
    }
}

/// Q's enter time, in s.
pub const Q_ENTER: f64 = FAST_S;

/// Q's lines time, in s.
pub const Q_LINES: f64 = 0.12;

/// Q's leave time, in s.
pub const Q_LEAVE: f64 = mv_motion::LEAVE_MS / 1000.0;

/// Q's look: its alpha and reach.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QLook {
    pub alpha: f64,
    pub reach: f64,
}

/// Q at rest: full alpha and reach.
const Q_REST: QLook = QLook {
    alpha: 1.0,
    reach: 1.0,
};

/// When Q's enter reaches an alpha.
pub fn q_enter_time_of(alpha: f64) -> f64 {
    if alpha <= 0.0 {
        return 0.0;
    }
    if alpha >= 1.0 {
        return Q_ENTER;
    }
    progress_of(Ease::Enter, alpha, 0.41) * Q_ENTER
}

/// Q's enter look at a time.
pub fn q_enter_at(t: f64, level: Level) -> QLook {
    if level != Level::Full {
        return QLook {
            alpha: clamp01(t / REDUCED_S),
            reach: 1.0,
        };
    }
    QLook {
        alpha: 1.0f64.min(along(t, 0.0, Q_ENTER, Ease::Enter)),
        reach: 1.0f64.min(along(t, 0.0, Q_LINES, Ease::Enter)),
    }
}

/// Q's leave look at a time from an alpha.
pub fn q_leave_at(t: f64, from: f64, level: Level) -> QLook {
    let k = 0.0f64.max(
        from - t / if level == Level::Full {
            Q_LEAVE
        } else {
            REDUCED_S
        },
    );
    QLook {
        alpha: k,
        reach: if level == Level::Full { k } else { 1.0 },
    }
}

/// The ghost's share between two values at a time.
pub fn ghost_share_at(t: f64, from: f64, to: f64, level: Level) -> f64 {
    from + (to - from)
        * clamp01(
            t / if level == Level::Full {
                FAST_S
            } else {
                REDUCED_S
            },
        )
}

/// A flash's in time, in s.
pub const FLASH_IN: f64 = 0.08;

/// A flash's out time, in s.
pub const FLASH_OUT: f64 = 0.3;

/// A flash's end, in s.
pub const FLASH_END: f64 = 0.6;

/// A flash's alpha at a time.
pub fn flash_at(t: f64, level: Level) -> f64 {
    if t <= 0.0 || t >= FLASH_END {
        return 0.0;
    }
    if t < FLASH_IN {
        return clamp01(if level == Level::Full {
            ease_at(Ease::Enter, t / FLASH_IN)
        } else {
            t / FLASH_IN
        });
    }
    let out = if level == Level::Full {
        FLASH_OUT
    } else {
        FLASH_END - REDUCED_S
    };
    if t < out {
        1.0
    } else {
        1.0 - (t - out) / (FLASH_END - out)
    }
}

/// The motion layers: the paint, jump, switch, Q and ghost in flight.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotionLayers {
    pub paint: Option<f64>,
    pub jump: Option<JumpLayer>,
    pub fn_switch: Option<SwitchLayer>,
    pub q: Option<QLayer>,
    pub ghost: Option<GhostLayer>,
}

/// A jump in flight: from and to parameters since a start.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JumpLayer {
    pub from: Params,
    pub to: Params,
    pub start: f64,
}

/// A function switch in flight: to a function since a start.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SwitchLayer {
    pub func: Base,
    pub params: Params,
    pub start: f64,
}

/// A Q move in flight: in a direction at x since a start.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QLayer {
    pub dir: QDir,
    pub x: f64,
    pub from: f64,
    pub start: f64,
}

/// A Q move's direction: in or out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QDir {
    In,
    Out,
}

/// A ghost move in flight: from and to shares since a start.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GhostLayer {
    pub from: f64,
    pub to: f64,
    pub start: f64,
}

/// No motion in flight.
pub const AT_REST: MotionLayers = MotionLayers {
    paint: None,
    jump: None,
    fn_switch: None,
    q: None,
    ghost: None,
};

/// How long a Q layer runs.
fn q_duration(q: QLayer, level: Level) -> f64 {
    let full = level == Level::Full;
    if q.dir == QDir::In {
        (if full { Q_ENTER } else { REDUCED_S })
            - (if full {
                q_enter_time_of(q.from)
            } else {
                q.from * REDUCED_S
            })
    } else {
        q.from * (if full { Q_LEAVE } else { REDUCED_S })
    }
}

/// Settles the layers at now: drops what finished.
pub fn settle(layers: MotionLayers, now: f64, level: Level) -> MotionLayers {
    let mut out = AT_REST;
    if let Some(start) = layers.paint
        && now - start < FIRST_PAINT_END
    {
        out.paint = Some(start);
    }
    if let Some(l) = layers.jump
        && now - l.start < *JUMP_S
    {
        out.jump = Some(l);
    }
    if let Some(l) = layers.fn_switch
        && now - l.start < SWITCH_END_S
    {
        out.fn_switch = Some(l);
    }
    if let Some(l) = layers.q
        && now - l.start < q_duration(l, level)
    {
        out.q = Some(l);
    }
    if let Some(l) = layers.ghost
        && now - l.start
            < if level == Level::Full {
                FAST_S
            } else {
                REDUCED_S
            }
    {
        out.ghost = Some(l);
    }
    out
}

/// Whether any layer still moves.
pub fn is_moving(layers: &MotionLayers) -> bool {
    layers.paint.is_some()
        || layers.jump.is_some()
        || layers.fn_switch.is_some()
        || layers.q.is_some()
        || layers.ghost.is_some()
}

/// The parameters the layers show at now.
pub fn shown_params(stored: Params, layers: MotionLayers, now: f64) -> Params {
    if let Some(j) = layers.jump {
        return jump_at(j.from, j.to, now - j.start);
    }
    stored
}

/// Q's look under the layers at now.
pub fn q_look_at(layers: &MotionLayers, now: f64, level: Level) -> QLook {
    let Some(q) = layers.q else {
        return Q_REST;
    };
    let t = now - q.start;
    if q.dir == QDir::In {
        q_enter_at(
            t + if level == Level::Full {
                q_enter_time_of(q.from)
            } else {
                q.from * REDUCED_S
            },
            level,
        )
    } else {
        q_leave_at(t, q.from, level)
    }
}

/// A curve faded to an alpha, drawn to a share.
fn faded_curve(c: Curve, alpha: f64, draw_to: f64) -> Curve {
    let mut out = c;
    if alpha < 1.0 {
        out.alpha = Some(out.alpha.unwrap_or(1.0) * alpha);
    }
    if draw_to < 1.0 {
        out.draw_to = Some(draw_to);
    }
    out
}

/// An alpha below one, if it is one.
fn below_one(v: f64) -> Option<f64> {
    if v < 1.0 { Some(v) } else { None }
}

/// Builds the scene the motion shows at now in a view.
pub fn motion_scene(
    input: &PlaneSceneInput,
    layers: MotionLayers,
    now: f64,
    view: Option<View>,
    level: Level,
) -> Scene {
    let params = shown_params(input.params, layers, now);
    let first = layers.paint.map(|start| first_paint_at(now - start));
    let (sw_t, sw_at) = match layers.fn_switch {
        Some(sw) => (now - sw.start, switch_at(now - sw.start)),
        None => (
            0.0,
            SwitchLook {
                old: 0.0,
                trim: 1.0,
            },
        ),
    };
    let switching = layers.fn_switch.is_some();
    let q = q_look_at(&layers, now, level);
    let mut q_x = input.q_x;
    if let Some(l) = layers.q
        && q_x.is_none()
        && l.dir == QDir::Out
    {
        q_x = Some(l.x);
    }
    let mut ghost = if input.ghost_on { 1.0 } else { 0.0 };
    if let Some(g) = layers.ghost {
        ghost = ghost_share_at(now - g.start, g.from, g.to, level);
    }
    let mut shown = input.clone();
    shown.params = params;
    shown.q_x = q_x;
    shown.ghost_on = ghost > 0.0;
    let mut scene = build_plane_scene(&shown);
    let trim = first.map(|f| f.trim).unwrap_or(1.0).min(sw_at.trim);

    let mut trace: Option<Trace> = None;
    let mut vp: Option<Viewport> = None;
    if switching && let Some(v) = view {
        let laid_out = view_of(v);
        vp = Some(laid_out);
        for c in &scene.curves {
            if c.id == "f" {
                trace = Some(curve_trace(laid_out, &c.func));
            }
        }
    }
    let land = |t: f64, x: f64| -> f64 {
        match &trace {
            None => 1.0,
            Some(tr) => landing_at(t, share_at_x(tr, to_screen_x(vp.unwrap(), x))),
        }
    };

    let mut curves: Vec<Curve> = Vec::with_capacity(scene.curves.len() + 2);
    if switching && sw_at.old > 0.0 {
        let sw = layers.fn_switch.unwrap();
        let mut before = input.clone();
        before.func = sw.func;
        before.params = sw.params;
        before.q_x = None;
        for c in build_plane_scene(&before).curves {
            let is_f = c.id == "f";
            let mut old = faded_curve(c, sw_at.old, 1.0);
            old.id = if is_f {
                "f-old".to_string()
            } else {
                "original-old".to_string()
            };
            curves.push(old);
        }
    }
    for c in std::mem::take(&mut scene.curves) {
        let shown_curve = match c.id.as_str() {
            "original" => faded_curve(c, ghost, trim),
            "f" => faded_curve(c, 1.0, trim),
            _ => c,
        };
        if matches!(shown_curve.alpha, Some(a) if a <= 0.0) {
            continue;
        }
        curves.push(shown_curve);
    }

    let p_alpha = first.map(|f| f.p).unwrap_or(1.0);
    let mut points = Vec::with_capacity(scene.points.len());
    for mut p in std::mem::take(&mut scene.points) {
        if p.id == POINT_IDS[Point::P] {
            p.alpha = below_one(p_alpha * land(sw_t, p.x));
            if let Some(f) = first {
                p.label_alpha = below_one(f.label);
                p.label_rise = f.rise;
            }
        } else if p.id == POINT_IDS[Point::Q] {
            p.alpha = below_one(q.alpha * land(sw_t, p.x));
            p.reach = below_one(q.reach);
        }
        points.push(p);
    }
    let mut handles = Vec::with_capacity(scene.handles.len());
    for mut h in std::mem::take(&mut scene.handles) {
        h.alpha = below_one(p_alpha * land(sw_t, h.x));
        handles.push(h);
    }
    let mut q_alpha = q.alpha;
    if let Some(x) = q_x {
        q_alpha *= land(sw_t, x);
    }
    let mut guides = Vec::with_capacity(scene.guides.len());
    for mut g in std::mem::take(&mut scene.guides) {
        g.alpha = below_one(q_alpha);
        guides.push(g);
    }
    if let Some(f) = first {
        scene.grid_alpha = below_one(f.grid);
    }
    scene.curves = curves;
    scene.points = points;
    scene.handles = handles;
    scene.guides = guides;
    scene
}

#[cfg(test)]
mod tests {
    use super::*;
    use mv_motion::{ease_at, ease_draw};

    fn percent(v: f64) -> i64 {
        (v * 100.0).round() as i64
    }

    fn close_to(a: f64, b: f64, digits: i32) -> bool {
        (a - b).abs() < 10f64.powi(-digits) / 2.0
    }

    const BASE: PlaneSceneInput = PlaneSceneInput {
        func: Base::X2,
        params: crate::form::DEFAULT_PARAMS,
        p_x: 2.0,
        q_x: None,
        ghost_on: true,
        p_lit: false,
        active: None,
        handle_lit: None,
        handle_held: None,
        p_was: None,
    };

    const R2_VIEW: View = View {
        zoom: 50.0,
        extent: mv_plane::Extent {
            min_x: -9.36,
            max_x: 9.36,
            min_y: -7.92,
            max_y: 7.92,
        },
    };

    const FROM: Params = enum_map::EnumMap::from_array([2.0, 1.0, 1.5, -1.0]);

    fn curve_ids(scene: &Scene) -> Vec<String> {
        scene.curves.iter().map(|c| c.id.clone()).collect()
    }

    fn expect_ids(got: &[String], want: &[&str]) {
        assert_eq!(got.len(), want.len());
        for (i, id) in want.iter().enumerate() {
            assert_eq!(got[i], *id);
        }
    }

    fn curve_of<'a>(scene: &'a Scene, id: &str) -> Option<&'a Curve> {
        scene.curves.iter().find(|c| c.id == id)
    }

    #[test]
    fn a_jump_springs_a_b_h_and_k_together() {
        let want = [0, 28, 62, 82, 92, 99];
        for (i, at) in [0.0, 0.08, 0.16, 0.24, 0.32, 0.48].iter().enumerate() {
            let p = jump_at(FROM, crate::form::DEFAULT_PARAMS, *at);
            assert_eq!(
                percent((FROM[Param::A] - p[Param::A]) / (FROM[Param::A] - 1.0)),
                want[i]
            );
            assert!(close_to(
                (FROM[Param::B] - p[Param::B]) / 0.5,
                (FROM[Param::A] - p[Param::A]) / 1.0,
                12
            ));
            assert!(close_to(
                (p[Param::H] - FROM[Param::H]) / 1.0,
                (FROM[Param::K] - p[Param::K]) / 1.0,
                12
            ));
        }
    }

    #[test]
    fn a_jump_lands_exactly_when_the_spring_has_settled() {
        assert!(close_to(*JUMP_S, 0.702, 3));
        assert_eq!(
            jump_at(FROM, crate::form::DEFAULT_PARAMS, *JUMP_S),
            crate::form::DEFAULT_PARAMS
        );
    }

    #[test]
    fn a_switch_draws_the_new_curve_and_fades_the_old_one() {
        let want = [0, 3, 15, 50, 85, 100];
        for (i, at) in [0.0, 0.16, 0.3, 0.45, 0.6, 0.9].iter().enumerate() {
            assert_eq!(percent(switch_at(*at).trim), want[i]);
        }
        assert_eq!(switch_at(0.0).old, 1.0);
        assert!(close_to(switch_at(0.08).old, 0.5, 9));
        assert_eq!(switch_at(0.16).old, 0.0);
    }

    #[test]
    fn a_switch_lands_a_mark_as_the_pen_passes_it() {
        assert!(close_to(draw_time_of(0.5), 0.45, 6));
        assert_eq!(draw_time_of(0.0), 0.0);
        assert_eq!(draw_time_of(1.0), 0.9);
        assert_eq!(landing_at(0.45, 0.5), 0.0);
        assert!(close_to(
            landing_at(0.53, 0.5),
            1.0f64.min(ease_at(Ease::Enter, 0.5)),
            6
        ));
        assert_eq!(landing_at(0.61, 0.5), 1.0);
    }

    #[test]
    fn a_switch_lands_p_at_x_1_on_sin_x_between_the_filmstrips_450_and_600_ms() {
        let layers = MotionLayers {
            paint: None,
            jump: None,
            fn_switch: Some(SwitchLayer {
                func: Base::X2,
                params: crate::form::DEFAULT_PARAMS,
                start: 0.0,
            }),
            q: None,
            ghost: None,
        };
        let input = PlaneSceneInput {
            func: Base::Sin,
            p_x: 1.0,
            ..BASE.clone()
        };
        let p_at = |input: &PlaneSceneInput, layers: MotionLayers, at: f64| -> f64 {
            motion_scene(input, layers, at, Some(R2_VIEW), Level::Full).points[0]
                .alpha
                .unwrap_or(1.0)
        };
        assert_eq!(p_at(&input, layers, 0.45), 0.0);
        assert!(p_at(&input, layers, 0.6) > 0.9);
    }

    #[test]
    fn the_first_paint_fades_the_grid_in_draws_the_curve_then_p_and_its_label_arrive() {
        assert!(close_to(first_paint_at(0.15).grid, ease_draw(0.375), 9));
        assert_eq!(first_paint_at(0.4).grid, 1.0);
        assert_eq!(first_paint_at(0.35).trim, 0.0);
        assert!(close_to(first_paint_at(0.8).trim, 0.5, 9));
        assert_eq!(first_paint_at(1.25).trim, 1.0);
        assert_eq!(first_paint_at(1.15).p, 0.0);
        assert_eq!(first_paint_at(1.31).p, 1.0);
        let at_130 = first_paint_at(1.3);
        assert!(at_130.label == 0.0 && at_130.rise == 8.0);
        let at_162 = first_paint_at(1.62);
        assert!(at_162.label == 1.0 && at_162.rise == 0.0);
    }

    #[test]
    fn the_first_paint_nudges_each_ruler_10_px_left_and_back() {
        assert_eq!(nudge_at(1.7, 0), 0.0);
        assert_eq!(nudge_at(1.95, 0), -10.0);
        assert_eq!(nudge_at(2.3, 0), 0.0);
        assert!(close_to(
            nudge_at(1.95, 3),
            -10.0 * ease_at(Ease::Enter, 0.07 / 0.25),
            9
        ));
        assert_eq!(nudge_at(2.55, 3), 0.0);
        let least = nudge_at(1.8, 0)
            .min(nudge_at(1.85, 0))
            .min(nudge_at(1.9, 0));
        assert!(least < -10.0);
    }

    #[test]
    fn a_first_paint_frame_grid_faint_curve_part_way_p_not_yet_in() {
        let layers = MotionLayers {
            paint: Some(0.0),
            jump: None,
            fn_switch: None,
            q: None,
            ghost: None,
        };
        let at_15 = motion_scene(&BASE, layers, 0.15, Some(R2_VIEW), Level::Full);
        assert!(close_to(
            at_15.grid_alpha.unwrap_or(-1.0),
            ease_draw(0.375),
            9
        ));
        let f = curve_of(&at_15, "f").unwrap();
        assert_eq!(f.draw_to.unwrap_or(-1.0), 0.0);
        let at_80 = motion_scene(&BASE, layers, 0.8, Some(R2_VIEW), Level::Full);
        assert!(at_80.grid_alpha.is_none());
        let f = curve_of(&at_80, "f").unwrap();
        assert!(close_to(f.draw_to.unwrap_or(-1.0), 0.5, 9));
        assert_eq!(at_80.points[0].alpha.unwrap_or(-1.0), 0.0);
        assert_eq!(at_80.handles.len(), 2);
        for h in &at_80.handles {
            assert_eq!(h.alpha.unwrap_or(-1.0), 0.0);
        }
        let at_135 = motion_scene(&BASE, layers, 1.35, Some(R2_VIEW), Level::Full);
        assert!(at_135.points[0].alpha.is_none());
        assert!(close_to(
            at_135.points[0].label_alpha.unwrap_or(-1.0),
            0.59,
            2
        ));
        assert!(close_to(at_135.points[0].label_rise, 8.0 * 0.41, 1));
    }

    #[test]
    fn q_fades_in_in_160_ms_and_grows_its_drop_lines_in_120() {
        assert_eq!(
            q_enter_at(0.0, Level::Full),
            QLook {
                alpha: 0.0,
                reach: 0.0
            }
        );
        assert_eq!(q_enter_at(0.12, Level::Full).reach, 1.0);
        assert_eq!(
            q_enter_at(0.16, Level::Full),
            QLook {
                alpha: 1.0,
                reach: 1.0
            }
        );
        assert!(close_to(
            q_enter_at(q_enter_time_of(0.5), Level::Full).alpha,
            0.5,
            6
        ));
    }

    #[test]
    fn q_leaves_the_same_way_reversed_linearly_in_120_ms() {
        assert_eq!(
            q_leave_at(0.06, 1.0, Level::Full),
            QLook {
                alpha: 0.5,
                reach: 0.5
            }
        );
        assert_eq!(q_leave_at(0.12, 1.0, Level::Full).alpha, 0.0);
    }

    #[test]
    fn q_only_fades_under_reduced_motion() {
        assert_eq!(
            q_enter_at(0.06, Level::Reduced),
            QLook {
                alpha: 0.5,
                reach: 1.0
            }
        );
        assert_eq!(
            q_leave_at(0.06, 1.0, Level::Reduced),
            QLook {
                alpha: 0.5,
                reach: 1.0
            }
        );
    }

    #[test]
    fn a_leaving_q_draws_where_it_was_its_guide_fading_with_it() {
        let layers = MotionLayers {
            paint: None,
            jump: None,
            fn_switch: None,
            q: Some(QLayer {
                dir: QDir::Out,
                x: -1.5,
                from: 1.0,
                start: 0.0,
            }),
            ghost: None,
        };
        let scene = motion_scene(&BASE, layers, 0.06, Some(R2_VIEW), Level::Full);
        let q = &scene.points[1];
        assert_eq!(q.id, "q");
        assert_eq!(q.x, -1.5);
        assert_eq!(q.y, 2.25);
        assert_eq!(q.alpha.unwrap_or(-1.0), 0.5);
        assert_eq!(q.reach.unwrap_or(-1.0), 0.5);
        assert_eq!(scene.guides.len(), 1);
        assert_eq!(
            scene.guides[0],
            mv_plane::Guide {
                x: -1.5,
                alpha: Some(0.5)
            }
        );
    }

    #[test]
    fn the_original_fades_0_to_75_percent_in_160_ms() {
        assert!(close_to(
            ghost_share_at(0.08, 1.0, 0.0, Level::Full),
            0.5,
            9
        ));
        assert!(close_to(
            ghost_share_at(0.06, 0.0, 1.0, Level::Reduced),
            0.5,
            9
        ));
        let layers = MotionLayers {
            paint: None,
            jump: None,
            fn_switch: None,
            q: None,
            ghost: Some(GhostLayer {
                from: 1.0,
                to: 0.0,
                start: 0.0,
            }),
        };
        let input = PlaneSceneInput {
            params: FROM,
            ghost_on: false,
            ..BASE.clone()
        };
        let scene = motion_scene(&input, layers, 0.08, Some(R2_VIEW), Level::Full);
        let original = curve_of(&scene, "original").unwrap();
        assert!(close_to(original.alpha.unwrap_or(-1.0), 0.375, 9));
        expect_ids(
            &curve_ids(&motion_scene(
                &input,
                AT_REST,
                0.0,
                Some(R2_VIEW),
                Level::Full,
            )),
            &["f"],
        );
    }

    #[test]
    fn a_changed_term_flashes_in_80_ms_holds_and_lets_go_by_600() {
        assert_eq!(flash_at(0.0, Level::Full), 0.0);
        assert_eq!(flash_at(0.2, Level::Full), 1.0);
        assert!(close_to(flash_at(0.45, Level::Full), 0.5, 9));
        assert_eq!(flash_at(0.6, Level::Full), 0.0);
        assert!(close_to(flash_at(0.54, Level::Reduced), 0.5, 9));
    }

    fn expect_same_point(a: &mv_plane::Point, b: &mv_plane::Point) {
        assert_eq!(a.id, b.id);
        assert_eq!(a.x, b.x);
        assert_eq!(a.y, b.y);
        assert_eq!(a.style, b.style);
        assert_eq!(a.ink, b.ink);
        assert_eq!(a.name, b.name);
        assert_eq!(a.label_place, b.label_place);
        assert_eq!(a.focus, b.focus);
        assert_eq!(a.drop_lines, b.drop_lines);
        assert_eq!(a.edge_marker, b.edge_marker);
        assert_eq!(a.draggable, b.draggable);
        assert_eq!(a.was, b.was);
        assert_eq!(a.alpha, b.alpha);
        assert_eq!(a.label_alpha, b.label_alpha);
        assert_eq!(a.label_rise, b.label_rise);
        assert_eq!(a.reach, b.reach);
    }

    fn expect_same_handle(a: &mv_plane::Handle, b: &mv_plane::Handle) {
        assert_eq!(a.id, b.id);
        assert_eq!(a.x, b.x);
        assert_eq!(a.y, b.y);
        assert_eq!(a.shape, b.shape);
        assert_eq!(a.ink, b.ink);
        assert_eq!(a.halo, b.halo);
        assert_eq!(a.held, b.held);
        assert_eq!(a.alpha, b.alpha);
    }

    fn expect_same_scene(a: &Scene, b: &Scene) {
        assert_eq!(a.grid_alpha, b.grid_alpha);
        assert_eq!(a.curves.len(), b.curves.len());
        for (c, d) in a.curves.iter().zip(b.curves.iter()) {
            assert!(
                c.id == d.id
                    && c.ink == d.ink
                    && c.width == d.width
                    && c.glow == d.glow
                    && c.dash == d.dash
                    && c.alpha == d.alpha
                    && c.avoid == d.avoid
                    && c.back == d.back
                    && c.draw_to == d.draw_to
            );
        }
        assert_eq!(a.points.len(), b.points.len());
        for (p, q) in a.points.iter().zip(b.points.iter()) {
            expect_same_point(p, q);
        }
        assert_eq!(a.handles.len(), b.handles.len());
        for (h, i) in a.handles.iter().zip(b.handles.iter()) {
            expect_same_handle(h, i);
        }
        assert_eq!(a.guides, b.guides);
        assert_eq!(a.annotations.len(), b.annotations.len());
    }

    #[test]
    fn at_rest_the_frame_is_the_views_own_scene() {
        expect_same_scene(
            &motion_scene(&BASE, AT_REST, 5.0, Some(R2_VIEW), Level::Full),
            &build_plane_scene(&BASE),
        );
        let r3 = PlaneSceneInput {
            params: FROM,
            q_x: Some(-2.0),
            ..BASE.clone()
        };
        let scene = motion_scene(&r3, AT_REST, 5.0, Some(R2_VIEW), Level::Full);
        expect_same_scene(&scene, &build_plane_scene(&r3));
        let f = curve_of(&scene, "f").unwrap();
        assert_eq!(mv_plane::curve_at(&f.func, -1.0), 1.0);
    }

    #[test]
    fn reset_all_keeps_the_original_until_the_curve_lands_on_it() {
        let layers = MotionLayers {
            paint: None,
            jump: Some(JumpLayer {
                from: FROM,
                to: crate::form::DEFAULT_PARAMS,
                start: 0.0,
            }),
            fn_switch: None,
            q: None,
            ghost: None,
        };
        let mid = motion_scene(&BASE, layers, 0.16, Some(R2_VIEW), Level::Full);
        expect_ids(&curve_ids(&mid), &["original", "f"]);
        let p = &mid.points[0];
        assert_eq!(p.x, 2.0);
        assert!(close_to(
            p.y,
            mv_plane::curve_at(&mid.curves[1].func, 2.0),
            12
        ));
    }

    #[test]
    fn the_old_curve_fades_under_the_new_ones_draw_on() {
        let layers = MotionLayers {
            paint: None,
            jump: None,
            fn_switch: Some(SwitchLayer {
                func: Base::X2,
                params: crate::form::DEFAULT_PARAMS,
                start: 0.0,
            }),
            q: None,
            ghost: None,
        };
        let input = PlaneSceneInput {
            func: Base::Sin,
            ..BASE.clone()
        };
        let scene = motion_scene(&input, layers, 0.08, Some(R2_VIEW), Level::Full);
        expect_ids(&curve_ids(&scene), &["f-old", "f"]);
        assert_eq!(scene.curves[0].alpha.unwrap_or(-1.0), 0.5);
        assert!(scene.curves[0].draw_to.is_none());
        assert!(scene.curves[1].alpha.is_none());
        assert_eq!(
            scene.curves[1].draw_to.unwrap_or(-1.0),
            ease_draw(0.08 / 0.9)
        );
    }

    #[test]
    fn each_layer_settles_once_it_has_run() {
        let layers = MotionLayers {
            paint: Some(0.0),
            jump: Some(JumpLayer {
                from: FROM,
                to: crate::form::DEFAULT_PARAMS,
                start: 0.0,
            }),
            fn_switch: Some(SwitchLayer {
                func: Base::X2,
                params: crate::form::DEFAULT_PARAMS,
                start: 0.0,
            }),
            q: Some(QLayer {
                dir: QDir::In,
                x: 0.0,
                from: 0.0,
                start: 0.0,
            }),
            ghost: Some(GhostLayer {
                from: 0.0,
                to: 1.0,
                start: 0.0,
            }),
        };
        assert_eq!(settle(layers, 0.1, Level::Full), layers);
        let at_2 = settle(layers, 0.2, Level::Full);
        assert!(at_2.q.is_none() && at_2.ghost.is_none());
        let at_8 = settle(layers, 0.8, Level::Full);
        assert!(at_8.jump.is_none() && at_8.fn_switch == layers.fn_switch);
        assert!(settle(layers, 1.1, Level::Full).fn_switch.is_none());
        assert!(!is_moving(&settle(layers, 2.6, Level::Full)));
        assert!(is_moving(&settle(layers, 2.5, Level::Full)));
    }
}
