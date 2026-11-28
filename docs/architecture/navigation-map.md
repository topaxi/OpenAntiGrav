# Navigation map: what is on screen, and what survives a switch

This page answers one question without reading `crates/game`: when the window
switches from one screen to another, what state is carried, what is dropped, and
who owns it. It maps our own code, so it carries no confidence scores. Every
`file:line` is pinned to commit `e30b48161`; lines drift, function names do not.

[ADR-0099](adr/0099-a-run-context-and-an-activity-stack-own-navigation.md)
is the structure this page now describes: migration steps 1 to 3 have landed
(the run context, the tournament on the run, the activity stack). Line numbers
in the sections the stack rewrote are dropped; function names hold.

Related pages, not repeated here: the boot sequence ([front-end boot](frontend-boot.md)),
the race load and its hand-off ([load-to-race transition](race-load-transition.md)),
the menu tree ([menus](menus.md)), the replay viewer ([replays](../ui/replays.md))
and the replay timeline ([replay timeline](replay-timeline.md)).
[Engine lifecycle](engine-lifecycle.md)'s "States" diagram is the early plan; this
page is what is built.

## The three layers

What is on screen is decided in three places, not one.

1. **The activity stack** (`crates/game/src/app/activity_stack.rs`), one field
   `Session::stack` of `Stage` entries (`crates/game/src/app/stage.rs`). Five
   variants: `Launcher` (the disc chooser), `Loading` (two jobs: the boot's
   movie wait and a race load), `Frontend` (the boot reel and language picker, and
   2048's hosted campaign map), `Menu` and `Race`. The top entry is on screen;
   every entry under it is alive and paused. Each entry names the run it belongs
   to (`RunId`, kept on `RunContext::id`).
2. **Screens inside `Stage::Menu`.** `MenuStage` (`crates/game/src/app/menu_stage.rs`)
   holds the menu tree plus one `Option` per full-screen flow drawn over it:
   `picker` and `partner` (Track and Team Selection), `campaign` (Grid and Cell
   Selection), `pure_chain` (Pure's event chain), `join`, `lan`, `layout`
   (window layout), `assign` (seat assignment) and `prompt`. More than one can be
   `Some`; the order `Session::escape` tests them in
   (`crates/game/src/app/session/escape.rs:210`) is the only stack order there is:
   picker, campaign, assign, layout, lan, join, pure chain, prompt, then the menu
   tree's own `back()`.
3. **Overlays inside `Stage::Race`**, held on `Session`, not on the stage:
   `pause` (the pause state), `photo` (photo mode), `viewer` (the replay viewer
   over a rebuilt race), and on `RaceStage` itself `endrace` (the end-of-race
   screens) and `demo` / `benchmark` (what kind of race this is).

## Where the switch happens

Four exits write the stack and nothing else does (`ActivityStack::push`, `pop`,
`replace`, `unwind_to`); each hands back what it removed, so the session
decides where a race scene is released. `rg "self\.stack\.(push|pop|replace|unwind_to|replace_all|take_paused_race)\(" crates/game/src`
lists every write. `replace_all` (a pop of every entry, then a replace of the
bottom) and `take_paused_race` (pop the pause menu, then let it replace the race
under it) are built from those exits in `crates/game/src/app/stage/held.rs`.

| Site | Exit | From -> to | What happens to the outgoing entry |
| --- | --- | --- | --- |
| `App` build (`app/handler.rs`) | the first entry | nothing -> `Launcher`, `Race` (`--race`) or `Loading` (boot) | none |
| `finish_launcher` (`session/load.rs`) | replace | `Launcher` -> `Loading` (boot) | dropped |
| `finish_loading` (`session/load.rs`) | replace | `Loading` (boot) -> `Frontend` | dropped; the boot shell moves into `Session::shell` |
| `open_menus` (`session/menus.rs`) | push over an unfinished race (pause), else unwind to menus and replace | any -> `Menu` | an unfinished, unnetworked, unwatched race that is not a demo or benchmark stays under the menus; anything else is dropped |
| `start_race_loading` (`session/load.rs`) | `replace_all` | any -> `Loading` (race) | every entry dropped; `launch_race` and `start_watching` first drop a paused race off the frame thread (`drop_paused_race`) |
| `finish_race_loading` (`session/load.rs`) | replace | `Loading` (race) -> `Race` | dropped once the built race is taken |
| `resume_race` (`session/escape.rs`) | pop | `Menu` -> the paused `Race` | the menu is dropped; `resumed()` gives the race its music, rumble and title back |
| `open_hosted_map` (`session/hosted_map.rs`) | push | `Menu` -> `Frontend` (hosted map) | stays under the map |
| `poll_hosted_map` (`session/hosted_map.rs`) | pop | `Frontend` (hosted map) -> `Menu` | the map is dropped |
| `quit_parked_race`, `quit_campaign_race` | `take_paused_race` | `Menu` over `Race` -> `Menu` | the paused race is dropped |

A pop drops the popped entry's run when no entry left names it: a race's run
goes with its race (`RaceStage::run`), and the pending `Session::run` is ended
(`Session::pop_activity`). The entry underneath is told it is resumed
(`Session::resumed`); **a finished race popped back to keeps `result_saved`**
(`activity_stack::rearms_on_resume`), so its result is not recorded twice.

Reproduce the counts:

```sh
rg -c "Stage::(Launcher|Loading|Frontend|Menu|Race)\b" crates/game | awk -F: '{s+=$2} END {print s}'
rg -n "self\.stack\.(push|pop|replace|unwind_to|replace_all|take_paused_race)\(" crates/game/src
```

## Transitions by flow

Each row is a trigger and the function chain it runs. "Carried" is what the
next screen can still read; "dropped" is what is gone. File paths are under
`crates/game/src/app/session/` unless they say otherwise.

### Boot

| From -> to | Trigger | Chain | Carried | Dropped |
| --- | --- | --- | --- | --- |
| Launcher -> Loading | a disc picked | `frame` -> `finish_launcher` (`load.rs:53`) | `race_options` built from the chosen disc (`load.rs:84`) | the chooser |
| Loading -> Frontend | movies decoded | `frame` -> `finish_loading` (`load.rs:217`) | the boot shell, as `Session::shell` | the loading screen |
| Frontend -> Menu | START at the title screen | `frame` (`frame.rs:175`) -> `open_menus`, then `follow_launch` | the front end's held backdrop picture, and a launch the front end asked for (2048) | the front end |

### Front end

| From -> to | Trigger | Chain | Carried | Dropped |
| --- | --- | --- | --- | --- |
| Menu -> Menu | any row, page or screen | `handle_menu`, `handle_picker`, `handle_campaign`, ... | everything: the stage is not replaced | nothing; a screen is an `Option` on `MenuStage` set to `None` |
| Menu -> Frontend (map) | 2048 campaign tile | `handle_campaign` -> `open_hosted_map` | the menu, under the map on the stack | nothing |
| Frontend (map) -> Menu | map left or event launched | `poll_hosted_map` (a pop) -> `follow_launch` | the menu under the map; an event name in `pending_event`, its card in `pending_card` | the map |

### Race start

Every launch ends in `launch_race` (`load.rs:473`) -> `start_race_loading`
(`load.rs:646`), except a replay watch, which calls `start_race_loading`
directly.

| Trigger | Chain | Writes first |
| --- | --- | --- |
| RACE page START | `start_race_box` (`menus.rs:530`) -> pickers -> `launch_from_settings` (`menus.rs:665`) -> `finish_launch` | `start_race_box` and `launch_from_settings` end the pending run (`Session::end_run`); `launch_from_settings` rebuilds `race_options` |
| campaign cell | `handle_campaign` -> `launch_campaign_cell` (`campaign.rs:524`) -> `launch_campaign_race` -> `finish_launch` | `race_options`, a fresh `run` with `campaign_cell`, `campaign_difficulty`, `campaign_ai_skill_scale`, `campaign_home` for a mod event, `tournament` for a Tournament cell |
| 2048 event | `follow_launch` (`launch2048.rs:24`) -> `launch_race` | a fresh `run` with `pending_event` and `pending_card`; a tier cell sets the run's `campaign_add_on` |
| attract demo | `launch_demo` (`demo.rs:168`) | moves the player's `run` into `demo.saved` and launches a run of its own whose `options` are the demo's; `race_options` untouched |
| benchmark | `launch_benchmark` (`benchmark.rs:244`) | moves the player's `run` and autopilot fields into `benchmark.saved`, launches a run of its own; `race_options` untouched |
| LAN / online | `launch_lan_race` (`lan.rs:735`), `advance_online` | `net` |
| launch keys | `launch_keyed` | ends the pending run, `race_options` |
| WATCH, WATCH REPLAY, a shot | `watch_path` or `begin_shot` (`shot_open.rs`) -> `start_watching` (`replays.rs`) | `watching`; `watch_path` ends the pending run and starts the watch's own, whose `origin` is the REPLAYS page; drops a paused race, clears the run's `pending_event` and `pending_card` |

`launch_race` and `start_watching` both **drop a paused race** before they
load (`drop_paused_race`): a race paused behind the menus does not survive a new
launch or a replay watch.

While the race builds, `advance_race_build` (`load.rs:781`) decides what kind of
race it is by which `Session` slot is `Some`, in this order: `net` (a network
race), `demo.loading` (a demo), `watching` (the viewer, `load.rs:874`), else an
ordinary race. In the same step it **moves** `Session::run` into the new
`RaceStage` (`RaceStage::run`). `launch_race` filled the run's `options` with
`race_options` as it saw them, before its per-launch refresh. From here on the
race, not the session, owns the run.

### In the race

| From -> to | Trigger | Chain | Carried | Dropped |
| --- | --- | --- | --- | --- |
| Race -> Menu (pause) | pause on a title with a pause screen | `escape` -> `open_menus` (a push) | the race, under the menus (unfinished, not networked, not watched, not a demo or benchmark) | nothing |
| Menu (pause) -> Race | RESUME, or the pause page closed | `handle_menu` -> `resume_race` (a pop) | the paused race | the pause menu |
| Menu (pause) -> Loading | RESTART | `restart_parked_race` (`pause_menu.rs:209`) -> `finish_launch` | `campaign_*` re-armed from the parked race | the parked race |
| Race -> Menu (quit) | QUIT on the pause menu, or escape on a title with no pause screen | `quit_parked_race` -> `quit_campaign_race` or `leave_finished_race`; `escape` -> `quit_campaign_race` for a campaign race, else `open_menus` | a campaign race reopens its Cell Selection; a plain escape pauses an unfinished race under the menus | the race and `tournament` on the quit paths (both reach `leave_finished_race`) |
| Race -> Race (photo) | photo mode | `photo` set on `Session` | the race, ticking stopped | nothing |
| Race -> Menu | network race ended early | `frame` (`frame.rs:1107`) -> `open_menus` | nothing | the race, `net`, `lan` |

### Race end

The end-of-race screens live on the finished `RaceStage` (`endrace`), so they
exist only while that race is on the stack. `handle_endrace_menu_option`
(`endrace.rs:557`):

| Option | Chain | Carried | Dropped |
| --- | --- | --- | --- |
| RACE AGAIN | clones the race's run into `Session::run` (`rerun_race_on_screen`), `finish_launch` | the run (options, `campaign_*`, `tournament`) | the finished race |
| NEXT RACE | `advance_tournament_leg`: the same clone, `Progress::advance`, the run's `options.track`, `finish_launch` | the run, its `tournament` standings advanced | the finished leg |
| RETURN TO GRID | `return_to_campaign` -> `leave_finished_race` -> `reopen_cell_selection` | the cell name | the race, `tournament` |
| RETURN TO MENU | `leave_finished_race` (`endrace.rs:681`) -> `open_menus` (-> `reenter_multiplayer` for two people) | the split lobby | the race, `tournament` |
| WATCH REPLAY | `watch_last_race` (`replays.rs:335`) -> `watch_path` -> `start_watching` -> `start_race_loading` | the recording's path | **the finished race and its end screens** (see below) |

A title with no end menu (Pure) confirms the results board in `frame`
(`frame.rs:559`): NEXT leg if the tournament has one, else `escape`.

### Replay

| From -> to | Trigger | Chain | Carried | Dropped |
| --- | --- | --- | --- | --- |
| Menu (REPLAYS) -> Loading | WATCH | `watch_selected_replay` -> `watch_path` -> `start_watching` | a run whose `origin` is `Origin::Replays` | a paused race |
| Loading -> Race (viewer) | race built | `advance_race_build` -> `open_viewer`; `viewer` set | the rebuilt race, which takes the watch's run | `watching` |
| Race (viewer) -> Menu | EXIT or escape | `leave_viewer` reads the race's run `origin` (none for a shot) -> `open_menus_from`, which pushes the REPLAYS page | nothing | the viewer's race (never paused: `watching` is set on it) |

A recording whose load fails lands on REPLAYS the same way: the failed-load arm
ends the run and passes its `origin` on (`open_viewer`'s refusal puts the run
back to pending first).

### Campaign and tournament

Grid, Cell Selection and Pure's chain are `MenuStage` screens (layer 2); a
cell's race is an ordinary race start. A Tournament cell differs only in its
run's `tournament` (`RunContext::tournament`, ADR-0099 migration step 2), a
`race::tournament::Progress` created by `launch_campaign_cell`.
`record_finished_leg` (called from `frame`) folds each leg's board into the
race's run, `tournament_has_next_leg` turns RACE AGAIN into NEXT RACE, and NEXT
RACE carries the run on to the next leg. It ends with its run: the finished
race dropped when its end menu is left, Back out of Team Selection
(`end_run`), any RACE-page, keyed or 2048 launch (`end_run`), and
`finish_race_loading`'s failed-load arm (see "A leg that will not load").

### Network

`open_menus` sets `net` and `lan` to `None` unconditionally (`menus.rs:378`), so
any return to the menus leaves a LAN or online room. `online_back_to_room`
(`online.rs:1467`) is the online lobby's own way back. A networked race is never
parked, and the end-of-race menu offers no WATCH REPLAY while `net` is up.

## Single-slot holders

Each of these is one value on `Session` that more than one flow writes. A nested
flow that writes it overwrites the outer flow's value; nothing restores it
unless the flow saved it by hand.

| Holder | Written by | Collision |
| --- | --- | --- |
| `stack` | the four exits, at the sites above | a launch from any screen replaces the whole stack; a paused race is dropped by `launch_race`, `start_watching`, the quit paths: WATCH from REPLAYS over a paused race loses the race; the end screens die with their race |
| `race_options` | boot, `launch_from_settings`, campaign launch, launch keys, `restore_campaign_home` | the RACE page's options and a campaign's still share one slot; a launch snapshots it into its run (`RunContext::options`), so a relaunch, a demo and a benchmark no longer write it |
| `run` (`RunContext`: `options`, `campaign_cell`, `campaign_custom`, `campaign_difficulty`, `campaign_ai_skill_scale`, `campaign_add_on`, `campaign_home`, `pending_event`, `pending_card`, `tournament`) | campaign launch, 2048 launch, `launch_race`'s snapshot, RACE AGAIN, NEXT RACE, RESTART, demo, benchmark; ended by a RACE-page or keyed launch and a failed load | moved into the race at build (ADR-0099 migration steps 1 and 2); RACE AGAIN, NEXT RACE and RESTART clone it back from the race; a Tournament's standings end with it |
| `campaign_home` (on the run) | a mod campaign event | `open_menus` restores `race_options`' source, craft source and difficulty from the pending run's or the race on screen's, on every return, whatever the return is for |
| `watching`, `viewer` | replay watch | `watching` is also how `advance_race_build` knows the race is for the viewer |
| `demo.saved`, `benchmark.saved` | demo, benchmark | the player's pending run moved aside (and the benchmark's settings and autopilot): a push and pop without a stack |
| `net`, `lan` | LAN, online | dropped by every `open_menus` |
| `split_lobby` | multiplayer page | read back by `reenter_multiplayer` after a race |
| `pause`, `photo` | race overlays | one each; photo mode over the viewer is a mode of `photo` |

`RaceStage` also holds the launch facts a finished race needs for its end
screens and its relaunch: its `run` (`campaign_cell`, `campaign_custom`,
`campaign_difficulty`, `campaign_ai_skill_scale`, the launch options), `campaign_2048_event`,
`tournament_final_rank`, `result_key`, `endrace` and `replay_header`. They are
dropped with the race.

## The failing case: WATCH REPLAY in the middle of a Tournament

Wanted: after watching a mid-Tournament leg, the leg's end menu returns with
NEXT RACE, and the tournament continues at the next leg with its standings
intact.

**Today the case cannot be reached.** `WatchReplayOffer::on`
(`crates/game/src/app/race_stage/endrace.rs:742`) refuses the row while
`tournament_next_leg` is set (`endrace.rs:423` in the session). On a Tournament's
last leg the row is offered. The trace below is what the guard prevents.

1. **Launch.** `launch_campaign_cell` writes `race_options` and a fresh run with
   `campaign_cell`, `campaign_difficulty`, `campaign_ai_skill_scale` and
   `tournament = Some(Progress)`. `advance_race_build` moves the run into the
   leg's `RaceStage`.
2. **The leg finishes.** `record_finished_leg` folds the board into the race's
   run. The end menu is built with NEXT RACE.
3. **WATCH REPLAY.** `watch_last_race` writes the recording, then
   `start_watching` drops a paused race and calls `start_race_loading`, whose
   `replace_all` **drops the finished leg's `RaceStage`**, and its run with it: the cell, rung, skill scale, custom
   launch, the standings, `tournament_final_rank` and the end screens with
   their cursor. NEXT RACE clones the run from `Stage::Race`, so after this
   point there is nothing to continue from.
4. **The viewer's race builds** with no run of the tournament's.
5. **EXIT.** `leave_viewer` reads the watch run's REPLAYS origin and calls
   `open_menus_from`. The viewer's race is not kept. The REPLAYS page is pushed. There is no
   path back to an end menu: it existed only on the dropped race.

Since ADR-0099's migration step 2 the whole tournament (standings and leg
context) sits on the run, so it lives or dies in one piece. A fix has to keep
the finished race, and so its run, alive behind the viewer (migration steps 3
and 4).

### A leg that will not load

A leg whose load fails takes `finish_race_loading`'s error arm, which ends the
run (`end_run`) and then calls `open_menus`. The leg's run is still the pending
one on `Session` (the race build never took it), so the tournament ends with it
and its standings are dropped.

Without that the standings outlived the leg. Observed on Pulse PSP EU with a
two-leg mod Tournament whose first circuit was broken by an override: the next
RACE-page race's end menu led with NEXT RACE, offered no RACE AGAIN and no WATCH
REPLAY, and confirming it loaded the tournament's second circuit.
`drive_ground_truth::tournament` drives that flow and reads the `tournament` key
of the drive state (the run on screen's, else the pending run's, else the
parked race's). Without the arm's `end_run` it fails on the menus the failed
load returns to (`tournament` reads `1/2`).

**A second failure path is unverified.** `finish_launch` only logs a synchronous
`launch_race` error. After NEXT RACE that would leave the finished leg on screen
with the pending run's `Progress` advanced and the race's own not; no way to
make it happen was found, and RETURN TO MENUS from there drops the race's run.

**The last leg no longer leaves it set.** WATCH REPLAY is offered on a
Tournament's last leg; the watch's load drops the finished race and its run, so
the standings go with them (by construction, ADR-0099 migration step 2).
