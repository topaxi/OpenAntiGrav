# The Race Campaign is authored, all of it - shape *and* content

2026-09-08. Two passes in one day. The first was archaeology over the disc's
front-end XML and concluded the campaign's *screens* are authored and its
*content* is not; the second held the Ghidra bridge and found the content, in
sixteen files the first pass never opened. **The thread's title is kept only so
the index line still matches; the finding it names is superseded.**

Read, in this order:

- [`docs/ghidra/functions/psp-pulse-usa/race-campaign.md`](../../docs/ghidra/functions/psp-pulse-usa/race-campaign.md)
  - the authoritative page: the authored data, every parser, the medal
  evaluator, the points table, the unlock gate, the screens, and what is still
  unread.
- [`docs/formats/race-setup.md`](../../docs/formats/race-setup.md)'s Race
  Campaign section - the screen-side reading, now carrying its own corrections.
- [`docs/gameplay/race-modes.md`](../../docs/gameplay/race-modes.md) - the mode
  list, and the lap-count and Zone/Eliminator target items this closed.
- [`docs/formats/endrace-screens.md`](../../docs/formats/endrace-screens.md) and
  [`docs/ghidra/functions/psp-pulse-usa/endrace-screens.md`](../../docs/ghidra/functions/psp-pulse-usa/endrace-screens.md) -
  the three screens a campaign race ends on (`EndRace Results`/`Rewards`/`Menu`),
  read 2026-09-14: what fills them, the medal-glyph mechanism, and the loyalty
  ticker.

## What is now established

**The campaign's content is on the disc, in `Data\Plugins\grids\grid_00.xml`
.. `grid_15.xml` inside `Data.wad`** (listed by
`Data\Plugins\grids\Definition.xml`). Sixteen `PI_Grid` records holding **236
`PI_Cell` records** between them, 8-16 to a grid. Every cell authors its track,
mode, speed class, lap count, weapons and damage switches, AI count, AI skill,
and **its own gold, silver and bronze targets**.

The law, all decompiled and all in `race-campaign.md`:

- **A medal is worth gold 3, silver 2, bronze 1** (`Cell_MedalPoints`,
  `0x088bf530`). There is no separate points table on disc because there is no
  separate points table in the game.
- **A medal is a three-way threshold compare** against the cell's own
  `Target0..2`, comparison direction flipped for `Zone` and `Elimination`
  (`Cell_EvaluateMedal`, `0x088bf620`). Ordinal `0 = gold` .. `2 = bronze`,
  `0xff = none`, corroborated independently by `Unlock_MedalValue`
  (`0x0888ef80`) mapping the `<Unlock Medal="Gold">` string to the same numbers.
- **`<Unlock Grid="Grid0"/>` means "you scored at least grid0's
  `RequiredPoints` in grid0"** (`Unlock_GridPointsMet`, `0x0888ebd8`). The
  ladder is 12, 16, 20, 24, then 28 for every grid from `grid4` on, against
  maxima of 24, 30, 36, 42, 48.
- **The campaign touches `FEData.wad`'s 24 per-track `stats.xml` records for
  exactly one thing: AI difficulty.** A cell's `skill`/`skillEasy`/`skillHard`
  is a *position on the track's own `SkillScaleValue` curve*, interpolated by
  `AI_ResolveSkillScale` (`0x08834df4`). Nothing else - not a target, not a
  mode, not a lap count - comes from that family. This also closes
  `ai-stats.md`'s "`SkillScaleValue` unchased" item.
- **Lap counts are per speed class and exact**: 3 Venom, 4 Flash, 4 Rapier, 5
  Phantom across all 236 cells with no exception; `Speed Lap` always 7, `Zone`
  always 0.
- **The mode enum is nine values, not seven**: 3 `Race`, 4 `Tournament`, 5
  `Time Trial`, 6 `Zone`, 8 `Elimination`, 9 `Head2Head`, 10 `Speed Lap`, 11
  `Custom Grid`, 12 `AI Race`, read off the table at `0x08ab062c`. `7` is
  absent.
