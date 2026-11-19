#!/usr/bin/env python3
"""Find which frame of the looping menu backdrop movie a menu pair's original shows, and record it on the pair.

    python3 scripts/compare-menu-movie-phase.py [--dry-run] [--min-gain 1.0] [--fly-by-clip preview.bik] PAIR_DIR...

The movie runs from boot and never restarts, so the frame a reference still caught is an accident of how long the
walk took, and `--menu-page` draws frame zero unless told otherwise. No emulator runs: every frame of the movie is
scored against `theirs.png`, on the pixels of the pair's `ours.png` that are the bare movie (no widget over them), and
the winner goes into `pair.json`'s `ours.flags` as `--menu-movie-frame`. Re-render the pair with
`scripts/compare-rerender-ours.py` afterwards. A pair whose best frame does not beat frame zero by `--min-gain` grey
levels over that mask is left as it is: a dark stretch of the loop names no frame.

Only pairs drawn through `--menu-page` take the flag; a pair driven by `--until` is reported and skipped. The movie is
read from the cache `oag-game` writes on its first run, so one must have run for the title.

**An HD/Fury Track Select pair** (`--fly-by-clip`, the circuit's `fe/preview.bik` read off the disc with
`scripts/psarc.py cat`) has no backdrop movie: the movie is the `FlyByMovie` panel's clip, 602 frames of Vineta K's,
and the pair's `--menu-movie-frame` is that clip's frame. Every frame, scaled to the panel's box inside the Safe Area
the pair's original ran at, is scored against `theirs.png` over the box alone. This is a fit to the frame, not a read
of the emulator's frame counter: record it as such on the pair.
"""

import argparse
import glob
import json
import os
import subprocess
import sys
from pathlib import Path

import numpy as np
from PIL import Image

#: The transcoded backdrop per pair profile, under `~/.cache/oag/movies/`.
MOVIES = {
    "pulse-psp": "*-1071104-480x272-av1ll-270.ivf",
    "pulse-ps2": "54748_DATA_MOVIES_BG640.IPF-*-640x448-av1ll-270.ivf",
}
#: A pixel of `ours.png` within this of the movie frame it was drawn over counts as bare movie.
BARE = 6.0
#: The cache holds studio-range luma flagged full range: what the renderer's conversion does to it, on grey-ish teal.
GAIN, OFFSET = 255.0 / 219.0, -16.0 * 255.0 / 219.0


def movie_frames(profile, size):
    found = sorted(glob.glob(os.path.expanduser("~/.cache/oag/movies/" + MOVIES[profile])))
    if not found:
        sys.exit("no cached backdrop movie for %s: run oag-game on that title once" % profile)
    width, height = size
    raw = subprocess.run(["ffmpeg", "-v", "error", "-i", found[0], "-vf", "scale=%d:%d:flags=lanczos" % size,
                          "-f", "rawvideo", "-pix_fmt", "rgb24", "-"], capture_output=True, check=True).stdout
    frames = np.frombuffer(raw, np.uint8).reshape(-1, height, width, 3).astype(np.float32)
    return np.clip(frames * GAIN + OFFSET, 0, 255)


#: The `FlyByMovie` widget's rect on HD's 1920x1080 grid: `X`, `Y`, `Width`, `height` of the widget, not offset by its `Item`.
FLY_BY_RECT = (500.0, 190.0, 260.0, 170.0)


def fly_by_box(size, safe_percent):
    """The panel's pixel box `(x, y, w, h)` in a `size` frame whose front end is drawn at `safe_percent` of the screen."""
    width, height = size
    scale = safe_percent / 100.0
    x, y, w, h = FLY_BY_RECT
    scale_x, scale_y = scale * width / 1920.0, scale * height / 1080.0
    return (round((1 - scale) * width / 2 + x * scale_x), round((1 - scale) * height / 2 + y * scale_y),
            round(w * scale_x), round(h * scale_y))


