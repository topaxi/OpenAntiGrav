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
python3 scripts/rpcs3-drive.py stop           # always: stops RPCS3 AND Xvfb :77
```

`--load-shots N` adds N evenly spaced screenshots across the load window, which
is the only way to catch a screen that is up for none of the run either side of
it - the loading screen is the case it was added for. See
[hd-loading.md](../formats/hd-loading.md).

Measured on that path: `Main Menu` in about 45 seconds, a race running on
Talon's Junction about 80 seconds after that, and screenshots that are real
frames rather than black.

**Two things this path needs that an emulator update took away on 2026-09-11,
and the way each failure reads.** The system `rpcs3-bin` went from
`0.0.42-19777` (every capture above) to `19980` that morning, and the next
`capture` sat for its whole timeout with `TROPHY: checking` as the last
`TTY.log` line. Neither symptom was what it looked like:

1. **A firmware.** The new build refuses the disc without one - a modal
   `Booting ... failed! Reason: Firmware is missing` under `--no-gui`,
   `ppu_loader: PS3 firmware is not installed or the installed firmware is
   invalid` in `RPCS3.log`. The disc carries its own
   (`PS3_UPDATE/PS3UPDAT.PUP`, version 2.76): `File -> Install Firmware` in
   the GUI, or `DISPLAY=127.0.0.1:77 rpcs3 --installfw <pup>` on the virtual
   display with `xdotool` clicking `Yes` on both prompts (install?, then
   "old firmware detected ... continue?"). `~/.config/rpcs3/dev_flash/`
   then holds it and `RPCS3.log` says `Successfully installed PS3 firmware
   version 2.76`. The emulator's own config directory, reversible by
   deleting that folder; nothing in this tree. [toolchain.md](toolchain.md)'s
   "no firmware install is needed" is about *decrypting* the `EBOOT`, which
   is still true.
2. **The pad profile.** `~/.config/rpcs3/input_configs/global/oag.yml` was
   gone (only `Default.yml` left - whether the update or a GUI session
   removed it is not known), so `RPCS3.log` said `Input configuration empty.
   Adding default keyboard pad handler` and every press went to a keyboard
   with no window. **`TROPHY: checking` is then simply the last line the
   game prints before the `EpilepsyWarning` dialog**, which it sits on
   forever waiting for a cross that never arrives - a screenshot of the
   virtual display shows the health warning with `CONTINUE` highlighted. `uv
   run --with evdev python3 scripts/rpcs3_pad.py install-config` writes the
   profile back; `Pad 0: device='OpenAntiGrav Virtual Pad', handler=Evdev`
   in the log is the confirmation. `preflight` checks for it, and would have
   said so first - run it before a capture, not after one fails. And the
   dialog needs the *pressing* wait: `capture`, `race`, `shot`, `boot` and
   `record` all waited for `Main Menu` without pressing anything (only
   `browse` pressed), which was fine while the firmware-less boot skipped
   the dialog; they use `wait_for_screen_pressing` now.
3. **`stop` has to know the process's real name.** `rpcs3-bin`'s `/opt/rpcs3`
   layout runs `AppRun` (a script) that `exec`s `AppRun.wrapped ->
   usr/bin/rpcs3`, so the surviving process is called `AppRun.wrapped` and
   `pgrep -x rpcs3` finds nothing: `stop` reported only Xvfb stopped, the
   orphaned emulator kept the GDB port bound, and the next two captures
   timed out on `qSupported` - trap 2 of `rpcs3_debugger.py` reached without
   any client ever disconnecting. `EMULATOR_PROCESS_NAMES` carries both names
   now; `ss -ltnp | grep 2345` is the one-line check when a capture times out
   on its first packet.

Two afternoons of misreads worth not repeating: the trophy line was chased
as an HLE hang (ecryptfs, tmpfs, seeded `TROPUSR.DAT`, the pinned `19777`
AppImage now under `data/tools/rpcs3-19777/`) when a screenshot of the
display would have shown the dialog in one step - **when a walk stalls,
`import -window root` on `DISPLAY=127.0.0.1:77` before anything else.** And
write any `bin/rpcs3` wrapper as a file, never as a symlink to `AppRun`: a
`cat >` through a symlink overwrites the AppImage's own `AppRun` with a
script that `exec`s itself, and the fork loop looks like a boot that never
prints anything.

**Run `stop` (or `just rpcs3-stop`) when the whole session is done, not just
after one race, and even if the driving script itself already exited.**
`boot`/`race`/`shot`/`capture`/`browse`/`record` each stop their own RPCS3
process *on a normal exit* (`Session.__exit__`), but RPCS3 runs detached
(`start_new_session=True`), so a driving script that is killed, crashes, or
loses its terminal leaves it running with nothing attached to it - one agent
left RPCS3 up for thirty minutes today this way. `stop` kills it
unconditionally if one is up; RPCS3 does not support a second instance at all
(see the stale-lock trap below), so there is no risk of reaching "someone
else's" RPCS3. The display is a separate, independent gap: it is
deliberately long-lived across many of the calls above, so nothing tears it
down between them either - measured directly as an orphaned two-day-old
Xvfb process with no client attached. Unlike the emulator, `stop` only ever
stops a display this tooling itself started; it leaves alone one that was
already running before `display` ran. The whole command is safe to call
twice, or with nothing up at all. Never `pkill -x Xvfb` to clean up by hand -
see the trap below for why that specific command is dangerous rather than just
imprecise.

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

### It flushes in bursts, so a timestamp on a `Switching Screen` line is not the transition's

**Measured 2026-09-05, and it decides what a screen-keyed screenshot can and
cannot catch.** `TTY.log` arrives in chunks: on both boot-chain captures, five
transitions - `Language Selection`, `Blank`, `PreFMVConnect`, `Studio Logo` and
the picker's own departure - appeared with **one** poll, having been written
over several real seconds. Two consequences:

- **A shot taken when a screen's name appears cannot catch a screen that
  auto-redirected inside a burst.** The first pass of `bootchain` photographed
  five of eight steps for this reason; the three it missed are the three that
  advance without a press.
- **Elapsed times read off the log are upper bounds**, not the moment the screen
  came up. Fine for ordering, wrong for anything measuring how long a screen was
  displayed.

The workaround is a fixed-interval grab alongside the screen-keyed one -
`rpcs3-drive.py bootchain --film N`. It is not a full fix: a root-window grab
itself costs 1.5-3 s, so a screen up for less than that can still fall between
two frames, which is exactly what happened to `Language Selection` in the
original three cold boots - concluding "the screen never displays" from a
film that missed it would have been reading absence of evidence as evidence.
What settled it instead was timing the screen off `RPCS3.log`'s own
`sys_tty_write` lines rather than a root-window grab at all; see
[hd-frontend.md](../formats/hd-frontend.md#what-the-capture-did-not-settle-and-what-a-later-one-did).

**Ordering, by contrast, is trustworthy.** Bursts preserve write order, so the
*sequence* of `Switching Screen` lines is sound even when their timings are not
- which is why a boot chain can be measured off this log at all.

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

**A working recipe, verified end to end since**: `Single Player`'s own entrance
animation is still running when `press_once()` reports the screen name change
(`TTY.log` names a transition at its start, not its end); the very first
`right` tap fired immediately after arrival was silently eaten even with a
longer inter-tap settle, twice in a row. Sleeping 3 seconds before the first
tap, then four `right` taps at `settle=1.5` with a screenshot after each one
(confirm step 4 reads `ZONE` before pressing `cross`), reliably lands on Zone.

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

### `Z3` (read watchpoint) is fully verified end to end

**2026-08-31**, extending the same build:
[`scripts/patches/rpcs3-gdb-read-watchpoints.patch`](../../scripts/patches/rpcs3-gdb-read-watchpoints.patch)
adds `Z3`/`z3`, tracked separately from the write patch (apply that one
first - `just build-rpcs3-watchpoints` now applies both in sequence). Reads
have no single per-width `vm::read<T>()` to hook the way writes have
`vm::write<T>()` in `vm.h`; every scalar and vector PPU load instead funnels
through `ppu_feed_data<T>()` in `PPUInterpreter.cpp` (confirmed by reading
through its call sites, not assumed), which already had an existing
`RPCS3_HAS_MEMORY_BREAKPOINTS`-gated `bp_read` check for the Qt debugger
sitting right there for the new hook to sit beside. Same scope limitation as
`Z2`/`Z0`: interpreter-only.

Protocol level came first: `Z3,<addr>,<len>` and `z3,<addr>,<len>` both
answer `OK` against a live session, parsing and registry bookkeeping
matching `Z2`'s exactly. The first end-to-end attempt, on `0x00c81a5c`,
found no trap in an Arcade race - root-caused (not guessed) to that
address's own reader not executing outside Zone mode: a `Z0` breakpoint on
the reader instruction itself, checked properly (resume-slice ->
`pause()` -> walk every thread's PC via `wait_at()`, not `wait_for_stop()`,
which can go silent on a parked thread the same way it does for `Z0` - see
above), showed the reader never runs in Arcade, and a 120-second `Z3` watch
cross-checked against `RPCS3.log`'s own `Read watchpoint hit` line (written
synchronously regardless of whether a stop reply ever reaches a client)
confirmed zero occurrences to match.

**A rerun with a genuine, screenshot-confirmed Zone race (`RACE TYPE: ZONE`,
`Zone_HUD.xml` in `TTY.log`) closed it the same day.** A `Z0` sanity check
confirmed the reader instruction (`Scene_PrepareFrame`, `0x003aaf8c`) now
executes at all - closing the mode-gating question outright - then `Z3`
armed on `0x00c81a5c` fired within 5 seconds:

```
·S 0:01:14.616975 {PPU[0x1000000] Thread (main_thread) [0x003aaf8c]} GDB: Read watchpoint hit: 4 byte(s) at 0xc81a5c.
```

**That log line's PC is a second, independent way to get an accurate PC**,
useful regardless of the `Assume External Debugger` finding below: it comes
from the watch-check call site itself (`gdb_watch_check_read`/
`gdb_watch_check_write` log their own `addr` argument, not `ppu.cia`), so it
is correct even when a stop reply's register-dump PC is not. `Z3` is now
proven at every layer: parse, arm, trap on a real guest read, remove.

### `Z2`/`Z3` and vector stores: not a gap, confirmed from source - the real bug was PC reporting

**2026-08-31. Closed, after two rounds of an apparent gap that turned out not
to exist.** `STVX` (`PPUInterpreter.cpp:4577`) expands `PPU_WRITE(v128, ...)`
to `vm::write<v128>(addr, value, &ppu)` - the identical generic template
every scalar store already goes through. `LVX` (`:4252`) calls
`ppu_feed_data<v128, Flags...>(ppu, addr)` - the same choke point `Z3` hooks
for scalar reads (see above). Both confirmed by reading the handlers
directly, not inferred. **`stvx`/`lvx` were never blind to either
watchpoint.** (The one real, pre-existing gap is misaligned `STVLX`/`STVRX`
partial stores, which go through a raw pointer loop instead of
`vm::write<T>()` - symmetric between `Z2` and `Z3`, not new, and not what
either run below hit.)

That retires the whole "vector stores" hypothesis this section carried for a
day. What actually happened across the two runs:

| Run | Addresses armed | Result |
| --- | --- | --- |
| 1 | `0x00c50c00`, `0x00c50c10`, `0x00c49130` | zero hits over 90 s of a confirmed Zone race |
| 2 | `0x00c49110`, `0x00c49120`, `0x00c49130` | a hit, reported PC `0x003aae80` |

**Run 2's PC was never the trapping instruction, and now there's a
mechanism, not just a puzzle.** `ppu.cia` (the interpreter's own idea of
"current instruction address", what a stop reply's PC comes from) is only
kept updated per-instruction when `is_debugger_present()` was true **at the
moment that particular opcode's dispatch thunk was first JIT-built**
(`PPUInterpreter.cpp` ~142/264, `Utilities/Thread.cpp:199`) - and that
function checks for a *native* debugger attached to the RPCS3 process, or
the `Assume External Debugger` config flag. **A GDB client on port 2345
satisfies neither.** With both that flag and `PPU Debug` left at their
shared-config defaults (`false`), `cia` never updates between branches - it
sits wherever the last `bl`/`b` left it, which is exactly the "a `nop` right
after an unrelated `bl`" symptom run 2 produced. **Fix: set `Assume External
Debugger: true` in `config.yml` before booting** - it is baked into each
opcode's dispatch thunk the first time that opcode executes, so it cannot be
toggled mid-session. **Treat this the same as the `PPU Decoder` switch
below**: a temporary, session-scoped change to the shared config, not a new
permanent default - the flag's name suggests it may disable optimizations or
affect thread-suspend heuristics, unconfirmed either way, so switch it back
when a run doesn't need accurate stop-reply PCs.

**Run 1's zero-hit result still isn't explained, but the space of
explanations just got smaller.** With the hook itself cleared, the two live
candidates are: the write that produced the observed non-zero values
happened once, early (at load or scene-init), rather than every frame as the
static trail assumed - so a 90-second window arming *after* that point would
legitimately see nothing further; or the gate genuinely was shut for that
particular run. Telling those apart needs no watchpoint at all: **poll the
memory once a second across a race and watch for the value to change** -
that is the test this thread ran next, see
[zone-effectsettings-loader.md](../ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md).

**The rule that survives all of this:** a stop reply's PC is not trustworthy
evidence of *where* a trap fired unless `Assume External Debugger` was set
before boot - a watch or breakpoint can genuinely hit while still reporting
a PC that looks unrelated. A zero-hit result is still worth double-checking
against a poll-based test when the stakes are high, but not because of
vector stores specifically - that possibility is closed.

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

### A breakpoint's stop reply is queued at once, and an unread one kills the stub

Measured 2026-10-02. After `vCont;c` with a `Z0` already set, the breakpoint hit
**20 ms later** (`SYS: Emulation is being paused... (mark=1)`), the stub sent
its stop reply and waited for the client's `+`. A client that then slept two
seconds and sent `\x03` (which is what `run_for()` and a `resume()` / sleep /
`pause()` loop do) made the stub log `GDB: Wrong acknowledge character
received: ''` and answer nothing more - every later packet timed out, 25 s each,
and the boot was spent. Read the reply right after the resume instead:

```python
gdb.resume()
stop = gdb.wait_for_stop(30.0)   # the reply, acked; None if nothing stopped
if stop is None:
    gdb.pause()                  # only now is \x03 safe
