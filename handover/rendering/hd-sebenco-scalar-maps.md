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
- **Vineta's teal behind the glass is not the water** (2026-10-07, `hd-water`, rcsmaterial.md "Water family"): `waves2` is a
  normal map and is no longer painted as a picture, which moves the hex-window pixels by 2 to 6 levels of the 30 that
  separate us from the reference. The panes see the sky cube; the teal needs the sky lane's open item and the glass
  multiply. `paraboloidReflectionTex` for `water_test_2`/`water_noref` is undrawn: HD's binding for it was not found
  (`PerMaterialEnvMap`/`skyreflect.gtf` is authored data the HD executable does not name; 2048 binds a per-environment
  `skyParaboloid.gxt`).
- **The ice pool's water layer is drawn** (constants lerped by the pond mask and a facing term, `mesh::rcs::ice`). Open:
  the bump-perturbed facing term (the reference's streaks), the program's own specular, and brightness - the reference
  pool is paler (`(155,244,249)`) than ours; no matched camera exists to measure it against.
- **127 slots changed by the pick rule have one reference frame between them** (Sol 2). Candidates worth a frame:
  `glass_2nduv_reflect_glow` on 02_track (18+11 slots), `2rocksandblend_via_diffuse` on Vineta K, `nr_twinblend`.
- Sebenco reverse: the forward and reverse pool captures (`ref`, `ref3`) differ in camera yaw from ours (the craft's
  attitude after `place` is not ours on reverse); landmarks agree, pixels do not.

## Next Steps

1. Find how HD binds `paraboloidReflectionTex` (engine parameter table slot 8, `Shader_InitEngineParams`): an RPCS3 draw
   capture of one `water_test_2` draw, or the setter of the parameter entry's `+0x18`. Until then the reflection stays
   undrawn.
2. Take a Sebenco sky capture at a pose with a large open sky and a stationary craft (`--speed 0` placed on a flat
   stretch) and fit the factor against `Fog.Fog Color`/tone before touching `sky_cube::build`.
3. Ice: perturb `F` by the two normal-map taps (`and_snow_norm` at `TC4.xy`, `256norm4` at `TC4.xy * 0.2444`) and add the
   program's specular; take a matched capture with a recorded camera.
