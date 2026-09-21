# GNF: the PS4's native texture container

**Confidence: 80 for the container and descriptor, 75 for the BC7 block
decoder, unscored for tiling (unimplemented).** A genuine Sony SDK format
("Gnm Format") with no first-party spec this project holds, so the layout is
triangulated from two independent, non-affiliated open-source implementations
and one public AMD hardware reference rather than read off an SDK header. See
[`psarc.md`](psarc.md#the-check-the-confidence-rests-on) for why this project
scores a publicly-documented-but-not-recovered-here container in the 80s-90s
rather than treating "publicly documented" as a free pass to 100.

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

## Tiling: why `Thin_1DThin` is not untiled

**Every one of the 1,407 valid `.gnf` entries this census found declares
`TileMode(0x0d)`.** `Texture::decode` refuses it by name
(`Error::Tiled { tile_mode: 13 }`) rather than guessing at a picture - the
same rule this project applied when the header/descriptor work first landed,
now backed by a specific, measured reason it still applies.

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

### The micro-tile formula: strong partial confirmation, not yet a full match

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

### The unexplained part: a precise, periodic corruption - confirmed to be tiling, not content

The 1024x1024 pair's break does **not** land on a tile-row boundary (its
row is 32 tiles wide; the break is at tile 14), so it is a different
symptom from the 128x64 pair's break (which lands exactly at its own
4-tile row boundary). Per-tile-row mean brightness across the whole
1024x1024 image (`data/scratch/drive-2026-09-21/gnf/harimau-diag/`) shows
a precise, sustained **period-2 tile-row alternation**: rows 0, 2, 4, ...,
30 average brightness 7-16 (real content), rows 1, 3, 5, ..., 31 average
0.2-2.6 (near-black - either all-zero bytes, or bytes this project's own
`bc7()` happens to decode near-black).

**Ruled out by directly comparing the decoded and oracle images
side by side** (the content-vs-tiling test this page's own method
elsewhere and `gxt.md`'s precedent both call for): the pattern is
horizontal banding with a sharp, regular period, not a spatially coherent
shape - a genuine content difference (a remaster redrawing this ship's
sponsor decal) would show as a logo-shaped region, not an every-other-row
stripe. This is a tiling artifact.

**Hypotheses tested and ruled out, all measured**:

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

None explain the period-2 pattern. It remains open.

### What would close this

A real PS4, an accurate GCN-generation emulator, or a leaked/documented
`libSceGnm` source for `ComputeSurfaceAddrFromCoordMicroTiled`'s exact
tile-row indexing on this hardware, to render a known texture and compare
directly - the way `docs/formats/hd-frontend.md`'s own boot chain was
settled by three cold boots on RPCS3. Absent that, the next empirical step
is sweeping the period-2 pattern's phase/parity against more oracle pairs
of varying tile-grid width and height (odd vs. even `tiles_x`/`tiles_y`) to
see whether the alternation is keyed to a coordinate parity this project's
own `Texture` struct already decodes (row parity, `pitch` parity, mip
count parity) rather than to an unrecovered hardware constant.

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

## Implemented where

`oag_texture::gnf`, since 2026-09-16; `Texture::decode` and the BC7 block
decoder since 2026-09-21. `Texture::parse` reads the header, contents and
first descriptor; `Texture::decode` untiles and decodes a **linear** surface
only, returning `Error::Tiled` for every real sample (see above).
`crates/texture/tests/omega_gnf_ground_truth.rs` (`#[ignore]`d, needs
`data/extracted/ps4/omega-eu/uroot/`) parses every real `.gnf` across all
five base archives without a panic, and pins the worked example above to its
exact decoded fields. `crates/texture/tests/omega_gnf_pixels_ground_truth.rs`
(`#[ignore]`d, needs the patch archives too) runs `decode` over the whole
corpus - all nine archives, `data08`'s `Data/fe/` subtree swept in full -
asserting it never panics and that every `Error::Tiled` names the exact
`tile_mode` the descriptor declares. `crates/texture/src/gnf/search_tests.rs`
(`#[ignore]`d) holds both tiling-configuration searches this page's "Tiling"
section reports: the superseded macro-tile one, and the micro-tile one with
its still-open periodic-corruption diagnostics.

## See also

- [Format index](README.md)
- [PSARC](psarc.md) - the container these files live inside, and the
  real/zero split that governs which `.gnf` entries have real bytes at all
- [GXT](gxt.md) - the cross-title oracle method this page's BC7/HD comparison
  reuses, and the "a smoothness metric was tried first and is too weak to
  use" precedent the disproven calibration pass above ran into
- [GTF](gtf.md) - the PS3 sibling container the oracle's `.gtf` half decodes
  through
