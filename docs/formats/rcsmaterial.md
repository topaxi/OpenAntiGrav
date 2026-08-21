# `.rcsmaterial`: not one shader, a container of variants

**1,632 files, 104 MiB, and the thing that decides what a surface's *second*
texture is for.** A `.rcsmodel` material record names two `.gtf` paths - one at
`+0x58` and one at `+0x78` - and [rcsmodel.md](rcsmodel.md) has carried "one
slot, at least four uses: a normal map, an emissive map, a coverage mask and a
lightmap" since the record was read. This page is the first reading of the file
that decides which.

Read 2026-08-18 with [`scripts/ps3-sho.py`](../../scripts/ps3-sho.py), which is
where every number here comes from:

```sh
scripts/ps3-sho.py material data/images/hdfury-ps3-eu-dec.iso \
    /data/environments/talons_junction/materials/track_surface.rcsmaterial
scripts/ps3-sho.py materials data/images/hdfury-ps3-eu-dec.iso talons_junction
```

## The container

```text
+0x00  u32  variant count
+0x04  u32  offset of the variant table
+0x08  u32  offset of a table of word offsets into it
```

The variant table is **0x40 bytes per record**, and `count * 0x40` lands exactly
on the third word's table on the files measured. A record holds two hash-shaped
words, two small counts, and then `(offset, length)` pairs that point at `SHO`
blocks inside the same file.

**Confidence 70 on the container and 85 on what it implies.** The record's
fields are not all read - what the two leading hashes select is open - but the
count, the stride and the fact that a variant names its own programs are all
checked: `etched_glass_tech.rcsmaterial` declares 70 variants and carries 84
framed `SHO` blocks, 20 of them fragment programs.

## Why "one slot, four uses" could not be read before

**A material is twenty fragment programs, and they disagree about which texture
unit a sampler sits at.** `track_surface`'s twenty put `DiffuseTexture` at unit
0 twelve times and `NormalTexture` there four times; at unit 1 they put
`NormalTexture` nine times, `lightmap` three and `shadowMapTex` once.

So a single sampler table names roles but does not, on its own, say which role
belongs to slot 1 of *this* draw - that needs the variant the draw selects, and
**what selects a variant is unread**. A majority vote across variants is a vote,
not a reading, and this page does not dress one up as an answer.

## The sampler names, by preimage

