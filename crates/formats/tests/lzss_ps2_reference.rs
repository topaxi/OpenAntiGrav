//! Differential test: `oag_formats::lzss` against a second decoder transcribed
//! from the **PS2** executable.
//!
//! # Why this file exists
//!
//! `docs/formats/lzss.md` says outright that its evidence has one gap: the
//! decoder has only ever been checked against data and against its own inverse.
//! The encoder in the unit tests was written to match the decoder, so it is not
//! an independent oracle, and "every archive entry produced its declared size"
//! is close to a tautology because the decoder stops *on* that size.
//!
//! `reference_decode` below closes that gap. `oag_formats::lzss` was derived
//! from `Lzss_Decode` at `0x089419d8` in the PSP `BOOT.BIN`; this one is
//! transcribed from `Lzss_Decode` at `0x00214080` in the PS2 `SCES_547.48`, a
//! different binary built by a different compiler for a different ISA, and it is
//! written in the shape the PS2 code actually has - a resumable state machine
//! with a walking mask byte, a match-in-progress flag and a length stored biased
//! by two - rather than in the shape of the Rust decoder. Two transcriptions
//! from two binaries agreeing byte for byte on arbitrary input is evidence
//! neither one can manufacture alone.
//!
//! See `docs/ghidra/functions/ps2-pulse-eu/lzss.md` for the disassembly this is
//! transcribed from.
//!
//! # What it does not cover
//!
//! The PS2 ring buffer is a fresh heap allocation that is never zeroed, so what
//! the original produces for a match reading a ring slot the stream has not yet
//! written is whatever the allocator left there. Both decoders here assume
//! zeros. Random streams read unwritten slots constantly, so this test pins the
//! two implementations to each other, not to the console.

/// Ring size, from the `& 0x1fff` masks on every ring index in `Lzss_Decode`.
const RING_SIZE: usize = 8192;

/// Decodes exactly `expected_len` bytes, transcribed from PS2 `Lzss_Decode`.
///
/// Returns the bytes produced before the input ran out, as an `Err`, when the
/// stream ends early. The original has no such check: it reads past its staging
/// buffer. The bound is placed where the original would have needed another
/// input byte, which is the same point `oag_formats::lzss` gives up at.
fn reference_decode(input: &[u8], expected_len: usize) -> Result<Vec<u8>, Vec<u8>> {
    let mut s = Ps2Stream {
        input,
        cursor: 0,
        // `this+0x28`, seeded to 0x80 by Lzss_InitFromFile.
        mask: 0x80,
        // `this+0x2c`, the input byte the mask walks across.
        current: 0,
        // `this+0x24`, seeded to 1 by Lzss_InitFromFile.
        write: 1,
        // `this+0x30`, `this+0x34`, `this+0x38`, `this+0x3c`.
        in_match: false,
        progress: 0,
        length_biased: 0,
        position: 0,
        ring: [0u8; RING_SIZE],
    };

    let mut out = Vec::with_capacity(expected_len.min(1 << 16));
    while out.len() < expected_len {
        if s.in_match {
            // The PS2 copy step: one byte per iteration, reading the ring at an
            // absolute position plus how far the match has come.
            let byte = s.ring[(s.position + s.progress) & (RING_SIZE - 1)];
            out.push(byte);
            s.ring[s.write] = byte;
            s.progress += 1;
            s.write = (s.write + 1) & (RING_SIZE - 1);
            // `*(this+0x30) = *(this+0x38) < progress ^ 1`, so the match stays
            // live while progress <= length_biased and emits length_biased + 1
            // bytes in total.
            s.in_match = s.progress <= s.length_biased;
            continue;
        }

        let Some(flag) = s.flag_bit() else {
            return Err(out);
        };
        if flag {
            let Some(byte) = s.take(8) else {
                return Err(out);
            };
            let byte = byte as u8;
            out.push(byte);
            s.ring[s.write] = byte;
            s.write = (s.write + 1) & (RING_SIZE - 1);
        } else {
            let (Some(position), Some(length)) = (s.take(13), s.take(4)) else {
                return Err(out);
            };
            s.position = position as usize;
            // `*(this+0x38) = length + 2`, and the copy loop runs while
            // progress <= that, so a match is length + 3 bytes.
            s.length_biased = length as usize + 2;
            s.progress = 0;
            s.in_match = true;
        }
    }
    Ok(out)
}

#[derive(Debug)]
struct Ps2Stream<'a> {
    input: &'a [u8],
    cursor: usize,
    mask: u8,
    current: u8,
    write: usize,
    in_match: bool,
    progress: usize,
    length_biased: usize,
    position: usize,
    ring: [u8; RING_SIZE],
}

impl Ps2Stream<'_> {
    /// One bit, exactly as the PS2 code takes it.
    ///
    /// A fresh input byte is fetched whenever the mask is back at `0x80`, the
    /// mask walks right, and it is reset to `0x80` once it walks off the end.
    /// The bit's value is `current & mask` *before* the shift, which is what
    /// makes this MSB first.
    fn flag_bit(&mut self) -> Option<bool> {
        if self.mask == 0x80 {
            self.current = *self.input.get(self.cursor)?;
            self.cursor += 1;
        }
        let bit = self.mask;
        self.mask >>= 1;
        if self.mask == 0 {
            self.mask = 0x80;
        }
        Some(self.current & bit != 0)
    }

    /// `count` bits, transcribed from PS2 `Lzss_ReadBits`.
    ///
    /// The original walks an output mask `1 << (count - 1)` rightwards and ORs
    /// it in when the input bit is set, so the first bit read is the field's
    /// most significant.
    fn take(&mut self, count: u32) -> Option<u32> {
        let mut out = 0u32;
        let mut place = 1u32 << (count - 1);
        while place != 0 {
            if self.flag_bit()? {
                out |= place;
            }
            place >>= 1;
        }
        Some(out)
    }
}

