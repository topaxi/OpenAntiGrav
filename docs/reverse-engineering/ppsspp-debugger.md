# Driving PPSSPP from its websocket debugger

PPSSPP exposes a JSON-over-websocket debugger. It is how [M3's verification
harness](verification-protocol.md) reads the original's state per tick, and it is
also the cheapest way to check a static reading against the game actually
running. This page is what it takes to use it, including four traps that each
cost a working session to find.

Everything here was measured against PPSSPP **v1.20.4** and Pulse PSP
(`UCUS98712`) on 2026-07-27. `scripts/ppsspp_debugger.py` encodes it;
`scripts/psp-trace.py` is the capture tool built on top.

## Getting a debugger you can connect to

```sh
# headless: no window, no dialogs, and it breaks at start
PPSSPPHeadless data/cache/pulse-psp-usa.iso --debugger=47800 --graphics=software --timeout=1800

# the SDL build: a real GPU backend, working system dialogs, save states
printf '[General]\nRemoteDebuggerOnStartup = True\nRemoteDebuggerLocal = True\nRemoteISOPort = 47810\n' > /tmp/debugger.ini
PPSSPPSDL --appendconfig=/tmp/debugger.ini --windowed data/cache/pulse-psp-usa.iso
```

Either way the endpoint is `ws://127.0.0.1:PORT/debugger`. Scripts need
`websocket-client`, so run them as
`uv run --with websocket-client scripts/psp-trace.py ...`.

**Both builds can reach the main menu.** The SDL build is still the one to use -
see below - but the claim that headless "cannot get past" Pulse's first-boot name
entry is **false, and was tested**: headless answers the same dialog with the
same key sequence and lands on `Main menu`, in about three minutes of software
rendering. What it does not do is *persist* the profile, so the walk is needed on
every headless run, while the SDL build writes it once
(`~/.config/ppsspp/PSP/SAVEDATA/UCES00465P0000` - note the **EU** save id, which
is what this USA-serial disc writes) and later boots skip straight to the menu.

**Use the SDL build anyway**, for two reasons that outweigh the profile: it
renders fast enough to sit through menus at 30-60 fps rather than software
rendering's crawl, and its window can be **screenshotted through the
compositor**, which turns blind state-name navigation into sighted navigation.
Pulse's menus are a hex grid whose state name stays `Cell Selection` across every
cell, so a screenshot is the difference between navigating and guessing:

```sh
niri msg action screenshot-window --id N --write-to-disk false && wl-paste -t image/png > /tmp/shot.png
```

**The SDL build throttles hard when its window is not focused**, which looks
exactly like the game hanging. Focus it (`niri msg action focus-window --id N`,
or whatever the compositor offers) before timing anything.

## The four traps

### Breakpoints only arm while the CPU is stepping

A breakpoint added while the CPU is running is accepted, appears in
`cpu.breakpoint.list` with the right address and `"enabled": true`, and **never
fires**. Under the JIT and under `--ir` alike. Armed before the first
`cpu.resume`, the same breakpoint fires within a second.

So the sequence is always `cpu.stepping` (break) -> `cpu.breakpoint.add` ->
`cpu.resume`. `Debugger.add_breakpoint` raises rather than let this happen
silently.

Confidence **95**: tested on both CPU backends, on addresses independently
confirmed to be executing, and the positive case reproduces every time.

### `cpu.stepping` is rebroadcast when the breakpoint list changes

Adding or removing a breakpoint while the CPU is stopped makes PPSSPP broadcast
a fresh `cpu.stepping` event carrying **the pc it was already stopped at**. A
client that waits for the next `cpu.stepping` reads that as an instant
breakpoint hit, at 0.00 s, at the wrong address. Three rounds of testing here
reported bogus hits before the pattern was clear.

Wait for a `cpu.stepping` whose `pc` equals the breakpoint address, and nothing
else. That is what `Debugger.wait_for_break(address)` does.

Confidence **90**: consistently reproducible, and the phantom event always
carries the previous pc rather than the new breakpoint's.

### A memory read costs 520 ms running and 0.3 ms stepping

Measured over 50 calls each, on the SDL build:

| Command | CPU running | CPU stepping |
| --- | ---: | ---: |
| `memory.read_u32` | 521 ms | 0.29 ms |
| `memory.read`, 256 bytes | 561 ms | 0.47 ms |
| `memory.read`, 4 KB | - | 9.5 ms |
| `cpu.status` | 0.1 ms | 0.1 ms |
| `cpu.resume` + `cpu.stepping` round trip | - | 0.5 ms |

