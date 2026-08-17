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
                Format::Dxt1 => dxt1(block.try_into().ok()?),
                Format::Dxt23 => dxt23(block.try_into().ok()?),
                Format::Dxt45 => dxt45(block.try_into().ok()?),
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

/// A `R5G6B5` endpoint expanded to RGB8, replicating the high bits into the low
/// ones so that white stays white.
fn rgb565(value: u16) -> [u8; 3] {
    let r = u32::from(value >> 11 & 0x1f);
    let g = u32::from(value >> 5 & 0x3f);
    let b = u32::from(value & 0x1f);
    [
        ((r * 255 + 15) / 31) as u8,
        ((g * 255 + 31) / 63) as u8,
        ((b * 255 + 15) / 31) as u8,
    ]
}

/// The four colours a BC1 colour block selects between, and their alphas.
///
/// `c0 > c1` is the opaque four-colour mode; otherwise the fourth entry is
/// transparent black and the third is a plain midpoint. BC2 and BC3 carry alpha
/// of their own, so they always take the four-colour arithmetic - which is why
/// `opaque` is a parameter rather than read from the comparison.
fn palette(block: &[u8; 8], opaque: bool) -> [[u8; 4]; 4] {
    let c0 = u16::from_le_bytes([block[0], block[1]]);
    let c1 = u16::from_le_bytes([block[2], block[3]]);
    let a = rgb565(c0);
    let b = rgb565(c1);
    let mut out = [[0u8; 4]; 4];
    out[0] = [a[0], a[1], a[2], 255];
    out[1] = [b[0], b[1], b[2], 255];
    if opaque || c0 > c1 {
        for channel in 0..3 {
            let (x, y) = (u32::from(a[channel]), u32::from(b[channel]));
            out[2][channel] = ((2 * x + y) / 3) as u8;
            out[3][channel] = ((x + 2 * y) / 3) as u8;
        }
        out[2][3] = 255;
        out[3][3] = 255;
    } else {
        for channel in 0..3 {
            let (x, y) = (u32::from(a[channel]), u32::from(b[channel]));
            out[2][channel] = ((x + y) / 2) as u8;
        }
        out[2][3] = 255;
        out[3] = [0, 0, 0, 0];
    }
    out
}

/// The sixteen 2-bit selectors, one byte per row, low bits leftmost.
fn selectors(block: &[u8; 8]) -> [usize; 16] {
    let mut out = [0usize; 16];
    for row in 0..4 {
        let bits = block[4 + row];
        for column in 0..4 {
            out[row * 4 + column] = usize::from(bits >> (2 * column) & 0b11);
        }
    }
    out
}

fn dxt1(block: &[u8; 8]) -> [[u8; 4]; 16] {
    let colours = palette(block, false);
    selectors(block).map(|index| colours[index])
}

fn dxt23(block: &[u8; 16]) -> [[u8; 4]; 16] {
    let colour: &[u8; 8] = block[8..].try_into().expect("eight bytes");
    let colours = palette(colour, true);
    let mut out = selectors(colour).map(|index| colours[index]);
    for (index, texel) in out.iter_mut().enumerate() {
        // Four bits per texel, two texels per byte, low nibble first.
        let nibble = block[index / 2] >> (4 * (index % 2)) & 0x0f;
        texel[3] = nibble * 17;
    }
    out
}

fn dxt45(block: &[u8; 16]) -> [[u8; 4]; 16] {
    let colour: &[u8; 8] = block[8..].try_into().expect("eight bytes");
    let colours = palette(colour, true);
    let mut out = selectors(colour).map(|index| colours[index]);

    let alphas = alpha_ramp(block[0], block[1]);
    // Six bytes of 3-bit selectors, two rows to a 24-bit little-endian group.
    for half in 0..2 {
        let at = 2 + half * 3;
        let bits =
            u32::from(block[at]) | u32::from(block[at + 1]) << 8 | u32::from(block[at + 2]) << 16;
        for slot in 0..8 {
            out[half * 8 + slot][3] = alphas[(bits >> (3 * slot) & 0b111) as usize];
        }
    }
    out
}

/// The eight alpha values a BC3 block selects between.
fn alpha_ramp(a0: u8, a1: u8) -> [u8; 8] {
    let mut out = [0u8; 8];
    out[0] = a0;
    out[1] = a1;
    let (x, y) = (u32::from(a0), u32::from(a1));
    if a0 > a1 {
        for step in 1..7u32 {
            out[1 + step as usize] = (((7 - step) * x + step * y) / 7) as u8;
        }
    } else {
        for step in 1..5u32 {
            out[1 + step as usize] = (((5 - step) * x + step * y) / 5) as u8;
        }
        out[6] = 0;
        out[7] = 255;
    }
    out
}
