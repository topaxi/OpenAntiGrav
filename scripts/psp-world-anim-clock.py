#!/usr/bin/env python3
"""The time a Pulse world mesh's texture animation is fed, read against the race clock.

    source data/scratch/<lane>/pp/env.sh
    DISPLAY=$DISPLAY uv run --with websocket-client python3 scripts/psp-world-anim-clock.py \\
        --port $OAG_PPSSPP_PORT --home $PP_HOME --state data/saves/pulse-psp/<point>.ppst [--hits 60]

Loads the state, stops the CPU straight after the load, and breaks `Mesh_UpdateTextureTransforms`
(`0x0890e160`, a0 = mesh) for the next hits. At each hit it prints the mesh's own time (`mesh+0x40`) and the
race clock (`0x08B344B0`), so the offset between them (the countdown, if the mesh clock starts at the race's
start rather than at GO) can be read straight off. See docs/ghidra/functions/psp-pulse-usa/texture-animation.md.
"""

import argparse
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from ppsspp_debugger import Debugger  # noqa: E402

UPDATE_TEXTURE_TRANSFORMS = 0x0890E160
CLOCK_ADDRESS = 0x08B344B0


def regs(dbg):
    r = dbg.call("cpu.getAllRegs")
    gpr = next(c for c in r["categories"] if c["name"] == "GPR")
    return dict(zip(gpr["registerNames"], gpr["uintValues"]))


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--port", type=int, required=True)
    ap.add_argument("--home", required=True)
    ap.add_argument("--state", required=True)
    ap.add_argument("--hits", type=int, default=40)
    args = ap.parse_args()
    dbg = Debugger(args.port)
    if dbg.is_stepping():
        dbg.call("cpu.resume")
        time.sleep(1.0)
    subprocess.run([sys.executable, str(HERE / "psp-state.py"), "--port", str(args.port), "--home", args.home,
                    "load", "2", "--file", args.state], check=True)
    dbg.call("cpu.stepping")
    time.sleep(1.0)
    print("clock at the stop: %.4f" % dbg.read_f32(CLOCK_ADDRESS))
    rows = []
    for index, _ in dbg.each_hit(UPDATE_TEXTURE_TRANSFORMS, args.hits, timeout=20.0):
        mesh = regs(dbg)["a0"]
        rows.append((mesh, dbg.read_f32(mesh + 0x40), dbg.read_f32(mesh + 0x18C), dbg.read_f32(CLOCK_ADDRESS)))
    print("mesh        mesh+0x40     mesh+0x18c    race clock  mesh time - clock")
    for mesh, t, cached, clock in rows:
        print("0x%08x  %12.4f  %12.4f  %10.4f  %10.4f" % (mesh, t, cached, clock, t - clock))


if __name__ == "__main__":
    main()
