#!/usr/bin/env python3
"""Live knockouts of the lighting object's terms on Omega's hull, on the `wipeout_pc` native port (1.07 eboot).

Walks to a grid as `wipeout-pc-engine-light-live.py` does, switches the engine light off (the producer returns at once),
finds the circuit's lighting object in the process by the authored floats of its `EnvSettings` (the four-float run
`Sky height offset, Sky scale, Sky brightness, Ship diffuse brightness`, then the ambient and the sun colour triples),
and grabs frames with one term forced at a time, each undone before the next:

    uv run --with evdev python3 scripts/emu-run.py --silence 200 --total 560 -- \
        python3 scripts/wipeout-pc-hull-tone-live.py --out data/scratch/<lane>/live --game <game dir> --port <port checkout> \
        --track-steps 8 --sky-brightness 2.264 --ship-diffuse 0.25 --ambient 0.309804,0.258824,0.713726 \
        --sun-diffuse 4,3.607843,2.164706

The values are the circuit's `track.EnvSettings` ones (`omega-hull-tone` report); the scan is by value, so a wrong value
finds nothing and the run stops. Every write is to this run's own process. `docs/ghidra/functions/ps4-omega-eu/hull-tone.md`.
"""

import argparse
import importlib.util
import json
import os
import struct
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("port_pair", HERE / "wipeout-pc-omega-pair.py")
pp = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pp)
spec2 = importlib.util.spec_from_file_location("engine_light_live", HERE / "wipeout-pc-engine-light-live.py")
el = importlib.util.module_from_spec(spec2)
spec2.loader.exec_module(el)


def floats(text):
    return [float(x) for x in text.split(",")]


def regions(pid):
    """Anonymous or heap read-write regions under 1 GiB: the lighting object is a plain allocation."""
    out = []
    for line in open("/proc/%d/maps" % pid):
        parts = line.split()
        lo, hi = (int(x, 16) for x in parts[0].split("-"))
        perms = parts[1]
        name = parts[5] if len(parts) > 5 else ""
        if "r" not in perms or "w" not in perms:
            continue
        if name and name != "[heap]":
            continue
        if hi - lo > (1 << 30):
            continue
        out.append((lo, hi))
    return out


