---
status: re-first
titles: [hd, omega, 2048]
updated: 2026-11-25
summary: "HD, Omega and 2048 Eliminators are won on score and drawn by their own layouts; per-pad cycle, 2048 end test, ScoreFeedBack open"
---

# HD's Eliminator: what is left after the score law

2026-11-07, `hd-score-law`. Law, evidence and departures:
`docs/ghidra/functions/ps3-hdfury-eu/eliminator-score.md`. Guarded by
`crates/race/src/score.rs`, `crates/raceplay/src/tests/eliminator.rs` and
`crates/raceplay/tests/hd_eliminator_hud_ground_truth.rs`.

## Open

- **Pads' per-pad cycle** (2048, Omega): each pad's start phase and the dark cooling colour are not
  drawn, because a class shares one glow entry and the table holds 64 (`docs/rendering/pads.md`,
  "2048 and Omega").
- **2048's event end test.** Ours ends an Elimination event on the elite score (chosen); the
  executable's tiered target state and end test are unread (`docs/formats/2048-campaign.md`, "The
  objective law"). Which events are tiered also decides whether the rows show at all.
- **Omega's own writer of the 2048-era elimination HUD** is unread; ours runs 2048's law there.
- **2048's opponent scale table:** which of the two the race reads is unconfirmed; ours takes the
  loaded `WeaponAIStats` row.
- **Pace on 2048 and Omega** is HD's, tuned on HD alone.
- **Flip outside an Eliminator** (`race_eliminator_flip`, Omega's event `+0x21c`), 2048's stick
  trigger and the opponents' flip (`eliminator-flip.md`, "Per title").
- **The 2048 and Omega lane's play report:** which Omega circuit the maintainer flew is unknown.
  Both layouts now head the race with its score.
- ~~**Racebox TARGET row.**~~ Landed 2026-11-09: `race.score_target` offers the slider's 21 steps and
  reaches `Options::eliminator_score_target` (`eliminator-score.md`, "What ours does"). Omega offers
  its own slider's `50..600` by `25`; 2048 has no slider and no row.
- **`ScoreFeedBack0-15`.** Authored and read, never caught in a frame.
- **Opponent lap refill.** Ours refills every craft; on HD an AI's shield did not move on any lap.
- **Not captured:** the split-screen Eliminator pair (`splitscreen_elimination_hud_{0,1}.xml`, same
  widgets, unchecked).
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
