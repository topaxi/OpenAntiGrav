//! BC7 (BPTC): one 128-bit block, one of eight modes, decoded from the public
//! spec text alone - no reference decoder's code was read.
//!
//! Source: Khronos's `GL_ARB_texture_compression_bptc` extension specification,
//! <https://www.khronos.org/registry/OpenGL/extensions/ARB/ARB_texture_compression_bptc.txt>,
//! "Appendix: BPTC Compressed Texture Image Format": `Table.M` (mode field
//! widths), `Table.P2`/`Table.P3` (partition assignment), `Table.A2`/`Table.A3a`/
//! `Table.A3b` (anchor index per partition) and the interpolation weights,
//! reproduced verbatim in [`tables`]. `oag_texture::gnf` is the only caller:
//! only Omega's `.gnf` carries a BC7 surface (`docs/formats/gnf.md`'s census).
//!
//! # Block layout
//!
//! Bits are read LSB-first, byte 0 bit 0 first, in this fixed field order:
//! mode (unary: N zero bits then a one bit), partition number, rotation,
//! index selection bit, colour (R then G then B, each subset's endpoint 0
//! then endpoint 1, then the next subset), alpha (same layout, if the mode
//! carries one), per-endpoint or shared P-bits, primary indices (one per
//! texel, raster order, the subset's anchor texel one bit narrower since
//! its top bit is known to be zero), secondary indices (same shape, if the
//! mode carries them).
//!
//! Endpoints are widened to 8 bits by replicating their high bits into the low
//! ones, as [`crate::bcn::rgb565`] does for BC1-3, generalised in [`expand`].

mod tables;

/// One field width per mode, `Table.M` in the spec, indexed by mode number.
struct Mode {
    /// Number of subsets (independent-gradient regions) in the block.
    subsets: u32,
    /// Partition-number field width.
    partition_bits: u32,
    /// Rotation field width (0 or 2).
    rotation_bits: u32,
    /// Index-selection field width (0 or 1).
    index_selection_bits: u32,
    /// Bits per colour channel per endpoint, before any P-bit.
    color_bits: u32,
    /// Bits of alpha per endpoint, before any P-bit; 0 means alpha is not
    /// stored and every endpoint's alpha is 255.
    alpha_bits: u32,
    /// Whether every endpoint carries its own extra low bit.
    per_endpoint_pbit: bool,
    /// Whether one extra bit is shared by both endpoints of a subset.
    shared_pbit: bool,
    /// Primary index field width.
    index_bits: u32,
    /// Secondary index field width; 0 means there is no secondary index.
    index_bits2: u32,
}

const MODES: [Mode; 8] = [
    Mode {
        subsets: 3,
        partition_bits: 4,
        rotation_bits: 0,
        index_selection_bits: 0,
        color_bits: 4,
        alpha_bits: 0,
        per_endpoint_pbit: true,
        shared_pbit: false,
        index_bits: 3,
        index_bits2: 0,
    },
    Mode {
        subsets: 2,
        partition_bits: 6,
        rotation_bits: 0,
        index_selection_bits: 0,
        color_bits: 6,
        alpha_bits: 0,
        per_endpoint_pbit: false,
        shared_pbit: true,
        index_bits: 3,
        index_bits2: 0,
    },
    Mode {
        subsets: 3,
        partition_bits: 6,
        rotation_bits: 0,
        index_selection_bits: 0,
        color_bits: 5,
        alpha_bits: 0,
        per_endpoint_pbit: false,
        shared_pbit: false,
        index_bits: 2,
        index_bits2: 0,
    },
    Mode {
        subsets: 2,
        partition_bits: 6,
        rotation_bits: 0,
        index_selection_bits: 0,
        color_bits: 7,
        alpha_bits: 0,
        per_endpoint_pbit: true,
        shared_pbit: false,
        index_bits: 2,
        index_bits2: 0,
    },
    Mode {
        subsets: 1,
        partition_bits: 0,
        rotation_bits: 2,
        index_selection_bits: 1,
        color_bits: 5,
        alpha_bits: 6,
        per_endpoint_pbit: false,
        shared_pbit: false,
        index_bits: 2,
        index_bits2: 3,
    },
    Mode {
        subsets: 1,
        partition_bits: 0,
        rotation_bits: 2,
        index_selection_bits: 0,
        color_bits: 7,
        alpha_bits: 8,
        per_endpoint_pbit: false,
        shared_pbit: false,
        index_bits: 2,
        index_bits2: 2,
    },
    Mode {
        subsets: 1,
        partition_bits: 0,
        rotation_bits: 0,
        index_selection_bits: 0,
        color_bits: 7,
        alpha_bits: 7,
        per_endpoint_pbit: true,
        shared_pbit: false,
        index_bits: 4,
        index_bits2: 0,
    },
    Mode {
        subsets: 2,
        partition_bits: 6,
        rotation_bits: 0,
        index_selection_bits: 0,
        color_bits: 5,
        alpha_bits: 5,
        per_endpoint_pbit: true,
        shared_pbit: false,
        index_bits: 2,
        index_bits2: 0,
    },
];

