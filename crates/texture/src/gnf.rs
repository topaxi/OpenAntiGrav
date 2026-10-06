//! The PS4's texture container (`.gnf`) - header and descriptor only.
//!
//! **Confidence: 80.** A public Sony format ("Gnm Format") with no first-party
//! spec held here, so the layout below is triangulated from two open-source
//! implementations; the AMD GCN "T#" image descriptor it embeds is from a public
//! AMD document. Not confirmed against a running PS4, hence 80 rather than the
//! 90s a byte-identical Sony spec would earn (the same distinction as
//! [`docs/formats/psarc.md`](../../../docs/formats/psarc.md)).
//!
//! ```text
//! header, 8 bytes:
//!   +0x00  char[4]  "GNF "
//!   +0x04  u32      contents size (bytes from +0x08 to where pixel data starts)
//!
//! contents, 8 bytes, starting at +0x08:
//!   +0x08  u8       version         2 on every file this project has seen
//!   +0x09  u8       texture count
//!   +0x0a  u8       alignment       8 on every file this project has seen
//!   +0x0b  u8       reserved
//!   +0x0c  u32      stream size (total file length)
//!
//! then `texture count` descriptors, 36 bytes each, starting at +0x10:
//!   +0x00  u32      word0 - GPU base address, always 0 in a file on disk
//!   +0x04  u32      word1 - min_lod_clamp:12 @8, surface_format:6 @20, channel_type:4 @26
//!   +0x08  u32      word2 - (width-1):14 @0, (height-1):14 @14, sampler_modulation:3 @28
//!   +0x0c  u32      word3 - channel_order x/y/z/w: 3 bits each @0/3/6/9,
//!                            base_mip_level:4 @12, last_mip_level:4 @16,
//!                            tile_mode:5 @20, is_pow2_pad:1 @25, texture_type:4 @28
//!   +0x10  u32      word4 - (depth-1):13 @0, (pitch-1):14 @13
//!   +0x14  u32      word5 - base_array_slice:13 @0, last_array_slice:13 @13
//!   +0x18  u32      word6 - min_lod_warning, mip stats, DCC flags (not decoded further)
//!   +0x1c  u32      metadata offset (or pixel-data length, for a single texture)
//!
//! then, at `+0x08 + contents_size` (conventionally `+0x100`): pixel data, one
//! run per texture, `stream_size - (+0x08 + contents_size)` bytes total.
//! ```
//!
//! Little-endian throughout. See `docs/formats/README.md`'s Omega Collection rows.
//!
//! # Where this layout comes from
//!
//! Three independent open-source projects agree on it, checked against this
//! project's extracted `.gnf` files (see [`tests`]):
//!
//! 1. [GFD-Studio](https://github.com/tge-was-taken/GFD-Studio)'s `GNFTexture`
//!    gives the offsets and bitfield widths above, verified on
//!    `Data/art/published/hdships/harimau/Livery2/Holographic_02_GLOW.gnf`
//!    (`omega-ps4-eu`): `word1` is `SurfaceFormat::BC7`/`ChannelType::Srgb`,
//!    `word2` 128x64, `word3` an RGBA channel order (4,5,6,7) and tile mode 13
//!    (`Thin_1DThin`, micro-tiled; **14** is `Thin_2DThin`, macro-tiled).
//! 2. The [PlayStation GNF Image page](https://rewiki.miraheze.org/wiki/PlayStation_GNF_Image)
//!    gives the same header/contents split and fixed values (version 2,
//!    alignment 8).
//! 3. [shadPS4](https://github.com/shadps4-emu/shadPS4)'s `AmdGpu::Image`
//!    (`src/video_core/amdgpu/resource.h`) is the same descriptor one layer down,
//!    the GCN "T#" of AMD's public "Sea Islands" ISA manual (Table 8.13); its
//!    `DataFormat`/`NumberFormat` values match GFD-Studio's (`BC7 = 0x29`,
//!    `Srgb = 9`).
//!
//! # What this does not do
//!
//! **[`Texture::decode`] untiles a linear surface unconditionally, and a
//! micro-tiled (`Thin_1DThin`) BC7 surface only when its base level carries no
//! corrupt block.** AMD's `ComputeSurfaceAddrFromCoordMicroTiled` (Mesa `addrlib`,
//! `egbaddrlib.cpp`) has no banks, pipes or row-size unknowns. Against
//! oracle-paired textures (`crates/texture/src/gnf/micro_tile_tests.rs`,
//! `#[ignore]`d) a single-tile 32x32 image decodes **exactly** (MAD 0.00), and
//! larger ones decode near-perfectly until a region of invalid BC7 blocks
//! (byte 0 with no valid mode bit; no real encoder emits one,
//! `crates/texture/examples/gnf_tile_row_byte_check.rs`). That is the
//! PSARC-level missing/garbage-content population of `docs/formats/psarc.md`'s
//! "Block data location", not a wrong tile order; see `docs/formats/gnf.md`'s
//! "Tiling".
//!
//! So [`Texture::decode`] scans the base level's block grid for that signature and
//! refuses the whole surface with [`Error::CorruptBlocks`]. A census of the front
//! end's sprite sheet (`crates/texture/examples/gnf_frontend_census.rs`) finds
//! many single-mip images clean and others (mostly multi-mip, with per-level tile
//! padding) well into double digits. `SurfaceFormat::Bc1`/`Bc3` decode only
//! through the linear path; no sampled `.gnf` ships either under `TileMode(13)`.

