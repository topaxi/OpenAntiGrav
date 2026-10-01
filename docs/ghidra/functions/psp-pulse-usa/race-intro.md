# The pre-race flyby: `RaceMode_UpdateIntro`'s camera pass

How a race opens on Pulse PSP (`UCUS98712`, image base `0x08804000`): the circuit's own camera
animation plays under the track-description panel, then the countdown runs. The behaviour and
its measurements are [`race-intro.md`](../../../gameplay/race-intro.md); this page is the code.

Read from the decompiler (Ghidra headless on a scratch copy of the project, 2026-10-01) and
**checked against live PPSSPP runs**: a breakpoint inside the render-view publisher once a
frame, logging the camera node, its tripod, the animation's own clock and the intro's counter
(`scripts/psp-flyby.py`), plus a breakpoint on `RaceMode_UpdateIntro` itself.

## Names

| Address | Name | Confidence | What it is |
| --- | --- | ---: | --- |
| `0x08829e6c` | `RaceMode_UpdateIntro` | 85 | the five-substate machine every mode's state 0 runs. **Substate 1 is the flyby**, not "the countdown proper" as `zone-mode.md` and `countdown-voice.md` first read it (corrected there) |
| `0x088834bc` | `Camera_PickGridCamera` | 70 | substate 0's helper: walks the scene under `g_ingame` for nodes whose type id equals a tag object it builds (read as `gridCamera`: the node it chose was one on every capture) and makes `buffer[(int)*(g_ingame + 0x40) % count]` the render node's camera (`+0x3c`); every circuit has exactly one, so the modulo is moot |
| `0x08883484` | `Camera_IntroProgress` | 78 | `100.0` with no camera, else `GridCamera_Progress` of it |
| `0x08908a70` | `GridCamera_Progress` | 88 | `0.0` until the camera's own timer passes its delay, then `anim->0x40 / anim->0x58` (the parent `Anim Transform`'s clock over its `AnimEnd`), `1.0` if the parent is not an `Anim Transform` |
| `0x08908394` | `GridCamera_Update` | 80 | the camera node's per-frame update: `+0x40 += dt`; once past `+0xc8` it clears the parent animation's pause flag (`parent+0x84`); then `FUN_08901c6c` (`+0x40 += dt` **again**, wrap, world matrix) |

`GridCamera_RegisterClass` (`0x08908aec`, [billboards.md](billboards.md)) is the class these
belong to. The render-view publisher is `FUN_08882cbc` (`0x08882cbc`, not renamed: the name
would be a guess): it copies the active tripod's rows and field of view into the render node
(`DAT_08ab10d8`) and the global view (`DAT_08ab10b0`), with the **negated** eye at `+0x70`.

## `RaceMode_UpdateIntro`, substate by substate

`mode+0x7cc` is the substate, `mode+0x7c8` the outer state (`RaceMode_SetState`). Read from the
decompile; the live values are the right-hand column.

| Substate | What it does | Live |
| --- | --- | --- |
| 0 | `g_hud+0x2c &= ~6` (HUD hidden), `Audio_SetSfxFadeTarget(0, ...)`, `StateMachine_TransitionTo("InGameTrackDescriptionScreen")` unless `g_game_mode == 2` (Demo), then `Camera_PickGridCamera(DAT_08ab10d8, ...)` and substate 1 | the state name reads `InGameTrackDescriptionScreen` |
| 1 | copies the camera's tripod into the render view; decrements `mode+0x1a04`; music calls at `0x28` and `0`; then the **exit test** below | the counter `59, 58 ... 0` (60 ticks), one per frame |
| 2 | waits `0.5` s (`FUN_0882748c(0.5)`, a `+0x7c4 >= 0.5` test), then `g_hud+0x2c \|= 6` (HUD shown), and `StateMachine_TransitionTo("InGame")` unless Demo | `InGame` from the first frame; substate 3 `30` ticks later |
| 3 | picks the ghost message (`MSC_GHOST_LOAD...`) or goes straight on | one tick |
| 4 | waits `mode+0x7b8` (`1.0` s, `RaceManager_Construct`), then `RaceMode_SetState(mode, 1)` and `ready` | substate 4 to the outer state 1: `61` ticks |

Substates 2 to 4 and the outer countdown's 180 ticks sum to the **272** the countdown has always
measured: `30 + 1 + 61 + 180`. What the old measurement called "dialog dismiss" is substate 1
ending.

### The exit test

