//! The PS3's texture container (`.gtf`).
//!
//! ```text
//! +0x00  u32   version              0x01050000 or 0x02010100
//! +0x04  u32   size of everything after the header
//! +0x08  u32   texture count        1 in all 7,333 files on the HD disc
//! then `count` descriptors, 36 bytes each:
//! +0x00  u32   id
//! +0x04  u32   offset of the texel data, from the start of the file
//! +0x08  u32   length of the texel data
//! +0x0c  u8    format               low 5 bits texel format, 0x20 linear, 0x40 unnormalised
//! +0x0d  u8    mip levels           at least 1
//! +0x0e  u8    dimension            2 in all 7,333
//! +0x0f  u8    cubemap              1 in 23 of 7,333
//! +0x10  u32   remap                a channel permutation; see `remap`
//! +0x14  u16   width
//! +0x16  u16   height
//! +0x18  u16   depth                1 in all 7,333
//! +0x1a  u8    location             0 in all 7,333
//! +0x1b  u8    padding
//! +0x1c  u32   pitch                bytes of one base-level row, or 0
//! +0x20  u32   GPU offset           filled in at load
//! ```
//!
//! Everything is big-endian, this being a PS3 file, and the descriptor is
//! Sony's own `CellGcmTexture` written out - which is why it carries fields a
//! file has no use for, `location` and the GPU `offset` among them.
//!
//! # The whole thing is self-checking, and it closes on the whole disc
//!
//! **Confidence 92.** Two independent arithmetic invariants hold on **7,333 of
//! 7,333** `.gtf` files across all seven of Wipeout HD / Fury's archives:
//!
//! 1. `12 + 36 * count + size` is the file length, or equivalently the texel
//!    data starts at `+0x80` and runs to the end.
//! 2. The declared `length` is exactly what the format, the dimensions, the mip
//!    count and the pitch imply - see [`Texture::chain_len`].
//!
//! Neither can come out right by accident on a file this varied: the corpus runs
//! 3x1 to 2048x2048, one to twelve mip levels, ten distinct format bytes and
//! 131 non-power-of-two textures.
//!
//! 92 rather than higher because **nothing has been compared against the running
//! original**, which is the ceiling this project's rubric puts on a static
//! reading, and because two of the three swizzled formats are refused rather
//! than decoded.
//!
//! # `pitch` does not halve down the mip chain
//!
//! This is the rule that four files turn on, and getting it wrong looks like a
//! corrupt file rather than a wrong reading. When `pitch` is non-zero it is one
//! row of the **base** level, and every further level uses the same pitch rather
//! than half of it. `zone_2/gradienttex_tr01_set01.gtf` is the smallest witness:
//! 3x1 `A8R8G8B8`, pitch 12, two levels, 24 bytes - which is 12 + 12, not
//! 12 + 6. `amphiseum/textures/dds/air_traffic_test_a_atoc.gtf` is the one that
//! makes it unmistakable: 1028x256 `DXT45`, pitch 4112, eleven levels,
//! **538,672 bytes, which is 4,112 x 131** where 131 is the sum of the chain's
//! block-row counts.
//!
//! Those four also show that **a pitch can be declared for a compressed
//! texture**, where it is bytes of one *block* row rather than one pixel row -
//! 4,112 is 257 blocks of 16. A reader that treats pitch as pixels there is out
//! by a factor of four.
//!
//! # What is on the disc
//!
//! | Count | Format byte | Texel format | Decoded |
//! | ---: | --- | --- | :-: |
//! | 4,137 | `0x88` | `DXT45` | yes |
//! | 2,485 | `0x86` | `DXT1` | yes |
//! | 527 | `0x87` | `DXT23` | yes |
//! | 126 | `0xa5` | `A8R8G8B8`, linear | yes |
//! | 37 | `0x85` | `A8R8G8B8`, **swizzled** | no |
//! | 9 | `0x81` | `B8`, swizzled | no |
//! | 7 | `0x9e` | `A8B8G8R8`, swizzled | no |
//! | 3 | `0xa8` | `DXT45`, linear | yes |
//! | 1 | `0xa6` | `DXT1`, linear | yes |
//! | 1 | `0xa7` | `DXT23`, linear | yes |
//!
//! 7,280 of 7,333 decode. The 53 that do not are the ones with **no `0x20`
//! bit** and no block compression, which means the texels are in the RSX's
//! Morton order; that permutation is not implemented, and a linear read of a
//! swizzled texture is a recognisable picture in scrambled tiles, which is
//! exactly the kind of wrong answer that survives review. [`Texture::to_rgba`]
//! refuses them by name instead. Block-compressed formats are never swizzled -
//! the block layout is the tiling - so the `0x20` bit is not consulted for them.
//!
//! # And 23 cubemaps, which are parsed and not decoded
//!
//! `cubemap` is set on 23 files, all of them a `sky` or an environment probe.
//! Six faces follow one another and the length invariant holds on all 23 - with
//! **an unexplained 360 bytes** on the 20 that carry a mip chain, the same 360
//! whether the faces are 128x128 `DXT1` or 2048x2048. Six times sixty, and
//! sixty is not a multiple of any block size here. Nothing reads a cubemap yet,
//! so the slack is recorded in [`Texture::chain_len`] rather than explained
//! away.
//!
//! See `docs/formats/gtf.md` for the evidence, and `docs/formats/hd-hud.md` for
//! what needed this.

