#!/usr/bin/env python3
"""One original-vs-ours pair of Wipeout Pulse (PSP) from a race the emulator is in, environment-checked.

    OAG_PPSSPP_PORT=47897 source <lane>/pp/env.sh       # DISPLAY, PP_HOME (scripts/ppsspp-start.sh)
    DISPLAY=127.0.0.1:97 OAG_SHOT_DISPLAY=127.0.0.1:97 HOME=<instance HOME> XDG_CONFIG_HOME=<scratch>/profile/cfg \\
    uv run --with websocket-client python3 scripts/psp-pulse-pair.py \\
        --port 47897 --out data/scratch/<lane>/moa --track 03_Track --state data/saves/pulse-psp/<point>.ppst \\
        [--reversed] [--team Feisar]

The environment is `scripts/compare-environment.toml`, platform `pulse-psp`: the PPSSPP instance must have
been started with its `ppsspp-ini` text (`PPSSPP_EXTRA_INI`, see ppsspp-start.sh); the 960x544 window the pause
menu needs to load a state is resized to the native 480x272 afterwards, so the frame is the console's own
size with nothing scaled. Ours is rendered at 480x272 with the profile's flags and settings file, the original's
camera and its speed-widened fov, after the countdown and the original's race clock, with `--pose-hold` putting
the craft back on the original's pose for the frame (a resting craft turns in both, so running the clock from
an already-turned pose turned ours twice). A second, `--trace-out` run checks that pose landed: an import check
of the basis, convention and fov, not of the simulation (`scripts/compare-resting-yaw.py` measures that). A pair that differs from the profile
(image size, settings read back, layout file, mode, class, pose, fov) is refused with the mismatch named, and
`pair.json` records both sides.

The load stops at the state's own first frame (`psp-state.py load --stop-at`, bit-identical run to run) and `--target-clock T`
runs the original on to a stored pair's moment; a race shot reads the pose row's drawn lean, roll phase and flare boost timer
(`psp-trace.py --flare`) and passes `--pose-lean` and `--pose-boost` to ours, checking ours' row against them.
"""

import argparse
import math
import os
import subprocess
import sys
import time
from pathlib import Path

#: The race clock in seconds (f32) in PPSSPP's Pulse USA RAM: 40.07 at a state whose HUD read 0:40.0, and it
#: counts 1:1 with ours (read 2026-10-20 by scanning RAM for a float that advances with the emulation).
CLOCK_ADDRESS = 0x08B344B0
#: Our race's tick at which its race clock leaves 0, `oag_race::COUNTDOWN_TICKS` (the countdown's length, measured
#: on the original: the lap clock is 0.0 until the tick the thrust gate opens). It was 276 here until 2026-10-21,
#: fitted to `0:00.4` at tick 300 and `0:40.4` at tick 2700, which truncated tenths cannot tell from 272; that put
#: ours' HUD clock 4 ticks (0.067 s) ahead of the original's in every pair (ours showed `0.13.3` against `0.13.2`).

#: `g_weapons_enabled` (byte, 0 = off), docs/ghidra/functions/psp-pulse-usa/shield.md.
WEAPONS_ENABLED = 0x08B30FA8
#: `g_camera_fov_degrees` (f32), the fov the frame is drawn with: the view's authored fov (`<ExternalCameraClose>` 60,
#: `<InternalCamera>` 65 on these ships) plus `0.075 * dot(fwd, vel)`, docs/ghidra/functions/psp-pulse-usa/camera.md.
CAMERA_FOV_ADDRESS = 0x08B34310
#: `<InternalCamera fov>` on every Pulse ship read so far (our log: `<InternalCamera>: fov 65`); a check below
#: compares it with the fov in RAM, so a ship that authors another value is refused rather than mis-posed.
INTERNAL_FOV = 65.0
#: `|velocity| * 3.6`, the HUD's speed readout (PlayerStatus_Update, docs/ghidra/functions/psp-pulse-usa/race-progress.md).
SPEED_TO_KMH = 3.6

#: `g_game_mode` (u32): 6 is Zone (`docs/ghidra/functions/psp-pulse-usa/zone-start.md`).
GAME_MODE = 0x08B31048
ZONE_MODE = 6
#: `g_race_manager` (`race-progress.md`): its word is the race's mode object, whose Zone fields are `+0x1a10` the zone number
#: (u16), `+0x1a12` the clean-zone count (u16), `+0x1a1c` the score (s32) and `+0x1a24` the seconds into the zone (f32)
#: (`zone-mode.md`, "The mode object"; the pointer hop and the three reads were checked live on 2026-10-21 against `craft+0x28c`).
RACE_MANAGER = 0x08B317B4
ZONE_COUNTER_OFFSET = 0x28C

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
sys.path.insert(0, str(HERE))
import compare_env as ce  # noqa: E402
import compare_provenance as cp  # noqa: E402

#: Our race's tick at which the race clock leaves 0: `oag_race::COUNTDOWN_TICKS`, read from ours by `compare_env.go_tick` (one
#: length for every title today; it was a hard-coded 276 here until 2026-10-21, 4 ticks late).
GO_TICK = ce.go_tick()

#: How many stops of the player's `Ship_UpdateCraft` the presented frame lags the stop it is taken at: the
#: screenshot at stop `i` shows the game state read at stop `i - 1`. Measured 2026-10-21 by `scripts/psp-presented-lag.py`
#: on a moving craft (the race state's Autopilot, 163 units/s): ours rendered at each of the six rows before a shot, the
#: smallest mean grey difference named row `i - 1` on all seven shots (21.3 against 26.6 and up for every other row).
#: So the pose, camera, field and HUD are read at stop 0 and the screenshot taken at stop `PRESENTED_LAG`.
PRESENTED_LAG = 1
#: `Ship_UpdateCraft`: the load stops at its first hit, the state's own first frame (bit-identical run to run), where the
#: old load-then-freeze landed 0.6 s of race clock late and differently each time (`psp-state.py do_load_stopped`).
SHIP_UPDATE_CRAFT = 0x08849618
#: The flare node's boost length (`+0xb4`, 0.8 s, `exhaust.md`): its `+0xb8` timer counts down from it, `--pose-boost` is the
#: elapsed time, so the age of a boost with `t` seconds left is `BOOST_SECONDS - t`.
BOOST_SECONDS = 0.8


