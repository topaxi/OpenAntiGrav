# Race records persist across a restart, and now have an on-screen surface

2026-09-08: best lap, best total time and the last result now survive a
restart: `crates/game/src/records.rs` (`<config dir>/oag/records.toml`),
captured at two sites outside `Race::tick` -
`Session::frame`'s finish-transition arm and `Session::escape` - and merged
best-of into a `Store` keyed on `(title, track, mode, class)`. Full design
and reasoning: [ADR-0049](../../docs/architecture/adr/0049-race-records-are-a-chosen-schema-captured-outside-the-tick.md)
and [`docs/architecture/persistence.md`](../../docs/architecture/persistence.md).

2026-09-09: `Cell::evaluate_medal`/`records::Medal`/`Observation::campaign_medal`
landed (`0c938742`), still unfed by any real race - see
`docs/ghidra/functions/psp-pulse-usa/race-campaign.md` and
`docs/architecture/persistence.md`'s "where a career system attaches" section.

2026-09-09 (this change): the results table now draws what was stored. See
"What is now on screen" below and `docs/architecture/persistence.md`'s new
"The results table" section for the full account. `handover/frontend/the-huds-four-remaining-items.md`'s
in-race HUD caption tier is a **different, still-blocked** thing - see that
file, and do not conflate the two.

## What is now on screen

`crate::scoreboard::draw_list` draws up to two extra lines under the results
table's `RACE TIME` footer:

- `PERSONAL BEST LAP <time>`, highlighted and suffixed `- NEW!` when this
  race is the one that set it.
- `BEST MEDAL <tier>`, same highlighting rule, shown only when the row
  carries one. **Fed by a real race since 2026-09-14** for a race launched
  through `Cell Selection` - see
  `handover/frontend/the-campaign-grid-draws-and-does-not-launch.md` - and
  still never for one launched any other way (the ordinary RACE page, RACE
  REMIX, `--race`), since none of those set `RaceStage::campaign_cell` at
  all.

Both come from `oag_game::records::PersonalBest::compare`, a pure function
of the row `Store::get` returned *before* this race's `Observation` was
folded in (`Store::record` mutates in place, so "before" has to be read
first) and that same `Observation`. `Session::frame`'s finish arm is the
only writer of `RaceStage::personal_best`; `Session::escape`'s capture site
never has a results table to draw one on, so it does not need one.

A `--race --screenshot` capture (`crates/game/src/race/capture.rs`,
`crate::capture::run`) draws the identical line, **reading** (never
writing) whatever `records.toml` already holds under the same key
`Stage::build_race_stage` would resolve - `race::CaptureOptions::previous_best`.
Verified live, three ways, off a real disc
(`data/images/pulse-psp-eu.chd`, `--race --mode time_trial --autopilot
--ticks 8000 --screenshot`):

1. Fresh `XDG_CONFIG_HOME`, no `records.toml`: `PERSONAL BEST LAP 0.41.28 -
   NEW!`, no medal line.
2. A seeded row with a deliberately slow `best_lap_ticks` (100000): this
   race's real 0.41.28 beats it, same `- NEW!` line as above.
3. A seeded row with a deliberately fast `best_lap_ticks` (1) and
   `best_medal = "silver"`: `PERSONAL BEST LAP 0.00.01` (no `NEW!`, plain
   colour - this race did not beat it) and `BEST MEDAL SILVER` (no `NEW!` -
   `campaign_medal` is `None` on this path too, so the medal line only ever
   reports the standing one, never credits this race with it).

All three screenshots were read back with the Read tool, not just asserted
by a test - see the lane report this thread's own commit points at for
where they were written.

Test coverage: `crates/game/src/records/tests.rs` gained
`PersonalBest::compare`'s own six cases (mirroring `Store::record`'s
existing best-of tests one for one), and `crates/game/src/scoreboard/tests.rs`
gained six more for the drawing itself (no personal best draws nothing,
plain vs. highlighted lap, plain vs. highlighted medal, both together still
fit the 480x272 grid).

## Still open

