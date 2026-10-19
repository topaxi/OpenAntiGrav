//! Bottom and top level acceleration structures over the buffers the raster draws.
//!
//! A [`ModelAccel`] is the BLASes of one [`Model`]: its static geometry as one, and each
//! `Anim Transform` node's geometry as one of its own (the nodes are rigid, so a moving
//! node costs a TLAS instance per frame, not a rebuild). A [`Scene`] is the TLAS over a
//! circuit's [`ModelAccel`] and the craft hulls.
//!
//! **What the structures hold.** The opaque and alpha-tested draws, all flagged opaque:
//! wgpu's naga has no candidate intersections yet, so a non-opaque BLAS geometry returns
//! no hit at all. A cutout fence therefore casts and receives as a solid, the gap
//! `docs/rendering/ray-tracing.md` names. Blended draws (glass, exhaust, sprites) are left
//! out: a ray should not stop on a plume.
//!
//! **The animated nodes.** `GpuVertex::xform` is a per-vertex attribute, but every vertex
//! of a draw carries the same one (`Model::moving` says which draws), so a node's draws
//! partition cleanly by the first vertex of their index range. The vertices are baked in
//! the node's own space, exactly as the vertex shader receives them, and the instance
//! carries `Model::sample_anim_nodes`' matrix for it.

use std::time::{Duration, Instant};

use wgpu::util::DeviceExt as _;

use crate::mesh::{DrawCall, GpuVertex, Model};

/// `TlasInstance::custom_data` of the circuit's static geometry.
pub const KIND_STATIC: u32 = 0;
/// `TlasInstance::custom_data` of a craft hull.
pub const KIND_HULL: u32 = 1;
/// `TlasInstance::custom_data` of a moving scenery node.
pub const KIND_NODE: u32 = 2;

/// What one model's structures hold, for the loader report.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stats {
    /// Triangles in all the model's BLASes.
    pub triangles: u32,
    /// Triangles of those that belong to an animated node.
    pub moving_triangles: u32,
    /// Distinct BLASes: the static one, if any, plus one per animated node.
    pub blases: u32,
    /// Of those, the ones under an animated node.
    pub moving_blases: u32,
    /// Triangles the model has that are left out: blended draws.
    pub skipped_triangles: u32,
    /// Wall clock of the builds, submitted and waited on.
    pub build: Duration,
    /// Device memory the build added, from the allocator report: what was allocated, and
    /// what the allocator reserved to hold it. `None` where the backend reports none.
    /// Includes the index buffers the BLASes were built from.
    pub bytes: Option<(u64, u64)>,
}

impl std::fmt::Display for Stats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} triangles in {} BLAS ({} on {} moving nodes, {} blended left out), built in {:.1} ms",
            self.triangles,
            self.blases,
            self.moving_triangles,
            self.moving_blases,
            self.skipped_triangles,
            self.build.as_secs_f64() * 1000.0
        )?;
        if let Some((allocated, reserved)) = self.bytes {
            const MIB: f64 = 1024.0 * 1024.0;
            write!(
                f,
                ", {:.1} MiB allocated ({:.1} MiB reserved)",
                allocated as f64 / MIB,
                reserved as f64 / MIB
            )?;
        }
        Ok(())
    }
}

/// One BLAS and the index buffer it was built from, kept alive with it.
#[derive(Debug)]
struct Part {
    blas: wgpu::Blas,
    /// The `xform` slot of the node it moves under, `0` for the static part.
    xform: u32,
    _indices: wgpu::Buffer,
}

/// The BLASes of one model.
#[derive(Debug)]
pub struct ModelAccel {
    parts: Vec<Part>,
    /// The model's nodes at time zero (`Model::sample_anim_nodes`): where a hull's parts
    /// sit before anything moves them. A circuit is updated with live ones instead.
    rest: Vec<[f32; 16]>,
    pub stats: Stats,
}

/// Row-major 3x4 from a 4x4 whose axes sit in its first three rows and translation in the
/// fourth (`vex::multiply`'s convention, and glam's column-major array, which is the same
/// bytes). `TlasInstance::transform` is the column-vector form, so element `(r, c)` is
/// `m[c * 4 + r]`.
#[must_use]
pub fn affine(m: &[f32; 16]) -> [f32; 12] {
    let mut out = [0.0; 12];
    for r in 0..3 {
        for c in 0..4 {
            out[r * 4 + c] = m[c * 4 + r];
        }
    }
    out
}