def run(cmd, **kw):
    print("+", " ".join(str(c) for c in cmd), file=sys.stderr)
    return subprocess.run([str(c) for c in cmd], check=True, **kw)


def xdo(*args, capture=False):
    return subprocess.run(["xdotool", *args], capture_output=capture, text=True, check=True)


def emulator_window():
    """`(id, x, y, w, h)` of PPSSPP's own window: the largest visible one that is not the root."""
    best = None
    for wid in xdo("search", "--onlyvisible", "--name", "", capture=True).stdout.split():
        geo = xdo("getwindowgeometry", "--shell", wid, capture=True).stdout
        v = dict(line.split("=") for line in geo.split() if "=" in line)
        x, y, w, h = (int(v[k]) for k in ("X", "Y", "WIDTH", "HEIGHT"))
        if (w, h) in ((960, 544), (480, 272)):
            best = (wid, x, y, w, h)
    if best is None:
        sys.exit("no PPSSPP window of 960x544 or 480x272 on DISPLAY")
    return best


def set_window(w, h):
    wid, x, y, cw, ch = emulator_window()
    if (cw, ch) != (w, h):
        xdo("windowsize", wid, str(w), str(h))
        time.sleep(2.0)
    return emulator_window()


def freeze(port):
    """Stop the CPU straight after the load, so the race clock stays at the state's own value while the
    "State loaded" toast fades (the toast runs on wall time, not emulated time)."""
    from ppsspp_debugger import Debugger
    Debugger(port).call("cpu.stepping")


def resume(port):
    """A previous `psp-trace.py` leaves the CPU stopped at its breakpoint; the pause-menu logic reads that as
    "the menu is open", so run it first."""
    from ppsspp_debugger import Debugger
    dbg = Debugger(port)
    if dbg.is_stepping():
        dbg.call("cpu.resume")
        time.sleep(1.0)


#: The Autopilot pickup's countdown (`record+0x148`) is armed at `1.0e9` by the rail and the hunt so a state keeps
#: flying; the HUD prints that timer, so a frame taken in such a state shows `1000000000.00`. Anything above this is an
#: armed-forever pickup, not a player's state.
AUTOPILOT_TIMER_SANE_S = 600.0
#: `<Weapon type="Autopilot"><Stats time>`: the timer the pickup arms with (4.9666 read two dispatches after arming,
#: `scripts/psp-autopilot-rail.py` logs it as `authored_timer`), which is what a player who just took the pickup sees.
AUTOPILOT_AUTHORED_S = 5.0


def lean_and_boost_flags(pair, theirs_row):
    """`--pose-lean` and `--pose-boost` for the frame, from the pose row's own columns (`psp-trace.py --flare`): the drawn
    lean (`steer_lean`) and barrel-roll phase the original's per-tick update had written, and the exhaust flare's boost
    timer. The boost age is `BOOST_SECONDS - timer`; a timer of 0 carries no boost."""
    lean, roll, timer = theirs_row.get("steer_lean"), theirs_row.get("roll_phase"), theirs_row.get("boost_timer")
    pair.check("the trace row carries the drawn lean, the roll phase and the flare's boost timer",
               None not in (lean, roll, timer), "steer_lean %s, roll_phase %s, boost_timer %s" % (lean, roll, timer))
    pair.rec["original"]["pose_row_display"] = {"steer_lean": lean, "roll_phase": roll, "boost_timer": timer,
                                                "intensity": theirs_row.get("intensity")}
    flags = ["--pose-lean"]
    if timer is not None and timer > 0.0:
        flags += ["--pose-boost", "%.6f" % (BOOST_SECONDS - timer)]
    return flags


def boost_phase(dbg):
    """The player's exhaust flare's boost timer over its length (`*(craft+0x78)+0xb8 / +0xb4`), what `Hud_Update`
    (0x0881bf50) turns into the HUD's copy count and the speed bar's glow stamp: 0 with no boost running."""
    craft = dbg.read_u32(dbg.read_u32(RACE_MANAGER) + 0x2C0)  # `g_race_manager + 0x2c0`, the player's craft
    flare = dbg.read_u32(craft + 0x78)
    length, timer = dbg.read_f32(flare + 0xB4), dbg.read_f32(flare + 0xB8)
    return timer / length if length > 0.0 and timer > 0.0 else 0.0


def restore_autopilot_timer(port, pair):
    """Put the authored timer back over one armed forever, so the HUD prints what a player's own pickup prints
    (`5.00` counting down) rather than `1000000000.00`. The CPU does not run, so the moment (clock, pose, field,
    camera) is exactly the state's; a run to let the pickup expire was tried and left the moment (the craft slid
    off the line and the pack was gone after 0.8 s, and the clock differed run to run). The pair is refused when a
    timer above the sane bound is still there."""
    import struct
    from ppsspp_debugger import Debugger
    rail = race_field_module().rail
    dbg = Debugger(port)
    p = rail.player(dbg)
    if p is None:
        pair.check("the player's record was found to read the Autopilot timer", False, "rail.player() found no player")
        return
    addr = p["record"] + rail.AUTO_TIMER
    timer = rail.f32s(dbg, addr, 1)[0]
    pair.rec["original"]["autopilot_timer_in_state_s"] = timer
    if timer > AUTOPILOT_TIMER_SANE_S:
        dbg.write(addr, struct.pack("<f", AUTOPILOT_AUTHORED_S))
        pair.caveat("the state was saved with the player's Autopilot armed for good (timer %.3g s, which the HUD would "
                    "print); the timer was put back to the pickup's authored %.1f s after the load, the CPU not run, so "
                    "the HUD shows a pickup's own countdown and every other value is the state's" % (timer, AUTOPILOT_AUTHORED_S))
    after = rail.f32s(dbg, addr, 1)[0]
    pair.rec["original"]["autopilot_timer_at_stop_s"] = after
    pair.check("no armed-forever Autopilot timer at the stop (the HUD prints it)", after <= AUTOPILOT_TIMER_SANE_S,
               "record+0x148 reads %.3g s" % after)


