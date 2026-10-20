//! Layer, shadow and inset effects ported from `effects.odin` (`Effect`,
//! `effect`, `Layer`, `LAYER_REST`, `layer_config`, `RING_WIDTH`,
//! `spread_radius`, `spread_radii`, `grow_box`, `draw_ring`, `draw_inset`,
//! `draw_shadow`).
//!
//! The data types and geometry are pure. The draw helpers call the `t0c`
//! NanoVG and Clay bindings, and `effect` boxes an [`Effect`] for the frame
//! like Odin's temp-allocator clone.

use std::ffi::c_void;

use mv_nvg::Context;
use mv_sys::clay;
use mv_theme::{Color, Shadow};

use crate::draw::{inset_radius, rounded_rect};

/// What CSS's box-shadow draws around and inside the page's boxes, which Clay
/// has no word for. Ported from `Effect`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Effect {
    /// The shadow under the element, outside its box.
    pub shadow: Option<Shadow>,
    /// Pixels of stroke inside the box's edge, 0 for none.
    pub inset: f32,
    /// Colour of the inset stroke.
    pub inset_ink: Color,
    /// The subtree drawn as one layer, faded, scaled and moved.
    pub layer: Option<Layer>,
}

/// No effect: no shadow, no inset, no layer.
pub const EFFECT_NONE: Effect = Effect {
    shadow: None,
    inset: 0.0,
    inset_ink: [0.0; 4],
    layer: None,
};

/// Boxes an element's effects for its declaration's user data, valid until
/// the frame ends. Ported from `effect`.
///
/// The payload moves into a frame-lived store and the pointer aims at the
/// stored box, as Odin temp-allocates a clone the same way. `draw_frame`
/// frees the store before the next layout, so a pointer handed to Clay stays
/// valid until paint. Each payload is separately boxed, so pushing more never
/// moves a payload an earlier pointer aims at. Mirrors `widgets::custom_data`.
pub fn effect(e: Effect) -> *mut c_void {
    EFFECTS.with(|store| {
        let mut store = store.borrow_mut();
        store.push(Box::new(e));
        let last = store.last().expect("payload just pushed");
        &**last as *const Effect as *mut Effect as *mut c_void
    })
}

// The frame-lived Effect payloads `effect` keeps, freed by `draw_frame`
// before the next layout.
std::thread_local! {
    // Each payload is separately boxed so pushing more never moves a payload
    // an earlier pointer aims at; the lint's plain `Vec` would reallocate.
    #[allow(clippy::vec_box)]
    static EFFECTS: std::cell::RefCell<Vec<Box<Effect>>> = const {
        std::cell::RefCell::new(Vec::new())
    };
}

/// Frees the frame-lived Effect payloads before the next layout.
///
/// `draw_frame` calls this beside `widgets::clear_customs` (its file belongs
/// to render-widgets, so the call itself is that milestone's line to add).
pub(crate) fn clear_effects() {
    EFFECTS.with(|store| store.borrow_mut().clear());
}

/// CSS's opacity and transform on an element: its subtree drawn apart and
/// composited as one. Ported from `Layer`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layer {
    /// Opacity multiplier for the subtree.
    pub alpha: f32,
    /// Uniform scale about `origin`.
    pub scale: f32,
    /// Pixels from the element's own top left scaled about.
    pub origin: [f32; 2],
    /// Pixels the subtree moves by.
    pub offset: [f32; 2],
}

/// A layer at rest: opaque, unscaled, unmoved. Ported from `LAYER_REST`.
pub const LAYER_REST: Layer = Layer {
    alpha: 1.0,
    scale: 1.0,
    origin: [0.0; 2],
    offset: [0.0; 2],
};

/// What an element declares for a layer: its overlay colour and its user
/// data. At rest no bracket: the subtree draws as any other. Ported from
/// `layer_config`.
pub fn layer_config(l: Layer, e: Effect) -> (clay::Color, *mut c_void) {
    if l.alpha >= 1.0 && l.scale == 1.0 && l.offset == [0.0, 0.0] {
        if e == EFFECT_NONE {
            return ([0.0; 4], std::ptr::null_mut());
        }
        return ([0.0; 4], effect(e));
    }
    (
        [0.0, 0.0, 0.0, 1.0],
        effect(Effect {
            layer: Some(l),
            ..e
        }),
    )
}

/// The keyboard's focus ring: 3 px outside the box. Ported from `RING_WIDTH`.
pub const RING_WIDTH: f32 = 3.0;

/// A box-shadow's corner radius once spread by `spread` pixels: a corner of
/// radius r >= spread grows by the spread; a sharper one by less, so a square
/// corner stays square. Ported from `spread_radius`.
pub fn spread_radius(r: f32, spread: f32) -> f32 {
    if r <= 0.0 {
        return 0.0;
    }
    if r >= spread {
        return r + spread;
    }
    let ratio = r / spread - 1.0;
    r + spread * (1.0 + ratio * ratio * ratio)
}