/// [`affine`] of the identity.
pub const IDENTITY: [f32; 12] = [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0];

fn allocated(device: &wgpu::Device) -> Option<(u64, u64)> {
    device
        .generate_allocator_report()
        .map(|r| (r.total_allocated_bytes, r.total_reserved_bytes))
}

fn xform_of(model: &Model, draw: &DrawCall) -> u32 {
    model
        .indices
        .get(draw.range.start as usize)
        .and_then(|&i| model.vertices.get(i as usize))
        .map_or(0, |v| v.xform)
}

impl ModelAccel {
    /// Builds the model's BLASes from `vertex_buffer`, which must be the buffer holding
    /// `model.vertices` and have been created with `BLAS_INPUT` (see `super::blas_input`).
    /// `None` for a model with nothing opaque in it.
    ///
    /// Submits and waits for the build, so the stats carry a real time: this runs once per
    /// load, never in a frame.
    #[must_use]
    pub fn build(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        label: &str,
        model: &Model,
        vertex_buffer: &wgpu::Buffer,
    ) -> Option<Self> {
        let before = allocated(device);
        let started = Instant::now();

        let mut by_node: std::collections::BTreeMap<u32, Vec<u32>> = Default::default();
        for draw in model.draws.iter().chain(&model.alpha_tested_draws) {
            let (start, end) = (draw.range.start as usize, draw.range.end as usize);
            let Some(run) = model.indices.get(start..end) else {
                continue;
            };
            by_node
                .entry(xform_of(model, draw))
                .or_default()
                .extend_from_slice(run);
        }
        let skipped = model
            .transparent_draws
            .iter()
            .map(|d| d.range.len() as u32 / 3)
            .sum();

        let vertex_count = u32::try_from(model.vertices.len()).ok()?;
        let mut parts = Vec::new();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("ray build"),
        });
        let mut stats = Stats {
            skipped_triangles: skipped,
            ..Stats::default()
        };
        // The entries borrow the buffers and BLASes, so they are collected first and built
        // in one call.
        let mut staged = Vec::new();
        for (xform, indices) in &by_node {
            let triangles = indices.len() as u32 / 3;
            if triangles == 0 {
                continue;
            }
            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("ray indices"),
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::BLAS_INPUT,
            });
            let size = wgpu::BlasTriangleGeometrySizeDescriptor {
                vertex_format: wgpu::VertexFormat::Float32x3,
                vertex_count,
                index_format: Some(wgpu::IndexFormat::Uint32),
                index_count: Some(indices.len() as u32),
                flags: wgpu::AccelerationStructureGeometryFlags::OPAQUE,
            };
            let blas = device.create_blas(
                &wgpu::CreateBlasDescriptor {
                    label: Some(label),
                    flags: wgpu::AccelerationStructureFlags::PREFER_FAST_TRACE,
                    update_mode: wgpu::AccelerationStructureUpdateMode::Build,
                },
                wgpu::BlasGeometrySizeDescriptors::Triangles {
                    descriptors: vec![size.clone()],
                },
            );
            stats.triangles += triangles;
            if *xform != 0 {
                stats.moving_triangles += triangles;
                stats.moving_blases += 1;
            }
            stats.blases += 1;
            staged.push((size, blas, index_buffer, *xform));
        }
        if staged.is_empty() {
            return None;
        }
        let entries: Vec<_> = staged
            .iter()
            .map(|(size, blas, index_buffer, _)| wgpu::BlasBuildEntry {
                blas,
                geometry: wgpu::BlasGeometries::TriangleGeometries(vec![
                    wgpu::BlasTriangleGeometry {
                        size,
                        vertex_buffer,
                        first_vertex: 0,
                        vertex_stride: std::mem::size_of::<GpuVertex>() as u64,
                        index_buffer: Some(index_buffer),
                        first_index: Some(0),
                        transform_buffer: None,
                        transform_buffer_offset: None,
                    },
                ]),
            })
            .collect();
        encoder.build_acceleration_structures(entries.iter(), std::iter::empty());
        drop(entries);
        queue.submit([encoder.finish()]);
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
        for (_, blas, indices, xform) in staged {
            parts.push(Part {
                blas,
                xform,
                _indices: indices,
            });
        }
        stats.build = started.elapsed();
        stats.bytes = before.zip(allocated(device)).map(|((a0, r0), (a1, r1))| {
            (a1.saturating_sub(a0), r1.saturating_sub(r0))
        });
        log::info!("ray: {label}: {stats}");
        Some(Self {
            parts,
            rest: model.sample_anim_nodes(0.0),
            stats,
        })
    }
}

