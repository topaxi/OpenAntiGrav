//! `Speedup Pad`/`Weapon Pad` geometry, from the `.rcsmodel` beside the
//! `.vex`, through their own node-ordered pass.
//!
//! # Why this needs its own pass
//!
//! A `Weapon Pad`/`Speedup Pad` node names its chunk at the mesh payload's
//! own `+0x30` exactly like an ordinary `Mesh` node does, but the chunk it
//! names is baked in world space regardless -
//! `docs/formats/rcsmodel.md`, "The pads are second-pass geometry with a
//! first-pass-shaped reference". [`super::referenced`] only ever walks
//! `Mesh`-class nodes, so a pad node's hash was never in the set it
//! excludes, and the chunk fell into the same "no node references this,
//! draw it in world space" bucket as the road and every other second-pass
//! chunk. Left there, a pad chunk is indistinguishable from ordinary track
//! geometry and drew in every mode - see `docs/gameplay/race-modes.md` for
//! the report that caught it on `Weapon Pad` first.
//!
//! [`super::build_scene`]'s own world-space pass excludes both classes'
//! chunks via [`pad_chunk_hashes`] rather than drawing them into a shared
//! "everything nobody claims" bucket the way it used to for `Weapon Pad`
//! alone - that bucket has no per-node correspondence to build
//! `Model::node_vertex_ranges` from, and without that field
//! `Drawable::tint_weapon_pads` has nothing to zip
//! against. [`build_pads`]/[`build_weapon_pads`] draw the excluded chunks
//! through [`build_pad_class`] instead: one node-ordered pass per class,
//! walking [`oag_vex::pads::volumes`]'s own node order so
//! `oag_raceplay::load`'s trigger list and this module's vertex ranges
//! stay lined up one entry each.

use anyhow::{Context, Result, bail};
use oag_rcs::rcsmodel;
use oag_vex::vex;

use crate::mesh::slots;

use super::emissive::EMISSIVE_LIMIT;
use super::{
    Geometry, MaterialSetup, Model, Report, Textures, anim_node, authored, bounding_sphere,
    declares_no_texcoord, emit, face_normals, material_setup, node_geometry, pad_ne, surface,
};

/// Every chunk hash a node of `class_id` names at its mesh payload's own
/// `+0x30`, in no particular order - a pure lookup, unlike [`super::referenced`],
/// which also excludes a chunk the *ordinary* node pass would double-draw. A
/// pad chunk is never a candidate for that pass in the first place (its class
/// never matches `mesh_class`), so there is nothing here to exclude.
///
/// Not filtered through [`super::is_world_baked`] either: every pad chunk
/// measured on the disc is world-baked already (`docs/formats/rcsmodel.md`),
/// so a hash naming no chunk this file has is the only thing worth checking
/// for, and [`build_pad_class`] already skips those on its own.
pub(super) fn pad_chunk_hashes<'a>(
    data: &'a [u8],
    nodes: &'a [vex::Node],
    order: oag_formats::ByteOrder,
    class_id: u32,
) -> impl Iterator<Item = u32> + 'a {
    nodes
        .iter()
        .filter(move |node| node.class_id == class_id)
        .filter_map(move |node| node_geometry(&data[node.payload()], order).map(|(hash, ..)| hash))
}

/// Whether a material file is one of the two pad programs: `weapon_pads`,
/// which `talons_junction` and `02_track` also author their speed pads
/// under, and `speedup_material`, which `amphiseum` does.
pub(super) fn is_pad_material(name: &str) -> bool {
    let leaf = name.rsplit('/').next().unwrap_or(name);
    matches!(
        leaf,
        "weapon_pads.rcsmaterial" | "speedup_material.rcsmaterial"
    )
}

/// Whether a material file is `diffuse_normal_specular_emmissive` (sic): the
/// start line, the pit lane and the rail walls (`ds_sf`, `ds_pit`, `ds_wall`,
/// `ds_rail`). Its fragment program reads the `_ne` file the way the pad
/// programs do - the RGB as a tangent normal and the alpha as the mask of an
/// additive `alpha * colour` term, the colour being the material's own
/// authored `0x7611a2d8` - so it is bound by the same reading, which
/// [`super::pad_ne::pad_ne`] only accepts after matching the program itself.
pub(super) fn is_light_bar_material(name: &str) -> bool {
    name.rsplit('/').next().unwrap_or(name) == "diffuse_normal_specular_emmissive.rcsmaterial"
}

