#!/usr/bin/env python3
"""Read an `rpcs3-drive.py countdown` recording: where the gantry flips and the craft moves.

    python3 scripts/hd-countdown-frames.py REC.mp4 --out DIR

Decodes the video at its own 30 fps to PNGs under DIR, then reports, per frame
index of the *decoded clip* (frame 0 = the clip's first frame):

- `red`: pixels of the gantry board's red digit strip in the fixed crop
  (x 540-700, y 225-262 of 1280x720 - the grid view is the same every boot);
- `teal`: pixels of the board's green face;
- `motion`: mean absolute grey difference of a road crop against a frame that
  is certainly still, which leaves ~5 while the craft is held and climbs the
  frame the camera first moves.

and the three events: the green step (red collapses and teal arrives), the
first frame the road differs from the held one by more than 2x the idle noise,
and the first red frame (the `3` sliding in). The lap timer is read by eye off
the `--timer` crops, which this also writes (digits are too soft to OCR).
Needs Pillow and ffmpeg.
"""
import argparse
import subprocess
import sys
from pathlib import Path

from PIL import Image, ImageChops

GANTRY = (540, 225, 700, 262)
ROAD = (300, 300, 980, 420)
TIMER = (60, 625, 300, 675)


def decode(video, out, start, length):
    out.mkdir(parents=True, exist_ok=True)
    subprocess.run(["ffmpeg", "-v", "error", "-y", "-ss", str(start), "-t",
                    str(length), "-i", str(video), "-vf", "scale=1280:720",
                    "-start_number", "0", str(out / "%04d.png")], check=True)
    return sorted(out.glob("*.png"))


def counts(im):
    px = list(im.convert("RGB").crop(GANTRY).getdata())
    red = sum(1 for r, g, b in px if r > 170 and g < 90 and b < 90)
    teal = sum(1 for r, g, b in px if g > r + 40 and b > r + 30 and r < 90)
    return red, teal


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("video")
    ap.add_argument("--out", required=True)
    ap.add_argument("--start", type=float, default=28.0)
    ap.add_argument("--length", type=float, default=14.0)
    args = ap.parse_args()
    out = Path(args.out)
    frames = decode(args.video, out, args.start, args.length)
    rows = []
    for i, f in enumerate(frames):
        rows.append((i,) + counts(Image.open(f)))
    peak = max(r[1] for r in rows)
    step = None
    seen_red = False
    first_red = None
    for i, red, teal in rows:
        if red > peak * 0.5:
            seen_red = True
        if red > 40 and first_red is None:
            first_red = i
        if seen_red and step is None and red < peak * 0.1 and teal > 200:
            step = i
    ref = Image.open(frames[(step or 100) - 8]).convert("L").crop(ROAD)
    motion = []
    for i, f in enumerate(frames):
        d = ImageChops.difference(Image.open(f).convert("L").crop(ROAD), ref)
        n = ROAD[2] - ROAD[0]
        m = ROAD[3] - ROAD[1]
        motion.append(sum(d.getdata()) / (n * m))
    idle = max(motion[(step or 100) - 14:(step or 100) - 4])
    move = next((i for i in range((step or 100) - 3, len(motion))
                 if motion[i] > max(2 * idle, idle + 3)), None)
    print("frames %d, peak red %d" % (len(frames), peak))
    print("first red (3 sliding in): frame %s" % first_red)
    print("green step (red collapses, teal arrives): frame %s" % step)
    print("craft first moves (road motion > idle*2): frame %s (idle %.1f)"
          % (move, idle))
    if step is not None:
        for i in range(step - 3, step + 4):
            print("  %4d red %5d teal %5d motion %5.1f" %
                  (i, rows[i][1], rows[i][2], motion[i]))
        timer = Image.new("RGB", (240 * 2 * 8, 100 * 2))
        for k, i in enumerate(range(step - 2, step + 14)):
            c = Image.open(frames[i]).convert("RGB").crop(TIMER)
            timer.paste(c.resize((480, 100)), ((k % 8) * 480, (k // 8) * 100))
        timer.save(out / "timer.png")
        print("timer strip (frames %d..%d): %s" % (step - 2, step + 13,
                                                   out / "timer.png"))


if __name__ == "__main__":
    sys.exit(main())
