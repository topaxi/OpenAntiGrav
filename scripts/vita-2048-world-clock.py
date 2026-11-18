#!/usr/bin/env python3
"""Find the scenery clock of Wipeout 2048 in a running Vita3K: floats shared by many objects that advance together.

    export HOME=<scratch>/vhome SDL_AUDIODRIVER=dummy OAG_VITA3K_CACHE=<scratch>/vc OAG_VITA3K_CONFIG=<scratch>/vc/config.yml
    OAG_VITA3K_DISPLAY=95 XDG_CONFIG_HOME=<scratch>/profile/cfg \\
    uv run --with numpy python3 scripts/vita-2048-world-clock.py --out <scratch>/clock --seconds 5 [--event "2048 - Event 2"]

Walks to a race with `vita-2048-pair.py`'s own walk (the Vita3K is this process's child, so `/proc/<pid>/mem` opens), then
dumps every readable-writable mapping of the guest window twice, `--seconds` apart, and prints the groups of words that held one
value at the first dump (at least `--min-copies` of them, 0 to 100000 as an f32), all moved to one other value by the second.
`MeshImporter`'s counterpart on HD keeps its last-seen world time in every animated node (`docs/ghidra/functions/ps3-hdfury-eu/gantry-clock.md`),
so a clock shared by tens of nodes is that stamp. Prints the groups, most copies first, and writes them as JSON.
"""

import argparse
import importlib.util
import json
import os
import signal
import sys
import time
from pathlib import Path
from types import SimpleNamespace

HERE = Path(__file__).resolve().parent


def load_pair():
    spec = importlib.util.spec_from_file_location("vita_2048_pair", HERE / "vita-2048-pair.py")
    mod = importlib.util.module_from_spec(spec)
    sys.modules["vita_2048_pair"] = mod
    spec.loader.exec_module(mod)
    return mod


def regions(pair, pid):
    out = []
    for line in open("/proc/%d/maps" % pid):
        span, perms = line.split()[:2]
        lo, hi = (int(v, 16) for v in span.split("-"))
        if perms.startswith("rw") and pair.HOST_WINDOW[0] <= lo < pair.HOST_WINDOW[1]:
            out.append((lo, hi))
    return out


def dump(pid, spans, chunk=1 << 24):
    mem = open("/proc/%d/mem" % pid, "rb", 0)
    out = {}
    for lo, hi in spans:
        for start in range(lo, hi, chunk):
            try:
                mem.seek(start)
                out[start] = mem.read(min(chunk, hi - start))
            except OSError:
                continue
    return out


def main():
    import numpy as np
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--node", type=int, nargs=2, default=(510, 340), metavar=("X", "Y"),
                    help="the node to tap: the HD Campaign's first (Vineta K Race), open on a fresh save")
    ap.add_argument("--boot-timeout", type=float, default=240.0)
    ap.add_argument("--seconds", type=float, default=5.0)
    ap.add_argument("--min-copies", type=int, default=6)
    ap.add_argument("--display", type=int, default=int(os.environ.get("OAG_VITA3K_DISPLAY", 0)) or None)
    ap.add_argument("--keep-running", action="store_true", help="leave Vita3K up afterwards")
    args = ap.parse_args()
    pair = load_pair()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    ns = SimpleNamespace(display=args.display, socket="wayland-oag%d" % args.display, patch_hull_shake=False, boot_timeout=args.boot_timeout,
                         load_timeout=240.0, game_mode="hd", rename_track=None, hop=[], node=tuple(args.node),
                         node_recentred=None, node_template=None, node_pick=0, load_wait=25.0, zone_capture=0.0, vita_pid=None)
    s = pair.Session(args.display, out / "frames")
    pair.boot_and_walk(s, ns)
    pid = ns.vita_pid
    print("vita3k pid", pid, flush=True)
    spans = regions(pair, pid)
    print("%d rw spans, %.0f MiB" % (len(spans), sum(h - l for l, h in spans) / 2**20), flush=True)
    os.kill(pid, signal.SIGSTOP)
    a = dump(pid, spans)
    os.kill(pid, signal.SIGCONT)
    t0 = time.monotonic()
    time.sleep(args.seconds)
    os.kill(pid, signal.SIGSTOP)
    b = dump(pid, spans)
    os.kill(pid, signal.SIGCONT)
    wall = time.monotonic() - t0
    rows = []
    for start, da in a.items():
        db = b.get(start)
        if db is None or len(db) != len(da):
            continue
        n = len(da) // 4
        fa = np.frombuffer(da[: n * 4], dtype="<f4")
        fb = np.frombuffer(db[: n * 4], dtype="<f4")
        with np.errstate(all="ignore"):
            m = np.isfinite(fa) & np.isfinite(fb) & (fa > 0) & (fa < 1e5) & (fb > fa) & (fb - fa < 4 * args.seconds + 5)
        idx = np.nonzero(m)[0]
        for i in idx:
            rows.append((start + 4 * int(i) - pair.GUEST_BASE, float(fa[i]), float(fb[i])))
    groups = {}
    for addr, va, vb in rows:
        groups.setdefault((round(va, 4), round(vb, 4)), []).append(addr)
    top = sorted(((k, v) for k, v in groups.items() if len(v) >= args.min_copies), key=lambda kv: -len(kv[1]))
    print("wall %.2f s, %d changed words, %d groups of >= %d copies" % (wall, len(rows), len(top), args.min_copies))
    res = []
    for (va, vb), addrs in top[:25]:
        print("%9.4f -> %9.4f  delta %.4f  copies %4d  e.g. %s" % (va, vb, vb - va, len(addrs), " ".join("%#x" % x for x in addrs[:5])))
        res.append({"a": va, "b": vb, "delta": vb - va, "copies": len(addrs), "addresses": addrs[:64]})
    clocks = [g for g in top if g[0][0] > 1.0 and abs((g[0][1] - g[0][0]) - wall) < 0.5 and len(g[1]) >= 20]
    if clocks:
        (va, vb), addrs = clocks[0]
        print("clock group: %d copies, first %#x; looking for the singleton(s) holding %.3f and the node around %#x" % (len(addrs), addrs[0], vb, addrs[0]))
        members = set(addrs)
        singles = []
        for start, db in b.items():
            n = len(db) // 4
            fb = np.frombuffer(db[: n * 4], dtype="<f4")
            with np.errstate(all="ignore"):
                hit = np.nonzero(np.abs(fb - vb) < 0.08)[0]
            for i in hit:
                a = start + 4 * int(i) - pair.GUEST_BASE
                if a not in members:
                    singles.append((a, float(fb[i])))
        print("%d words within 0.08 of %.3f outside the group" % (len(singles), vb))
        for a, v in singles[:40]:
            print("  %#x %.4f" % (a, v))
        node = addrs[0] - 0x80
        base = pair.GUEST_BASE + node
        mem = open("/proc/%d/mem" % pid, "rb", 0)
        mem.seek(base)
        raw = mem.read(0x200)
        import struct
        print("node bytes from %#x (first copy minus 0x80):" % node)
        for off in range(0, 0x200, 16):
            w = struct.unpack("<4I", raw[off:off + 16]); f = struct.unpack("<4f", raw[off:off + 16])
            print("  +%03x %08x %08x %08x %08x   %s" % (off, *w, " ".join("%.4g" % x for x in f)))
    (out / "groups.json").write_text(json.dumps({"wall_s": wall, "groups": res}, indent=1))
    if not args.keep_running:
        os.kill(pid, signal.SIGTERM)


if __name__ == "__main__":
    main()
