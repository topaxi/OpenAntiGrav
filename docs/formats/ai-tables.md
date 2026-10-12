# AI tuning tables: `AIControlStats.xml` and `AIRaceStats_<class>.xml`

**Status: decoded and refused-by-name over every file the census found.**
Readers: [`oag_tables::ai_control_stats`](../../crates/tables/src/ai_control_stats.rs)
and [`oag_tables::ai_race_stats`](../../crates/tables/src/ai_race_stats.rs)
(`parse_class_race`, the whole file; `parse`, the thrust view the finished
player's law reads). The parser that reads them in the original, the record
layout and what each field governs are
[ai-stats.md](../ghidra/functions/psp-pulse-usa/ai-stats.md) (confidence 88)
and [ai-controls.md](../ghidra/functions/psp-pulse-usa/ai-controls.md); the
design is [gameplay/ai.md](../gameplay/ai.md). This page is the file side only.
No tuning value is reproduced here (ADR-0006): the readers parse them off the
disc and the tests compare titles against each other.

## What the files hold

Both are the front end's shortened XML on PSP and plain text on PS2, PS3, Vita
and PS4, one `<<Class>Stats>` element a speed class (`Venom`, `Flash`, `Rapier`,
`Phantom`; 2048 and Omega add `SuperPhantom`, a copy of Phantom's).

| File | Under each class | Read into |
| --- | --- | --- |
| `AIControlStats.xml` | one `<Controller>`: `SteerMul`, `SteerDamp`, `LookAheadSecs`, `xtrackMul`, `xtrackDamp` | `ai_control_stats::Controller` |
| `AIRaceStats_<class>.xml` | one `<RaceBalancing>`: `PosBalancing` (`PlayerInPos1..8`: `AIThrust`, `SpreadDist`), `RubberBanding` (`WhenLeading`, `WhenBehind`: `SpeedMul`, `MinSpeedChangeDist`, `MaxSpeedChange`), `StartStats` (`GridPlace1..8`: `StartBoost`, `BaseStartThrust`), `SkillScale` (`SkillScalePoint1..3`: `ThrustOffset`, `ThrustMultiplier`, `SpreadMultiplier`; `AIPackSwapping1..3`: `numAIPositionSwaps`, `numIntermediatePositions`, `intermediateOffsetSize`), and HD-era files add `SplitScreenMultiplier` (`Easy`, `Medium`, `Hard`) | `ai_race_stats::ClassRace` |

Notes that bite:

- **`xtrackMax` is never authored** in any shipped controller file. The reader
  returns `None` for any attribute a file leaves out, so the executable's
  pre-filled defaults (`AiStats_SetDefaults`) stay the consumer's to apply.
- **`BaseStartThrust` is authored and the original never reads it**: its parser
  looks for `BaseThrust` (ai-stats.md). The reader carries it as authored, named
  for the file, and nothing may treat it as live.
- `numAIPositionSwaps` and `numIntermediatePositions` are stored as floats and
  cast to integers by the original (ai-controls.md); the reader keeps `f32`.
- **Refuses by name**, not by guess: an element under `<RaceBalancing>` the
  reader does not know (`UnknownElement`), an attribute it does not know on a
  row it reads (`UnknownAttribute`), a missing row or a non-numeric value. The
  original's parser skips unknown tags silently; this reader does not, so a
  new field in a future file is a red test and not a dropped number.

## Census

Counted over the discs and archives below, **by listing a PSARC's own paths** for
HD, 2048 and Omega, and **by looking up the known names in the hashed WADs** on
PSP and PS2 (a WAD stores a name hash only, so a file whose name the project has
not mined cannot be counted there). Test: `ai_control_stats_ground_truth.rs` and
`ai_race_full_ground_truth.rs`, one test per source.

| Source | Controller files | Race files | Outcome |
| --- | --- | --- | --- |
| Pulse PSP USA, `Data.wad` | 1 | 4 | all 5 decode |
| Pulse PSP EU, `Data.wad` | 1 | 4 | all 5 decode, equal to USA's |
| Pulse PS2 EU, `WADS2.WAD` | 1 | 4 | all 5 decode (plain text), equal to PSP's |
| Pure PSP USA and EU, `Data.wad` | 1 | 1 | **both refused** by these readers (the controller file adds `<ControllerPO>`, the race file has no `<RaceBalancing>`; the text equals HD's `_kart` pair); read by `oag_tables::pure_ai_stats` instead (2026-10-12), below |
| HD disc `DATA02` (8 files) | 2 | 6 | `aicontrolstats` and the four per-class files decode, equal to Pulse's. Refused by name: `aicontrolstats_kart` (`ControllerPO`), `airacestats` (`ShipSpacingVariation`), `airacestats_kart` (no `RaceBalancing`) |
| HD disc `DATA03` | 0 | 4 | decode: Pulse's four plus `SplitScreenMultiplier` |
| HD disc `DATA00`, `01`, `04`, `05`, `06` | 0 | 0 | none |
| HD PSN `data02`, `data03` | as the disc | as the disc | identical outcomes; `data01`, `data04` none |
| 2048 EU (`PCSF00007`) and USA (`PCSA00015`) base `data.psarc` (15 files each) | 3 | 12 | `aicontrolstats` and `aicontrolstats2048` decode (5 classes, Phantom's copy for SuperPhantom); 10 race files decode (Pulse's plus `SplitScreenMultiplier`; the `..._<class>2048` set equals the plain set); refused: `aicontrolstats_kart`, `airacestats`, `airacestats_kart` |
| 2048 `dlc1`, `dlc2`, both `patch-v104` archives, both regions | 0 | 0 | none |
| Omega `data00` (15 files) | 3 | 12 | the same set as 2048, same outcomes |
| Omega `data01`-`04` and the day-one patch's `data05`, `07`, `08`, `09` | 0 | 0 | none |

The shipped values are **Pulse's own** across HD, 2048 and Omega
(`ai_stats_lineage_ground_truth.rs` compares attribute by attribute; the new
tests compare the typed records), with `SplitScreenMultiplier` and 2048's
`SuperPhantomStats` as the only additions.

## Cross-title verdicts

| Title | Verdict |
| --- | --- |
| Pulse (PSP USA/EU, PS2) | **ported**: both readers, ground truth on all three. Read whole for the campaign's original opponent (`oag_ai::original`, 2026-10-12), which `oag_title::RaceDefaults::original_opponent` turns on for Pulse only; `original_ai_ground_truth.rs` holds it to the live capture. |
| HD / Fury | **checked, applies, not wired**: the readers parse the disc and PSN files to Pulse's own records (plus `SplitScreenMultiplier`, which HD reads with more than one local player). HD's thrust law differs from Pulse's in four places (the lane's thread); no HD consumer exists. `oag_ai::original::Field` would run on HD's `ClassRace` unchanged, but HD's planner, steering and rail are unread, so `RaceDefaults::original_opponent` is `false` and a ground truth keeps HD's campaign on this project's driver. |
| 2048 | **ported**: same, plus `SuperPhantomStats`; 2048's campaign flies the original driver on all five classes (`SpeedClass::SUPER_PHANTOM`, `AIRaceStats_superphantom.xml`; `vita_2048_original_ai_ground_truth.rs`). |
| Omega | **checked, applies, not wired**: `data00` carries 2048's set unchanged; no Omega driver reads it, and `original_opponent` is `false` (2048's verdict carries over). |
| Pure | **checked, differs; read by its own reader** (2026-10-12): Pure's two files are the `_kart` family, which these readers refuse by name. `oag_tables::pure_ai_stats` reads them the way Pure's executable does: the legacy block of `AIRaceStats.xml` (`StartStats`, `ZoningStats`, `FatigueStats`, `GridSpread`; `<NewStats>` and `MasterSpeedTweak` skipped by name, the executable never selects them), `<Controller>` (`<ControllerPO>` skipped, never parsed), and a track `stats.xml`'s `<<Class>Tweak Min Max>`. `pure_original_ai_ground_truth.rs` reads all five classes on the USA disc. The HD, 2048 and Omega `_kart` files carry the same text but no title's executable was found reading them; the reader was not run on them. See [psp-pure-usa/ai-controls.md](../ghidra/functions/psp-pure-usa/ai-controls.md). |

The combined `airacestats.xml` (HD `DATA02`, 2048, Omega: `ShipSpacingVariation`
over all classes) and the `_kart` pair are **unread by every executable
checked** (HD's EBOOT carries none of their element names,
[ps3-hdfury-eu/ai-stats.md](../ghidra/functions/ps3-hdfury-eu/ai-stats.md)), so
refusing them loses nothing a title consumes.
