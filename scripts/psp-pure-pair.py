#!/usr/bin/env python3
"""One original-vs-ours pair of Wipeout Pure (PSP, USA) from a Time Trial state, environment-checked.

    source data/scratch/<lane>/pp/env.sh        # DISPLAY, OAG_PPSSPP_PORT, PP_HOME (scripts/ppsspp-start.sh, USA image)
    OAG_SHOT_DISPLAY=$DISPLAY HOME=$PP_HOME XDG_CONFIG_HOME=<scratch>/profile/cfg \\
    uv run --with websocket-client python3 scripts/psp-pure-pair.py \\
        --port $OAG_PPSSPP_PORT --out data/scratch/<lane>/pairs/vineta --track 01_Vineta_K \\
        --state data/saves/pure-psp/<state>.ppst --class vector --team Feisar

The Pure counterpart of `psp-pulse-pair.py`, shorter because it has no field or run-to-clock modes; `--zone` is the Zone frame (below). Everything is
read out of the loaded state's own RAM at its first `Ship_UpdateCraft` stop (`0x0892d214`, `a0` craft, `a1` body):

- the craft's basis, position and velocity from the body (`+0x00`, `+0x30`, `+0x180`; Pure's body is Pulse's shifted by
  `0x40` after the inverse inertia, docs/ghidra/functions/psp-pure-usa/craft-physics.md);
- the camera from the active tripod `*(controller + 0x40)` at `Camera_CopyTripodToView` (`0x08849970`, `a0` the controller),
  rows `right, up, forward` and the negated eye at `+0x30`, the fov at `+0x50` (camera.md);
- the race clock from `Race_UpdateLaunchGrade`'s `a0 + 0x130` (`0x0880f870`, craft-physics.md; it counts seconds).

Ours takes the craft's pose (`--pose-from`) and builds its own chase camera: Pure's tripod matrix is a SHEAR (right axis of
length 0.995, third axis not perpendicular to the second), and a camera matrix fed to ours is re-orthonormalised by its
`look_at`, which draws the pitch 5.71 degrees wrong (camera.md, "The view matrix is a shear"). So the camera is not fed;
ours' own rig (`chase_view_sheared`) draws it and the pair checks ours' eye against the original's read one.

`--zone ZONE,SCORE` makes the pair a Zone frame, from a state saved while the Autopilot flew a Zone race
(`scripts/psp-pure-zone-state.py`): ours renders at tick 0 (`--mode zone`) on the original's craft pose, with the zone number and
score passed through `--hud-state` and the Autopilot countdown read from the pickup record. Pure's Zone counters are not located in
RAM (no address is documented), so the zone and the score are read off the original's HUD by eye and given here; the zone clock is
written as 0 and recorded as a caveat. In Zone ours reads Zone_01's own camera blocks (`oag_raceplay::load::zone_cameras`: Close eye 2.25 up and 7.125 back, not the racing
3 and 11.25), so the close-eye and the hull-offset checks are recorded as numbers, not gated (the engine glow and ground shadow dominate the hull box),
and the fov gate drops its at-rest speed term: the field is the authored 70 plus 0.075 times the forward speed, fed with `--camera-fov`.
"""

import argparse
import json
import math
import os
import struct
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
sys.path.insert(0, str(HERE))
import compare_env as ce  # noqa: E402
import compare_provenance as cp  # noqa: E402

