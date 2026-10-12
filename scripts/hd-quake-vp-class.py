#!/usr/bin/env python3
"""Which vertex-program class (`Static` or `StaticQuake`) each draw of a dumped RSX frame ran under.

    python3 scripts/hd-quake-vp-class.py <dir> <stem>...

Reads `<dir>/<stem>-4*.bin` (`scripts/rpcs3-hd-whiteout.py --quake --no-targets`,
`HOOK_RING_ONLY=1`), collects the vertex-program words uploaded before each draw
(method `0xb80`, reset by `0x1e9c`) and renders them with `ps3-microcode.py`'s own
decoder. A `StaticQuake` program is the one with a `COS` in it (its
displacement, `docs/ghidra/functions/ps3-hdfury-eu/quake.md`); no `Static` program
has one. Prints, per render target, how many draws ran each class and the values
of the four Quake registers (`quakePointB` 463, `quakePointA` 464, `quakeOffset`
465, `quakeTrackUpNormal` 462) the `StaticQuake` ones held.
"""
import collections
import glob
import importlib.util
import os
import struct
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import rsx_fifo as r  # noqa: E402

spec = importlib.util.spec_from_file_location("mc", os.path.join(HERE, "ps3-microcode.py"))
mc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mc)


def floats(words):
    return tuple(round(struct.unpack(">f", struct.pack(">I", w))[0], 3) for w in words)


def is_quake(words):
    for i in range(0, len(words) - 3, 4):
        try:
            text = mc.vp_render(*words[i:i + 4])
        except Exception:
            continue
        if "COS" in text:
            return True
    return False


def classify(directory, stem):
    mem = r.Mem.load(sorted(glob.glob("%s/%s-4*.bin" % (directory, stem))))
    walk = r.Walk(mem)
    regs, draws, upload, current = {}, [], [], ()
    for pos, meth, noinc, args in walk.run(0x1000):
        if (mem.word(pos) >> 13) & 7:
            continue
        if meth == 0x1e9c:
            upload = []
        if meth == 0xb80:
            upload = upload + list(args)
            current = tuple(upload)
        if meth == 0x1efc:
            if len(args) >= 5:
                regs.setdefault("c", {})
                base = args[0]
                for k in range((len(args) - 1) // 4):
                    regs["c"][base + k] = args[1 + 4 * k: 5 + 4 * k]
            continue
        for i, v in enumerate(args):
            regs[meth if noinc else meth + 4 * i] = v
        if meth in (0x1814, 0x1824):
            draws.append((current, regs.get(0x210), regs.get(0x8e4), dict(regs.get("c", {}))))
    return draws


def main():
    directory = sys.argv[1]
    for stem in sys.argv[2:]:
        draws = classify(directory, stem)
        cache = {}
        per = collections.Counter()
        waves = collections.Counter()
        for words, target, fp, consts in draws:
            if words not in cache:
                cache[words] = is_quake(words)
            q = cache[words]
            per[(target, "StaticQuake" if q else "other")] += 1
            if q and target == 0xF50000:
                waves[tuple(floats(consts[k]) if k in consts else None for k in (463, 464, 465, 462))] += 1
        print("%s: %d draws, %d distinct programs, %d StaticQuake" % (
            stem, len(draws), len(cache), sum(1 for v in cache.values() if v)))
        for (target, kind), n in sorted(per.items(), key=lambda kv: -kv[1])[:8]:
            print("   target %s %-11s %d" % (hex(target) if target is not None else None, kind, n))
        for key, n in waves.most_common(4):
            print("   scene-target StaticQuake draws x%d: B %s A %s offset %s up %s" % ((n,) + key))


if __name__ == "__main__":
    main()
