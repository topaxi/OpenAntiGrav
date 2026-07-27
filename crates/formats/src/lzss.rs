//! The LZSS variant used to compress PS2 WAD entries.
//!
//! A single MSB-first bit stream. Nothing is byte-aligned:
//!
//! ```text
//! bit 1  ->  8 bits: a literal byte
//! bit 0  ->  13 bits: an absolute ring position
//!            4 bits:  length, copy length + 3 bytes
//! ```
//!
//! The ring is 8192 bytes, zero-initialised, with the write cursor starting at
//! **1**. Positions are absolute indices into the ring, not distances back from
//! the cursor.
//!
//! Recovered from `Lzss_Decode` at `0x089419d8` in the PSP `BOOT.BIN` and
//! confirmed against the PS2 archives, which are the only shipped data that
//! uses it. See `docs/formats/lzss.md`.
//!
//! # Why the details matter
//!
//! 13 + 4 bits is 17, so a match does not fit in whole bytes. That rules out
//! the conventional Okumura arrangement, where a byte of flags is followed by
//! byte-aligned literals and 12/4 matches. Everything here comes from one
//! continuous bit stream instead.
//!
//! Absolute positions rather than back-distances mean a match can read ring
//! bytes that have not been written during this stream. We return zero for
//! those. The PS2 does not: its ring is a fresh heap allocation that is never
//! cleared, so the original reads whatever was there. No shipped stream tells
//! the two apart, because the encoder never emits such a match, but a
//! hand-built one can - see `docs/formats/lzss.md`.
//!
//! The cursor starting at 1 rather than 0 is load-bearing: starting at 0
//! corrupts roughly 1300 of 1500 bytes on the first entry tested.
//!
//! # Checked against the other binary
//!
//! `tests/lzss_ps2_reference.rs` holds a second decoder transcribed from
//! `Lzss_Decode` at `0x00214080` in the PS2 `SCES_547.48`, and the two agree on
//! every byte of 40,000 random streams. Changing any parameter in this file
//! without changing that one will fail the comparison, which is the point: the
//! unit tests below share this module's assumptions and cannot catch a
//! misreading of the format.

/// Size of the sliding-window ring buffer.
const RING_SIZE: usize = 8192;

/// Mask for ring indices. `RING_SIZE` is a power of two.
const RING_MASK: usize = RING_SIZE - 1;

/// Initial write cursor. Not zero, and not the Okumura `N - F` convention.
const RING_START: usize = 1;

/// Bits in an encoded ring position. Addresses all of `RING_SIZE`.
const POSITION_BITS: u32 = 13;

/// Bits in an encoded match length.
const LENGTH_BITS: u32 = 4;

/// Added to every encoded length, so matches are 3..=18 bytes.
const LENGTH_BIAS: usize = 3;

/// Something wrong with a compressed stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The input ran out before `expected_len` bytes had been produced.
    ///
    /// Usually means the stream is not this format, or the expected length is
    /// wrong.
    UnexpectedEnd {
        /// Bytes produced before the input ran out.
        produced: usize,
        /// Bytes the caller asked for.
        expected: usize,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnexpectedEnd { produced, expected } => write!(
                f,
                "compressed stream ended after producing {produced} of {expected} bytes"
            ),
        }
    }
}

impl std::error::Error for Error {}

/// Result alias for this module.
pub type Result<T> = std::result::Result<T, Error>;

/// Decompresses `input` into exactly `expected_len` bytes.
///
/// The length must be supplied: the format has no end-of-stream marker, and the
/// WAD directory always knows it.
///
/// # Errors
///
/// Returns [`Error::UnexpectedEnd`] if the input runs out early.
///
/// ```
/// # use oag_formats::lzss;
/// // A literal 'A' then a match copying it back: see the tests for the layout.
/// let out = lzss::decompress(&[0b1010_0000, 0b1000_0000], 1).unwrap();
/// assert_eq!(out, b"A");
/// ```
pub fn decompress(input: &[u8], expected_len: usize) -> Result<Vec<u8>> {
    // Reserve for what the input could plausibly produce, not for what it
    // claims. `expected_len` comes from a WAD entry's size field, so a hostile
    // archive can declare 2 GiB and have it committed before a single bit is
    // read. The densest possible encoding is a 17-bit match producing 18 bytes,
    // so 9 output bytes per input byte is a generous ceiling; anything beyond it
    // will fail on input exhaustion anyway, and the Vec grows if it is wrong.
    decompress_with(&mut BitReader::new(input), expected_len)
}

