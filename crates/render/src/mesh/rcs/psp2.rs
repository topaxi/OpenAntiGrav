//! Drawing Wipeout 2048's `.rcsmodel`.
//!
//! [`super`] builds Wipeout HD's container, which needs the `.vex` beside it -
//! a chunk is addressed by hash and its vertex stride comes from the node's
//! authored bounding box. **2048's container needs neither**: every submesh
//! carries its own counts and its own buffer pointers, and the stride falls out
//! of the buffer packing. So this builder takes the model blob alone.
//!
//! **A node-bound submesh is placed, and moved, by its node.** The model's
//! own table names a node for 130 of `altima`'s 1,153 meshes (a
//! median 1,086 units from where they are authored, as-is), and the
//! `.rcsskeleton`/`.rcsanimclip` beside the circuit animate those nodes -
//! see [`placement`] for how a node either bakes through its static world
//! matrix or takes a slot of the shader's node table, and
//! `docs/formats/2048-animation.md` for the format and its evidence.
//!
//! What it draws is positions, triangles, normals, diffuse texture
//! coordinates - the file's own where a submesh's stride carries one, computed
//! off the triangles where normals do not (see [`super::face_normals`]) - and
//! **each submesh's own material's texture**, through the per-submesh material
//! index [`psp2::SubMesh::material`] carries.
//!
//! **That binding is per-submesh, not per-model**, which is what makes a
//! multi-material model - the craft (6 materials) and the circuit (527) both
//! are - paint more than one texture. Each distinct material resolves to at
//! most one decoded [`ModelTexture`], shared by every draw that names it, so a
//! 2,800-submesh circuit uploads a few hundred textures rather than 2,800
//! copies of them. A submesh whose material has no index, no texture path, no
//! archive entry or a texture that will not decode gets **no texture at all**
//! rather than a neighbour's - see [`Report::describe`], which counts each of
//! those separately instead of leaving a silent difference.

use anyhow::Result;

use oag_core::math::{Mat4, Vec3};
use oag_rcs::rcsmodel::psp2;
use oag_texture::{gnf, gxt};

use super::Textures;
use crate::mesh::{Bounds, DrawCall, GpuVertex, Model, ModelTexture};

pub mod placement;

pub use placement::Animation;

/// What one build found.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Report {
    /// Submeshes drawn.
    pub submeshes: usize,
    /// Triangles emitted.
    pub triangles: usize,
    /// Relocation entries naming a GPU pointer this reading does not account
    /// for - see [`psp2::Model::unpaired_pointers`].
    pub unpaired: usize,
    /// Vertices whose normal came from the file rather than off the
    /// triangles - see [`super::face_normals`]. Reported for the same reason
    /// HD's own build counts it: the fallback is silent and the difference is
    /// visible.
    pub authored_normals: usize,
    /// Vertices whose decoded diffuse texcoord was non-finite and got
    /// substituted with the origin - see
    /// [`psp2::SubMesh::non_finite_texcoords`]. `0` on almost every model;
    /// surfaced because a hidden substitution here is a hidden difference
    /// from what the file authored, the same reasoning
    /// [`Self::authored_normals`] already applies to its own fallback.
    pub non_finite_texcoords: usize,
    /// Vertices whose submesh's declaration named a `tangent` attribute for
    /// this stride, so [`psp2::SubMesh::tangents`] carries one - **not** a
    /// count of non-zero/well-formed tangents the way [`Self::authored_normals`]
    /// counts real values against a zero sentinel. `docs/formats/2048-rcsmodel.md`'s
    /// own reading measures ~42% of decoded tangents as short or near-zero
    /// (degenerate at UV poles/seams), so this field answers "was the
    /// attribute present", not "was the value good" - naming it
    /// `authored_tangents` would have implied the latter.
    ///
    /// **Decoded and counted, not yet drawn with.** Nothing in this title's
    /// mesh path samples a tangent-space normal map, so there is no visual
    /// difference between a model with this at `0` and one with it equal to
    /// [`Self::submeshes`]' own vertex total - see
    /// `docs/formats/2048-rcsmodel.md#tangent-is-cracked-too-at-a-lower-confidence`
    /// for why this is reported anyway (confidence 76, not yet wired into a
    /// consumer) rather than silently dropped on the way from `oag_rcs`.
    pub decoded_tangents: usize,
    /// How many materials this model's own table names, whether or not this
    /// build could bind one to a draw - see [`psp2::material`]. `0` for a
    /// model with no readable table, which is not the same state as a model
    /// this build simply chose not to texture.
    pub materials: usize,
    /// The first texture this build painted anything with, for a one-line
    /// report. `None` when nothing painted at all.
    pub diffuse_texture: Option<String>,
    /// Distinct `.gxt` files decoded and uploaded.
    pub textures: usize,
    /// Draws that got a texture bound.
    pub textured_draws: usize,
    /// Draws whose material named a texture that did not resolve in the
    /// archive or would not decode - the honest count of what is still
    /// missing, kept apart from a submesh that simply has no material.
    pub unresolved_draws: usize,
    /// Submeshes bound to a node of the model's own table and placed by
    /// it - through the skeleton where one was given, through the model's
    /// bind matrix otherwise. See [`placement`].
    pub node_bound: usize,
    /// Of those, the submeshes the shader's node table moves per frame.
    pub moving: usize,
    /// Nodes the table moves - the entries of [`Model::anim_nodes`].
    pub anim_nodes: usize,
    /// Submeshes under a node authored invisible that nothing ever shows,
    /// not emitted.
    pub hidden: usize,
    /// Submeshes on a node the file gives no transform - no written bind
    /// matrix, no skeleton entry - not emitted, since nothing authored says
    /// where they go. Counted apart from [`Self::hidden`], which is the
    /// file's own choice. See [`placement::Placement::unplaced`].
    pub unplaced: usize,
    /// Model nodes the skeleton did not name, placed by the model's own bind
    /// matrix - see [`placement::Plan::unmatched`].
    pub unmatched_nodes: usize,
    /// Nodes frozen at time zero past the table's ceiling - see
    /// [`placement::Plan::frozen`].
    pub frozen_nodes: usize,
}

