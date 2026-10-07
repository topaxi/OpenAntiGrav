#!/usr/bin/env python3
"""Poll guest memory of a running HD race on RPCS3 at about 100 Hz without pausing it.

Measured 2026-10-07 (`docs/reverse-engineering/rpcs3-capture.md`, "Polling guest
memory live"): RPCS3 maps the PS3's address space at host `0x300000000`, so a
process that is an ancestor of the emulator (ptrace_scope 1) reads guest address
`A` as `/proc/<pid>/mem` at `0x300000000 + A`. Every other tool here pauses the
target for each read; this one never does, so a time series costs the game
nothing and a one-second effect is sampled a hundred times.

    OAG_RPCS3_DISPLAY=94 OAG_RPCS3_GDB=127.0.0.1:2394 \\
    OAG_RPCS3_PAD_NAME="OAG Pad <lane>" XDG_CONFIG_HOME=... XDG_CACHE_HOME=... \\
        uv run --with evdev python3 scripts/rpcs3-mem-poll.py \\
        --image data/images/hdfury-ps3-eu-dec.iso --out <dir> \\
        --vtable 0x00864b38 --states 10 --gap 12

`--vtable` finds every object of a class (a vtable word at offset 0) in
`0x30000000-0x40000000` before the race starts; the report names the object
addresses. `--manager-of` additionally finds the block that holds the
objects' pointer array (`manager = hit - 0xcc`, the `MissileManager` layout).
Each sample is `(seconds since the first shot, state, blocks)`, pickled to
`<out>/samples.pkl`. A one-second `gdb.pause()` before the first shot leaves a
frozen stretch in the recording: with it, host time maps to video time (about
+5 s in the 2026-10-07 boots).
"""
import argparse
import importlib.util
import pickle
import shutil
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
SLOT_OFFSET = 0x5EDC
STATE = 0x204
CODE_CHECK = (0x155568, bytes.fromhex("39400010"))


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


def behind_rival(gdb, distance):
    """Teleport the player `distance` behind the nearest other craft, with its heading and speed."""
    gdb.pause()
    ships = struct.unpack(">8I", gdb.read(place.CRAFT_ARRAY, 32))
    poses = []
    for ship in ships:
        if ship:
            role = struct.unpack(">I", gdb.read(ship + place.OFF_ROLE, 4))[0]
            body = struct.unpack(">I", gdb.read(ship + place.OFF_BODY, 4))[0]
            poses.append((role, body, place.read_pose(gdb, body)))
    me = next(p for p in poses if p[0] == 0)
    others = [p for p in poses if p[0] != 0]
    near = min(others, key=lambda p: sum((a - b) ** 2 for a, b in zip(p[2]["pos"], me[2]["pos"])))
    pose = near[2]
    fwd = pose["rows"][2]
    speed = sum(v * f for v, f in zip(pose["vel"], fwd))
    pos = tuple(pose["pos"][i] - distance * fwd[i] for i in range(3))
    place.write_pose(gdb, me[1], pos, pose["rows"], speed)
    print("behind rival: speed %.1f units/s, rival at %s" % (speed, tuple(round(c, 1) for c in pose["pos"])),
          flush=True)
    gdb.resume()


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--image", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--states", type=int, nargs="*", default=[])
    ap.add_argument("--gap", type=float, default=12.0)
    ap.add_argument("--vtable", type=lambda s: int(s, 0))
    ap.add_argument("--block", type=lambda s: int(s, 0), default=0x1C0)
    ap.add_argument("--manager-of", action="store_true")
    ap.add_argument("--behind-rival", type=float, default=0.0,
                    help="before each shot, put the player this many units behind the nearest rival, "
                         "on its heading and at its speed")
    args = ap.parse_args()
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    port = int(drive.GDB_SERVER.rsplit(":", 1)[1])
    before = set(drive.recordings())
    with drive.Session(args.image, str(out / "logs")) as session:
        if not session.wait_for_screen_pressing("Main Menu", 240):
            sys.exit("never reached the Main Menu")
        time.sleep(20)
        if session.walk_to_race() not in drive.RACE_ARRIVED:
            sys.exit("no race")
        time.sleep(70)
        session.tap("cross", settle=22)
        gdb = Debugger(port=port)
        gdb.pause()
        ship, _body = place.find_player(gdb)
        slot = struct.unpack(">I", gdb.read(ship + SLOT_OFFSET, 4))[0]
        gdb.resume()
        mem = open("/proc/%d/mem" % session.proc.pid, "rb", 0)

        def read(addr, n):
            mem.seek(GUEST_BASE + addr)
            return mem.read(n)

        if read(CODE_CHECK[0], 4) != CODE_CHECK[1]:
            sys.exit("guest base is not 0x300000000 on this build")
        objs = scan(read, struct.pack(">I", args.vtable)) if args.vtable else []
        blocks = [(o, args.block) for o in objs]
        if args.manager_of and len(objs) >= 2:
            ptrs = struct.pack(">II", objs[0], objs[1])
            blocks = [(m - 0xCC, 0x200) for m in scan(read, ptrs)] + blocks
        print("objects", [hex(o) for o in objs], flush=True)
        print("blocks", [hex(b) for b, _ in blocks], flush=True)
        session.toggle_recording()
        time.sleep(2)
        gdb.pause()
        time.sleep(1.0)
        gdb.resume()
        t0 = time.time() - 1.0
        time.sleep(1.0)
        samples = []
        for st in args.states:
            gdb.pause()
            gdb.write(slot + STATE, struct.pack(">ii", st, st))
            gdb.resume()
            time.sleep(0.5)
            if args.behind_rival:
                behind_rival(gdb, args.behind_rival)
            session.pad.set("triangle", True)
            time.sleep(0.12)
            session.pad.set("triangle", False)
            print("fired", st, "t=%.2f" % (time.time() - t0), flush=True)
            end = time.time() + args.gap
            while time.time() < end:
                samples.append((time.time() - t0, st, [read(b, n) for b, n in blocks]))
                time.sleep(0.01)
        session.toggle_recording()
        time.sleep(8)
        pickle.dump({"objects": objs, "blocks": blocks, "samples": samples},
                    open(out / "samples.pkl", "wb"))
    for path in [p for p in drive.recordings() if p not in before]:
        shutil.copy(path, out / "rec.mp4")
        print("recording ->", out / "rec.mp4")


if __name__ == "__main__":
    main()
