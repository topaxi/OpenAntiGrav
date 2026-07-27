# The LZSS decoder (PS2)

Functions in `SCES_547.48` (Wipeout Pulse, PS2, SCES-54748), image base
`0x00100000`.

The format is documented in [formats/lzss.md](../../../formats/lzss.md), which
was written from the **PSP** `Lzss_Decode` and then validated against the PS2
archives as data. This page is the other half: the PS2 executable's own decoder,
read line by line. It matters because the PSP binary contains no compressed
entry to test against and the PS2 archives contain 6,053, so until now the code
and the corpus came from different machines.

**The names below are applied**, from [names.tsv](names.tsv). One is below 70
and carries a `_q`.

| Address | Name | Conf |
| --- | --- | ---: |
| `0x00214080` | `Lzss_Decode` | 92 |
| `0x00213f70` | `Lzss_ReadBits` | 92 |
| `0x00213e58` | `Lzss_InitFromFile` | 88 |
| `0x00213dc0` | `Lzss_AllocBuffers` | 85 |
| `0x00213e08` | `Lzss_FreeBuffers` | 85 |
| `0x00214318` | `Lzss_CloseStream` | 82 |
| `0x00214270` | `Lzss_SkipForward` | 80 |
| `0x00284fc0` | `g_lzss_ring` (pointer to 8192 B) | 92 |
| `0x00284fcc` | `g_lzss_input_buffer` (pointer to 1024 B) | 90 |
| `0x00284fc4` | `g_lzss_encoder_tree` (pointer to 98316 B) | 68 |

## It is a stream, not a one-shot decompressor

`Wad_Open` (see [wad-subsystem.md](wad-subsystem.md)) builds this object for any
entry whose stored and uncompressed sizes differ, and it is a VFS stream: the
vtable at `0x0029c678` exposes an ordinary `read(buffer, count)`, and
`Lzss_Decode` is that read. It decodes **exactly `count` bytes per call**,
suspending mid-match if it has to, and resumes on the next call. Everything the
resume needs lives in the object:

| Offset | Field | Initial value |
| --- | --- | --- |
| `+0x14` | vtable | `0x0029c678` |
| `+0x18` | backing file | the archive |
| `+0x1c` | bytes still owed | `size_out` |
| `+0x20` | `size_out` | from the WAD entry |
| `+0x24` | **ring write cursor** | **1** |
| `+0x28` | flag mask byte | **`0x80`** |
| `+0x2c` | the input byte the mask walks | 0 |
| `+0x30` | a match is in progress | 0 |
| `+0x34` | bytes of the match emitted so far | 0 |
| `+0x38` | **match length + 2** | - |
| `+0x3c` | match position | - |
| `+0x40` | cursor into the input staging buffer | `0x400` |
| `+0x44` | stored bytes not yet read | `size_in` |

## The algorithm, read off `Lzss_Decode`

**One bit.** A fresh input byte is fetched only when the mask is back at `0x80`;
the value of the bit is `current & mask` taken *before* the mask shifts right,
and the mask resets to `0x80` once it shifts to zero:

```c
if (mask == 0x80) { current = staging[in++]; }   /* refill if needed */
bit  = mask;
mask >>= 1;
if (mask == 0) mask = 0x80;
taken = (current & bit) != 0;
```

That is eight bits per byte, **most significant first**.

**`Lzss_ReadBits(n)`** (`0x00213f70`) repeats the same bit fetch `n` times,
walking an output place value from `1 << (n - 1)` rightwards and ORing it in
when the bit is set. So a multi-bit field also arrives most significant first,
and it is read from the same continuous stream as the flags - there is no
byte alignment anywhere.

**The dispatch.**

```c
if (bit == 1) {                       /* literal */
    b = Lzss_ReadBits(8);
    *out++ = b;
    ring[write] = b;
    write = (write + 1) & 0x1fff;
} else {                              /* match */
    position      = Lzss_ReadBits(13);
    length_biased = Lzss_ReadBits(4) + 2;
    progress      = 0;
    in_match      = 1;
}
```

