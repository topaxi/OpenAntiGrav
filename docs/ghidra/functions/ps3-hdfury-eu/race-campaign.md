# PI_Cell's campaign medal law: the `NitroElim*` triple is measured, its consumer is not

`oag_tables::race_campaign::Cell::nitro_elimination_targets` (a `(novice, skilled,
elite)` triple read off `<NitroElimNovice>`/`<NitroElimSkilled>`/`<NitroElimElite>`)
was carried since 2026-09 as **chosen, not measured**: real on `Elimination`/
`NitroBattle` cells, dummy `1`/`1`/`1` elsewhere, but with no executable evidence
that the field means what its name implies. This page closes the "is it a real
`PI_Cell` attribute" half of that question and documents why the other half - the
function that actually compares a race result against it, and by what rule - is
still open.

## The attribute table: `0x008ae898`

`PI_Cell`'s constructor (`FUN_001e2960`, decompiled below) installs
`PTR_PTR_008ae898` as the class's type/reflection descriptor the moment a `PI_Cell`
named `"PI_Cell"` (as opposed to a freed one, `param_2 == 0xffff` with `param_1 ==
0`) is destroyed - the only place in the image this pointer is referenced besides
`get_xrefs_to` on the descriptor address itself:

```
void _opd_FUN_001e2960(int param_1,int param_2)
{
  ...
  else if ((param_1 == 0) && (param_2 == 0xffff)) {
    puVar1 = (undefined4 *)(PTR_DAT_008ae940 + 0x78);
    *(undefined **)PTR_DAT_008ae940 = PTR_PTR_008ae898;
    FUN_00676198(puVar2,*puVar1);
    ...
```

Reading `0x008ae898` directly as raw memory (`read_memory`, not disassembly -
Ghidra has not typed this region) gives an array of 4-byte pointers into the
string pool, and dereferencing each one in order gives:

```
..., Values, Laps, Weapons, Damage, AICount, Status, Locked, ShipChoice, Track,
Ship, skillMedium, skillEasy, skillHard, Mode, Class, Gold, Target, Silver,
Bronze, HardGold, HardSilver, HardBronze, MediumGold, MediumSilver, MediumBronze,
EasyGold, EasySilver, EasyBronze, NitroElimElite, NitroElimSkilled,
NitroElimNovice, TournamentTrack, track, ...
```

This is `PI_Cell`'s own attribute-name table, read from the executable's raw
memory rather than decompiled - every name in it is an attribute
`oag_tables::race_campaign::parse` already reads (`Values`'s own children through
`TournamentTrack`), in what reads as C++ member-declaration order, not XML
document order or alphabetical order. **`NitroElimElite`/`Skilled`/`Novice` sit
in this table as three ordinary `PI_Cell` attributes, immediately after the
`EasyGold`..`EasyBronze` block and immediately before `TournamentTrack`** - not
metadata for an unrelated subsystem that happens to share the spelling. This is
what the finding upgrades: not "the file authors these three elements" (already
known from the shipped XML), but "the executable's own `PI_Cell` class declares
them as its own fields, in the same table as the medal-target attributes it is
already known to read."

| Address | Kind | Name | Confidence |
| --- | --- | --- | --- |
| `0x008ae898` | data | `PI_Cell_AttributeTable` | 88 |

88, not higher, because the table's *own* type (string pointer vs. an
{name,offset,type,default} descriptor of some width) was not fully decoded - see
"What is not determined" below - so its role as "the reflection binder's own
property list" is inferred from the string content and ordering matching known
attributes exactly, not from finding the generic binder function that walks it.

### A red herring worth recording: these strings are pooled, and the pool is shared with an unrelated subsystem