- **Eliminator's kill target comes from the cell's gold target**
  (`Eliminator_UpdateKillTarget`, `0x0882ce18`), 10/7/5 on all 22 campaign
  Eliminator cells. Zone's campaign targets are 18-24 per track and are *not*
  `FEData`'s `Zone="25"`.

39 names landed in `names.tsv` against `race-campaign.md`.

## Open

- ~~The in-race HUD medal tier is still unread...~~ **Closed 2026-09-28.**
  The writer is `PlayerStatus_Update` (`0x0883b3b8`), runtime-verified with a
  PPSSPP write watchpoint (all four write PCs land inside it, across an
  eight-second drive of a real Venom Time Trial). The field is genuinely a
  different ordinal from `Cell_EvaluateMedal`'s, as suspected - `0 = BRONZE`
  up to `3 = RECORD`, related to `Cell_EvaluateMedal`'s `0 = gold, 1 = silver,
  2 = bronze` by exactly `2 - medal` on the three tiers they share - and it
  is evaluated live, every tick, against the same cell's `gold`/`silver`/
  `bronze` fields for a campaign Time Trial/Speed Lap cell, or against a
  separate, still-unread per-track ghost/record store (`FUN_088091a0`)
  otherwise, always showing `RECORD` on that second path. Wired into the HUD
  for the campaign branch: `oag_game::hud::TimeTrialPace`. Full law:
  [`docs/ghidra/functions/psp-pulse-usa/race-progress.md`](../../docs/ghidra/functions/psp-pulse-usa/race-progress.md#the-target-time-readout-0x780x7c0x80-closed-2026-09-28).
  Still open, and out of this pass's scope: the non-campaign ghost/record
  branch, which needs a split-time record store this project's own
  `oag_game::records` has no equivalent of.
- **`FUN_088085d0` and `FUN_08808624` are deliberately unnamed.** Ghidra
  recovers three parameters where every call site passes four; the fourth reads
  as create-if-missing but that is inference. Fixing the prototype in Ghidra and
  re-reading would let both be named and would firm up the record-store section.
- **`Locked` on both `PI_Cell` (`+0xb9`) and `PI_Grid` (`+0xa0`) is fully
  traced now, 2026-09-14** - see `race-campaign.md`'s new "Unlock rules, cell
  and tier". The cell's `Lock_x_y` glyph is visible exactly when
  `cell->Locked != 0` **and** `Cell_BestMedal(cell) == 0xff` **and** no
  hex-adjacent cell has a medal either (`CellSelection_PopulateGrid`,
  `0x088d5de4`) - three terms, not two: the six-neighbour adjacency loop the
  previous pass flagged as a possible *second unlock mechanism* is now fully
  decompiled and is not a second mechanism, but it **is** real, player-visible
  unlock-display logic (`Grid_FindCellAtCoordinate`,
  `g_anCellNeighbourOffsets` at `0x08ab1e68`, a `GridController_*` widget-flag
  family) - `Locked` is the static default an unmedalled cell starts from,
  and a medal (its own, or with a brief shared-timer fade, a neighbour's)
  clears the glyph. It never writes `cell->Locked`, `PI_Grid.Locked`,
  `Status`, or calls any `Unlock_*` predicate - only the glyph's visibility
  changes. `GridSelection_PopulateTiles` (`0x088de630`, newly named) draws
  the tier's own lock the same shape, off `grid->Locked` (`+0xa0`), with a
  hard-coded "is the immediately-preceding tile's own points-required
  already met" shortcut instead of calling `Unlock_GridPointsMet`. `Status`
  on a `PI_Cell` remains untraced; `Group="1"` still just marks
  `grid12`..`grid15`, consumer untraced. **Separately open**: whether an
  absent `Locked` attribute truly defaults to `true` still can't be read from
  `PI_Cell_ParseElement` alone - it never writes a default to `+0xb9`, so the
  answer is in the (unlocated) object allocator, not this parser.
- **Answered, runtime-verified 2026-09-14: yes, `Locked` (either byte) gates
  `Confirm`, and the refusal sits upstream of every function this project has
  decompiled so far.** Four PPSSPP breakpoints, each with its own positive
  control on an unlocked tile of the same kind: on a `Locked`, unmedalled
  `Cell Selection` cell and on a `Locked` `Grid Selection` tier,
  `StateMachine_TransitionTo` is never entered at all, so neither screen's
  own `CommitSelection` vtable slot ever gets a chance to check the byte -
  the confirm press is swallowed before the state machine is even asked to
  transition. Not cosmetic at either level. The successful positive-control
  runs pin `StateMachine_TransitionTo`'s own caller at `ra=0x088c8a10` - a
  concrete, previously unseen address, not yet decompiled (no Ghidra bridge
  held this pass) - as the next target for finding the actual generic rule.
  `GridController_SetTileFlags` also fired live with exactly the
  `(widget, layer, column, row)` shape `GridSelection_PopulateTiles`'s
  decompile already predicted, reproducing the lock-layer branch tile for
  tile on a real page. Full four-breakpoint transcript:
  `race-campaign.md`'s "Runtime-verified 2026-09-14 (deliverable 1)"
  subsection under "Unlock rules, cell and tier".
- **`Grid`/`Grid1` are a double-buffer pair**, settled this pass:
  `GridSelection_Update` toggles which of the two holds the currently-shown
  page (`+0x114`, flipped on every page move) while the other is repopulated
  with the incoming page - not two independent widgets. This also explains
  the earlier "no crossfade" screen capture: an index flip between two
  already-populated widgets has nothing to interpolate.
- **Negative result: the cell d-pad movement rule is not the six-neighbour
  table.** `Grid_FindCellAtCoordinate` and `g_anCellNeighbourOffsets` have
  exactly one caller between them (`CellSelection_PopulateGrid`), confirmed
  by `get_function_callers` - no input-handling code reaches either. The real
  cursor-movement code was not located; a reimplementation is still on its
  own chosen nearest-neighbour search.
- ~~The `Tournament` arm of `Race_RecordResult` does a second record
  lookup... `DAT_08b31158` itself was still not identified... The per-leg
  accumulation itself... is still not traced.~~ **Closed, 2026-09-14, a
  separate pass**: [`docs/ghidra/functions/psp-pulse-usa/tournament.md`](../../docs/ghidra/functions/psp-pulse-usa/tournament.md)
  has the full law. Headline: a leg's points are a **fixed table by finishing
  position** (`g_tournament_points_by_position`, `0x08ab0ba0`: 8/6/5/4/3/2/1/0
  for 1st..8th, zero for a destroyed or DNF craft) - not the campaign's own
  3/2/1 medal points and not arithmetic on position. `DAT_08b31158+0xa0` is
  the **leg count**, correcting this thread's own "reset standings" framing
  of it - the standings live in the profile-keyed record, unchanged.
  `Race_BuildEndRaceResult`'s previously-untraced Tournament block
  accumulates each craft's per-leg points into a persistent per-craft total
  and sorts descending; **that sorted rank, not the last leg's own
  finishing position and not the raw point total, is what
  `Cell_EvaluateMedal` compares** for a Tournament cell's medal. A tie in
  total points is broken by grid-slot order (no swap on an exact tie), not
  a secondary criterion. `Tournament_AdvanceLeg` (`0x0882e0e0`) is what
  moves a tournament between legs, gated by `EndRace Menu`'s own
  `ER_NEXT_RACE` row. Confirmed that a Custom Race `Tournament C`
  (`TournamentSelection`) writes the identical `DAT_08b31158` and calls the
  identical leg-append functions the campaign's own `Cell Selection` does,
  so a Custom Race tournament is a faithful test of the campaign's law.
  Still open: `DAT_08b30fa0`'s own write site, `DAT_08b31158+0xdc`'s exact
  meaning, and Head2Head entirely - see `tournament.md`'s own "What is not
  determined". `docs/gameplay/race-modes.md`'s new Tournament section
  carries the implementation-facing summary, and
  `handover/gameplay/tournament-scoring-is-read-and-the-mode-is-next.md`
  is the implementation brief.
- **`Unlock_LoyaltyMet` (`0x0888ea30`) compares against a whole 32-bit word** at
  the record's `+8`, which for a cell is `difficulty | medal << 8`. Either the
  team record's payload differs or the arithmetic does something this pass did
  not follow. Named `_q` at 72 on the field it reads, not the arithmetic.
- **Only the USA pressing was read.** EU and PS2 Pulse, Pure and HD/Fury were
  not checked for `Data\Plugins\grids` at all.
- **Runtime-verified 2026-09-14, four of the five priority breakpoints.**
  `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s "Runtime-verified
  2026-09-14" subsection has the full transcript;
  `/tmp/oag-drive/campaign-ppsspp-measure.md` is the session's own scratch
  file. Headline: `CellSelection_CommitSelection` (`0x088d6138`, confidence
  80 -> **93**) fires on a real confirm and writes the documented nine
  `Globals_Set` (`0x08888ee0`, confidence 85 -> **92**) keys in the
  documented order, with resolved values matching the confirmed cell's own
  authored XML digit for digit (`Track="16_Track"`, `Opponents="0"`,
  `Damage="Off"`, ...); `DAT_08b30ffc` was read directly at four points from
  confirm through a live race and never changed; `CellSelection_PopulateGrid`
  (`0x088d5de4`, confidence 78 -> **82**) fires once per screen entry with
  two int arguments, not once per cell, and a fresh capture shows the lock
  glyph on exactly the `grid0` cells with **no** `Locked` attribute at all in
  `grid_00.xml` - suggesting an absent `Locked` defaults to `true`
  (confidence 72, new this pass). The fifth breakpoint (a Tournament cell)
  was blocked: every grid but `grid0` reads `Locked="true"` on the profile
  used, and confirming a locked `Grid Selection` tile does nothing, so no
  Tournament cell was reachable without earning points first. A tooling trap
  worth carrying forward: sending a button press whose own code path hits an
  armed execution breakpoint leaves that `input.buttons.press` call
  unacknowledged (client-side timeout) even though the press was delivered
  and the breakpoint fired correctly - catch the timeout and check
  `wait_for_break` anyway rather than treating it as a failure.