**`Session::escape`'s own capture site has still not been runtime-verified**
- only `Session::frame`'s finish transition has, live. This sandbox has no
way to deliver an Escape keypress to the game's window (a real Wayland
surface; `xdotool` is X11-only and finds nothing, and no
`wtype`/`ydotool`/`wlrctl` is installed), and there is no `SIGTERM` handler
anywhere in `crates/game/src/main`, so a `kill` bypasses `escape()` rather
than exercising it. Speed Lap and Zone - the two modes that depend on this
site entirely, since `Race::finished` never turns `true` for either - have
not themselves produced a saved row on a real desktop yet. Run this on one:

```sh
oag-game --race --mode speed_lap --autopilot
# let it complete a lap or two, then press Escape
cat "${XDG_CONFIG_HOME:-$HOME/.config}/oag/records.toml"
# expect a row: mode = "speed_lap", last_finished = false, a real best_lap_ticks
```

**No records browser.** A player still sees a stored time in full only
right after running the circuit that set it, or by reading the TOML by
hand - there is nothing in `assets/ui/menu.toml` letting them browse every
circuit's stored bests from the menus. Scoped out on purpose: see "Next
Steps" below.

**Closed, 2026-09-14** - `handover/frontend/the-campaign-grid-draws-and-does-not-launch.md`
wired `RaceStage::campaign_medal` into `RaceStage::observation`, so
`BEST MEDAL` now draws in a real session launched through `Cell Selection`.
Still `None` for every other launch path (the ordinary RACE page, RACE
REMIX, `--race`), which is correct: none of those select a campaign cell at
all, so there is no medal to credit.

**The in-race HUD caption tier is unrelated and still blocked on RE** - see
`handover/frontend/the-huds-four-remaining-items.md`. Nothing in this change
touches `crates/game/src/hud/**` or feeds a campaign medal into
`Hud_UpdateTimeCluster_q`'s tier; the two are different encodings of
different things, by design - see `records.rs`'s own `Medal` doc comment for
why it round-trips as a word rather than the HUD's own ordinal.

2026-09-09 (this change): a RECORDS page landed - `assets/ui/menu.toml`'s
`records` page (MODE, TRACK, BACK), reachable from the main menu, drawing a
live per-class table below its rows. **It follows the disc's own shape, not
an invented one**: `Data\Plugins\PI001\GUI\RecordGrid_Definition.xml`'s
"Speed Lap Records" screen is a track picker over a table with one row per
speed class and a time column, and this project's own persisted schema
(`oag_game::records::Record`, one row per `(track, mode, class)`) can back
exactly that shape and no richer one - see
`docs/architecture/menus.md`'s new RECORDS section and
`docs/formats/fe-menu-definitions.md`'s new section on the file, confidence
92. TAG, TEAM and the boost/perfect-lap columns the disc's screen also
carries are left off entirely, since nothing in this project's schema backs
them - drawing invented values there would be exactly the
"plausible-looking stand-in" `CLAUDE.md`'s own root doc forbids.
`crates/game/src/main/records_page.rs` was the module; six unit tests over
its pure half (`held_text`/`show_total`/`build_table`) in
`crates/game/src/main/records_page/tests.rs`. **The page itself was
captured and read back off `pulse-psp-eu.chd`** via `--menu-page records
--screenshot`: title "RACE RECORDS", MODE/TRACK/BACK all render correctly,
nothing overlapping the footer. ~~**The live per-class table was not seen
on screen** - `--menu-page` draws one page statically with no `Menu::update`
and calls nothing in `records_page`, and `--screenshot`'s ordinary sequence
capture (`crates/game/src/capture.rs`) always hands `Launch Game` off to a
race rather than opening the real menus at all, so there is no headless
route to it.~~ ~~**`--menu-page records` is not wired to show the table** -
only `MODE`/`TRACK` - since `crates/game/src/capture/menu_page.rs` builds
its own draw list independently of the live session's `MenuStage::render`
and nothing there calls `records_page::table_for`; wiring that in is a
small, separate change for whoever next touches that file.~~ **Done,
2026-09-23** - see "The per-class table now draws in `--menu-page records`
too" below. The pure resolution moved to `oag_game::scoreboard::records_table`
so both the live session and `crates/game/src/capture/menu_page.rs` share
it; `crates/game/src/main/records_page.rs` is now a thin wrapper supplying
`Shell::tracks_for` as the live session's own track lookup, and its own
unit tests moved with the code they test into `crates/game/src/scoreboard/
tests.rs`.

