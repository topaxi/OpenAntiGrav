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
use oag_texture::gxt;

use super::Textures;
use crate::mesh::{Bounds, DrawCall, Flap, GnfForm, GpuVertex, Model, ModelTexture};
use oag_rcs::gxp::sampler_use;
use oag_rcs::rcsmaterial::name_hash;
use oag_rcs::rcsmodel::psp2::lighting_only;

mod glow;
pub mod placement;
mod transparency;
mod twin_blend;

pub use twin_blend::INHERITED_RATE as TWIN_BLEND_RATE;

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
    /// Distinct lightmap atlases decoded and bound as materials' second
    /// texture - see [`psp2::material::Material::lightmap`].
    pub lightmaps: usize,
    /// Lightmap atlases a material named that did not resolve in the archive or
    /// would not decode. Such a draw binds the black placeholder, which is the
    /// same picture as no lightmap at all.
    pub lightmap_misses: usize,
    /// Textured draws routed to the blended list by their material's state word
    /// (alpha-over, chosen) - see [`transparency`].
    pub blended_draws: usize,
    /// Of [`Self::blended_draws`], those drawn with the factor pair Wipeout HD
    /// authors for a material of the same name, not the alpha-over default - see
    /// [`psp2::lineage_blend`].
    pub inherited_blend_draws: usize,
    /// Textured draws routed to the alpha-tested list (reference `0.5`, chosen).
    pub cutout_draws: usize,
    /// Draws whose material named a texture that did not resolve in the
    /// archive or would not decode - the honest count of what is still
    /// missing, kept apart from a submesh that simply has no material.
    pub unresolved_draws: usize,
    /// Materials that draw an additive glow layer off their own
    /// `Emissive_UV_Offset`/`Scale` - see `glow`'s module doc.
    pub glow_layers: usize,
    /// Materials that scroll their one texture off `speed_multipliaer`.
    pub scrolling_materials: usize,
    /// Materials that scroll off HD's vertex law, **inherited by shader name and
    /// authored rate hashes, no 2048 or Omega instruction read** - see
    /// `vertex_scroll::inherited_rate`.
    pub inherited_scrolls: usize,
    /// Of those, the ones admitted without a readable shader program to
    /// confirm the vertex stage declares `time` (every Omega one: GCN).
    pub inherited_unread: usize,
    /// `nr_twinblend` materials drawn by HD's law - **inherited from HD**, no 2048 or Omega
    /// instruction read - see `twin_blend`.
    pub twin_blend_materials: usize,
    /// `nr_twinblend` materials left as the generic reading drew them: a picture would not
    /// decode, or a submesh carries no `barsUV_blendUV`.
    pub twin_blend_unread: usize,
    /// Submeshes on a placeholder shader (`fc01_dummy`), drawn as the fog colour - see
    /// [`psp2::material::Material::is_placeholder`].
    pub placeholder_submeshes: usize,
    /// Submeshes whose albedo is their material's `Zone_ColourN` uniform - see
    /// [`psp2::material::Material::zone_colour`]. Counted among
    /// [`Self::unresolved_draws`] too, since they have no texture.
    pub zone_colour_submeshes: usize,
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
    /// How the `.gnf` textures this build decoded were held - see
    /// [`ModelTexture::from_gnf_form`]. Counted per decode, so a texture two
    /// materials name is counted once for each.
    pub gnf: crate::mesh::GnfCounts,
    /// Materials whose picture the build refused because the material's own
    /// fragment programs declare it only in their lit permutations
    /// (`oag_rcs::gxp::sampler_use`): a normal, specular or reflection lookup.
    pub lighting_only_materials: usize,
    /// Of those, the materials that drew the constant colour their ambient-only
    /// program names, as a one-texel picture.
    pub constant_colour_materials: usize,
    /// Of those, the ones whose answer is 2048's, carried over because the
    /// material's record matches it exactly and its own programs (PS4) are
    /// unread - `oag_rcs::rcsmodel::psp2::lighting_only`.
    pub inherited_lighting_only: usize,
}

