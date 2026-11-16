#!/usr/bin/env python3
"""Live read of the hull's bound pixel constants on Omega, on the `wipeout_pc` native port (1.07 eboot).

The race hull's pixel program (`diffuse_with_specular_from_alpha_n_vcol`, the `shCoeffs` variants) binds one constant
block per draw: `Chromeness, shCoeffs[28], probeOffsetAndDiffuse[3], liveLighting0specular, liveLighting0diffuse,
liveLighting0direction, ...`. This finds those blocks by the value of `liveLighting0diffuse` (the circuit's
`Sun diffuse colour * Sun diffuse intensity`), dumps the floats around every copy, then repeats the dump and grabs frames
with `Ship diffuse brightness` written to other values and with the live sun colour zeroed:

    uv run --with evdev python3 scripts/emu-run.py --silence 200 --total 560 -- \
        python3 scripts/wipeout-pc-hull-probe-live.py --out data/scratch/<lane>/live --game <game dir> --port <port checkout> \
        --track-steps 8 --sky-brightness 2.264 --ship-diffuse 0.25 --sun-diffuse 4,3.607843,2.164706 --sun-intensity 1.6

Every write is to this run's own process and is undone. `docs/ghidra/functions/ps4-omega-eu/hull-tone.md`, "The bound
constants".
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

HERE = Path(__file__).resolve().parent


def load(name, file):
    spec = importlib.util.spec_from_file_location(name, HERE / file)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


pp = load("port_pair", "wipeout-pc-omega-pair.py")
el = load("engine_light_live", "wipeout-pc-engine-light-live.py")
ht = load("hull_tone_live", "wipeout-pc-hull-tone-live.py")

BEFORE = 0x140  # bytes dumped before a `liveLighting0diffuse` copy (its dword 40 back past dword 0)
AFTER = 0x60
LIVE_SUN = 0x800EB4810
GUEST_TOP = 0x2000000000  # the same memory is mapped again high in the host's range


def guest_regions(pid):
    """The port's guest direct memory (`/memfd:gow3-guest-memory` at its guest addresses): the per-draw constants live here."""
    out = []
    for line in open("/proc/%d/maps" % pid):
        parts = line.split()
        lo, hi = (int(x, 16) for x in parts[0].split("-"))
        if len(parts) > 5 and "gow3-guest-memory" in parts[5] and "r" in parts[1] and hi <= GUEST_TOP:
            out.append((lo, hi))
    return out


def scan_guest(pid, pattern):
    fd = os.open("/proc/%d/mem" % pid, os.O_RDONLY)
    hits = []
    for lo, hi in guest_regions(pid):
        pos = lo
        while pos < hi:
            n = min(64 << 20, hi - pos)
            try:
                blob = os.pread(fd, n, pos)
            except OSError:
                pos += n
                continue
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
    ap.add_argument("--sun-diffuse", type=ht.floats, required=True)
    ap.add_argument("--sun-intensity", type=float, required=True)
    ap.add_argument("--n", type=int, default=4, help="frames per state")
    args = ap.parse_args()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    rig = pp.Rig(out / "port", args.game, args.port, "hull-probe")
    if pp.pad_seat.own_pads(rig.seat):
        sys.exit("seat %s already has a pad" % rig.seat)
    note = {"states": {}}

    def save():
        (out / "note.json").write_text(json.dumps(note))

    signal.signal(signal.SIGTERM, lambda *_: sys.exit("terminated"))
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
        hits = ht.scan(pid, key)
        print("[probe] lighting object hits: %s" % [hex(h) for h in hits], flush=True)
        if not hits:
            sys.exit("STALLED: the lighting object was not found by value")
        base = hits[0]
        sun16 = struct.pack("<3f", *[x * args.sun_intensity for x in args.sun_diffuse])

        def dump(tag):
            """Every copy of the sun constant with the floats around it, read as soon as found."""
            rows = []
            for h in scan_guest(pid, sun16):
                try:
                    blob = os.pread(fd, BEFORE + AFTER, h - BEFORE)
                except OSError:
                    continue
                rows.append({"at": hex(h), "floats": list(struct.unpack("<%df" % (len(blob) // 4), blob))})
            note["states"].setdefault(tag, {})["blocks"] = rows
            print("[probe] %s: %d sun copies" % (tag, len(rows)), flush=True)
            return rows

        def state(tag, writes, dumped=False):
            keep = [(a, os.pread(fd, len(raw), a)) for a, raw in writes]
            for a, raw in writes:
                os.pwrite(fd, raw, a)
            time.sleep(0.5)
            if dumped:
                dump(tag)
            else:
                note["states"].setdefault(tag, {})["frames"] = el.grabs(rig, out, tag, args.n)
            save()
            for a, raw in keep:
                os.pwrite(fd, raw, a)

        def f(*v):
            return struct.pack("<%df" % len(v), *v)

        ship, shadow = base + 12, base + 16
        zero_sun = (LIVE_SUN, f(0.0, 0.0, 0.0))
        # Frames first, all of them inside a few seconds: a scan of the guest memory takes long enough for the grid to move on.
        frames = [
            ("s0-baseline", []),
            ("s1-ship0", [(ship, f(0.0))]),
            ("s1-ship1", [(ship, f(1.0))]),
            ("s2-ship0-shadow0", [(ship, f(0.0)), (shadow, f(0.0))]),
            ("s3-sun0", [zero_sun]),
            ("s4-sun0-ship0-shadow0", [zero_sun, (ship, f(0.0)), (shadow, f(0.0))]),
            ("s5-restored", []),
        ]
        for tag, writes in frames:
            state(tag, writes)
        for tag, writes in (("d0-baseline", []), ("d1-ship0", [(ship, f(0.0))]), ("d1-ship1", [(ship, f(1.0))]), ("d1-ship4", [(ship, f(4.0))])):
            state(tag, writes, dumped=True)
    finally:
        save()
        rig.stop()
    print("[probe] done: %s" % (out / "note.json"), flush=True)


if __name__ == "__main__":
    main()
