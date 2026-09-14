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

## Wipeout HD and Fury: the same schema, extended, split across four archives

**Status: understood, for the schema; the medal law's difficulty axis is
unmeasured.** Measured 2026-09-14 against `hdfury-ps3-eu-dec.iso`
(EU, decrypted) in
`crates/hd/tests/campaign_grids_ground_truth.rs` (`#[ignore]`d, `just
test-data`), with HD's own entry names and archive facts in
[`oag_hd::campaign`](../../crates/hd/src/campaign.rs) per ADR-0022, mirroring
[`oag_pulse::campaign`](../../crates/pulse/src/campaign.rs).

**The discriminating question this thread opened with** - does
`race_campaign::from_blob` parse HD's grids unchanged, or does the schema need
extending - answers *extending*, additively: three new optional [`Cell`]
fields, one new [`Mode`] variant, five new optional [`Grid`] fields, and one
attribute alias. No existing Pulse field changed type or meaning, and
`crates/tables/tests/race_campaign_ground_truth.rs` passes unchanged.

### The campaign is on four archives, not one, and they disagree

`Data\Plugins\grids\` is on `DATA00.PSARC`, `DATA02.PSARC`, `DATA04.PSARC` and
`DATA06.PSARC` (of the title's seven), and confirms
`docs/formats/hd-status.md`'s reading of the schema at a glance - but not at
the file level:

| Archive | Grids | Cells | Schema | `Campaign=` |
| --- | --- | --- | --- | --- |
| `DATA00.PSARC` | `grid_08`..`grid_15` (Fury-only - absent from every other archive) | 80 | per-difficulty | `"Fury"` |
| `DATA02.PSARC` | `grid_00`..`grid_07` | 77 | flat, Pulse's own shape | absent |
| `DATA04.PSARC` | `grid_00`..`grid_07` | 77 | per-difficulty | absent |
| `DATA06.PSARC` | `grid_00`..`grid_07` | 77 | per-difficulty | `"HD"` |

Sixteen grids either way - the same count as Pulse's campaign - but
`oag_assets::Archives`'s own precedence (`data` = `DATA00`, `fe` = `DATA02`,
then `DATA01`/`DATA03`/`DATA04`/`DATA05`/`DATA06`) reaches `DATA00` for
`grid_08`..`15` and falls through to `DATA02` for `grid_00`..`07`, since
`DATA00` does not carry those eight at all. **So the campaign a normal read of
this title reaches is genuinely mixed-schema**: the first eight grids flat,
the last eight per-difficulty - a fact about this ordering, the same shape
`hd-hud.md` and `oag_hd::names` already record for the language table and the
front-end `Definition.xml`, not a measurement of what a PS3 loads.
`DATA04.PSARC`'s and `DATA06.PSARC`'s own per-difficulty copies of
`grid_00`..`07` are real and on the disc, just not reached by that precedence;
`every_grid_file_on_every_archive_parses` opens all four directly to prove the
schema on data the precedence never touches.

`DATA00`'s own `Definition.xml` is the one `Archives` resolves to for that
path (the fullest, per the same rule that picks its copy of the front-end
plugin definition), and is the only one of the three copies (`DATA00`,
`DATA02`, `DATA04`) naming all sixteen grids; `DATA02`'s and `DATA04`'s own
copies list only the eight they carry.

### The per-difficulty schema

`grid_08`..`grid_15`, and the `DATA04`/`DATA06` copies of `grid_00`..`07`,
replace each cell's single `<Gold>`/`<Silver>`/`<Bronze>` with three triples -
`<EasyGold>`/`<EasySilver>`/`<EasyBronze>`, `<Medium...>`, `<Hard...>` - one
set of medal targets per difficulty rather than one for all three. `Cell::gold`/
`silver`/`bronze` keep meaning the `medium` rung when a cell authors this
shape, so every existing caller of `evaluate_medal` (which takes one target
triple) keeps working unchanged; `Cell::difficulty_targets` carries the full
`DifficultyTargets { easy, medium, hard }` for a caller that wants the other
two, and `Cell::targets_for_difficulty(u8)` mirrors
`Cell::skill_for_difficulty`'s own difficulty-index convention for the target
side of a cell.

The same eight-generation cells also rename `skill` (Pulse's single
AI-scaling value) to **`skillMedium`** - read as an alias in
`Cell::skill`, tried first under `skill` and then `skillMedium`, never both on
one cell in what this pass measured.

**Every per-difficulty cell also carries a fourth, not-per-difficulty triple**:
`<NitroElimNovice>`/`<NitroElimSkilled>`/`<NitroElimElite>`, present on every
cell of a per-difficulty grid regardless of mode - a plain `Race` or `Speed
Lap` cell included, authored at a dummy `1` there, same as the equally dummy
`1`/`2`/`3` the per-difficulty `Gold`/`Silver`/`Bronze` rungs carry on those
same cells. Only `Elimination` and the HD-only `NitroBattle` mode carry
meaningful values (e.g. `200`/`200`/`200` on an `Elimination` cell measured, or
`20`/`15`/`12` on a `NitroBattle` one). **Confidence 0 - chosen, not
measured**: `Cell::nitro_elimination_targets` carries the raw
`(novice, skilled, elite)` triple and nothing here decides whether `elite`
is this triple's gold-equivalent, or which mode's own `evaluate_medal` call
should read this triple instead of `gold`/`silver`/`bronze` - that is a
decompiled-HD-executable question, out of this pass's scope; see the
handover thread this measurement opened.

### Two new mode spellings, kept raw

`"NitroBattle"` and `"Detonator"` are on the Fury-only grids (`grid8`
onwards) and have no entry in the Pulse `g_mode_name_table` reading this
crate's `Mode` enum is built from. Per this crate's own "a parser cannot fail
on a field it does not understand" rule - the same one `Cell::class`'s raw
`"Zone"` already follows - an unrecognised `mode=` is now `Mode::Other(name)`
rather than a hard parse error (`Error::UnknownMode`, kept in the `Error` enum
for API stability but no longer produced by this crate's own parser). No
shipped Pulse cell has ever authored an unrecognised mode, so this is a
change to behaviour on data nothing here has ever observed, not a change to
what a real Pulse cell reads as.

### New grid-level attributes, five of them, all front-end concerns

`Campaign="HD"|"Fury"`, `TitleColor`/`TextColor` (raw hex text, not parsed to
a colour - this title's channel order is not measured), `FlyerName` (the
cell-selection screen's own flyer image stem) and `BillboardName` (this
grid's advert `.vex`) are new `<Values>` attributes on `PI_Grid`, all read raw
into new optional `Grid` fields. None of the four analysed types
(`bool`/`u32`/`f32`/`i64`) apply to any of them, so nothing here parses them
further than a `String`.

### `grid_04.xml`'s own `<Values>` tag is broken on the disc

In **all three** of its copies (`DATA02`, `DATA04`, `DATA06`, byte-identical
at the break): `...vex"</Values>` where every other grid in the corpus reads
`...vex"></Values>` - the opening `<Values>` tag is missing its closing `>`
before `BillboardName`'s attribute value ends. Read structurally that
swallows the text meant to close `<Values>` into the start tag's own
attribute soup, so the five `<PI_Cell>` elements that follow become
descendants of the still-open `<Values>` node rather than of `<PI_Grid>`, and
`race_campaign::parse` correctly reads **zero cells** for a grid that authors
five. `the_precedence_resolved_campaign_is_sixteen_grids_mixed_schema` asserts
this directly, so a change to `oag_tables::fexml`'s tag scanner that started
tolerating the break would have to notice the assertion rather than silently
disagree with it.

