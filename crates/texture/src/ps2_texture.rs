//! PS2 palette-indexed textures: a Graphics Synthesizer upload packet.
//!
//! Where the PSP stores a header, palette and pixels ([`crate::texture`]), the
//! PS2 stores the **DMA packet that uploads it**: a 13-byte header, then GIF
//! packets transferring the texels and palette into GS local memory, so pixels
//! are in the GS's layout rather than raster order.
//!
//! ```text
//! +0x00  u8    log2 dimensions, high nibble log2(height), low nibble log2(width)
//! +0x01  u8    bits_per_pixel      4 or 8
//! +0x02  u16   flags               0x2000 or 0x2040
//! +0x04  u16   height
//! +0x06  u16   width
//! +0x08  u32   unknown, correlates with +0x0c
//! +0x0c  u8    unknown
//! +0x0d  qword[8]   GS state, not decoded
//! +0x8d  qword      A+D write, TRXPOS  (0x51)
//! +0x9d  qword      A+D write, TRXREG  (0x52)
//! +0xad  qword      A+D write, TRXDIR  (0x53)
//! +0xbd  qword      GIFtag, FLG=IMAGE, NLOOP*16 == texel bytes
//! +0xcd  texels, width * height * bpp / 8 bytes
//!        qword      A+D write, TRXPOS
//!        qword      A+D write, TRXREG
//!        qword      A+D write, TRXDIR
//!        qword      GIFtag, FLG=IMAGE, EOP, NLOOP*16 == palette bytes
//!        palette, (1 << bpp) * 4 bytes, RGBA8888
//!        padding
//! ```
//!
//! Each transfer block is *budgeted* 256 bytes even when it needs fewer, so the
//! total is `205 + max(texels, 256) + 64 + max(palette, 256)`, but the blocks sit
//! back to back and the slack lands at the end. A 4x4 texture is 1,549 bytes
//! with 240 of padding.
//!
//! See `docs/formats/ps2-texture.md` for the evidence.
//!
//! # The dimensions are stored height first
//!
//! Reading `+0x04` as the width decodes every square texture and scrambles every
//! other one, a half-right that survives a spot check. `TRXREG` settles it: it
//! is `(width/2, height/2)` for the 32-bit transfer path, so the pair at `+0x04`
//! is `(height, width)`.
//!
//! # Three transfer shapes, and the file says which
//!
//! `TRXREG` gives the destination rectangle; the texel byte count divided by its
//! area is the bytes per destination pixel, which identifies the transfer:
//!
//! - `(width/2, height/2)` at 4 bytes each: 8-bit texels blitted as **PSMCT32**.
//!   The stored bytes are pre-swizzled into GS `PSMT8` order and permuted back.
//! - `(width, height)` at 1 byte each: a direct `PSMT8` transfer, raster order.
//!   Used whenever the width is 8 or less, where halving gives no valid rectangle.
//! - `(width/2, height/4)` at 4 bytes each: the same trick for 4-bit texels
//!   (`PSMT4`). Five blobs on the disc, the five PS2 font atlases: see
//!   [`psmt4_offset`] and [`crate::fnt`].
//!
//! # Alpha is 0-128, not 0-255
//!
//! The GS treats 128 as full intensity through texture-modulate, as
//! `oag_vex::vex` documents for vertex colour. [`Ps2Texture`] keeps the palette
//! as stored and [`Ps2Texture::to_rgba`] doubles it.

/// Bytes of header before the GS state block.
pub const HEADER_LEN: usize = 13;

/// Quadwords of GS setup between the header and the texel data.
pub const SETUP_QWORDS: usize = 12;

/// Offset of the texel data.
pub const TEXEL_OFFSET: usize = HEADER_LEN + SETUP_QWORDS * 16;

/// Offset of the `TRXREG` write, whose data word gives the transfer rectangle.
pub const TRXREG_OFFSET: usize = HEADER_LEN + 9 * 16;

/// Offset of the texel `GIFtag`.
pub const TEXEL_GIFTAG_OFFSET: usize = HEADER_LEN + 11 * 16;

/// Quadwords of GS setup between the texels and the palette.
pub const CLUT_SETUP_QWORDS: usize = 4;

/// Smallest block the game transfers, in bytes.
///
/// Each block is *budgeted* this much even when it needs less (a 4x4 texture is
/// 1,549 bytes, not 1,309); the slack lands at the end of the file.
pub const MIN_TRANSFER_BYTES: usize = 256;

