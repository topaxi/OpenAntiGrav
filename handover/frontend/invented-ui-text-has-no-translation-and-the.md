# Invented UI text has no translation - all four sources are wired, but nothing is translated yet

The disc's own localisation works and is already read: each language is a
`Data\Plugins\PI0NN` plugin, `<Entry ID="..." String="...">` in a per-language
`entries.xml`, loaded into `crates/game/src/language.rs`'s `StringTable` by
`boot::load_strings` (`crates/game/src/boot.rs:1637`), and resolved by a
widget's `idstring` at draw time - see
[frontend-boot.md](../../docs/architecture/frontend-boot.md). Nothing in that
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

## Landed 2026-09-07: the AI PILOTS page is fully covered, and a gate stops the gap reopening

- **`assets/ui/menu.toml`'s `pilots` page names a `string_id` on every one of
  its 9 rows and a `title_string_id` on the page itself** (`title_string_id`
  is new - `Definition::parse` resolves a page's `title` the same way
  `resolve` already did a row's `label`, through one shared `resolved`
  helper). `assets/ui/strings/english.toml` is no longer close to empty:
  `OAG_PILOT_PILOT`/`_AXIS`/`_LOW`/`_HIGH`/`_SAVE`/`_PAGE_TITLE`, a shared
  `OAG_MENU_BACK`, and `OAG_PILOT_BUILT_IN_SUFFIX` (the "(built-in)" marker
  `pilot_choice` builds into a value, not a `label`) joined the ids the
  pilot-editor session already added.
- **A hard gate now exists: `just check-strings`
  (`scripts/check-strings.py`).** It fails if a *new* `menu.toml` row or page
  names no `string_id`/`title_string_id`, or if any id used anywhere resolves
  to nothing in `english.toml`. The other 73 rows and 8 titles that predate
  this change are frozen in the script's own `BASELINE_LABELS`/
  `BASELINE_TITLES` - the same ratchet shape `check-size` already is - so old
  debt is bounded and visible without blocking, and it is wired into `just
  check`.
- **The maintainer's own call, since it bears on the second bullet below**:
  a language is allowed to lag a translation, but only by naming the gap
  under its own `[untranslated]` table, never by omitting the id (a
  machine translation may fill a row since 2026-10-06, marked `human = false`) - see `english.toml`'s own doc comment for the worked
  example. No second language file exists yet to prove this against, so the
  rule is written ahead of a caller rather than proven by one.

## Landed 2026-09-27: the HUD and the OPTIONS page's LANGUAGE row both follow the chosen language live, not this thread's own mechanism but the same shape of bug

Two player-reported bugs, adjacent to everything above but about the *disc's*
own table rather than the project's: `race::hud::load_hud`'s one caller
(`race/load.rs`) passed `None` - "the chain's own default" - instead of the
player's saved `settings.language`, so the in-race HUD showed whichever
language a title's plugin order happened to list first, invisibly correct
only for whoever tested with that one. On the EU pressing (see this thread's
own "no English plugin" gotcha above) that default is French, so a German
pick showed a French HUD. Separately, the OPTIONS page's own LANGUAGE row
wrote `settings.language` and stopped - the string table, the menus' own
faces and every disc-labelled row stayed whatever they were at boot, so a
switch a player just made showed nowhere until the next launch.

Both fixed: `race::Options` grows a `language` field, threaded from
`settings::Settings::language` at every launch site and refreshed again in
`Session::launch_race` so it cannot go stale against a live LANGUAGE-row
change; `Session::resupply_language` (`crates/game/src/main/session/language.rs`)
re-runs `boot::load_shell` with the new language and folds the result through
`Shell::from_boot` - the same function a fresh boot uses - re-supplying the
open menu's language-scoped rows and the GPU-side fonts without losing cursor
position or restarting. `boot::fonts::role_font` and `race::hud::hud_font`
had the same "first plugin, no preference" bug one level down for font roles;
both now ask the chosen language first. Full writeup, including what a
headless run could and could not verify, in
`docs/architecture/menus.md#switching-language-without-relaunching` and
`docs/architecture/menus.md#the-hud-following-the-front-ends-language`.

