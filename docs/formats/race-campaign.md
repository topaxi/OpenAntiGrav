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
its points table are implemented here, on the type they evaluate - see
below. **2026-09-23: the unlock-points comparison across a whole grid
(`Unlock_GridPointsMet`) is too** - see
["The unlock gate, implemented"](#the-unlock-gate-implemented-grid_points_met)
below. Picking *which* cell a race is run against, and what value that race
actually scored, is `crate::main::session::campaign::launch_campaign_cell`'s
job (`oag_game`) - see `docs/architecture/persistence.md`'s "A campaign cell
now reaches `Observation::campaign_medal` for real".

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

## The unlock gate, implemented: `grid_points_met`

`oag_tables::race_campaign::grid_points_met(grids, target, medal_of)`
reimplements `Unlock_GridPointsMet` (`0x0888ebd8`) directly: find the grid
among `grids` whose own `name` matches `target` case-insensitively - `Grid0`
in an `<Unlock Grid="Grid0"/>` row against `grid0` in `grid_00.xml` - and pass
when that grid's own `Grid::points_earned` (`Grid_PointsEarned`, the sum of
`medal_of`'s answer per cell, converted to points) reaches its own
`required_points`. `target` is read straight off the disc's own
`Grid::unlock_grid`, never a hand-typed table. Unit-tested against an
invented fixture in `crates/tables/src/race_campaign/tests.rs` and against
the real disc's own `grid0`/`grid1` pair in
`crates/tables/tests/race_campaign_ground_truth.rs`
(`grid_points_met_reproduces_the_unlock_ladder_on_real_data`).

**Not implemented: `Unlock_GridName`'s own `asSelected` reading** - the
literal string a caller's own currently-selected grid, rather than a name -
since no `<Unlock Grid="...">` row on any of the sixteen shipped grids
authors it. See that function's own doc comment.

`oag_game::main::campaign_stage::CampaignStage::grid_is_unlocked` is the
caller: it is what `Session::handle_campaign` gates `Grid Selection`'s own
Confirm on, in place of the display-only lock-glyph shortcut
(`GridSelection::tier_shows_lock`, `GridSelection_PopulateTiles`'s own
"previous tile" comparison) that mechanism used to reuse. The two predicates
cannot disagree on the shipped disc - every grid from `grid1` on names
exactly the tile before it - but `grid_points_met` is the recovered law
itself, not a shortcut that happens to reproduce it; see
`docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s "the tier unlock
rule" for why the two are only equivalent by construction on this data.

Picking *which* cell a real race was run against, and what value that race
actually scored in `evaluate_medal`'s own terms, is
`crate::main::session::campaign::launch_campaign_cell` and
`RaceStage::observation`'s job (`oag_game`) - see
`docs/architecture/persistence.md`'s "A campaign cell now reaches
`Observation::campaign_medal` for real" for the full per-mode mapping.

## Wipeout HD and Fury: the same schema, extended, split across four archives

**Status: understood, for the schema. The medal *icon* is confirmed
per-difficulty (2026-09-28, see "One shape per difficulty" in
`docs/ui/campaign-screens.md`); the *evaluate/award* law's difficulty axis
is still open past "every evaluator reads one `GameState`-held rung, not a
per-cell stored one" -
`docs/ghidra/functions/ps3-hdfury-eu/race-campaign.md`'s "The medal-evaluate
function is found" section has the full account, including what stays
unsettled.** Measured 2026-09-14 against `hdfury-ps3-eu-dec.iso`
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
| `DATA02.PSARC` | `grid_00`..`grid_07` | 87 | flat, Pulse's own shape | absent |
| `DATA04.PSARC` | `grid_00`..`grid_07` | 87 | per-difficulty | absent |
| `DATA06.PSARC` | `grid_00`..`grid_07` | 87 | per-difficulty | `"HD"` |

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

**The engine reads `grid_00`..`07` off `DATA06` since 2026-10-08** (`Title::campaign.grid_archive`,
`oag_hd::campaign::GRID_ARCHIVE`; PSN installs keep precedence). Measured over all 87
cells (`crates/game/tests/hd_campaign_ground_truth.rs`): `DATA02`'s flat
`Gold`/`Silver`/`Bronze` equals `DATA06`'s **hard** rung on every Time Trial, Speed Lap
and Zone cell (`grid0_2_2`: flat 111.00/114.00/120.00 s, easy 120.00/123.00/129.00 s), so
before this a novice, the default rung, was judged and shown the elite times. `Race` and
`Tournament` cells author no rungs (1st/2nd/3rd). Confidence 85 that `DATA06` is the
copy the original plays: it is the copy `Campaign Selection` and `Cell Selection` already
read, tagged `Campaign="HD"`. No capture compared a base-campaign target number.
Omega: `grid_archive` is `None`, its grids read by precedence (`checked, differs`:
one archive set).

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
`20`/`15`/`12` on a `NitroBattle` one).

**2026-09-21, upgraded from "chosen, not measured" to measured, in part**: on
`EBOOT-ps3-hdfury-eu.elf`, `PI_Cell`'s own attribute-name table
(`0x008ae898`, read directly as memory) lists `NitroElimElite`/`Skilled`/
`Novice` as three ordinary `PI_Cell` fields, immediately after the
`EasyGold`..`EasyBronze` block - not a coincidental reuse of the spelling by
an unrelated subsystem. A second, independent string
(`0x00779a98`, in a save-data migration function) reads `"...to have
HARD(ELITE) for best skill level"`, measuring `Novice`/`Skilled`/`Elite` as
this title's own words for `Easy`/`Medium`/`Hard` - the same equivalence
`docs/ui/campaign-screens.md`'s RPCS3 capture already showed on screen. See
`docs/ghidra/functions/ps3-hdfury-eu/race-campaign.md` for the full evidence.

**Still chosen, not measured**: whether `elite` is this triple's
gold-equivalent (or whether the triple represents medal tiers at all - see
below), and which mode's own medal-award consumer reads this triple instead
of `gold`/`silver`/`bronze`. `PI_Cell`'s attribute binder is reflection-driven,
so no function in the image references these three attributes' names at their
point of use; finding the consumer needs the attribute table's own per-entry
encoding decoded first - not reached this pass. `Cell::evaluate_medal` is
unchanged, and its result is **not meaningful** for an `Elimination`/
`NitroBattle` cell on Wipeout HD, since that cell's own flat
`gold`/`silver`/`bronze` (and every rung of `difficulty_targets`) are the
dummy `1`/`2`/`3` this section already describes.
**Since 2026-10-08 `Cell::evaluate_medal_for_difficulty` returns `None` for such a cell**
(`Cell::medal_law_is_unmeasured`: `Elimination` or `NitroBattle` with a `NitroElim*` triple
other than `1/1/1`), and `plan_cell` leaves the race's own kill target alone: before, one
kill scored gold on every rung and a launched cell ended on the first kill (its kill
target was the dummy `1`). The unit of `200`..`300` (kills or score points) is the open
question. Pulse's `Eliminator` is unaffected. Omega: ported, the guard is on the shared
`Cell` (`omega_campaign_launch_ground_truth.rs`).
`Cell::nitro_elimination_target_for_difficulty` reads the one measured number
per rung instead - deliberately not a synthesized `MedalTargets` triple, since
this executable spells medal tiers `Gold`/`Silver`/`Bronze` and difficulty
rungs `Easy`/`Medium`/`Hard`/`Novice`/`Skilled`/`Elite` as two distinct
vocabularies everywhere else observed, and `NitroElim*` is spelled in the rung
vocabulary. See that method's own doc comment for the full reasoning and the
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

### `grid_04.xml`'s own `<Values>` tag is broken on the disc, and the original tolerates it

In **all three** of its copies (`DATA02`, `DATA04`, `DATA06`, byte-identical
at the break): `...vex"</Values>` where every other grid in the corpus reads
`...vex"></Values>` - the opening `<Values>` tag is missing its closing `>`
before `BillboardName`'s attribute value ends. Read structurally that
swallows the text meant to close `<Values>` into the start tag's own
attribute soup, so the ten `<PI_Cell>` elements that follow become
descendants of the still-open `<Values>` node rather than of `<PI_Grid>` -
which is what `race_campaign::parse` used to read as **zero cells** for a
grid that authors ten (an earlier pass of this doc counted five, a miscount
against the actual archive contents; the direct read is ten).

**Whether Wipeout HD's own XML reader tolerates the same malformed tag was
this section's own open question, and it is settled now: it does.**
`Campaign Selection`'s own `GOLD MEDALS` widget draws a campaign's total cell
count as its denominator (every cell carries exactly one `<Gold>` target),
and RPCS3 reads `"0 / 87"` for the base `Wipeout HD` campaign - the sum of
`grid0`..`grid7`'s own eight cell counts, `grid4`'s ten included, not the 77
a parser that drops them totals. That is first-party evidence about the
original's own parser, not a guess at which `<PI_Cell>` boundary it intended,
so `oag_tables::fexml::tag_end` now recovers the same way: an unquoted `<`
(this exact shape, an attribute value run straight into the next tag's own
`<`) ends the tag it is in, one character short of it - the same place the
missing `>` should have been. `the_precedence_resolved_campaign_is_sixteen_grids_mixed_schema`
and `every_grid_file_on_every_archive_parses` both assert the corrected
counts (`grid4` = 10, `DATA02`/`04`/`06` = 87 each) against the real disc.

### Does any other title have a per-difficulty medal? No - checked directly, 2026-09-28

**Pulse**: no. Every Pulse cell authors exactly one `<Gold>`/`<Silver>`/
`<Bronze>` triple - `Cell::difficulty_targets` is `None` on every row this
crate has ever parsed off a Pulse disc, and `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`
never found a difficulty-rung attribute on `PI_Cell` at all. There is
nothing for a per-difficulty icon to key on.

**Pure**: not reachable to check. This project's own Pure build boots to a
Time Trial and never reaches a campaign screen at all
(`docs/overview/status.md`'s M8 row), and no `race_campaign` parse exists
for it in this crate. Absence of evidence, not evidence of absence - Pure's
own disc was not read for this question.

**2048**: no, and not the same *shape* of law even where it might look
similar. `oag_2048::campaign::evaluate_tier` is a two-rung `Pass`/`Elite`
**score bar**, not a three-rung `Easy`/`Medium`/`Hard` **difficulty** -
`Tier::Elite` maps to `Medal::Gold` and `Tier::Pass` to `Medal::Bronze`
purely for this project's own `records.rs` bookkeeping
(`docs/architecture/persistence.md`'s 2048 paragraph), not because 2048's
own objective law has a middle rung or a selectable AI difficulty to earn
either bar against.

### What this section does not resolve

- **The medal-award consumer for `Elimination`/`NitroBattle`** - which
  function compares a kill count against `NitroElimNovice`/`Skilled`/`Elite`,
  and by what rule one number per difficulty rung becomes a medal - see above
  and `docs/ghidra/functions/ps3-hdfury-eu/race-campaign.md`'s "what is not
  determined" section.
- **US and Fury-disc-only pressings were not checked.** Only the EU Fury
  pressing (`hdfury-ps3-eu-dec.iso`) was measured; a base-HD-only disc (no
  Fury update) was not available to this pass, so whether `DATA00`'s
  Fury-only grids are present on one is unknown.

## Two open questions this pass did not resolve

- **`Status` on a cell, `Group` on a grid** are parsed but no consumer of
  either was traced by the Ghidra pass, so nothing here interprets them
  either - see the parent page's "What is not determined" section.
  **`Locked`, on both `PI_Cell` and `PI_Grid`, is no longer in this list** -
  the parent page's "Unlock rules, cell and tier" section settled a real
  consumer for each (the lock glyph, and this engine's own Confirm gate,
  `grid_points_met`/`CampaignStage::grid_is_unlocked`), at confidence 82-85.
- **Only the USA PSP pressing and the EU PS3 Fury pressing are validated.**
  EU PSP, PS2 and Pure were not checked for `Data\Plugins\grids` at all.
