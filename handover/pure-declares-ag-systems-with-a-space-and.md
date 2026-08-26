# Pure declares `AG Systems` with a space, and the team is dropped from its roster

2026-08-26, found while proving `boot::load_teams`' Pulse stand-in was dead
code (see `crates/game/tests/roster_declared_ground_truth.rs`). **Both Pure
pressings declare ten teams and this build offers nine.** The one lost is
`AG Systems`, and it is lost to a spelling:

```
pure-psp-usa.chd: 10 declared
  id=AG Systems     loc=Data\Ships\AG_Systems    hull=false stats=false
  id=Feisar         loc=Data\Ships\Feisar        hull=true  stats=true
  ... eight more, all hull=true stats=true
pure-psp-eu.chd:   identical
```

`Data\Plugins\PI001\Definition.xml` authors `<PI_Team name="AG Systems">` over
`<Values location="Data\Ships\AG_Systems">`. `oag_game::catalogue::read_team`
takes the `name` attribute as the id and keeps the `location` beside it;
`boot::roster::raceable` then composes **both** entry names it checks from the
id alone - `oag_pulse::race::ships::entry_name(id, HULL)` and
`oag_formats::handling::entry_name(id)` - so the space goes into the path, both
lookups miss, and the filter drops the team. Confidence 95: the id and the
location are read straight off the disc, the two pressings agree, and every
other team on both discs has a name identical to its directory, which is why
only this one falls out.

**Pulse never exposed it.** `crates/game/src/catalogue/tests.rs`'s
`a_team_id_is_the_leaf_of_its_directory` states the assumption outright and
`dlc_ground_truth` re-checks it against real content - but only Pulse's, where
it holds for all twelve. The PS2 pressing and Wipeout HD hold too (twelve each,
none dropped).

**The fix is not just "use `location`".** `catalogue::Team::id` doubles as the
string-table key the display name is looked up under, as what `--team` accepts,
and as what `settings.rs` stores. Deriving the id from the location leaf changes
all four at once, and which spelling Pure keys its own display name under -
`AG Systems` or `AG_Systems` - has not been read off the disc. That question is
step one, not the code change.

**Not the `Van_Uber` case.** That thread is about a Pure team whose `Ship.vex`
does not resolve by name at all; `Van_Uber` is not among the ten `PI_Team` nodes
Pure's definition declares, so it never reaches this filter. The ten are
`Feisar`, `Auricom`, `Qirex`, `AG Systems`, `Piranha`, `Assegai`, `Triakis`,
`Harimau`, `Zone` and `Medievil`.

## Open

- Which spelling Pure's string table keys the `AG Systems` display name under.
  Until that is read, deriving the id from the location leaf may fix the path
  and break the label.
- Whether the id and the path should be separated instead - `raceable` and
  `race::load` composing from `Team::location`, the id staying the declared
  name. `load_teams`' own doc comment claims the filter and the loader "cannot
  disagree about how a path is spelled" because both compose from the same
  functions; that stays true either way, but both would have to move together.
- `raceable` reaches for `oag_pulse::race::ships::entry_name` on every source -
  a fourth Pulse constant applied to all three titles, in the same function the
  roster stand-in was just removed from. HD keeps all twelve teams through it,
  so it is not wrong today, and nothing has established why.
- Whether a Pure race actually flies a wrong grid because of this, or only shows
  a short menu. `livery::teams_for_slots` cycles a short list, so the field is
  filled either way.

## Next Steps

- Read Pure's string table for both spellings of the `AG Systems` key
  (`oag_game::boot::load_strings` resolves the table; the ids are what
  `strings.get_or_id` is called with in `main/prepare.rs`).
- Then decide id-from-location versus path-from-location, and change `raceable`
  and `race::load` together.
- `roster_declared_ground_truth::only_pure_loses_a_declared_team_and_it_loses_one`
  pins the current nine-of-ten and fails loudly when the drop stops happening -
  update its docs and its count to ten rather than deleting it.