Three orders of magnitude. `cpu.status` is answered by the debugger's own thread
and is cheap either way; memory access is marshalled onto the emulation thread
and only lands there when that thread reaches a safe point.

**Consequence, and it shapes the whole harness:** per-tick capture must be
breakpoint-driven - break, read the whole structure in one `memory.read`, resume
- and must never poll a running CPU. Polling state at 60 Hz is not slow, it is
impossible.

Confidence **90** for the numbers as measured; the mechanism behind the 520 ms is
inferred rather than read out of PPSSPP's source.

### Capture runs the emulator far below real time

Even at 0.5 ms per resume/break cycle, a breakpoint round trip in practice costs
much more than a frame: a 900-hit capture advanced the game's own lap clock by
about one second. So anything with a timer in it - a start-line countdown, a
loading screen - cannot be sat through *during* capture.

`psp-trace.py --warmup SECONDS` exists for this: run free with the buttons held,
then start capturing. Confidence **85**, from the game clock rather than from
instrumentation.

### The shoulder buttons are `ltrigger` and `rtrigger`, not `l` and `r`

`input.buttons.send` answers `{"l": true}` with

```
input.buttons.send: Unsupported 'buttons' object key 'l'
```

and the first real scripted capture died on it. Probed against v1.20.4 by sending
each candidate and reading the reply: **`ltrigger` and `rtrigger` are accepted**,
while `l`, `r`, `L`, `R`, `trigger.left`, `lt`, `rt` and `shoulder.left` are all
refused. The API table above said `l` and `r`, which had been taken from the PSP's
own naming rather than tested.

`scripts/input_script.py`'s `PPSSPP_NAME` is the one place this mapping lives, so
a future rename is a one-line fix rather than a silent mis-send.

Confidence **95**: probed directly, and the accepted pair drives a working
capture.

### Input set at a breakpoint lands three frames later

A controller state set while stopped in `Ship_UpdateCraft` is **not** seen by the
frame the breakpoint is in, nor by the next one. Measured with
`verification/scenarios/steer-both-ways.inputs`, whose whole purpose is that the
recorded `steer` column dates its own transitions: at `--script-lead 0` the script
asked for `left` at tick 60 and `steer` first moved at row **63**, and the same +3
held on all four of that capture's transitions (release at 90 seen at 93, right at
120 seen at 123, release at 150 seen at 153).

`oag-trace`'s replay shows a change at row `T+1`, since input `T` drives the step
from row `T` to row `T+1`. So the emulator is **two ticks behind our own
convention** and `--script-lead 2` closes it; re-captured at that lead, `steer`
moves at row 61 for the script's tick 60 and at row 109 for its tick 108.

Two consequences worth knowing:

- **A `--warmup-hold` handover costs the same three frames.** Releasing the
  warmup's thrust just before the loop and letting the script's first `cross`
  arrive normally leaves two ticks of zero throttle at rows 1 and 2 that the
  script never asked for. Warm up holding the button the script's first line
  holds, and it is harmless.
- The mechanism is not established. It is consistent with the game latching input
  before `Ship_UpdateCraft` runs, but that has not been read out of the binary,
  which is why this is a switch and not a constant.

Confidence **90**: four transitions in one capture, confirmed by a second capture
taken at the corrected lead - but one binary and one emulator version.

### A race updates every craft from the same function

`Ship_UpdateCraft` is the *entity* update, not the player's: in a race the
breakpoint fires once per ship per tick, with a different craft in `a0` each
time. `psp-trace.py` used to stop at the second address, on the reasoning that
interleaving two ships into one file records nothing in particular - which is
right, but makes a trace of a race impossible to take at all.

It now **follows one craft and resumes past the others**: `--ship N` picks the
Nth by order of first appearance, `--craft ADDRESS` pins one, and every address
seen is printed to stderr so a first run enumerates them. Cost is proportional:
one recorded tick costs one breakpoint round trip per ship on the grid.

Two things a first race capture found:

- **The player's craft is the one whose `throttle` is 0 or 100.** It is a button,
  so a human input is digital; the AI's is fractional, and read 34 to 51 on the
  four craft observed in a Single Race. A capture that shows only fractional
  throttles is not following the player.
- **Only four craft were updated per tick in that eight-ship race**, over 40
  consecutive hits. Why is not established. A time trial has exactly one, which
  is a further reason to prefer it for a first capture.

## What the API gives you

