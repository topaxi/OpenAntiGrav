#!/usr/bin/env python3
"""Does HD's frictionless-floor spark ride its locator? A live read of the spark's `LinkObj` node over frames.

    OAG_RPCS3_ATTACH=1 uv run --with evdev python3 scripts/rpcs3-hd-floor-spark-ride.py --image ISO \\
        [--out DIR] [--push 3.0] [--frames 24]

`ShipCollisionFx_Trigger` kind 2 (`0x002d9d18`) makes a new 0x180-byte `LinkObj` (vtable word `0x008745a0`
at `node + 8`) per floor record, a child of the nearest of the craft's ten locator nodes
(`ship + 0x79d0`), with an identity local matrix at `node + 0xf0..+0x130`. Whether that matrix then follows
the locator is the question (`docs/ghidra/functions/ps3-hdfury-eu/collision-cues.md`, "What kind 2 spawns").

In a race on an attached RPCS3 (restore any HD race state, `emu-restore-state.sh`) it taps START RACE until the
lap clock runs to `--run-to` seconds, scans the heap for `LinkObj` nodes, pushes the player's body `--push`
units into the road along its own down axis and steps single game frames (the lap clock advanced 1/60 s a step).
After each step it scans again; a new node whose parent word (`node + 0xc`) is one of the craft's locator nodes
is a candidate spark, and from then on every frame prints the node's translation row (`+0x120`), its parent's,
and the craft's body position. `--out` keeps `trace.json`; `--dump-instances` is always on: the particle instance of each node (found by the back pointer at `+0x2c` and a scale-block slot 0 of `1.0` at `+0x30`) is
kept raw per frame for a diff. `--shots` adds a screenshot per frame once a node
exists (a reset-plane contact teleports the craft, so a spark that rides its locator jumps with the hull and one that
stays is left at the contact).
"""
import argparse
import importlib.util
import json
import struct
import sys
import time
from pathlib import Path
import os

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import oag_tmp  # noqa: E402

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))


def load(name, file):
    spec = importlib.util.spec_from_file_location(name, HERE / file)
    mod = importlib.util.module_from_spec(spec)
    sys.modules[name] = mod
    spec.loader.exec_module(mod)
    return mod


poll = load("rpcs3_mem_poll", "rpcs3-mem-poll.py")
drive = poll.drive
place = poll.place
import rpcs3_hd_field  # noqa: E402
from rpcs3_debugger import Debugger  # noqa: E402

LINKOBJ_VTABLE = 0x008745A0
LAP_CLOCK = 0x7014
LOCATORS = 0x79D0
TRANSLATION = 0x120


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--image", required=True)
    ap.add_argument("--out", type=Path)
    ap.add_argument("--push", type=float, default=3.0, help="units the body is pushed along its own down axis")
    ap.add_argument("--run-to", type=float, default=4.0, help="lap clock seconds before the push")
    ap.add_argument("--frames", type=int, default=30)
    ap.add_argument("--shots", action="store_true", help="a screenshot of every paused frame once a spark node exists")
    ap.add_argument("--flush", type=float, default=1.5)
    ap.add_argument("--pushes", type=int, default=3, help="frames the push is repeated")
    args = ap.parse_args()
    out = args.out or Path(oag_tmp.output("floor-spark"))
    out.mkdir(parents=True, exist_ok=True)
    port = int(drive.GDB_SERVER.rsplit(":", 1)[1])
    with drive.Session(args.image, str(out / "logs")) as session:
        gdb = Debugger(port=port)
        mem = rpcs3_hd_field.Mem(session.open_mem())

        def nodes():
            return {a - 8 for a in poll.scan(mem.rd, struct.pack(">I", LINKOBJ_VTABLE))}

        for _ in range(60):
            gdb.pause()
            try:
                ship, body = place.find_player(gdb)
                clock = mem.f32(ship + LAP_CLOCK)
            except RuntimeError:
                clock = 0.0
            if clock >= args.run_to:
                break
            gdb.resume()
            if clock < 0.5:
                session.tap("cross", settle=6)
            else:
                time.sleep(2)
        else:
            sys.exit("the race never reached %.1f s on the lap clock" % args.run_to)
        session.pad.set("cross", True)
        locs = [mem.u32(ship + LOCATORS + 4 * i) for i in range(10)]
        print("player %#x body %#x locators %s" % (ship, body, [hex(x) for x in locs]), flush=True)
        before = nodes()
        print("%d LinkObj node(s) before" % len(before), flush=True)

        def step():
            clock0 = mem.f32(ship + LAP_CLOCK)
            gdb.resume()
            deadline = time.time() + 8
            while time.time() < deadline:
                now = mem.f32(ship + LAP_CLOCK)
                if now - clock0 >= 1.0 / 60 - 1e-4 or now < clock0:
                    break
                time.sleep(0.002)
            gdb.pause()

        trace = []
        sparks = {}
        instances = None
        for n in range(args.frames):
            if n < args.pushes:
                pose = place.read_pose(gdb, body)
                down = [-x for x in pose["rows"][1]]
                pos = tuple(pose["pos"][i] + down[i] * args.push for i in range(3))
                place.write_pose(gdb, body, pos, [tuple(x) for x in pose["rows"]],
                                 sum(v * v for v in pose["vel"]) ** 0.5)
                gdb.write(body + place.B_VELOCITY, struct.pack(">3f", *pose["vel"]))
            step()
            pose = place.read_pose(gdb, body)
            now = nodes()
            for node in sorted(now - before - set(sparks)):
                parent = mem.u32(node + 0xC)
                if parent in locs:
                    sparks[node] = parent
                    print("frame %d: new node %#x, child of locator %#x" % (n, node, parent), flush=True)
            row = {"frame": n, "body": pose["pos"], "clock": mem.f32(ship + LAP_CLOCK), "new": len(now - before),
                   "sparks": {}}
            for node, parent in sparks.items():
                if node in now:
                    row["sparks"][hex(node)] = {
                        "node": struct.unpack(">4f", mem.rd(node + TRANSLATION, 16)),
                        "parent": struct.unpack(">4f", mem.rd(parent + TRANSLATION, 16)),
                    }
            if sparks and instances is None:
                instances = {}
                for node in sparks:
                    for hit in poll.scan(mem.rd, struct.pack(">I", node)):
                        if hit - 0x2C != node and mem.u32(hit - 0x2C + 0x30) == 0x3F800000:
                            instances[hit - 0x2C] = node
                print("instances (back pointer at +0x2c): %s" % {hex(i): hex(n) for i, n in instances.items()}, flush=True)
            if instances:
                row["instances"] = {hex(i): mem.rd(i, 0x200).hex() for i in list(instances)[:6]}
            trace.append(row)
            print(json.dumps({k: v for k, v in row.items() if k != "instances"}), flush=True)
            if sparks and args.shots:
                time.sleep(args.flush)
                drive.screenshot(out / ("f%02d.png" % n), trim=True)
        session.pad.set("cross", False)
        gdb.resume()
    (out / "trace.json").write_text(json.dumps(trace, indent=1))


if __name__ == "__main__":
    main()
