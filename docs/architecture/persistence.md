# Persistence: what survives a restart, and where it lives

This is a map of the three files this project writes to a player's own
machine, not inside the repository or the disc image. All three live under
`<config dir>/oag/` - `dirs::config_dir()`'s answer, which is
`$XDG_CONFIG_HOME/oag` (or `~/.config/oag`) on Linux, `~/Library/Application
Support/oag` on macOS, and `%APPDATA%\oag` on Windows.

| File | Owner module | What it holds |
| --- | --- | --- |
| `settings.toml` | `crates/game/src/settings.rs` | Display, graphics, audio, controls, and the last-picked race options - one row per key, rewritten canonically every run. |
| `pilots/*.toml` | `crates/game/src/pilots.rs` | Player-authored AI opponents - one file per pilot, hand-editable, never rewritten wholesale. |
| `records.toml` | `crates/game/src/records.rs` | Best lap, best total time and the last result, per circuit/mode/class. This page. |

Read `settings.rs`'s and `pilots.rs`'s own module docs first if you have not -
`records.rs` follows the first's file shape and diverges from both on error
handling, and this page assumes both are already familiar rather than
re-explaining them.

## What is in `records.toml`

One `[[records]]` row per `(title, track, mode, class)` combination a player
has actually raced. A title with no circuits played yet simply has no rows -
see `oag_game::records`'s own module doc, "no circuit list is hardcoded here".

```toml
[[records]]
title = "wipeout pulse"
track = "data\\environments\\16_track\\track.vex"
mode = "single_race"
class = "venom"
best_lap_ticks = 1987
best_total_ticks = 6042
last_finished = true
last_place = 2
last_laps_completed = 3
last_tick = 6042
last_best_lap_ticks = 1987
```

`title`, `track`, `mode` and `class` together are the key - see
[`oag_game::records::Key`](../../crates/game/src/records.rs) for exactly how
each part is spelled and lower-cased. `best_lap_ticks` and
`best_total_ticks` are the two numbers worth keeping forever: the quickest
completed lap and the quickest *finished* race ever recorded on that row,
each only ever improving. The five `last_*` fields are the most recent race
on that row, whatever it says - better or worse than the standing best,
because "last" is a different question than "best".

Every value is ticks (`u32`/`u64`), the same unit `oag_game::hud::format_lap_time`
already takes, never a formatted string - formatting is a presentation
concern, and this file is not one.

## Where a race's result is captured, and why there are two sites

**Never from inside `Race::tick`.** This project's core architectural rule -
the simulation must not know a renderer, or by extension a save system,
exists - applies here exactly as it does to the scoreboard the results table
is already built from. Nothing in `oag-race`, `oag-gameplay`, `oag-ai` or
`crates/game/src/race/` calls into `records.rs`, imports it, or knows it
exists.

Two places in the composition root do, both reading `Race`'s already-public
state after the fact:

1. **`Session::frame`**, in the match arm that already exists for "the race
   has finished" (`crates/game/src/main/session/frame.rs`). Guarded by a
   `result_saved` flag on `RaceStage` so the once-per-tick arm saves exactly
   once, on the tick the transition happens, not on every frame the results
   table is on screen afterwards.
2. **`Session::escape`** (`crates/game/src/main/session/menus.rs`), on
   leaving a `Stage::Race` - whether that is a player backing out to the
   menus, or an unfinished `--race` run quitting outright. Guarded by the
   same flag, so a race already saved on finishing does not get saved again
   for nothing new to say.

**`result_saved` means "since this stage last became live", not "ever".** An
*unfinished* race `escape` leaves is parked rather than discarded
(`Session::suspended_race`), keeping the same `RaceStage` - flag included -
across the trip through the menus, and `Session::resume_race` swaps it
straight back into `Stage::Race`. Left set, a Speed Lap lap improved after
resuming would never be recorded, and a Time Trial escaped mid-race and then
resumed to a real finish would have that finish silently swallowed by the
finish arm's own guard. `resume_race` clears the flag back to `false` on the
way out of a park for exactly this reason - found by reasoning through the
resume path during review, not by a test, since neither capture site's own
unit tests touch `Session` at all.

The second site exists because `oag_race::Mode::laps_target` is `None` for
Speed Lap and Zone, so `Race::finished` never turns `true` for either mode -
and Speed Lap's entire purpose is a fast lap time. Leaving a race is the one
exit every mode has in common; finishing is not. See ADR-0049 for the full
reasoning, including what this still cannot catch (a hard quit or a crash
between the last `escape`/finish and the moment it happens - `records.rs`'s
own module doc names this as a known, accepted gap rather than a silent one).

Both sites build the same `oag_game::records::Observation` off the same
helper, `RaceStage::observation` (`crates/game/src/main/race_stage.rs`), so
the field list only has to agree with `oag_game::records` in one place.

## The key: resolved once, at load, never re-derived

`RaceStage::result_key` is built once, in `Stage::build_race_stage`
(`crates/game/src/main/stage.rs`), from `race::Loaded::title` and the
`race::Options` the load was actually started from - not re-read later,
because neither is reachable from a finished or an escaped race stage any
other way. This is also why `Stage::build_race_stage` and `Stage::race` each
gained one more parameter, `track_entry: Option<&str>`: `race::Loaded` itself
carries no `.vex` entry name, only the geometry it resolved to, and the key
wants the disc's own path a menu-launched or `--race`-launched race was
actually requested with.

## What a bad file does, and does not, do

Unlike `settings.toml`, which fails the boot loudly on one bad value on
purpose - see that module's own doc for why a settings file is small enough
to want that - `records.toml` never fails a boot and never panics. A file
this build cannot fully make sense of degrades, in order:

1. **One malformed row** is dropped and logged; every other row is kept.
2. **The whole document not parsing as TOML at all** moves the file aside to
   `records.toml.invalid` next to itself, rather than overwriting it in
   place, and starts empty.
3. **No config directory on this platform** is silent - the same case
   `settings.rs` and `pilots.rs` both already treat as "nothing to persist
   to".

See `oag_game::records`'s own module doc, "deliberately not `Settings`'s own
rule on a bad file", for the reasoning, and its test module
(`crates/game/src/records/tests.rs`) for what each of the three cases above
actually asserts.

## Where a career system attaches

`Record` carries a best lap, a best total time and a last result, and
nothing about a medal, an unlock or a tournament standing - those are a
separate finding, not a guess made here (see the `campaign` handover thread
for what the disc actually authors, if it is still open; check `HANDOVER.md`
first). Two ways to grow this file when that lands, and both are additive so
neither needs `Key`, `parse` or `Store::record` to change:

- A new `#[serde(default)]` **field** on `Record`, for something that is
  still one number per circuit/mode/class - a medal tier, say.
- A new **sibling table** in the same file, alongside `[[records]]`, for
  something that spans more than one circuit - a tournament standing.

## What is not built yet

There is no on-screen row showing a stored best lap or a personal-best
comparison during or after a race - today's surface is the persisted file
itself, plus whatever a future session prints or draws from it. Wiring a
"personal best" line into the results table, or a records browser of its own,
is left for whoever picks this up next; the storage and the two capture sites
are the part of this that does not change shape underneath it.
