# Invented UI text has no translation, and the four sources that need it have no id-based indirection yet

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
literals, not lookups.

**The format, the loader and the override layer landed** (this session):
`oag_game::strings` merges one project-owned `assets/ui/strings/<language>.toml`
over the disc's own `StringTable` in `boot::load_strings`, project entries
winning over a matching disc id - see the module's own doc comment for the
format and `StringTable::merge` for the primitive it needed. That already
gives the disc's own table the override path it never had: a wrong or missing
disc idstring (Pure's, per
[pures-string-tables-are-not-read-and-its.md](pures-string-tables-are-not-read-and-its.md))
can now be corrected by a project file instead of needing different disc data
(ADR-0006) - **for something the disc already names**. It does nothing yet for
the four sources above, none of which look anything up by id, so
`assets/ui/strings/english.toml` ships empty rather than with invented
content nothing reads.

**`oag_race::Mode::fallback_label` turns out already covered, with no code
change** (this session, second pass): `menu::mode_label` reads through the
same `StringTable` `boot::load_strings` merges the project overlay into, so
naming `Mode::string_id` (`crates/race/src/mode.rs:132`) in a project file
already wins over the hardcoded fallback, even with no disc entry present -
proved in `crates/game/src/menu/tests/definition.rs`'s
`a_project_override_of_a_mode_id_wins_over_the_hardcoded_fallback`. This is
the one of the four sources that happens to run **after** boot, with a real
disc-merged `StringTable` already in hand; the other three do not, which is
the open question below.

## Open

- **`assets/ui/menu.toml`, `loading/wording.rs` and `main/hints.rs` all run
  before any disc-derived `StringTable` exists, which is a bigger blocker than
  "nothing reads the id yet".** Checked this session, not assumed:
  - `menu.toml` is parsed by `prepare::definition` (`crates/game/src/main/prepare.rs:321`),
    called from `main()` before `source::resolve` opens a disc at all
    (`crates/game/src/main.rs:326`) - its own doc comment already says so
    ("reads no disc and depends on nothing this module produces").
  - `loading/wording.rs`'s strings are what the loading screen says **while**
    the boot sequence's movies and language plugins are still being read -
    `heading`'s `"READING THE ARCHIVES"` case fires before there is anything
    to look a translation up in.
  - `main/hints.rs`'s `TITLE`/`MENU_KEYS` print before the boot sequence
    starts.

  So passing the disc's own merged `StringTable` into any of the three is not
  available even in principle, not just unwired - unlike `mode_label` above,
  which runs after boot and already has one. The fix these three need is a
  **second, disc-independent table**: `crate::strings` currently only exposes
  `overlay`, which merges *onto* an already-built disc `StringTable`; what is
  missing is a constructor that builds a project-only table straight from the
  embedded file, needing nothing but a language *name* - which is available
  this early, from `Settings::language: Option<String>`
  (`crates/game/src/settings.rs:137`), already loaded by the time any of the
  three run. Something close to `crate::strings::project_table(language:
  Option<&str>) -> StringTable`, falling back to `"English"` the way
  `boot::chosen_language` does. `menu::mode_label` would keep using the
  disc-merged table it already has; these three would use this one instead.
- `assets/ui/menu.toml` labels are literal English text; the file's own header
  (`menu.toml:20-22`) already reserves a `string_id` field, and nothing reads
  it. **Also blocked on `crates/game/src/menu.rs` itself**: it is at its
  `check-file-size` ratchet ceiling (1785/1785 lines,
  `scripts/check-file-size.py`'s `BASELINE`) as of this session, so
  `resolve()` (`crates/game/src/menu.rs:803`) cannot grow even by the few
  lines a lookup call needs until something moves out first. `Definition::parse`
  (`crates/game/src/menu.rs:662`), the function that would gain the
  `StringTable` parameter, has roughly twenty call sites across
  `main/prepare.rs`, `capture/menu_page.rs` and the `menu/tests/*.rs` files -
  mechanical to update (most only need `&StringTable::default()`), but real
  breadth, worth doing as its own commit separate from the `menu.rs` split.
- `crates/game/src/loading/wording.rs` is an entire module of invented
  loading-screen prose with no id, no disc string behind it, and no
  per-language variant.
- `crates/game/src/main/hints.rs` (window titles, stdout key hints) is the
  same shape: plain constants, no indirection - turning them into functions
  taking a `&StringTable` touches every print/window-title call site, fewer
  than `menu.toml`'s but not zero.
- Pure's own `StringTable` loading empty is still a bug in the read side, not
  fixed and not papered over by the override layer above - see the next
  section.

## Next Steps

- Add `crate::strings::project_table` (or similar): a disc-independent
  `StringTable` built from the embedded file alone, keyed on a language name
  rather than a loaded `Language`. This is the one piece all three blocked
  sources need before any of the rest of this list is reachable.
- Move something out of `crates/game/src/menu.rs` first (it has zero lines of
  ratchet headroom) - `mod raw` plus `resolve`/`condition_from`/`warning_from`
  (`crates/game/src/menu.rs:573`-`1012`, ~440 lines) is the obvious seam: it is
  already how `frame`/`rows`/`skin`/`strip` were split out, and it is exactly
  the code `string_id` wiring needs to touch. Then thread `project_table`'s
  output through `Definition::parse` -> `resolve()` and its ~twenty call
  sites, via `StringTable::get_or_id`.
- Do the same for `loading/wording.rs` and `main/hints.rs`: give each string
  an id, add it to `assets/ui/strings/english.toml` with its current English
  text as the value (this is the point where that file stops being empty),
  and route each call site through `StringTable::get_or_id` against
  `project_table` instead of the literal.
- Once callers exist, add a second language's file
  (`assets/ui/strings/french.toml`, ...) and a real translation, to prove the
  mechanism end to end rather than only against English overriding itself.
- Cross-check against
  [pures-string-tables-are-not-read-and-its.md](pures-string-tables-are-not-read-and-its.md):
  Pure's own table loading empty is a bug in the read side, distinct from this
  feature, and the override layer must not be used to paper over it silently -
  fixing `load_strings` for Pure stays the real fix.
