//! Bitstream reader used throughout the ATRAC9 decoder.

/// Lightweight bit reader over a byte slice.
/// Position is tracked in *bits*.
#[derive(Debug, Clone)]
pub struct BitReader<'a> {
    buffer: &'a [u8],
    /// Current bit position (0 = first bit of first byte).
    position: usize,
}

impl<'a> BitReader<'a> {
    /// Create a new reader starting at bit 0.
    #[inline]
    pub fn new(buffer: &'a [u8]) -> Self {
        Self {
            buffer,
            position: 0,
        }
    }

    /// Current bit position.
    #[inline]
    pub fn position(&self) -> usize {
        self.position
    }

    /// Remaining bits in the buffer (approximate; does not clamp on over-read).
    #[inline]
    pub fn remaining_bits(&self) -> isize {
        (self.buffer.len() * 8) as isize - self.position as isize
    }

    /// Peek `bits` bits without advancing the position.
    #[inline]
    pub fn peek(&self, bits: u32) -> u32 {
        debug_assert!(bits <= 32);
        if bits == 0 {
            return 0;
        }

        let byte_index = self.position / 8;
        let bit_index = self.position % 8;
        let buffer = self.buffer;

        // Four whole bytes cover any read of up to 25 bits at any bit offset: the same bits
        // the narrower paths below cut out of fewer bytes.
        if bits <= 25 {
            if let Some(&[b0, b1, b2, b3]) = buffer
                .get(byte_index..)
                .and_then(|rest| rest.first_chunk::<4>())
            {
                let word = u32::from_be_bytes([b0, b1, b2, b3]);
                return (word << bit_index) >> (32 - bits);
            }
        }

        // Fast paths matching the original C implementation.
        if bits <= 9 {
            let b0 = *buffer.get(byte_index).unwrap_or(&0) as u32;
            let b1 = *buffer.get(byte_index + 1).unwrap_or(&0) as u32;
            let mut value = (b0 << 8) | b1;
            value &= 0xFFFF >> bit_index;
            value >>= 16 - bits - bit_index as u32;
            return value;
        }

        if bits <= 17 {
            let b0 = *buffer.get(byte_index).unwrap_or(&0) as u32;
            let b1 = *buffer.get(byte_index + 1).unwrap_or(&0) as u32;
            let b2 = *buffer.get(byte_index + 2).unwrap_or(&0) as u32;
            let mut value = (b0 << 16) | (b1 << 8) | b2;
            value &= 0xFFFFFF >> bit_index;
            value >>= 24 - bits - bit_index as u32;
            return value;
        }

        if bits <= 25 {
            let b0 = *buffer.get(byte_index).unwrap_or(&0) as u32;
            let b1 = *buffer.get(byte_index + 1).unwrap_or(&0) as u32;
            let b2 = *buffer.get(byte_index + 2).unwrap_or(&0) as u32;
            let b3 = *buffer.get(byte_index + 3).unwrap_or(&0) as u32;
            let mut value = (b0 << 24) | (b1 << 16) | (b2 << 8) | b3;
            value &= 0xFFFFFFFFu32.wrapping_shr(bit_index as u32);
            value >>= 32 - bits - bit_index as u32;
            return value;
        }

        // Fallback for larger bit counts (rarely used).
        self.peek_fallback(bits)
    }

    fn peek_fallback(&self, mut bit_count: u32) -> u32 {
        let mut value = 0u32;
        let mut byte_index = self.position / 8;
        let mut bit_index = self.position % 8;
        let buffer = self.buffer;

        while bit_count > 0 {
            if bit_index >= 8 {
                bit_index = 0;
                byte_index += 1;
            }
            let bits_to_read = (8 - bit_index).min(bit_count as usize);
            let mask = (1u32 << bits_to_read) - 1;
            let byte = *buffer.get(byte_index).unwrap_or(&0) as u32;
            let extracted = (byte >> (8 - bit_index - bits_to_read)) & mask;
            value = (value << bits_to_read) | extracted;
            bit_count -= bits_to_read as u32;
            bit_index += bits_to_read;
        }
        value
    }

    /// Read `bits` bits and advance the position.
    #[inline]
    pub fn read(&mut self, bits: u32) -> u32 {
        let value = self.peek(bits);
        self.position += bits as usize;
        value
    }

