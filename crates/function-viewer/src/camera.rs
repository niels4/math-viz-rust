//! The Cartesian plane's camera: drag to pan, flick to glide, wheel to zoom.
//!
//! A port of `src/components/views/CartesianPlane/useCamera.ts` (with the
//! camera's share of `queryParams.ts` and `constants.ts`) at math-viz2
//! `c409aad`. The hook's per-frame React writes become plain state the owner
//! reads after each [`Camera::step`]; the `requestAnimationFrame` loop becomes
//! `step`, called once per frame with the frame's `dt`.
//!
//! Wiring (owned by the h2-input coder): feed pointer and wheel events in,
//! call `step` while [`Camera::animating`] reports a scheduled frame, forward
//! `center`/`scale` to `url.rs` after each frame, and when `step` returns
//! false while [`Camera::settled`] holds, flush the URL (the hook's
//! `onSettle`). External camera values (URL back/forward) enter through
//! [`Camera::adopt_if_idle`], never by field writes.

use std::collections::VecDeque;

use mv_numfmt::{js_to_string, to_fixed, to_precision};

/// A plane unit is this many px wide at scale 1 (`constants.ts`).
pub const UNSCALED_UNIT_SIZE: f64 = 25.0;

/// The zoom's range: a plane unit is 2500 px wide at 0.01, 0.25 px at 100
/// (`queryParams.ts`).
pub const MIN_SCALE: f64 = 0.01;
/// The zoom's range (`queryParams.ts`).
pub const MAX_SCALE: f64 = 100.0;

/// The camera the URL shows when it names none (`queryParams.ts` defaults).
pub const DEFAULT_CENTER: [f64; 2] = [0.0, 0.0];
/// The zoom the URL shows when it names none (`queryParams.ts` default).
pub const DEFAULT_SCALE: f64 = 0.5;

// A wheel event multiplies the scale by exp(deltaY * scaleSpeed), so equal
// notches zoom by equal ratios at every scale.
const SCALE_SPEED: f64 = 0.001;
// The scale eases toward its target: each ms covers this fraction of the
// remaining distance, so the distance shrinks by e every 83 ms.
const ZOOM_RATE: f64 = 0.012;
// The zoom snaps to its target once it is within this fraction of it, so the
// snap moves a point 400 px from the cursor by under half a px at any scale.
const SCALE_EPSILON: f64 = 0.001;
// Release faster than this (px per ms) starts a glide; slower just stops.
const MIN_FLICK_SPEED: f64 = 0.15;
// Exponential decay of the glide per ms. Higher stops sooner.
const FRICTION: f64 = 0.0045;
// The glide ends below this speed (px per ms).
const STOP_SPEED: f64 = 0.02;
// The release speed is the pointer's average over this many ms before
// letting go, so a pause before the release counts as standing still.
const FLICK_WINDOW: f64 = 80.0;
// Clamp frame steps so a backgrounded tab does not jump on return.
const MAX_FRAME_DT: f64 = 50.0;
// The first frame of an animation has no earlier frame to measure from,
// and its timestamp can be earlier than the event that requested it.
const FIRST_FRAME_DT: f64 = 1000.0 / 60.0;

/// Which button a press used. A mouse press drags only with button 0, the
/// primary button, as the TypeScript checks `button === 0`; pen and touch
/// always drag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerButton {
    Mouse(u32),
    Other,
}

// A pointer position and its event's timeStamp. Event times, not the time
// the handler runs, so moves the browser dispatches back to back keep the
// spacing they happened with.
struct Sample {
    time: f64,
    at: [f64; 2],
}

// Drops samples from before the flick window that ends at `now`, keeping
// the latest one at or before its start, where the release speed measures
// from.
fn trim_samples(samples: &mut VecDeque<Sample>, now: f64) {
    while samples.len() >= 2 && samples[1].time <= now - FLICK_WINDOW {
        samples.pop_front();
    }
}

fn clamp_scale(scale: f64) -> f64 {
    MAX_SCALE.min(MIN_SCALE.max(scale))
}

/// Pan, glide and wheel zoom for the Cartesian plane, on one animation
/// loop. A drag pans, a fast release glides on, and the wheel eases the zoom
/// about the cursor; all three can run at once.
///
/// While the camera moves, its own values are the truth. External values
/// (URL back/forward) only take effect while it is still, through
/// [`Camera::adopt_if_idle`].
pub struct Camera {
    /// What is drawn: the plane point at the canvas center.
    pub center: [f64; 2],
    /// What is drawn: the zoom.
    pub scale: f64,
    // Where the zoom is heading. The center has no target: a zoom shifts
    // it by however much the scale changed this frame.
    target_scale: f64,
    // Cursor of the latest wheel event, as offset from the canvas center.
    anchor: [f64; 2],
    // Release velocity in screen px per ms while the plane glides.
    glide: Option<[f64; 2]>,
    // The pointer that drags the plane: which one it is, where it last was,
    // and its recent positions for the release speed.
    pointer: Option<PointerDrag>,
    // A frame is scheduled: the next `step` runs the loop again.
    running: bool,
    // The next `step` is the loop's first frame.
    first_frame: bool,
    // The plane unmounted: everything after this is ignored.
    stopped: bool,
}

struct PointerDrag {
    id: u64,
    last: [f64; 2],
    samples: VecDeque<Sample>,
}

impl Camera {
    /// The camera the URL params hold.
    #[must_use]
    pub fn new(center: [f64; 2], scale: f64) -> Self {
        Self {
            center,
            scale,
            target_scale: scale,
            anchor: [0.0, 0.0],
            glide: None,
            pointer: None,
            running: false,
            first_frame: false,
            stopped: false,
        }
    }