/// Every chunk hash the world-space pass leaves to a node: the ones a `Mesh`
/// node names ([`super::referenced`]) and the ones either pad class names.
///
/// **Both pad classes' chunks are excluded here too, not only the ordinary
/// `Mesh` ones `referenced` names.** They would otherwise fall through to the
/// unreferenced-chunk loop in [`super::build_scene`] exactly the way `referenced`'s
/// own doc comment describes for an ordinary node - a `Weapon Pad`/`Speedup
/// Pad` node's chunk hash never matches `mesh_class`, so `referenced` never
/// even looks at it. [`build_pads`]/[`build_weapon_pads`] draw
/// these same chunks through their own node-ordered pass instead, and this is
/// what stops a pad drawing twice once a caller uses both - see that pair's own
/// doc comment for why a separate pass exists at all.
///
/// One function for [`super::build_scene`] and the pad passes' report
/// ([`world_pass_pad_chunks`]), so the chunks the report counts as drawn
/// here are the chunks the scene pass leaves for itself.
pub(super) fn placed_hashes(
    data: &[u8],
    nodes: &[vex::Node],
    classes: Option<vex::classes::Classes>,
    order: oag_formats::ByteOrder,
    model: &rcsmodel::Model,
) -> Vec<u32> {
    let mut placed = match classes.and_then(|c| c.mesh) {
        Some(mesh_class) => super::referenced(data, nodes, mesh_class, model),
        None => Vec::new(),
    };
    for class in [
        classes.and_then(|c| c.speedup_pad),
        classes.and_then(|c| c.weapon_pad),
    ]
    .into_iter()
    .flatten()
    {
        placed.extend(pad_chunk_hashes(data, nodes, order, class));
    }
    placed
}

/// The chunks [`super::build_scene`]'s world-space pass draws on a pad
/// material, and their triangles: what a pad is on a circuit whose pad nodes
/// name no chunk. `named` is every hash a pad node of either class names.
///
/// The same routing [`bind_scene_pad_masks`] does for the `_ne` mask, read
/// for the report; it chooses nothing, since the scene pass draws these
/// chunks whether or not this counts them.
pub(super) fn world_pass_pad_chunks(model: &rcsmodel::Model, named: &[u32]) -> (usize, usize) {
    let mut chunks = 0;
    let mut triangles = 0;
    for chunk in model.meshes.iter().filter(|c| !named.contains(&c.hash)) {
        if !chunk.surfaces().any(|s| {
            model
                .materials
                .get(s.material as usize)
                .is_some_and(|m| is_pad_material(&m.name))
        }) {
            continue;
        }
        chunks += 1;
        triangles += chunk
            .surfaces()
            .flat_map(|s| s.submeshes.iter())
            .filter(|sub| sub.vertex_count != 0 && sub.index_count != 0)
            .map(|sub| sub.index_count / 3)
            .sum::<usize>();
    }
    (chunks, triangles)
}

/// Binds the `_ne` mask to the pad materials [`super::build_scene`]'s
/// unreferenced-chunk pass is about to draw, and to every chunk of the
/// emissive family ([`is_light_bar_material`]): the speed pads of the four
/// original circuits, routed by material because their `Speedup Pad` nodes
/// name a hash no chunk carries (18, 16, 17 and 15 nodes against as many
/// chunks on a pad material). A chunk an addressed pad node owns is in
/// `placed`, so the 12 circuits whose nodes resolve bind nothing here.
pub(super) fn bind_scene_pad_masks(
    model: &rcsmodel::Model,
    placed: &[u32],
    out: &mut Model,
    textures: Textures<'_>,
    report: &mut Report,
) {
    let mut pad_slots = vec![false; model.materials.len()];
    for chunk in model.meshes.iter() {
        let unplaced = !placed.contains(&chunk.hash);
        for surface in chunk.surfaces() {
            let Some(flag) = pad_slots.get_mut(surface.material as usize) else {
                continue;
            };
            let name = &model.materials[surface.material as usize].name;
            // A pad is routed only when no pad node owns its chunk; the
            // start line's own material is ordinary scenery and is routed
            // wherever it is drawn.
            *flag |= (unplaced && is_pad_material(name)) || is_light_bar_material(name);
        }
    }
    super::mag_wave::merge(
        &mut out.pad_masks,
        pad_ne::pad_ne(
            model,
            &out.material_variants,
            textures,
            &mut out.material_slots,
            &mut out.emissive,
            Some(&pad_slots),
            report,
        ),
    );
}

