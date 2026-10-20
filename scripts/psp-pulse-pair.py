#!/usr/bin/env python3
"""One original-vs-ours pair of Wipeout Pulse (PSP) from a race the emulator is in.

    DISPLAY=127.0.0.1:97 OAG_SHOT_DISPLAY=127.0.0.1:97 HOME=<instance HOME> \\
    uv run --with websocket-client python3 scripts/psp-pulse-pair.py \\
        --port 47897 --out data/scratch/<lane>/moa --track 03_Track \\
        [--reversed] [--state data/saves/pulse-psp/<point>.ppst] [--team Feisar]

Optionally restores a saved state first (`psp-state.py load`), waits for the
"State loaded" toast to fade, captures four ticks of ship and camera pose with
one window shot (`psp-trace.py --camera --shot-every 2`), crops the 960x544
window out of the Xvfb screen, and renders our frame from the same row
(`oag-game --race --pose-from`). Writes `theirs.png`, `ours.png`, `side.png` and
`trace.csv` under `--out`. The pair is only a pair when the circuit and team on
both sides are the same: check `side.png` by eye, because a wrong `--track`
renders a plausible-looking wrong circuit around the right pose.
"""

import argparse
import os
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent


def run(cmd, **kw):
    print("+", " ".join(str(c) for c in cmd), file=sys.stderr)
    return subprocess.run([str(c) for c in cmd], check=True, **kw)


def window_origin():
    out = subprocess.run(["xdotool", "search", "--onlyvisible", "--name", ""],
                         capture_output=True, text=True).stdout.split()
    for wid in out:
        geo = subprocess.run(["xdotool", "getwindowgeometry", "--shell", wid],
                             capture_output=True, text=True).stdout
        vals = dict(line.split("=") for line in geo.split() if "=" in line)
        if vals.get("WIDTH") == "960" and vals.get("HEIGHT") == "544":
            return int(vals["X"]), int(vals["Y"])
    return 0, 0


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--port", type=int, default=int(os.environ.get("OAG_PPSSPP_PORT", 47810)))
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--track", required=True, help="circuit directory, e.g. 03_Track")
    ap.add_argument("--reversed", action="store_true", help="the track_reversed layout")
    ap.add_argument("--team", help="our --team, when the original's is not the default")
    ap.add_argument("--state", help="a .ppst to restore into slot 2 first")
    ap.add_argument("--home", default=os.environ.get("PP_HOME"),
                    help="the instance's own HOME (`PP_HOME` from ppsspp-start.sh's env.sh); "
                    "the state is copied into its memstick, so there is no default to fall back on")
    ap.add_argument("--image", default=str(ROOT / "data/images/pulse-psp-usa.chd"))
    ap.add_argument("--toast-wait", type=float, default=6.0)
    args = ap.parse_args()
    if "OAG_SHOT_DISPLAY" not in os.environ or "DISPLAY" not in os.environ:
        sys.exit("DISPLAY and OAG_SHOT_DISPLAY must name the instance's own Xvfb")
    if args.state and not args.home:
        sys.exit("--home (or PP_HOME) must name the instance's own HOME: the state is copied "
                 "into <HOME>/.config/ppsspp, and a default would be the maintainer's own")
    if not os.environ.get("XDG_CONFIG_HOME", "").startswith(str(ROOT.parent.parent / "OpenAntiGrav" / "data" / "scratch")):
        sys.exit("XDG_CONFIG_HOME must be a directory under data/scratch/<lane>/: oag-game "
                 "persists settings and progress there, never in the maintainer's profile")
    args.out.mkdir(parents=True, exist_ok=True)
    uv = ["uv", "run", "--with", "websocket-client", "python3"]
    if args.state:
        run(uv + [HERE / "psp-state.py", "--port", args.port, "--home", args.home,
                         "load", "2", "--file", args.state])
        time.sleep(args.toast_wait)
    shots = args.out / "shots"
    csv = args.out / "trace.csv"
    with open(csv, "w") as f:
        run(uv + [HERE / "psp-trace.py", "--port", args.port, "--ticks", 4, "--camera",
                  "--shot-every", 2, "--shot-dir", shots], stdout=f)
    ox, oy = window_origin()
    run(["magick", shots / "tick00000.png", "-crop", "960x544+%d+%d" % (ox, oy), "+repage",
         args.out / "theirs.png"])
    entry = "Data\\Environments\\%s\\track%s.vex" % (args.track, "_reversed" if args.reversed else "")
    cmd = [ROOT / "target/debug/oag-game", args.image, "--no-audio", "--race", "--track", entry,
           "--screenshot", args.out / "ours.png", "--ticks", 0, "--size", "960x544",
           "--pose-from", csv, "--pose-tick", 0]
    if args.team:
        cmd += ["--team", args.team]
    run(cmd, stdout=subprocess.DEVNULL)
    run(["magick", "montage", args.out / "theirs.png", args.out / "ours.png", "-tile", "2x1",
         "-geometry", "+2+2", args.out / "side.png"])
    print("wrote", args.out / "side.png")


if __name__ == "__main__":
    main()
