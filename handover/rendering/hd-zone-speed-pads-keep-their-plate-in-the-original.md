---
status: ready-to-wire
titles: [hd]
updated: 2026-11-16
summary: "HD Zone: chunks with render flags 0x0e00 (pads, signs) keep their plain program under the stage ambient; ours recolours them. Recipe inside"
---

# HD's Zone speed pads keep their authored plate in the original; ours paints them road-teal with a white bar

2026-11-16 (`zone-pad-line` measured the picture, `zone-pad-re` read the law). A
maintainer report from play: on Wipeout HD Fury, a Zone race on Moa Therma,
every speed pad shows a horizontal white line across it.

## The law (confidence 90)

Evidence page: `docs/ghidra/functions/ps3-hdfury-eu/zone-plain-chunks.md`
(instructions, three RSX frames, the per-material census, 2048/Omega).

- In a Zone race the original builds the permutation word `ZoneMode | Sun`
  for every chunk **except** one whose render-block flags have any of bits 9,
  10, 11 (`Mesh::render_flags & 0x0e00`): that chunk gets the ordinary word
  and runs its material's plain lit program (`0x00401760` in
  `Scene_DrawOpaqueChunkBucket`, the same test in all seven draw consumers).
- Bit 9 is the speed pads (418 chunks disc-wide), bit 10 the weapon pads
  (208), bit 11 is 315 signs, billboards and screens. Moa Therma: 12 + 8 + 22.
- A Zone frame has no lightmap or baked-vertex light source on any draw
  (`EnvSettings+0x4b0` cleared at `0x003ad8fc`; 1,091 of 1,091 live draws are
  `Ambient` words). A flagged chunk is therefore
  `0.5 * fog(albedo * (stage Constant Ambient Colour + sat(N.L) * stage Sun colour) + own glow + specular)`.
  Live at Sub Venom: ambient `1.5`, sun `0`, pad fog zero (the stage's Track
  fog), so a speed pad is `0.75 * albedo + 0.5 * ne.a * Colour`.

## Ours, and the exact difference

- `shade.wesl` gives every chunk `zone_weight = zone_on` (line 949), so the pad
  plate is replaced by the Zone surface (`zoneTex * Track.Texture Colour` plus
  the rim terms, visualiser glow and ring): road-teal, with a white bar that is
  most likely the Track zone texture's painted line at the pad's UV.
- With the Zone weight off alone (the earlier throwaway) the plate is dark:
  ours lights the pad through its lightmapped program, and the stage's sun is
  `0`. The original uses the `Ambient` program in a Zone frame.

## Recipe

1. `crates/rcs/src/rcsmodel/render_block.rs`: `RENDER_ZONE_PLAIN: u16 = 0x0e00`
   (bit 9 speed pad, bit 10 weapon pad, bit 11 authored "keeps its program")
   and `Mesh::zone_plain()`, documented from the evidence page.
2. `crates/mesh/src/mesh/rcs.rs`, beside `slots::ZONE_TRACK` (line 274): carry
   the per-chunk flag to the shader. **Per chunk, not per material**: 30 of 646
   flagged material slots also serve unflagged chunks (all bit 11; no pad slot
   is mixed). `slots` has no free bit (0 to 31 are assigned), so the carrier is
   the wiring lane's choice: a free vertex field, or a glow-table rate like
   `ZONE_EXEMPT_RATE` for the unmixed slots plus a split of the 30 mixed ones.
3. `crates/mesh/shaders/shade.wesl`, for a flagged chunk while `zone_on`:
   `zone_weight = 0` (drops the Zone surface and `zone_glow_term`), and the
   light is the `Ambient` case: `scene.light.ambient + sat(N.L) * sun`, with
   the lightmap `prelit` term and the vertex-light curve off. `pad_glow`,
   the specular and the `/2` stay as they are. Off a Zone race nothing changes.
4. Fog (second, small at pad distance): a track-flagged chunk takes the
   stage's `Lighting.Track Fog colour`/`density`, which `ZoneGrade::fog` does
   not read (it reads the primary pair for everything).
5. Tests: a disc-backed check in `crates/rcs/tests/rcsmodel_ground_truth.rs`
   (Moa Therma 12 `speedup_material` at `0x0201`, 8 `weapon_pads` at `0x0401`,
   22 at bit 11; bits 9 and 10 only on the three pad materials disc-wide), and
   a GPU test beside `crates/mesh/tests/zone_recolour.rs` that draws a flagged
   chunk in a Zone race and asserts `0.5 * albedo * ambient`, not the Zone
   colour (fails if the flag or the ambient switch is dropped).
6. Picture: `oag-game <hdfury image> --race --mode zone --track
   /data/environments/03_track/track.vex --pose=-350,6.1,-405.6,0 --ticks 0
   --screenshot x.png` against the original at the same pose (Racebox ZONE,
   WEAPONS OFF, Moa Therma, craft placed at `-350, 6.1, -405.6` after the HUD
   is up): two speed pads right of the craft, grey textured plates with a dark
   outline and cyan tiles, no white bar.

## Open

- Whether the Zone surface itself (unflagged chunks) is multiplied by a
  lightmap term in ours: the original's Zone word is `Ambient` on every draw.
  Check `shade.wesl`'s light for a lightmapped chunk under `zone_on` while in
  there.
- The start gantry's card (`ZONE_EXEMPT_RATE`, chosen): `sign_emissive_glow`
  has 54 bit-11 chunks, so the stand-in is probably this flag. Check the
  gantry's quad chunk's flags and retire the stand-in if so.
- Weapon pads in a Zone race: bit 10 chunks are scrubbed from the PVS when
  `gameState+0x18` is zero (meaning unread); the WEAPONS OFF frames showed
  speed pads only. Whether ours draws weapon pads in a weapons-off Zone race
  is not looked at.
- Omega's HD-lineage circuits in Zone: not checked (program selection there
  is unread, and ours draws no Zone grade on Omega). 2048 and Omega's
  2048-lineage circuits: checked, differs (`trackZone` authors textured pad
  materials); a 2048 Zone pad frame of ours has not been looked at.
- Bit 11's authored meaning; a later stage's frame with a non-zero sun.

## Next Steps

1. Steps 1 to 3 of the recipe, with step 5's tests.
2. Render step 6's pose before and after, at the size a player sees.
3. Then the gantry check and the Track fog.
