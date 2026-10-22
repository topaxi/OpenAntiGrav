#!/usr/bin/env python3
"""One original-vs-ours pair of Wipeout Pure (PSP, USA) from a Time Trial state, environment-checked.

    source data/scratch/<lane>/pp/env.sh        # DISPLAY, OAG_PPSSPP_PORT, PP_HOME (scripts/ppsspp-start.sh, USA image)
    OAG_SHOT_DISPLAY=$DISPLAY HOME=$PP_HOME XDG_CONFIG_HOME=<scratch>/profile/cfg \\
    uv run --with websocket-client python3 scripts/psp-pure-pair.py \\
        --port $OAG_PPSSPP_PORT --out data/scratch/<lane>/pairs/vineta --track 01_Vineta_K \\
        --state data/saves/pure-psp/<state>.ppst --class vector --team Feisar

The Pure counterpart of `psp-pulse-pair.py`, shorter because it has no field, Zone or run-to-clock modes. Everything is
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
"""

import argparse
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
#: `+0x1cc` the body pointer, as on Pulse; the body's own offsets are Pure's.
BODY_POINTER = 0x1CC
BODY_VELOCITY = 0x180
#: Disc id and version in a state's file name (`UCUS98612`, the image's 2.00).
SERIAL = "UCUS98612_2.00"
#: Close chase eye: 3.0 up, 11.25 back at the 0.75 craft scale, 11.643 from the body (camera.md, read live).
CLOSE_DISTANCE = 11.643
#: How many stops the presented frame lags the stop it is taken at: Pulse's measurement (`psp-presented-lag.py`), inherited
#: here unmeasured. A resting craft cannot tell: the pose is the same at both stops; the clock differs by one tick.
PRESENTED_LAG = 1


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


