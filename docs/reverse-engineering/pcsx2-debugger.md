# Driving PCSX2 from PINE

The PSP side of this project drives PPSSPP from a JSON-over-websocket debugger
that reads memory, sets breakpoints *and presses buttons*
([ppsspp-debugger.md](ppsspp-debugger.md)). The PS3 side drives RPCS3 from a GDB
stub, a kernel-level virtual pad and a virtual display
([rpcs3-debugger.md](rpcs3-debugger.md)). This page answers the same question
for the PS2, measured against `PCSX2 v2.7.494` (the string `MsgVersion` returns)
and Wipeout Pulse (`SCES-54748`, PAL) on 2026-08-23.

**The short answer: everything the other two harnesses do, and one thing
neither of them can - a bit-exact replay.** A script boots the disc with no
window on anyone's desktop, walks its front end into a race, holds thrust, and
captures a frame; separately it reads and writes guest memory at the Ghidra
corpus's own addresses at 50 million reads a second. And from a savestate, the
same scripted 30 frames produce a **pixel-identical** PNG twice in a row, with
EE RAM bit-identical to a SHA-256 over 8 MB.

The transport is three pieces, none of which is the piece the PS3 harness uses:

```
Xvfb :78 ── pcsx2-qt (OpenGL, -batch -nogui) ── PINE unix socket   (memory, savestates)
                  ▲
                  └── XTEST synthetic key events                    (buttons, hotkeys)
```

Everything here is behind [`scripts/pcsx2_pine.py`](../../scripts/pcsx2_pine.py)
and [`scripts/pcsx2-drive.py`](../../scripts/pcsx2-drive.py), whose module
docstrings carry the same trap list in the form a caller needs it.

## The one command

```sh
just pcsx2-boot                                          # cold boot, headless, ~25 s to a frame
just pcsx2-frame /tmp/f.png 30 cross --from-state 2      # 30 exact frames on thrust, then shoot
just pcsx2-stop                                          # always
```

`pcsx2-frame` is the one that matters: it loads a savestate, steps a *verified*
number of emulated frames with whatever buttons held, and grabs the paused
frame. Run it twice and the two PNGs are byte-identical - measured across
separate `just` invocations, and across emulator restarts, both landing on the
same guest frame counter.

**`--from-state <slot>` is not optional for repeatability.** Without it the run
starts from wherever the emulator happens to be, and the frame index it reports
is only meaningful within that one session. The savestate has to be made once,
by walking the front end into a race (see below) and running
`just pcsx2-state save 2`.

**`just pcsx2-stop` really does mean always, not just when `pcsx2-qt` is
still up.** It also tears down the Xvfb `:78` that `display`/`boot` started,
and nothing else ever does - `display`, `boot`, `press`, `shot` and `input`
are deliberately separate invocations against one long-lived display, so a
session that ends without this call leaves that display running. Measured
directly: three orphaned Xvfb processes on this project's own machine, all
two or more days old with no client attached, one of them `:78` from this
harness. `pcsx2-stop` is safe to call twice and safe to call when nothing is
up at all - it only ever stops a display this tooling itself started, never
one that was already there.

## Booting without a human, and without a window

PCSX2 has **no `--headless`**. `-nogui` hides the *main* window and still opens
a render window, and that render window is where both the frames and the key
events go - so the answer is a display that is not the user's, exactly as for
RPCS3, and for a second reason on top of RPCS3's.

```sh
python3 scripts/pcsx2-drive.py display     # Xvfb :78, TCP-addressable
python3 scripts/pcsx2-drive.py boot        # writes the ini, launches, waits for PINE
```

Measured on that path, from cold: PINE answering in about 2 s, the disc
identified as `SCES-54748` immediately after, and a real (non-black) frame at
about 25 s. A race is reachable in about four minutes of scripted walking, most
of which is the game's own loading.

Four details in the launch, each of which cost a run:

- **`Renderer = 12` (OpenGL), not the default `Auto`.** Auto picks Vulkan,
  Vulkan finds no present queue on Xvfb, and PCSX2 gives up before the VM
  exists:

  ```
  VK: Failed to find an acceptable present queue.
  Failed to create GS device
  ReportErrorAsync: Startup Error: Failed to initialize GS.
  ```

  The process then exits quietly under `-batch`, so the symptom is an emulator
  that "did not start" with a perfectly good command line.
