---
status: ready-to-wire
titles: [hd, omega]
updated: 2026-11-13
summary: "HD Eliminator/NitroBattle medal is the place vs the rung's 1/2/3; wire place, HD flip set, drop medal_law_is_unmeasured"
size: S (70)
difficulty: easy (75)
---

# An HD Eliminator cell's medal is its place on the score board, not its kills

2026-11-13, from the `elim-medal` lane. Evidence and addresses:
`docs/ghidra/functions/ps3-hdfury-eu/cell-medal-award.md`. The maintainer won a Fury Eliminator
cell in ours and got no medal: `Cell::medal_law_is_unmeasured` withholds it on purpose. The law is
now read (82, static):

- **Medal** = the player's place on the end-of-race score board, compared `place <= target`
  (unflipped) against the cell's per-rung `Gold`/`Silver`/`Bronze`, which every HD Eliminator and
  NitroBattle rung authors as `1`/`2`/`3`. Gold 1st, silver 2nd, bronze 3rd, nothing from 4th.
- **`NitroElim*`** is the race-end score target for the rung (already wired as
  `eliminator_score_target`), not a medal threshold.
- **NitroBattle** (`PI_Cell` mode 13) takes the same compare with its race position (70).

## Open

1. **The value.** `crates/game/src/main/race_stage.rs` (`campaign_medal`) passes
   `player_standing().kills` for `CampaignMode::Elimination`. On a cell with
   `difficulty_targets` (HD's per-rung schema) pass the player's 1-based place on the score
   board instead. The original's end-of-race order (`0x0005ac00`'s bubble sort) is: higher
   truncated score first; equal score, **fewer kills** first; equal score and kills, the player
   above an opponent. That differs from `oag_race::score::ranked` (the HUD's rule, later grid
   slot on top), so the results board and the medal may need different tie rules; name the
   choice where a tie decides bronze. Pulse keeps kills (its flat `Gold` is a kill target).
2. **The flip.** `Cell::evaluate_medal_for_difficulty` flips for `Zone | Elimination` (Pulse).
   HD's `PI_Cell_EvaluateMedalForValue` flips for modes 6 and 14 only (Zone, Detonator), never 8.
   Make the flip set title data (ADR-0058), not a name branch. Detonator (`Other("Detonator")`,
   six-figure rungs) needs the flip the day its medal is wired.
3. **Remove the guard.** `Cell::medal_law_is_unmeasured` goes away for HD Elimination; keep or
   drop it for NitroBattle depending on whether NitroBattle races at all
   (`race_mode_for_cell` refuses it today, so `plan_cell` never launches one).
4. **Tests that fail if the law is dropped:** an HD `grid8_3_2` cell awards gold for place 1,
   bronze for 3, nothing for 4 at every rung; passing kills (say 6 kills, place 4) must not
   award gold; an HD Elimination value must not be flipped. The existing
   `crates/game/tests/hd_campaign_ground_truth.rs` elimination block (line ~167, kills per rung)
   and `omega_campaign_launch_ground_truth.rs` (line ~217) assert the old "no medal" behaviour
   and change with it.
5. **Not to wire without a decision, measured:** the saved `SkillLevel` byte is the rung of the
   best medal; a better medal at an easier rung overwrites a harder one, an equal medal keeps
   the harder. Ours (`CampaignRecord::best_difficulty`) never downgrades. Ask the maintainer.

## Next Steps

1. Wire items 1 to 4 in `oag-tables` (flip set) and `oag-game` (value), one change, with the
   regression gate quoted before and after.
2. Optional live confirmation on RPCS3, recipe in the evidence page's "Live check".
3. Omega: checked, applies (same record case, same evaluator and flip set; evidence page, "Omega"). The same `Cell` change covers it; its Eliminator ranks a kill board in ours.
