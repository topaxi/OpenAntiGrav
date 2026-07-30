//! Loads a `.vex` model out of an archive and flattens it for the GPU.

use anyhow::{Context, Result, bail};
use oag_assets::Archive;
use oag_formats::{ps2_texture, vex, wad};

/// A vertex as the mesh shader expects it.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub colour: [f32; 4],
    pub texcoord: [f32; 2],
    /// 1.0 to apply the viewer's light rig, 0.0 for geometry that is prelit.
    ///
    /// Track batches carry vertex colours *and* normals, and their colours are
    /// baked lighting. Lighting them again multiplies two lighting terms and the
    /// track comes out nearly black. Ship batches have no vertex colour, so they
    /// need the rig. This is the viewer's own choice, not the game's: the GE
    /// decides per draw from state we have not recovered.
    pub lit: f32,
    /// 1.0 for a vertex whose texture is the shared blink-light palette
    /// (`colours_flashing_GLOW.tga`), 0.0 otherwise. See
    /// [`is_blink_light_texture`] for what qualifies and the evidence behind
    /// it. The shader adds a per-frame scroll offset to this vertex's V
    /// texture coordinate when `glow` is 1.0, which is the whole animation:
    /// the light's colour and brightness curve live in the texture, not in
    /// any code here.
    pub glow: f32,
}

/// A run of indices sharing one texture.
#[derive(Debug)]
pub struct DrawCall {
    pub range: std::ops::Range<u32>,
    /// Index into [`Model::textures`], or `None` for untextured.
    pub texture: Option<usize>,
}

/// Whether a decoded texture's name identifies its surface as the shared
/// blink-light palette, animated by scrolling its V (row) coordinate.
///
/// **Confidence: 85.** Every one of the 8 playable PSP ships carries a mesh
/// named `glowingShape` whose material resolves to the exact same shared
/// texture, `Data\Tex\colours_flashing_GLOW.tga` - not a per-ship asset, a
/// common one. The ship-specific mesh names first noticed on Feisar
/// (`underbrake_flashrightShape`/`underbrake_flashleftShape`) and Triakis
/// (`flasherShape`/`flasher1Shape`) resolve to the identical texture, which is
/// why matching by mesh name generalised badly (each ship names its extra
/// copies of this light differently, or not at all) while matching by the
/// texture it actually paints generalises to all of them.
///
/// The texture's rows turned out to be the animation itself - see
/// `docs/formats/vex.md`, "The animation is authored in the texture, on its V
/// axis", for the full survey and the capture that confirmed it
/// (`crates/render/tests/blink_lights_ground_truth.rs` checks the texture
/// match against every real ship).
///
/// Matching on the texture rather than the mesh name also means a mesh with
/// more than one material - Feisar's `self_illuminatedShape` has one batch on
/// this texture and another on the ship's own steady-lit skin - is judged
/// batch by batch instead of being wrongly all-or-nothing.
#[must_use]
pub fn is_blink_light_texture(label: &str) -> bool {
    label.to_ascii_lowercase().contains("flashing_glow")
}

#[cfg(test)]
mod blink_texture_tests {
    use super::is_blink_light_texture;

    #[test]
    fn matches_the_shared_blink_texture_however_it_is_cased() {
        assert!(is_blink_light_texture("colours_flashing_GLOW.tga"));
        assert!(is_blink_light_texture("COLOURS_FLASHING_GLOW.TGA"));
    }

    #[test]
    fn excludes_unrelated_textures() {
        assert!(!is_blink_light_texture("engine_general.tga"));
        assert!(!is_blink_light_texture("texture1.tga"));
    }
}

