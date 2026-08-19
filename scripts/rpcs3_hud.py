"""Read WipEout HD's speed readout out of an RPCS3 recording.

`scripts/rpcs3-drive.py record` captures a driven lap, and HD's HUD puts the
speed in the frame - so a recording is, in principle, a trace. This turns it
into one, at whatever rate the frames are sampled:

    uv run --with numpy python3 scripts/rpcs3_hud.py read <recording.mp4> --fps 4

**Read the coverage number it prints before using the output.** This is an
optical reading of an alpha-blended HUD, not the engine's own value, and the
two things that limit it are structural rather than tuning:

1. **The HUD is blended over the scene**, so over a bright stretch of Talon's
   Junction the glyph strokes are barely above the background. A global
   threshold reads about 13 % of frames; the high-pass here (subtract a box
   blur wider than a glyph, keep what is locally brighter) reads about 60 %.
   The rest are honestly reported as gaps rather than guessed.
2. **The template set is what one clip contained.** Digits are matched against
   bitmaps clustered out of a 52-second run, which held 0, 2, 3, 4, 5, 7 and 8
   and never a 1, 6 or 9 - so a frame showing one of those reads as unknown and
   is dropped. `glyphs` re-runs the clustering on any recording and writes a
   labelled montage, which is how the set grows; it is deliberately not a
   generic OCR.

A darker circuit would lift both numbers, and the memory route (read the value
the engine holds, through `scripts/rpcs3_debugger.py`) would replace them
entirely - see docs/reverse-engineering/rpcs3-debugger.md.

Geometry is for 1280x720 footage, which is what `recording.yml` ships.
"""

import argparse
import os
import subprocess
import sys

import numpy as np

# The speed plate, with enough margin around the numerals for the high-pass to
# have a background to subtract. w, h, x, y.
CROP = (190, 34, 1000, 546)
# Rows inside the crop holding glyphs rather than the plate's own border, and
# the columns holding the numerals rather than the "KM/H" label.
BAND = (6, 28)
DIGITS = (90, 150)
GLYPH = (10, 16)          # every glyph is normalised to this before matching
BLUR_RADIUS = 6           # wider than a glyph, so a glyph cannot flatten itself
HIGH_PASS = 0.05          # how far above its surroundings a stroke has to be
MIN_COLUMN = 5            # pixels lit in a column before it counts as ink
GLYPH_WIDTH = (4, 20)
MAX_DISTANCE = 0.18       # beyond this a glyph is unknown, not a nearest guess

# The numerals are right-aligned in a fixed field: measured over a 52-second
# run, the last run ends at column 53 of the numeral window in 100 frames of
# 111, and consecutive runs are 13 columns apart. Checking a read against that
# is what turns a *dropped* digit into a rejected frame instead of a plausible
# wrong number - `35` sitting between `353` and `355` was the failure this
# exists to stop, and it is not detectable from the digits alone.
SLOT_RIGHT = 53
SLOT_PITCH = 13
SLOT_TOLERANCE = 2
LEFT_INK_COLUMNS = 2      # lit columns in the empty slot left of the number
# The numeral field is three slots wide, starting at these columns. Anything to
# the left of the first is the plate's own bracket, not a digit, which is why
# the "is there ink we missed?" check only looks at slots inside the field.
SLOT_LEFTMOST = 17