SHIP_UPDATE_CRAFT = 0x0892D214
CAMERA_COPY_TRIPOD = 0x08849970
RACE_LAUNCH_GRADE = 0x0880F870
CLOCK_OFFSET = 0x130
#: The scenery clock: `*(0x08A8C5F4) + 0x44` is the world-state object's running time, advanced by the frame's dt at `0x0880c78c` and copied
#: into each animated node (`node + 0x134`) by the node updater at `0x0888681c` (docs/ghidra/functions/psp-pure-usa/world-clock.md).
WORLD_STATE_POINTER = 0x08A8C5F4
WORLD_CLOCK_FIELD = 0x44
#: `+0x1cc` the body pointer, as on Pulse; the body's own offsets are Pure's.
BODY_POINTER = 0x1CC
BODY_VELOCITY = 0x180
#: Disc id and version in a state's file name (`UCUS98612`, the image's 2.00).
SERIAL = "UCUS98612_2.00"
#: Close chase eye: 3.0 up, 11.25 back at the 0.75 craft scale, 11.643 from the body (camera.md, read live).
CLOSE_DISTANCE = 11.643
#: How many stops the presented frame lags the stop it is taken at (`compare_env.PURE_PRESENTED_LAG`). A resting craft's pose
#: is the same at every one of them; the clocks differ.
PRESENTED_LAG = ce.PURE_PRESENTED_LAG
#: The player's Autopilot pickup record of a Zone race flown on the Autopilot: its running byte `+0x154` and its timer `+0x15c`
#: (`docs/ghidra/functions/psp-pure-usa/autopilot.md`). Its address belongs to the state (the `autopilot_record` of the state's
#: `.json` sidecar, `psp-pure-zone-state.py`; `--autopilot-record` overrides); a load restores it.
#: The HUD's km/h is the body's speed in units a second times this (Pulse's factor, `psp-pulse-pair.py`; checked here against the HUD).
SPEED_TO_KMH = 3.6


def track_vex(track):
    """The `track.vex` path ours takes: a bare directory name is a circuit under `Data\\Environments`, a name with a separator
    (`ClassicsNS/01_Karbonis`, the Classic league's own location) is relative to `Data`."""
    if "/" in track or "\\" in track:
        return "Data\\%s\\track.vex" % track.replace("/", "\\")
    return "Data\\Environments\\%s\\track.vex" % track


#: The chase-camera hull's region of the 480x272 frame: x, y, w, h (the craft sits bottom-centre, about 120 px wide).
CRAFT_REGION = (180, 176, 120, 96)
#: How far, in px, the best shift is searched.
CRAFT_SEARCH = 12


def edge_frame(path):
    """A 480x272 frame's blurred edge map, 8-bit grey bytes: the recipe `compare_env.overlap_score` scores frames with."""
    return subprocess.run(["magick", str(path), "-colorspace", "Gray", "-blur", "0x1.2", "-edge", "1", "-depth", "8", "gray:-"],
                          capture_output=True, check=True).stdout


def craft_offset(theirs, ours):
    """Where the original's hull sits against ours', in px, as the best normalised cross-correlation of the two frames' edge
    maps over `CRAFT_REGION` (edges, so the hull's shading, darker in ours, does not count). `dx`, `dy`: how far the
    original's picture is shifted right and down of ours' where the hull edges line up; (0, 0) is the same place. A posed
    craft drawn through a camera with the wrong pitch is about 24 px off in `dy` (camera.md, "The view matrix is a shear")."""
    from operator import mul
    a, b = edge_frame(ours), edge_frame(theirs)
    x, y, w, h = CRAFT_REGION

    def patch(buf, dx, dy):
        return [v for j in range(h) for v in buf[(y + dy + j) * 480 + x + dx:(y + dy + j) * 480 + x + dx + w]]

    def centred(v):
        m = sum(v) / len(v)
        return [c - m for c in v]

    pa = centred(patch(a, 0, 0))
    na = sum(map(mul, pa, pa)) ** 0.5
    scores = {}
    for dy in range(-CRAFT_SEARCH, CRAFT_SEARCH + 1):
        for dx in range(-CRAFT_SEARCH, CRAFT_SEARCH + 1):
            if x + dx < 0 or y + dy < 0 or x + dx + w > 480 or y + dy + h > 272:
                continue
            pb = centred(patch(b, dx, dy))
            nb = sum(map(mul, pb, pb)) ** 0.5
            scores[(dx, dy)] = sum(map(mul, pa, pb)) / (na * nb) if na and nb else 0.0
    (bx, by), best = max(scores.items(), key=lambda kv: kv[1])
    return {"dx": bx, "dy": by, "ncc_at_best": best, "ncc_at_zero": scores[(0, 0)],
            "region": list(CRAFT_REGION), "search_px": CRAFT_SEARCH}


def run(cmd, **kw):
    print("+", " ".join(str(c) for c in cmd), file=sys.stderr)
    return subprocess.run([str(c) for c in cmd], check=True, **kw)