    /// Adopt externally set values (URL back/forward) while the camera is
    /// still. While it moves, its own values are the truth and this is a
    /// no-op. The hook reads this from its props at the top of its wheel
    /// and pointer-down handlers; here the fields already are the truth
    /// while idle, so the caller applies outside changes through this.
    pub fn adopt_if_idle(&mut self, center: [f64; 2], scale: f64) {
        if self.stopped || !self.settled() {
            return;
        }
        self.center = center;
        self.scale = scale;
        self.target_scale = scale;
    }

    /// True while a frame is scheduled: keep calling [`Camera::step`].
    /// A wheel event that changes nothing and a drag with no motion leave
    /// this false.
    #[must_use]
    pub fn animating(&self) -> bool {
        self.running
    }

    /// True when the camera is at rest: no pointer holds it, no glide runs,
    /// no zoom is in flight. Check after [`Camera::step`] returns false:
    /// that moment is the hook's `onSettle`, where the owner flushes the URL.
    #[must_use]
    pub fn settled(&self) -> bool {
        self.pointer.is_none() && self.glide.is_none() && self.target_scale == self.scale
    }

    /// The plane unmounted: cancel the loop, drop the glide and the held
    /// pointer. The hook's cleanup removes the listeners and cancels the
    /// frame; after it, late events do nothing.
    pub fn stop(&mut self) {
        self.running = false;
        self.glide = None;
        self.pointer = None;
        self.stopped = true;
    }

    fn ensure_loop(&mut self) {
        if !self.running {
            self.running = true;
            self.first_frame = true;
        }
    }

    // Moves the center so the plane follows the pointer or the glide by
    // (dx, dy) screen px at the scale drawn now.
    fn pan_by(&mut self, dx: f64, dy: f64) {
        let unit = UNSCALED_UNIT_SIZE / self.scale;
        self.center = [self.center[0] - dx / unit, self.center[1] + dy / unit];
    }

    /// A wheel notch about the cursor: `dy` in px (positive is the DOM's
    /// `deltaY` direction, which `useCamera.ts` reads raw with no `deltaMode`
    /// branch), `at` the cursor's offset from the canvas center in CSS px, as
    /// the draw code measures that center. Already at a scale limit, nothing
    /// starts.
    pub fn wheel(&mut self, dx: f64, dy: f64, at: [f64; 2]) {
        // The TypeScript reads only deltaY; a horizontal wheel does not move
        // the plane.
        let _ = dx;
        if self.stopped {
            return;
        }
        self.anchor = at;
        self.target_scale = clamp_scale(self.target_scale * (dy * SCALE_SPEED).exp());
        // Already at a scale limit: nothing to animate.
        if self.target_scale == self.scale {
            return;
        }
        self.ensure_loop();
    }

    /// A pointer went down on the plane. Catching the plane ends a glide;
    /// a zoom in flight carries on.
    pub fn pointer_down(&mut self, id: u64, at: [f64; 2], time_ms: f64, button: PointerButton) {
        if self.stopped || self.pointer.is_some() {
            return;
        }
        if matches!(button, PointerButton::Mouse(b) if b != 0) {
            return;
        }
        self.glide = None;
        self.pointer = Some(PointerDrag {
            id,
            last: at,
            samples: VecDeque::from([Sample { time: time_ms, at }]),
        });
    }

    /// A held pointer moved. Several moves may land in one frame; each pans
    /// at once and the frame publishes once.
    pub fn pointer_move(&mut self, id: u64, at: [f64; 2], time_ms: f64) {
        if self.stopped {
            return;
        }
        let Some(drag) = self.pointer.as_mut() else {
            return;
        };
        if drag.id != id {
            return;
        }
        let dx = at[0] - drag.last[0];
        let dy = at[1] - drag.last[1];
        drag.last = at;
        drag.samples.push_back(Sample { time: time_ms, at });
        trim_samples(&mut drag.samples, time_ms);
        self.pan_by(dx, dy);
        self.ensure_loop();
    }

    /// A held pointer let go (or was cancelled). The next frame glides on,
    /// or settles and writes the URL.
    pub fn pointer_up(&mut self, id: u64, time_ms: f64) {
        if self.stopped {
            return;
        }
        let Some(mut drag) = self.pointer.take() else {
            return;
        };
        if drag.id != id {
            self.pointer = Some(drag);
            return;
        }
        let velocity = release_velocity(&mut drag.samples, time_ms);
        if velocity[0].hypot(velocity[1]) >= MIN_FLICK_SPEED {
            self.glide = Some(velocity);
        }
        self.ensure_loop();
    }

    /// Run one animation frame with the frame's `dt` in ms. Returns true
    /// while another frame is scheduled (still animating). The first frame
    /// after idle uses `1000/60`; later frames clamp to 50 ms, as the hook's
    /// `firstFrameDt` and `maxFrameDt`.
    #[must_use]
    pub fn step(&mut self, dt_ms: f64) -> bool {
        if self.stopped || !self.running {
            return false;
        }
        let dt = if self.first_frame {
            FIRST_FRAME_DT
        } else {
            dt_ms.min(MAX_FRAME_DT)
        };
        self.first_frame = false;

        let prev_scale = self.scale;
        self.scale += (self.target_scale - self.scale) * (1.0 - (-ZOOM_RATE * dt).exp());
        if (self.target_scale - self.scale).abs() < SCALE_EPSILON * self.target_scale {
            self.scale = self.target_scale;
        }
        // A plane unit is unscaledUnitSize / scale px wide, so this shift
        // keeps the plane point under the anchor at the same place on screen.
        // Screen y points down and plane y up.
        let d = (prev_scale - self.scale) / UNSCALED_UNIT_SIZE;
        self.center = [
            self.center[0] + self.anchor[0] * d,
            self.center[1] - self.anchor[1] * d,
        ];

        // The glide moves the plane after the zoom, at the scale drawn this
        // frame, so the plane point under the cursor moves with the glide.
        if let Some(glide) = self.glide {
            let decay = (-FRICTION * dt).exp();
            let glide = [glide[0] * decay, glide[1] * decay];
            if glide[0].hypot(glide[1]) < STOP_SPEED {
                self.glide = None;
            } else {
                self.glide = Some(glide);
                self.pan_by(glide[0] * dt, glide[1] * dt);
            }
        }

        if self.settled() {
            self.running = false;
            return false;
        }
        // A held pointer with nothing else moving: the next pointermove
        // starts the loop again.
        if self.glide.is_none() && self.target_scale == self.scale {
            self.running = false;
            return false;
        }
        true
    }
}

