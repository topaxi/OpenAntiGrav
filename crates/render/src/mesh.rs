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
}

/// A run of indices sharing one texture.
#[derive(Debug)]
pub struct DrawCall {
    pub range: std::ops::Range<u32>,
    /// Index into [`Model::textures`], or `None` for untextured.
    pub texture: Option<usize>,
}

/// A texture decoded from the model.
#[derive(Debug)]
pub struct ModelTexture {
    pub label: String,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// A model flattened into one vertex and one index buffer.
#[derive(Debug)]
pub struct Model {
    /// Human-readable source, for the window title.
    pub label: String,
    pub vertices: Vec<GpuVertex>,
    pub indices: Vec<u32>,
    /// One per material run, in draw order.
    pub draws: Vec<DrawCall>,
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
pub fn build(label: &str, data: &[u8]) -> Result<Model> {
    build_with_textures(label, data, None)
}

/// As [`build`], with an external texture set replacing the embedded one.
///
/// PS2 models need this: their embedded texture block is empty by design, so
/// there is nothing for a material to resolve to unless the set is supplied
/// from outside. Passing `None` uses whatever the file embeds, which is what
/// every PSP model wants.
pub fn build_with_textures(
    label: &str,
    data: &[u8],
    external: Option<Vec<Option<ModelTexture>>>,
) -> Result<Model> {
    if !vex::has_magic(data) {
        bail!("{label} is not a .vex file (no VEXX magic)");
    }

    let nodes = vex::nodes(data).context("walking the node tree")?;
    let slots = nodes
        .iter()
        .filter(|n| n.class_id == vex::CLASS_TEXTURE)
        .count();

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
    let mut mesh_count = 0;

    for (index, node) in nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.class_id == vex::CLASS_MESH)
    {
        let payload = &data[node.payload()];
        let to_world = world[index];
        let mut contributed = false;

        // List A is the ordinary render list; B is a second pass. Taking only A
        // avoids drawing the same surface twice.
        let materials = vex::mesh_materials(payload);

        for batch in vex::mesh_batches(payload, 0).context("decoding batches")? {
            let base = vertices.len() as u32;
            let first_index = indices.len() as u32;
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
                });
            }
            for tri in batch.triangles() {
                indices.extend([base + tri[0], base + tri[1], base + tri[2]]);
                contributed = true;
            }

            let last_index = indices.len() as u32;
            if last_index > first_index {
                // material index -> texture ordinal -> a texture we decoded.
                // Any link in that chain can be missing, and a missing one draws
                // untextured rather than borrowing a neighbour's skin.
                let texture = materials
                    .get(usize::from(batch.material_index))
                    .copied()
                    .flatten()
                    .map(|t| t as usize)
                    .filter(|&t| textures.get(t).is_some_and(Option::is_some));
                draws.push(DrawCall {
                    range: first_index..last_index,
                    texture,
                });
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
