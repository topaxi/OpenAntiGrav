---
status: ready-to-wire
titles: [hd, omega, 2048]
updated: 2026-11-09
summary: "HD Eliminator is scored, drawn, ranked on POINTS, sets its target and runs at the original's pace; ScoreFeedBack popups and difficulty shield are open"
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
- **Shield by difficulty: a measured cross-mode defect, not wired here.**
  - The data: HD's `DATA02.PSARC` `/data/ships/<team>/handlingstats.xml` authors
    `<Misc shield easyshield>`. For Qirex those are `105` and `145`.
  - What ours does: `oag_gameplay::handling` reads `Misc::shield_for(DEFAULT_SKILL_LEVEL)` with
    `DEFAULT_SKILL_LEVEL = 1`, the medium slot, at every difficulty.
  - What the original does: its Novice field started at 135 to 145 live
    (`docs/formats/handling-stats.md`, "Ours races every difficulty on the medium slot").
  - Wiring the race difficulty into `shield_for` moves every mode, Single Race included, so it is
    its own lane, to be raised with the maintainer. Pulse authors `easyshield` too.
- **`ScoreFeedBack0-15`.** Authored and read, never caught in a frame.
- **Opponent lap refill.** Ours refills every craft; on HD an AI's shield did not move on any lap.
- **Not captured:** the split-screen Eliminator pair (`splitscreen_elimination_hud_{0,1}.xml`, same
  widgets, unchecked).
- **2048 and Omega: checked, applies, not wired.** Both keep the kill rule, inherited from Pulse.
  Their score writers are unread, and Omega's named file has no `<EliminatorAIStats>` row.
- **`HighlightBar`'s width** needs a font in the HUD's draw list.
- ~~**Ours reaches the target about four times sooner.**~~ Landed 2026-11-09 (`eliminator-pace`,
  `oag_raceplay::score_pace`, `docs/gameplay/race-modes.md`, "The pace of a score Eliminator"). The
  "three laps" reading was taken at `Options`' default Elite. At Novice ours never reached 200,
  because a Novice opponent held a Mine for good. Novice now runs a median of ten laps. Left open:
  - **Lap time.** Our Novice opponents lap Vineta K in about 48 s; the original's took 80 to 100 s
    in an Eliminator. Unexplained.
  - **HD's Repulser runs Pulse's law** (`I:Pulse`), and it was the biggest damage source in ours.
    Its HD law is unread.
  - **Skilled and Elite have no reference.** Only Novice was captured.
- **Results table, landed:** `0x00229bf0`, `docs/ghidra/functions/ps3-hdfury-eu/eliminator-score.md`
  ("The results table"). Open in it: how the original orders equal points, which function calls the
  filler, and the second colon `ER_DEATHS` would add (ours drops the string's). A networked board
  (`oag_net::wire::board::BoardRow`) carries `eliminations` only, no points.
- **`--race` ends on this build's own board** (`RACE COMPLETE`, the person first). That is the
  capture path; the front end's results are landed (below).

## Next Steps

1. ~~Racebox TARGET row as HD's `Slider`~~ landed.
2. Take a frame burst across one of the person's hits for `ScoreFeedBack`; read `hud+0x3d0 + 12 * i`.
