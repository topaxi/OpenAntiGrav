//! The `Weapon Pad` chunk hashes a `.vex`'s node tree names, split out of
//! [`super::build_scene`]'s world-space pass so a caller can gate them apart
//! from the rest of the circuit.
//!
//! # Why this needs its own pass
//!
//! A `Weapon Pad` node names its chunk at the mesh payload's own `+0x30`
//! exactly like an ordinary `Mesh` node does, but the chunk it names is baked
//! in world space regardless - `docs/formats/rcsmodel.md`, "The pads are
//! second-pass geometry with a first-pass-shaped reference". [`super::referenced`]
//! only ever walks `Mesh`-class nodes, so a `Weapon Pad` node's hash was never
//! in the set it excludes, and the chunk fell into the same "no node
//! references this, draw it in world space" bucket as the road and every
//! other second-pass chunk. Left there, a weapon pad chunk is
//! indistinguishable from ordinary track geometry and drew in every mode -
//! see `docs/gameplay/race-modes.md` for the report that caught it.
//!
//! `Speedup Pad` chunks are the same shape and are deliberately left alone:
//! nothing gates them by mode, on this disc or the original's.

use oag_formats::{rcsmodel, vex};

use super::{DrawCall, Model};

/// The chunk hashes every node of `pad_class` names, restricted to hashes this
/// `.rcsmodel` actually carries.
///
/// Not filtered through [`super::is_world_baked`] the way [`super::referenced`]
/// is: every `Weapon Pad` chunk measured on the disc is world-baked already
/// (`docs/formats/rcsmodel.md`), so the only thing left to check here is
/// whether the hash names a chunk this file has at all.
pub(super) fn hashes(
    data: &[u8],
    nodes: &[vex::Node],
    pad_class: u32,
    model: &rcsmodel::Model,
) -> Vec<u32> {
    let order = vex::byte_order(data);
    nodes
        .iter()
        .filter(|node| node.class_id == pad_class)
        .filter_map(|node| {
            let (hash, ..) = super::node_geometry(&data[node.payload()], order)?;
            model.mesh(hash).is_some().then_some(hash)
        })
        .collect()
}

/// An empty model that shares `out`'s texture setup - the skeleton
/// [`super::build_scene`]'s weapon-pad chunks emit into, so a material
/// resolves to the same texture, lightmap and shader role it would in the
/// circuit's own model.
///
/// **Deliberately leaves `node_vertex_ranges` empty** - see its own doc
/// comment on `Model`. This pass has no per-node correspondence for a
/// world-baked chunk to build one from, so the model this returns draws
/// (the bug this module fixes) but never cycles its ready/cooling-down
/// colour the way a PSP-shaped weapon-pad model does.
pub(super) fn skeleton(label: &str, out: &Model) -> Model {
    Model {
        textures: out.textures.clone(),
        lightmaps: out.lightmaps.clone(),
        material_slots: out.material_slots.clone(),
        material_variants: out.material_variants.clone(),
        // **Carried because `material_slots` is.** A slot's word holds its
        // index into this table in its high half, so copying the roles without
        // the table leaves a pad's vertices indexing an empty one - which
        // reads as an all-zero entry and silently drops the glow rather than
        // failing. See `mesh::slots::MATERIAL_SHIFT`.
        emissive: out.emissive.clone(),
        vertex_colour_is_light: out.vertex_colour_is_light,
        // Carried for the same reason: a pad chunk resolves to the circuit's
        // own material table, so a cutout among them must be tested against
        // the reference that table authors and not against the shader's
        // PSP default. See `super::cutout`.
        alpha_test_ref: out.alpha_test_ref,
        ..Model::none(label)
    }
}

/// `None` for a chunk-less pad model, otherwise the same finishing
/// [`super::build_scene`] gives its own circuit model: face normals derived
/// where the file authored none, and a bounding sphere to frame it by.
pub(super) fn finish(mut pad_out: Model) -> Option<Model> {
    if pad_out.indices.is_empty() {
        return None;
    }
    super::face_normals(&mut pad_out);
    let (centre, radius) = super::bounding_sphere(&pad_out.vertices);
    pad_out.centre = centre;
    pad_out.radius = radius;
    Some(pad_out)
}

/// Puts a weapon-pad model back onto the circuit it was split out of,
/// offsetting its indices and draw ranges past what `out` already holds.
///
/// The one caller is [`super::scene_from`]: every browser or diagnostic goes
/// through it rather than [`super::build_scene`] directly, wants the whole
/// disc on screen, and has no `Mode` to gate the split by.
pub(super) fn merge_back(out: &mut Model, extra: Model) {
    let vertex_offset = u32::try_from(out.vertices.len()).unwrap_or(u32::MAX);
    let index_offset = u32::try_from(out.indices.len()).unwrap_or(u32::MAX);
    out.vertices.extend(extra.vertices);
    out.indices
        .extend(extra.indices.into_iter().map(|i| i + vertex_offset));
    let shift = |mut draws: Vec<DrawCall>| {
        for draw in &mut draws {
            draw.range = (draw.range.start + index_offset)..(draw.range.end + index_offset);
        }
        draws
    };
    out.draws.extend(shift(extra.draws));
    out.alpha_tested_draws
        .extend(shift(extra.alpha_tested_draws));
    out.transparent_draws.extend(shift(extra.transparent_draws));
    out.mesh_count += extra.mesh_count;
}