| Command | Notes |
| --- | --- |
| `cpu.status` | `stepping`, `pc`, `ticks`. Cheap, and the only reliable way to know the CPU's state. |
| `cpu.stepping` / `cpu.resume` | Stop and start. `cpu.resume` answers with an unticketed broadcast, not a reply to its own ticket. |
| `cpu.breakpoint.add` / `.remove` / `.list` | See the arming trap above. |
| `cpu.getAllRegs` | Categories `GPR`, `FPU`, `VFPU`. Registers arrive as parallel name and value arrays. |
| `memory.read` / `read_u32` | `read` returns `base64`; `read_u32` returns `value` (**not** `uintValue`). |
| `memory.disasm` | Address plus count, with PPSSPP's own symbol names for calls. |
| `hle.thread.list` | Thread names, pcs and wait states. `Main thread`'s entry is `Game_Bootstrap`. |
| `hle.func.list` | Every function PPSSPP's analysis found, with address and size. Useful for finding which function contains an address. |
| `hle.module.list` | Confirms the module base: `WO_Game` at `0x08804000`, size `0x391800`. |
| `input.buttons.send` / `.press` / `input.analog.send` | Scripted input. `press` takes a duration in frames; `send` sets held state. Names are `cross`, `circle`, `square`, `triangle`, `start`, `select`, `up`, `down`, `left`, `right`, **`ltrigger`, `rtrigger`** - *not* `l` and `r`, see below. |
| `gpu.stats.get` | Actual and target fps, vblank rate. |

`gpu.buffer.screenshot` **does not work** on either path tried here: it needs the
CPU or GPU stepping, and then fails with `Could not download output` under both
the software renderer in headless and OpenGL in the SDL build. Recorded so nobody
re-derives it. Screenshot the emulator's window through the compositor instead
(`niri msg action screenshot-window --id N --write-to-disk false` then
`wl-paste`, or `grim`), which works and is what produced the front-end walk
below.

## Two things worth knowing about the game itself

**The pc at the initial break is `0x08804000`.** That is a third independent
confirmation of the PSP image base, after PPSSPP's loader log and `BOOT.BIN`'s
ELF program headers.

**The front end announces where it is, in text.** `0x08b31784` holds a pointer to
the state machine, and the machine's current state name is an inline character
buffer at `+0x18c`. Reading it every second is a text-mode view of the front end,
which is what makes blind navigation practical. See
[main-loop.md](../ghidra/functions/psp-pulse/main-loop.md) for the corrected
layout and the state names observed at runtime.

## Getting into a race, once

The first boot needs answering, and only once, because the profile it writes
persists in PPSSPP's memory stick:

```
LogoFMV -> Show Logo -> RemoveMemoryStickWarning -> NameSetup2FromBoot
        -> TagSetup2FromBoot -> CreateFromBoot -> Main menu
```

`start` at the title, `cross` at the warning, then `right` x10 and `cross` to
take the default name, `right` x3 and `cross` for the tag, `cross` to write the
save and `circle` to leave the dialog. From `Main menu`: `down`, `cross` for
Racebox, `cross` for Custom Race, `right` x2 to reach TIME TRIAL, `cross`,
`cross` for the track, `cross` for the ship, then `cross` to dismiss the track
description. The state name reads `InGame` from the ship-select confirmation
onwards.

**Do not hold thrust during the countdown.** Pulse penalises a false start by
stalling the engine, and the symptom through the debugger is unmistakable and
easy to misread: `craft+0x2b8` shows the full throttle you are sending, the
craft's cached speed climbs to 10 and resets, and the ship does not move at all.
That is a stalled start, not a broken input path and not a wrong position offset.

Recovering from one needs the pause menu: `start`, then `down` x4 and `cross` for
RESTART RACE. Then hold **nothing** for about 25 seconds while the countdown
runs, and only then start the capture with thrust held:

```sh
just trace --ticks 200 --hold cross --warmup 6 --out data/traces/talons-junction.csv
```

Confirmed working: 200 ticks, 73.85 world units travelled, speed steady at 21.5
to 22.7, and a fresh craft address after the restart. The stalled and moving
cases are told apart in one line - if the distance between the first and last
position is near zero while the throttle column reads 100, the start was stalled.

## Save states, and the input-recording API that may replace them

