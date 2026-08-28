# GXT: the Vita's texture container

**Status: `UBC2` decodes at confidence 85, `PVRTII4BPP` at 92 - together
88.9% of the corpus.** `oag_formats::gxt` for the container,
`oag_formats::pvrtc` for the PowerVR codec. Measured on
`data/extracted/vita/PCSF00007` (Wipeout 2048, EU, patch v1.04), 2026-08-26
and 2026-08-27.

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
| `0x83` | `PVRTII4BPP` | `0x83000000` | 8,430 | **yes** |
| `0x86` | `UBC2` (BC2/`DXT23`) | `0x86000000` | 370 | **yes** |
| `0x85` | `UBC1` (BC1/`DXT1`) | `0x85000000` | 505 | no |
| `0x87` | `UBC3` (BC3/`DXT45`) | `0x87000000` | 493 | no |
| `0x0c` | `U8U8U8U8` | `0x0c001000` | 99 | no |
| `0x98` | `U8U8U8` | `0x98001000` | 13 | no |

`UBC2` decodes through `oag_formats::bcn::dxt23` - the same BC2 block math
[`.gtf`](gtf.md) uses, moved into a shared `oag_formats::bcn` module on
2026-08-26 since the block layout is a hardware standard rather than something
either console's container defines. `PVRTII4BPP` decodes through
`oag_formats::pvrtc`, added 2026-08-27; the section below is its evidence.
`UBC1`/`UBC3` are the same BC family as `UBC2` and would each be a small
addition to `bcn`'s existing `dxt1`/`dxt45` if a texture reaching them needed
one; none measured here does. **The last two rows' raw `format` carries
non-zero low bits** (`0x001000`) where every `UBC2`/`PVRTC` row's is clean -
this module reads only the top byte, so what that low-bit pattern means (a
swizzle/channel-order variant) is unread rather than folded into the format
name.

**`0x0c` was mislabeled `U4U4U4U4` until 2026-08-28.** Corroborated against
the public `vitasdk` headers (`psp2/gxm.h`, `SceGxmTextureBaseFormat`) two
ways rather than one: the enum's own ordering (`U8` at `0x00`, `S8` at `0x01`,
`U4U4U4U4` at **`0x02`**, ..., `U8U8U8U8` at **`0x0c`**), and the raw `format`
these files carry, `0x0c001000`, decomposing exactly as
`SCE_GXM_TEXTURE_BASE_FORMAT_U8U8U8U8 | SCE_GXM_TEXTURE_SWIZZLE4_ARGB` -
`0x0c000000 | 0x00001000`. The low three bytes are not noise, then: they are
`SceGxmTextureSwizzle4Mode`, and this title's whole corpus at this format byte
carries the *same* swizzle (`ARGB`, `0x001000`), not a mix. **Still not
decoded, and for a reason beyond the channel order**: `U8U8U8U8` is simple
math - four bytes a texel, no block or bit-packing - but whether those four
bytes read in raster or the same Morton/twiddle order [`blocks`] uses for
`UBC2` is unset by anything in the file (`type` at descriptor `+0x10` is `0`
on the `ZoneMode2048`/`ZoneMode2048Track` textures measured, the same
uninformative value the `UBC2` reticle texture carried before that question
was settled by decoding both ways and looking at the picture). Settling both
questions the way the reticle atlas or `.gtf`'s "ASSEGAI DEVELOPMENTS" text
settled theirs needs either a `U8U8U8U8` file with less ambiguous art than
this format byte's own corpus has offered so far, or the same
both-ways-and-look method applied carefully - a wrong tiling order can
produce a plausible-looking wrong picture on abstract art the way it briefly
did on the reticle before the twiddle order was found.

## `PVRTII4BPP`: PowerVR texture compression, and not the PVRTC-I lookalike

**Confidence 92.** This is 85% of every `.gxt` on the disc and effectively
every texture a `.rcsmodel` material names, so until 2026-08-27
[2048-status.md](2048-status.md)'s "the circuit and the craft draw,
untextured" had two independent causes and this was one of them. See
`crates/formats/src/pvrtc.rs`, whose module doc carries the bit layout.

**It is not a block codec, and treating it as one produces a picture that is
wrong rather than obviously broken.** A word is 8 bytes over 4x4 texels but
stores only *two* colours plus sixteen 2-bit modulation values: those colours
are a low-frequency image sampled once per word, and every output texel
bilinearly interpolates the four words around it before blending by its own
modulation. Words are stored in the same Morton order the `UBC2` block grid
uses, applied **once**, inside the codec.

**PVRTC-II differs from PVRTC-I in three ways that all matter here**, and the
public PowerVR SDK decompressor implements PVRTC-I only:

