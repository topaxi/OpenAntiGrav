# ADR-0099: A run context and an activity stack own navigation

## Status

Accepted (2026-11-28).

## Context

[The navigation map](../navigation-map.md) shows how the window decides what is
on screen today. Three facts from it drive this ADR.

1. **One screen at a time, two parked slots.** `Session::stage` holds one of five
   `Stage` variants. Eleven sites write it, and every write drops the outgoing
   stage unless it is one of two cases: an unfinished race parked behind the
   menus (`suspended_race`) or the menus parked behind 2048's map
   (`parked_menu`). Each slot is one deep. Nothing can be parked behind a
   finished race.
2. **The launch facts live in loose single slots.** `race_options`, the four
   `campaign_*` slots, `campaign_home`, `campaign_add_on`, `pending_event`,
   `pending_card` and `tournament` are fields on `Session`. A launch writes them,
   the race build drains most of them into the `RaceStage`, and RACE AGAIN and
   NEXT RACE copy them back out of the finished race. The attract demo and the
   benchmark save and restore them by hand (`demo.saved`, `benchmark.saved`): a
   push and a pop without a stack.
3. **A Tournament is split across both.** Its standings (`Session::tournament`)
   outlive every race. Its leg context (cell, difficulty, AI skill scale, custom
   launch, final rank, the end screens) lives on the finished `RaceStage` and dies
   with it.

The maintainer wants WATCH REPLAY from a mid-Tournament leg's end menu, and
leaving the replay to return to that menu with NEXT RACE. Today
`WatchReplayOffer::on` refuses the row there, because the watch's race load
replaces the finished leg and its end screens with it. The acceptance for the
fix:

> After watching a mid-Tournament leg, the leg's end menu returns with NEXT RACE,
> and the tournament continues at the next leg with its standings intact.

