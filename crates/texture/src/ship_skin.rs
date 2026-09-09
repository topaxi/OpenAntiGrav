//! Ship skin textures - `ship_alt.dat` and `ship_eliminator.dat`.
//!
//! **A skin is a texture swap on the same geometry, not a second model.**
//! A file is a `0x20`-byte header (the team's own *display* name,
//! NUL-terminated, `AG Systems` with a space rather than the `AG_Systems`
//! directory name) followed by four palette-plus-pixels blocks, one per
//! texture slot of the hull model, matched to the model's own texture names
//! by [`ship-skin.md`](../../../docs/ghidra/functions/psp-pulse-usa/ship-skin.md)'s
//! `Skin_ApplyToModel`:
//!
//! ```text
//! +0x0000  0x20    header: NUL-terminated team display name
//! +0x0020  0x40    palette 1 - sixteen RGBA8888 entries
//! +0x0060  0x2000  texture 1 - 128x128, 4 bits per pixel
//! +0x2060  0x40    palette 2
//! +0x20a0  0x2000  texture 2 - 128x128, 4bpp
//! +0x40a0  0x40    palette 3
//! +0x40e0  0x2000  texture 3 - 128x128, 4bpp
//! +0x60e0  0x40    palette 4
//! +0x6120  0x800   texture 4 - 64x64, 4bpp
//!          0x6920  total
//! ```
//!
//! # There is no dimension field anywhere in the file
//!
//! `Texture_UploadPaletted` reads width, height and palette length from the
//! **target texture's own descriptor**, never from the block. The `0x40`
//! palette and `0x2000` pixel sizes above are what the hull's own textures
//! happen to be, not something the file declares - so [`parse`] does not read
//! a dimension out of the blob either. What it does instead is derive the
//! four blocks' shape from the file's own total length: every one of the
//! sixteen shipped `.dat` files is exactly 26,912 bytes, which is
//! `HEADER_LEN + 3 * BLOCK_STRIDE + PALETTE_LEN + SMALL_PIXELS_LEN` with
//! nothing left over, and that arithmetic is the closest thing to a
//! dimension check this format has. A file of any other length is refused
//! rather than guessed at.
//!
//! # Mips are generated, never stored
//!
//! The original halves each block repeatedly in index space with a
//! palette-aware 2x2 filter (`Texture_Downsample4bpp`). [`Block::indices`] is
//! level 0 only, the same choice [`crate::texture::Texture`] makes for a
//! `.mip`'s stored mip chain.
//!
//! # The fourth block's composite use is not decoded here
//!
//! When the applier's flag argument is zero, block 4 is not what gets
//! uploaded to `\TEXTURE4.TGA` - a composite of quarter-scale downsamples of
//! blocks 1-3 is, built by `Skin_ComposeQuarterAtlas`, and its top-right
//! quadrant is left as uninitialised stack in the original. That composition
//! is model-application logic, not part of this file's own byte layout, and
//! is deliberately out of scope here; see
//! [`ship-skin.md`](../../../docs/ghidra/functions/psp-pulse-usa/ship-skin.md)'s
//! own section on it.
//!
//! Confidence 90 on the layout: the offsets and the 128x128 size are plain
//! immediates in `Skin_ApplyToModel` / `Skin_ComposeQuarterAtlas`, and the
//! arithmetic closes with nothing left over against all sixteen real files.

/// Bytes of the NUL-terminated team-name header before the first palette.
pub const HEADER_LEN: usize = 0x20;

/// Bytes of one palette: sixteen RGBA8888 entries.
pub const PALETTE_LEN: usize = 0x40;

/// Side length of blocks 1-3's textures, in pixels. Square, 4bpp.
pub const LARGE_SIDE: usize = 128;

/// Side length of block 4's texture, in pixels. Square, 4bpp.
pub const SMALL_SIDE: usize = 64;

/// Bytes from the start of one of blocks 1-3's palette to the start of the
/// next: one palette plus one `LARGE_SIDE`-square 4bpp texture.
pub const BLOCK_STRIDE: usize = PALETTE_LEN + LARGE_SIDE * LARGE_SIDE / 2;

/// Bytes block 4's packed pixels occupy: `SMALL_SIDE * SMALL_SIDE / 2`.
pub const SMALL_PIXELS_LEN: usize = SMALL_SIDE * SMALL_SIDE / 2;