/// Craft hulls: the BLASes of each distinct hull model.
pub type Hulls = Vec<ModelAccel>;

/// The TLAS over a circuit and its craft.
#[derive(Debug)]
pub struct Scene {
    tlas: wgpu::Tlas,
    track: ModelAccel,
    hulls: Hulls,
    /// Where the instance list ends for the circuit's own parts; craft follow.
    craft_capacity: u32,
    /// Instances in the last build.
    live: u32,
}

impl Scene {
    /// A TLAS able to hold the circuit's parts and `craft_capacity` craft.
    #[must_use]
    pub fn new(device: &wgpu::Device, track: ModelAccel, hulls: Hulls, craft_capacity: u32) -> Self {
        let max_instances = track.parts.len() as u32
            + craft_capacity * hulls.iter().map(|h| h.parts.len() as u32).max().unwrap_or(1);
        let tlas = device.create_tlas(&wgpu::CreateTlasDescriptor {
            label: Some("ray scene"),
            max_instances,
            flags: wgpu::AccelerationStructureFlags::PREFER_FAST_BUILD,
            update_mode: wgpu::AccelerationStructureUpdateMode::Build,
        });
        Self {
            tlas,
            track,
            hulls,
            craft_capacity,
            live: 0,
        }
    }

    /// The circuit's stats, for the loader report.
    #[must_use]
    pub fn track_stats(&self) -> Stats {
        self.track.stats
    }

    /// Rebuilds the TLAS: the circuit's static part at the identity, each moving node at
    /// `nodes[xform - 1]` (`Model::sample_anim_nodes`), and each `(hull, model)` craft at
    /// its model matrix - the matrices the raster draws with, so a traced result and the
    /// picture cannot disagree. Records into `encoder`; nothing is submitted.
    pub fn update(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        nodes: &[[f32; 16]],
        craft: &[(usize, [f32; 16])],
    ) {
        let mut next = 0usize;
        for part in &self.track.parts {
            let (transform, kind) = if part.xform == 0 {
                (IDENTITY, KIND_STATIC)
            } else {
                match nodes.get(part.xform as usize - 1) {
                    Some(m) => (affine(m), KIND_NODE),
                    None => continue,
                }
            };
            self.tlas[next] = Some(wgpu::TlasInstance::new(&part.blas, transform, kind, 0xFF));
            next += 1;
        }
        for (hull, model) in craft.iter().take(self.craft_capacity as usize) {
            let Some(accel) = self.hulls.get(*hull) else {
                continue;
            };
            for part in &accel.parts {
                // A part under a hull node is baked in that node's space: node first, then
                // the craft (the row-vector product `Model::sample_anim_nodes` also uses).
                let placed = match part.xform {
                    0 => *model,
                    x => match accel.rest.get(x as usize - 1) {
                        Some(node) => oag_vex::vex::multiply(node, model),
                        None => continue,
                    },
                };
                self.tlas[next] = Some(wgpu::TlasInstance::new(
                    &part.blas,
                    affine(&placed),
                    KIND_HULL,
                    0xFF,
                ));
                next += 1;
            }
        }
        for slot in next..self.live as usize {
            self.tlas[slot] = None;
        }
        self.live = next as u32;
        encoder.build_acceleration_structures(std::iter::empty(), std::iter::once(&self.tlas));
    }

    /// The TLAS, for a bind group.
    #[must_use]
    pub fn tlas(&self) -> &wgpu::Tlas {
        &self.tlas
    }

    /// Instances in the last [`Self::update`].
    #[must_use]
    pub fn live(&self) -> u32 {
        self.live
    }
}
