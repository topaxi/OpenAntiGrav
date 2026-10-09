# GXT: the Vita's texture container

**Status: understood - all six format bytes the corpus carries decode.**
370 + 8,430 + 99 + 505 + 493 + 13 = 9,910 textures, one per file (every
`.gxt` this corpus ships carries exactly one), so that is also 9,910 of
9,910 files. `UBC2` at confidence 85, `PVRTII4BPP` at 92, `U8U8U8U8` at 80,
`UBC1`/`UBC3` at 88, `U8U8U8` at 70 for its tiling and packing - its channel
order is **chosen, not measured** (no score; see its own section below for
why the corpus cannot settle it). `oag_texture::gxt` for the container,
`oag_texture::pvrtc` for the PowerVR codec, `oag_texture::bcn` for the
shared BC1-3 block math. Measured on `data/extracted/vita/PCSF00007`
(Wipeout 2048, EU, patch v1.04), 2026-08-26, 2026-08-27, 2026-08-28 and
2026-09-16.

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
| `0x85` | `UBC1` (BC1/`DXT1`) | `0x85000000` | 505 | **yes** |
| `0x87` | `UBC3` (BC3/`DXT45`) | `0x87000000` | 493 | **yes** |
| `0x0c` | `U8U8U8U8` | `0x0c001000` | 99 | **yes** |
| `0x98` | `U8U8U8` | `0x98001000` | 13 | **yes** |

`UBC2` decodes through `oag_texture::bcn::dxt23` - the same BC2 block math
[`.gtf`](gtf.md) uses, moved into a shared `oag_texture::bcn` module on
2026-08-26 since the block layout is a hardware standard rather than something
either console's container defines. `PVRTII4BPP` decodes through
`oag_texture::pvrtc`, added 2026-08-27; the section below is its evidence.
`UBC1`/`UBC3` decode through `bcn::dxt1`/`dxt45`, added 2026-09-16 - see
"`UBC1`/`UBC3` decode too" below. **The last two rows' raw `format` carries
non-zero low bits** (`0x001000`) where every `UBC2`/`PVRTC` row's is clean -
this module reads only the top byte, so what that low-bit pattern means (a
swizzle/channel-order variant) is unread rather than folded into the format
name, except where `U8U8U8U8`'s own decode below reads it directly.

**`0x0c` was mislabeled `U4U4U4U4` until 2026-08-28.** Corroborated against
the public `vitasdk` headers (`psp2/gxm.h`, `SceGxmTextureBaseFormat`) two
ways rather than one: the enum's own ordering (`U8` at `0x00`, `S8` at `0x01`,
`U4U4U4U4` at **`0x02`**, ..., `U8U8U8U8` at **`0x0c`**), and the raw `format`
these files carry, `0x0c001000`, decomposing exactly as
`SCE_GXM_TEXTURE_BASE_FORMAT_U8U8U8U8 | SCE_GXM_TEXTURE_SWIZZLE4_ARGB` -
`0x0c000000 | 0x00001000`. The low three bytes are not noise, then: they are
`SceGxmTextureSwizzle4Mode`, and this title's whole corpus at this format byte
carries the *same* swizzle (`ARGB`, `0x001000`), not a mix. **Decoded as of
2026-08-28** - see the section below.

## `U8U8U8U8`: no block structure, twiddled at texel granularity

**Confidence 80.** `oag_texture::gxt::Format::Argb8888`, `unit_len` 4 bytes,
one texel. Unlike the two compressed formats, `U8U8U8U8` has no block
structure to quantise a mip level's storage to - `Texture::level_len` reads
each level as the plain `width * height * 4`, with no `MIN_LEVEL_LEN` floor,
which is measured rather than assumed: all 99 `0x0c` textures in the base
package, across nine distinct `(width, height, mip count)` shapes down to a
chain that bottoms out at a single 4x4 level, agree with the unfloored
arithmetic exactly.

