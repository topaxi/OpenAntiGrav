//! A model's opaque draws that an `Anim Transform` moves, grouped by the node
//! that moves them, for the `mapped` shadow tier: the circuit's own moving
//! parts are the only scenery that tier casts, because everything that stands
//! still already has its shadow in the disc's own lighting.
//!
//! **Why a group per node.** The vertex shader places a moving vertex with
//! `node_anims.transform[xform] * position`, and the caster pass binds no node
//! matrices - it draws `mvp * position`. So a node's geometry is in the node's
//! own space, and the caster for it is that geometry at the node's world
//! matrix this frame. Every vertex of a draw carries the same `xform`, so a
//! draw belongs to exactly one node.

use oag_core::math::frustum::Frustum;
use oag_core::math::{Mat4, Vec3};

/// One animated node's opaque geometry, in the node's own space.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MovingCaster {
    /// The node's `GpuVertex::xform` slot: `1..`, an index into
    /// `Model::sample_anim_nodes` one past its own.
    pub(crate) xform: u32,
    /// The node's opaque index ranges.
    pub(crate) ranges: Vec<std::ops::Range<u32>>,
    /// A sphere around those ranges' vertices, in the node's own space.
    pub(crate) centre: Vec3,
    pub(crate) radius: f32,
}

impl MovingCaster {
    /// Whether this node, at `node` (its world matrix this frame), can reach
    /// `frustum`: its sphere carried into the world, the radius grown by the
    /// matrix's largest axis scale.
    pub(crate) fn reaches(&self, node: Mat4, frustum: &Frustum) -> bool {
        let centre = node.transform_point3(self.centre);
        let scale = node
            .x_axis
            .truncate()
            .length()
            .max(node.y_axis.truncate().length())
            .max(node.z_axis.truncate().length());
        frustum.intersects_sphere(centre, self.radius * scale)
    }
}

/// Groups `opaque`'s moving draws by node, each with its own-space bound.
/// Draws that do not move, or whose vertices name no node, are left out.
pub(super) fn group(
    model: &oag_mesh::mesh::Model,
    opaque: &[&oag_mesh::mesh::DrawCall],
) -> Vec<MovingCaster> {
    let mut by_node: std::collections::BTreeMap<u32, Vec<std::ops::Range<u32>>> =
        std::collections::BTreeMap::new();
    for draw in opaque.iter().filter(|draw| draw.moving) {
        let xform = model
            .indices
            .get(draw.range.start as usize)
            .and_then(|&i| model.vertices.get(i as usize))
            .map_or(0, |v| v.xform);
        if xform == 0 {
            continue;
        }
        by_node.entry(xform).or_default().push(draw.range.clone());
    }
    by_node
        .into_iter()
        .map(|(xform, ranges)| {
            let positions = || {
                ranges
                    .iter()
                    .flat_map(|range| &model.indices[range.start as usize..range.end as usize])
                    .filter_map(|&i| model.vertices.get(i as usize))
                    .map(|v| Vec3::from_array(v.position))
            };
            let (low, high) = positions().fold(
                (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
                |(low, high), p| (low.min(p), high.max(p)),
            );
            let centre = (low + high) * 0.5;
            let radius = positions().map(|p| p.distance(centre)).fold(0.0, f32::max);
            MovingCaster {
                xform,
                ranges,
                centre,
                radius,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use oag_core::math::camera;

    fn caster(centre: Vec3, radius: f32) -> MovingCaster {
        MovingCaster {
            xform: 1,
            ranges: std::iter::once(0..3).collect(),
            centre,
            radius,
        }
    }

    /// A light box 10 across and 20 deep, looking down -z from z = 0 .. -20.
    fn light_box() -> Frustum {
        Frustum::from_view_projection(camera::orthographic(-5.0, 5.0, -5.0, 5.0, 0.0, 20.0))
    }

    /// The node's own matrix decides: the same geometry is in the box at one
    /// place and out of it at another.
    #[test]
    fn a_node_reaches_the_box_where_its_matrix_puts_it() {
        let node = caster(Vec3::ZERO, 1.0);
        let inside = Mat4::from_translation(Vec3::new(0.0, 0.0, -10.0));
        let outside = Mat4::from_translation(Vec3::new(40.0, 0.0, -10.0));
        assert!(node.reaches(inside, &light_box()));
        assert!(!node.reaches(outside, &light_box()));
    }

    /// A scaled node's sphere grows with it, so a big node just outside the
    /// box by its unscaled radius still counts.
    #[test]
    fn a_scaled_node_reaches_by_its_scaled_radius() {
        let node = caster(Vec3::ZERO, 1.0);
        let at = Mat4::from_translation(Vec3::new(8.0, 0.0, -10.0));
        assert!(!node.reaches(at, &light_box()));
        assert!(node.reaches(at * Mat4::from_scale(Vec3::splat(4.0)), &light_box()));
    }
}
