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
- **The marker half turn:** `Hud_UpdateRaceLine` rotates a marker by pi when `craft+0x6ad8` is
  set; the byte is unidentified (it also gates the absorb in `weapon-ai-fire.md`). Markers are upright.
- **Not captured:** the split-screen Eliminator pair (`splitscreen_elimination_hud_{0,1}.xml`, same
  widgets, unchecked).
- **2048 and Omega: checked, applies, not wired.** Both keep the kill rule, inherited from Pulse.
  Their score writers are unread, and Omega's named file has no `<EliminatorAIStats>` row.
- **`HighlightBar`'s width** needs a font in the HUD's draw list.

## Next Steps

1. Give `Board` the score for a scored race and draw `POINTS` in HD's results table.
2. Take a frame burst across one of the person's hits for `ScoreFeedBack`; read `hud+0x3d0 + 12 * i`.
