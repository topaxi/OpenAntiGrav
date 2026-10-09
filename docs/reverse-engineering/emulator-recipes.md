# Emulator capture recipes: keep it running, fail fast, save states

Measured 2026-10-08/09 (lane `emu-tooling`) on the host's RPCS3 and PPSSPP v1.20.4.
The reason this page exists: on 2026-10-08 the 37 members spent about 9.6 h waiting
on background capture logs and 24 waits ran into the 10 minute tool ceiling (about
4 h), many on runs that had stalled at a boot or a menu and were retried the same
way. Three rules follow, each backed by a tool.

1. **Keep the emulator running between captures.** Reach the next state from the
   current one (restart, quit to a menu, a save state, the menu walk) instead of
   booting fresh. Boot only when the measurement needs the boot (boot-sequence RE).
2. **A capture says where it stalled, within about two minutes.** Run it in the
   foreground under `scripts/emu-run.py`, or wait on its status file; never poll a
   log for a success marker.
3. **Save states restore a drivable race in about 10 s on HD (RPCS3) and 2 s on Pulse (PPSSPP)** (below);
   take one at a point you will reuse, and boot fresh only when the measurement needs the boot.

## Tools

| Tool | What it does |
| --- | --- |
| `scripts/emu-env.sh <lane> <display> <gdb-port> [ppsspp-port]` | Builds a member's private RPCS3 tree (symlinked firmware, copied `dev_hdd0`, pad profile naming `OAG Pad <lane>`) and `data/scratch/<lane>/env.sh`. About 2 GB, one command. |
| `scripts/emu_guard.py` | The watchdog every RPCS3 `Session` now carries. Stages (`boot`, `menu:<screen>`, `loading`, `countdown`, `capture`), a heartbeat from every GDB packet, `<log-dir>/status.json`, and on a stall: `STALLED at <stage> after <n>s`, the emulator's process group killed **by pid**, exit status 3. In `boot` and `menu:<screen>` only a screen change counts as progress (a press does not), so a walk parked on one screen stalls 90 s after its last screen change (boot: 100 s). Falsified-then-confirmed 2026-10-09: with presses ignored, `STALLED at menu:HUD after 91s`, emulator gone, exit 3. `OAG_EMU_STALL=<s>` overrides every stage limit, `OAG_EMU_TOTAL=<s>` adds a whole-run ceiling. |
| `scripts/emu-run.py [--silence 150] [--total 570] [--child-status <log-dir>/status.json] -- CMD...` | Foreground wrapper for any capture (RPCS3, PPSSPP, PCSX2). No output for `--silence` seconds, or a guarded child idle that long, is a stall: it prints the last line and the child's stage, kills the group by pid, exits 3. `--total` (default 570, under the tool ceiling) exits 124. |
| `scripts/emu-run.py --bg --status F -- CMD...` and `scripts/emu-run.py wait F [--max 540]` | The detached form for a long run, and the correct way to wait for it: prints each stage change, returns 0 done, 1 failed, 3 stalled, 4 still healthy at `--max` (call it again). |
| `scripts/rpcs3-drive.py serve` / `status` / `press` / `stop` | A **long-lived** RPCS3: boot once, hold the pad and a GDB proxy, stay up. Scripts built on `Session` attach with `OAG_RPCS3_ATTACH=1`. |
| `scripts/ppsspp-start.sh <lane> <display> <port> <image>` | A private muted PPSSPP SDL under its own Xvfb, stopped by recorded pid (`... <lane> stop`). |
| `scripts/psp-state.py` | PPSSPP save and load through the pause menu. |
| `scripts/emu-rebuild-states.sh <lane> pulse-grid\|hd-grid [name]` | Rebuilds a state from the user's own disc into `data/saves/`. |
| `scripts/emu-restore-state.sh <lane> <state> [--restart] [--no-gdb]` | Restores an RPCS3 state into a long-lived `serve` with pad, GDB proxy and memory, in about 10 s. `scripts/emu-save-state.sh <lane> <name>` takes one from a running `serve`; `scripts/rpcs3-state-probe.py` checks GDB and memory after a restore. |

### How attaching works, and what it costs

