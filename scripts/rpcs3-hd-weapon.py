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

`--teleport-back D` moves the craft D units back along its own forward right after
the shot, so a laid Bomb or Mine stays in front of the chase camera instead of
under it. The recording is RPCS3's own (30 fps, 1280x720), copied to
`<log-dir>/rec.mp4`; the game ran at about 0.6x real time on the measuring host, so
read game time off the HUD clock, not the video's.
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
        session.tap("cross", settle=args.countdown)
        gdb = Debugger(port=port)
        gdb.pause()
        ship, body = place.find_player(gdb)
        slot = struct.unpack(">I", gdb.read(ship + SLOT_OFFSET, 4))[0]
        gdb.resume()
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