def read_tripod(dbg):
    g = stop_at(dbg, CAMERA_COPY_TRIPOD)
    tripod = dbg.read_u32(g["a0"] + 0x40)
    m = struct.unpack("<21f", dbg.read(tripod, 84))
    return {"address": tripod, "right": m[0:3], "up": m[4:7], "fwd": m[8:11], "neg_eye": m[12:15], "fov": m[20]}


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
    ap.add_argument("--track", required=True, help="circuit directory, e.g. 01_Vineta_K")
    ap.add_argument("--state", required=True, help="a .ppst to restore into slot 2 first")
    ap.add_argument("--home", default=os.environ.get("PP_HOME"))
    ap.add_argument("--platform", default="pure-psp")
    ap.add_argument("--oag-game", default=str(ROOT / "target/debug/oag-game"))
    ap.add_argument("--class", dest="klass", default="vector", help="our --class (the speed class the state was made in)")
    ap.add_argument("--team", help="our --team (the original's craft)")
    ap.add_argument("--ours-ticks", type=int, default=None)
    ap.add_argument("--toast-wait", type=float, default=6.0)
    ap.add_argument("--meta", default="", help="JSON object for the manifest: id, title, track, craft, state, note")
    cp.add_arguments(ap)
    args = ap.parse_args()
    if "OAG_SHOT_DISPLAY" not in os.environ or "DISPLAY" not in os.environ:
        sys.exit("DISPLAY and OAG_SHOT_DISPLAY must name the instance's own Xvfb")
    if not args.home:
        sys.exit("--home (or PP_HOME) must name the instance's own HOME")
    if not os.environ.get("XDG_CONFIG_HOME", "").startswith(str(ce.MAIN / "data" / "scratch")):
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
    flags = ce.ours_flags(plat) + ["--mode", facts.get("mode", "time_trial"), "--class", args.klass]
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
    tripod = read_tripod(dbg)
    _, clock_at = read_clock(dbg)
    # The clock is read at the state's own first stop, where the frame on screen is the one before it: load again (a load is
    # bit-identical run to run, psp-state.py), read the clock at the address found above and take the picture there.
    set_window(960, 544)
    resume_if_stopped(args.port)
    run(uv + [HERE / "psp-state.py", "--port", args.port, "--home", args.home, "load", "2", "--file", args.state,
              "--serial", SERIAL, "--stop-at", hex(SHIP_UPDATE_CRAFT)])
    dbg = Debugger(args.port)
    time.sleep(args.toast_wait)
    pose1 = read_pose(dbg)
    clock = dbg.read_f32(clock_at)
    dt = dbg.read_f32(pose1["craft"] + 0x1C8)
    wid, wx, wy, ww, wh = set_window(*plat["output"])
    pair.check_equal("original window size", [ww, wh], list(plat["output"]))
    shot = out / "window.png"
    run(["import", "-window", "root", "-crop", "%dx%d+%d+%d" % (ww, wh, wx, wy), "+repage", shot])
    shot.replace(out / "theirs.png")
    shown = clock - PRESENTED_LAG * dt
    clock = shown
    pair.rec["original"]["race_clock_at_stop_s"] = shown + PRESENTED_LAG * dt
    pair.rec["original"].update({"race_clock_s": clock, "frame_dt_s": dt, "race_clock_address": "%#x" % clock_at,
                                 "presented_lag_ticks": PRESENTED_LAG})
    cp.build(pair, args, "scenery", "savestate_restore", args.state,
             {"value": PRESENTED_LAG, "unit": "ticks", "basis": "inherited from Pulse PSP, unmeasured on Pure",
              "what": "picture"})
    drift = math.dist(pose0["pos"], pose1["pos"])
    pair.check("the state loads to the same pose twice (a load is bit-identical)", drift < 0.001,
               "%.5f units between the two loads' first stops" % drift)
    eye = [-v for v in tripod["neg_eye"]]
    eye_body = body_frame([e - p for e, p in zip(eye, pose0["pos"])], pose0)
    dist = math.dist(eye, pose0["pos"])
    pair.check("the original's camera is the Close chase view (11.643 units from the body, 3.0 up, 11.25 back)",
               abs(dist - CLOSE_DISTANCE) < 0.1 and abs(eye_body[1] - 3.0) < 0.05 and abs(eye_body[2] + 11.25) < 0.1,
               "eye in the body frame (right, up, fwd) = (%.3f, %.3f, %.3f), %.3f from the body" % (*eye_body, dist))
    fov = tripod["fov"]
    pair.rec["original"]["fov_deg"] = fov
    pair.rec["original"]["camera"] = {"pos": eye, "right": tripod["right"], "up": tripod["up"], "fwd": tripod["fwd"],
                                      "source": "*(controller+0x40) at 0x08849970, camera.md"}
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

    ticks = args.ours_ticks if args.ours_ticks is not None else ce.go_tick() + ce.tenths_matched_ticks(clock)
    pair.rec["ours"]["ticks"] = ticks
    pair.rec["ours"]["race_clock_s"] = (ticks - ce.go_tick()) / 60
    pair.check("race clock phase", abs((ticks - ce.go_tick()) / 60 - clock) <= 0.05,
               "original %.2f s, ours %.2f s (ticks %d, GO at tick %d)" % (clock, (ticks - ce.go_tick()) / 60, ticks, ce.go_tick()))
    base = [args.oag_game, image, "--race", "--track", "Data\\Environments\\%s\\track.vex" % args.track, "--pose-from", csv, "--pose-tick", 0, "--no-camera",
            "--pose-hold", "--camera-fov", "%.4f" % fov, "--camera-view", "close", "--ticks", ticks] + flags
    pair.rec["ours"]["flags"] = [str(c) for c in base[2:]]
    pair.rec["ours"].update({"mode": facts.get("mode", "time_trial"), "class": args.klass, "team": args.team or "default",
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
    pair.check("the circuit is the original's: ours' own spawn is within 25 units of its grid pose, facing the same way",
               start <= 25 and heading <= 90, "%.1f units, %.1f deg" % (start, heading))
    pair.rec["ours"]["spawn_probe"] = {"start_distance": start, "heading_deg": heading}
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
    off = craft_offset(out / "theirs.png", out / "ours.png")
    pair.rec["craft_offset_px"] = off
    pair.check("ours' hull is on the original's (ours' own chase camera is checked here: its trace has no camera columns)",
               abs(off["dx"]) <= 3 and abs(off["dy"]) <= 3,
               "best edge correlation %.3f at (%+d, %+d) px, %.3f at (0, 0); a lost shear reads about 24 px in dy"
               % (off["ncc_at_best"], off["dx"], off["dy"], off["ncc_at_zero"]))
    run(["magick", "montage", out / "theirs.png", out / "ours.png", "-tile", "2x1", "-geometry", "+2+2", out / "side.png"])
    if args.meta:
        import json
        pair.rec["meta"] = json.loads(args.meta)
    pair.finish()
    print("wrote", out / "side.png")


if __name__ == "__main__":
    main()
