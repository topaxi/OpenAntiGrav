# Sebenco and Vineta K: scalar maps fixed; ice water, sea colour and the sky tint stay open

2026-10-07, branch `hd-sebenco`. The cause and fix are in `docs/formats/rcsmaterial.md`, "A picture the program reads one
lane wide is a scalar, not the colour".

## Open

- **Sky tint is not a function of `Lighting.Sky colour` alone.** Sebenco Climb and Sol 2 both author 255. Sol 2's cube
  pixels measured x2 against the untinted cube; Sebenco's matched pose (forward, start slot yaw 180, `ref2/01.png` vs
  `runs/fw_1_*`/`tint0.5_1.png`) reads reference 0.78-0.89 against the untinted cube's 0.68-0.74 in the mid rows (ratio
  about 1.2), while our x2 clips every cube pixel to 1.0 (reference: 75% clipped, 10th percentile 0.38). The reference
  sky is near-white as ours is, but not as a flat clip. Differences between the two files that could carry the missing
  factor: Sebenco's `Fog.Fog Color` is HDR `(0.65, 1.18, 1.85)` at density 0.0004, Sol 2's grey 0.25 at 0.002; prelit
  and tone keys differ. Not tuned. Second matched pose (`ref2/02`) is unmatched in view (the craft slid 4.8 units).
- **Vineta's teal behind the glass is the sea, not the sky.** `water_noref` / `water_test_2` draw `waves2.gtf` as the
  picture; their programs sample the engine-bound paraboloid reflection (`0x9edd3243`) and constants we do not evaluate,
  so the sea behind the panes is blue where the reference is teal (teal = bright backdrop x the glass's authored
  `[0.2, 0.6, 0.6]`). A water-family lane.
- **The ice pool's water layer** (constants lerped by the pond mask, fresnel, `0x9edd3243` reflection) is not drawn; the
  pool is snow-white where the reference has cyan streaks (`pair_01.png`).
- **127 slots changed by the pick rule have one reference frame between them** (Sol 2). Candidates worth a frame:
  `glass_2nduv_reflect_glow` on 02_track (18+11 slots), `2rocksandblend_via_diffuse` on Vineta K, `nr_twinblend`.
- Sebenco reverse: the forward and reverse pool captures (`ref`, `ref3`) differ in camera yaw from ours (the craft's
  attitude after `place` is not ours on reverse); landmarks agree, pixels do not.

## Next Steps

1. Water family: decode `water_noref` block, wire constants, leave the reflection undrawn and say so in the loader report.
2. Take a Sebenco sky capture at a pose with a large open sky and a stationary craft (`--speed 0` placed on a flat
   stretch) and fit the factor against `Fog.Fog Color`/tone before touching `sky_cube::build`.