use std::ops::Range;

mod decode;

/// Bytes of file header before the first descriptor.
pub const HEADER_LEN: usize = 12;

/// Bytes of one texture descriptor.
pub const DESCRIPTOR_LEN: usize = 36;

/// Bit `0x20` of the format byte: the texels are in raster order.
const LINEAR: u8 = 0x20;

/// Bit `0x40` of the format byte: texture coordinates are in texels.
const UNNORMALISED: u8 = 0x40;

/// Most textures one file may declare.
///
/// Every file on the HD disc declares exactly one. The cap exists so that a
/// garbage count cannot make the header arithmetic accept an arbitrary blob.
pub const MAX_TEXTURES: u32 = 64;

/// Most mip levels a descriptor may declare.
///
/// Twelve is the largest on the disc, on a 2048x2048 sky. Same reasoning as
/// [`MAX_TEXTURES`].
pub const MAX_MIP_LEVELS: u8 = 16;

/// What went wrong reading a `.gtf`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// Fewer bytes than the file header needs.
    TooShort {
        /// Bytes supplied.
        got: usize,
    },
    /// The texture count is implausible, or the descriptors run off the end.
    BadTextureCount {
        /// The count found.
        count: u32,
    },
    /// The mip count is implausible.
    BadMipCount {
        /// The count found.
        mip_levels: u8,
    },
    /// Width, height or depth was zero.
    ZeroSized {
        /// Width found.
        width: u16,
        /// Height found.
        height: u16,
    },
    /// The low five bits of the format byte are not a format this knows.
    UnknownFormat {
        /// The whole format byte.
        format: u8,
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
    /// The strongest signal that a blob is not a `.gtf` at all, the length being
    /// fully determined by the rest of the descriptor. Holds on 7,333 of 7,333.
    ChainLengthMismatch {
        /// Length the descriptor implies.
        expected: usize,
        /// Length it declares.
        declared: u32,
    },
    /// The texels are in the RSX's Morton order, which is not implemented.
    ///
    /// See this module's format table: 53 files on the HD disc, none of them a
    /// HUD or a circuit texture.
    Swizzled {
        /// The whole format byte.
        format: u8,
    },
    /// A cubemap, whose face layout is parsed but not decoded.
    Cubemap,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooShort { got } => write!(f, "need at least {HEADER_LEN} bytes, got {got}"),
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
            Self::UnknownFormat { format } => write!(f, "unknown texel format 0x{format:02x}"),
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
            Self::Swizzled { format } => {
                write!(
                    f,
                    "format 0x{format:02x} is swizzled, which is not implemented"
                )
            }
            Self::Cubemap => write!(f, "cubemap faces are parsed but not decoded"),
        }
    }
}

