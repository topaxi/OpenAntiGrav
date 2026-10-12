#!/usr/bin/env python3
"""Dump the player's body pose, its scene-graph nodes and the newest anchor-ring record in HD.

The evidence reproducer for the LeachBeam anchor reading in
`docs/ghidra/functions/ps3-hdfury-eu/leach-beam-strips.md`. Runs on any paused
frame (the countdown works): `LeachTrail_RecordAnchor` records every craft every tick.

    OAG_RPCS3_DISPLAY=93 ... uv run --with evdev python3 scripts/rpcs3-hd-anchor.py \\
        --image data/images/hdfury-ps3-eu-dec.iso --log-dir <dir> [--countdown 6]

Writes `<log-dir>/anchor.json`: for each of the eight craft the body rows and
position (`+0x1d0..+0x200` of the rigid body), the ring head and count, the newest
ring records, and a walk of the craft's scene-graph children (`+0x14` child,
`+0x10` sibling) with every node's first 0x100 bytes and the name block at
`*(node+0xc8)+0x10`.
"""
import argparse
import importlib.util
import json
import struct
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
_spec = importlib.util.spec_from_file_location("rpcs3_drive", HERE / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(drive)
import rpcs3_place as place  # noqa: E402
from rpcs3_debugger import Debugger  # noqa: E402

RING = 0x110
HEAD = 0x5ED0
COUNT = 0x5ED4
RECORD = 0x50
NODE_SPAN = 0x100
MAX_NODES = 400


def u32(b, at=0):
    return struct.unpack_from(">I", b, at)[0]


def valid(p):
    return 0x00010000 <= p < 0xE0000000 and p % 4 == 0


def read(gdb, addr, size):
    out = b""
    while len(out) < size:
        n = min(0x200, size - len(out))
        out += gdb.read(addr + len(out), n)
    return out


def walk(gdb, root):
    seen, order, stack = set(), [], [(root, 0)]
    while stack and len(order) < MAX_NODES:
        node, depth = stack.pop()
        if not valid(node) or node in seen:
            continue
        seen.add(node)
        try:
            blob = read(gdb, node, NODE_SPAN)
        except Exception:
            continue
        info = {"addr": node, "depth": depth, "hex": blob.hex()}
        name = u32(blob, 0xC8)
        if valid(name):
            try:
                info["name_hex"] = read(gdb, name + 0x10, 0x40).hex()
            except Exception:
                pass
        order.append(info)
        stack.append((u32(blob, 0x10), depth))
        stack.append((u32(blob, 0x14), depth + 1))
    return order


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--image", required=True)
    ap.add_argument("--log-dir", required=True)
    ap.add_argument("--countdown", type=float, default=6.0)
    args = ap.parse_args()
    out = Path(args.log_dir)
    out.mkdir(parents=True, exist_ok=True)
    port = int(drive.GDB_SERVER.rsplit(":", 1)[1])
    with drive.Session(args.image, str(out / "logs")) as session:
        if not session.wait_for_screen_pressing("Main Menu", 240):
            sys.exit("never reached the Main Menu")
        session.settle_menu(20)
        if session.walk_to_race() not in drive.RACE_ARRIVED:
            sys.exit("no race")
        session.wait_for_load(70)
        session.tap("cross", settle=args.countdown)
        gdb = Debugger(port=port)
        gdb.pause()
        ships = struct.unpack(">8I", gdb.read(place.CRAFT_ARRAY, 32))
        result = []
        for slot, ship in enumerate(ships):
            if not valid(ship):
                continue
            body = u32(gdb.read(ship + place.OFF_BODY, 4))
            pose = place.read_pose(gdb, body)
            head = u32(gdb.read(ship + HEAD, 4))
            count = u32(gdb.read(ship + COUNT, 4))
            ring = read(gdb, ship + RING, 300 * RECORD)
            entry = {
                "slot": slot,
                "ship": ship,
                "body": body,
                "role": u32(gdb.read(ship + place.OFF_ROLE, 4)),
                "pose": {"pos": list(pose["pos"]), "rows": [list(r) for r in pose["rows"]], "vel": list(pose["vel"])},
                "head": head,
                "count": count,
                "ring_newest_hex": ring[(head % 300) * RECORD:(head % 300 + 1) * RECORD].hex(),
                "ship_hex": read(gdb, ship, 0x300).hex(),
                "nodes": walk(gdb, ship),
            }
            result.append(entry)
            print("slot %d ship %#x head %d count %d nodes %d" % (slot, ship, head, count, len(entry["nodes"])), flush=True)
        gdb.resume()
        (out / "anchor.json").write_text(json.dumps(result))
        drive.screenshot(out / "race.png")


if __name__ == "__main__":
    main()