- **`-datapath <dir>` appends `PCSX2/` of its own.** An ini written to
  `<dir>/inis/PCSX2.ini` is never read; it has to be `<dir>/PCSX2/inis/PCSX2.ini`.
  The tell is in the log, and it is easy to skim past because it looks like
  confirmation rather than contradiction:

  ```
  BIOS Directory: /home/topaxi/.cache/oag-pcsx2/PCSX2/bios
  ```

  With the ini in the wrong place PCSX2 silently uses built-in defaults - which
  means no PINE, no bindings, and a BIOS directory that does not exist.
- **`[Folders] Bios` must be absolute** or it resolves inside that same data
  path. The harness points it at the user's real `~/.config/PCSX2/bios` rather
  than copying a BIOS anywhere. **That folder being empty is a real, silent
  failure mode**, hit 2026-09-03 in a session with no prior PCSX2 setup:
  `just pcsx2-boot` reports nothing to the caller beyond the recipe's own exit
  code, and the actual reason is one line into the emulator's own log,
  `ReportErrorAsync: Startup Error: PCSX2 requires a PlayStation 2 BIOS in
  order to run` - check `~/.config/PCSX2/bios` before assuming the harness
  itself is broken. A BIOS is exactly as user-supplied as the disc images
  under `data/images/` and just as uncommittable; this project ships none.
- **Xvfb must listen on TCP** (`-listen tcp -nolisten unix`, addressed as
  `127.0.0.1:78`), for the reason the RPCS3 page records: a sandboxed session
  may be unable to write `/tmp/.X11-unix`, and then the unix socket never
  appears while the server itself is fine. `QT_QPA_PLATFORM=xcb` and dropping
  `WAYLAND_DISPLAY` are needed for the same reason as there.

The harness keeps all of this in `~/.cache/oag-pcsx2`, a data path of its own,
and never touches `~/.config/PCSX2`. That is deliberate: the user's interactive
PCSX2 keeps its own settings, its own memory card and its own snapshots, and a
scripted run cannot corrupt them.

`just launch-pulse-ps2` remains the interactive recipe and is unchanged.

## PINE is compiled in, and it is the whole reason this is worth doing

PINE ("Protocol for Instrumentation of Emulators", the old "PCSX2 IPC") is a
length-prefixed binary protocol over a unix socket. **The stock `pcsx2-qt`
binary has it** - `strings` on `/usr/bin/pcsx2-qt` shows `PINE.cpp`,
`EnablePINE` and `PINE: Cannot open socket! Shutting down...`, and it answers.
No debug build, no rebuild, no flag.

It is off by default and there is no command-line switch for it. Two ini keys
under `[EmuCore]`:

```ini
EnablePINE = true
PINESlot = 28011
```

### The socket is not where the documentation says

`$XDG_RUNTIME_DIR/pcsx2.sock` - `/run/user/1000/pcsx2.sock` here. The
`/tmp/pcsx2.sock` path that most PINE documentation quotes is only the fallback
for a session with no runtime directory; it is also in the binary's strings,
which makes it look confirmed. A non-default `PINESlot` appends `.<slot>`.

### The opcodes are `0x00`-`0x0F`

Not the `0xF0`-`0xF7` range several third-party PINE clients use for the
metadata commands. Every opcode in that range returns `FAIL` here, and so does
everything in `0x10`-`0xFF`; a sweep of the whole byte finds exactly sixteen
implemented commands.

| Opcode | Command | Arguments | Reply |
| --- | --- | --- | --- |
| `0x00`-`0x03` | Read8/16/32/64 | `u32 addr` | 1/2/4/8 bytes |
| `0x04`-`0x07` | Write8/16/32/64 | `u32 addr`, value | - |
| `0x08` | Version | - | `u32 len` + string, `PCSX2 v2.7.494` |
| `0x09` | SaveState | `u8 slot` | - |
| `0x0A` | LoadState | `u8 slot` | - |
| `0x0B` | Title | - | `WipEout Pulse` |
| `0x0C` | ID | - | `SCES-54748` |
| `0x0D` | UUID | - | `f8ae6ff2`, the ELF CRC |
| `0x0E` | GameVersion | - | `1.00` |
| `0x0F` | Status | - | `u32`: 0 running, 1 paused, 2 shutdown |

