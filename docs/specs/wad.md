# WAD container and LZSS

An archive format that packs a game's files into one blob, looked up by a hash
of the file name. Used by Wipeout Pure (PSP) and Wipeout Pulse (PSP and PS2).
The same container and the same name hash serve all of them. HD, 2048 and Omega
use other containers (PSARC, see [`docs/formats/psarc.md`](../formats/psarc.md)).

All integers are little-endian. Confidence scores follow the
[rubric](../reverse-engineering/confidence-rubric.md) (0-100).

## Layout

```text
+0x00  u32   version        always 1
+0x04  u32   entry_count
+0x08        entry[entry_count], 16 bytes each
             zero padding up to the next 64-byte boundary
             blobs, each starting on a 64-byte boundary
```

Entry, 16 bytes:

| Offset | Type | Field | Meaning |
| ---: | --- | --- | --- |
| +0x00 | u32 | `name_hash` | Hash of the entry name (below). Names are not stored. |
| +0x04 | u32 | `offset` | Byte offset of the blob from the start of the file. |
| +0x08 | u32 | `size_uncompressed` | Size after decompression. Bit 31 is a flag (below). |
| +0x0c | u32 | `size` | Bytes actually stored in the archive. |

Confidence 94. Two independent loaders (PSP and PS2 executables) read this
layout, and the offset chain below holds on all nine shipped archives.

### Blob placement

Blobs are packed in directory order. Each starts at the previous blob's end
rounded up to 64 bytes, and the first starts after the padded directory:

```text
entry[0].offset   == align64(8 + entry_count * 16)
entry[i+1].offset == align64(entry[i].offset + entry[i].size)
```

A reader does not need this to find a blob (it has `offset`), but it is a quick
integrity check and it is what fixes the order of the two size fields.

### Is a blob compressed

Decide in this order:

1. If `size_uncompressed & 0x7fffffff == size`, the blob is stored verbatim.
   Bit 31 is ignored in this case.
2. Otherwise, if bit 31 is set, the blob is a zlib stream.
3. Otherwise it is [LZSS](#lzss).

Do not shortcut this to "bit 31 clear means LZSS": every entry of the PSP
archives has bit 31 clear and equal sizes, and the shortcut would decompress all
of them. An incompressible blob that the encoder left at its own size is also
stored, so equal sizes do not prove the encoder never ran.

The equal-size test comes first in both executables (the PS2 mount and lookup
functions are scored 88 on the evidence page; the bit-31 branch carries no
score of its own). No archive on the Pulse or Pure base discs sets bit 31. The
zlib branch is exercised by Pure's downloadable content packs (187 of 192
entries in one pack), which use the same entry layout.

### Worked example

`FE.wad` from Pulse PSP, first 32 bytes:

```text
00000000  01 00 00 00 1b 00 00 00  4c 01 5e f5 c0 01 00 00
00000010  30 53 00 00 30 53 00 00  b9 1f 1a d6 00 55 00 00
```

