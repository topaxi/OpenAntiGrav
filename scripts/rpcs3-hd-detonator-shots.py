#!/usr/bin/env python3
"""Paused frames of a Detonator race on RPCS3: frame, camera, pose, clocks, optionally with the cannon held.

    source data/scratch/<lane>/env.sh
    OAG_COMPARE_PLATFORM=hd-fury OAG_RPCS3_ATTACH=1 UV_CACHE_DIR=~/.cache/uv UV_OFFLINE=1 \\
    uv run --with evdev python3 scripts/emu-run.py --child-status <log-dir>/status.json -- \\
        python3 scripts/rpcs3-hd-detonator-shots.py --out data/scratch/<lane>/shots \\
            --shot 6.0 --fire-clock 9.0 --fire-hold 0.5 --shot 9.4

Attached to a `serve` restored from a Detonator Racebox grid state (`scripts/emu-restore-state.sh`, Racebox
RACE TYPE DETONATOR, `GetMode()==14`), this taps START RACE until the lap clock runs, then plays the race on:
for every `--shot T` it pauses at the first stop whose lap clock (`ship+0x7014`) is at or past T and keeps the
frame (`rpcs3-drive.py grab_with_clock`, the world clock beside it), the camera read from the RSX pushbuffer
and the player's pose, to `OUT/NN.png` and `OUT/NN.json` in the shape `rpcs3-drive.py capture` writes (so
`rpcs3-hd-pair.py --json OUT/NN.json` takes it). `--fire-clock F --fire-hold H` holds a button (`--fire-button`, default TRIANGLE; SQUARE changes the camera view) to fire the pulse cannon
for H wall seconds from the first stop at or past lap clock F; the record carries the clocks before and after
the hold, so ours runs the same ticks with the same button held. The Detonator craft drives itself (auto-speed):
no throttle is pressed.

Each shot is taken twice, the second `--interval` seconds later, because the camera pick compares two frames
(`ps3_pose.pick_camera`); the first is the one kept. `docs/ghidra/functions/ps3-hdfury-eu/detonator-mode.md`.
"""
import argparse
import importlib.util
import json
import os
import struct
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
_spec = importlib.util.spec_from_file_location("rpcs3_drive", HERE / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(drive)
import compare_env  # noqa: E402
import ps3_pose  # noqa: E402
import rpcs3_hd_field  # noqa: E402
import rpcs3_place as place  # noqa: E402
from rpcs3_debugger import Debugger  # noqa: E402

CLOCK = 0x7014


def race_clock(gdb, ship):
    return struct.unpack(">f", gdb.read(ship + CLOCK, 4))[0]


def run_until(gdb, ship, target, label):
    """Resume in short steps and stop paused at the first stop whose lap clock is at or past `target`."""
    for _ in range(400):
        gdb.pause()
        now = race_clock(gdb, ship)
        if now >= target:
            return now
        gdb.resume()
        time.sleep(max(0.05, min(0.4, (target - now) * 0.5)))
    sys.exit("%s: the lap clock stopped short of %.2f (a race still on START RACE?)" % (label, target))


def observe(session, gdb, ship, body, out, stem, flush):
    """One paused frame: the screenshot with its world clock, the pushbuffer camera candidates, the pose."""
    time.sleep(flush)
    shot, grab_clock = drive.grab_with_clock(out / ("%s.png" % stem), trim=True)
    pose = place.read_pose(gdb, body)
    pose["race_clock_s"] = race_clock(gdb, ship)
    try:
        pose["scene_width"] = __import__("hd_scene_width").live(gdb.read)
    except Exception as error:  # noqa: BLE001 - a bonus
        print("  %s: no scene width level (%s)" % (stem, error), flush=True)
    try:
        pose["gantry_seconds"] = rpcs3_hd_field.gantry_seconds(rpcs3_hd_field.Mem(session.open_mem()))
    except Exception as error:  # noqa: BLE001 - a bonus
        print("  %s: no gantry time (%s)" % (stem, error), flush=True)
    pose["scenery_clock"] = (None if grab_clock is None or grab_clock["world_clock_s"] is None else
                             {"time_s": grab_clock["world_clock_s"], "nodes": grab_clock["nodes"], "total": grab_clock["total"]})
    blobs = []
    for chain, size in drive.PUSHBUFFER_REGIONS:
        at = drive.resolve_chain(gdb, chain)
        if at:
            blobs.append((at, gdb.read(at, size)))
    candidates = []
    for at, blob in blobs:
        candidates.extend(ps3_pose.packet_candidates(blob, base=at))
    print("  %s: lap clock %.3f, world clock %s, %d camera candidate(s)" % (
        stem, pose["race_clock_s"], None if grab_clock is None else grab_clock["world_clock_s"], len(candidates)), flush=True)
    return shot, candidates, pose


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--image", default="/home/topaxi/projects/OpenAntiGrav/data/images/hdfury-ps3-eu-dec.iso")
    ap.add_argument("--shot", type=float, action="append", required=True, metavar="LAP_CLOCK",
                    help="pause at this lap-clock second and keep the frame; repeat, ascending")
    ap.add_argument("--fire-clock", type=float, help="hold SQUARE from the first stop at or past this lap clock")
    ap.add_argument("--fire-button", default="triangle",
                    help="the pad button held for --fire-clock (the button that fires the cannon in the original is measured here, "
                         "not assumed: SQUARE changed the camera view, hiding the hull)")
    ap.add_argument("--fire-hold", type=float, default=0.5, help="wall seconds SQUARE is held (a loaded host drops a short press)")
    ap.add_argument("--reload-clock", type=float, help="tap CIRCLE (reload) at this lap clock")
    ap.add_argument("--steer", metavar="LAP_CLOCK,WALL_HOLD,X", action="append", default=[],
                    help="from the first stop at or past LAP_CLOCK hold the left stick at X (-1..1) for WALL_HOLD wall seconds; repeatable")
    ap.add_argument("--emp-clock", type=float, help="tap L1 (the EMP) at the first stop at or past this lap clock")
    ap.add_argument("--interval", type=float, default=0.12, help="seconds between the two frames of a stop")
    ap.add_argument("--flush", type=float, default=3.0, help="seconds a paused frame is given to be presented")
    ap.add_argument("--team", default="feisar", help="recorded beside the camera")
    ap.add_argument("--pre-wait", type=float, default=30.0,
                    help="wall seconds to wait after the restore before the first START RACE tap: a tap before the prompt "
                         "is up is held over into the race")
    args = ap.parse_args()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    port = int(drive.GDB_SERVER.rsplit(":", 1)[1])
    pair = importlib.util.spec_from_file_location("rpcs3_hd_pair", HERE / "rpcs3-hd-pair.py")
    pair_mod = importlib.util.module_from_spec(pair)
    pair.loader.exec_module(pair_mod)
    pair_mod.set_window(*compare_env.platform("hd-fury")["output"])
    with drive.Session(args.image, str(out / "logs")) as session:
        print("rpcs3 pid %d" % session.proc.pid, flush=True)
        gdb = Debugger(port=port)
        time.sleep(args.pre_wait)
        for _ in range(60):
            gdb.pause()
            try:
                ship, body = place.find_player(gdb)
                clock = race_clock(gdb, ship)
            except RuntimeError:
                clock = 0.0
            gdb.resume()
            if clock > 0.5:
                break
            session.tap("cross", settle=6)
        else:
            sys.exit("the race never started")
        stops = []
        fire = None
        events = [[t, "shot"] for t in args.shot]
        if args.fire_clock is not None:
            events.append([args.fire_clock, "fire"])
        if args.reload_clock is not None:
            events.append([args.reload_clock, "reload"])
        for item in args.steer:
            c, h, x = (float(v) for v in item.split(","))
            events.append([c, "steer", h, x])
        if args.emp_clock is not None:
            events.append([args.emp_clock, "emp"])
        events.sort(key=lambda e: e[0])
        for at, kind, *rest in events:
            now = run_until(gdb, ship, at, kind)
            if kind == "fire":
                before = now
                gdb.resume()
                session.pad.set(args.fire_button, True)
                time.sleep(args.fire_hold)
                session.pad.set(args.fire_button, False)
                gdb.pause()
                after = race_clock(gdb, ship)
                fire = {"press_clock_before_s": before, "press_clock_after_s": after, "hold_wall_s": args.fire_hold, "button": args.fire_button}
                print("fire: lap clock %.3f before, %.3f after the hold" % (before, after), flush=True)
                continue
            if kind == "steer":
                gdb.resume()
                session.pad.stick(rest[1], 0.0)
                time.sleep(rest[0])
                session.pad.stick(0.0, 0.0)
                gdb.pause()
                print("steer %.2f for %.2f wall s from lap clock %.3f to %.3f" % (rest[1], rest[0], now, race_clock(gdb, ship)), flush=True)
                continue
            if kind == "emp":
                gdb.resume()
                session.pad.set("l1", True)
                time.sleep(0.3)
                session.pad.set("l1", False)
                gdb.pause()
                continue
            if kind == "reload":
                gdb.resume()
                session.pad.set("circle", True)
                time.sleep(0.3)
                session.pad.set("circle", False)
                gdb.pause()
                continue
            stem = "%02d" % len(stops)
            first = observe(session, gdb, ship, body, out, stem, args.flush)
            gdb.resume()
            time.sleep(args.interval)
            gdb.pause()
            second = observe(session, gdb, ship, body, out, stem + "b", args.flush)
            stops.append((stem, first, second))
            gdb.resume()
        picks = []
        for stem, first, second in stops:
            picks.append(ps3_pose.pick_camera([first[1], second[1]])[0])
        for (stem, first, _), (camera, reason, count) in zip(stops, picks):
            shot, _, pose = first
            if fire is not None and pose["race_clock_s"] >= fire["press_clock_after_s"]:
                pose["fire"] = fire
            record = drive.describe(camera, reason, count, drive.track_name(), shot, args.team, None, pose)
            (out / ("%s.json" % stem)).write_text(json.dumps(record, indent=2) + "\n")
            print("  %s: camera %s" % (stem, "eye %s" % ["%.1f" % v for v in camera["eye"]] if camera else "null (%s)" % reason), flush=True)
        gdb.resume()


if __name__ == "__main__":
    main()