`serve` starts RPCS3 once and then owns three things a script cannot share any other
way: the **virtual pad** (a kernel device that dies with its process, and RPCS3 binds
it only at start), **`/proc/<pid>/mem`** (Yama `ptrace_scope=1` lets only an ancestor
read it) and the **GDB stub** (one client per launch, and the server thread dies for
good when it leaves, which also shows RPCS3's "likely crashed" banner;
[rpcs3-debugger.md](rpcs3-debugger.md) trap 2). It serves the pad and memory over a
unix socket and fronts the stub with `scripts/rpcs3_gdb_proxy.py`: a script connects
to its usual port (`OAG_RPCS3_GDB`), the proxy pauses the target on connect, and
resumes it when the script leaves, so the next script meets what a fresh launch gave
it. Every script that goes through `rpcs3-drive.py`'s `Session` needs no change:
`session.pad`, `session.proc.pid`, `session.open_mem()` and `Debugger(port=...)` work
attached. A script that wants `interpreter=True` or its own `config=` must be served
with the same flags (it refuses otherwise).

```sh
scripts/emu-env.sh <lane> 94 23494 45494 ; source data/scratch/<lane>/env.sh
(nohup uv run --with evdev python3 -u scripts/rpcs3-drive.py --image <abs iso> \
   --log-dir $W/serve serve > $W/serve.log 2>&1 &)        # waits for Main Menu, ~25-60 s
export OAG_RPCS3_ATTACH=1 OAG_MENU_SETTLE=4
uv run --with evdev python3 scripts/emu-run.py -- python3 -u scripts/rpcs3-hd-weapon.py \
   --image x --log-dir $W/w1 --state 9 --watch 4 --no-record      # attached: 46-62 s
python3 scripts/rpcs3-drive.py press start down down down down down --shot-dir $W/nav   # look around
python3 scripts/rpcs3-drive.py stop                       # by pid; touches nobody else's emulator
```

Between attached scripts the emulator is wherever the last one left it.
`wait_for_screen_pressing("Main Menu")` (every script's first step) quits a race to
`Cell Selection` of the same grid, and `walk_to_race` carries on from there, so the
next run costs 3 presses and a load, not a boot. Replacements for the fixed sleeps
that dominated the old flow:

| Was | Now | Measured |
| --- | --- | --- |
| `sleep(70)` after the walk | `session.wait_for_load(70)`: HD prints `Play welcome` once the race is up | loaded 5 s after `InGame` on a warm cache |
| `sleep(20)` after Main Menu | `session.settle_menu(20)`, `OAG_MENU_SETTLE=<s>` overrides | 4 s was enough on every attached run |

`rpcs3-drive.py stop` and `clear_stale_lock` used `pgrep/pkill -x rpcs3`, which saw
every member's emulator: one member's `stop` killed the others' runs, and while any
RPCS3 was up a dead run's `RPCS3.buf` was never cleared (the "Another instance"
modal, then a boot that reaches no screen). They key on a pidfile in the member's own
cache now; `stop --all` is the old behaviour, for the maintainer's own machine only.

### Why menu walks stalled, as far as measured

Classified from `data/scratch/*/*.log` on 2026-10-09 (confidence 70: logs, not a controlled test):

- **Boot never produced a screen** (`never reached the Main Menu (last screen: ?)`, 36 presses over
  180 s): one log (billboards-3/cap4.log), directly after `removed a stale RPCS3.buf`. 38 logs carry that
  stale-lock line, i.e. the run before them was killed without cleanup. `clear_stale_lock` could not
  clear a dead run's lock while anybody's RPCS3 was up (it asked `pgrep -x rpcs3`), and RPCS3 then shows
  an "Another instance" modal that looks like a game that booted and stalled. Now keyed on this run's
  pidfile, and it refuses to remove a lock a process still has open (a hand-started RPCS3).
- **The walk itself** (hd-motion-blur run2/3/4): all reached `InGame` in 11 presses. The "(dropped)" lines
  are a fixed pattern (5 s per screen, ~25 s per walk), not a failure; unchanged. The hours went after the
  walk: hook-ring retry loops, a GDB read nobody answered, `sleep(70)` loads.
