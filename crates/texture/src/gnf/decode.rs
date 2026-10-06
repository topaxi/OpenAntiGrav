//! Pixel decoding for a linear `.gnf` surface, and a micro-tiled
//! (`TileMode(13)`, `Thin_1DThin`) BC7 one whose base level carries no
//! invalid-mode block.
//!
//! Every sampled `.gnf` declares `TileMode(13)`, `Thin_1DThin` (GFD-Studio's
//! `TileMode.cs`), micro-tiled. AMD's micro-tile-only address formula
//! (`EgBasedLib::ComputeSurfaceAddrFromCoordMicroTiled`, Mesa's MIT `addrlib`)
//! matches oracle-paired textures exactly on an isolated single-tile image and
//! over many consecutive tiles on larger ones. `docs/formats/gnf.md`'s "Tiling"
//! section closes the open question: the "bad" regions of the ship-livery oracle
//! pair are not a wrong tile order but the PSARC-level missing/garbage-content
//! population `docs/formats/psarc.md`'s "Block data location" documents
//! family-wide.
//!
//! **What this still refuses, by design.** Per the rule against inventing what
//! the assets author: draw nothing rather than a picture with silent garbage
//! patches. [`decode`] scans the *base level's* block grid (the descriptor's
//! `width`/`height` are the base level's, so smaller mips' tile padding is
//! excluded) for any block whose byte 0 carries no valid BC7 mode (a real
//! encoder never emits an all-zero mode field) and refuses the whole surface
//! with [`Error::CorruptBlocks`]. A whole-file census
//! (`crates/texture/examples/gnf_frontend_census.rs`) over-counts, since it walks
//! every later mip's padding too; at the base level alone most of the front
//! end's sprite sheet is clean - see `docs/formats/gnf.md`'s "Tiling" section.
//!
//! A `Display_LinearAligned`/`Display_LinearGeneral` surface (none found in the
//! corpus) decodes through the plain row-major path below unconditionally.

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
/// `pixelBit0..5 = x0,y0,x1,y1,x2,y2`. Verified against a real oracle pair - see
/// `docs/formats/gnf.md`'s "Tiling" section. Mirrors
/// `crates/texture/src/gnf/oracle_tests.rs::micro_tile_index` rather than
/// importing it (test-only tree).
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

/// The base mip level's block grid: the descriptor's `width` x `height` are the
/// base level's and GNM lays a chain out largest-first, tile-aligned, so the base
/// occupies `tiles_x * tiles_y` tiles from [`Texture::data_offset`] whatever
/// follows.
fn base_level_tile_grid(texture: &Texture) -> (u32, u32, u32, u32) {
    let width_blocks = texture.width.div_ceil(4);
    let height_blocks = texture.height.div_ceil(4);
    let pitch_blocks = texture.pitch.max(texture.width).div_ceil(4);
    let tiles_x = pitch_blocks.div_ceil(8);
    let tiles_y = height_blocks.div_ceil(8);
    (width_blocks, height_blocks, tiles_x, tiles_y)
}

/// A BC7 mode field is unary (`N` zero bits then a one bit) over the first byte
/// for modes 0-7; byte `0x00` has no such bit and no real encoder emits it. See
/// `docs/formats/gnf.md`'s "Tiling" for why this is the corruption check.
fn has_valid_bc7_mode(byte0: u8) -> bool {
    byte0 != 0x00
}

/// One level's blocks, untiled into plain row-major order.
///
/// `offset` is the level's first byte from [`Texture::data_offset`]. Counts
/// blocks with no valid mode bit and reports the count beside the bytes, so a
/// caller can refuse a level without a second pass.
fn untile_level(
    texture: &Texture,
    blob: &[u8],
    offset: usize,
    width_blocks: u32,
    height_blocks: u32,
    tiles_x: u32,
) -> Result<(Vec<u8>, usize)> {
    let mut out = Vec::with_capacity((width_blocks * height_blocks) as usize * 16);
    let mut corrupt = 0usize;
    for by in 0..height_blocks {
        for bx in 0..width_blocks {
            let start =
                texture.data_offset + offset + micro_tiled_block_offset(bx, by, tiles_x) as usize;
            let block = blob.get(start..start + 16).ok_or(Error::DataOutOfBounds {
                need: start + 16,
                got: blob.len(),
            })?;
            corrupt += usize::from(!has_valid_bc7_mode(block[0]));
            out.extend_from_slice(block);
        }
    }
    Ok((out, corrupt))
}

