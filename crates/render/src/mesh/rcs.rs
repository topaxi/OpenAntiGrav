//! Building a drawable [`Model`] from a PS3 `.vex` and the `.rcsmodel` beside
//! it.
//!
//! The PS3 counterpart of [`super::build_class`], and deliberately a separate
//! function rather than a branch inside it. The two share the *scene* - node
//! tree, class table, world transforms, all read by `oag_formats::vex` exactly
//! as before - and share nothing at all below that: a PSP mesh's geometry is a
//! batch list in its own payload, and a PS3 mesh's is a chunk in another file,
//! addressed by hash and quantised through a bias and scale. Threading that
//! through the 300-line PSP builder would have put a `if ps3` at every step.
//!
//! # What it draws, and what it does not
//!
//! **Textured, lit, and blended where the material says so**, all of it out of
//! the disc: positions, triangle indices and vertex normals from the
//! `.rcsmodel`, the texture coordinate from the last four bytes of each vertex,
//! and the `.gtf` each material names through [`oag_formats::gtf`]. Nothing is
//! substituted - a made-up normal or an invented alpha would light a model
//! wrongly rather than visibly failing, which is the failure mode `CLAUDE.md`
//! names.
//!
//! What is **not** drawn, and is counted rather than hidden: a mesh whose chunk
//! cannot be found or whose vertex stride cannot be recovered, and the second
//! texture a material may name at `+0x78`. That second slot is a *mask* on
//! Talon's Junction's cloud plate (`cloud mask.gtf` beside `clouds_new.gtf`), a
//! lightmap on its road, an emissive map on its tunnels and a normal map on a
//! craft - one field with at least four uses, selected by a shader nothing here
//! reads. So a surface whose coverage lives in that second texture still paints
//! solid; see [`surface`].

use anyhow::{Context, Result, bail};
use oag_core::math::{Mat4, Vec3};
use oag_formats::{rcsmodel, vex};

use super::{Bounds, DrawCall, GpuVertex, Model, ModelTexture, slots};

/// How far a dequantised point may miss the authored box face by, in world
/// units, before a stride is rejected.
///
/// Two quantisation steps at the `1/128` scale every measured file uses. Read
/// from each mesh's own scale rather than hardcoded, so a file that quantises
/// differently is judged on its own terms.
const TOLERANCE_STEPS: f32 = 2.0;

mod report;

pub use report::Report;

/// The `.rcsmodel` entry name beside a `.vex` one.
///
/// A path rewrite rather than a lookup, because that is how the disc pairs
/// them: `/data/ships/assegai/ship.vex` sits beside
/// `/data/ships/assegai/ship.rcsmodel`, and every pair on the disc is spelled
/// that way. Returns `None` for a name that does not end in `.vex`, since there
/// is then no pairing rule to apply.
#[must_use]
pub fn sibling_name(vex_name: &str) -> Option<String> {
    let stem = vex_name.strip_suffix(".vex").or_else(|| {
        // The game's own spelling is case-insensitive in both containers, and a
        // PSP-shaped name reaches here through `oag_hd`'s tables.
        vex_name
            .len()
            .checked_sub(4)
            .filter(|&at| vex_name[at..].eq_ignore_ascii_case(".vex"))
            .map(|at| &vex_name[..at])
    })?;
    Some(format!("{stem}.rcsmodel"))
}

/// The `.rcsmodel` beside `name`, when this `.vex` needs one and the archive
/// carries it.
///
/// `None` covers three different things and deliberately does not tell them
/// apart, because every caller does the same thing with all three - draws
/// nothing and says so:
///
/// - a PSP or PS2 `.vex`, which has its geometry inside it and needs no sibling;
/// - a name with no `.vex` extension to rewrite;
/// - a sibling the archive does not have.
///
/// A caller that wants the distinction has [`super::geometry_is_external`] for the
/// first of them.
#[must_use]
pub fn sibling_geometry(spec: &str, name: &str, data: &[u8]) -> Option<Vec<u8>> {
    if !super::geometry_is_external(data) {
        return None;
    }
    super::read_blob(spec, &sibling_name(name)?).ok()
}

