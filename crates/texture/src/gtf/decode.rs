//! Texel decoding: block compression, the linear layout, and the RSX's
//! Morton-order tiling.
//!
//! # The colour endpoints are little-endian inside a big-endian file
//!
//! Every field of a [`.gtf`](super) header is big-endian, but a `DXT` block's
//! two `R5G6B5` endpoints are not: the texel payload is the block layout a
//! `.dds` stores.
//!
//! **Measured**, by decoding both readings of every HUD texture on the disc and
//! comparing smoothness (the test [`crate::ps2_texture`] uses for its swizzle).
//! Real art is smooth across a block boundary and a byte-swapped endpoint is
//! not. Numbers: `docs/formats/gtf.md`. The 2-bit index word is read a byte per
//! row, the same either way round.
//!
//! # The Morton order is the RSX's standard `cellGcm` tiling
//!
//! [`morton_index`]'s bit-interleave is the documented PS3 SDK swizzle, also
//! used by RPCS3. Measured here: `docs/formats/gtf.md` runs the same roughness
//! comparison over every swizzled `A8R8G8B8`/`A8B8G8R8` texture against a
//! row-major misread.

use crate::bcn;

use super::Format;

/// Decodes one mip level to straight RGBA8, or `None` for a layout this does
/// not read.
///
/// `pitch` is the descriptor's, in bytes, 0 meaning tightly packed; only the
/// linear layouts use it. `linear` is the descriptor's `0x20` bit; only the two
/// uncompressed formats consult it, so a block-only caller
/// ([`super::decode_level`]) can pass either.
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
        // One byte per texel, into blue only. The broadcast is the descriptor's
        // own `remap` (all 9 `B8` files select blue for every output) - see
        // [`super::Remap`].
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
/// The standard `cellGcm` tiling: interleave the low bit of `x` then of `y`,
/// shifting each pair out, until the *narrower* dimension's bits run out; the
/// wider dimension's remaining high bits then continue linearly. A square
/// power-of-two texture never reaches that second phase; only the 9 `B8`
/// ambient shadows (128x64) exercise it.
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

/// Four bytes per texel in the RSX's Morton order ([`morton_index`]). `order`
/// is the channel permutation [`linear_texels`] takes.
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
/// No `B8` file on the disc is linear; only [`super::tests`] exercise this.
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
/// The address function [`swizzled`] uses, a byte offset at one byte per texel.
/// All 9 `B8` files are 128x64, the only non-square swizzled textures on disc,
/// so this is where [`morton_index`]'s non-square branch carries weight.
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