**The tiling question is settled the same way `UBC2`'s was, but at texel
rather than block granularity.** `type` at descriptor `+0x10` is `0` on every
`0x0c` texture measured, the same uninformative value the `UBC2` reticle
texture carried before that question was settled by decoding both ways and
looking at the picture. Raster order on `data/Tex/zoneModeTrack{0,7,14}.gxt`
(256x256, the "Track" half of 2048's Zone/Detonator speed-class art) decodes
to horizontal-banded noise, the same signature as `UBC2`'s wrong-order
control. Reading the same bytes through
[`twiddle`]'s general grid algorithm applied directly to texel coordinates
`(x, y)` - there being no 4x4 block to twiddle over - decodes cleanly: every
texel's `A`, `R` and `G` bytes agree exactly (0 mismatches across all 65,536
texels checked), a binary stencil mask opaque on exactly 2,048 of them on
every one of the three sampled stages, while `B` alone carries a smooth
multi-valued gradient. Composited over a checkerboard the mask reads as a
thin horizontal band whose *shape* escalates across the stage ladder (solid
at stage 0, increasingly dashed by 7 and 14) while its *area* does not - see
`the_zone_track_art_decodes_to_a_shape_that_escalates_across_stages` in
`crates/texture/tests/gxt_ground_truth.rs`, whose renders this rests on, and
the renders at stages 0, 7 and 14 (not kept in the repository).

**Why 80 and not higher**: the escalating-shape check is real evidence but a
weaker oracle than a font atlas or an HD-decoded ground truth - unlike
`PVRTII4BPP`, no independently-decoded copy of this exact art exists to
compare against (HD's own `.gtf` version of the same "Track" texture set is a
different container and a different codec, read long before this decoder
existed, and confirms only that a varying-versus-flat pair exists in the same
role, not the exact picture). The channel-order finding is strong on its own
terms (a lockstep three-channel binary mask alongside one continuous channel
is not something a wrong bit layout would produce by accident), which is
what earns this a confidence score at all where `U8U8U8`'s own channel order
gets none - see that format's own section below for why its corpus cannot
check it the same way.

## `PVRTII4BPP`: PowerVR texture compression, and not the PVRTC-I lookalike

**Confidence 92.** This is 85% of every `.gxt` on the disc and effectively
every texture a `.rcsmodel` material names, so until 2026-08-27
[2048-status.md](2048-status.md)'s "the circuit and the craft draw,
untextured" had two independent causes and this was one of them. See
`crates/texture/src/pvrtc.rs`, whose module doc carries the bit layout.

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
[`oag_texture::gtf`](gtf.md) decoded long before this, and as a `PVRTII4BPP`
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
`oag_texture::gxt::MIN_LEVEL_LEN` carries it.

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
See `crates/texture/tests/gxt_ground_truth.rs`, which is where those renders
come from and re-runs on demand.

**Corroborated externally**: `ClassiCube`'s own Vita port computes the same
even/odd bit-interleave masks (`TwiddleCalcFactors`) to write
`sceGxmTextureInitSwizzled` textures - this is Sony's documented hardware
tiling scheme, not a guess that happened to render something.

`oag_texture::gxt::twiddle` implements the general (non-square) algorithm:
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

## `UBC1`/`UBC3` decode too: the same twiddled block walk, BC1/BC3 instead of BC2

**Confidence 88, added 2026-09-16.** `UBC1` (BC1, 505 files) and `UBC3` (BC3,
493 files) were the two format bytes this page previously named as "the same
BC family as `UBC2` and would each be a small addition". They were: both
route through the same `blocks()` twiddled-block walk `UBC2` already uses,
generalised to take a block size and a decode function rather than being
hard-coded to BC2's 16 bytes and `bcn::dxt23` - BC1 is 8 bytes (`bcn::dxt1`,
no alpha channel), BC3 is 16 (`bcn::dxt45`, an interpolated alpha ramp). The
twiddle order is a property of the block *grid*, not of what a block decodes
to, so nothing about generalising it needed re-measuring on its own terms -
what did need checking, separately, was the length arithmetic and the
tiling/channel picture for each new block size.

