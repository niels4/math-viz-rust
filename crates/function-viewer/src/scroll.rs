//! Scroll containers (`Scroller`, `SCROLLER_IDS`, `Scroll`, `scrolled`,
//! `scroll_container`, `update_scroll`, `snap_px`, `scroll_wheel`,
//! `scroll_into_view`, `scroll_target`, `grow_box`, `inside`).
//!
//! The math and region matching are pure. [`update_scroll`] replays Clay's
//! scroll-container data through `mv-clay`'s free element functions.

use mv_sys::clay;

use crate::app::App;
use crate::input::Region;

/// The page's scroll containers: the panel, and in the stacked layout the
/// page itself. Ported from `Scroller`; order matches the Odin declaration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, enum_map::Enum)]
#[repr(u8)]
pub enum Scroller {
    /// The whole page scrolls.
    Page,
    /// The side panel scrolls.
    Panel,
}

/// Clay element ids of the page and panel scroll containers. Ported from
/// `SCROLLER_IDS`.
pub const SCROLLER_IDS: enum_map::EnumMap<Scroller, &'static str> =
    enum_map::EnumMap::from_array(["Page", "Panel"]);

/// A scroll container as the last layout left it. Ported from `Scroll`.
#[derive(Clone, Copy, Debug)]
pub struct Scroll {
    /// Whether it scrolls in the layout shown.
    pub active: bool,
    /// Its scroll offset in pixels.
    pub offset: f32,
    /// The furthest it goes: its content's height less its own.
    pub max: f32,
    /// What of the window it shows.
    pub r#box: clay::BoundingBox,
    /// Its border's pixels, inside its box.
    pub border: f32,
}

/// The scroll a container's children take this frame. Ported from `scrolled`.
pub fn scrolled(app: &App, s: Scroller) -> [f32; 2] {
    if app.scroll[s].active {
        [0.0, -app.scroll[s].offset]
    } else {
        [0.0, 0.0]
    }
}

/// Marks which containers scroll in this layout, as the layout declares
/// them. Ported from `scroll_container`.
pub fn scroll_container(app: &mut App, s: Scroller, active: bool, border: f32) {
    app.scroll[s].active = active;
    app.scroll[s].border = border;
}

/// Replays the layout to learn each container's extent and keep its offset
/// within it. True when an offset moved.
///
/// Ported from `update_scroll`: the data comes from Clay's scroll-container
/// and element queries for the scroller's id, through `mv-clay`'s free
/// functions (the layout the last `layout` call left is still current).
pub fn update_scroll(app: &mut App) -> bool {
    let mut moved = false;
    for s in [Scroller::Page, Scroller::Panel] {
        let sc = &mut app.scroll[s];
        if !sc.active {
            sc.max = 0.0;
        } else {
            let data = mv_clay::scroll_container_data(mv_clay::id(SCROLLER_IDS[s], 0));
            if data.found {
                let r#box = mv_clay::element_data(mv_clay::id(SCROLLER_IDS[s], 0));
                if r#box.found {
                    sc.r#box = r#box.boundingBox;
                }
                // Clay's padding takes the border's px (Clay draws a border
                // without laying it out).
                let content = data.contentDimensions.height - 2.0 * sc.border;
                let client = sc.r#box.height - 2.0 * sc.border;
                sc.max = (snap_px(content) - client).max(0.0);
            }
        }
        let clamped = clamp(sc.offset, 0.0, sc.max);
        if clamped != sc.offset {
            sc.offset = clamped;
            moved = true;
        }
    }
    moved
}

/// Whole pixels, halves up, as a browser snaps a scroll extent. Ported from
/// `snap_px`.
fn snap_px(v: f32) -> f32 {
    (v + 0.5) as i64 as f32
}

/// The wheel over `pos`, `delta` the page's pixels (+y down): the innermost
/// container under the pointer that can go that way takes it. True when
/// something moved.
pub fn scroll_wheel(app: &mut App, pos: [f32; 2], delta: [f32; 2]) -> bool {
    for s in [Scroller::Panel, Scroller::Page] {
        let sc = &mut app.scroll[s];
        if !sc.active || !inside(sc.r#box, pos) {
            continue;
        }
        let to = sc.offset + delta[1];
        let to = if to < 0.0 {
            0.0
        } else if to > sc.max {
            sc.max
        } else {
            to
        };
        if to != sc.offset {
            sc.offset = to;
            app.dirty = true;
            return true;
        }
    }
    false
}

