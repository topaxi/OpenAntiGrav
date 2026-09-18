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
//! Everything is **little-endian**, matching the rest of this title's package
//! (see `docs/formats/2048-status.md`) - unlike [`crate::gtf`], which is
//! big-endian throughout because it is a PS3 file. There is no `pitch` field:
//! a mip chain is tightly packed, one level's block rows immediately followed
//! by the next's.
//!
//! # The field layout is Sony's own `SceGxtHeader`/`SceGxtTextureInfo`
//!
//! Corroborated against the public `vitasdk`/`vita-toolchain` headers
//! (`psp2/gxt.h`), the same kind of external check [`crate::gtf`]'s own header
//! rests on for `CellGcmTexture`. Every field this module reads was also
//! independently re-derived from the bytes: on
//! `Data\XML\2048_hud\Texture\hud_2048.gxt` (524,352 bytes),
//! `dataOffset` (0x40) + `dataSize` (0x80000) is the file length exactly, and
//! the descriptor's own `dataOffset`/`dataSize` repeat the same two numbers -
//! the same "header states its own length and gets it right" invariant
//! [`crate::gtf`] closes on, checked here across the nine `.gxt` files
//! `oag_2048::hud::ART` actually reaches rather than the whole disc, which
//! this project has not swept.
//!
//! # `format` names a Sony `SceGxmTextureBaseFormat`, and this title uses six
//!
//! The low three bytes carry swizzle/channel-order bits this module does not
//! act on; only the top byte - [`Format::from_byte`] - is read. All six
//! formats below decode; this project's own corpus sweep
//! (`texture(gxt): UBC1, UBC3 and U8U8U8 decode; all 9,910 .gxt files now
//! decode`) is what the "all decode" claim rests on, not a narrower survey:
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
//! [`crate::gtf`] uses - the block layout is a hardware standard, not
//! something either console's container defines. `PVRTII4BPP` is PowerVR
//! texture compression, a different codec family entirely (bilinear-upscaled
//! low-frequency and high-frequency images plus a modulation layer, not a
//! per-4x4-block palette) - see [`crate::pvrtc`] for the decode and its own
//! confidence. A format byte outside this table still refuses rather than
//! guesses - see
//! [`Error::Unsupported`]. This costs nothing today: [`oag_2048::hud::LAYOUTS`]
//! reads the `2048_hud` skin exclusively (see `docs/formats/2048-hud.md`), so
//! every texture a real race's HUD reaches is `UBC2`.
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

/// Most textures one file may declare. Same reasoning as
/// [`crate::gtf::MAX_TEXTURES`]: keeps a garbage count from making the header
/// arithmetic accept an arbitrary blob.
pub const MAX_TEXTURES: u32 = 64;

/// Most mip levels a descriptor may declare, on the same terms as
/// [`crate::gtf::MAX_MIP_LEVELS`].
pub const MAX_MIP_LEVELS: u8 = 16;

