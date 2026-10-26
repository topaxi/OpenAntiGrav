#!/usr/bin/env python3
"""Read the collision sparks the original threw just before a race state's frame, as `--pose-spark` flags for ours.

    OAG_PPSSPP_PORT=45092 source <lane>/pp/env.sh
    uv run --with websocket-client python3 scripts/psp-contact-fx-read.py --port 45092 --home $PP_HOME \\
        --state data/saves/pulse-psp/race-talons-venom-alongside.ppst --target-clock 39.3429 \\
        --field data/reference/compare-page/pulse-psp-race/talons-boost/field.csv --out <scratch>/sparks.json

The state is loaded and run on as `psp-pulse-pair.py --field --target-clock` does (the same functions), to `--window`
seconds before the target, and every call of `ShipCollisionFx_Trigger` (`0x089246b4`: `a0` the instance, `a1` the kind,
`a2` the damaged flag, `f12` the clamped intensity `min(|p| * 0.0125, 1)`) up to the target is logged. A call whose
instance gate (`instance + 100`, a world-clock time) has opened against `*(g_ingame + 0x40)` spawns a spark
(`ShipCollisionFx_Trigger`'s own test, contact-response.md), so those calls are the sparks the frame holds. Each is
mapped from the original's grid slot (the order of `g_race_manager + 0x78`) to ours' `--field-from` row by team name
(a pair's `field.csv` lists the player first, so the two orders differ), and written with its age in seconds (the HUD
clock between the call and the target) and its severity (`intensity * 2 + 0.4`). The CPU is left stopped inside the
last call; nothing is written to the emulator. Raw captures are derived game data: keep the output under `data/`.
"""

import argparse
import csv
import importlib.util
import json
import struct
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

_spec = importlib.util.spec_from_file_location("psp_pulse_pair", HERE / "psp-pulse-pair.py")
pair = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(pair)

#: `ShipCollisionFx_Trigger`.
FX_TRIGGER = 0x089246B4
G_INGAME = 0x08AB0818
CLOCK_OFFSET = 0x40
GRID_SLOTS = 0x78
FX_LIST = 0xC80
INSTANCE_GATE = 100
#: The kind-0/1 cooldown `ShipCollisionFx_Trigger` stamps `now + 0.8` after a spawn.
INTENSITY_SLOPE = 2.0
INTENSITY_FLOOR = 0.4


def gpr(dbg):
    return dict(zip(*[next(c for c in dbg.call("cpu.getAllRegs")["categories"] if c["name"] == "GPR")[k]
                      for k in ("registerNames", "uintValues")]))


def f12(dbg):
    fpu = next(c for c in dbg.call("cpu.getAllRegs")["categories"] if c["name"] == "FPU")
    return struct.unpack("<f", struct.pack("<I", dict(zip(fpu["registerNames"], fpu["uintValues"]))["f12"]))[0]


def instance_table(dbg):
    """`{instance address: (grid slot, node index)}` and `{grid slot: team name}` for the field."""
    rf = pair.race_field_module()
    field = rf.Field(dbg)
    table, teams = {}, {}
    for s in field.slots:
        teams[s["slot"]] = field.team_name(s)
        for k in range(10):
            pointer = dbg.read_u32(s["entity"] + FX_LIST + 4 * k)
            if rf.ram(pointer):
                table[pointer] = (s["slot"], k)
    return table, teams


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--port", type=int, required=True)
    ap.add_argument("--home", required=True)
    ap.add_argument("--state", required=True)
    ap.add_argument("--target-clock", type=float, required=True)
    ap.add_argument("--field", type=Path, required=True, help="the pair's field.csv (slot, team, ...): ours' slot order")
    ap.add_argument("--window", type=float, default=0.6, help="seconds before the target to start logging")
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--toast-wait", type=float, default=6.0)
    args = ap.parse_args()
    uv = ["uv", "run", "--with", "websocket-client", "python3"]
    pair.set_window(960, 544)
    pair.resume(args.port)
    subprocess.run([str(c) for c in uv + [HERE / "psp-state.py", "--port", args.port, "--home", args.home, "load", "2",
                                          "--file", args.state, "--stop-at", hex(pair.SHIP_UPDATE_CRAFT)]], check=True)
    pair.freeze(args.port)
    time.sleep(args.toast_wait)
    pair.run_to_clock(args.port, args.target_clock - args.window)
    from ppsspp_debugger import Debugger
    dbg = Debugger(args.port)
    table, teams = instance_table(dbg)
    ours_row = {row["team"]: int(row["slot"]) for row in csv.DictReader(open(args.field))}
    calls = []
    for _ in dbg.each_hit(FX_TRIGGER, 100000, timeout=30.0):
        clock = dbg.read_f32(pair.CLOCK_ADDRESS)
        if clock >= args.target_clock - 0.0005:
            break
        r = gpr(dbg)
        now = dbg.read_f32(dbg.read_u32(G_INGAME) + CLOCK_OFFSET)
        gate = dbg.read_f32(r["a0"] + INSTANCE_GATE)
        intensity = f12(dbg)
        kind01 = r["a1"] in (0, 1)
        grid, node = table.get(r["a0"], (None, None))
        calls.append({"hud_clock": round(clock, 4), "grid_slot": grid, "node": node, "kind": r["a1"], "damaged": r["a2"],
                      "intensity": intensity, "gate": gate, "now": now, "spawns": (not kind01) or gate <= now})
    sparks = []
    for c in calls:
        if not c["spawns"] or c["kind"] != 0 or not c["damaged"] or c["grid_slot"] is None:
            continue
        team = teams[c["grid_slot"]]
        age = args.target_clock - c["hud_clock"]
        severity = c["intensity"] * INTENSITY_SLOPE + INTENSITY_FLOOR
        sparks.append({"ours_slot": ours_row.get(team), "team": team, "grid_slot": c["grid_slot"], "node": c["node"],
                       "age_s": round(age, 4), "severity": round(severity, 4),
                       "flag": "%s:%d:%.4f:%.4f" % (ours_row.get(team), c["node"], age, severity)})
    args.out.write_text(json.dumps({"target_clock": args.target_clock, "window": args.window, "calls": calls,
                                    "sparks": sparks, "flags": [x for s in sparks for x in ("--pose-spark", s["flag"])]},
                                   indent=1))
    print("%d trigger call(s) in the window, %d spark spawn(s):" % (len(calls), len(sparks)))
    for s in sparks:
        print("  %s (grid slot %s) node %d, %.3f s before the frame, severity %.3f -> --pose-spark %s"
              % (s["team"], s["grid_slot"], s["node"], s["age_s"], s["severity"], s["flag"]))


if __name__ == "__main__":
    main()
