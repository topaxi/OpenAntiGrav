# GTF: the PS3's texture container

**Status: understood, confidence 92.** `.gtf` is the single largest thing on a
*Wipeout HD / Fury* disc - **7,333 files, 2.4 GiB**, more than half its archives
by volume - and it is Sony's own container rather than anything Studio Liverpool
invented: a twelve-byte header, a count, and one `CellGcmTexture` descriptor per
texture written out verbatim.

Implemented in [`oag_formats::gtf`](../../crates/formats/src/gtf.rs); swept
against the whole disc by
[`crates/formats/tests/gtf_ground_truth.rs`](../../crates/formats/tests/gtf_ground_truth.rs).
Measured 2026-08-17 on `hdfury-ps3-eu-dec.iso`, serial `BCES-00664`.

```sh
OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-formats --run-ignored all \
    -E 'binary(gtf_ground_truth)'
```

92 rather than higher because **nothing has been compared against the running
original**, which is where the [confidence
rubric](../reverse-engineering/confidence-rubric.md) caps a static reading.
9 of the 7,333 (swizzled `B8`) are still refused rather than decoded - see
below, where the other 44 swizzled files stopped being part of that count.

## Layout

```text
+0x00  u32   version              0x01050000 (6,980) or 0x02010100 (353)
+0x04  u32   size of everything after the header
+0x08  u32   texture count        1 in all 7,333
then `count` descriptors, 36 bytes each:
+0x00  u32   id
+0x04  u32   offset of the texel data, from the start of the file
+0x08  u32   length of the texel data
+0x0c  u8    format               low 5 bits texel format, 0x20 linear, 0x40 unnormalised
+0x0d  u8    mip levels           1 to 12
+0x0e  u8    dimension            2 in all 7,333
+0x0f  u8    cubemap              1 in 23 of 7,333
+0x10  u32   remap                a channel permutation, not acted on
+0x14  u16   width                3 to 2048
+0x16  u16   height               1 to 2048
+0x18  u16   depth                1 in all 7,333
+0x1a  u8    location             0 in all 7,333
+0x1b  u8    padding
+0x1c  u32   pitch                bytes of one base-level row, or 0
+0x20  u32   GPU offset           filled in at load, 0 in a file
```