**This is not a bug in this crate's parser.** The file is broken on the disc,
identically in three independent archive copies, and there is no way to
recover the intended cell boundaries without guessing which of the five
`<PI_Cell>` elements the original author meant to close `<Values>` before.
Whether Wipeout HD's own XML reader tolerates the same malformed tag - and so
whether the real game shows a five-cell `grid4` or an empty one - is a
decompiled-HD-executable question, out of this pass's scope.

### What this section does not resolve

- **Which target set a mode's own `evaluate_medal` reads** when a cell
  carries both the per-difficulty `Gold`/`Silver`/`Bronze` and
  `NitroElimNovice`/`Skilled`/`Elite` - see above.
- **Whether HD's own front end reads `grid4`'s five cells at all**, given the
  broken `<Values>` tag - see above.
- **US and Fury-disc-only pressings were not checked.** Only the EU Fury
  pressing (`hdfury-ps3-eu-dec.iso`) was measured; a base-HD-only disc (no
  Fury update) was not available to this pass, so whether `DATA00`'s
  Fury-only grids are present on one is unknown.

## Two open questions this pass did not resolve

- **`Status`/`Locked` on a cell, `Locked`/`Group` on a grid** are parsed (all
  four fields exist on the types) but no consumer of any of them was traced by
  the Ghidra pass, so nothing here interprets them either - see the parent
  page's "What is not determined" section.
- **Only the USA PSP pressing and the EU PS3 Fury pressing are validated.**
  EU PSP, PS2 and Pure were not checked for `Data\Plugins\grids` at all.
