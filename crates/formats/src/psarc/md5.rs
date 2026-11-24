//! MD5, because a PSARC directory row carries one and this crate checks it.
//!
//! RFC 1321, hand-rolled like `png`'s CRC-32 and Adler-32: a fixed, specified
//! algorithm with published test vectors. **Not** a security primitive: the
//! digest is a directory-integrity field, and this module buys the check that
//! [`super::path_digest`] predicts every entry's row from its own path.

/// Per-round left-rotation amounts, four rounds of four.
const SHIFTS: [u32; 64] = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, //
    5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, //
    4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, //
    6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
];

/// `floor(abs(sin(i + 1)) * 2^32)`, the constant table RFC 1321 tabulates.
///
/// Written out rather than computed, so no transcendental reaches this file
/// (`docs/architecture/determinism.md`, `just check-determinism`).
#[rustfmt::skip]
const K: [u32; 64] = [
    0xd76a_a478, 0xe8c7_b756, 0x2420_70db, 0xc1bd_ceee,
    0xf57c_0faf, 0x4787_c62a, 0xa830_4613, 0xfd46_9501,
    0x6980_98d8, 0x8b44_f7af, 0xffff_5bb1, 0x895c_d7be,
    0x6b90_1122, 0xfd98_7193, 0xa679_438e, 0x49b4_0821,
    0xf61e_2562, 0xc040_b340, 0x265e_5a51, 0xe9b6_c7aa,
    0xd62f_105d, 0x0244_1453, 0xd8a1_e681, 0xe7d3_fbc8,
    0x21e1_cde6, 0xc337_07d6, 0xf4d5_0d87, 0x455a_14ed,
    0xa9e3_e905, 0xfcef_a3f8, 0x676f_02d9, 0x8d2a_4c8a,
    0xfffa_3942, 0x8771_f681, 0x6d9d_6122, 0xfde5_380c,
    0xa4be_ea44, 0x4bde_cfa9, 0xf6bb_4b60, 0xbebf_bc70,
    0x289b_7ec6, 0xeaa1_27fa, 0xd4ef_3085, 0x0488_1d05,
    0xd9d4_d039, 0xe6db_99e5, 0x1fa2_7cf8, 0xc4ac_5665,
    0xf429_2244, 0x432a_ff97, 0xab94_23a7, 0xfc93_a039,
    0x655b_59c3, 0x8f0c_cc92, 0xffef_f47d, 0x8584_5dd1,
    0x6fa8_7e4f, 0xfe2c_e6e0, 0xa301_4314, 0x4e08_11a1,
    0xf753_7e82, 0xbd3a_f235, 0x2ad7_d2bb, 0xeb86_d391,
];

/// The 128-bit digest of `data`.
#[must_use]
pub fn digest(data: &[u8]) -> [u8; 16] {
    let mut state: [u32; 4] = [0x6745_2301, 0xefcd_ab89, 0x98ba_dcfe, 0x1032_5476];

    let (whole, rest) = data.as_chunks::<64>();
    for block in whole {
        compress(&mut state, block);
    }

    // The tail, a 0x80 byte, zeros and the bit length: one block when the tail leaves 8 bytes
    // free, two otherwise. Built on the stack, never in a copy of the message.
    let mut tail = [0u8; 128];
    tail[..rest.len()].copy_from_slice(rest);
    tail[rest.len()] = 0x80;
    let end = if rest.len() < 56 { 64 } else { 128 };
    let bit_len = (data.len() as u64).wrapping_mul(8);
    tail[end - 8..end].copy_from_slice(&bit_len.to_le_bytes());
    for block in tail[..end].as_chunks::<64>().0 {
        compress(&mut state, block);
    }

    let mut out = [0u8; 16];
    for (slot, word) in out.as_chunks_mut::<4>().0.iter_mut().zip(state) {
        *slot = word.to_le_bytes();
    }
    out
}

