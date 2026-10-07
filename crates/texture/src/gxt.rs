//! The Vita's texture container (`.gxt`).
//!
//! ```text
//! +0x00  u32   tag                  "GXT\0"
//! +0x04  u32   version              0x10000003 on every file measured
//! +0x08  u32   texture count
//! +0x0c  u32   texel data offset, from the start of the file
//! +0x10  u32   texel data length
//! +0x14  u32   16-entry (P4) palette count
//! +0x18  u32   256-entry (P8) palette count
//! +0x1c  u32   padding
//! then `count` descriptors, 32 bytes each:
//! +0x00  u32   texel data offset, from the start of the file
//! +0x04  u32   texel data length
//! +0x08  u32   palette index, or 0xffffffff for none
//! +0x0c  u32   flags
//! +0x10  u32   type
//! +0x14  u32   format               `SceGxmTextureBaseFormat`, top byte only
//! +0x18  u16   width
//! +0x1a  u16   height
//! +0x1c  u8    mip levels           at least 1
//! +0x1d  u8[3] padding
//! ```
//!
//! Everything is **little-endian** (see `docs/formats/2048-status.md`), unlike
//! the big-endian [`crate::gtf`]. There is no `pitch` field: a mip chain is
//! tightly packed.
//!
//! # The field layout is Sony's own `SceGxtHeader`/`SceGxtTextureInfo`
//!
//! Corroborated against the public `vitasdk` headers (`psp2/gxt.h`) and
//! re-derived from the bytes: on `Data\XML\2048_hud\Texture\hud_2048.gxt`
//! (524,352 bytes), `dataOffset` (0x40) + `dataSize` (0x80000) is the file
//! length exactly, and the descriptor repeats the same two numbers. Checked
//! across the nine `.gxt` files `oag_2048::hud::ART` reaches, not the whole disc.
//!
//! # `format` names a Sony `SceGxmTextureBaseFormat`, and this title uses six
//!
//! The low three bytes carry swizzle/channel-order bits this module does not
//! act on; only the top byte ([`Format::from_byte`]) is read. All six decode,
//! across the whole corpus sweep (all 9,910 `.gxt` files):
//!
//! | Format byte | `SceGxmTextureBaseFormat` | Where |
//! | --- | --- | --- |
//! | `0x86` | `UBC2` (BC2/`DXT23`) | every `2048_hud\` texture - the played skin |
//! | `0x83` | `PVRTII4BPP` | the bare-root HUD skin, and every front-end atlas measured (`data/FE/NewImages/canvasTexture*.gxt`) |
//! | `0x85` | `UBC1` (BC1/`DXT1`) | |
//! | `0x87` | `UBC3` (BC3/`DXT5`) | |
//! | `0x0c` | `ARGB8888` | |
//! | `0x98` | `U8U8U8` | |
//!
//! `UBC2` decodes through [`crate::bcn::dxt23`], the same BC2 block math
//! [`crate::gtf`] uses. `PVRTII4BPP` is PowerVR texture compression, not a
//! per-block palette codec - see [`crate::pvrtc`] for the decode and its
//! confidence. A format byte outside this table refuses with
//! [`Error::Unsupported`]. [`oag_2048::hud::LAYOUTS`] reads the `2048_hud` skin
//! exclusively (`docs/formats/2048-hud.md`), so every race HUD texture is `UBC2`.
//!
//! [`oag_2048::hud::LAYOUTS`]: https://github.com/topaxi/OpenAntiGrav/blob/main/crates/2048/src/hud.rs

use crate::bcn;
use crate::pvrtc;

/// Bytes of file header before the first descriptor.
pub const HEADER_LEN: usize = 32;

/// Bytes of one texture descriptor.
pub const DESCRIPTOR_LEN: usize = 32;

/// `"GXT\0"`, little-endian as a `u32`.
const MAGIC: u32 = 0x0054_5847;

/// Most textures one file may declare; same reasoning as
/// [`crate::gtf::MAX_TEXTURES`].
pub const MAX_TEXTURES: u32 = 64;

/// Most mip levels a descriptor may declare, as [`crate::gtf::MAX_MIP_LEVELS`].
pub const MAX_MIP_LEVELS: u8 = 16;

