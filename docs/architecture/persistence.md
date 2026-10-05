# Persistence: what survives a restart, and where it lives

This is a map of the files this project writes to a player's own
machine, not inside the repository or the disc image. All of them live under
`<config dir>/oag/` - `dirs::config_dir()`'s answer, which is
`$XDG_CONFIG_HOME/oag` (or `~/.config/oag`) on Linux, `~/Library/Application
Support/oag` on macOS, and `%APPDATA%\oag` on Windows.

| File | Owner module | What it holds |
| --- | --- | --- |
| `settings.toml` | `crates/game/src/settings.rs` | Display, graphics, audio, controls, and the last-picked race options - one row per key, rewritten canonically every run. |
| `pilots/*.toml` | `crates/raceplay/src/pilots.rs` | Player-authored AI opponents - one file per pilot, hand-editable, never rewritten wholesale. |
| `records.toml` | `crates/game/src/records.rs` | Best lap, best total time and the last result, per circuit/mode/class. This page. |
| `ghosts/<title>/<mode>/<class>/<track>.oagr` | `crates/game/src/ghosts.rs` | The best lap raced as a ghost in Time Trial and Speed Lap, one replay file per `records.toml` key, written only when a lap beats it. See [ADR-0055](adr/0055-replays-are-inputs-and-a-ghost-is-poses.md). |

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
best_medal = "gold"
best_points = 3
last_medal = "gold"
```

`title`, `track`, `mode` and `class` together are the key - see
[`oag_game::records::Key`](../../crates/game/src/records.rs) for exactly how
each part is spelled and lower-cased. `best_lap_ticks` and
`best_total_ticks` are the two numbers worth keeping forever: the quickest
completed lap and the quickest *finished* race ever recorded on that row,
each only ever improving. The five `last_*` fields are the most recent race
on that row, whatever it says - better or worse than the standing best,
because "last" is a different question than "best".

`best_medal`, `best_points` and `last_medal` are the campaign medal fields,
added 2026-09-09 once `oag_tables::race_campaign::Cell::evaluate_medal`
reimplemented `Cell_EvaluateMedal`'s law - see
[`docs/formats/race-campaign.md`](../formats/race-campaign.md). `Medal` is
serialized as a lowercase word, never the bare ordinal the original's own
`0 = gold` convention would suggest: the in-race HUD carries a *different*
medal-tier ordinal running the opposite direction, so a stored integer would
misread. **`best_medal`/`best_points` are `None` on every row today**, not
because the law is unimplemented but because nothing yet selects *which*
campaign cell a real race was run against, or converts that race's result
into the value `evaluate_medal` wants - see
[`oag_game::records::Observation::campaign_medal`](../../crates/game/src/records.rs)'s
own doc, and the join proven with an invented cell in
`crates/game/tests/campaign_medal.rs`.

Every value is ticks (`u32`/`u64`), the same unit `oag_hud::format_lap_time`
already takes, never a formatted string - formatting is a presentation
concern, and this file is not one.

## Where a race's result is captured, and why there are two sites

**Never from inside `Race::tick`.** This project's core architectural rule -
the simulation must not know a renderer, or by extension a save system,
exists - applies here exactly as it does to the scoreboard the results table
is already built from. Nothing in `oag-race`, `oag-gameplay`, `oag-ai` or
`crates/raceplay/src/` calls into `records.rs`, imports it, or knows it
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

**Only `Session::frame`'s finish arm also builds a
`oag_game::records::PersonalBest`** - see "the results table" below - because
that is the one site whose race actually has a `scoreboard::Board` for it to
be drawn on; `Session::escape` leaves before a results table exists on the
two modes it alone captures.

## The results table

`crate::scoreboard::draw_list` draws up to two extra lines under the `RACE
TIME` footer, in the same all-caps convention every other label on the panel
already uses:

- `PERSONAL BEST LAP <time>`, shown whenever the row's own comparison has a
  best lap to report at all - which, after the very first race on a key, is
  always, since `Store::record`'s "seeded from itself" rule means a first
  race sets its own best lap.
- `BEST MEDAL <GOLD|SILVER|BRONZE>`, shown only when the row carries one -
  which, per "where a career system attaches" above, is never yet, since
  nothing selects a campaign cell for a real race.

Either line highlights in the same colour the player's own standings row
already uses (`scoreboard::PLAYER`) and appends `- NEW!` when *this* race is
the one that set the figure - `oag_game::records::PersonalBest::compare`'s
`lap_improved`/`medal_improved` flags, not re-derived by the drawing code.
`PersonalBest::compare` is a pure function of a `Record` (or none) and an
`Observation`, mirroring `Store::record`'s own best-of rule without calling
it - `Session::frame`'s finish arm reads the row `Store::get` returns
*before* folding this race's `Observation` in, because `Store::record`
mutates the row in place and the comparison needs "before" and "after" both.

A `--race --screenshot` capture (`crates/game/src/race_capture.rs`,
`crate::capture::run`) draws the same line, reading (never writing)
whatever `records.toml` already holds for the key it resolves the same way
`Stage::build_race_stage` does - see
`race::CaptureOptions::previous_best`'s own doc for why that path stays
read-only.

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

`Record` carries a best-ever campaign medal and its points, and the most
recent race's own medal - `oag_tables::race_campaign::Cell::evaluate_medal`
reimplements the law that produces one (`Cell_EvaluateMedal`). Still absent
is an unlock or a tournament standing. Two ways to grow this file further,
still additive, neither needing `Key`, `parse` or `Store::record` to change:

- A new `#[serde(default)]` **field** on `Record`, for something that is
  still one number per circuit/mode/class - the difficulty a medal was
  earned at, say.
