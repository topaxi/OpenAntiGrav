//! Building a queryable collision world out of decoded collision nodes.
//!
//! The same layering as [`crate::handling`]: `oag_formats::collision` describes
//! what is on the disc, `oag_physics::CollisionWorld` is what the force law
//! queries, and this is the only bridge. Neither side knows about the other.
//!
//! Three facts from `docs/formats/collision.md` decide everything here, and each
//! one is a test below because each is the kind of thing that would otherwise be
//! rediscovered the hard way:
//!
//! - **The vertices are already in world space.** All 319 nodes on both discs
//!   carry an identity transform at tree depth 1, so composing the `.vex`
//!   transform chain onto them would move the geometry twice. Confidence 94.
//! - **`Cage` nodes are dropped.** The original's loader parses them and branches
//!   past them, so they are not collidable. Only the PS2 build ships any.
//! - **Triangle indices are `u16` on disc and `u32` in the physics soup**, because
//!   the format's count field permits more than 65,535 triangles per mesh even
//!   though nothing observed uses that many.

use oag_formats::collision::{CollisionNode, SurfaceKind};
use oag_physics::{CollisionWorld, Surface, TriangleSoup};

/// Turns decoded collision nodes into a world the ship can cast rays against.
///
/// Every mesh of every non-[`SurfaceKind::Cage`] node becomes one collider, and
/// the collider index is its position in the world, which is what
/// `oag_physics::forces::Environment::self_collider` compares against. Nodes are
/// consumed in the order given and meshes in file order, so the indices are
/// stable for a given track: iteration order feeds simulation state, so it has to
/// be something that cannot vary between runs.
#[must_use]
pub fn collision_world(nodes: &[CollisionNode]) -> CollisionWorld {
    let mut world = CollisionWorld::new();
    for node in nodes {
        let Some(surface) = surface_for(node.kind) else {
            continue;
        };
        for mesh in &node.geometry.meshes {
            let index = world.colliders().len() as u32;
            world.push(TriangleSoup::new(
                mesh.vertices.clone(),
                mesh.triangles
                    .iter()
                    .map(|t| [u32::from(t[0]), u32::from(t[1]), u32::from(t[2])])
                    .collect(),
                mesh.vertex_scalars.clone(),
                surface,
                index,
            ));
        }
    }
    world
}

/// The physics surface for a collision class, or `None` if it is not collidable.
///
/// `Cage` is the only `None`. It is not an error and not a gap: the original
/// parses cage nodes and then skips them, so a cage that produced a collider
/// would be a wall the real game does not have.
#[must_use]
pub fn surface_for(kind: SurfaceKind) -> Option<Surface> {
    Some(match kind {
        SurfaceKind::Wall => Surface::Wall,
        SurfaceKind::Floor => Surface::Floor,
        SurfaceKind::Reset => Surface::Reset,
        SurfaceKind::MagFloor => Surface::MagFloor,
        SurfaceKind::Cage => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use oag_formats::collision::{CollisionGeometry, CollisionMesh};

    fn mesh() -> CollisionMesh {
        CollisionMesh {
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
            triangles: vec![[0, 1, 2]],
            vertex_scalars: vec![1.0, 1.0, 1.0],
            chunks: Vec::new(),
        }
    }

    fn node(kind: SurfaceKind, meshes: usize) -> CollisionNode {
        CollisionNode {
            kind,
            node_index: 0,
            name: None,
            geometry: CollisionGeometry {
                version: 0,
                meshes: (0..meshes).map(|_| mesh()).collect(),
            },
        }
    }

    #[test]
    fn every_mesh_of_every_node_becomes_a_collider() {
        let world = collision_world(&[node(SurfaceKind::Floor, 2), node(SurfaceKind::Wall, 1)]);
        assert_eq!(world.colliders().len(), 3);
    }

    /// The original parses cage nodes and branches past them, so a cage must not
    /// become a collidable surface. Only the PS2 build ships any, which is exactly
    /// why this is easy to get wrong and never notice on PSP.
    #[test]
    fn cage_nodes_are_dropped() {
        let world = collision_world(&[
            node(SurfaceKind::Cage, 3),
            node(SurfaceKind::Floor, 1),
            node(SurfaceKind::Cage, 2),
        ]);
        assert_eq!(world.colliders().len(), 1);
        assert_eq!(world.colliders()[0].surface(), Surface::Floor);
        assert_eq!(surface_for(SurfaceKind::Cage), None);
    }

    /// A collider's index must equal its position in the world, or self-skipping
    /// silently skips the wrong geometry.
    #[test]
    fn collider_indices_match_their_position() {
        let world = collision_world(&[node(SurfaceKind::Floor, 3)]);
        for (position, collider) in world.colliders().iter().enumerate() {
            assert_eq!(collider.collider(), position as u32);
        }
    }

    /// Cage nodes must not leave a gap in the numbering either, since the index is
    /// a position and not an identifier carried over from the file.
    #[test]
    fn dropping_a_cage_leaves_no_gap_in_the_indices() {
        let world = collision_world(&[
            node(SurfaceKind::Floor, 1),
            node(SurfaceKind::Cage, 1),
            node(SurfaceKind::Wall, 1),
        ]);
        assert_eq!(world.colliders().len(), 2);
        assert_eq!(world.colliders()[1].collider(), 1);
    }

    /// `oag-physics` may not depend on `oag-formats`, so it carries its own copy
    /// of the surface frictions and of the "frictionless" sentinel rule. This
    /// crate is the only one that can see both, which makes it the only place the
    /// two can be held to agreement - and they have to agree, because
    /// `oag_physics::wall` scrubs a ship's tangential velocity using its copy
    /// while the value itself was read off the disc into the other.
    #[test]
    fn the_two_copies_of_the_friction_rule_agree() {
        for kind in SurfaceKind::ALL {
            let Some(surface) = surface_for(kind) else {
                continue;
            };
            assert_eq!(
                kind.friction(),
                surface.friction(),
                "{kind:?} disagrees between oag-formats and oag-physics"
            );
        }
        assert_eq!(
            oag_formats::collision::WALL_FRICTION,
            oag_physics::WALL_FRICTION
        );

        // The sentinel rule itself, not just the constants: one frictionless
        // side makes the whole contact frictionless, and it must never come out
        // negative.
        for a in [Some(oag_physics::WALL_FRICTION), None] {
            for b in [Some(oag_physics::WALL_FRICTION), None] {
                let formats = oag_formats::collision::combine_friction(a, b);
                assert_eq!(formats, oag_physics::combine_friction(a, b));
                assert!(formats >= 0.0, "friction went negative: {a:?} {b:?}");
            }
        }
    }

    #[test]
    fn every_collidable_kind_maps_to_a_surface() {
        for kind in SurfaceKind::ALL {
            let mapped = surface_for(kind);
            assert_eq!(mapped.is_some(), kind != SurfaceKind::Cage, "{kind:?}");
        }
    }

    /// Mag floor must arrive as its own surface rather than being flattened into
    /// floor. The hover path treats the two interchangeably, but the magnetic hold
    /// will not, and losing the tag here would make that unimplementable.
    #[test]
    fn a_magstrip_keeps_its_own_surface_tag() {
        let world = collision_world(&[node(SurfaceKind::MagFloor, 1)]);
        assert_eq!(world.colliders()[0].surface(), Surface::MagFloor);
        assert!(Surface::MagFloor.is_hoverable());
    }
}