fn decompress_with(bits: &mut BitReader<'_>, expected_len: usize) -> Result<Vec<u8>> {
    let ceiling = bits.data.len().saturating_mul(9).max(RING_SIZE);
    let mut out = Vec::with_capacity(expected_len.min(ceiling));
    let mut ring = [0u8; RING_SIZE];
    let mut cursor = RING_START;

    while out.len() < expected_len {
        let end = || Error::UnexpectedEnd {
            produced: out.len(),
            expected: expected_len,
        };

        if bits.bit().ok_or_else(end)? == 1 {
            let byte = bits.take(8).ok_or_else(end)? as u8;
            out.push(byte);
            ring[cursor] = byte;
            cursor = (cursor + 1) & RING_MASK;
        } else {
            let position = bits.take(POSITION_BITS).ok_or_else(end)? as usize;
            let length = bits.take(LENGTH_BITS).ok_or_else(end)? as usize;

            let mut from = position & RING_MASK;
            for _ in 0..length + LENGTH_BIAS {
                // A match may legitimately run past the expected length; the
                // encoder has no reason to avoid it on the final match.
                if out.len() == expected_len {
                    break;
                }
                let byte = ring[from];
                out.push(byte);
                ring[cursor] = byte;
                from = (from + 1) & RING_MASK;
                cursor = (cursor + 1) & RING_MASK;
            }
        }
    }

    Ok(out)
}

/// Bytes a well-formed stream may leave unread at the end.
///
/// Every one of the 6,053 LZSS streams in the PS2 archives leaves **one or two**
/// bytes untouched, never zero and never three, which is what an encoder with a
/// 16-bit output bit buffer looks like when it flushes at the end.
///
/// That regularity is the useful part. A stream is read to within two bytes of
/// its end or the layout is wrong, and unlike the output length that is not a
/// property the decoder can satisfy by construction.
pub const MAX_TRAILING_BYTES: usize = 2;

/// Decompresses, and reports how much of the input was consumed.
///
/// # Why this exists
///
/// [`decompress`] stops as soon as it has produced `expected_len` bytes, so
/// `out.len() == expected_len` is a tautology on success and says nothing about
/// whether the bit layout is right. What *is* informative is where the reader
/// stopped: a correct decode consumes every input byte, since the encoder had no
/// reason to emit bytes nobody reads. A wrong field order or bit order generally
/// still terminates, but it terminates somewhere else in the stream.
///
/// So this returns how much input was touched, and
/// `leftover <= MAX_TRAILING_BYTES` is a real check where a size match is not.
///
/// # Errors
///
/// As [`decompress`].
pub fn decompress_reporting(input: &[u8], expected_len: usize) -> Result<Decoded> {
    let mut reader = BitReader::new(input);
    let bytes = decompress_with(&mut reader, expected_len)?;
    let consumed = reader.bytes_touched();
    Ok(Decoded {
        bytes,
        consumed,
        leftover: input.len() - consumed,
    })
}

/// A decompressed blob, with what it cost to read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoded {
    /// The decompressed bytes.
    pub bytes: Vec<u8>,
    /// Input bytes the reader touched, including a partially used final byte.
    pub consumed: usize,
    /// Input bytes never read. Should be zero for a well-formed stream.
    pub leftover: usize,
}

