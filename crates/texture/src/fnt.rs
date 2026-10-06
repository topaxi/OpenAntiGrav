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
//! # Wipeout HD writes the same layout big-endian
//!
//! Every field above, the glyph records and atlas header included, with nothing
//! moved. The first four bytes are one `u32` constant in the file's own order
//! (`\x01FNT` on PSP and PS2, `TNF\x01` on PS3), so [`byte_order`] sniffs it
//! and no caller passes a platform in, as `oag_vex::vex::byte_order` does.
//!
//! This is not a word swap: HD's codepoint table reads `00 20 00 21 00 22`,
//! ascending as big-endian `u16`s, and every offset in `pulsehud.fnt` closes on
//! that reading (`docs/formats/fnt.md`). Two HD atlas traits that are *not* byte
//! order and would each be a silently wrong picture:
//!
//! - **`flags` is 0**, so the texels are linear. The unswizzle keys on the bit,
//!   not on a console, so it skips itself.
//! - **The palette's alpha is full-range 0-255** (`0xd9`, `0xf6`, `0xfe`), so the
//!   PS2's 0-128 doubling in `oag_assets::Archives::read_font` must not reach
//!   it. It cannot: that is the no-embedded-atlas branch.
//!
//! # The atlas header is 64 bytes
//!
//! It is not a [`crate::texture`] `.mip` header but the same **`Texture` node
//! payload** a `oag_vex::vex` model embeds; data starts at `+0x40`:
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
//! A `.mip` reading (palette `+0x10`, texels `+0x50`) is 48 bytes early: 12 of 16
//! palette entries come out transparent. At `+0x40`/`+0x80` it is a clean 16-level
//! alpha ramp.
//!
//! # The texels are stored already swizzled
//!
//! `Texture_SwizzleForGe` (`0x08926da8` in the PSP executable) converts a linear
//! image into the GE's 16-byte by 8-row block layout, and
//! `Texture_BindEmbeddedData` calls it **only when bit 0 of `flags` is clear**,
//! setting the bit afterwards. Every font atlas ships with the bit set, so the
//! file holds swizzled data and a reader must undo it. The same bit is at `+0x07`
//! of a standalone [`.mip`](crate::texture) header, set on 6 of the 13 in
//! `FE.wad`.
//!
//! # 4bpp packs the left pixel in the low nibble
//!
//! As in [`crate::texture`]: the one 4bpp `.mip` on the disc is a smooth hexagon
//! read low-nibble-first and a combed one read the other way.
//!
//! **Byte order does not change it (measured).** Each glyph's box must hold no ink
//! in the column left of `u0` or at `u1`: HD's three Latin faces spill **0** rows
//! low-nibble-first against 1,776 to 5,490 the other way (table in
//! `docs/formats/fnt.md`). A one-pixel pair swap still reads as a font, so the
//! boxes are the test.

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

pub use oag_formats::swizzle::{FLAG_SWIZZLED, SWIZZLE_BLOCK_BYTES, SWIZZLE_BLOCK_ROWS};

use oag_formats::ByteOrder;

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
    /// Shared by every glyph on the same atlas row, unlike the horizontal pair.
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

/// Which way round this `.fnt` is written, or `None` if it is not one.
///
/// The first four bytes are one `u32` constant in the file's own order:
/// `\x01FNT` little-endian, `TNF\x01` big-endian. Nothing else is consulted,
/// as in `oag_vex::vex::byte_order`.
#[must_use]
pub fn byte_order(data: &[u8]) -> Option<ByteOrder> {
    let head = data.get(..4)?;
    if head[0] == VERSION && &head[1..] == MAGIC {
        return Some(ByteOrder::Little);
    }
    if head[3] == VERSION && head[..3].iter().rev().eq(MAGIC) {
        return Some(ByteOrder::Big);
    }
    None
}

/// Whether `data` starts with the `.fnt` version and magic, either way round.
#[must_use]
pub fn looks_like_font(data: &[u8]) -> bool {
    byte_order(data).is_some()
}

