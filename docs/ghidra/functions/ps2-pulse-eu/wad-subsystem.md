# WAD subsystem (PS2)

Functions in `SCES_547.48` (Wipeout Pulse, PS2, SCES-54748), image base
`0x00100000`, language `r5900:LE:32:default`.

The format itself is documented in [formats/wad.md](../../../formats/wad.md);
the PSP side of the same subsystem is
[psp-pulse-usa/wad-subsystem.md](../psp-pulse-usa/wad-subsystem.md). This page records
where it lives in the PS2 executable and, more usefully, **which of the PSP
page's claims a second binary agrees with**.

Note that `.rodata` and `.data` are visible at three mirrored address ranges
(`0x002xxxxx`, `0x202xxxxx`, `0x303xxxxx` - cached, uncached, uncached and
accelerated). Every address on this page is the canonical `0x002xxxxx` form.

**The names below are applied**, from [names.tsv](names.tsv). Per
[ADR-0005](../../../architecture/adr/0005-ghidra-conventions.md), applying a
rename needs a page carrying its evidence; this page is that evidence for the
subsystem as a whole. Nothing here scores below 70, so no `_q` suffix is used.

## Hashing

| Address | Name | Conf |
| --- | --- | ---: |
| `0x0020c638` | `Wad_HashName` | 95 |
| `0x0020c480` | `Wad_BuildCrcTable` | 93 |
| `0x00303618` | `g_wad_crc_table` (256 x u32, `.bss`) | 93 |
| `0x0020c4e0` | `Wad_ParseLiteralHash` | 90 |
| `0x002c6d98` | `g_tolower_table` (256 B) | 90 |
| `0x00203b78` | `Resource_HashName` | 92 |
| `0x00203b18` | `Resource_BuildCrcTable` | 90 |
| `0x00303160` | `g_resource_crc_table` (256 x u32, `.bss`) | 90 |

**There are two independent implementations of this hash in the binary**, in
two translation units, each with its own lazily-built CRC table and its own
initialised-flag. They compute the same function by different means, which is
why both are named rather than one being dismissed as a duplicate. Only one of
them is reached from the archive, and the names say which:

- `Wad_HashName` (`0x0020c638`) is the copy the WAD **device** uses. `Wad_Open`
  (`0x002132c0`) calls it directly and compares its result against the entry
  array, so it is the structural counterpart of PSP's `Wad_HashName`.
- `Resource_HashName` (`0x00203b78`) is reached only from the name-keyed
  resource registry at `0x001fd618` / `0x001fd6b8` / `0x001fd8c0`, which is a
  cache of loaded files and **not** the WAD directory. Same arithmetic,
  different consumer, hence the different prefix.

Both reproduce the documented PSP behaviour:

- the running value initialises to **0** and is complemented at the end;
- `\` (`0x5c`) is hashed as `/` (`0x2f`);
- uppercase folds to lowercase;
- the `#` plus **eight uppercase hex digits** literal-hash escape hatch exists,
  with the same `length == 9` guard.

They differ only in mechanics. `Resource_HashName` inlines the `#`-hash path
and case-folds through `_ctype_[c] & 0x01` (newlib's `_U`, ASCII `A`-`Z` only,
adding `0x20`), testing for the separator first. `Wad_HashName` factors the
`#`-hash path out into `Wad_ParseLiteralHash` (`0x0020c4e0`), case-folds through
a **precomputed 256-byte lowercase table** (`g_tolower_table`, `0x002c6d98`,
verified by reading it: `0x41`..`0x5a` map to `0x61`..`0x7a`, everything else is
identity), tests for the separator *after* folding, and takes a
`{ptr, length}` string reference rather than a NUL-terminated string. Folding
before or after the separator test is equivalent, because `\` is unaffected by
case folding.

`Wad_BuildCrcTable` (`0x0020c480`) and `Resource_BuildCrcTable` (`0x00203b18`)
are byte-identical: each materialises `0xEDB88320` as an immediate
(`lui v1,0xedb8`) and runs eight shift-and-conditional-xor rounds per index into
its own table, guarded by a one-shot flag in the word above it.

Confidence **95** / **92**: two independent implementations in one binary
agreeing with a third in another binary, all reproducing the same documented
arithmetic. Not higher because nothing here was run.

A **third** copy of the same table-builder exists at `0x001fed88` (table at
`0x003025e0`), feeding a family at `0x001fede8` / `0x001fee90` / `0x001fef20`.
That family folds case the **other** way (lowercase to uppercase, via
`_ctype_[c] & 0x02`) and is therefore not the WAD hash; it is left unnamed.

### Not the same as `crc32`

`crc32` (`0x002068c8`) is a separate, ordinary CRC-32: initial value
`0xFFFFFFFF`, no normalisation, and a **statically initialised** table in
`.data` at `0x002849e8` (`g_crc32_table`, found by searching for the standard
table's first entries `00000000 77073096 EE0E612C 990951BA`). It has exactly one
caller and is not part of the WAD path. Recorded so nobody mistakes it for the
name hash - the zero initial value is precisely what separates them.

| Address | Name | Conf |
| --- | --- | ---: |
| `0x002068c8` | `crc32` | 85 |
| `0x002849e8` | `g_crc32_table` (256 x u32, `.data`) | 90 |

## Mount, lookup and the archive layout

| Address | Name | Conf |
| --- | --- | ---: |
| `0x00213058` | `Wad_MountArchive` | 88 |
| `0x002132c0` | `Wad_Open` | 88 |
| `0x002134c8` | `Wad_EntryExists` | 85 |
| `0x002131e8` | `Wad_Unmount` | 82 |
| `0x0029c620` | `g_wad_device_vtable` | 80 |

`Wad_MountArchive(this, path, device_name)` is the WAD device's constructor. It
is registered into the VFS device chain from `0x001fd028` with the arguments
`"WADS2.WAD"` (`0x002c0370`) and `"PS2Wad"` (`0x002c0380`). It then:

1. opens `path` through the VFS;
2. reads **8 bytes** of header into a stack buffer, of which the second word is
   the entry count;
3. resizes a vector of **16-byte** elements (`this+0x3c` begin, `this+0x40` end,
   pointer arithmetic in strides of `0x10` throughout) to that count;
4. reads `count * 16` bytes straight into it.

**This is a second-binary confirmation of the WAD container layout**: an 8-byte
header of two `u32`s followed by a flat array of 16-byte directory entries, read
in one shot with no per-entry parsing. It is the same shape
[formats/wad.md](../../../formats/wad.md) derives from the PSP binary and from
the shipped archives.

`Wad_Open(this, name)` hashes the name with `Wad_HashName` and then walks
the entry array comparing `entry[0]` against the hash. On a hit it seeks the
backing file to `entry[1]` and builds a stream object:

```c
if (entry[3] == entry[2])                 // size_in == size_out
    stream = Stored(file, entry[2]);      // 0x24 bytes, plain passthrough
