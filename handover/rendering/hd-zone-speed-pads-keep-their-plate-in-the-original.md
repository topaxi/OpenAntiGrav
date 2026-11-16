---
status: measure
titles: [hd, omega, 2048]
updated: 2026-11-16
summary: "HD Zone speed pads: original keeps the grey plate, no white line; ours paints it road-teal with a white bar. Cause open"
---

# HD's Zone speed pads keep their authored plate in the original; ours paints them road-teal with a white bar

2026-11-16 (`zone-pad-line`). A maintainer report from play: on Wipeout HD
Fury, a Zone race on Moa Therma, every speed pad shows a horizontal white line
across it.

## Measured

- **Ours reproduces it** (confidence 95). `oag-game <hdfury image> --race --mode zone
  --track /data/environments/03_track/track.vex --autopilot --ticks 580 --screenshot x.png`
  (Moa Therma is `03_track` on HD, Metropia `02_track`): two pads ahead, each a
  teal plate (the colour of the road) with the cyan tile block and a white bar along the
  far side of the tiles that runs past the pad's edge. Same pose, `--mode time_trial`: no
  such bar, a grey pad plate.
- **The original has no line** (confidence 85, six frames over two races of one
  RPCS3 boot). RPCS3 Racebox, RACE TYPE `ZONE` (four Rights from the first row on a
  fresh save), Track Select two Rights (MOA THERMA), then
  `rpcs3-drive.py place` at x = -372 .. -340, z = -405.6 (the pads sit at x about -330,
  z about -402), settle 0.35 s. At 10 to 40 units from a pad the plate is **light grey
  with the pad's own texture** (a bevelled tile grid, a black outline, a green edge
  line, a mesh insert), the tiles cyan, no white bar. It is not road-teal.
  The pad is therefore **not** drawn as a plain Zone surface in the original.
- The original's pad is the same pad as the lit one (`time_trial` frame), tinted:
  the plate is lighter than ours' lit plate in a Zone race (ours' lit pad is dark
  there; the original's is mid grey).

## Ruled out, with evidence

- **Wrong Zone pair.** Every `speedup_material` (12) and `weapon_pads` (8) chunk of
  `03_track` has bit 0 of its render flags set, as the road chunks do
  (`diffuse_normal_specular`, `diffuse_specular`): the pads take the Track pair
  already.
- **The material has no Zone variant.** `speedup_material` and `weapon_pads` carry 40
  of 70 variants with `zoneColourTint`, and for the key `HalfBright ZoneMode
  Ambient|IleLightmap Sun Spot0` both resolve to one that declares it, like the road
  materials. The `IleVertex` key has no variant on any of them (road included).
- **The rim uses the wrong normal.** The pad's Zone fragment program (variant at
  `0x8d20` of `03_track`'s `speedup_material`) computes `1 - |N.V|` from the
  interpolated vertex normal `f[TC2]`, as ours does; the `_ne` normal map only feeds
  the lighting.
- **The Zone variant keeps the albedo on pads.** It does not: the pad's program
  samples the albedo (unit 0) for `.w` only, the same as the road's, so neither has an
  RGB albedo term and the grey plate does not come from this variant.
- **Exempting pads from the Zone grade** (a throwaway `zone_weight = 0` for `PAD_NE`
  chunks): the bar goes, the plate becomes ours' dark lit plate. It does not
  match the original's grey plate either, so the exemption alone is not the answer.

## Open

- What puts a mid-grey plate with the pad's own texture on the pad in the
  original. Candidates left: the pad chunks resolve a Zone variant under a key whose
  program differs from the one probed here (the frame-determined axes
  `Sun`, `ShadowMap`, `Spot<n>`, `IBL`, `ShadowToAlpha` are unrecovered per chunk,
  `docs/formats/rcsmaterial.md`); or `zoneColourTint.xy` and the zone texture's
  sampling at a pad's UV (`zoneUV = tint.xy * (1 - uv)`) are not what the road's
  tiling needs. Next: dump the pad draws of a paused RPCS3 Zone frame
  (`scripts/rpcs3-hd-whiteout.py`'s `--want-mats speedup_material` dump, `scripts/rsx_fp_match.py`)
  and read which fragment program the pad runs and with which constants.
- Omega's and 2048's Zone pads: not measured (no original frame). Ours draws them
  through the same Zone path in `shade.wesl`, so a fix lands for all three; the
  Omega/2048 check is owed with it (CLAUDE.md, "A 2048 finding is checked against
  Omega").
- Weapon pads in Zone: not captured; the same material family.

## Next Steps

1. Paused RPCS3 Zone frame on Moa Therma at a pad, dump the draws, name the pad's
   fragment program (`rsx_fp_match.py`) and its constants.
2. Port that program's picture for the pad chunks only; test against the chunk's
   program hash; render the pose against `place` frames.
