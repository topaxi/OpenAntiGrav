#!/usr/bin/env python3
"""Read the original's tone map luma ring on a native-port (wipeout_pc) Omega grid.

    OAG_PAD_SEAT=N UV_OFFLINE=1 python3 scripts/emu-run.py -- python3 scripts/wipeout-pc-tonemap-ring.py \
        --scratch data/scratch/<lane>/inst/a --game <the port's patched game directory> --port <the port's checkout> \
        --out data/scratch/<lane>/runs/<circuit>-a --track-steps N

`--track-steps` is the Right presses on Track Select from Vineta K (Moa Therma 2, Sebenco Climb 5, Modesto Heights 8, The Amphiseum 9;
`wipeout-pc-omega-pair.py`, "Track Select order"). One boot walks MAIN MENU > Race Box > Track Select > Ship Select > loading > START RACE
exactly as `wipeout-pc-omega-pair.py pair` does (its `Rig`, its HUD detector), reads `/proc/<gow3-probe pid>/mem` in a loop from the main
menu on (a row to `samples.jsonl` whenever the tone map object or its ring changes), `SIGSTOP`s the port right after the grab of the first
HUD frame plus 2, and dumps the object (`frozen.json`) and every heap pointer in it (`frozen-pointers.json`). Reads only; ours is not
rendered and no pair manifest is written. The state is proved by the object itself: `+0x98/+0x9c` (exposure min/max), `+0x80` (lum a) and
`+0x88` times 60 (response) equal the circuit's own `.EnvSettings`. Layout and results: `docs/formats/omega-status.md`, "HD-mode frame
brightness: the original's luma ring, read".

The object pointer is the qword at guest `0x021dbdb8` (the 1.07 patch eboot; `docs/ghidra/functions/ps4-omega-eu/patch-v107-addresses.md`),
whose value changes from boot to boot. The ring is `float[4]` at the pointer held at object `+0x1d8` (begin, end, capacity), one slot
written per frame, `+0xc2` the index written last.
"""
import argparse
import importlib.util
import json
import os
import signal
import struct
import subprocess
import sys
import threading
import time
from pathlib import Path

WT = Path(__file__).resolve().parent
sys.path.insert(0, str(WT))
spec = importlib.util.spec_from_file_location("pair", WT / "wipeout-pc-omega-pair.py")
pair = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pair)
sp = pair.sp

POINTER_AT = pair.IMAGE_BASE + (0x021DBDB8 - 0x01000000)
OBJ_SIZE = 0x210


def read(pid, addr, n):
    try:
        with open("/proc/%d/mem" % pid, "rb", buffering=0) as m:
            m.seek(addr)
            return m.read(n)
    except (OSError, ValueError, OverflowError):
        return b""


def heap_ptr(v):
    return 0x100000000 <= v < 0x4000000000


def snapshot(pid):
    """(object bytes, {offset: bytes behind pointer}) or None."""
    raw = read(pid, POINTER_AT, 8)
    if len(raw) < 8:
        return None
    obj_addr = struct.unpack("<Q", raw)[0]
    if not heap_ptr(obj_addr):
        return {"obj_addr": obj_addr, "obj": b"", "ring": b""}
    obj = read(pid, obj_addr, OBJ_SIZE)
    ring = b""
    if len(obj) == OBJ_SIZE:
        begin, end = struct.unpack_from("<QQ", obj, 0x1D8)
        if heap_ptr(begin) and begin <= end <= begin + 0x1000:
            ring = read(pid, begin, end - begin)
    return {"obj_addr": obj_addr, "obj": obj, "ring": ring}


class Sampler(threading.Thread):
    def __init__(self, pid, path):
        super().__init__(daemon=True)
        self.pid, self.path, self.stop_flag = pid, path, False
        self.last, self.n = None, 0

    def run(self):
        with open(self.path, "w") as f:
            while not self.stop_flag:
                s = snapshot(self.pid)
                if s is not None:
                    key = (s["obj_addr"], s["obj"], s["ring"])
                    if key != self.last:
                        self.last = key
                        self.n += 1
                        f.write(json.dumps({"t": time.time(), "obj_addr": hex(s["obj_addr"]), "obj": s["obj"].hex(), "ring": s["ring"].hex()}) + "\n")
                        f.flush()
                time.sleep(0.001)


