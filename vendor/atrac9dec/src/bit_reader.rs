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
    pub fn peek(&self, bits: u32) -> u32 {
        debug_assert!(bits <= 32);
        if bits == 0 {
            return 0;
        }

        let byte_index = self.position / 8;
        let bit_index = self.position % 8;
        let buffer = self.buffer;

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

/// Clamp a value into the i16 range.
#[inline]
pub fn clamp16(value: i32) -> i16 {
    value.clamp(i16::MIN as i32, i16::MAX as i32) as i16
}

#[cfg(test)]
mod tests {
    use super::*;

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
