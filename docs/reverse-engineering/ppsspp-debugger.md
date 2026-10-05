# Driving PPSSPP from its websocket debugger

PPSSPP exposes a JSON-over-websocket debugger. It is how [M3's verification
harness](verification-protocol.md) reads the original's state per tick, and it is
also the cheapest way to check a static reading against the game actually
running. This page is what it takes to use it, including four traps that each
cost a working session to find.

Everything here was measured against PPSSPP **v1.20.4** and Pulse PSP
(`UCUS98712`) on 2026-07-27. `scripts/ppsspp_debugger.py` encodes it;
`scripts/psp-trace.py` is the capture tool built on top.

**Never spawn a window into a developer's real desktop session.** This page's
own advice below ("use the SDL build anyway") is about which PPSSPP *build*
to run, not about where its window lands, and it has been misread that way
before - an agent launched `PPSSPPSDL --windowed` straight at a live Wayland
session and put a visible emulator window on the user's own screen. The
preference order is:

1. **The SDL build under Xvfb first.** `PPSSPPHeadless` runs
   `--graphics=software` unconditionally - there is no hardware-accelerated
   headless mode - and software rendering is genuinely slow (the "about three
   minutes" to reach the main menu below is that cost showing up on a single
   boot; it compounds on anything longer). The SDL build under Xvfb gets a
   real GPU backend (confirmed: `AMD Radeon Graphics (RADV RENOIR)` alongside
   the `llvmpipe` fallback, see below) while staying exactly as invisible to
   the developer as headless is - no window lands anywhere a human can see
   it, since Xvfb is a virtual framebuffer, not a real display. Default to
   this for anything beyond a single trivial read. See ["Running without a
   real display"](#running-without-a-real-display-xvfb-works-no-compositor-needed).
2. **Plain `PPSSPPHeadless`, no Xvfb, only when Xvfb itself isn't available**
   or the task is a single quick read where the extra setup isn't worth it.
   It still reaches the main menu and drives the same websocket debugger API
   (see "both builds can reach the main menu" below) - it is correct, just
   slow.
3. **A real windowed launch on the developer's own session is not a default
   choice** - only do it if the human explicitly asks to watch the emulator
   themselves.

## Getting a debugger you can connect to

**PPSSPP v1.20.4 boots a `.chd` directly** - measured 2026-08-11, the whole way to
a driveable single race - so the extraction step below is optional on that version
and `chdman` is not needed at all. It is kept because it is what older notes and
older builds assume, and because an ISO is what `PPSSPPHeadless` was tested with
here.

```sh
# optional on v1.20.4, which boots the CHD itself; needed if your build does not
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

**The SDL build is worth reaching for over headless** when either of two
things matter: it renders fast enough to sit through menus at 30-60 fps rather
than software rendering's crawl, and its window can be **screenshotted**,
which turns blind state-name navigation into sighted navigation. Pulse's race
campaign is a hex grid whose state name stays the same across every cell, so a
screenshot is the difference between navigating and guessing.

**That grid is not the main menu**, though this page used to imply it was. It is
`CellMode_Definition.xml`'s `Grid Selection`, reached from `RACE CAMPAIGN`; the
main menu itself is a plain vertical list of seven rows. See
[front-end menu definitions](../formats/fe-menu-definitions.md). The point about
sighted navigation stands either way, and for a better reason: **state names do
not match screen titles at all**, so a screenshot is the only thing that says
which screen a name is. **Run it under
Xvfb by default** (see below) - the screenshot recipe there
(`import -window root`) works the same way against a virtual display as this
one does against a real compositor:

```sh
# only when driving a real, visible session the human asked to watch -
# never as a default; see the preference order above
niri msg action screenshot-window --id N --write-to-disk false && wl-paste -t image/png > /tmp/shot.png
```

**The SDL build throttles hard when its window is not focused**, which looks
exactly like the game hanging. Focus it (`niri msg action focus-window --id N`,
or whatever the compositor offers) before timing anything - this trap is
specific to a real compositor session; see below for whether it also applies
under Xvfb (not yet confirmed either way).

## Two probe findings (2026-10-05, fx-age-clocks)

- **A breakpoint condition can dereference memory and compare it with a raw float bit pattern.**
  `cpu.breakpoint.add` takes `condition`, and `[a0+0x40] < 0x40a00000` keeps only hits where the word at
  `a0+0x40` is below `5.0f` (positive floats order like integers). That cut a probe on a function hit
  for every animated mesh every frame down to the few objects of interest.
  `Debugger.add_breakpoint(address, condition)` and `each_hit(..., condition=...)` carry it.
- **A `timeout`-killed probe leaves the CPU stepping with its breakpoint still armed**, and the
  debugger then looks hung (`state` times out). `brk()`, `remove_breakpoint` for each entry of
  `cpu.breakpoint.list`, `resume()` clears it.

## Running without a real display: Xvfb works, no compositor needed

Verified 2026-08-04, PPSSPP v1.20.4, `pulse-psp-usa.iso`, on a machine with no
Wayland session running at all:

```sh
Xvfb :97 -screen 0 1280x720x24 & XVFB_PID=$!
DISPLAY=:97 SDL_VIDEODRIVER=x11 setsid PPSSPPSDL --appendconfig=/tmp/debugger.ini \
    --windowed data/cache/pulse-psp-usa.iso < /dev/null &
DISPLAY=:97 import -window root shot.png   # ImageMagick, stands in for the compositor grab below
# ... drive the session ...
kill "$XVFB_PID"                           # always - see the note below
```

**Nothing here tears this Xvfb down automatically, unlike `scripts/pcsx2-drive.py
stop` and `scripts/rpcs3-drive.py stop` for the other two harnesses' displays.**
This one is started by hand at the shell, one command at a time, so
`$XVFB_PID` from the line that started it is the only handle on it - capture it
and kill it yourself when the session ends, or it joins the orphans: three
Xvfb processes were found running on this project's own machine, all two or
more days old with no client attached, after sessions that never killed the
one they started. **Never `pkill Xvfb` or `pkill -x Xvfb`** to clean up
instead - that reaches every virtual display on the machine, including one a
different script or a different agent's session is using right now, not just
`:97`.

The SDL build boots normally under Xvfb: GPU probing finds a real Vulkan device
(`AMD Radeon Graphics (RADV RENOIR)`) alongside the `llvmpipe` software
fallback, the log reaches `Booted data/cache/pulse-psp-usa.iso...`, and the
websocket debugger opens its port exactly as under a real session. `import
-window root` pulled the whole virtual screen and returned a real in-game
frame, not a black or blank one - so the compositor screenshot recipe above
(`niri msg action screenshot-window`) is a convenience specific to a Wayland
session, not a requirement: any X11 screenshot tool works once a `DISPLAY`
exists, real or virtual.

What this pass did **not** confirm, so treat as untested rather than ruled out:

- **The focus-throttle trap above assumes a window manager** granting and
  revoking focus. Xvfb has none, so whether PPSSPP throttles under it is
  unknown either way.
- **Only boot, GPU init, the debugger port and one screenshot were checked.**
  The menu walk, scripted capture and save-state overlay were not re-run under
  Xvfb in this pass; they are expected to carry over unchanged, since nothing
  about them depends on a compositor being real, but that is an inference, not
  a measurement.

Confidence **75**: the parts exercised worked cleanly and repeatably in one
session; the rest of this page's workflow is assumed rather than independently
re-verified under Xvfb.

**Two more traps found 2026-08-05, running a real capture (not just a
screenshot) under Xvfb for longer than the pass above did:**

- **`import -window root` can come back solid black even though it "worked"
  the pass above.** Cause: with no window manager, SDL places its window
  wherever it likes on the virtual screen - one run put it at `(967, 1200)`
  on a `1280x720` canvas, entirely off-screen, so `root` captured empty space
  around it. Fix: reposition the window onto the canvas before shooting,
  with `python3-xlib`:
  ```python
  from Xlib import display
  d = display.Display(":97")
  root = d.screen().root
  # find the SDL window (its name/class identifies it) via root.query_tree(),
  # then:
  window.configure(x=0, y=0)
  d.sync()
  ```
  Confirm the window's actual position before trusting a screenshot is real;
  a black image from `import -window root` under Xvfb means "check the window
  position," not "the capture is broken."
- **ImageMagick 7.1.2's `import -window <id-or-name>` (anything other than
  literally `root`) fails outright here** with a misleading
  `missing an image filename` error, regardless of whether the id is hex,
  decimal, or the window's name. `-window root` plus repositioning (above) is
  the only path that worked, not a fallback of convenience.

### 2026-08-11: the whole walk runs under Xvfb, at full speed, on the CHD

The pass above left the menu walk and everything after it as "expected to carry
over, but an inference". It carries over. One session, `pulse-psp-usa.chd`,
`Xvfb :99 -screen 0 1280x720x24`, software GL (`LIBGL_ALWAYS_SOFTWARE=1`), no
window manager and no compositor of any kind: first-boot dialogs answered, main
menu, Custom Race set to SINGLE RACE / VENOM, track and ship confirmed, track
description dismissed, countdown sat out, craft on the grid at
`(-132.30, -49.58, -175.27)`, then a lap driven by holding thrust through the
debugger. Screenshots at every step with `magick import -window root`, all real
frames. This is what settled the HUD's top-right anchor - see
[hud.md](../ui/hud.md#the-place-and-the-total-time-are-authored-at-one-anchor-so-one-of-them-has-to-go).

**The focus-throttle unknown is retired: there is no throttle.** Measured off the
PSP cycle counter rather than off the frame rate - `cpu.status`'s `ticks` advanced
3.00 s of emulated time in 3.00 s of wall clock, so **100 % speed** with nothing
focused and nothing to focus it. Which also means the wall-clock sleeps the menu
walk is built on hold under Xvfb; they would not survive a 20 % emulator.

Four things this pass adds:

- **Give the instance its own profile directory, and it is not only for
  parallelism.** `HOME=<dir> XDG_CONFIG_HOME=<dir>/.config PPSSPPSDL ...` with
  `<dir>/.config/ppsspp/PSP/SYSTEM/ppsspp.ini` written beforehand works, and is
  the "separate `PPSSPP_HOME`" the section above guessed at. It was **required**
  here: `~/.config/ppsspp` was not writable, PPSSPP said so once as
  `Error saving config (Loaded appended config)` and carried on - and the
  websocket debugger never opened, so `--appendconfig`'s
  `RemoteDebuggerOnStartup` did not take effect. The two variables were changed
  together with the ini, so "unwritable config silently loses appended settings"
  is the reading rather than an isolated cause; either way, **an ini on disk in a
  writable profile is the path that worked**.
- **A failed debugger handshake looks like a hang, not a refusal.** With the
  debugger off, `GET /debugger` answers `301` to `/debugger/index.html` - the
  file server's own behaviour, and the tell that the flag did not apply. With it
  on but the emulator still grinding through the boot logos, the WebSocket
  upgrade simply times out. Retry before concluding anything; it connected on the
  next attempt with no change.
- **The main menu has two spellings**, and it stopped the walk dead:
  `Main menu` out of a fresh profile's first-boot dialogs, `Main Menu` every
  other way in. `psp-drive.py` compared exactly and reported "the front end is in
  `'Main menu'`, not `'Main Menu'`", which reads like a changed menu tree. It now
  compares case-insensitively.
- **The attract demo is a race by state name.** Idle ~120 s at the menus and the
  game starts driving itself, announcing `Demo InGame` - which contains `InGame`,
  so `psp-drive.py menu` read it as "already in a race" and returned success
  having done nothing. Any button leaves it and one press lands at the main menu;
  the walk now presses out of it rather than trusting the name.

**The menu walk is deliberately track-blind** (by design - it doesn't try to
identify which circuit is loaded), so reaching a *specific* track needs manual
navigation: pause -> quit race -> Racebox -> Custom Race -> Track Select,
identified by screenshot (the state name doesn't change per track) rather than
by state. **The menu's display name and the disc's directory name don't
obviously match anywhere in `docs/formats/`** - e.g. "Moa Therma" is
`03_Track`, found via a code comment (`crates/render/tests/road_uv_ground_truth.rs`)
rather than a documented table. Worth a proper name-to-directory table
somewhere in `docs/formats/track.md` if track selection by name becomes a
recurring need.

A headless Wayland compositor (`sway`/`wlroots` with `WLR_BACKENDS=headless`)
was considered as the alternative for reaching a truly compositor-driven
screenshot path, but Xvfb already answered the feasibility question with tools
already on the machine, so it was not installed or tried.

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

**The lead closes the phase error and opens a second one, at the start only.**
Sending tick `k + 2` at tick `k` means the script's first two states are never
sent at all - at tick 0 the pointer already stands at index 2. So a capture taken
at lead 2 is a recording of the script *with its first two ticks released*, and
the replay side has to release them too: `oag-trace run --script-lead 2`. It cost
a session and stood as M4's blocker before anyone looked, because the steering
ramp never settles on its target and so carries two ticks of head start all the
way round a lap. The measurement, and what correcting it was worth, is in
[`oag-trace.md`](../tools/oag-trace.md#the-first-two-ticks-of-a-script-never-reach-the-emulator).

Two further consequences worth knowing:

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

### Pairing an entry and an exit of one call: swap the breakpoint inside the call

Because only the newest breakpoint fires, reading a function's input and its output in the *same* call
is done by swapping: at the entry hit read the input, `cpu.breakpoint.remove` the entry, `add` the exit
address, `cpu.resume`, wait for the pc to equal the exit (the stepping-rebroadcast rule above still
applies), read the output, swap back and resume to the next entry. The pair is one call's own input and
output, which is what a per-frame comparison against the original's variable timestep needs. Used for
`Camera_SubmitScene` (`0x08878874` entry, `0x08878af0` after its shake block) in
`docs/ghidra/functions/ps2-pulse-eu/collision-shake.md`, 2026-09-30; about two round trips a call, 300 calls
in under ten minutes. **Memory can be written while stopped at the entry**, which makes a *forced* input
possible (the shake's struct fields were rewritten there to arm a full-strength shake on a craft at rest);
write every field the function reads, or it reads the stale ones (a first forced arm left the falloff
positions zero and measured no envelope at all). Screenshots of the stopped emulator show a frame or two
behind the stopped call.

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
| `hle.func.list` | Every function PPSSPP's analysis found, with address, name and size. Useful for finding which function contains an address, and for harvesting what PPSSPP autodetected - see [ppsspp-symbol-bridge.md](ppsspp-symbol-bridge.md). Works right after boot, before reaching any menu: module analysis runs at load time. |
| `hle.module.list` | Confirms the module base: `WO_Game` at `0x08804000`, size `0x391800`. |
| `hle.func.add` / `.remove` / `.rename` / `.scan` | A live-session alternative to loading a `.sym` through the Debug menu - naming a function without a GUI at all. Exercised below; **`.remove` immediately followed by `.add` at the same address crashes the emulator**, `.rename` does not. |
| `input.buttons.send` / `.press` / `input.analog.send` | Scripted input. `press` takes a duration in frames; `send` sets held state. Names are `cross`, `circle`, `square`, `triangle`, `start`, `select`, `up`, `down`, `left`, `right`, **`ltrigger`, `rtrigger`** - *not* `l` and `r`, see below. |
| `gpu.stats.get` | Actual and target fps, vblank rate. |

`gpu.buffer.screenshot` **does not work** on either path tried here: it needs the
CPU or GPU stepping, and then fails with `Could not download output` under both
the software renderer in headless and OpenGL in the SDL build. Recorded so nobody
re-derives it. Screenshot the emulator's window through the compositor instead
(`niri msg action screenshot-window --id N --write-to-disk false` then
`wl-paste`, or `grim`), which works and is what produced the front-end walk
below.

## Naming a function live, with `hle.func.add`/`.rename`, and a crash to avoid

Confirmed against PPSSPP v1.20.4, 2026-09-09, `psp-pulse-usa`, over
`Debugger.call` (`scripts/ppsspp_debugger.py`) - these are not wrapped by a
dedicated method there yet, call them with `dbg.call("hle.func.rename", ...)`
directly.

**`hle.func.rename(address, name, size=<optional>)`** renames an existing
entry in place. Worked example, renaming `z_un_08814014` (a function
`hle.func.list` already reported at size 3528):

```python
>>> dbg.call("hle.func.rename", address=0x08814014, name="OagRoundTripTest")
{'event': 'hle.func.rename', 'ticket': '4', 'address': 142688276, 'size': 3528, 'name': 'OagRoundTripTest'}
```

`memory.disasm` immediately reflects it, and correctly for the *whole*
function body, not just its entry address - every instruction from
`0x08814014` up to (not including) the next known function's start reports
`"function": "OagRoundTripTest"`, and only the entry instruction carries
`"symbol": "OagRoundTripTest"` (the rest carry `"symbol": null`, same as
before the rename):

```
0x08814014  function=OagRoundTripTest  symbol=OagRoundTripTest  addiu sp,sp,-0x1C0
0x08814018  function=OagRoundTripTest  symbol=None              lui a1,0x8B3
...
0x08814dd8  function=OagRoundTripTest  symbol=None              addiu sp,sp,0x1C0   # last insn of the body
0x08814ddc  function=z_un_08814ddc     symbol=z_un_08814ddc     lw a1,0x08B32C6C     # next function, untouched
```

The `size` argument to `.rename` was tried and **appears to be ignored** -
passing `size=0` alongside a rename left the registered size at the
original 3528, confirmed by re-reading `hle.func.list` afterward. Renaming
does not appear to be a route to changing a function's extent, only its name.

**`hle.func.add(address, name, size=<optional>)` fails cleanly if a function
already exists at `address`** (`"Function already exists at 'address'"`) -
useful as an existence probe, and this is the case `PpssppImportSymFile.py`'s
own default behaviour (create-or-skip) needs, since almost every text
address already has *some* entry (`z_un_...` if nothing else) once
`hle.func.list` has run.

**The crash: `hle.func.remove` then `hle.func.add` at the same address is
fatal, every time.** Reproduced twice independently (different addresses,
one at the entry point / current PC, one far from it, `size >= 32` either
time):

```python
>>> dbg.call("hle.func.remove", address=addr)
{'event': 'hle.func.remove', 'ticket': N, 'address': addr, 'size': orig_size}   # succeeds
>>> dbg.call("hle.func.add", address=addr, name="Anything", size=0)
# no reply ever arrives - the process has already exited
```

The emulator's own stderr, both times:

```
.../bits/stl_vector.h:1253: reference std::vector<unsigned int>::operator[](size_type) [_Tp = unsigned int, _Alloc = std::allocator<unsigned int>]: Assertion '__n < this->size()' failed.
```

An out-of-bounds vector access, consistent with `.remove` shrinking some
index PPSSPP's own analyzer or symbol table rebuilds lazily, and the very
next `.add` reading a now-stale index into it. Whether the crash needs
`size=0` specifically, or fires on *any* `.add` right after a `.remove`
regardless of arguments, was not isolated further - reproducing it a third
time to narrow that down was judged not worth another crashed session. **The
practical rule: never call `hle.func.remove` and `hle.func.add` back to
back on the same address. Use `hle.func.rename` to change an existing
entry's name instead** - confirmed safe above, and it is what every actual
use case here needs anyway (PPSSPP already has *some* entry at nearly every
address, so "add" is rarely the right call once `hle.func.list` has run
once).

This is the same shape as the already-documented `memory.write` empty-payload
crash below: a websocket call that is accepted syntactically and kills the
process instead of erroring. Treat any `hle.func.*` mutation as one more
address needing the same caution.

## Two things worth knowing about the game itself

**The pc at the initial break is `0x08804000`.** That is a third independent
confirmation of the PSP image base, after PPSSPP's loader log and `BOOT.BIN`'s
ELF program headers.

**The front end announces where it is, in text.** `0x08b31784` holds a pointer to
the state machine, and the machine's current state name is an inline character
buffer at `+0x18c`. Reading it every second is a text-mode view of the front end,
which is what makes blind navigation practical. See
[main-loop.md](../ghidra/functions/psp-pulse-usa/main-loop.md) for the corrected
layout and the state names observed at runtime.

**This address is `psp-pulse-usa`-specific and does not carry over to
`psp-pulse-eu`.** Confirmed live (2026-08-05): the EU binary's equivalent
global (`Game_MainLoop`'s `_DAT_00058334` in Ghidra) is not reachable at
runtime the same way - `memory.read_u32` on it returns `Invalid address` on a
running EU capture, plausibly related to the EU image's data-segment
rebasing `corroboration.md` already flags elsewhere. Since the project's
Ghidra target of record is now EU (see `corroboration.md`), state-name-based
menu walking (`psp-drive.py`'s approach, and the table below) needs its own
EU address before it works there; until that's found, drive an EU capture by
screenshot navigation under Xvfb instead (confirmed to work reliably) rather
than assuming the USA recipe below carries over unchanged.

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

### Every circuit, not three: the dev-unlock byte (2026-09-29)

A fresh profile offers three circuits, but `Definition_IsUnlocked`
(`0x0888e29c`, [race-box-screens.md](../ghidra/functions/psp-pulse-usa/race-box-screens.md))
passes everything when a byte of the profile object is set:
`*(u32 *)0x08b31774 + 0x45f` (or `sceKernelGetGPI() & 1`, a devkit pin that
PPSSPP reads as zero). **Confirmed live**: write `1` there while the game runs,
any time after `Main Menu` and before Track Select builds its list, and Track
Select reads `1 / 24`. A plain `memory.write` works without stepping the CPU. The
byte is in the loaded profile, so a reboot or a profile reload clears it.

With it set, Track Select's order from Talon's Junction White, pressing `down`,
is the definition's order with each Black/White pair adjacent:

| Downs | Circuit | | Downs | Circuit |
| ---: | --- | --- | ---: | --- |
| 0 | Talon's Junction White (`16_Track`) | | 12 | The Amphiseum White |
| 1 | Talon's Junction Black | | 13 | The Amphiseum Black |
| 2 | Moa Therma White | | 14 | Fort Gale White |
| 3 | Moa Therma Black | | 15 | Fort Gale Black |
| 4 | Metropia White | | 16 | Basilico White |
| 5 | Metropia Black | | 17 | **Basilico Black (`01_Track`)** |
| 6 | Arc Prime White | | 18 | Platinum Rush White |
| 7 | Arc Prime Black | | 19 | Platinum Rush Black |
| 8 | De Konstruct White | | 20 | Vertica White |
| 9 | De Konstruct Black | | 21 | Vertica Black |
| 10 | Tech De Ra White | | 22 | Outpost 7 White |
| 11 | Tech De Ra Black | | 23 | Outpost 7 Black |

Identified by screenshot of each entry's title. Which `NN_Track` each title
loads is in [track.md](../formats/track.md#white-and-black), read off the disc's
own definition and string table and confirmed by start position for four of
them (2026-10-03, `pulse-variant-map`). The working scripts
(`unlock.py`, `to_track_select.py`, `race_log.py`, `place_trace.py`) were
scratch, under `data/scratch/sunk-craft-2/`.

**Zone was selectable on a used profile (2026-10-02, `zone-bloom`).** On a copy of
a 70-run PPSSPP profile, `Racebox -> Custom Race -> RACE TYPE`, `right` x5, one
key at a time with 1 s waits, selects ZONE with the dev byte set or `0`, and
the race comes up with `g_game_mode == 6` and its own `zone_track.vex`
(`psp-drive.py menu --race-type 5` landed on mode 3 in the same state). A fresh
profile was not tried. See `docs/ghidra/functions/psp-pulse-usa/bloom.md`, the
Zone section.

**Two traps found walking to Metropia and Tech De Ra (2026-10-01).** `psp-drive.py
menu --track-down N` presses down from wherever the profile left Track Select, so
a count that reached a circuit once does not reach it again, and the table above
did not predict what came up (four downs from the saved position gave
`02_Track` reversed, a further one an unidentified circuit, then `03_Track`
forward, then `05_Track` reversed, then `04_Track` reversed). **Identify the
circuit after every walk**: take the craft's start position and heading from
`psp-drive.py state` and find the `NN_Track\{track,track_reversed}.vex` spline
(`oag-trace track`) whose nearest row is within about 20 units with a tangent that
agrees (dot `+-1`). The reversed file is a separate spline, and `psp-drive.py
place` with a tangent from the wrong one reads as "a respawn rather than a drive":
the craft is back on the grid within a dozen ticks. To leave a race for the menu
without a reboot: `start`, `down` five times, `cross` (QUIT RACE is the sixth of
the pause menu's six rows) lands on `Main Menu`, and `menu` then walks on.

`Debugger.each_hit` used to fail with `cpu.resume: CPU not stepping` when the CPU
had been left stopped at the breakpoint's own address (a previous script that
died inside a hit): adding the breakpoint rebroadcasts `cpu.stepping` carrying
that pc, the wait accepted the rebroadcast as a hit while the CPU was in fact
running, and the next resume refused. It now empties the queue behind a
`cpu.status` round trip before each resume.

### Stopping *at* `Track Creation` or `Team Selection`, instead of walking through them

**Scripted since 2026-09-09: `scripts/psp-frontend-capture.py`.** It does
the whole of what this section describes and more - from wherever the
emulator is, through the first-boot dialogs if any, it screenshots every
row of `Main Menu`, `Racebox` and `Single Player`, every `RACE TYPE`, both
selection screens per entry twice a second apart, and the `*_Help` and
music-select overlays, then backs out to `Main Menu` without launching a
race. 54 frames in about four minutes against the SDL build under Xvfb:

```sh
uv run --with websocket-client scripts/psp-frontend-capture.py --out data/cache/fe-capture
```

The reading of those frames is [selection-screens.md](../ui/selection-screens.md).
The notes below are what the script was built from and still apply to a
hand-driven capture.

`menu()` is deliberately not reusable for this: it presses `cross`, `cross`
straight through both screens into `InGame` with no stop in between, because
finishing a race is the whole point of that function. Screenshotting the
screens themselves (done for
[race-setup.md's live capture](../formats/race-setup.md#captured-live-in-ppsspp-2026-09-05))
needs the same back-out/`Racebox`/`Custom Race` prefix but has to interpose a
screenshot and stop before the two `cross` presses that `menu()` fires
unconditionally.

The reusable shape: import `psp-drive.py` as a module (`importlib` off its
file path, since `psp-drive` is not a valid Python identifier for a normal
`import`) and reuse its `Debugger`, `tap`, `expect`, `named` and `FIRST_BOOT`
rather than re-deriving the two naming gotchas above (`Main menu` vs
`Main Menu`, `Demo InGame`). Everything up to and including
`expect(dbg, TRACK_SELECT, ...)` is copy-pasteable from `menu()` verbatim;
what comes after is whatever the capture needs instead of the next `tap`.
`Team Selection` has no exported constant - compare against the literal
string, as `race-setup.md`'s capture script did.

Two things that capture found worth keeping here:

- **`square` is inert on `Track Creation`, opens `Pre Race Music Select` on
  `Team Selection`, and `triangle`/`select` both open a `*_Help` overlay on
  either screen** (`Track Help`, `Team Help`) - three button bindings that
  were not otherwise recorded on this page.
- **The wrapping three-entry track list can be walked all the way round with
  `down` alone** (four presses lands back at the start) to enumerate every
  reachable circuit and its stated distance, without ever needing to read the
  index off memory.

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
event` on v1.20.4 - confirmed twice, the second time by dumping the build's own
string table (`strings /usr/bin/PPSSPPSDL`) rather than by guessing more command
names, which is the stronger form of the same claim. So a state cannot be
*created* through the API; it has to be created the way a human does, once, by
hand.

**Loading one is a different question, and it works.** `PPSSPPSDL --help` lists
`--state=FILE` - "load state from FILE" - which combines with
`--appendconfig=/tmp/debugger.ini` to boot straight into a saved pose with the
websocket debugger already live. The earlier `xdotool key F1` failure is not
evidence against the hotkey: that binary is not installed on this machine, and
PPSSPP runs under `SDL_VIDEODRIVER=wayland`, which X11 key injection cannot
reach regardless of which key it tries. With `wtype` (Wayland's own virtual
keyboard protocol, paired with `niri msg action focus-window --id N` so the
press lands on the right window) the actual save flow is:

1. Focus the PPSSPP window, then `wtype -k Escape` - **not** F1, F2, F5, Tab, a
   number key or Space, all of which do nothing silently and read exactly like
   the original dead end. Escape opens PPSSPP's *own* overlay (five save
   slots plus Settings/Continue on the right), which is a different screen
   from the game's in-race pause menu (`Start`, "GAME PAUSED", reached by the
   `cross`-button sequence elsewhere on this page) - it is easy to confuse the
   two from a screenshot alone.
