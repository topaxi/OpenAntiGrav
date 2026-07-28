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
# the image PPSSPP wants is an ISO; data/images/ holds CHDs, so extract one once
chdman extractdvd -i data/images/pulse-psp-usa.chd -o data/cache/pulse-psp-usa.iso

# headless: no window, no dialogs, and it breaks at start
PPSSPPHeadless data/cache/pulse-psp-usa.iso --debugger=47800 --graphics=software --timeout=1800

# the SDL build: a real GPU backend, working system dialogs, save states
printf '[General]\nRemoteDebuggerOnStartup = True\nRemoteDebuggerLocal = True\nRemoteISOPort = 47810\n' > /tmp/debugger.ini
SDL_VIDEODRIVER=wayland PPSSPPSDL --appendconfig=/tmp/debugger.ini --windowed data/cache/pulse-psp-usa.iso
```

Two things about that last line that each cost a restart to find:

- **`SDL_VIDEODRIVER=wayland` is not optional on a Wayland session.** SDL2 picks
  X11 first, and with no `DISPLAY` the build dies on
  `Unable to initialize SDL: x11 not available` after printing a full page of
  successful Vulkan probing - so the failure reads like a graphics problem and is
  not one.
- **Launch it detached** (`setsid ... < /dev/null &`), or the process can be
  reaped along with the shell that started it, leaving a log file whose last line
  is from the *previous* attempt. That looks exactly like a hang.

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

**Multiple emulator instances in parallel are a hardware non-issue - the
bottleneck is harness isolation, not CPU.** A workstation that runs one PPSSPP
comfortably runs several, so concurrent capture work (two scenarios recorded at
once, a capture running while another session iterates a menu walk) should not
be serialised on principle. What actually needs isolating per instance before
that works:

- **The websocket debugger port.** The scripts under `scripts/` connect to one
  discovered instance; two emulators need distinct ports and an explicit
  port/instance selector on the scripts (a small addition, not yet made - check
  `psp-drive.py`/`psp-trace.py` before assuming it exists).
- **The PPSSPP config and save profile.** Instances share
  `~/.config/ppsspp/` by default; concurrent config writes and the
  focus-throttle interaction are untested. `--config`/separate `PPSSPP_HOME`
  style isolation is the likely answer, unverified.

Until someone adds the port selector, one instance per machine is a tooling
limit, not a law. The thing that genuinely cannot be parallelised is a shared
git working tree - concurrent capture *agents* editing the same checkout have
bitten repeatedly (see `HANDOVER.md`'s staged-index trap).

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

**Refined 2026-07-28, and the refinement changes what is worth attempting:**
timed against the wall clock rather than against the game's, a breakpoint-driven
capture runs at **18-22 ticks/s, or 0.37x real time** - a 200-tick reference
capture in 9 seconds and a whole lap in under two and a half minutes. The
"advanced the lap clock by one second" reading is not a contradiction, because
[the game's own race clock does not count emulated
frames](#what-tells-you-the-race-finished); it is simply the wrong instrument.
Sitting out a *countdown* under capture is still not worth it, but a lap is.

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

## Walking the menus without a human

`scripts/psp-drive.py menu` gets the front end from wherever it is into a live
Time Trial, and `just scripted-emu` calls it, so a scripted run needs a booted
emulator and nothing else. It is idempotent: already in a race, it returns.

What makes it possible is that **the menu tree announces itself** through
`0x08b31784+0x18c` even though the hex-grid cells do not. The walk verifies at
every state that has a name and counts presses only in between:

| Step | Keys | State afterwards |
| --- | --- | --- |
| Back out of anywhere | `circle` until it lands | `Main Menu` |
| Racebox | `down`, `cross` | `Racebox` |
| Custom Race | `cross` | `Single Player` |
| Race type | `left` x8, `right` x2 | still `Single Player` |
| Speed class | `down`, `left` x8, `up` | still `Single Player` |
| Confirm | `cross` | `Track Creation` |
| Track, ship | `cross`, `cross` | `InGame` |
| Description, countdown | `cross`, then wait | racing |

Three things that were measured rather than assumed, each of which broke a
version of this walk first:

- **The Custom Race option selectors clamp, they do not wrap.** RACE TYPE sitting
  on `TOURNAMENT` reads `SINGLE RACE` after eight `left`s and stays there. That is
  what makes a blind walk possible at all: saturate, then count. From `SINGLE
  RACE`, `right` x2 is `TIME TRIAL`, and selecting it flips `WEAPONS` to `OFF` and
  `AI DIFFICULTY` to `N/A` on its own - the reference scenario's configuration
  falls out of the mode rather than needing to be set.
- **Track Select is a *wrapping* list of three, on the up/down axis.** Left and
  right do nothing there. A wrapping list has no reachable anchor when its index
  can only be read off the screen, so **the walk does not touch the track
  selector at all**: the profile persists it and nothing in this repository moves
  it. What it does instead is **check afterwards** - the craft's start position on
  Talon's Junction is `(6.07, -50.07, -196.10)` to every digit printed, on three
  separate restarts, so a race that comes up anywhere else is a different track
  and the walk says so and stops. `--any-track` downgrades that to a warning.
  (This is also how the earlier "`right` x2 from the docs' walk" went wrong: from
  `Racebox` it selected a *tournament*, and the run that followed was on a track
  nobody chose.)
- **`QUIT RACE` lands directly on `Main Menu`**, which is why backing out with
  `circle` is the walk's first move rather than a special case.

The first-boot dialogs are handled too, from the sequence below, so a fresh
memory stick needs no hand-holding either - though the SDL build persists the
profile, so that path runs once per install.

## Getting into a race, once

**Automated now** - see the section above; what follows is the underlying
sequence, kept because it is what the walk encodes and what to fall back on when
a layout changes.

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
RESTART RACE. **The confirm on RESTART RACE is routinely swallowed**: the item
highlights, the `cross` is accepted, and the state name stays
`InGame Pause SP Time Trial`. Press it again - a restart loop should press until
the state actually leaves the pause menu rather than assume one press took, or
the "capture" that follows records a paused game. Then hold **nothing** for about
25 seconds while the countdown runs, and only then start the capture with thrust
held:

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

**A reproducible starting point exists anyway, and it is not a save state.**
`scripts/psp-drive.py restart` runs the pause-menu sequence below end to end -
`start`, `down` x4, `cross` until the state actually leaves the pause menu, a
sleep, `cross` to dismiss the track description, a sleep through the countdown -
and hands back a stationary craft on the start line. Run three times on
2026-07-28 it produced position `(6.07, -50.07, -196.10)` **every time, to every
digit printed**.

**That is not the same as a fixed starting *pose*, and the difference cost a
whole-lap replay.** Two restarts an hour apart put the craft 0.03 units apart and
**2.51 degrees apart in heading**: a craft on the start line is sitting on its
hover rather than at rest, and where its nose has drifted to depends on how many
frames it has been sitting. That is invisible to a player, harmless to a
closed-loop run, and fatal to an open-loop script - see
[`oag-trace`'s page](../tools/oag-trace.md#the-lap-does-not-replay-into-the-emulator-and-why)
for the run where it produced a wall at tick 150 instead of a lap. What it is not is *fast*: it costs the 44 seconds of loading, description
and countdown that a save state would skip. The two sleeps are sleeps rather than
state polls because the front end's state name reads `InGame` for all of it -
loading, description and countdown are not distinguishable through `0x08b31784`,
so there is nothing to poll.

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
converted into the committed script format rather than guessed at.

### The replay API is a dead end on v1.20.4, and this is what it does

It was tried, and the result is a **crash in PPSSPP itself**. Answering that
they answer is not the same as answering that they work:

- `replay.begin` starts recording and `replay.status` reports `saving: true`.
  What comes out of a short `replay.flush` is a **17-byte-per-item stream keyed
  to absolute emulated time**: a 3-second recording of `cross`+`left` flushed as
  34 bytes, two items, each `action` byte + `u64` timestamp + a payload that
  reads `0x4080` - `CTRL_CROSS | CTRL_LEFT` - for the button item and
  `80 80 80 80` (a centred stick) for the analog one. Absolute timestamps are
  why this can only ever reproduce **from a matching point**, which without a
  save-state command means from boot.
- **`replay.flush` kills the emulator as soon as the recording spans a real
  screen transition.** Three runs, same signature every time: the websocket
  reply never arrives and the process is gone, leaving
  `stl_vector.h:1272: Assertion '__n < this->size()' failed` as the last line of
  its log. It survived a recording that stayed on the boot logo pressing `down`
  (222 bytes, 13 items); it died on a recording that walked into `Racebox`, and
  again on one that walked into a Time Trial and drove.
- So **the record half cannot produce a blob for anything worth replaying**, and
  `replay.execute` on a real session is untestable from here: there is nothing
  to hand it. Nothing about this is fixable from this repository - it is
  upstream's bounds check.

**Do not spend another session on this API** unless a newer PPSSPP fixes the
assert. The start-pose problem it was the candidate for is solved below without
it.

For the record, since it is the obvious other idea: **there is no published
Wipeout Pulse TAS** to borrow inputs from. PSP TASing through libtas exists and
Wipeout *Pure* is mentioned as working, but no Pulse run was found.

## The start pose is pinnable, and the craft was never settling

The 2.51 degrees above were read as a craft *settling* onto its hover, which
would mean waiting longer helps. **It is not settling. It is yawing, at a
constant rate, and it never stops.** Sampled for 40 seconds after a restart, on
three restarts:

| | |
| --- | --- |
| heading at the first breakpoint hit | `96.0944`, `96.0944`, `96.0841` degrees |
| drift rate | `0.005187` deg/frame = **`0.3109` deg/s, identical to four digits on all three** |
| position at the first hit | within `0.00082` units across the three |
| creep | `0.0145` units/s, monotone, in the same direction every time |

So the start pose is a **deterministic function of how many frames the craft has
sat there** - not a random settling, and not something a longer wait improves.
The 2.51-degree spread was `2.51 / 0.3109 = 8` seconds of difference in how long
the two runs waited, which is exactly the wall-clock jitter between
`psp-drive.py restart` exiting and the next process reaching its first
breakpoint (a `uv` start, a websocket connect, a menu check). Confidence **90**:
three restarts, six-digit agreement on the rate, and the mechanism predicts the
old number.

That makes the fix cheap and self-anchoring: **wait for a heading rather than
for a duration.** `psp-trace.py --start-heading DEGREES` discards breakpoint
ticks until `atan2(fwd.x, fwd.z)` passes the target and records tick 0 there.
It needs no frame counter shared between processes, no save state and no memory
write, and it is exact to one tick - `0.005` degrees. Measured before and after,
three restarts each, with a deliberate `0`/`6`/`14`-second jitter in the
handover to stand in for a real chain's:

| | heading spread | position spread |
| --- | ---: | ---: |
| wall-clock handover (what the chain did) | **`4.3468` degrees** | `0.0527` units |
| `--start-heading 101.0` | **`0.0001` degrees** | `0.0022` units |

The mechanism is visible in the run log rather than only in the result: the
three pinned runs discarded **942, 583 and 104 ticks** before starting, which is
the jitter they were absorbing, and all three still began within `0.0001`
degrees of each other.

One flake to expect while doing this: `psp-drive.py restart` fails perhaps one
time in three with `memory.read_u32: Invalid address` - the craft is read while
the race is still loading and the pointer is not live yet. It is transient and a
plain retry fixes it; the measurement above retried once per restart. Anything
scripting restarts in a loop should retry rather than treat it as a failure.

`just scripted-emu` passes trailing arguments straight to `psp-trace.py`, so the
whole chain takes it without a recipe change:

```sh
just scripted-emu verification/scenarios/steer-left.inputs \
    data/traces/steer-left.csv --start-heading 101.0
