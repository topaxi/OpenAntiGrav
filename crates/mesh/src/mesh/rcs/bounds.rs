//! The model's framing sphere, split out of `rcs.rs` under the 1,000-line rule.

use oag_core::math::Vec3;

use crate::mesh::GpuVertex;

/// The whole model's framing sphere, which the viewer's camera is placed from.
///
/// Centre of the axis-aligned bounds rather than of the vertices: a circuit
/// carries most of its vertices in the few most detailed corners, and averaging
/// them puts the camera looking at a corner of the track.
pub(super) fn bounding_sphere(vertices: &[GpuVertex]) -> ([f32; 3], f32) {
    if vertices.is_empty() {
        return ([0.0; 3], 0.0);
    }
    let mut min = [f32::MAX; 3];
    let mut max = [f32::MIN; 3];
    for v in vertices {
        for i in 0..3 {
            min[i] = min[i].min(v.position[i]);
            max[i] = max[i].max(v.position[i]);
        }
    }
    let centre: [f32; 3] = std::array::from_fn(|i| (min[i] + max[i]) / 2.0);
    let radius = vertices
        .iter()
        .map(|v| (Vec3::from_array(v.position) - Vec3::from_array(centre)).length())
        .fold(0.0f32, f32::max)
        .max(0.001);
    (centre, radius)
}
