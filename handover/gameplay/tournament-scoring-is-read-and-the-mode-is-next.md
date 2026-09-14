# Tournament scoring is read, and the mode is next

2026-09-14. `Tournament` is 27 of the campaign's 236 cells and the only mode
with per-leg state; `oag_game::campaign::race_mode_for_cell` still refuses
it (and `Head2Head`) because nobody had read what a leg scores or what
carries state between legs. This pass reads that law. It does not implement
the mode - this is the implementation brief for whoever does.

Read, in this order:

- [`docs/ghidra/functions/psp-pulse-usa/tournament.md`](../../docs/ghidra/functions/psp-pulse-usa/tournament.md) -
  the full decompilation: the points table, `DAT_08b31158`'s fields, what
  advances a leg, what value a Tournament cell's medal actually compares,
  the tie-break, and the save/resume mechanism.
- [`docs/ghidra/functions/psp-pulse-usa/race-campaign.md`](../../docs/ghidra/functions/psp-pulse-usa/race-campaign.md) -
  `Cell_EvaluateMedal`/`Cell_MedalPoints` (the campaign-grid 3/2/1 medal
  points - a *different* table from Tournament's own per-leg one),
  `Race_RecordResult`'s mode dispatch, and `CellSelection_CommitSelection`'s
  Tournament branch (how a campaign cell seeds `DAT_08b31158`).
- [`docs/gameplay/race-modes.md`](../../docs/gameplay/race-modes.md)'s new
  Tournament section - the implementation-facing summary of the law below.
- [`docs/formats/race-setup.md`](../../docs/formats/race-setup.md) - the
  `Tournament C`/`Selection_Definition.xml` screen shape (four track slots
  on PSP, twelve on PS2) and the eight `ER_END_TOUR_1..8` placement strings.

## What is now established

**A leg's points are a fixed table by finishing position**: 1st through 8th
score 8, 6, 5, 4, 3, 2, 1, 0; a destroyed or DNF craft scores 0 regardless
of where it stopped. Not the campaign's own gold/silver/bronze medal points
(3/2/1), and not arithmetic on position. `g_tournament_points_by_position`
(`0x08ab0ba0`), confidence 88.

**`DAT_08b31158` is the runtime tournament object**, now fully fielded:
name (`+0x74`), leg count (`+0xa0` - not "standings", correcting the
previous pass's hedge), a multiplayer-tournament bool (`+0xa4`), twelve
4-byte leg name-hashes (`+0xa8`..`+0xd4`), and a byte flag (`+0xdc`) whose
exact meaning is still open. `DAT_08b30fa4` is the current leg index (0-based);
`DAT_08b30fa0` is the session's own copy of the total leg count, used in
every "is this the last leg" comparison - **its write site was not found**.

**A leg advances via `Tournament_AdvanceLeg` (`0x0882e0e0`)**, on the state
machine transitioning to `Load_Next_Race` - reached by `EndRace Menu`'s own
`ER_NEXT_RACE` row, offered on every leg but the last.

**The medal-eligible value is the final standings rank by total points**,
not the last leg's own finishing position and not the raw point total.
`Race_BuildEndRaceResult`'s previously-untraced Tournament block
accumulates each craft's per-leg points into a persistent per-craft total
(`DAT_08b34320`, a large pre-existing global this project has not named),
sorts descending, and passes that rank into `Race_RecordResult` - which
runs the same `Cell_EvaluateMedal` threshold compare a `Race` cell's
position goes through. A tie in total points is broken by grid-slot order
(the sort never swaps on an exact tie) - no fastest lap, no head-to-head
leg result, no later-leg placing acts as a tie-break anywhere in this path.

**Progress saves and resumes between legs**, into the same name-hashed
profile store the campaign's per-cell records use
(`Tournament_SaveProgress`/`Tournament_LoadProgress`) - the mechanism
behind `MSC_EVENT_TOURN`'s "you can also save your tournament progress
between races" and the `MSC_MSG_AUTOSAVE3` dialog.

**A Custom Race `TOURNAMENT` (Racebox's `Tournament C`) exercises the
identical law**: `TournamentSelection_CommitSelection` writes the same
`DAT_08b31158` and calls the same leg-append functions
(`Tournament_AppendLegByName`/`Tournament_AppendLegHash`)
`CellSelection_CommitSelection` does for a campaign cell. Confidence 88 for
that equivalence specifically - two independently-positioned vtable slots
on two different screens, calling the same sibling functions.

## Open

- **`DAT_08b30fa0`'s write site.** `get_xrefs_to` returns eleven reads and
  zero writes across every function this pass and the previous one
  decompiled. Needs a live watch, not another cross-reference search.
- **`DAT_08b31158+0xdc`'s exact meaning.** Written `1` by a campaign launch,
  `0` by both Racebox paths (editing vs. playing is the working guess,
  untested) - nothing reads it back in any function decompiled so far.
- **The leg-name-hash to `PI_Track`-pointer resolution.**
  `Tournament_AdvanceLeg` reads from what looks like a *different* array
  (`param_2 + leg*4 + 0x138`) than `DAT_08b31158`'s own `+0xa8` hash slots -
  the resolution step between the two was not located.
- **`DAT_08b34320`'s own identity and its `+0x90` field's reset condition.**
  Read by over twenty unrelated functions (including two texture loaders),
  so it reads as a general per-craft/per-team persistent array, not
  something Tournament-specific. Its one write site (`FUN_08820d78`) was
  found but not decompiled - does a fresh tournament actually zero a
  craft's running total, or can it carry over from an unrelated prior race?
- **The copy into `g_endrace_result`'s own per-craft `+0x35`(name)/`+0x134`(points)
  fields** that `EndRaceResults_PopulateTournamentTable` reads for the
  on-screen standings table - the sort in `Race_BuildEndRaceResult` operates
  on a different scratch array, so something else must copy the sorted
  rows across, and that copy site was not located.
- **Head2Head was not reached this pass.** Deliverable 4 of this cycle's
  assignment; the points/standings chain above was the higher-value target
  and consumed the pass's own budget.
- **Live verification.** See below.
- **Only the USA pressing was read.** Per
  [ADR-0048](../../docs/architecture/adr/0048-eu-is-the-psp-pulse-re-target-of-record.md),
  none of `tournament.md`'s addresses have been cross-checked against
  `psp-pulse-eu`.

## Next Steps

This is the implementation brief for `oag_race`/`oag_game`, in the shape
this project's other modes already follow (`Mode::has_opponents`,
`Options::eliminator_kill_target`, etc. - see
[`race-modes.md`](../../docs/gameplay/race-modes.md)):

1. **`oag_race::Mode::Tournament`** needs a per-craft points-total field
   that survives a leg boundary. Every mode implemented today resets
   `RaceState` fresh per race, because every one of them ends there;
   Tournament is the first that does not. This is genuinely new shape, not
   an extension of an existing field.
2. **The campaign session** (`crates/game/src/main/session/campaign.rs`,
   `Session::launch_campaign_cell`) needs to carry the leg list (the
   cell's own `tournament_tracks: Vec<String>`,
   `oag_tables::race_campaign::Cell`) and the running per-craft point
   totals across a relaunch, rather than treating each leg as an
   independent `launch_campaign_cell` call the way a `Race`/`Time Trial`
   cell is today.
3. **Scoring a finished leg** is `g_tournament_points_by_position[position]`
   (8/6/5/4/3/2/1/0), zero for a craft that did not finish - reuse
   `Standing`'s own finishing-position field, the same one
   `Cell_EvaluateMedal`-equivalent code already reads for `Race`.
4. **What `EndRace` shows between legs**: a per-craft standings table
   (position, name, running point total) - the drawing side of this is
   `endrace-draw`'s lane (`pulses-endrace-screens-are-read-and-not-drawn.md`),
   not this thread's; this thread only establishes what the *values* are.
   `ER_NEXT_RACE` is the button that continues to the next leg; the last
   leg instead resolves to the ordinary `EndRace Menu` options plus the
   medal-worthy standings rank.
5. **The medal**, once a tournament finishes, is `Cell_EvaluateMedal`
   applied to the *final standings rank*, not the last leg's placing -
   reuse the existing `oag_tables::race_campaign::Cell::evaluate_medal`
   unchanged, just feed it the right value.
6. Do not implement Head2Head or the open items above as part of the same
   change unless they turn out to be free - they are separate, unread
   questions.
