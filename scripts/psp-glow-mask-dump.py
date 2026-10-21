#!/usr/bin/env python3
"""Dump the glow mask (the framebuffer's alpha, the stencil) and the colour of both EDRAM framebuffers of Pulse PSP.

    source data/scratch/<lane>/pp/env.sh
    uv run --with websocket-client python3 scripts/psp-glow-mask-dump.py --port $OAG_PPSSPP_PORT --home $PP_HOME \\
        --state data/saves/pulse-psp/<point>.ppst --out DIR

Loads the state, stops the CPU, and writes `fb<N>-alpha.png` and `fb<N>-rgb.png` (480x272, stride 512) for the two
buffers at EDRAM `0x0` and `0x88000` (docs/rendering/glow-mask.md): the buffer being displayed holds the finished
scene, its alpha the mask the bloom's bright pass reads. Needs ImageMagick.
"""
import argparse
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from ppsspp_debugger import Debugger  # noqa: E402

EDRAM = 0x04000000
BUFFERS = (0x0, 0x88000)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--port", type=int, required=True)
    ap.add_argument("--home", required=True)
    ap.add_argument("--state", required=True)
    ap.add_argument("--out", type=Path, required=True)
    args = ap.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    dbg = Debugger(args.port)
    if dbg.is_stepping():
        dbg.call("cpu.resume")
        time.sleep(1.0)
    subprocess.run([sys.executable, str(HERE / "psp-state.py"), "--port", str(args.port), "--home", args.home,
                    "load", "2", "--file", args.state], check=True)
    dbg.call("cpu.stepping")
    time.sleep(1.0)
    for index, offset in enumerate(BUFFERS):
        raw = dbg.read(EDRAM + offset, 512 * 272 * 4)
        path = args.out / ("fb%d.rgba" % index)
        path.write_bytes(raw)
        base = ["magick", "-size", "512x272", "-depth", "8", "rgba:%s" % path]
        subprocess.run(base + ["-crop", "480x272+0+0", "+repage", "-alpha", "extract", args.out / ("fb%d-alpha.png" % index)], check=True)
        subprocess.run(base + ["-crop", "480x272+0+0", "+repage", "-alpha", "off", args.out / ("fb%d-rgb.png" % index)], check=True)
        print("wrote fb%d" % index)


if __name__ == "__main__":
    main()
