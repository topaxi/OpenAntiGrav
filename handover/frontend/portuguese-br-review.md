# Brazilian Portuguese: nobody has reviewed it, and a German reader artifact

`PortugueseBR` is offered on every title and all four titles that need one have
a disc-keyed file (`docs/ui/project-languages.md`): Pulse 1,560 of 1,619 ids,
Pure 797 of 888, HD 1,503 of 1,602, 2048 2,704 of 2,965. Omega ships its own.
Landed 2026-10-06; what is left is review.

## Open

- Nobody has reviewed any of it: every entry in every file is `human = false`.
  A native-speaker pass flips `human` to `true` per entry.
- Pure's 719 English-text ids are stored under `h_xxxxxxxx` hashes of the disc id
  (maintainer's ruling 2026-10-06), so a reviewer reads the Portuguese without a
  key: map a hash to its id with `oag_ui::strings::hashed_key` over the disc's table.
- Pure's `small.fnt` has no `º` and its `FX300ANG.fnt` no `ª`: `º` now draws as
  `o` in every language on every title (chosen, not measured). Pulse's faces draw
  every letter of its file (ground truth).
- `OAG_` ids: all 128 translated. French still defers 42 of them under `[untranslated]`.
- Pulse PS2 and the Pulse US pressing read the same `pulse` namespace; ids that only
  those releases carry fall back to English. Not checked id by id.
- Front-end screens drawn by `--screen` show an unwrapped one-line paragraph for
  a 2048 disc screen's text with `\n` in it (`SPComplete`), in English and in
  Portuguese alike: a renderer limit, not a translation one.

## Next Steps

1. A native speaker reviews one file at a time; Pure and HD first (the smaller
   two) and flips `human` per entry.

## Open: German reader artifact

The disc help texts that span paragraphs (Pulse PSP 25 ids, HD 4, Omega 51) read as a
key that *is* the text (it starts `\n`) with an empty value, in English and German
alike. It looks like `string_entries` splitting a multi-paragraph entry, so the
continuation becomes its own empty-valued key. Measured 2026-10-06 while finding
German gaps; not a German gap and left alone. Worth a reader check if those help
pages ever draw empty.
