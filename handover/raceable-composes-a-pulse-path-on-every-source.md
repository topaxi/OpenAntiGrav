# `raceable` composes a Pulse path on every source, and nothing has established why that works

Left over from the roster work of 2026-08-26, which fixed the roster's two
actual defects (a Pulse stand-in applied to every source, and a team dropped
because its path was built from its declared name rather than its folder) and
did **not** touch this one, because it is not currently wrong on any disc.

`oag_game::boot::roster::raceable` asks two questions of every source through
two Pulse-owned helpers:

```rust
archives.locate(&oag_pulse::race::ships::entry_name(id, oag_pulse::race::ships::HULL))
    && archives.locate(&oag_formats::handling::entry_name(id))
```

`ships::entry_name` formats `Data\Ships\{team}\{model}.vex` and
`handling::entry_name` formats `Data\Ships\{team}\handlingstats.xml`. Both are
Pulse's spellings, reached for whatever title was opened - the same shape as the
four literals ADR-0023 moved into the title package and the fifth removed on the
same day. `race::assets::ship_entry_name`, `boost_entry_name`,
`shield_entry_names` and `livery::flare` all go through the first of them too,
so this is the roster's path composition and the race's, not one function's
quirk.

**It is not broken today**, which is the whole reason it is a thread and not a
fix: Wipeout HD keeps all twelve of its declared teams through it, and Pure and
the PS2 pressing keep all ten and all twelve. HD's ship directories are
lowercase where Pulse's are capitalised (`oag_hd::names::TEAMS`), and it still
resolves - so either the lookup folds case somewhere, or HD's declared locations
carry the spelling that matches. Nobody has established which, and "it works" is
not the same finding as "it is right".

## Resolved: why HD resolves through a Pulse-spelled template

**Confidence 100 - it is case folding in the container layer, not a matching
spelling in HD's declared locations.** Both container kinds fold case and
path-separator before ever comparing a name:

- `Psarc::index_of_path` (`crates/assets/src/psarc.rs:122`) normalises both
  the query and every stored path through `normalise` (`crates/assets/src/psarc.rs:175`):
  strip a leading `/`, fold `\` to `/`, lowercase. Pinned by
  `psarc::tests::a_path_matches_however_the_caller_spells_it`.
- `Wad::hash_name` (`crates/formats/src/wad.rs:464`) does the identical fold
  - `\` to `/`, `A..=Z` to lowercase - before hashing, pinned by its own
  doctest (`hash_name(r"Data\FE\Images\hex_bg.mip")` ==
  `hash_name(r"DATA\FE\IMAGES\HEX_BG.MIP")`).

Both existed from each container's original implementation commit
(`297d13dd` for PSARC), not added to accommodate the roster - so `raceable`
resolving HD's lowercase ship folders against Pulse's capitalised
`Data\Ships\{team}\...` template is the container search working as designed
on every source, not a coincidence specific to HD's own spelling. This
answers the question the previous pass left indistinguishable from green
tests: `Archives::locate` never sees a case mismatch to begin with, because
neither container it delegates to (`Container::contains`,
`crates/assets/src/container.rs:100`) does either.

This also weakens the second Open item below: there is no case-based reason
to split `Data\Ships\{team}\...` into a per-title axis, since the fold
absorbs any spelling difference a fourth title could introduce. It does not
settle whether a *non-case* difference (a genuinely different path shape, not
just capitalisation) would still need one.

## Open

- Whether `Data\Ships\{team}\...` is a per-title axis in waiting, the way
  `FrontEnd::root` and `Title::plugin_definition` turned out to be. Three titles
  agreeing is the ADR-0022 bar for *not* splitting it yet; a fourth is what
  would settle it. Case is no longer the reason it might need to split (see
  above) - only a structural path difference would be.
- `oag_formats::handling::TEAMS` still lives in `oag-formats`, a format crate,
  while being Pulse's roster. Five test files consume it as exactly that and it
  is not wrong, but a title's data in a format crate is the ADR-0022 smell that
  the roster stand-in turned out to be a real instance of.

## Next Steps

- No open next step on the case-folding question - it is answered.
- The two Open items above are both ADR-0022-shaped design calls (move
  `handling::TEAMS` out of `oag-formats`, and whether to split the ship-path
  template into a title axis) rather than something a fourth disc measurement
  would resolve on its own. Revisit either the next time a fourth title's
  roster is added, since that is what would supply the missing data point for
  both.