```c
progress = Camera_IntroProgress(render_node);          /* t / AnimEnd, 0 until the camera's delay passes */
skip = false;
if (progress > 0.0) {
    if (Input_IsHeld(g_input, 5, 0))                    /* button 5 is Cross */
        skip = true;
    else if (g_debug_mode_override == 0 && g_game_mode == 2)
        skip = true;                                    /* Demo */
    else if (*(u8 *)(mode + 0x40) != 0)
        skip = true;                                    /* unread */
}
if (mode->0x1a04 == 0 && (Camera_IntroProgress(render_node) >= 1.0 || skip)) {
    music_stop();
    ScreenFlash(DAT_08ab2200, 10, ...);                 /* kind 10, screen-flash-callers.md */
    render_node->flags &= ~2;
    if (g_debug_mode_override != 0 || g_game_mode != 2) DAT_08ab10e8_node->flags |= 6;   /* meaning not read */
    RaceMode_SetSubstate(mode, 2);
}
```

So the flyby ends when the counter has reached zero **and** the animation has reached its end
or Cross is held. Button 5 is Cross in the abstract layer ([input.md](input.md)): holding the
accelerate button skips it, which is why "holding thrust through the description" ended it at
tick 61 on every earlier capture. `mode+0x1a04` is a frame counter, not a clock: it counts down
one per call.

## The camera pass

`Camera_PickGridCamera` stores the chosen `gridCamera` node as the render node's `+0x3c`, the
**tripod** `FUN_08882cbc` publishes. The gridCamera is a `Camera` (`0xf7`) specialisation: its
constructor `FUN_08908108` calls `Camera_Construct` and zeroes the timers; its bind
(`FUN_08901954`) reads the payload's aim point into `+0xa0` and the `LoopEnd` attribute into
`+0xb0`.

`GridCamera_Update` reads:

```c
cam->0x40 += dt;                                   /* the camera's own clock */
if (cam->0xc8 < cam->0x40) {                       /* the delay: 1.0 s, live */
    parent = cam->0x8;                             /* grid_camera1, an Anim Transform */
    if (parent->type == AnimTransform) parent->0x84 = 0;   /* release its pause */
}
FUN_08901c6c(dt, cam);                             /* += dt again; wrap at +0xb0; world matrix */
```

`FUN_08901c6c` adds `dt` to the same field, so the camera's clock runs at **two `dt` per tick**
and the animation is released after half a second of real time. The published pose is the
camera's world matrix, which `Vex_UpdateNodeWorldMatrix` builds from the parent's *current local
matrix*: that is why the pose lags the animation clock by one update.

## Live

`16_Track` and `03_Track`, Time Trial and Single Race, PPSSPP v1.20.4, own profile. Per frame,
at the publisher (`0x08882e9c`): the tripod's `+0x40..+0xff` (pose at `+0x60`, field at `+0x50`),
the animation's `+0x40` (clock), `+0x58` (`AnimEnd`), `+0x84` (pause), the camera's `+0x40` and
`+0xc8`, `mode+0x1a04`, `mode+0x7c8/+0x7cc`, `g_ingame+0x40`.

| Reading | Value |
| --- | --- |
| camera clock, per frame | `+0.0333` (two `dt`); `0.183` at the first stop |
| camera delay `+0xc8` | `1.0` |
| animation `+0x58` | `25.0` on both circuits (`AnimEnd` 1500 keys at 1/60) |
| pause `+0x84` | `1` until the camera clock passes `1.0`, then `0` |
| animation clock | `0.0834` five frames after it leaves zero; `24.99` at the last frame |
| counter `+0x1a04` | `57` at the third stop, `0` from the sixtieth |
| pose against the animation clock, same frame | median error `0.74` units |
| pose against the clock one frame earlier | median `5e-5`, p99 `3e-4`, over 1,521 frames |
| field of view `+0x50` | `54.30899429` (`0x42593c69`) on both circuits, every frame |
| field of view, the same circuit's `gridCamera` payload | not derivable: the 48-byte payload is identical on all twelve circuits but for the aim point |
| exit | substate `1 -> 2` at animation clock `24.994` (`16_Track`) |
| a 4-frame Cross press at stop 299 | substate 2 on the next call |

Unexplained, and recorded rather than guessed at: a fresh load from the menu walk on `03_Track`
and Metropia reached the intro's first reachable call already in substate 2 with the counter at
zero (`RaceMode_UpdateIntro` trace, `data/scratch/pulse-flyby/it-r`); RESTART RACE on both
played the flyby. `16_Track` fresh played it.

## Not read

The field of view's source (`FUN_08901954`'s tail `FUN_08908f98` binds the payload, no write to
`+0x50` was found); `mode+0x40`; what the ScreenFlash kind 10 looks like; the music calls; the
body of `FUN_08882cbc` beyond the pose copy (it has a second branch keyed on `tripod+0xc0` that
looks up `"start position 1"` in the named registry, never taken in these captures).
