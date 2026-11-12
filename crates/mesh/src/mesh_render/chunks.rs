//! A model's vertices split across several GPU buffers, for a device whose
//! `max_buffer_size` is smaller than the circuit's vertex data.
//!
//! **Cut where no draw straddles.** A draw indexes `model.indices` with
//! absolute vertex numbers, so a chunk is a contiguous vertex range and the
//! draw is issued with a negative `base_vertex` that shifts those numbers into
//! the chunk's buffer. That keeps the one index buffer, the draw ranges and
//! every per-draw table as they are; only the vertex buffer binding and the
//! base vertex change. A cut between vertex `b - 1` and `b` is allowed when no
//! draw touches both sides of it.
//!
//! A model that fits is one chunk and is drawn exactly as before: bound once,
//! base vertex 0.

use std::ops::Range;

use crate::mesh::{GpuVertex, Model};

/// Bytes of one vertex in the buffer this module sizes.
pub const VERTEX_BYTES: u64 = std::mem::size_of::<GpuVertex>() as u64;

/// The vertex ranges a model's buffers are cut into, in order, tiling
/// `0..vertices` with no gap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VertexChunks {
    ranges: Vec<Range<u32>>,
}

/// A draw reaches across every cut that would keep the chunks under the cap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unsplittable {
    /// The vertices of the stretch with no allowed cut inside the cap.
    pub stretch: Range<u32>,
    /// The cap, in vertices.
    pub cap: u32,
}

impl std::fmt::Display for Unsplittable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "vertices {}..{} cannot be cut into buffers of {} vertices: draws straddle every cut",
            self.stretch.start, self.stretch.end, self.cap
        )
    }
}

impl std::error::Error for Unsplittable {}

impl VertexChunks {
    /// One chunk holding `vertices`.
    #[must_use]
    pub fn whole(vertices: u32) -> Self {
        Self {
            ranges: std::iter::once(0..vertices).collect(),
        }
    }

    /// The chunks of `model` under a buffer ceiling of `max_buffer_bytes`.
    ///
    /// # Errors
    ///
    /// [`Unsplittable`] when some stretch of vertices longer than the cap has
    /// no cut that every draw respects.
    pub fn plan(model: &Model, max_buffer_bytes: u64) -> Result<Self, Unsplittable> {
        let draws = model
            .draws
            .iter()
            .chain(&model.alpha_tested_draws)
            .chain(&model.transparent_draws)
            .map(|draw| draw.range.clone());
        Self::plan_ranges(
            model.vertices.len() as u32,
            &model.indices,
            draws,
            max_buffer_bytes,
        )
    }

    /// [`Self::plan`] over `vertices` vertices, whose draws are `ranges` of
    /// `indices`.
    ///
    /// # Errors
    ///
    /// As [`Self::plan`].
    pub fn plan_ranges(
        vertices: u32,
        indices: &[u32],
        ranges: impl Iterator<Item = Range<u32>>,
        max_buffer_bytes: u64,
    ) -> Result<Self, Unsplittable> {
        let cap = (max_buffer_bytes / VERTEX_BYTES).min(u64::from(u32::MAX)) as u32;
        if vertices <= cap {
            return Ok(Self::whole(vertices));
        }
        let straddled = straddled_cuts(vertices, indices, ranges);
        Self::cut(vertices, cap, &straddled)
    }

    fn cut(vertices: u32, cap: u32, straddled: &[bool]) -> Result<Self, Unsplittable> {
        let mut ranges = Vec::new();
        let mut start = 0u32;
        while start < vertices {
            let limit = start.saturating_add(cap).min(vertices);
            let end = if limit == vertices {
                vertices
            } else {
                (start + 1..=limit)
                    .rev()
                    .find(|&cut| !straddled[cut as usize])
                    .ok_or(Unsplittable {
                        stretch: start..limit,
                        cap,
                    })?
            };
            ranges.push(start..end);
            start = end;
        }
        Ok(Self { ranges })
    }

    /// How many buffers.
    #[must_use]
    pub fn len(&self) -> usize {
        self.ranges.len()
    }

    /// Whether there are no vertices at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ranges.iter().all(Range::is_empty)
    }

    /// Whether one buffer holds everything: the unchanged, bind-once path.
    #[must_use]
    pub fn is_whole(&self) -> bool {
        self.ranges.len() <= 1
    }

    /// Chunk `chunk`'s vertex range in the model.
    #[must_use]
    pub fn range(&self, chunk: usize) -> Range<u32> {
        self.ranges[chunk].clone()
    }

    /// Every chunk's vertex range, in order.
    #[must_use]
    pub fn ranges(&self) -> &[Range<u32>] {
        &self.ranges
    }

    /// The chunk holding vertex `vertex`.
    #[must_use]
    pub fn chunk_of(&self, vertex: u32) -> usize {
        self.ranges
            .partition_point(|r| r.end <= vertex)
            .min(self.ranges.len().saturating_sub(1))
    }

    /// The `base_vertex` that maps the model's absolute vertex numbers into
    /// chunk `chunk`'s buffer.
    #[must_use]
    pub fn base_vertex(&self, chunk: usize) -> i32 {
        -(self.ranges[chunk].start as i32)
    }

    /// `count` vertices from vertex `first` of the model, split along the
    /// chunk boundaries: `(chunk, first vertex within the chunk, the span of
    /// the source slice those vertices come from)`.
    pub fn split(
        &self,
        first: u32,
        count: u32,
    ) -> impl Iterator<Item = (usize, u32, Range<usize>)> + '_ {
        let end = first + count;
        let from = self.chunk_of(first);
        self.ranges[from..]
            .iter()
            .enumerate()
            .map(move |(i, r)| (from + i, r))
            .take_while(move |(_, r)| r.start < end)
            .filter_map(move |(chunk, r)| {
                let lo = first.max(r.start);
                let hi = end.min(r.end);
                (lo < hi).then(|| {
                    (
                        chunk,
                        lo - r.start,
                        (lo - first) as usize..(hi - first) as usize,
                    )
                })
            })
    }
}

/// `result[b]` is whether some draw uses a vertex below `b` and one at or
/// above it, so a cut before vertex `b` would split that draw in two.
fn straddled_cuts(
    vertices: u32,
    indices: &[u32],
    draws: impl Iterator<Item = Range<u32>>,
) -> Vec<bool> {
    let mut opened = vec![0i32; vertices as usize + 2];
    for draw in draws {
        let Some(span) = indices.get(draw.start as usize..draw.end as usize) else {
            continue;
        };
        let (Some(&lo), Some(&hi)) = (span.iter().min(), span.iter().max()) else {
            continue;
        };
        // Cuts lo + 1 ..= hi are straddled.
        opened[lo as usize + 1] += 1;
        opened[hi as usize + 1] -= 1;
    }
    let mut open = 0;
    opened[..=vertices as usize]
        .iter()
        .map(|delta| {
            open += delta;
            open > 0
        })
        .collect()
}

#[cfg(test)]
mod tests;