- **A press landing on the START RACE prompt** (hd-ride-height b2-b4; reproduced 2026-10-09 as
  "FLYBY SKIPPED"): `Launch Game` to `HUD` changes inside the gap between a loop's screen check and its
  `cross`. The refusal now sits in `press_once` itself, immediately before the press, for the race
  screens and `Team Launch Transition`/`Launch Game`; `rpcs3-height.py`'s local patch is gone. Verified:
  two attached runs, 14 flyby rows, first height 2.13 and 2.13, first frame shows `START RACE`.
- **Attract mode (measured, new).** Idle on `Main Menu` for about a minute and HD starts its demo:
  `Demo Launch`, `Demo Launch Real`, `Demo InGame`, `HUD`. `HUD` is in `RACE_ARRIVED`, so a slow walk
  reported "in a race" on the demo. With presses ignored the walk reached `HUD` through the demo in 93 s.
  `Session.in_demo()` now reads it off `TTY.log`; the walk keeps pressing (any button leaves it) and an
  attach leaves a demo first. A race left idle is safe: 5 minutes parked in a running race stayed in the race.
- **Pad crosstalk: possible, seen once (confidence 40).** RPCS3 booted and walked with `OAG_RPCS3_PAD_NAME`
  set to a name its profile does not contain, so evdev binding is not by name alone and another member's pad
  could be read. Not observed as a fault. Each member's pad is still named per lane by `scripts/emu-env.sh`.
- Not found: a copied-config press drop, safe-area or profile differences.

## Save states and the other ways to reach a state

### RPCS3 (HD / Fury)

**Reset and navigation routes** (all measured on the grid of Talon's Junction):

| From | To | How | Cost |
| --- | --- | --- | --- |
| in a race | same grid, fly-over with START RACE | `session.restart_race()`: pause (`start`), `down` x5, `cross` (rows: Continue, Pilot Assist, Game Options, Audio Options, Photo Mode, View Invites (disabled, skipped), **Restart Race**, Quit Race) | **7.7 s** (twice) to the new `Play welcome` |
| in a race | `Cell Selection` of that grid | `session.leave_race()`: `start`, `down` x6, `cross` (**Quit Race**, no confirm) | 7 s |
| `Cell Selection` | race | `walk_to_race()`: `cross` x3 | **12.1 s** including the load |
| `Main Menu` | race | `walk_to_race()` | **28 s** including the load |
| any menu | `Main Menu` | `circle` repeatedly (not wired; the attached walk does not need it) | |

**RPCS3 save states: they save and restore reliably and are drivable (measured 2026-10-09,
second pass; the first pass of the same day, "1 write in 6, restores undrivable", was five
config mistakes, not an emulator limit).**

One command restores a race with a working pad, GDB proxy and `/proc` memory:

```sh
scripts/emu-env.sh <lane> 94 23494 ; source data/scratch/<lane>/env.sh
scripts/emu-restore-state.sh <lane> data/saves/hd-fury/<state>.SAVESTAT.zst   # about 10 s
OAG_RPCS3_ATTACH=1 uv run --with evdev python3 scripts/rpcs3-state-probe.py   # gdb + mem check
```

| Point (`data/saves/hd-fury/`) | Lands on | Next |
| --- | --- | --- |
| `grid-flyover-talons` | Campaign race, Talon's Junction grid fly-over, `START RACE` up | `cross` when the prompt shows (a `cross` before it is ignored) |
| `hd-racebox-talons-venom-weapons-grid` | Racebox Single Race, Venom, weapons ON, Feisar, 8 ships, fly-over | same; weapon pickups seen in the race |
| `hd-timetrial-talons-venom-start` | Racebox Time Trial, Venom, Feisar, 4.5 s after `GO`, ship at 0 km/h | drive at once |
| `hd-racebox-talons-venom-midrace-absorb` | Racebox race, 312 km/h, `Absorb` pickup held, lap 1 | drive at once |

`scripts/emu-restore-state.sh ... --restart` is for attached scripts that wait for TTY lines: a
restored run has none (`current_screen()` reads `?`), so the walk's first step finds no screen. After
**one** `cross` into a running race, `scripts/rpcs3-drive.py restart` (pause menu, Restart Race, 7.7-10 s)
makes the game print `Play welcome` and the screen reads `HUD`; from there everything attached works.
`restart` also knows Time Trial's pause menu (screen `InGame Pause SP Time Trial`, an extra `GHOST`
row, so Restart Race is 6 downs instead of 5). `--restart` itself does not press `START RACE` for
you, so it only suits a state taken in a running race.

