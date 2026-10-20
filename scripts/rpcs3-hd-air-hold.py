#!/usr/bin/env python3
"""Place the HD player craft before a jump, hold the throttle, and log every frame (RPCS3).

Player input only: the original's Autopilot is never armed (its rail writes the
craft's velocity, so a law measured under it is not a player's law). Attach to a
running `rpcs3-drive.py serve` (`OAG_RPCS3_ATTACH=1`) sitting in a race after GO,
for example a restored Time Trial state.

Each `--run name=x,y,z,fx,fy,fz,ux,uy,uz,speed` (or `name=here,lift,speed`, a drop test from
the craft's own pose) is one repetition:

1. hold cross (and the stick, `--stick-y`) for `--spool` seconds, so the throttle
   `craft+0x30c` is already up when the craft is placed;
2. pause, write the pose with `rpcs3_place.write_pose` (velocity along forward,
   rate zeroed), resume with the same input still held;
3. log one row per new body position for `--seconds` host seconds.

A row carries what `rpcs3-hd-autopilot.py` logs (body velocity, rate `body+0x1a0`,
basis rows, position, the craft's up `craft+0x200`, the located sample's down
`ship+0x7850`, flags `craft+0x264`, grounded `craft+0x304`, throttle `+0x30c`, the
team block's `+0x9c`, the hover timer `+0x348` and state `+0x2f8`) plus the five
control floats read **through `craft+0x84`** (the record the craft steps with) and the pickup
slot's flags, whose bit `0x800`
is the Autopilot's running bit and must read clear on every row.

Writes `<out>/<name>.csv`.
"""
import argparse
import importlib.util
import struct
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parent
sys.path.insert(0, str(REPO))
_spec = importlib.util.spec_from_file_location("rpcs3_drive", REPO / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(drive)
import rpcs3_place as place  # noqa: E402
from rpcs3_debugger import Debugger  # noqa: E402

GUEST = 0x300000000
SLOT = 0x5EDC
SHIELD = 0x5FA0
CRAFT = 0x5FAC

HEADER = ("t,clock,flags,mult,blend,speed,throttle,grounded,px,py,pz,vx,vy,vz,shield,slot_flags,"
          "slot_timer,c0,c1,c2,c3,c4,ctlptr_minus_craft,wx,wy,wz,r0x,r0y,r0z,upx,upy,upz,"
          "fwx,fwy,fwz,hupx,hupy,hupz,sdx,sdy,sdz,team_wd,target,timer,height,state\n")


def parse_run(text):
    """`name=x,y,z,fx,fy,fz,ux,uy,uz,speed`, or `name=here,lift,speed`: the craft's own pose at
    the moment of placement, raised `lift` units along world y (a drop test)."""
    name, _, pose = text.partition("=")
    parts = pose.split(",")
    if parts[0] == "here" and len(parts) == 3:
        return name, ("here", float(parts[1]), float(parts[2]))
    v = [float(p) for p in parts]
    if len(v) != 10:
        raise SystemExit("--run name=x,y,z,fx,fy,fz,ux,uy,uz,speed or name=here,lift,speed: %r"
                         % text)
    return name, v


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--image", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--run", action="append", required=True)
    ap.add_argument("--seconds", type=float, default=6.0)
    ap.add_argument("--spool", type=float, default=2.5)
    ap.add_argument("--stick-y", type=float, default=0.0,
                    help="left stick y held through the run, -1..1 (0 centred)")
    args = ap.parse_args()
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    port = int(drive.GDB_SERVER.rsplit(":", 1)[1])
    with drive.Session(args.image, str(out / "logs")) as session:
        gdb = Debugger(port=port)
        gdb.pause()
        ship, body = place.find_player(gdb)
        slot = struct.unpack(">I", gdb.read(ship + SLOT, 4))[0]
        craft = struct.unpack(">I", gdb.read(ship + CRAFT, 4))[0]
        gdb.resume()
        print("ship %#x body %#x craft %#x slot %#x" % (ship, body, craft, slot), flush=True)
        mem = session.open_mem()

        def read(addr, n):
            mem.seek(GUEST + addr)
            return mem.read(n)

        def f32(addr):
            return struct.unpack(">f", read(addr, 4))[0]

        def u32(addr):
            return struct.unpack(">I", read(addr, 4))[0]

        team = u32(craft + 0x78)
        print("team block %#x +0x9c %.3f" % (team, f32(team + 0x9c)), flush=True)
        for spec in args.run:
            name, v = parse_run(spec)
            session.pad.set("cross", True)
            session.pad.stick(0.0, args.stick_y)
            time.sleep(args.spool)
            gdb.pause()
            if v[0] == "here":
                now = place.read_pose(gdb, body)
                p0 = now["pos"]
                pos, rows, speed = (p0[0], p0[1] + v[1], p0[2]), now["rows"], v[2]
            else:
                pos, fw, up, speed = v[0:3], v[3:6], v[6:9], v[9]
                rows = place.basis_rows(fw, up)
            place.write_pose(gdb, body, pos, rows, speed)
            gdb.resume()
            if session.guard is not None:
                session.guard.stage("capture:" + name, limit=args.seconds + 60)
            fh = open(out / (name + ".csv"), "w")
            fh.write(HEADER)
            t0 = time.time()
            last = None
            n = 0
            armed = 0
            while time.time() - t0 < args.seconds:
                pos_bytes = read(body + 0x200, 12)
                if pos_bytes == last:
                    time.sleep(0.002)
                    continue
                last = pos_bytes
                ctlptr = u32(craft + 0x84)
                ctl = struct.unpack(">5f", read(ctlptr + 0, 20)) if ctlptr else (0.0,) * 5
                b = read(body + 0x190, 0x80)
                vel = struct.unpack_from(">3f", b, 0)
                p = struct.unpack(">3f", pos_bytes)
                cr = read(craft + 0x264, 0xe8)
                g = lambda off: struct.unpack_from(">f", cr, off - 0x264)[0]  # noqa: E731
                flags = struct.unpack_from(">I", cr, 0)[0]
                sflags = u32(slot + 0x200)
                armed += bool(sflags & 0x800)
                pose = (struct.unpack_from(">3f", b, 0x10) + struct.unpack_from(">3f", b, 0x40)
                        + struct.unpack_from(">3f", b, 0x50) + struct.unpack_from(">3f", b, 0x60)
                        + struct.unpack(">3f", read(craft + 0x200, 12))
                        + struct.unpack(">3f", read(ship + 0x7850, 12)) + (f32(team + 0x9c),)
                        + (g(0x344), g(0x348), f32(craft + 0x260), u32(craft + 0x2f8)))
                row = (time.time() - t0, g(0x308), flags, g(0x2f0), g(0x278), g(0x340), g(0x30c),
                       g(0x304), *p, *vel, f32(ship + SHIELD), sflags, f32(slot + 0x158), *ctl,
                       ctlptr - craft, *pose)
                fh.write(",".join(str(x) for x in row) + "\n")
                n += 1
            fh.close()
            session.pad.set("cross", False)
            session.pad.stick(0.0, 0.0)
            print("%s: rows %d, autopilot bit set on %d" % (name, n, armed), flush=True)
            time.sleep(0.5)


if __name__ == "__main__":
    main()