## The per-class table now draws in `--menu-page records` too

2026-09-23: `crates/game/src/capture/menu_page.rs`'s `"records"` page draw
now calls `oag_game::scoreboard::records_table` - the same function
`crate::records_page::table_for` calls for the live session - supplying a
track lookup over the capture's own boot-survey `tracks` list rather than
`Shell::tracks_for(mode)`. The one real difference: a capture's list carries
no distinct Zone tracks, so a capture with `race.mode` seeded to `zone`
resolves no track and draws no table, rather than one built against the
wrong list - an honest gap, not an invented one, per `CLAUDE.md`'s
do-not-invent rule. The store itself is read the same "read-only, off
whatever `<config dir>/oag/records.toml` already holds" way
`race::CaptureOptions::previous_best` already reads it for a race capture -
loaded once in `capture.rs`, right before the `menu_page` call, so the
function itself stays testable against a hand-built `Store`.

**Runtime-verified off `pulse-psp-eu.chd`**, `--menu-page records
--screenshot --no-audio`, read back with the Read tool: title "RACE
RECORDS", MODE/CIRCUIT/BACK (this disc's own French label set was current:
"RETOUR") all correct, and four class rows (VENOM/FLASH/RAPIER/PHANTOM)
drawn below them, nothing overlapping the footer bar. The machine's own real
`records.toml` and `settings.toml` were used unmodified - not seeded - so
every class row read `-`: the settings file's own `race.track` (`01_Track`,
Basilico Black) has never been raced under `single_race`, its current
`race.mode`. That is the honest result for this machine's own data; the
same machine's `records.toml` does carry real times for other track/mode
pairs (`03_Track`/`single_race`, for one), not captured here to avoid
editing the real `settings.toml` just to pick a different row for a
screenshot. Screenshot saved at
`~/.cache/oag/drive/reports/records-capture/records-page.png`.

New tests: `scoreboard::tests::class_table_*` (three, moved verbatim from
`records_page/tests.rs`), `scoreboard::tests::show_total_is_false_only_for_speed_lap_and_zone`
(moved), and `scoreboard::tests::records_table_tests::*` (three, new -
page-id guard, an unresolvable track, and a built table with one real row
and one dash).

## Next Steps

1. ~~A records browser (circuit list plus best times)~~ - **done, this
   change.**
2. **Runtime-verify the live per-class table on a real desktop** - the one
   thing this change could not confirm itself, for the same reason
   `Session::escape` below could not: no headless route reaches it. `oag-game
   data/images/pulse-psp-eu.chd --press start,cross` (real window, no
   `--screenshot`) reaches Main Menu; MAIN -> RECORDS -> read the table. The
   `--menu-page records --screenshot` still (see above) is the closest this
   sandbox can get, and it now shows the real table, not just MODE/TRACK.
3. ~~Wire `crates/game/src/capture/menu_page.rs`'s own `"records"` page draw
   so `--menu-page records --screenshot` shows the per-class table too, not
   only the MODE/TRACK rows - mirrors the `"pilots"` arm already there for
   `pilots::axis_preview_for`.~~ **Done, 2026-09-23** - see "The per-class
   table now draws in `--menu-page records` too" above.
4. If this project's persisted schema ever grows a pilot tag, a team, or a
   per-lap breakdown (see `docs/architecture/persistence.md`'s "where a
   career system attaches"), `RecordGrid_Definition.xml`'s Time Trial/Race/
   Zone sub-screens are the shape to draw *those* rows in - not authored
   yet because the data to fill them is not either.
5. ~~If `campaign`'s cell-selection thread lands, feed a real
   `campaign_medal` into `RaceStage::observation` and the `BEST MEDAL` line
   starts drawing in a real session with no further change to
   `records.rs`/`scoreboard.rs` - both already carry the value end to end.~~
   **Done, 2026-09-14** - see
   `handover/frontend/the-campaign-grid-draws-and-does-not-launch.md`;
   `BEST MEDAL` now draws for a race launched through `Cell Selection`.
6. Runtime-verify `Session::escape`'s capture site on a real desktop, per
   "Still open" above - this sandbox cannot deliver the keypress.