/// MSB-first bit reader.
struct BitReader<'a> {
    data: &'a [u8],
    byte: usize,
    bit: u32,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            byte: 0,
            bit: 0,
        }
    }

    fn bit(&mut self) -> Option<u8> {
        let byte = *self.data.get(self.byte)?;
        let value = (byte >> (7 - self.bit)) & 1;

        self.bit += 1;
        if self.bit == 8 {
            self.bit = 0;
            self.byte += 1;
        }
        Some(value)
    }

    /// Input bytes the reader has touched, counting a partly used byte.
    fn bytes_touched(&self) -> usize {
        self.byte + usize::from(self.bit > 0)
    }

    fn take(&mut self, count: u32) -> Option<u32> {
        let mut value = 0u32;
        for _ in 0..count {
            value = (value << 1) | u32::from(self.bit()?);
        }
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a stream from a description, so tests read as the format does.
    #[derive(Default)]
    struct Writer {
        out: Vec<u8>,
        bit: u32,
    }

    impl Writer {
        fn push(&mut self, value: u32, count: u32) {
            for i in (0..count).rev() {
                if self.bit == 0 {
                    self.out.push(0);
                }
                if (value >> i) & 1 != 0 {
                    let last = self.out.len() - 1;
                    self.out[last] |= 0x80 >> self.bit;
                }
                self.bit = (self.bit + 1) % 8;
            }
        }
        fn literal(&mut self, b: u8) {
            self.push(1, 1);
            self.push(u32::from(b), 8);
        }
        fn matched(&mut self, position: u32, length: u32) {
            self.push(0, 1);
            self.push(position, POSITION_BITS);
            self.push(length, LENGTH_BITS);
        }
    }

    #[test]
    fn decodes_literals() {
        let mut w = Writer::default();
        for b in b"WIPEOUT" {
            w.literal(*b);
        }
        assert_eq!(decompress(&w.out, 7).unwrap(), b"WIPEOUT");
    }

    #[test]
    fn decodes_a_match_from_an_absolute_ring_position() {
        let mut w = Writer::default();
        for b in b"abc" {
            w.literal(*b);
        }
        // The cursor starts at 1, so "abc" occupies ring[1..4].
        w.matched(1, 0); // position 1, length 0 + 3 = 3 bytes -> "abc"
        assert_eq!(decompress(&w.out, 6).unwrap(), b"abcabc");
    }

    #[test]
    fn the_cursor_starts_at_one() {
        // Reading ring position 0 gives the byte before the first literal,
        // which is the ring's initial zero. If the cursor started at 0 this
        // would return 'a' instead.
        let mut w = Writer::default();
        w.literal(b'a');
        w.matched(0, 0);
        assert_eq!(decompress(&w.out, 4).unwrap(), b"a\0a\0");
    }

    #[test]
    fn a_match_may_overlap_itself() {
        // Classic run-length behaviour: the match reads bytes it is writing.
        let mut w = Writer::default();
        w.literal(b'x');
        w.matched(1, 5); // 8 bytes from position 1, which is 'x' onwards
        assert_eq!(decompress(&w.out, 9).unwrap(), b"xxxxxxxxx");
    }

    #[test]
    fn a_final_match_is_cut_short_by_the_expected_length() {
        let mut w = Writer::default();
        for b in b"abcd" {
            w.literal(*b);
        }
        w.matched(1, 15); // asks for 18 bytes
        assert_eq!(decompress(&w.out, 6).unwrap(), b"abcdab");
    }

    #[test]
    fn stops_exactly_at_the_expected_length() {
        let mut w = Writer::default();
        for b in b"abcdefgh" {
            w.literal(*b);
        }
        assert_eq!(decompress(&w.out, 3).unwrap(), b"abc");
    }

    #[test]
    fn a_truncated_stream_is_an_error() {
        // Flag says literal, but the byte is missing.
        assert!(matches!(
            decompress(&[0x80], 4),
            Err(Error::UnexpectedEnd { .. })
        ));
    }

    #[test]
    fn an_empty_expectation_reads_nothing() {
        assert_eq!(decompress(&[], 0), Ok(Vec::new()));
    }

    #[test]
    fn ring_positions_wrap() {
        let mut w = Writer::default();
        w.literal(b'z');
        // Start at the last ring slot and copy three bytes, so the read wraps
        // 8191 -> 0 -> 1. Slots 8191 and 0 are still zero, but slot 1 holds the
        // 'z' just written, so the wrap is visible in the output.
        w.matched(8191, 0);
        assert_eq!(decompress(&w.out, 4).unwrap(), b"z\0\0z");
    }
}