/// A texture decoded from the model.
#[derive(Debug)]
pub struct ModelTexture {
    pub label: String,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Whether [`build_with_textures`] draws every child of an authored
/// `LodGroup` (class `0x2ee`), or only the first.
///
/// **This is not a quality tier and it does not switch by distance.** The
/// original PSP binary never does either: it registers the class but never
/// reads its `child_count` or switch-distance fields, and its generic tree
/// walker draws every child of every node unconditionally, always, with no
/// live re-evaluation against the camera - see `docs/formats/vex.md`,
/// "`LodGroup`: authored, but never switched at runtime". Ten of the eleven
/// `LodGroup` instances on `16_Track` carry two children with real,
/// differently-detailed mesh geometry in both, so [`Self::Both`] (matching
/// the original) means genuinely overlapping duplicate geometry, not a
/// "higher quality" picture. [`Self::Single`] is a one-time choice made when
/// the model is built, not a live switch: it keeps the higher-detail tier and
/// permanently discards the other, removing that duplication at the cost of
/// no longer matching the original.
///
/// A real distance-based switch (the original's authored switch-distance
/// value, re-checked against the camera every frame) would need `Model` to
/// carry per-node bounds and the render loop to re-evaluate them each frame -
/// the same live-camera mechanism the roadmap's unimplemented frustum-culling
/// entry needs, and not yet built.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lod {
    /// Draw every child of every `LodGroup`, exactly as the original does -
    /// including the duplicate geometry that results.
    #[default]
    Both,
    /// Draw only the first child of a two-child `LodGroup` - the
    /// higher-triangle-count tier in all ten measured cases - and skip the
    /// rest. A permanent, load-time choice, not a live switch. Removes real
    /// duplicate geometry the original always draws twice, at the cost of no
    /// longer matching it.
    Single,
}

impl Lod {
    /// The spelling used in a settings file and on a menu row.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Both => "both",
            Self::Single => "single",
        }
    }

    /// Every mode, for the menus and for error messages.
    pub const ALL: [Self; 2] = [Self::Both, Self::Single];
}

impl std::str::FromStr for Lod {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|lod| lod.name().eq_ignore_ascii_case(text))
            .ok_or_else(|| format!("{text:?} is not a level-of-detail mode; try both or single"))
    }
}

impl std::fmt::Display for Lod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// A model flattened into one vertex and one index buffer.
#[derive(Debug)]
pub struct Model {
    /// Human-readable source, for the window title.
    pub label: String,
    pub vertices: Vec<GpuVertex>,
    pub indices: Vec<u32>,
    /// One per material run, in draw order. Opaque; drawn with depth write on
    /// and no blending. Neither `is_transparent()` nor `is_alpha_tested()`.
    pub draws: Vec<DrawCall>,
    /// Batches tagged `is_alpha_tested()` (see [`build_with_textures`]),
    /// indexing the same `vertices`/`indices` as [`Self::draws`]. Meant to be
    /// drawn with a cutout (`discard` below a threshold) rather than a
    /// hardcoded alpha of 1.0, but otherwise opaque - depth write stays on, so
    /// overlapping cutout surfaces still occlude each other and the geometry
    /// behind them correctly.
    pub alpha_tested_draws: Vec<DrawCall>,
    /// Batches tagged `is_transparent()` (see [`build_with_textures`]),
    /// indexing the same `vertices`/`indices` as [`Self::draws`]. Meant to be
    /// drawn after every other list, blended and with depth write off.
    pub transparent_draws: Vec<DrawCall>,
    /// Textures embedded in the model, one slot per `Texture` node.
    ///
    /// `None` where the node exists but this build cannot decode it. The slots
    /// are positional because materials name a texture by its ordinal, so
    /// compacting them would re-skin the model.
    pub textures: Vec<Option<ModelTexture>>,
    /// Centre of the bounding box, so the camera can frame the model.
    pub centre: [f32; 3],
    /// Radius of the bounding sphere.
    pub radius: f32,
    /// How many meshes contributed.
    pub mesh_count: usize,
}

/// Reads one named blob out of an archive inside a disc image.
///
/// `spec` is `<image>:<path-on-disc>`, matching `oag-wad`.
pub fn read_blob(spec: &str, name: &str) -> Result<Vec<u8>> {
    let mut archive = Archive::open(spec)?;
    archive
        .read_name(name)
        .with_context(|| format!("reading {name} from {}", archive.label()))
}

