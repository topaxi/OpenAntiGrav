//! The S3TC/BC1-3 block math shared by every container that carries a DXT
//! family texture: [`crate::gtf`] (PS3) and [`crate::gxt`] (Vita).
//!
//! The block layout is a hardware standard, so the bit-twiddling is the same
//! whichever container led here; endianness and pitch rules stay in each
//! format's own module.
//!
//! [`bc7`] is BC7/BPTC, used only by [`crate::gnf`] (neither `.gtf` nor `.gxt`
//! ships a BC7 surface), kept here because the block format is the same hardware
//! standard.

mod bc7;
pub(crate) use bc7::bc7;

/// A `R5G6B5` endpoint expanded to RGB8, replicating the high bits into the low
/// ones so that white stays white.
pub(crate) fn rgb565(value: u16) -> [u8; 3] {
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
/// transparent black and the third a plain midpoint. BC2 and BC3 carry their own
/// alpha and always take the four-colour arithmetic, hence `opaque` as a parameter.
pub(crate) fn palette(block: &[u8; 8], opaque: bool) -> [[u8; 4]; 4] {
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
pub(crate) fn selectors(block: &[u8; 8]) -> [usize; 16] {
    let mut out = [0usize; 16];
    for row in 0..4 {
        let bits = block[4 + row];
        for column in 0..4 {
            out[row * 4 + column] = usize::from(bits >> (2 * column) & 0b11);
        }
    }
    out
}

/// One BC1 (`DXT1`) block: 8 bytes, no alpha channel.
pub(crate) fn dxt1(block: &[u8; 8]) -> [[u8; 4]; 16] {
    let colours = palette(block, false);
    selectors(block).map(|index| colours[index])
}

/// One BC2 (`DXT23`) block: 16 bytes, four bits of alpha per texel.
pub(crate) fn dxt23(block: &[u8; 16]) -> [[u8; 4]; 16] {
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

/// One BC3 (`DXT45`) block: 16 bytes, interpolated alpha ramp.
pub(crate) fn dxt45(block: &[u8; 16]) -> [[u8; 4]; 16] {
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
