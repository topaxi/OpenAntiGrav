//! Which end of a word comes first, for the formats two consoles share.
//!
//! Every Wipeout asset format here is authored by the same Maya exporter, and
//! the PS3 build writes the *same layouts* with the bytes reversed (see
//! `docs/formats/hd-status.md`), so a parser needs one extra piece of
//! information and no other change.
//!
//! # It is read from the data, never from the console
//!
//! `oag_assets::source::Layout` states the rule and names its one exception. A
//! `.vex` obeys it for free: the magic at `+0x0c` is `VEXX` on PSP and PS2 and
//! `XXEV` on PS3, which `oag_vex::vex::byte_order` sniffs. A payload *inside*
//! that file cannot (a `WO Track` magic reads `WOtd` in both games), so those
//! parsers take the order their container sniffed: still data, not a platform
//! branch.
//!
//! # The trap this type exists to prevent
//!
//! Swapping each field in place is not reading the file big-endian, and the
//! difference is silent. `oag_vex::pvs` reads a 64-bit visibility mask as two
//! `u32`s, identical to one `u64` on a little-endian file and **wrong** on a
//! big-endian one (halves reversed). Measured, that is 100 % dangling on 15 of
//! Wipeout HD's 24 circuits with no error. [`ByteOrder::u64`] reads the eight
//! bytes as one quantity so the question never arises.

/// Which end of a multi-byte field comes first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ByteOrder {
    /// PSP and PS2: least significant byte first.
    #[default]
    Little,
    Big,
}

impl ByteOrder {
    /// A 16-bit field at `at`.
    ///
    /// # Panics
    ///
    /// If `data` is shorter than `at + 2`. Callers have already bounds-checked
    /// the record.
    #[must_use]
    pub fn u16(self, data: &[u8], at: usize) -> u16 {
        let bytes = [data[at], data[at + 1]];
        match self {
            Self::Little => u16::from_le_bytes(bytes),
            Self::Big => u16::from_be_bytes(bytes),
        }
    }

    /// A 32-bit field at `at`.
    ///
    /// # Panics
    ///
    /// If `data` is shorter than `at + 4`.
    #[must_use]
    pub fn u32(self, data: &[u8], at: usize) -> u32 {
        let bytes = [data[at], data[at + 1], data[at + 2], data[at + 3]];
        match self {
            Self::Little => u32::from_le_bytes(bytes),
            Self::Big => u32::from_be_bytes(bytes),
        }
    }

    /// A 64-bit field at `at`, read as **one** quantity.
    ///
    /// Not two [`ByteOrder::u32`]s; see the module docs for the 22 %-versus-56 %
    /// measurement.
    ///
    /// # Panics
    ///
    /// If `data` is shorter than `at + 8`.
    #[must_use]
    pub fn u64(self, data: &[u8], at: usize) -> u64 {
        let bytes: [u8; 8] = data[at..at + 8].try_into().expect("eight bytes");
        match self {
            Self::Little => u64::from_le_bytes(bytes),
            Self::Big => u64::from_be_bytes(bytes),
        }
    }

    /// A signed 16-bit field at `at`.
    ///
    /// # Panics
    ///
    /// If `data` is shorter than `at + 2`.
    #[must_use]
    pub fn i16(self, data: &[u8], at: usize) -> i16 {
        self.u16(data, at) as i16
    }

    /// A 32-bit float at `at`.
    ///
    /// # Panics
    ///
    /// If `data` is shorter than `at + 4`.
    #[must_use]
    pub fn f32(self, data: &[u8], at: usize) -> f32 {
        f32::from_bits(self.u32(data, at))
    }
}

#[cfg(test)]
mod tests {
    use super::ByteOrder;

    /// The same four bytes, read both ways round.
    #[test]
    fn the_two_orders_disagree_on_everything_but_a_palindrome() {
        let data = [0x12, 0x34, 0x56, 0x78];
        assert_eq!(ByteOrder::Little.u32(&data, 0), 0x7856_3412);
        assert_eq!(ByteOrder::Big.u32(&data, 0), 0x1234_5678);
        assert_eq!(ByteOrder::Little.u16(&data, 0), 0x3412);
        assert_eq!(ByteOrder::Big.u16(&data, 0), 0x1234);

        let palindrome = [0xab, 0xab];
        assert_eq!(
            ByteOrder::Little.u16(&palindrome, 0),
            ByteOrder::Big.u16(&palindrome, 0),
            "which is why a byte-order bug can hide in a file for a long time"
        );
    }

    /// The PVS trap, in miniature: one 64-bit read against two 32-bit ones.
    #[test]
    fn a_64_bit_field_is_not_two_32_bit_fields_swapped_in_place() {
        let data: [u8; 8] = [0, 0, 0, 1, 0, 0, 0, 2];

        assert_eq!(ByteOrder::Big.u64(&data, 0), 0x0000_0001_0000_0002);

        let lo = u64::from(ByteOrder::Big.u32(&data, 0));
        let hi = u64::from(ByteOrder::Big.u32(&data, 4));
        assert_eq!(
            (hi << 32) | lo,
            0x0000_0002_0000_0001,
            "the halves come out the other way round, and nothing errors"
        );
    }

    #[test]
    fn a_float_is_the_bit_pattern_its_order_names() {
        let one = 1.0f32;
        assert_eq!(ByteOrder::Little.f32(&one.to_le_bytes(), 0), one);
        assert_eq!(ByteOrder::Big.f32(&one.to_be_bytes(), 0), one);
    }

    #[test]
    fn a_signed_16_bit_field_keeps_its_sign() {
        let minus_two = (-2i16).to_be_bytes();
        assert_eq!(ByteOrder::Big.i16(&minus_two, 0), -2);
        assert_eq!(ByteOrder::Little.i16(&(-2i16).to_le_bytes(), 0), -2);
    }
}