/// Fewest bytes any one mip level occupies.
///
/// **Measured.** A `PVRTII4BPP` level with a single 4x4 word would be 8 bytes
/// on the plain arithmetic and is stored as 16; levels with two words or more
/// match the plain figure. Over all three EU packages the plain arithmetic
/// closes on 2,923 of 10,204 `PVRTII4BPP` textures and this floor on **10,204
/// of 10,204**. Free for `UBC2`, whose one block is already 16 bytes.
pub const MIN_LEVEL_LEN: usize = 16;

/// What went wrong reading a `.gxt`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// Fewer bytes than the file header needs.
    TooShort {
        /// Bytes supplied.
        got: usize,
    },
    /// The magic tag is not `"GXT\0"`.
    BadMagic {
        /// The four bytes found, as a `u32`.
        tag: u32,
    },
    /// The texture count is implausible, or the descriptors run off the end.
    BadTextureCount {
        /// The count found.
        count: u32,
    },
    /// The header's `dataOffset`/`dataSize` disagree with where the descriptor
    /// table ends and the file's length.
    ///
    /// Every `.gxt` measured has `dataOffset == HEADER_LEN + count *
    /// DESCRIPTOR_LEN` and `dataOffset + dataSize == file length`.
    BadHeaderExtent {
        /// `dataOffset` the header declares.
        offset: u32,
        /// `dataSize` the header declares.
        size: u32,
    },
    /// The mip count is implausible.
    BadMipCount {
        /// The count found.
        mip_levels: u8,
    },
    /// Width or height was zero.
    ZeroSized {
        /// Width found.
        width: u16,
        /// Height found.
        height: u16,
    },
    /// A descriptor's texel range is not inside the blob.
    DataOutOfBounds {
        /// Offset the descriptor declares.
        offset: u32,
        /// Length it declares.
        length: u32,
        /// Bytes supplied.
        got: usize,
    },
    /// The declared texel length is not what the descriptor implies.
    ///
    /// The strongest signal a blob is not a `.gxt`, as
    /// [`crate::gtf::Error::ChainLengthMismatch`].
    ChainLengthMismatch {
        /// Length the descriptor implies.
        expected: usize,
        /// Length it declares.
        declared: u32,
    },
    /// A format byte this module does not decode.
    ///
    /// Not a parse failure: the descriptor is read either way, so a caller can
    /// still report width, height and format.
    Unsupported {
        /// The whole format `u32`.
        format: u32,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooShort { got } => write!(f, "need at least {HEADER_LEN} bytes, got {got}"),
            Self::BadMagic { tag } => write!(f, "bad magic 0x{tag:08x}, expected \"GXT\\0\""),
            Self::BadTextureCount { count } => {
                write!(f, "{count} textures (expected 1 to {MAX_TEXTURES})")
            }
            Self::BadMipCount { mip_levels } => {
                write!(
                    f,
                    "{mip_levels} mip levels (expected 1 to {MAX_MIP_LEVELS})"
                )
            }
            Self::ZeroSized { width, height } => write!(f, "zero-sized: {width}x{height}"),
            Self::DataOutOfBounds {
                offset,
                length,
                got,
            } => write!(f, "texels at {offset}+{length} run past {got} bytes"),
            Self::ChainLengthMismatch { expected, declared } => {
                write!(
                    f,
                    "declared {declared} texel bytes, the descriptor implies {expected}"
                )
            }
            Self::BadHeaderExtent { offset, size } => {
                write!(
                    f,
                    "header declares data at {offset}+{size}, which does not land where the \
                     descriptor table ends and the file itself does"
                )
            }
            Self::Unsupported { format } => {
                write!(f, "format 0x{format:08x} is not decoded")
            }
        }
    }
}

impl std::error::Error for Error {}

/// Result alias for this module.
pub type Result<T> = std::result::Result<T, Error>;

/// The texel format this module decodes, from the top byte of the
/// descriptor's `format` field (`SceGxmTextureBaseFormat`).
///
/// All six format bytes this title's corpus carries. Any other byte parses (see
/// [`Texture::format_byte`]) but refuses [`Texture::to_rgba`] with
/// [`Error::Unsupported`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    /// `0x85`, BC1: 8 bytes per 4x4 block, no alpha channel.
    Ubc1,
    /// `0x86`, BC2: 16 bytes per 4x4 block, four bits of alpha per texel.
    Ubc2,
    /// `0x87`, BC3: 16 bytes per 4x4 block, an interpolated alpha ramp.
    Ubc3,
    /// `0x83`, PVRTC-II at 4 bits per texel: 8 bytes per 4x4 **word**, not a
    /// block codec - see `crate::pvrtc`.
    Pvrtii4bpp,
    /// `0x0c`, `SceGxmTextureSwizzle4Mode::ARGB`: four raw bytes a texel - see
    /// [`argb8888`].
    Argb8888,
    /// `0x98`, `U8U8U8`: three raw bytes a texel, tightly packed; channel order
    /// is chosen, see [`rgb888`].
    Rgb888,
}

