//! Bitmap fonts (`.fnt`): metrics plus a swizzled 4bpp glyph atlas.
//!
//! ```text
//! +0x00  u8       version, 1
//! +0x01  u8[3]    "FNT"
//! +0x04  u32      codepoint count
//! +0x08  u32      offset of the codepoint table
//! +0x0c  u32      offset of the glyph-record offset table
//! +0x10  u32      line height in pixels
//! +0x14  u32      unknown; 0 in three fonts, 4 in the other two
//! +0x18  u32      offset of the atlas
//! +0x1c  u8[20]   zero
//! ```
//!
//! Then the codepoint table (`count` ascending `u16`), the offset table
//! (`count` `u32`), 18-byte glyph records, and the atlas.
//!
//! See `docs/formats/fnt.md` for the evidence.
//!
//! # The atlas header is 64 bytes, and that was the whole problem
//!
//! It is not a [`crate::texture`] `.mip` header. It is the same **`Texture`
//! node payload** a [`crate::vex`] model embeds, and the data does not start
//! until `+0x40`:
//!
//! ```text
//! +0x00  u16   width
//! +0x02  u16   height
//! +0x04  u8    bits_per_pixel, 4
//! +0x05  u8    mip_count, 1
//! +0x06  u8    flags; bit 0 means the texels are already swizzled
//! +0x07  u8    texture index
//! +0x08  u32   clut_size, 64
//! +0x0c  u32   texel_size, width * height / 2
//! +0x10  ptr   texels, zero at rest
//! +0x14  ptr   clut, zero at rest
//! +0x18  u8[40] zero
//! +0x40  clut, 16 RGBA8888 entries
//! +0x80  texels
//! ```
//!
//! Reading the palette at `+0x10` and the texels at `+0x50`, as a `.mip` header
//! would imply, puts both 48 bytes early. The palette then comes out with 12 of
//! its 16 entries fully transparent - which is the tell, because no font can be
//! drawn with that - and the texels come out shifted by 96 pixels, which no
//! amount of sweeping block geometries can undo. Read at `+0x40` and `+0x80`
//! the palette is a clean 16-level alpha ramp and the atlas is a font.
//!
//! # The texels are stored already swizzled
//!
//! Unusually: every other texture on the disc is stored linearly and swizzled
//! at load. `Texture_SwizzleForGe` (`0x08926da8` in the PSP executable) converts
//! linear to swizzled in 16-byte by 8-row blocks, and `Texture_BindEmbeddedData`
//! calls it **only when bit 0 of `flags` is clear**, setting the bit afterwards.
//! Every font atlas ships with that bit already set, so the game skips the
//! conversion: the file holds swizzled data and a reader has to undo it.
//!
//! # 4bpp packs the left pixel in the low nibble
//!
//! The same order as [`crate::texture`], confirmed the same way: the one 4bpp
//! `.mip` on the disc is a smooth hexagon read low-nibble-first and a combed one
//! read the other way.

/// Bytes of file header before the codepoint table.
pub const HEADER_LEN: usize = 0x30;

/// The magic that follows the version byte.
pub const MAGIC: &[u8; 3] = b"FNT";

/// The only version seen.
pub const VERSION: u8 = 1;

/// Bytes per glyph record.
pub const GLYPH_LEN: usize = 18;

/// Bytes of atlas header before the palette.
pub const ATLAS_HEADER_LEN: usize = 0x40;

/// Bytes of one swizzle block row.
pub const SWIZZLE_BLOCK_BYTES: usize = 16;

/// Rows in one swizzle block.
pub const SWIZZLE_BLOCK_ROWS: usize = 8;

/// `flags` bit meaning the texels are stored swizzled.
pub const FLAG_SWIZZLED: u8 = 1;