def run_to_clock(port, target):
    """Run the stopped original on to the first player tick whose race clock reaches `target` (a stored pair's own
    moment), in ONE resume: a breakpoint on `Ship_UpdateCraft` conditional on the player's craft and on the clock's float
    bits (`[CLOCK_ADDRESS] >= bits(target)`, valid for a positive float). Stepping tick by tick instead (`each_hit`, a stop
    and a resume per hit) landed the same state on clocks 11 ulp apart in 1 of 3 runs; one resume gave the same bits in 4
    of 4. Returns the clock it stopped at."""
    import struct
    from ppsspp_debugger import Debugger
    dbg = Debugger(port)
    craft = race_field_module().Field(dbg).player["craft"]
    clock = dbg.read_f32(CLOCK_ADDRESS)
    if clock >= target - 0.0005:
        return clock
    bits = struct.unpack("<I", struct.pack("<f", target - 0.0005))[0]
    dbg.brk()
    dbg.add_breakpoint(SHIP_UPDATE_CRAFT, "a0 == 0x%08x && [0x%08x] >= 0x%08x" % (craft, CLOCK_ADDRESS, bits))
    dbg.call("cpu.status")
    dbg.pending = []
    dbg.call("cpu.resume")
    try:
        dbg.wait_for_break(SHIP_UPDATE_CRAFT, timeout=120)
    finally:
        if not dbg.is_stepping():
            dbg.brk()
        dbg.remove_breakpoint(SHIP_UPDATE_CRAFT)
    return dbg.read_f32(CLOCK_ADDRESS)


