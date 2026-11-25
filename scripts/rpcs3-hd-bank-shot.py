#!/usr/bin/env python3
"""Pose HD's player craft, give it a weapon, fire it and read its projectiles every game frame.

Attached to a `serve` already in a race with weapons On (`OAG_WEAPONS_ON=1 scripts/emu-hd-race-walk.sh`,
or the same walk with the GDB stub kept). Every frame is stepped off the lap clock with the target paused
between frames (the missile film's pattern); the craft is re-placed on the pose every frame until `--hold`
frames after the press. Writes `<out>/frames.jsonl`: per frame the clock and each live projectile's raw
bytes.

    OAG_RPCS3_ATTACH=1 uv run --with evdev python3 scripts/rpcs3-hd-bank-shot.py --out D \
        --pos=-173.3360,22.4464,-321.5013 --forward=0.91059,0.05145,0.41009 \
        --up=0.24577,0.73033,-0.63736 --frames 170 --press-at 5 --hold 34          # Plasma (state 7)
    ... --objects --vtable 0x863e38 --state 0 ...                                  # the Rockets
    ... --objects --vtable 0x863d18 --state 1 ...                                  # the Missile

Layouts and the Moa Therma run: docs/ghidra/functions/ps3-hdfury-eu/plasma.md, "On Moa Therma's bank".
"""
import argparse
import importlib.util
import json
import struct
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parent
sys.path.insert(0, str(REPO))


def load(name, file):
    spec = importlib.util.spec_from_file_location(name, REPO / file)
    mod = importlib.util.module_from_spec(spec)
    sys.modules[name] = mod
    spec.loader.exec_module(mod)
    return mod


poll = load("rpcs3_mem_poll", "rpcs3-mem-poll.py")
drive = poll.drive
place = poll.place
import rpcs3_hd_field  # noqa: E402
from rpcs3_debugger import Debugger  # noqa: E402

LAP_CLOCK = 0x7014
SLOT_OFFSET = 0x5EDC


def floats(s):
    return tuple(float(v) for v in s.split(","))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--pos", required=True)
    ap.add_argument("--forward", required=True)
    ap.add_argument("--up", required=True)
    ap.add_argument("--speed", type=float, default=140.0)
    ap.add_argument("--state", type=int, default=7, help="pickup state: 7 Plasma, 0 Rocket, 1 Missile")
    ap.add_argument("--vtable", type=lambda s: int(s, 0), default=0x00864698, help="the projectile manager's vtable")
    ap.add_argument("--pool", type=lambda s: int(s, 0), default=0x84, help="manager offset of the 16 slot pointers")
    ap.add_argument("--objects", action="store_true", help="--vtable names the projectile objects themselves; log each one whose bytes changed")
    ap.add_argument("--span", type=lambda s: int(s, 0), default=0x200)
    ap.add_argument("--press-at", type=int, default=10, help="frame the fire button goes down")
    ap.add_argument("--hold", type=int, default=62, help="frames after the press the craft stays on the pose")
    ap.add_argument("--frames", type=int, default=260)
    ap.add_argument("--button", default="triangle")
    ap.add_argument("--step-timeout", type=float, default=8.0)
    args = ap.parse_args()
    out = args.out
    out.mkdir(parents=True, exist_ok=True)
    port = int(drive.GDB_SERVER.rsplit(":", 1)[1])
    pos, fwd, up = floats(args.pos), floats(args.forward), floats(args.up)
    rows = place.basis_rows(fwd, up)

    with drive.Session("data/images/hdfury-ps3-eu-dec.iso", str(out / "logs")) as session:
        gdb = Debugger(port=port)
        mem = rpcs3_hd_field.Mem(session.open_mem())
        read = mem.rd
        gdb.pause()
        managers = poll.scan(read, struct.pack(">I", args.vtable))
        print("objects with vtable %#x: %d" % (args.vtable, len(managers)), flush=True)
        objects = managers if args.objects else []
        base = {o: read(o, args.span) for o in objects}
        if args.objects:
            managers = [managers[0]]
        if len(managers) != 1:
            gdb.resume()
            sys.exit("expected one manager")
        manager = managers[0]
        ship, body = place.find_player(gdb)
        slot = place.u32(gdb.read(ship + SLOT_OFFSET, 4))
        gdb.write(slot + 0x204, struct.pack(">ii", args.state, args.state))
        print("ship %#x body %#x manager %#x clock %.3f" % (ship, body, manager, mem.f32(ship + LAP_CLOCK)), flush=True)

        def step():
            clock0 = mem.f32(ship + LAP_CLOCK)
            gdb.resume()
            deadline = time.time() + args.step_timeout
            while time.time() < deadline:
                now = mem.f32(ship + LAP_CLOCK)
                if now - clock0 >= 1.0 / 60 - 1e-4 or now < clock0:
                    break
                time.sleep(0.002)
            else:
                gdb.pause()
                sys.exit("the lap clock stopped at %.4f" % clock0)
            gdb.pause()
            return mem.f32(ship + LAP_CLOCK) - clock0

        def pool():
            rows_ = []
            if args.objects:
                for i, o in enumerate(objects):
                    blob = read(o, args.span)
                    if blob != base[o]:
                        rows_.append({"slot": i, "addr": "%08x" % o, "raw": blob.hex(), "charging": 0, "age": 0.0,
                                      "pos": (0.0, 0.0, 0.0), "normal": (0.0, 0.0, 0.0), "destroy": 0})
                return rows_
            ptrs = struct.unpack(">16I", read(manager + args.pool, 64))
            for i, p in enumerate(ptrs):
                if not p:
                    continue
                blob = read(p, 0x140)
                flags = struct.unpack_from(">I", blob, 0x40)[0]
                if not flags & 8:
                    continue
                rows_.append({
                    "slot": i, "addr": "%08x" % p, "flags": flags,
                    "charging": blob[0x50], "charge": struct.unpack_from(">f", blob, 0x54)[0],
                    "age": struct.unpack_from(">f", blob, 0x58)[0],
                    "launch_kmh": struct.unpack_from(">f", blob, 0x4c)[0],
                    "pos": struct.unpack_from(">3f", blob, 0xa0),
                    "normal": struct.unpack_from(">3f", blob, 0x110),
                    "destroy": blob[0x12c],
                    "raw": blob.hex(),
                })
            return rows_

        log = open(out / "frames.jsonl", "w")
        for frame in range(args.frames):
            if frame <= args.press_at + args.hold:
                place.write_pose(gdb, body, pos, rows, args.speed)
            if frame == args.press_at:
                session.pad.set(args.button, True)
            if frame == args.press_at + 4:
                session.pad.set(args.button, False)
            dt = step()
            pose = place.read_pose(gdb, body)
            record = {"frame": frame, "dt": dt, "clock": mem.f32(ship + LAP_CLOCK),
                      "craft": pose["pos"], "count": 0 if args.objects else struct.unpack(">I", read(manager + args.pool + 0x40, 4))[0],
                      "pool": pool()}
            log.write(json.dumps(record) + "\n")
            log.flush()
            live = record["pool"]
            print("f%03d dt %.4f count %d %s" % (frame, dt, record["count"], " | ".join(
                "s%d chg%d age %.2f pos (%.1f,%.1f,%.1f) n (%.2f,%.2f,%.2f) d%d" % (
                    r["slot"], r["charging"], r["age"], *r["pos"], *r["normal"], r["destroy"]) for r in live)), flush=True)
        gdb.resume()


if __name__ == "__main__":
    main()
