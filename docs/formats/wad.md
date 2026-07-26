# WAD container

**Status: partial.** The directory structure is decoded and validated. Entry
names are not: the first field is almost certainly a hash, and the hash function
is unknown.

Appears in Wipeout Pure (PSP), Wipeout Pulse (PSP) and Wipeout Pulse (PS2), in
the same shape across all three.

## Layout

All values are little-endian, on both platforms.

```
+0x00  u32  version          always 1 in every file examined
+0x04  u32  entry_count
+0x08       entry[entry_count]
            padded to a 64-byte boundary
            blob data, each blob starting on a 64-byte boundary
```

### Entry, 16 bytes

| Offset | Type | Field | Notes |
| ---: | --- | --- | --- |
| +0x00 | `u32` | `name_hash` | Presumed a hash of the entry name. **Unconfirmed.** |
| +0x04 | `u32` | `offset` | Byte offset of the blob from the start of the file |
| +0x08 | `u32` | `size` | Stored size |
| +0x0c | `u32` | `size_uncompressed` | Equal to `size` in every entry examined |

`size` and `size_uncompressed` being consistently equal suggests a
compressed/uncompressed pair where nothing in these files happens to be
compressed, or a size/capacity pair. Two fields that always agree is a weak
observation either way; the field names above are provisional.

## Evidence

From `PSP_GAME/USRDIR/FE.wad`, Pulse PSP:

```
00000000  01 00 00 00 1b 00 00 00  4c 01 5e f5 c0 01 00 00  |........L.^.....|
00000010  30 53 00 00 30 53 00 00  b9 1f 1a d6 00 55 00 00  |0S..0S.......U..|
00000020  f0 93 00 00 f0 93 00 00  d7 cb 0e a5 00 e9 00 00  |................|
00000030  f0 93 00 00 f0 93 00 00  d4 03 04 17 00 7d 01 00  |.............}..|
```

Header: version `1`, `entry_count` `0x1b` = 27.

Entries, reading 16 bytes from `+0x08`:

| # | `name_hash` | `offset` | `size` | `size_uncompressed` |
| ---: | --- | ---: | ---: | ---: |
| 0 | `0xf55e014c` | 0x1c0 | 0x5330 | 0x5330 |
| 1 | `0xd61a1fb9` | 0x5500 | 0x93f0 | 0x93f0 |
| 2 | `0xa50ecbd7` | 0xe900 | 0x93f0 | 0x93f0 |
| 3 | `0x170403d4` | 0x17d00 | 0x10cb0 | 0x10cb0 |

**The offsets are self-checking**, which is what settles the field ordering:

```
entry 0 ends at 0x1c0  + 0x5330 = 0x64f0   -> round up to 64 -> 0x5500 = entry 1's offset
entry 1 ends at 0x5500 + 0x93f0 = 0xe8f0   -> round up to 64 -> 0xe900 = entry 2's offset
entry 2 ends at 0xe900 + 0x93f0 = 0x17cf0  -> round up to 64 -> 0x17d00 = entry 3's offset
```

Any other assignment of the four fields fails this chain immediately.

The table itself is consistent with the same rule:

```
header 8 + 27 entries x 16 = 440 = 0x1b8  -> round up to 64 -> 0x1c0 = entry 0's offset
```

And the bytes at `0x1b8..0x1c0` are indeed zero padding, with the first blob
beginning at `0x1c0`:

```
000001b0  50 25 00 00 50 25 00 00  00 00 00 00 00 00 00 00  |P%..P%..........|
000001c0  01 46 4e 54 c6 00 00 00  30 00 00 00 bc 01 00 00  |.FNT....0.......|
```

`\x01FNT` is a sub-format tag: a version byte followed by a three-character type
code. A font, given the name.

Confidence: **92**. The offset chain validates across the whole table, the
padding is where the rule predicts, and the header size is consistent. Not 95+
because it has not been validated against a second file programmatically, nor
against the PS2 files.

## Cross-title

| Title | File | Header | Entries |
| --- | --- | --- | ---: |
| Pulse PSP | `Data.wad` | `01 00 00 00  76 04 00 00` | 1,142 |
| Pulse PSP | `FEData.wad` | `01 00 00 00  f2 00 00 00` | 242 |
| Pulse PSP | `FE.wad` | `01 00 00 00  1b 00 00 00` | 27 |
| Pulse PSP | `BEData.wad` | `01 00 00 00  36 00 00 00` | 54 |
| Pulse PS2 | `WADS2.WAD` | `01 00 00 00  20 1c 00 00` | 7,200 |
| Pulse PS2 | `WADSP.WAD` | `01 00 00 00  c1 00 00 00` | 193 |
| Pure PSP | `Data.wad` | `01 00 00 00  40 03 00 00` | 832 |
| Pure PSP | `FEData.wad` | `01 00 00 00  f0 00 00 00` | 240 |
| Pure PSP | `FE.wad` | `01 00 00 00  9d 00 00 00` | 157 |

Entry counts scale sensibly with file size in every case, which is a useful
sanity check on the `entry_count` reading.

**Not this format**, despite the extension: `PS2MUSIC.WAD` (header
`10 00 00 00`) and `PRERACE.WAD` (header `20 00 00 00`). See the
[PS2 disc layout](../ps2/pulse-disc-layout.md).

## Open questions

### How are entries named?

The 32-bit first field is presumed a hash. Nothing in the header points at a
string table, and the blobs start immediately after the directory, so names are
not stored in the archive.

The lookup function in `BOOT.BIN` will contain the hash algorithm. Finding it is
the fastest route, and it is a good first target for M2 because it is small,
self-contained, and unlocks all of M1.

Candidates to test against, once some names are known: CRC-32, FNV-1a, a
Sony-internal hash, or a simple polynomial rolling hash.

**Until names are recovered, extracted entries can only be identified by their
content**, using the sub-format tag at the start of each blob.

### Is anything compressed?

`size` equals `size_uncompressed` in every entry examined so far, which is a
small sample from one file. Checking whether any entry anywhere has them differ
would settle what those fields mean.

### What are the sub-format tags?

`\x01FNT` is the only one seen. Enumerating the first four bytes of every blob
across all the archives would produce the inventory, and is cheap to do once the
directory parser exists.

## Implementation

Not yet implemented. When it is, it goes in `oag-formats` with tests built from
hand-authored archives rather than game data. See
[ADR-0006](../architecture/adr/0006-no-copyrighted-content.md).

Reproduce the evidence above with:

```sh
just unpack hexdump data/images/pulse-psp-usa.chd PSP_GAME/USRDIR/FE.wad --length 320
just unpack hexdump data/images/pulse-psp-usa.chd PSP_GAME/USRDIR/FE.wad --offset 400 --length 128
```