use std::fmt;

/// The four bytes a `.gnf` starts with.
pub const MAGIC: [u8; 4] = *b"GNF ";

/// Bytes before the contents struct.
pub const HEADER_LEN: usize = 8;

/// Bytes per texture descriptor (8 dwords plus the metadata-offset dword).
pub const DESCRIPTOR_LEN: usize = 36;

/// What went wrong reading a `.gnf`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// Fewer bytes than the structure being read needs.
    TooShort {
        /// Bytes required.
        need: usize,
        /// Bytes supplied.
        got: usize,
    },
    /// The first four bytes are not `GNF `.
    BadMagic {
        /// The four bytes found.
        found: [u8; 4],
    },
    /// A tiled surface with no address formula: every [`TileMode`] but linear and
    /// `Thin_1DThin` (0x0d).
    Tiled {
        /// The declared [`TileMode`] index.
        tile_mode: u8,
    },
    /// A [`SurfaceFormat`] [`Texture::decode`] has no block decoder for.
    UnsupportedFormat {
        /// The format found.
        format: SurfaceFormat,
    },
    /// Pixel data ran past the end of the blob.
    DataOutOfBounds {
        /// Bytes the decode needed.
        need: usize,
        /// Bytes supplied.
        got: usize,
    },
    /// The base level has a BC7 block whose byte 0 carries no valid mode bit: the
    /// missing/garbage-content population of `docs/formats/psarc.md`'s "Block
    /// data location", not a wrong tile order. [`Texture::decode`] refuses the
    /// whole surface.
    CorruptBlocks {
        /// Blocks in the base level's own grid with no valid mode bit.
        count: usize,
    },
    /// The bytes after the header are not exactly one surface's tile-padded mip
    /// chain as [`Texture::block_levels`] reads it (a cubemap, array, volume or
    /// trailing bytes). Refused, since the first surface alone would be wrong.
    ChainLayout {
        /// Bytes one 2D chain occupies.
        expected: usize,
        /// Bytes the file holds past its header.
        found: usize,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooShort { need, got } => write!(f, "GNF needs {need} bytes, got {got}"),
            Self::BadMagic { found } => write!(f, "not a GNF: magic {found:02x?}"),
            Self::Tiled { tile_mode } => write!(
                f,
                "TileMode({tile_mode}) has no address formula here - decode() only untiles a linear surface or Thin_1DThin (13), see docs/formats/gnf.md"
            ),
            Self::UnsupportedFormat { format } => {
                write!(f, "no block decoder for {format:?}")
            }
            Self::DataOutOfBounds { need, got } => {
                write!(f, "pixel data needs {need} bytes, got {got}")
            }
            Self::ChainLayout { expected, found } => write!(
                f,
                "the file holds {found} bytes past its header but one 2D mip chain is {expected}"
            ),
            Self::CorruptBlocks { count } => write!(
                f,
                "{count} block(s) in the base level have no valid BC7 mode bit - refusing rather than decoding around missing bytes, see docs/formats/gnf.md"
            ),
        }
    }
}

impl std::error::Error for Error {}

/// Shorthand for this module's results.
pub type Result<T> = std::result::Result<T, Error>;

/// The GCN surface format: AMD's "Sea Islands Series Instruction Set
/// Architecture" manual, Table 8.13, `DataFormat`. Only values seen or plausible
/// on a PS4 disc are named; the rest are [`SurfaceFormat::Other`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SurfaceFormat {
    /// Four 8-bit channels.
    Format8_8_8_8,
    /// BC1 (S3TC DXT1-shaped) block-compressed surface.
    Bc1,
    /// BC2 block-compressed surface.
    Bc2,
    /// BC3 (S3TC DXT5-shaped) block-compressed surface.
    Bc3,
    /// BC4 block-compressed surface.
    Bc4,
    /// BC5 block-compressed surface.
    Bc5,
    /// BC6H block-compressed surface.
    Bc6,
    /// BC7 block-compressed surface.
    Bc7,
    /// A code this module does not name, kept as the raw 6-bit value.
    Other(u8),
}

