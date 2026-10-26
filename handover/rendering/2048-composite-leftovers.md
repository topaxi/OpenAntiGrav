---
status: re-first
titles: [2048]
updated: 2026-10-26
summary: "2048 composite: vignette, screen tint, alphaTex, DLC light and fog from the pack's copy, other texel programs"
---

# 2048's race composite: what is still left out

2026-10-26, `2048-exposure-wire`. A 2048 race now ends in its own `wo_*` chain
(`oag_post::wo_composite`): gamma-2 scene, gate, two diagonal blurs, composite,
and the exposure adapting in play (`ExposureScale / mean luminance`, the link
closed with evidence) or held at 1.15 for the compare pairs (`--hold-exposure`).
The law, the closed link and the frame means are in
[`docs/ghidra/functions/vita-2048-eu-v104/exposure.md`](../../docs/ghidra/functions/vita-2048-eu-v104/exposure.md);
the compare-side switch in
[`docs/tools/compare-page.md`](../../docs/tools/compare-page.md).

Held at 1.15, ours' frame means are 54.9 / 61.4 / 36.7 against the original's
60.2 / 83.1 / 48.1 (Queens Mall, Empire Climb, Vineta K): now a little darker.

## Open

- **The vignette**: `wo_composite_vp`'s third output and the `vignetteTint`
  values `PostFx_RenderChain` packs (`local_348`/`local_344`); the composite is
  drawn as a mesh whose `pos.w` carries it. Ours draws `v = 1`.
- **The screen tint and `alphaTex`**: the tint's source (`FUN_813b1d28`) and
  what the second texture holds in a race. Ours draws tint 0 and `c = main`.
- **A DLC circuit's light and fog take the base copy** of its
  `track.EnvSettings`; only the five exposure keys prefer the pack's. On
  Vineta K the pack's sun diffuse is twice the base's, so ours lights it too
  dimly. Generalising `envsettings_exposure`'s pack preference to
  `envsettings_light`/`envsettings_fog` is the fix; check Omega's DLC circuits
  read the same way.
- **Which other 2048 programs square their texel**: only the lightmapped track
  program is read. Ours squares every pipeline built for the scene's store
  (meshes, clouds, exhaust, beams, ghosts); `oag-fx`'s particles, mist, flash and
  weapon quads keep 2.2. Read the ship, sky and particle programs the same way
  (Vita3K's SPIR-V by the container's SHA-256).
- **Empire Climb's track and walls draw flat** in ours; that, not exposure, is
  most of its remaining gap.
- **The default 2048 race's near-white track at tick 1500 did not reproduce**
  (2026-10-26, `w2048-white-floor`, main `9c166f405`): `just play 2048 --race
  --no-audio --ticks 1500 --screenshot` on a clean profile and on a copy of the
  maintainer's `settings.toml` both draw Altima's floor as dark grey panelling
  with bright pads (mean grey 86 over the floor box, rows 450-800 and columns
  0-1000 of the 1440x816 frame). No material is white. If it shows in a
  window, the cause is not the floor's materials: capture that frame's
  settings, resolution and exposure state (the adapting exposure runs per
  rendered frame, so a slow window may differ from `--ticks`) before reading
  any program.
- The Zone race's composites (`0x816a4d80`, `0x816a4d84`): every race of ours
  runs `wo_composite_nocc_fp`.
- The step runs once per rendered frame at 60 Hz; the original steps per
  frame it renders (about 30 on Vita3K). A labelled choice.

## Next Steps

1. Pack preference for the light and fog keys (an hour with a disc-backed
   test on Vineta K), then re-render the DLC pair.
2. Read `vignetteTint` and the composite's vertex program; wire `v`.
3. Adopt the three re-rendered 2048 pairs (the lead's command in the lane
   report) so the page shows "exposure held at 1.15".
