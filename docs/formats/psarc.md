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

### Manifest delimiter and entry/path correspondence - confidence 88

**The manifest is NUL-delimited, not newline-delimited.** Measured directly
on `data00.psarc`: its 598,798-byte manifest carries zero `\n` bytes.
Splitting on `\x00` and dropping empty segments recovers 10,714 well-formed
paths - **not** the 22,582 a naive `split(NUL).len()` reports, which is
counting 11,868 empty segments produced by long zero-byte runs inside the
manifest text itself (three runs, 3,083 + 1,955 + 6,833 bytes, one of them
truncating a real name mid-string - `Qirex_Col_A` where a `.gnf` extension
should follow). Those runs are already deflate output, not a read bug (no
`ShortBlock`), so the game genuinely ships stale zeroed-out manifest text -
recorded as unrecoverable, not reconstructed.

**Entry order carries no relationship to manifest order at all.** The PS3
invariant "entry `n + 1` is manifest line `n`" does not hold: the entry table
is a concatenation of several separately digest-ascending runs (one descent
in the sequence per extra run - `data00.psarc` has four, `data01.psarc` two,
`data02.psarc`/`data04.psarc` one each, `data03.psarc` none), and it carries
thousands of fully-zeroed placeholder rows (digest, `first_block`, `size` and
`offset` all `0`) for manifest paths this particular archive does not store.
A fourth row shape turns up too, one per archive on `data00`/`data01`/
`data02`/`data04`, always at index 2183: a digest that is fourteen zero bytes
and two real ones, not the all-zero placeholder shape and not a real path's
MD5 either, with plausible-looking geometry beside it. `match_paths_to_entries`
handles it the same way it handles any real entry whose digest matches no
manifest path - drops it - so it needs no special case, but the "placeholder
vs. real" split above is not the whole shape of the table.

The correspondence that *does* hold, checked against `path_digest` on all
five archives:

| Archive | Manifest paths | Real (non-zero-digest) entries | Matched by digest |
| --- | ---: | ---: | ---: |
| `data00.psarc` | 10,714 | 1,533 | 1,492 |
| `data01.psarc` | 4,641 | 894 | 891 |
| `data02.psarc` | 5,611 | 927 | 923 |
| `data03.psarc` | 661 | 328 | 327 |
| `data04.psarc` | 4,569 | 840 | 837 |

97-99% of real entries resolve to a manifest path by MD5, on every archive -
1,492/1,533, 891/894, 923/927, 327/328 and 837/840 respectively. That is not
an *exact* invariant the way the PS3 check is (every one of 11,664 matches
there); it corroborates the digest-based correspondence rather than proving
it outright, which is what keeps this claim's confidence at 88 rather than
in the PS3 page's 92. The 86-90% of manifest paths
that *don't* resolve to a local entry are not a split-namespace scheme
either: checked directly, at most 2 of `data00.psarc`'s 9,222 orphaned paths
turn up as a real entry in any of the other four archives - noise, not a
pattern. They are dead text, most plausibly left over from incremental
repacking that zeroed a removed file's manifest and entry-table rows in
place rather than compacting around them (the three zero-byte manifest runs
above are the same behaviour caught mid-edit).

**One digest is shared by two entries on `data00.psarc`** (1,533 real rows,
1,532 distinct digests): entries 6846 and 6857 carry the same digest and the
same `first_block`, but entry 6857's `size` and `offset` are both zero -
another instance of the "digest survives, geometry doesn't" pattern the
corrupt-row section below documents, not a second file with the same name.
`match_paths_to_entries` resolves both to the same path, so `Archive::paths`
lists it twice; `index_of_path`'s first-match `.position()` resolves to
entry 6846, the row with real geometry, so lookup by name is unaffected.

Implemented as the content-based branch in `parse_manifest` and the
always-by-digest `match_paths_to_entries`, which drops both an unmatched
manifest path and an unmatched real entry rather than guessing at either.
`crates/formats/src/psarc/tests.rs` pins the NUL split, the newline split
on a manifest that would have been misread by a version-based dispatch, and
the digest match/drop behaviour with a synthetic table;
`crates/assets/examples/psarc_list` reproduces the table above's "Matched by
digest" column against real data (its own path count, not the manifest-path
or real-entry counts, which need reading the directory and manifest
separately):

```sh
cargo run -p oag-assets --example psarc_list -- data/extracted/ps4/omega-eu/uroot/data00.psarc
```

### A single corrupt row per archive - confidence 75