/// Reads one `.vex` entry out of an archive inside a disc image.
pub fn load(spec: &str, name: &str) -> Result<Model> {
    let data = read_blob(spec, name)?;
    build(name, &data)
}

/// Flattens every mesh in a `.vex` into one buffer pair.
///
/// Draws every child of an authored `LodGroup`, matching the original's own
/// behaviour - see [`build_with_textures`] to choose [`Lod::Single`] instead.
pub fn build(label: &str, data: &[u8]) -> Result<Model> {
    build_with_textures(label, data, None, Lod::Both)
}

/// As [`build`], with an external texture set replacing the embedded one, and
/// a choice of [`Lod`] behaviour.
///
/// PS2 models need the texture set: their embedded texture block is empty by
/// design, so there is nothing for a material to resolve to unless the set is
/// supplied from outside. Passing `None` uses whatever the file embeds, which
/// is what every PSP model wants.
pub fn build_with_textures(
    label: &str,
    data: &[u8],
    external: Option<Vec<Option<ModelTexture>>>,
    lod: Lod,
) -> Result<Model> {
    if !vex::has_magic(data) {
        bail!("{label} is not a .vex file (no VEXX magic)");
    }

    let nodes = vex::nodes(data).context("walking the node tree")?;
    let slots = nodes
        .iter()
        .filter(|n| n.class_id == vex::CLASS_TEXTURE)
        .count();

    // `Lod::Single` skips every node under a two-child `LodGroup`'s second
    // child - see `Lod`'s own doc comment for why this is an invented
    // divergence rather than a recovered one. Built as a mark-and-skip set
    // over the whole tree up front, the same shape a mesh's own descendants
    // would need if this ever grows past direct children.
    let skip: std::collections::HashSet<usize> = if lod == Lod::Single {
        let mut children: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
        for (i, node) in nodes.iter().enumerate() {
            if let Some(p) = node.parent {
                children[p].push(i);
            }
        }
        fn mark(children: &[Vec<usize>], root: usize, out: &mut std::collections::HashSet<usize>) {
            out.insert(root);
            for &child in &children[root] {
                mark(children, child, out);
            }
        }
        let mut skip = std::collections::HashSet::new();
        for (i, node) in nodes.iter().enumerate() {
            if node.class_id != vex::CLASS_LOD_GROUP {
                continue;
            }
            let payload = &data[node.payload()];
            let Some(child_count) = payload.get(0x50..0x54) else {
                continue;
            };
            if u32::from_le_bytes(child_count.try_into().expect("checked len 4")) == 2
                && let Some(&second) = children[i].get(1)
            {
                mark(&children, second, &mut skip);
            }
        }
        skip
    } else {
        std::collections::HashSet::new()
    };

    // Positional: materials name a texture by its ordinal among the `Texture`
    // nodes, so an entry this build cannot decode has to stay in place as `None`
    // rather than shift every later index.
    let embedded: Vec<Option<ModelTexture>> = vex::textures(data)
        .context("extracting textures")?
        .into_iter()
        .map(|slot| {
            slot.map(|t| ModelTexture {
                label: t
                    .name
                    .as_deref()
                    .and_then(|n| n.rsplit(['/', '\\']).next())
                    .unwrap_or("?")
                    .to_string(),
                width: u32::from(t.width),
                height: u32::from(t.height),
                rgba: t.to_rgba(),
            })
        })
        .collect();
    // A short external set leaves the tail untextured rather than misaligning
    // the ordinals a material indexes with.
    let textures = external.map_or(embedded, |mut set| {
        set.resize_with(set.len().max(slots), || None);
        set
    });

    // Mesh vertices are in the local space of whichever transform encloses them,
    // nested up to 25 deep on a track, so a model is only assembled once these
    // are composed. A ship has one transform and looks the same either way,
    // which is why this was not missed sooner.
    let world = vex::world_transforms(data, &nodes);

    let mut vertices: Vec<GpuVertex> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut draws: Vec<DrawCall> = Vec::new();
    let mut alpha_tested_draws: Vec<DrawCall> = Vec::new();
    let mut transparent_draws: Vec<DrawCall> = Vec::new();
    let mut mesh_count = 0;

    for (index, node) in nodes
        .iter()
        .enumerate()
        .filter(|(i, n)| n.class_id == vex::CLASS_MESH && !skip.contains(i))
    {
        let payload = &data[node.payload()];
        let to_world = world[index];
        let mut contributed = false;

        // A batch belongs to list A while `pass_mask & 1` is set, and to list B
        // while `pass_mask & 2` is set - one bit split of one contiguous batch
        // array, not two independent passes over the same geometry (see
        // `docs/formats/vex.md`, "Geometry is pre-batched GE display lists").
        // `vex::mesh_batches` already stops at the first batch tagged for the
        // other list, so reading both here never draws a shared batch twice;
        // a mesh made up entirely of list-B batches has nothing in list A at
        // all, so skipping either list would drop that mesh's geometry
        // completely rather than avoid a duplicate.
        //
        // Which list a batch is *in* is not the same question as which pipeline
        // it needs: on `16_Track`'s PSP build every blended batch happens to
        // live in list B alone, but the PS2 build tags its tree billboards
        // `is_alpha_tested()` while leaving them in list A. Destination is
        // decided from the batch's own attributes, not from which list produced
        // it - `is_transparent()` and `is_alpha_tested()` were never observed
        // set together on the same batch, but transparent takes priority if
        // they ever are, since blending is the more permissive of the two.
        let materials = vex::mesh_materials(payload);

        for batch_list in [0u8, 1u8] {
            for batch in vex::mesh_batches(payload, batch_list).context("decoding batches")? {
                let out: &mut Vec<DrawCall> = if batch.is_transparent() {
                    &mut transparent_draws
                } else if batch.is_alpha_tested() {
                    &mut alpha_tested_draws
                } else {
                    &mut draws
                };
                let base = vertices.len() as u32;
                let first_index = indices.len() as u32;

                // material index -> texture ordinal -> a texture we decoded. Any
                // link in that chain can be missing, and a missing one draws
                // untextured rather than borrowing a neighbour's skin.
                let texture = materials
                    .get(usize::from(batch.material_index))
                    .copied()
                    .flatten()
                    .map(|t| t as usize)
                    .filter(|&t| textures.get(t).is_some_and(Option::is_some));
                let glow = texture
                    .and_then(|t| textures.get(t))
                    .and_then(Option::as_ref)
                    .is_some_and(|t| is_blink_light_texture(&t.label));

                for v in &batch.vertices {
                    vertices.push(GpuVertex {
                        position: vex::transform_point(&to_world, v.position),
                        // A batch without normals is prelit, so face it at the
                        // camera rather than leaving it black.
                        normal: v.normal.unwrap_or([0.0, 0.0, 1.0]),
                        colour: v.colour.map_or([0.75, 0.78, 0.82, 1.0], |c| {
                            [
                                f32::from(c[0]) / 255.0,
                                f32::from(c[1]) / 255.0,
                                f32::from(c[2]) / 255.0,
                                f32::from(c[3]) / 255.0,
                            ]
                        }),
                        texcoord: v.texcoord.unwrap_or([0.0, 0.0]),
                        lit: if v.colour.is_some() { 0.0 } else { 1.0 },
                        glow: f32::from(glow),
                    });
                }
                for tri in batch.triangles() {
                    indices.extend([base + tri[0], base + tri[1], base + tri[2]]);
                    contributed = true;
                }

                let last_index = indices.len() as u32;
                if last_index > first_index {
                    out.push(DrawCall {
                        range: first_index..last_index,
                        texture,
                    });
                }
            }
        }
        if contributed {
            mesh_count += 1;
        }
    }

    if indices.is_empty() {
        bail!("{label} decoded to no triangles");
    }

    let mut lo = [f32::MAX; 3];
    let mut hi = [f32::MIN; 3];
    for v in &vertices {
        for i in 0..3 {
            lo[i] = lo[i].min(v.position[i]);
            hi[i] = hi[i].max(v.position[i]);
        }
    }
    let centre = [
        (lo[0] + hi[0]) * 0.5,
        (lo[1] + hi[1]) * 0.5,
        (lo[2] + hi[2]) * 0.5,
    ];
    let radius = (0..3)
        .map(|i| (hi[i] - lo[i]) * 0.5)
        .fold(0.0f32, f32::max)
        .max(0.001);

    Ok(Model {
        label: label.to_string(),
        vertices,
        indices,
        draws,
        alpha_tested_draws,
        transparent_draws,
        textures,
        centre,
        radius,
        mesh_count,
    })
}

