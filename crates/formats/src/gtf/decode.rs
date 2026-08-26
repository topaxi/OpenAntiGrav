//! Texel decoding: block compression, and the one linear layout that ships.
//!
//! # The colour endpoints are little-endian inside a big-endian file
//!
//! Every field of a [`.gtf`](super) header is big-endian, so the natural guess
//! is that a `DXT` block's two `R5G6B5` endpoints are too. They are not: the
//! texel payload is whatever the RSX consumes, and that is the same block layout
//! a `.dds` stores, endpoints included.
//!
//! **Measured rather than assumed**, by decoding both readings of every HUD
//! texture on the disc and comparing how smooth each comes out - the same test
//! [`crate::ps2_texture`] settles its swizzle permutation with. Real art is
//! smooth across a block boundary and a byte-swapped `R5G6B5` endpoint is not:
//! swapping moves five bits of red into the low bits of blue. See
//! `docs/formats/gtf.md` for the numbers.
//!
//! The 2-bit index word is read a byte per row, which is the same answer either
//! way round and so needs no such argument.

use crate::bcn;

use super::Format;

/// Decodes one mip level to straight RGBA8, or `None` for a layout this does
/// not read.
///
/// `pitch` is the descriptor's, in bytes, and 0 means tightly packed.
pub fn level(
    format: Format,
    texels: &[u8],
    width: u32,
    height: u32,
    pitch: usize,
) -> Option<Vec<[u8; 4]>> {
    let pixels = (width as usize).checked_mul(height as usize)?;
    let mut out = vec![[0u8; 4]; pixels];
    match format {
        Format::Dxt1 | Format::Dxt23 | Format::Dxt45 => {
            blocks(format, texels, width, height, pitch, &mut out)?;
        }
        Format::A8R8G8B8 => linear(texels, width, height, pitch, &mut out, [1, 2, 3, 0])?,
        Format::A8B8G8R8 => linear(texels, width, height, pitch, &mut out, [3, 2, 1, 0])?,
        // `B8` is one channel and every one on the disc is swizzled, so there is
        // nothing to test a reading of.
        Format::B8 => return None,
    }
    Some(out)
}

/// Four bytes per texel, `order` naming which source byte feeds R, G, B and A.
///
/// `A8R8G8B8` is a big-endian `u32` with alpha on top, so the bytes arrive A, R,
/// G, B and red is byte 1.
fn linear(
    texels: &[u8],
    width: u32,
    height: u32,
    pitch: usize,
    out: &mut [[u8; 4]],
    order: [usize; 4],
) -> Option<()> {
    let row_bytes = width as usize * 4;
    let stride = if pitch == 0 { row_bytes } else { pitch };
    for y in 0..height as usize {
        let row = texels.get(y * stride..y * stride + row_bytes)?;
        for x in 0..width as usize {
            let at = x * 4;
            out[y * width as usize + x] = [
                row[at + order[0]],
                row[at + order[1]],
                row[at + order[2]],
                row[at + order[3]],
            ];
        }
    }
    Some(())
}

/// Bytes one 4x4 block occupies.
fn block_len(format: Format) -> usize {
    format.unit_len()
}

/// Walks the block grid and expands each block into `out`.
fn blocks(
    format: Format,
    texels: &[u8],
    width: u32,
    height: u32,
    pitch: usize,
    out: &mut [[u8; 4]],
) -> Option<()> {
    let unit = block_len(format);
    let across = (width as usize).div_ceil(4);
    let down = (height as usize).div_ceil(4);
    let stride = if pitch == 0 { across * unit } else { pitch };

    for by in 0..down {
        for bx in 0..across {
            let at = by * stride + bx * unit;
            let block = texels.get(at..at + unit)?;
            let texels16 = match format {
                Format::Dxt1 => bcn::dxt1(block.try_into().ok()?),
                Format::Dxt23 => bcn::dxt23(block.try_into().ok()?),
                Format::Dxt45 => bcn::dxt45(block.try_into().ok()?),
                _ => return None,
            };
            for (index, texel) in texels16.into_iter().enumerate() {
                let x = bx * 4 + index % 4;
                let y = by * 4 + index / 4;
                // A block on the right or bottom edge of a non-multiple-of-four
                // texture carries texels the image does not have.
                if x < width as usize && y < height as usize {
                    out[y * width as usize + x] = texel;
                }
            }
        }
    }
    Some(())
}
