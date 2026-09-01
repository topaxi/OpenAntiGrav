# Invented UI text has no translation, and the disc's own strings have no override path either

The disc's own localisation works and is already read: each language is a
`Data\Plugins\PI0NN` plugin, `<Entry ID="..." String="...">` in a per-language
`entries.xml`, loaded into `crates/game/src/language.rs`'s `StringTable` by
`boot::load_strings` (`crates/game/src/boot.rs:1637`), and resolved by a
widget's `idstring` at draw time - see
[frontend-boot.md](../docs/architecture/frontend-boot.md). Nothing in that
path is invented. What this project has added on top of it is not covered at
all: `assets/ui/menu.toml`'s roughly 60 `label` rows (`OPTIONS`, `GRAPHICS`,
`VSYNC`, `MASTER VOLUME`, ...) are literal English, `oag_race::Mode::fallback_label`
(`crates/race/src/mode.rs:147`) is a hardcoded English-only fallback for when
the disc's table has no short mode name, and `crates/game/src/loading/wording.rs`
is an entire module of loading-screen prose ("READING THE ARCHIVES",
"TRANSCODING MOVIES", "{done} / {total} {verb}") that the disc has no string
for at all, by its own doc comment's admission - "the disc's single string
cannot say 'TRANSCODING MOVIES'". `crates/game/src/main/hints.rs` supplies the
window title and stdout key-hint text the same way. None of these four
sources has any id-based indirection - they are plain `&str`/`format!`
literals, not lookups - so there is nothing today a translation file could
attach to. Separately, the disc's own table has no override path either: when
it is wrong or empty (Pure's, per
[pures-string-tables-are-not-read-and-its.md](pures-string-tables-are-not-read-and-its.md))
the only fix available today is upstream disc data, which the project cannot
ship (ADR-0006).

## Open

- `assets/ui/menu.toml` labels are literal English text; the file's own header
  (`menu.toml:20-22`) already reserves a `string_id` field for pulling the
  disc's own wording, and nothing reads it.
- `oag_race::Mode::fallback_label` (`crates/race/src/mode.rs:147`) is
  English-only, used by `menu::mode_label` (`crates/game/src/menu.rs:241`)
  whenever the disc's table lacks a short mode name.
- `crates/game/src/loading/wording.rs` is an entire module of invented
  loading-screen prose with no id, no disc string behind it, and no
  per-language variant.
- `crates/game/src/main/hints.rs` (window titles, stdout key hints) is the
  same shape: plain constants, no indirection.
- The disc's own `StringTable` (`crates/game/src/language.rs:165`) has no
  override layer - a wrong or missing entry (Pure's table currently loads
  empty) can only be corrected by shipping different disc data, not by this
  project.
- No project-owned translation file format, loader, or on-disk location
  exists anywhere in the repo - confirmed no `.po`/`.ftl`/locale JSON and no
  i18n/l10n mechanism beyond the disc's own read side.

## Next Steps

- Design one `id -> string` file, one per language, as **TOML**, not a
  transcription of the disc's XML shape - matching `menu.toml`'s own format
  rather than `entries.xml`'s (`id = "string"` pairs, or a `[strings]` table
  of them). The id space is still shared with the disc's own `idstring`s:
  `menu.toml`'s reserved `string_id`, a new id per `fallback_label` mode, and
  a new id per `loading/wording.rs` and `main/hints.rs` string all live in it.
- Extend `StringTable::get_or_id` (`crates/game/src/language.rs:226`)'s
  fallback chain by one link: project file first, then the disc's own table,
  then the id itself - so the same lookup both supplies invented strings and
  overrides a disc `idstring` when the project file names one.
- Wire `menu.toml`'s `string_id` through `resolve()`
  (`crates/game/src/menu.rs:954`), the way `menu::mode_label` already falls
  back from a disc string to `fallback_label`, then do the same for
  `fallback_label`, `loading/wording.rs` and `main/hints.rs` themselves.
- Load the project file in `boot::load_strings` off the same
  `chosen_language` (`crates/game/src/boot.rs:1622`) already used for the
  disc's table, so one language choice governs both, and decide where the
  files live - tracked in-repo alongside `menu.toml` under `assets/`, per
  language.
- Once this lands, cross-check against
  [pures-string-tables-are-not-read-and-its.md](pures-string-tables-are-not-read-and-its.md):
  Pure's own table loading empty is a bug in the read side, distinct from
  this feature, and the override layer should not paper over it silently -
  fixing `load_strings` for Pure stays the real fix.
