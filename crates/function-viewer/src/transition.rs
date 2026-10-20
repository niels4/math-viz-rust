//! The theme cross-fade: the previous frame over the new one.
//!
//! Ported from `src/util/viewTransition.ts` at `adb0534`: math-viz2 wraps
//! the theme change in `document.startViewTransition`, so the page
//! cross-fades to its new look. Measured on the reference (Chromium, seeded
//! dark/arctic-ice, click to tron-cyan): the `::view-transition-old(root)`
//! fade-out and `::view-transition-new(root)` fade-in both run 250 ms,
//! `linear`, starting in the first frame after the click; the pixel frames
//! stop changing by ~270 ms after the click. The port draws the new frame
//! and paints the snapshot taken at the last stable frame over it with
//! falling alpha over the same 250 ms, linear.

use crate::app::App;
use crate::app_loop::{MS, Time, clock_now};

/// The cross-fade's length: the browser's default view-transition duration,
/// measured on the reference.
pub const TRANSITION_MS: f64 = 250.0;

/// The cross-fade in flight: when the change happened on the app's clock.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ActiveTransition {
    /// The change's frame time, in nanoseconds.
    pub start: Time,
}

/// The transition state: the fade in flight, if any, with its image, and
/// the last stable frame's pixels, which the fade paints over the new frame.
#[derive(Clone, Debug, Default)]
pub struct Transition {
    pub active: Option<ActiveTransition>,
    /// The fade's uploaded image, or 0 before the first fading frame.
    pub image: i32,
    /// The last stable frame's backing pixels, bottom-up RGBA, at
    /// `snapshot_size`.
    pub snapshot: Vec<u8>,
    pub snapshot_size: [i32; 2],
}

impl Transition {
    /// The fade's progress at `now`: 0 old to 1 new, linear over 250 ms.
    pub fn progress(active: ActiveTransition, now: Time) -> f32 {
        ((now - active.start) as f64 / MS as f64 / TRANSITION_MS).clamp(0.0, 1.0) as f32
    }

    /// Whether the fade has run its 250 ms at `now`.
    pub fn is_done(active: ActiveTransition, now: Time) -> bool {
        now - active.start >= (TRANSITION_MS * MS as f64) as Time
    }
}

/// Starts the cross-fade on the app's clock and marks the frame dirty.
pub fn start(app: &mut App) {
    let now = clock_now(app);
    app.transition.active = Some(ActiveTransition { start: now });
    app.dirty = true;
}