impl Report {
    /// One line for a load report.
    #[must_use]
    pub fn describe(&self) -> String {
        let texture = match (&self.diffuse_texture, self.materials) {
            (Some(path), n) => format!(
                "{}/{} draw(s) textured from {} of {n} material(s) (e.g. {path}), \
                 {} unresolved",
                self.textured_draws, self.submeshes, self.textures, self.unresolved_draws
            ),
            (None, 0) => "untextured, no material table".to_string(),
            (None, n) => format!("untextured, {n} material(s), none of whose textures resolved"),
        };
        let poisoned = if self.non_finite_texcoords == 0 {
            String::new()
        } else {
            format!(
                ", {} non-finite texcoord(s) zeroed",
                self.non_finite_texcoords
            )
        };
        let tangents = if self.decoded_tangents == 0 {
            String::new()
        } else {
            format!(
                "; {} tangent(s) decoded, unused (no normal-map consumer yet)",
                self.decoded_tangents
            )
        };
        let nodes = if self.node_bound == 0 {
            String::new()
        } else {
            let frozen = if self.frozen_nodes == 0 {
                String::new()
            } else {
                format!(", {} frozen past the table", self.frozen_nodes)
            };
            let unmatched = if self.unmatched_nodes == 0 {
                String::new()
            } else {
                format!(", {} node(s) not in the skeleton", self.unmatched_nodes)
            };
            let hidden = if self.hidden == 0 {
                String::new()
            } else {
                format!(", {} hidden", self.hidden)
            };
            let unplaced = if self.unplaced == 0 {
                String::new()
            } else {
                format!(
                    ", {} on a node with no matrix and no skeleton entry, not drawn",
                    self.unplaced
                )
            };
            format!(
                "; {} node-bound submesh(es), {} moving on {} animated node(s){hidden}{unplaced}{frozen}{unmatched}",
                self.node_bound, self.moving, self.anim_nodes
            )
        };
        format!(
            "{} triangle(s) over {} submesh(es), {texture}, {} authored \
             normal(s) (rest off face normals); {} unaccounted GPU pointer(s){poisoned}{tangents}{nodes}",
            self.triangles, self.submeshes, self.authored_normals, self.unpaired
        )
    }
}

