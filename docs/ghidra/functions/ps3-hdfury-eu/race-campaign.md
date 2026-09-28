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

## Whether the `SkillScale`/`stats.xml` AI-difficulty mechanism is shared with Pulse - checked, not decompiled, 2026-09-28

A separate lane wired Pulse's own `AI_ResolveSkillScale`
(`docs/ghidra/functions/psp-pulse-usa/race-campaign.md`) into `oag-ai`
this pass, and checked whether this binary carries the same mechanism
before extending the wiring here. It does, by every string-level signal
this budget covers, but **no HD consumer function was found or
decompiled** - this is a narrower claim than the Pulse page's own,
recorded so the next pass does not have to re-run the same search.

**What is shared, confirmed by string search alone:**

- `SkillScaleValue` (`0x0078fb78`), `SkillScalePoint1`/`2`/`3`
  (`0x007809f8`/`0x00780a10`/`0x00780a28`) and `SkillScale` itself
  (`0x00780b88`) - the identical `AIControlStats.xml`/`AIRaceStats_<class>.xml`
  vocabulary `ai-stats.md` reads off the PSP binary.
- `skillMedium` (`0x0078f098`) - already used by
  `oag_tables::race_campaign::Cell::skill_for_difficulty` as this executable's
  own alias for Pulse's `skill` attribute, per that field's own doc comment.
- The literal format string `%s\stats.xml` (`0x0078fc20`) - the same
  per-track-directory template `TrackStats_Load` opens on the PSP build.

**What is not confirmed: the consumer, on either end.** Both addresses
`get_xrefs_to(0x0078fc20)` names as callers (`0x006fabe0`, `0x006fdca0`)
decompile as unrelated `FIOS` disk-cache flush routines - the same
unrelocated-address artifact `ai-stats.md`'s own "Read this first" section
documents for the PSP binary, now observed here too. Locating the real
reader needs the same route that worked on PSP: a live caller of whatever
this executable's own `AI_ResolveSkillScale` equivalent is, not a caller of
the loader - and that equivalent has not been named on this binary at all.

**Consequence for this project's own implementation**: `oag_game`'s
campaign-cell skill-scale resolution
(`Session::resolve_campaign_ai_skill_scale`,
`crates/game/src/main/session/campaign.rs`) is wired generically - it runs
for any campaign cell, HD's included, since `Cell::skill_for_difficulty`
already reads `skillMedium`. What is **not** wired is HD's own archive
path: the resolver only opens `PSP_GAME/USRDIR/FEData.wad`, which does not
exist on an HD source (seven `DATA0*.PSARC` archives, no WAD at all), so
an HD campaign launch today falls back to
`oag_tables::track_stats::resolve_skill_scale`'s own documented default
curve (`1.0`/`2.0`/`3.0`) rather than HD's real per-track `SkillScaleValue`
numbers - a real, but coarser, application of the same mechanism, until
someone finds which of HD's seven archives (and under which of HD's own
directory names - `docs/formats/race-setup.md`'s `01_Vineta_K`-style
example, not necessarily Pulse's own `NN_Track`) carries the `stats.xml`
family here.

## The TOC-xref trap: `get_xrefs_to` on `RB_AI_DIF`/`RB_DIF` resolves to the wrong function - 2026-09-28

