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
/// test in `mesh.wgsl` assumes.
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
