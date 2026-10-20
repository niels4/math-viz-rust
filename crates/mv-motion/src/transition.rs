//! Ported from `src/motion/transition.odin` (95 lines): one CSS transition as
//! the browsers run it, including the reversing shortening.

use crate::tokens::{Ease, ease_at};

/// A CSS transition of one number. Times in seconds.
///
/// Ported from `transition.odin:9` (`Transition :: struct { from, to, start,
/// duration, ease, reversing_from, shortening }`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transition {
    pub from: f64,
    pub to: f64,
    pub start: f64,
    pub duration: f64,
    pub ease: Ease,
    /// The reversing-adjusted start value: where a reversal goes back to.
    pub reversing_from: f64,
    /// The reversing shortening factor, 0 … 1.
    pub shortening: f64,
}

/// A value at rest at `value`: no transition runs.
///
/// Ported from `transition.odin:19` (`transition_rest`).
pub fn transition_rest(value: f64) -> Transition {
    Transition {
        from: value,
        to: value,
        start: 0.0,
        duration: 0.0,
        ease: Ease::Linear,
        reversing_from: value,
        shortening: 1.0,
    }
}

/// The output progress of `t` at `now`: 0 before it starts, 1 once it is over.
///
/// Ported from `transition.odin:26` (`output_at`, file-private).
fn output_at(t: Transition, now: f64) -> f64 {
    if t.duration <= 0.0 || now >= t.start + t.duration {
        return 1.0;
    }
    if now <= t.start {
        return 0.0;
    }
    ease_at(t.ease, (now - t.start) / t.duration)
}

/// The value at `now`.
///
/// Ported from `transition.odin:37` (`transition_at`).
pub fn transition_at(t: Transition, now: f64) -> f64 {
    t.from + (t.to - t.from) * output_at(t, now)
}

/// Whether it still moves at `now`.
///
/// Ported from `transition.odin:42` (`transition_running`).
pub fn transition_running(t: Transition, now: f64) -> bool {
    t.from != t.to && t.duration > 0.0 && now < t.start + t.duration
}

/// The property's value changed to `to` at `now`: the transition the browsers
/// start then. Changed to where it is already going, it goes on.
///
/// Ported from `transition.odin:49` (`transition_to`; the `^Transition`
/// in-place mutation is `&mut Transition`).
pub fn transition_to(t: &mut Transition, to: f64, now: f64, duration: f64, ease: Ease) {
    if to == t.to {
        return;
    }
    let current = transition_at(*t, now);
    let running = transition_running(*t, now);
    if !running {
        *t = Transition {
            from: current,
            to,
            start: now,
            duration,
            ease,
            reversing_from: current,
            shortening: 1.0,
        };
    } else if current == to || duration <= 0.0 {
        // Cancelled: it rests at its new value.
        *t = transition_rest(to);
    } else if t.reversing_from == to {
        // Back to where it came from: shortened by the share of the way it had come.
        let factor = (output_at(*t, now) * t.shortening + (1.0 - t.shortening))
            .abs()
            .clamp(0.0, 1.0);
        *t = Transition {
            from: current,
            to,
            start: now,
            duration: duration * factor,
            ease,
            reversing_from: t.to,
            shortening: factor,
        };
    } else {
        *t = Transition {
            from: current,
            to,
            start: now,
            duration,
            ease,
            reversing_from: current,
            shortening: 1.0,
        };
    }
    if t.duration <= 0.0 {
        *t = transition_rest(to);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokens::Ease;

    fn near(a: f64, b: f64, within: f64) -> bool {
        (a - b).abs() <= within
    }

    #[test]
    fn a_transition_runs_from_where_it_is_to_its_new_value() {
        let mut tr = transition_rest(0.0);
        assert!(!transition_running(tr, 0.0));
        transition_to(&mut tr, 1.0, 0.0, 0.16, Ease::Linear);
        assert!(near(transition_at(tr, 0.08), 0.5, 1e-12));
        assert!(transition_running(tr, 0.1));
        assert_eq!(transition_at(tr, 0.16), 1.0);
        assert!(!transition_running(tr, 0.16));
        // Told to go where it is already going, it goes on.
        transition_to(&mut tr, 1.0, 0.08, 0.5, Ease::Linear);
        assert_eq!(tr.duration, 0.16);
        // A third value, mid-way: from where it is, the whole duration.
        tr = transition_rest(0.0);
        transition_to(&mut tr, 1.0, 0.0, 0.16, Ease::Linear);
        transition_to(&mut tr, 2.0, 0.08, 0.2, Ease::Linear);
        assert!(near(tr.from, 0.5, 1e-12));
        assert_eq!(tr.duration, 0.2);
        assert!(near(transition_at(tr, 0.18), 1.25, 1e-12));
    }

    #[test]
    fn a_reversed_transition_is_shortened() {
        let mut tr = transition_rest(0.0);
        transition_to(&mut tr, 1.0, 0.0, 0.16, Ease::Linear);
        transition_to(&mut tr, 0.0, 0.08, 0.2, Ease::Linear);
        assert!(near(tr.duration, 0.1, 1e-12));
        assert!(near(transition_at(tr, 0.13), 0.25, 1e-12));
        assert_eq!(transition_at(tr, 0.18), 0.0);
        // Reversed again at 0.13: the factor is the way it came back times the last factor, plus what
        // the last one had cut: 0.5 · 0.5 + 0.5.
        transition_to(&mut tr, 1.0, 0.13, 0.16, Ease::Linear);
        assert!(near(tr.shortening, 0.75, 1e-12));
        assert!(near(tr.duration, 0.12, 1e-12));
        assert!(near(transition_at(tr, 0.19), 0.625, 1e-12));
        // A spring's output past 1 makes a whole reversal, not more.
        tr = transition_rest(0.0);
        transition_to(&mut tr, 1.0, 0.0, 0.643, Ease::CssEnter);
        assert!(transition_at(tr, 0.27) > 1.0);
        transition_to(&mut tr, 0.0, 0.27, 0.2, Ease::Linear);
        assert_eq!(tr.shortening, 1.0);
        assert_eq!(tr.duration, 0.2);
    }
}
