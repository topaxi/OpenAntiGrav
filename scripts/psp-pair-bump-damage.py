#!/usr/bin/env python3
"""Pulse PSP: does an unshielded craft-to-craft bump charge the energy pool, and by how much per contact?

`FUN_088418e0`'s contact loop calls `Ship_Damage` (`0x088439ac`) from `0x088426ac` once per ring record with a positive
friction. This script stages a rear-end hit and logs every such call: the entity (`a0`), the amount (`f12`), the source
(`a1`), the ring record the loop is on (`s0`: kind, impulse vector, friction) and the pool (`entity + 0x88`) at the call
and again at its return. Run it on a state `psp-state.py load --stop-at 0x08849618` has just loaded:

    source <scratch>/pp/env.sh
    uv run --with websocket-client python3 scripts/psp-state.py load 1 --file <state.ppst> --stop-at 0x08849618
    uv run --with websocket-client python3 scripts/psp-pair-bump-damage.py --port $OAG_PPSSPP_PORT \\
        --speed 40 --gap 20 --out <scratch>/run-40.json

`--control` stages nothing and runs the same window. The breakpoints are swapped one at a time: PPSSPP v1.20.4 fires
only the most recently added one. Output is derived game data: keep it under `data/`.
"""

import argparse
import importlib.util
import json
import struct
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from ppsspp_debugger import Debugger  # noqa: E402


def load(name, file):
    spec = importlib.util.spec_from_file_location(name, HERE / file)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


capture = load("psp_pair_capture", "psp-pair-capture.py")
race_field = load("psp_race_field", "psp-race-field.py")

SHIP_DAMAGE = 0x088439AC
#: The instruction after the contact loop's `jal Ship_Damage` (`0x088426ac`) and its delay slot.
LOOP_RETURN = 0x088426B4
CLOCK = 0x08B344B0
G_WEAPONS = 0x08B30FA8
G_DAMAGE = 0x08B30FA9
ENTITY_POOL = 0x88
ENTITY_STATE = 0x8C
ENTITY_SHIELD_LINK = 0x4C
SHIELD_FLAGS = 0x1B8
ENTITY_FLAGS = 0x860


def regs(dbg):
    cats = {c["name"]: c for c in dbg.call("cpu.getAllRegs")["categories"]}
    gpr = dict(zip(cats["GPR"]["registerNames"], cats["GPR"]["uintValues"]))
    fpu = dict(zip(cats["FPU"]["registerNames"], cats["FPU"]["uintValues"]))
    gpr["f12"] = struct.unpack("<f", struct.pack("<I", fpu["f12"]))[0]
    return gpr


def entity_view(dbg, s):
    e = s["entity"]
    link = dbg.read_u32(e + ENTITY_SHIELD_LINK)
    shield_flag = dbg.read_u32(link + SHIELD_FLAGS) if race_field.ram(link) else None
    return {
        "slot": s["slot"], "player": s["player"], "entity": e,
        "pool": dbg.read_f32(e + ENTITY_POOL), "state": dbg.read_u32(e + ENTITY_STATE),
        "shield_link": link, "shield_flag": shield_flag, "flags": dbg.read_u32(e + ENTITY_FLAGS),
    }


def record_view(dbg, record):
    kind = dbg.read_u32(record)
    point = list(dbg.read_f32s(record + 0x10, 3))
    impulse = list(dbg.read_f32s(record + 0x20, 3))
    friction = dbg.read_f32(record + 0x30)
    return {
        "address": record, "kind": kind, "point": point, "impulse": impulse,
        "p": sum(x * x for x in impulse) ** 0.5, "friction": friction,
    }


