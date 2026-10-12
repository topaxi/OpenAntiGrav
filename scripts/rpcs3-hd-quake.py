#!/usr/bin/env python3
"""Fire a Quake (held-weapon state 2) on an HD race on RPCS3 and film the wave.

State 2 is the Quake: `Weapon_FireFromSlot` (`0x0012d970`)'s state-2 handler sets
slot `+0x200` bit `0x8` and `QuakeManager_Fire` (`0x00147d18`) clears that bit
and arms the Quake object (`docs/ghidra/functions/ps3-hdfury-eu/quake.md`).
Built on `rpcs3-hd-weapon.py`'s recipe, with two differences measured on the
first run: the START RACE prompt ignores a `cross` pressed before it shows, so
the tap is repeated until the race screen changes, and the player is held still
(no throttle) so the wave leaves a craft on the grid.

    OAG_RPCS3_ATTACH=1 uv run --with evdev python3 scripts/emu-run.py -- \\
        python3 -u scripts/rpcs3-hd-quake.py --image data/images/hdfury-ps3-eu-dec.iso \\
        --log-dir <dir> [--prompt-wait 12] [--countdown 12] [--watch 14] [--throttle]

Writes `<log-dir>/rec.mp4` (RPCS3's own recording, 30 fps, 1280x720) and a
screenshot at the moment of the shot. The game runs at about 0.6x real time on a
loaded host, so read game time off the HUD clock, not the video's.
"""
import argparse
import importlib.util
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
QUAKE_STATE = 2


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--image", required=True)
    ap.add_argument("--log-dir", required=True)
    ap.add_argument("--state", type=int, default=QUAKE_STATE)
    ap.add_argument("--prompt-wait", type=float, default=12.0, help="seconds after the load before the first START RACE tap")
    ap.add_argument("--countdown", type=float, default=12.0, help="seconds from the START RACE tap to the shot")
    ap.add_argument("--watch", type=float, default=14.0, help="seconds filmed after the shot")
    ap.add_argument("--throttle", action="store_true", help="hold cross (throttle) from the shot on")
    ap.add_argument("--ahead", type=float, default=0.0, help="units to move the craft forward along its own heading right after the shot, so the wave (270 u/s from 15 ahead) overtakes it")
    ap.add_argument("--button", default="triangle")
    args = ap.parse_args()
    out = Path(args.log_dir)
    out.mkdir(parents=True, exist_ok=True)
    port = int(drive.GDB_SERVER.rsplit(":", 1)[1])
    before = set(drive.recordings())
    with drive.Session(args.image, str(out / "logs")) as session:
        print("rpcs3 pid %d" % session.proc.pid, flush=True)
        if not session.wait_for_screen_pressing("Main Menu", 240):
            sys.exit("never reached the Main Menu")
        session.settle_menu(20)
        if session.walk_to_race() not in drive.RACE_ARRIVED:
            sys.exit("no race")
        session.wait_for_load(70)
        t0 = time.time()
        session.toggle_recording()
        time.sleep(args.prompt_wait)
        print("t+%.0fs screen %s: tapping START RACE" % (time.time() - t0, drive.current_screen()), flush=True)
        session.pad.press("cross", 0.15)
        time.sleep(args.countdown)
        print("t+%.0fs screen %s: shot" % (time.time() - t0, drive.current_screen()), flush=True)
        gdb = Debugger(port=port)
        gdb.pause()
        ship, body = place.find_player(gdb)
        slot = struct.unpack(">I", gdb.read(ship + SLOT_OFFSET, 4))[0]
        gdb.write(slot + STATE, struct.pack(">ii", args.state, args.state))
        gdb.resume()
        time.sleep(0.5)
        drive.screenshot(out / "before-shot.png", trim=True)
        if args.throttle:
            session.pad.set("cross", True)
        session.pad.set(args.button, True)
        time.sleep(0.12)
        session.pad.set(args.button, False)
        shot_t = time.time()
        if args.ahead:
            gdb.pause()
            cur = place.read_pose(gdb, body)
            fwd = cur["rows"][2]
            pos = tuple(cur["pos"][i] + args.ahead * fwd[i] for i in range(3))
            place.write_pose(gdb, body, pos, cur["rows"], 0.0)
            gdb.resume()
        time.sleep(args.watch)
        drive.screenshot(out / "after-watch.png", trim=True)
        if args.throttle:
            session.pad.set("cross", False)
        session.toggle_recording()
        time.sleep(8)
        print("shot at t+%.0fs, film ends t+%.0fs" % (shot_t - t0, time.time() - t0), flush=True)
    fresh = [p for p in drive.recordings() if p not in before]
    for path in fresh:
        shutil.copy(path, out / "rec.mp4")
        print("recording ->", out / "rec.mp4")


if __name__ == "__main__":
    main()