impl Format {
    /// The format the top byte of `format` names, or `None` for a byte this
    /// module does not decode.
    fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0x0c => Some(Self::Argb8888),
            0x83 => Some(Self::Pvrtii4bpp),
            0x85 => Some(Self::Ubc1),
            0x86 => Some(Self::Ubc2),
            0x87 => Some(Self::Ubc3),
            0x98 => Some(Self::Rgb888),
            _ => None,
        }
    }

    /// Bytes one 4x4 block or word occupies. Meaningless for
    /// [`Self::Argb8888`]/[`Self::Rgb888`] (no block grid); [`Texture::level_len`]
    /// branches around both.
    const fn unit_len(self) -> usize {
        match self {
            Self::Ubc1 => 8,
            Self::Ubc2 | Self::Ubc3 => 16,
            Self::Pvrtii4bpp => pvrtc::WORD_LEN,
            Self::Argb8888 => 4,
            Self::Rgb888 => 3,
        }
    }
}

/// One texture descriptor, and where its texels are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Texture {
    /// The whole `format` field, unshifted. [`Self::format`] is the top byte
    /// of this, when it names a format [`Format`] decodes.
    pub format_byte: u32,
    /// Mip levels stored, at least 1.
    pub mip_levels: u8,
    /// Width of the base level, in texels.
    pub width: u16,
    /// Height of the base level, in texels.
    pub height: u16,
    /// Where the texels are in the blob this came out of.
    pub data: std::ops::Range<usize>,
}

impl Texture {
    /// The decoded format, or `None` for a format byte [`Format`] does not
    /// name.
    #[must_use]
    pub fn format(&self) -> Option<Format> {
        Format::from_byte((self.format_byte >> 24) as u8)
    }

    /// Width and height of one mip level, each floored at 1.
    #[must_use]
    pub fn level_size(&self, level: u8) -> (u32, u32) {
        let shift = u32::from(level);
        (
            (u32::from(self.width) >> shift).max(1),
            (u32::from(self.height) >> shift).max(1),
        )
    }

    /// Bytes one mip level occupies, for a format [`Self::format`] names.
    ///
    /// `None` for a format this module does not decode: the block size is
    /// unknowable, so [`Gxt::parse`]'s length check only runs on decodable
    /// textures and [`Error::Unsupported`] is a decode-time error.
    ///
    /// [`Format::Argb8888`] and [`Format::Rgb888`] have no minimum storage unit
    /// to floor at [`MIN_LEVEL_LEN`]. Measured: all 99 `0x0c` textures in the
    /// base package (nine `(width, height, mip count)` shapes down to a single
    /// 4x4 level) match `width * height * 4` with nothing floored, and all 13
    /// `0x98` textures (one 512x64 level each) match `width * height * 3`.
    fn level_len(&self, level: u8) -> Option<usize> {
        let format = self.format()?;
        let (width, height) = self.level_size(level);
        if format == Format::Argb8888 || format == Format::Rgb888 {
            return Some(width as usize * height as usize * format.unit_len());
        }
        let across = (width as usize).div_ceil(4);
        let down = (height as usize).div_ceil(4);
        Some((across * down * format.unit_len()).max(MIN_LEVEL_LEN))
    }

    /// Bytes the whole mip chain occupies, tightly packed (no `pitch` field).
    fn chain_len(&self) -> Option<usize> {
        (0..self.mip_levels)
            .map(|level| self.level_len(level))
            .sum()
    }

    /// Byte range of one mip level, inside the blob.
    fn level_range(&self, level: u8) -> Option<std::ops::Range<usize>> {
        let start = self.data.start
            + (0..level)
                .map(|l| self.level_len(l))
                .sum::<Option<usize>>()?;
        Some(start..start + self.level_len(level)?)
    }

