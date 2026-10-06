//! What the light's own projection is asserted to do, with no GPU in it.
//!
//! The pass itself is checked by `tests/shadow_map_coverage.rs`, which needs
//! an adapter; the fit is arithmetic and is checked here, because a box that
//! does not contain its casters produces a shadow the size of the overlap -
//! which is what the first cut of Wipeout HD's tier drew.

use super::*;

fn fit() -> Fit {
    Fit {
        centre: Vec3::new(10.0, -5.0, 20.0),
        radius: 12.0,
        towards_light: Vec3::new(0.0, 1.0, 0.0),
    }
}

/// A point maps into the map's clip box iff it is inside the fitted radius.
#[test]
fn the_fit_contains_what_it_was_sized_for() {
    let matrix = fit().matrix();
    let inside = |point: Vec3| {
        let clip = matrix * point.extend(1.0);
        let ndc = clip.truncate() / clip.w;
        (-1.0..=1.0).contains(&ndc.x) && (-1.0..=1.0).contains(&ndc.y)
    };
    assert!(inside(fit().centre), "its own centre");
    // Just inside the radius, on both axes across the light.
    assert!(inside(fit().centre + Vec3::new(11.5, 0.0, 0.0)));
    assert!(inside(fit().centre + Vec3::new(0.0, 0.0, 11.5)));
    // And just outside it.
    assert!(!inside(fit().centre + Vec3::new(12.5, 0.0, 0.0)));
    assert!(!inside(fit().centre + Vec3::new(0.0, 0.0, 12.5)));
}

/// Depth runs `0..1` across the box, which is what the receiver's own `ndc.z`
/// test in `mesh`'s shader assumes.
#[test]
fn the_depth_range_is_the_one_the_receiver_tests_against() {
    let matrix = fit().matrix();
    let at = |point: Vec3| {
        let clip = matrix * point.extend(1.0);
        clip.z / clip.w
    };
    // The centre sits in the middle of the box's depth, not at either end.
    let middle = at(fit().centre);
    assert!(
        (0.1..=0.9).contains(&middle),
        "the centre's depth is {middle}"
    );
    // Towards the light is nearer the map's near plane, away is further.
    assert!(at(fit().centre + Vec3::Y * 5.0) < middle);
    assert!(at(fit().centre - Vec3::Y * 5.0) > middle);
}

/// The projection is orthographic: a directional light has no position to
/// build a perspective frustum around, and a perspective one would make a
/// craft's shadow grow as it climbed.
#[test]
fn the_projection_is_orthographic() {
    let matrix = fit().matrix();
    for offset in [Vec3::ZERO, Vec3::Y * 8.0, Vec3::NEG_Y * 8.0] {
        let clip = matrix * (fit().centre + offset).extend(1.0);
        assert!((clip.w - 1.0).abs() < 1e-5, "w is {} at {offset:?}", clip.w);
    }
}

/// A rig with no usable direction still produces a finite matrix.
///
/// The strength that reaches the shader is the caller's, so nothing is drawn
/// either way - what this rules out is a NaN reaching a vertex position, which
/// takes the whole pass with it rather than the shadow alone.
#[test]
fn a_degenerate_light_direction_still_gives_a_finite_matrix() {
    let matrix = Fit {
        towards_light: Vec3::ZERO,
        ..fit()
    }
    .matrix();
    assert!(matrix.to_cols_array().iter().all(|value| value.is_finite()));
}

/// A zero radius does not divide the box to nothing.
#[test]
fn a_zero_radius_is_clamped_to_something_drawable() {
    let matrix = Fit {
        radius: 0.0,
        ..fit()
    }
    .matrix();
    assert!(matrix.to_cols_array().iter().all(|value| value.is_finite()));
    let clip = matrix * fit().centre.extend(1.0);
    assert!((clip.w - 1.0).abs() < 1e-5);
}