```

A breakpoint that fires every tick (the HUD's, here) is hit at once; one that
does not needs `wait_for_stop` with a real timeout, not a sleep.

### A member's own emulator, and what a scan costs

`scripts/rpcs3-drive.py` read `~/.cache/rpcs3/TTY.log` and the stock config
whatever `XDG_*` said, so a second user's private instance waited on the
first's file and never saw a screen change. It now honours `XDG_CACHE_HOME` and
`XDG_CONFIG_HOME` (the log, `TTY.log`, the lock, the recordings and the input
profile all move), `OAG_RPCS3_DISPLAY` (the Xvfb), `OAG_RPCS3_SCRATCH_CONFIG`
(where the generated config copy goes - `data/tools/` is shared between
worktrees) and `OAG_RPCS3_GDB=127.0.0.1:2391` (the stub's port, written into
the copy). A private config directory needs its own copy of `dev_hdd0`
(the save decides the boot chain) and the `input_configs` profile; `dev_flash`
can be a symlink.

A heap scan is cheaper than the table above says: 540 KiB (`0x307c0000`-
`0x30844000`, 2 KiB per read) took **11 s** against a Recompiler boot, and the
HD HUD's record, the HUD object and the ship were each found in under a minute.
The HUD object was at the same address (`0x32921a30`) in all four boots of one
session; the ship and the record moved by a few kilobytes.

### The shared Xvfb never clears, so an old window's pixels can ghost into a new screenshot

**Found 2026-09-01**, capturing HD/Fury's Main Menu. `start_display()` reuses
`:77` whenever it is already listening rather than restarting it, which is
right for not paying a ~1 s Xvfb boot on every run - but Xvfb itself never
repaints a region once the window that owned it closes, so whatever was drawn
there last stays in the root window's framebuffer forever, or until something
else overdraws it. `screenshot(trim=True)` only crops the outer
`#000000`-on-`#000000` border, so a leftover window sitting beside the new one
survives the crop intact and reads as part of the frame. Two boots of the same
`browse --screen "Main Menu"` command an hour apart came back with the real
menu on the right both times and **two completely different, unrelated
images** on the left - a race HUD once, a blue architectural render the next -
neither of which HD/Fury's front end draws at all. Both were leftovers from
some earlier, already-exited RPCS3 window that Xvfb had never been told to
forget. **A screenshot from this harness is only trustworthy up to the real
window's own rectangle**, and nothing here currently reports where that is;
until it does, treat an unexplained region beside the expected content as
ghosting first, not as a feature, and crop it out by hand (the real content in
both cases started at a consistent x-offset from the left edge of the
trimmed image). Restarting Xvfb (`python3 scripts/rpcs3-drive.py stop`, then
let `start_display()` spawn a fresh one) is the sure fix but costs every
other script sharing `:77` its state; finding the live RPCS3 window's own
geometry (`xdotool` or `xwininfo` against the game's window, not the root)
and cropping to exactly that would fix it without the blast radius.
**Never `pkill Xvfb` or `pkill -x Xvfb` for this** - that reaches every
virtual display on the machine, including one another script or another
agent's session is using right now, not just `:77`; `rpcs3-drive.py stop`
kills only the specific process this tooling's own marker file names.

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

> **Re-measured 2026-10-09, see [emulator-recipes.md](emulator-recipes.md).** A state taken on the
> grid **did** restore to the grid (fly-over with `START RACE`, 7.7 s after launch; two loads), unlike
> the 2026-08-19 account below. But the write succeeded once in six attempts (cause not found), and in
> the one loaded run the pad was dead and the stub off (seen once). It is good for a still or video of
> the fly-over and nothing that needs input or GDB. The advice stands: keep an emulator running.

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

**Superseded 2026-10-09:** a state now restores to the grid it was taken on, with a working pad, stub and
memory, in about 10 s; the "Campaign / Event 01/08" landing below came from a load that ignored the config.
See `emulator-recipes.md`, "RPCS3 save states". The two-settings table above stands: `Compatible Savestate
Mode` must be `true`.

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

## Two more channels, both measured on the Fury menu backdrop (2026-09-14)

**Reading a function's arguments at a breakpoint settles a decompile that will
not read.** `0x00182358` is 587 lines of VMX permutes taking a view and a
projection matrix in place; rather than read it, `scripts/hd-fury-backdrop-break.py`
stops `BackgroundAnimFury_Render` at the `bl` before it and at the instruction
after it and reads both matrices off the stack (`r1 + 0x430`, `+0x470`) - eight
frames, bit-identical before and after, so the function is inert here and the
question closed in an afternoon. Three things that script does differently from
`hd-flare-rendertick-break.py`, each because the other way desynced the stub:

- **One breakpoint armed at a time.** Arm, run, read, disarm, arm the next. A
  thread parked on an armed address is never resumed onto it, and no second
  hit's stop reply ever queues behind the first - the two conditions under which
  a `qfThreadInfo` came back with nothing and the session died on a timeout.
- **`wait_for_stop(0.3)` before `pause()`, not instead of it.** The stub may or
  may not announce a hit; a short wait catches the reply when it comes, and the
  interrupt still follows when it does not.
- **Write guest memory to hold the target on the path you want.** The picker's
  debug kind field (`g_FurySettings+0x418`) written to `3` through `M` keeps
  every later clip on a static path, so the mode this build draws is sampled
  every time instead of one pick in three. The write costs one packet.

Wall clock: about 40 s per sample (two stops, ~20 register dumps, five memory
reads, a screenshot), so a run of twelve fits in ten minutes with the boot.

**Render targets cannot be read this way.** The same script's `--dump-targets`
reads windows of the backdrop's three targets at their RSX local addresses
(`0xc5b30000`, `0xc5ef0000`, `0xc5fe0000`, off the texture objects) and gets
zeros for every byte on three frames while the screen shows the hull. RPCS3
keeps a render target on the host GPU and writes it to guest memory only when
the game itself reads it back, so a target's contents are not an observable on
this transport; the texture *objects* (size, pitch, format, filter word) are,
and were.

**RPCS3's shader log is a second decoder for RSX microcode.** `Log shader
programs: true` in `config.yml` (Video) writes every program the renderer builds
to `~/.cache/rpcs3/shaderlog/` as `VertexProgramN.spirv` and `FragmentProgramN.spirv`
- **GLSL text despite the extension** under Vulkan, `#version 450` and readable.
Constants are renumbered from zero in first-use order (`c[201]` became
`_fetch_constant(8)`), so a program is found by its shape - `fract` plus `log2`
plus `exp2` for a `RadioHead`. It carried modifiers `scripts/ps3-microcode.py` did
not decode: the four `clamp(.., 0.0, 1.0)` on `VertexProgram11` were the NV40
saturate flag, bit 26 of dword 0, and the whole of why the backdrop drew white.
Cheap - one boot with the flag on - and the check to make before trusting a
disassembly's arithmetic. Turn the flag off after: it writes on every shader
compile.

## Four more things from the flare and Zone breakpoint runs (2026-09-15)

**Every scripted launch is muted and on a copy of the config now.**
`rpcs3-drive.py` passes `--config data/tools/rpcs3-scratch-config.yml`, a
full copy of `~/.config/rpcs3/config.yml` with `Audio: Renderer: "Null"` and,
under `--interpreter`, `Core: PPU Decoder: Interpreter (static)`; every
subcommand and every script built on `Session` gets it, `--stock-config`
opts out. Two things it took a boot each to learn: `--config` **replaces**
the whole configuration rather than overlaying it (a three-key file would
lose `GDB Server` and `Assume External Debugger`), and the audio value has
to be the quoted string `"Null"` - a bare `Null` is YAML's null, and
`RPCS3.log`'s own `Used configuration:` dump still said `Renderer: Cubeb`
with it. That dump is the check: `Session.config_report()` / `config_report()`
read `Audio: Renderer` and `Core: PPU Decoder` back out of it.

**`vCont;s` does not move a thread parked on a breakpoint.** Answered, but
on 213 of 213 tries the thread's PC still read the breakpoint address
afterwards. A 30 ms free run does move it but lets the rest of the frame go
by, so a breakpoint on a per-object function samples the *first* object of
each frame almost exclusively - 172 hits of 214 on one craft of eight. What
samples every call is a **hop**: with the breakpoint removed, arm
`address + 4`, resume, and the thread executes one instruction and parks
again; re-arm `address` and the next call is the next object. Never two
addresses armed at once, never a resume onto an armed address -
`scripts/hd-flare-owner-break.py`'s `Breaker.step_off`. With it, 30 frames
of all eight crafts in a fixed queue order.

**Two Zone-race details of this build.** `TTY.log` prints `ScreenManager
Load "Data\XML\Zone_HUD.xml"` for a Zone race and **no** `RACE TYPE:`
line - a check keyed on the line an earlier build printed aborts on a real
Zone race. And Zone waits on a `START RACE` prompt (cross) after the load
that the held-thrust single race never shows; nothing moves until it is
pressed.

**The front end's own `Loading Screen Finished` comes first.** A wait keyed
on that line returning at all sees the front end's, not the track's; count
the occurrences before the walk and wait for a new one alongside
`Loading track model`.

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