/// Decodes a PS2 texture set into slots a model can be re-skinned with.
///
/// PS2 `.vex` files declare a texture block of zero length: their textures are
/// separate archive entries, and a model's set is gathered into a **nested WAD**
/// of its own, one entry per `Texture` node and in the same order. Each entry is
/// a Graphics Synthesizer upload packet, see
/// [`oag_formats::ps2_texture`].
///
/// Slots are positional for the same reason [`build`] keeps them positional: a
/// material names a texture by its ordinal, so an entry this build cannot decode
/// stays `None` in place rather than shifting every later one.
pub fn ps2_texture_set(blob: &[u8]) -> Result<Vec<Option<ModelTexture>>> {
    let count = wad::Directory::peek_entry_count(blob).context("not a nested WAD")?;
    let directory = wad::Directory::parse(blob, Some(blob.len() as u64))
        .map_err(|e| anyhow::anyhow!("{e}"))
        .context("parsing the texture set directory")?;

    let mut out = Vec::with_capacity(count as usize);
    for (index, entry) in directory.entries.iter().enumerate() {
        let start = entry.offset as usize;
        let end = start + entry.size as usize;
        let Some(stored) = blob.get(start..end) else {
            out.push(None);
            continue;
        };
        let decompressed = match entry.compression {
            wad::Compression::None => stored.to_vec(),
            wad::Compression::Lzss => {
                match oag_formats::lzss::decompress(stored, entry.size_uncompressed as usize) {
                    Ok(bytes) => bytes,
                    Err(_) => {
                        out.push(None);
                        continue;
                    }
                }
            }
            wad::Compression::Zlib => {
                out.push(None);
                continue;
            }
        };
        out.push(
            ps2_texture::parse(&decompressed)
                .ok()
                .map(|texture| ModelTexture {
                    label: format!("#{index} {:08x}", entry.name_hash),
                    width: u32::from(texture.width),
                    height: u32::from(texture.height),
                    rgba: texture.to_rgba(),
                }),
        );
    }
    Ok(out)
}