**Restore time (corrected after a review: the first figure was when the GDB stub starts listening, not
when the game is back).** `emu-restore-state.sh` returns at `Savestate has been moved (hidden)`, the last
line of RPCS3's load: **9.5 and 10.5 s** from the command (mid-race and Time Trial states), with the first
visible game frame about 2 s later; before that the screen is RPCS3's `Linking PPU Modules...` splash
(frame 0.8 s after the return: mean luminance 0.03, then 0.47 and 0.63 at 3.5 s). The stub listens at
1.8 s and the pad binds at 1.4 s, so a script that waited for either pressed into a splash. Against
about 100-120 s for a cold boot, walk and load, and the 10 minute boots other lanes paid. Each of the four
states was checked after a restore: `rpcs3-state-probe.py` (the GDB proxy answers, a 16-byte read of the
player's craft through GDB equals the same read through `/proc/<pid>/mem`, three states), and the pad:
`cross` starts the race on the two grid states, `start` opens the pause menu on the mid-race and Time Trial
states (screens `InGame Pause SP` and `InGame Pause SP Time Trial` appear in `TTY.log`, so the screen
lines are back after the first switch). The mid-race frame after restore shows `Absorb` still held with the
race clock continuing (0:55.8 against 0:49.6 saved), which is also the evidence that the Racebox race
has weapons on; the `Machine Gun` banner that shows in both races is not (the Time Trial has weapons off).

**Why the saves failed and the restores were dead: five causes, all found in `RPCS3.log`.**

1. **`Compatible Savestate Mode` must be `true`** (it is in `rpcs3-debugger.md`'s table, and the generated
   config did not set it). Off: the write dies with `Thread terminated due to fatal error: Verification
   failed (object: 0x0)` at `cellSysutil.cpp:119 sysutil_cb_manager::save` and `Saving savestate failed
   due to fatal error!`.
2. **The GDB stub's client must be off while the state is taken.** With the proxy connected (stub on,
   `Compatible` on, 4 s gap, attempt 6, the control that separates this from cause 1 and 3) the overlay
   prints `Stopping emulator...` and then the join thread loops `Thread [GDB Server] is too sleepy. Waiting
   for it ...us` (doubling from 0.25 s past 33 s) and never writes. Stub listening with no client, with
   `Compatible` on and a 4 s gap, was not tried; the two no-client attempts that wrote nothing used a 1.5 s
   gap (cause 3). `OAG_RPCS3_NO_GDB=1` blanks `GDB Server`: without it the stock `config.yml`'s
   `127.0.0.1:2345` stays on, which is why earlier "stub off" runs were not off.
3. **A gap of about 4 s between opening the `SaveState` page and the confirming `cross`.** 1.6 s after
   it, the log shows `User selected savestate in home menu` and nothing else, the game carries on, and
   no file appears: the "1 write in 6" signature (seen three times, stub off and on).
4. **A load ignores `--config` and `--input-config`.** `rpcs3 --savestate` applies
   `custom_configs/config_<serial>.yml` and the global `input_configs/global/Default.yml` instead.
   `Used configuration:` after a load showed stock values (`GDB Server: 127.0.0.1:2345`, Compatible off),
   and `Pad 0: device='Keyboard'` instead of the virtual pad: the "dead pad, stub off" of the first pass.
   `serve --load-state` (`prepare_state_boot`) writes both into the private config tree, and refuses
   to run against `~/.config`.
5. **RPCS3 consumes the file it loads.** It renames it to `used_<name>` (or moves it to
   `savestates/used_<serial>/`), so a second boot from the same path opens a modal `No savestate file
   found or savestate not compatible` that reads as a boot hang (two "hangs" in this session, one with
   the log frozen at `Booting savestate from command line`). `--load-state` boots a private copy.

