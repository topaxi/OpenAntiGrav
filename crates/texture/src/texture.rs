//! Palette-indexed textures (`.mip`).
//!
//! ```text
//! +0x00  u16      width
//! +0x02  u16      height
//! +0x04  u8       bits_per_pixel      4 or 8
//! +0x05  u8       unk_0x05            0 in every standalone .mip seen
//! +0x06  u8       mip_levels          1 in FE.wad; 4 on the engine-flare sprite
//! +0x07  u8       flags               bit 0 is FLAG_SWIZZLED
//! +0x08  u8       unk_0x08            complement of +0x07 bit 0
//! +0x09  u8[7]    zero
//! +0x10  RGBA8888 palette, 1 << bits_per_pixel entries
//!        level 0, width * height * bits_per_pixel / 8 bytes
//!        then each further level, both dimensions halved (floored at 1)
//! ```
//!
//! **`+0x06` is a mip count**, which this parser assumed was always 1 until
//! `Data\Tex\EngineFlare\grabbedEngineFlare128x64x8.mip` - 128x64 at 8bpp,
//! `+0x06 == 4`, 11,920 bytes against the 9,232 one level implies - showed
//! otherwise. `psp-texture.md` had predicted exactly that gap. Only level 0 is
//! decoded; the rest of the chain is validated and skipped.
//!
//! See `docs/formats/psp-texture.md` for the evidence.
//!
//! # These are not swizzled
//!
//! The PSP GPU reads textures in a swizzled layout and games commonly store them
//! pre-swizzled, which would make a literal decode come out recognisable but
//! scrambled in 16-byte-wide blocks. Pulse's `.mip` blobs are **not**: every one
//! decodes to a clean image, and `unk_0x07` and `unk_0x08` were ruled out as
//! swizzle flags because they do not vary with anything that would matter.
//! [`Texture::looks_swizzled`] stays as a triage heuristic for other archives,
//! not because this question is open. See `docs/formats/psp-texture.md`.
//!
//! # 4bpp needs an even pixel count
//!
//! Two pixels share a byte with no row padding, so `width * height` must be
//! even for the packed data to describe whole pixels. Every real texture is
//! power-of-two in both dimensions, so this never comes up in practice, and it
//! is refused rather than guessed at: inventing a padding rule for a case the
//! game cannot produce would be a decoder that lies about untrusted input.

use oag_formats::swizzle::{FLAG_SWIZZLED, SWIZZLE_BLOCK_BYTES, unswizzle};

/// Bytes of header before the palette.
pub const HEADER_LEN: usize = 16;

/// Something wrong with a texture blob.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Fewer bytes than the header needs.
    TooShort {
        /// Bytes supplied.
        got: usize,
    },
    /// `bits_per_pixel` was not 4 or 8.
    UnsupportedDepth {
        /// The value found.
        bits_per_pixel: u8,
    },
    /// Width or height was zero.
    ZeroSized {
        /// Width found.
        width: u16,
        /// Height found.
        height: u16,
    },
    /// 4bpp with an odd pixel count, which cannot be packed into whole bytes.
    OddPixelCountAt4Bpp {
        /// Width found.
        width: u16,
        /// Height found.
        height: u16,
    },
    /// The blob is not the size the header implies.
    ///
    /// The strongest signal that a blob is not a texture at all, since the
    /// size is fully determined by the header.
    SizeMismatch {
        /// Size the header implies.
        expected: usize,
        /// Size supplied.
        got: usize,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooShort { got } => write!(f, "need at least {HEADER_LEN} bytes, got {got}"),
            Self::UnsupportedDepth { bits_per_pixel } => {
                write!(
                    f,
                    "unsupported depth {bits_per_pixel} bpp (expected 4 or 8)"
                )
            }
            Self::ZeroSized { width, height } => write!(f, "zero-sized texture {width}x{height}"),
            Self::OddPixelCountAt4Bpp { width, height } => write!(
                f,
                "{width}x{height} is an odd number of pixels, which 4bpp cannot pack"
            ),
            Self::SizeMismatch { expected, got } => {
                write!(f, "header implies {expected} bytes, blob is {got}")
            }
        }
    }
}

impl std::error::Error for Error {}

/// Result alias for this module.
pub type Result<T> = std::result::Result<T, Error>;

