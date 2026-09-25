# Tournament is built; Head2Head and five RE gaps are still open

2026-09-23. The mode this thread's own implementation brief asked for is
built: `oag_race::Mode::Tournament`, `oag_race::tournament` (the points
table and the running standings), `crate::race::tournament::Progress` (the
game-layer leg list and totals) and the campaign/EndRace wiring
(`crate::main::session::tournament`, `RaceStage::tournament_final_rank`).
See [`docs/gameplay/race-modes.md#tournament`](../../docs/gameplay/race-modes.md#tournament)
for the full picture and
[`tournament.md`](../../docs/ghidra/functions/psp-pulse-usa/tournament.md)
for the law it implements - neither changed by this pass, both cited rather
than re-derived, per this project's own rule.

Three things this build chose rather than measured, each flagged at its own
site too:

- **A craft that has not finished a leg when it ends scores `0`**, standing
  in for the original's own "destroyed, or race-state 7" guard - a byte
  this engine's `oag_race::Standing` does not carry. See
  `oag_race::tournament::points_for_finish`'s own doc.
- **No save/resume between legs.** `Session::tournament` is discarded the
  moment its `EndRace Menu` is left, whichever option is picked -
  `Tournament_SaveProgress`/`_LoadProgress` are read but not reproduced.
- **No authored standings table.** A leg's `EndRace Results`/`Rewards`
  reuse the same screens every other campaign race draws (the leg's own
  placing, and on the last leg the medal); `EndRaceResults_PopulateTournamentTable`'s
  own `PRO_POS`/`ER_TEAM`/`ER_POINTS` columns are not drawn, because the
  copy into their fields was never located in the decompile either.

## Open

Unchanged by this pass - none of them was forced by the implementation, so
none was answered on a guess:

- **Head2Head entirely.** Deliverable 4 of the pass that read the
  Tournament law was never attempted, and this pass did not touch it
  either - it is a separate, unread question (a two-craft race's own
  opponent count and HUD), not an extension of Tournament's own law.
- **`DAT_08b30fa0`'s write site** - narrowed to the `Team Selection`
  confirm -> `Launch Game` window, not pinned to a function.
- **`DAT_08b31158+0xdc`'s exact meaning** - a "live tournament in progress"
  reading fits every write site seen, unconfirmed.
- **The leg-name-hash to `PI_Track`-pointer resolution** `Tournament_AdvanceLeg`
  appears to read from a different array than `DAT_08b31158`'s own leg
  hashes.
- **`DAT_08b34320`'s own identity and its `+0x90` field's reset condition**
  between tournaments - this build's own "start every craft at zero" is
  chosen for exactly this reason (see above), not a reading of
  `FUN_08820d78`.

## Next Steps

- Whoever reads Head2Head's own law should check whether any of
  `oag_race::tournament`'s points/standings machinery is reusable, or
  whether a two-craft mode needs its own from scratch.
- If a live Ghidra pass ever pins `DAT_08b30fa0`'s writer or
  `DAT_08b31158+0xdc`'s meaning, neither changes this build's own behaviour
  - both are read-only facts about the original nothing here depends on.
- The authored standings table is the one open item that *would* change
  what a player sees: locating the copy into `g_endrace_result`'s own
  `+0x35`/`+0x134` fields is what unblocks drawing `PRO_POS`/`ER_TEAM`/
  `ER_POINTS` for real, in `oag_ui::endrace`, rather than reusing the
  ordinary results screen.

## From the HANDOVER.md index (moved 2026-09-25)

2026-09-23. `oag_race::Mode::Tournament` races a leg exactly like a single race; `oag_race::tournament` implements the points table (8/6/5/4/3/2/1/0 by leg placing) and the standings rank (grid-slot tie-break); the campaign session carries the leg list and running totals across a relaunch (`crate::main::session::tournament`), and the final leg's medal compares the standings rank (`RaceStage::tournament_final_rank`), not a leg's own placing. Chosen, not measured: a non-finisher scores 0 (standing in for the original's own destroyed/race-state-7 guard), no save/resume between legs, and no authored standings table (a leg reuses the ordinary results/rewards screens). Open, unforced by the build: Head2Head entirely, `DAT_08b30fa0`'s writer, `DAT_08b31158+0xdc`'s meaning, the leg-hash-to-track resolution, `DAT_08b34320`'s reset condition
