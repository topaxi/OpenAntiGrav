#!/usr/bin/env python3
"""Fire an opponent's Quake at the player in a live PPSSPP race and log what the hit does, per frame.

    python3 scripts/psp-quake-hit.py --port 45300 --out hit.pkl [--frames 0,1,2,4,8] [--cockpit]
    ... --no-fire            the control run: the same load, nothing fired
    ... --no-autopilot       hand the craft back to the pad (see trap 3)
    ... --shooter N          a fixed grid index instead of "an AI 70-420 units behind the player"

One row per **player** frame (a conditional breakpoint on the hull-matrix submit,
`0x08842264`, `s2 == <the player's entity>`, so the seven AI stops cost nothing), each holding
raw copies of the structures a hit touches: the entity (`0x900` bytes), the craft, the rigid body,
the HUD object and its two widget groups, the screen-flash node, the cockpit-hit node, the camera
and `g_camera_fov_degrees`. Decode offline, so a new offset needs no new capture. `--frames` also
reads both EDRAM framebuffers (software renderer only) on those rows after the hit's latch edge,
`--pre` on those absolute rows before it. `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`,
"2026-10-26: why a Quake hit feels stronger on the original", is what it was written for.

Four traps, each of which cost a run:

1. **A save state must be caught before it loads.** Connect the moment the debugger answers and
   stop the CPU (`connect_frozen`); the first hull-site stop after the state loads is then the same
   on every boot (three boots, identical positions to three decimals). A late attach lands seconds
   into a free-running race.
2. **The weapon record's direction vector must be written.** `Weapon_FireQuake` hands
   `record+0x40` (four floats) to `Quake_Init`, whose sign decides which way the wave travels. An
   AI that has never fired holds zeros there, so the wave runs *backwards* from it. This writes the
   shooter's own forward vector first. `record+0xf0` (the entity) is 0 in these states too, and
   crashes `Quake_Init` unless set.
3. **`race-talons-venom-alongside` has the Autopilot running** (`record+0x1b8 & 0x800`, timer
   `1e9`), so the player's steering, airbrakes and speed are the Autopilot's. It is not a physics
   reference (maintainer, 2026-10-20). `--no-autopilot` clears the bit and the timer; the craft
   then flies straight on the stick it is given and meets a wall within about two seconds.
   The state is also **Weapons OFF, skill 0**: `Ship_Damage` halves every amount (a Quake costs 7.5,
   against 15), and the pool is 135, not the 95 of skill 1 and 2.
4. **`input.buttons.press` does not answer while the CPU is stopped.** Use `hold(...)` and release
   it a few frames later. `--cockpit` does this to cycle the camera onto the internal view
   (CLOSE, FAR, INT); `entity+0x6d` reads 1 once it is there.
"""

import argparse
import pickle
import struct
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ppsspp_debugger import Debugger, DebuggerError  # noqa: E402

DISPATCH = 0x08861814  # Weapons_DispatchFire
SITE = 0x08842264  # the hull-matrix submit in FUN_088418e0, once per craft per frame
G_HUD, G_CAM, G_FLASH, G_COCKPIT, G_FOV = 0x08AB0838, 0x08AB10B0, 0x08AB2200, 0x08AB2120, 0x08B34310
FIRE_QUAKE = 0x8


def connect_frozen(port):
    """Connect as soon as the debugger is up and stop the CPU before the save state has loaded."""
    while True:
        try:
            dbg = Debugger(port=port, connect_timeout=1)
            break
        except Exception:
            time.sleep(0.05)
    while True:
        try:
            dbg.call("cpu.stepping")
            return dbg
        except DebuggerError:
            time.sleep(0.02)


def gpr(dbg):
    registers = dbg.call("cpu.getAllRegs")
    category = next(c for c in registers["categories"] if c["name"] == "GPR")
    return dict(zip(category["registerNames"], category["uintValues"]))


def in_ram(pointer):
    return 0x08000000 <= pointer < 0x0A000000


def blob(dbg, pointer, size):
    return dbg.read(pointer, size) if in_ram(pointer) else None


def learn_field(dbg):
    """One frame of hull-site stops: entity, weapon record, controller class, position and forward per grid index."""
    entities, geometry = {}, {}
    for _, _ in dbg.each_hit(SITE, 40, timeout=30.0):
        regs = gpr(dbg)
        entity, matrix = regs["s2"], regs["s0"]
        index = dbg.read_u32(entity + 0x360)
        entities[index] = (entity, dbg.read_u32(entity + 0x4C), dbg.read_u32(entity + 0x368))
        geometry[index] = (dbg.read_f32s(matrix + 0x30, 3), dbg.read_f32s(matrix + 0x20, 3))
        if len(entities) == 8:
            break
    return entities, geometry


