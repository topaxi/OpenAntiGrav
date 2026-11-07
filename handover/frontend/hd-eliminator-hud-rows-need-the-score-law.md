---
status: ready-to-wire
titles: [hd, omega, 2048]
updated: 2026-11-07
summary: "HD Eliminator is scored and its rows draw; the results screen's POINTS table, the TARGET row and ScoreFeedBack popups are open"
---

# HD's Eliminator: what is left after the score law

2026-11-07, `hd-score-law`. Law, evidence and departures:
`docs/ghidra/functions/ps3-hdfury-eu/eliminator-score.md`. Guarded by
`crates/race/src/score.rs`, `crates/raceplay/src/tests/eliminator.rs` and
`crates/raceplay/tests/hd_eliminator_hud_ground_truth.rs`.

## Open

- **Results screen.** The original ranks by `POINTS` under `2ND PLACE - KILLS: 6 DEATHS: 0`
  (`data/reference/hd-eliminator-score/results.png`, local). Ours draws `oag_raceplay::scoreboard`'s
  places under `Headline::Unresolved`.
- **Racebox TARGET row.** HD's row is a `Slider` (`200..600`, step `20`). `race_setup` is `None`, so
  every Racebox race runs on the minimum.
- **`ScoreFeedBack0-15`.** Authored and read, never caught in a frame.
- **Opponent lap refill.** Ours refills every craft; on HD an AI's shield did not move on any lap.
- **Not captured:** the split-screen Eliminator pair (`splitscreen_elimination_hud_{0,1}.xml`, same
  widgets, unchecked).
- **2048 and Omega: checked, applies, not wired.** Both keep the kill rule, inherited from Pulse.
  Their score writers are unread, and Omega's named file has no `<EliminatorAIStats>` row.
- **`HighlightBar`'s width** needs a font in the HUD's draw list.
- **Ours reaches the target about four times sooner.** The original's Novice race took ten laps
  to reach 200; ours on the same circuit and seed-free setup was over by tick 6000, about three
  laps. The awards match row for row, so the likely cause is how often our field lands a kill,
  not the law. Unmeasured. Re-measure the pace first: pads stopped disarming after the 3-laps
  reading, so the supply went up.
- **`--race` ends on this build's own board** (`RACE COMPLETE`, the person first). That is the
  capture path; the front end's results are the first item above.

## Next Steps

1. Give `Board` the score for a scored race and draw `POINTS` in HD's results table.
2. Take a frame burst across one of the person's hits for `ScoreFeedBack`; read `hud+0x3d0 + 12 * i`.