def walk_and_sample(rig, args, out):
    rig.write_ini()
    rig.start_sway()
    rig.start_port()
    rig.start_pad()
    pair.wait_main_menu(rig)
    pid = rig.probe_pid()
    smp = Sampler(pid, out / "samples.jsonl")
    smp.start()
    stamps = {}
    rig.press("right")
    rig.press("cross", 3)
    rig.press("cross", 3)  # Race Box -> Track Select
    for _ in range(args.track_steps):
        rig.press("right", 1.5)
    time.sleep(1.5)
    rig.grab(out / "track-select.png")
    pair.name_crop(out / "track-select.png", out / "circuit-name.png")
    rig.press("cross", 3)
    rig.press("down")
    rig.press("down")
    time.sleep(1)
    rig.grab(out / "ship-select.png")
    rig.press("cross", 3)
    time.sleep(12)
    rig.grab(out / "loading.png")
    time.sleep(14)
    rig.press("cross", 0.2)
    stamps["start_tap"] = time.time()
    frames, hud_at, ring0 = [], None, None
    t0 = time.time()
    pick_i = None
    while time.time() - t0 < 25:
        ta = time.time()
        path = rig.grab(out / ("frame%02d.png" % len(frames)))
        tb = time.time()
        frames.append((path, ta, tb))
        ring = sp.region_mean(path, sp.HUD_REGION, "(mean.r+mean.g)/2-mean.b")
        ring0 = ring if ring0 is None else ring0
        if hud_at is None and (ring > 0.03 or ring - ring0 > pair.HUD_RISE):
            hud_at = len(frames) - 1
        if hud_at is not None and len(frames) - 1 == hud_at + sp.GRAB_AFTER_HUD:
            pick_i = len(frames) - 1
            os.kill(pid, signal.SIGSTOP)  # freeze right after the picked frame was grabbed
            stamps["frozen"] = time.time()
            break
        time.sleep(0.3)
    if pick_i is None:
        smp.stop_flag = True
        sys.exit("STALLED at grid: no HUD in %d frames" % len(frames))
    time.sleep(0.2)
    smp.stop_flag = True
    smp.join(2)
    frozen = snapshot(pid)
    (out / "frozen.json").write_text(json.dumps({"obj_addr": hex(frozen["obj_addr"]), "obj": frozen["obj"].hex(), "ring": frozen["ring"].hex()}))
    # the whole 4 KiB around the pointers named in the object, for the layout notes
    obj = frozen["obj"]
    ptrs = {}
    if len(obj) == OBJ_SIZE:
        for off in range(0, OBJ_SIZE - 7, 8):
            v = struct.unpack_from("<Q", obj, off)[0]
            if heap_ptr(v):
                ptrs[hex(off)] = {"ptr": hex(v), "bytes": read(pid, v, 128).hex()}
    (out / "frozen-pointers.json").write_text(json.dumps(ptrs, indent=1))
    try:
        rig.grab(out / "frozen-again.png")
    finally:
        os.kill(pid, signal.SIGCONT)
    red = sp.board_red(frames[pick_i][0])
    base = sp.board_red(frames[hud_at][0])
    rec = {"probe_pid": pid, "frames": len(frames), "hud_first_frame": hud_at, "picked_frame": pick_i,
           "pick_grab_window": [frames[pick_i][1], frames[pick_i][2]], "frame_times": [[f[1], f[2]] for f in frames],
           "stamps": stamps, "board_red": [round(red, 3), round(base, 3)], "sampler_changes": smp.n}
    (out / "capture.json").write_text(json.dumps(rec, indent=1))
    return rec


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--scratch", required=True)
    ap.add_argument("--game", required=True)
    ap.add_argument("--port", required=True)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--track-steps", type=int, required=True)
    a = ap.parse_args()
    pair.pad_seat.require_free(pair.pad_seat.seat())
    out = a.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    rig = pair.Rig(a.scratch, a.game, a.port, Path(a.scratch).resolve().parent.name)
    try:
        rec = walk_and_sample(rig, a, out)
        print(json.dumps({k: v for k, v in rec.items() if k not in ("frame_times",)}))
        frozen = json.loads((out / "frozen.json").read_text())
        obj, ring = bytes.fromhex(frozen["obj"]), bytes.fromhex(frozen["ring"])
        if len(obj) == OBJ_SIZE and len(ring) == 16:
            slots = struct.unpack("<4f", ring)
            lavg = sum(slots) / 4
            a_, b_, resp, c_l, e_min, e_max, s_a, s_b = (struct.unpack_from("<f", obj, o)[0] for o in (0x80, 0x84, 0x88, 0x8C, 0x98, 0x9C, 0xA8, 0xAC))
            e = min(max((a_ + b_ * lavg) / max(lavg, 1e-4), e_min), e_max) * c_l
            print(json.dumps({"ring": [round(s, 5) for s in slots], "ring_mean": round(lavg, 5), "E": round(e, 4), "t1": round(max(s_a + s_b * lavg, 0.01), 4),
                              "exposure_min_max": [round(e_min, 4), round(e_max, 4)], "lum_a": round(a_, 4), "response": round(resp * 60, 4)}))
    finally:
        rig.stop()


if __name__ == "__main__":
    main()