- A new **sibling table** in the same file, alongside `[[records]]`, for
  something that spans more than one circuit - a tournament standing.

**2026-09-14: a campaign cell is now such a sibling table, `[[campaign]]`,
keyed on `(title, cell name)`.** This is the wiring the paragraph below used
to say was missing - see `crates/game/src/records.rs`'s own module doc,
"where a career system attaches", for the full reasoning. In short: keying
a campaign medal by `(title, track, mode, class)` - `Key`'s own four parts -
would have meant two different cells that share a track/mode/class
overwriting one row, and a `Zone` cell's own `class="Zone"` cannot even
build a matching `Key` at all (`Race::start` falls back to whatever class
was last selected for a Zone launch, since this engine has no Zone handling
block of its own - see [race-modes.md](../gameplay/race-modes.md)'s own
"every title ships one Zone handling block, and this engine does not read
it"). Keying on the cell's own `name` string instead is also what the
original measurably does - see
[race-campaign.md](../ghidra/functions/psp-pulse-usa/race-campaign.md)'s
"how a campaign event launches" - and needs no change to the four-part `Key`
at all. `Store::campaign_medal`/`Store::record_campaign` are the two new
methods; `oag_ui_screens::campaign::GridSummary::from_grid_with_medals`/
`CellSelection::with_medals` are what `Grid Selection`'s `Medals`/`Points`
rows and `Cell Selection`'s `Line6`/`Line7` read them through, fed by
`crates/game/src/main/campaign_stage.rs`.

The ordinary `[[records]]` row for a campaign race is still written too, at
its usual `(title, track, mode, class)` key - the two tables answer
different questions ("the best result on this track/mode/class" against
"what did this cell earn") and a campaign race contributes to both.

**2026-09-21: Wipeout 2048's own `SP.xml` campaign is the same
`[[campaign]]` table's second tenant**, keyed on `(title, event name)` the
same way a `PI_Grid` cell is keyed on `(title, cell name)` - no schema
change, same reasoning: 2048's own event names are unique the way a cell's
`name` is, and a track/mode/class key would collide across the many events
that share a circuit. `oag_2048::campaign::{event_objectives, EventOutcome,
Tier, evaluate_tier}` (`docs/formats/2048-campaign.md`'s "The objective law"
section) is this title's own medal law, read off `M_PASSOBJECTIVE`/
`M_ELITEOBJECTIVE` rather than reused from `Cell_EvaluateMedal` - 2048's
own law is genuinely two-tier (pass/elite, never a third authored rung),
where Pulse/HD's is three-tier (bronze/silver/gold). `RaceStage::observation`
(`crates/game/src/main/race_stage.rs`) maps `Tier::Elite` to `Medal::Gold`
and `Tier::Pass` to `Medal::Bronze` - **chosen, not measured, no confidence
score**: floor to floor and top to top, `Medal::Silver` never produced by
this title. `race::load_event` resolves the event's own objectives once, at
load (`race::Loaded::campaign_2048_event`), the same "resolved once, at
load" shape [`RaceStage::result_key`](#the-key-resolved-once-at-load-never-re-derived)
already follows, rather than re-parsing `SP.xml` at grading time.

The map itself never re-reads `records.toml` live: `oag_ui::frontend::
Frontend::refresh_campaign_progress` folds a `Store` snapshot in once, at
`Session::finish_loading`, because a race on this title never returns to the
touch front end within one process - see
`docs/formats/2048-frontend.md`'s "Wired" section. `MapEvent::requires`
(the unlock graph, `oag_2048::campaign::unlock_gates`) is disc-only and
carries no dependency on this file's own schema at all; only the tier
*earned* against a gate's own name comes from `[[campaign]]`.

## What is not built yet

There is still no records **browser** - a circuit list plus every stored best
time, reachable from the menus rather than only shown after a race just run
on that circuit. That is a bigger, separate piece, probably its own
`menu.toml` page (which, unlike the results table's own free text, *would*
need `string_id`s - see `scripts/check-strings.py`). Not started.

**A campaign cell now reaches `Observation::campaign_medal` for real**,
closing what this section used to describe as the gap:
`Session::launch_campaign_cell` (`crates/game/src/main/session/campaign.rs`)
resolves the cell's own track against the open source
(`crate::catalogue::Track::entry_name`, matched by the cell's own `track=`
id, which turned out to share `catalogue::Track::id`'s own spelling - no
mismatch to bridge there after all), maps `oag_tables::race_campaign::Mode`
onto the five `oag_race::Mode` this engine implements
(`oag_game::campaign::race_mode_for_cell`, `None` for `Tournament`/
`Head2Head`/`Custom Grid`/`AI Race`, which refuse to launch and log why),
and carries the cell itself on `Session::campaign_cell` through to
`RaceStage::campaign_cell`. `RaceStage::observation` (`crates/game/src/main/race_stage.rs`)
evaluates the per-mode value and calls `Cell::evaluate_medal`:

- `Race` and `Time Trial` gate on `finished` - a running position or an
  in-progress clock is not a result, only a completed one.
- `Time Trial`'s value is the finish tick, converted to centiseconds.
  Confirmed against `Data\Plugins\grids\grid_00.xml`'s own authored
  targets, not merely inferred from the field list: a cell's gold sits at
  `10000`-`11500` (100-115 s), which is a **3-lap total**, not a lap -
  `race-modes.md` measures a 3-lap Venom race ending "around tick 7,500",
  ~125 s. This is new since the "Next Steps" line below was written, which
  had named `Observation::tick` without checking which quantity a real
  target actually measures.
- `Speed Lap`'s value is the best single lap, not the tick - checked the
  same way: `03_Track`'s own Speed Lap cell golds `4000` (40 s) against
  `laps="7"`, a fifth of a 7-lap total at the same pace and squarely one
  lap's own length. Gated on nothing, since the mode never finishes and the
  best lap so far only ever improves - unlike the tick, which would read as
  "however long the player happened to stay" on an escaped run.
- `Zone`'s value is `RaceState::zone`, the count of *completed* 10-second
  steps (`advance_zone` increments it only once a step finishes), gated on
  nothing for the same "only ever grows" reason as Speed Lap's lap.
- `Elimination`'s value is the player's own kill count, gated on nothing,
  same reason.

See `RaceStage::campaign_medal`'s own doc comment for the full per-mode
table and the worked evidence.

**Two things this pass chose rather than measured, both logged when they
happen:** a `Zone` cell's own `class="Zone"` is not a speed class this
engine's handling reads, so a Zone launch races at whatever class the RACE
page last had selected; and the campaign's own `skill`/`skillEasy`/
`skillHard` AI-difficulty curve (`AI_ResolveSkillScale`) is not
implemented, so every campaign launch uses the ordinary `[ai] difficulty`
setting rather than the cell's own figure. Neither invents a number the
disc does not author - both are documented gaps this engine already had
before the campaign could reach them, now visibly reached.

**Still not implemented: `Head2Head` at all** - no `oag_race::Mode` variant
exists for it, see `oag_game::campaign::race_mode_for_cell`'s own doc.
`Tournament`'s own per-leg state is wired (`docs/gameplay/race-modes.md#tournament`).

**2026-09-23: grid-tier locking (`Unlock_GridPointsMet`) is wired too.**
`oag_tables::race_campaign::grid_points_met` reimplements the named-grid
comparison directly - `Unlock_GridName`'s target, matched case-insensitively,
against that grid's own `points_earned` and `required_points` - and
`CampaignStage::grid_is_unlocked` (`crates/game/src/main/campaign_stage.rs`)
is what `Session::handle_campaign` gates `Grid Selection`'s own Confirm on,
in place of the display-only lock-glyph shortcut it used to reuse for the
purpose. See [race-campaign.md](../formats/race-campaign.md) and
`docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s "Grid0..Grid14:
what the gate actually evaluates".