def race_field_module():
    """`psp-race-field.py` (a hyphenated name cannot be imported): the original's whole field out of RAM."""
    import importlib.util
    spec = importlib.util.spec_from_file_location("psp_race_field", HERE / "psp-race-field.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


#: `Mesh_UpdateTextureTransforms` (a0 = mesh): the world meshes' texture animation updater, texture-animation.md.
UPDATE_TEXTURE_TRANSFORMS = 0x0890E160


def world_clock_offset(port, hits=40):
    """The world meshes' animation clock minus the HUD's race clock, in seconds, read live.

    Every world mesh is fed one clock that stands at the HUD clock plus a constant per state (16.2 s on Fort Gale's
    state, 13.6 to 24.3 s over the 24: `scripts/psp-world-anim-clock.py`; inferred to be the time since the circuit
    loaded), not at the HUD clock. The most common non-zero `mesh+0x40` over `hits` updater calls is that clock (other meshes hold 0 or a
    clock of their own), read with the HUD clock at the same stop. Runs the CPU for the hits, so call it after
    everything else has been captured from the state.
    """
    from collections import Counter
    from ppsspp_debugger import Debugger
    dbg = Debugger(port)
    seen = Counter()
    for _ in dbg.each_hit(UPDATE_TEXTURE_TRANSFORMS, hits, timeout=20.0):
        regs = dbg.call("cpu.getAllRegs")
        a0 = dict(zip(*[next(c for c in regs["categories"] if c["name"] == "GPR")[k] for k in ("registerNames", "uintValues")]))["a0"]
        t = dbg.read_f32(a0 + 0x40)
        if t > 0.0:
            seen[round(t - dbg.read_f32(CLOCK_ADDRESS), 3)] += 1
    if not seen:
        return None
    (offset, n), = seen.most_common(1)
    return offset, n, sum(seen.values())


def read_zone_ram(port):
    """Zone's own state out of the stopped original: the mode, the zone number, the clean-zone count, the score and the
    seconds into the zone, from the race's mode object."""
    import struct
    from ppsspp_debugger import Debugger
    dbg = Debugger(port)
    obj = dbg.read_u32(RACE_MANAGER)
    blob = dbg.read(obj + 0x1A10, 0x1C)
    zone, clean = struct.unpack_from("<2H", blob, 0)
    return {"game_mode": dbg.read_u32(GAME_MODE), "mode_object": obj, "zone": zone, "clean_zones": clean,
            "score": struct.unpack_from("<i", blob, 12)[0], "zone_seconds": struct.unpack_from("<f", blob, 20)[0]}


def zone_pair(args, pair, plat, out, image, facts, flags, csv, theirs_row, theirs, fov, view_flags, profile_dir,
              clock, zone_ram, world):
    """Ours for a Zone frame: the player alone (Zone has no opponents), posed on the original's body and camera, rendered at
    tick 0 with the original's zone number, score and zone clock written into the race's own state (`--hud-state`'s zone
    fields) so the readout, the colour grade and the stage sphere read the zone the original was on."""
    import json
    from ppsspp_debugger import Debugger
    after = read_zone_ram(args.port)
    pair.rec["original"]["zone"] = {"at_load_stop": zone_ram, "after_the_trace_ticks": after,
                                    "source": "race-manager mode object +0x1a10/+0x1a12/+0x1a1c/+0x1a24, g_game_mode"}
    pair.check("the original is in Zone (g_game_mode == 6)", zone_ram["game_mode"] == ZONE_MODE,
               "g_game_mode reads %d" % zone_ram["game_mode"])
    pair.check("the state's zone number is unchanged across the capture's four ticks",
               after["zone"] == zone_ram["zone"], "%d at the load stop, %d after" % (zone_ram["zone"], after["zone"]))
    layout = args.layout if args.layout != "auto" else "forward"
    declared = facts.get("layout")
    pair.check("layout asserted matches the state's declared layout", declared in (None, layout),
               "asserted %s, sidecar declares %s" % (layout, declared))
    shield = theirs_row.get("shield")
    shield_percent = 100.0 if shield is not None and shield >= args.shield_max - 0.5 else None
    pair.rec["original"]["hud"] = {"zone": zone_ram["zone"], "score": zone_ram["score"], "zone_seconds": zone_ram["zone_seconds"],
                                   "shield": shield, "shield_max_assumed": args.shield_max, "race_clock_f32_s": clock}
    pair.rec["ours"].update({"mode": facts.get("mode", "zone"), "class": facts.get("class", "zone"),
                             "team": args.team or "default", "layout": layout})
    pair.rec["original"]["layout"] = layout
    entry = "Data\\Environments\\%s\\track%s.vex" % (args.track, "_reversed" if layout == "reversed" else "")
    # The top-left counter of a Zone frame is the lap: read the player's own (`entity+0xacc - 1`, as the race pair does), and
    # hand the player alone to `--field-from` so `--field-warmup` gives the exhaust the velocity and thrust history it has at speed.
    rf = race_field_module()
    # (read at the pose's own stop by `psp-trace.py --field-out`, not after the trace: four ticks later the craft is 20 units on)
    player = next(r for r in json.loads((out / "field.json").read_text())["rows"] if r["player"])
    lap_number = max(1, player["lap"] - 1)
    pair.rec["original"]["lap_number"] = lap_number
    ce.write_field_csv(out / "field.csv", [player], with_player=True)
    hud = "1,%d,%.4f,%.4f,%.4f,%s,0,%d,%d,%.4f" % (
        lap_number, clock, clock, theirs_row["speed"] * SPEED_TO_KMH, "%.2f" % shield_percent if shield_percent is not None else "100",
        zone_ram["zone"], zone_ram["score"], zone_ram["zone_seconds"])
    if shield_percent is None:
        pair.caveat("the original's shield read %.1f against the assumed pool of %.1f: ours shows a full bar" % (shield, args.shield_max))
    base = [args.oag_game, image, "--race", "--track", entry, "--pose-from", csv, "--pose-tick", 0, "--pose-lean",
            "--field-from", out / "field.csv", "--field-warmup", "--hud-state", hud] + flags + ["--camera-fov", "%.4f" % fov]
    pair.rec["ours"]["ticks"] = 0
    pair.rec["ours"]["flags"] = [str(c) for c in base[2:]] + ["--ticks", 0]
    pair.caveat("Zone state read from the original's RAM and imposed on ours: zone %d, score %d, %.2f s into the zone; ours runs no "
                "ticks, so its exhaust and trail are an exhaust-only warm-up on the player alone (`--field-warmup` with a one-row field.csv: the "
                "captured velocity and full thrust, the path chosen); the lap counter is the original's lap (%d) and the clocks are imposed from the "
                "original's RAM but Zone_HUD authors none of them; the world meshes' clock is "
                "the original's own mesh clock, imposed" % (zone_ram["zone"], zone_ram["score"], zone_ram["zone_seconds"], lap_number))
    ours_csv = out / "ours-trace.csv"
    run(base + ["--ticks", 1, "--trace-out", ours_csv], stdout=subprocess.DEVNULL)
    run(base + ["--ticks", 0] + view_flags + ["--screenshot", out / "ours.png"], stdout=subprocess.DEVNULL)
    got, bad = ce.read_back(plat, profile_dir)
    pair.rec["ours"]["settings_readback"] = got
    for b in bad:
        pair.check("ours settings", False, b)
    pair.check("ours settings read back", not bad, "%d keys equal the profile" % (len(got) - len(bad)))
    ours_row = ce.csv_row(ours_csv, 0)
    pair.check_pose(theirs, ce.pose_of(ours_row), fov, fov)
    pair.rec["ours"]["shield"] = ours_row.get("shield")
    pair.check_size("ours", out / "ours.png")
    run(["magick", "montage", out / "theirs.png", out / "ours.png", "-tile", "2x1", "-geometry", "+2+2", out / "side.png"])
    if args.meta:
        pair.rec["meta"] = json.loads(args.meta)
    pair.finish()
    print("wrote", out / "side.png")


def display_checks(pair, theirs_row, ours_row):
    """Ours' own `--trace-out` row against the original's at the pose's tick: the drawn lean and roll phase equal, and the
    exhaust's boost timer within one tick of the original's (the arm and its `aged` advances are whole ticks)."""
    for col, tol in (("steer_lean", 1e-5), ("roll_phase", 1e-5), ("boost_timer", 1.0 / 60.0)):
        t, o = theirs_row.get(col), ours_row.get(col)
        pair.rec["ours"].setdefault("display_row", {})[col] = o
        pair.check("ours' %s equals the original's (tolerance %g)" % (col, tol),
                   t is not None and o is not None and abs(t - o) <= tol, "original %s, ours %s" % (t, o))


def camera_craft_distance(row):
    return ce.math.dist([row["cam_pos_" + a] for a in "xyz"], [row["pos_" + a] for a in "xyz"])


def camera_view_of(row):
    """`internal` when the original's camera sits inside the craft (3.0 units from it at its own height), `close`
    at the close chase view's 11.64 units (3 up, 11.25 back at the 0.75 scale), and `None` for any other distance
    (the far chase view, a photo or destroy camera, a state mid-transition): the pair is refused rather than drawn
    in a view it was not taken in."""
    d = camera_craft_distance(row)
    if abs(d - 3.0) < 0.5:
        return "internal"
    if abs(d - 11.64) < 0.5:
        return "close"
    return None


def detect_layout(oag, image, track, theirs_row, flags, tol):
    """Which of `track.vex` / `track_reversed.vex` the original loaded: our own start on each layout against
    the original's grid pose (position and heading). Returns (layout, report) or exits naming the circuit."""
    report = {}
    for layout, name in (("forward", "track.vex"), ("reversed", "track_reversed.vex")):
        csv = Path(tol["scratch"]) / ("layout-%s.csv" % layout)
        entry = "Data\\Environments\\%s\\%s" % (track, name)
        run([oag, image, "--race", "--track", entry, "--ticks", 1, "--trace-out", csv] + flags,
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        ours = ce.pose_of(ce.csv_row(csv, 0))
        theirs = ce.pose_of(theirs_row)
        d = ce.math.dist(ours["pos"], theirs["pos"])
        a = ce.vec_angle(ours["fwd"], theirs["fwd"])
        report[layout] = {"start_distance": d, "heading_deg": a}
    best = min(report, key=lambda k: report[k]["start_distance"] + report[k]["heading_deg"])
    return best, report


def field_pair(args, pair, plat, out, image, facts, flags, csv, theirs_row, theirs, fov, layout, profile_dir, clock, view_flags):
    """Ours for a race shot: every craft of the original's field, the player's pose and the camera from the
    one stop (`field.json`, `trace.csv`), rendered at that tick with no ticks run."""
    import json
    rows = json.loads((out / "field.json").read_text())["rows"]
    player = next(r for r in rows if r["player"])
    teams = ce.write_field_csv(out / "field.csv", rows, with_player=True)
    pair.rec["original"]["field"] = [{k: r[k] for k in ("slot", "team", "place", "lap", "lap_time", "pos")} for r in rows]
    # The frame's HUD text is one tick OLDER than its 3D: `Hud_Update` reads `PLAYER_HUD` before this tick's
    # `PlayerStatus_Update`, so the frame shows the block as the previous tick left it. The CPU is stopped one tick
    # before the pose's stop, where the block holds exactly that value (3 of 3 live runs: the frame read 390, 383 and
    # 377 km/h against the block's 390.66, 383.24 and 377.20; body |velocity| * 3.6 at the pose's stop read 392.08,
    # 384.52 and 378.15, which was the 572 against 587 of the first race pair).
    hud_speed = pair.rec["original"].get("hud_at_freeze", {}).get("speed_kmh")
    lap_number = player["lap"] - 1  # PlayerStatus_Update: the HUD's lap is `entity+0xacc - 1`
    pair.rec["original"]["hud"] = {"place": player["place"], "of": len(rows), "lap_number": lap_number,
                                   "lap_time_s": player["lap_time"], "race_clock_s": clock,
                                   "shield": player["shield"], "shield_max": player["shield_max"],
                                   "best_lap_s": player["best_lap_cs"] / 100,
                                   "speed_kmh": hud_speed,
                                   "speed_kmh_at_pose_stop": player.get("hud_kmh"),
                                   "body_speed_kmh": theirs_row["speed"] * SPEED_TO_KMH}
    pair.check("the HUD's own speed readout (PLAYER_HUD+0) was found in the registry", hud_speed is not None,
               "the block whose lap time (+0x24) is the player's; at the pose's stop it reads %s, body |velocity| * 3.6 %.2f"
               % (player.get("hud_kmh"), theirs_row["speed"] * SPEED_TO_KMH))
    if hud_speed is None:
        pair.finish()
    pair.check_equal("original field size", len(rows), 8)
    ce.pack_check(pair, player["place"], [theirs_row["cam_pos_" + a] for a in "xyz"],
                  [-theirs_row["cam_right_z"], -theirs_row["cam_up_z"], -theirs_row["cam_fwd_z"]], player["pos"],
                  [r["pos"] for r in rows if not r["player"]], near=args.pack_near)
    pair.check("every craft has a team name", all(t and not t.startswith("?") for t in teams), ", ".join(teams))
    pair.rec["ours"].update({"mode": facts.get("mode", "single_race"), "class": facts.get("class", "venom"),
                             "team": args.team or "default", "layout": layout, "opponent_teams": teams,
                             "weapons": "off (settings [race] weapons)"})
    pair.rec["original"]["layout"] = layout
    entry = "Data\\Environments\\%s\\track%s.vex" % (args.track, "_reversed" if layout == "reversed" else "")
    base = [args.oag_game, image, "--race", "--track", entry, "--pose-from", csv, "--pose-tick", 0,
            "--field-from", out / "field.csv", "--field-warmup",
            "--hud-state", "%d,%d,%.4f,%.4f,%.4f,%.4f,%.4f" % (
                player["place"], lap_number, player["lap_time"], clock,
                hud_speed, 100.0 * player["shield"] / player["shield_max"],
                player["best_lap_cs"] / 100),
            "--hud-boost-phase", "%.6f" % pair.rec["original"]["hud_at_freeze"]["boost_phase"]] + lean_and_boost_flags(
                pair, theirs_row) + flags + ["--camera-fov", "%.4f" % fov]
    pair.rec["ours"]["ticks"] = 0
    pair.rec["ours"]["flags"] = [str(c) for c in base[2:]] + ["--ticks", 0]
    pair.caveat("every craft's trail and plume in ours are an exhaust-only warm-up (`--field-warmup`): each craft "
                "flown on a straight path to its captured pose at its captured velocity (RAM, body+0x140), full "
                "thrust for all (the player's reads 100, an opponent's is not read), the path and the heading chosen, "
                "not measured; the HUD's place, lap and clocks are imposed from the original's RAM (%d/%d, lap %d, "
                "%.1f s), the speed and shield bars read the original's own values; the pickup row (the Autopilot icon) "
                "is not drawn" %
                (player["place"], len(rows), lap_number, player["lap_time"]))
    ours_csv = out / "ours-trace.csv"
    # `--ticks 0` writes the one row the frame is drawn from (after the warm-up, the boost and the lean), so the display columns checked below are the picture's
    run(base + ["--ticks", 0, "--trace-out", ours_csv], stdout=subprocess.DEVNULL)
    run(base + ["--ticks", 0] + view_flags + ["--screenshot", out / "ours.png"], stdout=subprocess.DEVNULL)
    got, bad = ce.read_back(plat, profile_dir)
    pair.rec["ours"]["settings_readback"] = got
    for b in bad:
        pair.check("ours settings", False, b)
    pair.check("ours settings read back", not bad, "%d keys equal the profile" % (len(got) - len(bad)))
    ours_row = ce.csv_row(ours_csv, 0)
    pair.check_pose(theirs, ce.pose_of(ours_row), fov, fov)
    display_checks(pair, theirs_row, ours_row)
    pair.rec["ours"]["shield"] = ours_row.get("shield")
    pair.check_size("ours", out / "ours.png")
    run(["magick", "montage", out / "theirs.png", out / "ours.png", "-tile", "2x1", "-geometry", "+2+2", out / "side.png"])
    if args.meta:
        pair.rec["meta"] = json.loads(args.meta)
    pair.finish()
    print("wrote", out / "side.png")


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--port", type=int, default=int(os.environ.get("OAG_PPSSPP_PORT", 47810)))
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--track", required=True, help="circuit directory, e.g. 03_Track")
    ap.add_argument("--layout", choices=("auto", "forward", "reversed"), default="auto",
                    help="auto: the layout whose own start is on the original's grid pose; forward/reversed asserts it")
    ap.add_argument("--team", help="our --team, when the original's is not the default")
    ap.add_argument("--state", required=True, help="a .ppst to restore into slot 2 first")
    ap.add_argument("--home", default=os.environ.get("PP_HOME"), help="the instance's own HOME (PP_HOME)")
    ap.add_argument("--platform", default="pulse-psp")
    ap.add_argument("--oag-game", default=str(ROOT / "target/debug/oag-game"))
    ap.add_argument("--ours-ticks", type=int, default=None,
                    help="ticks our race runs before the frame. Default: GO_TICK + the original's race clock, read "
                         "out of its RAM (%#x, seconds as a float) after the load, so both sides are the same "
                         "number of seconds after GO and show the same GO board and record panel" % CLOCK_ADDRESS)
    ap.add_argument("--field", action="store_true",
                    help="a race shot: the state is a mid-race frame of a Single Race. Ours takes ALL eight craft "
                         "(pose, team) and the camera from the same stop of the original as the shot, renders at that "
                         "tick (--ticks 0) and the layout must be asserted with --layout forward|reversed (the grid "
                         "probe is meaningless mid-race); the HUD race state is recorded as a caveat")
    ap.add_argument("--pack-near", type=float, default=45.0,
                    help="with --field: how close to the player a craft in view must be (units); the pair is "
                         "refused when the frame is not a pack")
    ap.add_argument("--zone", action="store_true",
                    help="a Zone frame: the state is a Zone race (g_game_mode 6, checked). Ours is the player alone at tick 0 on the "
                         "original's pose and camera, with the zone number, score and zone clock read from the mode object")
    ap.add_argument("--shield-max", type=float, default=135.0,
                    help="with --zone: the shield pool, read as 135.0 on the Assegai in Zone (a full bar shows 100 %%)")
    ap.add_argument("--toast-wait", type=float, default=6.0)
    ap.add_argument("--run-ticks", type=int, default=0,
                    help="after the load and the Autopilot timer restore, run this many of the player's ticks first (a trace "
                         "with no shots) so the frame is taken that far into the state, e.g. onto a speed pad")
    ap.add_argument("--target-clock", type=float, default=None,
                    help="after the load stop and the Autopilot timer restore, run the player's ticks on to the first one whose "
                         "race clock reaches this many seconds (a stored pair's own moment, e.g. 38.4082), instead of "
                         "--run-ticks")
    ap.add_argument("--meta", default="", help="JSON object for the manifest: id, title, track, craft, state, note")
    cp.add_arguments(ap)
    args = ap.parse_args()
    if "OAG_SHOT_DISPLAY" not in os.environ or "DISPLAY" not in os.environ:
        sys.exit("DISPLAY and OAG_SHOT_DISPLAY must name the instance's own Xvfb")
    if not args.home:
        sys.exit("--home (or PP_HOME) must name the instance's own HOME")
    scratch_root = str(ce.MAIN / "data" / "scratch")
    if not os.environ.get("XDG_CONFIG_HOME", "").startswith(scratch_root):
        sys.exit("XDG_CONFIG_HOME must be a directory under data/scratch/<lane>/: oag-game persists settings there")
    pair = ce.Pair(args.platform, args.out)
    plat = pair.plat
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    pair.out = out
    image = ce.MAIN / plat["image"]
    facts = ce.state_facts(args.state)
    pair.rec["original"]["state"] = {"file": args.state, **facts}
    pair.rec["original"]["emulator"] = dict(plat["emulator"])
    pair.rec["ours"]["binary"] = ce.binary_record(args.oag_game)

    profile_dir = Path(os.environ["XDG_CONFIG_HOME"]).parent
    ce.write_ours_settings(plat, profile_dir)
    if args.field:
        # ours races with weapons off the way the original's Racebox row was set (checked below against
        # `g_weapons_enabled` in the state's own RAM); the saved WEAPONS row is what `Mode::SingleRace` reads.
        ce.set_ours_weapons_off(profile_dir)
    flags = ce.ours_flags(plat)
    mode = facts.get("mode", "single_race" if args.field else "time_trial")
    klass = facts.get("class", "venom")
    flags += ["--mode", mode, "--class", klass]
    if args.team:
        flags += ["--team", args.team]

    uv = ["uv", "run", "--with", "websocket-client", "python3"]
    set_window(960, 544)
    resume(args.port)
    run(uv + [HERE / "psp-state.py", "--port", args.port, "--home", args.home, "load", "2", "--file", args.state,
         "--stop-at", hex(SHIP_UPDATE_CRAFT)])
    freeze(args.port)
    time.sleep(args.toast_wait)
    from ppsspp_debugger import Debugger
    if args.target_clock is not None:
        reached = run_to_clock(args.port, args.target_clock)
        pair.rec["original"]["target_clock_s"] = args.target_clock
        pair.check("the original reached the target race clock", abs(reached - args.target_clock) < 0.02,
                   "stopped at %.4f for a target of %.4f" % (reached, args.target_clock))
    elif args.run_ticks > 0:
        run(uv + [HERE / "psp-trace.py", "--port", args.port, "--ticks", args.run_ticks], stdout=subprocess.DEVNULL)
    if args.field or args.zone:
        # After the run to the frame's moment: the timer is written at the stop the frame is taken at (CPU not run
        # after it), so the HUD prints the pickup's own `5.00`, not a countdown the run itself already spent.
        restore_autopilot_timer(args.port, pair)
    clock = Debugger(args.port).read_f32(CLOCK_ADDRESS)
    ram_fov = Debugger(args.port).read_f32(CAMERA_FOV_ADDRESS)
    if not 0.0 <= clock <= 3600.0:
        sys.exit("the race clock at %#x reads %r: not a clock, the address is wrong for this state" % (CLOCK_ADDRESS, clock))
    pair.rec["original"]["race_clock_s"] = clock
    pair.rec["original"]["presented_lag_ticks"] = PRESENTED_LAG
    cp.build(pair, args, "zone" if args.zone else "race" if args.field else "scenery", "savestate_restore", args.state,
             {"value": PRESENTED_LAG, "unit": "ticks", "basis": "measured", "what": "picture, psp-presented-lag.py"})
    if not args.zone:  # a Zone frame renders at tick 0 on the mode object's counters, not on a run-up clock
        if args.ours_ticks is None:
            args.ours_ticks = GO_TICK + ce.tenths_matched_ticks(clock)
        pair.rec["ours"]["ticks"] = args.ours_ticks
        pair.rec["ours"]["race_clock_s"] = (args.ours_ticks - GO_TICK) / 60
        pair.check("race clock phase", abs((args.ours_ticks - GO_TICK) / 60 - clock) <= 0.05,
                   "original %.2f s after GO, ours %.2f s (ticks %d, GO at tick %d)"
                   % (clock, (args.ours_ticks - GO_TICK) / 60, args.ours_ticks, GO_TICK))
    trace_extra = []
    zone_ram = read_zone_ram(args.port) if args.zone else None
    if args.zone and not args.field:
        zone_craft = race_field_module().Field(Debugger(args.port)).player["craft"]
        trace_extra = ["--craft", hex(zone_craft), "--field-out", out / "field.json"]
    if args.field:
        rf = race_field_module()
        from ppsspp_debugger import Debugger as _D
        probe = _D(args.port)
        player_craft = rf.Field(probe).player["craft"]
        wp = probe.read(WEAPONS_ENABLED, 1)[0]
        pair.rec["original"]["weapons_enabled_byte"] = wp
        pair.check("weapons off in the original's own RAM", wp == 0,
                   "g_weapons_enabled byte at %#x reads %d (0 = off)" % (WEAPONS_ENABLED, wp))
        trace_extra = ["--craft", hex(player_craft), "--field-out", out / "field.json"]
        fr = [r for r in rf.Field(probe).rows() if r["player"]][0]
        pair.rec["original"]["hud_at_freeze"] = {"speed_kmh": fr.get("hud_kmh"), "lap_time": fr["lap_time"], "vel_kmh": math.sqrt(sum(v * v for v in fr["vel"])) * 3.6,
                                                 "boost_phase": boost_phase(probe)}
    wid, wx, wy, ww, wh = set_window(*plat["output"])
    pair.check_equal("original window size", [ww, wh], list(plat["output"]))
    shots = out / "shots"
    csv = out / "trace.csv"
    with open(csv, "w") as f:
        run(uv + [HERE / "psp-trace.py", "--port", args.port, "--ticks", 4, "--camera", "--flare", "--shot-every", 1,
                  "--shot-dir", shots] + trace_extra, stdout=f)
    set_window(960, 544)
    run(["magick", shots / ("tick%05d.png" % PRESENTED_LAG), "-crop", "%dx%d+%d+%d" % (ww, wh, wx, wy), "+repage", out / "theirs.png"])
    theirs_row = ce.csv_row(csv, 0)
    theirs = ce.pose_of(theirs_row)
    view = camera_view_of(theirs_row)
    pair.check("the original's camera is the internal (3.0 units) or the close chase (11.64) view",
               view is not None, "camera %.2f units from the craft" % camera_craft_distance(theirs_row))
    if view is None:
        pair.finish()
    fov = ce.speed_fov(theirs_row, INTERNAL_FOV if view == "internal" else 60.0)
    pair.rec["original"]["fov_source"] = ("the view's authored fov (%s) plus 0.075 * dot(fwd, vel), checked against "
                                          "g_camera_fov_degrees (%#x) read at the clock's stop" % (
                                              INTERNAL_FOV if view == "internal" else 60.0, CAMERA_FOV_ADDRESS))
    pair.rec["original"]["fov_ram_at_stop"] = ram_fov
    pair.check("the original's fov in RAM at the stop is the view's authored fov plus the speed term (4 ticks of drift allowed)",
               abs(ram_fov - fov) < 1.0, "RAM %.3f, law %.3f" % (ram_fov, fov))
    world = world_clock_offset(args.port)
    pair.check("the world meshes' animation clock was read", world is not None and world[1] * 2 >= world[2],
               "%s" % (world,))
    if world is not None:
        offset, n, total = world
        pair.rec["original"]["world_clock_offset_s"] = offset
        pair.rec["original"]["world_clock_note"] = ("world meshes' animation clock minus the HUD race clock, the most common "
                                                    "non-zero value (%d of %d updater hits), live" % (n, total))
        # a Zone frame renders at tick 0, so ours' animation clock is the offset alone: the original's mesh clock
        ours_offset = offset + clock if args.zone else offset - GO_TICK / 60
        flags += ["--world-clock-offset", "%.3f" % ours_offset]
        pair.rec["ours"]["world_clock_offset_s"] = ours_offset
    pair.rec["original"]["camera"] = {"pos": [theirs_row["cam_pos_" + a] for a in "xyz"],
                                      "fwd": [theirs_row["cam_fwd_" + a] for a in "xyz"], "source": "psp-trace.py --camera"}
    pair.rec["original"]["shield"] = theirs_row.get("shield")
    pair.rec["original"]["camera_view"] = {"view": view, "source": "camera-to-craft distance (%.2f units; the chase "
                                           "views sit 11.64 away, the cockpit view 3.0)" % camera_craft_distance(theirs_row)}
    pair.rec["ours"]["camera_view"] = view
    view_flags = ["--camera-view", view]
    pair.check_size("original", out / "theirs.png")
    pair.check("no emulator notification over the original's frame", not ce.toast_over_frame(out / "theirs.png"),
               "PPSSPP's green 'Game controller connected' banner (any pad appearing on the machine raises it) covers the top-left")

    if args.zone:
        return zone_pair(args, pair, plat, out, image, facts, flags, csv, theirs_row, theirs, fov, view_flags, profile_dir,
                         clock, zone_ram, world)
    if args.field:
        if args.layout == "auto":
            sys.exit("--field needs --layout forward|reversed: the grid probe is a start-line test")
        layout = args.layout
        declared = facts.get("layout")
        pair.check("layout asserted matches the state's declared layout", declared in (None, layout),
                   "asserted %s, sidecar declares %s" % (layout, declared))
        return field_pair(args, pair, plat, out, image, facts, flags, csv, theirs_row, theirs, fov, layout, profile_dir,
                          clock, view_flags)
    layout, rep = detect_layout(args.oag_game, image, args.track, theirs_row, flags, {"scratch": str(out)})
    pair.rec["ours"]["layout_probe"] = rep
    asserted = {"forward": False, "reversed": True}.get(args.layout)
    pair.check("layout file matches the start position", asserted is None or (layout == "reversed") == asserted,
               "probe says %s (%s); %s" % (
                   layout, ", ".join("%s %.1f units / %.1f deg" % (k, v["start_distance"], v["heading_deg"])
                                     for k, v in rep.items()),
                   "asserted %s" % args.layout if asserted is not None else "nothing asserted, the probe chooses"))
    # the heading is only a layout discriminator here (the wrong layout points 178 degrees away); the ship may have
    # yawed on the grid since the start, and the real pose parity is checked at the rendered tick below
    pair.check("circuit start within 25 units of the original's grid pose, facing the same way (under 90 deg)",
               rep[layout]["start_distance"] <= 25 and rep[layout]["heading_deg"] <= 90,
               "%.1f units, %.1f deg" % (rep[layout]["start_distance"], rep[layout]["heading_deg"]))
    pair.rec["ours"].update({"mode": mode, "class": klass, "team": args.team or "default", "layout": layout})
    pair.rec["original"]["layout"] = layout
    entry = "Data\\Environments\\%s\\track%s.vex" % (args.track, "_reversed" if layout == "reversed" else "")
    base = [args.oag_game, image, "--race", "--track", entry, "--pose-from", csv, "--pose-tick", 0] + flags
    pair.rec["ours"]["flags"] = [str(c) for c in base[2:]] + ["--camera-fov", "%.4f" % fov, "--ticks", args.ours_ticks]
    if pair.failed():
        pair.finish()
    # A craft resting after GO turns, in the original as in ours (bank-to-yaw, docs/physics/grid-state.md), and the
    # original's pose has already turned for the clock it was read at. Running that clock again from it turned ours
    # twice, so `--pose-hold` puts the craft back on the fed pose for the frame and the last trace row; the clock,
    # the GO board, the record panel and the exhaust still run their ticks. The pose check below is then an import
    # check (basis, convention, fov); `scripts/compare-resting-yaw.py` measures the turn itself against the pairs.
    ours_csv = out / "ours-trace.csv"
    cmd = base + ["--pose-hold"]
    pair.rec["ours"]["pose_hold"] = True
    pair.rec["ours"]["flags"] = [str(c) for c in cmd[2:]] + ["--camera-fov", "%.4f" % fov, "--ticks", args.ours_ticks]
    run(cmd + ["--camera-fov", "%.4f" % fov, "--ticks", args.ours_ticks] + view_flags + ["--screenshot", out / "ours.png"],
        stdout=subprocess.DEVNULL)
    run(cmd + ["--camera-fov", "%.4f" % fov, "--ticks", args.ours_ticks, "--trace-out", ours_csv],
        stdout=subprocess.DEVNULL)
    got, bad = ce.read_back(plat, profile_dir)
    pair.rec["ours"]["settings_readback"] = got
    for b in bad:
        pair.check("ours settings", False, b)
    pair.check("ours settings read back", not bad, "%d keys equal the profile" % (len(got) - len(bad)))
    ours_row = ce.csv_row(ours_csv, args.ours_ticks)
    pair.check_pose(theirs, ce.pose_of(ours_row), fov, fov)
    pair.rec["ours"]["shield"] = ours_row.get("shield")
    pair.check_size("ours", out / "ours.png")
    run(["magick", "montage", out / "theirs.png", out / "ours.png", "-tile", "2x1", "-geometry", "+2+2", out / "side.png"])
    if args.meta:
        import json
        pair.rec["meta"] = json.loads(args.meta)
    pair.finish()
    print("wrote", out / "side.png")


if __name__ == "__main__":
    main()
