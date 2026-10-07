# Sebenco and Vineta K: scalar maps fixed, ice water drawn; the sea's reflection and Omega's water stay open

2026-10-07, branch `hd-sebenco`. The cause and fix are in `docs/formats/rcsmaterial.md`, "A picture the program reads one
lane wide is a scalar, not the colour".

## Open

- ~~**Sky tint is not a function of `Lighting.Sky colour` alone.**~~ **Closed 2026-10-08 (`hd-sky-law`)**: the sky is
  `texel * (byte/255)^(1/2.2)`, live-measured on Sebenco, Sol 2, Talon's and Amphiseum (x1.00 at 255); ported as
  `byte / 255`. The x2 was the original's race-start exposure, not a tint. Evidence: renderer.md, "The sky law, read off
  live draws". **Open from it**: a scene-target dump at race clock 0.00.3 to measure the exposure transient (our `--ticks 30`
  and `--ticks 2400` frames are identical, so the adaptation start-up is not modelled), and who ORs the gamma bits
  `0x700000` into the texture ADDRESS word.
- **Vineta's sea** (2026-10-07, `vineta-k-fidelity`; rcsmaterial.md "Vineta K against a draw capture"): HD binds
  `paraboloidReflectionTex` at unit 1 of the `water_test_2` draw, a 512x256 `A8R8G8B8` **render target** (a dual paraboloid of
  the environment, sky above the middle row, teal sea below) readable only with RPCS3's `Write Color Buffers`. Not a disc
  texture, so it is not drawn; the sheet's own vertex-colour multiply was wrong and is fixed. `Sebenco_ice`'s and Sebenco's own
  probe (`and_ice2`, `ds_dualparaboloid_c`) is the same engine slot: a Sebenco capture of one `sebenco_ice` draw would say whether
  its reflection weights (0 on this material) ever matter.
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

1. Closed 2026-10-07: the probe is a runtime render target (see above). Open: what renders into it, and whether its upper
   half is the sky cube through a paraboloid (it looks like it); the lower half has no disc source.
2. ~~Sebenco sky capture~~ done 2026-10-08, see above.
3. Omega: wire `Sebenco_ice` through the `psp2` material path (colours from the model parameters, mask on `Uv2`), and read
   what `Water_noref`/`WATER_Test_2` declare in its GNM shaders.
4. Ice: perturb `F` by the two normal-map taps (`and_snow_norm` at `TC4.xy`, `256norm4` at `TC4.xy * 0.2444`) and add the
   program's specular; take a matched capture with a recorded camera.