/// The `.rcsskeleton` and `.rcsanimclip` a Wipeout 2048 circuit ships beside
/// its `.vex`, in that order - the pair [`Animation::parse`] takes.
///
/// Same stem rule as [`super::sibling_name`]. Wipeout HD ships neither (its scenery
/// animation is in the `.vex` itself), so a caller reads them where they
/// resolve and passes `None` where they do not.
#[must_use]
pub fn animation_names(vex_name: &str) -> Option<(String, String)> {
    animation_names_beside(&super::sibling_name(vex_name)?)
}

/// The `.rcsskeleton` and `.rcsanimclip` that sit beside an `.rcsmodel`
/// already found, whatever its spelling - `track.rcsmodel` gives
/// `track.rcsskeleton`, and the Omega Collection's `track.final.rcsmodel`
/// gives `track.final.rcsskeleton`. See [`super::sibling_name_cooked`].
#[must_use]
pub fn animation_names_beside(model_name: &str) -> Option<(String, String)> {
    let stem = model_name.strip_suffix(".rcsmodel")?;
    Some((format!("{stem}.rcsskeleton"), format!("{stem}.rcsanimclip")))
}

/// Whether a blob is 2048's container rather than Wipeout HD's.
///
/// Cheap and exact: the two containers disagree on their very first word, and
/// neither value is a plausible reading of the other.
#[must_use]
pub fn is_psp2(model_blob: &[u8]) -> bool {
    model_blob
        .get(0..4)
        .is_some_and(|b| u32::from_le_bytes(b.try_into().expect("four bytes")) == psp2::MAGIC)
}

/// Decodes one material's diffuse `.gxt`, or `None` for a blob that will not
/// parse as one or names a texture this build cannot decode.
///
/// **A `.gxt` that will not decode draws nothing rather than something** - the
/// same rule [`super::skin::decode_texture`] applies to a Wipeout HD `.gtf`.
fn decode_gxt_texture(label: &str, blob: &[u8]) -> Option<ModelTexture> {
    let parsed = gxt::Gxt::parse(blob).ok()?;
    let texture = parsed.only()?;
    let rgba = texture.to_rgba(blob).ok()?;
    Some(ModelTexture::rgba8(
        label.to_string(),
        u32::from(texture.width),
        u32::from(texture.height),
        rgba.into_iter().flatten().collect(),
        None,
    ))
}

/// Decodes one material's diffuse `.gnf` - the PS4 Omega Collection's texture
/// container, which its `.rcsmodel` names where 2048's names a `.gxt`.
///
/// **The same rule as [`decode_gxt_texture`]**: a `.gnf` that will not parse,
/// or that [`oag_texture::gnf`] refuses (a corrupt base level, a format with no
/// block decoder), draws nothing rather than something.
fn decode_gnf_texture(label: &str, blob: &[u8]) -> Option<ModelTexture> {
    let parsed = gnf::Texture::parse(blob).ok()?;
    let rgba = parsed.decode(blob).ok()?;
    Some(ModelTexture::rgba8(
        label.to_string(),
        parsed.width,
        parsed.height,
        rgba.into_iter().flatten().collect(),
        None,
    ))
}

/// Decodes a material's diffuse texture in whichever container its path names.
fn decode_material_texture(path: &str, blob: &[u8]) -> Option<ModelTexture> {
    if path.to_ascii_lowercase().ends_with(".gnf") {
        decode_gnf_texture(path, blob)
    } else {
        decode_gxt_texture(path, blob)
    }
}

