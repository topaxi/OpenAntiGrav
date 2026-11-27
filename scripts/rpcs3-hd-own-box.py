#!/usr/bin/env python3
"""Does HD's firing craft's own box answer a projectile's probe? Logged off a running RPCS3.

    scripts/emu-restore-state.sh <lane> data/saves/hd-fury/hd-racebox-talons-venom-weapons-grid-rpcs3-20574.SAVESTAT.zst --interpreter
    OAG_RPCS3_ATTACH=1 uv run --with evdev python3 scripts/rpcs3-hd-own-box.py capture \\
        --log-dir <dir> --weapon plasma --fires 5
    python3 scripts/rpcs3-hd-own-box.py replay <dir>/*.json

`--weapon missile --lift 40` raises each new missile that far at its first update, so its probe
finds open air (`docs/ghidra/functions/ps3-hdfury-eu/open-air-arms.md`, "Measured live").

`capture` starts the race (if it is still on `START RACE`), leaves the craft parked, gives it a weapon,
presses fire and logs every call of `Collision_SweepSegment` (`0x0007be58`) whose segment starts near
the player: the segment, the caller, every box-test call the march makes (`0x00036cac`) with the
collider's matrix, dimensions and surface word, the shared hit record around that call, and the value
returned. Needs a `serve` started with `--interpreter`: `Z0` breakpoints do not fire under the LLVM
recompiler.

`replay` checks the logged box calls against the model `oag_weapons::projectile::own_box_blocks`
implements (`docs/ghidra/functions/ps3-hdfury-eu/own-craft-box.md`).
"""

import argparse
import importlib.util
import json
import math
import os
import struct
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

#: Entry of the march inside `Collision_SweepSegment`: r3 world, r4 start, r5 end, r8 the flag byte.
AT_MARCH = 0x0007BEA4
#: The box test's call inside `Collision_MarchSegment`, and the instruction after it.
AT_BOX = 0x00036CAC
AFTER_BOX = 0x00036CB0
#: The two returns of `Collision_SweepSegment`: `0x7f`, and a mesh collider's `+0x74`.
RETURNS = (0x0007BEE4, 0x0007BF3C)
INNER = (AT_BOX, AFTER_BOX) + RETURNS
#: A collider's slot in the world: `world + index * 8 + 0xf764`.
COLLIDER_TABLE = 0xF764
#: Segments starting further than this from the player belong to somebody else's weapon.
NEAR = 80.0
STATES = {"rocket": 0, "missile": 1, "plasma": 7}
SLOT_OFFSET = 0x5EDC
SLOT_STATE = 0x204


def load(name, filename):
    spec = importlib.util.spec_from_file_location(name, HERE / filename)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def answered_by_box(box):
    """The box test moved the shared record: kind 3 and a nearer distance."""
    return box["kind_after"] == 3 and box["distance_after"] < box["distance_before"]


class Reader:
    def __init__(self, gdb):
        from rpcs3_debugger import REG_GPR, REG_PC

        self.gdb = gdb
        self.gpr_at = REG_GPR
        self.pc_at = REG_PC
        self.last = None

    def gpr(self, regs, i):
        at = self.gpr_at + 8 * i
        return int.from_bytes(regs[at : at + 8], "big") & 0xFFFFFFFF

    def u32(self, address):
        return struct.unpack(">I", self.gdb.read(address, 4))[0]

    def floats(self, address, count):
        return list(struct.unpack(">%df" % count, self.gdb.read(address, 4 * count)))

    def parked(self, sites, only=None):
        """`(tid, registers, pc)` of a thread parked at one of `sites`, or `None`."""
        order = [only] if only else ([self.last] if self.last else []) + self.gdb.threads()
        for tid in order:
            regs = self.gdb.registers(tid)
            if regs is None:
                continue
            pc = int.from_bytes(regs[self.pc_at : self.pc_at + 8], "big")
            if pc in sites:
                self.last = tid
                return tid, regs, pc
        return None

    def run_to(self, sites, seconds, only=None):
        """Resume until a thread parks at one of `sites`, the target left stopped."""
        end = time.time() + seconds
        while time.time() < end:
            self.gdb.resume()
            if self.gdb.wait_for_stop(0.25) is None:
                self.gdb.pause()
            self.gdb.drain(0.05)
            found = self.parked(sites, only)
            if found:
                return found
        return None

    def step_over(self, tid, address):
        self.gdb.remove_breakpoint(address)
        self.gdb.step(tid)
        self.gdb.drain(0.05)
        self.gdb.add_breakpoint(address)

    def collider(self, world, index):
        obj = self.u32(world + index * 8 + COLLIDER_TABLE)
        blob = self.gdb.read(obj, 0x1A0)
        return {
            "address": obj,
            "kind": struct.unpack_from(">I", blob, 0x0C)[0],
            "surface": struct.unpack_from(">I", blob, 0x74)[0],
            "word_84": struct.unpack_from(">I", blob, 0x84)[0],
            "rows": [list(struct.unpack_from(">3f", blob, 0x30 + 16 * i)) for i in range(3)],
            "centre": list(struct.unpack_from(">3f", blob, 0x60)),
            "dimensions": list(struct.unpack_from(">3f", blob, 0x190)),
        }


