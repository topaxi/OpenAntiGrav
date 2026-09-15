#!/usr/bin/env python3
"""Count how many mines one press lays, on the original running in PPSSPP.

`Weapon_DropMines` (`0x088675cc`) lays one mine every `0.1 s` and decrements a
per-craft round counter at `craft+0x1ac` until it reaches zero, at which point
it clears its own fire bit. Two static sweeps found nothing else that writes
that counter, so the number a press starts it at - `oag_gameplay::projectile::
mine::CLUSTER` - was invented. This measures it instead, and records who
writes the counter while it is at it.

Two instruments, armed together:

1. **An execution breakpoint on `Weapon_DropMines`.** The dispatcher calls it
   once per frame per craft whose bit `0x2` is set, so its argument registers
   are a per-frame trail of `(subsystem, craft record, craft index)` for every
   cluster in flight. At each hit this reads the record's `+0x1ac` (rounds),
   `+0x1b0` (reload), `+0x1b8` (fire word), `+0x1bc` (held weapon) and the
   pool's `+0x164` (live count), plus the PSP cycle counter for a frame clock.
   The first hit of a cluster reads the counter at press time - that is the
   answer - and the trail from there is the spacing.
2. **A non-halting write watchpoint on every craft record's `+0x1ac`**
   (`enabled: False, log: True`), which makes PPSSPP's own stdout carry
   `CHK Write32(CPU) at <addr>, PC=<pc>` for every store, including the one
   that arms the counter before the first drop. That PC is the grant path no
   static sweep has found. A positive control (the player's rigid body, written
   every physics step) is armed alongside so a silent log is legibly "nothing
   happened" rather than "the instrument is dead".

The craft record here is **not** `Ship_UpdateCraft`'s craft: `Weapons_DispatchFire`
walks an inline array at `world + 0x70 + index * 0x1f0` (see
`psp-fire-weapon.py`). The world is learned from one `Weapons_DispatchFire`
hit and never guessed.

Nothing here writes game memory: the mines that get counted are the ones the
AI (or a driving player) actually collects and fires, through the game's own
grant path. Setting bit `0x2` by hand would skip the write that initialises
the counter and count down from garbage.

    uv run --with websocket-client scripts/psp-count-mines.py --port 47810 \\
        --minutes 8 --out ~/.cache/oag/ppsspp-mines/drops.tsv

Then read the writer PCs out of the emulator's stdout:

    grep 'CHK Write' ~/.cache/oag/ppsspp-mines/ppsspp.log
"""

import argparse
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ppsspp_debugger import Debugger

SHIP_UPDATE_CRAFT = 0x08849618
BODY_POINTER = 0x1CC
WEAPONS_DISPATCH_FIRE = 0x08861814
WEAPON_DROP_MINES = 0x088675CC

CRAFT_ARRAY = 0x70
CRAFT_STRIDE = 0x1F0
CRAFT_COUNT = 8

ROUNDS = 0x1AC
RELOAD = 0x1B0
FIRE_WORD = 0x1B8
HELD = 0x1BC
POOL_LIVE = 0x164

MINE_BIT = 0x2
PSP_TICKS_PER_FRAME = 222e6 / 59.94


def gpr(dbg):
    registers = dbg.call("cpu.getAllRegs")
    category = next(c for c in registers["categories"] if c["name"] == "GPR")
    return dict(zip(category["registerNames"], category["uintValues"]))


def frame(dbg):
    return dbg.call("cpu.status")["ticks"] / PSP_TICKS_PER_FRAME


def in_ram(address):
    return 0x08000000 <= address < 0x0A000000


def learn(dbg, address, register):
    """One hit of `address`, the named register's value, breakpoint removed."""
    value = None
    for _, _ in dbg.each_hit(address, 1, timeout=60.0):
        value = gpr(dbg)[register]
    return value


def arm_watch(dbg, address, size, log):
    dbg.call(
        "memory.breakpoint.add",
        address=address,
        size=size,
        enabled=False,
        log=log,
        read=False,
        write=True,
        change=False,
    )


def watch_hits(dbg):
    reply = dbg.call("memory.breakpoint.list")
    return {b["address"]: b.get("hits", 0) for b in reply.get("breakpoints", [])}


