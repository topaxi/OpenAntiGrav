#!/usr/bin/env python3
"""Fire a Rocket (1) or Plasma (8) from Pure's player craft and log every update of the projectile.

Pure USA (UCUS98612 2.00) on PPSSPP. Load a Time Trial restart state from data/saves/pure-psp/
(psp-state.py --serial UCUS98612_2.00), then run this: it holds cross, gives the weapon at
Ship_UpdateWeapons hit --skip (psp-pure-weapon-fire.py's method), and breaks at each
Rocket_Update (0x0885f28c) or Plasma_Update (0x0885e348) entry reading the entity (a0): position,
velocity, carried normal and age. See docs/ghidra/functions/psp-pure-usa/floor-follow.md.

    uv run --with websocket-client python3 scripts/psp-pure-projectile-log.py 1 data/scratch/<lane>/r.json \
        --port 45137 --skip 30 --updates 300

Raw captures are derived game data: write them under data/, never commit them.
"""
import argparse
import json
import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
from ppsspp_debugger import Debugger
SHIP_UPDATE_WEAPONS = 0x0892935c
W = {1: dict(update=0x0885F28C, pos=0xA0, vel=0xB0, normal=0xD0, age=0x50, flags=0x40),
     8: dict(update=0x0885E348, pos=0xA0, vel=0xF0, normal=0x110, age=0x58, flags=0x40)}
ap = argparse.ArgumentParser()
ap.add_argument("weapon", type=int); ap.add_argument("out")
ap.add_argument("--skip", type=int, default=205); ap.add_argument("--updates", type=int, default=400)
ap.add_argument("--port", type=int, default=45137)
a = ap.parse_args(); w = W[a.weapon]
d = Debugger(a.port)
def gpr():
    r = d.call("cpu.getAllRegs"); c = next(c for c in r["categories"] if c["name"] == "GPR")
    return dict(zip(c["registerNames"], c["uintValues"]))
d.hold(cross=True)
craft = None
for j, _ in d.each_hit(SHIP_UPDATE_WEAPONS, a.skip + 1, timeout=60):
    if j == a.skip:
        ship = gpr()["a0"]
        holder = d.read_u32(ship + 0xd0); rec = d.read_u32(d.read_u32(ship + 200) + 0x48)
        d.write_u32(holder + 0x1d8, a.weapon); d.write(rec + 0x15, bytes([1]))
        craft = dict(ship=ship)  # noqa: F841
d.remove_breakpoint(SHIP_UPDATE_WEAPONS)
rows = []
try:
    for j, _ in d.each_hit(w["update"], a.updates, timeout=20):
        e = gpr()["a0"]
        rows.append(dict(entity=e, age=d.read_f32(e + w["age"]), flags=d.read_u32(e + w["flags"]),
                         pos=d.read_f32s(e + w["pos"], 3), vel=d.read_f32s(e + w["vel"], 3),
                         normal=d.read_f32s(e + w["normal"], 3)))
except TimeoutError:
    pass
d.hold(cross=False); d.resume()
json.dump(dict(craft=craft, rows=rows), open(a.out, "w"))
print(len(rows), "updates;", len({r["entity"] for r in rows}), "entities")
