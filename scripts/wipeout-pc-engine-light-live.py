#!/usr/bin/env python3
"""Live check of Omega's engine light on the `wipeout_pc` native port (1.07 eboot): walk to a grid, read the vertex-light
table, then grab frames with the engine light as shipped, suppressed (the producer returns at once) and boosted, or
(`--calibrate`) with every craft's range forced to `Radius + w`.

    uv run --with evdev python3 scripts/emu-run.py --silence 200 --total 560 -- \
        python3 scripts/wipeout-pc-engine-light-live.py --out data/scratch/<lane>/live --game <game dir> --port <port checkout> \
        [--track-steps 8] [--game-mode hd|2048] [--series 60] [--calibrate=-1,-0.5,0,0.5,1]

The walk and the rig are `scripts/wipeout-pc-omega-pair.py`'s (its own headless sway, `OAG_PAD_SEAT`, muted). Memory goes
through `/proc/<pid>/mem`, which Yama allows because this process is the port's ancestor. Addresses are the 1.07 Ghidra
program's; runtime = `0x800000000 + address - 0x01000000`. Every write is to this run's own process and is undone or dies
with it. `docs/ghidra/functions/ps4-omega-eu/engine-light.md`, "Live check", has the results.
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

BASE = 0x800000000
GHIDRA_BASE = 0x01000000
TABLE = 0x02047E60
COUNT = 0x020482E0
PRODUCER = 0x0173E350
PROLOGUE = bytes.fromhex("554889e5415741564154534883ec204989fc498bbc241801")
CONSTS = 0x01863ABC  # 12 floats


def rt(addr):
    return BASE + addr - GHIDRA_BASE


def descendants(pid):
    out, todo = [], [pid]
    while todo:
        p = todo.pop()
        for task in Path("/proc/%d/task" % p).glob("*/children"):
            try:
                kids = [int(k) for k in task.read_text().split()]
            except OSError:
                continue
            out += kids
            todo += kids
    return out


def game_pid(root):
    for p in descendants(root):
        try:
            if any(line.startswith("800000000-") for line in open("/proc/%d/maps" % p)):
                return p
        except OSError:
            pass
    return None


class Mem:
    def __init__(self, pid):
        self.fd = os.open("/proc/%d/mem" % pid, os.O_RDWR)

    def read(self, addr, n):
        return os.pread(self.fd, n, rt(addr))

    def write(self, addr, data):
        return os.pwrite(self.fd, data, rt(addr))


def records(blob, count):
    out = []
    for i in range(min(count, 32)):
        px, py, pz, inv, r, g, b, rng, owner = struct.unpack_from("<8fi", blob, i * 0x24)
        out.append({"i": i, "pos": [px, py, pz], "inv_range": inv, "colour": [r, g, b], "range": rng, "owner": owner})
    return out


def sample(mem, n=600, gap=0.002):
    """Many reads of the table; the frame clears and refills it, so keep each read and report the fullest."""
    seen = []
    for _ in range(n):
        blob = mem.read(TABLE, 0x484)
        count = struct.unpack_from("<i", blob, 0x480)[0]
        seen.append((count, blob))
        time.sleep(gap)
    return seen


def summarise(seen):
    top = max(c for c, _ in seen)
    full = [records(b, c) for c, b in seen if c == top]
    hist = {}
    for c, _ in seen:
        hist[c] = hist.get(c, 0) + 1
    return {"count_hist": hist, "top": top, "fullest": full[:40]}


def grabs(rig, out, tag, n):
    paths = []
    for k in range(n):
        paths.append(str(rig.grab(out / ("%s-%02d.png" % (tag, k)))))
        time.sleep(0.15)
    return paths


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--game", required=True, help="the game folder (`wipeout-pc-omega-pair.py prepare-game`)")
    ap.add_argument("--port", required=True, help="the port's checkout (built: out/gow3-probe)")
    ap.add_argument("--track-steps", type=int, default=8)
    ap.add_argument("--game-mode", default="hd")
    ap.add_argument("--class-steps", type=int, default=0)
    ap.add_argument("--hud-labels-ref", type=Path)
    ap.add_argument("--hud-timer-ref", type=Path)
    ap.add_argument("--load-wait", type=float, default=12)
    ap.add_argument("--load-wait2", type=float, default=14)
    ap.add_argument("--settle", type=float, default=12, help="seconds after the grid before the A/B/C frames (opponents gone)")
    ap.add_argument("--calibrate", default="", help="comma list of forced w (range = Radius + w): grab frames at each and stop")
    ap.add_argument("--series", type=int, default=6)
    ap.add_argument("--boost", type=float, default=50.0)
    args = ap.parse_args()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    rig = pp.Rig(out / "port", args.game, args.port, "engine-light")
    if pp.pad_seat.own_pads(rig.seat):
        sys.exit("seat %s already has a pad" % rig.seat)
    note = {}
    try:
        pick, seen = pp.capture_original(rig, args, out)
        t_grid = time.time()
        note["grid"] = {"pick": str(pick), **seen}
        pid = game_pid(rig.pids["port"])
        if pid is None:
            sys.exit("STALLED: no descendant maps 0x800000000")
        mem = Mem(pid)
        head = mem.read(PRODUCER, len(PROLOGUE))
        note["prologue_ok"] = head == PROLOGUE
        note["prologue"] = head.hex()
        consts = mem.read(CONSTS, 48)
        note["consts"] = list(struct.unpack("<12f", consts))
        print("[live] pid %d prologue_ok %s consts %s" % (pid, note["prologue_ok"], note["consts"]), flush=True)
        if not note["prologue_ok"]:
            sys.exit("STALLED: the producer's bytes are not at the mapped address")
        note["table_countdown"] = summarise(sample(mem))
        print("[live] countdown table top %d" % note["table_countdown"]["top"], flush=True)
        time.sleep(max(0, args.settle - (time.time() - t_grid)))
        note["table_rest"] = summarise(sample(mem))
        if args.calibrate:
            # range = Radius + w with w forced: the flare update's `w = r * scale + [0x01863a20]` loses its product
            # (VMULSS xmm1, xmm3, xmm1 at 0x017384cf becomes VXORPS xmm1, xmm1, xmm1) and the addend is written.
            MUL, ADDEND = 0x017384CF, 0x01863A20
            assert mem.read(MUL, 4) == bytes.fromhex("c5e259c9") and mem.read(ADDEND, 4) == struct.pack("<f", -1.0)
            mem.write(MUL, bytes.fromhex("c5f057c9"))
            cal = []
            for w in [float(x) for x in args.calibrate.split(",")]:
                mem.write(ADDEND, struct.pack("<f", w))
                time.sleep(0.4)
                tab = summarise(sample(mem, 200))
                frames = grabs(rig, out, "cal-w%+.2f" % w, 4)
                cal.append({"w": w, "ranges": [round(r["range"], 4) for r in (tab["fullest"][0] if tab["fullest"] else [])], "frames": frames})
                print("[live] w %+.2f ranges %s" % (w, cal[-1]["ranges"]), flush=True)
            note["calibration"] = cal
            mem.write(ADDEND, struct.pack("<f", -1.0))
            mem.write(MUL, bytes.fromhex("c5e259c9"))
            return
        # how often the player's record (the last) changes: timestamped full reads at about 1 ms
        seq, t0 = [], time.time()
        while time.time() - t0 < 2.0:
            blob = mem.read(TABLE, 0x484)
            if struct.unpack_from("<i", blob, 0x480)[0] == note["table_rest"]["top"]:
                seq.append((round(time.time() - t0, 4), records(blob, 32)[note["table_rest"]["top"] - 1]["range"]))
        note["player_range_seq"] = seq
        note["frames_a"] = grabs(rig, out, "a-shipped", args.series)
        mem.write(PRODUCER, b"\xc3")
        time.sleep(0.5)
        note["table_suppressed"] = summarise(sample(mem, 300))
        note["frames_b"] = grabs(rig, out, "b-suppressed", args.series // 2)
        mem.write(PRODUCER, PROLOGUE[:1])
        boosted = list(note["consts"])
        for k in (0, 1, 2, 5, 6, 7, 8):  # the three bases' channels: 3.0 0.5 0.1, 4 40 10, 1.5
            boosted[k] *= args.boost
        mem.write(CONSTS, struct.pack("<12f", *boosted))
        time.sleep(0.5)
        note["table_boosted"] = summarise(sample(mem, 300))
        note["frames_c"] = grabs(rig, out, "c-boosted", 6)
        mem.write(CONSTS, consts)
        time.sleep(0.5)
        note["frames_d"] = grabs(rig, out, "d-restored", 2)
    finally:
        (out / "note.json").write_text(json.dumps(note, indent=1))
        rig.stop()
    print("[live] done: %s" % (out / "note.json"), flush=True)


if __name__ == "__main__":
    main()