/// Something wrong with a font.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Fewer bytes than the header needs.
    TooShort {
        /// Bytes supplied.
        got: usize,
    },
    /// The magic or version was wrong.
    NotAFont,
    /// A declared table or the atlas runs past the end of the file.
    OutOfRange {
        /// What was being read.
        what: &'static str,
        /// Where it claims to start.
        offset: usize,
    },
    /// `bits_per_pixel` was not 4.
    UnsupportedDepth {
        /// The value found.
        bits_per_pixel: u8,
    },
    /// The atlas block is not the size its header implies.
    AtlasSizeMismatch {
        /// Size the header implies.
        expected: usize,
        /// Size available.
        got: usize,
    },
    /// A glyph record's box disagrees with its declared size.
    BoxMismatch {
        /// The codepoint.
        codepoint: u16,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooShort { got } => write!(f, "need at least {HEADER_LEN} bytes, got {got}"),
            Self::NotAFont => write!(f, "not a .fnt (bad version or magic)"),
            Self::OutOfRange { what, offset } => {
                write!(f, "the {what} at {offset} is past the end")
            }
            Self::UnsupportedDepth { bits_per_pixel } => {
                write!(
                    f,
                    "unsupported atlas depth {bits_per_pixel} bpp (expected 4)"
                )
            }
            Self::AtlasSizeMismatch { expected, got } => {
                write!(
                    f,
                    "the atlas header implies {expected} bytes, {got} available"
                )
            }
            Self::BoxMismatch { codepoint } => {
                write!(f, "glyph {codepoint:#06x} has a box that is not its size")
            }
        }
    }
}

impl std::error::Error for Error {}

/// Result alias for this module.
pub type Result<T> = std::result::Result<T, Error>;

/// One glyph's metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Glyph {
    /// The character this draws.
    pub codepoint: u16,
    /// Width in pixels.
    pub width: u8,
    /// Height in pixels.
    pub height: u8,
    /// Left edge in the atlas.
    pub u0: u16,
    /// Right edge, exclusive.
    pub u1: u16,
    /// Top edge in the atlas.
    ///
    /// Shared by every glyph on the same atlas row rather than tight to the
    /// ink, unlike the horizontal pair.
    pub v0: u16,
    /// Bottom edge, exclusive.
    pub v1: u16,
    /// How far the pen moves after drawing this.
    pub advance: u8,
}

/// A decoded font.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Font {
    /// Distance between baselines, in pixels.
    pub line_height: u32,
    /// The word at `+0x14`, 0 or 4. Not decoded.
    pub unknown: u32,
    /// Atlas width in pixels.
    pub width: u16,
    /// Atlas height in pixels.
    pub height: u16,
    /// Atlas palette, 16 RGBA8888 entries.
    pub palette: Vec<[u8; 4]>,
    /// One palette index per atlas pixel, unswizzled into raster order.
    pub indices: Vec<u8>,
    /// Glyphs, in codepoint order.
    pub glyphs: Vec<Glyph>,
}

fn word(data: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]])
}

fn half(data: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([data[at], data[at + 1]])
}

/// Whether `data` starts with the `.fnt` version and magic.
#[must_use]
pub fn looks_like_font(data: &[u8]) -> bool {
    data.len() >= 4 && data[0] == VERSION && &data[1..4] == MAGIC
}

/// Undoes the PSP's 16-byte by 8-row block swizzle.
///
/// `row_bytes` is the image's width in **bytes**, so half the pixel width at
/// 4bpp. This is the exact inverse of `Texture_SwizzleForGe`: that function
/// walks block rows, then block columns, then the 8 rows within a block,
/// copying 16 bytes at a time out of a linear image.
#[must_use]
pub fn unswizzle(src: &[u8], row_bytes: usize, height: usize) -> Vec<u8> {
    let mut dst = vec![0u8; src.len()];
    if row_bytes % SWIZZLE_BLOCK_BYTES != 0 || height % SWIZZLE_BLOCK_ROWS != 0 {
        dst.copy_from_slice(src);
        return dst;
    }
    let mut read = 0;
    for block_row in 0..height / SWIZZLE_BLOCK_ROWS {
        for block_col in 0..row_bytes / SWIZZLE_BLOCK_BYTES {
            for row in 0..SWIZZLE_BLOCK_ROWS {
                let at = (block_row * SWIZZLE_BLOCK_ROWS + row) * row_bytes
                    + block_col * SWIZZLE_BLOCK_BYTES;
                let Some(chunk) = src.get(read..read + SWIZZLE_BLOCK_BYTES) else {
                    return dst;
                };
                dst[at..at + SWIZZLE_BLOCK_BYTES].copy_from_slice(chunk);
                read += SWIZZLE_BLOCK_BYTES;
            }
        }
    }
    dst
}

