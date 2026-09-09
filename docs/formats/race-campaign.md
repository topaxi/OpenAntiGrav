# Race campaign: `PI_Grid`/`PI_Cell`, `Data\Plugins\grids\grid_00.xml`..`grid_15.xml`

**Status: understood.** Implemented in
[`oag_tables::race_campaign`](../../crates/tables/src/race_campaign.rs), with
the sixteen entry names in
[`oag_pulse::campaign`](../../crates/pulse/src/campaign.rs) per
[ADR-0022](../architecture/adr/0022-title-packages.md) - the file's shape is a
format fact, the sixteen names it ships are a title fact.

This is the schema half of
[`docs/ghidra/functions/psp-pulse-usa/race-campaign.md`](../ghidra/functions/psp-pulse-usa/race-campaign.md),
which carries the decompiled law. As of 2026-09-09 the medal evaluator and
its points table are also implemented here, on the type they evaluate - see
below - but the unlock-points comparison across a whole grid, and picking
*which* cell a race is run against, are not.

## What is on disc

Sixteen files, `Data\Plugins\grids\grid_00.xml` through `grid_15.xml`, listed
by `Data\Plugins\grids\Definition.xml`, all inside `Data.wad`. Each is one
`PI_Grid` holding 8-16 `PI_Cell` records - **236 cells in total on the USA PSP
pressing**, confirmed by
`crates/tables/tests/race_campaign_ground_truth.rs` (`#[ignore]`d, run with
`just test-data`). Every number that ground-truth test asserts was measured by
the Ghidra pass and is reproduced here rather than re-derived:

- 16 grids, 236 cells.
- Lap counts exact per class with no exception: 3 Venom, 4 Flash, 4 Rapier, 5
  Phantom; Speed Lap always 7; Zone always 0; Elimination carries no `laps`
  attribute at all.
- Mode census: Race 59, Time Trial 47, Speed Lap 42, Tournament 27, Head2Head
  23, Elimination 22, Zone 16.
- `RequiredPoints` ladder 12/16/20/24/28 (28 from `grid4` on, 0 on `grid15`)
  against maxima 24/30/36/42/48.
- `Group="1"` marks exactly `grid12`..`grid15`.

## A finding this parser surfaced: `class="Zone"` is not a speed class

`docs/ghidra/functions/psp-pulse-usa/race-campaign.md` reads
`g_class_name_table` (`0x08ab067c`) as four entries: `Venom`, `Flash`,
`Rapier`, `Phantom`. Every shipped `Zone`-mode cell's own `class=` attribute is
nonetheless the literal text `"Zone"` - confirmed directly against
`grid_00.xml` on `pulse-psp-usa.chd` (`grid0_4_2`: `mode="Zone"
class="Zone"`) and consistent on every Zone cell checked. Either the table has
a fifth entry unread by that pass, or the field is left holding an
unrecognised name rather than being rejected; either way it is real, authored
text and not a parser artefact.

`race_campaign::Cell::class` is therefore the **raw** string, never forced
into `oag_tables::handling::SpeedClass`. `Cell::speed_class` does the
fallible mapping and returns `None` for `"Zone"` - an unrecognised value is
not an error, per this project's own established lesson that a parser must
not fail on a field it does not understand.

## The medal law, implemented on `Cell`: `evaluate_medal`

`Cell::evaluate_medal(value: i64) -> Option<Medal>` reimplements
`Cell_EvaluateMedal` (`0x088bf620`) directly: a three-way threshold compare
of `value` against the cell's own gold/silver/bronze targets, ordinal
`Gold`/`Silver`/`Bronze` (`0`/`1`/`2` in the original), comparison direction
**flipped for `Zone` and `Elimination`** (`>=`, more is better) and ordinary
(`<=`, less is better) everywhere else. `Medal::points` reimplements
`Cell_MedalPoints` (`0x088bf530`): gold 3, silver 2, bronze 1. Both are
covered by unit tests against the invented fixture, including three cases
chosen specifically to fail if the direction flip is dropped or wrongly
applied to a non-counting mode - see `crates/tables/src/race_campaign/tests.rs`.

`evaluate_medal` treats `value <= 0` as "no result yet", alongside the
original's own literal `0xFFFF_FFFF` unset-`u32` sentinel. **The `<= 0`
guard is this reimplementation's own choice, not measured** - the original
checks only the exact value `0`, but no mode here ever legitimately measures
a negative position, time, zone count or kill count, so the wider guard
changes no real value the original could produce.

## What this module still does not implement

The unlock-points comparison across a whole grid (`Unlock_GridPointsMet`),
and - more importantly for wiring a race - **picking which cell a real race
was run against, and what value that race actually scored in
`evaluate_medal`'s own terms.** Neither the launch path (which globals
`Cell Selection` writes, per the `campaign` handover thread's own "Next
Steps") nor the mapping from `oag_race::Mode`/`crate::catalogue::Track` back
onto a `Cell` was traced this pass. `crates/game/src/records.rs` carries the
persistence half - a `Medal` and its points, additive on the existing
per-circuit/mode/class row - and documents exactly where it is, and is not,
fed yet; see `docs/architecture/persistence.md`.

## Two open questions this pass did not resolve

- **`Status`/`Locked` on a cell, `Locked`/`Group` on a grid** are parsed (all
  four fields exist on the types) but no consumer of any of them was traced by
  the Ghidra pass, so nothing here interprets them either - see the parent
  page's "What is not determined" section.
- **Only the USA PSP pressing is validated.** EU, PS2, Pure and HD/Fury were
  not checked for `Data\Plugins\grids` at all.
