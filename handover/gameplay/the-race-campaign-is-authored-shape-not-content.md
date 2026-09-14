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

- **The in-race HUD medal tier is still unread, and it is a different number
  from the campaign's.** `hud.md`'s five-way caption table runs `0 = BRONZE` up
  to `3 = RECORD`; `Cell_EvaluateMedal` runs `0 = gold` and has no `RECORD`
  tier. The field is `*(hud + 0x3c) + 0x34`, cached against `hud + 0x190`,
  siblings `+0x30` (target value) and `+0x38` (redden bool). Ruled out as its
  writer: `Hud_BindWidgets`' `DAT_08b30ffc` reads at `0x088207d8`/`0x088207e4`,
  `Eliminator_UpdateKillTarget`, `AI_ResolveSkillScale`. Confidence 50, not
  renamed. This is the one item `hud.md` is still explicitly blocked on.
- **`FUN_088085d0` and `FUN_08808624` are deliberately unnamed.** Ghidra
  recovers three parameters where every call site passes four; the fourth reads
  as create-if-missing but that is inference. Fixing the prototype in Ghidra and
  re-reading would let both be named and would firm up the record-store section.
- **`Locked` on a `PI_Cell` is now settled, 2026-09-14**: `CellSelection_PopulateGrid`
  (`0x088d5de4`) draws the `Lock_x_y` overlay exactly when `cell->Locked
  (+0xb9) != 0` **and** `Cell_BestMedal(cell) == 0xff` (no medal earned yet) -
  see `race-campaign.md`'s "Locked does drive the Lock_x_y overlay". Safe to
  implement a lock from the attribute now, gated the same way. **Still open**:
  `Status` on a `PI_Cell`, `Locked`/`Group` on a `PI_Grid`, and a six-neighbour
  adjacency loop the same function runs when no medal is earned
  (`FUN_088c072c`, coordinate table `0x08ab1e68`) that may be a *second*
  unlock path layered on `Locked` - not traced, found on the way and left for
  the next pass. `Definition_IsUnlocked` and `FUN_0888e5e4` remain ruled out
  as any of these bytes' reader. `Group="1"` marks `grid12`..`grid15`.
- **The `Tournament` arm of `Race_RecordResult` does a second record lookup**
  that no other arm does, keyed on `Libc_HashString(DAT_08b31158 + 0x74)`
  (named 2026-09-14; formerly `FUN_08945890`) rather than on the cell - the
  tournament's own standings, the state behind `ER_TOUR_STAN` /
  `ER_RACE_POINTS` / `ER_END_TOUR_1..8`. `DAT_08b31158` itself was still not
  identified, but its **seeding** now is: confirming a Tournament-mode cell
  (`CellSelection_CommitSelection`, `0x088d6138`) resets `DAT_08b31158+0xa0` to
  0, sets `+0xdc` to 1, resets the leg counter `DAT_08b30fa4` to 0, and
  appends one entry per `TournamentTrack` row via `FUN_088c3990` - see
  `race-campaign.md`'s new launch section. The per-leg accumulation itself
  (what `Race_RecordResult` does to `DAT_08b31158` after each leg) is still
  not traced. Tournament has 27 authored cells and is the only mode with
  per-leg state, so this is where a Tournament implementation starts.
- **`Unlock_LoyaltyMet` (`0x0888ea30`) compares against a whole 32-bit word** at
  the record's `+8`, which for a cell is `difficulty | medal << 8`. Either the
  team record's payload differs or the arithmetic does something this pass did
  not follow. Named `_q` at 72 on the field it reads, not the arithmetic.
- **Only the USA pressing was read.** EU and PS2 Pulse, Pure and HD/Fury were
  not checked for `Data\Plugins\grids` at all.
- **Nothing is runtime-verified.** No PPSSPP breakpoint was taken; every score
  is capped in the 84-92 range.
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
  → `save`/`parse` round trip → medal and points survive). **Not done, and
  the next step now**: `RaceStage::observation` still passes
  `campaign_medal: None` unconditionally - no medal is captured from any
  real race yet, because nothing selects which `PI_Cell` a launch
  corresponds to. See `docs/architecture/persistence.md`'s "where a career
  system attaches" for the three concrete gaps (the launch path below, the
  `Cell`/catalogue name mismatch, and Zone/Elimination's value not being in
  `Observation` at all).
- ~~Do not implement the cell-selection wiring without first tracing the
  launch path.~~ **Traced, 2026-09-14** - see the `Open` entry above and
  `race-campaign.md`'s new section. **Still the prerequisite before wiring a
  real launch**, because nothing on the Rust side yet models `DAT_08b30ffc`
  (a "current cell" concept `World`/`Stage` construction would need to carry
  through to `RaceStage::observation`) or the front-end global store
  `Globals_Set` writes into (this project's own menu code already has some
  equivalent - `crates/ui`/`crates/game`'s own global-key mechanism, not
  audited against this page this pass). `oag_tables::race_campaign::Cell`
  already carries every field the write site reads (`name`, `track`, `mode`,
  `class`, `laps`, `AICount`... - checked against the struct directly this
  pass, **no new field is needed there**). What *would* need new code: the
  hash function itself (`FUN_08945a08`, called by the newly-named
  `Libc_HashString` but not itself decompiled) is what a Rust reimplementation
  of the record-store key would need to match byte-for-byte if the key ever
  has to round-trip through the original's save format; nothing this pass
  found requires it for `records.toml`, which already keys on its own
  `Key` type per `docs/architecture/persistence.md`.
- **Close the HUD tier.** Find the writer of `*(hud + 0x3c) + 0x34` - the
  in-race structure the HUD mirrors. Everything ruled out is listed above, so a
  next pass starts from a shorter list. Closing it unblocks `hud.md`'s medal
  target item, which has been blocked on RE since the HUD work.
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
