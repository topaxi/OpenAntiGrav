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

**Read further 2026-08-24**: the variant selection is closed now, and it says
this material's picture is not in the second slot either - it is on a **third**
sampler unit that a `.rcsmodel` material has no room to name. See "Talon's
Junction's missing floor is a glass floor, and its coverage is the disc's own"
below, which also measures what those six chunks actually cover.

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
across seven archives, and `oag_rcs::rcsmaterial`'s ground-truth tests
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
> `oag_rcs::rcsmaterial` for one commit before this read caught it.

### The frame's pass order

Thirteen job names in order at vaddr `0x7b1008`, with the run function of each
reached through slot 3 of its vtable:

```text
PrecomputeTrackFrameData | Preprocess Visibility Fence | RenderBillBoards
RenderModelShadowMaps | RenderSpotShadowMaps | RenderTrackReflect
RenderTrackRefract | RenderTrackWithLights_zWriters (0x408fa8)
RenderModelShadowsOnTrack (0x4053e0) | RenderModelAmbientShadowsOnTrack
RenderTrackWithLights_blended (0x4074e0) | RenderShips (0x6cdfc0: 0x3f0950 then 0x3ea368)
ClearTrackVisibilityFlags
```

Addresses quoted elsewhere as `0x7a1008`/`0x7a3460` are **file offsets**; load
addresses are `+0x10000`.

### The header declares its inputs - and does not name a lighting family

A `SHO` block's header carries its parameter and sampler tables, so what a
program is *fed* reads without decoding an instruction:
`oag_rcs::rcsmaterial::Declared` parses them, and the three tables abutting
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

`oag_rcs::rcsmaterial::fragment` ports the decoder from
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

**A fourth, `0x3c`, is in Mesa's own `nvfx_shader.h`** - `LIT_EX2_NV40` -
confirmed against the primary source 2026-09-03, and now named in
`fragment.rs`. It occurs **zero** times in shipped code (checked disc-wide,
`crates/render/examples/hd_litex2_census.rs`), so it changes nothing here;
worth recording only because "exactly three opcodes outside nouveau's table"
and "a name this project has not yet ported" are different claims, and this
was briefly the latter for an opcode that was never the former. See
`renderer.md`'s "Ships have no Lambert diffuse either" for a second `op3B`
usage shape found while tracing the specular exponent's disputed values,
and why it does not move `op3B`/`NRM` past the same confidence-70 line this
page's own account of it already sits at.

**`0x3b` and `0x3d` cross that line 2026-09-25, off a different primary
source than `0x3c`'s.** Absent from Mesa's `nvfx_shader.h` as recorded above,
both are present in RPCS3's own `rpcs3/Emu/RSX/Program/Assembler/FPOpcodes.h`
(GPLv2, independently reverse-engineered against real hardware and shipping
games): `RSX_FP_OPCODE_DIVSQ = 0x3B` ("Divide by Square Root", `a / sqrt(b)`)
and `RSX_FP_OPCODE_FENCT = 0x3D` ("Fence T?" - RPCS3's own hedge on the exact
meaning). Both were checked disc-wide with a new census
(`crates/render/examples/hd_op3b_op3d_census.rs`, all seven archives, 1,632
`.rcsmaterial` files, 76,358 fragment blocks - the same corpus size
`rcsmaterial_ground_truth.rs` already establishes):

- **`0x3d`: 59,256 uses, every single one writing destination register 63** -
  the 6-bit destination field's all-ones value, and, independently checked by
  hand on a sample, with `nvfx_shader.h`'s own `NV40_FP_OP_OUT_NONE` bit (bit
  30 of the instruction's first dword) set. Zero counterexamples across the
  full sweep - an exact, disc-wide structural invariant, not a sampled rate.
  Whatever `0x3d`'s precise hardware semantics, it writes no real destination
  on this disc, which is what "fence" would predict and what a real
  arithmetic contributor to a shading result would not do. **Confidence 90**
  on "no data effect"; the rubric's top structural band, held just below
  "Established" because nothing here traces an actual synchronisation effect
  on real hardware or in RPCS3 itself, whose own name for the opcode is
  hedged.
- **`0x3b`: 183,623 uses, splitting into exactly the two shapes `renderer.md`'s
  "op3B" section already found irreconcilable under a dedicated-`NRM`
  reading** - `DP3(v, v) -> d; op3B(v, d)` (105,800 uses, different source
  registers - `v * rsqrt(d)`, i.e. normalize) and `op3B(x, x)` (38,284 uses,
  identical source register both operands, no preceding self-`DP3` - `x /
  sqrt(x) = sqrt(x)`, the standard single-instruction square-root idiom on
  hardware with no native `SQRT`). A generic two-operand `DIVSQ(a, b) = a /
  sqrt(b)` produces both from one formula; a dedicated `NRM` cannot produce
  the second at all, which is exactly the contradiction that kept `op3B`
  below the rename line. The remaining 39,539 uses take an `Input` or
  `Constant` operand rather than two plain registers - additional argument
  variety a generic arithmetic primitive is expected to have, not a
  counter-example to the shape above. **Confidence 84**: matches the primary
  source unhedged and is structurally uniform disc-wide across both shapes,
  capped below the 85+ band because nothing traces an actual computed value
  against a known-correct oracle (no live GPU trace).

Named in `fragment.rs` and `scripts/ps3-microcode.py`'s `FP_OPS`.
`docs/rendering/pads.md`'s "Wiring attempted and stopped" thread was blocked
on exactly this naming for its pad-glow specular scalar (which, traced by
hand, turns out to use only `0x3b`/`DIVSQ`, never `0x3d`) - see that page for
what this clears.

**`0x3e` is `FENCB`, named the same day off the same header**:
`RSX_FP_OPCODE_FENCB = 0x3E` ("Fence B?", hedged by RPCS3 like `FENCT`). The
same census, extended to it, finds **3,115 of 3,115** uses writing destination
register 63, the invariant `FENCT` rests on, with no counterexample. The raw
`SHO\x08` walk in `every_fragment_program_on_the_disc_decodes_to_a_clean_end`
counts 1,155 of them because it reaches fewer blocks (37,461 against the
census's 76,358 variant programs). **Confidence 90** on "writes no real
destination", for the same reasons as `FENCT`. **No opcode in shipped HD code
is unnamed any more**, and that test now asserts it.

**The rest of RPCS3's table is named too, off that header alone**: `0x2b`
`BEM`, `0x2c` `PKG`, `0x2d` `UPG`, `0x33` `TEXBEM`, `0x34` `TXPBEM`, `0x35`
`BEMLUM`, `0x37` `TIMESWTEX`, `0x38` `DP2`, `0x39` `NRM`. None occurs in
shipped HD code, so nothing here corroborates them and nothing depends on
them; they carry no confidence score, and their operand counts stay at the
reference decoder's default of two. `0x39` being RPCS3's `NRM` is one more
reason `0x3b` never was. `0x30` and `0x32` are in neither header and stay
unnamed.

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
   see-through materials looked to be in that state - `clouds`,
   `tunnel_fx_glass`, `dc_lightcone`, `etched_glass_tech`, `emissive_bloom` and
   `mageffectloop` - each apparently asking for a blend whose only alpha source
   is 1.0, and every one of them painting an opaque sheet.

   **Corrected 2026-08-24, and the count is no longer six.** That reading took
   the *alpha channel* of each first texture; the microcode takes a channel of
   its own choosing. `etched_glass_tech`'s output alpha is
   `dc_iridescent_gradient.gtf`'s **red** - `0..214`, mean ~33, half of it under
   8 - a real varying signal and the opposite of uniform, and it is now read
   that way (`slots::alpha_channel(0)`). See "Talon's Junction's missing floor
   is a glass floor" below, where that is measured end to end. `clouds` stays on
   the list with its reason changed rather than removed: its alpha is also unit
   0's red, and *that channel* is uniformly bright (`156..255`, 0 % under 128),
   so the sheet is authored. The remaining four have not been re-measured
   channel by channel, so the honest count today is **one confirmed authored
   (`clouds`), one refuted (`etched_glass_tech`), four unmeasured** - and the
   ratchet is that recount, not the original six. **One of the four is fixed as
   of 2026-08-24**: `dc_lightcone` takes its colour from its second texture
   now, off the sampler-hash reading below, and its cones draw as gradients
   rather than noise.
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
this way.** `oag_rcs::rcsmodel::Material::second_texture` is populated on
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

## Talon's Junction's missing floor is a glass floor, and its coverage is the disc's own (2026-08-24)

**The report was "the floor is missing where the autopilot banks through the
loop; the frame shows the cloud plate's mottled grey instead of a road".** It
is neither missing geometry nor the cloud plate. The surface the craft rides on
at that point is `etched_glass_tech.rcsmaterial`, and it is drawn as an
iridescent stripe over a transparent gap because that is what the material's own
arithmetic asks for. Three separate candidates were measured out first, each
with the harness this change ships (`mesh::rcs::isolate`).

**Repro**, and every number below is from it:

```sh
cargo build --release -p oag-game
target/release/oag-game --race data/images/hdfury-ps3-eu-dec.iso \
    --autopilot --team feisar_c1 --ticks 295 --screenshot /tmp/t295.png
```

### What it is not

1. **Not the cloud plate.** `clouds_new.gtf`'s **red channel alone** - the
   channel the plate's own microcode writes to output alpha, and the one the
   combined `r`/`g`/`b` figure above does not answer for - is `156..255`, mean
   `218.6`, **0 % of texels under 128**
   (`crates/render/examples/hd_channel_stats.rs`). There is no silhouette in it,
   so the near-opaque wisp this renderer draws is the disc's numbers read
   correctly and there is no decode bug between
   `mesh::rcs::skin::roles` and `mesh.wgsl` to find. Confidence **90**.
2. **Not something the cloud plate hides.** `OAG_SKIP_MATERIAL=clouds` at that
   tick leaves the region blown-out white rather than a road - unlike the
   "cloud plate paints a solid sheet" case above, where dropping the plate
   restored lane markings. That white is the **sky cube's own haze below the
   horizon**: `OAG_ONLY_MATERIAL=__nothing__` renders exactly it, pixel for
   pixel, with no track in the frame at all. Confidence **95**.
3. **Not an over-bright road.** `OAG_OPAQUE_ONLY=1` - every chunk whose material
   asks for a blend dropped - shows **no opaque floor at all** under the craft
   at that pose. Nothing solid is being covered. Confidence **95**.

### What it is

Nearest draw call to the craft at tick 295 (`[327.2, -42.0, -158.2]`), measured
**point-to-triangle** rather than by nearest vertex, which on a long sparsely
tessellated ribbon reads very differently
(`crates/render/examples/hd_near_probe.rs`, `OAG_NEAREST=1`):

| distance | list | texture |
| --- | --- | --- |
| **3.35** | blended | `dds/dc_iridescent_gradient.gtf` (642 tri) |
| 11.74 | opaque | `dds/track/ds_floor_cs.gtf`, `tunnel_fx_diffuse.gtf` |
| 24.05 | opaque | `dds/track/ds_wall_cs.gtf` |

The race's own telemetry puts the craft `3.28` above its ground on that tick, so
the `3.35` chunk is the surface it is riding. Its material is Talon's Junction
slot 417, `materials/etched_glass_tech.rcsmaterial`, blending
`SrcAlpha`/`OneMinusSrcAlpha` - so an alpha of zero is a **vanished floor**, not
an additive highlight that correctly contributes nothing.

**The coverage is authored, traced end to end.** The lit-race key resolves this
material to fragment block #7 at `0x5ef0`
(`scripts/ps3-microcode.py fp-file etched_glass_tech.rcsmaterial 7`):

```text
@0x19  TEX H2.xyz, -R2.wwww   unit2   ; sampler 0x94b2b285 - the base colour
@0x2b  TEX H6.x,   f[TC5].zwzz unit0  ; sampler 0x3bdc0403 = Texture1
@0x2c  ADD H4.xyz, H6.xxxx, H4        ; unit 0's red is added to the colour
@0x2f  TEX H7.xyz, R2.zwzz    unit1   ; sampler 0x9edd3243, a computed coord
@0x3d  MAD H4.xyz, H5, H4.wwww, H4    ; unit 1's tint, weighted by H6.x
@0x42  MOV H0.w,   H6.xxxx  END       ; the ALPHA is unit 0's red
```

Every step of that reaches this renderer intact:

- `Texture1` is `Material::texture`, so unit 0 **is** `dc_iridescent_gradient.gtf`.
- `f[TC5].zw` is `v[2].xy`, and the paired vertex program declares attribute
  `0x427214fc` at slot 2. The chunk's own vertex declaration names exactly four
  attributes - `position`, `normal`, **`Uv1` (`0x427214fc`)**, and the RGBE
  colour set - and `VertexDecl::diffuse_texcoord` picks `Uv1`. So `in.texcoord`
  in `mesh.wgsl` is the program's own coordinate, not an approximation.
- `roles()` reads the alpha lane as unit 0 channel 0 and packs
  `slots::alpha_channel(0)`; that is what the `MOV H0.w, H6.xxxx` says.

So the coverage this renderer computes is the material's own texture, at the
material's own coordinate, through the material's own channel. Sampled that way
across all six of the circuit's `etched_glass_tech` draws
(`crates/render/examples/hd_coverage_probe.rs`):

```text
alpha channel 0: 0..214 mean ~33, ~51 % of vertices under 8, ~80 % under 64
u -3.58..5.66   v 0.13..1.00   on a 512x16 texture
```

`dc_iridescent_gradient.gtf` is a **512x16 hue ramp** - rainbow across the left
~55 %, black across the right ~45 % - and the chunk's `u` tiles it about five
times along its length. The floor therefore paints as iridescent bands with
fully transparent gaps between them, which is what a top-down capture of just
those six chunks shows:

```sh
OAG_ONLY_MATERIAL=etched_glass_tech cargo run --release -p oag-render \
    --example model_probe -- \
    data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA00.PSARC \
    /data/environments/talons_junction/track.vex /tmp/glass_top.png
```

Two caveats on that histogram, because it is easy to over-read. It samples the
ramp **at each vertex's own UV**, so it is a per-vertex sample of a tiling ramp
and not an area-weighted coverage figure; what makes the conclusion an area
claim is the capture above, where the bands and the gaps between them are
visible directly. And the `v` span is immaterial - the ramp is constant down
its 16 rows.

**A reference capture arrived the same day and refutes the conclusion this
paragraph used to draw.** Everything measured above stands; what does not is
the inference that our frame is therefore about right. A capture of the
original at Talon's Junction's glass floor (`data/reference/hd-talons-glass/`,
supplied 2026-08-24 - `data/` is gitignored, so the file is not in the
repository and this is its only record) shows the drivable surface as a
**continuous, patterned, translucent sheet**: a fine square grid with white
boundary lines and chamfered hexagonal-mesh inset panels, laid in the plane of
the floor and following its perspective, with the city visible through it. That
pattern is `dds/glass_etched_tech.gtf` - the material's own **second** texture,
1024x512, and no other material on the circuit names it. Our frame at the same
kind of place is a thin iridescent sliver over empty sky
(`--pose 76.9,-46,148` with `OAG_ONLY_MATERIAL=etched_glass_tech`, against a
craft-to-surface distance of **1.19** units). So the picture is wrong, not
merely uncoloured, and "the coverage is authored" is **not** established.

**And the refutation lands on the microcode reading, not on the arithmetic.**
Every one of this material's **30 variants** declares `Texture1`
(`0x3bdc0403`) at unit 0, and across all 20 distinct fragment blocks the only
sampler ever read at a plain surface coordinate is that unit 0 - the 512x16
ramp. Unit 1 (`0x9edd3243`) is read at a reflection-vector coordinate
(`R2.zw`, built from `N`, `V` and `N*(V.N)` over ten instructions), unit 2
(`0x94b2b285`) at a scalar facing term, and the rest are projected screen-space
lookups. **No variant of this material can paint a grid at the surface's own
UV**, yet the original plainly does. Something in this project's chain from
"the model names two `.gtf`" to "the shader samples unit N" is wrong, and the
next section is where to look: the model record binds `glass_etched_tech.gtf`
to sampler `lightmap`, which no variant of this material declares at all.

The block survey below stands as recorded and is still the first thing to
re-check if the variant key is the wrong link: this material ships **twenty**
fragment blocks and they do not all end the same way.

| final alpha write | blocks |
| --- | --- |
| from a `TEX ... unit0` result, as #7 does | 0, 1, 2, 5, 6, 7, 10, 11, 12, 15, 16, 17 |
| `1 - TXP(f[TC0], 0x730df9ee)` - a **projected**, screen-space lookup | 8, 9, 13, 14, 18, 19 |
| the constant `1.0` | 3, 4 |

So "this floor's alpha comes from a texture" is a property of the row the
lit-race key resolves to, not of the material - `variants()`'s own doc calls
the pass half of that key "a single reading of an ordinary lit race". The six
`1 - TXP` rows are a poor candidate for the row we should have picked instead:
`0x730df9ee` is sampled through a projected screen coordinate on all six, which
is a depth-fade or refraction buffer the engine supplies rather than anything
an ordinary opaque race pass has. **And swapping rows would not fix the
picture anyway**, which is the useful part of having surveyed all twenty: not
one of them samples a second texture at a surface UV, so no choice of variant
produces the grid the reference shows. The wrong link is upstream of variant
selection.

**The colour is wrong too, and the reference says by how much.** The base
colour in the resolved block is `TEX H2.xyz, -R2.wwww unit2` - sampler
`0x94b2b285`, which no texture is bound to, because a `.rcsmodel` material
names two `.gtf` and this program samples three units. `roles()` correctly
leaves `ALBEDO_FROM_SECOND` clear (the colour merge is `Mixed`), so the albedo
falls back to unit 0 - the **lookup ramp**, not a picture. The original paints
`glass_etched_tech.gtf` there instead. Nothing here fakes that; see "The glass
family's second slot: traced, not solved" above, which reached the same
conclusion for the same family from the other direction.

**The reference gap this section originally recorded is closed**, and closing
it is what produced the refutation above: a capture of the original at this
circuit's glass floor arrived on 2026-08-24 and sits at
`data/reference/hd-talons-glass/talons-glass-floor.png`. `data/` is gitignored -
no game content may be committed, see [legal](../overview/legal.md) - so that
path is a note about a working copy, not a file a fresh checkout has. What it
shows is described above; anyone reproducing this needs their own capture of
the same place.

### The isolation harness this shipped

`mesh::rcs::isolate` is the technique the cloud-plate section used, kept rather
than rebuilt each time. Three environment variables, all off by default, all
matching a material's own path **or either `.gtf` path it names**, and all
counted in `Report::isolated` so a mutilated frame says so in the loader report:

| variable | effect |
| --- | --- |
| `OAG_SKIP_MATERIAL=clouds,glass` | drop the chunks that match |
| `OAG_ONLY_MATERIAL=track_surface` | keep only the chunks that match |
| `OAG_OPAQUE_ONLY=1` | drop every chunk the material asks to be blended |
| `OAG_TINT_MATERIALS=1` | paint every material a flat colour keyed by its slot |

**`OAG_TINT_MATERIALS` is the cheap half of an ID render**, added 2026-08-24
after the question "*what* is this pixel" came up three times and was answered
three times by guesswork. It replaces each material's picture with a 1x1 solid
colour from a golden-ratio walk round the hue wheel and draws the geometry
unlit, so the palette entry reaches the frame as itself and one sampled pixel
decodes to a material ordinal. It needs no pipeline of its own - the texture is
simply substituted - which is why it exists at all where a per-draw-call tint
would have been a pass, a shader entry point and a bind group. It found the
structure behind Talon's Junction's floating platforms in one frame:
`(79, 162, 235)` decodes to slot 441, `uv_anim_diffuse_alpha` /
`air_traffic_test_a_atoc.gtf`, the circuit's largest material at 66 chunks -
drawn, and so pale in an ordinary frame that it reads as sky.

Matching the texture and not only the material is what makes it usable on a
circuit: Talon's Junction names `track_surface.rcsmaterial` in fifteen slots
with a different `.gtf` in each, and the texture is the only thing that tells
those slots apart.

## A texture slot names its sampler, and the slot's ordinal is not its unit (2026-08-24)

**Found while asking why `etched_glass_tech`'s unit 1 and unit 2 are bound to
nothing, and it is bigger than that material.** Each `.gtf` path in a
`.rcsmodel` material record is followed eight bytes later by a **sampler name
hash**, and that hash - not the slot's position - is what a resolved shader
variant maps to a texture unit.

```text
material +0x58  u32  file offset of a .gtf path      +0x60  u32  its sampler hash
material +0x78  u32  a second .gtf path              +0x80  u32  its sampler hash
```

Read on `oag_rcs::rcsmodel::Material::texture_sampler` and
`second_texture_sampler`. Three things make it a sampler hash rather than a word
that happens to be there:

1. The values are the sampler-name preimages this page already recovered:
   `Texture1` (`0x3bdc0403`), `diffuse` (`0x515e298e`), `lightmap`
   (`0x37b5db58`).
2. **8,021 of the disc's 13,933 populated slots carry a hash the material's own
   lit-race variant declares as a sampler**, with a unit
   (`crates/render/examples/hd_sampler_bind.rs`, all 28 circuit models).
3. **Only 776 of those 8,021 - 9.7 % - land on the slot's own ordinal.** The
   first texture reaches unit 1 in 4,751 cases, unit 2 in 471, unit 3 in 36 and
   unit 4 in 24; the second texture reaches unit 2 in 1,685.

Talon's Junction's `track_surface` is the clearest single case: its first
texture (`ds_floor_cs.gtf`) carries `0x739a786e`, which its resolved variant
declares at **unit 1**, and its second (`ds_floor_n_rh.gtf`) carries `lightmap`,
declared at **unit 2**. Neither is where "first texture is unit 0, second is
unit 1" - the rule `mesh/rcs/skin.rs` and `mesh.wgsl` are both built on - would
put them.

**Confidence 80, and the counter-example is why it is not higher.** The cloud
plate's second slot carries `lightmap` too, its resolved variant declares no
`lightmap` sampler at all, and its microcode plainly samples `cloud mask.gtf` at
unit 1 - so either the record's word is authoring-time metadata a shipped
variant can override, or the variant resolution is matching the wrong row for
that material. 5,912 slots disc-wide fall in the same "declared by no sampler of
that variant" bucket, which is too many to wave through.

**One consequence lands on a claim this project already acts on.** "The
lightmap is identified" above rests on four signals, the third being that the
*material* declares a `lightmap` sampler. This word would sharpen that from the
material to the **slot**: it says which texture binds to it. The two do not
agree today - Talon's Junction carries `lightmap` in **151** second slots
against the **85** materials whose second texture is one of its
`lmaps/*-lmap.gtf`, and `etched_glass_tech`'s `glass_etched_tech.gtf` is one of
the extra 66. Signal 4's "no exceptions" is untouched (it pairs the `lmaps/`
path with the `lightmapUV` attribute, and neither is this word), but a
slot-level signal 3 would change answers, so it is named here rather than
folded in quietly.