/// The track's `Speedup Pad` geometry, from the `.rcsmodel` beside the
/// `.vex` - the PS3 counterpart of [`super::super::build_pads`].
///
/// See [`build_weapon_pads`] for the shared implementation and the doc
/// comment explaining why this cannot simply read out of
/// [`super::build_scene`]'s own world-space pass.
pub fn build_pads(
    label: &str,
    data: &[u8],
    model_blob: &[u8],
    textures: Textures<'_>,
) -> Result<(Model, Report)> {
    build_pad_class(label, data, model_blob, textures, |c| c.speedup_pad)
}

/// The track's `Weapon Pad` geometry, the same way as [`build_pads`] - the
/// PS3 counterpart of [`super::super::build_weapon_pads`].
///
/// # Why this cannot reuse `build_scene`'s world-space pass
///
/// A pad chunk *is* drawn somewhere by [`super::build_scene`] unless
/// excluded via [`pad_chunk_hashes`] - but that pass walks `.rcsmodel`
/// chunks in **file** order, with no node identity attached, into one shared
/// buffer with every other track chunk. `Drawable::tint_weapon_pads` needs
/// the opposite: one [`Model::node_vertex_ranges`] entry per pad **node**, in
/// the same **tree** order [`oag_vex::pads::volumes`] walks to build the
/// trigger list `oag_raceplay::Race` drives ready/cooling state from - see
/// that function and `crates/raceplay/src/pads.rs`. So this is a second,
/// dedicated pass over exactly the nodes of one pad class.
///
/// One [`Model::node_vertex_ranges`] entry per node of the wanted class,
/// pushed **unconditionally**, empty for a node whose chunk does not resolve
/// or decode. That is the same "never skip a slot" rule
/// `super::super::build_class` uses, and it is what keeps the array lined up
/// with [`oag_vex::pads::volumes`]'s own count, one entry each.
///
/// Positions are already in world space - a `Weapon Pad`/`Speedup Pad`
/// chunk's coordinates ignore its node the same way every other world-baked
/// chunk [`super::build_scene`] draws does - so this bakes each node's chunk
/// with the identity transform rather than the node's own `to_world`, and
/// the caller draws the resulting model at `Mat4::IDENTITY`, exactly as it
/// already draws [`super::super::build_pads`]'s PSP output.
///
/// # Errors
///
/// As [`super::build_scene`]. Empty, rather than an error, for a `.vex`
/// version this project has not recovered the wanted class id for - the
/// same "authors none" answer `super::super::build_optional_class` gives.
pub fn build_weapon_pads(
    label: &str,
    data: &[u8],
    model_blob: &[u8],
    textures: Textures<'_>,
) -> Result<(Model, Report)> {
    let (mut model, report) = build_pad_class(label, data, model_blob, textures, |c| c.weapon_pad)?;
    each_node_its_own_glow(&mut model);
    Ok((model, report))
}

/// Gives every node's `_ne` glow its own [`Model::emissive`] entry, a copy of
/// the one the node's material resolved to.
///
/// A weapon pad's bar colour is **per pad** at run time: HD's own pad object
/// carries a cycle position and a cooldown, and the program's inline constant
/// is written from them (`oag_title::weapon_pad`). The glow table is shared by
/// value (`pad_ne` deduplicates), so every pad of one material would otherwise
/// read the same entry; copying it per node lets a caller rewrite one pad's
/// tint without touching another's. The copies hold the authored value, so a
/// caller that never rewrites them draws what it drew before.
///
/// A node whose vertices carry no glow entry, or a table with no room left
/// ([`EMISSIVE_LIMIT`]), is left on the shared one.
fn each_node_its_own_glow(model: &mut Model) {
    for range in model.node_vertex_ranges.clone() {
        let Some(first) = model.vertices.get(range.start as usize) else {
            continue;
        };
        let shared = slots::material_index(first.slots) as usize;
        if first.slots & slots::PAD_NE == 0 || shared == 0 {
            continue;
        }
        let Some(layer) = model.emissive.get(shared - 1).copied() else {
            continue;
        };
        if model.emissive.len() + 1 >= EMISSIVE_LIMIT {
            break;
        }
        model.emissive.push(layer);
        let own = u32::try_from(model.emissive.len()).unwrap_or(0);
        for vertex in &mut model.vertices[range.start as usize..range.end as usize] {
            vertex.slots = (vertex.slots & slots::ROLE_MASK) | (own << slots::MATERIAL_SHIFT);
        }
    }
}