def resolver_probe(dbg, bodies):
    """Stop at the first `Body_ResolveContactPair` impulse for the staged pair and read `j`, the normal and the
    relative normal velocity, so the ring's `|p|` can be compared with the impulse itself. Leaves the CPU stopped
    with the resolver's breakpoint armed."""
    capture.move_breakpoint(dbg, capture.SHIP_UPDATE_CRAFT, capture.CALL_B)
    for skipped in range(400):
        dbg.call("cpu.status")
        dbg.pending = []
        dbg.call("cpu.resume")
        dbg.wait_for_break(capture.CALL_B, timeout=60.0)
        r = capture.gpr(dbg)
        if {r["s0"], r["s1"]} == bodies:
            break
    else:
        return {"error": "the staged pair never reached the resolver"}
    a, b = capture.body(dbg, r["s0"]), capture.body(dbg, r["s1"])
    c = capture.contact(dbg, r["s2"])
    ia, ib = capture.vec3(dbg, r["s4"]), capture.vec3(dbg, r["s5"])
    vn = capture.dot(capture.sub(capture.point_velocity(a, c["point"]), capture.point_velocity(b, c["point"])), c["normal"])
    return {
        "skipped": skipped, "body_a": r["s0"], "body_b": r["s1"], "impulse_a": ia, "j": capture.length(ia),
        "j_signed": capture.dot(ia, c["normal"]), "normal": c["normal"], "depth": c["depth"], "vn_before": vn,
        "inv_mass_a": a["inv_mass"], "inv_mass_b": b["inv_mass"],
        "velocity_a": a["velocity"], "velocity_b": b["velocity"],
    }


