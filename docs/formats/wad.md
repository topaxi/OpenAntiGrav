# WAD container

**Status: understood.** The container is decoded, implemented in
[`oag-formats::wad`](../../crates/formats/src/wad.rs), and validated against all
nine archives across three releases.

Entry *names* are still unknown: the directory stores a 32-bit value where a
name would be, and the hash function has not yet been recovered from the game
binary.

## Layout

All values little-endian, on both platforms.

```
+0x00  u32  version          always 1
+0x04  u32  entry_count
+0x08       entry[entry_count]
            padding to a 64-byte boundary
            blob data, each blob starting on a 64-byte boundary
```

### Entry, 16 bytes

| Offset | Type | Field | Notes |
| ---: | --- | --- | --- |
| +0x00 | `u32` | `name_hash` | Presumed a hash of the name. **Algorithm unknown.** |
| +0x04 | `u32` | `offset` | Byte offset of the blob from the start of the file |
| +0x08 | `u32` | `size_uncompressed` | Size after decompression |
| +0x0c | `u32` | `size` | Bytes actually stored |

## Evidence

### The offset chain pins the layout

Blobs are packed back to back, each starting at the previous blob's end rounded
up to 64 bytes, with the first starting after the padded directory:

```
entry[i+1].offset == align64(entry[i].offset + entry[i].size)
entry[0].offset   == align64(8 + entry_count * 16)
```

No other assignment of the four words produces a consistent chain, which is what
makes this a determination rather than a guess. `oag-formats` checks the
property and reports breaks; it does not require it, since a legitimate archive
with gaps would be a finding rather than a corruption.

### Which size field is which

**This is easy to get backwards, and I did at first.** Every PSP archive stores
everything uncompressed, so the two size fields are always equal there and their
order cannot be observed.

The PS2 archives are compressed, and the offset chain resolves it. Testing both
orderings against all 193 entries of `WADSP.WAD`:

| Reading | Entries whose offset chains correctly |
| --- | ---: |
| `+0x08` = stored, `+0x0c` = uncompressed | 2 / 193 |
| `+0x08` = uncompressed, `+0x0c` = stored | **193 / 193** |

Confidence: **95**. Verified across two independent archives on the compressed
platform, and consistent with all seven uncompressed archives.

### Worked example

`PSP_GAME/USRDIR/FE.wad`, Pulse PSP:

```
00000000  01 00 00 00 1b 00 00 00  4c 01 5e f5 c0 01 00 00  |........L.^.....|
00000010  30 53 00 00 30 53 00 00  b9 1f 1a d6 00 55 00 00  |0S..0S.......U..|
```

Version 1, 27 entries. Entry 0: hash `0xf55e014c`, offset `0x1c0`, both sizes
`0x5330`.

```
directory  8 + 27*16 = 440 = 0x1b8  -> align64 -> 0x1c0 = entry 0's offset
entry 0    0x1c0 + 0x5330 = 0x64f0  -> align64 -> 0x5500 = entry 1's offset
entry 1    0x5500 + 0x93f0 = 0xe8f0 -> align64 -> 0xe900 = entry 2's offset
```

## Validated archives

Every archive in every in-scope release parses, with a consistent offset chain:

| Release | Archive | Entries | Stored | Compressed entries |
| --- | --- | ---: | ---: | ---: |
| Pulse PSP | `Data.wad` | 1,142 | 300 MiB | 0 |
| Pulse PSP | `FEData.wad` | 242 | 6.7 MiB | 0 |
| Pulse PSP | `BEData.wad` | 54 | 934 KiB | 0 |
| Pulse PSP | `FE.wad` | 27 | 797 KiB | 0 |
| Pulse PS2 | `WADS2.WAD` | 7,200 | 360 MiB | **5,861** |
| Pulse PS2 | `WADSP.WAD` | 193 | 1.4 MiB | **192** |
| Pure PSP | `Data.wad` | 832 | 223 MiB | 0 |
| Pure PSP | `FEData.wad` | 240 | 6.3 MiB | 0 |
| Pure PSP | `FE.wad` | 157 | 2.5 MiB | 0 |

**No PSP archive compresses anything; the PS2 archives compress most entries.**
That is consistent with the platforms' constraints: the PSP streams from a slow
UMD into little RAM and benefits from being able to read a blob straight into
place, while the PS2 reads from a faster DVD with more RAM to spare.

The same container serves Pure PSP, Pulse PSP and Pulse PS2 unchanged, which is
the first hard evidence that one asset pipeline can cover the lineage.

## Blob contents

Blobs are heterogeneous and are **not** uniformly tagged. An earlier reading of
`\x01FNT` as a general "version byte plus three-character type code" was too
hasty: that pattern holds for fonts and little else.

For `FE.wad` (Pulse PSP), 13 of 27 blobs are
[textures](psp-texture.md), identified by the leading `u16 width, u16 height`.
Five are `\x01FNT` fonts. The rest begin with small integers such as
`03000000` and `06000000`, and are undecoded.

**PS2 blob leading bytes are meaningless** until decompressed, since what you
see is the head of a compressed stream. `oag-wad tags` on a PS2 archive
correctly reports near-random tags; that is the compression, not a format.

## Open questions

### How are entries named?

The 32-bit first field is presumed a hash. Nothing in the archive points at a
string table, and blobs begin immediately after the directory, so names are not
stored.

The lookup function in `BOOT.BIN` contains the algorithm. Finding it is the
highest-value single task in M1: without it, nothing can be identified by name.

Candidates to test once any names are known: CRC-32, FNV-1a, a `h*31+c`
polynomial, or a Sony-internal hash.

### What compression do the PS2 archives use?

5,861 of 7,200 entries in `WADS2.WAD` are compressed, at roughly 2.7x on the
entries sampled. The algorithm is unidentified. Likely candidates for the era
are LZSS, a Sony-internal LZ variant, or zlib. The decompressor lives in
`SCES_547.48`.

### What are the undecoded blob types?

`03000000` and `06000000` in `FE.wad` are probably small structured records,
given their leading values look like counts. Not investigated.

## Usage

```sh
# Straight from a disc image, no extraction needed
oag-wad list data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/FE.wad
oag-wad tags data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/FE.wad
oag-wad extract data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/FE.wad -o /tmp/fe
```

Extracted blobs are named `<index>_<hash>.<tag>`, because the real names are not
available.
