#!/usr/bin/env python3
"""Walk HD's front end on a long-lived RPCS3 and photograph each screen at 1920x1080.

    source data/scratch/<lane>/env.sh        # the lane's own display, pad, config
    OAG_COMPARE_PLATFORM=hd-fury ... rpcs3-drive.py serve     # once, waits for Main Menu
    OAG_RPCS3_ATTACH=1 uv run --with evdev python3 scripts/rpcs3-hd-menu-walk.py \\
        --out data/scratch/<lane>/orig --plan "shot:main,right,cross,shot:racebox"

`--plan` is a comma list. A button name (`cross`, `circle`, `right`, ...) taps it and waits
`--settle` seconds, `wait:SECONDS` sleeps, and `shot:NAME` grabs the emulator window as
`NAME.png` (1920x1080, the profile's output) and records the screen HD last said it switched to
(`TTY.log`, `Switching Screen "A" to "B"`), so a pair can refuse when the screen names differ.
`steps.json` in `--out` lists every step: button, screen before and after, seconds, shot file.

The window is put at the origin at 1920x1080 first (the default 1760x1072 carries black bars,
`rpcs3-hd-pair.py set_window`). Grab no sooner than `--settle` after a transition: HD names a
screen at the start of its animation (`rpcs3-drive.py` `SCREEN_SETTLE`).
"""
import argparse
import importlib
import json
import os
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
drive = importlib.import_module("rpcs3-drive")


def window(w, h):
    """Resize and move the emulator's window on this lane's Xvfb; `(x, y, w, h)` afterwards."""
    display = os.environ.get("OAG_RPCS3_DISPLAY")
    if not display:
        sys.exit("OAG_RPCS3_DISPLAY must name the instance's own Xvfb (source its env.sh)")
    env = dict(os.environ, DISPLAY="127.0.0.1:%s" % display)

    def geometry(wid):
        geo = subprocess.run(["xdotool", "getwindowgeometry", "--shell", wid], capture_output=True,
                             text=True, env=env).stdout
        v = dict(line.split("=") for line in geo.split() if "=" in line)
        return int(v["WIDTH"]), int(v["HEIGHT"])

    wins = subprocess.run(["xdotool", "search", "--onlyvisible", "--name", ""], capture_output=True,
                          text=True, env=env).stdout.split()
    cands = sorted((geometry(i)[0] * geometry(i)[1], i) for i in wins)
    emu = cands[-2][1] if len(cands) > 1 else cands[-1][1]
    if geometry(emu) != (w, h):
        subprocess.run(["xdotool", "windowsize", emu, str(w), str(h)], env=env, check=True)
        for _ in range(60):
            if geometry(emu) == (w, h):
                break
            time.sleep(1.0)
        time.sleep(3.0)
    subprocess.run(["xdotool", "windowmove", emu, "0", "0"], env=env, check=True)
    time.sleep(1.0)
    return geometry(emu)


def grab(path, w, h):
    display = os.environ["OAG_RPCS3_DISPLAY"]
    subprocess.run(["import", "-display", "127.0.0.1:%s" % display, "-window", "root",
                    "-crop", "%dx%d+0+0" % (w, h), "+repage", str(path)], check=True)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--plan", required=True)
    ap.add_argument("--settle", type=float, default=3.0, help="seconds after a tap before the next step")
    ap.add_argument("--size", default="1920x1080")
    args = ap.parse_args()
    w, h = (int(v) for v in args.size.split("x"))
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    info = drive.live_session()
    if info is None:
        sys.exit("no live `rpcs3-drive.py serve` (set OAG_RPCS3_ATTACH=1 and the lane's XDG_CACHE_HOME)")
    pad = drive.RemotePad(info["socket"])
    got = window(w, h)
    steps = []
    print("window", got, flush=True)
    for item in filter(None, args.plan.split(",")):
        before = drive.current_screen()
        step = {"step": item, "screen_before": before}
        kind, _, rest = item.partition(":")
        if kind == "shot":
            time.sleep(0.5)
            grab(out / (rest + ".png"), w, h)
            step["file"] = rest + ".png"
        elif kind == "wait":
            time.sleep(float(rest))
        else:
            pad.press(kind, 0.15)
            time.sleep(float(rest) if rest else args.settle)
        step["screen_after"] = drive.current_screen()
        step["at"] = time.strftime("%Y-%m-%dT%H:%M:%S")
        steps.append(step)
        print("%-14s %-24s -> %s" % (item, before, step["screen_after"]), flush=True)
    (out / "steps.json").write_text(json.dumps({"size": [w, h], "window": list(got), "steps": steps}, indent=1))


if __name__ == "__main__":
    main()
