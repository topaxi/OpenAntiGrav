# Navigation map: what is on screen, and what survives a switch

This page answers one question without reading `crates/game`: when the window
switches from one screen to another, what state is carried, what is dropped, and
who owns it. It maps our own code, so it carries no confidence scores. Every
`file:line` is pinned to commit `e30b48161`; lines drift, function names do not.

[ADR-0099](adr/0099-a-run-context-and-an-activity-stack-own-navigation.md)
proposes the structure that would replace what this page describes.

Related pages, not repeated here: the boot sequence ([front-end boot](frontend-boot.md)),
the race load and its hand-off ([load-to-race transition](race-load-transition.md)),
the menu tree ([menus](menus.md)), the replay viewer ([replays](../ui/replays.md))
and the replay timeline ([replay timeline](replay-timeline.md)).
[Engine lifecycle](engine-lifecycle.md)'s "States" diagram is the early plan; this
page is what is built.

## The three layers

What is on screen is decided in three places, not one.

1. **`Stage`** (`crates/game/src/app/stage.rs:31`), one field `Session::stage`.
   Five variants: `Launcher` (the disc chooser), `Loading` (two jobs: the boot's
   movie wait and a race load), `Frontend` (the boot reel and language picker, and
   2048's hosted campaign map), `Menu` and `Race`. Whatever is not the current
   variant is dropped unless something parks it.
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

`rg` finds about 420 mentions of a `Stage` variant in `crates/game`, almost all
of them `match`es and `matches!` tests. **Eleven sites write `Session::stage`.**
Every other transition is a call into one of them.

| Site | From | To | What it does with the outgoing stage |
| --- | --- | --- | --- |
| `app/handler.rs:294` | nothing | `Launcher` | first stage when several discs were found |
| `app/handler.rs:329` | nothing | `Race` | `--race`: straight onto a track, no menus ever built |
| `app/handler.rs:400` | nothing | `Loading` (boot) | every other windowed boot |
| `session/load.rs:93` (`finish_launcher`) | `Launcher` | `Loading` (boot) | dropped |
| `session/load.rs:398` (`finish_loading`) | `Loading` (boot) | `Frontend` | dropped; the boot shell moves into `Session::shell` |
| `session/menus.rs:304` (`open_menus`) | any | `Menu` | `mem::replace`; an unfinished race is parked in `suspended_race` (`menus.rs:387`), anything else dropped |
| `session/load.rs:739` (`start_race_loading`) | any | `Loading` (race) | **plain assignment: dropped**, whatever it was |
| `session/load.rs:1187` (`finish_race_loading`) | `Loading` (race) | `Race` | dropped once the built race is taken |
| `session/escape.rs:93` (`resume_race`) | `Menu` | `Race` | the menu is dropped; the race comes back out of `suspended_race` |
| `session/hosted_map.rs:68` (`open_hosted_map`) | `Menu` | `Frontend` (hosted map) | the menu is parked in `parked_menu` |
| `session/hosted_map.rs:97` (`poll_hosted_map`) | `Frontend` (hosted map) | `Menu` | the menu comes back out of `parked_menu` |

So the engine has two parked slots and one depth each: `suspended_race` (a race
behind the menus) and `parked_menu` (the menus behind 2048's map). Nothing can
be parked behind a race, and nothing can be parked two deep.

Reproduce the counts:

```sh
rg -c "Stage::(Launcher|Loading|Frontend|Menu|Race)\b" crates/game | awk -F: '{s+=$2} END {print s}'
rg -n "self\.stage = " crates/game/src
rg -nU "mem::replace\(\s*&mut self\.stage" crates/game/src
rg -n "Stage::(launcher|race|loading)\(" crates/game/src/app/handler.rs
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
| Menu -> Frontend (map) | 2048 campaign tile | `handle_campaign` -> `open_hosted_map` (`hosted_map.rs:23`) | the menu, in `parked_menu` | nothing |
| Frontend (map) -> Menu | map left or event launched | `poll_hosted_map` (`hosted_map.rs:80`) -> `follow_launch` | the parked menu; an event name in `pending_event`, its card in `pending_card` | the map |

### Race start

Every launch ends in `launch_race` (`load.rs:473`) -> `start_race_loading`
(`load.rs:646`), except a replay watch, which calls `start_race_loading`
directly.

| Trigger | Chain | Writes first |
| --- | --- | --- |
| RACE page START | `start_race_box` (`menus.rs:530`) -> pickers -> `launch_from_settings` (`menus.rs:665`) -> `finish_launch` | `start_race_box` and `launch_from_settings` end the pending run (`Session::end_run`); `launch_from_settings` rebuilds `race_options` |
| campaign cell | `handle_campaign` -> `launch_campaign_cell` (`campaign.rs:524`) -> `launch_campaign_race` -> `finish_launch` | `race_options`, a fresh `run` with `campaign_cell`, `campaign_difficulty`, `campaign_ai_skill_scale`, `campaign_home` for a mod event; `tournament` |
| 2048 event | `follow_launch` (`launch2048.rs:24`) -> `launch_race` | a fresh `run` with `pending_event` and `pending_card`; a tier cell sets the run's `campaign_add_on` |
| attract demo | `launch_demo` (`demo.rs:168`) | moves the player's `run` into `demo.saved` and launches a run of its own whose `options` are the demo's; `race_options` untouched |
| benchmark | `launch_benchmark` (`benchmark.rs:244`) | moves the player's `run` and autopilot fields into `benchmark.saved`, launches a run of its own; `race_options` untouched |
| LAN / online | `launch_lan_race` (`lan.rs:735`), `advance_online` | `net` |
| launch keys | `launch_keyed` | ends the pending run, `race_options` |
| WATCH, WATCH REPLAY, a shot | `watch_path` or `begin_shot` (`shot_open.rs:186`) -> `start_watching` (`replays.rs:427`) | `watching`; drops `suspended_race`, clears the run's `pending_event` and `pending_card` |

`launch_race` and `start_watching` both **drop `suspended_race`** before they
load (`load.rs:485`, `replays.rs:432`): a race paused behind the menus does not
survive a new launch or a replay watch.

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
| Race -> Menu (pause) | pause on a title with a pause screen | `escape` -> `open_menus` | the race, in `suspended_race` (unfinished, not networked, not watched, not a demo or benchmark) | nothing |
| Menu (pause) -> Race | RESUME, or the pause page closed | `handle_menu` -> `resume_race` (`escape.rs:64`) | the parked race | the pause menu |
| Menu (pause) -> Loading | RESTART | `restart_parked_race` (`pause_menu.rs:209`) -> `finish_launch` | `campaign_*` re-armed from the parked race | the parked race |
| Race -> Menu (quit) | QUIT on the pause menu, or escape on a title with no pause screen | `quit_parked_race` (`pause_menu.rs:230`) -> `quit_campaign_race` (`escape.rs:155`) or `leave_finished_race`; `escape` -> `quit_campaign_race` for a campaign race, else `open_menus` | a campaign race reopens its Cell Selection; a plain escape parks an unfinished race in `suspended_race` | the race and `tournament` on the quit paths (both reach `leave_finished_race`) |
| Race -> Race (photo) | photo mode | `photo` set on `Session` | the race, ticking stopped | nothing |
| Race -> Menu | network race ended early | `frame` (`frame.rs:1107`) -> `open_menus` | nothing | the race, `net`, `lan` |

### Race end

The end-of-race screens live on the finished `RaceStage` (`endrace`), so they
exist only while that race is `Session::stage`. `handle_endrace_menu_option`
(`endrace.rs:557`):

| Option | Chain | Carried | Dropped |
| --- | --- | --- | --- |
| RACE AGAIN | clones the race's run into `Session::run` (`rerun_race_on_screen`), `finish_launch` | the run (options and `campaign_*`), `tournament` | the finished race |
| NEXT RACE | `advance_tournament_leg`: the same clone, `Progress::advance`, the run's `options.track`, `finish_launch` | the run, `tournament` with its standings | the finished leg |
| RETURN TO GRID | `return_to_campaign` -> `leave_finished_race` -> `reopen_cell_selection` | the cell name | the race, `tournament` |
| RETURN TO MENU | `leave_finished_race` (`endrace.rs:681`) -> `open_menus` (-> `reenter_multiplayer` for two people) | the split lobby | the race, `tournament` |
| WATCH REPLAY | `watch_last_race` (`replays.rs:335`) -> `watch_path` -> `start_watching` -> `start_race_loading` | the recording's path | **the finished race and its end screens** (see below) |

A title with no end menu (Pure) confirms the results board in `frame`
(`frame.rs:559`): NEXT leg if the tournament has one, else `escape`.

### Replay

| From -> to | Trigger | Chain | Carried | Dropped |
| --- | --- | --- | --- | --- |
| Menu (REPLAYS) -> Loading | WATCH | `watch_selected_replay` -> `watch_path` -> `start_watching` | `return_to_replays = true` | `suspended_race` |
| Loading -> Race (viewer) | race built | `advance_race_build` -> `open_viewer`; `viewer` set, `return_to_replays = false` | the rebuilt race | `watching` |
| Race (viewer) -> Menu | EXIT or escape | `leave_viewer` (`replays.rs:804`): `return_to_replays = !from_shot`, `open_menus`, which pushes the REPLAYS page (`menus.rs:396`) | nothing | the viewer's race (never parked: `watching` is set on it) |

`return_to_replays` is a one-bit "where to go back to": it can name the REPLAYS
page and nothing else.

### Campaign and tournament

Grid, Cell Selection and Pure's chain are `MenuStage` screens (layer 2); a
cell's race is an ordinary race start. A Tournament cell differs only in
`Session::tournament`, a `race::tournament::Progress` created at
`campaign.rs:711` and the one launch fact that outlives a race:
`record_finished_leg` (`tournament.rs:37`, called from `frame.rs:1176`) folds
each leg's board into it, `tournament_has_next_leg` (`tournament.rs:54`) turns
RACE AGAIN into NEXT RACE, and `abandon_tournament` (`tournament.rs:111`) drops
it. Its four callers: `leave_finished_race`, `handle_picker`'s Back out of Team
Selection (`picker.rs:816`), `leave_shared` (`picker/seat.rs:259`) and
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
| `stage` | the eleven sites above | a launch from any screen drops that screen; the end screens die with their race |
| `suspended_race` | `open_menus` | dropped by `launch_race`, `start_watching`, `quit_campaign_race`: WATCH from REPLAYS over a paused race loses the race |
| `parked_menu` | `open_hosted_map` | 2048 only; one deep |
| `return_to_replays` | `watch_path`, `leave_viewer`, the viewer's open | a bool for one return target |
| `race_options` | boot, `launch_from_settings`, campaign launch, launch keys, `restore_campaign_home` | the RACE page's options and a campaign's still share one slot; a launch snapshots it into its run (`RunContext::options`), so a relaunch, a demo and a benchmark no longer write it |
| `run` (`RunContext`: `options`, `campaign_cell`, `campaign_custom`, `campaign_difficulty`, `campaign_ai_skill_scale`, `campaign_add_on`, `campaign_home`, `pending_event`, `pending_card`) | campaign launch, 2048 launch, `launch_race`'s snapshot, RACE AGAIN, NEXT RACE, RESTART, demo, benchmark; ended by a RACE-page or keyed launch and a failed load | moved into the race at build (ADR-0099 migration step 1); RACE AGAIN, NEXT RACE and RESTART clone it back from the race |
| `campaign_home` (on the run) | a mod campaign event | `open_menus` restores `race_options`' source, craft source and difficulty from the pending run's or the race on screen's, on every return, whatever the return is for |
| `tournament` | Tournament cell launch | outlives every race by design; cleared only at the four sites above |
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

1. **Launch.** `launch_campaign_cell` writes `race_options`, `campaign_cell`,
   `campaign_difficulty`, `campaign_ai_skill_scale` and
   `tournament = Some(Progress)`. `advance_race_build` drains the four
   `campaign_*` slots into the leg's `RaceStage`.
2. **The leg finishes.** `record_finished_leg` folds the board into
   `Session::tournament`. The end menu is built with NEXT RACE.
3. **WATCH REPLAY.** `watch_last_race` writes the recording, then
   `start_watching` drops `suspended_race` and calls `start_race_loading`, whose
   `self.stage = Stage::race_loading(...)` (`load.rs:739`) **drops the finished
   leg's `RaceStage`**. Lost with it: `campaign_cell`, `campaign_difficulty`,
   `campaign_ai_skill_scale`, `campaign_custom`, `tournament_final_rank` and the
   end screens with their cursor. This is the first and decisive loss:
   `advance_tournament_leg` reads the four `campaign_*` values back from
   `Stage::Race` (`tournament.rs:75`), so after this point NEXT RACE would launch
   the next leg with no cell, no medal and the ambient AI scale.
4. **The viewer's race builds.** `campaign_cell` on `Session` is already `None`
   (drained in step 1), so the viewer's race carries no campaign facts.
5. **EXIT.** `leave_viewer` sets `return_to_replays` and calls `open_menus`.
   `open_menus` runs `restore_campaign_home` (`menus.rs:147`), which resets
   `race_options`' source, craft source and difficulty when the cell was a mod
   campaign event. The viewer's race is not parked. The REPLAYS page is pushed.
   There is no path back to an end menu: it existed only on the dropped race.
6. **The standings survive, orphaned.** `Session::tournament` is still `Some`:
   only `leave_finished_race`, `handle_picker` and `leave_shared` clear it, and
   this path runs none of them. The standings are intact; everything needed to
   continue them is gone.

So the tournament context is split in two. The standings (`Progress`) sit on
`Session` and survive. The leg context (cell, difficulty, skill scale, custom
launch, final rank, the end screens) sits on the finished `RaceStage` and dies
at the first `self.stage =` that replaces it. A fix has to keep the second
alive behind the viewer, not just the first.

### A leg that will not load

A leg whose load fails takes `finish_race_loading`'s error arm, which calls
`abandon_tournament` and then `open_menus`. The leg's launch facts went with the
loading stage, so there is nothing to continue from: the tournament ends and its
standings are dropped. The arm clears it for every failed load: by reading, no
other load happens while a tournament has legs to go (WATCH REPLAY is refused
mid-tournament, and a paused leg's menus offer no launch).

Without that call the standings outlived the leg. Observed on Pulse PSP EU with a
two-leg mod Tournament whose first circuit was broken by an override: the next
RACE-page race's end menu led with NEXT RACE, offered no RACE AGAIN and no WATCH
REPLAY, and confirming it loaded the tournament's second circuit.
`drive_ground_truth::tournament` drives that flow and reads the `tournament` key
of the drive state.

`launch_from_settings` and `launch_keyed` (the two menu launch funnels) still do
not clear `tournament`; ADR-0099's migration step 2 (a run owns it, and ending
the run drops it) is where that becomes one place. This failed-load case is the
only part of that step built.

**A second failure path is unverified.** `finish_launch` only logs a synchronous
`launch_race` error. After NEXT RACE that would leave the finished leg on screen
with `Progress` already advanced; no way to make it happen was found, and
RETURN TO MENUS from there clears the tournament.

**The last leg leaves it set, harmlessly.** WATCH REPLAY is offered on a
Tournament's last leg, and that path (`start_watching`, `leave_viewer`,
`open_menus`) never calls `abandon_tournament`. With every leg run,
`tournament_has_next_leg` is false, the standings table is chosen by the race's
mode, not by `tournament` (`endrace.rs:260` in the session), and the medal reads
`tournament_final_rank` only for a Tournament cell, whose launch replaces
`tournament`. So the leftover `Progress` is overwritten by the next RACE-page
race's board (`record_leg` overwrites the current leg) and nothing shows it.