def pick_shooter(geometry, player):
    """An AI behind the player (70 to 420 units along the player's forward) heading the same way."""
    candidates = []
    for index, (position, forward) in geometry.items():
        if index == player:
            continue
        offset = [a - b for a, b in zip(position, geometry[player][0])]
        along = sum(x * y for x, y in zip(offset, geometry[player][1]))
        heading = sum(x * y for x, y in zip(forward, geometry[player][1]))
        if heading > 0.5 and -420 < along < -70:
            candidates.append((along, index))
    if not candidates:
        raise SystemExit("no AI craft behind the player; pass --shooter")
    return sorted(candidates)[len(candidates) // 2][1]


def fire_quake(dbg, record, entity, forward):
    for _, _ in dbg.each_hit(DISPATCH, 1, timeout=40.0):
        if dbg.read_u32(record + 0xF0) == 0:
            dbg.write_u32(record + 0xF0, entity)
        dbg.write_f32s(record + 0x40, [forward[0], forward[1], forward[2], 0.0])
        dbg.write_u32(record + 0x1B8, dbg.read_u32(record + 0x1B8) | FIRE_QUAKE)


def cycle_to_cockpit(dbg, player_entity):
    waits = [8, 30]
    for frame, _ in dbg.each_hit(SITE, 80 * 8, timeout=20.0, condition="s2 == 0x%x" % player_entity):
        if waits and frame >= waits[0]:
            waits.pop(0)
            dbg.hold(cross=True, select=True)
        if frame in (14, 36):
            dbg.hold(cross=True, select=False)
        if frame >= 70:
            break
    print("internal view flag (entity+0x6d):", dbg.read_u8(player_entity + 0x6D), file=sys.stderr)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--port", type=int, required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--rows", type=int, default=300, help="player frames to log")
    parser.add_argument("--shooter", type=int, help="grid index to fire from (default: picked)")
    parser.add_argument("--no-fire", action="store_true")
    parser.add_argument("--no-autopilot", action="store_true")
    parser.add_argument("--cockpit", action="store_true")
    parser.add_argument("--frames", default="", help="rows after the latch edge that also dump EDRAM")
    parser.add_argument("--pre", default="", help="absolute rows before the edge that also dump EDRAM")
    args = parser.parse_args()
    after = {int(x) for x in args.frames.split(",") if x}
    before = {int(x) for x in args.pre.split(",") if x}

    with connect_frozen(args.port) as dbg:
        dbg.hold(cross=True)
        entities, geometry = learn_field(dbg)
        player = next(i for i, v in entities.items() if v[2] == 0)
        player_entity = entities[player][0]
        shooter = args.shooter if args.shooter is not None else pick_shooter(geometry, player)
        print("player", player, "shooter", shooter, file=sys.stderr)
        if args.no_autopilot:
            record = entities[player][1]
            dbg.write_u32(record + 0x1B8, dbg.read_u32(record + 0x1B8) & ~0x800)
            dbg.write_f32s(record + 0x148, [0.0])
        if args.cockpit:
            cycle_to_cockpit(dbg, player_entity)
        if not args.no_fire:
            fire_quake(dbg, entities[shooter][1], entities[shooter][0], geometry[shooter][1])

        rows, latch_row, started = [], None, time.time()
        for index, _ in dbg.each_hit(SITE, args.rows * 8, timeout=20.0, condition="s2 == 0x%x" % player_entity):
            regs = gpr(dbg)
            craft = dbg.read_u32(player_entity + 0x94)
            hud = dbg.read_u32(G_HUD)
            row = {
                "i": index,
                "wall": time.time() - started,
                "E": dbg.read(player_entity, 0x900),
                "C": blob(dbg, craft, 0x300),
                "B": blob(dbg, dbg.read_u32(player_entity + 0x794), 0x3A0),
                "M": dbg.read(regs["s0"], 0x40),
                "H": blob(dbg, hud, 0x300),
                "FL": blob(dbg, dbg.read_u32(G_FLASH), 0x1B0),
                "CK": blob(dbg, dbg.read_u32(G_COCKPIT), 0x68),
                "CAM": blob(dbg, dbg.read_u32(G_CAM), 0x130),
                "FOV": dbg.read(G_FOV, 12),
            }
            if in_ram(hud):
                row["G1"] = blob(dbg, dbg.read_u32(hud + 0x274), 0x100)
                row["G2"] = blob(dbg, dbg.read_u32(hud + 0x278), 0x100)
            latched = (struct.unpack_from("<I", row["E"], 0x860)[0] >> 6) & 1
            if latched and latch_row is None:
                latch_row = len(rows)
            relative = None if latch_row is None else len(rows) - latch_row
            if (relative is None and len(rows) in before) or (relative is not None and relative in after):
                row["fb"] = [dbg.read(0x04000000, 0x88000), dbg.read(0x04088000, 0x88000)]
            row["hs"] = [dbg.read_f32(entities[k][0] + 0x870) for k in sorted(entities)]
            rows.append(row)
            if len(rows) >= args.rows:
                break
        pickle.dump(
            {"ents": entities, "geo": geometry, "player": player_entity, "shooter": shooter, "rows": rows},
            open(args.out, "wb"),
        )
        print("rows", len(rows), "latch row", latch_row, file=sys.stderr)


if __name__ == "__main__":
    main()
