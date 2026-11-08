//! PPSSPP's texture-replacement key: what a pack's `textures.ini` is indexed by.
//!
//! A replacement pack names a texture by a hash of the bytes the game left in
//! guest memory for the GE: palette-indexed and, on both Wipeout titles,
//! swizzled. It is not a hash of the decoded picture, so the input here is a
//! texture's *stored* layout, not an RGBA8888 buffer. See
//! `docs/formats/ppsspp-texture-pack.md` for the evidence and the census.
//!
//! # The 24-hex key
//!
//! With `ignoreAddress = true` (both packs) a key is
//!
//! ```text
//! 00000000  LLLLLLLL  DDDDDDDD
//! address   dim^clut  data hash
//! ```
//!
//! - `DDDDDDDD`: the low 32 bits of XXH64 (seed `0xBACD7814`) over the texture's
//!   bytes, [`data_hash`].
//! - `LLLLLLLL`: the texture size register's `dim` ([`dim`]), XORed - only for a
//!   palette format - with [`clut_hash`] XOR the whole `CLUTFORMAT` register word.
//!   [`clut_register`] builds that word, command byte `0xC5` included.

use crate::xxhash::{xxh32, xxh64};

/// Seed of the data hash (`TextureReplacer::ComputeHash`).
pub const DATA_SEED: u64 = 0xBACD_7814;

/// Seed of the palette hash (`TextureCache*::UpdateCurrentClut` while a
/// replacer is enabled).
pub const CLUT_SEED: u32 = 0xC010_8888;

/// The GE command byte (`0xC5`, `CLUTFORMAT`) held in the top of the register
/// word PPSSPP XORs into the palette hash.
pub const CLUT_FORMAT_COMMAND: u32 = 0xC500_0000;

/// The GE palette entry formats a `CLUTFORMAT` register names (bits 0-1).
pub const CLUT_FORMAT_ABGR8888: u32 = 3;

/// `textureBitsPerPixel` for the two palette-index formats Wipeout uses.
pub const CLUT4_BITS: u32 = 4;
/// See [`CLUT4_BITS`].
pub const CLUT8_BITS: u32 = 8;

/// The data hash of a texture's bytes, as PPSSPP computes it before decode.
///
/// `memory` starts at the texture's first byte (level 0). `bufw` is the GE's
/// buffer width in pixels, `width`/`height` the texture size the game declared
/// (powers of two). Swizzling is not taken into account, so a swizzled texture
/// is hashed as raw bytes - the byte order the GE reads.
///
/// `None` when `memory` is shorter than the hashed span (PPSSPP returns 0 and
/// logs an error there).
///
/// `max_seen_v` is PPSSPP's highest sampled V for a 512-tall texture drawn in
/// through mode; pass 0 when unknown.
#[must_use]
pub fn data_hash(
    memory: &[u8],
    bufw: u32,
    width: u32,
    height: u32,
    bits_per_pixel: u32,
    max_seen_v: u16,
) -> Option<u32> {
    let height = if height == 512 && max_seen_v < 512 && max_seen_v != 0 {
        u32::from(max_seen_v)
    } else {
        height
    };
    if bufw <= width {
        // Contiguous: the used pixels, the last row short by `width - bufw`
        // (which is negative here, hence the wrapping add PPSSPP also does).
        let total_pixels = (bufw * height).wrapping_add(width.wrapping_sub(bufw));
        let size = (bits_per_pixel * total_pixels / 8) as usize;
        let span = memory.get(..size)?;
        Some(xxh64(span, DATA_SEED) as u32)
    } else {
        let per_line = (bits_per_pixel * width / 8) as usize;
        let stride = (bits_per_pixel * bufw / 8) as usize;
        let mut result = 0u32;
        for y in 0..height as usize {
            let row = memory.get(y * stride..y * stride + per_line)?;
            result = result.wrapping_mul(11) ^ (xxh64(row, DATA_SEED) as u32);
        }
        Some(result)
    }
}

/// The palette hash: XXH32 of the palette bytes as the GE reads them.
#[must_use]
pub fn clut_hash(palette: &[u8]) -> u32 {
    xxh32(palette, CLUT_SEED)
}

/// The `dim` field of a cache key: `log2(width) | log2(height) << 8`, the two
/// exponents of the GE's texture size register (`texsize & 0xf0f`).
#[must_use]
pub fn dim(width: u32, height: u32) -> u32 {
    width.trailing_zeros() | (height.trailing_zeros() << 8)
}

/// The `CLUTFORMAT` register word: format in bits 0-1, index shift in 2-6, mask
/// in 8-15, start position (in units of 16) in 16-20, and the command byte.
#[must_use]
pub fn clut_register(format: u32, shift: u32, mask: u32, start_blocks: u32) -> u32 {
    CLUT_FORMAT_COMMAND | format | (shift << 2) | (mask << 8) | (start_blocks << 16)
}

/// A 24-hex key with the address field dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Key {
    /// The middle eight hex digits: `dim`, XOR the palette hash and register for
    /// a palette format.
    pub dim_clut: u32,
    /// The last eight: [`data_hash`].
    pub data: u32,
}

impl Key {
    /// Parses `AAAAAAAALLLLLLLLDDDDDDDD`, ignoring the address half. `None` for
    /// anything that is not exactly 24 hex digits.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        if text.len() != 24 || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        Some(Self {
            dim_clut: u32::from_str_radix(&text[8..16], 16).ok()?,
            data: u32::from_str_radix(&text[16..24], 16).ok()?,
        })
    }

    /// The key of a palette texture: `dim ^ clut_hash ^ clut_register`.
    #[must_use]
    pub fn with_palette(dim: u32, clut_hash: u32, clut_register: u32, data: u32) -> Self {
        Self {
            dim_clut: dim ^ clut_hash ^ clut_register,
            data,
        }
    }
}

impl std::fmt::Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "00000000{:08x}{:08x}", self.dim_clut, self.data)
    }
}

#[cfg(test)]
mod tests;