Framing: request is `u32 total_len` (including itself) then the commands;
reply is `u32 total_len`, **one** result byte for the whole request (`0x00` ok,
`0xFF` fail), then every command's payload concatenated in order.

**One request may carry many commands.** This is the difference between a
14 000-reads-a-second toy and a real capture channel, and it is not obvious from
the framing:

| Batch | Request | Wall clock | Rate |
| ---: | ---: | ---: | ---: |
| 1 read32 | 9 B | 0.071 ms | 14 000/s |
| 1 024 | 5 KiB | 0.12 ms | 8.6 M/s |
| 16 384 | 80 KiB | 0.22 ms | 76 M/s |
| 65 536 | 320 KiB | 1.25 ms | 52 M/s |

**Keep a request under about 256 KiB.** A 640 KiB request never answers at all
and the client hangs until its own timeout; the emulator survives it, the
connection does not. `Pine.read_words` splits at that ceiling by itself.

At those rates a per-frame trace is affordable in a way neither of the other two
transports is: 8 MB of EE RAM hashes in about 0.7 s, and a few hundred bytes of
ship state per frame costs microseconds. Compare RPCS3's 41 ms per packet and
total silence while the target runs. **PINE answers while the game is running**,
which is the property that matters.

### Addresses are the Ghidra corpus's own, unrebased

Verified twice, at addresses the corpus already names in
[`ps2-pulse-eu/names.tsv`](../ghidra/functions/ps2-pulse-eu/names.tsv):

- `g_crc32_table` (`0x002849e8`) reads
  `00000000 77073096 ee0e612c 990951ba 076dc419 706af48f e963a535 9e6495a3` -
  the standard CRC-32 table for polynomial `0xEDB88320`, generated
  independently and matched word for word.
- `g_tolower_table` (`0x002c6d98`) has `61 62 63 64 65` at offsets `0x41`-`0x45`,
  which is `A`-`E` mapping to `a`-`e`.

So every row in that file works as typed, no rebasing and no offset. Confidence
94, the top of the "Confident" band: two independent tables whose contents are
known *a priori* rather than derived from the binary, both exact, read at
runtime. It does not reach 95 because the
[rubric](confidence-rubric.md) reserves that band for a claim corroborated in a
second binary, and this one is about one image in one emulator.

### PINE answers before the VM exists

`MsgVersion` succeeds within a second of launch. `MsgTitle` and `MsgID` return
`FAIL` and `MsgStatus` returns `2` (shutdown) for as much as twenty seconds
after that. A script that queries once and reports the answer prints a boot
failure that is not there - this page's first session recorded exactly that.
`Pine.wait_for_game()` polls; use it.

## Input: XTEST, and two silent gates

RPCS3 has no input API and reads `/dev/input` directly, so a `/dev/uinput`
virtual pad works there. **That does not transfer.** PCSX2's `Keyboard/*`
bindings are Qt key events delivered by the X server, and Xvfb has no evdev
driver attached, so a uinput device is invisible to it. (The SDL input source
*would* see one, since SDL enumerates `/dev/input` itself - but that needs
`SDL-0/...` bindings written into the ini and a pad that exists before PCSX2
starts. It was not needed and was not pursued.)

What works is XTEST against the virtual display, which needs no permissions at
all and reaches PCSX2's stock keyboard bindings:

```sh
uv run --with python-xlib python3 scripts/pcsx2-drive.py press cross start
```

The chain was verified end to end rather than assumed. Holding `Keyboard/K`
while reading the game's *own* abstract input mask through PINE:

```
pad 0 connected=1 held=0x0000 pressed=0x0000
while holding cross    held=0x0020 (cross)
```

`0x0020` is the `cross` bit exactly as
[`ps2-pulse-eu/input.md`](../ghidra/functions/ps2-pulse-eu/input.md) documents
it, read at `+0x44` of the pad block. `scripts/pcsx2-drive.py input --button
cross` is that check as a command, and it is the right first thing to run when
a walk stops working.