fn require_micro_tiled_bc7(texture: &Texture) -> Result<()> {
    if texture.surface_format != SurfaceFormat::Bc7 {
        return Err(Error::UnsupportedFormat {
            format: texture.surface_format,
        });
    }
    Ok(())
}

fn decode_micro_tiled(texture: &Texture, blob: &[u8]) -> Result<Vec<[u8; 4]>> {
    require_micro_tiled_bc7(texture)?;
    let (width_blocks, height_blocks, tiles_x, _tiles_y) = base_level_tile_grid(texture);
    let (blocks, corrupt) = untile_level(texture, blob, 0, width_blocks, height_blocks, tiles_x)?;
    if corrupt > 0 {
        return Err(Error::CorruptBlocks { count: corrupt });
    }
    row_major_blocks(&blocks, texture.width, texture.height, 16, |b| {
        crate::bcn::bc7(b.try_into().expect("checked length"))
    })
}

pub(super) fn bc7_level(blocks: &[u8], width: u32, height: u32) -> Option<Vec<[u8; 4]>> {
    row_major_blocks(blocks, width, height, 16, |b| {
        crate::bcn::bc7(b.try_into().expect("checked length"))
    })
    .ok()
}

/// Every mip level's BC7 blocks, untiled and row-major, base level first.
///
/// Levels follow one another, each padded to whole 8x8-block micro tiles:
/// `ceil(wb/8) * ceil(hb/8) * 1024` bytes for a level `wb` x `hb` blocks. That
/// accounts for every byte past the header of 15,413 of the 15,4xx BC7 `.gnf`
/// files on disc - see `docs/formats/gnf.md`.
pub(super) fn block_levels(texture: &Texture, blob: &[u8]) -> Result<Vec<Vec<u8>>> {
    require_micro_tiled_bc7(texture)?;
    if texture.tile_mode.0 != super::TileMode::THIN_1D_THIN {
        return Err(Error::Tiled {
            tile_mode: texture.tile_mode.0,
        });
    }
    let count = usize::from(
        texture
            .last_mip_level
            .saturating_sub(texture.base_mip_level),
    ) + 1;
    let grids: Vec<(u32, u32, u32, u32)> = (0..count)
        .map(|level| {
            if level == 0 {
                base_level_tile_grid(texture)
            } else {
                let wb = (texture.width >> level).max(1).div_ceil(4);
                let hb = (texture.height >> level).max(1).div_ceil(4);
                (wb, hb, wb.div_ceil(8), hb.div_ceil(8))
            }
        })
        .collect();
    let expected: usize = grids
        .iter()
        .map(|(_, _, tx, ty)| (tx * ty) as usize * MICRO_TILE_BYTES as usize)
        .sum();
    // Exact, not "at least": a cubemap or array carries more than one chain, and
    // reading its first as the only one would be a wrong picture.
    let found = blob.len().saturating_sub(texture.data_offset);
    if found != expected {
        return Err(Error::ChainLayout { expected, found });
    }
    let mut levels = Vec::with_capacity(count);
    let mut offset = 0usize;
    let mut corrupt = 0usize;
    for (width_blocks, height_blocks, tiles_x, tiles_y) in grids {
        let (blocks, bad) =
            untile_level(texture, blob, offset, width_blocks, height_blocks, tiles_x)?;
        corrupt += bad;
        levels.push(blocks);
        offset += (tiles_x * tiles_y) as usize * MICRO_TILE_BYTES as usize;
    }
    if corrupt > 0 {
        return Err(Error::CorruptBlocks { count: corrupt });
    }
    Ok(levels)
}

/// Walks a row-major grid of 4x4 blocks, `unit_len` bytes each, decoding each
/// with `decode_block` into an RGBA8 raster of `width` x `height`. Only reached
/// for a linear surface, so block index `by * blocks_x + bx` is the byte order.
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
