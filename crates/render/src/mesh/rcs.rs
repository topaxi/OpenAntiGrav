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
//! **Untextured triangles, flat-shaded by the viewer's own rig.** That is what
//! [`oag_formats::rcsmodel`] recovers: positions and indices. Normals, texture
//! coordinates and the `.gtf` textures they would address are all undecoded,
//! and none of them is substituted - a made-up normal lights a model wrongly
//! rather than visibly failing, which is the failure mode `CLAUDE.md` names.
//!
//! A mesh whose geometry cannot be found or whose vertex stride cannot be
//! recovered contributes **nothing**, and [`Report`] counts it so the caller can
//! say so out loud.

use anyhow::{Context, Result, bail};
use oag_core::math::{Mat4, Vec3};
use oag_formats::{rcsmodel, vex};

use super::{Bounds, DrawCall, GpuVertex, Model};

/// How far a dequantised point may miss the authored box face by, in world
/// units, before a stride is rejected.
///
/// Two quantisation steps at the `1/128` scale every measured file uses. Read
/// from each mesh's own scale rather than hardcoded, so a file that quantises
/// differently is judged on its own terms.
const TOLERANCE_STEPS: f32 = 2.0;

/// What a build found, for the loader report.
///
/// **Every field here is a way the picture is incomplete**, which is why they
/// are counted rather than logged and forgotten: a circuit that silently drew
/// two thirds of itself would look like a working feature.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Report {
    /// Mesh nodes of the wanted class in the `.vex`.
    pub nodes: usize,
    /// Of those, the ones whose hash found a chunk in the `.rcsmodel`.
    pub addressed: usize,
    /// Of those, the ones whose vertex stride the authored box settled.
    pub drawn: usize,
    /// Of those, the ones no rule could settle a vertex stride for.
    ///
    /// Counted rather than derived from `addressed - drawn`, which that
    /// difference used to be: it silently absorbed every other reason a mesh
    /// contributes nothing - a chunk whose submeshes are all strays, and now a
    /// [`see_through`] one - and so read as a stride failure whatever went
    /// wrong.
    pub no_stride: usize,
    /// Triangles emitted.
    pub triangles: usize,
    /// Submeshes dropped because they do not share their mesh's vertex stride.
    pub strays: usize,
    /// Chunks no `.vex` node references, drawn in world space. On a circuit
    /// this is the circuit; see [`build_scene`].
    pub unreferenced: usize,
    /// Vertices whose normal came out of the file rather than off the
    /// triangles. Reported because the difference is visible and the fallback
    /// is silent: see [`face_normals`].
    pub authored_normals: usize,
    /// Chunks whose material says the surface is not drawn solid, and which are
    /// therefore not drawn at all until `.gtf` supplies an alpha. See
    /// [`see_through`].
    pub see_through: usize,
}

impl Report {
    /// One line for a load report.
    #[must_use]
    pub fn describe(&self) -> String {
        format!(
            "{} of {} mesh node(s) drawn from the .rcsmodel ({} triangle(s)); \
             {} addressed no chunk, {} had no recoverable vertex stride",
            self.drawn,
            self.nodes,
            self.triangles,
            self.nodes - self.addressed,
            self.no_stride,
        ) + &match self.strays {
            0 => String::new(),
            n => format!(", {n} submesh(es) dropped as strays"),
        } + &match self.unreferenced {
            0 => String::new(),
            n => format!(", plus {n} chunk(s) no node references, drawn in world space"),
        } + &match self.see_through {
            0 => String::new(),
            n => {
                format!(", {n} chunk(s) left out as see-through, which needs an alpha out of .gtf")
            }
        } + &match self.authored_normals {
            0 => ", lit off face normals computed from the triangles".to_string(),
            n => format!(", {n} authored vertex normal(s)"),
        }
    }
}

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
    build_scene(name, data, &geometry).map(Some)
}

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

