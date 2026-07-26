# Main loop, frame pacing and state machine

Functions in `PSP_GAME/SYSDIR/BOOT.BIN` (Pulse PSP, UCUS-98712), image base
`0x08804000`.

The timing conclusions and their consequences are in
[frame pacing](../../../psp/frame-pacing.md) and
[ADR-0007](../../../architecture/adr/0007-fixed-timestep-vs-original.md). This
page records where things live.

**Names are proposals, not applied.** See
[ADR-0005](../../../architecture/adr/0005-ghidra-conventions.md).

## The loop

`0x08807244`, identified by the literal `"Main Game Loop"` at `0x08a785d0`
handed to the profiler immediately before the loop. Confidence **95**: a
function that names itself is about as good as static evidence gets.

Per iteration, in order:

| Address | Proposed name | Role | Conf |
| --- | --- | --- | ---: |
| stub `0x08a77114` | `sceKernelDelayThread(100)` | 100 us yield | 95 |
| `0x08804978` | `Game_UpdateFrame` | measure delta, step the scene graph | 92 |
| `0x08804ad4` | `Game_RenderFrame` | build the GE display list | 88 |
| `0x0891e1b8` | `Gfx_PresentFrame` | vblank wait and buffer swap | 90 |
| `0x0891e3c0` | `Gfx_FlushRenderManager` | sort plus deferred draw callbacks | 78 |
| `0x088046a8` | (stripped no-op) | profiler hook, `jr ra` only | 95 |

So the order is **input, simulate, render, present**, once per frame. Input is
not a top-level call: the controller is a scene-graph node updated inside
`Game_UpdateFrame`. `Input_ReadAnalog` (`0x0894ebd0`) applies a 0.25 deadzone
and a 1.25 gain, which is worth having when input is reimplemented.

Reached from `Game_Bootstrap` (`0x088071d8`), which first creates a 10 ms
VTimer named `"main_timer"` whose handler calls
`sceKernelRotateThreadReadyQueue(0)`. That is a scheduler-fairness watchdog and
**not** a simulation tick; mistaking it for one would be an easy error.

## Other loops

| Address | Proposed name | Pacing | Conf |
| --- | --- | --- | ---: |
| `0x0890b52c` | `Loading_ThreadMain` | own thread, `vcount + 2` = 30 Hz | 88 |
| `0x0893fb3c` | `Utility_DialogLoop` | 1 vblank plus swap | 82 |

The loading screen runs on its own thread and drives its animation from
`sceKernelGetSystemTimeLow` with a 33,333 us divisor.

## State machine

**Not a switch on an integer.** It is a string-keyed hierarchical state machine.

| Address | Proposed name | Conf |
| --- | --- | ---: |
| `0x0889123c` | `StateMachine_TransitionTo` | 88 |
| `0x08890ce4` | `StateMachine_FindState` | 75 |
| `0x08812d58` / `0x088130e4` | `InGame_Construct` / `InGame_Destruct` | 85 |
| `0x08813244` | `InGame_UpdatePauseInput` | 85 |
| `0x08818fd8` | `Demo_UpdateAttractMode` | 80 |

The machine is at `0x08b31784`, with the current state's name at `+0x18c`.
Transition takes a state name as a string; queries are `strcmp` against the
current name. 31 functions call the transition.

Confirmed transitions: boot enters `"Language Selection"` or `"Launch Game"`
depending on `0x08ab07e3`; START/SELECT toggles `"InGame"` and
`"InGame Pause"`; a 120-second attract timeout enters `"Reset Demo"`.

Further names in `.rodata`, reachable through the same machine but with entry
paths not traced: `"InGame Restart"`, `"InGame Void"`,
`"InGame Exception Pause"`, `"InGame Ghost Retry"`, `"InGame Photo"` and about
ten photo sub-states, `"Race End Photo"`, `"Race End Save"`,
`"Race End Records"`, `"Race End Proceed"`, `"Race End Alone"`, `"Title"`,
`"Menus"`. The frontend source filename leaks as `"FrontendRoot.cpp"`.

Confidence **85** for the mechanism, **70** for the state list, which was
enumerated by string search rather than by walking the registration table.

`0x08b31048` is a separate game-mode enum compared against 5, 6, 7, 0xa, 0xc,
0xd, 0xe and 0x11. Not decoded.

## Globals

| Address | Proposed name |
| --- | --- |
| `0x08b30f00` | `g_Game` |
| `0x08abf5d4` | `g_Display` |
| `0x08b31784` | `g_StateMachine` |
| `0x08abfe88` | `g_LastVcount` |
| `0x08abf5d8` | `g_ForceFixed30_HideHud` (never written) |
| `0x08abf5d9` | `g_ForceFixed60` (never written) |
| `0x08abf5cc` | `g_ForceFixed30` (never written) |
| `g_Display + 0x116c` | `vblanksPerFrame_is1` |

## Import stubs

Resolved from the `.lib.stub` NID tables at `0x08a774d0`. Worth labelling
because they anchor everything else:

`0x08a77494` `sceDisplaySetMode`, `0x08a7749c` `sceDisplaySetFrameBuf`,
`0x08a774a4` `sceDisplayWaitVblankStartCB`, `0x08a774ac` `sceDisplayGetVcount`,
`0x08a77114` `sceKernelDelayThread`, `0x08a7715c` `sceKernelGetSystemTimeLow`,
`0x08a7718c` `sceKernelUSec2SysClock`, `0x08a7719c` `sceKernelGetSystemTimeWide`,
`0x08a771ac` `sceKernelRotateThreadReadyQueue`.

Only four `sceDisplay` functions are imported, and **`sceDisplayWaitVblankStart`
(non-callback) is not among them** — only the `CB` variant is used.

## Not determined

- **Whether the craft physics integrator sub-steps at 1/60 or integrates raw
  delta.** The most valuable follow-up in this area; see
  [frame pacing](../../../psp/frame-pacing.md).
- The full state list and the state registration table.
- `0x08b31048` (game mode) and `0x08abf45c` (loading-screen type).
- Nothing verified at runtime.