**The copy**, one byte per outer iteration so that a `read` boundary can fall
inside it:

```c
b = ring[(position + progress) & 0x1fff];
*out++ = b;
ring[write] = b;
progress++;
write = (write + 1) & 0x1fff;
in_match = (progress <= length_biased);
```

The loop therefore emits `length_biased + 1` bytes, and `length_biased` is the
encoded length plus two, so a match is **encoded length + 3** bytes: 3 to 18.

Every claim [formats/lzss.md](../../../formats/lzss.md) makes about the format
is visible in the four snippets above, including the two it flags as easy to get
wrong:

- **Positions are absolute.** `ring[(position + progress) & 0x1fff]` indexes the
  ring directly. Nothing subtracts the write cursor.
- **The cursor starts at 1.** `Lzss_InitFromFile` writes `1` to `+0x24`, not `0`
  and not `N - F`.

The `& 0x1fff` on every ring index fixes the window at **8192** bytes, and
`Lzss_AllocBuffers` allocates exactly `0x2000` for it.

## Input staging, and the size the decoder trusts

Input arrives through a **global** 1 KiB buffer, refilled when its cursor
reaches `0x400`:

```c
n = min(0x400, remaining_stored);
file.read(g_lzss_input_buffer + 0x400 - n, n);   /* right-aligned */
in         = 0x400 - n;
remaining_stored -= n;
```

Right-aligning a short final read means the cursor arithmetic needs no special
case at the end. `remaining_stored` starts from the entry's `size_in`, so the
decoder never reads past the compressed blob - **but it does not check for
exhaustion either.** When `size_in` runs out, `n` is 0, the cursor stays at
`0x400`, and the next byte comes from one past the end of the buffer. The
original relies entirely on the output length running out first. Ours returns
`Error::UnexpectedEnd` at that point instead, which is the same place, handled.

## Two things the PSP page did not say

**The ring is not zeroed.** `Lzss_AllocBuffers` obtains it from the ordinary
allocator (`0x00209be0`, a thin wrapper over `0x00209c08`), which does not
clear, and `Lzss_CloseStream` frees it again, so every stream starts on whatever
the heap left behind. Since positions are absolute, a match may legitimately
address a ring slot the current stream has not written - which
[formats/lzss.md](../../../formats/lzss.md) says "read as zero". **On the PS2
they read as garbage.** Both our decoder and the reference in
`crates/formats/tests/lzss_ps2_reference.rs` assume zeros; that assumption is
consistent with 6,053 real streams decoding correctly, which means the shipped
encoder does not emit such a match, not that zero is what the console produces.
Confidence **80** on the negative (the allocator was read, not traced), and it
is recorded because a fuzzed or hand-built stream can tell the two apart where
no shipped archive can.

**A dead encoder workspace is still allocated.** `Lzss_AllocBuffers` also
allocates `0x1800c` bytes into `g_lzss_encoder_tree`, and `Lzss_FreeBuffers`
frees it. Nothing in the binary ever reads or writes it - those two sites are
its only cross-references.

`0x1800c` is 98,316, which is exactly `3 * (8192 + 1) * 4`: three `int` arrays
of `N + 1` entries for a window of `N = 8192`. That is Okumura's `lson`, `rson`
and `dad` - the binary search tree an LZSS *compressor* uses to find matches,
and nothing a decompressor needs. So the retail build allocates and frees the
encoder's workspace on every decompressed entry because the allocation sits in
an init routine the two halves shared, and the encoder itself was compiled out.

Confidence **68** for the name, hence `g_lzss_encoder_tree_q`: the size
arithmetic is exact and the Okumura lineage is not in doubt, but a buffer that
is never touched cannot be confirmed by use, and 98,316 bytes has other
factorisations. It is worth recording anyway, because it **confirms the window
size from a completely independent direction**: the tree arrays are sized for
8192 whether or not anyone reads them.

## The rest of the class

