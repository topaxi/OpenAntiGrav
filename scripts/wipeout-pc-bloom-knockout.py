#!/usr/bin/env python3
"""Omega's bloom, knocked out live on the `wipeout_pc` native port (1.07 eboot): what the original's bloom adds to a grid frame.

Walks to a grid as `wipeout-pc-omega-pair.py` does, freezes the port (SIGSTOP), scans its memory for the circuit's own bloom
constants (taken from its `.EnvSettings`), then alternates conditions, each applied while frozen, run for a few frames (SIGCONT), frozen
again and grabbed (the grid camera drifts, so each condition sits between two base grabs):

    OAG_PAD_SEAT=N UV_OFFLINE=1 python3 scripts/emu-run.py -- python3 scripts/wipeout-pc-bloom-knockout.py \\
        --scratch <dir>/inst/a --game <the port's game directory> --port <a copy of the port checkout> \\
        --out <dir>/runs/<circuit>-a --track-steps N --env <circuit>.EnvSettings

The constants (`docs/ghidra/functions/ps4-omega-eu/tonemap.md`, "The bloom chain"), found by value:

- `blend`: `Bloom.Blend factors` as four floats; `f[0]` is the composite's `BloomScale`. The level array keeps `f[i]` at `+0` of a
  `0x108` stride, so a single `f[0]` followed by `f[1]` 0x108 later is a second kind of hit. Condition `nobloom` writes `f[0] = 0`
  in every hit.
- `bright`: `Threshold, Offset, Scale, Power` then `cBrightness` (a third three times) and `AlphaScale`, as eight floats. Condition
  `ehalf` writes the ToneMap object's `cL` (`+0x8c`) to 0.5, halving the exposure, to compare the output with what the curve and the encode
  predict. `nomask` writes `AlphaScale = 0` (the threshold term alone survives); condition `highthr` (not run by default: it wedged the GPU queue) writes `Threshold = 2` (`p = 0`, so
  only `AlphaScale * mask` survives).

A poke that leaves the frame byte-identical has hit a dead copy and is recorded as such. The pokes go to this run's own process
and are written back before the run ends.
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

STRIDE = 0x108
THIRD = struct.unpack("<f", bytes.fromhex("abaaaa3e"))[0]


def env_values(path):
    keys = {}
    for line in Path(path).read_text(errors="replace").splitlines():
        if line.startswith('"Bloom.'):
            name, _, value = line.partition("=")
            keys[name.strip('"')[len("Bloom."):]] = [float(x) for x in value.split()]
    return keys


def pack(*v):
    return struct.pack("<%df" % len(v), *v)


def read(fd, addr, n):
    try:
        return os.pread(fd, n, addr)
    except (OSError, OverflowError):
        return b""


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--scratch", required=True)
    ap.add_argument("--game", required=True)
    ap.add_argument("--port", required=True)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--env", type=Path, required=True)
    ap.add_argument("--track-steps", type=int, required=True)
    ap.add_argument("--game-mode", default="hd")
    ap.add_argument("--class-steps", type=int, default=0)
    ap.add_argument("--load-wait", type=float, default=12)
    ap.add_argument("--load-wait2", type=float, default=14)
    ap.add_argument("--region-limit", type=int, default=8 << 30)
    ap.add_argument("--rounds", type=int, default=6)
    ap.add_argument("--conditions", default="nobloom,nomask")
    ap.add_argument("--run", type=float, default=0.06, help="seconds the port runs after each poke")
    args = ap.parse_args()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    keys = env_values(args.env)
    thr, off, scale, power = (keys["Bright-pass " + k][0] for k in ("threshold", "offset", "scale", "power"))
    alpha = keys["Bright-pass alpha scale"][0]
    blend = keys["Blend factors"]
    note = {"args": {k: str(v) for k, v in vars(args).items()}, "env": keys}
    rig = pp.Rig(args.scratch, args.game, args.port, "bloom-knockout")
    if pp.pad_seat.own_pads(rig.seat):
        sys.exit("seat %s already has a pad" % rig.seat)
    stopped, fd, originals = [], None, {}
    signal.signal(signal.SIGTERM, lambda *_: sys.exit("stopped by SIGTERM"))

    def save():
        (out / "note.json").write_text(json.dumps(note, indent=1))

    def poke(addr, data):
        try:
            os.pwrite(fd, data, addr)
        except OSError:
            note.setdefault("poke_failed", []).append(hex(addr))

    try:
        pick, seen = pp.capture_original(rig, args, out)
        note["grid"] = {"pick": str(pick), **seen}
        pid = el.game_pid(rig.pids["port"])
        if pid is None:
            sys.exit("STALLED: no descendant maps 0x800000000")
        fd = os.open("/proc/%d/mem" % pid, os.O_RDWR)
        os.kill(pid, signal.SIGSTOP)
        stopped.append(pid)
        rig.grab(out / "frozen.png")
        t0 = time.time()
        full = pack(thr, off, scale, power, THIRD, THIRD, THIRD, alpha)
        pats = {"blend": pack(*blend), "bright": pack(thr, off, scale, power), "full": full, "f0": pack(blend[0])}
        hits = sheen.scan(fd, pid, pats, args.region_limit)
        note["scan_seconds"] = round(time.time() - t0, 1)
        # the guest's own address range only: a hit in the host's heap or a host-side mirror of the guest's memory is a copy the
        # video core owns, and writing it wedged the GPU queue twice ("Wrong PM4 type 1")
        guest = lambda hs: [h for h in hs if 0x800000000 <= h < 0x4000000000]
        hits = {k: guest(v) for k, v in hits.items()}
        level_hits = [h for h in hits["f0"]
                      if all(read(fd, h + i * STRIDE, 4) == pack(blend[i]) for i in range(1, 4))]
        targets = {
            "nobloom": [(h, pack(0.0)) for h in hits["blend"] + level_hits],
            "nomask": [(h + 28, pack(0.0)) for h in hits["full"]],
            "highthr": [(h, pack(2.0)) for h in hits["bright"]],
        }
        # the exposure colour scale `cL` (`+0x8c` of the ToneMap object whose pointer sits at guest 0x021dbdb8, 1.0 on every circuit read):
        # `E = clamp((a + b LAvg) / LAvg, min, max) * cL`, so 0.5 halves `E` and the frame follows the curve and the encode
        obj = struct.unpack("<Q", read(fd, pp.IMAGE_BASE + 0x021DBDB8 - 0x01000000, 8))[0]
        cl_at = obj + 0x8C
        note["tonemap_object"] = {"ptr": hex(obj), "cL": struct.unpack("<f", read(fd, cl_at, 4))[0] if read(fd, cl_at, 4) else None}
        targets["ehalf"] = [(cl_at, pack(0.5))] if read(fd, cl_at, 4) == pack(1.0) else []
        originals = {n: [(a, read(fd, a, 4)) for a, _ in t] for n, t in targets.items()}
        note["hits"] = {"blend": [hex(h) for h in hits["blend"]], "level_array": [hex(h) for h in level_hits],
                        "bright": [hex(h) for h in hits["bright"]], "full": [hex(h) for h in hits["full"]]}
        save()
        print("[bloom] hits: blend %d level %d bright %d full %d" % (len(hits["blend"]), len(level_hits), len(hits["bright"]),
                                                                    len(hits["full"])), flush=True)

        def run_frozen(label, name):
            """Apply `name`'s pokes (none for the base), run, freeze, read the pokes back, grab."""
            for a, d in (targets[name] if name else []):
                poke(a, d)
            os.kill(pid, signal.SIGCONT)
            time.sleep(args.run)
            os.kill(pid, signal.SIGSTOP)
            held = sum(read(fd, a, 4) == d for a, d in (targets[name] if name else []))
            path = out / ("%s.png" % label)
            rig.grab(path)
            if name:
                for a, d in originals[name]:
                    poke(a, d)
            row = {"label": label, "condition": name, "pokes": len(targets[name]) if name else 0, "held_after_run": held,
                   "frame": str(path)}
            note.setdefault("rows", []).append(row)
            save()
            return row

        try:
            for r in range(args.rounds):
                run_frozen("base-%d" % r, None)
                for name in args.conditions.split(","):
                    run_frozen("%s-%d" % (name, r), name)
                    run_frozen("base-%d-after-%s" % (r, name), None)
        except ProcessLookupError:
            # the port ends on its own after a few dozen stop/continue cycles ("Wrong PM4 type 1" in its GPU log); what was
            # grabbed up to there stands, and the last row may be a frame of a dead process
            note["ended_early_after"] = len(note.get("rows", []))
        print("[bloom] done %d grabs%s" % (len(note["rows"]), " (port ended early)" if "ended_early_after" in note else ""), flush=True)
    finally:
        if fd is not None:
            for items in originals.values():
                for a, d in items:
                    try:
                        os.pwrite(fd, d, a)
                    except OSError:
                        pass
        for pid in stopped:
            try:
                os.kill(pid, signal.SIGCONT)
            except ProcessLookupError:
                pass
        save()
        rig.stop()


if __name__ == "__main__":
    main()