2. `wtype -k Left` four times, then `wtype -k Right` once, to reach a slot's
   "Save state" button - the overlay's default focus is not on it - then
   `wtype -k Return`.
3. A `.ppst` (the state, tens of MB) and a `.jpg` thumbnail appear under
   `~/.config/ppsspp/PSP/PPSSPP_STATE/`, which was confirmed empty beforehand;
   that emptiness is exactly the check that told the original `xdotool` attempt
   it had failed.

One trap while the overlay is open: **`memory.read_u32` and friends time out**,
because loading a state (or having the save overlay open at all) suspends
emulation, and the debugger's polling has nothing to catch. That reads as a
crashed debugger to a script that does not expect it; it recovers as soon as
the overlay is dismissed.

A state file is a full RAM dump of the game, so it is game content like
anything under `data/`: gitignored, never committed, and now covered by
`just audit-leakage`'s extension list (`.ppst`, `.p2s`, `.state`), which did not
include it before this pass despite `data/traces/` already getting the same
treatment.

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

### Measured: a save state does not pin the pose as tightly as `--start-heading` does, and is not adopted

With loading now possible, the obvious next question is whether a save state
gives a *tighter* fixed start than the pause-menu walk. Measured 2026-07-30, on
a state cut at the start line after letting the craft's hover settle (`speed`
readings flat across two consecutive polls before saving) and loaded twice from
two independent, fully killed-and-relaunched PPSSPP processes:

