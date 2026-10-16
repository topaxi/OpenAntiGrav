#!/usr/bin/env python3
"""Sample what HD's `ThrustBar` shows: the HUD record's `+0x8` beside the craft.

`FUN_00086e88` crops `ThrustBar` to `(*(hud+0x40))[+8] * 0.01` of its width and
prints `(int)[+8]` and `%` in `ThrustBarText`. This finds that record the way
`rpcs3-hud-probe.py` does, then samples `record+0x0` and `record+0x8` together
with the local craft's throttle cell (`ship+0x30c`) and its speed, idle on the
grid, then with cross held. One row per sample, as JSON.

    OAG_RPCS3_* from scripts/emu-env.sh, then
    uv run --with evdev python3 scripts/rpcs3-hd-thrust-bar.py --image <iso> --out <dir>
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


def load(name, file):
    spec = importlib.util.spec_from_file_location(name, HERE / file)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


probe = load("probe", "rpcs3-hud-probe.py")
drive = probe.drive
Debugger = probe.Debugger


def sample(gdb, cells, t0, label):
    gdb.pause()
    gdb.drain(0.2)
    rec = gdb.read(cells.record, 0x60)
    f = lambda b, o: struct.unpack(">f", b[o:o + 4])[0]  # noqa: E731
    row = {
        "t": round(time.time() - t0, 2),
        "phase": label,
        "rec_0": f(rec, 0x0),
        "rec_4": f(rec, 0x4),
        "rec_8": f(rec, 0x8),
        "throttle_30c": probe.f32r(gdb, cells.ship + 0x30C),
        "state_2f8": probe.u32(gdb, cells.ship + 0x2F8),
    }
    gdb.resume()
    return row


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--image", default="data/images/hdfury-ps3-eu-dec.iso")
    ap.add_argument("--out", required=True)
    ap.add_argument("--seconds", type=float, default=25.0)
    ap.add_argument("--record-window", default="0x307c0000,0x30844000")
    ap.add_argument("--hud-window", default="0x32800000,0x32a40000")
    args = ap.parse_args()
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    window = lambda t: tuple(int(x, 16) for x in t.split(","))  # noqa: E731
    rows = []
    with drive.Session(args.image, out) as session:
        probe.boot_race(session, settle=1.0)
        with Debugger(port=int(os.environ.get("OAG_RPCS3_GDB", "127.0.0.1:%d" % probe.PORT).rsplit(":", 1)[1])) as gdb:
            cells = probe.Cells(gdb, window(args.record_window), window(args.hud_window))
            print(json.dumps({k: hex(v) for k, v in vars(cells).items() if k != "slots"}))
            gdb.resume()
            t0 = time.time()
            for _ in range(6):
                rows.append(sample(gdb, cells, t0, "idle"))
                time.sleep(0.5)
            session.pad.set("cross", True)
            end = time.time() + args.seconds
            while time.time() < end:
                rows.append(sample(gdb, cells, t0, "thrust"))
                time.sleep(0.8)
            session.pad.set("cross", False)
            for _ in range(8):
                rows.append(sample(gdb, cells, t0, "coast"))
                time.sleep(0.8)
    (out / "thrust.json").write_text(json.dumps(rows, indent=1))
    for r in rows:
        print(r)


if __name__ == "__main__":
    main()
