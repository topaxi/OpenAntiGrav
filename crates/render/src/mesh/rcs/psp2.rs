//! Drawing Wipeout 2048's `.rcsmodel`.
//!
//! [`super`] builds Wipeout HD's container, which needs the `.vex` beside it -
//! a chunk is addressed by hash and its vertex stride comes from the node's
//! authored bounding box. **2048's container needs neither**: every submesh
//! carries its own counts and its own buffer pointers, and the stride falls out
//! of the buffer packing. So this builder takes the model blob alone.
//!
//! What it draws is positions and triangles, untextured, lit off computed face
//! normals - the same honest half-picture `super::build` gives an HD model
//! whose material binding has not been read. `oag_formats::rcsmodel::psp2`'s
//! module docs list what is not decoded yet; the ones that show here are the
//! per-vertex normals, the texture coordinates and the material binding.

use anyhow::Result;

use oag_formats::rcsmodel::psp2;

use crate::mesh::{Bounds, DrawCall, GpuVertex, Model};

/// What one build found.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Report {
    /// Submeshes drawn.
    pub submeshes: usize,
    /// Triangles emitted.
    pub triangles: usize,
    /// Relocation entries naming a GPU pointer this reading does not account
    /// for - see [`psp2::Model::unpaired_pointers`].
    pub unpaired: usize,
}

impl Report {
    /// One line for a load report.
    #[must_use]
    pub fn describe(&self) -> String {
        format!(
            "{} triangle(s) over {} submesh(es), untextured and lit off face \
             normals; {} unaccounted GPU pointer(s)",
            self.triangles, self.submeshes, self.unpaired
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

/// Builds every submesh of a 2048 `.rcsmodel` into one model.
///
/// **No transform is composed onto anything**, and that is a property of the
/// files rather than a simplification: a circuit's positions come out in world
/// coordinates and a craft's about its own origin, so both are already in the
/// space their caller draws them in. See [`psp2::Model::positions`].
///
/// # Errors
///
/// Propagates [`psp2::parse`].
pub fn build(label: &str, model_blob: &[u8]) -> Result<(Model, Report)> {
    let decoded = psp2::parse(model_blob)
        .map_err(|e| anyhow::anyhow!("{label}: the .rcsmodel beside it: {e}"))?;

    let mut model = Model::none(label);
    let mut report = Report {
        unpaired: decoded.unpaired_pointers,
        ..Report::default()
    };
    for submesh in &decoded.submeshes {
        let base = u32::try_from(model.vertices.len()).unwrap_or(u32::MAX);
        let first = u32::try_from(model.indices.len()).unwrap_or(u32::MAX);
        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        for position in &submesh.positions {
            for a in 0..3 {
                lo[a] = lo[a].min(position[a]);
                hi[a] = hi[a].max(position[a]);
            }
            model.vertices.push(GpuVertex {
                position: *position,
                // **Nothing is invented here.** The per-vertex normal is in the
                // file, packed into the four bytes at +0x0c, and unread - so
                // this leaves a zero and the shader's own face-normal path
                // lights the surface, rather than a made-up direction.
                normal: [0.0; 3],
                colour: [1.0, 1.0, 1.0, 1.0],
                texcoord: [0.0, 0.0],
                lit: 1.0,
                lightmap_texcoord: [0.0, 0.0],
                anim: 0,
                slots: 0,
                xform: 0,
                sun_mask: 1.0,
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
            layer: oag_formats::vex::LAYER_DEFAULT,
            node: None,
            chunk: None,
        });
        report.submeshes += 1;
        report.triangles += submesh.triangle_count();
    }
    model.mesh_count = report.submeshes;
    // **The authored normals are in the file and are not read** - four bytes at
    // each vertex's +0x0c, unplaced. So the shape is lit off the triangles this
    // builder already decoded, which is a fact about the geometry rather than a
    // stand-in for data nobody has. See `super::face_normals`, whose docs make
    // the same argument for the HD container.
    super::face_normals(&mut model);
    finish_bounds(&mut model);
    Ok((model, report))
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
