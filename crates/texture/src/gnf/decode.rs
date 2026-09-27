//! Pixel decoding for a linear `.gnf` surface, and a micro-tiled
//! (`TileMode(13)`, `Thin_1DThin`) BC7 one whose base level carries no
//! invalid-mode block.
//!
//! Every real `.gnf` this project has sampled declares `TileMode(13)` -
//! `Thin_1DThin` (GFD-Studio's own `TileMode.cs` enum), micro-tiled, not the
//! macro-tiled `Thin_2DThin` (14) an earlier pass here mislabeled it as.
//! AMD's much simpler micro-tile-only address formula
//! (`EgBasedLib::ComputeSurfaceAddrFromCoordMicroTiled`, Mesa's MIT
//! `addrlib`) gets strong confirmation against real oracle-paired textures -
//! an exact match on an isolated single-tile image, many consecutive
//! correctly-decoded tiles on larger ones - and `docs/formats/gnf.md`'s
//! "Tiling" section closes the remaining open question: a byte-level check
//! on the ship-livery oracle pair that first found the periodic corruption
//! shows the "bad" regions are not a wrong tile order at all, they are the
//! same PSARC-level missing/garbage-content population
//! `docs/formats/psarc.md`'s "Block data location" section already
//! documents family-wide, on this specific file's own copy.
//!
//! **What this still refuses, by design, not by gap.** This project's own
//! rule against inventing what the assets already author reads a
//! partially-corrupt block the same way it reads a byte range that will not
//! parse at all: draw nothing rather than a picture with silent garbage
//! patches standing in for missing content. So [`decode`] scans the *base
//! level's own* block grid - `width`/`height` in the descriptor are always
//! the base (largest, first-in-file) level's own, so this already excludes
//! every smaller mip level's own tile-alignment padding without needing to
//! know how many follow - for any block whose byte 0 carries no valid BC7
//! mode (a real encoder never emits an all-zero mode field), and refuses the
//! whole surface with [`Error::CorruptBlocks`] the moment it finds one,
//! rather than decoding around it. A whole-file census
//! (`crates/texture/examples/gnf_frontend_census.rs`) over-counts this,
//! since it walks every byte sequentially including every later mip's own
//! padding tiles; measured at the base level alone, most of the front end's
//! own sprite sheet is clean - see `docs/formats/gnf.md`'s "Tiling" section
//! for the corrected count and the worked padding arithmetic.
//!
//! A genuinely `Display_LinearAligned`/`Display_LinearGeneral` surface -
//! none has been found among the corpus so far - still decodes through the
//! plain row-major path below unconditionally, since a linear surface has
//! no tile order to get wrong in the first place.

use super::{Error, Result, SurfaceFormat, Texture};

pub(super) fn decode(texture: &Texture, blob: &[u8]) -> Result<Vec<[u8; 4]>> {
    if texture.is_linear() {
        return decode_linear(texture, blob);
    }
    if texture.tile_mode.0 == super::TileMode::THIN_1D_THIN {
        return decode_micro_tiled(texture, blob);
    }
    Err(Error::Tiled {
        tile_mode: texture.tile_mode.0,
    })
}

fn decode_linear(texture: &Texture, blob: &[u8]) -> Result<Vec<[u8; 4]>> {
    let data = blob
        .get(texture.data_offset..)
        .ok_or(Error::DataOutOfBounds {
            need: texture.data_offset,
            got: blob.len(),
        })?;

    match texture.surface_format {
        SurfaceFormat::Bc1 => row_major_blocks(data, texture.width, texture.height, 8, |b| {
            crate::bcn::dxt1(b.try_into().expect("checked length"))
        }),
        SurfaceFormat::Bc3 => row_major_blocks(data, texture.width, texture.height, 16, |b| {
            crate::bcn::dxt45(b.try_into().expect("checked length"))
        }),
        SurfaceFormat::Bc7 => row_major_blocks(data, texture.width, texture.height, 16, |b| {
            crate::bcn::bc7(b.try_into().expect("checked length"))
        }),
        other => Err(Error::UnsupportedFormat { format: other }),
    }
}

/// `Lib::ComputePixelIndexWithinMicroTile`, `addrlib1.cpp`, the
/// `ADDR_NON_DISPLAYABLE` ("Thin") ordering for a 128-bit element:
/// `pixelBit0..5 = x0,y0,x1,y1,x2,y2`. Verified against a real oracle pair -
/// see `docs/formats/gnf.md`'s "Tiling" section and
/// `crates/texture/src/gnf/oracle_tests.rs::micro_tile_index`, which this
/// mirrors rather than imports (that copy stays `pub(super)` to
/// `oracle_tests`/`micro_tile_tests`'s own test-only tree).
fn micro_tile_pixel_index(bx: u32, by: u32) -> u32 {
    let x0 = bx & 1;
    let x1 = (bx >> 1) & 1;
    let x2 = (bx >> 2) & 1;
    let y0 = by & 1;
    let y1 = (by >> 1) & 1;
    let y2 = (by >> 2) & 1;
    x0 | (y0 << 1) | (x1 << 2) | (y1 << 3) | (x2 << 4) | (y2 << 5)
}

