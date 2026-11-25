#!/usr/bin/env python3
"""Read the matrix `PulseBullet_Update` hands its wall spark, off the live particle instance.

    source data/scratch/<lane>/env.sh
    scripts/emu-restore-state.sh <lane> data/saves/hd-fury/hd-detonator-talons-feisar-grid.SAVESTAT.zst
    OAG_COMPARE_PLATFORM=hd-fury OAG_RPCS3_ATTACH=1 uv run --with evdev python3 scripts/emu-run.py \\
        --child-status <log-dir>/status.json -- \\
        python3 scripts/rpcs3-hd-pulse-spark-poll.py --out data/scratch/<lane>/spark1

No breakpoint (the recompiler is kept): the cannon is held, every new round's velocity is turned
`--angle-deg` to the craft's right or left (alternating, only `+0xd0` written) and its rebound count
cleared (`+0x12c`), so its first wall is its death. When a round's wall-death flags appear the game is
paused and every particle-system instance (vtable `0x0086a070`) is read, with its 4x4 matrix at
`+0xf0`. Instances present before the first death are listed too, so a new one is told from an old one.
`docs/ghidra/functions/ps3-hdfury-eu/detonator-cannon.md`, "The spark's matrix, read live".
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


def _load(name, file):
    spec = importlib.util.spec_from_file_location(name, HERE / file)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


drive = _load("rpcs3_drive", "rpcs3-drive.py")
poll = _load("pulse_poll", "rpcs3-hd-pulse-round-poll.py")
import rpcs3_place as place  # noqa: E402
from rpcs3_debugger import Debugger  # noqa: E402

INSTANCE_VTABLE = 0x0086A070
INSTANCE_SPAN = 0x180
WALL_DEATH = 0x14


def instance(blob, addr):
    return {
        "addr": addr,
        "matrix": [list(struct.unpack_from(">4f", blob, 0xF0 + 16 * row)) for row in range(4)],
        "matrix_ptr": struct.unpack_from(">I", blob, 0xD0)[0],
        "effect": struct.unpack_from(">I", blob, 0x134)[0],
        "spawn_flags": struct.unpack_from(">I", blob, 0xE0)[0],
    }


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--image", default="/home/topaxi/projects/OpenAntiGrav/data/images/hdfury-ps3-eu-dec.iso")
    ap.add_argument("--angle-deg", type=float, default=30.0)
    ap.add_argument("--hits", type=int, default=2)
    ap.add_argument("--fire-clock", type=float, default=6.0)
    ap.add_argument("--grab-after", default="0.1,0.3",
                    help="comma-separated seconds of running after a death at which to grab "
                         "(`<out>/hit-N-K.png`)")
    ap.add_argument("--seconds", type=float, default=120.0)
    args = ap.parse_args()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    port = int(drive.GDB_SERVER.rsplit(":", 1)[1])
    with drive.Session(args.image, str(out / "logs")) as session:
        print("rpcs3 pid %d" % session.proc.pid, flush=True)
        gdb = Debugger(port=port)
        time.sleep(20)
        for _ in range(60):
            gdb.pause()
            try:
                ship, body = place.find_player(gdb)
                clock = struct.unpack(">f", gdb.read(ship + poll.CLOCK, 4))[0]
            except RuntimeError:
                clock = 0.0
            gdb.resume()
            if clock > 0.5:
                break
            session.tap("cross", settle=6)
        else:
            sys.exit("the race never started")
        while clock < args.fire_clock:
            time.sleep(max(0.05, (args.fire_clock - clock) * 0.5))
            gdb.pause()
            clock = struct.unpack(">f", gdb.read(ship + poll.CLOCK, 4))[0]
            gdb.resume()
        mem = session.open_mem()

        def read(addr, n):
            mem.seek(poll.GUEST_BASE + addr)
            return mem.read(n)

        pool = sorted(poll.scan(read, struct.pack(">I", poll.PULSE_BULLET_VTABLE)))
        needle = struct.pack(">I", INSTANCE_VTABLE)

        def instances():
            return [instance(read(a, INSTANCE_SPAN), a) for a in poll.scan(read, needle)]

        gdb.pause()
        pose = place.read_pose(gdb, body)
        before = instances()
        gdb.resume()
        print("pool %d, %d instances before, pose %s" % (len(pool), len(before), pose["pos"]), flush=True)
        right, _up, fwd = pose["rows"]
        known = {i["addr"] for i in before}
        history = {}
        turned = 0
        dead = set()
        hits = []
        session.pad.set("triangle", True)
        t0 = time.time()
        while time.time() - t0 < args.seconds and len(hits) < args.hits:
            for addr in pool:
                blob = read(addr, poll.SPAN)
                row = poll.decode(blob)
                age = row["age"]
                if not (0.0 < age < 3.0):
                    continue
                track = history.setdefault(addr, [])
                if not track or track[-1]["age"] != age:
                    if track and age < track[-1]["age"]:
                        track.clear()
                        dead.discard(addr)
                    track.append(row)
                if age < 0.07 and len(track) == 1 and not row.get("turned"):
                    a = math.radians(args.angle_deg) * (1 if turned % 2 == 0 else -1)
                    turned += 1
                    v = [500.0 * (math.cos(a) * fwd[i] + math.sin(a) * right[i]) for i in range(3)]
                    gdb.pause()
                    gdb.write(addr + 0xD0, struct.pack(">3f", *v))
                    gdb.write(addr + 0x12C, struct.pack(">I", 0))
                    gdb.resume()
                    track[0]["turned"] = v
                if row["flags"] & WALL_DEATH == WALL_DEATH and addr not in dead:
                    dead.add(addr)
                    gdb.pause()
                    final = poll.decode(read(addr, poll.SPAN))
                    now = instances()
                    new = [i for i in now if i["addr"] not in known]
                    known.update(i["addr"] for i in now)
                    hit = {"round_addr": addr, "final": final, "history": track[-6:], "new_instances": new,
                           "camera_pose": place.read_pose(gdb, body)}
                    start = struct.unpack(">f", gdb.read(ship + poll.CLOCK, 4))[0]
                    hit["clock"] = start
                    hit["grabs"] = []
                    previous = 0.0
                    for k, delay in enumerate(float(d) for d in args.grab_after.split(",")):
                        gdb.resume()
                        time.sleep(delay - previous)
                        previous = delay
                        gdb.pause()
                        clock = struct.unpack(">f", gdb.read(ship + poll.CLOCK, 4))[0]
                        name = "hit-%d-%d.png" % (len(hits), k)
                        drive.grab_with_clock(out / name, trim=True)
                        hit["grabs"].append({"image": name, "clock": clock,
                                             "camera_pose": place.read_pose(gdb, body)})
                    gdb.resume()
                    hits.append(hit)
                    print("hit %d: %d new instance(s) at clock %.3f" % (len(hits), len(new), start), flush=True)
                    for i in new:
                        print("  ", hex(i["addr"]), i["matrix"], flush=True)
        session.pad.set("triangle", False)
        (out / "spark.json").write_text(json.dumps(
            {"pose": pose, "angle_deg": args.angle_deg, "before": len(before), "hits": hits}) + "\n")
        print("done: %d hit(s)" % len(hits), flush=True)


if __name__ == "__main__":
    main()
