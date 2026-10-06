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
//! +0x10  u32   remap                per-channel source and force; see `Remap`
//! +0x14  u16   width
//! +0x16  u16   height
//! +0x18  u16   depth                1 in all 7,333
//! +0x1a  u8    location             0 in all 7,333
//! +0x1b  u8    padding
//! +0x1c  u32   pitch                bytes of one base-level row, or 0
//! +0x20  u32   GPU offset           filled in at load
//! ```
//!
//! Everything is big-endian, and the descriptor is Sony's own `CellGcmTexture`
//! written out, hence `location` and the GPU `offset`.
//!
//! # Self-checking: closes on the whole disc
//!
//! **Confidence 92.** Two arithmetic invariants hold on **7,333 of 7,333**
//! `.gtf` files across all seven Wipeout HD / Fury archives:
//!
//! 1. `12 + 36 * count + size` is the file length.
//! 2. The declared `length` is what the format, dimensions, mip count and pitch
//!    imply - see [`Texture::chain_len`].
//!
//! The corpus runs 3x1 to 2048x2048, one to twelve mips, ten format bytes and
//! 131 non-power-of-two textures. 92 rather than higher because nothing has
//! been compared against the running original.
//!
//! # `pitch` does not halve down the mip chain
//!
//! When `pitch` is non-zero it is one row of the **base** level, and every
//! further level uses the same pitch. Getting it wrong looks like a corrupt
//! file. `zone_2/gradienttex_tr01_set01.gtf`: 3x1 `A8R8G8B8`, pitch 12, two
//! levels, 24 bytes (12 + 12, not 12 + 6).
//! `amphiseum/textures/dds/air_traffic_test_a_atoc.gtf`: 1028x256 `DXT45`, pitch
//! 4112, eleven levels, **538,672 bytes = 4,112 x 131**, 131 being the sum of
//! the chain's block-row counts.
//!
//! Those four files also show **a pitch can be declared for a compressed
//! texture**, where it is bytes of one *block* row (4,112 = 257 blocks of 16).
//!
//! # What is on the disc
//!
//! | Count | Format byte | Texel format | Decoded |
//! | ---: | --- | --- | :-: |
//! | 4,137 | `0x88` | `DXT45` | yes |
//! | 2,485 | `0x86` | `DXT1` | yes |
//! | 527 | `0x87` | `DXT23` | yes |
//! | 126 | `0xa5` | `A8R8G8B8`, linear | yes |
//! | 37 | `0x85` | `A8R8G8B8`, **swizzled** | **yes** |
//! | 9 | `0x81` | `B8`, **swizzled** | **yes** |
//! | 7 | `0x9e` | `A8B8G8R8`, **swizzled** | **yes** |
//! | 3 | `0xa8` | `DXT45`, linear | yes |
//! | 1 | `0xa6` | `DXT1`, linear | yes |
//! | 1 | `0xa7` | `DXT23`, linear | yes |
//!
//! **All 7,333 decode.**
//!
//! The 9 `B8` files are each a ship's `textures/ambient_shadow.gtf`, 128x64.
//! Read in Morton order each is that team's craft in silhouette; the stored
//! byte is the shadow's coverage, and the descriptor's own [`Remap`] broadcasts
//! it to four channels. See
//! `gtf::tests::a_single_channel_texture_is_broadcast_by_its_own_remap`.
//!
//! **The 44 swizzled `A8R8G8B8`/`A8B8G8R8` decode through
//! [`decode::morton_index`]**, the RSX's Morton-order texel address (no `0x20`
//! bit, no block compression). A linear read of one is a recognisable picture
//! in scrambled tiles, the kind of wrong answer that survives review. The
//! address function is the platform's documented `cellGcm` tiling, not reversed
//! from this disc - **confidence 88**: an exact match on a 4x4 synthetic
//! fixture (`gtf::tests::a_swizzled_texture_reads_the_rsx_z_order_not_raster_order`),
//! and **all 43 judgeable swizzled files** on disc decode smoother than a
//! deliberately wrong linear misread, measured on both axes because a raster
//! misread of a tiled surface comes out as row-uniform stripes that a
//! within-row metric scores as smooth. Numbers: `docs/formats/gtf.md` and
//! `crates/texture/tests/gtf_ground_truth.rs`. Not corroborated against the
//! executable: `EBOOT.elf` carries no `swizzle` string and no upload routine
//! has been located. Block-compressed formats are never swizzled, so the `0x20`
//! bit is not consulted for them.
//!
//! # 23 cubemaps, decoded face by face
//!
//! `cubemap` is set on 23 files, all a `sky` or an environment probe. Six faces
//! follow one another, face-major, decoded by [`Texture::face_to_rgba`] and
//! drawn as a race's sky by `oag_mesh::mesh::sky_cube`. The length invariant
//! holds on all 23, with **an unexplained 360 bytes** on the 20 that carry a
//! mip chain, the same whether the faces are 128x128 `DXT1` or 2048x2048
//! (recorded in [`Texture::chain_len`]).
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
/// Every HD file declares one; the cap stops a garbage count passing.
pub const MAX_TEXTURES: u32 = 64;

