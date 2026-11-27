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
| `scripts/emu-env.sh <lane> <display> <gdb-port> [ppsspp-port]` | Builds a member's private RPCS3 tree (symlinked firmware, copied `dev_hdd0`, pad profile naming `OAG Pad <lane>`) and a throwaway script, not kept. About 2 GB, one command. |
| `scripts/emu_guard.py` | The watchdog every RPCS3 `Session` now carries. Stages (`boot`, `menu:<screen>`, `loading`, `countdown`, `capture`), a heartbeat from every GDB packet, `<log-dir>/status.json`, and on a stall: `STALLED at <stage> after <n>s`, the emulator's process group killed **by pid**, exit status 3. In `boot` and `menu:<screen>` only a screen change counts as progress (a press does not), so a walk parked on one screen stalls 90 s after its last screen change (boot: 100 s). Falsified-then-confirmed 2026-10-09: with presses ignored, `STALLED at menu:HUD after 91s`, emulator gone, exit 3. `OAG_EMU_STALL=<s>` overrides every stage limit, `OAG_EMU_TOTAL=<s>` adds a whole-run ceiling. |
| `scripts/emu-run.py [--silence 150] [--total 570] [--child-status <log-dir>/status.json] -- CMD...` | Foreground wrapper for any capture (RPCS3, PPSSPP, PCSX2). No output for `--silence` seconds, or a guarded child idle that long, is a stall: it prints the last line and the child's stage, kills the group by pid, exits 3. `--total` (default 570, under the tool ceiling) exits 124. |
| `scripts/emu-run.py --bg --status F -- CMD...` and `scripts/emu-run.py wait F [--max 540]` | The detached form for a long run, and the correct way to wait for it: prints each stage change, returns 0 done, 1 failed, 3 stalled, 4 still healthy at `--max` (call it again). |
| `scripts/rpcs3-drive.py serve` / `status` / `press` / `stop` | A **long-lived** RPCS3: boot once, hold the pad and a GDB proxy, stay up. Scripts built on `Session` attach with `OAG_RPCS3_ATTACH=1`. Source the lane's `env.sh` first: without a private `XDG_CACHE_HOME` and `XDG_CONFIG_HOME` it exits 78 and touches nothing (`just check-rpcs3-drive-env`; `OAG_RPCS3_OWN_HOME=1` only to drive your own profile). |
| `scripts/ppsspp-start.sh <lane> <display> <port> <image>` | A private muted PPSSPP SDL under its own Xvfb, stopped by recorded pid (`... <lane> stop`: only this lane's processes, SIGKILL if TERM fails, safe twice; `start` refuses a running lane; `just check-ppsspp-start`). |
| `scripts/psp-menu-pair.py walk\|pairs` | PPSSPP front-end screens (2026-10-27, `menu-pairs-pulse-pure`): `walk --plan "key:cross,st:Grid Selection,shot:a,back:Main Menu,pu:cross:title"` photographs any plan at 480x272 (Pulse waits on the game's own state name, Pure on a frame predicate); `pairs --title pulse\|pure` builds the menu pairs and gaps. `docs/tools/compare-page.md`, "Menu pairs: Wipeout Pulse and Wipeout Pure" |
| `scripts/psp-state.py` | PPSSPP save and load through the pause menu. **Resume the CPU before it runs** (a script that exits with the CPU stopped makes the next `load` click do nothing while still printing a clock; 2026-11-11, `pair-bump-damage`): compare the printed clock with the state's own, and give `--file` every time. |
| `scripts/emu-rebuild-states.sh <lane> pulse-grid\|hd-grid [name]` | Rebuilds a state from the user's own disc into `data/saves/`. |
| `scripts/emu-hd-timetrial-state.sh <lane> <circuit-index> <team-rights> <classic 0\|1> <name>` | Cold boot to a saved HD Time Trial state in about 3 min (2026-10-20, `compare-hd-pose`): reads the screen after each step (`Single Player`, then `Track Creation` is Track Select, `Team Selection`), backs out of Campaign when the first `right` is dropped, and fails with the screen it stopped on. 12 circuits and Harimau classic saved this way. A sixth argument is the number of `right`s on the SPEED CLASS row from Venom (3 is PHANTOM), screenshotted before the walk goes on. |
| `scripts/rpcs3-hd-hud-sway.py` | Holds cross and the stick, then scans guest memory for HD's HUD sway offset `-15 * ship+0x6a58` (2026-10-21, `hd-cockpit-lean`); found no matching node, see `ship-lean.md`, "The HUD sway". |
| `scripts/hd_scene_width.py <frame.png>...` / `--record <pair-dir>...` | The width an HD 1080p frame's scene was drawn at (1300 to 1920 in 20 px levels), read off the frame's resampling periodicity; `--record` writes `original.scene_width` into a pair's `pair.json` and nothing else. `live(read)` reads the level (`0x00c81b08`) and the throttle's enable byte in a pause; `rpcs3-drive.py capture` stores it as `ship.scene_width`, `rpcs3-hd-pair.py` writes both. Race frames only: a 2D frame with a regular pattern scores like a stretch. `docs/ghidra/functions/ps3-hdfury-eu/scene-width-throttle.md`, "Reading the width off a frame". |
| `scripts/rpcs3-hd-lock-sight.py --state N [--behind D] [--shot-on-change]` | HD's lock-on sight (2026-11-07, `hd-lock-sight`): finds the HUD object by its ten sight-widget pointers (`vtable 0x866210`, scan `0x30000000..0x38000000`, ~60 s), gives the player pickup state `N` (1 Missile, 10 LeachBeam), holds cross, optionally puts the craft `D` units behind the nearest rival at its heading and speed, and polls `hud+0xdc/0xd4/0xd8/0xe0/0xe4/0xec/0x604/0x634/0x6b8` (`0xd4/0xd8` the chase centre, `0xe0/0xe4` the target's projection; the scan also matches runs of image widgets elsewhere in the heap, so it keeps the candidate whose widgets are at their authored sizes; `hd-lock-pursuit`, 2026-11-07) and the widgets' size, rotation, bits and vertex colours without pausing; `--shot-on-change` photographs the paused frame at each lock-byte, latch or widget-bit change. The HUD object's address moves between races (re-scan after a restart), a slot written with another state first will not fire state 10. `docs/ghidra/functions/ps3-hdfury-eu/hud-sight.md`, "Measured live". |
| `scripts/rpcs3-hd-weapon.py --state N --shot-at T,T [--teleport-back D] [--hold S] [--fire-after S]` | A held weapon fired and photographed as paused frames (2026-11-07, `hd-weapons-2`): game seconds after the shot (the lap clock, not the wall clock), frame, pushbuffer camera and pose per frame into `NN.png`/`NN.json`, with the clock window the press fell in; attached, it fires on the race the `serve` is in. `scripts/hd-weapon-pair.py --capture DIR --weapon W --team T [--fire-clock S]` renders ours at the same pose, camera and fire second, `scripts/hd-weapon-region.py --box X,Y,W,H a.png b.png` measures a box (red, white, centroid, extent, mean RGB). One weapon per fresh grid; `weapons.md`, "The other weapons against the original". |
| `scripts/rpcs3-hd-whiteout.py --image ISO --out DIR [--fire-state N [--behind] \| --bomb \| --leach] --delay S --no-targets --want-mats MATS [--want-name TEXT]` | HD weapon draws dumped at a pause (2026-11-14, `hd-weapon-taps`): fires pickup state N (0 Rocket, 1 Missile, 7 Plasma, 9 Bomb, 10 LeachBeam) from an attached race (restore `hd-racebox-talons-venom-weapons-grid`, `press cross` for START RACE, wait out the countdown), and `--want-mats` retries the pause-and-dump until the frame holds a fragment block of one of the `.rcsmaterial` files in MATS. About 4 minutes a hit, 25 for a Plasma detonation. `scripts/rsx_fp_match.py DIR STEM MATS` then names each draw by the program it runs and prints unit 0's address word (gamma nibble in bits 20-23). `docs/rendering/hd-unlit-programs.md`, "The weapon units, read live". |
| `scripts/rpcs3-hd-camera-flags.py --image ISO [--hold l3,r3,l1,r1 \| --press B]` | HD's per-craft view bytes `ship + 0x5f42`/`+0x5f43` read live while a pad button is held (2026-11-25, `hd-contact-sparks`): R1 held reads `01 01` on the player's craft, every rival reads `00 00`. `particle-triggers.md`, "the camera-mode flags". |
| `scripts/rpcs3-hd-floor-spark-ride.py --image ISO [--shots]` | HD's kind 2 floor spark followed live (2026-11-25): pushes the body into the road, finds the new `LinkObj` nodes by the vtable word `0x008745a0`, and records each node's matrix, its particle instance (back pointer `+0x2c`) and the hull per frame; the instance's world translation follows the hull through a reset teleport. `collision-cues.md`, "does the floor spark ride its locator". |
| `scripts/rpcs3-hd-gantry-board.py --out DIR` | HD's start gantry time and lap-crossing counter at parked lap clocks, the Shield's countdown words (slot and HUD object) beside HUD frames, then a thrust across the line; attached `serve` on the weapons-on Talon's grid state, about 2 minutes (`gantry-clock.md`, `hud-readouts.md`). |
| `scripts/rpcs3-hd-shield.py steady\|flash\|plain\|draws --out DIR` | HD's raised Shield, an absorbed hit on it (no rival) and the no-shield control, each frame with the shield object read beside it (attached `serve`; `rpcs3-capture.md`, "An absorbed hit on a shielded craft"). `scripts/hd-shield-pair.py` makes the comparison pair. `draws` dumps one frame's RSX command ring with the shell up (and without, unless `--no-plain`), read with `rsx-draw-list.py DIR s00 --samplers` (`shield.md`, "The shell's draw state"). |
| `scripts/hd-hit-posed-pair.py --film DIR --team T` | Ours posed on the struck rival of a Missile film (craft 0 from `--pose-from`, the film's pushbuffer camera, `--force-shield` and `--force-hit`), scored over a box around the rival; not an adoptable pair (the rival's team is unrecorded and ours' hull is held at rest). `particle-triggers.md`, "a posed pair for the hit". `rpcs3-hd-missile-film.py --rival-shield S` writes the rival's shield before the shot. |
| `scripts/rpcs3-hd-absorb-film.py --image ISO --out DIR [--ages A,B] [--dump-at A,B] [--dump-attempts N] [--control]` | HD's player absorb as a film from the press (`weapon-absorb-pairs.md`, "Reproducing"). `--dump-at` (2026-11-09, `hd-absorb-tint-2`) also dumps the RSX draws at that age while the target is paused (`dNN-*.bin`, read with `rsx-draw-list.py DIR dNN --samplers`; the shell is the draws of fragment program `MAD /2` with `TEX R0.w` then `TEX H1.xyz`, absent in `--control`). The ring walk completed on 2 of 5 boots and stalled at 27 draws on the other 3 (cause not found); a stalled attempt is retried `--dump-attempts` times, each running the game 0.3 s on. |
| `scripts/rpcs3-hd-rocket-film.py --out DIR [--back D] [--mode fly\|hold]` | HD's Rocket craft hit as a kept film with its particle vertex buffers (2026-11-26, `blast-capture`): restore `hd-racebox-talons-venom-weapons-grid-rpcs3-20574` with `--no-gdb`, then run attached (`OAG_RPCS3_NO_GDB=1`; the script freezes the emulator with `SIGSTOP` and reads through the `serve` socket). About 80 s a hit, hit on the first try 3 of 3. **A stop can catch the ring with no particle draws** (11 of 24 stops after the shot had none), so check the reader's line for an age before relying on it, and film twice. `scripts/hd-particle-quads.py DIR` reads depth, size cap and screen fill off the dumps. A lane's first load in a fresh `XDG_CACHE_HOME` compiles PPU modules for about 3 minutes, past `emu-restore-state.sh`'s 120 s wait (it prints `restore failed` while `serve` carries on: wait for `serving` in `serve.log`). `docs/rendering/particle-fill.md`, "Against the original at a near blast". |
| `scripts/rpcs3-hd-missile-film.py [--hold-after-hit] --out DIR` | HD's Missile hit as a kept film (2026-11-09, `hd-missile-capture`): restore `hd-racebox-talons-venom-weapons-grid` (Weapons On), START RACE, hold the player 12 units behind the nearest rival, fire state 1, then step single game frames from the explosion pool's rise and keep, at each pool age (`pool[0]+0x170`), the frame, the pushbuffer camera, the blast centre and every craft's pose to `DIR/aNNNN.png/json` plus `steps.json`; `--hold-after-hit` parks the player at its hit pose so the eye stays near the blast. About 5 minutes a hit; the pool is found by a vtable scan once and cached. `scripts/hd-missile-film-pair.py --film DIR --out OURS` renders ours from the same camera relative to the blast, `scripts/hd-missile-film-table.py DIRS` prints luma and `e` per age. `docs/ghidra/functions/ps3-hdfury-eu/missile-explosion.md`, "Film". |
| `scripts/rpcs3-hd-zone-frame.py --image ISO --out DIR [--at S] [--after-go S]` | One paused Zone countdown frame of HD (2026-11-19, `zone-grade-sky`): a cold boot into Racebox Zone (four RIGHT presses on RACE TYPE, each repeated until the settled box changes), START RACE, a pause `--at` seconds into the countdown, then the frame's RSX draw state and fragment programs, every colour target a draw names (the scene before the resolve, the bloom ladder, the feedback buffer), `g_FunkLayer`, the settings block `0x00c489f0` and the vertex arrays of the scene's small draws; `--after-go` adds a tone timeline through the release. Run with `--bg --total 0` (about 6.5 minutes). `docs/ghidra/functions/ps3-hdfury-eu/zone-frame.md`. |
| `scripts/rpcs3-hd-hud-gate.py --image ISO --out DIR [--poke] [--any-mode --rights 0]` | HD's whole-HUD gate at the start (2026-11-19, `zone-hud-gate`): a cold boot into a Racebox race, then samples of the race manager (`*0x0095ae78`: phase, sub-state, timers) and the whole `g_Hud` (`*0x0098767c`) through `/proc/<pid>/mem` from the flyby to after GO, with a frame at every change. `--poke` writes `g_Hud+0x115` 1 then 0 mid-countdown and photographs each. The Racebox remembers its race type, so read `GetMode()` before trusting the mode. `docs/ghidra/functions/ps3-hdfury-eu/hud-start-gate.md`. |
| `scripts/rpcs3-hd-zone-palette.py --image ISO --out DIR [--drive-secs S]` | HD's Zone palette textures read live (2026-11-11, `hd-zone-stage`): cold boot into Racebox Zone (each RIGHT tap on the race type verified against the frame; refuses a non-`GetMode()==6` race), cross held, then 1 Hz samples of the seven palette/gradient handle arrays at `0x00c81330`, their FP16 texels and the stage state `H[0]` through `/proc/<pid>/mem`, to `DIR/palette.pkl`. `docs/ghidra/functions/ps3-hdfury-eu/zone-palette-textures.md`. |
| `scripts/rpcs3-hd-fe-adapted.py [reads] [seconds]` | HD's front-end adapted luma and light block read live (2026-11-12, `hd-ship-select-light-wire`): `FunkLayer + 0x25c` (the frame's luma), `+0x254` (it lerped: the adapted luma) and `+0x240..+0x248` (the ladder's mean colour; the last two since 2026-11-18, `hd-menu-cell`), the global block's tone constants, which are the screen's, and the front end's own block at `*(0x009948ec) + 0x60`, through `/proc/<pid>/mem`. Start the `serve` with `OAG_COMPARE_PLATFORM=hd-fury` (at RPCS3's own 1280x720 scale 150 the luma reads 44 % high), walk to Ship Select (`right cross right cross cross`), attach. `docs/ghidra/functions/ps3-hdfury-eu/front-end-model-lighting.md`, "The live read". |
| `scripts/rpcs3-hd-track-step.py capture`, `scripts/hd_track_hex.py` | HD's Track Select lattice step read from RAM (2026-11-25, `hd-front-end-ranked`): the `TrackHexSelection` widget is found by its vtable word `0x00868090` (one hit; `+0xe0` pulse counter, `+0xec` slide timer, `+0xb4` parity, `+0x11c` circuit, `+0x110` row), the slide timer is the frame counter while a step runs (`0.1 - (k + 1) / 60`) and the pulse counter (0.05 a frame) after it. `capture --aims 4,5,6,7,8,9` presses `right`, stops the game at that frame, reads the widget and grabs through the stopped window (hides the Fury backdrop). `rpcs3-hd-menu-pair.py` reads the same widget's pulse counter and the turntable angle in one stopped grab for `track-select` (`original.ring_phase`, ours `--menu-ring-seconds`). Walk to Track Creation (`cross cross` from RACEBOX), about 1 minute for 12 grabs. `docs/ghidra/functions/ps3-hdfury-eu/ship-select-ring.md`, "Track Select". |
| `scripts/rpcs3-hd-ship-step.py select\|timeline\|capture\|pairs`, `scripts/hd_ship_step.py` | `select --hull <team>` steps Ship Select until RAM names that hull (`docs/reverse-engineering/rpcs3-capture.md`, "Picking the hull on Ship Select"). HD's Ship Select step read a game frame at a time (2026-11-19, `hd-settle-step`): the `Team Selection` screen object is found by a heap scan (exactly one hit; `st+0x100` selection, `+0x194` model, `+0x1014/+0x1020` depth target and drawn depth, `+0x100c/+0x1010/+0x1018/+0x101c` slides, six input terms), a frame is a change of that tuple. `timeline --buttons right,left` logs every frame of a press unpaused; `capture --aims 3,6,10` stops the game at that many frames after the selection moved, reads the state in the stop and grabs through `grab_with_clock` (hides the Fury backdrop, records the adapted luma); `pairs --frames 6,10` makes compare-page pairs with ours at the same frame less `PICTURE_LAG_FRAMES`. Walk to Ship Select (`right cross right cross cross`), press `right` twelve times first (a team's first visit loads its hull and the craft stays absent past frame 10). About 2 minutes for 32 grabs. `docs/ghidra/functions/ps3-hdfury-eu/team-selection-model.md`, "The step against RAM". |
| `scripts/rpcs3-hd-vwipe.py scan\|watch\|stop\|analyze` | HD's `vwipe` Image widgets and their page-entry timeline (2026-11-20, `hd-vwipe-pages`): `scan` finds the 71 widgets by the `Image` vtable word `0x866210` and `+0x120 & 2` (every page's tree exists from boot, so addresses are stable) with name, page root and `+0x68/+0x6c/+0x74/+0x9c/+0xa0`; `watch --page P --setup B --buttons B --back B --runs N` polls that page's widgets unpaused across the press and logs every change plus the root's present flag (`+0xc4`); `analyze` turns the log into armed countdown, frames to the ramp start and ramp length; `stop --aims 0.6,0.3,-1` stops the game when the countdown reads at or below an aim (-1: first ramp frame), reads the widgets in the stop and again 0.3 s later; `discover` names the widgets that move. A press made just after a stop or a page switch is sometimes dropped (wait 1.5 to 5 s). `docs/ghidra/functions/ps3-hdfury-eu/image-widget.md`, "Page entry". |
| `scripts/rpcs3-hd-air-hold.py --run name=x,y,z,fx,fy,fz,ux,uy,uz,speed [--stick-y Y]` | Player-input physics captures (2026-10-20, `hd-air-player-input`): holds cross (and the stick), places the craft at a pose and speed (or `name=here,lift,speed`, a drop test), logs every new body position with the control record through `craft+0x84` and the Autopilot bit, which must read clear. Never arms the Autopilot. |
| `scripts/rpcs3-hd-escape-gate.py [--contacts]` | HD's hover escape gate under player input (2026-10-21, `deviations-to-original`): cross held, the stick swung, a breakpoint on `0x000ee628` (or the contact class test `0x000ee294`) logging each probe hit's `+0x28` tag, class `+0x74` and shape `+0xc`. Needs a `serve` started with `--interpreter`; about 5 s a stop at the escape gate. |
| `scripts/emu-restore-state.sh <lane> <state> [--restart] [--no-gdb]` | Restores an RPCS3 state into a long-lived `serve` with pad, GDB proxy and memory, in about 10 s. `scripts/emu-save-state.sh <lane> <name>` takes one from a running `serve`; `scripts/rpcs3-state-probe.py` checks GDB and memory after a restore. |
| `scripts/hd-ibl-poll.py <ibl> <refl> [s] [button...]`, `hd-ibl-sample.py`, `hd-ibl-catch.py`, `hd-draw-dump.py` | HD environment-probe reads on an attached RPCS3: poll the two live maps at ~100 Hz without pausing and print when they change; pause on a drive and write the player's position and velocity with both maps (the `data/reference/hd-craft-ibl-probe/` set); pause on the first change and dump the frame's draws; dump any frame's draws. See [hd-probes.md](../formats/hd-probes.md). |

**Copying another lane's RPCS3 tree is a trap (2026-10-20).** `xdg/config/rpcs3/input_configs/*/oag.yml` names the pad
device `OAG Pad <lane>`; a copy keeps the old lane's name, so the boot's `cross` presses (which auto-redirect screens
ignore anyway) look fine and every d-pad press is dropped (`press ... (dropped)` twelve times at `Main Menu`).
`sed` the lane name in that file, or build the tree with `emu-env.sh`. `place` attached to an emulator that is not in
a race walks the menus now (`session.in_race()` decides; it read an undefined name before).

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
  could be read. Not observed as a fault, and not re-tested when seats landed. Each member's pad is still named per lane
  by `scripts/emu-env.sh` (`OAG seat N pad rpcs3` under `OAG_PAD_SEAT=N`). SDL emulators (the wipeout_pc port, shadPS4) are
  isolated by seat instead of by name: `docs/tools/compare-page.md`, "Pad seats".
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
| `hd-racebox-talons-venom-weapons-grid` | **RPCS3 0.0.43-20574 (the binary replaced 2026-11-09) cannot load any October state** (`No savestate file found or savestate not compatible`); rebuild with the recipe below and a new name. Racebox Single Race, Venom, weapons ON, Feisar, 8 ships, fly-over. **Rebuilt 2026-11-09** (`hd-missile-capture`): `OAG_WEAPONS_ON=1 emu-hd-race-walk.sh <lane> 8 0 0`, then `emu-save-state.sh <lane> hd-racebox-talons-venom-weapons-grid` at `START RACE`; the file was gone. The one to use for any capture that fires a weapon. **Refused by 0.0.43-20574 on 2026-11-26** (`No savestate file found or savestate not compatible`): use the `-rpcs3-20574` file of the same name, with `--no-gdb` (with the stub on, in a lane's cold PPU cache, the stub did not answer within `serve`'s 25 s; warm with the stub on was not tried) | same; weapon pickups seen in the race; `cross` for START RACE, then `rpcs3-hd-missile-film.py` |
| `hd-timetrial-talons-venom-start` | Racebox Time Trial, Venom, Feisar, 4.5 s after `GO`, ship at 0 km/h | drive at once |
| `hd-race-talons-venom-pack` | **Rebuilt 2026-11-10** (`hd-talons-pack-align`; the October file is refused by RPCS3 0.0.43-20574 and was overwritten, and the 2026-11-08 rebuild is the same recipe): `emu-hd-race-walk.sh <lane> 8 0 0`, then `rpcs3-hd-race-hunt.py --place 3-6 --pin-scene-width` (lap 2, 6th of 8; the flag clears the scene-width throttle byte so the state draws 1920 px wide). Racebox Single Race, Venom, Talon's Junction, **the Fury Feisar** (`walk ... 8 0 0` leaves classic off: the player and the field read plain `Feisar`, `Icaras`, ... in RAM; the sidecar says `Feisar`, and `rpcs3-hd-pair.py --team` must match it), the Autopilot flying with its timer armed for good. Its HUD shows `Absorb`, but **Weapons are Off** (2026-11-09 a Missile pickup state written here was never fired by 3.5 game seconds of TRIANGLE): do not use it to fire anything. **`--pause` does not stop on the state's own tick**: three restores stopped at race clocks 11.2 s, 7.3 s and another, so every capture is a new moment | `emu-restore-state.sh <lane> <state> --pause`, then `rpcs3-hd-pair.py --field --keep-autopilot --accept-provenance autopilot_timer=...`; check the frame against RAM with `hd-pair-lag-scan.py` (`docs/tools/compare-page.md`, "A moving HD craft") |
| `hd-detonator-talons-feisar-grid` | Racebox Detonator (`GetMode()==14`), Talon's Junction, Feisar, at the START RACE prompt (2026-11-22, `det-pairs`; walk by hand: `right cross`, six `right` on RACE TYPE, `cross`, eight `right`, `cross`, `cross`, then `emu-save-state.sh`). Restore with `OAG_COMPARE_PLATFORM=hd-fury` exported | wait 30 s, then `rpcs3-hd-detonator-shots.py` (TRIANGLE appears to fire the cannon, seen once); `rpcs3-hd-detonator-endscreen.py` runs one on to `EndRace Results` |
| `hd-racebox-talons-venom-midrace-absorb` | Racebox race, 312 km/h, `Absorb` pickup held, lap 1 | drive at once |
| `hd-timetrial-talons-assegai-start` | Racebox Time Trial, Venom, `Assegai_c1` (the row Team Selection opens on), at rest just after `GO` | drive at once |
| `hd-timetrial-talons-assegai-classic-start` | the same with the classic `Assegai` hull (Team Selection `down` once) | drive at once |
| `hd-timetrial-talons-harimau-classic-start` | the same with the classic `Harimau` (nine `right`s from `Feisar`, then `down`) | drive at once |
| `hd-timetrial-vineta-k-harimau-classic-start` | Racebox Time Trial, Vineta K, Venom, classic `Harimau`, started and about 9 s after `GO`, at rest (`emu-hd-timetrial-state.sh`, 2026-10-20) | `--restart`, then `rpcs3-hd-pair.py` |
| `hd-timetrial-anulpha-pass-harimau-classic-start` | Racebox Time Trial, Anulpha Pass, Venom, classic `Harimau`, started and about 9 s after `GO`, at rest (`emu-hd-timetrial-state.sh`, 2026-10-20) | `--restart`, then `rpcs3-hd-pair.py` |
| `hd-timetrial-moa-therma-harimau-classic-start` | Racebox Time Trial, Moa Therma (`03_Track`), Venom, classic `Harimau`, started and about 9 s after `GO`, at rest (`emu-hd-timetrial-state.sh`, 2026-10-20) | `--restart`, then `rpcs3-hd-pair.py` |
| `hd-timetrial-chenghou-project-harimau-classic-start` | Racebox Time Trial, Chenghou Project, Venom, classic `Harimau`, started and about 9 s after `GO`, at rest (`emu-hd-timetrial-state.sh`, 2026-10-20) | `--restart`, then `rpcs3-hd-pair.py` |
| `hd-timetrial-metropia-harimau-classic-start` | Racebox Time Trial, Metropia (`02_Track`), Venom, classic `Harimau`, started and about 9 s after `GO`, at rest (`emu-hd-timetrial-state.sh`, 2026-10-20) | `--restart`, then `rpcs3-hd-pair.py` |
| `hd-timetrial-sebenco-climb-harimau-classic-start` | Racebox Time Trial, Sebenco Climb, Venom, classic `Harimau`, started and about 9 s after `GO`, at rest (`emu-hd-timetrial-state.sh`, 2026-10-20) | `--restart`, then `rpcs3-hd-pair.py` |
| `hd-timetrial-ubermall-harimau-classic-start` | Racebox Time Trial, Ubermall, Venom, classic `Harimau`, started and about 9 s after `GO`, at rest (`emu-hd-timetrial-state.sh`, 2026-10-20) | `--restart`, then `rpcs3-hd-pair.py` |
| `hd-timetrial-moa-therma-harimau-std-start`, `hd-timetrial-chenghou-harimau-std-start` | Racebox Time Trial, Moa Therma (`03_Track`) or Chenghou Project (`04_Chenghou_Project`), Venom, the **standard** Harimau (a fresh profile has the classic hull locked, so `emu-hd-timetrial-state.sh <lane> 2 9 0 <name>`, classic 0), saved at the `START RACE` prompt (2026-11-17, `water-hdcap`; the Moa walk stopped on Team Selection because `down` for the classic hull picked a locked one, read the shot). Restore **without** `--restart` (that Restart Race walk is for a state taken in a running race; from this one it opened Photo Mode) | `press cross` three times 4 s apart, wait 8 s, then `hd-sea-place-dump.py` |
| `hd-timetrial-ubermall-feisar-phantom-start` | Racebox Time Trial, Ubermall, **PHANTOM**, Feisar, started and about 9 s after `GO`, at rest (`emu-hd-timetrial-state.sh <lane> 6 0 0 <name> 3`, 2026-10-20) | attach, then `rpcs3-hd-air-hold.py` (placed, throttle held) |
| `hd-timetrial-sol-2-harimau-classic-start` | Racebox Time Trial, Sol 2, Venom, classic `Harimau`, started and about 9 s after `GO`, at rest (`emu-hd-timetrial-state.sh`, 2026-10-20) | `--restart`, then `rpcs3-hd-pair.py` |
| `hd-timetrial-the-amphiseum-harimau-classic-start` | Racebox Time Trial, The Amphiseum, Venom, classic `Harimau`, started and about 9 s after `GO`, at rest (`emu-hd-timetrial-state.sh`, 2026-10-20) | `--restart`, then `rpcs3-hd-pair.py` |
| `hd-timetrial-modesto-heights-harimau-classic-start` | Racebox Time Trial, Modesto Heights, Venom, classic `Harimau`, started and about 9 s after `GO`, at rest (`emu-hd-timetrial-state.sh`, 2026-10-20) | `--restart`, then `rpcs3-hd-pair.py` |
| `hd-timetrial-tech-de-ra-harimau-classic-start` | Racebox Time Trial, Tech de Ra, Venom, classic `Harimau`, started and about 9 s after `GO`, at rest (`emu-hd-timetrial-state.sh`, 2026-10-20) | `--restart`, then `rpcs3-hd-pair.py` |

**A new team needs a cold boot, not a restore (2026-10-12, `hd-hull-wash-2`).** Restoring
`hd-timetrial-talons-venom-start` with the stub off, quitting to the Main Menu and walking
Racebox, Time Trial, Talon's Junction to a new team reached `Launch Game` and loaded the track. The
game then called `abort()` from `0x0046007c` right after `Loading Screen Finished`, and the
emulator exited. That was seen once with nothing attached. An earlier try of the same walk
ended too, but an `rpcs3-drive.py shot` had started its own session on it, so it does not count.
Booting cold with the stub off and walking the same route loaded every time (3 of 3). The
route from `Main Menu`: `right`, `cross` (Racebox setup), `right` (`RACE TYPE` to Time Trial),
`cross`, `right` x8 (Talon's), `cross`, `right` x7 (Assegai) or x9 (Harimau), `down` for
the classic hull, `cross`, then `cross` at `START RACE`. Then take the state with
`scripts/emu-save-state.sh`. A walk that needs a breakpoint then restores that state with
`--interpreter` and the stub on, and attaches after the load. This also avoids the
`Thread [GDB Server] is too sleepy` hang seen when the stub was attached through a race load.
`rpcs3-drive.py shot` starts a session of its own, so never run it against a running `serve`:
it ended one mid-load. A `right` straight after the cold boot's `Main Menu` is
sometimes dropped (1 in 3, the walk went into Campaign). Read the screen after that first press.

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

**A fresh instance restores a state too (2026-10-20, `compare-pairs-pulse`; two separate boots, each
loaded `time-trial-talons-venom-grid.ppst` from the title screen in 2.4 s and read the pose
`(5.64, -50.08, -196.67)`).** An earlier run reported "loaded" and stayed on the title screen. Two
causes, both in `psp-state.py`: (1) the clicks were made in the window's own coordinates but
`xdotool` moves the pointer in **root** coordinates; with no window manager on the 1280x800 Xvfb
`ppsspp-start.sh` declares, the 960x544 window sits at (160, 128), so every click landed in the black
margin (the script now reads the window's origin); (2) `pose`/`find_craft` stops the CPU at a
breakpoint and left it stopped, so `paused()` read true forever and the next `Escape` toggled the menu
the wrong way (`pose` now resumes, and `load` waits for the game to run before it touches `Escape`).

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

## Member setup facts

What a drive member needs before its first capture (`.claude/skills/oag-drive/member-rules.md` points here):

- PPSSPP returns zeroed framebuffers unless its config has `[Graphics] SoftwareRenderer = True` (`scripts/ppsspp-start.sh` sets it; a hand-written config must).
- RPCS3 comparison frames need `Write Color Buffers: true` in the private `config.yml` **and** in `custom_configs/config_BCES00664.yml` (off, the original reads about 1.74x too bright).
- `rpcs3-drive.py serve` needs a short `XDG_CACHE_HOME` (for example `/tmp/oag-<lane>-cache`; a longer one fails with `AF_UNIX path too long`) and its `uv` run with `UV_CACHE_DIR=~/.cache/uv UV_OFFLINE=1`.
