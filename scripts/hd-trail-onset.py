#!/usr/bin/env python3
"""How far from its blast each `trail` chain of a Rocket craft hit starts, in an HD film's frame.

    scripts/hd-trail-onset.py data/reference/hd-capture/rocket-craft-hit/hit2 a0233 [STEM...]

Works in view space off the frame's command ring (`hd-particle-quads.py`'s `draws`): a blast's `glow_longer` is
the single capped additive quad coloured from its table (red 255, blue under 128) and stands on the blast's site;
its `trail` is the run of alpha-over draws that follow it with 8x4-cell white quads, one draw per trail head. For
every such draw the distance from the nearest quad to the nearest site is printed: the head's spawn radius plus at
most one tick of travel (`docs/rendering/particle-fill.md`, "The trails start on the blast's hemisphere").
"""
import importlib.util
import math
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("hpq", HERE / "hd-particle-quads.py")
hpq = importlib.util.module_from_spec(spec)
sys.path.insert(0, str(HERE))
spec.loader.exec_module(hpq)


def cells(quad):
    us = [v[7] for v in quad]
    vs = [v[8] for v in quad]
    return round(max(us) - min(us), 3), round(max(vs) - min(vs), 3)


def centre(quad):
    return tuple(sum(v[k] for v in quad) / 4 for k in range(3))


def onsets(film, stem):
    draws = list(hpq.draws(film, stem))
    sites = [centre(q) for blend, quads, _ in draws if blend == "add" and len(quads) == 1
             for q in quads if q[0][3] == 255 and q[0][5] < 128 and cells(q) == (1.0, 1.0)]
    out = []
    for blend, quads, _ in draws:
        chain = [centre(q) for q in quads if cells(q) == (0.125, 0.25) and q[0][3] == 254]
        if blend != "over" or len(chain) < 3 or not sites:
            continue
        out.append(min(math.dist(p, s) for p in chain for s in sites))
    return sites, out


def main():
    if len(sys.argv) < 3:
        sys.exit(__doc__)
    film = Path(sys.argv[1])
    for stem in sys.argv[2:]:
        sites, near = onsets(film, stem)
        print("%s %s: %d sites, %d chains, onset %s" % (film.name, stem, len(sites), len(near),
                                                      " ".join("%.1f" % d for d in sorted(near))))


if __name__ == "__main__":
    main()
