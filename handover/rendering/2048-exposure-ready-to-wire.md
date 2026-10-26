# 2048's exposure and race composite: recovered, ready to wire

2026-10-26, `2048-exposure` (a measure lane). The law, its addresses and the
live evidence are in
[`docs/ghidra/functions/vita-2048-eu-v104/exposure.md`](../../docs/ghidra/functions/vita-2048-eu-v104/exposure.md).
Nothing of it is drawn by ours yet: a 2048 race has no post chain at all
(`environment::staging` builds `hd_bloom` only from HD's `HDR and Bloom` keys
and `omega_tonemap` only for `GeometryKind::Ps4`), and its lit programs work in
gamma space.

## What the lane found

- **The premise was half an artifact.** The "47.9 / 170.4" frame means averaged
  ours' alpha channel in. Alpha off, original against ours: Vineta K 48.1 / 80.1,
  Empire Climb 83.1 / 107.7, Queens Mall 60.2 / 106.2, so ours is 1.3 to 1.8x
  brighter, not 3.5x. `compare_env.py`'s mean grey difference and RMSE were never
  wrong (alpha off gives the same numbers); `docs/tools/compare-page.md` is fixed.
- **2048's race frame ends in `wo_composite_nocc_fp`**:
  `out = sqrt(e L / (1 + L) + BloomFactor * bloom) * vignette + tint`,
  `e = E * (alpha.rgb + (1 - alpha.a) * main)`, `L = dot(e, (0.3, 0.59, 0.11))`,
  with E adapted once per frame toward `ExposureScale / a` (`a` = mean alpha
  byte of the gate target / 64) at `ExposureSpeed`, clamped at `ExposureMax`.
- **The scene buffer is gamma-2 linear**: a lightmapped track program squares
  its diffuse texel and multiplies the raw lightmap scale-bias in; the composite's
  `sqrt` is the encode.
- **Vita3K draws every 2048 frame at E = 1.15**: it never writes the gate target
  back to guest memory, so the adaptation never runs. Filling that memory by hand
  makes E adapt exactly as read. Every 2048 pair is a reference for E = 1.15.

## What wiring needs (an afternoon to a day, not short)

1. **Gamma-2 decode on 2048's diffuse fetches** (square the texel; lightmap
   left raw), and a linear float scene target for a 2048 race. The other 2048
   programs (ship, sky, particles, adverts) need the same check: does each
   square its colour texel? Read them the same way (Vita3K's SPIR-V by the
   container's SHA-256, walked by hand).
2. **The composite** as an `oag_post` pass, per-title data: the `Lighting.*`
   keys read through `staged_envsettings`-style lookup, a DLC circuit's from its
   pack (Vineta K's live values are the `dlc1.psarc` copy's).
3. **The gate and blur bloom** (`wo_bloom_gate_fp` is read; `wo_blur_ds_fp`,
   `wo_blur_fp` and `bloomScaleBias` are not).
4. **The adaptation**: per original frame, not dt-scaled; Vita3K updates about
   30 times a second. Running it at 60 Hz in ours is a choice to label.
5. **Omega**: checked, differs (it replaced this chain with `Tonemap.*`, which
   ours runs, and its frames already match). Nothing to forward-port.

## The comparison problem (the lead's call, not decided here)

A faithful ours adapts E; the references are frozen at 1.15. Two options:

- **A caveat on every 2048 pair** ("original drawn at E = 1.15, Vita3K does not
  read back the gate target") and compare ours at its own adapted E.
- **A labelled ours-side E hold for probes** (a dev flag pinning E to 1.15) so a
  pair compares like for like, never the default.

## Open

- The vignette (`wo_composite_vp`'s third output, `vignetteTint`), the screen
  tint's source, `accumFactor`.
- When `FUN_81071300` resets E to 1.15 (`0x81071470`).
- What a running Vita settles at: inferred about 0.3 to 0.8 on the three pairs,
  not measured. A capture path that performs the readback would settle it;
  Vita3K's `memory-mapping: external-host` lost the device on this machine's
  RADV (2026-10-26), so do not try the mapping modes unattended.

## Next Steps

1. Read the ship, sky and particle programs' texel handling (step 1 above).
2. Wire steps 1, 2 and 4 together behind the per-title data and re-render the
   three 2048 pairs with `scripts/compare-rerender-ours.py --out <scratch>`.
3. Decide the comparison problem before quoting any brightness distance.
