#!/usr/bin/env python3
"""Read the light inputs bound to Omega's crowd and prop draws on the `wipeout_pc` native port (1.07 eboot).

Walks to a grid as `wipeout-pc-omega-pair.py` does, freezes the game (SIGSTOP) a few frames after the HUD shows and reads
its guest memory through `/proc/<pid>/mem` without writing anything:

    OAG_PAD_SEAT=N uv run --with evdev python3 scripts/emu-run.py --bg --status F -- \\
        python3 scripts/wipeout-pc-crowd-light-live.py --out data/scratch/<lane>/runs/sol-a --game <game dir> --port <port checkout> \\
        --game-mode 2048 --track-steps 6 --class-steps 2 --hud-labels-ref <png> --hud-timer-ref <png>

1. The frame constants the programs read: the constant name table's entry is `~crc32(name)` with the value pointer at `+0x10`
   (`wipeout-pc-road-sheen-live.py`). Read for `constantAmbientColour`, `liveLighting0diffuse`, `liveLighting0direction`,
   `liveLighting0specular`, `fogColour`, `fadeResCoeffs`.
2. The staged copies of a pixel constant block: the plain crowd programs `0x41a4` / `0x3c04` take `dw0 constantAmbientColour`,
   `dw4 liveLighting0diffuse`, `dw8 liveLighting0direction`, `dw12 fogColour`, `dw16 fadeResCoeffs` (80 bytes); the `_shadow`
   programs put `Shadowed` and `Ambient` (4 dwords) in front. Every place that holds the live `fogColour`'s xyz is dumped with
   the 64 bytes before it and the 48 after it, with the kind of mapping it sits in, so a per-draw value shows as a block
   whose lighting floats differ from the frame constants.
3. The material records `Shadowed` and `Ambient` (the model's own parameter records: hash, count 1, value pointer at `+0x10`).

`docs/formats/omega-status.md`, "Crowd and scene light inputs, read live", has the results. Reads only.
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
sheen_spec = importlib.util.spec_from_file_location("sheen", HERE / "wipeout-pc-road-sheen-live.py")
sheen = importlib.util.module_from_spec(sheen_spec)
sheen_spec.loader.exec_module(sheen)
pp, el = sheen.pp, sheen.el

#: Frame constants, with the floats read from each table entry's value pointer.
FRAME = {"constantAmbientColour": 4, "liveLighting0diffuse": 4, "liveLighting0direction": 4, "liveLighting0specular": 4,
         "fogColour": 4, "fadeResCoeffs": 4, "shadowLightDirection": 4}
RECORDS = {"Shadowed": 1, "Ambient": 3}
#: Bytes dumped around a `fogColour` copy. The `_shadow` crowd programs' 96-byte block (`gcn_dump`, `Crowd_Sprite_Diffuse_Normal_Alpha_Shadow`
#: `0x21e4`) is `Shadowed` (dw0), `Ambient` (dw1-3), then the plain 80-byte block: `constantAmbientColour` (dw4), `liveLighting0diffuse` (dw8),
#: `liveLighting0direction` (dw12), `fogColour` (dw16), `fadeResCoeffs` (dw20), so `fogColour` sits 64 bytes in.
BEFORE, AFTER = 64, 32


def kind(pid, addr, maps):
    for lo, hi, name in maps:
        if lo <= addr < hi:
            return name or "anon"
    return "?"


def read_maps(pid):
    out = []
    for line in open("/proc/%d/maps" % pid):
        parts = line.split()
        lo, hi = (int(x, 16) for x in parts[0].split("-"))
        out.append((lo, hi, parts[5] if len(parts) > 5 else ""))
    return out


def read(fd, addr, n):
    try:
        return os.pread(fd, n, addr)
    except (OSError, OverflowError):
        return b""


def frame_constants(fd, hits):
    """Per name: every table entry whose value pointer reads, and the pointer most entries share."""
    out = {}
    for name, count in FRAME.items():
        entries = []
        for h in hits[name]:
            raw = read(fd, h + 0x10, 8)
            if len(raw) < 8:
                continue
            ptr = struct.unpack("<Q", raw)[0]
            data = read(fd, ptr, 4 * count)
            if len(data) == 4 * count:
                entries.append((h, ptr, struct.unpack("<%df" % count, data)))
        # a hit of the hash that is not a table entry reads a small integer or a float where the pointer would be
        ptrs = [p for _, p, _ in entries if p >= 0x800000000]
        best = max(set(ptrs), key=ptrs.count) if ptrs else None
        out[name] = {"entries": [(hex(h), hex(p), list(v)) for h, p, v in entries[:12]], "n_entries": len(entries),
                     "ptr": hex(best) if best else None,
                     "value": list(next(v for _, p, v in entries if p == best)) if best else None}
    return out


def records(fd, hits):
    out = {}
    for name, count in RECORDS.items():
        seen = {}
        for h in hits[name]:
            raw = read(fd, h + 4, 20)
            if len(raw) < 20:
                continue
            c, ptr = struct.unpack("<I8xQ", raw)
            data = read(fd, ptr, 4 * count)
            if c == 1 and len(data) == 4 * count:
                v = struct.unpack("<%df" % count, data)
                if all(abs(x) < 1e6 for x in v):
                    seen.setdefault(tuple(round(x, 6) for x in v), []).append(hex(ptr))
        out[name] = [{"value": list(k), "n": len(v), "ptrs": v[:4]} for k, v in sorted(seen.items(), key=lambda kv: -len(kv[1]))]
    return out


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
    ap.add_argument("--region-limit", type=int, default=8 << 30)
    ap.add_argument("--lighting-key", type=lambda t: [float(x) for x in t.split(",")],
                    help="Sky height offset,Sky scale,Sky brightness,Ship diffuse brightness of the circuit's EnvSettings: dumps the "
                         "floats around every copy of that run (the lighting object, hull-tone.md)")
    args = ap.parse_args()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    rig = pp.Rig(out / "port", args.game, args.port, "crowd-light")
    if pp.pad_seat.own_pads(rig.seat):
        sys.exit("seat %s already has a pad" % rig.seat)
    note = {"args": {k: str(v) for k, v in vars(args).items()}}
    stopped = []
    signal.signal(signal.SIGTERM, lambda *_: sys.exit("stopped by SIGTERM"))

    def save():
        (out / "note.json").write_text(json.dumps(note, indent=1))

    try:
        pick, seen = pp.capture_original(rig, args, out)
        note["grid"] = {"pick": str(pick), **seen}
        pid = el.game_pid(rig.pids["port"])
        if pid is None:
            sys.exit("STALLED: no descendant maps 0x800000000")
        os.kill(pid, signal.SIGSTOP)
        stopped.append(pid)
        note["frozen_at"] = time.time()
        rig.grab(out / "frozen.png")
        fd = os.open("/proc/%d/mem" % pid, os.O_RDONLY)
        maps = read_maps(pid)
        t0 = time.time()
        pats = {n: struct.pack("<I", sheen.name_hash(n)) for n in [*FRAME, *RECORDS]}
        hits = sheen.scan(fd, pid, pats, args.region_limit)
        note["pass1_seconds"] = round(time.time() - t0, 1)
        note["hash_hits"] = {n: len(v) for n, v in hits.items()}
        note["frame_constants"] = frame_constants(fd, hits)
        note["records"] = records(fd, hits)
        save()
        print("[crowd] pass 1 %.0f s: %s" % (note["pass1_seconds"], note["hash_hits"]), flush=True)
        fog = note["frame_constants"]["fogColour"]["value"]
        if not fog:
            sys.exit("STALLED: no fogColour table entry resolved")
        t1 = time.time()
        key = struct.pack("<3f", *fog[:3])
        found = sheen.scan(fd, pid, {"fog": key}, args.region_limit)["fog"]
        note["pass2_seconds"] = round(time.time() - t1, 1)
        note["fog_copies"] = len(found)
        blocks = []
        for a in found:
            raw = read(fd, a - BEFORE, BEFORE + 16 + AFTER)
            if len(raw) == BEFORE + 16 + AFTER:
                blocks.append({"at": hex(a - BEFORE), "kind": kind(pid, a, maps), "hex": raw.hex()})
        (out / "blocks.json").write_text(json.dumps(blocks))
        print("[crowd] pass 2 %.0f s: %d fog copies, %d dumped" % (note["pass2_seconds"], len(found), len(blocks)), flush=True)
        if args.lighting_key:
            key = struct.pack("<4f", *args.lighting_key)
            lit = sheen.scan(fd, pid, {"key": key}, args.region_limit)["key"]
            note["lighting_objects"] = [{"at": hex(a), "kind": kind(pid, a, maps),
                                         "floats": list(struct.unpack("<48f", read(fd, a - 0x20, 192)))} for a in lit[:40]]
            print("[crowd] lighting key: %d copies" % len(lit), flush=True)
        save()
    finally:
        for pid in stopped:
            try:
                os.kill(pid, signal.SIGCONT)
            except ProcessLookupError:
                pass
        save()
        rig.stop()
    print("[crowd] done: %s" % (out / "note.json"), flush=True)


if __name__ == "__main__":
    main()
