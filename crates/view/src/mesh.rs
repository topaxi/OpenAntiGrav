//! Loads a `.vex` model out of an archive and flattens it for the GPU.

use anyhow::{Context, Result, bail};
use oag_disc::DiscImage;
use oag_formats::lzss;
use oag_formats::vex;
use oag_formats::wad::{self, Compression, Directory};

/// A vertex as the mesh shader expects it.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub colour: [f32; 4],
    pub texcoord: [f32; 2],
}

/// A run of indices sharing one texture.
pub struct DrawCall {
    pub range: std::ops::Range<u32>,
    /// Index into [`Model::textures`], or `None` for untextured.
    pub texture: Option<usize>,
}

/// A texture decoded from the model.
pub struct ModelTexture {
    pub label: String,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// A model flattened into one vertex and one index buffer.
pub struct Model {
    /// Human-readable source, for the window title.
    pub label: String,
    pub vertices: Vec<GpuVertex>,
    pub indices: Vec<u32>,
    /// One per material run, in draw order.
    pub draws: Vec<DrawCall>,
    /// Textures embedded in the model.
    pub textures: Vec<ModelTexture>,
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
    let (image, inner) = spec
        .rsplit_once(':')
        .filter(|(image, inner)| image.len() >= 2 && !inner.is_empty())
        .context("expected <image>:<path-on-disc>")?;

    let mut disc = DiscImage::open(image).with_context(|| format!("opening {image}"))?;
    let entry = disc
        .entries()?
        .iter()
        .find(|e| !e.is_directory && e.path.eq_ignore_ascii_case(inner))
        .cloned()
        .with_context(|| format!("{inner} is not on {image}"))?;

    let header = disc.read_entry_range(&entry, 0, wad::HEADER_LEN as u64)?;
    let count =
        Directory::peek_entry_count(&header).map_err(|e| anyhow::anyhow!("{inner}: {e}"))?;
    let dir_bytes = disc.read_entry_range(&entry, 0, Directory::directory_len(count))?;
    let dir = Directory::parse(&dir_bytes, Some(entry.size))
        .map_err(|e| anyhow::anyhow!("{inner}: {e}"))?;

    let hash = wad::hash_name(name);
    let wad_entry = dir
        .entries
        .iter()
        .find(|e| e.name_hash == hash)
        .with_context(|| format!("{name} (hash {hash:08x}) is not in {inner}"))?;

    let raw = disc.read_entry_range(
        &entry,
        u64::from(wad_entry.offset),
        u64::from(wad_entry.size),
    )?;
    match wad_entry.compression {
        Compression::None => Ok(raw),
        Compression::Lzss => {
            lzss::decompress(&raw, wad_entry.size_uncompressed as usize).context("decompressing")
        }
        Compression::Zlib => bail!("zlib entries are not supported"),
    }
}

/// Reads one `.vex` entry out of an archive inside a disc image.
pub fn load(spec: &str, name: &str) -> Result<Model> {
    let data = read_blob(spec, name)?;
    build(name, &data)
}

/// Flattens every mesh in a `.vex` into one buffer pair.
pub fn build(label: &str, data: &[u8]) -> Result<Model> {
    if !vex::has_magic(data) {
        bail!("{label} is not a .vex file (no VEXX magic)");
    }

    let nodes = vex::nodes(data).context("walking the node tree")?;

    let textures: Vec<ModelTexture> = vex::textures(data)
        .context("extracting textures")?
        .into_iter()
        .map(|t| ModelTexture {
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
        .collect();

    let mut vertices: Vec<GpuVertex> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut draws: Vec<DrawCall> = Vec::new();
    let mut mesh_count = 0;

    for node in nodes.iter().filter(|n| n.class_id == vex::CLASS_MESH) {
        let payload = &data[node.payload()];
        let mut contributed = false;

        // List A is the ordinary render list; B is a second pass. Taking only A
        // avoids drawing the same surface twice.
        let materials = vex::mesh_materials(payload);

        for batch in vex::mesh_batches(payload, 0).context("decoding batches")? {
            let base = vertices.len() as u32;
            let first_index = indices.len() as u32;
            for v in &batch.vertices {
                vertices.push(GpuVertex {
                    position: v.position,
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
                });
            }
            for tri in batch.triangles() {
                indices.extend([base + tri[0], base + tri[1], base + tri[2]]);
                contributed = true;
            }

            let last_index = indices.len() as u32;
            if last_index > first_index {
                // The material index selects an entry in this mesh's material
                // list, which in turn indexes the model's texture array.
                let texture = materials
                    .get(usize::from(batch.material_index))
                    .map(|&t| t as usize)
                    .filter(|&t| t < textures.len());
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
