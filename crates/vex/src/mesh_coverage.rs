//! Byte coverage of a `Mesh`-shaped `.vex` payload.
//!
//! `Mesh` (`vex::CLASS_MESH`), `Skycube` (a `Mesh` payload verbatim, see
//! [`vex::CLASS_SKYCUBE`]) and the two pad classes (a `Mesh` bind, then the pad's
//! trigger box already in the same header, see [`crate::pads`]) decode through
//! this one layout, so one coverage walk serves all four. See
//! `crates/formats/src/coverage.rs` for what a claim and a gap mean and
//! `docs/formats/README.md#coverage` for why this exists: the risk is highest in
//! a container of tables reached by offsets, which a batch list, material array
//! and texture-transform block array are.
//!
//! **A batch's vertex or VIF payload is claimed as one span**, not split into
//! sub-fields: [`vex::mesh_batches`] already validates those (every decoded
//! vertex must fall inside the batch's bounding box), and this instrument checks
//! whether a region is reached, not whether every field is understood. The batch
//! walk is a second, independent read of `pass_mask`, the header-size flag and
//! `payload_size`, not a call into `mesh_batches`, which returns vertices and
//! never the file offsets they came from.

use oag_formats::ByteOrder;
use oag_formats::coverage::Coverage;

use crate::vex;

/// Bytes of a mesh-shaped payload's own header, before the material array.
const HEADER_LEN: usize = 0x30;

/// Stride of one material record, from `+0x30`.
const MATERIAL_STRIDE: usize = 0x14;

/// Stride of one material's texture-transform block, immediately after the
/// material array. See [`vex::mesh_tex_transforms`].
const TEX_TRANSFORM_STRIDE: usize = 0x40;

/// A batch header is at least this long; the extended form is
/// [`BATCH_EXT_HEADER`].
const BATCH_HEADER: usize = 0x40;

/// The extended batch header, selected by `+0x03 & 0x40`.
const BATCH_EXT_HEADER: usize = 0x80;

/// Payload alignment every node's data is padded to.
const ALIGN: usize = 0x10;

fn u16_le(data: &[u8], at: usize) -> u16 {
    ByteOrder::Little.u16(data, at)
}

fn u32_le(data: &[u8], at: usize) -> u32 {
    ByteOrder::Little.u32(data, at)
}

fn align_up(value: usize) -> usize {
    value.div_ceil(ALIGN) * ALIGN
}

/// Coverage of one `Mesh`-shaped payload.
///
/// Safe on any payload: a truncated or non-mesh blob stops claiming early, and
/// `min` in [`Coverage::gaps`] turns a missing header into a visible gap.
#[must_use]
pub fn coverage(payload: &[u8]) -> Coverage {
    let mut seen = Coverage::new(payload.len());
    if payload.len() < HEADER_LEN {
        return seen;
    }
    seen.claim(0, HEADER_LEN, "the mesh header");

    let material_count = usize::from(u16_le(payload, 2));
    let materials_len = material_count * MATERIAL_STRIDE;
    seen.claim(HEADER_LEN, materials_len, "the material array");

    let tex_transform_base = HEADER_LEN + materials_len;
    let materials = vex::mesh_materials(payload);
    if materials.iter().flatten().any(|m| m.flags & 0x10 != 0) {
        seen.claim(
            tex_transform_base,
            material_count * TEX_TRANSFORM_STRIDE,
            "the texture-transform block array",
        );
        for (index, material) in materials.iter().enumerate() {
            let Some(material) = material else { continue };
            if material.flags & 0x10 == 0 {
                continue;
            }
            claim_tex_transform_keys(&mut seen, payload, tex_transform_base, index);
        }
    }

    for (list, terminator) in [(0u8, 1u16), (1u8, 2u16)] {
        claim_batch_list(&mut seen, payload, list, terminator);
    }

    // `Skycube`'s own leftover (`docs/formats/skycube.md#the-extra-block`); only
    // fires when there is a gap before the geometry offset, else an ordinary gap.
    let materials_end = align_up(tex_transform_base);
    if let Some(block) = skycube_extra_block(payload, materials_end) {
        seen.claim(
            block.at,
            block.len,
            "an unidentified per-material block (stale editor memory, docs/formats/skycube.md)",
        );
    }

    seen
}

/// Claims one material's texture-transform key arrays, the offsets
/// [`vex::mesh_tex_transforms`] resolves internally but does not expose.
fn claim_tex_transform_keys(
    seen: &mut Coverage,
    payload: &[u8],
    tex_transform_base: usize,
    material_index: usize,
) {
    let block = tex_transform_base + material_index * TEX_TRANSFORM_STRIDE;
    if block + 0x18 > payload.len() {
        return;
    }
    let offset_count = usize::from(u16_le(payload, block));
    let scale_count = usize::from(u16_le(payload, block + 2));
    let mut claim_track = |count: usize, times_rel: usize, values_rel: usize| {
        if block + values_rel + 4 > payload.len() {
            return;
        }
        let times_at = tex_transform_base + u32_le(payload, block + times_rel) as usize;
        let values_at = tex_transform_base + u32_le(payload, block + values_rel) as usize;
        if times_at + count * 2 <= payload.len() {
            seen.claim(times_at, count * 2, "a texture-transform key time array");
        }
        if values_at + count * 4 <= payload.len() {
            seen.claim(values_at, count * 4, "a texture-transform key value array");
        }
    };
    claim_track(offset_count, 0x04, 0x10);
    claim_track(scale_count, 0x08, 0x14);
}

