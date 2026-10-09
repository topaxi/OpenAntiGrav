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
zero (`RaceMode_UpdateIntro` trace, a scratch directory, not kept); RESTART RACE on both
played the flyby. `16_Track` fresh played it.

## The track-description panel: which strings, and how it fades

`StateMachine_TransitionTo("InGameTrackDescriptionScreen")` (substate 0 above) brings up the
screen authored in `Data\Plugins\PI001\GUI\InGame_Definition.xml`: two bars with a hexagon
tile and a gradient rule each, a `Viewport` (`deltaWidth="470"`, `enabletransition="2"`) holding
two `Text` widgets, `InGameTrackName` (font `Menu`, scale `1.2`, at `10,5`) and `InGameTrackText`
(`widthlimited`, at `10,205`). Both author `string=""`.

**What fills them** is the tail of `RaceManager_Construct` (`0x08829124`, named on
[race-progress.md](race-progress.md)), read from the decompiler and the three strings it cites
(`0x08a7a710` `InGameTrackName`, `0x08a7a720` `MSC_TRACK_%.2s`, `0x08a7a730` `InGameTrackText`):

```c
name_widget = FindWidget(FindState(g_state_machine, "InGameTrackDescriptionScreen"), "InGameTrackName");
SetText(name_widget, track_id /* *(DAT_08b310b4 + 0x74): "16_Track" */, 0, 1);   /* an id: looked up in the table */
sprintf(buf, "MSC_TRACK_%.2s", track_id);                                         /* "MSC_TRACK_16" */
SetText(FindWidget(..., "InGameTrackText"), buf, 0, 1);
mode->0x1a04 = 60;                                                                 /* the flyby's lock counter */
```

So the name is the circuit's **`PI_Track` id** as a string-table key (`16_Track` is "Talon's
Junction White", `32_Track` "Talon's Junction Black" - all 32 are in the English table) and the
paragraph is `MSC_TRACK_<first two characters of the id>` (all 32 present too). The id is the
`PI_Track` entry of `Definition.xml` for the circuit's directory and its `Reversed` flag
(24 entries, 24 distinct pairs; `18_Track` is `02_Track` reversed). Confidence 85: the format
string, its one caller and the two live captures that agree (the name and paragraph on
`16_Track` and `03_Track` against the table's own values) - the `SetText` helper `FUN_088cd0f4`
was not decompiled, so "id looked up in the table" is read from the captures, not the helper.

**The fade is `Widget_UpdateTransitionFraction`'s own** (`0x0888d8e4`,
[race-box-screens.md](race-box-screens.md)). Measured on PPSSPP (v1.20.4, software renderer,
2026-10-02, `16_Track`, Single Race): a breakpoint on it collected the screen's eleven widgets,
then `RaceMode_UpdateIntro` was broken on once a frame and each widget's `+0x64` (fraction) and
`+0x8c` (elapsed) read, with a Cross held from call 100 and a screenshot per frame. All 11 share a
parent; eight `Image`, two `Text` and the `Viewport`.

| Reading | Value |
| --- | --- |
| `+0x68`/`+0x6c` on the images and both texts | `0.7` / `0.7` (not authored on them: the class default, the same `0.7` every other widget of the front end reads) |
| `+0x68`/`+0x6c` on the viewport | `2.0` / `0.7` (the XML's `enabletransition="2"`, the disable inheriting the default) |
| entering | every fraction rises by `dt` per call over its enable seconds: images and texts reach `1.0` after 42 calls, the viewport's `+0x64` is `+0x8c / 2.0` and had reached `0.92` when the flyby ended (substate 2 first read at call 102) |
| leaving (from substate 2, the call after the Cross-held test passed) | `+0x8c` is clamped to the **disable** seconds first, so the viewport and every other widget start from `1.0` and fall `1 / 0.7` a second together: `0.964, 0.917, ... 0.010, 0.000` at calls 104 ... 146, 42 calls |
| the viewport's width | `deltaWidth * fraction`: the title's ink reached its full 333 px width at fraction `0.71` (`470 * 0.71 = 333`), and while leaving the clip edge followed `470 * fraction` to within a glyph (`WH` at `0.57`, `JUNCTION` at `0.48`). The screenshot gives only the cut glyph's edge, so this is read to about 20 px, not fitted |
| text alpha | equals the images' (`+0x64`), **not multiplied by the viewport's**: the paragraph is solid white while the viewport is a third open |

Frames: a scratch directory, not kept (scratch, not committed). The panel is gone at
call 146, 44 calls after substate 2 first read, and still `0.34` when substate 3 begins at call 130
(substate 2's `0.5` s), which is when the HUD arrives: the two overlap for the panel's last 16 calls.

Not read: the draw routine's own use of `+0x64` and the viewport's `+0xbc` (a one-frame lag of the
fraction, which is the value the clip followed); what `deltaWidth` means to the class beyond
"the width at fraction 1"; the wrap width of the paragraph (the XML authors none; `450`, the
viewport less the text's `x` on both sides, reproduces the original's line breaks on `16_Track`
and `03_Track` and is **chosen, not measured**).

## Not read

The field of view's source (`FUN_08901954`'s tail `FUN_08908f98` binds the payload, no write to
`+0x50` was found); `mode+0x40`; what the ScreenFlash kind 10 looks like; the music calls; the
body of `FUN_08882cbc` beyond the pose copy (it has a second branch keyed on `tripod+0xc0` that
looks up `"start position 1"` in the named registry, never taken in these captures).