/// One 64-byte block into `state`: RFC 1321's four rounds of sixteen steps, each round its own
/// loop so its mixing function and word order are constants.
fn compress(state: &mut [u32; 4], block: &[u8; 64]) {
    let mut m = [0u32; 16];
    for (word, bytes) in m.iter_mut().zip(block.as_chunks::<4>().0) {
        *word = u32::from_le_bytes(*bytes);
    }
    let [mut a, mut b, mut c, mut d] = *state;
    macro_rules! round {
        ($range:expr, |$b:ident, $c:ident, $d:ident| $f:expr, |$i:ident| $g:expr) => {
            for $i in $range {
                let f = {
                    let ($b, $c, $d) = (b, c, d);
                    $f
                };
                let tmp = d;
                d = c;
                c = b;
                b = b.wrapping_add(
                    a.wrapping_add(f)
                        .wrapping_add(K[$i])
                        .wrapping_add(m[$g])
                        .rotate_left(SHIFTS[$i]),
                );
                a = tmp;
            }
        };
    }
    round!(0..16, |b, c, d| (b & c) | (!b & d), |i| i);
    round!(16..32, |b, c, d| (d & b) | (!d & c), |i| (5 * i + 1) % 16);
    round!(32..48, |b, c, d| b ^ c ^ d, |i| (3 * i + 5) % 16);
    round!(48..64, |b, c, d| c ^ (b | !d), |i| (7 * i) % 16);
    state[0] = state[0].wrapping_add(a);
    state[1] = state[1].wrapping_add(b);
    state[2] = state[2].wrapping_add(c);
    state[3] = state[3].wrapping_add(d);
}

#[cfg(test)]
mod tests {
    use super::digest;

    fn hex(bytes: [u8; 16]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// RFC 1321's own test suite: what says this is MD5, not something that
    /// mixes bits.
    #[test]
    fn rfc_1321_test_vectors() {
        for (input, want) in [
            ("", "d41d8cd98f00b204e9800998ecf8427e"),
            ("a", "0cc175b9c0f1b6a831c399e269772661"),
            ("abc", "900150983cd24fb0d6963f7d28e17f72"),
            ("message digest", "f96b697d7cb7938d525a2f31aaf161d0"),
            (
                "abcdefghijklmnopqrstuvwxyz",
                "c3fcd3d76192e4007dfb496cca67e13b",
            ),
            (
                "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789",
                "d174ab98d277d9f5a5611c2c9f419d9f",
            ),
            (
                "1234567890123456789012345678901234567890\
                 1234567890123456789012345678901234567890",
                "57edf4a22be3c955ac49da2e2107b67a",
            ),
        ] {
            assert_eq!(hex(digest(input.as_bytes())), want, "MD5({input:?})");
        }
    }

    /// Lengths either side of each block boundary, where the tail pads into one block or two
    /// (digests of `x` repeated, from Python's `hashlib`).
    #[test]
    fn lengths_around_the_block_boundaries_match_hashlib() {
        for (len, want) in [
            (1, "9dd4e461268c8034f5c8564e155c67a6"),
            (55, "04364420e25c512fd958a70738aa8f72"),
            (57, "693037871c4a9d3d8685018905cb530a"),
            (63, "7dc2ca208106a2f703567bdff99d8981"),
            (64, "c1bb4f81d892b2d57947682aeb252456"),
            (65, "1bc932052302d074bdec39795fe00cf6"),
            (119, "ab347a5f68c8a443cfcddc633f12c24f"),
            (120, "fb98667f98096de92620b64f46e1c5b5"),
            (121, "e8bb0ccd83750100b37f4831cd5d7ee2"),
            (127, "a0b28c1da68705c2ff883fe279b72753"),
            (128, "d69cb61a6ee87200676eb0d4b90edbcb"),
            (129, "3926841d393c00c3f36260e5ace10dc1"),
        ] {
            assert_eq!(hex(digest(&vec![b'x'; len])), want, "MD5 of {len} bytes");
        }
    }

    /// The padding branch landing exactly on a block boundary, which an
    /// implementation gets wrong without noticing.
    #[test]
    fn a_message_of_exactly_one_block_pads_into_a_second() {
        let input = vec![b'x'; 56];
        assert_eq!(
            hex(digest(&input)),
            "668a72d5ba17f08e62dabcafad6db14b".to_string(),
            "56 bytes leaves no room for the length, so a second block is added"
        );
    }
}