Everything is big-endian - **except the texel payload**, which is not. See
[the endpoints](#the-dxt-endpoints-are-little-endian-inside-a-big-endian-file).

## Two invariants, and they close on the whole disc

Nothing here was guessed at and then found to work on a handful of files. Both
of these are arithmetic the file fully determines, and both hold on **7,333 of
7,333**:

1. `12 + 36 * count + size` is the file length. Since `count` is 1 throughout,
   the texels start at `+0x80` and run to the end.
2. The declared texel `length` is exactly what the format, both dimensions, the
   mip count, the cubemap flag and the pitch imply.

The second is the load-bearing one. It cannot come out right by accident on a
corpus this varied - 3x1 to 2048x2048, one to twelve mip levels, ten distinct
format bytes, **131 non-power-of-two textures** - and it is what makes
`Gtf::parse` a real identification rather than a cast.

## `pitch` does not halve down the mip chain

The rule four files turn on, and getting it wrong reads as a corrupt file rather
than as a wrong reading. When `pitch` is non-zero it is one row of the **base**
level, and every further level uses **the same pitch**, not half of it.

| File | Descriptor | Declared | Why |
| --- | --- | ---: | --- |
| `zone_2/gradienttex_tr01_set01.gtf` | 3x1 `A8R8G8B8`, pitch 12, 2 levels | 24 | 12 + 12, not 12 + 6 |
| `fe/images/hexmedal_hd.gtf` | 1024x768 `DXT45`, pitch 4096, 1 level | 786,432 | 4,096 x **192 block rows** |
| `amphiseum/…/air_traffic_test_a_atoc.gtf` | 1028x256 `DXT45`, pitch 4112, 11 levels | 538,672 | 4,112 x **131**, the sum of the chain's block-row counts |
| `amphiseum/…/air_traffic_test_emissive.gtf` | 1028x256 `DXT1`, pitch 2056, 11 levels | 269,336 | 2,056 x 131 |

Those last three also settle a second question: **a pitch can be declared for a
block-compressed texture**, and there it is bytes of one *block* row rather than
one pixel row - 4,112 is 257 blocks of 16, and 2,056 is 257 blocks of 8. A
reader that treats a compressed pitch as pixels is out by a factor of four and
fails the length check, which is how this was found.

## What is on the disc

| Count | Byte | Texel format | Linear | Decoded |
| ---: | --- | --- | :-: | :-: |
| 4,137 | `0x88` | `DXT45` (BC3) | - | yes |
| 2,485 | `0x86` | `DXT1` (BC1) | - | yes |
| 527 | `0x87` | `DXT23` (BC2) | - | yes |
| 126 | `0xa5` | `A8R8G8B8` | yes | yes |
| 37 | `0x85` | `A8R8G8B8` | **no** | **yes** |
| 9 | `0x81` | `B8` | **no** | no |
| 7 | `0x9e` | `A8B8G8R8` | **no** | **yes** |
| 3 | `0xa8` | `DXT45` | yes | yes |
| 1 | `0xa6` | `DXT1` | yes | yes |
| 1 | `0xa7` | `DXT23` | yes | yes |

**7,154 of 7,333 are block-compressed**, which is why implementing BC1/BC2/BC3
covers almost the whole disc. Block compression *is* a tiling, so the `0x20`
"linear" bit is not consulted for those; it is consulted for the uncompressed
ones, and the 53 without it are in the RSX's Morton order.

**44 of those 53 - the swizzled `A8R8G8B8`/`A8B8G8R8` - decode now, 2026-09-02.**
A linear read of a swizzled texture is a recognisable picture in scrambled
tiles - exactly the kind of wrong answer that survives a review, because it
looks like a bug in something else - which is why `Texture::to_rgba` refused
them by name for as long as it did, rather than risk it. What unblocked it:
`corner2.gtf` (`Data\FE\Images\corner2.gtf`, `DATA06`, 16x16 `A8R8G8B8`
swizzled), a corner mask for `<Bracket corner="true">` widgets, needed
decoding to check a hypothesis about the main menu's own tab shape (see
[hd-frontend.md](hd-frontend.md)) - and once one file needed reading, reading
all 44 properly was the right size of fix, not a one-off workaround for that
file alone.