**`g_input` (`0x00302ec0`) holds a pointer, not the block.** The offsets on that
page are into `*g_input` - `0x00362710` in every run observed, the value being
stable across boots. Reading the documented offsets *at* `g_input` gives zeros
that never change, which reads exactly like input not arriving. The four analog
axes at `+0x154`/`+0x158`/`+0x160`/`+0x164` and the connected pair at
`+0x3c`/`+0x40` all land where the page says once the pointer is followed, which
is independent corroboration of that page from a second direction.

### Gate one: nothing has focus

There is no window manager on the virtual display, so the input focus is
`PointerRoot` and the pointer sits at the centre of a 1600x1200 screen - outside
a 640x480 render window pinned at the top-left. **Every key press goes to the
root window and vanishes.** No error, no log line, nothing in `g_input`.

The fix is two calls, `set_input_focus` on the render window and a pointer warp
into it, and `Keyboard.focus()` does both before every press. Diagnosing it
without knowing to look is expensive, because it is indistinguishable from a
wrong binding, a wrong keysym, or a game that is not listening.

### Gate two: a `[Pad1]` section with no bindings in it

This is PCSX2's version of RPCS3's missing input profile, and it fails the same
way. A `[Pad1]` that names only `Type = DualShock2` produces a pad the game sees
as **connected** -

```
Pad: DS2 Config Finished - P1/S1 - AL: On - AB: Locked - VS: Normal ...
```

- and **zero** host bindings. Measured on that config: not one `MapController`
line in the log, and `held` stayed `0x0000` for every button pressed. The game
is not ignoring you; there is nothing to ignore.

Every binding has to be written out explicitly. `PAD1_BINDINGS` in
`pcsx2-drive.py` is PCSX2's own stock set, written into the generated ini, and
the check for whether it took is `grep -c MapController` on the log: 24 lines,
or something is wrong.

## Screenshots: two routes, and what each one resamples

### The GS framebuffer is 512x512, and only one setting gives it to you

`[EmuCore/GS] ScreenshotSize` decides what the `F8` hotkey writes. Measured on
`SCES-54748` at an upscale multiplier of 1:

| Value | Meaning | Output |
| ---: | --- | --- |
| 0 | screen resolution | 640x480 - the render window |
| 1 | internal resolution, aspect corrected | 682x512 |
| **2** | **internal resolution, uncorrected** | **512x512** |

**512x512 is what the PS2 actually rasterises**, and the PAL CRTC stretches it
to the 640x512 it announces in the log (`Set GS CRTC configuration. PAL
640x512 @ 50.000 Interlaced (FIELD)`). The other two values are that buffer
resampled, one of them to a resolution the console never produced. A comparison
against our own renderer should be against the buffer, so the harness defaults
to 2.

Three more things about `F8`:

- **It fails until the GS has presented a frame**, logging
  `Failed to render/download screenshot` and writing nothing. A shot taken two
  seconds after boot fails the same way a lost key press does.
  `gs_screenshot()` re-taps until a file appears.
- **The PNG is encoded on a worker thread**, so the file exists before it is
  complete. Moving it immediately truncates it, and the symptom is ImageMagick's
  `unexpected end-of-file`, which reads like a corrupt frame rather than a race.
  Wait for the size to stop changing.
- **It does nothing at all while the VM is paused.** Which is exactly when a
  frame-exact capture wants it.

### So a paused frame is an X grab, and the window has to be resized to match

`-fullscreen` does not enlarge the render window: with no window manager on the
display nobody honours the request and it stays 640x480+1+25. **But a plain
`ConfigureWindow` does**, needing no window manager at all - it is a direct X
request and PCSX2's swapchain follows it. Verified at 512x512, 1024x1024 and
1280x1024, parent and child window both.

That plus `AspectRatio = Stretch` makes the presented image the internal buffer
1:1, and the paused grab is then 512x512 with no resampling and no letterbox.
Under the default `Auto 4:3/3:2` the same square window is letterboxed and the
grab sits 30 % RMSE away from the `F8` shot; with `Stretch` that falls to 11 %,
and the residual is that the two captures are of *different frames* - the VM is
running between them, and there is no way to take both of one frame because
`F8` is inert while paused. So "the paused grab is the internal buffer 1:1" is
argued from geometry (window size equals buffer size, aspect is Stretch),
not from a pixel-equality proof: confidence 80.

## Deterministic capture: this is what PCSX2 has that the others do not

### Savestates work, from PINE, with no menu walk

