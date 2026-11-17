# Water, title by title

How each original draws water, which circuits carry it, and what this engine
draws of it today. Measured 2026-11-17 (lane `water-re`) off the disc data and
the shipped shader programs; **no emulator frame was taken in this lane**, so
nothing here is in the rubric's runtime band. Earlier readings this page builds
on: [rcsmaterial.md](../formats/rcsmaterial.md) ("Water family", "Vineta K
against a draw capture"), [hd-track-tone.md](hd-track-tone.md) ("The reflection
map is the sky"), [the 2048 pass page](../ghidra/functions/vita-2048-eu-v104/refraction-pass.md),
[gxp.md](../formats/gxp.md).

## Summary

| Title | What water is | Animation | Reflection | Confidence |
| --- | --- | --- | --- | ---: |
| Pulse PSP (disc) | textured meshes (`NewSeaTest`, `sea`, `water`, `NWaterfall*`) | authored texture-offset tracks where a material has one, not surveyed per surface | none | 85 |
| Pulse, Vostok Reef (PS2 disc, PSP download) | the above plus a **caustic pass**: flagged batches redrawn with a ripple texture projected from above | 16 frames at 30 fps | none | 75 (PSP code), 90 (which data) |
| Pure | textured meshes; Vineta K's `.vex` embeds 16 caustic frames | not read | none | 60 |
| HD/Fury sea (`reflectplane_dc_seawater`) | flat colour, two wave-normal taps, sun diffuse and specular, **planar reflection** | taps scroll `u + 0.05 t` and `-u + 0.05 t` | `screenSpaceReflectionTex` through `reflectProject`, fed by chunks with render flag bit 3 | 85 (program), 88 (which chunks) |
| HD Vineta K (`water_test_2`, `water_noref`) | vertex colour times light, three wave taps | three scales and rates per material | `paraboloidReflectionTex` (the sky) | 90 |
| HD Amphiseum (`water`) | flat colour, two wave taps | as the sea, in the fragment program | `paraboloidReflectionTex` | 85 |
| 2048 own circuits (`fc15`/`fc17_effects_vscroll_water`) | flat colour, **one** wave normal map, sun diffuse and specular | `u + 0.05 t` | none | 80 |
| 2048 HD circuits | HD's programs, recompiled | as HD | Moa Therma and Ubermall: planar | 80 |
| Omega 2048 circuits (`reflectplane_mid`, `_edge`, `_pond`, ...) | two wave normal maps, sun specular, Fresnel-weighted **planar reflection** | `Speed` parameter (0.5 on Altima) | `screenSpaceReflectionTex`, fed by `wohdmeshreflect` meshes | 75 |
| Omega HD circuits | HD's materials | as HD | as HD; Amphiseum reads a cube instead | 75 |

The maintainer's from-play description ("HD has a simple wavy effect") is the
two scrolled normal taps; the mirrored scenery in Moa Therma's sea is the
planar reflection.

## Census

Method: every circuit model of each title dumped by material
(`cargo run -p oag-rcs --example hd_material_dump -- <entry> <DATA0n.PSARC>...`
for HD, `--example psp2_material_dump -- <entry> 0,0,0 <archives>...` for 2048
and Omega), filtered for `water|waves2|ocean|ripple|lake|river|pond|sebenco_ice|
caustic|watersea|reflectplane` in the material path or its texture names. A
sea whose material and textures carry none of those words is not in it.
PSP and PS2: `water_caustic_sweep` (`crates/vex/tests`), by embedded texture
name and by the caustic bit. Confidence 90 for what is listed, 70 for
completeness.

### HD/Fury (28 circuit models, forward and reverse)

| Circuit | Material | Chunks / triangles | Textures |
| --- | --- | --- | --- |
| Vineta K | `water_test_2` | 2 / 533 | `waves2` |
| | `water_noref` | 3 / 516 | `waves2` twice, `skyreflect` (unread by the program) |
| | `reflectplane_dc_seawateredging` | 34 / 1,768 | `and_waterfoam_blend`, `dc_watergrad_blend`, `dc_waterdiffuse` |
| | `cf_waterfall`, `and_waterfall` | 3 / 220, 4 / 132 | `martin_waterfallspray_alphablend`, `and_bubbles` |
| Moa Therma (`03_track`) | `reflectplane_dc_seawater` | 31 / 474 | `dc_waternormalmap` twice |
| | `reflectplane_dc_seawateredging` | 228 / 5,855 | `dc_waterdiffuse`, `dc_watergrad_alphablend`, `dc_waternormalmap` |
| | `mar_diffuse_specular` | 1 / 216 | `and_water1` (a picture) |
| Chenghou Project | `reflectplane_dc_seawater` | 3 / 78 | `dc_waternormalmap` twice |
| Ubermall | `reflectplane_dc_seawater` | 13 / 2,421 (reverse 7 / 1,899) | `dc_waternormalmap` twice |
| | `mr_waterfall` | 13 / 999 | `martin_*waterfall*`, `and_waterhighlights_add` |
| Sebenco Climb | `sebenco_ice` | 4 / 4,912 | six, see rcsmaterial.md |
| Amphiseum | `water` | 2 / 229 | `dc_waternormalmap` twice, `dc_waterreflection` (unread by the program) |
| Modesto Heights | `reflectplane_dc_seawater` | 5 / 483 | `dc_waternormalmap` twice |

None on Metropia (`02_track`), Sol 2, Anulpha Pass, Talon's Junction, Tech De
Ra or the four Zone circuits.

### 2048 (Vita, 48 circuit models over base, patch and both DLC archives)

| Circuit | Material | Submeshes / triangles | Textures |
| --- | --- | --- | --- |
| Altima | `fc15_effects_vscroll_water` | 5 / 224 | `water_lapping_waves2_n` |
| | `fc17_effects_vscroll_wateredge_vertexcol` | 5 / 815 | `water_lapping_waves2_n` |
| | `fc06_effects_vscroll_emissive_alpha` (waterfalls) | 44 / 1,636 | `waterfalltemp`, `waterfalltemp02` |
| Park (Unity Square's park) | `fc17_effects_vscroll_water` | 4 / 390 | `water_lapping_waves2_n` |
| Mall | `fc15_effects_vscroll_water` 3 / 51; `fc06_lambert_alpha` 22 / 391; `fc06_lambert_simple` 5 / 132 | | `waternormalmap`; `water_surf`; `waternospec` |
| Tower | `fc15_effects_vscroll_water` 4 / 51; `fc06_lambert_alpha` 23 / 397; `fc12_phong_simple` 3 / 32 | | `waternormalmap`; `water_surf`; `water_pool` |
| Square | `fc15_effects_vscroll_water` 4 / 122 | | `dc_waternormalmap` |
| Bridge | `fc01_effects_vscroll_emissive` 13 / 219 | | `watersea` (a scrolled picture) |
| Arena, Cathedral, Subway | plain lambert or emissive on `watersea`, `wateremissive`, `dc_waterdiffuse` | 16 to 540 triangles | pictures |
| Vineta K (DLC) | `dc_seawater` 5 / 716; `reflectplane_dc_seawateredging` 1 / 50; waterfalls | | `waves2` |
| Moa Therma (DLC) | `reflectplane_dc_seawater` 19 / 438; `dc_seawater_cheap` 15 / 36 | | `dc_waternormalmap` |
| Ubermall (DLC) | `reflectplane_dc_seawater` 16 / 655; `ref_dc_seawater_noref-nospec` 1 / 200; `mr_waterfall` | | `dc_waternormalmap` |
| Chenghou Project (DLC) | `reflectplane_dc_seawater` 3 / 43 | | `dc_waternormalmap` |
| Modesto Heights (DLC) | `fc06_lambert_simple` 6 / 485 | | `dc_waternormalmap` |

No hit on Sol, the Vita's Amphiseum, Metropia, Sebenco Climb, Sol 2, Anulpha
Pass, Talon's Junction, Tech De Ra or the Zone models other than those listed.

### Omega (PS4, 48 circuit models)

HD mode ships HD's materials: Vineta K `water_test_2_castnoshipshadows` 2 / 533,
`temp_water_castnoshipshadows` 3 / 684 (`waternormalmap1`, where HD has
`water_noref`), `water_edge_temp` 34 / 1,768; Moa Therma
`reflectplane_dc_seawater` 31 / 474 and `...edging` 213 / 5,241; Chenghou
`reflectplane_dc_seawater` and `_new`; Ubermall 13 / 2,421; Modesto 4 / 434;
Amphiseum `water` 2 / 229; Sebenco `rt_snow_ref` (the pond, where HD has
`sebenco_ice`; Omega ships `Sebenco_ice` files and no submesh of the model uses them).

2048 mode **replaces 2048's water materials** on the same meshes:

| Circuit | Omega material (2048's on the Vita) | Submeshes / triangles |
| --- | --- | --- |
| Altima | `reflectplane_mid` (`fc15_effects_vscroll_water`) | 5 / 224 |
| | `reflectplane_edge` (`fc17_..._wateredge_vertexcol`) | 5 / 815 |
| | `reflectplane_rt_seawateredging` (new) | 1 / 240 |
| | `waterfall_new`, `waterfall_new_start` (with `_nrm` normal maps) | 32 / 3,181 |
| Park | `reflectplane_dc_seawater` (`fc17_effects_vscroll_water`) | 4 / 390 |
| Mall | `reflectplane_dc_seawater_new` 8 / 183; `reflectplane_rt_seawateredging` 22 / 999 (`fc06_lambert_alpha` on `water_surf`) | |
| Tower | `reflectplane_dc_seawater_tow` 5 / 130; `reflectplane_rt_seawateredging` 23 / 2,325 | |
| Square | `reflectplane_pond` 7 / 205 | |
| Bridge | `reflectplane_dc_seawater_new` 13 / 219 (the Vita's scrolled `watersea` picture) | |

Omega's `trackZone` models keep the Vita's materials
(`fc17_effects_vscroll_wateredge_vertexcol`, `fc06_lambert_alpha`).

### Pulse (PSP EU disc, PS2 EU disc)

By embedded texture name, PSP `Data.wad`: `03_Track` (Moa Therma) `NewSeaTest`,
`WaterFoam`, and in its later variants `GenericTrackTextures\NWaterfall1`/`2`;
`06_Track` `sea`, `AAdc_waterFoam`; `16_Track` `water`; `14_Track` Zone
`rich_sea_GLOW`. PS2 `.vex` files embed no texture names, so the PS2 disc is
read by the caustic bit alone: Vostok Reef (`08_Track`), three files, 205 to
257 batches each. No PSP disc batch carries the bit.

The engine's class table has `sea` (`0x3d6`), `seareflect` (`0x3d7`) and
`seaweed` (`0x3d5`) ([vex-classes](../ghidra/functions/ps3-hdfury-eu/vex-classes.md));
whether any shipped `.vex` holds a node of those classes, and what the PSP does
with one, was **not checked** in this lane.

### Pure (PSP EU disc)

By embedded texture name: Vineta K `AAdc_Water`, `AAdc_WaterDark`,
`AAdc_waterFoam`, `AAdc_Seabed*`, plus **16 caustic frames**
(`Z:\data\Tex\caustics\save.NN.tga`, the odd numbers 01 to 31); Blue Ridge
`Water`, `Waterfall_Ani`, `Waterfall_Ani_Noalpha`, `Waterfall_static`,
`WaterTunnelLightA_BLEND_GLOW`. The sweep's batch walk does not decode Pure's
version-4 batches, so the caustic bit is unread on Pure. A GE list already
showed Vineta's layered water plane at `y = -50` drawn with material ambient
`(107, 216, 255)` and `(128, 128, 128)`
([hull-shine.md](../ghidra/functions/psp-pure-usa/hull-shine.md)).

## How it is drawn

### Pulse: textures, and a caustic pass on Vostok Reef

Water surfaces are ordinary textured batches. The one effect is the caustic
pass: a batch with header byte 3 bit `0x04` is redrawn with frame
`(int)(track_time * 30) & 0xf` of `Data\Tex\caustics\save.NN.mip`, texture
coordinates generated by a texture matrix `scale(0.014) * quarter turn * node
world` (one tile per 71.4 units, projected from above), ambient tint
`(0.2, 1.0, 0.8)` times a per-mesh byte, depth equal. Functions, constants and
confidence: [caustic-pass.md](../ghidra/functions/psp-pulse-usa/caustic-pass.md).

### Pure: not read

Hypothesis, confidence 45, no rename: Pure's Vineta K plays the 16 embedded
caustic frames as a texture swap on its tunnel floor, the mechanism Pulse later
moved into a global pass. What to read: the frames' `Texture` nodes in
`01_Vineta_K`'s `.vex` (which material binds them, and whether an
`Anim Transform` or a frame index drives the swap), then a PPSSPP GE list in
the tunnel.

### HD/Fury: `reflectplane_dc_seawater`

Read off `scripts/ps3-microcode.py vp-file` / `fp-file` of
`03_track/materials/reflectplane_dc_seawater.rcsmaterial` (vertex block 3,
fragment blocks 2 and 3; confidence 85, the constant slots follow
`c[n] = c(n + 256)`, which `time` at `c464` against `c[208]` confirms).

```text
vertex:   A = (u + 0.05 t, v)          B = (-u + 0.05 t, v)        t = engine `time`
fragment: n1 = 2 tex(DiffuseTexture,  A).ag - 1
          n2 = 2 tex(DiffuseTexture1, 1.15 B).ag - 1
          N  = normalize((n1.x + n2.x) T + (n1.y + n2.y) B + (z1 + z2) Nv)
          P' = (P.x + distortion.z N.x,  P.y,  P.z + distortion.z N.z)
          q  = reflectProject * (P', 1)
          R  = tex(screenSpaceReflectionTex, (0.5 + q.x / 2 q.w, 0.5 - q.y / 2 q.w))
          out = fog( (constantAmbientColour + sun sat(N.L)) * ColourConstant
                     + sun * SpecularColour * sat(N.H)^SpecularPower * sat(8 N.L)
                     + Reflectivity * R ) / 2
```

`DiffuseTexture` (`0x11cb4f74`) and `DiffuseTexture1` (`0x06a77e50`) both bind
`dc_waternormalmap` (a two-channel normal, x in alpha, y in green).
`ColourConstant` `0x4d60d566`, `Reflectivity` `0x5e5b1937`, `SpecularColour`
`0x370a63cb`, `SpecularPower` `0x81e0e773`, `distortion` `0x9fc59444`,
`reflectProject` `0xac608eb9`, `screenSpaceReflectionTex` `0xec2b3fc2` are
exact `~crc32` preimages (the last three are in the engine's own parameter
table, [renderer.md](../ghidra/functions/ps3-hdfury-eu/renderer.md)).

Authored values:

| Circuit | `ColourConstant` | `Reflectivity` | `SpecularColour` | `SpecularPower` |
| --- | --- | ---: | --- | ---: |
| Moa Therma | `(0.0799, 0.1257, 0.128)` | 0.3 | `(0.521, 0.507, 0.300)` | 75 |
| Ubermall | `(0.0206, 0.166, 0.1505)`, one slot `(0, 0.153, 0.198)` | 0.36 | `(0.753, 0.910, 1.0)`, `(0.835, 0.659, 0.536)` | 52.8, 52 |
| Chenghou, Modesto | `(0.0206, 0.166, 0.1505)` | 0.36 | `(0.753, 0.910, 1.0)` | 52.8 |

`distortion` and `reflectProject` are engine-supplied and not authored.

**What feeds the reflection target (confidence 88).** Chunk render flag bit 3
(`0x08`) is set on 257 chunks of Moa Therma and 73 of Ubermall and on no other
HD circuit; they are scenery (blimps, buildings, glass, `lambertzeroalpha`).
`Scene_SortAndLightVisibleChunks` appends such a chunk to draw bucket B2 as
well as its ordinary one
([zone-effectsettings-loader.md](../ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md)),
and B2's consumer `FUN_00403a30` has the same body as B3's, which is the
measured behind-the-glass target pass
([visibility.md](../ghidra/functions/ps3-hdfury-eu/visibility.md)). Omega's
copies of the same two circuits tag **268 and 73** mesh objects
`wohdmeshreflect`, the name 2048 and Omega turn into instance flag `0x10`, the
flag their pass kind 1 draws. Ubermall agrees to the mesh; that is the second
binary. Render flag bit 1 (`0x02`) is set on exactly the 30 Moa Therma chunks
of `reflectplane_dc_seawater` and nowhere else on the disc; the sorter does not
consult it. It probably marks the mirror surface (confidence 50, not named).

**Not read:** the reflection camera (which plane it mirrors about, the target's
size and clear), `distortion`'s value, and what the target holds on Chenghou,
Modesto and Ubermall's reverse layout, which have the sea material and no
bit-3 chunk beyond those counted. One RPCS3 frame on Moa Therma's sea answers
all three ([rpcs3-capture.md](../reverse-engineering/rpcs3-capture.md),
"Capturing one frame's draws").

`reflectplane_dc_seawateredging` (the shore strip) is a blended picture
(`dc_waterdiffuse` or `and_waterfoam_blend`), an alpha gradient
(`dc_watergrad_*blend`) and a normal map, with `SpecularColour` and
`SpecularPower` 65. Its program was disassembled and not taken apart.

### HD/Fury: Vineta K and Amphiseum

`water_test_2` and `water_noref` are on rcsmaterial.md. New here, the vertex
scroll (vertex block 3, confidence 85): with `s = 0.05 t`, three taps at
`uv * scale_i + s * rate_i`:

| Material | scales (`0xf4bc8b2f`, `0x6db5da95`, `0x1ab2ea03`) | rates (`0x02567724`, `0x9b5f269e`, `0xec581608`) |
| --- | --- | --- |
| `water_test_2` | 1, 2, 4 | 1, 2, 1.2914 |
| `water_noref` | 1.4, 2.14, 7.164 | 0.3, 0.2, 0.1 |

Amphiseum's `water` (fragment block 2, confidence 85) builds the same two taps
as the sea, in the fragment program (`u + 0.05 t` and `1.15 (-u + 0.05 t)`),
and reflects the sky instead of a planar target:

```text
out = (light + constantAmbientColour) * ColourConstant
      + Reflectivity * probeTint(tex(paraboloidReflectionTex, paraboloid(reflect(-E, N))))
```

`ColourConstant` `(0.0178, 0.0459, 0.055)`, `Reflectivity` 0.7. `probeTint` is
the `iblScalePower` law the window glass already uses (hd-track-tone.md).

### 2048 (Vita)

`fc15_effects_vscroll_water`, programs `#2` (vertex) and `#3` (fragment) of
the Altima material (`scripts/vita-gxp.py dump`, `scripts/vita-usse.py disasm`):
the fragment program declares `NormalMap`, `ColourConstant`, `SpecularPower`,
`constantAmbientColour`, the three `liveLighting0*` uniforms and `fogColour`,
and **no reflection sampler**. The vertex program multiplies `time` by a
literal and adds it to `uv1.x` (secondary instruction 2, primary 31); the
program's literal block holds `0.05`. So: one normal map scrolled `u + 0.05 t`,
sun diffuse and specular over a constant colour. Altima authors `ColourConstant`
`(0.133, 0.173, 0.219)`, `SpecularPower` 250. Confidence 80: the literal's
register is matched by position, not traced. `fc17_..._wateredge_vertexcol`
adds `Speed`, `Opacity`, `Watercolour`, `SpecularColour` and a vertex colour.

Of the HD circuits' materials on the Vita, `reflectplane_dc_seawater` declares
`screenSpaceReflectionTex` and `time` (34 programs each), `dc_seawater`
(Vineta K) `time` only, `dc_seawater_cheap` neither. Moa Therma's Vita model
tags 353 mesh objects `wohdmeshreflect`, Ubermall's 52, Altima's none. The
target those feed is the half-size one `Render_GetRefractionTexture` returns,
bound to both `screenSpaceReflectionTex` and `screenSpaceRefractionTex`
(the 2048 pass page). **Checked against Omega: differs** on 2048's own
circuits (below), **applies** on the HD circuits (same material names and the
same reflect tag).

### Omega (PS4)

`crates/rcs --example gcn_dump` on the materials' GCN programs (declared
interface only, no instruction read; confidence 75):

- `reflectplane_mid` (Altima): `NormalMap`, `NormalMap1`,
  `screenSpaceReflectionTex`, `reflectProject`, `distortion`, `time`, `Speed`,
  `Watercolour`, `SpecularColour`, `SpecularPower`, and four Fresnel-shaped
  weights `ReflectivityMin`, `ReflectivityPower`, `ReflectivityScale`,
  `ReflectivityStrength`. Authored on Altima: `Watercolour` `(0.0546, 0.0546,
  0.0546)`, `SpecularColour` `(0.673, 0.659, 0.470)`, `SpecularPower` 250,
  `Speed` 0.5, `ReflectivityMin` 0, `ReflectivityPower` 4, `ReflectivityScale`
  1, `ReflectivityStrength` 0.8. How the four combine is unread.
- `reflectplane_dc_seawater` (Moa Therma): HD's interface exactly
  (`DiffuseTexture`, `DiffuseTexture1`, `screenSpaceReflectionTex`,
  `reflectProject`, `distortion`, `ColourConstant`, `Reflectivity`,
  `SpecularColour`, `SpecularPower`, `time`). **Checked against HD: applies.**
- `water` (Amphiseum): `NormalTexture`, `NormalTexture2`, `ColourConstant`,
  `Reflectivity`, `time`, and `cubeReflectionTex` where HD reads the
  paraboloid. **Checked against HD: differs** in the probe.
- `wohdmeshreflect` mesh objects: Altima 30, Mall 38, Tower 45, Park 24, Moa
  Therma 268, Ubermall 73. Omega gives 2048's circuits the planar reflection
  the Vita does not have. **Checked against 2048: differs.**

**Not checkable here:** a live Omega value (`distortion`, the reflection
camera); the executable is in Ghidra and `TrackPass_FilterSubmesh` kind 1 is
the place to start.

## What this engine draws today

| Surface | Today | Missing |
| --- | --- | --- |
| Pulse, Pure water textures | the texture, with any authored texture-offset track | nothing known |
| Pulse Vostok Reef caustics | nothing | the whole pass |
| HD `water_test_2`, `water_noref` | lit vertex colour, or black plus fog (`mesh::rcs::water`) | the wave normals, their scroll and the sky reflection |
| HD `sebenco_ice` | the three-colour lerp (`mesh::rcs::ice`) | bump-perturbed facing, own specular |
| HD `reflectplane_dc_seawater` | the generic reading; `mesh::rcs::emissive` refuses its normal map as a glow (`CIRCUIT_SURFACE_MAP_SAMPLERS`). No water-specific path, and not judged on a frame in this lane | the colour constant as the law states it, both taps, the scroll, specular, the planar reflection |
| HD `water` (Amphiseum) | generic | the taps and the sky reflection |
| 2048 and Omega seas | the constant colour times light, flat (`psp2::lighting_only`, `gxp::sampler_use`) | normals, scroll, specular, reflection |
| Refraction target (Vineta K glass) | drawn on HD, 2048 and Omega | n/a; it is the pattern the reflection target follows |

What already exists and the water can reuse: the sky as a dual paraboloid with
`probeTint` (`mesh::sky_paraboloid`, `window_glass`), the behind-glass target
and its `refractProject` (`mesh_render::uniforms`, `oag_raceplay`'s
`scene::behind_glass`), HD's vertex-side scroll as `AnimTrack::Scroll`
(`mesh::rcs::vertex_scroll`), `Mesh::render_flags`, and the 2048 name-tag split
(`psp2::refraction_only`).

## Lineage, per finding

| Finding | 2048 | Omega | HD |
| --- | --- | --- | --- |
| Sea law (two taps, planar reflection) | checked, applies on its HD circuits, not wired | checked, applies, not wired | measured here |
| Reflect set (bit 3 / `wohdmeshreflect`) | checked, applies (name tag, 353 and 52), not wired | checked, applies (268 and 73), not wired | measured here |
| 2048 own water (`fc15`/`fc17`) | measured here | **checked, differs**: replaced by `reflectplane_*` | not applicable |
| Amphiseum `water` | not checked (the Vita's Amphiseum shows no water material by name) | checked, differs (cube probe) | measured here |
| Vineta K scroll table | checked, differs (`dc_seawater`) | checked, applies (`water_test_2_castnoshipshadows`), `water_noref` replaced by `temp_water`, not wired | measured here |
| Pulse caustic pass | not applicable | not applicable | HD ships `data/tex/caustics/` and `wo_underwater_godrays.pob`; consumer not looked for |

## Open

1. One RPCS3 frame at Moa Therma's sea: the reflection target's size, camera
   and contents, `distortion`, and whether bit 1 marks the mirror.
2. Vostok Reef on PCSX2 or PPSSPP (with the download pack): a frame of the
   caustic pass, and batch state `0x202`'s blend.
3. Pure's caustic frames on Vineta K.
4. Omega's `Reflectivity*` combine and `Speed` law (a GCN instruction read),
   and 2048's `fc17` edge program.
5. `sea`, `seareflect`, `seaweed` class nodes: a class census over PSP `.vex`.
