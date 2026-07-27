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

**Use the SDL build for anything that has to get into a race.** Headless boots
and runs, but Pulse's first boot asks for a player name and tag through the
system save dialog, and headless has no persistent memory stick, so it asks
again on every run and nothing gets past it. Under the SDL build the same
sequence is answerable, the profile is written once, and later boots skip it.

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
| `input.buttons.send` / `.press` / `input.analog.send` | Scripted input. `press` takes a duration in frames; `send` sets held state. Names are `cross`, `circle`, `square`, `triangle`, `start`, `select`, `up`, `down`, `left`, `right`, `l`, `r`. |
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