pub(super) fn build_pad_class(
    label: &str,
    data: &[u8],
    model_blob: &[u8],
    textures: Textures<'_>,
    pick: fn(vex::classes::Classes) -> Option<u32>,
) -> Result<(Model, Report)> {
    if !vex::has_magic(data) {
        bail!("{label} is not a .vex file (no VEXX magic)");
    }
    let classes = vex::classes_of(data).with_context(|| format!("{label}: class table"))?;
    let Some(class_id) = pick(classes) else {
        return Ok((Model::none(label), Report::default()));
    };
    let model = rcsmodel::Model::parse(model_blob)
        .map_err(|e| anyhow::anyhow!("{label}: the .rcsmodel beside it: {e}"))?;
    let nodes = vex::nodes(data).context("walking the node tree")?;
    let order = vex::byte_order(data);

    let mut out = Model::none(label);
    let mut report = Report::default();
    let MaterialSetup {
        textures: skins,
        lightmaps: seconds,
        material_slots,
        material_specular_exponent,
        material_variants,
        emissive,
        alpha_test_ref,
        material_anim,
        anim_tracks,
        ..
    } = material_setup(&model, model_blob, textures, &mut report);
    out.textures = skins;
    out.lightmaps = seconds;
    out.material_slots = material_slots;
    out.emissive = emissive;
    // The `_ne` mask, third beside the lightmap - see `pad_ne`.
    out.pad_masks = pad_ne::pad_ne(
        &model,
        &material_variants,
        textures,
        &mut out.material_slots,
        &mut out.emissive,
        None,
        &mut report,
    );
    out.material_specular_exponent = material_specular_exponent;
    out.material_variants = material_variants;
    out.alpha_test_ref = alpha_test_ref;
    out.material_anim = material_anim;
    out.anim_tracks = anim_tracks;
    // **`true`, exactly like every other PS3 model this crate builds.** A pad
    // chunk's `in.colour` is HD's baked per-vertex *light*, the term the
    // fragment program adds inside its authored lighting sum - see
    // [`super::emit`] and `shaders/shade.wesl`'s `lit_texel`. It is not a tint, and a
    // pad's own colour is not in it: `12_sol_2`'s ten `Speedup Pad` and eight
    // `Weapon Pad` chunks all carry a flat `0,0,0` colour set, and the blue
    // chevron and red cross are painted into `ds_speedup_cs.gtf` and
    // `ds_weaponup_cs.gtf` respectively.
    //
    // This was `false` between 2026-09-02 and 2026-09-03, paired with a
    // `lit = 0.0` sweep, so that a flat invented tint written over
    // `in.colour` would show up. It did show up, and what it showed was a pad
    // lifted out of the circuit's own light rig and repainted - see
    // `docs/rendering/pads.md`.
    out.vertex_colour_is_light = true;

    let mut node_vertex_ranges = Vec::new();
    for node in nodes.iter().filter(|n| n.class_id == class_id) {
        let node_first_vertex = out.vertices.len() as u32;
        report.nodes += 1;
        if let Some((hash, ..)) = node_geometry(&data[node.payload()], order)
            && let (Some(chunk_index), Some(chunk)) = (model.mesh_index(hash), model.mesh(hash))
        {
            report.addressed += 1;
            if emit_chunk(
                &mut out,
                &model,
                model_blob,
                chunk_index,
                chunk,
                super::View::Main,
                &mut report,
            ) {
                report.drawn += 1;
                out.mesh_count += 1;
            }
        }
        node_vertex_ranges.push(node_first_vertex..out.vertices.len() as u32);
    }
    out.node_vertex_ranges = node_vertex_ranges;
    if report.nodes > report.addressed {
        let named = placed_hashes(data, &nodes, Some(classes), order, &model);
        (report.routed_chunks, report.routed_triangles) = world_pass_pad_chunks(&model, &named);
    }
    face_normals(&mut out);
    let (centre, radius) = bounding_sphere(&out.vertices);
    out.centre = centre;
    out.radius = radius;
    Ok((out, report))
}