def scan(pid, pattern):
    fd = os.open("/proc/%d/mem" % pid, os.O_RDONLY)
    hits = []
    for lo, hi in regions(pid):
        pos = lo
        while pos < hi:
            n = min(64 << 20, hi - pos)
            try:
                blob = os.pread(fd, n, pos)
            except OSError:
                break
            i = blob.find(pattern)
            while i != -1:
                hits.append(pos + i)
                i = blob.find(pattern, i + 1)
            pos += n
    os.close(fd)
    return hits


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--game", required=True)
    ap.add_argument("--port", required=True)
    ap.add_argument("--track-steps", type=int, default=8)
    ap.add_argument("--game-mode", default="hd")
    ap.add_argument("--class-steps", type=int, default=0)
    ap.add_argument("--hud-labels-ref", type=Path)
    ap.add_argument("--hud-timer-ref", type=Path)
    ap.add_argument("--load-wait", type=float, default=12)
    ap.add_argument("--load-wait2", type=float, default=14)
    ap.add_argument("--settle", type=float, default=12)
    ap.add_argument("--sky-brightness", type=float, required=True)
    ap.add_argument("--ship-diffuse", type=float, required=True)
    ap.add_argument("--ambient", type=floats, required=True)
    ap.add_argument("--sun-diffuse", type=floats, required=True)
    ap.add_argument("--n", type=int, default=3, help="frames per state")
    args = ap.parse_args()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    rig = pp.Rig(out / "port", args.game, args.port, "hull-tone")
    if pp.pad_seat.own_pads(rig.seat):
        sys.exit("seat %s already has a pad" % rig.seat)
    note = {}
    try:
        pick, seen = pp.capture_original(rig, args, out)
        t_grid = time.time()
        note["grid"] = {"pick": str(pick), **seen}
        pid = el.game_pid(rig.pids["port"])
        if pid is None:
            sys.exit("STALLED: no descendant maps 0x800000000")
        mem = el.Mem(pid)
        if mem.read(el.PRODUCER, len(el.PROLOGUE)) != el.PROLOGUE:
            sys.exit("STALLED: the producer's bytes are not at the mapped address")
        mem.write(el.PRODUCER, b"\xc3")
        time.sleep(max(0, args.settle - (time.time() - t_grid)))
        fd = os.open("/proc/%d/mem" % pid, os.O_RDWR)

        key = struct.pack("<4f", 0.0, 1.0, args.sky_brightness, args.ship_diffuse)
        hits = scan(pid, key)
        note["lighting_hits"] = [hex(h) for h in hits]
        print("[hull] four-float key hits: %s" % note["lighting_hits"], flush=True)
        amb = struct.pack("<3f", *args.ambient)
        sun = struct.pack("<3f", *args.sun_diffuse)
        note["ambient_hits"] = [hex(h) for h in scan(pid, amb)]
        note["sun_hits"] = [hex(h) for h in scan(pid, sun)]
        print("[hull] ambient hits %d sun hits %d" % (len(note["ambient_hits"]), len(note["sun_hits"])), flush=True)
        if not hits:
            sys.exit("STALLED: the lighting object was not found by value")
        base = hits[0]
        dump = os.pread(fd, 0x40, base - 0x20)
        note["around"] = list(struct.unpack("<16f", dump))

        def put(addr, data):
            os.pwrite(fd, data, addr)

        def grab(tag):
            note.setdefault("frames", {})[tag] = el.grabs(rig, out, tag, args.n)

        image = [h for h in hits if h >= 0x800000000]
        amb_img = [int(a, 16) for a in note["ambient_hits"] if int(a, 16) >= 0x800000000]
        sun_img = [int(a, 16) for a in note["sun_hits"] if int(a, 16) >= 0x800000000]
        note["image_hits"] = {"key": [hex(x) for x in image], "ambient": [hex(x) for x in amb_img], "sun": [hex(x) for x in sun_img]}
        first = min(amb_img + sun_img)
        note["lighting_block"] = list(struct.unpack("<48f", os.pread(fd, 48 * 4, first - 0x20)))
        note["lighting_block_at"] = hex(first - 0x20)

        def f3(addr):
            return struct.unpack("<3f", os.pread(fd, 12, addr))

        def scaled(addrs, k):
            return [(a, [x * k for x in f3(a)]) for a in addrs]

        def f1(addr):
            return struct.unpack("<f", os.pread(fd, 4, addr))[0]

        derived = {
            "ambient_x_ship": [x * args.ship_diffuse for x in args.ambient],
            "sun_x_1.6": [x * 1.6 for x in args.sun_diffuse],
            "sun_x_1.6_x_ship": [x * 1.6 * args.ship_diffuse for x in args.sun_diffuse],
        }
        note["derived_hits"] = {k: [hex(h) for h in scan(pid, struct.pack("<3f", *v))][:12] for k, v in derived.items()}
        print("[hull] derived hits %s" % {k: len(v) for k, v in note["derived_hits"].items()}, flush=True)
        note["fields"] = {n: f1(base + o) for n, o in (("ship", 12), ("ship_shadow", 16), ("refl", 20), ("chrome", 24), ("spec_pow", 28), ("spec_chrome_pow", 32))}
        states = [
            ("s0-baseline", []),
            ("s1-ship0", [(base + 12, [0.0])]),
            ("s1-ship1", [(base + 12, [1.0])]),
            ("s2-shadow0", [(base + 16, [0.0])]),
            ("s2-shadow1", [(base + 16, [1.0])]),
            ("s2-shadow4", [(base + 16, [4.0])]),
            ("s3-specpow4", [(base + 28, [4.0])]),
            ("s3-specpow1", [(base + 28, [1.0])]),
            ("s4-refl0", [(base + 20, [0.0])]),
            ("s4-chrome0", [(base + 24, [0.0])]),
            ("s4-refl0-chrome0", [(base + 20, [0.0]), (base + 24, [0.0])]),
            ("s5-suncolour0", [(base - 24, [0.0, 0.0, 0.0])]),
            ("s6-sun4810-0", scaled(sun_img[1:], 0.0)),
            ("s7-restored", []),
        ]
        for k, name in enumerate(derived):
            hs = [int(h, 16) for h in note["derived_hits"][name] if int(h, 16) >= 0x800000000]
            if hs:
                states.insert(-1, ("s8-zero-" + name, [(h, [0.0, 0.0, 0.0]) for h in hs]))
        for tag, writes in states:
            keep = [(a, os.pread(fd, 4 * len(v), a)) for a, v in writes]
            for a, v in writes:
                put(a, struct.pack("<%df" % len(v), *v))
            time.sleep(0.4)
            grab(tag)
            for a, raw in keep:
                put(a, raw)
    finally:
        (out / "note.json").write_text(json.dumps(note, indent=1))
        rig.stop()
    print("[hull] done: %s" % (out / "note.json"), flush=True)


if __name__ == "__main__":
    main()
