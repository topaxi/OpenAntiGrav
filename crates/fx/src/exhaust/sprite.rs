//! The camera-facing quad builders, split from `exhaust.rs` under the
//! 1,000-line ratchet in `scripts/check-file-size.py`; a move, with no
//! behaviour change.

use oag_core::math::Vec3;

use oag_mesh::mesh::GpuVertex;

/// One camera-facing additive sprite at a world point, in the flare's own shape.
///
/// **Not part of the recovered exhaust.** It exists so a caller with something
/// else to draw as a glowing dot - a projectile in flight, today - can reuse the
/// flare's pipeline and texture instead of standing up a second one for a
/// placeholder. `half_size` is in world units and `alpha` is the additive
/// weight; the flare's own values come from [`super::Exhaust`] and are not what
/// a caller here wants.
///
/// The billboard is built from the caller's `right` and `up`, which are read out
/// of the view matrix the same way [`super::Exhaust::vertices`]' are - see its
/// docs for why that is world-space rather than a post-projection sprite.
#[must_use]
pub fn sprite(centre: Vec3, right: Vec3, up: Vec3, half_size: f32, alpha: f32) -> [GpuVertex; 6] {
    quad(centre, right * half_size, up * half_size, alpha)
}

/// Six vertices - two triangles - for one camera-facing quad.
///
/// Wound as an explicit triangle list rather than a strip, matching how the
/// rest of the exhaust module fills its buffers. The colour's alpha carries
/// the flicker, which the additive blend then weights by - `src.rgb * src.a +
/// dst.rgb`, recovered from `ExhaustFlare_BuildDisplayList`.
pub(crate) fn quad(centre: Vec3, right: Vec3, up: Vec3, alpha: f32) -> [GpuVertex; 6] {
    let corner = |sx: f32, sy: f32, u: f32, v: f32| GpuVertex {
        position: (centre + right * sx + up * sy).to_array(),
        // The flare is emissive: `lit = 0.0` keeps the mesh light rig off it.
        normal: [0.0, 0.0, 1.0],
        colour: [1.0, 1.0, 1.0, alpha],
        texcoord: [u, v],
        lit: 0.0,
        ..bytemuck::Zeroable::zeroed()
    };
    let bl = corner(-1.0, -1.0, 0.0, 1.0);
    let br = corner(1.0, -1.0, 1.0, 1.0);
    let tl = corner(-1.0, 1.0, 0.0, 0.0);
    let tr = corner(1.0, 1.0, 1.0, 0.0);
    [bl, br, tl, br, tr, tl]
}
