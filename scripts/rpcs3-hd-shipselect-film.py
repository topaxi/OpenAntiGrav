#!/usr/bin/env python3
"""Film HD's Team Selection (Ship Select) while a button plan runs.

Attached to a long-lived `rpcs3-drive.py serve` (OAG_RPCS3_ATTACH=1) that sits on
`Team Selection`. Grabs the emulator's X display with ffmpeg x11grab at --fps
into a lossless mkv, taps the buttons of --plan at the given offsets, and writes
events.json with each press's time in seconds from the start of the grab (the
grab's own start lag is measured by a first marker press-free interval, so
treat times as +-1/fps plus ~0.1 s).

  --plan "circle@1,cross@5,right@12,right@18,left@24,left@30" --duration 70

Frames afterwards: ffmpeg -i film.mkv -vf crop=1280:720:X:Y f%04d.png
"""
import argparse, json, subprocess, sys, time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import importlib
drive = importlib.import_module("rpcs3-drive")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--plan", default="")
    ap.add_argument("--duration", type=float, default=60.0)
    ap.add_argument("--fps", type=int, default=30)
    args = ap.parse_args()
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    info = drive.live_session()
    if info is None:
        print("no live session", file=sys.stderr)
        return 1
    pad = drive.RemotePad(info["socket"])
    plan = []
    for item in filter(None, args.plan.split(",")):
        button, at = item.split("@")
        plan.append((float(at), button))
    plan.sort()
    ff = subprocess.Popen(
        ["ffmpeg", "-y", "-loglevel", "error", "-f", "x11grab", "-framerate",
         str(args.fps), "-video_size", "2000x1200", "-i", ":" + drive.DISPLAY.split(":")[-1],
         "-c:v", "ffv1", "-t", str(args.duration), str(out / "film.mkv")])
    print("ffmpeg pid", ff.pid, flush=True)
    t0 = time.time()
    events = []
    for at, button in plan:
        while time.time() - t0 < at:
            time.sleep(0.005)
        t = time.time() - t0
        pad.press(button, 0.15)
        events.append({"t": round(t, 3), "button": button,
                       "screen": drive.current_screen()})
        print("%6.2f %s" % (t, button), flush=True)
    ff.wait()
    (out / "events.json").write_text(json.dumps(
        {"fps": args.fps, "events": events}, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