`MsgSaveState`/`MsgLoadState` take a slot byte and write
`sstates/SCES-54748 (F8AE6FF2).<slot>.p2s`. Measured: 4.3 MB at the main menu,
12 MB in a race, 158 ms to write. Nothing like RPCS3's nine-item overlay walk,
and unlike RPCS3's the state **restores to where it was taken** - a savestate
made on the grid reloads on the grid.

Loading is bit-exact: a 1 MB SHA-256 of EE RAM immediately after `loadstate`,
taken on two separate runs, is the same digest both times.

**Load while the VM is paused, not while it is running.** A `loadstate` on a
running VM leaves the guest free-running for however long the script takes to
notice, and that is not a rounding error: `loadstate`, one `status` query and a
six-second sleep drifted the guest **770 frames** past the anchor. Pause,
load, re-pause, then step. `cmd_frames`'s `--from-state` does exactly that and
is why the two captures above land on the same counter value.

### A frame advance is exact only if you verify it

The `FrameAdvance` hotkey is **unbound by default**; the harness binds it to
`Keyboard/F7`. Given it, the naive sequence - pause, hold thrust, tap F7 sixty
times - does not reproduce. Two runs from the identical savestate moved the
guest **61 and 66 frames** for sixty taps each, and their EE RAM hashes differ.

The reason is that a tap can be dropped *or* overshoot, silently in both
directions, and it depends on how long the key is held and how long you wait
after it. Twenty taps at each timing, counting the guest's own frame counter
either side:

| hold | after | frames per tap |
| ---: | ---: | --- |
| 0.02 s | 0.02 s | 0 x11, 1 x8, 2 x1 |
| 0.02 s | 0.05 s | 0 x6, 1 x13, 2 x1 |
| 0.02 s | 0.15 s | 0 x2, 1 x17, 2 x1 |
| **0.05 s** | **0.30 s** | **1 x20** |

0.05/0.30 costs about three stepped frames a second - slow, and exact. On top of
that `advance_frames()` reads the counter either side of every tap and raises
rather than guess: a dropped tap is retried, an overshoot cannot be stepped back
out of and fails the run (`--from-state` reloads and retries the whole
sequence).

With verification, from one savestate, twice:

```
run 1  stepped 60, counter 24060 -> 24120 (+60)   hash 3b044154e347fbd1...
run 2  stepped 60, counter 24060 -> 24120 (+60)   hash 3b044154e347fbd1...
IDENTICAL
```

The same with `cross` held throughout gives a different hash (`b157f5ef...`,
so the input really is being consumed) and that hash is identical across runs
too. And the captured PNGs compare at `AE = 0` - **pixel-identical**.

Two things follow, and they are the point of this whole page:

1. A PS2 reference frame is now a *reproducible artefact*, not a thing a human
   caught by pressing a key at roughly the right moment.
2. A per-frame trace against our own simulation is reachable on this transport.
   Step one verified frame, batch-read the state, repeat - at three frames a
   second, which is slow but is a trace rather than nothing.

### The frame counter, and what it is not

The verification needs a guest-side "did this frame happen" signal. Scanning
2 MB of EE RAM across four verified single-frame steps found **exactly two**
words that increment by one every time: `0x0027a7e8` and `0x002850fc`, which
track each other 25 apart. The harness uses the first.

**Neither is named**, here or in the Ghidra corpus, and that is deliberate. That
a word increments once per frame is measured and certain; *what it is* - a vsync
counter, a game tick, a timer the front end owns - is not, and naming it would
be a corpus change needing its own evidence page under
[`docs/ghidra/functions/ps2-pulse-eu/`](../ghidra/functions/ps2-pulse-eu/README.md).
Confidence that it advances exactly once per emulated frame: 90, from four
verified steps plus 20 further single-step observations. Confidence in any
particular meaning: not offered.

### One verified frame is one video field, not one game tick

`scripts/pcsx2-trace.py`'s first real capture (2026-09-16) found this the
hard way: five independent craft/body fields - position, velocity, angular
velocity, `speed_cached`, `throttle` - read **bit-identical** across one
verified `frames 1` step and only changed on every *other* one, over 20
consecutive steps. Not "close" - `%.7g`-formatted values matching to seven
significant figures on the stale rows, the kind of exact repeat that only
happens when nothing recomputed them at all.