def xdo(*args, capture=False):
    return subprocess.run(["xdotool", *args], capture_output=capture, text=True, check=True)


def emulator_window():
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


def regs(dbg):
    out = {}
    for c in dbg.call("cpu.getAllRegs")["categories"]:
        out.update(zip(c["registerNames"], c["uintValues"]))
    return out


def stop_at(dbg, address, timeout=30.0):
    """Resume to the next hit of `address`, leave the CPU stopped there and return the registers."""
    dbg.add_breakpoint(address)
    dbg.call("cpu.status")
    dbg.pending = []
    dbg.call("cpu.resume")
    try:
        dbg.wait_for_break(address, timeout=timeout)
    finally:
        if not dbg.is_stepping():
            dbg.brk()
        dbg.remove_breakpoint(address)
    return regs(dbg)


def resume_if_stopped(port):
    from ppsspp_debugger import Debugger
    dbg = Debugger(port)
    if dbg.is_stepping():
        dbg.call("cpu.resume")
        time.sleep(1.0)


def read_pose(dbg):
    """At a `Ship_UpdateCraft` stop: the body's basis, position, velocity, and the race clock and tripod, read at their own stops
    afterwards (the craft is at rest or the pair is refused for a pose drift between the two reads)."""
    g = regs(dbg)
    craft, body = g["a0"], g["a1"]
    if dbg.read_u32(craft + BODY_POINTER) != body:
        sys.exit("craft+0x1cc is not a1 at %#x: this is not Pure's Ship_UpdateCraft" % SHIP_UPDATE_CRAFT)
    rows = struct.unpack("<12f", dbg.read(body, 48))
    pos = struct.unpack("<3f", dbg.read(body + 0x30, 12))
    vel = struct.unpack("<3f", dbg.read(body + BODY_VELOCITY, 12))
    return {"craft": craft, "body": body, "right": rows[0:3], "up": rows[4:7], "fwd": rows[8:11], "pos": pos, "vel": vel}


def read_tripod(dbg, craft=None):
    """The active tripod at `Camera_CopyTripodToView`. With `craft`, also that craft's body pose read at the same stop (`body`): the
    tripod is the tick AFTER the pose `read_pose` reads at `Ship_UpdateCraft`'s entry, so a moving craft's eye is measured against this one."""
    g = stop_at(dbg, CAMERA_COPY_TRIPOD)
    tripod = dbg.read_u32(g["a0"] + 0x40)
    m = struct.unpack("<21f", dbg.read(tripod, 84))
    out = {"address": tripod, "right": m[0:3], "up": m[4:7], "fwd": m[8:11], "neg_eye": m[12:15], "fov": m[20]}
    if craft:
        body = dbg.read_u32(craft + BODY_POINTER)
        rows = struct.unpack("<12f", dbg.read(body, 48))
        out["body"] = {"right": rows[0:3], "up": rows[4:7], "fwd": rows[8:11], "pos": struct.unpack("<3f", dbg.read(body + 0x30, 12))}
    return out


def read_clock(dbg):
    g = stop_at(dbg, RACE_LAUNCH_GRADE)
    return dbg.read_f32(g["a0"] + CLOCK_OFFSET), g["a0"] + CLOCK_OFFSET