impl SurfaceFormat {
    fn from_bits(bits: u32) -> Self {
        match bits {
            0x0a => Self::Format8_8_8_8,
            0x23 => Self::Bc1,
            0x24 => Self::Bc2,
            0x25 => Self::Bc3,
            0x26 => Self::Bc4,
            0x27 => Self::Bc5,
            0x28 => Self::Bc6,
            0x29 => Self::Bc7,
            other => Self::Other(other as u8),
        }
    }

    /// Whether this is one of the three BC formats [`crate::bcn`] can decode;
    /// necessary, not sufficient (see [`Texture::is_linear`]).
    #[must_use]
    pub fn is_bcn(self) -> bool {
        matches!(self, Self::Bc1 | Self::Bc3 | Self::Bc7)
    }
}

/// How a stored value is interpreted as a pixel - AMD's `NumberFormat`,
/// Table 8.13. Only the values this module has seen are named.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ChannelType {
    /// Unsigned, normalized to `[0, 1]`.
    Unorm,
    /// Unsigned, normalized and gamma-encoded.
    Srgb,
    /// A code this module does not name, kept as the raw 4-bit value.
    Other(u8),
}

impl ChannelType {
    fn from_bits(bits: u32) -> Self {
        match bits {
            0x0 => Self::Unorm,
            0x9 => Self::Srgb,
            other => Self::Other(other as u8),
        }
    }
}

/// How pixels are ordered in GPU memory: GFD-Studio's `TileMode`, an index into
/// the PS4 SDK's 32-entry table (pipe config, micro/macro tile mode, bank swizzle).
/// This keeps the raw index and names only the two linear entries and
/// `Thin_1DThin`, since the rest need the whole table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileMode(pub u8);

impl TileMode {
    /// `Display_LinearAligned`: row-major, CPU-readable.
    const LINEAR_ALIGNED: u8 = 0x08;
    /// `Display_LinearGeneral`: the same, the SDK's "do not use" mode.
    const LINEAR_GENERAL: u8 = 0x1f;
    /// `Thin_1DThin`: micro-tiled only, no banks/pipes. Every sampled `.gnf`
    /// declares it; see `docs/formats/gnf.md`'s "Tiling" section.
    pub(crate) const THIN_1D_THIN: u8 = 0x0d;

    /// Whether this mode is one of the two linear (untiled) ones.
    #[must_use]
    pub fn is_linear(self) -> bool {
        matches!(self.0, Self::LINEAR_ALIGNED | Self::LINEAR_GENERAL)
    }
}

/// The header and one texture descriptor.
///
/// Only single-texture files are read: every sampled `.gnf` declares
/// `texture_count == 1`, as GFD-Studio's reader assumes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Texture {
    /// Always 2 on every file seen.
    pub version: u8,
    /// Always 8 on every file seen.
    pub alignment: u8,
    /// Total file length as declared; not trusted for slicing, see [`Texture::parse`].
    pub stream_size: u32,
    /// The GCN surface (pixel) format.
    pub surface_format: SurfaceFormat,
    /// How a stored value maps to a pixel value.
    pub channel_type: ChannelType,
    /// Texel width.
    pub width: u32,
    /// Texel height.
    pub height: u32,
    /// Depth (1 for a 2D texture).
    pub depth: u32,
    /// Bytes of one tiled row at the base mip level; meaningless on a linear surface.
    pub pitch: u32,
    /// First mip level this descriptor covers.
    pub base_mip_level: u8,
    /// Last mip level this descriptor covers - `last - base + 1` levels.
    pub last_mip_level: u8,
    /// First array slice this descriptor covers.
    pub base_array_slice: u16,
    /// Last array slice this descriptor covers.
    pub last_array_slice: u16,
    /// Whether dimensions are padded to the next power of two internally.
    pub is_pow2_pad: bool,
    /// Memory layout - see [`TileMode`].
    pub tile_mode: TileMode,
    /// Byte offset of this texture's pixel data, from the start of the file.
    pub data_offset: usize,
}

