//! The camera-facing quad every billboard particle is drawn as.

use oag_core::math::Vec3;

use oag_mesh::mesh::GpuVertex;

/// Six vertices - two triangles - for one camera-facing quad.
pub(super) fn quad(
    centre: Vec3,
    right: Vec3,
    up: Vec3,
    cap: f32,
    colour: [f32; 3],
    alpha: f32,
) -> [GpuVertex; 6] {
    let corner = |sx: f32, sy: f32, u: f32, v: f32| GpuVertex {
        position: (centre + right * sx + up * sy).to_array(),
        normal: [0.0, 0.0, 1.0],
        colour: [colour[0], colour[1], colour[2], alpha],
        texcoord: [u, v],
        // The `lit` slot is repurposed by this pipeline: particles are
        // emissive (never lit by the mesh rig), so it carries the cap
        // fraction the fragment profile needs - see `sparks.wgsl`.
        lit: cap,
        ..bytemuck::Zeroable::zeroed()
    };
    let bl = corner(-1.0, -1.0, 0.0, 1.0);
    let br = corner(1.0, -1.0, 1.0, 1.0);
    let tl = corner(-1.0, 1.0, 0.0, 0.0);
    let tr = corner(1.0, 1.0, 1.0, 0.0);
    [bl, br, tl, br, tr, tl]
}
