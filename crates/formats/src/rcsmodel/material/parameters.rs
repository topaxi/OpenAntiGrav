//! The per-instance parameter table a material record carries, past everything
//! `material.rs` reads.
//!
//! **This is where a shipped shader's numbers actually live.** A
//! `.rcsmaterial` declares that its program takes a `power1` and a `scale1`; it
//! does not say what they are. The *values* are authored per model, in the
//! material record inside the `.rcsmodel`, and until this module existed they
//! were the unread tail `material.rs` describes as "records are not a fixed
//! size ... everything above sits inside the smallest of them".
//!
//! # Layout
//!
//! Two more words of the record name the table:
//!
//! ```text
//! +0x30  u32   entry count
//! +0x34  u32   file offset of the entries
//! ```
//!
//! and each entry is `0x20` bytes:
//!
//! ```text
//! +0x00  u32   name hash, `!crc32(name)` - the same hashing
//!              `oag_formats::rcsmaterial::name_hash` already does
//! +0x04  u32   kind: `0x8001` for a sampler, `0` for a parameter
//! +0x18  u32   file offset of the value (a parameter) or of the texture path
//!              (a sampler)
//! +0x1c  u32   how many `vec4`s the value is
//! ```
//!
//! A parameter's value is a big-endian `f32` quad however few components the
//! shader declares, so a `float1` is a quad with its answer in `x` and zeroes
//! after it.
//!
//! # What it reads on the disc
//!
//! Feisar's `engineflare.rcsmodel`, whose one material is
//! `engines/flame_test.rcsmaterial`, carries eight entries - two samplers and
//! **six parameters**, every one of which the flame's own fragment program
//! patches into an inline constant:
//!
//! | Hash | Name | Value |
//! | --- | --- | ---: |
//! | `0xaa5e39a1` | `power1` | 10.0 |
//! | `0x961a1154` | `scale1` | 0.3 |
//! | `0xfa51b981` | `min1` | 0.45 |
//! | `0x31182e0d` | `Speed` | 2.0 |
//! | `0x92fc84bf` | (no preimage) | 2.0 |
//! | `0x17d9b3d3` | (no preimage) | 1.0 |
//!
//! and a hull's materials carry `SpecScale` (`0x4232e459`) at 235 on the paint
//! and 500 on the glass, and `Bloom` (`0xf8f3ade0`) at 1.0 on
//! `emissive_bloom` - so the specular exponent
//! `docs/ghidra/functions/ps3-hdfury-eu/renderer.md` records as "32, the
//! commonest of three round values, a stand-in" is authored per material
//! instance after all. **That is a finding this module makes possible and does
//! not act on**; nothing reads `SpecScale` yet.

use crate::ByteOrder;

/// Bytes per entry.
const ENTRY_LEN: usize = 0x20;

/// The `+0x04` value a **sampler** entry carries, where a parameter carries 0.
///
/// Kept as a named constant because it is what makes a value pointer safe to
/// follow: a sampler's `+0x18` is a string offset, and reading four floats
/// there would be reading a path as numbers.
pub const KIND_SAMPLER: u32 = 0x8001;

/// One named value a material instance supplies to its shader.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Parameter {
    /// `!crc32(name)`. The names are recovered by preimage, a few at a time -
    /// see `docs/ghidra/functions/ps3-hdfury-eu/engine-flare.md`.
    pub hash: u32,
    /// The first `vec4` of the value.
    ///
    /// **The first only, and that is deliberate.** Every parameter on the
    /// craft models measured declares a count of one, and a caller that wanted
    /// a longer array would need the shader's declared type to know how to read
    /// it - which lives in the `.rcsmaterial`, not here. A count above one is
    /// reported by [`Self::quads`] rather than silently truncated.
    pub value: [f32; 4],
    /// How many `vec4`s the record says the value is.
    pub quads: u32,
}

/// Reads the parameter entries of the material record at `at`.
///
/// Samplers are skipped: their `+0x18` is a path rather than a value. An entry
/// whose value would leave the file is skipped too, which is what a record
/// shorter than `+0x38` looks like from here.
#[must_use]
pub fn parameters(data: &[u8], at: usize) -> Vec<Parameter> {
    let Some(count) = word(data, at + 0x30) else {
        return Vec::new();
    };
    let Some(table) = word(data, at + 0x34) else {
        return Vec::new();
    };
    // A record that carries no table at all reads as a huge count off whatever
    // those bytes happen to be, so the walk is bounded by the file as well as
    // by the count.
    (0..count as usize)
        .map_while(|index| {
            let entry = table as usize + index * ENTRY_LEN;
            let hash = word(data, entry)?;
            let kind = word(data, entry + 0x04)?;
            let offset = word(data, entry + 0x18)?;
            let quads = word(data, entry + 0x1c)?;
            Some((hash, kind, offset, quads))
        })
        .filter(|&(_, kind, _, quads)| kind != KIND_SAMPLER && quads > 0)
        .filter_map(|(hash, _, offset, quads)| {
            let at = offset as usize;
            let value = [
                float(data, at)?,
                float(data, at + 4)?,
                float(data, at + 8)?,
                float(data, at + 12)?,
            ];
            Some(Parameter { hash, value, quads })
        })
        .collect()
}

/// A big-endian `u32`, or `None` where it would leave the file.
fn word(data: &[u8], at: usize) -> Option<u32> {
    (at + 4 <= data.len()).then(|| ByteOrder::Big.u32(data, at))
}

/// A big-endian `f32`, or `None` where it would leave the file.
fn float(data: &[u8], at: usize) -> Option<f32> {
    (at + 4 <= data.len()).then(|| ByteOrder::Big.f32(data, at))
}
