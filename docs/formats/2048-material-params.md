# Wipeout 2048's material instance table: the uniforms and samplers a model hands its shader

**2026-10-05.** `oag_rcs::rcsmodel::psp2::material` read a Vita `.rcsmodel`'s
materials for a year as a name, a technique and a list of `.gxt` paths found by
scanning, because the table of shader inputs after each header "changes shape
with entry count". It does not. It is one table of one entry size, with two
kinds of entry, and reading it is what makes 2048's scrolling surfaces and its
Zone colours readable at all.

## Layout (confidence 92)

Per material header (`+0x00` to `+0x3f`, see [2048-rcsmodel.md](2048-rcsmodel.md)):

```text
+0x20 u32  count of 32-bit floats      +0x24 u32  offset of the float pool
+0x28 u32  count of half floats        +0x2c u32  offset of the half pool
+0x30 u32  entry count                 +0x34 u32  offset of the entries
```

Each entry is `0x18` bytes:

```text
+0x00 u32  name hash, ~crc32(name)  (the hash Wipeout HD's records use)
+0x04 u32  kind: 0x12 a sampler, 1 a uniform
sampler:  +0x10 u32  offset of the .gxt path
uniform:  +0x0c u32  offset of the value
          +0x14 u32  its pool: 0x1000 the float pool, 0x2000 the half pool
```

A value runs to the next entry's value in the same pool, at most four
components. Halves are IEEE binary16.

**What the numbers say**, `crates/rcs/tests/psp2_material_param_ground_truth.rs`
over the three EU packages:

