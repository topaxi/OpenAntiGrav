//! A model's vertex buffers on the GPU: one buffer for a model that fits the
//! device's `max_buffer_size`, several cut by [`VertexChunks`] for one that
//! does not.
//!
//! **A model that fits costs what it always did.** One buffer, bound once by
//! [`Geometry::cursor`], every draw issued with base vertex 0. Only a chunked
//! model looks a draw's chunk up, rebinds when it changes, and draws with the
//! chunk's negative base vertex.

use std::ops::Range;

use oag_gpu::writes::Writes;

use super::chunks::{Unsplittable, VERTEX_BYTES, VertexChunks};
use crate::mesh::{GpuVertex, Model};

/// Bytes of one streamed texture coordinate.
const TEXCOORD_BYTES: u64 = std::mem::size_of::<[f32; 2]>() as u64;

/// A model's vertex buffers and what it takes to draw from them.
#[derive(Debug, Clone)]
pub struct Geometry {
    chunks: VertexChunks,
    vertices: Vec<wgpu::Buffer>,
    /// The streamed coordinates, chunked like [`Self::vertices`] and bound to
    /// slot 1 - see [`super::Texcoords::Streamed`].
    texcoords: Option<Vec<wgpu::Buffer>>,
    /// `(first index of a draw, its chunk)` for every draw of the model, sorted;
    /// empty while the model is one chunk.
    draw_chunk: Vec<(u32, u32)>,
}

impl Geometry {
    /// Empty buffers for `model`, cut to fit `max_buffer_bytes`.
    ///
    /// # Errors
    ///
    /// [`Unsplittable`] when draws straddle every cut that would fit.
    pub fn new(
        device: &wgpu::Device,
        model: &Model,
        max_buffer_bytes: u64,
    ) -> Result<Self, Unsplittable> {
        let chunks = VertexChunks::plan(model, max_buffer_bytes)?;
        let vertices = chunks
            .ranges()
            .iter()
            .map(|range| {
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("vertices"),
                    size: u64::from(range.len() as u32) * VERTEX_BYTES,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                })
            })
            .collect();
        let draw_chunk = if chunks.is_whole() {
            Vec::new()
        } else {
            let mut table: Vec<(u32, u32)> = model
                .draws
                .iter()
                .chain(&model.alpha_tested_draws)
                .chain(&model.transparent_draws)
                .filter_map(|draw| {
                    let first = *model.indices.get(draw.range.start as usize)?;
                    Some((draw.range.start, chunks.chunk_of(first) as u32))
                })
                .collect();
            table.sort_unstable();
            table.dedup();
            table
        };
        Ok(Self {
            chunks,
            vertices,
            texcoords: None,
            draw_chunk,
        })
    }

    /// One caller-made buffer of `vertices` vertices as a geometry.
    #[must_use]
    pub fn single(buffer: wgpu::Buffer, vertices: u32) -> Self {
        Self {
            chunks: VertexChunks::whole(vertices),
            vertices: vec![buffer],
            texcoords: None,
            draw_chunk: Vec::new(),
        }
    }

    /// Whether one buffer holds every vertex: the bind-once path.
    #[must_use]
    pub fn is_whole(&self) -> bool {
        self.chunks.is_whole()
    }

    /// How many vertex buffers.
    #[must_use]
    pub fn buffer_count(&self) -> usize {
        self.vertices.len()
    }

    /// The vertex ranges the buffers hold.
    #[must_use]
    pub fn chunks(&self) -> &VertexChunks {
        &self.chunks
    }

    /// The sole vertex buffer of a geometry that is [`Self::is_whole`], for the
    /// callers that draw a small model without a [`Cursor`].
    ///
    /// # Panics
    ///
    /// When the geometry is cut into more than one buffer.
    #[must_use]
    pub fn sole(&self) -> &wgpu::Buffer {
        assert!(
            self.is_whole(),
            "a model cut into {} vertex buffers needs a Cursor",
            self.vertices.len()
        );
        &self.vertices[0]
    }

    /// Every vertex buffer, in chunk order.
    #[must_use]
    pub fn buffers(&self) -> &[wgpu::Buffer] {
        &self.vertices
    }

    /// Uploads `vertices` (the model's, all of them) into the buffers now.
    pub fn write_all(&self, queue: &wgpu::Queue, vertices: &[GpuVertex]) {
        self.write(queue, 0, vertices);
    }

    /// Writes `vertices` over the model's vertices from vertex `first`,
    /// splitting the write at the chunk boundaries.
    pub fn write(&self, queue: &dyn Writes, first: u32, vertices: &[GpuVertex]) {
        if self.is_whole() {
            queue.write_buffer(
                &self.vertices[0],
                u64::from(first) * VERTEX_BYTES,
                bytemuck::cast_slice(vertices),
            );
            return;
        }
        for (chunk, at, span) in self.chunks.split(first, vertices.len() as u32) {
            queue.write_buffer(
                &self.vertices[chunk],
                u64::from(at) * VERTEX_BYTES,
                bytemuck::cast_slice(&vertices[span]),
            );
        }
    }

    /// Parks the vertex upload [`Self::write_all`] would do, reading from
    /// `model` itself under an open [`oag_gpu::deferred_upload`] scope.
    pub fn defer(&self, model: &std::sync::Arc<Model>) {
        struct Vertices(std::sync::Arc<Model>);
        impl oag_gpu::deferred_upload::Bytes for Vertices {
            fn bytes(&self) -> &[u8] {
                bytemuck::cast_slice(&self.0.vertices)
            }
        }
        let source = std::sync::Arc::new(Vertices(model.clone()));
        for (buffer, range) in self.vertices.iter().zip(self.chunks.ranges()) {
            let bytes = range.start as usize * VERTEX_BYTES as usize
                ..range.end as usize * VERTEX_BYTES as usize;
            oag_gpu::deferred_upload::defer_buffer(buffer, 0, source.clone(), bytes);
        }
    }

    /// Adds the streamed coordinates of `model` as slot 1, chunked as the
    /// vertices are.
    pub fn stream_texcoords(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, model: &Model) {
        let buffers = self
            .chunks
            .ranges()
            .iter()
            .map(|range| {
                let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("streamed texcoords"),
                    size: u64::from(range.len() as u32) * TEXCOORD_BYTES,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                let coordinates: Vec<[f32; 2]> = model.vertices
                    [range.start as usize..range.end as usize]
                    .iter()
                    .map(|v| v.texcoord)
                    .collect();
                oag_gpu::deferred_upload::write_buffer(queue, &buffer, coordinates);
                buffer
            })
            .collect();
        self.texcoords = Some(buffers);
    }

    /// Writes streamed `coordinates` over the model's from vertex `first`,
    /// splitting the write at the chunk boundaries. Does nothing without
    /// [`Self::stream_texcoords`].
    pub fn write_texcoords(&self, queue: &dyn Writes, first: u32, coordinates: &[[f32; 2]]) {
        let Some(buffers) = &self.texcoords else {
            return;
        };
        for (chunk, at, span) in self.chunks.split(first, coordinates.len() as u32) {
            queue.write_buffer(
                &buffers[chunk],
                u64::from(at) * TEXCOORD_BYTES,
                bytemuck::cast_slice(&coordinates[span]),
            );
        }
    }

    /// Whether `other` draws from the very buffers this one does.
    #[must_use]
    pub fn shares_buffers_with(&self, other: &Self) -> bool {
        self.vertices == other.vertices
    }

    /// The streamed coordinates' buffers, if [`Self::stream_texcoords`] made them.
    #[must_use]
    pub fn texcoords(&self) -> Option<&[wgpu::Buffer]> {
        self.texcoords.as_deref()
    }

    /// The largest buffer this geometry holds, in bytes.
    #[must_use]
    pub fn largest_buffer(&self) -> u64 {
        self.vertices
            .iter()
            .map(wgpu::Buffer::size)
            .max()
            .unwrap_or(0)
    }

    /// Binds what the pass draws from and returns the state [`Cursor::draw`]
    /// keeps current. A whole geometry is bound here, once.
    pub fn cursor<'g>(&'g self, pass: &mut wgpu::RenderPass<'_>) -> Cursor<'g> {
        let mut cursor = Cursor {
            geometry: self,
            bound: None,
        };
        if self.is_whole() {
            cursor.bind(pass, 0);
        }
        cursor
    }

    fn chunk_of_draw(&self, first_index: u32) -> Option<usize> {
        self.draw_chunk
            .binary_search_by_key(&first_index, |&(start, _)| start)
            .ok()
            .map(|at| self.draw_chunk[at].1 as usize)
    }
}