/// `GIFtag` `FLG` value for an image-mode transfer.
const GIF_FLG_IMAGE: u64 = 2;

/// Something wrong with a PS2 texture blob.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Fewer bytes than the header and setup block need.
    TooShort {
        /// Bytes supplied.
        got: usize,
    },
    /// `bits_per_pixel` was not 4 or 8.
    UnsupportedDepth {
        /// The value found.
        bits_per_pixel: u8,
    },
    /// The log2 byte at `+0x00` disagrees with the dimensions at `+0x04`.
    ///
    /// The two encode the same thing, so a disagreement means not a texture.
    DimensionMismatch {
        /// The packed log2 byte.
        packed: u8,
        /// Width read from `+0x06`.
        width: u16,
        /// Height read from `+0x04`.
        height: u16,
    },
    /// A `GIFtag` was not an image-mode transfer of the expected length.
    BadGifTag {
        /// Offset of the tag.
        offset: usize,
        /// Bytes the tag declares.
        declared: usize,
        /// Bytes the header implies.
        expected: usize,
    },
    /// `TRXREG` did not match any known transfer shape.
    UnknownTransfer {
        /// Transfer width.
        rrw: u32,
        /// Transfer height.
        rrh: u32,
    },
    /// The blob is not the size the header implies.
    SizeMismatch {
        /// Size the header implies.
        expected: usize,
        /// Size supplied.
        got: usize,
    },
    /// A layout that is recognised but not decoded.
    UnsupportedLayout(Layout),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooShort { got } => write!(f, "need at least {TEXEL_OFFSET} bytes, got {got}"),
            Self::UnsupportedDepth { bits_per_pixel } => {
                write!(
                    f,
                    "unsupported depth {bits_per_pixel} bpp (expected 4 or 8)"
                )
            }
            Self::DimensionMismatch {
                packed,
                width,
                height,
            } => write!(
                f,
                "packed dimensions {packed:#04x} disagree with {width}x{height}"
            ),
            Self::BadGifTag {
                offset,
                declared,
                expected,
            } => write!(
                f,
                "GIFtag at {offset:#x} declares {declared} bytes, header implies {expected}"
            ),
            Self::UnknownTransfer { rrw, rrh } => {
                write!(f, "unrecognised transfer rectangle {rrw}x{rrh}")
            }
            Self::SizeMismatch { expected, got } => {
                write!(f, "header implies {expected} bytes, blob is {got}")
            }
            Self::UnsupportedLayout(layout) => write!(f, "{layout:?} is not decoded"),
        }
    }
}

impl std::error::Error for Error {}

/// Result alias for this module.
pub type Result<T> = std::result::Result<T, Error>;

/// How the texels sit in the blob, as identified by `TRXREG`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    /// Raster order, uploaded by a direct `PSMT8` transfer.
    Linear,
    /// `PSMT8` swizzle, uploaded as a `PSMCT32` blit of half the dimensions.
    Psmt8,
    /// `PSMT4` swizzle, uploaded as a `PSMCT32` blit of half the width and a
    /// quarter of the height.
    Psmt4,
}

/// A decoded PS2 texture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ps2Texture {
    /// Width in pixels.
    pub width: u16,
    /// Height in pixels.
    pub height: u16,
    /// 4 or 8.
    pub bits_per_pixel: u8,
    /// The flag word at `+0x02`, 0x2000 or 0x2040 on this disc.
    pub flags: u16,
    /// How the texels were stored before this decode unpicked it.
    pub layout: Layout,
    /// Palette as stored: RGBA8888 with alpha on the GS's 0-128 scale.
    pub palette: Vec<[u8; 4]>,
    /// One palette index per pixel, in raster order from the top left.
    pub indices: Vec<u8>,
}

/// Just enough of the header to classify a blob, without decoding it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    /// Width in pixels.
    pub width: u16,
    /// Height in pixels.
    pub height: u16,
    /// 4 or 8.
    pub bits_per_pixel: u8,
    /// The flag word at `+0x02`.
    pub flags: u16,
    /// How the texels are stored.
    pub layout: Layout,
    /// Bytes of texel data the header implies, before padding.
    pub texel_bytes: usize,
    /// Bytes of palette the header implies, before padding.
    pub palette_bytes: usize,
    /// The size the whole blob must be.
    pub total_bytes: usize,
}

fn read_u16(data: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([data[at], data[at + 1]])
}

fn read_u64(data: &[u8], at: usize) -> u64 {
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&data[at..at + 8]);
    u64::from_le_bytes(bytes)
}

