//! Flexbox-style shrink pass ported from `flex.odin` (`Flex_Item`, `flex_shrink`).
//!
//! Pure arithmetic over base sizes, minimums and shrink factors: no platform
//! calls, so this module has no external milestone needs.

/// One flexible item fed to [`flex_shrink`]. Ported from `Flex_Item`.
#[derive(Clone, Copy, Debug)]
pub struct FlexItem {
    /// Preferred (basis) size of the item.
    pub base: f32,
    /// Minimum size the item may shrink to.
    pub min: f32,
    /// `flex: none`: never shrunk.
    pub none: bool,
}

/// Distributes a shortage of `room` over `sizes`, clamping at each item's
/// minimum, following the CSS flexbox shrink procedure. Ported from
/// `flex_shrink`.
pub fn flex_shrink(items: &[FlexItem], room: f32, sizes: &mut [f32]) {
    assert_eq!(sizes.len(), items.len());
    let mut frozen = vec![false; items.len()];
    let mut total = 0.0f32;
    for (i, item) in items.iter().enumerate() {
        sizes[i] = item.base.max(item.min);
        total += sizes[i];
        frozen[i] = item.none || item.base < item.min;
    }
    if total <= room {
        return;
    }
    loop {
        let mut free = room;
        let mut scaled = 0.0f32;
        for (i, item) in items.iter().enumerate() {
            if frozen[i] {
                free -= sizes[i];
            } else {
                free -= item.base;
                scaled += item.base;
            }
        }
        if scaled <= 0.0 {
            return;
        }
        let mut violation = 0.0f32;
        for (i, item) in items.iter().enumerate() {
            if !frozen[i] {
                sizes[i] = item.base + free * item.base / scaled;
                if sizes[i] < item.min {
                    violation += item.min - sizes[i];
                }
            }
        }
        if violation <= 0.0 {
            return;
        }
        for (i, item) in items.iter().enumerate() {
            if !frozen[i] && sizes[i] < item.min {
                sizes[i] = item.min;
                frozen[i] = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(base: f32, min: f32, none: bool) -> FlexItem {
        FlexItem { base, min, none }
    }

    #[test]
    fn no_shrink_when_everything_fits() {
        let items = [item(40.0, 10.0, true), item(60.0, 10.0, true)];
        let mut sizes = [0.0; 2];
        flex_shrink(&items, 100.0, &mut sizes);
        assert_eq!(sizes, [40.0, 60.0]);
    }

    #[test]
    fn shrink_distributes_proportionally_to_base() {
        let items = [item(40.0, 0.0, false), item(60.0, 0.0, false)];
        let mut sizes = [0.0; 2];
        flex_shrink(&items, 80.0, &mut sizes);
        assert!((sizes[0] - 32.0).abs() < 1e-4, "got {sizes:?}");
        assert!((sizes[1] - 48.0).abs() < 1e-4, "got {sizes:?}");
    }

    #[test]
    fn zero_shrink_item_is_frozen_and_keeps_base() {
        let items = [item(40.0, 10.0, true), item(60.0, 10.0, false)];
        let mut sizes = [0.0; 2];
        flex_shrink(&items, 80.0, &mut sizes);
        assert_eq!(sizes, [40.0, 40.0]);
    }

    #[test]
    fn minimums_are_respected_with_freeze_and_retry() {
        let items = [item(40.0, 35.0, false), item(60.0, 10.0, false)];
        let mut sizes = [0.0; 2];
        flex_shrink(&items, 80.0, &mut sizes);
        assert!((sizes[0] - 35.0).abs() < 1e-4, "got {sizes:?}");
        assert!((sizes[1] - 45.0).abs() < 1e-4, "got {sizes:?}");
    }
}