impl std::error::Error for Error {}

/// Result alias for this module.
pub type Result<T> = std::result::Result<T, Error>;

/// The texel format, from the low five bits of the descriptor's format byte.
///
/// Named as Sony names them, so `A8R8G8B8` is a big-endian `u32` whose top byte
/// is alpha - which on this console means the bytes arrive in the order A, R, G,
/// B. Only the variants the HD disc actually carries are here; a format byte
/// outside them is [`Error::UnknownFormat`] rather than a guess.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    /// `0x01`, one byte per texel.
    B8,
    /// `0x05`, four bytes per texel.
    A8R8G8B8,
    /// `0x06`, BC1: 8 bytes per 4x4 block.
    Dxt1,
    /// `0x07`, BC2: 16 bytes per block, four bits of alpha per texel.
    Dxt23,
    /// `0x08`, BC3: 16 bytes per block, interpolated alpha.
    Dxt45,
    /// `0x1e`, four bytes per texel in the opposite order to [`Self::A8R8G8B8`].
    A8B8G8R8,
}

impl Format {
    /// The format the low five bits of `byte` name.
    fn from_byte(byte: u8) -> Option<Self> {
        Some(match byte & 0x1f {
            0x01 => Self::B8,
            0x05 => Self::A8R8G8B8,
            0x06 => Self::Dxt1,
            0x07 => Self::Dxt23,
            0x08 => Self::Dxt45,
            0x1e => Self::A8B8G8R8,
            _ => return None,
        })
    }

    /// Whether texels come in 4x4 blocks rather than one at a time.
    #[must_use]
    pub const fn is_block_compressed(self) -> bool {
        matches!(self, Self::Dxt1 | Self::Dxt23 | Self::Dxt45)
    }

    /// Bytes one 4x4 block occupies, or bytes one texel occupies.
    #[must_use]
    pub const fn unit_len(self) -> usize {
        match self {
            Self::B8 => 1,
            Self::A8R8G8B8 | Self::A8B8G8R8 => 4,
            Self::Dxt1 => 8,
            Self::Dxt23 | Self::Dxt45 => 16,
        }
    }
}

/// One texture descriptor, and where its texels are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Texture {
    /// The descriptor's own id.
    pub id: u32,
    /// Texel format.
    pub format: Format,
    /// The whole format byte, flag bits included.
    pub format_byte: u8,
    /// Mip levels stored, at least 1.
    pub mip_levels: u8,
    /// `+0x0e`. 2 on every file measured; 1 and 3 would be 1D and 3D.
    pub dimension: u8,
    /// Whether six faces follow one another.
    pub cubemap: bool,
    /// `+0x10`, a channel permutation this does not act on.
    ///
    /// Three distinct values on the HD disc - `0xaae4` on 7,317 of 7,333,
    /// `0xa9ff` on 9 and `0xa9e4` on 7 - and what they select has not been
    /// read, so nothing here consults it. The 16 that differ are all
    /// single-channel or swizzled formats that [`Texture::to_rgba`] refuses
    /// anyway, so the gap costs nothing today and would matter the day `B8` is
    /// decoded.
    pub remap: u32,
    /// Width of the base level, in texels.
    pub width: u16,
    /// Height of the base level, in texels.
    pub height: u16,
    /// `+0x18`. 1 on every file measured.
    pub depth: u16,
    /// `+0x1a`. 0 on every file measured; a memory pool selector at runtime.
    pub location: u8,
    /// Bytes of one base-level row, or 0 when the levels are tightly packed.
    ///
    /// **A row of blocks** for a compressed format. See this module's header for
    /// why it does not halve down the chain.
    pub pitch: u32,
    /// `+0x20`, the GPU address, filled in at load and meaningless in a file.
    pub gpu_offset: u32,
    /// Where the texels are in the blob this came out of.
    pub data: Range<usize>,
}

