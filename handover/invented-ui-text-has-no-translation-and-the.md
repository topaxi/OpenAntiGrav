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

## Open

- `assets/ui/menu.toml` labels are literal English text; the file's own header
  (`menu.toml:20-22`) already reserves a `string_id` field for pulling the
  disc's own wording, and nothing reads it. **Blocked on more than wiring**:
  `crates/game/src/menu.rs` is at its `check-file-size` ratchet ceiling
  (1785/1785 lines, `scripts/check-file-size.py`'s `BASELINE`) as of this
  session, so `resolve()` (`crates/game/src/menu.rs:803`) cannot grow even by
  the few lines a lookup call needs until something else in the file moves out
  first.
- `oag_race::Mode::fallback_label` (`crates/race/src/mode.rs:147`) is
  English-only, used by `menu::mode_label` (`crates/game/src/menu.rs:241`)
  whenever the disc's table lacks a short mode name.
- `crates/game/src/loading/wording.rs` is an entire module of invented
  loading-screen prose with no id, no disc string behind it, and no
  per-language variant.
- `crates/game/src/main/hints.rs` (window titles, stdout key hints) is the
  same shape: plain constants, no indirection.
- Pure's own `StringTable` loading empty is still a bug in the read side, not
  fixed and not papered over by the override layer above - see the next
  section.

## Next Steps

- Move something out of `crates/game/src/menu.rs` first (it has zero lines of
  ratchet headroom), then wire `menu.toml`'s `string_id` through `resolve()`
  (`crates/game/src/menu.rs:803`, where `let label = entry.label.clone();`
  sits today) via `StringTable::get_or_id`, the way `menu::mode_label` already
  falls back from a disc string to `fallback_label`. `resolve()` has no
  `StringTable` in scope yet either - check what's available at menu-load time
  versus draw time before assuming it can just be threaded through.
- Do the same for `fallback_label`, `loading/wording.rs` and `main/hints.rs`:
  give each string an id, add it to `assets/ui/strings/english.toml` with its
  current English text as the value (this is the point where that file stops
  being empty), and route the call site through `StringTable::get_or_id`
  instead of the literal.
- Once callers exist, add a second language's file
  (`assets/ui/strings/french.toml`, ...) and a real translation, to prove the
  mechanism end to end rather than only against English overriding itself.
- Cross-check against
  [pures-string-tables-are-not-read-and-its.md](pures-string-tables-are-not-read-and-its.md):
  Pure's own table loading empty is a bug in the read side, distinct from this
  feature, and the override layer must not be used to paper over it silently -
  fixing `load_strings` for Pure stays the real fix.