The protocol wants a save state as the fixed starting point, so that a scenario
does not have to be re-reached by the menu walk below every time. **The websocket
debugger has no save-state command**: `savestate.save`, `savestate.load`,
`game.savestate` and `state.save` are all answered with `Bad message: unknown
event` on v1.20.4. Driving the SDL build's own hotkey from outside did not work
either - `xdotool key F1` at the window produced no file under
`~/.config/ppsspp/PSP/PPSSPP_STATE`. So the starting point is still the menu walk,
and a capture still costs a countdown.

What the API *does* have, and what is probably the better answer, is **input
recording**:

| Command | Notes |
| --- | --- |
| `replay.begin` | Start recording the input stream |
| `replay.flush` | Stop, and return it as `version` plus `base64` |
| `replay.execute` | Play a recorded stream back; wants `version` and the data |
| `replay.abort` / `replay.status` | Cancel, and ask whether either is running |

All four answer on v1.20.4. That is a route to the two things missing here that
hand-authoring a script does badly: a **full lap**, which nobody wants to write 6,000
ticks of by hand, and a *human-driven* reference run that could be dumped and
converted into the committed script format rather than guessed at. Nothing has
been built on it, and how `replay.execute`'s stream aligns with the per-tick
breakpoint is unknown.

For the record, since it is the obvious other idea: **there is no published
Wipeout Pulse TAS** to borrow inputs from. PSP TASing through libtas exists and
Wipeout *Pure* is mentioned as working, but no Pulse run was found.

## The reference scenario

Reproduced from a cold boot, and worth keeping as *the* scenario so captures
stay comparable with each other and with `oag-trace`'s runs:

**Time Trial, Venom, Talon's Junction White, Assegai.** One ship, no weapons, no
AI, 5,178 m. From `Main Menu`: `down`, `cross` (Racebox), `cross` (Custom Race),
`right` x2 (`SINGLE RACE` -> `TIME TRIAL`), `cross`, `cross` (track 1/3),
`cross` (ship 1/8), `cross` (track description). The settings persist in the
profile, so a second run of the same scenario is the same key sequence.

```sh
just trace --port 47810 --ticks 200 --hold cross --warmup 6 \
    --out data/traces/talons-junction-venom-assegai.csv
cargo run -p oag-trace -- run data/traces/talons-junction-venom-assegai.csv \
    --source data/images/pulse-psp-usa.chd --team Assegai --class venom --hold cross
```

The same scenario driven by a **committed input script** instead, which is what
makes it more than a straight line - see
[`oag-trace`'s page](../tools/oag-trace.md#input-scripts) for the format:

```sh
just trace --port 47810 --script verification/scenarios/steer-both-ways.inputs \
    --script-lead 2 --warmup-hold cross --warmup 6 \
    --out data/traces/talons-junction-steer.csv
cargo run -p oag-trace -- run data/traces/talons-junction-steer.csv \
    --source data/images/pulse-psp-usa.chd --team Assegai --class venom \
    --script verification/scenarios/steer-both-ways.inputs
```

#### The scripted form

Three things about it specifically:

- **A scripted capture warms up holding nothing.** `--hold` holds through the
  warmup, which is right for it and wrong here - the false-start trap above is
  what a scripted capture would walk into if its first line held thrust through a
  countdown. `--warmup-hold cross` is the opt-in for the other case, reaching the
  script's starting speed before tick 0 so a scenario begins the way the `--hold`
  reference capture did rather than from a standstill.
- **`--ticks` defaults to the script's own length**, so the two cannot disagree by
  accident. Asking for more than the script covers holds its last state, and says
  so on stderr.
- **`--script-lead 2` is the measured value and the default is 0.** Input set at a
  breakpoint lands three frames later, which is two ticks behind `oag-trace`'s own
  convention; see [the trap above](#input-set-at-a-breakpoint-lands-three-frames-later).
  Pass it, or every comparison carries a two-tick phase error that reads as a lag
  in the physics.

What that capture reads: 200 ticks, `dt` mean 0.016684 (59.94 Hz, min 0.016316,
max 0.017029), speed 23.6 to 25.1, 78.0 units travelled, `throttle` a flat 100,
and `cross(row0, up) = forward` on 200 of 200 ticks.

**One crash worth knowing.** PPSSPP v1.20.4 died twice on the Vulkan backend
while confirming the track for a Time Trial - process gone, last log line an
unrelated `sceKernelDeleteSema` warning. A third attempt on the same build and
the same path worked, and the difference appears to be that the earlier two ran
while a second instance was still shutting down (`Secondary instance 2 -
silencing audio` in the log). If it dies there, check nothing else is holding the
port and start one instance at a time.
