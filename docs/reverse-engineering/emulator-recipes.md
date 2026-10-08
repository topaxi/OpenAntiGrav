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
3. **Save states are a convenience with measured limits** (below), not the default.

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
| `scripts/emu-rebuild-states.sh <lane> pulse-grid\|hd-grid` | Rebuilds a state from the user's own disc into `data/saves/`. |

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

**RPCS3 save states, re-measured 2026-10-09 (the 2026-08-19 result was partly stale).**

- Creating: overlay `ps`, `up` x3 (`SaveState`), `cross`, `cross` (`Save Emulation State And Exit`).
  Needs `Suspend Emulation Savestate Mode: true` (`OAG_RPCS3_SUSPEND_STATE=1` in the generated config).
  Gives a 120 MB `.SAVESTAT.zst` in `<config>/rpcs3/savestates/<TITLE>/`.
- With the GDB stub enabled the write failed both times it was tried (one with the proxy connected, one
  listening only), and succeeded once with it off; see the next point for what that is worth.
- **The write is unreliable.** One file in 6 attempts (that one with the stub off; two with the stub
  on, three more with it off wrote nothing). Each failure logged "User selected savestate in home
  menu", then nothing, with the emulator left running and unusable (restart it). Whether the stub
  matters is **not established**; cause not found. `emu-rebuild-states.sh hd-grid` fails with a
  message instead of looping; run it again from a fresh `serve`.
- **New: it restores to the grid, not the campaign screen.** Loading the state taken on Talon's
  grid fly-over (`OAG_RPCS3_LOAD_STATE=<file>`, which passes `--savestate`) shows the fly-over with the
  `START RACE` prompt **7.7 s** after launch, against about 100-120 s for boot, walk and load cold.
  Same result in two loads; the earlier "Campaign / Event 01/08" landing was not reproduced.
- **But a loaded state is not drivable (seen once, confidence 50):** the virtual pad did nothing (a
  `cross` on `START RACE` was ignored for 40 s; the prompt stayed up and the fly-over looped), the
  loaded run's effective config showed `GDB Server: ""` although `--config` named a port (one load), and
  `TTY.log` shows no screen line, so `current_screen()` reads `?`. So a loaded RPCS3 state serves a **still or video of the fly-over
  and nothing that needs input, GDB or memory-by-screen waits**. For anything else, keep the
  emulator running and use the routes above.

Good RPCS3 points: `grid-flyover-talons` (`data/saves/hd-fury/grid-flyover-talons.SAVESTAT.zst`,
`scripts/emu-rebuild-states.sh <lane> hd-grid`): restores in 7.7 s, same place, GDB and pad dead.
Not worth taking: Cell Selection, the pause menu, mid-race (same limits, no input to continue).
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
