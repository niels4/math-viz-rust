//! How far a transform drag moves f(P): the moved badge's state machine.
//!
//! Ported from
//! `src/components/views/FunctionViewer/SideBar/PointsSection/useMovedBy.ts`
//! at `96f1e84`. The hook's render-time `seen`/`before` states become struct
//! fields fed `(dragging, x, y)` each frame — `dragging` being any transform
//! scrub mode set (`transformDragAtom`) — with the 600 ms linger as a
//! timestamp on the app's ms clock (`(app.input.time / MS) as u64`, as
//! `scrub_value.rs` reads it).

/// How long the badge lingers after the drag's release (`LINGER_MS`), in ms.
pub const LINGER_MS: u64 = 600;

/// A y move past this counts as moved; at or below it the curve sat still.
const MOVE_EPS: f64 = 1e-9;

/// P's position when a drag started: the last frame's x and y, and whether
/// f(P) has moved since.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Before {
    x: f64,
    y: f64,
    moved: bool,
}

/// The moved badge's state (`useMovedBy` as owned state): the last frame's
/// inputs, P's drag-start position while one is held or lingering, and when
/// the linger started.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MovedBy {
    seen_dragging: bool,
    seen_x: f64,
    seen_y: f64,
    before: Option<Before>,
    linger_start_ms: Option<u64>,
}

impl MovedBy {
    /// An idle badge: no drag seen, nothing held, nothing lingering.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Feeds one frame's `(dragging, x, y)` at `now_ms` on the app's ms clock.
    ///
    /// On any input change: a drag's start takes the last frame's x and y
    /// (the drag's first change can land in the same frame as its start);
    /// `before` is dropped when x changed, or when not dragging and y changed
    /// since the last frame; while dragging, the first y change past
    /// [`MOVE_EPS`] marks it moved. After the drag, `before` lingers
    /// [`LINGER_MS`] ms, then clears; the linger starts at the release and is
    /// not restarted while it runs, as the source's `[lingering]` effect.
    pub fn update(&mut self, dragging: bool, x: f64, y: f64, now_ms: u64) {
        if self.seen_dragging != dragging || self.seen_x != x || self.seen_y != y {
            let was = (self.seen_dragging, self.seen_x, self.seen_y);
            self.seen_dragging = dragging;
            self.seen_x = x;
            self.seen_y = y;
            let mut next = if dragging && !was.0 {
                Some(Before {
                    x: was.1,
                    y: was.2,
                    moved: false,
                })
            } else {
                self.before
            };
            if let Some(slot) = next.as_mut() {
                if x != slot.x || (!dragging && y != was.2) {
                    next = None;
                } else if dragging && !slot.moved && (y - slot.y).abs() > MOVE_EPS {
                    slot.moved = true;
                }
            }
            if next != self.before {
                self.before = next;
            }
        }
        if !dragging && self.before.is_some() {
            match self.linger_start_ms {
                None => self.linger_start_ms = Some(now_ms),
                Some(start) if now_ms.wrapping_sub(start) >= LINGER_MS => {
                    self.before = None;
                    self.linger_start_ms = None;
                }
                _ => {}
            }
        } else {
            self.linger_start_ms = None;
        }
    }

    /// How far the drag moved f(P): `y − before.y` while `before` is marked
    /// moved, else `None`. The caller prints it with
    /// [`crate::point_format::format_change`].
    #[must_use]
    pub fn moved(&self) -> Option<f64> {
        match self.before {
            Some(before) if before.moved => Some(self.seen_y - before.y),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::point_format::format_change;

    /// PRow's "says how far a transform's drag moves f(P), until 600 ms after
    /// letting go", as pure values: a coarse k drag from 0 to 1 at p = 2 on
    /// x1 (y 2 → 3) reads `+1.00`, on to −0.5 (y 1.5) reads `−0.50`, present
    /// 599 ms after the release and gone at 600.
    #[test]
    fn moved_badge_tracks_a_coarse_drag_until_600ms_after_release() {
        let mut badge = MovedBy::new();
        badge.update(false, 2.0, 2.0, 0);
        assert_eq!(badge.moved(), None);

        badge.update(true, 2.0, 2.0, 100);
        assert_eq!(badge.moved(), None);

        badge.update(true, 2.0, 3.0, 200);
        assert_eq!(badge.moved(), Some(1.0));
        assert_eq!(format_change(badge.moved().unwrap_or(0.0)), "+1.00");

        badge.update(true, 2.0, 1.5, 300);
        assert_eq!(badge.moved(), Some(-0.5));
        assert_eq!(format_change(badge.moved().unwrap_or(0.0)), "−0.50");

        badge.update(false, 2.0, 1.5, 1000);
        assert_eq!(badge.moved(), Some(-0.5));
        badge.update(false, 2.0, 1.5, 1599);
        assert_eq!(badge.moved(), Some(-0.5));
        badge.update(false, 2.0, 1.5, 1600);
        assert_eq!(badge.moved(), None);
    }

    /// PRow's "stays away while a drag leaves f(P) where it was": an h drag
    /// with x and y unchanged never marks the badge.
    #[test]
    fn moved_badge_stays_away_while_a_drag_leaves_f_p_where_it_was() {
        let mut badge = MovedBy::new();
        badge.update(false, 2.0, 2.0, 0);
        badge.update(true, 2.0, 2.0, 100);
        assert_eq!(badge.moved(), None);
        badge.update(true, 2.0, 2.0, 200);
        assert_eq!(badge.moved(), None);
        badge.update(false, 2.0, 2.0, 300);
        assert_eq!(badge.moved(), None);
    }

    /// PRow's "goes at once when something other than a drag changes the
    /// curve": a same-frame drag start and y change marks the badge, and a y
    /// change outside the drag clears it at once rather than lingering.
    #[test]
    fn moved_badge_goes_at_once_when_a_non_drag_changes_the_curve() {
        let mut badge = MovedBy::new();
        badge.update(false, 2.0, 2.0, 0);
        // The drag's first change lands in the same frame as its start, so
        // P's start is the last frame's (2, 2): moved −1.00.
        badge.update(true, 2.0, 1.0, 100);
        assert_eq!(badge.moved(), Some(-1.0));

        badge.update(false, 2.0, 1.0, 200);
        assert_eq!(badge.moved(), Some(-1.0));

        badge.update(false, 2.0, 3.0, 300);
        assert_eq!(badge.moved(), None);
    }
}