/// Concatenates several models into one buffer pair.
///
/// Used to draw more than one thing at a time without the pipeline learning
/// about scenes: `oag-view --collision --with-spline` overlays the collision
/// soup on the driveable ribbon so the two can be compared in place.
///
/// Index and texture-slot references are rebased, since both are positional.
/// Empty models are skipped rather than contributing an empty draw call.
///
/// This is a *concatenation*, not a scene: the models share one depth buffer and
/// one opaque pipeline, so a solid model will occlude anything inside it. That is
/// why the collision view draws outlines by default; see
/// [`crate::collision`].
#[must_use]
pub fn merge(label: &str, models: Vec<Model>) -> Model {
    let mut out = Model {
        label: label.to_string(),
        vertices: Vec::new(),
        indices: Vec::new(),
        draws: Vec::new(),
        alpha_tested_draws: Vec::new(),
        transparent_draws: Vec::new(),
        textures: Vec::new(),
        centre: [0.0; 3],
        radius: 1.0,
        mesh_count: 0,
    };

    for model in models {
        if model.vertices.is_empty() || model.indices.is_empty() {
            continue;
        }
        let vertex_base = out.vertices.len() as u32;
        let index_base = out.indices.len() as u32;
        let texture_base = out.textures.len();

        out.vertices.extend(model.vertices);
        out.indices
            .extend(model.indices.iter().map(|i| i + vertex_base));
        let rebase = |d: DrawCall| DrawCall {
            range: (d.range.start + index_base)..(d.range.end + index_base),
            texture: d.texture.map(|t| t + texture_base),
        };
        out.draws.extend(model.draws.into_iter().map(rebase));
        out.alpha_tested_draws
            .extend(model.alpha_tested_draws.into_iter().map(rebase));
        out.transparent_draws
            .extend(model.transparent_draws.into_iter().map(rebase));
        out.textures.extend(model.textures);
        out.mesh_count += model.mesh_count;
    }

    let mut lo = [f32::MAX; 3];
    let mut hi = [f32::MIN; 3];
    for v in &out.vertices {
        for i in 0..3 {
            lo[i] = lo[i].min(v.position[i]);
            hi[i] = hi[i].max(v.position[i]);
        }
    }
    if !out.vertices.is_empty() {
        out.centre = [
            (lo[0] + hi[0]) * 0.5,
            (lo[1] + hi[1]) * 0.5,
            (lo[2] + hi[2]) * 0.5,
        ];
        out.radius = (0..3)
            .map(|i| (hi[i] - lo[i]) * 0.5)
            .fold(0.0f32, f32::max)
            .max(0.001);
    }
    out
}

