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
