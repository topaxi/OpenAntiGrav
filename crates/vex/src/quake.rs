//! `Quake` `0x3c7`: which road vertices a Quake's ripple moves, and how far
//! along the track each one sits.
//!
//! The original does not find the road it deforms at runtime. Every circuit
//! authors one `Quake` node whose payload is a ready-made table of **road
//! spans**; the loader (`QuakeNode_Load`, `0x0891beac`) points
//! `g_quake_span_table` at it and fixes up three self-relative offsets per record
//! (`Quake_FixupSpan`, `0x0891b67c`). Nothing else builds it.
//!
//! ```text
//! payload +0x00  u32  record count
//!         +0x04  u32  live count        runtime, 0 on disc
//!         +0x08  u32  1 on disc
//!         +0x0c  u32  record base       runtime, 0 on disc
//!         +0x10  records, 0x80 bytes each
//!
//! record  +0x00  f32[4]  the span's "down" axis at its start, batch-local
//!         +0x10  f32[4]  the same at its end
//!         +0x20  f32[2]  where each forward neighbour starts, in this span's distance
//!         +0x28  f32[2]  each backward neighbour's end, less this span's length
//!         +0x30  u16[2]  forward neighbours, 0xffff = none
//!         +0x34  u16[2]  backward neighbours
//!         +0x3a  u16     vertex stride
//!         +0x3c  u16     parameter stride
//!         +0x3e  u16     this record's own index
//!         +0x40  f32     length, world units
//!         +0x44  i32     first vertex's position, relative to +0x44 itself
//!         +0x48  i32     per-vertex parameters, relative to +0x48
//!         +0x4c  i32     the GE batch header, relative to +0x4c
//!         +0x50  u16     vertex count
//!         +0x52  i16     the AiTrack path this span lies along
//!         +0x54  f32     t_start, normalised along that path
//!         +0x58  f32     t_end
//!         +0x5c..0x80    runtime ripple state, zero on disc
//! ```
//!
//! # A span is one whole GE batch
//!
//! Measured on all 24 Pulse circuit files (9,226 records): `+0x4c` lands on a
//! batch header of a `Mesh`, `Speedup Pad` or `Weapon Pad` payload, `+0x44` on
//! that batch's first vertex position, and `+0x50` equals its vertex count, every
//! time. A span needs no vertex range of its own: [`Span::batch`] names the batch
//! and the whole batch is the span.
//!
//! # A parameter is where along the span a vertex is
//!
//! `0.0` at the span's start, `1.0` at its end, in units of [`Span::length`].
//! A parameter whose bits are `0xffffffff` is pinned: the fixup replaces it with
//! `1.0e8`, which clamps it out of every bump, so it is [`None`] here.
//!
//! Evidence and confidence scores:
//! `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`, "the ripple
//! itself".

use crate::vex::{Error, Node, Result};
use oag_formats::ByteOrder;

/// Class ID of a `Quake` node, in version 6.
///
/// From the exporter's class table (`docs/formats/vex.md`), confirmed by use:
/// `QuakeNode_Load` is in the class's vtable and nothing else writes the table it
/// loads.
pub const CLASS_QUAKE: u32 = 0x3c7;

/// Bytes of payload header before the first record.
const HEADER: usize = 0x10;

/// Bytes per record.
const RECORD: usize = 0x80;

/// A missing neighbour.
const NONE: u16 = 0xffff;

/// One road span: a GE batch the ripple moves, and where it lies on the track.
#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    /// The track's "down" axis at the span's start, in the batch's own space.
    ///
    /// Exactly `(0, -1, 0)` on level road. A vertex moves along
    /// `-lerp(down_start, down_end, parameter)`, which is up.
    pub down_start: [f32; 3],
    /// The same at the span's end.
    pub down_end: [f32; 3],
    /// The spans that start ahead of this one, and where each starts, in this
    /// span's own distance.
    pub forward: [Option<(u16, f32)>; 2],
    /// The spans that end behind this one, and the gap `g` that places each:
    /// the neighbour ends at `length + g` in this span's own distance.
    pub backward: [Option<(u16, f32)>; 2],
    /// World units from the span's start to its end.
    pub length: f32,
    /// File offset of the GE batch header this span moves.
    pub batch: usize,
    /// File offset of the first vertex's position field.
    ///
    /// Kept so a reader can check it against the batch's layout, which says
    /// [`Self::batch`] was resolved right.
    pub first_position: usize,
    /// The AiTrack path the span lies along.
    pub path: i16,
    /// Where the span starts, as a fraction of its path's length.
    pub t_start: f32,
    /// Where it ends.
    pub t_end: f32,
    /// Each vertex's position along the span, `0.0..=1.0`; `None` for a vertex
    /// the ripple never moves.
    pub parameters: Vec<Option<f32>>,
}

