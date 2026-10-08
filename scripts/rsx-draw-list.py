#!/usr/bin/env python3
"""Print one dumped frame's draws in order, with the state each was issued under.

    python3 scripts/rsx-draw-list.py <dir> <stem> [--from N] [--to M]

`<dir>/<stem>-4*.bin` are the spans `rpcs3_draw_hook.py` writes. One line per
draw: its number, begin mode, vertex count, fragment program, blend
(enable, src, dst, equation), alpha test (enable, func), cull (enable, face),
depth (test, mask, func), colour mask, viewport and the first four bound
textures. Only subchannel-0 writes are kept: the blits on subchannels 3 to 7
reuse the method numbers `0x300`-`0x30c` and would overwrite the alpha-test
and blend registers. Register numbers are read off the stream, not from a gcm
header (`0x183c` as cull enable is inferred from its 0/1 spread). A line `SURF` marks a render-target switch
and `CLEAR` a clear, so passes read off the list.
"""
import argparse
import glob
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import rsx_fifo as r  # noqa: E402

SURFACE = (0x0208, 0x020c, 0x0210, 0x0214, 0x0218, 0x0194, 0x0198, 0x019c, 0x01a0, 0x01a4)


def draws(directory, stem):
    mem = r.Mem.load(sorted(glob.glob("%s/%s-4*.bin" % (directory, stem))))
    regs, out, events = {}, [], []
    walk = r.Walk(mem)
    for pos, meth, noinc, args in walk.run(0x1000):
        if (mem.word(pos) >> 13) & 7:
            continue
        if meth in (0x1efc, 0xb80):
            if meth == 0x1efc and len(args) >= 5:
                regs.setdefault("c", {})
                base = args[0]
                for k in range((len(args) - 1) // 4):
                    regs["c"][base + k] = args[1 + 4 * k: 5 + 4 * k]
            continue
        for i, v in enumerate(args):
            regs[meth if noinc else meth + 4 * i] = v
        if meth == 0x1d94:
            events.append((len(out), "CLEAR %08x" % args[0]))
        if meth in (0x0208, 0x0194, 0x0a00) and len(args) >= 1 and meth != 0x0a00:
            events.append((len(out), "SURF %s" % " ".join("%08x" % regs.get(m, 0) for m in SURFACE)))
        if meth in (0x1814, 0x1824):
            snap = dict(regs)
            snap["c"] = dict(regs.get("c", {}))
            snap["n"] = len(out)
            out.append(snap)
    return out, events


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("dir")
    ap.add_argument("stem")
    ap.add_argument("--from", dest="lo", type=int, default=0)
    ap.add_argument("--to", dest="hi", type=int, default=10 ** 9)
    a = ap.parse_args()
    ds, events = draws(a.dir, a.stem)
    ev = {}
    for at, text in events:
        ev.setdefault(at, []).append(text)
    print("%d draws" % len(ds))
    for d in ds:
        n = d["n"]
        for text in ev.get(n, []):
            print("      ----", text)
        if not (a.lo <= n <= a.hi):
            continue
        tex = ["%08x" % d.get(0x1a00 + 0x20 * u, 0) for u in range(4) if d.get(0x1a00 + 0x20 * u + 0xc, 0) & 0x80000000]
        cnt = ((d.get(0x1824, 0) >> 24) & 0xff) + 1 if 0x1824 in d else 0
        print("%4d tgt %08x idx%-4d fp %08x bl %d %x/%x eq %x at %d/%x cull %d/%x dep %d/%d/%x cm %08x vp %08x/%08x tex %s" % (
            n, d.get(0x210, 0), cnt, d.get(0x8e4, 0), d.get(0x310, 0) & 1, d.get(0x314, 0), d.get(0x318, 0), d.get(0x320, 0),
            d.get(0x304, 0) & 1, d.get(0x308, 0), d.get(0x183c, 0) & 1, d.get(0x1830, 0),
            d.get(0xa74, 0) & 1, d.get(0xa70, 0) & 1, d.get(0xa6c, 0), d.get(0x324, 0),
            d.get(0xa00, 0), d.get(0xa04, 0), ",".join(tex)))


if __name__ == "__main__":
    main()
