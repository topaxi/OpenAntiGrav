//! The PS4's texture container (`.gnf`) - header and descriptor only.
//!
//! **Confidence: 80.** A public Sony format (the name is a genuine SDK
//! acronym, "Gnm Format") with no first-party spec this project holds, so
//! the layout below is triangulated from two independent open-source
//! implementations rather than read off an SDK header - the AMD GCN "T#"
//! image descriptor it embeds is drawn from a public AMD document. Not
//! confirmed against a running PS4, hence 80 rather than the 90s a
//! byte-identical Sony spec would earn: [`docs/formats/psarc.md`](../../../docs/formats/psarc.md)'s
//! own PSARC page draws the same "publicly documented, not recovered here"
//! distinction for exactly this reason.
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
//! Little-endian throughout - the one PS4 container this project reads that
//! is not big-endian, since nothing upstream of it (PS3/Vita/PSP/PS2) is a
//! little-endian target. See `docs/formats/README.md`'s Omega Collection
//! rows.
//!
//! # Where this layout comes from
//!
//! Three independently-maintained, non-affiliated open-source projects agree
//! on it, checked directly against this project's own extracted `.gnf`
//! files rather than trusted on citation alone (see [`tests`]):
//!
//! 1. [GFD-Studio](https://github.com/tge-was-taken/GFD-Studio)'s
//!    `GFDLibrary.Textures.GNF.GNFTexture` - a from-scratch reader/writer
//!    used by Fallout 4/Skyrim SE PS4 mods and `image2gnf` - gives the exact
//!    byte offsets and bitfield widths quoted above, verified here against
//!    `Data/art/published/hdships/harimau/Livery2/Holographic_02_GLOW.gnf`
//!    (a real, non-zero `omega-ps4-eu` entry - `docs/formats/psarc.md`'s
//!    "Block data location" section): its `word1` decodes to
//!    `SurfaceFormat::BC7`/`ChannelType::Srgb` and its `word2` decodes to
//!    128x64, both plausible for a livery decal, and its `word3` decodes to
//!    a standard RGBA channel order (4,5,6,7) and tile mode index 13 -
//!    `TileMode::Thin_1DThin` (GFD-Studio's own `TileMode.cs` enum,
//!    fetched and re-checked directly: index 13 is `Thin_1DThin`,
//!    micro-tiled only; index **14** is `Thin_2DThin`, macro-tiled - an
//!    earlier pass here had this off by one), which is why this module
//!    stops at identification (see "What this does not do", below).
//! 2. The [PlayStation GNF Image page](https://rewiki.miraheze.org/wiki/PlayStation_GNF_Image)
//!    on the reverse-engineering wiki gives the same header/contents split
//!    independently, naming the same three fixed values (version 2,
//!    alignment 8) this project's own sample also carries.
//! 3. [shadPS4](https://github.com/shadps4-emu/shadPS4)'s
//!    `AmdGpu::Image` (`src/video_core/amdgpu/resource.h`) is the same
//!    32-byte, 8-dword descriptor one layer down - the raw GCN "T#" texture
//!    resource descriptor AMD's own public "Sea Islands Series Instruction
//!    Set Architecture" reference manual documents (Table 8.13) - and its
//!    `DataFormat`/`NumberFormat` enums have the identical numeric values as
//!    GFD-Studio's `SurfaceFormat`/`ChannelType` (`BC7 = 0x29` on both,
//!    `Srgb = 9` on both), which is what makes two unrelated projects'
//!    readings the same fact rather than two guesses that happen to agree on
//!    a name.
//!
//! # What this does not do
//!
//! **[`Texture::decode`] untiles a linear surface unconditionally, and a
//! micro-tiled (`Thin_1DThin`) BC7 surface only when its base level carries
//! no corrupt block.** AMD's `ComputeSurfaceAddrFromCoordMicroTiled` (Mesa's
//! MIT `addrlib`, `egbaddrlib.cpp`) is a simple row-major-tiles formula with
//! no banks, pipes or row-size unknowns - unlike the macro-tiled formula the
//! earlier, disproven pass reached for. Checked against real oracle-paired
//! textures (`crates/texture/src/gnf/micro_tile_tests.rs`, `#[ignore]`d): a
//! single-micro-tile 32x32 image decodes **exactly** (MAD 0.00), and on
//! larger multi-tile images the first several on-disk tiles decode
//! near-perfectly before a region that used to read as an unexplained
//! periodic corruption. **That region is now explained**: a byte-level scan
//! of the same ship-livery oracle pair
//! (`crates/texture/examples/gnf_tile_row_byte_check.rs`) finds it dense
//! with BC7 blocks whose byte 0 carries no valid mode bit - a pattern a real
//! encoder never produces, and the same PSARC-level missing/garbage-content
//! population `docs/formats/psarc.md`'s "Block data location" section
//! already documents family-wide, landing on this specific ship texture's
//! own copy rather than on a wrong tile order. See `docs/formats/gnf.md`'s
//! "Tiling" section for the full evidence trail.
//!
//! So [`Texture::decode`] ships the row-major micro-tile formula for real
//! use, guarded rather than open-ended: it scans the base level's own block
//! grid for that same invalid-mode signature first, and refuses the whole
//! surface with [`Error::CorruptBlocks`] the moment it finds one, rather
//! than decoding around missing bytes into a picture with silent garbage
//! patches. A census of the front end's own sprite sheet
//! (`crates/texture/examples/gnf_frontend_census.rs`) finds a wide spread -
//! many single-mip images clean at 0%, others (mostly multi-mip ones, where
//! a per-level tile-alignment pad is expected past the base level this
//! module never reads) well into double digits - so this guard is doing
//! real, title-wide work, not gating on one bad file. `SurfaceFormat::Bc1`/
//! `Bc3` still decode only through the linear path; no real `.gnf` this
//! project has sampled ships either format under `TileMode(13)`.

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
    /// A tiled surface [`Texture::decode`] has no address formula for at
    /// all - every [`TileMode`] except a linear one and `Thin_1DThin`
    /// (0x0d). See this module's own "What this does not do".
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
    /// The base level's own micro-tile grid has at least one BC7 block
    /// whose byte 0 carries no valid mode bit - the same PSARC-level
    /// missing/garbage-content population `docs/formats/psarc.md`'s "Block
    /// data location" section documents family-wide, not a wrong tile
    /// order. [`Texture::decode`] refuses the whole surface rather than
    /// decoding around it - see this module's own "What this does not do".
    CorruptBlocks {
        /// Blocks in the base level's own grid with no valid mode bit.
        count: usize,
    },
    /// The bytes after the header are not exactly one surface's mip chain in
    /// the tile-padded layout [`Texture::block_levels`] reads - a cubemap, an
    /// array or a volume, or a file with trailing bytes. Refused rather than
    /// read as its first surface, which would be a wrong picture.
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