/// The metrics half of a `.fnt`: everything but the pixels.
///
/// The PS2 build ships exactly this and keeps its atlas elsewhere; see
/// [`Font::with_atlas`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Metrics {
    /// Which way round the file is written, from its own magic.
    ///
    /// Kept because the atlas header shares it and has no magic of its own.
    pub order: ByteOrder,
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
    /// Parses the header, codepoint table and glyph records, stopping before the atlas.
    ///
    /// # Errors
    ///
    /// The magic is wrong, a declared table runs past the end, or a glyph
    /// record's box disagrees with its declared size.
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < HEADER_LEN {
            return Err(Error::TooShort { got: data.len() });
        }
        let Some(order) = byte_order(data) else {
            return Err(Error::NotAFont);
        };

        let count = order.u32(data, 0x04) as usize;
        let codepoints_at = order.u32(data, 0x08) as usize;
        let offsets_at = order.u32(data, 0x0c) as usize;
        let line_height = order.u32(data, 0x10);
        let unknown = order.u32(data, 0x14);
        let atlas_at = order.u32(data, 0x18) as usize;

        let range = |what: &'static str, at: usize, len: usize| -> Result<&[u8]> {
            data.get(at..at + len)
                .ok_or(Error::OutOfRange { what, offset: at })
        };
        let codepoints = range("codepoint table", codepoints_at, count * 2)?;
        let offsets = range("offset table", offsets_at, count * 4)?;

        let mut glyphs = Vec::with_capacity(count);
        for index in 0..count {
            // A terminating 0x0000 (three of five fonts) has no record behind it.
            if order.u16(codepoints, index * 2) == 0 {
                break;
            }
            let at = order.u32(offsets, index * 4) as usize;
            let record = range("glyph record", at, GLYPH_LEN)?;
            let glyph = Glyph {
                codepoint: order.u16(record, 0),
                width: record[2],
                height: record[3],
                u0: order.u16(record, 4),
                u1: order.u16(record, 6),
                v0: order.u16(record, 8),
                v1: order.u16(record, 10),
                advance: record[12],
            };
            // Box and size are stored separately; they must agree.
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
            order,
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
            order,
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
        // The atlas header inherits the file's byte order; its size fields
        // closing on each other below proves it.
        let width = order.u16(atlas, 0);
        let height = order.u16(atlas, 2);
        let bits_per_pixel = atlas[4];
        let flags = atlas[6];
        let clut_size = order.u32(atlas, 8) as usize;
        let texel_size = order.u32(atlas, 0x0c) as usize;

        if bits_per_pixel != 4 {
            return Err(Error::UnsupportedDepth { bits_per_pixel });
        }
        let pixels = usize::from(width) * usize::from(height);
        let expected = ATLAS_HEADER_LEN + clut_size + texel_size;
        // The block closes exactly with no padding, which pins the header at 64 bytes.
        if texel_size != pixels / 2 || clut_size != 4 << bits_per_pixel || atlas.len() != expected {
            return Err(Error::AtlasSizeMismatch {
                expected,
                got: atlas.len(),
            });
        }

        let palette = atlas[ATLAS_HEADER_LEN..ATLAS_HEADER_LEN + clut_size]
            .as_chunks::<4>()
            .0
            .to_vec();

        let texels = &atlas[ATLAS_HEADER_LEN + clut_size..];
        let row_bytes = usize::from(width) / 2;
        let linear = if flags & FLAG_SWIZZLED == 0 {
            texels.to_vec()
        } else {
            // The walk covers whole 16-byte columns and 8-row bands. Shipped atlases
            // are 256 or 512 by 128 or 256, so this never fires; refusing beats
            // wrong pixels, as `texture` does for odd 4bpp.
            if row_bytes % SWIZZLE_BLOCK_BYTES != 0 || usize::from(height) % SWIZZLE_BLOCK_ROWS != 0
            {
                return Err(Error::UnswizzleableAtlas { width, height });
            }
            oag_formats::swizzle::unswizzle(texels, row_bytes, usize::from(height))
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

    /// Builds a font from metrics with no pixels and an atlas read elsewhere.
    ///
    /// This is the PS2 build: its `.fnt` ends where the atlas would start and
    /// the glyph sheet is its own [`PSMT4`](crate::ps2_texture::Layout::Psmt4)
    /// archive entry. `indices` is one palette index per pixel in raster order,
    /// `palette` is RGBA8888.
    ///
    /// # Alpha is the caller's to normalise
    ///
    /// A [`crate::ps2_texture`] palette keeps the GS's 0-128 alpha and
    /// [`Self::alpha_at`] reads coverage on 0-255, so the caller doubles it first
    /// ([`crate::ps2_texture::Ps2Texture::to_rgba`] has the rule). Doing it here
    /// would be wrong for an already full-range palette.
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
    /// Every palette entry in every shipped font is a **neutral grey** (`r == g ==
    /// b` on all 16 entries of all five Pulse fonts and all six of Pure's), so
    /// this reads the red channel rather than a weighted luminance.
    ///
    /// # What it means
    ///
    /// The **body/outline mask**. The three menu fonts are a single pure white, so
    /// this is a constant 255 for them. The two HUD fonts carry six distinct
    /// greys and are pre-outlined: light is glyph body, dark is outline. A
    /// renderer wanting the original's look mixes the text colour toward the
    /// border colour by this and takes opacity from [`Self::alpha_at`].
    ///
    /// Reading alpha alone draws an outlined digit as a filled box (measured).
    /// See `docs/formats/fnt.md` and `oag_game::font`.
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
    /// True when the palette holds more than one distinct grey: the two HUD fonts
    /// against the three menu ones. Says whether [`Self::luma_at`] carries information.
    #[must_use]
    pub fn is_outlined(&self) -> bool {
        let mut seen: Option<u8> = None;
        for entry in &self.palette {
            // Fully transparent entries carry no colour worth comparing.
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

/// Byte coverage of one `.fnt` blob.
///
/// Works on a PS2 font (only [`Metrics::parse`] succeeds, no atlas to claim) and
/// a PSP/PS3 font (the atlas header, palette and texels are claimed too). The 20
/// bytes at `+0x1c` are left unclaimed: nothing reads them to check they are zero.
#[must_use]
pub fn coverage(data: &[u8]) -> oag_formats::coverage::Coverage {
    let mut seen = oag_formats::coverage::Coverage::new(data.len());
    let Ok(metrics) = Metrics::parse(data) else {
        return seen;
    };
    seen.claim(0, 0x1c, "the font header");
    let order = metrics.order;
    let count = metrics.glyphs.len().max(1);
    let codepoints_at = order.u32(data, 0x08) as usize;
    let offsets_at = order.u32(data, 0x0c) as usize;
    // Claimed at the declared `+0x04` count, which may be one longer than
    // `glyphs.len()` when a trailing zero ends the list early.
    let declared_count = order.u32(data, 0x04) as usize;
    seen.claim(codepoints_at, declared_count * 2, "the codepoint table");
    seen.claim(offsets_at, declared_count * 4, "the offset table");
    // `Glyph` keeps no file offset, so this re-walks the offset table as
    // `Metrics::parse` does.
    for i in 0..count.min(metrics.glyphs.len()) {
        let Some(bytes) = data.get(offsets_at + i * 4..offsets_at + i * 4 + 4) else {
            break;
        };
        let at = order.u32(bytes, 0) as usize;
        seen.claim(at, GLYPH_LEN, "a glyph record");
    }

    if metrics.has_embedded_atlas(data)
        && let Ok(font) = Font::parse(data)
    {
        let atlas_at = metrics.atlas_at;
        seen.claim(atlas_at, ATLAS_HEADER_LEN, "the atlas header");
        let clut_size = font.palette.len() * 4;
        seen.claim(atlas_at + ATLAS_HEADER_LEN, clut_size, "the atlas palette");
        let texel_size = usize::from(font.width) * usize::from(font.height) / 2;
        seen.claim(
            atlas_at + ATLAS_HEADER_LEN + clut_size,
            texel_size,
            "the atlas texels",
        );
    }

    seen
}

#[cfg(test)]
mod tests;
