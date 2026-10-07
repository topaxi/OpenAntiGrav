# Capturing a frame of Wipeout 2048 from Vita3K

The first time this project drove the Vita title. Measured 2026-09-16 with
`Vita3K v0.2.1 4095-84184a36` (the Arch package `vita3k r4095.84184a363-1`,
the Qt build) and Wipeout 2048 **EU `PCSF00007` v1.04** with both DLC packs
installed into Vita3K's `ux0/`. That is the same build `data/extracted/vita/
PCSF00007` is extracted from and the same one the Ghidra program
`/vita-2048-eu-v104` is, so a frame captured this way is directly comparable
with the layouts read off the package and with the executable's reading. The
USA base `PCSA00015` v1.00 is installed beside it and runs the same way
(`-r PCSA00015`); it was not used.

What it produced: `oag_2048::hud::ALWAYS_ON` and the Vita coordinate space
`oag_display::space::Space::VITA` - see [2048-hud.md](../formats/2048-hud.md).

## The one thing that did not work, and the fix

**Xvfb cannot present a Vulkan swapchain from a real GPU.** Vita3K creates its
Vulkan device fine on an Xvfb display (it enumerates the Radeon), then fails
`Failed to select proper Vulkan queues. This is likely a bug.` and shows a
`Failed to initialise the renderer.` dialog. The Mesa line above it in the log
is the real cause:

```
MESA: info: vulkan: No DRI3 support detected - required for presentation
```

Xvfb has no DRI3, so radv has no way to present. The OpenGL backend under
Xvfb would be llvmpipe, and lavapipe would be the Vulkan equivalent - both
software, both far slower than the title needs.

**The fix is a headless Wayland compositor on the real GPU with a rooted
Xwayland on top.** Weston's headless backend renders through EGL on the
Radeon, Xwayland speaks DRI3 to it over dmabuf, and Vita3K's X11 window on
that Xwayland presents at full speed - 60 fps in the front end, 30 fps in a
race, on an RX 7800 XT. Nothing touches the user's own session: it is a
separate Wayland socket and a separate X display, so the user locking their
screen mid-capture (which happened) changes nothing.

```sh
# 1. Headless weston on the GPU, its own socket, GL renderer.
weston --backend=headless --renderer=gl --width=1280 --height=800 \
    --socket=wayland-oag94 --idle-time=0 &

# 2. A rooted Xwayland :94 on it. Unset DISPLAY so it does not nest itself.
env -u DISPLAY WAYLAND_DISPLAY=wayland-oag94 Xwayland :94 -noreset -geometry 1280x800 &

# 3. Vita3K on :94 through X11. Both SDL spellings, and WAYLAND_DISPLAY
#    unset: with it set, SDL picks Wayland, connects to the *user's*
#    compositor, and dies with `xdg_surface: must ack the initial configure`.
env -u WAYLAND_DISPLAY DISPLAY=:94 SDL_VIDEODRIVER=x11 SDL_VIDEO_DRIVER=x11 \
    vita3k -c ~/.cache/oag/2048-hud/config.yml -w -r PCSF00007 &
