# Race records persist across a restart; nothing on screen shows one yet

2026-09-08. Best lap, best total time and the last result now survive a
restart: `crates/game/src/records.rs` (`<config dir>/oag/records.toml`),
captured at two sites outside `Race::tick` -
`Session::frame`'s finish-transition arm and `Session::escape` - and merged
best-of into a `Store` keyed on `(title, track, mode, class)`. Full design
and reasoning: [ADR-0049](../../docs/architecture/adr/0049-race-records-are-a-chosen-schema-captured-outside-the-tick.md)
and [`docs/architecture/persistence.md`](../../docs/architecture/persistence.md).
Both capture sites and the schema itself have their own unit tests
(`crates/game/src/records/tests.rs`, 18 tests) and the full workspace suite
(3,330 tests) and the disc-backed regression gate
(`race_ground_truth::a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round`,
all twelve clean) were both green with this change in.

## Open

**`Session::escape`'s own capture site has not been runtime-verified** - only
`Session::frame`'s finish transition has, live, twice, with a real process
restart in between. This sandbox has no way to deliver an Escape keypress to
the game's window (a real Wayland surface; `xdotool` is X11-only and finds
nothing, and no `wtype`/`ydotool`/`wlrctl` is installed), and there is no
`SIGTERM` handler anywhere in `crates/game/src/main`, so a `kill` bypasses
`escape()` rather than exercising it. The code is compile-checked and
reviewed against the same live-confirmed `RaceStage::observation()` the
finish site uses, but Speed Lap and Zone - the two modes that depend on this
site entirely, since `Race::finished` never turns `true` for either - have
not themselves produced a saved row on a real desktop yet. Run this on one:

```sh
oag-game --race --mode speed_lap --autopilot
# let it complete a lap or two, then press Escape
cat "${XDG_CONFIG_HOME:-$HOME/.config}/oag/records.toml"
# expect a row: mode = "speed_lap", last_finished = false, a real best_lap_ticks
```

A `result_saved` bug was found and fixed the same day, by reasoning through
`Session::escape`/`Session::resume_race` rather than by a test: an
*unfinished* race `escape` parks (`Session::suspended_race`) keeps its
`RaceStage`, guard flag included, across a trip through the menus, so
without `resume_race` clearing the flag on the way back out, a Speed Lap
improved after resuming would never be recorded and a Time Trial escaped
mid-race and then resumed to a real finish would have that finish silently
dropped. Worth re-checking on a real desktop too: escape mid-race, resume,
then either improve a Speed Lap or finish a Time Trial, and confirm the
result lands.

**Nothing draws a stored record.** A player can see their times only by
reading `records.toml` by hand - there is no "personal best" line on the
results table, no records browser, nothing in `assets/ui/menu.toml`. This
was scoped out deliberately rather than missed: the foundation (a schema that
survives a restart, captured in the right two places) was this pass's actual
ask, and a UI surface is a separable, smaller piece of work on top of it.

**Where the campaign structure attaches, when `campaign`'s thread lands.**
`Record` carries nothing about a medal, an unlock or a tournament standing on
purpose - see the ADR's "growth" section. Two additive attach points already
designed in: a new `#[serde(default)]` field on `Record` for a per-row value
(a medal tier), or a new sibling table beside `[[records]]` for something
that spans more than one row (a tournament standing). Neither needs `Key`,
`parse` or `Store::record` to change - check `campaign`'s own findings before
inventing either shape from scratch.

## Next Steps

1. Wire a "personal best" line into `crate::scoreboard`'s results table -
   `Session::frame`'s finish arm already knows the key and now the
   `records::Store`, so the plumbing exists; only the draw call is missing.
   Needs a `string_id`/`english.toml` entry only if it becomes a `menu.toml`
   row rather than free text drawn the way the rest of the scoreboard is
   (`check-strings.py` covers `menu.toml` rows and page titles, not
   `scoreboard.rs`'s own `const &str`s - see that file for the precedent).
2. A records browser (circuit list plus best times) is a bigger, separate
   piece - probably its own `menu.toml` page, which *would* need string ids.
   Not started.
3. If `campaign` has landed by the time this is picked up, read its findings
   first rather than guessing at the medal/unlock shape - the schema is
   designed to grow but was not designed *for* a specific shape, on purpose.