/// A decoded texture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Texture {
    /// Width in pixels.
    pub width: u16,
    /// Height in pixels.
    pub height: u16,
    /// 4 or 8.
    pub bits_per_pixel: u8,
    /// Palette, `1 << bits_per_pixel` entries of RGBA8888.
    pub palette: Vec<[u8; 4]>,
    /// One palette index per pixel, unpacked from 4-bit pairs where needed.
    ///
    /// Always exactly `width * height` long. Consumers rely on that: `to_rgba`
    /// feeds [`crate::png::encode_rgba`], which asserts the buffer matches the
    /// dimensions it is given.
    pub indices: Vec<u8>,
    /// The header bytes whose meaning is not established, `+0x05` to `+0x08`.
    pub unknown: [u8; 4],
    /// Mip levels stored in the blob, from `+0x06`; at least 1.
    ///
    /// Only level 0 is decoded - [`Texture::indices`] is always exactly
    /// `width * height` - but the count has to be read to know how long the blob
    /// should be. See [`Texture::parse`].
    pub mip_levels: u8,
}

/// Most mip levels a blob may declare.
///
/// A 4-level chain is the largest seen. The cap exists so a garbage byte at
/// `+0x06` cannot make the size arithmetic accept an arbitrary blob: without it,
/// a large enough level count sums to almost any length.
pub const MAX_MIP_LEVELS: u8 = 16;

/// Bytes the whole mip chain occupies.
///
/// Each level halves both dimensions, floored at 1, and rounds up to a whole byte.
///
/// # Levels after the first pad their rows to 16 bytes
///
/// Confidence **88**, and the evidence is two exact hits from one rule:
///
/// | Texture | Levels | Unpadded | Padded tail | Blob |
/// | --- | ---: | ---: | ---: | ---: |
/// | `grabbedEngineFlare128x64x8` 128x64 | 4 | 11,920 | 11,920 | **11,920** |
/// | `Engine_noise` 64x64 | 4 | 6,480 | **6,544** | **6,544** |
///
/// The flare is unaffected because every one of its levels is already at least 16
/// bytes wide; the noise texture's level 3 is 8x8, an 8-byte row padded to 16, and
/// that single difference is the whole 64-byte discrepancy. `SWIZZLE_BLOCK_BYTES`
/// is the same 16, which is not a coincidence: it is the GE's texture row
/// alignment.
///
/// **Level 0 is deliberately not padded**, and that asymmetry is a statement about
/// the evidence rather than about the hardware. No observed texture has a level-0
/// row under 16 bytes, so the question is untested there - and leaving it unpadded
/// makes the single-level arithmetic byte-identical to what it was before mip
/// support existed, which is what keeps the 346 already-decoding entries decoding.
/// If a narrow single-level texture ever fails to identify, this is the first thing
/// to try.
fn mip_chain_len(width: u16, height: u16, bits_per_pixel: u8, levels: u8) -> usize {
    let mut total = 0usize;
    let (mut w, mut h) = (usize::from(width), usize::from(height));
    for level in 0..levels.max(1) {
        let row = (w * usize::from(bits_per_pixel)).div_ceil(8);
        let stride = if level == 0 {
            row
        } else {
            row.next_multiple_of(SWIZZLE_BLOCK_BYTES)
        };
        total += stride * h;
        w = (w / 2).max(1);
        h = (h / 2).max(1);
    }
    total
}

impl Texture {
    /// Whether `data` is a texture.
    ///
    /// This is a full parse, not a header sniff, so it needs the whole blob and
    /// it allocates. That makes it strong (the declared size must match the
    /// blob exactly, so a false positive is very unlikely) and it makes it
    /// expensive: running it over every entry of a 315 MiB archive reads and
    /// decompresses all of it. Worth knowing before using it as a filter.
    #[must_use]
    pub fn looks_like_texture(data: &[u8]) -> bool {
        Self::parse(data).is_ok()
    }

    /// Parses a texture blob.
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < HEADER_LEN {
            return Err(Error::TooShort { got: data.len() });
        }

        let width = u16::from_le_bytes([data[0], data[1]]);
        let height = u16::from_le_bytes([data[2], data[3]]);
        let bits_per_pixel = data[4];

        if width == 0 || height == 0 {
            return Err(Error::ZeroSized { width, height });
        }
        if !matches!(bits_per_pixel, 4 | 8) {
            return Err(Error::UnsupportedDepth { bits_per_pixel });
        }

        let pixels = width as usize * height as usize;
        if bits_per_pixel == 4 && !pixels.is_multiple_of(2) {
            return Err(Error::OddPixelCountAt4Bpp { width, height });
        }

        let colours = 1usize << bits_per_pixel;
        let palette_len = colours * 4;
        // Exact, not truncating: the odd 4bpp case is refused above.
        let pixel_len = pixels * bits_per_pixel as usize / 8;

