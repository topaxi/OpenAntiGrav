# HD/Fury's Zone race keeps some chunks on their plain program: render flags `0x0e00`

2026-11-16 (`zone-pad-re`). Read because a Zone race on Wipeout HD Fury shows
its speed pads as a light-grey plate with the pad's own texture, where this
port drew them as Zone road surface with a white bar across the tiles.

**The law (confidence 90).** In a Zone race every draw consumer builds the
material permutation word as `ZoneMode | Sun` (`0x101`), **except** for a chunk
whose render-block flags have any of bits 9, 10 or 11 (`flags & 0x0e00`): that
chunk gets `Sun` alone (`0x001`), the same word as an ordinary race. So a
flagged chunk runs its material's plain lit program, with its own albedo, and
none of the Zone surface (zone texture, rim terms, visualiser glow, wavefront
ring). The frame's light and fog constants are still the Zone stage's, so the
chunk is lit by the stage's `Constant Ambient Colour` and `Sun colour`.

On disc the three bits are the speed pads (bit 9), the weapon pads (bit 10) and
315 authored signs, billboards and screens (bit 11).

## The names

| Address | Kind | Name | Confidence |
| --- | --- | --- | --- |
| `0x00400a00` | function | `Scene_DrawOpaqueChunkBucket` | 78 |

`0x00400a00` is what `chunk-cull.md` and `zone-effectsettings-loader.md` call
the B4 consumer: it walks the six-byte records of bucket B4 (`g+0x431f4`,
count `g+0x491f4`), and per record picks the fog buffer, the Zone parameter
set, the permutation word and the lights, then calls
`RenderOps_DrawChunkSurface`. Decompiled in full here. The name is 78, not
higher, because "opaque" rests on the sorter's routing rule on the other page
(`material+0x10 & 1 == 0`), which this lane did not re-read.

The six sibling consumers stay unnamed (`FUN_003fc140`, `FUN_003ff860`,
`FUN_00403a30`, `FUN_004074e0`, `FUN_00408fa8`) or keep their name
(`Shadow_CompileShadowedTrackRedraw`): only the flag test was located in them,
their bodies were not re-read.

## The instructions

`Scene_DrawOpaqueChunkBucket`, with `r30 = record[2] >> 1` (the chunk's
render-block flags, `Mesh::render_flags`; bit 0 track, bit 5 alternate fog, as
the fog pick at the head of the same loop uses them):

```text
0040175c  ...                         ; the Zone parameter set (Track or Scene) is published above
00401760  rlwinm r11,r30,0,20,22      ; r11 = flags & 0x0e00
00401764  lwz    r0,0xfc(r1)          ; the frame's Zone bits: 0x100 ZoneMode, 0x200 ZoneTrans, or 0
00401768  cmpwi  cr7,r11,0
0040176c  ori    r9,r0,1              ; word = zoneBits | Sun
00401770  beq    cr7,0x00400df8       ; no flag: keep it
00401774  b      0x00400df4           ; flagged:
00400df4  li     r9,1                 ;   word = Sun alone
00400df8  ...
00400e04  or     r24,r5,r9            ; | the chunk's ambient source bits (0xf4(r1))
          ...
          bl     Shader_GetVariantHash  ; table[word | NoAlbedo bit]
```

The same branch is taken with `word = 1` when the frame's Zone byte
(`g+0x1104`) is clear, which is the ordinary race. The frame's Zone bits come
from the head of the function: `0x200` (`ZoneTrans`) when the stage entry's
float at `+0x18` is below a constant, else `0x100` (`ZoneMode`), and `0` (plus
`NoAlbedo`, `0x400`) when the byte at `g+0x2053a4` is set.

**The mask is in every draw consumer and nowhere else.** A scan of the whole
image for `rlwinm rA,rS,0,20,22` finds seven instructions, one per consumer:

