# HD/Fury's Zone shader, read out of the RSX fragment microcode

**The Zone effect is not an engine program.** No Zone name appears in the
EBOOT's own 121-name shader census. Zone is a set of compiled **variants
inside the materials themselves** - 1,467 of the disc's 1,590
`.rcsmaterial` files carry one. That is why every search of the executable for
a "Zone shader" came back empty: there is nothing there to find.

Read in full from three materials that agree with each other:

```sh
# the three materials read here (they live in DATA00; the textures in DATA02)
data/environments/zone_1/materials/billboarddiffuse.rcsmaterial
data/environments/01_vineta_k/materials/cf_constantcolourglow.rcsmaterial
data/environments/zone_2/materials/gradientcolour1.rcsmaterial
```

This page is the combination rule. What *feeds* it is on
[zone-effectsettings-loader.md](zone-effectsettings-loader.md) (the parameter
table, the per-stage texture arrays, the stage blend); what the palette table
authors is in [effectsettings.md](../../../formats/effectsettings.md).

## The rule, from `zone_1/materials/billboarddiffuse.rcsmaterial` block `0x2cf0`

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

## Two findings that change what the artists were doing

**The zone recolour applies only where the material's own albedo is black.**
`saturate((r+g+b) * 100000)` is a hand-rolled `step(0, x)`, and `surface =
albedo + zoneCol * (1 - blackMask)`. So the artists chose which parts of each
surface light up in Zone mode **by painting them pure black in the ordinary
diffuse texture**. Confidence 78. Nothing in the palette table or the parameter
list expresses this; it is authored in the albedo.

**The visualiser glow is gated to up-facing surfaces**: `saturate(N.y - 0.5)`.
That is the microcode independently producing the maintainer's own play
observation - *the floor* displays the equaliser - from a direction this
session did not go looking for it.

## `zoneBase*` is **not** a separate family - retracted, and the count says so

**The first version of this section said `zoneBase<I|O>` and
`zoneBaseAlt<I|O>` "never appear in the same block as `zoneTex*`" and belonged
to an untextured family. That is wrong, and it is wrong in the way this page
keeps warning about: it generalised from three materials read by hand.**

A per-block census over all 1,590 `.rcsmaterial` files in `DATA00`, counting
the parameter and sampler hashes each fragment block *declares*
(`~crc32(name)`, validated first against four known names in a block already
read by hand - `fogColour`, `constantAmbientColour` and the two
`directionalLight0*`):

| Zone-bearing fragment blocks | 20,048 |
| --- | ---: |
| declare **both** `zoneTex*` and `zoneBase*` | **17,906** |
| `zoneBase*` only | 2,052 |
| `zoneTex*` only | 90 |

So the two co-occur in **89%** of Zone blocks. The three materials the original
reading was drawn from - `cf_constantcolourglow`, `frontendconstantfranelblend`,
`cf_fetracks` - all sit in the 10% minority that happens to carry no
`zoneTex*`; `cf_fetracks` is among the census's own examples of that bucket.
Reading three of them and concluding "never" was sampling the exception.

**What survives.** The formula itself was read from real microcode in blocks
that genuinely have no texture term, and it stands *for those blocks*:

```text
colour = zoneBase<I|O>.rgb * rim^10  +  zoneBaseAlt<I|O>.rgb * rim^5
```

The `10` and `5` are inline literals, not parameters. What does **not** survive
is "a different shader family": the rim terms are part of ordinary Zone
shading, and in the 17,906 blocks that also sample a zone texture they combine
with the texture term rather than replacing it. **How they combine there is
unread** - the formula above is the untextured case, and nothing here measures
the textured one. Confidence 88 on the census (a count, re-derived
independently by two parties who agree on 17,906), 80 on the formula in the
blocks it was read from, and no claim at all about the combined case.

`zoneAnisoPower` was also said here not to be used by "this family". That is
retracted too: `zone_1/materials/billboarddiffuse.rcsmaterial` declares it
(`0x7d494659`, **float2**, confirming the two-lane reading) in the same block
that samples `zoneTexInner` and declares `zoneColourTint`.

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
