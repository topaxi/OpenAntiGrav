#!/usr/bin/env python3
"""HD/Fury's Ship Select step against ours: the original's frame N game frames after the selection moved, N read from RAM.

    source data/scratch/<lane>/env.sh ; export XDG_CACHE_HOME=/tmp/oag-<lane>-cache
    OAG_COMPARE_PLATFORM=hd-fury python3 scripts/rpcs3-drive.py --image data/images/hdfury-ps3-eu.iso serve   # to Main Menu
    # walk to Ship Select by hand: `right cross right cross cross`
    OAG_RPCS3_ATTACH=1 uv run --with evdev python3 scripts/emu-run.py -- python3 scripts/rpcs3-hd-ship-step.py timeline \\
        --buttons right,right,left --out data/scratch/<lane>/timeline.json
    OAG_RPCS3_ATTACH=1 uv run --with evdev python3 scripts/emu-run.py -- python3 scripts/rpcs3-hd-ship-step.py capture \\
        --out data/scratch/<lane>/trials --aims 2,3,4,6,8,12,20,30

`timeline` presses buttons and logs, without pausing, every game frame of the screen's fit (`hd_ship_step.py`): the
selection, the model's name and length, the depth target and drawn depth, both slides and the player's six input
terms. `capture` presses `right`, stops the game once the frame count since the selection moved reaches an aim, reads the
state in the stop, grabs the window through `grab_with_clock` and resumes: `<out>/NN.png` and `NN.json` (the frame
count, from and to model, state, adapted luma, the clock sidecar). The Fury backdrop is hidden for the run
(`hd_fury_backdrop.py`), as the rest pair does. `pairs` turns a capture into compare-page pairs, ours drawn with
`--menu-picker-step-seconds` at the same frame count.
"""
import argparse
import importlib
import json
import os
import sys
import threading
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
MAIN = Path("/home/topaxi/projects/OpenAntiGrav")
sys.path.insert(0, str(HERE))
import hd_fury_backdrop as fb  # noqa: E402
import hd_ship_step as hs  # noqa: E402

drive = importlib.import_module("rpcs3-drive")
walk = importlib.import_module("rpcs3-hd-menu-walk")

GDB_PORT = int(os.environ.get("OAG_RPCS3_GDB", "127.0.0.1:2345").rsplit(":", 1)[1])
FRAME_SECONDS = 1.0 / 60.0
#: Seconds the window is given to show the frame the stopped game last handed the RSX (`hd_flyer_clock.PAUSE_FLUSH_S`).
FLUSH_S = 0.6
#: Frames before the aim at which the stop is asked for: the stub answers a few frames late.
LEAD = 1
TEAMS = ("AG_Systems", "Assegai", "Auricom", "EGX", "Feisar", "Goteki", "Harimau", "Icaras", "Mirage", "Piranha", "Qirex", "Triakis")


def team_of(model_name):
    """`Piranha` for `piranha_c1`."""
    for team in TEAMS:
        if model_name and model_name.lower().startswith(team.lower() + "_"):
            return team
    return None


def attach():
    info = drive.live_session()
    if info is None:
        sys.exit("no live `rpcs3-drive.py serve` (OAG_RPCS3_ATTACH=1, the lane's XDG_CACHE_HOME)")
    mem = drive.RemoteMem(info["socket"])
    return drive.RemotePad(info["socket"]), mem, hs.Screen(mem)


def described(screen, state):
    name = screen.model_name(state["model"])
    return dict(state, name=name, length=round(screen.length(state["model"]), 3))


def cmd_timeline(args):
    pad, mem, screen = attach()
    rows = []
    for button in args.buttons.split(","):
        time.sleep(args.rest)
        counter = hs.Counter()
        first = screen.state()
        counter.feed(first)
        presser = threading.Thread(target=lambda b=button: pad.press(b, 0.15))
        t0 = time.time()
        presser.start()
        last = None
        while time.time() - t0 < args.seconds:
            state = screen.state()
            frame = counter.feed(state)
            if hs.key(state) != last:
                last = hs.key(state)
                rows.append({"button": button, "t": round(time.time() - t0, 4), "frame": frame,
                             **described(screen, state)})
        presser.join()
    Path(args.out).write_text(json.dumps(rows, indent=1) + "\n")
    print("%d rows to %s" % (len(rows), args.out))


