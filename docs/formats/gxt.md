# GXT: the Vita's texture container

**Status: `UBC2` decodes, confidence 85.** `oag_formats::gxt`. Measured on
`data/extracted/vita/PCSF00007` (Wipeout 2048, EU, patch v1.04, base
package), 2026-08-26.

```sh
OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-formats --run-ignored all \
    -E 'binary(gxt_ground_truth)'
```

## The container

```text
+0x00  u32   tag                  "GXT\0"
+0x04  u32   version              0x10000003 on every file measured
+0x08  u32   texture count
+0x0c  u32   texel data offset, from the start of the file
+0x10  u32   texel data length
+0x14  u32   16-entry (P4) palette count
+0x18  u32   256-entry (P8) palette count
+0x1c  u32   padding
then `count` descriptors, 32 bytes each:
+0x00  u32   texel data offset, from the start of the file
+0x04  u32   texel data length
+0x08  u32   palette index, or 0xffffffff for none
+0x0c  u32   flags
+0x10  u32   type
+0x14  u32   format               `SceGxmTextureBaseFormat`, top byte only
+0x18  u16   width
+0x1a  u16   height
+0x1c  u8    mip levels           at least 1
+0x1d  u8[3] padding
```

**Little-endian throughout**, unlike [`.gtf`](gtf.md)'s big-endian PS3
descriptor - this title's whole package is little-endian, per
[2048-status.md](2048-status.md). There is no `pitch` field: a mip chain is
tightly packed.

Corroborated against the public `vitasdk`/`vita-toolchain` headers
(`psp2/gxt.h`, `SceGxtHeader`/`SceGxtTextureInfo`), the same kind of external
check `.gtf`'s own header rests on for `CellGcmTexture`. Every field was also
independently re-derived from the bytes: on
`Data\XML\2048_hud\Texture\hud_2048.gxt` (524,352 bytes), `dataOffset` (0x40)
+ `dataSize` (0x80000) is the file length exactly, and the descriptor's own
`dataOffset`/`dataSize` repeat the same two numbers.

## The format byte names a `SceGxmTextureBaseFormat`

The low three bytes of `format` carry swizzle/channel-order bits this module
does not act on; only the top byte is read. Swept across all 9,910 `.gxt`
files the base package ships:

| Format byte (top byte of `format`) | `SceGxmTextureBaseFormat` | Raw `format` seen | Count | Decoded |
| --- | --- | --- | ---: | :-: |
| `0x86` | `UBC2` (BC2/`DXT23`) | `0x86000000` | 370 | **yes** |
| `0x83` | `PVRTII4BPP` | `0x83000000` | 8,430 | no |
| `0x85` | `UBC1` (BC1/`DXT1`) | `0x85000000` | 505 | no |
| `0x87` | `UBC3` (BC3/`DXT45`) | `0x87000000` | 493 | no |
| `0x0c` | uncompressed (`U4U4U4U4` range) | `0x0c001000` | 99 | no |
| `0x98` | `U8U8U8` | `0x98001000` | 13 | no |

**Only `UBC2` decodes**, through `oag_formats::bcn::dxt23` - the same BC2
block math [`.gtf`](gtf.md) uses, moved into a shared `oag_formats::bcn`
module on this pass since the block layout is a hardware standard rather than
something either console's container defines. Everything else is refused
rather than guessed at: `PVRTII4BPP` (8,430 files, the large majority of the
corpus) is PowerVR texture compression, a wholly different codec family
- bilinear-upscaled low/high-frequency images plus a modulation layer, not a
per-4x4-block palette. `UBC1`/`UBC3` are the same BC family as `UBC2` and
would each be a small addition to `bcn`'s existing `dxt1`/`dxt45` if a texture
reaching them needed one; none measured here does. **The last two rows' raw
`format` carries non-zero low bits** (`0x001000`) where every `UBC2`/`PVRTC`
row's is clean - this module reads only the top byte, so what that low-bit
pattern means (a swizzle/channel-order variant, on the doc comment above's own
account of what the low three bytes hold) is unread rather than folded into
the format name.