### Corrected hours later: the hash belongs to a different path, and there is no "second slot" at all

**The pairing was off by one entry, and that single mistake is most of this
page's confusion.** A material record's texture information is not two
hardcoded slots at `+0x58` and `+0x78`. It is the material's own **input
table** - the one `oag_rcs::rcsmodel::material::parameters` has walked
since the flame's numbers were read - whose entries are `0x20` apart and shaped

```text
+0x00  u32  sampler or parameter name hash
+0x04  u32  kind: 0x8001 for a sampler, 0 for a parameter
+0x18  u32  the .gtf path (a sampler) or the value (a parameter)
```

So the hash sitting eight bytes *after* a path belongs to the **next** entry.
Pairing them that way put every texture on its neighbour's sampler.

**Three things settle the correct pairing, independently.**

1. **A normal-map decode.** Read correctly, `track_surface`'s
   `ds_floor_cs.gtf` binds `0x11cb4f74` at unit 0 and `ds_floor_n_rh.gtf`
   binds `0x739a786e` at unit 1 - and that variant's `TEX R4.xyz, R0 unit1`
   is followed twice by `MAD R4.w, R4.yyyy, {2,-1}`, which is a tangent-space
   normal unpack. The other pairing puts the *diffuse* through that decode.
2. **The reference frame.** `etched_glass_tech`'s `glass_etched_tech.gtf`
   binds `Texture1` at unit 0, sampled at `f[TC5].zw` - an ordinary vertex UV -
   so the etched grid paints in the plane of the floor, exactly as the capture
   shows, and `dc_iridescent_gradient.gtf` binds `0x94b2b285` at unit 2,
   sampled at the scalar `-R2.w`, which is what a 512x16 iridescence ramp is
   for. Talon's Junction's glass floor draws from this change.
3. **The cloud plate stops being an inversion.** `clouds_new.gtf` binds
   `0xfd669142` at unit 1 and supplies the colour; `cloud mask.gtf` binds
   `diffuse` at unit 0 and supplies the alpha from its red - `0..255`, mean
   58, 60 % of texels under 32, a real silhouette. **"The roles are the
   opposite of the slot names" above is therefore wrong**, and wrong for a
   reason worth keeping: it was a correct microcode trace laid over an
   off-by-one pairing. The mask is the mask and the diffuse is the diffuse.

### And there is no fixed number of texture slots

**Talon's Junction's 442 materials carry 1,291 sampler entries** - 1 material
with one, 181 with two, 150 with three, 74 with four, 35 with five, and
`tunnel_fx_glass` with **seven**. `Material::texture` and `second_texture` are
entries 0 and 1 of that table and nothing more; everything past them was
invisible to this project.

That retires "one slot, at least four uses" outright. The roles are not
overloaded - they are **separate entries whose hashes name them**, and
`tunnel_fx_glass` reads in order:

| hash | `.gtf` | role |
| --- | --- | --- |
| `0xeedee991` | `tunnel_fx_alpha.gtf` | coverage |
| `0x11cb4f74` | `tunnel_fx_diffuse.gtf` | diffuse |
| `0xfa79b1cd` | `tunnel_fx_emissive.gtf` | emissive |
| `0x173fbce2` | `lightstrip_emissive.gtf` | a second emissive |
| `0x20c3e476` | `tunnel_fx_spec.gtf` | specular |
| `0x994bbcf1` | `tunnel_fx_facingramp.gtf` | the facing ramp |
| `0x37b5db58` | (none supplied) | `lightmap` |

`0x20c3e476` lands on a `*_spec.gtf` on all 63 of its uses and `0x994bbcf1`
/ `0x35281c78` on a `*facing*ramp*.gtf` - so the **`FacingRamp` input
`HANDOVER.md` lists as absent is named, supplied and sitting in the file**,
and the lightmap is *declared* by 118 of the circuit's materials rather than
inferred from an `lmaps/` path on 85 of them.

**`oag_rcs::rcsmodel::material::samplers` reads the whole list now**;
`oag-render` still binds the first two, so the third onwards is read and not
drawn. Which of them a shader wants is no longer a guess - it is the hash,
against the resolved variant's own declaration.

### The renderer binds by role now, not by entry position - and the census says what that is worth

`mesh::rcs::skin::picks` chooses **which** of a material's sampler entries each
of this renderer's two bindings comes from: the entry whose declared sampler
the program's colour lane reaches, and the entry its alpha lane reaches.
Entries 0 and 1 remain the answer where nothing resolves, and a lightmapped
material keeps entry 1 regardless - the lightmap is the one role of that
binding this project has identified, and an alpha trace must not displace it.

**Measured before it was built, and the number is small on purpose to state**
(`crates/render/examples/hd_role_census.rs`, Talon's Junction's 302 drawn
materials): the colour lane never resolves past entry 0, and the alpha lane
resolves past entry 1 on **nine** materials. Everything else lands on an entry
this renderer already bound, or does not resolve at all. So this is plumbing:
it removes "first and second slot" as a concept from the renderer and makes
every later role reachable, and it is **not** what would change a frame.

**What would change a frame is the roles this shader does not implement**, and
they are all named and supplied in the same table: the specular map
(`0x20c3e476`, a `*_spec.gtf` on all 63 of its uses), the emissives, and the
facing ramp (`0x994bbcf1` / `0x35281c78`, always a `*facing*ramp*.gtf`). Each
needs its operation read out of the microcode before anything binds it - the
rule this project keeps: a role with no recovered shading stays unwired rather
than guessed.

**The clearest of those is Talon's Junction's glass floor, and its coordinate
is already traced.** `etched_glass_tech`'s `dc_iridescent_gradient.gtf` binds
`0x94b2b285` at unit 2, and block #7 samples it at a **scalar**:

```text
@0x01  op3B R2.xyz, f[TC3], R0     ; normalize(normal)
@0x11  op3B R1.xyz, R1, R0.wwww    ; normalize(view)
@0x12  DP3  R2.w, -R1, R2          ; dot(-V, N)
@0x19  TEX  H2.xyz, -R2.wwww unit2 ; sampled at dot(V, N)
```

So the iridescence is a **view-angle sheen**, not a light projected onto the
floor and not a pattern painted into it - which is what a 512x16 hue ramp is
for. This renderer still binds that ramp as the albedo and samples it at the
surface's own UV, so it reads as fixed bands welded to the road instead of a
sheen that slides with the camera. That is the next thing to fix here, and
unlike the rest of the family its combine is already traced.

### The lightmap is bound wherever it sits, not only when it is entry 1 (2026-08-24)

**A third of Talon's Junction's baked lighting was in a slot nothing looked
at.** `Material::lightmap` reads a path pattern - `lmaps/` and `-lmap.gtf` -
on the **second** entry alone, which was the only entry this project could see
when it was written. The file says it outright, and says it anywhere: the
`lightmap` sampler (`0x37b5db58`, a recovered preimage) is named by **275** of
the circuit's 442 materials, and this renderer bound the atlas for **85**.

`Material::lightmap_entry` finds it by hash, `skin::picks` gives it the second
binding wherever it sits, and `roles` keys `SECOND_IS_LIGHTMAP` off the same
answer. Talon's Junction goes 85 -> 275 lightmapped materials, amphiseum to
359, `12_sol_2` to 309.

**What it does to a frame, measured rather than asserted**, on the glass-floor
pose: 1.1 % of pixels move; the fraction clipped to white falls **22.65 % ->
21.57 %** and the fraction crushed to near-black rises **16.52 % -> 17.12 %**.
Both directions are the reading working - the prelit term adds light where the
atlas is bright, and `mask = baked.a * in.sun_mask` now takes the sun away
where the atlas says shadow, on 190 materials that previously kept full sun
because nothing was bound. Against a reference capture of the same circuit
(17.4 % clipped, 7.1 % dark) the clipped end moves toward the original and the
dark end away from it, which is the tone gap this page records elsewhere and
not this change's to close.

**It does not explain the black walls.** `track_wall` - 22 chunks, and the
single largest dark area of that frame at 12,363 sampled pixels of mean
luminance **12/255** - names a `lightmap` entry with **no path**, so nothing
changes for it. Its own texture is an ordinary mid-grey (`ds_wall_cs.gtf`,
channel means 89/99/112, 27 % of texels under 32), so the darkness is in the
lighting and not the art. That is open, and it is the largest single thing
between this renderer and the reference frame: the walls flanking the track
read as flat black silhouettes where the original has grey concrete, which is
what a report of "big structures missing" looks like from the player's seat.

### Three sampler roles identified by what they bind, and the glass floor stops painting a ramp

**A hash with no preimage can still be identified, by the files it binds
across the disc.** Sweeping every HD circuit's materials and reading each
bound `.gtf`'s own dimensions
(`crates/render/examples/hd_sampler_bind.rs`, `OAG_RAMPS=1`):

| Hash | Uses | Binds | Reading | Confidence |
| --- | ---: | --- | --- | ---: |
| `0x739a786e` | 20 | `ds_floor_n_rh`, `ds_pit_box_n`, `ds_wall_n` - nothing else | **normal map** | 88 |
| `0x20c3e476` | 64 | `blue_metal_spec`, `tunnel_fx_spec`, `tunnel_fx_lights_spec`, `tunnel_02_posts` | **specular map** | 85 |
| `0x35281c78` | 37 | `blue_metal_facing_ramp.gtf`, and nothing else | **facing ramp** | 88 |
| `0x994bbcf1` | 27 | `blue_metal_facing_ramp.gtf`, `tunnel_fx_facingramp.gtf` | **facing ramp** | 88 |
| `0x94b2b285` | 1 | `dc_iridescent_gradient.gtf` | **facing ramp** | 85 |
| `0x11cb4f74` | 94 | 16 ordinary art files | diffuse | 80 |

**Every** use of the three facing-ramp hashes binds a texture 32 texels or
less in one dimension, which is what a lookup addressed by a scalar looks
like and what a picture addressed by a UV does not. `0x739a786e` never binds
anything but a `*_n*.gtf`, which is the same fact the pairing correction above
rests on, reached from the other side. So **the `FacingRamp` input
`HANDOVER.md` lists as absent is named, supplied and countable**, and so are
the normal and specular maps.

**Acted on, narrowly.** `mesh::rcs::skin::NOT_A_PICTURE` lists those hashes
plus `lightmap`, and [`picks`] uses it for one thing only: where a material's
colour lane does not resolve, the albedo fallback takes the first entry that
is *not* one of these rather than entry 0 blindly. On `etched_glass_tech`
entry 0 **is** the iridescence ramp, so Talon's Junction's glass floor was
painting a 512x16 hue ramp at the road's own UV - the rainbow bands welded to
the surface. It now binds `glass_etched_tech.gtf`, the etched grid, and the
floor matches the reference capture: grid, white boundary lines, chamfered
hex-mesh panels, structure visible through it.

**Nothing samples these in their own role yet, and that is the rule rather
than an omission.** The facing ramp wants `dot(V, N)`, the normal map wants a
tangent frame, the specular map wants the exponent chain - and only the first
of those has its coordinate traced. `etched_glass_tech`'s block #7 computes it
outright:

```text
@0x01  op3B R2.xyz, f[TC3], R0     ; normalize(normal)
@0x11  op3B R1.xyz, R1, R0.wwww    ; normalize(eye - position)
@0x12  DP3  R2.w, -R1, R2          ; dot(-V, N)
@0x19  TEX  H2.xyz, -R2.wwww unit2 ; sampled at dot(V, N)
```

so the iridescence is a **view-angle sheen**, not light projected onto the
floor and not a pattern authored into it. Its combine is traced too -
`vertexLight * (ramp + c) + ramp`, plus the grid's red as an additive term and
a `paraboloidReflectionTex` tint weighted by that same red, with the grid's
**RGB never sampled at all** and its red the output alpha. That is the next
thing to implement here, and it is the only member of the glass family whose
arithmetic is read end to end.

### Five more sampler names, by preimage over the executable

Sweeping `EBOOT.elf`'s 9,097 identifier-shaped strings the way `zoneTexInner`
was found:

| Hash | Name |
| --- | --- |
| `0x9edd3243` | `paraboloidReflectionTex` |
| `0x730df9ee` | `shadowMapTex` |
| `0x9becc725` | `directionalLight0ShadowTex` |
| `0xa51b8b14` | `directionalLight0LightmapTex` |
| `0x52834137` / `0xcc090349` | `zoneTexOuterNearest` / `zoneTexOuter` |

`paraboloidReflectionTex` confirms the glass family's unit 1 from the other
side: the coordinate this page traced as "a scalar built from a clamped dot
product, offset by a constant" - `(R1.w + 0.25, -R1.z * 0.5 + 0.5)` - is a
**dual-paraboloid** reflection lookup, which is exactly that remap. It is an
engine-supplied probe, not one of the material's own `.gtf`, which is why no
`.rcsmodel` names it.

### Acted on 2026-08-24, additively, and it fixed the light cones

**`mesh::rcs::skin::units` reads the hash and `roles` uses it - but only ever
to *add* an identification, never to replace one.** Two measurements forced
that shape, and both are worth keeping because a strict rule looks obviously
right until you render it:

1. **Replacing the ordinal takes working surfaces away.** It moved 27 of
   Talon's Junction's 442 slots off `ALPHA_FROM_SECOND`, and the circuit's
   perforated trackside barrier - the one beside the blimp at tick 295 - lost
   its holes and washed out to a flat sheet. The cloud plate is the same
   hazard on paper: its second slot's hash is `lightmap`, its variant declares
   no `lightmap`, and its microcode samples the mask at unit 1 regardless.
2. **The second slot must never be the lightmap.** With the hash accepted but
   no guard, nine lightmapped slots gained `ALPHA_FROM_SECOND` and the same
   barrier washed out again - because a lightmap's alpha is the sun-occlusion
   mask (`mesh.wgsl` reads it as exactly that, `mask = baked.a * in.sun_mask`)
   and its RGB is a light term. Pointing coverage at it paints a shadow map as
   a stencil. `roles` now refuses both roles wherever `SECOND_IS_LIGHTMAP` is
   set.

With both rules in place the change is **strictly additive**: on Talon's
Junction four slots gain a second-slot role and none loses one, and the
tick-295 race frame is byte-identical. The gains are real where those surfaces
are on screen. `dc_lightcone` - one of the six materials the ratchet above
names - is the clearest: it painted `dc_gradient_noise.gtf`, and its cones
rendered as ragged noisy wisps; the hash puts its second texture
`dc_gradient_e.gtf` on the unit the program samples for colour, and they render
as the smooth teardrop falloffs a light cone is. Both circuits that author it
change the same way.