| Site | Function |
| --- | --- |
| `0x003fd474` | `FUN_003fc140` (bucket B3) |
| `0x004005b8` | `FUN_003ff860` (bucket B5) |
| `0x00401760` | `Scene_DrawOpaqueChunkBucket` (bucket B4) |
| `0x00404d6c` | `FUN_00403a30` (bucket B2) |
| `0x00406e5c` | `Shadow_CompileShadowedTrackRedraw` |
| `0x00408934` | `FUN_004074e0` (`RenderTrackWithLights_blended`'s run) |
| `0x00409dc8` | `FUN_00408fa8` (`RenderTrackWithLights_zWriters`' run) |

Confidence 84 on "all seven do what B4 does": the instruction is identical and
the functions are the documented consumers, the six other bodies were not
decompiled.

### The lightmap term is off in a Zone frame

The chunk's ambient source bits (`IleVertex` `0b010`, `IleLightmap` `0b100`)
are added only when the byte at `EnvSettings+0x4b0` is set (`0x00400d40`, and
the same load in the six siblings). `Scene_PrepareFrame` writes that byte:
`1` at `0x003abaf8` on the ordinary arm, and `0` at `0x003ad8fc`, straight
after its call to `Environment_UpdateStageBlend` (`0x003ad8e8`), on the arm
taken when the Zone-active byte is set (`lbz r9,0(r17)` / `bne` at
`0x003abac8`..`0x003abae8`). So every Zone draw is an `Ambient` permutation:
no lightmap, no baked vertex colours. Confidence 80 static, 90 with the live
frames below (1,091 of 1,091 attributed draws).

## Live: three RSX frames (RPCS3 0.0.43-20574)

Racebox, RACE TYPE `ZONE`, WEAPONS `OFF`, Moa Therma (`03_Track`), zone 0
(Sub Venom). Each frame's draws were dumped at a pause
(`scripts/rpcs3_draw_hook.py`, `HOOK_LIGHT=1 HOOK_NO_HEADS=1`) and every
draw's fragment program was matched, constants stripped, against the blocks
of the circuit's 61 `.rcsmaterial` files (`scripts/rsx_fp_match.py`); a block
names its permutation through the material's variant table.

| Frame | Attributed draws | `HalfBrightZoneMode*` (Zone) | plain |
| --- | ---: | ---: | --- |
| pre-race fly-over, grid | 381 | 380 | 1: `speedup_material#5` |
| pre-race fly-over, grid (the race entered a second time) | 386 | 385 | 1: `speedup_material#5` |
| race running, craft parked at `-350, 6.1, -405.6` | 324 | 317 | 7 (below) |

The seven plain draws of the race frame:

| Draws | Program | Permutation | Flag class |
| --- | --- | --- | --- |
| 116, 117, 182 | `speedup_material#5` (`fp@0x49b0`) | `HalfBrightAmbientSunSpot0SVC0` | bit 9, speed pads |
| 259, 306 | `dc_emissivetexture#3` | `HalfBright{Ambient,IleLightmap}SunSpot0SVC0` (one shared block) | bit 11 |
| 307, 308 | `dc_diffusenormalspecular#3` | `HalfBrightAmbientSunSpot0SVC0` | bit 11 |

Those three materials are exactly the materials of Moa Therma's flagged chunks
(census below), and no other material ran a plain program. The Zone draws all
ran `HalfBrightZoneModeAmbientSunSpot0SVC0` (`0x101`; in the race frame 7 of
them the `ShadowToAlpha` twin and 6 `SVC1`), never an `Ile*` word. The screenshot of
the race frame shows two speed pads with a grey textured plate, a dark
outline, cyan tiles and no white bar.

`speedup_material#5` and `weapon_pads#5` are one program, so the match cannot
tell them apart; all three pad draws bind one texture pair, and the frame
shows speed pads only.

### What the pad's plain program computes, with the live constants

`speedup_material.rcsmaterial` block #5 (`scripts/ps3-microcode.py fp-file`),
samplers the albedo (`0x3bdc0403`, unit 0) and the `_ne` map (`0xa2d555b9`,
unit 1), no lightmap unit:

```text
N       = normalize(T * ne.x' + B * ne.y' + Nv * ne.z')       ; ne' = ne.rgb * 2 - 1
light   = constantAmbientColour + saturate(N . L) * directionalLight0Colour
colour  = albedo.rgb * light
        + ne.a * Colour                                        ; the pad's own glow (pads.md)
        + albedo.a * directionalLight0Colour * specular        ; N.H term, literal 32
fog     = exp(-(fogColour.w * depth)^2)
out.rgb = 0.5 * lerp(fogColour.rgb, colour, fog)               ; the /2 of every HalfBright program
```

Patched constants on all four dumped pad draws (two frames):

| Parameter | Live value | The stage's authored key (`zonemode.effectsettings`, `1 Sub Venom`) |
| --- | --- | --- |
| `constantAmbientColour` (`0x81db67ea`) | `1.5 1.5 1.5` | `Lighting.Constant Ambient Colour` `1.5 1.5 1.5` |
| `directionalLight0Colour` (`0x2dba643d`) | `0 0 0` | `Lighting.Sun colour` `0 0 0` |
| `fogColour` (`0x3dc31258`) | `0 0 0 0` | `Lighting.Track Fog colour` `0 0 0`, `Track Fog density` `0` |
| light direction (`0x02df31e5`) | `0.5 0.707107 0.5` | not a stage key |
| `Colour` (`0x02ab9f07`) | `0 0.768628 0.992157` | the material instance's own value |

So at Sub Venom a speed pad is `0.75 * albedo + 0.5 * ne.a * Colour`, unfogged,
with no sun and no specular: a flat, evenly lit copy of the pad's own texture.
That is the light-grey plate with its outline. The bit-11 draws carry the same
ambient and sun, and `fogColour` `0 1.30588 1.8 0.0021`, the stage's
`Lighting.Fog colour` and `Fog density`: a flagged chunk without the track bit
takes the stage's primary fog, a pad (track bit set) the stage's Track fog, by
the fog pick at the head of the loop (`flags & 0x20` alternate, `flags & 1` in
Zone the Track buffer, `Scene_GetZoneTrackFog`).

A road draw of the same frame (`diffuse_specular#15`, `0x101`) carries the
same `1.5`, `0` and zero Track fog, so the pad reads the same rig as the road;
only the program differs.

## Which chunks carry the bits

Counted over the 72 `.rcsmodel` under `data/environments/` in `DATA00`,
`DATA02` and `DATA03` (`Mesh::render_flags & 0x0e00`, by the chunk's material
file name). Confidence 88 on the counts; they agree with the per-bit totals
`zone-effectsettings-loader.md` took independently (418, 208, 315).

| Flags | Chunks | Materials |
| --- | ---: | --- |
| `0x0201`, `0x0221` | 418 | `speedup_material` 153, `weapon_pads` 208 (the four original circuits' speed pads use that file), `diffuse_normal_specular_emmissive` 57 |
| `0x0401`, `0x0421` | 208 | `weapon_pads` 208 |
| `0x0800`, `0x0808` | 315 | `sign_emissive_glow` 54, `cf_billboard1` 48, `mt_uvanim_diffuse_emissive` 46, `dc_emissivetexture` 41, `cf_vex_billboard` 30, `lambert_simple` 26, `billboarddiffuse` 15, `nr_tvarray_02` 12, 10 more files |

Moa Therma (`03_track/track.rcsmodel`, 1,685 chunks): 12 `speedup_material`
at `0x0201`, 8 `weapon_pads` at `0x0401`, and 22 at `0x0800`/`0x0808`
(`dc_emissivetexture` 17, `sign_emissive_glow` 3, `dc_diffusenormalspecular` 2).

**The flag is per chunk, not per material.** 646 material slots serve a
flagged surface and 30 of them also serve unflagged ones, all bit-11 scenery
(Talon's Junction 6, Tech De Ra 2, one each on Moa Therma, Chenghou Project
and Anulpha Pass, per direction, and two `constantmaterial` slots on each of
`zone_1`..`zone_4`). No pad slot is shared with an unflagged chunk.

Bits 9 and 10 were already known as the bits `Pvs_LoadForTrack` scrubs from
the PVS per mode (`zone-effectsettings-loader.md`), with "no claim about which
pieces they are": they are the speed pads and the weapon pads. What bit 11
means beyond "keeps its own program in Zone" is not read; `sign_emissive_glow`
is the start gantry's card material, so this is the likely measured reason
the gantry's board survives a Zone race (`oag_mesh::mesh::ZONE_EXEMPT_RATE`
is labelled chosen), not checked chunk by chunk.

## Against this port

**Wired (2026-11-16, `zone-pad-wire`).** `oag_rcs::rcsmodel::Mesh::zone_plain` (`RENDER_ZONE_PLAIN`,
`0x0e00`) is carried per chunk as `slots::ZONE_PLAIN` (bit 5 of the vertex `slots` word, below the
seven-bit glow index at bits 24 to 30). `shade.wesl` gives such a chunk,
in a Zone race only, no Zone surface and no Zone glow, the stage's `ambient + sat(N.L) * sun` as its
light (no lightmap, no baked vertex light, no occlusion mask; a program fed no light keeps its
picture) and the plain variant of every program whose Zone variant drops a term (the wave, the
chevron pulse, the alpha glow). Off a Zone race nothing moves: a Time Trial frame at the same pose is
pixel-identical before and after. Tests: `moa_therma_zone_plain_chunks_are_its_pads_and_signs`
(`oag-rcs`, 12/8/22), `moa_therma_marks_the_zone_plain_chunks_and_only_them` (`oag-mesh`, the carrier
on the built model) and `a_zone_plain_chunk_draws_its_own_albedo_under_the_stage_ambient`
(`zone_recolour.rs`, the pixel). Picture, Moa Therma at `--pose=-335.5,8.11,-401.77,0` against the
original's frame at the settled pose: the pad plate's non-cyan pixels average 171/180/184 (original
148/158/161; before the change 220/228/229 with a white bar), 21 % of the box (original 17 %, before
7 %). Open on the same page's list: the Track fog, the start gantry's card, weapon pads with WEAPONS
OFF. The unflagged Zone surface's own lightmap term is closed (2026-11-19: it takes the stage's
ambient and sun too, [zone-frame.md](zone-frame.md), "The Zone surface's light").

The earlier state of this section, for the record:

- Ours sent every chunk through the Zone surface in a Zone race
  (`shade.wesl`, `zone_weight`), pads included: the pad plate became
  `zoneTex * Track.Texture Colour + rim terms`, the road's look. The white bar
  is most likely the Track zone texture's painted line sampled at the pad's
  own UV (60, not isolated: it disappears with the Zone weight, which is all
  the fix needs).
- Switching the Zone weight off for the pad alone is not enough, as a
  throwaway had shown: ours then lights the pad through its lightmapped
  program (`pow(lightmap) * scale + lightmap.a * N.L * sun`), which with the
  stage's `Sun colour` `0` is dark. The original's Zone frame has no lightmap
  term at all; the pad is lit by `Constant Ambient Colour` alone.
- ~~Ours fogs every Zone chunk with the stage's primary fog; the original
  gives a track-flagged chunk the stage's Track fog (zero density at Sub
  Venom).~~ Wired (`det-mines-ahead-structure`, `mesh_render::TrackFog`;
  [detonator-look.md](detonator-look.md), "Track fog and the glass floor").