This pass went looking for the executable's own choice between `RB_AI_DIF`
("AI DIFFICULTY") and `RB_DIF` ("DIFFICULTY") on `DifficultyButton`'s square-
button prompt (`docs/ui/campaign-screens.md`'s "Which block is which
difficulty" and its 2026-09-28 correction). `get_xrefs_to` on either string's
address (`0x00793218`/`0x00793228`) resolves to a single function,
`.opd.FUN_00621eb0` - and its decompile is Sulpha/DECI3 debug-port setup
code, wholly unrelated to any UI text. **This is not a one-off**: the same
tool on the widget-name string `"DifficultyButton"` (`0x00793020`) resolves
to `.opd.FUN_0061ff90`, a font-glyph-rendering loop, equally unrelated.

**Root cause, confirmed by comparing the decompiler's own output against the
xref tool's claim.** This binary's ABI is `PowerPC:BE:64:A2ALT-32addr` - the
Cell PPU's non-standard variant, where each function call goes through an
`.opd` descriptor pairing an entry address with its own TOC/`r2` base, and a
binary this size links multiple object files that do not all share one TOC.
Decompiling `FUN_00621eb0` directly shows its `r2`-relative loads resolving
to a *different* base (`PTR_DAT_008bfff8`-region Sulpha globals) than the one
every GUI/campaign function in this section uses (`0x008ad4d8`, where
`PTR_g_GameState` lives, confirmed via `Profile_SetDifficultyRC`'s own
callers below) - the decompiler re-derives each function's real TOC
correctly. `get_xrefs_to` does not: it is backed by Ghidra's
`ReferenceAnalyzer`/`SymbolicPropagator` pass, which builds the static xref
index using one context-register value for `r2`, not each function's own
`.opd`-declared base. A function whose real TOC differs from that default
gets every TOC-relative reference computed against the wrong base - which is
exactly what a `stb r9,0x0(r10)` in `FUN_00621eb0` (a debug-flag write) was
doing when it got reported as a "read" of `RB_AI_DIF`.

**Workaround used instead, this pass**: `just wad`/`oag-unpack extract` +
`cargo run -p oag-tools --example psarc_grep` to read
`/data/plugins/frontend/gui/cellmode_definition.xml` off `DATA06.PSARC`
directly - confirms `DifficultyButton` authors one unconditional literal
(`string="Change Difficulty"`, no `<Entry>` redirect nearby), so the
`RB_AI_DIF`/`RB_DIF` choice is genuinely the executable's, not an XML swap
this pass missed. The actual call site that resolves one string or the other
was **not found** this pass: neither `CellSelection_UpdateDifficultyButton_q`
(below, `0x0021db80` - the widget's own square-press/update handler, found
through a real `bl` xref to `Profile_SetDifficultyRC`, not a TOC-based one)
nor the screen's large per-mode detail dispatcher (`0x002181a8`, called from
`0x00217b20`/`FlyerSelection`-family init) references either string's TOC
slot in its own disassembly.

**A real fix exists and has precedent in this project** (`just build-allegrex`
patches Ghidra's Sleigh model for the PSP's Allegrex/VFPU ISA, a different
subsystem solving the same class of problem: stock Ghidra not modelling a
console-specific compiler/ISA convention). Here it would be a post-analysis
script that walks every `.opd` pair, sets the correct per-function `r2`
register context before the reference analyzer runs, then reruns it. Out of
scope for this pass; flagged here so the next person chasing an xref on this
binary does not spend the same hour on it that this pass did - **trust the
decompiler's own TOC resolution over `get_xrefs_to` for any `PowerPC:BE:64:A2ALT-32addr`
program**, and prefer a `bl`-reached call chain (like `Profile_SetDifficultyRC`'s
callers, found this way) or the disc's own XML/PSARC as ground truth instead.

## `Profile_GetDifficultyRC`/`Profile_SetDifficultyRC` and the difficulty-cycle handler - 2026-09-28

