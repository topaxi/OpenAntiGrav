#!/usr/bin/env python3
"""Print the Quake's four vertex-program uniforms from a dumped RSX frame.

    python3 scripts/hd-quake-uniforms.py <dir> <stem>...

`<dir>/<stem>-4*.bin` are the command-stream spans `rpcs3_draw_hook.py` writes
(`scripts/rpcs3-hd-whiteout.py --quake --no-targets`, `HOOK_RING_ONLY=1`). The
`StaticQuake` vertex program (`docs/ghidra/functions/ps3-hdfury-eu/quake.md`)
reads `quakePointB` at `c[207]` (register 463), `quakePointA` at `c[208]` (464),
`quakeOffset` at `c[209]` (465) and `quakeTrackUpNormal` at `c[206]` (462), so
this lists, per frame, every distinct value those four registers held across the
frame's draws and how many draws held it.
"""
import collections
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import importlib.util

spec = importlib.util.spec_from_file_location("rsx_draw_list", os.path.join(os.path.dirname(os.path.abspath(__file__)), "rsx-draw-list.py"))
dl = importlib.util.module_from_spec(spec)
spec.loader.exec_module(dl)

NAMES = {462: "trackUp", 463: "pointB", 464: "pointA", 465: "offset"}


def floats(words):
    return tuple(round(struct.unpack(">f", struct.pack(">I", w))[0], 4) for w in words)


def is_up(v):
    """The track-up normal: a unit vector with `w == 0` (every other program's
    value in register 462 is a bias, a scale or zero)."""
    return v is not None and abs(sum(x * x for x in v[:3]) - 1.0) < 0.01 and v[3] == 0.0 and v[:3] != (1.0, 0.0, 0.0)


def main():
    directory = sys.argv[1]
    for stem in sys.argv[2:]:
        draws, _events = dl.draws(directory, stem)
        quake = collections.Counter()
        for d in draws:
            c = d.get("c", {})
            vals = [floats(c[k]) if k in c else None for k in (462, 463, 464, 465)]
            if is_up(vals[0]):
                quake[tuple(vals)] += 1
        print("%s: %d draws, %d StaticQuake-looking" % (stem, len(draws), sum(quake.values())))
        for (up, b, a, off), count in quake.most_common(8):
            print("  x%-3d up %s  B %s  A %s  offset %s" % (count, up, b, a, off))
        continue
        seen = {k: collections.Counter() for k in NAMES}
        for d in draws:
            for k in NAMES:
                words = d.get("c", {}).get(k)
                if words is not None:
                    seen[k][floats(words)] += 1
        print("%s: %d draws" % (stem, len(draws)))
        for k, name in NAMES.items():
            for value, count in seen[k].most_common(6):
                print("  c%d %-8s x%-4d %s" % (k, name, count, value))


if __name__ == "__main__":
    main()
