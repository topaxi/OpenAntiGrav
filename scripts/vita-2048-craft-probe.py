#!/usr/bin/env python3
"""Probe the craft array of Wipeout 2048 in a running Vita3K: which body is the player's, and does a write to it move the camera.

    export HOME=<scratch>/vhome SDL_AUDIODRIVER=dummy OAG_VITA3K_CACHE=<scratch>/vc OAG_VITA3K_CONFIG=<scratch>/vc/config.yml \\
        XDG_CACHE_HOME=<scratch>/vhome/.cache
    OAG_VITA3K_DISPLAY=96 XDG_CONFIG_HOME=<scratch>/profile/cfg \\
    uv run --with numpy python3 scripts/vita-2048-craft-probe.py --out <scratch>/probe --stages survey,poke

Walks to a race with `vita-2048-pair.py`'s walk (Vita3K is this process's child, so `/proc/<pid>/mem` opens), then runs the stages
named by `--stages`, each on the same boot, printing and writing `probe.json`:

    survey   the eight ships of the craft array: role word, body pointer, position and velocity read through `*(ship+0x5f84)`, twice
             `--gap` seconds apart (the guest stopped for each read), next to every drawn-hull matrix in memory.
    poke     for `--candidates` (ship indices, default all), write +20 y into the body's position with the guest stopped, let it run 0.3 s
             and grab a frame; a body the camera follows shifts the view.
"""

import argparse
import importlib.util
import json
import os
import signal
import struct
import sys
import time
from pathlib import Path
from types import SimpleNamespace

HERE = Path(__file__).resolve().parent

CRAFT_ARRAY = 0x818A6BF8
OFF_BODY = 0x5F84
OFF_ROLE = 0x7060
B_POS = 0x38
B_VEL = 0x90


def load_pair():
    spec = importlib.util.spec_from_file_location("vita_2048_pair", HERE / "vita-2048-pair.py")
    mod = importlib.util.module_from_spec(spec)
    sys.modules["vita_2048_pair"] = mod
    spec.loader.exec_module(mod)
    return mod


class Guest:
    def __init__(self, pair, pid):
        self.pair, self.pid = pair, pid
        self.mem = open("/proc/%d/mem" % pid, "r+b", 0)

    def read(self, addr, n):
        self.mem.seek(self.pair.GUEST_BASE + addr)
        return self.mem.read(n)

    def write(self, addr, data):
        self.mem.seek(self.pair.GUEST_BASE + addr)
        self.mem.write(data)

    def u32(self, addr):
        return struct.unpack("<I", self.read(addr, 4))[0]

    def f32s(self, addr, n):
        return struct.unpack("<%df" % n, self.read(addr, 4 * n))

    def stop(self):
        os.kill(self.pid, signal.SIGSTOP)

    def cont(self):
        os.kill(self.pid, signal.SIGCONT)


def read_ships(g):
    ships = []
    for i, ptr in enumerate(struct.unpack("<8I", g.read(CRAFT_ARRAY, 32))):
        rec = {"index": i, "ship": ptr}
        if ptr:
            body = g.u32(ptr + OFF_BODY)
            rec.update(role=g.u32(ptr + OFF_ROLE), body=body)
            if body:
                rec["pos"] = g.f32s(body + B_POS, 3)
                rec["vel"] = g.f32s(body + B_VEL, 3)
                rec["rows"] = g.f32s(body + 0x08, 12)
        ships.append(rec)
    return ships


