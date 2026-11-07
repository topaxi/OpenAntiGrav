#!/usr/bin/env python3
"""Give the player an HD weapon in a running race on RPCS3, fire it, film it.

Measured 2026-10-07 (`docs/reverse-engineering/rpcs3-capture.md`, "Giving the
player a weapon"): the held weapon is the state word of the craft's pickup slot,
`*(ship + 0x5edc) + 0x204`, mirrored at `+0x208` (write both; `-1` is empty).
Fire is TRIANGLE, absorb CIRCLE. States 11 and up crash the game.

    OAG_RPCS3_DISPLAY=94 OAG_RPCS3_GDB=127.0.0.1:2354 \\
    OAG_RPCS3_PAD_NAME="OAG Pad <lane>" XDG_CONFIG_HOME=... XDG_CACHE_HOME=... \\
        uv run --with evdev python3 scripts/rpcs3-hd-weapon.py \\
        --image data/images/hdfury-ps3-eu-dec.iso --log-dir <dir> \\
        --state 9 --teleport-back 30 --watch 10

`--shot-at T,T,...` replaces the film with paused frames: each T is game seconds (the craft's race
clock, not the wall clock) after the shot, the target is paused, the frame grabbed and the camera read from the
RSX pushbuffer together with the player's pose, and `<log-dir>/NN.json` is written the way `rpcs3-drive.py
capture` writes it (`render_with` carries the `--camera-pose` numbers for ours). The record also holds the race
clock at the press (`fire_clock_before_s`/`_after_s`), so ours can be run the same number of ticks after its
own shot. Attached (`OAG_RPCS3_ATTACH=1`, a `serve` already in a race) the walk is skipped and the shot repeats
on the same race, so a second weapon costs a minute, not a boot.

`--teleport-back D` moves the craft D units back along its own forward right after
the shot, so a laid Bomb or Mine stays in front of the chase camera instead of
under it. The recording is RPCS3's own (30 fps, 1280x720), copied to
`<log-dir>/rec.mp4`; the game ran at about 0.6x real time on the measuring host, so
read game time off the HUD clock, not the video's.
"""
import argparse
import importlib.util
import json
import os
import shutil
import struct
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
_spec = importlib.util.spec_from_file_location("rpcs3_drive", HERE / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(drive)
import rpcs3_place as place  # noqa: E402
from rpcs3_debugger import Debugger  # noqa: E402

SLOT_OFFSET = 0x5EDC
STATE = 0x204
CLOCK = 0x7014


def race_clock(gdb, ship):
    return struct.unpack(">f", gdb.read(ship + CLOCK, 4))[0]


def shoot_and_photograph(session, gdb, ship, body, slot, args, out):
    """The `--shot-at` mode: fire, then pause at each game time and keep frame, camera and pose."""
    import ps3_pose
    import compare_env
    pair = importlib.util.spec_from_file_location("rpcs3_hd_pair", HERE / "rpcs3-hd-pair.py")
    pair_mod = importlib.util.module_from_spec(pair)
    pair.loader.exec_module(pair_mod)
    pair_mod.set_window(*compare_env.platform("hd-fury")["output"])
    times = sorted(float(t) for t in args.shot_at.split(","))
    for _ in range(100):
        gdb.pause()
        if race_clock(gdb, ship) >= args.fire_after:
            break
        gdb.resume()
        time.sleep(0.5)
    gdb.pause()
    gdb.write(slot + STATE, struct.pack(">ii", args.state, args.state))
    before = race_clock(gdb, ship)
    gdb.resume()
    time.sleep(1.0)
    session.pad.set(args.button, True)
    time.sleep(args.hold)
    session.pad.set(args.button, False)
    gdb.pause()
    after = race_clock(gdb, ship)
    if args.teleport_back:
        cur = place.read_pose(gdb, body)
        fwd = cur["rows"][2]
        pos = tuple(cur["pos"][i] - args.teleport_back * fwd[i] for i in range(3))
        place.write_pose(gdb, body, pos, cur["rows"], 0.0)
    print("shot: race clock %.3f before the press, %.3f after" % (before, after), flush=True)
    shots = []
    for n, at in enumerate(times):
        # the shot happened between `before` and `after`; the frame is wanted `at` seconds past `after`
        for _ in range(200):
            now = race_clock(gdb, ship)
            if now >= after + at:
                break
            gdb.resume()
            time.sleep(max(0.05, min(0.5, (after + at - now) / 0.6 - 0.1)))
            gdb.pause()
        else:
            sys.exit("the race clock stopped at %.3f (a race still on its START RACE prompt?)" % now)
        time.sleep(args.flush)
        stem = "%02d" % n
        shot = drive.screenshot(out / ("%s.png" % stem), trim=True)
        pose = place.read_pose(gdb, body)
        pose["race_clock_s"] = race_clock(gdb, ship)
        pose["fire_clock_before_s"] = before
        pose["fire_clock_after_s"] = after
        import rpcs3_hd_field
        pose["field"] = rpcs3_hd_field.field_rows(rpcs3_hd_field.Mem(session.open_mem()))
        blobs = []
        for chain, size in drive.PUSHBUFFER_REGIONS:
            at_addr = drive.resolve_chain(gdb, chain)
            if at_addr:
                blobs.append((at_addr, gdb.read(at_addr, size)))
        candidates = []
        for addr, blob in blobs:
            candidates.extend(ps3_pose.packet_candidates(blob, base=addr))
        print("  %s: clock %.3f, %d camera candidate(s)" % (stem, pose["race_clock_s"], len(candidates)), flush=True)
        shots.append((stem, shot, candidates, pose))
    gdb.resume()
    picks = ps3_pose.pick_camera([c for _, _, c, _ in shots])
    for (stem, shot, _, pose), (camera, reason, count) in zip(shots, picks):
        record = drive.describe(camera, reason, count, drive.track_name(), shot, None, None, pose)
        (out / ("%s.json" % stem)).write_text(json.dumps(record, indent=2) + "\n")
        print("  %s: camera %s" % (stem, "eye %s" % ["%.1f" % v for v in camera["eye"]] if camera else "null (%s)" % reason), flush=True)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--image", required=True)
    ap.add_argument("--log-dir", required=True)
    ap.add_argument("--state", type=int, required=True, help="0..10; see the doc")
    ap.add_argument("--button", default="triangle")
    ap.add_argument("--teleport-back", type=float, default=0.0)
    ap.add_argument("--watch", type=float, default=10.0, help="seconds filmed after the shot")
    ap.add_argument("--no-record", action="store_true")
    ap.add_argument("--countdown", type=float, default=22.0)
    ap.add_argument("--shot-at", default="", help="game seconds after the shot to pause and photograph, comma separated")
    ap.add_argument("--restart", action="store_true",
                    help="attached: Restart Race from the pause menu first, so the shot is on a fresh grid (the lap clock "
                         "of a race left running reads minutes, and ours would have to run as many ticks)")
    ap.add_argument("--hold", type=float, default=0.5, help="seconds the button is held in the paused-frame mode; a loaded host drops a short press")
    ap.add_argument("--fire-after", type=float, default=3.0,
                    help="lap-clock seconds to let pass before the shot: a press in the first second after GO fired nothing")
    ap.add_argument("--flush", type=float, default=3.0, help="seconds a paused frame is given to be presented")
    args = ap.parse_args()
    out = Path(args.log_dir)
    out.mkdir(parents=True, exist_ok=True)
    port = int(drive.GDB_SERVER.rsplit(":", 1)[1])
    before = set(drive.recordings())
    with drive.Session(args.image, str(out / "logs")) as session:
        print("rpcs3 pid %d" % session.proc.pid, flush=True)
        attached = os.environ.get("OAG_RPCS3_ATTACH") == "1" and (session.in_race() or drive.current_screen() == "InGame")
        if not attached:
            if not session.wait_for_screen_pressing("Main Menu", 240):
                sys.exit("never reached the Main Menu")
            session.settle_menu(20)
            if session.walk_to_race(max_presses=30) not in drive.RACE_ARRIVED:
                sys.exit("no race")
            session.wait_for_load(70)
        if args.restart and attached:
            session.restart_race()
            time.sleep(5)
        gdb = Debugger(port=port)
        for _ in range(60):
            # START RACE is tapped until the lap clock runs: a loaded host drops presses and loads slowly
            gdb.pause()
            try:
                ship, body = place.find_player(gdb)
                clock = race_clock(gdb, ship)
                started = clock > 0.5 and not (args.restart and clock > 20.0)
            except RuntimeError:
                clock, started = 0.0, False
            gdb.resume()
            if started:
                break
            # with --restart a clock past 20 s is the race being left; only a grid or fly-over (clock 0) wants the START RACE press
            session.tap("cross", settle=6) if clock < 0.5 else time.sleep(3)
        else:
            sys.exit("the race never started")
        gdb.pause()
        slot = struct.unpack(">I", gdb.read(ship + SLOT_OFFSET, 4))[0]
        gdb.resume()
        if args.shot_at:
            shoot_and_photograph(session, gdb, ship, body, slot, args, out)
            return
        if not args.no_record:
            session.toggle_recording()
        time.sleep(2)
        gdb.pause()
        gdb.write(slot + STATE, struct.pack(">ii", args.state, args.state))
        gdb.resume()
        time.sleep(0.5)
        session.pad.set(args.button, True)
        time.sleep(0.12)
        session.pad.set(args.button, False)
        if args.teleport_back:
            gdb.pause()
            cur = place.read_pose(gdb, body)
            fwd = cur["rows"][2]
            pos = tuple(cur["pos"][i] - args.teleport_back * fwd[i] for i in range(3))
            place.write_pose(gdb, body, pos, cur["rows"], 0.0)
            gdb.resume()
        time.sleep(args.watch)
        if not args.no_record:
            session.toggle_recording()
            time.sleep(8)
    fresh = [p for p in drive.recordings() if p not in before]
    for path in fresh:
        shutil.copy(path, out / "rec.mp4")
        print("recording ->", out / "rec.mp4")


if __name__ == "__main__":
    main()
