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

## Open

- **What selects a variant.** The two leading hashes of a variant record are the
  obvious place and are unread. Everything else on this page is downstream of
  it.
- **The rest of the 0x40-byte record**: two counts and further `(offset, length)`
  pairs, one of which points at 16 bytes rather than a program.
- **87 of the 125 sampler hashes**, including the three commonest
  (`0xd5e000d1`, `0x00e5b679`, `0x1f6f85a3`, ~4,400 uses each across every unit),
  which by their spread are engine samplers rather than a material's own.
- **The microcode**, which is where the operation - multiply, add, replace -
  actually is.

## See also

- [rcsmodel](rcsmodel.md) - the material record, and the two texture paths
- [renderer.md](../ghidra/functions/ps3-hdfury-eu/renderer.md) - the `SHO` block
  framed, and the executable's own 124 shader programs
- [gtf](gtf.md) - the textures both slots name
