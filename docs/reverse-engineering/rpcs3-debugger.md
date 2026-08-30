# Driving RPCS3 from its GDB stub

The PSP side of this project drives PPSSPP from a JSON-over-websocket debugger
that reads memory, sets breakpoints *and presses buttons*
([ppsspp-debugger.md](ppsspp-debugger.md)). The question this page answers is
how much of that is reachable on the PS3 side, measured against
`RPCS3 v0.0.42-19777-3be5aa99 Alpha | master` (the build string `RPCS3.log`
prints on its first line) and WipEout HD / Fury (`BCES-00664`) on 2026-08-19.

**The short answer: all three, by routes PPSSPP does not need.** A script boots
the title with no window on anyone's desktop, walks its front end into a race,
holds thrust, and screenshots the result - and separately reads and writes guest
memory at the Ghidra corpus's own addresses. What it gets there with is a
kernel-level virtual pad (RPCS3 has no input API), the game's own `printf` (the
GDB stub answers nothing while the target runs), and a virtual display
(`--headless` cannot reach the front end at all). Breakpoints are answered `OK`
and are not usable in practice; see below.

Everything here is behind `scripts/rpcs3_debugger.py`, whose module docstring
carries the same trap list in the form a caller needs it.

## Booting without a human, and without a window

**`--headless` boots and cannot reach the front end.** This is the first thing
to get right, because the way it fails looks exactly like something else. The
null renderer starts, the game prints as far as

```
LOADING SCREEN TYPE type == 0
TROPHY: Checking free disk space... 41942784
```

and then stops for good, with the main thread parked inside `cellGameDataCheck`
(`HLE:0x0144179c`, the PC every sample lands on) and the process burning
200-330 % CPU. Twenty scripted button presses over four minutes changed nothing,
because there was no screen to receive them. `Use native user interface: false`
does not help either. **`--headless` is for a boot-and-read-memory session and
nothing more.**

What works is a real renderer on a display that is not the user's:

```sh
python3 scripts/rpcs3-drive.py display        # Xvfb :77, TCP-addressable
uv run --with evdev python3 scripts/rpcs3-drive.py race --drive 20 --shots
```

`--load-shots N` adds N evenly spaced screenshots across the load window, which
is the only way to catch a screen that is up for none of the run either side of
it - the loading screen is the case it was added for. See
[hd-loading.md](../formats/hd-loading.md).

Measured on that path: `Main Menu` in about 45 seconds, a race running on
Talon's Junction about 80 seconds after that, and screenshots that are real
frames rather than black.

Three details in the display that each cost a run:

- **Xvfb must listen on TCP** (`-listen tcp -nolisten unix`, addressed as
  `127.0.0.1:77`). A sandboxed session may be unable to write `/tmp/.X11-unix`,
  and then the unix socket never appears while the server itself is fine - Qt
  reports `could not connect to display :77` and RPCS3 exits before logging
  anything, so the previous run's `TTY.log` is still sitting there looking like
  a fresh boot that stalled.
- **`QT_QPA_PLATFORM=xcb`, and drop `WAYLAND_DISPLAY`.** RPCS3 is Qt; on a
  Wayland session it will not look at an X display otherwise.
- **The pad has to exist before RPCS3 does.** It binds pads when it enumerates
  devices and does not rescan.

Two more things about the invocation:

- **It must be the layer-1 decrypted image.** `games.yml` already registers
  `BCES00664` against `hdfury-ps3-eu-dec.iso`; the encrypted twin beside it
  reads as noise. See [ps3-disc.md](../formats/ps3-disc.md).
- **A second launch while the first process lives exits quietly**, observed
  twice: the new process is gone within seconds and port 2345 stays bound to the
  old one. Whether that is an explicit single-instance lock or just the failed
  bind was not established; the symptom is what matters, and it is that a stale
  process looks exactly like a broken command line.
