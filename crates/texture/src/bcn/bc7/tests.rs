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
    assert_eq!(weight(2, 0), 0);
    assert_eq!(weight(2, 3), 64);
    assert_eq!(weight(3, 0), 0);
    assert_eq!(weight(3, 7), 64);
    assert_eq!(weight(4, 0), 0);
    assert_eq!(weight(4, 15), 64);
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
    let anchor_weight = weight(4, 0b111);
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
