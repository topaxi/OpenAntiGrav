#!/usr/bin/env python3
"""Live knockouts of the panel-map road's sky sheen on the `wipeout_pc` native port (1.07 eboot).

Walks to a 2048-mode grid as `wipeout-pc-omega-pair.py` does. While the race intro loops (the circuit is loaded, the race not
started) it finds, in the guest's memory, the road material's values through the model's own parameter records (`!crc32(name)`,
a count of 1, and at `+0x10` a pointer to the float; one material's ten values are neighbours) and the frame constants the
program reads (the constant name table's entry is the hash with the value pointer at `+0x10`), applies `--pre` and only then
starts the race, so the grid frames are the pair's own pose with the change in. After `--settle` seconds (the camera at rest)
it grabs frames with one change at a time, each undone before the next:

    OAG_PAD_SEAT=3 uv run --with evdev python3 scripts/emu-run.py --silence 200 --total 560 -- \\
        python3 scripts/wipeout-pc-road-sheen-live.py --out data/scratch/<lane>/live --game <game dir> --port <port checkout> \\
        --game-mode 2048 --track-steps 6 --class-steps 2 --hud-labels-ref <png> --hud-timer-ref <png> \\
        --panel 3.5,1.8,4 --reflectivity 10,4,0.6,0.5 --spec 5,100 --blur 0.3 [--pre ReflectivityAmount=0]

`--reflectivity` is `Scale,Power,Min,Amount`, `--spec` is `SpecStrength,SpecPower`, all the circuit's authored values
(`docs/ghidra/functions/ps4-omega-eu/panel-displacement.md`, "Authored values"); the records are matched by value, so a wrong
value finds nothing and the run stops. A change names a material parameter (`ReflectivityAmount=0`) or a frame constant's lane
(`novaScaleBiasPower.0=0`; the game refills the frame constants every frame, so those do not hold). Every write is to this
run's own process, replaces only the value it expects to find, and is written back only where it still stands.

What does not work, kept out on purpose: writing the copies of the program's constant block (`__GLOBAL_CB__`, found by its
first eight floats). Those are per-frame staging copies in the guest and the port's own caches of them; a write changes no
frame, and a write-back into one that was reused since corrupts the heap. They are read only: one is dumped (`staged_cb`,
the live `novaScaleBiasPower`, light and `reflectiveBlurCoeffs` values) and counted by layout (`staged_layouts`). The scan
takes about two minutes (the guest's memory is a `memfd` mapping of several GiB). The same page's "Live" section has the results.
"""

import argparse
import importlib.util
import json
import os
import signal
import struct
import sys
import time
import zlib
from pathlib import Path

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("port_pair", HERE / "wipeout-pc-omega-pair.py")
pp = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pp)
spec2 = importlib.util.spec_from_file_location("engine_light_live", HERE / "wipeout-pc-engine-light-live.py")
el = importlib.util.module_from_spec(spec2)
spec2.loader.exec_module(el)

#: Frame constants of the lit program, with their float counts.
GLOBALS = {"novaScaleBiasPower": 3, "liveLighting0diffuse": 3, "liveLighting0specular": 3, "shadowLightSpecularDiffuse": 4,
           "liveLighting0direction": 3, "shadowLightDirection": 3, "reflectiveBlurCoeffs": 4, "fogColour": 4}
#: The lit program's `__GLOBAL_CB__` (`gcn_dump`'s `dwN`): the staged copy is found by its first eight floats and only read.
CB_FLOATS = 64
CB_HEAD = ["Depth", "SlopeWidth", "Aspect", "ReflectivityScale", "ReflectivityPower", "ReflectivityMin", "ReflectivityAmount",
           "SpecStrength"]
CB_BLURRINES = 52
#: Float index of each material parameter in the program's constant block.
CB_AT = {**{n: i for i, n in enumerate(CB_HEAD)}, "Blurrines": 52, "SpecPower": 53}


def name_hash(name):
    return (~zlib.crc32(name.encode())) & 0xFFFFFFFF


def floats(text):
    return [float(x) for x in text.split(",")]