```

The residual `0.0022` units is the craft's own creep during the one tick of
heading resolution, and it is 24x smaller than before. Pick a target that every
run can still reach: the heading starts at `96.09` and only ever increases, so
`101.0` allows about 16 seconds of handover jitter, and a target already passed
is refused with a message rather than waited out for a lap. The wait costs about
`3.9` degrees at `0.311` deg/s of *emulated* time, and the capture runs at
`0.37x`, so budget roughly 35 seconds of wall clock for it.

One thing this does **not** claim: the numbers above are Talon's Junction's
start line only, and another track's drift rate has not been measured, though the
mechanism gives no reason for one to be at rest either.

### Measured: a pinned pose is necessary and nowhere near sufficient

The whole-lap open-loop replay this section used to flag as "the next
measurement" has now been run, twice, and the answer is a clean negative.

**The pinning half works perfectly.** Both runs discarded exactly **943 ticks**
and started at heading **101.0018** - the same numbers to every digit, from two
independent restarts.

**The replay half does not.** The 3,146-tick committed lap script
(`verification/scenarios/talons-junction-time-trial-lap.inputs`) driven
open-loop from that pinned start does not reproduce the lap it was recorded
from, and - the decisive comparison - it does not even reproduce *itself*:

| Separation reaches | vs the recorded lap (start `0.77` deg apart) | run 1 vs run 2 (starts `0.0000` deg apart) |
| --- | ---: | ---: |
| `1` unit | tick 68 | tick 123 |
| `10` units | tick 172 | tick 182 |
| `100` units | tick 495 | tick 495 |
| at tick 3,146 | `194` units | `545` units |

Over the full 3,146 ticks the open-loop run travels `1,461` units where the
recorded lap covers `5,608`: it loses the racing line inside two seconds and
spends the rest of the run grinding along walls.

**The mechanism is the original's variable timestep, and it is visible in the
captures' own `dt` column.** Comparing the two pinned runs tick by tick, `dt` is
identical on **31 of 3,146 ticks (1.0 %)** and differs from tick 0 onward - same
mean (`0.016683`), different sequence, because the game integrates a *measured*
frame duration ([ADR-0007](../architecture/adr/0007-fixed-timestep-vs-original.md))
and the emulator's frame durations depend on host load. The positions differ by
`0.0017` units on tick 0 and grow from there. No amount of pose pinning can fix
that: it is not an initial-condition problem.

**What this means for the workflow**, and it is a genuine constraint rather than
a defect to fix:

- **A capture and its script are one run.** Anything that needs the original's
  own trajectory has to be recorded closed-loop (`psp-autopilot.py`) or seeded
  from a capture (`oag-trace run`), which is what every comparison in `docs/`
  already does.
- **Open-loop scripts are still fine for short scenarios.** The pitch, steer and
  airbrake captures are 200-360 ticks and diverge by well under a unit over their
  useful windows; the pinning is what makes *those* repeatable.
- `just scripted-sim` - the same script through our own simulation - is
  deterministic by construction and unaffected. Its usefulness as a *fidelity*
  metric is what this bounds: two runs of the original disagree with each other
  by 100 units at tick 495, so no target for that recipe is meaningful past the
  first few hundred ticks.

## Driving a whole race, and the three numbers that make it possible

The "nobody wants to write 6,000 ticks of input by hand" problem above is solved,
and not by input recording. There are now three ways to put input into a running
Pulse, and picking the wrong one is what makes a lap look impossible:

| Tool | How it drives | Emulator speed | Records |
| --- | --- | --- | --- |
| `psp-trace.py` | breaks every tick, sends at the breakpoint | ~0.37x | a full per-tick trace |
| `psp-drive.py drive` | free-running, paced by the emulator's own clock | **1.00x** | a coarse progress log only |
| `psp-autopilot.py` | breaks every tick, steers closed-loop off the spline | ~0.37x | a trace **and** the input it chose |

Three measurements, all taken on PPSSPP v1.20.4 with `UCUS98712` on 2026-07-28:

- **A breakpoint-driven tick costs about 45 ms**, so a capture runs at **18-22
  ticks/s**, or 0.37x real time. That is the number the earlier "far below real
  time" warning was pointing at, and stated as a rate it is far less alarming
  than it sounds: a 200-tick reference capture takes 9 seconds, and **a whole lap
  of Talon's Junction - 3,146 ticks - takes 2 minutes 22**. A three-lap Time
  Trial is under eight minutes. Nothing about a lap needs a save state or an
  overnight run.
- **`cpu.status` is answered by the debugger's own thread and costs 0.1 ms
  whether the CPU is running or not**, and its `ticks` field is the PSP cycle
  counter. Dividing by `222e6 / 59.94` turns it into an emulated frame index, and
  that is a *free* frame clock for a running emulator - which is what lets
  `psp-drive.py` send a run-length script at 1.00x real time without ever
  stopping the CPU. Measured: 900 ticks driven in 15.0 s with **zero** sends
  landing late.
- **A craft's address is not stable across a restart.** Two restarts gave
  `0x09a0c270` and one gave `0x09a0c9c0`, from identical starting positions. Learn
  it from one breakpoint hit per run and never cache it across one.

### What tells you the race finished

**`Ship_UpdateCraft` stops being called.** The breakpoint simply never fires
again and the capture times out; that is a finished race rather than a hang, and
`psp-autopilot.py` treats it as one.

Everything else has to be read off the HUD, because `gpu.buffer.screenshot` does
not work (above) and no lap counter has been located in memory. Screenshotting
the window through the compositor at each lap boundary is what
`psp-autopilot.py --shot-dir` does, and it is the only evidence in this tree that
the *game* agrees a lap happened rather than the spline arithmetic agreeing with
itself. On the committed lap it read **`Lap 2 of 3`, `best 1.11.08`** at tick
3,087.

**A compositor screenshot can hand you the PREVIOUS one.** `niri msg action
screenshot-window --write-to-disk false` puts the image on the Wayland
clipboard and the `wl-paste` that follows reads whatever is there *now*: if the
copy has not landed yet, the file written is the last thing that was copied,
with no error anywhere. Measured on the `omega_*` lap - the shot labelled
`lap1-tick03084.png` came out as a **`TRACK SELECT` menu** from a previous
session, and the one labelled `end-tick03140.png` held the lap-boundary frame.
So the whole `--shot-dir` series can be one grab behind and one of them can be
arbitrarily stale. When a shot is the evidence, take it deliberately -
`wl-copy --clear`, screenshot, sleep, `wl-paste` - and sanity-check that the
image is even in-game before reading a lap counter off it.

**The game's own race clock does not run on emulated frames.** Two laps of nearly
identical length - 3,069 and 3,087 ticks, both 51.2 s at 59.94 Hz - were timed by
the game at `0.50.25` and `1.11.08`. Whatever it counts, it is not the frames the
capture counts, and **no timing read off the HUD during a breakpoint-driven run
means anything**. The lap *counter* is unaffected and is what to use.

**A Time Trial that reaches its third lap starts a fresh attempt behind you**:
the counter goes back to `Lap 1 of 3` and `best` clears. A run that keeps driving
past the finish is therefore recording a second race, not more of the first.

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

#### The reference scenario is not a free-running ship, and this matters

**A `--warmup 6 --hold cross` capture is taken from a craft that has already hit
something**, and every number the scenario is quoted for - "speed steady at 21.5
to 22.7", "23.6 to 25.1" - is a *post-impact* speed rather than the craft's own.
Measured 2026-07-28 against a standing start on the same configuration
(`data/traces/talons-junction-standing-start.csv`): a craft launched from rest
with thrust reaches **49.9 units/s in 61 ticks** and is still gaining 68
units/s^2 when, at tick 61, one frame of `-790` units/s^2 along its own right
axis ends the run - it drives itself into the first wall, because nothing is
steering. From there it settles at 21-22 units/s and stays there, which is the
regime the six-second warmup hands to `--script`.

Free running it does not stop at 22. A capture that survived longer
(`--warmup 2`, saved as `talons-junction-free-run-118.csv`) reads **118 units/s**
with the engine pinned at its documented cap. So the reference scenario cruises
at about a *sixth* of the craft's own speed, and any measurement that treats it
as an equilibrium of the force law is measuring the impact instead. See
[the force-balance page](../physics/force-balance-ground-truth.md).

Keep taking the reference scenario - it is the only capture two years of numbers
are quoted against, and comparability is worth more than realism. But take the
standing start too, and take it *first* when the question is about forces:

```sh
# from InGame, having sat out the countdown holding NOTHING (~25 s), so that the
# craft is stationary, past the start-line boost window and not false-started
just trace --port 47810 --ticks 300 --hold cross \
    --out data/traces/talons-junction-standing-start.csv