Three of the five archives (`data00.psarc` entry 9042, `data02.psarc` entry
4676, `data04.psarc` entry 318) each carry **exactly one** row with a real,
non-zero digest and a `first_block` in the billions - `data02`'s and
`data04`'s also declare a `size` past their own archive's length (56.5 GB and
740 GB, inside 9.1 GB and 6.0 GB files). No candidate block-table width could
ever cover a `first_block` that large, so before this was handled the whole
directory failed to parse - two of the five archives (`data02`, `data04`)
could not be opened at all. `read_block_table` now excludes any entry whose
`first_block` exceeds what the narrowest possible block table could hold from
its own `highest`-block computation (the same treatment the already-fixed
`size == 0` sentinel gets), and `Directory::parse` no longer validates every
entry's block range up front - that check already exists on
`Directory::entry_range` and now runs lazily, so this one bad row surfaces
as a read error on the single path that names it instead of refusing the
archive. Confidence is 75 rather than higher because *why* exactly one row
per archive is left this way is not established - a single torn write is the
working description, not a verified cause.

### Block data location - open, and a real/zero split rather than uniformly broken

**`entry.offset` locates real content for a substantial fraction of real
entries already, through the crate's existing, unmodified reader - and zero
bytes for the rest, on every archive checked.** Corrected from an earlier
version of this page, which sampled only entries whose first block's table
row is non-zero (a description of stored-block *shape*, not of whether the
content is real - see below) and reported zero hits; a full sweep over every
real entry, using `Archive::read_path`/`psarc_cat` directly rather than a
hand-rolled reimplementation, finds:

| Archive | Real entries (`size > 0`) | Non-zero at `entry.offset` | All-zero |
| --- | ---: | ---: | ---: |
| `data00.psarc` | 1,528 | 819 (54%) | 709 |
| `data01.psarc` | 892 | 348 (39%) | 544 |
| `data03.psarc` | 327 | 99 (30%) | 228 |

Two concrete examples, both read correctly by `psarc_cat` as it stands today,
no code change: `data03.psarc`'s
`Data/art/published/hdships/harimau/Livery2/Holographic_02_GLOW.gnf` opens on
`GNF ` (Sony's PS4 texture magic), and
`Data/art/published/hdships/auricom/Ship_LOD.vex` opens on version `6` then
`VEXX` at the expected `+0x0c`. **`Ship_LOD3.vex`** (a different, similarly-named
file, declared offset 417,565,545, single 976-byte block), the example an
earlier version of this page used to argue extraction was uniformly broken,
is real but is one of the ~70% zero cases on `data03.psarc` - not
representative, and left below only as a reproducible all-zero instance.

The zero/non-zero split does not correlate with anything checked so far:

- **Not the stored-block shape.** A block's table row is either `0` (a full,
  padded `block_size`) or equal to the entry's exact remaining byte count (a
  short, unpadded stored block, never a genuinely shrunk compressed one - see
  below). Both shapes turn up real and zero content in roughly the same
  proportion on `data03.psarc` (91/274 real for the full-padded shape, 8/53
  for the short shape).
- **Not file position.** Real and zero entries are interleaved throughout
  `data03.psarc`'s whole offset range in 200 MB buckets, not confined to a
  prefix, a suffix, or any other contiguous region.
- **The block table's own arithmetic is otherwise self-consistent.** Only
  12,099 of `data03.psarc`'s 39,640 block-table rows (31%) are referenced by
  any real entry's `first_block` + block count - the rest describe blocks no
  entry claims - and `max(entry.offset + entry.size)` over every real entry
  lands **exactly** on the file's true size, 2,576,997,583 bytes. Read as a
  coordinate system, `entry.offset` spans the archive precisely; it is
  specific entries' *content* that reads as zero; the offsets these entries
  keep company with are not obviously wrong as numbers.

**One thing this does rule out: it is not a codec mismatch.** Classified
every block belonging to a real entry, on `data00.psarc`, `data01.psarc` and
`data03.psarc`: each block's table value is either exactly `0` (a full,
padded `block_size` of stored bytes) or exactly equal to the entry's
remaining byte count at that block (a *short* stored block, never padded).
**Zero** blocks fall between those two cases - the signature a genuinely
`deflate`-shrunk block would leave. The header's `compression: "zlib"` field
reads the same four bytes as every PS3 archive, but nothing checked here is
actually deflated: every real file sampled on this family is stored raw. A
non-zero entry that reads correctly is therefore a plain byte copy, and a
zero one is not a decompression failure either - there is no decoding step
in either case to have gotten wrong.