    /// Read a signed integer of `bits` bits (sign-extended).
    #[inline]
    pub fn read_signed(&mut self, bits: u32) -> i32 {
        let value = self.read(bits) as i32;
        sign_extend32(value, bits as i32)
    }

    /// Read an offset-binary value of `bits` bits.
    #[inline]
    pub fn read_offset_binary(&mut self, bits: u32) -> i32 {
        let offset = 1i32 << (bits - 1);
        self.read(bits) as i32 - offset
    }

    /// Align the bit position to the next multiple of `multiple` bits.
    pub fn align(&mut self, multiple: usize) {
        if multiple == 0 {
            return;
        }
        let rem = self.position % multiple;
        if rem != 0 {
            self.position += multiple - rem;
        }
    }

    /// Skip `bits` bits.
    #[inline]
    pub fn skip(&mut self, bits: u32) {
        self.position += bits as usize;
    }
}

/// Sign-extend a value that occupies the low `bits` bits.
#[inline]
pub fn sign_extend32(value: i32, bits: i32) -> i32 {
    let shift = 32 - bits;
    (value << shift) >> shift
}

/// `(value + 0.5).floor() as i32`, the C decoder's `Round`, without the `floor` call: on the
/// baseline x86-64 target that call is a software routine, and it ran once per output sample.
/// `as i32` truncates toward zero, saturates and maps NaN to 0, which `floor(..) as i32` does
/// too, so a truncation that landed above a negative non-integer is the only step down.
#[inline]
pub fn round_half_up(value: f64) -> i32 {
    let x = value + 0.5;
    let t = x as i32;
    t.saturating_sub(i32::from(x < f64::from(t)))
}

/// Clamp a value into the i16 range.
#[inline]
pub fn clamp16(value: i32) -> i16 {
    value.clamp(i16::MIN as i32, i16::MAX as i32) as i16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_half_up_is_the_floor_of_the_value_plus_a_half() {
        let reference = |v: f64| (v + 0.5).floor() as i32;
        let mut cases = vec![
            0.0,
            -0.0,
            0.5,
            -0.5,
            -0.500_000_000_000_000_1,
            0.499_999_999_999_999_9,
            1.5,
            -1.5,
            32_767.5,
            -32_768.5,
            2_147_483_646.5,
            2_147_483_647.0,
            2_147_483_647.5,
            2_147_483_648.0,
            -2_147_483_648.0,
            -2_147_483_648.5,
            -2_147_483_649.5,
            1e300,
            -1e300,
            f64::MAX,
            f64::MIN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
            f64::MIN_POSITIVE,
        ];
        let mut state = 0x9e37_79b9_7f4a_7c15u64;
        for _ in 0..200_000 {
            state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            cases.push(f64::from_bits(state));
            cases.push((state >> 11) as f64 / (1u64 << 40) as f64 - 4096.0);
            cases.push(f64::from((state >> 40) as i32) / 64.0 + 0.5);
        }
        for v in cases {
            assert_eq!(round_half_up(v), reference(v), "{v:e}");
        }
    }

    #[test]
    fn peek_equals_the_bit_by_bit_reader_at_every_offset_and_length() {
        let mut state = 0x9e37_79b9_7f4a_7c15u64;
        let data: Vec<u8> = (0..19)
            .map(|_| {
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                (state >> 56) as u8
            })
            .collect();
        for position in 0..data.len() * 8 + 40 {
            let mut br = BitReader::new(&data);
            br.skip(position as u32);
            for bits in 1..=32u32 {
                assert_eq!(
                    br.peek(bits),
                    br.peek_fallback(bits),
                    "{bits} bits at bit {position}"
                );
            }
        }
    }

    #[test]
    fn read_simple() {
        // 0b1010_1100 0b0011_1111
        let data = [0xAC, 0x3F];
        let mut br = BitReader::new(&data);
        assert_eq!(br.read(4), 0b1010);
        assert_eq!(br.read(4), 0b1100);
        assert_eq!(br.read(6), 0b001111);
        assert_eq!(br.position(), 14);
    }

    #[test]
    fn signed_and_offset() {
        let data = [0b1111_0000];
        let mut br = BitReader::new(&data);
        // 4 bits: 0b1111 → signed -1
        assert_eq!(br.read_signed(4), -1);
    }
}
