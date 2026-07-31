# WAD container

**Status: understood.** The container is decoded, implemented in
[`oag-formats::wad`](../../crates/formats/src/wad.rs), and validated against all
nine archives across three releases.

Entry *names* are not stored. The directory holds only a CRC-32 of each name,
so an entry can be **found** by name but an archive cannot be **listed** with
names. See [the hash](#the-name-hash) below.

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
| +0x00 | `u32` | `name_hash` | CRC-32 of the name; see [below](#the-name-hash) |
| +0x04 | `u32` | `offset` | Byte offset of the blob from the start of the file |
| +0x08 | `u32` | `size_uncompressed` | Size after decompression; **bit 31 selects zlib over LZSS**, but only for an entry that is compressed at all |
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

Confidence: **94**. Verified across two independent archives on the compressed
platform, and consistent with all seven uncompressed archives: an exact
arithmetic invariant (the offset chain) resolved between the two orderings on
193/193 real entries. Per the
[rubric](../reverse-engineering/confidence-rubric.md), that is data agreement,
not a runtime trace, so it caps at 94; the earlier 95 predated the rubric
saying so.

### And now a second binary, which reads the layout back in one shot

The layout above was derived from the PSP `BOOT.BIN` and from the shipped
archives. The **PS2 `SCES_547.48` implements it independently**, and its loader
is unusually direct evidence because it does no per-entry parsing at all:
`Wad_MountArchive` (`0x00213058`) reads **8 bytes** of header into a stack
buffer, takes the *second* word as the entry count, resizes a vector of
**16-byte** elements to that count, and reads `count * 16` bytes straight into
it. A wrong header size or a wrong entry stride cannot survive that; there is no
loop to absorb the error.

`Wad_Open` (`0x002132c0`) then pins the four fields by use rather than by
position: `entry[0]` is compared against `Wad_HashName`'s result, `entry[1]` is
passed to the backing file's seek, and the `entry[3] == entry[2]` test selects a
passthrough stream over a decompressing one - so `entry[2]`/`entry[3]` are the
uncompressed and stored sizes, in that order. That is
`{name_hash, offset, size_uncompressed, size}`, exactly the table above.

It also corroborates the **compression rule's outer test**: the size-equality
question comes first, and no bit-31 flag test appears at this level at all. See
[the PS2 WAD subsystem page](../ghidra/functions/ps2-pulse/wad-subsystem.md#mount-lookup-and-the-archive-layout)
for the full reading; confidence **88** on the lookup and mount functions there.

**One genuine divergence, and it is not in the layout.** The PSP's `Wad_Open`
scans the entry array from a *rotating* start point (`last_hit + 1`, wrapping);
the PS2's scans from the beginning every time, with no stored cursor. First match
wins in both, so the same entry is found either way - only the search cost
differs. It does mean a reimplementation **cannot cite the PS2 build as evidence
for the rotating cursor**. Confidence **85** on the difference.

Confidence for the layout: **94**, unchanged, but resting on two legs instead of
one. The number does not move, and that is deliberate - the
[rubric](../reverse-engineering/confidence-rubric.md)'s top band needs a runtime
trace **and** a second binary; this now has the second binary and still has no
trace. Nothing in this file has ever been executed under an emulator on either
platform, and a second reader cannot supply that leg. What changed is where the
remaining doubt lives: not in the field layout, which two independent loaders now
agree on, but in the surrounding assumptions. This follows
[LZSS](lzss.md#an-independent-decoder-from-the-other-binary)'s precedent exactly,
for the same reason.

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

**Validated against three discs**: `pulse-psp-usa`, `pulse-ps2-eu` and
`pure-psp-usa`. `oag-wad verify` decodes every entry of all nine archives to its
declared size, Pure's 1,229 included. The
[Pure probe](pure-status.md) found nothing in the container that needed
changing, and the [name hash](#the-name-hash) below resolves Pulse-shaped paths
against Pure's directories with no salt or seed change.

## The name hash

CRC-32 with the standard reflected polynomial `0xEDB88320`, but **initialised to
0 rather than `0xFFFFFFFF`**, so it is *not* zlib's `crc32`. The name is
normalised first: `\` becomes `/`, and ASCII `A`-`Z` fold to lowercase. Bytes at
or above `0x80` pass through untouched, because the fold is driven by newlib's
`_ctype_` table.

```rust
let mut crc: u32 = 0;                  // not 0xFFFFFFFF
for byte in name.bytes() {
    let c = match byte {
        b'\\' => b'/',
        b'A'..=b'Z' => byte + 0x20,
        other => other,
    };
    crc = crc32_step(crc, c);          // reflected, poly 0xEDB88320
}
!crc
```

Recovered from `Wad_HashName` at `0x08940d0c`; see
[the WAD subsystem page](../ghidra/functions/psp-pulse/wad-subsystem.md).
Implemented as [`oag_formats::wad::hash_name`](../../crates/formats/src/wad.rs).

Verified against 176 real entries across four archives. Worked examples:

| Name | Hash |
| --- | --- |
| `Data\FE\Images\hex_bg.mip` | `0x1aa87b99` |
| `Data\Sound\frontend.bnk` | `0x75a91641` |
| `Data\Psys\WO_SHIP_COLL_SPARK_DAMAGE.POB` | `0xeff1f331` |

Normalisation means `Data\FE\Images\hex_bg.mip`, `data/fe/images/hex_bg.mip`
and `DATA\FE\IMAGES\HEX_BG.MIP` are the same entry. One archive entry matched
a string already written in lowercase with forward slashes while its siblings
matched backslashed mixed-case paths, which confirms both rules at once.

There is also an escape hatch: a name of exactly `#` plus eight **uppercase** hex
digits is taken as a literal hash. No such string exists in the binary, so it is
probably tools-only.

**Three independent implementations agree, across two binaries.** The PS2
`SCES_547.48` carries *two* copies of this hash in separate translation units,
each with its own lazily-built CRC table: `Wad_HashName` (`0x0020c638`), the one
`Wad_Open` actually calls, and `Resource_HashName` (`0x00203b78`), reached only
from a name-keyed resource cache. Both reproduce every rule above - the running
value initialised to `0` and complemented at the end, `\` hashed as `/`,
uppercase folded to lowercase, and the `#` plus eight uppercase hex digits
literal-hash escape with the same `length == 9` guard. They differ only in
mechanics: `Resource_HashName` folds through newlib's `_ctype_` table like the
PSP does, while `Wad_HashName` uses a precomputed 256-byte lowercase table
(`g_tolower_table`, `0x002c6d98`) and tests for the separator *after* folding,
which is equivalent because `\` is unaffected by case folding. Details and the
address correspondence are on
[the PS2 WAD subsystem page](../ghidra/functions/ps2-pulse/wad-subsystem.md#hashing).

Do not confuse it with `crc32` (`0x002068c8`), an ordinary CRC-32 with initial
value `0xFFFFFFFF`, no normalisation and a statically initialised table. The zero
initial value is exactly what separates them.

Confidence: **94**. Recovered from decompilation (`Wad_HashName`), checked against
176 real entries across four archives including the mixed-case normalisation case
above, and now corroborated by two further implementations in a second binary.
Per the [rubric](../reverse-engineering/confidence-rubric.md), data agreement plus
a second binary is still short of the top band's other leg - a runtime trace - so
it caps at 94; the earlier 97 predated the rubric saying so.

```sh
oag-wad hash 'Data\FE\Images\hex_bg.mip'
oag-wad list <archive> --names <candidates.txt>
```

## Blob contents

Blobs are heterogeneous and are **not** uniformly tagged. An earlier reading of
`\x01FNT` as a general "version byte plus three-character type code" was too
hasty: that pattern holds for fonts and little else.

For `FE.wad` (Pulse PSP), 13 of 27 blobs are
[textures](psp-texture.md) (`.mip`), identified by the leading
`u16 width, u16 height`. Five are `\x01FNT` fonts. `06000000` is a
[`.vex` model](vex.md) version word, and `03000000` a `.bnk` sound bank.

**PS2 blob leading bytes are meaningless** until decompressed, since what you
see is the head of a compressed stream. `oag-wad tags` on a PS2 archive
correctly reports near-random tags; that is the compression, not a format.

## Open questions

### ~~How are entries named?~~ Answered

See [the name hash](#the-name-hash). Names remain unrecoverable *in bulk*,
because the archive stores no strings: a listing with names requires hashing
candidate names and matching. Most names are assembled at runtime from format
strings such as `%s\\%strack%s.vex`, so the templates matter more than the
literals.

### ~~What compression do the PS2 archives use?~~ Answered

[LZSS](lzss.md), selected when bit 31 of the uncompressed-size field is clear
**and** the two size fields differ. Equal sizes mean the blob is stored, whatever
bit 31 says: see
[the compression rule](../ghidra/functions/psp-pulse/wad-subsystem.md#compression).
Implemented and verified against all 6,053 compressed entries across both PS2
archives. The game also supports zlib behind that bit, but no shipped archive
sets it.

### What are the remaining blob types?

`.bnk` sound banks (`03000000`) are [partially decoded](psp-audio.md), `\x01FNT`
fonts are [decoded](fnt.md), `SYSP` particle systems (`.pob`) are
[partially decoded](pob.md) - the container and name, not the payload - and
`.dat` files paired with ships remain undecoded.

## Usage

```sh
# Straight from a disc image, no extraction needed
oag-wad list data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/FE.wad
oag-wad tags data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/FE.wad
oag-wad extract data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/FE.wad -o /tmp/fe
```

Extracted blobs are named `<index>_<hash>.<tag>`, because the real names are not
available.
