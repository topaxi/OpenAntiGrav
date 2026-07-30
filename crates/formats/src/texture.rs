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

/// Bytes of header before the palette.
pub const HEADER_LEN: usize = 16;

/// Bit 0 of `+0x07`: the pixels are stored swizzled for the GE.
///
/// `Texture_BindEmbeddedData` in the PSP executable copies this bit into the
/// texture node's own flags, and swizzles at load **only** when it is clear.
/// See `docs/formats/psp-texture.md`.
pub const FLAG_SWIZZLED: u8 = 1;

/// Bytes of one swizzle block row.
pub const SWIZZLE_BLOCK_BYTES: usize = 16;

/// Rows in one swizzle block.
pub const SWIZZLE_BLOCK_ROWS: usize = 8;

/// Undoes the GE's 16-byte by 8-row block swizzle.
///
/// `row_bytes` is the image's width in bytes. This is the exact inverse of
/// `Texture_SwizzleForGe` (`0x08926da8`), which walks block rows, then block
/// columns, then the 8 rows within a block, copying 16 bytes at a time out of a
/// linear image.
///
/// A `row_bytes` of 16 or a `height` under 8 makes the swizzle the identity,
/// which is why the disc's one 4bpp texture cannot tell the two readings apart.
#[must_use]
pub fn unswizzle(src: &[u8], row_bytes: usize, height: usize) -> Vec<u8> {
    let mut dst = vec![0u8; src.len()];
    if !row_bytes.is_multiple_of(SWIZZLE_BLOCK_BYTES) || row_bytes == 0 {
        dst.copy_from_slice(src);
        return dst;
    }
    let mut read = 0;
    for block_row in 0..height / SWIZZLE_BLOCK_ROWS {
        for block_col in 0..row_bytes / SWIZZLE_BLOCK_BYTES {
            for row in 0..SWIZZLE_BLOCK_ROWS {
                let at = (block_row * SWIZZLE_BLOCK_ROWS + row) * row_bytes
                    + block_col * SWIZZLE_BLOCK_BYTES;
                let (Some(chunk), true) = (
                    src.get(read..read + SWIZZLE_BLOCK_BYTES),
                    at + SWIZZLE_BLOCK_BYTES <= dst.len(),
                ) else {
                    return dst;
                };
                dst[at..at + SWIZZLE_BLOCK_BYTES].copy_from_slice(chunk);
                read += SWIZZLE_BLOCK_BYTES;
            }
        }
    }
    // A height that is not a multiple of 8 leaves a tail the block walk never
    // reaches; copy it through rather than leaving it black.
    let done = (height / SWIZZLE_BLOCK_ROWS) * SWIZZLE_BLOCK_ROWS * row_bytes;
    if done < src.len() {
        dst[done..].copy_from_slice(&src[done..]);
    }
    dst
}

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
            .chunks_exact(4)
            .map(|c| [c[0], c[1], c[2], c[3]])
            .collect();

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
mod tests {
    use super::*;

    /// Builds a texture blob by hand. No game data in any test.
    fn build(width: u16, height: u16, bpp: u8, fill: u8) -> Vec<u8> {
        let colours = 1usize << bpp;
        let mut out = Vec::new();
        out.extend(width.to_le_bytes());
        out.extend(height.to_le_bytes());
        out.push(bpp);
        out.extend([0u8, 1, 2, 0]);
        out.extend([0u8; 7]);
        assert_eq!(out.len(), HEADER_LEN);

        for i in 0..colours {
            out.extend([i as u8, 0, 0, 255]);
        }
        out.extend(std::iter::repeat_n(
            fill,
            width as usize * height as usize * bpp as usize / 8,
        ));
        out
    }

    /// `+0x06` is a mip count, and a chain of them still decodes to level 0.
    ///
    /// This exists because the parser assumed one level for a long time and
    /// `psp-texture.md` flagged the assumption as untested against `Data.wad`.
    /// The engine-flare sprite is the counter-example: 128x64 at 8bpp with
    /// `+0x06 == 4` and 11,920 bytes, where one level implies 9,232. The shape is
    /// reproduced here synthetically - no game data in any test.
    #[test]
    fn a_mipmapped_blob_declares_its_chain_and_decodes_level_zero() {
        let (w, h) = (128u16, 64u16);
        let mut blob = build(w, h, 8, 0);
        blob[6] = 4;
        // Levels 1..3, appended after the level-0 pixels `build` already wrote.
        // Every row here is already at least 16 bytes, so no padding applies -
        // which is exactly why the flare sprite's length is unaffected by it.
        for (lw, lh) in [(64usize, 32usize), (32, 16), (16, 8)] {
            blob.extend(std::iter::repeat_n(0u8, lw * lh));
        }

        // The exact length the disc's own sprite has, arrived at independently.
        assert_eq!(blob.len(), 11_920);

        let parsed = Texture::parse(&blob).expect("a mipmapped texture must parse");
        assert_eq!(parsed.mip_levels, 4);
        assert_eq!(parsed.width, w);
        assert_eq!(parsed.height, h);
        // Level 0 only: the extra levels must not lengthen `indices`, or
        // `png::encode_rgba` would reject the buffer for its dimensions.
        assert_eq!(parsed.indices.len(), usize::from(w) * usize::from(h));

        // Truncating the chain must be refused rather than read as a shorter
        // texture, which is the property that keeps `looks_like_texture` strong.
        blob.truncate(blob.len() - 1);
        assert!(matches!(
            Texture::parse(&blob),
            Err(Error::SizeMismatch { .. })
        ));
    }