/// The whole of a PS3 model, read out of the archive `spec` names, or `None`
/// when `name` is not one.
///
/// **`None` is the answer for every PSP and PS2 source**, so a caller keeps its
/// existing path for those unchanged and adds one match arm rather than a
/// branch at every step.
///
/// The one call a caller needs: it decides whether this `.vex` has its geometry
/// elsewhere, fetches the sibling and builds the pair. Everything a PSP or PS2
/// source does is unchanged, because [`sibling_geometry`] answers `None` for
/// them and the caller keeps its existing path.
///
/// # Errors
///
/// As [`build_scene`].
pub fn scene_from(spec: &str, name: &str, data: &[u8]) -> Result<Option<(Model, Report)>> {
    let Some(geometry) = sibling_geometry(spec, name, data) else {
        return Ok(None);
    };
    build_scene(name, data, &geometry, &mut |path| {
        super::read_blob(spec, path).ok()
    })
    .map(Some)
}

/// How a build fetches a material's `.gtf` out of whatever archive the caller
/// holds.
///
/// A closure rather than an archive spec because the two callers hold different
/// things: the viewer has a `<image>:<path>` string and the game has an
/// `oag_assets::Archives` spanning seven of them. `None` for a texture the
/// archive does not have, which [`Report::untextured`] counts.
pub type Textures<'a> = &'a mut dyn FnMut(&str) -> Option<Vec<u8>>;

/// A loader that finds nothing, for a caller with no archive in hand.
///
/// **Not a convenience** - it is the honest way to build geometry when the
/// textures cannot be reached, and it produces the untextured model this module
/// produced before `.gtf` was read, with every slot counted as missing rather
/// than silently white.
pub fn no_textures(_: &str) -> Option<Vec<u8>> {
    None
}

mod isolate;
mod skin;
use skin::{flips, picks, roles, skin, variants};