else
    stream = Decompressing(file, entry[2], entry[3]);   // 0x48 bytes
```

Two things follow, and the first is the important one.

**The size-equality test is the outer question here too.** The PSP page states
that `Wad_Read` asks "is `size_in` equal to `size_out & 0x7fffffff`?" *first*,
and only consults bit 31 for entries whose sizes differ - and warns that getting
that order wrong makes nonsense of the shipped data. The PS2 build makes the
same test the sole branch at open time, structurally, with no flag test in
sight at this level. That is an independent binary agreeing with the reading,
which is the strongest corroboration available short of running it. Confidence
**88**.

**The entry field order is confirmed.** `entry[0]` is compared against a name
hash, `entry[1]` is passed to the file seek, and `entry[2]`/`entry[3]` are the
uncompressed and stored sizes in that order (`entry[2]` is also what
`Wad_EntryExists` tests for non-zero). That is `{name_hash, offset, size_out,
size_in}`, matching `WadDiskEntry`.

### Where the PS2 differs: the scan is not rotating

`Wad_Open` starts its linear scan **at the beginning of the entry array every
time**. There is no stored last-hit index and no wrap-around; the loop is a
plain `for (e = begin; e != end; ++e)`.

The PSP page describes a *rotating* start point - "from `last_hit + 1` to the
end, then from 0 to `last_hit`, storing the hit index back at `device+0x40`".
`Wad_MountArchive` does zero a field at `this+0x48`, but nothing in `Wad_Open`
reads or writes it.

**This is a genuine divergence between the two builds, not a misreading of
either.** It does not change which entry is found (first match wins in both, and
the PSP page records zero hash collisions within any archive), only how long
the search takes. It does mean a reimplementation cannot cite the PS2 build as
evidence for the rotating cursor, and that anyone measuring lookup cost against
the original has to say which original. Confidence **85** on the difference,
from reading both loops; the PSP side is not re-verified here, since this
session did not open `BOOT.BIN`.

`Wad_EntryExists` (`0x002134c8`) is the same scan returning `entry[2] != 0`,
guarded by a lock on `this+0x4c` (a named object, `"WadSource"` at
`0x002c74f0`). `Wad_Unmount` (`0x002131e8`) is the destructor: close the backing
file, release the lock, free the entry vector.

## Not determined

- **The stored-blob stream** at `0x00257740` (`0x24` bytes). Deliberately
  unnamed; only its size and construction site are known.

`Wad_Open`'s unequal-sizes branch builds the `0x48`-byte decompressing stream at
`0x00213e58`, which **is** now read and documented: see [lzss.md](lzss.md).
- **Mount points.** The PS2 build has no `wad:` / `fe:` / `fedata:` /
  `bedata:` table like the PSP's five call sites; `fedata:` and `bedata:` do
  exist as strings but reach a different lookup. The archives referenced by name
  in this binary are `WADS2.WAD`, `PS2MUSIC.WAD` and `PRERACE.WAD`, the last two
  as `cdrom0:\54748\...` paths.
- **Whether bit 31 of `size_out` selects zlib on PS2.** Not seen at the
  `Wad_Open` level; if it exists it is inside the decompressing stream.
- **Nothing here was verified at runtime.**

## Cross-platform

| Function | PS2 (`SCES_547.48`) | PSP (`BOOT.BIN`) |
| --- | --- | --- |
| `Wad_HashName` | `0x0020c638` | `0x08940d0c` |
| `Wad_BuildCrcTable` | `0x0020c480` | `0x08940cb0` |
| `g_wad_crc_table` | `0x00303618` | `0x08afbffc` |
| `Resource_HashName` | `0x00203b78` | no second copy located |
| `Resource_BuildCrcTable` | `0x00203b18` | no second copy located |
| `g_resource_crc_table` | `0x00303160` | no second copy located |
| `Wad_MountArchive` | `0x00213058` | `0x08941e70` |
| `Wad_Open` | `0x002132c0` | `0x089411e8` |
| `Wad_Unmount` | `0x002131e8` | `0x089410d4` |
| `Wad_EntryExists` | `0x002134c8` | not located |
| `g_wad_device_vtable` | `0x0029c620` | `0x08ad3314` |
| `crc32` | `0x002068c8` | not located |

## History

- 2026-07-27: first pass on the PS2 binary. Hashing 92-95 from two independent
  in-binary implementations agreeing with the PSP reading; mount and lookup
  85-88 from decompilation. Rotating-scan divergence recorded.
