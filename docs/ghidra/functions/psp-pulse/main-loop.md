# Main loop, frame pacing and state machine

Functions in `PSP_GAME/SYSDIR/BOOT.BIN` (Pulse PSP, UCUS-98712), image base
`0x08804000`.

The timing conclusions and their consequences are in
[frame pacing](../../../psp/frame-pacing.md) and
[ADR-0007](../../../architecture/adr/0007-fixed-timestep-vs-original.md). This
page records where things live.

**The names here are applied**, from [names.tsv](names.tsv). See
[ADR-0005](../../../architecture/adr/0005-ghidra-conventions.md).

## The loop

`Game_MainLoop` at `0x08807244`, identified by the literal `"Main Game Loop"` at
`0x08a785d0` handed to the profiler immediately before the loop. Confidence
**95**: a function that names itself is about as good as static evidence gets.

Per iteration, in order:

| Address | Name | Role | Conf |
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

`Game_Bootstrap` is five calls long and every one of them is an import stub, so
there is nothing to misread: `sceKernelUSec2SysClock(10000)`,
`sceKernelCreateVTimer("main_timer")`, `sceKernelSetVTimerHandler`,
`sceKernelStartVTimer`, then `Game_MainLoop`. Confidence **90**, since the name
is an interpretation of a function that only sets up a timer and enters the loop.

## Other loops

| Address | Name | Pacing | Conf |
| --- | --- | --- | ---: |
| `0x0890b52c` | `Loading_ThreadMain` | own thread, `vcount + 2` = 30 Hz | 88 |
| `0x0893fb3c` | `Utility_DialogLoop` | 1 vblank plus swap | 82 |

The loading screen runs on its own thread and drives its animation from
`sceKernelGetSystemTimeLow` with a 33,333 us divisor.

## State machine

**Not a switch on an integer.** It is a string-keyed hierarchical state machine.

| Address | Name | Conf |
| --- | --- | ---: |
| `0x0889123c` | `StateMachine_TransitionTo` | 88 |
| `0x08890ce4` | `StateMachine_FindState` | 75 |
| `0x08812d58` / `0x088130e4` | `InGame_Construct` / `InGame_Destruct` | 85 |
| `0x08813244` | `InGame_UpdatePauseInput` | 85 |
| `0x08818fd8` | `Demo_UpdateAttractMode` | 80 |

`0x08b31784` holds a **pointer to** the machine, not the machine itself - the
static reading said the machine was at that address, and running the game
corrects it. The machine was at `0x08d0a820` in the session that measured this,
which is a heap address and so is not itself a constant. The current state's
name is at `+0x18c` **of the machine**, as an inline character buffer rather than
a pointer into `.rodata`. Transition takes a state name as a string; queries are
`strcmp` against the current name. 31 functions call the transition.

The inline buffer is what makes this useful: reading it is a text-mode view of
where the front end is, which is what
[the debugger page](../../../reverse-engineering/ppsspp-debugger.md) navigates
the menus by. It is also how the buffer was shown to be inline rather than a
pointer - it read `"Show Logo\0election\0"`, a shorter name overwriting
`"Language Selection"` in place, leaving the tail of the longer one behind.
Confidence **95**, runtime-observed and reproducible.

Runtime observation also extends the state list below with names that string
search had not connected to a boot path. In order, from a cold boot of the USA
disc with an empty memory stick:

```
LogoFMV -> Show Logo -> RemoveMemoryStickWarning -> NameSetup2FromBoot
   -> TagSetup2FromBoot -> CreateFromBoot -> Main menu -> Racebox
   -> Single Player -> Track Creation -> Team Selection
   -> InGame -> InGameTrackDescriptionScreen -> InGame
```

Two things there are worth keeping. `Main menu` and `Track Creation` are spelled
in a different case convention from the rest, and `Track Creation` is the state
name for what the screen itself calls TRACK SELECT, while `Team Selection` is
SHIP SELECT - **the state names do not match the screen titles**, so neither can
be inferred from the other. Confidence **90** for the sequence, which is observed
rather than derived; it is one path through the machine and not the whole graph.

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

| Address | Name | Conf |
| --- | --- | ---: |
| `0x08b30f00` | `g_game` | 85 |
| `0x08abf5d4` | `g_display` | 87 |
| `0x08b31784` | `g_state_machine` | 85 |
| `0x08abfe88` | `g_last_vcount` | 87 |
| `0x08abf5d8` | `g_force_fixed_30_hide_hud` (never written) | 88 |
| `0x08abf5d9` | `g_force_fixed_60` (never written) | 88 |
| `0x08abf5cc` | `g_force_fixed_30` (never written) | 88 |
| `g_display + 0x116c` | `vblanks_per_frame_is_1` | 87 |

The scores are those of the mechanism each global belongs to: 85 for the state
machine and the game object, whose roles come from this page's decompilation, and
87 to 88 for the presentation and fixed-step globals, which
[frame pacing](../../../psp/frame-pacing.md) pins to specific constants and
call sites.

## Import stubs

Every stub this page names is confirmed by NID: `sceKernelDelayThread` at
`0x08a77114`, `sceKernelGetSystemTimeWide` at `0x08a7719c`,
`sceKernelRotateThreadReadyQueue` at `0x08a771ac`, and the four `sceDisplay`
entries at `0x08a77494`-`0x08a774ac`. See
[import stubs](imports.md), which resolves 306 of the binary's 335.

Only four `sceDisplay` functions are imported, and **`sceDisplayWaitVblankStart`
(non-callback) is not among them** — only the `CB` variant is used. That was a
reading of the stub table; it is now a checked fact.

## Not determined

- **Whether the craft physics integrator sub-steps at 1/60 or integrates raw
  delta.** The most valuable follow-up in this area; see
  [frame pacing](../../../psp/frame-pacing.md).
- The full state list and the state registration table.
- `0x08b31048` (game mode) and `0x08abf45c` (loading-screen type).
- Nothing verified at runtime.
