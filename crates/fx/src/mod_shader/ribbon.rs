//! A straight ribbon between two points: a mod's beam. The strip of [`crate::beam`] and
//! [`crate::leach_strip`] each carry a weapon's wobble and state; this carries neither.

use oag_core::math::Vec3;
use oag_mesh::mesh::GpuVertex;

/// Stretches along a beam: its side follows the eye, which a beam's near end can pass close to.
const SEGMENTS: usize = 12;

/// The triangles of a ribbon from `from` to `to` facing `eye`, `half_width` to each side:
/// `u` runs `0..1` along it and `v` `0..1` across, every corner `colour`. A ribbon with no
/// length adds nothing.
pub fn ribbon(
    from: Vec3,
    to: Vec3,
    half_width: f32,
    eye: Vec3,
    colour: [f32; 4],
    out: &mut Vec<GpuVertex>,
) {
    let along = to - from;
    if along.length_squared() < 1e-6 {
        return;
    }
    let side = |at: Vec3| {
        let side = along.cross(eye - at);
        if side.length_squared() < 1e-6 {
            Vec3::ZERO
        } else {
            side.normalize() * half_width
        }
    };
    let corner = |at: Vec3, u: f32, v: f32| GpuVertex {
        position: at.to_array(),
        colour,
        texcoord: [u, v],
        ..bytemuck::Zeroable::zeroed()
    };
    for segment in 0..SEGMENTS {
        let (u0, u1) = (
            segment as f32 / SEGMENTS as f32,
            (segment + 1) as f32 / SEGMENTS as f32,
        );
        let (a, b) = (from + along * u0, from + along * u1);
        let (sa, sb) = (side(a), side(b));
        let (a0, a1) = (corner(a - sa, u0, 0.0), corner(a + sa, u0, 1.0));
        let (b0, b1) = (corner(b - sb, u1, 0.0), corner(b + sb, u1, 1.0));
        out.extend_from_slice(&[a0, b0, a1, b0, b1, a1]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ribbon_runs_between_its_ends_and_faces_the_eye() {
        let (from, to) = (Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 100.0));
        let mut out = Vec::new();
        ribbon(
            from,
            to,
            2.0,
            Vec3::new(0.0, 50.0, 50.0),
            [1.0; 4],
            &mut out,
        );
        assert_eq!(out.len(), SEGMENTS * 6);
        assert_eq!(out[0].texcoord, [0.0, 0.0]);
        assert_eq!(out[out.len() - 2].texcoord, [1.0, 1.0]);
        for vertex in &out {
            let [x, y, z] = vertex.position;
            assert!((0.0..=100.0).contains(&z));
            assert_eq!(y, 0.0, "the eye is straight above, so the width is flat");
            assert_eq!(x.abs(), 2.0);
        }
    }

    #[test]
    fn a_ribbon_with_no_length_adds_nothing() {
        let mut out = Vec::new();
        ribbon(Vec3::ONE, Vec3::ONE, 2.0, Vec3::ZERO, [1.0; 4], &mut out);
        assert!(out.is_empty());
    }
}