/// The vertex colour of a `fc01_dummy` submesh before the loader knows the
/// fog: negative components no authored colour has. `oag_raceplay` replaces it
/// with the circuit's `fogColour`; one left in place is drawn black.
pub const FOG_FLAT: [f32; 4] = [-1.0, -1.0, -1.0, 1.0];

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
        let placeholders = if self.placeholder_submeshes == 0 {
            String::new()
        } else {
            format!(
                "; {} submesh(es) on fc01_dummy drawn as the circuit's fog colour (its three-instruction program outputs `fogColour` and nothing else)",
                self.placeholder_submeshes
            )
        };
        let scrolls = if self.glow_layers + self.scrolling_materials + self.inherited_scrolls == 0 {
            String::new()
        } else {
            format!(
                "; {} glow layer(s) and {} plain scroll(s) off the materials' own uniforms \
                 (rates chosen, not measured), {} scroll(s) inherited from HD's vertex law by \
                 shader name and authored rate (no instruction read here)",
                self.glow_layers, self.scrolling_materials, self.inherited_scrolls
            )
        };
        let twins = if self.twin_blend_materials + self.twin_blend_unread == 0 {
            String::new()
        } else {
            format!(
                "; {} nr_twinblend material(s) drawn by HD's law (two pictures, a wipe scrolling at HD's rate, bars; inherited from HD, not measured here), {} left as the generic reading",
                self.twin_blend_materials, self.twin_blend_unread
            )
        };
        let zone_colours = if self.zone_colour_submeshes == 0 {
            String::new()
        } else {
            format!(
                "; {} submesh(es) take a Zone_ColourN uniform as their colour, drawn unlit (chosen, not measured)",
                self.zone_colour_submeshes
            )
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
        let see_through = if self.blended_draws + self.cutout_draws == 0 {
            String::new()
        } else {
            format!(
                "; {} draw(s) blended off the state word ({} with HD's authored factors for the same material name, the rest alpha-over, chosen: neither title authors factors) and {} alpha-tested at 0.5 (chosen)",
                self.blended_draws, self.inherited_blend_draws, self.cutout_draws
            )
        };
        let lightmaps = if self.lightmaps == 0 && self.lightmap_misses == 0 {
            String::new()
        } else {
            format!(
                "; {} lightmap atlas(es) bound, {} unresolved",
                self.lightmaps, self.lightmap_misses
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
        let gnf = self.gnf.describe();
        let lighting_only = if self.lighting_only_materials == 0 {
            String::new()
        } else {
            format!(
                "; {} material(s) bind no picture from a sampler their own programs read only for lighting ({} drew the constant colour the unlit program names, {} of them 2048's answer carried over where the record matches)",
                self.lighting_only_materials,
                self.constant_colour_materials,
                self.inherited_lighting_only
            )
        };
        format!(
            "{} triangle(s) over {} submesh(es), {texture}, {} authored \
             normal(s) (rest off face normals); {} unaccounted GPU pointer(s){poisoned}{placeholders}{zone_colours}{scrolls}{twins}{tangents}{lightmaps}{see_through}{nodes}{gnf}{lighting_only}",
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

/// The Zone-mode model that sits beside a 2048 circuit's `track.vex`:
/// `trackZone.rcsmodel`, whose `.rcsskeleton`, `.rcsanimclip` and `.pvs`
/// carry the same stem.
///
/// **A model of its own, not a second skeleton for `track.rcsmodel`**: on
/// `altima` it is 37 MB against the race model's 17, with its own PVS. The
/// reversed circuit ships no Zone model, so both directions map to the same
/// file. `None` for a name that is not a `.vex`.
#[must_use]
pub fn zone_model_name(vex_name: &str) -> Option<String> {
    let dir_end = vex_name.rfind(['\\', '/']).map_or(0, |at| at + 1);
    vex_name[dir_end..]
        .to_ascii_lowercase()
        .ends_with(".vex")
        .then(|| format!("{}trackZone.rcsmodel", &vex_name[..dir_end]))
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

/// Decodes a material's diffuse texture in whichever container its path names.
///
/// **Offered to the texture sink** ([`crate::mesh_render::TextureSinkScope`]),
/// so a load that has a device uploads each texture as it is decoded.
fn decode_material_texture(path: &str, blob: &[u8], report: &mut Report) -> Option<ModelTexture> {
    let (texture, form) = decode_texture_form(path, blob)?;
    if let Some(form) = form {
        report.gnf.record(form);
    }
    Some(crate::mesh_render::offer_to_texture_sink(texture))
}

/// [`decode_material_texture`]'s decode alone: the texture, and the form a
/// `.gnf` took for the report's counts.
fn decode_texture_form(path: &str, blob: &[u8]) -> Option<(ModelTexture, Option<GnfForm>)> {
    if path.to_ascii_lowercase().ends_with(".gnf") {
        ModelTexture::from_gnf_form(path, blob).map(|(texture, form)| (texture, Some(form)))
    } else {
        decode_gxt_texture(path, blob).map(|texture| (texture, None))
    }
}

/// A diffuse decoded once per path, and the form to count again on each
/// material that shares it, so the report reads as it did per material.
type Decoded = Option<(std::sync::Arc<ModelTexture>, Option<GnfForm>)>;

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
    build_planned(
        label,
        model_blob,
        &|scene| placement::plan(scene, animation),
        textures,
    )
}

/// [`build`], with the motion taken from the `.vex` the model was exported
/// beside - see [`placement::plan_from_vex`]. What Omega's front-end scene
/// needs: it ships no `.rcsskeleton` or `.rcsanimclip`.
///
/// # Errors
///
/// Propagates [`psp2::parse`].
pub fn build_with_vex(
    label: &str,
    model_blob: &[u8],
    vex_blob: &[u8],
    textures: Textures<'_>,
) -> Result<(Model, Report)> {
    build_planned(
        label,
        model_blob,
        &|scene| placement::plan_from_vex(scene, vex_blob),
        textures,
    )
}

/// Which side an airbrake mesh is, by the artists' own name: the last
/// component of its Maya path, `Airbrake_Left` or `Airbrake_Right` (the shape
/// node under it, `Airbrake_LeftShape`, is the same mesh).
fn flap_side(path: &str) -> Option<usize> {
    let leaf = path.rsplit(['|', '/']).next().unwrap_or(path);
    let leaf = leaf.strip_suffix("Shape").unwrap_or(leaf);
    if leaf.eq_ignore_ascii_case("Airbrake_Left") {
        Some(0)
    } else if leaf.eq_ignore_ascii_case("Airbrake_Right") {
        Some(1)
    } else {
        None
    }
}

fn build_planned(
    label: &str,
    model_blob: &[u8],
    planner: &dyn Fn(&psp2::nodes::Scene) -> placement::Plan,
    textures: Textures<'_>,
) -> Result<(Model, Report)> {
    let decoded = psp2::parse(model_blob)
        .map_err(|e| anyhow::anyhow!("{label}: the .rcsmodel beside it: {e}"))?;
    let plan = planner(&decoded.scene);

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
    // Each material's own scroll, resolved before any vertex is written; the
    // glow layer's emissive texture is decoded here too, so a layer exists
    // only where its picture does.
    let mut glow = glow::plan(&decoded.materials, &mut *textures, &mut report);
    // `nr_twinblend` takes one more entry of the same table.
    let twin = twin_blend::plan(&decoded, &mut glow.emissive, &mut *textures, &mut report);
    model.emissive = glow.emissive.clone();
    model.anim_tracks = glow.tracks.clone();
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
        // `fc01_dummy`'s fragment program samples nothing and declares one
        // uniform, `fogColour`; its three instructions leave the circuit's fog
        // colour as the pixel (docs/formats/2048-material-params.md, "What
        // `fc01_dummy` outputs"). The colour is the scene's, not the model's,
        // so these vertices carry [`FOG_FLAT`] and the loader that knows the
        // fog writes it in.
        let placeholder = submesh
            .material
            .and_then(|m| decoded.materials.get(m))
            .is_some_and(|m| m.is_placeholder());
        report.placeholder_submeshes += usize::from(placeholder && !place.hidden);
        report.unplaced += usize::from(place.unplaced);
        // The second texture is this material's lightmap, where its submesh's
        // own declaration carries a `lightmapUV` - which is the same set of
        // submeshes, exactly (`oag_rcs`'s declaration ground truth).
        let role = if submesh.lightmap_texcoords.is_empty() {
            0
        } else {
            crate::mesh::slots::SECOND_IS_LIGHTMAP
        };
        // A Zone material's colour is drawn as it is: its own colour, unlit.
        // **Chosen, not measured.** Its fragment program also declares a
        // lightmap, `liveLighting0` and a `Zone_ColourN_Emissive` scalar
        // (`0.5` on all eight), and how the three combine is GPU bytecode
        // nobody here has decoded. Both readings were drawn on `altima`:
        // lit, the colour is multiplied by a sun of `2.0 1.8 1.7` and every
        // surface blows out to white; unlit, the eight colours read as eight
        // colours. The scalar is read and not applied.
        let zone = submesh
            .material
            .and_then(|m| decoded.materials.get(m))
            .and_then(|m| m.zone_colour());
        report.zone_colour_submeshes += usize::from(zone.is_some());
        // HD's `nr_twinblend`: unlit, three coordinate pairs, one glow-table entry.
        let twin_here = twin.of(submesh.material);
        let (colour, lit, role) = match zone {
            None if twin_here.is_some() => (
                [1.0; 4],
                0.0,
                crate::mesh::slots::NO_AMBIENT
                    | crate::mesh::slots::NO_SUN
                    | twin_here.map_or(0, |t| t.slot << crate::mesh::slots::MATERIAL_SHIFT),
            ),
            None if placeholder => (
                FOG_FLAT,
                0.0,
                role | crate::mesh::slots::NO_AMBIENT | crate::mesh::slots::NO_SUN,
            ),
            Some([r, g, b]) => (
                [r, g, b, 1.0],
                0.0,
                role | crate::mesh::slots::NO_AMBIENT | crate::mesh::slots::NO_SUN,
            ),
            None => ([1.0; 4], 1.0, role),
        };
        let to_world = Mat4::from_cols_array(&place.to_world);
        let at_zero = Mat4::from_cols_array(&place.world_at_zero);
        // Normals through the bake's inverse transpose: 166 skeleton nodes
        // across the corpus scale non-uniformly (88 of them carry meshes),
        // and a normal turned by the plain matrix skews under one. A moving
        // node's normals go through the shader's node matrix instead, which
        // takes no inverse transpose - `mesh.wesl`'s own caveat, on lit
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
            // The atlas coordinate a lightmapped submesh's own declaration
            // names, and nothing where it names none - so a Vita model, which
            // never has one, is unchanged to the bit.
            let lightmap_texcoord = submesh
                .lightmap_texcoords
                .get(i)
                .copied()
                .unwrap_or([0.0, 0.0]);
            // The wipe's pair rides the lightmap coordinate and the bars' the second set, as
            // HD's `nr_twinblend` binds them.
            let pair = twin_here.and_then(|_| submesh.bars_blend_texcoords.get(i));
            let lightmap_texcoord = pair.map_or(lightmap_texcoord, |p| [p[2], p[3]]);
            let texcoord2 = pair.map_or([0.0, 0.0], |p| [p[0], p[1]]);
            model.vertices.push(GpuVertex {
                quake: 0.0,
                position,
                normal,
                colour,
                texcoord,
                lit,
                lightmap_texcoord,
                anim: glow.anim(submesh.material),
                slots: if submesh.lightmap_texcoords.is_empty() && twin_here.is_none() {
                    role | glow.role(submesh.material)
                } else {
                    role
                },
                xform: place.xform,
                sun_mask: 1.0,
                specular_exponent: crate::mesh::DEFAULT_SPECULAR_EXPONENT,
                glow: 0.0,
                texcoord2,
                tangent: 0,
            });
        }
        if !place.hidden {
            for &index in &submesh.indices {
                model.indices.push(base + u32::from(index));
            }
            // An airbrake is a node-bound mesh named `Airbrake_Left` or
            // `Airbrake_Right` whose vertices are about the flap's own hinge,
            // which is the node's matrix the bake above has just applied: the
            // same hinge-frame arrangement a Pulse flap has, so the same
            // `Flap` swings it. See `Flap::deflect`.
            let side = submesh
                .mesh
                .and_then(|m| decoded.scene.meshes.get(m))
                .and_then(|m| flap_side(&m.name));
            if let (Some(side), true) = (side, place.xform == 0) {
                let end = u32::try_from(model.vertices.len()).unwrap_or(u32::MAX);
                Flap::record(&mut model.airbrakes[side], base..end, to_world);
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
            writes_depth: false,
            range: first..u32::try_from(model.indices.len()).unwrap_or(u32::MAX),
            texture: None,
            bounds: Bounds { centre, radius },
            moving: place.xform != 0,
            culled: false,
            blend: None,
            blend_state: None,
            depth_point: None,
            layer: oag_vex::vex::LAYER_DEFAULT,
            node: None,
            // The mesh object this submesh belongs to: the join key for the
            // circuit's `.pvs`, whose bitmaps are indexed by exactly this
            // number (`oag_rcs::hd_pvs`, the `Psp2` dialect). `None` when the
            // node table did not read, which the PVS test always allows.
            chunk: submesh.mesh.and_then(|mesh| u32::try_from(mesh).ok()),
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

    let ps4 = psp2::is_ps4(model_blob);
    bind_textures(&decoded, (&glow, &twin), &mut model, &mut report, textures, ps4);
    transparency::route(&decoded, &mut model, &mut report);
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
    (glow, twin): (&glow::Plan, &twin_blend::Plan),
    model: &mut Model,
    report: &mut Report,
    textures: Textures<'_>,
    ps4: bool,
) {
    // `build` pushes exactly one draw per submesh, in order, which is what
    // makes this zip a binding rather than an off-by-one waiting to happen -
    // and a misbound draw would paint a submesh with its neighbour's texture,
    // the one outcome `psp2::parse` already refuses to allow.
    debug_assert_eq!(model.draws.len(), decoded.submeshes.len());
    let mut cache: Vec<Option<Option<usize>>> = vec![None; decoded.materials.len()];
    // A material file's programs say which of its samplers feed only the
    // lighting; read once per distinct file.
    let mut rules: std::collections::HashMap<&str, Option<LightingRule>> =
        std::collections::HashMap::new();
    // A lightmap is shared by however many materials name the same file, and a
    // 2,048-square atlas is 16 MiB decoded, so decode each path once.
    let mut atlases: std::collections::HashMap<&str, Option<std::sync::Arc<ModelTexture>>> =
        std::collections::HashMap::new();
    // Materials share a `.gxt` between them as well: Altima's track resolves
    // 523 material slots to 164 distinct files. Each slot still gets its own
    // entry, so a lightmap stays positionally beside its diffuse.
    let mut diffuses: std::collections::HashMap<&str, Decoded> = std::collections::HashMap::new();
    for (draw, submesh) in model.draws.iter_mut().zip(&decoded.submeshes) {
        let Some(index) = submesh.material else {
            continue;
        };
        let Some(slot) = cache.get_mut(index) else {
            continue;
        };
        let resolved = *slot.get_or_insert_with(|| {
            let layer = glow.layers.get(index).and_then(Option::as_ref);
            let twin_here = twin.of(Some(index));
            let material = &decoded.materials[index];
            let rule = lighting_rule(material, ps4, &mut rules, &mut *textures);
            let refused: Vec<u32> = rule.as_ref().map_or_else(Vec::new, |r| r.refused.clone());
            let path = match layer {
                _ if twin_here.is_some() => twin_here.map(|t| t.first.as_str()),
                Some(layer) => Some(layer.diffuse.as_str()),
                None => material.diffuse_texture_without(&refused),
            };
            let refused_here = layer.is_none()
                && material
                    .samplers
                    .iter()
                    .any(|(hash, _)| refused.contains(hash));
            report.lighting_only_materials += usize::from(refused_here && path.is_none());
            let (texture, form) = match path {
                Some(path) => diffuses
                    .entry(path)
                    .or_insert_with(|| {
                        let blob = textures(path)?;
                        let (texture, form) = decode_texture_form(path, &blob)?;
                        let texture = crate::mesh_render::offer_to_texture_sink(texture);
                        Some((std::sync::Arc::new(texture), form))
                    })
                    .clone()?,
                None => {
                    let colour = refused_here
                        .then(|| constant_colour(material, rule.as_ref()?.colour?))
                        .flatten()?;
                    report.constant_colour_materials += 1;
                    report.inherited_lighting_only +=
                        usize::from(rule.as_ref().is_some_and(|r| r.inherited));
                    (std::sync::Arc::new(colour), None)
                }
            };
            let path = path.unwrap_or("constant colour");
            if let Some(form) = form {
                report.gnf.record(form);
            }
            let at = model.textures.len();
            model.textures.push(Some(texture));
            // Positionally beside the diffuse, as `Model::lightmaps` is
            // defined; left empty for a model no material of which names one,
            // which is every Vita model.
            if twin.any() {
                debug_assert_eq!(model.pad_masks.len(), at);
                model.pad_masks.push(twin_here.map(|t| t.wipe.clone()));
                model.wave_maps.push(twin_here.map(|t| t.bars.clone()));
            }
            if glow.has_layers() || twin.any() || decoded.materials.iter().any(|m| m.lightmap.is_some()) {
                debug_assert_eq!(model.lightmaps.len(), at);
                let atlas = twin_here.map(|t| t.second.clone()).or_else(|| layer.map(|l| l.texture.clone())).or_else(|| {
                    decoded.materials[index]
                        .lightmap
                        .as_deref()
                        .and_then(|lmap| {
                            let entry = atlases.entry(lmap).or_insert_with(|| {
                                let atlas = textures(lmap)
                                    .and_then(|blob| decode_material_texture(lmap, &blob, report))
                                    .map(std::sync::Arc::new);
                                report.lightmaps += usize::from(atlas.is_some());
                                report.lightmap_misses += usize::from(atlas.is_none());
                                atlas
                            });
                            entry.clone()
                        })
                });
                model.lightmaps.push(atlas);
            }
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

/// What decides a material's lighting-only samplers: their name hashes and the
/// colour parameter's, from its own programs or from 2048's answer.
#[derive(Debug, Clone)]
struct LightingRule {
    refused: Vec<u32>,
    colour: Option<u32>,
    inherited: bool,
}

/// The material's rule, read once per `.rcsmaterial` file. A Vita material
/// reads its own programs; a PS4 one, whose programs are unread, takes 2048's
/// answer where its instance record matches it.
fn lighting_rule<'m>(
    material: &'m psp2::material::Material,
    ps4: bool,
    cache: &mut std::collections::HashMap<&'m str, Option<LightingRule>>,
    textures: &mut dyn FnMut(&str) -> Option<Vec<u8>>,
) -> Option<LightingRule> {
    if ps4 {
        // Judged per instance: two materials may share a `.rcsmaterial` path
        // and differ in their record, and the answer is the record's.
        let row = lighting_only::inherited(material)?;
        return Some(LightingRule {
            refused: row.lighting_only.to_vec(),
            colour: Some(row.colour),
            inherited: true,
        });
    }
    cache
        .entry(material.name.as_str())
        .or_insert_with(|| {
            let uses = sampler_use::sampler_use(&textures(&material.name)?)?;
            // Only where the ambient-only program samples nothing: the case
            // `psp2_sampler_use_ground_truth` pins. A lit-only sampler beside
            // a picture the program does sample is left as it was.
            uses.colourless().then(|| LightingRule {
                refused: uses.lighting_only.iter().map(|n| name_hash(n)).collect(),
                colour: uses.colour_uniform.as_deref().map(name_hash),
                inherited: false,
            })
        })
        .clone()
}

/// The constant a lighting-only material takes its colour from: the one
/// uniform its ambient-only program declares besides the ambient and fog
/// colours, read from the material's own value, as a one-texel picture.
///
/// Unorm bytes, no sRGB step, the way `skin::constant_diffuse` binds HD's.
/// `None` where the program names no single colour or the material carries no
/// value for it, which leaves the draw unresolved and counted as such.
fn constant_colour(material: &psp2::material::Material, colour: u32) -> Option<ModelTexture> {
    let name = format!("{colour:08x}");
    let value = material.param(colour)?;
    let byte = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u8;
    let channel = |i: usize| byte(value.get(i).copied().unwrap_or(1.0));
    Some(crate::mesh_render::offer_to_texture_sink(
        ModelTexture::rgba8(
            format!("constant colour {name}"),
            1,
            1,
            vec![channel(0), channel(1), channel(2), 255],
            None,
        ),
    ))
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