# Clustered out of a Talon's Junction run and labelled by eye; several entries
# are the same digit at different render quality, which is why this is a list
# and not a dict.
TEMPLATES = [
    ("0", "ffc00001fee1f03c0f03c0f03c0f87ffc00003ff"),
    ("0", "7fbffc0f03c0f03c0f03c0f03c0fff3f80000000"),
    ("2", "ffc0000000ffc0300cff7fb80c0300ffc0000000"),
    ("2", "3f1ff00000000773ff80c0200c01ff3fc00003fe"),
    ("2", "ff3ff00c0300c067f3fc00000000df7fc3000000"),
    ("3", "ffc0000000ffc0300c03fffff00c03ffc0000000"),
    ("3", "ffbff00c0300c03fffff00c0300fffff80000000"),
    ("3", "ffc00003fe00c0300c7fffc0300c03ffc000021d"),
    ("4", "ffc00000000703c1e1c3e0fff00c0300c0000304"),
    ("4", "0381c070783c5e1f07fffff970040300c0000000"),
    ("5", "ffc0000000fff00c03807f8ff00c03ffc0000000"),
    ("7", "ffc0000000ffc07038380c07070380e000000000"),
    ("7", "ff8030000000000c03c00e03f07c000000000000"),
    ("7", "ffffe003ffffc070301c1c0e038380c0300003ff"),
    ("8", "fffff000fe7ff83c0f03fff03c0f837fdfe003ff"),
]


def _templates():
    size = GLYPH[0] * GLYPH[1]
    out = []
    for digit, hexed in TEMPLATES:
        bits = bin(int(hexed, 16))[2:].rjust(size, "0")
        out.append((digit, np.array([b == "1" for b in bits],
                                    dtype=np.float32).reshape(GLYPH[1], GLYPH[0])))
    return out


def _blur(a, radius):
    """Separable box blur, so this needs numpy and nothing else."""
    k = 2 * radius + 1
    pad = np.pad(a, ((0, 0), (radius, radius)), mode="edge")
    run = np.cumsum(pad, axis=1, dtype=np.float32)
    run = np.concatenate([run[:, :1], run], axis=1)
    a = (run[:, k:] - run[:, :-k]) / k
    pad = np.pad(a, ((radius, radius), (0, 0)), mode="edge")
    run = np.cumsum(pad, axis=0, dtype=np.float32)
    run = np.concatenate([run[:1, :], run], axis=0)
    return (run[k:, :] - run[:-k, :]) / k


def frames(video, fps):
    """Grey crops of the speed plate, one per sampled frame."""
    w, h, x, y = CROP
    cmd = ["ffmpeg", "-v", "error", "-i", str(video), "-map", "0:v:0",
           "-vf", "fps=%g,crop=%d:%d:%d:%d,format=gray" % (fps, w, h, x, y),
           "-f", "rawvideo", "-"]
    raw = subprocess.run(cmd, capture_output=True).stdout
    if len(raw) < w * h:
        raise SystemExit("ffmpeg produced no frames for %s" % video)
    return np.frombuffer(raw, dtype=np.uint8).reshape(-1, h, w)


def ink(crop):
    """Where the crop is locally brighter than its surroundings."""
    a = crop.astype(np.float32) / 255.0
    return (a - _blur(a, BLUR_RADIUS)) > HIGH_PASS


def columns(mask):
    """Glyph-wide runs of lit columns inside the numeral window."""
    band = mask[BAND[0]:BAND[1], DIGITS[0]:DIGITS[1]]
    lit = band.sum(axis=0) >= MIN_COLUMN
    runs, start = [], None
    for i, on in enumerate(lit):
        if on and start is None:
            start = i
        elif not on and start is not None:
            runs.append((start, i))
            start = None
    if start is not None:
        runs.append((start, len(lit)))
    return band, [r for r in runs if GLYPH_WIDTH[0] <= r[1] - r[0] <= GLYPH_WIDTH[1]]


def normalise(band, a, b):
    sub = band[:, a:b]
    rows = np.where(sub.sum(axis=1) >= 2)[0]
    if len(rows) < 8:
        return None
    sub = sub[rows[0]:rows[-1] + 1]
    ys = np.arange(GLYPH[1]) * sub.shape[0] // GLYPH[1]
    xs = np.arange(GLYPH[0]) * sub.shape[1] // GLYPH[0]
    return sub[np.ix_(ys, xs)].astype(np.float32)