/// Fewest bytes any one mip level occupies, whatever its own dimensions
/// imply.
///
/// **Measured, not assumed, and it is the whole difference between reading
/// this container's `PVRTII4BPP` textures and rejecting them.** A `PVRTII4BPP`
/// level whose block grid is a single 4x4 word would be 8 bytes on the plain
/// arithmetic, and is stored as 16; every level with two words or more is
/// stored at exactly the plain figure. Swept over all three EU packages: the
/// plain arithmetic closes on 2,923 of 10,204 `PVRTII4BPP` textures and this
/// floor closes on **10,204 of 10,204**, with the shortfall being exactly one
/// word on exactly those chains that bottom out at a single-word level. Free
/// for `UBC2`, whose one block is already 16 bytes.
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
    /// The file header's own `dataOffset`/`dataSize` disagree with where the
    /// descriptor table actually ends and how long the file actually is.
    ///
    /// Every `.gxt` measured has `dataOffset == HEADER_LEN + count *
    /// DESCRIPTOR_LEN` and `dataOffset + dataSize == the file's own length`,
    /// so this is the file-header half of the same "the header states its own
    /// length and gets it right" invariant [`Error::ChainLengthMismatch`]
    /// checks per descriptor.
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
    /// The strongest signal that a blob is not a `.gxt` at all, on the same
    /// terms as [`crate::gtf::Error::ChainLengthMismatch`].
    ChainLengthMismatch {
        /// Length the descriptor implies.
        expected: usize,
        /// Length it declares.
        declared: u32,
    },
    /// A format byte this module does not decode - `PVRTC`, a palette format,
    /// or anything else outside [`Format`].
    ///
    /// Not a parse failure: the descriptor is read either way, so a caller can
    /// still report width, height and format for a texture this cannot turn
    /// into pixels, the way [`crate::gtf::Error::Swizzled`] does for the RSX's
    /// Morton order.
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
/// All six format bytes this title's corpus carries, as of the `UBC1`/`UBC3`/
/// `U8U8U8` decodes that closed out the last three. A format byte outside
/// them parses (see [`Texture::format_byte`]) but refuses
/// [`Texture::to_rgba`] with [`Error::Unsupported`] rather than guessing at a
/// decode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    /// `0x85`, BC1: 8 bytes per 4x4 block, no alpha channel.
    Ubc1,
    /// `0x86`, BC2: 16 bytes per 4x4 block, four bits of alpha per texel.
    Ubc2,
    /// `0x87`, BC3: 16 bytes per 4x4 block, an interpolated alpha ramp.
    Ubc3,
    /// `0x83`, PVRTC-II at 4 bits per texel: 8 bytes per 4x4 **word**, and
    /// not a block codec at all - see `oag_formats::pvrtc`.
    Pvrtii4bpp,
    /// `0x0c`, `SceGxmTextureSwizzle4Mode::ARGB`: four raw bytes a texel, no
    /// block or bit-packing at all - see [`argb8888`].
    Argb8888,
    /// `0x98`, `U8U8U8`: three raw bytes a texel, tightly packed (measured -
    /// see [`rgb888`]'s module doc for why the channel order is read as
    /// R/G/B rather than independently confirmed).
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

    /// Bytes one 4x4 block or word occupies. Not meaningful for
    /// [`Self::Argb8888`]/[`Self::Rgb888`], which have no block grid - see
    /// [`Texture::level_len`], the only caller, which branches around both.
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
    /// `None` for a format this module does not decode, since the block size
    /// (and so the chain length) is not knowable without one. That means
    /// [`Gxt::parse`]'s length-agreement check only runs on textures this
    /// module can also decode; an unsupported format's declared length is
    /// trusted rather than verified, which is why [`Error::Unsupported`] is a
    /// decode-time error rather than a parse one.
    ///
    /// [`Format::Argb8888`] and [`Format::Rgb888`] have no block grid to
    /// floor at [`MIN_LEVEL_LEN`] - unlike the compressed formats, neither is
    /// quantised to a minimum storage unit larger than one texel. Measured
    /// directly rather than assumed: all 99 `0x0c` textures in the base
    /// package's `.gxt` corpus, nine distinct `(width, height, mip count)`
    /// shapes down to a single 4x4 level, agree with the plain
    /// `width * height * 4` formula with zero floored, and all 13 `0x98`
    /// textures (one 512x64 level each) agree with `width * height * 3` to
    /// the byte - so there is no evidence a floor applies to either the way
    /// there is for `PVRTII4BPP`'s own single-word minimum.
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

    /// Bytes the whole mip chain occupies, tightly packed - there is no
    /// `pitch` field on this container, unlike [`crate::gtf::Texture`].
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
    /// [`Error::DataOutOfBounds`] if the level does not fit the blob -
    /// `Gxt::parse` has already checked this for a supported format, so it
    /// means the wrong blob was passed.
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
            // Not a block walk: every texel reads four words, and the word
            // grid's own Morton order is applied inside the codec rather
            // than here. See [`crate::pvrtc`].
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
/// order, and expands each BC2 block.
///
/// # The block grid is tiled, and this is measured rather than assumed
///
/// A naive raster read of `Data\XML\2048_hud\Texture\missile_reticule.gxt`
/// (256x256, one `UBC2` level) decodes to noise with an odd clean horizontal
/// band - the signature of a swizzle that happens to agree with raster order
/// along one axis. Reading the same bytes in Morton order over the *block*
/// coordinates - `bx`/`by` bit-interleaved via [`twiddle`], `bx` in the odd
/// positions and `by` in the even ones - decodes to a clean, recognisable
/// lock-on reticle atlas: a dashed bracket ring, a filled circle backdrop, a
/// thin dashed ring and a crosshair with a dashed arc. Confirmed against the
/// public PowerVR/GXM twiddle algorithm independently: `ClassiCube`'s own
/// Vita port computes the same even/odd bit-interleave masks
/// (`TwiddleCalcFactors`) to write `sceGxmTextureInitSwizzled` textures, so
/// this is Sony's own documented hardware tiling scheme rather than a
/// guess that happened to render something.
///
/// **Why raster order was the first thing tried, and not obviously wrong on
/// its own terms**: [`crate::gtf`]'s own `.gtf` container carries an
/// equivalent `is_linear` bit that states whether a texture is raster or
/// swizzled, and every real texture on that disc is linear. `.gxt`'s
/// descriptor carries no such bit this module reads (`+0x10`, `type`, was 0
/// on every `2048_hud` texture measured, including this one) - so unlike
/// `.gtf`, nothing in the file states the layout, and it had to be settled by
/// decoding both ways and looking at the picture, the same method
/// [`crate::gtf::decode`] uses to settle its own endianness question.
///
/// **Scope**: measured on this one 256x256 square texture. `blocks` applies
/// [`twiddle`]'s general (non-square) algorithm to every BC-family texture
/// this module decodes, including `hud_2048.gxt`'s 1024x512 - untwiddled
/// visually on that file specifically, since it draws sprite art rather than
/// one recognisable shape, but consistent with the same rule.
///
/// `UBC1` (BC1, 8-byte blocks via [`bcn::dxt1`]) and `UBC3` (BC3, 16-byte
/// blocks via [`bcn::dxt45`]) reuse this same walk with their own `unit` and
/// decode function - the twiddle order is a property of the block *grid*,
/// not of what each block decodes to, and nothing in the corroboration above
/// (the reticle atlas, `ClassiCube`'s `TwiddleCalcFactors`) is specific to
/// BC2's own bit layout. See `docs/formats/gxt.md`'s "`UBC1`/`UBC3` decode
/// too" section for why this generalisation is measured rather than assumed.
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