// The pointer's average velocity in px per ms over the flick window before
// `now`. A pointer held still sends no moves, so a rest before letting go
// lowers the speed, down to zero for a rest as long as the window.
fn release_velocity(samples: &mut VecDeque<Sample>, now: f64) -> [f64; 2] {
    trim_samples(samples, now);
    let (Some(first), Some(last)) = (samples.front(), samples.back()) else {
        return [0.0, 0.0];
    };
    if now <= first.time {
        return [0.0, 0.0];
    }
    let dt = now - first.time;
    [
        (last.at[0] - first.at[0]) / dt,
        (last.at[1] - first.at[1]) / dt,
    ]
}

/// `centerParam`'s serialiser: `toFixed(4)`, through `mv-numfmt`.
#[must_use]
pub fn format_center(center: [f64; 2]) -> String {
    format!("{},{}", to_fixed(center[0], 4), to_fixed(center[1], 4))
}

/// `scaleParam`'s serialiser: `String(Number(v.toPrecision(3)))`, three
/// significant digits at every scale, through `mv-numfmt`.
#[must_use]
pub fn format_scale(scale: f64) -> String {
    // The precision string always parses (digits with at most one `.` and
    // exponent), so this nests the two conversions as the TS does.
    let n: f64 = to_precision(scale, 3).parse().unwrap_or(f64::NAN);
    js_to_string(n)
}

/// JavaScript's `Number(text)`: surrounding whitespace ignored, empty text
/// is 0, and `0x`/`0b`/`0o` integers read in their radix. A signed or dotted
/// hex literal is `NaN` in JS and `None` here, as is a value beyond `u64`
/// (JS would round those to an `f64`).
fn js_number(text: &str) -> Option<f64> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Some(0.0);
    }
    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        return parse_int_digits(hex, 16);
    }
    if let Some(bin) = trimmed
        .strip_prefix("0b")
        .or_else(|| trimmed.strip_prefix("0B"))
    {
        return parse_int_digits(bin, 2);
    }
    if let Some(oct) = trimmed
        .strip_prefix("0o")
        .or_else(|| trimmed.strip_prefix("0O"))
    {
        return parse_int_digits(oct, 8);
    }
    trimmed.parse().ok()
}

fn parse_int_digits(digits: &str, radix: u32) -> Option<f64> {
    if digits.is_empty() || digits.starts_with(['+', '-']) {
        return None;
    }
    u64::from_str_radix(digits, radix).ok().map(|n| n as f64)
}

/// `centerParam`'s parser: `x,y`, both finite.
#[must_use]
pub fn parse_center(text: &str) -> Option<[f64; 2]> {
    let (x, y) = text.split_once(',')?;
    let x = js_number(x)?;
    let y = js_number(y)?;
    if x.is_finite() && y.is_finite() {
        Some([x, y])
    } else {
        None
    }
}