A second, smaller symptom is reachable now: a leg whose load fails returns to the
menus without clearing `tournament`, so the next unrelated race can be offered
NEXT RACE (the map's last section; found by reading, not yet run). WATCH REPLAY on a
Tournament's last leg leaves `tournament` set the same way, without a visible
effect today.

This ADR's `RunContext` is not ADR-0093's custom-campaign "run state". That one
is a mod's bounded blob; a `RunContext` for a custom campaign would hold it.

## Decision

1. **A `RunContext` owns everything a launched run carries across stages.** One
   value per run, where a run is what one launch started and every relaunch of
   it continues: a single race and its RACE AGAINs, a campaign cell, a whole
   Tournament, an attract demo chain, a benchmark, a replay watch, a network
   session. It holds the launch options (today `race_options` as the launch saw
   them), the campaign slots (`campaign_cell`, `campaign_custom`,
   `campaign_difficulty`, `campaign_ai_skill_scale`, `campaign_add_on`,
   `campaign_home`), 2048's `pending_event` and `pending_card`, the tournament's
   `Progress`, and the run's **origin**: the screen to return to when the run
   ends (the RACE page, a Cell Selection, the REPLAYS page, the main menu).
2. **A `RaceStage` borrows its run; it never owns a copy.** The race holds a
   handle to its `RunContext` (an index into the stack below, or the context
   itself moved in and back out). RACE AGAIN, NEXT RACE, RESTART and RETURN TO
   GRID read it there; nothing copies `campaign_*` back onto `Session`.
3. **The window is an activity stack.** `Session::stage: Stage` becomes a stack
   of entries, each a `Stage` value plus the `RunContext` it belongs to, if any.
   The top entry is drawn, ticked and receives input. An entry is exited with one
   of four results, and nothing else writes the stack:
   - **push**: a new activity over this one, which stays alive and paused
     (WATCH REPLAY over a finished race; the menus over a paused race; 2048's
     map over the menus);
   - **pop**: back to the entry underneath, which is told it is resumed;
   - **replace**: this entry ends and another takes its place (a race load
     becoming the race; NEXT RACE replacing a finished leg);
   - **unwind to** a named kind: pop until that entry is on top (RETURN TO MENU,
     a network failure back to the room).
4. **A defined pop.** Popping an entry drops it and, if no other entry still
   references its `RunContext`, drops the run. A run's end is therefore one
   place: abandoning a tournament is its run being dropped, on every path,
   including a failed load. The entry underneath gets `resumed()`. Its race and
   run were never dropped, so the end screens need no rebuild, but `resumed()`
   restores the session-wide state the pushed activity changed: race music and
   sound effects, rumble, the window title and the held menu backdrop. **A pop
   back to a finished race does not re-arm its result**: today's `resume_race`
   sets `result_saved = false` and settles poses, which is right for a paused
   unfinished race and would record a finished leg twice.
5. **WATCH REPLAY pushes.** The finished race stays on the stack, end screens
   and run intact. The watch is a new run (its origin is "pop") on a new entry:
   a race load, replaced by the viewer's race. EXIT pops it, and the finished
   leg's end menu is on top again with NEXT RACE. `WatchReplayOffer`'s
   `tournament_next_leg` refusal is removed in the same change. WATCH from the
   REPLAYS page pushes the same way, over the menus, so it no longer drops a
   paused race.
6. **The menu screens stay inside `MenuStage` for now.** The picker, campaign,
   Pure chain, join, LAN, window layout and seat assignment screens keep their
   `Option` fields and `escape`'s test order. Moving them onto the stack is a
   later decision; it is not needed for the acceptance above.

### Migration

Each step lands on its own and leaves the game playable.

1. **Introduce `RunContext`** in `crates/game/src/app/`, holding the launch slots
   listed in decision 1. `Session` keeps one `Option<RunContext>` for the run
   being launched. The race build moves it into the `RaceStage` instead of
   draining five fields; RACE AGAIN, NEXT RACE and RESTART read it from there.
   Demo and benchmark keep the player's run aside by moving it, not by copying
   fields. Existing tests unchanged.
2. **Move `tournament` into `RunContext`.** Abandoning is dropping the run. The
   failed-load case is fixed here, with a test that fails the leg's load and
   checks the next RACE-page race offers no NEXT RACE.
3. **Replace `stage`, `suspended_race`, `parked_menu` and `return_to_replays`
   with the stack** and the four exits. `open_menus` becomes a push (pause) or an
   unwind (quit, finish). `resume_race` becomes a pop. The 2048 map becomes a
   push. The REPLAYS page is the origin of a WATCH run, so `return_to_replays`
   goes.
4. **WATCH REPLAY pushes; lift the Tournament refusal.** Add the acceptance as a
   test: a Tournament run on Pulse, leg 1 finished, WATCH REPLAY, EXIT, the end
   menu shows NEXT RACE, NEXT RACE launches leg 2, and the standings after leg 2
   count leg 1.
5. **Measure the held race.** Two race scenes are resident while a finished leg
   waits under its replay. Measure resident memory on the Steam Deck and on a
   phone before and after step 4 ([memory](../memory.md)). If it does not fit,
   the held entry drops its GPU scene and keeps only the `RaceStage` state the end
   screens need; the end screen's backdrop is then rebuilt on pop.

### What this does not cover

- **Network play.** Leaving a LAN or online room to watch a replay needs its own
  decision; the end-of-race row stays off while `net` is up.
- **Tournament save and resume** across quitting the game (the original's
  `Tournament_SaveProgress` and `_LoadProgress`). A dropped run is still lost.
- **Photo mode** stays an overlay on whatever race is on top, as do pause and
  the end screens.
- **2048's touch strip** has no WATCH REPLAY tile; adding one is a separate UI
  change. Pure draws no end menu and offers no row.
- **The menu screens inside `MenuStage`** (decision 6).
- **The `--race` path**, which builds no menus, has a stack of one.

## Alternatives considered

- **One more slot: hold the finished race in `Session::held_endrace` for the
  viewer.** The smallest change that meets the acceptance. Rejected because it is
  a third one-deep slot of the kind that causes the collisions in the map (a
  paused race and a finished race are two different slots that each work only
  once), and the stale-tournament bug stays.
- **Rebuild the end menu from saved results instead of keeping the race.** Keeps
  memory flat, but the end screens read the race (`results`, slot teams, records,
  the end screen's prefetch) in many places. Kept as the fallback in migration
  step 5, not as the plan.
- **An explicit state machine: one enum of every screen and a transition table.**
  Makes every edge visible, but the map counts dozens of entry points and the
  menu screens alone have nine overlapping states. A stack with four exits covers
  the nesting with far less surface.
- **Only fix the stale tournament** (clear it in the failed-load arm). Worth doing
  on its own if this ADR is not accepted; it does not make WATCH REPLAY possible.

## Consequences

- WATCH REPLAY can be offered mid-Tournament, and a replay watched from REPLAYS
  no longer drops a paused race.
- A run has one place it ends, so the tournament (and every other launch slot)
  cannot outlive its run.
- Two race scenes can be resident at once. That is a real memory cost on the
  Steam Deck and on phones, unmeasured until migration step 5.
- Every site that writes `Session::stage` today changes, and many `match &self.stage`
  readers become "the top of the stack". Steps 1 and 2 are small; step 3 touches
  most of `crates/game/src/app/session/`.
- `escape`'s ordered `if` chain over the menu screens survives (decision 6), so
  there are still two navigation mechanisms until a later ADR moves the menu
  screens.
