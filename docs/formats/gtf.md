# GTF: the PS3's texture container

**Status: understood, confidence 92.** `.gtf` is the single largest thing on a
*Wipeout HD / Fury* disc - **7,333 files, 2.4 GiB**, more than half its archives
by volume - and it is Sony's own container rather than anything Studio Liverpool
invented: a twelve-byte header, a count, and one `CellGcmTexture` descriptor per
texture written out verbatim.

Implemented in [`oag_texture::gtf`](../../crates/texture/src/gtf.rs); swept
against the whole disc by
[`crates/texture/tests/gtf_ground_truth.rs`](../../crates/texture/tests/gtf_ground_truth.rs).
Measured 2026-08-17 on `hdfury-ps3-eu-dec.iso`, serial `BCES-00664`.

```sh
OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-formats --run-ignored all \
    -E 'test(/^gtf_ground_truth::/)'
```

92 rather than higher because **nothing has been compared against the running
original**, which is where the [confidence
rubric](../reverse-engineering/confidence-rubric.md) caps a static reading.
**All 7,333 decode as of 2026-09-02**; the last 9, swizzled `B8`, are the
section below.

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
+0x10  u32   remap                per-channel source and force table; acted on
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
| 9 | `0x81` | `B8` | **no** | **yes** |
| 7 | `0x9e` | `A8B8G8R8` | **no** | **yes** |
| 3 | `0xa8` | `DXT45` | yes | yes |
| 1 | `0xa6` | `DXT1` | yes | yes |
| 1 | `0xa7` | `DXT23` | yes | yes |

**7,154 of 7,333 are block-compressed**, which is why implementing BC1/BC2/BC3
covers almost the whole disc. Block compression *is* a tiling, so the `0x20`
"linear" bit is not consulted for those; it is consulted for the uncompressed
ones, and the 53 without it are in the RSX's Morton order.

**All 53 decode as of 2026-09-02.**
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
**43 of 43 judgeable files are smoother under the Morton reading, with no
exceptions at all**, the other 10 being too flat (a uniform-RGB alpha mask,
`corner2.gtf` among them) to judge either way.

That is up from *33 of 34 with one named exception*, and the improvement is
two corrections to the measurement rather than a change to the decode:

1. **The remap was applied to one side only.** `to_rgba` applies the
   descriptor's `remap`; the deliberately-wrong reading it was compared
   against did not. So forcing a channel to a constant lowered the native
   reading's roughness and nothing else's - which is the whole of the evidence
   that previously appeared to support reading `0xa9e4` as forcing *blue*.
   Both sides get the same remap now.
2. **Roughness was measured along rows only.** Reading a tiled surface as
   raster order lays each tile out as a run of consecutive texels, so the
   *wrong* reading comes out as horizontal stripes - and stripes are uniform
   along a row. A within-row metric scores the misread as the smooth one on
   exactly the blocky test charts that were carried as exceptions. Adding the
   vertical axis (`roughness_2d`) retires all three named files, which is
   better than keeping a list of files the metric cannot see.

**Confidence 88** - an exact match on a synthetic 4x4 fixture plus a
corpus-wide statistical result, both real evidence, but nothing corroborated
against the executable: `EBOOT.elf` carries no `swizzle` string and no
texture-upload routine has been located to check the address function
against a decompiled read.

## The 9 `B8` files are ship shadows, and their own `remap` broadcasts them

The last format to decode, 2026-09-02, and its **names** were what settled it:
all 9 are `/data/ships/<team>/textures/ambient_shadow.gtf`, 128x64, one each
for `ag_systems`, `assegai`, `egx`, `feisar`, `goteki`, `piranha`, `qirex`,
`triakis` and `zone`. A one-channel texture called `ambient_shadow` is a
craft's contact shadow, so there is a right answer to look for rather than a
plausible-looking one to settle for: read in Morton order each is that team's
craft in soft silhouette - Feisar's delta with its tailfin, Qirex's blunt oval
- and read in raster order each is horizontal banding. Morton is also 1.6x to
2.5x smoother on every one of the nine, but the picture is the evidence and
the number is what
[`the_single_channel_textures_are_ship_shadows_and_read_in_morton_order`](../../crates/texture/tests/gtf_ground_truth.rs)
pins.

