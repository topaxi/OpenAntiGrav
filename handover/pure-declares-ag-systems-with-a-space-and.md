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

**Not a DLC absence, checked rather than assumed.** The obvious rival
explanation is that this team's ship simply is not on the disc - Pulse's
`Auricom`, `Harimau`, `Icaras` and `Mantis` are exactly that case, and a team
whose content sits in an unmounted pack looks identical from here. It does not
hold: with **no packs mounted**, the underscore spelling resolves both files on
both pressings, and only the declared spelling misses.

```
pure-psp-usa.chd: AG_Systems   hull=true  stats=true
pure-psp-usa.chd: AG Systems   hull=false stats=false
pure-psp-eu.chd:  AG_Systems   hull=true  stats=true
pure-psp-eu.chd:  AG Systems   hull=false stats=false
```

So the content is present and reachable, the location leaf is the spelling that
reaches it, and the id is the spelling that does not. That also settles half of
the design question below: `Team::location`'s leaf is known to be the right path
component, and what is left open is only what the *id* is for.

## What the string table says, and why Pure could not answer it

The open question this thread was written with - which spelling Pure keys the
display name under - **cannot be answered on Pure at all today, and the reason
is a second thread.** `load_strings` finds no entries for Pure's language
plugins, so its table is measurably empty and *every* lookup misses whatever it
is keyed on:

```
pure-psp-usa.chd  string table:    0 entries
pure-psp-eu.chd   string table:    0 entries
pulse-psp-usa.chd string table: 1621 entries
```

Every one of Pure's ten teams returns `None` for the id, `None` for the location
leaf and authors no `helpText` at all - which looks like an answer and is not
one. See `handover/pures-string-tables-are-not-read-and-its.md`; until that
lands, nothing here can be measured on Pure.

**Pulse answers it from the other side, and that evidence is independent of
Pure's empty table.** Pulse declares `PI_Team name="AG_Systems"` - the
underscore - and its string table maps that id to the display string
`"AG Systems"`, with the space:

```
pulse-psp-usa: id=AG_Systems -> "AG Systems"  help=MSC_TEAMDES_AGS -> "A resurgent AG Systems ..."
pulse-psp-usa: id=Goteki     -> "Goteki 45"
pulse-psp-usa: id=EGX        -> "EG-X"
```

So the string Pure puts in its `name` attribute is **exactly the display string
Pulse resolves to**, not the key Pulse resolves it from - and Pulse is
independently shown to keep display names out of ids, since `Goteki` reads
`Goteki 45` and `EGX` reads `EG-X`. The reading that fits both discs is that
Pure authors the *label* in `name` and keeps the id in `location`, where Pulse
authors the id in `name` and puts the label in the table. Confidence 80: it
turns on the one team of ten whose two spellings differ, which is the
discriminating case but is still a single team.

**That makes the fix cheaper than this thread first assumed.** The id is a
string-table key only where a string table exists, and on Pure none does - so
changing what Pure composes its paths from cannot break a lookup that already
misses. The risk the open question was guarding against is vacuous today and
becomes live only when Pure's string tables are read.

**Not the `Van_Uber` case.** That thread is about a Pure team whose `Ship.vex`
does not resolve by name at all; `Van_Uber` is not among the ten `PI_Team` nodes
Pure's definition declares, so it never reaches this filter. The ten are
`Feisar`, `Auricom`, `Qirex`, `AG Systems`, `Piranha`, `Assegai`, `Triakis`,
`Harimau`, `Zone` and `Medievil`.

## Open

- **Answered for now, re-open when Pure's strings are read:** which spelling
  Pure keys the display name under is unmeasurable while its table is empty, and
  Pulse's own table says the `name` attribute holds the label on Pure and the id
  on Pulse (confidence 80, one discriminating team). If
  `handover/pures-string-tables-are-not-read-and-its.md` lands and Pure turns
  out to key teams at all, check which spelling before trusting the label.
- Whether `Team::id` should keep the declared `name` (a label on Pure, an id on
  Pulse) or become the location leaf on every title. The path question is
  settled - the leaf is what resolves - but the id is also what `--team` accepts
  and what `settings.rs` stores, and those want a stable, typeable string rather
  than one with a space in it. The two titles disagree about what `name` *is*,
  which is an ADR-0022 axis question rather than a bug fix.
- `load_teams`' doc comment claims the filter and the loader "cannot disagree
  about how a path is spelled" because both compose from the same functions.
  True either way, but `raceable` and `race::load` have to move together.
- `raceable` reaches for `oag_pulse::race::ships::entry_name` on every source -
  a fourth Pulse constant applied to all three titles, in the same function the
  roster stand-in was just removed from. HD keeps all twelve teams through it,
  so it is not wrong today, and nothing has established why.
- Whether a Pure race actually flies a wrong grid because of this, or only shows
  a short menu. `livery::teams_for_slots` cycles a short list, so the field is
  filled either way.

## Next Steps

- Compose the ship and handling-stats paths from `Team::location` rather than
  `Team::id`, in `boot::roster::raceable` and in `race::load` together. That is
  the whole fix for the dropped team, it needs no further measurement, and it
  leaves the id question below untouched: on Pulse the leaf equals the id, so
  nothing changes there, and on Pure the leaf is what already resolves.
- Leave `Team::id` alone in the same change. What the id *is* differs between
  the titles and wants deciding on its own evidence, not as a side effect of
  fixing a path.
- `roster_declared_ground_truth::only_pure_loses_a_declared_team_and_it_loses_one`
  pins the current nine-of-ten and fails loudly when the drop stops happening -
  update its docs and its count to ten rather than deleting it.
