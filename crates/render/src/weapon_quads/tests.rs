//! Split from an inline `#[cfg(test)] mod` under the 200-line rule in
//! `scripts/check-file-size.py`.

use oag_core::math::Vec3;

use super::geometry::{
    BOLT_HALF_WIDTH, BOLT_NEAR_FRACTION, FLASH_SIZE_RANGE, FLASH_SIZE_SCALE, bolt_vertices,
    flash_vertices,
};
use super::random::flash_roll;

const RIGHT: Vec3 = Vec3::new(1.0, 0.0, 0.0);
const UP: Vec3 = Vec3::new(0.0, 1.0, 0.0);

#[test]
fn the_bolt_streak_spans_the_far_fraction_of_the_inter_tick_segment() {
    let prev = Vec3::new(0.0, 0.0, 0.0);
    let curr = Vec3::new(0.0, 0.0, 10.0);
    let vertices = bolt_vertices(prev, curr, RIGHT, UP);
    // `far` (uv.u = 1) sits at `prev` exactly; `near` (uv.u = 0) sits at
    // `curr + BOLT_NEAR_FRACTION * (prev - curr)`, i.e. the closest fifth of
    // the segment (measured from `curr`) is left uncovered.
    let near_z = curr.z + (prev.z - curr.z) * BOLT_NEAR_FRACTION;
    for v in &vertices {
        if v.texcoord[0] == 0.0 {
            assert!((v.position[2] - near_z).abs() < 1e-5, "{v:?}");
        } else {
            assert!((v.position[2] - prev.z).abs() < 1e-5, "{v:?}");
        }
    }
}

#[test]
fn the_bolts_two_quads_are_perpendicular_diagonals() {
    let prev = Vec3::new(0.0, 0.0, 0.0);
    let curr = Vec3::new(0.0, 0.0, 1.0);
    let vertices = bolt_vertices(prev, curr, RIGHT, UP);
    // Quad 1 is vertices[0..6], quad 2 is vertices[6..12]. Each quad's own
    // width vector is the offset between its two `x`-differing corners at
    // the same `z` - reading it back this way is independent of
    // `ribbon_quad`'s own corner order.
    let width_vec = |quad: &[crate::mesh::GpuVertex]| {
        let a = quad[0].position;
        let b = quad[2].position;
        [b[0] - a[0], b[1] - a[1]]
    };
    let w1 = width_vec(&vertices[0..6]);
    let w2 = width_vec(&vertices[6..12]);
    let dot = w1[0] * w2[0] + w1[1] * w2[1];
    assert!(
        dot.abs() < 1e-4,
        "not perpendicular: {w1:?} . {w2:?} = {dot}"
    );
    let mag1 = (w1[0] * w1[0] + w1[1] * w1[1]).sqrt();
    let mag2 = (w2[0] * w2[0] + w2[1] * w2[1]).sqrt();
    let expected = BOLT_HALF_WIDTH * 2.0 * std::f32::consts::SQRT_2;
    assert!((mag1 - expected).abs() < 1e-4, "{mag1} vs {expected}");
    assert!((mag2 - expected).abs() < 1e-4, "{mag2} vs {expected}");
}

#[test]
fn the_flash_quad_is_a_square_of_the_requested_half_size() {
    let center = Vec3::new(1.0, 2.0, 3.0);
    let vertices = flash_vertices(center, RIGHT, UP, 2.0, 0.0, 0.5);
    for v in &vertices {
        let dx = v.position[0] - center.x;
        let dy = v.position[1] - center.y;
        assert!((dx.abs() - 2.0).abs() < 1e-5, "{v:?}");
        assert!((dy.abs() - 2.0).abs() < 1e-5, "{v:?}");
        assert_eq!(v.colour, [1.0, 1.0, 1.0, 0.5]);
    }
}

#[test]
fn rotating_the_flash_keeps_corners_equidistant_from_the_centre() {
    let center = Vec3::ZERO;
    let vertices = flash_vertices(center, RIGHT, UP, 3.0, 0.7, 1.0);
    let expected = 3.0 * std::f32::consts::SQRT_2;
    for v in &vertices {
        let d = (v.position[0].powi(2) + v.position[1].powi(2)).sqrt();
        assert!((d - expected).abs() < 1e-4, "{v:?} vs {expected}");
    }
}

#[test]
fn flash_roll_ranges_match_the_recovered_constants() {
    for slot in 0..8u32 {
        for tick in 0..64u32 {
            let (rotation, half_size, alpha) = flash_roll(slot, tick);
            assert!((0.0..std::f32::consts::TAU).contains(&rotation));
            assert!(
                (FLASH_SIZE_RANGE.0 * FLASH_SIZE_SCALE..=FLASH_SIZE_RANGE.1 * FLASH_SIZE_SCALE)
                    .contains(&half_size),
                "{half_size}"
            );
            assert!((0.588..=1.0).contains(&alpha), "{alpha}");
        }
    }
}

#[test]
fn flash_roll_is_deterministic_and_varies_with_its_inputs() {
    assert_eq!(flash_roll(3, 100), flash_roll(3, 100));
    assert_ne!(flash_roll(3, 100), flash_roll(3, 101));
    assert_ne!(flash_roll(3, 100), flash_roll(4, 100));
}
