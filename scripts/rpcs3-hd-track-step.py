#!/usr/bin/env python3
"""HD/Fury's Track Select lattice slide against ours: the original's frame N game frames after a circuit step, N read from RAM.

    source data/scratch/<lane>/env.sh ; export XDG_CACHE_HOME=$PWD/data/scratch/<lane>/cache
    OAG_COMPARE_PLATFORM=hd-fury python3 scripts/rpcs3-drive.py --image data/images/hdfury-ps3-eu.iso serve   # to Main Menu
    # walk to Track Creation by hand: `cross cross` from RACEBOX (Single Player), `left` back to circuit 0
    OAG_RPCS3_ATTACH=1 uv run --with evdev --with numpy python3 scripts/emu-run.py -- python3 scripts/rpcs3-hd-track-step.py capture \\
        --out data/scratch/<lane>/track-step --aims 1,2,3,4

`capture` presses `right`, stops the game once the slide's timer says the step has run the aimed number of game frames, reads the
widget in the stop, grabs the window through `grab_with_clock` and resumes: `<out>/NN.png` and `NN.json`. The slide timer is the
frame counter while it runs: `HexSelection_Step` sets it to 0.1 and the same update takes one frame off, so game frame `k` after
the step reads `0.1 - (k + 1) / 60` (`ship-select-ring.md`, "The lattice slide"); the columns then stand at
`i * 72 + dir * (1 - 10 t) * 72`. Frames after the slide ends are counted by the pulse counter (0.05 a frame). The
Fury backdrop is hidden for the run (`hd_fury_backdrop.py`), as the rest pair does.
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
sys.path.insert(0, str(HERE))
import hd_fury_backdrop as fb  # noqa: E402
import hd_track_hex as hth  # noqa: E402

drive = importlib.import_module("rpcs3-drive")
walk = importlib.import_module("rpcs3-hd-menu-walk")

GDB_PORT = int(os.environ.get("OAG_RPCS3_GDB", "127.0.0.1:2345").rsplit(":", 1)[1])
FLUSH_S = 0.6
FRAME_S = 1.0 / 60.0
SLIDE_S = 0.1


def frame_of(timer):
    """The game frame a slide timer reads, `None` outside a running slide."""
    if not 0.0 < timer < SLIDE_S:
        return None
    return int(round((SLIDE_S - timer) / FRAME_S - 1))


def frame_since(first, state):
    """The game frame of `state`: the frame the slide timer first gave, plus the frames the pulse counter (0.05 a frame, the
    widget's own update count) has advanced since, so frames after the slide's end are counted too."""
    k, phase = first
    return k + int(round(((state["phase"] - phase) % hth.PHASE_PERIOD) / hth.PHASE_PER_FRAME))


def cmd_capture(args):
    info = drive.live_session()
    if info is None:
        sys.exit("no live `rpcs3-drive.py serve` (OAG_RPCS3_ATTACH=1, the lane's XDG_CACHE_HOME)")
    pad, mem = drive.RemotePad(info["socket"]), drive.RemoteMem(info["socket"])
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    w, h = (int(v) for v in args.size.split("x"))
    walk.window(w, h)
    if drive.current_screen() != "Track Creation":
        sys.exit("the emulator is on %r, not Track Creation" % drive.current_screen())
    widget = hth.Widget(mem)
    backdrop = fb.Widget(mem)
    hidden = None if args.show_backdrop else backdrop.hide()
    if hidden is not None:
        backdrop.wait_frames(fb.FRAMES_TO_DECAY)
    from rpcs3_debugger import Debugger
    aims = [int(a) for a in args.aims.split(",")]
    trials = []
    try:
        for n in range(args.trials):
            aim = aims[n % len(aims)]
            time.sleep(args.rest)
            before = widget.state()
            dbg = Debugger(port=GDB_PORT)
            dbg.resume()
            time.sleep(0.05)
            presser = threading.Thread(target=lambda: pad.press(args.button, 0.15))
            t0 = time.time()
            presser.start()
            frame = first = None
            while time.time() - t0 < 4.0:
                state = widget.state()
                frame = frame_of(state["slide_timer"])
                if frame is not None and first is None:
                    first = (frame, state["phase"])
                if first is not None and (frame_since(first, state) or 0) >= aim - args.lead:
                    break
            dbg.pause()
            post = widget.state()
            presser.join()
            if first is None:
                dbg.resume()
                dbg.close()
                trials.append({"n": n, "aim": aim, "failed": "the slide never ran"})
                continue
            time.sleep(FLUSH_S)
            png = out / ("%02d.png" % n)
            walk.grab(png, w, h)
            again = widget.state()
            dbg.resume()
            dbg.close()
            record = {"n": n, "aim": aim, "frame": frame_since(first, post), "before": before, "after": post,
                      "still": again["slide_timer"] == post["slide_timer"] and again["phase"] == post["phase"], "png": png.name}
            (out / ("%02d.json" % n)).write_text(json.dumps(record, indent=1) + "\n")
            trials.append(record)
            print("%02d aim %d frame %s selected %d -> %d timer %.4f still %s" % (
                n, aim, record["frame"], before["selected"], post["selected"], post["slide_timer"], record["still"]), flush=True)
            # Step back so the next trial starts from the same circuit.
            pad.press("left" if args.button == "right" else "right", 0.15)
            time.sleep(0.6)
    finally:
        if hidden is not None:
            backdrop.restore()
    (out / "trials.json").write_text(json.dumps({"hidden": hidden, "trials": trials}, indent=1) + "\n")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    cap = sub.add_parser("capture")
    cap.add_argument("--out", required=True)
    cap.add_argument("--aims", default="1,2,3,4")
    cap.add_argument("--trials", type=int, default=8)
    cap.add_argument("--button", default="right")
    cap.add_argument("--size", default="1920x1080")
    cap.add_argument("--rest", type=float, default=1.0)
    cap.add_argument("--lead", type=int, default=1, help="frames before the aim at which the stop is asked for: the stub answers late")
    cap.add_argument("--show-backdrop", action="store_true")
    args = ap.parse_args()
    cmd_capture(args)


if __name__ == "__main__":
    main()