def fly_by_frames(clip, box):
    """Every frame of the Bink clip, scaled to `box`."""
    x, y, w, h = box
    raw = subprocess.run(["ffmpeg", "-v", "error", "-i", str(clip), "-vf", "scale=%d:%d:flags=area" % (w, h),
                          "-f", "rawvideo", "-pix_fmt", "rgb24", "-"], capture_output=True, check=True).stdout
    return np.frombuffer(raw, np.uint8).reshape(-1, h, w, 3).astype(np.float32)


def rgb(path):
    return np.asarray(Image.open(path).convert("RGB"), np.float32)


def recorded_frame(flags):
    return int(flags[flags.index("--menu-movie-frame") + 1]) if "--menu-movie-frame" in flags else 0


def set_frame(flags, frame):
    while "--menu-movie-frame" in flags:
        at = flags.index("--menu-movie-frame")
        del flags[at:at + 2]
    if frame:
        flags += ["--menu-movie-frame", str(frame)]


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("pairs", nargs="+", type=Path)
    ap.add_argument("--dry-run", action="store_true")
    ap.add_argument("--min-gain", type=float, default=1.0)
    ap.add_argument("--fly-by-clip", type=Path, help="an HD Track Select pair's circuit clip (`fe/preview.bik`)")
    args = ap.parse_args()
    movies = {}
    for pair in args.pairs:
        rec = json.loads((pair / "pair.json").read_text())
        flags = rec["ours"]["flags"]
        if "--menu-page" not in flags:
            print("%s: skipped, not a --menu-page pair" % pair.name)
            continue
        theirs, ours = rgb(pair / "theirs.png"), rgb(pair / "ours.png")
        if args.fly_by_clip:
            percent = ((rec.get("original") or {}).get("safe_area") or {}).get("percent", 100)
            x, y, w, h = fly_by_box(theirs.shape[1::-1], percent)
            frames = fly_by_frames(args.fly_by_clip, (x, y, w, h))
            # The box's own edge pixels blend with the panel behind them in a scaled frame: the inner box is scored.
            inner = (slice(y + 4, y + h - 4), slice(x + 4, x + w - 4))
            scores = np.abs(frames[:, 4:-4, 4:-4] - theirs[inner]).mean(axis=(1, 2, 3))
            order = np.argsort(scores)
            best = int(order[0])
            print("%s: fly-by box %s, best frame %d (%.2f), next %d (%.2f), median %.2f, frame 0 %.2f" % (
                pair.name, (x, y, w, h), best, scores[best], order[1], scores[order[1]], np.median(scores), scores[0]))
            if not args.dry_run:
                set_frame(flags, best)
                rec["original"]["fly_by_frame"] = {
                    "frame": best, "clip_frames": len(frames),
                    "basis": "fitted: the clip frame whose picture, scaled to the panel box, is nearest the original's pixels there "
                             "(mean abs %.2f, the next frame's %.2f, the median frame's %.2f); the emulator's frame counter is not read"
                             % (scores[best], scores[order[1]], np.median(scores))}
                (pair / "pair.json").write_text(json.dumps(rec, indent=2) + "\n")
            continue
        key = (rec["profile"], theirs.shape[:2])
        if key not in movies:
            movies[key] = movie_frames(rec["profile"], theirs.shape[1::-1])
        frames = movies[key]
        mask = np.abs(ours - frames[recorded_frame(flags) % len(frames)]).max(axis=2) < BARE
        scores = np.abs(frames[:, mask] - theirs[mask]).mean(axis=(1, 2))
        best = int(np.argmin(scores))
        chosen = best if scores[0] - scores[best] >= args.min_gain else 0
        print("%s: bare %.0f%%, best frame %d (%.2f), frame 0 %.2f -> %s" % (
            pair.name, 100 * mask.mean(), best, scores[best], scores[0], chosen))
        if not args.dry_run:
            set_frame(flags, chosen)
            (pair / "pair.json").write_text(json.dumps(rec, indent=2) + "\n")


if __name__ == "__main__":
    main()