**Save recipe** (`scripts/emu-rebuild-states.sh <lane> hd-grid [name]`, or
`scripts/emu-save-state.sh <lane> <name>` on a running `serve`): `OAG_RPCS3_SUSPEND_STATE=1
OAG_RPCS3_COMPAT_STATE=1 OAG_RPCS3_NO_GDB=1`, no `OAG_RPCS3_GDB`; `ps`, `up` x3, `cross`, wait 4 s,
`cross`; the file (105-121 MB `.SAVESTAT.zst`) lands in 18 s and **the emulator exits** ("Save Emulation
State And Exit"), so a save is the last step of a session. **Five of five writes with the recipe**
(Talon's grid, rebuild, Racebox grid, Time Trial start, mid-race); none of the five earlier tries this
session had all three conditions (stub off, Compatible on, 4 s gap). A state taken after a `--no-gdb` restore is fine:
`emu-restore-state.sh --no-gdb`. Confidence 80: one lane, one machine, one RPCS3 build
(`0.0.42-19980-028d1e8f`), HD EU only.

Racebox route used for the states (`Main Menu`): `right`, `cross` (`Single Player` is the Racebox
setup page: `RACE TYPE` Single Race / Time Trial by `right`, Venom, weapons ON, novice AI),
`cross` (`Track Select`), `right` x8 for Talon's Junction (the ninth of twelve), `cross` (Feisar is
the default), then `walk_to_race`. Throttle is `cross` held (`RemotePad.press("cross", seconds)`), not `r2`.

Fallback: `serve` and the routes above.

### PPSSPP (Pulse, Pure)

PPSSPP keeps running by design (scripts attach to its debugger port), so the routes are
the existing `psp-drive.py` subcommands: `menu` (cold boot to Time Trial grid, **90 s**
including first-boot profile questions), `restart` (pause, RESTART RACE, countdown), `state`.

**Save states work and restore fast.** The debugger has no
save-state command and `xdotool key F2`/`F4` write nothing on a bare Xvfb. The pause menu
does: `Escape`, then a click built from `mousemove`, 0.3 s, `mousedown`, 0.2 s, `mouseup`
(a bare `xdotool click` is ignored) on a slot's `Save state` (x=280) or `Load state`
(x=406), `y = 28 + 96 * (slot - 1)` in the 960x544 window. `Escape` toggles, so
`scripts/psp-state.py` reads the menu's state from whether the CPU cycle counter is
standing still. Measured on Pulse USA, Time Trial, Talon's Junction, VENOM, on the grid
after the countdown:

| Step | Result |
| --- | --- |
| save | 9.5 MB `.ppst` (+ 130 KB `.jpg`) in the instance's own `PPSSPP_STATE/` |
| load | **1.9 s** by the menu, 5 s wall with the pose read, against 90 s for `menu` |
| restored pose | `(5.65, -50.08, -196.67)` on both loads, after driving the craft 5-6 m away in between (two loads, one boot: confidence 55). Not compared against the menu walk here; `verification-protocol.md`'s 2026-07-30 figure (0.031 units between two loads, no tighter than the walk) stands untouched |
| load from a file | `psp-state.py load 2 --file data/saves/pulse-psp/time-trial-talons-venom-grid.ppst` copies it into the slot name and loads it |

So for a **weapon or visual capture, where the craft's exact tick matters less**, a grid
state is a fast start: 2 s instead of 90 s. For a **trace compared tick for tick**, the
verification protocol's built start (`--start-heading`) still stands. The state is tied to
the `.chd`'s own serial (`UCUS98712`), so a USA state does not load on the EU disc.

Good PPSSPP points: `data/saves/pulse-psp/time-trial-talons-venom-grid.ppst`
(`scripts/emu-rebuild-states.sh <lane> pulse-grid`). Other circuits and teams: run
`psp-drive.py menu` with the wanted selection (`--track-down`, see its `--help`), then
`psp-state.py save <slot> --keep data/saves/pulse-psp/<point>.ppst`. A pre-fly-by or
mid-race point is taken the same way at that moment. Pure and the EU disc: not measured here, and `psp-state.py load --file` needs `--serial
<SERIAL>_<VERSION>` for them (the default is Pulse USA's `UCUS98712_1.00`); a state is per disc serial. Fallback where a state
does not restore: `psp-drive.py menu` / `restart`.

Everything under `data/saves/` is game-derived and gitignored; members reach it through
`just link-data`. Never commit one (`just audit-leakage`).

## Waiting correctly

See `.claude/skills/oag-drive/member-rules.md`, "Waiting on an emulator capture".