/// Which chunk a pass has bound, so a draw rebinds only on a change.
#[derive(Debug)]
pub struct Cursor<'g> {
    geometry: &'g Geometry,
    bound: Option<usize>,
}

impl Cursor<'_> {
    fn bind(&mut self, pass: &mut wgpu::RenderPass<'_>, chunk: usize) {
        pass.set_vertex_buffer(0, self.geometry.vertices[chunk].slice(..));
        if let Some(texcoords) = &self.geometry.texcoords {
            pass.set_vertex_buffer(1, texcoords[chunk].slice(..));
        }
        self.bound = Some(chunk);
    }

    /// Draws index range `range`, one instance.
    pub fn draw(&mut self, pass: &mut wgpu::RenderPass<'_>, range: &Range<u32>) {
        self.draw_of(pass, range.start, range);
    }

    /// Draws `range` of an index buffer that holds the model's draw whose first
    /// index is `draw_start` somewhere else - the reversed copy of the indices
    /// is one.
    pub fn draw_of(
        &mut self,
        pass: &mut wgpu::RenderPass<'_>,
        draw_start: u32,
        range: &Range<u32>,
    ) {
        if self.geometry.is_whole() {
            pass.draw_indexed(range.clone(), 0, 0..1);
            return;
        }
        let Some(chunk) = self.geometry.chunk_of_draw(draw_start) else {
            debug_assert!(
                false,
                "index range {range:?} is not one of the model's draws"
            );
            return;
        };
        if self.bound != Some(chunk) {
            self.bind(pass, chunk);
        }
        pass.draw_indexed(range.clone(), self.geometry.chunks.base_vertex(chunk), 0..1);
    }
}

#[cfg(test)]
mod tests;
