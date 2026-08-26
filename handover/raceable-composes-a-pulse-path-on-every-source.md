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

## Open

- Why HD resolves through a Pulse-spelled template at all, given its own roster
  is lowercase. Case folding in `Archives::locate`, or the declared location
  already carrying the right spelling - the two are indistinguishable from the
  green tests.
- Whether `Data\Ships\{team}\...` is a per-title axis in waiting, the way
  `FrontEnd::root` and `Title::plugin_definition` turned out to be. Three titles
  agreeing is the ADR-0022 bar for *not* splitting it yet; a fourth is what
  would settle it.
- `oag_formats::handling::TEAMS` still lives in `oag-formats`, a format crate,
  while being Pulse's roster. Five test files consume it as exactly that and it
  is not wrong, but a title's data in a format crate is the ADR-0022 smell that
  the roster stand-in turned out to be a real instance of.

## Next Steps

- Establish which of the two explanations makes HD resolve - read
  `oag_assets::Archives::locate` for case handling first, since that is a
  question about this repository rather than about a disc, and it decides
  whether the rest of this thread is a real axis or a coincidence.