| | position delta | heading delta |
| --- | ---: | ---: |
| two loads of the same `.ppst` | `0.031` units | `1.41` degrees |
| `psp-drive.py restart`, for comparison | `0.03` units | `2.51` degrees |
| `--start-heading` (below) | `0.0022` units | `0.0001` degrees |

The craft pointer (`0x09a0cff0`) was identical across both loads - the state
loads the same heap layout both times, so the mechanism itself is not
suspect - but the two reads showed a nonzero, sign-differing residual velocity
(hover jitter that never fully stops even at rest), and `psp-drive.py`'s
breakpoint-driven read fires an indeterminate number of ticks after boot rather
than at a synchronized instant, so **the two-degree-scale spread here cannot be
cleanly attributed to the save mechanism versus this measurement's own
attach-timing jitter** - resolving that would need a tick-exact read (a frame
counter compared across runs), which this pass did not build.

Either way, the number that matters is the comparison: a save state is roughly
the same precision as `restart` and **two orders of magnitude worse** than
`--start-heading`, which is already committed, already wired into
`just scripted-emu`, and does not need a RAM dump, `wtype`, or a window to
focus. **A save state is not adopted as the fixed-start mechanism.** What this
pass leaves behind instead: `--state=FILE` and the wtype/Escape recipe above
work and are documented, in case a future need (loading mid-race state that
`--start-heading` cannot reach, or a PPSSPP version where the replay API above
gets fixed) makes them worth returning to.

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
  in the physics. **Pass the matching `oag-trace run --script-lead 2` on the
  replay side as well**, or the comparison carries the mirror-image error instead:
  the lead leaves the script's first two ticks undelivered, and our side applies
  them.

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

