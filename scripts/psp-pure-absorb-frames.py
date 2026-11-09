#!/usr/bin/env python3
"""Absorb a held pickup on Pure's player craft in a running PPSSPP race and photograph the frames after it.

The Pure counterpart of `psp-absorb-frames.py`, on the addresses of `docs/rendering/weapon-absorb-pairs.md` ("Pure PSP"):
`Ship_UpdateWeapons` (`0x0892935c`, once per game update, `a0` the craft) reads the absorb button as
`*(*(craft + 0xc8) + 0x48) + 0x16` and, with a held id at the weapon record's `+0x1d8` (`*(craft + 0xd0)`), consumes it and
calls the burst (`FUN_08925e20`). This writes the held id with the CPU stopped, holds circle, and counts frames from the
first update whose held id is back at 0 (the absorb). `--control` grants and presses nothing: the no-absorb control taken
from the same state.

    source data/scratch/<lane>/pp/env.sh
    uv run --with websocket-client python3 scripts/psp-pure-absorb-frames.py --port $OAG_PPSSPP_PORT \\
        --display $DISPLAY --state data/saves/pure-psp/<state>.ppst --home $PP_HOME --out data/scratch/<lane>/cap/pure-a1
    ... --control --out data/scratch/<lane>/cap/pure-control

Each frame is a 480x272 crop of the instance's window (set to its native size), taken with the CPU stopped on the update;
the displayed frame lags the stop by about two updates (the page's lag). `frames.jsonl` holds the update count and the
craft's clock step per frame.
"""

import argparse
import json
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from ppsspp_debugger import Debugger  # noqa: E402

SHIP_UPDATE_WEAPONS = 0x0892935C
SERIAL = "UCUS98612_2.00"
HELD_OFFSET = 0x1D8
RECORD_OFFSET = 0xD0
CONTROLLER_OFFSET = 0xC8
BUTTONS_OFFSET = 0x48
ABSORB_BYTE = 0x16
FRAME_DT = 0x1C8


def xdo(*args):
    return subprocess.run(["xdotool", *args], capture_output=True, text=True, check=True).stdout


def window():
    best = None
    for wid in xdo("search", "--onlyvisible", "--name", "").split():
        geo = xdo("getwindowgeometry", "--shell", wid)
        v = dict(line.split("=") for line in geo.split() if "=" in line)
        x, y, w, h = (int(v[k]) for k in ("X", "Y", "WIDTH", "HEIGHT"))
        if (w, h) in ((960, 544), (480, 272)):
            best = (wid, x, y, w, h)
    if best is None:
        sys.exit("no PPSSPP window of 960x544 or 480x272")
    return best


def native_window():
    wid, x, y, w, h = window()
    if (w, h) != (480, 272):
        xdo("windowsize", wid, "480", "272")
        time.sleep(2.0)
        wid, x, y, w, h = window()
    return x, y


def shot(display, origin, path):
    x, y = origin
    subprocess.run(["import", "-window", "root", "-crop", "480x272+%d+%d" % (x, y), "+repage", str(path)],
                   env={"DISPLAY": display, "PATH": "/usr/bin:/bin"}, check=True)


def a0(dbg):
    for category in dbg.call("cpu.getAllRegs")["categories"]:
        if category["name"] == "GPR":
            return dict(zip(category["registerNames"], category["uintValues"]))["a0"]
    raise RuntimeError("no GPR category")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--port", type=int, required=True)
    ap.add_argument("--display", required=True)
    ap.add_argument("--home", required=True)
    ap.add_argument("--state", required=True)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--frames", type=int, default=40, help="updates photographed after the absorb (or after update 3 for a control)")
    ap.add_argument("--held", type=int, default=1)
    ap.add_argument("--control", action="store_true")
    ap.add_argument("--lead", type=int, default=3, help="updates run before the grant")
    args = ap.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    uv = ["uv", "run", "--with", "websocket-client", "python3"]
    dbg0 = Debugger(args.port)
    if dbg0.is_stepping():
        dbg0.call("cpu.resume")
        time.sleep(1.0)
    dbg0.close()
    native_window()
    subprocess.run([*uv, str(HERE / "psp-state.py"), "--port", str(args.port), "--home", args.home, "load", "2",
                    "--file", args.state, "--serial", SERIAL], check=True,
                   env={**__import__("os").environ, "DISPLAY": args.display})
    origin = native_window()
    dbg = Debugger(args.port)
    log = open(args.out / "frames.jsonl", "w")
    granted = None
    zero_at = None
    written = 0
    try:
        for hit, _ in dbg.each_hit(SHIP_UPDATE_WEAPONS, 100000, timeout=60.0):
            craft = a0(dbg)
            holder = dbg.read_u32(craft + RECORD_OFFSET)
            held = dbg.read_u32(holder + HELD_OFFSET)
            dt = dbg.read_f32(craft + FRAME_DT)
            if args.control:
                if hit < args.lead:
                    continue
                zero_at = zero_at if zero_at is not None else hit
            elif granted is None:
                if hit < args.lead:
                    continue
                dbg.write_u32(holder + HELD_OFFSET, args.held)
                dbg.hold(circle=True)
                granted = hit
                print("granted %d at update %d (craft %#x record %#x)" % (args.held, hit, craft, holder), file=sys.stderr)
                continue
            elif zero_at is None:
                if held == 0:
                    zero_at = hit
                    dbg.hold(circle=False)
                elif hit - granted > 30:
                    sys.exit("held id never came back to 0 (%d): no absorb" % held)
                else:
                    continue
            index = hit - zero_at
            name = "f%02d" % index
            shot(args.display, origin, args.out / (name + ".png"))
            log.write(json.dumps({"frame": index, "hit": hit, "held": held, "dt": dt}) + "\n")
            log.flush()
            written += 1
            if written >= args.frames:
                break
    finally:
        dbg.hold(circle=False)
        dbg.resume()
        dbg.close()
        log.close()
    print("wrote %d frames to %s" % (written, args.out), file=sys.stderr)


if __name__ == "__main__":
    main()