```sh
OAG_ONLY_MATERIAL=dc_lightcone cargo run --release -p oag-render \
    --example model_probe -- \
    data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA00.PSARC \
    /data/environments/amphiseum/track.vex /tmp/cones.png
```

Swept for regressions on six circuit models through `model_probe`: five move
by single-digit lit pixels and one (`01_vineta_k`) not at all; a full race
frame on `amphiseum` at tick 400 is byte-identical. **What it did not fix is
the glass floor**, and that was predicted rather than discovered: that
material's colour merge comes out `Texel::Mixed` and its alpha traces to
unit 0 either way, so no remapping of units can move it.

**What is still not rebound.** The hash is not yet used to *bind* a texture to
a unit the model record's slots do not already reach - `etched_glass_tech`'s
`glass_etched_tech.gtf` names sampler `lightmap`, which none of its 30 variants
declares, so the picture the reference shows still has no route into the
shader. That is the open half, and it is the one the glass floor waits on.

**The glass floor's contradiction is still where it has to be resolved.** The
section above shows the original painting `glass_etched_tech.gtf` in the plane
of the floor while no variant of that material samples any second texture at a
surface UV. The model record binds that `.gtf` to sampler `lightmap`, which
that material's variants never declare - so either this word is not the
binding it appears to be, or the variant rows being resolved are not the ones
the original runs. **That contradiction now has a picture to be tested
against**, which it did not before, and it is the reason to do this work next
rather than eventually: an implementation of the binding either makes the
reference's grid appear or it does not, and either answer settles the reading
in one frame.

## The vertex program says which way up a texture coordinate is (2026-08-24)

**Confidence 92.** [`oag_rcs::rcsmaterial::fragment`] traces what a program
does with the texels it samples. Its sibling `vertex` traces the varying it
samples *at*, and the one thing this project needed from it is that **HD's
circuit shaders do not agree on the orientation of `v`.**

`talons_junction/track_wall`'s lit race-pass variant opens

```text
 4  ADD o[TC6].y, -v[3].yyyy, c[206].yyyy
 5  MOV o[TC6].x, v[3].xxxx
```

- `v[3]` is the attribute the block itself declares as `Uv1`
  (`0x427214fc`), read from the vertex `SHO` block's attribute table.
- `c[206].y` is 1.0: instruction 9 of the same block is
  `MAD R0, v[2], c[206].xxxx, -c[206].yyyy` on the packed `tangent`, the
  standard `t * 2 - 1` unpack, which fixes `c[206]` as `(2.0, 1.0, ...)`.

So the coordinate reaching `f[TC6]` is `(Uv1.x, 1 - Uv1.y)`.

**Per material, and the census is what makes that a reading rather than a
guess.** Across Talon's Junction, `track_wall` flips in 18 of its 94 vertex
blocks and `track_surface` in **0 of 64**; of the 283 variants the circuit's
drawn slots resolve to, **exactly one** flips. `track_surface`'s coordinates
were always right, and a global flip would have broken them to fix the walls.

`Program::flips(hash)` answers it, and it is deliberately narrow: an
instruction counts only when it negates the named attribute's input register
**into an output texture coordinate**, using a source slot the opcode actually
reads. A negate into a temporary, into `o[POS]`, or parked in a slot that
`MUL` never looks at is not a flip - each is a test in
`crates/rcs/src/rcsmaterial/vertex/tests.rs`.

The renderer applies it on the CPU, once per vertex at build time, through
`mesh::slots::FLIP_V`. That bit rides in the same word as the texture-unit
roles because it is the same kind of statement - what this material's own
microcode says - but unlike the other bits it never reaches `mesh.wgsl`: the
orientation is a property of the material, not of the pixel.