impl Texture {
    /// Whether the texels are in raster order.
    ///
    /// Block-compressed formats are always readable in block order, so this
    /// reports the `0x20` bit for what it is and [`Self::to_rgba`] only acts on
    /// it for the uncompressed ones.
    #[must_use]
    pub const fn is_linear(&self) -> bool {
        self.format_byte & LINEAR != 0
    }

    /// Whether texture coordinates address texels rather than the unit square.
    #[must_use]
    pub const fn is_unnormalised(&self) -> bool {
        self.format_byte & UNNORMALISED != 0
    }

    /// Faces stored: 6 for a cubemap, 1 otherwise.
    #[must_use]
    pub const fn faces(&self) -> usize {
        if self.cubemap { 6 } else { 1 }
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

    /// Bytes one mip level of one face occupies.
    #[must_use]
    pub fn level_len(&self, level: u8) -> usize {
        let (width, height) = self.level_size(level);
        let rows = if self.format.is_block_compressed() {
            height.div_ceil(4)
        } else {
            height
        };
        let row = if self.pitch == 0 {
            let across = if self.format.is_block_compressed() {
                width.div_ceil(4)
            } else {
                width
            };
            across as usize * self.format.unit_len()
        } else {
            // One row of the *base* level, for every level. See the module docs.
            self.pitch as usize
        };
        row * rows as usize
    }

    /// Bytes the whole texture occupies, every level of every face.
    ///
    /// The 360 bytes a multi-level cubemap carries beyond six of its own faces
    /// are included, because the check this feeds is against the declared
    /// length and that length includes them on all 20. **What they are is not
    /// known**; see the module docs.
    #[must_use]
    pub fn chain_len(&self) -> usize {
        let one_face: usize = (0..self.mip_levels)
            .map(|level| self.level_len(level))
            .sum();
        let slack = usize::from(self.cubemap && self.mip_levels > 1) * CUBEMAP_SLACK;
        one_face * self.faces() + slack
    }

    /// Byte range of one mip level, inside the blob.
    ///
    /// **Non-cubemaps only.** For a cubemap this is level `level` of
    /// whichever face comes first, and that the first face starts at offset
    /// zero is the natural reading rather than a checked one - see this
    /// module's header and the 360 unexplained bytes. [`Self::to_rgba`]
    /// refuses a cubemap, so nothing reaches this with one today.
    #[must_use]
    pub fn level_range(&self, level: u8) -> Range<usize> {
        let start = self.data.start + (0..level).map(|l| self.level_len(l)).sum::<usize>();
        start..start + self.level_len(level)
    }

    /// Byte range of one mip level of one **face**.
    ///
    /// **All of a face's levels are consecutive, then the next face's.** That is
    /// what [`Self::chain_len`] computes and what the length invariant holds on
    /// across all 23 of the disc's cubemaps: `one_face * faces() + slack`. A
    /// caller that walked level-major instead would read face 1's base level as
    /// face 0's second.
    ///
    /// `face` past [`Self::faces`] answers the last face rather than panicking,
    /// which keeps this total; [`Self::face_to_rgba`] range-checks properly.
    #[must_use]
    pub fn face_range(&self, face: usize, level: u8) -> Range<usize> {
        let one_face: usize = (0..self.mip_levels).map(|l| self.level_len(l)).sum();
        let face = face.min(self.faces().saturating_sub(1));
        let start = self.data.start
            + face * one_face
            + (0..level).map(|l| self.level_len(l)).sum::<usize>();
        start..start + self.level_len(level)
    }

    /// Decodes the base level of one face to straight RGBA8.
    ///
    /// # The face order is the RSX's, which is OpenGL's
    ///
    /// `0`..`5` are `+X, -X, +Y, -Y, +Z, -Z`. **That is the published order
    /// rather than something measured here** - `CELL_GCM_TEXTURE_DIMENSION_CUBE`
    /// inherits it, and nothing in a `.gtf` names a face. What it is checked
    /// against is a picture: on a sky the four side faces have to meet at their
    /// vertical edges, and a wrong order or a wrong rotation shows as a seam.
    /// See `docs/formats/gtf.md`.
    ///
    /// # Errors
    ///
    /// [`Error::Cubemap`] for a texture that is *not* one, since a face index is
    /// meaningless there; [`Error::UnknownFormat`] for `B8`, as
    /// [`Self::to_rgba`]; [`Error::Swizzled`] for a layout this does not read;
    /// and [`Error::DataOutOfBounds`] for a face past the six.
    pub fn face_to_rgba(&self, blob: &[u8], face: usize) -> Result<Vec<[u8; 4]>> {
        if !self.cubemap {
            return Err(Error::Cubemap);
        }
        if face >= self.faces() {
            let range = self.face_range(face, 0);
            return Err(Error::DataOutOfBounds {
                offset: range.start as u32,
                length: (range.end - range.start) as u32,
                got: blob.len(),
            });
        }
        if !self.format.is_block_compressed() && !self.is_linear() {
            return Err(Error::Swizzled {
                format: self.format_byte,
            });
        }
        if self.format == Format::B8 {
            return Err(Error::UnknownFormat {
                format: self.format_byte,
            });
        }
        let range = self.face_range(face, 0);
        let texels = blob.get(range.clone()).ok_or(Error::DataOutOfBounds {
            offset: range.start as u32,
            length: (range.end - range.start) as u32,
            got: blob.len(),
        })?;
        let (width, height) = self.level_size(0);
        // **Not `Swizzled`.** The swizzled and `B8` cases are both refused
        // above, so a `None` here is the decoder running out of texels - a
        // truncated blob - and reporting that as "swizzled, not implemented"
        // sent the reader looking for a Morton order that is not the problem.
        // Finding F7 of the 2026-08-18 review, in a codebase that prizes
        // honest errors.
        decode::level(self.format, texels, width, height, self.pitch as usize).ok_or(
            Error::DataOutOfBounds {
                offset: range.start as u32,
                length: (range.end - range.start) as u32,
                got: blob.len(),
            },
        )
    }

    /// Decodes the base mip level to straight RGBA8.
    ///
    /// `blob` must be the bytes [`Gtf::parse`] was given, since [`Self::data`]
    /// indexes into it.
    ///
    /// # Errors
    ///
    /// [`Error::Swizzled`] for a texture in the RSX's Morton order,
    /// [`Error::Cubemap`] for a cubemap, and [`Error::DataOutOfBounds`] if the
    /// level does not fit - which `parse` has already ruled out, so it means the
    /// wrong blob was passed, or a blob whose texels stop short of what the
    /// descriptor declares.
    pub fn to_rgba(&self, blob: &[u8]) -> Result<Vec<[u8; 4]>> {
        if self.cubemap {
            return Err(Error::Cubemap);
        }
        if !self.format.is_block_compressed() && !self.is_linear() {
            return Err(Error::Swizzled {
                format: self.format_byte,
            });
        }
        // `B8` has one channel and every one on the disc is swizzled, so there is
        // nothing to test a reading of - and a *linear* one would fall past the
        // guard above and be reported as swizzled, which would be the wrong
        // reason. No such file ships; named here rather than left to be guessed.
        if self.format == Format::B8 {
            return Err(Error::UnknownFormat {
                format: self.format_byte,
            });
        }
        let range = self.level_range(0);
        let texels = blob.get(range.clone()).ok_or(Error::DataOutOfBounds {
            offset: range.start as u32,
            length: (range.end - range.start) as u32,
            got: blob.len(),
        })?;
        let (width, height) = self.level_size(0);
        // **Not `Swizzled`.** The swizzled and `B8` cases are both refused
        // above, so a `None` here is the decoder running out of texels - a
        // truncated blob - and reporting that as "swizzled, not implemented"
        // sent the reader looking for a Morton order that is not the problem.
        // Finding F7 of the 2026-08-18 review, in a codebase that prizes
        // honest errors.
        decode::level(self.format, texels, width, height, self.pitch as usize).ok_or(
            Error::DataOutOfBounds {
                offset: range.start as u32,
                length: (range.end - range.start) as u32,
                got: blob.len(),
            },
        )
    }
}

/// The unexplained tail on a cubemap that carries a mip chain.
///
/// The same 360 bytes on all 20 of them, whatever the face size or format. See
/// the module docs.
const CUBEMAP_SLACK: usize = 360;

/// A parsed `.gtf`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gtf {
    /// `+0x00`. `0x01050000` on 6,980 of the HD disc's files and `0x02010100`
    /// on 353; nothing observed differs between them.
    pub version: u32,
    /// The descriptors, in file order.
    pub textures: Vec<Texture>,
}

