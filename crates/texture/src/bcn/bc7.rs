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

/// The six-bit interpolation weights for a `bits`-wide index (2, 3 or 4).
fn weights(bits: u32) -> &'static [u32] {
    match bits {
        2 => &tables::WEIGHTS2,
        3 => &tables::WEIGHTS3,
        _ => &tables::WEIGHTS4,
    }
}

/// `(64 - w) * e0 + w * e1`, rounded, over a 6-bit weight - the spec's own
/// "always performed using a 6-bit interpolation factor".
fn lerp(e0: u8, e1: u8, w: u32) -> u8 {
    (((64 - w) * u32::from(e0) + w * u32::from(e1) + 32) >> 6) as u8
}

/// Red, green and blue of an endpoint in three 16-bit lanes, so one multiply
/// interpolates all three: a lane holds at most `255 * 64 + 32`, which never
/// carries into the next.
fn pack(endpoint: [u8; 4]) -> u64 {
    u64::from(endpoint[0]) | u64::from(endpoint[1]) << 16 | u64::from(endpoint[2]) << 32
}

/// The `+ 32` of [`lerp`] in each lane of [`pack`].
const ROUND: u64 = 32 | 32 << 16 | 32 << 32;

/// [`pack`] with alpha in the fourth lane, for a mode whose alpha shares the
/// colour's weight.
fn pack_rgba(endpoint: [u8; 4]) -> u64 {
    pack(endpoint) | u64::from(endpoint[3]) << 48
}

/// The `+ 32` of [`lerp`] in each lane of [`pack_rgba`].
const ROUND_RGBA: u64 = ROUND | 32 << 48;

/// Reads one index per texel, the anchor texel of each subset one bit
/// narrower (its top bit is known to be zero).
///
/// Out of one 64-bit window rather than a `u128` shift per index: no mode's
/// index field is wider than 63 bits (mode 6's primary, `16 * 4 - 1`). Each
/// index's position is its start in a field of `width` bits less one per anchor
/// before it, so the sixteen reads do not wait on one another.
#[inline(always)]
fn read_indices(
    bits: &mut BitStream,
    width: u32,
    subset_of: &[u8; 16],
    anchors: [usize; 3],
) -> [u32; 16] {
    let window = (bits.bits >> bits.pos) as u64;
    let mut start = 0;
    let mut out = [0u32; 16];
    for (t, slot) in out.iter_mut().enumerate() {
        let n = width - u32::from(t == anchors[subset_of[t] as usize]);
        *slot = ((window >> start) & ((1u64 << n) - 1)) as u32;
        start += n;
    }
    bits.pos += start;
    out
}

/// No partition: every texel is subset 0.
const ONE_SUBSET: [u8; 16] = [0; 16];

/// Decodes one 16-byte BC7 block to 16 RGBA8 texels, raster order.
///
/// A block whose low byte is zero is the spec's reserved encoding ("should not
/// be used when encoding a BPTC texture"); it returns opaque black rather than
/// panicking, so one malformed block cannot take down an image.
///
/// Everything that depends only on the mode and partition (the subset of each
/// texel, the anchors, which index drives colour and which alpha, the weight
/// tables) is chosen once per block, not per texel, which made a block about a
/// quarter faster than the plain spec-order decoder (measured 2026-10-12).
/// `tests.rs` keeps that decoder and holds the two equal.
pub(crate) fn bc7(block: &[u8; 16]) -> [[u8; 4]; 16] {
    // The mode is unary from bit 0: its number is the count of zero bits before
    // the first one.
    match block[0].trailing_zeros() {
        0 => decode_mode::<0>(block),
        1 => decode_mode::<1>(block),
        2 => decode_mode::<2>(block),
        3 => decode_mode::<3>(block),
        4 => decode_mode::<4>(block),
        5 => decode_mode::<5>(block),
        6 => decode_mode::<6>(block),
        7 => decode_mode::<7>(block),
        _ => [[0, 0, 0, 255]; 16],
    }
}

/// [`bc7`] for a block of mode `MODE`. The mode is a constant here, so every
/// field width and loop bound below folds away in the compiled copy for that
/// mode.
fn decode_mode<const MODE: usize>(block: &[u8; 16]) -> [[u8; 4]; 16] {
    let mode = &MODES[MODE];
    let mut bits = BitStream::new(block);
    bits.pos = MODE as u32 + 1;

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

    let (subset_of, anchors): (&[u8; 16], [usize; 3]) = match mode.subsets {
        1 => (&ONE_SUBSET, [0, 0, 0]),
        2 => (
            &tables::P2[partition],
            [0, tables::A2[partition] as usize, 0],
        ),
        _ => (
            &tables::P3[partition],
            [
                0,
                tables::A3A[partition] as usize,
                tables::A3B[partition] as usize,
            ],
        ),
    };

    let primary = read_indices(&mut bits, mode.index_bits, subset_of, anchors);
    let secondary = if mode.index_bits2 > 0 {
        read_indices(&mut bits, mode.index_bits2, subset_of, anchors)
    } else {
        [0u32; 16]
    };

    // Which index drives colour and which alpha: the primary for both without a
    // secondary; with one, the index-selection bit swaps them where the mode
    // has that bit.
    let (color_index, color_bits, alpha_index, alpha_bits) = if mode.index_bits2 == 0 {
        (&primary, mode.index_bits, &primary, mode.index_bits)
    } else if mode.index_selection_bits > 0 && index_selection {
        (&secondary, mode.index_bits2, &primary, mode.index_bits)
    } else {
        (&primary, mode.index_bits, &secondary, mode.index_bits2)
    };
    let color_weights = weights(color_bits);
    let alpha_weights = weights(alpha_bits);

    let mut out = [[0u8; 4]; 16];
    if mode.index_bits2 == 0 {
        // One index drives colour and alpha alike, and no such mode rotates, so
        // all four channels interpolate in one multiply.
        let packed: [u64; 6] = std::array::from_fn(|e| pack_rgba(endpoint_rgba[e]));
        for t in 0..16 {
            let subset = subset_of[t] as usize;
            let weight = color_weights[color_index[t] as usize];
            let texel = (packed[2 * subset] * u64::from(64 - weight)
                + packed[2 * subset + 1] * u64::from(weight)
                + ROUND_RGBA)
                >> 6;
            out[t] = [
                texel as u8,
                (texel >> 16) as u8,
                (texel >> 32) as u8,
                (texel >> 48) as u8,
            ];
        }
        return out;
    }

    let packed: [u64; 6] = std::array::from_fn(|e| pack(endpoint_rgba[e]));
    for t in 0..16 {
        let subset = subset_of[t] as usize;
        let e0 = endpoint_rgba[2 * subset];
        let e1 = endpoint_rgba[2 * subset + 1];
        let cw = color_weights[color_index[t] as usize];
        let aw = alpha_weights[alpha_index[t] as usize];
        let colour = (packed[2 * subset] * u64::from(64 - cw)
            + packed[2 * subset + 1] * u64::from(cw)
            + ROUND)
            >> 6;
        let r = colour as u8;
        let g = (colour >> 16) as u8;
        let b = (colour >> 32) as u8;
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
