# Sebenco and Vineta K: scalar maps fixed, ice water drawn; the sea's reflection, sky tint and Omega's water stay open

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
- **Vineta's teal behind the glass** (2026-10-07, `hd-water`, rcsmaterial.md "Water family"): `waves2` is a normal map and
  is no longer painted as a picture. The two compared panes do not show the water (skipping it moves them 1 to 4
  levels): their residual is the backdrop, the sky tint and the glass multiply. The sea band beside them went from blue to
  pale `(176,217,219)` against the reference's teal `(15,145,145)`, because the program's reflection times `0x7480de6d`
  is undrawn: HD's binding for `paraboloidReflectionTex` was not found (`PerMaterialEnvMap`/`skyreflect.gtf` is authored
  data the HD executable does not name; 2048 binds a per-environment `skyParaboloid.gxt`).
- **The ice pool's water layer is drawn** (constants lerped by the pond mask and a facing term, `mesh::rcs::ice`). Open:
  the bump-perturbed facing term (the reference's streaks), the program's own specular, and brightness - the reference
  pool is paler (`(155,244,249)`) than ours; no matched camera exists to measure it against.
- **127 slots changed by the pick rule have one reference frame between them** (Sol 2). Candidates worth a frame:
  `glass_2nduv_reflect_glow` on 02_track (18+11 slots), `2rocksandblend_via_diffuse` on Vineta K, `nr_twinblend`.
- Sebenco reverse: the forward and reverse pool captures (`ref`, `ref3`) differ in camera yaw from ours (the craft's
  attitude after `place` is not ours on reverse); landmarks agree, pixels do not.

- **Omega ships the same water materials** (`Water_noref`, `WATER_Test_2`, `Sebenco_ice` in `data01`/`data02`/`data05`/`data08`
  with `_1`/`_2` shader splits; PS4 GNM shaders, so the microcode classifiers here do not run). Checked, applies, not wired: the
  three ice colours are model parameters, so wiring `Sebenco_ice` there is a parameter read plus the same shader branch
  (`slots::ICE`, `GpuVertex::texcoord2`). 2048 differs (`dc_seawater`, no Sebenco ice).

## Next Steps

1. Find how HD binds `paraboloidReflectionTex` (engine parameter table slot 8, `Shader_InitEngineParams`): an RPCS3 draw
   capture of one `water_test_2` draw, or the setter of the parameter entry's `+0x18`. Until then the reflection stays
   undrawn.
2. Take a Sebenco sky capture at a pose with a large open sky and a stationary craft (`--speed 0` placed on a flat
   stretch) and fit the factor against `Fog.Fog Color`/tone before touching `sky_cube::build`.
3. Omega: wire `Sebenco_ice` through the `psp2` material path (colours from the model parameters, mask on `Uv2`), and read
   what `Water_noref`/`WATER_Test_2` declare in its GNM shaders.
4. Ice: perturb `F` by the two normal-map taps (`and_snow_norm` at `TC4.xy`, `256norm4` at `TC4.xy * 0.2444`) and add the
   program's specular; take a matched capture with a recorded camera.
