# Wipeout HD/Fury's campaign grids now parse, and `Grid Selection`/`Cell Selection` draw

2026-09-14, later the same day. Both screens now draw off the real disc -
see `docs/ui/campaign-screens.md`'s "Wipeout HD/Fury" section for the full
picture (what's measured, what's chosen, two shared-parser fixes, capture
paths). This section is the historical record of the parsing-only pass;
the screen-drawing pass's own detail lives in the docs page, not repeated
here.

2026-09-14. The discriminating question was: does `oag_tables::race_campaign::from_blob`
parse HD's `plugins/grids/grid_*.xml` unchanged, or does the schema need a
variant? **It needs extending, additively.** Done: `oag_tables::race_campaign`
now reads all 32 grid files across HD's four archives (`DATA00`, `DATA02`,
`DATA04`, `DATA06`), Pulse's own ground truth is unchanged, and
`crates/hd/src/campaign.rs` / `crates/hd/tests/campaign_grids_ground_truth.rs`
/ `docs/formats/race-campaign.md`'s new HD section carry the schema diff, the
counts, and what is and is not measured. Full detail is in that doc section -
this thread is the open work, not a restatement of what landed.

## What landed

- `race_campaign::Cell` gained `difficulty_targets: Option<DifficultyTargets>`
  (three `MedalTargets` triples: easy/medium/hard) and
  `nitro_elimination_targets: Option<(i64, i64, i64)>` (the raw
  `NitroElimNovice`/`Skilled`/`Elite` triple). `gold`/`silver`/`bronze` keep
  meaning the medium rung when the per-difficulty shape is authored, so no
  existing caller (`evaluate_medal`, `crates/game/src/records.rs`) changed.
- `race_campaign::Mode` gained `Other(String)` for `"NitroBattle"` and
  `"Detonator"`, HD's two mode spellings with no Pulse ordinal.
  `Mode::ordinal()`/`as_str()` changed signature (`&self`, `Option<u32>`) -
  contained inside `oag-tables`, nothing outside it calls either.
- `race_campaign::Grid` gained `campaign`/`title_color`/`text_color`/
  `flyer_name`/`billboard_name`, all raw `Option<String>`.
- `skill` reads `skillMedium` as a fallback attribute name.
- New: `crates/hd/src/campaign.rs` (entry names, the four-archive table),
  `crates/hd/tests/campaign_grids_ground_truth.rs` (four `#[ignore]`d tests,
  all green against `hdfury-ps3-eu-dec.iso`).
- One real, on-disc finding: `grid_04.xml`'s own `<Values>` tag is missing a
  closing `>` in all three of its copies, so this project's parser correctly
  reads zero cells for a grid that authors five. Not fixed - see the doc
  section for why.

## Open

- **Which target set a mode's own medal law reads is partly measured now,
  2026-09-21.** `EBOOT-ps3-hdfury-eu.elf`'s own `PI_Cell` attribute table
  (`0x008ae898`) lists `NitroElimNovice`/`Skilled`/`Elite` as three of its own
  fields, immediately after `EasyGold`..`EasyBronze` - confirming this is a
  real `PI_Cell` attribute, not a coincidental string reuse from an unrelated
  subsystem, and a second string (`0x00779a98`) measures
  `Novice`/`Skilled`/`Elite` as this title's own words for
  `Easy`/`Medium`/`Hard`. `Cell::nitro_elimination_target_for_difficulty` now
  reads the measured number for a rung. **Still open**: the actual medal
  comparison consumer was not found - `PI_Cell`'s binder is
  reflection-driven, so nothing in the image references these attribute
  names at their point of use, and reaching the consumer needs the
  attribute table's own per-entry encoding decoded first. Also open: whether
  the triple represents three medal tiers at all, or one pass/fail target
  per rung (the disc's own vocabulary - `NitroElim*` spelled in rung words,
  not tier words - argues for the latter, but this is not independently
  confirmed). See `docs/ghidra/functions/ps3-hdfury-eu/race-campaign.md` for
  the full evidence and `docs/formats/race-campaign.md`'s HD section for the
  data-format summary.
- **Whether HD's own front end tolerates `grid_04.xml`'s broken `<Values>`
  tag** - i.e. whether the real game shows five cells there or none - is the
  same kind of question, also not chased.
- **Only the EU Fury PS3 pressing was measured** (`hdfury-ps3-eu-dec.iso`). A
  base-HD-only disc (pre-Fury, no `DATA00` grid_08..15) was not available to
  this pass.

## Next Steps

Done, this same day: both screens draw, pointer-driven, off the real disc -
see `docs/ui/campaign-screens.md`'s "Wipeout HD/Fury" section. What is
still open moved there too (the archive-precedence/RPCS3 question, the
stray hex artifact, the undrawn flyer model, the launch path's live
verification) rather than being duplicated in both places - this thread's
own `## Open` above is the parsing-level items only, unchanged by the
drawing pass.
