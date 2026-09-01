# HD/Fury's Zone shader, read out of the RSX fragment microcode

**The Zone effect is not an engine program.** No Zone name appears in the
EBOOT's own 121-name shader census. Zone is a set of compiled **variants
inside the materials themselves**, which is why every search of the executable
for a "Zone shader" came back empty: there is nothing there to find. The
measured scale is **20,214 Zone-bearing fragment blocks**, asserted against the
image by `crates/formats/tests/zone_shader_census_ground_truth.rs`.

**A file count that stood here has been removed rather than corrected**, and
the reason is worth more than the number was. It read "1,467 of the disc's
1,590 `.rcsmaterial` files". The disc has **1,632**
(`rcsmaterial_ground_truth.rs`); `DATA00` alone has **693**. `1,590` is neither
- it is the size of a `/tmp` extraction directory that a second process had
extracted other archives into, and it reached this page three separate times:
in that sentence, in a census whose absolute numbers all had to be restated,
and in a "two independent parties agree" claim that turned out to be one corpus
counted twice. **Nobody has measured how many files carry a Zone variant**, so
this page no longer says. See
[methodology.md](../../../reverse-engineering/methodology.md#rules-learned-the-expensive-way).

Read in full from three materials that agree with each other:

```sh
# the three materials read here. NOTE the archive split is not uniform:
# cf_constantcolourglow is in DATA02, the other two in DATA00, and the
# textures are in DATA02 - guessing one archive for "materials" is the same
# mistake that contaminated a census below.
data/environments/zone_1/materials/billboarddiffuse.rcsmaterial
data/environments/01_vineta_k/materials/cf_constantcolourglow.rcsmaterial
data/environments/zone_2/materials/gradientcolour1.rcsmaterial
```

This page is the combination rule. What *feeds* it is on
[zone-effectsettings-loader.md](zone-effectsettings-loader.md) (the parameter
table, the per-stage texture arrays, the stage blend); what the palette table
authors is in [effectsettings.md](../../../formats/effectsettings.md).

## The rule - but read the next section first: this is the **arena** form, not the common one

**Corrected 2026-08-31.** The block below was read from a `zone_1` material and
was published here as *the* Zone rule. A census says it is the rare one: the
`albedo + zoneCol * (1 - blackMask)` shape occurs in **130 of 20,214**
Zone-bearing fragment blocks (0.6%), and every one of them is in
`zone_1`..`zone_4` - HD's four Zone **arenas**. All twelve racing circuits
carry a different surface line with **no albedo term at all**:

```text
surface = zoneTex<I|O>(zoneUV).rgb * E.rgb
          + zoneBase<I|O>.rgb * rim^10
          + zoneBaseAlt<I|O>.rgb * rim^5
```

Checked here independently, and the check controls for material type by using
the *same material name* on both sides: `billboarddiffuse.rcsmaterial` in
`zone_1` contains the distinctive `100000.0` `blackMask` literal **four
times**; the same file in `05_ubermall` contains it **zero** times.

That difference is what a frame comparison had already shown without
explaining: raced on Moa Therma, the original's crowds, banners, advertising
and concrete all vanish into the zone terms, which a rule that only adds where
the albedo is already black could never do. On a racing circuit the albedo is
not in the equation.

Everything else below - the UV transform, the `zoneTexVis` lookup, the sphere
test, the rim exponents - is unaffected and was read in both forms.

`E` is `zoneEffectInner` or `zoneEffectOuter`, chosen by the sphere test below.

```text
zoneUV    = zoneColourTint.xy * (1 - meshUV)
rim       = 1 - dot(N, -V)
band      = zoneTex<I|O>Nearest(zoneUV).a
zoneCol   = zoneTex<I|O>(zoneUV).rgb * E.rgb
            + 2 * zoneAnisoPalette<|Outer>[ pow(rim, zoneAnisoPower.<x|y>) ]
albedo    = Texture1(meshUV).rgb
blackMask = saturate((albedo.r + albedo.g + albedo.b) * 100000)   ; 1 unless black
surface   = albedo + zoneCol * (1 - blackMask)
glow      = max( saturate(N.y - 0.5) * (1 - windowDepth) * E.w
                 * zoneTexVis[band].rgb , 0 )
            + 5.0 * saturate(1 - 0.1 * (distance - zoneColourTint.w))
light     = vpConstant + constantAmbientColour
            + saturate(dot(N, dirLight0Dir)) * dirLight0Colour
colour    = light * surface + glow
fog       = exp( -(fogColour.w * viewDepth)^2 )
out.rgb   = lerp(fogColour.rgb, colour, fog)
```

Confidence **82** on the structure, read twice from independent materials.

Three parts of it are worth naming separately, because each answers a question
this project had open and each is counter-intuitive from the names alone.

### The zone texture is sampled with the mesh's own UV, not screen space

`zoneUV = zoneColourTint.xy * (1 - meshUV)`. Not projected through
`zoneOrigin`, not screen-space. Confidence 82. **And `zoneColourTint` is not a
colour** in this shader: `.xy` is the zone-UV scale and `.w` is the sphere
radius below; `.z` is never read. Confidence 78, cross-checked by the ordinary
lighting and `fogColour` parameters in the same program landing exactly where
their own names demand - so the reading is not a mis-decode that happens to
land on the Zone parameters.

### `zoneTexVis` is a 256-entry lookup keyed on the zone texture's **alpha**

`zoneTexVis[ zoneTex<I|O>Nearest(zoneUV).a ]` - which is what the
point-filtered `...Nearest` clone exists for, and why it is not cosmetic: an
interpolated alpha between two regions indexes a wrong palette entry.

**The shipped art confirms the alpha is a discrete band index, measured rather
than inferred.** Both texture sets are 256x256 DXT5 (GTF format `0x88`,
`COMPRESSED_DXT45`, 9 mips, 87,552 B) - and DXT5 is precisely the format chosen
when alpha must survive independently of RGB. Histogramming the decoded alpha
of the top mip, 65,536 texels:

| file | distinct alpha values | the plateaus |
| --- | ---: | --- |
| `zonemode3` (placeholder set) | **1** | 255 everywhere |
| `zonemodetrack8` | 10 | 26, 35, 46, 55, 66, 75, 86, 90, 169, 255 |
| **`zonemodetrack9`** | 11 | **31, 32, 33, 34, 35, 36, 37, 38, 39, 40**, 255 |
| **`zonemodetrack10`** | 10 | **31, 32, 33, 34, 35, 36, 37, 38, 39, 40** |
| `zonemodetrack14` | 66 | groups of 4 on a stride of 10 |
| `zonemodetrack6`/`7` | 162 | 1, 2, 3, ..., 162 |

**Ten consecutive integers, each covering a comparable share of the surface, is
a band index and cannot be a gradient** - a smooth ramp gives a broad
continuous distribution, not discrete plateaus at consecutive integers.
`track14`'s 4-wide groups on a stride of 10 is a *grid* of tagged regions.

**2026-09-01: every row of this table now has a meaning, and it is the same
one.** The per-frame writer is read
([zone-visualiser.md](zone-visualiser.md)): `zoneTexVis` holds **sixteen
ten-segment bar meters**, band `b` at texels `1 + 10b`..`10 + 10b`, with a
smooth slot each at `161 + b`. So `track9`/`10`'s `{31..40}` is band 3's bar,
`track14`'s "groups of 4 on a stride of 10" is four segments of every band -
the stride *is* the band stride - `track6`/`7`'s `{1..162}` is every bar plus
the first two smooth slots, and `track8`'s `169` is band 8's smooth slot
(`161 + 8`). Four histograms, measured here before that layout was read, all
four landing on it. This is the strongest corroboration either page carries.

The arithmetic closes: alpha `i` becomes texture coordinate `i/255`, which on a
**256**-wide LUT lands on texel `round(256*i/255) == i` for every `i`. 256
alpha values, 256 entries, index-preserving - a coincidence in a colour-ramp
design, and not one in a lookup-table design. Confidence **84** that the alpha
is a discrete region index; **80** that this is the audio-spectrum mechanism
the maintainer observes in play (see
[effectsettings.md](../../../formats/effectsettings.md)).

It also re-confirms the placeholder finding from the *file* side:
`zonemode3.gtf`'s alpha is 255 everywhere, so the general set indexes one
single LUT entry and can display nothing at all. **A port that follows
`Scene_PrepareFrame` alone draws a flat colour where the original draws an
equaliser.**

### Inner versus outer is a world-space sphere

`distance(worldPos, zoneOrigin) < zoneColourTint.w`. Confidence 82, and read
twice with *opposite* compiler polarity in the two materials - the same rule
both times, which is the check that separates a real read from a sign slip.

`zoneAnisoPower` is a **float2**: `.x` and `.y` are the inner and outer
exponents of `pow(1 - dot(N,-V), p)`, which indexes `zoneAnisoPalette` and
`zoneAnisoPaletteOuter` respectively. Confidence 82.

**A second, independent confirmation that `zoneColourTint.w` is the radius -
from the materials rather than the executable.** In
`01_vineta_k/materials/cf_constantcolourglow.rcsmaterial` (DATA02), twelve of
twenty fragment blocks are Zone-bearing, and they split exactly:

- six declare **Inner only** (`zoneBaseInner`, `zoneBaseAltInner`) and **no**
  `zoneColourTint`;
- six declare **Inner and Outer** and **do** declare `zoneColourTint`.

12 of 12, no exceptions. **These blocks sample nothing at all**, so
`zoneColourTint.xy` - a texture-coordinate scale - is of no possible use to
them. The only lane they can want is the one that selects between an Inner and
an Outer of every other parameter, which is `.w`. The rule "a block declares
`zoneColourTint` exactly when it needs the radius" then also holds from the
other side: `billboarddiffuse`'s Inner-only blocks *do* declare it, because
they sample and therefore need `.xy`.

Confidence 82 - the correlation is exact but rests on one material. It is
worth more than its score suggests, because it is derived from the *declaration
tables* and so shares no step with the `vsel` lane-3 result in
[zone-effectsettings-loader.md](zone-effectsettings-loader.md)'s twenty-fourth
pass, which reached the same conclusion from the per-frame blend in the
executable.

## Two findings that change what the artists were doing

**On the four Zone arenas - and only there - the recolour applies where the
material's own albedo is black.** `saturate((r+g+b) * 100000)` is a hand-rolled
`step(0, x)`, and `surface = albedo + zoneCol * (1 - blackMask)`. So on those
circuits the artists chose which parts of each surface light up **by painting
them pure black in the ordinary diffuse texture** - authored in the albedo,
expressed nowhere in the palette table or the parameter list.

**This was published as universal and is not**: 130 blocks of 20,214, all in
`zone_1`..`zone_4`. The twelve racing circuits drop the albedo term entirely -
see the retraction at the top of this page. Confidence 78 on the mechanism
where it occurs; the scope was the error, not the reading.

**The visualiser glow is gated to up-facing surfaces**: `saturate(N.y - 0.5)`.
That is the microcode independently producing the maintainer's own play
observation - *the floor* displays the equaliser - from a direction this
session did not go looking for it.

### The gate is universal, and the threshold takes exactly two values

Read from two materials above, and **counted over all of them** on
2026-09-01, by the same per-block census the retraction below rests on.
Every fragment block on the disc that *fetches* `zoneTexVis` - 18,050 of them
- carries exactly one negative literal on a saturating `ADD`:

| threshold | blocks |
| --- | ---: |
| `saturate(N.y - 0.5)` | **16,834** |
| `saturate(N.y - 1.0)` | 1,216 |
| anything else | **0** |

Confidence **88** on the count, which is mechanical
(`crates/formats/tests/zone_shader_census_ground_truth.rs`'s
`the_visualiser_glow_is_gated_to_up_facing_surfaces_everywhere`, re-derived
off `hdfury-ps3-eu-dec.iso` on every `just test-data`).

**`1.0` reads as the same gate authored off** - `saturate(N.y - 1)` is zero
for every unit normal, so those blocks multiply the lookup by zero -
confidence **70**, not 88: it depends on the register feeding the `ADD` being
a normalised `N.y`, which arrives through fragment opcode `0x3b`, an opcode
this project's decoder does not name. It is read as a normalise from its
pairing with a `DP3` of a vector against itself, twice in the same block
(`01_vineta_k/materials/diffuse_normal_specular.rcsmaterial` block 27).

**What this settles**: there is no second, billboard-specific glow shape.
`billboarddiffuse` (64 blocks), `cf_billboard1` (56), `cf_vex_billboard`
(56), `nr_crowd_bustle` (212), `ns_adbanner` (16), `scanlinebillboard` (28)
and every other `*billboard*`, `*crowd*`, `*banner*`, `*screen*` and
`*scanline*` material on the disc compiles the same up-gate as the track
surface does. The maintainer's play observation names the floors **and** the
billboards, and the hypothesis that the billboard half must be a second
fragment block with a different gate is refuted here.

### So whether a billboard lights up is a *geometry* fact, and the meshes say "partly"

`N` is the authored vertex normal in the mesh's own space, passed through
untouched - `billboarddiffuse`'s vertex program writes `MOV o[TC1].xyz,
v[1].xyzx`, and the fragment program normalises that same register and dots
it against the directional light's direction, so `N.y` is world up rather
than a view- or tangent-space quantity. Histogramming the shipped
`.rcsmodel` normals per material (`rcsmodel_vertex_ground_truth.rs`'s
`billboard_geometry_is_mixed_where_crowd_and_banner_geometry_is_not`):

| material | vertices | `N.y > 0.5` |
| --- | ---: | ---: |
| `track_surface` | 227,528 | 80.2% |
| `track_coloured_specular` | 63,664 | 82.4% |
| `wes_billboardholographicscanlines` | 3,648 | 32.7% |
| `billboarddiffuse` | 1,168 | 27.2% |
| `cf_billboard1` | 27,576 | 20.9% |
| `nr_crowd_bustle` | 831,952 | **0.0%** |
| `cf_cheap_crowd` | 239,664 | **0.0%** |
| `ns_adbanner` | 352 | **0.0%** |

So the recovered rule does light a fifth to a third of a billboard mesh - the
up-facing parts of it - and cannot light the crowds or the flat banners at
all. Confidence **85**; the test asserts the *ordering* rather than the
percentages, because a change to the normal unpacking would move the latter
and not the former.

## `zoneBase*` is **not** a separate family - retracted, and the count says so

**The first version of this section said `zoneBase<I|O>` and
`zoneBaseAlt<I|O>` "never appear in the same block as `zoneTex*`" and belonged
to an untextured family. That is wrong, and it is wrong in the way this page
keeps warning about: it generalised from three materials read by hand.**

A per-block census over all 1,632 `.rcsmaterial` files on the disc, counting
the parameter and sampler hashes each fragment block *declares*
(`~crc32(name)`, validated first against four known names in a block already
read by hand - `fogColour`, `constantAmbientColour` and the two
`directionalLight0*`):

| Zone-bearing fragment blocks | **20,214** |
| --- | ---: |
| `zoneBase*` **and** a sampled `zoneTex*` | **18,032** |
| `zoneBase*` with no zone texture | 2,052 |
| the `zoneAniso` shape, with a zone texture | 90 |
| the `zoneAniso` shape, without one | 40 |

So `zoneBase*` and `zoneTex*` co-occur in **89%** of Zone blocks, and the
"never in the same block" claim is refuted 18,032 times over. The three
materials the original reading was drawn from - `cf_constantcolourglow`,
`frontendconstantfranelblend`, `cf_fetracks` - all sit in the 2,052-block
minority that happens to carry no `zoneTex*`. Reading three of them and
concluding "never" was sampling the exception.

**Counted mechanically off the disc image, and asserted rather than reported**:
`crates/formats/tests/zone_shader_census_ground_truth.rs` re-derives every
number in this table from `data/images/hdfury-ps3-eu-dec.iso` on each
`just test-data` run, so the table cannot quietly drift from the disc.
Confidence **88** on the census.

**One earlier version of these figures was wrong and is worth recording as a
trap.** The first count was taken over a directory that a second process had
extracted *other* archives into, so it swept 1,592 files believing them to be
DATA00's 693 and reported 20,092 / 19,958 / 134. The shape of the finding
survived unchanged - it is a ratio, and both corpora were dominated by the
same materials - but every absolute number was wrong. The lesson is narrow and
sharp: **a census taken over an extraction directory is only as trustworthy as
that directory's exclusivity.** The test above reads the image directly and
has no extraction step for this reason.

### How the two terms combine, which was previously unread

In a block that has both, the texture term is a `MAD` **onto** the rim terms:

```text
@0x2c  MAD H3.xyz, H3, R3.yyyy, R2      ; zoneBase*rim^10 + zoneBaseAlt*rim^5
@0x48  MAD H3.xyz, H6, H7, H3           ; + zoneTex * zoneEffect
@0x4b  MAD H1.xyz, H2, H3, H4           ; colour = light * surface + glow
```

read from `02_track/materials/diffuse.rcsmaterial` block 11 and
`05_ubermall/materials/billboarddiffuse.rcsmaterial` block 11, which are
**byte-identical** - two unrelated materials on two unrelated circuits. So:

```text
surface = zoneTex(zoneUV).rgb * zoneEffect<I|O>.rgb
        + zoneBase<I|O>.rgb    * rim^10
        + zoneBaseAlt<I|O>.rgb * rim^5
```

and **the albedo is not in that sum at all.** In block 11 the material binds
five samplers and every one is a zone texture; in the sibling
`02_track/materials/diffuse_specular.rcsmaterial` block 15 `Texture1` *is*
bound but only its `.w` is read, for gloss. That is what makes the original's
Zone frame near-monochrome, and it is a stronger statement than the rim terms
themselves - `rim` is ~0 on a camera-facing surface, so `rim^10` and `rim^5`
are silhouette-only. Confidence **82**: two programs read end to end, plus the
census showing the shape is universal on circuits, against no dataflow proof
that the minority of blocks which *do* fetch albedo RGB keep it out of the
surface.

**One retraction here was itself too broad, and is withdrawn.** This page also
said `zoneAnisoPower` "is not used by this family at all", and that was struck
on the grounds that `zone_1/materials/billboarddiffuse.rcsmaterial` declares it
(`0x7d494659`, **float2**, which does confirm the two-lane reading). That is
not a counterexample: `billboarddiffuse` is the *sampling* shape, not the
rim-only one, so it says nothing about the shape the sentence was about. Read
off the disc, `cf_constantcolourglow` declares `zoneAnisoPower` in **none** of
its twelve Zone blocks. **The original sentence was true of the material it was
written about; only the word "family" was wrong.** Over-correcting a claim is
its own way of putting something false on the page.

`GradientColour1` is declared as a **sampler on unit 0** in
`zone_2/materials/gradientcolour1.rcsmaterial` - the slot `Texture1` occupies
in the otherwise byte-identical `billboarddiffuse` program. So parameters 68-71
hold texture handles, which raises
[zone-effectsettings-loader.md](zone-effectsettings-loader.md)'s twenty-third
pass from "confidence only 60 that they hold texture pointers at all" to about
**78**.

## What this does not settle

**Corrected 2026-08-31: `0x003d8b40` is not `zoneTexVis`'s build loop.** It
was read as one - "a load-time RGB-to-RGBA expansion of a static table, nine
passes of 64 texels" - and that reading was wrong on both the object and the
pass count. Traced independently: it is ten passes (the loop's own
`bne`/`cmpwi` tests the *pre-decrement* offset, which includes zero, so the
prior count missed the last pass), 640 texels, and it builds a **different**
object entirely - a 64x10 three-colour sparse glyph at `iVar8+0x335c`
(`0x00c8130c`), not the 256x1 strip at `iVar8+0x32b0` (`0x00c81260`) that
`zoneTexVis` actually is. The two addresses are 172 bytes apart in the same
struct, which is almost certainly how the first read conflated them.

**`zoneTexVis`'s own load-time fill is a plain zero-fill**, 256 texels of
`0x00000000`, a few dozen instructions after the `0x335c` loop in the same
function (`Environment_LoadStageTextures`).

**And it *is* rewritten every frame, found this pass**:
`Environment_UpdateStageBlend` (`0x003da540`), called every frame from
`Scene_PrepareFrame` (its only caller, and the same function that publishes
every other Zone shader parameter), writes computed packed-RGBA colour words
into at least eleven of `zoneTexVis`'s 256 texel slots (indices `1`-`10` and
`161`, traced explicitly; the surrounding threshold dispatch was not fully
unwound and may touch more). So "static ramp vs. rewritten per frame"
resolves to **rewritten per frame**, confidence 74 - the writer and its
per-frame call path are both found statically, capped below the loop-mechanics
score because *what* is written is not: the value stored is a float `f1`
compared against per-stage thresholds, and whether that float is itself
audio-reactive or purely a stage-progress fraction is untraced. The eight
vec4s at `0x00c81460`-`0x00c814d0` remain a separate, still-unfound per-frame
handle in this shader - this finding does not settle those.

Full evidence, the corrected disassembly and the trace that re-derives
`zoneTexVis`'s own address independently (rather than trusting this page's
prior citation) are in
[zone-effectsettings-loader.md](zone-effectsettings-loader.md)'s own pass on
this - see that page for the instruction-level detail this summary omits.

Settling it needs an RPCS3 **write watchpoint on the pixel buffer**
(`*(texture object at 0x00c81260 + 16)`) during a Zone race, not more static
reading. Confidence 75 that this is the right next experiment. Until it runs,
a port can reproduce the *lookup* faithfully and must not invent what varies.

## Two `scripts/ps3-microcode.py` defects found on the way - both now fixed

Both would silently mislead the next reader, which is worse than a crash:

1. **The per-instruction condition code was dropped** (word1 bits 20:18 test,
   bits 28:21 swizzle, `TR`/`xyzw` = `0x727` the unconditional default).
   Without it the inner arm of the sphere test reads as dead code and the
   whole inner/outer mechanism disappears. Confidence 80.
2. **The parameter patch chain was described in the tool's own docstring but
   never implemented**, so constants printed as `{0, 0, 0, 0}` where a
   parameter belongs. Confidence 82.

**Both fixed 2026-08-31**, and swept over 400 materials / 656,219 fragment
instructions: no crashes, 60% of inline constants now resolve to the parameter
that patches them (the rest are genuine literals, which a shader also has), and
3% of instructions carry a non-default condition - a rate consistent with real
predication rather than with a field being misread as one. A second copy of
`cf_constantcolourglow` carries 20 `EQ(wwww)` against 18 `GE(wwww)`: near-equal
counts of complementary predicates on one lane, which is the shape of two arms
of a selection and not the shape of noise.

**One caveat kept rather than smoothed over**: the condition field is verified
on ALU instructions. Texture ops decode to non-default values too, and mixed
opcodes carrying conditions is part of what argues the decode is real - but
whether those bits mean the same thing for a texture fetch is unchecked, so a
condition printed on a `TEX`/`TXP` is unconfirmed.

**Anything read with that tool before 2026-08-31 and not hand-checked against
the raw words should still be treated as provisional** - the fix does not
retroactively validate output produced without it.

## See also

- [zone-effectsettings-loader.md](zone-effectsettings-loader.md) - the
  parameter table, the texture arrays and the stage blend that feed this
- [renderer.md](renderer.md) - the engine parameter table and the shader registry
- [effectsettings.md](../../../formats/effectsettings.md) - the authored table,
  and the play observation this page corroborates