The `SHO` sampler record is `(name hash, texture unit)` and the hash is
`~crc32` - see
[renderer.md](../ghidra/functions/ps3-hdfury-eu/renderer.md#the-name-hash-is-crc-32).
Sweeping candidates against the 125 distinct sampler hashes on the disc named
**38**:

| Group | Names |
| --- | --- |
| Slots by number | `Texture1` `Texture2` `Texture3` |
| Diffuse | `DiffuseTexture` `DiffuseTexture1` `diffuseTexture` `diffuseTexture1` `diffuseTexture2` `Diffuse` `diffuse` |
| Surface maps | `NormalTexture` `NormalTexture2` `NormalMap` `Normal` `SpecularTexture` `SpecMap` `Spec` |
| Light | **`lightmap`** `shadowMapTex` `EmissiveTexture` `Emissive` `emissive` `ReflectionMap` `EnvMap` `EnvMap1` |
| Coverage | `Alpha` `AlphaMask` `AlphaTexture` `BlendTexture` |
| Other | `Ramp` `RampTexture` `Noise` `Clouds` `Dirt` `Colour` `Colour1` `smokeTexture` `smokeTexture1` |

A name that lands on a 32-bit hash is a preimage rather than a resemblance, and
each also agrees in shape: every one of these carries a texture unit and no
constant register, which is what a sampler record is.

**`lightmap` is `0x37b5db58`, 1,669 uses**, and it is the one this project acts
on - see below.

## The lightmap is identified, and nothing else in the second slot is

Four independent signals agree, and the fourth has **no exceptions**:

1. The path sits under the circuit's own `lmaps/` directory.
2. The file name ends `-lmap.gtf`.
3. The material's shader names a sampler `lightmap`.
4. **Every chunk whose second texture is one of those declares a `lightmapUV`
   vertex attribute** - 76 of 76 on Talon's Junction, 128 of 128 on
   `amphiseum`, and **zero** chunks anywhere with an `lmaps/` second texture and
   no `lightmapUV`. The attribute name is the `.rcsmodel` vertex declaration's
   own, read separately and long before this page - see
   [rcsmodel.md](rcsmodel.md#the-names-and-they-are-mayas).

Signal 4 is the discriminator: it is a correspondence between two files that
were decoded independently, and a wrong reading of either would break it.

**What is *done* with a lightmap is an assumption and is stated as one.**
Multiplying it into the diffuse is the conventional reading and this project
applies it; nothing here reads the microcode that would confirm the operation,
and the load report says so per race.

### How much of the disc it covers

**3,584 chunks over 12 circuits, and 217 distinct atlases.** Not evenly, which
is why one circuit is not a measurement:

| Circuit | Lightmapped chunks | | Circuit | Lightmapped chunks |
| --- | ---: | --- | --- | ---: |
| `12_sol_2` | 638 | | `10_sebenco_climb` | 294 |
| `05_ubermall` | 501 | | `15_anulpha_pass` | 290 |
| `01_vineta_k` | 462 | | `04_chenghou_project` | 227 |
| `02_track` | 348 | | `03_track` | 208 |
| `tech_de_ra` | 311 | | `amphiseum` | 128 |
| `modesto_heights` | 101 | | **`talons_junction`** | **76** |

The four Zone circuits author **none**, and Talon's Junction - the circuit
`oag_hd::race::DEFAULTS` boots - has the fewest of any that do. On it the
change is 1,028 pixels of 1,175,040 at the starting grid; on `12_sol_2` it is
**35,693**, and the difference is visible as shading on the overhead structures
rather than flat grey. Measuring on the default circuit alone would have read
as "the feature does nothing".

**The other three uses stay unwired.** An emissive map, a normal map and a
coverage mask all need the variant selection above, and
`mageffect08_floor` is the case that shows why guessing would be wrong: its
second texture is `mag_emiss_talons.gtf`, which reads as emissive, while the
majority sampler at unit 1 across its twenty fragment programs is `Normal`.

## What this explains about the picture

**A surface can be painted with a gradient because a gradient is genuinely its
first texture.** `etched_glass_tech` names
`dds/dc_iridescent_gradient.gtf` - **512x16**, a ramp - at `+0x58` and
`dds/glass_etched_tech.gtf` - 1024x512, the etched pattern - at `+0x78`. Six
chunks and 4,208 triangles of Talon's Junction's glass walkways are painted
with the ramp, correctly decoded and correctly bound, because the pattern is in
the slot nothing samples. `mageffectloop` is the same pair the other way round.

So "the glass draws a gradient" is not a colour bug, a wrong texture or a
swizzle: it is the second slot, and closing it needs the variant selection.

## What selects a variant: the key is read (2026-08-20)

**A variant is keyed by the two leading hashes of its own record, and both are
`~crc32` of a string the executable builds.**

    variant = the record whose ([0], [1]) == (hash(vertex class), hash(feature permutation))

`[0]` is the **vertex-processing class**. Four exist disc-wide and all four
preimage to NUL-terminated strings in `EBOOT.elf` at `0x7a3560`:

| Hash | Class | Variants | Materials |
| --- | --- | ---: | ---: |
| `0x7893d2ec` | `Static` | 17,333 | 631 |
| `0xd29c9ee2` | `StaticQuake` | 7,797 | 171 |
| `0xdd70bfd5` | `RigidBody` | 4,389 | 153 |
| `0xa9edfe7e` | `StaticUncompressed` | 1 | 1 |

Those four counts are `DATA00`'s; the disc-wide figures are in the note below.

`[1]` is the **feature permutation**, and its name is the concatenation of the
enabled feature tokens in one fixed order. The tokens are the twenty
NUL-terminated strings at `0x7a3460` -

```text
ShadowToAlpha HalfBright Sun ShadowMap FalseLight ZoneMode ZoneTrans
NoAlbedo SVC1 SVC0 IBL Ambient IleVertex IleLightmap
Spot0 Spot1 Spot2 Spot3 ZAlphaOnly AmbientShadow
```

- plus `SunOcclusionLightmap` and `SunOcclusionVertex` at `0x7a3588`.

**Confidence 96.** Every one of the **143 distinct `[1]` values across all
29,520 variants of the 693 materials in `DATA00.PSARC`** is reproduced exactly
by `~crc32` of such a concatenation - 100 %, no exceptions. (`DATA00` is where
that sweep ran; the whole disc carries **1,632 materials and 76,358 variants**
across seven archives, and `oag_formats::rcsmaterial`'s ground-truth tests
confirm every *structural* claim at that larger scale - no class outside the
four, no repeated key in any file, no program offset carrying two different
content hashes. The per-class census disc-wide is `Static` 43,370,
`StaticQuake` 19,257, `RigidBody` 13,724, `StaticUncompressed` 7.) 143 independent 32-bit preimages is not
a resemblance. Three further checks make it self-consistent rather than lucky:
the pairwise token-precedence graph over all 143 names is **acyclic** (19
tokens, 86 edges), so one canonical order exists; the executable ships a built
name as a literal at `0x7a17d0` - `HalfBrightAmbientSunSpot0SVC0`, which is
variant #5 of `track_surface.rcsmaterial`; and `(h0, h1)` is unique within every
file, **29,520 of 29,520**, so the pair is a key and not a filter.

**`0x868f8229` is `SpuVertexColours`**, which closes the question
[renderer.md](../ghidra/functions/ps3-hdfury-eu/renderer.md) leaves open under
"The vertex-light constants are read": that attribute is a stream the SPU
writes, not one a `.rcsmodel` carries, which is why none declares it. `SVC0` -
what the disc's static geometry uses - means "no such stream".

### Which half the chunk decides

Confidence 75, deliberately lower: this is a correlation measured over the
assets plus a string table, and **no code that computes the key has been read**.
Counts are exact over all 29,520 variants.

| Implication | Holds | Exceptions |
| --- | ---: | --- |
| `IleLightmap` -> a `lightmapUV` attribute | 5,796 / 5,796 | none |
| `IleVertex` -> the colour set `0x1aaf7631` | 4,464 / 4,464 | none |
| `SVC1` -> `SpuVertexColours` | 13,026 / 13,026 | none |
| `ShadowToAlpha` -> the `shadowMapTex` sampler | 6,672 / 6,672 | none |
| `IleLightmap` -> a `lightmap` sampler | 5,548 / 5,796 | **248, unexplained** |

The converses hold with **named** rather than residual exceptions: a
`lightmapUV` without `IleLightmap` occurs on exactly 360 variants and every one
is the standalone permutation `SunOcclusionLightmap`; the colour set without
`IleVertex` on exactly 360, every one `SunOcclusionVertex`. The 248 residue is
12 emissive/billboard materials carrying a lightmapped permutation whose shader
never samples one.

So the **chunk-determined** axes are the ones the model file already answers -
`lightmapUV` -> `IleLightmap`, colour set -> `IleVertex`, neither -> `Ambient`,
and `SVC0` for anything the SPU does not write. The **frame-determined** axes
are `HalfBright` vs `ShadowToAlpha`, `Sun`, `ShadowMap`, `Spot0..3`,
`ZoneMode`/`ZoneTrans`, `IBL`, `NoAlbedo`, `FalseLight` and the four standalone
names.

**Two hypotheses are refuted, not merely unproven.** `track_surface` variants
**#7, #17 and #27** declare the identical attribute set
`{position, normal, tangent, Uv1, lightmapUV}` and differ only by
`ZoneMode`/`ZoneTrans`, so **no function of a chunk's declaration** - exact set,
best subset, or an attribute-input-mask compare - can pick a unique variant. Nor
is the key in the asset: `track.rcsmodel` contains none of the candidate hashes.

### The record, re-framed

```text
+0x00  u32  variant count        +0x04  u32  offset of the variant table
+0x08  u32  offset of a pointer-relocation list      +0x0c  u32  0
```

Each record is `0x40` bytes of sixteen big-endian `u32` (the last record's
trailing zeros are elided, so `tab2 == tab + count * 0x40 - 0x10` on 693/693):

```text
[0]  hash of the vertex class     [1]  hash of the feature permutation
[2]  vertex-side binding count    [3]  fragment-side binding count
[4]  vertex `SHO` offset          [5]  fragment `SHO` offset
[6]  its size                     [7]  its size
[8]  vertex program content hash  [9]  fragment program content hash
[10] u16 (slot, reg) table        [11] pointer into the file's name list
```

`[8]` is constant across every set of variants sharing a `[4]` (25,040 of
25,040) and `[9]` likewise across `[5]` (10,276 of 10,276) - they are content
hashes, which is why variants share blocks. `[11]` advances by exactly
`4 * ([2] + [3])` between records on 29,520 of 29,520, and `[10]`'s table holds
`2 * ([2] + [3])` u16 entries on 29,520 of 29,520, with `reg >= 0x8000` meaning
texture unit `reg & 0xff`. The table at `+0x08` is a **relocation list**, not an
index: a count followed by that many word offsets whose contents are file
offsets - on `track_surface`, `2 + 70 * 4` entries, the header's two pointers
plus each record's `+0x10, +0x14, +0x28, +0x2c`.

### One slot, one variant

**Every chunk sharing a material slot needs the same variant.** Measured over
all 123 `.rcsmodel` of `DATA00.PSARC` and the 3,566 material slots they use:
**zero** slots serve chunks whose chunk-determined keys differ.

That is an architectural fact rather than a curiosity. The chunk-to-material
map is many-to-one, so a slot *could* have had to be two things at once, and
`oag-render` binds one texture bind group per material slot rather than per
draw. Because it never does, a per-material variant selector can ride in that
bind group; if it ever did, the selector would have to move to the draw call.
`a_material_slot_never_needs_two_different_variants` holds the line.

### The permutation word is read, and so is the lit race pass (2026-08-20)

**The key's second half is a twelve-bit word of fields**, not one bit per
token, and `0x003f0ff8` is the whole accessor:

```text
3f0ff8: slwi 3,3,2 ; lwz 9,-21696(2) -> 0x00d3e220 ; lwzx 3,9,3 ; blr
```

so the permutation hash is literally `table[word]`. `0x003f1028` is the startup
loop that fills that table: it runs a mask from **0 to 4095**
(`cmpdi 7,28,4096` at `0x3f11d0`), builds each name, hashes it (`bl 0x5a2090`
then `not 3,3`) and stores it. The loop's own bit tests give the layout:

| Bits | Meaning |
| --- | --- |
| 0 | `Sun` |
| 1-2 | `Ambient` / `IleVertex` / `IleLightmap` / `IBL` |
| 3-4 | `Spot0` / `Spot1` / `Spot2` / `Spot3` |
| 5 | set `ShadowToAlpha`, clear `HalfBright` |
| 6 | `ShadowMap` |
| 7 | `FalseLight` |
| 8 | `ZoneMode` |
| 9 | `ZoneTrans` |
| 10 | `NoAlbedo` |
| 11 | set `SVC1`, clear `SVC0` |

`ZAlphaOnly`, `AmbientShadow`, `SunOcclusionLightmap` and `SunOcclusionVertex`
are **not in this word**: `0x003f1300` hashes each standalone into
`0x00d3e220 + 0x4a20`, past the `0x4000` bytes the table occupies.

**The concatenation order is now read too**, from the append sequence at
`0x3f111c`-`0x3f11bc`, and it agrees with the order derived a round earlier
from a precedence graph over the 143 names. Two derivations sharing no method
agreeing on one order is worth more than either alone.

**What an ordinary lit race pass sets: `Sun`, and nothing else it decides.**
The word is `1` - `HalfBright` and `SVC0` are the clear sides of bits 5 and 11
and `Spot0` is field zero - which names `HalfBrightAmbientSunSpot0SVC0`, the
one permutation the executable ships as a *string literal* rather than
building. Add the chunk half on top: `| 0b100` lightmapped, `| 0b010`
vertex-coloured. Confidence 95: `Job RenderTrackWithLights_zWriters`
(`0x408fa8`) and `_blended` (`0x4074e0`), reached through slot 3 of each job's
vtable, never write bits 5, 6 or 7; `ShadowToAlpha` belongs to the shadow
compositing pass (`li 9,32` at `0x405d48`) and `ShadowMap` only to the model
and ship path (`li 6,64` at `0x3ea58c`) - which is why no shipped variant pairs
`ShadowMap` with an `Ile*` token.

Checked against the disc: sweeping all 4096 words gives **4096 distinct
hashes, no collisions**, and of the **143** distinct permutation values across
every material, **138 come from the word and 5 from the standalone names,
leaving none unmatched** (`every_shipped_permutation_is_one_this_reading_can_build`).

**What that sweep does and does not prove.** It is *invariant under any
permutation of which bit means what*: if `name_for'(v) = name_for(pi(v))` for a
bijection `pi` on 0..4095, the generated name set - and so the hash set - is
unchanged. So the sweep is decisive about the **token vocabulary**, the **field
structure** (two two-bit selectors rather than four independent bits) and the
**concatenation order**, and contributes **nothing** about numeric bit
positions. Those rest on the loop's own bit tests and on four independent
agreements from the pass code, which is why the layout is recorded at
confidence **93** while the vocabulary and order sit at 97. And even at 97 it is
a *sufficiency* check: it proves every shipped name is generated, not that no
other generator would also generate them.

The practical consequence is that `LIT_RACE_PASS = 1` is a **code reading**, not
a disc measurement. A permuted layout would still reproduce all 143 names while
putting the lit race pass at a different word.

> **A retracted reading.** An earlier round of this page described a "pointer
> table" of 21 one-bit tokens at vaddr `0x8b7f08`. **There is no such table.**
> Those 24 consecutive words are a slice of the **TOC** at base `0x8bd3c4` -
> the token strings sit together in `.rodata` because they were declared
> together, so their TOC entries sit together too. The entry immediately before
> them points at `0x00d3e220` itself, and two entries after them are `time` and
> `viewProj`. The "empty string at bit 3" was the empty string the loop's
> *clear* chain loads for every optional token. A selector built on that order
> would have picked the wrong variant every time, and it shipped in
> `oag_formats::rcsmaterial` for one commit before this read caught it.

### The frame's pass order

Thirteen job names in order at vaddr `0x7b1008`, with the run function of each
reached through slot 3 of its vtable:

```text
PrecomputeTrackFrameData | Preprocess Visibility Fence | RenderBillBoards
RenderModelShadowMaps | RenderSpotShadowMaps | RenderTrackReflect
RenderTrackRefract | RenderTrackWithLights_zWriters (0x408fa8)
RenderModelShadowsOnTrack (0x4053e0) | RenderModelAmbientShadowsOnTrack
RenderTrackWithLights_blended (0x4074e0) | RenderShips (0x3ea368)
ClearTrackVisibilityFlags
```

Addresses quoted elsewhere as `0x7a1008`/`0x7a3460` are **file offsets**; load
addresses are `+0x10000`.

### The header declares its inputs - and does not name a lighting family

A `SHO` block's header carries its parameter and sampler tables, so what a
program is *fed* reads without decoding an instruction:
`oag_formats::rcsmaterial::Declared` parses them, and the three tables abutting
exactly is its framing check.

**Splitting the lighting families on that was tried and does not work.**
Classifying each drawn chunk of Talon's Junction by its *resolved* variant's
fragment declaration - samples `lightmap` / takes `constantAmbientColour` /
neither - gives 322, 144 and **447**, and that third bucket is not a family.
Alongside the billboards and scanline screens it holds `track_wall` (22
chunks), `glasstest` (33), `simplefogdiffuse` (31) and `diffusewithalphachannel`
(29), which are ordinary lit surfaces.

The reason is structural: **a surface lit only by the interpolated per-vertex
term declares neither of those two, because both of its light sources are
interpolators rather than uniforms**, and an interpolator is not in the
declaration table. Telling a lit surface from an emissive one is the question
of whether `f[TC1]` reaches the albedo multiply, which is an instruction-level
fact. Treating the bucket as unlit would have drawn the track walls at full
albedo.

So `Declared` ships with `samples_lightmap` and `takes_constant_ambient`, which
are facts, and deliberately **no** `lighting()`, which would invite a caller to
believe the header can answer a question it cannot.

Note also that this per-*variant* reading is a different measurement from the
per-*material* taxonomy in the session reports: 322 prelit here against 535
there, because a material may declare a lightmap in some variants and not in
the one an ordinary lit pass binds. The 322 sits beside the 327 chunks that
declare a `lightmapUV`, which is the consistency check.

### The fragment microcode decodes in Rust (2026-08-20)

`oag_formats::rcsmaterial::fragment` ports the decoder from
[`scripts/ps3-microcode.py`](../../scripts/ps3-microcode.py), which stays the
reference: that script established the container facts empirically, by scoring
every candidate ordering, and this reimplements the reading rather than
re-deriving it.

Three container facts it rests on, none guessable from the bytes:

- **A fragment dword is stored with its 16-bit halves swapped** - a word is
  `(u16 at +2) << 16 | (u16 at +0)`. Read as a plain big-endian `u32` the
  stream decodes to garbage.
- **An instruction whose source selects an inline constant is followed by that
  constant's 16 bytes**, so the stream advances 32 rather than 16 there.
- The code's byte length is at the program sub-header's `+0x00`, its offset at
  `+0x10`.

**Checked against the reference rather than only against itself.** On
`DATA00.PSARC`'s 693 materials the two agree on **10,276 of 10,276** fragment
blocks reaching a clean `END` at exactly their declared length, and on **330**
instructions carrying opcode `0x3e`. Diffing three real blocks instruction by
instruction - `track_surface` #7 and #9, and `detonator_ship_rich_iridescent`
#2 - gives identical opcode and destination sequences, differing only where the
reference prints its `op3B`/`op3D` placeholders and this leaves them unnamed.

Across all seven archives: **37,461 of 37,461 blocks clean, 1,920,588
instructions** (`every_fragment_program_on_the_disc_decodes_to_a_clean_end`).
That check is sharper than it looks - a wrong stride, a mis-detected inline
constant or a wrong end bit desynchronises the stream within a few
instructions, and the block then runs past its declared length instead of
stopping on an `END`.

**Exactly three opcodes in shipped code are outside nouveau's table**: `0x3b`
(86,664 uses), `0x3d` (28,634) and `0x3e` (1,155). They stay **unnamed** here
rather than guessed - `renderer.md` reads `0x3b`/`0x3d` as normalise/rsq
helpers from their position in the stream, which is a reading of a use rather
than of an opcode. A fourth appearing would mean the corpus grew or the stride
is wrong somewhere, and the test says so.

### What the microcode does and does not settle

With the decoder there, the natural next question is whether a surface takes
the scene's light or is emissive - which the header could not answer. It has a
dataflow answer and **the dataflow does not settle it either**.

`Program::output_lit_by` is a forward taint over the register file, per
channel, with **taint blocked at a texture lookup**: a sampled value depends on
the texture's contents, not on the magnitude of the coordinate that addressed
it, so blocking there turns "did this interpolator reach the picture" into "was
it *combined* into it". That primitive is right, and checked against two blocks
read by hand - `track_surface` #8 and #9 both report `TC1`, their per-vertex
light, and **not** `TC4`, their albedo coordinate
(`the_lighting_dataflow_agrees_with_the_blocks_read_by_hand`).

**As a lit-versus-emissive classifier it still fails.** Taking "the output is
combined with some interpolator other than `FOGC`" over Talon's Junction's
drawn chunks gives 745 lit, 168 unlit - and the split is wrong at both ends.
`cf_billboard1` (53 chunks) and `scanlinebillboard` (49) come out **lit**,
because a billboard is modulated by an interpolated term too - a scroll or a
fade - which is not scene light. `track_wall` splits 11 lit against 22 unlit.

The reason is the same one that defeated the header, one level down: **which
interpolator carries scene light is a per-material fact**, not a property of
the program's shape. It is `f[TC1]` on `track_surface` and `f[TC0]` on
`bluemetal`, and no generic rule over the instruction stream distinguishes a
light term from any other interpolated modulation.

So two general rules have now been tried and rejected on evidence. What is left
is per-material work: read a material, name what each of its interpolators
carries, and record it - the way `track_surface` and
`detonator_ship_rich_iridescent` already are in
[renderer.md](../ghidra/functions/ps3-hdfury-eu/renderer.md). The decoder makes
that reading cheap; it does not make it automatic.

### Still open

- **The frame-determined half.** A renderer can pick the chunk-determined axes
  today; it cannot yet know which of `HalfBright`/`ShadowToAlpha`, `Sun`,
  `ShadowMap`, `Spot0..3` or the Zone pair an ordinary race pass sets. The
  anchor is surfaced: the `Job Render*` strings at `0x7a1048`-`0x7a1168`
  (`Job RenderTrackWithLights_zWriters`, `Job RenderModelShadowsOnTrack`,
  `Job RenderShips`, `Job RenderSpotShadowMaps`, ...). Reading which flags each
  job sets is what turns the 75 above into a read.
- **`StaticUncompressed` rests on seven occurrences disc-wide** (one in
  `DATA00`) plus its preimage, and is weaker than the other three classes.
- The 248 `IleLightmap`-without-a-`lightmap`-sampler variants.

## The cloud plate paints a solid sheet, and the microcode says why (2026-08-20)

**This is what a frame comparison read as "a solid wall is missing from Talon's
Junction".** It is not missing. A two-triangle cloud quad is painting an opaque
bright sheet in front of it.

Found by an **ID render** - every draw call tinted by its chunk index, the
resolve passed straight through with no exposure or gamma, and the pixels read
back. The huge white expanse filling the left of a start-line frame decodes to
chunks 182, 183 and 184, all `talons_junction/materials/clouds.rcsmaterial`,
2 to 60 triangles each. Dropping just those chunks restores the roadway's lane
markings and the trackside structures behind them, so nothing there was ever
unaddressed, unplaced or culled - it was covered.

**Why it paints solid.** `oag_render`'s PS3 path takes a blended surface's
coverage from its first texture's alpha channel. `clouds_new.gtf`'s alpha is
**255 everywhere** (measured per channel: `r`/`g`/`b` 156..255 mean 219,
`a` 255..255), so `SRC_ALPHA`/`ONE_MINUS_SRC_ALPHA` is a no-op and the quad
replaces what is behind it with a bright grey plate.

**What the disc actually does**, from the material's own fragment microcode
(`scripts/ps3-microcode.py fp-file clouds.rcsmaterial`; blocks #2 and #3 agree,
and #3 is the full lit-and-fogged one):

```text
@0x1f  TEX H4.xyz, R3.zwzz unit1     ; cloud mask.gtf -> RGB
@0x2e  TEX H3.x,   f[TC4].zwzz unit0 ; clouds_new.gtf -> one channel only
@0x30  MAD H2.xyz, H5, H4, H2        ; the mask is the COLOUR
@0x34  ADD H0.xyz, R0, H2
@0x35  MOV H0.w,   H3.xxxx END       ; the ALPHA is unit 0's .x
```

So the roles are **the opposite of the slot names**: unit 0 - the sampler named
`diffuse` (`0x515e298e`, per [renderer.md](../ghidra/functions/ps3-hdfury-eu/renderer.md))
carrying `clouds_new.gtf` - contributes **only its red channel, as alpha**, and
never its RGB; unit 1, `cloud mask.gtf`, is what supplies colour. The channel
statistics corroborate the direction: the "mask" is the one with a silhouette
in it (0..255, mean 58, **60 % of texels under 32**) while the "diffuse" is a
flat bright plate with no silhouette at all (156..255). A reader that assumes
"first texture is the picture, its alpha is the coverage" gets both halves
wrong on this material.

**Two consequences worth carrying.**

1. **A see-through material whose first texture has uniform alpha is a
   contradiction, and it is measurable.** Six of Talon's Junction's 21
   see-through materials are in that state - `clouds`, `tunnel_fx_glass`,
   `dc_lightcone`, `etched_glass_tech`, `emissive_bloom` and `mageffectloop` -
   each asking for a blend whose only alpha source is 1.0. Every one of them
   currently paints an opaque sheet. That count is a ready-made ratchet for
   whatever fixes this.
2. **This is the sixth role for the second slot**, after normal map, emissive
   map, coverage mask, lightmap and the glass family's facing ramp below - and
   the first one read end to end out of the microcode rather than inferred
   from a file name. It is also the one that most changes the picture.

**Implemented 2026-08-21, and one half of the prescription below did not land
as scoped.** `rcsmaterial::fragment::Program::output_texels` is the
classifier this section called buildable: a swizzle-aware forward taint
(`fragment.rs`'s doc comment) that answers, per output lane, which texture
unit and channel reaches it - honouring swizzles (the difference this section
is named for) and keeping `H`/`R` register files apart, both measured against
this same `clouds.rcsmaterial` block. `mesh::rcs::skin::roles` turns a
positive trace into `mesh::slots` bits per material (`ALBEDO_FROM_SECOND`,
`ALPHA_FROM_SECOND`, packed with the already-shipped `SECOND_IS_LIGHTMAP`),
carried per vertex and decoded in `mesh.wgsl`'s `fs_main`. Verified visually:
Talon's Junction's cloud plate now draws as a translucent wisp with the
roadway and trackside structures visible through it, where it used to paint
the opaque sheet described above; a sanity sweep of Wipeout Pulse and three
further HD circuits (Vineta K, Sol 2, Talon's Junction) at one tick each
showed no corruption from the change.

**The alpha did not come from a second UV set - it is sampled at the first
texture's own coordinate instead**, which `mesh.wgsl` states plainly as an
approximation, not a reading: the cloud plate's own program builds both
units' coordinates from the same interpolator and a chunk in this role
declares no second coordinate set to use instead, but a material that tiles
its two textures at different scales would come out wrong under this rule -
visibly mismatched rather than misplaced, and not yet measured on any such
material. Only a *positive* trace is acted on, **per lane, independently for
colour and alpha** - a colour merge that comes out `Texel::Mixed` leaves the
albedo at the first texture untouched even when the alpha lane traces
cleanly. Checked directly on the glass family below: both
`glass_texture_customr`'s and `etched_glass_tech`'s colour merges come out
`Mixed` (unsurprising - their combine is the unresolved `DP3_SAT`/Fresnel
chain that section describes), so neither gets `ALBEDO_FROM_SECOND`; but
`etched_glass_tech`'s alpha traces to unit 0 channel 0, which *does* move its
alpha channel off the `slots::DEFAULT` assumption of channel 3 - a real
reading, not a guess, and not the glass family's "not solved" combine, which
is about colour. Normal map, emissive map and coverage mask - three of the
five other roles this slot plays - are still not selected by anything.

## The glass family's second slot: traced, not solved (2026-08-20)

**Why Talon's Junction's reflective glass and its crowd-filled tunnel tubes
render flat instead of tinted and populated.** Prompted by a side-by-side
against a real RPCS3 capture, which is a stronger prompt than a code-read
alone: the annotated frame showed the tube's glass as plain grey where the
reference shows green-tinted glass with a crowd visible through it.

**The second texture is real and unloaded, for four glass materials measured
this way.** `oag_formats::rcsmodel::Material::second_texture` is populated on
`glass_texture_customr` (`j_tower_glass_r.gtf`), `etched_glass_tech`
(`glass_etched_tech.gtf`, matching "What this explains about the picture"
above), `tunnel_fx_glass` (`tunnel_fx_diffuse.gtf`) and `ds_booth_glass` (a
real `-lmap.gtf`, so `Material::lightmap()` does return it there). Only the
lightmap case is ever loaded: `mesh/rcs/skin.rs::skin` calls
`Material::lightmap()`, which answers `None` for all four **unless** the path
matches the `lmaps/`+`-lmap.gtf` pattern - so `j_tower_glass_r.gtf`,
`glass_etched_tech.gtf` and `tunnel_fx_diffuse.gtf` are never even read off
the disc today, let alone bound.

**The resolved variant - not a majority vote across all twenty - names the
sampler that wants it.** Using the same `Class::Static` /
`LIT_RACE_PASS`-keyed resolution `skin.rs::variants` already does, then
reading that one fragment block's own declaration
(`rcsmaterial::Declared::parse` at the resolved `Variant::fragment.offset`):
`glass_texture_customr`'s and `etched_glass_tech`'s resolved variants both
declare `unit 0 = 0x3bdc0403` (`Texture1`, the first `.gtf`) and
`unit 1 = 0x9edd3243` - unnamed, and the same hash `ds_booth_glass`'s
resolved variant declares at unit 2 alongside its lightmap at unit 1. So the
second slot's consumer is real and it is not the lightmap: this is the fifth
distinct role recorded for this slot family, after normal map, emissive map,
coverage mask and lightmap.

**The fragment program samples it through a computed coordinate, not a raw
vertex UV - traced instruction by instruction on `glass_texture_customr`'s
resolved block (`scripts/ps3-microcode.py fp-file`, block 5 at `0x2db0`).**
Unit 0 samples `f[TC5].zw` directly - an ordinary vertex UV, confirmed against
the paired vertex program (`vp-file`, same block index), which writes
`o[TC5].zw = v[2].xxxy` straight from an attribute. Unit 1 samples a register
built over roughly ten instructions from a saturated dot product
(`DP3_SAT R0.y, R3, R0.yzww`) and several `MAD`/`ADD` remaps, ending
`ADD R3.x, R0.wwww, {0.25, ...}` immediately before the `TEX ... unit1`. That
shape - a scalar coordinate built from a clamped dot product, offset by a
constant - is what a Fresnel-style facing term feeding a lookup looks like,
not what a diffuse UV does, and it would make `0x9edd3243` a real, if
unnamed, candidate for the `FacingRamp` input already listed as absent in
`HANDOVER.md`'s per-material row.

**What broke that read, and what a closer look found instead: `c[206]` is not
an engine input at all - it is a static zero, baked into the file, dead in
the combine.** `o[TC1].xyz` and `o[TC2].xyz` - both operands the fragment
block's early `DP3` chain reads - are written `MOV o[TCn].xyz, c[206].xxxx`,
which first read as a scene-wide uniform (a camera-relative direction, say)
the material declares nowhere. It is not one. `Rsx_UploadVertexConstants`
(`0x005c176c`, confidence 88, already named in
[renderer.md](../ghidra/functions/ps3-hdfury-eu/renderer.md)) is the same raw
constant-patch mechanism that page already established for HD's vertex-light
RGBE decode - a sliding, undeclared register carrying a literal baked at
export time, not a per-frame value. Read the same way here (validated first
against that page's own `c[464] = (255, 128, 0, 2)` / `c[463] = (1, 0, 0, 0)`
before trusting it on glass): `const[462]` (`c[206]` under the `+256` rule)
is `(0.0, 0.0, 0.0, 0.0)`, in both `glass_texture_customr`'s and
`etched_glass_tech`'s resolved blocks. And it is not merely inert: `f[TC2]` is
read a second time at `@0x21 MOV R1.xyz, f[TC2]`, overwriting `R1` with that
same zero, and `R1` is never written again before
`@0x50 ADD H0.xyz, R1, H0 END` - the block's own last instruction adds this
zero directly into the final colour. Confidence 90 on "dead", checked by
exhaustive register-flow tracing rather than by pattern alone.

**The `DP3_SAT` Fresnel-shaped term is real, on different and better-attested
operands.** `o[TC3].xyz = v[1].xyzx` is a genuine per-vertex attribute (the
world normal). The chain's other side, traced through `R1`'s `.w`-lane
assembly (`f[TC0].w`, `f[TC1].w`, `f[TC2].w`), resolves to camera position
minus world position - a real, per-vertex **view vector**, using `c[209]`
(camera position, already pinned in renderer.md's ship reading) rather than
`c[206]`. Confidence 70 on this reattribution: mechanical register tracing,
not the pattern-matching the original "Fresnel-shaped" read was.

**A finding that changes what the term even is: the patched light direction
and colour feed the coordinate build too, not only the classic `N.L` and
diffuse multiply.** Decoded the `patch fslot` to `const@slot` mapping this
page and renderer.md's "sun is real" section both left unresolved - a `u32`
entry count and offset table inside the block's own sub-header, each entry a
`u16` count of `u16` code-slot indices a parameter patches - and validated it
first against `diffuse_with_specular_from_alpha`'s already-published const
slots (`0x9`, `0x13`) before trusting it on glass. On
`glass_texture_customr`'s block: `directionalLight0DirectionWorldSpace`
patches **both** the expected `N.L`-shaped slot **and** the coordinate-build
`ADD` at `@0x13`; `directionalLight0Colour` patches **both** the Lambert
multiply **and** two slots deep in the final combine (`@0x44`, `@0x48`). A
worked numeric example (a head-on normal, view and light direction) saturates
the term to `1.0` rather than the low value a grazing-angle Fresnel term
should give there - a signal this is not a plain view-facing Fresnel lookup
but something closer to "brighten where both lit and facing", gated by the
same sun-occlusion mask the diffuse path already uses. Confidence ~55 on the
coordinate build as a whole: internally consistent and now traced on real
operands, but resting on one unconfirmed opcode (`op3B`, very likely `NRM` -
a G70 instruction one generation past the NV30/NV40 headers this project's
disassembler is built from, and already glossed as "normalize" in renderer.md's
own prose without the opcode table agreeing - see renderer.md for the
reasoning). `op3D`, seen only in `etched_glass_tech`, has no hypothesis at
all.

**One formula does not cover the family.** `etched_glass_tech`'s resolved
block diverges from `glass_texture_customr`'s in code, not only in patched
constants: a third sampler unit (`0x94b2b285`, plausibly the etched detail
layer its own name implies), two parameters neither shares, `op3D` where
`glass_texture_customr` has none, and a different ending - a separate alpha
write from the diffuse sample, against `glass_texture_customr`'s single
`ADD ... END` with alpha left untouched since the diffuse `TEX`. Both share
the "`c[206]` dead, twice over" finding independently; neither shares the
other's combine logic. The same caveat renderer.md's sun-diffuse reading
already carries for `track_surface`'s minority applies here at the level of
the whole glass family, not one exception inside it.

**Not the first time this shape has turned up.** `renderer.md`'s "Ships have
no Lambert diffuse either" reads `detonator_ship_rich_iridescent` carrying "a
view-driven colour dodge and burn... and an environment lookup on a third
sampler, neither of which is a light term and neither of which `oag-render`
implements" - stated flatly, deliberately, "and nothing here fakes it". The
glass family's second slot reads as the same category of gap on different
materials, not a new one, which is why it stays unfaked here too: **loading**
`second_texture` is now real (`mesh/rcs/skin.rs::skin`, 2026-08-20 -
`Report::second_texture_loaded`, 252 of Talon's Junction's materials), but
**wiring** it into the shader on a ~55-confidence combine, per material,
would still be inventing the picture more than reading it. Loaded and
unbound is the honest middle state between "not read at all" and "guessed
at".

**What would close it, and what probably would not.** More static reading is
not expected to move the last ~15-20 points of confidence efficiently: the
gap is one unconfirmed opcode feeding a coordinate whose exact curve this
page cannot verify without either an RPCS3 register-state watch at the
`TEX ... unit1` call, or another disc material that uses `op3B`/`op3D` in a
more decomposable context than glass's does. Two things below that ceiling
are worth doing regardless of whether the formula ever closes: the
`patch fslot` to `const@slot` decoder is a durable technique - it should be
promoted out of this page's evidence trail and into
[renderer.md](../ghidra/functions/ps3-hdfury-eu/renderer.md) itself, which
already has (see above); and per-material work on the glass family, should it
become worth wiring, has to be scoped like the sun-diffuse reading already
is - `glass_texture_customr` and `etched_glass_tech` are two materials, not
one reading.

## Open

- **87 of the 125 sampler hashes**, including the three commonest
  (`0xd5e000d1`, `0x00e5b679`, `0x1f6f85a3`, ~4,400 uses each across every unit),
  which by their spread are engine samplers rather than a material's own.
- **87 of the 125 sampler hashes**, including the three commonest
  (`0xd5e000d1`, `0x00e5b679`, `0x1f6f85a3`, ~4,400 uses each across every unit),
  which by their spread are engine samplers rather than a material's own. Three
  of those three are now named - `zoneTexInner`, `zoneTexInnerNearest`,
  `zoneTexVis` - by a preimage sweep over the executable's own identifier-shaped
  strings, which is the technique that should be tried on the rest.
- **The microcode**, which is where the operation - multiply, add, replace -
  actually is.

## See also

- [rcsmodel](rcsmodel.md) - the material record, and the two texture paths
- [renderer.md](../ghidra/functions/ps3-hdfury-eu/renderer.md) - the `SHO` block
  framed, and the executable's own 124 shader programs
- [gtf](gtf.md) - the textures both slots name