```

`--hold` sets the pad before the breakpoint loop arms rather than at a
breakpoint, so it does **not** pay the three-frame latency the scripted path
does: `throttle` reads 0 on rows 0 and 1 and 100 from row 2. Those two rows are a
feature - they are the only rows in any capture where the craft is at rest under
its own hover, and `trace-force-balance.py` drops the first of them because
rolling resistance normalises a velocity that is exactly zero there.

**The column set has grown twice since, and the same rule bit twice.** The
angular *velocity* `body+0x150` (`omega_x/y/z`) was added after the first
completed lap was captured, so that lap could not tell a wrong inertia tensor
from a second rotation source and the lap had to be **re-flown**, which cost
2 minutes 21 of capture and answered the question outright (see
[cornering-ground-truth.md](../physics/cornering-ground-truth.md#the-refutation-is-scoped-the-inverted-section-was-carrying-it)).
The lesson is cheap to act on: adding a column to
`scripts/psp_trace_fields.py` costs nothing at capture time, so add the one you
might want rather than the one you need. `craft+0x280` is the next such column
and is still not recorded.

**Every capture taken before 2026-07-28 predates five columns** - the angular
velocity at `body+0x160` (`avel_x/y/z`) and the two engine-gate timers
`craft+0x290` and `craft+0x2e0` - and none of them can be backfilled, because a
capture is a hand-driven session rather than a reproducible build step.
`oag-trace` reads those columns when they are there and reports them as *not
compared* when they are not, so the old files stay usable; but the questions they
were added for - what the yaw rate actually does frame by frame, and whether the
engine gate fired at all - need the scenario re-taken. Both were re-taken on
2026-07-28 and both are answered: the engine gate never armed in any of the six
captures, and the angular column turns out to be
[negated body-local, scaled by 21.2](../physics/angular-velocity-column.md). Re-taking it is the two
commands above, unchanged: the columns come along on their own.

**One crash worth knowing.** PPSSPP v1.20.4 died twice on the Vulkan backend
while confirming the track for a Time Trial - process gone, last log line an
unrelated `sceKernelDeleteSema` warning. A third attempt on the same build and
the same path worked, and the difference appears to be that the earlier two ran
while a second instance was still shutting down (`Secondary instance 2 -
silencing audio` in the log). If it dies there, check nothing else is holding the
port and start one instance at a time.