**Checked 2026-08-30 and not reproduced: the currently-committed
`talons-junction-standing-start.csv` shows no tick-61 impact at all.** Its
`speed_cached` climbs smoothly through 45-58 across ticks 55-70 with no
lateral-velocity discontinuity anywhere near there, and free-runs to `119.4`
units/s by tick 187 before the first real contact. Same launch profile
(`--hold cross`, no steering), different outcome - this is the "derived
evidence under `data/` is not durable" trap from `HANDOVER.md`'s own read-this-first
section, now caught in a file this page itself cites: whatever produced the
61-tick impact above is not preserved by the committed CSV, and a session
reasoning about *this* file's numbers should check what it actually contains
rather than trust the paragraph above. Not re-diagnosed; noted so the next
session does not lose time re-discovering the mismatch.

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

**The column set has grown three times since, and the same rule bit twice.**
2026-08-27 added the sideshift trigger's five timer floats
(`ss_tap_window_l`/`_r`, `ss_shift_l`/`_r`, `ss_lockout`) for
`sideshift-double-tap.inputs`'s first real capture - see
`docs/ghidra/functions/psp-pulse-usa/input-bindings.md`'s runtime section.
Before that, the
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

## Memory writes are a committed tool now, and the two traps that ride along

`memory.write` and `memory.write_u32` work on v1.20.4 and are wrapped in
`scripts/ppsspp_debugger.py` (`write`, `write_u32`, `write_f32s`) since
2026-08-04; the first committed consumer is `psp-drive.py place` below. Before
that the only write this project ever sent was the ad-hoc roll perturbation the
`ALIGNMENT_INERTIA` measurement in `crates/physics/src/hover.rs` records, so the
mechanism itself has been proven since that pass.

- **An empty base64 payload kills the emulator outright** (promoted here from
  HANDOVER, where it cost a session). The wrapper refuses a zero-length write
  rather than trusting every caller to remember.
- Writes are only meaningful while the CPU is stepping, the same as reads:
  everything `place` writes happens inside one breakpoint hit, so no frame ever
  integrates a half-written body.

## Only the most recently added execution breakpoint fires

Two execution breakpoints cannot coexist on v1.20.4. Armed ship-then-camera, a
live race stops **only** at the camera address, 189 consecutive hits with the
ship's `Ship_UpdateCraft` never firing once; armed camera-then-ship, the same
race stops only at the ship address; each breakpoint alone fires every tick,
and `cpu.breakpoint.list` reports both `enabled=True` throughout. Confidence
**90**: reproduced in both orders against a running race, but one emulator
build and no read of PPSSPP's own source for the mechanism.

Consequence for tooling: interleaving two per-tick streams with two breakpoints
is impossible. The pattern that works - and what `psp-trace.py --camera` does -
is to take **one** hit of the second breakpoint to learn a stable address out
of a register, remove it, and read that address per tick from the surviving
breakpoint. `Debugger.each_hit_any` stays for probes that expect one of a set
to fire at all, and its docstring carries this warning.

## Teleporting the craft works, and what a settle looks like

