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
`rpcs3-hd-pair.py --json OUT/NN.json` takes it). `--fire-clock F --fire-hold H` holds a button (`--fire-button`, default TRIANGLE, which fired once; SQUARE changed the camera view on four boots) to fire the pulse cannon
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
#: `*(STAGE_POINTER)` is the Environment stage block (`0x00c7dfb0`); at `+0x32b8` sixteen raw band floats
#: (`SoundSystem_GetBandLevel(b) * gain`) and at `+0x32f8` the sixteen held ones the floor's `zoneTexVis` reads
#: (`docs/ghidra/functions/ps3-hdfury-eu/zone-equaliser.md`).
STAGE_POINTER = 0x008B7940
STAGE_BLOCK = 0x00C7DFB0
RAW_BANDS = 0x32B8
HELD_BANDS = 0x32F8


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


def read_bands(gdb):
    """The stage block's raw and held music bands in one read, paused; refuses a block that moved."""
    block = struct.unpack(">I", gdb.read(STAGE_POINTER, 4))[0]
    if block != STAGE_BLOCK:
        raise RuntimeError("stage block pointer %#x, expected %#x" % (block, STAGE_BLOCK))
    blob = gdb.read(block + RAW_BANDS, HELD_BANDS - RAW_BANDS + 64)
    floats = struct.unpack(">32f", blob[:128])
    return {"block": block, "raw": list(floats[:16]), "held": list(floats[16:]),
            "raw_offset": RAW_BANDS, "held_offset": HELD_BANDS}


def observe(session, gdb, ship, body, out, stem, flush):
    """One paused frame: the screenshot with its world clock, the pushbuffer camera candidates, the pose."""
    time.sleep(flush)
    shot, grab_clock = drive.grab_with_clock(out / ("%s.png" % stem), trim=True)
    pose = place.read_pose(gdb, body)
    pose["race_clock_s"] = race_clock(gdb, ship)
    pose["zone_bands"] = read_bands(gdb)
    try:
        pose["scene_width"] = __import__("hd_scene_width").live(gdb.read)
    except Exception as error:  # noqa: BLE001 - a bonus
        print("  %s: no scene width level (%s)" % (stem, error), flush=True)
    try:
        pose["gantry_seconds"] = rpcs3_hd_field.gantry_seconds(rpcs3_hd_field.Mem(session.open_mem()))
    except Exception as error:  # noqa: BLE001 - a bonus
        print("  %s: no gantry time (%s)" % (stem, error), flush=True)
    try:
        pose["mines"] = rpcs3_hd_field.detonator_mines(rpcs3_hd_field.Mem(session.open_mem()), keep_raw=bool(os.environ.get("OAG_MINE_RAW")))
        print("  %s: %d active mines" % (stem, len(pose["mines"])), flush=True)
    except Exception as error:  # noqa: BLE001 - a bonus
        print("  %s: no mines (%s)" % (stem, error), flush=True)
    pose["scenery_clock"] = (None if grab_clock is None or grab_clock["world_clock_s"] is None else
                             {"time_s": grab_clock["world_clock_s"], "nodes": grab_clock["nodes"], "total": grab_clock["total"]})
    blobs = []
    for chain, size in drive.PUSHBUFFER_REGIONS:
        at = drive.resolve_chain(gdb, chain)
        if at:
            blobs.append((at, gdb.read(at, size)))
    candidates = []
    hulls = []
    for at, blob in blobs:
        candidates.extend(ps3_pose.packet_candidates(blob, base=at))
        hulls.extend(ps3_pose.hull_matrices(blob, base=at))
        (out / ("%s-%08x.bin" % (stem, at))).write_bytes(blob)
    # Where the frame draws the player's hull: the pause's RAM is 1 or 2 whole ticks ahead of the frame, so the pair feeds
    # ours this translation, not the body's (`compare-page.md`, "The skew is the pause, not the original's draw order").
    pose["hull_pb"] = ps3_pose.hull_near(hulls, pose["pos"])
    print("  %s: %d hull-shaped matrices, player's %s" % (stem, len(hulls), None if pose["hull_pb"] is None else
          "x%d at %s" % (pose["hull_pb"]["count"], ["%.2f" % v for v in pose["hull_pb"]["translation"]])), flush=True)
    print("  %s: lap clock %.3f, world clock %s, %d camera candidate(s), held bands %s" % (
        stem, pose["race_clock_s"], None if grab_clock is None else grab_clock["world_clock_s"], len(candidates),
        " ".join("%.2f" % v for v in pose["zone_bands"]["held"])), flush=True)
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
                         "not assumed: SQUARE changed the camera view)")
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
        # A stop whose own second frame was not redrawn (the game ran under a tick between the two reads) has no
        # frame-unique camera of its own: its camera is the value loaded into registers 256 and 260 that no other STOP's
        # frame carries, so a different stop's frame stands in as the discriminator.
        for at, (pick, stop) in enumerate(zip(picks, stops)):
            if pick[0] is None and len(stops) > 1:
                other = stops[(at + 1) % len(stops)]
                retry = ps3_pose.pick_camera([stop[1][1], other[1][1]])[0]
                if retry[0] is not None:
                    picks[at] = retry
        for (stem, first, second), (camera, reason, count) in zip(stops, picks):
            shot, _, pose = first
            # The next stop's bands, `--interval` of wall time later: how far the held levels move between two reads
            # bounds what the presented frame's age (0 to 3 ticks) can change.
            pose["zone_bands_next_stop"] = dict(second[2]["zone_bands"], race_clock_s=second[2]["race_clock_s"])
            if fire is not None and pose["race_clock_s"] >= fire["press_clock_after_s"]:
                pose["fire"] = fire
            record = drive.describe(camera, reason, count, drive.track_name(), shot, args.team, None, pose)
            (out / ("%s.json" % stem)).write_text(json.dumps(record, indent=2) + "\n")
            print("  %s: camera %s" % (stem, "eye %s" % ["%.1f" % v for v in camera["eye"]] if camera else "null (%s)" % reason), flush=True)
        gdb.resume()


if __name__ == "__main__":
    main()