/// Reads bits LSB-first out of a 16-byte block, advancing a cursor.
///
/// The block is one little-endian `u128`, whose bit `i` is bit `i % 8` of byte
/// `i / 8` (the stream order), so a field is a shift and a mask. A bit-at-a-time
/// loop was 94% of the Omega GNF pixel sweep's profile.
struct BitStream {
    bits: u128,
    pos: u32,
}

impl BitStream {
    fn new(data: &[u8; 16]) -> Self {
        Self {
            bits: u128::from_le_bytes(*data),
            pos: 0,
        }
    }

    /// Reads `n` bits (`n` up to 32), stream LSB into bit 0. `n == 0` reads
    /// nothing and returns 0, so a zero field width needs no branch.
    fn read(&mut self, n: u32) -> u32 {
        let out = ((self.bits >> self.pos) & ((1u128 << n) - 1)) as u32;
        self.pos += n;
        out
    }
}

/// Widens a `bits`-wide value to 8 bits by replicating its high bits into the low
/// ones (the spec: "the top bits of the value are replicated into any remaining
/// bits in the byte"). BC7 channel widths are at least 4, so one pass fills the
/// byte (`2 * bits >= 8`).
fn expand(value: u32, bits: u32) -> u8 {
    if bits == 0 {
        return 0;
    }
    if bits >= 8 {
        return value as u8;
    }
    let shifted = value << (8 - bits);
    (shifted | (shifted >> bits)) as u8
}

/// Subset index (0, 1 or 2) of texel `t` (raster order, `x + y * 4`).
fn subset_of(subsets: u32, partition: usize, t: usize) -> usize {
    match subsets {
        1 => 0,
        2 => tables::P2[partition][t] as usize,
        _ => tables::P3[partition][t] as usize,
    }
}

/// The anchor texel (the one whose top index bit is not stored) for
/// `subset` under this partition.
fn anchor_of(subsets: u32, partition: usize, subset: usize) -> usize {
    match (subsets, subset) {
        (_, 0) => 0,
        (2, 1) => tables::A2[partition] as usize,
        (3, 1) => tables::A3A[partition] as usize,
        (3, 2) => tables::A3B[partition] as usize,
        _ => 0,
    }
}

/// The six-bit interpolation weight for index `index` out of `bits` bits.
fn weight(bits: u32, index: u32) -> u32 {
    match bits {
        2 => tables::WEIGHTS2[index as usize],
        3 => tables::WEIGHTS3[index as usize],
        4 => tables::WEIGHTS4[index as usize],
        _ => 0,
    }
}

/// `(64 - w) * e0 + w * e1`, rounded, over a 6-bit weight - the spec's own
/// "always performed using a 6-bit interpolation factor".
fn lerp(e0: u8, e1: u8, w: u32) -> u8 {
    (((64 - w) * u32::from(e0) + w * u32::from(e1) + 32) >> 6) as u8
}