/// The GCN surface format a texture's pixels are stored in - AMD's public
/// "Sea Islands Series Instruction Set Architecture" reference manual, Table
/// 8.13, `DataFormat`. Only the values seen or plausible on a PS4 game disc
/// are named; an unrecognised value is kept as [`SurfaceFormat::Other`]
/// rather than refused, since this module's job is identification, not
/// validation.
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

    /// Whether this is one of the three BC formats [`crate::bcn`] can
    /// decode - the necessary condition for pixel decoding, not sufficient:
    /// the surface also has to be untiled, which [`Texture::is_linear`]
    /// checks separately.
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

/// How a texture's pixels are ordered in GPU memory - GFD-Studio's
/// `TileMode`, itself an index into a fixed 32-entry table the PS4 SDK
/// defines (pipe config, micro/macro tile mode, bank swizzle per entry).
/// This module keeps the raw index and only distinguishes the two entries
/// that mean "not tiled at all", because untiling every other entry needs
/// that whole table - see this module's own "What this does not do".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileMode(pub u8);

impl TileMode {
    /// `Display_LinearAligned` - a surface stored row-major, CPU-readable
    /// with no GPU-side untiling step.
    const LINEAR_ALIGNED: u8 = 0x08;
    /// `Display_LinearGeneral` - the same, at the SDK's own "hugely
    /// inefficient, do not use" mode.
    const LINEAR_GENERAL: u8 = 0x1f;
    /// `Thin_1DThin` - micro-tiled only, no banks/pipes. Every real `.gnf`
    /// this project has sampled declares this one; see `decode`'s own doc
    /// comment and `docs/formats/gnf.md`'s "Tiling" section.
    pub(crate) const THIN_1D_THIN: u8 = 0x0d;

    /// Whether this mode is one of the two linear (untiled) ones.
    #[must_use]
    pub fn is_linear(self) -> bool {
        matches!(self.0, Self::LINEAR_ALIGNED | Self::LINEAR_GENERAL)
    }
}

/// The header and one texture descriptor.
///
/// Only single-texture files are read - every real `.gnf` this project has
/// sampled declares `texture_count == 1`, the same restriction GFD-Studio's
/// own reader carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Texture {
    /// Always 2 on every file seen.
    pub version: u8,
    /// Always 8 on every file seen.
    pub alignment: u8,
    /// Total file length, as declared - not trusted for slicing; see
    /// [`Texture::parse`].
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
    /// Bytes of one tiled row at the base mip level, if tiled; meaningless
    /// on a linear surface's own terms but still decoded for completeness.
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
    /// Reads the header, contents and first texture descriptor out of a
    /// whole `.gnf` file (or at least its first `+0x100`-ish bytes - the
    /// pixel data itself is not read here, only located).
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

    /// Whether this texture's own [`TileMode`] is one of the two linear
    /// ones - the necessary-and-sufficient condition (together with
    /// [`SurfaceFormat::is_bcn`]) for this module's pixel bytes to already
    /// be in the row-major order [`crate::bcn`] expects.
    #[must_use]
    pub fn is_linear(&self) -> bool {
        self.tile_mode.is_linear()
    }

    /// Decodes this texture's base level to straight RGBA8.
    ///
    /// `blob` must be the bytes [`Texture::parse`] was given, since
    /// [`Texture::data_offset`] indexes into it.
    ///
    /// # Errors
    ///
    /// [`Error::Tiled`] for every real `.gnf` this project has sampled
    /// (see this module's own "What this does not do") - only a linear
    /// [`TileMode`] reaches the decoder at all. [`Error::UnsupportedFormat`]
    /// for a [`SurfaceFormat`] with no block decoder here, and
    /// [`Error::DataOutOfBounds`] for a blob shorter than the level it
    /// declares.
    pub fn decode(&self, blob: &[u8]) -> Result<Vec<[u8; 4]>> {
        decode::decode(self, blob)
    }
}

impl Texture {
    /// The BC7 blocks of every mip level, untiled into row-major order and
    /// still compressed - what a GPU with block compression takes as it is.
    ///
    /// One entry per level, base first, each `ceil(w/4) * ceil(h/4)` blocks of
    /// 16 bytes. **Unlike [`Self::decode`] this refuses a corrupt block in
    /// any level, not only the base**: a decoded picture can draw from its
    /// base alone, but a chain with a hole in it would sample garbage at
    /// distance.
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