def regions(pid, limit):
    out = []
    for line in open("/proc/%d/maps" % pid):
        parts = line.split()
        lo, hi = (int(x, 16) for x in parts[0].split("-"))
        name = parts[5] if len(parts) > 5 else ""
        # anonymous memory and the guest's memfd-backed memory, writable or not: a write through /proc/<pid>/mem must never
        # reach a file on disk
        guest = name.startswith("/memfd:")
        if "r" not in parts[1] or (name and name != "[heap]" and not guest) or (hi - lo > limit and not guest):
            continue
        out.append((lo, hi))
    return out


def scan(fd, pid, patterns, limit):
    """Every address of every pattern (a dict of name to bytes), one pass over the writable anonymous regions."""
    hits = {k: [] for k in patterns}
    longest = max(len(p) for p in patterns.values())
    said = time.time()
    for lo, hi in regions(pid, limit):
        pos = lo
        while pos < hi:
            if time.time() - said > 30:  # the watchdog reads silence as a stall
                said = time.time()
                print("[sheen] scanning at 0x%x" % pos, flush=True)
            n = min(64 << 20, hi - pos)
            try:
                blob = os.pread(fd, n + longest - 1, pos)
            except OSError:
                break
            for k, p in patterns.items():
                i = blob.find(p)
                while i != -1 and i < n:
                    hits[k].append(pos + i)
                    i = blob.find(p, i + 1)
            pos += n
    return hits


def f32(fd, addr):
    try:
        return struct.unpack("<f", os.pread(fd, 4, addr))[0]
    except (OSError, struct.error):
        return None


def change(text):
    """`Name=v` or `name.lane=v` to (name, lane, value)."""
    key, value = text.split("=")
    name, _, lane = key.partition(".")
    return name, int(lane or 0), float(value)


class Probe:
    """The material stores and frame constants of one process, found once."""

    def __init__(self, pid, want, limit):
        self.fd = os.open("/proc/%d/mem" % pid, os.O_RDWR)
        self.want = want
        patterns = {"cb": struct.pack("<8f", *(want[n] for n in CB_HEAD))}
        patterns.update({n: struct.pack("<I", name_hash(n)) for n in [*GLOBALS, *CB_AT]})
        t0 = time.time()
        hits = scan(self.fd, pid, patterns, limit)
        self.seconds = round(time.time() - t0, 1)
        # the model's own record of a material parameter: the hash, a count of 1, and at +0x10 a pointer to the value
        self.records = {n: [] for n in CB_AT}
        for n in CB_AT:
            for h in hits[n]:
                try:
                    count, ptr = struct.unpack("<I8xQ", os.pread(self.fd, 20, h + 4))
                    value = struct.unpack("<f", os.pread(self.fd, 4, ptr))[0]
                except (OSError, OverflowError, struct.error):
                    continue
                if count == 1 and abs(value - want[n]) < 1e-6:
                    self.records[n].append(ptr)
        # one material's values are neighbours (ten floats in a row), so a record of another material that happens to
        # author the same value is told apart by its distance from a `ReflectivityAmount` of this one
        anchors = self.records["ReflectivityAmount"]
        for n in CB_AT:
            self.records[n] = [p for p in self.records[n] if any(abs(p - a) <= 0x40 for a in anchors)]
        self.cbs = [a for a in hits["cb"] if abs((f32(self.fd, a + 4 * CB_BLURRINES) or 0) - want["Blurrines"]) < 1e-6]
        # which lit program the staged blocks belong to: `Blurrines, SpecPower` sit at dword 52 with a shadow map
        # (`0xe144`, `0x14284`: `shadowCoeffs` takes dwords 12 to 27) and at dword 36 without (`0x10244`, `0x164e4`)
        pair = struct.pack("<2f", want["Blurrines"], want["SpecPower"])
        self.layouts = {"dw%d" % d: sum(os.pread(self.fd, 8, a + 4 * d) == pair for a in hits["cb"]) for d in (36, 52)}
        self.cb = list(struct.unpack("<%df" % CB_FLOATS, os.pread(self.fd, 4 * CB_FLOATS, self.cbs[0]))) if self.cbs else None
        # a constant's table entry: the hash, then the value pointer at +0x10, which must be readable
        self.globals, self.global_entries = {}, {}
        for n, count in GLOBALS.items():
            for h in hits[n]:
                try:
                    ptr = struct.unpack("<Q", os.pread(self.fd, 8, h + 0x10))[0]
                    value = struct.unpack("<%df" % count, os.pread(self.fd, 4 * count, ptr))
                except (OSError, OverflowError, struct.error):
                    continue
                self.global_entries.setdefault(n, []).append((h, ptr, value))
            ptrs = [ptr for _, ptr, _ in self.global_entries.get(n, [])]
            if ptrs:
                self.globals[n] = max(set(ptrs), key=ptrs.count)

    def read(self, name):
        return list(struct.unpack("<%df" % GLOBALS[name], os.pread(self.fd, 4 * GLOBALS[name], self.globals[name])))

    def targets(self, name, lane):
        if name in GLOBALS:
            return [self.globals[name] + 4 * lane] if name in self.globals else []
        return sorted(set(self.records[name]))

    def apply(self, changes):
        """Writes each change where the value it replaces is still the expected one; returns what to write back."""
        keep = []
        for name, lane, value in changes:
            for a in self.targets(name, lane):
                old = os.pread(self.fd, 4, a)
                if name in self.want and abs(struct.unpack("<f", old)[0] - self.want[name]) > 1e-6:
                    continue  # a constant block the frame has moved on from
                new = struct.pack("<f", value)
                os.pwrite(self.fd, new, a)
                keep.append((a, old, new))
        return keep

    def undo(self, keep):
        """Writes back only where the written value still stands (a staged copy may have been reused since)."""
        for a, old, new in reversed(keep):
            if os.pread(self.fd, 4, a) == new:
                os.pwrite(self.fd, old, a)

    def note(self):
        return {"scan_seconds": self.seconds, "staged_cbs": len(self.cbs), "staged_layouts": self.layouts,
                "records": {n: [hex(a) for a in sorted(set(v))] for n, v in self.records.items()}, "staged_cb": self.cb,
                "globals": {n: {"at": hex(a), "value": self.read(n)} for n, a in self.globals.items()},
                "global_entries": {n: [(hex(h), hex(p), list(v)) for h, p, v in e][:8] for n, e in self.global_entries.items()}}


