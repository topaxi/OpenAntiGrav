# WAD subsystem

Functions in `PSP_GAME/SYSDIR/BOOT.BIN` (Pulse PSP, UCUS-98712), image base
`0x08804000`, language `Allegrex:LE:32:default`.

The format itself is documented in [formats/wad.md](../../../formats/wad.md);
this page records where it lives in the binary.

**The names below are applied**, from [names.tsv](names.tsv) via
`just apply-names`. Per
[ADR-0005](../../../architecture/adr/0005-ghidra-conventions.md), applying a
rename needs a page carrying its evidence; this page is that evidence for the
subsystem as a whole. Anything under 70 would gain a `_q` suffix; nothing here
does.

## Hashing

| Address | Name | Conf |
| --- | --- | ---: |
| `0x08940d0c` | `Wad_HashName` | 97 |
| `0x08940cb0` | `Wad_BuildCrcTable` | 95 |
| `0x08afbffc` | `g_wad_crc_table` (256 x u32) | 95 |
| `0x08a90d20` | `_ctype_` (newlib) | 93 |
| `0x0897349c` | `strlen` | 95 |

`Wad_BuildCrcTable` materialises the polynomial as
`lui a2,0xedb9 / addiu a2,a2,-0x7ce0` = `0xEDB88320`, then eight
shift-and-conditional-xor rounds per index.

`Wad_HashName` initialises the running value to **0** at `0x08940df0`
(`li s0,0x0`) and complements at the end (`li a0,-1 / xor s0,s0,a0`). That
initial value is what makes it *not* zlib's `crc32`.

Normalisation, in the loop at `0x08940e04`-`0x08940e88`: `\` becomes `/`;
otherwise, if `_ctype_[c] & 0x01` (newlib's `_U`, ASCII `A`-`Z` only) the byte
has `0x20` added. Bytes at or above `0x80` are untouched.

**Verified**, not merely read: 176 filename strings from the binary hash to
values present in the four real archives. `Data\FE\Images\hex_bg.mip` gives
`0x1aa87b99`, which is an entry in `FE.wad`, and its `.mip` extension matches
the [texture format](../../../formats/psp-texture.md) found at that entry.
Independently reimplemented in
[`oag_formats::wad::hash_name`](../../../../crates/formats/src/wad.rs) with the
same results.

One entry in `Data.wad` matched `data/defaults/loading/LoadingPulseOverlay.mip`,
already lowercase with forward slashes, while its siblings matched
`Data\...\*.mip`. That single case confirms both normalisation rules at once.

There is also an escape hatch at `0x08940d5c`-`0x08940dd8`: a name of exactly
`#` plus 8 **uppercase** hex digits is parsed as a literal hash. No such string
exists in the binary, so it is probably tools-only. Confidence that the path
exists: 90. That it is ever used: unknown.

## Lookup and I/O

| Address | Name | Conf |
| --- | --- | ---: |
| `0x089411e8` | `Wad_Open` | 93 |
| `0x08941e70` | `Wad_MountArchive` | 90 |
| `0x08942450` | `Wad_Read` | 90 |
| `0x08941874` | `Wad_ParseResidentDirectory` | 88 |
| `0x08941484` | `Wad_Close` | 85 |
| `0x089410d4` | `Wad_Unmount` | 85 |
| `0x08941648` | `Wad_ReadAsyncBegin` | 78 |
| `0x0894178c` | `Wad_ReadAsyncWait` | 75 |
| `0x08ad3314` | `g_wad_device_vtable` | 90 |
| `0x0893e308` | `Vfs_SplitDevicePath` | 88 |

`Wad_Open` strips a leading separator before hashing, then does a **linear scan
with a rotating start point**: from `last_hit + 1` to the end, then from 0 to
`last_hit`, storing the hit index back at `device+0x40`. No sorting, no buckets.

That is corroborated by the data: entry hashes are **not** in ascending order in
any archive, while offsets strictly are. There are also zero hash collisions
within any archive (27/27, 54/54, 242/242, 1142/1142 distinct), which is what a
first-match-wins scan needs.

An entry with a live handle (`entry+0x14 != 0`) cannot be opened twice; the
second call returns `-1`.

## Mount points

`Wad_MountArchive(this, mount_name, path, flags, crypt_ctx)`, five call sites:

| Caller | Mount | Path | Flags |
| --- | --- | --- | ---: |
| `0x0893dd9c` | `wad:` | `umd:Data.wad` | 0 |
| `0x0894f72c` | `fe:` | `umd:FE.wad` | 1 |
| `0x0888b7c0` | `fedata:` | `umd:FEData.wad` | 9 |
| `0x08813f80` | `bedata:` | `umd:BEData.wad` | 1 |
| `0x0888caac` | (FEData remount) | - | - |

`crypt_ctx` is 0 everywhere.

Flag bit `0x01` makes the whole archive RAM-resident and turns reads into
`memcpy`; `0x08` selects a different allocator; `0x10` means the caller supplies
an already-loaded buffer. Consistent with the sizes: the 315 MiB `Data.wad` is
streamed with flags 0, while the three small archives are resident.

Confidence 88 for the flow, 80 for individual flag meanings.

