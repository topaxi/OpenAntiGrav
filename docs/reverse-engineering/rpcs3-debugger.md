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
| `vCont;c` | resume |
| `vCont;s:<tid>` | single-step one thread |
| `\x03` | interrupt; replies `S05`, sometimes seconds late |

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

### `Z0` answers `OK` for breakpoints that can never fire

Two separate reasons, both silent:

1. **The default `PPU Decoder: Recompiler (LLVM)` does not honour PPU
   breakpoints.** `Interpreter (static)` is the decoder that does; it was tried
   here (via `--config` on a copied `config.yml`, which is the clean way to
   change a setting without touching the user's) and the title still booted, in
   about 75 seconds rather than 30.
2. **HLE import stubs are not PPU code paths.** `ppu_loader` logs the game's
   `sys_io` imports at boot - `[cellPadGetData] (0x8b72cda1) -> 0x759a44`, and
   `0x759a44` really does hold `li r12, 0` - but a breakpoint there never fires
   under either decoder.

Under the interpreter, no breakpoint fired on any address sampled from a stopped
thread either - but the sampled addresses are the wrong test, because nothing
says they ever run again. **All 14 threads `qfThreadInfo` lists sit at identical
PCs across a 1-second and a 10-second window of real running**, measured twice
in sessions where the interrupt and resume were separately confirmed to work.
The reading, at confidence 65: those are PPU threads parked in HLE waits, the
game is waiting on something, and the 200-330 % CPU the process burns meanwhile
is RSX and SPU work - which is consistent with HD, and which the thread list
does not cover.

So confidence that breakpoints work at all on this build is **35** - untested
rather than refuted. The test needs an address known to execute, which needs the
game past the screen it is sitting on, which needs input.

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

`Start/Stop Recording` in the same overlay is unexplored and is the more
interesting item: it would give a driven lap as video, which is the one
observable a race has that `TTY.log` does not.

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