- **52,637 sampler entries**, every one a `.gxt` path the older address-order
  scan also finds - the very 52,637 `.gxt` strings the corpus counted before
  this table was read. 52,609 of them are names the material's own shader
  declares; the other 28 are HD materials ported into 2048 (`and_waterfall`'s
  `0xa2d555b9` is HD's `Texture2`).
- **15,565 uniform entries**, 15,561 of them a name one of the material's own
  GXP programs declares in the clear ([gxp.md](gxp.md)). The four others are one
  uniform on Tech de Ra's `tech_de_ra_cloud_effect`, in both directions of the
  circuit.
- **Zone colours agree with the materials' names**: `C_Red` reads `1 0 0`,
  `C_Pink` `1 0 1`; the eight `Zone_Colour1`..`8` files read eight colours.

The names are preimages (`~crc32(name)` equals the hash), found by hashing the
uniform and sampler names the GXP programs declare:
`oag_rcs::rcsmaterial::names::KNOWN_PSP2_PARAMETER_NAMES` carries them -
`Zone_Colour1..8`, `Zone_Colour1..8_Emissive`, `TimeScaler`,
`Emissive_UV_Offset`, `Emissive_UV_Scale`, `GlowTint`, `speed_multipliaer` (sic),
`time`, `frameRate`.

**Omega reads the same table**, widened to 64-bit pointers and `0x28`-byte
entries, with each uniform stating its component count
([omega-status.md](omega-status.md#the-material-uniform-and-sampler-table-2026-10-05-omega-uv-scroll)).
Omega's `uv_anim` materials are 2048's shaders, and the glow layer below plays
them unchanged.

## What a Zone circuit is made of

**A 2048 Zone race draws `trackZone.rcsmodel`**, not `track.rcsmodel` with a
filter. `altima`'s is 37 MB against the race model's 17, with its own skeleton
(966 nodes), clip (7 tracks at 30 Hz, a 6.7 s loop), PVS (1.27 MB against 428 KB)
and probes. Its 6,349 submeshes:

| Materials | Submeshes | What the file says |
| --- | ---: | --- |
| `zonefc07_cube_animation_1` | 2,406 | node-bound, 2,302 hidden at bind; nothing in the 7-track clip shows them |
| `fc01_dummy` | 1,716 | the road and walls; a fragment program with one uniform (`fogColour`) and no sampler |
| `zonefc06_colour_emissive_scalar_01..08` | 1,587 | `Zone_ColourN` (rgb) and `Zone_ColourN_Emissive` (`0.5` on every one) |
| `zonefc09_eq_gradient`, `_pixelated` | 371 | textured |

**What is drawn, and what is chosen, not measured:**

- **`fc01_dummy` is not drawn.** What its program outputs is GPU bytecode no
  one here has decoded; a white or a black stand-in would be invented. The
  report says `1716 submesh(es) on the placeholder shader fc01_dummy not drawn
  (its output is unread)`. **This removes the Zone circuit's road from the
  picture.** No Vita3K capture of a Zone race exists in this tree to compare.
- **`Zone_ColourN` is drawn as the surface's colour, unlit.** The fragment
  program also declares a lightmap and `liveLighting0`, so lit is the other
  candidate. Both were drawn on `altima`: lit, the colour is multiplied by a sun
  of `2.0 1.8 1.7` and every surface blows out to white; unlit, the eight
  colours read as eight colours. The `0.5` emissive scalar is read and not
  applied.
- **`zonefc07_cube_animation_1`'s 2,302 hidden nodes stay hidden.** Their
  visibility is the skeleton's own, and the clip shows none of them; what shows
  them in the original (the zone counter, the music) is not read.

Every 2048 material also declares `zoneGrowingPaletteScene`,
`zoneGrowingPaletteTrack` and `zoneGrowingTexture` samplers and
`zoneGrowingTextureFactors`, `zoneShipPos` uniforms: 2048's Zone look has a
runtime term that recolours the circuit around the craft, which no file here
authors. It is not read.

**Nothing outside `trackZone` meets the placeholder or a Zone colour**: 0
`fc01_dummy` and 0 `Zone_ColourN` submeshes over the 983 Vita models (base,
DLC1, DLC2, ships and skies included) and Omega's 1,150, so `build`'s skip and
colour reach no other model (`the_placeholder_and_the_zone_colours_exist_only_in_the_zone_models`).
Likewise the 355 glow-layer materials' sampler-named diffuse is always the
diffuse the older scan already picked, so the layer changes no texture.

## What scrolls, and what is acted on

Census over every race and Zone circuit's model, in submeshes:

| Shape | Names the material authors | Submeshes | Acted on |
| --- | --- | ---: | --- |
| glow layer | `Emissive_UV_Offset`, `Emissive_UV_Scale`, `TimeScaler` or `time`, `GlowTint`, an emissive and a diffuse sampler | about 790 (`fc09`/`fc10_effects_vscroll_lambertalpha_emissive` 474, `mt_uvanim_*` 179, `uvanim_diffuse_emissive*` 131) | yes |
| plain V scroll | `speed_multipliaer` on one sampler | 113 (`fc01_effects_vscroll_emissive` 41, `fc06_effects_vscroll_emissive_alpha` 72) | yes |
| `TimeScaler` alone | `fc02_effects_uscroll_scanlines_emissive` 85, `videoscreen_..._scrolling` 66 | 151 | no |
| flipbook | `frameRate`, `fc07_lambert_alpha_uvanim_5x5colume` | 167 | no |
| `time` and nothing else | some 60 materials (`mageffect08` 115, `fc08_effects_crowd` 98, `scanlinebillboard` 60, `mr_uvanim_em_alpha` 57, `scroller_glow_v3` 47, ...) | | no: the constants are inline literals in bytecode |

**The glow layer is Wipeout HD's**, and the uniforms are HD's own hashes: the
same three (`Emissive_UV_Offset` `0x78256a45`, `Emissive_UV_Scale`
`0x78787596`, `GlowTint` `0xe8bcd7f5`) that HD's `uvanim_diffuse_emissive` reads
([rcsmaterial.md](rcsmaterial.md), "A surface scrolls off an engine `time`"),
now with their names. So the layer is `oag_mesh::mesh::Emissive` and
`mesh::slots::ADD_SECOND`, sampled at `(u, (v + offset) * scale + time * rate)`,
added to the albedo and gated by the diffuse alpha, with the emissive texture
bound beside the diffuse the way HD binds its second one.

**Chosen, not measured, and labelled so in the load report**: `rate` is
`TimeScaler` where authored, else the authored `time` value, else `1.0` (HD's
engine clock); and the plain scroll runs `+v` at `speed_multipliaer` per
second. No 2048 bytecode was read to settle either, and the **direction of the
plain scroll is a guess** - Wipeout HD's Talon's Junction authors `-1` for the
same material family. Where `time` is authored on a 2048 material it is `1.0`,
`0.0` or `1.1448`: a multiplier, by the values, and not the engine's clock.

`crates/render/tests/psp2_glow_ground_truth.rs` holds what the build does with
`arena`'s three glow layers (rates `0.05`, `1.0`, `3.0`) and two plain scrolls
(`0.3`), and `altima`'s twelve plain scrolls.

## Not read

- **Bytecode.** No fragment or vertex program was decoded: how a Zone colour,
  the light and the emissive scalar combine, where `TimeScaler` enters the
  coordinate, which sign a plain scroll takes, what `fc01_dummy` outputs.
  [gxp.md](gxp.md) has the container; the instruction set is not read.
- **The other scroll shapes** in the table above, and the flipbook's 5 by 5
  frame cycle.
- **The names of 2 of `cf_uvanim_emssive_glowtint`'s uniforms** (`0x87d769dc`,
  `0x2481ef75`) and of `mt_uvanim_diffuse_emissive3tokey`'s `0xef18f362`.

## Why a 2048 floor reads white (2026-10-05, fix-2048-particles-floor)

Two causes, found by dropping materials from the build (a temporary needle on the material name, not kept)
and by drawing unlit; frames in `data/scratch/fix-2048-particles-floor/`.

- **Altima: the white floor is `track_a_glass_etched`.** Dropping materials whose name or texture contains
  `glass_etched` (`fc01_emissive_alpha_emistint` 402 triangles, `fc12_phong_alpha_emistint_spectint_specpow`
  260, technique `tracksurface:*`) removes it and shows the rock below. `psp2::build_planned` leaves
  `blend: None` on every draw, so a glass floor whose material names `Alpha` draws opaque. Drawing the
  blend is the queued transparent-materials lane, not this one; no blend rule was invented here.
- **Tower: the lit sum saturates.** Drawn unlit, tower's floor is a grey panelled texture; lit, it is
  peach, so the term is the light. Altima's authored rig is sun diffuse `2.0 1.8 1.7` over ambient
  `0.15 0.25 0.38`, the file also authors `Lighting.ExposureScale 0.4`, `ExposureMax 12`, `BloomFactor 0.3`,
  `BloomGate 0.3`, none read by this port, and the registrar's field addresses have no readable consumer
  by absolute xref (Vita `movw/movt`). Scaling the sun to 0.2 or 0 still left the floor pale, so the sun
  alone is not the whole of it.
- **Not the merged `mag_wave` dodge:** tower at tick 3640 is byte-identical on main and in the render from
  before that merge (`cmp`), so the white predates it and 2048 is untouched by it.
- **Vita lightmaps are bindable and unbound.** `Material::lightmap` is `None` on every Vita material, but the
  sampler table now carries the `lightmap` hash (`0x37b5db58`, HD's) with an `lmaps/*-lmap.gxt` path: 36
  atlases on Altima, and the atlas is mostly white with dark patches. Binding them changed no pixel of the
  floor (the floor submeshes declare no `lightmapUV`), so it was not shipped. Next step.
- **Nova prelit and `NO_SUN` on lightmapped draws** (Omega's combination) changed nothing either; the
  `no_sun` bit only feeds `emissive` in `mesh.wgsl`.

**Next:** read the Vita `fc16_phong_alphaspec_normal_emissive` and `fc18` fragment programs (the GXP
instruction set is unread) for how sun, ambient and the exposure keys combine; the Vita3K reference frames
in `data/reference/2048-hud/` are other circuits, so a same-circuit capture of Altima or Tower is needed to
compare.

## Altima's floor, Tower's floor, and what was not found (2026-10-05, `omega-2048-materials`)

- **Altima: `track_a_glass_etched` is a texture, not a material, and the white floor no longer reproduces.**
  No Vita header names `etched`; the two materials the floor draws with are `fc01_emissive_alpha_emistint`
  (79 blended materials in the base package) and `fc12_phong_alpha_emistint_spectint_specpow` (6), both state
  mode 1 (blended). Before `transparent-floors` routed mode 1 they drew opaque, which is the blown-white sheet the
  previous section describes; at tick 300 and 600 on current main Altima's floor is grey panelling with the
  bright pads as glow decals (`data/scratch/omega-2048-materials/shots/before_altima_*.png`). The blend those two
  use is alpha-over, **chosen**: neither name is in HD, so nothing is inherited.
- **Tower: not solved.** Its floor is still peach where drawn unlit it is grey. Re-read this pass: the file authors
  `Lighting.ExposureScale 0.4`, `ExposureMax 12` and `ExposureSpeed` (strings `0x81508818`, `0x81508830`,
  `0x81508848`) and no consumer was located, so how the exposure enters the lit sum is unread. Omega's tone map
  ([tonemap.md](../ghidra/functions/ps4-omega-eu/tonemap.md)) is a different, adaptive cubic and was **not**
  assumed to apply. A second Omega shader (`0x01968180`, `ExposureMin`/`Max` clamping `m_cExposure / max(L,
  0.001)`, referenced by no Omega code) has the shape of this triple, a hypothesis below 50 and not applied.
  No term was scaled on a guess, so Tower is unchanged.
- **Fragment-program blend in the executable:** [fragment-programs.md](../ghidra/functions/vita-2048-eu-v104/fragment-programs.md).
