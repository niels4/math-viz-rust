//! The plane's visible plot range from its size and camera: what the P and Q
//! points read and ride on.
//!
//! Ported from `src/components/views/CartesianPlane/state.ts` at `96f1e84`
//! (`VisibleRange`, `visibleRangeAtom`): `None` while either size is 0, else
//! the camera centre spread by half the plane over the px-per-unit scale.

use crate::camera::UNSCALED_UNIT_SIZE;

/// The plot range the plane shows (`VisibleRange`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VisibleRange {
    /// The leftmost plot x.
    pub min_x: f64,
    /// The rightmost plot x.
    pub max_x: f64,
    /// The bottommost plot y.
    pub min_y: f64,
    /// The topmost plot y.
    pub max_y: f64,
}

/// The visible plot range for a plane of `width × height` px at `center` and
/// `scale` (`visibleRangeAtom` as a pure function): `None` when either size
/// is 0; else the centre (`[x, y]`, as the camera holds it) spread by half
/// the plane — floored at the centre px — over `px_per_unit`.
/// `center` and `scale` are the camera's live values, as the atom reads them.
/// The plane's visible plot range at the live camera over the last frame's
/// content box (the atom as a function of the app): `None` before the first
/// layout or off a degenerate span, when the slider's press returns early.
/// The slider and the Points rows read this, as the atom's subscribers do.
#[must_use]
pub fn live(app: &crate::app::App) -> Option<VisibleRange> {
    let data = app
        .clay
        .as_ref()
        .expect("clay context initialised")
        .get_element_data(mv_clay::id(crate::plane::PLANE_ID, 0));
    if !data.found {
        return None;
    }
    let content = crate::plane::canvas_content(&data.boundingBox);
    visible_range(
        f64::from(content.width),
        f64::from(content.height),
        app.camera.center,
        app.camera.scale,
    )
}

#[must_use]
pub fn visible_range(
    width: f64,
    height: f64,
    center: [f64; 2],
    scale: f64,
) -> Option<VisibleRange> {
    if width == 0.0 || height == 0.0 {
        return None;
    }
    let [x, y] = center;
    let px_per_unit = UNSCALED_UNIT_SIZE / scale;
    let center_x = (width / 2.0).floor();
    let center_y = (height / 2.0).floor();
    Some(VisibleRange {
        min_x: x - center_x / px_per_unit,
        max_x: x + (width - center_x) / px_per_unit,
        min_y: y - (height - center_y) / px_per_unit,
        max_y: y + center_y / px_per_unit,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_camera_shows_x_minus5_to_5_and_y_minus4_to_4() {
        assert_eq!(
            visible_range(500.0, 400.0, [0.0, 0.0], 0.5),
            Some(VisibleRange {
                min_x: -5.0,
                max_x: 5.0,
                min_y: -4.0,
                max_y: 4.0,
            })
        );
    }

    #[test]
    fn scale_1_widens_the_view_tenfold() {
        assert_eq!(
            visible_range(500.0, 400.0, [0.0, 0.0], 1.0),
            Some(VisibleRange {
                min_x: -10.0,
                max_x: 10.0,
                min_y: -8.0,
                max_y: 8.0,
            })
        );
    }

    #[test]
    fn a_shifted_centre_shifts_the_range() {
        assert_eq!(
            visible_range(500.0, 400.0, [3.0, 0.0], 1.0),
            Some(VisibleRange {
                min_x: -7.0,
                max_x: 13.0,
                min_y: -8.0,
                max_y: 8.0,
            })
        );
    }

    #[test]
    fn a_zero_size_shows_no_range() {
        assert_eq!(visible_range(0.0, 400.0, [0.0, 0.0], 0.5), None);
        assert_eq!(visible_range(500.0, 0.0, [0.0, 0.0], 0.5), None);
    }
}