#: `Rocket_Update`'s probe and travel sweep sites: r25 holds the rocket at both, so its words
#: (position `+0xd0`, velocity `+0xe0`) are kept at each call.
ROCKET_RETURNS = (0x001249E0, 0x00124448)
ROCKET_BYTES = 0x180
#: `Missile_Update`'s probe and travel sweep sites: r28 holds the missile (position `+0xa0`,
#: velocity `+0xb0`, previous position `+0xc0`, carried normal `+0xd0`).
MISSILE_RETURNS = (0x0011D3A8, 0x0011D4C0)
#: An instruction in `Missile_Update`'s prologue, r3 still the missile: where `--lift` moves it
#: into open air.
AT_MISSILE_UPDATE = 0x0011D29C


def one_call(read, tid, regs, pose):
    """From the march's entry in `Collision_SweepSegment` to its return, with every box call."""
    gdb = read.gdb
    frame = read.gpr(regs, 1)
    call = {
        "caller": int.from_bytes(gdb.read(frame + 256, 8), "big") & 0xFFFFFFFF,
        "start": read.floats(read.gpr(regs, 4), 3),
        "end": read.floats(read.gpr(regs, 5), 3),
        "flag_byte": read.gpr(regs, 8) & 0xFF,
        "skip_collider": read.gpr(regs, 7),
        "body": pose,
        "boxes": [],
    }
    for sites, register in ((ROCKET_RETURNS, 25), (MISSILE_RETURNS, 28)):
        if call["caller"] in sites:
            held = read.gpr(regs, register)
            call["rocket"] = held
            call["object"] = gdb.read(held, ROCKET_BYTES).hex()
    for address in INNER:
        gdb.add_breakpoint(address)
    read.step_over(tid, AT_MARCH)
    pending = None
    while True:
        found = read.run_to(INNER, 20.0, only=tid)
        if found is None:
            raise RuntimeError("the call from %#x never returned" % call["caller"])
        _, regs, pc = found
        if pc == AT_BOX:
            march = read.gpr(regs, 1)
            index = read.gpr(regs, 8) & 0xFFFF
            pending = {
                "index": index,
                "march": march,
                "kind_before": read.u32(march + 424),
                "distance_before": read.floats(march + 352, 1)[0],
                "collider": read.collider(read.gpr(regs, 3), index),
            }
        elif pc == AFTER_BOX and pending:
            march = pending.pop("march")
            pending["kind_after"] = read.u32(march + 424)
            pending["distance_after"] = read.floats(march + 352, 1)[0]
            pending["index_after"] = read.u32(march + 416)
            call["boxes"].append(pending)
            pending = None
        elif pc in RETURNS:
            call["returned"] = read.gpr(regs, 3)
            # The caller's hit record is at +112 of the frame: collider index +32, kind +40.
            march_frame = read.gpr(regs, 1) - 240
            call["hit_index"] = read.u32(march_frame + 144)
            call["hit_kind"] = read.u32(march_frame + 152)
            break
        read.step_over(tid, pc)
    for address in INNER:
        gdb.remove_breakpoint(address)
    return call


