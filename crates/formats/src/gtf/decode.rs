//! Texel decoding: block compression, the linear layout, and the RSX's
//! Morton-order tiling.
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
//!
//! # The Morton order is the RSX's standard `cellGcm` tiling, not a guess
//!
//! [`morton_index`]'s bit-interleave is the documented PS3 SDK swizzle used
//! everywhere the platform tiles a 2D surface, not something reverse-engineered
//! from this disc alone - it is the same address function RPCS3's own texture
//! cache and every other PS3 homebrew GCM reader use. What *is* measured against
//! this disc, rather than assumed from the platform, is that it is the right one
//! *here*: `docs/formats/gtf.md` runs the same roughness comparison the DXT
//! endianness question above used, over every swizzled `A8R8G8B8`/`A8B8G8R8`
//! texture on the disc, against a deliberately wrong permutation (row-major, as
//! if the `0x20` linear bit had been misread). See that page for the numbers.

use crate::bcn;

use super::Format;

/// Decodes one mip level to straight RGBA8, or `None` for a layout this does
/// not read.
///
/// `pitch` is the descriptor's, in bytes, and 0 means tightly packed. Consulted
/// for the linear layouts only - a swizzled texture's addressing has no notion
/// of a row stride, and block compression is a tiling of its own that the
/// `0x20` bit is not read for either; see [`Format::is_block_compressed`].
///
/// `linear` is the descriptor's `0x20` bit, and only the two uncompressed
/// formats consult it - a compressed one is block-order regardless, so a
/// caller with nothing to say about linearity (`oag_render`'s block-only
/// decode fallback, [`super::decode_level`]) can pass either.
pub fn level(
    format: Format,
    texels: &[u8],
    width: u32,
    height: u32,
    pitch: usize,
    linear: bool,
) -> Option<Vec<[u8; 4]>> {
    let pixels = (width as usize).checked_mul(height as usize)?;
    let mut out = vec![[0u8; 4]; pixels];
    match (format, linear) {
        (Format::Dxt1 | Format::Dxt23 | Format::Dxt45, _) => {
            blocks(format, texels, width, height, pitch, &mut out)?;
        }
        (Format::A8R8G8B8, true) => {
            linear_texels(texels, width, height, pitch, &mut out, [1, 2, 3, 0])?
        }
        (Format::A8R8G8B8, false) => swizzled(texels, width, height, &mut out, [1, 2, 3, 0])?,
        (Format::A8B8G8R8, true) => {
            linear_texels(texels, width, height, pitch, &mut out, [3, 2, 1, 0])?
        }
        (Format::A8B8G8R8, false) => swizzled(texels, width, height, &mut out, [3, 2, 1, 0])?,
        // One byte per texel, landing in **blue** and nothing else. The
        // broadcast that makes it a picture is the descriptor's own `remap`,
        // which on all 9 of the disc's `B8` files selects the blue source for
        // every output channel - see [`super::Remap`]. Doing it here instead
        // would be inventing what the file already says.
        (Format::B8, true) => single(texels, width, height, pitch, &mut out)?,
        (Format::B8, false) => swizzled_single(texels, width, height, &mut out)?,
    }
    Some(out)
}

/// Four bytes per texel, `order` naming which source byte feeds R, G, B and A.
///
/// `A8R8G8B8` is a big-endian `u32` with alpha on top, so the bytes arrive A, R,
/// G, B and red is byte 1.
fn linear_texels(
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

/// The RSX's Morton-order texel index for `(x, y)` in a `width x height`
/// swizzled surface.
///
/// The standard `cellGcm` tiling: interleave the low bit of `x` then the low
/// bit of `y`, one pair at a time, shifting each pair out as it is consumed,
/// until the *narrower* dimension's bits run out - then let the wider
/// dimension's remaining high bits continue linearly rather than interleave
/// with nothing. A square power-of-two texture never reaches that second
/// phase, since both dimensions run out together - which was every swizzled
/// file this read until the 9 `B8` ambient shadows joined them at 128x64, the
/// only textures on the disc that exercise it.
fn morton_index(x: u32, y: u32, width: u32, height: u32) -> usize {
    let (mut bits_x, mut bits_y) = (width.trailing_zeros(), height.trailing_zeros());
    let (mut x, mut y) = (x, y);
    let mut index = 0u32;
    let mut shift = 0u32;
    while bits_x > 0 || bits_y > 0 {
        if bits_x > 0 {
            index |= (x & 1) << shift;
            x >>= 1;
            shift += 1;
            bits_x -= 1;
        }
        if bits_y > 0 {
            index |= (y & 1) << shift;
            y >>= 1;
            shift += 1;
            bits_y -= 1;
        }
    }
    index as usize
}

/// Four bytes per texel, in the RSX's Morton order rather than raster order -
/// see [`morton_index`]. `order` is the same per-format channel permutation
/// [`linear_texels`] takes; swizzling moves where a texel's four bytes sit,
/// not what they mean.
fn swizzled(
    texels: &[u8],
    width: u32,
    height: u32,
    out: &mut [[u8; 4]],
    order: [usize; 4],
) -> Option<()> {
    for y in 0..height {
        for x in 0..width {
            let at = morton_index(x, y, width, height).checked_mul(4)?;
            let texel = texels.get(at..at + 4)?;
            out[(y * width + x) as usize] = [
                texel[order[0]],
                texel[order[1]],
                texel[order[2]],
                texel[order[3]],
            ];
        }
    }
    Some(())
}

/// One byte per texel in raster order, into blue.
///
/// No `B8` file on the disc is linear, so this path is exercised by
/// [`super::tests`] and nothing else. It is here rather than refused because a
/// one-byte row is the same arithmetic [`linear_texels`] already does, not a
/// reading that would need its own evidence.
fn single(texels: &[u8], width: u32, height: u32, pitch: usize, out: &mut [[u8; 4]]) -> Option<()> {
    let row_bytes = width as usize;
    let stride = if pitch == 0 { row_bytes } else { pitch };
    for y in 0..height as usize {
        let row = texels.get(y * stride..y * stride + row_bytes)?;
        for x in 0..width as usize {
            out[y * width as usize + x][2] = row[x];
        }
    }
    Some(())
}

/// One byte per texel in the RSX's Morton order, into blue.
///
/// The same address function [`swizzled`] uses - a texel index, so at one byte
/// per texel it is the byte offset outright. **`128x64` is where the
/// non-square branch of [`morton_index`] first carries weight**: every other
/// swizzled texture on the disc is square, and all 9 `B8` files are not.
fn swizzled_single(texels: &[u8], width: u32, height: u32, out: &mut [[u8; 4]]) -> Option<()> {
    for y in 0..height {
        for x in 0..width {
            let at = morton_index(x, y, width, height);
            out[(y * width + x) as usize][2] = *texels.get(at)?;
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
