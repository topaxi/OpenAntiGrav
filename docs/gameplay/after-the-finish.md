# After the finish: the race keeps running under the end-race panels

**Status: measured on Pulse PSP, 2026-10-01 (Time Trial and Single Race, Talon's
Junction White `16_Track`, four captures plus three probe runs).** The craft hand-over,
the running world and the spectator camera's node cameras are ported; two of its four
views and the `Race End Photo` state are not (see [Open](#open)).

The maintainer's play report said it first: once a race is over the race keeps playing in
the background with the player's craft driven by AI, and nobody knew what the camera does
there. The discriminating question was whether the craft keeps moving with no input and
whether its control word is the AI's. It was **falsifiable** (a craft that stops or coasts to
rest would have ended the thread) and it **did not fall**.

Evidence for the code behind each row is
[`race-finish.md`](../ghidra/functions/psp-pulse-usa/race-finish.md); the panels drawn over the
race are [`endrace-screens.md`](../ui/endrace-screens.md).

## What the original does, frame by frame

`F` is the frame the player's `entity+0x912` (finished) reads 1. All four captures agree to
the frame on every row marked `*`, except the throttle step, which reads `F+2`, `F+4`, `F+5` and `F+7`.

| Frame | What happens | Seen |
| --- | --- | --- |
| `F` | `Craft_UpdateLapProgress` sets `finished`; the same frame `FUN_088418e0` calls `Ship_SetState(entity, 2)` on a craft in state 1 (`craft+0x2a4` reads 2). A craft does this at **its own** crossing, so the field finishes one by one | `*` |
| `F+1` | the race manager's mode state goes `2 -> 3` (`ArcadeRace_UpdateRacing`: `Hud_Hide`, `Race_BuildEndRaceResult`, `RaceMode_SetState(3)`) and the **HUD is hidden** (`g_hud+0x168 = 1`). The craft's control record pointer (`craft+0x78`) moves from the pad's record to the blend buffer at `craft+0x44`: the AI now flies it | `*` |
| `F+1 .. F+6` | the AI's throttle reads `100` for one to six frames, then **`56.7`** and stays there for the rest of the log (35 s, a whole lap and more); the steer swings `+-10..50` as a follower's does | `*` |
| `F+61` | the camera object (`0x08b32c64`) is given a subject, the player's craft, and node mode `7` | `*` (four captures, to the frame) |
| about `F+61..F+89` | the front end enters **`Race End Photo`**: the legend `PRESS SELECT BUTTON FOR PHOTO MODE / PRESS X TO CONTINUE` is drawn over the running race; X leaves for `EndRace Results`, then `Rewards`, then `Menu` | state name sampled every 30 frames |
| every 600 frames | the subject is re-picked: a random live craft other than the previous one (`F+661`, `F+1261`, ...). Time Trial has one craft, so it stays the player | `*` |
| at random | a **cut**: the camera node and mode change (modes `3`, `2`, `6` and `7` all seen) | |

**The craft keeps moving with no input at all.** The decisive run released the pad and switched
the pickup autopilot off 43 frames before the line (control record `0, 0` from then to the line, speed
falling `142 -> 94`), and from `F+1` the record is the AI's. Over the 35 s logged the craft covered
`3,500-3,900` units (35.0 s, 2,099 frames), round Talon's Junction's far bend and back past the line, at
`89-110` u/s with excursions to `118-154` over speed pads. The Time Trial runs do the same with a lone craft, one
of them a **stock three-lap race** with no lap shortcut (finish on the fourth crossing, `3,519` units).

**The whole field keeps lapping.** Every opponent goes to state 2 at its own crossing (`F+107 ..
F+375` in the Single Race captures, all eight inside 6.3 s) and every craft goes on driving. Nobody
stops.

**The driver does not change.** `craft+0x3c` (the pad's record) reads the same pointer before and after
the finish on every craft, and `craft+0x40` (the AI's) never changes either. What changes is `craft+0x1d4`, an autopilot weight (`Craft_SetAutopilotBlend`)
that reads `0.0` through the 40 frames the pad flew the craft and **`1.0` on frame `F`**, after which
`Ship_UpdateCraft` copies the AI's record (`craft+0x40`) over the blend buffer. The same mechanism flies the Autopilot pickup, with the weight
running down in the pickup's last second.

### The throttle is not the AI's racing throttle, and its source is not found

The finished player's throttle is a constant `56.7` (percent of full) against `100` for the Autopilot
pickup on the same circuit and `75-112` for opponents. Opponents' own post-finish values are
constants too (`84.3`, `100`, `111.9`), which reads as a per-craft, rank-balanced value
(`AI_ComputeOpponentThrust`, player-coupled). Whether `56.7` is the rank-1 value, the player's AI
tuning or something else is **open**: two captures are both rank 1 (a Time Trial, and a Single Race the
pickup won). Nothing was ported from it; see [what ours does](#what-ours-does).

## What ours does

| Behaviour | State |
| --- | --- |
| The player's craft is flown by `oag-ai`'s driver from the tick the player crosses the line for the last time (`Race::flown_for_the_player` gains `standing.finished()`), put on the piece of racing line it is on at that tick | **ported**, `post_finish_ground_truth` |
| The pace of that driver | **chosen, not measured**: the AI's own pace under the player's physics (the maintainer's standing rule). The original's `0.567` of full thrust is not reproduced, so ours cruises at about `125-135` u/s against `92-110` |
| The world is stepped under the end-race panels (`Race::runs_on_after_the_line`, `Session::frame`, neutral input) and the headless capture runs on past the line | **ported** |
| The standing, best lap, splits and the board stay frozen at the line | already true: `Standing::update` returns once finished, and the board is taken once. The original only records a lap in craft state 1 |
| Only the **line** keeps the world running | **chosen**: a wreck, an Eliminator target or a Zone run does not step the world (the original was not measured there). **Since 2026-10-02 the cosmetics of such a finish do step** (`Race::tick_cosmetics`): a Single Race or Zone wreck plays out under the panels - the state-5 shake, the fire, the big explosion 1.5 s on with its ring and wash, the destroy camera's ease. The opponents, shots in flight and trails stay frozen under it (**chosen, not measured**). Seen for a Single Race and a Zone wreck; the panel is drawn over it at once, where the original's wreck frames carry none for the first 120 frames checked (the `Race End Photo` gap above) |
| HUD hidden from `F+1` | ours draws the results panels over the scene at once, which hides the HUD; not separately measured |
| The `Race End Photo` state and its legend | **not built**: ours goes straight to `EndRace Results` |
| Seen in the window | **walked live 2026-10-01** (Xvfb, llvmpipe, release build, isolated profile, `--autopilot --no-audio`; `RACEBOX`, Time Trial, Venom, Basilico Black, 3 laps, finished `1.44.78`): under `EndRace Results` the scene keeps moving and cuts between the chase view and trackside views from frame to frame; Cross (Enter) went on to `EndRace Menu` (a Time Trial earns no Rewards page) with the scene still moving behind it and the craft unaffected; a second Cross left for the main menu. Frames: `data/scratch/pulse-postrace/shots/f-0123.png`, `f-4to9.png`, `rewards-menu.png` (gitignored) |
| The spectator camera | **ported for the node views** (`race::finish_camera`): starts on `F+61`, follows the player, re-picks the subject every 600 frames, cuts by the 60-unit rule and the 26/25/25/24 mode roll, modes `6`/`7` sit on the circuit's authored `Camera` nodes. Modes `2` and `3` (`above`, `front`) are **chosen, not measured**: the chase camera stands in, so about half the cuts leave it. The random stream is the director's own, so it matches the distribution, not the frames. Pulse PSP only |
| Audio under the panels | **unmeasured** (the emulator was muted); ours spins the engine down as before |

## The camera, as far as it is read

Ported for modes `6` and `7`; recorded in full in
[`race-finish.md`](../ghidra/functions/psp-pulse-usa/race-finish.md#the-spectator-camera). The shape,
from three captures and a decompile:

- `F .. F+60`: the player's own chase camera, unchanged. At `F+61` the spectator director starts.
- **Subject**: the player at first; every 600 frames a random live craft that is not the previous one.
- **Node cuts**: the camera sits on one of the circuit's authored `Camera` nodes and stays until the subject
  is more than 60 units from it, when it takes a random node within 60 units of the subject and rolls a
  mode: `3` 26 %, `2` 25 %, `6` 25 %, `7` 24 % (`rand() % 100` against `26 / 51 / 76`). Modes `5`, `6`, `7` are
  the node-mounted cameras (they share one update; the death camera, mode 5, is the same code with a
  different width), `2` and `3` are craft-relative views whose geometry was not read.
- A node camera looks at a smoothed copy of the subject's position (rate `0.4 / 0.5 / 0.6 / 0.6` a frame for Venom / Flash / Rapier / Phantom, `0.3` by default) and zooms its field of view toward `2 atan(width / 2 / distance)` with `width` `50` (mode `7`),
  `17` (mode `6`), `35` (mode `5`).
- The sequence is random in the original (C `rand`), so a port can match the **distribution**, not the
  frames.

## Open

- **A player's craft shot down after the finish.** Ours: an AI-flown craft that is destroyed is a wreck that never respawns (the world runs on, the race stays finished), and the spectator camera is checked before the destroy camera in `Race::view_unshaken`, so it stays on its node view where the original would be expected to switch to mode 5. Unmeasured.
- **The engine voice** spins down under the panels while the craft is still flying at about 130 u/s: audio after the flag was not measured, so ours keeps its pre-existing finished-race behaviour.
- What produces the `56.7` throttle, and whether it is rank-dependent. Measure a Single Race the player
  finishes **not** first.
- The `Race End Photo` state (about one second of clean view, then a legend that waits for X before the panels).
  Ours shows the panels at once.
- Modes `2` and `3` (`above`, `front`), whose geometry is in the `FUN_08880c04` cases not read; what drives `cam+0x274`.
- The camera nodes are checked against the original's own runtime list on `16_Track` only (ten nodes, eye and aim, to 0.1 unit); the other circuits' lists were not read live.
- Audio after the flag; the other endings (wreck, Eliminator, Zone); whether Pure, HD/Fury and 2048 do the
  same (not checked: Pulse was the priority).
- `Race_FinishAllCrafts` (`0x08824e10`) is **not** this path: see the correction in
  [`grid.md`](../ghidra/functions/psp-pulse-usa/grid.md#correction-2026-10-01-the-finished-craft-is-not-handed-over-by-race_finishallcrafts).

## Method

`scripts/psp-postrace.py`: restart the race, hold thrust through the countdown, give the player's weapon
record the Autopilot pickup (fire bit `0x1000`) with its timer raised so the original's own autopilot
drives the laps, `--laps-hack 1` to shorten the race (a mid-race write to `g_race_laps`, said so in the
log), then break in `Weapons_DispatchFire` once a frame and log every craft, the manager, the HUD flag and
the camera object, with photographs. `--disarm-x -60` stops the pickup and releases thrust 43 frames before
the line so that **nothing but the game's own post-finish driver is left**. `--probe setstate` breaks on
`Ship_SetState`; `--probe ctl` and `--probe scale` stop on writes to the control record and to
`craft+0x1d4`. Captures: `data/scratch/pulse-postrace/{sr-test,sr-noinput,tt-noinput,probe-*}/log.json`
and the photographs beside them (gitignored).

| Run | What | Result |
| --- | --- | --- |
| `sr-test` | Single Race, pickup autopilot throughout | finish `F`; craft laps on for 2,100 frames; opponents finish `F+107..F+375` |
| `sr-noinput` | Single Race, pickup off and pad released 43 frames before the line | same, control record zero until `F`, the AI's from `F+1` |
| `tt-noinput` | Time Trial, the same | same, `56.7` from `F+2` |
| `tt-stock` | Time Trial, **stock three laps**, the same | finish on the fourth crossing; `craft+0x1d4` `0.0 -> 1.0` on `F`; `56.7` from `F+7`; 3,519 units in 35 s |
| `probe-setstate` | `Ship_SetState` calls | every `(entity, 2)` comes from `0x08841ab8`, the player's first |
| `probe-scale` | writes to `craft+0x1d4` | `0x08848664` from `0x08841b28` (every frame) and `0x0883bf34` (`PlayerStatus_Update`) |
| `probe-ctl` | writes to the pad record | `PlayerInput_Update` (`0x0883c870`) at its three stores, a frame after the flag the pad record is no longer what the craft reads |

Single boots each for the probes; the four timeline captures agree with each other on every starred
row. Not run: a second circuit, a Single Race the player finishes other than first.
