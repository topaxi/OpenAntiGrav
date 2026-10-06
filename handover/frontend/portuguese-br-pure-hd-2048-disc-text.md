# Brazilian Portuguese: Pure, HD and 2048 disc text are still English

The project language `PortugueseBR` is offered on every title and Pulse's disc
text is translated (1,560 of 1,619 ids, `human = false`); see
`docs/ui/project-languages.md`. Pure, HD and 2048 resolve their disc ids through
the disc's own English, with only our `OAG_` ids in Portuguese. Omega needs no
lane: it ships `portuguesebr` itself.

## Open

- **Pure**: 911 English entries. One `assets/ui/strings/disc/pure/portuguesebr.toml`,
  then `disc_strings: Some("pure")` in `crates/pure/src/lib.rs` and a `built_in_disc`
  arm in `crates/ui/src/strings.rs`. Pure's HUD ids (`HUD_Lap`) and Pulse's differ,
  which is why the namespace is per title.
- **HD / Fury**: about 1,602 entries, and HD's circuits are keyed `NN_TRACK` through
  `CircuitNames`, which picks its own copy of the table - check a translated circuit
  name still resolves there.
- **2048**: 2,965 entries. 2048 ships a European `Portuguese` plugin; the project
  language stands on 2048's English instead. The `2048` namespace
  (`disc_strings: Some("2048")`) already exists, taken by the German gap file
  (`assets/ui/strings/disc/2048/german.toml`); a project language is overlaid there,
  the disc's own German only filled.
- Match terms to Omega's `portuguesebr` table (it is on disc, already loaded by
  `load_languages`); write sentences ourselves. Never commit the English beside them.
- Nobody has reviewed any of it: every entry is `human = false`.
- Pulse PS2 and the Pulse US pressing read the same `pulse` namespace; ids that only
  those releases carry fall back to English. Not checked id by id.
- `OAG_` ids: all 128 translated. French still defers 42 of them under `[untranslated]`.

## Next Steps

1. Pure first (smallest, 911): dump its English table to `data/scratch/`, translate in
   chunks, write the TOML, add the namespace and a ground-truth line like
   `portuguese_br_ground_truth.rs`'s Pulse one.
2. Then HD, then 2048.
3. A native speaker pass flips `human` to `true` per entry.

## Open: German reader artifact

The disc help texts that span paragraphs (Pulse PSP 25 ids, HD 4, Omega 51) read as a
key that *is* the text (it starts `\n`) with an empty value, in English and German
alike. It looks like `string_entries` splitting a multi-paragraph entry, so the
continuation becomes its own empty-valued key. Measured 2026-10-06 while finding
German gaps; not a German gap and left alone. Worth a reader check if those help
pages ever draw empty.