def body_frame(vec, pose):
    """`vec` (world) in the craft's right, up, forward axes."""
    return [sum(a * b for a, b in zip(vec, axis)) for axis in (pose["right"], pose["up"], pose["fwd"])]


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--port", type=int, default=int(os.environ.get("OAG_PPSSPP_PORT", 47810)))
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--track", required=True,
                    help="circuit directory under Data\\Environments, e.g. 01_Vineta_K, or a Data-relative path with a separator, e.g. ClassicsNS/01_Karbonis")
    ap.add_argument("--state", required=True, help="a .ppst to restore into slot 2 first")
    ap.add_argument("--home", default=os.environ.get("PP_HOME"))
    ap.add_argument("--platform", default="pure-psp")
    ap.add_argument("--oag-game", default=str(ROOT / "target/debug/oag-game"))
    ap.add_argument("--class", dest="klass", default="vector", help="our --class (the speed class the state was made in)")
    ap.add_argument("--team", help="our --team (the original's craft)")
    ap.add_argument("--ours-ticks", type=int, default=None)
    ap.add_argument("--toast-wait", type=float, default=6.0)
    ap.add_argument("--hull-differs", default="", metavar="CAVEAT",
                    help="the original draws another hull than ours here (the Classic league): skip the hull-offset gate and record this text as a caveat")
    ap.add_argument("--meta", default="", help="JSON object for the manifest: id, title, track, craft, state, note")
    ap.add_argument("--zone", default="", metavar="ZONE,SCORE",
                    help="a Zone frame: the zone number and score the original's HUD shows in the state (read by eye, no RAM address is "
                         "documented). Ours renders at tick 0 in --mode zone with them")
    ap.add_argument("--autopilot-record", type=lambda v: int(v, 0), default=None,
                    help="with --zone: the Autopilot pickup record's address (default: the state's sidecar `autopilot_record`)")
    ap.add_argument("--zone-kmh", type=float, default=None, help="with --zone: the km/h the original's HUD shows (default: body speed * 3.6)")
    cp.add_arguments(ap)
    args = ap.parse_args()
    if "OAG_SHOT_DISPLAY" not in os.environ or "DISPLAY" not in os.environ:
        sys.exit("DISPLAY and OAG_SHOT_DISPLAY must name the instance's own Xvfb")
    if not args.home:
        sys.exit("--home (or PP_HOME) must name the instance's own HOME")
    if not os.environ.get("XDG_CONFIG_HOME", "").startswith(str(ce.MAIN / "data" / "scratch")):
        sys.exit("XDG_CONFIG_HOME must be a directory under data/scratch/<lane>/: oag-game persists settings there")
    zone = None
    if args.zone:
        zone_number, zone_score = (int(v) for v in args.zone.split(","))
        zone = {"zone": zone_number, "score": zone_score}
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
    mode = "zone" if zone else facts.get("mode", "time_trial")
    flags = ce.ours_flags(plat) + ["--mode", mode, "--class", args.klass]
    if args.team:
        flags += ["--team", args.team]

    uv = ["uv", "run", "--with", "websocket-client", "python3"]
    from ppsspp_debugger import Debugger
    set_window(960, 544)
    resume_if_stopped(args.port)
    run(uv + [HERE / "psp-state.py", "--port", args.port, "--home", args.home, "load", "2", "--file", args.state,
              "--serial", SERIAL, "--stop-at", hex(SHIP_UPDATE_CRAFT)])
    dbg = Debugger(args.port)
    pose0 = read_pose(dbg)
    tripod = read_tripod(dbg, pose0["craft"] if zone else None)
    clock_at = None if zone else read_clock(dbg)[1]
    # The clock is read at the state's own first stop, where the frame on screen is the one before it: load again (a load is
    # bit-identical run to run, psp-state.py), read the clock at the address found above and take the picture there.
    set_window(960, 544)
    resume_if_stopped(args.port)
    run(uv + [HERE / "psp-state.py", "--port", args.port, "--home", args.home, "load", "2", "--file", args.state,
              "--serial", SERIAL, "--stop-at", hex(SHIP_UPDATE_CRAFT)])
    dbg = Debugger(args.port)
    time.sleep(args.toast_wait)
    pose1 = read_pose(dbg)
    clock = 0.0 if zone else dbg.read_f32(clock_at)
    dt = dbg.read_f32(pose1["craft"] + 0x1C8)
    world_state = dbg.read_u32(WORLD_STATE_POINTER)
    world_ram = dbg.read_f32(world_state + WORLD_CLOCK_FIELD) if 0x08800000 <= world_state < 0x0A000000 else None
    if zone:
        sidecar = Path(args.state + ".json")
        record = args.autopilot_record or (int(json.loads(sidecar.read_text()).get("autopilot_record", "0"), 0) if sidecar.exists() else 0)
        if not record:
            sys.exit("--zone needs the Autopilot record's address: --autopilot-record, or `autopilot_record` in %s" % sidecar)
        autopilot = {"record": hex(record), "running": dbg.read_u8(record + 0x154), "timer_s": dbg.read_f32(record + 0x15C)}
    else:
        autopilot = None
    wid, wx, wy, ww, wh = set_window(*plat["output"])
    pair.check_equal("original window size", [ww, wh], list(plat["output"]))
    shot = out / "window.png"
    run(["import", "-window", "root", "-crop", "%dx%d+%d+%d" % (ww, wh, wx, wy), "+repage", shot])
    shot.replace(out / "theirs.png")
    shown = clock - PRESENTED_LAG * dt
    clock = shown
    world_shown = None if world_ram is None else world_ram - PRESENTED_LAG * dt
    pair.check("the scenery clock was read (world-state pointer %#x, +%#x)" % (WORLD_STATE_POINTER, WORLD_CLOCK_FIELD),
               world_shown is not None and math.isfinite(world_shown) and world_shown > 0.0,
               "state %#x, clock %s" % (world_state, world_ram))
    if world_shown is not None and world_shown > 0.0:
        pair.rec["original"]["world_clock_s"] = world_shown
        pair.rec["original"]["world_clock_offset_s"] = world_shown - shown
        pair.rec["original"]["world_clock_note"] = ("the world-state object's running time (*(%#x)+%#x) at the stop, less the "
                                                    "presented-frame lag, less the race clock" % (WORLD_STATE_POINTER, WORLD_CLOCK_FIELD))
    if zone:
        pair.rec["original"].update({"frame_dt_s": dt, "presented_lag_ticks": PRESENTED_LAG, "autopilot_at_stop": autopilot,
                                     "zone_hud_read_by_eye": zone})
        pair.rec["original"]["speed_units_s"] = math.hypot(*pose0["vel"])
    else:
        pair.rec["original"]["race_clock_at_stop_s"] = shown + PRESENTED_LAG * dt
        pair.rec["original"].update({"race_clock_s": clock, "frame_dt_s": dt, "race_clock_address": "%#x" % clock_at,
                                     "presented_lag_ticks": PRESENTED_LAG})
    cp.build(pair, args, "scenery", "savestate_restore", args.state, ce.pure_lag_provenance(bool(zone)))
    drift = math.dist(pose0["pos"], pose1["pos"])
    pair.check("the state loads to the same pose twice (a load is bit-identical)", drift < 0.001,
               "%.5f units between the two loads' first stops" % drift)
    eye = [-v for v in tripod["neg_eye"]]
    eye_body = body_frame([e - p for e, p in zip(eye, pose0["pos"])], pose0)
    dist = math.dist(eye, pose0["pos"])
    eye_text = "eye in the body frame (right, up, fwd) = (%.3f, %.3f, %.3f), %.3f from the body" % (*eye_body, dist)
    if zone:
        tb = tripod["body"]
        tick_eye = body_frame([e - p for e, p in zip(eye, tb["pos"])], tb)
        tick_dist = math.dist(eye, tb["pos"])
        pair.rec["original"]["eye_body_frame"] = {
            "at_the_tripods_own_stop": {"right": tick_eye[0], "up": tick_eye[1], "fwd": tick_eye[2], "distance": tick_dist},
            "against_the_pose_one_tick_earlier": {"right": eye_body[0], "up": eye_body[1], "fwd": eye_body[2], "distance": dist},
            "body_advance_between_the_two_stops": math.dist(pose0["pos"], tb["pos"]),
            "close_rest": [0.0, 3.0, -11.25], "close_distance": CLOSE_DISTANCE}
        pair.caveat("the original's Zone eye in the body frame (right, up, fwd) = (%.3f, %.3f, %.3f), %.3f from "
                    "the body, read against the body at the tripod's own stop (against the pose one tick earlier: %s; the body moved %.2f units "
                    "between the stops); the racing Close rig is (0, 3.000, -11.250), %.3f. Zone_01's file authors Close at pos_height 3.0 and pos_length -9.5, "
                    "7.125 back and 2.25 up after the 0.75 scale, which ours' own rig reads" % (*tick_eye, tick_dist, eye_text.split("= ")[1], math.dist(pose0["pos"], tb["pos"]), CLOSE_DISTANCE))
    else:
        pair.check("the original's camera is the Close chase view (11.643 units from the body, 3.0 up, 11.25 back)",
                   abs(dist - CLOSE_DISTANCE) < 0.1 and abs(eye_body[1] - 3.0) < 0.05 and abs(eye_body[2] + 11.25) < 0.1, eye_text)
    fov = tripod["fov"]
    pair.rec["original"]["fov_deg"] = fov
    pair.rec["original"]["camera"] = {"pos": eye, "right": tripod["right"], "up": tripod["up"], "fwd": tripod["fwd"],
                                      "source": "*(controller+0x40) at 0x08849970, camera.md"}
    if zone:
        pair.check("the original's fov is a plausible lens (read from the tripod, fed to ours as it is)", 40.0 <= fov <= 110.0,
                   "fov %.4f, speed %.3f units/s" % (fov, math.hypot(*pose0["vel"])))
    else:
        pair.check("the original's fov is the authored 60 at rest (no speed term read for Pure)", abs(fov - 60.0) < 0.05 and
                   math.hypot(*pose0["vel"]) < 1.0, "fov %.4f, speed %.3f units/s" % (fov, math.hypot(*pose0["vel"])))
    pair.check_size("original", out / "theirs.png")
    pair.check("no emulator notification over the original's frame", not ce.toast_over_frame(out / "theirs.png"),
               "PPSSPP's green banner covers the top-left")

    csv = out / "trace.csv"
    values = {"tick": 0, "dt": 1 / 60, "pos_x": pose0["pos"][0], "pos_y": pose0["pos"][1], "pos_z": pose0["pos"][2],
              "vel_x": pose0["vel"][0], "vel_y": pose0["vel"][1], "vel_z": pose0["vel"][2],
              "speed": math.hypot(*pose0["vel"])}
    for name, key in (("right", "right"), ("up", "up"), ("fwd", "fwd")):
        for a, v in zip("xyz", pose0[key]):
            values["%s_%s" % (name, a)] = v
    for name in ("right", "up", "fwd"):
        for a, v in zip("xyz", tripod[name]):
            values["cam_%s_%s" % (name, a)] = v
    for a, v in zip("xyz", eye):
        values["cam_pos_" + a] = v
    ce.write_trace_row(csv, values)

    if zone:
        ticks = 0
        pair.rec["ours"]["ticks"] = 0
    else:
        ticks = args.ours_ticks if args.ours_ticks is not None else ce.go_tick() + ce.tenths_matched_ticks(clock)
        pair.rec["ours"]["ticks"] = ticks
        pair.rec["ours"]["race_clock_s"] = (ticks - ce.go_tick()) / 60
        pair.check("race clock phase", abs((ticks - ce.go_tick()) / 60 - clock) <= 0.05,
                   "original %.2f s, ours %.2f s (ticks %d, GO at tick %d)" % (clock, (ticks - ce.go_tick()) / 60, ticks, ce.go_tick()))
    base = [args.oag_game, image, "--race", "--track", track_vex(args.track), "--pose-from", csv, "--pose-tick", 0, "--no-camera",
            "--pose-hold", "--camera-fov", "%.4f" % fov, "--camera-view", "close", "--ticks", ticks] + flags
    if zone:
        kmh = args.zone_kmh if args.zone_kmh is not None else math.hypot(*pose0["vel"]) * SPEED_TO_KMH
        hud = "1,1,0,0,%.2f,100,0,%d,%d,0" % (kmh, zone["zone"], zone["score"])
        base += ["--hud-state", hud]
        if autopilot["running"] and autopilot["timer_s"] > 0:
            base += ["--hud-autopilot", "%.4f" % autopilot["timer_s"]]
        pair.rec["ours"]["hud_state"] = hud
        pair.caveat("Zone state imposed on ours: zone %d and score %d read off the original's HUD by eye (Pure's Zone counters are not "
                    "located in RAM), %.1f km/h; the zone clock is written as 0, ours runs no ticks (no exhaust or trail history), and the "
                    "shield is drawn full" % (zone["zone"], zone["score"], kmh))
    if world_shown is not None and world_shown > 0.0:
        # ours' animation clock is ticks / 60 plus the offset, so the offset puts ours' clock on the original's at the tick ours renders
        ours_offset = world_shown - ticks / 60
        base += ["--world-clock-offset", "%.3f" % ours_offset]
        pair.rec["ours"]["world_clock_offset_s"] = ours_offset
    pair.rec["ours"]["flags"] = [str(c) for c in base[2:]]
    pair.rec["ours"].update({"mode": mode, "class": args.klass, "team": args.team or "default",
                             "camera": "ours' own Close chase rig, not fed (Pure's tripod is sheared)"})
    if pair.failed():
        pair.finish()
    env = {**os.environ, **ce.ours_env(profile_dir)}
    probe_csv = out / "spawn-trace.csv"
    run([args.oag_game, image, "--race", "--track", base[4], "--ticks", 1, "--trace-out", probe_csv] + flags,
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, env=env)
    spawn = ce.pose_of(ce.csv_row(probe_csv, 0))
    start = math.dist(spawn["pos"], pose0["pos"])
    heading = ce.vec_angle(spawn["fwd"], pose0["fwd"])
    pair.rec["ours"]["spawn_probe"] = {"start_distance": start, "heading_deg": heading}
    if zone:
        # A Zone state is mid-circuit, so ours' own spawn says nothing; the posed craft's height over the road in ours' trace does.
        pair.rec["ours"]["spawn_probe"]["note"] = "informational: the state is mid-circuit"
    else:
        pair.check("the circuit is the original's: ours' own spawn is within 25 units of its grid pose, facing the same way",
                   start <= 25 and heading <= 90, "%.1f units, %.1f deg" % (start, heading))
    if pair.failed():
        pair.finish()
    run(base + ["--screenshot", out / "ours.png"], stdout=subprocess.DEVNULL, env=env)
    ours_csv = out / "ours-trace.csv"
    run(base + ["--trace-out", ours_csv], stdout=subprocess.DEVNULL, env=env)
    got, bad = ce.read_back(plat, profile_dir)
    pair.rec["ours"]["settings_readback"] = got
    for b in bad:
        pair.check("ours settings", False, b)
    pair.check("ours settings read back", not bad, "%d keys equal the profile" % (len(got) - len(bad)))
    ours_row = ce.csv_row(ours_csv, ticks)
    pair.check_pose(ce.pose_of(ce.csv_row(csv, 0)), ce.pose_of(ours_row), fov, fov)
    pair.check_size("ours", out / "ours.png")
    if zone:
        pair.rec["ours"]["posed_row"] = {k: v for k, v in ours_row.items() if k in ("height", "spline", "speed", "grounded", "zone")}
    off = craft_offset(out / "theirs.png", out / "ours.png")
    pair.rec["craft_offset_px"] = off
    if zone:
        pair.rec["craft_offset_px"]["gated"] = False
        pair.caveat("hull offset recorded, not gated: the engine glow and the ground shadow differ in this region (dx %+d, dy %+d px, edge correlation %.3f)"
                    % (off["dx"], off["dy"], off["ncc_at_best"]))
    elif args.hull_differs:
        # The offset search assumes both sides draw one hull; with a different one it reads noise (Karbonis: +7 px, edge correlation 0.09).
        # The camera is still held by the eye and fov checks above, and the pair says so.
        pair.caveat(args.hull_differs)
        pair.rec["craft_offset_px"]["not_checked"] = "the two hulls differ, so the shift of one against the other is noise"
    else:
        pair.check("ours' hull is on the original's (ours' own chase camera is checked here: its trace has no camera columns)",
                   abs(off["dx"]) <= 3 and abs(off["dy"]) <= 3,
                   "best edge correlation %.3f at (%+d, %+d) px, %.3f at (0, 0); a lost shear reads about 24 px in dy"
                   % (off["ncc_at_best"], off["dx"], off["dy"], off["ncc_at_zero"]))
    run(["magick", "montage", out / "theirs.png", out / "ours.png", "-tile", "2x1", "-geometry", "+2+2", out / "side.png"])
    if args.meta:
        pair.rec["meta"] = json.loads(args.meta)
    pair.finish()
    print("wrote", out / "side.png")


if __name__ == "__main__":
    main()
