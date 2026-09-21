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
Red/Green/Blue/Alpha order, and its tile mode decodes to `Thin_2DThin` - a
genuinely tiled mode, not a coincidence a wrong reading could produce by
accident.

## Census: every valid `.gnf` across all nine Omega archives

`crates/texture/examples/gnf_census.rs` parses every entry ending `.gnf` in
the five base (`omega-eu`) and four patch (`omega-eu-patch`) archives - 2,670
entries examined, 1,407 with a valid `GNF ` header and a sane descriptor (the
rest are the `psarc.md`-measured "garbage"/"all-zero" population, a property
of this local dump rather than of the reader - see that page's "Block data
location" section):

| `SurfaceFormat` | `TileMode` | Count | Size range | Archives | Example |
| --- | --- | ---: | --- | --- | --- |
| `Bc7` | `0x0d` (`Thin_2DThin`) | 1,378 | 2x1 .. 8192x8192 | all nine | `Data/fe/images/badges.2x.gnf` |
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

## Tiling: why `Thin_2DThin` is not untiled

**Every one of the 1,407 valid `.gnf` entries this census found declares
`TileMode(0x0d)`.** `Texture::decode` refuses it by name
(`Error::Tiled { tile_mode: 13 }`) rather than guessing at a picture - the
same rule this project applied when the header/descriptor work first landed,
now backed by a specific, measured reason it still applies.

**The address algorithm itself is public and was read**: AMD's GCN macro-tile
addressing (`ComputeSurfaceAddrFromCoordMacroTiled`,
`ComputePixelIndexWithinMicroTile`, `ComputePipeFromCoord`,
`ComputeBankFromCoord`) is documented in Mesa's MIT-licensed `addrlib`,
specifically `src/amd/addrlib/src/r800/{egbaddrlib,siaddrlib}.cpp` - the
GFX6/SI-generation chip family PS4's GCN 1.1 "Liverpool" GPU shares its
tiling generation with. Not shadPS4's (GPL) code - `siaddrlib.cpp`'s
`ComputePipeFromCoord` and `egbaddrlib.cpp`'s `ComputeBankFromCoord`,
`ComputeSurfaceAddrFromCoordMacroTiled` and `ComputePixelIndexWithinMicroTile`
were read and are cited here for provenance; shadPS4 was not consulted at
all for this pass.

**What the algorithm needs that a single `TileMode` index does not supply**:
five parameters (`pipe_config`, `bank_width`, `bank_height`, `num_banks`,
`macro_tile_aspect`) that a naive reading would expect to be a fixed row in a
32-entry table, the way desktop GCN's own `GB_TILE_MODE0..31` registers work.
They are not, for this format. `EgBasedLib::HwlReduceBankWidthHeight`
(`egbaddrlib.cpp`) **reduces `bank_width`, `bank_height` and
`macro_tile_aspect` per surface**, from `tile_size * bank_width * bank_height
<= m_row_size` where `tile_size` depends on the surface's own
bits-per-element and `m_row_size` is a DRAM row-size hardware constant this
project has not recovered. A single `TileMode` index is only the *starting*
(unreduced) configuration; the reduction is what a real driver applies before
ever computing an address.

**Measured, not assumed**: `crates/texture/examples/gnf_pitch_sieve.rs`
sweeps every `Bc7`/`TileMode(13)` entry's own `pitch` field (which a real
driver pads up to whichever `macro_tile_pitch` survives the reduction) against
its `width`. 1,330 of 1,378 declare `pitch == width` (naturally aligned,
uninformative), but the 48 that pad show the reduction is real and
*surface-dependent*:

| Width (blocks) | Padded pitch (blocks) | File |
| ---: | ---: | --- |
| 33 | 40 | `Data/fe/NewImages/medals/Icon_Pass_medal_medium.gnf` |
| 65 | 72 | `Data/fe/images/detonator.gnf` |
| 15 | 16 | `Data/fe/images/infinity_symbol.gnf` |
| 526 | 1,024 | `Data/crowd/Textures/crowd_rig_sprite_N.gnf` |

The first three round up to the next multiple of 8 - consistent with a fully
reduced (`bank_width = 1`, small `macro_tile_aspect`) configuration on a
small surface. The fourth rounds to a multiple of 512 - consistent with a
much larger, unreduced (or less-reduced) macro tile pitch on a surface big
enough that the row-size constraint never bites. **Both are `TileMode(13)`.**
A fixed five-parameter lookup cannot produce two different `macro_tile_pitch`
values for the same `TileMode`; the per-surface reduction algorithm is the
only explanation this project has found that fits both rows, and it has not
been reimplemented.

**A calibration pass was attempted and its result was disproven, which is
worth recording as a trap for the next contributor rather than erasing.**
Brute-forcing the five parameters against one small oracle-paired texture
(the `Holographic_02_GLOW.gnf` worked example above) found a configuration
that decoded to a picture a human would call legible - a recognisable decal
pattern, not noise. Cross-checked against AMD's own
`EgBasedLib::SanityCheckMacroTiled` (`egbaddrlib.cpp`), that same
configuration turned out to violate a real hardware constraint
(`banks >= macroAspectRatio`, "this will generate macro tile height <= 1"
otherwise) badly enough that `macro_tile_bytes` truncates to zero and the
address space demonstrably aliases - at most half the block positions are
distinct. The "legible picture" was two mostly-white, sparse-line-art images
compared against each other, which is a weak control this project's own
`gxt.md` warns about by name ("a smoothness metric was tried first and is too
weak to use"); it was not a match. **A picture that looks right is not
evidence by itself** - this is the concrete instance of that rule, not just
the abstract one.

**What would close this**: the true `pipe_config` (a global GPU constant,
plausibly recoverable from a PS4 devkit's `GB_ADDR_CONFIG` register or a
leaked/documented `libSceGnm` tile-mode table this project does not hold) and
`m_row_size` (a DRAM row-size constant), from which
`HwlReduceBankWidthHeight`'s reduction is deterministic and checkable against
the pitch-padding evidence above across all 48 padded samples, not just one.
Short of that, a real PS4 or an accurate GCN-generation emulator to render a
known texture and compare would settle it directly, the way `docs/formats/hd-frontend.md`'s
own boot chain was settled by three cold boots on RPCS3.

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
`tile_mode` the descriptor declares.

## See also

- [Format index](README.md)
- [PSARC](psarc.md) - the container these files live inside, and the
  real/zero split that governs which `.gnf` entries have real bytes at all
- [GXT](gxt.md) - the cross-title oracle method this page's BC7/HD comparison
  reuses, and the "a smoothness metric was tried first and is too weak to
  use" precedent the disproven calibration pass above ran into
- [GTF](gtf.md) - the PS3 sibling container the oracle's `.gtf` half decodes
  through