**These are also the only textures on the disc that exercise `morton_index`'s
non-square branch.** Every other swizzled file is square, so both dimensions'
bits run out together and the "let the wider dimension continue linearly"
phase never runs. `128x64` runs it, and
`gtf::tests::a_single_channel_texture_is_broadcast_by_its_own_remap` pins a
4x2 case against the hand-derived order.

**Nothing in the decoder decides that one byte means grey.** It lands in blue
and stays there; the descriptor's own `remap` is what broadcasts it - see
below.

## `remap` is a per-channel source and force table, and it is read now

`+0x10` is two tables packed into sixteen bits, laid out the way the RSX's
`NV4097_SET_TEXTURE_CONTROL1` register reads them: the **low** byte holds four
2-bit *source* selectors and the **high** byte four 2-bit *controls*, both in
`A`, `R`, `G`, `B` order from the least significant pair up. A source names
which channel an output reads (`0` = A, `1` = R, `2` = G, `3` = B); a control
says whether that read happens at all, or the output is forced to zero or one.

Three words appear on the disc, and **each decomposes into something the file
carrying it independently agrees with** - which is why this is a reading and
not an assertion:

| Word | Files | Decomposes to | Agrees with |
| --- | ---: | --- | --- |
| `0xaae4` | 7,317 | every control "read", sources `A<-A, R<-R, G<-G, B<-B` | the identity, which is what decoding a texel straight already does - the value a wrong reading of the packing would *not* land on |
| `0xa9e4` | 7 | the same, with **alpha** forced to one | `fealphaluminancetexture.gtf` is 100% grey - all four of every texel's bytes equal - so its RGB is a luminance and its alpha is not stored art |
| `0xa9ff` | 9 | alpha forced to one, every other output reading the **blue** source | these are exactly the 9 `B8` files, whose one stored byte *is* the blue channel |

The third row is the load-bearing one. The low byte `0xff` selects source `3`
four times; `3` is blue under the same table that makes `0xe4` the identity;
and the format that carries `0xa9ff` stores one byte in blue and nothing else.
Format and remap are read from different halves of the descriptor and say the
same thing.

### Corroborated against the executable, 2026-09-02

