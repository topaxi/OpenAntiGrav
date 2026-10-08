# Tournament and Head2Head are built; four RE gaps are still open

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

## Update, 2026-09-28: the standings table now draws

The one open item that "would change what a player sees" (see Next Steps
below, as this thread stood before this update) is closed:
`EndRaceResults_PopulateTournamentTable`'s own `PRO_POS`/`ER_TEAM`/
`ER_POINTS` columns are read for real now - the copy site
(`Race_BuildEndRaceResult`'s own tournament block) was found, and it turned
out to write **two** tables, not one: the leg's own placings and the
cumulative standings, which the original cycles between every three
seconds (`EndRaceResults_Update`, new this pass). See
[`tournament.md`](../../docs/ghidra/functions/psp-pulse-usa/tournament.md)'s
own dated correction for the decompile and
[`docs/ui/endrace-screens.md`](../../docs/ui/endrace-screens.md)'s new
Tournament section for what draws
(`oag_ui_screens::endrace::TournamentResults`/`tournament_results_draw_list`,
`crate::race_stage::endrace::tournament_results`). **Not yet checked
against a live PPSSPP capture of a real tournament leg ending** - the same
autopilot cost `tournament.md`'s own "Live verification" section
documents, not spent again this pass; `--menu-page
endrace-results-tournament-leg`/`-standings` exercises the drawing code off
a synthetic field instead.

A second, unrelated bug was found while reading this screen's own widgets
closely enough to draw a new table over them: `perfectlap{n}`'s own
`idstring="MSC_PL"` text overlay (nested, unnamed, inside the `<Image>`)
was leaking through on **every** `EndRace Results` capture, tournament or
not - a faint "TP" past every row. Fixed in the same pass, both tables -
see `docs/ui/endrace-screens.md`'s own updated row for the ordinary table.

## Update, 2026-09-28: Head2Head plays

Deliverable 4, the one item this thread's own "Open" list carried since it
opened, is closed: `oag_race::Mode::Head2Head` exists, `oag_game::campaign
::race_mode_for_cell` maps a campaign cell's own `Head2Head` onto it, and a
Head2Head cell now launches a two-craft field (the player plus one AI
opponent - measured off `AICount="1"` on all 23 authored cells, not
designed), weapons locked off, the cell's own laps, and the campaign's
ordinary win-or-nothing medal (`1`/`0`/`0`, also flat across all 23 cells).
See [`head2head.md`](../../docs/ghidra/functions/psp-pulse-usa/head2head.md)
for the law and [`docs/gameplay/race-modes.md#head2head`](../../docs/gameplay/race-modes.md#head2head)
for what implements it - `oag_race::tournament`'s points/standings
machinery was not reusable, since a two-craft single leg needs none of it;
the mode piggybacks on the same `Race_SpawnGrid`-descended field-size
machinery `SingleRace`/`Tournament`/`Eliminator` already use, parametrised
by a new `Mode::opponent_count` rather than the old all-or-nothing
`has_opponents` boolean.

**Chosen, not measured, same footing as Tournament's own gaps above:**
which team the AI opponent flies (`crates/livery/src/lib.rs`'s
`teams_for_slots`, the identical open question `Single Race`'s own seven
opponents already carry), and the sole opponent's grid slot (extrapolated
by composing two separately-measured rules, not independently captured for
a two-craft field).

**Read but not wired**, unlike everything above: the disc's own HUD swap
for this mode - `MSC_EVENT_HTH`'s "track the distance between you and your
opponent", a `HeadToHeadBar` widget both `Hud_BindWidgets` and its per-tick
updater are now fully decompiled - is not drawn by this build. See
`head2head.md`'s own HUD section for the addresses; a player racing a
Head2Head cell today gets no gap readout.

## Open

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

- If a live Ghidra pass ever pins `DAT_08b30fa0`'s writer or
  `DAT_08b31158+0xdc`'s meaning, neither changes this build's own behaviour
  - both are read-only facts about the original nothing here depends on.
- ~~The authored standings table is the one open item that *would* change
  what a player sees...~~ **Done 2026-09-28** - see the dated update above.
  **Live capture done 2026-09-29** (`tournament.md`): layout, points, columns, page cycle match; open are display-name team cells (ours prints the folder id) and the `Default` font.
- Trace `param_1+0x98`'s own writer (`tournament.md`'s own "What is not
  determined", new 2026-09-28) - the leg table's own row order depends on
  it, and this pass drew it as finish order on behavioural evidence, not a
  traced one.
- ~~**Draw Head2Head's own `HeadToHeadBar` HUD swap.**~~ **Done 2026-09-29**
  (`crates/hud/src/head_to_head.rs`, fed by `Race::head_to_head`); the
  field semantics are pinned in `head2head.md` (the bar is a vertical
  connector, `+0xa0` is `Height`, vtable `+0xec` is `GetY`). Static evidence
  only. **Measured live 2026-09-29** (`head2head.md` "Live measurement"): bar, clamps, label, both colours, leading swap confirmed; the rows use the `Default` font and carry names (`1st Goteki 45`/`2nd AAA`), and the player's row throbs (`0x200`). Still open from it: the two row **names** (`craft+0x798`,
  `DAT_08b31774+0x457`), the player-row `0x200` highlight bit, the multiplayer
  branch, and a PPSSPP frame at a known gap to check the bar's size and colours.