**Incidental finding, not fixed, not this lane's to fix**: the HD/Fury
pressing's own sixteen-language table (`hdfury-ps3-eu-dec.iso`) reports
Japanese, Korean and TraditionalChinese all under the *native name*
`"Svenska"` (Swedish), and Russian's native name as `"P??????"` - literal `?` bytes in the
disc's file, not a decoding fault (read from its own table since 2026-10-09). Neither breaks the mechanism this thread or the 2026-09-27 entry
above are about (the *English* name, which both key off, is unaffected), but
the LANGUAGE row's own picker would show a Swedish label for three languages
that are not Swedish. Measured with `--race --dry-run --no-audio` against
`hdfury-ps3-eu-dec.iso`; not chased further here.

## Open

- **Stale claim from an earlier pass corrected 2026-09-09: the `string_id`
  conversion is done, not "73 of 82 rows still open".** Verified against
  `main` directly: `just check-strings` reports `OK: 83 menu row(s) (0
  baselined), 9 page title(s) (0 baselined), 109 string id(s) all resolve in
  english.toml`, and `scripts/check-strings.py`'s own `BASELINE_LABELS`/
  `BASELINE_TITLES` are both empty sets. Every page converted across the
  sessions that did this work; nothing is left to convert on that axis.
- **`english.toml` still carries its own copy of every `OAG_HINTS_*`/
  `OAG_LOADING_*` literal, alongside `hints.rs`/`wording.rs`'s own fallback -
  and the table wins.** Fixing a typo in either file's literal changes
  nothing a player sees until `english.toml`'s copy is fixed too, which is
  exactly backwards from what a developer reading only the `.rs` file would
  expect. `check-strings.py`'s own byte-for-byte extractor is the check that
  catches the two drifting apart (`STRING_CONSUMERS`'s doc comment says "kept
  in step by hand"); nothing catches a *correct* edit to one side landing
  without its pair. Still unfixed - out of scope for the language work below,
  which only ever touches string *content*, not this duplication.
- **The EU Pulse disc ships no English language plugin at all.**
  `pulse-psp-eu.chd`'s own boot report names exactly four: `French (PI008),
  German (PI009), Spanish (PI010), Italian (PI011)` - no `PI012`. So
  `settings.language = "English"` against the EU disc silently falls through
  `boot::chosen_language`'s own fallback chain to whichever plugin loaded
  first (French, on this disc), not to English. This is real disc content,
  not a bug in the fallback - but it means **an English capture must name
  `pulse-psp-usa.chd`, never the EU disc `just play` defaults to**, or the
  screenshot silently comes back in the wrong language with no error
  anywhere. Cost about twenty minutes to a session proving French landed
  before the mismatch was noticed; naming it here so the next screenshot
  session does not repeat it.

## Landed 2026-09-09: `assets/ui/strings/french.toml`, the first real second-language file

- **86 of the 109 ids that exist have real French text; the other 23 are
  named under `french.toml`'s own `[untranslated]`** - the PC-graphics/
  display jargon block, Wipeout's own `SIDESHIFT`, every prose string
  carrying a `%s`/button-glyph substitution, the three long key-hint
  sentences, and the two suffixes concatenated onto two of those sentences.
  See `french.toml`'s own header for the id-by-id reasoning. This is the
  first real exercise of the `[untranslated]` convention `english.toml`'s
  doc comment wrote ahead of a caller - nothing was machine-translated to
  clear it, and every gap is named rather than silently missing.
- **`strings::built_in` now embeds it** (`language.eq_ignore_ascii_case
  ("French")`), which is the actual gap that made a `french.toml` on disk
  not the same thing as a player seeing French - `overlay`/`project_table`
  already merged whatever `built_in` handed them, but `built_in` recognised
  only `"English"` before this change. A new test,
  `every_file_under_assets_ui_strings_is_reachable_through_built_in` (in
  `crates/game/src/strings.rs`), fails the same way this gap did if a future
  language file lands with no matching `built_in` branch.
- **Proved end to end with headless screenshots**, `pulse-psp-usa.chd`
  (the EU disc has no English to contrast against - see Open above), two
  pages, at `1440x816` (this build's real default `--screenshot` size):
  `--menu-page options` and `--menu-page audio`. OPTIONS reads AFFICHAGE /
  GRAPHISMES / AUDIO / COMMANDES / PILOTES IA / LANGUE / RETOUR in French
  against DISPLAY / GRAPHICS / AUDIO / CONTROLS / AI PILOTS / LANGUAGE /
  BACK in English; AUDIO reads VOLUME DE LA MUSIQUE / VOLUME DES EFFETS /
  VOLUME DU COMMENTATEUR / VOLUME GÉNÉRAL / SOURCE MUSICALE against MUSIC
  VOLUME / SFX VOLUME / ANNOUNCER VOLUME / MASTER VOLUME / MUSIC SOURCE.
  `VOLUME GÉNÉRAL`'s `É` renders as the real accented capital, not a folded
  blank or a wrong glyph - `crates/game/src/font.rs`'s fold chain and this
  file's own accented text both confirmed by the same screenshot. The boot
  report's own two lines (`language French from settings, skipping the
  picker` and `assets/ui/strings/french.toml: 86 override(s)`) were read
  back too, not just the picture - see `docs/architecture/frontend-boot.md`
  for what each line means.
- Settings reached without touching a developer's own `~/.config/oag/`:
  `XDG_CONFIG_HOME=<scratch dir> oag-game ... --menu-page ... --screenshot
  ...` with a `settings.toml` naming `language` under that scratch
  directory - `dirs = "6"` (`settings::path`) honours `$XDG_CONFIG_HOME` on
  Linux, and `oag-game` has no `--settings`/`--language` CLI flag of its
  own, only the saved setting and `--pick-language` (which shows the picker
  rather than naming a language outright).

## Next Steps

- Convert another language the same way: pick the ids `french.toml` marked
  `[untranslated]` that a fluent speaker of a *different* language can
  translate confidently (the PC-jargon block is probably the same
  cross-language sticking point, but `SIDESHIFT` and the prose lines are
  worth a second, independent judgement call rather than assuming French's
  choice generalises).
- Revisit `french.toml`'s own `[untranslated]` list once a French speaker
  can check it against a real shipped French Wipeout release (PS2/PSP Pulse
  itself, or Pure) - in particular whether `SIDESHIFT` has a canonical
  in-series French term, which would let it graduate out of the list rather
  than staying a guess deferred forever.
- The `english.toml`/`hints.rs`/`wording.rs` literal-duplication friction
  named in Open above is a mechanism-level fix (probably: `hints.rs`/
  `wording.rs` stop carrying their own literal and read `english.toml`
  through `strings::built_in` for their English fallback too, so there is
  one copy instead of two) - out of scope for a translation-content change,
  named here so it does not get lost.
- **Settled 2026-09-08, cited for whoever still has this open in a stale
  checkout: `load_strings` reading empty for Pure is fixed.** `boot.rs`'s
  `load_strings` now re-parses a language plugin's own `Definition.xml` as
  the string table when it names no external `entries.xml` - which is every
  one of Pure's five picker languages, English included, not only the one
  this used to single out. `--dry-run --no-video` against
  `pure-psp-usa.chd` reports `Data\Plugins\PI009\Definition.xml: 873 strings
  for German`, the exact language the old `German names no string table`
  failure named. See
  `docs/formats/pure-status.md#the-language-plugin-id-space-is-pures-own-not-pulses`.
  The distinction this bullet drew still holds going forward: the override
  layer here must not be used to paper over a *read*-side bug in some future
  title, only to add or replace text this project authors itself.

## From the HANDOVER.md index (moved 2026-09-25)

the mechanism landed: `oag_game::strings` merges a project-owned `assets/ui/strings/<language>.toml` over a `StringTable`, project entries winning, either onto the disc's own (`boot::load_strings`) or, for a caller with no disc open yet, a project-only one (`strings::project_table`). Every `menu.toml` row/page now names a `string_id`/`title_string_id` (`just check-strings` reports 0 baselined either way), and `assets/ui/strings/french.toml` is the first second-language file: 86 of 109 ids translated, 23 honestly named under its own `[untranslated]` rather than guessed at, proved on screen with headless `--menu-page options`/`--menu-page audio` screenshots against `pulse-psp-usa.chd` (the EU disc ships no English plugin at all, a real disc-content gotcha named in the thread). Still open: `english.toml` duplicates the `hints.rs`/`wording.rs` literals by hand rather than being their one source of truth, and only one of the four other languages the disc offers (German, Spanish, Italian, ...) has a project translation yet