The engine **constructs** these words in code rather than only copying them out
of files, and `Texture_BuildGcmRegisters` (`0x005a9998`, see
[renderer.md](../ghidra/functions/ps3-hdfury-eu/renderer.md#texture_buildgcmregisters-and-what-it-settles-about-a-gtfs-remap))
is where they land: it takes one packed `(format << 16) | remap` word, extracts
the format's low five bits from the high half exactly as `Format::from_byte`
does, and writes the low **seventeen** bits - the sixteen-bit remap plus the
`order` bit above it - into a register slot of their own.

Two things follow that the disc's files alone could not give:

- **`0xa9ff` is the engine's own one-channel remap.** `FUN_00175798` builds
  four single-channel runtime surfaces per iteration with the packed word
  `0x0100a9ff`, whose format nibble is `0x01`, `B8`. The engine's own scratch
  surfaces and the disc's 9 shadow textures arrive at the same pairing
  independently.
- **A fourth word settles the pair order.** The same function compares against
  the literal `0x1c0009e4`. Under this reading `0x09e4` is alpha forced to one,
  red read, green and blue forced to zero - a single-channel-in-red texture.
  Under the opposite pair order it is green alone with blue forced on and alpha
  off, which is not a thing. Starker on `0xa9ff`: the opposite order forces
  *blue* to one, and blue is the only channel a `B8` stores, so it would
  discard the texture's whole content.

**Confidence 88**, up from the 75 this carried as a bare constant matched by
value. The layout predicts three distributions on disc, one of them
cross-checks against an unrelated field, and a fourth word in the executable
decomposes sensibly under this pair order and nonsensically under the other.
Short of 92 because the command-buffer write that consumes `+0x30` is not
traced - the slot is identified by taking exactly the remap word, not against a
decompiled `cellGcmSetTexture`.

**This corrected a real bug.** `0xa9e4` was previously read as forcing *blue*
to one - the alpha control pair mistaken for blue's - which rendered all 7
`A8B8G8R8` files solid blue, `fealphaluminancetexture.gtf` among them. The
roughness numbers that appeared to confirm it were the one-sided-remap
artefact described above.

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
`face_stride * 5 + one_face`, the stride being the next section's - and it is
now asserted rather than left implicit,
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

### Each face starts on a 128-byte boundary

**Confidence 85.** A face's whole mip chain is followed by padding up to the
next 128-byte boundary before the next face begins; the last face is not
padded. Measured 2026-11-16 (`sol2-sky`) on all 23 cubemaps of
`hdfury-ps3-eu-dec.iso`:

| Cubemaps | One face's chain | Mod 128 | Pad per face | Declared length |
| --- | --- | --- | --- | --- |
| 15 single-level (1024x1024, 2048x2048 and 64x64 `DXT1`) | a multiple of 128 | 0 | 0 | `6 * one_face` |
| 8 multi-level (`12_sol_2/sky.gtf` 2048x2048 x12, `zone_3/sky.gtf` and three `env.gtf` probes 256x256 x9, three `envlight`/`envblur` 128x128 x8) | 2,796,216 / 43,704 / 10,936 | 56 | 72 | `5 * (one_face + 72) + one_face` |

`5 * align128(one_face) + one_face` equals the declared length on **23 of 23**.
This is what was recorded here as "an unexplained 360 bytes" (and as "sixty
bytes a face"): it is 72 bytes after each of the first five faces. It looked
constant because every full `DXT1` chain down to 1x1 from 8x8 or larger ends at
56 mod 128 (the 8x8, 4x4, 2x2 and 1x1 levels are 32 + 8 + 8 + 8 bytes).

**128, not 256, is chosen rather than read off the disc**: every multi-level
face here also ends at 184 mod 256, so a 256-byte boundary pads the same 72
bytes on every file. 128 is the RSX's texture address alignment, the boundary a
face is bound at. Nothing on the disc tests a face under 128 bytes either; the
reader pads one all the same.

**What the unaligned reading drew.** Read without the padding, face `n` starts
`72 * n` bytes early. On `DXT1`, 8 bytes a 4x4 block, that slides it sideways by
`36 * n` texels, with the previous face's last columns showing at its left
edge. Sol 2's sky is the only racing circuit's sky with a mip chain, and it drew
a hard-edged vertical band of the wrong sky (about 60 px wide at 1440x816, from
the top of the frame to the horizon) where face 1 or 4 met its neighbour; the
glass reflections (`mesh::sky_paraboloid`, built from the same faces) carried it
too.

The picture-free check is the side seams, around the horizon ring `+Z, +X, -Z,
-X` (faces 4, 0, 5, 1), each face's last column against the next face's first,
as mean absolute difference a channel:

| Sky | Unaligned (worst of four) | Aligned (four seams) |
| --- | --- | --- |
| `12_sol_2` | 39.21 (11.85, 39.21, 33.06, 25.35) | 2.62 (2.20, 2.62, 1.89, 1.92) |
| the other eleven racing skies, `ZoneSky.gtf` | unchanged (single-level) | 0.70 (Amphiseum) to 6.76 (`02_track`); `ZoneSky` 3.44 |
| `fe/rank/textures/env.gtf` (probe) | 45.39 | 1.57 to 2.43 on three seams; `-X` to `+Z` 32.07 |

`gtf_ground_truth.rs` asserts the four seams at 8.0 or better on every sky a race
draws (each circuit's `sky.gtf` but the Zone arenas', plus `ZoneSky.gtf`, named
from `oag_hd`), that the last face's chain ends at the declared data's end on
all 23, and that 8 carry a mip chain;
`gtf::tests::a_mip_chained_cubemap_reads_each_face_from_its_aligned_start`
pins the stride on an 8x8 four-level fixture, which the unaligned reading also
accepted as `6 * 56 + 360` bytes. The zenith meets all four sides on Sol 2 at
1.5 to 1.7.

### Sol 2's nadir is stored flipped, and is drawn as stored

Under the GL face table (`mesh::sky_cube`'s `FACES`) Sol 2's nadir meets none
of the four sides (37 to 60); flipped top to bottom it meets all four (1.4 to
1.7). It is the only sky that does: the tool-made probes close their nadir
under the GL table (3.85, against 15.7 flipped), which confirms the table, and
the other racing skies either have a blank nadir (0.0 either way) or a nadir
that meets no side either way (`02_track` 65, `05_ubermall` 110). So this reads
as how the art was authored, and the RSX samples the face as stored: nothing is
changed. Below the horizon it is under the circuit; in a sky-only render it
shows as a crease. **Not checked against a frame of the original.**

### The Zone arenas' skies are another layout

`zone_1`, `zone_2`, `zone_4` (byte-identical) and `zone_3` do not meet under the
GL face table at all: the zenith misses its four sides by 205 (`zone_3`: 107)
and the side ring by 66.54 (`zone_3`: 32.25). Their faces do meet, in another
arrangement: on `zone_1` face 0's top row is face 3's bottom (0.12), face 1's
top face 2's bottom (0.66), face 2's left column face 4's top (0.59), face 1's
right face 5's left (1.34). Which convention that is has not been worked out,
because a Zone race, which is what the arenas (`oag_hd::race::ZONE_TRACKS`)
host, draws `Data/Tex/ZoneSky.gtf` **instead** of the arena's own
([zone-sky.md](../ghidra/functions/ps3-hdfury-eu/zone-sky.md)). Whether any
other mode can put an arena's own sky on screen has not been checked; if one
does, this layout needs reading first, and our `sky_cube` would draw it with
mismatched seams today.

### 2048 and Omega: checked, differs

Neither ships Sol 2's sky as one cubemap file. 2048 (DLC1 `environments/Sol_2/`)
and Omega (`environments/12_sol_2/`) draw it as `skycube.rcsmodel` over six
separate flat face textures, `SkyCube/SkyCube_Sol2_1..6` (`.gxt` and `.gnf`), so
there is no face offset to get wrong and no band. Omega also ships a
`skyCube.gnf` cubemap; the `.gnf` reader refuses cubemaps and nothing draws it
([gnf.md](gnf.md)). Its chain is the `.gnf` micro-tile layout, not this RSX
alignment.

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

Implemented as `oag_mesh::mesh::Texels`, chosen in
`oag_mesh::mesh::rcs::skin::blocks` and uploaded by
`oag_mesh::mesh_render::texture::upload`.

## What this was read for

[The HUD](hd-hud.md). Its twelve textures are nine `DXT45`/`DXT23` atlases, and
`crates/raceplay/tests/hd_hud_ground_truth.rs` now checks each of the **1,029 HUD
sprites' source rectangles against the dimensions of the texture it names** -
a check that tests both readings at once, since a layout misread puts a
sub-rectangle outside its texture and a dimension misread fails to contain
rectangles that really do fit.

**All 1,029 land, with one authored exception**: `VoiceCom0`-`VoiceCom7` take a
62x64 patch from `V="1"` of a 64x64 image, one texel past the bottom edge. 32 of
the 56 copies do it and the rest use `V="0"`, which is how a duplicated fragment
diverges. It is pinned in the test rather than tolerated.

## Not done

- **Where a ship's ambient shadow is *drawn*.** The 9 `B8` files decode; what
  places the quad under a craft, at what size and blend, is a renderer
  question this format layer says nothing about and nothing here does yet.
- **A cubemap's mip levels.** `face_range` addresses one and nothing decodes one,
  the same gap the flat textures have.
- **`remap`'s packing against the executable.** All three of the disc's words
  are read and acted on - see [the section above](#remap-is-a-per-channel-source-and-force-table-and-it-is-read-now)
  - but the bit layout is corroborated by what it predicts about the files,
  not by a decompiled read of the code that writes the register. That is the
  gap between confidence 85 and higher, and it is the same gap the Morton
  address function has.
- **Mip levels of a cubemap, and of a texture that declares a pitch.**
  `face_range` addresses the first and nothing decodes one; the second is 5
  files on the disc and takes the decode-and-box-filter path. Flat, tightly
  packed chains are bound as the disc stores them - see [the section
  above](#the-mip-chain-is-the-discs-and-a-box-filter-is-not-it).
- **The front end's `<Image>` widgets.** `oag_render` uploads a `.gtf` now -
  HD's HUD samples ten of them, and an HD craft and circuit are painted from
  their materials' own textures through
  [`oag_mesh::mesh::rcs`](rcsmodel.md#the-texture-a-material-paints-with) -
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
