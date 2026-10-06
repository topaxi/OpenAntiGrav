# PSARC: the PS3 asset archive

**Confidence: 92.** The container is publicly documented rather than recovered
here; what this page adds is verification against a real disc, and one field
that had to be measured rather than read off a description. The check is named
below and it is the strongest kind short of a runtime trace: **11,664 of 11,664
entries carry a 16-byte digest that this project can predict from the entry's
own path**, across seven archives declaring 3.83 GiB of payload between them. It
does not reach 95 because no PS3 executable has been read; nothing here is
corroborated against the code that consumes it.

The decompression half is corroborated separately and by other pages rather than
by a count here: every file [hd-status](hd-status.md) pulls through this reader
carries the magic its extension predicts, and the 40 `.vex` files it surveys
close three independent arithmetic invariants apiece. Deflate streams do not
survive a container misread.

Applies to [`hdfury-ps3-eu.iso`](../reverse-engineering/source-images.md#hdfury-ps3-euiso---wipeout-hd--fury-ps3),
the only PS3 image this project holds. What is *in* the archives, and how much of
it the existing parsers already read, is [hd-status](hd-status.md).

**Not the PSP's `PSAR`.** The firmware update payload on a UMD starts with the
same four bytes and is a different container entirely. The
[format index](README.md) carries both rows for that reason.

## It sits behind the disc layer, not beside it

A `.psarc` on the disc lies inside an encrypted region, so
[PS3 disc decryption](ps3-disc.md) has to happen first. Everything on this page
assumes an image `scripts/ps3iso.py decrypt` has already written; against an
encrypted one the header read finds noise instead of `PSAR` and says so.

Nothing needs extracting to read an archive. `scripts/psarc.py` walks ISO 9660
for the archive's LBA and decompresses blocks in place, which is why listing all
seven takes seconds against a 2.1 GiB image.

## Layout

Big-endian throughout, which is the first thing to get wrong on a format whose
name is usually met on little-endian tooling.

```text
header, 32 bytes:
  +0x00  char[4]  "PSAR"
  +0x04  u16      version major, 1
  +0x06  u16      version minor, 3
  +0x08  char[4]  compression, "zlib"
  +0x0c  u32      total length of header + entry table + block table
  +0x10  u32      bytes per entry, 30
  +0x14  u32      entry count, the manifest included
  +0x18  u32      uncompressed block size, 65536
  +0x1c  u32      flags, 3 on every archive here

entry, 30 bytes:
  +0x00  u8[16]   MD5 of the entry's own path, uppercased
  +0x10  u32      index of this entry's first block
  +0x14  u40      uncompressed length
  +0x19  u40      byte offset of the first block, from the archive's start

block table:
  the rest of the declared length, as a flat array of block sizes
```

**Entry 0 is the manifest, not a file.** It inflates to a newline-separated list
of every other entry's path, in entry order, so entry `n + 1` is the `n`th line.
Its own digest field is sixteen zero bytes - it has no path to hash - and it is
the only entry in any of the seven for which that is true.

### The block table's element width is not declared

It is whatever divides the table evenly, and here that is **2 bytes**: a 64 KiB
block cannot deflate to more than 65,535 bytes and the exporter picked the
narrowest width that fits. Both readers try 2, 3 and 4 and take the first width
that both divides the remaining table length and leaves room for the highest
block index any entry names, rather than assuming the one this disc uses.

**The probe is only sound one way round**, and that is worth stating rather than
leaving for someone to find. An odd table length rules pairs out, so a 3-byte
width is recovered; an even one can *always* be read as pairs, so a 4-byte width
is undecidable from the table alone and the narrowest wins. An archive declaring
a block size above 65,535 would be the case to revisit it for, and none is known.

### A block size of zero means "stored"

An entry is read by walking blocks from its `+0x10` index until its declared
length is reached. A block whose size is `0` is a full `block_size` of raw
bytes - the exporter's way of saying deflating it would not have paid. Otherwise
the size is the *compressed* length, and this project decides between inflating
and copying by looking for zlib's `0x78` header byte rather than by trusting the
size alone.

**The `0x78` test applies to short blocks only, and getting that wrong cost a
file.** A full-size block is raw *by definition* - the size of zero is the table
saying so - and there is nothing left for the marker to disambiguate. Asking
anyway is asking a question with no meaning, because `0x78` is an ordinary byte
in the middle of arbitrary content: on `hdfury-ps3-eu-dec.iso`,
`Data\Music\Exceeder\music_stereo.mp3` is `DATA01.PSARC` entry 16, its block 574
is a raw MP3 block that happens to begin `78`, and the reader handed it to
`miniz_oxide` and reported the whole entry unreadable. One music track of
fifteen, dropped from a listing rather than reported.

Fixed on 2026-08-17; `psarc::tests::a_full_stored_block_beginning_with_the_zlib_marker_is_not_inflated`
pins it with a body that is *entirely* `0x78`, so every block of the fixture is
the pathological case. [`scripts/psarc.py`](../../scripts/psarc.py) never had
the bug - it emits a full block and `continue`s without testing anything - which
is what made the two implementations disagree on one file and agree on 11,663,
and is the argument for keeping the reference script around.

## The check the confidence rests on

The 16-byte field is the **MD5 of the path with every character uppercased**,
leading slash included:

```python
hashlib.md5("/DATA/FE/FONTS/CHINESE.FNT".encode()).digest() == entry.md5
```

That holds on **11,664 of 11,664 entries** across all seven archives, and it is
worth more than the count suggests, because a single check ties together three
readings that could each be wrong on their own:

1. **The manifest parse.** A path off by one line hashes to nothing.
2. **The entry ordering.** The manifest is in entry order and nothing states so;
   a different order would fail on entry two.
3. **The entry stride.** Reading 30-byte entries one byte adrift shifts every
   digest.

The lowercase spelling - which is how the paths are actually stored - matches
**zero** entries, so the uppercasing is measured rather than assumed. Reproduce
with:

```sh
just psarc verify data/images/hdfury-ps3-eu-dec.iso
```

Two weaker checks corroborate it. The manifest's line count equals the entry
count minus one on 7 of 7 archives; and every block run an entry names lies
inside the block table on 7 of 7, which a wrong block-width guess would break.

## Wipeout HD / Fury's archives

```sh
just psarc info data/images/hdfury-ps3-eu-dec.iso
```

Described by what each measurably holds, rather than by a purpose. The counts
are top-level directories inside the archive, and the split is **not** by kind -
`data/environments` and `data/ships` each appear in four of the seven, so an
asset's archive is not predictable from its path.

| Archive | Entries | Largest directories |
| --- | ---: | --- |
| `DATA00.PSARC` | 3,157 | `environments` 2,259, `fe` 265, `ships` 176, `xml` 96 |
| `DATA01.PSARC` | 53 | `sound` 31, `music` 22 - and nothing else |
| `DATA02.PSARC` | 5,486 | `environments` 3,052, `ships` 941, `fe` 377, `tex` 320 |
| `DATA03.PSARC` | 1,163 | `environments` 697, `ships` 260, `xml` 126 |
| `DATA04.PSARC` | 54 | `plugins` 29, `fe` 23 - the Asian fonts and the trophy models |
| `DATA05.PSARC` | 151 | `fe` 98, `plugins` 39 |
| `DATA06.PSARC` | 1,600 | `ships` 1,422, `fe` 72, `plugins` 53 |

All seven are `PSAR 1.3`, zlib, 64 KiB blocks, flags 3. Paths are stored in full
and in the clear, which is the one structural way this container is *easier*
than the [WAD](wad.md) it replaces: a WAD stores only a name hash and most names
still have to be [mined](../tools/oag-unpack.md), where a `.psarc` hands over
`/data/environments/talons_junction/track.vex` directly.

## Not read

- **The flags word** at `+0x1c` is 3 on all seven, so nothing here can say what
  either bit means.
- **Whether the digest is ever used at runtime.** It is a directory-integrity
  field as far as this project can tell, and the manifest makes lookup by name
  possible without it - but the executable has not been read on this point, and
  a lookup path keyed on the digest rather than on the manifest would be an
  ordinary thing for a shipping game to do.
- **`lzma` compression.** The container declares its codec and every archive
  here says `zlib`, so the other branch is unexercised and unimplemented.

## Implemented where

**`oag_formats::psarc`**, since 2026-08-17, and
[`scripts/psarc.py`](../../scripts/psarc.py) still, which is what `just psarc`
and [`hd-survey`](hd-status.md#reproducing-this) drive. The two are independent
readings of the same page, which is worth keeping: the script was written first
and the Rust reader reproduces its counts.

The Rust module does **no I/O**. An archive is gigabytes and lives inside a
disc image, so `Directory::parse` takes the declared table of contents,
`Directory::entry_range` names the bytes one entry needs, and
`Directory::read_entry` turns exactly those bytes into the entry - the caller
owns the seeking, the way `oag_assets::Archive` already does for the
[WAD](wad.md).

Two things it hand-rolls and one it does not. **MD5** is a page of RFC 1321 with
published test vectors, so `psarc::path_digest` costs nothing to own.
**Inflate** is not: deflate is a published standard whose failure mode is silent
garbage, so `miniz_oxide` does it - pure Rust with no C toolchain, the same bar
`re_rav1d` is held to, and the first third-party dependency in `oag-formats`'
default build.

The checks above run as `crates/formats/tests/psarc_ground_truth.rs`, `#[ignore]`d
like every ground-truth test because they need the decrypted image:

```sh
cargo nextest run -p oag-formats --run-ignored all -E 'binary(psarc_ground_truth)'
```

**What is not implemented**: the `lzma` branch, which no archive here declares,
and the block-width probe's 4-byte case, which is
[undecidable from an even table](#the-block-tables-element-width-is-not-declared)
and so falls back to 2.

## The PS4 Omega Collection family

Applies to [`omega-ps4-eu.pkg`/`omega-ps4-eu-patch.pkg`'s](../reverse-engineering/source-images.md#omega-ps4-eupkg--omega-ps4-eu-patchpkg---wipeout-omega-collection-ps4)
five `dataNN.psarc` archives (PS4, *Wipeout: Omega Collection*). Same 32-byte
header, same 30-byte entry stride, but two structural differences from the
PS3 archives above - split by confidence, because they are evidenced very
differently.

**Named by archive family, not by declared version, after a regression.**
An earlier version of this section called these "version 1.4" archives and
keyed the container's own behaviour off `header.version_minor >= 4` -
plausible, since all five of `omega-ps4-eu`'s archives happen to declare
1.4, but wrong: Vita `2048`'s `data.psarc` also declares version 1.4 and is
the well-behaved PS3 shape throughout (newline-delimited manifest, zero
placeholder rows, 100% positional). A version-only dispatch broke every
Vita-backed path lookup on `main` for exactly as long as the fix below was
merged without this correction. `parse_manifest` now reads the manifest's
own bytes to choose a delimiter (no `\n` byte anywhere, but at least one
`\x00`, reads as NUL-delimited; anything else reads as newline-delimited),
and `match_paths_to_entries` always matches by digest rather than
dispatching on version - which reproduces the well-behaved case's own
positional order as a corollary, not a fact that needs its own code path.
See `crates/formats/src/psarc.rs`'s module docs for the full argument.

### Manifest delimiter and entry/path correspondence - confidence 92

**Every number on this page is from the corrected extraction**
([below](#block-data-location-and-the-short-read-extraction)): the short-read
copy this project first measured made these archives look torn, and nothing
about that picture survives a whole extraction. What was retired is listed in
the same section, so a reader holding an older note can find it.

**The manifest is NUL-delimited, not newline-delimited.** `data00.psarc`'s
598,798-byte manifest carries zero `\n` bytes and 10,925 `\x00` bytes:
exactly the separators between 10,926 paths, with no empty segment and no
zero run anywhere in the text. Splitting on `\x00` recovers every path, and
they are 10,926 distinct strings. All nine `omega-ps4-eu{,-patch}`
archives read the same way.

**The entry table is digest-ascending, one run, and is not in manifest
order.** The PS3 invariant "entry `n + 1` is manifest line `n`" does not hold
here: `psarc_census` finds it true for 3 of `data00`'s 10,926 entries, and
the manifest is not sorted either. The entry table is the ordinary
PSARC 1.4 shape - the manifest first, then every file sorted by the MD5 of
its path - so a path is found by digest, which is what
`match_paths_to_entries` always does. **There is no placeholder row and no
orphan**: on all five base archives `zero-digest rows`, `entries with no
path` and `paths with no entry` are all 0.

| Archive | Manifest paths | Entries behind the manifest | Matched by digest |
| --- | ---: | ---: | ---: |
| `data00.psarc` | 10,926 | 10,926 | 10,926 |
| `data01.psarc` | 4,641 | 4,641 | 4,641 |
| `data02.psarc` | 5,611 | 5,611 | 5,611 |
| `data03.psarc` | 661 | 661 | 661 |
| `data04.psarc` | 4,569 | 4,569 | 4,569 |

The patch's four (`data05` 8,205, `data07` 6, `data08` 11,266, `data09` 120)
are the same: a bijection, 46,005 of 46,005 across the nine archives. That is
as exact as the PS3 check (every one of 11,664), and is why the confidence is
92 rather than the 88 an incomplete table earned. It is the same
self-consistency evidence, not a runtime trace, so it does not go higher.

**Entries share storage.** 4,121 of `data00`'s entries sit at an
`(offset, size)` another entry also names, 367 of `data03`'s 661: the packer
stores one copy of identical content and points every name at it. Eleven
ship liveries' `ShieldHexagonal_ALPHA.gnf` on `data03.psarc` (entries 16, 66,
75, 85, 110, 111, 122, 125, 208, 287, 308) all declare
`(114606848, 49408)` and all read `GNF `; two `Holographic_02_GLOW.gnf`
(12, 100) share `(114590208, 16640)` just before them. `Archive::paths` lists
each name once, and `index_of_path` resolves to the entry that carries it, so
sharing changes nothing for a caller.

**Sixteen entries on `data00.psarc` have size zero** - all
`Data/audio/sound/*.txt`, sharing `first_block` 25,567 and `offset`
1,619,318,029 - and read as empty (the patch's `data08` has sixteen size-zero
entries too). They are empty files, not missing content; `psarc_oracle`
reports them `unvalidated_all_zero`.

Implemented as the content-based branch in `parse_manifest` and the
always-by-digest `match_paths_to_entries`.
`crates/formats/src/psarc/tests.rs` pins the NUL split, the newline split
on a manifest that would have been misread by a version-based dispatch, and
the digest match/drop behaviour with a synthetic table;
`crates/assets/examples/psarc_census` reproduces the table above, and
`crates/assets/tests/omega_psarc_ground_truth.rs` ratchets it and reads every
entry:

```sh
cargo run -p oag-assets --release --example psarc_census -- data/extracted/ps4/omega-eu/uroot/data00.psarc
```

### No corrupt row - retired

An earlier version of this page described one row per archive on
`data00`/`data02`/`data04` (entries 9042, 4676, 318) with a real digest, a
`first_block` in the billions and, on two, a `size` past the archive's own
length, and put it down to a torn write; it also described a fourth row shape
at index 2183 (a digest of fourteen zero bytes and two real ones) and a
duplicated digest on `data00` (entries 6846 and 6857). **All three were the
extraction tool's zero-padding**, not properties of the archive: the entry
table is part of the file the short read damaged. On a whole extraction every
row is well-formed - `data00`'s size-zero entries above carry a sane
`first_block` and `offset`, and all 46,005 entries read without error.

The code that came out of it stays, because it is sound on its own terms:
`read_block_table` still leaves a `first_block` no block table could hold out
of the width probe, and `Directory::parse` still validates an entry's block
range lazily in `Directory::entry_range` rather than up front. Neither
triggers on a whole extraction; a damaged one is exactly where a reader
should degrade to a per-path error rather than refuse the archive.

### Block data location and the short-read extraction

**Most of what this page used to say here was measured on a short-read
extraction and is wrong.** `PkgTool.Core`'s `PFSCReader.ReadSector` made one
`Stream.Read` and treated a short return as a whole sector, leaving the rest
of the sector zero. The tool was fixed and both `.pkg` files re-extracted
2026-09-27; that copy became `data/extracted/ps4/{omega-eu,omega-eu-patch}`
on 2026-09-29 (the short-read one is `data/extracted/ps4.bak`, a local
backup with no other use). The root cause, the patch and the before-and-after
are in [`gnf.md`'s "Root cause"](gnf.md#root-cause-confidence-90-a-short-streamread-in-the-extraction-tool-not-this-projects-reader)
and [`source-images.md`](../reverse-engineering/source-images.md); this page
keeps only the measurements that survive.

**What the short read produced, retired here:**

- A "real / all-zero / garbage" split of **30-70 % real** per archive, an
  "unexplained block-data-location" problem, and a first-byte oracle, its
  replacement magic oracle, and both of their tables (`.gnf` valid 413 / 188 /
  256 / 58 / 305 on the five archives, and their `garbage` and `all-zero`
  columns). Every one of those counts was the same measurement with more or
  fewer zero-padded sectors in it.
- A `data00` manifest "truncated mid-string" with three zero runs, and 10,714
  paths against 1,533 real entries - 86-90 % "dead text", "thousands of
  placeholder rows", "several descending runs" in the entry table. The
  manifest was itself read through the buggy decompressor and the entry table
  is part of the file it damaged.
- A constant-offset check on three `garbage` `.gnf` entries, a theory that
  PlayGo streaming state could leave a static dump un-resolved, and a lead
  about an unclean base/patch merge. Each was a lead about the symptom; the
  merge one closed negative for its own reason, kept below.
- `ShieldHexagonal_ALPHA.gnf` being "genuinely missing content at a correct
  offset": it is real GNF content, shared by eleven names (see above).

**The corrected oracle** (`crates/assets/examples/psarc_oracle.rs`, which reads
every entry and checks the format's own magic - `VEXX` at `+0x0c` on `.vex`,
`GNF ` on `.gnf`, `ED AD 5C CA` on `.rcsmodel`/`.rcsskeleton`/`.rcsanimclip`,
`E5 AD 5C CA` on `.rcsmaterial`) on the whole extraction:

| Archive | valid | big-endian `.vex` | magic missing | empty (size 0) | no magic to check |
| --- | ---: | ---: | ---: | ---: | ---: |
| `data00.psarc` | 9,137 | 6 | 0 | 16 | 1,767 |
| `data01.psarc` | 4,507 | 10 | 0 | 0 | 124 |
| `data02.psarc` | 5,421 | 8 | 0 | 0 | 182 |
| `data03.psarc` | 632 | 2 | 0 | 0 | 27 |
| `data04.psarc` | 4,465 | 0 | 0 | 0 | 104 |

**The 26 `.vex` an earlier count called "garbage" carry the other byte
order's magic.** Every one opens `00 00 00 06` ... `58 58 45 56` - `XXEV` at
`+0x0c`, the big-endian spelling of `VEXX` that the PS3 files carry
([`vex.md`](vex.md)); examples are `start_grid_*`, `Junk/*`, `hdships/zone/*`,
`mageffect*` and the `Qirex_LANDSCAPE` billboard. Presumably the packer took
them from the PS3 tree unconverted; the container-level check here only
proves the magic, not that the tree decodes. `psarc_oracle` reports them as
`valid_big_endian`, the ground-truth test counts them per archive, and
`oag_vex::vex` accepts both spellings. **Every entry with a magic carries one
of its two spellings; there is no all-zero content entry and no torn row.**

**Two structural findings hold and are re-measured:**

- **The block table's arithmetic is self-consistent.**
  `max(entry.offset + entry.size)` lands **exactly** on the file's size on
  all nine archives (`data03`: 2,576,997,583). Referenced block rows are
  almost all of the table now - `data03` 39,587 of 39,640, `data00` 212,009
  of 212,718 - where the short-read copy referenced 12,099 of `data03`'s.
- **Nothing is deflated.** Every block row is exactly `0` (a full, padded
  block of stored bytes) or exactly the entry's remaining byte count (a short
  stored block, never padded); **zero** rows fall between - `data00` 203,537
  full and 8,472 short, `data03` 39,182 and 405, the patch's `data05`,
  `data08` and `data09` the same shape. The header's `compression: "zlib"`
  reads the same four bytes as every PS3 archive, but every block sampled or
  counted here is a plain copy.

**The game never reads a `.psarc`'s block table itself.** `eboot.bin`'s own
code was read looking for the loader this project's reader should be compared
against
([`docs/ghidra/functions/ps4-omega-eu/psarc-mount.md`](../ghidra/functions/ps4-omega-eu/psarc-mount.md)),
and there isn't one: `PsarcArchive_Mount` (confidence 85) calls straight into
Sony's own `sceFiosArchiveGetMountBufferSizeSync`/`sceFiosArchiveMountSync`
FIOS2 exports, behind `PsarcArchive_WaitAndMountAll` (confidence 80), a
background-thread loop that polls `scePlayGoGetLocus` per archive and mounts
each only once its PlayGo chunk reports locally installed. That corroborates
`.psarc` being a first-party Sony container with a first-party mounter, from
the executable side. The block-read implementation lives inside
`libSceFios2.prx`, a separate signed system module this project holds but has
not opened in Ghidra.

**The patch is its own extraction and adds content.** `pkg_extract` takes
exactly one `.pkg` and one output directory (`PkgTool/Program.cs`, read from
source: no patch-chain or merge logic anywhere), so
`omega-ps4-eu-patch.pkg` extracts to a directory of its own, and its `uroot/`
holds **four archives with names the base `.pkg` does not have** - `data05`
(654 MiB), `data07` (20 MiB), `data08` (5.3 GiB), `data09` (6.9 MiB), no
`data06`, and no `data00`-`data04` - so it cannot be "the missing bytes" of
any base archive. The base `.pkg` alone extracts to 40.6 GiB across five
archives (`data00` 13 GiB, `data01` 11 GiB, `data02` 9.1 GiB, `data03` 2.5 GiB,
`data04` 6.0 GiB). See `source-images.md`'s Omega Collection section for the
full patch record.

`psarc_sweep` (`crates/assets/examples/psarc_sweep.rs`) is superseded by
`psarc_oracle` and kept only as the record of a first-byte heuristic that
misread real files whose payload starts after a run of zeros; its numbers, like
every other short-read number, describe the zero-padding.

```sh
cargo run -p oag-assets --release --example psarc_oracle -- \
  data/extracted/ps4/omega-eu/uroot/data03.psarc
# gnf: valid 331 ... vex valid 154, valid_big_endian 2

cargo run -p oag-assets --example psarc_cat -- \
  data/extracted/ps4/omega-eu/uroot/data03.psarc \
  "Data/art/published/hdships/icaras_n1/Livery1/ShieldHexagonal_ALPHA.gnf" \
  | xxd | head -2
# GNF at byte zero: the shared entry, read whole
```

**Consequence for this crate:** `Directory::entry_range`/`Directory::read_entry`
are unchanged and trust `entry.offset` directly, exactly as the PS3 reading
does, and on a whole extraction that is all it takes: every entry
`Archive::paths` lists reads real content, as it does for Vita `2048`'s
`data.psarc` and for HD. Naming an entry and reading it are the same thing
here; the caution the older `Archive::paths` doc carried belonged to the
damaged copy.

## See also

- [Format index](README.md) - the `.psarc` row, and the PSP `PSAR` row it is not
- [PS3 disc encryption](ps3-disc.md) - the layer that has to come first
- [HD status](hd-status.md) - what is inside, and what already parses
- [Legal](../overview/legal.md) - why no key and no content is committed

## `scripts/psarc.py` reads a NUL-delimited manifest too (2026-10-06)

The Python helper split the manifest on `\n` only, so on a PS4 archive its
`list` printed one giant entry and `cat`/`extract` found nothing by name. A
census made with it said Omega ships no `arcade_hud.xml`; it ships five roots
at HD's own paths (`omega-status.md`, the HUD section). It now splits on NUL as
well, as `parse_manifest` does. `extract` on `data00.psarc` still fails with a
zlib "incorrect header check" on that archive: use the Rust reader there.
