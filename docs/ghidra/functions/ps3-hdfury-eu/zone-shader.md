# HD/Fury's Zone shader, read out of the RSX fragment microcode

**The Zone effect is not an engine program.** No Zone name appears in the
EBOOT's own 121-name shader census. Zone is a set of compiled **variants
inside the materials themselves** - 1,467 of the disc's 1,590
`.rcsmaterial` files carry one. That is why every search of the executable for
a "Zone shader" came back empty: there is nothing there to find.

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

**Whether the 256 LUT entries hold live audio levels.** The LUT's *contents*
are a memory question, not a shader question. The build at `0x003d8b40` is a
load-time RGB-to-RGBA expansion of a static table (nine passes of 64 texels;
the source base resolves through the TOC to `0x008c2d78` -
`python3 scripts/ps3-toc.py resolve 0x003d6dc8 -0x57D0`). **So if nothing
rewrites the texture's pixel buffer per frame, it is a static ramp and the
animation comes from somewhere else** - the eight vec4s at
`0x00c81460`-`0x00c814d0`, whose writer is still unfound, are the only other
per-frame handle in this shader.

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