- **A live capture (this engine's own front end, or PPSSPP) of an actual
  Head2Head cell launch** - field size, opponent identity, grid slot - was
  not done this pass; the in-process `Race::start` unit test
  (`crates/raceplay/src/tests/spawn.rs`) is the verification this pass
  has, the same "synthetic, not live" shape the standings-table item above
  already carried until its own live check.

## From the HANDOVER.md index (moved 2026-09-25)

2026-09-23. `oag_race::Mode::Tournament` races a leg exactly like a single race; `oag_race::tournament` implements the points table (8/6/5/4/3/2/1/0 by leg placing) and the standings rank (grid-slot tie-break); the campaign session carries the leg list and running totals across a relaunch (`crate::main::session::tournament`), and the final leg's medal compares the standings rank (`RaceStage::tournament_final_rank`), not a leg's own placing. Chosen, not measured: a non-finisher scores 0 (standing in for the original's own destroyed/race-state-7 guard), no save/resume between legs, and no authored standings table (a leg reuses the ordinary results/rewards screens). Open, unforced by the build: Head2Head entirely, `DAT_08b30fa0`'s writer, `DAT_08b31158+0xdc`'s meaning, the leg-hash-to-track resolution, `DAT_08b34320`'s reset condition

## Update, 2026-10-08 (pulse-h2h): Head2Head is built, quit returns to Cell Selection

Head2Head was already mapped, launched and drawn (the brief's "refused by
`race_mode_for_cell`" was stale); `Race_RecordResult`'s mode-9 arm is read
(same arm as `Race`) and a campaign Head2Head cell was walked live. Quitting a
campaign race now lands on `Cell Selection` in every mode, Pulse and HD
([`campaign-quit.md`](../../docs/ghidra/functions/psp-pulse-usa/campaign-quit.md)).

## Open (2026-10-08)

- Which team the Head2Head opponent flies: Qirex on `grid4_5_2`, Goteki 45 on a
  custom race, our `teams_for_slots` cycle is chosen, not measured.
- A Head2Head cell watched live to its finish and medal (place 1 gold is
  decompile only).
- `FUN_088099b4`, the test that picks `Show Unlocks`' redirect, and a quit walk
  of the Time Trial, Speed Lap, Zone, Elimination and Tournament cells (same
  `IG_PAUSE_QUIT` data, not walked).
- HD/Fury's own quit destination and cursor, and a pause menu with CONTINUE:
  escape discards a campaign race here (chosen, not measured).
- Omega launches Head2Head cells with one opponent already; its own rules are unread.