```

`-c <file>.yml` reads a configuration file from elsewhere and `-w` keeps
Vita3K from writing it back, so the user's `~/.config/Vita3K/config.yml` is
copied once and never modified. The copy differs from the user's in five
lines only: `log-level: 2` (the user's `0` is TRACE and floods),
`discord-rich-presence: false`, `show-compile-shaders: false` (that overlay
would land in the frames), `validation-layer: false`, and
`check-for-updates-mode: 0`. `pref-path` stays the user's, which is where
the installed title and its shader cache live.

The log goes to `~/.cache/Vita3K/vita3k.log` and to stdout. Thousands of
`Unhandled SIGSEGV at rip ...` lines on stdout are Vita3K's guest-memory
fault handler doing its job, not a crash - `grep -v SIGSEGV` and read what is
left.

## The game window, and reading it at 1:1

Vita3K's Qt window is 1280x720 with an app list and a log pane; the emulated
screen is a child window of its own, titled

```
WipEout® 2048 (PCSF00007) | Vulkan | 30 FPS (34 ms) | 960x544 | Bilinear
```

and sized exactly 960x544 at `resolution-multiplier: 1` (check with `xdotool
getwindowgeometry`). Capture **that** window, not the root:

```sh
WID=$(DISPLAY=:94 xdotool search --name 'PCS[AF]000' | head -1)
DISPLAY=:94 import -window "$WID" frame.png
```

The PNG is then the Vita's own 960x544 pixel grid with its origin at the
window's origin, so a HUD widget's authored `x`/`y` can be read straight off
it. There is no `xwininfo` on this machine; `xdotool` does the same job.

## Input: three kinds, and two of them need care

- **Buttons** are keyboard keys per the config (`cross=x circle=c square=z
  triangle=v start=Return select=Shift_R`, d-pad the arrow keys, left stick
  `w/a/s/d`, `L1=q R1=e L2=u R2=o`). Send them with XTEST after focusing the
  game window: `xdotool windowfocus --sync $WID; xdotool key x`.
  `xdotool key --window` uses `XSendEvent`, which SDL ignores.
- **Touch** is the mouse on the game window, and **a plain click is too short
  to register.** `xdotool click 1` did nothing on the Game Mode grid twice;
  `mousedown 1; sleep 0.3; mouseup 1` at the same spot opened it. Every menu
  in this title is touch-first - the Game Mode grid, the campaign nodes, the
  Play/confirm buttons - so almost all navigation is taps.
- **Accelerate is R1 (`e`), not cross.** Holding `x` for eighteen seconds on
  the grid left the craft where it was; holding `e` moved it. Square (`z`)
  did **not** fire the held weapon - what does is unmeasured.

A held key is `xdotool keydown e` ... `keyup e` around a screenshot loop.

## Walking into a race on a fresh save

Touch coordinates are in the 960x544 game window, which is at the display's
origin, so they double as root coordinates.

| Step | Screen | Action |
| --- | --- | --- |
| 1 | intro movie, then the title's 3D attract scene | `key Return`, then `key x` |
| 2 | "Access to online features ... requires a PSN account" | `key x` |
| 3 | GAME MODE grid | tap `(295, 162)` Single Player Campaign |
| 4 | welcome text | tap `(882, 480)` the checkmark |
| 5 | campaign map, "TOUCH TO START" on the first node | tap `(487, 270)` |
| 6 | event card (Empire Climb, No Weapons) | tap `(882, 480)` Play |
| 7 | mode description card | tap `(882, 480)` |
| 8 | ship intro | tap `(882, 480)`, wait ~20 s for the load |
| 9 | the race, HUD up, craft on the grid, clock running | `keydown e` |

The first event is a **no-weapons** race. Finishing it (8th is a pass -
"finish the event") unlocks a weapons race (Queens Mall, offensive weapons
only), and after that a time trial (Metro Park, beat 2:10, which Extreme
assist passed at 2:02.95). The fourth event needs 5th place, which assist-only
driving does not reach, so the single-player campaign's Zone event was not
reached. **The HD campaign (DLC) opens its first group at once**: Game Mode,
tap `(477, 162)` HD Campaign, and the `Uplift` group has a Race and a **Zone**
event on Vineta K (`(505, 345)` and `(600, 395)`) with nothing to unlock
first - that is where the Zone frames came from.

**Pilot Assist: Extreme** is what makes an unattended lap possible. Pause
(`key Return`), tap `(139, 192)` Options, `(296, 322)` Pilot Assist, the
right arrow `(910, 243)` once (Normal to Extreme), `(880, 477)` confirm,
`(880, 477)` resume. With it, holding `e` alone gets the craft round every
circuit tried at about 1:30 a lap, hitting walls for a few percent of shield
each. It also puts the `PilotAssist` HUD icon up, which is why that widget is
recorded as state-gated rather than always-on. Zone needs no input at all.

Pause menu: `(620, 477)` quits (then `(402, 312)` confirms), `(880, 477)`
resumes. Post-race summary: `(710, 480)` accepts.

## Cleanup

`scripts/vita3k-drive.py stop` does this now - Vita3K first so it saves its
pipeline cache, then Xwayland, then weston, and only the pair this tooling's
own `display`/`boot`/`race` started (the same ownership-marker idiom
`scripts/xvfb_display.py` uses for `Xvfb`, never a bare `pkill`). Nothing
persistent is left behind except the campaign save on the user's own
`pref-path` (the events above are now completed on it) and the pilot-assist
option, which is stored in that save.

## `scripts/vita3k-drive.py`, on the model of `scripts/rpcs3-drive.py`

Written 2026-09-20, replacing the twenty-line-each `launch.sh`, `xwayland.sh`,
`shot.sh`, `key.sh`, `tap.sh`, `hold.sh`, `race.sh` this recipe's own pass
used by hand (still under `~/.cache/oag/2048-hud/` beside the frames, and
`strip.sh` still has no Python equivalent - it tiles a capture sequence for
eyeballing, which this recipe's own steps already do one shot at a time).
Subcommands: `display`, `stop`, `boot`, `race`, `shot`, `tap`, `key`, `hold` -
see the script's own module docstring for each.

**`display`/`boot`/`stop` are solid, checked end to end.** `tap`/`key`/`hold`/
`shot` are the same primitives the shell scripts used, individually checked
by driving a real race by hand: `python3 scripts/vita3k-drive.py boot`, then
`tap`/`shot` in a loop watching each frame, reached Metro Park's Time Trial
on this machine's own (already-progressed, not fresh) save and `hold e 6`
moved the craft for a real ~10 seconds (`TOTAL 0.00.00` to `0.10.29`, `XP 0`
to `20`).

**`race`'s own fixed menu walk is not proven reliable unattended**, even
though every primitive it is built from is. It targets Empire Climb the way
this page's own walk-in table does, from a save this fresh - but on this
machine's actual save (already three events in, from this same recipe's own
prior pass), the campaign map's pan position no longer matches the
coordinates, and even the Game Mode grid's own two-tap select-then-confirm
did not reproduce back-to-back inside an unattended run the way driving it
by hand did. `FIRST_EVENT_TAPS`'s own doc comment in the script has the
measurement. `race --hold-seconds N --screenshot out.png` is still the
one-command path to try; `boot` then `tap`/`key`/`hold` by hand for whatever
a session's own save and map actually show is the reliable fallback, no
different in kind from a human adapting the recipe above by eye.

## Reading and writing guest memory, and the 2048 craft teleport (2026-10-07, vita3k-teleport)

**Verdict: the memory mechanism works (confidence 90); `place` is not built, because
the player's physics body was not isolated (negative, see below).**

### The mechanism that works: `/proc/<pid>/mem` on a child process

- **Guest address = host address - `0x400000000`** (confidence 90, read on two boots).
  The guest's 4 GiB window is one host reservation; `/proc/<pid>/maps` shows the
  eboot's `rw-p` segment at `0x481000000` for the guest `0x81000000`, and a read of
  `0x481000000` returns the eboot's own first bytes (`30bd10b54ff60030`, the same as
  the file). Heap and data are the `rw-p` ranges inside `0x4605fa000-0x488be8000`
  (about 330 MiB, 240 ranges).
- **Only an ancestor may open it.** `ptrace_scope` is 1 here, so a Vita3K started by
  an earlier command and re-parented to init cannot be read. A tool has to start
  Vita3K itself (`launch_vita3k` in `scripts/vita3k-drive.py`, as a `subprocess`
  child) and keep running; the lane's probe was a small daemon that did exactly that
  and served `R`/`W`/`STOP`/`CONT`/`DUMP` on a unix socket. `place` therefore has to
  be one run: boot, walk to the race, place, photograph.
- **Pause with `SIGSTOP`/`SIGCONT`.** A stop-the-world dump of all 328 MiB takes
  0.5 s, so two consistent dumps a second apart are cheap. A `W` while stopped is
  seen by the guest after `SIGCONT`. A harmless write (rewrite a float with itself,
  then a 3-unit y offset on a body) was read back exactly.
- `scripts/vita3k-drive.py` now honours `OAG_VITA3K_CACHE` and `OAG_VITA3K_CONFIG`
  so a second instance has its own pid file, frames and config.

### The gdbstub does not do this (negative, confidence 85)

`gdbstub: true` in the config opens port 2159, **accepts one connection and then the
listener is gone**. `?` answers `S05`, `m<addr>,<len>` reads guest memory and `g`
returns zeros. After `vCont;c` (plain `c` is unsupported) the stub never answers again:
`m`, the `0x03` interrupt and an empty packet all time out for the rest of the race, so
there is no way to pause a running guest. With the stub enabled the guest also starts
halted until a client sends `vCont;c`. Use `/proc/<pid>/mem`.

### Traps

- **A `:95` that already answers is not yours.** `xdpyinfo -display :95` succeeded on
  this machine because the user's `xwayland-satellite` session accepts any number;
  `display` reported "already up" and would have put Vita3K on the maintainer's
  desktop. Use a number that gives `unable to open display` first (`:96` here).
- **A pref-path of its own**: `ux0/app`, `ux0/addcont` and `ux0/license` symlinked
  from the real one, `ux0/user` copied (4 MiB), config copied with `pref-path` and
  `audio-volume: 0` changed. The save in the copy is the user's progress, so the HD
  campaign's Vineta K race (`(477,162)` Game Mode, `(505,345)`, `(882,480)` twice)
  is reachable from it.
- **A guest write to a render copy is overwritten within 0.1 s, but only while the
  craft moves.** A craft resting on a wall shows no overwrite for a second or more,
  so "the write stuck" proves nothing there.
- **Restart the race to get a still grid**: pause (`Return`), tap `(750,480)`, then
  `(402,312)`. The countdown stands for about 4 s starting 4.5 s after the tap.

### What was found about the 2048 craft (Vineta K, EU v1.04)

| Fact | Evidence | Confidence |
| --- | --- | --- |
| Craft array `0x818a6bf8`: 8 pointers, vtable `0x81511d00` | the only 8-pointer array with one vtable and objects 0x6000+ apart that sits in the eboot data; positions read through it are all on Vineta K | 70 |
| `ship+0x7060` is `0` on exactly one ship (index 7 on this boot), `0xffffffff` on the other seven | diff of the eight objects' first 0x8000 bytes (HD has the same role word at `+0x7a60`) | 70 |
| `*(ship+0x5f84)` is a body whose `+0x08/+0x18/+0x28` are three unit basis rows (w 0), `+0x38` is the position (w 1), `+0x48/+0x58/+0x68` the transpose, `+0x90` a vector that reads as linear velocity | `camera.md` already documents `bodyPos = *(*(ship+0x5f84)+0x38)`; the layout read live on all 8 ships | 65 |
| The same position appears as 0.75-scaled rows then a position in 35 render-side copies (`0x874a8a30`, `0x82252b00`, `0x815f0d68` in the eboot data...) | `g_craft_scale` is 0.75; all 35 were rewritten to a unique value and snapped back | 80 |

**The unresolved part, and why `place` stops here.** Writing the body at `+0x38`
(position, basis, transpose, zero `+0x90`) is accepted and the body hovers and
settles with no respawn, but **the body of ship index 7 is not the craft the camera
follows**: with the visible craft at `(-728, 5.3, -178)` the body read
`(-819, -148, 81)` and did not move, the seven others raced at 80-100 units/s, and
after a teleport to an AI's pose it was pulled back to the visible craft's old
place within 2 s (`-645,36.7,-240.7` to `-769.8,-4.7,-161.1`). Either index 7 is a
ghost or an inactive eighth entity and the player body is reached another way, or
the body is slaved to the visible craft. The visible craft did not respond to a
held accelerate in the last test either (copy delta 0 over 0.4 s), so the "player
moves under my input" discriminator was never obtained on this boot. Not tried:
a match of the visible craft's copies against a body field, steering-only input
(which changes the player and no AI), a stable chain from a pointer to the body
stored in an object the camera rig reads (`Ship_UpdateCameraRigs` at `0x811bd89a`).

**Coordinates.** Vineta K in 2048 is not at HD's coordinates: the grid sat near
`(118, 35, -97)` (slot centre, forward along `-x`) against HD's `(6, -52, -196)`.
No transform to `oag-game` was measured: our build could not resolve the DLC's
Vineta K entry name (`Data\art\published\environments\<stem>\track.vex` with
`vineta`, `vineta_k`, `vinetak`, `vinetta` all missing), so no pair was taken.

**Omega: not checkable** (no PS4 emulator). **HD:** the same body shape as
`rpcs3_place.py` (basis rows, w-0 rows, position w 1, transpose after them), with
different offsets and little-endian; the role-word trick carries over.

## What a frame is, legally

A screenshot of the running game is game content, so no frame is committed.
They stay under `~/.cache/oag/2048-hud/frames/` and the docs cite them by
name and describe what each shows - see [legal.md](../overview/legal.md).
