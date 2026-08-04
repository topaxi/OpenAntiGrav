# LZSS compression

**Status: understood.** Implemented in
[`oag-formats::lzss`](../../crates/formats/src/lzss.rs) and validated against
every compressed entry in both PS2 archives.

Used for [WAD](wad.md) entries whose stored size differs from their uncompressed
size and whose zlib flag is clear. That is 5,861 of 7,200 entries in
`WADS2.WAD` and 192 of 193 in `WADSP.WAD`. **No PSP archive uses it.**

**Validated against one disc**, `pulse-ps2-eu`, because it is the only one that
compresses anything. *Wipeout Pure*'s UMD was checked and stores all 1,229 of
its entries uncompressed, exactly like Pulse PSP, so Pure adds no validation
here - a deliberate non-result, recorded in the [Pure probe](pure-status.md) so
nobody spends the afternoon again.

## Format

A single MSB-first bit stream. Nothing is byte-aligned.

```text
bit 1  ->  8 bits   literal byte
bit 0  ->  13 bits  absolute ring position
           4 bits   length; copy length + 3 bytes
```

| Parameter | Value |
| --- | --- |
| Ring size | 8192 bytes, zero-initialised |
| Initial write cursor | **1** |
| Position encoding | Absolute index into the ring |
| Match length | 3 to 18 bytes |
| Bit order | MSB first |
| End of stream | None; the caller supplies the expected length |

Every byte written, whether literal or copied, also goes into the ring at the
cursor, which then advances and wraps.

## Three details that are easy to get wrong

**It is not byte-aligned.** 13 + 4 is 17 bits. The conventional Okumura
arrangement uses 12/4 precisely so a match fits in two bytes, with a separate
byte of flag bits and byte-aligned literals. This is not that: flags, literals
and match fields all come from one continuous bit stream.

