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
//! `Texture_SwizzleForGe` (`0x08926da8` in the PSP executable) converts a linear
//! image into the GE's 16-byte by 8-row block layout, and
//! `Texture_BindEmbeddedData` calls it **only when bit 0 of `flags` is clear**,
//! setting the bit afterwards. Every font atlas ships with that bit already set,
//! so the game skips the conversion: the file holds swizzled data and a reader
//! has to undo it. The same bit lives at `+0x07` of a standalone
//! [`.mip`](crate::texture) header, where it is set on 6 of the 13 in `FE.wad`.
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

pub use crate::texture::{FLAG_SWIZZLED, SWIZZLE_BLOCK_BYTES, SWIZZLE_BLOCK_ROWS};

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
    /// The atlas is swizzled but not a whole number of swizzle blocks.
    UnswizzleableAtlas {
        /// Atlas width.
        width: u16,
        /// Atlas height.
        height: u16,
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
            Self::UnswizzleableAtlas { width, height } => write!(
                f,
                "a swizzled {width}x{height} atlas is not a whole number of \
                 {SWIZZLE_BLOCK_BYTES}x{SWIZZLE_BLOCK_ROWS} blocks"
            ),
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

/// The metrics half of a `.fnt`: everything but the pixels.
///
/// Split out because the PS2 build ships exactly this and keeps its atlas
/// somewhere else - see [`Font::with_atlas`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Metrics {
    /// Distance between baselines, in pixels.
    pub line_height: u32,
    /// The word at `+0x14`, 0 or 4. Not decoded.
    pub unknown: u32,
    /// Where the atlas block begins. Equal to the file's own length when there
    /// is no atlas in the file, which is how the PS2 build stores it.
    pub atlas_at: usize,
    /// Glyphs, in codepoint order.
    pub glyphs: Vec<Glyph>,
}

impl Metrics {
    /// Parses the header, the codepoint table and the glyph records, and stops
    /// before the atlas.
    ///
    /// # Errors
    ///
    /// Fails when the magic is wrong, a declared table runs past the end, or a
    /// glyph record's box disagrees with its declared size.
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

        Ok(Self {
            line_height,
            unknown,
            atlas_at,
            glyphs,
        })
    }

    /// Whether the file carries its own atlas.
    ///
    /// False on every PS2 `.fnt`, where `atlas_at` is the file's own length.
    #[must_use]
    pub fn has_embedded_atlas(&self, data: &[u8]) -> bool {
        self.atlas_at + ATLAS_HEADER_LEN <= data.len()
    }
}