/// Claims every batch of one list by re-reading the three header fields
/// [`vex::mesh_batches`] steps on (`pass_mask`, header-size flag at `+0x03`,
/// `payload_size` at `+0x0c`), stopping on the same conditions, so it walks
/// exactly the batches that decode.
fn claim_batch_list(seen: &mut Coverage, payload: &[u8], list: u8, terminator: u16) {
    let list_offset = u32_le(payload, if list == 0 { 4 } else { 8 }) as usize;
    // Zero is real ("offset to batch list B, 0 on every sky",
    // `docs/formats/skycube.md`): it means the list does not exist, not that it
    // starts at the header. Walking from inside the header would read its bytes as
    // a `pass_mask` and fabricate "batch" claims instead of the honest gap.
    if list_offset < HEADER_LEN {
        return;
    }
    let mut at = list_offset;
    while at + BATCH_HEADER <= payload.len() {
        let pass_mask = u16_le(payload, at);
        if pass_mask & terminator == 0 {
            // Not a batch: the header-shaped record that ends the list. It
            // occupies a fixed `BATCH_HEADER` bytes on every list-end in the
            // corpus, so it is claimed (else the sweep reported it as a gap).
            seen.claim(at, BATCH_HEADER, "a batch-list terminator record");
            break;
        }
        let flags = payload[at + 3];
        let header_size = if flags & 0x40 != 0 {
            BATCH_EXT_HEADER
        } else {
            BATCH_HEADER
        };
        let payload_size = usize::from(u16_le(payload, at + 0x0c));
        let step = header_size + payload_size;
        if step == 0 {
            break;
        }
        seen.claim(at, step, "a batch");
        at += step;
    }
}

/// A structurally-closed but semantically undecoded block between the
/// material array and the batch geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkycubeExtraBlock {
    /// File offset of the block, at the aligned end of the material array.
    pub at: usize,
    /// Length in bytes, reaching exactly to the geometry offset at `+0x04`.
    pub len: usize,
    /// How many `0x40`-byte records the block carries.
    pub record_count: usize,
}

/// Recognises the shape `docs/formats/skycube.md#the-extra-block` measured on
/// `06_Track`'s sky: a `0x10`-byte lead-in whose first word states the byte
/// length of what follows, that many bytes of `0x40`-byte records each
/// self-indexed at `+0x20` (`0`, `1`, `2`, ...), then zero padding to the
/// geometry offset.
///
/// **Structure only, not meaning.** The records hold stale PSP main-RAM pointers
/// (`0x080db6c0` on every one measured) and neither `Skycube` handler is
/// recovered, so what reads this block is open. `None` on anything that does not
/// close exactly: a near-miss is an ordinary reported gap.
#[must_use]
pub fn skycube_extra_block(payload: &[u8], materials_end: usize) -> Option<SkycubeExtraBlock> {
    if materials_end + 0x30 > payload.len() {
        return None;
    }
    let geometry_at = u32_le(payload, 4) as usize;
    if geometry_at <= materials_end || geometry_at > payload.len() {
        return None;
    }
    let gap = geometry_at - materials_end;
    if gap < 0x10 + TEX_TRANSFORM_STRIDE {
        return None;
    }
    let records_len = u32_le(payload, materials_end) as usize;
    if records_len == 0 || !records_len.is_multiple_of(TEX_TRANSFORM_STRIDE) {
        return None;
    }
    let lead_in = 0x10;
    if lead_in + records_len > gap {
        return None;
    }
    let records_at = materials_end + lead_in;
    let record_count = records_len / TEX_TRANSFORM_STRIDE;
    for index in 0..record_count {
        let record = records_at + index * TEX_TRANSFORM_STRIDE;
        if u32_le(payload, record + 0x20) as usize != index {
            return None;
        }
    }
    let pad_at = records_at + records_len;
    let pad_len = geometry_at - pad_at;
    if payload[pad_at..pad_at + pad_len].iter().any(|&b| b != 0) {
        return None;
    }
    Some(SkycubeExtraBlock {
        at: materials_end,
        len: gap,
        record_count,
    })
}

/// Coverage of a `fogCube` payload: fixed-size with no offset table, so nothing to
/// claim beyond the 128 bytes. Kept so the corpus sweep in
/// `crates/vex/tests/payload_coverage_ground_truth.rs` reports every environment
/// class it touches.
#[must_use]
pub fn fogcube_coverage(payload: &[u8]) -> Coverage {
    let mut seen = Coverage::new(payload.len());
    seen.claim(0, payload.len().min(128), "the fogCube volume");
    seen
}

#[cfg(test)]
mod tests;
