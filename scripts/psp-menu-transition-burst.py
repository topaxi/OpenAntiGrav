#!/usr/bin/env python3
"""Photograph the first frames of a front-end page change, one per presented frame.

Breaks on `Gfx_PresentFrame` (0x0891e1b8) and runs `import` at every hit, the
method `docs/ui/menus-original.md` describes, so a 0.5 s transition yields a
dozen distinct frames instead of two. Starts from `Main Menu`, presses the
given button once and photographs the frames after it.

    uv run --with websocket-client scripts/psp-menu-transition-burst.py \\
        --port 45498 --display :98 --out data/scratch/<lane>/burst-main-grid
"""

import argparse
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ppsspp_debugger import Debugger  # noqa: E402

PRESENT_FRAME = 0x0891E1B8


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--port", type=int, default=47810)
    ap.add_argument("--display", default=":97")
    ap.add_argument("--out", required=True)
    ap.add_argument("--button", default="cross")
    ap.add_argument("--frames", type=int, default=30)
    ap.add_argument("--expect", default=None)
    args = ap.parse_args()
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    dbg = Debugger(args.port)
    start = dbg.state_name()
    print("start state", start)
    dbg.press(args.button, duration=8)
    time.sleep(0.05)
    for index, _ in dbg.each_hit(PRESENT_FRAME, args.frames, timeout=10.0):
        path = out / ("f%02d.png" % index)
        subprocess.run(["import", "-window", "root", str(path)], env={"DISPLAY": args.display}, check=True)
    time.sleep(0.5)
    dbg.resume()
    time.sleep(0.5)
    print("end state", dbg.state_name())
    if args.expect and dbg.state_name() != args.expect:
        print("expected", args.expect, file=sys.stderr)
        raise SystemExit(1)


if __name__ == "__main__":
    main()
