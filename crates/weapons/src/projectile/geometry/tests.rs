//! What [`super`]'s geometry helpers are asserted to do.
//!
//! Moved here with the functions themselves on the day the Plasma's wind-up
//! took `projectile.rs` past the 1,000-line rule in
//! `scripts/check-file-size.py`: a test that only exercises a segment against a
//! sphere belongs beside the segment-against-sphere code rather than beside the
//! flight model.

use super::*;
use oag_physics::params::Dimensions;

/// The two degenerate cases the quadratic would otherwise get wrong: a
/// segment that starts inside must report the start, and one aimed away from
/// a sphere it is outside of must report nothing however long it is.
#[test]
fn the_sphere_test_handles_starting_inside_and_pointing_away() {
    let centre = Vec3::new(0.0, 0.0, 10.0);
    assert_eq!(
        segment_sphere(centre, centre + Vec3::Z * 100.0, centre, 2.0),
        Some(0.0)
    );
    assert_eq!(
        segment_sphere(Vec3::ZERO, Vec3::Z * -100.0, centre, 2.0),
        None,
        "a segment aimed away from a sphere hit it"
    );
    assert_eq!(
        segment_sphere(Vec3::ZERO, Vec3::X * 100.0, centre, 2.0),
        None,
        "a segment that passes wide hit it"
    );
    // And one that reaches exactly the near face.
    let t = segment_sphere(Vec3::ZERO, Vec3::Z * 8.0, centre, 2.0).expect("a grazing hit");
    assert!((t - 1.0).abs() < 1e-4, "entered at {t}");
}

/// The radius is half the *largest* dimension, so a long craft is not
/// modelled by its narrowest axis - **and the sphere is therefore wider
/// than the hull**, which is the part [`hull_radius`]' docs are careful
/// about and which this pins rather than leaves to the prose.
#[test]
fn the_hull_radius_circumscribes_the_longest_axis_and_bulges_past_the_rest() {
    let dimensions = Dimensions {
        length: 4.0,
        width: 2.0,
        height: 1.0,
        ..Dimensions::default()
    };
    let radius = hull_radius(&dimensions);
    assert_eq!(radius, 2.0);
    // Nose to tail it matches the box exactly...
    assert_eq!(radius, dimensions.length * 0.5);
    // ...and on both other axes it reaches further, so a shot that the box
    // would miss still hits. Generous, not conservative.
    assert!(radius > dimensions.width * 0.5);
    assert!(radius > dimensions.height * 0.5);
}
