# LZSS compression

**Status: understood.** Implemented in
[`oag-formats::lzss`](../../crates/formats/src/lzss.rs) and validated against
every compressed entry in both PS2 archives.

Used for [WAD](wad.md) entries whose stored size differs from their uncompressed
size and whose zlib flag is clear. That is 5,861 of 7,200 entries in
`WADS2.WAD` and 192 of 193 in `WADSP.WAD`. **No PSP archive uses it.**

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
slots this stream has not written, which read as zero. The ring's initial
contents are part of the format.

**The cursor starts at 1, not 0.** This looks like a detail and is not. Starting
at 0 corrupts 1,333 of 1,503 bytes on the first entry tested. It is also not the
Okumura `N - F` convention, which would be 8174 and corrupts 1,358 of 1,503.

## Evidence

Read from `Lzss_Decode` at `0x089419d8` in the PSP `BOOT.BIN`, with the bit
reader at `0x08941c84`. See
[the WAD subsystem page](../ghidra/functions/psp-pulse/wad-subsystem.md).

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

Confidence: **94**. The bit layout is constrained from two directions now, output
length and input consumption, on 6,053 real streams. Not higher because nothing
was executed under an emulator: per the
[rubric](../reverse-engineering/confidence-rubric.md) that is what caps data
validation at 94, and the earlier 97 predated the rubric saying so. A compressor
was not written to check the inverse either, and the encoder that exists in the
tests was written to match the decoder, so it is not an independent oracle.

## Not implemented

The game also supports **zlib 1.2.2**, selected by bit 31 of the entry's
uncompressed-size field. No shipped archive sets that bit, so there is nothing
to validate an implementation against; `oag-wad` refuses such entries rather
than shipping an unverified decoder. If one ever turns up, the decoders are at
`0x08940f48` (memory) and `0x08940f70` (streaming).

The decoder in the game stages input through a **global** 4 KiB buffer at
`0x08b66450`, so the original is not reentrant. Ours is, since it reads from a
caller-supplied slice.