/// Walks the **texel** grid in twiddled (Morton/Z-order) order and reads each
/// texel's four raw bytes as `A, R, G, B` (`SceGxmTextureSwizzle4Mode::ARGB`,
/// the swizzle bits `format`'s low three bytes already carried - see the
/// module docs).
///
/// `Format::Argb8888` carries no block structure the way `UBC2` does, so
/// [`twiddle`]'s general grid algorithm is applied directly over texel
/// coordinates `(x, y)` here, rather than over block coordinates `(bx, by)`
/// as [`blocks`] does - one texel is this format's own smallest unit.
///
/// # Measured on 2048's Zone/Detonator speed-class art, not assumed
///
/// `data/Tex/zoneModeTrack{0,7,14}.gxt` (256x256, `0x0c001000`) is the
/// corpus [docs/formats/gxt.md](../../../docs/formats/gxt.md) left this
/// format's tiling order open on. Raster order decodes to horizontal-banded
/// noise - the same "a swizzle that happens to agree with raster order along
/// one axis" signature `blocks`'s own doc records for `UBC2`'s reticle
/// texture. Twiddled order decodes cleanly instead: every texel's `A`, `R`
/// and `G` bytes agree exactly (measured across all 65,536 texels, zero
/// mismatches) - a binary stencil mask, opaque on exactly 2,048 of them, the
/// same count on all three sampled stages - while `B` alone carries a smooth
/// multi-valued gradient. The mask's *shape* is what escalates, not its
/// area: composited over a checkerboard it reads as a thin horizontal band,
/// solid at stage 0 and increasingly dashed by stage 7 and 14 - see
/// `the_zone_track_art_decodes_to_a_shape_that_escalates_across_stages` in
/// `gxt_ground_truth.rs`, whose per-stage renders are what this rests on. Not
/// the same finding as Wipeout HD's own already-decoded `.gtf` copy of this
/// art (`docs/formats/gxt.md`'s "Track" texture-set section) - that is a
/// different container, a different codec, and was read long before this
/// decoder existed; the two agree only in role, both being the varying half
/// of a matched flat/varying pair, not in exact picture.
///
/// Channel order is confirmed independently of the shape: `A`/`R`/`G`'s
/// lockstep binary mask versus `B`'s continuous gradient is consistent with
/// the ARGB swizzle [`docs/formats/gxt.md`](../../../docs/formats/gxt.md)
/// already corroborated against the public `vitasdk` headers by
/// bit-decomposing `format`, now also corroborated by which channel of the
/// decode turns out to carry continuous art versus a flat mask.
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
/// `0x98`, `U8U8U8`, the last format byte this corpus carries that was still
/// refused. The length arithmetic settles tiling granularity and packing on
/// its own terms - `plain3` (`width * height * 3`) matches the declared texel
/// length exactly on all 13 shipped textures, so this is 3 tightly-packed
/// bytes a texel, not 4-byte-padded the way [`Format::Argb8888`] is.
///
/// **The tiling order is measured (confidence 70), the channel order is
/// chosen, not measured, and carries no confidence score of its own.** All
/// 13 files are `Data\FE\NewImages\scepresents\scee_presents_<language>.gxt`
/// (512x64), a first-party splash reused per language; raster order decodes
/// to noise and twiddled order decodes to a crisp, legible "Sony Computer
/// Entertainment presents" - the same signature `argb8888`'s own doc records
/// for the Zone/Detonator art. But **every texel in every one of the 13
/// files has `max(byte) - min(byte) == 0`** (measured directly, zero
/// exceptions): the art is pure grayscale, so `R`/`G`/`B`/any permutation of
/// the three bytes decodes to the identical picture, and nothing in this
/// corpus can prefer one reading over another. Read as `R, G, B` in file
/// order for consistency with [`argb8888`]'s own byte-order convention (and
/// with how this corpus's vertex formats lay out positions and normals - see
/// `docs/formats/2048-rcsmodel.md`), which is a **pick**, not a decode
/// backed by a colour sample - none exists to check against, so this is
/// disclosed as chosen rather than presented as verified. A future `0x98`
/// texture with actual colour, if one ever ships, is the only thing that
/// could turn this into a checked claim; nothing in the base package or
/// either DLC pack does.
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
/// Bits of `bx` and `by` interleave one at a time - `by` even, `bx` odd -
/// while both dimensions still have more than one step left; once one
/// dimension is exhausted, the remaining bits of the other are appended
/// linearly rather than interleaved with nothing. That second half is what
/// makes this correct for a non-square grid like `hud_2048.gxt`'s 256x128
/// blocks, where the naive "interleave every bit" reading `missile_reticule
/// .gxt`'s 64x64 grid was measured against would run out of `by` bits eight
/// steps before it ran out of `bx` ones. Same algorithm as `ClassiCube`'s
/// Vita port's `TwiddleCalcFactors`, reimplemented as a direct bit-scatter
/// rather than a mask-and-subtract loop.
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
        let lowest = remaining & remaining.wrapping_neg();
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
    /// against what its width, height, format and mip count imply, the same
    /// way [`crate::gtf::Gtf::parse`] checks a `.gtf`'s. A texture in a format
    /// this module does not decode is **not** rejected here - its length is
    /// unverifiable without knowing the block size - so parsing an
    /// unsupported texture succeeds and [`Texture::to_rgba`] is what refuses
    /// it.
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

        // The file header states its own texel span too - `dataOffset` right
        // after the descriptor table, `dataOffset + dataSize` the file's own
        // length - and every file measured agrees with both halves. Checked
        // here rather than only in the module docs' worked example, so the
        // claim is true of the parser and not just of one file's arithmetic.
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
    /// Every HUD texture measured declares exactly one, on the same terms
    /// [`crate::gtf::Gtf::only`] states for HD.
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
/// `Gxt::parse` already checks the file header's own texel span against the
/// descriptor table's end and against the file's own length (see
/// `Gxt::parse`'s doc comment), so this container is expected to close
/// exactly for every file it parses at all - there is no slack between a
/// texture's own `data` range and the next one's, the descriptor table, or
/// the header, because `header_end == data.len()` is already enforced. What
/// this adds beyond that check is making the claim explicit and per-texture,
/// the same "claim what the parser already knows" shape
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