/// Most mip levels a descriptor may declare.
///
/// Twelve is the largest on disc (a 2048x2048 sky). Same reasoning as
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
    /// The strongest signal a blob is not a `.gtf`. Holds on 7,333 of 7,333.
    ChainLengthMismatch {
        /// Length the descriptor implies.
        expected: usize,
        /// Length it declares.
        declared: u32,
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
            Self::Cubemap => write!(f, "cubemap faces are parsed but not decoded"),
        }
    }
}

impl std::error::Error for Error {}

/// Result alias for this module.
pub type Result<T> = std::result::Result<T, Error>;

/// The texel format, from the low five bits of the descriptor's format byte.
///
/// Named as Sony names them: `A8R8G8B8` is a big-endian `u32` with alpha on top,
/// so bytes arrive A, R, G, B. Only formats the HD disc carries are here; any
/// other byte is [`Error::UnknownFormat`].
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

/// What one output channel does, from a [`Remap`]'s control table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    /// Force `0x00`.
    Zero,
    /// Force `0xff`.
    One,
    /// Read the source channel [`Remap::source`] names.
    Read,
}

/// Pair index (`A`, `R`, `G`, `B`, as the word packs them) to the slot that
/// channel occupies in an RGBA texel.
const CHANNEL_SLOT: [usize; 4] = [3, 0, 1, 2];

/// The descriptor's `+0x10` channel remap, decomposed.
///
/// Two per-channel tables packed into sixteen bits, laid out as the RSX's
/// `NV4097_SET_TEXTURE_CONTROL1` register reads them: the **low** byte holds
/// four 2-bit *source* selectors and the **high** byte four 2-bit *controls*,
/// both in `A`, `R`, `G`, `B` order from the least significant pair up. A
/// source names which of the texel's channels an output reads (`0` = A, `1` =
/// R, `2` = G, `3` = B); a control says whether that read happens, or the
/// output is forced to zero or one.
///
/// # Corroborated by the disc
///
/// Three words appear on the HD disc, and each decomposes into something its
/// file independently agrees with:
///
/// | Word | Files | Decomposes to | Agrees with |
/// | --- | ---: | --- | --- |
/// | `0xaae4` | 7,317 | every control `Read`, sources `A<-A, R<-R, G<-G, B<-B` | the identity, which is what decoding a texel straight already does |
/// | `0xa9e4` | 7 | the same, but alpha forced to one | `fealphaluminancetexture.gtf` is **100% grayscale** - all four of every texel's bytes equal - so its RGB is a luminance and its alpha is not stored art |
/// | `0xa9ff` | 9 | alpha forced to one, and **every** other output reading the *blue* source | these are exactly the disc's 9 [`Format::B8`] files, whose one stored byte **is** the blue channel |
///
/// The third row is the load-bearing one: `0xff` selects source `3` (blue)
/// four times, and the format carrying it stores one byte in blue. Format and
/// remap come from different halves of the descriptor and agree.
///
/// **Confidence 85.** The layout predicts three distributions, one of them
/// (`0xa9ff`) cross-checked against an unrelated field. Not corroborated
/// against the executable. See `docs/formats/gtf.md`.
///
/// An earlier constant forced *blue* to one for `0xa9e4` (alpha's control pair
/// misplaced) and rendered all 7 `A8B8G8R8` files solid blue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Remap {
    /// Which slot of the decoded texel each output reads, in R, G, B, A order.
    pub source: [usize; 4],
    /// What each output does, in R, G, B, A order.
    pub control: [Control; 4],
}

impl Remap {
    /// Decomposes one `+0x10` word.
    #[must_use]
    pub fn decode(word: u32) -> Self {
        let mut source = [0usize; 4];
        let mut control = [Control::Read; 4];
        for pair in 0..4 {
            let slot = CHANNEL_SLOT[pair];
            source[slot] = CHANNEL_SLOT[((word >> (pair * 2)) & 3) as usize];
            control[slot] = match (word >> (8 + pair * 2)) & 3 {
                0 => Control::Zero,
                1 => Control::One,
                _ => Control::Read,
            };
        }
        Self { source, control }
    }

    /// Whether this leaves a decoded texel exactly as it is.
    #[must_use]
    pub fn is_identity(&self) -> bool {
        self.source == [0, 1, 2, 3] && self.control.iter().all(|c| *c == Control::Read)
    }

