#!/usr/bin/env python3
"""Matched frames from a race already running on a `serve` emulator.

`rpcs3-drive.py capture` boots its own session and walks the menus, so it
cannot start from a restored state or a fly-over already on screen. This
attaches (`OAG_RPCS3_ATTACH=1`) to the race as it stands at the `START RACE`
prompt, taps `cross` to skip the fly-over, and then photographs the countdown
grid frame by frame, pausing the target with the GDB stub for each one so the
screenshot and the pushbuffer read describe the same instant. Throttle is held
for the last frames so the camera moves: `ps3_pose.pick_camera` refuses a
camera that is bit-identical across frames (the grid is stationary), and a
moving frame is what makes the stationary one "frame-unique" as well.

Every frame is cropped to one fixed rectangle (`--crop`, default the 1882x1058
picture inside RPCS3's 2000x1200 Xvfb root, offset 118,7) instead of trimmed to
black: a dark frame trims smaller than a bright one, and that padding is what
once invalidated the clipped-white reference band.

    OAG_RPCS3_ATTACH=1 uv run --with evdev python3 scripts/hd-grid-capture.py \\
        --out data/reference/hd-capture/metropia-bloom --team feisar_c1 \\
        --hull-variant concept1
"""
import argparse
import importlib.util
import json
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
_spec = importlib.util.spec_from_file_location("rpcs3_drive", HERE / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(_spec)
sys.modules["rpcs3_drive"] = drive
_spec.loader.exec_module(drive)

import ps3_pose  # noqa: E402
from rpcs3_debugger import Debugger  # noqa: E402


def grab(path, crop):
    result = subprocess.run(
        ["import", "-display", drive.DISPLAY, "-window", "root", "-crop", crop,
         "+repage", str(path)], capture_output=True)
    return path if result.returncode == 0 else None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--crop", default="1882x1058+118+7")
    ap.add_argument("--team", default=None)
    ap.add_argument("--hull-variant", default=None)
    ap.add_argument("--grid-frames", type=int, default=6,
                    help="frames photographed on the grid, before GO")
    ap.add_argument("--grid-gap", type=float, default=0.8)
    ap.add_argument("--first-delay", type=float, default=1.0,
                    help="seconds after the skip tap before the first grid frame")
    ap.add_argument("--drive-frames", type=int, default=3)
    ap.add_argument("--drive-gap", type=float, default=2.0)
    ap.add_argument("--no-skip", action="store_true",
                    help="the fly-over is already skipped; do not tap cross")
    args = ap.parse_args()

    out = Path(args.out).resolve()
    out.mkdir(parents=True, exist_ok=True)
    regions = list(drive.PUSHBUFFER_REGIONS)
    port = int(drive.GDB_SERVER.rsplit(":", 1)[1])
    track = drive.track_name()
    print("track:", track, flush=True)

    shots = []
    with drive.Session("-", str(out / "log")) as session:
        pad = session.pad
        with Debugger(port=port) as gdb:
            if not args.no_skip:
                gdb.resume()
                pad.press("cross", 0.15)
                time.sleep(args.first_delay)
                gdb.pause()
            total = args.grid_frames + args.drive_frames
            for n in range(total):
                driving = n >= args.grid_frames
                if n:
                    if driving and n == args.grid_frames:
                        pad.set("cross", True)
                    gdb.resume()
                    time.sleep(args.drive_gap if driving else args.grid_gap)
                    gdb.pause()
                stem = "%02d" % n
                shot = grab(out / ("%s.png" % stem), args.crop)
                blobs = []
                for chain, size in regions:
                    try:
                        at = drive.resolve_chain(gdb, chain)
                        if at:
                            blobs.append((at, gdb.read(at, size)))
                    except Exception as error:  # noqa: BLE001
                        print("  %s: %s" % (chain, error), flush=True)
                cands = []
                for at, blob in blobs:
                    cands.extend(ps3_pose.packet_candidates(blob, base=at))
                print("  %s: %d candidates%s" % (stem, len(cands),
                      " (throttle)" if driving else " (grid)"), flush=True)
                shots.append((stem, shot, cands))
            pad.set("cross", False)
            gdb.resume()

    picks = ps3_pose.pick_camera([c for _, _, c in shots])
    for (stem, shot, _), (camera, reason, count) in zip(shots, picks):
        record = drive.describe(camera, reason, count, track, shot,
                                args.team, args.hull_variant)
        (out / ("%s.json" % stem)).write_text(json.dumps(record, indent=2) + "\n")
        if camera:
            print("  %s: eye %s" % (stem, ["%.1f" % v for v in camera["eye"]]))
            print("       " + record["camera"].get("render_with", ""), flush=True)
        else:
            print("  %s: camera null (%s)" % (stem, reason), flush=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
