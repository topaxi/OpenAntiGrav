# Opponents fly their own team's handling, so the AI needs a speed plan per hull

Every opponent on every title now flies its own team's `handlingstats.xml` for the race's class
(`oag_raceplay::load::family::opponent_handling`, `Setup::opponent_handling`); before, every
opponent flew the player's numbers (`start.rs`). HD/Fury opponents also fly the player's exact
variant (`TeamVariants::opponents_match_player_variant`, static read, confidence 55:
`docs/ghidra/functions/ps3-hdfury-eu/opponent-handling.md`; a live RPCS3
check is open). Campaign opponents (`original_ai`) take the same handling, the controller reads none.
Facts: [`hd-status.md`](../../docs/formats/hd-status.md), "Opponents follow the player's hull family".

The AI's speed plan was learned on one craft. It is now learned once on the **player's** craft
(else the most agile AI hull whose plan verifies) and each opponent follows it at
`min(1, own yaw ceiling / reference yaw ceiling)` (`oag_ai::Driver::plan_deficit`, chosen, not
measured). That keeps every circuit's plan verifying and removes the Eliminated rows, but a hull
unlike the reference still scrapes: the default grid's slot 1 is Triakis (6.0 wide, slowest turn)
and 36 `ai_clean_lap_gate` rows, Black seed 1 (forward) and the stall test's launch tick were
regenerated for it (commit message and `ai_clean_lap_gate.rs` give one reason per row).

## Open

- **A speed plan per hull, cached on disk.** Key: circuit + the hull stats the plan reads
  (handling, dimensions, inertia, class) + a plan version. Built once ever, never at race start;
  until a hull's own plan exists it borrows the cached plan of the most similar hull (turn rate,
  width), and could be built race by race or in the background. Measured cost of building at
  load: about 0.7 to 3.5 s per plan in debug, seven distinct hulls a grid, so a per-race build was
  rejected. Per-hull plans were measured as the better fit before the shared one: Triakis' own plan
  verified clean on most layouts but failed on `01`/`07`/`13`/`14_Track` jumps (OffLine after a
  held run-up), which a retry with a lowerable hold did not fix, so the search itself needs work.
- **Read HD's live rival controller.** `AIControlStats.xml` is unread for HD and
  `docs/formats/hd-ai-netconfigs.md` says the `.nnt` networks are dormant; our AI could learn
  from what the original's rival does with the same hulls.
- **Contacts for wide hulls** (`02_Track` FLASH 0 to 328 on Triakis): the plan's tight flags come
  from the reference hull; a per-craft clearance would be cheaper than a whole plan.
- **Per-team handling on Pure, 2048 and Omega** was not exercised with a field; a team whose file
  fails to load keeps the player's handling and the load report says so.
- **Shield with a field before and after** was measured on Pulse `01_Track`/`03_Track` and HD
  Talons Junction, one seed each.

## Next Steps

1. Build the per-hull plan cache (`oag_ai::plan` + a key in `oag_source`'s derived-cache directory),
   2-3 days.
2. Fall back to the nearest cached hull until a plan exists; measure `ai_clean_lap_gate` with the
   default grid's slot 1 on its own plan, expect the 36 regenerated rows to come back down.
3. Read `AIControlStats.xml` on HD: a decompile-and-read lane, an afternoon.