- **How a campaign event launches - traced 2026-09-14, not runtime-verified.**
  `CellSelection_CommitSelection` (`0x088d6138`, confidence 80 - it is word
  39 of the `CellSelection` vtable, positionally confirmed the same way
  `Update`/`OnEnter`/`OnExit` already were, with the identical slot found on
  `TrackSelection` and `TeamSelection` too) is where a confirmed cell's
  fields reach two places at once: `DAT_08b30ffc` (a raw `PI_Cell` pointer,
  already read elsewhere by `Cell_EvaluateMedal`'s callers,
  `Eliminator_UpdateKillTarget`, `AI_ResolveSkillScale` and `Hud_BindWidgets`)
  and `Globals_Set` (`0x08888ee0`), the **same** string-keyed front-end global
  store `TrackSelection_OnExit` writes `Track` into for a custom race -
  confirmed by `get_function_callers` listing both. Keys written:
  `Mode`, `Class`, `Track`, `Team`, `Weapons`, `Opponents`, `Laps`, `Damage`,
  `SkillLevel`, and `Tournament` for a Tournament cell. So it is **not**
  "one mechanism or the other" between the campaign and the custom Race Box -
  both write the identical global keys, and the campaign *additionally*
  carries the raw cell pointer for the numeric consumers a display string
  can't serve. Full detail, the record-store key (a hash of the cell's own
  `name` string, not a grid/cell index pair), and what remains
  unverified: `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s new
  "How a campaign event launches" section. **What is still open**:
  `CellMode_Definition.xml`
  carries two named, buttonless `<Redirect>` blocks (`Cell Mode Redirect
  Team` -> `Team Selection`, `Cell Mode Redirect Game` -> `Launch Game`
  directly) whose selection logic was not located - whether a campaign
  launch ever takes the second one is unknown.
- **Points-vs-medals in the grid summary is settled but worth restating**, since
  the earlier reading was ambiguous: `GridSelection_Update` binds `Medals` to
  `Grid_CountMedalsAtLeast(grid, 0) / Grid_CellCount(grid)` - **gold** medals
  over cell count - and `Points` to `Grid_PointsEarned / Grid_PointsPossible`.
  The `"00/16"` and `"000/110"` XML strings are template widths, not data.

- **`0x088c8a10` decompiled, 2026-09-14: a negative result, not the lock
  check** - and now, as of a second 2026-09-14 pass, closed further still.
  It sits inside `StateMachine_EvaluateRedirect` (`0x088c8798`, confidence
  75) - the generic, screen-agnostic `<Redirect><Entry item= equals=
  goto=>...<Default goto=>` evaluator every screen's own confirm handling
  goes through (eleven call sites, found while reading the EndRace screens -
  see
  [`docs/ghidra/functions/psp-pulse-usa/endrace-screens.md`](../../docs/ghidra/functions/psp-pulse-usa/endrace-screens.md)).
  It never reads `Locked`, a medal, or a `PI_Cell`/`PI_Grid`-shaped offset -
  only the redirect node's own `Entry` list, `Default` target and the
  special-cased widget name `"Focus"`. **`0x088d7e1c`, this section's own
  earlier lead, is ruled out**: full decompile shows it is the held-confirm
  variant used by `InGame Photo`, not `Cell Selection` - it never runs on
  this screen. Breakpointing `StateMachine_EvaluateRedirect` directly (cursor
  on `grid0_3_1` unlocked, then `grid0_2_1` locked - the same pair the
  "Runtime-verified 2026-09-14 (deliverable 1)" pass used) found the real
  caller: `$ra = 0x088c907c`, inside `ConfirmButton_Update` (`0x088c8e88`,
  confidence 85) - fires on the unlocked cell, never fires on the locked
  one. `ConfirmButton_Update` was then cleared too: it runs identically
  every frame regardless of lock state, and its own accept-branch reaches
  `StateMachine_EvaluateRedirect` unconditionally once "accept is pressed"
  is true, with no `Locked`/medal read anywhere in it. **The gate is now
  pinned to input consumption**: something makes `ConfirmButton_Update`'s
  own `Input_IsPressed(g_input, 4, 0)` check read false on a locked-tile
  frame despite `cross` being held for the full 6 s of the negative control
  - not located this pass, but a single, narrow target rather than eleven
  candidates or an unconfirmed flag. See `race-campaign.md`'s own
  "`0x088d7e1c` ruled out..." subsection for the full transcript.
- **A second, unrelated finding from the same EndRace pass closes the
  `Unlock_LoyaltyMet` anomaly this thread flagged below - and, as of
  2026-09-14, closes the loyalty-award computation itself too.**
  `Unlock_LoyaltyMet` reads a per-*team* record (`DAT_08b31774`, keyed by
  team name through the same generic accessor `race-box-screens.md` names
  for `TeamSelection`'s rating table), not a per-*cell* one -
  `EndRaceRewards_Update` independently reads that same store's `+8` as a
  plain accumulated loyalty total for the `EndRace Rewards` screen's own
  "Total loyalty" figure. Raised 72/78 -> 82/82. **The writer itself is now
  found and decompiled**: `Race_ComputeLoyaltyAward` (`0x0880ac50`), called
  from the race-end summariser `Race_BuildEndRaceResult` (`0x0882a498`).
  Its law is exactly the "30 points per lap" reading, generalised: `laps*15
  + perfectLaps*25` for the Race family, `laps*30 + perfectLaps*50` for
  Time Trial/Speed Lap, `laps*10 + perfectLaps*20` otherwise, a per-kill
  term, and a Race-family-only difficulty multiplier (doubled again for a
  suggested-ship race). Confirmed on two independent live races (different
  paces, same `laps=3, perfectLaps=0` shape), both reproducing `award = 90`
  exactly - live-read off `g_endrace_result+8` and shown on-screen
  (`Assegai Loyalty: 90 Points`, `Total loyalty: 90`, up from `0` on a fresh
  profile). Confidence **95**.
- **A new thread carries the drawing-lane implications of all three EndRace
  screens**: `handover/gameplay/pulses-endrace-screens-are-read-and-not-drawn.md`.

## Next Steps

- ~~Implement the medal law and persist it.~~ **Done, 2026-09-09.**
  `oag_tables::race_campaign::Cell::evaluate_medal` reimplements
  `Cell_EvaluateMedal` directly on `Cell` (three-way threshold compare,
  direction flipped for `Zone`/`Elimination`, `Medal::points` for
  `Cell_MedalPoints`), tested against three cases chosen to fail if the flip
  is dropped or wrongly applied elsewhere. `crates/game/src/records.rs`
  gained `Record::best_medal`/`best_points`/`last_medal` and
  `Observation::campaign_medal`, additive per this thread's own earlier
  "field" suggestion. `crates/game/tests/campaign_medal.rs` proves the two
  pieces agree end to end with an invented cell (evaluate → convert → merge
  → `save`/`parse` round trip → medal and points survive).
  ~~**Not done, and the next step now**: `RaceStage::observation` still
  passes `campaign_medal: None` unconditionally.~~ **Done, 2026-09-14**, in
  the `campaign-launch-wiring` lane: `Session::campaign_cell` is the Rust
  model of `DAT_08b30ffc` (drained into `RaceStage::campaign_cell` the
  moment a race stage is built), the `Cell`/catalogue name mismatch this
  bullet worried about was not one - a cell's own `track=` id already
  shares `catalogue::Track::id`'s spelling - and `Zone`/`Elimination`'s
  value reached `RaceStage::campaign_medal` without widening `Observation`
  at all, since that function already has `self.race` in hand. A second,
  cell-name-keyed `[[campaign]]` table in `records.toml` is what
  `Grid Selection`/`Cell Selection` read the medal back through - see
  `docs/architecture/persistence.md`'s "where a career system attaches" for
  the full detail and why it is a sibling table rather than a fourth `Key`
  field. The record-store key's own hash function (`FUN_08945a08`,
  below) is still not needed: `records.toml` keys on the cell's own bare
  `name` string, not a hash of it, so nothing here had to reproduce the
  original's hash byte-for-byte.
- ~~Do not implement the cell-selection wiring without first tracing the
  launch path.~~ **Traced, 2026-09-14, and wired the same day** - see the
  `Open` entry above, `race-campaign.md`'s new section, and
  `docs/ui/campaign-screens.md`'s "Confirming a cell launches".
  `oag_tables::race_campaign::Cell` needed no new field, as this bullet
  predicted. **Still open**: the hash function itself (`FUN_08945a08`,
  called by the newly-named `Libc_HashString` but not itself decompiled)
  would still be needed the day a save has to round-trip through the
  original's own format, which nothing in this project does.
- ~~Close the HUD tier...~~ **Done, 2026-09-28** - see the `Open` entry
  above. The next step this opens, if anyone picks it up: `FUN_088091a0`'s
  own per-team, per-track split-time record store, for the non-campaign
  `RECORD` branch this pass left unimplemented.
- ~~Parse the grid files properly rather than by hand.~~ **Done, 2026-09-08.**
  `oag_tables::race_campaign` parses `PI_Grid`/`PI_Cell` off the existing
  `fexml` reader, `oag_pulse::campaign` carries the sixteen entry names, and
  `crates/tables/tests/race_campaign_ground_truth.rs` (`#[ignore]`d,
  `just test-data`) reproduces every count this thread's own summary above
  cites against the real USA PSP disc: 16 grids, 236 cells, the exact
  per-class lap census, the mode census (Race 59 / Time Trial 47 / Speed Lap
  42 / Tournament 27 / Head2Head 23 / Elimination 22 / Zone 16) and the
  `RequiredPoints` ladder. See `docs/formats/race-campaign.md`. **One new
  finding**: a `Zone` cell's own `class=` attribute is the literal text
  `"Zone"`, not one of the four speed classes `g_class_name_table` was read
  as - either that table has a fifth entry unread, or the field just carries
  an unrecognised name. The parser kept it as a raw string rather than
  guessing which. **Deliberately not done**: the medal law
  (`Cell_EvaluateMedal`, the points table, the unlock-points comparison) and
  wiring a cell into an actual race - both gameplay, not parsing, and the
  launch-path tracing below is still the prerequisite for either.