/// Brings the focused part into view in its scroll container, as WebKit and
/// Chromium scroll a focused element. True when the offset moved. Ported from
/// `scroll_into_view`.
pub fn scroll_into_view(app: &mut App, r: Region) -> bool {
    let s = if r.clip.is_some() {
        Scroller::Panel
    } else if app.scroll[Scroller::Page].active {
        Scroller::Page
    } else {
        return false;
    };
    let sc = &mut app.scroll[s];
    if !sc.active {
        return false;
    }
    let view = grow_box(sc.r#box, -sc.border);
    let to = scroll_target(
        sc.offset,
        sc.max,
        [view.y, view.y + view.height],
        [r.r#box.y, r.r#box.y + r.r#box.height],
    );
    if to == sc.offset {
        return false;
    }
    sc.offset = to;
    app.dirty = true;
    true
}

/// Where a container scrolled `offset` goes to show a part with top and
/// bottom `part`, the window it shows being `view`. Ported from
/// `scroll_target`.
pub fn scroll_target(offset: f32, max: f32, view: [f32; 2], part: [f32; 2]) -> f32 {
    let (top, bottom) = (part[0], part[1]);
    let (lo, hi) = (view[0], view[1]);
    if top >= lo && bottom <= hi {
        return offset;
    }
    let delta = if bottom <= lo || top >= hi {
        (top + bottom) / 2.0 - (lo + hi) / 2.0
    } else if top < lo || bottom - top > hi - lo {
        top - lo
    } else {
        bottom - hi
    };
    clamp((offset + delta + 0.5).floor(), 0.0, max)
}

/// Whether `pos` falls inside `box`.
pub fn inside(b: clay::BoundingBox, pos: [f32; 2]) -> bool {
    pos[0] >= b.x && pos[0] < b.x + b.width && pos[1] >= b.y && pos[1] < b.y + b.height
}

/// Grows a box by `by` on every side. Moved here from the deleted
/// `effects.rs`: the scroll-container clip is this geometry, and this is its
/// only kept user.
pub fn grow_box(b: clay::BoundingBox, by: f32) -> clay::BoundingBox {
    clay::BoundingBox {
        x: b.x - by,
        y: b.y - by,
        width: b.width + 2.0 * by,
        height: b.height + 2.0 * by,
    }
}

fn clamp(v: f32, lo: f32, hi: f32) -> f32 {
    if v < lo {
        lo
    } else if v > hi {
        hi
    } else {
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_leaves_a_shown_part_alone() {
        assert_eq!(scroll_target(20.0, 100.0, [0.0, 50.0], [10.0, 20.0]), 20.0);
        assert_eq!(scroll_target(10.0, 100.0, [0.0, 50.0], [15.0, 30.0]), 10.0);
    }

    #[test]
    fn target_centres_an_unseen_part() {
        assert_eq!(scroll_target(0.0, 100.0, [0.0, 50.0], [60.0, 70.0]), 40.0);
    }

    #[test]
    fn target_clamps_to_reach() {
        assert_eq!(scroll_target(0.0, 30.0, [0.0, 50.0], [100.0, 120.0]), 30.0);
        assert_eq!(scroll_target(10.0, 100.0, [0.0, 50.0], [-20.0, -10.0]), 0.0);
    }

    #[test]
    fn target_moves_a_taller_part_by_its_top() {
        assert_eq!(scroll_target(0.0, 100.0, [0.0, 50.0], [0.0, 100.0]), 0.0);
        assert_eq!(scroll_target(0.0, 100.0, [0.0, 50.0], [10.0, 100.0]), 10.0);
    }

    #[test]
    fn target_floors_before_clamping_on_fractional_max() {
        // Worked from the Odin formula: the part is wholly past the view, so
        // delta centres it: (415 + 435) / 2 - (0 + 50) / 2 = 400, and
        // floor(0 + 400 + 0.5) = 400 clamps to the fractional max 30.5.
        // Clamping first would floor the clamped 30.5 down to 30.0.
        assert_eq!(scroll_target(0.0, 30.5, [0.0, 50.0], [415.0, 435.0]), 30.5);
    }

    #[test]
    fn snap_px_rounds_halves_up() {
        assert_eq!(snap_px(718.5), 719.0);
        assert_eq!(snap_px(109.4), 109.0);
    }

    #[test]
    fn inside_covers_left_top_edges_only() {
        let b = clay::BoundingBox {
            x: 10.0,
            y: 20.0,
            width: 30.0,
            height: 40.0,
        };
        assert!(inside(b, [10.0, 20.0]));
        assert!(inside(b, [39.9, 59.9]));
        assert!(!inside(b, [40.0, 20.0]));
        assert!(!inside(b, [10.0, 60.0]));
    }
}