/// Total bytes a skin file holds - fully determined by the layout above, and
/// never read from the file itself. Every one of the sixteen shipped
/// `ship_alt.dat` / `ship_eliminator.dat` files on `pulse-psp-usa.chd` is
/// exactly this length.
pub const FILE_LEN: usize = HEADER_LEN + 3 * BLOCK_STRIDE + PALETTE_LEN + SMALL_PIXELS_LEN;

/// Something wrong with a skin blob.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The blob is not [`FILE_LEN`] bytes.
    ///
    /// The size is fully determined by the fixed layout, so a mismatch means
    /// this is not a skin file rather than that it is a damaged one.
    SizeMismatch {
        /// Bytes supplied.
        got: usize,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SizeMismatch { got } => {
                write!(f, "a skin file is exactly {FILE_LEN} bytes, got {got}")
            }
        }
    }
}

impl std::error::Error for Error {}

/// Result alias for this module.
pub type Result<T> = std::result::Result<T, Error>;

/// One of the four texture blocks a skin file carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    /// Pixels wide.
    pub width: usize,
    /// Pixels tall.
    pub height: usize,
    /// Sixteen RGBA8888 entries.
    pub palette: [[u8; 4]; 16],
    /// One palette index per pixel, unpacked from 4-bit pairs, low nibble
    /// first. Always exactly `width * height` long.
    pub indices: Vec<u8>,
}

impl Block {
    fn parse(data: &[u8], width: usize, height: usize) -> Self {
        let palette = data[..PALETTE_LEN]
            .as_chunks::<4>()
            .0
            .try_into()
            .expect("PALETTE_LEN is 16 four-byte entries");
        let packed = &data[PALETTE_LEN..PALETTE_LEN + width * height / 2];
        // Low nibble first, the same convention `texture::Texture` uses:
        // pixel 0 is the low half of byte 0.
        let mut indices = Vec::with_capacity(width * height);
        for &byte in packed {
            indices.push(byte & 0x0f);
            indices.push(byte >> 4);
        }
        Self {
            width,
            height,
            palette,
            indices,
        }
    }

    /// Expands to RGBA8888, row-major from the top left.
    #[must_use]
    pub fn to_rgba(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.indices.len() * 4);
        for &index in &self.indices {
            out.extend_from_slice(&self.palette[index as usize]);
        }
        out
    }
}

/// A decoded ship skin: a team's display name and its four texture blocks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skin {
    /// The team's display name, from the header - `AG Systems`, not the
    /// `AG_Systems` directory name.
    pub team_name: String,
    /// Blocks 1-4, matched by `ship-skin.md`'s `g_skin_texture_slot_names`
    /// table to `\TEXTURE1.TGA` through `\TEXTURE4.TGA`.
    pub blocks: [Block; 4],
}

/// Parses a `ship_alt.dat` / `ship_eliminator.dat` blob.
pub fn parse(data: &[u8]) -> Result<Skin> {
    if data.len() != FILE_LEN {
        return Err(Error::SizeMismatch { got: data.len() });
    }

    // The header holds one NUL-terminated string and nothing else. Six of the
    // eight base teams have zero bytes after the terminator; the two that
    // don't (Assegai, Qirex) are residue from a shorter name overwriting a
    // longer one in a reused export buffer, not a second field - see
    // ship-skin.md's "`ms` tag does not exist" section. A parser must not
    // read past the terminator.
    let header = &data[..HEADER_LEN];
    let name_end = header.iter().position(|&b| b == 0).unwrap_or(header.len());
    let team_name = String::from_utf8_lossy(&header[..name_end]).into_owned();

    let sizes = [
        (LARGE_SIDE, LARGE_SIDE),
        (LARGE_SIDE, LARGE_SIDE),
        (LARGE_SIDE, LARGE_SIDE),
        (SMALL_SIDE, SMALL_SIDE),
    ];
    let mut offset = HEADER_LEN;
    let mut blocks = Vec::with_capacity(4);
    for (width, height) in sizes {
        let block_len = PALETTE_LEN + width * height / 2;
        blocks.push(Block::parse(
            &data[offset..offset + block_len],
            width,
            height,
        ));
        offset += block_len;
    }
    debug_assert_eq!(offset, FILE_LEN);

    Ok(Skin {
        team_name,
        blocks: blocks.try_into().expect("exactly four blocks pushed above"),
    })
}

#[cfg(test)]
mod tests;