def capture(args):
    drive = load("rpcs3_drive", "rpcs3-drive.py")
    import rpcs3_place as place
    from rpcs3_debugger import Debugger

    out = Path(args.log_dir)
    out.mkdir(parents=True, exist_ok=True)
    port = int(drive.GDB_SERVER.rsplit(":", 1)[1])
    clock = 0x7014
    with drive.Session(os.environ.get("OAG_IMAGE", "x"), str(out / "logs"), interpreter=True) as session:
        with Debugger(port=port) as gdb:
            read = Reader(gdb)
            for _ in range(80):
                gdb.pause()
                ship, body = place.find_player(gdb)
                now = read.floats(ship + clock, 1)[0]
                gdb.resume()
                print("race clock %.2f" % now, flush=True)
                if now > args.after:
                    break
                if now < 0.01:
                    session.tap("cross", settle=6)
                else:
                    time.sleep(3)
            else:
                sys.exit("the race never reached %.1f s" % args.after)
            gdb.pause()
            slot = read.u32(ship + SLOT_OFFSET)
            fires = []
            placed = None
            if args.place:
                placed = json.load(open(args.place))[0]["pose_at_press"]
            for number in range(args.fires):
                gdb.pause()
                if placed:
                    place.write_pose(gdb, body, placed["pos"], [tuple(r) for r in placed["rows"]], 0.0)
                    gdb.resume()
                    time.sleep(args.settle)
                    gdb.pause()
                pose = place.read_pose(gdb, body)
                record = {"weapon": args.weapon, "race_clock_s": read.floats(ship + clock, 1)[0], "pose_at_press": pose, "calls": []}
                gdb.write(slot + SLOT_STATE, struct.pack(">ii", STATES[args.weapon], STATES[args.weapon]))
                gdb.add_breakpoint(AT_MARCH)
                session.pad.set("triangle", True)
                held = time.time()
                started = time.time()
                last_near = None
                lifted = set()
                if args.lift:
                    gdb.add_breakpoint(AT_MISSILE_UPDATE)
                while time.time() - started < args.timeout:
                    if held and time.time() - held > args.hold:
                        session.pad.set("triangle", False)
                        held = 0
                    found = read.run_to((AT_MARCH,) + ((AT_MISSILE_UPDATE,) if args.lift else ()), 2.0)
                    if found is not None and found[2] == AT_MISSILE_UPDATE:
                        tid, regs, _ = found
                        missile = read.gpr(regs, 3)
                        if missile not in lifted:
                            lifted.add(missile)
                            x, y, z = read.floats(missile + 0xA0, 3)
                            gdb.write(missile + 0xA0, struct.pack(">3f", x, y + args.lift, z))
                            record.setdefault("lifted", []).append({"missile": missile, "from": [x, y, z], "by": args.lift})
                            print("fire %d: lifted missile %#x from y %.2f by %.1f" % (number, missile, y, args.lift), flush=True)
                        read.step_over(tid, AT_MISSILE_UPDATE)
                        continue
                    if found is None:
                        if last_near and time.time() - last_near > args.quiet:
                            break
                        continue
                    tid, regs, _ = found
                    here = place.read_pose(gdb, body)
                    start = read.floats(read.gpr(regs, 4), 3)
                    if math.dist(start, here["pos"]) > NEAR:
                        read.step_over(tid, AT_MARCH)
                        if last_near and time.time() - last_near > args.quiet:
                            break
                        continue
                    call = one_call(read, tid, regs, here)
                    call["clock_s"] = read.floats(ship + clock, 1)[0]
                    record["calls"].append(call)
                    # Written per call: a stub that stops answering must not lose the fire.
                    (out / ("%s.json" % args.weapon)).write_text(json.dumps(fires + [record], indent=1))
                    last_near = time.time()
                    boxed = [b for b in call["boxes"] if answered_by_box(b)]
                    print("fire %d call %2d from %#x len %.2f ret %#x kind %d boxes %d (%d answered) dist %.2f" % (
                        number, len(record["calls"]), call["caller"], math.dist(call["start"], call["end"]),
                        call["returned"], call["hit_kind"], len(call["boxes"]), len(boxed),
                        math.dist(call["start"], here["pos"])), flush=True)
                    if len(record["calls"]) >= args.calls:
                        break
                session.pad.set("triangle", False)
                gdb.pause()
                gdb.remove_breakpoint(AT_MARCH)
                if args.lift:
                    gdb.remove_breakpoint(AT_MISSILE_UPDATE)
                fires.append(record)
                (out / ("%s.json" % args.weapon)).write_text(json.dumps(fires, indent=1))
                print("fire %d: %d calls, craft speed %.2f" % (number, len(record["calls"]), math.hypot(*record["pose_at_press"]["vel"])), flush=True)
                gdb.resume()
                time.sleep(args.between)
            gdb.pause()
            gdb.resume()