/// A box-shadow's corner radii once spread by `spread` pixels. Ported from
/// `spread_radii`.
pub fn spread_radii(r: clay::CornerRadius, spread: f32) -> clay::CornerRadius {
    clay::CornerRadius {
        topLeft: spread_radius(r.topLeft, spread),
        topRight: spread_radius(r.topRight, spread),
        bottomLeft: spread_radius(r.bottomLeft, spread),
        bottomRight: spread_radius(r.bottomRight, spread),
    }
}

/// Grows a box by `by` on every side. Ported from `grow_box`.
pub fn grow_box(b: clay::BoundingBox, by: f32) -> clay::BoundingBox {
    clay::BoundingBox {
        x: b.x - by,
        y: b.y - by,
        width: b.width + 2.0 * by,
        height: b.height + 2.0 * by,
    }
}

/// A ring of `width` pixels outside `box`: the area between the two rounded
/// rectangles. Ported from `draw_ring`; needs the `t0c` NanoVG path bindings.
pub fn draw_ring(
    vg: &mut Context,
    b: clay::BoundingBox,
    radius: clay::CornerRadius,
    width: f32,
    ink: Color,
) {
    vg.begin_path();
    rounded_rect(vg, grow_box(b, width), spread_radii(radius, width));
    rounded_rect(vg, b, radius);
    vg.path_winding(mv_nvg::Winding::CW);
    vg.fill_color(ink);
    vg.fill();
}

/// A stroke of `width` pixels inside the box's edge: the area between the box
/// and the box shrunk by the width. Ported from `draw_inset`; needs the `t0c`
/// NanoVG path bindings.
pub fn draw_inset(
    vg: &mut Context,
    b: clay::BoundingBox,
    radius: clay::CornerRadius,
    width: f32,
    ink: Color,
) {
    vg.begin_path();
    rounded_rect(vg, b, radius);
    rounded_rect(vg, grow_box(b, -width), inset_radius(radius, width));
    vg.path_winding(mv_nvg::Winding::CW);
    vg.fill_color(ink);
    vg.fill();
}

/// A box-shadow with no spread under `box`: its offset copy of the box,
/// blurred. Ported from `draw_shadow`; needs the `t0c` NanoVG path and paint
/// bindings.
pub fn draw_shadow(
    vg: &mut Context,
    b: clay::BoundingBox,
    radius: clay::CornerRadius,
    shadow: Shadow,
) {
    if shadow.color[3] <= 0.0 {
        return;
    }
    let sigma = shadow.blur / 2.0;
    let feather = (sigma * (2.0 * std::f32::consts::PI).sqrt()).max(1.0);
    let glow = clay::BoundingBox {
        x: b.x + shadow.offset[0],
        y: b.y + shadow.offset[1],
        width: b.width,
        height: b.height,
    };
    let r = radius
        .topLeft
        .max(radius.topRight)
        .max(radius.bottomLeft)
        .max(radius.bottomRight);
    let transparent = [shadow.color[0], shadow.color[1], shadow.color[2], 0.0];
    let paint = vg.box_gradient(
        glow.x,
        glow.y,
        glow.width,
        glow.height,
        r,
        feather,
        shadow.color,
        transparent,
    );
    vg.begin_path();
    vg.rect(
        glow.x - shadow.blur * 2.0,
        glow.y - shadow.blur * 2.0,
        glow.width + shadow.blur * 4.0,
        glow.height + shadow.blur * 4.0,
    );
    rounded_rect(vg, b, radius);
    vg.path_winding(mv_nvg::Winding::CW);
    vg.fill_paint(&paint);
    vg.fill();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spread_radius_scales_and_eases() {
        assert_eq!(spread_radius(0.0, 5.0), 0.0);
        assert_eq!(spread_radius(8.0, 5.0), 13.0);
        let eased = spread_radius(2.0, 5.0);
        assert!(eased > 2.0 && eased < 7.0, "got {eased}");
    }

    #[test]
    fn grow_box_expands_every_side() {
        let grown = grow_box(
            clay::BoundingBox {
                x: 10.0,
                y: 20.0,
                width: 30.0,
                height: 40.0,
            },
            5.0,
        );
        assert_eq!(grown.x, 5.0);
        assert_eq!(grown.y, 15.0);
        assert_eq!(grown.width, 40.0);
        assert_eq!(grown.height, 50.0);
    }

    #[test]
    fn layer_config_skips_noop_layer() {
        let (tint, ptr) = layer_config(LAYER_REST, EFFECT_NONE);
        assert_eq!(tint, [0.0; 4]);
        assert!(ptr.is_null());
    }

    #[test]
    fn effect_store_holds_only_the_current_frame() {
        // One simulated frame: every overlay boxes its own payload, each
        // pointer separately boxed so none moves.
        clear_effects();
        let first = effect(EFFECT_NONE);
        for i in 0..64 {
            effect(Effect {
                inset: i as f32,
                ..EFFECT_NONE
            });
        }
        let last = effect(EFFECT_NONE);
        assert_ne!(first, last);
        assert_eq!(EFFECTS.with(|store| store.borrow().len()), 66);
        // The frame ends: `draw_frame` frees the store before the next
        // layout, so only the next frame's entries stay.
        clear_effects();
        effect(EFFECT_NONE);
        assert_eq!(EFFECTS.with(|store| store.borrow().len()), 1);
        clear_effects();
    }
}
