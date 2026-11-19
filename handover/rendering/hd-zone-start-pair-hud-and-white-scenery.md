---
status: re-first
titles: [hd]
updated: 2026-11-19
summary: "HD Zone start pair (hd/zone-vineta-k-z5) at 7.42 after the Zone frame's light, bloom block and feedback; the board glyph, the hull's light and a pulse's feedback remain"
---

# HD Zone start pair: what is left after the Zone frame's light and post chain

The adopted pair `hd/zone-vineta-k-z5` (Vineta K, Zone start, stage 1 Sub
Venom) moved 53.04 -> 28.77 mean grey on the sky dome, the light-bar term, the
zone equaliser and the pre-GO HUD gate, and 28.77 -> 7.42 (rmse 0.165 ->
0.085) on the Zone frame itself
(`docs/ghidra/functions/ps3-hdfury-eu/zone-frame.md`): the Zone surface lit by
the stage's ambient instead of the circuit's lightmap (17.04), then the Zone
`HDR and Bloom` block and the resolve's `Bloom feedback` `-0.4` (7.42). The
white scenery, the sky's 17 % and the dome's flat band are closed by those
three. By region after: sky 0.10, scenery 1.93, road and pads 1.50, the board
1.32, the craft 1.92.

## Open

- **Which countdown moment the original's frame is.** The gantry board shows
  a glyph ours does not (ours' board assembly starts at tick 63). The live
  read for the HUD gate (`docs/ghidra/functions/ps3-hdfury-eu/hud-start-gate.md`)
  saw the same `3` glyph in phase 1, the countdown, so the frame is inside the
  countdown; the HUD gate is phase-based, so it needs no re-timing. The pair's
  `--ticks 0` may be earlier than the original's moment; pinning it needs the
  countdown timer read in the same pause (`RaceManager +0x1958 - +0x1968`).
  A pair-timing question for the lead, not a board law. 1.32 grey.
- **The hull's light in a Zone frame.** The original's Zone hull reads
  `(60, 85, 98)` over the craft box, ours `(33, 56, 62)`: 1.92 grey. The hull
  is not a Zone-surface chunk; whether `EnvSettings + 0x4b0` (cleared by the
  Zone arm at `0x003ad8fc`) changes the hull's ambient source is the first
  thing to read (zone-frame.md, "Open").
- **A pulse's history in the feedback term.** The resolve's feedback buffer is
  the zoom ring's history target; ours resolves against the frame's own
  quarter-res scene, so a damage pulse in a Zone race is not in the term.
- **Radial bloom.** A Zone race enables it (contribution `0.04`); the port has
  no radial pass.
- **Detonator and Zone Battle** take their own arms of the same block
  (zone-frame.md, "The Zone arm"); `oag_hd::race::ZONE_FRAME` is applied in
  Zone alone.
- **2048 on HD-lineage circuits draws the same grey Flash sky** (zone-sky.md,
  "2048 and Omega"): checked, applies, not wired; the port draws the circuit
  dome there. Omega's Flash frame shows peach instead: differs, unread.
- **2048 and Omega have no Zone frame law read.** Both end the Zone frame in
  `wo_composite_zone_fp` (Omega also `wo_composite_zone_hdfury_fp`), unread.
- **The adopted pair's provenance says zone 1, score 0 were read off the
  original's HUD**, but the frame draws no HUD; the values are the race's
  start state, not a reading.
- **`_ne.a * zoneBaseInner`.** Two of Vineta K's
  `diffuse_normal_specular_emmissive` Zone variants add it; which variant a
  chunk runs is unread, and the frame shows none.

## Next Steps

1. After this lands, re-render the pair from the main checkout:
   `just compare-rerender-ours --only hd/zone-vineta-k-z5 --no-build` (7.42
   expected).
2. Read the countdown timer beside the pair's grab and re-time ours'
   `--ticks` if the moments differ (proposal to the lead; the pair is adopted).
3. The hull: dump one Zone frame with `scripts/rpcs3-hd-zone-frame.py`, match
   the hull draws' programs (`scripts/rsx_fp_match.py`) and read which ambient
   variant they are.