**The extraction-provenance lead is closed, negative.** An earlier version of
this page treated the `data/extracted/ps4/omega-eu/` directory's ~40.8 GiB
(against `source-images.md`'s recorded "~25 GiB, base `.pkg` only") as
unexplained, and floated an unclean base/patch merge as a possible cause of
the real/zero split. Checked directly, 2026-09-15: `PkgTool.Core pkg_extract`
takes exactly one `.pkg` and one output directory
(`PkgTool/Program.cs`'s `pkg_extract` verb, read from source - no patch-chain
or merge logic anywhere in the tool) and simply overwrites nothing it doesn't
touch, so a base-only extraction cannot have silently absorbed patch content.
The `~25 GiB` figure was just an imprecise earlier estimate; the base `.pkg`
alone reproducibly extracts to 40.6 GiB across its five archives (`data00`
13 GiB, `data01` 11 GiB, `data02` 9.1 GiB, `data03` 2.5 GiB, `data04` 6.0 GiB -
`source-images.md`'s own count corrected to match).

Extracting `omega-ps4-eu-patch.pkg` on its own, same tool, same verb, settles
it further: the patch's `uroot/` holds **four archives with names the base
`.pkg` does not have at all** - `data05` (654 MiB), `data07` (20 MiB), `data08`
(5.3 GiB), `data09` (6.9 MiB), no `data06`, and critically **no `data00`-`data04`** -
so a patch extraction cannot be "the missing bytes" for any of this page's
five base archives; it adds new content, it does not complete old content.
See `source-images.md`'s Omega Collection section for the full patch
extraction record.

**And the same real/zero split is already present in the patch's own,
freshly-extracted archives**, measured the same way (`psarc_sweep`, below):
`data05.psarc` 415/1,207 (34%), `data07.psarc` 6/7 (86%), `data08.psarc`
796/1,789 (44%), `data09.psarc` 84/121 (69%) - the same 30-70%-ish range as
the five base archives, on a directory this session extracted itself,
minutes before measuring it. That rules out "one bad extraction run" as an
explanation as thoroughly as the merge theory above did: two independent
extractions, from two different `.pkg` files, on the same machine, in the
same session, both show the split. Whatever produces it is a property of
this archive family (or of `PkgTool.Core`'s own PFS reader, not chased
further - see below), not of one directory's provenance.

Reproduce the corrected measurement, and its trap:

```sh
cargo run -p oag-assets --release --example psarc_sweep -- \
  data/extracted/ps4/omega-eu/uroot/data03.psarc
# 100/328 (30.5%) non-zero - matches this page's original table (99/327, 30%)
```

**The trap this tool had to be rewritten around**: a first draft of
`psarc_sweep` checked `bytes.iter().any(|&b| b != 0)` over the *whole*
decoded entry - "is there a nonzero byte anywhere" - and got wildly higher
non-zero rates (87% on `data00`, 70% on `data03`) that looked, briefly, like
either a crate bug only triggered by reading many entries through one
`Archive` in a loop, or like this page's own original table being stale.
Neither was true: a large buffer that is genuinely corrupt/zero at the
*header* - the only place that matters, since that is what a real reader
checks first - can still contain a single stray nonzero byte tens of
kilobytes in, and "any nonzero byte" counts that as real. Checking
`bytes.first()` instead - matching this table's own "non-zero **at**
`entry.offset`" column header literally - reproduces the original 54%/39%/30%
figures almost exactly (820/1,530, 347/892, 100/328). Two named entries this
page already cited settled which check to trust before the fix: both
reproduced byte-for-byte identically to their original description either
way, so the discriminator had to be a *third* set of entries, not the two
already on this page.

Reproduce a real read and an all-zero one:

```sh
cargo run -p oag-assets --example psarc_cat -- \
  data/extracted/ps4/omega-eu/uroot/data03.psarc \
  "Data/art/published/hdships/harimau/Livery2/Holographic_02_GLOW.gnf" | xxd | head -2
# opens on `GNF ` - real content, through the existing reader, unmodified

cargo run -p oag-assets --example psarc_cat -- \
  data/extracted/ps4/omega-eu/uroot/data03.psarc \
  "Data/art/published/hdships/auricom_n1/Ship_LOD3.vex" | xxd | head
# 976 bytes, all zero - a real entry, but one of the ~70% zero cases
```

**Consequence for this crate:** `Directory::entry_range`/`Directory::read_entry`
are unchanged and still trust `entry.offset` directly, exactly as the PS3
reading does - and, for the fraction of entries measured above, that already
produces correct content with no code change. `Archive::paths` on one of
`omega-ps4-eu`'s five archives specifically names entries this crate can
*locate in the directory and match to a path*; for any individual one of
them, whether reading it back gives real bytes is not yet predictable from
anything this page has found - roughly a third to a half will, and which
third is still open. This is a property of that archive family, not of a
declared version number - every other archive read so far, Vita `2048`'s
included, reads real content for every entry `paths()` lists.

## See also

- [Format index](README.md) - the `.psarc` row, and the PSP `PSAR` row it is not
- [PS3 disc encryption](ps3-disc.md) - the layer that has to come first
- [HD status](hd-status.md) - what is inside, and what already parses
- [Legal](../overview/legal.md) - why no key and no content is committed
