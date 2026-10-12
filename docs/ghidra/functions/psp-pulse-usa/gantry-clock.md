# The start gantry's clock after `GO`, and the free Turbo's moment

**2026-10-04, `pulse-start-leftovers` lane.** Binary `BOOT.BIN` of
`pulse-psp-usa.chd` (Ghidra `/pulse/BOOT-psp-pulse-usa.BIN`), read on the GUI
bridge. Static reads, matched against the 2026-09-30 PPSSPP capture of a
stationary craft (`docs/rendering/start-gantry.md`, "What the original does
after `GO`"). HD's race manager runs the same law with its own numbers
([`ps3-hdfury-eu/gantry-clock.md`](../ps3-hdfury-eu/gantry-clock.md)); the two
were read independently, HD's first.

**Answer.** The gantry's animation time is the `Mesh` node's `+0x40` under
the slot-8 billboard's model. Three writers set it, and nothing else does:

1. The intro's last substate sets it to **0.0** as it enters the countdown, on
   the same call that plays `ready` (`RaceMode_UpdateIntro_q`, `0x08829e6c`).
2. `RaceMode_UpdateCountdown` (`0x088274b4`) sets it to **3.0** every frame the
   countdown's elapsed time is past 3.0, which is the release frame.
3. `RaceManager_Update` (`0x08829778`), from race state 2 on, keeps it inside a
   window picked by the player craft's **crossing count** `craft+0xac8`,
   calling set-time with the window's start whenever it reads outside:

| Condition (`n = craft+0xac8`, `laps = *0x08b30f9c`) | Window `[from, to)` s | Floats | Bits |
| --- | --- | --- | --- |
| `n == 0` (before the first line crossing) | `[3.2, 5.5)` | `0x08a7a49c`, `0x08a7a4a0` | `0x404ccccd`, `0x40b00000` |
| `n == laps - 1` | `[9.5, 12.0)` | `0x08a7a4ac`, `0x08a7a4b0` | `0x41180000`, `0x41400000` |
| `n == laps` (also plays `FINAL_LAP` once) | `[12.4, 13.3)` | `0x08a7a4b4`, `0x08a7a4b8` | `0x41466666`, `0x4154cccd` |
| any other `n` | `[6.0, 9.0)` | `0x08a7a4a4`, `0x08a7a4a8` | `0x40c00000`, `0x41100000` |

The release's 3.0 is the float at `0x08a7a498` (`0x40400000`), the word before
the table. **Confidence 85** for the law: every link read in the decompile and
the disassembly, the constants read as bit patterns, two independent sites
agreeing on the field (below), and it reproduces both things the capture
measured without being fitted to either.

## The names

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x08829778` | `RaceManager_Update` | 82 |
| `0x089128ac` | `Model_GetAnimTime` | 85 |
| `0x08912890` | `Model_SetAnimTime` | 85 |
| `0x0881a3b8` | `Hud_SetCountdownWidgetTime` | 75 |
| `0x0882dc60` | `TimeTrial_Update` | 80 |
| `0x0882ddd8` | `TimeTrial_UpdateRacing` | 80 |
| `0x08b323c0` (data) | `Billboard_Slots` | 80 |

## The chain, link by link

### 1. Which object: `manager+0x7b4` is billboard slot 8

`RaceManager_Construct` (`0x08829124`) clears `manager+0x7b4` at `0x08829410`,
then, unless the mode is Zone (`g_game_mode != 6` or the debug override set,
`0x08829414`-`0x08829430`), loads `*(0x08b323c0 + 0x20)` and stores it there
(`0x08829438`-`0x08829448`), then calls `0x08900d8c(obj, 1)`. A byte search for
every `sw rt, 0x7b4(rs)` in the image (`b4 07 ?? ae`) finds only those two
stores.

`0x08b323c0` is the billboard slot array: `billboards.md` reads both
billboard constructors indexing a 9-entry global array by `num` at
`DAT_00058c28`, an unrelocated `.bss` address, and `.bss` loads at segment 1's
base `+0x08ad9798` ([`anim-transform.md`](anim-transform.md)):
`0x00058c28 + 0x08ad9798 = 0x08b323c0`. `+0x20` is index 8, the start gantry
(`TrackStartup.xml`'s slot 8, `321Go_StartFinish.vex`). The billboard keeps its
loaded model at `+0x3c` ([`billboards.md`](billboards.md)), which is the
pointer every call below passes. Zone gets no gantry clock at all, which is
what its own circuits' missing gantry predicts. Two pages had this array as
something else: [`race-progress.md`](race-progress.md) called the slot-8 load
"the skybox object (`DAT_08b323e0`)", and [`mesh-draw.md`](mesh-draw.md)'s
`DAT_08b323c0` table read by a `>> 2` index is the same address; whether that
render-pass table is this array was not checked.

### 2. Which field: the `Mesh` node's `+0x40`

`Model_GetAnimTime` (`0x089128ac`) builds a type query whose class is the
return of `jal 0x08a6bd48` (`0x089128cc`) - `Mesh`'s class-identity function,
pinned by `Mesh_Register` storing the same tag ([texture-animation.md's
correction](texture-animation.md#correction-2026-08-18-which-three-classes-the-walker-dispatches)) -
finds the first such node with `0x08911dd0`, and returns `lwc1 f0, 0x40(node)`
(`0x089128fc`). `Mesh_SetAnimTime` (`0x0890e240`) stores its argument to the
same `mesh+0x40`, and `Mesh_UpdateTextureTransforms` gates the board's texture
walk on it. `Model_SetAnimTime` (`0x08912890`) is the thin wrapper that
tail-calls `Node_SetAnimTimeTree` (`0x089114fc`), which dispatches that
setter (and `AnimTransform_SetAnimTime`) over the whole tree: the board's UV
offset and its nodes' motion move together.

### 3. The writers

`Model_SetAnimTime` has seventeen callers. The three on the gantry:

```c
/* RaceMode_UpdateIntro_q (0x08829e6c), intro substate 4, done */
if (manager->gantry) {                         // +0x7b4
    0x08900d8c(manager->gantry, 0);
    Model_SetAnimTime(0.0, manager->gantry->model);   // +0x3c
}
Hud_SetCountdownWidgetTime(0.0, g_hud);
RaceMode_SetState(manager, 1);                 // `ready` (countdown-voice.md)

/* RaceMode_UpdateCountdown (0x088274b4), state 1, every frame */
if (3.0 < manager->state_time) {               // +0x7c0; 0x08a7a498
    if (manager->gantry) Model_SetAnimTime(3.0, manager->gantry->model);
    Hud_SetCountdownWidgetTime(3.0, g_hud);
}
if ((int)whole_seconds_left == 0) { Race_StartRacing(...); RaceMode_SetState(manager, 2); /* `go` */ }

/* RaceManager_Update (0x08829778), every frame */
if (manager->state >= 2) {                     // +0x7c8
    manager->race_time += dt;                  // +0x2b8
    if (manager->gantry) {
        t = Model_GetAnimTime(manager->gantry->model);
        n = manager->player->crossings;        // +0x2c0 -> +0xac8
        if (n == 0)              { if (t < 3.2  || t >= 5.5)  { Model_SetAnimTime(3.2, ...);  Hud_SetCountdownWidgetTime(3.2, g_hud); } }
        else if (n == laps - 1)  { if (t < 9.5  || t >= 12.0) Model_SetAnimTime(9.5, ...); }
        else if (n == laps)      { if (t < 12.4 || t >= 13.3) Model_SetAnimTime(12.4, ...);
                                   if (!manager->final_lap_said && manager->state == 2) { play FINAL_LAP; manager->final_lap_said = 1; } }
        else                     { if (t < 6.0  || t >= 9.0)  Model_SetAnimTime(6.0, ...); }
    }
}
```

The window block is `0x088299e8`-`0x08829ba4`; each bound is an `lwc1` off
`0x08a80000` (`-0x5b64` = `0x08a7a49c` and so on) followed by `c.le.s` and a
`bc1f`, so the test is `from <= t && t < to` stays, else reset. The order of
the tests is `n == 0`, `n == laps - 1`, `n == laps`, else, all equalities
(`bne`), so there is **no finish clause**: past the finish the count is
`laps + 1` and the else branch shows the between-laps board again. HD's
selection takes the finish to the last window; Pulse's does not.

`craft+0xac8` is the crossing count `wraps + past_line`
([race-progress.md](race-progress.md)): its seed gives `wraps = -1` with
`past = 1` past the line and `0` with `0` behind it, so it is **0 on the grid
on either side of the line** and becomes 1 on the first forward crossing.

`RaceManager_Update` is called first thing by every mode's update -
`ArcadeRace_Update`, `Zone_UpdateState`, `TimeTrial_Update` and six more -
which is the name's basis; 82 rather than higher because the rest of its
body (positions, the replay camera's input, the `RACE_COMPLETE` cues) is read
only at branch level.

### 4. What it predicts, against the capture

- **`GO` on the release.** The countdown's free run puts the board at frame
  180 on the release tick (the measured 92 + 180 = 272 in this project's
  ticks), the countdown's own write pins 3.0 there, and the next race-manager
  frame reads `3.0 + dt`, outside `[3.2, 5.5)`, and jumps to 3.2: frame 192,
  past the `u` step at 181. Green one tick after the release is what four
  captures measured (start-gantry.md).
- **`GO` held with no `Board`.** A craft that never crosses stays in
  `[3.2, 5.5)` forever: frames 192 to 330, a 138-tick (2.3 s) loop. The `Board`
  teleport is at 350, never reached; no digit is replayed. That is the 21 s
  stationary capture.
- **The loop's own seam, seen in the capture.** The 2026-09-30 contact sheet
  (`sheet2.png`, one frame per ~0.15 s of HUD
  clock) shows, among the strobing `GO` frames, one frame at **0.02.3** of race
  clock with a dim `3 2 1` ghost on the green board. 2.3 s after the release is
  exactly one window period, and frame 192 is the frame that ghost belongs to
  (this project's render of frame 192 shows the same ghost). The chosen
  216..349 loop this replaces has no such frame. Not a watchpoint, so it does
  not raise the score on its own; it is a prediction the capture happened to
  contain.

The `Board` (between laps), `FINAL LAP` (the lap before the last) and the
chequered state (the last lap) were not watched on the original when this
section was written; section 4b below does, with a forced crossing count.

### 4b. Read live on PPSSPP (2026-10-12, `pulse-race-start`): the law goes from 85 to 90

`scripts/psp-gantry-clock.py` breaks on the `lwc1 f0, 0x40(s0)` at `0x089128fc`
inside `Model_GetAnimTime` (`s0` is the `Mesh` node, the field is not loaded
yet, so `node+0x40` is exactly what `RaceManager_Update` compares) and logs
one row per race-manager call: the node (`0x09839ec0`, the same on every
call), the caller (`ra` saved at `sp+0x48` = `0x08829a04`, the return into
`RaceManager_Update`), `mesh+0x40`, and the player's `craft+0xac8`. The
manager is the caller's `s0` saved at `sp+0x40`; its `+0x2c0` is the player,
which is **not** the craft the first `Ship_UpdateCraft` hit names in a Time
Trial (`0x09a0c030` against `0x09a0cff0`), so a write to the wrong one reads
as "the law is refuted" - it nearly did. Time Trial, Talon's Junction, VENOM,
craft not driven, `pulse-psp-usa.chd`, PPSSPP v1.20.4.

Natural run, `n == 0`, 300 consecutive manager calls: `3.0` on the first call
(the release's write), `3.2` on the second, then `3.2168`, `3.2335`, ... in
steps of `0.01668` s (the emulator's 59.94 Hz frame), and the reset to `3.2`
on calls 140 and 279, **139 calls apart**. That is the predicted
`[3.2, 5.5)` loop; its period is 139 manager calls, one more than the 138
ticks `ReleaseWindow::period` counts, because a reset reads `3.2` twice
(the call that resets, then the next call, before the first advance). Whether
that extra read is a drawn frame depends on whether the board is drawn before
or after the manager's call in a frame, which was not read: it is the one tick
of phase uncertainty `clock.rs` already carries, not a new difference.

Forced crossing count: the craft was not flown across a lap line. The
player's `craft+0xac8` is recomputed every tick (a single write holds for one
manager call), so the script rewrites it before every manager call in a
phase, with the CPU stopped at the breakpoint. Every phase jumps on its first
call to the predicted window start and then loops inside the predicted window:

| Forced `n` | Window start the first call writes | Loop period (manager calls) | Predicted window |
| --- | --- | ---: | --- |
| 1 | `6.0` (from a read of 3.55) | 181 | `[6.0, 9.0)`, 3.0 s |
| `laps - 1` = 2 | `9.5` (from 7.97) | 151 | `[9.5, 12.0)`, 2.5 s |
| `laps` = 3 | `12.4` (from 11.97) | 55 | `[12.4, 13.3)`, 0.9 s |
| `laps + 1` = 4 | `6.0` (from 12.80) | 181 | the `else` window: **no finish clause** |

The `laps + 1` row is the one a copy of HD's selection would get wrong (HD
takes the finish to the last window): the original shows the between-laps
`Board` again past the finish. `laps` read `3` (`*0x08b30f9c`) on this Time
Trial.

Matched frames (`import -window root` of the emulator while stopped, one per
phase, 60 calls in): `n == 0` the green `GO` board, `n == 1` the teal `Board`
with the swirl logo and its clipped caption, `n == laps` the grey `FINAL LAP`
board, `n == laps + 1` the chequered board. The same four boards render from
`321Go_StartFinish.vex` with `oag-view --mesh 'Data\Environments\321_Go\321Go_StartFinish.vex'
--anim-seconds 4.0/7.0/10.5/12.6 --yaw 1.5708 --pitch 0 --screenshot`: the
asset's content at those times is the original's. That compares the asset,
not our in-race frame: the standalone viewer draws every panel at once where
the race hides the others with `PanelCull`, and no headless route forces a
race to the second and third crossings, so an in-race pair is still open.

**Confidence 90** for the window selection and bounds (live, forced `n`);
the natural lap crossing itself was not flown, so the events that move `n`
rest on [race-progress.md](race-progress.md). PS2 Pulse: checked, applies
(same windows inherited), unread on `SCES_547.48`. HD: its own numbers, read
independently ([ps3-hdfury-eu/gantry-clock.md](../ps3-hdfury-eu/gantry-clock.md));
not re-measured here.

### 5. The cockpit countdown widget rides the same writes

`Hud_SetCountdownWidgetTime` (`0x0881a3b8`) is `if (hud+0x250)
Model_SetAnimTime(t, *(hud+0x250)+0x94)`. `hud+0x250` is the `Cockpit321Go`
widget ([countdown-widgets.md](countdown-widgets.md)). It is set to 0 with the
gantry at `ready`, to 3.0 on the release, and to 3.2 in the `n == 0` window
only, so the overlay's `Cockpit_321GO.vex` clock is the gantry's own until the
first crossing and then runs free. 75: the widget pointer is read, the `+0x94`
child is not identified. This answers the overlay's open timing question; the
overlay is drawn only where a circuit has no gantry. **Rewired 2026-10-12**
(`pulse-race-start`): `oag_game::hud_countdown::widget_seconds` runs its clock from
the gantry's start tick (92) so `GO` lands on the release, where it had run off tick 0
and shown `GO` 92 ticks (1.5 s) early. The widget's own time was not watched live,
so the rewiring rests on the writes above (75), not on a capture.

## The free Turbo: granted on the line crossing, not on the release

Found while chasing the hexagon the brief noticed over the banner on a
stationary craft: it was this project's Time Trial Turbo icon, granted on the
release tick. The original's three racing-state handlers that read the
crossed-this-tick byte:

```c
/* TimeTrial_UpdateRacing (0x0882ddd8), state 2 of TimeTrial_Update (0x0882dc60) */
0x0882798c(manager, 1);
if (manager->player->crossed_this_tick)              // craft+0x911
    manager->player->entity->held = 4;               // craft+0x4c -> +0x1bc, 4 = Turbo
```

`0x0882d578` and `0x08823270` carry the same two statements (an instruction search
for every `sw ..., 0x1bc(..)` finds exactly these three among the mode handlers);
which modes they are was not established, so they stay unnamed. Turbo is 4 at
`+0x1bc` per [ai-stats.md](ai-stats.md). `TimeTrial_Update` is
`0x0882dc60` because its state-1 call is `0x0882ddb0`, the handler
[countdown-voice.md](countdown-voice.md) watched live in a Time Trial.
`craft+0x911` is set by `Craft_UpdateLapProgress` (`0x08842a18`) on the first
forward crossing (with `+0x910`, once) and on every completed lap. So **a craft
that never reaches the line never holds a Turbo**, which is why the stationary
capture shows no pickup icon. Ported: `oag_race::Outcome::first_crossing`,
`Race::grant_free_turbo`. 85 for the moment.