1. **One opacity flag, not two.** PVRTC-I gives colour A its own opaque bit at
   bit 15; PVRTC-II spends bit 15 on a hard-transition flag and lets bit 31
   answer for both colours.
2. **A hard-transition mode**, where the central 4x4 texels of a word quad
   stop interpolating and take one word's colours flat. **Not a corner case
   here: 4,802,259 of 67,683,840 words (7.1%) set that bit**, so a PVRTC-I
   decoder would be wrong on 7% of this corpus's words, not merely imprecise.
3. **Colour B's low alpha bit is forced to 1** in transparent mode.

The bit layout and arithmetic are ported from **Vita3K**'s
`vita3k/renderer/src/texture/pvrt-dec.cpp`, whose PVRTC-II path that project
credits to its own team as an addition to Imagination's PVRTC-I decompressor -
an emulator that runs real Vita titles through it, which is the same class of
external corroboration the twiddle finding below rests on. The 92 is not the
reference's, though; it is what was measured here against it.

### Wipeout HD decodes the same art, and this agrees with it

**The strongest evidence this decode has**, and it exists because 2048's DLC
re-ships HD/Fury's circuits and its whole fourteen-team roster: 2,284 textures
exist twice, as a `.gtf` on the PS3 disc in a BC format
[`oag_formats::gtf`](gtf.md) decoded long before this, and as a `PVRTII4BPP`
`.gxt` here. The same HD-as-ground-truth method that settled the `WO Track`
point tail, 2048's vertex normal and its `Uv1`
([2048-rcsmodel.md](2048-rcsmodel.md)).

Two lossy codecs never agree bit for bit on one source image, so the argument
is a comparison of comparisons and the controls carry it. Mean absolute
per-channel difference out of 255, **median over 2,284 pairs**:

| Compared against Wipeout HD's own decode of the same art | Median |
| --- | ---: |
| this decode | **3.83** |
| this decode, with HD's copy flipped vertically | 10.01 |
| the same payload read in **raster** word order | 34.60 |
| a different texture of the same size (chance level) | 59.74 |

The flipped row is what says the Vita's rows are top-down where the PS3's are
bottom-up - measured, not assumed. The raster row is the wrong answer this
codec is most likely to give, and it lands nine times further away. About a
quarter of the pairs are genuinely different art sharing a basename (several
circuits each ship their own `billboard3`), which is why the median is the
statistic and not the mean.

### A font atlas is the picture check, because a smoothness metric is not

`RussianHud.gxt` (1024x1024) renders the full Latin and Cyrillic alphabets,
crisp, right way up, over a twiddled surface - see `data/shots/`. Letterforms
survive none of a wrong word order, a wrong bit layout or a wrong
transposition. The ground-truth test asserts what a font atlas *is*: **100.0%
of its texels are monochrome** (the three channels come out of three different
bit fields in both colour modes, so a wrong layout tints them), 82.2% have
alpha at one extreme or the other, and 63.9% are fully clear.

**A smoothness metric was tried first and is too weak to use**, which is worth
recording: an untwiddled control scores only 1.4x rougher than the real decode
(mean adjacent-texel delta 9.60 against 6.65), because PVRTC's bilinear
upscale smooths the wrong answer too. Contrast `.gtf`'s own endianness
question, where the same style of metric settled it outright.

### The length arithmetic closes, once a level is floored at 16 bytes

`Gxt::parse` only length-checks a texture whose unit size this crate knows, so
teaching it `PVRTII4BPP` put 8,430 more textures in this package (10,204
across all three) under the same check - and they only pass because of one
measured rule: **a mip level occupies at least 16 bytes.**

| Per-level formula | Closes on |
| --- | ---: |
| `ceil(w/4) * ceil(h/4) * 8` | 2,923 of 10,204 |
| the same, floored at 16 bytes | **10,204 of 10,204** |

The shortfall is exactly one word, on exactly those chains that bottom out at
a single-word (4x4) level; every level with two words or more is stored at the
plain figure. Free for `UBC2`, whose one block is already 16 bytes.
`oag_formats::gxt::MIN_LEVEL_LEN` carries it.

### What is implemented but unexercised

The local-palette path (`+30`), which needs the hard-transition bit *and*
modulation mode 1 on the same quad. **No texel in the base package reaches it -
0 of 1,082,941,440 measured.** It is ported from the reference anyway, but it
has no evidence either way, and the reference's own index into its palette
table is transposed relative to how that table reads. That is what holds this
at 92 rather than higher.

Also unread: 35 of the 10,204 textures have a base level below 8 texels in one
dimension, which is below the smallest surface this codec can address at all.
They decode at the 8x8 minimum, with absent words read as zero, and are
cropped back - matching what the reference does modulo its own out-of-bounds
read.

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