Searching cross-references *to the string addresses themselves* (rather than to
the table) for `NitroElimNovice`/`Skilled`/`Elite`, and for `Gold`/`Silver`/
`Bronze`/`Target`/`HardGold`/`EasyGold` etc., resolves almost entirely to a large
family of near-identical constructor stubs in the `0x0059d5xx`-`0x0059fxxx` range
(e.g. `FUN_0059e124`, `FUN_0059dfec`), each calling a base constructor, a
`"value"` query, and something taking a `"submitAsEncrypted"` flag - a PSN
stat/leaderboard registration system (`GoldTarget`, `HardGold`, `ZoneGoldMedal`,
`DetonatorGoldMedal` etc. are online-stat category names, not XML parse call
sites) that happens to reuse the identical string literals via pool
deduplication. **Chasing these xrefs first cost most of this pass's Ghidra time
and found nothing about the medal law** - the actual `PI_Cell` attribute table is
a *data* cross-reference (`get_xrefs_to` on the string address, filtered to
`[DATA]` rather than `[PARAM]`/`[READ]`), not a code one, because the binder that
reads this table is reflection-driven and never embeds the attribute name as a
literal at its own point of use. Recorded so nobody re-runs the string-xref sweep
expecting a different answer.

## `HARD` ≡ `ELITE`: the rung-name equivalence, measured

```
void _opd_FUN_0001cbe0(int param_1)
{
  if (*(int *)(param_1 + 0x154) != 0) {
    if (*(int *)(param_1 + 0x154) != 1) {
      FUN_00676208(PTR_s____you_have_up_to_date_stats_008a5978);
    }
    return;
  }
  FUN_00676208(PTR_s____updating_all_timetrial__speed_008a5974);
  iVar3 = _opd_FUN_0015d490(local_70);
  if (0 < iVar3) {
    piVar1 = *(int **)(iStack_b4 + -0x7b80);
    iVar7 = 0;
    do {
      iVar4 = _opd_FUN_0015d2e0(local_6c,*(undefined4 *)(iVar7 * 4));
      if (0 < iVar4) {
        iVar8 = 0;
        do {
          while( true ) {
            iVar2 = *(int *)(*(int *)(iVar8 * 4) + 0x110);
            if ((((iVar2 != 5) && (iVar2 != 10)) && (iVar2 != 6)) ||
               (cVar5 = _opd_FUN_001e1138(), cVar5 == -1)) break;
            ...
            *(undefined1 *)(piVar6 + 2) = 2;
            ...
```

`0x00779a98` is the string `"-- updating all timetrial, speed lap and zone
campaign cells with medals to have HARD(ELITE) for best skill level"` -
this function's own purpose statement, printed once (a version-gate at
`param_1+0x154`) before it walks every grid's cells and, for a cell whose mode
ordinal at `cell+0x110` is `5`, `10` or `6`, forces its saved record's own medal
byte to `2` if the player already has one.

Two things are directly measured here, not inferred:

1. **`HARD` and `ELITE` name the same rung.** The debug string is the function's
   own description of what it does, and what it does is act only on cells whose
   mode is `5`/`10`/`6` while writing that up as "HARD(ELITE)" - the same
   equivalence `docs/ui/campaign-screens.md`'s RPCS3 capture already showed at
   the UI layer (`TARGET 200 (NOVICE)` → `TARGET 200 (SKILLED)`, footer legend
   `DIFFICULTY (NOVICE)`/`DIFFICULTY (SKILLED)`), now corroborated from the
   executable's own text rather than only a screen capture. `Easy`↔`Novice` and
   `Medium`↔`Skilled` follow from the same three-rung vocabulary pairing, not
   independently measured on their own.
2. **HD reuses Pulse's own mode ordinals for `TimeTrial`/`SpeedLap`/`Zone`.**
   `*(cell+0x110)` compared against `5`, `10`, `6` in the same order the string
   lists "timetrial, speed lap and zone" matches
   `oag_tables::race_campaign::Mode::TimeTrial`/`SpeedLap`/`Zone`'s own ordinals
   (`5`/`10`/`6`) exactly - the same table `docs/ghidra/functions/psp-pulse-usa/`
   measured on Pulse's binary. `Elimination = 8` is not re-derived here; see
   `mode-manager.md`'s own `MPElimination`/`SPElimination` rows for that
   confirmation instead.

