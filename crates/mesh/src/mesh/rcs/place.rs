//! Where a chunk's coordinates live, and which node places it.
//!
//! Split out of [`super`] under the 1,000-line rule in
//! `scripts/check-file-size.py`. One question, asked three ways: what a `Mesh`
//! node's payload says about its chunk ([`node_geometry`]), whether the chunk
//! it names is baked in world space anyway ([`is_world_baked`]), and which
//! hashes the node pass therefore leaves to the world-space one
//! ([`referenced`]). A move, with no behaviour change.

use oag_core::math::{Mat4, Vec3};
use oag_rcs::rcsmodel;
use oag_vex::vex;

/// The bounding box and chunk hash a PS3 `Mesh` node's payload carries.
///
/// The layout the PSP uses for geometry, with the geometry taken out: the box
/// pair at `+0x10`/`+0x20` is measured `min <= max` on 1,638 of 1,638 nodes,
/// and the word at `+0x30` is the `.rcsmodel` chunk's own first word.
pub(super) fn node_geometry(
    payload: &[u8],
    order: oag_formats::ByteOrder,
) -> Option<(u32, [f32; 3], [f32; 3])> {
    if payload.len() < 0x34 {
        return None;
    }
    let read3 = |at: usize| std::array::from_fn(|i| order.f32(payload, at + i * 4));
    Some((order.u32(payload, 0x30), read3(0x10), read3(0x20)))
}

/// Whether a `Mesh` node's chunk is baked in world space despite the node
/// naming it: the node's own authored box, carried through its own `to_world`
/// transform, lands within [`WORLD_BAKE_TOLERANCE`] of the chunk's own bias.
///
/// # The pad precedent, generalised
///
/// **Confidence 88.** A `Weapon Pad`/`Speedup Pad` node carries a chunk hash
/// at the mesh payload's own `+0x30`, but the chunk it names is baked in
/// world space anyway - `docs/formats/rcsmodel.md`, "The pads are second-pass
/// geometry with a first-pass-shaped reference". The node pass only ever
/// draws the `Mesh` class, so those chunks structurally never reach it and
/// [`referenced`] excluding their hashes is enough on its own.
///
/// **The same pattern recurs on ordinary `Mesh` nodes, and there it is not
/// enough to fix in `referenced` alone.** A `wohdtrack_*` node - and a
/// handful of other names, `startscreenShape`, `sign_emissive_glow`'s
/// billboards, `cf_startbeam_glow` among them - addresses a chunk baked the
/// same way, but the node pass *does* iterate these (same class as every
/// ordinary prop), so leaving [`build`]'s node loop unchanged risks drawing
/// the chunk **twice** once [`referenced`] stops excluding its hash: once
/// through the node transform, wherever `submesh_fits`'s loose tolerance
/// happens to pass by coincidence on a small chunk, and once at identity
/// through the world-space pass. Measured, not assumed:
/// `crates/render/examples/hd_double_submit_check.rs` found the node path
/// still succeeding on a real fraction of the excluded hashes across
/// `12_sol_2`, `15_anulpha_pass`, `10_sebenco_climb`, `05_ubermall`,
/// `01_vineta_k`, `02_track` and `03_track`. So this predicate is shared: the
/// node loop in [`build`] skips a world-baked chunk before it ever reaches
/// [`Mesh::solve_stride`] or `submesh_fits`, exactly where a pad's different
/// class already keeps it out, and [`referenced`] excludes the same hash so
/// the world-space pass in [`build_scene`] is where it draws instead.
///
/// **Structural, not tuned.** A chunk this applies to reads 0.01-0.02 world
/// units away - quantisation noise on an exact match - and every ordinary
/// node-local chunk measured is at least an order of magnitude further; on
/// the census that found this (`crates/render/examples/hd_floor_census.rs`)
/// the next-closest non-match was 2.5 units and most sit in the tens or
/// hundreds. One world unit sits in the gap between the two clusters with
/// room either side, not on either cluster's edge.
pub(super) fn is_world_baked(
    mesh: &rcsmodel::Mesh,
    min: [f32; 3],
    max: [f32; 3],
    to_world: Mat4,
) -> bool {
    // **The disc answers this outright, and the geometry below was a guess at
    // it.** A chunk's `+0x07` byte says which space its positions are in - see
    // `rcsmodel::Space`, which carries the measurement. Where the two differ,
    // the chunk's own bias sides with the byte: on 5,724 chunks the byte calls
    // node-local and this test called baked, the median bias is 1.2 units, and
    // on the 775 the other way it is 544.3. The test stays for a value neither
    // 1 nor 2, which nothing on this disc has.
    match mesh.space {
        rcsmodel::Space::World => return true,
        rcsmodel::Space::Node => return false,
        rcsmodel::Space::Unknown(_) => {}
    }
    let centre = Vec3::new(
        (min[0] + max[0]) / 2.0,
        (min[1] + max[1]) / 2.0,
        (min[2] + max[2]) / 2.0,
    );
    let world_centre = to_world.transform_point3(centre);
    world_centre.distance(Vec3::from_array(mesh.bias)) < WORLD_BAKE_TOLERANCE
}

/// How close a `Mesh` node's own authored box, carried through its own
/// transform chain into world space, must land to a chunk's own bias before
/// [`is_world_baked`] treats the chunk as baked in world space rather than
/// node-local. See [`is_world_baked`] for the evidence behind the number.
const WORLD_BAKE_TOLERANCE: f32 = 1.0;

/// The chunk hashes the node pass consumes: `+0x30` of every `Mesh` node -
/// **except a chunk that is baked in world space despite the node naming it**,
/// see [`is_world_baked`].
///
/// **Only what [`build`] draws, and that reverses an earlier over-collection.**
/// This used to scan every node's whole payload for anything shaped like a
/// chunk hash, on the theory that a hash wrongly counted as referenced would
/// still be drawn through its node. Measured false: a `Weapon Pad` and a
/// `Speedup Pad` node each carry a chunk hash at the mesh payload's own
/// `+0x30`, the node pass only draws the `Mesh` class, and so all 423 pad
/// chunks on the disc were drawn by nobody. Their positions are baked in world
/// space like every other circuit chunk - each pad chunk's centre sits beside
/// its node's world translation, never at the origin and never doubled -
/// so the world-space pass is the right place for them, and the way to hand
/// them to it is to stop counting them here.
pub(super) fn referenced(
    data: &[u8],
    nodes: &[vex::Node],
    mesh_class: u32,
    model: &rcsmodel::Model,
) -> Vec<u32> {
    let order = vex::byte_order(data);
    let world = vex::world_transforms(data, nodes);
    nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| node.class_id == mesh_class)
        .filter_map(|(index, node)| {
            let (hash, min, max) = node_geometry(&data[node.payload()], order)?;
            if let Some(mesh) = model.mesh(hash) {
                let to_world = Mat4::from_cols_array(&world[index]);
                if is_world_baked(mesh, min, max, to_world) {
                    return None;
                }
            }
            Some(hash)
        })
        .collect()
}
