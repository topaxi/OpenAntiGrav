//! The camera-facing quad every billboard particle is drawn as.

use oag_core::math::Vec3;

use oag_mesh::mesh::GpuVertex;

use super::{EmitterSpec, Particle, sprite};

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

/// One particle's camera-facing quad, rolled if its emitter rolls and mapped
/// onto its sprite's cell of the sheet if it has one - what a billboard
/// emitter draws, shared by every blend class.
///
/// `cap = 0.5` collapses the shader's cap/cross profile to the plain radial
/// falloff a round sprite wants.
#[allow(clippy::too_many_arguments)]
pub(super) fn billboard(
    spec: &EmitterSpec,
    particle: &Particle,
    age: f32,
    half: f32,
    right: Vec3,
    up: Vec3,
    rgb: [f32; 3],
    alpha: f32,
) -> [GpuVertex; 6] {
    let mut corners = match &spec.rotation {
        Some(rotation) => rotation.quad(particle, age, half, right, up, rgb, alpha),
        None => quad(particle.position, right * half, up * half, 0.5, rgb, alpha),
    };
    if let Some(rect) = spec.sheet_rect {
        sprite::map_to_cell(&mut corners, spec.atlas.cell(rect, particle.frame));
    }
    corners
}
