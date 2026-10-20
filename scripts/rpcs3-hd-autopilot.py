#!/usr/bin/env python3
"""Fly a whole HD race on the original's Autopilot and log the craft per frame (RPCS3).

Attaches to a running `rpcs3-drive.py serve` (`OAG_RPCS3_ATTACH=1`) sitting on
Track Select (or on the EndRace menu's Race Again) with the circuit, class and
weapons already chosen. It writes Pilot Assist off, enters the race, presses
START RACE until the craft leaves its grid state, then arms the Autopilot by
setting the pickup slot's running bit `0x800` (`*(ship+0x5edc)+0x200`) and
holding its timer `+0x158` at 10000, and samples until `--seconds` pass.

Each row of `<out>/samples.csv` is one new body position: the craft's engine
flags `craft+0x264`, multiplier `+0x2f0`, blend `+0x278`, speed `+0x340`,
throttle `+0x30c`, grounded fraction `+0x304`, position and velocity, the
shield `ship+0x5fa0`, the slot's flags and timer, and the five control floats at
`craft+0x4c` (steer, pitch, airbrakes; `craft+0x84` points there under the
Autopilot). Key on position, not on `craft+0x308`: that clock stood still for a
whole 2 s jump on Ubermall. Writing the slot's state word `6` before GO crashed
the game; the bit alone, after GO, did not. See
`docs/ghidra/functions/ps3-hdfury-eu/autopilot.md`.
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
OPTIONS_PTR = 0x008B098C


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--image", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--seconds", type=float, default=300.0)
    ap.add_argument("--no-arm", action="store_true")
    ap.add_argument("--skip-walk", action="store_true")
    ap.add_argument("--ramp", action="store_true",
                    help="sample from the grid on, through GO and the hover release ramp, unarmed")
    args = ap.parse_args()
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    port = int(drive.GDB_SERVER.rsplit(":", 1)[1])
    with drive.Session(args.image, str(out / "logs")) as session:
        gdb = Debugger(port=port)
        gdb.pause()
        holder = struct.unpack(">I", gdb.read(OPTIONS_PTR, 4))[0]
        opts = struct.unpack(">I", gdb.read(holder, 4))[0]
        before = gdb.read(opts + 0x473, 2)
        gdb.write(opts + 0x473, b"\x00\x00")
        gdb.resume()
        print("pilot assist", before.hex(), "-> 0000", flush=True)
        if not args.skip_walk:
            if session.walk_to_race() not in drive.RACE_ARRIVED:
                sys.exit("no race")
            session.wait_for_load(70)
        time.sleep(0 if args.ramp else 8)
        gdb.pause()
        ship, body = place.find_player(gdb)
        slot = struct.unpack(">I", gdb.read(ship + SLOT, 4))[0]
        craft = struct.unpack(">I", gdb.read(ship + 0x5FAC, 4))[0]
        gdb.resume()
        print("ship %#x body %#x craft %#x slot %#x" % (ship, body, craft, slot), flush=True)
        mem = session.open_mem()
        # Tap cross (START RACE) until the craft leaves its grid state (flag 0x2)
        # and is moving.
        deadline = time.time() + (0 if args.ramp else 120)
        while time.time() < deadline:
            mem.seek(GUEST + craft + 0x264)
            fl = struct.unpack(">I", mem.read(4))[0]
            mem.seek(GUEST + craft + 0x340)
            sp = struct.unpack(">f", mem.read(4))[0]
            print("waiting: flags %#x speed %.1f" % (fl, sp), flush=True)
            if not fl & 0x2:
                break
            if fl & 0x2:
                session.pad.set("cross", True)
                time.sleep(0.15)
                session.pad.set("cross", False)
            time.sleep(4)
        if not args.ramp:
            time.sleep(2)

        def read(addr, n):
            mem.seek(GUEST + addr)
            return mem.read(n)

        def f32(addr):
            return struct.unpack(">f", read(addr, 4))[0]

        def u32(addr):
            return struct.unpack(">I", read(addr, 4))[0]

        if not (args.no_arm or args.ramp):
            gdb.pause()
            w = struct.unpack(">I", gdb.read(slot + 0x200, 4))[0]
            gdb.write(slot + 0x200, struct.pack(">I", w | 0x800))
            gdb.write(slot + 0x158, struct.pack(">f", 10000.0))
            gdb.resume()
        print("armed" if not (args.no_arm or args.ramp) else "not armed", "slot flags %#x timer %.1f" % (u32(slot + 0x200), f32(slot + 0x158)), flush=True)
        rows = []
        if getattr(session, "guard", None) is not None:
            session.guard.stage("capture", limit=args.seconds + 120)
        fh = open(out / "samples.csv", "w")
        fh.write("t,clock,flags,mult,blend,speed,throttle,grounded,px,py,pz,vx,vy,vz,shield,slot_flags,slot_timer,c0,c1,c2,c3,c4,ctlptr_minus_craft,"
                 "wx,wy,wz,r0x,r0y,r0z,upx,upy,upz,fwx,fwy,fwz,hupx,hupy,hupz,sdx,sdy,sdz,team_wd,"
                 "target,timer,height,state\n")
        team = u32(craft + 0x78)
        print("team block %#x weight_distribution (+0x9c) %.3f" % (team, f32(team + 0x9c)), flush=True)
        t0 = time.time()
        last_pos = None
        last_print = 0
        last_tap = 0.0
        while time.time() - t0 < args.seconds:
            # `--ramp`: tap START RACE from inside the sampler, so the countdown, GO and the
            # release ramp are all rows (`hover-target.md`: the target `+0x344`, its clamp
            # `+0x348`, the height `+0x260` and the entry state `+0x2f8`).
            if args.ramp and u32(craft + 0x264) & 0x2 and time.time() - last_tap > 4:
                last_tap = time.time()
                session.pad.set("cross", True)
                time.sleep(0.15)
                session.pad.set("cross", False)
            clock = f32(craft + 0x308)
            pos_bytes = read(body + 0x200, 12)
            if pos_bytes == last_pos:
                if time.time() - last_print > 10:
                    last_print = time.time()
                    print("clock %.2f, rows %d" % (clock, len(rows)), flush=True)
                time.sleep(0.004)
                continue
            last_pos = pos_bytes
            ctl = struct.unpack(">5f", read(craft + 0x4c, 20)) + (float(u32(craft + 0x84) - craft),)
            b = read(body + 0x190, 0x80)
            vel = struct.unpack_from(">3f", b, 0)
            pos = struct.unpack(">3f", read(body + 0x200, 12))
            cr = read(craft + 0x264, 0xe0)
            flags = struct.unpack_from(">I", cr, 0)[0]
            blend = struct.unpack_from(">f", cr, 0x278 - 0x264)[0]
            mult = struct.unpack_from(">f", cr, 0x2f0 - 0x264)[0]
            grounded = struct.unpack_from(">f", cr, 0x304 - 0x264)[0]
            throttle = struct.unpack_from(">f", cr, 0x30c - 0x264)[0]
            speed = struct.unpack_from(">f", cr, 0x340 - 0x264)[0]
            shield = f32(ship + SHIELD)
            sflags = u32(slot + 0x200)
            stimer = f32(slot + 0x158)
            # `--pose`: angular velocity `body+0x1a0`, the basis rows `+0x1d0..+0x1f0`, the
            # craft's up `craft+0x200` and the located sample's down `ship+0x7850` (the
            # pitch gate's two inputs, `Craft_UpdatePitch` `0x000f0980`), and the team
            # block's `+0x9c` (`<Misc weight_distribution>`, the in-air pitch bias).
            pose = (struct.unpack_from(">3f", b, 0x10) + struct.unpack_from(">3f", b, 0x40)
                    + struct.unpack_from(">3f", b, 0x50) + struct.unpack_from(">3f", b, 0x60)
                    + struct.unpack(">3f", read(craft + 0x200, 12))
                    + struct.unpack(">3f", read(ship + 0x7850, 12)) + (f32(team + 0x9c),)
                    + (f32(craft + 0x344), f32(craft + 0x348), f32(craft + 0x260), u32(craft + 0x2f8)))
            rows.append((time.time() - t0, clock, flags, mult, blend, speed, throttle, grounded,
                         *pos, *vel, shield, sflags, stimer, *ctl, *pose))
            fh.write(",".join(str(x) for x in rows[-1]) + "\n")
            if len(rows) % 200 == 0:
                fh.flush()
            if time.time() - last_print > 10:
                last_print = time.time()
                print("t %.0f clock %.2f flags %#x mult %.2f blend %.2f speed %.1f thr %.1f pos (%.0f,%.0f,%.0f) shield %.1f slot %#x %.0f"
                      % (rows[-1][0], clock, flags, mult, blend, speed, throttle, *pos, shield, sflags, stimer), flush=True)
        fh.close()
        print("rows", len(rows), flush=True)


if __name__ == "__main__":
    main()