    /// Decodes the base mip level to straight RGBA8.
    ///
    /// # Errors
    ///
    /// [`Error::Unsupported`] for a format [`Format`] does not name, and
    /// [`Error::DataOutOfBounds`] if the level does not fit the blob (the wrong
    /// blob, since `Gxt::parse` checked).
    pub fn to_rgba(&self, blob: &[u8]) -> Result<Vec<[u8; 4]>> {
        let format = self.format().ok_or(Error::Unsupported {
            format: self.format_byte,
        })?;
        let range = self.level_range(0).ok_or(Error::Unsupported {
            format: self.format_byte,
        })?;
        let texels = blob.get(range.clone()).ok_or(Error::DataOutOfBounds {
            offset: range.start as u32,
            length: (range.end - range.start) as u32,
            got: blob.len(),
        })?;
        let (width, height) = self.level_size(0);
        let decoded = match format {
            Format::Ubc1 => blocks(texels, width, height, 8, |b| {
                bcn::dxt1(b.try_into().expect("8-byte block"))
            }),
            Format::Ubc2 => blocks(texels, width, height, 16, |b| {
                bcn::dxt23(b.try_into().expect("16-byte block"))
            }),
            Format::Ubc3 => blocks(texels, width, height, 16, |b| {
                bcn::dxt45(b.try_into().expect("16-byte block"))
            }),
            // Not a block walk: the word grid's Morton order is applied inside
            // the codec. See [`crate::pvrtc`].
            Format::Pvrtii4bpp => pvrtc::decode_ii_4bpp(texels, width, height),
            Format::Argb8888 => argb8888(texels, width, height),
            Format::Rgb888 => rgb888(texels, width, height),
        };
        decoded.ok_or(Error::DataOutOfBounds {
            offset: range.start as u32,
            length: (range.end - range.start) as u32,
            got: blob.len(),
        })
    }
}

/// Walks the block grid in **twiddled (Morton/Z-order) order**, not raster
/// order, and expands each block.
///
/// # The block grid is tiled (measured)
///
/// A raster read of `Data\XML\2048_hud\Texture\missile_reticule.gxt` (256x256,
/// one `UBC2` level) is noise with one clean band; Morton order over the *block*
/// coordinates (`bx` odd bits, `by` even, via [`twiddle`]) is a clean reticle
/// atlas, and matches `ClassiCube`'s Vita `TwiddleCalcFactors`. No linear bit
/// exists to read (`+0x10`, `type`, was 0 on every `2048_hud` texture).
///
/// **Scope**: measured on this one square texture; `blocks` applies [`twiddle`]'s
/// non-square algorithm to every BC-family texture, including `hud_2048.gxt`'s
/// 1024x512. `UBC1` ([`bcn::dxt1`]) and `UBC3` ([`bcn::dxt45`]) reuse the walk, as
/// the order belongs to the block *grid*: `docs/formats/gxt.md`, "`UBC1`/`UBC3`
/// decode too".
fn blocks(
    texels: &[u8],
    width: u32,
    height: u32,
    unit: usize,
    decode: impl Fn(&[u8]) -> [[u8; 4]; 16],
) -> Option<Vec<[u8; 4]>> {
    let pixels = (width as usize).checked_mul(height as usize)?;
    let mut out = vec![[0u8; 4]; pixels];
    let across = (width as usize).div_ceil(4) as u32;
    let down = (height as usize).div_ceil(4) as u32;

    for by in 0..down {
        for bx in 0..across {
            let index = twiddle(bx, by, across, down) as usize;
            let at = index * unit;
            let block = texels.get(at..at + unit)?;
            let texels16 = decode(block);
            for (i, texel) in texels16.into_iter().enumerate() {
                let x = bx as usize * 4 + i % 4;
                let y = by as usize * 4 + i / 4;
                // A block on the right or bottom edge of a non-multiple-of-four
                // texture carries texels the image does not have.
                if x < width as usize && y < height as usize {
                    out[y * width as usize + x] = texel;
                }
            }
        }
    }
    Some(out)
}