/// Decodes one 16-byte BC7 block to 16 RGBA8 texels, raster order.
///
/// A block whose low byte is zero is the spec's reserved encoding ("should not
/// be used when encoding a BPTC texture"); it returns opaque black rather than
/// panicking, so one malformed block cannot take down an image.
pub(crate) fn bc7(block: &[u8; 16]) -> [[u8; 4]; 16] {
    let mut bits = BitStream::new(block);

    let Some(mode_number) = (0..8).find(|_| bits.read(1) == 1) else {
        return [[0, 0, 0, 255]; 16];
    };
    let mode = &MODES[mode_number as usize];

    let partition = bits.read(mode.partition_bits) as usize;
    let rotation = bits.read(mode.rotation_bits);
    let index_selection = bits.read(mode.index_selection_bits) != 0;

    let endpoints = 2 * mode.subsets as usize;
    let mut red = [0u32; 6];
    let mut green = [0u32; 6];
    let mut blue = [0u32; 6];
    let mut alpha = [0u32; 6];
    for slot in red.iter_mut().take(endpoints) {
        *slot = bits.read(mode.color_bits);
    }
    for slot in green.iter_mut().take(endpoints) {
        *slot = bits.read(mode.color_bits);
    }
    for slot in blue.iter_mut().take(endpoints) {
        *slot = bits.read(mode.color_bits);
    }
    if mode.alpha_bits > 0 {
        for slot in alpha.iter_mut().take(endpoints) {
            *slot = bits.read(mode.alpha_bits);
        }
    }

    let mut pbit = [0u32; 6];
    if mode.per_endpoint_pbit {
        for slot in pbit.iter_mut().take(endpoints) {
            *slot = bits.read(1);
        }
    } else if mode.shared_pbit {
        for subset in 0..mode.subsets as usize {
            let p = bits.read(1);
            pbit[2 * subset] = p;
            pbit[2 * subset + 1] = p;
        }
    }
    let has_pbit = mode.per_endpoint_pbit || mode.shared_pbit;
    let color_full_bits = mode.color_bits + u32::from(has_pbit);
    let alpha_full_bits = if mode.alpha_bits == 0 {
        0
    } else {
        mode.alpha_bits + u32::from(has_pbit)
    };

    let mut endpoint_rgba = [[0u8; 4]; 6];
    for e in 0..endpoints {
        let with_pbit = |component: u32| -> u32 { (component << u32::from(has_pbit)) | pbit[e] };
        endpoint_rgba[e][0] = expand(with_pbit(red[e]), color_full_bits);
        endpoint_rgba[e][1] = expand(with_pbit(green[e]), color_full_bits);
        endpoint_rgba[e][2] = expand(with_pbit(blue[e]), color_full_bits);
        endpoint_rgba[e][3] = if mode.alpha_bits == 0 {
            255
        } else {
            expand(with_pbit(alpha[e]), alpha_full_bits)
        };
    }

    let mut primary_index = [0u32; 16];
    for (t, slot) in primary_index.iter_mut().enumerate() {
        let subset = subset_of(mode.subsets, partition, t);
        let anchor = anchor_of(mode.subsets, partition, subset);
        let width = if t == anchor {
            mode.index_bits - 1
        } else {
            mode.index_bits
        };
        *slot = bits.read(width);
    }

    let mut secondary_index = [0u32; 16];
    if mode.index_bits2 > 0 {
        for (t, slot) in secondary_index.iter_mut().enumerate() {
            let subset = subset_of(mode.subsets, partition, t);
            let anchor = anchor_of(mode.subsets, partition, subset);
            let width = if t == anchor {
                mode.index_bits2 - 1
            } else {
                mode.index_bits2
            };
            *slot = bits.read(width);
        }
    }

    let mut out = [[0u8; 4]; 16];
    for t in 0..16 {
        let subset = subset_of(mode.subsets, partition, t);
        let e0 = endpoint_rgba[2 * subset];
        let e1 = endpoint_rgba[2 * subset + 1];

        let (color_bits, color_index, alpha_bits, alpha_index) = if mode.index_bits2 == 0 {
            (
                mode.index_bits,
                primary_index[t],
                mode.index_bits,
                primary_index[t],
            )
        } else if mode.index_selection_bits > 0 {
            if index_selection {
                (
                    mode.index_bits2,
                    secondary_index[t],
                    mode.index_bits,
                    primary_index[t],
                )
            } else {
                (
                    mode.index_bits,
                    primary_index[t],
                    mode.index_bits2,
                    secondary_index[t],
                )
            }
        } else {
            (
                mode.index_bits,
                primary_index[t],
                mode.index_bits2,
                secondary_index[t],
            )
        };

        let cw = weight(color_bits, color_index);
        let aw = weight(alpha_bits, alpha_index);
        let r = lerp(e0[0], e1[0], cw);
        let g = lerp(e0[1], e1[1], cw);
        let b = lerp(e0[2], e1[2], cw);
        let a = lerp(e0[3], e1[3], aw);
        // Rotation swaps alpha with one colour channel. A choice of whole arrays,
        // not a `swap`: the swap caused a store-forwarding stall (over half this
        // function's time).
        out[t] = match rotation {
            1 => [a, g, b, r],
            2 => [r, a, b, g],
            3 => [r, g, a, b],
            _ => [r, g, b, a],
        };
    }

    out
}

#[cfg(test)]
mod tests;