    /// Rewrites each texel's four channels in place.
    ///
    /// Public so a caller that decoded a level itself can apply the remap too.
    pub fn apply(&self, out: &mut [[u8; 4]]) {
        if self.is_identity() {
            return;
        }
        for texel in out {
            let read = *texel;
            for (slot, channel) in texel.iter_mut().enumerate() {
                *channel = match self.control[slot] {
                    Control::Zero => 0x00,
                    Control::One => 0xff,
                    Control::Read => read[self.source[slot]],
                };
            }
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
    /// `+0x10`, a per-channel source-and-force table.
    ///
    /// Kept raw; [`Remap::decode`] reads it, and [`Self::to_rgba`] and
    /// [`Self::face_to_rgba`] apply it. See [`Remap`].
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
    /// A row of blocks for a compressed format; does not halve down the chain.
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

    /// Applies [`Self::remap`] to a decoded RGBA8 buffer; a no-op on the 7,317
    /// identity files.
    fn apply_remap(&self, out: &mut [[u8; 4]]) {
        Remap::decode(self.remap).apply(out);
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
    /// Includes the 360 unexplained bytes a multi-level cubemap carries, since
    /// the declared length includes them on all 20; see the module docs.
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
    /// **Non-cubemaps only.** For a cubemap this is the level of the first face,
    /// assuming it starts at offset zero (not checked); see the 360 unexplained
    /// bytes in the module docs. [`Self::to_rgba`] refuses a cubemap.
    #[must_use]
    pub fn level_range(&self, level: u8) -> Range<usize> {
        let start = self.data.start + (0..level).map(|l| self.level_len(l)).sum::<usize>();
        start..start + self.level_len(level)
    }

    /// Byte range of one mip level of one **face**.
    ///
    /// **All of a face's levels are consecutive, then the next face's**
    /// (`one_face * faces() + slack`, holding on all 23 cubemaps). A level-major
    /// walk would read face 1's base level as face 0's second.
    ///
    /// `face` past [`Self::faces`] answers the last face rather than panicking;
    /// [`Self::face_to_rgba`] range-checks.
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
    /// `0`..`5` are `+X, -X, +Y, -Y, +Z, -Z`: the published order, not measured
    /// here (nothing in a `.gtf` names a face). Checked by picture: on a sky the
    /// four side faces must meet at their vertical edges. See
    /// `docs/formats/gtf.md`.
    ///
    /// # Errors
    ///
    /// [`Error::Cubemap`] for a texture that is *not* one, since a face index is
    /// meaningless there, and [`Error::DataOutOfBounds`] for a face past the six.
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
        let range = self.face_range(face, 0);
        let texels = blob.get(range.clone()).ok_or(Error::DataOutOfBounds {
            offset: range.start as u32,
            length: (range.end - range.start) as u32,
            got: blob.len(),
        })?;
        let (width, height) = self.level_size(0);
        // `None` is a truncated blob, not an unimplemented swizzle.
        let mut out = decode::level(
            self.format,
            texels,
            width,
            height,
            self.pitch as usize,
            self.is_linear(),
        )
        .ok_or(Error::DataOutOfBounds {
            offset: range.start as u32,
            length: (range.end - range.start) as u32,
            got: blob.len(),
        })?;
        self.apply_remap(&mut out);
        Ok(out)
    }

    /// Decodes the base mip level to straight RGBA8.
    ///
    /// `blob` must be the bytes [`Gtf::parse`] was given.
    ///
    /// # Errors
    ///
    /// [`Error::Cubemap`] for a cubemap, and [`Error::DataOutOfBounds`] if the
    /// level does not fit (the wrong blob, since `parse` ruled it out).
    pub fn to_rgba(&self, blob: &[u8]) -> Result<Vec<[u8; 4]>> {
        if self.cubemap {
            return Err(Error::Cubemap);
        }
        let range = self.level_range(0);
        let texels = blob.get(range.clone()).ok_or(Error::DataOutOfBounds {
            offset: range.start as u32,
            length: (range.end - range.start) as u32,
            got: blob.len(),
        })?;
        let (width, height) = self.level_size(0);
        // `None` is a truncated blob, not an unimplemented swizzle.
        let mut out = decode::level(
            self.format,
            texels,
            width,
            height,
            self.pitch as usize,
            self.is_linear(),
        )
        .ok_or(Error::DataOutOfBounds {
            offset: range.start as u32,
            length: (range.end - range.start) as u32,
            got: blob.len(),
        })?;
        self.apply_remap(&mut out);
        Ok(out)
    }
}

/// Decodes one mip level to straight RGBA8, given that level's own texels.
///
/// The free-function half of [`Texture::to_rgba`], for `oag_render`'s fallback
/// on an adapter with no block-compression support. `pitch` is the descriptor's,
/// in bytes, 0 meaning tightly packed.
///
/// `linear` is the descriptor's `0x20` bit ([`Texture::is_linear`]); it only
/// matters for the two uncompressed formats, so `true` is safe for a
/// block-compressed `format`.
///
/// `None` for texels that stop short of the dimensions given.
#[must_use]
pub fn decode_level(
    format: Format,
    texels: &[u8],
    width: u32,
    height: u32,
    pitch: usize,
    linear: bool,
) -> Option<Vec<[u8; 4]>> {
    decode::level(format, texels, width, height, pitch, linear)
}

/// The unexplained tail on a cubemap that carries a mip chain.
///
/// The same 360 bytes on all 20, whatever the face size or format.
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
        // Saturating: a garbage count must fail the bounds test, not wrap.
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
    /// Every HD file declares exactly one. `None` only for a file declaring
    /// none, which `parse` refuses.
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
