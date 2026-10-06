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
//! the Okumura arrangement (a flag byte, byte-aligned literals, 12/4 matches).
//!
//! Absolute positions mean a match can read ring bytes not yet written in this
//! stream. We return zero; the PS2's ring is a never-cleared heap allocation,
//! so the original reads whatever was there. No shipped stream tells the two
//! apart (the encoder never emits such a match), but a hand-built one can; see
//! `docs/formats/lzss.md`.
//!
//! The cursor starting at 1 is load-bearing: at 0 roughly 1300 of 1500 bytes
//! of the first entry tested are corrupt.
//!
//! # Checked against the other binary
//!
//! `tests/lzss_ps2_reference.rs` holds a second decoder transcribed from
//! `Lzss_Decode` at `0x00214080` in the PS2 `SCES_547.48`; the two agree on
//! every byte of 40,000 random streams. The unit tests below share this
//! module's assumptions and cannot catch a misreading of the format; that
//! comparison can.

/// Size of the sliding-window ring buffer.
const RING_SIZE: usize = 8192;

/// Mask for ring indices. `RING_SIZE` is a power of two.
const RING_MASK: usize = RING_SIZE - 1;

/// Initial write cursor. Not zero, and not the Okumura `N - F` convention.
const RING_START: usize = 1;

const POSITION_BITS: u32 = 13;

const LENGTH_BITS: u32 = 4;

/// Added to every encoded length, so matches are 3..=18 bytes.
const LENGTH_BIAS: usize = 3;

/// Something wrong with a compressed stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The input ran out before `expected_len` bytes had been produced.
    /// Usually the stream is not this format or the expected length is wrong.
    UnexpectedEnd {
        /// Bytes produced before the input ran out.
        produced: usize,
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

pub type Result<T> = std::result::Result<T, Error>;

/// Decompresses `input` into exactly `expected_len` bytes.
///
/// The length must be supplied: the format has no end-of-stream marker.
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
    // Reserve for what the input could plausibly produce, not what it claims:
    // `expected_len` is a WAD size field, so a hostile archive could declare
    // 2 GiB. The densest encoding is a 17-bit match producing 18 bytes, so 9
    // output bytes per input byte is a generous ceiling; the Vec grows if wrong.
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
                // A match may run past the expected length; the encoder has no
                // reason to avoid it on the final match.
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
/// All 6,053 LZSS streams in the PS2 archives leave **one or two** bytes
/// untouched, never zero or three: an encoder with a 16-bit output bit buffer
/// flushing at the end. A stream is read to within two bytes of its end or the
/// layout is wrong, which the decoder cannot satisfy by construction.
pub const MAX_TRAILING_BYTES: usize = 2;

/// Decompresses, and reports how much of the input was consumed.
///
/// # Why this exists
///
/// [`decompress`] stops once it has `expected_len` bytes, so
/// `out.len() == expected_len` is a tautology on success. What is informative
/// is where the reader stopped: a correct decode consumes every input byte,
/// while a wrong field or bit order terminates somewhere else. So
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
        // Ring position 0 is the ring's initial zero; with the cursor at 0 this
        // would return 'a'.
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
        // Copy three bytes from the last ring slot so the read wraps 8191 -> 0
        // -> 1; slot 1 holds the 'z' just written, so the wrap shows.
        w.matched(8191, 0);
        assert_eq!(decompress(&w.out, 4).unwrap(), b"z\0\0z");
    }
}
