//! Ported from `src/motion/presence.odin` (113 lines): an overlay's presence on
//! its way in and out, and a restartable fade-in.

use crate::tokens::{
    ENTER_SETTLE_MS, Ease, FAST_MS, Level, OVERLAY_LEAVE_MS, REDUCED_FADE_MS, ease_at,
    overlay_leave_ms,
};
use crate::transition::{
    Transition, transition_at, transition_rest, transition_running, transition_to,
};

/// An overlay on its way in and out.
///
/// Ported from `presence.odin:15` (`Presence :: enum u8`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Presence {
    Closed,
    Open,
    Closing,
}

/// An overlay: its mount state and its opacity and shape transitions.
///
/// Ported from `presence.odin:21` (`Overlay :: struct { open, shown, since,
/// exit, alpha, shape }`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Overlay {
    pub open: bool,
    /// Mounted: open, or closing until `since + exit`.
    pub shown: bool,
    /// When it last opened or closed.
    pub since: f64,
    /// How long its exit lasts, s.
    pub exit: f64,
    /// Its opacity, 0 … 1.
    pub alpha: Transition,
    /// 0 at its starting transform, 1 in its place (a spring may pass it).
    pub shape: Transition,
}

/// The overlay opens or closes at `now`.
///
/// Ported from `presence.odin:31` (`overlay_set`; `^Overlay` is `&mut Overlay`).
pub fn overlay_set(o: &mut Overlay, open: bool, now: f64, level: Level) {
    if open == o.open {
        return;
    }
    let reduced = level == Level::Reduced;
    if open {
        if !o.shown || overlay_presence(*o, now) == Presence::Closed {
            // Mounted afresh: its transitions start from its starting style (@starting-style).
            o.alpha = transition_rest(0.0);
            o.shape = transition_rest(if reduced { 1.0 } else { 0.0 });
        }
        o.open = true;
        o.shown = true;
        o.since = now;
        o.exit = 0.0;
        if reduced {
            transition_to(
                &mut o.alpha,
                1.0,
                now,
                REDUCED_FADE_MS / 1000.0,
                Ease::Linear,
            );
            o.shape = transition_rest(1.0);
        } else {
            transition_to(&mut o.alpha, 1.0, now, FAST_MS / 1000.0, Ease::Linear);
            transition_to(
                &mut o.shape,
                1.0,
                now,
                ENTER_SETTLE_MS / 1000.0,
                Ease::CssEnter,
            );
        }
        return;
    }
    o.open = false;
    o.since = now;
    o.exit = overlay_leave_ms(level) / 1000.0;
    if reduced {
        transition_to(
            &mut o.alpha,
            0.0,
            now,
            REDUCED_FADE_MS / 1000.0,
            Ease::Linear,
        );
        o.shape = transition_rest(transition_at(o.shape, now));
    } else {
        transition_to(
            &mut o.alpha,
            0.0,
            now,
            OVERLAY_LEAVE_MS / 1000.0,
            Ease::Linear,
        );
        transition_to(
            &mut o.shape,
            0.0,
            now,
            OVERLAY_LEAVE_MS / 1000.0,
            Ease::Linear,
        );
    }
}

/// Whether it is open, closing, or closed at `now`.
///
/// Ported from `presence.odin:62` (`overlay_presence`).
pub fn overlay_presence(o: Overlay, now: f64) -> Presence {
    if o.open {
        return Presence::Open;
    }
    if o.shown && now - o.since < o.exit {
        return Presence::Closing;
    }
    Presence::Closed
}

/// Its opacity (0 … 1) and its shape's progress at `now`.
///
/// Ported from `presence.odin:73` (`overlay_look -> (alpha, shape)`; the
/// multi-return is a tuple).
pub fn overlay_look(o: Overlay, now: f64) -> (f64, f64) {
    (
        transition_at(o.alpha, now).clamp(0.0, 1.0),
        transition_at(o.shape, now),
    )
}

/// Whether its look still changes at `now`, or its exit still runs.
///
/// Ported from `presence.odin:78` (`overlay_moving`).
pub fn overlay_moving(o: Overlay, now: f64) -> bool {
    transition_running(o.alpha, now)
        || transition_running(o.shape, now)
        || overlay_presence(o, now) == Presence::Closing
}