/// The bounding box and chunk hash a PS3 `Mesh` node's payload carries.
///
/// The layout the PSP uses for geometry, with the geometry taken out: the box
/// pair at `+0x10`/`+0x20` is measured `min <= max` on 1,638 of 1,638 nodes,
/// and the word at `+0x30` is the `.rcsmodel` chunk's own first word.
fn node_geometry(
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
fn is_world_baked(mesh: &rcsmodel::Mesh, min: [f32; 3], max: [f32; 3], to_world: Mat4) -> bool {
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
fn referenced(
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

/// The whole of a PS3 model: the meshes its `.vex` places, and the geometry
/// nothing in the `.vex` mentions.
///
/// # Two passes, because Wipeout HD authors two kinds of geometry
///
/// **A craft is all first pass and a circuit is almost all second.** Every one
/// of Assegai's 15 `Mesh` nodes addresses a chunk, and its positions are in the
/// node's own space - the PSP arrangement with the vertices moved out. All 126
/// of Talon's Junction's `Mesh` nodes are *props*: blimps, girders, sky
/// traffic. The road, the walls, the scenery and both kinds of pad are among
/// the 913 of 983 chunks no `Mesh` node addresses, each carrying a
/// **world-space** bias, drawn without a node transform because there is no
/// node - a `Weapon Pad` or `Speedup Pad` node names its chunk at the mesh
/// payload's own `+0x30`, but as [`referenced`] records, the chunk's
/// coordinates ignore the node anyway.
///
/// So a circuit that drew only the first pass drew its skybox traffic and no
/// track, which is exactly what this looked like before the second existed.
///
/// **56 of Talon's Junction's prop nodes stay honestly absent.** Their hashes
/// are in no `.rcsmodel` on the disc except *other environments'* - the same
/// `tanker1aShape` hash appears in Amphiseum's and Tech De Ra's own track
/// models, so the hash is content-derived and those donors were simply never
/// baked into this circuit's file. The sky traffic that is visible here is
/// the world-space `animating_traffic` chunks, which the second pass draws.
///
/// # Errors
///
/// As [`build`].
pub fn build_scene(
    label: &str,
    data: &[u8],
    model_blob: &[u8],
    textures: Textures<'_>,
) -> Result<(Model, Report)> {
    let (mut out, mut report) =
        build_with_options(label, data, model_blob, textures, |c| c.mesh, true)?;

    let model = rcsmodel::Model::parse(model_blob)
        .map_err(|e| anyhow::anyhow!("{label}: the .rcsmodel beside it: {e}"))?;
    let nodes = vex::nodes(data).context("walking the node tree")?;
    let mesh_class = vex::classes_of(data)
        .ok()
        .and_then(|c| c.mesh)
        .context("no mesh class id for this .vex version")?;
    let placed = referenced(data, &nodes, mesh_class, &model);

    for mesh in &model.meshes {
        if placed.contains(&mesh.hash) {
            continue;
        }
        if isolate::excludes(&model, mesh) {
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
        let surface = surface(&model, mesh, &out.textures, &out.material_slots);
        report.see_through += usize::from(surface.blend.is_some());
        report.no_texcoord += usize::from(declares_no_texcoord(mesh));
        let mut emitted = false;
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
            report.authored_normals += normals.as_deref().map_or(0, authored);
            emit(
                &mut out,
                Geometry {
                    points: &points,
                    normals: normals.as_deref(),
                    texcoords: texcoords.as_deref(),
                    lightmap_texcoords: lightmap_texcoords.as_deref(),
                    vertex_light: vertex_light.as_deref(),
                    indices: &indices,
                },
                Mat4::IDENTITY,
                None,
                surface,
            );
            report.triangles += indices.len() / 3;
            emitted = true;
        }
        if emitted {
            report.unreferenced += 1;
            out.mesh_count += 1;
        }
    }

    face_normals(&mut out);
    let (centre, radius) = bounding_sphere(&out.vertices);
    out.centre = centre;
    out.radius = radius;
    Ok((out, report))
}

/// Whether a chunk's declaration names no texture coordinate.
///
/// `false` for a chunk with no declaration at all - an inline one, where
/// `rcsmodel::Mesh::texcoords` still reads the last four bytes and the count
/// would be about this module rather than about the data.
fn declares_no_texcoord(mesh: &rcsmodel::Mesh) -> bool {
    mesh.decl
        .as_ref()
        .is_some_and(|decl| decl.diffuse_texcoord().is_none())
}

/// How one chunk is drawn: which texture slot, and blended or not.
///
/// # `.gtf` is what turned the see-through chunks back on
///
/// **The alpha a blend needs is in the texture and nowhere else**, and until
/// `oag_formats::gtf` was read this module had none - so a see-through chunk was
/// *left out* rather than blended, because alpha-over at `alpha = 1.0` paints
/// exactly the opaque pixels while dropping depth write and additive blows a
/// glass panel to white. That stopgap is gone: `Texture::to_rgba` returns RGBA,
/// the shader's `fs_main_blend` multiplies the texel's alpha into its output,
/// and these surfaces are drawn with the equation the material asks for.
///
/// The stakes are the same as they were: on Talon's Junction the largest surface
/// in the whole file is a 63-triangle `clouds` plate spanning 2,011 x 2,195
/// world units, and drawn opaque it covers the circuit - seen from above the
/// track was one white blob. 4.3 % to 35.9 % of chunks are see-through across
/// all 16 circuits (`crates/formats/tests/rcsmodel_material_ground_truth.rs`).
///
/// # A texture that will not paint is *not* a reason to blend
///
/// A material whose `.gtf` is missing or refused has no alpha either, so
/// blending it would put the white 1x1 through the transparent pass and paint
/// the same sheet with depth write off - strictly worse than before. Such a
/// chunk stays in the opaque pass and is counted in [`Report::untextured`].
fn surface(
    model: &rcsmodel::Model,
    mesh: &rcsmodel::Mesh,
    skin: &[Option<ModelTexture>],
    material_slots: &[u32],
) -> Surface {
    let slot = mesh.material as usize;
    let texture = skin.get(slot).and_then(Option::as_ref).map(|_| slot);
    // `DEFAULT` for a slot with no reading, which is what every title but HD
    // has and what an HD material whose microcode did not trace answers.
    let roles = material_slots.get(slot).copied().unwrap_or(slots::DEFAULT);
    let blend = match model.material_of(mesh).map(rcsmodel::Material::blend) {
        // Only a painted surface can be blended - see above.
        Some(rcsmodel::Blend::Factors { src, dst }) if texture.is_some() => {
            Some(blend_state(src, dst))
        }
        _ => None,
    };
    Surface {
        texture,
        blend,
        roles,
    }
}

/// One authored factor pair as the state a pipeline is built with.
///
/// **A translation, not a decision.** `oag_formats::rcsmodel::Factor` is the
/// disc's own four values under names, and each has exactly one counterpart in
/// `wgpu`; the operation is `Add` because the RSX's blend equation register is a
/// separate field this reading has not touched and `GL_FUNC_ADD` is what it
/// holds at reset.
///
/// **Alpha follows colour rather than splitting**, for the reason
/// [`crate::mesh_render::ADDITIVE_BLEND`] gives: a target later read as
/// premultiplied should not disagree with its own colour channels. The disc says
/// nothing about the alpha channel either way - a `CellGcmBlendFunc` pair is
/// programmed for both and only the colour half is what these materials vary.
pub fn blend_state(src: rcsmodel::Factor, dst: rcsmodel::Factor) -> wgpu::BlendState {
    fn factor(f: rcsmodel::Factor) -> wgpu::BlendFactor {
        match f {
            rcsmodel::Factor::One => wgpu::BlendFactor::One,
            rcsmodel::Factor::SrcColour => wgpu::BlendFactor::Src,
            rcsmodel::Factor::SrcAlpha => wgpu::BlendFactor::SrcAlpha,
            rcsmodel::Factor::OneMinusSrcAlpha => wgpu::BlendFactor::OneMinusSrcAlpha,
        }
    }
    let component = wgpu::BlendComponent {
        src_factor: factor(src),
        dst_factor: factor(dst),
        operation: wgpu::BlendOperation::Add,
    };
    wgpu::BlendState {
        color: component,
        alpha: component,
    }
}

/// One submesh's decoded arrays, as [`emit`] takes them.
///
/// A struct rather than four parameters because they are one thing - the same
/// submesh read four ways, all of them the same length - and because the two
/// call sites would otherwise differ only in an argument's position.
#[derive(Debug, Clone, Copy)]
struct Geometry<'a> {
    points: &'a [[f32; 3]],
    /// `None` leaves every vertex normal zero, which is [`face_normals`]'
    /// signal to derive one.
    normals: Option<&'a [[f32; 3]]>,
    /// `None` leaves every coordinate at the origin of the texture.
    texcoords: Option<&'a [[f32; 2]]>,
    /// Where the circuit's lightmap atlas is sampled, for a chunk that declares
    /// a `lightmapUV`. `None` leaves it at the origin, which a white lightmap
    /// makes harmless.
    lightmap_texcoords: Option<&'a [[f32; 2]]>,
    /// HD's **baked per-vertex light and sun-occlusion mask**, `[r, g, b,
    /// mask]`, from `oag_formats::rcsmodel::Mesh::vertex_light`. `None` on 632
    /// of Talon's Junction's 983 chunks - not a gap but the other half of the
    /// split: a chunk bakes into the lightmap atlas **or** its vertices, never
    /// both.
    vertex_light: Option<&'a [[f32; 4]]>,
    indices: &'a [u16],
}

/// What [`surface`] decided, carried into [`emit`].
#[derive(Debug, Clone, Copy, Default)]
struct Surface {
    /// Index into `Model::textures`, or `None` for a material with no painted
    /// texture.
    texture: Option<usize>,
    /// What the material's own microcode says its two texture units are for,
    /// packed as [`slots`] and handed to every vertex this surface emits.
    roles: u32,
    /// The blend equation the material authors, or `None` for the opaque pass.
    ///
    /// The state itself rather than a `vex::BlendClass`, because a PS3 material
    /// authors a factor *pair* and Pulse's three classes have no member for
    /// most of them - see `mesh::DrawCall::blend_state`.
    blend: Option<wgpu::BlendState>,
}

/// Zero for a coordinate that is not a finite number.
///
/// **Almost nothing reaches this any more, and it stays.** 1.68 % of a
/// circuit's vertices used to decode to an infinity or a NaN here, and every
/// one of them was a `tangent` or a colour set read as a coordinate because the
/// reader took the last four bytes of a vertex; reading where
/// `rcsmodel::VertexDecl` says leaves Talon's Junction with **9** of 600,280
/// and Assegai with 6 of 25,144, which are a different and still-unexplained
/// thing. An inline chunk has no declaration at all, so the guard is also what
/// those go through. A NaN in a vertex buffer is not a visible failure, it is a
/// hole in the rasteriser's output, so it is pinned to zero here.
fn finite(v: f32) -> f32 {
    if v.is_finite() { v } else { 0.0 }
}

/// How many of a submesh's decoded normals the file actually authored.
///
/// **A zero one did not come from the file**, it came from a vertex record that
/// is zero from the position onward - padding at the end of a buffer, and
/// `cockpit_screenShape` is one that carries some. `emit` derives a normal for
/// those from the triangles, so counting them here would make the load report
/// claim the disc's data where the fallback ran.
fn authored(normals: &[[f32; 3]]) -> usize {
    normals
        .iter()
        .filter(|n| Vec3::from_array(**n).length_squared() > 1e-12)
        .count()
}

/// Appends one submesh's geometry to a model, as its own draw call.
///
/// `normals` are the file's own, already decoded; `None` leaves the vertex
/// normal zero, which is [`face_normals`]'s signal to derive one.
fn emit(out: &mut Model, mesh: Geometry<'_>, to_world: Mat4, node: Option<u32>, surface: Surface) {
    let Geometry {
        points,
        normals,
        texcoords,
        lightmap_texcoords,
        vertex_light,
        indices,
    } = mesh;
    let flip_v = surface.roles & slots::FLIP_V != 0;
    let first_vertex = u32::try_from(out.vertices.len()).unwrap_or(u32::MAX);
    let first_index = u32::try_from(out.indices.len()).unwrap_or(u32::MAX);
    let mut centre = Vec3::ZERO;
    for (k, point) in points.iter().enumerate() {
        let p = to_world.transform_point3(Vec3::from_array(*point));
        centre += p;
        // The node transforms in these files are rigid, so rotating the
        // direction and renormalising is the whole of it - an inverse transpose
        // would be needed only under non-uniform scale.
        let normal = normals
            .and_then(|n| n.get(k))
            .map(|n| {
                to_world
                    .transform_vector3(Vec3::from_array(*n))
                    .normalize_or_zero()
                    .to_array()
            })
            .unwrap_or([0.0, 0.0, 0.0]);
        let light = vertex_light.and_then(|c| c.get(k));
        out.vertices.push(GpuVertex {
            position: p.to_array(),
            // Zero means the file gave none, and `face_normals` derives it.
            normal,
            // HD's `f[TC1]`: the per-vertex light the fragment program
            // **adds** to the lightmap term before multiplying the albedo, so
            // `mesh.wgsl` folds it into the authored sum, not into a tint.
            // Alpha stays the texture's own multiplier - `colour.a` is not
            // where the fourth byte goes, see `sun_mask` below.
            // **Zero for a chunk with no colour set is read**: the blocks
            // without it write `MOV o[TC1].xyz, c[K].xxxx`, and `c[K]` against
            // each program's local-constant table is 0.0 in 11,180 of 11,184.
            colour: light
                .map(|&[r, g, b, _]| [finite(r), finite(g), finite(b), 1.0])
                .unwrap_or([0.0, 0.0, 0.0, 1.0]),
            // A non-finite half is a vertex whose coordinate this reading does
            // not explain - 1.68 % of a circuit's stride-18 ones. Zero rather
            // than a NaN travelling into the vertex buffer.
            lightmap_texcoord: lightmap_texcoords
                .and_then(|t| t.get(k))
                .map(|&[u, v]| [finite(u), finite(v)])
                .unwrap_or([0.0, 0.0]),
            // **`1 - v` where the material's own vertex program writes it**,
            // which one of Talon's Junction's 283 resolved variants does. See
            // `oag_formats::rcsmaterial::vertex` for the microcode and
            // `skin::flips` for why this is per material rather than global.
            texcoord: texcoords
                .and_then(|t| t.get(k))
                .map(|&[u, v]| {
                    let v = finite(v);
                    [finite(u), if flip_v { 1.0 - v } else { v }]
                })
                .unwrap_or([0.0, 0.0]),
            // **Unlit under the tint diagnostic**, so the flat colour reaches
            // the frame as itself: `lit` 0.0 takes `mesh.wgsl`'s stand-in
            // path, whose light and tint are both 1.0 for an HD model, and
            // the palette entry can be matched exactly rather than by hue
            // through a coloured light rig.
            lit: if isolate::tinting() || isolate::unlit() {
                0.0
            } else {
                1.0
            },
            anim: 0,
            slots: surface.roles,
            xform: 0,
            // The colour set's fourth byte - see
            // `oag_formats::rcsmodel::Mesh::vertex_light` and
            // `docs/ghidra/functions/ps3-hdfury-eu/renderer.md`, "The sun is
            // real and it is masked". `1.0` (unmasked) for a chunk with no
            // colour set, which is what a lightmapped chunk uses instead -
            // `mesh.wgsl` multiplies this by the lightmap's own alpha, so
            // `1.0` here leaves that gate untouched.
            sun_mask: light.map(|&[.., m]| finite(m)).unwrap_or(1.0),
        });
    }
    let centre = centre / points.len() as f32;
    let radius = out.vertices[first_vertex as usize..]
        .iter()
        .map(|v| (Vec3::from_array(v.position) - centre).length())
        .fold(0.0f32, f32::max);

    out.indices
        .extend(indices.iter().map(|&i| first_vertex + u32::from(i)));
    let list = match surface.blend {
        Some(_) => &mut out.transparent_draws,
        None => &mut out.draws,
    };
    list.push(DrawCall {
        moving: false,
        range: first_index..u32::try_from(out.indices.len()).unwrap_or(u32::MAX),
        texture: surface.texture,
        bounds: Bounds {
            centre: centre.to_array(),
            radius,
        },
        culled: false,
        // **`None`, and that is not an omission.** `DrawCall::blend` holds the
        // class a *Pulse* batch's `pass_mask` names, and a PS3 material names
        // none - it authors a factor pair, which `blend_state` carries. Putting
        // the nearest member there instead would be the same fold this module
        // just stopped doing: 144 of the disc's see-through materials are
        // `ONE`/`ONE`, and calling them `AlphaOver` in a field defined as a
        // recovered class is a fabricated value even where nothing reads it.
        // Which list a draw is in, above, is what says it is transparent.
        blend: None,
        blend_state: surface.blend,
        // A PS3 chunk has no `.vex` mesh payload and so no derived layer; the
        // uniform value leaves `Model::sort_by_layer` holding chunk order.
        layer: vex::LAYER_DEFAULT,
        node,
    });
}

/// Flattens every node of `class` into one buffer pair, taking its geometry
/// from `model_blob`.
///
/// # Errors
///
/// A `.vex` that will not walk, a `.rcsmodel` that will not parse, and a class
/// id this file's version does not number. **A mesh that cannot be drawn is not
/// an error** - it is counted in the returned [`Report`], because on a circuit
/// that is the ordinary case and failing the load over it would draw nothing at
/// all.
///
/// **Never skips a world-baked node reference on its own** - see
/// [`build_with_options`], which this calls with `world_space_fallback:
/// false`. That is what every caller outside this module wants: a craft's
/// `mesh::rcs::build` call (`oag_game::livery`, `livery::flare`) has no
/// second pass to catch a wrongly-skipped part, so skipping here would draw
/// nothing for it rather than draw it through the (harmless, if wrong-transform)
/// node path - an invisible ship part being strictly worse than a
/// coincidentally-placed one. Only [`build_scene`] - which *does* have a
/// second, world-space pass right below it - is allowed to ask for the skip.
pub fn build(
    label: &str,
    data: &[u8],
    model_blob: &[u8],
    textures: Textures<'_>,
    pick: fn(vex::classes::Classes) -> Option<u32>,
) -> Result<(Model, Report)> {
    build_with_options(label, data, model_blob, textures, pick, false)
}

/// [`build`], with the world-bake skip [`is_world_baked`] documents - on only
/// when the caller has a world-space pass ready to draw the skipped chunk
/// instead, which is why this is not `pub`: [`build_scene`] is the one caller
/// that qualifies, and every other caller goes through [`build`] instead.
fn build_with_options(
    label: &str,
    data: &[u8],
    model_blob: &[u8],
    textures: Textures<'_>,
    pick: fn(vex::classes::Classes) -> Option<u32>,
    world_space_fallback: bool,
) -> Result<(Model, Report)> {
    if !vex::has_magic(data) {
        bail!("{label} is not a .vex file (no VEXX magic)");
    }
    let classes = vex::classes_of(data).with_context(|| format!("{label}: class table"))?;
    let Some(class_id) = pick(classes) else {
        bail!(
            "{label} is .vex version {}, and the class id for this node type has not \
             been recovered for it",
            classes.version
        );
    };
    let model = rcsmodel::Model::parse(model_blob)
        .map_err(|e| anyhow::anyhow!("{label}: the .rcsmodel beside it: {e}"))?;

    let nodes = vex::nodes(data).context("walking the node tree")?;
    // Composed exactly as the PSP path composes them: a mesh's positions are in
    // the space of whichever transform encloses it, nested up to 25 deep on a
    // circuit. Nothing about that moved to the PS3.
    let world = vex::world_transforms(data, &nodes);
    let order = vex::byte_order(data);

    let mut out = Model::none(label);
    let mut report = Report::default();
    // **The variant first**, because which sampler entry each of this
    // renderer's two bindings comes from is a property of the shader the
    // lit-race key resolves to, not of the entry's position - see
    // `skin::picks`.
    let material_variants = variants(&model, textures, &mut report);
    let picks = picks(&model, &material_variants, textures);
    let (skins, seconds) = skin(&model, &picks, textures, &mut report);
    // After the variants, because the roles are read off the resolved one.
    let mut material_slots = roles(&model, &material_variants, &picks, &seconds, textures);
    // **The coordinate's orientation, off the resolved *vertex* block** rather
    // than the fragment one the roles come from, and folded into the same word
    // because it is the same kind of statement: what this material's own
    // microcode says. See `skin::flips`.
    for (packed, flipped) in
        material_slots
            .iter_mut()
            .zip(flips(&model, &material_variants, textures))
    {
        if flipped {
            *packed |= slots::FLIP_V;
        }
    }
    out.textures = skins;
    out.lightmaps = seconds;
    // Every vertex this module writes carries HD's baked per-vertex light in
    // `colour`, which the fragment programs **add** rather than multiply - see
    // `emit`. The flag is what stops the stand-in shading path from tinting by
    // it, and it is a property of the model rather than of the target because
    // the capture and viewer paths draw these same models into a gamma one.
    out.vertex_colour_is_light = true;
    out.material_variants = material_variants;
    out.material_slots = material_slots;

    for (index, node) in nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.class_id == class_id)
    {
        report.nodes += 1;
        let Some((hash, min, max)) = node_geometry(&data[node.payload()], order) else {
            continue;
        };
        let Some(mesh) = model.mesh(hash) else {
            continue;
        };
        // **Skip a chunk this node names but does not actually place.** See
        // `is_world_baked`: drawing it through `to_world` below would test
        // its already-world-space positions against this node's un-transformed
        // local box, which cannot pass, and letting it fall through to
        // `report.no_stride`/`strays` invites `referenced`'s exclusion (which
        // hands the same hash to the world-space pass) to draw it twice
        // wherever the loose `submesh_fits` tolerance happens to pass anyway.
        let to_world = Mat4::from_cols_array(&world[index]);
        if world_space_fallback && is_world_baked(mesh, min, max, to_world) {
            report.world_baked += 1;
            continue;
        }
        if isolate::excludes(&model, mesh) {
            report.isolated += 1;
            continue;
        }
        report.addressed += 1;
        let tolerance = mesh.scale.iter().fold(0.0f32, |a, &b| a.max(b)) * TOLERANCE_STEPS;
        // **What the chunk declares, first.** The searches below fit a stride
        // to the authored box and to buffer layout, and they exist because this
        // field had not been read; see `rcsmodel::vertex_decl`. They stay for
        // an inline chunk, which declares nothing.
        let Some(stride) = mesh
            .declared_stride()
            .or_else(|| mesh.solve_stride(model_blob, (min, max), tolerance))
            .or_else(|| mesh.solve_stride_by_layout())
            .or_else(|| mesh.solve_stride_by_normals(model_blob))
        else {
            report.no_stride += 1;
            continue;
        };

        let surface = surface(&model, mesh, &out.textures, &out.material_slots);
        report.see_through += usize::from(surface.blend.is_some());
        report.no_texcoord += usize::from(declares_no_texcoord(mesh));
        let mut emitted = false;
        for submesh in &mesh.submeshes {
            if submesh.vertex_count == 0 || submesh.index_count == 0 {
                continue;
            }
            // **The stride is the mesh's, and one submesh may not share it.**
            // `solve_stride` tolerates that; drawing must not, or the odd
            // submesh's attribute bytes are read as positions and scatter over
            // the world. See `rcsmodel::Mesh::submesh_fits`.
            if !mesh.submesh_fits(model_blob, submesh, stride, (min, max)) {
                report.strays += 1;
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
            report.authored_normals += normals.as_deref().map_or(0, authored);
            emit(
                &mut out,
                Geometry {
                    points: &points,
                    normals: normals.as_deref(),
                    texcoords: texcoords.as_deref(),
                    lightmap_texcoords: lightmap_texcoords.as_deref(),
                    vertex_light: vertex_light.as_deref(),
                    indices: &indices,
                },
                to_world,
                u32::try_from(index).ok(),
                surface,
            );
            report.triangles += indices.len() / 3;
            emitted = true;
        }
        if emitted {
            report.drawn += 1;
            out.mesh_count += 1;
        }
    }

    face_normals(&mut out);
    let (centre, radius) = bounding_sphere(&out.vertices);
    out.centre = centre;
    out.radius = radius;
    Ok((out, report))
}

/// Gives every vertex the average of the faces meeting at it.
///
/// # This is a derivation, not a substitute
///
/// **The `.rcsmodel` does carry authored normals and they are not read** - they
/// are in the 8 to 16 attribute bytes after each position, whose layout is
/// unrecovered. What is computed here comes out of the triangles this module
/// already decoded, so it is a fact about the geometry rather than a stand-in
/// for data nobody has: an unlit model is a white silhouette, and a silhouette
/// hides exactly the decoding mistakes this is meant to expose.
///
/// It will differ from the authored normals wherever the artists split or
/// smoothed them by hand, so a model lit this way is a shape check and not a
/// match against the original. Reading the real ones supersedes it.
fn face_normals(model: &mut Model) {
    // **Only where the file gave none.** A zero normal is `emit`'s signal that
    // `rcsmodel::Mesh::normals` had nothing for that vertex; an authored one is
    // better than anything derivable here, because it carries the hard edges the
    // exporter split vertices for and a smooth average by construction cannot.
    let mut derived = vec![Vec3::ZERO; model.vertices.len()];
    for triangle in model.indices.as_chunks::<3>().0 {
        let [a, b, c] = [triangle[0], triangle[1], triangle[2]].map(|i| i as usize);
        let (pa, pb, pc) = (
            Vec3::from_array(model.vertices[a].position),
            Vec3::from_array(model.vertices[b].position),
            Vec3::from_array(model.vertices[c].position),
        );
        let face = (pb - pa).cross(pc - pa);
        for index in [a, b, c] {
            derived[index] += face;
        }
    }
    for (vertex, derived) in model.vertices.iter_mut().zip(derived) {
        if Vec3::from_array(vertex.normal).length_squared() > 1e-12 {
            continue;
        }
        // A vertex on no triangle, or on exactly cancelling ones, keeps a
        // usable up rather than a zero the shader would normalise to NaN.
        vertex.normal = if derived.length_squared() > 1e-12 {
            derived.normalize().to_array()
        } else {
            [0.0, 1.0, 0.0]
        };
    }
}

/// The whole model's framing sphere, which the viewer's camera is placed from.
///
/// Centre of the axis-aligned bounds rather than of the vertices: a circuit
/// carries most of its vertices in the few most detailed corners, and averaging
/// them puts the camera looking at a corner of the track.
fn bounding_sphere(vertices: &[GpuVertex]) -> ([f32; 3], f32) {
    if vertices.is_empty() {
        return ([0.0; 3], 0.0);
    }
    let mut min = [f32::MAX; 3];
    let mut max = [f32::MIN; 3];
    for v in vertices {
        for i in 0..3 {
            min[i] = min[i].min(v.position[i]);
            max[i] = max[i].max(v.position[i]);
        }
    }
    let centre: [f32; 3] = std::array::from_fn(|i| (min[i] + max[i]) / 2.0);
    let radius = vertices
        .iter()
        .map(|v| (Vec3::from_array(v.position) - Vec3::from_array(centre)).length())
        .fold(0.0f32, f32::max)
        .max(0.001);
    (centre, radius)
}

#[cfg(test)]
mod tests;