def rearm(dbg, old, new, condition=None):
    dbg.brk()
    if old is not None:
        dbg.remove_breakpoint(old)
    dbg.add_breakpoint(new, condition)
    dbg.call("cpu.status")
    dbg.pending = []
    dbg.call("cpu.resume")


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--port", type=int, required=True)
    ap.add_argument("--speed", type=float, default=40.0, help="the placed craft's speed along the rival's forward")
    ap.add_argument("--gap", type=float, default=20.0)
    ap.add_argument("--control", action="store_true", help="stage nothing, run the same window")
    ap.add_argument("--window", type=float, default=3.0, help="race seconds to log after the stage")
    ap.add_argument("--all-callers", action="store_true", help="log every Ship_Damage caller, not only the contact loop")
    ap.add_argument("--probe-j", action="store_true",
                    help="first stop at the staged pair's resolver impulse and read j (the pools before the first "
                    "logged call then include every tick up to that stop)")
    ap.add_argument("--wait", type=float, default=15.0, help="wall seconds to wait for the next call before stopping")
    ap.add_argument("--weapons", type=int, choices=(0, 1), help="write g_weapons_enabled before the stage (Ship_Damage halves with it clear)")
    ap.add_argument("--hold-cross", action="store_true", help="hold the throttle through the window")
    ap.add_argument("--out", type=Path, required=True)
    args = ap.parse_args()

    dbg = Debugger(args.port)
    field = race_field.Field(dbg)
    result = {
        "speed": None if args.control else args.speed, "gap": args.gap, "control": args.control,
        "weapons_enabled": dbg.read_u8(G_WEAPONS), "damage_enabled": dbg.read_u8(G_DAMAGE),
        "clock_load": dbg.read_f32(CLOCK),
    }
    if args.weapons is not None:
        dbg.write(G_WEAPONS, bytes([args.weapons]))
        result["weapons_enabled_written"] = dbg.read_u8(G_WEAPONS)
    result["clock_stage"] = dbg.read_f32(CLOCK)
    result["before"] = [entity_view(dbg, s) for s in field.slots]
    result["poses_stage"] = {s["slot"]: {"pos": list(dbg.read_f32s(s["body"] + 0x30, 3)),
                                         "vel": list(dbg.read_f32s(s["body"] + 0x140, 3))} for s in field.slots}
    staged = None
    armed = None
    if not args.control:
        with open("/dev/null", "w") as null:
            where, bodies = capture.stage_head_on(dbg, field.player["craft"], args.speed, args.gap, null, "same")
        rival = None
        for s in field.slots:
            if s["body"] in bodies and not s["player"]:
                rival = s
        fwd = list(dbg.read_f32s(rival["body"] + 0x20, 3))
        rv = list(dbg.read_f32s(rival["body"] + 0x140, 3))
        rival_speed = sum(a * b for a, b in zip(rv, fwd))
        staged = {"rival_slot": rival["slot"], "rival_speed_along_fwd": rival_speed,
                  "closing_speed": rival_speed - args.speed}
        result["staged"] = staged
        if args.probe_j:
            result["resolver"] = resolver_probe(dbg, bodies)
            armed = capture.CALL_B
        else:
            armed = capture.SHIP_UPDATE_CRAFT
    if args.hold_cross:
        dbg.hold(cross=True)

    calls = []
    t0 = dbg.read_f32(CLOCK)
    condition = None if args.all_callers else "ra == 0x%08x" % LOOP_RETURN
    rearm(dbg, armed, SHIP_DAMAGE, condition)
    while True:
        try:
            dbg.wait_for_break(SHIP_DAMAGE, timeout=args.wait)
        except TimeoutError:
            break
        r = regs(dbg)
        call = {
            "clock": dbg.read_f32(CLOCK), "entity": r["a0"], "source": r["a1"], "a2": r["a2"], "a3": r["a3"],
            "ra": r["ra"], "amount": r["f12"], "s0": r["s0"],
            "record": record_view(dbg, r["s0"]) if race_field.ram(r["s0"]) else None,
            "pool_entry": dbg.read_f32(r["a0"] + ENTITY_POOL), "state": dbg.read_u32(r["a0"] + ENTITY_STATE),
            "shield_flag": None,
            "pos": {sl["slot"]: [round(x, 2) for x in dbg.read_f32s(sl["body"] + 0x30, 3)] for sl in field.slots},
            "vel": {sl["slot"]: [round(x, 2) for x in dbg.read_f32s(sl["body"] + 0x140, 3)] for sl in field.slots},
        }
        link = dbg.read_u32(r["a0"] + ENTITY_SHIELD_LINK)
        if race_field.ram(link):
            call["shield_flag"] = dbg.read_u32(link + SHIELD_FLAGS)
        rearm(dbg, SHIP_DAMAGE, r["ra"])
        dbg.wait_for_break(r["ra"], timeout=args.wait)
        call["pool_return"] = dbg.read_f32(r["a0"] + ENTITY_POOL)
        calls.append(call)
        if call["clock"] - t0 > args.window:
            break
        rearm(dbg, r["ra"], SHIP_DAMAGE, condition)
    dbg.brk()
    for address in (SHIP_DAMAGE, LOOP_RETURN, calls[-1]["ra"] if calls else SHIP_DAMAGE):
        try:
            dbg.remove_breakpoint(address)
        except Exception:  # noqa: BLE001 - the one not armed
            pass
    result["calls"] = calls
    result["clock_end"] = dbg.read_f32(CLOCK)
    result["after"] = [entity_view(dbg, s) for s in field.slots]
    result["poses_end"] = {s["slot"]: {"pos": list(dbg.read_f32s(s["body"] + 0x30, 3)),
                                       "vel": list(dbg.read_f32s(s["body"] + 0x140, 3))} for s in field.slots}
    dbg.resume()
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(result, indent=1))
    print("%d Ship_Damage calls from the contact loop; wrote %s" % (len(calls), args.out))
    for c in calls:
        rec = c["record"] or {"kind": -1, "p": 0.0}
        print("  clock %.4f entity %#x ra %#x source %d kind %d |p| %.4f amount %.5f ratio %.5f pool %.4f -> %.4f" % (
            c["clock"], c["entity"], c["ra"], c["source"], rec["kind"], rec["p"], c["amount"],
            c["amount"] / rec["p"] if rec["p"] else float("nan"), c["pool_entry"], c["pool_return"]))


if __name__ == "__main__":
    main()
