# After the finish: the race keeps running under the end-race panels

**Status: measured on Pulse PSP, 2026-10-01 (Time Trial and Single Race, Talon's
Junction White `16_Track`, four captures plus three probe runs).** The craft hand-over,
the running world and the spectator camera (all six views a single-player finish reaches,
and `Race End Photo`'s d-pad) are ported, the `Race End Photo` state for line finishes, and
since 2026-10-04 the finished player's thrust law (see [Open](#open)).

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
| `F+1 .. F+6` | the AI's throttle reads `100` for one to six frames, then **`56.7`** and stays there for most of the log (35 s, a whole lap and more; late in `sr-noinput` it rose to `63.7` and `72.5`, the law's clamp letting go, [below](#the-throttle-is-the-ais-position-balancing-law-run-on-the-players-own-place)); the steer swings `+-10..50` as a follower's does | `*` |
| `F+61` | the camera object (`0x08b32c64`) is given a subject, the player's craft, and node mode `7` | `*` (four captures, to the frame) |
| `F+61` | the front end enters **`Race End Photo`** (state name read every frame on a fresh capture: `InGame` through `F+60`, `Race End Photo` from `F+61`, the same frame the spectator camera starts). The screen is `InGame_Definition.xml`'s, two `Stats`-font texts with nothing behind them: `PRESS SELECT BUTTON FOR PHOTO MODE` over `PRESS ε TO CONTINUE` (ε is the cross button's glyph in that font), left-aligned at x = 20, y = 212 and 242. **The legend fades in linearly from `F+62` to full ink at about `F+104`** (42 frames, 0.7 s; a photograph every second frame, two text lines, two-frame photograph lag allowed for). X leaves for `EndRace Results`, then `Rewards`, then `Menu` | `*` (state flip: four captures bracket it to `F+50..59 -> F+80..89`, the fifth pins it to `F+61`) |
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

### The throttle is the AI's position-balancing law, run on the player's own place

**Found and measured 2026-10-04** ([race-finish.md](../ghidra/functions/psp-pulse-usa/race-finish.md#the-finished-players-thrust-ai_computeopponentthrust-with-the-players-own-rank-2026-10-04),
confidence 90). The finished player's driver writes `100` until the racer record's copy of `finished` is set on `F+1`,
then `AI_ComputeOpponentThrust`, the same law the opponents race on, indexed by the player's own finishing place `r`.
For the finished player the law sits on its lower stop:

```text
thrust = off + (0.7 * AIThrust[r] - AIThrust[1]) * mul + AIThrust[1]
```

`AIThrust` is the race's speed class's `PlayerInPos1..8` row (`AIRaceStats_<class>.xml`), `off` and `mul` the class's
`SkillScale` `ThrustOffset`/`ThrustMultiplier` at the race's skill scale. Venom, Easy, Talon's Junction: skill `0.9`,
`off = -7.7`, so **first place `56.7`, fourth `56.0`**, both read live. It is **rank-dependent, and lower places do not drive
faster**: the player aims about 250 units behind the craft one place ahead and the clamp holds it at 70 % of its place's
`AIThrust`. The opponents' post-finish constants are the same law (`84.3`, `83.3`, `111.9`, `100`).

## What ours does

| Behaviour | State |
| --- | --- |
| The player's craft is flown by `oag-ai`'s driver from the tick the player crosses the line for the last time (`Race::flown_for_the_player` gains `standing.finished()`), put on the piece of racing line it is on at that tick | **ported**, `post_finish_ground_truth` |
| The pace of that driver | **ported 2026-10-04** (`race::finished_thrust`): the driver steers, and its thrust is capped at the original's law for the player's finishing place (`AIRaceStats_<class>.xml` and the race's skill scale off the disc: `56.7` % after a first place at Venom Easy on Talon's Junction). That the law is a **cap** on our driver's own thrust rather than a replacement (our driver keeps its corner lift) is **chosen, not measured**. The law's unsaturated regime (a player more than about 195 units behind its target) is not reproduced. A lone Venom Novice craft now covers `3,423` units in 35 s after the line against the original's `3,500-3,900`, its throttle on the cap on 1,854 of 2,100 ticks (`post_finish_ground_truth`); at the default Elite tier (the original's Hard rung) the cap is higher, `70.4` %. The skill scale's Easy/Medium/Hard rung is ours `Novice`/`Skilled`/`Elite` (and `Ace`, which has no rung, reads Hard); the Single Race and Head to Head mode terms are added as `AI_ResolveSkillScale` adds them, a campaign cell's own scale replaces it. Opponents are untouched. On Pulse PS2 no track `stats.xml` is read, so the skill scale is `2.0` (the original's fallback for a missing table, which PS2 does not lack): **chosen, not measured**, about 64.4 % at every tier. Tournament adds no mode term, as `AI_ResolveSkillScale` reads (`g_game_mode` 4) |
| The world is stepped under the end-race panels (`Race::runs_on_after_the_line`, `Session::frame`, neutral input) and the headless capture runs on past the line | **ported** |
| The standing, best lap, splits and the board stay frozen at the line | already true: `Standing::update` returns once finished, and the board is taken once. The original only records a lap in craft state 1 |
| The world keeps running under the panels | **For the line and for a Single Race wreck** (`Race::runs_on_after_the_end`). A wreck: the player's craft stays down for good, the field races on, the wreck's explosion, fire and shake play as before, and the destroy camera hands over to the spectator director 240 ticks after the wreck call, 210 after the race ended (the wreck's own state-6 timer running out, **measured twice**, below), which follows the field in the death camera's mode 5 for good. **Zone, Eliminator**: not looked at; Zone stands still with cosmetics only (`Race::tick_cosmetics`), Eliminator respawns. Opponents', shots' and trails' state under a Zone wreck stays frozen (**chosen, not measured**) |
| HUD hidden from `F+1` | ours stops drawing the HUD from the tick the race finishes (the end-race flow replaces it), the clean view of `Race End Photo` included; seen in the window |
| The `Race End Photo` state and its legend | **ported 2026-10-02, for a line finish and for a Single Race wreck** (`oag_ui_screens::endrace::photo`, `race_stage::endrace_flow`). Ours holds a clean view for 61 ticks, the HUD already gone, then draws the disc's own two lines (read off `InGame_Definition.xml`'s `Race End Photo`: strings, font role, colour and places are the file's; the `ε` draws as the cross button) fading in over 42 ticks, and waits. **X, Start or a click** leaves for `EndRace Results` once the state is entered (`F+61`; a press before that is spent, so thrust held across the line cannot skip it). **SELECT does nothing**: photo mode is not built, and the line promising it is drawn anyway because the disc authors it. The `Stats` face is the `menu` atlas scaled (no `Pulse_14.fnt` atlas is loaded), narrowed by 0.90 to match the capture's line width (**chosen, not measured**: width is matched, glyph height is then near it - 9 pixel rows against 8 on the first line before the narrowing - and the narrowed text was not re-photographed in English). The fade's length is measured; why it is 0.7 s is unread. **For a wreck the original has no such state** (below); ours shows it after the wreck on the line's own timing, **chosen, not measured**. **Wipeout HD/Fury, Eliminator and Zone endings are unchanged**: the panels come up at once. Seen in a window (Xvfb, llvmpipe, release): `sheet_race_end3.png` and `cmp_legend_crop.png` (both **before** the 0.90 narrowing), `sheet_all.png` and `ours2_full.png` (after, in French); a wreck: `sheet_manual3.png` |
| Seen in the window | **walked live 2026-10-01** (Xvfb, llvmpipe, release build, isolated profile, `--autopilot --no-audio`; `RACEBOX`, Time Trial, Venom, Basilico Black, 3 laps, finished `1.44.78`): under `EndRace Results` the scene keeps moving and cuts between the chase view and trackside views from frame to frame; Cross (Enter) went on to `EndRace Menu` (a Time Trial earns no Rewards page) with the scene still moving behind it and the craft unaffected; a second Cross left for the main menu. Frames: `f-0123.png`, `f-4to9.png`, `rewards-menu.png` (gitignored) |
| The spectator camera | **ported, all four views** (`race::finish_camera`, `oag_render::camera::craft_view`): starts on `F+61`, follows the player, re-picks the subject every 600 frames, cuts by the 60-unit rule and the 26/25/25/24 mode roll. Modes `6`/`7` sit on the circuit's authored `Camera` nodes; **mode `2` is a rigid rear view** (6 behind, 2.5 above the craft, looking the way it flies) **and mode `3` a rigid front view** (12 ahead, 3 above, looking back), both at a fixed 65 degrees: read from the decompile and **measured** on PPSSPP (407 frames, rotation exact, eye to `6e-5`, [camera.md](../ghidra/functions/psp-pulse-usa/camera.md#the-spectator-views-craft-relative-modes-and-the-directors-hand-off-rules-2026-10-02)). The picture follows the *previous* subject (`cam+0x1e4`), which takes the subject's place only at a cut. The random stream is the director's own, so it matches the distribution, not the frames. Pulse PSP only. Seen in a window (headless, `OAG_SCRATCH_NO_BOARD` frames, English): `seqA.png`, `seqB.png` |
| The d-pad on `Race End Photo` | **ported 2026-10-04** (`FinishCamera::cycle`, `watch_step`, `Race::spectator_press`, wired in `Session::tick_endrace` once the screen is entered): up and down step the camera through `1 -> 4 -> 3 -> 2 -> 7 -> 1` keeping the view width, left and right watch the previous and next grid slot from its nearest node, as `RaceManager_Update` does (read, button mapping measured live). Modes **`1`, a nose view** (5 ahead, looking forward) and **`4`, a far chase view** (12 behind, 3 above), 65 degrees, measured (379 frames to `5.4e-5`). Line finishes only: the original has no photo state after a wreck, so ours' chosen wreck legend takes no d-pad. **Wired, not seen through the front end**: `Race::spectator_press` is tested, its call in `Session::tick_endrace` is not (no session-level harness drives the EndRace flow). Not reproduced: the original's subject and drawn craft drifting apart after a left/right press (race-finish.md). Pointer: none (the original's screen takes only buttons). Frames (headless, the mode forced by a scratch hack, uncommitted): `nose-9000.png`, `chase-9150.png` against the original's `cam14/v0100.png`, `v0300.png` |
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
  different width), `2` and `3` are the craft-relative views: a rigid rear view (6 behind, 2.5 above) and a
  rigid front view (12 ahead, 3 above, looking back), 65 degrees, read and measured 2026-10-02.
- A node camera looks at a smoothed copy of the subject's position (rate `0.4 / 0.5 / 0.6 / 0.6` a frame for Venom / Flash / Rapier / Phantom, `0.3` by default) and zooms its field of view toward `2 atan(width / 2 / distance)` with `width` `50` (mode `7`),
  `17` (mode `6`), `35` (mode `5`).
- The sequence is random in the original (C `rand`), so a port can match the **distribution**, not the
  frames.

## A player wreck never reaches `Race End Photo`

**Measured 2026-10-02 (Pulse PSP, PPSSPP software renderer, Single Race, Talon's Junction, `Ship_SetState(player, 4)` injected 330 frames after GO).**
`manager+0x7c8` goes `2 -> 3` and the HUD hides 31 frames after the call (state 5 begins at 30), exactly as for a finish. The front end then **stays on
`InGame`**: read every frame to `k+660`, then once a second to about `k+17,800` (4.9 minutes). No legend, no panel, the
spectator camera cutting between the field, the other seven craft racing on, the wrecked craft in state 6 with its timer
running negative. A finish flips to `Race End Photo` 60 frames after mode state 3; a wreck does not. What the line has and a wreck lacks
(the player's `finished` flag, craft state 2, a `Race_BuildEndRaceResult` the front end waits on) is **unread**. The wreck was
injected through `Ship_SetState`, not drained to zero shield, so a real kill might differ; the mode state going to 3 on the
same frame rule suggests not. `log.json`, `wreck-watch/log.json`.

**The camera (`wreck2/log.json`, then `wreckA` of 2026-10-02 with the director's own fields read every frame):** the destroy camera (mode 5) keeps the
player as the subject and the drawn craft, on the same node. **What ends it is the wreck's own timer, not a fixed delay**: state 4 lasts 30 frames, state 5
90 and state 6 120, and `entity+0x874` (the state timer) crosses zero at **`k+240`**, the same frame `Camera_UpdateSpectator` clears the subject and
`Camera_PickSubject` takes another live craft. The picture changes at the next cut, when the new subject is more than 60 units from the node's aim
point and some node is within 60 of it: **`k+279`** in the 2026-10-02 run, **`k+290`** in the first (the "259 frames after the mode state went to 3"
the previous lane quoted is that second number, one sample of a geometry-dependent delay). **The mode stays 5 for good because `Ship_SetState` case 4
clears `cam+0x26c`, the flag `Camera_PickRandomMode` tests**, so every cut after a wreck is a node camera of view width 35
([camera.md](../ghidra/functions/psp-pulse-usa/camera.md#after-a-player-wreck-the-camera-stays-in-mode-5-for-good-confidence-92)).

### What ours does for a wreck (maintainer decision)

The maintainer decided on 2026-10-02: **"hold, then results"** - after a player wreck in a Single Race the race keeps running in spectator view with
the HUD hidden, as the original does, then the same `Race End Photo` legend and X/pointer continue lead to `EndRace Results`. Two parts are
**chosen, not measured**: the clean-view delay before the legend (the line finish's measured `F+61` and 42-tick fade, reused) and that a wreck shows
the legend at all. Built: the world runs on (`Race::runs_on_after_the_wreck`), the director takes over from the destroy camera 240 ticks after the wreck call, 210 after the race ended (the wreck's state-6 timer, measured
twice) in mode 5 with the wreck still drawn until the first cut, the legend comes up 61 ticks after the race ended - over the explosion, which goes off at 90 ticks - and X leaves for the panels. Zone and
Eliminator are not part of the decision and keep their old endings. Seen in a window: `sheet_manual3.png`.

## Open

- **A player's craft shot down after the finish.** Ours: an AI-flown craft that is destroyed is a wreck that never respawns (the world runs on, the race stays finished), and the spectator camera is checked before the destroy camera in `Race::view_unshaken`, so it stays on its node view where the original would be expected to switch to mode 5. Unmeasured.
- **The engine voice** spins down under the panels while the craft is still flying (about 98 u/s at Novice, 112 at Elite since the thrust cap): audio after the flag was not measured, so ours keeps its pre-existing finished-race behaviour.
- The finished player's thrust **when the clamp lets go** (more than about 195 units behind its target): it needs the
  opponents' `spread` values (`FUN_08852ef4`), which this port does not model.
- **What ends a wrecked player's race in the original** ([above](#a-player-wreck-never-reaches-race-end-photo)): unread; ours leaves by the legend
  and the panels as the maintainer decided. The Eliminator and Zone endings were not looked at; ours puts the panels up at once for both.
- **Photo mode** (`SELECT` on `Race End Photo`): not built; `Race End Photo Redirect`'s `forward="select"` goes to `InGame Photo`.
  Both the redirect and `PhotoMessage` carry `GSDisable="1"` (off under Game Share, which this port has no equivalent of).
- Why the legend fades over 0.7 s: the screen authors no `Transition`. Whether the original reads a press before `F+61`
  is moot, the state does not exist yet.
- ~~What drives `cam+0x274`, cases `0`, `1`, `4`, `8`~~ **Closed 2026-10-04**: `1` and `4` ported (the d-pad above); `+0x274` and `8` belong to the multiplayer GriefReport path and `0` has no writer, so none is reachable after a single-player finish ([race-finish.md](../ghidra/functions/psp-pulse-usa/race-finish.md#race-end-photos-d-pad-modes-1-and-4-and-the-unreachable-cases-2026-10-04)). Whether a barrel roll is inside the craft matrix the craft views ride on: not seen.
- The camera nodes are checked against the original's own runtime list on `16_Track` only (ten nodes, eye and aim, to 0.1 unit); the other circuits' lists were not read live.
- Audio after the flag; the other endings (wreck, Eliminator, Zone); whether Pure, HD/Fury and 2048 do the
  same (not checked: Pulse was the priority).
- `Race_FinishAllCrafts` (`0x08824e10`) is **not** this path: see the correction in
  [`grid.md`](../ghidra/functions/psp-pulse-usa/grid.md#correction-2026-10-01-the-finished-craft-is-not-handed-over-by-race_finishallcrafts).

## Method

`scripts/psp-postrace.py` (`--state-every 1` reads the front end's state name every frame; `psp-wreck-capture.py --ui-state` does the same
and adds the mode state): restart the race, hold thrust through the countdown, give the player's weapon
record the Autopilot pickup (fire bit `0x1000`) with its timer raised so the original's own autopilot
drives the laps, `--laps-hack 1` to shorten the race (a mid-race write to `g_race_laps`, said so in the
log), then break in `Weapons_DispatchFire` once a frame and log every craft, the manager, the HUD flag and
the camera object, with photographs. `--disarm-x -60` stops the pickup and releases thrust 43 frames before
the line so that **nothing but the game's own post-finish driver is left**. `--probe setstate` breaks on
`Ship_SetState`; `--probe ctl` and `--probe scale` stop on writes to the control record and to
`craft+0x1d4`. Captures: `log.json`
and the photographs beside them (gitignored).

| Run | What | Result |
| --- | --- | --- |
| `sr-test` | Single Race, pickup autopilot throughout | finish `F`; craft laps on for 2,100 frames; opponents finish `F+107..F+375` |
| `sr-noinput` | Single Race, pickup off and pad released 43 frames before the line | same, control record zero until `F`, the AI's from `F+1` |
| `tt-noinput` | Time Trial, the same | same, `56.7` from `F+2` (no place-2 craft: the law's `ref` is missing, `gap` is minus the craft's own progress, the lower stop) |
| `tt-stock` | Time Trial, **stock three laps**, the same | finish on the fourth crossing; `craft+0x1d4` `0.0 -> 1.0` on `F`; `56.7` from `F+7`; 3,519 units in 35 s |
| `probe-setstate` | `Ship_SetState` calls | every `(entity, 2)` comes from `0x08841ab8`, the player's first |
| `probe-scale` | writes to `craft+0x1d4` | `0x08848664` from `0x08841b28` (every frame) and `0x0883bf34` (`PlayerStatus_Update`) |
| `sr-back` (2026-10-04) | Single Race, the player left on the grid for 300 frames after GO (`--hold-back`), then the pickup | finishes **4th**; thrust `100` to `F+1`, `56.0` from `F+2`; the racer records and the player's driver logged every frame |
| `probe-ctl` | writes to the pad record | `PlayerInput_Update` (`0x0883c870`) at its three stores, a frame after the flag the pad record is no longer what the craft reads |

Single boots each for the probes; the four timeline captures agree with each other on every starred
row. Not run: a second circuit, a second speed class or difficulty.