## 2048 and Omega

**Checked, differs** (2048, and Omega's 2048-lineage circuits). A Zone race
there loads a separately cooked `trackZone.rcsmodel`
(`data/art/published/environments/<circuit>/` on Vita,
`Data/environments2048/<circuit>/` on PS4), and that model authors its own
pads as ordinary textured materials: Altima's `trackZone` has
`speed_pads.rcsmaterial` (31 submeshes, `speedpad_diff.gxt` and
`speedpad_norm.gxt`) and `weapon_pads.rcsmaterial` (61 submeshes) beside its
`Zone_ColourN` materials (`cargo run -p oag-rcs --example psp2_material_dump
-- data/art/published/environments/altima/trackZone.rcsmodel 0,0,1
<data.psarc>`). The pads keep their picture by authoring, not by a flag. The
permutation-word mechanism is not in Omega's executable: `strings` over its
`eboot.bin` finds none of `HalfBright`, `ZoneTrans`, `IleLightmap`,
`ShadowToAlpha` or `SVC0`, and does find `Zone_Colour1`..`8`. The bit this
page describes is HD's render-block halfword, which only the PS3 model
carries.

**Ported (2026-11-25, `omega-zone-fixes`): HD-lineage circuits on 2048's container.** Omega's
HD-lineage circuits (and 2048's DLC1 ones) take HD's Zone surface, so their speed pads were drawn as
road with a white bar, the HD defect above (seen in play on Omega; posed frames of Vineta K at
`--pose=-797.5,-142,272.4`, Omega and 2048 DLC1). The container has no render-block bits, so the
loader marks each of the circuit's own pad submeshes `slots::ZONE_PLAIN` (`mesh::rcs::psp2`,
the pad rule of `docs/gameplay/pickups.md`, "A weapons-off race draws no weapon pad"). Which program
Omega's executable runs for them is unread (a different material container) and no live Omega
value exists: **inherited from HD** by lineage, no confidence of its own. The wired weapon pads are
moot in play, since a Zone race draws none. 2048-native circuits (`altima` on both titles) are
pixel-identical: their pads draw no Zone surface to begin with. HD's bit 11 (signs and screens) has
no counterpart on this container and stays unmarked. Ground truth:
`crates/mesh/tests/psp2_circuit_pads_ground_truth.rs`.

## Reproduce

```sh
# the draw dump: boot, RACEBOX, RACE TYPE ZONE, MOA THERMA, start the race, then
HOOK_LIGHT=1 HOOK_NO_HEADS=1 OAG_RPCS3_ATTACH=1 python3 scripts/hd-draw-dump.py <out> 00
python3 scripts/psarc.py extract <image>:PS3_GAME/USRDIR/DATA02.PSARC <mats> 03_track/materials
python3 scripts/rsx_fp_match.py <out> 00 <mats>/data/environments/03_track/materials
python3 scripts/ps3-fp-live.py <out>/00-c0<fp>.bin     # a draw's patched constants
python3 scripts/ps3-microcode.py fp-file <mats>/.../speedup_material.rcsmaterial 5
```

`rpcs3-drive.py place` taps START RACE once and can miss it; a pose written
during the fly-over does not hold (the craft is parked under the grid at
`-402, -32, -14`). Check the HUD is up before placing.

## Open

- Bit 11's authored meaning, and whether the start gantry's card chunks are
  among its 54 `sign_emissive_glow` chunks.
- The six sibling consumers' bodies (only the flag test was located).
- Weapon pads in a Zone race: bit 10 is scrubbed from the PVS when
  `gameState+0x18` is zero (`zone-effectsettings-loader.md`); with WEAPONS
  `OFF` no second pad texture pair was drawn, which fits, and is not proof.
- A later stage's frame (a stage with a non-zero `Sun colour`), where the
  pad's `N.L` and specular terms would show.
