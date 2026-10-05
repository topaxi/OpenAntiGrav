//! [`split`]: dividing one built model into the node groups its `.vex` authors.
//!
//! **The mechanism a Wipeout HD engine flare needs.** Its `engineflare.vex`
//! hangs five shapes under a `Transform` named `EF_Main` and five more under one
//! named `EF_Boost`, and which of the two a shape sits under is the *only*
//! thing in the file that says when it is drawn. `rcs::build` flattens all ten
//! into one buffer pair - correctly, since they share a material - so telling
//! the always-on flame from the boost plume is a question about the node tree,
//! answered after the build rather than during it.
//!
//! Nothing here re-reads geometry. [`DrawCall::node`] already carries the index
//! of the `.vex` node each run came from, so the split is a walk up the parent
//! chain and a filter over the draw lists; the vertex and index buffers are
//! shared verbatim by every part, and a range no part keeps is simply never
//! drawn.

use super::*;

/// One part of a split model: the group it was named for and the model itself.
#[derive(Debug, Clone)]
pub struct Part {
    /// The group node's own name, e.g. `EF_Main`.
    pub group: String,
    /// The geometry hanging under it, sharing the source's buffers.
    pub model: Model,
}

impl Part {
    /// How many triangles this part draws, over all three lists.
    #[must_use]
    pub fn triangles(&self) -> usize {
        [
            &self.model.draws,
            &self.model.alpha_tested_draws,
            &self.model.transparent_draws,
        ]
        .into_iter()
        .flatten()
        .map(|d| (d.range.end - d.range.start) as usize / 3)
        .sum()
    }
}

/// Splits `model` into one [`Part`] per name in `groups`, by the node tree in
/// `data`.
///
/// A group that the file does not author, or that no drawn geometry hangs
/// under, comes back with an **empty** part rather than being dropped: the
/// caller's report distinguishes "this craft authors no boost group" from "this
/// build could not read one", and a silently shorter vector cannot.
///
/// Geometry under none of the named groups is not returned at all. That is the
/// intended behaviour and not a loss - on a flare model the only nodes outside
/// them are the root and the file's dummy `Texture` node, neither of which
/// draws.
///
/// # Errors
///
/// Propagates a `.vex` whose node tree will not walk. A model built from a
/// *different* file than `data` is not detectable here and is the caller's
/// contract to keep: the node indices would resolve, to the wrong names.
pub fn split(model: &Model, data: &[u8], groups: &[&str]) -> anyhow::Result<Vec<Part>> {
    let nodes = vex::nodes(data).context("walking the node tree")?;
    let owner = owners(&nodes, groups);
    Ok(groups
        .iter()
        .enumerate()
        .map(|(g, name)| Part {
            group: (*name).to_string(),
            model: part(model, &owner, g, name),
        })
        .collect())
}

/// [`split`] for a 2048 model, whose draws carry no `.vex` node.
///
/// **Joined by name instead.** A 2048 `.rcsmodel` names each mesh object after
/// its shape node (`ef_OuterShape`), and [`DrawCall::chunk`] is that mesh
/// object's index, so a draw's group is the group of the `.vex` node of the
/// same name. A mesh whose name no node carries belongs to no group and is
/// not returned, the same rule [`split`] applies to a node outside them.
///
/// # Errors
///
/// Propagates a `.vex` whose node tree will not walk, or a `.rcsmodel` that
/// is not 2048's container.
pub fn split_psp2(
    model: &Model,
    model_blob: &[u8],
    vex_blob: &[u8],
    groups: &[&str],
) -> anyhow::Result<Vec<Part>> {
    let nodes = vex::nodes(vex_blob).context("walking the node tree")?;
    let owner = owners(&nodes, groups);
    let decoded = oag_rcs::rcsmodel::psp2::parse(model_blob)
        .map_err(|e| anyhow::anyhow!("the .rcsmodel: {e}"))?;
    let by_mesh: Vec<Option<usize>> = decoded
        .scene
        .meshes
        .iter()
        .map(|mesh| {
            nodes
                .iter()
                .position(|n| {
                    n.name
                        .as_deref()
                        .is_some_and(|n| n.eq_ignore_ascii_case(&mesh.name))
                })
                .and_then(|i| owner[i])
        })
        .collect();
    Ok(groups
        .iter()
        .enumerate()
        .map(|(g, name)| Part {
            group: (*name).to_string(),
            model: part_by(model, name, |call| {
                call.chunk
                    .and_then(|c| by_mesh.get(c as usize).copied().flatten())
                    == Some(g)
            }),
        })
        .collect())
}

