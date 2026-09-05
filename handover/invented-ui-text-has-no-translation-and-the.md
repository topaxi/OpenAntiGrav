# Invented UI text has no translation - all four sources are wired, but nothing is translated yet

The disc's own localisation works and is already read: each language is a
`Data\Plugins\PI0NN` plugin, `<Entry ID="..." String="...">` in a per-language
`entries.xml`, loaded into `crates/game/src/language.rs`'s `StringTable` by
`boot::load_strings` (`crates/game/src/boot.rs:1637`), and resolved by a
widget's `idstring` at draw time - see
[frontend-boot.md](../docs/architecture/frontend-boot.md). Nothing in that
path is invented. What this project has added on top of it started out with
none of that: `assets/ui/menu.toml`'s roughly 60 `label` rows (`OPTIONS`,
`GRAPHICS`, `VSYNC`, `MASTER VOLUME`, ...) were literal English,
`oag_race::Mode::fallback_label` (`crates/race/src/mode.rs:147`) is a
hardcoded English-only fallback for when the disc's table has no short mode
name, `crates/game/src/loading/wording.rs` is an entire module of
loading-screen prose ("READING THE ARCHIVES", "TRANSCODING MOVIES",
"{done} / {total} {verb}") that the disc has no string for at all, by its own
doc comment's admission, and `crates/game/src/main/hints.rs` supplies the
window title and stdout key-hint text the same way.

**All four are wired now, one of them (`Mode::fallback_label`) turning out to
be already covered by the mechanism** - landed across two sessions, in this
order:

1. **The mechanism**: `oag_game::strings` (`crates/game/src/strings.rs`)
   merges one project-owned `assets/ui/strings/<language>.toml` over a
   `StringTable`, project entries winning over a matching id -
   `StringTable::merge` is the one primitive it needed. `boot::load_strings`
   wires it onto the disc's own table; `strings::project_table` builds a
   disc-independent one straight from the embedded file, for a caller with no
   disc open yet.
2. **`oag_race::Mode::fallback_label` needed no change at all**:
   `menu::mode_label` already reads through the disc-merged table, so naming
   `Mode::string_id` in a project file already overrides the hardcoded
   fallback - proved in `menu/tests/definition.rs`'s
   `a_project_override_of_a_mode_id_wins_over_the_hardcoded_fallback`.
3. **`assets/ui/menu.toml`'s `string_id` is read**: `Definition::parse` takes
   a `&StringTable` now and `resolve()` looks a row's `string_id` up in it,
   falling back to the literal `label` - proved end to end in
   `a_string_id_is_resolved_against_the_table_parse_is_given`.
   `prepare::definition` (the `--menu`/built-in path, which runs before any
   disc opens) passes `strings::project_table(settings.language.as_deref())`;
   `capture::menu_page` (which draws a page of an already-open title) passes
   the real disc-merged table it already had. **This build's own `menu.toml`
   still names no `string_id`** - the mechanism exists, the content does not,
   which is the difference between "wired" and "translated".
4. **`loading/wording.rs` is wired**: `heading`, `step_line` and `counts`
   each take a `&StringTable` now and look up an `OAG_LOADING_*` id per
   string, falling back to the current literal exactly like `resolve()`.
   Dynamic strings (the transcode frame counter, the done/total/verb line,
   the cached/failed suffixes) go through a small `resolved()` helper that
   substitutes `{name}` placeholders after the lookup rather than using
   `format!`, so a translation can reorder them freely. The table is built
   once, in `Screen::new` (`strings::project_table`, threaded through a new
   `language: Option<&str>` parameter from `Settings::language` at every
   real caller), not per `draw_list` call - proved in
   `loading/wording.rs`'s own `#[cfg(test)] mod tests`
   (`an_id_with_an_override_resolves_to_it`,
   `an_id_with_no_override_falls_back_to_the_current_literal`,
   `one_table_answers_every_call_it_is_passed_to`, and the placeholder tests).
5. **`main/hints.rs` is wired too**: its eight `pub(crate) const &str` became
   functions taking a `&StringTable`, each looked up by an `OAG_HINTS_*` id
   and falling back to the old literal. None of these eight has a disc
   idstring to override - they are window titles and stdout key hints, not
   anything the front end draws - so every call site builds its own
   `strings::project_table(language)`, except `prepare::windowed`'s
   `MENU_KEYS` line, which already had `boot_shell.strings` (the real
   disc-merged table) in hand and reuses it instead. `Gpu::new` grew a
   `language: Option<&str>` parameter for `TITLE`, the one hint set before
   any of the other machinery exists. Proved by `hints.rs`'s own tests -
   an override resolves, a miss falls back, and `every_hint_has_its_own_id`
   checks that overriding one of the eight ids never changes another's
   answer.

## Open

- `assets/ui/menu.toml` names no `string_id` on any row, and
  `assets/ui/strings/english.toml` ships empty as a result - the mechanism
  has nothing to demonstrate yet beyond its own unit tests. Adding either
  is straightforward now that both exist; nobody has decided which rows are
  worth an id first.
- No second language file exists yet (`assets/ui/strings/french.toml`, ...),
  so the override layer has only ever been proved against English overriding
  English, or an empty table falling through to the id. A real translation is
  the only way to prove the *language* half of this, as opposed to the
  *override* half.
- Pure's own `StringTable` loading empty is still a bug in the read side, not
  fixed and not papered over by the override layer above - see the next
  section.

## Next Steps

- Now that all four sources have real ids, give `assets/ui/menu.toml` a
  `string_id` on a row or two and add matching entries to
  `assets/ui/strings/english.toml` - this is the point where that file
  stops being empty, and where "the mechanism works" becomes "a row is
  actually overridden".
- Add a second language's file (`assets/ui/strings/french.toml`, ...) with a
  real translation of whatever ids exist by then, to prove the *language*
  half rather than only the *override* half.
- Cross-check against
  [pures-string-tables-are-not-read-and-its.md](pures-string-tables-are-not-read-and-its.md):
  Pure's own table loading empty is a bug in the read side, distinct from this
  feature, and the override layer must not be used to paper over it silently -
  fixing `load_strings` for Pure stays the real fix.