PAL is confirmed interlaced (`Interlaced (FIELD)` in PCSX2's own boot log,
`docs/tools/oag-trace.md`'s boot session), which is the natural explanation:
`FRAME_COUNTER` above almost certainly counts video **fields** at 50 Hz,
while `Ship_UpdateCraft` runs once per video **frame** - 25 Hz, half the
step rate this whole page's frame-advance measurements are keyed on.
Confidence 85: five fields, all independently addressed, all agreeing on
which steps are real and which are stale, across one continuous 20-step
run - not yet cross-checked against a second capture or a second craft.

**This does not invalidate anything else on this page.** Every
`advance_frames` measurement here is about whether a *requested number of
verified steps* lands exactly - it does, regardless of what the game does
with any given step. What it changes is how a caller should read a capture
taken one step at a time: `scripts/pcsx2-trace.py` now steps two verified
frames per recorded row (`PHYSICS_STEP_FRAMES = 2`) so that every row is a
real tick rather than half of them reading as the craft going motionless.
An earlier same-session check for whether `dt` is measured or fixed - a
memory diff across one verified step that found 58 words reading a stable
`0.02` - is superseded by this finding: that test most likely straddled a
field pair with no real tick in it either, so it is not evidence about the
true per-tick `dt`, which would be nearer `0.04` if fixed at all.

## Steering from a plan, not a human

`--hold cross` alone cannot reach anywhere that needs a turn - it is throttle
with no steering, and on Moa Therma that puts the craft into a wall by frame
~520. Reaching a specific point on a track - a speed pad, a corner apex - needs
either a human driving by hand once and a savestate, or a scripted turn
sequence. **`oag-trace plan` already generates the second one, for our own
simulation, and it transfers to PCSX2 verbatim**, because the plan's script
format is the same abstract button layer this harness's own bindings speak:

```sh
cargo run -p oag-trace -- plan --source data/images/pulse-ps2-eu.chd \
    --track 'Data\Environments\03_Track\track.vex' --team Assegai \
    --class venom --pad 0 --look-min 18 --look-speed 0.35 \
    --out /tmp/pad0.inputs
```

writes a run-length script - `<ticks> <buttons...>` per line, `none` for
nothing held - that drives *our* physics through pad 0's trigger box. Measured
on Moa Therma's first speed pad: the default lookahead misses the pad's own
half-width (11.45 units off against a 4.8 half-width), but `--look-min 18
--look-speed 0.35` lands inside the trigger box from tick 475 of 538 and
crosses the gate 2.31 units off centre. The parameters are a plan-quality knob,
not a physics one - sweep them per target rather than assume the defaults work.

The script's tokens map straight onto `pcsx2-drive.py`'s abstract buttons:
`cross`->`cross`, `left`/`right` unchanged, and `l`/`r` (the airbrakes,
`oag_gameplay::Button::L`/`R`) to `l1`/`r1`. Replaying it is then the same
verified `advance_frames` machinery `cmd_frames` already uses, just called once
per row instead of once per whole command: pause, `loadstate` the grid save,
re-pause, then for each row `Keyboard.down`/`up` the buttons that changed and
`advance_frames(pine, keyboard, ticks)` for that row's count before moving to
the next. Because every frame is verified rather than timed, the whole
sequence reproduces exactly: replaying `pad0.inputs` this way stepped exactly
538 frames with no drops or overshoots, landing within the pad's own trigger
box on the first attempt - a maneuver a hand-driven session had already failed
at once. At the harness's own frame rate for a *verified* step (about three a
second, see the `FrameAdvance` measurement above), a few hundred ticks costs a
few minutes, which is why this is worth doing for "reach point X on the track"
in general rather than only for this one pad.

This does not replace a human-driven savestate for a race actually *played*
start to finish - the plan's steering is a pursuit controller tuned to arrive
somewhere, not a lap the original game would call clean - but for "get a real
console frame at this specific point on the track," it turns a manual driving
session into an unattended one.

## GS dumps: what the console actually drew

A savestate and a frame step tell you what the game *set up*; a GS dump tells you
what the GS was *told*: every register write and every GIF packet of one frame.
It is the PCSX2 counterpart of the PSP's `gpu.record.dump`, and it settled the
PS2 shield (`ps2-pulse-eu/shield-pickup.md`) in an afternoon that three rounds of
reading the executable had not.

