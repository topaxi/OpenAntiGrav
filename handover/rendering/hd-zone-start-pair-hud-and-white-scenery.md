---
status: re-first
titles: [hd]
updated: 2026-11-19
summary: "HD Zone start pair (hd/zone-vineta-k-z5) at 28.77 after the sky dome, light-bar and pre-GO HUD gate fixes; white scenery and sky exposure remain"
---

# HD Zone start pair: what is left after the sky dome and light-bar fixes

The adopted pair `hd/zone-vineta-k-z5` (Vineta K, Zone start, stage 1 Sub
Venom) moved 53.04 -> 36.83 mean grey (rmse 0.322 -> 0.219) on two fixes:
the Zone gradient dome as the sky
(`docs/ghidra/functions/ps3-hdfury-eu/zone-sky.md`, "The dome's colours, and
the dome is what the frame shows") and no `_ne` light bar on the Zone surface
(`docs/rendering/hd-zone-recolour.md`, "The light-bar term in a Zone race").
A later zone-equaliser fix took it to 30.02, and the Zone HUD's start gate
(`docs/ghidra/functions/ps3-hdfury-eu/hud-start-gate.md`: no Zone HUD until
the release) to 28.77 (rmse 0.165).

## Open

- **Which countdown moment the original's frame is.** The gantry board shows
  a glyph ours does not (ours' board assembly starts at tick 63). The live
  read for the HUD gate (`docs/ghidra/functions/ps3-hdfury-eu/hud-start-gate.md`)
  saw the same `3` glyph in phase 1, the countdown, so the frame is inside the
  countdown; the HUD gate is phase-based, so it needs no re-timing. The pair's
  `--ticks 0` may be earlier than the original's moment; pinning it needs the
  countdown timer read in the same pause (`RaceManager +0x1958 - +0x1968`).
  A pair-timing question for the lead, not a board law.
- **White scenery.** The original draws Vineta K's buildings and billboards
  near white (`Scene.Texture Colour` 0.72/0.91/0.96 plus rim terms); ours
  shows some of them pale grey-brown with window texture, and the whole frame
  darker (bias -15 to -22 grey outside the sky). Unread which chunks.
- **Sky exposure.** Ours' dome reads about 17 % brighter than the original's
  at eye level, uniformly.
- **The dome's band is flatter than read.** The frame rises less toward
  the zenith than `tan(e) / 0.75` (zone-sky.md); the band geometry stays at
  the static 82.
- **2048 on HD-lineage circuits draws the same grey Flash sky** (zone-sky.md,
  "2048 and Omega"): checked, applies, not wired; the port draws the circuit
  dome there. Omega's Flash frame shows peach instead: differs, unread.
- **The adopted pair's provenance says zone 1, score 0 were read off the
  original's HUD**, but the frame draws no HUD; the values are the race's
  start state, not a reading.
- **`_ne.a * zoneBaseInner`.** Two of Vineta K's
  `diffuse_normal_specular_emmissive` Zone variants add it; which variant a
  chunk runs is unread, and the frame shows none.

## Next Steps

1. Adopt the HUD-gate fix on the pair: drop `--hud-state 1,1,0,0,0,100,0,1,0,0`
   from `hd/zone-vineta-k-z5`'s `ours.flags`, then
   `just compare-rerender-ours --only hd/zone-vineta-k-z5 --no-build` (28.77
   expected; with the flag the imposed HUD state keeps the HUD drawn, 30.02).
2. Read the countdown timer beside the pair's grab and re-time ours'
   `--ticks` if the moments differ (proposal to the lead; the pair is adopted).