**This costs nothing for the HUD, and a great deal for everything else.**
Every texture [`oag_2048::hud::LAYOUTS`]'s played skin (`2048_hud\`) reaches
is `UBC2`. But the corpus this sweep covers is not HUD-only: every environment
and ship texture measured (460 files under `environments/altima` and
`Ships/`, `.gxt` alongside their own `.rcsmodel`/`.rcsmaterial`) is also a
`.gxt`, and `PVRTII4BPP` at 8,430 of 9,910 files is almost certainly most of
them - not measured directly per-file, but the proportion makes the
alternative implausible. [2048-status.md](2048-status.md)'s "the circuit and
the craft draw, untextured" is not only a `.rcsmodel` texture-coordinate gap,
then: even with coordinates placed, most of what they would sample is a
codec this module refuses. `PVRTII4BPP` is the highest-value format left
here, not a footnote.

## The block grid is twiddled, not raster - measured, not assumed

**This is the load-bearing finding.** A naive raster-order block read of
`missile_reticule.gxt` (256x256, one `UBC2` level) decodes to noise, with one
anomalous horizontal band that reads clean - the signature of a swizzle that
happens to agree with raster order along one axis. There is no bit in the
descriptor this module reads that states the layout (`type`, `+0x10`, was 0
on every texture measured, and nothing here has proven what value would mean
"raster") - unlike `.gtf`, whose `is_linear` bit states exactly this. So it
was settled the way `.gtf`'s own endianness question was: decode both ways
and look at the picture.

Reading the block grid in **Morton (Z-order) order** instead decodes cleanly:
`missile_reticule.gxt` renders four recognisable lock-on reticle pieces (a
dashed bracket ring, a filled circle backdrop, a thin dashed ring, a
crosshair with a dashed arc) and `hud_2048.gxt` (1024x512, the non-square
case) renders a full, legible sprite sheet - weapon pickup icons, position
chevrons, a warning triangle, a speed-bar gradient ellipse, all right way up.
See `crates/formats/tests/gxt_ground_truth.rs`, which is where those renders
come from and re-runs on demand.

**Corroborated externally**: `ClassiCube`'s own Vita port computes the same
even/odd bit-interleave masks (`TwiddleCalcFactors`) to write
`sceGxmTextureInitSwizzled` textures - this is Sony's documented hardware
tiling scheme, not a guess that happened to render something.

`oag_formats::gxt::twiddle` implements the general (non-square) algorithm:
`bx` bits in the odd positions and `by` in the even ones while both
dimensions have more than one step left, then the remaining bits of whichever
dimension is larger are appended **linearly** once the smaller one is
exhausted - `hud_2048.gxt`'s 256x128 block grid is what forced the general
form over a plain full bit-interleave, which only agrees with it for a square
grid like `missile_reticule.gxt`'s 64x64.

**Not flipped vertically**, unlike `.gtf`'s bottom-up PS3 rows: both renders
above came out the right way up as decoded, so the Vita's GXM convention is
top-down. Confidence 85 rather than higher for the same reason `.gtf`'s
picture-based checks are not the ceiling: no sprite has been compared against
a running frame of the original, only against "does this look like authored
art" - see [2048-hud.md](2048-hud.md#what-is-not-done) for what that still
leaves open.

## Wired into the sprite loader

`oag_game::sprite::Image::decode` tries `.gxt` as a fourth branch alongside
the PSP `.mip`, PS2 and PS3 `.gtf` readers, so a HUD sprite reference that
resolves to a shipped `.gxt` entry decodes to real pixels through the same
`Sheet` every other title's HUD uses. See
[2048-hud.md](2048-hud.md#what-is-not-done) for why nothing draws in a race
yet regardless.

## See also

- [gtf](gtf.md) - the PS3 sibling this module's doc comments compare against
  throughout, and the source of the shared `oag_formats::bcn` block math
- [2048-hud](2048-hud.md) - what this format unblocks and what it does not
- [2048-status](2048-status.md) - where this fits in the wider probe
