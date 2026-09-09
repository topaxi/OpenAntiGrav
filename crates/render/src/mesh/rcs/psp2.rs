//! Drawing Wipeout 2048's `.rcsmodel`.
//!
//! [`super`] builds Wipeout HD's container, which needs the `.vex` beside it -
//! a chunk is addressed by hash and its vertex stride comes from the node's
//! authored bounding box. **2048's container needs neither**: every submesh
//! carries its own counts and its own buffer pointers, and the stride falls out
//! of the buffer packing. So this builder takes the model blob alone.
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

use oag_rcs::rcsmodel::psp2;
use oag_texture::gxt;

use super::Textures;
use crate::mesh::{Bounds, DrawCall, GpuVertex, Model, ModelTexture};

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
        format!(
            "{} triangle(s) over {} submesh(es), {texture}, {} authored \
             normal(s) (rest off face normals); {} unaccounted GPU pointer(s){poisoned}",
            self.triangles, self.submeshes, self.authored_normals, self.unpaired
        )
    }
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

/// Builds every submesh of a 2048 `.rcsmodel` into one model.
///
/// **No transform is composed onto anything**, and that is a property of the
/// files rather than a simplification: a circuit's positions come out in world
/// coordinates and a craft's about its own origin, so both are already in the
/// space their caller draws them in. See [`psp2::Model::positions`].
///
/// `textures` is called once per **distinct material** that names a `.gxt`,
/// never once per draw - see `bind_textures` below.
///
/// # Errors
///
/// Propagates [`psp2::parse`].
pub fn build(label: &str, model_blob: &[u8], textures: Textures<'_>) -> Result<(Model, Report)> {
    let decoded = psp2::parse(model_blob)
        .map_err(|e| anyhow::anyhow!("{label}: the .rcsmodel beside it: {e}"))?;

    let mut model = Model::none(label);
    let mut report = Report {
        unpaired: decoded.unpaired_pointers,
        materials: decoded.materials.len(),
        ..Report::default()
    };
    for submesh in &decoded.submeshes {
        let base = u32::try_from(model.vertices.len()).unwrap_or(u32::MAX);
        let first = u32::try_from(model.indices.len()).unwrap_or(u32::MAX);
        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        for (i, position) in submesh.positions.iter().enumerate() {
            for a in 0..3 {
                lo[a] = lo[a].min(position[a]);
                hi[a] = hi[a].max(position[a]);
            }
            // The file's own normal where this submesh's stride carries one
            // (`psp2::SubMesh::normals`) - a zero otherwise, which is
            // `super::face_normals`'s signal to derive one from the
            // triangles, exactly as it already does for an HD model.
            let normal = submesh.normals.get(i).copied().unwrap_or([0.0; 3]);
            report.authored_normals += usize::from(normal != [0.0; 3]);
            // The file's own diffuse coordinate where the submesh's
            // declaration names one (`psp2::SubMesh::texcoords`), the
            // vertex's origin otherwise - the same fallback shape `normal`
            // above uses, and equally honest: an untextured vertex painted
            // through `(0, 0)` is indistinguishable from one with no texture
            // bound at all, which is what `report.diffuse_texture` being
            // `None` already says.
            let texcoord = submesh.texcoords.get(i).copied().unwrap_or([0.0, 0.0]);
            model.vertices.push(GpuVertex {
                position: *position,
                normal,
                colour: [1.0, 1.0, 1.0, 1.0],
                texcoord,
                lit: 1.0,
                lightmap_texcoord: [0.0, 0.0],
                anim: 0,
                slots: 0,
                xform: 0,
                sun_mask: 1.0,
                specular_exponent: crate::mesh::DEFAULT_SPECULAR_EXPONENT,
            });
        }
        for &index in &submesh.indices {
            model.indices.push(base + u32::from(index));
        }
        let centre: [f32; 3] = std::array::from_fn(|a| (lo[a] + hi[a]) * 0.5);
        let radius = (0..3)
            .map(|a| (hi[a] - lo[a]) * 0.5)
            .fold(0.0f32, |acc, half| acc + half * half)
            .sqrt();
        model.draws.push(DrawCall {
            range: first..u32::try_from(model.indices.len()).unwrap_or(u32::MAX),
            texture: None,
            bounds: Bounds { centre, radius },
            moving: false,
            culled: false,
            blend: None,
            blend_state: None,
            layer: oag_vex::vex::LAYER_DEFAULT,
            node: None,
            chunk: None,
        });
        report.submeshes += 1;
        report.triangles += submesh.triangle_count();
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
            let texture = decode_gxt_texture(path, &blob)?;
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