/// Decodes every surface of one `.rcsmodel` chunk into `out`, baked at the
/// identity transform, and reports what happened doing it.
///
/// Shared by [`super::build_scene`]'s world-space pass (chunks no node
/// references) and [`build_pad_class`] (a `Speedup Pad`/`Weapon Pad` node's
/// own chunk, which is baked in world space the same way - see
/// `super::is_world_baked`'s doc comment): both draw a whole chunk at
/// [`anim_node::Placement::STATIC`] and count the same things while doing
/// it - a world-baked chunk's own coordinates already ignore whatever node
/// named it, so there is no per-chunk transform to thread through here.
/// `view` says which chunks this build draws, see [`super::View`]. The draws
/// carry no node: a chunk drawn here has no useful node identity of its own
/// (see `DrawCall::node`'s own doc comment on ambiguity across sources).
///
/// Returns whether anything was actually emitted, which is what tells a
/// caller whether to count the chunk as drawn at all.
pub(super) fn emit_chunk(
    out: &mut Model,
    model: &rcsmodel::Model,
    model_blob: &[u8],
    chunk_index: usize,
    chunk: &rcsmodel::Mesh,
    view: super::View,
    report: &mut Report,
) -> bool {
    // A chunk the original draws only into its behind-the-glass target is left
    // out of the main view, and the reverse (`rcsmodel::RENDER_BEHIND_GLASS`).
    if !view.draws(chunk) {
        report.behind_glass += usize::from(chunk.is_behind_glass());
        return false;
    }
    let mut emitted = false;
    // **Every surface, not just the chunk's own.** A quarter of the disc's
    // chunks declare more than one, each with its own material, bias and
    // descriptors, and they are 40 % more geometry than the first surfaces
    // carry between them. See `rcsmodel::Mesh::surfaces`.
    for mesh in chunk.surfaces() {
        if super::isolate::excludes(model, mesh) {
            report.isolated += 1;
            continue;
        }
        // **What the chunk declares, first**, which is what turned these on:
        // 3,382 of the disc's chunks declare a stride the search below cannot
        // fit, and every one of them used to be skipped silently here. See
        // `rcsmodel::vertex_decl`.
        let Some(stride) = mesh
            .declared_stride()
            .or_else(|| mesh.solve_stride_without_a_box(model_blob))
        else {
            report.no_stride += 1;
            continue;
        };
        let surface = surface(
            model,
            mesh,
            &out.textures,
            &out.material_slots,
            &out.material_specular_exponent,
            &out.material_anim,
        );
        report.see_through += usize::from(surface.blend.is_some());
        report.cutout += usize::from(surface.cutout);
        report.no_texcoord += usize::from(declares_no_texcoord(mesh));
        report.track_surface += usize::from(std::ptr::eq(mesh, chunk) && mesh.is_track());
        for submesh in &mesh.submeshes {
            if submesh.vertex_count == 0 || submesh.index_count == 0 {
                continue;
            }
            let (Ok(points), Ok(indices)) = (
                mesh.positions(model_blob, submesh, stride),
                mesh.indices(model_blob, submesh),
            ) else {
                continue;
            };
            let normals = mesh.normals(model_blob, submesh, stride).ok();
            let texcoords = mesh.texcoords(model_blob, submesh, stride).ok();
            let lightmap_texcoords = mesh.lightmap_texcoords(model_blob, submesh, stride).ok();
            let vertex_light = mesh.vertex_light(model_blob, submesh, stride).ok();
            let texcoords2 =
                super::ice::second_uv(surface.roles, mesh, model_blob, submesh, stride);
            report.authored_normals += normals.as_deref().map_or(0, authored);
            emit(
                out,
                Geometry {
                    points: &points,
                    normals: normals.as_deref(),
                    texcoords: texcoords.as_deref(),
                    lightmap_texcoords: lightmap_texcoords.as_deref(),
                    texcoords2: texcoords2.as_deref(),
                    vertex_light: vertex_light.as_deref(),
                    indices: &indices,
                    chunk: u32::try_from(chunk_index).ok(),
                },
                &anim_node::Placement::STATIC,
                None,
                surface,
            );
            report.triangles += indices.len() / 3;
            emitted = true;
        }
    }
    emitted
}