impl Gtf {
    /// Reads a `.gtf`.
    ///
    /// # Errors
    ///
    /// See [`Error`]. Every check is arithmetic the file determines, so a blob
    /// that is not a `.gtf` fails one of them rather than decoding to noise.
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < HEADER_LEN {
            return Err(Error::TooShort { got: data.len() });
        }
        let version = be32(data, 0);
        let count = be32(data, 8);
        // Saturating rather than wrapping: a garbage count must fail the bounds
        // test below, not wrap round it.
        let descriptors_end =
            HEADER_LEN.saturating_add(DESCRIPTOR_LEN.saturating_mul(count as usize));
        if count == 0 || count > MAX_TEXTURES || descriptors_end > data.len() {
            return Err(Error::BadTextureCount { count });
        }

        let textures = (0..count as usize)
            .map(|index| Texture::parse(data, HEADER_LEN + index * DESCRIPTOR_LEN))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self { version, textures })
    }

    /// The one texture a Wipeout HD `.gtf` carries.
    ///
    /// Every file on that disc declares exactly one, so this is what a caller
    /// wants and the plural is what a caller has to handle. `None` only for a
    /// file that declares none, which `parse` already refuses.
    #[must_use]
    pub fn only(&self) -> Option<&Texture> {
        self.textures.first()
    }
}