STATES = [
    ("s1-amount0", ["ReflectivityAmount=0"]),
    ("s2-flat", ["ReflectivityScale=0", "ReflectivityMin=1"]),
    ("s3-mirror", ["ReflectivityScale=0", "ReflectivityMin=1", "ReflectivityAmount=2", "Blurrines=0"]),
    ("s4-scale0", ["ReflectivityScale=0"]),
    ("s5-blur0", ["Blurrines=0"]),
    ("s6-spec0", ["SpecStrength=0"]),
    ("s7-shadowspec0", ["shadowLightSpecularDiffuse.1=0"]),
    ("s8-sun0", ["liveLighting0diffuse.0=0", "liveLighting0diffuse.1=0", "liveLighting0diffuse.2=0"]),
    ("s9-nova0", ["novaScaleBiasPower.0=0"]),
    ("s10-sheen-only", ["novaScaleBiasPower.0=0", "liveLighting0diffuse.0=0", "liveLighting0diffuse.1=0", "liveLighting0diffuse.2=0",
                        "SpecStrength=0"]),
    ("s11-base-only", ["ReflectivityAmount=0", "SpecStrength=0", "shadowLightSpecularDiffuse.1=0"]),
    ("s12-restored", []),
]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--game", required=True)
    ap.add_argument("--port", required=True)
    ap.add_argument("--track-steps", type=int, default=6)
    ap.add_argument("--game-mode", default="2048")
    ap.add_argument("--class-steps", type=int, default=0)
    ap.add_argument("--hud-labels-ref", type=Path)
    ap.add_argument("--hud-timer-ref", type=Path)
    ap.add_argument("--load-wait", type=float, default=12)
    ap.add_argument("--load-wait2", type=float, default=14)
    ap.add_argument("--panel", type=floats, required=True, help="Depth,SlopeWidth,Aspect")
    ap.add_argument("--reflectivity", type=floats, required=True, help="Scale,Power,Min,Amount")
    ap.add_argument("--spec", type=floats, required=True, help="SpecStrength,SpecPower")
    ap.add_argument("--blur", type=float, required=True, help="Blurrines")
    ap.add_argument("--pre", default="", help="comma list of changes applied before the race starts (the grid frames carry them)")
    ap.add_argument("--settle", type=float, default=14, help="seconds after the grid before the one-change frames")
    ap.add_argument("--region-limit", type=int, default=8 << 30)
    ap.add_argument("--n", type=int, default=3, help="frames per state")
    args = ap.parse_args()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    rig = pp.Rig(out / "port", args.game, args.port, "road-sheen")
    if pp.pad_seat.own_pads(rig.seat):
        sys.exit("seat %s already has a pad" % rig.seat)
    scale, power, rmin, amount = args.reflectivity
    want = {"Depth": args.panel[0], "SlopeWidth": args.panel[1], "Aspect": args.panel[2], "ReflectivityScale": scale,
            "ReflectivityPower": power, "ReflectivityMin": rmin, "ReflectivityAmount": amount, "SpecStrength": args.spec[0],
            "SpecPower": args.spec[1], "Blurrines": args.blur}
    note = {"want": want, "pre": args.pre, "frames": {}, "stuck": {}}
    probe, pre_keep = [], []
    pre = [change(c) for c in args.pre.split(",") if c]

    def save():
        (out / "note.json").write_text(json.dumps(note, indent=1))

    grid_frames = pp.grid_frames_2048

    def grid_after_probe(rig, args_, out_):
        """The race intro is looping: find the values, apply `--pre`, then start the race as the pair script does."""
        pid = el.game_pid(rig.pids["port"])
        if pid is None:
            sys.exit("STALLED: no descendant maps 0x800000000")
        p = Probe(pid, want, args.region_limit)
        probe.append(p)
        note["intro"] = p.note()
        print("[sheen] intro scan %.1f s: records %s, %d staged, globals %s" % (p.seconds, {n: len(set(v)) for n, v in p.records.items()}, len(p.cbs), sorted(p.globals)), flush=True)
        save()
        if not any(p.records.values()):
            sys.exit("STALLED: the material's values were not found")
        pre_keep.extend(p.apply(pre))
        note["pre_writes"] = len(pre_keep)
        return grid_frames(rig, args_, out_)

    pp.grid_frames_2048 = grid_after_probe
    # the watchdog stops this process with SIGTERM: leave through `finally`, so the port, its sway and the pad go too
    signal.signal(signal.SIGTERM, lambda *_: sys.exit("stopped by SIGTERM"))
    try:
        pick, seen = pp.capture_original(rig, args, out)
        t_grid = time.time()
        note["grid"] = {"pick": str(pick), **seen}
        p = probe[0]
        time.sleep(max(0, args.settle - (time.time() - t_grid)))
        note["settled_globals"] = {n: p.read(n) for n in p.globals}
        # back to the authored values, whatever `--pre` was
        for name, lane, value in pre:
            for a in p.targets(name, lane):
                if name in want and f32(p.fd, a) == value:
                    os.pwrite(p.fd, struct.pack("<f", want[name]), a)
        p.undo(pre_keep)
        time.sleep(0.5)
        note["frames"]["s0-baseline"] = el.grabs(rig, out, "s0-baseline", args.n)
        for tag, changes in STATES:
            keep = p.apply([change(c) for c in changes])
            time.sleep(0.5)
            # a write the game left alone; a constant it refills every frame reads back as the old value
            note["stuck"][tag] = "%d of %d" % (sum(os.pread(p.fd, 4, a) == new for a, _, new in keep), len(keep))
            note["frames"][tag] = el.grabs(rig, out, tag, args.n)
            p.undo(keep)
            save()
            print("[sheen] %s: writes stuck %s" % (tag, note["stuck"][tag]), flush=True)
    finally:
        save()
        rig.stop()
    print("[sheen] done: %s" % (out / "note.json"), flush=True)


if __name__ == "__main__":
    main()
