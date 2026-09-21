//! Pixel decoding for a **linear** `.gnf` surface only.
//!
//! `Texture::decode` refuses every genuinely tiled surface by name
//! ([`Error::Tiled`]) rather than guessing at a picture - see `gnf.rs`'s own
//! "What this does not do" and `docs/formats/gnf.md`'s tiling-calibration
//! section for why: AMD's own address library
//! (`EgBasedLib::HwlReduceBankWidthHeight`, `src/amd/addrlib/src/r800/egbaddrlib.cpp`
//! in Mesa) reduces a macro-tiled surface's bank width/height/aspect
//! *per surface*, from its bits-per-element and a DRAM row-size constant
//! this project has not recovered, rather than the fixed lookup a single
//! `TileMode` index would be if the reduction never fired. Measured
//! directly: `.gnf` entries under 40 blocks wide pad their own `pitch`
//! field to a small, size-dependent multiple (33 blocks -> 40, 65 -> 72,
//! 15 -> 16 - each just the next multiple of 8), while a 526-block-wide one
//! pads to 1,024 - two different `macro_tile_pitch` values on the same
//! declared `TileMode(13)`, which only the reduction algorithm explains.
//! Reimplementing that algorithm from Mesa's C++ without a real PS4 to
//! render against and check the picture on is exactly the "stand-in that
//! reads as legible" this project's rule against inventing what the assets
//! already author exists to prevent - so it stays unimplemented, named,
//! rather than guessed.

use super::{Error, Result, SurfaceFormat, Texture};

pub(super) fn decode(texture: &Texture, blob: &[u8]) -> Result<Vec<[u8; 4]>> {
    if !texture.is_linear() {
        return Err(Error::Tiled {
            tile_mode: texture.tile_mode.0,
        });
    }

    let data = blob
        .get(texture.data_offset..)
        .ok_or(Error::DataOutOfBounds {
            need: texture.data_offset,
            got: blob.len(),
        })?;

    match texture.surface_format {
        SurfaceFormat::Bc1 => blocks(data, texture.width, texture.height, 8, |b| {
            crate::bcn::dxt1(b.try_into().expect("checked length"))
        }),
        SurfaceFormat::Bc3 => blocks(data, texture.width, texture.height, 16, |b| {
            crate::bcn::dxt45(b.try_into().expect("checked length"))
        }),
        SurfaceFormat::Bc7 => blocks(data, texture.width, texture.height, 16, |b| {
            crate::bcn::bc7(b.try_into().expect("checked length"))
        }),
        other => Err(Error::UnsupportedFormat { format: other }),
    }
}

/// Walks a row-major grid of 4x4 blocks, `unit_len` bytes each, decoding
/// every block with `decode_block` and writing its 16 texels into an
/// RGBA8 raster of `width` x `height`. Only reached for a linear surface,
/// so "block index `by * blocks_x + bx`" is already the byte order - no
/// untiling step sits between this and the file.
fn blocks(
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