impl Texture {
    /// Reads the header, contents and first descriptor out of a `.gnf` (or its
    /// first `+0x100`-ish bytes); pixel data is located, not read.
    ///
    /// # Errors
    ///
    /// [`Error::TooShort`] or [`Error::BadMagic`].
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < HEADER_LEN {
            return Err(Error::TooShort {
                need: HEADER_LEN,
                got: data.len(),
            });
        }
        let magic: [u8; 4] = data[0..4].try_into().expect("four bytes");
        if magic != MAGIC {
            return Err(Error::BadMagic { found: magic });
        }
        let contents_size = u32_le(data, 0x04) as usize;
        let data_offset = HEADER_LEN + contents_size;

        let contents_end = HEADER_LEN + 8;
        if data.len() < contents_end + DESCRIPTOR_LEN {
            return Err(Error::TooShort {
                need: contents_end + DESCRIPTOR_LEN,
                got: data.len(),
            });
        }
        let version = data[HEADER_LEN];
        let alignment = data[HEADER_LEN + 2];
        let stream_size = u32_le(data, HEADER_LEN + 4);

        let at = contents_end;
        let word1 = u32_le(data, at + 0x04);
        let word2 = u32_le(data, at + 0x08);
        let word3 = u32_le(data, at + 0x0c);
        let word4 = u32_le(data, at + 0x10);
        let word5 = u32_le(data, at + 0x14);

        Ok(Self {
            version,
            alignment,
            stream_size,
            surface_format: SurfaceFormat::from_bits((word1 & 0x03f0_0000) >> 20),
            channel_type: ChannelType::from_bits((word1 & 0x3c00_0000) >> 26),
            width: (word2 & 0x3fff) + 1,
            height: ((word2 >> 14) & 0x3fff) + 1,
            depth: (word4 & 0x1fff) + 1,
            pitch: ((word4 >> 13) & 0x3fff) + 1,
            base_mip_level: ((word3 >> 12) & 0xf) as u8,
            last_mip_level: ((word3 >> 16) & 0xf) as u8,
            base_array_slice: (word5 & 0x1fff) as u16,
            last_array_slice: ((word5 >> 13) & 0x1fff) as u16,
            is_pow2_pad: (word3 >> 25) & 1 != 0,
            tile_mode: TileMode(((word3 >> 20) & 0x1f) as u8),
            data_offset,
        })
    }

    /// Whether the [`TileMode`] is one of the two linear ones; with
    /// [`SurfaceFormat::is_bcn`], the condition for pixel bytes to already be in
    /// the row-major order [`crate::bcn`] expects.
    #[must_use]
    pub fn is_linear(&self) -> bool {
        self.tile_mode.is_linear()
    }

    /// Decodes this texture's base level to straight RGBA8.
    ///
    /// `blob` must be the bytes [`Texture::parse`] was given.
    ///
    /// # Errors
    ///
    /// [`Error::Tiled`] for any tile mode other than linear or `Thin_1DThin`,
    /// [`Error::UnsupportedFormat`] for a [`SurfaceFormat`] with no block decoder,
    /// [`Error::DataOutOfBounds`] for a blob shorter than the level, and
    /// [`Error::CorruptBlocks`] as in "What this does not do".
    pub fn decode(&self, blob: &[u8]) -> Result<Vec<[u8; 4]>> {
        decode::decode(self, blob)
    }
}

impl Texture {
    /// The BC7 blocks of every mip level, untiled into row-major order and
    /// still compressed - what a GPU with block compression takes as it is.
    ///
    /// One entry per level, base first, each `ceil(w/4) * ceil(h/4)` blocks of 16
    /// bytes. **Unlike [`Self::decode`] this refuses a corrupt block in any
    /// level**: a chain with a hole would sample garbage at distance.
    ///
    /// # Errors
    ///
    /// [`Error::Tiled`] for any tile mode but `Thin_1DThin`,
    /// [`Error::UnsupportedFormat`] for anything but BC7,
    /// [`Error::DataOutOfBounds`] for a blob shorter than its chain, and
    /// [`Error::CorruptBlocks`] as above.
    pub fn block_levels(&self, blob: &[u8]) -> Result<Vec<Vec<u8>>> {
        decode::block_levels(self, blob)
    }
}

/// Decodes one level of [`Texture::block_levels`] to RGBA8 texels.
///
/// For a device without BC support and for anything that wants to read the
/// texels. `None` for a level shorter than `width` x `height` implies.
#[must_use]
pub fn decode_bc7_level(blocks: &[u8], width: u32, height: u32) -> Option<Vec<[u8; 4]>> {
    decode::bc7_level(blocks, width, height)
}

fn u32_le(data: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]])
}

mod decode;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod oracle_tests;

#[cfg(test)]
mod search_tests;

#[cfg(test)]
mod micro_tile_tests;

#[cfg(test)]
mod chain_tests;