So a lookup of `wad:Data\FE\Images\hex_bg.mip` splits at the colon
(`Vfs_SplitDevicePath`, `0x0893e308`), finds the `wad:` device, and hashes
`Data\FE\Images\hex_bg.mip` — which is exactly the string form that matched.

## Compression

`Wad_Read` asks **two** questions, in this order:

1. Is `size_in` equal to `size_out & 0x7fffffff`? Then the blob is **stored**,
   whatever bit 31 says, and it is read straight through
   (`Wad_ApplyStreamCrypt` runs over it either way).
2. Only for an entry whose sizes differ does **bit 31 of the uncompressed-size
   field** choose zlib over LZSS.

That order matters and it is the natural thing to get wrong. "Bit 31 clear means
LZSS" on its own makes nonsense of the shipped data: every one of `Data.wad`'s
1,142 entries has bit 31 clear and equal sizes, so that rule would have the game
LZSS-decode the entire 315 MiB archive. Confidence **90**, from the branch
structure at `0x08942450`: the size comparison is the outer test and the flag
test sits inside its else arm.

The consequence is not academic. `size_in == size_out` is **not** evidence that a
blob was stored rather than compressed, since an incompressible blob can encode to
exactly its own length, so neither the game nor
[`oag-wad verify`](../../../tools/README.md) can tell those two cases apart.

The decoders themselves:

| Address | Name | Conf |
| --- | --- | ---: |
| `0x089419d8` | `Lzss_Decode` | 85 |
| `0x08941c84` | `Lzss_ReadBits` | 88 |
| `0x0894199c` / `0x08941960` | `Lzss_InitFromMemory` / `Lzss_InitFromFile` | 85 / 80 |
| `0x08940efc` / `0x08940eb0` | `Wad_DecompressLzssMem` / `Wad_DecompressLzssStream` | 85 |
| `0x08940f48` / `0x08940f70` | `Wad_DecompressZlibMem` / `Wad_DecompressZlibStream` | 85 |
| `0x08956940` | `zlib_uncompress` | 90 |
| `0x089545dc` | `zlib_inflateInit_` | 90 |
| `0x08b66450` | `g_decompress_staging_buffer` (4096 B) | 88 |

**Bit clear: LZSS.** 8192-byte ring buffer, write cursor starting at 1, ring not
pre-filled. MSB-first flag bits: set means an 8-bit literal, clear means a
**13-bit absolute ring position plus a 4-bit length**, with `count = len + 3`.
Okumura-style, but with a 13/4 split rather than the usual 12/4. Input is staged
through a **global** 4 KiB buffer, so the decoder is not reentrant.

**Bit set: zlib 1.2.2**, identified by the `"1.2.2"` version string passed to
`inflateInit_` with `sizeof(z_stream) == 0x38`.

Confidence 82 on the LZSS bit layout. It was read, never executed: no PSP
archive contains a compressed entry, so there was nothing to round-trip against.
**The PS2 archives do use it** (192 of 193 entries in `WADSP.WAD`, all with bit
31 clear), so they are the test corpus when the decoder is implemented. If it
fails, re-check the `+3` length bias and the ring starting at 1 first.

## Structures

```c
typedef struct {              // 16 bytes on disk
    uint32_t name_hash;
    uint32_t offset;          // absolute, always 64-aligned
    uint32_t size_out;        // uncompressed; bit31 = zlib, mask 0x7FFFFFFF
    uint32_t size_in;         // stored
} WadDiskEntry;

typedef struct {              // 24 bytes in RAM
    WadDiskEntry disk;
    uint32_t     pos;         // +0x10 read cursor
    void        *file;        // +0x14 VFS handle, NULL when closed
} WadEntry;
```

The `size_out` / `size_in` order is confirmed twice over: from `Wad_Read`, which
clamps reads to `size_out & 0x7FFFFFFF` and passes `size_in` as the input length
to the decompressor, and independently from the offset chain across all 193
entries of `WADSP.WAD`. Two unrelated lines of evidence agreeing is why this
sits at 95 rather than 85.

`WadDevice` is 0x70 bytes; the fields that matter are `+0x34` version,
`+0x38` entry count, `+0x3c` entry array, `+0x40` rotating scan index, `+0x48`
flags, `+0x4c` resident buffer. Fields `+0x28`, `+0x54`, `+0x58` are only ever
zeroed in the code read so far, confidence 50.

## Not determined

- **No dispatch on a 4-byte blob magic was found.** Asset type appears to be
  resolved by extension or by call site. Confidence 55 on that negative; the
  search was not exhaustive.
- **The cipher behind `Wad_ApplyStreamCrypt` (`0x089508e8` → `0x08986d94`).**
  Dead for retail, since `crypt_ctx` is NULL at every mount. Likely for
  downloadable content, given the nearby `MSC_MSG_DL1` / `MSC_MSG_DL2` strings.
- **Vtable slots `+0x34` and `+0x3c`** (`0x0894151c`, `0x08941524`) are not
  recognised as functions by Ghidra. Seek/tell is inferred from call-site
  argument shape only.
- **Nothing was verified at runtime.** Every score above 90 rests on static
  reading plus agreement with the shipped archive bytes.
