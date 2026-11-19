---
status: re-first
titles: [hd]
updated: 2026-11-19
summary: "HD Zone start pair (hd/zone-vineta-k-z5) at 36.83 after the sky dome and light-bar fixes; the pre-GO HUD gate, white scenery and sky exposure remain"
---

# HD Zone start pair: what is left after the sky dome and light-bar fixes

The adopted pair `hd/zone-vineta-k-z5` (Vineta K, Zone start, stage 1 Sub
Venom) moved 53.04 -> 36.83 mean grey (rmse 0.322 -> 0.219) on two fixes:
the Zone gradient dome as the sky
(`docs/ghidra/functions/ps3-hdfury-eu/zone-sky.md`, "The dome's colours, and
the dome is what the frame shows") and no `_ne` light bar on the Zone surface
(`docs/rendering/hd-zone-recolour.md`, "The light-bar term in a Zone race").

## Open

- **The Zone HUD before GO** (about 7 grey on the zone list and gauge). The
  original's frame (lap clock 0.0) draws no Zone HUD; ours at `--ticks 0`
  (race clock -4.53 s) draws the zone list, header and gauge. Not found in
  `RaceManager_UpdateCountdown` (`0x00057690`): it writes bytes `+0x68`
  (countdown running) and `+0x69` (released) of `*(*0x008a6890 = 0x0098767c)
  + 0x40` and calls `RaceManager_SetPhase(2)`; the readers of those two bytes
  are the next thing to read, then the per-player HUD reset `0x000cf970` that
  phase 1 calls (`countdown-voice.md`). `Hud_Update` (`0x0009e3d0`) is 1,400
  decompiled lines and gates per widget; no whole-HUD gate seen in its first
  part. Whether the gate is Zone-only is unknown: no HD pair sits in a
  countdown outside Zone.
- **Which countdown moment the original's frame is.** The gantry board shows
  a glyph ours does not (ours' board assembly starts at tick 63). The pair's
  `--ticks 0` may be earlier than the original's moment; pinning it needs the
  countdown timer read in the same pause (`RaceManager +0x1958 - +0x1968`).
  A pair-timing question for the lead, not a board law.
- **White scenery.** The original draws Vineta K's buildings and billboards
  near white (`Scene.Texture Colour` 0.72/0.91/0.96 plus rim terms); ours
  shows some of them pale grey-brown with window texture, and the whole frame
  darker (bias -15 to -22 grey outside the sky). Unread which chunks.
- **Sky exposure.** Ours' dome reads about 17 % brighter than the original's
  at eye level, uniformly.
- **`_ne.a * zoneBaseInner`.** Two of Vineta K's
  `diffuse_normal_specular_emmissive` Zone variants add it; which variant a
  chunk runs is unread, and the frame shows none.

## Next Steps

1. `get_xrefs_to` the byte fields `+0x68`/`+0x69` of the object at
   `0x0098767c + 0x40` in the PS3 executable; find the HUD draw's gate.
2. Read the countdown timer beside the pair's grab and re-time ours'
   `--ticks` if the moments differ (proposal to the lead; the pair is adopted).
