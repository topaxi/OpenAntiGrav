//! A billboard's UV keys: the texture-transform block the `.vex` beside a
//! Vita or PS4 `.rcsmodel` authors per mesh.
//!
//! Wipeout HD animates a billboard material's `uvOffset`/`uvScale` with an
//! Edge Animation curve inside the `.rcsmodel` ([`super::super::curve_track`]).
//! 2048's and Omega's models carry no curve: their exporter wrote the motion
//! into each `Mesh` node of the `.vex` as Pulse's keyframe block
//! (`oag_vex::vex::external_mesh_tex_transforms`), and the engine points the
//! material's two uniforms at a per-mesh instance the block fills
//! (`docs/ghidra/functions/ps4-omega-eu/billboards.md`, "The UV keys").
//!
//! So the track belongs to the **mesh**, and reaches only a material that
//! declares `uvOffset` or `uvScale`: the redirect is by those two name hashes.

use oag_rcs::rcsmaterial::name_hash;
use oag_rcs::rcsmodel::psp2;
use oag_vex::vex;

use super::Report;
use crate::mesh::{ANIM_TRACK_LIMIT, AnimTrack};

/// Per mesh object of the model: the anim slot its vertices ride, `0` for none.
#[derive(Default)]
pub(super) struct Plan {
    slots: Vec<u32>,
}

impl Plan {
    /// The slot a vertex of `mesh` drawn with `material` rides, or `None` where
    /// the mesh has no keys or the material no uniform for them to reach.
    pub fn anim(
        &self,
        mesh: Option<usize>,
        material: Option<&psp2::material::Material>,
    ) -> Option<u32> {
        let slot = *self.slots.get(mesh?)?;
        let material = material?;
        let reads = |name: &str| material.param(name_hash(name)).is_some();
        (slot != 0 && (reads("uvOffset") || reads("uvScale"))).then_some(slot)
    }
}

/// Joins each mesh object of `decoded` to its `Mesh` node in `vex_blob` by the
/// hash both carry, and gives the meshes whose node authors keys a slot in
/// `tracks`. Equal blocks share one slot; a block past the shader's table
/// draws still.
pub(super) fn plan(
    decoded: &psp2::Model,
    vex_blob: &[u8],
    tracks: &mut Vec<AnimTrack>,
    report: &mut Report,
) -> Plan {
    let (Ok(nodes), Ok(classes)) = (vex::nodes(vex_blob), vex::classes_of(vex_blob)) else {
        return Plan::default();
    };
    let Some(mesh_class) = classes.mesh else {
        return Plan::default();
    };
    let keyed: Vec<(u32, vex::TexTransform)> = nodes
        .iter()
        .filter(|node| node.class_id == mesh_class)
        .filter_map(|node| {
            let payload = vex_blob.get(node.payload())?;
            // The engine copies the first block alone into the mesh's instance.
            let keys = vex::external_mesh_tex_transforms(payload)
                .into_iter()
                .next()??;
            Some((vex::external_mesh_hash(payload)?, keys))
        })
        .collect();
    let mut first_slot_of: Vec<(vex::TexTransform, u32)> = Vec::new();
    let slots = decoded
        .scene
        .meshes
        .iter()
        .map(|mesh| {
            let Some((_, keys)) = keyed.iter().find(|(hash, _)| *hash == mesh.vex_hash) else {
                return 0;
            };
            if let Some((_, slot)) = first_slot_of.iter().find(|(known, _)| known == keys) {
                report.uv_keyed_meshes += 1;
                return *slot;
            }
            if tracks.len() + 1 >= ANIM_TRACK_LIMIT {
                return 0;
            }
            tracks.push(AnimTrack::Psp(keys.clone()));
            let slot = u32::try_from(tracks.len()).unwrap_or(0);
            first_slot_of.push((keys.clone(), slot));
            report.uv_keyed_meshes += 1;
            slot
        })
        .collect();
    Plan { slots }
}
