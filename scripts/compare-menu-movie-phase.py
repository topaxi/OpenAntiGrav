#!/usr/bin/env python3
"""Find which frame of the looping menu backdrop movie a menu pair's original shows, and record it on the pair.

    python3 scripts/compare-menu-movie-phase.py [--dry-run] [--min-gain 1.0] PAIR_DIR...

The movie runs from boot and never restarts, so the frame a reference still caught is an accident of how long the
walk took, and `--menu-page` draws frame zero unless told otherwise. No emulator runs: every frame of the movie is
scored against `theirs.png`, on the pixels of the pair's `ours.png` that are the bare movie (no widget over them), and
the winner goes into `pair.json`'s `ours.flags` as `--menu-movie-frame`. Re-render the pair with
`scripts/compare-rerender-ours.py` afterwards. A pair whose best frame does not beat frame zero by `--min-gain` grey
levels over that mask is left as it is: a dark stretch of the loop names no frame.

Only pairs drawn through `--menu-page` take the flag; a pair driven by `--until` is reported and skipped. The movie is
read from the cache `oag-game` writes on its first run, so one must have run for the title.
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
    args = ap.parse_args()
    movies = {}
    for pair in args.pairs:
        rec = json.loads((pair / "pair.json").read_text())
        flags = rec["ours"]["flags"]
        if "--menu-page" not in flags:
            print("%s: skipped, not a --menu-page pair" % pair.name)
            continue
        theirs, ours = rgb(pair / "theirs.png"), rgb(pair / "ours.png")
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