/// Rounds a transfer block up to the minimum the game uses.
fn padded(bytes: usize) -> usize {
    bytes.max(MIN_TRANSFER_BYTES)
}

/// Reads and checks the header without touching the pixel data.
///
/// Every field is cross-checked against another, so this is safe as a classifier
/// over a whole archive: the log2 byte must agree with the dimension words, both
/// `GIFtag`s must declare the byte counts the dimensions imply, and the total
/// must equal the blob's length.
pub fn header(data: &[u8]) -> Result<Header> {
    if data.len() < TEXEL_OFFSET {
        return Err(Error::TooShort { got: data.len() });
    }

    let packed = data[0];
    let bits_per_pixel = data[1];
    let flags = read_u16(data, 2);
    let height = read_u16(data, 4);
    let width = read_u16(data, 6);

    if !matches!(bits_per_pixel, 4 | 8) {
        return Err(Error::UnsupportedDepth { bits_per_pixel });
    }
    // Both nibbles are a shift count; above 15 would overflow (largest on disc is 9).
    let log_height = u32::from(packed >> 4);
    let log_width = u32::from(packed & 0x0f);
    if 1u32.checked_shl(log_height) != Some(u32::from(height))
        || 1u32.checked_shl(log_width) != Some(u32::from(width))
    {
        return Err(Error::DimensionMismatch {
            packed,
            width,
            height,
        });
    }

    let pixels = usize::from(width) * usize::from(height);
    let texel_bytes = pixels * usize::from(bits_per_pixel) / 8;
    let palette_bytes = (1usize << bits_per_pixel) * 4;

    check_giftag(data, TEXEL_GIFTAG_OFFSET, texel_bytes, false)?;

    // The palette packet follows the texels **unpadded**; padding to the 256-byte
    // minimum lands at the end of the file. Padding the offset here would read
    // correctly on all but 70 of the disc's textures, the small ones.
    let clut_setup = TEXEL_OFFSET + texel_bytes;
    let total_bytes =
        TEXEL_OFFSET + padded(texel_bytes) + CLUT_SETUP_QWORDS * 16 + padded(palette_bytes);
    if data.len() != total_bytes {
        return Err(Error::SizeMismatch {
            expected: total_bytes,
            got: data.len(),
        });
    }
    check_giftag(data, clut_setup + 3 * 16, palette_bytes, true)?;

    let trxreg = read_u64(data, TRXREG_OFFSET);
    // Masked to the GS field width, so both fit a u32 with room to spare.
    let rrw = u32::try_from(trxreg & 0xfff).unwrap_or(u32::MAX);
    let rrh = u32::try_from((trxreg >> 32) & 0xfff).unwrap_or(u32::MAX);
    let (w32, h32) = (u32::from(width), u32::from(height));
    let layout = if bits_per_pixel == 8 && rrw == w32 / 2 && rrh == h32 / 2 {
        Layout::Psmt8
    } else if bits_per_pixel == 8 && rrw == w32 && rrh == h32 {
        Layout::Linear
    } else if bits_per_pixel == 4 && rrw == w32 / 2 && rrh == h32 / 4 {
        Layout::Psmt4
    } else {
        return Err(Error::UnknownTransfer { rrw, rrh });
    };

    Ok(Header {
        width,
        height,
        bits_per_pixel,
        flags,
        layout,
        texel_bytes,
        palette_bytes,
        total_bytes,
    })
}

fn check_giftag(data: &[u8], offset: usize, expected: usize, last: bool) -> Result<()> {
    if data.len() < offset + 16 {
        return Err(Error::TooShort { got: data.len() });
    }
    let tag = read_u64(data, offset);
    let declared = (tag & 0x7fff) as usize * 16;
    let flg = (tag >> 58) & 3;
    let eop = (tag >> 15) & 1 == 1;
    if flg != GIF_FLG_IMAGE || declared != expected || eop != last {
        return Err(Error::BadGifTag {
            offset,
            declared,
            expected,
        });
    }
    Ok(())
}

/// Whether `data` is a PS2 texture.
///
/// Cheaper than [`parse`] and as strict: [`header`] checks every declared size.
#[must_use]
pub fn looks_like_ps2_texture(data: &[u8]) -> bool {
    header(data).is_ok()
}

