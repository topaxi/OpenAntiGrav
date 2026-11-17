# Water, title by title

How each original draws water, which circuits carry it, and what this engine
draws of it today. Measured 2026-11-17 (lane `water-re`) off the disc data and
the shipped shader programs; the HD planar reflection's target, camera and constants
were read off live RPCS3 frames the same day (lane `water-hdcap`, "The planar
reflection, read off live frames"), and are the only runtime-band numbers on
this page. Earlier readings this page builds
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
| HD/Fury sea (`reflectplane_dc_seawater`) | flat colour, two wave-normal taps, sun diffuse and specular, **planar reflection** | taps scroll `u + 0.05 t` and `-u + 0.05 t` | `screenSpaceReflectionTex` through `reflectProject`, fed by chunks with render flag bit 3: a 960 x 540 R5G6B5 target drawn under the main camera mirrored about y = -25 (Moa Therma, read live) | 85 (program), 88 (which chunks), 90 (target, camera, constants) |
| HD Vineta K (`water_test_2`, `water_noref`) | vertex colour times light, three wave taps | three scales and rates per material | `paraboloidReflectionTex` (the sky) | 90 |
| HD Amphiseum (`water`) | flat colour, two wave taps | as the sea, in the fragment program | `paraboloidReflectionTex` | 85 |
| 2048 own circuits (`fc15`/`fc17_effects_vscroll_water`) | flat colour, **one** wave normal map (RGB, through the tangent frame), sun diffuse and Phong specular weighted by the map's alpha | `u + 0.05 t` (`fc15`); `v + Speed t` (`fc17`) | none | 85 (`fc15`), 85 (`fc17` colour, specular, scroll, alpha chain), 75 (which lanes feed it, below) |
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

**Drawn** on the PS2 disc's Vostok Reef under `original` (`mesh::caustic_pass`; test
`caustic_pass_ground_truth` in `oag-vex` and `oag-game`): the floor and walls of the halls show
the moving ripple web, `off` draws the plain surface. The PS2 frames are `save.NN.pct`; the
PSP disc has no flagged batch. Lineage: Pulse-only, not applicable to 2048/Omega; HD's
`data/tex/caustics/` consumer is not looked for.

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

