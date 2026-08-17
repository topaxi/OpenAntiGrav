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
rubric](../reverse-engineering/confidence-rubric.md) caps a static reading, and
because 53 of the 7,333 are refused rather than decoded.

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
| 37 | `0x85` | `A8R8G8B8` | **no** | no |
| 9 | `0x81` | `B8` | **no** | no |
| 7 | `0x9e` | `A8B8G8R8` | **no** | no |
| 3 | `0xa8` | `DXT45` | yes | yes |
| 1 | `0xa6` | `DXT1` | yes | yes |
| 1 | `0xa7` | `DXT23` | yes | yes |

**7,154 of 7,333 are block-compressed**, which is why implementing BC1/BC2/BC3
covers almost the whole disc. Block compression *is* a tiling, so the `0x20`
"linear" bit is not consulted for those; it is consulted for the uncompressed
ones, and the 53 without it are in the RSX's Morton order.

**Those 53 are refused by name rather than read linearly.** A linear read of a
swizzled texture is a recognisable picture in scrambled tiles - exactly the kind
of wrong answer that survives a review, because it looks like a bug in something
else. `Texture::to_rgba` returns `Error::Swizzled` instead. None of them is a
HUD or a circuit texture.

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

## 23 cubemaps, parsed and not decoded

`cubemap` is set on 23 files, every one a `sky` or an environment probe. Six
faces follow one another and the length invariant holds on all 23 - **with an
unexplained 360 bytes** on the 20 that carry a mip chain.

The same 360, whether the faces are 128x128 `DXT1` (`envlight.gtf`: 65,976
against six faces of 10,936, and 65,976 - 65,616 = 360) or 2048x2048
(`12_sol_2/sky.gtf`: 16,777,656 against 16,777,296). Sixty bytes a face, and
sixty is not a multiple of any block size here. The three single-level cubemaps
carry no slack at all.

Nothing reads a cubemap yet, so the slack is recorded in `Texture::chain_len`
and `to_rgba` returns `Error::Cubemap` rather than a face. **Do not assume the
first face starts at offset zero** without checking; that is the natural
reading and it has not been verified against anything.

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

- **The 53 swizzled textures.** RSX Morton order, unimplemented.
- **The 23 cubemaps' face layout**, and the 360 bytes.
- **`remap`.** Three values on the disc - `0xaae4` on 7,317, `0xa9ff` on 9,
  `0xa9e4` on 7 - and what they select is unread. The 16 that differ are all
  formats `to_rgba` refuses anyway, so the gap costs nothing today and would
  matter the day `B8` is decoded.
- **Mip levels past the base.** They are measured, because the length check
  needs them, and `Texture::level_range` will address one; nothing decodes one.
- **Nothing is drawn.** `oag_render` does not upload a `.gtf` yet, so no HD
  texture has reached a shader.
- **No emulator check**, which is the ceiling on the score.

## See also

- [hd-hud](hd-hud.md) - the HUD, which is what needed this
- [hd-status](hd-status.md) - the format layer across the whole disc
- [psarc](psarc.md) - the container these come out of
- [rcsmodel](rcsmodel.md) - HD's geometry, whose texture coordinates this
  unblocks checking
- [psp-texture](psp-texture.md) and [ps2-texture](ps2-texture.md) - what the
  lineage did before this