Version 1, 27 entries. Entry 0 (bytes 8 to 23) has hash `0xf55e014c`, offset
`0x1c0`, and both sizes `0x5330`. Entry 1 starts with hash `0xd61a1fb9` and
offset `0x5500`. The chain checks out: the directory is `8 + 27*16 = 0x1b8`,
which aligns up to `0x1c0` (entry 0's offset), and entry 0 ends at
`0x1c0 + 0x5330 = 0x54f0`, which aligns up to `0x5500` (entry 1's offset).

## The name hash

```text
crc = 0
for each byte b of the name:
    if b == '\\':          b = '/'
    elif 'A' <= b <= 'Z':  b = b + 0x20
    crc = crc32_step(crc, b)        // reflected CRC-32, polynomial 0xEDB88320
hash = ~crc
```

This is the standard reflected CRC-32 table and step, but the running value
starts at `0`, not `0xFFFFFFFF`, so a stock `crc32()` gives different results.
Bytes at or above `0x80` pass through unchanged. Names are matched
case-insensitively with `\` and `/` equal, so `Data\FE\Images\hex_bg.mip`,
`data/fe/images/hex_bg.mip` and `DATA\FE\IMAGES\HEX_BG.MIP` are one entry.

| Name | Hash |
| --- | --- |
| `Data\FE\Images\hex_bg.mip` | `0x1aa87b99` |
| `Data\Sound\frontend.bnk` | `0x75a91641` |
| `Data\Psys\WO_SHIP_COLL_SPARK_DAMAGE.POB` | `0xeff1f331` |

The original also accepts a name of `#` followed by exactly eight uppercase hex
digits as a literal hash. Both executables' hash functions implement it with the same length
guard; no shipped name uses it, so it is probably a tools feature. Not scored.

Confidence 94 for the hash. Verified on 176 real entries across four archives,
and three separate implementations in two binaries agree.

Names are not recoverable in bulk: an archive can be searched by name, not
listed. A listing needs a list of candidate names to hash. Most paths are built
at runtime from format strings such as `%s\%strack%s.vex`.

Lookup scans the entry array and the first match wins. The PSP build starts the
scan at the last hit and wraps; the PS2 starts from the beginning. Both find the
same entry, so a reader can scan from the start.

## LZSS

Used for PS2 WAD entries selected by the rule above (5,861 of 7,200 entries in
`WADS2.WAD`, 192 of 193 in `WADSP.WAD`). No PSP archive compresses anything.

The compressed data is one MSB-first bit stream. Nothing is byte aligned. The
decoder runs until it has produced `size_uncompressed` bytes; there is no end
marker.

```text
read 1 bit
  1 -> read 8 bits: a literal byte
  0 -> read 13 bits: position   (absolute index into the ring)
       read  4 bits: length     (copy length + 3 bytes, so 3 to 18)
```

| Parameter | Value |
| --- | --- |
| Ring size | 8192 bytes |
| Initial ring contents | zero |
| Initial write cursor | 1 (not 0, not 8174) |
| Position | Absolute ring index, not a distance back |
| Bit order | MSB first, one continuous stream |

Every output byte, literal or copied, is also written to `ring[cursor]`, and the
cursor then advances modulo 8192. A copy reads `ring[position]`, advancing the
read index modulo 8192 as it goes, so it can overlap bytes it has just written.

```rust
fn lzss_decode(input: &[u8], out_len: usize) -> Vec<u8> {
    let mut bits = MsbBitReader::new(input);
    let (mut ring, mut cursor, mut out) = ([0u8; 8192], 1usize, Vec::new());
    while out.len() < out_len {
        if bits.take(1) == 1 {
            let b = bits.take(8) as u8;
            out.push(b);
            ring[cursor] = b;
            cursor = (cursor + 1) & 8191;
        } else {
            let mut from = bits.take(13) as usize;
            let n = bits.take(4) as usize + 3;
            for _ in 0..n.min(out_len - out.len()) {
                let b = ring[from];
                out.push(b);
                ring[cursor] = b;
                from = (from + 1) & 8191;
                cursor = (cursor + 1) & 8191;
            }
        }
    }
    out
}
```

Confidence 94. Checks that give this teeth:

- All 6,053 compressed entries on the PS2 disc decode to valid content (the first
  one tested is well-formed XML only under cursor 1 and absolute positions).
- Every stream ends one or two bytes short of its stored length, never exactly at
  the end and never further back. The decoder stops on output length and does not
  look at the tail, so this is an independent constraint on the bit layout. One
  or two slack bytes is what a 16-bit output buffer flushing at the end would
  leave. The trailing bytes themselves have not been inspected.
- The PS2 executable allocates an unused 98,316-byte buffer, exactly the size of
  the encoder's search tree for an 8192-byte window.

Known behaviour outside the shipped data: the PS2 does not zero its ring between
streams, so a hand-built stream that reads a slot it has not yet written reads
stale memory on the console and zero here. Shipped streams never do this.
Confidence 80.

## Blob contents

Blobs carry no uniform tag. Identify by content:

| Content | How to recognise |
| --- | --- |
| PSP texture (`.mip`) | Size matches the header arithmetic, see [textures](textures.md) |
| Font (`.fnt`) | Starts `01 'F' 'N' 'T'` |
| Model (`.vex`) | First word is a version (`6` on Pulse), see [vex](vex.md) |
| Sound bank (`.bnk`) | First word `3` |
| Particle system (`.pob`) | Starts `SYSP`, see [pob](pob.md) |

On PS2 the first bytes of a compressed blob are the head of the compressed
stream and mean nothing until decoded. A PS2 `.vex` declares no textures of its
own; its textures are separate entries (see [textures](textures.md)).

## Shipped archives

| Release | Archive | Entries | Compressed entries |
| --- | --- | ---: | ---: |
| Pulse PSP | `Data.wad` | 1,142 | 0 |
| Pulse PSP | `FEData.wad` | 242 | 0 |
| Pulse PSP | `BEData.wad` | 54 | 0 |
| Pulse PSP | `FE.wad` | 27 | 0 |
| Pulse PS2 | `WADS2.WAD` | 7,200 | 5,861 |
| Pulse PS2 | `WADSP.WAD` | 193 | 192 |
| Pure PSP | `Data.wad` | 832 | 0 |
| Pure PSP | `FEData.wad` | 240 | 0 |
| Pure PSP | `FE.wad` | 157 | 0 |

A PS2 texture set is a nested WAD: the same 8-byte header and 16-byte entries
inside one entry of the outer archive.

## Known unknowns

- Whether any shipped archive stores a zlib entry at the base-disc level: none
  seen.
- The contents of the one or two bytes after each LZSS stream.
- The `version` word is 1 everywhere; whether the game rejects other values is
  not tested.

## Evidence

- Evidence pages: [`docs/formats/wad.md`](../formats/wad.md),
  [`docs/formats/lzss.md`](../formats/lzss.md).
- Reader: [`crates/formats/src/wad.rs`](../../crates/formats/src/wad.rs),
  [`crates/formats/src/lzss.rs`](../../crates/formats/src/lzss.rs).
- Tool: `oag-wad list|tags|extract|verify|hash`.