/// Every chunk hash any node of the `.vex` mentions anywhere in its payload.
///
/// **A scan rather than a field read, deliberately.** A `Mesh` node carries its
/// hash at `+0x30`, but a `Weapon Pad` carries one too and a `Quake` node
/// carries two at `+0x958` and `+0x9d8` - and those offsets were found by
/// looking, not read out of anything. What this is *for* is the complement:
/// deciding which chunks nothing references, so they can be drawn in world
/// space. Over-collecting is the safe direction there, since a hash wrongly
/// counted as referenced only means that chunk is drawn through its node.
fn referenced(data: &[u8], nodes: &[vex::Node], model: &rcsmodel::Model) -> Vec<u32> {
    let known: std::collections::HashSet<u32> = model.meshes.iter().map(|m| m.hash).collect();
    let order = vex::byte_order(data);
    let mut out = Vec::new();
    for node in nodes {
        let payload = &data[node.payload()];
        for at in (0..payload.len().saturating_sub(3)).step_by(4) {
            let word = order.u32(payload, at);
            if known.contains(&word) {
                out.push(word);
            }
        }
    }
    out
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
/// traffic. The road, the walls and the scenery are among the 904 of 983 chunks
/// no node references, each carrying a **world-space** bias, drawn without a
/// node transform because there is no node.
///
/// So a circuit that drew only the first pass drew its skybox traffic and no
/// track, which is exactly what this looked like before the second existed.
///
/// # Errors
///
/// As [`build`].
pub fn build_scene(label: &str, data: &[u8], model_blob: &[u8]) -> Result<(Model, Report)> {
    let (mut out, mut report) = build(label, data, model_blob, |c| c.mesh)?;

    let model = rcsmodel::Model::parse(model_blob)
        .map_err(|e| anyhow::anyhow!("{label}: the .rcsmodel beside it: {e}"))?;
    let nodes = vex::nodes(data).context("walking the node tree")?;
    let placed = referenced(data, &nodes, &model);

    for mesh in &model.meshes {
        if placed.contains(&mesh.hash) {
            continue;
        }
        if see_through(&model, mesh) {
            report.see_through += 1;
            continue;
        }
        // No authored box to check against, so the stride comes from where the
        // file puts its buffers, and failing that from the geometry's own
        // compactness - see `rcsmodel::Mesh::solve_stride_without_a_box`.
        let Some(stride) = mesh.solve_stride_without_a_box(model_blob) else {
            continue;
        };
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
            report.authored_normals += normals.as_deref().map_or(0, authored);
            emit(
                &mut out,
                &points,
                normals.as_deref(),
                &indices,
                Mat4::IDENTITY,
                None,
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

/// Whether a chunk's material says its surface is not drawn solid.
///
/// # Why this skips the chunk instead of blending it
///
/// **There is no alpha to blend with.** A see-through HD surface gets its
/// coverage from the `.gtf` texture beside its `.rcsmaterial`, and `.gtf` is
/// unread - 7,333 files and 2.4 GiB, the largest single thing in the archives.
/// Nothing else in the file carries one: positions, indices and normals are
/// decoded, and the four bytes at `+0x0a` were measured against the
/// vertex-colour hypothesis and are not one.
///
/// So the three options were to draw these solid, to blend them at `alpha =
/// 1.0`, or to leave them out, and the middle one is not a middle: alpha-over at
/// alpha 1 paints exactly the same pixels as opaque while additionally dropping
/// depth write, and additive would blow every glass panel to white.
///
/// Between the other two, the disc decides it. On Talon's Junction the largest
/// surface in the file is a 63-triangle `clouds` plate spanning 2,011 x 2,195
/// world units - a see-through sheet drawn solid *covers the entire circuit*,
/// and seen from above the track was one white blob. Skipping it is what makes
/// the road, the pit lane and the markings visible at all. The fraction skipped
/// is 4.3 % to 35.9 % of chunks across all 16 circuits
/// (`crates/formats/tests/rcsmodel_material_ground_truth.rs`), so no circuit is
/// mostly this.
///
/// **This is not transparency and is not meant to stand.** It is an honest
/// absence in place of a wrong picture, in the sense `CLAUDE.md` means, and
/// [`Report::see_through`] counts it so a load says so out loud. The condition
/// to reverse it is exactly one thing: when `.gtf` supplies an alpha, these
/// chunks are drawn again, blended with the class
/// [`rcsmodel::Material::blend`] already recovers for each of them.
fn see_through(model: &rcsmodel::Model, mesh: &rcsmodel::Mesh) -> bool {
    model
        .material_of(mesh)
        .is_some_and(rcsmodel::Material::is_see_through)
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
fn emit(
    out: &mut Model,
    points: &[[f32; 3]],
    normals: Option<&[[f32; 3]]>,
    indices: &[u16],
    to_world: Mat4,
    node: Option<u32>,
) {
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
        out.vertices.push(GpuVertex {
            position: p.to_array(),
            // Zero means the file gave none, and `face_normals` derives it.
            normal,
            colour: [1.0, 1.0, 1.0, 1.0],
            texcoord: [0.0, 0.0],
            lit: 1.0,
            anim: 0,
        });
    }
    let centre = centre / points.len() as f32;
    let radius = out.vertices[first_vertex as usize..]
        .iter()
        .map(|v| (Vec3::from_array(v.position) - centre).length())
        .fold(0.0f32, f32::max);

    out.indices
        .extend(indices.iter().map(|&i| first_vertex + u32::from(i)));
    out.draws.push(DrawCall {
        range: first_index..u32::try_from(out.indices.len()).unwrap_or(u32::MAX),
        texture: None,
        bounds: Bounds {
            centre: centre.to_array(),
            radius,
        },
        culled: false,
        blend: None,
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
pub fn build(
    label: &str,
    data: &[u8],
    model_blob: &[u8],
    pick: fn(vex::classes::Classes) -> Option<u32>,
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
        report.addressed += 1;
        if see_through(&model, mesh) {
            report.see_through += 1;
            continue;
        }
        let tolerance = mesh.scale.iter().fold(0.0f32, |a, &b| a.max(b)) * TOLERANCE_STEPS;
        // The box first, because it is the tightest oracle there is. Where it
        // settles nothing, the buffer layout can - and that is safe to fall back
        // to here rather than merely plausible, because every submesh still has
        // to fit the authored box below before a triangle of it is drawn.
        let Some(stride) = mesh
            .solve_stride(model_blob, (min, max), tolerance)
            .or_else(|| mesh.solve_stride_by_layout())
            .or_else(|| mesh.solve_stride_by_normals(model_blob))
        else {
            report.no_stride += 1;
            continue;
        };

        let to_world = Mat4::from_cols_array(&world[index]);
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
            report.authored_normals += normals.as_deref().map_or(0, authored);
            emit(
                &mut out,
                &points,
                normals.as_deref(),
                &indices,
                to_world,
                u32::try_from(index).ok(),
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
    for triangle in model.indices.chunks_exact(3) {
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
mod tests {
    use super::*;

    #[test]
    fn a_sibling_is_the_same_path_with_the_other_extension() {
        assert_eq!(
            sibling_name("/data/ships/assegai/ship.vex").as_deref(),
            Some("/data/ships/assegai/ship.rcsmodel")
        );
        assert_eq!(
            sibling_name(r"Data\Ships\Assegai\Ship.VEX").as_deref(),
            Some(r"Data\Ships\Assegai\Ship.rcsmodel"),
            "both containers match a name case-insensitively"
        );
        assert_eq!(sibling_name("Data\\Tex\\thing.mip"), None);
        assert_eq!(sibling_name(".vex").as_deref(), Some(".rcsmodel"));
    }

    /// An empty model frames at the origin rather than dividing by zero.
    #[test]
    fn a_model_with_no_vertices_still_has_a_usable_radius() {
        let (centre, radius) = bounding_sphere(&[]);
        assert_eq!(centre, [0.0; 3]);
        assert_eq!(radius, 0.0);
    }

    /// The report says what is missing, in the two ways it can be missing.
    #[test]
    fn the_report_names_both_kinds_of_absence() {
        let report = Report {
            nodes: 126,
            addressed: 70,
            drawn: 59,
            no_stride: 11,
            triangles: 4_000,
            strays: 2,
            unreferenced: 639,
            see_through: 257,
            authored_normals: 531_904,
        };
        let line = report.describe();
        assert!(line.contains("59 of 126"), "{line}");
        assert!(line.contains("56 addressed no chunk"), "{line}");
        assert!(
            line.contains("257 chunk(s) left out as see-through"),
            "the picture's largest remaining absence has to be in the line: {line}"
        );
        assert!(
            line.contains("11 had no recoverable vertex stride"),
            "{line}"
        );
        assert!(line.contains("2 submesh(es) dropped as strays"), "{line}");
        assert!(line.contains("531904 authored vertex normal(s)"), "{line}");
        // And the other way round: a model whose vertices carry no normal says
        // that the shading is this project's derivation, not the disc's data.
        let derived = Report {
            authored_normals: 0,
            ..report
        };
        assert!(
            derived
                .describe()
                .contains("lit off face normals computed from the triangles"),
            "{}",
            derived.describe()
        );
        assert!(
            line.contains("639 chunk(s) no node references"),
            "a circuit is mostly this, so the line has to say it: {line}"
        );
    }
}
