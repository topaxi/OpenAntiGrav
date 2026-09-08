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
- **`Status`/`Locked` on a `PI_Cell`, `Locked`/`Group` on a `PI_Grid`** are
  parsed and no consumer of any of them was traced. `Definition_IsUnlocked`
  (which reads `+0x99`/`+0x9c`) and `FUN_0888e5e4` (a `"ms:"`/`"PID"`
  source-path test, i.e. memory-stick/DLC content, not a lock) are both ruled
  out. So whether `Locked="true"` drives `Cell Selection`'s `Lock_x_y` overlay
  or is redundant against the `<Unlock>` rows beside it is **open at 50** -
  do not implement a lock from the attribute. `Group="1"` marks
  `grid12`..`grid15`.
- **The `Tournament` arm of `Race_RecordResult` does a second record lookup**
  that no other arm does, keyed on `FUN_08945890(DAT_08b31158 + 0x74)` rather
  than on the cell - the tournament's own standings, the state behind
  `ER_TOUR_STAN` / `ER_RACE_POINTS` / `ER_END_TOUR_1..8`. `DAT_08b31158` was not
  identified and the per-leg accumulation was not traced. Tournament has 27
  authored cells and is the only mode with per-leg state, so this is where a
  Tournament implementation starts.
- **`Unlock_LoyaltyMet` (`0x0888ea30`) compares against a whole 32-bit word** at
  the record's `+8`, which for a cell is `difficulty | medal << 8`. Either the
  team record's payload differs or the arithmetic does something this pass did
  not follow. Named `_q` at 72 on the field it reads, not the arithmetic.
- **Only the USA pressing was read.** EU and PS2 Pulse, Pure and HD/Fury were
  not checked for `Data\Plugins\grids` at all.
- **Nothing is runtime-verified.** No PPSSPP breakpoint was taken; every score
  is capped in the 84-92 range.
- **How a campaign event actually launches was not traced.** `Cell Selection`
  redirects through `Team Selection` to `Launch Game`; which globals the cell
  writes on the way (mode, track, class, laps, weapons, AI count) were not read,
  and that is what an implementation needs first.
- **Points-vs-medals in the grid summary is settled but worth restating**, since
  the earlier reading was ambiguous: `GridSelection_Update` binds `Medals` to
  `Grid_CountMedalsAtLeast(grid, 0) / Grid_CellCount(grid)` - **gold** medals
  over cell count - and `Points` to `Grid_PointsEarned / Grid_PointsPossible`.
  The `"00/16"` and `"000/110"` XML strings are template widths, not data.

## Next Steps

- **Do not implement from this page without first tracing the launch path.**
  Read `Cell Selection`'s `Update`/`OnExit` (`0x088d6430`, and the `OnExit` slot
  at word 31 of `0x08acfb64`) and `Team Selection`'s exit to find which globals
  a cell writes. That is the gap between "the law is known" and "a cell can be
  raced".
- **Close the HUD tier.** Find the writer of `*(hud + 0x3c) + 0x34` - the
  in-race structure the HUD mirrors. Everything ruled out is listed above, so a
  next pass starts from a shorter list. Closing it unblocks `hud.md`'s medal
  target item, which has been blocked on RE since the HUD work.
- **Parse the grid files properly rather than by hand.** `oag-formats` already
  reads the `<code>`-dictionary front-end XML dialect; sixteen grid files with a
  known schema are a small, well-specified loader, and per `CLAUDE.md`'s rule
  the campaign must read the disc's data rather than carry a transcribed table.
  The schema is the two parser tables in `race-campaign.md`.
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