/// Builds every submesh of a 2048 `.rcsmodel` into one model.
///
/// **A submesh with no node is in the space its caller draws in** - a
/// circuit's in world coordinates, a craft's about its own origin - and
/// nothing is composed onto it. **A node-bound one is in its node's space**
/// and is placed by [`placement::plan`]: baked through the node's static
/// world matrix, or left where it is and moved by the shader's node table
/// when `animation` names keys for it. See [`psp2::Model::positions`] and
/// [`psp2::nodes`].
///
/// `textures` is called once per **distinct material** that names a `.gxt`,
/// never once per draw - see `bind_textures` below.
///
/// # Errors
///
/// Propagates [`psp2::parse`].
pub fn build(
    label: &str,
    model_blob: &[u8],
    animation: Option<&Animation>,
    textures: Textures<'_>,
) -> Result<(Model, Report)> {
    let decoded = psp2::parse(model_blob)
        .map_err(|e| anyhow::anyhow!("{label}: the .rcsmodel beside it: {e}"))?;
    let plan = placement::plan(&decoded.scene, animation);

    let mut model = Model::none(label);
    let mut report = Report {
        unpaired: decoded.unpaired_pointers,
        materials: decoded.materials.len(),
        anim_nodes: plan.anim_nodes.len(),
        unmatched_nodes: plan.unmatched,
        frozen_nodes: plan.frozen,
        ..Report::default()
    };
    model.anim_nodes = plan.anim_nodes;
    // Every draw is pushed in submesh order below, and `bind_textures`
    // zips on that; a hidden submesh still gets its draw, empty, so the
    // zip stays a binding.
    for submesh in &decoded.submeshes {
        let place = submesh
            .node
            .and_then(|n| plan.placements.get(n).copied())
            .unwrap_or(placement::Placement::STATIC);
        report.node_bound += usize::from(submesh.node.is_some());
        report.moving += usize::from(place.xform != 0);
        report.hidden += usize::from(place.hidden && !place.unplaced);
        report.unplaced += usize::from(place.unplaced);
        let to_world = Mat4::from_cols_array(&place.to_world);
        let at_zero = Mat4::from_cols_array(&place.world_at_zero);
        // Normals through the bake's inverse transpose: 166 skeleton nodes
        // across the corpus scale non-uniformly (88 of them carry meshes),
        // and a normal turned by the plain matrix skews under one. A moving
        // node's normals go through the shader's node matrix instead, which
        // takes no inverse transpose - `mesh.wgsl`'s own caveat, on lit
        // geometry here rather than Pulse's prelit.
        let normal_to_world = to_world.inverse().transpose();
        let base = u32::try_from(model.vertices.len()).unwrap_or(u32::MAX);
        let first = u32::try_from(model.indices.len()).unwrap_or(u32::MAX);
        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        for (i, position) in submesh.positions.iter().enumerate() {
            if place.hidden {
                break;
            }
            // Baked through the node's static matrix, or left in the node's
            // own space for the table to move; the bounds are world-space
            // either way, which for a moving node means time zero.
            let position = to_world
                .transform_point3(Vec3::from_array(*position))
                .to_array();
            let placed = if place.xform == 0 {
                position
            } else {
                at_zero
                    .transform_point3(Vec3::from_array(position))
                    .to_array()
            };
            for a in 0..3 {
                lo[a] = lo[a].min(placed[a]);
                hi[a] = hi[a].max(placed[a]);
            }
            // The file's own normal where this submesh's stride carries one
            // (`psp2::SubMesh::normals`) - a zero otherwise, which is
            // `super::face_normals`'s signal to derive one from the
            // triangles, exactly as it already does for an HD model. Turned
            // with the bake and renormalised.
            let normal = submesh.normals.get(i).copied().unwrap_or([0.0; 3]);
            let normal = if normal == [0.0; 3] {
                normal
            } else {
                normal_to_world
                    .transform_vector3(Vec3::from_array(normal))
                    .normalize_or_zero()
                    .to_array()
            };
            report.authored_normals += usize::from(normal != [0.0; 3]);
            // Decoded but not sampled by anything yet - see
            // `Report::decoded_tangents`.
            report.decoded_tangents += usize::from(submesh.tangents.get(i).is_some());
            // The file's own diffuse coordinate where the submesh's
            // declaration names one (`psp2::SubMesh::texcoords`), the
            // vertex's origin otherwise - the same fallback shape `normal`
            // above uses, and equally honest: an untextured vertex painted
            // through `(0, 0)` is indistinguishable from one with no texture
            // bound at all, which is what `report.diffuse_texture` being
            // `None` already says.
            let texcoord = submesh.texcoords.get(i).copied().unwrap_or([0.0, 0.0]);
            model.vertices.push(GpuVertex {
                position,
                normal,
                colour: [1.0, 1.0, 1.0, 1.0],
                texcoord,
                lit: 1.0,
                lightmap_texcoord: [0.0, 0.0],
                anim: 0,
                slots: 0,
                xform: place.xform,
                sun_mask: 1.0,
                specular_exponent: crate::mesh::DEFAULT_SPECULAR_EXPONENT,
                glow: 0.0,
            });
        }
        if !place.hidden {
            for &index in &submesh.indices {
                model.indices.push(base + u32::from(index));
            }
        }
        let centre: [f32; 3] = if place.hidden {
            [0.0; 3]
        } else {
            std::array::from_fn(|a| (lo[a] + hi[a]) * 0.5)
        };
        let radius = if place.hidden {
            0.0
        } else {
            (0..3)
                .map(|a| (hi[a] - lo[a]) * 0.5)
                .fold(0.0f32, |acc, half| acc + half * half)
                .sqrt()
        };
        model.draws.push(DrawCall {
            range: first..u32::try_from(model.indices.len()).unwrap_or(u32::MAX),
            texture: None,
            bounds: Bounds { centre, radius },
            moving: place.xform != 0,
            culled: false,
            blend: None,
            blend_state: None,
            layer: oag_vex::vex::LAYER_DEFAULT,
            node: None,
            chunk: None,
            // A Vita material authors no PSP-style reference.
            alpha_test_ref: None,
        });
        report.submeshes += 1;
        if !place.hidden {
            report.triangles += submesh.triangle_count();
        }
        report.non_finite_texcoords += submesh.non_finite_texcoords;
    }
    model.mesh_count = report.submeshes;
    // Fills in only the vertices left at a zero normal above - a submesh
    // whose stride was too small to carry one (not observed on any of the
    // 993 shipped files). See `super::face_normals`, whose docs make the
    // same fallback argument for the HD container.
    super::face_normals(&mut model);
    finish_bounds(&mut model);

    bind_textures(&decoded, &mut model, &mut report, textures);
    Ok((model, report))
}