def survey(g, pair, gap, out):
    g.stop()
    a = read_ships(g)
    hulls = pair.scan_hull_matrices(g.pid)
    g.cont()
    time.sleep(gap)
    g.stop()
    b = read_ships(g)
    g.cont()
    for ra, rb in zip(a, b):
        if "pos" in ra and "pos" in rb:
            ra["pos2"] = rb["pos"]
            ra["moved"] = sum((p - q) ** 2 for p, q in zip(ra["pos"], rb["pos"])) ** 0.5
    top = [{"count": n, "pos": list(p), "rows": list(r)} for n, p, r in hulls[:40]]
    res = {"ships": a, "hulls": top}
    for r in a:
        print("ship %d %#x role %s body %#x pos %s vel %s moved %s" % (
            r["index"], r["ship"], hex(r.get("role", 0)), r.get("body", 0),
            ["%.1f" % v for v in r.get("pos", ())], ["%.1f" % v for v in r.get("vel", ())], "%.2f" % r["moved"] if "moved" in r else "-"))
    for h in top[:12]:
        print("hull x%-3d pos %s" % (h["count"], ["%.1f" % v for v in h["pos"]]))
    (out / "survey.json").write_text(json.dumps(res, indent=1))
    return res


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--stages", default="survey")
    ap.add_argument("--node", type=int, nargs=2, default=(270, 68), metavar=("X", "Y"))
    ap.add_argument("--rename-track", nargs=2, default=None, metavar=("OLD", "NEW"))
    ap.add_argument("--game-mode", default="single")
    ap.add_argument("--boot-timeout", type=float, default=240.0)
    ap.add_argument("--gap", type=float, default=1.0)
    ap.add_argument("--candidates", default="0,1,2,3,4,5,6,7")
    ap.add_argument("--display", type=int, default=int(os.environ.get("OAG_VITA3K_DISPLAY", 0)) or None)
    ap.add_argument("--keep-running", action="store_true")
    args = ap.parse_args()
    pair = load_pair()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    ns = SimpleNamespace(display=args.display, socket="wayland-oag%d" % args.display, patch_hull_shake=False, boot_timeout=args.boot_timeout,
                         load_timeout=240.0, game_mode=args.game_mode, rename_track=args.rename_track, hop=[], node=tuple(args.node),
                         node_recentred=None, node_template=None, node_pick=0, load_wait=25.0, zone_capture=0.0, vita_pid=None)
    s = pair.Session(args.display, out / "frames")
    for name in ("shot", "tap", "key"):
        def beat(*a, _f=getattr(s, name), _n=name, **k):
            print("walk %s %s" % (_n, a[:1]), flush=True)
            return _f(*a, **k)
        setattr(s, name, beat)
    try:
        pair.boot_and_walk(s, ns)
    except BaseException:
        if ns.vita_pid:
            os.kill(ns.vita_pid, signal.SIGKILL)
        raise
    pid = ns.vita_pid
    print("vita3k pid", pid, flush=True)
    g = Guest(pair, pid)
    result = {"pid": pid}
    try:
        for stage in args.stages.split(","):
            if stage == "survey":
                result["survey"] = survey(g, pair, args.gap, out)
                s.shot("survey.png")
            elif stage == "poke":
                ships = read_ships(g)
                res = []
                for idx in (int(c) for c in args.candidates.split(",")):
                    r = ships[idx]
                    if "pos" not in r:
                        continue
                    before = s.shot("poke-%d-before.png" % idx)
                    g.stop()
                    x, y, z = g.f32s(r["body"] + B_POS, 3)
                    g.write(r["body"] + B_POS, struct.pack("<3f", x, y + 20.0, z))
                    g.cont()
                    time.sleep(0.3)
                    after = s.shot("poke-%d-after.png" % idx)
                    g.stop()
                    now = g.f32s(r["body"] + B_POS, 3)
                    g.cont()
                    res.append({"index": idx, "wrote": [x, y + 20.0, z], "read_back": now})
                    print("poke ship %d: wrote y %.1f, read back %.1f" % (idx, y + 20.0, now[1]), flush=True)
                result["poke"] = res
    finally:
        (out / "probe.json").write_text(json.dumps(result, indent=1))
        if not args.keep_running:
            os.kill(pid, signal.SIGKILL)


if __name__ == "__main__":
    main()