/// Deterministic xorshift, so a failure reproduces exactly.
#[derive(Debug)]
struct Rng(u64);

impl Rng {
    fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, bound: usize) -> usize {
        (self.next_u64() % bound as u64) as usize
    }

    fn bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| (self.next_u64() >> 24) as u8).collect()
    }
}

/// Runs both decoders over one stream. Reports whether it decoded in full.
///
/// The distinction matters and is why this returns anything at all. A run that
/// ends in exhaustion only compares *where* the two readers gave up, which no
/// amount of ring, cursor or match-length disagreement can affect - the bit
/// consumption is the same either way. Only a complete decode compares content.
/// Callers count the completions so a test cannot quietly decay into checking
/// nothing but the exhaustion point.
fn compare(input: &[u8], expected_len: usize) -> bool {
    let ours = oag_formats::lzss::decompress(input, expected_len);
    let theirs = reference_decode(input, expected_len);

    match (&ours, &theirs) {
        (Ok(a), Ok(b)) => {
            assert_eq!(
                a, b,
                "decoders disagree on {expected_len} bytes from {input:02x?}"
            );
            true
        }
        (Err(oag_formats::lzss::Error::UnexpectedEnd { produced, .. }), Err(partial)) => {
            assert_eq!(
                *produced,
                partial.len(),
                "decoders ran out of input at different points on {input:02x?}"
            );
            false
        }
        _ => panic!(
            "one decoder succeeded and the other did not on {expected_len} \
             bytes from {input:02x?}: ours {ours:?}, reference {:?}",
            theirs.as_ref().map(Vec::len)
        ),
    }
}

/// Random bit streams, decoded to random lengths, by both implementations.
///
/// Random input is the point: it puts roughly half the flags on the match
/// branch, spreads match positions across the whole ring, and lands field
/// boundaries at every offset within a byte. A field order, a bit order, a
/// length bias or a cursor start that differed between the two would show up
/// within a handful of streams.
#[test]
fn agrees_with_the_ps2_decoder_on_random_streams() {
    let mut rng = Rng(0x5eed_1234_9abc_def0);
    let mut complete = 0;
    for _ in 0..20_000 {
        let len = 1 + rng.below(96);
        let input = rng.bytes(len);
        // Well under what the input can produce, so most runs decode fully and
        // the comparison is on content rather than on where it gave up.
        let expected_len = 1 + rng.below(input.len() * 4);
        complete += usize::from(compare(&input, expected_len));
    }
    assert!(
        complete > 15_000,
        "only {complete} of 20000 streams decoded in full, so this test is \
         mostly comparing exhaustion points rather than output"
    );
}

/// The same, driven past the end of the input on purpose.
///
/// Asking for far more output than the stream can supply makes every run end in
/// exhaustion, which checks that both decoders consume the same number of bits
/// before giving up - a stricter constraint than agreeing on a complete decode.
#[test]
fn agrees_with_the_ps2_decoder_when_the_input_runs_out() {
    let mut rng = Rng(0x0bad_c0de_1234_5678);
    for _ in 0..20_000 {
        let len = 1 + rng.below(24);
        let input = rng.bytes(len);
        compare(&input, input.len() * 12 + 64);
    }
}

/// Streams built entirely from one repeated byte.
///
/// All-zero input is every flag on the match branch, position 0, length 0, so
/// it exercises reading ring slots this stream never wrote. All-ones is every
/// flag on the literal branch. Neither is likely to come up at random.
#[test]
fn agrees_with_the_ps2_decoder_on_degenerate_streams() {
    for fill in [0x00u8, 0x01, 0x55, 0xaa, 0xfe, 0xff] {
        for len in [1usize, 2, 3, 7, 16, 33] {
            compare(&vec![fill; len], 512);
        }
    }
    // An all-zero stream is a run of minimum-length matches at ring position 0:
    // eighteen bits in, three bytes out. 4 KiB of input therefore carries about
    // 5,460 bytes of output, so asking for 4,096 compares content rather than
    // stopping at an exhausted reader.
    assert!(
        compare(&[0x00; 4096], 4096),
        "the all-zero stream exhausted, so nothing was compared"
    );
}

/// The ring wraps at 8192 and the write cursor starts at 1, so a stream long
/// enough to fill the ring puts the two implementations' cursors out of step if
/// either one wraps differently.
///
/// The output length is chosen so these decode in full rather than exhausting:
/// random flags average a little over eleven output bytes per twenty-seven bits
/// of input, so 4 KiB of input carries well past one wrap.
#[test]
fn agrees_with_the_ps2_decoder_across_a_full_ring_wrap() {
    let mut rng = Rng(0xfeed_face_dead_beef);
    for _ in 0..64 {
        let input = rng.bytes(4096);
        assert!(
            compare(&input, RING_SIZE + 1024),
            "stream exhausted before the ring wrapped, so nothing was compared"
        );
    }
}