**Positions are absolute, not distances back.** A match can therefore read ring
slots this stream has not written. Our decoder reads those as zero, and no
shipped stream distinguishes the choice - but see
[the ring is not zeroed](#the-ring-is-not-zeroed) below, because the PS2 does
not zero it and the two are only equivalent for data the shipped encoder
produced.

**The cursor starts at 1, not 0.** This looks like a detail and is not. Starting
at 0 corrupts 1,333 of 1,503 bytes on the first entry tested. It is also not the
Okumura `N - F` convention, which would be 8174 and corrupts 1,358 of 1,503.

## Evidence

Read from `Lzss_Decode` at `0x089419d8` in the PSP `BOOT.BIN`, with the bit
reader at `0x08941c84`. See
[the WAD subsystem page](../ghidra/functions/psp-pulse-usa/wad-subsystem.md).

**And, independently, from `Lzss_Decode` at `0x00214080` in the PS2
`SCES_547.48`**, with the bit reader at `0x00213f70`. See
[the PS2 LZSS page](../ghidra/functions/ps2-pulse-eu/lzss.md). Every parameter in
the table above is visible there: the `& 0x1fff` on each ring index, the write
cursor seeded to `1`, the 13-bit position used as a direct ring index with
nothing subtracted, the 4-bit length stored biased by two into a loop that runs
one iteration longer than the bias, and a flag mask byte seeded to `0x80` and
shifted right. Two different compilers, two different instruction sets, the
same format.

Since no PSP archive contains a compressed entry, the static reading could not
be round-tripped there and sat at confidence 82. The PS2 archives supplied the
missing corpus.

Candidate parameters were tested against four real entries by brute force. Six
variants produced the correct output *length*, so length alone does not settle
it. Content does: the first sample decompresses to XML, and only one variant
makes it **100% printable**.

| Variant | Printable | Bytes differing from the winner |
| --- | ---: | ---: |
| cursor 1, absolute | **100.0%** | - |
| cursor 0, relative | 99.9% | 1,199 |
| cursor 0, absolute | 94.9% | 1,333 |
| cursor 8174, absolute | 91.9% | 1,358 |

The winner's output parses as valid XML:

```xml
<?xml version="1.0" encoding="utf-8" ?>
<Screen name="Top">
  <LoadXML>
    <Values Src="Data\Plugins\grids\grid_00.xml"></Values>
  </LoadXML>
  ...
</Screen>
```

## Mass validation

```sh
oag-wad verify data/images/pulse-ps2-eu.chd:54748/WADS2.WAD
```

| Archive | Entries | LZSS | Stored | Decompressed | Result |
| --- | ---: | ---: | ---: | ---: | --- |
| `WADSP.WAD` | 193 | 192 | 1.4 MiB | 4.3 MiB | all correct |
| `WADS2.WAD` | 7,200 | 5,861 | 221 MiB | 516 MiB | all correct |

Every entry produced **exactly** its declared uncompressed size.

That sentence needs a caveat, and the caveat is the interesting part. The
decoder loops `while out.len() < expected_len`, so on success the output length
is a **tautology**: it cannot come out any other way. "All 7,393 entries produced
the right size" therefore only means that no stream ran out of input early, which
is a real signal but a much weaker one than it reads as.

The check with teeth is where the *reader* stopped:

| Archive | LZSS streams | 1 byte left | 2 bytes left | 0 or 3+ |
| --- | ---: | ---: | ---: | ---: |
| `WADSP.WAD` | 192 | 49 | 143 | 0 |
| `WADS2.WAD` | 5,861 | 1,391 | 4,470 | 0 |

**Every stream ends one or two bytes short of its stored length**, never exactly
at the end and never further back. That is not something the decoder can arrange
for itself: it stops on output length and never looks at the trailer, so landing
within two bytes of the end 6,053 times is an independent constraint on the bit
layout being right. A wrong field order or bit order terminates too, just not
there.

One or two bytes of slack is what an encoder with a **16-bit output bit buffer**
leaves when it flushes at the end, which is the obvious reading and is not
verified. What the trailing bytes contain has not been checked.

`oag-wad verify` asserts the bound (`lzss::MAX_TRAILING_BYTES`) and prints the
histogram, so this stays a test rather than a paragraph.

## An independent decoder, from the other binary

The paragraph above used to end by admitting the gap: "the encoder that exists
in the tests was written to match the decoder, so it is not an independent
oracle." That gap is now closed from the code side.

[`crates/formats/tests/lzss_ps2_reference.rs`](../../crates/formats/tests/lzss_ps2_reference.rs)
holds a second decoder transcribed from the **PS2** `Lzss_Decode`, written in
the shape that function actually has - a resumable state machine with a walking
mask byte, a match-in-progress flag and a length biased by two - rather than in
the shape of `oag_formats::lzss`, which came from the PSP. The two are compared
over 40,000 pseudorandom bit streams, a full ring wrap, and degenerate
all-one-byte streams, and they agree on every byte.

Random input is the point. It puts about half the flags on the match branch,
spreads positions across the whole ring, lands field boundaries at every offset
within a byte, and reads unwritten ring slots constantly - none of which the
shipped archives are guaranteed to do. Eight deliberate mutations of the
reference were each caught by the comparison (length bias, flag polarity, bit
order within a field, field order, mask reset point, cursor start, absolute
versus relative positions, window size), so it is a real check and not a pair of
implementations agreeing by construction.

One caveat on the word "independent", since closing an independence gap is this
section's whole job. The reference was written after reading
`oag_formats::lzss`, by the same author, in the same session. The mutation sweep
shows the comparison discriminates - eight wrong readings of the PS2 code all
fail it - not that the second transcription was arrived at blind. A blind
transcription, or a trace, would close what is left.

Confidence: **94**, unchanged, but resting on two legs instead of one.

The number does not move, and that is deliberate. The
[rubric](../reverse-engineering/confidence-rubric.md)'s top band needs a runtime
trace **and** a second binary; this now has the second binary and still has no
trace. Nothing here has ever been executed under an emulator on either platform,
and the code-agreement leg cannot supply that: two readers can share a
misreading of the same original if the original itself was misread the same way
twice. What has changed is how much of the remaining doubt is about the *bit
layout* - very little - versus about the surrounding assumptions, which is where
it now lives.

### The ring is not zeroed

The PS2 allocates its 8192-byte ring from the ordinary heap allocator on every
stream and frees it on close, with no clear in between, so a match reading a
slot the current stream has not written picks up whatever was there before.
Our decoder returns zero for those slots.

All 6,053 shipped streams decode correctly under the zero assumption, so the
shipped encoder evidently never emits such a match - but that is a fact about
the encoder, not about the format, and it means **a hand-built or fuzzed stream
can distinguish our decoder from the original where no real archive can**. If a
tool ever needs to round-trip against the console rather than against the disc,
this is the assumption to revisit first. Confidence 80, from reading the
allocator rather than tracing it.

### The window size, confirmed by a buffer nobody reads

The PS2's LZSS init also allocates 98,316 bytes that nothing in the binary ever
touches. That is exactly `3 * (8192 + 1) * 4`: Okumura's `lson`, `rson` and
`dad` arrays for a window of 8192 - the search tree an LZSS *compressor* needs
and a decompressor does not.

So the retail build carries the encoder's workspace allocation with the encoder
itself compiled out, and its size independently pins the window at 8192 without
reference to any decode path. Details on
[the PS2 LZSS page](../ghidra/functions/ps2-pulse-eu/lzss.md).

## Not implemented

The game also supports **zlib 1.2.2**, selected by bit 31 of the entry's
uncompressed-size field. No shipped archive sets that bit, so there is nothing
to validate an implementation against; `oag-wad` refuses such entries rather
than shipping an unverified decoder. If one ever turns up, the decoders are at
`0x08940f48` (memory) and `0x08940f70` (streaming).

The decoder in the game stages input through a **global** 4 KiB buffer at
`0x08b66450`, so the original is not reentrant. Ours is, since it reads from a
caller-supplied slice.

The PS2 build is the same in kind: a global 1 KiB staging buffer, a global ring,
and a process-wide lock taken for the lifetime of every decompressing stream, so
one entry decompresses at a time. The buffer size differs and nothing about the
format depends on it - it is a read-ahead window on the input, not part of the
encoding.