**The hotkey is `GSDumpSingleFrame`** (`GSDumpMultiFrame` is the other). Bind it
in `[Hotkeys]` like the harness binds `FrameAdvance`:

```ini
[Hotkeys]
GSDumpSingleFrame = Keyboard/F9
```

The id is not `SaveSingleFrameGSDump`, which is what the menu entry's label
("Save Single Frame GS Dump") suggests and silently does nothing. Tell: no
`OSD [GSDump]: Saving single frame GS dump...` line in the log after the tap.
`strings` on `pcsx2-qt` finds the ids next to `ToggleSoftwareRendering`.

How a dump is taken from a paused savestate, with the verified stepping this page
already has:

1. Pause, `loadstate`, re-pause, do whatever the capture needs (write memory).
2. `advance_frames(N)` to the state you want.
3. Tap the hotkey, then **advance at least six more frames and wait several
   seconds.** The file is written on a worker thread and is closed by a later
   frame; read at once it is a truncated Zstandard stream (`Read error (39):
   premature end`, about 1.3 MB of 7 MB). `zstd -dc` salvages a partial one, which
   is how the first dump was found to parse, and is not a complete frame.

The dump lands in `<datapath>/PCSX2/snaps/` as `<title>_<serial>_<stamp>.gs.zst`
with a PNG beside it. It records **two fields** of an interlaced PAL frame (two
`vsync` packets with `field` 0 and 1), so a draw appears twice, with different
scroll phases.

**Reading it:** `scripts/pcsx2-gsdump.py frame.gs` lists the state groups and
`--draw N` prints one group's registers and vertices; its docstring has the file
layout. Traps that cost this page time:

- **Texture-pool addresses are not stable.** The game uploads every texture it
  draws into a streaming VRAM pool each frame (`PSMT8` as `PSMCT32` at half
  size - `dbp ... 64x64`, 16,384 bytes for a 128x128), so the `TBP0` of the same
  texture differs between two runs of the same savestate. Diff a frame against a
  control by *sequence* (`difflib` on `PRIM`, primitive count, `ALPHA`, `TEST`,
  texture shape), never by `TEX0`. The shell was the one group inserted.
- **The upload is the texture file's transfer packet.** The bytes in the `IMAGE`
  data are what sits in EE RAM, so a search of a PINE read of EE RAM (1.6 s for
  32 MB) finds the source, and the name string stored just before it names the
  texture.
- **The `CLUT` is stored `CSM1`-swizzled**: entry `i` is at
  `(i & ~0x18) | ((i & 8) << 1) | ((i & 0x10) >> 1)`. Read linearly, a 15-colour
  palette looks like 9 colours with two alphas.
- **`XYOFFSET` is not `2048`.** Pulse sets `0x7000` (`1792.0`), so a screen
  position is `xy - 1792` (the last `XYOFFSET_1` in the dump), inside a scissor of
  `0..511` both ways - the 512x512 buffer this page already measured.

Confidence 90 for the method: it reproduced the same draw's registers to the bit
in eight dumps.

## The walk into a race

Measured 2026-08-23 from a cold boot with an **unformatted** memory card, which
is the first-run path and the long one. Every step is a screenshot; the game
gives no `TTY.log` equivalent to key on (see below).

| # | Screen | Input |
| --- | --- | --- |
| 1 | autosave warning | `cross` |
| 2 | "switch to 60 Hz mode?" | `down`, `cross` (i.e. **no** - stay at PAL 50 Hz) |
| 3 | title, "press start button" | `start` |
| 4 | `ENTER YOUR NAME` | `right` x11, `cross` (walks the cursor to `CONFIRM`) |
| 5 | `ENTER YOUR TAG` | `right` x3, `cross` |
| 6 | `FORMAT` memory card? | `down`, `cross` |
| 7 | `SAVE PROFILE`, slot list | `cross` |
| 8 | `SAVE OK` | `cross` |
| 9 | `MAIN MENU` | `cross` (Race Campaign) |
| 10 | `RACE CAMPAIGN`, Grid 1 | `cross` |
| 11 | Single Race, Moa Therma White, Venom, 3 laps | `cross` |
| 12 | `SHIP SELECT`, Assegai | `cross` |
| 13 | loading | wait ~60 s |
| 14 | on the grid, "GO" | - |