/// Walks the **texel** grid in twiddled order and reads each texel's four raw
/// bytes as `A, R, G, B` (`SceGxmTextureSwizzle4Mode::ARGB`, from `format`'s low
/// three bytes).
///
/// [`twiddle`] runs over texel coordinates `(x, y)` here, not the block
/// coordinates [`blocks`] uses.
///
/// # Measured on 2048's Zone/Detonator speed-class art
///
/// `data/Tex/zoneModeTrack{0,7,14}.gxt` (256x256, `0x0c001000`): raster order is
/// banded noise, twiddled order decodes cleanly. Every texel's `A`, `R` and `G`
/// agree (65,536 texels, zero mismatches), a stencil mask opaque on exactly 2,048
/// texels on all three stages, while `B` carries a smooth gradient. The mask's
/// *shape* escalates (solid at stage 0, increasingly dashed by 7 and 14): see
/// `the_zone_track_art_decodes_to_a_shape_that_escalates_across_stages` in
/// `gxt_ground_truth.rs` and [docs/formats/gxt.md](../../../docs/formats/gxt.md),
/// which also covers the ARGB channel order and the HD `.gtf` copy of the art.
fn argb8888(texels: &[u8], width: u32, height: u32) -> Option<Vec<[u8; 4]>> {
    let pixels = (width as usize).checked_mul(height as usize)?;
    let mut out = vec![[0u8; 4]; pixels];
    for y in 0..height {
        for x in 0..width {
            let index = twiddle(x, y, width, height) as usize;
            let at = index.checked_mul(4)?;
            let texel = texels.get(at..at + 4)?;
            let (a, r, g, b) = (texel[0], texel[1], texel[2], texel[3]);
            out[y as usize * width as usize + x as usize] = [r, g, b, a];
        }
    }
    Some(out)
}

/// Walks the texel grid in twiddled order and reads each texel's three raw
/// bytes as `R, G, B`, opaque.
///
/// `0x98`, `U8U8U8`. `width * height * 3` matches the declared texel length on
/// all 13 shipped textures, so this is 3 tightly packed bytes a texel.
///
/// **The tiling order is measured (confidence 70); the channel order is
/// chosen, not measured, and carries no score.** All 13 files are
/// `Data\FE\NewImages\scepresents\scee_presents_<language>.gxt` (512x64);
/// raster order decodes to noise and twiddled order to a legible "Sony
/// Computer Entertainment presents". But **every texel in all 13 files has
/// `max(byte) - min(byte) == 0`**: the art is pure grayscale, so any byte
/// permutation decodes to the same picture and nothing can prefer one reading.
/// `R, G, B` in file order follows [`argb8888`]'s convention (and the vertex
/// formats in `docs/formats/2048-rcsmodel.md`). A `0x98` texture with real
/// colour would be the only thing to check it; none ships in the base package
/// or either DLC pack.
fn rgb888(texels: &[u8], width: u32, height: u32) -> Option<Vec<[u8; 4]>> {
    let pixels = (width as usize).checked_mul(height as usize)?;
    let mut out = vec![[0u8; 4]; pixels];
    for y in 0..height {
        for x in 0..width {
            let index = twiddle(x, y, width, height) as usize;
            let at = index.checked_mul(3)?;
            let texel = texels.get(at..at + 3)?;
            out[y as usize * width as usize + x as usize] = [texel[0], texel[1], texel[2], 255];
        }
    }
    Some(out)
}

/// The Morton/Z-order block index `(bx, by)` reads from, over a grid `across`
/// blocks wide and `down` blocks tall.
///
/// Bits of `bx` and `by` interleave one at a time (`by` even, `bx` odd) while
/// both dimensions have steps left; once one is exhausted the other's remaining
/// bits are appended linearly. That makes it correct for a non-square grid like
/// `hud_2048.gxt`'s 256x128 blocks. Same algorithm as `ClassiCube`'s Vita port's
/// `TwiddleCalcFactors`, as a direct bit-scatter.
pub(crate) fn twiddle(bx: u32, by: u32, across: u32, down: u32) -> u32 {
    let (mut w, mut h) = (across, down);
    let mut mask_x = 0u32;
    let mut mask_y = 0u32;
    let mut shift = 0u32;
    while w > 1 || h > 1 {
        if w > 1 && h > 1 {
            mask_y |= 1 << shift;
            mask_x |= 1 << (shift + 1);
            shift += 2;
        } else if w > 1 {
            mask_x |= 1 << shift;
            shift += 1;
        } else {
            mask_y |= 1 << shift;
            shift += 1;
        }
        w >>= 1;
        h >>= 1;
    }
    scatter(bx, mask_x) | scatter(by, mask_y)
}

/// Deposits the low bits of `value`, one at a time, into the set bit
/// positions of `mask`, lowest bit of `value` to the lowest set bit of
/// `mask`. The scalar equivalent of the `pdep` instruction.
fn scatter(value: u32, mask: u32) -> u32 {
    let mut out = 0u32;
    let mut remaining = mask;
    let mut source = value;
    while remaining != 0 {
        let lowest = remaining.isolate_lowest_one();
        if source & 1 != 0 {
            out |= lowest;
        }
        source >>= 1;
        remaining &= !lowest;
    }
    out
}

