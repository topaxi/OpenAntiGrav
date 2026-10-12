use super::*;

/// Writes bits LSB-first into a 16-byte block, the inverse of
/// [`BitStream::read`], to build synthetic blocks with known field values.
struct BitWriter {
    data: [u8; 16],
    pos: u32,
}

impl BitWriter {
    fn new() -> Self {
        Self {
            data: [0u8; 16],
            pos: 0,
        }
    }

    fn write(&mut self, value: u32, n: u32) {
        for i in 0..n {
            let bit = (value >> i) & 1;
            let bit_index = self.pos + i;
            if bit != 0 {
                self.data[(bit_index / 8) as usize] |= 1 << (bit_index % 8);
            }
        }
        self.pos += n;
    }
}

#[test]
fn expand_replicates_high_bits_into_the_gap() {
    // 5-bit max value widens to 8-bit max, not just a left shift.
    assert_eq!(expand(0b11111, 5), 0xff);
    assert_eq!(expand(0b00000, 5), 0x00);
    // The worked example in the spec's own text: a 6-bit field's top two
    // bits land in the byte's bottom two once widened.
    assert_eq!(expand(0b111111, 6), 0xff);
    // 8-bit fields pass through unchanged.
    assert_eq!(expand(0x5a, 8), 0x5a);
}

#[test]
fn weight_tables_start_at_zero_and_end_at_64() {
    assert_eq!(weights(2)[0], 0);
    assert_eq!(weights(2)[3], 64);
    assert_eq!(weights(3)[0], 0);
    assert_eq!(weights(3)[7], 64);
    assert_eq!(weights(4)[0], 0);
    assert_eq!(weights(4)[15], 64);
}

#[test]
fn lerp_at_the_index_extremes_returns_an_endpoint_exactly() {
    assert_eq!(lerp(10, 200, 0), 10);
    assert_eq!(lerp(10, 200, 64), 200);
}

/// Mode 6 is the simplest to hand-build: one subset (no partition table), no
/// rotation, no index-selection bit, a per-endpoint P-bit and a 4-bit index per
/// texel.
#[test]
fn mode6_round_trips_known_endpoints_through_every_texel() {
    let mut w = BitWriter::new();
    w.write(0b1000000, 7); // mode 6: six zero bits then a one, LSB-first
    // Endpoint 0: R=0x7f G=0x00 B=0x00 A=0x7f (7 bits each), pbit 0 -> widens to
    // 0xfe/0x00/0x00/0xfe (value<<1|0).
    w.write(0x7f, 7); // r0
    w.write(0x7f, 7); // r1 (endpoint 1's red, read before g/b/a per spec order)
    w.write(0x00, 7); // g0
    w.write(0x00, 7); // g1
    w.write(0x00, 7); // b0
    w.write(0x00, 7); // b1
    w.write(0x7f, 7); // a0
    w.write(0x7f, 7); // a1
    w.write(0, 1); // pbit endpoint 0
    w.write(1, 1); // pbit endpoint 1
    // 16 indices, 4 bits each except the anchor (texel 0) at 3. All zero ->
    // every texel decodes to endpoint 0.
    for t in 0..16 {
        w.write(0, if t == 0 { 3 } else { 4 });
    }

    let decoded = bc7(&w.data);
    // Endpoint 0: r/a 0x7f with pbit 0 -> (0x7f<<1)|0 = 0xfe (color_full_bits==8).
    for texel in decoded {
        assert_eq!(texel, [0xfe, 0x00, 0x00, 0xfe]);
    }
}

#[test]
fn mode6_max_index_reaches_endpoint_one_except_at_the_capped_anchor() {
    let mut w = BitWriter::new();
    w.write(0b1000000, 7);
    w.write(0x00, 7); // r0
    w.write(0x7f, 7); // r1
    w.write(0x00, 7); // g0
    w.write(0x7f, 7); // g1
    w.write(0x00, 7); // b0
    w.write(0x7f, 7); // b1
    w.write(0x00, 7); // a0
    w.write(0x7f, 7); // a1
    w.write(0, 1); // pbit endpoint 0
    w.write(1, 1); // pbit endpoint 1
    for t in 0..16 {
        // Every stored index is all-ones, but the anchor texel (0) stores only 3
        // of 4 bits (its top bit is zero by the spec's endpoint ordering), so
        // its index tops out at 7, not 15, and it falls short of endpoint 1.
        w.write(0b1111, if t == 0 { 3 } else { 4 });
    }

    let decoded = bc7(&w.data);
    // Endpoint 1: value 0x7f, pbit 1 -> (0x7f<<1)|1 = 0xff.
    for texel in &decoded[1..] {
        assert_eq!(*texel, [0xff, 0xff, 0xff, 0xff]);
    }
    let anchor_weight = weights(4)[0b111];
    let expected = lerp(0x00, 0xff, anchor_weight);
    assert_eq!(decoded[0], [expected; 4]);
    assert_ne!(
        decoded[0], [0xff; 4],
        "the anchor's index is capped below max"
    );
}

#[test]
fn an_all_zero_block_is_the_reserved_encoding_and_does_not_panic() {
    let block = [0u8; 16];
    assert_eq!(bc7(&block), [[0, 0, 0, 255]; 16]);
}

#[test]
fn every_mode_number_decodes_without_panicking() {
    // Not a correctness check: a sweep that every mode's field widths fit the
    // 16-byte block for an all-ones block (the densest pattern).
    for mode in 0..8u32 {
        let mut block = [0xffu8; 16];
        // Clear the low byte down to just this mode's own unary marker.
        block[0] = 1 << mode;
        let _ = bc7(&block);
    }
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

/// The decoder `bc7` replaced on 2026-10-12, kept verbatim as its oracle: the
/// spec's field order, with the subset, anchor and weight looked up per texel.
///
/// A block whose low byte is zero is the spec's reserved encoding ("should not
/// be used when encoding a BPTC texture"); it returns opaque black rather than
/// panicking, so one malformed block cannot take down an image.
fn reference_bc7(block: &[u8; 16]) -> [[u8; 4]; 16] {
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

/// A small deterministic generator, so the sweep below is the same every run.
fn next(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

/// The hoisted decoder against the one it replaced, on blocks of every mode,
/// partition, rotation and index selection: random bits above a forced mode
/// prefix, plus the reserved all-zero first byte.
#[test]
fn bc7_matches_the_spec_order_decoder_on_every_mode() {
    let mut state = 0x9e37_79b9_7f4a_7c15u64;
    for mode in 0..8u32 {
        for _ in 0..200_000 {
            let mut block = [0u8; 16];
            block[..8].copy_from_slice(&next(&mut state).to_le_bytes());
            block[8..].copy_from_slice(&next(&mut state).to_le_bytes());
            // `mode` zero bits, then a one.
            block[0] = (block[0] & !(((2u16 << mode) - 1) as u8)) | (1 << mode);
            assert_eq!(
                bc7(&block),
                reference_bc7(&block),
                "mode {mode} {block:02x?}"
            );
        }
    }
    let mut reserved = [0xffu8; 16];
    reserved[0] = 0;
    assert_eq!(bc7(&reserved), reference_bc7(&reserved));
}