def replay(args):
    model = load("box_replay", "psp-craft-box-replay.py")
    total = wrong = answered = 0
    for path in args.captures:
        for number, fire in enumerate(json.load(open(path))):
            for n, call in enumerate(fire["calls"]):
                for box in call["boxes"]:
                    collider = box["collider"]
                    if math.dist(collider["centre"], call["body"]["pos"]) > 2.0:
                        continue
                    rows = collider["rows"]
                    centre = collider["centre"]
                    extent = [v * 0.5 for v in collider["dimensions"]]
                    columns = [[rows[j][i] for j in range(3)] for i in range(3)]
                    frame = columns if args.frame == "cols" else rows
                    local = lambda p: [model.dot(a, model.sub(p, centre)) for a in frame]
                    start, end = call["start"], call["end"]
                    gate = model.separating_axis_passes(local(start), local(end), extent)
                    face = model.face_crossing(rows, centre, extent, start, end) if gate else None
                    predicted = face is not None and face * math.dist(start, end) < box["distance_before"]
                    observed = answered_by_box(box)
                    total += 1
                    wrong += predicted != observed
                    answered += observed
                    if args.v or predicted != observed:
                        print("%s fire %d call %d collider %d: observed %s predicted %s (face %s, before %.3f)%s" % (
                            path, number, n + 1, box["index"], observed, predicted, face,
                            box["distance_before"], "" if predicted == observed else "  <-- MISS"))
    print("%d own-box calls, %d answered kind 3, %d mispredicted with the %s frame" % (total, answered, wrong, args.frame))
    return 1 if wrong else 0


def fall(args):
    """The Missile's velocity at its sweep against four laws for the probe that found nothing
    (`docs/ghidra/functions/ps3-hdfury-eu/open-air-arms.md`, "Measured live")."""
    vector = lambda call, at: list(struct.unpack_from(">3f", bytes.fromhex(call["object"]), at))
    errors = {}
    for path in args.captures:
        for fire in json.load(open(path)):
            for probe, sweep in zip(fire["calls"], fire["calls"][1:]):
                if (probe["caller"], sweep["caller"]) != MISSILE_RETURNS or probe["returned"] != 0x7F:
                    continue
                before, after = vector(probe, 0xB0), vector(sweep, 0xB0)
                normal = vector(probe, 0xD0)
                speed = math.hypot(*before)
                dt = math.dist(sweep["start"], sweep["end"]) / speed
                into = sum(v * n for v, n in zip(before, normal))
                along = [v - into * n for v, n in zip(before, normal)]
                laws = {
                    "fall on world Y": [before[0], before[1] - 50.0 * dt, before[2]],
                    "fall along the normal": [v - 50.0 * dt * n for v, n in zip(before, normal)],
                    "unchanged": before,
                    "parallel turn": [v / math.hypot(*along) * speed for v in along],
                }
                for name, predicted in laws.items():
                    errors.setdefault(name, []).append(math.dist(after, predicted))
    for name, found in errors.items():
        print("%-22s %d updates, error %.6f to %.6f" % (name, len(found), min(found), max(found)))
    return 0 if errors else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    commands = parser.add_subparsers(dest="command", required=True)
    c = commands.add_parser("capture")
    c.add_argument("--log-dir", required=True)
    c.add_argument("--weapon", choices=sorted(STATES), required=True)
    c.add_argument("--fires", type=int, default=5)
    c.add_argument("--calls", type=int, default=10, help="calls logged per fire")
    c.add_argument("--after", type=float, default=3.0, help="race clock seconds before the first fire")
    c.add_argument("--hold", type=float, default=2.0, help="wall seconds triangle is held")
    c.add_argument("--timeout", type=float, default=240.0, help="wall seconds per fire")
    c.add_argument("--quiet", type=float, default=15.0, help="wall seconds without a near call that end a fire")
    c.add_argument("--between", type=float, default=8.0)
    c.add_argument("--place", help="a capture whose first fire's pose the craft is put back to before each fire")
    c.add_argument("--lift", type=float, default=0.0, help="raise each new missile this many units before its first update")
    c.add_argument("--settle", type=float, default=4.0, help="wall seconds the placed craft runs before the fire")
    r = commands.add_parser("replay")
    r.add_argument("captures", nargs="+")
    r.add_argument("--frame", choices=("cols", "rows"), default="cols")
    r.add_argument("-v", action="store_true")
    f = commands.add_parser("fall")
    f.add_argument("captures", nargs="+")
    args = parser.parse_args()
    if args.command == "capture":
        capture(args)
        return 0
    if args.command == "fall":
        return fall(args)
    return replay(args)


if __name__ == "__main__":
    sys.exit(main())
