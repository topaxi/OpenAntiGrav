---
status: ready-to-wire
titles: [hd, omega, 2048]
updated: 2026-11-25
summary: "HD Zone plain chunks wired, 2048/Omega pads inherit it; open: Track fog, gantry card stand-in, unflagged-surface lightmap"
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

Steps 1 to 3 and 5 to 6 landed (branch `zone-pad-wire`): `RENDER_ZONE_PLAIN` and `Mesh::zone_plain()`
in `render_block.rs`; the carrier is `slots::ZONE_PLAIN` (bit 5), set per chunk in `mesh::rcs::surface`; `shade.wesl` drops the Zone surface and
lights the chunk `ambient + sat(N.L) * sun`. Tests and the pad-plate numbers are on
`docs/ghidra/functions/ps3-hdfury-eu/zone-plain-chunks.md`, "Against this port". Step 4 is still open:

4. Fog (second, small at pad distance): a track-flagged chunk takes the stage's
   `Lighting.Track Fog colour`/`density`, which `ZoneGrade::fog` does not read (it reads the primary
   pair for everything).

## Open

- Whether the Zone surface itself (unflagged chunks) is multiplied by a
  lightmap term in ours: the original's Zone word is `Ambient` on every draw.
  Check `shade.wesl`'s light for a lightmapped chunk under `zone_on` while in
  there.
- The start gantry's card (`ZONE_EXEMPT_RATE`, chosen): `sign_emissive_glow`
  has 54 bit-11 chunks, so the stand-in is probably this flag. Check the
  gantry's quad chunk's flags and retire the stand-in if so.
- HD's weapon pads in a Zone race: bit 10 chunks are scrubbed from the PVS when
  `gameState+0x18` is zero (meaning unread); the WEAPONS OFF frames showed
  speed pads only. Ours draws none: HD's weapon pad model is gated by the mode,
  and 2048's and Omega's are dropped by the loader (`docs/gameplay/pickups.md`,
  "A weapons-off race draws no weapon pad", 2026-11-25).
- Omega's and 2048's HD-lineage circuits in Zone: their pads now keep their own
  program, inherited from HD (2026-11-25, `zone-plain-chunks.md`, "2048 and
  Omega"); which program Omega's executable runs for them is unread.
- Bit 11's authored meaning; a later stage's frame with a non-zero sun.

## Next Steps

1. The gantry check (are its card chunks among the 54 `sign_emissive_glow` bit-11 chunks? then retire `ZONE_EXEMPT_RATE`) and the Track fog.
2. A pad frame of ours against the original at a matched camera (the original's was at its settled pose with a wider view; ours differs by about 15 % in plate brightness).