/// Parses a PS2 texture blob.
///
/// # Errors
///
/// Every way [`header`] can fail, plus [`Error::UnsupportedLayout`] for a
/// swizzled blob too small for its permutation: [`Layout::Psmt4`] below one
/// 128x128 GS page, [`Layout::Psmt8`] narrower than 16 or shorter than 4.
/// Neither exists on either disc. See [`psmt4_offset`] and [`psmt8_offset`].
pub fn parse(data: &[u8]) -> Result<Ps2Texture> {
    let head = header(data)?;
    // [`psmt4_offset`]'s transposition works inside a 128x128 page; a smaller
    // texture would address missing pixels. The five on disc are 256x128,
    // 512x256 and 512x512.
    if head.layout == Layout::Psmt4 && (head.width < 128 || head.height < 128) {
        return Err(Error::UnsupportedLayout(head.layout));
    }
    // The same guard for PSMT8: [`psmt8_offset`] is a permutation of
    // `0..width * height` only for widths of 16+ and heights of 4+ (both powers
    // of two), and below that it indexes past the texels - a self-consistent
    // 8x8 blob reaches byte 77 of 64 and would panic. Archive browses hand the
    // parser whatever the file claims. The disc stores narrow textures
    // [`Layout::Linear`] for this reason.
    if head.layout == Layout::Psmt8 && (head.width < 16 || head.height < 4) {
        return Err(Error::UnsupportedLayout(head.layout));
    }

    let texels = &data[TEXEL_OFFSET..TEXEL_OFFSET + head.texel_bytes];
    let clut_at = TEXEL_OFFSET + head.texel_bytes + CLUT_SETUP_QWORDS * 16;
    let stored_palette = &data[clut_at..clut_at + head.palette_bytes];

    let width = usize::from(head.width);
    let height = usize::from(head.height);

    let indices = match head.layout {
        Layout::Linear => texels.to_vec(),
        Layout::Psmt8 => {
            let mut out = vec![0u8; width * height];
            for y in 0..height {
                for x in 0..width {
                    out[y * width + x] = texels[psmt8_offset(x, y, width)];
                }
            }
            out
        }
        Layout::Psmt4 => {
            let mut out = vec![0u8; width * height];
            for y in 0..height {
                for x in 0..width {
                    let (at, nibble) = psmt4_offset(x, y, width);
                    out[y * width + x] = (texels[at] >> (nibble * 4)) & 0x0f;
                }
            }
            out
        }
    };

    let palette = unswizzle_clut(stored_palette, head.bits_per_pixel);

    Ok(Ps2Texture {
        width: head.width,
        height: head.height,
        bits_per_pixel: head.bits_per_pixel,
        flags: head.flags,
        layout: head.layout,
        palette,
        indices,
    })
}

/// Byte offset of texel `(x, y)` inside a `PSMT8`-swizzled block.
///
/// The GS lays 8-bit textures out in 16x16 blocks of 16x4 columns with a
/// two-of-four row swap (the `swap` and `byte_select` terms). This is a
/// permutation of `0..width * height` for every power-of-two width of 16 or
/// more **and height of 4 or more**, which the tests assert; narrower textures
/// on the disc are stored [`Layout::Linear`]. Below those bounds it indexes
/// past the texels, so [`parse`] refuses the shape.
#[must_use]
pub fn psmt8_offset(x: usize, y: usize, width: usize) -> usize {
    let block = (y & !0xf) * width + (x & !0xf) * 2;
    let swap = (((y + 2) >> 2) & 1) * 4;
    let row = ((((y & !3) >> 1) + (y & 1)) & 0x7) * width * 2;
    let column = ((x + swap) & 0x7) * 4;
    let byte_select = ((y >> 1) & 1) + ((x >> 2) & 2);
    block + row + column + byte_select
}