`psp-drive.py place` writes the followed craft's rigid body inside one
`Ship_UpdateCraft` hit: basis rows `+0x000/+0x010/+0x020` built as
`row2 = tangent`, `row1 = up re-orthogonalised`, `row0 = cross(row1, row2)` (the
recorded rows' own orientation identity), the transpose block at
`+0x0c0/+0x0d0/+0x0e0`, position `+0x030`, velocity `+0x140` along the new
forward, and zeros into `+0x150`/`+0x160`. Whether the transpose write is
*necessary* was not isolated - the rows and the transpose were always written
together - only that the pair is sufficient.

Observed live (2026-08-04, Talon's Junction, Time Trial, three placements):

- **The game accepts the pose.** No respawn, no snap-back: placed 50 units
  before speedup pad 0 at 120 u/s, the craft simply drives on - it covered 55
  units of the placed forward during a 30-tick settle, decaying 120 to 99 u/s
  on a closed throttle, and on a second run **crossed the pad and took its
  boost** (speed rose to 124), so triggers fire normally for a teleported
  craft.
- **The settle is real and must be waited out.** Hover height, `speed_cached`
  and the camera spring all re-derive over the first dozens of ticks;
  `--settle 30` at speed, or 60-90 at rest, left the craft flying normally.
  Off-axis drift during a settle measured 7-15 units (track curvature and the
  hover finding its height); `place` prints the settled pose so a capture uses
  reality rather than the request.
- The camera *snaps* with the craft rather than springing across the map - the
  first captured frame after a settle already frames the new location.

## Screenshots at the breakpoint, and the clipboard's one-shot lag

`psp-trace.py --shot-every N --shot-dir D` screenshots the emulator window at
every Nth recorded tick, through `scripts/niri_shot.py` (factored out of the
autopilot). The CPU is stopped at the shot, so shots are tick-addressable. Two
measured behaviours:

- **niri's clipboard write lags the screenshot action**, reproducibly a full
  shot behind at capture cadence: triggering the action again re-queues another
  late write, so the fix is one action then polling `wl-paste` until the
  content *changes* (up to 2 s). Before the fix, alternating shots were
  byte-identical copies of their predecessor - HANDOVER's stale-clipboard trap,
  now detected: a shot that cannot be told apart from the clipboard's previous
  content is written as `.stale.png` rather than cited as evidence.
- **The shot-vs-row offset is below measurement at low speed.** Matching window
  shots of a coasting craft against our own renders of neighbouring captured
  rows (`just frame-shot`, RMSE metric), rows within +/-2 of the shot's tick
  differ by under 0.1 % of the cross-renderer RMSE floor - so the presented
  frame is the current row's within about two ticks, and for a *stationary*
  comparison the offset does not matter at all, which is how
  `docs/tools/frame-compare.md` calibrates fov. A fast-moving per-tick capture
  could pin it exactly; nothing needed it yet.

## Texture dumping settles sprite-vs-mesh, offline and bit-exact

`DumpTextures = True` in the `--appendconfig` ini makes PPSSPP write every
texture it decodes as a PNG under
`~/.config/ppsspp/PSP/TEXTURES/<GAMEID>/new/`, named by VRAM address and
content hash. It is the cheapest instrument on this page - no breakpoints, no
websocket, no stepping - and it answers a question screenshots alone cannot:
**is this thing on screen a rendered mesh, or an image the game shipped?**

The method, and it closed a wrong reading of Pure's race-box previews on
2026-09-10 (`docs/formats/race-setup.md`):

1. Boot with `DumpTextures = True`, walk to the screen, and note which dumps
   are new by mtime.
2. Decode every entry of the candidate archive to PNG and compare. A **RMSE 0**
   match between a dump and a shipped entry means the pixels on screen *are*
   the shipped texture, and nothing rendered them.

Two corollaries worth carrying:

- **Per-entity distinctness proves nothing.** "A different silhouette per
  team" is equally true of a per-team sprite. That observation was the whole
  basis of the wrong reading it replaced.
- **Raise the internal resolution and look again** for a 10-second version of
  the same check. PPSSPP upscales geometry *and* render targets, but not a
  fixed-size source image - so a preview that stays blurry at 5x is a sprite.
  Anti-aliased edges and soft gradients say the same thing on their own: PSP
  hardware produces neither.

The dump directory is game content and is gitignored territory - cite
measurements and entry indices, never commit the PNGs.

## Session hygiene

Moved here from `HANDOVER.md` on 2026-08-09; the traps above are about the
protocol, these are about running the emulator at all.

- **`pkill -f PPSSPPSDL` inside a shell command whose own text contains that
  string kills the calling shell.**
- **Two emulator instances can be running** (another agent's), so match on pid
  and window id rather than on app id.
- **An unfocused window grabs as pure black** with no error - the documented SDL
  throttle. Focus it and check the grab's mean luma before reading anything off
  it.
- **`niri msg windows` prints no absolute coordinates**, but `niri msg --json
  windows` gives `tile_pos_in_workspace_view`, which is what `grim -g` wants.
  Getting it wrong crops a corner and looks like a HUD missing half its widgets.
- **`RemoteISOPort` does not pin the debugger port** - PPSSPP has bound an
  ephemeral 46659 while the config said 47810. Read the port out of the running
  process rather than the config.
- **Restore the savedata backup (move, do not delete)** before concluding a
  session that touched `~/.config/ppsspp/PSP/SAVEDATA/UCES00465P0000`.
- **Run every debugger script as `uv run --with websocket-client
  scripts/...`** - plain `python3` dies on the missing `websocket` module.
- **If the emulator is stuck in `Race End Photo`**, neither `psp-drive.py
  restart` nor `menu` can leave it: press `cross`/`start`/`circle` in a loop for
  about five rounds to walk out through `EndRace Results`/`Rewards`/`Menu` to
  `Main Menu`.


## Memory watchpoints, and the control that makes a zero mean something

**Verified 2026-08-17, PPSSPP v1.20.4.** `memory.breakpoint.add` works and is not
in the API table above because nothing had used it until now. It is the tool for
"does anything read this?", which no amount of static sweeping answers as well.

```python
dbg.call("memory.breakpoint.add", address=0x09881900, size=156,
         enabled=False, log=True, read=True, write=False, change=False)
```

- `memory.breakpoint.list` returns each watchpoint with a **`hits` counter**, so
  a run needs no break/resume round trip at all: arm it, let the emulator run at
  full speed, and poll the count. That sidesteps the 520 ms cost of
  `memory.read` while running, and it sidesteps the execution-breakpoint bug
  above entirely.
- `enabled: False, log: True` **counts without halting the CPU**, which is what a
  full-speed observation wants.
- Two watchpoints on two ranges both fire, unlike execution breakpoints where
  only the most recently added one ever does. Measured with a read watchpoint and
  a write watchpoint armed at once.
- **The debugger's own `memory.read` and `memory.write` do not count as hits.**
  Checked directly - five reads and a write over an armed range left the counter
  at zero. So reading around a watchpoint does not poison it.

### Always arm a positive control

**A watchpoint that never fires looks exactly like a watchpoint that does not
work.** This repository has been caught by that shape twice already - the first
`hover::sweep` was gated so it never fired, which reads as "the fix did nothing",
and `just apply-names` reported 383 renames it had not made. So a zero is only
evidence if something else was counting at the same time.

The control that works: arm a second watchpoint, same flags, same process, over a
couple of hundred bytes of a **live craft struct**, which the physics, the camera
and the HUD all read every frame. It counts 350,000-400,000 reads a minute. A
target that counts zero beside it is genuinely not being read.

Getting a craft address needs no new tooling: `psp-drive.py state` prints one.

### A stale or invalid watchpoint address counts nothing, so bounds-check it

**Verified 2026-08-31, PPSSPP v1.20.4, `pulse-psp-usa.chd`.** A control watch
computed as `craft + BODY_POINTER` came back reading a body pointer of `0`
(see the next trap for why), so the watchpoint was armed on address `0x30`
instead of a real one - well outside `0x08000000`-`0x0A000000`. It sat there,
`enabled: false`, at 0 hits. **Always check a computed watch address is in PSP
RAM bounds before arming it**: a watch on a bogus address is not an instrument,
it is a zero that looks like a measurement, which is exactly the failure
["always arm a positive control"](#always-arm-a-positive-control) exists to
catch.

**Corrected 2026-09-05 - this trap used to claim the bad watchpoint *caused* an
emulator halt. It did not.** The same session ended in `E[MEMMAP] Bad memory
access detected! 00000030 ... Stopping emulation.` at JIT block
`08872f98_z_un_08872f54`, and the coincidence of the number `0x30` made that
read as cause and effect. It is not: a PPSSPP memory watchpoint cannot produce a
host `movaps` load fault with the guest memory base in `rbx`, and the halt has
since been reproduced deliberately on two clean boots **with no watchpoint armed
at all**. It is the emulated game loading a quad from `null + 0x30`, triggered
by writing the LeachBeam fire bit by hand - see
[the `00000030` halt](../ghidra/functions/psp-pulse-usa/bad-memory-access-halt.md).
The practical rule that replaces the old one: **do not fire the LeachBeam
(`0x8000`) through `scripts/psp-fire-weapon.py`'s raw-bit mechanism**; it halts
the emulator within a frame, every time.

### Two connections racing the same execution breakpoint corrupts the read, not just the timing

**Found the same session.** `psp-drive.py restart`'s own `settle_into_race`
breaks at `Ship_UpdateCraft` internally to print its own progress line. A
second, independent `Debugger` connection that *also* adds, waits on, and
removes an execution breakpoint at that same address - to learn a craft
address for its own purposes - is not just redundant, it corrupts the read:
one run produced a craft address that matched `psp-drive.py`'s own output
exactly, but a field read off that craft one instruction later (`craft +
0x1CC`, the rigid-body pointer) came back `0`, which is what produced the
invalid watchpoint address in the trap above. PPSSPP execution breakpoints
are global CPU state shared across every connected client - two clients each
resuming and re-breaking the same address race each other for which one's
resume actually advances the CPU past which one's read. **The fix is not
retrying the read - it's not re-deriving state a script that is already
running (`psp-drive.py restart`/`state`) already printed.** Take the address
from that output instead of re-breaking to learn it a second time.

### Locating a heap structure without a breakpoint

If a structure's contents are authored and distinctive, **scan for them** rather
than breakpointing the code that builds it. A full sweep of PSP user RAM
(`0x08800000`-`0x0A000000`) in 64 KB `memory.read` chunks takes **about 21
seconds** while the CPU is stepping, and an authored float run is usually unique
in it - the `WeaponAIstats` table's 39 floats matched in exactly one place.

That also answers a question a static sweep cannot: whether the structure is on
the heap or at a link-time address. A heap address rules out any consumer reading
it through absolute `lui`/`lwc1` immediates, which is the escape route a
displacement-based static sweep would miss.

**Once you know the owner, stop scanning.** A 21-second scan is fine once and
tedious every run, and a heap address changes between runs. If the structure hangs
off a singleton stored in a global - which Ghidra will show you - dereference that
instead. For the `WeaponAIstats` record it is one read:

```python
table = dbg.read_u32(0x08b317b4) + 0x710
```

Verify the fingerprint at the computed address before arming anything, and again
afterwards: that is what rules out having watched a recycled heap block rather
than the structure you meant.

### `log: True` will fill your disk and wedge the emulator

**This is the trap to know before arming anything wide.** A logging watchpoint
writes a line per hit. A 256-byte watchpoint over a live craft struct fires about
**6,000 times a second**, and PPSSPP's stdout reached roughly **6 GB**, filled
`/tmp` and wedged the emulator: every `memory.read` then times out, hit counters
freeze, and the shell starts returning exit 1 with no output. `/tmp` here is a
6.8 G tmpfs, which is also why
[the `chdman` ground-truth test fails](../../HANDOVER.md) - the same disk.

**That also explains the one anomaly this page used to record as unexplained**: a
wide watchpoint reporting 6,488 hits at 15 s and 9,706 at 30 s and then freezing
was the log filling, not the game reading. A frozen counter and a quiet one look
identical.

So: redirect the log somewhere with room, cap the run, or watch a narrow range.
And **re-arm before believing any non-zero count** - the same rule the flaky
`ps2_source_ground_truth` transcode tests get in `HANDOVER.md`.

### But the log is the prize, not just a hazard

Its lines carry the **address and the program counter**, at full emulation speed,
with no halting:

```text
CHK Read32(CPU) at 09881944 ((09881944)), PC=08851af4
```

That is strictly more than the `hits` counter gives, and it is how the
`WeaponAIstats` reader was finally identified after three static sweeps and one
watchpoint run had missed it. **Prefer the log over the counter** whenever the
question is *who* rather than *whether*.

Two mechanics that each cost time:

- Restarting the emulator: kill with `-9` and **wait for port 47810 to free**,
  or the new instance cannot bind and the debugger talks to the dying race.
- **Delete the log rather than truncate it.** PPSSPP's fd keeps its offset, so a
  truncated file becomes sparse and stale hit lines survive in it; grep then reads
  them as the new run's data. That produced one bogus intermediate result.

### A zero means the instrument worked, not that the scenario happened

The hardest lesson of the run this page documents. A read watchpoint on the
`WeaponAIstats` table measured **0 hits over four minutes with a control counting
400,000 a minute** - and the record was being read all along, about 400 times a
second, in a race where the player was *driving*.

The earlier race had the player **parked**, and the reader gates on a signed
along-track range test (`ahead < 150`, `behind > -200`). A stationary player never
satisfies it. Same watch address, same control craft, opposite answer.

**So a positive control proves the instrument, and nothing about the scenario.**
Before reporting a zero, ask what would have to happen in the game for the code
under test to run at all, and then make it happen.

### A worked example: finding a pending-impulse writer no static sweep found

`scripts/psp-watch-pending-impulse.py` is this whole recipe as a reusable
script, not just a description of one - read all live entity addresses off a
breakpoint hit, arm a write watchpoint on each one's derived field plus one
control, resume, and let a driven race do the rest. It is what found
`Weapon_PostBlastImpulse_q` (`0x0886794c`) and a second, still-unnamed writer
into `Ship_ApplyCollisionImpulse`'s pending vector after a static sweep across
every plausible displacement and call tree came up empty - see
`docs/ghidra/functions/psp-pulse-usa/contact-response.md#two-writers-found-at-a-live-write-breakpoint-2026-08-19`.
The shape generalises to any "who writes this heap field" question: derive the
address live rather than guessing it, arm write-only/log-true, always run a
control alongside, and read the writer's PC out of the emulator's own stdout.

### A controlled watch can still miss a write - the pool slot outlives the log, does not

**Found 2026-08-19**, chasing `contact-response.md`'s fuse-arming candidate
(`FUN_0885bf84`). The "always arm a positive control" recipe above was
followed exactly - not skipped, not half-done - and it still produced an
uninterpretable result, which is worth a trap of its own since the existing
sections only cover a control that *isn't* run, or a scenario that *doesn't*
happen. Neither applies here.

Setup: a write log watch (`enabled: False, log: True, size: 4`) on a
freshly-armed entity's own field, plus a positive control (`write: True`,
156 bytes) over a live craft body, both armed together, both left running
free for 90 seconds. The control counted **30,413 hits** in the window - the
watch mechanism, the logging path and the emulator were all demonstrably
alive and unwedged the entire time, and the target's own single expected hit
(the arming write itself) was caught and logged correctly, PC and all. By
every check this page already recommends, the setup was sound.

**And yet the target's own value changed during the window with no second
log line for it.** Read directly at the end: the entity's type tag had
flipped to a different value and the watched field held a new float, neither
of which matches anything the arming write set. Something wrote there.
Nothing logged it. Repeating the experiment on a second entity, lighter-
weight and without the control, reproduced the same shape: the entity's tag
changed within two to three minutes with no corresponding `CHK Write*(CPU)`
line anywhere in the emulator's own stdout for that address, checked by
grepping the whole log file, not just the watch's own hit counter.

**The most likely reading: whatever recycles a freed pool slot's memory does
not go through the CPU-instruction store path this debugger's watchpoints
hook.** `strncpy` is proof some native/HLE helpers *are* instrumented -
elsewhere in this same session a stray watchpoint caught three separate
`Write8(CPU)` lines from `strncpy` touching the same address a byte at a
time, PC correctly reported as `strncpy`'s own entry. So "native helper"
alone does not explain a miss; whatever actually reinitializes a freed
weapon-instance slot is either a still-different native path, a GPU/audio
DMA write, or something else this page's author did not chase down. That
mechanism is **not established** - what is established, directly and
reproducibly, is the *symptom*: a passing positive control does not certify
that every write to the target was seen, only that the ordinary path was
capable of being seen.

**Consequence for the existing "always arm a control" advice above: it is
necessary, not sufficient.** A zero (or a low, fully-accounted-for) hit
count on the target, even beside a healthy control, is good evidence nothing
*ordinary* wrote there - it is not proof nothing wrote there at all if the
target's own value is capable of changing by a route other than a normal
CPU store. Before trusting an absence, cross-check the target's own value
before and after the window, the way this session did by accident rather
than by original design; a changed value with a silent log is the tell that

## Reading the frame's GE list: `gpu.record.dump`

**Measured 2026-09-30, PPSSPP v1.20.4, `UCUS98712`.** The websocket answers
`gpu.record.dump` with a `data:` URI holding a PPSSPP GE dump (`.ppdmp`) of the next
frame the GPU draws - about 0.2 to 0.9 s, 500 to 750 KB, with the CPU running and
no stepping. It is the one instrument here that shows **which batches the original
submits**, and the reason it is needed: the original draws through compiled call
lists (`Mesh_CompileBatchSet`/`Mesh_DrawBatchSet` in
[mesh-draw.md](../ghidra/functions/psp-pulse-usa/mesh-draw.md)), so a breakpoint on
`Gu_DrawArray` does not fire per draw, and a screenshot cannot tell a hidden mesh
from a culled one.

`scripts/psp-ge-dump.py dump` saves the file and `census` reads it: one record per
PRIM with its vertex count and its world-space bounding box under the world matrix
in force (the GE's fixed-point positions are scaled by it, so a box only makes sense
after the transform). A batch is then recognised by **(vertex count, sorted box
extents)**, taken off this project's own draws in the same way, which survives the
matrix and needs no address. The layout the script reads is in its docstring.

Two traps:

- **A pose placed at speed is not the pose dumped.** `psp-drive.py place --speed 41
  --settle 40` left the craft 23 units from where it was placed before the dump ran.
  Place at `--speed 0 --settle 12` (the craft then drifts under a unit), dump, and
  take the ours-side pose from a `psp-trace.py --camera` capture of the same resting
  craft.
- **A moving batch's box depends on the animation phase**: our side reads the rest
  geometry, the original's is after the node's transform, so a rotating or scaling
  mesh can miss its twin. Use the signature to prove a batch is *present*, and treat
  a miss on a moving one as weaker than a miss on a static one (static twins
  matched 88 to 98 % at well-posed dumps; method and results:
  [frame-audit.md](../rendering/frame-audit.md) section 3).

## Halting on a hot function silently voids a timed capture

**Measured 2026-08-28.** An execution breakpoint on `Gfx_BindTexture` -
called roughly twenty times a rendered frame during ordinary track drawing -
was run for 22 real seconds, meant to cover a known ~24 s window (dismissing
the track description through to the green light). It produced 435 hits
across 13 distinct texture pointers, every one present from the first hit to
the last and none new after the first few - which reads exactly like a clean
negative: "nothing else binds a texture in this window."

**It was not a negative. It was a voided capture, and nothing about the
output said so.** Each breakpoint round trip costs about 44 ms measured
directly from inter-hit timestamps, and the emulated CPU is fully halted for
that whole span - no game time passes between hits, only in the brief
resume-to-next-hit gap. At roughly twenty binds per frame, 435 hits is on the
order of twenty to thirty rendered frames: **about 0.3-0.5 s of game time
elapsed across 22 s of wall clock**, not the ~24 s of countdown the capture
was meant to observe. Covering the real window this way would need on the
order of twenty minutes of wall clock per attempt.

**The tell, in hindsight, was in the data the whole time**: a genuinely
covered 24 s countdown crosses several visually distinct states (description,
counting down, green light, cars moving) and a function called every frame
should show *some* drift in which callers reach it as the scene changes. A
flat, saturated pointer set that stops growing almost immediately is the
signature of a capture that stalled in near-real-time, not one that finished
early because there was nothing left to find.

**Consequence: never halt on a function called every frame (or close to it)
when the thing being measured is "what happens over N seconds of game time",
even briefly.** This is the same "enabled: False, log: True" non-halting
watchpoint the section above already recommends for "does anything read
this" - the addition here is that the failure mode for getting it wrong on a
*hot* function is not a crash or an obvious timeout, it is a plausible-
looking, fully-populated, wrong answer. Everything a texture bind needs to
identify itself typically lives on the texture object it operates on (a
residency flag, a last-uploaded timestamp, a handle passed to the display
list) - watch *that* object's fields with a non-halting read watchpoint
instead of halting on the function that touches it.
the watch missed something, not that nothing happened.

## Five traps from the absorb-overlay probe (2026-09-23)

Measured on PPSSPP v1.20.4, `UCUS98712`, while settling whether Pulse draws
its absorb hull overlay in play. The probe itself is in
[`cannon-quake-leachbeam.md`](../ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md),
"2026-09-23 (later)".

- **Two armed execution breakpoints already voided one conclusion.** An
  earlier run armed the per-craft update and the absorb gate together, and
  it logged 60 hits of the update and none of the gate. That result went
  into the docs as "the gate is never evaluated in play". It was the trap
  [above](#only-the-most-recently-added-execution-breakpoint-fires): the HUD
  calls that gate every frame. **To test whether code runs, watch a
  constant only that code reads.** A logged, non-halting read watchpoint on
  `0x08abf4a4` (a `10.0` whose only reader is `HullOverlay_Submit`) counted
  that function's executions at full speed, with no execution breakpoint
  at all. The log's `PC=` names the reader.
- **`input.buttons.press` never replies if a breakpoint halts the CPU inside
  its duration.** The reply comes when the press ends, and a stopped CPU
  never gets there. The client times out, and the CPU is left stepping with
  the breakpoint still armed. Send `input.buttons.send` with the button
  held, then again with it released: both reply at once.
- **A button held for twenty seconds registered no rising edge.** It was
  `hold(circle=True)`, with no release until the run timed out, and the
  absorb never happened. The same hold released after 0.15-0.2 s absorbed
  every time. Not explained. Release a held button promptly.
- **PPSSPP's own screenshot key beats `import -window root`.** The
  profile's `controls.ini` binds it (`Screenshot = 1-35`, keyboard `g`).
  Move the window onto the Xvfb canvas with `xdotool windowmove <id> 0 0`,
  then send `xdotool key --window <id> g`. PNGs land in
  `PSP/SCREENSHOT/` under the instance's own memory stick. They were real
  frames every time, where `import` had given black ones. Each shot carries
  the "saved" toast of the one before it, top centre.
- **A race left alone ends in the attract demo, and the player object goes
  stale.** Minutes after the field finishes, `InGame` stays the state name
  but the screen is a demo with other craft. Absorbs silently stop working
  because the craft pointer read earlier no longer drives anything.
  `psp-drive.py restart` brings back a live grid. Re-read
  `*(g_race_manager + 0x2c0)` after it, because the craft is reallocated.

## Probing a per-node test live, and pinning a target (2026-10-01)

Two recipes from `pulse-cull`, both on one boot of a Single Race:

- **A breakpoint at a predicate's entry, reading its operands.** `FUN_08902a98`
  (the box test) takes the box struct in `a0`; at each hit read the box, the
  view-projection (`0x08af2500`) and the world-times-VP the caller just stored
  (`0x08af24c0`), and on the *next* hit read the previous box's flag at `+0xc`.
  A call that reaches the function without having run the world multiply (the
  non-`Mesh` callers) leaves `0x08af24c0` stale - tell them apart by the node's
  class pointer at `node+4` (`0x08a6bd48` for `Mesh`), not by the layout.
  Roughly 12 hits a second; `each_hit` leaves the CPU stopped between them.
- **Pinning an opponent in front of the player** so a lock holds for as long as
  the capture runs: write its body position (`craft+0x1cc`, then `+0x30`) and
  zero its velocity at every `HudSight_Update` entry, so the HUD reads the pinned
  pose the same call. Give the player a LeachBeam by writing `10` to `+0x1bc` of
  its weapon record (`entity+0x4c`, `entity = *(*g_race_manager + 0x2c0)`), and
  fire the **unlocked** arm (`+0x16c = -1`, then bit `0x8000` of `+0x1b8`): the
  locked arm needs the target's matrix pointer at `+0x168` and halts the
  emulator without one. `scripts/psp-leach-sight-capture.py` is the whole thing.
- **The window shows a stale frame while the CPU steps frame by frame.** A
  screenshot taken during such a capture showed no sight for either weapon even
  though the HUD's own state read visible and locked; trust the memory reads and
  say so rather than reading the picture. Also: a race left idle for tens of
  minutes ends, the HUD is hidden and `HudSight_Update` never fires again - the
  probes then time out with no error. Reboot rather than debug it.


## Firing a weapon at a matched state: `psp-weapon-pair.py` (2026-10-01)

`scripts/psp-weapon-pair.py <weapon>` is the weapon-comparison harness: it restarts
the race, holds `cross` through the countdown (not a false start), takes the first
frame the player's throttle word (`craft+0x2b8`) is non-zero as GO, and ORs the
weapon's fire bit into the player's weapon record inside `Weapons_DispatchFire`
`--go-offset` frames later, so the craft is at a known state when it fires (speed
106.2 at `--go-offset 120`, on every one of more than ten restarts). Then it either
photographs the window at native 480x272 on chosen frames after the fire
(`--shots`), or breaks on a probe address for `--probe-frames` frames
(`--probe rocket|spawns|mine|bomb|shield`: `Rocket_Update`, `Psys_Spawn_q`, `Mine_PoseNode`,
`Bomb_Init`'s drop point and direction, and `ShipShield_Update`'s whole object plus its `dt`
in `f12`). Every frame row also carries `clock`, `g_ingame->0x40`, the one animation clock.
`--no-fire` is the control, `--fire-frame` fires a stationary craft, `--set-word
0x1ac=5` arms the Mine's round counter, `--camera` records the camera node's eye.
The matching `oag-game` side is `verification/scenarios/weapon-after-go.inputs`.

Four things it learned the slow way:

- **The race manager's player slot is the ship entity, not the craft.**
  `*(*(0x08b317b4) + 0x2c0)` has `+0x94` pointing at the craft `Ship_UpdateCraft`
  takes (and the craft's `+0x1c4` points back); its `+0x4c` is the weapon record.
  The craft has the throttle word and the body.
- **Do not nest a probe breakpoint in the dispatch loop.** It records its first
  frame and then waits out its timeout, because the dispatch breakpoint is still
  armed and takes the next stop. Leave the loop first.
- **The presented frame lags the stop by two frames.** The first screenshot that
  differs from a no-fire control is `fire+3`, on two boots (RMSE 0.068 -> 0.11).
- **An unmanaged Xvfb puts the SDL window off-screen** (one run: x 1760 of 1280).
  `--place-window` moves it to `160,88`; the window is the PSP's 480x272 doubled,
  `InternalResolution = 1` in the profile keeps the picture native, and
  `iShowStatusFlags = 0` hides the FPS counter that otherwise sits in every frame.


### Reading the camera at the same state: `psp-camera-pair.py` (2026-10-01)

`scripts/psp-camera-pair.py` is the same harness with nothing fired: it logs, for
every frame in `--from-go .. --to-go`, the camera node's eye in the body's own
frame, `|eye - craft|`, `g_camera_fov_degrees` and the speed, and photographs the
window at `--shots`. Its first use told which camera block the original flies on a
fresh profile (`OPT_CLOSE`, eye `(-11.25, +3.0)`): see
[camera.md](../ghidra/functions/psp-pulse-usa/camera.md#the-default-view-is-opt_close-measured-2026-10-01).
Start it on a **fresh `HOME`** to read a default and on a copied one to read a
choice; `psp-drive.py menu` gets a fresh home to a Time Trial unattended (run it
with `python3`, not `uv run --with websocket-client`, which needs the network).

### More ways to read the same launch (2026-10-01)

`scripts/psp-weapon-pair.py` also does, each in its own run:

- `--probe flare` / `--probe rolled` break on `ParticleSystem_DrawPoolSquares` and
  `ParticleSystem_DrawRolledQuads` and log, per live instance, its scale words, its view
  matrix and every pool particle's world position, size, colour, frame and roll
  (`particle-system.md`). Hit once per instance per frame.
- `--edram` writes both EDRAM framebuffers (`0x04000000`, `0x04088000`, 480x272 at stride
  512) beside each shot. **Needs the software renderer**: set `SoftwareRenderer = True` in
  the profile *with the emulator stopped* (it rewrites the file on exit). Alpha is the
  bloom's glow mask; the displayed buffer is the one whose RGB matches the screenshot. A
  `--no-fire` control beside it is what makes a difference mean something
  (`glow-mask.md`, "The weapon bodies").
- `--ge-dump-k K` requests `gpu.record.dump` at stop frame fire+K. The request has to be
  **sent while the CPU is stopped and answered after it resumes** - the synchronous call
  waits for a frame that cannot draw - so the script keeps the ticket and reads the reply
  after the loop. `psp-ge-dump.py census` reads the file; a replay of its command words up
  to a PRIM (`TEXLEVEL` `0xc8`, `BLENDMODE` `0xdf`, `STENCILTEST` `0xdc`, ...) is how the
  Bomb's ring and dome states in `mine.md` were read.
- `--detonate-bomb-at K --detonate-ahead D` moves the first laid Bomb `D` units ahead of the
  craft and runs its fuse out (`mine.md`, third pass). With `--probe` it fires inside the
  probe loop instead, **at the first probe hit after K frames** - and a `rolled` probe is not
  hit until something draws through `ParticleSystem_DrawRolledQuads`, so in one run it fired at
  frame 72.9 and not 8, the craft 70 frames further down the road. Read the logged
  `detonation` row for where the blast went.

### The explosion probes in `scripts/psp-wreck-capture.py` (2026-10-01)

The wreck harness (a grid opponent or the player put into `Ship_SetState(entity, 4)` at a chosen frame, then
`kNNN.png` shots) also takes, each in its own boot:

- `--pools K0:K1` breaks on every `ParticleSystem_DrawEmitterPool` from frame K0 to K1 after the call and logs
  each instance's resource (`+0x20`, with its render mode `+0xb8` and blend class `+0xc0`), scale words (`+0x28`),
  node matrix (`+0xf0`) and every pool particle's world position, half-size, colour, atlas frame, second point
  and raw bytes. Pools only: the sprite templates are not in them.
- `--templates K0:K1` breaks on `ParticleSystem_DrawParticle` (`0x089186bc`) and logs each template particle's first
  `0x90` bytes (position `+0x00`, size `+0x30`, colour `+0x34`, roll `+0x5c`, aspect `+0x64`).
- `--hits ADDR:K0:K1` logs every hit of any function with `a0`, `a1`, `f12` and the `0xb0` bytes at each pointer -
  it read `ShipShockwave_Update`'s object frame by frame (`ship-shockwave.md`).
- `--ge-dump-k K` and `--edram` as in `psp-weapon-pair.py`, above.
- `--hud` adds the HUD object's words (`g_hud` `0x08ab0838`: `+0x2c` flags, `+0x3c`, `+0x40` widget mask, `+0x168` the hidden
  flag `Hud_Hide` writes, `+0x274`/`+0x278`) to every frame row; `--timeout S` is the wall time allowed per `Ship_UpdateCraft`
  stop (default 30 s - a **software-rendered** emulator on an Xvfb runs a heavy scene at a few frames a second and the default
  then reads as "the craft stopped updating"; 200-600 s is safe). `--inject-frame` can be after GO (about 270 on a Single Race).
- A hit on `Ship_SpawnExplosionBig` (`--hits 088407b0:100:140`) gives the call's own frame, to order the ring (`ShipShockwave_Update`,
  `--hits 0885efc4`) and the particles (`--templates`) against it; each is its own boot and the frame phase varies by about one.
- **Only one execution breakpoint fires at a time** (`each_hit_any`'s note), so two streams of one boot cannot be interleaved; and a
  `memory.breakpoint.add` write watch on the HUD flag did not stop within 300 s of wall time (the write was about 316 emulated frames
  away on a slow software-rendered run, so that is not evidence it cannot fire); a breakpoint on the writing function (`Hud_Hide`,
  `0x0881a128`) is the route not tried yet and the better one.

The frames in `K` are counted from the call, one per `Ship_UpdateCraft` stop of that craft, and the probe's own
clock is the PSP cycle counter over 222 MHz / 59.94, so a boot-to-boot start of the same effect can differ by a
frame: compare an effect's shape against itself, not its absolute frame against another boot's. `--restart`
walks the pause menu's RESTART RACE; a `psp-drive.py restart` run first sits out the countdown, so the opponents
have already left the grid.
## The original's race after the finish: `psp-postrace.py` (2026-10-01)

`scripts/psp-postrace.py` gets a whole race finished without steering and logs what follows the line: it gives
the player's weapon record the Autopilot pickup (fire bit `0x1000`) and raises the record's countdown
(`+0x148`) to `1e9`, so the original's own autopilot flies the laps at 1.00x, then breaks in
`Weapons_DispatchFire` once a frame on the final lap and logs every craft (state, control record,
driver, body, crossings), the manager's mode state, the HUD's hidden flag and the camera object, with
photographs. See [after-the-finish.md](../gameplay/after-the-finish.md) for what it found.

- **`--disarm-x X`** stops the pickup and releases thrust as the craft passes `x` on the start straight (`--disarm-z` pins
  the straight), so only the game's own post-finish driver is left. Without it a finish reads the same, but the pickup
  could be what flies the craft.
- **`--probe setstate|ctl|scale`** swaps the per-frame breakpoint for `Ship_SetState`, or for a **break-on-write**
  memory watchpoint (`memory.breakpoint.add ... enabled=True, write=True`) on the player's control record or on
  `craft+0x1d4`, and logs the writer's `pc` and `ra`. A watchpoint armed with `enabled=True` stops the CPU on the write;
  the stop arrives as a bare `cpu.stepping` with an unknown `pc`, so the wait takes any stop and reads `cpu.status`.
- **A resume can be answered by a stop the wait never sees**: `cpu.status` then says the CPU is stopped at the
  breakpoint's own address while `wait_for_break` times out. The script's `hits()` yields that stop instead of raising.
- **The weapon pickup's autopilot runs only 4.95 s unless its timer is raised**; the timer lives at `record+0x148`.
- **`--state-every N`** reads the front end's state name every N frames (default 30; `1` pins the frame `Race End Photo` is entered on:
  `F+61`). `psp-wreck-capture.py --ui-state` logs the same plus the manager's mode state, which is how a wreck was shown to stay on `InGame`
  for 17,800 frames (2026-10-02). After a capture ends the emulator is left running; a one-second poll of `state_name()` plus a
  photograph on each change watches it for minutes at no cost.
- **From `Race End Photo` the pause menu does not exist**: `psp-drive.py restart` and the script's restart walk both
  press into the end-race panels. `cross` five times lands on `Main Menu` (`Race End Photo`, `EndRace Results`, `Rewards`,
  `Menu`), then `psp-drive.py menu`.
- A second `x` on a long race is not unique: the circuit passes the same `x` away from the start straight, and a
  disarm there flies the craft into a wall. Pin `z` too.

## A real absorb, and reading the mask without a mid-frame halt: `psp-absorb-frames.py` (2026-10-01)

`psp-fire-weapon.py` sets a bit in the fire-request word, and the absorb is not one: it is
`Ship_AbsorbHeldPickup` (`0x08844ec4`), which reads the controller's absorb byte (**circle**) while the craft holds a
pickup. So `scripts/psp-absorb-frames.py` does the two things the 2026-09-23 probe did, as a tool: write `0` into the
player's weapon record (`*(*(*(0x08b317b4) + 0x2c0) + 0x4c) + 0x1bc`, the held-weapon slot) **at a
`Weapons_DispatchFire` breakpoint**, and hold circle through `input.buttons.send`. The window is the race clock
`player+0x830` minus the stamp `player+0x878` (`-10.0` until the first absorb, so a stale stamp from an earlier run is not
mistaken for this one). `--edram` writes both framebuffers and the bloom layer at every `--every`-th frame of the
window, `--ge-dump` records one frame's GE list inside it.

- **Read EDRAM at a frame boundary, in both buffers.** A halt at an arbitrary moment (`brk` after a sleep) can land
  between a draw and the pass that overwrites it: boot 1 of this tool's own run read the absorb overlay's `255` over the
  hull in one buffer at 0.58 s and 0.68 s, and `4` in every frame-boundary read. A breakpoint in a once-a-frame function
  (`Weapons_DispatchFire`) stops between frames, where both buffers are complete.
- **The software and hardware backends disagree about the stencil.** The same absorb draws a white-hot hull on the OpenGL
  backend and a modestly brightened one on the software renderer; see
  [glow-mask.md](../rendering/glow-mask.md), "The hull overlay's mask is wiped". Say which backend a comparison used.
- **`gpu.record.dump` needs the CPU running.** With the once-a-frame breakpoint still armed the next frame never
  completes and the call times out (`no reply to gpu.record.dump`): remove the breakpoint, resume, call, stop again.
- **A grant by writing the held slot works only for the absorb.** Granting `held = 10` (the LeachBeam) the same way and
  holding fire (`square`), once straight away and once after a 1 s wait for a lock, did **not** halt PPSSPP and did not
  produce a beam either: the slot cleared within half a second, `craft+0x85c` (the lock target) stayed `-1` and
  `craft+0x860 & 1` stayed clear on the grid with the field ahead. The halt is therefore the fire-*bit* route's
  (`+0x1b8 |= 0x8000`, which skips the construction a real fire does, `bad-memory-access-halt.md`); a real LeachBeam
  needs `Ship_AcquireLock` to have found a target, which a held-slot write did not cause. Not solved.
- **The craft drifts off the grid if left alone.** After a couple of minutes the player's position changed (the race
  clock keeps running), so a matched-pose capture follows a `psp-drive.py restart` within a few seconds.

## The original's pre-race flyby: `psp-flyby.py` (2026-10-01)

`scripts/psp-flyby.py` walks the front end into a race exactly as `psp-drive.py menu` does (or `--restart`s a live
one through the pause menu) and **watches the load instead of sitting it out**. See
[race-intro.md](../gameplay/race-intro.md) for what it found.

- No flag: once a second, the front-end state name, the PSP cycle counter, the camera object's mode word and a 480x272
  photograph. This is how the flyby was first seen: `InGameTrackDescriptionScreen` for 1,565 frames with the camera
  moving.
- **`--frames N`** breaks at `0x08882e9c` (inside the render-view publisher, which only runs while the intro's camera
  pass does) once per frame and logs the camera node's tripod pose and field, the animation's own clock, `AnimEnd`, the
  pause flag, the intro's counter, `mode+0x7c8/+0x7cc` and `g_ingame+0x40`. **`--after M`** then switches to
  `Weapons_DispatchFire` for `M` frames **without a gap** (a hit timeout in between runs the emulator free for 20 s
  and the countdown with it), and **`--skip-frame K`** presses `--skip-button` (Cross) at hit `K`.
- **`--intro-trace N`** logs `N` calls of `RaceMode_UpdateIntro` (`0x08829e6c`): the mode object's `+0x40` byte, the
  substate, the counter and `g_game_mode`. It is what showed a fresh menu-walk load reaching the intro already in
  its fade-out substate on two circuits.
- **`--hit-timeout S`**: a cold first load of a circuit can take longer than the default 20 s to reach the camera
  pass; with the timeout spent, the script concludes "no flyby" when it only missed it. Two early runs read exactly
  that way. Raise it.
- **A write watchpoint left armed hangs `input.buttons.press`**: the CPU is stopped on the next write and nothing
  answers until the watchpoint is removed (`memory.breakpoint.remove`, in a `finally`).
- **Only the most recently added execution breakpoint fires**, as `Debugger.each_hit_any` says: the flyby capture needs
  the publisher's one breakpoint and reads everything else from memory with the CPU stopped.
- **The publisher's breakpoint address `0x08882e9c` is mid-function**: a breakpoint on the function's own entry
  (`0x08882cbc`) never fired though the function ran every frame of the flyby (cause not found). The write watchpoint on
  the render view's translation (`+0x70` of `*(0x08ab10b0)`) is what gave the mid-function pc, and a breakpoint on that
  does fire.
