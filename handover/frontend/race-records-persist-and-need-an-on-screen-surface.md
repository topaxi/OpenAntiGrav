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
  carries one - which is still never, in a real session, because nothing
  selects a campaign cell yet (see "Still open" below).

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

**No wiring from a real race to a campaign cell**, so `BEST MEDAL` never
draws in a real session (only in the read-only `--screenshot` demo above,
off a hand-seeded file). `RaceStage::observation` still passes
`campaign_medal: None` unconditionally - see
`docs/architecture/persistence.md`'s "where a career system attaches"
section for exactly what closing this needs (a `Cell Selection` launch-path
trace, a track/mode name-mapping, a per-mode value to evaluate).

**The in-race HUD caption tier is unrelated and still blocked on RE** - see
`handover/frontend/the-huds-four-remaining-items.md`. Nothing in this change
touches `crates/game/src/hud/**` or feeds a campaign medal into
`Hud_UpdateTimeCluster_q`'s tier; the two are different encodings of
different things, by design - see `records.rs`'s own `Medal` doc comment for
why it round-trips as a word rather than the HUD's own ordinal.

## Next Steps

1. A records browser (circuit list plus best times) is a bigger, separate
   piece - probably its own `menu.toml` page, which *would* need string ids
   (unlike the results table's own free text). Not started.
2. If `campaign`'s cell-selection thread lands, feed a real
   `campaign_medal` into `RaceStage::observation` and the `BEST MEDAL` line
   starts drawing in a real session with no further change to
   `records.rs`/`scoreboard.rs` - both already carry the value end to end.
3. Runtime-verify `Session::escape`'s capture site on a real desktop, per
   "Still open" above - this sandbox cannot deliver the keypress.
