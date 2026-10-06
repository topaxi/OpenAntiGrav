# 2048's sky turns and its fog draws on HD's curve; the 2048 law, two sky keys and Omega's pink road are open

2026-10-06. 2048 (Vita) and Omega (PS4) circuits now turn their sky dome by the
authored `Lighting.Sky rotation` and draw fog from the authored
`Lighting.Fog colour` (rgb, fourth number as density). See
[2048-sky.md](../../docs/formats/2048-sky.md),
[envsettings.md](../../docs/formats/envsettings.md),
[2048-status.md](../../docs/formats/2048-status.md),
[omega-status.md](../../docs/formats/omega-status.md) and
`crates/game/tests/psp2_sky_fog_ground_truth.rs` (two tests on each of the ten
base circuits). Omega: **ported**, same code branch, identical fog bytes, its
own six-face sky cube and rotation.

## Open

- **The fog curve is HD's, inherited.** 2048's own fog term lives in its
  shader microcode (GXP), which is unread. The loader report and docs label it
  INHERITED. Reading the GXP fog term would replace it with a measured law.
- **The sky rotation's sign is HD's choice, chosen not measured.** The unit
  (degrees) is measured off Anulpha Pass, authored 230 in HD and -130 in 2048.
- **`Sky brightness` and `Sky height offset` are authored and read by
  nothing**: their consumer is unlocated.
- **The fog region ladder and `Depth Fog Offset RecipRange` are unwired.**
- **No 2048 reference capture exists** to judge the picture against. As played,
  the fog is a faint haze on sol and bridge and nothing on altima, and the dome
  is rarely in view from the grid, so the rotation does not show in start
  frames.
- **Omega's altima road draws flat pink at the grid.** Found during this lane
  and not caused by it (it was there before). Not investigated.

## Next Steps

1. Read 2048's fog term out of a circuit material's GXP program
   (`handover/tooling/2048s-gxp-containers-decode-the-usse-stream.md` has the
   decoder state).
2. Look into Omega altima's pink road: a missing texture or a material family
   no reader handles is the first guess, unverified.