def frames_of(counter, post, screen):
    """The frame count of a state read in the stop: a polled row's own, else the polled count plus the frames the
    walking phase of the law needs to reach the read depth (`exact` False beyond the walk)."""
    for frame, state in counter.rows:
        if hs.key(state) == hs.key(post):
            return frame, True
    frame, last = counter.rows[-1]
    depth, target = last["depth"], last["target"]
    for n in range(1, 8):
        depth, target = hs.forward(depth, target, 1)
        if abs(depth - post["depth"]) < 2e-4 and abs(target - post["target"]) < 2e-3:
            return frame + n, True
    return frame + 1, False


def cmd_capture(args):
    pad, mem, screen = attach()
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    w, h = (int(v) for v in args.size.split("x"))
    walk.window(w, h)
    if drive.current_screen() != "Team Selection":
        sys.exit("the emulator is on %r, not Team Selection" % drive.current_screen())
    widget = fb.Widget(mem)
    hidden = None if args.show_backdrop else widget.hide()
    if hidden is not None:
        widget.wait_frames(fb.FRAMES_TO_DECAY)
    aims = [int(a) for a in args.aims.split(",")]
    trials = []
    from rpcs3_debugger import Debugger
    try:
        for n in range(args.trials):
            aim = aims[n % len(aims)]
            time.sleep(args.rest)
            before = described(screen, screen.state())
            counter = hs.Counter()
            counter.feed(before)
            dbg = Debugger(port=GDB_PORT)
            dbg.resume()
            time.sleep(0.05)
            presser = threading.Thread(target=lambda: pad.press("right", 0.15))
            t0 = time.time()
            presser.start()
            polled = None
            while time.time() - t0 < 4.0:
                state = screen.state()
                frame = counter.feed(state)
                if frame is not None and frame >= aim - args.lead:
                    polled = frame
                    break
            stopped_at = time.time() - t0
            dbg.pause()
            post = screen.state()
            presser.join()
            record = {"n": n, "aim": aim, "polled_frame": polled, "stopped_after_s": round(stopped_at, 4), "before": before}
            if polled is None or not counter.rows:
                record["failed"] = "the selection never moved"
                dbg.resume()
                dbg.close()
                trials.append(record)
                continue
            frame, exact = frames_of(counter, post, screen)
            time.sleep(FLUSH_S)
            png = out / ("%02d.png" % n)
            walk.grab(png, w, h)
            again = screen.state()
            luma = widget.adapted_luma()
            dbg.resume()
            dbg.close()
            record.update({"frame": frame, "frame_exact": exact, "after": described(screen, post),
                           "still": hs.key(again) == hs.key(post), "adapted_luma": luma,
                           "png": png.name, "rows": [(f, s["target"], s["depth"], s["slide"]) for f, s in counter.rows]})
            (out / ("%02d.json" % n)).write_text(json.dumps(record, indent=1) + "\n")
            trials.append(record)
            print("%02d aim %2d polled %s frame %s%s %s -> %s depth %.3f" % (
                n, aim, polled, frame, "" if exact else "?", before["name"], record["after"]["name"], post["depth"]), flush=True)
    finally:
        if hidden is not None:
            widget.restore()
    (out / "trials.json").write_text(json.dumps({"hidden": hidden, "trials": [
        {k: v for k, v in t.items() if k != "rows"} for t in trials]}, indent=1) + "\n")


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    t = sub.add_parser("timeline", help="log every game frame of the fit across button presses, unpaused")
    t.add_argument("--buttons", required=True)
    t.add_argument("--out", required=True)
    t.add_argument("--seconds", type=float, default=2.5)
    t.add_argument("--rest", type=float, default=1.5)
    t.set_defaults(run=cmd_timeline)
    c = sub.add_parser("capture", help="stop the game at aimed frames after a step and grab the window")
    c.add_argument("--out", required=True)
    c.add_argument("--aims", default="2,3,4,6,8,12,20,30")
    c.add_argument("--trials", type=int, default=8)
    c.add_argument("--rest", type=float, default=3.0)
    c.add_argument("--lead", type=int, default=LEAD, help="frames before the aim the stop is asked for")
    c.add_argument("--size", default="1920x1080")
    c.add_argument("--show-backdrop", action="store_true")
    c.set_defaults(run=cmd_capture)
    args = ap.parse_args()
    return args.run(args)


if __name__ == "__main__":
    sys.exit(main())