impl Font {
    /// Parses a font.
    ///
    /// # Errors
    ///
    /// Fails when the magic is wrong, a declared offset runs past the end, the
    /// atlas is not 4bpp, or the atlas block is not exactly
    /// `64 + clut_size + width * height / 2` bytes.
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < HEADER_LEN {
            return Err(Error::TooShort { got: data.len() });
        }
        if !looks_like_font(data) {
            return Err(Error::NotAFont);
        }

        let count = word(data, 0x04) as usize;
        let codepoints_at = word(data, 0x08) as usize;
        let offsets_at = word(data, 0x0c) as usize;
        let line_height = word(data, 0x10);
        let unknown = word(data, 0x14);
        let atlas_at = word(data, 0x18) as usize;

        let range = |what: &'static str, at: usize, len: usize| -> Result<&[u8]> {
            data.get(at..at + len)
                .ok_or(Error::OutOfRange { what, offset: at })
        };
        let codepoints = range("codepoint table", codepoints_at, count * 2)?;
        let offsets = range("offset table", offsets_at, count * 4)?;

        let mut glyphs = Vec::with_capacity(count);
        for index in 0..count {
            // A terminating 0x0000 is present in three of the five fonts and
            // has no record behind it, so it ends the list rather than being
            // an error.
            if half(codepoints, index * 2) == 0 {
                break;
            }
            let at = word(offsets, index * 4) as usize;
            let record = range("glyph record", at, GLYPH_LEN)?;
            let glyph = Glyph {
                codepoint: half(record, 0),
                width: record[2],
                height: record[3],
                u0: half(record, 4),
                u1: half(record, 6),
                v0: half(record, 8),
                v1: half(record, 10),
                advance: record[12],
            };
            // The box and the size are stored separately, so they agree or this
            // is not a glyph record.
            if u16::from(glyph.width) != glyph.u1.wrapping_sub(glyph.u0)
                || u16::from(glyph.height) != glyph.v1.wrapping_sub(glyph.v0)
            {
                return Err(Error::BoxMismatch {
                    codepoint: glyph.codepoint,
                });
            }
            glyphs.push(glyph);
        }

        let atlas = data.get(atlas_at..).ok_or(Error::OutOfRange {
            what: "atlas",
            offset: atlas_at,
        })?;
        if atlas.len() < ATLAS_HEADER_LEN {
            return Err(Error::TooShort { got: atlas.len() });
        }
        let width = half(atlas, 0);
        let height = half(atlas, 2);
        let bits_per_pixel = atlas[4];
        let flags = atlas[6];
        let clut_size = word(atlas, 8) as usize;
        let texel_size = word(atlas, 0x0c) as usize;

        if bits_per_pixel != 4 {
            return Err(Error::UnsupportedDepth { bits_per_pixel });
        }
        let pixels = usize::from(width) * usize::from(height);
        let expected = ATLAS_HEADER_LEN + clut_size + texel_size;
        // The block closes exactly, with no padding. That is what pins the
        // header at 64 bytes rather than 16.
        if texel_size != pixels / 2 || clut_size != 4 << bits_per_pixel || atlas.len() != expected {
            return Err(Error::AtlasSizeMismatch {
                expected,
                got: atlas.len(),
            });
        }

        let palette = atlas[ATLAS_HEADER_LEN..ATLAS_HEADER_LEN + clut_size]
            .chunks_exact(4)
            .map(|c| [c[0], c[1], c[2], c[3]])
            .collect();

        let texels = &atlas[ATLAS_HEADER_LEN + clut_size..];
        let row_bytes = usize::from(width) / 2;
        let linear = if flags & FLAG_SWIZZLED == 0 {
            texels.to_vec()
        } else {
            unswizzle(texels, row_bytes, usize::from(height))
        };

        // Low nibble first: pixel 0 is the low half of byte 0, as in `.mip`.
        let mut indices = Vec::with_capacity(pixels);
        for &byte in &linear {
            indices.push(byte & 0x0f);
            indices.push(byte >> 4);
        }

        Ok(Self {
            line_height,
            unknown,
            width,
            height,
            palette,
            indices,
            glyphs,
        })
    }

    /// Expands the atlas to RGBA8888, row-major from the top left.
    #[must_use]
    pub fn to_rgba(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.indices.len() * 4);
        for &index in &self.indices {
            let colour = self
                .palette
                .get(index as usize)
                .copied()
                .unwrap_or([255, 0, 255, 255]);
            out.extend_from_slice(&colour);
        }
        out
    }

    /// Alpha of the atlas pixel at `(x, y)`, or 0 outside it.
    #[must_use]
    pub fn alpha_at(&self, x: usize, y: usize) -> u8 {
        let (w, h) = (usize::from(self.width), usize::from(self.height));
        if x >= w || y >= h {
            return 0;
        }
        self.indices
            .get(y * w + x)
            .and_then(|&i| self.palette.get(usize::from(i)))
            .map_or(0, |c| c[3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a font by hand. No game data in any test.
    fn font(width: u16, height: u16, glyphs: &[(u16, u8, u8, u16, u16, u16, u16)]) -> Vec<u8> {
        let count = glyphs.len();
        let codepoints_at = HEADER_LEN;
        let offsets_at = codepoints_at + count * 2;
        let records_at = offsets_at + count * 4;
        let atlas_at = records_at + count * GLYPH_LEN;

        let mut out = vec![0u8; HEADER_LEN];
        out[0] = VERSION;
        out[1..4].copy_from_slice(MAGIC);
        out[4..8].copy_from_slice(&(count as u32).to_le_bytes());
        out[8..12].copy_from_slice(&(codepoints_at as u32).to_le_bytes());
        out[12..16].copy_from_slice(&(offsets_at as u32).to_le_bytes());
        out[16..20].copy_from_slice(&13u32.to_le_bytes());
        out[24..28].copy_from_slice(&(atlas_at as u32).to_le_bytes());

        for g in glyphs {
            out.extend_from_slice(&g.0.to_le_bytes());
        }
        for i in 0..count {
            out.extend_from_slice(&((records_at + i * GLYPH_LEN) as u32).to_le_bytes());
        }
        for g in glyphs {
            out.extend_from_slice(&g.0.to_le_bytes());
            out.push(g.1);
            out.push(g.2);
            out.extend_from_slice(&g.3.to_le_bytes());
            out.extend_from_slice(&g.4.to_le_bytes());
            out.extend_from_slice(&g.5.to_le_bytes());
            out.extend_from_slice(&g.6.to_le_bytes());
            out.push(8);
            out.extend_from_slice(&[0xff; 5]);
        }

        let texel_size = usize::from(width) * usize::from(height) / 2;
        let mut atlas = vec![0u8; ATLAS_HEADER_LEN];
        atlas[0..2].copy_from_slice(&width.to_le_bytes());
        atlas[2..4].copy_from_slice(&height.to_le_bytes());
        atlas[4] = 4;
        atlas[5] = 1;
        atlas[6] = FLAG_SWIZZLED;
        atlas[8..12].copy_from_slice(&64u32.to_le_bytes());
        atlas[12..16].copy_from_slice(&(texel_size as u32).to_le_bytes());
        for i in 0..16u8 {
            atlas.extend_from_slice(&[255, 255, 255, i * 17]);
        }
        // Swizzled texels whose linear form is a horizontal ramp per row.
        let row_bytes = usize::from(width) / 2;
        let mut linear = vec![0u8; texel_size];
        for y in 0..usize::from(height) {
            for x in 0..row_bytes {
                linear[y * row_bytes + x] = ((x + y) % 251) as u8;
            }
        }
        let mut swizzled = vec![0u8; texel_size];
        let mut write = 0;
        for br in 0..usize::from(height) / SWIZZLE_BLOCK_ROWS {
            for bc in 0..row_bytes / SWIZZLE_BLOCK_BYTES {
                for r in 0..SWIZZLE_BLOCK_ROWS {
                    let at = (br * SWIZZLE_BLOCK_ROWS + r) * row_bytes + bc * SWIZZLE_BLOCK_BYTES;
                    swizzled[write..write + SWIZZLE_BLOCK_BYTES]
                        .copy_from_slice(&linear[at..at + SWIZZLE_BLOCK_BYTES]);
                    write += SWIZZLE_BLOCK_BYTES;
                }
            }
        }
        atlas.extend_from_slice(&swizzled);
        out.extend_from_slice(&atlas);
        out
    }

    #[test]
    fn a_font_parses_and_its_atlas_closes() {
        let data = font(64, 16, &[(65, 4, 8, 0, 4, 0, 8), (66, 5, 8, 6, 11, 0, 8)]);
        let parsed = Font::parse(&data).expect("parse");
        assert_eq!(parsed.line_height, 13);
        assert_eq!(parsed.width, 64);
        assert_eq!(parsed.height, 16);
        assert_eq!(parsed.palette.len(), 16);
        assert_eq!(parsed.indices.len(), 64 * 16);
        assert_eq!(parsed.glyphs.len(), 2);
        assert_eq!(parsed.glyphs[1].codepoint, 66);
        assert_eq!(parsed.glyphs[1].advance, 8);
    }

    #[test]
    fn the_swizzle_is_undone() {
        let data = font(64, 16, &[(65, 4, 8, 0, 4, 0, 8)]);
        let parsed = Font::parse(&data).expect("parse");
        // The fixture's linear image is `(x + y) % 251` per byte, so pixel
        // 2x of row y is the low nibble of that.
        for y in 0..16usize {
            for x in 0..32usize {
                let byte = ((x + y) % 251) as u8;
                assert_eq!(parsed.indices[y * 64 + x * 2], byte & 0x0f, "({x},{y}) low");
                assert_eq!(
                    parsed.indices[y * 64 + x * 2 + 1],
                    byte >> 4,
                    "({x},{y}) high"
                );
            }
        }
    }

    #[test]
    fn unswizzle_is_a_permutation() {
        for (row_bytes, height) in [(16usize, 8usize), (128, 128), (256, 256), (32, 64)] {
            let src: Vec<u8> = (0..row_bytes * height).map(|i| (i % 251) as u8).collect();
            let out = unswizzle(&src, row_bytes, height);
            let mut a = src.clone();
            let mut b = out.clone();
            a.sort_unstable();
            b.sort_unstable();
            assert_eq!(a, b, "{row_bytes}x{height}");
        }
    }

    #[test]
    fn an_unswizzled_atlas_is_taken_verbatim() {
        let mut data = font(64, 16, &[(65, 4, 8, 0, 4, 0, 8)]);
        let atlas_at = data.len() - (ATLAS_HEADER_LEN + 64 + 64 * 16 / 2);
        data[atlas_at + 6] = 0;
        let parsed = Font::parse(&data).expect("parse");
        // The fixture's stored bytes are swizzled, so taking them verbatim puts
        // stored byte 16 - which is the block's second row, value 1 - at linear
        // byte 16, i.e. pixel 32 of row 0. Unswizzled it would be pixel 0 of
        // row 1 instead.
        assert_eq!(parsed.indices[32], 1);
        let swizzled = Font::parse(&font(64, 16, &[(65, 4, 8, 0, 4, 0, 8)])).expect("parse");
        assert_eq!(swizzled.indices[64], 1);
        assert_ne!(swizzled.indices, parsed.indices);
    }

    #[test]
    fn a_box_that_disagrees_with_its_size_is_refused() {
        let data = font(64, 16, &[(65, 9, 8, 0, 4, 0, 8)]);
        assert_eq!(
            Font::parse(&data),
            Err(Error::BoxMismatch { codepoint: 65 })
        );
    }

    #[test]
    fn a_wrong_atlas_size_is_refused() {
        let mut data = font(64, 16, &[(65, 4, 8, 0, 4, 0, 8)]);
        data.push(0);
        assert!(matches!(
            Font::parse(&data),
            Err(Error::AtlasSizeMismatch { .. })
        ));
    }

    #[test]
    fn a_non_font_is_refused() {
        assert_eq!(Font::parse(&[0u8; 64]), Err(Error::NotAFont));
        assert!(matches!(
            Font::parse(&[1u8, 70, 78]),
            Err(Error::TooShort { .. })
        ));
    }
}