- **A terminated run poisons every run after it, and on a newer build the
  failure changed shape.** `rpcs3-drive.py`'s `Session.__exit__` calls
  `proc.terminate()`, and a terminated RPCS3 leaves `~/.cache/rpcs3/RPCS3.buf`
  behind. On build `0.0.42-19777` the next launch does **not** exit quietly: it
  puts a modal dialog on the virtual display ("Another instance of RPCS3 is
  running"), so the process stays up, `TTY.log` stays empty, and every wait
  times out against what looks exactly like a game that booted and stalled. Two
  runs were lost to it before a screenshot of the display showed the dialog -
  which is the general lesson, since the display is the only channel that
  carries a dialog. `Session.__enter__` clears the lock now when no RPCS3 is
  running.

`just launch-hdfury-ps3` defaulted to the *encrypted* image until 2026-08-19 and
so could never boot; the recipe now points at the decrypted one.

## `TTY.log` is the state channel, and HD names every screen on it

RPCS3 writes the game's own `printf` to `~/.cache/rpcs3/TTY.log`, and HD logs a
line on every front-end transition:

```
Switching Screen "Main Menu" to "Campaign Selection"
```

That single line is the whole front-end state machine, free, and it is what
every wait in `rpcs3-drive.py` keys on - there is nothing to poll and no cost.
Nothing on the PSP side has an equivalent; `psp-drive.py` has to read the state
machine out of memory at `G_STATE_MACHINE`.

**Its limit, said out loud: it goes quiet during a race.** Loading prints
plenty (`Loading track model Data\Environments\Talons_Junction\track.rcsmodel`,
`Track Vex Allocated 4757KB`, `Loading Screen Finished`, `MemoryStatus: Game
Running`), and then nothing per-frame. A driven lap is observable by screenshot,
not by `TTY.log`.

### The walk into a race is six taps of cross

Measured 2026-08-19, from a cold boot, with no d-pad at all - every step is the
default highlighted row:

| # | From | To |
| --- | --- | --- |
| 1 | `Main Menu` | `Campaign Selection` |
| 2 | `Campaign Selection` | `Grid Selection Fury` |
| 3 | `Grid Selection Fury` | `Cell Selection` |
| 4 | `Cell Selection` | `Team Selection` |
| 5 | `Team Selection` | `Launch Game` |
| 6 | `Launch Game` | `InGame` |

`circle` goes back (`Switching Screen "NULL" to "Campaign Selection"`). `up` and
`down` produce no `TTY.log` line, which is expected - moving a highlight is not
a screen change - and so are *not* evidence either way about whether the d-pad
arrives.

### `Session.navigate()`'s taps have no confirmation, and a dropped one fails silently

**2026-08-30, cost a wrong "genuine Zone race" claim before it was caught.**
`navigate()` presses each button in a plan with a fixed settle
(`self.tap(button, settle=1.2)`), unlike `walk_to_race()`'s own `cross` steps,
which retry against `TTY.log` until the screen actually changes. Moving a
carousel highlight produces no `TTY.log` line at all (see above), so there is
nothing to confirm against - a tap sent while the previous one is still being
processed is silently lost, and the plan's own `print("nav %s at %s" ...)`
line fires whether or not the press actually landed. Four `right` presses
meant to cycle Racebox's `Single Player` Mode list from its default to `Zone`
landed on `Eliminator` (one entry short) in a live run, with nothing in the
log to say so - only a screenshot after the fact caught it. **Verify with a
screenshot before confirming a multi-step d-pad sequence, especially under a
slower decoder** (interpreter mode makes a drop more likely, not less);
counting `nav` log lines is not evidence the presses all landed.

The same investigation measured Racebox's actual on-screen Mode order, which
is not simply the XML's own declaration order by name - `racebox_definition.xml`
names the fourth entry `Tournament`, but its displayed text is `ELIMINATOR`:

| presses from default | shown |
| --- | --- |
| 0 | `Arcade` |
| 1 | `Time Trial` |
| 2 | `Speed Lap` |
| 3 | `Eliminator` (XML idstring `Tournament`) |
| 4 | `Zone` |

## The GDB stub is real, and needs no special build

`config.yml` ships `GDB Server: 127.0.0.1:2345` and the stub honours it with
`Debug Console Mode: false` and a stock AppImage - no rebuild, no flag. What it
answers, all measured:

| Packet | Result |
| --- | --- |
| `qSupported` | `PacketSize=1200`, so ~590 bytes per memory read |
| `qfThreadInfo` | the PPU thread list (14 on HD at the loading screen) |
| `Hg<tid>` | selects a thread for `g` |
| `g` | 556 bytes: 32 GPRs, 32 FPRs, PC, MSR, CR, LR, CTR, and a trailing pair |
| `m<addr>,<len>` | memory read |
| `M<addr>,<len>:<hex>` | memory write - **honours page permissions**, `E03` on a read-only page, `OK` and readable-back on a writable one |
| `Z0` / `z0` | software breakpoint set/clear, always `OK` (see the trap below) |
| `Z2` | **not implemented** - empty reply, not `OK`. No write watchpoints on this build; see below |
| `vCont;c` | resume |
| `vCont;s:<tid>` | single-step one thread |
| `\x03` | interrupt; replies `S05`, sometimes seconds late |

**`Z2` (write watchpoint) is not implemented.** Measured 2026-08-26 arming one
on a live `RenderManager` instance mid-race: the reply was an empty packet,
not `OK` and not `E`-prefixed either - the stub simply does not recognise the
packet type. `Z0` software breakpoints are the only stop mechanism this stub
offers; finding what writes to a particular address needs a different route
(bracket it between two known call sites and read the value at each, or work
backward from a candidate writer's own decompilation) rather than trapping
the write itself. See
[renderer.md](../ghidra/functions/ps3-hdfury-eu/renderer.md#runtime-verified-118-real-draws-two-object-families-no-watchpoint-support)
for the investigation this came out of.

**Not a gap in this build specifically - upstream has never implemented it.**
Checked 2026-08-30 against `master`'s own `rpcs3/Emu/GDB.cpp`, fetched directly
from GitHub rather than assumed: `cmd_set_breakpoint`/`cmd_remove_breakpoint`
handle `type == '0'` only, and every other type (`Z1`-`Z4`, watchpoints
included) falls through to `return send_cmd_ack("")` - the identical empty
reply measured here, on the exact same code path. GitHub's issue and PR search
for "watchpoint" against `RPCS3/rpcs3` returns zero results either way - nobody
has filed for this. GDB server support shipped in v0.0.3 (2017-07-20); the
`Emu/GDB.cpp` commit history since is refactors only, no protocol additions.
**So a newer build, or the `rpcs3-git` AUR package (also tracks `master`), buys
nothing here** - this has been Z0-only for the emulator's entire life. Adding
`Z2` would mean patching RPCS3 itself: the least invasive route is extending
the same interpreter dispatch path `ppu_breakpoint` already uses for `Z0` to
also check store instructions against a registered watch address, which stays
within the same interpreter-only limitation this page already documents for
breakpoints - realistically a few hours to a day of emulator-side work plus an
ongoing fork to maintain across updates.

### A patched build exists, `Z2` genuinely works on it, and the patch is tracked

**2026-08-30: built and verified live against a real HD/Fury race, then moved
into version control for reproducibility.** The patch itself -
[`scripts/patches/rpcs3-gdb-write-watchpoints.patch`](../../scripts/patches/rpcs3-gdb-write-watchpoints.patch) -
is tracked in this repo; RPCS3's own source is not (a diff against GPL-2.0
code carries no licensing weight the way vendoring the source would, and it
is not this project's source regardless). Build it with:

```sh
just build-rpcs3-watchpoints
```

which runs [`scripts/build-rpcs3-watchpoints.sh`](../../scripts/build-rpcs3-watchpoints.sh):
clones RPCS3 at the exact commit the patch was verified against
(`3ef20ebb0`, `RPCS3_GIT_VERSION: 19884-3ef20ebb`) into
`data/tools/rpcs3-watchpoints` (gitignored, several GiB - this is a large
checkout, not a quick one), applies the patch, and configures/builds with
the flags below. Binary ends up at
`data/tools/rpcs3-watchpoints/build/bin/rpcs3`. `--clean` starts fresh;
`--ref <commit>` rebases onto a newer RPCS3, which needs re-verifying the
patch still applies and re-running the end-to-end test below, not just a
rebuild.

`-DUSE_LTO=OFF` and a `-Wl,--start-group`/`--end-group` wrap around the
vendored ffmpeg archives in `3rdparty/CMakeLists.txt` (part of the patch,
Linux-only via a `PLATFORM_ID` generator expression) are local
build-environment workarounds for GNU `ld` on this host, not part of the
feature - upstream's own CI almost certainly links with `lld` and never hits
either, and both are harmless to apply on a system that already uses `lld`.
`-DHAS_MEMORY_BREAKPOINTS=ON` is the real prerequisite: a genuine,
first-class CMake option (`option(HAS_MEMORY_BREAKPOINTS ... OFF)` in the
top-level `CMakeLists.txt`) that **defaults off** and gates the entire
memory-write-hook code path in `vm.h` - including RPCS3's own pre-existing
Qt memory-breakpoint feature, which is silently compiled out on every
default build for the same reason `Z2` was never reachable. Building with it
on also surfaces a real, tiny, pre-existing upstream bug (`vm.h` uses
`CHAR_BIT` without including `<climits>`), fixed by the same patch.

The patch adds a small range-checked watch registry alongside the existing
Qt one (not folded into it - a GDB watchpoint has to fire on any write
overlapping `[addr, addr+len)`, where the Qt feature only matches a write's
exact start address), wired into `Z2`/`z2` and reusing the identical
`dbg_pause` + `check_state()` + `gdb_server::pause_from()` stop/notify path
`Z0` already uses. Verified end to end with a real GDB client against a live
Main Menu session: `Z2` answers `OK`, a watched write stops execution with a
real `S05` (twice, proving reproducibility, not a one-off), `z2` removes it
and further writes produce silence - client-side transcript and
`RPCS3.log`'s own `Write watchpoint hit` lines matched one-for-one, and the
patch file (re-verified with `git apply --check` against a clean checkout of
the pinned commit before it was committed here) applies cleanly from
scratch, not just to the working tree it was developed against. Full
engineering log, including three dead ends that were each root-caused rather
than papered over (an `asmjit` per-opcode JIT-caching trap that looked like
the real fix but wasn't, and the actual cause - the CMake flag defaulting
off - found only after that), is this project's own git history for the
commit that added the patch file.

**Same scope limitation as `Z0`, not a new one**: only fires under `PPU
Decoder: Interpreter (static)` - the LLVM recompiler emits stores as
generated machine code that never calls `vm::write()`, so there is no choke
point to hook, the same reason `ppu_breakpoint()` itself refuses to arm
under LLVM.

**A real caveat, root-caused rather than guessed at**: a watch broad enough
that *several threads* hit it in the same instant can leave a "losing"
thread's `dbg_pause` flag stuck forever - `gdb_thread::pause_from()` is
idempotent and only the first thread to report in a stop cycle gets its flag
cleared by `cmd_vcont`, so any other thread that set the flag in the same
instant re-triggers a stop on every future resume regardless of whether
anything is still armed. This is a pre-existing property of
`cmd_vcont`/`check_state()`, not specific to this patch - it is the same
"queues one stop reply per thread" shape this page already documents for a
`Z0` breakpoint hit by several threads. **Prefer a narrow, specific watch**
(a single known address, not "most of main memory") - a single-address watch
on `_gcm_intr_thread`'s own `0x16a2d58` passed clean end to end with no such
issue, where a ~1.5 GiB watch spanning dozens of active threads produced
stray extra stops that were not a removal bug.

`~/.config/rpcs3/config.yml` is shared with the system `rpcs3-bin` install
this page's other measurements use - switching `Core: PPU Decoder` to
`Interpreter (static)` to use this patched build affects that install too
until it is switched back. Every process-management and one-shot-GDB-session
trap this page already documents for the stock build applies identically
here; nothing about the patch changes them.

Not implemented, and this is the first thing to get wrong: **bare `c` and `s`
return an *empty packet*.** An empty packet is indistinguishable from an
immediate breakpoint hit if the client is not looking for it, so a session built
on `c` produces a stream of convincing, entirely fictional "hits" at a steady
41 ms apart. That is exactly what happened here before `vCont?` was asked - it
answers `vCont;c;s;C;S`, and only those work.

**Guest addresses are the ELF's own virtual addresses, unrebased.** `m10000,10`
returns `7f454c46020201 66...` - the decrypted EBOOT's ELF header - so every row
in [`names.tsv`](../ghidra/functions/ps3-hdfury-eu/names.tsv) is usable as
typed. The register dump's layout was settled the same way: LR at byte 532 lands
at `0x00011114`, inside the code range, which no other split does.

### The stub answers nothing at all while the target runs

Measured directly: after `vCont;c`, an `m` packet gets **no reply** - not a slow
one, none, until something stops the target. The stub services packets only
while stopped. PPSSPP's equivalent trap is a *cost* (520 ms running against
0.3 ms stepping); here it is a hard wall, and a client that polls a running
target simply hangs. `\x03` is the only thing a running target answers.

### A round trip costs 41 ms *while paused*, and that number decides the design

With `TCP_NODELAY` set, a packet round trip against a stopped target is about
41 ms; without it, 83 ms. So a session gets roughly **24 operations a second**,
against PPSSPP's 0.3 ms per read while stepping. Two consequences:

- Read a whole struct in one `m`, never field by field.
- Nothing per-frame is affordable. Injecting input at 60 Hz would need two or
  three packets a frame and would run the game at about 8 fps even if the
  breakpoints worked.

The `\x03` stop reply is usually immediate but not always: it arrives when the
stub next reaches a scheduling point, one probe here waited 25 s and never got
it, and the connection is unrecoverable if a client gives up on it and carries
on sending. Read it on its own generous timeout and treat its absence as fatal
to the session rather than something to retry around.

## The trap set

Each of these cost a full 40-second reboot to find, and three of them leave the
emulator in a state that reads like a different bug.

### Connecting pauses emulation

The stub pauses the moment a client attaches (`Emulation is being paused...` in
`RPCS3.log`). A connect is never passive; `resume()` before timing anything.

### The server thread dies for good on disconnect, but keeps the port

Closing the client socket kills the GDB thread with `SIG: Thread terminated due
to fatal error: Tried to read char, but no data was available`. It does not
listen again - **and the socket stays bound to the dead process**, so a later
launch of RPCS3 silently gets no debugger while `ss -tln` still shows 2345
listening and every packet times out. One debug session per emulator launch, and
kill the old process before starting the next one.

### Disconnecting while paused freezes the emulator

Same disconnect, different state: if the target was paused, the log adds
`Emulation has been frozen!` and the game never resumes. Disconnecting while
*resumed* leaves the game running, debugger-less but alive. So a script that
means to hand a live emulator back should `vCont;c` before dropping the socket.

### `Z0` breakpoints fire, but only under the interpreter

**Settled 2026-08-19**, once a race gave an address that provably executes. This
page previously carried it at confidence 35, untested; it is now measured.

**Under `PPU Decoder: Interpreter (static)` they work.** Mid-race, a breakpoint
at `Collision_ProcessPairs` (`0x00038428`) stopped the main thread with a full
register set - `lr = 0x000f88c0`, the unnamed caller one frame up - and a
separate run stopped in `FwMutex_Lock` (`0x003931e0`). Booting under the
interpreter costs about 75 seconds rather than 30 and runs the race perceptibly
slower, but the front-end walk and the race both work.

**Under the default `Recompiler (LLVM)` they do not**, and the stub still
answers `OK`, so a session built on them silently never stops. **HLE import
stubs never fire under either decoder** - `ppu_loader` logs the game's `sys_io`
imports at boot (`[cellPadGetData] (0x8b72cda1) -> 0x759a44`, and `0x759a44`
really does hold `li r12, 0`), but they are not PPU code paths.

### The stub parks a thread at a breakpoint without announcing it

**This is the most expensive trap on the page** - it cost four runs and produced
two confident, entirely wrong conclusions before an in-race control caught it.

A `Z0` breakpoint stops the thread that reaches it, but RPCS3 does **not**
always send a stop reply for that thread. `wait_for_stop()` therefore blocks
until its timeout and reports nothing, which reads exactly like "this address
never executes".

Measured. A breakpoint on `Physics_StepWorld` (`0x000f8610`) returned nothing in
90 seconds of a live race - screenshot-confirmed as lap 1 of 3 with the clock
running and thrust held. Arming `FwMutex_Lock` in the *same* session then
stopped in 0.04 s and reported:

    physics step (tid 01000000, r3=0x3056cea0)
    FwMutex_Lock (tid 0100000c, r3=0x00b6caf8)
    FwMutex_Lock (tid 01000017, r3=0x32903730)

Thread `01000000` had been sitting at the physics breakpoint the whole time.
The only reliable question is "is any thread's PC at this address", and asking
it means stopping the target first: arm, resume a slice, `pause()`, then walk
every thread's registers. `Debugger.wait_at()` does exactly that and is what
breakpoint work should use.

**A negative from `wait_for_stop()` is not a negative.** Two readings written
during this session - that the `Collision_*` functions are "contact-gated, not
per-frame", and that the physics tick "does not run during a race" - were both
produced this way and are both wrong. The first is separately ruled out by the
disassembly: `bl 0x00038428` at `0x000f88bc` is straight-line code, so pair
processing runs on every physics step. See
[physics.md](../ghidra/functions/ps3-hdfury-eu/physics.md).

### Breakpoints do re-arm, and that was never the problem

Three consecutive hits on `FwMutex_Lock`, 0.04 s each, resuming between them
without removing or re-adding anything. Re-arming is not required and is not
where the once-then-never pattern came from.

What *does* break is arming after an interrupt-pause while stop replies are
still queued: `remove` then `resume` then `pause()` then `add` timed out
outright. Two rules follow, both cheap:

- **Arm on the attach-pause**, before the first `resume()`. Connecting pauses
  the target anyway, so drive the pad into position *first* and attach last.
- **A breakpoint on a function several threads run queues one stop reply per
  thread.** The first memory read afterwards then consumes a *stop reply* as its
  answer, everything after is off by one, and the session dies on a timeout that
  looks exactly like a hung emulator. `Debugger.drain()` exists for this; call
  it after every breakpoint stop, and give it a longer settle than the default
  before any `add_breakpoint`.

### What a stopped thread's PCs do and do not tell you

Before a race was reachable, all 14 threads `qfThreadInfo` listed sat at
identical PCs across a 1-second and a 10-second window of real running, measured
twice with interrupt and resume separately confirmed. The reading, at confidence
65, was that those are PPU threads parked in HLE waits while the 200-330 % CPU
the process burns is RSX and SPU work the thread list does not cover. That
reading stands, and the boot section explains what they were waiting on.

## The address space during a race is 480 MiB in six pieces

Probed a slice at a time with the stub, mid-race, 32 of 256 16-MiB slices
answer a read:

| Range | Size | What it is |
| --- | --- | --- |
| `0x01000000` | 16 MiB | main memory; the EBOOT's own code and data live below it |
| `0x10000000` | 16 MiB | - |
| `0x30000000`..`0x36ffffff` | 112 MiB | **the game heap** - every pointer the physics breakpoint handed back lands here |
| `0x40000000`..`0x45ffffff` | 96 MiB | - |
| `0x50000000` | 16 MiB | - |
| `0xc0000000`..`0xcfffffff` | 256 MiB | RSX local memory |

**A 512-MiB probe misses the heap entirely**, which is how an earlier pass here
concluded the mapped space was 26 MiB: `0x30000000` is at 768 MiB. Probe the
whole 4 GiB or find nothing.

What that costs, stated because it decides the approach: at the stub's 13.2
KiB/s a blind scan of the heap alone is about **2.4 hours**. Pointer-chasing
from a breakpoint is the affordable route, and a scan is a last resort rather
than a plan.

## Input: two silent gates, both now closed

RPCS3 has no input API - not in the GDB stub, not on the command line. The only
route is to hand the *kernel* a gamepad and let RPCS3's evdev handler read it
like any other device, which needs no window and no focus and so works in
`--headless` at full speed. That is
[`scripts/rpcs3_pad.py`](../../scripts/rpcs3_pad.py), and the reason it leads
with a preflight is that **both things that can stop it fail silently.**

```sh
just rpcs3-preflight     # says which of the two is wrong, and the exact fix
```

### Gate 1: `/dev/uinput` is root-only until a udev rule says otherwise

Nothing on a stock Arch install creates a rule for it, so the node is
`0600 root:root` and a harness dies at the first press with a bare
`PermissionError`. The persistent fix, and the one this machine now carries:

```sh
echo 'KERNEL=="uinput", GROUP="input", MODE="0660"' | sudo tee /etc/udev/rules.d/99-uinput.rules
sudo modprobe uinput
sudo udevadm control --reload-rules && sudo udevadm trigger /dev/uinput
```

`ls -l /dev/uinput` should then read `crw-rw---- root input`, and the account has
to be in group `input` (`sudo usermod -aG input $USER`, then log out and back
in). `modprobe` and `chmod` on their own work until the next reboot; the rule is
what survives one. `preflight` checks the module, the node, its group and the
caller's membership separately, and prints only the fixes that apply.

### Gate 2: with no input profile, RPCS3 binds a keyboard and eats every press

This is the one that looks like a broken pad. With no
`input_configs/global/*.yml`, the log says `Input configuration empty. Adding
default keyboard pad handler` - and `--headless` has no window to deliver keys
to, so every press vanishes with no error anywhere. Three lines fix it, and
RPCS3 fills every other key from its own defaults:

```yaml
# ~/.config/rpcs3/input_configs/global/oag.yml   (`just rpcs3-input-config`)
Player 1 Input:
  Handler: Evdev
  Device: OpenAntiGrav Virtual Pad
```

Select it with `--input-config oag`, which is what
`just launch-hdfury-ps3-headless` now passes.

### What that gets you, measured

With the pad held open *before* the emulator starts - RPCS3 binds pads when it
enumerates devices - a headless launch logs, in order:

```
{Pad Thread} Input: Loading input configuration: '.../input_configs/global/oag.yml'
{Pad Thread} evdev: Capability info for OpenAntiGrav Virtual Pad: rumble=1, motion=0
{Pad Thread} Input: Pad 0: device='OpenAntiGrav Virtual Pad', handler=Evdev
{Pad Thread} Input: Evdev device 0 connected
```

The device itself has 13 buttons and 6 axes - d-pad as `ABS_HAT0*`, both
sticks, `BTN_SOUTH`-family names rather than the legacy aliases - and declares
DualShock 3 vendor and product ids. **Only the Evdev handler has been tested**,
and RPCS3 logged `VID=0x0, PID=0x0` through it, so those ids are not yet
observed doing anything; they are there for the SDL handler, which is untested.
Verified: the node appears in `/proc/bus/input/devices`, evdev enumerates it,
and presses, d-pad and stick writes all land without error.

**Kill a previous pad process before starting another.** The profile names the
device by *name*, so two live pads are two devices called the same thing and
RPCS3 binds one of them - which can be the stale one, with nothing in the log
to say which it took.

One more thing about running any of this from a worktree: `data/` is gitignored
and does not travel, so `just launch-hdfury-ps3-headless` there resolves its
default relative image path to a file that isn't present and RPCS3 answers
`Invalid file or folder`. Pass an absolute path, or run it from the main
checkout.

**Verified that a press reaches the game**, on the real-renderer path above:
`cross` at the Main Menu produces `Switching Screen "Main Menu" to "Campaign
Selection"` in `TTY.log`, and six of them start a race. Under `--headless` the
same presses change nothing - not because input fails, but because the game
never arrives at a screen (see the boot section). That distinction is the whole
reason this page separates the two.

### Kill a previous pad before starting another

The profile names the device by *name*, so two live pads are two devices called
the same thing and RPCS3 binds one of them - which can be the stale one, with
nothing in the log to say which it took.

### The two routes not taken

- **The keyboard pad handler** is dead in `--headless` for the reason gate 2
  describes, and on a virtual display this project's own experience with
  `xdotool` against a winit window is that the effect could not be confirmed
  either way.
- **Writing the game's pad buffer over GDB** is ruled out twice over: it needs a
  breakpoint on the `cellPadGetData` return, which is the one address proven not
  to fire, and the 41 ms round trip would cap it near 8 fps even if it did.

## Savestates work, drive off the pad, and are not worth adopting

They were expected to be the payoff - `rpcs3 --savestate <path>` boots straight
into one, which would have collapsed the 130-second cold walk to a flag. Tried
end to end on 2026-08-19 and **not adopted**, for the same shape of reason the
PSP `.ppst` was not: it does not do what the boot does, and it does not do it
faster.

### Creating one is a pad walk through RPCS3's own overlay

There is no savestate hotkey that lands here - `xdotool key` reaches nothing,
with or without `windowfocus`, and `windowclose` does not close the window
either, so RPCS3's Qt frame ignores synthetic X input on this display. What does
work is the **PS button**, which opens RPCS3's own home menu overlay and takes
d-pad and cross from the virtual pad:

```
{Pad Thread} Input: opening home menu...
{Overlay Input Thread} Input: SetIntercepted: pads=1, keyboards=1, mice=1
{Overlay Input Thread} RSX: User selected 'SaveState' in ''
```

The menu is nine items - `Resume Game`, `Settings`, `Trophies`, `Take
Screenshot`, `Start/Stop Recording`, `Toggle Fullscreen`, `SaveState`, `Restart
Game`, `Exit Game` - and it wraps, so **`up` three times from the top is
`SaveState`**, which is fewer presses and does not risk landing on `Exit Game`.
One more `cross` opens a page whose single entry is `Save Emulation State And
Exit`, and a third `cross` takes it.

**The overlay needs a display taller than 720p to show the whole list.** At
1280x720 only the first six items are visible and the view does not scroll, so
`SaveState` is invisible and the highlight walks off the bottom - which reads
exactly like the d-pad not working. 1600x1200 shows all nine. That the d-pad
works at all is worth stating, since HD's own menus give no `TTY.log` line for a
highlight move and so say nothing either way.

### Two settings decide whether the file is written and whether it loads

| Setting | Value | What happens otherwise |
| --- | --- | --- |
| `Suspend Emulation Savestate Mode` | `true` | no savestate is written at all |
| `Compatible Savestate Mode` | `true` | capture dies: `Emu State Capture Thread ... Verification failed (object: 0x0)`, then `Saving savestate failed due to fatal error!`, leaving an **empty `savestates/BCES00664/` directory** that looks exactly like success to anything checking for the directory rather than a file |
| `Save Disc Game Data` | `false` | the file balloons from 44 MB to **2.0 GB** and then refuses to load: `HDD0 deserialization failed: Invalid directory name` |

### What it restores to, and why that ends it

It loads cleanly in about 60 seconds - and restores HD to its **Campaign /
Event 01/08 screen**, not to the grid the state was taken on. So the trade is a
60-second load that lands mid-front-end against a 45-second boot that lands at
`Main Menu`: **slower, and not appreciably closer to a race.** Whether the race
context is lost to `Compatible Savestate Mode` or to HD's own resume path was
not established, and is not worth establishing unless the payoff changes.

`Start/Stop Recording` in the same overlay turned out to be the item worth
having; see below.

## Video is the observable a race actually has

`TTY.log` goes quiet the moment the front end hands over, and the GDB stub
answers nothing while the target runs - so a driven lap has no per-tick channel
at all. RPCS3's own overlay records one, and it is two rows from `SaveState`:

```sh
just rpcs3-record --drive 45
# -> ~/.config/rpcs3/recordings/BCES00664/BCES00664_recording_<stamp>.mp4
```

`Start/Stop Recording` is a *toggle*: select it, drive, select it again. The
file lands under `recordings/<TITLE_ID>/` - a **subdirectory per title**, which
is worth saying because globbing `recordings/*` finds nothing and reads exactly
like recording having silently failed.

What comes out, verified: 1280x720 at 30 fps, MPEG-4 video and AAC audio, about
0.5 MiB a second. **And HD's HUD is in the frame** - lap number, position, lap
time, shield percentage and speed in km/h - so a recording is not just a picture
of a race, it is a readable trace of one at 30 Hz. That is the closest thing to
M3's per-tick capture this platform offers, and it is worth saying plainly that
it is a *reading off the screen*, not the engine's own numbers.

One cosmetic wart, recorded rather than fixed: `recording.yml` ships
`Video Codec: ""` and `AVCodecID: 12` (MPEG-4), and RPCS3 picks the first output
format that matches the codec - which is `rtp_mpegts`. The file is therefore an
**MPEG-TS stream with a `.mp4` extension**. `ffmpeg` and `ffprobe` read it
without complaint. Setting `AVCodecID: 27` (H.264) in `recording.yml` makes the
match land on `mp4` instead; that file is the user's rather than the
repository's, so it is left alone.

### The HUD reads back as numbers, sparsely and honestly

[`scripts/rpcs3_hud.py`](../../scripts/rpcs3_hud.py) turns a recording into a
speed trace:

```sh
just rpcs3-speed ~/.config/rpcs3/recordings/BCES00664/<recording>.mp4 > speed.csv
```

Measured on a 52-second Talon's Junction run at 15 Hz: **63 samples over 788
frames - 8 % coverage - spanning 225 to 444 km/h**, with a mean frame-to-frame
change of 4.3 km/h, which is what a real acceleration curve looks like at that
rate. Every surviving value is physically coherent; the point of the design is
that the other 92 % are *gaps*, not guesses.

Three things decide that trade, and only the last is tuning:

1. **The HUD is alpha-blended over the scene.** Over a bright stretch of Talon's
   Junction the glyph strokes sit barely above the background, and a global
   threshold reads 13 % of frames. The high-pass here - subtract a box blur
   wider than a glyph, keep what is *locally* brighter - is what makes the
   readable frames readable at all.
2. **The numerals are right-aligned in a fixed field**, and that is what turns a
   *dropped* digit into a rejected frame rather than a plausible wrong number.
   Measured: the last digit's run ends at column 53 in 100 frames of 111, and
   consecutive digits are 13 columns apart. Without the alignment and
   "is there ink still sitting in the slot to the left?" checks, the reader
   emitted `35` between `353` and `355`, and `747` - **coverage went from 8 % to
   28 % and the output stopped being trustworthy**, which is the wrong trade to
   make silently.
3. **The template set is what one clip contained** - 0, 2, 3, 4, 5, 7 and 8, and
   never a 1, 6 or 9. A frame showing one of those reads as unknown and is
   dropped. `rpcs3_hud.py glyphs` re-clusters any recording and writes the
   bitmaps out for labelling, which is how the set grows; this is deliberately
   not a general OCR.

**What would actually fix the coverage is not more tuning.** A darker circuit
would raise it a great deal - the washout is background brightness, and Talon's
Junction is the brightest thing on the disc. And the value the engine *holds*
would replace the reading entirely: that is a memory address, reachable through
`rpcs3_debugger.py`, and finding it is the honest next step rather than
squeezing this further.

### The menu walk has to be keyed on the screen, not counted

HD runs at about 9 fps here, and a `cross` that lands mid-transition does
nothing at all. A fixed six presses ends on `Team Selection` about a third of
the time. `Session.press_until` presses, watches `TTY.log` for the expected
screen, and re-presses up to four times - which is the whole reason the walk is
written as the list of screens rather than as a count of buttons.

## What is worth doing next, in order

1. **A committed input script and a lap.** The walk is six taps and thrust is a
   held button; what does not exist yet is `psp-drive.py`'s scripted form - a
   file of per-tick input replayed at the emulator and at our own physics. The
   pacing problem is different here: there is no cycle counter to key on (the
   GDB stub answers nothing while running), so a drive is wall-clock paced and
   is *not* frame-exact. Say so wherever a measurement leans on it.
2. **`Start/Stop Recording` from the home menu**, for a driven lap as video.
   The overlay is already pad-drivable and the item is two rows from
   `SaveState`; nothing else gives a race a continuous observable.
3. **Re-test breakpoints against an address that provably runs.** A race is full
   of them - `RaceManager_GetInstance` at `0x00054628` for a start - and now
   that a race is reachable, the confidence-35 hold above can be settled either
   way. Remember the decoder: `Interpreter (static)`, not the default LLVM
   recompiler.
4. **The front-end questions in
   [hd-frontend.md](../formats/hd-frontend.md)** that were parked on "needs the
   title running" - the runtime boot order, whether `Language Selection` is ever
   shown, step 4's real path. Each is now a `TTY.log` reading rather than an
   inference.

A per-tick trace harness comparable to M3's is a further step again and is not
costed here: the stub is silent while the target runs and costs 41 ms a packet
while it is stopped, so there is no capture channel on this transport at all.
Screenshots are the observable a race actually has.
