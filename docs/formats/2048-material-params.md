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
| `fc01_dummy` | 1,716 | the road and walls; a fragment program with one uniform (`fogColour`) and no sampler; draws that colour flat |
| `zonefc06_colour_emissive_scalar_01..08` | 1,587 | `Zone_ColourN` (rgb) and `Zone_ColourN_Emissive` (`0.5` on every one) |
| `zonefc09_eq_gradient`, `_pixelated` | 371 | textured |

**What is drawn, and what is chosen, not measured:**

- **Revised 2026-10-27: not flat.** The instruction listing ([gxp.md](gxp.md#the-instruction-stream)) reads
  the setup op as `-fogColour + 0`, not a clamp, so the pixel is `fogColour * (1 - f)` with `f` the vertex
  program's exponential-squared fog visibility (1 near, 0 far): black near, `fogColour` in the distance.
  The paragraph below is the 2026-10-19 reading and is wrong on this point; the renderer's `FOG_FLAT`
  draw has not been re-read against it.
- **`fc01_dummy` draws a flat fog colour (2026-10-19, corrected above).** Its fragment program is a
  phase marker, two arithmetic instructions and a closing scale, in two
  variants that differ in one swizzle (which component of the interpolant
  multiplies; 1 and 8 copies per package, byte-identical across `data`,
  `data1` and `data2`, absent from DLC1/DLC2). In plain terms: the setup block
  clamps the negated `fogColour` at zero, which is zero for any non-negative
  colour; the one multiply-add is that clamped value times the interpolant's
  component plus `fogColour`, so the colour is `fogColour` whatever the
  interpolant holds; the closing scale (taken as 1.0, the constant every
  fragment program ends with) writes alpha 0. **The pixel is `fogColour`**:
  no texture, no light, no fog lerp. Method: the operand fields of those words
  were matched by eye against a few hundred labelled words from two Vita3K
  shader logs (an emulator's own disassembly was read for behaviour on the
  maintainer's 2026-10-19 allowance; nothing of its text or tables is copied
  here, and the instruction set is still not named). Confidence 70 on the
  arithmetic: no frame of Altima's Zone was captured (the HD campaign's Zone
  event on Vineta K runs DLC `zone_N` circuits and never this program; 2048's
  own Zone event needs campaign progress the capture recipe cannot reach).
  **Which value the engine binds as `fogColour` is chosen, not measured**: the
  circuit's `Lighting.Fog colour` (Altima `0.129 0.110 0.165`), fixed at load.
  Two known divergences: (1) the Zone grade replaces the scene fog per stage
  and this colour does not follow it (a road pixel stays `34 29 43` at 960x544
  while distant geometry fogs toward the graded colour); (2) our pipeline
  fogs these draws like any other, where the original's program has no fog
  lerp. The Vita3K reference frame of a Zone race (Vineta K, zone 0 and 5)
  shows a near-black road, which the authored colour matches and the graded
  scene fog (light grey in our Altima Zone 0) does not; that picture is the
  only evidence for the choice. Open lead: the binding is CPU-side in
  `/vita-2048-eu-v104` (Ghidra) and would settle both. `oag_mesh` marks the
  vertices (`psp2::FOG_FLAT`), `oag_raceplay::load` writes the colour in and
  reports `fc01_dummy: N vertices drawn as fogColour`; the game's Zone race
  now draws `trackZone.rcsmodel` (`Options::zone_model`, a reversal of the
  2026-10-05 off-by-default decision, for the lead). **Omega: checked, applies in part, not
  wired (2026-10-19).** An earlier line of this page said Omega has no
  `fc01_dummy`; that was wrong, it only held outside `trackZone`. Census over
  Omega's psarcs (`data00`, `data04`, patch `data08`): the ten 2048 circuits'
  `trackZone.rcsmodel` ship re-cooked under `Data/environments2048/`, with
  `fc01_dummy` on most of their materials (altima 173 of 239, subway 919 of
  1,037, bridge 612 of 680, sol 148 of 228) beside 22 to 63 `Zone_ColourN`
  ones each. Omega's own `fc01_Dummy.rcsmaterial` is not Vita's program
  family: its pixel programs, read off the GCN words by hand (no tool of this
  project decodes them), are one 24-byte program that exports zero and no
  constant, seven 216-byte programs reading `fogColour` and `fadeResCoeffs`
  that export three render targets (a deferred-style pack, not a colour), and
  two 440 and 532-byte `zoneGrowing*` programs reading ambient, light and the
  palette textures. So the Vita law (flat `fogColour`) is **not confirmed and
  not applied on Omega**: the output is packed, has a fade term, and the
  `zoneGrowing` variants are the runtime Zone look this page lists as unread.
  Omega's Zone race is not wired (`is_ps4` passes the Zone model over), so
  nothing draws it; reading the 216-byte program through to its packed
  channels is the open step.
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

## The picture is the sampler named `DiffuseMap`, then the first in file order

2026-10-19, `omega-pink-road-sun`. Name hashes of the sampler entries resolve
(`name_hash`, `~crc32`): `0x38c2e35a` `DiffuseMap`, `0x48f37f5a` `NormalMap`,
`0x19d166c1` `EmissiveMap`, `0x9ee31012` `Diffuse`, `0xd9d6922d` `Normal`,
`0x739a786e` `NormalTexture`, `0xb90df195` `RockColour`, `0x37b5db58` `lightmap`.
`Material::diffuse_texture` takes the first of `DiffuseTexture`, `DiffuseAlphaMap`,
`DiffuseMap` the material binds, and the first non-lightmap texture otherwise.
The file-order answer was right for 7,340 of the 7,419 `DiffuseMap` entries over
Omega's circuit models and wrong for the other 79: `road_alphaspec_normal_emissive_panelmap1`
lists `track_a_floor_panel_df` (`0x4736b3bf`, a 1024x256 single-channel `R16F` panel
map the decoder refuses) first, and Altima's whole road drew untextured. Omega: **ported**
(altima unresolved draws 690 to 449). 2048's Vita road is
`fc16_phong_alphaspec_normal_emissive` with `track_a_floor_ds` as `DiffuseMap`: **checked,
differs**, unchanged. The R16F map's own use (a panel layout the shader reads) is unread.

## What scrolls, and what is acted on

Census over every race and Zone circuit's model, in submeshes:

| Shape | Names the material authors | Submeshes | Acted on |
| --- | --- | ---: | --- |
| glow layer | `Emissive_UV_Offset`, `Emissive_UV_Scale`, `TimeScaler` or `time`, `GlowTint`, an emissive and a diffuse sampler | about 790 (`fc09`/`fc10_effects_vscroll_lambertalpha_emissive` 474, `mt_uvanim_*` 179, `uvanim_diffuse_emissive*` 131) | yes |
| plain V scroll | `speed_multipliaer` on one sampler | 113 (`fc01_effects_vscroll_emissive` 41, `fc06_effects_vscroll_emissive_alpha` 72) | yes |
| `TimeScaler` alone | `fc02_effects_uscroll_scanlines_emissive` 85, `videoscreen_..._scrolling` 66 | 151 | no (reason below) |
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

**What the glow layer's rate is, as of 2026-10-06 (`psp2-scroll`):**

- **No `TimeScaler`: the engine clock, `1.0`, inherited from HD.** The
  `mt_uvanim_diffuse_emissive*` and `and_anim_spec` records are HD's own
  materials carried into 2048's DLC1: the same `Emissive_UV_Offset` and
  `Emissive_UV_Scale` values and the same `GlowTint` to three decimals on
  Anulpha Pass, Vineta K and Chenghou (`crates/render/examples/psp2_scroll_census.rs`). HD's matching fragment program was
  read at instruction level (`MAD R0.w, R0.zzzz, {b}, R0` with `R0.w = time`:
  `(v + a) * b + time`, rcsmaterial.md, "A surface scrolls off an engine
  `time`"), and the engine clock enters at coefficient one. HD's records never
  author `time`; 2048's author `0.0`, `1.0`, `1.0144`, `1.1448` and, on
  `nr_twinblend`, `19992.0` on the same shader, which reads as a leftover
  tool-side snapshot of a clock rather than a rate. **The authored `time` is
  therefore no longer read as the rate** (it was, until this change: a `time`
  of `0.0` froze a layer, `1.1448` slowed it). No 2048 instruction was read:
  **inherited by name and by identical authored values, not measured on 2048.**
- **`TimeScaler` authored: the rate is `TimeScaler`, chosen, not measured.**
  The three families that author it (`fc09`/`fc10_effects_vscroll_lambertalpha_emissive`,
  `fc02_effects_*vscroll*`) are 2048's own, no HD material shares the name, so
  HD's law has nothing to say. What can be said is structural: the GXP parameter
  tables put `time` and `TimeScaler` (and `time` and `speed_multipliaer`) in the
  **same stage** on every such shader (`crates/render/examples/psp2_scroll_stages.rs`: fragment for `fc09`/`fc10`, vertex for
  `fc01`, `fc02`, `fc06`), the shape of HD's `uv + time * rate`. At
  `TimeScaler 1.0` the choice equals HD's coefficient of one.
- **The plain scroll runs `+v` at `speed_multipliaer` per second: chosen, not
  measured.** The same-stage evidence above supports the shape (a rate times
  the clock added to the coordinate) and says nothing about the axis or the
  sign. The earlier note pointing at Talon's Junction's `-1` was a glow layer's
  `Emissive_UV_Scale`, not a plain-scroll sign, and is withdrawn.
- **The unit stays seconds**, inherited from HD (`time` is one tile a second at
  rate one there); not measured on 2048.

## Inherited from HD: the vertex scroll (2026-10-06, `psp2-scroll`)

HD's vertex programs add `time * rate` to the coordinate with the rate **authored
on the material** ([rcsmaterial.md](rcsmaterial.md), "HD's vertex programs scroll
the texture coordinate"). **2048 and Omega author the very same rate hashes on
the same shader names** (`crates/render/examples/psp2_hd_rate_census.rs`):

| Shader | HD rate hashes | 2048 records (base, DLC1, DLC2) | Omega records |
| --- | --- | ---: | ---: |
| `cf_uvanim_emssive*` | `0x87d769dc` x, `0x2481ef75` y | 103 | 70 |
| `basic_uv_scroll` | `USpeed`, `VSpeed` | 6 | 6 |
| `emissive_bloom`, `emissive_lights` | `0x68292521` (v) | 60 | 92 |
| `hologram` | `Speed` (u) | 16 | 22 |
| `uv_anim_diffuse_alpha` | `0x33d51367` (u) | 0 | 8 |

(Records are `.rcsmodel` material entries, so one shader repeats per circuit.)

On 2048 the shader's vertex program also **declares `time`** (`cf_uvanim_emssive*`
and `emissive_lights`: `time:V` in the stage census), the same stage as HD. So
`oag_mesh::mesh::rcs::vertex_scroll::inherited_rate` admits a material on a
name HD ships, a record that authors the rate hashes (all of a shape's, never a partial set), exactly one texture besides
its lightmap (HD's own rule), and, **where the programs can be listed**, a vertex
program that declares `time`. On Omega the programs are GCN and unread, so the
name and the hashes alone admit it. It reaches the existing
`AnimTrack::Scroll`, deduplicated with the plain scrolls.

**Reach**, the report's `inherited_scrolls` (`psp2_scroll_reach`):
Anulpha Pass 18 (2048) and 11 (Omega), Ubermall 10 and 11, Amphiseum 6 and 15,
Modesto Heights 4 and 7, Tech de Ra 3 and 5, Moa Therma 3 and 7-9, Vineta K 2
and 6, Sol 2 1 and 1, Chenghou 0 and 2; **0 on every 2048 circuit of its own** (and Altima's frames are byte-identical on main and this branch, on 2048 and on Omega),
which is the guard that nothing switched on beyond the families checked.
`crates/mesh/tests/psp2_glow_ground_truth.rs` pins Anulpha Pass on both titles.

**Checked, differs:** 2048's `mr_uvanim_em_*`, `cf_chenghou_sign*`, `nr_twinblend`
and the like declare `time` and author no HD rate hash, so their constants are
inline in bytecode: still. HD's values also differ per circuit (Anulpha's
`cf_uvanim_emssive_glowtint` pair is `1,0` on some copies and `0,0.066` on
another), so nothing was copied across: each record's own authored rate plays.

**Still, with the reason:**

| Family | Submeshes | Why it stays still |
| --- | ---: | --- |
| `TimeScaler` alone (`fc02_effects_uscroll_scanlines_emissive`, `videoscreen_*_scrolling`) | 151 | `TimeScaler` and `time` are vertex-stage; the axis and sign live in the name only (`uscroll`) |
| flipbook, `fc07_lambert_alpha_uvanim_5x5colume`, `frameRate` | 167 | the 5 by 5 grid and the frame order are in the material **name**, not in any authored field; no HD material ships the family |
| inline-`time` families | about 60 | the constants are literals in bytecode |

Both flipbook and `TimeScaler`-alone need a decoded instruction stream, which
`gxp.md` still does not have (97,899 containers decode, no opcode is named).

**Omega: ported.** The glow layer's clock rule and the inherited vertex scroll
reach Omega through the same plan; Omega's `scrollingalpha` `V_Offset` is on no
circuit material (weapons and muzzle flash only), as on HD.

`crates/mesh/tests/psp2_glow_ground_truth.rs` holds what the build does with
`arena`'s three glow layers (rates `0.05`, `1.0`, `3.0`) and two plain scrolls
(`0.3`), and `altima`'s twelve plain scrolls.

## Not read

- **Bytecode.** No fragment or vertex program was decoded: how a Zone colour,
  the light and the emissive scalar combine, where `TimeScaler` enters the
  coordinate, which sign a plain scroll takes, (`fc01_dummy`'s output is read, above). Only
  the parameter tables (`gxp.md`) were read, for which stage declares which name.
  [gxp.md](gxp.md) has the container; the instruction set is not read.
- **The other scroll shapes** in the table above, and the flipbook's 5 by 5
  frame cycle.
- **The names of 2 of `cf_uvanim_emssive_glowtint`'s uniforms** (`0x87d769dc`,
  `0x2481ef75`) and of `mt_uvanim_diffuse_emissive3tokey`'s `0xef18f362`.

## Why a 2048 floor reads white (2026-10-05, fix-2048-particles-floor)

Two causes, found by dropping materials from the build (a temporary needle on the material name, not kept)
and by drawing unlit.

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
  bright pads as glow decals. The blend those two
  use is alpha-over, **chosen**: neither name is in HD, so nothing is inherited.
- **Tower: not solved.** Its floor is still peach where drawn unlit it is grey. Re-read this pass: the file authors
  `Lighting.ExposureScale 0.4`, `ExposureMax 12` and `ExposureSpeed` (strings `0x81508818`, `0x81508830`,
  `0x81508848`) and no consumer was located, so how the exposure enters the lit sum is unread. Omega's tone map
  ([tonemap.md](../ghidra/functions/ps4-omega-eu/tonemap.md)) is a different, adaptive cubic and was **not**
  assumed to apply. A second Omega shader (`0x01968180`, `ExposureMin`/`Max` clamping `m_cExposure / max(L,
  0.001)`, referenced by no Omega code) has the shape of this triple, a hypothesis below 50 and not applied.
  No term was scaled on a guess, so Tower is unchanged.
- **Fragment-program blend in the executable:** [fragment-programs.md](../ghidra/functions/vita-2048-eu-v104/fragment-programs.md).

## Vertex-lit and lightmapped combines (2026-10-31, `2048-track-lighting`)

**Method.** Vita3K caches the SPIR-V it translates each program to under the SHA-256 of the GXP
container (`~/.cache/Vita3K/shaders/<title>/eboot.bin/vk13-<sha>.spv`), so hashing every container
`scripts/vita-gxp.py` finds (`patch-v104/eboot.elf`, `data.psarc`, `dlc1.psarc`, `data1.psarc`)
names the translation of a program a race actually ran: 940 of 984 cached programs matched. The
SPIR-V was walked symbolically over its half-register files (the same walker as the exposure read,
with `FMax`'s second argument kept). Vita3K is a behaviour oracle only; the formulas below are
written by hand. Conf. in brackets.

**The lightmapped family** (every program that declares `lightmap`: 123 of 473 cached fragment
programs; all take `novaScaleBiasPower`, `liveLighting0diffuse`, `liveLighting0direction`,
`fogColour`, and **none declares `constantAmbientColour`**) [90]. For `fc06_lambert_simple` +
`DiffuseMap` + `lightmap` (sha `efaebb76...`):

```text
N      = normalize(interpolated normal)
L      = lightmap texel (rgba, raw, no decode)
prelit = L.rgb * scale + bias + sunDiffuse * max(0, N . liveLighting0direction) * L.a
rgb    = mix(fogColour, prelit * D*D, fogVisibility)        D = DiffuseMap texel
```

`(scale, bias)` are the first two lanes of `novaScaleBiasPower`; **the third lane (power) is read by
no instruction**, unlike Omega's, whose GCN applies `pow(lightmap, power)`
([lightmap-prelit.md](../ghidra/functions/ps4-omega-eu/lightmap-prelit.md)). So the combine is
Omega's law with the power fixed at 1: *ported*, with the Omega-only power left out. The lightmap's
alpha is the sun mask, as on HD and Omega.

**The vertex-lit family** (a submesh whose vertex declaration carries `NOVAColor`, no lightmap; the
variants `fc06_lambert_simple` `45dd7539...` and its vertex program `e0625aba...`) [88]. The vertex
program writes `rgb = NOVAColor.rgb * scale + bias` and `a = NOVAColor.a` as varyings (no power, no
log/exp), and the fragment program computes `(sunDiffuse * max(0, N . dir) * a + rgb) * D*D`,
fogged. No constant ambient either. The attribute name is proved: `~crc32("NOVAColor") =
0x1aaf7631`, a four-byte attribute `rcsmodel_decl_ground_truth.rs` had counted as unnamed (type `0x44`,
four unsigned normalised bytes, at stride offset 0x14 on Vineta K's 24-byte layout).

**The ambient family** (`constantAmbientColour`, no lightmap, no `NOVAColor`; `2a9a18e3...`):
`(ambient + sunDiffuse * max(0, N . dir)) * D*D`, fogged. This is what ours drew everywhere.

**Two reader bugs this turned up** (both fixed here):

1. `Material::lightmap` was hard-coded `None` on the Vita, so no atlas was ever bound
   (no `lightmap atlas(es) bound` report line at all on Vineta K, though its floor materials name
   `nova_mesh_combine_track_N-lmap.gxt`). It is now the sampler table's
   `LIGHTMAP_SAMPLER` entry: 20 atlases bound on Vineta K.
2. `SubMesh::lightmap_texcoords` filtered on the PS4's half type (`1`), so no Vita submesh ever
   yielded a lightmap coordinate; the Vita's half type is `8`. Fixed (`half_type`).

**What changed in the picture** (ours only, three 2048 race pairs at 960x544, `--hold-exposure
1.15`; boxes `x0,y0,x1,y1`):

| Region | Original | Before | After |
| --- | --- | --- | --- |
| Vineta K floor `(0,330,960,544)` | 32/38/43 | 29/28/28 | 24/23/22 |
| Vineta K right wall `(600,140,960,330)` | 69/55/50 | 56/43/44 | 46/36/33 |
| Empire Climb road `(150,250,800,544)` | 141/111/88 | 116/99/84 | 111/92/73 |
| Mall, mean grey distance (lower is nearer) | - | 25.92 | 23.51 |
| Vineta K, mean grey distance | - | 23.29 | 20.64 |
| Empire Climb, mean grey distance | - | 32.16 | 25.94 |

The wall's large baked roof shadow now appears (it was absent: the atlas was unbound). The mean
grey distance to the original fell on all three pairs, the colour targets (floor B-R >= +8, road
R >= 130) were **not** met: ours is now darker than the original. Left over, none of it read yet:
the floor's blue panels are an **emissive** layer (`diffuse_normal_specular_emmissive`: `DiffuseMap`,
`EmissiveMap`, `lightmap`) that ours does not draw on this family; `Lighting.Exposure*`/`BloomFactor`
act on the composite only; the floor's own normal map and `shadowMap`/`occlusionMap` terms.

**Against Omega:** the lightmapped law is *ported* (shared `with_nova_prelit` path, power held at
1 for 2048). The vertex-lit `NOVAColor` family is wired for 2048 only (`slots`: `sun_mask >= 2`
marker in `shade.wesl`); Omega's `NOVAColor` variant stays *checked, applies, not wired* (its
declaration reader differs, type nibble `11` is the PS4 colour, and the 2048 filter excludes it).

## The emissive add of the `*_emmissive_alpha` family (2026-10-31, `2048-floor-emissive`)

**The lane's premise was off by one name.** The floor panels of Vineta K are not the
`diffuse_normal_specular_emmissive` programs with an `EmissiveMap`, and not the lightmapped
`diffuse_normal_specular` floor (`ds_floorplain_*`, which declares no emissive input at all). They are
`diffuse_normal_specular_emmissive_alpha` (`ds_wall_cs`, `ds_wall_n`, `ds_wall_a`: 17 + 11 + 10 submeshes
with a blue `Constant1`), opaque (state 124, mode `Opaque`, no `OpKill` in the program). Method as in the
section above (Vita3K's cached SPIR-V by container SHA-256, walked symbolically; handles mapped to
bindings through their `OpLoad %fragTex_*`). Conf. in brackets.

**Isolation, the picture.** A temporary needle on the material name (not kept) hid one family at a time
on the `dlc-vineta-k` pair frame (960x544, tick 179, exposure held 1.15):

| Hidden | Floor box `(0,330,960,544)` mean | What shows |
| --- | --- | --- |
| nothing (main `84ba82deb`) | 24/23/22 | black panels in a grey grid |
| `diffuse_normal_specular_emmissive*` | - | a large blue-teal surface underneath, where the original draws its blue panels [90] |
| `fc06_lambert_emmissive*` | unchanged | the floor is not this one |
| `fc17_phong*`, `diffuse_normal_specular` | 24.7/24.6/23.2, 24.3/25.2/24.9 | no panel change |

So ours draws the panel opaque and black over a blue layer; the original's panel is itself blue.

**The law** (program `f7b139cf...`, `diffuse_normal_specular_emmissive_alpha`, binding order `Texture1`,
`Texture2`, `Texture2Alpha`, `lightmap`, `occlusionMap`, `shadowMap`) [80]:

```text
lit  = the lightmapped combine of the section above (L = lightmap, D = Texture1, N from Texture2)
rgb  = mix(fogColour, spec + Texture2Alpha.rgb^2 * Constant1 + lit * D*D, fogVisibility)
```

`Texture2Alpha` is `0x21745d36` (`~crc32("Texture2Alpha")`, verified); `Constant1` is `0x7611a2d8`
(`~crc32("Constant1")`, the same hash the HD pad doc names). The emissive term sits **outside** the
lightmap product: it is neither lit nor shadowed, so a dark baked panel still glows. `Constant1` is
`(0.0, 0.7686, 0.9922)` on the 38 blue submeshes (the panel colour) and `0.9912` grey on the `_lh`
variants. The `lightmap` alpha times `Texture1.a` (gloss) gates the `liveLighting0specular` term,
unchanged from the section above.

**Its sibling, `fc06_lambert_emmissive_alpha`** (program `d8bd89a9...`, `DiffuseMap` + `lightmap`, uniform
`Tint`, `0x71c4bbb5`) [80]:

```text
rgb = mix(fogColour, Tint * D.a + prelit_sun * D*D, fogVisibility)
```

`D.a` is the glow mask here (no second texture), the add again outside the light. Vineta K authors 23 tints
such as `1.0 0.858 0.604` (`and_tunnellights`), `0.008 0.807 0.930` (`j_lampost`).

**Wired (2026-10-31, `2048-emissive-wire`), and the floor was the plain family after all.** Painting
each family's add a solid red on the `dlc-vineta-k` pair frame showed:

- the `*_emmissive_alpha` family (`ds_wall_cs`/`ds_wall_n`/`ds_wall_a`) covers the **side walls** and
  nothing on the floor; its add (mode 8, `Texture2Alpha.rgb^2 * Constant1`) is wired and draws, but the
  floor box does not move (24/23/22 -> 24/24/23);
- the plain `diffuse_normal_specular_emmissive` (13 slots, techs `Track_Surface_New3`...) is exactly the
  panel shape in the original's floor. Its programs (`a17f5c52...` vertex-lit, `419db654...` lightmapped,
  `Constant1`, `Texture1`, `Texture2`, the same symbolic walk) add **`Texture2.a * Constant1`**, the
  normal map's alpha as the mask, **unsquared**, outside the lit terms and under the fog mix [70: the
  lane order of `Constant1`'s packed halves is read, not tested against a capture; the picture decides it].
  Mode 9.

| `dlc-vineta-k` (960x544, held exposure 1.15) | Original | Main `b7b08dd80` | Wired |
| --- | --- | --- | --- |
| floor `(0,330,960,544)` mean R/G/B | 32/38/43 (B-R +11) | 24/23/22 | 26/31/37 (B-R +11) |
| right wall `(600,140,960,330)` | 69/55/50 | 46/36/33 | 49/39/36 |
| mean-grey distance | - | 20.64 | 18.66 |
| Empire Climb, Queens Mall distance | - | 25.94, 23.51 | 25.94, 23.51 (unchanged) |

Wiring: `mesh::rcs::psp2::emissive_add` (`Texture2`/`Texture2Alpha` in `Model::pad_masks`, the tint
(`Constant1`) in a glow-table entry of `PROGRAM_EMISSIVE_ADD`/`PROGRAM_EMISSIVE_ALPHA_MASK`, the vertex's
material-index bits), added to `shade.wesl`'s `pad_glow`. Vita only. `fc06_lambert_emmissive_alpha`'s
`Tint * D.a` is **not wired** (unchanged: read, open). The add is dimmer than the original's (floor G/B
31/37 against 38/43) with the rest of the frame, the open darkness item.

**Against Omega: checked, differs, not wired.** Omega's `diffuse_normal_specular_emmissive` pixel programs
(`data01.psarc`, `0x6bcbd09c`: `Constant1`, `novaScaleBiasPower`, `Texture1Sampler`, `lightmapSampler`,
`shadowMapSampler`) sample **one** diffuse map and name no `Texture2` (normal) or `Texture2Alpha`
sampler, so the mask source is not 2048's `Texture2.a`; the GCN body is unread [conf 60]. Not the same
few lines; left to the generic reading.

(`diffuse_normal_specular_emmissive`, `fc06_lambert_emmissive_alpha`: 63 and 33 entries per variant over
its packages), but its GCN programs were not read here, so whether Omega applies the same
outside-the-light add is open. `diffuse_normal_specular_emmissivenew` (Chenghou, 7 slots) is the plain member with `Norm` for
`Texture2` and the colour named `Texture1`: read and wired 2026-11-11 (`gxp.md`,
"`diffuse_normal_specular_emmissivenew` colours by `Texture1`"). Omega's plain member is wired the
same way (its ambient GCN program read; **ported**).
