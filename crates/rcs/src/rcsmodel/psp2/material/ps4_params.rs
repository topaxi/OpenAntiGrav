//! The PS4 material instance table: the uniforms and samplers an Omega
//! `.rcsmodel` hands each shader, read off the material header's own table.
//!
//! The Vita table (`docs/formats/2048-material-params.md`) with 64-bit
//! pointers, and one improvement: a uniform states its own component count.
//!
//! ```text
//! header  +0x3c u32  count of 32-bit floats      +0x40 u64  offset of the float pool
//!         +0x48 u32  entry count                 +0x50 u64  offset of the entries
//! entry (0x28 bytes)
//!         +0x00 u32  name hash, ~crc32(name)
//!         +0x04 u32  kind: 1 a uniform, 0x12 a sampler
//! uniform +0x10 u64  offset of the value         +0x18 u32  components << 16
//!         +0x1c u32  its pool, 0x1000 the float pool
//! sampler +0x18 u64  offset of the .gnf path     +0x20 u32  sampler state
//! ```
//!
//! There is no half pool on PS4: every uniform names the float pool.

use super::{Param, cstr_at, ends_with_ci, u32_at, u64_at};

const POOL_COUNT: usize = 0x3c;
const POOL_POINTER: usize = 0x40;
const ENTRY_COUNT: usize = 0x48;
const ENTRY_TABLE: usize = 0x50;
/// Bytes per entry.
pub const STRIDE: usize = 0x28;
const KIND_UNIFORM: u32 = 1;
const KIND_SAMPLER: u32 = 0x12;
const UNIFORM_VALUE: usize = 0x10;
const UNIFORM_COMPONENTS: usize = 0x18;
const UNIFORM_POOL: usize = 0x1c;
const SAMPLER_PATH: usize = 0x18;
const POOL_F32: u32 = 0x1000;
const MAX_ENTRIES: usize = 256;

/// The entry table of the header at `header_at`: `(first entry, count)`.
fn table(cpu: &[u8], header_at: usize) -> Option<(usize, usize)> {
    let count = usize::try_from(u32_at(cpu, header_at + ENTRY_COUNT)?).ok()?;
    let at = usize::try_from(u64_at(cpu, header_at + ENTRY_TABLE)?).ok()?;
    (count > 0 && count <= MAX_ENTRIES && at.checked_add(count * STRIDE)? <= cpu.len())
        .then_some((at, count))
}

/// The uniforms of the material header at `header_at`, in table order. An
/// entry whose value does not lie wholly inside the float pool is dropped.
pub(super) fn params_at(cpu: &[u8], header_at: usize) -> Vec<Param> {
    let Some((table_at, count)) = table(cpu, header_at) else {
        return Vec::new();
    };
    let pool_len = u32_at(cpu, header_at + POOL_COUNT).unwrap_or(0) as usize;
    let Some(pool) = u64_at(cpu, header_at + POOL_POINTER).and_then(|p| usize::try_from(p).ok())
    else {
        return Vec::new();
    };
    let pool_end = pool.saturating_add(pool_len * 4);
    (0..count)
        .filter_map(|i| {
            let at = table_at + i * STRIDE;
            if u32_at(cpu, at + 4)? != KIND_UNIFORM || u32_at(cpu, at + UNIFORM_POOL)? != POOL_F32 {
                return None;
            }
            let value = usize::try_from(u64_at(cpu, at + UNIFORM_VALUE)?).ok()?;
            let components = (u32_at(cpu, at + UNIFORM_COMPONENTS)? >> 16) as usize;
            let end = value.checked_add(components * 4)?;
            if components == 0
                || components > 4
                || value < pool
                || end > pool_end
                || end > cpu.len()
            {
                return None;
            }
            let bits = (0..components)
                .map(|k| u32_at(cpu, value + k * 4).unwrap_or(0))
                .collect();
            Some(Param {
                hash: u32_at(cpu, at)?,
                bits,
            })
        })
        .collect()
}

/// The sampler entries of the material header at `header_at`: name hash and
/// the `.gnf` path.
pub(super) fn samplers_at(cpu: &[u8], header_at: usize) -> Vec<(u32, String)> {
    let Some((table_at, count)) = table(cpu, header_at) else {
        return Vec::new();
    };
    (0..count)
        .filter_map(|i| {
            let at = table_at + i * STRIDE;
            if u32_at(cpu, at + 4)? != KIND_SAMPLER {
                return None;
            }
            let target = usize::try_from(u64_at(cpu, at + SAMPLER_PATH)?).ok()?;
            let path = cstr_at(cpu, target)?;
            ends_with_ci(&path, ".gnf").then_some((u32_at(cpu, at)?, path))
        })
        .collect()
}