What it moves is in [`rcsmodel.md`](rcsmodel.md#a-texture-coordinates-orientation-is-per-material-in-the-vertex-microcode),
with the six explanations that were measured and died before the microcode was
read.

## The lighting family is three-way, and the directional light is what makes it hold (2026-08-24)

**Confidence 88.** [`renderer.md`](../ghidra/functions/ps3-hdfury-eu/renderer.md)
records that splitting materials into lighting families on what their `SHO`
header *declares* was tried and refuted: keying on the lightmap sampler and
`constantAmbientColour` put **447 of Talon's Junction's 978 drawn chunks** in a
"neither" bucket alongside `track_wall` and `glasstest`, which are ordinary lit
surfaces.

**Asking about the directional light as well is what fixes that**, and it is
one extra question rather than a different idea. `track_wall` and `glasstest`
declare `directionalLight0DirectionWorldSpace` and `directionalLight0Colour`,
so under a three-way key they land in "sun only" - which is what they are - and
the "neither" bucket empties of everything except the surfaces that really are
fed no scene light.

| Fed | Talon's Junction | Anulpha Pass |
| --- | --- | --- |
| ambient + sun | 24 materials, 144 chunks | 58 materials, 179 chunks |
| sun only | 224 materials, 618 chunks | 218 materials, 827 chunks |
| **neither** | 35 materials, 151 chunks | 33 materials, 95 chunks |

**The confirmation is in the names, and it was not designed for.** The split is
made on declared parameters alone, and every material that falls into "neither"
on either circuit is a sign or a glow: `sign_emissive`, `sign_emissive_glow`,
`cf_glow_tube`, `cf_plasma_glow2`/`3`, `cf_startbeam_glow`,
`mr_uvanim_em_alpha`, `cf_uvanim_emssive_glowtint`, `nr_holobowlparallax`,
`nr_twinblend`, `cf_billboard1`, `scanlinebillboard`, `dc_lightcone`,
`loopmaterial`, `nr_scalinguvs`. Two independent circuits, no exceptions.

`oag_rcs::rcsmaterial::Declared::takes_directional_light` reads it, and
`mesh::slots::NO_AMBIENT`/`NO_SUN` carry the pair to the renderer.

### What is acted on, and what is not

**Only the emissive branch.** `mesh.wgsl` gives a surface fed no scene light an
`authored` of `1.0` and no specular, so its albedo is the picture exactly as
the microcode leaves it. Measured: 3.2 % of an Anulpha frame changes and 6.9 %
of a Talon's Junction one, 87 % and 62 % of it brighter.

**Dropping the ambient from the "sun only" family is not shipped**, though the
bit that would gate it is read. It was tried on 2026-08-24 and is a regression:
those materials declare `prelitBias` and `prelitScaleSpecular` and are lit by
the lightmap, and this renderer applies no bias term at all
(`prelit = scale * lightmap^power`, against the microcode's
`... - bias`). Removing their ambient before supplying what replaces it turned
Anulpha Pass from brown to black. The number that decides it is the overlap
between those 827 chunks and the ones that actually carry a lightmap, and that
has not been measured yet.

## A surface scrolls off an engine `time`, not off a keyframe block (2026-08-31)

**HD authors no texture-transform keyframe block at all**, and that is a
property of where the block lives rather than a gap in this reading. Pulse
stores one `0x40`-byte block per material *inside the mesh payload*
([vex.md](vex.md), "The texture-transform keyframe block"), and HD emptied that
payload out - a PS3 `Mesh` node is a bounding-box pair and a chunk hash
([rcsmodel.md](rcsmodel.md)). The animation moved into the shader.

**The mechanism, read at instruction level.** `uvanim_diffuse_emissive`'s lit
fragment block on Amphiseum, disassembled with `scripts/ps3-microcode.py`:

```text
@0x00  MOV R2.xy, f[TC3]
@0x01  MOV R0.w, {const}          [<- parameter 0x906b67ba]
@0x03  ADD R0.z, R2.yyyy, {const} [<- parameter 0x78256a45]
@0x05  MAD R0.w, R0.zzzz, {const}, R0  [<- parameter 0x78787596]
@0x07  TEX H0, f[TC3] unit0
@0x08  MOV R0.z, R2.xxxx
@0x0c  TEX H1.xyz, R0.zwzz unit1
@0x0d  MUL H1.xyz, H1, {const}    [<- parameter 0xe8bcd7f5]
@0x0f  MAD H0.xyz, H0.wwww, H1, H0
```

So unit 1 is sampled at `(u, (v + a) * b + time)` where `a` and `b` are the
material's own authored floats and `time` is engine-supplied, and its result
is **added** to the diffuse, gated by the diffuse alpha.

**That excerpt is block #2, the *unlit* variant, and it stops at the `MAD` -
which is what made the layer's position in the shade ambiguous for a year.**
Read block #3, the lit and fogged one, and the position is not ambiguous at
all:

```text
@0x1c  MAD H6.xyz, H1.wwww, {const}, H6   [<- 0x2dba643d]  ; ambient + ndl*sun
@0x1e  ADD H4.xyz, R2, H6                                  ; + prelit
@0x22  MUL H4.xyz, H0, H4                                  ; albedo * light
@0x23  MAD H0.xyz, H0.wwww, H1, H4                         ; + alpha * tinted glow
```

**The accumulate reads a value the light has already multiplied, so a glow is
not itself lit.** Block #4 (`MUL` at `0x23`, `MAD` at `0x25`) is the same pair
in the same order, and block #2's own `MUL H0.xyz, H0, H2` at `0x0b` puts its
one light term on the same side. Three variants, no exceptions; confidence 95,
the ordering being read rather than inferred. `mesh.wgsl` adds the layer after
its own light multiply on both of its paths for this reason - see
[renderer.md](../ghidra/functions/ps3-hdfury-eu/renderer.md), "The emissive
glow belongs on the lit path". On Talon's Junction's
`mt_uvanim_diffuse_emissive2` those floats are `a = 0` and `b = -1`, so the
emissive layer scrolls one texture unit per second in `-v`.

**`0x906b67ba` is `time`, by preimage**, which is what turns "a constant the
engine patches" into a clock. The same technique names 63 more; see below.

**How far it reaches**, from `crates/render/examples/hd_uv_time_census.rs`:
**310 of the disc's 1,186 `.rcsmaterial` declare `time`** in a fragment block,
and a circuit's own material table runs from 4 of Sol 2's 442 slots to 101 of
Modesto Heights' 859. `and_arrowmaterial`, `animating_traffic`,
`scanlinebillboard`, `hologram`, `nr_crowd_bustle` and `cf_startbeam_glow` are
among them - the same content classes Pulse animates through its keyframe
blocks.

**Wired 2026-08-31**, as `mesh::slots::ADD_SECOND` plus a per-material table.
This section previously ended "none of it is wired, and wiring the scroll
alone would change no pixel", which was right about the ordering: `mesh.wgsl`
*selects* between its two textures where this family **adds** one to the
other, so the layer had to come first and the clock rides in with it.

What decided it was a shape census rather than a name
(`scripts/hd_time_shapes.py`): **290 of 291** materials take `time` into a
texture coordinate and **2,230 of 2,305 blocks** combine the sample by
accumulate. The 35 materials that are not *purely* accumulate all mix
accumulating blocks with multiply-only ones - the pass dimension, which
`skin::variants` already resolves - and exactly one material on the disc,
`hd_waketrail`, is multiply-only throughout. That is one coherent change, not
the classifier problem that killed `output_lit_by`.

**What it reaches**, from `crates/render/examples/hd_emissive_reach.rs`, which
joins the three conditions the layer needs all of - the lit variant's program
accumulates, the second `.gtf` decoded, and it is not the baked atlas:

| | Count |
| --- | ---: |
| Slots declaring `time` across 16 circuits | 236 |
| Accumulating | 189 |
| With a decoded second texture | 222 |
| Not lightmapped | 169 |
| **Drawing the layer** | **119** |

On the built model that is **62,904 of Modesto Heights' 1,151,776 vertices**,
24,296 of Talon's Junction's and 17,448 of Anulpha Pass's -
`crates/render/tests/hd_emissive_glow_ground_truth.rs`, which also measures the
glow's own pixel contribution by rendering the same model with the bit cleared.

**One approximation is stated rather than hidden**: the program addresses unit
1 from `f[TC3]`, and whether the UV set feeding that interpolator is the one
this renderer carries as `texcoord` is unestablished - the same open question
`skin::roles` already has for the second texture's own sampling. Only `v`
moves, so a mismatch shows as a glow tiled wrongly rather than as a missing
one.

### The parameter names, by preimage

The sweep that named the samplers, pointed at the *parameter* table:
`crates/render/examples/hd_param_names.rs` hashes every identifier-shaped
string in `EBOOT.elf` and matches against the hashes the disc's `.rcsmodel`
material records carry. **64 of 300 land.** The `.rcsmaterial` files are no
help here and that is itself a finding: their own tables store the hashes and
never the strings, so 0 of 300 preimage against the whole shader corpus.

| Hash | Name | Uses | Where |
| --- | --- | ---: | --- |
| `0xea1dcc4c` | `uvScale` | 1,364 | `lambert`, `simpletextureuvoffsetscale`, everywhere |
| `0x1eb13436` | `uvOffset` | 1,364 | the same records, always paired with the above |
| `0xce5c4410` | `W_Cycle` | 338 | `weapon_pads` |
| `0x02ab9f07` | `Colour` | 156 | `mageffect08`, `speedup_material` |
| `0xab31c2b1` | `Refbrightness` | 97 | the glass family |
| `0xf0d90109` | `speed` | 79 | `uvanim_diffuse_emissive` and its five siblings |
| `0x31182e0d` | `Speed` | 66 | `animhexlights`, `hologram` |
| `0x336d2dcc` | `V_Offset` | 47 | `scrollingalpha`, `hd_muzzleflash` |
| `0x1028bf46` | `Transparency` | 16 | `glass_colour_spec_trans` |
| `0x8f2fe704` | `UV_offset` | 5 | `hd_plasmaring_glow` and the explosion glows |

`uvScale`/`uvOffset` being the two commonest parameters on the disc, and always
appearing together and in equal number, looked like the static half of the same
story. **The values say otherwise, and this section used to claim otherwise.**
The operation is confirmed at instruction level -
`simpletextureandtexturealphauvoffsetscale`'s vertex block does
`MUL R1.xy, v[2].xy, c[207].xy` then `ADD R1.xy, R1.xy, c[208].xy`, with
`c[207]`/`c[208]` the two parameters (a vertex program's `c[N]` is register
`N + 256`), so it is `uv * uvScale + uvOffset` exactly as the names say. What is
*authored* is almost always nothing:

| | Records |
| --- | ---: |
| Carry the pair (4 archives swept) | 2,582 |
| **Of those, the identity** - scale `(1, 1)`, offset `(0, 0)` | **2,510** |
| Offset by a whole tile in `v`, a no-op under a repeat sampler | 21 |
| A genuine sub-tile offset | **~51**, every one on an `hd_adverts` billboard |

**`uvScale` is `(1, 1)` on every record on the disc** - all 42 distinct value
pairs have it - so the scale half is authored and never used. This page
previously said "the surfaces carrying one would tile wrongly first", and
`crates/render/examples/hd_uv_transform_census.rs` is what withdrew it: reading
these would move about fifty billboard surfaces, not a circuit.

## HD's vertex programs scroll the texture coordinate (2026-10-06, `hd-anim-textures`)

**The second place HD animates a texture, besides the fragment `time` of the
section above.** A material's lit vertex block copies the coordinate attribute
to the interpolator its fragment program samples with a multiply-add between
(`scripts/ps3-microcode.py vp-file`, `basic_uv_scroll` block 2 onwards; every
block that declares `time` was swept):

```text
MOV R0.w, c[207]                      ; one factor
MAD o[TC3].x, R0.w, c[208], v[2].x    ; other factor, plus the coordinate
MAD o[TC3].y, R0.w, c[206], v[2].y
```

One factor is `time` (`0x906b67ba`, register 463 in `basic_uv_scroll`), the
other a parameter the **material record authors** - `USpeed` `0x1abbe1f7` and
`VSpeed` `0x9c2f9359` there, so the law is `uv + time * rate` per axis and the
rate is the disc's own, not chosen. The fragment block samples its one material
texture at `f[TC3]` (`TC4`/`TC5` in the other variants, the same law) and the
lightmap at a different, packed interpolator, so the surface scrolls and its
bake does not. `basic_uv_scroll` on Tech de Ra authors `USpeed` 0.45, `VSpeed`
0: the orange emissive strips slide at 0.45 tiles a second.

**Census** (`crates/render/examples/hd_anim_family_census.rs`, 16 circuits, a
material counted once per circuit it appears on). The earlier `hd_uv_time_census`
looked at the fragment block only and so read `Speed`/`VSpeed`/`speed` materials
as time-less: **50 shaders declare `time` in a vertex block** (22 only there, 28
in both stages), and 32 of the 50 feed it through a multiply-add into an output
coordinate.

| Law | Shaders (materials on circuits) | Rate hash(es) |
| --- | --- | --- |
| `uv + time * (u, v)`, **wired** | `basic_uv_scroll` (3), `cf_uvanim_emssive` (17), `hologram` (11), `emissive_bloom` (11), `emissive_lights` (7), `uv_anim_diffuse_alpha` (3) | `USpeed`/`VSpeed`; `0x87d769dc`/`0x2481ef75`; `Speed`; `0x68292521` (v); `0x33d51367` |

**Admitted only on what was read**: the rate hash set above, a lit vertex block
that declares `time` and the hash, and a record that names exactly one texture
besides the lightmap. **76 materials across 12 circuits** (Tech de Ra 5,
Amphiseum 15, Modesto Heights 7, Talon's Junction 2, Vineta K 7, `02_track` 1,
`03_track` 7, Chenghou 2, Ubermall 11, Sebenco 1, Sol 2 1, Anulpha Pass 17; the
four Zone circuits 0), pinned per circuit by
`crates/render/tests/hd_vertex_scroll_ground_truth.rs`. They reach the renderer
as `mesh::AnimTrack::Scroll`, the track 2048's plain scroll already is, through
`mesh::rcs::vertex_scroll`. A rate of zero authors no track.

**Read, and still drawn still, each named**:

- `animlights`, `animhexlights`, `dc_hologramwithstatic2`: the scroll is on the
  second UV set (`v[8]`) or the material has a second texture the scroll may or
  may not reach. Not measured.
- `cf_uvanim_emssive_glowtint*`, `cf_uvanim_emssivealpha`, `jd_uvanim_*`,
  `mr_uvanim_em_*`, `mr_waterfall`, `cf_waterfall`: two material textures.
- `nr_twinblend`, `reflectplane_dc_seawater`, `cf_chenghou_sign`,
  `sign_emissive_glow2uv`: the rate is a **literal in the shader**, not a
  record parameter. Readable, not read here.
- `loopmaterial`, `cf_startbeam_glow`, `cf_constantcolourglow_ramp_*`,
  `jd_landinglights`, `cf_laserrail_cap`, `dc_flashingglow`: `time` is declared
  by the vertex block but never added to a coordinate by a multiply-add this
  sweep matched (colour or intensity drive).
- Fragment-only `time`: `nr_crowd_bustle` (33), `nr_billboardholographicscanlines`
  (35), `scanlinebillboard*`, `uvdistortion_*`, `water`, `pipefx_v2`, the
  holograms: unread laws.
- `weapon_pads` `W_Cycle` (231) is the pads' own thing; `scrollingalpha`
  `V_Offset` is on no circuit material (weapons and muzzle flash only).

**Pixels are small and that is the truth of the content, not of the wiring.**
At the whole-circuit camera of the ground-truth test the scroll paints 25
(Tech de Ra) and 13 (Anulpha Pass) pixels against the same model with the
tracks cleared; `orange_emissive` is a smooth strip texture, so a slide along
its length barely changes it. Amphiseum's 15 materials are hologram boards that
sit off the whole-circuit view (0 pixels there). A matched race camera on one
of them was tried and not framed (`--camera-pose` from the placed vertices put
the eye inside a wall); no frame of a scrolled hologram is in the evidence.

**Omega: checked, applies in name, not wired.** Omega carries `basic_uv_scroll`
and `scrollingalpha` with HD's `VSpeed`/`V_Offset` names
([omega-status.md](omega-status.md)), but its programs are GCN and unread, so
whether Omega's vertex stage computes `uv + time * rate` is not established.
**2048 and Omega: ported by name, 2026-10-06** - both author the same rate hashes
on the same shader names and play them through the same track, labelled inherited
(2048's vertex programs declare `time`; Omega's are unread). See
[2048-material-params.md](2048-material-params.md#inherited-from-hd-the-vertex-scroll-2026-10-06-psp2-scroll).

## The ship hull's dark materials: a wrong colour-set match, not a wrong vertex class (2026-09-13)

**The diagnosed lead - `skin::variants()` always asking for `Class::Static`
- is refuted for this symptom, measured rather than assumed.**
`crates/render/examples/hd_ship_class_census.rs` walks every `/data/ships/
*/ship.vex` on the disc (37 files, 178 drawn materials) and asks, per
material, which `Class` the file's own table carries and which one the
ordinary lit-race key resolves through. Trying every `Class` instead of only
`Static` moves **zero** of the 78 materials `Static` misses: every ship
material's table carries an identical `Static` and `RigidBody` row at every
feature hash it ships, confirmed at the program level too -
`crates/render/examples/hd_class_fragment_share_check.rs` checked every
`.rcsmaterial` on the disc for a same-feature-hash pair across classes and
found the **fragment** block byte-identical in all 36,257 cross-class pairs
disc-wide (only the *vertex* program differs, and this project's own
position/normal decode does not run it - see `oag_rcs::rcsmodel::Mesh::
positions`). Confidence 92 (an exact arithmetic invariant across many real
files, per the confidence rubric's ceiling for that evidence class).

**The class hardcode is still a real, separate bug, just not this one's.** A
material whose table ships *only* `RigidBody` and no `Static` row at all
never resolves regardless of the fix above -
`data/fe/frontendscene/frontendscene_hd_atg.vex`'s
`basic_vertexemissive.rcsmaterial` (56 chunks) is exactly that case, a
front-end consumer of the same gap tracked separately under `HANDOVER.md`'s
open threads. `crates/mesh/src/mesh/rcs/skin.rs`'s `variants()` now tries
every `Class`
in `Class::ALL`'s order (`Static` first, since it is what most materials
ship), which resolves that one material and, per the paragraph above, changes
no ship's shading.

**The actual cause: `VertexDecl::vertex_colour()` shape-matched a ship's
`VertexColour1` attribute as HD's baked-light colour set.** Every one of the
78 misses wants the `IleVertex` feature (`Features::chunk_word` sees "this
chunk has a colour set" and asks for it), and no ship material on the disc
ships that permutation under any class - `hd_ship_class_census.rs` reports
this exactly, with the shipped feature hashes decoded to prove none of them
carry `IleVertex`. `vertex_colour()` matched by shape (four normalised bytes,
`RSX_UBYTE_NORM`, not named `tangent`) rather than by hash, and that shape
also fits `VertexColour1` (`0x7493d450`) - Maya's own name for a **distinct**
attribute from the `colorSet1`/unnamed-`0x1aaf7631` pair the shape rule was
written for. `crates/render/examples/hd_ship_vcol_dump.rs` found this by
dumping a ship chunk's full declaration; `hd_colour_attr_census.rs` then
swept all 643 `.rcsmodel` files disc-wide and found the population the
original rule's doc comment never saw, because it was measured over Talon's
Junction alone, which has no ship file to carry `VertexColour1`:

| Hash | Name | Chunk attributes disc-wide |
| --- | --- | ---: |
| `0x1aaf7631` | (unnamed) | 13,485 |
| `0x7493d450` | `VertexColour1` | **2,595** |
| `0xdbe5f417` | `tangent` | 5,535 |
| `0xce5cd9d9` | `colorSet1` | 484 |
| `0xed9a85ea` | (unnamed) | 18 |

`VertexColour1` also appears on 10 **track** files sharing the same Maya
authoring pipeline (`01_vineta_k`, `02_track`, `03_track`,
`04_chenghou_project`, `05_ubermall`, `10_sebenco_climb`, `amphiseum`,
`modesto_heights`, `tech_de_ra`, all four Zone circuits) - not on Talon's
Junction, Anulpha Pass or Sol 2, so this page's other disc-wide chunk counts
against those three circuits are unaffected. What `VertexColour1` actually
holds is unread; only that it is not the light term is established.

**Fixed by splitting the question in two, not by narrowing the one method -
the first draft narrowed it and broke a second, real consumer.**
`VertexDecl::vertex_colour()` is still the shape match (four normalised
bytes, not `tangent`) it always was, because
`oag_livery::flare::alpha_ramp` reads `VertexColour1` through exactly
this method (via `Mesh::vertex_light`) for a real and unrelated purpose: the
engine flame's own alpha ramp, `alpha = alpha_scale * (1 - f) *
VertexColour1.w`, per
[engine-flare.md](../ghidra/functions/ps3-hdfury-eu/engine-flare.md). A first
version of this fix narrowed `vertex_colour()` itself to the two colour-set
hashes and shipped green against every test run by hand - until `just
test-data`'s full ground-truth suite caught `hd_engine_flare_ground_truth
::the_flame_carries_an_opacity_ramp_in_its_vertex_alpha` failing with "the
flame's ramp spans 1 to 1, so it is not a ramp": the same physical attribute
is legitimately read for two different reasons, and narrowing the shared
method for one broke the other silently everywhere `just` alone would not
have shown it.

The actual fix is a new method, `VertexDecl::light_colour_set()`, matching
only the two hashes established to be the same colour-set attribute
(`0x1aaf7631`, `colorSet1`), which `Features::chunk_word`/`for_chunk` now
call instead of `vertex_colour()`. `vertex_colour()` itself is untouched, so
the flame ramp keeps reading `VertexColour1` exactly as it did.
`light_colour_set()` excludes it and the second unnamed `0xed9a85ea`
(population too small to identify, excluded on the same "positive reading
only" principle). Confidence 90: exact on the population that explains the
78 misses (`hd_ship_class_census.rs` reports 0 misses left disc-wide after
the fix, restated below), short of the top band because what `VertexColour1`
actually encodes for a *hull* material - as opposed to the flame, where it is
read - is still unread.

**Measured before/after**: disc-wide, 178 drawn ship materials, 535 drawn
chunks.

| | Materials resolved | Chunks covered |
| --- | ---: | ---: |
| Before | 100 of 178 | 234 of 535 |
| After | 178 of 178 | 535 of 535 |

In the running game (`--race --team feisar_c1`), `Data\Ships\feisar_c1\
Ship.vex` moves from "2 of 5 drawn material(s) resolved ... covering 3 of 12
chunk(s)" to "5 of 5 ... covering 12 of 12 chunk(s)", and every other team's
hull in the same frame reaches 100 % as well.

**What resolving buys, and what it does not - read off the newly-resolved
`diffuse_with_specular_from_alpha_n_vcol.rcsmaterial`'s microcode**
(`crates/render/examples/hd_ship_material_dump.rs`, `Ambient` key, `Static`
fragment `@0x3df0`). Its declaration does take `constantAmbientColour` and
both `directionalLight0*` parameters - so once resolved it is no longer
misclassified as emissive or ambient-only, and `mesh.wgsl`'s `NO_AMBIENT`/
`NO_SUN` bits (previously left unset by default, because they are only ever
set from a resolved declaration) are now the *read* answer instead of an
accidental default that happened to agree. **But the material's own name is
literal**: block `#49` is a third `TEX`, on **unit 2** -
`TEX H1.zw, R1.zzzz unit2` - which is the specular-from-alpha sample the
name promises, and `oag_render`'s `skin::picks`/`skin::roles` only ever bind
a colour and an alpha lane off units 0 and 1 (`docs/formats/rcsmaterial.md`,
"A texture slot names its sampler" above). So this material draws correctly
lit by the constant ambient and the sun now, but its own specular-from-alpha
term is not painted - a genuine, separate gap the fix does not close, not a
place the fix under-delivers. Confidence 80 on the microcode read (decoded
mechanically, one block, not cross-checked against a second material of the
same family). `specular_exponent()` reads `0.0` on this block, unpatched by
`SpecularPower` (not among the block's 8 declared parameters), so it falls to
`mesh::DEFAULT_SPECULAR_EXPONENT` - consistent with `docs/ghidra/functions/
ps3-hdfury-eu/renderer.md`'s "Ships have no Lambert diffuse either" bucket
for a real, unexplained `0.0`.

## Talon's Junction's magstrip floor was black because its material was never asked to resolve at all (2026-09-13)

**The player report was exact**: the flat-black road panel a few dozen units
ahead of `data/reference/hd-capture/talons-matched/03.png`'s camera is where a
magstrip section's own floor material sits, and it is a **second, distinct**
material from `etched_glass_tech` - `materials/mag_effect_loop_opaque.rcsmaterial`
(the base, opaque pass) painted under `materials/mageffectloop.rcsmaterial`
(a blended, `ALPHA_FROM_SECOND` glow on top), both naming
`mag_emiss_floor_seethru_talons.gtf` as their primary texture and
`dds/dc_iridescent_gradient.gtf` (the same ramp `etched_glass_tech` uses) as
their second.

**Identified by tint, confirmed by chunk-count.** `OAG_TINT_MATERIALS=1` at
the matched pose decodes the black region's pixel (`(255, 3, 74)`) to material
slot 330 at a squared error of 9.4 against a next-best candidate at 20.7 -
`mag_effect_loop_opaque`. `crates/render/examples/hd_slot_check.rs` on that
slot read **`no resolved variant`** and **`0 chunk(s)`**, even though a
752-triangle opaque draw calling itself out by that exact texture sits 0.98
units from the camera's forward ray (`hd_near_probe.rs`, `OAG_NEAREST=1`).

**Why zero chunks: the chunk-counting pass never walks a chunk's extra
surfaces.** `oag_rcs::rcsmodel::Mesh::extra_surfaces` is a second (or later)
material painted over the *same* geometry as a chunk's own `.material` -
`Mesh::surfaces()` is `once(self).chain(extra_surfaces.iter())`, and
`mesh/rcs.rs`'s real emit loop already walks it ("Every surface, as the
world-space pass does"), which is how the 752-tri opaque draw reaches the
screen at all. But `mesh/rcs/skin.rs`'s `variants()` and `flips()` each built
their own `chunks_of`/`decl_of` map by iterating `model.meshes` directly -
only the *first* surface of every chunk - so a material that is exclusively
an extra surface (never a chunk's own top-level `.material`, which
`mag_effect_loop_opaque` is: `mageffectloop` is the chunk's primary surface,
`mag_effect_loop_opaque` its extra one) reads as zero chunks and falls into
`variants()`'s "declared and never drawn" branch, which skips resolving a
variant for it - not a permutation the resolver asked for and missed, one it
never asked for at all. `roles()`/`picks()` then answer for a slot with no
resolved variant exactly the way they do for a genuinely-unused one: `packed
= slots::DEFAULT`, `Pick::default()` (entries 0/1), no `ADD_SECOND` /
`NO_AMBIENT` / alpha-channel reading - none of the material's own microcode
runs.

**Fixed**: both maps now walk `model.meshes.iter().flat_map(rcsmodel::Mesh::surfaces)`
instead of `&model.meshes` (`crates/mesh/src/mesh/rcs/skin.rs`). This is a
resolver-counting fix, not a shading addition - it lets `variants()` attempt
the lookup it already knows how to do, using the class/feature fallback and
`Static`-first order this page's "What selects a variant" section already
established, for a slot that used to be skipped outright.

**Measured, not just built.** `mag_effect_loop_opaque` now resolves to
`fragment@0x6ae0`, decoding to `ADD_SECOND | NO_AMBIENT`, `alpha_channel=3`,
`material_index=1` into `Model::emissive` - the second texture (the
iridescent ramp) is a tinted glow **added** to the albedo, un-ambient-lit,
still sun-lit. Disc-wide on this one circuit the fix moves Talon's Junction's
own report line from **286 of 302** drawn materials resolved (929 of 983
chunks) to **423 of 439** (1,759 of 1,813) - 137 more materials, because every
extra surface anywhere on the circuit was subject to the identical bug, not
only this one. A ship hull sample moved too (`feisar_c1/Ship.vex`: 12 of 12
chunks covered → 16 of 16) with `variants_unshipped` still 0 both times - the
bug existed on ships as well, it simply never left one of their materials
unresolved because no ship hull sampled happens to have an extra surface
whose material misses every fallback class.

**Screenshot verdict**: rendering `oag-game` at the matched frame `03` pose
before and after (`--camera-pose` from `03.json`'s own `render_with` line)
turns the panel from flat, uniform black into a textured surface with a
visible iridescent band where `ADD_SECOND`'s glow term is strong - closer to
the reference's continuous lit grey/white grid, but **not a match**: the
reference shows the whole panel lit, and this renders lit unevenly, still
dark over most of its area. That gap is not this bug. `mag_effect_loop_opaque`'s
resolved block (like `etched_glass_tech`'s own block #7, "Talon's Junction's
missing floor is a glass floor" above) is a five-sampler reflective combine -
units 0 through 4 - and this renderer's `skin::picks`/`skin::units` bind only
two of them (`first_unit`/`second_unit`). The other three - a specular-style
map at unit 3, the paraboloid-reflection-coordinate unit 2, a facing term at
unit 4 - are read by the fragment decoder and not routed to a texture bind,
the same "no route to draw it" gap this page already records for the glass
family under "The glass family's second slot: traced, not solved" and
"Talon's Junction's missing floor is a glass floor" above. **Not invented
here**: no stand-in was added for the unbound units - draw nothing for a role
nothing binds, per CLAUDE.md's rule.

**The falsifier, answered.** Does `mag_effect_loop_opaque` (or the same
extra-surface shape) draw correctly anywhere that is not a magstrip? Yes -
the fix is general, not magstrip-specific: the resolved-material count moved
on ship hulls too, which have no magstrip at all, so the *bug* is a plain
resolver gap and the *symptom* (black exactly at a magstrip) is this
particular circuit's own choice to author a magstrip floor as a two-surface
chunk (opaque base + blended overlay) rather than a coincidence with
`etched_glass_tech`'s separate, already-diagnosed glass-family gap. The two
are related only in that both bottom out in the same missing sampler
routing, not in cause.

**Collision class**: not checked directly this session - `docs/rendering/shadows.md`'s
`Floor`/`MagFloor` collision filter answers a physics query, not a draw-call
question, and the render-side identification above (tint plus chunk-level
geometry match, 0.98 units from the camera's own forward ray) already pins
the chunk without it. Left for whoever routes the remaining sampler units, if
it turns out to matter which collision class the same chunk carries.

**Confidence 95** on the root cause and the fix (read directly in
`crates/mesh/src/mesh/rcs/skin.rs`, confirmed by `hd_slot_check`'s
before/after and by the disc-wide report-line count change, gated by
`just`/`just test-data` both green - 3,452 and 4,242 tests, 0 failures,
`hd_ship_hull_material_ground_truth` and
`a_material_slot_never_needs_two_different_variants` both still pass). **Not
95+**: this is a code-level finding about this project's own renderer, not an
RE claim the confidence rubric's runtime-trace ceiling is written for, and
the remaining sampler-routing gap was not itself re-measured against RPCS3
pixel-for-pixel this session.

## A trackside wall panel drew solid black because its picture was its glow decal (2026-09-13)

**Amphiseum's two angled trackside wall panels either side of the road drew
flat black, at every position along the circuit that draws them.** Unlike
Talon's Junction's magstrip floor above, this is not a resolver miss: the
matched-camera pose's own load report already read **641 of 641 drawn
material(s) resolved** for this circuit before this fix, and still does
after it (640 of 640 building `track.vex` alone through `scene_from`, which
does not also walk the speedup/weapon-pad passes the race's own report
line does) - so "is it resolved at all" was never the question here.

**Identified by tint, the same way the magstrip floor was.**
`OAG_TINT_MATERIALS=1` at the matched pose (`data/reference/hd-capture/
amphiseum-matched/00.json`) decodes both panels' flat colour to
`materials/track_wall.rcsmaterial` (slot 549 of `track.vex`'s 641, hue
distance 1 against the next-best candidate). `OAG_ALBEDO_ONLY=1` reads the
same solid black - which for most materials would say "the art itself is
dark" (the technique `rcsmaterial.md`'s per-material probe section already
established), but is a **false read for a lightmapped material specifically**:
`OAG_ALBEDO_ONLY` clears `GpuVertex::lit`, which takes `mesh.wgsl`'s
"prelit" stand-in path and skips the lightmap/sun/ambient sum entirely, so a
lightmapped surface's *unlit* diffuse tells you nothing about its *lit*
appearance. `OAG_SKIP_MATERIAL=track_wall.rcsmaterial` and
`OAG_ONLY_MATERIAL=track_wall.rcsmaterial` were the deciding pair instead:
skipping it removes exactly the two panels (nothing else moves), and
isolating it alone reproduces the same solid black shape end to end - so the
geometry is real, resolved, and lit, and the picture itself is wrong.

**The material's own sampler table names four entries, and the renderer was
reading the wrong one.** `oag_rcs::rcsmodel::Material::samplers` for slot 549
is, in file order:

| Entry | Sampler hash | Name (preimage) | Bound `.gtf` |
| --- | --- | --- | --- |
| 0 | `0xb1f2a176` | `EmissiveTexture` | `track_wall_emissive.gtf` - **95%+ solid `(0,0,0)`** off a 50-point grid sample, one small hexagonal highlight |
| 1 | `0x3bdc0403` | `Texture1` | `ds_wall01_cs.gtf` - the panel's real diffuse+specular, a detailed grey structural texture |
| 2 | `0xa2d555b9` | `Texture2` | `ds_wall01_rh_n.gtf` - a tangent-space normal map |
| 3 | `0x37b5db58` | `lightmap` | this circuit's own baked-lighting atlas |

`mesh::rcs::skin::picks`'s lightmap branch - the one that runs for any
material `Material::lightmap_entry()` finds a lightmap on, which this one is
- does not read the microcode at all: it picks the surface's albedo as
"whichever entry is first, by position, that is not in `NOT_A_PICTURE`".
Nothing excluded `EmissiveTexture` before this fix, so entry 0 won by being
first, even though the material's *own* sampler name says outright that it
is a glow, not a picture, and entry 1's name says outright that it is. The
lightmap/ambient/sun sum this renderer computes for the slot was correct the
whole time; it was multiplying a near-solid-black decal into it instead of
the panel's real diffuse.

**Fixed by excluding the hash, the same way the glass family's ramps and
normal maps already are.** `EmissiveTexture` joins
`crates/mesh/src/mesh/rcs/skin.rs`'s `NOT_A_PICTURE` list - it is one of
the 38 preimages this page's own "sampler names, by preimage" table above
already carries, grouped there under "Light" beside `lightmap` and
`shadowMapTex`, and a disc-wide binding census
(`crates/render/examples/hd_emissive_texture_bind_census.rs`) agrees with
the name independently: swept over all seven archives, every one of its 71
distinct bound paths reads as a glow, an advert or a gradient by its own
file name - `*_emissive.gtf`, `*_e.gtf`, `advert_*.gtf`, `dc_grad*.gtf` -
never a plain diffuse. The exclusion only matters when a *better* entry
exists later in the table to fall through to: a material where
`EmissiveTexture` is the only populated entry is unaffected, because
`position` finds nothing past it and `unwrap_or` still answers entry 0.

**How much of the disc this shape covers**: exactly **25 lightmapped
material slots disc-wide** name `EmissiveTexture` positionally first with a
better entry after it
(`crates/render/examples/hd_emissive_first_census.rs`, swept over all seven
archives and 8,129 lightmapped materials) - all 25 on Amphiseum's own
`track.rcsmodel` and `track_reversed.rcsmodel`, split across the two files.
No other circuit's own `track_wall.rcsmaterial` (or any other lightmapped
material) authors its sampler table in this order, which is why this bug
never showed on Talon's Junction: that circuit's own `track_wall.rcsmaterial`
had a different, already-documented bug instead (the coordinate-flip case
`skin.rs`'s `flips` doc comment carries), on a file authored differently by
the same artist team.

**Measured before/after** at the matched pose (`scripts/hd-frame-compare.py
--pair-dir data/reference/hd-capture/amphiseum-matched --pose 00` renders;
before/after crop means are a direct pixel mean over the panel's own screen
rectangle, not the script's own road/sky/distant-geometry boxes, which
straddle other content too):

| Panel | Reference | Before | After |
| --- | ---: | ---: | ---: |
| Right (x∈[980,1250], y∈[380,550]) | 0.470 mean luma | 0.062 | **0.458** |
| Left (x∈[30,300], y∈[380,550]) | 0.165 mean luma | 0.274 | 0.292 |

The right panel goes from a 0.41-point gap to a 0.01-point one - solid black
to a close match. The left panel barely moves and stays brighter than the
reference; that box sits closer to the crowd stands
(`materials/nr_crowd_bustle.rcsmaterial`, a different material entirely,
already over-bright in both the before and after render) and to this
circuit's own overall darkness gap the "matched-camera comparison" section
of `renderer.md` tracks separately (+0.04 to +0.19 across Amphiseum's own
whole-frame regions) - neither of which this fix touches or claims to.
Talon's Junction pose 00 is unmoved: its own `track_wall.rcsmaterial` never
matches the `EmissiveTexture`-first shape, so `hd-frame-compare.py --pose 00`
on `talons-matched` reads the same numbers this fix started from.

**Confidence 92** on the cause and the fix (an exact sampler-table read
against a 38-preimage table already at confidence 90+, a disc-wide binding
census with zero counter-examples, and a before/after screenshot at the
matched pose) - short of the top band because the *why* a material author
would order the table this way on exactly one circuit's `track_wall`
remains unread; the *what* is not in question. Pinned by
`hd_amphiseum_wall_material_ground_truth.rs`
(`every_track_wall_slot_binds_its_own_diffuse_not_its_glow_decal`), confirmed
to fail against the pre-fix code.

**What is still open on these two panels**: the colour is closer, not
identical - ours reads a flat grey where the reference reads a brighter,
more saturated cyan/white, the same direction (not magnitude) as the
circuit-wide darkness gap `renderer.md` already tracks as a separate,
unresolved thread. Nothing here touches that thread's own candidates.

**The crowd stands are a different material, over-bright in both the before
and after render, and not this fix's to close.** Amphiseum's stands draw
through `materials/nr_crowd_bustle.rcsmaterial` (a cutout list, sampling
`hd_textures/crowd/crowd_avatars_22x4.gtf`, a tiled crowd-figure sprite
sheet) - not `EmissiveTexture`, not lightmapped, not touched by this fix
at all. A crop over the stands region at the matched pose reads **0.42-0.46
mean luma both before and after** against the reference's **0.18-0.19** -
the *opposite* direction from the panel bug (too bright, not too dark), and
consistent with the pair-level "Amphiseum reads +0.04..+0.19 brighter
overall" reading already recorded as the circuit's own general darkness/
brightness gap, not a symptom this material's own microcode was read for.

## A ship hull's own normal map stopped glowing as an additive layer (2026-09-13)

**`mesh::rcs::emissive::emissive` treated any material whose fragment program
structurally accumulates hardware unit 1 as a glow, whatever texture
`Pick::aux` actually loaded there.** On a ship hull that texture is
routinely the ship's own normal map - its tangent-space unpack (`MAD dst,
src.<component>, 2.0, -1.0`) trips the same self-referencing `MAD`/`ADD`
register-reuse shape a real additive glow has, feeding ordinary Blinn-Phong
lighting arithmetic rather than an accumulate. See
[hd-ship-materials.md](../rendering/hd-ship-materials.md), "Finding 1,
resolved", for the full mechanism, the disc-wide before/after counts, the
guard test and the player-eye screenshots - not repeated here.

**The one thing worth restating on this page**: the fix reads the role of
the sampler `Pick::aux` *actually* resolved to
(`material.samplers[pick.aux].0`), not of whatever a `Declared`
cross-reference says sits at hardware unit 1 independently of `Pick::aux`.
The two disagree on `tunnel_fx_noalpha` - it declares `SpecularTexture` at
unit 1 and its own `EmissiveTexture` at unit 2, while `Pick::aux` (no
lightmap, an untraced alpha lane) resolves to the emissive one, the texture
this material genuinely glows with. Asking about hardware unit 1 there would
have refused a real, working glow - measured before landing, not assumed.
Confidence 90.

## The glass floor's routing question is settled, and it was neither open hypothesis (2026-09-17, `lane-hd-glass`)

**Read directly off the disc, not inferred.** `hd_glass_sheen_census.rs --params etched_glass_tech` against
`talons_junction/track.vex` gives `etched_glass_tech.rcsmaterial`'s own sampler table in table order:

```text
0x94b2b285 (no preimage, the facing ramp) -> dc_iridescent_gradient.gtf
0x3bdc0403 (Texture1)                     -> glass_etched_tech.gtf   (the etched grid)
0x37b5db58 (lightmap)                     -> None
```

and the resolved lit-race variant (`fragment@0x5ef0`, `StaticQuake` class, feature `0x56c94426`) declares
exactly `0x3bdc0403@unit0`, `0x94b2b285@unit2`, `0x9edd3243@unit1` (`paraboloidReflectionTex`) - both real
textures, at the exact units block #7's traced microcode samples them ("Talon's Junction's missing floor is a
glass floor" above, `@0x2b`/`@0x19`).

**The grid binds `Texture1`, never `lightmap`.** Two sections above ("A texture slot names its sampler" at
line ~1292 and "Acted on 2026-08-24, additively") state that `glass_etched_tech.gtf` names sampler `lightmap`,
which no variant declares - that claim is **stale**, superseded the same day by "Corrected hours later"'s
pairing fix a few sections earlier on this same page, and was never updated to match it. Left as written there
rather than edited, per this project's own rule that a correction is a new dated section, not a silent rewrite;
this section is that correction. The `lightmap`-hash entry is real, but it is the material's **third** sampler
entry and carries no path (`None`) - an ordinary "left at whatever the engine binds" entry, per
`oag_rcs::rcsmodel::Material::samplers`'s own doc, not a binding for the grid at all. A pre-correction
off-by-one pairing is what put the grid's path beside that empty entry's hash in the earlier reading.

**So neither hypothesis the open routing question stood on holds.** The
record's word is exactly the binding it appears to be (`Texture1`, not `lightmap`), and the variant this
project resolves is the right one - it already declares both of the material's real samplers, at the units the
traced microcode reads them from. Confidence **95**: read directly off the shipped record and the shipped
variant, on the same terms "Talon's Junction's magstrip floor was black" above claims that number for - a
code-level finding about this project's own renderer, not an RE claim the confidence rubric's runtime-trace
ceiling is written for.

**The real gap was downstream, in `mesh::rcs::skin::picks`, and it is a `Pick` collision, not a hash
mismatch.** This material's colour lane traces to more than one unit (`Texel::Mixed`), so the load-time albedo
fallback (`NOT_A_PICTURE`, above) picked the grid - the one entry not on that list - for **both** of this
renderer's bindings: the alpha lane also resolves to the grid's own unit, so `aux` collapsed onto `albedo`
(`.filter(|&i| i != albedo)` clears it, then `.or(default.aux)` puts it right back on the same entry), and the
ramp - the material's own primary `Material::texture` - was never bound to anything. `crates/mesh/src/mesh/rcs/glass_sheen.rs`
is the fix: a fact-based classifier (exactly `{Texture1, one facing-ramp hash, paraboloidReflectionTex}`
declared, colour `Mixed`, alpha at `Texture1`'s own unit - none of it a material name) that routes `picks()` to
`albedo = ` the ramp entry, `aux = ` the grid entry instead, ahead of the generic fallback. Swept disc-wide
(`hd_glass_sheen_census.rs`, all four PS3 archives): **6 materials, 15 chunks**, three circuits -
`etched_glass_tech.rcsmaterial` (Talon's Junction, `tech_de_ra`) and its sibling `etched_glass.rcsmaterial`
(`modesto_heights`), every one keyed on the same ramp hash (`0x94b2b285`) - the other two facing-ramp hashes
never co-occur with `Texture1` and `paraboloidReflectionTex` in one declared set, so the classifier does not
pull in the unrelated `blue_metal` family.

**`mag_effect_loop_opaque.rcsmaterial` does not classify, and that is correct, not a miss.** Its own resolved
block (`fragment@0x6ae0`) declares five units - a fourth real texture (`ds_mag_wave_c.gtf`, unit 3), the grid
and the ramp on *different* units than here (grid at unit 1, ramp at unit 4), a fog-idiom wrapper
(`EX2_SAT` against a `1.44269`-scaled term - the shape `scripts/ps3-microcode.py`'s own docstring names), and
an alpha lane this project's decoder reads as `Texel::Untraced` rather than a resolved unit. "The same
five-sampler reflective combine as `etched_glass_tech`'s" (this page's 2026-09-13 section, above) is true only
as a family resemblance - close enough to place the routing gap there, not close enough to reproduce it.
Drawing it from `etched_glass_tech`'s own traced combine would be inventing a program this project has not
read, which `CLAUDE.md` rules out; `glass_sheen::classify` does not match it, and it is left classified as "a
different combine" but undrawn. `docs/formats/rcsmaterial.md`'s own line ~710 read `op3D` as seen only in
`etched_glass_tech`; `mag_effect_loop_opaque`'s block #7 carries it twice more (`@0x43`, `@0x60`, both writing
the discard register `R63`), a free correction found while tracing it for this comparison.

**Verified as a player would, and it is strictly additive.** `oag-game`'s recovered camera at
`data/reference/hd-capture/talons-matched/03.json` frames the *magstrip* panel, not this material (confirmed
2026-09-13, above) - rendering that exact pose before and after this change is **byte-identical**
(`frame03-before.png`/`frame03-after.png` under `data/reference/hd-capture/talons-glass-lane/`, gitignored;
1,175,040 of 1,175,040 pixels equal). `model_probe --pose 76.9,-46,148 OAG_ONLY_MATERIAL=etched_glass_tech`
(the pose this page's "Talon's Junction's missing floor is a glass floor" section already used) shows the same
six chunks - the material's alpha, and therefore its screen coverage, is byte-for-byte unchanged (`grid.r` was
already the alpha before this change, reached through `ALBEDO_FROM_SECOND`'s absence rather than
`ALPHA_FROM_SECOND`'s presence, the same value through a different path) - only their colour moves, from the
flat grid picture to the traced sheen: `vertexLight * ramp + ramp`, plus the grid's red as an additive term and
the output alpha, reproducing visible iridescent colour variation across the ramp's own hue range rather than a
flat picture.

**Three things this pass leaves out, all named rather than guessed shut.**

1. **The per-material constant `c`** (`ADD H4.xyz, H2, {c}` in block #7 - parameter `0x512f8e65`, no preimage).
   Read off the model's own parameter table: `[0.26562, 0.26562, 0.26562, 0]` on `etched_glass_tech` - real and
   non-zero, not a no-op to drop silently. `ramp` stands in for `ramp + c` in `mesh.wgsl`'s combine. Not wired
   this pass: it needs the same per-material plumbing `GpuVertex::specular_exponent` already has (a vertex
   field, a `vertex_attr_array!` slot, threading through `MaterialSetup`), and `mesh.rs`/`mesh/rcs/skin.rs`/
   `mesh_render.rs` had 10, 59 and 88 lines of headroom respectively under `scripts/check-file-size.py`'s
   1,000-line cap - not enough to add a fourth per-material float in the same change as the routing fix without
   a further split none of those files' own shape suggested cleanly. Next action: add the field the same way
   `specular_exponent` did (patch-checked against this parameter's hash via `Program::patches`), then read it
   in the combine in place of the bare `ramp`.
2. **`vertexLight`'s exact composition is not fully reproduced.** Traced, it is
   `f[TC0] + f[TC1] + f[TC5].x * (sun.colour * N.L)` - the third summand is read outright (`@0x02`/`@0x05`:
   `DP3_SAT` against the sun direction, `MUL` by the sun colour, exactly `scene.light.sun * ndl` this file
   already computes as `sun_diffuse`) and wired as such, weighted by `in.texcoord.x` (`f[TC5].zw` is already
   established as `in.texcoord`, line 830 above, making `f[TC5].x` the same attribute's first component).
   `f[TC0]` and `f[TC1]` are a stated approximation, not a reading: both stand in as `in.colour.rgb`, the one
   per-vertex light term this project already decodes for every HD material - on the same terms this file's
   own second-texture coordinate ("The second texture is sampled at the diffuse coordinate...") is already a
   stated approximation rather than a reading. The paired **vertex** program (which would say what `f[TC0]` and
   `f[TC1]` actually carry) was not traced this pass.
3. **`paraboloidReflectionTex`'s reflection tint, weighted by the grid's red**, is left out entirely - this
   renderer has no dual-paraboloid probe and does not invent one, confirmed again here: neither material's own
   sampler table carries `0x9edd3243` at all (the hash names an engine-supplied probe, not a `.gtf`), which is
   the file's own shape for "not authored here", not a gap in reading it.

## The magstrip floor scrolls a wave texture off the engine clock (2026-10-05, `lane/magstrip-floor-anim`)

**The player report**: "Magfloors themselves also have some (texture?) animations
on their own, at least in the latest title." The law is in the fragment programs,
and it is one law for the whole family.

**Which materials.** `crates/render/examples/hd_mag_params.rs` sweeps all four PS3
archives for fragment variants declaring the wave sampler `0x85c9fd48`
(`ds_mag_wave_c.gtf`), the emissive sampler `0x1202d8df`, `time` (`0x906b67ba`),
`Colour` (`0x02ab9f07`) and a scale `k` (`0x220cf0e6`) together: `mageffect08` and
`mageffect08_floor` (Talon's Junction, Amphiseum, Modesto Heights, `02_track`,
`03_track`), `mageffectloop`, `mag_effect_loop_opaque`, `chevron_pulse` and
`mageffect_modded` (Ubermall). Talon's Junction's wall `mageffect08` (94 variants)
declares the wave and `Colour` but **not** `time` or `k`: it is not scrolled by this
law and is left alone.

**The microcode** (`scripts/ps3-microcode.py fp-file`; `mageffectloop` block `@0x6b20`,
`mag_effect_loop_opaque` `@0x6ae0`, `mageffect08_floor` `@0x61d0`, all lit blocks;
unlit block #2 of `mageffectloop` has the same lines):

```text
MAD  R.xy, uv, {k}, time      ; time added, no multiplier, to BOTH axes
TEX  wave, R.xy               ; ds_mag_wave_c.gtf (unit 3, or 4 on mageffect08)
TEX  e, uv                    ; the emissive picture (1202d8df)
m = e.rgb + e.a * (wave - e.rgb)
d = e.rgb / (1 - m * Colour)  ; MAD 1 - x*c, RCP, per channel
```

`time` is the engine's seconds clock ([renderer.md](../ghidra/functions/ps3-hdfury-eu/renderer.md),
"the engine's own parameter table"), so the wave **repeats exactly once a second on a
diagonal**: measured on `oag-game`, frames 60 ticks apart match to 1 pixel of the
panel region, frames 15, 30 and 45 ticks apart differ by 49k, 59k and 62k.
Talon's Junction authors `k = 0.2` and `Colour = (0, 1, 0.147)` on the loop pair and
`(0, 0.861, 0.127)` on `mageffect08_floor`: only green and blue take the dodge, which
is why the panels swing to green-white as the band passes. The wave texture is a
horizontal bright band (about 10% of its height) over vertical streaks. Talon's
`mag_emiss_*` pictures carry alpha 255 everywhere, so `m` is the wave itself there;
the lerp is implemented as read for circuits that author real alpha.

**The floor loop pair's combine** (`mag_effect_loop_opaque`, `mageffectloop`, which
this page called "open" on 2026-09-17), same blocks:

```text
grid = tex(a2d555b9).x                ; glass_etched_tech.gtf, unit 1
ramp = tex(cc98c527, -dot(N, V))      ; dc_iridescent_gradient.gtf, unit 4
colour = vertexLight * grid * (ramp + c) + (grid + ramp) * d     ; c = 0x6c57ba63
alpha  = grid
```

`vertexLight` is the same `f[TC0] + f[TC1] + f[TC5].x * sun * N.L` the glass sheen
reads (the same approximation: `in.colour`). `c` is `0.67188` here. Left out, named:
the `paraboloidReflectionTex` term (unit 2, engine-supplied, no probe here), as
`glass_sheen` leaves it.

**Wired as** `mesh::rcs::mag_wave` and `slots::MAG_WAVE` / `slots::MAG_LOOP`
(`MATERIAL_SHIFT` 17 to 19): the emissive picture rides in the third binding (the
`pad_masks` slot, which no wave material shares with a pad), the wave in a new
fourth (`Model::wave_maps`, binding 4), `Colour`, `k` and rate 1 in the material's
`Emissive` entry (and `c` in its `offset` on the loop pair). The matched pose
`talons-matched/03` now draws the blue grid with translucent panels the original's
frame shows, where it drew black over a rainbow ramp before; `ADD_SECOND` is cleared
on these materials because their second texture is not an additive glow.

**Chosen, not measured** (no confidence): the dodge's divisor is floored at 0.05 and
the result clamped to 1, because `Colour.g = 1` and the band reaches 1 so the quotient
diverges and a black texel gives `0/0`; the RSX saturates at 8 bits. The dodge is
taken on sRGB-decoded samples, the domain the existing glow uses, and encoded for the
gamma path. `uv` stands for `f[TC3]`/`f[TC5].zw` on the same unproven-equality terms
`skin::roles` already states.

**Tests.** `crates/render/tests/hd_mag_wave_lit_path.rs` (no disc: the pixel swings
with `scene.time` and repeats at +1 s) and `hd_mag_wave_ground_truth.rs` (Talon's
builds the bindings and classifies the loop). **Not done:** a reference capture of
the moving strip from RPCS3 (the stills agree with `03.png` in layout only; the rate
is from the code), 2048's Vita `mageffect08` program, and the `mageffect08` floor's
own diffuse-plus-normal-plus-lightmap composition (only the wave term is added to it).

## A Zone race draws the magstrip family without its wave (2026-10-05, `hd-zone-magfloor`)

**The report from play:** a Zone race on Talon's Junction no longer looked like it did before the wave
landed. Cause: `slots::MAG_WAVE`/`MAG_LOOP` replaced the shaded result outright, so the strip floor drew
exactly as in Time Trial while the walls and sky took the Zone grade.

**What the original does.** The six wave materials (`mageffect08`, `mageffect08_floor`, `mageffectloop`,
`mag_effect_loop_opaque`, `chevron_pulse`, `mageffect_modded`) each ship `ZoneMode` fragment variants. Those
declare `zoneColourTint`, `zoneEffectInner` (and `Outer`), the zone textures, the grid and the paraboloid
reflection - and **no wave sampler (`0x85c9fd48`), no emissive picture (`0x1202d8df`), no `time`, no iridescent
ramp (`0xcc98c527`)**. Read with `scripts/ps3-microcode.py fp-file` on Talon's `mag_effect_loop_opaque`,
block #15 (`@0xbb40`): the colour is `light * (zoneTex * zoneEffect + zoneBase * rim^10 + zoneBaseAlt *
rim^5 [+ grid-alpha glow]) + reflection`, the same Zone surface every other material gets. So in Zone the
strip is recoloured by the stage, not scrolled. Census: `crates/render/examples/hd_zone_variants.rs` prints
the per-variant table; `hd_zone_wave_census_ground_truth.rs` asserts no Zone variant of any wave material
declares the wave, across all four PS3 archives. Not one exception, so one gate serves the family.

**Wired as** a `scene.zone.enabled == 0.0` condition on both branches in `mesh.wgsl` (the wave dodge and the
floor combine); the floor then falls through to the generic Zone surface. A branch, not a mix: the Time Trial
frame at the lead's matched pose is byte-identical before and after. Test:
`hd_mag_wave_lit_path::a_zone_race_draws_the_strip_without_its_wave` (fails without the gate).

**Not done:** an RPCS3 Zone frame of the strip for a side-by-side (the decoded program is the evidence); the
Zone variant's own grid-alpha glow term and paraboloid reflection are not drawn (as off Zone); whether the
Zone floor's alpha (`@0x82 MOV H0.w`, a literal the decoder prints as 0) means anything for a blended pass.

## Omega and 2048 draw their see-through materials off the state word (2026-10-05, `transparent-floors`)

Omega and 2048 authored HD's state word on every material (PS4 header `+0x22`, Vita `+0x12`; evidence
and the draw-order key in [`material-state.md`](../ghidra/functions/ps4-omega-eu/material-state.md)),
and `oag_mesh::mesh::rcs::psp2::transparency` routes a textured draw with mode 1 to the blended list and
mode 2 to the alpha-tested list, where before **every** Omega and 2048 draw was opaque.

- **Law recovered:** mode bits as HD (confidence 75), sort priority bits 9 to 11 (75, not yet used for ordering).
- **Chosen, not measured:** the blend equation (alpha-over; Omega's header carries no factor pair - the additive
  family is now HD's own pair by name, see the next section) and the alpha-test reference `0.5` (HD's).
- **Looks:** Tech De Ra's glass tubes now show the crowd through them and its road panels take their see-through
  layer; 2048's cockpit glass and billboard signs draw. Frames `data/scratch/transparent-floors/shots/{before,after}_{tdr,2048}_300.png`.
- **Not done:** HD is untouched (its own factor path); the `etched_glass_tech` sheen and `Transparency` param on
  Omega are not read; GCN pixel programs were not read for an alpha source, so a blended draw uses the first texture's alpha.

## Omega and 2048 blend with HD's own factors where the material name is HD's (2026-10-05, `omega-2048-materials`)

The previous section drew every Omega and 2048 blended material alpha-over and left the additive family a
dim sheet. This pass looked for the real equation and found it nowhere in the two titles' own data, then took
the one honest source left: **Wipeout HD authors a factor pair beside the state word, and the three discs
share material names.**

- **Not found in Omega or 2048.** Omega's header holds no pair (see the evidence page
  [`material-state.md`](../ghidra/functions/ps4-omega-eu/material-state.md)). In 2048 the blend is a
  `SceGxmBlendInfo` given at runtime to `sceGxmShaderPatcherCreateFragmentProgram`, whose single wrapper
  `FUN_812f6bee` has 22 callers and none is the model-material pass
  ([`fragment-programs.md`](../ghidra/functions/vita-2048-eu-v104/fragment-programs.md), 80). The one blend table
  readable there (a sprite batcher) holds alpha-over, `SRC_ALPHA`/`ONE` and a reverse-subtract, so the engine
  does draw additive, which the state word cannot say. No GCN pixel program was read: no decoder exists, so the
  **alpha source stays the first texture's alpha, chosen, not measured**.
- **Census (disc, `crates/rcs/tests/hd_lineage_blend_ground_truth.rs`).** HD's blended materials author
  `SRC_ALPHA`/`ONE_MINUS_SRC_ALPHA` on 1,648 of 2,362 and the rest `SRC_ALPHA`/`ONE` (348), `ONE`/`ONE` (144),
  `ONE`/`ONE_MINUS_SRC_ALPHA` (142), `SRC_COLOR`/`ONE` (57), and two odd pairs. Over Omega's five base archives 1,738
  materials are mode 1 on 211 names, 2048's base package 1,481 on 134; 156 and 92 of those names exist in HD. Three HD names
  author two pairs (`basicalpha`, `lambert`, `dc_lightcone`) and are left alone.
- **What ships.** `oag_rcs::rcsmodel::psp2::lineage_blend::INHERITED`: 70 names whose HD pair is single and not
  the default, each a name Omega or 2048 draws in mode 1; `psp2::transparency::route` gives such a draw HD's
  pair through the same `blend_state` HD's own path uses, and every other blended draw stays alpha-over. 363 of Omega's
  1,738 and 260 of 2048's 1,481 blended materials take an inherited pair (`Report::inherited_blend_draws`
  counts the draws, and the load report says so). The ground-truth test rebuilds the table from the three discs, so a dropped,
  added or edited row fails. **Inherited from HD, not measured on Omega or 2048**: no confidence score.
  Names only the two later titles have (388 Omega and 893 2048 blended materials, `fc06_lambert_alpha`,
  `fc01_emissive_alpha_emistint`, `2048_ship_glass_dg`, `2048_engine_additive`, ...) draw alpha-over, chosen.
- **Looks.** Tech De Ra's start beam (`cf_startbeam_glow`, `SRC_COLOR`/`ONE`) and the hex glass band
  (`glass_texture`, `ONE`/`ONE_MINUS_SRC_ALPHA`) change; most of the inherited names are weapon and shield
  effects that a stationary lap never meets, and Altima and Tower, whose floors are `fc01`/`fc12` families HD
  does not have, are byte-identical. HD's `talons-matched/03` pose is byte-identical before and after (the
  code path is Omega and 2048's only).
- **A third mode.** 16 2048 materials (`fc06_lambert_alpha`) carry low bits `3`, which `Material::mode`
  reads as none and draws opaque; on HD's register reading that is a blend and an alpha test together. Not wired.

## Open

- **63 of the 125 sampler hashes**, after the wider sweep below - down from
  the 87 this section used to carry, which was stale: it never folded in the
  `zoneTexInner`/five-more-names batches this same page already recorded
  above. The three commonest residual hashes
  (`0xd5e000d1`, `0x00e5b679`, `0x1f6f85a3`, ~4,400 uses each across every
  unit) are `zoneTexInner`/`zoneTexInnerNearest`/`zoneTexVis` themselves and
  so are **not** part of the 63 - they were the correction, not new debt.
- **197 of the 300 parameter hashes**, likewise down from 236.
- **2026-09-16 preimage sweep, done properly this time**
  (`crates/render/examples/rcs_preimage_sweep.rs`, reproducible with
  `cargo run --release -p oag-render --example rcs_preimage_sweep -- <image>
  <decrypted EBOOT.elf> <decrypted DFEngine.prx>`): four candidate sources -
  `EBOOT.elf`'s own identifier-shaped strings, `DFEngine.sprx` decrypted the
  same way (`rpcs3 --decrypt`, which works on it exactly as it does on
  `EBOOT.BIN`), every identifier-shaped string inside every
  `.rcsmaterial`/`.rcsmodel`/`.vex` on the disc, and a generated vocabulary
  (the known names' own tokens, recombined camelCase/PascalCase/snake_case,
  with digit suffixes and `_vp`/`_fp`/`Tex`/`Texture`/`Map`/`Colour`/`Color`
  affixes). Named **15 more samplers** (13 from `EBOOT.elf`, 2 generated:
  `Pitch`, `textureSpot0Tex`, `textureSpot0ShadowTex`, `Rock`,
  `alpha_emissive`, `Electricity`, `Wave`, `GradientColour1`,
  `paraboloidIblTex`, `ambientShadowTex`, `textureSpot1Tex`,
  `zoneAnisoPalette`, `textureSpot1ShadowTex`, `zoneAnisoPaletteOuter`,
  `screenSpaceReflectionTex`) plus **1 on a second pass**
  (`TextureGradient`, once the first batch's own tokens joined the
  vocabulary) - 46 to 62 of 125. And **39 more parameters**, all from the
  generated vocabulary alone (`colour1`, `SpecPower`, `ShadowAlpha`,
  `SpecularScalar`, `SpecularColour`, `spec`, `SpecScale`, `spec_power`,
  `SpecularColor`, `DiffuseColour`, `diffuse`, `power`, `NormalPower`,
  `FresnelMin`, `EmissiveColour`, `ColourTint`, `DiffuseColour1`,
  `ShadowColour`, `Scaler`, `DistortionSpeed`, `VScale`, `Reflection`,
  `SpecularPower`, `FresnelScale`, `scale1`, `colour2`, `VSpeed`, `Spec`,
  `power1`, `Speed2`, `FresnelPower`, `SpecColour`, `SmokeScale`, `colour3`,
  `noiseScale`, `Specular`, `min1`, `Emissive`, `SmokeScale1`) - 64 to 103 of
  300. Zero ambiguous (no two distinct candidate strings landed on the same
  hash) across both sweeps. All names now live in
  [`oag_rcs::rcsmaterial::names`](../../crates/rcs/src/rcsmaterial/names.rs),
  looked up rather than re-derived, with a unit test that every entry
  round-trips through its own hash and a ground-truth test
  (`rcsmaterial_ground_truth.rs`) that every entry's hash is one the disc
  itself declares or authors.
- **Confidence, by source.** `EBOOT.elf`-sourced names sit at confidence 90 -
  a 32-bit hash match against a string demonstrably shipped in the
  executable, the same standard `viewProj`/`fogColour` were confirmed at in
  [renderer.md](../ghidra/functions/ps3-hdfury-eu/renderer.md). Names from the
  generated vocabulary alone sit at confidence 82: still a genuine preimage
  (the false-positive rate over ~90,000 candidates against a ~260-hash
  residual population is on the order of 1 in 200 across the whole batch,
  not per name), but without the independent "this string ships somewhere"
  corroboration the executable gives.
- **The discriminating target the sweep was also run against**:
  `0xce5c4410` (`Weapon Pad`'s own colour parameter) was already named
  `W_Cycle` by the 2026-08-31 parameter sweep above, in this same page - the
  miss was that [pads.md](../rendering/pads.md) never cross-referenced it.
  `0x7611a2d8` (`Speedup Pad`'s parameter on two circuits) stays unresolved
  even after this wider sweep. See pads.md's 2026-09-16 update: naming the
  hash was never going to settle "is this the glow colour" on its own
  (`W_Cycle` turns out to sit on files still called `weapon_pads.rcsmaterial`
  even where the geometry is a `Speedup Pad`) - reading the *value* across
  every circuit is what settled it.
- **The remaining 63 sampler / 197 parameter hashes.** The residue is not
  arbitrary: the named ones are artist-facing (`Colour`, `Speed`,
  `Brightness`) and the unnamed ones cluster on one material each, which is
  what a name that never reaches the executable's own string table looks
  like.
- **The `time` scroll is read and unwired**, and what blocks it is the additive
  second-texture layer rather than the scroll itself. `uvScale`/`uvOffset` are
  read and unwired too and are **not** the cheaper first step they looked like:
  2,510 of 2,582 records author the identity.
- **The microcode**, which is where the operation - multiply, add, replace -
  actually is.

## The PS4 Omega Collection wraps this in a different, unread container

**2026-09-16, `lane/omega-rcs`.** Same finding as [rcsmodel.md's own PS4
section](rcsmodel.md#the-ps4-omega-collection-a-different-container-not-a-byte-swap-of-this-one),
which has the full evidence; this page carries the summary specific to
`.rcsmaterial`.

Every real (non-all-zero) `.rcsmaterial` sample on `omega-ps4-eu`'s five
`dataNN.psarc` archives opens `e5 ad 5c ca` at `+0x00` - `0xCA5CADE5`
little-endian, the same `0xCA5CADxx` tag family `.rcsmodel` carries as
`0xCA5CADED`. **Confidence 85.** This is not the PS3 container: this page's
own "The container" section above describes a 0x0c-byte header naming a
variant table and a `0x40`-byte-per-record layout, and no field of a tagged
PS4 sample matches that shape at any byte order. `crates/assets/examples/psarc_oracle.rs`
now scores the tag the same way it already scores `.vex`'s `VEXX` and
`.gnf`'s `GNF `, turning "no magic, zero-vs-nonzero only" into a real
valid/all-zero/garbage split - **439 of 1,270 `.rcsmaterial` entries carry
the tag** disc-wide; see [psarc.md](psarc.md#block-data-location-and-the-short-read-extraction)
for the per-archive table.

**The names inside still resolve.** `crates/rcs/examples/ps4_hash_scan.rs`
found at least one known sampler/parameter `~crc32` preimage (this page's own
"The sampler names" / "The parameter names" tables, and
[`oag_rcs::rcsmaterial::names`](../../crates/rcs/src/rcsmaterial/names.rs))
in **386 of the 439 tag-confirmed files (88%)**, 2,451 hits total - `Texture1`/
`Texture2`/`Texture3`, `ShadowColour` (at the identical `+0x724` offset on
every hit, which a byte-position coincidence does not produce), `power`,
`Colour_Tint`, `EmissiveTexture`, `lightmap` and `ColourAnim` among the
matches. Same reading as `.rcsmodel`: whatever the PS4 container turns out to
be, the material data inside it hashes names in the same namespace PS3 uses.

**No parser exists for this container either**, for the same reason: a
materially different, unread layout from a handful of samples is a
reverse-engineering project of its own, and this session's evidence supports
measuring and naming the container's existence, not guessing its fields.
`oag_rcs::rcsmaterial`'s existing container reader is untouched.

## HD's light cone was a grey wedge; its combine is two taps and a saturate (2026-10-05)

`dc_lightcone.rcsmaterial` (Talon's Junction slot 440, Amphiseum slot 610 -
exactly two materials disc-wide, `crates/render/examples/hd_light_cone_census.rs`)
drew as opaque grey radial wedges across the upper left of
`talons-matched/03`, where the original draws pale translucent streaks over
the blue tunnel. Cause: the picture was `Texture1` (`dc_gradient_noise.gtf`,
grey, alpha 255 everywhere) with the texture's own alpha as coverage.

**The program** (resolved fogged variant, `scripts/ps3-microcode.py fp-file`;
confidence 85 for the arithmetic, read instruction by instruction):

    colour = fog-lerp(noise.x * K)         K = parameter 0x60eaf40d, 100 on Talon's Junction, absent (1) on Amphiseum
    alpha  = noise.x * s * ramp(N.V)       s = parameter 0x7611a2d8, 1.0 on Talon's Junction, 0.49596 on Amphiseum
    N.V    = (N . V) / sqrt(|N|^2 |V|^2)   ramp = dc_gradient_e.gtf at (cos, cos); its red is a function of u alone, 0 at 0 and 1 from 0.5 up

The vertex program writes the attribute in slot 1 into the `w` of three
interpolators and `eyePositionWorldSpace - position` into `TC2.xyz`. **That
attribute is a constant `(0, 0, 1)` on every authored cone** (a `CMP` field,
raw bytes `7f 80 00 00` in every vertex, `hd_unlit_probe`), not a surface
normal: the fade is against a fixed axis. Confidence 90.

**The colour saturates before it blends.** With `K` = 100 an unclamped
colour on this project's float target is `100 * noise` of light times the
alpha, and it drew a white wall; the original's 8-bit surface clamps to 1
first. The clamp is the cone's own in `mesh.wgsl`, not a global one.

**The ramp tap is predicated on Talon's Junction's variant** (`@0x1550`: `FENCT
R63, R0, R0` then `TEX H2.x, R2.wwww unit1 [NE(wwww)]`) and not on
Amphiseum's (`@0x1520`). `H2.x` already holds the noise, so a skipped tap
makes the alpha `s * noise^2`. `R0` is `f[TC1]`, whose `w` is the normal's
`y`, which is zero on every cone. **Confidence 60, one matched frame**: that
`FENCT` sets the condition register from its source is a hypothesis (the
opcode writes no register, 59,256 of 59,256 uses name 63). The alternative
reading draws the shafts as an opaque white wall; the skipped-ramp reading
reproduces the translucent streaks of `talons-matched/03`. Amphiseum's
variant, whose tap is unconditional, is read from its microcode only: no
matched pose frames its cone.

Implemented in `mesh::rcs::light_cone` (`slots::LIGHT_CONE`, bit 19, which
moved `MATERIAL_SHIFT` from 19 to 20) and `mesh.wgsl`; pinned by
`hd_light_cone_lit_path` (fixture) and `hd_light_cone_ground_truth` (disc).

## A lightmapped material's picture must be a sampler its variant declares (2026-10-07, `hd-vineta-floor`)

**The report** was the maintainer's, from play on Vineta K (tick 2820, world `[-839.8, -146.6, 215.0]`): "the floor
texture seems off compared to the original". **Reference:** an RPCS3 capture of the same circuit, eight shots, under
`data/reference/hd-capture/vineta-floor/` (shot `03`, eye `[-861.6, -145.6, 179.0]`, looks at the reported spot;
untracked, `data/` is gitignored). It shows a grey road with a blue stripe and drain grates across the whole width.
Ours drew the **left third as black glass with cyan ripple rings**.

**Cause (measured).** The road tile, chunk 1051 of `01_vineta_k/track.rcsmodel` (DATA02), has six surfaces, each its
own material. Surface 5 is `d_s_n_customr` with, in file order: `ds_dualparaboloid_c` (512x256 reflection map,
sampler `0x8365b1f3`), `ds_floorplain_wet_cs` (2048x512, `0x3bdc0403`), `ds_floorplain_waterleadin_n` (`0xa2d555b9`
normal) and the lightmap. Its resolved variant declares `{0x37b5db58: unit 2, 0x3bdc0403: unit 0, 0x9edd3243: unit 3,
0xa2d555b9: unit 1}` - **`0x8365b1f3` is declared by no sampler**, the engine binds that slot itself. A lightmapped
material skipped the microcode read in `skin::picks` and took "the first entry that is not a lookup", which is the
reflection map. Skipping chunk 1051 removes the black; drawing the reflection map as albedo is the black-and-ripples.
This is the `Pick` collision of the glass-floor case (a), not a UV, decode or combine fault. Its colour lane is
`Texel::Mixed`, so the unit trace cannot name the picture either; the declared-sampler filter does.

**Fix.** `skin::lightmapped_albedo`: the first non-lookup entry **among those the resolved variant declares**, the old
rule where no variant resolves. Before/after at the capture's own camera (`hd-frame-compare`'s comparison setting):
`data/scratch/hd-vineta-floor/before_03.png` / `after_03.png` beside `pair/03.png`.
Pinned by `crates/render/tests/hd_lightmapped_albedo_ground_truth.rs` (fails with the filter removed).

**Disc-wide census** (`crates/render/examples/hd_lightmap_albedo_census.rs`, all 28 circuit models, 8,129 lightmapped
materials with a resolved declaration): the albedo entry changes on **14** slots, three distinct materials:
`d_s_n_customr` (`ds_floorplain_wet_cs`, `ds_floorchevron_wet_cs`; Vineta K both directions) and
`sebenco_ice` (`and_ice2` -> `and_ice1`; Sebenco Climb reversed, a DLC material, **same undeclared `0x8365b1f3` at
entry 0, no reference frame taken**). Nothing else on the disc is touched.

**Still open on this frame** (not this lane): the hex windows show teal sea in the original and black in ours (sky
lane); the wet floor's reflection (`ds_dualparaboloid_c`, an engine-bound slot) is not drawn, so the floor is the
albedo and lightmap only, which matches the capture to the eye.

**2048 / Omega check: not checkable.** The rule reads a PS3 `Declared` sampler table out of an NV40 microcode block;
2048's Vita and Omega's PS4 shader containers are unread here (see the two sections above), so whether they carry an
engine-bound slot like `0x8365b1f3` cannot be asked.

## Vineta K's ceiling: the tunnel glass reads the screen, and the arch lights lost their glow (2026-10-07, `hd-vineta-ceiling`)

**The report** was the maintainer's: meshes on Vineta K's ceiling "that should not be there". **Reference:** RPCS3
`data/reference/hd-capture/vineta-teleport/pair-00.png` and `vineta-floor/03.png` (matched pose rendered with
`--camera-pose`, untracked). In the original the tunnel's hex windows are teal glass with the sea behind them and
the arch pillars carry smooth yellow-orange light blocks. Ours drew the hex frames over **black** panels and the
pillar blocks as an orange honeycomb with dark cells. Two causes, neither a texture-pick fault.

### 1. The hex glass is `out = fog(C + grab * W)` over an engine-bound screen grab (confidence 85)

`mt_tunnelrefraction` (slots 80 and 635) and `cl_tunnelrefraction` (slot 95) declare sampler `0x88a0df95`, which no
`.rcsmaterial` record names a texture for (`hd_refraction_census.rs`; the census over all 28 circuit models finds it
in **six slots, all Vineta K, both directions**). Disassembly (`ps3-microcode.py fp-file`, `mt` block `@0x1fb0` and
`@0x9de0`, `cl` block `@0x18b0`): a normal map at unit 0 (`mt_tunnelhex_n`), the hex picture at unit 1, and unit 2
sampled at `(clip.xy / w) * {1,-1} + {0.5, 0.5}` perturbed by the normal map - **a copy of the frame behind the
glass, not an environment probe the disc fails to ship**. The colour is

    C = diffuse * lit + specular
    mt: W = diffuse.a * 0x78575769          record value [0.2, 0.6, 0.6]  (the teal, authored)
    cl: W = 2 * 0x78575769                  record value [1, 1, 1]; C is scaled by DiffuseColour 0.2549
    out = f * (C + grab * W) + (1 - f) * fogColour

with a constant output alpha, so it is not a blended surface in the authored sense: it replaces the pixel with
something that includes it. `mt_tunnelhex_d` has alpha 0 on the frame struts and 255 in the panes, which is why the
frames stay metal and the panes show the sea. Drawn as an ordinary opaque lit surface, `C` alone is black.

**Fix** (`mesh::rcs::refraction`, `slots::REFRACTION`/`REFRACT_GRAB`, `mesh.wesl`'s `fs_main_blend`): no scene-copy
pass is needed, the equation is linear in the grab. A chunk is emitted twice into the transparent list, depth write
off like every transparent draw: pass 1 `dst = dst * src` with shader output `f * W`, pass 2 `dst = dst + src` with
output `C`. `MATERIAL_SHIFT` moves 20 -> 22 for the two new bits. **Chosen, not measured:** the normal map's offset of
the grab coordinate is not drawn (the grab is read at the pixel's own position); transparent draws never write depth,
so a blended draw submitted after the glass (the sea foam, slot 770) can land over it - it did not show at
`vineta-floor/03`. Pinned by `crates/render/tests/hd_vineta_ceiling_ground_truth.rs`
(`the_tunnel_glass_is_two_blended_passes_over_the_screen`, fails with the classifier removed).

### 2. `Program::accumulates` read a write to `H2.w` as taking `H2.xyz` away (confidence 90)

`2uv_offset_lights` (slot 14, the pillar light blocks; `under_strut.gtf` plus `under_strut_glow.gtf`) ends
`MAD H1.xyz, H1.wwww, H2, H1` with `H2 = unit1 * tint` - an accumulate - but `@0x11 DIVSQ_SAT H2.w` sits between the
sample and its use. `accumulates` tracked taint per register, so the lane-w write cleared it and the material got no
additive glow layer: the honeycomb diffuse drew alone. The taint is now per lane (`BTreeMap<(reg, half), lanes>`;
a write clears only the lanes it covers). Unit test `a_write_to_another_lane_does_not_clobber_the_sample`, ground
truth `the_arch_light_strips_carry_their_glow_layer` (fails with the old rule).
**Known approximation:** the glow is sampled at the diffuse coordinate, the program samples it at `f[TC0].zw`
(a second set); the strip reads as the reference's smooth block either way.

**Reach, measured, and wider than the lane:** `hd_add_second_census.rs` counts vertices carrying `ADD_SECOND` per
circuit model before/after: 718,576 -> 860,624 vertices, 147 -> 194 role words, **20 of 28 models move** (Amphiseum,
Modesto Heights 52k -> 62k, Talon's Junction 29k -> 61k, Tech De Ra, Vineta K, Anulpha Pass, Sebenco Climb, Ubermall,
Chenghou, Sol 2 and the main track among them). Checked against a reference where one exists: `talons-matched/03`
(the only pose of 00/01/03 that moves, 9,622 px of 1.7 M) gains **brighter cyan light bars on the upper-left wall,
matching the reference's cyan bars** (`data/scratch/hd-vineta-ceiling/tal_crop03.png`). No second-circuit reference
frame was taken beyond Talon's Junction.

**Still open on this frame (not this lane):** the sky behind the glass (blue/purple, a volcano drawn saturated red)
is `hd-sky-luma`'s; every before/after of the glass is confounded by it. The reference sea is teal.

**2048 / Omega check: not checkable** for the glass (PS3 microcode and an engine-bound sampler; 2048's Vita and Omega's
PS4 shader containers are unread). **Checked, applies, not wired** for `accumulates`: it is a PS3 microcode reader
and does not run on either.

## A picture the program reads one lane wide is a scalar, not the colour (2026-10-07, `hd-sebenco`)

**Reports** (maintainer play): Sebenco Climb's rails and surfaces violet (tick 720, world `[-337.1, 16.7, -14.9]`; tick
4380, `[-429.9, 21.7, 369.4]`), and Vineta K's tunnel glass showing a saturated red shape. **References:** RPCS3
`place` captures, `data/scratch/hd-sebenco/ref/` (forward, both poses), `ref2/` (forward sky poses), `ref3/`
(reversed); ours at the kept pose beside each (`pair_00.png`, `pair_01.png`, `rvpair_0.png`, `rvpair_1.png`,
`vk_rab.png`; untracked).

Three materials, one cause class: **a lightmapped or opaque material's picture was bound by file order, and the entry
first in the file is a map the program reads as a scalar or a normal.**

| Material (circuit) | What was bound | What the program reads | Now |
| --- | --- | --- | --- |
| `track_coloured_specular_alpha4glow` (Sebenco, the solar wall, slot 102) | `ds_solarwall_n` (normal map, sampler `0x48f37f5a`, entry 0) as colour: the violet wall | the picture is `ds_solarwall_c_withglow` at unit 0 (`0x3bdc0403`, entry 2) | `0x48f37f5a` joins `skin::NOT_A_PICTURE` (confidence 85: its 5 binds disc-wide are `ds_solarwall_n`, `tracknormal`, `aadc_build_g_window_n`, `jd_chenghou_windowtrans_01_n`) |
| `sebenco_ice` (Sebenco, the pool, slot 366 and 5 more) | `and_ice1` (purple, mean `(69,25,160)`) | `TEX H1.x ... unit0`: one lane, a specular-power scalar (`H0.w = H1.x * 296.89`). The colour is constants (`(0.0475,0.106,0.147)` deep, `(0.13,0.755,0.872)` cyan, `(0.66,0.76,1.0)` pale) lerped by the pond mask (unit 1, `.x`) and `and_snow3alpha`'s `.xyz` (unit 2), plus the paraboloid reflection (`0x9edd3243`, engine-bound) times parameters that are 0 on this material | `Program::samples_colour(unit)`; the first entry the program reads as a colour wins, then the first declared. Binds `and_snow3alpha` (white). The constant water/ice lerp is drawn since 2026-10-07: see "Water family" below |
| `and_rocktosand` (Vineta K backdrop terrain, slot 26) | `j_rockblend5`, a red-channel blend mask, as the terrain's colour: the red shape behind the glass | mask at unit 2 (one lane), sand `and_sand_sand` unit 0, rock `and_rock4` unit 1 (colour lane `Mixed`) | opaque materials: the same `declared_picture` rule in `skin::picks`' fallback, so it binds `and_rock4` |

**This corrects two earlier readings.** The 2026-10-07 `hd-vineta-floor` section's `sebenco_ice` change (`and_ice2` ->
`and_ice1`, "no reference frame taken") **was wrong**: `and_ice1` is the purple scalar map; the reference pool is white with
cyan water (`pair_01.png`, `rvpair_1.png`). It also named the reversed circuit only: the forward track has the same slots
(slot 366), which is where the maintainer saw it. The Vineta glass lane's "sky behind the glass is `hd-sky-luma`'s" is
**not the sky**: `OAG_SKIP_SKY`-style masking changes nothing behind the panes (`vk_skysky.png`); the red was terrain
(`OAG_TINT_MATERIALS` + `hd_slot_list` hue decode: slot 26), now grey-green rock (`vk_rab.png`, ref | before | after).

**Disc-wide reach, measured** (`hd_lightmap_colour_census.rs`, `hd_pick_dump.rs`, all 28 circuit models): the lightmapped
rule changes 63 slots in 16 (family, old -> new) pairs, nearly all a `*_alpha.gtf`/mask first entry replaced by the window or
land colour (`glass_2nduv_reflect_glow` on 02/03/04/Sol 2/Vineta K, `2rocksandblend_via_diffuse` on Vineta K,
`mt_diffuse_glow_specular_01` on Anulpha, `sebenco_ice`); the opaque fallback adds 68 more over 9 families (window glass,
`nr_twinblend`, `nr_facinglcdstrips`). Blended materials are deliberately excluded from the fallback (glass has its own
rules, not measured here). **Reference check:** Sol 2 pose 00 (the only pose of Sol 2 00/01 and Talon's 00/01 that moves,
12k px) shows the right-hand building's window rows gaining colour with no visible regression
(`data/scratch/hd-sebenco/cmp/sol2_00_rab.png`); Talon's 00/01 are bit-identical. The other 66 slots have no reference.
Pins: `crates/render/tests/hd_lightmapped_albedo_ground_truth.rs` (three new tests, each fails with its rule removed),
`a_unit_sampled_one_lane_wide_is_not_a_colour`.

**2048 / Omega:** not checkable (PS3 microcode and sampler hashes; their shader containers are unread). Confidence 90
for the cause of the wall, the pool's wrong picture and the red shape (each found by decode, fixed, and the frame
re-read); 60 for the 127 other slots (rule justified by the program text, not by a reference frame each).

## Water family: `water_test_2`, `water_noref` and `sebenco_ice` (2026-10-07, `hd-water`)

**Reports:** Vineta K's sea behind the tunnel glass is blue where the RPCS3 capture is teal, and Sebenco Climb's pool is
snow-white where it has cyan streaks. **References:** `data/reference/hd-capture/vineta-floor/03.png` (camera
`-861.568,-145.613,178.961`), `data/scratch/hd-sebenco/ref/01.png`; before/after frames in `data/scratch/hd-water/`
(untracked).

### Census (measured, confidence 95)

`crates/render/examples/hd_water_census.rs`, all 28 circuit models of DATA00-03 (zones 1-4 have one direction): **979
slots in 24 models resolve to a program that declares `paraboloidReflectionTex` (`0x9edd3243`)** - overwhelmingly the glass
and `d_s_n_r` families. The water members:

| Material | Slots (per direction) | Program reads |
| --- | --- | --- |
| `water_test_2` (Vineta K) | 1 (slot 5) | `waves2` as a normal map, vertex colour, ambient + sun, the probe |
| `water_noref` (Vineta K) | 3 (93, 387, 547) | `waves2` at 3 taps as a normal, its alpha, the probe, a sun glint |
| `sebenco_ice` (Sebenco Climb) | 3 (270, 366, 374; reversed 258, 349, 357) | three constants lerped, see below |
| `reflectplane_dc_seawater[edging]`, `water` (Amphiseum), `cf_waterfall`, `and_waterfall` | other circuits | **not this family**: no probe, or a different program; not touched |

### The two programs of Vineta K (confidence 90, read off `scripts/ps3-microcode.py fp-file`)

`waves2.gtf` is **not a picture**: `water_test_2` block `@0x1d60` and `water_noref` block `@0x3870` fetch it three times,
each decoded `MAD r, tap, {2,-1}.x, {2,-1}.y` (`2 tap - 1`) and summed into a normal. Binding it as the albedo drew a blue
sheet of normal-map colour. The vertex program (`vp-file`, block 3) writes `TC1 = vertex colour`, `TC2 = eye - position`
(world), `TC4/TC0/TC6.yzw` = tangent, bitangent, normal, and scrolls the wave coordinates off `time`. The colours:

```text
water_test_2:  out = fog( TC1 * (ambient + sun * sat(N.L)) + TC1 * R * 0x7480de6d-tint )
water_noref:   out = fog( R * 0x7480de6d-tint * (0.2 w + 0.4) + sun * glint * (0.2 sat(w * 0x697f57e3) + 0.4) )
R              = tex(paraboloidReflectionTex, u, v)      u = 0.25 + (d.x > 0 ? -d.z : d.z + 0.5),  v = 0.5 - 0.5 d.y
d              = n (n.w) - w                             w = unit vector to the eye, n the perturbed normal
```

`water_noref` declares no `constantAmbientColour` and reads no vertex colour: with the reflection taken out it is **black plus
a glint plus fog**. Reading `water_test_2` the same way gives lit vertex colour with no picture. Both are now drawn that
way (`mesh::rcs::water`, report "water material(s) whose picture is a normal map"); **the reflection is not drawn**.

### What the reflection is: not established (confidence 40 that it is a sky panorama; do not wire)

**Superseded 2026-10-07:** measured as a runtime dual-paraboloid render target bound at unit 1; see "Vineta K against a draw capture", section 2, at the end of this page.

- The `.rcsmodel` records of `water_noref` carry `skyreflect.gtf` (512x512, a sky panorama mirrored about its middle row)
  under sampler `0x8365b1f3`, and `sebenco_ice` carries `and_ice2`, `d_s_n_customr` `ds_dualparaboloid_c`, Amphiseum's
  `water` `dc_waterreflection`. **`0x8365b1f3` is `~crc32("PerMaterialEnvMap")`, an exact preimage (confidence 95)**, and no
  HD program declares it.
- 2048's `Environment_Load` (Ghidra `/2048/eboot-vita-2048-eu-v104.elf`, `0x8102fe42`) hashes `"PerMaterialEnvMap"` and
  scans every material for it, and loads `skyParaboloid.gxt` beside `skycube.rcsmodel` as the environment's probe
  (`DAT_816a3ee4`); 2048 ships 14 `skyParaboloid.gxt` in its base archive. **HD's executable contains neither string**
  (`grep -a -c PerMaterialEnvMap EBOOT.elf` = 0; `paraboloidReflectionTex` is there once, as the engine parameter name),
  and its engine parameter table slot 8 (`paraboloidReflectionTex`) is filled by something this lane could not find. So
  the lineage says the probe is a sky paraboloid, and HD's data names a per-material map the HD engine does not
  visibly read. A frame with the reflection drawn from `skyreflect.gtf` by the program's own coordinate was built and
  **reverted**: on a flat sea `d.y` is 0, so `v` is 0.5 and the lookup lands on the dark horizon strip.
- `0x7480de6d`, the engine tint that scales `R`, has no preimage and no authored value.

### Vineta K: measured; the windows are not the water, the sea beside them got paler (confidence 75)

Window pixel means at the capture's pose (x scaled by 1882/1600), reference / before / after / water skipped
(`OAG_SKIP_MATERIAL=water_noref,water_test_2`, after the change):

| Window | Reference | Before | After | After, water skipped |
| --- | --- | --- | --- | --- |
| right pane `(1250,330,1330,420)` | `(33,147,147)` | `(47,117,159)` | `(52,116,153)` | `(50,117,156)` |
| middle pane `(900,400,1000,470)` | `(16,145,146)` | `(43,94,147)` | `(48,96,141)` | `(46,97,143)` |
| sea band, left `(600,400,700,450)` | `(15,145,145)` | `(127,178,220)` | `(176,217,219)` | `(132,196,160)` |

- **The two panes do not show the water**: skipping both programs moves them by 1 to 4 levels, and the change moves them
  by 2 to 6. Their residual (green about 30 low, red 15 to 20 high) is the backdrop behind the glass; a sky-only frame has
  mean `(234,228,182)` there, so the sky tint (`hd-sky-luma`'s open item) and the glass multiply decide it.
- **The sea band beside them did move, and away from the reference**: it was blue (the normal map painted as a picture)
  and is now pale `(176,217,219)` against the reference's teal. A program with no picture is lit vertex colour, which is
  pale here; the reference's teal is therefore carried by the term this lane leaves out, the reflection times the engine
  tint `0x7480de6d`. Drawing the program's colour minus its one unknown term is the honest reading and is also further
  from the capture in this window. If the maintainer prefers the old blue, dropping the `water::water` call in
  `rcs/setup.rs` restores it, and `OAG_WATER_OFF=1` shows it.
- `OAG_ONLY_SLOT` isolation: `water_test_2` draws the sea plane (teal when seen alone, `iso_test2.png`) and `water_noref`
  a horizon band.

### Sebenco ice: closes entirely from data (confidence 85 for the program, 60 for the picture)

`sebenco_ice` blocks `@0x111b0` (lightmapped, the pond) and `@0x4480` (no lightmap), `fp-file`:

```text
mask  = tex(unit 1 = 0xf1d875a1, TC4.zw).x      the pond transition map, on the SECOND uv set (`Uv2`, byte 22)
snow  = tex(unit 2 = 0x2e7d71db, TC4.xy).rgb    and_snow3alpha
F     = V . N'                                  N' = vertex normal bumped by two normal-map taps
water = deep + F (cyan - deep)                  0x1f3b345d, 0xef8869cd
ice   = pale + F (snow - pale)                  0x4042b6e6
A     = water + mask (ice - water)
out   = fog( A * L + spec ) ,   L = the baked lightmap lighting; reflection weight 0xe7a83aef, 0xa1b54b80 = 0
```

The reflection weights are authored **0** on every `sebenco_ice` slot, so the probe drops out and nothing is missing.
Authored values: deep `(0.0475,0.106,0.147)`, cyan `(0.131,0.755,0.872)` on the pond slot and `(0.392,0.596,0.634)` on
the cave-entrance slots, pale `(0.663,0.763,1.0)`. The pond mask is dark in the pond and white around it
(`and_snow_ice_transition_pond.gtf`, 256x256). **`Uv2` is `TC4.zw`**: `Mesh::second_texcoords` reads it (declared `Uv1` at
byte 18, `Uv2` at 22 on all three chunk kinds), `GpuVertex::texcoord2` carries it (a 13th attribute, last), and the
fragment varying widens `lightmap_texcoord` to a `vec4` rather than adding a location. `mesh::rcs::ice` classifies by
program facts (declares the probe, the mask and snow samplers, patches all three colours, mask read one lane wide), puts
the three colours in three consecutive glow-table entries, binds snow first and the mask third, and sets
`slots::ICE` (bit 22; `MATERIAL_SHIFT` moved 22 to 23).

**Chosen, not measured:** `N'` is the vertex normal, so `F` is smooth where the original's streaks; the three colours are
display values decoded with `pow(2.2)` to the domain the textures are sampled in; the program's own specular is left to
the generic one. **Result** (`data/scratch/hd-water/sb_obl_before.png` / `sb_obl_ice2.png`, an oblique camera at the pond;
`OAG_WATER_OFF=1` restores the before): the pool goes from a snow-white sheet to an authored pond - deep-blue blotches in
the middle (the mask's dark region), pale icy blue around, white beyond. The reference (`ref/01.png`) is paler
and brighter - pool pixels `(155,244,249)` with 63 % cyan and 39 % white, against ours `(19,95,130)` in the pond - and
the difference is not closed: the original's bump-perturbed `F` and its HDR sun and bloom are not reproduced. No matched
camera exists (the reference's chase camera is not recoverable from the capture), so this is a look comparison, not a
measurement.

### 2048 / Omega

- **2048 (Vita): checked, differs.** Its Vineta K ships `dc_seawater` and `reflectplane_*`, not `water_noref` or
  `water_test_2`; it has no Sebenco ice material. It does ship `waves2.gxt` and 14 `skyParaboloid.gxt`, which is the
  evidence for what the probe is there.
- **Omega (PS4): checked, applies, not wired.** The PS4 data ships the same materials: `Water_noref`, `WATER_Test_2`
  (`data01`, `data05`, `data08`, each with `_1`/`_2` shader splits) and `Sebenco_ice` (`data02`, `data05`, `data08`), and
  `Water_lapping_waves2_N.gnf`. Omega's circuits read through the 2048 `.rcsmodel` path whose shaders are PS4 GNM, not RSX
  microcode, so the classifiers here (microcode facts) do not run on it. The three ice colours are model parameters in
  HD's record and probably in Omega's, so wiring `Sebenco_ice` there is a data read plus the same shader branch; left open.
- **Not checkable:** whether Omega's `paraboloidReflectionTex` string exists (its string search found none, which
  is not a proof).

## Vineta K against a draw capture: the start line's light bars, the sea's real inputs, and the alternate fog (2026-10-07, `vineta-k-fidelity`)

**Reports (the maintainer's):** the sea is cyan where the original's is teal; meshes show on the tunnel ceiling that the
original does not show; purple textures at the start line. **Method:** one RPCS3 frame at the maintainer's pose
(`place --pose=-839.8,-146.6,215.0`, kept `-840.6,-146.7,214.2`, three boots agree to 0.1) and one at the start slot
(`178.0,36.9,-116.75`), each with the RSX command stream, every fragment program, the index and vertex ranges and the
first bytes of every bound texture read out of guest memory while the emulator was paused
([rpcs3-capture.md](../reverse-engineering/rpcs3-capture.md), "Capturing one frame's draws"). Frames, dumps and analysis:
`data/scratch/vineta-k-fidelity/` (untracked; the report there lists each file). 285 draws at pose A (ours: 463).

### 1. Purple start line = the emissive term was never bound (fixed; confidence 85)

`diffuse_normal_specular_emmissive` (sic: `ds_sf`, `ds_sfline_trench`, `ds_pit`, `ds_wall`, `ds_rail`; 13 slots on Vineta K)
has the pad programs' shape in its lit variant (block `#5`): unit 1 is the `_ne` file, its RGB a tangent normal, and
`@0x26 MAD H4.xyz, R1.wwww, {C}, H0` adds `_ne.a * C` after the light, `C` patched by `0x7611a2d8`
(`Program::alpha_gated_parameter` already reads exactly this). The model authors `C` per slot: `(0, 0.25, 1)` on
`ds_pit`/`ds_wall`, `(0, 0.271, 0.656)` on `ds_sf`/`ds_rail`, `(0.975, 0.975, 0.975)` on the other wall slots. `bind_scene_pad_masks`
bound the `_ne` mask only for the speed pads' unreferenced chunks (its doc says this family "is a different program"), so
these surfaces drew as the dark diffuse alone: navy with a violet cast. **Fix:** `pads::is_light_bar_material` adds the
family to that routing; `pad_ne` still accepts a slot only after matching the program.

| Start slot, 1882x1058 | bright-blue pixels (b>150, r<90) | their mean |
| --- | --- | --- |
| RPCS3 (`out/boot7/00.png`) | 40,440 | `(28,120,214)` |
| before | 1,529 | `(84,95,174)` (violet: red 84) |
| after | 49,896 | `(24,95,174)` |

The camera of the two is not identical (ours sits closer), so the count is an extent, not a pixel match; by eye
(`pairB_fix1.png`: reference, before, after) the panels, the cyan wall lamps and the blue chevrons match. After is about
20 % dimmer than the reference (G 95 against 120): the bloom/exposure stage, not measured. **Reach:** Sol 2's start strip
changes from blue-violet dashes to the reference's red line (`talons`/`sol2` matched frames,
`reach_sol2-matched_00.png`); Talon's Junction changes 662 px; Amphiseum's and every other matched reference frame
are bit-identical (`reach/`). Pinned by `hd_light_bar_ground_truth`.

**Reach census** (`hd_light_bar_census`, every `track.vex` of the disc, `light_bar_census.tsv`): the family has 3 to 26
slots on each of the 24 circuit models of the 12 circuits and none in the four Zone models, and `pad_ne` binds every one of
them (unread 0): Vineta K 13/14, `02_track` 26/24, `03_track` 4, Chenghou 4, Ubermall 4, Sebenco Climb 11/8, Sol 2 5/7, Anulpha Pass 6,
Talon's, Amphiseum, Modesto and Tech De Ra 3 each (beside their pads). **Judged against a reference: Vineta K (start slot and
the tunnel pose) and Sol 2 (matched frame 00); not judged: the direction of Talon's 662 px (matched 00), and every other circuit
with no reference frame.** It is a global change on a program-match, as the pads' was.

### 2. The sea: what the original's `water_test_2` draw reads (measured, confidence 90 unless noted)

Draw 66 of the pose-A frame is `water_test_2`: **one opaque draw, 186 indices**, blend off, depth `LEQUAL` with write on, back-face
cull with CCW front. The sheet is at world `y = -50.8` and the eye at `y = -143.8`: **the sea is 93 units above the tunnel
and is seen from below.** Its vertex colour (attribute 2) is `(0, 0.498, 0.486)`, the teal, on all 47 vertices. The program
in RAM (`ps3-fp-live.py`) is the one this page decoded from the disc:
`out = fog(TC1 * (0.4 + sun * sat(N.L)) + TC1 * R)`, `sun = (8, 4.52, 2.16)`.

- **The vertex colour multiplies the light.** The generic reading adds it to the light and multiplies a white picture, so the
  sheet drew pale and blown out (a white sheet over the sky; whether it is what the earlier lane's `(176,217,219)` band measured was not diffed). Drawn now as the program states it:
  `slots::ICE` with `WATER_FLAG` (glow-entry rate 2.0) selects, in `shade.wesl`, picture = vertex colour and
  light = ambient + sun diffuse, with no vertex-light or prelit term. Pinned by `hd_light_bar_ground_truth`.
- **`paraboloidReflectionTex` is bound, at unit 1, and is a runtime render target.** `0xc4065380` in VRAM, 512x256 linear
  `A8R8G8B8`, clamp (`0x60730303`). With RPCS3's `Write Color Buffers` on its contents read back
  (`data/scratch/vineta-k-fidelity/probe11.png`): **a dual paraboloid of the environment - two discs, the sky with its clouds,
  mountain horizon and sun glow above the middle row, a teal sea gradient below it.** The row split at `v = 0.5` is the
  program's own coordinate law (`v = 0.5 - 0.5 d.y`). Seen from below, the sheet reads the **lower** half: mean
  `(0.15, 0.36, 0.41)` (upper half `(0.43, 0.59, 0.57)`, whole texture `(0.31, 0.49, 0.51)`). Without `Write Color Buffers` the same address reads as stale noise, which is what an earlier
  reading of this capture saw. It is not a disc texture (no `.gtf` of 512x256 `A8R8G8B8` matches any of its samples) and
  HD's executable never names a source for it.
- **What this settles of the 2026-10-07 `hd-water` reading:** `skyreflect.gtf`/`PerMaterialEnvMap` is not what the program reads
  (confidence 85); the probe is an environment render, so "a sky panorama" (confidence 40 there) is half right: its upper half is
  the sky, its lower half is not any disc data this project has located. **Not drawn, on the repo's own rule**: the
  lower half is the part this view needs, and nothing authored supplies it. `TC1 * R` is therefore missing from the sheet
  (about `(0, 0.18, 0.20)` added to a lit term of `(0, 0.2, 0.2)` before the sun's share).
- **The panes' teal is probably the glass over a bright background, not the sea sheet (confidence 60, inferred).**
  `mt_tunnelrefraction`'s `W = (0.2, 0.6, 0.6)` times a near-white grab is `(0.2, 0.6, 0.6)`, the reference's `(33,147,147)`;
  the white swirls read as clouds. `W` times the probe's own upper-half sky `(0.43, 0.59, 0.57)` would give about `(22, 90, 87)`,
  so the grab is brighter than that sky. The sheet itself covers
  only the upper-left of the frame there (draw 66's projected triangles, `overlay66.png`). Ours grabs a light-blue sky
  (`(52,116,153)`): that is the sky-tint item in `hd-sky-luma`'s thread, not the water.

### 3. The ceiling meshes: the original fogs the scenery beyond the glass differently, and does not draw some of the meshes ours shows

Two separate measured facts, not one cause.

**(a) The original uses the alternate fog pair for one group of draws (confidence 90 for the values).** The fragment
programs' patched fog constants are `{0, 0.031373, 0.031373, 0.0045}` on draws 39-74 of the pose-A frame (scenery beyond the
tunnel glass: `j_arch_support`, `and_metal_struts`, `j_arch_lights`, `and_rock4`, `and_sand_sand`, `and_tower_3pyt2`,
`mar_glowstrips`, the sea sheet) against `{0.039216, 0.086275, 0.070588, 0.00025}` on the track draws (`ps3-fp-live.py`).
**Both are the circuit's own `.envsettings`**: `Fog.Alternate Fog Color` / `Alternate Fog Density` against `Fog Color` /
`Fog Density`. This repo reads the primary pair for every draw (`environment.rs`: "what selects [the alternates] is unread").
`exp(-(0.0045 d)^2)` is 0.9 at 70 units, 0.56 at 170, 0.16 at 300 and 0.0006 at 600: **it darkens the near struts and takes the
far scenery to dark teal; it does not remove what is within 170 units.** The group is **not** spatial (members 23 to 930
units from the camera, `y` -158 to +9), **not** a material family (`ds_concrete_band_cs` is in it, `ds_wall_cs` is not) and
**The selector is the chunk's own render flag `0x20` (confidence 88; 2026-10-07, `hd-glass-opus`).** Both fog
publishers, B4 (`FUN_00400a00`, `0x00400c38`) and B5 (`FUN_003ff860`, `0x003ffa90`), pick the draw's `fogColour`
buffer from the record's flag halfword (`block->0x06`, `Mesh::render_flags`): bit `0x20` set reads
`Scene_GetAlternateFog` (`0x00c49120`, which `Scene_PrepareFrame` fills from `+0x4f0`/`+0x504` at `0x003acb88`),
clear reads `Scene_GetPrimaryFog` (`0x00c49110`); a track chunk (bit 0) in Zone reads `0x00c49130` instead. Checked
draw by draw on the pose-A capture, each draw tied to its chunk by vertex offset: **all 35 alternate-fog draws are
`0x20` chunks and all 76 primary-fog draws are not.** An earlier reading matched by texture, which mixes chunks
of both kinds, and could not see it. Not wired. See
[visibility.md](../ghidra/functions/ps3-hdfury-eu/visibility.md), 2026-10-07.

**(b) The meshes ours draws on the tunnel ceiling are culled by the original's frustum, which is narrower than
its picture (confidence 85; 2026-10-07, `hd-glass-opus`; supersedes the "unexplained" reading).** Tied draw by draw
to chunks, the original submits **51 track chunks** at pose A and this project 255. The kind-1 chunks it skips
(16-18, 55, 1315 and 1317-1321: `and_metalstruts_pt2`, `and_metal_struts`, `and_metalshine2`, `and_bubbles`,
`mar_col_rim3`, `and_girder3`, one column of struts and girders at `(-695..-728, y -112..-152, z 282..314)`)
pass the PVS and lie inside the drawn picture. The engine's cull planes are built from the authored
`<ExternalCameraFar fov="60">` (live: 30 degrees vertical, 45.7 horizontal half-angles), and the picture is drawn
4/3 wider in tangent (75.2 degrees vertical). The column sits in the ring between the two, at the left edge, so the
original never submits it. Rendering ours from the original's exact eye at 75.2 degrees shows the column through
the upper-left glass where the original shows teal (`data/scratch/hd-glass-opus/cmp_camA.png`). **They are not
light helpers**: ordinary main-list materials (`lambertzeroalpha`, `jd_simplespecular`, `lambert_simple`). Chunks
1305 and 1286, named here before by texture, are node-placed and sit at the origin in both games (the original's
live world sphere and ours agree); they were never the ones on screen. Ours also draws all 197 node-placed chunks
the PVS allows with no frustum test, against the original's 5 or 6. The law, the live read and the score are on
[visibility.md](../ghidra/functions/ps3-hdfury-eu/visibility.md), 2026-10-07.

### 4. Lineage

Omega ships `Water_noref` and `WATER_Test_2` (GNM shaders; the microcode classifiers do not run). **The light-bar family:** `diffuse_normal_specular_emmissive` is in Omega's `data00`, `data01` and `data02` archives
(`rg -a` over the extraction); checked, applies, not wired (PS4 GNM programs, so the microcode check `pad_ne` makes does not
run); 2048 differs (its own material names). **The sea:** checked, applies, not wired; Omega's probe would also be a runtime target
(2048's per-environment `skyParaboloid.gxt` is the authored half of the same idea). **The alternate fog:** `Alternate Fog` keys
exist in every HD-lineage `.envsettings`; Omega not checkable (no PS4 emulator).

## See also

- [rcsmodel](rcsmodel.md) - the material record, and the two texture paths
- [renderer.md](../ghidra/functions/ps3-hdfury-eu/renderer.md) - the `SHO` block
  framed, and the executable's own 124 shader programs
- [gtf](gtf.md) - the textures both slots name