/// A fit whose centre sits exactly on a `map_size`-texel grid point for a
/// light pointing straight up: `x` and `z` are both already whole multiples
/// of the texel size (zero qualifies), so a nudge has a full half-texel of
/// room in every direction before it crosses into the next bucket.
///
/// The plain [`fit`] fixture above does not have this property - its centre
/// is arbitrary, so a fixed-size nudge can land arbitrarily close to a
/// rounding boundary already, and cross it. That is a property of the
/// fixture, not of [`Fit::snapped`]: this fixture exists so the nudge tests
/// below are testing snapping and not, by accident, a boundary they didn't
/// choose.
fn snap_fixture() -> Fit {
    Fit {
        centre: Vec3::new(0.0, -5.0, 0.0),
        radius: 12.0,
        towards_light: Vec3::Y,
    }
}

/// The whole reason [`Fit::snapped`] exists: a centre nudged by less than half
/// a texel must land on the exact same texel, so its matrix comes out
/// byte-identical to the unnudged one. This is the "the mapped shadow box
/// slides and rotates with the player" bug's own regression test - without
/// snapping, any nonzero nudge moves the matrix by some amount, never zero.
#[test]
fn a_sub_texel_nudge_snaps_to_the_same_matrix() {
    let map_size = 1024_u32;
    let base = snap_fixture().snapped(map_size);
    let texel = snap_fixture().radius.max(1.0) * 2.0 / map_size as f32;
    // A light straight up has its light-space axes in the world's X/Z plane,
    // so nudging on X and Z is nudging in light space.
    for nudge in [
        Vec3::new(texel * 0.1, 0.0, 0.0),
        Vec3::new(0.0, 0.0, texel * 0.4),
        Vec3::new(-texel * 0.2, 0.0, texel * 0.3),
        Vec3::new(texel * 0.45, 0.0, -texel * 0.45),
    ] {
        let nudged = Fit {
            centre: snap_fixture().centre + nudge,
            ..snap_fixture()
        }
        .snapped(map_size);
        assert_eq!(
            nudged.matrix().to_cols_array(),
            base.matrix().to_cols_array(),
            "nudge {nudge:?} was not absorbed by snapping"
        );
    }
}

/// A nudge past half a texel does move the snapped result - snapping quantises
/// to the grid, it does not freeze the centre in place.
#[test]
fn a_whole_texel_nudge_moves_the_snapped_centre_by_a_whole_texel() {
    let map_size = 1024_u32;
    let texel = snap_fixture().radius.max(1.0) * 2.0 / map_size as f32;
    let base = snap_fixture().snapped(map_size);
    let moved = Fit {
        centre: snap_fixture().centre + Vec3::new(texel, 0.0, 0.0),
        ..snap_fixture()
    }
    .snapped(map_size);
    let delta = (moved.centre - base.centre).length();
    assert!(
        (delta - texel).abs() < 1e-4,
        "delta was {delta}, expected one texel ({texel})"
    );
}

/// Snapping only touches the two axes across the light: the axis along it -
/// depth into the light, never a texel choice - passes through untouched.
#[test]
fn snapping_does_not_move_the_centre_along_the_light() {
    let map_size = 1024_u32;
    let towards_light = fit().towards_light.normalize_or_zero();
    let snapped = fit().snapped(map_size);
    let along_before = fit().centre.dot(towards_light);
    let along_after = snapped.centre.dot(towards_light);
    assert!((along_before - along_after).abs() < 1e-4);
}

/// A degenerate light direction still snaps to a finite centre - the same
/// fallback [`Fit::matrix`] uses, read the same way, so the two never
/// disagree on what "no usable direction" means.
#[test]
fn a_degenerate_light_direction_still_snaps_to_something_finite() {
    let snapped = Fit {
        towards_light: Vec3::ZERO,
        ..fit()
    }
    .snapped(1024);
    assert!(snapped.centre.is_finite());
}
