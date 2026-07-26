//! Palette-indexed textures (`.mip`).
//!
//! ```text
//! +0x00  u16      width
//! +0x02  u16      height
//! +0x04  u8       bits_per_pixel      4 or 8
//! +0x05  u8       unk_0x05            0 in every standalone .mip seen
//! +0x06  u8       unk_0x06            1 in every standalone .mip seen
//! +0x07  u8       unk_0x07            2 or 3
//! +0x08  u8       unk_0x08            0 or 1
//! +0x09  u8[7]    zero
//! +0x10  RGBA8888 palette, 1 << bits_per_pixel entries
//!        indices, width * height * bits_per_pixel / 8 bytes
//! ```
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
        if bits_per_pixel == 4 && pixels % 2 != 0 {
            return Err(Error::OddPixelCountAt4Bpp { width, height });
        }

        let colours = 1usize << bits_per_pixel;
        let palette_len = colours * 4;
        // Exact, not truncating: the odd 4bpp case is refused above.
        let pixel_len = pixels * bits_per_pixel as usize / 8;
        let expected = HEADER_LEN + palette_len + pixel_len;

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

        let packed = &data[HEADER_LEN + palette_len..];
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