        // `+0x06` is a **mip count**, which this parser assumed was always 1
        // until `Data.wad` produced a counter-example. `psp-texture.md` predicted
        // exactly that ("the size arithmetic above holds for this corpus but
        // would not for a mipmapped texture. Worth re-checking against
        // `Data.wad`"), and the engine-flare sprite is the case:
        // `Data\Tex\EngineFlare\grabbedEngineFlare128x64x8.mip` is 128x64 at 8bpp
        // with `+0x06 == 4`, and 11,920 bytes rather than the 9,232 a single
        // level implies. The chain accounts for it exactly:
        //
        //     16 + 1024 + 8192 + 2048 + 512 + 128 = 11,920
        //
        // A zero is read as one, since a blob with no pixels at all is not a
        // texture worth accepting.
        let mip_levels = data[6].clamp(1, MAX_MIP_LEVELS);
        let expected =
            HEADER_LEN + palette_len + mip_chain_len(width, height, bits_per_pixel, mip_levels);

        // The size is fully determined by the header, so a mismatch means this
        // is not a texture rather than that it is a damaged one.
        if data.len() != expected {
            return Err(Error::SizeMismatch {
                expected,
                got: data.len(),
            });
        }

        let palette = data[HEADER_LEN..HEADER_LEN + palette_len]
            .as_chunks::<4>()
            .0
            .to_vec();

        // Level 0 only. Slicing to `pixel_len` rather than taking the rest of the
        // blob is what keeps `indices` exactly `width * height` long on a
        // mipmapped texture - `to_rgba` feeds `png::encode_rgba`, which asserts
        // that.
        let stored = &data[HEADER_LEN + palette_len..HEADER_LEN + palette_len + pixel_len];
        // Bit 0 of `+0x07` says the pixels are stored in the GE's swizzled
        // layout rather than raster order. Five of `FE.wad`'s thirteen textures
        // set it, and reading those linearly gives noise: `00006_cee1c5a4`
        // decodes to the Japanese Wipeout logotype only once unswizzled.
        let row_bytes = usize::from(width) * usize::from(bits_per_pixel) / 8;
        let swizzled = data[7] & FLAG_SWIZZLED != 0;
        let unswizzled;
        let packed: &[u8] = if swizzled && row_bytes % SWIZZLE_BLOCK_BYTES == 0 {
            unswizzled = unswizzle(stored, row_bytes, usize::from(height));
            &unswizzled
        } else {
            stored
        };
        let indices = if bits_per_pixel == 8 {
            packed.to_vec()
        } else {
            // Low nibble first: pixel 0 is the low half of byte 0. Getting this
            // backwards mirrors every pair of pixels, which is subtle enough to
            // miss on a noisy texture and obvious on text.
            let mut out = Vec::with_capacity(pixel_len * 2);
            for &byte in packed {
                out.push(byte & 0x0f);
                out.push(byte >> 4);
            }
            out
        };

        Ok(Self {
            width,
            height,
            bits_per_pixel,
            palette,
            indices,
            unknown: [data[5], data[6], data[7], data[8]],
            mip_levels,
        })
    }

    /// Expands to RGBA8888, row-major from the top left.
    #[must_use]
    pub fn to_rgba(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.indices.len() * 4);
        for &index in &self.indices {
            let colour = self
                .palette
                .get(index as usize)
                .copied()
                // An out-of-range index cannot happen for 8bpp with a 256-entry
                // palette, but magenta is a louder failure than a silent black.
                .unwrap_or([255, 0, 255, 255]);
            out.extend_from_slice(&colour);
        }
        out
    }

    /// Heuristic for whether the pixel data looks swizzled.
    ///
    /// Swizzled data is stored in 16-byte-wide by 8-row blocks, so decoding it
    /// linearly makes adjacent rows uncorrelated while adjacent *blocks* stay
    /// correlated. This compares how similar vertically-adjacent pixels are
    /// against horizontally-adjacent ones: natural images are similar in both
    /// directions, and a linear read of swizzled data is not.
    ///
    /// **This is a hint, not a determination.** It is here to prioritise
    /// investigation, not to drive decoding.
    #[must_use]
    pub fn looks_swizzled(&self) -> bool {
        let (w, h) = (self.width as usize, self.height as usize);
        if w < 32 || h < 16 {
            return false;
        }

        let diff = |a: usize, b: usize| {
            i32::from(self.indices[a]).abs_diff(i32::from(self.indices[b])) as u64
        };

        let mut horizontal = 0u64;
        let mut vertical = 0u64;
        for y in 0..h - 1 {
            for x in 0..w - 1 {
                horizontal += diff(y * w + x, y * w + x + 1);
                vertical += diff(y * w + x, (y + 1) * w + x);
            }
        }

        // Vertical neighbours much less similar than horizontal ones is the
        // signature of reading swizzled data linearly.
        vertical > horizontal.saturating_mul(3)
    }
}

#[cfg(test)]
mod tests;