    /// A tail level narrower than 16 bytes pads its rows to 16.
    ///
    /// This is the rule the noise texture forced:
    /// `Data\Tex\engineFlare\Engine_noise.mip` is 64x64 at 8bpp with 4 levels
    /// and **6,544** bytes, where an unpadded chain implies 6,480. The whole
    /// 64-byte difference is level 3, an 8x8 whose 8-byte rows pad to 16.
    #[test]
    fn a_narrow_tail_level_pads_its_rows_to_sixteen_bytes() {
        let mut blob = build(64, 64, 8, 0);
        blob[6] = 4;
        for (lw, lh) in [(32usize, 32usize), (16, 16)] {
            blob.extend(std::iter::repeat_n(0u8, lw * lh));
        }
        // Level 3 is 8x8: 8 rows of a 16-byte stride, not of an 8-byte one.
        blob.extend(std::iter::repeat_n(0u8, 16 * 8));

        assert_eq!(blob.len(), 6_544, "the length the disc's own texture has");
        let parsed = Texture::parse(&blob).expect("a padded chain must parse");
        assert_eq!(parsed.mip_levels, 4);
        assert_eq!(parsed.indices.len(), 64 * 64);

        // Without the padding the blob is 64 bytes shorter, and must be refused -
        // that is what makes the rule load-bearing rather than cosmetic.
        let mut unpadded = build(64, 64, 8, 0);
        unpadded[6] = 4;
        for n in [32 * 32usize, 16 * 16, 8 * 8] {
            unpadded.extend(std::iter::repeat_n(0u8, n));
        }
        assert_eq!(unpadded.len(), 6_480);
        assert!(matches!(
            Texture::parse(&unpadded),
            Err(Error::SizeMismatch { .. })
        ));
    }

    /// A single-level blob is unaffected by the mip arithmetic.
    ///
    /// The 346 standalone `.mip` entries that already decoded must keep decoding;
    /// this pins that the change only widened what parses.
    #[test]
    fn one_level_is_still_the_plain_case() {
        for (w, h, bpp) in [(64u16, 16u16, 8u8), (32, 32, 4), (8, 8, 8)] {
            let blob = build(w, h, bpp, 0);
            let parsed = Texture::parse(&blob).expect("parse");
            assert_eq!(parsed.mip_levels, 1);
            assert_eq!(parsed.indices.len(), usize::from(w) * usize::from(h));
        }
    }

    #[test]
    fn unswizzle_is_a_permutation_and_its_own_documented_inverse() {
        for (row_bytes, height) in [
            (16usize, 8usize),
            (128, 128),
            (256, 256),
            (32, 64),
            (512, 128),
        ] {
            let src: Vec<u8> = (0..row_bytes * height).map(|i| (i % 251) as u8).collect();
            let out = unswizzle(&src, row_bytes, height);
            let mut a = src.clone();
            let mut b = out.clone();
            a.sort_unstable();
            b.sort_unstable();
            assert_eq!(a, b, "{row_bytes}x{height} is not a permutation");
        }
        // One block column makes the swizzle the identity, which is exactly why
        // the disc's 32x32 4bpp texture cannot distinguish the two readings.
        let src: Vec<u8> = (0..16 * 32).map(|i| (i % 251) as u8).collect();
        assert_eq!(unswizzle(&src, 16, 32), src);
    }

    #[test]
    fn a_swizzled_blob_is_unswizzled_on_parse() {
        let mut blob = build(64, 16, 8, 0);
        blob[7] = 3; // bit 0 set: stored swizzled
        let pixels = 64 * 16;
        let start = HEADER_LEN + 256 * 4;
        for i in 0..pixels {
            blob[start + i] = (i % 251) as u8;
        }
        let parsed = Texture::parse(&blob).expect("parse");
        let expected = unswizzle(&blob[start..start + pixels], 64, 16);
        assert_eq!(parsed.indices, expected);
        assert_ne!(parsed.indices, blob[start..start + pixels].to_vec());

        blob[7] = 2; // bit 0 clear: stored linear
        let linear = Texture::parse(&blob).expect("parse");
        assert_eq!(linear.indices, blob[start..start + pixels].to_vec());
    }

