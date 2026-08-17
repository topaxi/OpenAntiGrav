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

## See also

- [Format index](README.md) - the `.psarc` row, and the PSP `PSAR` row it is not
- [PS3 disc encryption](ps3-disc.md) - the layer that has to come first
- [HD status](hd-status.md) - what is inside, and what already parses
- [Legal](../overview/legal.md) - why no key and no content is committed