/// Byte offset and nibble of texel `(x, y)` inside a `PSMT4`-swizzled blob.
///
/// The nibble is 0 for the low half of the byte and 1 for the high half, the
/// same order [`crate::texture`] and [`crate::fnt`] use.
///
/// # Derived from [`psmt8_offset`], not from a table
///
/// Both answer where in a **linear `PSMCT32` source image** an indexed texel
/// lives (the blob is the source of a blit, not GS memory). `PSMT8`: source
/// `(width/2, height/2)`, page 128x64, block 16x16. `PSMT4`: source `(width/2,
/// height/4)`, page 128x128, block 32x16. `PSMT8`'s blocks land in raster order,
/// so it needs no page term; within a `PSMT4` page the block at block-column `bx`,
/// block-row `by` sits at source block-column `by`, source block-row `bx` (the
/// transposition is the only new fact; `blockTable4` is the transpose of
/// `blockTable32`). Inside a block, `x`'s low three bits pick the source column,
/// its next two the byte within that word, `y`'s bit 1 the nibble, and `y`'s
/// remaining bits the source row via [`psmt8_offset`]'s two-of-four row swap.
///
/// # Evidence
///
/// A permutation of `0..width * height` at every disc shape (asserted by the
/// tests; the non-transposed reading fails it). The five blobs are the PS2 font
/// atlases and every lit texel of `pulse_text` lands inside a glyph box its `.fnt`
/// declares. See `docs/formats/ps2-texture.md`.
#[must_use]
pub fn psmt4_offset(x: usize, y: usize, width: usize) -> (usize, usize) {
    // Pages tile the source in raster order, and a PSMT4 page covers a whole
    // 64x32 source page, so this term needs no transposing.
    let page = (x / 128) * 64 + (y / 128) * 32 * (width / 2);
    // Within the page, block columns become source block rows and back.
    let block = ((y % 128) / 16) * 8 + ((x % 128) / 32) * 8 * (width / 2);
    let swap = (((y + 2) >> 2) & 1) * 4;
    let column = (x + swap) & 0x7;
    let row = (((y & !3) >> 1) + (y & 1)) & 0x7;
    let word = page + block + row * (width / 2) + column;
    (word * 4 + ((x >> 3) & 3), (y >> 1) & 1)
}

/// Reorders a 256-entry palette out of the GS's `CSM1` layout.
///
/// A 256-entry CLUT is uploaded as a 16x16 `PSMCT32` rectangle, which swaps
/// entries 8-15 and 16-23 of each 32. Getting it wrong leaves shapes intact and
/// bands the colours every eight indices. A 16-entry CLUT is a plain 8x2
/// rectangle.
#[must_use]
pub fn unswizzle_clut(stored: &[u8], bits_per_pixel: u8) -> Vec<[u8; 4]> {
    let entries: Vec<[u8; 4]> = stored.as_chunks::<4>().0.to_vec();
    if bits_per_pixel != 8 {
        return entries;
    }
    (0..entries.len())
        .map(|i| entries[(i & 0xe7) | ((i & 0x08) << 1) | ((i & 0x10) >> 1)])
        .collect()
}

/// [`Ps2Texture::roughness`] over an arbitrary index buffer.
///
/// Free-standing so a caller can score a *different* reading as a control.
#[must_use]
pub fn roughness_of(indices: &[u8], width: usize, height: usize) -> u64 {
    if width < 2 || height < 2 || indices.len() < width * height {
        return 0;
    }
    let diff =
        |a: usize, b: usize| u64::from(i32::from(indices[a]).abs_diff(i32::from(indices[b])));
    let mut total = 0u64;
    for y in 0..height - 1 {
        for x in 0..width - 1 {
            total += diff(y * width + x, y * width + x + 1);
            total += diff(y * width + x, (y + 1) * width + x);
        }
    }
    total
}

impl Ps2Texture {
    /// Total variation of the decoded indices: the sum of `|delta|` between
    /// horizontally and vertically adjacent pixels.
    ///
    /// This **checks the decoder**, not describes the texture: any wrong
    /// permutation scatters pixels that belong together and raises local
    /// discontinuity. Comparing against the same texels read the other way says
    /// the [`Layout::Psmt8`] permutation is correct, not merely bijective; the
    /// ground-truth test runs it over the whole disc.
    ///
    /// The neighbour-*asymmetry* test of [`crate::texture::Texture::looks_swizzled`]
    /// does **not** work here: PSP swizzle moves 16-byte rows, but the GS
    /// `PSMT8` permutation is local in both axes (541 textures flagged decoded
    /// against 531 read raw, no signal). Total variation separates them.
    #[must_use]
    pub fn roughness(&self) -> u64 {
        let (w, h) = (usize::from(self.width), usize::from(self.height));
        roughness_of(&self.indices, w, h)
    }

    /// Expands to RGBA8888, row-major from the top left.
    ///
    /// Alpha is doubled and saturated, taking the GS's 128 to 255.
    #[must_use]
    pub fn to_rgba(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.indices.len() * 4);
        for &index in &self.indices {
            let colour = self
                .palette
                .get(index as usize)
                .copied()
                // Cannot happen for 8bpp with a 256-entry palette; magenta beats silent black.
                .unwrap_or([255, 0, 255, 128]);
            out.extend_from_slice(&[colour[0], colour[1], colour[2], colour[3].saturating_mul(2)]);
        }
        out
    }
}

#[cfg(test)]
mod tests;
