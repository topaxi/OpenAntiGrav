# ADR-0049: race records are a chosen schema, captured outside the tick, at two sites

## Status

Accepted.

## Context

Before this change, a race in this project ended, drew a scoreboard, and the
result evaporated on the next `escape` - nothing about a circuit, a mode or a
best lap survived a restart. This is the foundation half of closing that gap:
best lap, best total time and the last result, per circuit/mode/class,
surviving a process restart. A sibling thread (`campaign`) is separately
finding out how much of the original's campaign structure the disc itself
authors - medals, unlocks, tournament standings. This ADR is deliberately
narrower than that: it is the part that is true regardless of what that thread
finds, and it has to leave room for it without guessing at it.

Three questions had no existing answer to reuse wholesale:

1. **What file shape.** `crates/game/src/settings.rs` and
   `crates/raceplay/src/pilots.rs` already establish two different persistence
   philosophies in this codebase - a machine-owned file that is rewritten
   canonically every run and errors loudly on a bad value (`settings.toml`),
   and a hand-authored file that is never rewritten wholesale and is edited
   surgically through `toml_edit` (`pilots/*.toml`). Records are machine-owned
   like the first, but losing them to one bad row is a worse failure than
   losing a setting, because nothing else recreates the history.
2. **Where the capture happens.** `CLAUDE.md`'s core principle - the
   simulation must not know a renderer, or by extension a save system, exists
   - rules out writing from inside `oag_race`/`oag_gameplay`. The precedent is
   `crate::scoreboard`, already built from a captured snapshot rather than a
   live tick. But a race's finish condition
   ([`oag_race::Mode::laps_target`]) is `None` for Speed Lap and Zone, so
   [`crate::race::Race::finished`] never turns `true` for either - and Speed
   Lap's entire purpose is a fast lap.
3. **What the key is.** The disc authors circuit names, mode names and speed
   classes, and this project already reads them for its menus - inventing a
   parallel enumeration here would violate "never invent what the assets
   already author" for no reason, since the actual race that just ran already
   names all three.

## Decision

### The schema is chosen, not measured, and carries no confidence score