#[cfg(test)]
mod merge_tests {
    use super::*;

    fn model(label: &str, vertices: usize, textures: usize) -> Model {
        Model {
            label: label.to_string(),
            vertices: (0..vertices)
                .map(|k| GpuVertex {
                    position: [k as f32, 0.0, 0.0],
                    normal: [0.0, 1.0, 0.0],
                    colour: [1.0; 4],
                    texcoord: [0.0; 2],
                    lit: 1.0,
                    glow: 0.0,
                })
                .collect(),
            indices: (0..vertices as u32).collect(),
            draws: vec![DrawCall {
                range: 0..vertices as u32,
                texture: (textures > 0).then_some(0),
            }],
            alpha_tested_draws: Vec::new(),
            transparent_draws: Vec::new(),
            textures: (0..textures).map(|_| None).collect(),
            centre: [0.0; 3],
            radius: 1.0,
            mesh_count: 1,
        }
    }

    /// Indices are positions in a shared buffer, so the second model's have to
    /// be rebased or it draws the first model's geometry twice.
    #[test]
    fn indices_are_rebased_onto_the_combined_buffer() {
        let out = merge("both", vec![model("a", 3, 0), model("b", 3, 0)]);
        assert_eq!(out.vertices.len(), 6);
        assert_eq!(out.indices, vec![0, 1, 2, 3, 4, 5]);
        assert_eq!(out.draws[1].range, 3..6);
    }

    /// A material names a texture by its ordinal, so the slot index has to move
    /// with the slots or the second model draws the first model's skin.
    #[test]
    fn texture_slots_are_rebased_too() {
        let out = merge("both", vec![model("a", 3, 2), model("b", 3, 1)]);
        assert_eq!(out.textures.len(), 3);
        assert_eq!(out.draws[1].texture, Some(2));
    }

    #[test]
    fn empty_models_contribute_nothing() {
        let out = merge("one", vec![model("a", 3, 0), model("empty", 0, 0)]);
        assert_eq!(out.draws.len(), 1);
        assert_eq!(out.vertices.len(), 3);
    }

    #[test]
    fn merging_nothing_is_still_a_usable_model() {
        let out = merge("none", Vec::new());
        assert!(out.vertices.is_empty());
        assert!(out.radius > 0.0);
    }
}