- **Check the EU and PS2 pressings for `Data\Plugins\grids\Definition.xml`.**
  The path is now known, so this is one `oag-wad cat` per image, and a
  divergence would be a finding (PS2 already diverges on the *custom* grid,
  twelve `Tournament C` slots against four).
- **Resolve the region ambiguity on `data/images/pulse-psp-usa.chd`** - still
  open from the first pass; it identifies as `UCUS-98712` by boot path but
  carries a `SCEE` volume id and an EU-serial subtree.
- **Two facts other lanes can use now.** For a weapons/Eliminator lane: the kill
  target is the cell's gold target, the race ends when any ship reaches it, and
  the count lives at `entity + 0x8d8`. For a career/profile lane: the profile
  keeps gold/silver/bronze counters at `+0x160`/`+0x164`/`+0x168` and a dirty
  flag at `+0x45e`, and a custom grid's medals deliberately do **not** count
  toward them.
- **A front-end pass can now draw both locks for real**, 2026-09-14, and both
  are the same three-term shape - `Locked` as the static default, cleared by
  *this thing's own progress* or by *the adjacent thing's progress*: the cell
  glyph needs `cell->Locked != 0` and no medal of its own and no hex-adjacent
  cell with a medal either; the tier glyph needs `grid->Locked != 0` and this
  grid's own `Grid_PointsEarned == 0` and the previous tile's own
  points-required not yet met (see `race-campaign.md`'s "Unlock rules, cell
  and tier"). The tier side's
  "previous tile" shortcut is the original's own actual code path, though
  calling `Unlock_GridPointsMet` against the grid's own `<Unlock
  Grid=>` target is the more correct rule to implement (they agree on all 16
  shipped grids; only the hard-coded shortcut would diverge on a hypothetical
  non-adjacent `<Unlock Grid=>`, which nothing shipped exercises). **Whether a
  `Locked` glyph also has to block actually entering/playing that cell or
  tier is still open** - see the new bullet above - so a first pass should
  treat the glyph as authoritative for *drawing* the lock and treat
  *blocking* the confirm action as a separate, still-unverified decision;
  gating on `Locked` for both would match the tier-level PPSSPP capture at
  the cost of possibly over-blocking cells if the cell-level lock turns out
  to be cosmetic-only in the original.
- ~~Next PPSSPP pass, three breakpoints...~~ **Done, 2026-09-14** - see the
  `Open` entry above. All three questions collapsed to one answer: the
  refusal happens before `StateMachine_TransitionTo` is entered, on both
  screens. ~~**The new next step**: decompile `0x088c8a10`...~~ **Done,
  2026-09-14, and it is a negative result** - see the `Open` entry above.
  ~~**The next step now**: a PPSSPP breakpoint on `0x088d7e1c`'s entry...~~
  **Done, 2026-09-14 (second pass): `0x088d7e1c` ruled out, real dispatcher
  found and cleared too** - see the `Open` entry above.
  **The next step now**: whatever calls `Input_ConsumePress`/otherwise
  suppresses button `4` on a locked-tile frame, upstream of
  `ConfirmButton_Update`'s (`0x088c8e88`) own `Input_IsPressed` check - a
  single input-consumption question, not a widget-tree search.
- **Read the three EndRace screens.** ~~Not started~~ **Done, 2026-09-14** -
  see the new `Open` entries above and
  [`handover/gameplay/pulses-endrace-screens-are-read-and-not-drawn.md`](pulses-endrace-screens-are-read-and-not-drawn.md)
  for the drawing-lane implications. **What is still open there**: the
  function that computes and writes a race's own loyalty award
  (`g_endrace_result + 8`), and the exact per-lap/per-mode point values
  behind the `MSC_LOY_*` idstrings, which are a one-observation hypothesis
  (`30 x 3 laps = 90`) rather than a decompiled computation.

## From the HANDOVER.md index (moved 2026-09-25)

2026-09-08, two passes in one day. The first read the disc's front-end XML and concluded the campaign's screens are authored and its content is not; the second held the Ghidra bridge and **found the content in full, in sixteen files the first pass never opened**: `Data\Plugins\grids\grid_00.xml` ..`grid_15.xml` inside `Data.wad`, sixteen `PI_Grid` records holding **236 `PI_Cell` records**, every one authoring its track, mode, speed class, lap count, weapons and damage switches, AI count, AI skill and **its own gold, silver and bronze targets**. The law is decompiled: **a medal is worth gold 3, silver 2, bronze 1** (`Cell_MedalPoints`, `0x088bf530`); **a medal is a three-way threshold compare** against the cell's own targets with the direction flipped for Zone and Elimination (`Cell_EvaluateMedal`, `0x088bf620`, ordinal `0 = gold`); **`<Unlock Grid="Grid0"/>` means "you scored at least grid0's `RequiredPoints` in grid0"** (`Unlock_GridPointsMet`, `0x0888ebd8`), a 12/16/20/24/28 ladder against 24/30/36/42/48 maxima. **The campaign reads `FEData.wad`'s 24 per-track records for exactly one thing, AI difficulty** - a cell's `skill`/`skillEasy`/`skillHard` is a position on the track's own `SkillScaleValue` curve, interpolated by `AI_ResolveSkillScale` (`0x08834df4`), which also closes `ai-stats.md`'s unchased-string item. Also measured: **lap counts are per speed class and exact** (3 Venom, 4 Flash, 4 Rapier, 5 Phantom across all 236 cells, Speed Lap always 7, Zone 0), the **mode enum is nine values** off the table at `0x08ab062c` (adding `Custom Grid` and `AI Race` to the seven already known, `7` absent), and **Eliminator's kill target is the cell's gold target** (10/7/5 on all 22 cells) while Zone's campaign targets are 18-24 per track and are *not* `FEData`'s `Zone="25"`. 39 names landed against [`../../docs/ghidra/functions/psp-pulse-usa/race-campaign.md`](../../docs/ghidra/functions/psp-pulse-usa/race-campaign.md), the authoritative page. **Trap worth keeping**: the first pass's negative was a negative over the WAD names it thought to try - `Data\Plugins\grids\*` resolves only if you already know the path, and the string that gives it away is `"%s\Definition.xml"` at `0x08a7d6a8` applied to `"Data\Plugins\grids"` at `0x08a7d38c`. Open: the **in-race HUD medal tier is still unread and is a different ordinal** from the campaign's (`hud.md` runs `0 = BRONZE`..`3 = RECORD`; the field is `*(hud + 0x3c) + 0x34`, and `Hud_BindWidgets`' `DAT_08b30ffc` reads, `Eliminator_UpdateKillTarget` and `AI_ResolveSkillScale` are ruled out as its writer) - confidence 50, not renamed, and `hud.md`'s medal-target item stays blocked; **how a campaign event actually launches was not traced**, which is what an implementation needs first; nothing is runtime-verified and only the USA pressing was read