`Lzss_SkipForward` (`0x00214270`) implements a forward seek by decoding into a
16-byte stack buffer and throwing the result away, sixteen bytes at a time. It
rejects anything but `whence == 1` and a non-negative offset, so this stream
cannot seek backwards - consistent with there being no way to rewind an LZSS
state cheaply.

`Lzss_CloseStream` (`0x00214318`) frees both buffers and releases the global
lock the constructor took (`0x00284fc8`, a named lock object). **One
LZSS stream at a time, process-wide**, exactly as on PSP, and for the same
reason: the ring and the staging buffer are globals.

## How this was checked

`crates/formats/tests/lzss_ps2_reference.rs` contains a second decoder
transcribed from `Lzss_Decode` above, written in the shape the PS2 code has - a
resumable state machine with a walking mask byte, a match-in-progress flag and a
length biased by two - rather than in the shape of
[`oag_formats::lzss`](../../../../crates/formats/src/lzss.rs), which came from
the PSP binary. The two are compared over 40,000 pseudorandom streams, a full
ring wrap, and degenerate all-one-byte streams. They agree on every byte.

Eight deliberate mutations of the reference were each caught, so the comparison
has teeth:

| Mutation | Tests that caught it |
| --- | ---: |
| length bias 3 -> 2 | 4 / 4 |
| flag polarity inverted | 4 / 4 |
| field bit order reversed | 4 / 4 |
| position/length fields swapped (4/13 instead of 13/4) | 4 / 4 |
| mask reset one bit early | 4 / 4 |
| write cursor starting at 0 | 2 / 4 |
| positions read as distances back | 2 / 4 |
| window 4096 instead of 8192 | 2 / 4 |

The last three are caught by fewer tests because a run that ends in input
exhaustion compares only *where* the two readers stopped, and ring semantics
cannot affect that. The tests therefore count their complete decodes and assert
a floor, so they cannot quietly decay into checking nothing but the exhaustion
point.

**This is not a runtime check.** No disc image was available in the session that
wrote this page (`data/images/` was empty), so the decoder was not run against a
real archive here, and it has never been run under an emulator on either
platform. What it is: two transcriptions, from two different binaries built by
two different compilers for two different instruction sets, producing identical
output on arbitrary input. See [formats/lzss.md](../../../formats/lzss.md) for
what that does and does not do to the format's score.

## Cross-platform

| Function | PS2 (`SCES_547.48`) | PSP (`BOOT.BIN`) |
| --- | --- | --- |
| `Lzss_Decode` | `0x00214080` | `0x089419d8` |
| `Lzss_ReadBits` | `0x00213f70` | `0x08941c84` |
| `Lzss_InitFromFile` | `0x00213e58` | `0x08941960` |
| `Lzss_InitFromMemory` | not located | `0x0894199c` |
| `Lzss_CloseStream` | `0x00214318` | not located |
| `Lzss_SkipForward` | `0x00214270` | not located |
| `Lzss_AllocBuffers` | `0x00213dc0` | not located; the PSP ring is not heap-allocated |
| `g_lzss_input_buffer` | `0x00284fcc` (1 KiB) | `0x08b66450` (4 KiB, `g_decompress_staging_buffer`) |

The staging buffer is 1 KiB here and 4 KiB on PSP. Nothing about the format
depends on it: it is a read-ahead window on the input, not part of the encoding.

## Not determined

- **The zlib branch.** Bit 31 of `size_out` selects zlib on PSP. No test of it
  was found at the PS2 `Wad_Open` level, and no shipped archive sets the bit, so
  whether this build even has a zlib path is open.
- **What the trailing one or two bytes of every stream contain.** Unchanged from
  [formats/lzss.md](../../../formats/lzss.md); this page adds nothing to it.
- **Whether the original produces anything meaningful for a match that reads an
  unwritten ring slot.** See above. It cannot without a zeroing allocator, and
  it does not have one.

## History

- 2026-07-27: first pass. Algorithm 92 from an unambiguous decompilation whose
  independent transcription agrees byte for byte with a decoder already
  validated on 6,053 real streams. `g_lzss_encoder_tree` at 68, from exact size
  arithmetic on a buffer nothing reads.
