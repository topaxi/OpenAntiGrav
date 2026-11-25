#!/usr/bin/env python3
"""Hold the Detonator cannon on RPCS3 and poll every `PulseBullet` round live, without pausing.

    source data/scratch/<lane>/env.sh
    OAG_COMPARE_PLATFORM=hd-fury OAG_RPCS3_ATTACH=1 UV_CACHE_DIR=~/.cache/uv UV_OFFLINE=1 \\
    uv run --with evdev python3 scripts/emu-run.py --child-status <log-dir>/status.json -- \\
        python3 scripts/rpcs3-hd-pulse-round-poll.py --out data/scratch/<lane>/poll1 \\
            --yaw-deg 35 --fire-clock 6.0 --fire-hold 0.4

Attached to a `serve` restored from a Detonator Racebox grid state (`scripts/emu-restore-state.sh`), this taps
START RACE until the lap clock runs, runs to `--fire-clock`, pauses and turns the craft `--yaw-deg` to its right
(`rpcs3_place.write_pose`: forward `cos f + sin r`, speed kept), then holds the use button and reads the whole
round pool through `/proc/<pid>/mem` as fast as Python goes. The pool is every object whose first word is the
`PulseBullet` vtable (`0x00863d98`, `docs/ghidra/functions/ps3-hdfury-eu/detonator-cannon.md`). A sample of a round
is kept whenever its age (`+0x4c`) changed (`--craft-at` first sets the craft, e.g. at the foot of a loop; `--place` writes one round, which rides only near the craft): `+0x40` flags, `+0x12c` rebounds left, `+0xd0` velocity, `+0xe0` prev,
the rows `+0x140` right, `+0x150` up, `+0x160` forward, `+0x170` position, `+0x180` surface normal. Writes
`<out>/poll.json`.
"""
import argparse
import importlib.util
import json
import math
import struct
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
_spec = importlib.util.spec_from_file_location("rpcs3_drive", HERE / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(drive)
import rpcs3_place as place  # noqa: E402
from rpcs3_debugger import Debugger  # noqa: E402

GUEST_BASE = 0x300000000
PULSE_BULLET_VTABLE = 0x00863D98
CLOCK = 0x7014
SPAN = 0x1C0


def scan(read, needle, lo=0x30000000, hi=0x40000000):
    hits = []
    for a in range(lo, hi, 0x100000):
        try:
            blk = read(a, 0x100000)
        except OSError:
            continue
        i = blk.find(needle)
        while i >= 0:
            if i % 4 == 0:
                hits.append(a + i)
            i = blk.find(needle, i + 4)
    return hits


def decode(blob):
    f = lambda o, n=1: list(struct.unpack_from(">%df" % n, blob, o))
    w = lambda o: struct.unpack_from(">I", blob, o)[0]
    return {
        "age": f(0x4C)[0], "flags": w(0x40), "rebounds": w(0x12C),
        "vel": f(0xD0, 3), "prev": f(0xE0, 3), "right": f(0x140, 3), "up": f(0x150, 3),
        "fwd": f(0x160, 3), "pos": f(0x170, 3), "normal": f(0x180, 3), "launch_speed": f(0x124)[0],
    }


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--image", default="/home/topaxi/projects/OpenAntiGrav/data/images/hdfury-ps3-eu-dec.iso")
    ap.add_argument("--yaw-deg", type=float, default=0.0, help="turn the craft this far to its right before firing")
    ap.add_argument("--fire-clock", type=float, default=6.0)
    ap.add_argument("--fire-hold", type=float, default=0.4)
    ap.add_argument("--settle", type=float, default=0.2, help="wall seconds the turned craft runs before the trigger")
    ap.add_argument("--redirect-deg", type=float, default=0.0,
                    help="turn each newly fired round's velocity this many degrees to the craft's right (alternating left) "
                         "while it is under 0.07 s old, to send it at a side wall; only +0xd0 is written")
    ap.add_argument("--place", default="",
                    help="x,y,z,nx,ny,nz,fx,fy,fz: put the first round fired (under 0.07 s old) at the point with that "
                         "surface normal (also its up row) and a velocity of 500 along the direction; writes +0x170, "
                         "+0xe0, +0x150, +0x180 and +0xd0. The round's floor probe only finds collision near the craft: placed "
                         "700 units away it flew straight; use --craft-at to fire from where the round should ride")
    ap.add_argument("--craft-at", default="",
                    help="x,y,z,ux,uy,uz,fx,fy,fz,speed: put the craft there before firing (`rpcs3_place.write_pose`, "
                         "right = up x forward); the round's floor probe only finds collision near the craft, so a "
                         "round meant for a loop is fired from the loop's foot")
    ap.add_argument("--poll-seconds", type=float, default=2.0)
    ap.add_argument("--pre-wait", type=float, default=30.0)
    ap.add_argument("--fire-button", default="triangle")
    args = ap.parse_args()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    port = int(drive.GDB_SERVER.rsplit(":", 1)[1])
    with drive.Session(args.image, str(out / "logs")) as session:
        print("rpcs3 pid %d" % session.proc.pid, flush=True)
        gdb = Debugger(port=port)
        time.sleep(args.pre_wait)
        for _ in range(60):
            gdb.pause()
            try:
                ship, body = place.find_player(gdb)
                clock = struct.unpack(">f", gdb.read(ship + CLOCK, 4))[0]
            except RuntimeError:
                clock = 0.0
            gdb.resume()
            if clock > 0.5:
                break
            session.tap("cross", settle=6)
        else:
            sys.exit("the race never started")
        for _ in range(400):
            gdb.pause()
            clock = struct.unpack(">f", gdb.read(ship + CLOCK, 4))[0]
            if clock >= args.fire_clock:
                break
            gdb.resume()
            time.sleep(max(0.05, min(0.4, (args.fire_clock - clock) * 0.5)))
        mem = session.open_mem()

        def read(addr, n):
            mem.seek(GUEST_BASE + addr)
            return mem.read(n)

        pool = sorted(scan(read, struct.pack(">I", PULSE_BULLET_VTABLE)))
        print("pool %d objects, first %s, stride %s" % (
            len(pool), hex(pool[0]) if pool else None,
            sorted({b - a for a, b in zip(pool, pool[1:])})[:4]), flush=True)
        pose = place.read_pose(gdb, body)
        if args.yaw_deg:
            psi = math.radians(args.yaw_deg)
            right, up, fwd = pose["rows"]
            nf = tuple(math.cos(psi) * fwd[i] + math.sin(psi) * right[i] for i in range(3))
            nr = tuple(math.cos(psi) * right[i] - math.sin(psi) * fwd[i] for i in range(3))
            speed = sum(v * f for v, f in zip(pose["vel"], fwd))
            place.write_pose(gdb, body, pose["pos"], [nr, up, nf], speed)
            gdb.resume()
            time.sleep(args.settle)
            gdb.pause()
            pose = place.read_pose(gdb, body)
        if args.craft_at:
            c = [float(x) for x in args.craft_at.split(",")]
            up, fwd = c[3:6], c[6:9]
            right = (up[1] * fwd[2] - up[2] * fwd[1], up[2] * fwd[0] - up[0] * fwd[2], up[0] * fwd[1] - up[1] * fwd[0])
            place.write_pose(gdb, body, tuple(c[0:3]), [right, tuple(up), tuple(fwd)], c[9])
            gdb.resume()
            time.sleep(args.settle)
            gdb.pause()
            pose = place.read_pose(gdb, body)
        print("craft pose %s rows %s" % (pose["pos"], pose["rows"]), flush=True)
        last = {}
        seen = set()
        samples = []
        turned = 0
        placed = []
        gdb.resume()
        session.pad.set(args.fire_button, True)
        t0 = time.time()
        released = False
        while time.time() - t0 < args.poll_seconds:
            now = time.time() - t0
            if not released and now > args.fire_hold:
                session.pad.set(args.fire_button, False)
                released = True
            lap = struct.unpack(">f", read(ship + CLOCK, 4))[0]
            for addr in pool:
                blob = read(addr, SPAN)
                age = struct.unpack_from(">f", blob, 0x4C)[0]
                if last.get(addr) == age:
                    continue
                last[addr] = age
                if not (0.0 < age < 3.0):
                    continue
                row = decode(blob)
                if args.place and not placed and age < 0.07:
                    placed.append(addr)
                    v = [float(x) for x in args.place.split(",")]
                    gdb.pause()
                    gdb.write(addr + 0x170, struct.pack(">3f", *v[0:3]))
                    gdb.write(addr + 0xE0, struct.pack(">3f", *v[0:3]))
                    gdb.write(addr + 0x150, struct.pack(">3f", *v[3:6]))
                    gdb.write(addr + 0x180, struct.pack(">3f", *v[3:6]))
                    gdb.write(addr + 0xD0, struct.pack(">3f", *[500.0 * c for c in v[6:9]]))
                    gdb.resume()
                    row["placed"] = v
                if args.redirect_deg and addr not in seen and age < 0.07:
                    seen.add(addr)
                    a = math.radians(args.redirect_deg) * (1 if turned % 2 == 0 else -1)
                    turned += 1
                    right, _up, fwd = pose["rows"]
                    v = [500.0 * (math.cos(a) * fwd[i] + math.sin(a) * right[i]) for i in range(3)]
                    gdb.pause()
                    gdb.write(addr + 0xD0, struct.pack(">3f", *v))
                    gdb.resume()
                    row["redirected_to"] = v
                row.update(t=round(now, 4), lap=lap, addr=addr)
                samples.append(row)
        if not released:
            session.pad.set(args.fire_button, False)
        gdb.pause()
        final = place.read_pose(gdb, body)
        gdb.resume()
        (out / "poll.json").write_text(json.dumps({"pool": pool, "start_pose": pose, "end_pose": final,
                                                    "yaw_deg": args.yaw_deg, "placed": placed,
                                                    "samples": samples}) + "\n")
        print("done: %d samples, %d distinct rounds" % (len(samples), len({s["addr"] for s in samples})), flush=True)


if __name__ == "__main__":
    main()