/// `scaleParam`'s parser: finite and inside the zoom's range.
#[must_use]
pub fn parse_scale(text: &str) -> Option<f64> {
    let n = js_number(text)?;
    if n.is_finite() && (MIN_SCALE..=MAX_SCALE).contains(&n) {
        Some(n)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The canvas sits at (100, 50) on the page and is 400 by 300 px, so the
    // point the draw code treats as its center is at (300, 200).
    const CANVAS_CENTER_X: f64 = 300.0;
    const CANVAS_CENTER_Y: f64 = 200.0;

    // Animation frames run only when a test calls run_frame, 16 ms apart.
    // Pointer events carry the same clock in their time, so a pointer that
    // moves between frames has a velocity in px per ms of test time.
    const FRAME_DT: f64 = 16.0;

    // The hook's writes to React, in order: the center write, then the scale
    // write, then the settle, exactly as its publish and onSettle order them.
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    enum Ev {
        Center,
        Scale,
        Settle,
    }

    struct Rig {
        cam: Camera,
        now: f64,
        told_center: [f64; 2],
        told_scale: f64,
        log: Vec<Ev>,
        center_hist: Vec<f64>,
        scale_hist: Vec<f64>,
    }

    impl Rig {
        fn new(center: [f64; 2], scale: f64) -> Self {
            Self {
                cam: Camera::new(center, scale),
                now: 0.0,
                told_center: center,
                told_scale: scale,
                log: Vec::new(),
                center_hist: Vec::new(),
                scale_hist: Vec::new(),
            }
        }

        fn offset(client_x: f64, client_y: f64) -> [f64; 2] {
            [client_x - CANVAS_CENTER_X, client_y - CANVAS_CENTER_Y]
        }

        fn wheel(&mut self, delta_y: f64) {
            self.wheel_at(delta_y, CANVAS_CENTER_X, CANVAS_CENTER_Y);
        }

        fn wheel_at(&mut self, delta_y: f64, x: f64, y: f64) {
            self.cam.wheel(0.0, delta_y, Self::offset(x, y));
        }

        fn down(&mut self, x: f64, y: f64) {
            self.cam
                .pointer_down(1, [x, y], self.now, PointerButton::Mouse(0));
        }

        fn mv(&mut self, x: f64, y: f64) {
            self.cam.pointer_move(1, [x, y], self.now);
        }

        // The release carries only the pointer's id and time; where it lets
        // go does not enter the hook.
        fn up(&mut self, x: f64, y: f64) {
            let _ = (x, y);
            self.cam.pointer_up(1, self.now);
        }

        // A rerender with new props: the camera adopts them while still.
        fn rerender(&mut self, center: [f64; 2], scale: f64) {
            self.cam.adopt_if_idle(center, scale);
            self.told_center = center;
            self.told_scale = scale;
        }

        fn run_frame(&mut self) {
            self.now += FRAME_DT;
            if !self.cam.animating() {
                return;
            }
            let still_going = self.cam.step(FRAME_DT);
            if self.cam.center != self.told_center {
                self.told_center = self.cam.center;
                self.log.push(Ev::Center);
                self.center_hist.push(self.told_center[0]);
            }
            if self.cam.scale != self.told_scale {
                self.told_scale = self.cam.scale;
                self.log.push(Ev::Scale);
                self.scale_hist.push(self.told_scale);
            }
            if !still_going && self.cam.settled() {
                self.log.push(Ev::Settle);
            }
        }

        fn run_until_still(&mut self) {
            for _ in 0..1000 {
                if !self.cam.animating() {
                    break;
                }
                self.run_frame();
            }
            assert!(!self.cam.animating());
        }

        // A flick to the right from the canvas center: the pointer moves 20
        // px a frame for five frames and lets go a frame later, fast enough
        // to glide.
        fn flick_right(&mut self) {
            self.down(CANVAS_CENTER_X, CANVAS_CENTER_Y);
            for i in 1..=5 {
                self.run_frame();
                self.mv(CANVAS_CENTER_X + 20.0 * f64::from(i), CANVAS_CENTER_Y);
            }
            self.run_frame();
            self.up(CANVAS_CENTER_X + 100.0, CANVAS_CENTER_Y);
        }

        fn latest(&self) -> ([f64; 2], f64) {
            (self.told_center, self.told_scale)
        }

        fn count(&self, ev: Ev) -> usize {
            self.log.iter().filter(|e| **e == ev).count()
        }

        fn clear_writes(&mut self) {
            self.log.clear();
            self.center_hist.clear();
            self.scale_hist.clear();
        }
    }

    // The plane point drawn at a page position, as drawCartesianPlane places it.
    fn plane_point_at(center: [f64; 2], scale: f64, x: f64, y: f64) -> [f64; 2] {
        let unit = UNSCALED_UNIT_SIZE / scale;
        [
            center[0] + (x - CANVAS_CENTER_X) / unit,
            center[1] - (y - CANVAS_CENTER_Y) / unit,
        ]
    }

    // The page position where a plane point is drawn.
    fn page_point_of(center: [f64; 2], scale: f64, p: [f64; 2]) -> [f64; 2] {
        let unit = UNSCALED_UNIT_SIZE / scale;
        [
            CANVAS_CENTER_X + (p[0] - center[0]) * unit,
            CANVAS_CENTER_Y - (p[1] - center[1]) * unit,
        ]
    }

    // vitest's toBeCloseTo with its digit count: within half a unit there.
    #[track_caller]
    fn assert_close(actual: f64, expected: f64, digits: i32) {
        let tol = 10f64.powi(-digits) / 2.0;
        assert!(
            (actual - expected).abs() < tol,
            "{actual} is not close to {expected} within {digits} digits"
        );
    }

    mod zoom {
        use super::*;

        #[test]
        fn zooms_by_the_product_of_a_burst_of_wheel_notches() {
            let mut r = Rig::new([0.0, 0.0], 1.0);

            // A fast wheel sends three notches before the next frame.
            r.wheel(100.0);
            r.wheel(100.0);
            r.wheel(-50.0);
            assert!(r.cam.animating());

            // The first frame eases part of the way, not the whole zoom at once.
            r.run_frame();
            let target = 0.1f64.exp() * 0.1f64.exp() * (-0.05f64).exp();
            let (_, scale) = r.latest();
            assert!(scale > 1.0 && scale < target);

            // The zoom comes to rest on the product of the three notches, on
            // one animation the whole way.
            for _ in 0..1000 {
                if !r.cam.animating() {
                    break;
                }
                assert!(r.cam.animating());
                r.run_frame();
            }
            assert!(!r.cam.animating());
            assert_close(r.latest().1, target, 12);
            assert!(r.count(Ev::Scale) > 0);
        }

        #[test]
        fn adds_up_trackpad_scroll_steps_too_small_to_zoom_on_their_own() {
            let mut r = Rig::new([0.0, 0.0], 1.0);

            // A slow trackpad scroll sends ten tiny steps before the next
            // frame. Each one moves the target less than the distance the
            // zoom snaps from.
            for _ in 0..10 {
                r.wheel(0.2);
            }

            // The zoom covers all ten, not only the last.
            r.run_until_still();
            assert_close(r.latest().1, 0.002f64.exp(), 12);
        }

        #[test]
        fn stops_at_the_scale_limits_without_starting_an_animation() {
            // Zoomed out near the limit, a big notch outward stops at scale 100.
            let mut zoomed_out = Rig::new([0.0, 0.0], 99.0);
            zoomed_out.wheel(1000.0);
            zoomed_out.run_until_still();
            assert_eq!(zoomed_out.latest().1, 100.0);

            // At scale 100, another notch outward has nothing to animate.
            let mut at_max = Rig::new([0.0, 0.0], 100.0);
            at_max.wheel(100.0);
            assert!(!at_max.cam.animating());
            assert_eq!(at_max.count(Ev::Scale), 0);

            // Likewise at scale 0.01 for a notch inward.
            let mut at_min = Rig::new([0.0, 0.0], 0.01);
            at_min.wheel(-100.0);
            assert!(!at_min.cam.animating());
            assert_eq!(at_min.count(Ev::Scale), 0);
        }

        #[test]
        fn ends_a_zoom_at_scale_0_02_without_a_jump_at_the_edge_of_the_plane() {
            // Zoomed far in, where a unit is 1250 px wide, a notch inward at
            // the canvas center eases toward scale 0.02 / e^0.1.
            let mut r = Rig::new([0.0, 0.0], 0.02);
            r.wheel(-100.0);
            r.run_until_still();
            assert_close(r.latest().1, 0.02 * (-0.1f64).exp(), 12);

            // Each frame moves a point at the canvas edge, 200 px from the
            // center, by 200 px times the scale's ratio to the frame before.
            // The last frame snaps the rest of the way, and that snap moves
            // it by under a px.
            let mut scales = vec![0.02];
            scales.extend_from_slice(&r.scale_hist);
            assert!(scales.len() > 5);
            let n = scales.len();
            let edge_shift = 200.0 * (scales[n - 2] / scales[n - 1] - 1.0).abs();
            assert!(edge_shift < 1.0);
        }

        #[test]
        fn keeps_the_plane_point_under_the_cursor_in_place_while_zooming() {
            let start = ([2.0, -1.0], 1.0);
            let mut r = Rig::new(start.0, start.1);

            // The cursor rests near the top right corner of the canvas and zooms in.
            let (x, y) = (450.0, 90.0);
            let under = plane_point_at(start.0, start.1, x, y);
            r.wheel_at(-300.0, x, y);

            // Every frame draws the same plane point under the cursor.
            while r.cam.animating() {
                r.run_frame();
                let (center, scale) = r.latest();
                let p = plane_point_at(center, scale, x, y);
                assert_close(p[0], under[0], 9);
                assert_close(p[1], under[1], 9);
            }

            // The zoom happened, and it panned the center toward the cursor.
            let (center, scale) = r.latest();
            assert_close(scale, (-0.3f64).exp(), 12);
            assert!(center[0] > 2.0);
            assert!(center[1] > -1.0);
        }

        #[test]
        fn zooms_without_panning_when_the_cursor_is_at_the_canvas_center() {
            let mut r = Rig::new([2.0, -1.0], 1.0);

            r.wheel(200.0);
            r.run_until_still();

            assert_close(r.latest().1, 0.2f64.exp(), 12);
            assert_eq!(r.count(Ev::Center), 0);
        }

        #[test]
        fn writes_the_url_once_when_the_zoom_comes_to_rest() {
            let mut r = Rig::new([0.0, 0.0], 1.0);

            // Two frames into a zoom, the scale has changed but the url has not.
            r.wheel_at(100.0, 350.0, 120.0);
            r.run_frame();
            r.run_frame();
            assert_eq!(r.count(Ev::Scale), 2);
            assert_eq!(r.count(Ev::Settle), 0);

            // Another notch mid-zoom carries the same zoom further.
            r.wheel_at(100.0, 350.0, 120.0);
            r.run_until_still();

            // One url write, after the last scale change.
            assert_eq!(r.count(Ev::Settle), 1);
            let last_scale = r.log.iter().rposition(|e| *e == Ev::Scale).unwrap();
            let settle = r.log.iter().rposition(|e| *e == Ev::Settle).unwrap();
            assert!(settle > last_scale);
        }

        #[test]
        fn zooms_from_where_back_or_forward_left_the_camera() {
            let mut r = Rig::new([0.0, 0.0], 1.0);

            // A zoom at the canvas center comes to rest, and React renders it.
            r.wheel(100.0);
            r.run_until_still();
            assert_close(r.latest().1, 0.1f64.exp(), 12);
            let rendered = r.latest();
            r.rerender(rendered.0, rendered.1);

            // Back goes to a url with center=3,4 and scale=2.
            let after_back = ([3.0, 4.0], 2.0);
            r.rerender(after_back.0, after_back.1);

            // The next zoom starts from scale 2 and keeps the point under the
            // cursor that url shows, not the one the first zoom left there.
            let (x, y) = (400.0, 200.0);
            let under = plane_point_at(after_back.0, after_back.1, x, y);
            r.wheel_at(100.0, x, y);
            r.run_until_still();
            let (center, scale) = r.latest();
            assert_close(scale, 2.0 * 0.1f64.exp(), 12);
            let p = plane_point_at(center, scale, x, y);
            assert_close(p[0], under[0], 9);
            assert_close(p[1], under[1], 9);
        }

        #[test]
        fn stops_zooming_when_the_plane_unmounts() {
            let mut r = Rig::new([0.0, 0.0], 1.0);

            // A zoom is one frame in when the plane goes away.
            r.wheel_at(100.0, 350.0, 120.0);
            r.run_frame();
            r.cam.stop();
            assert!(!r.cam.animating());
            r.clear_writes();

            // Nothing moves afterwards, and a late wheel event does nothing.
            r.run_frame();
            r.wheel_at(100.0, 350.0, 120.0);
            r.run_frame();
            assert!(!r.cam.animating());
            assert_eq!(r.count(Ev::Center), 0);
            assert_eq!(r.count(Ev::Scale), 0);
            assert_eq!(r.count(Ev::Settle), 0);
            assert!(!r.cam.step(FRAME_DT));
        }
    }

    mod pan_and_glide {
        use super::*;

        // The centers written to React, x only.
        fn center_xs(r: &Rig) -> Vec<f64> {
            r.center_hist.clone()
        }

        #[test]
        fn moves_the_plane_with_a_drag_one_write_per_frame_however_many_moves_land_in_it() {
            let mut r = Rig::new([0.0, 0.0], 1.0);

            // At scale 1 a plane unit is 25 px wide. Two moves land before the
            // next frame, together 25 px right and 10 px down.
            r.down(300.0, 200.0);
            r.mv(310.0, 200.0);
            r.mv(325.0, 210.0);
            assert_eq!(r.count(Ev::Center), 0);

            // The frame writes the center once: the plane followed the
            // pointer, so the center is one unit left and 0.4 up.
            r.run_frame();
            assert_eq!(r.count(Ev::Center), 1);
            let (center, _) = r.latest();
            assert_close(center[0], -1.0, 12);
            assert_close(center[1], 0.4, 12);

            // Holding the pointer still leaves no animation running.
            assert!(!r.cam.animating());
        }

        #[test]
        fn stops_without_a_glide_when_the_pointer_is_held_still_before_letting_go() {
            let mut r = Rig::new([0.0, 0.0], 1.0);

            // A drag at flick speed, 20 px a frame for five frames: 100 px
            // right, which is four units at 25 px a unit.
            r.down(300.0, 200.0);
            for i in 1..=5 {
                r.run_frame();
                r.mv(300.0 + 20.0 * f64::from(i), 200.0);
            }
            r.run_frame();
            assert_close(r.latest().0[0], -4.0, 12);

            // The pointer is held still for six frames, about 100 ms, then lets go.
            for _ in 0..6 {
                r.run_frame();
            }
            r.up(400.0, 200.0);

            // The plane stays where it was held, and the url is written.
            r.run_until_still();
            assert_close(r.latest().0[0], -4.0, 12);
            assert_eq!(r.count(Ev::Settle), 1);
        }

        #[test]
        fn stops_without_a_glide_when_the_mouse_twitches_as_it_lets_go_after_a_pause() {
            let mut r = Rig::new([0.0, 0.0], 1.0);

            // A drag at flick speed, 20 px a frame for five frames: 100 px
            // right, which is four units at 25 px a unit.
            r.down(300.0, 200.0);
            for i in 1..=5 {
                r.run_frame();
                r.mv(300.0 + 20.0 * f64::from(i), 200.0);
            }
            r.run_frame();
            assert_close(r.latest().0[0], -4.0, 12);

            // The pointer rests for about half a second, 30 frames.
            for _ in 0..30 {
                r.run_frame();
            }

            // Letting go, the mouse twitches 1 px and the button comes up right after.
            r.mv(401.0, 200.0);
            r.up(401.0, 200.0);

            // The plane follows the twitch and stops there, with the url written.
            r.run_until_still();
            assert_close(r.latest().0[0], -4.04, 12);
            assert_eq!(r.count(Ev::Settle), 1);
        }

        #[test]
        fn drags_at_the_scale_a_zoom_has_reached_and_never_jumps_back() {
            let mut r = Rig::new([0.0, 0.0], 1.0);

            // A zoom out at the canvas center, which leaves the center in
            // place, is two frames in.
            r.wheel(500.0);
            r.run_frame();
            r.run_frame();
            let mid_zoom = r.latest().1;
            assert!(mid_zoom > 1.0 && mid_zoom < 0.5f64.exp());

            // A 25 px drag right moves the plane by 25 px at the scale drawn
            // then: midZoom.scale units, not one unit as the props' scale 1
            // would give.
            r.down(300.0, 200.0);
            r.mv(325.0, 200.0);
            r.run_frame();
            assert_close(r.latest().0[0], -mid_zoom, 12);

            // The drag goes on and lets go while the zoom finishes.
            r.run_frame();
            r.mv(350.0, 200.0);
            r.run_frame();
            r.up(350.0, 200.0);
            r.run_until_still();

            // Every write moved the center further left, never back toward
            // the props, and the zoom reached its target.
            let xs = center_xs(&r);
            for w in xs.windows(2) {
                assert!(w[1] < w[0]);
            }
            assert_close(r.latest().1, 0.5f64.exp(), 12);
        }

        #[test]
        fn glides_and_zooms_together_on_one_animation_until_both_stop() {
            let mut r = Rig::new([0.0, 0.0], 1.0);

            // A wheel notch at the canvas center just after a flick lets go.
            r.flick_right();
            r.wheel(300.0);

            // One animation runs both, frame after frame, until both stop.
            while r.cam.animating() {
                r.run_frame();
            }
            assert!(!r.cam.animating());

            // The plane kept gliding right, never turning back, and the zoom
            // reached its target. One url write at the end.
            let xs = center_xs(&r);
            for w in xs.windows(2) {
                assert!(w[1] < w[0]);
            }
            let (center, scale) = r.latest();
            assert_eq!(center[1], 0.0);
            assert_close(scale, 0.3f64.exp(), 12);
            assert_eq!(r.count(Ev::Settle), 1);
        }

        #[test]
        fn zooms_about_the_cursor_during_a_glide_and_lets_the_glide_carry_on() {
            // Without a zoom, at scale 1 (25 px a unit), record how far the
            // glide moves the plane each frame.
            let mut glide_only = Rig::new([0.0, 0.0], 1.0);
            glide_only.flick_right();
            let mut glide_steps: Vec<f64> = Vec::new();
            while glide_only.cam.animating() {
                let before = glide_only.latest().0;
                glide_only.run_frame();
                glide_steps.push((before[0] - glide_only.latest().0[0]) * UNSCALED_UNIT_SIZE);
            }
            assert!(glide_steps.len() > 20);

            // The same flick, with a wheel notch near the top right corner
            // just after letting go.
            let mut r = Rig::new([0.0, 0.0], 1.0);
            r.flick_right();
            let (x, y) = (450.0, 90.0);
            r.wheel_at(-300.0, x, y);

            // Each frame, the plane point that was under the cursor moves
            // right by exactly the glide's step without a zoom: the zoom
            // holds it under the cursor and the glide carries it at its own
            // speed.
            let mut i = 0;
            while r.cam.animating() {
                let (center, scale) = r.latest();
                let under_cursor = plane_point_at(center, scale, x, y);
                r.run_frame();
                let (center, scale) = r.latest();
                let (sx, sy) = {
                    let p = page_point_of(center, scale, under_cursor);
                    (p[0], p[1])
                };
                assert_close(sx - x, glide_steps.get(i).copied().unwrap_or(0.0), 9);
                assert_close(sy, y, 9);
                i += 1;
            }
            assert_close(r.latest().1, (-0.3f64).exp(), 12);
        }

        #[test]
        fn stops_a_glide_when_the_plane_is_caught_and_lets_a_zoom_finish() {
            let mut r = Rig::new([0.0, 0.0], 1.0);

            // A wheel notch at the canvas center just after a flick lets go,
            // and two frames of both.
            r.flick_right();
            r.wheel(300.0);
            r.run_frame();
            r.run_frame();

            // The pointer goes down again without moving: the plane stays
            // where it was caught while the zoom goes on to its target.
            r.down(300.0, 200.0);
            let caught = r.latest().0;
            r.run_until_still();
            assert_eq!(r.latest().0, caught);
            assert_close(r.latest().1, 0.3f64.exp(), 12);

            // No url write while the pointer is down; letting go writes it.
            assert_eq!(r.count(Ev::Settle), 0);
            r.up(300.0, 200.0);
            r.run_until_still();
            assert_eq!(r.count(Ev::Settle), 1);
        }

        #[test]
        fn writes_the_url_once_after_a_drag_zoom_and_glide_have_all_stopped() {
            let mut r = Rig::new([0.0, 0.0], 1.0);

            // A drag starts, and a wheel notch mid-drag zooms all the way
            // while the pointer is still down.
            r.down(300.0, 200.0);
            r.run_frame();
            r.mv(320.0, 200.0);
            r.run_frame();
            r.wheel(200.0);
            r.run_until_still();
            assert_close(r.latest().1, 0.2f64.exp(), 12);
            assert_eq!(r.count(Ev::Settle), 0);

            // The drag goes on and lets go fast enough to glide.
            for i in 1..=3 {
                r.run_frame();
                r.mv(320.0 + 20.0 * f64::from(i), 200.0);
            }
            r.run_frame();
            r.up(380.0, 200.0);
            assert_eq!(r.count(Ev::Settle), 0);

            // One url write, after the glide's last move.
            r.run_until_still();
            assert_eq!(r.count(Ev::Settle), 1);
            let last_center = r.log.iter().rposition(|e| *e == Ev::Center).unwrap();
            let settle = r.log.iter().rposition(|e| *e == Ev::Settle).unwrap();
            assert!(settle > last_center);
        }

        #[test]
        fn stops_gliding_when_the_plane_unmounts() {
            let mut r = Rig::new([0.0, 0.0], 1.0);

            // A glide is one frame in when the plane goes away.
            r.flick_right();
            r.run_frame();
            r.cam.stop();
            assert!(!r.cam.animating());
            r.clear_writes();

            // Nothing moves afterwards, and a late drag does nothing.
            r.run_frame();
            r.down(300.0, 200.0);
            r.mv(340.0, 200.0);
            r.run_frame();
            assert!(!r.cam.animating());
            assert_eq!(r.count(Ev::Center), 0);
            assert_eq!(r.count(Ev::Settle), 0);
        }
    }

    mod with_url_params {
        use super::*;

        // The camera wired to the url params the way CartesianPlane wires it.
        // The params' debounce and history live in url.rs; these tests assert
        // every camera-side value: the formatted
        // strings the url ends up holding, the drawn scales' monotonicity,
        // and the write order the debounce interleaving rests on.

        #[test]
        fn keeps_zooming_smoothly_when_a_drag_lets_go_mid_zoom() {
            let mut r = Rig::new([1.0, 2.0], 1.0);

            // A drag 60 px right at scale 1, where a unit is 25 px wide.
            r.down(300.0, 200.0);
            for i in 1..=3 {
                r.run_frame();
                r.mv(300.0 + 20.0 * f64::from(i), 200.0);
            }
            r.run_frame();
            let (center, _) = r.latest();
            assert_close(center[0], -1.4, 12);
            assert_eq!(center[1], 2.0);

            // A wheel notch at the canvas center zooms out toward scale
            // e^0.5, and the pointer rests a moment and lets go while the
            // zoom runs.
            r.wheel(500.0);
            for _ in 0..6 {
                r.run_frame();
            }
            r.up(360.0, 200.0);

            // The zoom eases on. Its center stopped changing, so the center
            // reaches the url first: the last center write precedes the last
            // scale write, and only the merge's debounced url lines can show
            // the new center beside the old scale.
            r.run_until_still();
            let last_center = r.log.iter().rposition(|e| *e == Ev::Center).unwrap();
            let last_scale = r.log.iter().rposition(|e| *e == Ev::Scale).unwrap();
            assert!(last_center < last_scale);

            // The plane drew a growing scale every frame, never a step back to 1.
            let mut drawn = vec![1.0];
            drawn.extend_from_slice(&r.scale_hist);
            for w in drawn.windows(2) {
                assert!(w[1] >= w[0]);
            }

            // At rest, the url has both.
            let (center, scale) = r.latest();
            assert_close(scale, 0.5f64.exp(), 12);
            assert_eq!(format_center(center), "-1.4000,2.0000");
            assert_eq!(format_scale(scale), "1.65");
            assert_eq!(r.count(Ev::Settle), 1);
        }

        #[test]
        fn keeps_the_zoom_when_a_drag_lets_go_just_after_the_zoom_stops() {
            let mut r = Rig::new([1.0, 2.0], 1.0);

            // The pointer goes down and a wheel notch at the canvas center
            // zooms out toward scale e^0.5.
            r.down(300.0, 200.0);
            r.wheel(500.0);

            // A slow drag, 1 px a frame, goes on until the zoom reaches its
            // target.
            let mut x = 300.0;
            while r.cam.animating() {
                x += 1.0;
                r.mv(x, 200.0);
                r.run_frame();
            }
            // Center and scale changed every frame, so neither has reached
            // the url (the merge re-checks the debounced lines in url.rs).
            assert_eq!(r.latest().1, 0.5f64.exp());

            // The pointer lets go a frame later, while the scale still waits
            // for its url write. The camera comes to rest and writes both.
            r.up(x, 200.0);
            r.run_frame();
            assert!(!r.cam.animating());

            // The url keeps the new scale, and so does the plane.
            assert_eq!(r.latest().1, 0.5f64.exp());
            assert_eq!(r.latest().0[1], 2.0);
            assert_eq!(format_scale(r.latest().1), "1.65");
            assert_eq!(r.count(Ev::Settle), 1);
        }
    }

    mod params {
        use super::*;

        #[test]
        fn holds_the_query_params_defaults_and_limits() {
            assert_eq!(DEFAULT_CENTER, [0.0, 0.0]);
            assert_eq!(DEFAULT_SCALE, 0.5);
            assert_eq!(MIN_SCALE, 0.01);
            assert_eq!(MAX_SCALE, 100.0);
            assert_eq!(UNSCALED_UNIT_SIZE, 25.0);
        }

        #[test]
        fn formats_the_center_with_four_decimals() {
            assert_eq!(format_center([0.0, 0.0]), "0.0000,0.0000");
            assert_eq!(format_center([-1.4, 2.0]), "-1.4000,2.0000");
            assert_eq!(format_center([3.0, 4.0]), "3.0000,4.0000");
            assert_eq!(format_center([-0.0, 0.0]), "0.0000,0.0000");
            // A nonzero value that rounds to zero keeps its sign, as
            // `toFixed(4)` does (`(-0.00001).toFixed(4) === "-0.0000"`).
            assert_eq!(format_center([-0.00001, 0.00004]), "-0.0000,0.0000");
            // The binary value decides, not the decimal digits.
            assert_eq!(format_center([2.675, 1.0005]), "2.6750,1.0005");
            assert_eq!(format_center([2.6755, -2.6755]), "2.6755,-2.6755");
        }

        #[test]
        fn formats_the_scale_with_three_significant_digits() {
            assert_eq!(format_scale(1.0), "1");
            assert_eq!(format_scale(0.5f64.exp()), "1.65");
            assert_eq!(format_scale(2.0 * 0.1f64.exp()), "2.21");
            assert_eq!(format_scale(100.0), "100");
            assert_eq!(format_scale(0.01), "0.01");
            // The binary value decides: 2.675 is 2.6749999... underneath.
            assert_eq!(format_scale(2.675), "2.67");
        }

        #[test]
        fn parses_what_it_formats() {
            assert_eq!(parse_center("1.0000,2.0000"), Some([1.0, 2.0]));
            assert_eq!(parse_center("-1.4000,2.0000"), Some([-1.4, 2.0]));
            assert_eq!(parse_scale("1.00"), Some(1.0));
            assert_eq!(parse_scale("1.65"), Some(1.65));
            for (c, s) in [([3.0, 4.0], 2.0), ([-1.4, 2.0], 0.5f64.exp())] {
                let (pc, ps) = (
                    parse_center(&format_center(c)),
                    parse_scale(&format_scale(s)),
                );
                assert_close(pc.unwrap()[0], c[0], 4);
                assert_close(pc.unwrap()[1], c[1], 4);
                assert!((ps.unwrap() - s).abs() < 0.005 * s);
            }
        }

        #[test]
        fn rejects_what_the_params_reject() {
            assert_eq!(parse_center(""), None);
            assert_eq!(parse_center("1,2,3"), None);
            assert_eq!(parse_center("a,b"), None);
            assert_eq!(parse_center("Infinity,2"), None);
            // JavaScript's Number("") is 0, so "1," is the pair [1, 0].
            assert_eq!(parse_center("1,"), Some([1.0, 0.0]));
            // The radix literals JavaScript's Number reads.
            assert_eq!(parse_center("0x10,2"), Some([16.0, 2.0]));
            assert_eq!(parse_scale("0x10"), Some(16.0));
            assert_eq!(parse_scale("0b101"), Some(5.0));
            assert_eq!(parse_scale("0o17"), Some(15.0));
            assert_eq!(parse_scale("+0x10"), None);
            assert_eq!(parse_center("0x,2"), None);
            assert_eq!(parse_scale(""), None);
            assert_eq!(parse_scale("abc"), None);
            assert_eq!(parse_scale("0.001"), None);
            assert_eq!(parse_scale("1000"), None);
            assert_eq!(parse_scale("Infinity"), None);
            assert_eq!(parse_scale("0.01"), Some(0.01));
            assert_eq!(parse_scale("100"), Some(100.0));
        }
    }
}
