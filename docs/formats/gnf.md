# GNF: the PS4's native texture container

**Confidence: 80.** A genuine Sony SDK format ("Gnm Format") with no
first-party spec this project holds, so the layout is triangulated from two
independent, non-affiliated open-source implementations and one public AMD
hardware reference rather than read off an SDK header. See
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

## What is not decoded

**No pixel.** `SurfaceFormat::Bc1`/`Bc3`/`Bc7` are exactly the block formats
[`oag_texture::bcn`](../../crates/texture/src/bcn.rs) already decodes for PS3
`.gtf` and Vita `.gxt`, but only for a **linear** surface; every real `.gnf`
sampled so far declares a genuinely tiled `TileMode` (`Thin_2DThin` on the
worked example above). Untiling a GCN surface correctly needs the
macro/micro tile, pipe-config and bank-swizzle table `shadPS4`'s own
`tiling.h` spends 32 `TileMode` entries and several supporting enums on -
reimplementing that from two secondary sources with no real PS4 to render
against and check the picture on is exactly the "stand-in that reads as
legible" this project's rule against inventing what the assets already
author exists to prevent. `Texture::is_linear` names the two `TileMode`
values (`Display_LinearAligned`, `Display_LinearGeneral`) that would make
this safe; none has been found among the entries `psarc.md` classifies
"valid" so far.

## Implemented where

`oag_texture::gnf`, since 2026-09-16. `Texture::parse` reads the header,
contents and first descriptor; `crates/texture/tests/omega_gnf_ground_truth.rs`
(`#[ignore]`d, needs `data/extracted/ps4/omega-eu/uroot/`) parses every real
`.gnf` `psarc.md`'s reader can name across all five base archives without a
panic, and pins the worked example above to its exact decoded fields.

## See also

- [Format index](README.md)
- [PSARC](psarc.md) - the container these files live inside, and the
  real/zero split that governs which `.gnf` entries have real bytes at all