/// Binds each draw to its own submesh's material's texture.
///
/// **One decode per distinct material, not per draw.** Altima's circuit has
/// 2,800 submeshes over 527 materials and its materials share `.gxt` files
/// between them, so decoding per draw would decode the same texture hundreds
/// of times; the cache is keyed by material index and every miss is decoded
/// once. A material whose texture does not resolve caches the miss too, so a
/// missing archive entry costs one lookup rather than one per submesh.
fn bind_textures(
    decoded: &psp2::Model,
    model: &mut Model,
    report: &mut Report,
    textures: Textures<'_>,
) {
    // `build` pushes exactly one draw per submesh, in order, which is what
    // makes this zip a binding rather than an off-by-one waiting to happen -
    // and a misbound draw would paint a submesh with its neighbour's texture,
    // the one outcome `psp2::parse` already refuses to allow.
    debug_assert_eq!(model.draws.len(), decoded.submeshes.len());
    let mut cache: Vec<Option<Option<usize>>> = vec![None; decoded.materials.len()];
    for (draw, submesh) in model.draws.iter_mut().zip(&decoded.submeshes) {
        let Some(index) = submesh.material else {
            continue;
        };
        let Some(slot) = cache.get_mut(index) else {
            continue;
        };
        let resolved = *slot.get_or_insert_with(|| {
            let path = decoded.materials[index].diffuse_texture()?;
            let blob = textures(path)?;
            let texture = decode_material_texture(path, &blob)?;
            let at = model.textures.len();
            model.textures.push(Some(std::sync::Arc::new(texture)));
            if report.diffuse_texture.is_none() {
                report.diffuse_texture = Some(path.to_string());
            }
            report.textures += 1;
            Some(at)
        });
        match resolved {
            Some(at) => {
                draw.texture = Some(at);
                report.textured_draws += 1;
            }
            None => report.unresolved_draws += 1,
        }
    }
}

/// The model's own centre and radius, from every vertex it ended up with.
fn finish_bounds(model: &mut Model) {
    let mut lo = [f32::MAX; 3];
    let mut hi = [f32::MIN; 3];
    for vertex in &model.vertices {
        for a in 0..3 {
            lo[a] = lo[a].min(vertex.position[a]);
            hi[a] = hi[a].max(vertex.position[a]);
        }
    }
    if model.vertices.is_empty() {
        return;
    }
    model.centre = std::array::from_fn(|a| (lo[a] + hi[a]) * 0.5);
    model.radius = model
        .vertices
        .iter()
        .map(|v| {
            (0..3)
                .map(|a| (v.position[a] - model.centre[a]).powi(2))
                .sum::<f32>()
        })
        .fold(0.0f32, f32::max)
        .sqrt();
}
