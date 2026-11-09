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

use oag_formats::swizzle::{FLAG_SWIZZLED, SWIZZLE_BLOCK_BYTES, swizzle};

use crate::xxhash::{xxh32, xxh64};
use crate::{fnt, texture};

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

/// How many bytes of guest memory the palette hash reads, from the palette's
/// first byte: a full 256-entry RGBA8888 palette. The game loads every palette
/// as 256 entries, a 4-bit texture's sixteen included, so for those the hash
/// also covers the 960 bytes after the palette.
pub const CLUT_WINDOW_BYTES: usize = 1024;

/// The `CLUTFORMAT` word the game programs: RGBA8888 entries, no shift, index
/// mask `0xff`, start 0.
pub const CLUT_REGISTER: u32 = 0xC500_FF03;

/// A palette-indexed texture as a file stores it, before the loader has put it
/// where the GE reads it.
#[derive(Debug, Clone, Copy)]
pub struct Stored<'a> {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// 4 or 8.
    pub bits_per_pixel: u32,
    /// Levels in the texel block.
    pub mip_levels: u8,
    /// Whether the texels are already in the GE's block order (the file's flag).
    /// The loader swizzles a texture whose flag is clear and leaves a flagged one
    /// alone.
    pub swizzled: bool,
    /// The palette bytes, RGBA8888.
    pub palette: &'a [u8],
    /// The texel block, every level.
    pub texels: &'a [u8],
    /// Whether level 0's rows are padded to a 16-byte stride like every level
    /// below it. `.vex` and `.fnt` textures are; a standalone `.mip` stores
    /// level 0 unpadded (which only differs for a row under 16 bytes).
    pub pad_level0: bool,
}

/// What a texture's key is made of, and whether it can be known from the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Replacement {
    /// The `dim` field of the key.
    pub dim: u32,
    /// [`data_hash`] of level 0 as it sits in guest memory.
    pub data: u32,
    /// The full key; `None` when the palette hash reads past the end of this
    /// texture's own block (palette plus texels shorter than
    /// [`CLUT_WINDOW_BYTES`]) into whatever the heap holds next.
    pub key: Option<Key>,
}

impl<'a> Stored<'a> {
    /// A standalone `.mip` blob, validated by the same checks
    /// [`texture::Texture::parse`] makes.
    ///
    /// # Errors
    ///
    /// Whatever [`texture::Texture::parse`] refuses the blob for.
    pub fn from_mip(data: &'a [u8]) -> Result<Self, texture::Error> {
        let parsed = texture::Texture::parse(data)?;
        let palette_len = 4usize << parsed.bits_per_pixel;
        Ok(Self {
            width: u32::from(parsed.width),
            height: u32::from(parsed.height),
            bits_per_pixel: u32::from(parsed.bits_per_pixel),
            mip_levels: parsed.mip_levels,
            swizzled: data[7] & FLAG_SWIZZLED != 0,
            palette: &data[texture::HEADER_LEN..texture::HEADER_LEN + palette_len],
            texels: &data[texture::HEADER_LEN + palette_len..],
            pad_level0: false,
        })
    }

    /// One texture of a PSP `.vex`, from the header fields and the palette and texel bytes
    /// exactly as the file stores them (`oag_vex::vex::StoredTexture`). Rows are padded to
    /// the 16-byte stride at every level, level 0 included.
    #[must_use]
    pub fn from_vex(
        (width, height): (u16, u16),
        bits_per_pixel: u8,
        mip_levels: u8,
        flags: u8,
        palette: &'a [u8],
        texels: &'a [u8],
    ) -> Self {
        Self {
            width: u32::from(width),
            height: u32::from(height),
            bits_per_pixel: u32::from(bits_per_pixel),
            mip_levels,
            swizzled: flags & FLAG_SWIZZLED != 0,
            palette,
            texels,
            pad_level0: true,
        }
    }

    /// The glyph atlas of a PSP `.fnt`, validated by [`fnt::Font::parse`].
    ///
    /// # Errors
    ///
    /// Whatever [`fnt::Font::parse`] refuses the font for.
    pub fn from_font_atlas(data: &'a [u8]) -> Result<Self, fnt::Error> {
        let font = fnt::Font::parse(data)?;
        let metrics = fnt::Metrics::parse(data)?;
        let atlas = &data[metrics.atlas_at..];
        let clut_size = metrics.order.u32(atlas, 8) as usize;
        Ok(Self {
            width: u32::from(font.width),
            height: u32::from(font.height),
            bits_per_pixel: 4,
            mip_levels: 1,
            swizzled: atlas[6] & FLAG_SWIZZLED != 0,
            palette: &atlas[fnt::ATLAS_HEADER_LEN..fnt::ATLAS_HEADER_LEN + clut_size],
            texels: &atlas[fnt::ATLAS_HEADER_LEN + clut_size..],
            pad_level0: true,
        })
    }
}

impl Stored<'_> {
    /// The texel block as the GE reads it: as stored when flagged, otherwise
    /// each level swizzled on its own at its own stride (`Texture_SwizzleForGe`).
    #[must_use]
    pub fn guest_texels(&self) -> Vec<u8> {
        if self.swizzled {
            return self.texels.to_vec();
        }
        let mut out = Vec::with_capacity(self.texels.len());
        let mut at = 0;
        for level in 0..u32::from(self.mip_levels.max(1)) {
            let height = (self.height >> level).max(1) as usize;
            let (_, stride) = self.level_strides(level);
            let Some(plane) = self.texels.get(at..at + stride * height) else {
                break;
            };
            out.extend_from_slice(&swizzle(plane, stride, height));
            at += stride * height;
        }
        out.extend_from_slice(&self.texels[at.min(self.texels.len())..]);
        out
    }

    /// `(bytes of picture, bytes of stride)` of one level's rows.
    fn level_strides(&self, level: u32) -> (usize, usize) {
        let width = (self.width >> level).max(1);
        let row = (width * self.bits_per_pixel).div_ceil(8) as usize;
        if level == 0 && !self.pad_level0 {
            (row, row)
        } else {
            (row, row.next_multiple_of(SWIZZLE_BLOCK_BYTES))
        }
    }

    /// The key PPSSPP would name this texture by, from the file's bytes alone.
    ///
    /// `None` when level 0 does not fit the block (a truncated file).
    #[must_use]
    pub fn replacement(&self) -> Option<Replacement> {
        let guest = self.guest_texels();
        let (_, stride) = self.level_strides(0);
        let bufw = (stride as u32 * 8) / self.bits_per_pixel;
        let level0 = guest.get(..stride * self.height as usize)?;
        let data = data_hash(
            level0,
            bufw,
            self.width,
            self.height,
            self.bits_per_pixel,
            0,
        )?;
        let dim = dim(self.width, self.height);
        let own = self.palette.len() + guest.len();
        let key = (own >= CLUT_WINDOW_BYTES).then(|| {
            let mut window = Vec::with_capacity(CLUT_WINDOW_BYTES);
            window.extend_from_slice(&self.palette[..self.palette.len().min(CLUT_WINDOW_BYTES)]);
            let rest = CLUT_WINDOW_BYTES - window.len();
            window.extend_from_slice(&guest[..rest.min(guest.len())]);
            Key::with_palette(dim, clut_hash(&window), CLUT_REGISTER, data)
        });
        Some(Replacement { dim, data, key })
    }
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