This function is a one-time save-data migration (version `0` → `1`): a player who
already earned a medal on a `TimeTrial`/`SpeedLap`/`Zone` campaign cell under
whatever scheme predated per-difficulty targets gets that medal credited at the
hardest rung. **It says nothing about `Elimination`/`NitroBattle`** - those two
modes are conspicuously absent from the migration's own `5`/`10`/`6` list, which
is consistent with (but does not prove) their medal law never having gone through
the flat-to-per-difficulty schema change this migration exists to paper over.

| Address | Kind | Name | Confidence |
| --- | --- | --- | --- |
| `0x0001cbe0` | function | `SaveData_MigrateCellMedalsToHardElite` | 85 |

85: the decompiled body, the call sites' mode-ordinal literals, and the
function's own debug string all agree with each other and with the independently
measured Pulse mode-ordinal table; the only thing not directly observed is the
saved-record struct's own field layout beyond the one byte this function writes.

## What the disc's own data says these two fields mean, cross-checked

Not a Ghidra finding, but the fact this page's medal-law question turns on:
reading `DATA00.PSARC`'s eight Fury grids directly (`oag_hd::open` +
`race_campaign::from_blob`, see
`crates/hd/tests/campaign_grids_ground_truth.rs`'s
`eliminationfamily_cells_carry_a_real_nitro_triple_and_a_dummy_flat_one`) shows:

- `grid8_3_2` (`Elimination`): `nitro_elimination_targets = (200, 200, 200)`;
  `difficulty_targets` is `MedalTargets{1,2,3}` on **every** rung (easy, medium
  *and* hard identically) - the same dummy shape [`Cell::gold`]'s own flat
  fields take.
- `grid8_2_1` (`Other("NitroBattle")`): `nitro_elimination_targets = (12, 15,
  20)`; `difficulty_targets` again `{1,2,3}` on every rung.
- `grid8_4_3` (`Other("Detonator")`): the exact inverse -
  `nitro_elimination_targets = (1, 1, 1)` (dummy), `difficulty_targets` real and
  six-figure (`easy 100000/90000/80000`, `medium 180000/170000/160000`,
  `hard 340000/330000/320000`).