def main():
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument("--port", type=int, default=47810)
    parser.add_argument("--minutes", type=float, default=8.0, help="how long to watch")
    parser.add_argument("--out", required=True, help="TSV of every Weapon_DropMines hit")
    parser.add_argument(
        "--hold-cross", action="store_true", help="hold accelerate on the player"
    )
    args = parser.parse_args()

    dbg = Debugger(args.port)
    dbg.resume()

    craft = learn(dbg, SHIP_UPDATE_CRAFT, "a0")
    body = dbg.read_u32(craft + BODY_POINTER)
    world = learn(dbg, WEAPONS_DISPATCH_FIRE, "a0")
    print("Ship_UpdateCraft craft 0x%08x body 0x%08x; world 0x%08x" % (craft, body, world))
    if not (in_ram(body) and in_ram(world)):
        raise SystemExit("a learned address is outside PSP RAM; not arming anything")

    records = [world + CRAFT_ARRAY + i * CRAFT_STRIDE for i in range(CRAFT_COUNT)]
    for i, record in enumerate(records):
        print(
            "record %d 0x%08x  rounds %d  reload %.3f  fire 0x%08x  held %d"
            % (
                i,
                record,
                dbg.read_u32(record + ROUNDS),
                dbg.read_f32(record + RELOAD),
                dbg.read_u32(record + FIRE_WORD),
                dbg.read_u32(record + HELD) if dbg.read_u32(record + HELD) < 0x80000000
                else dbg.read_u32(record + HELD) - 0x100000000,
            )
        )
        arm_watch(dbg, record + ROUNDS, 4, log=True)
    arm_watch(dbg, body, 16, log=False)
    print("armed 8 write watches on +0x1ac and a control on the body at 0x%08x" % body)

    if args.hold_cross:
        dbg.hold(cross=True)

    out = open(args.out, "w")
    out.write("frame\tindex\trecord\tsubsystem\trounds\treload\tfire\theld\tpool_live\n")
    end = time.time() + args.minutes * 60.0
    hits = 0
    dbg.brk()
    dbg.add_breakpoint(WEAPON_DROP_MINES)
    try:
        while time.time() < end:
            # A timed-out wait leaves the CPU running; `resume` checks first.
            dbg.resume()
            try:
                dbg.wait_for_break(WEAPON_DROP_MINES, timeout=min(30.0, max(1.0, end - time.time())))
            except TimeoutError:
                continue
            regs = gpr(dbg)
            subsystem, record, index = regs["a0"], regs["a1"], regs["a2"]
            row = (
                frame(dbg),
                index,
                record,
                subsystem,
                dbg.read_u32(record + ROUNDS),
                dbg.read_f32(record + RELOAD),
                dbg.read_u32(record + FIRE_WORD),
                dbg.read_u32(record + HELD),
                dbg.read_u32(subsystem + POOL_LIVE),
            )
            out.write("%.1f\t%d\t0x%08x\t0x%08x\t%d\t%.4f\t0x%08x\t0x%08x\t%d\n" % row)
            out.flush()
            hits += 1
            if hits % 10 == 1:
                print(
                    "hit %d: frame %.0f craft %d rounds %d reload %.3f pool %d"
                    % (hits, row[0], index, row[4], row[5], row[8]),
                    file=sys.stderr,
                )
    finally:
        dbg.brk()
        dbg.remove_breakpoint(WEAPON_DROP_MINES)
        if args.hold_cross:
            dbg.hold(cross=False)
        counts = watch_hits(dbg)
        for i, record in enumerate(records):
            print("watch +0x1ac craft %d: %d hits" % (i, counts.get(record + ROUNDS, -1)))
        print("control body: %d hits" % counts.get(body, -1))
        for i, record in enumerate(records):
            dbg.call("memory.breakpoint.remove", address=record + ROUNDS, size=4)
        dbg.call("memory.breakpoint.remove", address=body, size=16)
        dbg.resume()
        out.close()
        dbg.close()
    print("%d Weapon_DropMines hits written to %s" % (hits, args.out))


if __name__ == "__main__":
    main()
