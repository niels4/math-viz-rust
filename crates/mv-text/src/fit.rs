//! The scale that fits a line to a width.

/// The slack kept on each side of a fitted line.
pub const FIT_SLACK: f32 = 2.0;

/// The scale that fits a measured width to a container: two looks, the second
/// correcting the first by how the width moves with the scale.
pub fn fit(container: f32, width_at: &dyn Fn(f32) -> f32) -> f32 {
    let room = container - FIT_SLACK;
    let natural = width_at(1.0);
    if room <= 0.0 || natural <= room {
        return 1.0;
    }
    let first = room / natural;
    let width = width_at(first);
    if width <= room {
        return first;
    }
    let scaling = (natural - width) / (1.0 - first);
    let solved = (room - (natural - scaling)) / scaling;
    if solved > 0.0 { solved } else { first }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_is_use_fit_width() {
        struct Part {
            scaling: f32,
            fixed: f32,
        }
        let fits = Part {
            scaling: 100.0,
            fixed: 20.0,
        };
        assert_eq!(
            fit(300.0 + FIT_SLACK, &|scale| fits.scaling * scale
                + fits.fixed),
            1.0
        );
        let wide = Part {
            scaling: 200.0,
            fixed: 20.0,
        };
        let width_at = |scale: f32| wide.scaling * scale + wide.fixed;
        let scale = fit(120.0 + FIT_SLACK, &width_at);
        assert!((scale - 0.5).abs() < 1e-5, "scale {scale}, not 0.5");
        assert!(
            (width_at(scale) - 120.0).abs() < 1e-3,
            "{} px wide, not 120",
            width_at(scale)
        );
    }
}