Cross-checked against `docs/ui/campaign-screens.md`'s 2026-09-21 RPCS3 capture of
this exact cell (`Fury`'s `Eliminator`, `grid8`): the screen read `TARGET 200` at
both the `NOVICE` and `SKILLED` rung presses. `200` matches only the nitro
triple; `difficulty_targets`' dummy `1`/`2`/`3` would have shown `TARGET 1`. This
is what rules out the display (and, by the same reasoning, any consumer) reading
`difficulty_targets` or the flat `gold`/`silver`/`bronze` fields for this cell -
not proof of which function does the reading, but proof that whatever does, it
is not those two.

## What is not determined

**The comparison consumer itself was not found.** `PI_Cell`'s attribute binder is
reflection-driven (see the "red herring" section above): nothing in the image
references `NitroElimNovice`/`Skilled`/`Elite`'s string addresses from a
comparison or evaluate-style function, because the binder reads attribute values
generically through `0x008ae898`'s own table rather than through per-attribute
code. Reaching the actual consumer needs the table's per-entry encoding decoded
first (each entry past the bare name pointers appears to carry further words -
e.g. a `-100.0f`/`1.0f` default-value pair was observed immediately after the
`Gold`/`Target`/`Silver`/`Bronze` block - not yet attributed to specific fields),
then finding what reads the resulting struct offset. Not attempted this pass;
flagged for whoever continues, starting from `0x008ae898`.

**Whether the triple is three medal tiers or one target per rung is
unmeasured, and the disc's own vocabulary argues for the latter.** This
executable spells medal tiers `Gold`/`Silver`/`Bronze` everywhere observed
(`EasyGold`, `MediumSilver`, `HardBronze`, the `GoldTarget`/`SilverTarget`/
`BronzeTarget` stat names, `ZoneGoldMedal` etc.) and spells difficulty rungs
`Easy`/`Medium`/`Hard` ≡ `Novice`/`Skilled`/`Elite` (this page's own finding
above). `NitroElimNovice`/`Skilled`/`Elite` is spelled in **rung** vocabulary,
not tier vocabulary, and sits in the attribute table immediately after the
`Easy`/`Medium`/`Hard` block rather than beside `Gold`/`Silver`/`Bronze`. Reading
it as "one number per rung" rather than "a fourth three-tier medal ladder" is the
plain reading of that vocabulary, not the reading this page can independently
confirm from a decompiled comparison. **No Gold/Silver/Bronze-named online stat
category exists anywhere in the image for `Elimination` or `NitroBattle`**
(there is a `ZoneGoldMedal`, `DetonatorGoldMedal`, `SpeedLapGoldMedal` - no
`EliminationGoldMedal` or `NitroBattleGoldMedal`), which is consistent with
these two modes never producing a three-tier medal through this mechanism at
all, though absence of a string is not proof of absence of the mechanism.

Given both of the above, `oag_tables::race_campaign::Cell` gained
[`Cell::nitro_elimination_target_for_difficulty`] returning the single measured
number per rung, **not** a synthesized `MedalTargets` triple - manufacturing a
gold/silver/bronze split out of one value would be inventing data the disc does
not author. `Cell::evaluate_medal` itself is unchanged: Pulse's own `Eliminator`
cells (kill target in the flat `gold` field, confidence 50 per
`docs/ghidra/functions/psp-pulse-usa/race-campaign.md`, pinned by that crate's
ground-truth tests) still need its current behaviour, and this pass found no
evidence HD's own consumer routes through the same function at all.

**Archive copy and difficulty-rung selection for `grid_00.xml`..`grid_07.xml`**
(priority 3 of this lane) was not reached this pass - see
`docs/ui/campaign-screens.md`'s HD section for the RPCS3-side half of that open
question.

## The medal-evaluate function is found, and it always reads one shared rung - 2026-09-28

The player-facing observation this pass started from - HD's medals are earned
per difficulty, and each difficulty's medal has a different icon shape - is
confirmed on the icon-shape half (see `docs/ui/campaign-screens.md`'s "One
shape per difficulty" section for the texture evidence). This section covers
the executable side: the comparison consumer the section above left
unfound, and what it does and does not settle about *storage*.

`get_xrefs_to` on `PTR_DAT_008ae894` (the lookup table `SaveData_MigrateCellMedalsToHardElite`
reads through) turns up a small, closed family of accessors, all at
`0x001e1xxx`/`0x001e2xxx`:

| Address | What it does |
| --- | --- |
| `0x001e1138`/`0x001e1188` | `TryGetValue`-shaped: fetch a cell's own saved record by its `+0x58` id, copy `0x48` bytes to the stack, return one byte of it (`+9` and `+8` respectively - adjacent fields, not independently confirmed as *which* two fields). |
| `0x001e11e0` | Same fetch; returns the record's raw first field (a `u32`) for `Mode::Elimination`/`3`/`9`/`0xd`, else `0xffffffff`/`0xffffffff`-shaped sentinel through a different arm. Not fully read this pass. |
| `0x001e25c0` | **`Cell_EvaluateMedal`'s own HD analogue.** Fetches the record, reads its raw `u32` value, then compares it against `param_1 + (*(GameState+0xdc) * 0xc) + {0xb0,0xb4,0xb8}` - three `u32` targets `0xc` (12) bytes apart, exactly [`oag_tables::race_campaign::MedalTargets`]'s own three-field-per-rung shape - returning tier `0`/`1`/`2` (gold/silver/bronze) or `0xff`. The comparison flips (`>=` instead of `<=`) for `*(cell+0x110) == 6` (`Mode::Zone`) or `== 0xe` (14 - not `Mode::Elimination`'s own `8`; unidentified, possibly `NitroBattle`/`Detonator`, not chased further). |
| `0x001e1028` | The same compare, `param_2` (a raw value) in place of the record fetch - a "what would this score earn" pure function, same `GameState+0xdc` selector. |
| `0x0001c6d0` | Walks a whole cell list, calls `0x001e25c0` on each, and OR's a per-cell bit into a six-word, 192-bit mask (`param_1+0x434..+0x448`) wherever the result is not `0xff`. Read directly against `FUN_0015e638`'s own list, not yet confirmed as *which* list (a whole campaign profile's cells fit the bit count; a single grid's own cell count does not need 192 bits) or what the mask itself feeds - not the medal *tier*, only "has one".

None of these five is named or entered in `names.tsv` this pass: `0x001e25c0`
is legible enough to describe with confidence, but naming it
`Cell_EvaluateMedal` invites conflating it with the identically-named
Pulse function at a different address on a different binary, and the other
four turned up answering a narrower question (see below) than the one this
section opened with, not a clean enough read of their own purpose to commit
a name past `_q`. Flagged for whoever next has Ghidra time on this program.

**What this settles: every medal evaluation this pass found reads one
GameState-held difficulty (`PTR_g_GameState_008ae88c + 0xdc`), not a
per-cell stored one.** Both `0x001e25c0` (record-backed) and `0x001e1028`
(value-backed) multiply the *same* field by `0xc` and add it to the *cell's*
own base pointer - a single scalar, read fresh each call, not indexed by
which cell is being evaluated. Confidence 75 that this field is HD's own
`CellSelection::difficulty`-equivalent (a currently-browsed/selected rung,
not a per-profile setting) - consistent with, but not independently
confirmed against, a live capture of `DifficultyButton` changing what a
fresh, unearned target row shows.

**What this does not settle, and why this pass stops here rather than
picking a side.** `SaveData_MigrateCellMedalsToHardElite` writes a specific
per-cell record *byte* (`piVar6+0x6c` relative to its own found-record base,
adjacent to but not confirmed identical in addressing to the `+8`/`+9`
bytes `0x001e1138`/`0x001e1188` read) only for `TimeTrial`/`SpeedLap`/`Zone`
cells that already have a medal - modes whose raw value does not depend on
AI skill, unlike a `Race`/`Elimination` placement, which is consistent with
that byte meaning "the rung this stored value should be judged against" (a
real per-cell field, worth keeping). But no function this pass found *reads*
that byte to select a rung the way [`oag_tables::race_campaign::Cell::evaluate_medal_for_difficulty`]
does - every evaluator instead reads the one shared `GameState+0xdc` value.
Reconciling "a migration writes a per-record rung byte" with "no evaluator
reads a per-record rung byte" needs the saved-record struct's own field
layout pinned down past what `0x48`-byte blind copies give here - specifically
whether `0x001e1138`'s `+9` and the migration's `+0x6c` (relative to a
*different* found-record base - a linked-list node, not the hashmap
`TryGetValue` these accessors use) are the same field at all. **Not
attempted further this pass** - the check that would settle it (five Ghidra
calls: the `Medal_%d_%d`/`Medal_%d` widget-name strings at `0x0078b138`/
`0x00797b70` have only `[DATA]` xrefs, so the draw site is reflection-bound
the same way `PI_Cell`'s own attribute table is, and neither string search
nor `GameState+0xdc`'s own two readers led to a badge-draw or
`DifficultyButton`-handler function within that budget) is flagged here for
whoever picks this up with more Ghidra time.

**Consequence for this project's own implementation, stated plainly**:
`oag_game::records::CampaignRecord::best_difficulty` (a per-cell stored
rung, never downgraded, tie-breaking a harder rung over a better medal at
an easier one) and `oag_ui::campaign::hd::hd_medal_frame`'s own
earned-difficulty keying for `Medal_{x}_{y}` are this project's own
**chosen, not measured** design - plausible, and not contradicted by
anything found this pass, but not read off a confirmed original consumer
either. See that struct's and that function's own doc comments for the
full reasoning `docs/ui/campaign-screens.md` and this page together give.
