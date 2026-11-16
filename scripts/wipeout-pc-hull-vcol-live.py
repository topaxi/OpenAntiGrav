#!/usr/bin/env python3
"""Live A/B of the hull's vertex colour, albedo and shadow on Omega, on the `wipeout_pc` native port (1.07 eboot).

Walks to a grid as `wipeout-pc-hull-probe-live.py` does, switches the engine light off, then grabs frames in states
that change the hull alone, so the adaptive exposure stays put:

- the hull's `VertexColour1` bytes written to white in the port's guest memory (the vertex buffers are found by the
  first bytes of each buffer, given in `--vb`);
- every `track.shprobes` record of the live manager (`g_sh_probes`, `+0x20` records, `+0x28` count) written flat
  (`sh[0] = 1`, the other eight zero), so the probe term is `kmix` alone;
- `Ship diffuse brightness` (`K`) and its `(shadow)` twin (`Ks`) at 0 or 1;
- the live sun colour zeroed (this one moves the whole scene and the exposure).

`--vb` is a text file, one buffer a line: `stride=<n> off=<n> count=<n> sig=<hex of the buffer's first bytes>`, `off`
the byte offset of the four colour bytes in a vertex; other `key=value` words on a line are ignored. The lines of
`docs/ghidra/functions/ps4-omega-eu/hull-tone.md`, "The vertex colour", came from a print in `oag_rcs`'s PS4 submesh
reader.

    uv run --with evdev python3 scripts/emu-run.py --silence 200 --total 560 -- \
        python3 scripts/wipeout-pc-hull-vcol-live.py --out data/scratch/<lane>/live --game <game dir> --port <port checkout> \
        --track-steps 8 --sky-brightness 2.264 --ship-diffuse 0.25 --vb <file>

Every write is to this run's own process and is undone.
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
hp = load("hull_probe_live", "wipeout-pc-hull-probe-live.py")

LIVE_SUN = 0x800EB4810
SH_MANAGER = 0x020084E8  # `g_sh_probes`
RECORD = 0x78


def buffers(path):
    out = []
    for line in Path(path).read_text().splitlines():
        words = dict(w.split("=", 1) for w in line.split() if "=" in w)
        if "sig" in words:
            out.append({"stride": int(words["stride"]), "off": int(words["off"]), "count": int(words["count"]), "sig": bytes.fromhex(words["sig"])})
    return out


class Guest:
    """Writes into the port's guest memory through its memfd: the low mapping `/proc/<pid>/mem` reaches is read-only."""

    def __init__(self, pid):
        self.maps = []
        for line in open("/proc/%d/maps" % pid):
            parts = line.split()
            if len(parts) > 5 and "gow3-guest-memory" in parts[5]:
                lo, hi = (int(x, 16) for x in parts[0].split("-"))
                self.maps.append((lo, hi, int(parts[2], 16)))
        self.fd = None
        for name in os.listdir("/proc/%d/fd" % pid):
            try:
                if "gow3-guest-memory" in os.readlink("/proc/%d/fd/%s" % (pid, name)):
                    self.fd = os.open("/proc/%d/fd/%s" % (pid, name), os.O_RDWR)
                    break
            except OSError:
                continue

    def write(self, addr, raw):
        """`raw` at `addr`, piece by piece: a buffer may span mappings."""
        done = 0
        while done < len(raw):
            at = addr + done
            for lo, hi, offset in self.maps:
                if lo <= at < hi:
                    n = min(len(raw) - done, hi - at)
                    os.pwrite(self.fd, raw[done : done + n], at - lo + offset)
                    done += n
                    break
            else:
                near = [(hex(lo), hex(hi)) for lo, hi, _ in self.maps if abs(lo - at) < (1 << 30)]
                raise OSError("%#x is not guest memory; mappings near it: %s" % (at, near))


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
    ap.add_argument("--vb", required=True)
    ap.add_argument("--n", type=int, default=3, help="frames per state")
    ap.add_argument("--pairs", type=int, default=2, help="shipped and white states, alternated this many times")
    ap.add_argument("--scan-lo", type=lambda v: int(v, 0), default=0, help="guest address to scan for the buffers from")
    ap.add_argument("--scan-hi", type=lambda v: int(v, 0), default=1 << 48, help="and up to (a whole scan takes 24 s, and the craft leaves the grid some 40 s after it)")
    args = ap.parse_args()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    wanted = buffers(args.vb)
    rig = pp.Rig(out / "port", args.game, args.port, "hull-vcol")
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

        hits = ht.scan(pid, struct.pack("<4f", 0.0, 1.0, args.sky_brightness, args.ship_diffuse))
        print("[vcol] lighting object hits: %s" % [hex(h) for h in hits], flush=True)
        if not hits:
            sys.exit("STALLED: the lighting object was not found by value")
        base = hits[0]
        ship, shadow = base + 12, base + 16

        def f(*v):
            return struct.pack("<%df" % len(v), *v)

        def grab(tag):
            note["states"].setdefault(tag, {})["frames"] = el.grabs(rig, out, tag, args.n)
            save()

        # The probe records, flat: `sh[0] = (1, 1, 1)`, the rest zero.
        guest = Guest(pid)
        if guest.fd is None:
            sys.exit("STALLED: the guest memory's descriptor was not found")
        manager = mem.read(SH_MANAGER, 0x30)
        records, count = struct.unpack_from("<QI", manager, 0x20)
        if not (1000 < count < 100000 and count % 3 == 0):
            # `g_sh_probes` holds the manager's address.
            manager = os.pread(fd, 0x30, struct.unpack_from("<Q", manager)[0])
            records, count = struct.unpack_from("<QI", manager, 0x20)
        note["manager"] = manager.hex()
        print("[vcol] probe records at %#x, count %d" % (records, count), flush=True)
        sh_keep = sh_flat = None
        if 0 < count < 100000 and count % 3 == 0:
            sh_keep = os.pread(fd, count * RECORD, records)
            flat = bytearray(sh_keep)
            one = f(1.0, 1.0, 1.0) + bytes(24 * 4)
            for k in range(count):
                flat[k * RECORD + 12 : (k + 1) * RECORD] = one
            sh_flat = bytes(flat)
        note["probes"] = {"records": hex(records), "count": count, "usable": sh_keep is not None}

        # The hull's vertex buffers, by their first bytes; one pass over the guest memory for all of them.
        t0 = time.time()
        found = {k: [] for k in range(len(wanted))}
        lead = {}
        for k, b in enumerate(wanted):
            lead.setdefault(b["sig"][:8], []).append(k)
        for lo, hi in hp.guest_regions(pid):
            lo, hi = max(lo, args.scan_lo), min(hi, args.scan_hi)
            pos = lo
            while pos < hi:
                n = min(64 << 20, hi - pos)
                try:
                    blob = os.pread(fd, n + 64, pos)
                except OSError:
                    pos += n
                    continue
                for head, ks in lead.items():
                    i = blob.find(head)
                    while i != -1 and i < n:
                        for k in ks:
                            sig = wanted[k]["sig"]
                            if blob[i : i + len(sig)] == sig:
                                found[k].append(pos + i)
                        i = blob.find(head, i + 1)
                pos += n
        note["buffers"] = [{"count": b["count"], "stride": b["stride"], "at": [hex(a) for a in found[k]]} for k, b in enumerate(wanted)]
        print("[vcol] %d of %d buffers found in %.1f s (%d copies)" % (sum(1 for v in found.values() if v), len(wanted), time.time() - t0, sum(len(v) for v in found.values())), flush=True)
        save()

        white, shipped = [], []
        for k, b in enumerate(wanted):
            for at in found[k]:
                raw = os.pread(fd, b["count"] * b["stride"], at)
                new = bytearray(raw)
                for v in range(b["count"]):
                    o = v * b["stride"] + b["off"]
                    new[o : o + 3] = b"\xff\xff\xff"
                white.append((at, bytes(new)))
                shipped.append((at, raw))

        def put(writes):
            for a, raw in writes:
                try:
                    os.pwrite(fd, raw, a)
                except OSError:
                    guest.write(a, raw)

        def state(tag, writes=(), vcol=False, flat=False):
            keep = [(a, os.pread(fd, len(raw), a)) for a, raw in writes]
            put(writes)
            if vcol:
                put(white)
            if flat and sh_flat:
                put([(records, sh_flat)])
            time.sleep(0.45)
            grab(tag)
            put(keep)
            if vcol:
                put(shipped)
            if flat and sh_flat:
                put([(records, sh_keep)])
            note["states"][tag]["t"] = round(time.time() - t_grid, 2)

        k0 = [(ship, f(0.0)), (shadow, f(0.0))]
        k1s0 = [(ship, f(1.0)), (shadow, f(0.0))]
        k0s1 = [(ship, f(0.0)), (shadow, f(1.0))]
        k1s1 = [(ship, f(1.0)), (shadow, f(1.0))]
        sun0 = [(LIVE_SUN, f(0.0, 0.0, 0.0))]
        # The control: the largest buffer's positions raised by half a unit. A frame that does not move says the
        # port draws from its own copy of the vertex data, and the white states prove nothing.
        big = max(range(len(wanted)), key=lambda k: wanted[k]["count"])
        raised = []
        for at in found[big]:
            raw = bytearray(os.pread(fd, wanted[big]["count"] * wanted[big]["stride"], at))
            for v in range(wanted[big]["count"]):
                o = v * wanted[big]["stride"] + 4
                raw[o : o + 4] = f(struct.unpack_from("<f", raw, o)[0] + 0.5)
            raised.append((at, bytes(raw)))
        for k in range(args.pairs):
            state("s%d-shipped" % k)
            state("w%d-white" % k, vcol=True)
        keep = [(a, os.pread(fd, len(raw), a)) for a, raw in raised]
        put(raised)
        time.sleep(0.45)
        grab("c-raised")
        put(keep)
        state("k0", k0)
        state("f-k1s1", k1s1, flat=True)
        state("f-k2s2", [(ship, f(2.0)), (shadow, f(2.0))], flat=True)
        state("f-k4s4", [(ship, f(4.0)), (shadow, f(4.0))], flat=True)
        state("f-k1s0", k1s0, flat=True)
        state("s8-shipped")
        state("r-sun0-k0", sun0 + k0)
        state("fr-sun0-k1s1", sun0 + k1s1, flat=True)
        state("r2-sun0-k0", sun0 + k0)
        state("s9-shipped")
    finally:
        save()
        rig.stop()
    print("[vcol] done: %s" % (out / "note.json"), flush=True)


if __name__ == "__main__":
    main()