/// Every span of a file's `Quake` node, in table order.
///
/// `file` is the whole `.vex`; `node` its `Quake` node. The three offsets resolve
/// to file offsets as `Quake_FixupSpan` resolves them to pointers (meaningful
/// because the loader keeps a file in one piece). Little-endian: only the PSP
/// authors geometry for a span to move.
pub fn spans(file: &[u8], node: &Node) -> Result<Vec<Span>> {
    let payload = node.payload();
    if payload.end > file.len() || payload.len() < HEADER {
        return Err(Error::TooShort { got: payload.len() });
    }
    let le = ByteOrder::Little;
    let count = le.u32(file, payload.start) as usize;
    let records = payload.start + HEADER;
    let end = records + count * RECORD;
    if end > file.len() {
        return Err(Error::OutOfBounds {
            what: "quake span records",
            end,
            len: file.len(),
        });
    }
    let vec3 = |at: usize| [le.f32(file, at), le.f32(file, at + 4), le.f32(file, at + 8)];
    let relative = |at: usize| at.checked_add_signed(le.u32(file, at) as i32 as isize);
    let link = |index_at: usize, value_at: usize| {
        let index = le.u16(file, index_at);
        (index != NONE).then(|| (index, le.f32(file, value_at)))
    };
    (0..count)
        .map(|i| {
            let r = records + i * RECORD;
            let out_of_bounds = |what| Error::OutOfBounds {
                what,
                end: file.len() + 1,
                len: file.len(),
            };
            let batch = relative(r + 0x4c).ok_or(out_of_bounds("quake span batch"))?;
            let first_position = relative(r + 0x44).ok_or(out_of_bounds("quake span vertices"))?;
            let parameters_at = relative(r + 0x48).ok_or(out_of_bounds("quake span parameters"))?;
            let stride = usize::from(le.u16(file, r + 0x3c));
            let vertex_count = usize::from(le.u16(file, r + 0x50));
            let last = parameters_at + vertex_count.saturating_sub(1) * stride + 4;
            if vertex_count > 0 && last > file.len() {
                return Err(Error::OutOfBounds {
                    what: "quake span parameters",
                    end: last,
                    len: file.len(),
                });
            }
            let parameters = (0..vertex_count)
                .map(|k| {
                    let at = parameters_at + k * stride;
                    (le.u32(file, at) != u32::MAX).then(|| le.f32(file, at))
                })
                .collect();
            Ok(Span {
                down_start: vec3(r),
                down_end: vec3(r + 0x10),
                forward: [link(r + 0x30, r + 0x20), link(r + 0x32, r + 0x24)],
                backward: [link(r + 0x34, r + 0x28), link(r + 0x36, r + 0x2c)],
                length: le.f32(file, r + 0x40),
                batch,
                first_position,
                path: le.u16(file, r + 0x52) as i16,
                t_start: le.f32(file, r + 0x54),
                t_end: le.f32(file, r + 0x58),
                parameters,
            })
        })
        .collect()
}

/// The offset of every batch header in one mesh payload's list, in the order
/// [`crate::vex::mesh_batches`] decodes them.
///
/// **The same walk, repeated**: `mesh_batches` does not hand its offsets out and
/// a span names its batch by offset. `batch_offsets_match_mesh_batches` in
/// `tests/quake_ground_truth.rs` holds the two to the same count on every mesh of
/// every circuit.
#[must_use]
pub fn batch_offsets(payload: &[u8], batch_list: u8) -> Vec<usize> {
    let le = ByteOrder::Little;
    if payload.len() < 0x30 {
        return Vec::new();
    }
    let terminator = if batch_list == 0 { 1u16 } else { 2 };
    let mut at = le.u32(payload, if batch_list == 0 { 4 } else { 8 }) as usize;
    let mut out = Vec::new();
    while at + 0x40 <= payload.len() && le.u16(payload, at) & terminator != 0 {
        out.push(at);
        let header_size = if payload[at + 3] & 0x40 != 0 {
            0x80
        } else {
            0x40
        };
        let step = header_size + usize::from(le.u16(payload, at + 0x0c));
        at += step;
    }
    out
}

#[cfg(test)]
mod tests;