/// A fade in from nothing each time what it shows changes.
///
/// Ported from `presence.odin:89` (`Fade_In :: struct { start, duration, ease }`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FadeIn {
    pub start: f64,
    /// s; 0 at rest.
    pub duration: f64,
    pub ease: Ease,
}

/// Starts the fade at `now`.
///
/// Ported from `presence.odin:96` (`fade_in_start`).
pub fn fade_in_start(f: &mut FadeIn, now: f64, duration_ms: f64, ease: Ease) {
    *f = FadeIn {
        start: now,
        duration: duration_ms / 1000.0,
        ease,
    };
}

/// Its opacity at `now`: 0 … 1, a spring's overshoot clamped as an opacity is.
///
/// Ported from `presence.odin:101` (`fade_in_at`).
pub fn fade_in_at(f: FadeIn, now: f64) -> f64 {
    if f.duration <= 0.0 || now >= f.start + f.duration {
        return 1.0;
    }
    if now <= f.start {
        return 0.0;
    }
    ease_at(f.ease, (now - f.start) / f.duration).clamp(0.0, 1.0)
}

/// Whether the fade still runs at `now`.
///
/// Ported from `presence.odin:111` (`fade_in_running`).
pub fn fade_in_running(f: FadeIn, now: f64) -> bool {
    f.duration > 0.0 && now < f.start + f.duration
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokens::FAST_MS;

    fn near(a: f64, b: f64, within: f64) -> bool {
        (a - b).abs() <= within
    }

    #[test]
    fn an_overlay_stays_closing_for_its_exit_then_closes() {
        let mut o = Overlay {
            open: false,
            shown: false,
            since: 0.0,
            exit: 0.0,
            alpha: transition_rest(0.0),
            shape: transition_rest(0.0),
        };
        assert_eq!(overlay_presence(o, 0.0), Presence::Closed);
        overlay_set(&mut o, true, 0.0, Level::Reduced);
        assert_eq!(overlay_presence(o, 0.0), Presence::Open);
        overlay_set(&mut o, false, 1.0, Level::Reduced);
        assert_eq!(overlay_presence(o, 1.0), Presence::Closing);
        assert_eq!(overlay_presence(o, 1.119), Presence::Closing);
        assert_eq!(overlay_presence(o, 1.12), Presence::Closed);
        // Full motion leaves in 200 ms.
        overlay_set(&mut o, true, 2.0, Level::Full);
        overlay_set(&mut o, false, 3.0, Level::Full);
        assert_eq!(overlay_presence(o, 3.199), Presence::Closing);
        assert_eq!(overlay_presence(o, 3.2), Presence::Closed);
    }

    #[test]
    fn an_overlay_opens_again_mid_exit() {
        let mut o = Overlay {
            open: false,
            shown: false,
            since: 0.0,
            exit: 0.0,
            alpha: transition_rest(0.0),
            shape: transition_rest(0.0),
        };
        overlay_set(&mut o, true, 0.0, Level::Reduced);
        overlay_set(&mut o, false, 1.0, Level::Reduced);
        overlay_set(&mut o, true, 1.04, Level::Reduced);
        assert_eq!(overlay_presence(o, 1.04), Presence::Open);
        assert_eq!(overlay_presence(o, 1.24), Presence::Open);
        // It fades back in from where its fade out had got to: 1 - 40/120, over the share it had gone.
        let (alpha, _) = overlay_look(o, 1.04);
        assert!(near(alpha, 2.0 / 3.0, 1e-12));
        assert!(near(o.alpha.duration, 0.04, 1e-12));
        let (alpha, _) = overlay_look(o, 1.08);
        assert_eq!(alpha, 1.0);
    }

    /// The settings panel's from-transform (translateY(-8px) scale(0.9), from its
    /// top right) at the shape's progress: its scale and its y.
    fn panel_at(shape: f64) -> (f64, f64) {
        (0.9 + 0.1 * shape, -8.0 * (1.0 - shape))
    }

    #[test]
    fn an_overlay_moves_as_the_settings_board_draws_it() {
        struct Frame {
            ms: f64,
            alpha: f64,
            scale: f64,
            y: f64,
        }
        let opening = [
            Frame {
                ms: 0.0,
                alpha: 0.0,
                scale: 0.90,
                y: -8.0,
            },
            Frame {
                ms: 40.0,
                alpha: 0.25,
                scale: 0.92,
                y: -6.77,
            },
            Frame {
                ms: 80.0,
                alpha: 0.50,
                scale: 0.94,
                y: -4.45,
            },
            Frame {
                ms: 120.0,
                alpha: 0.75,
                scale: 0.97,
                y: -2.30,
            },
            Frame {
                ms: 160.0,
                alpha: 1.0,
                scale: 0.99,
                y: -0.79,
            },
            Frame {
                ms: 240.0,
                alpha: 1.0,
                scale: 1.00,
                y: 0.38,
            },
            Frame {
                ms: 400.0,
                alpha: 1.0,
                scale: 1.00,
                y: 0.09,
            },
            Frame {
                ms: 643.0,
                alpha: 1.0,
                scale: 1.00,
                y: -0.01,
            },
        ];
        let mut o = Overlay {
            open: false,
            shown: false,
            since: 0.0,
            exit: 0.0,
            alpha: transition_rest(0.0),
            shape: transition_rest(0.0),
        };
        overlay_set(&mut o, true, 10.0, Level::Full);
        for f in &opening {
            let (alpha, shape) = overlay_look(o, 10.0 + f.ms / 1000.0);
            let (scale, y) = panel_at(shape);
            assert!(
                near(alpha, f.alpha, 0.0051),
                "{} ms: opacity {}",
                f.ms,
                alpha
            );
            assert!(near(scale, f.scale, 0.0051), "{} ms: scale {}", f.ms, scale);
            assert!(near(y, f.y, 0.011), "{} ms: y {}", f.ms, y);
        }
        assert!(overlay_moving(o, 10.642));
        assert!(!overlay_moving(o, 10.643));
        let closing = [
            Frame {
                ms: 0.0,
                alpha: 1.0,
                scale: 1.0,
                y: 0.0,
            },
            Frame {
                ms: 50.0,
                alpha: 0.75,
                scale: 0.98,
                y: -2.0,
            },
            Frame {
                ms: 100.0,
                alpha: 0.50,
                scale: 0.95,
                y: -4.0,
            },
            Frame {
                ms: 150.0,
                alpha: 0.25,
                scale: 0.93,
                y: -6.0,
            },
            Frame {
                ms: 200.0,
                alpha: 0.0,
                scale: 0.90,
                y: -8.0,
            },
        ];
        overlay_set(&mut o, false, 20.0, Level::Full);
        for f in &closing {
            let (alpha, shape) = overlay_look(o, 20.0 + f.ms / 1000.0);
            let (scale, y) = panel_at(shape);
            assert!(
                near(alpha, f.alpha, 1e-12),
                "{} ms: opacity {}",
                f.ms,
                alpha
            );
            assert!(near(scale, f.scale, 0.0051), "{} ms: scale {}", f.ms, scale);
            assert!(near(y, f.y, 1e-9), "{} ms: y {}", f.ms, y);
        }
        assert_eq!(overlay_presence(o, 20.201), Presence::Closed);
        // The list's new content as the scheme changes: 160 ms, linearly.
        let mut swap = FadeIn {
            start: 0.0,
            duration: 0.0,
            ease: Ease::Linear,
        };
        fade_in_start(&mut swap, 30.0, FAST_MS, Ease::Linear);
        for (i, ms) in [0.0, 40.0, 80.0, 120.0, 160.0].iter().enumerate() {
            assert!(near(
                fade_in_at(swap, 30.0 + ms / 1000.0),
                0.25 * i as f64,
                1e-12
            ));
        }
        assert!(!fade_in_running(swap, 30.16));
    }

    #[test]
    fn an_overlay_only_fades_under_reduced_motion() {
        let mut o = Overlay {
            open: false,
            shown: false,
            since: 0.0,
            exit: 0.0,
            alpha: transition_rest(0.0),
            shape: transition_rest(0.0),
        };
        overlay_set(&mut o, true, 0.0, Level::Reduced);
        let (alpha, shape) = overlay_look(o, 0.06);
        assert!(near(alpha, 0.5, 1e-12));
        assert_eq!(shape, 1.0);
        assert!(!overlay_moving(o, 0.12));
        overlay_set(&mut o, false, 1.0, Level::Reduced);
        let (alpha, shape) = overlay_look(o, 1.03);
        assert!(near(alpha, 0.75, 1e-12));
        assert_eq!(shape, 1.0);
    }
}
