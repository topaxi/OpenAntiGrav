#!/usr/bin/env python3
"""Film the trail on an attached RPCS3 race: hold throttle, grab N frames.

    OAG_RPCS3_ATTACH=1 uv run --with evdev python3 scripts/rpcs3-hd-trail-film.py OUT_DIR \
        [--hold 6] [--frames 10] [--gap 0.25] [--steer left|right] [--airbrake l1|r1]

The restored race must be running (Time Trial start state: throttle at once).
Frames are root-window grabs of the virtual display, `trail-NN.png`.
"""
import argparse
import importlib.util
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("rpcs3_drive", HERE / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(spec)
spec.loader.exec_module(drive)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("out")
    ap.add_argument("--hold", type=float, default=6.0)
    ap.add_argument("--frames", type=int, default=10)
    ap.add_argument("--gap", type=float, default=0.25)
    ap.add_argument("--steer", choices=["left", "right"])
    ap.add_argument("--airbrake", choices=["l1", "r1"])
    a = ap.parse_args()
    out = Path(a.out)
    out.mkdir(parents=True, exist_ok=True)
    pad = drive.RemotePad(drive.read_session()["socket"])
    pad.set("cross", True)
    time.sleep(a.hold)
    if a.steer:
        pad.set(a.steer, True)
    if a.airbrake:
        pad.set(a.airbrake, True)
    try:
        for i in range(a.frames):
            drive.screenshot(out / ("trail-%02d.png" % i))
            time.sleep(a.gap)
    finally:
        for b in ("cross", "left", "right", "l1", "r1"):
            pad.set(b, False)


if __name__ == "__main__":
    sys.exit(main())