Steps 4-8 happen **once**: the memory card lives in the harness's own data path,
so every later boot goes 1, 2, 3, 9. And once step 14 is reached, a savestate
makes the whole walk a `loadstate`. The recommended shape is to walk once by
hand, save a state on the grid, and script from there.

The 60 Hz prompt at step 2 is answered `no` on purpose: PAL 50 Hz is what the
disc is, and the mode is a rendering-rate change nothing else in this project
models.

**The prompt language varies between boots** - German on one run, Spanish on the
next, English on a third - so a walk cannot be keyed on the text. Why it varies
was not established (the BIOS language setting is the obvious candidate,
confidence 50) and it does not matter: the button sequence is identical either
way.

## The state channel: EE console, and its limit

`EnableEEConsole` and `EnableIOPConsole` put the game's own `printf` into the
log, and Pulse prints plenty at boot:

```
loadmodule: fname cdrom0:IOPSTREAM.IRX;1 args 0 arg
Exit MultiStream V7.3 Thread Load
UpdateVSyncRate: Mode Changed to PAL.
Set GS CRTC configuration. PAL  640x512 @ 50.000 (49.76) Interlaced (FIELD)
```

**It has no front-end transitions in it.** HD's `Switching Screen "Main Menu" to
"Campaign Selection"` has no counterpart here - nothing is printed when a menu
changes, so there is nothing to wait on and a walk is either screenshot-driven
or memory-driven. That is a real regression against the PS3 harness, and the
compensation is that memory *is* cheap here: a front-end state word, once
someone finds one, would be a 70-microsecond read rather than a log tail.

## What was tried and rejected

| Tried | Result |
| --- | --- |
| `pcsx2-qt --help` (two dashes) | exits 1, prints nothing. It is `-help`, one dash, for every option. |
| Vulkan / `Renderer = -1` (Auto) on Xvfb | no present queue, GS fails to initialise, process exits |
| `/dev/uinput` virtual pad, as RPCS3 uses | invisible to PCSX2's keyboard source; Xvfb has no evdev driver |
| `-fullscreen` on the virtual display | ignored, window stays 640x480; no window manager to honour it. A direct `ConfigureWindow` resize *does* work |
| `ScreenshotSize = 0` or `1` | 640x480 and 682x512, both resampled off the 512x512 buffer the console draws |
| PINE opcodes `0xF0`-`0xF7` | all `FAIL`; the metadata commands are `0x08`-`0x0F` |
| PINE request of 640 KiB | no reply at all, client hangs; keep under 256 KiB |
| `F8` while the VM is paused | writes nothing; use an X grab for paused frames |
| Unverified `FrameAdvance` taps | 60 taps moved the guest 61 and 66 frames on two runs |
| Wall-clock-timed thrust from a savestate | two runs 2.4 % RMSE apart - close, and not a comparison |

One more, smaller, worth writing down because it produced a confusing error
message rather than a wrong result: `scripts/pcsx2_pine.py loadstate 2`
originally took a positional called `slot`, which argparse merged with the
top-level `--slot` for the *PINE* slot. It then looked for `pcsx2.sock.2` and
reported "no PINE socket ... is PCSX2 running?" while PCSX2 was running fine.
The positional is `index` now.

## What is worth doing next, in order

1. **A front-end state word.** The one thing the PS3 harness has that this does
   not is a screen-transition channel. Here it would be better than there:
   memory is 70 microseconds a read, so a `press_until(screen)` would be
   cheaper and more reliable than RPCS3's log tail. Finding it is a diff of EE
   RAM across a menu transition, which this harness can now do in one command.
2. **A per-frame trace.** Verified stepping plus a batch read is an M3-shaped
   capture channel; what does not exist is the schema - which addresses, and the
   comparison against `oag-trace`. The ship state block is the obvious first
   target and is not yet located in the PS2 corpus.
3. **Name the frame counter**, with an evidence page, once someone is in Ghidra
   on `SCES_547.48` anyway. Two candidates, both measured, neither read.
4. **The PS2/PSP corroboration this makes cheap.** `input.md`'s claim that the
   abstract button layer survives the port is now checkable by holding a button
   on each machine and reading both masks, which is the 95-band evidence the
   rubric asks for.
