# Race campaign: `PI_Grid`/`PI_Cell`, `Data\Plugins\grids\grid_00.xml`..`grid_15.xml`

**Status: understood.** Implemented in
[`oag_formats::race_campaign`](../../crates/formats/src/race_campaign.rs), with
the sixteen entry names in
[`oag_pulse::campaign`](../../crates/pulse/src/campaign.rs) per
[ADR-0022](../architecture/adr/0022-title-packages.md) - the file's shape is a
format fact, the sixteen names it ships are a title fact.

This is the schema half of
[`docs/ghidra/functions/psp-pulse-usa/race-campaign.md`](../ghidra/functions/psp-pulse-usa/race-campaign.md),
which carries the decompiled law (the medal evaluator, the points table, the
unlock gate) that this page's types deliberately do not implement - see
below.

## What is on disc

Sixteen files, `Data\Plugins\grids\grid_00.xml` through `grid_15.xml`, listed
by `Data\Plugins\grids\Definition.xml`, all inside `Data.wad`. Each is one
`PI_Grid` holding 8-16 `PI_Cell` records - **236 cells in total on the USA PSP
pressing**, confirmed by
`crates/formats/tests/race_campaign_ground_truth.rs` (`#[ignore]`d, run with
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
into `oag_formats::handling::SpeedClass`. `Cell::speed_class` does the
fallible mapping and returns `None` for `"Zone"` - an unrecognised value is
not an error, per this project's own established lesson that a parser must
not fail on a field it does not understand.

## What this module deliberately does not implement

`Cell_EvaluateMedal`, `Cell_MedalPoints` and the grid-unlock points comparison
are gameplay rules, not properties of the file, and wiring the campaign into a
race is explicitly a later pass (it collides with the `eliminator` and
`career` lanes running the same day this was written). This module surfaces
the authored numbers only: the three medal targets, `RequiredPoints`, the
`<Unlock Grid=>` name, `AICount`, `skill`/`skillEasy`/`skillHard`. What a
caller does with them is out of scope here.

## Two open questions this pass did not resolve

- **`Status`/`Locked` on a cell, `Locked`/`Group` on a grid** are parsed (all
  four fields exist on the types) but no consumer of any of them was traced by
  the Ghidra pass, so nothing here interprets them either - see the parent
  page's "What is not determined" section.
- **Only the USA PSP pressing is validated.** EU, PS2, Pure and HD/Fury were
  not checked for `Data\Plugins\grids` at all.