impl Texture {
    /// Reads one descriptor at `at`, which the caller has bounds-checked.
    fn parse(data: &[u8], at: usize) -> Result<Self> {
        let format_byte = data[at + 12];
        let format = Format::from_byte(format_byte).ok_or(Error::UnknownFormat {
            format: format_byte,
        })?;
        let mip_levels = data[at + 13];
        if mip_levels == 0 || mip_levels > MAX_MIP_LEVELS {
            return Err(Error::BadMipCount { mip_levels });
        }
        let width = be16(data, at + 20);
        let height = be16(data, at + 22);
        if width == 0 || height == 0 {
            return Err(Error::ZeroSized { width, height });
        }

        let offset = be32(data, at + 4);
        let length = be32(data, at + 8);
        let end = (offset as usize).checked_add(length as usize);
        if end.is_none_or(|end| end > data.len()) {
            return Err(Error::DataOutOfBounds {
                offset,
                length,
                got: data.len(),
            });
        }

        let texture = Self {
            id: be32(data, at),
            format,
            format_byte,
            mip_levels,
            dimension: data[at + 14],
            cubemap: data[at + 15] != 0,
            remap: be32(data, at + 16),
            width,
            height,
            depth: be16(data, at + 24),
            location: data[at + 26],
            pitch: be32(data, at + 28),
            gpu_offset: be32(data, at + 32),
            data: offset as usize..offset as usize + length as usize,
        };

        let expected = texture.chain_len();
        if expected != length as usize {
            return Err(Error::ChainLengthMismatch {
                expected,
                declared: length,
            });
        }
        Ok(texture)
    }
}

fn be16(data: &[u8], at: usize) -> u16 {
    u16::from_be_bytes([data[at], data[at + 1]])
}

fn be32(data: &[u8], at: usize) -> u32 {
    u32::from_be_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]])
}

#[cfg(test)]
mod tests;
