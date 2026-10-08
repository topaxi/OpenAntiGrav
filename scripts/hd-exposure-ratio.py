#!/usr/bin/env python3
"""Final-over-scene luma ratio of one RPCS3 HD frame: the original's exposure, measured.

    python3 scripts/hd-exposure-ratio.py <scene-dump.bin> <screenshot.png>

`<scene-dump.bin>` is the 1280x720 `A8R8G8B8` resolved scene target the
`rpcs3-drive.py place --dump 0x40cc0000:0x384000` (or `capture --region`)
writes beside a shot: the picture before the resolve's exposure and bloom and
before the HUD. `<screenshot.png>` is that shot, resampled to 1280x720.

The resolve is `scene * scale + bloom`, so the whole-frame luma ratio is the
exposure `scale` plus the bloom's share. Measured 2026-10-08 on ten captures
it is 1.02 to 1.14 (see renderer.md, "The exposure is unity, and the
brightness gap is per surface"): `scale` is 1.0 in the original's frames, at
race clock 0.00.0 as at 0.31.7. The HUD is in the screenshot and not in the
dump, so a HUD-heavy frame reads a little above the true ratio.

Needs numpy and Pillow. Prints one line; exits 1 on a size mismatch.
"""
import sys

import numpy as np
from PIL import Image

W = np.array([0.299, 0.587, 0.114])


def main():
    if len(sys.argv) != 3:
        print(__doc__)
        return 2
    raw = np.fromfile(sys.argv[1], dtype=np.uint8)
    if raw.size != 1280 * 720 * 4:
        print("%s: %d bytes, not a 1280x720 A8R8G8B8 surface" % (sys.argv[1], raw.size))
        return 1
    scene = raw.reshape(720, 1280, 4)[:, :, 1:4] / 255.0
    shot = Image.open(sys.argv[2]).convert("RGB").resize((1280, 720), Image.LANCZOS)
    final = np.asarray(shot) / 255.0
    s, f = (scene @ W).mean(), (final @ W).mean()
    print("scene luma %.3f  final luma %.3f  final/scene %.3f" % (s, f, f / s))
    return 0


if __name__ == "__main__":
    sys.exit(main())
