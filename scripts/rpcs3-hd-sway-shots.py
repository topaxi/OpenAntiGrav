#!/usr/bin/env python3
"""Frames of HD's race HUD at held stick plateaus, paired with `ship+0x6a54/58/5c` (RPCS3).

Attach to a running `rpcs3-drive.py serve` in a race after GO. For each `--stick` value: cross and
the stick held `--hold` seconds (the lean follower settles), the game paused over the GDB stub, the
float quad read, a `--flush` second wait for the RSX to present the frames it was handed (the
presented frame lags RAM otherwise, `rpcs3-hd-pair.py`), a root screenshot, resume. Player input
only. The window is sized to 1280x720 first so the frame is the render target 1:1.
Writes `<out>/s<i>.png`, `<out>/shots.csv` (stick, 6a54, 6a58, 6a5c, 6a44, 6a48) and `<out>/blocks.bin`
(per frame: ship+0x6800..0x6c00 then body+0..0x600, for an offline correlation).
"""
import argparse
import importlib.util
import os
import struct
import subprocess
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parent
sys.path.insert(0, str(REPO))
_spec = importlib.util.spec_from_file_location("rpcs3_drive", REPO / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(drive)
import rpcs3_place as place  # noqa: E402
from rpcs3_debugger import Debugger  # noqa: E402

GUEST = 0x300000000


def set_window(w, h):
    display = os.environ["OAG_RPCS3_DISPLAY"]
    env = dict(os.environ, DISPLAY="127.0.0.1:%s" % display)
    wins = subprocess.run(["xdotool", "search", "--onlyvisible", "--name", ""], capture_output=True,
                          text=True, env=env).stdout.split()
    cands = []
    for wid in wins:
        geo = subprocess.run(["xdotool", "getwindowgeometry", "--shell", wid], capture_output=True,
                             text=True, env=env).stdout
        v = dict(line.split("=") for line in geo.split() if "=" in line)
        cands.append((int(v["WIDTH"]) * int(v["HEIGHT"]), wid, int(v["WIDTH"]), int(v["HEIGHT"])))
    cands.sort()
    emu = cands[-2] if len(cands) > 1 else cands[-1]
    if (emu[2], emu[3]) != (w, h):
        subprocess.run(["xdotool", "windowsize", emu[1], str(w), str(h)], env=env, check=True)
        time.sleep(3.0)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--image", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--stick", default="0,1,-1,0.5,-0.5,0")
    ap.add_argument("--hold", type=float, default=3.0)
    ap.add_argument("--flush", type=float, default=3.0)
    ap.add_argument("--no-cross", action="store_true")
    args = ap.parse_args()
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    port = int(drive.GDB_SERVER.rsplit(":", 1)[1])
    with drive.Session(args.image, str(out / "logs")) as session:
        set_window(1280, 720)
        gdb = Debugger(port=port)
        gdb.pause()
        ship, body = place.find_player(gdb)
        blocks = []
        gdb.resume()
        rows = ["i,stick,6a54,6a58,6a5c,6a44,6a48"]
        if not args.no_cross:
            session.pad.set("cross", True)
        for i, s in enumerate(float(v) for v in args.stick.split(",")):
            session.pad.stick(s, 0.0)
            time.sleep(args.hold)
            gdb.pause()
            vals = [struct.unpack(">f", gdb.read(ship + o, 4))[0] for o in (0x6A54, 0x6A58, 0x6A5C, 0x6A44, 0x6A48)]
            blocks.append(gdb.read(ship + 0x6800, 0x400) + gdb.read(body, 0x600))
            time.sleep(args.flush)
            drive.screenshot(out / ("s%d.png" % i), trim=True)
            gdb.resume()
            rows.append("%d,%.2f," % (i, s) + ",".join("%.5f" % v for v in vals))
            print(rows[-1], flush=True)
        session.pad.stick(0.0, 0.0)
        session.pad.set("cross", False)
        (out / "blocks.bin").write_bytes(b"".join(blocks))
        (out / "shots.csv").write_text("\n".join(rows) + "\n")


if __name__ == "__main__":
    main()
