#!/usr/bin/env python3
"""One line per dumped HD frame: is it complete, which `FunkLayerZoom` inputs it carried, did the pass run.

    python3 scripts/rsx-frame-census.py <dir> [<dir> ...]

Reads every `<dir>/<stem>.json` that `scripts/rpcs3-hd-postchain.py` wrote beside a frame, and the
frame's draws (`scripts/rsx-draw-list.py`). A frame is **complete** when a draw on the bloom chain's last
target (`0x02240000`) is followed by a draw on a screen buffer (`0x00010000` or `0x00394000`); a pause that
caught the main thread mid-write keeps the scene and loses exactly the post-chain passes this reads, so a
program's absence from an incomplete frame says nothing. `funk` is the live `FunkLayer` `+0x58` (damage
pulse P) and `+0x64` (engine glow E) at the pause, `zoom` the draws that ran `FunkLayerZoom_fp`
(program `0x92b180`, which sits at `0x00741180` in the dumps) with the vertex constant `size` they used,
`history_reads` the render targets sampled before the dumped frame wrote them (a previous frame's picture).
"""
import glob
import importlib.util
import json
import os
import struct
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ZOOM_FP = 0x00741180


def load(name, file):
    spec = importlib.util.spec_from_file_location(name, os.path.join(HERE, file))
    mod = importlib.util.module_from_spec(spec)
    sys.modules[name] = mod
    spec.loader.exec_module(mod)
    return mod


def complete(draws):
    ends = [i for i, d in enumerate(draws) if d.get(0x210) == 0x2240000]
    return bool(ends) and any(d.get(0x210) in (0x10000, 0x394000) for d in draws[ends[-1]:])


RENDER_TARGETS = (0x10000, 0x394000, 0xF50000, 0xCC0000, 0x2150000, 0x2280000, 0x22C0000, 0x2240000,
                  0x2300000, 0x1660000, 0x1680000)


def history_reads(draws):
    """Render targets a draw samples before any draw of the dumped frame has written them: the
    previous frame's picture, read back. The first draw of a list can also be a half frame's tail, so
    read this on complete frames only."""
    written, out = set(), []
    for d in draws:
        for unit in range(16):
            base = 0x1A00 + 0x20 * unit
            if d.get(base + 0xC, 0) & 0x80000000:
                offset = d.get(base, 0)
                if offset in RENDER_TARGETS and offset not in written and offset not in out:
                    out.append(offset)
        written.add(d.get(0x210, 0))
    return [hex(x) for x in out]


def zoom_draws(draws):
    out = []
    for number, d in enumerate(draws):
        if d.get(0x8E4, 0) & ~0xF == ZOOM_FP:
            size = d["c"].get(467)
            floats = struct.unpack(">4f", struct.pack(">4I", *size)) if size else None
            out.append((number, tuple(round(x, 5) for x in floats[:2]) if floats else None))
    return out


def main():
    rdl = load("rsx_draw_list", "rsx-draw-list.py")
    for directory in sys.argv[1:]:
        for path in sorted(glob.glob(directory + "/*.json")):
            stem = os.path.basename(path)[:-5]
            try:
                draws, _ = rdl.draws(directory, stem)
            except Exception as error:
                print("%s/%s: unreadable (%s)" % (directory, stem, error))
                continue
            funk = json.load(open(path)).get("funk_0x50_0x6c")
            inputs = (funk[2], funk[5]) if funk else None
            print("%-24s %4d draws complete=%-5s funk(P,E)=%s zoom=%s history_reads=%s" % (
                os.path.basename(directory.rstrip("/")) + "/" + stem, len(draws), complete(draws), inputs,
                zoom_draws(draws), history_reads(draws)))


if __name__ == "__main__":
    main()