Searching for `"DifficultyRC"` (`0x0077a5a0`) - the same profile-record key
name Pulse's own `DifficultyRC` persisted-rung mechanism uses
(`docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s "The `DifficultyRC`
persisted rung" section) - turns up a real getter/setter pair on HD, reached
through genuine `bl` call xrefs (not the TOC-slot trap above):

- **`Profile_GetDifficultyRC`** (`0x00023ee8`). Hashes `"DifficultyRC"`
  (`FUN_0032a740`), walks a profile's own linked list of named records
  (`*(profile+0x450)`, next-pointer at `+0x50`), and returns the matching
  node's field at `+0x64` (`node[0x19]`) - or, if no record exists yet, a
  single global fallback `iRam00000000`. Confidence 75: the hash-lookup
  shape matches `Profile_SetDifficultyRC` below exactly, and the field index
  matches between getter and setter, but the fallback global's own
  initializer was not read this pass (see "not settled" below).
- **`Profile_SetDifficultyRC`** (`0x0002a4a8`). Same hash, find-or-insert
  into the same list (allocates a new `0x68`-byte record via
  `FUN_006762b8`/`Profile.cpp`-tagged allocator when none exists yet), writes
  `param_2` into the same `+0x64` field. Confidence 75, same basis.

**`CellSelection_UpdateDifficultyButton_q`** (`0x0021db80`) is one of two
real callers (the other, `0x00216870`, fires unconditionally at the end of a
much larger cell-selection routine and was not fully read this pass).
Confidence 60 - legible and internally consistent, but the exact class this
vtable slot belongs to was not pinned down (see below), hence the `_q`. Found
via `Profile_SetDifficultyRC`'s own `bl` xrefs, not the TOC-slot search that
misled the RB_AI_DIF/RB_DIF hunt above. Disassembly and decompile agree
exactly (both read the same way; no TOC-trap risk here since every load in
this function is a direct `bl` or a `GameState`-relative load matching
`Profile_SetDifficultyRC`'s own callers' TOC). Its shape, condensed:

```
if (this->accumulator += dt; !this->focused || *some_global) return base_update(...);
mode = GameState->mode;                      // GameState+0xe0
if mode not in {3,5,6,8,9,10,0xd} and not (mode == 0xe and GameState->0x90 != 0):
    return base_update(...);                 // no cycle: button inert this frame
rung = GameState->difficulty;                // GameState+0xdc
new_rung = (cycle rung 0->1->2->0 against this->selection_index, firing a
            "set_easy"/"set_medium"/"set_hard"-shaped debug string per step)
GameState->difficulty = new_rung;
if mode in {0xd, 0x15}: GameState->0xee (a short) = per-rung table lookup
if mode in {8, 0x14}:   GameState->0x20 (an int)   = per-rung table lookup
Profile_SetDifficultyRC(profile, new_rung);
return base_update(...);
```

**What this settles**: the mode set where HD/Fury's own `DifficultyButton`
square-press actually cycles the rung on `Cell Selection` is
`{Race(3), TimeTrial(5), Zone(6), Elimination(8), Head2Head(9), SpeedLap(10),
0xd, 0xe(conditional)}` - **not** `Tournament(4)`. `0xd`/`0x14`/`0x15` are HD
ordinals past Pulse's own 0-12 range (`oag_tables::race_campaign::Mode::ordinal`
doesn't cover them; this page's own earlier section already found `0xe` as
"possibly `NitroBattle`/`Detonator`, not chased further" on a *different*
struct's mode field - `0xd`/`0x14`/`0x15` here plausibly the same two
spellings' Fury-grid ordinals, not independently confirmed this pass either).
[`oag_ui::campaign::hd::hd_difficulty_button_line`] reuses this exact mode
set to decide whether it returns computed text at all, and keeps `Tournament`
on the disc's own static `"Change Difficulty"` string on that basis - see its
own doc comment for the full reasoning and confidence per mode.

**What this does not settle**: which idstring (`RB_AI_DIF` vs `RB_DIF`) the
prompt shows - this function never resolves either string, so that choice
lives in a still-unfound draw/text-refresh routine (see the TOC-xref trap
section above for what was checked and why it came up empty), and
`Profile_GetDifficultyRC`'s own fallback global (the value read on a record-
less profile, i.e. plausibly the fresh-profile default rung) was not traced
to its initializer this pass - `docs/ui/campaign-screens.md`'s RPCS3 capture
is this project's only evidence for the default, not this reading.