def aligned(band, runs):
    """Do these runs sit in the fixed, right-aligned digit slots?

    Also rejects a read with ink still sitting in the slot to its left, which
    is what a digit that failed to segment looks like.
    """
    if abs(runs[-1][1] - SLOT_RIGHT) > SLOT_TOLERANCE:
        return False
    for (a, _), (b, _) in zip(runs, runs[1:]):
        if abs((b - a) - SLOT_PITCH) > SLOT_TOLERANCE:
            return False
    left = runs[0][0] - SLOT_PITCH
    if left >= SLOT_LEFTMOST - SLOT_TOLERANCE:
        slot = band[:, left:runs[0][0]]
        if int((slot.sum(axis=0) >= 2).sum()) >= LEFT_INK_COLUMNS:
            return False
    return True


def read_frame(crop, templates):
    """The speed in this frame, or None when it cannot be read honestly."""
    band, runs = columns(ink(crop))
    if not 1 <= len(runs) <= 3:
        return None
    if not aligned(band, runs):
        return None
    digits = []
    for a, b in runs:
        glyph = normalise(band, a, b)
        if glyph is None:
            return None
        best, distance = None, MAX_DISTANCE
        for digit, template in templates:
            d = float(np.abs(template - glyph).mean())
            if d < distance:
                best, distance = digit, d
        if best is None:
            return None
        digits.append(best)
    return int("".join(digits))


def read(video, fps):
    """`(seconds, speed_or_None)` for every sampled frame."""
    templates = _templates()
    return [(n / fps, read_frame(crop, templates))
            for n, crop in enumerate(frames(video, fps))]


def cmd_read(args):
    rows = read(args.video, args.fps)
    got = [s for _, s in rows if s is not None]
    print("t_seconds,speed_kmh")
    for t, speed in rows:
        print("%.3f,%s" % (t, "" if speed is None else speed))
    print("# %d/%d frames read (%.0f %%), speed %d-%d km/h"
          % (len(got), len(rows), 100.0 * len(got) / max(1, len(rows)),
             min(got) if got else 0, max(got) if got else 0), file=sys.stderr)
    return 0


def cmd_glyphs(args):
    """Cluster this recording's glyphs and write them out for labelling."""
    out = os.path.abspath(args.out)
    os.makedirs(out, exist_ok=True)
    centres, counts = [], []
    for crop in frames(args.video, args.fps):
        band, runs = columns(ink(crop))
        if not 1 <= len(runs) <= 3:
            continue
        for a, b in runs:
            glyph = normalise(band, a, b)
            if glyph is None:
                continue
            distances = [float(np.abs(c - glyph).mean()) for c in centres]
            if distances and min(distances) < 0.22:
                i = int(np.argmin(distances))
                centres[i] = (centres[i] * counts[i] + glyph) / (counts[i] + 1)
                counts[i] += 1
            else:
                centres.append(glyph.copy())
                counts.append(1)
    order = np.argsort(counts)[::-1][:args.keep]
    for rank, i in enumerate(order):
        path = os.path.join(out, "%02d.gray" % rank)
        ((centres[i] > 0.5).astype(np.uint8) * 255).tofile(path)
    print("%d clusters, kept %d covering %d of %d glyphs; raw %dx%d bitmaps in %s"
          % (len(centres), len(order), sum(counts[i] for i in order),
             sum(counts), GLYPH[0], GLYPH[1], out), file=sys.stderr)
    print("render with: magick -size %dx%d -depth 8 gray:%s/00.gray -resize 700%% t.png"
          % (GLYPH[0], GLYPH[1], out), file=sys.stderr)
    return 0


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="command", required=True)

    rd = sub.add_parser("read")
    rd.add_argument("video")
    rd.add_argument("--fps", type=float, default=4.0)
    rd.set_defaults(run=cmd_read)

    gl = sub.add_parser("glyphs")
    gl.add_argument("video")
    gl.add_argument("--fps", type=float, default=6.0)
    gl.add_argument("--keep", type=int, default=24)
    gl.add_argument("--out", default="/tmp/rpcs3-hud-glyphs")
    gl.set_defaults(run=cmd_glyphs)

    args = parser.parse_args(argv)
    return args.run(args)


if __name__ == "__main__":
    sys.exit(main())
