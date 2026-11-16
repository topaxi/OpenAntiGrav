/// The bytes of the NUL-terminated string at `at`, without the NUL.
///
/// `None` when `at` is past the end or no NUL follows it. An empty string is
/// `Some(&[])`.
#[must_use]
pub fn cstr_at(data: &[u8], at: usize) -> Option<&[u8]> {
    let rest = data.get(at..)?;
    let end = rest.iter().position(|&b| b == 0)?;
    Some(&rest[..end])
}

/// Like [`cstr_at`], but a string that runs to the end of `data` without a
/// NUL ends there instead of failing.
#[must_use]
pub fn cstr_at_or_end(data: &[u8], at: usize) -> Option<&[u8]> {
    let rest = data.get(at..)?;
    let end = rest.iter().position(|&b| b == 0).unwrap_or(rest.len());
    Some(&rest[..end])
}

use crate::ByteOrder;

macro_rules! word_at {
    ($name:ident, $ty:ty, $n:literal) => {
        /// The word at `at` in the given order, or `None` when too few bytes remain.
        #[must_use]
        pub fn $name(data: &[u8], at: usize, order: ByteOrder) -> Option<$ty> {
            let bytes: [u8; $n] = data.get(at..at.checked_add($n)?)?.try_into().ok()?;
            Some(match order {
                ByteOrder::Little => <$ty>::from_le_bytes(bytes),
                ByteOrder::Big => <$ty>::from_be_bytes(bytes),
            })
        }
    };
}

word_at!(u16_at, u16, 2);
word_at!(u32_at, u32, 4);
word_at!(u64_at, u64, 8);

/// A 32-bit float at `at` in the given order, or `None` when fewer than four
/// bytes remain.
#[must_use]
pub fn f32_at(data: &[u8], at: usize, order: ByteOrder) -> Option<f32> {
    u32_at(data, at, order).map(f32::from_bits)
}

macro_rules! fixed_order {
    ($(#[$doc:meta])* $module:ident, $order:expr) => {
        $(#[$doc])*
        pub mod $module {
            use crate::ByteOrder;

            #[must_use]
            pub fn u16_at(data: &[u8], at: usize) -> Option<u16> {
                super::u16_at(data, at, $order)
            }

            #[must_use]
            pub fn u32_at(data: &[u8], at: usize) -> Option<u32> {
                super::u32_at(data, at, $order)
            }

            #[must_use]
            pub fn u64_at(data: &[u8], at: usize) -> Option<u64> {
                super::u64_at(data, at, $order)
            }

            #[must_use]
            pub fn f32_at(data: &[u8], at: usize) -> Option<f32> {
                super::f32_at(data, at, $order)
            }
        }
    };
}

fixed_order!(
    /// [`u16_at`] and its siblings for a little-endian format, without the order argument.
    le,
    ByteOrder::Little
);
fixed_order!(
    /// [`u16_at`] and its siblings for a big-endian format, without the order argument.
    be,
    ByteOrder::Big
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_read_in_the_named_order_or_not_at_all() {
        let d = [0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0];
        assert_eq!(u16_at(&d, 0, ByteOrder::Big), Some(0x1234));
        assert_eq!(le::u16_at(&d, 0), Some(0x3412));
        assert_eq!(be::u32_at(&d, 4), Some(0x9abc_def0));
        assert_eq!(le::u32_at(&d, 4), Some(0xf0de_bc9a));
        assert_eq!(be::u64_at(&d, 0), Some(0x1234_5678_9abc_def0));
        assert_eq!(le::f32_at(&1.5f32.to_le_bytes(), 0), Some(1.5));
        assert_eq!(le::u32_at(&d, 5), None);
        assert_eq!(le::u16_at(&d, 7), None);
        assert_eq!(le::u16_at(&d, usize::MAX), None);
    }

    #[test]
    fn cstr_at_stops_at_the_nul() {
        assert_eq!(cstr_at(b"ab\0cd\0", 3), Some(&b"cd"[..]));
        assert_eq!(cstr_at(b"ab\0", 2), Some(&b""[..]));
    }

    #[test]
    fn cstr_at_wants_a_nul_and_an_offset_in_range() {
        assert_eq!(cstr_at(b"ab", 0), None);
        assert_eq!(cstr_at(b"ab\0", 4), None);
    }

    #[test]
    fn cstr_at_or_end_accepts_an_unterminated_tail() {
        assert_eq!(cstr_at_or_end(b"ab", 0), Some(&b"ab"[..]));
        assert_eq!(cstr_at_or_end(b"ab", 2), Some(&b""[..]));
        assert_eq!(cstr_at_or_end(b"ab", 3), None);
    }
}
