---
status: ready-to-wire
titles: [hd, omega, 2048]
updated: 2026-11-09
summary: "HD Eliminator is scored, drawn, ranked on POINTS and its Racebox TARGET row sets the target; ScoreFeedBack popups are open"
---

# HD's Eliminator: what is left after the score law

2026-11-07, `hd-score-law`. Law, evidence and departures:
`docs/ghidra/functions/ps3-hdfury-eu/eliminator-score.md`. Guarded by
`crates/race/src/score.rs`, `crates/raceplay/src/tests/eliminator.rs` and
`crates/raceplay/tests/hd_eliminator_hud_ground_truth.rs`.

## Open

- ~~**Racebox TARGET row.**~~ Landed 2026-11-09: `race.score_target` offers the slider's 21 steps and
  reaches `Options::eliminator_score_target` (`eliminator-score.md`, "What ours does"). Omega's own
  slider is `50..600` by `25` and its rule is kills, so the row is HD's alone until Omega's awards are read.
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
- **Results table, landed:** `0x00229bf0`, `docs/ghidra/functions/ps3-hdfury-eu/eliminator-score.md`
  ("The results table"). Open in it: how the original orders equal points, which function calls the
  filler, and the second colon `ER_DEATHS` would add (ours drops the string's). A networked board
  (`oag_net::wire::board::BoardRow`) carries `eliminations` only, no points.
- **`--race` ends on this build's own board** (`RACE COMPLETE`, the person first). That is the
  capture path; the front end's results are landed (below).

## Next Steps

1. ~~Racebox TARGET row as HD's `Slider`~~ landed.
2. Take a frame burst across one of the person's hits for `ScoreFeedBack`; read `hud+0x3d0 + 12 * i`.
