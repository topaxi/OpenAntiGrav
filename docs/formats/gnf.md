# GNF: the PS4's native texture container

**Confidence: 80 for the container and descriptor, 75 for the BC7 block
decoder, 80 for the micro-tile (`Thin_1DThin`) address formula.** A genuine
Sony SDK format ("Gnm Format") with no first-party spec this project holds,
so the layout is triangulated from two independent, non-affiliated
open-source implementations and one public AMD hardware reference rather
than read off an SDK header. See
[`psarc.md`](psarc.md#the-check-the-confidence-rests-on) for why this project
scores a publicly-documented-but-not-recovered-here container in the 80s-90s
rather than treating "publicly documented" as a free pass to 100. The
address formula's own 80 is a whole-image MAD of 1.66 against an HD `.gtf`
oracle on a real, multi-tile-row front-end image (`hex_select.gnf`, see
"Tiling" below) - single digits is the "correct match" band
[`gxt.md`](gxt.md)'s own method uses - not higher because the corpus that
would raise it (a wider spread of oracle-paired sizes, decoded and diffed in
a committed ground-truth test rather than a scratch example run by hand) has
not been built.

Applies to `.gnf` entries inside `omega-ps4-eu`'s `dataNN.psarc` archives -
see [`psarc.md`'s](psarc.md#the-ps4-omega-collection-family) own PS4 section
for how those are read and which entries' bytes are actually present.

## Layout

Little-endian - the one container in this project that is, since nothing
upstream of PS4 (PSP, PS2, PS3, Vita) is a little-endian target.

```text
header, 8 bytes:
  +0x00  char[4]  "GNF "
  +0x04  u32      contents size

contents, 8 bytes, at +0x08:
  +0x08  u8       version           2 on every file seen
  +0x09  u8       texture count
  +0x0a  u8       alignment         8 on every file seen
  +0x0b  u8       reserved
  +0x0c  u32      stream size (total file length)

per texture, 36 bytes, at +0x10:
  +0x00  u32      word0 - GPU base address, always 0 on disk
  +0x04  u32      word1 - min_lod_clamp:12@8, surface_format:6@20, channel_type:4@26
  +0x08  u32      word2 - (width-1):14@0, (height-1):14@14, sampler_modulation:3@28
  +0x0c  u32      word3 - channel order x/y/z/w: 3 bits each @0/3/6/9,
                          base_mip_level:4@12, last_mip_level:4@16,
                          tile_mode:5@20, is_pow2_pad:1@25, texture_type:4@28
  +0x10  u32      word4 - (depth-1):13@0, (pitch-1):14@13
  +0x14  u32      word5 - base_array_slice:13@0, last_array_slice:13@13
  +0x18  u32      word6 - min_lod_warning, mip stats, DCC flags
  +0x1c  u32      metadata offset (pixel-data length, for one texture)

pixel data, at +0x08 + contents_size (conventionally +0x100)
```

`word1`-`word6` are the raw AMD GCN "T#" image resource descriptor - eight
dwords, 256 bits - the same hardware structure AMD's public "Sea Islands
Series Instruction Set Architecture" reference manual documents in Table
8.13 as `SQ_IMG_RSRC_WORD0..7`. GNF's own `+0x1c` metadata-offset dword is
the one field on top of the raw hardware descriptor.

## Where this comes from

Three independent sources agree, checked here against a real
`omega-ps4-eu` entry rather than trusted on citation alone:

1. [GFD-Studio](https://github.com/tge-was-taken/GFD-Studio)'s
   `GFDLibrary.Textures.GNF.GNFTexture` - a from-scratch C# reader/writer
   used by PC ports of Fallout 4/Skyrim SE that need to *produce* valid
   `.gnf` files for a PS4 build - gives the exact byte offsets and bitfield
   widths above.
2. The [PlayStation GNF Image](https://rewiki.miraheze.org/wiki/PlayStation_GNF_Image)
   reverse-engineering wiki page independently gives the same
   header/contents split, naming the same fixed values (version 2, alignment
   8).
3. [shadPS4](https://github.com/shadps4-emu/shadPS4)'s `AmdGpu::Image`
   (`src/video_core/amdgpu/resource.h`) is the same 8-dword descriptor one
   layer down, and its `DataFormat`/`NumberFormat` enums carry the identical
   numeric values as GFD-Studio's `SurfaceFormat`/`ChannelType` (`BC7 =
   0x29`, `Srgb = 9` on both) - two unrelated projects reading the same fact
   rather than one guess repeated.

Verified against `Data/art/published/hdships/harimau/Livery2/Holographic_02_GLOW.gnf`
(a real, non-zero `data03.psarc` entry - see `psarc.md`'s "valid" bucket):
`word1` decodes to `SurfaceFormat::Bc7`/`ChannelType::Srgb`, `word2` decodes
to 128x64, `word3`'s four channel-order fields decode to the ordinary
Red/Green/Blue/Alpha order, and its tile mode decodes to index 13 - a
genuinely tiled mode, not a coincidence a wrong reading could produce by
accident. **That index is `Thin_1DThin`, not `Thin_2DThin` as an earlier
pass here read it** - GFD-Studio's own `TileMode.cs` enum (fetched directly
from its GitHub source and re-checked, since this project's earlier label
traced back to the same source without verifying the exact index) lists
`Thin_1DThin = 0x0000000D` and `Thin_2DThin = 0x0000000E`: micro-tiled
only, one index earlier than this page previously said. See "Tiling"
below for how that was found and what it changes.

## Census: every valid `.gnf` across all nine Omega archives

`crates/texture/examples/gnf_census.rs` parses every entry ending `.gnf` in
the five base (`omega-eu`) and four patch (`omega-eu-patch`) archives - 2,670
entries examined, 1,407 with a valid `GNF ` header and a sane descriptor (the
rest are the `psarc.md`-measured "garbage"/"all-zero" population, a property
of this local dump rather than of the reader - see that page's "Block data
location" section):

| `SurfaceFormat` | `TileMode` | Count | Size range | Archives | Example |
| --- | --- | ---: | --- | --- | --- |
| `Bc7` | `0x0d` (`Thin_1DThin`) | 1,378 | 2x1 .. 8192x8192 | all nine | `Data/fe/images/badges.2x.gnf` |
| `Bc4` | `0x0d` | 11 | 1024x512 .. 4096x2048 | patch `data08` | `data/fe/fonts/chinese.gnf` |
| `Bc1` | `0x0d` | 4 | 4096x4096 | base `data00`, `data04` | `Grass_Patch_Red_Terrain_BC1.gnf` |
| `Other(0x01)` | `0x0d` | 7 | 2048x1024 .. 4096x2048 | base `data00`, patch `data07` | `data/fe/fonts/chinese.gnf` |
| `Other(0x05)` | `0x0d` | 6 | 512x512 .. 1024x1024 | base `data01`, `data02` | `ds_wall_displacement_df2.gnf` |
| `Other(0x0c)` | `0x0d` | 1 | 512x1024 | base `data01` | `ds_stripe_displacement_df3.gnf` |

**Every single valid entry declares `TileMode(0x0d)`.** No linear
(`Display_LinearAligned`/`Display_LinearGeneral`) `.gnf` has been found
anywhere in the corpus - `Texture::is_linear` names both values, and this
census is the negative evidence that neither has turned up. `Other(0x01)`
and `Other(0x05)`/`Other(0x0c)` are AMD `DataFormat` codes this module does
not name (single- or dual-channel formats used for font atlases and
displacement maps); at 14 files combined they are out of scope for this pass
and are refused by `Error::UnsupportedFormat` rather than guessed at.

## The cross-title oracle

The same method [`gxt.md`](gxt.md)'s `PVRTII4BPP`/`UBC1`/`UBC3` sections used
for Vita `.gxt` against Wipeout HD's `.gtf`, generalised to a third title and
a third codec: Omega re-ships HD's ship liveries and 2048's DLC circuits, so
a same-team, same-livery-slot, same-dimension `.gnf`/`.gtf` (or `.gnf`/`.gxt`)
pair is the same authored texture through two encodes.
`crates/texture/examples/gnf_census.rs` finds **336 `.gnf`<->`.gtf` pairs**
(matched by basename and dimensions) and **403 `.gnf`<->`.gxt` pairs** against
the base HD/2048 corpora - 738 total, **all but one `Bc7`**. This is a large,
real oracle set; what it has not yet been used for is below.

## The BC7 block decoder

**Confidence 75.** `oag_texture::bcn::bc7`, added 2026-09-21, transcribed
directly from the public Khronos `GL_ARB_texture_compression_bptc` extension
specification text (the same public description Microsoft's DirectX BC7 and
the Khronos Data Format Specification give) - not read from any reference
*implementation*, GPL or otherwise. `Table.M` (the eight modes' field
widths), `Table.P2`/`Table.P3` (partition assignment), `Table.A2`/`A3a`/`A3b`
(anchor index per partition) and the interpolation weight tables are
reproduced verbatim in `crates/texture/src/bcn/bc7/tables.rs`; the bit
reader, endpoint expansion and weight interpolation are `bc7.rs`'s own,
following the spec's stated field order and rounding rule. Unit-tested by
hand-building synthetic mode-6 blocks (bit-writer the exact inverse of the
decoder's own bit-reader) and checking known endpoint values round-trip
exactly, including the one-bit-narrower anchor index.

**75 rather than higher because nothing here has been checked against a real
decoded picture yet** - the 738-pair oracle above exists, but every paired
`.gnf` is also tiled, so reaching it needs the untiler below. This is
"decompilation only, consistent" territory on
[the confidence rubric](../reverse-engineering/confidence-rubric.md)'s own
scale, one step up for being a public spec rather than a decompilation: the
implementation is internally consistent and spec-exact, not yet externally
corroborated.

## Tiling: `Thin_1DThin` is untiled, guarded by a corrupt-block refusal

**Every one of the 1,407 valid `.gnf` entries this census found declares
`TileMode(0x0d)`.** `Texture::decode` now untiles it - see "The micro-tile
formula" below for the address formula and "The corruption is not tiling:
it is PSARC-level missing content" for what closed the periodic-corruption
question this section used to leave open. A tile mode with no address
formula here at all still refuses by name (`Error::Tiled { tile_mode }`),
the same rule this project applied when the header/descriptor work first
landed - just no real `.gnf` this project has sampled reaches it any more,
since every one declares 13.

### The array mode was misread by one index - and that changed everything

The first pass at this (below, "Superseded: the macro-tile search") read
index 13 as `Thin_2DThin`, macro-tiled, and searched that formula's
configuration space to a clean negative. **That premise was wrong.**
GFD-Studio's own `TileMode.cs` enum, fetched directly from its GitHub
source rather than trusted from this project's own earlier citation of it,
lists:

```
Thin_1DThin = 0x0000000D  // "Recommended for read-only non-volume textures."
Thin_2DThin = 0x0000000E  // "Recommended for non-displayable intermediate
                           //  render targets and read/write non-volume textures."
```

Index 13 - what every real `.gnf` this project has found declares - is
`Thin_1DThin`: **micro-tiled only**, one index earlier than macro-tiled
`Thin_2DThin`. This is why the macro-tile search scored at chance: it was
searching the right hardware family for the wrong array mode entirely.

### The micro-tile formula: strong partial confirmation on these two pairs

`EgBasedLib::ComputeSurfaceAddrFromCoordMicroTiled` (Mesa's MIT `addrlib`,
`egbaddrlib.cpp`) is the formula for a 1D-tiled surface - no banks, no
pipes, no row-size constant, dramatically simpler than the macro-tiled one:
micro tiles (8x8 elements - here, 8x8 BC7 blocks, since one block is one
128-bit "element") in plain row-major order across the surface, with
`ComputePixelIndexWithinMicroTile`'s "Thin" (non-displayable) bit order
inside each tile (`Lib::ComputePixelIndexWithinMicroTile`, `addrlib1.cpp`
- `pixelBit0..5 = x0,y0,x1,y1,x2,y2`, a 6-bit interleave over the tile's
own 8x8 grid). Implemented as a probe,
`crates/texture/src/gnf/search_tests.rs::micro_tiled_address_against_the_oracle_pairs`
(`#[ignore]`d), against real oracle pairs:

- **A single-micro-tile 32x32 image
  (`Black_White_OnOff_Mask.gnf`) decodes exactly - MAD 0.00, bit-perfect.**
  This is the whole intra-tile formula validated at once: BC7 block
  decoding, the six-bit pixel index order, and the base `data_offset`, all
  correct, on real shipped bytes.
- On a 128x64, 8-mip-level pair (`Holographic_02_GLOW.gnf`, `icaras`
  team), the **entire first tile row** (4 of 4 tiles wide) decodes
  near-perfectly (MAD < 1 each). The second tile row breaks down (MAD 24.6
  on the first tile of that row, ~78 on the rest).
- On a 1024x1024, single-mip-level pair (`Harimau_c1_Livery.gnf`), the
  **first 14 consecutive on-disk micro tiles** (of 32 in that row) decode
  near-perfectly (MAD < 1 each) under plain row-major order before an
  abrupt, sustained jump to MAD 40-90.

MAD under 1 across a full 1,024-texel tile is not achievable by a wrong
decode landing on the right answer by chance - this is real, structural
confirmation of the core formula, not a coincidence.

### The corruption is not tiling: it is PSARC-level missing content

The period-2 tile-row banding this section used to report as unexplained is
now explained, by a check the earlier passes skipped: **whether the "bad"
bytes are genuinely-encoded BC7 at all**, independent of what order they are
read in. A BC7 block's mode field is unary over its first byte (`N` zero
bits then a one bit, modes 0-7) - a byte of `0x00` has no such bit and is
the spec's own reserved pattern, never emitted by a real encoder. Counting
that pattern per on-disk 32-tile row of `Harimau_c1_Livery.gnf`
(`crates/texture/examples/gnf_tile_row_byte_check.rs`) finds every odd row
(1, 3, 5, ..., 31) at an identical 1840/2048 blocks (89.8%) invalid, every
single time - too precise to be organic image content, and dense enough to
answer the question outright: **this is missing data, not a wrong tile
order.** The same file's own PSARC block table
(`gnf_tile_row_byte_check.rs`'s own block-table dump) shows the corruption
boundary is a PSARC-block property, not a tile-row one: the archive's own
64 KiB blocks (`psarc.md`'s `header.block_size`) are each exactly two
32 KiB tile-rows wide for this file, and the first invalid byte inside
each of blocks 0-3 falls a varying ~10-15 KiB in (15104, 11952, 10208,
13856), which is a property of *that PSARC block's own stored bytes*, not
of the address formula reading them in the wrong order. This is the same
"garbage"/"all-zero" population [`psarc.md`'s](psarc.md#block-data-location-and-the-short-read-extraction)
own "Block data location" section documents family-wide - landing on this
specific ship-livery texture's own copy, not a defect in the tiling math.

The 128x64 `Holographic_02_GLOW.gnf` pair's own break (row 1, MAD 24.6 then
~78) is the same signature on direct byte inspection: its row 1 (file bytes
4352-8448) is 211/256 blocks (82.4%) invalid-mode, 85.7% zero bytes outright
- dense data loss, not a scrambled read.

**None of the tile-order/shift/deinterleave hypotheses below were ever going
to explain this**, because the premise they shared - that every declared
pixel byte is real, encoded content whose correct spatial slot has not been
found yet - was the wrong premise for these two specific oracle files. Kept
as the historical record of a real, measured negative:

| Hypothesis | Result |
| --- | --- |
| Column-major tile order | No improvement (mean MAD 46.56 vs. 42.45 baseline) |
| Morton (Z-order) tile order | No improvement (45.89) |
| Vertical (Y) flip | Worse (85.02) |
| Byte-level shift, ±8 to ±512 | One partial improvement at -512B (MAD 6.62, not a clean match) |
| Half the assumed tile-row stride | Worse (57.04 whole-image MAD vs. 48.26 baseline) |
| Even/odd tile-row deinterleaving (all even rows, then all odd rows) | Worse (54.85) |
| `word6` (DCC flags) | 0 - no DCC enabled |
| `base_array_slice`/`last_array_slice` | 0/0 - not an array |
| `is_pow2_pad` | `false` |
| Stride by the descriptor's own `pitch` rather than raw width | Already what the code does - `pitch == width` exactly for both multi-tile pairs tested, so this made no difference for either |

### Root cause, confidence 90: a short `Stream.Read` in the extraction tool, not this project's reader

`lane/omega-psarc`, 2026-09-27, closes the "missing data" finding above with
*why* it is missing. **This project's own PSARC/GNF readers are not the
cause** - `docs/formats/psarc.md`'s own "Block data location" section
already establishes the block table is self-consistent and the corruption
is not file-position-dependent; this section adds the mechanism.

The discriminating step: `dd`-ing `Harimau_c1_Livery.gnf`'s bytes directly
out of the already-extracted `data03.psarc` on disk, entirely outside this
project's code, reproduces the identical "small real prefix, then zero"
pattern `gnf_tile_row_byte_check.rs` measured - so the corruption is baked
into the extracted `.psarc` file's own bytes, not introduced by
`Directory::read_entry`'s block-copy loop (a straightforward sequential
`extend_from_slice` with no stride or width bug, confirmed by reading the
80 lines of `crates/formats/src/psarc.rs` that do it). The good/bad boundary
recurs with a period of **exactly 65,536 bytes** across the whole entry -
`docs/reverse-engineering/source-images.md`'s own extraction command names
the tool: `LibOrbisPkg`'s `PkgTool.Core pkg_extract` (`maxton/LibOrbisPkg`,
the exact revision this project's own repro instructions clone).

That tool's `PFS/PFSCReader.cs::ReadSector` decompresses each PFSC sector
(also 64 KiB on this title, matching the measured period) like this:

```csharp
// slow case: compressed sector
var sectorBuf = new byte[(int)sectorSize - 2];
_accessor.Read(sectorOffset + 2, sectorBuf, 0, (int)sectorSize - 2);
using (var bufStream = new MemoryStream(sectorBuf))
using (var ds = new DeflateStream(bufStream, CompressionMode.Decompress))
{
  ds.Read(output, 0, hdr.BlockSz);
}
```

`Stream.Read` (and `DeflateStream.Read` specifically) is never guaranteed to
fill the requested count in one call even when more data is available - the
.NET docs say so explicitly, and callers are required to loop. This call
does not: it reads once, discards the return value, and leaves whatever was
already in `output` (freshly allocated, hence zero) for every byte the one
call did not produce. A genuine compressed sector therefore decodes
correctly for however many bytes that single `Read` happened to return, and
zero for the rest - exactly the measured shape, and it also explains the
**other**, previously-separate "all-zero" bucket in `psarc.md`'s census: the
sibling branch two lines up, `else { Array.Clear(output, 0, hdr.BlockSz); }`
for a sector whose declared size exceeds `BlockSz2`, zeroes the whole sector
outright. Both buckets are the same function, two branches, one
never-looped `Read`.

**Verified by patching and re-running the actual tool against the real
`.pkg`, not just by reading its source:**

1. A minimal program linking `LibOrbisPkg.Core` (the same library
   `pkg_extract` uses) opened `data/images/omega-ps4-eu.pkg`, walked to
   `Harimau_c1_Livery.gnf`'s exact byte range inside `uroot/data03.psarc`
   through the library's own `PfsReader`/`PFSCReader`, and read it directly -
   no full extraction needed, under a second per run.
2. **Unpatched, two independent runs of the identical read against the
   identical bytes returned two different truncation lengths** (14,818 and
   ~14,832 bytes of identical real content before the zero tail) - the
   signature of a short read whose exact count depends on the stream's
   internal buffering state, not of deterministic content. Deterministic
   corruption (a wrong tile order, a bad offset) cannot do this; a
   non-looped `Read` against a buffered stream can and does.
3. **Patched** (`ReadSector`'s compressed branch loops `ds.Read` until
   `hdr.BlockSz` bytes are collected or the stream is exhausted, ~9 lines),
   the same read returns all 65,536 bytes of every sector with **zero**
   invalid-mode BC7 blocks across the 4,096 sampled, and this project's own,
   completely unmodified `oag_texture::gnf::Texture::decode` turns the
   result into a fully legible ship-livery texture.
   The file as extracted before 2026-09-29 (`data/extracted/ps4.bak/omega-eu`) still
   correctly raises `Error::CorruptBlocks { count: 49899 }` on the same
   entry - the refusal this project's reader is supposed to make on
   genuinely missing content, working exactly as designed against a
   genuinely broken extraction.

Confidence 90, not higher: read from the exact pinned tool's own source
(not decompiled or guessed) and confirmed by a controlled before/after
intervention against the real archive, but short of this project's own
"Established" band, which is reserved for this project's *own* engine
claims verified against a runtime trace. The full transcript, with the exact patch diff and repro commands, was a scratch file and is not kept.

**Not fixable in this project's own code** - the defect is upstream, in a
third-party extraction tool this project depends on but does not vendor.
The fix is the tool-side patch above, applied to a `LibOrbisPkg` checkout
before running the two `pkg_extract` commands
[`source-images.md`](../reverse-engineering/source-images.md#omega-ps4-eupkg--omega-ps4-eu-patchpkg---wipeout-omega-collection-ps4)
already documents; see that page for the exact commands and the full
before/after census once a corrected extraction lands in
`data/extracted/ps4/`.

### The formula validated on clean, multi-tile-row data instead

With the two original oracle pairs both explained as partially-missing
source data, the formula was checked on files that are *not* missing
content: real front-end sprites, decoded through `Texture::decode` (the
shipped path, not a scratch probe) and read as PNG.
`Data/fe/images/wipeout_omega_logo.gnf` (894x265, 9 tile rows) decodes to
the legible "WIPEOUT" wordmark and "Omega Collection" subtitle;
`Data/fe/images/presents_finnish.gnf` (1024x64) decodes to the legible
Finnish word "esittää" ("presents") - both multi-tile-row, both spatially
coherent, both matching what the filename says they are. Numerically,
`Data/fe/images/hex_select.gnf` (128x128, 2 tile rows) against its HD
`.gtf` twin scores **whole-image MAD 1.66** - single digits, the "correct
match" band [`gxt.md`](gxt.md)'s own method uses, on a multi-tile-row image
with no missing content. This is what the "Confidence" line at the top of
this page counts.

### The refusal this project ships instead

Because a `.gnf`'s own byte range can genuinely be missing PSARC-level
content, `Texture::decode` scans the base level's own block grid for this
same invalid-mode-byte signature before decoding it, and refuses the whole
surface with `Error::CorruptBlocks { count }` the moment it finds one,
rather than decoding around missing bytes into a picture with silent black
patches standing in for them - this project's rule against inventing what
the assets do not author, applied to a decode that only *sometimes* fails
rather than one that always does. Measured on the front end's own sprite
sheet (`crates/texture/examples/gnf_frontend_census.rs`, using
`Texture::decode` directly): **219 draw, 53 refused for a corrupt base
level, 17 unsupported format** (`Bc4`/other font surfaces this module does
not decode at all) out of 289 `.gnf` files the front end and campaign
screens reference. The base-level-only scan matters: a naive whole-file
byte scan over-counts by walking every smaller mip level's own
tile-alignment padding too (`Data/fe/images/wipeout_omega_logo.gnf`'s own
894x265 base level is fully clean; its whole declared range is not, purely
from the 5 padding block-rows `ceil(67/8)*8 - 67 = 5` needed to round its
67 real block-rows up to a whole number of 8-row tiles).

### Superseded: the macro-tile search (kept as evidence of a real negative, not the live theory)

Two calibration/search passes were run against the *macro-tiled*
(`Thin_2DThin`) formula before the off-by-one above was found. Both
scored at chance - now understood to be because they were searching the
right hardware family for the wrong array mode, not because the
methodology was flawed. Kept here rather than deleted, since a documented
negative is still evidence (a future re-read of the tile mode enum, or a
`Thin_2DThin` file if one ever turns up, would want to know this ground
was already covered):

*First pass, one texture, eyeballed.* Brute-forcing five macro-tile
parameters against one small oracle-paired texture found a configuration
that decoded to a picture a human would call legible. Cross-checked
against AMD's own `EgBasedLib::SanityCheckMacroTiled` (`egbaddrlib.cpp`),
that configuration turned out to violate a real hardware constraint and
the computed address space demonstrably aliased. **A picture that looks
right is not evidence by itself** - the same lesson `gxt.md`'s own
"a smoothness metric was tried first and is too weak to use" records.

*Second pass, five oracle pairs, measured -
`crates/texture/src/gnf/search_tests.rs::search_the_reduced_tile_config_space_against_multi_size_oracle_pairs`.*
A properly bounded search over all 14 AMD `pipe_config` values, `num_banks`
in `{2,4,8,16}`, starting `bank_width`/`bank_height`/`macro_tile_aspect` in
`{1,2,4,8}` each, and a DRAM row-size constant in `{1024,2048,4096}` bytes -
8,736 configurations that pass `SanityCheckMacroTiled` after
`EgBasedLib::HwlReduceBankWidthHeight`'s per-surface bank-width/height
reduction is faithfully applied (including the pre-alignment steps
`ComputeSurfaceAlignmentsMacroTiled` runs first), scored against five
team/livery-matched oracle pairs by mean absolute per-channel difference -
the same metric `gxt.md`'s `PVRTII4BPP`/`UBC1`/`UBC3` sections use, where a
correct match reads single digits, a wrong tiling order high tens, and
chance around 60. **Best mean score: 45.21**, per-pair spread 31.58-59.11 -
uniformly "wrong" to "chance", no candidate close to a match, completed in
69 seconds (not a search that ran out of budget).

The pitch-padding evidence this search's own writeup used to argue for a
per-surface *macro-tile* reduction (33 blocks wide -> pitch 40, 65 -> 72,
15 -> 16, 526 -> 1,024, all on files declaring `TileMode(13)`) is still
real and still measured - `crates/texture/examples/gnf_pitch_sieve.rs`'s
output does not change - but the mechanism producing it is now understood
to be `Thin_1DThin`'s own (much simpler) pitch alignment, not a macro-tile
bank/pipe reduction.

## Mip chains, and handing the blocks to the GPU untouched

2026-09-30. `Texture::decode` reads the base level only, which is all a sprite
needs. A textured race wants the whole chain, and the chain is in the file.
**Confidence: 90** for the layout, which the byte counts below settle
exactly.

**Layout of a `Thin_1DThin` BC7 chain.** Levels follow one another from
`data_offset`, largest first, each padded to whole micro tiles (8x8 blocks,
1,024 bytes) and no further: a level `w` x `h` texels is
`ceil(ceil(w/4)/8) * ceil(ceil(h/4)/8) * 1024` bytes, with `w`/`h` halved (to a
floor of 1) per level, and there are `last_mip_level - base_mip_level + 1` of
them. `crates/texture/examples/gnf_mip_layout_probe.rs` sums that against the
bytes each file holds past its header: **15,413 of the 15,525 BC7
`TileMode(13)` files across all nine archives match to the byte.** The other
112 are the files that carry more than one surface - cubemaps (`skyCube`,
`feenvmap_cube`), arrays, and a few front-end images with trailing bytes - and
they differ by whole extra chains, which is how they were told apart. Each
level untiles with the same micro-tile formula the base level does.

`Texture::block_levels` returns every level as linear BC7 blocks, still
compressed, and **refuses** anything its layout does not account for
(`Error::ChainLayout`) or a corrupt block in *any* level
(`Error::CorruptBlocks`, the base level's own guard extended down the chain,
since a chain with a hole in it samples garbage at distance). On the
corpus that is 0 corrupt chains among the 2D ones (the 55 `data08` and
handful of base-archive refusals the first pass counted were cubemaps read as
2D).
`decode_bc7_level` decodes one level to RGBA8 for a GPU without block
compression.

**What a renderer does with it.** `oag_render::ModelTexture::from_gnf` keeps a
chain of two or more levels on a block-aligned base as `Texels::Blocks` with
`BlockFormat::Bc7` and uploads it as `Bc7RgbaUnorm`, the way HD's `.gtf` DXT
chains already went. A single-level texture, or one with a base off the 4x4
grid, still decodes to RGBA8 and gets the renderer's own box-filtered chain.
One byte a texel on the CPU and on the GPU where the decoded picture is four,
with the disc's chain in place of a synthesised one.

**The one pixel difference this makes**, measured rather than assumed:
`crates/texture/tests/omega_gnf_mips_ground_truth.rs` (`#[ignore]`d) decodes
level 1 of every chain and compares it with a 2x2 box filter of the decoded
base - what the renderer used to synthesise. Mean absolute difference per
channel, in 8-bit units, over the archives: 1.04 (`data08`) to 2.75
(`data03`) over the six archives that hold BC7 chains, worst single texture 28. The base level is bit-identical to
`decode`'s (asserted). A Tech De Ra race still, RGBA8 path against blocks
path, differs by a mean 0.88 of 255 with 0.7% of pixels over 16, at
high-contrast edges: the authored mips, plus the GPU's own BC7 decode of the
base. Both are the hardware's own doing, which is what the original ran.

## Implemented where

`oag_texture::gnf`, since 2026-09-16; `Texture::decode` and the BC7 block
decoder since 2026-09-21; the micro-tile untiler and the `CorruptBlocks`
refusal since 2026-09-27. `Texture::parse` reads the header, contents and
first descriptor; `Texture::decode` untiles and decodes a **linear** surface
unconditionally, and a `Thin_1DThin` (`TileMode(13)`) BC7 surface whenever
its base level carries no block with an invalid mode byte - see "Tiling"
above for the evidence and the refusal's own reasoning.
`crates/texture/tests/omega_gnf_ground_truth.rs` (`#[ignore]`d, needs
`data/extracted/ps4/omega-eu/uroot/`) parses every real `.gnf` across all
five base archives without a panic, and pins the worked example above to its
exact decoded fields. `crates/texture/tests/omega_gnf_pixels_ground_truth.rs`
(`#[ignore]`d, needs the patch archives too) runs `decode` over the whole
corpus - all nine archives, `data08`'s `Data/fe/` subtree swept in full -
asserting it never panics, that `data08`'s own `Data/fe/` subtree decodes at
least one real image, and that a still-refused `Error::Tiled` names the
exact `tile_mode` the descriptor declares.
`crates/texture/src/gnf/micro_tile_tests.rs` and `search_tests.rs`
(`#[ignore]`d) hold the tiling-configuration searches this page's "Tiling"
section reports: the superseded macro-tile one, and the micro-tile probe
whose diagnostics `gnf_tile_row_byte_check.rs` (a plain example, public-API
only) extended to close the periodic-corruption question.
`crates/texture/examples/gnf_frontend_census.rs` reports the front end's own
draws/refused/unsupported split directly off `Texture::decode`, and
`crates/hud/src/sprite.rs::Image::decode_gnf` plus
`crates/game/src/boot/sprites.rs::gnf_sibling` (reused by
`crates/game/src/campaign.rs::load_omega`) are what actually wire a decoded
`.gnf` into the running front end - see `docs/formats/omega-status.md`.

## See also

- [Format index](README.md)
- [PSARC](psarc.md) - the container these files live inside, and the
  real/zero split that governs which `.gnf` entries have real bytes at all
- [GXT](gxt.md) - the cross-title oracle method this page's BC7/HD comparison
  reuses, and the "a smoothness metric was tried first and is too weak to
  use" precedent the disproven calibration pass above ran into
- [GTF](gtf.md) - the PS3 sibling container the oracle's `.gtf` half decodes
  through