/// Bytes one micro tile (8x8 BC7 blocks, one block being one 128-bit
/// element) occupies on disk.
const MICRO_TILE_BYTES: u64 = 64 * 16;

/// Byte offset of BC7 block `(bx, by)`, tile-row width `tiles_x` tiles,
/// from [`Texture::data_offset`].
fn micro_tiled_block_offset(bx: u32, by: u32, tiles_x: u32) -> u64 {
    let tile_index = u64::from((by / 8) * tiles_x + (bx / 8));
    let pixel_index = micro_tile_pixel_index(bx % 8, by % 8);
    let element_offset = u64::from(pixel_index) * 16;
    tile_index * MICRO_TILE_BYTES + element_offset
}

/// The base (largest, first-in-file) mip level's own block grid: `width` x
/// `height` in the descriptor is always the base level's, and GNM lays a
/// mip chain out largest-first, each level tile-aligned, so the base level
/// occupies exactly `tiles_x * tiles_y` tiles starting at
/// [`Texture::data_offset`] regardless of how many smaller levels follow it.
fn base_level_tile_grid(texture: &Texture) -> (u32, u32, u32, u32) {
    let width_blocks = texture.width.div_ceil(4);
    let height_blocks = texture.height.div_ceil(4);
    let pitch_blocks = texture.pitch.max(texture.width).div_ceil(4);
    let tiles_x = pitch_blocks.div_ceil(8);
    let tiles_y = height_blocks.div_ceil(8);
    (width_blocks, height_blocks, tiles_x, tiles_y)
}

/// A BC7 block's mode field is unary (`N` zero bits then a one bit) over its
/// first byte for every mode 0-7; byte `0x00` has no such bit and is not a
/// pattern a real encoder emits - see this module's own doc comment and
/// `docs/formats/gnf.md`'s "Tiling" section for why that is the corruption
/// check rather than a spatial one.
fn has_valid_bc7_mode(byte0: u8) -> bool {
    byte0 != 0x00
}

fn decode_micro_tiled(texture: &Texture, blob: &[u8]) -> Result<Vec<[u8; 4]>> {
    if texture.surface_format != SurfaceFormat::Bc7 {
        return Err(Error::UnsupportedFormat {
            format: texture.surface_format,
        });
    }

    let (width_blocks, height_blocks, tiles_x, _tiles_y) = base_level_tile_grid(texture);
    let mut corrupt = 0usize;
    for by in 0..height_blocks {
        for bx in 0..width_blocks {
            let start = texture.data_offset + micro_tiled_block_offset(bx, by, tiles_x) as usize;
            let byte0 = *blob.get(start).ok_or(Error::DataOutOfBounds {
                need: start + 1,
                got: blob.len(),
            })?;
            if !has_valid_bc7_mode(byte0) {
                corrupt += 1;
            }
        }
    }
    if corrupt > 0 {
        return Err(Error::CorruptBlocks { count: corrupt });
    }

    let mut out = vec![[0u8; 4]; (texture.width * texture.height) as usize];
    for by in 0..height_blocks {
        for bx in 0..width_blocks {
            let start = texture.data_offset + micro_tiled_block_offset(bx, by, tiles_x) as usize;
            let block: [u8; 16] = blob
                .get(start..start + 16)
                .and_then(|s| s.try_into().ok())
                .ok_or(Error::DataOutOfBounds {
                    need: start + 16,
                    got: blob.len(),
                })?;
            let texels = crate::bcn::bc7(&block);
            for ty in 0..4 {
                for tx in 0..4 {
                    let px = bx * 4 + tx;
                    let py = by * 4 + ty;
                    if px < texture.width && py < texture.height {
                        out[(py * texture.width + px) as usize] = texels[(ty * 4 + tx) as usize];
                    }
                }
            }
        }
    }
    Ok(out)
}

/// Walks a row-major grid of 4x4 blocks, `unit_len` bytes each, decoding
/// every block with `decode_block` and writing its 16 texels into an
/// RGBA8 raster of `width` x `height`. Only reached for a linear surface,
/// so "block index `by * blocks_x + bx`" is already the byte order - no
/// untiling step sits between this and the file.
fn row_major_blocks(
    data: &[u8],
    width: u32,
    height: u32,
    unit_len: usize,
    decode_block: impl Fn(&[u8]) -> [[u8; 4]; 16],
) -> Result<Vec<[u8; 4]>> {
    let blocks_x = width.div_ceil(4);
    let blocks_y = height.div_ceil(4);
    let mut out = vec![[0u8; 4]; (width * height) as usize];
    for by in 0..blocks_y {
        for bx in 0..blocks_x {
            let index = (by * blocks_x + bx) as usize;
            let start = index * unit_len;
            let block = data
                .get(start..start + unit_len)
                .ok_or(Error::DataOutOfBounds {
                    need: start + unit_len,
                    got: data.len(),
                })?;
            let texels = decode_block(block);
            for ty in 0..4 {
                for tx in 0..4 {
                    let px = bx * 4 + tx;
                    let py = by * 4 + ty;
                    if px < width && py < height {
                        out[(py * width + px) as usize] = texels[(ty * 4 + tx) as usize];
                    }
                }
            }
        }
    }
    Ok(out)
}