**The length arithmetic closes without the `MIN_LEVEL_LEN` floor ever
firing**, which is worth recording precisely because the floor exists
specifically for a format whose *word* is smaller than 16 bytes
(`PVRTII4BPP`, 8-byte words) and `UBC1` is the first *block* format at that
same 8-byte size. Swept across all three EU packages
(`crates/game/examples/vita_gxt_ubc13_extent.rs`): the plain
`ceil(w/4) * ceil(h/4) * unit_len` formula, with no floor at all, closes on
**835 of 835** `UBC1` textures and **957 of 957** `UBC3` textures found
across base+DLC1+DLC2 (more than the base package's own 505/493, which is
what `docs/formats/README.md`'s and this page's own corpus counts are scoped
to). Every shipped `UBC1` chain has `mips = 1` and a base level of at least
64x64 (16x16 blocks), and every `UBC3` chain that reaches a single 4x4-block
level is already exactly 16 bytes there (`UBC3`'s own block size), so **the
16-byte floor case is genuinely unexercised by the shipped corpus for either
format** - it is applied anyway (`Format::unit_len` feeds the same
`level_len` every other block format goes through) for consistency, since a
floor that never fires cannot be wrong on the files that exist, and there is
no reason to special-case these two out of a check every other block format
in this module passes through.

**The tiling order is confirmed by the same "decode both ways and look at the
picture" method this page already used for `UBC2` and `U8U8U8U8`, and no HD
`.gtf` twin exists for either format to check against numerically** -
`crates/game/examples/vita_gxt_ubc13_hd_oracle.rs`, the same cross-title
oracle that gave `PVRTII4BPP` its 3.83-of-255 median, finds **0 same-name
same-size pairs** across all three EU packages for `UBC1` or `UBC3` - these
998 files are book/manual pages and front-end callouts, not the shared
circuit or roster art the DLC re-ships from Wipeout HD, so there is nothing
for either format to be twinned against. So the picture check is the only
oracle available,
and it discriminates cleanly because BC has no bilinear upscale to smooth a
wrong answer the way PVRTC does:

- **`UBC1`**: `data/Books/Manual/Pages/02/001.gxt` (1024x1024, one level, the
  electronic manual's own title page). Raster block order decodes to
  scrambled horizontal noise; twiddled order renders the full page crisply -
  the "WIPEOUT 2048 - MANUEL" heading and cover art, legible right down to
  the sponsor logos on the in-art billboards. Roughness (mean adjacent-texel
  delta, the same metric `U8U8U8U8`'s smoothness check uses) is 3.56
  twiddled against 5.57 raster, 1.56x - smaller than a clean win alone would
  suggest, because BC1 already compresses fine detail; the picture is what
  carries this, not the number.
- **`UBC3`**: `data/FE/NewImages/TinyCallout_MP.gxt` (512x256, one level, the
  front-end's "MULTIPLAYER / TOUCH TO START" callout). Raster order decodes
  to noise with fragments of the red/white text bleeding through; twiddled
  order renders the callout legibly, including a controller icon and the
  callout's own border outline that only become visible once alpha
  composites correctly - BC3's interpolated ramp reproduces the punch-through
  edge exactly the way `bcn::dxt45` already does for HD's own `.gtf` copies
  of BC3 art. Roughness is 2.09 twiddled against 4.08 raster, 1.95x.

Both renders are pinned in `crates/texture/tests/gxt_ground_truth.rs`
(`ubc1_decodes_to_something_a_human_can_check`,
`ubc3_decodes_to_something_a_human_can_check`) and written to a `.png` under `data/shots/`. The
roughness numbers above are not part of that pinned test - they come from
the scratch probe `crates/game/examples/vita_gxt_ubc13_picture_check.rs`,
which also writes the raster-order control PNGs
(the control PNGs the probe writes under `data/shots/`)
that are the actual evidence for the twiddle claim on these two formats -
the ground-truth test only renders the correct decode. `UBC1`'s own
check cannot reuse `UBC2`/`UBC3`'s shared `render_checkerboard` helper - BC1
has no alpha channel at all (`bcn::dxt1` always decodes opaque), so the
helper's "not every texel opaque" flatness assertion is the wrong invariant
for it; `UBC1`'s test checks colour diversity and full opacity instead.

**Why 88 and not the 92 `PVRTII4BPP` holds**: no cross-title numeric oracle
exists for either format (unlike `PVRTII4BPP`'s 2,284 HD-paired textures), so
this rests on the same class of evidence `U8U8U8U8`'s 80 does - a picture
check plus a roughness metric - but scores higher than `U8U8U8U8`'s 80
because the picture check here is *legible running text at multiple sizes*
on two independent files with two independent block sizes, not one
escalating-mask art asset, and the length arithmetic had a real edge case
(the 8-byte-block floor) to fail on and didn't.

## `U8U8U8`: the last format byte, two measured claims and one chosen, not measured

**Confidence 70 for tiling and packing, added 2026-09-16. Channel order
carries no score - it is chosen, not measured, per this project's own rule
against scoring an unverified pick as if it were evidence.** `0x98`, 13
files, all `Data\FE\NewImages\scepresents\scee_presents_<language>.gxt`
(512x64, one level each) - the Vita's per-territory "Sony Computer
Entertainment presents" splash. `oag_texture::gxt::Format::Rgb888`,
`unit_len` 3 bytes, no block grid, the same shape [`Format::Argb8888`] is
except one byte narrower.

**Measured claim 1: 3 tightly-packed bytes a texel, no floor.** `width *
height * 3` matches the declared texel length exactly on all 13 files
(`crates/game/examples/vita_gxt_u8u8u8_extent.rs`) - the same verification
`Argb8888`'s own unfloored arithmetic rests on, not assumed by analogy to it.

**Measured claim 2: the tiling order.** Confirmed the same way as every
other format on this page: raster order decodes to noise; twiddled order
(texel granularity, the same [`twiddle`] algorithm `U8U8U8U8` uses) renders
"Sony Computer Entertainment presents" crisply, right way up, on every one
of the 13 files sampled (`crates/game/examples/vita_gxt_u8u8u8_picture_check.rs`).

**Chosen, not measured: the channel order.** The corpus itself cannot settle
it, and this is not a gap this reading glossed over. Measured directly
across all 13 files: **every texel has `max(byte) - min(byte) == 0`** - the
art is pure grayscale line work, so `R,G,B`, `B,G,R`, and every other
permutation of the three bytes decode to the bit-identical picture.
`Rgb888`'s decode reads `R, G, B` in file order, for consistency with
`Argb8888`'s own byte-order convention rather than because a colour sample
confirmed it - none exists in this corpus to check against, so this reading
is picked, not verified, and carries no confidence number of its own. This
is not the same situation `U8U8U8U8` was
in: that format's ARGB swizzle was corroborated against the public `vitasdk`
enum *and* against which channel of an actual varying-colour texture carried
continuous art versus a flat mask - both readings agreeing is what earned
`U8U8U8U8` its 80. `0x98`'s own identification as a 3-channel 8-bit
`SceGxmTextureBaseFormat` was already checked against the same public header
when `0x0c` was corrected on 2026-08-28 ("`0x98` (`U8U8U8`) was checked too
and is correct as already documented" - that commit's own message) and is
not in question here; what is missing is the second corroboration. No `0x98`
texture on this disc has a second channel to disagree with the first, so the
swizzle/channel-order bits cannot be read the way `0x0c`'s were. A future
`0x98` file with real colour, if one ever ships in a DLC pack this project
has not swept, is the only thing that could turn the channel-order pick into
a scored, checked claim rather than a documented choice.

**The decode is kept anyway, not withheld pending that corroboration** -
worth stating outright, since a chosen-not-measured field is exactly the
shape `CLAUDE.md`'s "never invent what the assets already author" rule
warns against. The difference here: every permutation of the three bytes
produces the *bit-identical* output on 100% of the shipped corpus (max
channel spread 0 on all 13 files), so this is not a stand-in painted over
missing data - it is one specific, disclosed reading of data that happens
to be unable to distinguish itself from five others on the only evidence
available.

## Coverage

`oag_texture::gxt::coverage` claims the header, the descriptor table and
every texture's own texel span. `Gxt::parse` already checks that the header's
own texel-span field equals `descriptors_end` on one side and `data.len()` on
the other before it accepts a file at all, so there is no room for a gap once
a file parses - and the sweep confirms it rather than only trusting the
parser's own check: **100.00%** of all 9,910 `.gxt` files in the base
package, 1.59 GB, `crates/texture/tests/gxt_coverage_ground_truth.rs`.

## Wired into the sprite loader

`oag_hud::sprite::Image::decode` tries `.gxt` as a fourth branch alongside
the PSP `.mip`, PS2 and PS3 `.gtf` readers, so a HUD sprite reference that
resolves to a shipped `.gxt` entry decodes to real pixels through the same
`Sheet` every other title's HUD uses. See
[2048-hud.md](2048-hud.md#what-is-not-done) for why nothing draws in a race
yet regardless.

## See also

- [gtf](gtf.md) - the PS3 sibling this module's doc comments compare against
  throughout, and the source of the shared `oag_texture::bcn` block math
- [2048-hud](2048-hud.md) - what this format unblocks and what it does not
- [2048-status](2048-status.md) - where this fits in the wider probe