**The frame and the light, read 2026-11-17 (`water-hd`, confidence 85).** The tangent is
the chunk's own attribute `0xdbe5f417` (four unsigned bytes, decoded `2x - 1`), and the
vertex block builds `B = cross(N, T) * w` with `w` raw (`MUL o[TC0].xyz, N x T, v[2].w`),
the frame `oag_mesh` calls `TangentFrame::Raw`. Each tap's `z` is rebuilt as
`sqrt(1 - x^2 - y^2)` (`RSQ`/`RCP`, `@0x0c`..`@0x1b`). The lightmapped variant (Moa
Therma's slots, block `@0x3560`) lights by the bake (`pow(lightmap, 0x002c73e8) *
0x8670f0be`) plus `sun * sat(N.L)` in place of `constantAmbientColour`, and its specular
carries the bake's alpha as the sun mask, the generic HD lightmapped light.

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

**Read off live frames, 2026-11-17 (`water-hdcap`; the section below).** The reflection
camera, the target's size and clear, `distortion` and `reflectProject` are measured on
Moa Therma; Chenghou's and Modesto's seas are not reached ("What is still unread").

### The planar reflection, read off live frames (2026-11-17, `water-hdcap`)

**Claim, and what would have falsified it.** The sea's `screenSpaceReflectionTex` is a
second render target the frame draws before the main view, with the bit-3 scenery and the
sky, under a camera that is the main one mirrored about a horizontal plane (a Householder
reflection, determinant -1; the identity would have meant a screen-space grab); the sea
program's `reflectProject` is the matrix that maps a world point to the main view's
screen, so it samples the picture where the point itself appears; and no other material
samples the target. Each was falsifiable on one frame, and none failed.

**Method.** RPCS3 0.0.43, `Write Color Buffers` on, Resolution Scale 100, audio renderer
`"Null"`, Time Trial, Moa Therma (`03_Track`), standard Harimau, one cold boot walked
to the grid (`scripts/emu-hd-timetrial-state.sh <lane> 2 9 0 <name>`) and saved, then
restored twice. In each restore the craft was placed over the sea with
`scripts/hd-sea-place-dump.py` (`x y z` = `210 38 -380` in two frames and `150 38 -400` in
one, facing `-z`; the camera then sits about 64 above the water) and one frame's draws dumped with
`HOOK_LIGHT=1 HOOK_NO_HEADS=1 HOOK_FP_LEN=0x900`
([rpcs3-capture.md](../reverse-engineering/rpcs3-capture.md), "Capturing one frame's
draws"); `scripts/hd-sea-reflect-read.py` prints every number below. A draw is named by
`rsx_fp_match.py` against the circuit's `.rcsmaterial` files, so the sea is
`reflectplane_dc_seawater#5`/`#8`. Two traps: the sea program is 79 to 86 instructions
and the hook's default 0x600-byte cut hides it (`HOOK_FP_LEN=0x900`), and a restored
state's command stream runs past the hook's default seed (`RING_DUMP=0x40000000:0x120000`,
else "0 draws"). Three Moa frames, two restores of one cold boot (two poses, two memory
layouts, not three cold boots); the Chenghou frames below are a second cold boot of a
second circuit.

| Quantity | Value, every frame | Confidence |
| --- | --- | ---: |
| The target | one surface the frame draws into before the main view: colour at VRAM offset `0x02a20000` (the offset moved by no frame here), depth `0x02b40000` | 90 |
| Size and format | **960 x 540**, `SET_SURFACE_FORMAT` `0x123` = R5G6B5 colour, Z16 depth, pitch `0x800`; clip, scissor and viewport are all `0x3c0 x 0x21c`, viewport offset `(480, 270)` and scale `(480, -270)` (the same sign pattern as the main view's `(650, -540)`) | 90 |
| Clear | `CLEAR_SURFACE` mask `0xf3` (colour, depth, stencil): colour `0xff000000` (opaque black), depth/stencil `0x0000ffff` | 90 |
| Draw state | depth test on, `LEQUAL`, write on, blend off on the scenery; **cull face `0x404` (front) against the main view's `0x405` (back)**: the mirror reverses the winding. The six sky faces (the plain sky program) draw first, unculled | 90 |
| What it holds | 6 sky faces and 17 to 58 scenery draws (`dc_diffuse`, `blimphull*`, `glass_reflect`, `dc_windows_b_opaque`, `diffuse_specular`, ...), the bit-3 set as far as the frame shows it; the sea, the track and the craft are not in it | 85 |
| The camera | its view-projection, `c[260..263]` of the target's draws, is the main view's `c[256..259]` with **row y negated and row 3 plus `K` times main row y**, `K = -50.0000` (least-squares fit, residual 0.00004 over all four columns), on all three frames. That is `y' = K - y`: a mirror about **y = K / 2 = -25.0**. Rows x and z, so the projection, the field of view and the aspect, are the main view's (not widened, unlike the behind-glass target) | 90 |
| `K` and the envsettings | Moa Therma's `track.envsettings` authors `Water plane height` = -50.0 (and `Water distortion` 40, `Refract distortion` 2); K equals it. Every circuit that ships a planar reflect authors -50, so this cannot tell "K is the setting" from "K is a constant"; Vineta K authors 100 and has no planar reflect | 70 for the link |
| The sea sheet | the five chunks decoded from the dump (`s16` position times `c[466]` plus `c[467]`; their x and z ranges tile edge to edge, which is the check on the decode) are flat at **y = -26.0547**, 1.05 below the mirror plane | 90 |
| `reflectProject` | the four rows of the **main** view-projection (the sea program's constants for `P.y`, `P.x`, `P.z` and the translation row equal `c[256..259]` of the same draw to 5 digits); the sea is sampled at the point's own screen position `(0.5 + q.x / 2w, 0.5 - q.y / 2w)` (a point on the mirror plane projects to the same pixel through the mirrored scene as through the real one). Each sea draw also carries the mirror matrix in `c[260..263]` | 90 |
| `distortion` | the sea program scales the normal's `x` and `z` by the constant `{0, 0, 40, 0}.zzzz`: **`distortion.z` = 40** on all 18 sea draws read, equal to the circuit's `Water distortion` (40 on all 11 envsettings read). The glass reads `(0, 0, -1, 1)` on Vineta K (`Refract distortion` -1), so `distortion.z` is the authored amount for the surface's kind (the sea program's slot reads `{0, 0, 40, 0}` and uses only `.z`) | 90 (value), 80 (the setting it comes from) |
| Only users | in the three frames **6, 5 and 7 draws sample the target and all are `reflectplane_dc_seawater`**; they are exactly the draws that bind the sea's normal map on units 0 and 2 (checked by texture, so not an artefact of reused program bytes). 55, 61 and 40 other main-view draws leave the target bound on unit 3; of those, 37, 46 and 31 have a program address no later draw reuses and their disassembly never reads unit 3, and the rest (18, 15, 9) share an address with a later draw, so for them the bytes read are not theirs and "never reads it" is not established. A census of the draws in one frame, by material, not of the circuit's chunks and not of render flags: the disc census (30 bit-1 chunks of this material, 257 bit-3 chunks) is the static side | 80 |
| Main view in the same frames | 1300 x 1080 (`clip 0x0514 x 0x0438`), the scene-width throttle (`--pin-scene-width`, [emulator-recipes.md](../reverse-engineering/emulator-recipes.md)); the reflection target was 960 x 540 beside it. Only one output size (1920 x 1080) was seen, so whether the target scales with the output or the throttle is unread | 85 (the values), unread (the dependence) |

What the engine should build from this (the thread's lane 4): a second pass before the
main view that draws the bit-3 chunks and the sky into a 960 x 540 R5G6B5 + Z16 target
cleared to opaque black / depth 1, with the main view-projection mirrored by `y' = K - y`
(`K` = the authored `Water plane height`, -50, so the plane is at -25), front-face culling; the sea samples it at its own screen position through the
main matrix, with its normal offsetting the sample point by `40 * N.x` and `40 * N.z` world
units (zero on a flat normal, so the ripples move the reflection by up to tens of units).
The original ran the target at 960 x 540 against a 1300 x 1080 main view at a 1920 x 1080
output; how it scales at other sizes is unread.

**Chenghou (a second cold boot, `04_Chenghou_Project`; confidence 90 for the target,
nothing for the sea).** Two frames, the start grid (615 draws) and a craft placed at
`0 20 0` (704 draws), neither holding a sea draw. **The reflection pass runs anyway**: in
both frames the same surface (`0x02a20000`, format `0x123`, pitch `0x800`, clip `0x3c0 x
0x21c`, clear mask `0xf3`, colour `0xff000000`, depth/stencil `0x0000ffff`, depth surface
`0x02b40000`) takes exactly **six draws, the sky's six faces** (the plain sky program), and
no scenery, which fits the disc census (Chenghou has no bit-3 chunk). The sky draws'
rotation matrix is the main sky's with row y negated, as the mirror requires (a sky
matrix has no translation, so `K` cannot be fitted there). So a Chenghou sea, were it to
sample this target, would see the mirrored sky over opaque black, and the target is the
same size and format as Moa Therma's. What the sea itself binds there is **unread**: no
sea draw was in either frame, and in the off-track frame 44 main-view draws left the target
bound on unit 2 (not Moa's unit 3), none sampling it. Modesto Heights was not booted.
Ours' half of why no sea draw appeared: from the Chenghou start section the three sea
chunks are culled with the visibility set on and submitted with it off (the same camera
at 1200 above the origin, `OAG_ONLY_MATERIAL=reflectplane_dc_seawater`, `--pvs true`
against `--pvs false`); that the original's set is picked by the craft's track section is a
hypothesis, not read.

**What is still unread, and the recipe.**

- **Chenghou's and Modesto's sea draw** (the claim's last part: which texture unit and
  surface the sea samples when no bit-3 chunk exists). Recipe: find a track section whose
  visibility set holds the sea (ours: the log line `N draw(s) submitted` with
  `OAG_ONLY_MATERIAL=reflectplane_dc_seawater --pvs true` at a pose on that section),
  place the craft there with `scripts/hd-sea-place-dump.py`, then
  `scripts/hd-sea-reflect-read.py <dump> <stem> <circuit materials dir>` prints which surface
  the sea's sampled unit points at, or `no sea draw in the frame`. States:
  `data/saves/hd-fury/hd-timetrial-chenghou-harimau-std-start.SAVESTAT.zst` (saved; Modesto:
  `emu-hd-timetrial-state.sh <lane> 10 9 0 <name>`).
- Whether bit 1 (`0x02`) marks the mirror: the frame census names draws by material, not by
  chunk flag. Read the chunk's flag word from the dump's vertex stage with the draw's
  chunk offset (not done).
- Ubermall (13/7 chunks, a pool): not framed.

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

Re-read 2026-11-17 (`water-hd`, `water_noref` vertex block 3 and fragment blocks
`@0x3870`, `@0x24b0`; `water_test_2` block `@0x1d60`, confidence 85): the scroll adds
`0.05 t * rate_i` to **both** axes of `uv * scale_i`; the three taps of `waves2` are decoded
`2x - 1` on all three channels and summed as `t3 + (t1 + t2) / 2` (two `MAD /2`), then
put through the same `T`, `B = cross(N, T) * w`, `N` frame as the sea. `water_noref` reads
its second map (`0x65f5d93c`) at `uv` plus the third tap; its alpha `a` weights the sky
reflection, `probeTint(sky) * a` in the lightmapped block `@0x3870` (`@0x5c`) and
`probeTint(sky) * (0.2 a + 0.4)` in the plain block `@0x24b0` (`@0x3e`, `@0x56`), and
`sat(a * 0x697f57e3)` tints its sun glint.

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
the Altima material (`scripts/vita-gxp.py dump`, `scripts/vita-usse.py disasm`;
the lightmapped permutations `#4`/`#5` differ only in how the baked light is
summed, which the generic lit path already does). **Read instruction by
instruction, 2026-11-17 (`water-2048`), confidence 85:**

- **Scroll.** The vertex program's secondary instruction 2 is `mul sa36.x, sa38.x,
  sa40.y` (`time` times the literal block's second word, `0.05`, read off the
  program's literals) and primary 31 is `add pa12.x, pa12.x, sa36.x`, `pa12` the
  `uv1` attribute that primary 32 and 33 pass on: the coordinate is `(u + 0.05 t, v)`.
- **Normal.** Fragment `@1` is `r0 = tex * 2 - 1` on **all of `.xyzw`**, `@2` to `@4`
  apply it as `r0.x T + r0.y B + r0.z N0` through the interpolated frame (the vertex
  program builds `B = cross(N0, T) * w`, `@24` to `@27`), `@5` to `@11` normalise
  that and the view vector. It is an **RGB** decode, not HD's `.ag` plus a rebuilt `z`.
- **Light.** `@15` is `N.L` (`liveLighting0direction`), `@22` (`@21` on the
  vertex-lit permutation) `(ambient + diffuse * sat(N.L)) * ColourConstant`.
- **Specular is Phong.** `@12` to `@14` build `R = 2 (N.V) N - V`, `@16` to `@20`
  `pow(sat(R.L), SpecularPower)`, and `@23` multiplies it by `liveLighting0specular`
  and by **the normal map's alpha** (`pa2.w`, never overwritten after the fetch).
  There is no `sat(8 N.L)` gate and no material specular colour, unlike HD's sea.
- **No reflection sampler**, and the fog lerp ends it.

Altima authors `ColourConstant` `(0.133, 0.173, 0.219)` and `SpecularPower` 250,
`water_lapping_waves2_n.gxt` as `NormalMap`. **`liveLighting0specular` is drawn as the sun's
diffuse colour, chosen, not measured**: 2048's envsettings author no sun specular colour or
intensity (Omega's carry both, `docs/formats/envsettings.md`), so the glint's brightness rests
on that stand-in; the engine's own `specular x0.00` line is the absent key, not a measured zero.
Whether the `time` uniform is in seconds is not recorded (our clock is). The literal of `@1` (`sa18.x`, `2.0039` in
`f16`) and `c15.y` (1) are not traced to a register by name; the shape is unambiguous.

**fc17, read whole (2026-11-17, `water-fc17`).**
`fc17_effects_vscroll_wateredge_vertexcol` (Altima's lavender sheets, state 61, blended)
and `fc17_effects_vscroll_water` (Park's sea, state 124, opaque) share fc15's decode,
frame, `N.L` and Phong exactly (`@1` to `@21`), then: `colour = (ambient + diffuse N.L)
* Watercolour`, `+ pow(sat(R.L), SpecularPower) * liveLighting0specular * SpecularColour`
(`@22` to `@24`, **no alpha gloss** and a material specular colour, unlike fc15), the fog
lerp (`@25`, `@26`). Confidence 85 on those.

- **Scroll.** Secondary 2 and 3 compute `Speed * time`, primary 34 adds it to `uv1`'s
  **second** lane (`add pa12.-y`): the coordinate is `(u, v + Speed t)`, `Speed` the
  material's own parameter (Altima's edge 0.05062, Park 0.05). fc15 scrolls `u` by a
  literal `0.05`.
- **Alpha, edge only.** `@27` to `@29` read `x = a * pa0.w`, `alpha = pa4.w * (Opacity - x)
  + x`, that is **`alpha = vc.y * Opacity + (1 - vc.y) * a * vc.x`**, `a` the map's alpha,
  `Opacity` the material's parameter (1.0 on Altima). Park's program ends `mul pa0.xyzw,
  pa0.xyz0, c15.y`: it has no `Opacity` or vertex colour, its alpha is never read (opaque).
- **Which lanes feed `pa0.w` and `pa4.w` (confidence 75).** The vertex program's outputs,
  by their `o` registers (`scripts/vita-usse.py disasm` on program #2, the plain
  permutation, and #4, the vertex-lit one): `o4..o7` = (`B.xyz`, **`VertexColour1.x`**,
  primary 33 `mov o6.-y, pa16.-x`, read before primary 40 overwrites `pa16`), `o8..o11` =
  (`V.xyz`, **`VertexColour1.y`**, primary 41 `or.u32 o11, pa17`), `o12..o15` = (`N.xyz`,
  fog, primaries 19 to 24), `o16/o17` the scrolled coordinate, `o18..o20` `T`. In the
  fragment the groups appear as `pa0` (`B`, so `pa0.w` = vc.x), `pa4` (`V`, so `pa4.w` =
  vc.y), `pa6` (`N` and the fog lerp's `pa6.w`), `pa8` (`T`), the same roles `@2` to `@11`
  and `@26` use them in. **Both permutations agree** (#4's groups are `B`+vc.x at `o4`,
  `N`+vc.y at `o8`, `V`+fog at `o12` and the fragment #5's `pa0.w`/`pa4.w` land on the
  same two lanes), which is what pins the pairing: they could be swapped only by a
  compiler that reorders the same source differently in two permutations. No iterator
  table in the container confirms the pairing, so this stays 75, not 90.
- **The vertex colour is `VertexColour1`, type 4, on the Vita** (four unsigned normalised
  bytes at offset 24 of Altima's 32-byte edge vertex). `oag_rcs::rcsmodel::psp2::SubMesh::
  colours` never carried it (`vertex_colour1` filters the PS4's type 11), so the loader gains
  `vita_vertex_colour1`, read for these two lanes only.
- **`Watercolour` is not authored on Altima's edge** (its four parameters are `Opacity`,
  `SpecularColour`, `SpecularPower`, `Speed`), and the program's uniform image holds only
  the literals (`2.0`, `0`), no default. **The edge takes the circuit's `fc15` pond
  `ColourConstant` as its `Watercolour`, chosen, not measured** (a neighbour authored by the
  same shader family, `(0.133, 0.173, 0.219)` against Park's own `Watercolour`
  `(0.133, 0.173, 0.218)`).

Drawn as kinds 7 and 8 of the water entry (`mesh::rcs::psp2::water`, `VITA_WATER_KIND`,
`VITA_EDGE_KIND`, `shade.wesl`'s `vita_fc17` and `vita_edge`), classified by interface
(`Law::of`): a time-scrolled material with one `NormalMap` and a `Watercolour`, or whose
ambient-only program also samples the map and declares exactly `Opacity` and `Watercolour`.
`Opacity` rides the header's `offset`, `SpecularColour` the specular lanes, the vertex
colour's `x` and `y` the second coordinate pair (`vita_edge` reads them as the chain's
`vc.x`, `vc.y`). The alpha chain is pinned by `vita_fc17_edge_lit_path` (a triangle drawn
through the pipeline, five readings; drop the map's alpha, `vc.x`, `vc.y` or `Opacity` and
one fails) and the batches by `omega_transparency_ground_truth`'s walk: Altima 5 `fc15` +
5 edge, Park 4 `fc17`, Mall 3, Tower 4, Square 4, 25 in all, 9 of them `fc17`, 5 blended.

Of the HD circuits' materials on the Vita, `reflectplane_dc_seawater` declares
`screenSpaceReflectionTex` and `time` (34 programs each), `dc_seawater`
(Vineta K) `time` only, `dc_seawater_cheap` neither. Moa Therma's Vita model
tags 353 mesh objects `wohdmeshreflect`, Ubermall's 52, Altima's none. The
target those feed is the half-size one `Render_GetRefractionTexture` returns,
bound to both `screenSpaceReflectionTex` and `screenSpaceRefractionTex`
(the 2048 pass page). **Checked against Omega: differs** on 2048's own
circuits (below), **applies** on the HD circuits (same material names and the
same reflect tag).

**What the build matches.** A `.rcsmaterial` whose lit fragment permutations declare
`NormalMap` and nothing else beyond the engine's samplers, whose ambient-only permutation
declares no material sampler and `ColourConstant`, and one of whose vertex programs declares
`time` (`oag_rcs::gxp::sampler_use`, `SamplerUse::time_scrolled`; by interface, not by
name). That is `fc15_effects_vscroll_water` and nothing else on the disc: Altima 5
submeshes (two materials, lightmapped and vertex-lit), Mall 3, Tower 4, Square 4, 16 in
all, pinned by `omega_transparency_ground_truth` (`the_2048_seas_are_the_fc15_batches`).
The DLC and patch circuits' seas (`reflectplane_dc_seawater`, `dc_seawater`) declare two
samplers and are not matched. A PS4 model is never matched: its programs are not GXP.

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
| Pulse, Pure water textures | under `off` and `original`: the texture, with any authored texture-offset track; under `enhanced` (this project's own, see below): HD's sea law over a generated normal map on the listed sea textures | nothing known |
| Pulse Vostok Reef caustics (PS2 disc) | under `original`: the pass (255 of 257 flagged batches; two under an animated node are left out), frame by the world clock; under `off`, nothing | a PCSX2 frame, the mesh fog |
| HD `water_test_2`, `water_noref` | under `original`: the three scrolled taps through the chunk's frame, `water_test_2` lit vertex colour plus vertex colour times the sky reflection, `water_noref` the sky reflection times `a` (or `0.2 a + 0.4`) plus the generic glint (`mesh::rcs::water`, `PROGRAM_WATER`); under `off`, lit vertex colour or black plus fog | `water_noref`'s own glint (`sat(a * 0x697f57e3)`, power `0x2a2dc10b`) |
| HD `sebenco_ice` | the three-colour lerp (`mesh::rcs::ice`) | bump-perturbed facing, own specular |
| HD `reflectplane_dc_seawater` | under `original`: `light * ColourConstant + sun * SpecularColour * sat(N.H)^SpecularPower * sat(8 N.L)` over the two scrolled taps, through the chunk's frame (`mesh::rcs::water`); under `off`, the generic reading, where the non-lightmapped slots (Ubermall, Chenghou, Modesto's plain slot) add `DiffuseTexture1`, a normal map, as a glow | the planar reflection (`Reflectivity * R`) |
| HD `water` (Amphiseum) | under `original`: the two taps, `light * ColourConstant + Reflectivity * probeTint(sky)`; under `off`, generic | nothing known |
| 2048 `fc15_effects_vscroll_water` | under `original`: the constant colour times light through one RGB normal map scrolled `u + 0.05 t` and the tangent frame, plus the map-alpha-weighted Phong highlight (`mesh::rcs::psp2::water`, kind `VITA_SEA_KIND`, HD's `sea_block` layout with the second tap at weight zero); under `off`, the flat colour | the program's `liveLighting0specular` is the scene sun here, not read per circuit |
| 2048 `fc17_*` (Altima's lavender sheets, Park) | under `original`: the constant `Watercolour` (the edge's a chosen stand-in) times light through the normal map scrolled `v + Speed t`, plus `SpecularColour` Phong; the edge blended by `vc.y * Opacity + (1 - vc.y) a vc.x` (`mesh::rcs::psp2::water`, kinds 7 and 8); under `off`, the normal map drawn as a picture (`VITA_FAULTS`) | the Vita3K frame of the original (4a) |
| Omega seas | the constant colour times light, flat (`psp2::lighting_only`) on the families whose instance is 2048's; `reflectplane_*` unread | normals, scroll, specular, reflection |
| Refraction target (Vineta K glass) | drawn on HD, 2048 and Omega | n/a; it is the pattern the reflection target follows |

What already exists and the water can reuse: the sky as a dual paraboloid with
`probeTint` (`mesh::sky_paraboloid`, `window_glass`), the behind-glass target
and its `refractProject` (`mesh_render::uniforms`, `oag_raceplay`'s
`scene::behind_glass`), HD's vertex-side scroll as `AnimTrack::Scroll`
(`mesh::rcs::vertex_scroll`), `Mesh::render_flags`, and the 2048 name-tag split
(`psp2::refraction_only`).

**What a player sees (2026-11-17, `water-hd`, headless `--screenshot`, 1280x720).** Moa
Therma's sea, a grey sheet before (the normal map painted as a picture), is a dark teal
plane whose fine ripples drift, with the sun's highlight only where the half vector
lines up; with no planar reflection it is matte. Two frames 2 s apart (ticks 300 and
420, camera `210,40,-380` looking down the `-z` sea, `--pose-hold`) differ on 389,342
pixels of the sea (mean 2.9 levels); off the sea the differences are the before pair's
own (the HUD clock, the sky traffic) plus a halo of 21,823 pixels outside a
40-level skip mask, 96% of them one level and at most 11, which follows the sea's
moving highlight (the bloom is the likely carrier; no switch isolates it). Vineta K's
open sea from above was a black sheet under `off`; under `original` it is a rippled sea
reflecting the sky with a sun glint. Ubermall's pool panels change by little.

**Against the original, one sea pose (2026-11-17, `water-hdcap`).** One RPCS3 frame
of Moa Therma's sea (craft placed at `210 38 -380`, the window at 1920 x 1080, taken while the
game was paused; the dump's main view-projection decomposed by `ps3_pose.py` into eye
`(209.615, 38.304, -367.677)`, forward `(0.0128, -0.1133, -0.9935)`, up `(0.0013, 0.9936,
-0.1133)`, field of view 60.014 degrees, aspect 1.7778) and ours rendered at that camera with
`oag-game --race --camera-pose=... --camera-fov 60.0144 --size 1920x1080 --ticks 572
--no-hull-shake`, the HD render profile's `water` set to `off` and to `original` (the
setting is per render profile, `[render_profiles."Wipeout HD (PS3)"]`, not under `[graphics]`).
**Not made with `rpcs3-hd-pair.py`** (not attempted: a placed craft is reset to the track
within seconds: one placed at y 150 was back on the track when read about 7 s later; so the
placement, the dump and the window grab are one attach in
`scripts/hd-sea-place-dump.py`). This pair has no `pair.json`, so it cannot be re-rendered by
`compare-rerender-ours` or put on the compare page; the frames stay under `data/`. The camera
registers well (the tower, its shadow ring and the horizon line up); the craft is only in
theirs, because ours places it on the grid.

Crops (the first two fixed before the metric was read; the third replaced a horizon strip that
took in the yacht's deck, theirs luma std 33.5 there against 4.2 now; mean RGB and Rec. 709 luma,
0 to 255):

| Crop | Original luma | `off` luma, MAE | `original` luma, MAE |
| --- | ---: | ---: | ---: |
| band `(620,500)-(1300,640)`, sea between the tower and the yacht | 140.4 | 138.7, 20.9 | 104.9, 36.0 |
| foreground `(0,640)-(780,840)`, shadowed sea | 138.2 | 138.1, 24.6 | 103.5, 35.3 |
| near-horizon strip `(700,500)-(1300,540)` | 145.2 | 153.0, 11.9 | 116.9, 28.4 |

Mean RGB in the band: original `(126.5, 144.5, 140.4)`, `off` `(139.2, 139.1, 133.1)`,
`original` `(88.6, 109.6, 106.4)`. **What a player sees.** The original's sea is a pale
teal, finely rippled, with the tower, its shadow ring and the yacht mirrored in it (soft,
broken by the ripples); the water brightens toward the horizon. `off` is a grey,
noisy sheet (the normal map painted as a picture), which happens to match the original's
brightness and is the wrong colour. `original` (the taps, no planar term) has the right hue
(teal, the authored `ColourConstant`) and fine ripples, and is a flat matte plane 28 to 36 luma
levels **darker** than the original in every crop: it lacks the reflection term
(`Reflectivity` 0.3 of the sky and scenery picture, plus the sun's glint in it), which is what
brightens and lifts the original's sea. By MAE `off` is closer (12 to 25) than `original`
(28 to 36), which says the unwired planar term, not the taps, is the biggest remaining
difference, and that `original` reads worse than `off` until it is wired. One frame, one
pose, one tick: a sea-crop number, not a verdict on the taps' phase (the scroll runs on the
race clock and was matched to the HUD's 5.0 s only by tick count).

## Enhanced: this project's own water on Pulse and Pure

2026-11-17, lane `water-enhanced`. **Chosen, not measured, everywhere but the shape of the
law**: the maintainer's decision (2026-11-17) is an `enhanced` value of the WATER row on a
title whose original draws no water program, with no other disc involved. The shape is HD's
sea (two taps of a wave normal map scrolled in opposite directions, light times a colour, a sun
glint, the sky's reflection and no planar term, as Amphiseum's), drawn by our shader
(`water.wesl`, family 5 of the PROGRAM_WATER entry; `mesh::vex_water`).

| Part | What it is |
| --- | --- |
| Surfaces | Per-circuit lists in `oag_pulse::race::WATER_CIRCUITS` and `oag_pure::race::WATER_CIRCUITS`, carried as `Title::water` (`oag_title::water`) and read by the loader; the generic crates name no title. A batch is water when it binds a listed embedded texture. |
| Normal map | Generated (`mesh::water_normal`): value noise on a wrapping integer lattice, four octaves, 128 square, slopes by wrapped central differences. Tiles exactly; a seed gives the same bytes everywhere (integer hash and IEEE `+ - * /` `sqrt` only), pinned by a test. `x` in alpha, `y` in green, as the sea's taps decode it. Never copied from a disc. |
| Taps | HD's two (`Tap::sea`: `u + 0.05 t` and `1.15 (-u + 0.05 t)`), on the world's `x` and `z`, one tile of the map per 240 world units. The surface's own texture coordinates give no wave size (Moa Therma's `NewSeaTest` repeats 16 times across 3,700 units, Vineta K's `AAdc_Water` once across 1,200), so they are not read. |
| Colour | The mean of the surface's own texture's base level (`oag_title::water::Circuit::textures`), times the vertex colour's baked light (Pulse's are prelit), times `0.5 + 0.5 N.L`. |
| Glint, sky | `0.5 * sat(N.H)^90 * sat(8 N.L)` in white, the sun the stand-in key light's direction `(0.4, 0.8, 0.5)`; the sky is `Reflectivity 0.22` times the sky at the reflected ray, weighted `0.4 + 0.6 (1 - N.V)^3`. |
| Sky map | A mesh sky (Pulse's and Pure's `Skycube` dome, 477 to 518 triangles) has no map of its own, so one is cast out of it at load, only where a circuit's water reads it (`sky_paraboloid::from_mesh`): the ray of each texel against the dome, texel times vertex colour, in the engine's dual-paraboloid layout. |

**Which surfaces are water, confirmed on a frame** (top-down and low camera poses, headless
`--screenshot`, the same pose under `off` and `enhanced`):

| Title | Circuit | Texture | Draws / vertices | On a frame |
| --- | --- | --- | --- | --- |
| Pulse | `03_Track` (Moa Therma), forward and reversed | `NewSeaTest` | 25 / 7,210 | the sea: flat near-black under `off`, rippled with a glint under `enhanced` |
| Pulse | `06_Track` | `sea` | 10 / 1,788 | the teal sea |
| Pulse | `14_Track` (Fort Gale) | `NewSeaTest` | 3 / 468 | the sea |
| Pulse | `16_Track` | `water` | 1 / 75 | the river, a narrow strip: it changes, little |
| Pure | `01_Vineta_K` | `AAdc_Water`, `AAdc_WaterDark` | 21 / 5,795 | the open sea, both halves |

**Not listed, and why.** `SeaRock`, `Seats`, `rich_Seagull`, `WaveBuilding_*` (a substring match
on `sea` and `wave`, not water); `WaterFoam`, `AAdc_waterFoam` (blended foam, drawn as it is);
`NWaterfall*`, `Waterfall_*` (waterfalls); `rich_sea_GLOW` (a Zone sign); `Seabed*`; Blue
Ridge's `Water`: embedded, **bound by no batch** (a census by name found it, the batch walk
did not). Pulse PS2 embeds no names and resolves them through its texture set: Moa Therma's sea
is found there too (12 / 4,775).

**What a player sees.** Moa Therma from above: the near-black plane becomes a dark slate sea
with moving ripples and bright glints (40% of the frame is sea; over it two frames 2 s apart,
ticks 2 and 122, differ by a mean 18.7 levels against 3.1 under `off`, the rest the sky).
Vineta K from above: a flat dark teal sheet becomes a rippled teal sea, 74% of the frame, 21.0
levels mean difference 2 s apart against 0.0 under `off`. Low over the water the waves and the
sky's reflection show as a real sea. In a race frame from the track the sea is often not in
view and the frame is unchanged. `off` and `original` are pixel-identical to the build before
this change on Pulse PSP, Pulse PS2, Pure and HD (Moa Therma's sea pose).

**The row.** WATER is shown on a title only when something in it changes the picture: its own
`original` differs from `off` (`Title::water.original_differs`: HD) or a value past those two
is offered (`enhanced` on a title that lists a sea, or a borrowed law that is drawn and whose
disc is present). Today that hides it on 2048 and Omega and shows it on Pulse, Pure and HD.
`oag_game::water_setting::row_changes_picture`.

Open: whether the glint and the sky are right beside an original frame (nothing to compare
against: the original draws a flat texture); one look or one per circuit; the sun is fixed in
the world, so a glint shows only on the sun's side.

**What a player sees on Altima (2026-11-17, `water-2048`, headless `--screenshot`, 1280x720, debug build).**
The pose is `--pose=-250,-8,1372 --pose-hold` with the camera pinned
(`--camera-pose=200,14,1330,-0.16,-0.56,0.81,0,1,0`, looking down along the reflection of the
sun `-0.16 0.56 0.81`), ticks 300 and 420. The big pond beyond the road, flat near-black under
`off`, shows a sun glint of fine rippled sparkle in a patch and a faint ripple shading across
the rest; the sparkle moves between the two frames. Over the 2 s pair, on against off differs on
3,706 pixels over 24 levels (3,428 at tick 420), all inside the box x 553 to 1177, y 318 to 468
(the pond); elsewhere on and off agree to the bloom's single levels. The 300-to-420 difference is
5,812 pixels over 24 levels larger with the law than without, in the same box (the road, the
HUD clock and the lavender `fc17` sheets scroll in both). `off` is **pixel-identical to main**
(0 differing pixels at both ticks against a build of main). The lavender sheets
(`fc17`) were the next thing: see the next paragraph. The highlight (its brightness a chosen stand-in, above) only shows near the
sun's mirror angle (power 250), so a typical chase view shows the ripple shading and an
occasional glint, not a constantly sparkling sea.

**What a player sees at Altima's pond edge and Park's sea (2026-11-17, `water-fc17`, headless
`--screenshot`, 1280x720, debug-profile `quick` build, ticks 300 / 120).** Altima, the pond
under the road beside the shore (`--pose=-250,-8,1372 --pose-hold`, cameras
`100,40,1150,0.05,-0.25,0.97` and `100,200,1100,0.1,-0.7,0.7`): under main's `original` the
shore carries lavender wedges, one fanned round the ferry and one along the far bank, the
water normal map painted as a picture. Under this change the wedges are gone: the pond is
the dark water it is elsewhere, and the sheets only shade it, with the shoreline's gravel
visible through them and the ferry sitting in dark water instead of a lavender fan. The
sheets are translucent (the alpha chain, with `vc.x` and `vc.y` as the loader reads them),
not a hard-edged foam strip; whether the original's edge shows a lighter foam colour is open,
because `Watercolour` is unauthored on the edge and our stand-in is the pond's own colour (a
Vita3K frame would settle it; **not taken**, see Open 4a). Park's sea changes by at most 4
levels and over 1,391 pixels above one level (the sun is far from the mirror angle from that
view and the colour was already `Watercolour`): the effect there is the moving ripple shading
and the glint only. `off` is **pixel-identical** to a build of the parent commit (0 differing
pixels at Altima's `100,200,1100` pose, 300 ticks, and Park's `1000,200,-3500` pose, 120
ticks).

## Lineage, per finding

| Finding | 2048 | Omega | HD |
| --- | --- | --- | --- |
| Sea law (two taps, planar reflection) | checked, applies on its HD circuits (`reflectplane_dc_seawater` declares `screenSpaceReflectionTex` and `time`), not wired: its draw goes through `psp2`, which classifies by the GXP program's interface and has no tap reader; the `0.05` and `1.15` literals are not checked in the GXP | checked, applies (HD's interface exactly), not wired: same `psp2` path, the literals unread in the GCN program | taps drawn without the planar term (`mesh::rcs::water`) |
| Reflect set (bit 3 / `wohdmeshreflect`) | checked, applies (name tag, 353 and 52), not wired | checked, applies (268 and 73), not wired | measured here |
| 2048 own water (`fc15`/`fc17`) | `fc15` and `fc17` measured and drawn here | **checked, differs**: Omega's 2048 mode replaces both with `reflectplane_*` (two taps, Fresnel planar reflection) on the same meshes, its programs are GCN, and the build matches GXP only; Omega's Zone models keep `fc17` and no `fc15` ships there: **checked, applies, not wired** for `fc17` - the build classifies GXP programs only and `water::plan` returns on a PS4 source, so the edge draws as before (flat constant, normal map unread); the law (same material name and, if the instance record matches, the same `Opacity`/`Speed`/`SpecularColour` parameters) would apply once the GCN program is read, which this lane did not do | not applicable |
| Amphiseum `water` | not checked (the Vita's Amphiseum shows no water material by name) | checked, differs (cube probe) | drawn (`mesh::rcs::water`) |
| Vineta K scroll table | checked, differs (`dc_seawater`) | checked, applies (`water_test_2_castnoshipshadows`), `water_noref` replaced by `temp_water`, not wired | drawn (`mesh::rcs::water`) |
| `enhanced` (own generated map, Pulse and Pure only) | checked, differs: nothing is offered (`Title::water.enhanced` empty); its seas are drawn through `psp2`, no `.vex` batches | checked, differs: same, and Omega's own law is the planned one | not applicable (HD's `original` is the law `enhanced` follows) |
| Pulse caustic pass | not applicable | not applicable | HD ships `data/tex/caustics/` and `wo_underwater_godrays.pob`; consumer not looked for |

## Open

1. Chenghou's and Modesto's seas, Ubermall's pool, and whether bit 1 marks the mirror
   (the planar reflection's other values are read on Moa Therma, "The planar reflection,
   read off live frames", with the recipe for these).
2. Vostok Reef on PCSX2 or PPSSPP (with the download pack): a frame of the
   caustic pass to compare against ours (batch state `0x202`'s blend is read: additive by alpha).
3. Pure's caustic frames on Vineta K.
4. Omega's `Reflectivity*` combine and `Speed` law (a GCN instruction read),
   (2048's `fc17` edge program is read whole, "2048 (Vita)": its `Watercolour` is unauthored
   and drawn as the pond's `ColourConstant`, chosen; Omega's Zone `fc17` is not wired).
4a. No Vita3K frame of Altima's sea exists: the look of `fc15` and `fc17` is not compared against the
   original (`water-fc17` did not take the 45-minute capture; the edge's stand-in `Watercolour`
   and the lane pairing of `pa0.w`/`pa4.w`, 75, are what a frame would test: walk Empire Climb in
   `scripts/vita3k-drive.py race`, hold position at the pond, `shot` and compare the shore wedges).
5. `sea`, `seareflect`, `seaweed` class nodes: a class census over PSP `.vex`.
6. Blue Ridge's `Water` texture is embedded and bound by no batch; whether a node of
   another class draws it is unread.