The original stores race results in its own PSP save-data format, on its own
trigger (`"Race End Save"`, a `.rodata` state name recovered but whose screen
is not built - see `crate::scoreboard`'s module doc). This project has no
obligation to reproduce that layout, because nothing here loads an original
save file. `crates/game/src/records.rs` is entirely this project's own
design.

`Key { title, track, mode, class }`, all four lower-cased, and a `Record` per
key carrying `best_lap_ticks`, `best_total_ticks`, and five `last_*` fields for
the most recent race under that key. `track` is
[`crate::catalogue::Track::entry_name`] - the disc's own `.vex` path, not a
name this module invented - which is also what keeps a forward and a reversed
run of one circuit as two separate rows: they are different lines to drive,
and merging them would be wrong, not merely imprecise.

### Two capture sites, both outside the tick, neither inside a gameplay crate

`Session::frame`'s already-existing "the race has finished" match arm gains one
more thing it does on the tick that arm is first reached: read
[`crate::race::Race::finished`], `::places`, and `.world`'s already-public
fields into an `Observation`, and merge it into the session's `Store`. This is
the same shape the arm already has for `Audio::race_tick` - reading a finished
race's state from the composition root, once, without stepping it.

`Session::escape` gains the same capture, unconditionally on entering it from
a `Stage::Race` - guarded by a `result_saved` flag on `RaceStage` so a race
already saved on finishing is not saved a second time for nothing new to say.
This is the site that makes Speed Lap and Zone reachable at all: leaving a
race is the one exit every mode has in common, where finishing is not.

Both sites read only fields `race/results.rs`, `race/field.rs` and
`gameplay/src/world.rs` already made `pub` for the scoreboard and the HUD.
Nothing was added to `oag-race`, `oag-gameplay` or `crates/raceplay/src/` to
make this possible, and nothing needed to be.

### Deliberately not `settings.rs`'s rule on a malformed file

`settings::load` treats a bad value as an error that reaches the player - "a
typo should be visible, not swallowed" - and the game does not boot until it is
fixed. That is right for a small, hand-checkable file. A records file is the
only copy of however many races' worth of times a player has driven, and
refusing to boot over one bad row would hold an unrelated race hostage to a
write this module made itself. So `records::parse` decodes the `records` array
one row at a time, keeps every row that decodes, and only names the ones that
do not; `records::load` moves a document that will not parse as TOML at all
aside to `records.toml.invalid` rather than overwriting it, and never returns
an `Err` a caller could feel obliged to propagate into a boot failure -
`Store::default()` plus a logged line is the worst case, always.

### Growth: additive fields or a sibling table, never rebuilt

`Record` carries a best lap, a best total time and a last result and nothing
about a medal, an unlock or a tournament standing - those are `campaign`'s own
finding, not a guess made here. Two ways to add them when they land, both
additive: a new `#[serde(default)]` field on `Record` for something that is
still one number per circuit/mode/class, or a new sibling table beside
`[[records]]` for something that spans more than one circuit. Neither needs
`Key`, `parse` or `Store::record` to change - the same promise `settings.rs`'s
own `#[serde(default)]`-everywhere shape already makes for its tables.

### `result_saved` means "since this stage last became live", not "ever"

`Session::escape` can park an *unfinished* race in `Session::suspended_race`
rather than discard it - a player backing out mid-race expects to get back to
it, and `Session::resume_race` swaps the same `RaceStage` straight back into
`Stage::Race`. The guard flag rides along with it, so `resume_race` clears it
back to `false` on the way out of a park. Without that: a Speed Lap lap
improved after resuming would never be recorded (`escape`'s own guard would
see the flag already set), and a Time Trial escaped mid-race and then resumed
to a real finish would have that finish silently swallowed by the finish
arm's own guard instead of ever reaching `Store::record`. Caught by reasoning
through the resume path explicitly rather than by a test that happened to
drive it - the two capture sites' unit tests do not touch `Session` at all,
so this is architectural review territory, not something `cargo nextest`
could have flagged.

## Alternatives considered

**A nested `BTreeMap<title, BTreeMap<track, BTreeMap<mode, BTreeMap<class,
Record>>>>` instead of `Vec<Record>` with the four key fields inline.**
Rejected: four levels of dynamic TOML table keys, one of them a backslashed
archive path needing escaping, reads worse than a flat array of tables and
buys nothing - `Store::record`'s linear scan over what is realistically a few
dozen rows costs nothing measurable, and every row already carries its own
key fields for `parse`'s tolerant per-row decode to work from.

**Canonical rewrite (`toml::to_string_pretty`) on every save, rather than a
`pilots.rs`-style surgical `toml_edit` upsert that touches only the changed
row.** Chosen deliberately over the surgical route despite one real cost: a
future field this build does not know about, present in a file written by a
newer version, is silently dropped on the next rewrite by an older binary -
the same trade-off `settings.toml` already accepts, and accepted here for the
same reason: `records.toml` is machine-owned and rewritten every run, unlike
a pilot file a player hand-edits and expects untouched. The surgical route
would preserve an unknown future field through an old binary, at the cost of
`records.rs` growing `pilots.rs`'s save-path complexity for a machine-owned
file that never needed it.

**Capturing only on the finish transition, not also on `escape`.** The
simpler design, matching the scoreboard's own precedent exactly - and
rejected because it silently drops Speed Lap and Zone entirely, which
`Mode::laps_target` being `None` for both makes structural rather than
incidental. See the Decision section's second point.

**Keying `Key::track` on [`crate::catalogue::Track::id`] plus a reversed
flag, rather than the `.vex` entry name directly.** Equivalent information,
and the id is arguably the more legible field to hand-inspect in the file -
but the entry name is what a load actually resolves to and is already the
value `race::Options::track` and `title.race.track` both carry, so using it
directly needed no extra lookup at the one call site that builds a `Key`.

## Consequences

- A player's best lap, best total time and last result now survive a process
  restart, for every mode including the two (`SpeedLap`, `Zone`) that never
  reach a `finished()` transition at all.
- A hard quit (window close, `kill -9`) between the last `escape`/finish and
  the crash still loses whatever changed since - there is no autosave-on-every-
  personal-best, on the same reasoning this project has no other autosave
  story either. Documented as a known gap in `records.rs`'s own module doc
  rather than silently accepted.
- `crates/game/src/records.rs` depends on nothing beyond `anyhow`, `log`,
  `serde` and `toml` - no `oag-race`, `oag-gameplay` or `oag-title` import at
  all - which is what keeps `Store::record` and `laps_completed` unit-testable
  with no disc, no GPU and no simulation step.
- `race_ground_truth::a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round`
  and the rest of the disc-backed suite are unaffected: nothing added here
  runs inside `Race::tick`, `Race::start` or any gameplay crate, so the
  simulation's own behaviour and its committed hashes do not move.