**The address function is `decode::morton_index`: the RSX's own documented
`cellGcm` tiling** (interleave the low bit of `x` then of `y`, one pair at a
time, until the narrower dimension's bits run out), not reverse-engineered
from this disc - it is the platform's standard 2D-surface swizzle. What *is*
measured here is that it is the right permutation for these files specifically,
by the same method the DXT endianness question below uses: decode every
swizzled `A8R8G8B8`/`A8B8G8R8` file two ways - the real Morton order, and a
deliberately wrong linear misread - and compare how smooth each comes out.
**33 of 34 judgeable files are smoother under the Morton reading**, well past
the same >90% bar the DXT question settles on, with the other 10 too flat (a
uniform-RGB alpha mask, `corner2.gtf` among them) to judge either way. The one
exception was inspected by eye rather than folded into "close enough":
`fealphaluminancetexture.gtf` decodes to a coherent blocky test chart whose
rows are individually uniform, which defeats a roughness metric that only
looks within a row - not scrambled tiles, the failure this test exists to
catch. Two more (`feburneffect_test.gtf`, a dissolve noise texture;
`fetesttexture_001.gtf`, a scratch/detail map) looked like exceptions before
the `remap` fix below landed, and cleared the bar once real blue-channel
garbage stopped leaking into the comparison - see
`crates/formats/tests/gtf_ground_truth.rs`.

**`Texture::remap` forces blue to `0xff` on all 7 `A8B8G8R8` files, and this
decode now acts on it rather than reading blue off the texel.** The "Not done"
list below already read `0xa9e4`'s bit packing at confidence 75, off the
published `CELL_GCM_REMAP_MODE` layout, and called the gap moot because
`to_rgba` refused every file it could apply to anyway. It stopped being moot
the moment those 7 files started decoding: two of the three roughness
exceptions above were this, not the address function - real, un-ignorable
blue-channel bytes on disc that the descriptor says to discard, contaminating
the "native" reading until `Texture::apply_remap` started discarding them
too. `B8`'s own `0xa9ff` remains unread; `to_rgba` still refuses that format
outright, so it stays moot.

**Confidence 88** - an exact match on a synthetic 4x4 fixture plus a
corpus-wide statistical result, both real evidence, but nothing corroborated
against the executable: `EBOOT.elf` carries no `swizzle` string and no
texture-upload routine has been located to check the address function
against a decompiled read. `B8` (9 files, still refused) is the one format
this leaves undecoded - one channel, swizzled on every file that carries it,
and what the byte itself means is unread, a separate question from the
address function this section settles.

## The DXT endpoints are little-endian inside a big-endian file

Every header field is big-endian, so the natural guess is that a block's two
`R5G6B5` endpoints are too. They are not: the texel payload is whatever the RSX
consumes, and that is the same block layout a `.dds` stores.

**Measured three ways, not assumed.**

1. **Corpus roughness.** Decode both readings and take the mean absolute
   difference between horizontally adjacent RGB texels; real art is smooth
   across a block boundary and a byte-swapped `R5G6B5` is not, since swapping
   moves five bits of red into the low bits of blue. Over `DATA00` and `DATA03`,
   2,040 compressed textures with anything to say: **little-endian is smoother
   on 1,969, byte-swapped on 69**, mean roughness ratio **6.58x**. (195 more are
   flat either way - white-RGB masks; see below.)
2. **The picture.** `hud_components.gtf` decodes to Wipeout HD's HUD atlas with
   its blue and yellow intact; the byte-swapped reading has the same shapes in
   magenta and pink with colour fringing on every edge.
3. **A different question with the same answer.** `bullet.gtf` and
   `bullet_back.gtf` are linear `A8R8G8B8`, and byte 0 is exactly 0 or 255 on
   100% of their texels where bytes 1-3 are on 0% of `bullet_back`'s. So byte 0
   is alpha and the bytes arrive A, R, G, B, which is `A8R8G8B8` read as a
   big-endian word - the header convention holding for the *channel* order at
   the same time as the endpoint words are little-endian.

`fe/images/trial/large/shot1.gtf` is the check anyone can repeat: a 1536x470
screenshot of a Feisar-liveried craft that comes out with a blue sky and red
accents under the A,R,G,B reading and an orange sky under B,G,R,A.

The 2-bit selector word is read a byte per row, which is the same answer either
way round and needs no argument.

## The retro HUD skins are white, and that is not a decode failure

`wo3_hud.gtf`, `missile_reticule.gtf` and `duel_bars.gtf` decode to RGB that is
pure white on **100% of their texels**, with every shape living in the alpha
channel - a `DXT23`'s four bits per texel. It reads as a broken decoder and is not: the HUD layouts carry
`Color="FEConst->HudColour1"` on those sprites and tint them at draw time, so a
white mask is exactly what the art should be. **195 of the 2,235 compressed
textures examined across `DATA00` and `DATA03`** are flat this way; the other
five archives were not swept for it.

This is worth knowing before measuring anything about a `.gtf` on its RGB alone,
which is why the endianness test above skips them rather than counting them as
ties.

## 23 cubemaps, and they decode

`cubemap` is set on 23 files, every one a `sky` or an environment probe. All 23
decode, six faces each, through `Texture::face_to_rgba`.

### Face-major, and the other reading has the same length

**All of one face's levels, then the next face's.** That is what
`Texture::chain_len` already had to assume for the length invariant to close -
`one_face * 6 + slack` - and it is now asserted rather than left implicit,
because the alternative reading is not caught by any arithmetic: **level-major
storage has exactly the same total length** and produces a scrambled sky.

What separates them is content. Talon's Junction's `sky.gtf` is 1024x1024 `DXT1`,
one level, and read face-major its six faces are:

| Face | RSX direction | Mean RGB | What it is |
| --- | --- | --- | --- |
| 0 | `+X` | `(204, 221, 229)` | horizon, with a city skyline |
| 1 | `-X` | `(203, 223, 233)` | horizon |
| 2 | `+Y` | `(78, 123, 165)` | **the zenith** - deep blue with cloud |
| 3 | `-Y` | `(255, 255, 255)` | **the nadir** - blank white |
| 4 | `+Z` | `(217, 229, 232)` | horizon |
| 5 | `-Z` | `(190, 214, 230)` | horizon |

A blue sky above, a blank floor below, and four sides whose horizon line sits at
the same height on each. Read level-major, a single-level cubemap's six "faces"
would each be a slice of one image and none of that would hold. The order
`+X, -X, +Y, -Y, +Z, -Z` is **published rather than measured** - the RSX
inherits OpenGL's - and what checks it is that picture.

### The content claim is asserted on one sky and reported for the other 15

Two circuits are why it is not a threshold over the corpus. `amphiseum`'s sky is
a **night** sky - every face near black, zenith luma 100 against a nadir of 0 -
so a brightest-against-darkest bar measures exposure rather than structure. And
`zone_1`'s is a featureless grey void whose zenith (117) and nadir (150)
genuinely are alike. Both are data. A bar that called either a failure would be
a bar picked to pass, so `gtf_ground_truth.rs` asserts the exact face means on
the one sky whose picture was looked at and prints a census of the rest.

### The 360 unexplained bytes are unchanged

Six faces follow one another and the length invariant holds on all 23 - **with
an unexplained 360 bytes** on the 20 that carry a mip chain. The same 360,
whether the faces are 128x128 `DXT1` (`envlight.gtf`: 65,976 against six faces of
10,936, and 65,976 - 65,616 = 360) or 2048x2048 (`12_sol_2/sky.gtf`: 16,777,656
against 16,777,296). Sixty bytes a face, and sixty is not a multiple of any block
size here. The three single-level cubemaps carry no slack at all - Talon's
Junction's sky is one of them, which is why it is the one that was decoded first.

## The mip chain is the disc's, and a box filter is not it

**The renderer binds the file's own DXT blocks and the file's own mip chain**,
rather than decoding the base level to RGBA8 and box-filtering a chain from it.
Two reasons, and the second is the one that matters for the picture.

The first is size. Of the 7,131 flat block-compressed textures on the disc,
**6,482 carry a chain** and only 649 are single-level. Across the textures one
race touches, the base levels are **78 MiB as blocks and 348 MiB decoded** -
113 `DXT45`, 72 `DXT1`, 44 `DXT23`, 4.5x either way. The whole authored chain
across the disc is 2,204 MiB of blocks against 7,690 MiB for base levels alone
as RGBA8.

The second is that **the chains are not the ones a box filter produces.**
Decoding both readings of every mipped `.gtf` on Talon's Junction - 245
textures - and comparing the authored level 1 against a 2x2 box filter of level
0 gives a mean absolute difference of **2.41 of 255**, and the worst are not
close at all:

| Mean | Max | Texture |
| ---: | ---: | --- |
| 20.18 | 255 | `textures/dds/dc_scanlines.gtf` |
| 20.18 | 89 | `textures/dds/billboard3.gtf` |
| 12.16 | 165 | `textures/dds/colours_flashing_glow.gtf` |
| 12.05 | 42 | `textures/adverts/hologramscanlines.gtf` |
| 12.02 | 81 | `hd_textures/crowd/crowdnoise.gtf` |

A scanline pattern is exactly where the two diverge hardest: a box filter of
alternating rows averages to flat grey, and whatever the artist did instead is
what the RSX minified with. So the disc's chain is not merely cheaper, it is the
correct one - the same rule that governs every other authored table here.

**Four conditions gate it**, each falling back to decode-and-box-filter, and
each is a small minority of the disc: a non-block format (202 files), a
single-level file (649), a declared pitch (5 - because a pitch is a *base*-level
row repeated down the chain, see above), and base dimensions off the 4x4 block
grid (15), which WebGPU refuses for a compressed texture. Cubemaps take the
`face_to_rgba` path and are untouched by this. An adapter with no
`TEXTURE_COMPRESSION_BC` - the GL backend - decodes the base level back through
`gtf::decode_level` rather than drawing nothing.

Implemented as `oag_render::mesh::Texels`, chosen in
`oag_render::mesh::rcs::skin::blocks` and uploaded by
`oag_render::mesh_render::texture::upload`.

## What this was read for

[The HUD](hd-hud.md). Its twelve textures are nine `DXT45`/`DXT23` atlases, and
`crates/game/tests/hd_hud_ground_truth.rs` now checks each of the **1,029 HUD
sprites' source rectangles against the dimensions of the texture it names** -
a check that tests both readings at once, since a layout misread puts a
sub-rectangle outside its texture and a dimension misread fails to contain
rectangles that really do fit.

**All 1,029 land, with one authored exception**: `VoiceCom0`-`VoiceCom7` take a
62x64 patch from `V="1"` of a 64x64 image, one texel past the bottom edge. 32 of
the 56 copies do it and the rest use `V="0"`, which is how a duplicated fragment
diverges. It is pinned in the test rather than tolerated.

## Not done

- **9 `B8` files, swizzled.** One channel, and what the byte itself means is
  unread - the Morton *address* function is the same one the 44
  `A8R8G8B8`/`A8B8G8R8` files above now decode with, but reading further into
  `B8`'s single byte needs its own reasoning `to_rgba` does not have yet.
- **A cubemap's mip levels.** `face_range` addresses one and nothing decodes one,
  the same gap the flat textures have.
- **The 360 bytes** on the 20 multi-level cubemaps. Their face layout is read;
  this tail is not.
- **`remap`, mostly the identity, and no longer entirely unread.** Three
  values on the disc. `0xaae4` on **7,317 of 7,333** unpacks, under libgcm's
  `CELL_GCM_REMAP_MODE` packing, to all four output channels taking "remap"
  rather than a forced 0 or 1, with sources `A<-A, R<-R, G<-G, B<-B` - the
  identity permutation, acted on by construction since that is what decoding
  a texel's four bytes straight already does. `0xa9e4`, on all 7 `A8B8G8R8`
  files, forces blue to one and **is** acted on now, by
  `Texture::apply_remap` - see above. `0xa9ff`, on all 9 `B8` files, broadcasts
  one channel to all four, which is moot for as long as `to_rgba` refuses
  `B8` outright. **Confidence 75** on the bit packing, which is a
  published-header reading rather than something measured against behaviour;
  the *distribution* (which files carry which value) is measured, and now so
  is `0xa9e4`'s effect - see the roughness numbers above, which dropped from
  three exceptions to one exactly when this was wired in.
- **Mip levels of a cubemap, and of a texture that declares a pitch.**
  `face_range` addresses the first and nothing decodes one; the second is 5
  files on the disc and takes the decode-and-box-filter path. Flat, tightly
  packed chains are bound as the disc stores them - see [the section
  above](#the-mip-chain-is-the-discs-and-a-box-filter-is-not-it).
- **The front end's `<Image>` widgets.** `oag_render` uploads a `.gtf` now -
  HD's HUD samples ten of them, and an HD craft and circuit are painted from
  their materials' own textures through
  [`oag_render::mesh::rcs`](rcsmodel.md#the-texture-a-material-paints-with) -
  but `boot::load_sprites` still offers a front-end image to the PSP `.mip`
  path alone and gets `zero-sized texture 1281x0`. A wiring gap, not a format
  one.
- **No emulator check**, which is the ceiling on the score.

## See also

- [hd-hud](hd-hud.md) - the HUD, which is what needed this
- [hd-status](hd-status.md) - the format layer across the whole disc
- [psarc](psarc.md) - the container these come out of
- [rcsmodel](rcsmodel.md) - HD's geometry, whose texture coordinates this
  unblocks checking
- [psp-texture](psp-texture.md) and [ps2-texture](ps2-texture.md) - what the
  lineage did before this