/// For each node, which of `groups` it descends from - itself included, so a
/// group node that carried geometry directly would count as its own.
fn owners(nodes: &[vex::Node], groups: &[&str]) -> Vec<Option<usize>> {
    // Resolved from the root down, so each node reads its parent's already
    // computed answer instead of walking the chain again. `vex::nodes` returns
    // the tree in file order, in which a parent always precedes its children -
    // the same property `world_transforms` composes matrices on.
    let mut owner: Vec<Option<usize>> = vec![None; nodes.len()];
    for (index, node) in nodes.iter().enumerate() {
        let mine = node
            .name
            .as_deref()
            .and_then(|name| groups.iter().position(|g| g.eq_ignore_ascii_case(name)));
        owner[index] = mine.or_else(|| node.parent.and_then(|p| owner[p]));
    }
    owner
}

/// One group's own model: the source's buffers, and only the draws whose node
/// belongs to it.
fn part(model: &Model, owner: &[Option<usize>], group: usize, name: &str) -> Model {
    part_by(model, name, |call| {
        call.node
            .and_then(|n| owner.get(n as usize).copied().flatten())
            == Some(group)
    })
}

/// [`part`] with the membership test supplied.
fn part_by(model: &Model, name: &str, keep: impl Fn(&DrawCall) -> bool) -> Model {
    let mut out = model.clone();
    out.label = format!("{} [{name}]", model.label);
    out.draws.retain(&keep);
    out.alpha_tested_draws.retain(&keep);
    out.transparent_draws.retain(&keep);
    // Recomputed rather than inherited: a part's bounding sphere is what the
    // viewer frames it with and what a bounds check would cull it by, and the
    // whole model's sphere is wrong for both. Over the vertices this part's
    // ranges actually name, not over the shared buffer.
    let (centre, radius) = sphere(&out);
    out.centre = centre;
    out.radius = radius;
    out.mesh_count = mesh_count(&out);
    out
}

/// The bounding sphere of the vertices this model's own draw ranges reach.
fn sphere(model: &Model) -> ([f32; 3], f32) {
    let mut sum = oag_core::math::Vec3::ZERO;
    let mut count = 0usize;
    let reached = |model: &Model| -> Vec<u32> {
        [
            &model.draws,
            &model.alpha_tested_draws,
            &model.transparent_draws,
        ]
        .into_iter()
        .flatten()
        .flat_map(|d| {
            model.indices[d.range.start as usize..d.range.end as usize]
                .iter()
                .copied()
        })
        .collect()
    };
    let indices = reached(model);
    for &i in &indices {
        let Some(v) = model.vertices.get(i as usize) else {
            continue;
        };
        sum += oag_core::math::Vec3::from_array(v.position);
        count += 1;
    }
    if count == 0 {
        return ([0.0, 0.0, 0.0], 0.0);
    }
    let centre = sum / count as f32;
    let radius = indices
        .iter()
        .filter_map(|&i| model.vertices.get(i as usize))
        .map(|v| (oag_core::math::Vec3::from_array(v.position) - centre).length())
        .fold(0.0f32, f32::max);
    (centre.to_array(), radius)
}

/// How many distinct `.vex` nodes this part still draws from.
fn mesh_count(model: &Model) -> usize {
    let mut seen: Vec<u32> = [
        &model.draws,
        &model.alpha_tested_draws,
        &model.transparent_draws,
    ]
    .into_iter()
    .flatten()
    .filter_map(|d| d.node)
    .collect();
    seen.sort_unstable();
    seen.dedup();
    seen.len()
}

#[cfg(test)]
mod tests;