/// A parsed `.gxt`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gxt {
    /// `+0x04`. `0x10000003` on every file measured.
    pub version: u32,
    /// The descriptors, in file order.
    pub textures: Vec<Texture>,
}

impl Gxt {
    /// Reads a `.gxt`.
    ///
    /// # Errors
    ///
    /// See [`Error`]. A supported texture's declared texel length is checked
    /// against what its dimensions, format and mip count imply, as
    /// [`crate::gtf::Gtf::parse`] does. An unsupported format is **not**
    /// rejected here (its length is unverifiable); [`Texture::to_rgba`] refuses it.
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < HEADER_LEN {
            return Err(Error::TooShort { got: data.len() });
        }
        let tag = le32(data, 0);
        if tag != MAGIC {
            return Err(Error::BadMagic { tag });
        }
        let version = le32(data, 4);
        let count = le32(data, 8);
        let descriptors_end =
            HEADER_LEN.saturating_add(DESCRIPTOR_LEN.saturating_mul(count as usize));
        if count == 0 || count > MAX_TEXTURES || descriptors_end > data.len() {
            return Err(Error::BadTextureCount { count });
        }

        // The header's texel span: `dataOffset` right after the descriptor
        // table, `dataOffset + dataSize` the file length. Every file measured agrees.
        let header_offset = le32(data, 0x0c);
        let header_size = le32(data, 0x10);
        let header_end = (header_offset as usize).checked_add(header_size as usize);
        if header_offset as usize != descriptors_end || header_end != Some(data.len()) {
            return Err(Error::BadHeaderExtent {
                offset: header_offset,
                size: header_size,
            });
        }

        let textures = (0..count as usize)
            .map(|index| Texture::parse(data, HEADER_LEN + index * DESCRIPTOR_LEN))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self { version, textures })
    }

    /// The one texture a Wipeout 2048 `.gxt` carries.
    ///
    /// Every HUD texture measured declares exactly one, as [`crate::gtf::Gtf::only`].
    #[must_use]
    pub fn only(&self) -> Option<&Texture> {
        self.textures.first()
    }
}

impl Texture {
    /// Reads one descriptor at `at`, which the caller has bounds-checked.
    fn parse(data: &[u8], at: usize) -> Result<Self> {
        let mip_levels = data[at + 0x1c];
        if mip_levels == 0 || mip_levels > MAX_MIP_LEVELS {
            return Err(Error::BadMipCount { mip_levels });
        }
        let width = le16(data, at + 0x18);
        let height = le16(data, at + 0x1a);
        if width == 0 || height == 0 {
            return Err(Error::ZeroSized { width, height });
        }

        let offset = le32(data, at);
        let length = le32(data, at + 4);
        let end = (offset as usize).checked_add(length as usize);
        if end.is_none_or(|end| end > data.len()) {
            return Err(Error::DataOutOfBounds {
                offset,
                length,
                got: data.len(),
            });
        }

        let texture = Self {
            format_byte: le32(data, at + 0x14),
            mip_levels,
            width,
            height,
            data: offset as usize..offset as usize + length as usize,
        };

        // Only checkable for a format this module knows the block size of -
        // see `Texture::chain_len`.
        if let Some(expected) = texture.chain_len()
            && expected != length as usize
        {
            return Err(Error::ChainLengthMismatch {
                expected,
                declared: length,
            });
        }
        Ok(texture)
    }
}

fn le16(data: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([data[at], data[at + 1]])
}

fn le32(data: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]])
}

/// Byte coverage of one `.gxt` blob.
///
/// `Gxt::parse` already checks the header's texel span against the descriptor
/// table's end and the file length, so every parsed file closes exactly with no
/// slack. This makes the claim explicit and per-texture, the shape
/// `oag_rcs::rcsmodel::coverage` uses.
#[must_use]
pub fn coverage(data: &[u8]) -> oag_formats::coverage::Coverage {
    let mut seen = oag_formats::coverage::Coverage::new(data.len());
    let Ok(gxt) = Gxt::parse(data) else {
        return seen;
    };
    seen.claim(0, HEADER_LEN, "the gxt header");
    seen.claim(
        HEADER_LEN,
        gxt.textures.len() * DESCRIPTOR_LEN,
        "the descriptor table",
    );
    for texture in &gxt.textures {
        seen.claim(texture.data.start, texture.data.len(), "texel data");
    }
    seen
}

#[cfg(test)]
mod tests;