impl Font {
    /// Parses a font whose atlas is inside the file, as the PSP build stores it.
    ///
    /// # Errors
    ///
    /// Every way [`Metrics::parse`] can fail, plus a missing or malformed
    /// atlas: not 4bpp, or a block that is not exactly
    /// `64 + clut_size + width * height / 2` bytes. A PS2 `.fnt` fails here
    /// with [`Error::TooShort`] and needs [`Font::with_atlas`] instead.
    pub fn parse(data: &[u8]) -> Result<Self> {
        let metrics = Metrics::parse(data)?;
        let Metrics {
            line_height,
            unknown,
            atlas_at,
            glyphs,
        } = metrics;

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
            // The block walk only covers whole 16-byte columns and whole 8-row
            // bands. Every shipped atlas is 256 or 512 wide by 128 or 256 tall,
            // so this never fires; refusing beats returning wrong pixels
            // silently, which is the same call `texture` makes for odd 4bpp.
            if row_bytes % SWIZZLE_BLOCK_BYTES != 0 || usize::from(height) % SWIZZLE_BLOCK_ROWS != 0
            {
                return Err(Error::UnswizzleableAtlas { width, height });
            }
            crate::texture::unswizzle(texels, row_bytes, usize::from(height))
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

    /// Builds a font from metrics that carry no pixels and an atlas read from
    /// somewhere else.
    ///
    /// This is the PS2 build: its `.fnt` ends where the atlas would start and
    /// the glyph sheet is its own archive entry, a
    /// [`PSMT4`](crate::ps2_texture::Layout::Psmt4) texture. `indices` is one
    /// palette index per pixel in raster order and `palette` is RGBA8888.
    ///
    /// # Alpha is the caller's to normalise
    ///
    /// A [`crate::ps2_texture`] palette keeps the GS's 0-128 alpha, and
    /// [`Self::alpha_at`] is read as coverage on the usual 0-255 scale, so a
    /// caller handing over a PS2 palette has to double it first -
    /// [`crate::ps2_texture::Ps2Texture::to_rgba`] is where that rule is
    /// written down. Doing it here would be wrong for a palette that was
    /// already full range.
    ///
    /// # Errors
    ///
    /// [`Error::AtlasSizeMismatch`] when `indices` is not `width * height`
    /// long.
    pub fn with_atlas(
        metrics: Metrics,
        width: u16,
        height: u16,
        palette: Vec<[u8; 4]>,
        indices: Vec<u8>,
    ) -> Result<Self> {
        let pixels = usize::from(width) * usize::from(height);
        if indices.len() != pixels {
            return Err(Error::AtlasSizeMismatch {
                expected: pixels,
                got: indices.len(),
            });
        }
        Ok(Self {
            line_height: metrics.line_height,
            unknown: metrics.unknown,
            width,
            height,
            palette,
            indices,
            glyphs: metrics.glyphs,
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
    ///
    /// The glyph's **silhouette**: for the two HUD fonts that includes the baked
    /// outline, so alpha alone does not say where the glyph body ends. See
    /// [`Self::luma_at`].
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

    /// Grey level of the atlas pixel at `(x, y)`, or 0 outside it.
    ///
    /// Every palette entry in every shipped font is a **neutral grey** - `r == g ==
    /// b` on all 16 entries of all five Pulse fonts and all six of Pure's - so this
    /// reads the red channel rather than computing a weighted luminance, and the
    /// choice costs nothing.
    ///
    /// # What it means
    ///
    /// The **body/outline mask**. The three menu fonts are a single pure white, so
    /// this is a constant 255 for them and carries nothing. The two HUD fonts carry
    /// six distinct greys and are pre-outlined: light is glyph body, dark is
    /// outline. A renderer that wants the original's look mixes the text colour
    /// toward the border colour by this and takes its opacity from
    /// [`Self::alpha_at`].
    ///
    /// Reading alpha alone draws body and outline in one colour, which turns an
    /// outlined digit into a filled box - measured, not hypothesised. See
    /// `docs/formats/fnt.md` and `oag_game::font`.
    #[must_use]
    pub fn luma_at(&self, x: usize, y: usize) -> u8 {
        let (w, h) = (usize::from(self.width), usize::from(self.height));
        if x >= w || y >= h {
            return 0;
        }
        self.indices
            .get(y * w + x)
            .and_then(|&i| self.palette.get(usize::from(i)))
            .map_or(0, |c| c[0])
    }

    /// Whether this font bakes an outline into its atlas.
    ///
    /// True when the palette holds more than one distinct grey, which is the
    /// property that distinguishes the two HUD fonts from the three menu ones. A
    /// caller that only wants to know whether [`Self::luma_at`] carries information
    /// can ask this instead of inspecting the palette.
    #[must_use]
    pub fn is_outlined(&self) -> bool {
        let mut seen: Option<u8> = None;
        for entry in &self.palette {
            // Fully transparent entries carry no colour worth comparing: their RGB
            // is never sampled, and in these fonts it is zero regardless.
            if entry[3] == 0 {
                continue;
            }
            match seen {
                None => seen = Some(entry[0]),
                Some(first) if first != entry[0] => return true,
                Some(_) => {}
            }
        }
        false
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