    /// The case that reached [`crate::png::encode_rgba`]'s assertion.
    ///
    /// A 3x1 4bpp blob truncates to one packed byte, which unpacks to two
    /// indices for three pixels, and `to_rgba` then produces 8 bytes where the
    /// dimensions call for 12. Refusing at parse keeps that gap from existing.
    #[test]
    fn rejects_4bpp_with_an_odd_pixel_count() {
        let mut blob = Vec::new();
        blob.extend(3u16.to_le_bytes());
        blob.extend(1u16.to_le_bytes());
        blob.push(4);
        blob.extend([0u8, 1, 2, 0]);
        blob.extend([0u8; 7]);
        blob.extend(std::iter::repeat_n(0u8, 16 * 4));
        blob.push(0x12);

        assert_eq!(
            Texture::parse(&blob),
            Err(Error::OddPixelCountAt4Bpp {
                width: 3,
                height: 1
            })
        );
    }

    /// The invariant `to_rgba` and the PNG encoder both depend on.
    #[test]
    fn indices_always_hold_one_entry_per_pixel() {
        for (w, h, bpp) in [(32u16, 16u16, 8u8), (32, 32, 4), (1, 1, 8), (2, 1, 4)] {
            let t = Texture::parse(&build(w, h, bpp, 0x11)).expect("parse");
            let pixels = usize::from(w) * usize::from(h);
            assert_eq!(t.indices.len(), pixels, "{w}x{h} at {bpp}bpp");
            assert_eq!(t.to_rgba().len(), pixels * 4);
        }
    }

    #[test]
    fn parses_an_8bpp_texture() {
        let t = Texture::parse(&build(32, 16, 8, 3)).unwrap();
        assert_eq!((t.width, t.height, t.bits_per_pixel), (32, 16, 8));
        assert_eq!(t.palette.len(), 256);
        assert_eq!(t.indices.len(), 32 * 16);
        assert!(t.indices.iter().all(|&i| i == 3));
    }

    #[test]
    fn parses_a_4bpp_texture_and_unpacks_nibbles() {
        // 0xAB unpacks to index 0xB then 0xA: low nibble is the first pixel.
        let t = Texture::parse(&build(32, 32, 4, 0xAB)).unwrap();
        assert_eq!(t.palette.len(), 16);
        assert_eq!(t.indices.len(), 32 * 32);
        assert_eq!(&t.indices[..4], &[0xB, 0xA, 0xB, 0xA]);
    }

    #[test]
    fn the_4bpp_size_matches_the_real_outlier() {
        // 16 + 16*4 + 32*32/2 = 592, the size of the one 4bpp blob in FE.wad.
        assert_eq!(build(32, 32, 4, 0).len(), 592);
    }

    #[test]
    fn the_8bpp_size_matches_a_real_texture() {
        // 16 + 256*4 + 512*128 = 66576.
        assert_eq!(build(512, 128, 8, 0).len(), 66_576);
    }

    #[test]
    fn rejects_a_blob_of_the_wrong_size() {
        let mut data = build(32, 32, 8, 0);
        data.push(0);
        assert!(matches!(
            Texture::parse(&data),
            Err(Error::SizeMismatch { .. })
        ));
    }

    #[test]
    fn rejects_an_unsupported_depth() {
        let mut data = build(32, 32, 8, 0);
        data[4] = 16;
        assert!(matches!(
            Texture::parse(&data),
            Err(Error::UnsupportedDepth { bits_per_pixel: 16 })
        ));
    }

    #[test]
    fn rejects_a_zero_dimension() {
        let mut data = build(32, 32, 8, 0);
        data[0..2].copy_from_slice(&0u16.to_le_bytes());
        assert!(matches!(
            Texture::parse(&data),
            Err(Error::ZeroSized { .. })
        ));
    }

    #[test]
    fn rejects_something_that_is_not_a_texture() {
        // A .vex model starts with its version word; it must not parse.
        let vex = [6u8, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14];
        assert!(!Texture::looks_like_texture(&vex));
    }

    #[test]
    fn expands_to_rgba() {
        let t = Texture::parse(&build(32, 16, 8, 5)).unwrap();
        let rgba = t.to_rgba();
        assert_eq!(rgba.len(), 32 * 16 * 4);
        assert_eq!(&rgba[..4], &[5, 0, 0, 255]);
    }

    #[test]
    fn a_flat_image_does_not_look_swizzled() {
        let t = Texture::parse(&build(64, 64, 8, 7)).unwrap();
        assert!(!t.looks_swizzled());
    }
}
